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
//! non-finite is `sim_failed`. **Escape** is the cadence's too (`kernel::detect::escape`): at each sync boundary that
//! is a window mark the escape test runs, and where it fires the march ends there and the sample is `escape`, its
//! `detail` the escaper's body index or 3, a triple ejection. It fires only once the shape point has settled: soon
//! for bodies that all fly apart, late for an escaper leaving a binary, whose phase keeps the shape point turning
//! (`plan/KNOWN.md`).
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
//! **Domain** (pitfalls §10): one frame, the chart, the window, the horizon and these values held fixed.

use std::num::NonZeroUsize;

use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::detect::t0::t0;
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::monitored;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::State;
use kernel::payload::{
    sd_detail, sd_state, set_detail, set_state, SimStateBaseOf, STATE_BOUNDED, STATE_RUNNING,
    STATE_SIM_FAILED,
};
use kernel::regularisation::az::{End, ExplicitTest};
use kernel::regularisation::heggie::{march, HgParams, DEFAULT_TIME};
use kernel::regularisation::sync::{boundary_at_or_before, delta_sync};
use render::colour::outcome::{shown, Shown, K, SWATCHES};

use crate::compute::ComputeError;
use crate::contract::sim_config::{SimConfig, Window};
use crate::decode_dispatch::decode_only_with;
use crate::first_slice::{narrow, uniforms, Quad};

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
}

/// The CPU backend: the module's note. Blocking, on as many threads as the machine has.
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
    let start = t0(ic, degenerate, &radius);
    if start.state != STATE_RUNNING {
        return Outcome {
            state: start.state,
            detail: start.detail,
        };
    }
    let mut cadence = WrapperCadence::with_radius(&ic.m, &ic.r, &ic.p, radius);
    let state = State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    };
    let out = march::<Rk4, f32, _, _>(&state, k_target, params, &mut cadence, &mut ExplicitTest);
    let (code, detail) = match out.count.end {
        End::NonFinite => (STATE_SIM_FAILED, failed_detail(&out.state)),
        End::Exhausted => (STATE_RUNNING, 0),
        End::Done | End::Marching if out.boundary >= k_target => (STATE_BOUNDED, 0),
        End::Done | End::Marching => (STATE_RUNNING, 0),
    };
    let base = SimStateBaseOf::<f32> {
        r: out.state.r,
        p: out.state.p,
        packed_a: set_detail(set_state(0, code), detail),
        ..SimStateBaseOf::default()
    };
    let labelled = monitored(base, &cadence);
    Outcome {
        state: sd_state(labelled.packed_a),
        detail: sd_detail(labelled.packed_a),
    }
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
        let sample = |i: usize| -> Result<Outcome, ComputeError> {
            let out = decode_only_with::<Latent<f32>, Defaults>(
                &quad.chart,
                quad.uv(i),
                &decode_key,
                None,
            )?;
            Ok(outcome(
                &narrow(&out.ic),
                out.degenerate,
                key.radius,
                k_target,
                &params,
            ))
        };
        let threads = std::thread::available_parallelism().map_or(1, NonZeroUsize::get);
        let outcomes = on_threads(quad.len(), threads, sample)?;
        Ok(Field { size, outcomes })
    }
}

/// `f` at every index below `n`, sample `i` on worker `i mod threads`, in index order; the error of the lowest failing
/// index where one fails.
fn on_threads<T: Send + Copy + Default, E: Send>(
    n: usize,
    threads: usize,
    f: impl Fn(usize) -> Result<T, E> + Sync,
) -> Result<Vec<T>, E> {
    let threads = threads.clamp(1, n.max(1));
    let f = &f;
    let lanes: Vec<Vec<(usize, Result<T, E>)>> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|lane| s.spawn(move || (lane..n).step_by(threads).map(|i| (i, f(i))).collect()))
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap_or_else(|p| std::panic::resume_unwind(p)))
            .collect()
    });
    let mut out = vec![T::default(); n];
    let mut failed: Option<(usize, E)> = None;
    for (i, r) in lanes.into_iter().flatten() {
        match r {
            Ok(v) => out[i] = v,
            Err(e) if failed.as_ref().is_none_or(|(j, _)| i < *j) => failed = Some((i, e)),
            Err(_) => {}
        }
    }
    match failed {
        Some((_, e)) => Err(e),
        None => Ok(out),
    }
}

/// The field through the CPU backend: [`Cpu`]'s [`FieldBackend::field`].
pub fn field(sim: &SimConfig, window: Window, size: [usize; 2]) -> Result<Field, ComputeError> {
    Cpu.field(sim, window, size)
}
