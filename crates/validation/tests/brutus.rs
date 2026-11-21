//! The independent convergence reference (`validation::reference::brutus`; TASK-M3-25): its truth on the starts that
//! have one, its stop rule, its independence from the kernel, and the kernel's march measured against it.
//!
//! - `brutus_lagrange_rigid`: Lagrange's triangle at masses (0.2, 0.3, 0.5) converges and turns rigidly.
//! - `brutus_figure_eight_closure` (anchor): the figure-eight converges and closes after its period.
//! - `brutus_rk45_agrees`: the Precision ring's RK45 reference, an independent integrator in f64, at the figure-eight's
//!   period.
//! - `brutus_kernel_march`: the kernel's march at f64 through Path A, Yoshida-4 at the figure-eight's period and Heggie ×
//!   RK4, the default occupant, at the sync boundary it stops on, the last at or before the period.
//! - `brutus_refuses`: the stop rule gives no result without agreement, from a vacuous ladder, or at a precision that
//!   cannot resolve its tolerance.
//! - `brutus_independent_source` and `brutus_kernel_closure`: the reference names nothing but `std` and `dashu_float`,
//!   and neither `kernel` nor `engine` depends on the precision crates.
//! - `brutus_burrau` (ignored, 84 s in a debug build, measured): Burrau's lightest
//!   body leaves in the literature's window, on the converged reference.
//!
//! **Pass criteria, fixed before the first run**:
//! - Agreement: two successive rungs within `10⁻²⁰` ([`LADDER`]), far below f64's rounding.
//! - Lagrange at half a turn: within `10⁻¹⁵` of the rigid rotation (scaled as the agreement is: `|Δrᵢ|/R`, `|Δvᵢ|/V`),
//!   the closed form's own f64 evaluation the floor; the accepted rung's `|ΔE/E| ≤ 10⁻²⁰`.
//! - The figure-eight: closure `≤ 10⁻⁶` (the ground-truth note's anchor; the published digits' own closure is
//!   `4.1·10⁻⁸`), and `|ΔE/E| ≤ 10⁻²⁰`.
//! - RK45 (`rtol = 10⁻¹³`) against the reference at the period: `phys ≤ 10⁻¹⁰`. The kernel's march against it:
//!   `phys ≤ 10⁻⁸`, the bound the ground-truth suite holds the march to against RK45.
//! - Burrau: the lightest body unbound from a time in 60–65 of Szebehely & Peters' units on, and escaped at 100
//!   (`tiers::BURRAU_EJECTION_WINDOW`, the ground-truth suite's), on a ladder agreeing to `10⁻⁶`: the ejection is read on
//!   a 0.5 grid from the sign of a two-body energy, and six digits are four more than that reading needs.
//!
//! **The controls**, each making its test fail: a wrong force (`G = 1.0001`) on Lagrange, which converges as well as the
//! right one and is not rigid; a figure-eight start off by `10⁻⁵` in one velocity; RK45 at a loose tolerance; Euler's
//! first-order occupant for the kernel's march, and Heggie × RK4 read at the period rather than at its boundary; a sound ladder handed to the refusals; a `crate::` import added to the
//! source; a crate the kernel does depend on (`libm`) named as forbidden; a Burrau ladder with a ten-step budget.
//!
//! **Domain** (pitfalls §10): these starts, horizons and ladders; the reference's convergence on one start says
//! nothing of another's, which is why the ladder runs per start and per horizon.

use kernel::decode::Ic;
use kernel::driver::advance::Advance;
use kernel::generated::links::Defaults;
use kernel::occupant::yoshida4::Yoshida4;
use kernel::regularisation::sync::delta_sync;

use validation::ground_truth::path_a;
use validation::ground_truth::tiers::{self, REFERENCE};
use validation::ground_truth::{landing_key, DefaultOccupant};
use validation::import::{root, unit_state};
use validation::negative_control;
use validation::reference::brutus::{self, Converged, Ladder, NotConverged, Output, Start};
use validation::reference::brutus_gate::{
    extern_crates, figure_eight_case, from_ic, from_published, independence, kernel_closure,
    lagrange_case, Case, LADDER, PRECISION_CRATES, SOURCE,
};
use validation::roundtrip::phys;

/// The agreement-scaled truth bound for Lagrange (the module's note).
const LAGRANGE_TOL: f64 = 1e-15;
/// The figure-eight's closure bound.
const CLOSURE_TOL: f64 = 1e-6;
/// The accepted rung's energy error bound.
const ENERGY_TOL: f64 = 1e-20;
/// RK45 against the reference.
const RK45_TOL: f64 = 1e-10;
/// The kernel's march against the reference.
const MARCH_TOL: f64 = 1e-8;

/// The ladder on `case`, which must converge.
fn converged(case: &Case) -> Converged {
    match brutus::converge(&case.start, &case.times, &LADDER) {
        Ok(c) => c,
        Err(e) => panic!("the ladder did not converge: {e}"),
    }
}

/// The accepted run's truth and energy against their bounds.
fn check_truth(name: &str, case: &Case, bound: f64) {
    let c = converged(case);
    for line in brutus_gate_table(&c) {
        println!("{name}: {line}");
    }
    let last = c.accepted().outputs.last().expect("an output");
    let d = (case.distance)(last, &case.start);
    println!(
        "{name}: {} at {}: {d:e} (≤ {bound:e}); |ΔE/E| {:e}",
        case.truth,
        last.t,
        last.energy_error.abs()
    );
    assert!(
        *c.disagreement.last().unwrap() <= LADDER.threshold(),
        "{name}: accepted without agreement"
    );
    assert!(d <= bound, "{name}: {d:e} beyond {}", case.truth);
    assert!(
        last.energy_error.abs() <= ENERGY_TOL,
        "{name}: the accepted rung's energy error {:e} beyond {ENERGY_TOL:e}",
        last.energy_error
    );
}

fn brutus_gate_table(c: &Converged) -> Vec<String> {
    validation::reference::brutus_gate::table(&c.runs, &c.disagreement)
}

#[test]
fn brutus_lagrange_rigid() {
    check_truth("lagrange", &lagrange_case(), LAGRANGE_TOL);
}

negative_control!(
    brutus_lagrange_rigid,
    "a wrong force, G = 1.0001, converges as well as the right one and is not rigid",
    expected = "beyond the rigid rotation",
    {
        let mut case = lagrange_case();
        case.start.g = 1.0001;
        check_truth("lagrange, G = 1.0001", &case, LAGRANGE_TOL);
    }
);

#[test]
fn brutus_figure_eight_closure() {
    check_truth("figure_eight", &figure_eight_case(), CLOSURE_TOL);
}

negative_control!(
    brutus_figure_eight_closure,
    "a figure-eight start off by 1e-5 in one velocity does not close",
    expected = "beyond closure",
    {
        let mut case = figure_eight_case();
        case.start.w[0][0] += 1e-5;
        check_truth("figure_eight, perturbed", &case, CLOSURE_TOL);
    }
);

// ── Against the f64 integrators ──────────────────────────────────────────────────────────────────────────────────

/// A reference output as a kernel state, momenta `mᵢvᵢ` in f64.
fn ic_of_output(m: [f64; 3], o: &Output) -> Ic<f64> {
    Ic {
        m,
        r: o.r,
        p: [0, 1, 2].map(|i| [m[i] * o.v[i][0], m[i] * o.v[i][1]]),
    }
}

/// The figure-eight through Path A: its decoded start, its period in canonical time, and the march by `A` there.
fn figure_eight_march<A: Advance>(occupant: &'static str) -> (Ic<f64>, f64, Ic<f64>) {
    let s = tiers::figure_eight();
    let x = unit_state(&s).expect("the figure-eight's start");
    let c = path_a::check1::<Defaults>(&x);
    let period = tiers::figure_eight_period() * tiers::time_scale(&s, c.lambda);
    let key = landing_key(period, 1e-3).expect("the sim key validates");
    let run = path_a::run::<Defaults, A>(&x, &key, period, ("figure_eight", occupant, "defaults"))
        .expect("the forward leg");
    let end = Ic {
        m: run.forward.start.m,
        r: run.forward.state.r,
        p: run.forward.state.p,
    };
    (run.check1.decoded, period, end)
}

/// The reference from the kernel's own decoded start (its bits, momenta divided at the rung's precision) at `t`.
fn reference_at(start: &Ic<f64>, t: f64) -> Ic<f64> {
    let s: Start = from_ic(start);
    let c = brutus::converge(&s, &[t], &LADDER).unwrap_or_else(|e| panic!("the reference: {e}"));
    ic_of_output(start.m, c.accepted().outputs.last().unwrap())
}

fn check_rk45(params: &engine::precision::rk45::Rk45Params) {
    let (start, period, _) = figure_eight_march::<Yoshida4>("Yoshida-4");
    let reference = reference_at(&start, period);
    let rk45 = tiers::reference(&start, &[period], params).expect("RK45 reaches the period");
    let d = phys(&reference, &rk45[0]);
    println!("RK45 (rtol {:e}) against the reference at the figure-eight's period: phys {d:e} (≤ {RK45_TOL:e})", params.rtol);
    assert!(
        d <= RK45_TOL,
        "RK45 is {d:e} from the convergence reference at the period"
    );
}

#[test]
fn brutus_rk45_agrees() {
    check_rk45(&REFERENCE);
}

negative_control!(
    brutus_rk45_agrees,
    "RK45 at rtol = 1e-6 is not within 1e-10 of the reference",
    expected = "from the convergence reference",
    check_rk45(&engine::precision::rk45::Rk45Params {
        rtol: 1e-6,
        atol: 1e-8,
        ..REFERENCE
    })
);

/// Where a march by `A` stands when asked for `t`: `t` for an unregularised occupant; for a regularised one the sync
/// boundary at or before it, `⌊t/Δ_sync⌋·Δ_sync`, where `ADVANCE` stops by design (`kernel::regularisation::sync`).
fn stands_at(t: f64, regularised: bool) -> f64 {
    if regularised {
        let quantum: f64 = delta_sync();
        (t / quantum).floor() * quantum
    } else {
        t
    }
}

fn check_march<A: Advance>(occupant: &'static str, regularised: bool) {
    let (start, period, end) = figure_eight_march::<A>(occupant);
    let at = stands_at(period, regularised);
    let d = phys(&reference_at(&start, at), &end);
    println!(
        "{occupant}'s march against the reference at {at} (the figure-eight's period {period}): phys {d:e} (≤ \
         {MARCH_TOL:e})"
    );
    assert!(
        d <= MARCH_TOL,
        "{occupant}'s march is {d:e} from the convergence reference at {at}"
    );
}

#[test]
fn brutus_kernel_march() {
    check_march::<Yoshida4>("Yoshida-4", false);
    check_march::<DefaultOccupant>("Heggie × RK4", true);
}

negative_control!(
    brutus_kernel_march,
    "Euler's first-order occupant is not within 1e-8 of the reference",
    expected = "from the convergence reference",
    check_march::<kernel::occupant::euler::Euler>("Euler", false)
);

mod kernel_march_off_the_boundary {
    use super::*;
    negative_control!(
        brutus_kernel_march,
        "Heggie × RK4 read at the period, not at the sync boundary where it stands, is not within 1e-8",
        expected = "from the convergence reference",
        check_march::<DefaultOccupant>("Heggie × RK4", false)
    );
}

// ── The stop rule ────────────────────────────────────────────────────────────────────────────────────────────────

/// `ladder` on Lagrange must give no result; returns why.
fn check_refused(ladder: &Ladder) -> NotConverged {
    let case = lagrange_case();
    match brutus::converge(&case.start, &case.times, ladder) {
        Ok(c) => panic!(
            "the ladder gave a result at {} digits",
            c.accepted().rung.digits
        ),
        Err(e) => {
            println!("refused, as it must be: {e}");
            e
        }
    }
}

fn check_refusals(sound: bool) {
    let vacuous = Ladder {
        step: if sound { LADDER.step } else { 0 },
        ..LADDER
    };
    assert!(matches!(
        check_refused(&vacuous),
        NotConverged::Vacuous { .. }
    ));
    let spent = Ladder { rungs: 3, ..LADDER };
    assert!(matches!(check_refused(&spent), NotConverged::Spent { .. }));
    let held = Ladder {
        fixed_bits: Some(53),
        ..LADDER
    };
    assert!(matches!(
        check_refused(&held),
        NotConverged::Failed {
            failure: brutus::Failure::Unresolvable { .. },
            ..
        }
    ));
}

#[test]
fn brutus_refuses() {
    check_refusals(false);
}

negative_control!(
    brutus_refuses,
    "a sound ladder handed to the vacuous refusal converges",
    expected = "the ladder gave a result",
    check_refusals(true)
);

// ── Independence ─────────────────────────────────────────────────────────────────────────────────────────────────

fn check_source(extra: &str) {
    let mut source = std::fs::read_to_string(root().join(SOURCE)).unwrap();
    source.push_str(extra);
    let manifest = std::fs::read_to_string(root().join("crates/validation/Cargo.toml")).unwrap();
    let externs = extern_crates(&manifest);
    assert!(
        externs.iter().any(|e| e == "kernel") && externs.iter().any(|e| e == "dashu_float"),
        "the manifest's crates are not read: {externs:?}"
    );
    let found = independence(&source, &externs);
    assert!(
        found.is_empty(),
        "the reference reaches beyond std: {found:?}"
    );
}

#[test]
fn brutus_independent_source() {
    check_source("");
    // The scan's reading of the ways in, each of which it must see.
    let externs = vec![
        "kernel".to_owned(),
        "engine".to_owned(),
        "dashu_float".to_owned(),
    ];
    for (line, what) in [
        ("use kernel::physics;", "extern crate `kernel`"),
        (
            "fn f() { let _ = ::engine::precision::rk45::energy; }",
            "extern crate `engine`",
        ),
        ("use super::free_group;", "reaches the rest of the crate"),
        ("mod other;", "a `mod` declaration"),
        ("const S: &str = include_str!(\"x\");", "include_str!"),
        (
            "fn f() { negative_control!(x); }",
            "the macro `negative_control!`",
        ),
        ("use serde::Deserialize;", "`use serde`"),
    ] {
        let found = independence(line, &externs);
        assert!(
            found.iter().any(|f| f.contains(what)),
            "`{line}` not caught: {found:?}"
        );
    }
    // A doc, a string and a comparison are not ways in.
    let clean = "/// kernel::physics\nfn f(a: u8) -> bool { let _ = \"crate::x\"; if !(a != 1) { true } else { false } }";
    assert!(
        independence(clean, &externs).is_empty(),
        "a doc, a string or `!` read as a way in"
    );
}

negative_control!(
    brutus_independent_source,
    "the reference with a crate:: import added must fail the scan",
    expected = "reaches the rest of the crate",
    check_source("\nuse crate::ground_truth::tiers as _;\n")
);

fn check_closure(forbidden: &[&str]) {
    let found = kernel_closure(&root(), forbidden).expect("cargo tree runs");
    assert!(
        found.is_empty(),
        "the kernel shares a crate with the reference: {found:?}"
    );
}

#[test]
fn brutus_kernel_closure() {
    check_closure(PRECISION_CRATES);
}

negative_control!(
    brutus_kernel_closure,
    "a crate the kernel does depend on, libm, named as forbidden",
    expected = "depends on `libm`",
    check_closure(&["libm"])
);

// ── Burrau ───────────────────────────────────────────────────────────────────────────────────────────────────────

/// Burrau's ladder: agreement to `10⁻⁶` (the module's note).
const BURRAU_LADDER: Ladder = Ladder {
    agree: 6,
    max_steps: 5_000_000,
    ..LADDER
};

fn check_burrau(ladder: &Ladder) {
    let s = tiers::burrau();
    let start = from_published(&s);
    let times = tiers::ejection_samples();
    let t0 = std::time::Instant::now();
    let c = brutus::converge(&start, &times, ladder).unwrap_or_else(|e| panic!("Burrau: {e}"));
    for line in brutus_gate_table(&c) {
        println!("burrau: {line}");
    }
    println!("burrau: the ladder took {:?}", t0.elapsed());
    let samples: Vec<(f64, Ic<f64>)> = c
        .accepted()
        .outputs
        .iter()
        .map(|o| (o.t, ic_of_output(s.m, o)))
        .collect();
    let lightest = tiers::lightest(s.m);
    let t_e = tiers::ejection_time(&samples, lightest);
    let end = tiers::outcome(&samples.last().unwrap().1);
    println!(
        "burrau, the convergence reference: the lightest body unbound from {t_e:?} of S&P's units on (window {:?}); at \
         100: {end:?}",
        tiers::BURRAU_EJECTION_WINDOW
    );
    assert!(
        t_e.is_some_and(
            |t| (tiers::BURRAU_EJECTION_WINDOW[0]..=tiers::BURRAU_EJECTION_WINDOW[1]).contains(&t)
        ),
        "the lightest body is unbound from {t_e:?}, outside the Burrau ejection window"
    );
    assert_eq!(
        end.escaper,
        Some(lightest),
        "the lightest body has not escaped at 100"
    );
}

/// Ignored for its time: 84 s in a debug build (measured; four rungs, 12 to 24 digits, 1.77 million force evaluations).
#[test]
#[ignore = "the converged Burrau reference to 100 of S&P's units takes 84 s in a debug build"]
fn brutus_burrau() {
    check_burrau(&BURRAU_LADDER);
}

negative_control!(
    brutus_burrau,
    "a Burrau ladder whose budget is ten steps gives no result",
    expected = "Burrau: not converged",
    check_burrau(&Ladder {
        max_steps: 10,
        ..BURRAU_LADDER
    })
);
