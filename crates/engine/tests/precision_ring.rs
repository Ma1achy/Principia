//! The Precision ring's four entry points called natively (parity_contract §1): `computeIC` returns a
//! full-f64 `SimState`, the wrapper's own integration at f64; `computeQuad` returns the quad's samples, each
//! `computeIC`'s; `stepOnce` takes one `STEP` of any bound stepper, the same step the wrapper's seam takes, of the
//! occupant's order; `decodeOnly` returns `(m, r, p, ICDescriptor)` from the decode stage's source at f64.
//!
//! Pass criteria, fixed before the first run:
//! - bitwise equality where the property is "the same computation" (the wrapper at f64, the seam's step, the decode
//!   stage at f64, `computeIC` per sample);
//! - "full f64": at least one position or momentum component of the result is not an f32 value, which a result
//!   computed at f32 and widened never satisfies;
//! - the local order of `stepOnce`: `log₂(err(h)/err(h/2)) ≥ p + ½` for an occupant of order `p` (the asymptote is
//!   `p + 1`), the error measured against the RK45 reference at `rtol = 10⁻¹³`, `atol = 10⁻¹⁵`, from the
//!   figure-eight's start, at `h = 0.04` and `0.02`, and at `0.16` and `0.08` for Yoshida-6, whose error at `0.02`
//!   (2.5·10⁻¹⁵, the first run's) sits at the reference's own (1.6·10⁻¹⁵, measured against 4096 fixed Dormand–Prince
//!   steps): the test asserts that the error at `h/2` is at least 100 times the reference's own error there, so an
//!   order it reports is one the reference resolves.

use kernel::chart::Latent;
use kernel::decode::Ic;
use kernel::driver::advance::{Advance, AdvanceParams, Cadence};
use kernel::driver::wrapper::integrate_detecting;
use kernel::generated::links::Defaults;
use kernel::occupant::euler::Euler;
use kernel::occupant::kdk::Kdk;
use kernel::occupant::rk4::Rk4;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::yoshida6::Yoshida6;
use kernel::occupant::{DebugOccupant, Geom, Occupant, Selection, State};
use kernel::stage::decode::sample_with;

use engine::contract::sim_config::{Horizon, SimConfig, Window};
use engine::contract::sim_uniforms::{dt_macro_for, SimUniforms};
use engine::dispatch::validate::validate;
use engine::first_slice::Quad;
use engine::precision::rk45::{self, Rk45End, Rk45Params};
use engine::precision::{compute_ic, compute_quad, decode_only, step_once, SimState64, StepParams};
use engine::trajectory::FigureEight;
use validation::negative_control;

/// A generic latent chart (as `branch_read_gpu`'s).
fn latent() -> Latent<f32> {
    let eight = |k: f32| core::array::from_fn(|i| k * (i as f32 - 3.5) / 8.0);
    Latent {
        z0: eight(1.0),
        q1: eight(0.5),
        q2: eight(-0.25),
    }
}

/// The horizon the tests march to: short, so the debug build stays quick, and long enough to take many steps.
const T: f64 = 1.0;

fn quad() -> Quad<Latent<f32>> {
    Quad {
        chart: latent(),
        window: Window::SQUARE,
        size: [3, 2],
    }
}

fn key() -> SimConfig {
    SimConfig {
        horizon: Horizon { t: T },
        ..quad().sim_key()
    }
}

/// Every position and momentum component's bits, and every other field's, as text: equal texts are equal bits for
/// every non-NaN value.
fn bits(s: &SimState64) -> String {
    format!("{s:?}")
}

/// Whether some component of `r` or `p` is not an f32 value.
fn wider_than_f32(r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]) -> bool {
    r.iter()
        .chain(p)
        .flatten()
        .any(|&x| f64::from(x as f32) != x)
}

// ── computeIC ────────────────────────────────────────────────────────────────────────────────────────────────────

const UV: [f64; 2] = [0.3, 0.6];

/// The wrapper's own integration at f64 of the f64 decode at `UV`, built here from its parts.
fn wrapper_at_f64() -> SimState64 {
    let k = key();
    let out = sample_with::<Latent<f32>, Defaults, f64>(&latent(), UV[0], UV[1], None);
    let valid = validate(&SimUniforms {
        t_horizon: T,
        dt_macro: dt_macro_for(T),
        ..SimUniforms::default()
    })
    .unwrap();
    assert_eq!(k.horizon.t, T);
    integrate_detecting::<Rk4, f64>(
        &out.ic,
        &valid.wrapper_uniforms_f64(),
        valid.collision_radius(),
    )
}

fn check_compute_ic(got: &SimState64) {
    assert!(
        wider_than_f32(&got.r, &got.p),
        "computeIC's state is not full f64: every component is an f32 value"
    );
    assert_eq!(
        bits(got),
        bits(&wrapper_at_f64()),
        "computeIC is not the wrapper's integration at f64, bit for bit"
    );
}

#[test]
fn precision_ring_entry_points_compute_ic() {
    let got = compute_ic::<Latent<f32>, Defaults, Rk4>(&latent(), UV, &key()).unwrap();
    check_compute_ic(&got);
}

/// The same IC marched at f32 by the first slice's path and widened: the f32 instantiation, not the Precision ring.
#[cfg(feature = "controls")]
fn widened_f32_march() -> SimState64 {
    let out =
        sample_with::<Latent<f32>, Defaults, f32>(&latent(), UV[0] as f32, UV[1] as f32, None);
    let s = engine::first_slice::march::<Rk4>(&out.ic, &engine::first_slice::uniforms(T).unwrap());
    let w = |v: [[f32; 2]; 3]| v.map(|x| x.map(f64::from));
    SimState64 {
        r: w(s.r),
        p: w(s.p),
        ..SimState64::default()
    }
}

negative_control!(
    precision_ring_entry_points_compute_ic,
    "the f32 instantiation's state, widened, is not full f64",
    expected = "is not full f64",
    check_compute_ic(&widened_f32_march())
);

// ── computeQuad ──────────────────────────────────────────────────────────────────────────────────────────────────

fn check_compute_quad(got: &[SimState64]) {
    let q = quad();
    assert_eq!(
        got.len(),
        q.len(),
        "computeQuad returns {} samples, not the quad's {}",
        got.len(),
        q.len()
    );
    for (i, s) in got.iter().enumerate() {
        let want = compute_ic::<Latent<f32>, Defaults, Rk4>(&q.chart, q.uv(i), &key()).unwrap();
        assert_eq!(
            bits(s),
            bits(&want),
            "computeQuad's sample {i} is not computeIC at the quad's sample {i}"
        );
    }
}

#[test]
fn precision_ring_entry_points_compute_quad() {
    let got = compute_quad::<Latent<f32>, Rk4>(&quad(), &key()).unwrap();
    check_compute_quad(&got);
}

negative_control!(
    precision_ring_entry_points_compute_quad,
    "the samples in the wrong order",
    expected = "is not computeIC at the quad's sample",
    {
        let mut got = compute_quad::<Latent<f32>, Rk4>(&quad(), &key()).unwrap();
        got.reverse();
        check_compute_quad(&got)
    }
);

// ── decodeOnly ───────────────────────────────────────────────────────────────────────────────────────────────────

fn check_decode_only(ic: &Ic<f64>, descriptor_m: [f64; 3]) {
    assert!(
        wider_than_f32(&ic.r, &ic.p),
        "decodeOnly's (r, p) is not full f64: every component is an f32 value"
    );
    let want = sample_with::<Latent<f32>, Defaults, f64>(&latent(), UV[0], UV[1], None);
    assert_eq!(
        format!("{ic:?}"),
        format!("{:?}", want.ic),
        "decodeOnly's (m, r, p) is not the decode stage's at f64"
    );
    assert_eq!(
        descriptor_m, ic.m,
        "decodeOnly's ICDescriptor does not describe its (m, r, p)"
    );
}

#[test]
fn precision_ring_entry_points_decode_only() {
    let out = decode_only::<Latent<f32>, Defaults>(&latent(), UV, &key()).unwrap();
    let d = &out.descriptor;
    check_decode_only(&out.ic, [d.m0, d.m1, d.m2]);
}

negative_control!(
    precision_ring_entry_points_decode_only,
    "the decode at f32, widened",
    expected = "is not full f64",
    {
        let out =
            sample_with::<Latent<f32>, Defaults, f32>(&latent(), UV[0] as f32, UV[1] as f32, None);
        let ic = Ic {
            m: out.ic.m.map(f64::from),
            r: out.ic.r.map(|v| v.map(f64::from)),
            p: out.ic.p.map(|v| v.map(f64::from)),
        };
        check_decode_only(&ic, ic.m)
    }
);

// ── stepOnce ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// A cadence that keeps nothing and never ends the march.
struct Quiet;

impl Cadence<f64> for Quiet {
    fn after_step(&mut self, _: &mut State<f64>, _: &mut Geom<f64>) {}
    fn done(&self) -> bool {
        false
    }
}

fn start() -> State<f64> {
    let ic = FigureEight::new().start();
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}

/// The seam's step: `ADVANCE` over one macro-step of `dt` at `N_sub = 1`, the wrapper's call of the occupant `A`.
fn seam_step<A: Advance>(s: &State<f64>, dt: f64, n_sub: u32) -> State<f64> {
    let g = Geom::new(&s.m, &s.r);
    A::advance(
        s,
        &g,
        0,
        1,
        &AdvanceParams {
            dt_macro: dt,
            n_sub,
        },
        &mut Quiet,
    )
    .state
}

/// The largest absolute difference over positions and momenta.
fn distance(a: &State<f64>, b: &State<f64>) -> f64 {
    a.r.iter()
        .chain(&a.p)
        .flatten()
        .zip(b.r.iter().chain(&b.p).flatten())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// The reference's own error over one span `h` from `s`: the tight run against 4096 fixed Dormand–Prince steps.
fn reference_error(s: &State<f64>, h: f64, tight: &Rk45Params) -> f64 {
    let run = rk45::integrate(s, &[h], tight);
    let dp = rk45::Tableau::dormand_prince();
    let n = 4096;
    let mut fine = *s;
    for _ in 0..n {
        fine = rk45::fixed_step(&dp, &fine, h / f64::from(n)).0;
    }
    distance(&run.states[0], &fine)
}

/// `stepOnce` under `selection` at `dt` against the seam's step of `A` with `seam_n_sub` substeps, bitwise; then its
/// local order against the reference, at least `order + ½`, at an `h/2` the reference resolves.
fn check_step<A: Advance>(selection: Selection, order: u32, seam_n_sub: u32, dt: f64) {
    let s = start();
    let params = StepParams {
        occupant: selection,
    };
    let one = step_once(&s, dt, &params);
    assert_eq!(
        format!("{:?}", one.state),
        format!("{:?}", seam_step::<A>(&s, dt, seam_n_sub)),
        "stepOnce under {selection:?} is not the seam's one STEP"
    );
    assert_eq!(
        format!("{:?}", one.geom),
        format!("{:?}", Geom::new(&one.state.m, &one.state.r)),
        "stepOnce under {selection:?} does not return the force evaluation at its positions"
    );
    let tight = Rk45Params {
        rtol: 1e-13,
        atol: 1e-15,
        max_steps: 1_000_000,
    };
    let err = |h: f64| {
        let reference = rk45::integrate(&s, &[h], &tight);
        assert_eq!(reference.end, Rk45End::Reached);
        distance(&step_once(&s, h, &params).state, &reference.states[0])
    };
    let (e1, e2) = (err(dt), err(dt / 2.0));
    let floor = reference_error(&s, dt / 2.0, &tight);
    let measured = (e1 / e2).log2();
    println!(
        "stepOnce {selection:?}: local error {e1:.3e} at h = {dt}, {e2:.3e} at h/2 (reference's own {floor:.3e}), \
         order {measured:.2}"
    );
    assert!(
        e2 >= 100.0 * floor,
        "stepOnce under {selection:?}: its error at h/2, {e2:.3e}, is not resolved by the reference's {floor:.3e}"
    );
    assert!(
        measured >= f64::from(order) + 0.5,
        "stepOnce under {selection:?}: local order {measured:.2} is below {order} + ½"
    );
}

#[test]
fn precision_ring_entry_points_step_once() {
    check_step::<Kdk>(Selection::Production(Occupant::Kdk), 2, 1, 0.04);
    check_step::<Yoshida4>(Selection::Production(Occupant::Yoshida4), 4, 1, 0.04);
    check_step::<Yoshida6>(Selection::Production(Occupant::Yoshida6), 6, 1, 0.16);
    check_step::<Rk4>(Selection::Production(Occupant::Rk4), 4, 1, 0.04);
    check_step::<Euler>(Selection::Debug(DebugOccupant::Euler), 1, 1, 0.04);
}

negative_control!(
    precision_ring_entry_points_step_once,
    "the seam taking two half steps is not one STEP",
    expected = "is not the seam's one STEP",
    check_step::<Rk4>(Selection::Production(Occupant::Rk4), 4, 2, 0.04)
);

mod step_once_order {
    use super::*;

    /// The local-order check alone fires: Euler held to KDK's order 2.
    #[test]
    fn precision_ring_entry_points_step_once_order() {
        check_step::<Euler>(Selection::Debug(DebugOccupant::Euler), 1, 1, 0.04);
    }

    negative_control!(
        precision_ring_entry_points_step_once_order,
        "Euler, order 1, held to order 2",
        expected = "is below 2 + ½",
        check_step::<Euler>(Selection::Debug(DebugOccupant::Euler), 2, 1, 0.04)
    );
}

// ── A failed decode is labelled, not marched ─────────────────────────────────────────────────────────────────────

/// An IC with a NaN in a position: `decode_failed` at step 0, its separation latches unset, as the wrapper writes a
/// state it observed nothing of.
fn check_failed(got: &SimState64) {
    use kernel::payload::{
        sd_dmin_pair, sd_state, tm_t_end_step, SD_DMIN_PAIR_SENTINEL, STATE_DECODE_FAILED,
    };
    assert_eq!(
        sd_state(got.packed_a),
        STATE_DECODE_FAILED,
        "a non-finite IC is not decode_failed"
    );
    assert_eq!(tm_t_end_step(got.times), 0, "a non-finite IC was marched");
    assert_eq!(got.total_substeps, 0, "a non-finite IC took steps");
    assert_eq!(
        sd_dmin_pair(got.packed_a),
        SD_DMIN_PAIR_SENTINEL,
        "a non-finite IC's dmin_pair is not the sentinel"
    );
    let unset = kernel::payload::set_d_min_unset(got.packed_a);
    assert_eq!(got.packed_a, unset, "a non-finite IC's d_min is not unset");
}

fn non_finite() -> Ic<f64> {
    let mut ic = FigureEight::new().start();
    ic.r[0][0] = f64::NAN;
    ic
}

#[test]
fn precision_ring_entry_points_compute_ic_state_failed() {
    let got = engine::precision::compute_ic_state::<Rk4>(&non_finite(), &key()).unwrap();
    check_failed(&got);
}

negative_control!(
    precision_ring_entry_points_compute_ic_state_failed,
    "the label alone, its latches left at 0",
    expected = "dmin_pair is not the sentinel",
    {
        let label = kernel::detect::t0::t0(
            &non_finite(),
            None,
            &kernel::detect::collision::CollisionRadius::none(),
        );
        check_failed(&SimState64 {
            packed_a: label.packed_a(0),
            ..SimState64::default()
        })
    }
);
