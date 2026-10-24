//! QA's tests for TASK-M2-19, written from the requirements it closes, not from `engine::lookup`'s code: the exact and
//! partial forms of lookup and the lock and slice they give (REQ-ENC-012; inverse_encode_contract Part 6, "Lookup",
//! and REQ-ENC-031's slice rule, "The partial specification's blocks and slice basis"), the validation ladder with its
//! clamp, its rejection naming the class a clamp would change, and layer 3's residuals (REQ-ENC-013; § "Chart-aware
//! validation", "Layer 1's space and bounds", "Layer 3's tolerance", "A qualitatively different IC"), exactly
//! coincident bodies clamped and never rejected (REQ-ENC-019; R-13, R-113), and the decode-sanity tolerance's claim
//! that decode's residuals sit under it at f64 and f32 over fuzzed `z` (REQ-ENC-027). Every quantity asserted is
//! computed here from the contract's own formulas (the masses carried to `M = 1` with the momenta by `M^{3/2}`,
//! Burrau's sides, `ν = sec θ − tan θ`, `q = (p_ρ, p_λ)`, `L_z`, `E`, `‖Σmᵢrᵢ‖/R_x`). Each test registers its negative
//! control (R-176). The mass unit (Part 6, "The physical form's masses", physics review 5630998530 of PR #284, P1) and
//! layer 1's zero class (§ "The zero class in floating point", REQ-ENC-036, and its band `k`, REQ-ENC-037) are tested
//! at the end.

use engine::lookup::{
    after, changed, lookup, lookup_partial, moves, residuals, sanity, Class, Classes, Exact,
    Extents, Failed, Layer, Locked, Moves, Partial, Rejected, Tolerances,
};
use kernel::chart::{Burrau, Constraint, Latent};
use kernel::decode::{self, Ic};
use kernel::encode::by_kind::Landing;
use kernel::encode::encode;
use kernel::generated::links::Defaults;
use ledger::constants::{DECODE_SANITY, EPS_PHYS, ZERO_CLASS};
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
/// Part 1's gauge group), in the mass unit `1/mass`: the masses times `mass` and, with `G = 1` and the length unit
/// kept, the velocities times `√mass`, so the momenta times `mass^{3/2}` (Part 6, "The physical form's masses", the
/// similarity T3 preserves the physics under), so every image is the same system.
fn gauge(
    x: &Ic<f64>,
    shift: [f64; 2],
    boost: [f64; 2],
    angle: f64,
    (scale, mass): (f64, f64),
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
    let carry = mass * mass.sqrt();
    Ic {
        m: x.m.map(|m| mass * m),
        r: x.r.map(|v| {
            let w = f(v);
            [scale * w[0] + shift[0], scale * w[1] + shift[1]]
        }),
        p: std::array::from_fn(|i| {
            let w = f(x.p[i]);
            [
                carry * w[0] / scale.sqrt() + mass * x.m[i] * boost[0],
                carry * w[1] / scale.sqrt() + mass * x.m[i] * boost[1],
            ]
        }),
    }
}

/// A physical entry locks `C(x)`'s `z`: every member of `x`'s gauge orbit, in any mass unit, locks the same state
/// (T1, T3), its masses the entry's proportions, its CoM at the origin and `Σpᵢ = 0` though the entry's were not,
/// `I = 1`, `M = Σmᵢ` recorded, and `E` and `L_z` the entry's carried to `M = 1` (`E/M²`, `L_z/M^{3/2}`; Part 6) and
/// then by Part 2's rescale; the full-state mirror raises `lookup_mirrored` on one side only.
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
        // M = Σmᵢ is recorded with the lock (Part 6).
        let gm: f64 = g.m.iter().sum();
        assert!(
            (lock.notices.mass - gm).abs() <= 4.0 * f64::EPSILON * gm,
            "the lock records M = {}, not the entry's {gm}",
            lock.notices.mass
        );
        // E and L_z of the entry in its own units and its own CoM frame, carried to M = 1 (Part 6: `E/M²`,
        // `L_z/M^{3/2}`, the positions kept, so `I_in` is read with the masses `mᵢ/M`), then by Part 2's rescale,
        // `E·√I_in` and `L_z/I_in^{1/4}`.
        let prop = g.m.map(|m| m / gm);
        let com: [f64; 2] = std::array::from_fn(|k| (0..3).map(|i| prop[i] * g.r[i][k]).sum());
        let ptot: [f64; 2] = std::array::from_fn(|k| (0..3).map(|i| g.p[i][k]).sum());
        let frame = Ic {
            m: g.m,
            r: g.r.map(|v| [v[0] - com[0], v[1] - com[1]]),
            p: std::array::from_fn(|i| {
                [g.p[i][0] - prop[i] * ptot[0], g.p[i][1] - prop[i] * ptot[1]]
            }),
        };
        let i_in: f64 = (0..3)
            .map(|i| prop[i] * (frame.r[i][0].powi(2) + frame.r[i][1].powi(2)))
            .sum();
        let (e_want, lz_want) = (
            energy(&frame) / (gm * gm) * i_in.sqrt(),
            lz(&frame) / (gm * gm.sqrt()) / i_in.powf(0.25),
        );
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
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 1.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 3.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 0.01), false),
            false,
        ),
        (
            gauge(x, [3.5, -1.25], [0.0, 0.0], 0.0, (1.0, 3.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.7, -0.4], 0.0, (1.0, 3.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 2.1, (1.0, 3.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, (4.0, 3.0), false),
            false,
        ),
        (
            gauge(x, [-2.0, 0.5], [0.3, 0.9], -0.8, (0.25, 12.0), false),
            false,
        ),
        (
            gauge(x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 3.0), true),
            true,
        ),
        (
            gauge(x, [1.0, 2.0], [-0.2, 0.1], 0.4, (2.5, 1000.0), true),
            true,
        ),
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
        let mut bad = gauge(&x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 3.0), true);
        bad.p = x.p;
        check_physical(&x, &[(bad, true)])
    }
);

mod momenta_kept {
    use super::*;
    negative_control!(
        lookup_forms_qa_physical_locks_the_canonical_state,
        "the masses times 3 with the momenta kept, the stale reading, is another system (velocities times 3), so its \
         lock must differ",
        expected = "a gauge image locks a different state",
        {
            let x = some_state();
            let mut bad = gauge(&x, [0.0, 0.0], [0.0, 0.0], 0.0, (1.0, 3.0), false);
            bad.p = x.p;
            check_physical(&x, &[(bad, false)])
        }
    );
}

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

// ── REQ-ENC-012: the mass unit (Part 6, "The physical form's masses"; physics review 5630998530 of PR #284, P1) ────

/// `K/|U|` of `x` (`G = 1`), each from its own definition.
fn virial(x: &Ic<f64>) -> f64 {
    let k: f64 = (0..3)
        .map(|i| (x.p[i][0].powi(2) + x.p[i][1].powi(2)) / (2.0 * x.m[i]))
        .sum();
    k / (k - energy(x))
}

/// Lagrange's equilateral relative equilibrium with the masses `m` (any masses, `G = 1`), side `a`, turning rigidly
/// about its CoM at `ω² = M/a³`, `M = Σmᵢ`: a circular central-configuration orbit, so `K/|U| = ½` and `E < 0`.
fn lagrange(m: [f64; 3], a: f64) -> Ic<f64> {
    let total: f64 = m.iter().sum();
    let w = (total / a.powi(3)).sqrt();
    let h = a * 3.0_f64.sqrt() / 2.0;
    let corners = [[0.0, 0.0], [a, 0.0], [a / 2.0, h]];
    let com: [f64; 2] =
        std::array::from_fn(|k| (0..3).map(|i| m[i] * corners[i][k]).sum::<f64>() / total);
    let r = corners.map(|v| [v[0] - com[0], v[1] - com[1]]);
    Ic {
        m,
        r,
        p: std::array::from_fn(|i| [-w * r[i][1] * m[i], w * r[i][0] * m[i]]),
    }
}

/// An entry of mass `M ≠ 1` with momenta locks the same motion (Part 6: `mᵢ ← mᵢ/M`, `pᵢ ← pᵢ/M^{3/2}`, which keeps
/// `K/|U|` and the signs of `E` and `L_z`): its lock's `K/|U|` is the entry's, its `E < 0`, its `L_z` the entry's sign
/// (the mirror undone), `E·L_z²` the entry's over `M⁵` (`E/M²`, `L_z/M^{3/2}`, and Part 2's rescale keeps `E·L_z²`),
/// and `M` is recorded.
fn check_mass_unit(entry: &Ic<f64>, want_ratio: f64) {
    let total: f64 = entry.m.iter().sum();
    let lock = exact(&latent(), Exact::Physical(*entry)).expect("the entry is refused");
    assert!(
        !lock.notices.lookup_clamped(),
        "fixture: the entry is clamped"
    );
    let x = decoded(&latent(), &lock);
    assert!(
        (virial(&x) - want_ratio).abs() <= 1e-12,
        "the lock's K/|U| = {} is not the entry's {want_ratio}",
        virial(&x)
    );
    assert!(
        energy(&x) < 0.0,
        "the lock's E = {} is not bound",
        energy(&x)
    );
    let sign = if lock.notices.mirrored { -1.0 } else { 1.0 };
    assert_eq!(
        (sign * lz(&x)).signum(),
        lz(entry).signum(),
        "the lock's sense of rotation is not the entry's"
    );
    let invariant = |x: &Ic<f64>| energy(x) * lz(x).powi(2);
    let want = invariant(entry) / total.powi(5);
    assert!(
        (invariant(&x) - want).abs() <= 1e-12 * want.abs(),
        "the lock's E·L_z² = {} is not the entry's over M⁵, {want}",
        invariant(&x)
    );
    assert!(
        (lock.notices.mass - total).abs() <= 4.0 * f64::EPSILON * total,
        "the lock records M = {}, not {total}",
        lock.notices.mass
    );
}

#[test]
fn lookup_forms_qa_mass_unit_keeps_the_motion() {
    for (m, a) in [
        ([1.0, 2.0, 3.0], 1.5),
        ([1.0, 1.0, 1.0], 1.0),
        ([0.002, 0.003, 0.004], 0.7),
        ([400.0, 250.0, 350.0], 2.0),
    ] {
        let x = lagrange(m, a);
        assert!(
            (virial(&x) - 0.5).abs() <= 1e-13 && energy(&x) < 0.0,
            "fixture: K/|U| = {}, E = {}",
            virial(&x),
            energy(&x)
        );
        check_mass_unit(&x, 0.5);
    }
}

negative_control!(
    lookup_forms_qa_mass_unit_keeps_the_motion,
    "the stale reading's lock is the masses as proportions with the M = 3/2 momenta kept, velocities times 3/2: \
     K/|U| = ½·(3/2)³, so the check must fail",
    expected = "is not the entry's",
    {
        let mut x = lagrange([0.25, 0.5, 0.75], 1.5);
        x.m = x.m.map(|m| m / 1.5);
        check_mass_unit(&x, 0.5)
    }
);

/// `M` is recorded with the lock (Part 6): the pair and triple forms' masses are the sides, so `M` is the perimeter;
/// a raw `z` decodes at `M = 1`.
fn check_mass_recorded(entries: &[(Exact, f64)]) {
    for &(entry, want) in entries {
        let lock = exact(&burrau(), entry).expect("the entry is refused");
        assert_eq!(
            lock.notices.mass, want,
            "{entry:?} records M = {}",
            lock.notices.mass
        );
    }
}

#[test]
fn lookup_forms_qa_mass_recorded() {
    check_mass_recorded(&[
        (Exact::Pythagorean { m: 3, n: 2 }, 30.0),
        (Exact::Pythagorean { m: 4, n: 1 }, 40.0),
        (Exact::Triple([10, 24, 26]), 60.0),
        (Exact::Triple([8, 15, 17]), 40.0),
        (Exact::Raw([0.3, -0.2, 0.1, 0.4, -0.5, 0.2, 0.6, -0.1]), 1.0),
    ]);
}

negative_control!(
    lookup_forms_qa_mass_recorded,
    "(3, 2)'s triangle is (5, 12, 13), perimeter 30; a check expecting the unit mass must fail",
    expected = "records M =",
    check_mass_recorded(&[(Exact::Pythagorean { m: 3, n: 2 }, 1.0)])
);

/// The partial form's default masses are `⅓` each, so a momentum block given without masses is read in `M = 1` units,
/// and masses given carry the momenta given with them (Part 6): on the latent chart, the configuration and momenta of
/// `some_state` with no masses lock as the exact entry with `default` masses, and with masses `(1, 2, 3)` as the exact
/// entry `(1, 2, 3)` and as its carried form `(⅙, ⅓, ½)` with the momenta over `6^{3/2}`; `M` is 1 and 6.
fn check_partial_mass_unit(default: [f64; 3]) {
    let chart = latent();
    let x = some_state();
    let given = [1.0, 2.0, 3.0];
    let carry = 6.0 * 6.0_f64.sqrt();
    let cases = [
        (None, Ic { m: default, ..x }, 1.0),
        (Some(given), Ic { m: given, ..x }, 6.0),
        (
            Some(given),
            Ic {
                m: given.map(|m| m / 6.0),
                r: x.r,
                p: x.p.map(|v| [v[0] / carry, v[1] / carry]),
            },
            6.0,
        ),
    ];
    for (masses, reference, total) in cases {
        let p = Partial {
            masses,
            positions: Some(x.r),
            momenta: Some(x.p),
        };
        let lock = partial(&chart, p, None).expect("a partial entry is refused");
        let want = exact(&chart, Exact::Physical(reference)).expect("the exact entry is refused");
        let (xl, xw) = (decoded(&chart, &lock), decoded(&chart, &want));
        assert!(
            phys(&xl, &xw) <= eps_phys(),
            "{p:?} does not lock as the exact {:?}: ‖Δ‖_phys = {:e}",
            reference.m,
            phys(&xl, &xw)
        );
        assert!(
            (lock.notices.mass - total).abs() <= 4.0 * f64::EPSILON * total,
            "{p:?} records M = {}, not {total}",
            lock.notices.mass
        );
    }
}

#[test]
fn lookup_forms_qa_partial_mass_unit() {
    check_partial_mass_unit([1.0 / 3.0; 3]);
}

negative_control!(
    lookup_forms_qa_partial_mass_unit,
    "default masses (1, 1, 1) are M = 3, whose momenta are carried by 3^{3/2}: another system, so the check must fail",
    expected = "does not lock as the exact",
    check_partial_mass_unit([1.0; 3])
);

// ── REQ-ENC-036: the zero class in floating point ─────────────────────────────────────────────────────────────────

/// The entered state's extents as the contract defines them (§ "The zero class in floating point"):
/// `R = λ·maxᵢ‖rᵢ‖` and `P = λ^{−1/2}·maxᵢ‖pᵢ‖` of the entry, before `C`'s translation, boost, rotation and rescale.
fn contract_extents(entry: &Ic<f64>, lambda: f64) -> (f64, f64) {
    let max = |v: &[[f64; 2]; 3]| v.iter().map(|w| w[0].hypot(w[1])).fold(0.0, f64::max);
    (lambda * max(&entry.r), max(&entry.p) / lambda.sqrt())
}

/// The first-order bounds on `L_z`'s and `E`'s rounding on `C(x)` at the entry's extents `(R, P)`, as the contract
/// writes them: `S_{L_z} = Σᵢ((1 + R)‖rᵢ‖‖pᵢ‖ + R‖pᵢ‖ + ‖rᵢ‖P)` and
/// `S_E = Σᵢ((1 + R)‖pᵢ‖²/2 + ‖pᵢ‖P)/mᵢ + Σ_{i<j} mᵢmⱼ/rᵢⱼ·(1 + R + 2R/rᵢⱼ)`.
fn contract_bounds(x: &Ic<f64>, (r, p): (f64, f64)) -> (f64, f64) {
    let norm = |v: [f64; 2]| v[0].hypot(v[1]);
    let s_lz: f64 = (0..3)
        .map(|i| {
            let (ri, pi) = (norm(x.r[i]), norm(x.p[i]));
            (1.0 + r) * ri * pi + r * pi + ri * p
        })
        .sum();
    let kinetic: f64 = (0..3)
        .map(|i| {
            let pi = norm(x.p[i]);
            ((1.0 + r) * pi * pi / 2.0 + pi * p) / x.m[i]
        })
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let d = dist(x.r[i], x.r[j]);
            x.m[i] * x.m[j] / d * (1.0 + r + 2.0 * r / d)
        })
        .sum();
    (s_lz, kinetic + potential)
}

/// A random state: masses on the simplex's interior, positions and momenta on `[−1, 1]²`.
fn random_state(rng: &mut Rng) -> Ic<f64> {
    let m: [f64; 3] = std::array::from_fn(|_| rng.uniform(0.05, 1.0));
    let t: f64 = m.iter().sum();
    Ic {
        m: m.map(|v| v / t),
        r: std::array::from_fn(|_| [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)]),
        p: std::array::from_fn(|_| [rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0)]),
    }
}

/// A random gauge image of `x` in its own mass unit: offset and boost on `[−span, span)²`, any rotation, scale
/// `2^[−octaves, octaves]`, either parity.
fn random_image(rng: &mut Rng, x: &Ic<f64>, span: f64, octaves: f64) -> Ic<f64> {
    let g = proper_image(rng, x, span, octaves);
    if rng.next() & 1 == 1 {
        gauge(&g, [0.0; 2], [0.0; 2], 0.0, (1.0, 1.0), true)
    } else {
        g
    }
}

/// A random gauge image of `x` as [`random_image`]'s, its parity kept.
fn proper_image(rng: &mut Rng, x: &Ic<f64>, span: f64, octaves: f64) -> Ic<f64> {
    gauge(
        x,
        [rng.uniform(-span, span), rng.uniform(-span, span)],
        [rng.uniform(-span, span), rng.uniform(-span, span)],
        rng.uniform(0.0, std::f64::consts::TAU),
        (2.0_f64.powf(rng.uniform(-octaves, octaves)), 1.0),
        false,
    )
}

/// `Extents::entered` gives the contract's `R` and `P`, and `Classes::within` gives `L_z`, `E` and the contract's
/// bounds `S_{L_z}` and `S_E` on the entered extents (REQ-ENC-036), at 64 gauge images of fuzzed states, each with
/// its own offset, boost and scale. `drop_p` leaves the boost's term `P` out of the expected bounds.
fn check_class_bounds(drop_p: bool) {
    let mut rng = Rng(0x2E80_0036);
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs().max(1e-300);
    for _ in 0..64 {
        let x = random_state(&mut rng);
        let g = random_image(&mut rng, &x, 8.0, 6.0);
        let e = encode::<Defaults, f64>(&g);
        let lambda = e.canonical.lambda;
        let ext = Extents::entered(&g, lambda);
        let (r, p) = contract_extents(&g, lambda);
        assert!(close(ext.r, r), "R is {}, not λ·maxᵢ‖rᵢ‖ = {r}", ext.r);
        assert!(
            close(ext.p, p),
            "P is {}, not λ^(−1/2)·maxᵢ‖pᵢ‖ = {p}",
            ext.p
        );
        let c = e.canonical.ic;
        let got = Classes::within(&c, &ext);
        let (s_lz, s_e) = contract_bounds(&c, (r, if drop_p { 0.0 } else { p }));
        let near = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs().max(1.0);
        assert!(near(got.lz, lz(&c)), "L_z = {}, not {}", got.lz, lz(&c));
        assert!(near(got.e, energy(&c)), "E = {}, not {}", got.e, energy(&c));
        assert!(
            close(got.lz_scale, s_lz),
            "L_z's bound is {}, not S_Lz = {s_lz}",
            got.lz_scale
        );
        assert!(
            close(got.e_scale, s_e),
            "E's bound is {}, not S_E = {s_e}",
            got.e_scale
        );
    }
}

#[test]
fn lookup_validation_ladder_zero_band_qa_bounds() {
    check_class_bounds(false);
}

negative_control!(
    lookup_validation_ladder_zero_band_qa_bounds,
    "without the boost's term P, the bound is the canonical state's alone, so the bound check must fail",
    expected = "L_z's bound is",
    check_class_bounds(true)
);

/// Class 0 is `|v| ≤ band·scale`, the edge included; outside it the class is `v`'s sign; a value whose scale is not
/// finite has no class to change (§ "The zero class in floating point"). `edge` is the value placed where the band
/// ends, at a scale and band whose product is exact.
fn check_zero_band(edge: f64) {
    let band = 2.0_f64.powi(-28);
    let scale = 3.0;
    let at = |lz: f64, e: f64| Classes {
        lz,
        lz_scale: scale,
        e,
        e_scale: scale,
    };
    let bound = -1.0;
    for sign in [1.0, -1.0] {
        // L_z at the band's edge and L_z = 0 are one class, so a clamp between them changes nothing.
        assert_eq!(
            changed(&at(sign * edge, bound), &at(0.0, bound), band),
            None,
            "L_z = {:e} at the band's edge is not class 0",
            sign * edge
        );
        assert_eq!(
            changed(&at(0.0, bound), &at(-sign * edge, bound), band),
            None,
            "L_z = {:e} at the band's edge is not class 0",
            -sign * edge
        );
        // E likewise.
        assert_eq!(
            changed(&at(1.0, sign * edge), &at(1.0, 0.0), band),
            None,
            "E = {:e} at the band's edge is not class 0",
            sign * edge
        );
        // One ulp past the edge is the sign's class: a clamp to 0 changes it.
        let past = (band * scale).next_up();
        assert_eq!(
            changed(&at(sign * past, bound), &at(0.0, bound), band),
            Some(Class::Rotation),
            "L_z one ulp past the band is class 0"
        );
        assert_eq!(
            changed(&at(1.0, sign * past), &at(1.0, 0.0), band),
            Some(Class::Energy),
            "E one ulp past the band is class 0"
        );
        // Opposite signs outside the band are two classes.
        assert_eq!(
            changed(&at(sign * past, bound), &at(-sign * past, bound), band),
            Some(Class::Rotation),
            "L_z's sign flipped outside the band is no change"
        );
    }
    // Relative: the same L_z is class 0 at a large scale and its sign's at a small one.
    let small = Classes {
        lz_scale: 1e-3,
        ..at(1e-9, bound)
    };
    assert_eq!(
        changed(&small, &Classes { lz: 0.0, ..small }, band),
        Some(Class::Rotation),
        "L_z = 1e-9 at scale 1e-3 is class 0"
    );
    let large = Classes {
        lz_scale: 1e3,
        ..at(1e-9, bound)
    };
    assert_eq!(
        changed(&large, &Classes { lz: 0.0, ..large }, band),
        None,
        "L_z = 1e-9 at scale 1e3 is not class 0"
    );
    // A scale not finite: the entered value has no class, so its clamp changes none.
    let unscaled = Classes {
        lz_scale: f64::INFINITY,
        ..at(1.0, bound)
    };
    assert_eq!(
        changed(&unscaled, &at(-1.0, bound), band),
        None,
        "an L_z whose scale is not finite has a class"
    );
}

#[test]
fn lookup_validation_ladder_zero_band_qa_edge() {
    check_zero_band(2.0_f64.powi(-28) * 3.0);
}

negative_control!(
    lookup_validation_ladder_zero_band_qa_edge,
    "one ulp past the band's edge is outside class 0, so the check that the edge is class 0 must fail",
    expected = "at the band's edge is not class 0",
    check_zero_band((2.0_f64.powi(-28) * 3.0).next_up())
);

/// A collinear state, unequal masses, moving along its line (`L_z = 0` in exact arithmetic, in every frame boosted
/// along it), entered at 64 orientations, each with its own offset, boost along the line, scale and mass unit: one
/// physical state, so one verdict (REQ-ENC-036). `β`'s clamp at the line moves `L_z` off 0 by about `ε_z` relative, a
/// change of the sense of rotation, so that verdict is a layer-1 rejection naming `L_z`'s class.
fn check_one_verdict(band: f64) {
    let t = Tolerances {
        zero_class: band,
        ..tol()
    };
    let x = Ic {
        m: [0.5, 0.3, 0.2],
        r: [[-0.9, 0.0], [0.4, 0.0], [1.7, 0.0]],
        p: [[0.25, 0.0], [-0.6, 0.0], [0.35, 0.0]],
    };
    let mut verdicts = Vec::new();
    for k in 0..64 {
        let angle = std::f64::consts::TAU * f64::from(k) / 64.0;
        let scale = [0.015625, 1.0, 64.0][k as usize % 3];
        let mass = [1.0, 5.0][k as usize % 2];
        let (s, c) = angle.sin_cos();
        let along = 0.3 * f64::from(k % 5) - 0.6;
        let g = gauge(
            &x,
            [2.5 - 0.1 * f64::from(k), -1.3],
            [along * c, along * s],
            angle,
            (scale, mass),
            false,
        );
        let got = lookup::<Defaults, _, _>(&latent(), &Exact::Physical(g), phys, &t);
        verdicts.push(got.map(|l| l.notices.moved.controls));
    }
    let first = verdicts[0];
    for (k, v) in verdicts.iter().enumerate() {
        assert_eq!(
            *v, first,
            "orientation {k} of 64 has its own verdict: {v:?}, not {first:?}"
        );
    }
    assert!(
        matches!(
            first,
            Err(Rejected {
                layer: Layer::Hypercube,
                failed: Failed::Clamp {
                    class: Class::Rotation,
                    ..
                }
            })
        ),
        "the collinear state's verdict is {first:?}, not a layer-1 rejection naming L_z's class"
    );
}

#[test]
fn lookup_validation_ladder_zero_band_qa_one_verdict() {
    check_one_verdict(tol().zero_class);
}

negative_control!(
    lookup_validation_ladder_zero_band_qa_one_verdict,
    "compared against exactly 0, a rounded L_z = 0 takes its rounding's sign, so the verdict follows the gauge",
    expected = "has its own verdict",
    check_one_verdict(0.0)
);

// ── REQ-ENC-038: the class a clamp moves ───────────────────────────────────────────────────────────────────────────

/// `σ`, the default links' logistic (inverse_encode_contract Part 3).
fn sigma(z: f64) -> f64 {
    1.0 / (1.0 + (-z).exp())
}

/// The canonical state of the decoder's parameters (dd_decoder §3.1, §3.2, §3.4; § "The class a clamp moves"): masses
/// `m` (`Σmᵢ = 1`), `α`, `β` and the Jacobi momenta `q = (p_ρ, p_λ)`, with `a = cos α/√μ_ρ`, `b = sin α/√μ_λ`,
/// `ρ = (a, 0)`, `λ = b·(cos β, sin β)`, the CoM at the origin and `Σpᵢ = 0`: `p₂ = p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`.
fn canonical_state(m: [f64; 3], alpha: f64, beta: f64, q: [f64; 4]) -> Ic<f64> {
    let t: f64 = m.iter().sum();
    let m = m.map(|v| v / t);
    let m01 = m[0] + m[1];
    let (a, b) = (
        alpha.cos() / (m[0] * m[1] / m01).sqrt(),
        alpha.sin() / (m[2] * m01).sqrt(),
    );
    let lam = [b * beta.cos(), b * beta.sin()];
    let c = [-m[2] * lam[0], -m[2] * lam[1]];
    let (w0, w1) = (m[0] / m01, m[1] / m01);
    let p2 = [q[2], q[3]];
    let p1 = [q[0] - w1 * q[2], q[1] - w1 * q[3]];
    Ic {
        m,
        r: [
            [c[0] - w1 * a, c[1]],
            [c[0] + w0 * a, c[1]],
            [c[0] + lam[0], c[1] + lam[1]],
        ],
        p: [[-p1[0] - p2[0], -p1[1] - p2[1]], p1, p2],
    }
}

/// `a` and `b` of a canonical state's masses at `α`.
fn ab(m: [f64; 3], alpha: f64) -> (f64, f64) {
    let m01 = m[0] + m[1];
    (
        alpha.cos() / (m[0] * m[1] / m01).sqrt(),
        alpha.sin() / (m[2] * m01).sqrt(),
    )
}

/// 64 gauge images of `x`: offset, rotation, scale `2^[−4, 4]`, mass unit, parity, and on every odd image a boost on
/// `[−4, 4)²`, so half are boosted and half not.
fn images(x: &Ic<f64>, seed: u64) -> Vec<Ic<f64>> {
    let mut rng = Rng(seed);
    (0..64)
        .map(|k| {
            let shift = [rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0)];
            let boost = [rng.uniform(-4.0, 4.0), rng.uniform(-4.0, 4.0)];
            let angle = rng.uniform(0.0, std::f64::consts::TAU);
            let scale = 2.0_f64.powf(rng.uniform(-4.0, 4.0));
            let mass = [1.0, 5.0, 0.2][k % 3];
            let boost = if k % 2 == 1 { boost } else { [0.0; 2] };
            gauge(x, shift, boost, angle, (scale, mass), k % 4 == 3)
        })
        .collect()
}

/// Every image of `x` is accepted at layer 1 with exactly the controls `want` clamped.
fn check_accepted(what: &str, x: &Ic<f64>, want: u32) {
    for (k, g) in images(x, 0x2E80_0038).iter().enumerate() {
        match exact(&latent(), Exact::Physical(*g)) {
            Ok(l) => assert_eq!(
                l.notices.moved.controls, want,
                "{what}: image {k}'s moved controls are not {want:#b}"
            ),
            Err(e) => panic!("{what}: image {k} of 64 is refused: {e:?}"),
        }
    }
}

/// Every image of `x` is rejected at layer 1, naming the controls `want` and the sense of rotation.
fn check_rejected(what: &str, x: &Ic<f64>, want: u32) {
    for (k, g) in images(x, 0x2E80_0039).iter().enumerate() {
        match exact(&latent(), Exact::Physical(*g)) {
            Err(Rejected {
                layer: Layer::Hypercube,
                failed:
                    Failed::Clamp {
                        controls,
                        class: Class::Rotation,
                    },
            }) => assert_eq!(controls, want, "{what}: image {k} names the wrong controls"),
            other => {
                panic!("{what}: image {k} of 64 is not rejected naming L_z's class: {other:?}")
            }
        }
    }
}

const Z_BETA: u32 = 1 << 1;
const Z_Q0: u32 = 1 << 2;
const Z_Q1: u32 = 1 << 3;
const Z_MASS: u32 = 0b1100_0000;

/// A collinear start, every momentum along the line, so `L_z = 0`, with `p_λ = (t, 0)`: `β = 0` clamps, and its move
/// `ΔL_z = −2b·sin(Δβ/2)·(p_λ·ê)` is nonzero as `p_λ` is, however small, so the clamp changes the sense of rotation
/// and is rejected, in every gauge (§ "The class a clamp moves", REQ-ENC-038's verify: `t = 10⁻⁴, 10⁻⁶, 10⁻⁸`).
fn collinear(t: f64) -> Ic<f64> {
    canonical_state([0.5, 0.3, 0.2], 0.8, 0.0, [0.4, 0.0, t, 0.0])
}

fn check_beta_rejected(ts: &[f64]) {
    for &t in ts {
        let x = collinear(t);
        assert_eq!(lz(&x), 0.0, "fixture: L_z is not 0");
        check_rejected(&format!("β's clamp at p_λ = {t:e}"), &x, Z_BETA);
    }
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_beta_rejected() {
    check_beta_rejected(&[1e-4, 1e-6, 1e-8]);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_beta_rejected,
    "at p_λ = 0 β's move of L_z is 0, so the clamp stands and the check expecting a rejection must fail",
    expected = "is not rejected naming L_z's class",
    check_beta_rejected(&[0.0])
);

/// The same collinear start at `p_λ = 0`, `p_ρ` along the line, bound (`E < 0`) and unbound (`E > 0`): `β`'s clamp
/// moves neither `L_z` (`p_λ·ê = 0`) nor `E`'s sign (`ΔE = ΔU`, of order `ε_z²`), so it stands in every gauge,
/// boosted or not.
fn check_beta_accepted(t: f64) {
    for q0 in [0.05, 1.5] {
        let x = canonical_state([0.5, 0.3, 0.2], 0.8, 0.0, [q0, 0.0, t, 0.0]);
        assert_eq!(lz(&x), 0.0, "fixture: L_z is not 0");
        check_accepted(
            &format!("β's clamp at p_λ = {t:e}, E = {:.3}", energy(&x)),
            &x,
            Z_BETA,
        );
    }
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_beta_accepted() {
    assert!(
        energy(&canonical_state(
            [0.5, 0.3, 0.2],
            0.8,
            0.0,
            [0.05, 0.0, 0.0, 0.0]
        )) < 0.0
            && energy(&canonical_state(
                [0.5, 0.3, 0.2],
                0.8,
                0.0,
                [1.5, 0.0, 0.0, 0.0]
            )) > 0.0,
        "fixture: the two starts are not one bound and one unbound"
    );
    check_beta_accepted(0.0);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_beta_accepted,
    "at p_λ = 10⁻⁶ β's clamp moves L_z off 0, so it is rejected and the check expecting it to stand must fail",
    expected = "is refused",
    check_beta_accepted(1e-6)
);

/// `L_z = 0` with `A = a·q₁ ≠ 0` cancelled by `B = −b·q₂` (`β = π/2`), `q₁ = 2.5` past `q_max`: `q₁`'s clamp moves
/// `L_z` by `a·Δq ≠ 0`, so it is rejected in every gauge. `balanced` false sets `q₂ = 0`, `L_z = a·q₁ > 0`, which the
/// clamp keeps positive.
fn q1_start(balanced: bool) -> Ic<f64> {
    let (m, alpha) = ([1.0 / 3.0; 3], 1.1);
    let (a, b) = ab(m, alpha);
    let q2 = if balanced { a * 2.5 / b } else { 0.0 };
    canonical_state(m, alpha, std::f64::consts::FRAC_PI_2, [0.3, 2.5, q2, 0.2])
}

fn check_q1_rejected(balanced: bool) {
    let x = q1_start(balanced);
    let (m, alpha) = ([1.0 / 3.0; 3], 1.1);
    let (a, b) = ab(m, alpha);
    assert!(
        a * 2.5 / b < Q_MAX * (1.0 - 2.0 * EPS_Q),
        "fixture: q₂ is past q_max"
    );
    check_rejected(&format!("q₁'s clamp at L_z = {:.1e}", lz(&x)), &x, Z_Q1);
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_q1_rejected() {
    check_q1_rejected(true);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_q1_rejected,
    "with q₂ = 0, L_z = a·q₁ > 0 and the clamp keeps it positive, so the check expecting a rejection must fail",
    expected = "is not rejected naming L_z's class",
    check_q1_rejected(false)
);

/// `q₀` past `q_max`: its clamp never moves `L_z` and lowers `E` (`ΔE = Δq·(q₀ + q₀')/2μ_ρ < 0`), here keeping `E > 0`,
/// at `L_z = 0` and at `L_z ≠ 0`, so it stands in every gauge. `with_q1` also puts `q₁ = 2.5` past `q_max`, balanced
/// by `q₂` to `L_z = 0`, whose clamp is rejected.
fn check_q0_accepted(with_q1: bool) {
    let (m, alpha, beta) = ([1.0 / 3.0; 3], 1.1, std::f64::consts::FRAC_PI_2);
    let (a, b) = ab(m, alpha);
    let starts = if with_q1 {
        vec![[3.0, 2.5, a * 2.5 / b, 0.0]]
    } else {
        vec![[3.0, 0.0, 0.0, 0.0], [3.0, 0.4, -0.5, 0.6]]
    };
    for q in starts {
        let x = canonical_state(m, alpha, beta, q);
        // After the clamp, q₀ = q_max: E stays above 0.
        let after = canonical_state(m, alpha, beta, [Q_MAX, q[1].min(Q_MAX), q[2], q[3]]);
        assert!(energy(&after) > 0.0, "fixture: the clamp makes E < 0");
        check_accepted(&format!("q₀'s clamp at q = {q:?}"), &x, Z_Q0);
    }
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_q0_accepted() {
    check_q0_accepted(false);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_q0_accepted,
    "with q₁ also past q_max at L_z = 0, q₁'s clamp moves L_z, so the check expecting the clamp to stand must fail",
    expected = "is refused",
    check_q0_accepted(true)
);

/// Extreme-mass-ratio rest starts, past `μ_max`: at rest `L_z = 0` and `E = U < 0`, and the mass clamp keeps both,
/// so it stands in every gauge, boosted or not. `moving` gives the third start `L_z = 0` with `A = a·q₁ ≠ 0`
/// cancelled by `B`, where the clamp changes `μ_ρ` and `μ_λ` by different ratios and so moves `L_z`
/// (`ΔL_z = A(√(μ_ρ/μ_ρ') − √(μ_λ/μ_λ'))`), which is rejected.
fn check_mass_accepted(moving: bool) {
    let starts: [([f64; 3], f64); 3] = [
        ([1.0, 1e-3, 1e-3], 0.8),
        ([1e3, 1.0, 1.0], 0.8),
        ([1.0, 1e-3, 0.5], 1.4),
    ];
    for (k, &(m, alpha)) in starts.iter().enumerate() {
        let t: f64 = m.iter().sum();
        let (a, b) = ab(m.map(|v| v / t), alpha);
        let beta = std::f64::consts::FRAC_PI_2;
        let q = if moving && k == 2 {
            [0.0, 0.5, a * 0.5 / b, 0.0]
        } else {
            [0.0; 4]
        };
        let x = canonical_state(m, alpha, beta, q);
        let e = encode::<Defaults, f64>(&x);
        let bits = e.report.clamped_controls;
        assert!(
            bits & Z_MASS != 0 && bits & !Z_MASS == 0,
            "fixture: masses {m:?} clamp {bits:#b}, not the masses alone"
        );
        check_accepted(&format!("the mass clamp at m = {m:?}, q = {q:?}"), &x, bits);
    }
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_mass_accepted() {
    check_mass_accepted(false);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_mass_accepted,
    "a moving L_z = 0 start with A ≠ 0 has its L_z moved by the mass clamp, so the check expecting it to stand must fail",
    expected = "is refused",
    check_mass_accepted(true)
);

/// The rule itself (§ "The zero class in floating point", "The class a clamp moves"): a constant in class 0 before the
/// clamp takes the class of its move at the move's bound, whatever its decoded value after; any other takes the class
/// of its value after at that value's bound. `band` is the zero class's `k·ε`.
fn check_rule(band: f64) {
    let before = |lz: f64, e: f64| Classes {
        lz,
        lz_scale: 1.0,
        e,
        e_scale: 1.0,
    };
    let moves = |lz: f64, lz_after: f64, e: f64, e_after: f64| Moves {
        lz,
        lz_scale: 1e-13,
        e,
        e_scale: 1e-13,
        lz_after,
        lz_after_scale: 1.0,
        e_after,
        e_after_scale: 1.0,
    };
    let verdict = |b: Classes, m: Moves| changed(&b, &after(&b, &m, band), band);
    // L_z in class 0: a move of 10⁻¹² against its bound of 10⁻¹³ is a sign, though the decoded value is in class 0.
    assert_eq!(
        verdict(before(1e-20, -1.0), moves(1e-12, 1e-20, 0.0, -1.0)),
        Some(Class::Rotation),
        "a move outside its own band is read as rounding"
    );
    // A move inside its own band is no change, though the decoded value is outside the band.
    assert_eq!(
        verdict(before(1e-20, -1.0), moves(1e-30, 0.5, 0.0, -1.0)),
        None,
        "the class after a clamp is read from the decoded value, not the move"
    );
    // L_z outside class 0: the value after decides, at its own bound.
    assert_eq!(
        verdict(before(0.3, -1.0), moves(-0.5, -0.2, 0.0, -1.0)),
        Some(Class::Rotation),
        "L_z's sign flipped by the clamp is no change"
    );
    assert_eq!(
        verdict(before(0.3, -1.0), moves(-0.1, 0.2, 0.0, -1.0)),
        None,
        "L_z kept positive by the clamp is a change"
    );
    // E in class 0: its move's sign is its class after.
    assert_eq!(
        verdict(before(0.3, 1e-20), moves(0.0, 0.3, -1e-12, 1e-20)),
        Some(Class::Energy),
        "E's move outside its own band is read as rounding"
    );
}

#[test]
fn lookup_validation_ladder_clamp_rule_qa_rule() {
    check_rule(ZERO_CLASS.number() * f64::EPSILON);
}

negative_control!(
    lookup_validation_ladder_clamp_rule_qa_rule,
    "a band of 10³, ten times the move over its bound, swallows the move, so the check that it is a sign must fail",
    expected = "a move outside its own band is read as rounding",
    check_rule(1e3)
);

/// Each clamped control's move against the contract's closed form, evaluated here from the encoded parameters and the
/// clamp's image `σ(z)` (§ "The class a clamp moves"; dd_decoder §3.2, §3.4), within the zero class's band of the
/// move's own bound, with its direction: `β` at a collinear start, `ΔL_z = −2b·sin(Δβ/2)·(p_λ·ê)` and
/// `ΔE = 2ab·(m₀m₁m₂/M₀₁)(cos β' − cos β)[1/(r₀₂r₀₂'(r₀₂ + r₀₂')) − 1/(r₁₂r₁₂'(r₁₂ + r₁₂'))]`; `q₁`, `ΔL_z = a·Δq`
/// and `ΔE = Δq(q₁ + q₁')/2μ_ρ < 0`; `q₀`, `ΔL_z` in class 0 and `ΔE = Δq(q₀ + q₀')/2μ_ρ < 0`; a rest start's
/// masses, `ΔL_z` in class 0 and `E` after equal to `U` at the clamped masses, below 0. `sign` multiplies the
/// expected `β` move.
fn check_closed_forms(sign: f64) {
    let k = ZERO_CLASS.number() * f64::EPSILON;
    let within = |got: f64, want: f64, bound: f64, what: &str| {
        assert!(
            (got - want).abs() <= k * bound,
            "{what} is {got:e}, not the closed form's {want:e} within k·ε·{bound:e}"
        );
    };
    let moved = |x: &Ic<f64>, want: u32| {
        let e = encode::<Defaults, f64>(x);
        assert_eq!(
            e.report.clamped_controls, want,
            "fixture: the clamped controls"
        );
        let ext = Extents::entered(x, e.canonical.lambda);
        let mv = moves::<Defaults>(&e, &e.z.z, &ext);
        (e, mv)
    };
    let mu_rho = |m: [f64; 3]| m[0] * m[1] / (m[0] + m[1]);
    // β at a collinear start, p_λ = (0.3, 0).
    {
        let (e, mv) = moved(&collinear(0.3), Z_BETA);
        let m = e.canonical.ic.m;
        let (a, b) = ab(m, e.blocks.alpha);
        let (beta, beta_c) = (e.blocks.beta, std::f64::consts::PI * sigma(e.z.z[1]));
        let pl = e.blocks.jacobi.p_lambda;
        let (db, mean) = (beta_c - beta, 0.5 * (beta + beta_c));
        let along = pl[0] * mean.cos() + pl[1] * mean.sin();
        let dl = -2.0 * b * (0.5 * db).sin() * along;
        within(mv.lz, sign * dl, mv.lz_scale, "β's ΔL_z");
        assert_eq!(
            mv.lz.signum(),
            -db.signum() * along.signum(),
            "β's ΔL_z has the wrong direction"
        );
        let m01 = m[0] + m[1];
        let (w0, w1) = (m[0] / m01, m[1] / m01);
        let r = |w: f64, c: f64| (b * b + w * w * a * a + 2.0 * w * a * b * c).sqrt();
        let (r02, r02c) = (r(w1, beta.cos()), r(w1, beta_c.cos()));
        let (r12, r12c) = (r(-w0, beta.cos()), r(-w0, beta_c.cos()));
        let dcos = -2.0 * mean.sin() * (0.5 * db).sin();
        let de = 2.0
            * a
            * b
            * (m[0] * m[1] * m[2] / m01)
            * dcos
            * (1.0 / (r02 * r02c * (r02 + r02c)) - 1.0 / (r12 * r12c * (r12 + r12c)));
        within(mv.e, de, mv.e_scale, "β's ΔE");
    }
    // q₁ past q_max at L_z = 0.
    {
        let (e, mv) = moved(&q1_start(true), Z_Q1);
        let m = e.canonical.ic.m;
        let (a, _) = ab(m, e.blocks.alpha);
        let q = e.blocks.jacobi.p_rho[1];
        let qc = Q_MAX * (2.0 * sigma(e.z.z[3]) - 1.0);
        within(mv.lz, a * (qc - q), mv.lz_scale, "q₁'s ΔL_z");
        assert!(mv.lz < 0.0, "q₁'s ΔL_z is not −sign(q₁)·a");
        within(
            mv.e,
            (qc - q) * (q + qc) / (2.0 * mu_rho(m)),
            mv.e_scale,
            "q₁'s ΔE",
        );
        assert!(mv.e < 0.0, "q₁'s ΔE is not below 0");
    }
    // q₀ past q_max.
    {
        let x = canonical_state([1.0 / 3.0; 3], 1.1, 1.2, [3.0, 0.4, -0.5, 0.6]);
        let (e, mv) = moved(&x, Z_Q0);
        let m = e.canonical.ic.m;
        let q = e.blocks.jacobi.p_rho[0];
        let qc = Q_MAX * (2.0 * sigma(e.z.z[2]) - 1.0);
        within(mv.lz, 0.0, mv.lz_scale, "q₀'s ΔL_z");
        within(
            mv.e,
            (qc - q) * (q + qc) / (2.0 * mu_rho(m)),
            mv.e_scale,
            "q₀'s ΔE",
        );
        assert!(mv.e < 0.0, "q₀'s ΔE is not below 0");
    }
    // A rest start's masses.
    {
        let x = canonical_state([1.0, 1e-3, 1e-3], 0.8, 1.3, [0.0; 4]);
        let bits = encode::<Defaults, f64>(&x).report.clamped_controls;
        let (e, mv) = moved(&x, bits);
        within(mv.lz, 0.0, mv.lz_scale, "the masses' ΔL_z");
        let mc = kernel::decode::mass::decode::<Defaults, f64>([e.z.z[6], e.z.z[7]]).m;
        let (alpha, beta) = (e.blocks.alpha, e.blocks.beta);
        let after = canonical_state(mc, alpha, beta, [0.0; 4]);
        within(
            mv.e_after,
            energy(&after),
            mv.e_after_scale,
            "E after the mass clamp",
        );
        assert!(
            mv.e_after < 0.0,
            "E after a rest start's mass clamp is not below 0"
        );
    }
}

#[test]
fn lookup_validation_ladder_clamp_qa_closed_forms() {
    check_closed_forms(1.0);
}

negative_control!(
    lookup_validation_ladder_clamp_qa_closed_forms,
    "β's move with its sign flipped is not the contract's closed form, so the check must fail",
    expected = "β's ΔL_z is",
    check_closed_forms(-1.0)
);

// ── REQ-ENC-037: the band's k against the rounding, each measurement on its own ───────────────────────────────────

/// `C`'s rounding of `L_z` and `E` against its bound, in units of `ε`, over gauge images of `n` fuzzed states (offset
/// and boost on `[−8, 8)²`, any rotation, scale `2^[−12, 12]`, either parity): two images of one state are one
/// physical state, so `|v₁ − v₂| ≤ ρ·ε·(S₁ + S₂)` where `ρ` bounds each one's rounding over its bound, and the worst
/// `|v₁ − v₂|/(ε(S₁ + S₂))` is a measurement of `ρ`; and for `n` collinear states moving along their line, whose `L_z`
/// is 0 exactly, `|L_z|/(ε·S_{L_z})` of one image. `canonical_only` reads the bounds at `R = P = 0`, the canonical
/// state's alone.
fn measure_gauge(n: usize, canonical_only: bool) -> f64 {
    let eps = f64::EPSILON;
    let mut rng = Rng(0x2E80_0037);
    let bounds = |g: &Ic<f64>, c: &kernel::encode::Encoded<f64>| {
        let ext = if canonical_only {
            (0.0, 0.0)
        } else {
            contract_extents(g, c.canonical.lambda)
        };
        contract_bounds(&c.canonical.ic, ext)
    };
    let ratio = |d: f64, s: f64| if d == 0.0 { 0.0 } else { d / (eps * s) };
    let mut worst = 0.0_f64;
    for _ in 0..n {
        let x = random_state(&mut rng);
        let (g1, g2) = (
            random_image(&mut rng, &x, 8.0, 12.0),
            random_image(&mut rng, &x, 8.0, 12.0),
        );
        let (e1, e2) = (encode::<Defaults, f64>(&g1), encode::<Defaults, f64>(&g2));
        let (c1, c2) = (Classes::of(&e1.canonical.ic), Classes::of(&e2.canonical.ic));
        let (s1, s2) = (bounds(&g1, &e1), bounds(&g2, &e2));
        worst = worst
            .max(ratio((c1.lz - c2.lz).abs(), s1.0 + s2.0))
            .max(ratio((c1.e - c2.e).abs(), s1.1 + s2.1));
    }
    for _ in 0..n {
        let mut x = random_state(&mut rng);
        x.r = x.r.map(|v| [v[0], 0.0]);
        x.p = x.p.map(|v| [v[0], 0.0]);
        let g = random_image(&mut rng, &x, 8.0, 12.0);
        let e = encode::<Defaults, f64>(&g);
        let c = Classes::of(&e.canonical.ic);
        worst = worst.max(ratio(c.lz.abs(), bounds(&g, &e).0));
    }
    worst
}

/// `k` is at least 4× above `C`'s measured rounding against its bound (REQ-ENC-037's first measurement).
fn check_gauge_k(k: f64, canonical_only: bool) {
    let worst = measure_gauge(1 << 13, canonical_only);
    println!("zero_class: C's rounding {worst:.3e} ε of its bound; k = {k:e}");
    assert!(
        4.0 * worst <= k,
        "C's rounding, {worst:.3e} ε of its bound, is not within k = {k:e} by 4×"
    );
}

#[test]
fn lookup_validation_ladder_zero_band_measure_qa_gauge() {
    check_gauge_k(ZERO_CLASS.number(), false);
}

negative_control!(
    lookup_validation_ladder_zero_band_measure_qa_gauge,
    "the canonical state's bound alone misses the entry's translation and boost, so C's rounding exceeds it",
    expected = "is not within k",
    check_gauge_k(ZERO_CLASS.number(), true)
);

/// A `z` past its clamp: controls on `[−1.5, 1.5]`, one or two of them pushed past the clamp (`|z| ∈ [16, 22]` for
/// `α`, `β` and the momenta, `[8, 12]` for the masses), decoded.
fn clamped_state(rng: &mut Rng) -> Ic<f64> {
    let mut z: [f64; 8] = std::array::from_fn(|_| rng.uniform(-1.5, 1.5));
    for _ in 0..1 + (rng.next() & 1) {
        let k = (rng.next() % 8) as usize;
        let sign = if rng.next() & 1 == 1 { 1.0 } else { -1.0 };
        z[k] = sign
            * if k < 6 {
                rng.uniform(16.0, 22.0)
            } else {
                rng.uniform(8.0, 12.0)
            };
    }
    d(z)
}

/// The moves' and the values after them, their rounding against their bounds, in units of `ε`, over two gauge images
/// each of `n` clamped states (offset and boost on `[−4, 4)²`, scale `2^[−4, 4]`, parity kept), the pair taken where
/// both images clamp the same controls under the same mirror decision, so that they are one canonical state and one
/// clamp (at the `α` poles `λ̃_y` is rounding, and `C` may mirror one image and not the other): `|v₁ − v₂|/(ε(s₁ + s₂))` for
/// `ΔL_z`, `ΔE`, and `L_z` and `E` after; and for moves 0 in exact arithmetic (`β` at a collinear start with
/// `p_λ = 0`, `q₀`'s `ΔL_z`), `|Δ|/(ε·s)` of one image. `by_difference` replaces each move by the difference of the two
/// rounded values, after (the decode of the clamped `z`) less before (`C(x)`), the form the contract forbids.
fn measure_moves(n: usize, by_difference: bool) -> (f64, usize) {
    let eps = f64::EPSILON;
    let mut rng = Rng(0x2E80_003A);
    let ratio = |d: f64, s: f64| if d == 0.0 { 0.0 } else { d / (eps * s) };
    let of = |g: &Ic<f64>| {
        let e = encode::<Defaults, f64>(g);
        let mut mv = moves::<Defaults>(&e, &e.z.z, &Extents::entered(g, e.canonical.lambda));
        if by_difference {
            let (b, a) = (Classes::of(&e.canonical.ic), Classes::of(&d(e.z.z)));
            mv.lz = a.lz - b.lz;
            mv.e = a.e - b.e;
        }
        ((e.report.clamped_controls, e.canonical.mirrored), mv)
    };
    let mut worst = 0.0_f64;
    let mut pairs = 0;
    for _ in 0..n {
        let x = clamped_state(&mut rng);
        let (g1, g2) = (
            proper_image(&mut rng, &x, 4.0, 4.0),
            proper_image(&mut rng, &x, 4.0, 4.0),
        );
        let ((b1, m1), (b2, m2)) = (of(&g1), of(&g2));
        // Near the α poles β is undefined, and `C`'s mirror decision and the clamped controls follow the gauge: the
        // two canonical states are then mirror images, each clamped toward the same `β` edge, so not one clamp.
        if b1 != b2 || b1.0 == 0 {
            continue;
        }
        pairs += 1;
        worst = worst
            .max(ratio((m1.lz - m2.lz).abs(), m1.lz_scale + m2.lz_scale))
            .max(ratio((m1.e - m2.e).abs(), m1.e_scale + m2.e_scale))
            .max(ratio(
                (m1.lz_after - m2.lz_after).abs(),
                m1.lz_after_scale + m2.lz_after_scale,
            ))
            .max(ratio(
                (m1.e_after - m2.e_after).abs(),
                m1.e_after_scale + m2.e_after_scale,
            ));
    }
    for _ in 0..n / 4 {
        let m = std::array::from_fn(|_| rng.uniform(0.1, 1.0));
        let alpha = rng.uniform(0.3, 1.3);
        let at_line = canonical_state(m, alpha, 0.0, [rng.uniform(-1.0, 1.0), 0.0, 0.0, 0.0]);
        let q0 = canonical_state(
            m,
            alpha,
            rng.uniform(0.3, 2.8),
            [
                3.0,
                rng.uniform(-1.0, 1.0),
                rng.uniform(-1.0, 1.0),
                rng.uniform(-1.0, 1.0),
            ],
        );
        for x in [at_line, q0] {
            let (_, mv) = of(&random_image(&mut rng, &x, 4.0, 4.0));
            worst = worst.max(ratio(mv.lz.abs(), mv.lz_scale));
        }
    }
    (worst, pairs)
}

/// `k` is at least 4× above the moves' and after-values' measured rounding against their bounds (REQ-ENC-037's
/// second measurement).
fn check_moves_k(k: f64, by_difference: bool) {
    let n = 1 << 12;
    let (worst, pairs) = measure_moves(n, by_difference);
    println!("zero_class: the moves' rounding {worst:.3e} ε of their bound over {pairs} pairs; k = {k:e}");
    assert!(
        pairs >= n / 2,
        "fixture: only {pairs} of {n} pairs are one clamp"
    );
    assert!(
        4.0 * worst <= k,
        "the moves' rounding, {worst:.3e} ε of their bound, is not within k = {k:e} by 4×"
    );
}

#[test]
fn lookup_validation_ladder_zero_band_measure_qa_moves() {
    check_moves_k(ZERO_CLASS.number(), false);
}

negative_control!(
    lookup_validation_ladder_zero_band_measure_qa_moves,
    "a move as the difference of two rounded values carries their rounding, far above the move's own bound",
    expected = "is not within k",
    check_moves_k(ZERO_CLASS.number(), true)
);

// ── REQ-ENC-038: the α = 0 pole (R-591) ─────────────────────────────────────────────────────────────────────────────

/// Masses for the pole starts: unequal (so `U`'s gradient in `λ` at the pole is not 0) and equal.
const POLE_MASSES: [[f64; 3]; 2] = [[0.5, 0.3, 0.2], [1.0 / 3.0; 3]];

/// A start at the `α = 0` pole (`b = 0`, body 2 at the inner pair's centre of mass, `β` undefined; `canonical_state`
/// is given `β = 0.7`, which `b = 0` erases), the inner pair breathing along `ρ` with `q₀` and `q₁ = 0`, so `A = 0`,
/// `λ × p_λ = 0` and `L_z = 0`, and `p_λ = t·(cos φ, sin φ)`.
fn pole_start(m: [f64; 3], q0: f64, t: f64, phi: f64) -> Ic<f64> {
    canonical_state(m, 0.0, 0.7, [q0, 0.0, t * phi.cos(), t * phi.sin()])
}

/// The verdict on `x` and its 64 gauge images, plus `x` itself (on `λ̃ = 0` exactly): `Ok(())` where lookup locks it
/// with `α` among the clamped controls, `Err` naming what it gave otherwise.
fn pole_verdicts(x: &Ic<f64>) -> Vec<Result<(), String>> {
    let mut all = vec![*x];
    all.extend(images(x, 0x2E80_0591));
    all.iter()
        .map(|g| match exact(&latent(), Exact::Physical(*g)) {
            Ok(l) if l.notices.moved.controls & 1 == 1 => Ok(()),
            other => Err(format!("{other:?}")),
        })
        .collect()
}

/// R-591: at the pole a clamp of `α` with `p_λ ≠ 0` is rejected in every gauge, naming `L_z`'s class and `α` among the
/// controls, whatever `p_λ`'s direction (along `ρ`, across it, between) and however small above rounding, bound or
/// unbound, equal masses or not.
fn check_pole_rejected(t: f64) {
    for m in POLE_MASSES {
        for q0 in [0.1, 1.5] {
            for phi in [0.0, std::f64::consts::FRAC_PI_2, 2.3] {
                let x = pole_start(m, q0, t, phi);
                assert!(lz(&x).abs() <= 1e-15, "fixture: L_z is not 0");
                let mut all = vec![x];
                all.extend(images(&x, 0x2E80_0592));
                for (k, g) in all.iter().enumerate() {
                    match exact(&latent(), Exact::Physical(*g)) {
                        Err(Rejected {
                            layer: Layer::Hypercube,
                            failed:
                                Failed::Clamp {
                                    controls,
                                    class: Class::Rotation,
                                },
                        }) => assert_eq!(
                            controls & 1,
                            1,
                            "pole, m = {m:?}, p_λ = {t:e}: image {k} does not name α"
                        ),
                        other => panic!(
                            "pole, m = {m:?}, q₀ = {q0}, p_λ = {t:e} at φ = {phi}: image {k} is not rejected \
                             naming L_z's class: {other:?}"
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn lookup_validation_ladder_pole_qa_rejected() {
    check_pole_rejected(1e-2);
    check_pole_rejected(1e-9);
}

negative_control!(
    lookup_validation_ladder_pole_qa_rejected,
    "with p_λ = 0 the pole's clamp moves L_z by nothing, so it stands and the check expecting it rejected must fail",
    expected = "is not rejected",
    check_pole_rejected(0.0)
);

/// R-591: at the pole a clamp of `α` with `p_λ = 0` is accepted in every gauge, boosts included (where `p_λ` is 0
/// only within its rounding), bound or unbound, the inner pair breathing either way; one physical state, one verdict.
fn check_pole_accepted(t: f64) {
    for m in POLE_MASSES {
        for q0 in [0.1, -0.1, 1.5] {
            let x = pole_start(m, q0, t, 0.4);
            for (k, v) in pole_verdicts(&x).iter().enumerate() {
                if let Err(e) = v {
                    panic!("pole, m = {m:?}, q₀ = {q0}, p_λ = {t:e}: image {k} is refused: {e}");
                }
            }
        }
    }
}

#[test]
fn lookup_validation_ladder_pole_qa_accepted() {
    let (bound, unbound) = (
        pole_start(POLE_MASSES[0], 0.1, 0.0, 0.0),
        pole_start(POLE_MASSES[0], 1.5, 0.0, 0.0),
    );
    assert!(
        energy(&bound) < 0.0 && energy(&unbound) > 0.0,
        "fixture: the starts are not one bound and one unbound"
    );
    check_pole_accepted(0.0);
}

negative_control!(
    lookup_validation_ladder_pole_qa_accepted,
    "at p_λ = 10⁻⁶ the worst case over β moves L_z off 0, so the clamp is rejected and the check expecting it to stand \
     must fail",
    expected = "is refused",
    check_pole_accepted(1e-6)
);
