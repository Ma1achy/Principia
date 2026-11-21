//! The engine-side Inspector (`engine::inspector`; TASK-M3-19): a clicked IC integrated through the shared kernel at
//! f64 on the CPU, one `Inspector` type for the IC Inspector and the trajectory viewer, with the RK45 reference beside
//! it.
//!
//! Pass criteria, fixed before the first run:
//! - **The shared kernel at f64**: the Inspector's `SimState` is `computeIC`'s, bit for bit; its path has
//!   `t_end_step + 1` states, the IC first and, on these runs, which reach their horizon, the `SimState`'s `(r, p)`
//!   last, bit for bit.
//! - **A click is the post-flip UV**: a click at `(x, y)` on a `W × H` canvas inspects `(x/W, 1 − y/H)`, the same
//!   inspection as that UV given directly.
//! - **The reference beside it**: on the figure-eight over one period `T₈`, under RK4 at the horizon's `dt_macro`
//!   (`10⁻³`), the reference (`rtol = atol = 10⁻¹²`) reaches every boundary the path has, and the kernel's path is
//!   within `10⁻⁸` of it at every boundary. RK4's global error at that step is of order `10⁻¹¹` on this orbit and the
//!   reference's below it, so `10⁻⁸` holds both with room; a first-order march (Euler) misses it by decades.
//! - **Under an occupant that owns its time mapping** (Heggie × RK4, at the test values `kernel`'s `heggie_seam`
//!   takes): the Inspector's `SimState` is `computeIC`'s under the same occupant, bit for bit; the path is at the sync
//!   grid's boundaries, time `k·Δ_sync` for `k = 0, 1, …`, every one to the horizon (`T = 1`, eight boundaries of
//!   `Δ_sync = 1/8`); and the reference reaches every one of them. The divergence is reported, not asserted.
//! - **A drift is never read off a NaN**: an IC with a NaN in a position has no `E₀`, so neither path reports an
//!   energy drift (`None`), never a drift of 0; and the reductions keep a NaN (`nan_max`).
//! - **Both views read one inspection**: the trajectory viewer's real-space paths are the path's positions, one per
//!   boundary.

use kernel::chart::Latent;
use kernel::generated::links::Defaults;
use kernel::occupant::rk4::Rk4;
use render::raster::Grid;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::first_slice::Quad;
use engine::inspector::{nan_max, Inspection, Inspector};
use engine::picking::Screen;
use engine::precision::rk45::{Rk45End, Rk45Params};
use engine::precision::{compute_ic, SimState64};
use engine::trajectory::{FigureEight, Provider};
use validation::negative_control;

fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

const T: f64 = 1.0;

fn key(t: f64) -> SimConfig {
    let quad = Quad {
        chart: latent(),
        window: Window::SQUARE,
        size: [1, 1],
    };
    SimConfig {
        horizon: Horizon { t },
        ..quad.sim_key()
    }
}

fn reference() -> Rk45Params {
    Rk45Params {
        rtol: 1e-12,
        atol: 1e-12,
        max_steps: 1_000_000,
    }
}

fn inspector(t: f64) -> Inspector<Latent<f32>> {
    Inspector {
        chart: latent(),
        sim_key: key(t),
        reference: reference(),
    }
}

fn bits(s: &SimState64) -> String {
    format!("{s:?}")
}

const UV: [f64; 2] = [0.3, 0.6];

// ── The shared kernel at f64 ─────────────────────────────────────────────────────────────────────────────────────

fn check_shared_kernel(got: &Inspection, sim_state: &SimState64) {
    assert_eq!(
        bits(&got.sim_state),
        bits(sim_state),
        "the Inspector's SimState is not computeIC's, bit for bit"
    );
    let steps = got.t_end_step() as usize;
    assert_eq!(
        got.path.len(),
        steps + 1,
        "the path has {} states for {steps} macro-steps",
        got.path.len()
    );
    for (k, t) in got.times.iter().enumerate() {
        assert_eq!(
            *t,
            k as f64 * got.dt_macro,
            "the path's state {k} is not at macro-step {k}'s time"
        );
    }
    assert_eq!(
        format!("{:?}", got.path[0].r),
        format!("{:?}", got.ic.r),
        "the path does not start at the IC"
    );
    let last = got.path.last().unwrap();
    assert_eq!(
        format!("{:?}", (last.r, last.p)),
        format!("{:?}", (got.sim_state.r, got.sim_state.p)),
        "the path does not end at the SimState"
    );
}

#[test]
fn inspector_shared_kernel_compute_ic() {
    let got = inspector(T).inspect::<Defaults, Rk4>(UV).unwrap();
    let want = compute_ic::<Latent<f32>, Defaults, Rk4>(&latent(), UV, &key(T)).unwrap();
    println!(
        "inspected {UV:?}: t_end_step {}, {} path states, reference {:?}, largest divergence {:.3e}",
        got.t_end_step(),
        got.path.len(),
        got.reference.end,
        got.divergence().into_iter().fold(0.0, nan_max)
    );
    check_shared_kernel(&got, &want);
}

negative_control!(
    inspector_shared_kernel_compute_ic,
    "the f32 instantiation's march, widened, is not the f64 kernel's",
    expected = "is not computeIC's",
    {
        let got = inspector(T).inspect::<Defaults, Rk4>(UV).unwrap();
        let out = kernel::stage::decode::sample_with::<Latent<f32>, Defaults, f32>(
            &latent(),
            UV[0] as f32,
            UV[1] as f32,
            None,
        );
        let s =
            engine::first_slice::march::<Rk4>(&out.ic, &engine::first_slice::uniforms(T).unwrap());
        let w = |v: [[f32; 2]; 3]| v.map(|x| x.map(f64::from));
        let widened = SimState64 {
            r: w(s.r),
            p: w(s.p),
            ..got.sim_state
        };
        check_shared_kernel(&got, &widened)
    }
);

// ── A click is the post-flip UV ──────────────────────────────────────────────────────────────────────────────────

fn screen() -> Screen {
    Screen {
        canvas: [200.0, 100.0],
        grid: Grid::new([4, 2], 3, 0, 5).expect("the picking grid"),
        z: [0.0; 8],
    }
}

fn check_click(at: [f64; 2], want_uv: [f64; 2]) {
    let i = inspector(T);
    let clicked = i
        .click::<Defaults, Rk4>(&screen(), at)
        .unwrap()
        .expect("a click on the canvas");
    assert_eq!(
        clicked.uv,
        Some(want_uv),
        "the click at {at:?} does not inspect {want_uv:?}"
    );
    let direct = i.inspect::<Defaults, Rk4>(want_uv).unwrap();
    assert_eq!(
        bits(&clicked.sim_state),
        bits(&direct.sim_state),
        "the click's inspection is not the UV's"
    );
    assert!(
        i.click::<Defaults, Rk4>(&screen(), [-1.0, 0.0])
            .unwrap()
            .is_none(),
        "a click off the canvas inspects"
    );
}

#[test]
fn inspector_shared_kernel_click() {
    check_click([60.0, 40.0], [0.3, 0.6]);
}

negative_control!(
    inspector_shared_kernel_click,
    "the UV unflipped, Y-down, as the event gives it",
    expected = "does not inspect",
    check_click([60.0, 40.0], [0.3, 0.4])
);

// ── The reference beside it ──────────────────────────────────────────────────────────────────────────────────────

fn check_reference(got: &Inspection) {
    let reference = &got.reference;
    assert_eq!(
        reference.end,
        Rk45End::Reached,
        "the reference did not reach the path's end"
    );
    assert_eq!(
        reference.states.len(),
        got.path.len(),
        "the reference is not at every boundary of the path"
    );
    let divergence = got.divergence();
    let worst = divergence.iter().copied().fold(0.0, nan_max);
    let (kernel_drift, reference_drift) = got.energy_drift();
    println!(
        "figure-eight over T₈: {} boundaries, t_end {:.6}, largest divergence {worst:.3e}, |ΔE| kernel {kernel_drift:?}, \
         reference {:?}",
        divergence.len(),
        got.t_end(),
        reference_drift
    );
    assert!(
        worst <= 1e-8,
        "the kernel's path is {worst:.3e} from the reference"
    );
}

fn figure_eight() -> (Inspector<Latent<f32>>, kernel::decode::Ic<f64>) {
    let f = FigureEight::new();
    (inspector(f.quantum()), f.start())
}

#[test]
fn inspector_shared_kernel_reference() {
    let (i, ic) = figure_eight();
    let got = i.inspect_state::<Rk4>(&ic).unwrap();
    check_reference(&got);
}

negative_control!(
    inspector_shared_kernel_reference,
    "a first-order march is far from the reference",
    expected = "from the reference",
    {
        let (i, ic) = figure_eight();
        check_reference(
            &i.inspect_state::<kernel::occupant::euler::Euler>(&ic)
                .unwrap(),
        )
    }
);

// ── Both views read one inspection ───────────────────────────────────────────────────────────────────────────────

fn check_views(got: &Inspection, bodies: &[Vec<[f64; 2]>; 3]) {
    for (k, body) in bodies.iter().enumerate() {
        assert_eq!(
            body.len(),
            got.path.len(),
            "body {k}'s path has {} points for {} states",
            body.len(),
            got.path.len()
        );
        for (j, (point, state)) in body.iter().zip(&got.path).enumerate() {
            assert_eq!(
                *point, state.r[k],
                "body {k}'s point {j} is not the path's position"
            );
        }
    }
}

#[test]
fn inspector_shared_kernel_one_inspection() {
    let got = inspector(T).inspect::<Defaults, Rk4>(UV).unwrap();
    check_views(&got, &got.bodies());
}

negative_control!(
    inspector_shared_kernel_one_inspection,
    "the paths of another IC",
    expected = "is not the path's position",
    {
        let got = inspector(T).inspect::<Defaults, Rk4>(UV).unwrap();
        let other = inspector(T).inspect::<Defaults, Rk4>([0.31, 0.6]).unwrap();
        check_views(&got, &other.bodies())
    }
);

// ── Under an occupant that owns its time mapping ─────────────────────────────────────────────────────────────────

/// Heggie's step-control values as `kernel`'s `heggie_seam` test takes them: test inputs, not calibrations.
struct Values;

impl kernel::regularisation::heggie::HgConfig for Values {
    fn params<R: kernel::regularisation::az::AzReal>() -> kernel::regularisation::heggie::HgParams<R>
    {
        kernel::regularisation::heggie::HgParams {
            eta: R::at_width(0.01, 0.01),
            delta_sync: R::at_width(0.125, 0.125),
            tiny: R::at_width(1e-37, 1e-300),
            predictive_f: R::at_width(0.02, 0.02),
            step_budget_rate: R::at_width(1e7, 1e7),
            n_min: 1_000_000,
            clamp: true,
            secant_tries: 4,
            bisect_tries: 64,
            time: kernel::regularisation::heggie::DEFAULT_TIME,
        }
    }
}

type HeggieRk4 =
    kernel::regularisation::Composed<Rk4, kernel::regularisation::heggie::Heggie<Values>>;

/// The sync quantum of [`Values`].
const DELTA_SYNC: f64 = 0.125;

fn check_heggie(got: &Inspection, sim_state: &SimState64) {
    assert_eq!(
        bits(&got.sim_state),
        bits(sim_state),
        "the Inspector's SimState is not computeIC's under Heggie, bit for bit"
    );
    let boundaries = (T / DELTA_SYNC) as usize;
    assert_eq!(
        got.times.len(),
        boundaries + 1,
        "the path has {} states for {boundaries} sync boundaries",
        got.times.len()
    );
    for (k, t) in got.times.iter().enumerate() {
        assert_eq!(
            *t,
            k as f64 * DELTA_SYNC,
            "the path's state {k} is not at sync boundary {k}"
        );
    }
    assert_eq!(
        got.reference.end,
        Rk45End::Reached,
        "the reference did not reach the path's end"
    );
    assert_eq!(
        got.reference.states.len(),
        got.path.len(),
        "the reference is not at every state of the path"
    );
}

#[test]
fn inspector_shared_kernel_heggie() {
    let got = inspector(T).inspect::<Defaults, HeggieRk4>(UV).unwrap();
    let want = compute_ic::<Latent<f32>, Defaults, HeggieRk4>(&latent(), UV, &key(T)).unwrap();
    println!(
        "Heggie × RK4 at {UV:?}: state {}, {} path states at {:?}, largest divergence from the reference {:.3e}",
        kernel::payload::sd_state(got.sim_state.packed_a),
        got.path.len(),
        got.times,
        got.divergence().into_iter().fold(0.0, nan_max)
    );
    check_heggie(&got, &want);
}

negative_control!(
    inspector_shared_kernel_heggie,
    "the stepper's computeIC is not Heggie's",
    expected = "is not computeIC's under Heggie",
    {
        let got = inspector(T).inspect::<Defaults, HeggieRk4>(UV).unwrap();
        let rk4 = compute_ic::<Latent<f32>, Defaults, Rk4>(&latent(), UV, &key(T)).unwrap();
        check_heggie(&got, &rk4)
    }
);

// ── A drift is never read off a NaN ──────────────────────────────────────────────────────────────────────────────

fn nan_ic() -> kernel::decode::Ic<f64> {
    let mut ic = FigureEight::new().start();
    ic.r[0][0] = f64::NAN;
    ic
}

fn check_no_drift(drift: (Option<f64>, Option<f64>)) {
    assert_eq!(
        drift.0, None,
        "a NaN IC's kernel path reports a drift of {:?}",
        drift.0
    );
    assert_eq!(
        drift.1, None,
        "a NaN IC's reference reports a drift of {:?}",
        drift.1
    );
}

#[test]
fn inspector_shared_kernel_nan_drift() {
    let got = inspector(T).inspect_state::<Rk4>(&nan_ic()).unwrap();
    println!(
        "NaN IC: {} path states, reference {:?} with {} states, drift {:?}",
        got.path.len(),
        got.reference.end,
        got.reference.states.len(),
        got.energy_drift()
    );
    assert!(nan_max(0.0, f64::NAN).is_nan(), "the reduction drops a NaN");
    check_no_drift(got.energy_drift());
}

negative_control!(
    inspector_shared_kernel_nan_drift,
    "the drift as the f64::max fold read it, which drops the NaN and reads 0",
    expected = "reports a drift of Some(0.0)",
    {
        let got = inspector(T).inspect_state::<Rk4>(&nan_ic()).unwrap();
        let e0 = engine::precision::rk45::energy(&got.path[0]);
        let old = |states: &[kernel::occupant::State<f64>]| {
            Some(
                states
                    .iter()
                    .map(|s| (engine::precision::rk45::energy(s) - e0).abs())
                    .fold(0.0, f64::max),
            )
        };
        check_no_drift((old(&got.path), old(&got.reference.states)))
    }
);
