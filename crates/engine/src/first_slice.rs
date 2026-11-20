//! The first slice (R-926 items 5 and 6; R-935, RQ-766 (a); REQ-DEC-057, REQ-PERF-120; TASK-M2-86): one real chart
//! at a time put through the route R-926 rules, against today's decode. The CPU evaluates the chart in f64 at each
//! sample, the quad's forward table is the `Ic<f64>` of each sample, one exact node per sample (RQ-766 (a)), each node
//! is narrowed once to `Ic<f32>` and written with `write_ic` as the record the integrator reads, and the CPU reference
//! integrates the records at f32. No new type: today's charts, today's [`Window`] (the latent chart's window its plane
//! `(z₀, q₁, q₂)`), today's record layout (`kernel::stage::decode::ic_words`). No backwards table, no slopes, no blend,
//! no time coordinate, no shader.
//!
//! **The charts** ([`SliceChart`]): the latent chart (`kernel::chart::latent`), the shape sphere under its exponential
//! map (`kernel::chart::shape_sphere`) and the `(L_z, E)` chart (`kernel::chart::invariant`'s [`InvariantLE`], which
//! `kernel::chart::energy`'s `Mapped` wraps, warped on both axes, bit for bit), each held at f32, the precision today's
//! decode and the GPU's uniform read; the CPU's f64 evaluation reads the parameters widened, never narrowed
//! (`decode_only_with`), so the f64 route and today's f32 decode start from the same chart (R-649: like for like).
//! Their parameters and REQ-DEC-057's grids are the measurement's, `validation::first_slice`.
//!
//! **A quad** ([`Quad`]): a chart, today's [`Window`] on it, and its samples; sample `(x, y)` of a `w × h` quad is the
//! window's point at `s = (x + ½)/w`, `t = (y + ½)/h`. The latent chart's window is its plane, so its quad's window is
//! [`Window::SQUARE`], `(u, v)` the plane's own coordinates.
//!
//! **Today's decode** ([`today`]): the same source at f32 (`kernel::stage::decode::sample_with::<…, f32>`), at the
//! sample's `(u, v)` rounded once to f32, as the decode stage's dispatch takes it.
//!
//! **The integration** ([`outcome`]): `kernel::driver::wrapper::integrate_detecting` at f32 under a horizon's
//! validated loop values and the key's collision threshold, Part 3's default `r_coll` ([`uniforms`]), the occupant a
//! type parameter. The wrapper's cadence labels collisions and escapes (`kernel::detect`); a march neither ends is
//! `bounded` at the horizon, whatever the occupant.
//!
//! **Domain** (pitfalls §10): a decode and a march from its record, the chart and the window held fixed.

use kernel::chart::{InvariantLE, Latent, ShapeSphere};
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::advance::Advance;
use kernel::driver::wrapper::{integrate_detecting, WrapperUniforms};
use kernel::generated::links::Defaults;
use kernel::payload::{sd_detail, sd_state, tm_t_end_step, SimStateBase};
use kernel::stage::decode::{
    ic_words, sample_with, write_ic, DecodeOut, DecodeVariant, InvariantLEDefaults, LatentDefaults,
    ShapeSphereDefaults, UniformChart,
};

use crate::compute::ComputeError;
use crate::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, SliceFrame, Window, Z_Q3,
};
use crate::contract::sim_uniforms::{dt_macro_for, SimUniforms};
use crate::decode_dispatch::decode_only_with;
use crate::dispatch::validate::{validate, DispatchError};

/// One quad: the chart, today's window on it, and its samples, `size = [w, h]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quad<C> {
    /// The chart, its parameters at f32.
    pub chart: C,
    /// The window on a curved chart's coordinates; [`Window::SQUARE`] on the latent chart, whose window is its plane.
    pub window: Window,
    /// `[w, h]`, the samples across and up.
    pub size: [usize; 2],
}

impl<C: SliceChart> Quad<C> {
    /// The samples, `w·h`.
    pub fn len(&self) -> usize {
        self.size[0] * self.size[1]
    }

    /// Whether the quad has no sample.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The chart point `(u, v)` of sample `(x, y)`, at f64: the window's point at `((x + ½)/w, (y + ½)/h)`. `x` and
    /// `y` may lie past the quad, a neighbour's point.
    pub fn at(&self, x: i64, y: i64) -> [f64; 2] {
        let s = (x as f64 + 0.5) / self.size[0] as f64;
        let t = (y as f64 + 0.5) / self.size[1] as f64;
        self.window.at(s, t)
    }

    /// The chart point of sample `i`, row by row from the bottom left.
    pub fn uv(&self, i: usize) -> [f64; 2] {
        let w = self.size[0];
        self.at((i % w) as i64, (i / w) as i64)
    }

    /// The physics sim key this quad decodes under: the window and, on the latent chart, the plane.
    pub fn sim_key(&self) -> SimConfig {
        SimConfig {
            chart: Chart {
                window: self.window,
                mapping: Default::default(),
            },
            plane: self.chart.plane(),
            slice: Slice {
                frame: SliceFrame::along(Z_Q3),
            },
            lock: Lock {
                locked: false,
                z_locked: [0.0; 8],
            },
            links: Links::defaults(),
            integrator: Integrator {},
            kernel_variant: KernelVariant::Physics,
            horizon: Horizon { t: 0.0 },
            collision: Collision {},
            quality: Quality {},
        }
    }
}

/// A chart of the slice: held at f32, decoded by the decode stage's variant [`SliceChart::Variant`] on the GPU.
pub trait SliceChart: UniformChart + Copy {
    /// The decode stage's variant for the chart under the default links.
    type Variant: DecodeVariant<Chart = Self, Links = Defaults>;

    /// The chart's name in the report.
    const NAME: &'static str;

    /// The plane the sim key holds: the latent chart's own; zero on a curved chart, which reads its window.
    fn plane(&self) -> Plane {
        Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        }
    }
}

impl SliceChart for Latent<f32> {
    type Variant = LatentDefaults;
    const NAME: &'static str = "latent";

    fn plane(&self) -> Plane {
        let wide = |v: [f32; 8]| v.map(f64::from);
        Plane {
            z0: wide(self.z0),
            q1: wide(self.q1),
            q2: wide(self.q2),
        }
    }
}

impl SliceChart for ShapeSphere<f32> {
    type Variant = ShapeSphereDefaults;
    const NAME: &'static str = "shape-sphere-exponential";
}

impl SliceChart for InvariantLE<f32> {
    type Variant = InvariantLEDefaults;
    const NAME: &'static str = "invariant-lz-e";
}

/// The quad's forward table (R-926 item 6; RQ-766 (a)): the CPU's f64 evaluation at each sample, `decode_only_with`
/// at the sample's `(u, v)` under the quad's sim key, `η_E` off; one exact node per sample, the `Ic<f64>`.
pub fn forward_table<C: SliceChart>(quad: &Quad<C>) -> Result<Vec<Ic<f64>>, ComputeError> {
    // Sized once, not grown by doubling: a doubling `collect` of a 1920 × 1080 table left the reallocations' blocks
    // in macOS malloc's large-allocation cache, and the bench's footprint grew past 3 GB over its runs.
    let mut table = Vec::with_capacity(quad.len());
    forward_table_into(quad, &mut table)?;
    Ok(table)
}

/// [`forward_table`] written into `table`, which is cleared first and keeps its capacity: the same nodes, in a buffer
/// the caller reuses (the bench's split of the timed work, REQ-PERF-120).
pub fn forward_table_into<C: SliceChart>(
    quad: &Quad<C>,
    table: &mut Vec<Ic<f64>>,
) -> Result<(), ComputeError> {
    let key = quad.sim_key();
    table.clear();
    table.reserve(quad.len());
    for i in 0..quad.len() {
        table.push(decode_only_with::<C, Defaults>(&quad.chart, quad.uv(i), &key, None)?.ic);
    }
    Ok(())
}

/// `ic` narrowed once to f32, each component rounded to the nearest f32 (`as`, IEEE 754's round to nearest, ties to
/// even).
pub fn narrow(ic: &Ic<f64>) -> Ic<f32> {
    Ic {
        m: ic.m.map(|x| x as f32),
        r: ic.r.map(|v| v.map(|x| x as f32)),
        p: ic.p.map(|v| v.map(|x| x as f32)),
    }
}

/// The records the host writes for `table`: each node narrowed once ([`narrow`]) and written with `write_ic` at its
/// sample, the integrator's input buffer, `ic_words` words a sample.
pub fn records(table: &[Ic<f64>]) -> Vec<u32> {
    let mut out = Vec::new();
    records_into(table, &mut out);
    out
}

/// [`records`] written into `out`, which is cleared first and keeps its capacity.
pub fn records_into(table: &[Ic<f64>], out: &mut Vec<u32>) {
    out.clear();
    out.resize(table.len() * ic_words(), 0);
    for (i, node) in table.iter().enumerate() {
        write_ic(out, i, &narrow(node));
    }
}

/// The slice's CPU evaluation of `quad`: its forward table, then its records. What REQ-PERF-120 times.
pub fn evaluate<C: SliceChart>(quad: &Quad<C>) -> Result<Vec<u32>, ComputeError> {
    Ok(records(&forward_table(quad)?))
}

/// Today's decode of `quad`: the decode stage's source at f32 (`sample_with::<C, Defaults, f32>`) at each sample's
/// `(u, v)` rounded once to f32, `η_E` off.
pub fn today<C: SliceChart>(quad: &Quad<C>) -> Vec<DecodeOut<f32>> {
    (0..quad.len())
        .map(|i| {
            let [u, v] = quad.uv(i);
            sample_with::<C, Defaults, f32>(&quad.chart, u as f32, v as f32, None)
        })
        .collect()
}

/// What a march reads from the sim key, as dispatch validates it: the f32 loop values and the collision threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarchKey {
    /// The loop values at f32 ([`crate::dispatch::validate::Validated::wrapper_uniforms_f32`]).
    pub loop_values: WrapperUniforms<f32>,
    /// The collision threshold of the key's `r_coll` ([`crate::dispatch::validate::Validated::collision_radius`]),
    /// Part 3's default `r_coll = 1e-3`: the collision branch fires as dispatch's integration would.
    pub radius: CollisionRadius,
}

/// The sim key with horizon `t` and every other field Part 3's default, `dt_macro` the horizon's (R-132), as dispatch
/// validates it: its [`MarchKey`]; dispatch's refusal for a horizon it refuses.
pub fn uniforms(t: f64) -> Result<MarchKey, DispatchError> {
    let key = SimUniforms {
        t_horizon: t,
        dt_macro: dt_macro_for(t),
        ..SimUniforms::default()
    };
    let valid = validate(&key)?;
    Ok(MarchKey {
        loop_values: valid.wrapper_uniforms_f32(),
        radius: valid.collision_radius(),
    })
}

/// What a march from one record ends in: its `state` (payload §2) and its `t_end_step`, the completed macro-steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The outcome class, `sd_state`.
    pub state: u32,
    /// The class's detail, `sd_detail` (payload §2): which body escaped, which pair collided; the class is
    /// `(state, detail)`.
    pub detail: u32,
    /// `t_end_step`.
    pub t_end_step: u32,
}

/// The record `ic` integrated by the CPU reference at f32 under `key`'s loop values and collision threshold, the
/// occupant `A`: the `SimState` it ends in. Escape is not labelled here: no detector writes `escape` until TASK-M3-12,
/// so a march that is not a collision runs to the horizon `bounded`.
pub fn march<A: Advance>(ic: &Ic<f32>, key: &MarchKey) -> SimStateBase {
    integrate_detecting::<A, f32>(ic, &key.loop_values, key.radius)
}

/// [`march`]'s [`Outcome`].
pub fn outcome<A: Advance>(ic: &Ic<f32>, key: &MarchKey) -> Outcome {
    let s = march::<A>(ic, key);
    Outcome {
        state: sd_state(s.packed_a),
        detail: sd_detail(s.packed_a),
        t_end_step: tm_t_end_step(s.times),
    }
}
