//! Heggie's gates (TASK-M3-78): `radial-collision` (REQ-VAL-045), `hamiltonian-fd` (REQ-VAL-170, REQ-VAL-171) and
//! `separation-floor`'s Heggie arm (REQ-INT-105): their verdicts at and past their thresholds, their controls, their
//! refusals, their flags, and the runner on their fixtures. Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use kernel::driver::advance::Cadence;
use kernel::occupant::{Geom, State};
use kernel::regularisation::az::{self as kaz, AzP, AzQ};
use kernel::regularisation::heggie::{HgP, HgQ, HgWatch};
use kernel::regularisation::pairs::HeggieTime;
use serde_json::json;
use validation::az::FLOOR_NAME;
use validation::decode_f32_eps::SplitMix;
use validation::gate::{self, Gate, GateConfig, Verdict};
use validation::heggie::{
    az_at, central, closest, decades, decide_fd, decide_regions, describe, differs_arm,
    discrepancy, distinct, draw, fall_per_halving, fall_verdict, falls, floor_control_heggie,
    halving, heggie_at, heggie_hamiltonian, incomplete, max_or_nan, passage, passes_through,
    propose_fd, propose_floor_heggie, propose_regions, radial_horizon, relative, round_up_one,
    rung_fault, tiny_above, uniform, unreadable, Closest, Fall, FdOccupant, HamiltonianFdGate,
    HgRecorder, Occupant, Passage, RadialCollisionGate, Reader, Separations, D_MIN_BELOW,
    FALL_PER_HALVING, FD_NAME, RADIAL_NAME,
};
use validation::negative_control;

/// This workspace's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A scratch directory for reports.
fn out(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("heggie-gates-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn config(name: &str, threshold: f64) -> GateConfig {
    let mut c = gate::config(&root(), name).expect("the gate's gate.json");
    c.threshold.value = threshold;
    c
}

// ── radial-collision ───────────────────────────────────────────────────────────────────────────────────────────

/// A `radial-collision` input at `η = 0.01` on the fixtures' four-rung halving ladder.
fn radial_input(occupant: &str, reader: &str) -> serde_json::Value {
    json!({
        "description": "a test input", "occupant": occupant, "reader": reader, "eta": 0.01,
        "ladder": [0.01, 0.005, 0.0025, 0.00125]
    })
}

/// `falls` and the ladder's checks at their edges: a fall per halving of exactly `1/2` falls, a hair less does not; the
/// fall per halving reads every rung (a first-order fall reads `1`, a rise `−1`); fewer than two rungs never fall; a
/// halving ladder is refused unless each rung is exactly half the one before, positive and finite; and the proposal's
/// minimum is the least length every sub-ladder from the gate's `η` or finer holds at.
fn check_falls(f: impl Fn(&[f64]) -> bool) {
    assert!(
        f(&[1.0, 0.5, 0.5]),
        "a fall of exactly 1/2 per halving does not read as falling"
    );
    assert!(
        !f(&[1.0, 0.5, 0.5 * 1.000_001]),
        "a fall short of 1/2 per halving reads as falling"
    );
    assert_eq!(fall_per_halving(&[4.0, 2.0, 1.0]), 1.0);
    assert_eq!(fall_per_halving(&[1.0, 2.0, 4.0, 8.0]), -1.0);
    assert_eq!(
        fall_per_halving(&[8.0, 4.0, 2.0, 1.0, 8.0]),
        0.2,
        "every rung read"
    );
    assert!(fall_per_halving(&[1.0]).is_nan());
    assert!(f(&[2.0, 1.0]), "two rungs, a fall of 2");
    assert!(!f(&[1.0]), "one rung");
    assert!(!f(&[]), "no rung");
    assert_eq!(FALL_PER_HALVING, 0.5);
    assert!(halving(&[0.01, 0.005]) && halving(&[0.01, 0.005, 0.0025]));
    for bad in [
        &[0.01][..],
        &[0.01, 0.01],
        &[0.005, 0.01],
        &[0.01, 0.005, 0.005],
        &[0.0, 0.0],
        &[f64::INFINITY, f64::INFINITY],
        &[-0.01, -0.005],
    ] {
        assert!(!halving(bad), "{bad:?} reads as halving");
    }
    let v = [
        (0.02, 4, Fall::Falls),
        (0.01, 2, Fall::Falls),
        (0.01, 3, Fall::Flat),
        (0.005, 2, Fall::Flat),
        (0.005, 3, Fall::Flat),
    ];
    assert_eq!(
        decide_regions(&v, 0.01, 4),
        Some(3),
        "the least length that holds"
    );
    assert_eq!(
        decide_regions(&v, 0.02, 4),
        None,
        "the coarse start counted"
    );
    assert_eq!(decide_regions(&[(0.01, 3, Fall::Flat)], 0.01, 3), Some(2));
    assert_eq!(decide_regions(&[(0.01, 3, Fall::Falls)], 0.01, 3), None);
}

#[test]
fn radial_collision_falls() {
    check_falls(falls);
}

negative_control!(
    radial_collision_falls,
    "the old reading, the first and last rungs alone against 2^(n − 1)",
    expected = "a fall of exactly 1/2 per halving does not read as falling",
    check_falls(|x: &[f64]| x.len() > 1 && x[0] >= x[x.len() - 1] * 2f64.powi(x.len() as i32 - 1))
);

/// A region decision: `(start, length, verdict)` per sub-ladder, the gate's `η` and the longest length, to the minimum.
type Decide = fn(&[(f64, usize, Fall)], f64, usize) -> Option<usize>;

/// The proposal's minimum needs a sub-ladder measured at it: where every eligible sub-ladder of two rungs falls and the
/// ladder holds no eligible one of three, no minimum holds, not a vacuous three; one measured sub-ladder that does not
/// fall gives two. `decide` is the decision (the control passes the vacuous reading, which accepts an `n` it measured
/// nothing at).
fn check_regions_evidence(decide: Decide) {
    assert_eq!(
        decide(
            &[
                (0.02, 3, Fall::Falls),
                (0.02, 2, Fall::Falls),
                (0.01, 2, Fall::Falls)
            ],
            0.01,
            3
        ),
        None,
        "a minimum proposed with no sub-ladder measured at it"
    );
    assert_eq!(
        decide(&[(0.02, 3, Fall::Falls), (0.01, 2, Fall::Flat)], 0.01, 3),
        Some(2)
    );
}

#[test]
fn radial_collision_regions_evidence() {
    check_regions_evidence(decide_regions);
}

negative_control!(
    radial_collision_regions_evidence,
    "the vacuous reading, every eligible sub-ladder of none reading \"does not fall\"",
    expected = "a minimum proposed with no sub-ladder measured at it",
    check_regions_evidence(|v, eta_gate, longest| {
        (2..=longest).find(|&n| {
            v.iter()
                .filter(|&&(start, len, _)| start <= eta_gate && len >= n)
                .all(|&(_, _, f)| f == Fall::Flat)
        })
    })
);

/// A passage that ended marching at `t = 2` with the Cartesian read-out `cartesian`.
fn rung(cartesian: f64) -> Passage {
    Passage {
        d_min: 1e-30,
        d_image: 1e-16,
        regularised: 1e-14,
        cartesian,
        steps: 100,
        end: kaz::End::Marching,
        t: 2.0,
    }
}

/// The fall with `η` is undetermined, never "does not fall", where a read-out is not finite and positive: a NaN on any
/// rung, `0` on the coarsest or a finer one, `+∞` or a negative value, or fewer than two rungs; the least positive
/// read-outs still read, and a fall still falls. A rung whose read-out cannot be read is named, and an undetermined
/// sub-ladder is never evidence for the region minimum (REQ-VAL-080, REQ-VAL-357). `verdict` is the verdict (the
/// control passes the old reading, `falls` alone, under which a NaN or `−∞` slope reads "does not fall").
fn check_fall_undetermined(verdict: fn(&[f64]) -> Fall) {
    for i in 0..3 {
        let mut x = [1e-13; 3];
        x[i] = f64::NAN;
        assert_eq!(
            verdict(&x),
            Fall::Undetermined,
            "a NaN read-out at rung {i} not undetermined"
        );
    }
    for x in [
        [0.0, 1e-13, 1e-13],
        [1e-13, 1e-13, 0.0],
        [f64::INFINITY, 1e-13, 1e-13],
        [1e-13, -1e-13, 1e-13],
    ] {
        assert_eq!(verdict(&x), Fall::Undetermined, "{x:?} not undetermined");
    }
    assert_eq!(verdict(&[1e-13]), Fall::Undetermined, "one rung");
    assert_eq!(verdict(&[f64::MIN_POSITIVE, f64::MIN_POSITIVE]), Fall::Flat);
    assert_eq!(verdict(&[4.0, 2.0, 1.0]), Fall::Falls);
    let named = unreadable(0.005, &rung(f64::NAN)).expect("a NaN read-out named");
    assert!(named.contains("η = 0.005"), "{named}");
    assert!(
        unreadable(0.005, &rung(0.0)).is_some(),
        "a zero read-out read"
    );
    assert_eq!(unreadable(0.005, &rung(f64::MIN_POSITIVE)), None);
    assert_eq!(
        decide_regions(&[(0.01, 2, Fall::Undetermined)], 0.01, 2),
        None,
        "an undetermined sub-ladder taken as evidence"
    );
    assert_eq!(
        decide_regions(
            &[(0.01, 2, Fall::Undetermined), (0.01, 3, Fall::Flat)],
            0.01,
            3
        ),
        Some(3)
    );
}

#[test]
fn radial_collision_fall_undetermined() {
    check_fall_undetermined(fall_verdict);
}

negative_control!(
    radial_collision_fall_undetermined,
    "the old reading, `falls` alone, under which a NaN slope reads \"does not fall\"",
    expected = "a NaN read-out at rung 0 not undetermined",
    check_fall_undetermined(|x| if falls(x) { Fall::Falls } else { Fall::Flat })
);

/// A rung fault: the rung's `η`, its passage and the horizon, to why it cannot be read.
type RungFault = fn(f64, &Passage, f64) -> Option<String>;

/// A passage that did not end `Marching` at the horizon, `t = 2`, is undetermined and named: one that stopped
/// non-finite, spent its budget, ended at an event, or ended marching one ulp short of `t = 2`; a passage that ended
/// marching at `t = 2` with a readable read-out is read (R-566, REQ-VAL-080). `fault` is the rung's check (the control
/// passes the read-out check alone, which reads a passage however it ended).
fn check_incomplete(fault: RungFault) {
    let horizon = radial_horizon();
    assert_eq!(horizon, 2.0, "the radial collision's horizon");
    for end in [kaz::End::NonFinite, kaz::End::Exhausted, kaz::End::Done] {
        let mut p = rung(1e-13);
        p.end = end;
        let named =
            fault(0.005, &p, horizon).unwrap_or_else(|| panic!("a rung that ended {end:?} read"));
        assert!(named.contains("η = 0.005"), "{named}");
    }
    let mut short = rung(1e-13);
    short.t = horizon.next_down();
    assert!(
        fault(0.005, &short, horizon).is_some(),
        "a rung that stopped short of the horizon read"
    );
    assert_eq!(fault(0.005, &rung(1e-13), horizon), None);
    let named = incomplete("the passage at η = 0.01", &short, horizon).expect("named");
    assert!(named.contains("the passage at η = 0.01"), "{named}");
    assert_eq!(
        incomplete("the passage at η = 0.01", &rung(1e-13), horizon),
        None
    );
}

#[test]
fn radial_collision_incomplete() {
    check_incomplete(rung_fault);
}

negative_control!(
    radial_collision_incomplete,
    "the read-out check alone, which reads a passage however it ended",
    expected = "a rung that ended NonFinite read",
    check_incomplete(|eta, p, _horizon| unreadable(eta, p))
);

/// The read-outs' arm on the gate's own passage, here at `η = 0.005` on a ladder starting at `0.01`, so that no ladder
/// rung's check reads it: a NaN or infinite Cartesian read-out, or a NaN regularised drift, fails the arm under both
/// occupants with a line naming the passage, though `NaN != x`; finite read-outs that differ hold; equal ones hold
/// under Aarseth–Zare alone (REQ-VAL-080). `arm` is the arm (the control passes the plain `!=` reading).
fn check_differs_arm(arm: fn(f64, &Passage, Occupant) -> (bool, String)) {
    for occupant in [Occupant::Heggie, Occupant::AarsethZare] {
        for (cartesian, regularised) in
            [(f64::NAN, 1e-14), (f64::INFINITY, 1e-14), (2e-13, f64::NAN)]
        {
            let mut p = rung(cartesian);
            p.regularised = regularised;
            let (holds, line) = arm(0.005, &p, occupant);
            assert!(
                !holds,
                "{occupant:?}: a non-finite read-out ({cartesian:e}, {regularised:e}) passed the differs arm"
            );
            assert!(
                line.contains("η = 0.005") && line.contains("not both finite"),
                "{line}"
            );
        }
        let (holds, _) = arm(0.005, &rung(2e-13), occupant);
        assert!(holds, "{occupant:?}: differing finite read-outs failed");
    }
    let mut same = rung(1e-14);
    same.regularised = 1e-14;
    assert!(
        !arm(0.005, &same, Occupant::Heggie).0,
        "Heggie's equal read-outs held"
    );
    assert!(
        arm(0.005, &same, Occupant::AarsethZare).0,
        "Aarseth–Zare's equal read-outs failed"
    );
}

#[test]
fn radial_collision_differs_arm() {
    check_differs_arm(differs_arm);
}

negative_control!(
    radial_collision_differs_arm,
    "the plain `!=` reading, under which a NaN read-out differs",
    expected = "a non-finite read-out",
    check_differs_arm(|_eta, p, occupant| {
        (
            distinct(p.cartesian != p.regularised, occupant),
            "not both finite η = 0.005".to_owned(),
        )
    })
);

/// `radial-collision --propose` prints each occupant's passages, every sub-ladder's verdict and the decision.
fn check_regions(ladder: &[f64]) {
    let out = propose_regions(ladder, 0.01);
    for part in [
        "## Heggie + RK4",
        "## AarsethZare + RK4",
        "| 0.01 to 0.005 | 2 |",
        "| 0.005 to 0.0025 | 2 |",
        "| 0.01 to 0.0025 | 3 |",
        "Sub-ladders reading \"falls\"",
    ] {
        assert!(out.contains(part), "no `{part}` in {out}");
    }
    assert!(
        out.contains("**Proposed:** a region minimum of") || out.contains("**No minimum holds**"),
        "{out}"
    );
}

#[test]
fn radial_collision_propose() {
    check_regions(&[0.01, 0.005, 0.0025]);
    gate::run_flags(&root(), RADIAL_NAME, &["--propose"]).expect("the proposal");
}

negative_control!(
    radial_collision_propose,
    "a ladder one rung short",
    expected = "no `| 0.005 to 0.0025 | 2 |`",
    check_regions(&[0.01, 0.005])
);

/// Both occupants' passages go below `1e-10` and hold the regularised drift below `1e-12`, the Cartesian read-out after
/// the passage differing from it, and the read-outs' arm Heggie's alone; the gate passes just above the read drift and
/// fails at it; the Cartesian reader fails under both; the report prints the passage and each region of the ladder; an
/// unknown occupant or reader and a short ladder are refused; the runner gives each fixture its expected outcome.
/// `reader` is the gate's reader for the passing verdicts (the control points it at the Cartesian).
fn check_radial(reader: &str) {
    for (name, occupant) in [
        ("heggie", Occupant::Heggie),
        ("aarseth-zare", Occupant::AarsethZare),
    ] {
        let p = passage(occupant, 0.01);
        println!("radial-collision: {name}: {p:?}");
        assert!(p.d_min < D_MIN_BELOW, "{name}: d_min {:.3e}", p.d_min);
        assert!(
            p.regularised < 1e-12,
            "{name}: the regularised drift {:.3e}",
            p.regularised
        );
        if occupant == Occupant::Heggie {
            assert_ne!(
                p.cartesian, p.regularised,
                "{name}: the Cartesian read-out is the regularised drift"
            );
        }
        assert_eq!(Reader::Regularised.read(&p), p.regularised);
        assert_eq!(Reader::Cartesian.read(&p), p.cartesian);
        let at = |threshold: f64, reader: &str| {
            RadialCollisionGate
                .run(
                    &config(RADIAL_NAME, threshold),
                    "t",
                    &radial_input(name, reader),
                )
                .map(|r| r.verdict)
        };
        let read = p.regularised;
        assert_eq!(
            at(read.next_up(), reader),
            Ok(Verdict::Pass),
            "{name}: just above the read drift"
        );
        assert_eq!(
            at(read, reader),
            Ok(Verdict::Fail),
            "{name}: at the read drift"
        );
        // The reader check binds under both occupants: on this passage the two read-outs differ under each.
        assert_ne!(
            p.cartesian.to_bits(),
            p.regularised.to_bits(),
            "{name}: the read-outs coincide"
        );
        assert_eq!(
            at(1.0, "cartesian"),
            Ok(Verdict::Fail),
            "{name}: the Cartesian reader"
        );
        let report = RadialCollisionGate
            .run(
                &config(RADIAL_NAME, 1e-12),
                "t",
                &radial_input(name, reader),
            )
            .expect("a report");
        let line = describe(&p);
        assert!(
            line.contains(&format!(
                "d_min {:.3e} (the Cartesian image's {:.3e})",
                p.d_min, p.d_image
            )) && line.contains(&format!("the regularised drift {:.3e}", p.regularised))
                && line.contains(&format!(
                    "the Cartesian read-out after the passage {:.3e}",
                    p.cartesian
                ))
                && line.contains(&format!("{} steps", p.steps)),
            "{line}"
        );
        assert!(report.scatter[0].ends_with(&line), "{:?}", report.scatter);
        for eta in [0.01, 0.005, 0.0025, 0.00125] {
            let rung = format!(
                "the ladder's region at η = {eta}: {}",
                describe(&passage(occupant, eta))
            );
            assert!(report.scatter.contains(&rung), "{name}: no region {rung}");
        }
    }
    let refused = |input: serde_json::Value, reads: &str| {
        let e = RadialCollisionGate
            .run(&config(RADIAL_NAME, 1e-12), "t", &input)
            .expect_err("refused");
        assert!(e.contains(reads), "{e}");
    };
    refused(radial_input("logh", "regularised"), "no occupant `logh`");
    refused(radial_input("heggie", "other"), "no reader `other`");
    refused(
        json!({ "description": "", "occupant": "heggie", "reader": "regularised", "eta": 0.01, "ladder": [0.01] }),
        "two rungs or more",
    );
    let two = json!({ "description": "", "occupant": "heggie", "reader": "regularised", "eta": 0.01, "ladder": [0.01, 0.005] });
    assert!(
        RadialCollisionGate
            .run(&config(RADIAL_NAME, 1e-12), "t", &two)
            .is_ok(),
        "a ladder of two rungs refused"
    );
    assert!(
        distinct(true, Occupant::Heggie) && distinct(true, Occupant::AarsethZare),
        "differing read-outs"
    );
    assert!(
        distinct(false, Occupant::AarsethZare),
        "Aarseth–Zare's read-outs may coincide"
    );
    assert!(
        !distinct(false, Occupant::Heggie),
        "Heggie's read-outs coincide and pass"
    );
    assert_eq!(relative(3.0, 2.0), 0.5);
    assert_eq!(relative(1.0, -2.0), 1.5);
    gate::run(&root(), RADIAL_NAME, &out("radial")).expect("each input its expected outcome");
}

#[test]
fn radial_collision_gate() {
    check_radial("regularised");
}

negative_control!(
    radial_collision_gate,
    "the gate's reader pointed at the Cartesian read-out",
    expected = "just above the read drift",
    check_radial("cartesian")
);

/// An input whose `η` is not the ladder's first rung runs its own passage at that `η`, not the first rung's: the report's
/// passage line is the passage at `η = 0.005` on the ladder `[0.01, 0.005]`, under both occupants. `expected` is the `η`
/// whose passage the line must show (the control points it at the first rung).
fn check_eta_off_first_rung(expected: f64) {
    for (name, occupant) in [
        ("heggie", Occupant::Heggie),
        ("aarseth-zare", Occupant::AarsethZare),
    ] {
        let own = describe(&passage(occupant, 0.005));
        assert_ne!(
            own,
            describe(&passage(occupant, 0.01)),
            "{name}: the two rungs' passages read alike"
        );
        let input = json!({
            "description": "", "occupant": name, "reader": "regularised", "eta": 0.005, "ladder": [0.01, 0.005]
        });
        let report = RadialCollisionGate
            .run(&config(RADIAL_NAME, 1e-12), "t", &input)
            .expect("a report");
        assert_eq!(
            report.scatter[0],
            format!(
                "{occupant:?} + RK4 at η = 0.005: {}",
                describe(&passage(occupant, expected))
            ),
            "{name}: the passage at η = 0.005 is not the one reported"
        );
    }
}

#[test]
fn radial_collision_eta_off_first_rung() {
    check_eta_off_first_rung(0.005);
}

negative_control!(
    radial_collision_eta_off_first_rung,
    "the passage expected at the ladder's first rung, not the input's η",
    expected = "the passage at η = 0.005 is not the one reported",
    check_eta_off_first_rung(0.01)
);

/// The gate refuses an input whose own `η`, apart from the ladder's, is not finite and positive, before any passage runs:
/// a passage at `η = 0` would never advance. The control offers a working `η` as one to refuse.
fn check_eta_refused(etas: &[f64]) {
    for &eta in etas {
        let input = json!({
            "description": "", "occupant": "heggie", "reader": "regularised", "eta": eta, "ladder": [0.01, 0.005]
        });
        match RadialCollisionGate.run(&config(RADIAL_NAME, 1e-12), "t", &input) {
            Err(e) => assert!(e.contains("is not finite and positive"), "{e}"),
            Ok(_) => panic!("η = {eta} is not refused"),
        }
    }
}

#[test]
fn radial_collision_eta_refused() {
    check_eta_refused(&[0.0, -0.01]);
}

negative_control!(
    radial_collision_eta_refused,
    "a working η, 0.005, offered as one to refuse",
    expected = "is not refused",
    check_eta_refused(&[0.005])
);

// ── hamiltonian-fd ─────────────────────────────────────────────────────────────────────────────────────────────

/// A `hamiltonian-fd` input over 8 states.
fn fd_input(occupant: &str, flip: bool, samples: u32) -> serde_json::Value {
    json!({ "description": "a test input", "occupant": occupant, "step": 1e-5, "samples": samples, "seed": 7, "flip": flip })
}

/// Each occupant's rates against the finite difference: the gate passes at the measured discrepancy and fails just
/// below it; the flipped sign fails; an unknown occupant and no samples are refused; the proposal's decision takes the
/// least row (the first on a tie) at ten times its discrepancy rounded up to one digit; the runner gives each fixture its
/// expected outcome. `flip` is the gate's input for the passing verdicts (the control flips it).
fn check_fd(flip: bool) {
    for (name, o) in [
        ("heggie-eq20", FdOccupant::HeggieEq20),
        ("heggie-eq22", FdOccupant::HeggieEq22),
        ("aarseth-zare", FdOccupant::AarsethZare),
    ] {
        assert_eq!(FdOccupant::parse(name), Some(o));
        let d = discrepancy(o, 1e-5, 8, 7, false);
        let flipped = discrepancy(o, 1e-5, 8, 7, true);
        println!("hamiltonian-fd: {name}: {d:.3e}, flipped {flipped:.3e}");
        assert!(
            d < 1e-6 && flipped > 0.5,
            "{name}: {d:.3e}, flipped {flipped:.3e}"
        );
        let at = |threshold: f64, flip: bool| {
            HamiltonianFdGate
                .run(&config(FD_NAME, threshold), "t", &fd_input(name, flip, 8))
                .map(|r| r.verdict)
        };
        assert_eq!(at(d, flip), Ok(Verdict::Pass), "{name}: at the discrepancy");
        assert_eq!(
            at(d.next_down(), flip),
            Ok(Verdict::Fail),
            "{name}: below it"
        );
        assert_eq!(at(1e-6, true), Ok(Verdict::Fail), "{name}: flipped");
    }
    assert_eq!(FdOccupant::parse("nothing"), None);
    for (input, reads) in [
        (fd_input("nothing", false, 8), "no occupant `nothing`"),
        (fd_input("heggie-eq20", false, 0), "no samples"),
    ] {
        let e = HamiltonianFdGate
            .run(&config(FD_NAME, 1.0), "t", &input)
            .expect_err("refused");
        assert!(e.contains(reads), "{e}");
    }
    let near = |x: f64, y: f64| ((x - y) / y).abs() < 1e-12;
    assert!(
        near(round_up_one(1.2e-9), 2e-9) && near(round_up_one(0.31), 0.4),
        "rounded up to one digit"
    );
    assert_eq!(round_up_one(3.0), 3.0, "a single digit kept");
    let (step, tolerance) =
        decide_fd(&[(1e-3, 4e-8), (1e-4, 2.4e-9), (1e-5, 2.4e-9), (1e-6, 5e-9)])
            .expect("a decision");
    assert!(
        step == 1e-4 && near(tolerance, 3e-8),
        "the least row, the first on a tie: {step:e}, {tolerance:e}"
    );
    let (step, tolerance) = decide_fd(&[(1e-3, 1.2e-9), (1e-4, 2.2e-9)]).expect("a decision");
    assert!(
        step == 1e-3 && near(tolerance, 2e-8),
        "the first row least: {step:e}, {tolerance:e}"
    );
    gate::run(&root(), FD_NAME, &out("fd")).expect("each input its expected outcome");
}

#[test]
fn hamiltonian_fd_gate() {
    check_fd(false);
}

negative_control!(
    hamiltonian_fd_gate,
    "the gate's input with a sign flipped",
    expected = "at the discrepancy",
    check_fd(true)
);

/// A proposal decision: rows `(step, largest discrepancy)` to the step and tolerance, or the steps not finite.
type DecideFd = fn(&[(f64, f64)]) -> Result<(f64, f64), Vec<f64>>;

/// The proposal proposes nothing where any row's discrepancy is not finite, and names its step: a NaN in the first row
/// or in a later one, or an infinite row, never passed over or carried into the tolerance; finite rows still decide;
/// and `propose_fd` over a step too small to move a component prints no proposal, naming it (REQ-VAL-080). `decide` is
/// the decision (the control passes the least-row choice that skips a NaN row).
fn check_fd_rows(decide: DecideFd) {
    for (table, steps) in [
        (vec![(1e-3, f64::NAN), (1e-4, 2e-9)], vec![1e-3]),
        (
            vec![(1e-3, 4e-8), (1e-4, f64::NAN), (1e-5, 2e-9)],
            vec![1e-4],
        ),
        (vec![(1e-3, 4e-8), (1e-4, f64::INFINITY)], vec![1e-4]),
        (vec![(1e-3, f64::NAN), (1e-4, f64::NAN)], vec![1e-3, 1e-4]),
    ] {
        assert_eq!(
            decide(&table),
            Err(steps),
            "a non-finite row passed over: {table:?}"
        );
    }
    let (step, tolerance) = decide(&[(1e-3, 4e-8), (1e-4, 2e-9)]).expect("finite rows decide");
    assert!(
        step == 1e-4 && ((tolerance - 2e-8) / 2e-8).abs() < 1e-12,
        "{step:e}, {tolerance:e}"
    );
    let out = propose_fd(&[1e-5, 1e-300], 2, 7);
    assert!(
        out.contains("**No proposal:** the discrepancy at step 1e-300 is not finite.")
            && !out.contains("**Proposed:**"),
        "{out}"
    );
}

#[test]
fn hamiltonian_fd_rows_non_finite() {
    check_fd_rows(decide_fd);
}

negative_control!(
    hamiltonian_fd_rows_non_finite,
    "the least-row choice, which passes over a NaN row",
    expected = "a non-finite row passed over",
    check_fd_rows(|table| {
        let mut best = table[0];
        for &row in &table[1..] {
            if row.1 < best.1 {
                best = row;
            }
        }
        Ok((best.0, round_up_one(10.0 * best.1)))
    })
);

/// The discrepancy's reductions keep a non-finite sample: `max_or_nan` is NaN where either operand is, and the larger
/// otherwise; at step `zero` every central difference is `0/0`, so every occupant's discrepancy, flipped or not, is NaN,
/// not the `0` a NaN-dropping `f64::max` leaves; the gate refuses a step of `0` or below; and at a step too small to
/// move any component (`1e-300`) the gate fails on the non-finite discrepancy (REQ-VAL-080). The control offers a
/// working step as `zero`.
fn check_fd_non_finite(zero: f64) {
    assert!(
        max_or_nan(f64::NAN, 1.0).is_nan() && max_or_nan(1.0, f64::NAN).is_nan(),
        "a NaN dropped by the reduction"
    );
    assert_eq!(max_or_nan(1.0, 2.0), 2.0);
    assert_eq!(max_or_nan(2.0, 1.0), 2.0);
    for o in FdOccupant::ALL {
        for flip in [false, true] {
            let d = discrepancy(o, zero, 2, 7, flip);
            assert!(
                d.is_nan(),
                "{o:?}: the discrepancy at step {zero:e} is not NaN: {d:e}"
            );
        }
    }
    for step in [0.0, -1e-5] {
        let input = json!({ "description": "", "occupant": "heggie-eq20", "step": step, "samples": 2, "seed": 7, "flip": false });
        let e = HamiltonianFdGate
            .run(&config(FD_NAME, 1.0), "t", &input)
            .expect_err("a step of 0 or below refused");
        assert!(e.contains("is not finite and positive"), "{e}");
    }
    let input = json!({ "description": "", "occupant": "heggie-eq20", "step": 1e-300, "samples": 2, "seed": 7, "flip": false });
    let report = HamiltonianFdGate
        .run(&config(FD_NAME, 1.0), "t", &input)
        .expect("a report");
    assert_eq!(report.verdict, Verdict::Fail, "a NaN discrepancy passed");
    assert!(
        report.negative_results[0].contains("is not finite"),
        "{:?}",
        report.negative_results
    );
}

#[test]
fn hamiltonian_fd_non_finite() {
    check_fd_non_finite(0.0);
}

negative_control!(
    hamiltonian_fd_non_finite,
    "a working step, 1e-5, offered as the step every difference reads 0/0 at",
    expected = "is not NaN",
    check_fd_non_finite(1e-5)
);

/// The gate's flags: `hamiltonian-fd` and `radial-collision` take `--propose` alone.
fn check_flags(name: &str, args: &[&str], reads: &str) {
    let e = gate::run_flags(&root(), name, args).expect_err("the flags were not refused");
    assert!(e.contains(reads), "{e}");
}

#[test]
fn heggie_gate_flags_refused() {
    check_flags(FD_NAME, &["--nothing"], "usage: --propose");
    check_flags(FD_NAME, &["--propose", "x"], "unrecognised flags");
    check_flags(RADIAL_NAME, &["--nothing"], "usage: --propose");
}

negative_control!(
    heggie_gate_flags_refused,
    "a flag the gate takes",
    expected = "the flags were not refused",
    check_flags(FD_NAME, &["--propose"], "")
);

// ── separation-floor's Heggie arm ──────────────────────────────────────────────────────────────────────────────

/// Heggie's floor control: a `TINY` ten times the passage's least pre-floor `dt/dτ` floors steps, the reference's none,
/// another is refused; the proposal carries Heggie's table; the runner gives each control its expected outcome.
fn check_floor(tiny: &str) {
    let (md, floored) = floor_control_heggie("t", "a test input", tiny).expect("the control");
    assert!(floored, "the floor binds above the least value: {md}");
    let (_, floored) = floor_control_heggie("t", "", "reference").expect("the control");
    assert!(!floored, "the reference's TINY floors a step");
    assert!(floor_control_heggie("t", "", "other")
        .unwrap_err()
        .contains("no TINY `other`"));
    let proposal = propose_floor_heggie();
    assert!(
        proposal.contains("## Heggie's arm") && proposal.contains("| Eq. 22 | f64 |"),
        "{proposal}"
    );
    gate::run(&root(), FLOOR_NAME, &out("floor")).expect("each control its expected outcome");
}

#[test]
fn separation_floor_heggie() {
    check_floor("above");
}

negative_control!(
    separation_floor_heggie,
    "the reference's TINY as the control that must bind",
    expected = "the floor binds above the least value",
    check_floor("reference")
);

/// The `"above"` control's `TINY` is ten times the least pre-floor `dt/dτ`, exactly. `tiny` is the rule checked (the
/// control adds the factor rather than multiplying by it). A quick check: a wrong factor gives a `TINY` near or past
/// `10`, and the control's march, which has no step budget, then runs for hours.
fn check_tiny_above(tiny: impl Fn(f64) -> f64) {
    for least in [2.0, 1e-12, 3.5e-200] {
        assert_eq!(
            tiny(least).to_bits(),
            (10.0 * least).to_bits(),
            "TINY is not ten times the least value {least:e}"
        );
    }
}

#[test]
fn separation_floor_tiny_above() {
    check_tiny_above(tiny_above);
}

negative_control!(
    separation_floor_tiny_above,
    "the factor added to the least value, not multiplied",
    expected = "TINY is not ten times the least value",
    check_tiny_above(|l| 10.0 + l)
);

// ── the helpers ────────────────────────────────────────────────────────────────────────────────────────────────

/// A state of three bodies at `r`, unit masses, at rest.
fn at_rest(r: [[f64; 2]; 3]) -> State<f64> {
    State {
        m: [1.0; 3],
        r,
        p: [[0.0; 2]; 3],
    }
}

/// The gates' helpers at their values: `closest` (the least pair separation, here `5`, the pair `(0, 1)`), the
/// `Closest` cadence keeping its least, `decades`, `passes_through` at its edge, the watches' finiteness tests, `uniform`
/// as the affine map of the generator's 53 bits, and `central` on a cubic, whose central difference is `3x² + δ²`.
fn check_helpers(closest: impl Fn(&[[f64; 2]; 3]) -> f64) {
    let near = [[1.0, 1.0], [4.0, 5.0], [20.0, 0.0]];
    let far = [[0.0, 0.0], [0.0, 30.0], [40.0, 0.0]];
    assert_eq!(closest(&near), 5.0, "the closest pair");
    assert_eq!(closest(&far), 30.0, "the closest pair, far");
    let mut cadence = Closest::default();
    assert_eq!(cadence.d_min, f64::INFINITY);
    for r in [near, far] {
        let mut s = at_rest(r);
        let mut g = Geom::new(&s.m, &s.r);
        Cadence::<f64>::after_step(&mut cadence, &mut s, &mut g);
        assert_eq!(cadence.d_min, 5.0, "the cadence keeps the least");
        assert!(!Cadence::<f64>::done(&cadence));
    }
    assert!(
        (decades(Some(1e-30), 1e-37) - 7.0).abs() < 1e-9,
        "seven decades"
    );
    assert!(decades(None, 1e-37).is_nan());
    assert!(!passes_through(D_MIN_BELOW), "d_min at the bound passes");
    assert!(passes_through(D_MIN_BELOW.next_down()));
    let hq = |x: f64| HgQ {
        q: [[x, 0.5], [0.25, 1.0], [1.0, 2.0]],
        t: 0.0,
    };
    let hp = HgP {
        p: [[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]],
    };
    let aq = |x: f64| AzQ {
        u1: [x, 0.5],
        u2: [0.25, 1.0],
        t: 0.0,
    };
    let ap = AzP {
        p1: [0.1, 0.2],
        p2: [0.3, 0.4],
    };
    for x in [1.0, f64::NAN] {
        let finite = x.is_finite();
        assert_eq!(
            HgWatch::finite(&mut HgRecorder::default(), &hq(x), &hp),
            finite
        );
        assert_eq!(
            HgWatch::finite(&mut Separations::default(), &hq(x), &hp),
            finite
        );
        assert_eq!(
            kaz::Watch::finite(&mut Separations::default(), &aq(x), &ap),
            finite
        );
    }
    let mut rng = SplitMix::new(7);
    let t = (rng.clone().next_u64() >> 11) as f64 / (1u64 << 53) as f64;
    assert_eq!(uniform(&mut rng, 2.0, 5.0), 2.0 + 3.0 * t, "uniform");
    let cube = |x: &[f64]| x[0] * x[0] * x[0];
    let d = central(&cube, &[3.0, 0.0], 0, 1e-2);
    assert!((d - (27.0 + 0.03 * 0.03)).abs() < 1e-9, "central: {d}");
}

#[test]
fn heggie_gate_helpers() {
    check_helpers(closest::<f64>);
}

negative_control!(
    heggie_gate_helpers,
    "a closest pair read with a sum where the difference belongs",
    expected = "the closest pair",
    check_helpers(|r: &[[f64; 2]; 3]| (r[0][0] + r[1][0]).hypot(r[0][1] - r[1][1]))
);

/// `az_at` builds the system the kernel's registration builds from the same state: the masses' factors `μ` and `K`, the
/// energy, the floor and the `(u, p)`, the reference body `0` (not in the longest pair, `(1, 2)`).
fn check_az_at(permute: bool) {
    let s = State {
        m: [1.0, 2.0, 3.0],
        r: [[0.0, 0.0], [1.0, 0.2], [-0.5, 1.3]],
        p: [[0.3, -0.1], [-0.2, 0.4], [-0.1, -0.3]],
    };
    let (system, q, p) = kaz::register(&s, 1e-300);
    assert_eq!(system.a, 0);
    let x = [
        q.u1[0], q.u1[1], q.u2[0], q.u2[1], p.p1[0], p.p1[1], p.p2[0], p.p2[1], system.e,
    ];
    let m = if permute {
        [system.mabc[1], system.mabc[0], system.mabc[2]]
    } else {
        system.mabc
    };
    let built = az_at(&x, m);
    assert_eq!(built.system, system, "the registration's system");
    assert_eq!(built.q, q);
    assert_eq!(built.p, p);
}

#[test]
fn heggie_fd_az_at() {
    check_az_at(false);
}

negative_control!(
    heggie_fd_az_at,
    "the masses permuted",
    expected = "the registration's system",
    check_az_at(true)
);

/// The finite-difference check's draws: each component on its range, both signs reached; Aarseth–Zare's `|R₃|` at least
/// `0.1`, by the kernel's own `|R₃|`; and each Heggie draw's Hamiltonian under its own time transformation, the masses
/// the draw's first three values.
fn check_draws(swap: bool) {
    for occupant in FdOccupant::ALL {
        // logH's draws, on their own ranges, are `logh_fd_draws`' (`tests/logh_gates.rs`).
        if matches!(occupant, FdOccupant::LogH | FdOccupant::LogHTtl) {
            continue;
        }
        let mut rng = SplitMix::new(11);
        let (n, ranges): (usize, [(f64, f64); 3]) = match occupant {
            FdOccupant::AarsethZare => (4, [(-1.2, 1.2), (-1.0, 1.0), (-3.0, 1.0)]),
            _ => (6, [(-1.2, 1.2), (-1.0, 1.0), (-3.0, 1.0)]),
        };
        let mut seen = [(f64::INFINITY, f64::NEG_INFINITY); 3];
        for _ in 0..2000 {
            let mut masses_rng = rng.clone();
            let masses = [0; 3].map(|_| uniform(&mut masses_rng, 0.3, 3.0));
            let d = draw(occupant, &mut rng);
            assert_eq!(d.x.len(), 2 * n + 1);
            for (i, &v) in d.x.iter().enumerate() {
                let g = (i / n).min(2);
                assert!(
                    v >= ranges[g].0 && v <= ranges[g].1,
                    "{occupant:?}: x[{i}] = {v}"
                );
                seen[g] = (seen[g].0.min(v), seen[g].1.max(v));
            }
            match occupant {
                FdOccupant::AarsethZare => {
                    let at = az_at(&d.x, masses);
                    let r3 = at.system.r3(&at.q);
                    assert!(r3 >= 0.1, "{occupant:?}: |R₃| = {r3}");
                }
                FdOccupant::LogH | FdOccupant::LogHTtl => unreachable!("skipped above"),
                FdOccupant::HeggieEq20 | FdOccupant::HeggieEq22 => {
                    let eq20 = (occupant == FdOccupant::HeggieEq20) != swap;
                    let time = if eq20 {
                        HeggieTime::Eq20
                    } else {
                        HeggieTime::Eq22
                    };
                    let at = heggie_at(&d.x, masses, time);
                    assert_eq!(
                        (d.hamiltonian)(&d.x),
                        heggie_hamiltonian(&at.system, &at.q, &at.p),
                        "{occupant:?}: the draw's Hamiltonian"
                    );
                }
            }
        }
        for (g, (lo, hi)) in seen.iter().enumerate() {
            assert!(
                *lo < ranges[g].0 / 2.0 && *hi > ranges[g].1 / 2.0,
                "{occupant:?}: group {g} spans {lo}..{hi}"
            );
        }
    }
}

#[test]
fn heggie_fd_draws() {
    check_draws(false);
}

negative_control!(
    heggie_fd_draws,
    "Heggie's time transformations swapped",
    expected = "the draw's Hamiltonian",
    check_draws(true)
);

/// `hamiltonian-fd --propose` runs and prints its proposal: the table, the criterion, the control and the step.
fn check_propose(args: &[&str]) {
    assert!(
        gate::run_flags(&root(), FD_NAME, args).is_ok(),
        "the proposal's flag refused"
    );
    let out = propose_fd(&[1e-5, 1e-6], 4, 7);
    for part in [
        "| step | Heggie Eq. 20 | Heggie Eq. 22 | Aarseth–Zare | logH | logH-TTL | largest |",
        "| 1e-5 |",
        "| 1e-6 |",
        "The control, one momentum rate's sign flipped",
        "**Proposed:** step 1e-",
    ] {
        assert!(out.contains(part), "no `{part}` in {out}");
    }
}

#[test]
fn hamiltonian_fd_propose() {
    check_propose(&["--propose"]);
}

negative_control!(
    hamiltonian_fd_propose,
    "a flag the gate does not take",
    expected = "the proposal's flag refused",
    check_propose(&["--nothing"])
);
