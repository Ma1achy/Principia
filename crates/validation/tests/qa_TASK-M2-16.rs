//! QA tests for TASK-M2-16, written from the requirements and their sources, not from the implementation: the shared
//! round-trip measure every T2/T3 test and the `encode-eps-phys` gate read (`validation::roundtrip`).
//!
//! - REQ-ENC-029 (`phys_norm_qa_part4`): `validation::roundtrip::phys` is inverse-encode Part 4's `‖x′ − x‖_phys`, on
//!   hand-computed pairs: the largest of `|Δmᵢ|`, `‖Δrᵢ‖/R_x` and `‖Δpᵢ‖/P_x` over the three bodies (a max, never a
//!   sum), the masses absolute, `R_x = max(1, maxⱼ‖rⱼ‖)` and `P_x = max(1, maxⱼ‖pⱼ‖)` the *reference* state's extent
//!   (absolute at most 1, relative above), unweighted by mass; a NaN fails every tolerance.
//! - REQ-ENC-024 (`eps_phys_qa_register`): `ε_phys = k·ε_R` with one `k` for both precisions (Part 4), `ε_R` 2⁻⁵² at
//!   f64 and 2⁻²³ at f32; `k` is the register's `eps_phys`, 64 as proposed, provisional until the M2 gate, citing its
//!   calibration requirement (R-71), and the gate's fixture carries the same value as provisional.
//!
//! Every expected value is exact in binary (powers of two and small integers), so the checks are equalities.
//! Each test has a registered negative control (R-176).

use kernel::decode::Ic;
use ledger::constants::{Citation, ConstantBuilder, EPS_PHYS};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// A reference state: positions `r`, momenta `p`, masses `(½, ¼, ¼)`.
fn st(r: [[f64; 2]; 3], p: [[f64; 2]; 3]) -> Ic<f64> {
    Ic {
        m: [0.5, 0.25, 0.25],
        r,
        p,
    }
}

const SMALL_R: [[f64; 2]; 3] = [[0.5, 0.0], [0.0, -0.5], [-0.25, 0.25]];
const ZERO_P: [[f64; 2]; 3] = [[0.0; 2]; 3];

/// Part 4's norm against hand-computed cases.
fn check_norm(norm: fn(&Ic<f64>, &Ic<f64>) -> f64) {
    let x = st(SMALL_R, ZERO_P);
    // Identical states: 0.
    assert_eq!(norm(&x, &x), 0.0, "‖x − x‖_phys");
    // Extent at most 1: absolute. Body 1 moved by (3/4, 1)·2⁻¹⁰ → 1.25·2⁻¹⁰.
    let mut y = x;
    y.r[1] = [SMALL_R[1][0] + 0.75 / 1024.0, SMALL_R[1][1] + 1.0 / 1024.0];
    assert_eq!(
        norm(&x, &y),
        1.25 / 1024.0,
        "a position change at extent ≤ 1 is absolute"
    );
    // Extent above 1: relative to the reference's largest ‖rⱼ‖ (4 here).
    let big = st([[4.0, 0.0], [0.0, -2.0], [-1.0, 1.0]], ZERO_P);
    let mut yb = big;
    yb.r[2] = [-1.0 + 1.0 / 1024.0, 1.0];
    assert_eq!(
        norm(&big, &yb),
        0.25 / 1024.0,
        "a position change is relative to R_x = 4"
    );
    // The extent is the reference's, not the measured state's: with the roles swapped R_x is the measured one's.
    let mut far = x;
    far.r[0] = [8.0, 0.0];
    assert_eq!(norm(&x, &far), 7.5, "‖far − x‖ with x's extent 1");
    assert_eq!(norm(&far, &x), 7.5 / 8.0, "‖x − far‖ with far's extent 8");
    // Momenta against P_x, independently of R_x.
    let mp = st(SMALL_R, [[2.0, 0.0], [-1.0, 0.0], [-1.0, 0.0]]);
    let mut yp = mp;
    yp.p[1] = [-1.0, 1.0 / 512.0];
    assert_eq!(
        norm(&mp, &yp),
        0.5 / 512.0,
        "a momentum change is relative to P_x = 2"
    );
    // Masses absolute, unweighted; the largest block wins, never their sum.
    let mut ym = x;
    ym.m = [0.5 - 1.0 / 256.0, 0.25 + 1.0 / 256.0, 0.25];
    ym.r[2] = [-0.25, 0.25 + 1.0 / 1024.0];
    assert_eq!(
        norm(&x, &ym),
        1.0 / 256.0,
        "masses absolute, the max over blocks and bodies"
    );
    // Unweighted by mass: the same position change on the heavy and on a light body measures the same.
    let (mut h, mut l) = (x, x);
    h.r[0][0] += 1.0 / 64.0;
    l.r[2][0] += 1.0 / 64.0;
    assert_eq!(
        norm(&x, &h),
        norm(&x, &l),
        "body 0 (m ½) and body 2 (m ¼) moved alike"
    );
    // A NaN anywhere admits no tolerance.
    let mut n = x;
    n.p[2][1] = f64::NAN;
    assert!(
        norm(&x, &n).is_nan(),
        "a NaN in x′ is not NaN, so a tolerance could admit it"
    );
    assert!(
        norm(&n, &x).is_nan(),
        "a NaN in x is not NaN, so a tolerance could admit it"
    );
}

#[test]
fn phys_norm_qa_part4() {
    check_norm(phys);
}

negative_control!(
    phys_norm_qa_part4,
    "a norm taking the measured state's extent in place of the reference's must fail",
    expected = "with x's extent 1",
    check_norm(|x, y| phys(y, x))
);

/// `ε_phys` at both precisions and the register entry `k`, `entry`, against Part 4 and REQ-ENC-024.
fn check_eps(entry: ConstantBuilder, at64: f64, at32: f64) {
    assert_eq!(entry.name, "eps_phys", "the register's name");
    assert_eq!(entry.number(), 64.0, "k as proposed");
    assert_eq!(
        entry.provisional,
        Some("M2"),
        "eps_phys is provisional until the M2 gate"
    );
    assert_eq!(
        entry.citation,
        Some(Citation::Calibration("REQ-ENC-024")),
        "eps_phys cites its calibration requirement"
    );
    assert_eq!(
        at64,
        entry.number() * 2f64.powi(-52),
        "ε_phys at f64 is k·2⁻⁵²"
    );
    assert_eq!(
        at32,
        entry.number() * 2f64.powi(-23),
        "ε_phys at f32 is k·2⁻²³"
    );
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/gates/encode-eps-phys/gate.json");
    let gate: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read the gate"))
            .expect("JSON");
    assert_eq!(
        gate["threshold"]["value"].as_f64(),
        Some(entry.number()),
        "the gate's threshold is k"
    );
    assert_eq!(
        gate["threshold"]["requirement"], "REQ-ENC-024",
        "the gate cites REQ-ENC-024"
    );
    assert_eq!(
        gate["threshold"]["status"], "provisional",
        "the gate's k is provisional"
    );
}

#[test]
fn eps_phys_qa_register() {
    check_eps(EPS_PHYS, eps_phys::<f64>(), eps_phys::<f32>());
}

negative_control!(
    eps_phys_qa_register,
    "an f32 tolerance at the f64 epsilon must fail the per-precision check",
    expected = "ε_phys at f32",
    check_eps(EPS_PHYS, eps_phys::<f64>(), eps_phys::<f64>())
);
