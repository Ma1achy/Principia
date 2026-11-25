//! Encode, lookup and ingest under the holds, through `Composed` (inverse_encode_contract Part 6, "Under a hold, at step
//! 4"; chart_composition § 5.3, "Encode"), on the latent momentum plane:
//! - `hold_encode_landing_reads_the_held_decode`: a state the unheld decode puts on a pixel is not that pixel's under a
//!   hold, so the held encode does not land on it, where the unheld decode's step 4 does (the control).
//! - `hold_encode_projects_a_state_off_the_hold`: a state off `{L_z = L*}`, `{E = E*}` or both is projected,
//!   `y = H(C(x))`, `lookup_held` raised with the gaps, and the lock is `y`: its `L_z` and `E` are the targets.
//! - `hold_encode_state_on_the_hold`: a held decode looked up raises no notice and locks as itself.
//! - `hold_encode_rejects`: the class rule on the projection, the collision-free infeasible target and the rest start under
//!   `η_E` are rejected, naming the outcome and the reason.
//! - `hold_encode_ingest`: ingest under a hold ingests the projection.
//!
//! **Pass criteria, written before the runs.** The lock's decoded `L_z` and `E` equal the targets to `1e-9`; the notice's
//! gaps equal `L_z(C(x)) − L*` and `E(C(x)) − E*` to `1e-9`; an on-hold state raises no notice; the rejections name the
//! outcome and reason above; with the holds off every call is the unheld one (`composed_decode_encode` holds the bits).

use engine::chart_authoring::composed::Composed;
use engine::chart_authoring::registry::{AxisSpec, ChartSpec, Constraints, Primitive, Registry};
use engine::chart_authoring::validate::validate_spec;
use engine::lookup::{Class, Exact, Failed, Layer, Locked, Rejected, Tolerances};
use kernel::chart::ChartDecode as _;
use kernel::decode::energy_norm::Outcome;
use kernel::decode::{self, Ic};
use kernel::encode::by_kind::{invariants, Landing};
use kernel::generated::links::Defaults;
use validation::negative_control;

const Z0: [f64; 8] = [0.4, -0.2, 0.6, -0.3, 0.2, 0.5, 0.2, -0.1];

fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

/// The latent plane of momentum controls `z_q0, z_q1` through `Z0`, under the holds `(E*, L*)`.
fn plane(e: Option<f64>, lz: Option<f64>) -> Composed {
    let dir = |k: usize| {
        let mut q = [0.0; 8];
        q[k] = 1.0;
        AxisSpec::of(Primitive::Direction(q))
    };
    let mut spec = ChartSpec::new(dir(2), dir(3), Z0);
    spec.constraints = Constraints {
        hold_e: e,
        hold_lz: lz,
    };
    let valid = validate_spec(&Registry::standard(), &spec).expect("the plane validates");
    Composed::new(&valid, Default::default()).expect("the plane composes")
}

/// The unheld decode at `(u, v)`.
fn unheld(u: f64, v: f64) -> Ic<f64> {
    plane(None, None).decode(u, v).ic
}

/// `L_z`, `E` of `x`.
fn lz_e(x: &Ic<f64>) -> (f64, f64) {
    let i = invariants(x);
    (i.lz, i.e)
}

/// The lock's state, the free decode of its `z`.
fn locked_state(l: &Locked) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&decode::Latent::new(l.z)).ic
}

/// A target pair in the state's own classes: `L*` half of its `L_z`, `E*` nine tenths of its `E`.
fn targets(x: &Ic<f64>) -> (f64, f64) {
    let (lz, e) = lz_e(x);
    (0.5 * lz, 0.9 * e)
}

/// A step-4 landing under test: the encode of `x` into the plane, where `through` is the composed chart it goes through.
type Landed = fn(&Composed, &Ic<f64>) -> Option<(f64, f64)>;

fn landing(c: &Composed, x: &Ic<f64>) -> Option<(f64, f64)> {
    match c.encode(x, &tol()).landing {
        Landing::Chart { u, v } => Some((u, v)),
        Landing::Latent => None,
    }
}

/// A state the unheld decode puts at `(u, v)` is not on the hold, so the held plane does not place it on any pixel.
fn check_landing(held: &Composed, through: Landed) {
    let x = unheld(0.6, 0.4);
    assert_eq!(
        landing(&plane(None, None), &x)
            .map(|(u, v)| ((u - 0.6).abs() < 1e-9, (v - 0.4).abs() < 1e-9)),
        Some((true, true)),
        "the unheld plane does not place the state at its own pixel"
    );
    let got = through(held, &x);
    assert!(
        got.is_none(),
        "step 4 landed an off-hold state on the pixel {got:?}, which shows another system under the hold"
    );
}

#[test]
fn hold_encode_landing_reads_the_held_decode() {
    let x = unheld(0.6, 0.4);
    let (l, e) = targets(&x);
    for (e, l) in [(None, Some(l)), (Some(e), None), (Some(e), Some(l))] {
        check_landing(&plane(e, l), landing);
    }
}

/// Step 4 on the unheld decode: the encode through a plane without the holds.
#[cfg(feature = "controls")]
fn unheld_decode(_: &Composed, x: &Ic<f64>) -> Option<(f64, f64)> {
    landing(&plane(None, None), x)
}

negative_control!(
    hold_encode_landing_reads_the_held_decode,
    "step 4 on the unheld decode lands on a wrong pixel",
    expected = "landed an off-hold state on the pixel",
    {
        let x = unheld(0.6, 0.4);
        let (l, _) = targets(&x);
        check_landing(&plane(None, Some(l)), unheld_decode)
    }
);

/// The lock of `x` under the holds `(e, l)`, `L_z` and `E` read from it, against the targets and the gaps.
fn check_projects(c: &Composed, e: Option<f64>, l: Option<f64>) {
    let x = unheld(0.6, 0.4);
    let (locked, held) = c.lookup_held(&Exact::Physical(x), &tol()).expect("lookup");
    let (lz0, e0) = lz_e(&x);
    let notice = held.expect("lookup_held is not raised for a state off the hold");
    let y = locked_state(&locked);
    let (lz, en) = lz_e(&y);
    if let Some(l) = l {
        assert!((lz - l).abs() < 1e-9, "the lock's L_z {lz} is not L* {l}");
        assert!((notice.lz_gap - (lz0 - l)).abs() < 1e-9, "L_z gap");
    } else {
        assert_eq!(notice.lz_gap, 0.0);
    }
    if let Some(e) = e {
        assert!((en - e).abs() < 1e-9, "the lock's E {en} is not E* {e}");
        assert!((notice.e_gap - (e0 - e)).abs() < 1e-9, "E gap");
    } else {
        assert_eq!(notice.e_gap, 0.0);
    }
}

#[test]
fn hold_encode_projects_a_state_off_the_hold() {
    let (l, e) = targets(&unheld(0.6, 0.4));
    for (e, l) in [(None, Some(l)), (Some(e), None), (Some(e), Some(l))] {
        check_projects(&plane(e, l), e, l);
    }
}

negative_control!(
    hold_encode_projects_a_state_off_the_hold,
    "a chart with no holds, asked for the targets, projects nothing",
    expected = "lookup_held is not raised",
    {
        let (l, e) = targets(&unheld(0.6, 0.4));
        check_projects(&plane(None, None), Some(e), Some(l))
    }
);

/// A plane under holds, built from `(E*, L*)`.
type Make = fn(Option<f64>, Option<f64>) -> Composed;

/// The state a lookup is given, from the plane it is looked up in.
type Source = fn(&Composed) -> Ic<f64>;

fn check_on_the_hold(source: Source) {
    let x = unheld(0.6, 0.4);
    let (l, e) = targets(&x);
    for (e, l) in [(None, Some(l)), (Some(e), None), (Some(e), Some(l))] {
        let c = plane(e, l);
        let y = source(&c);
        let (locked, held) = c.lookup_held(&Exact::Physical(y), &tol()).expect("lookup");
        assert!(held.is_none(), "a held decode raised lookup_held");
        let back = locked_state(&locked);
        let (a, b) = (lz_e(&back), lz_e(&y));
        assert!(
            (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
            "the lock is not the state"
        );
    }
}

#[test]
fn hold_encode_state_on_the_hold() {
    check_on_the_hold(|c| c.decode(0.6, 0.4).ic);
}

negative_control!(
    hold_encode_state_on_the_hold,
    "the unheld decode given as the held one is off the hold and raises the notice",
    expected = "a held decode raised lookup_held",
    check_on_the_hold(|_| unheld(0.6, 0.4))
);

fn check_rejects(make: Make) {
    let x = unheld(0.6, 0.4);
    let (lz, e) = lz_e(&x);
    // The class rule: L* of the other sense is a different class, never placed.
    let r = make(None, Some(-lz))
        .lookup(&Exact::Physical(x), &tol())
        .expect_err("the projection changes the sense of rotation");
    assert_eq!(
        r,
        Rejected {
            layer: Layer::Hold,
            failed: Failed::HeldClass(Class::Rotation)
        }
    );
    // Both held with E* below the potential: no state on the hold, ENERGY_INFEASIBLE.
    let r = make(Some(-1.0e3), Some(0.5 * lz))
        .lookup(&Exact::Physical(x), &tol())
        .expect_err("E* < U");
    assert_eq!(r.layer, Layer::Hold);
    assert!(
        matches!(
            r.failed,
            Failed::Held {
                outcome: Outcome::Infeasible,
                no_state: true
            }
        ),
        "{r:?}"
    );
    // η_E on a rest start at E* above U: states exist, but x's momenta give no direction.
    let mut rest = x;
    rest.p = [[0.0; 2]; 3];
    let r = make(Some(0.9 * e.abs()), None)
        .lookup(&Exact::Physical(rest), &tol())
        .expect_err("the rest start");
    assert!(
        matches!(
            r.failed,
            Failed::Held {
                outcome: Outcome::Unscalable,
                no_state: false
            }
        ),
        "{r:?}"
    );
}

#[test]
fn hold_encode_rejects() {
    check_rejects(plane);
}

negative_control!(
    hold_encode_rejects,
    "a plane with no holds rejects nothing",
    expected = "the projection changes the sense of rotation",
    check_rejects(|_, _| plane(None, None))
);

fn check_ingest(make: Make) {
    let x = unheld(0.6, 0.4);
    let (l, e) = targets(&x);
    let c = make(Some(e), Some(l));
    let (ing, notice) = c.ingest_held::<Defaults>(&x, &tol()).expect("ingest");
    assert!(notice.is_some(), "an off-hold ingest raised no lookup_held");
    let (lz, en) = lz_e(&decode::decode::<Defaults, f64>(&decode::Latent::new(ing.encoded.z.z)).ic);
    assert!(
        (lz - l).abs() < 1e-9 && (en - e).abs() < 1e-9,
        "the ingest is not the projection"
    );
    // Off: the ingest is the unheld one, and there is no notice.
    let (a, none) = plane(None, None)
        .ingest_held::<Defaults>(&x, &tol())
        .unwrap();
    assert!(none.is_none());
    assert_eq!(a, plane(None, None).ingest::<Defaults>(&x));
}

#[test]
fn hold_encode_ingest() {
    check_ingest(plane);
}

negative_control!(
    hold_encode_ingest,
    "a plane with no holds ingests the entry as it is, raising no notice",
    expected = "an off-hold ingest raised no lookup_held",
    check_ingest(|_, _| plane(None, None))
);

/// The decode of `c` at the pixel `(0.6, 0.4)`: the energy it reaches at f64 and at f32, against `e_star`, and with no
/// hold the decode is the kernel's own, bit for bit.
fn check_hold_e_decode(make: Make, e_star: f64) {
    let on = make(Some(e_star), None);
    let (_, e64) = lz_e(&on.decode(0.6, 0.4).ic);
    assert!(
        (e64 - e_star).abs() < 1e-9,
        "the decode's E is {e64}, not E* {e_star}"
    );
    let d32 = on.decode32(0.6, 0.4).expect("the latent plane holds f32");
    let widen = |a: [[f32; 2]; 3]| a.map(|b| b.map(f64::from));
    let ic32 = Ic {
        m: d32.ic.m.map(f64::from),
        r: widen(d32.ic.r),
        p: widen(d32.ic.p),
    };
    let (_, e32) = lz_e(&ic32);
    assert!(
        (e32 - e_star).abs() < 1e-4,
        "the f32 decode's E is {e32}, not E* {e_star}"
    );
    // Off: the composed decode is the unheld kernel decode, bit for bit.
    let off = make(None, None).decode(0.6, 0.4);
    let direct = kernel::chart::Latent {
        z0: Z0,
        q1: {
            let mut q = [0.0; 8];
            q[2] = 1.0;
            q
        },
        q2: {
            let mut q = [0.0; 8];
            q[3] = 1.0;
            q
        },
    }
    .decode_at::<Defaults, f64>(0.6, 0.4);
    assert_eq!(
        format!("{:?}", off.ic),
        format!("{:?}", direct.ic),
        "with no hold the composed decode moved"
    );
}

#[test]
fn hold_encode_hold_e_decode() {
    let x = unheld(0.6, 0.4);
    let (_, e) = targets(&x);
    check_hold_e_decode(plane, e);
}

negative_control!(
    hold_encode_hold_e_decode,
    "a plane that ignores the spec's hold E decodes at its own energy",
    expected = "the decode's E is",
    {
        let (_, e) = targets(&unheld(0.6, 0.4));
        check_hold_e_decode(|_, _| plane(None, None), e)
    }
);
