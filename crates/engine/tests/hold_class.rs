//! A projection under a hold is classed by `H`'s move, never by its decoded value, and the moves' rounding bounds hold.
//! - `hold_class_by_the_move`: each case is run through the lookup's own step 4 (`lookup::step4`) and its verdict must be
//!   the move form's. The control classes `y` by its decoded value, `Classes::of(&y)`, and must disagree.
//! - `hold_class_bounds`: over states, gauge images and targets, each move and each value after it, formed as `held_moves`
//!   forms it, is within its bound of exact arithmetic: `|form − exact| ≤ (k/4)·ε·bound`, `k` the register's `zero_class`.
//!
//! The cases are random latent states imaged to offsets 0, 8, 10⁴ and 10⁶ with `L*` or `E*` near the zero-class band,
//! kept where the two forms disagree, plus cases at the floor `E* = U`, with `η_E` near 0, with both holds, with `E < 0`
//! under a target at or above 0, and with `L* = 0` exactly. The bound draws include `E*` at and just above `U`.

use engine::chart_authoring::composed::Composed;
#[cfg(feature = "controls")]
use engine::lookup::changed;
use engine::lookup::{
    held_moves, phys, step4, Classes, Exact, Extents, Failed, Layer, Rejected, Tolerances,
};
use kernel::chart::{Latent, LzHold};
use kernel::decode::hold::{hold_ic, Targets};
use kernel::decode::{self, Ic};
use kernel::encode::encode_at;
use kernel::generated::links::{Defaults, LinkParams};
use ledger::constants::ZERO_CLASS;
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use validation::negative_control;

/// A held entry and its two verdicts: the move form's, and the decoded-value form's.
struct Case {
    name: &'static str,
    m: [f64; 3],
    r: [f64; 6],
    p: [f64; 6],
    e: Option<f64>,
    lz: Option<f64>,
    /// The projection is rejected by the class rule, read by `H`'s move.
    rejects: bool,
    /// It is rejected when `y` is classed by its decoded value, `changed(&before, &Classes::of(&y))`.
    decoded_rejects: bool,
}

const CASES: &[Case] = &[
    Case {
        name: "rest_in_band 0",
        m: [
            7.3504688755316894e-3,
            9.625602134016897e-1,
            3.0089317722778508e-2,
        ],
        r: [
            7.580784387844939e3,
            -3.472797923614302e3,
            7.592347833946728e3,
            -3.472797923614302e3,
            7.59271724612085e3,
            -3.472001804228085e3,
        ],
        p: [
            1.751184542649754e-1,
            3.7828744432574507e-1,
            2.2932150256289056e1,
            4.953758043238165e1,
            7.168515231784529e-1,
            1.5485285762851275e0,
        ],
        e: None,
        lz: Some(2.3918179303334726e-12),
        rejects: false,
        decoded_rejects: true,
    },
    Case {
        name: "rest_in_band 1",
        m: [
            6.9812934473096775e-3,
            7.708718560454271e-5,
            9.929416193670857e-1,
        ],
        r: [
            -8.66361751764569e3,
            -1.1643594218418305e3,
            -8.558562833308542e3,
            -1.1643594218418305e3,
            -8.661394966979318e3,
            -1.1689921278662375e3,
        ],
        p: [
            6.384670656922791e-1,
            2.731217265324112e-1,
            7.0499298685654105e-3,
            3.0158000640913628e-3,
            9.080846220056071e1,
            3.884580006186882e1,
        ],
        e: None,
        lz: Some(3.754609074043066e-11),
        rejects: false,
        decoded_rejects: true,
    },
    Case {
        name: "rest_past_band 0",
        m: [
            4.3211495604429245e-2,
            2.2701761358676498e-1,
            7.297708908088059e-1,
        ],
        r: [
            -8.486487600276429e3,
            8.702268688147087e3,
            -8.485810713842046e3,
            8.702268688147087e3,
            -8.4848291954817e3,
            8.704217776345144e3,
        ],
        p: [
            -2.50218653507982e0,
            -2.80022224262055e0,
            -1.314558563634932e1,
            -1.4711357756548844e1,
            -4.2257803650004405e1,
            -4.729113519158931e1,
        ],
        e: None,
        lz: Some(1.871787538607687e-12),
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "rest_past_band 1",
        m: [
            5.347265655515463e-3,
            7.688383829336004e-1,
            2.258143514108841e-1,
        ],
        r: [
            -5.820380130218261e3,
            2.4371605333230863e2,
            -5.807848568718037e3,
            2.4371605333230863e2,
            -5.808273653774231e3,
            2.4280210230239936e2,
        ],
        p: [
            1.5327074463963228e-1,
            4.6400472559965533e-1,
            2.2037512076516453e1,
            6.671533936893934e1,
            6.47260413466879e0,
            1.9594860796713306e1,
        ],
        e: None,
        lz: Some(6.683646746863546e-12),
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "e_in_band 0",
        m: [
            7.469694921145666e-3,
            2.886491819469485e-4,
            9.922416558969073e-1,
        ],
        r: [
            -1.9708300483827068e3,
            -7.340324168155578e2,
            -1.9127007891290352e3,
            -7.340324168155578e2,
            -1.9686440749941237e3,
            -7.312189610808134e2,
        ],
        p: [
            -2.1142862889194908e-1,
            4.436279361949653e-1,
            -9.167424604198018e-3,
            1.759876373988069e-2,
            -2.8121892219688718e1,
            5.887701819202849e1,
        ],
        e: Some(4.664158952137607e-13),
        lz: None,
        rejects: false,
        decoded_rejects: true,
    },
    Case {
        name: "e_in_band 1",
        m: [
            1.8380811186377893e-1,
            2.659525048535219e-3,
            8.135323630876858e-1,
        ],
        r: [
            -7.863844617461789e2,
            -5.29528665828863e3,
            -7.848169153490846e2,
            -5.29528665828863e3,
            -7.886537507604244e2,
            -5.296425938941582e3,
        ],
        p: [
            1.4605856874391597e1,
            -1.4976852305791205e1,
            1.9799944914825138e-1,
            -2.0503305039194683e-1,
            6.465710626852153e1,
            -6.633026272881082e1,
        ],
        e: Some(2.693234387232536e-12),
        lz: None,
        rejects: false,
        decoded_rejects: true,
    },
    Case {
        name: "near_rigid_to_zero 0",
        m: [
            8.45085851087246e-3,
            5.137985417565104e-1,
            4.7775059973261713e-1,
        ],
        r: [
            -1.071981627166835e1,
            1.3721798277188682e-1,
            8.763499868414411e-2,
            1.3721798277188682e-1,
            9.537385420169418e-2,
            -1.4999878440471884e-1,
        ],
        p: [
            -1.4047018115816165e-3,
            -1.097388097646329e-1,
            -8.540355445939479e-2,
            5.454344149331302e-2,
            8.68082562709764e-2,
            5.519536827131986e-2,
        ],
        e: None,
        lz: Some(0.0),
        rejects: true,
        decoded_rejects: false,
    },
    Case {
        name: "near_rigid_to_zero 1",
        m: [
            3.9170590802336085e-3,
            4.368056337132873e-1,
            5.592773072064791e-1,
        ],
        r: [
            -1.5399331447823341e1,
            -8.079413923352279e-3,
            3.5491346983877636e-1,
            -8.079413923352279e-3,
            -1.6934016597623155e-1,
            6.366754049576841e-3,
        ],
        p: [
            2.0897363606285936e-5,
            -3.97423115031695e-2,
            2.3264297259599914e-3,
            1.021437587174117e-1,
            -2.3473270895662773e-3,
            -6.2401447214242206e-2,
        ],
        e: None,
        lz: Some(0.0),
        rejects: true,
        decoded_rejects: false,
    },
    Case {
        name: "moving_to_zero 0",
        m: [
            7.595572127704043e-3,
            6.802447930602493e-5,
            9.9233640339299e-1,
        ],
        r: [
            -1.4654844962176745e0,
            1.491686307229875e0,
            1.1919745245648747e2,
            1.491686307229875e0,
            3.0461933521338546e-3,
            -1.1519966498984737e-2,
        ],
        p: [
            7.863381440046185e-1,
            -5.23814822563995e-1,
            1.6719087509473443e-2,
            1.3814167775140946e0,
            -8.03057231514092e-1,
            -8.576019549500997e-1,
        ],
        e: None,
        lz: Some(0.0),
        rejects: true,
        decoded_rejects: false,
    },
    Case {
        name: "moving_to_zero 1",
        m: [
            5.327601776858695e-1,
            4.333033942059407e-1,
            3.393642810818975e-2,
        ],
        r: [
            8.201213849680854e-2,
            -1.0507779851885796e-1,
            2.4361000329901863e-1,
            -1.0507779851885796e-1,
            -4.397924327632027e0,
            2.9912350539672268e0,
        ],
        p: [
            4.5375455210695526e-1,
            -4.932987305884944e-1,
            -1.2957309256959282e0,
            -7.110334106499019e-1,
            8.419763735889729e-1,
            1.2043321412383965e0,
        ],
        e: None,
        lz: Some(0.0),
        rejects: true,
        decoded_rejects: false,
    },
    Case {
        name: "eta_near_zero",
        m: [
            0.007245833709871785,
            0.991382470920751,
            0.0013716953693771166,
        ],
        r: [
            -10.141395938749488,
            -0.018496045569399752,
            0.07339228107368492,
            -0.018496045569399752,
            0.5271196629731021,
            13.465580654200139,
        ],
        p: [
            -0.2963783015757848,
            1.4449745380054377,
            -1.1182705842708678,
            -0.9566922632514232,
            1.4146488858466526,
            -0.48828227475401453,
        ],
        e: Some(-0.0008036384090168006),
        lz: None,
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "e_at_the_floor",
        m: [
            0.007245833709871785,
            0.991382470920751,
            0.0013716953693771166,
        ],
        r: [
            -10.141395938749488,
            -0.018496045569399752,
            0.07339228107368492,
            -0.018496045569399752,
            0.5271196629731021,
            13.465580654200139,
        ],
        p: [
            -0.2963783015757848,
            1.4449745380054377,
            -1.1182705842708678,
            -0.9566922632514232,
            1.4146488858466526,
            -0.48828227475401453,
        ],
        e: Some(-0.0008046060228180655),
        lz: None,
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "e_below_zero_to_above",
        m: [0.014739227915616663, 0.1680674084078079, 0.8171933636765755],
        r: [
            -2.5839509053049783,
            -0.9348954201602722,
            -1.8260713465068092,
            -0.9348954201602722,
            0.4221626550405363,
            0.2091366556193841,
        ],
        p: [
            -0.021235838120686776,
            0.008347619060624412,
            0.0056375216471637275,
            -0.013196875849097919,
            0.01559831647352305,
            0.0048492567884735075,
        ],
        e: Some(0.5),
        lz: None,
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "lz_pushes_e_across_zero",
        m: [0.014739227915616663, 0.1680674084078079, 0.8171933636765755],
        r: [
            -2.5839509053049783,
            -0.9348954201602722,
            -1.8260713465068092,
            -0.9348954201602722,
            0.4221626550405363,
            0.2091366556193841,
        ],
        p: [
            -0.021235838120686776,
            0.008347619060624412,
            0.0056375216471637275,
            -0.013196875849097919,
            0.01559831647352305,
            0.0048492567884735075,
        ],
        e: None,
        lz: Some(10.0),
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "both_held_e_across_zero",
        m: [0.014739227915616663, 0.1680674084078079, 0.8171933636765755],
        r: [
            -2.5839509053049783,
            -0.9348954201602722,
            -1.8260713465068092,
            -0.9348954201602722,
            0.4221626550405363,
            0.2091366556193841,
        ],
        p: [
            -0.021235838120686776,
            0.008347619060624412,
            0.0056375216471637275,
            -0.013196875849097919,
            0.01559831647352305,
            0.0048492567884735075,
        ],
        e: Some(0.5),
        lz: Some(-0.013282469430703847),
        rejects: true,
        decoded_rejects: true,
    },
    Case {
        name: "both_held_keeps_classes",
        m: [0.014739227915616663, 0.1680674084078079, 0.8171933636765755],
        r: [
            -2.5839509053049783,
            -0.9348954201602722,
            -1.8260713465068092,
            -0.9348954201602722,
            0.4221626550405363,
            0.2091366556193841,
        ],
        p: [
            -0.021235838120686776,
            0.008347619060624412,
            0.0056375216471637275,
            -0.013196875849097919,
            0.01559831647352305,
            0.0048492567884735075,
        ],
        e: Some(-0.04306445447631035),
        lz: Some(-0.013282469430703847),
        rejects: false,
        decoded_rejects: false,
    },
    Case {
        name: "rest_to_exact_zero",
        m: [
            0.0073504688755316894,
            0.9625602134016897,
            0.030089317722778508,
        ],
        r: [
            7580.784387844939,
            -3472.797923614302,
            7592.347833946728,
            -3472.797923614302,
            7592.71724612085,
            -3472.001804228085,
        ],
        p: [
            0.1751184542649754,
            0.37828744432574507,
            22.932150256289056,
            49.53758043238165,
            0.7168515231784529,
            1.5485285762851275,
        ],
        e: None,
        lz: Some(0.0),
        rejects: false,
        decoded_rejects: false,
    },
];

fn entry(c: &Case) -> Ic<f64> {
    let two = |a: &[f64; 6]| [[a[0], a[1]], [a[2], a[3]], [a[4], a[5]]];
    Ic {
        m: c.m,
        r: two(&c.r),
        p: two(&c.p),
    }
}

fn targets(c: &Case) -> Targets<f64> {
    Targets {
        e_star: c.e,
        lz_star: c.lz,
    }
}

fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

fn plane() -> Latent<f64> {
    let dir = |k: usize| {
        let mut q = [0.0; 8];
        q[k] = 1.0;
        q
    };
    Latent {
        z0: [0.4, -0.2, 0.6, -0.3, 0.2, 0.5, 0.2, -0.1],
        q1: dir(2),
        q2: dir(3),
    }
}

/// A verdict on a case: `true` where the class rule rejects the projection.
type Verdict = fn(&Case) -> bool;

/// The lookup's step 4: rejected by the class rule, or not.
fn by_the_move(c: &Case) -> bool {
    match step4::<Defaults, _, _>(
        &plane(),
        &entry(c),
        &phys,
        &tol(),
        &LinkParams::DEFAULT,
        &targets(c),
    ) {
        Ok(_) => false,
        Err(Rejected {
            layer: Layer::Hold,
            failed: Failed::HeldClass(_),
        }) => true,
        Err(other) => panic!("{}: refused for another reason: {other:?}", c.name),
    }
}

/// The decoded-value form: `y` classed by its decoded value.
#[cfg(feature = "controls")]
fn by_the_decoded_value(c: &Case) -> bool {
    let band = tol().zero_class;
    let x = entry(c);
    let e = encode_at::<Defaults, f64>(&x, &LinkParams::DEFAULT);
    let ext = Extents::entered(&x, e.report.lambda);
    let before = Classes::within(&e.canonical.ic, &ext);
    let (y, _) = hold_ic(&e.canonical.ic, &targets(c), false, LzHold::Free);
    changed(&before, &Classes::of(&y), band).is_some()
}

fn check_cases(verdict: Verdict) {
    for c in CASES {
        assert_eq!(
            verdict(c),
            c.rejects,
            "{}: the verdict disagrees with the contract's move form",
            c.name
        );
    }
}

#[test]
fn hold_class_by_the_move() {
    check_cases(by_the_move);
    // Eight cases separate the two forms.
    assert_eq!(
        CASES
            .iter()
            .filter(|c| c.rejects != c.decoded_rejects)
            .count(),
        8,
        "the cases that separate the forms"
    );
}

negative_control!(
    hold_class_by_the_move,
    "y classed by its decoded value disagrees with the move form",
    expected = "the verdict disagrees with the contract's move form",
    check_cases(by_the_decoded_value)
);

/// The same through `Composed`: an entry whose projection the move form accepts is locked, with `lookup_held` raised,
/// where `constraints` are the case's holds.
fn check_through_composed(constraints: engine::chart_authoring::registry::Constraints) {
    use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Primitive, Registry};
    use engine::chart_authoring::validate::validate_spec;
    let c = &CASES[0];
    assert!(
        !c.rejects,
        "{}: the case is one the move form accepts",
        c.name
    );
    let dir = |k: usize| {
        let mut q = [0.0; 8];
        q[k] = 1.0;
        AxisSpec::of(Primitive::Direction(q))
    };
    let mut spec = ChartSpec::new(dir(2), dir(3), plane().z0);
    spec.constraints = constraints;
    let valid = validate_spec(&Registry::standard(), &spec).expect("validates");
    let composed = Composed::new(&valid, Default::default()).expect("composes");
    let (_, held) = composed
        .lookup_held(&Exact::Physical(entry(c)), &tol())
        .unwrap_or_else(|e| panic!("{}: the move form's accepted entry is refused: {e}", c.name));
    assert!(held.is_some(), "{}: lookup_held is not raised", c.name);
}

#[test]
fn hold_class_by_the_move_through_composed() {
    use engine::chart_authoring::registry::Constraints;
    check_through_composed(Constraints {
        hold_e: CASES[0].e,
        hold_lz: CASES[0].lz,
    });
}

negative_control!(
    hold_class_by_the_move_through_composed,
    "a chart with no holds locks the entry unprojected, raising no lookup_held",
    expected = "lookup_held is not raised",
    check_through_composed(Default::default())
);

// ── The moves' rounding bounds, against exact arithmetic ───────────────────────────────────────────────────────────

fn q(x: f64) -> BigRational {
    BigRational::from_float(x).expect("finite")
}

fn int(n: i64) -> BigRational {
    BigRational::from_integer(BigInt::from(n))
}

/// `√v` to `2⁻¹⁶⁰`, for `v ≥ 0`.
fn sqrt(v: &BigRational) -> BigRational {
    let scale = BigInt::from(1) << 320;
    let n = (v.numer() * &scale) / v.denom();
    let root = BigUint::try_from(n).expect("non-negative").sqrt();
    BigRational::new(BigInt::from(root), BigInt::from(1) << 160)
}

fn lz_exact(x: &Ic<f64>) -> BigRational {
    (0..3)
        .map(|i| q(x.r[i][0]) * q(x.p[i][1]) - q(x.r[i][1]) * q(x.p[i][0]))
        .fold(int(0), |a, b| a + b)
}

fn k_exact(x: &Ic<f64>) -> BigRational {
    (0..3)
        .map(|i| (q(x.p[i][0]) * q(x.p[i][0]) + q(x.p[i][1]) * q(x.p[i][1])) / (int(2) * q(x.m[i])))
        .fold(int(0), |a, b| a + b)
}

fn u_exact(x: &Ic<f64>) -> BigRational {
    let d2 = |i: usize, j: usize| {
        let (dx, dy) = (q(x.r[i][0]) - q(x.r[j][0]), q(x.r[i][1]) - q(x.r[j][1]));
        &dx * &dx + &dy * &dy
    };
    -[(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| q(x.m[i]) * q(x.m[j]) / sqrt(&d2(i, j)))
        .fold(int(0), |a, b| a + b)
}

/// The exact moves of `H` on the f64 state `x`, and the values after them, from the contract's forms, as exact
/// rationals: `(ΔL_z, ΔE, L_z after, E after)`.
fn exact(x: &Ic<f64>, t: &Targets<f64>) -> [BigRational; 4] {
    let (l0, e0) = (lz_exact(x), k_exact(x) + u_exact(x));
    match (t.lz_star, t.e_star) {
        (Some(l), None) => {
            let l = q(l);
            let de = (&l * &l - &l0 * &l0) / int(2);
            [&l - &l0, de.clone(), l, &e0 + de]
        }
        (None, Some(e)) => {
            let e = q(e);
            let eta = sqrt(&((&e - u_exact(x)) / k_exact(x)));
            [(&eta - int(1)) * &l0, &e - &e0, &eta * &l0, e]
        }
        (Some(l), Some(e)) => {
            let (l, e) = (q(l), q(e));
            [&l - &l0, &e - &e0, l, e]
        }
        (None, None) => unreachable!(),
    }
}

/// `|form − exact|` over `ε·scale`, as a number; 0 where both are exactly equal, and a panic where the bound is 0 and
/// they differ.
fn ratio(form: f64, truth: &BigRational, scale: f64, what: &str) -> f64 {
    let d = q(form) - truth;
    let diff = if d < int(0) { -d } else { d };
    if scale == 0.0 {
        assert!(
            diff == int(0),
            "{what}: a value with no bound differs from exact by {diff}"
        );
        return 0.0;
    }
    let ratio = diff / q(f64::EPSILON * scale) * int(1 << 20);
    let n = ratio.numer() / ratio.denom();
    n.to_string().parse::<f64>().expect("a number") / f64::from(1 << 20)
}

/// A moves function under test: the contract's forms, or a control's.
type Moves = fn(&Classes, &Ic<f64>, &Extents, &Targets<f64>) -> engine::lookup::Moves;

fn contract_moves(
    b: &Classes,
    c: &Ic<f64>,
    e: &Extents,
    t: &Targets<f64>,
) -> engine::lookup::Moves {
    held_moves(b, c, e, t, false, LzHold::Free)
}

/// `ΔE` as a difference of two rounded energies, `E(y) − E(C(x))`, which the contract forbids.
#[cfg(feature = "controls")]
fn rounded_difference(
    b: &Classes,
    c: &Ic<f64>,
    e: &Extents,
    t: &Targets<f64>,
) -> engine::lookup::Moves {
    let mut m = held_moves(b, c, e, t, false, LzHold::Free);
    let (y, _) = hold_ic(c, t, false, LzHold::Free);
    m.e = Classes::of(&y).e - b.e;
    m
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn u(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn between(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.u()
    }
}

fn state(rng: &mut Rng) -> Ic<f64> {
    let mut z = [0.0; 8];
    for v in &mut z {
        *v = rng.between(-3.0, 3.0);
    }
    decode::decode::<Defaults, f64>(&decode::Latent::new(z)).ic
}

/// `x` with its momentum nearly the rigid rotation's: `‖δv‖/‖v‖ ≈ ratio`.
fn near_rigid(x: &Ic<f64>, ratio: f64, rng: &mut Rng) -> Ic<f64> {
    let w = [
        [-x.r[0][1], x.r[0][0]],
        [-x.r[1][1], x.r[1][0]],
        [-x.r[2][1], x.r[2][0]],
    ];
    let c = rng.between(0.5, 2.0);
    let mut d = [[0.0; 2]; 3];
    for b in &mut d {
        *b = [rng.between(-1.0, 1.0), rng.between(-1.0, 1.0)];
    }
    for k in 0..2 {
        let mean: f64 = (0..3).map(|i| x.m[i] * d[i][k]).sum();
        for b in &mut d {
            b[k] -= mean;
        }
    }
    let mut y = *x;
    for i in 0..3 {
        for k in 0..2 {
            y.p[i][k] = x.m[i] * (c * w[i][k] + ratio * d[i][k]);
        }
    }
    y
}

/// `x` translated and boosted: offsets and boosts as large as the gauge images of the zero-class measurement.
fn image(x: &Ic<f64>, rng: &mut Rng, off: f64, boost: f64) -> Ic<f64> {
    let t = [rng.between(-off, off), rng.between(-off, off)];
    let v = [rng.between(-boost, boost), rng.between(-boost, boost)];
    let mut y = *x;
    for i in 0..3 {
        for k in 0..2 {
            y.r[i][k] += t[k];
            y.p[i][k] += x.m[i] * v[k];
        }
    }
    y
}

/// The worst ratio of each row over the draws: `[ΔL_z, ΔE, L_z after, E after]` for each of hold `L_z`, hold `E` and both,
/// and the number of floor draws used and of those with `E*` at `U` or below its rounding (the `q ≤ 0` branch).
fn worst(moves: Moves, draws: usize) -> ([[f64; 4]; 3], [usize; 2]) {
    let mut rng = Rng(5);
    let mut worst = [[0.0f64; 4]; 3];
    for n in 0..draws {
        let x0 = state(&mut rng);
        let (off, boost) = [(0.0, 0.0), (8.0, 8.0), (1e4, 1e2)][n % 3];
        let x0 = match n % 4 {
            0 => near_rigid(&x0, [1e-3, 1e-5][n / 4 % 2], &mut rng),
            _ => x0,
        };
        let x = image(&x0, &mut rng, off, boost);
        let c = encode_at::<Defaults, f64>(&x, &LinkParams::DEFAULT);
        let ext = Extents::entered(&x, c.report.lambda);
        let before = Classes::within(&c.canonical.ic, &ext);
        let ic = c.canonical.ic;
        let d = kernel::decode::descriptor::describe(&ic);
        // Targets: `L*` near and far from `L₀`, `E*` above `U` and above the floor `U + L*²/2`.
        let l0 = before.lz;
        let l = match n % 5 {
            0 => l0 * (1.0 + 1e-6),
            1 => l0 * (1.0 - 1e-9),
            2 => 0.0,
            _ => rng.between(-2.0, 2.0),
        };
        let eta_e = d.V_0 + d.K_0 * rng.between(0.2, 3.0);
        let both_e = d.V_0 + 0.5 * l * l + rng.between(0.05, 2.0);
        for (row, t) in [
            (
                0,
                Targets {
                    e_star: None,
                    lz_star: Some(l),
                },
            ),
            (
                1,
                Targets {
                    e_star: Some(eta_e),
                    lz_star: None,
                },
            ),
            (
                2,
                Targets {
                    e_star: Some(both_e),
                    lz_star: Some(l),
                },
            ),
        ] {
            let (y, o) = hold_ic(&ic, &t, false, LzHold::Free);
            if o.tag().is_some() || phys(&y, &ic) <= tol().eps_phys {
                continue;
            }
            let m = moves(&before, &ic, &ext, &t);
            let truth = exact(&ic, &t);
            let rows = [
                (m.lz, m.lz_scale, "ΔL_z"),
                (m.e, m.e_scale, "ΔE"),
                (m.lz_after, m.lz_after_scale, "L_z after"),
                (m.e_after, m.e_after_scale, "E after"),
            ];
            for (k, (form, scale, what)) in rows.iter().enumerate() {
                let r = ratio(*form, &truth[k], *scale, what);
                worst[row][k] = worst[row][k].max(r);
            }
        }
    }
    // Hold `E` alone at and just above the floor `U`: `η_E` near 0 and the `q ≤ 0` branch.
    let mut used = [0usize; 2];
    for n in 0..draws {
        let x0 = state(&mut rng);
        let (off, boost) = [(0.0, 0.0), (8.0, 8.0), (1e4, 1e2)][n % 3];
        let x = image(&x0, &mut rng, off, boost);
        let c = encode_at::<Defaults, f64>(&x, &LinkParams::DEFAULT);
        let ext = Extents::entered(&x, c.report.lambda);
        let before = Classes::within(&c.canonical.ic, &ext);
        let ic = c.canonical.ic;
        let d = kernel::decode::descriptor::describe(&ic);
        let t = Targets {
            e_star: Some(d.V_0 + d.K_0 * [0.0, 1e-14, 1e-10, 1e-6][n % 4]),
            lz_star: None,
        };
        let (y, o) = hold_ic(&ic, &t, false, LzHold::Free);
        if o.tag().is_some() || phys(&y, &ic) <= tol().eps_phys {
            continue;
        }
        // The exact `√((E* − U)/K)` is real only where `E*` is not below the exact `U`.
        if q(t.e_star.expect("set")) < u_exact(&ic) {
            continue;
        }
        used[0] += 1;
        used[1] += usize::from(t.e_star == Some(d.V_0));
        let m = moves(&before, &ic, &ext, &t);
        let truth = exact(&ic, &t);
        let rows = [
            (m.lz, m.lz_scale, "ΔL_z"),
            (m.e, m.e_scale, "ΔE"),
            (m.lz_after, m.lz_after_scale, "L_z after"),
            (m.e_after, m.e_after_scale, "E after"),
        ];
        for (k, (form, scale, what)) in rows.iter().enumerate() {
            let r = ratio(*form, &truth[k], *scale, what);
            worst[1][k] = worst[1][k].max(r);
        }
    }
    (worst, used)
}

/// Every ratio at most `k/4`: the zero-class band, `k·ε`, has its 4× margin over what the moves' roundings reach.
fn check_bounds(moves: Moves) {
    let limit = ZERO_CLASS.number() / 4.0;
    let (w, floor) = worst(moves, 3000);
    assert!(
        floor[0] >= 300 && floor[1] >= 100,
        "the floor draws used: {floor:?}"
    );
    println!("worst |form − exact| / (ε·bound), rows [ΔL_z, ΔE, L_z after, E after], for hold L_z, hold E, both: {w:?}");
    for (row, name) in ["hold L_z", "hold E", "both"].iter().enumerate() {
        for (k, what) in ["ΔL_z", "ΔE", "L_z after", "E after"].iter().enumerate() {
            assert!(
                w[row][k] <= limit,
                "the bound on {what} under {name} is exceeded: {} of ε·bound, past k/4 = {limit}",
                w[row][k]
            );
        }
    }
}

#[test]
fn hold_class_bounds() {
    check_bounds(contract_moves);
}

/// `η_E`'s rounding bound dropped: the moves it feeds carry only their own value as scale.
#[cfg(feature = "controls")]
fn eta_bound_dropped(
    b: &Classes,
    c: &Ic<f64>,
    e: &Extents,
    t: &Targets<f64>,
) -> engine::lookup::Moves {
    let mut m = held_moves(b, c, e, t, false, LzHold::Free);
    if t.lz_star.is_none() {
        m.lz_scale = m.lz.abs();
        m.lz_after_scale = m.lz_after.abs();
    }
    m
}

negative_control!(
    hold_class_bounds,
    "ΔE as a difference of two rounded energies exceeds the product form's bound",
    expected = "the bound on ΔE under hold L_z is exceeded",
    check_bounds(rounded_difference)
);

#[cfg(feature = "controls")]
mod eta_bound {
    use super::*;
    negative_control!(
        hold_class_bounds,
        "η_E's rounding bound dropped leaves a value with no bound off its exact value",
        expected = "a value with no bound differs from exact",
        check_bounds(eta_bound_dropped)
    );
}
