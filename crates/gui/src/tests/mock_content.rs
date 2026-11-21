//! The mock's placeholder content, pinned (R-390): its fake samples, hover path, quads and bounds are deterministic in
//! their inputs, so each is pinned here by a fingerprint of its values at fixed points, one per outcome class. The
//! values are placeholder content, not corpus values; the pins keep them, and every capture of them, from drifting
//! unseen. Each check also runs on an input it must reject (R-176, through `rejects`).

use engine::contract::display;
use engine::contract::legend::SWATCHES;
use engine::contract::sample::Sample;
use engine::contract::sim_config::Plane;

use super::support::rejects;
use crate::explore::hover_label;
use crate::explore::trajectory::{kept_row, summary};
use crate::mock::quads;
use crate::mock::sample::{hash, hover_path, sample};
use crate::mock::{MOCK_Q1, MOCK_Q2, MOCK_Z0};

fn plane() -> Plane {
    Plane {
        z0: MOCK_Z0,
        q1: MOCK_Q1,
        q2: MOCK_Q2,
    }
}

/// The most one fake hash may differ between two platforms (applied per R-369: the pins hold on macOS and on Linux,
/// where CI's mutants run). `hash` is `fract(sin(x)·C)`, `C` = 43 758.5453, its argument IEEE-exact arithmetic on the
/// fixed points; `sin` is its one step that is not, and two libms' `sin`, each within 1 ulp of the true value, differ
/// by at most 2 ulp of a value in [-1, 1], 2·2⁻⁵³ = 2⁻⁵². The product, below 2¹⁶, then differs by at most C·2⁻⁵² plus
/// one ulp of it, 2⁻³⁷; `fract` is exact; the `+ 1` of a negative fraction adds at most half an ulp of 1, 2⁻⁵³. That
/// is 1.70e-11, held here at 2e-11 (macOS and glibc gave E₀ 2.2e-12 apart at the first pin as it was under `z_β =
/// 0.410`, 3.6e-12 in its hash). The bound holds only off a jump: at the pinned points every hash's product is at
/// least 5.3e-3 from an integer, every class, word letter and substep count at least 1.1e-2 of a step from a floor, and
/// every shape angle at least 3.3e-3 from a wrap of 2π, all far beyond the most they move (8e-7), so the fates, words
/// and counts are exact. The points are the first of their row, in steps of 0.0025, to hold each margin the earlier
/// pins held (2.2e-3, 8.8e-3 and 1.5e-3).
const EPS_H: f64 = 2e-11;

/// `a` within `k` hash errors of `b`, `k` the most `b` moves per unit change of the hashes it is made of (each change
/// at most [`EPS_H`]), plus a relative 1e-12 for the roundings after them (under an ulp each on a scalar) and the
/// paths' own `sin` and `cos` (under 1e-9 on a fingerprint, far below its `k` term).
fn near(a: f64, b: f64, k: f64) -> bool {
    (a - b).abs() <= k * EPS_H + 1e-12 * b.abs().max(1.0)
}

/// `t_end` and the path span, `t_max·(0.1 + 0.8h)`, `t_max` = 50: 40 per unit of `h`.
const K_T: f64 = 40.0;
/// FTLE, `0.02 + 0.1h`.
const K_FTLE: f64 = 0.1;
/// `E₀`, `-0.2 - 0.6h`.
const K_E0: f64 = 0.6;
/// The minimum separation, `1e-4·(1 + 9h)`.
const K_MIN_SEP: f64 = 9e-4;
/// `|ΔE/E|`, `1e-10·(1 + 9h)`.
const K_DRIFT: f64 = 9e-10;
/// The shape path's fingerprint, `Σ θ + 2φ` over 240 points, `τ` up to 50 and moving 40 per unit of the span's hash.
/// `θ = θ₀ + ωτ` mod 2π, `θ₀ = 2πh`, `ω = 0.6 + h'` below 1.6: 2π + 50 + 1.6·40 = 120.3. `φ = π/2 + 1.1 sin(0.37τ +
/// θ₀)`: 1.1·(0.37·40 + 2π) = 23.2. So 120.3 + 2·23.2 = 166.7 a point, 167 here.
const K_SHAPE: f64 = 240.0 * 167.0;
/// A body path's fingerprint, `Σ x - 3y` over 240 points, `x = r cos a`, `y = 0.7 r sin a`. The angle `ωτ + 2πk/3`:
/// 50 + 1.6·40 = 114. The radius `1 + 0.35 sin(2.3τ + k)`, plus `0.4·(τ - 0.7 t_end)` past 70% for an escaping body:
/// 0.35·2.3·40 + 0.4·(40 + 0.7·40) = 59.4, and at most 1.35 + 0.4·0.3·45 = 6.75. A point moves at most (1 + 2.1)·
/// (59.4 + 6.75·114) = 2569.6, 2570 here.
const K_BODY: f64 = 240.0 * 2570.0;

/// A sample's values at `at`, the playhead at the horizon 50: its fate, `t_end`, FTLE, `E₀`, F₂ word, substeps,
/// minimum separation, `|ΔE/E|`, path span, and fingerprints of its shape path and its three bodies' paths.
type Pin = (
    [f64; 2],
    &'static str,
    f64,
    f64,
    f64,
    &'static str,
    u64,
    f64,
    f64,
    f64,
    f64,
    [f64; 3],
);

/// One point per outcome class, at the mock's `z₀`. The fake hash reads the raw latent point, so the classes and values
/// here are those of `z_β = π·σ(0.410) − π/2`, the mock's `z₀` since `z_β` became the periodic link's control; under
/// `z_β = 0.410` the same nine classes were pinned at other points of the same row, with other values.
const PINS: [Pin; 9] = [
    (
        [0.0125, 0.37],
        "escape body 1",
        20.902622945723127,
        0.08788292513083434,
        -0.7097987667133565,
        "b a⁻¹ b a b",
        2441,
        0.0009943904332132662,
        3.950174125515332e-10,
        20.902622945723127,
        1423.329603190296,
        [-58.1710692249364, 16.097674994799902, 15.417215211507639],
    ),
    (
        [0.0275, 0.37],
        "escape body 2",
        42.809280414367095,
        0.053964145779464165,
        -0.5967731434197049,
        "a b a a b⁻¹",
        36658,
        0.000985869834827463,
        1.7061469156324164e-10,
        42.809280414367095,
        1557.1117336023667,
        [-9.335807039058423, 60.360937292558205, 2.1092384528065864],
    ),
    (
        [0.0725, 0.37],
        "escape body 3",
        44.76055376013392,
        0.028748285882757046,
        -0.40253783418193056,
        "b b⁻¹ a a⁻¹ a",
        31803,
        0.000648751939979411,
        8.13695124638707e-10,
        44.76055376013392,
        1477.9831683365442,
        [-14.245390380958053, -3.3975873535562204, 69.66721767245495],
    ),
    (
        [0.0075, 0.37],
        "bounded",
        50.0,
        0.02585936131210474,
        -0.25355496965057683,
        "a⁻¹ b b a b⁻¹",
        16250,
        0.0009199208232144884,
        8.837496637752338e-10,
        50.0,
        1507.8872369625687,
        [-3.4372501865067786, 2.739592736255132, -3.3302730100820965],
    ),
    (
        [0.0025, 0.37],
        "collision 1–2",
        39.28423515302711,
        0.07579256740711572,
        -0.6457807632803452,
        "b⁻¹ a a⁻¹ b⁻¹ a⁻¹",
        12746,
        0.0009336397729737655,
        7.102546186091786e-10,
        39.28423515302711,
        1512.901010326526,
        [-16.59970795995648, 5.840121006197681, 12.549939403801963],
    ),
    (
        [0.0, 0.37],
        "collision 1–3",
        32.23048243642552,
        0.10188153339615384,
        -0.32159741552604826,
        "a⁻¹ a⁻¹ a⁻¹ b a",
        24800,
        0.0005229786477182642,
        9.748060605757929e-10,
        32.23048243642552,
        1549.2102623161602,
        [-3.654183113747976, -16.153719902491222, 13.584449967027481],
    ),
    (
        [0.015, 0.37],
        "collision 2–3",
        33.00758164230501,
        0.044997102056659063,
        -0.5226536534115439,
        "a b⁻¹ b a a",
        35934,
        0.0005204919214622351,
        7.296797694849375e-10,
        33.00758164230501,
        1496.1595656479656,
        [-5.058018502666518, 3.4825026573688564, -2.6481858780014367],
    ),
    (
        [0.005, 0.37],
        "triple collision",
        44.581259199621854,
        0.10712411117012381,
        -0.6873745580262038,
        "b⁻¹ b b b a",
        4116,
        0.00028719692740924074,
        6.74971750361874e-10,
        44.581259199621854,
        1520.5137612033361,
        [-10.280304221037618, -11.674177701972866, 21.058904253171818],
    ),
    (
        [0.0225, 0.37],
        "triple ejection",
        23.205967058311217,
        0.03473529640061315,
        -0.635339828983706,
        "b⁻¹ a a b⁻¹ b⁻¹",
        39279,
        0.0005523456297967641,
        2.0559128092354514e-10,
        23.205967058311217,
        1568.7111636705504,
        [-12.664885108750148, -22.54227883894311, 22.017983890009738],
    ),
];

/// Asserts the sample at `pin`'s point matches it.
fn check_pin(plane: &Plane, pin: &Pin) {
    let (at, fate, t_end, ftle, e0, word, substeps, min_sep, drift, path_t, shape, bodies) = *pin;
    let s = sample(plane, at, 50.0, 50.0);
    assert_eq!(s.fate.as_deref(), Some(fate), "{at:?}");
    assert!(
        near(s.t_end.unwrap(), t_end, K_T),
        "t_end {:?} at {at:?}",
        s.t_end
    );
    assert!(
        near(s.ftle, ftle, K_FTLE) && near(s.e0, e0, K_E0),
        "FTLE {} E₀ {} at {at:?}",
        s.ftle,
        s.e0
    );
    assert_eq!((s.f2_word.as_str(), s.substeps), (word, substeps), "{at:?}");
    assert!(
        near(s.min_separation, min_sep, K_MIN_SEP) && near(s.energy_drift, drift, K_DRIFT),
        "{at:?}"
    );
    assert!(near(s.path_t, path_t, K_T), "path_t {} at {at:?}", s.path_t);
    let got: f64 = s.shape.iter().map(|q| q[0] + 2.0 * q[1]).sum();
    assert!(
        near(got, shape, K_SHAPE),
        "the shape path's fingerprint {got} at {at:?}"
    );
    for (k, body) in s.bodies.iter().enumerate() {
        let got: f64 = body.iter().map(|q| q[0] - 3.0 * q[1]).sum();
        assert!(
            near(got, bodies[k], K_BODY),
            "body {k}'s fingerprint {got} at {at:?}"
        );
    }
}

#[test]
fn mock_hover_content_samples_are_pinned() {
    let p = plane();
    for pin in &PINS {
        check_pin(&p, pin);
    }
    rejects("a pin moved to another point", || {
        let mut moved = PINS[0];
        moved.0 = [0.5, 0.5];
        check_pin(&p, &moved)
    });
    // Each tolerance still catches a value moved ten times past it.
    type Nudge = fn(&mut Pin);
    let nudges: [(&str, Nudge); 8] = [
        ("t_end", |m| m.2 += 10.0 * K_T * EPS_H),
        ("FTLE", |m| m.3 += 10.0 * K_FTLE * EPS_H),
        ("E₀", |m| m.4 += 10.0 * K_E0 * EPS_H),
        ("the minimum separation", |m| {
            m.7 += 10.0 * K_MIN_SEP * EPS_H + 1e-11
        }),
        ("|ΔE/E|", |m| m.8 += 1e-11),
        ("the path span", |m| m.9 += 10.0 * K_T * EPS_H),
        ("the shape path", |m| m.10 += 10.0 * K_SHAPE * EPS_H),
        ("a body path", |m| m.11[2] += 10.0 * K_BODY * EPS_H),
    ];
    for (what, nudge) in nudges {
        rejects(&format!("{what} moved past its tolerance"), || {
            let mut moved = PINS[0];
            nudge(&mut moved);
            check_pin(&p, &moved)
        });
    }
    for (x, h) in [
        (0.0, 0.0),
        (1.0, 0.5462073519520345),
        (-2.5, 0.729571438896528),
        (100.0, 0.17615513507917058),
    ] {
        assert!(near(hash(x), h, 1.0), "hash({x}) = {}", hash(x));
    }
    // A negative sine's fraction is brought into [0, 1).
    assert!((0..200)
        .map(|i| hash(f64::from(i) * 0.37 - 30.0))
        .all(|h| (0.0..1.0).contains(&h)));
}

#[test]
fn mock_hover_content_path_and_quads_are_pinned() {
    let p = plane();
    let path = hover_path(&p, [0.4, 0.6]);
    assert_eq!(path.len(), 160);
    let at: f64 = path.iter().map(|q| q.at[0] + 3.0 * q.at[1]).sum();
    let alpha: f64 = path.iter().map(|q| f64::from(q.alpha)).sum();
    assert!((at - 352.0).abs() < 1e-9, "{at}");
    // The path's tilt off the plane is the fake hash of the latent point, the mock's `z₀` in it.
    assert!((alpha - 92.44848132133484).abs() < 1e-4, "{alpha}");
    let leaves = quads::quads(&p);
    assert_eq!(leaves.len(), 682);
    let qs: f64 = leaves
        .iter()
        .map(|c| c[0] + 2.0 * c[1] + 3.0 * c[2] + 5.0 * c[3])
        .sum();
    assert!((qs - 3019.625).abs() < 1e-9, "{qs}");
    let bounds = quads::bounds(&p);
    assert_eq!(bounds.len(), 2728);
    let bs: f64 = bounds
        .iter()
        .map(|s| s[0][0] + 2.0 * s[0][1] + 3.0 * s[1][0] + 5.0 * s[1][1])
        .sum();
    // The tree is fixed in the plane's own coordinates and the figure is centred on `z₀`: the same 682 quads at the
    // mock's `z₀`, their bounds' `t` moved with it.
    assert!((bs - 16820.82040020611).abs() < 1e-6, "{bs}");
    rejects("another tree", || assert_eq!(leaves.len(), 681));
    rejects("the bounds about another z₀", || {
        let mut other = plane();
        other.z0[1] = 0.410;
        let bs: f64 = quads::bounds(&other)
            .iter()
            .map(|s| s[0][0] + 2.0 * s[0][1] + 3.0 * s[1][0] + 5.0 * s[1][1])
            .sum();
        assert!((bs - 16820.82040020611).abs() < 1e-6, "{bs}")
    });
}

// --- REQ-GUI-188: every label the mock draws reads the display module's names ----------------------------------------

/// The labels a person reads for `s`, the playhead at the horizon 50: the hover label, the Trajectory panel's summary
/// and kept row, and the Inspector's readouts.
fn labels_of(s: &Sample) -> Vec<String> {
    let mut out = vec![hover_label::text(s, 50.0, 50.0), summary(s), kept_row(0, s)];
    out.extend(
        crate::windows::inspector::readouts(s)
            .into_iter()
            .map(|(name, value)| format!("{name} {value}")),
    );
    out
}

/// Checks that `labels`, a person's reading of a sample whose fate is `fate`, name it, the one conversion function's
/// name of its class's `state` and `detail` (R-449), and show no internal id, neither "body 0" nor "pair " and a digit.
fn check_labels(labels: &[String], fate: &str) {
    assert!(
        SWATCHES
            .iter()
            .any(|w| display::name_of(w.state, w.detail) == Some(fate)),
        "the fate `{fate}` is no class's display name"
    );
    for l in labels {
        assert!(
            !display::shows_an_internal_id(l),
            "the label `{l}` shows an internal id"
        );
    }
    for l in &labels[..3] {
        assert!(l.contains(fate), "the label `{l}` does not name `{fate}`");
    }
    assert!(
        labels.contains(&format!("outcome {fate}")),
        "the Inspector's outcome does not read `{fate}`"
    );
}

#[test]
fn mock_content_labels_read_the_display_names() {
    let p = plane();
    // One pinned sample per class: each fate is R-449's name, "escape body 1", "collision 1–2", "bounded".
    for pin in &PINS {
        let s = sample(&p, pin.0, 50.0, 50.0);
        assert_eq!(s.fate.as_deref(), Some(pin.1), "{:?}", pin.0);
        check_labels(&labels_of(&s), pin.1);
    }
    let fates: Vec<&str> = PINS.iter().map(|pin| pin.1).collect();
    let names: Vec<&str> = SWATCHES
        .iter()
        .map(|w| display::name_of(w.state, w.detail).unwrap())
        .collect();
    for name in &names {
        assert!(fates.contains(name), "no pin of `{name}`");
    }
    let s = sample(&p, PINS[0].0, 50.0, 50.0);
    rejects("a mock label written 0-based", || {
        let mut s0 = s.clone();
        s0.fate = Some("escape body 0".to_owned());
        check_labels(&labels_of(&s0), "escape body 0")
    });
    rejects("a pair named by its id", || {
        let mut s0 = s.clone();
        s0.fate = Some("collision pair 2".to_owned());
        check_labels(&labels_of(&s0), "collision pair 2")
    });
    rejects("a label written 0-based beside a 1-based fate", || {
        let mut labels = labels_of(&s);
        labels.push("body 0 · t 3.0".to_owned());
        check_labels(&labels, "escape body 1")
    });
}
