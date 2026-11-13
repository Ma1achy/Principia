//! qa for TASK-M3-72: the displacement form of the FTLE shadow as dd_integrator §3.8 defines it (R-974, R-72;
//! REQ-VAL-038), written from the definition, not from the code.
//! - `qa_m372_start_exact` (§3.8 "Start"): the displacement form starts at `δ⃗₀ = ‖δ₀‖·u`, each component exact
//!   (`±‖δ₀‖/2` on the positions `u` moves, zero elsewhere), so `x₀ + δ⃗₀` is the offset `offset(x₀, ‖δ₀‖)` bit for bit;
//!   at f32 and f64, at ‖δ₀‖ = 3e-3, 1e-5 and 1e-20 (far below half an ulp of the state, where `x'₀ − x₀` would be
//!   zero), on Burrau, the figure-eight and Lagrange at (0.5, 0.3, 0.2), in both (a1) and (a2). Control: the absolute
//!   form's `δ⃗₀ = x'₀ − x₀`, which loses the offset at 1e-20 at f32, fails.
//! - `qa_m372_shadow_own_com` (§3.8 "Its projection", §3.4): after a run in the displacement form at unequal masses,
//!   the shadow `x + δ⃗` has its own centre of mass and total momentum at zero, to rounding: `δ⃗` is projected by the same
//!   linear map as `x`. `u` puts the shadow's CoM `‖δ₀‖·(m₀ − m₁, m₁ − m₂)/(2M)` off the sample's at the start, so a
//!   projection of `x` alone leaves it there. Control: the unprojected start, `offset(x₀, ‖δ₀‖)`, fails.
//! - `qa_m372_detector_watches_shadow` (§3.8 "Its energy and collision test"; R-972 (d)): in the displacement form the
//!   shadow's detector reads `x + δ⃗`, not `x`: on a close pair that `u` brings closer, its least `d²` at `r_coll = 0`
//!   is below the sample's; at an `r_coll` between the two it fires, and the shadow marches on, its `r_sh`, `p_sh`
//!   and `S` bit for bit those at `r_coll = 0`, the sample's fields the base tier's and its FTLE finite (it only
//!   watches). Control: a detector that read the sample's `x` (its least `d²` the sample's own) fails.
//! - `qa_m372_displacement_is_chosen` (TASK-M3-72's recorded choice, rule 3: (a1)): `Displacement` with no parameter
//!   is `Displacement<Plain>`. Control: `Displacement<CancellationFree>` fails.
//!
//! **Domain** (pitfalls §10): Yoshida-4, the `shadow-form` gate's occupant, at `r_coll = 0` except in the detector test.

use std::any::TypeId;

use kernel::decode::Ic;
use kernel::detect::collision::CollisionRadius;
use kernel::driver::benettin::{
    integrate_ftle_with, offset, Benettin, BenettinParams, CancellationFree, Displacement, Plain,
    ShadowForm,
};
use kernel::driver::cadence::WrapperCadence;
use kernel::driver::wrapper::{integrate_detecting, integrate_with_co, WrapperReal};
use kernel::occupant::yoshida4::Yoshida4;
use kernel::occupant::State;
use kernel::payload::{sd_state, tm_t_end_step, STATE_COLLISION};
use validation::drift::{ic_at, sim_key, AsR};
use validation::measure::benettin::{burrau, figure_eight, lagrange, Orbit};
use validation::negative_control;

/// A precision the tests run at.
trait P: WrapperReal + Into<f64> + std::fmt::Debug + 'static {}
impl P for f32 {}
impl P for f64 {}

fn state<R: Copy>(ic: &Ic<R>) -> State<R> {
    State {
        m: ic.m,
        r: ic.r,
        p: ic.p,
    }
}

fn bits<R: Into<f64> + Copy>(s: &State<R>) -> Vec<u64> {
    s.r.iter()
        .chain(s.p.iter())
        .flatten()
        .map(|&v| v.into().to_bits())
        .collect()
}

fn orbits() -> [Orbit; 3] {
    [
        burrau(),
        figure_eight(1.0),
        lagrange("Lagrange, (0.5, 0.3, 0.2)", [0.5, 0.3, 0.2], 1.0),
    ]
}

// ── qa_m372_start_exact (§3.8 "Start") ─────────────────────────────────────────────────────────────────────────────

/// The start's `δ⃗₀` of a form, at the sample's `x₀`.
type Start<R> = fn(&Ic<R>, BenettinParams<R>) -> State<R>;

fn start_of<R: P, F: ShadowForm>(ic: &Ic<R>, prm: BenettinParams<R>) -> State<R> {
    Benettin::<R, F>::new(ic, CollisionRadius::none(), prm).offset_from(&state(ic))
}

/// `δ⃗₀` is `‖δ₀‖·u` exactly (§3.8: `u` is `+½` on body 0's x, `−½` on body 1's x, `+½` on body 1's y, `−½` on body 2's
/// y, zero elsewhere), and `x₀ + δ⃗₀` is `offset(x₀, ‖δ₀‖)` bit for bit, for `start` at `R`.
fn check_start_at<R: P>(name: &str, start: Start<R>)
where
    f64: AsR<R>,
{
    for orbit in orbits() {
        let ic = ic_at::<R>(&orbit.ic);
        let x = state(&ic);
        for d0 in [3e-3, 1e-5, 1e-20] {
            let d: R = d0.to_r();
            let prm = BenettinParams {
                delta_0: d,
                n_renorm: 1,
            };
            let delta = start(&ic, prm);
            let h: f64 = d.into() / 2.0;
            let want = [[h, 0.0], [-h, h], [0.0, -h]];
            for (i, row) in want.iter().enumerate() {
                for (k, &w) in row.iter().enumerate() {
                    let got: f64 = delta.r[i][k].into();
                    assert!(
                        got.to_bits() == w.to_bits(),
                        "{name}, {}, ‖δ₀‖ = {d0:e}: δ⃗₀ r[{i}][{k}] = {got:e}, not ‖δ₀‖·u = {w:e}",
                        orbit.name
                    );
                    let p: f64 = delta.p[i][k].into();
                    assert!(
                        p.to_bits() == 0,
                        "{name}, {}, ‖δ₀‖ = {d0:e}: δ⃗₀ p[{i}][{k}] = {p:e}, not ‖δ₀‖·u = 0",
                        orbit.name
                    );
                }
            }
            let shadow = State {
                m: x.m,
                r: [0, 1, 2].map(|i| [0, 1].map(|k| x.r[i][k] + delta.r[i][k])),
                p: [0, 1, 2].map(|i| [0, 1].map(|k| x.p[i][k] + delta.p[i][k])),
            };
            assert!(
                bits(&shadow) == bits(&offset(&x, d)),
                "{name}, {}, ‖δ₀‖ = {d0:e}: x₀ + δ⃗₀ is not the offset, bit for bit",
                orbit.name
            );
        }
    }
}

fn check_start(f32_start: Start<f32>, f64_start: Start<f64>) {
    check_start_at::<f32>("f32", f32_start);
    check_start_at::<f64>("f64", f64_start);
}

#[test]
fn qa_m372_start_exact() {
    check_start(
        start_of::<f32, Displacement<Plain>>,
        start_of::<f64, Displacement<Plain>>,
    );
    check_start(
        start_of::<f32, Displacement<CancellationFree>>,
        start_of::<f64, Displacement<CancellationFree>>,
    );
}

negative_control!(
    qa_m372_start_exact,
    "a δ⃗₀ formed as x'₀ − x₀, which loses an offset below the state's ulp, must fail",
    expected = "not ‖δ₀‖·u",
    check_start(
        start_of::<f32, kernel::driver::benettin::Absolute>,
        start_of::<f64, kernel::driver::benettin::Absolute>,
    )
);

// ── qa_m372_shadow_own_com (§3.8 "Its projection") ─────────────────────────────────────────────────────────────────

/// The shadow `x + δ⃗` at the end of a displacement-form run on Lagrange at (0.5, 0.3, 0.2), with the run's sample.
fn shadow_after_run() -> (State<f64>, State<f64>) {
    let orbit = lagrange("Lagrange, (0.5, 0.3, 0.2)", [0.5, 0.3, 0.2], 2.0);
    let ic = ic_at::<f64>(&orbit.ic);
    let key = sim_key::<f64>(orbit.horizon);
    let prm = BenettinParams {
        delta_0: 3e-3,
        n_renorm: 50,
    };
    let (out, b) =
        integrate_ftle_with::<Yoshida4, f64, Displacement>(&ic, &key, CollisionRadius::none(), prm);
    assert!(
        tm_t_end_step(out.times) > 100,
        "fixture: the run is too short"
    );
    let x = State {
        m: ic.m,
        r: out.r,
        p: out.p,
    };
    (b.shadow(&x), x)
}

/// `|R_com|` and `|P_com|` of `s`, componentwise maxima.
fn com(s: &State<f64>) -> (f64, f64) {
    let m = s.m;
    let total = m[0] + m[1] + m[2];
    let r = [0, 1].map(|k| (m[0] * s.r[0][k] + m[1] * s.r[1][k] + m[2] * s.r[2][k]) / total);
    let p = [0, 1].map(|k| s.p[0][k] + s.p[1][k] + s.p[2][k]);
    (r[0].abs().max(r[1].abs()), p[0].abs().max(p[1].abs()))
}

/// The shadow's own CoM and momentum are zero to rounding. The bound, 1e-12, is f64 rounding on states of order one
/// with a wide margin; an unprojected `δ⃗` keeps its CoM at `‖δ₀‖·(m₀ − m₁)/(2M)` = 3e-4 here, eight decades above.
fn check_own_com(shadow: &State<f64>) {
    let (r, p) = com(shadow);
    assert!(
        r <= 1e-12 && p <= 1e-12,
        "the shadow x + δ⃗ is not projected by its own R_com, P_com: |R_com| = {r:e}, |P_com| = {p:e}"
    );
}

#[test]
fn qa_m372_shadow_own_com() {
    let (shadow, x) = shadow_after_run();
    let (rx, px) = com(&x);
    assert!(
        rx <= 1e-12 && px <= 1e-12,
        "fixture: the sample is off its CoM frame"
    );
    check_own_com(&shadow);
}

negative_control!(
    qa_m372_shadow_own_com,
    "a shadow whose δ⃗ is not projected (the unprojected start offset) must fail",
    expected = "not projected by its own R_com",
    {
        let orbit = lagrange("Lagrange, (0.5, 0.3, 0.2)", [0.5, 0.3, 0.2], 2.0);
        let x = state(&ic_at::<f64>(&orbit.ic));
        check_own_com(&offset(&x, 3e-3))
    }
);

// ── qa_m372_detector_watches_shadow (§3.8; R-972 (d)) ──────────────────────────────────────────────────────────────

/// A close pair, bodies 0 and 1 at separation 0.03 on the x axis in a near-circular orbit, body 2 far off: `u` moves
/// body 0 by `+½‖δ₀‖` and body 1 by `−½‖δ₀‖` along the pair's axis, so the shadow's pair starts `‖δ₀‖` closer.
fn close_pair() -> Ic<f64> {
    let p = (2.0f64 / 3.0 / 0.03).sqrt() / 2.0 / 3.0;
    Ic {
        m: [1.0 / 3.0; 3],
        r: [[-0.015, 0.0], [0.015, 0.0], [0.0, 1.0]],
        p: [[0.0, -p], [0.0, p], [0.0, 0.0]],
    }
}

fn pair_params() -> BenettinParams<f64> {
    BenettinParams {
        delta_0: 1e-3,
        n_renorm: 10,
    }
}

/// The sample's and the displacement-form shadow's least `d²` over a run at `r_coll = 0`, each its own detector's.
fn least_d2() -> (f32, f32) {
    let ic = close_pair();
    let key = sim_key::<f64>(1.0);
    let mut cadence = WrapperCadence::new(&ic.m, &ic.r, &ic.p);
    let mut shadow =
        Benettin::<f64, Displacement>::new(&ic, CollisionRadius::none(), pair_params());
    integrate_with_co::<Yoshida4, f64, _, _>(&ic, &key, &mut cadence, &mut shadow);
    (
        cadence.detector().latch().d2(),
        shadow.cadence().detector().latch().d2(),
    )
}

fn fields(r: &[[f64; 2]; 3], p: &[[f64; 2]; 3]) -> Vec<u64> {
    r.iter()
        .chain(p.iter())
        .flatten()
        .map(|v| v.to_bits())
        .collect()
}

/// `(sample_d2, shadow_d2)` the least `d²` the sample's and the shadow's detectors latched at `r_coll = 0`.
fn check_watches(sample_d2: f32, shadow_d2: f32) {
    assert!(
        shadow_d2 < sample_d2,
        "the shadow's detector does not read x + δ⃗: its least d² {shadow_d2} is not below the sample's {sample_d2}"
    );
    let ic = close_pair();
    let key = sim_key::<f64>(1.0);
    let radius = CollisionRadius::from_squared(0.5 * (shadow_d2 + sample_d2));
    let (free, _) = integrate_ftle_with::<Yoshida4, f64, Displacement>(
        &ic,
        &key,
        CollisionRadius::none(),
        pair_params(),
    );
    let (s, b) =
        integrate_ftle_with::<Yoshida4, f64, Displacement>(&ic, &key, radius, pair_params());
    assert!(
        b.cadence().detector().fired(),
        "the shadow's detector did not fire at an r_coll only the shadow comes inside"
    );
    assert!(
        fields(&s.r_sh, &s.p_sh) == fields(&free.r_sh, &free.p_sh)
            && s.S.to_bits() == free.S.to_bits(),
        "the shadow stopped at its own terminal: r_sh, p_sh or S differ from the run at r_coll = 0"
    );
    let base = integrate_detecting::<Yoshida4, f64>(&ic, &key, radius);
    assert!(
        fields(&s.r, &s.p) == fields(&base.r, &base.p)
            && s.packed_a == base.packed_a
            && s.times == base.times
            && sd_state(s.packed_a) != STATE_COLLISION,
        "the shadow's terminal reached the sample"
    );
    assert!(s.S.is_finite(), "the shadow's sum {} is not finite", s.S);
}

#[test]
fn qa_m372_detector_watches_shadow() {
    let (sample, shadow) = least_d2();
    check_watches(sample, shadow);
}

negative_control!(
    qa_m372_detector_watches_shadow,
    "a shadow detector that read the sample's x, not x + δ⃗, must fail",
    expected = "does not read x + δ⃗",
    {
        let (sample, _) = least_d2();
        check_watches(sample, sample)
    }
);

// ── qa_m372_displacement_is_chosen (the recorded choice) ───────────────────────────────────────────────────────────

fn check_chosen(chosen: TypeId) {
    assert!(
        TypeId::of::<Displacement>() == chosen,
        "`Displacement` does not name the form TASK-M3-72's measurement chose, (a1)"
    );
}

#[test]
fn qa_m372_displacement_is_chosen() {
    check_chosen(TypeId::of::<Displacement<Plain>>());
}

negative_control!(
    qa_m372_displacement_is_chosen,
    "the form the measurement did not choose, (a2), must fail",
    expected = "does not name the form",
    check_chosen(TypeId::of::<Displacement<CancellationFree>>())
);
