//! The field: one entry point, [`field`], from a sim key, a window on its chart and a size to an outcome per sample,
//! behind [`FieldBackend`], whose CPU implementation is [`Cpu`] (the GPU's comes later). The GUI calls only this.
//!
//! **What the CPU backend computes** at each sample of the latent chart's `w × h` quad (`first_slice::Quad`, sample
//! `(x, y)` at the window's point `((x + ½)/w, (y + ½)/h)`, row by row from the bottom left): the chart decoded at f64
//! and narrowed once to f32, as `first_slice` does; the dispatch-time label of that IC (`kernel::detect::t0`), which
//! settles a decode failure or a start already inside `r_coll` with no march; otherwise Heggie × RK4 at f32, registered
//! once at `t = 0` and marched to the horizon over the sync grid (`kernel::regularisation::heggie::march`), with the
//! wrapper's own cadence watching every step, so the collision detector, the projection and the invariant monitor run
//! as they do under any occupant (`WrapperCadence`, labelled by `monitored`). A march that reaches the horizon is
//! `bounded`; one whose step budget ran out is `running` (undetermined, as the wrapper labels it); one that went
//! non-finite is `sim_failed`. **Escape** is the cadence's too (`kernel::detect::escape_rule`): at each sync boundary
//! that is a window mark the escape rule re-reads its label, live by default, so the sample is `escape` where the last
//! mark's label is one, its `detail` the escaper's body index or 3, a triple ejection; under the stop option
//! ([`outcome_stopping`]) the first label ends the march there and stays. The default rule is Fewbody's termination
//! test (`kernel::detect::escape_rule::Fewbody`): it names the escape of the body outside the bound pair of least
//! Kepler `a` once that body is unbound from the pair, receding, and the pair's tidal ratio from it is below the
//! register's `δ`; with no bound pair, a triple ejection where every pair recedes and `E_0 > 0`.
//!
//! **The sync grid** is the register's `Δ_sync` (`kernel::regularisation::sync`), and the march stops on the latest
//! boundary at or before the horizon ([`boundaries`]), never past it (integrator_contract Part 2a; R-565).
//!
//! **Heggie's other step-control values** are calibrations with no value in the register yet. The plan is unclear here;
//! chose the integrator contract's radial-collision gate values (`η = 0.01`, the predictive limit's `f = 0.02`, the
//! landing's 4 secant retakes and 64 halvings, Eq. 22's time transformation), the reference `TINY` (`1e-37` at f32),
//! and a finite step budget (Part 3's starting rate, 4·10⁴ steps per unit time, floored at 10⁵ steps), because those
//! are the values the gate measures Heggie at, and a march with no budget can stall one sample.
//!
//! **The stored records** ([`record`]): beside each outcome the CPU backend keeps the sample's whole stored record, the
//! payload a stain's read side unpacks (its `SimState`, word and `ICDescriptor`), written from the same march
//! ([`Stored`], [`FieldBackend::stored`]).
//!
//! **Where it runs**: on the engine's shared CPU worker pool (`crate::pool`), which leaves a core free and returns
//! the samples in index order, so the field is the same bit for bit on any worker count.
//!
//! **Domain** (pitfalls §10): one frame, the chart, the window, the horizon and these values held fixed.

use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::generated::links::Defaults;
use kernel::occupant::State;
use kernel::regularisation::heggie::{HgParams, DEFAULT_TIME};
use kernel::regularisation::sync::{boundary_at_or_before, delta_sync};
use render::colour::outcome::{shown, Shown, K, SWATCHES};

use crate::compute::ComputeError;
use crate::contract::sim_config::{SimConfig, Window};
use crate::decode_dispatch::decode_only_with;
use crate::first_slice::{narrow, uniforms, Quad};

pub mod record;

pub use record::{Record, RecordBytes, Stored};

/// How one sample ends: its outcome class's `state` and `detail` (payload §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Outcome {
    /// `sd_state`.
    pub state: u32,
    /// `sd_detail`.
    pub detail: u32,
}

/// A field: an [`Outcome`] per sample of a `w × h` quad, row by row from the bottom left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// `[w, h]`.
    pub size: [usize; 2],
    /// The outcomes, `w·h`, row by row from the bottom left.
    pub outcomes: Vec<Outcome>,
}

impl Field {
    /// The outcome of sample `(x, y)`, `y` up from the bottom row.
    pub fn at(&self, x: usize, y: usize) -> Outcome {
        self.outcomes[y * self.size[0] + x]
    }

    /// How many samples end in each `(state, detail)`, in order. A failure (`sim_failed`, `decode_failed`, `running`)
    /// keeps its own key here, where the outcome palette would fold it into the bounded / nothing yet class.
    pub fn histogram(&self) -> Vec<(Outcome, usize)> {
        let mut counts = std::collections::BTreeMap::new();
        for o in &self.outcomes {
            *counts.entry(*o).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// How many samples the outcome palette draws in each of its classes, in `SWATCHES`' order; a sample it draws as
    /// the canvas background (a decode failure) is in none.
    pub fn classes(&self) -> [usize; K] {
        let mut counts = [0; K];
        for o in &self.outcomes {
            if let Shown::Swatch(k) = shown(o.state, o.detail, false) {
                counts[k] += 1;
            }
        }
        counts
    }

    /// The field as an image, 8-bit sRGB RGBA, rows from the top (as a texture is laid out): each sample its outcome
    /// palette swatch (`render::colour::outcome`'s default set) at alpha 255; a sample the palette draws as the canvas
    /// background (a decode failure) transparent, `[0, 0, 0, 0]`, for the canvas to draw its background there.
    pub fn rgba8(&self) -> Vec<u8> {
        let [w, h] = self.size;
        let mut out = Vec::with_capacity(w * h * 4);
        for row in (0..h).rev() {
            for x in 0..w {
                let o = self.at(x, row);
                out.extend_from_slice(&match shown(o.state, o.detail, false) {
                    Shown::Swatch(k) => {
                        let [r, g, b] = SWATCHES[k].srgb8;
                        [r, g, b, 255]
                    }
                    Shown::Forbidden | Shown::DecodeFailed => [0; 4],
                });
            }
        }
        out
    }
}

/// What computes a field.
pub trait FieldBackend: Send + Sync {
    /// The field of `sim`'s chart over `window`, `size = [w, h]` samples; an error where the sim key cannot be
    /// computed (not the physics kernel, a horizon dispatch refuses).
    fn field(
        &self,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Field, ComputeError>;

    /// The field with each sample's stored record ([`Stored`]); an error where this backend keeps no records, as
    /// the default does.
    fn stored(
        &self,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Stored, ComputeError> {
        let _ = (sim, window, size);
        Err(ComputeError(
            "this field backend keeps no stored records".into(),
        ))
    }
}

/// The CPU backend: the module's note. Blocking, on the engine's shared worker pool (`crate::pool::shared`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Cpu;

/// Heggie's step-control values at f32.
pub type HgParams32 = HgParams<f32>;

/// Heggie's step-control values the CPU backend marches with, at f32 (the module's note).
pub fn heggie_params() -> HgParams<f32> {
    HgParams {
        eta: 0.01,
        delta_sync: delta_sync(),
        tiny: 1e-37,
        predictive_f: 0.02,
        step_budget_rate: 4e4,
        n_min: 100_000,
        clamp: true,
        secant_tries: 4,
        bisect_tries: 64,
        time: DEFAULT_TIME,
    }
}

/// The sync boundaries a march to the horizon `t` takes: the latest boundary at or before `t`, `⌊t/Δ_sync⌋` decided by
/// the products `k·Δ_sync` (`kernel::regularisation::sync`), at f64. `ADVANCE` stops only on the grid and never past
/// its target (R-565), so a horizon off the grid ends on the boundary before it.
pub fn boundaries(t: f64, params: &HgParams<f32>) -> u32 {
    boundary_at_or_before(t, f64::from(params.delta_sync))
}

/// The latent chart `sim` holds, at f32: its plane's `z₀`, `q₁` and `q₂`, each rounded once.
pub fn latent(sim: &SimConfig) -> Latent<f32> {
    let narrow = |v: [f64; 8]| v.map(|x| x as f32);
    Latent {
        z0: narrow(sim.plane.z0),
        q1: narrow(sim.plane.q1),
        q2: narrow(sim.plane.q2),
    }
}

/// How the IC `ic`, decoded with the tag `degenerate`, ends under the collision threshold `radius`, marched by Heggie ×
/// RK4 at f32 over `k_target` sync boundaries with `params` (the module's note).
pub fn outcome(
    ic: &Ic<f32>,
    degenerate: Option<kernel::decode::degenerate::Degenerate>,
    radius: CollisionRadius,
    k_target: u32,
    params: &HgParams<f32>,
) -> Outcome {
    outcome_stopping(ic, degenerate, radius, k_target, params, false)
}

/// [`outcome`] under the stop option `stop_on_escape` (`SimUniforms::stop_on_escape`; `kernel::driver::cadence`'s
/// note): off, the escape label is live and the march goes on to `k_target`; on, the first label ends it and stays.
pub fn outcome_stopping(
    ic: &Ic<f32>,
    degenerate: Option<kernel::decode::degenerate::Degenerate>,
    radius: CollisionRadius,
    k_target: u32,
    params: &HgParams<f32>,
    stop_on_escape: bool,
) -> Outcome {
    record::sample(
        ic,
        degenerate,
        radius,
        k_target,
        params,
        stop_on_escape,
        false,
    )
    .outcome
}

/// `sim_failed`'s detail for a march that ended at `s` (payload §2): 0 for a NaN in the state, else 1 for a `±∞`,
/// else 2, a non-finite quantity derived from a finite state.
fn failed_detail(s: &State<f32>) -> u32 {
    let values = || s.r.iter().chain(s.p.iter()).flatten();
    if values().any(|x| x.is_nan()) {
        0
    } else if values().any(|x| x.is_infinite()) {
        1
    } else {
        2
    }
}

impl FieldBackend for Cpu {
    fn field(
        &self,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Field, ComputeError> {
        self.field_on(crate::pool::shared()?, sim, window, size)
    }

    fn stored(
        &self,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Stored, ComputeError> {
        self.stored_on(crate::pool::shared()?, sim, window, size)
    }
}

impl Cpu {
    /// [`FieldBackend::field`] on the workers of `pool`: the same field, bit for bit, on any pool.
    pub fn field_on(
        &self,
        pool: &crate::pool::Pool,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Field, ComputeError> {
        Ok(self.marched_on(pool, sim, window, size, false)?.field)
    }

    /// [`FieldBackend::stored`] on the workers of `pool`: the same records, bit for bit, on any pool. An error, beside
    /// [`Cpu::field_on`]'s, where the march targets more than 65535 sync boundaries, `t_end_step`'s 16 bits.
    pub fn stored_on(
        &self,
        pool: &crate::pool::Pool,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
    ) -> Result<Stored, ComputeError> {
        let stored = self.marched_on(pool, sim, window, size, true)?;
        if stored.read.horizon_steps > 0xffff {
            return Err(ComputeError(format!(
                "the horizon {} is {} sync boundaries, past the 65535 a stored step count holds",
                sim.horizon.t, stored.read.horizon_steps
            )));
        }
        Ok(stored)
    }

    /// The field and its records, the counts unchecked, the words marched where `word` (`record::sample`).
    fn marched_on(
        &self,
        pool: &crate::pool::Pool,
        sim: &SimConfig,
        window: Window,
        size: [usize; 2],
        word: bool,
    ) -> Result<Stored, ComputeError> {
        let key = uniforms(sim.horizon.t).map_err(|e| ComputeError(format!("{e:?}")))?;
        let params = heggie_params();
        let k_target = boundaries(sim.horizon.t, &params);
        let quad = Quad {
            chart: latent(sim),
            window,
            size,
        };
        let decode_key = SimConfig {
            kernel_variant: sim.kernel_variant,
            ..quad.sim_key()
        };
        let sample = |i: usize| -> Result<(Outcome, Record), ComputeError> {
            let out = decode_only_with::<Latent<f32>, Defaults>(
                &quad.chart,
                quad.uv(i),
                &decode_key,
                None,
            )?;
            let s = record::sample(
                &narrow(&out.ic),
                out.degenerate,
                key.radius,
                k_target,
                &params,
                key.stop_on_escape,
                word,
            );
            Ok((
                s.outcome,
                Record {
                    sim: s.sim,
                    word: s.word,
                    ic: record::narrow_descriptor(&out.descriptor),
                },
            ))
        };
        let (outcomes, records) = pool.map(quad.len(), sample)?.into_iter().unzip();
        Ok(Stored {
            field: Field { size, outcomes },
            records,
            read: record::read_params(k_target, &params),
        })
    }
}

/// The field through the CPU backend: [`Cpu`]'s [`FieldBackend::field`].
pub fn field(sim: &SimConfig, window: Window, size: [usize; 2]) -> Result<Field, ComputeError> {
    Cpu.field(sim, window, size)
}

/// The field with each sample's stored record through the CPU backend: [`Cpu`]'s [`FieldBackend::stored`].
pub fn stored(sim: &SimConfig, window: Window, size: [usize; 2]) -> Result<Stored, ComputeError> {
    Cpu.stored(sim, window, size)
}
