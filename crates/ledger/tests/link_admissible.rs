//! The registry check's three requirements at their boundaries (R-481 G3; dd_generation_root §3.9, "Tunable
//! parameters"; REQ-GEN-041): each codomain's bounds at f32 and f64, an output on a bound accepted and one an ulp past
//! it refused, the simplex's sum within `ε_phys` of 1, the round trip skipped on and past a clamp image and held to
//! `ε_phys` inside, the log-det's derivative against the differenced volume factor, and the seeded sample pinned.

use ledger::constants::Citation;
use ledger::links::admissible::{bounds, c1, constraint, round_trip, volume};
use ledger::links::{
    builders, check_with, controls, range_values, slots, tunables, Codomain, Constraint, Expr, Fl,
    Link, LinkBuilder, Op, Param, Tunable, Values,
};
use validation::negative_control;

fn leak(args: Vec<Expr>) -> &'static [Expr] {
    Vec::leak(args)
}

fn op(o: Op, args: Vec<Expr>) -> Expr {
    Expr::Op(o, leak(args))
}

const X: Expr = Expr::Input(0);
const X1: Expr = Expr::Input(1);

fn num(v: f64) -> Expr {
    Expr::Num(v)
}

/// A test link: its forward, inverse and log-det trees.
fn link(forward: Vec<Expr>, inverse: Vec<Expr>, log_det: Expr) -> Link {
    Link {
        name: "fx",
        constraint: Constraint::Symmetric,
        forward: leak(forward),
        inverse: leak(inverse),
        log_det,
        clamps: &[],
        params: &[],
        sampling_note: "a test link",
    }
}

/// `q_max` = 2 and `alpha_min` = `a`.
fn values(a: f64) -> Vec<Param> {
    vec![
        Param {
            name: "q_max",
            value: 2.0,
        },
        Param {
            name: "alpha_min",
            value: a,
        },
    ]
}

fn at<'a>(declared: &'a [Param]) -> Values<'a> {
    Values {
        declared,
        active: &[],
    }
}

// ── Each codomain's bounds ──────────────────────────────────────────────────────────────────────────────────────

/// A closed range's two ends.
type Ends = (f64, f64);

/// The bounds at f64 and f32, `alpha_min` = 0.25 and `q_max` = 2: π/2 and π toward zero at f32 (R-417).
fn check_bounds(beta_hi_f32: f32) {
    use std::f64::consts::{FRAC_PI_2, PI};
    let v = values(0.25);
    let half_pi_32 = f32::from_bits(0x3FC9_0FDA);
    let want: [(Codomain, Ends, Ends); 6] = [
        (
            Codomain::Alpha,
            (0.25, FRAC_PI_2 - 0.25),
            (0.25, f64::from(half_pi_32 - 0.25)),
        ),
        (Codomain::Beta, (0.0, PI), (0.0, f64::from(beta_hi_f32))),
        (Codomain::Momentum, (-2.0, 2.0), (-2.0, 2.0)),
        (
            Codomain::HalfLine,
            (0.0, f64::INFINITY),
            (0.0, f64::INFINITY),
        ),
        (
            Codomain::Mass,
            (f64::NEG_INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::INFINITY),
        ),
        (
            Codomain::RealLine,
            (f64::NEG_INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::INFINITY),
        ),
    ];
    for (c, b64, b32) in want {
        assert_eq!(
            bounds::<f64>(c, &at(&v)),
            b64,
            "`{}`'s bounds at f64",
            c.name()
        );
        assert_eq!(
            bounds::<f32>(c, &at(&v)),
            b32,
            "`{}`'s bounds at f32",
            c.name()
        );
    }
}

#[test]
fn link_admissible_bounds() {
    check_bounds(f32::from_bits(0x4049_0FDA));
}

negative_control!(
    link_admissible_bounds,
    "β's f32 upper end is π rounded toward zero, not to nearest",
    expected = "`beta`'s bounds at f32",
    check_bounds(std::f32::consts::PI)
);

// ── Requirement 1 ───────────────────────────────────────────────────────────────────────────────────────────────

fn up(v: f64) -> f64 {
    f64::from_bits(v.to_bits() + 1)
}

fn up32(v: f32) -> f64 {
    f64::from(f32::from_bits(v.to_bits() + 1))
}

/// Whether the constant forward `y` keeps `codomain` at `F`.
fn keeps<F: Fl>(codomain: Codomain, y: &[f64]) -> bool {
    let l = link(y.iter().map(|&v| num(v)).collect(), vec![X], num(0.0));
    let v = values(0.0);
    constraint::<F>(&l, codomain, &[F::of(0.0), F::of(0.0)], &at(&v)).is_ok()
}

/// Outputs on the momenta's bounds are kept, an ulp past them refused, at f64 and f32; NaN is refused; the simplex's
/// corner is kept, a negative or past-1 mass refused, and the sum held to `ε_phys` = 64 ulps of 1 at f64.
fn check_constraint(past_hi: f64) {
    let m = Codomain::Momentum;
    assert!(
        keeps::<f64>(m, &[2.0]) && keeps::<f64>(m, &[-2.0]),
        "the bounds"
    );
    assert!(
        !keeps::<f64>(m, &[past_hi]),
        "an ulp above q_max is kept at f64"
    );
    assert!(
        !keeps::<f64>(m, &[-up(2.0)]),
        "an ulp below −q_max is kept at f64"
    );
    assert!(
        keeps::<f32>(m, &[2.0]) && keeps::<f32>(m, &[-2.0]),
        "the bounds at f32"
    );
    assert!(
        !keeps::<f32>(m, &[up32(2.0)]),
        "an ulp above q_max is kept at f32"
    );
    assert!(
        !keeps::<f32>(m, &[-up32(2.0)]),
        "an ulp below −q_max is kept at f32"
    );
    assert!(!keeps::<f64>(m, &[f64::NAN]), "NaN is kept");
    let s = Codomain::Mass;
    assert!(keeps::<f64>(s, &[0.0, 1.0, 0.0]), "the simplex's corner");
    assert!(
        keeps::<f32>(s, &[0.0, 1.0, 0.0]),
        "the simplex's corner at f32"
    );
    assert!(
        !keeps::<f64>(s, &[-f64::MIN_POSITIVE, 1.0, f64::MIN_POSITIVE]),
        "a negative mass"
    );
    assert!(!keeps::<f64>(s, &[0.0, up(1.0), 0.0]), "a mass past 1");
    let tol = 64.0 * f64::EPSILON;
    assert!(keeps::<f64>(s, &[tol, 1.0, 0.0]), "a sum ε_phys past 1");
    assert!(
        !keeps::<f64>(s, &[2.0 * tol, 1.0, 0.0]),
        "a sum 2ε_phys past 1"
    );
    assert!(
        !keeps::<f64>(s, &[0.0, 1.0, -tol]),
        "a negative mass with sum 1 − ε_phys"
    );
    assert!(
        keeps::<f64>(s, &[0.25, 0.25, 0.5 - tol]),
        "a sum ε_phys short of 1"
    );
    assert!(
        !keeps::<f64>(s, &[0.25, 0.25, 0.5 - 2.0 * tol]),
        "a sum 2ε_phys short of 1"
    );
}

#[test]
fn link_admissible_constraint() {
    check_constraint(up(2.0));
}

negative_control!(
    link_admissible_constraint,
    "an output on q_max is kept, so its refusal's arm fails",
    expected = "an ulp above q_max is kept at f64",
    check_constraint(2.0)
);

// ── Requirement 2 ───────────────────────────────────────────────────────────────────────────────────────────────

/// The identity forward with the inverse `x + d`, its round trip at `z` against the clamp images `images`, at `F`.
fn trip<F: Fl>(d: f64, z: f64, images: &[(f64, f64)]) -> Result<(), String> {
    let l = link(vec![X], vec![op(Op::Add, vec![X, num(d)])], num(0.0));
    round_trip::<F>(&l, &[F::of(z)], images, &at(&[]))
}

/// The images must be finite and non-empty; a control on or past one is not tested, one inside is; the round trip's
/// error is held to `ε_phys` = 64 ulps at each float type.
fn check_round_trip(inside: f64) {
    let unit = [(-1.0, 1.0)];
    for bad in [
        (f64::NAN, 1.0),
        (-1.0, f64::INFINITY),
        (1.0, 1.0),
        (1.0, -1.0),
    ] {
        let e = trip::<f64>(0.0, 0.0, &[bad]).expect_err("a bad image is accepted");
        assert!(e.contains("not a finite clamp image"), "{e}");
    }
    assert!(trip::<f64>(0.0, 0.0, &unit).is_ok(), "an exact inverse");
    for z in [-1.0, 1.0, -3.0, 3.0] {
        assert!(
            trip::<f64>(1.0, z, &unit).is_ok(),
            "z = {z}, saturated, is tested"
        );
    }
    for z in [inside, 0.0, -f64::from_bits(1.0f64.to_bits() - 1)] {
        assert!(
            trip::<f64>(1.0, z, &unit).is_err(),
            "z = {z}, inside, is not tested"
        );
    }
    let tol64 = 64.0 * f64::EPSILON;
    assert!(
        trip::<f64>(tol64, 0.0, &unit).is_ok(),
        "an error of ε_phys at f64"
    );
    assert!(
        trip::<f64>(2.0 * tol64, 0.0, &unit).is_err(),
        "an error of 2ε_phys at f64"
    );
    let tol32 = 64.0 * f64::from(f32::EPSILON);
    assert!(
        trip::<f32>(tol32, 0.0, &unit).is_ok(),
        "an error of ε_phys at f32"
    );
    assert!(
        trip::<f32>(2.0 * tol32, 0.0, &unit).is_err(),
        "an error of 2ε_phys at f32"
    );
}

#[test]
fn link_admissible_round_trip() {
    check_round_trip(f64::from_bits(1.0f64.to_bits() - 1));
}

negative_control!(
    link_admissible_round_trip,
    "a control on the clamp image is not tested, so its round trip's failure is not seen",
    expected = "inside, is not tested",
    check_round_trip(1.0)
);

// ── Requirement 3 ───────────────────────────────────────────────────────────────────────────────────────────────

/// The forward `2x`, its log-det `ld`, at `z` = 0.3 at f64 and f32.
fn slope(ld: Expr) -> Result<(), String> {
    let l = link(vec![op(Op::Mul, vec![num(2.0), X])], vec![X], ld);
    c1::<f64>(&l, &[0.3], &at(&[]))?;
    c1::<f32>(&l, &[0.3], &at(&[]))
}

/// The log-det's derivative agrees with the forward's difference, or the check refuses; a NaN or +∞ log-det is no
/// derivative, and −∞ is the derivative 0 of a constant forward.
fn check_c1(ld: f64) {
    assert!(slope(op(Op::Log, vec![num(2.0)])).is_ok(), "log 2 for 2x");
    let e = slope(op(Op::Log, vec![num(ld)])).expect_err("a wrong log-det is accepted");
    assert!(e.contains("is not the forward's central difference"), "{e}");
    for bad in [f64::NAN, f64::INFINITY] {
        let e = slope(num(bad)).expect_err("a non-finite log-det is accepted");
        assert!(e.contains("not a finite derivative"), "{e}");
    }
    // The tolerance's rounding terms: `2x` at z = 0 differences exactly to 2 at both steps, so the tolerance is the
    // two steps' rounding, r + r/2 (r = 2ε_phys/2h), with ε_phys·e^ld beside it, far smaller.
    let r = 2.0 * 64.0 * f64::EPSILON / (2.0 * f64::EPSILON.cbrt());
    let at_zero = |gap: f64| {
        let l = link(
            vec![op(Op::Mul, vec![num(2.0), X])],
            vec![X],
            op(Op::Log, vec![num(2.0 + gap)]),
        );
        c1::<f64>(&l, &[0.0], &at(&[]))
    };
    assert!(at_zero(r).is_ok(), "a gap of r, inside r + r/2");
    assert!(at_zero(1.6 * r).is_err(), "a gap of 1.6r, outside r + r/2");
    // And `ε_phys·e^ld`: for `s·x` with `s·h` > 1 each step's rounding is `ε_phys·s`, so the tolerance is `3ε_phys·s`,
    // a third of it the log-det's own term.
    let s = 1e8;
    let steep = |gap: f64| {
        let l = link(
            vec![op(Op::Mul, vec![num(s), X])],
            vec![X],
            op(Op::Log, vec![num(s * (1.0 + gap * 64.0 * f64::EPSILON))]),
        );
        c1::<f64>(&l, &[0.0], &at(&[]))
    };
    assert!(steep(2.5).is_ok(), "a gap of 2.5ε_phys·s, inside 3ε_phys·s");
    assert!(
        steep(3.5).is_err(),
        "a gap of 3.5ε_phys·s, outside 3ε_phys·s"
    );
    let flat = link(vec![num(1.0)], vec![X], num(f64::NEG_INFINITY));
    assert!(
        c1::<f64>(&flat, &[0.3], &at(&[])).is_ok(),
        "a constant forward"
    );
}

#[test]
fn link_admissible_c1() {
    check_c1(2.1);
}

negative_control!(
    link_admissible_c1,
    "log 2 is the slope of 2x, so its refusal's arm fails",
    expected = "a wrong log-det is accepted",
    check_c1(2.0)
);

/// The differenced volume factor of a linear map, `√det(JᵀJ)`, and its rounding allowance, `2·ε_phys·max(1, |f|)/step`
/// per column, combined for two as `e_a|b| + |a|e_b + e_a·e_b`.
fn check_volume(want_area: f64) {
    let v = at(&[]);
    let area = |f: Vec<Expr>| volume::<f64>(&link(f, vec![X], num(0.0)), &[0.0, 0.0], 1.0, &v).0;
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9;
    let scaled = vec![
        op(Op::Mul, vec![num(2.0), X]),
        op(Op::Mul, vec![num(3.0), X1]),
        num(0.0),
    ];
    assert!(close(area(scaled), want_area), "the area of diag(2, 3)");
    let sheared = vec![op(Op::Add, vec![X, X1]), X1, num(0.0)];
    assert!(close(area(sheared), 1.0), "a shear keeps area 1");
    let skew = vec![op(Op::Add, vec![X, X1]), X1, X];
    assert!(
        close(area(skew), 3.0_f64.sqrt()),
        "columns (1,0,1) and (1,1,0)"
    );
    // Parallel columns: the Gram determinant rounds to a few ulps either side of 0, and the area is 0 or nearly so,
    // never NaN, for each of these pairs, some of which round below 0.
    for (a0, k) in [
        (1.0, 1.0),
        (0.1, 1.0),
        (0.1, 3.0),
        (0.7, 3.0),
        (1.3, 1.3),
        (3.0, 1.3),
    ] {
        let col = |s: f64| op(Op::Mul, vec![num(s), op(Op::Add, vec![X, X1])]);
        let d = area(vec![col(a0), col(1.0), col(a0 * k)]);
        assert!(
            d.is_finite() && d <= 1e-6,
            "parallel columns ({a0}, {k}): {d}"
        );
    }
    let eps = 64.0 * f64::EPSILON;
    let h = f64::EPSILON.cbrt();
    let r = 2.0 * eps / (2.0 * h);
    let one = link(vec![X], vec![X], num(0.0));
    assert_eq!(
        volume::<f64>(&one, &[0.0], 1.0, &v),
        (1.0, r),
        "the identity's"
    );
    let shifted = link(vec![op(Op::Add, vec![X, num(10.0)])], vec![X], num(0.0));
    let (_, r10) = volume::<f64>(&shifted, &[0.0], 1.0, &v);
    assert_eq!(
        r10,
        2.0 * eps * (10.0 + h) / (2.0 * h),
        "the shifted identity's"
    );
    let plane = link(vec![X, X1, num(0.0)], vec![X], num(0.0));
    let (a, r2) = volume::<f64>(&plane, &[0.0, 0.0], 1.0, &v);
    assert_eq!((a, r2), (1.0, r + r + r * r), "the plane's");
    let diag = link(
        vec![
            op(Op::Mul, vec![num(2.0), X]),
            op(Op::Mul, vec![num(3.0), X1]),
            num(0.0),
        ],
        vec![X],
        num(0.0),
    );
    let (_, r23) = volume::<f64>(&diag, &[0.0, 0.0], 1.0, &v);
    let want = r * 3.0 + 2.0 * r + r * r;
    assert!(
        (r23 - want).abs() <= 1e-12 * want,
        "diag(2, 3)'s rounding, r·|b| + |a|·r + r²: {r23}, not {want}"
    );
    let cube = link(vec![op(Op::Mul, vec![X, X, X])], vec![X], num(0.0));
    let (d1, _) = volume::<f64>(&cube, &[1.0], 1.0, &v);
    let (d2, _) = volume::<f64>(&cube, &[1.0], 2.0, &v);
    assert!(
        d1 != d2 && close(d1, 3.0) && close(d2, 3.0),
        "x³'s two steps: {d1}, {d2}"
    );
    let (d, _) = volume::<f64>(&cube, &[2.0], 1.0, &v);
    assert!(close(d, 12.0), "x³'s slope at 2: {d}");
}

#[test]
fn link_admissible_volume() {
    check_volume(6.0);
}

negative_control!(
    link_admissible_volume,
    "diag(2, 3)'s area is 6, not 5",
    expected = "the area of diag(2, 3)",
    check_volume(5.0)
);

// ── The entries' check ──────────────────────────────────────────────────────────────────────────────────────────

/// `tanh_q` named `name` with `edit` applied, and a `q_max` row on it over `[1, 3]`.
fn injected(
    name: &'static str,
    edit: impl Fn(&mut LinkBuilder),
) -> (Vec<LinkBuilder>, Vec<Tunable>) {
    let mut b = *builders().iter().find(|b| b.name == "tanh_q").unwrap();
    b.name = name;
    b.sampling_note = Some("an injected link");
    edit(&mut b);
    let mut all = builders().to_vec();
    all.push(b);
    let mut t = tunables().to_vec();
    t.push(Tunable {
        entry: name,
        param: "q_max",
        default: 2.0,
        lo: 1.0,
        hi: 3.0,
        citation: Citation::Calibration("REQ-GEN-038"),
    });
    (all, t)
}

/// The refusal lines of `name` among `check_with`'s.
fn refused(b: &[LinkBuilder], t: &[Tunable], name: &str) -> Vec<String> {
    let lines = check_with(b, &slots(), t).err().unwrap_or_default();
    lines
        .into_iter()
        .filter(|l| l.contains(&format!("link `{name}`")))
        .collect()
}

/// A tunable entry whose log-det is off by a constant factor is refused for C¹ once, at the range's first value; one
/// whose forward leaves its codomain without a row is refused at f64 and at f32.
fn check_entries(factor: f64) {
    let (b, t) = injected("scaled_ld_q", |b| {
        let ld = b.log_det.unwrap();
        b.log_det = Some(op(Op::Add, vec![ld, op(Op::Log, vec![num(1.5)])]));
    });
    let lines = refused(&b, &t, "scaled_ld_q");
    assert_eq!(lines.len(), 1, "one line per row: {lines:?}");
    assert!(
        lines[0].contains("link `scaled_ld_q`: with `q_max` = 1, C¹ fails at f64"),
        "{lines:?}"
    );
    let (b, _) = injected("leaking_q", |b| {
        let f = b.forward.unwrap()[0];
        b.forward = Some(leak(vec![op(Op::Mul, vec![num(factor), f])]));
    });
    let lines = refused(&b, tunables(), "leaking_q");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("constraint-preserving fails at f64"))
            && lines
                .iter()
                .any(|l| l.contains("constraint-preserving fails at f32")),
        "the leaking link at f64 and f32: {lines:?}"
    );
    // `3·tanh(x/20)` stays inside `[−2, 2]` over the drawn controls and leaves it only at the saturating ±20 and past,
    // so the fuzz is run past its first controls when they pass.
    let (b, _) = injected("late_leak_q", |b| {
        b.forward = Some(leak(vec![op(
            Op::Mul,
            vec![
                num(factor * 2.0),
                op(Op::Tanh, vec![op(Op::Mul, vec![num(0.05), X])]),
            ],
        )]));
    });
    let lines = refused(&b, tunables(), "late_leak_q");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("constraint-preserving fails at f64")),
        "the link leaking only at saturating controls: {lines:?}"
    );
    let (b, t) = injected("exact_q", |_| {});
    assert!(
        refused(&b, &t, "exact_q").is_empty(),
        "a copy of tanh_q with a row is refused"
    );
}

#[test]
fn link_admissible_entries() {
    check_entries(1.5);
}

negative_control!(
    link_admissible_entries,
    "a forward scaled by 1 keeps its codomain, so the leak's arm fails",
    expected = "the leaking link at f64 and f32",
    check_entries(1.0)
);

// ── The seeded sample ───────────────────────────────────────────────────────────────────────────────────────────

/// The fuzz and the interior sample are seeded, so the check is reproducible: their sizes, the saturating controls,
/// and the first three interior values of `[0, 1]`'s sample, by their bits.
fn check_sample(pinned: [u64; 3]) {
    let one = controls(1);
    assert_eq!(one.len(), 64 + 11, "one control's fuzz");
    assert_eq!(
        one[64..].iter().map(|v| v[0]).collect::<Vec<_>>(),
        [0.0, 0.5, -0.5, 20.0, -20.0, 40.0, -40.0, 1e3, -1e3, 1e8, -1e8],
        "the saturating controls"
    );
    assert!(
        one[..64].iter().all(|v| v[0] >= -8.0 && v[0] < 8.0),
        "the draws' box"
    );
    let two = controls(2);
    assert_eq!(two.len(), 64 + 11 * 13, "two controls' fuzz");
    assert!(
        two.contains(&vec![1e8, -1e8]),
        "a pair of saturating controls"
    );
    let mixed: Vec<f64> = (0..11)
        .map(|i| {
            let (ad, da) = (&two[64 + 13 * i + 11], &two[64 + 13 * i + 12]);
            assert_eq!(
                (ad[0], da[1]),
                (one[64 + i][0], one[64 + i][0]),
                "the saturating control beside a draw"
            );
            assert_eq!(
                ad[1], da[0],
                "the draw beside a saturating control, each way"
            );
            ad[1]
        })
        .collect();
    assert!(
        mixed.iter().all(|&d| (-8.0..8.0).contains(&d))
            && mixed.iter().any(|&d| d < 0.0)
            && mixed.iter().any(|&d| d > 0.0),
        "the draws beside the saturating controls span the box: {mixed:?}"
    );
    let r = range_values(1.0, 3.0);
    assert_eq!(r[..2], [1.0, 3.0], "the ends");
    assert_eq!(range_values(1.0, 3.0), r, "the sample is not reproducible");
    let unit = range_values(0.0, 1.0);
    let got: Vec<u64> = unit[2..5].iter().map(|v| v.to_bits()).collect();
    assert_eq!(
        got,
        pinned,
        "the seeded interior's first values: {:?}",
        &unit[2..5]
    );
    let first: Vec<u64> = one[..2].iter().map(|v| v[0].to_bits()).collect();
    assert_eq!(first, FUZZ, "the fuzz's first draws: {:?}", &one[..2]);
}

/// The first three interior values of `range_values(0, 1)`, splitmix64 from its seed, by their bits.
const PINNED: [u64; 3] = [
    4604904788677098426,
    4602623176530973818,
    4607170485509237997,
];

/// The first two draws of one control's fuzz, by their bits.
const FUZZ: [u64; 2] = [4616472085648620650, 13842846885110693694];

#[test]
fn link_admissible_sample() {
    check_sample(PINNED);
}

negative_control!(
    link_admissible_sample,
    "other interior values are not the seeded ones",
    expected = "the seeded interior's first values",
    check_sample([0, 1, 2])
);
