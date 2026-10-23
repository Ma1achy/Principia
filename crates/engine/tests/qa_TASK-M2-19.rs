//! QA's tests for TASK-M2-19, written from the requirements it closes, not from `engine::lookup`'s code: the exact and
//! partial forms of lookup and the lock and slice they give (REQ-ENC-012; inverse_encode_contract Part 6, "Lookup",
//! and REQ-ENC-031's slice rule, "The partial specification's blocks and slice basis"), the validation ladder with its
//! clamp, its rejection naming the class a clamp would change, and layer 3's residuals (REQ-ENC-013; § "Chart-aware
//! validation", "Layer 1's space and bounds", "Layer 3's tolerance", "A qualitatively different IC"), exactly
//! coincident bodies clamped and never rejected (REQ-ENC-019; R-13, R-113), and the decode-sanity tolerance's claim
//! that decode's residuals sit under it at f64 and f32 over fuzzed `z` (REQ-ENC-027). Every quantity asserted is
//! computed here from the contract's own formulas (masses as proportions, Burrau's sides, `ν = sec θ − tan θ`,
//! `q = (p_ρ, p_λ)`, `L_z`, `E`, `‖Σmᵢrᵢ‖/R_x`). Each test registers its negative control (R-176).

use engine::lookup::{
    lookup, lookup_partial, residuals, sanity, Class, Exact, Failed, Layer, Locked, Partial,
    Rejected, Tolerances,
};
use kernel::chart::{Burrau, Constraint, Latent};
use kernel::decode::{self, Ic};
use kernel::encode::by_kind::Landing;
use kernel::generated::links::Defaults;
use ledger::constants::{DECODE_SANITY, EPS_PHYS};
use validation::negative_control;
use validation::roundtrip::{charts, phys};

type Outcome = Result<Locked, Rejected>;

/// `q_max`, settled (R-10; inverse_encode_contract Part 3), the default link parameter.
const Q_MAX: f64 = 2.0;
/// `ε_q`, the momentum clamp (Part 3 quotes `ε_z = 10⁻⁶` for `s`, and the momentum clamp is the same interval).
const EPS_Q: f64 = 1e-6;

/// The curve tolerance REQ-ENC-026 proposes, `k = 128` at f64; only the Burrau chart reads it.
fn tol() -> Tolerances {
    Tolerances::register(128.0 * f64::EPSILON)
}

fn exact<C: kernel::encode::by_kind::ByKind<Param = f64>>(chart: &C, entry: Exact) -> Outcome {
    lookup::<Defaults, C, _>(chart, &entry, phys, &tol())
}

fn partial<C: engine::lookup::Fill<Param = f64>>(
    chart: &C,
    p: Partial,
    nu: Option<f64>,
) -> Outcome {
    lookup_partial::<Defaults, C, _>(chart, &p, nu, phys, &tol())
}

fn latent() -> Latent<f64> {
    charts::latent::<f64>()
}

fn burrau() -> Burrau<f64> {
    charts::burrau_full::<f64>()
}

/// The free decode of `z`.
fn d(z: [f64; 8]) -> Ic<f64> {
    decode::decode::<Defaults, f64>(&decode::Latent::new(z)).ic
}

/// The lock decoded as a pixel decodes it: its chart point on the chart, else its latent `z`.
fn decoded<C: kernel::encode::by_kind::ByKind<Param = f64>>(chart: &C, lock: &Locked) -> Ic<f64> {
    match lock.landing {
        Landing::Chart { u, v } => chart.decode_at::<Defaults, f64>(u, v).ic,
        Landing::Latent => d(lock.z),
    }
}

/// `ε_phys`, the register's `k` times f64's epsilon (REQ-ENC-024).
fn eps_phys() -> f64 {
    EPS_PHYS.number() * f64::EPSILON
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// `L_z = Σ rᵢ × pᵢ`.
fn lz(x: &Ic<f64>) -> f64 {
    (0..3)
        .map(|i| x.r[i][0] * x.p[i][1] - x.r[i][1] * x.p[i][0])
        .sum()
}

/// `E = Σ|pᵢ|²/2mᵢ − Σ mᵢmⱼ/rᵢⱼ` (`G = 1`).
fn energy(x: &Ic<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| -x.m[i] * x.m[j] / dist(x.r[i], x.r[j]))
        .sum();
    k + u
}

/// The sums of the magnitudes of `E`'s and `L_z`'s terms, `K + |U|` and `Σ|rᵢ × pᵢ|`, each floored at 1.
fn magnitude(x: &Ic<f64>) -> (f64, f64) {
    let k: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| x.m[i] * x.m[j] / dist(x.r[i], x.r[j]))
        .sum();
    let l: f64 = (0..3)
        .map(|i| (x.r[i][0] * x.p[i][1]).abs() + (x.r[i][1] * x.p[i][0]).abs())
        .sum();
    ((k + u).max(1.0), l.max(1.0))
}

/// A small deterministic generator (SplitMix64), so the fuzzed points are fixed by their seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^ (x >> 31)
    }

    /// Uniform on `[lo, hi]`.
    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * ((self.next() >> 11) as f64 / (1u64 << 53) as f64)
    }
}

// ── REQ-ENC-012: the exact forms ───────────────────────────────────────────────────────────────────────────────────

/// The lock's decoded state is a Burrau system (chart_reference §4.1–4.2): the masses proportional to the sides
/// `(a, b, c)` of the given triangle, each body opposite the side its mass is proportional to, at rest, and its CoM at
/// the origin.
fn check_burrau_state(x: &Ic<f64>, sides: [f64; 3], what: &str) {
    let tol = eps_phys();
    let total: f64 = sides.iter().sum();
    let mut got = x.m;
    got.sort_by(f64::total_cmp);
    let mut want = sides.map(|s| s / total);
    want.sort_by(f64::total_cmp);
    for k in 0..3 {
        assert!(
            (got[k] - want[k]).abs() <= tol,
            "{what}: the masses {:?} are not the sides' proportions {want:?}",
            x.m
        );
    }
    // Body i sits opposite the side joining the other two, whose length is proportional to mᵢ.
    let opposite = [
        dist(x.r[1], x.r[2]),
        dist(x.r[0], x.r[2]),
        dist(x.r[0], x.r[1]),
    ];
    let ratio = opposite[0] / x.m[0];
    for (i, (side, m)) in opposite.iter().zip(x.m).enumerate().skip(1) {
        assert!(
            (side / m / ratio - 1.0).abs() <= tol,
            "{what}: body {i}'s opposite side over its mass is {}, body 0's {ratio}",
            side / m
        );
    }
    assert!(
        x.p.iter().flatten().all(|&p| p.abs() <= tol),
        "{what}: the lock is not at rest: {:?}",
        x.p
    );
}

/// Each `(m, n)` of chart_reference §4.1's table, Euclid's triple and that triple times `k` lock the same Burrau system
/// on the Burrau chart, no notice raised and no slice set.
fn check_pythagorean(table: &[((u32, u32), [u64; 3])]) {
    let chart = burrau();
    for &((m, n), t) in table {
        let sides = t.map(|s| s as f64);
        let pair = exact(&chart, Exact::Pythagorean { m, n }).expect("a primitive pair is refused");
        check_burrau_state(&decoded(&chart, &pair), sides, &format!("({m}, {n})"));
        assert_eq!(pair.slice, None, "({m}, {n}): an exact entry sets a slice");
        assert!(!pair.notices.lookup_clamped(), "({m}, {n}) is clamped");
        for k in [1, 3] {
            let triple = exact(&chart, Exact::Triple(t.map(|s| s * k)))
                .expect("a Pythagorean triple is refused");
            let (xp, xt) = (decoded(&chart, &pair), decoded(&chart, &triple));
            assert!(
                phys(&xp, &xt) <= eps_phys(),
                "({m}, {n}) and {k}·{t:?} lock different systems: {}",
                phys(&xp, &xt)
            );
        }
    }
}

#[test]
fn lookup_forms_qa_pythagorean_and_triple_lock_burrau() {
    check_pythagorean(&[
        ((2, 1), [3, 4, 5]),
        ((3, 2), [5, 12, 13]),
        ((4, 1), [15, 8, 17]),
        ((4, 3), [7, 24, 25]),
    ]);
}

negative_control!(
    lookup_forms_qa_pythagorean_and_triple_lock_burrau,
    "(3, 2)'s triple is (5, 12, 13); its lock checked against (3, 4, 5)'s sides must fail",
    expected = "are not the sides' proportions",
    check_pythagorean(&[((3, 2), [3, 4, 5])])
);

/// Each entry is refused before the ladder, naming what it fails: `(m, n)` not coprime with `m > n > 0` and `m − n`
/// odd (chart_reference §4.1); sides that are no Pythagorean triple or have a zero side; a value not finite; a mass not
/// positive (Part 6, "The physical form's masses").
fn check_refused(entries: &[(Exact, Failed)]) {
    for &(entry, failed) in entries {
        let got = exact(&latent(), entry);
        assert_eq!(
            got.err(),
            Some(Rejected {
                layer: Layer::Entry,
                failed
            }),
            "{entry:?} is not refused as {failed:?}"
        );
    }
}

fn some_state() -> Ic<f64> {
    Ic {
        m: [0.4, 0.35, 0.25],
        r: [[-0.6, 0.1], [0.7, 0.3], [0.1, -0.8]],
        p: [[0.1, 0.2], [-0.3, 0.05], [0.2, -0.25]],
    }
}

#[test]
fn lookup_forms_qa_entries_refused() {
    let with = |f: &dyn Fn(&mut Ic<f64>)| {
        let mut x = some_state();
        f(&mut x);
        Exact::Physical(x)
    };
    check_refused(&[
        (Exact::Pythagorean { m: 1, n: 2 }, Failed::NotPrimitive),
        (Exact::Pythagorean { m: 3, n: 1 }, Failed::NotPrimitive),
        (Exact::Pythagorean { m: 4, n: 2 }, Failed::NotPrimitive),
        (Exact::Pythagorean { m: 2, n: 2 }, Failed::NotPrimitive),
        (Exact::Pythagorean { m: 1, n: 0 }, Failed::NotPrimitive),
        (Exact::Pythagorean { m: 0, n: 0 }, Failed::NotPrimitive),
        (Exact::Triple([3, 4, 6]), Failed::NotATriple),
        (Exact::Triple([0, 4, 4]), Failed::NotATriple),
        (Exact::Triple([4, 0, 4]), Failed::NotATriple),
        (
            Exact::Raw([0.0, 0.0, f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0]),
            Failed::NonFinite,
        ),
        (
            Exact::Raw([f64::INFINITY, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            Failed::NonFinite,
        ),
        (with(&|x| x.r[2][1] = f64::INFINITY), Failed::NonFinite),
        (with(&|x| x.p[0][0] = f64::NAN), Failed::NonFinite),
        (with(&|x| x.m[1] = 0.0), Failed::MassNotPositive),
        (with(&|x| x.m[2] = -0.25), Failed::MassNotPositive),
    ]);
}

negative_control!(
    lookup_forms_qa_entries_refused,
    "(2, 1) is a primitive pair, so a check expecting it refused must fail",
    expected = "is not refused as",
    check_refused(&[(Exact::Pythagorean { m: 2, n: 1 }, Failed::NotPrimitive)])
);

/// An entered raw `z` inside every clamp locks at that `z` bit for bit, with no notice and no slice; one control past
/// its clamp is the only control moved, onto the clamp, every other kept bit for bit (§ "Layer 1's space and bounds").
fn check_raw(past: f64) {
    let mut rng = Rng(0x5EED_0019);
    for _ in 0..64 {
        let z: [f64; 8] = std::array::from_fn(|_| rng.uniform(-4.0, 4.0));
        let lock = exact(&latent(), Exact::Raw(z)).expect("an interior raw z is refused");
        assert_eq!(
            lock.z.map(f64::to_bits),
            z.map(f64::to_bits),
            "an interior raw z is not locked bit for bit"
        );
        assert!(
            !lock.notices.lookup_clamped(),
            "an interior raw z is clamped"
        );
        assert_eq!(lock.slice, None, "an exact entry sets a slice");
        // z_q1 past its clamp: s = σ(z) above 1 − ε_q.
        let mut zc = z;
        zc[3] = past;
        let lock = exact(&latent(), Exact::Raw(zc)).expect("a clamped raw z is refused");
        assert_eq!(
            lock.notices.moved.controls,
            1 << 3,
            "z = {zc:?}: the moved controls are not z_q1 alone"
        );
        for k in (0..8).filter(|&k| k != 3) {
            assert_eq!(lock.z[k].to_bits(), zc[k].to_bits(), "control {k} moved");
        }
        let s = 1.0 / (1.0 + (-lock.z[3]).exp());
        assert!(
            ((1.0 - s) - EPS_Q).abs() <= 1e-3 * EPS_Q,
            "z_q1 is not on its clamp: 1 − s = {:e}",
            1.0 - s
        );
    }
}

#[test]
fn lookup_forms_qa_raw_kept_and_clamped() {
    // σ(30) = 1 − 9.4e-14, past the clamp's 1 − 10⁻⁶.
    check_raw(30.0);
}

negative_control!(
    lookup_forms_qa_raw_kept_and_clamped,
    "z_q1 = 3 is inside its clamp, so nothing moves and the check that z_q1 moved must fail",
    expected = "the moved controls are not z_q1 alone",
    check_raw(3.0)
);

/// `g·x` under a translation, a boost, a rotation, a rescale and the full-state mirror (inverse_encode_contract
/// Part 1's gauge group).
fn gauge(
    x: &Ic<f64>,
    shift: [f64; 2],
    boost: [f64; 2],
    angle: f64,
    scale: f64,
    mirror: bool,
) -> Ic<f64> {
    let (s, c) = angle.sin_cos();
    let f = |v: [f64; 2]| {
        let w = [c * v[0] - s * v[1], s * v[0] + c * v[1]];
        if mirror {
            [w[0], -w[1]]
        } else {
            w
        }
    };
    Ic {
        m: x.m.map(|m| 3.0 * m),
        r: x.r.map(|v| {
            let w = f(v);
            [scale * w[0] + shift[0], scale * w[1] + shift[1]]
        }),
        p: std::array::from_fn(|i| {
            let w = f(x.p[i]);
            [
                w[0] / scale.sqrt() + x.m[i] * boost[0],
                w[1] / scale.sqrt() + x.m[i] * boost[1],
            ]
        }),
    }
}

/// A physical entry locks `C(x)`'s `z`: every member of `x`'s gauge orbit locks the same state (T1), its masses the
/// entry's proportions, its CoM at the origin and `Σpᵢ = 0` though the entry's were not, `I = 1`, and `E` and `L_z` the
/// entry's carried by Part 2's rescale; the full-state mirror raises `lookup_mirrored` on one side only.
fn check_physical(x: &Ic<f64>, orbit: &[(Ic<f64>, bool)]) {
    let chart = latent();
    let base = exact(&chart, Exact::Physical(*x)).expect("a physical entry is refused");
    let xb = decoded(&chart, &base);
    let total: f64 = x.m.iter().sum();
    for i in 0..3 {
        assert!(
            (xb.m[i] - x.m[i] / total).abs() <= eps_phys(),
            "the lock's masses {:?} are not the entry's proportions",
            xb.m
        );
    }
    let (com, mom) = residuals(&xb);
    assert!(
        com <= tol().sanity && mom <= tol().sanity,
        "the lock is off the section: {com:e}, {mom:e}"
    );
    let inertia: f64 = (0..3)
        .map(|i| xb.m[i] * (xb.r[i][0].powi(2) + xb.r[i][1].powi(2)))
        .sum();
    assert!(
        (inertia - 1.0).abs() <= eps_phys(),
        "the lock's I = {inertia}"
    );
    for (g, mirrored) in orbit {
        let lock = exact(&chart, Exact::Physical(*g)).expect("a gauge image is refused");
        let xg = decoded(&chart, &lock);
        assert!(
            phys(&xb, &xg) <= eps_phys(),
            "a gauge image locks a different state: ‖Δ‖_phys = {:e}",
            phys(&xb, &xg)
        );
        assert_eq!(
            lock.notices.mirrored != base.notices.mirrored,
            *mirrored,
            "lookup_mirrored does not follow the mirror"
        );
        // Part 2: E and L_z carried by the rescale, `E·√I_in` and `L_z/I_in^{1/4}`, in the entry's own CoM frame.
        let gm: f64 = g.m.iter().sum();
        let prop = g.m.map(|m| m / gm);
        let com: [f64; 2] = std::array::from_fn(|k| (0..3).map(|i| prop[i] * g.r[i][k]).sum());
        let ptot: [f64; 2] = std::array::from_fn(|k| (0..3).map(|i| g.p[i][k]).sum());
        let frame = Ic {
            m: prop,
            r: g.r.map(|v| [v[0] - com[0], v[1] - com[1]]),
            p: std::array::from_fn(|i| {
                [g.p[i][0] - prop[i] * ptot[0], g.p[i][1] - prop[i] * ptot[1]]
            }),
        };
        let i_in: f64 = (0..3)
            .map(|i| frame.m[i] * (frame.r[i][0].powi(2) + frame.r[i][1].powi(2)))
            .sum();
        let (e_want, lz_want) = (energy(&frame) * i_in.sqrt(), lz(&frame) / i_in.powf(0.25));
        // Each to ε_phys relative to the magnitude of its terms, the scale of its rounding.
        assert!(
            (energy(&xg) - e_want).abs() <= eps_phys() * magnitude(&xg).0,
            "the lock's E = {} is not the entry's rescaled {e_want}",
            energy(&xg)
        );
        let lz_lock = if lock.notices.mirrored {
            -lz(&xg)
        } else {
            lz(&xg)
        };
        assert!(
            (lz_lock - lz_want).abs() <= eps_phys() * magnitude(&xg).1,
            "the lock's L_z = {lz_lock} (mirror undone) is not the entry's rescaled {lz_want}"
        );
    }
}

fn orbit(x: &Ic<f64>) -> Vec<(Ic<f64>, bool)> {
    vec![
        (gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, 1.0, false), false),
        (gauge(x, [3.5, -1.25], [0.0, 0.0], 0.0, 1.0, false), false),
        (gauge(x, [0.0, 0.0], [0.7, -0.4], 0.0, 1.0, false), false),
        (gauge(x, [0.0, 0.0], [0.0, 0.0], 2.1, 1.0, false), false),
        (gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, 4.0, false), false),
        (gauge(x, [-2.0, 0.5], [0.3, 0.9], -0.8, 0.25, false), false),
        (gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, 1.0, true), true),
        (gauge(x, [1.0, 2.0], [-0.2, 0.1], 0.4, 2.5, true), true),
    ]
}

#[test]
fn lookup_forms_qa_physical_locks_the_canonical_state() {
    let x = some_state();
    check_physical(&x, &orbit(&x));
}

negative_control!(
    lookup_forms_qa_physical_locks_the_canonical_state,
    "a configuration-only mirror is no gauge image (Part 1's classic bug), so its lock must differ",
    expected = "a gauge image locks a different state",
    {
        let x = some_state();
        let mut bad = gauge(&x, [0.0, 0.0], [0.0, 0.0], 0.0, 1.0, true);
        bad.p = x.p;
        check_physical(&x, &[(bad, true)])
    }
);

// ── REQ-ENC-012, REQ-ENC-031: the partial form and its slice ──────────────────────────────────────────────────────

fn unit(k: usize) -> [f64; 8] {
    std::array::from_fn(|n| if n == k { 1.0 } else { 0.0 })
}

/// The slice REQ-ENC-031's rule gives (Part 6): the first two unspecified controls in the order `z_α, z_β`, then
/// `z_μ1, z_μ2`, then `z_q0 … z_q3`, in the block layout `z_α, z_β | z_q0 … z_q3 | z_μ1, z_μ2`; none when every block
/// is given.
fn rule(masses: bool, positions: bool, momenta: bool, order: [usize; 8]) -> Option<(usize, usize)> {
    let free = |k: usize| match k {
        0 | 1 => !positions,
        2..=5 => !momenta,
        _ => !masses,
    };
    let mut open = order.into_iter().filter(|&k| free(k));
    Some((open.next()?, open.next()?))
}

const ORDER: [usize; 8] = [0, 1, 6, 7, 2, 3, 4, 5];

/// On the latent chart, each combination of given blocks: the blocks not given take Part 6's defaults (masses equal,
/// latent configuration controls 0, momenta at rest), the given masses are read as proportions, and the slice is
/// centred on the lock and spans the two unit directions `order` picks.
fn check_partial(order: [usize; 8]) {
    let chart = latent();
    let given_m = [1.0, 2.0, 3.0];
    let x = some_state();
    for bits in 0..8u8 {
        let (gm, gr, gp) = (bits & 1 != 0, bits & 2 != 0, bits & 4 != 0);
        let p = Partial {
            masses: gm.then_some(given_m),
            positions: gr.then_some(x.r),
            momenta: gp.then_some(x.p),
        };
        let lock = partial(&chart, p, None).expect("a partial entry is refused");
        let xl = decoded(&chart, &lock);
        let want_m = if gm {
            given_m.map(|m| m / 6.0)
        } else {
            [1.0 / 3.0; 3]
        };
        for i in 0..3 {
            assert!(
                (xl.m[i] - want_m[i]).abs() <= eps_phys(),
                "{p:?}: the lock's masses {:?} are not {want_m:?}",
                xl.m
            );
        }
        if !gp {
            assert!(
                xl.p.iter().flatten().all(|v| v.abs() <= eps_phys()),
                "{p:?}: momenta not given, but the lock is not at rest: {:?}",
                xl.p
            );
        }
        if !gr {
            assert!(
                lock.z[0].abs() <= eps_phys() && lock.z[1].abs() <= eps_phys(),
                "{p:?}: positions not given, but z_α, z_β = {:?}",
                &lock.z[..2]
            );
        }
        match (rule(gm, gr, gp, order), lock.slice) {
            (None, None) => {}
            (Some((i, j)), Some(s)) => {
                assert_eq!(s.z0, lock.z, "{p:?}: the slice is not centred on the lock");
                assert_eq!(
                    (s.q1, s.q2),
                    (unit(i), unit(j)),
                    "{p:?}: the slice spans the wrong controls"
                );
            }
            (want, got) => {
                panic!("{p:?}: the slice spans the wrong controls: want {want:?}, got {got:?}")
            }
        }
    }
}

#[test]
fn lookup_forms_qa_partial_defaults_and_slice() {
    check_partial(ORDER);
}

negative_control!(
    lookup_forms_qa_partial_defaults_and_slice,
    "the momenta before the masses is not REQ-ENC-031's order, so a slice checked against it must fail",
    expected = "the slice spans the wrong controls",
    check_partial([0, 1, 2, 3, 4, 5, 6, 7])
);

/// On a Burrau chart, a partial entry with no block given is the Burrau system at the locked `ν`, or at `ν = 1/2`
/// with none, at rest: `ν = sec θ − tan θ` (Part 5, kind 4), so `θ = π/2 − 2·atan ν` and the sides are
/// `(sin θ, cos θ, 1)`; the slice spans `z_α, z_β`.
fn check_partial_burrau(nus: &[(Option<f64>, f64)]) {
    let chart = burrau();
    for &(locked, nu) in nus {
        let lock =
            partial(&chart, Partial::default(), locked).expect("a Burrau partial entry is refused");
        let theta = std::f64::consts::FRAC_PI_2 - 2.0 * nu.atan();
        check_burrau_state(
            &decoded(&chart, &lock),
            [theta.sin(), theta.cos(), 1.0],
            &format!("ν = {nu}"),
        );
        let s = lock.slice.expect("a partial entry sets no slice");
        assert_eq!(
            (s.q1, s.q2),
            (unit(0), unit(1)),
            "ν = {nu}: the slice is not z_α, z_β"
        );
    }
}

#[test]
fn lookup_forms_qa_partial_burrau() {
    check_partial_burrau(&[
        (None, 0.5),
        (Some(0.5), 0.5),
        (Some(0.2), 0.2),
        (Some(0.7), 0.7),
    ]);
}

negative_control!(
    lookup_forms_qa_partial_burrau,
    "a locked ν = 0.2 is not ν = 1/2's system, so a check expecting the default must fail",
    expected = "ν = 0.5",
    check_partial_burrau(&[(Some(0.2), 0.5)])
);

// ── REQ-ENC-013: the ladder ────────────────────────────────────────────────────────────────────────────────────────

/// The free Jacobi momenta `q = (p_ρ, p_λ)` (Part 3): `p_λ = p₂`, `p_ρ = p₁ + (m₁/M₀₁)p_λ`.
fn jacobi_q(x: &Ic<f64>) -> [f64; 4] {
    let w = x.m[1] / (x.m[0] + x.m[1]);
    let pl = x.p[2];
    [x.p[1][0] + w * pl[0], x.p[1][1] + w * pl[1], pl[0], pl[1]]
}

/// A canonical state, `D(z)`, with its momenta scaled by `c`: layer 1 clamps exactly the momentum controls whose
/// `|q_k|` now exceeds `q_max`, the lock's `q` is inside `[−q_max, q_max]`, the classes of `L_z` and `E` are kept, so the
/// clamp stands, `lookup_clamped` (§ "Layer 1's space and bounds").
fn check_layer1_clamp(c: f64) {
    let z = [0.4, -0.3, 1.0, -0.05, 0.3, 0.0, 0.2, -0.1];
    let mut x = d(z);
    x.p = x.p.map(|p| [c * p[0], c * p[1]]);
    let q = jacobi_q(&x);
    let want: u32 = (0..4)
        .filter(|&k| q[k].abs() > Q_MAX)
        .map(|k| 1 << (k + 2))
        .sum();
    let lock = exact(&latent(), Exact::Physical(x))
        .expect("a momentum clamp that keeps every class is refused");
    let xl = decoded(&latent(), &lock);
    assert!(
        lz(&xl).signum() == lz(&x).signum() && energy(&xl).signum() == energy(&x).signum(),
        "fixture: the clamp changes a class"
    );
    assert!(
        lock.notices.lookup_clamped(),
        "q = {q:?} beyond q_max is not clamped"
    );
    assert_eq!(
        lock.notices.moved.controls, want,
        "q = {q:?}: the moved controls are not those beyond q_max"
    );
    for (k, v) in jacobi_q(&xl).iter().enumerate() {
        assert!(v.abs() <= Q_MAX, "the lock's q_{k} = {v} is beyond q_max");
    }
}

#[test]
fn lookup_validation_ladder_qa_layer1_clamp() {
    check_layer1_clamp(10.0);
}

negative_control!(
    lookup_validation_ladder_qa_layer1_clamp,
    "unscaled, every q is inside q_max, so nothing is clamped and the check must fail",
    expected = "beyond q_max is not clamped",
    check_layer1_clamp(1.0)
);

/// Bodies 0 and 1 close (`z_α = 12`), the inner pair moving apart along `ρ` at `|p_ρ| = a`, body 2 at rest, so
/// `L_z = 0`: with `E > 0`, the clamp of `p_ρ` to `q_max` makes `E < 0`, a qualitatively different IC, and lookup
/// rejects at layer 1 naming the energy class (§ "A qualitatively different IC"); with `E < 0` the clamp keeps it.
fn tight_pair(e_over_u: f64) -> Ic<f64> {
    let mut x = d([12.0, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    assert_eq!(x.r[0][1], x.r[1][1], "fixture: ρ is not on the x axis");
    x.p = [[0.0; 2]; 3];
    let u = energy(&x);
    // K = a²/2m₀ + a²/2m₁ with p₁ = −p₀ = (a, 0).
    let a = (-u * e_over_u / (0.5 / x.m[0] + 0.5 / x.m[1])).sqrt();
    x.p[0] = [-a, 0.0];
    x.p[1] = [a, 0.0];
    x
}

fn check_layer1_reject(e_over_u: f64) {
    let x = tight_pair(e_over_u);
    assert!(
        energy(&x) > 0.0 && lz(&x) == 0.0,
        "fixture: E = {}, L_z = {}",
        energy(&x),
        lz(&x)
    );
    assert!(jacobi_q(&x)[0] > Q_MAX, "fixture: p_ρ is inside q_max");
    match exact(&latent(), Exact::Physical(x)) {
        Err(Rejected {
            layer: Layer::Hypercube,
            failed:
                Failed::Clamp {
                    controls,
                    class: Class::Energy,
                },
        }) => assert_eq!(controls, 1 << 2, "the rejection names the wrong controls"),
        other => panic!("a clamp turning E > 0 into E < 0 is not rejected at layer 1: {other:?}"),
    }
}

#[test]
fn lookup_validation_ladder_qa_layer1_reject() {
    check_layer1_reject(1.01);
    // The same pair bound, E < 0: the clamp keeps the class and stands.
    let x = tight_pair(0.5);
    let lock = exact(&latent(), Exact::Physical(x)).expect("a clamp keeping E < 0 is refused");
    assert_eq!(
        lock.notices.moved.controls,
        1 << 2,
        "the bound pair's p_ρ is not the control clamped"
    );
}

negative_control!(
    lookup_validation_ladder_qa_layer1_reject,
    "at E < 0 the clamp keeps the class, so the check expecting a rejection must fail",
    expected = "fixture: E =",
    check_layer1_reject(0.5)
);

/// Layer 3's residuals are `‖Σmᵢrᵢ‖/R_x` and `‖Σpᵢ‖/P_x`, `R_x`, `P_x` the largest `‖rⱼ‖`, `‖pⱼ‖` floored at 1
/// (§ "Layer 3's tolerance"); a decoded state passes at the register's tolerance, and a CoM or `Σpᵢ` off by twice the
/// tolerance relative to its extent, or a mass not positive, fails, naming that constraint.
fn check_sanity(factor: f64) {
    let t = tol().sanity;
    let x = d([0.3, -0.7, 0.5, -0.2, 0.1, 0.9, -0.4, 0.6]);
    assert_eq!(sanity(&x, t), None, "a decoded state fails decode sanity");
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|w| w[0].hypot(w[1])).fold(1.0, f64::max);
    let shift = |x: &Ic<f64>, by: f64| Ic {
        r: x.r.map(|v| [v[0] + by, v[1]]),
        ..*x
    };
    let push = |x: &Ic<f64>, by: f64| Ic {
        p: [[x.p[0][0] + by, x.p[0][1]], x.p[1], x.p[2]],
        ..*x
    };
    // Inside the unit extent the residual is absolute; at extent 1000 it is relative to it.
    let big = Ic {
        r: x.r.map(|v| [1000.0 * v[0], 1000.0 * v[1]]),
        p: x.p.map(|v| [1000.0 * v[0], 1000.0 * v[1]]),
        ..x
    };
    let (shifted, pushed) = (shift(&big, 0.5), push(&big, 0.5));
    let (com, _) = residuals(&shifted);
    let r_x = extent(&shifted.r);
    assert!(r_x > 100.0, "fixture: R_x = {r_x}");
    assert!(
        (com * r_x - 0.5).abs() <= 1e-9,
        "the CoM residual is {com}, not 0.5/R_x = {}",
        0.5 / r_x
    );
    let (_, mom) = residuals(&pushed);
    let p_x = extent(&pushed.p);
    assert!(p_x > 100.0, "fixture: P_x = {p_x}");
    assert!(
        (mom * p_x - 0.5).abs() <= 1e-9,
        "the Σp residual is {mom}, not 0.5/P_x = {}",
        0.5 / p_x
    );
    // Offsets in units of the tolerance times the state's own extent.
    let (rx, px) = (extent(&x.r), extent(&x.p));
    assert_eq!(
        sanity(&shift(&x, factor * t * rx), t),
        Some(Constraint::ComAtOrigin),
        "a CoM {factor}× the tolerance off passes"
    );
    assert_eq!(
        sanity(&push(&x, factor * t * px), t),
        Some(Constraint::MomentumZero),
        "a Σp {factor}× the tolerance off passes"
    );
    assert_eq!(
        sanity(&shift(&x, t * rx / 4.0), t),
        None,
        "a CoM a quarter of the tolerance off fails"
    );
    let mut light = x;
    light.m[2] = 0.0;
    assert!(sanity(&light, t).is_some(), "a zero decoded mass passes");
    let mut nan = x;
    nan.r[1][0] = f64::NAN;
    assert!(sanity(&nan, t).is_some(), "a NaN position passes");
}

#[test]
fn lookup_validation_ladder_qa_sanity() {
    check_sanity(2.0);
}

negative_control!(
    lookup_validation_ladder_qa_sanity,
    "a CoM half the tolerance off is within it, so the check expecting it to fail must fail",
    expected = "the tolerance off passes",
    check_sanity(0.5)
);

/// Bodies within `r_coll` (10⁻³ by default) but apart pass lookup unrejected (§ "Chart-aware validation", R-13).
fn check_within(seps: &[f64]) {
    for &sep in seps {
        for (i, j) in [(0, 1), (1, 2), (0, 2)] {
            let mut x = some_state();
            x.r[j] = [x.r[i][0] + sep, x.r[i][1] + 0.5 * sep];
            let got = exact(&latent(), Exact::Physical(x));
            assert!(
                got.is_ok(),
                "bodies {i} and {j} {sep:e} apart are rejected: {got:?}"
            );
        }
    }
}

#[test]
fn lookup_validation_ladder_qa_within_r_coll() {
    check_within(&[1e-3, 1e-6, 1e-9, 1e-12]);
}

negative_control!(
    lookup_validation_ladder_qa_within_r_coll,
    "a NaN separation is no entry, so it is refused and the check must fail",
    expected = "are rejected",
    check_within(&[f64::NAN])
);

// ── REQ-ENC-019: exactly coincident bodies ─────────────────────────────────────────────────────────────────────────

/// Each pair exactly coincident, and all three, at rest and moving, and moving with `L_z = 0` on a line: lookup
/// locks, with `lookup_clamped`, and never rejects (R-13: "no coincident-bodies rejection branch").
fn coincident_states() -> Vec<(String, Ic<f64>)> {
    let mut out = Vec::new();
    for (i, j) in [(0, 1), (1, 2), (0, 2)] {
        let mut x = some_state();
        x.r[j] = x.r[i];
        out.push((format!("{i}{j} moving"), x));
        let mut rest = x;
        rest.p = [[0.0; 2]; 3];
        out.push((format!("{i}{j} at rest"), rest));
        // Collinear on the x axis, moving along it: L_z = 0 exactly.
        let mut line = Ic {
            m: [0.4, 0.35, 0.25],
            r: [[-0.6, 0.0], [0.7, 0.0], [0.1, 0.0]],
            p: [[0.1, 0.0], [-0.3, 0.0], [0.2, 0.0]],
        };
        line.r[j] = line.r[i];
        out.push((format!("{i}{j} on a line, L_z = 0"), line));
    }
    let mut all = some_state();
    all.r = [[0.2, -0.1]; 3];
    out.push(("all three".into(), all));
    out
}

fn check_coincident(states: &[(String, Ic<f64>)]) {
    let mut failures = Vec::new();
    for (what, x) in states {
        match exact(&latent(), Exact::Physical(*x)) {
            Ok(lock) if lock.notices.lookup_clamped() => {}
            Ok(_) => failures.push(format!(
                "{what}: coincident bodies lock without lookup_clamped"
            )),
            Err(e) => failures.push(format!("{what}: coincident bodies are rejected: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn lookup_coincident_qa_clamped_never_rejected() {
    check_coincident(&coincident_states());
}

negative_control!(
    lookup_coincident_qa_clamped_never_rejected,
    "bodies apart raise no lookup_clamped, so the coincident check must fail on them",
    expected = "lock without lookup_clamped",
    check_coincident(&[("apart".into(), some_state())])
);

// ── REQ-ENC-027: decode's residuals under the proposed tolerance ──────────────────────────────────────────────────

/// The largest CoM and `Σpᵢ` residual, in units of the precision's epsilon, of decode over `n` fuzzed `z` on
/// `[−16, 16]⁸` and its corners, at f64 and f32, each residual computed at f64 from the decoded values.
fn worst_residuals(n: usize, shift: f64) -> (f64, f64) {
    let mut rng = Rng(0xDEC0_DE19);
    let mut zs: Vec<[f64; 8]> = (0..n)
        .map(|_| std::array::from_fn(|_| rng.uniform(-16.0, 16.0)))
        .collect();
    zs.push([0.0; 8]);
    zs.push([16.0; 8]);
    zs.push([-16.0; 8]);
    zs.push(std::array::from_fn(
        |k| if k % 2 == 0 { 16.0 } else { -16.0 },
    ));
    let res = |x: &Ic<f64>| {
        let x = Ic {
            r: x.r.map(|v| [v[0] + shift, v[1]]),
            ..*x
        };
        let (c, p) = residuals(&x);
        c.max(p)
    };
    let (mut w64, mut w32) = (0.0f64, 0.0f64);
    for z in zs {
        w64 = w64.max(res(&d(z)) / f64::EPSILON);
        let x32 = decode::decode::<Defaults, f32>(&decode::Latent::new(z.map(|v| v as f32))).ic;
        let x = Ic {
            m: x32.m.map(f64::from),
            r: x32.r.map(|v| v.map(f64::from)),
            p: x32.p.map(|v| v.map(f64::from)),
        };
        assert!(
            x.m.iter().all(|&m| m > 0.0),
            "z = {z:?}: a decoded f32 mass is not positive"
        );
        w32 = w32.max(res(&x) / f64::from(f32::EPSILON));
    }
    (w64, w32)
}

fn check_decode_residuals(shift: f64) {
    let k = DECODE_SANITY.number();
    let (w64, w32) = worst_residuals(20_000, shift);
    assert!(
        w64 <= k,
        "decode's worst f64 residual is {w64} ε, above decode_sanity's {k}"
    );
    assert!(
        w32 <= k,
        "decode's worst f32 residual is {w32} ε, above decode_sanity's {k}"
    );
}

#[test]
fn lookup_validation_ladder_qa_decode_sanity_covers_decode() {
    check_decode_residuals(0.0);
}

negative_control!(
    lookup_validation_ladder_qa_decode_sanity_covers_decode,
    "a CoM moved 10⁻⁶ off the origin is far above k·ε at f64, so the check must fail",
    expected = "above decode_sanity's",
    check_decode_residuals(1e-6)
);
