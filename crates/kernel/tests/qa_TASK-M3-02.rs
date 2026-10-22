//! QA tests for TASK-M3-02, written from the requirements it closes, not from the implementation:
//! - REQ-INT-014: the occupant table, transcribed here from integrator_contract Part 2 and Part 2a, against what each
//!   built occupant advertises and binds by default; controls: Yoshida-4 `step_reversible` false, KDK
//!   `run_reversible` true.
//! - REQ-INT-015: Euler is in no tier default and no production list, and is the debug selection's.
//! - REQ-INT-013, REQ-INT-078: the profile's readers return Part 2's definition (drift, the two reversibility gates,
//!   cost, the error exponent and FTLE confidence), each read from the profile.
//! - REQ-INT-037: the shipped constants parse from dd_integrator §3.2's printed digits; Yoshida-4's `w₁` is the
//!   correctly rounded f64 of `1/(2 − 2^{1/3})`, checked exactly; `w₀` is bitwise `1 − 2w₁` and `w₀ + 2w₁ == 1.0`
//!   (R-515); Yoshida-6's `w₀ = 1 − 2(w₁ + w₂ + w₃)`; the kicks are `(d_{i−1} + dᵢ)/2`; controls: RQ-405's interim
//!   digits, and `w₁`'s f64 neighbour.
//! - REQ-INT-056: the Yoshida blocks carry Yoshida (1990)'s citation, Yoshida-6 its Table 1, solution A; the negative
//!   weights are Yoshida-4's `w₀` and Yoshida-6's `w₁`.
//! - REQ-INT-090: the order conditions, evaluated exactly in rational arithmetic on the shipped f64 constants, the
//!   nested fifth-order one by an independent BCH expansion in the free algebra truncated at weight 5; and the
//!   convergence order on a Kepler-embedded fixture of qa's own, with the 4th-significant-digit controls.
//! - REQ-VAL-032, REQ-VAL-191: dd §5 test 3 per dt over a dense grid of `dt/τ` inside the criterion's domain, on qa's
//!   own fixtures, at f64 and f32: KDK, Yoshida-4 and Yoshida-6 at or below `step_reversal_tol`, and the asymmetric
//!   controls (RK4, and Yoshida-4 with its stages made asymmetric `(w₁, w₁, w₀)`) above it at each dt, not only on the
//!   worst residual, over the closed domain `[edge, step_reversal_edge_max]` at 16 points per decade with both edges;
//!   the tolerance and the three edges are read from the register through `validation`; the fixtures include an
//!   approaching pair, whose step's conditioning the start's `τ`, from positions alone, does not bound. A second control takes the
//!   reversible steppers past the upper edge on Burrau's start, where they read above the criterion.

use std::collections::BTreeMap;

use kernel::occupant::table::{default_occupant, table, Binding, Built, Tier};
use kernel::occupant::{
    CapabilityProfile, DebugOccupant, Geom, Occupant, Selection, State, StepReal,
    StepperCoefficients,
};
use kernel::physics::energy::energy;
use num_bigint::BigInt;
use num_rational::BigRational;
use spirv_std::num_traits::{Signed, ToPrimitive, Zero};
use validation::{
    negative_control, STEP_REVERSAL_EDGE_F32, STEP_REVERSAL_EDGE_F64, STEP_REVERSAL_EDGE_MAX,
    STEP_REVERSAL_TOL,
};

// ── REQ-INT-014, REQ-INT-015: the table ──────────────────────────────────────────────────────────────────────────

/// A default binding as Part 2 / Part 2a's last column gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Default_ {
    Tiers(Tier, Tier),
    Validation,
    Debug,
    CloseEncounter,
    Future,
}

/// One row as qa transcribes it: name, order, force evals, symplectic, step-reversible, run-reversible, owns time
/// mapping, default.
type Spec = (&'static str, u32, u32, bool, bool, bool, bool, Default_);

/// integrator_contract Part 2's five rows and Part 2a's two ("The table gains two rows"), transcribed by qa. Part 2's
/// steppers own no time mapping (the wrapper's model); both Aarseth–Zare rows do (Part 2a).
const SPEC: [Spec; 7] = [
    ("Euler", 1, 1, false, false, false, false, Default_::Debug),
    (
        "KDK",
        2,
        1,
        true,
        true,
        false,
        false,
        Default_::Tiers(Tier::Potato, Tier::Low),
    ),
    (
        "Yoshida-4",
        4,
        3,
        true,
        true,
        false,
        false,
        Default_::Tiers(Tier::Medium, Tier::High),
    ),
    (
        "Yoshida-6",
        6,
        7,
        true,
        true,
        false,
        false,
        Default_::Tiers(Tier::Ultra, Tier::Extreme),
    ),
    (
        "RK4",
        4,
        4,
        false,
        false,
        false,
        false,
        Default_::Validation,
    ),
    (
        "AZ+RK4",
        4,
        4,
        false,
        false,
        false,
        true,
        Default_::CloseEncounter,
    ),
    ("AZ+TTL", 2, 1, true, true, false, true, Default_::Future),
];

fn spec_profile(s: &Spec) -> CapabilityProfile {
    CapabilityProfile {
        order: s.1,
        force_evals: s.2,
        symplectic: s.3,
        step_reversible: s.4,
        run_reversible: s.5,
        owns_time_mapping: s.6,
    }
}

fn binding_of(b: Binding) -> Default_ {
    match b {
        Binding::Tiers { from, to } => Default_::Tiers(from, to),
        Binding::ValidationCrossCheck => Default_::Validation,
        Binding::DebugOnly => Default_::Debug,
        Binding::CloseEncounter => Default_::CloseEncounter,
        Binding::AllowedFuture => Default_::Future,
    }
}

/// What the build advertises, row by row in Part 2 / Part 2a's order: the profile each built occupant's own selection
/// returns (the table's for the two Aarseth–Zare rows, not built here), its default, and whether it is built.
fn advertised() -> Vec<(CapabilityProfile, Default_, bool)> {
    table()
        .iter()
        .map(|row| {
            let (profile, built) = match row.built {
                Built::Stepper(sel) => {
                    assert_eq!(
                        sel.profile(),
                        row.profile,
                        "{}'s selection advertises a profile other than its row's",
                        row.name
                    );
                    (sel.profile(), true)
                }
                Built::Regularised | Built::NotBuilt => (row.profile, false),
            };
            (profile, binding_of(row.binding), built)
        })
        .collect()
}

/// The advertised rows equal qa's transcription; the five steppers are built and AZ + TTL is not (R-168).
fn check_table(rows: &[(CapabilityProfile, Default_, bool)]) {
    assert_eq!(rows.len(), SPEC.len(), "the table has {} rows", rows.len());
    for (spec, (profile, default, _)) in SPEC.iter().zip(rows) {
        let want = spec_profile(spec);
        assert_eq!(
            want.step_reversible, profile.step_reversible,
            "{}'s step_reversible differs from Part 2",
            spec.0
        );
        assert_eq!(
            want.run_reversible, profile.run_reversible,
            "{}'s run_reversible differs from Part 2",
            spec.0
        );
        assert_eq!(*profile, want, "{}'s profile differs from Part 2", spec.0);
        assert_eq!(*default, spec.7, "{}'s default differs from Part 2", spec.0);
    }
    let built: Vec<bool> = rows.iter().map(|r| r.2).collect();
    assert_eq!(
        built,
        [true, true, true, true, true, false, false],
        "built rows"
    );
}

#[test]
fn qa_occupant_profiles_equal_part_2() {
    check_table(&advertised());
    // Each production occupant and the debug one, through its own enum, equal the transcription by name.
    let by_name: BTreeMap<&str, CapabilityProfile> =
        SPEC.iter().map(|s| (s.0, spec_profile(s))).collect();
    assert_eq!(Occupant::Kdk.profile(), by_name["KDK"]);
    assert_eq!(Occupant::Yoshida4.profile(), by_name["Yoshida-4"]);
    assert_eq!(Occupant::Yoshida6.profile(), by_name["Yoshida-6"]);
    assert_eq!(Occupant::Rk4.profile(), by_name["RK4"]);
    assert_eq!(DebugOccupant::Euler.profile(), by_name["Euler"]);
    // Exactly one run-reversible occupant exists, logH's TTL mode (R-162), which is not a stepper: none here.
    assert!(SPEC.iter().all(|s| !s.5));
}

negative_control!(
    qa_occupant_profiles_equal_part_2,
    "a table giving Yoshida-4 step_reversible false",
    expected = "Yoshida-4's step_reversible differs from Part 2",
    {
        let mut rows = advertised();
        rows[2].0.step_reversible = false;
        check_table(&rows);
    }
);

mod kdk_run_reversible {
    use super::*;
    negative_control!(
        qa_occupant_profiles_equal_part_2,
        "a table giving KDK run_reversible true",
        expected = "KDK's run_reversible differs from Part 2",
        {
            let mut rows = advertised();
            rows[1].0.run_reversible = true;
            check_table(&rows);
        }
    );
}

/// Each tier's default stepper is Part 2's (KDK Potato–Low, Yoshida-4 Medium–High, Yoshida-6 Ultra–Extreme), and no
/// tier default and no production occupant is Euler; Euler is the debug selection (REQ-INT-015).
fn check_defaults(default: impl Fn(Tier) -> Selection, production: &[Selection]) {
    let euler = DebugOccupant::Euler.profile();
    for t in Tier::ALL {
        let want = match t {
            Tier::Potato | Tier::Low => Occupant::Kdk,
            Tier::Medium | Tier::High => Occupant::Yoshida4,
            Tier::Ultra | Tier::Extreme => Occupant::Yoshida6,
        };
        let got = default(t);
        assert!(
            got.profile() != euler && !matches!(got, Selection::Debug(_)),
            "tier {t:?} defaults to Euler"
        );
        assert_eq!(got, Selection::Production(want), "tier {t:?}'s default");
    }
    for s in production {
        assert!(
            !matches!(s, Selection::Debug(_)) && s.profile() != euler,
            "the production list offers Euler"
        );
    }
}

fn production() -> Vec<Selection> {
    Occupant::PRODUCTION
        .iter()
        .map(|&o| Selection::Production(o))
        .collect()
}

#[test]
fn qa_occupant_euler_debug_only() {
    check_defaults(
        |t| Selection::Production(default_occupant(t)),
        &production(),
    );
    assert_eq!(DebugOccupant::DEBUG, [DebugOccupant::Euler]);
    assert_eq!(
        Selection::Debug(DebugOccupant::Euler).profile(),
        spec_profile(&SPEC[0])
    );
    assert!(!Selection::Debug(DebugOccupant::Euler).profile().symplectic);
    // The production list is exactly Part 2's other four steppers.
    assert_eq!(
        Occupant::PRODUCTION,
        [
            Occupant::Kdk,
            Occupant::Yoshida4,
            Occupant::Yoshida6,
            Occupant::Rk4
        ]
    );
}

negative_control!(
    qa_occupant_euler_debug_only,
    "a production list offering Euler",
    expected = "the production list offers Euler",
    {
        let mut p = production();
        p.push(Selection::Debug(DebugOccupant::Euler));
        check_defaults(|t| Selection::Production(default_occupant(t)), &p);
    }
);

mod euler_default {
    use super::*;
    negative_control!(
        qa_occupant_euler_debug_only,
        "Potato defaulting to Euler",
        expected = "tier Potato defaults to Euler",
        check_defaults(
            |t| match t {
                Tier::Potato => Selection::Debug(DebugOccupant::Euler),
                t => Selection::Production(default_occupant(t)),
            },
            &production()
        )
    );
}

// ── REQ-INT-013, REQ-INT-078: the readers ────────────────────────────────────────────────────────────────────────

/// What Part 2 says a feature reads from a profile, written from its text: drift is a dynamics diagnostic on a
/// symplectic occupant; the reversibility diagnostic gates on `step_reversible`, the reversibility test on
/// `run_reversible`; a run's cost is `total_substeps · force_evals`; the error exponent is `order`; FTLE confidence is
/// `(symplectic, saturated, order)`.
fn check_readers(exponent: impl Fn(&CapabilityProfile) -> u32) {
    for s in &SPEC {
        let p = spec_profile(s);
        assert_eq!(p.drift_reads_dynamics(), s.3, "{}: drift", s.0);
        assert_eq!(p.reversibility_diagnostic(), s.4, "{}: diagnostic", s.0);
        assert_eq!(p.reversibility_test(), s.5, "{}: test", s.0);
        assert_eq!(p.substeps_follow_wrapper(), !s.6, "{}: substep law", s.0);
        for n in [0u32, 1, 7, u32::MAX] {
            assert_eq!(
                p.force_evaluations(n),
                u64::from(n) * u64::from(s.2),
                "{}: cost of {n}",
                s.0
            );
        }
        assert_eq!(
            p.error_exponent(),
            exponent(&p),
            "{}: the error exponent is not the order",
            s.0
        );
        for sat in [false, true] {
            let f = p.ftle_confidence(sat);
            assert_eq!(
                (f.volume_preserving, f.saturated, f.error_exponent),
                (s.3, sat, s.1),
                "{}: FTLE confidence",
                s.0
            );
        }
    }
}

#[test]
fn qa_capability_profile_readers() {
    check_readers(|p| p.order);
    // Part 2 carries the definition the readers implement (REQ-INT-078).
    let contract = include_str!("../../../docs/contracts/principia_integrator_contract.md");
    let part2 = contract
        .split("## Part 2 — Occupants and the capability profile")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("Part 2");
    for needle in [
        "What the profile's `order` sets",
        "REQ-INT-078",
        "total_substeps_log2",
        "**FTLE confidence**",
    ] {
        assert!(part2.contains(needle), "Part 2 lacks {needle}");
    }
}

negative_control!(
    qa_capability_profile_readers,
    "an error exponent read as force_evals",
    expected = "the error exponent is not the order",
    check_readers(|p| p.force_evals)
);

// ── REQ-INT-037: the constants ───────────────────────────────────────────────────────────────────────────────────

fn parse(s: &str) -> f64 {
    s.replace('−', "-").parse().expect("a printed digit string")
}

/// `x` exactly, as a rational.
fn q(x: f64) -> BigRational {
    BigRational::from_float(x).expect("finite")
}

/// Yoshida-4's identity of R-515: `w₀` bitwise `1 − 2w₁` and `w₀ + 2w₁` exactly 1 in f64.
fn check_y4_identity(w1: f64, w0: f64) {
    assert_eq!(
        w0.to_bits(),
        (1.0 - 2.0 * w1).to_bits(),
        "w₀ is not 1 − 2w₁"
    );
    let sum = w0 + 2.0 * w1;
    assert!(sum == 1.0, "w₀ + 2w₁ = {sum:?}, not 1.0");
}

/// `w` is the correctly rounded f64 of `x = 1/(2 − 2^{1/3})`: `x` lies strictly between the midpoints of `w` with its
/// neighbours. `x ↦ 2 − 1/x` is increasing on `x > 0`, and so is cubing, so `lo < x < hi` iff
/// `(2 − 1/lo)³ < 2 < (2 − 1/hi)³`, in exact rationals.
fn check_correctly_rounded(w: f64) {
    let (below, above) = (
        f64::from_bits(w.to_bits() - 1),
        f64::from_bits(w.to_bits() + 1),
    );
    let two = BigRational::from_integer(BigInt::from(2));
    let lo = (q(below) + q(w)) / &two;
    let hi = (q(w) + q(above)) / &two;
    let c = |m: &BigRational| {
        let t = &two - m.recip();
        &t * &t * &t
    };
    assert!(
        c(&lo) < two && two < c(&hi),
        "w₁ = {w:?} is not 1/(2 − 2^(1/3)) correctly rounded"
    );
}

#[test]
fn qa_occupant_coefficients_yoshida4() {
    let (w1, w0) = (f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    assert_eq!(w1.to_bits(), parse("1.3512071919596575").to_bits());
    assert_eq!(w0.to_bits(), parse("−1.702414383919315").to_bits());
    check_y4_identity(w1, w0);
    check_correctly_rounded(w1);
    // The kicks [w₁/2, (w₀+w₁)/2, (w₀+w₁)/2, w₁/2].
    assert_eq!(f64::YOSHIDA4_KICK_1.to_bits(), (w1 / 2.0).to_bits());
    assert_eq!(f64::YOSHIDA4_KICK_2.to_bits(), ((w0 + w1) / 2.0).to_bits());
}

negative_control!(
    qa_occupant_coefficients_yoshida4,
    "RQ-405's interim digits, 1.3512071919596578 and −1.7024143839193157",
    expected = "0.9999999999999998, not 1.0",
    {
        let (w1, w0) = (parse("1.3512071919596578"), parse("−1.7024143839193157"));
        let sum = w0 + 2.0 * w1;
        assert!(
            sum == 0.9999999999999998,
            "the interim digits sum to {sum:?}"
        );
        // Their w₀ is not 1 − 2w₁ either; the sum is the requirement's named failure, checked first.
        assert!(sum == 1.0, "w₀ + 2w₁ = {sum:?}, not 1.0");
        check_y4_identity(w1, w0);
    }
);

mod y4_not_correctly_rounded {
    use super::*;
    negative_control!(
        qa_occupant_coefficients_yoshida4,
        "w₁'s f64 neighbour above",
        expected = "is not 1/(2 − 2^(1/3)) correctly rounded",
        check_correctly_rounded(f64::from_bits(f64::YOSHIDA4_W1.to_bits() + 1))
    );
}

mod y4_interim_not_correctly_rounded {
    use super::*;
    negative_control!(
        qa_occupant_coefficients_yoshida4,
        "RQ-405's interim w₁, 1.3512071919596578",
        expected = "is not 1/(2 − 2^(1/3)) correctly rounded",
        check_correctly_rounded(parse("1.3512071919596578"))
    );
}

/// Yoshida-6's weights: the printed `w₁`, `w₂`, `w₃`, and `w₀ = 1 − 2(w₁ + w₂ + w₃)` in f64, which §3.2 prints to 15
/// significant digits.
fn check_y6(w: [f64; 4]) {
    let [w1, w2, w3, w0] = w;
    assert_eq!(w1.to_bits(), parse("−1.17767998417887").to_bits(), "w₁");
    assert_eq!(w2.to_bits(), parse("0.235573213359357").to_bits(), "w₂");
    assert_eq!(w3.to_bits(), parse("0.784513610477560").to_bits(), "w₃");
    assert_eq!(
        w0.to_bits(),
        (1.0 - 2.0 * (w1 + w2 + w3)).to_bits(),
        "w₀ is not 1 − 2(w₁ + w₂ + w₃)"
    );
    // The printed w₀, 1.31518632068391, is w₀ to its 15 digits.
    assert!(
        (w0 - 1.31518632068391).abs() <= 0.5e-14,
        "w₀ = {w0:?} does not print as 1.31518632068391"
    );
}

#[test]
fn qa_occupant_coefficients_yoshida6() {
    let w = [
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        f64::YOSHIDA6_W0,
    ];
    check_y6(w);
    // The kicks cᵢ = (d_{i−1} + dᵢ)/2 of d = [w₃, w₂, w₁, w₀, w₁, w₂, w₃], d₀ = d₈ = 0.
    let d = [w[2], w[1], w[0], w[3], w[0], w[1], w[2]];
    let c: Vec<f64> = (0..8)
        .map(|i| {
            let a = if i == 0 { 0.0 } else { d[i - 1] };
            let b = if i == 7 { 0.0 } else { d[i] };
            (a + b) / 2.0
        })
        .collect();
    let shipped = [
        f64::YOSHIDA6_KICK_1,
        f64::YOSHIDA6_KICK_2,
        f64::YOSHIDA6_KICK_3,
        f64::YOSHIDA6_KICK_4,
    ];
    for i in 0..4 {
        assert_eq!(shipped[i].to_bits(), c[i].to_bits(), "kick {}", i + 1);
        assert_eq!(
            c[i].to_bits(),
            c[7 - i].to_bits(),
            "kick {} symmetric",
            i + 1
        );
    }
}

negative_control!(
    qa_occupant_coefficients_yoshida6,
    "w₀ taken as the printed 1.31518632068391 rather than computed",
    expected = "w₀ is not 1 − 2(w₁ + w₂ + w₃)",
    check_y6([
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3,
        1.31518632068391
    ])
);

mod y6_w3_digit {
    use super::*;
    negative_control!(
        qa_occupant_coefficients_yoshida6,
        "w₃ with its last printed digit changed",
        expected = "w₃",
        check_y6([
            f64::YOSHIDA6_W1,
            f64::YOSHIDA6_W2,
            0.784513610477561,
            f64::YOSHIDA6_W0
        ])
    );
}

// ── REQ-INT-056: the citation ────────────────────────────────────────────────────────────────────────────────────

/// `source` carries Yoshida (1990)'s citation, and `extra`.
fn check_citation(what: &str, source: &str, extra: &str) {
    // The text with its comment markers and markdown emphasis removed and its line breaks joined.
    let plain = source
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches("//!")
                .trim_start_matches("///")
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .replace('*', "");
    for needle in [
        "Yoshida, H. (1990)",
        "Construction of higher order symplectic integrators",
        "Phys. Lett. A 150, 262–268",
        extra,
    ] {
        assert!(
            plain.contains(needle),
            "{what} lacks the citation: {needle}"
        );
    }
}

#[test]
fn qa_yoshida6_published() {
    check_citation(
        "yoshida6.rs",
        include_str!("../src/occupant/yoshida6.rs"),
        "Table 1, solution A",
    );
    check_citation(
        "yoshida4.rs",
        include_str!("../src/occupant/yoshida4.rs"),
        "1 / (2 − 2^{1/3})",
    );
    // The section that prints the digits carries it too (R-501).
    let dd = include_str!("../../../docs/design/principia_dd_integrator.md");
    let s32 = dd
        .split("### 3.2 Occupants")
        .nth(1)
        .and_then(|s| s.split("### 3.3").next())
        .expect("§3.2");
    check_citation("dd §3.2", s32, "Table 1, solution A");
    // The negative weight's label (R-57): Yoshida-4's w₀ and Yoshida-6's w₁, the others positive.
    const {
        assert!(f64::YOSHIDA4_W0 < 0.0 && f64::YOSHIDA4_W1 > 0.0);
        assert!(f64::YOSHIDA6_W1 < 0.0);
        assert!(f64::YOSHIDA6_W0 > 0.0 && f64::YOSHIDA6_W2 > 0.0 && f64::YOSHIDA6_W3 > 0.0);
    }
    assert!(
        !include_str!("../../../docs/contracts/principia_integrator_contract.md")
            .lines()
            .any(|l| l.contains("w₂ < 0") && !l.contains("*Was:*"))
    );
}

negative_control!(
    qa_yoshida6_published,
    "the KDK block, which cites nothing",
    expected = "lacks the citation",
    check_citation(
        "kdk.rs",
        include_str!("../src/occupant/kdk.rs"),
        "Table 1, solution A"
    )
);

// ── REQ-INT-090: the order conditions, exactly ───────────────────────────────────────────────────────────────────

/// The Yoshida-4 bound and the Yoshida-6 bound (REQ-INT-090).
const Y4_BOUND: f64 = 3e-15;
const Y6_BOUND: f64 = 5e-14;

/// A non-commutative polynomial in `A₁`, `A₃`, `A₅` (letters 1, 3, 5, of weights 1, 3, 5), truncated at weight 5.
type Poly = BTreeMap<Vec<u8>, BigRational>;

fn weight(w: &[u8]) -> u32 {
    w.iter().map(|&l| u32::from(l)).sum()
}

fn add_into(a: &mut Poly, b: &Poly, k: &BigRational) {
    for (w, c) in b {
        let e = a.entry(w.clone()).or_insert_with(BigRational::zero);
        *e += c * k;
    }
    a.retain(|_, c| !c.is_zero());
}

fn mul(a: &Poly, b: &Poly) -> Poly {
    let mut out = Poly::new();
    for (wa, ca) in a {
        for (wb, cb) in b {
            if weight(wa) + weight(wb) <= 5 {
                let mut w = wa.clone();
                w.extend_from_slice(wb);
                let e = out.entry(w).or_insert_with(BigRational::zero);
                *e += ca * cb;
            }
        }
    }
    out.retain(|_, c| !c.is_zero());
    out
}

fn one() -> Poly {
    Poly::from([(Vec::new(), BigRational::from_integer(1.into()))])
}

fn int(n: i64) -> BigRational {
    BigRational::from_integer(n.into())
}

/// `exp(x)`, `x` without constant term, truncated at weight 5.
fn exp(x: &Poly) -> Poly {
    // term = x^k / k!
    let mut out = one();
    let mut term = one();
    for k in 1..=5 {
        let mut next = Poly::new();
        add_into(&mut next, &mul(&term, x), &(int(1) / int(k)));
        term = next;
        add_into(&mut out, &term, &int(1));
    }
    out
}

/// `log(y)`, `y` with constant term 1, truncated at weight 5.
fn log(y: &Poly) -> Poly {
    let mut z = y.clone();
    add_into(&mut z, &one(), &int(-1));
    let mut out = Poly::new();
    let mut term = one();
    for k in 1..=5i64 {
        term = mul(&term, &z);
        let sign = if k % 2 == 1 { 1 } else { -1 };
        add_into(&mut out, &term, &(int(sign) / int(k)));
    }
    out
}

/// The composition of symmetric second-order steps `S(wᵢh) = exp(wᵢA₁ + wᵢ³A₃ + wᵢ⁵A₅)` (the `h` powers carried by
/// the weights), its logarithm's coefficient of `[A₁,[A₁,A₃]] = A₁A₁A₃ − 2A₁A₃A₁ + A₃A₁A₁`, read off the word `A₁A₁A₃`,
/// which no other weight-5 Lie element in these letters contains (the only other is `A₅`).
fn nested(w: &[BigRational]) -> BigRational {
    let mut y = one();
    for wi in w {
        let w3 = wi * wi * wi;
        let w5 = &w3 * wi * wi;
        let x = Poly::from([(vec![1], wi.clone()), (vec![3], w3), (vec![5], w5)]);
        y = mul(&y, &exp(&x));
    }
    let l = log(&y);
    // The log of a symmetric composition is odd: no weight-2 or weight-4 terms (a check of the algebra itself).
    assert!(
        l.keys().all(|k| weight(k) % 2 == 1),
        "the composition's log has even terms"
    );
    let c113 = l
        .get(&vec![1, 1, 3])
        .cloned()
        .unwrap_or_else(BigRational::zero);
    let c131 = l
        .get(&vec![1, 3, 1])
        .cloned()
        .unwrap_or_else(BigRational::zero);
    let c311 = l
        .get(&vec![3, 1, 1])
        .cloned()
        .unwrap_or_else(BigRational::zero);
    assert!(
        c131 == -(int(2) * &c113) && c311 == c113,
        "the weight-5 part in A₁, A₃ is not a multiple of [A₁,[A₁,A₃]]"
    );
    c113
}

fn ratio(a: &BigRational, b: &BigRational) -> f64 {
    (a.abs() / b.abs()).to_f64().expect("finite")
}

/// The order conditions of the drift sequence `d`, exactly on its f64 values: `|Σw − 1|`, `|Σw³|/Σ|w|³`, and, when
/// `sixth`, `|Σw⁵|/Σ|w|⁵` and the nested one; each at most `bound`.
fn check_conditions(what: &str, d: &[f64], bound: f64, sixth: bool) -> Vec<f64> {
    let w: Vec<BigRational> = d.iter().map(|&x| q(x)).collect();
    let a: Vec<BigRational> = w.iter().map(|x| x.abs()).collect();
    let pow = |v: &[BigRational], n: u32| -> BigRational {
        v.iter()
            .map(|x| (0..n).fold(int(1), |acc, _| acc * x))
            .fold(BigRational::zero(), |acc, x| acc + x)
    };
    let mut r = vec![
        (pow(&w, 1) - int(1)).abs().to_f64().expect("finite"),
        ratio(&pow(&w, 3), &pow(&a, 3)),
    ];
    if sixth {
        r.push(ratio(&pow(&w, 5), &pow(&a, 5)));
        r.push(ratio(&nested(&w), &nested(&a)));
    }
    println!(
        "{what}: order-condition residuals {:?}",
        r.iter().map(|x| format!("{x:.3e}")).collect::<Vec<_>>()
    );
    for (i, x) in r.iter().enumerate() {
        assert!(
            *x <= bound,
            "{what}: condition {i} reads {x:.3e}, which exceeds {bound:e}"
        );
    }
    r
}

fn y4_d(w1: f64, w0: f64) -> Vec<f64> {
    vec![w1, w0, w1]
}

fn y6_d(w1: f64, w2: f64, w3: f64) -> Vec<f64> {
    let w0 = 1.0 - 2.0 * (w1 + w2 + w3);
    vec![w3, w2, w1, w0, w1, w2, w3]
}

#[test]
fn qa_yoshida_order_conditions() {
    // The digit mover the controls use moves the digit it names.
    assert!((moved(1.3512071919596575, 15, 1.0) - 1.3512071919596675).abs() < 1e-15);
    assert!((moved(-1.17767998417887, 13, -1.0) - -1.17767998417987).abs() < 1e-15);
    let r4 = check_conditions(
        "Yoshida-4",
        &y4_d(f64::YOSHIDA4_W1, f64::YOSHIDA4_W0),
        Y4_BOUND,
        false,
    );
    assert_eq!(r4[0], 0.0, "R-515: Σw = 1 exactly");
    let r6 = check_conditions(
        "Yoshida-6",
        &y6_d(f64::YOSHIDA6_W1, f64::YOSHIDA6_W2, f64::YOSHIDA6_W3),
        Y6_BOUND,
        true,
    );
    // The BCH expansion is not vacuous: on Yoshida-4, a fourth-order composition, the fifth-order conditions read
    // order 1, far from rounding.
    let w: Vec<BigRational> = y4_d(f64::YOSHIDA4_W1, f64::YOSHIDA4_W0)
        .iter()
        .map(|&x| q(x))
        .collect();
    let a: Vec<BigRational> = w.iter().map(|x| x.abs()).collect();
    let n4 = ratio(&nested(&w), &nested(&a));
    println!("Yoshida-4's nested fifth-order condition (not one of its conditions): {n4:.3e}");
    assert!(n4 > 1e-3, "the nested condition is vacuous");
    assert_eq!(r6.len(), 4);
}

/// `x` moved by `k` units of its `n`th significant digit.
fn moved(x: f64, n: i32, k: f64) -> f64 {
    let e = x.abs().log10().floor() as i32;
    x + k * 10f64.powi(e - n + 1)
}

macro_rules! condition_control {
    ($name:ident, $what:literal, $d:expr, $bound:expr, $sixth:expr) => {
        mod $name {
            use super::*;
            negative_control!(
                qa_yoshida_order_conditions,
                $what,
                expected = "which exceeds",
                {
                    check_conditions($what, &$d, $bound, $sixth);
                }
            );
        }
    };
}

// Yoshida-4: each coefficient one unit of its 15th significant digit, each way (w₀ not recomputed: recomputing it
// restores Σw = 1 and leaves Σw³ the only condition, which the 15th digit of w₁ moves by ~1e-14 too).
condition_control!(
    y4_w1_up,
    "Yoshida-4 w₁ +1 unit of digit 15",
    y4_d(moved(f64::YOSHIDA4_W1, 15, 1.0), f64::YOSHIDA4_W0),
    Y4_BOUND,
    false
);
condition_control!(
    y4_w1_down,
    "Yoshida-4 w₁ −1 unit of digit 15",
    y4_d(moved(f64::YOSHIDA4_W1, 15, -1.0), f64::YOSHIDA4_W0),
    Y4_BOUND,
    false
);
condition_control!(
    y4_w0_up,
    "Yoshida-4 w₀ +1 unit of digit 15",
    y4_d(f64::YOSHIDA4_W1, moved(f64::YOSHIDA4_W0, 15, 1.0)),
    Y4_BOUND,
    false
);
condition_control!(
    y4_w0_down,
    "Yoshida-4 w₀ −1 unit of digit 15",
    y4_d(f64::YOSHIDA4_W1, moved(f64::YOSHIDA4_W0, 15, -1.0)),
    Y4_BOUND,
    false
);
condition_control!(
    y4_w1_up_recomputed,
    "Yoshida-4 w₁ +1 unit of digit 15, w₀ recomputed",
    {
        let w1 = moved(f64::YOSHIDA4_W1, 15, 1.0);
        y4_d(w1, 1.0 - 2.0 * w1)
    },
    Y4_BOUND,
    false
);
// Yoshida-6: w₁, w₂, w₃ one unit of their 13th significant digit, each way, w₀ recomputed.
condition_control!(
    y6_w1_up,
    "Yoshida-6 w₁ +1 unit of digit 13",
    y6_d(
        moved(f64::YOSHIDA6_W1, 13, 1.0),
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3
    ),
    Y6_BOUND,
    true
);
condition_control!(
    y6_w1_down,
    "Yoshida-6 w₁ −1 unit of digit 13",
    y6_d(
        moved(f64::YOSHIDA6_W1, 13, -1.0),
        f64::YOSHIDA6_W2,
        f64::YOSHIDA6_W3
    ),
    Y6_BOUND,
    true
);
condition_control!(
    y6_w2_up,
    "Yoshida-6 w₂ +1 unit of digit 13",
    y6_d(
        f64::YOSHIDA6_W1,
        moved(f64::YOSHIDA6_W2, 13, 1.0),
        f64::YOSHIDA6_W3
    ),
    Y6_BOUND,
    true
);
condition_control!(
    y6_w2_down,
    "Yoshida-6 w₂ −1 unit of digit 13",
    y6_d(
        f64::YOSHIDA6_W1,
        moved(f64::YOSHIDA6_W2, 13, -1.0),
        f64::YOSHIDA6_W3
    ),
    Y6_BOUND,
    true
);
condition_control!(
    y6_w3_up,
    "Yoshida-6 w₃ +1 unit of digit 13",
    y6_d(
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        moved(f64::YOSHIDA6_W3, 13, 1.0)
    ),
    Y6_BOUND,
    true
);
condition_control!(
    y6_w3_down,
    "Yoshida-6 w₃ −1 unit of digit 13",
    y6_d(
        f64::YOSHIDA6_W1,
        f64::YOSHIDA6_W2,
        moved(f64::YOSHIDA6_W3, 13, -1.0)
    ),
    Y6_BOUND,
    true
);

// ── Steppers written here (dd §3.2), for the controls ────────────────────────────────────────────────────────────

/// A precision: the kernel's two.
trait Pr: StepReal + Into<f64> {
    const EPS: f64;
    const NAME_: &'static str;
    fn of(x: f64) -> Self;
}

impl Pr for f64 {
    const EPS: f64 = f64::EPSILON;
    const NAME_: &'static str = "f64";
    fn of(x: f64) -> f64 {
        x
    }
}

impl Pr for f32 {
    const EPS: f64 = f32::EPSILON as f64;
    const NAME_: &'static str = "f32";
    fn of(x: f64) -> f32 {
        x as f32
    }
}

/// The composition of KDK steps with drift sequence `d` (weights in f64, rounded once to `R`), written here from
/// dd §3.2: kicks `cᵢ = (d_{i−1} + dᵢ)/2`, `d₀ = d_{n+1} = 0`; a kick is `p ← p + h·m·a(r)`, a drift `r ← r + h·p/m`.
fn compose<R: Pr>(d: &[f64], s: &State<R>, dt: R) -> State<R> {
    let n = d.len();
    let c: Vec<R> = (0..=n)
        .map(|i| {
            let a = if i == 0 { 0.0 } else { d[i - 1] };
            let b = if i == n { 0.0 } else { d[i] };
            R::of((a + b) / 2.0)
        })
        .collect();
    let mut st = *s;
    for i in 0..=n {
        let a = Geom::new(&st.m, &st.r).accel;
        let h = c[i] * dt;
        for ((p, &m), a) in st.p.iter_mut().zip(&st.m).zip(&a) {
            for (pk, &ak) in p.iter_mut().zip(a) {
                *pk = *pk + h * (m * ak);
            }
        }
        if i < n {
            let h = R::of(d[i]) * dt;
            for ((r, &m), p) in st.r.iter_mut().zip(&st.m).zip(&st.p) {
                for (rk, &pk) in r.iter_mut().zip(p) {
                    *rk = *rk + h * (pk / m);
                }
            }
        }
    }
    st
}

/// One `STEP` of a shipped occupant.
fn shipped<R: Pr>(o: Selection, s: &State<R>, dt: R) -> State<R> {
    o.step(s, dt, &Geom::new(&s.m, &s.r)).0
}

// ── REQ-INT-090: the convergence order (dd §5 test 1a) ───────────────────────────────────────────────────────────

/// qa's Kepler-embedded fixture: bodies 0 and 1, masses 0.6 and 0.4, on a Kepler orbit of semi-major axis 1 and
/// eccentricity 0.6 started at apocentre about their centre of mass, and body 2 of mass `10⁻¹²` (`m₂ → ε`) at rest
/// 40 away. Returns the state and the pair's period `2π√(a³/(m₀ + m₁))`.
fn kepler() -> (State<f64>, f64) {
    let (m0, m1, a, e): (f64, f64, f64, f64) = (0.6, 0.4, 1.0, 0.6);
    let mt = m0 + m1;
    let ra = a * (1.0 + e);
    let va = (mt * (1.0 - e) / (a * (1.0 + e))).sqrt();
    let mu = m0 * m1 / mt;
    let s = State {
        m: [m0, m1, 1e-12],
        r: [[-ra * m1 / mt, 0.0], [ra * m0 / mt, 0.0], [0.0, 40.0]],
        p: [[0.0, -mu * va], [0.0, mu * va], [0.0, 0.0]],
    };
    (s, std::f64::consts::TAU * (a * a * a / mt).sqrt())
}

fn energy_of(s: &State<f64>) -> f64 {
    energy(&s.m, &s.p, &Geom::new(&s.m, &s.r).pair)
}

/// `max_t |E(t) − E₀|/|E₀|` over one orbit at `n` steps per orbit.
fn max_energy_error(step: &dyn Fn(&State<f64>, f64) -> State<f64>, n: usize) -> f64 {
    let (mut s, period) = kepler();
    let e0 = energy_of(&s);
    let dt = period / n as f64;
    let mut worst = 0.0f64;
    for _ in 0..n {
        s = step(&s, dt);
        worst = worst.max(((energy_of(&s) - e0) / e0).abs());
    }
    worst
}

/// The least-squares slope of `log err` against `log dt` over the ladder `ns` (steps per orbit).
fn fitted_order(what: &str, step: &dyn Fn(&State<f64>, f64) -> State<f64>, ns: &[usize]) -> f64 {
    let pts: Vec<(f64, f64)> = ns
        .iter()
        .map(|&n| {
            let e = max_energy_error(step, n);
            println!("{what}: {n} steps per orbit, max relative energy error {e:.3e}");
            ((1.0 / n as f64).ln(), e.ln())
        })
        .collect();
    let k = pts.len() as f64;
    let (mx, my) = (
        pts.iter().map(|p| p.0).sum::<f64>() / k,
        pts.iter().map(|p| p.1).sum::<f64>() / k,
    );
    let slope = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>()
        / pts.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>();
    println!("{what}: fitted order {slope:.3}");
    slope
}

/// qa's ladders, steps per orbit, a factor 2^(1/2) apart: Yoshida-4's and Yoshida-6's, each above rounding.
const Y4_LADDER: [usize; 5] = [400, 566, 800, 1131, 1600];
const Y6_LADDER: [usize; 5] = [200, 283, 400, 566, 800];

fn check_order(what: &str, step: &dyn Fn(&State<f64>, f64) -> State<f64>, ns: &[usize], want: f64) {
    let p = fitted_order(what, step, ns);
    assert!(
        p.round() == want,
        "{what}'s fitted order {p:.3} does not round to {want}"
    );
}

#[test]
fn qa_yoshida_convergence_order() {
    let y4 = |s: &State<f64>, dt: f64| shipped(Selection::Production(Occupant::Yoshida4), s, dt);
    let y6 = |s: &State<f64>, dt: f64| shipped(Selection::Production(Occupant::Yoshida6), s, dt);
    // The controls' 4th-significant-digit change: 1.351… → 1.352…, −1.177… → −1.176…, 0.2355… → 0.2356….
    assert!((digit4(f64::YOSHIDA4_W1) - 1.3522071919596575).abs() < 1e-15);
    assert!((digit4(f64::YOSHIDA6_W1) - -1.17667998417887).abs() < 1e-15);
    assert!((digit4(f64::YOSHIDA6_W2) - 0.235673213359357).abs() < 1e-15);
    check_order("Yoshida-4", &y4, &Y4_LADDER, 4.0);
    check_order("Yoshida-6", &y6, &Y6_LADDER, 6.0);
    // qa's own composition with the shipped weights fits the same orders, so the controls below change only a digit.
    let d4 = y4_d(f64::YOSHIDA4_W1, f64::YOSHIDA4_W0);
    let d6 = y6_d(f64::YOSHIDA6_W1, f64::YOSHIDA6_W2, f64::YOSHIDA6_W3);
    check_order(
        "qa Yoshida-4",
        &|s: &State<f64>, dt: f64| compose(&d4, s, dt),
        &Y4_LADDER,
        4.0,
    );
    check_order(
        "qa Yoshida-6",
        &|s: &State<f64>, dt: f64| compose(&d6, s, dt),
        &Y6_LADDER,
        6.0,
    );
}

/// `x` with its 4th significant digit changed by one.
fn digit4(x: f64) -> f64 {
    moved(x, 4, 1.0)
}

macro_rules! order_control {
    ($name:ident, $what:literal, $d:expr, $ladder:expr, $want:expr) => {
        mod $name {
            use super::*;
            negative_control!(
                qa_yoshida_convergence_order,
                $what,
                expected = "does not round to",
                {
                    let d = $d;
                    check_order(
                        $what,
                        &|s: &State<f64>, dt: f64| compose(&d, s, dt),
                        &$ladder,
                        $want,
                    );
                }
            );
        }
    };
}

order_control!(
    y4_w1_digit4,
    "Yoshida-4 w₁ with its 4th significant digit changed, w₀ recomputed",
    {
        let w1 = digit4(f64::YOSHIDA4_W1);
        y4_d(w1, 1.0 - 2.0 * w1)
    },
    Y4_LADDER,
    4.0
);
order_control!(
    y6_w1_digit4,
    "Yoshida-6 w₁ with its 4th significant digit changed, w₀ recomputed",
    y6_d(digit4(f64::YOSHIDA6_W1), f64::YOSHIDA6_W2, f64::YOSHIDA6_W3),
    Y6_LADDER,
    6.0
);
order_control!(
    y6_w2_digit4,
    "Yoshida-6 w₂ with its 4th significant digit changed, w₀ recomputed",
    y6_d(f64::YOSHIDA6_W1, digit4(f64::YOSHIDA6_W2), f64::YOSHIDA6_W3),
    Y6_LADDER,
    6.0
);
order_control!(
    y6_w3_digit4,
    "Yoshida-6 w₃ with its 4th significant digit changed, w₀ recomputed",
    y6_d(f64::YOSHIDA6_W1, f64::YOSHIDA6_W2, digit4(f64::YOSHIDA6_W3)),
    Y6_LADDER,
    6.0
);

// ── REQ-VAL-032, REQ-VAL-191: dd §5 test 3, per dt ───────────────────────────────────────────────────────────────

/// The criterion, `k` in `k·ε_R` (REQ-VAL-191, provisional until M3).
const TOL: f64 = STEP_REVERSAL_TOL.number();

/// qa's fixtures, none of them the implementer's: a hierarchical triple (a tight 0–1 binary, the third orbiting it),
/// an equilateral Lagrange configuration in rigid rotation, Burrau's (3, 4, 5) Pythagorean start at rest, and an
/// approaching pair: equal masses 1, bodies 0 and 1 a unit apart moving at each other with speed 1 at impact parameter
/// 0.05 (a near-parabolic encounter, relative energy about 0), the third at rest 5 away. `τ`, from positions alone,
/// does not see the approach: within one step at `dt/τ ≤ step_reversal_edge_max` the pair can pass far closer than
/// the start's separation.
fn fixtures<R: Pr>() -> Vec<(&'static str, State<R>)> {
    let st = |m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3]| State {
        m: m.map(R::of),
        r: r.map(|v| v.map(R::of)),
        p: p.map(|v| v.map(R::of)),
    };
    // Lagrange: equal masses 1 on a circle of radius ρ = 1/√3·side, side 1, ω² = 3m/side³ = 3.
    let (side, w) = (1.0f64, 3.0f64.sqrt());
    let rho = side / 3.0f64.sqrt();
    let ang = |k: f64| k * std::f64::consts::TAU / 3.0 + 0.3;
    let lag_r = [0.0, 1.0, 2.0].map(|k| [rho * ang(k).cos(), rho * ang(k).sin()]);
    let lag_p = lag_r.map(|r| [-w * r[1], w * r[0]]);
    vec![
        (
            "hierarchical",
            st(
                [0.5, 0.45, 0.05],
                [[-0.095, 0.01], [0.105, -0.011], [1.5, 0.4]],
                [[0.03, -0.62], [-0.025, 0.618], [-0.005, 0.002]],
            ),
        ),
        ("lagrange", st([1.0, 1.0, 1.0], lag_r, lag_p)),
        (
            "burrau",
            st(
                [3.0, 4.0, 5.0],
                [[1.0, 3.0], [-2.0, -1.0], [1.0, -1.0]],
                [[0.0; 2]; 3],
            ),
        ),
        (
            "approach",
            st(
                [1.0, 1.0, 1.0],
                [[-0.5, 0.0], [0.5, 0.05], [0.0, 5.0]],
                [[1.0, 0.0], [-1.0, 0.0], [0.0, 0.0]],
            ),
        ),
    ]
}

/// The state's shortest two-body time `τ = min √(r³/(G(mᵢ + mⱼ)))`, `G = 1` (step_reversal_tol's definition).
fn tau<R: Pr>(s: &State<R>) -> f64 {
    [(0, 1), (1, 2), (0, 2)]
        .iter()
        .map(|&(i, j)| {
            let d: Vec<f64> = (0..2)
                .map(|k| s.r[i][k].into() - s.r[j][k].into())
                .collect();
            let r = (d[0] * d[0] + d[1] * d[1]).sqrt();
            (r * r * r / (s.m[i].into() + s.m[j].into())).sqrt()
        })
        .fold(f64::INFINITY, f64::min)
}

fn negate<R: Pr>(s: &State<R>) -> State<R> {
    State {
        m: s.m,
        r: s.r,
        p: s.p.map(|v| v.map(|x| -x)),
    }
}

fn flat<R: Pr>(s: &State<R>) -> Vec<f64> {
    s.r.iter()
        .chain(s.p.iter())
        .flat_map(|v| v.iter().map(|&x| x.into()))
        .collect()
}

/// dd test 3's residual: `STEP(dt)`, negate `p`, `STEP(dt)`, negate `p`, its Euclidean distance from the start over
/// `(r, p)` divided by the start's norm, in units of `R`'s epsilon. No projection, no substepping.
fn residual<R: Pr>(step: &dyn Fn(&State<R>, R) -> State<R>, s: &State<R>, dt: R) -> f64 {
    let back = negate(&step(&negate(&step(s, dt)), dt));
    let (a, b) = (flat(&back), flat(s));
    let diff = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt();
    let norm = b.iter().map(|y| y * y).sum::<f64>().sqrt();
    diff / norm / R::EPS
}

/// The domain's upper edge, `step_reversal_edge_max` (REQ-VAL-191, provisional until M3), at both precisions.
fn edge_max() -> f64 {
    STEP_REVERSAL_EDGE_MAX.number()
}

/// The `dt/τ` of a grid from `lower` to `upper`: `lower` itself, then 16 points per decade, then `upper` itself.
fn grid_to(lower: f64, upper: f64) -> Vec<f64> {
    let mut v = vec![lower];
    let mut x = lower;
    while x * 10f64.powf(1.0 / 16.0) < upper {
        x *= 10f64.powf(1.0 / 16.0);
        v.push(x);
    }
    v.push(upper);
    v
}

/// The `dt/τ` at which the per-dt check runs from a lower edge: the closed domain `[edge, step_reversal_edge_max]`,
/// both edges included, at 16 points per decade.
fn grid(edge: f64) -> Vec<f64> {
    grid_to(edge, edge_max())
}

fn edge<R: Pr>() -> f64 {
    if R::EPS == f64::EPSILON {
        STEP_REVERSAL_EDGE_F64.number()
    } else {
        STEP_REVERSAL_EDGE_F32.number()
    }
}

/// The cells, fixture by `dt/τ` of `ratios`, at which `step` at `R` fails: reads above [`TOL`] when `reversible`,
/// at or below it when a control. Each residual is printed.
fn per_dt<R: Pr>(
    name: &str,
    step: &dyn Fn(&State<R>, R) -> State<R>,
    ratios: &[f64],
    reversible: bool,
) -> Vec<String> {
    let mut failed = Vec::new();
    for (fixture, s) in fixtures::<R>() {
        let t = tau(&s);
        for &x in ratios {
            let r = residual(step, &s, R::of(x * t));
            let bad = if reversible { r > TOL } else { r <= TOL };
            println!(
                "{name} at {}, {fixture} (τ {t:.4}), dt/τ {x:.4e}: {r:.3e} ε{}",
                R::NAME_,
                if bad { "  <- FAILS" } else { "" }
            );
            if bad {
                failed.push(format!(
                    "{name} at {}, {fixture}, dt/τ {x:.4e}: {r:.3e} ε",
                    R::NAME_
                ));
            }
        }
    }
    failed
}

/// No cell failed; `what` says how a failing cell fails.
fn check_cells(failed: &[String], what: &str) {
    assert!(
        failed.is_empty(),
        "{what} step_reversal_tol {TOL} ε at {} cell(s): {failed:#?}",
        failed.len()
    );
}

fn y4_asymmetric<R: Pr>(s: &State<R>, dt: R) -> State<R> {
    compose(
        &[f64::YOSHIDA4_W1, f64::YOSHIDA4_W1, f64::YOSHIDA4_W0],
        s,
        dt,
    )
}

fn rk4<R: Pr>(s: &State<R>, dt: R) -> State<R> {
    shipped(Selection::Production(Occupant::Rk4), s, dt)
}

fn controls_at<R: Pr>(ratios: &[f64]) -> Vec<String> {
    let mut f = per_dt::<R>("RK4", &rk4::<R>, ratios, false);
    f.extend(per_dt::<R>(
        "Yoshida-4 asymmetric (w₁, w₁, w₀)",
        &y4_asymmetric::<R>,
        ratios,
        false,
    ));
    f
}

const NOT_ABOVE: &str = "a control reads not above";
const EXCEEDS: &str = "a step-reversible occupant's residual exceeds";

/// The asymmetric controls read above `step_reversal_tol` at each `dt` inside the domain, at f64 and f32 (physics's
/// request): the per-dt form of REQ-VAL-191's "the asymmetric controls above it at the same dt".
#[test]
fn qa_step_reversibility_controls_per_dt() {
    assert!(TOL > 0.0 && edge::<f64>() > 0.0 && edge::<f32>() > edge::<f64>());
    assert!(edge_max() > edge::<f32>(), "the f32 domain is empty");
    let mut f = controls_at::<f64>(&grid(edge::<f64>()));
    f.extend(controls_at::<f32>(&grid(edge::<f32>())));
    check_cells(&f, NOT_ABOVE);
}

negative_control!(
    qa_step_reversibility_controls_per_dt,
    "the controls at f64, dt/τ 10⁻³, a decade below the f64 edge",
    expected = "a control reads not above",
    check_cells(&controls_at::<f64>(&[edge::<f64>() / 10.0]), NOT_ABOVE)
);

fn reversible_at<R: Pr>(ratios: &[f64]) -> Vec<String> {
    let mut f = Vec::new();
    for (name, o) in [
        ("KDK", Occupant::Kdk),
        ("Yoshida-4", Occupant::Yoshida4),
        ("Yoshida-6", Occupant::Yoshida6),
    ] {
        f.extend(per_dt::<R>(
            name,
            &|s: &State<R>, dt: R| shipped(Selection::Production(o), s, dt),
            ratios,
            true,
        ));
    }
    f
}

/// KDK, Yoshida-4 and Yoshida-6 return each fixture within `step_reversal_tol` at each `dt` inside the domain, at
/// f64 and f32, negative weights included (REQ-VAL-032).
#[test]
fn qa_step_reversibility_per_dt() {
    let mut f = reversible_at::<f64>(&grid(edge::<f64>()));
    f.extend(reversible_at::<f32>(&grid(edge::<f32>())));
    check_cells(&f, EXCEEDS);
}

negative_control!(
    qa_step_reversibility_per_dt,
    "Yoshida-4 with its stages made asymmetric (w₁, w₁, w₀), at f64",
    expected = "a step-reversible occupant's residual exceeds",
    check_cells(
        &per_dt::<f64>(
            "Yoshida-4 asymmetric",
            &y4_asymmetric::<f64>,
            &grid(edge::<f64>()),
            true
        ),
        EXCEEDS
    )
);

/// The cells at which KDK, Yoshida-4 or Yoshida-6 at `R` on Burrau's start read above [`TOL`], over `R`'s lower edge
/// to `upper` at 16 points per decade, both ends included.
fn burrau_reversible<R: Pr>(upper: f64) -> Vec<String> {
    let s = fixtures::<R>()
        .into_iter()
        .find(|(n, _)| *n == "burrau")
        .expect("Burrau's start is among qa's fixtures")
        .1;
    let t = tau(&s);
    let mut f = Vec::new();
    for (name, o) in [
        ("KDK", Occupant::Kdk),
        ("Yoshida-4", Occupant::Yoshida4),
        ("Yoshida-6", Occupant::Yoshida6),
    ] {
        for x in grid_to(edge::<R>(), upper) {
            let r = residual(
                &|s: &State<R>, dt: R| shipped(Selection::Production(o), s, dt),
                &s,
                R::of(x * t),
            );
            let bad = r > TOL;
            println!(
                "{name} at {}, burrau (τ {t:.4}), dt/τ {x:.4e}: {r:.3e} ε{}",
                R::NAME_,
                if bad { "  <- FAILS" } else { "" }
            );
            if bad {
                f.push(format!(
                    "{name} at {}, burrau, dt/τ {x:.4e}: {r:.3e} ε",
                    R::NAME_
                ));
            }
        }
    }
    f
}

/// On Burrau's start, the fixture whose steps carry a pair far closer than its start's `τ`, KDK, Yoshida-4 and
/// Yoshida-6 return the state within `step_reversal_tol` up to and at the domain's upper edge
/// `step_reversal_edge_max`, at f64 and f32 (REQ-VAL-032, REQ-VAL-191).
#[test]
fn qa_step_reversibility_upper_edge() {
    let mut f = burrau_reversible::<f64>(edge_max());
    f.extend(burrau_reversible::<f32>(edge_max()));
    check_cells(&f, EXCEEDS);
}

negative_control!(
    qa_step_reversibility_upper_edge,
    "the f64 grid taken past step_reversal_edge_max to dt/τ 1, where a stage's close approach conditions a symmetric step past the criterion",
    expected = "a step-reversible occupant's residual exceeds",
    check_cells(&burrau_reversible::<f64>(1.0), EXCEEDS)
);
