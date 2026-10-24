//! The round-trip suite (inverse-encode contract Parts 1, 4, 6 and 8; dd_encode §3.2, §3.5 and §5 tests 1, 3 and 8;
//! dd_decoder §3.3, §3.4 and §5 test 7; TASK-M2-16), on the CPU at f64 and f32, the one source's two instantiations,
//! every residual in physical units (`validation::roundtrip`):
//! - `t2_per_chart`, generated for every chart of the registry (`validation::registered_charts!`): interior samples
//!   `(u, v)` → the chart's `D` → `E` → `D`, `‖·‖_phys ≤ ε_phys`, against the decoded state `x`, and on the shape sphere
//!   chart, whose lower-hemisphere decode is off `C`'s section, against `C(x)` (R-544; `validation::roundtrip::Against`);
//!   a sample with a control past its clamp asserts its `lookup_clamped` bit instead, and one inside asserts the bit
//!   clear (REQ-ENC-004). `t2_per_chart_registry_complete`:
//!   every chart `engine::chart_authoring` and `kernel::chart` declare has a registry entry, so a T2 test.
//! - `t2_qmax_boundary`: random `z` across the momentum clamps; decode applies no per-body cap, the decoded momenta
//!   being the Jacobi map of `q` with `|qₖ| ≤ q_max` alone bounding them; the round trip holds on the interior, and the
//!   clamped samples carry their flags (REQ-DEC-029).
//! - `t3_half_mirror`: dd_encode §5 test 3 on random scale, orientation, offset and parity: masses and shape match,
//!   `E` matches after the recorded rescale, `|L_z|` matches with its sign by `lookup_mirrored`, and `D(E(x))` is the
//!   orbit's canonical representative; a configuration-only mirror fails the `L_z` check (REQ-ENC-005).
//! - `t3_full_state_rigid_ops`: random-gauge `x` → `E` → `D` equals `C(x)`, the kernel's own, with the same `L_z` check;
//!   a configuration-only mirror and a configuration-only rotation each fail it; at `ρ = 0`, every sign of its zeros,
//!   `φ` is +0 and the full-state rotation the identity, at both precisions (REQ-DEC-016; R-437).
//! - `encode_reuses_decode`: `decode(encode(x)) = canonicalise(x)` within `ε_phys` on random physical states
//!   (REQ-ENC-001).
//! - `encode_report`: encode's discard report on the T1–T3 inputs: `λ = 1/s` and `lookup_mirrored` the gauge's parity
//!   (T1, T3), no clamp on the interior (T2), a clamped control's bit and the coincident pair's `lookup_clamped`; and the
//!   orbit's decode is its canonical representative (REQ-ENC-018).
//! - `conditioning_physical_space`: dd_encode §5 test 8: near the momentum clamps the `z` residual is ~10⁵× the physical
//!   one; the physical tolerance passes and a naive `z` tolerance fails (REQ-VAL-013).
//! - `encode_golden_pairs`: encode's golden pairs (dd_encode §5), `fixtures/states/encode_golden.json`: `z = 0` and the
//!   golden IC, and the physical Burrau (3, 4, 5) construction with its `ν = ½` and mass point `(5, 4, 3)/12`.
//!
//! **The tolerance** is `ε_phys` alone (REQ-ENC-024, proposed by the `encode-eps-phys` gate, provisional until the M2
//! gate): `ε_phys = k·ε_R`, `k` the register's `eps_phys`, in Part 4's norm `‖·‖_phys`. T3's checks of `L_z`, `E` and the
//! shape against the input's own are held to the change a state within `ε_phys` of the canonical one can make to each,
//! its first-order bound evaluated at the decoded state ([`bounds`]), so no second tolerance is set.
//!
//! **The link set** is the registry's defaults throughout, as the gate's is.
//!
//! **Domain** (pitfalls §10): encode and decode alone, on states held fixed; no trajectory.

use std::path::Path;

use kernel::canon::canonicalise;
use kernel::chart::{Chart, EuclidPlane};
use kernel::decode::momentum::to_particles;
use kernel::decode::{self, Decoded, Ic, Latent};
use kernel::encode::{blocks, encode, Report};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::Defaults;
use kernel::shape::shape;
use validation::decode_f32_eps::SplitMix;
#[cfg(feature = "controls")]
use validation::eps_phys::Mismatched;
use validation::gauge::{uniform, Gauge};
use validation::negative_control;
use validation::roundtrip::{
    self, eps_phys, margins, phys, registry, trip, trip_against, widen, Against, DecodeAt,
    Precision, Registered, Sides, Trip,
};

// ── Shared ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// `x` rounded to `R`.
fn narrow<R: Precision>(x: &Ic<f64>) -> Ic<R> {
    Ic {
        m: x.m.map(R::of),
        r: x.r.map(|v| v.map(R::of)),
        p: x.p.map(|v| v.map(R::of)),
    }
}

/// `x` widened to f64.
fn f<R: Precision>(x: R) -> f64 {
    x.into()
}

/// `R`'s width in bits.
fn bits<R>() -> usize {
    8 * std::mem::size_of::<R>()
}

/// Which encode a round trip runs: the defaults, the decode's own links, or, in the controls, `Mismatched`, every link
/// swapped on the encode side only (dd_encode §4, seam 2/13). Each item only a control reads is
/// compiled with the controls, under their feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Enc {
    Defaults,
    #[cfg(feature = "controls")]
    Mismatched,
}

fn round<R: Precision>(enc: Enc, x: &Ic<R>, against: Against) -> Trip<R> {
    match enc {
        Enc::Defaults => trip_against::<Defaults, Defaults, R>(x, against),
        #[cfg(feature = "controls")]
        Enc::Mismatched => trip_against::<Mismatched, Defaults, R>(x, against),
    }
}

/// `‖v‖`.
fn norm(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}

/// The free decode of `z` at `R`.
fn decode_z<R: Precision>(z: [R; 8]) -> Decoded<R> {
    decode::decode::<Defaults, R>(&Latent::new(z))
}

// ── T2 per chart (REQ-ENC-004) ──────────────────────────────────────────────────────────────────────────────────

/// The points `(u, v)` drawn per chart and precision.
const T2_SAMPLES: u32 = 4096;

/// T2 at one sample (Parts 4 and 8): every control past its clamp sets its `lookup_clamped` bit and every control inside
/// leaves it clear, by the oracle on `x`; and a sample encode does not clamp is held to `ε_phys`.
fn check_t2_sample<R: Precision>(what: &dyn Fn() -> String, x: &Ic<R>, t: &Trip<R>) {
    let sides = Sides::of(margins(&widen(x)));
    let bits = t.encoded.report.clamped_controls;
    assert!(
        sides.past & !bits == 0,
        "{}: lookup_clamped expected: controls {:08b} are past their clamps, the clamp bits are {bits:08b}",
        what(),
        sides.past
    );
    assert!(
        sides.inside & bits == 0,
        "{}: clamp bits {bits:08b} set on controls {:08b} inside their clamps",
        what(),
        sides.inside
    );
    if !t.encoded.report.lookup_clamped() {
        let eps = eps_phys::<R>();
        assert!(
            t.residual <= eps,
            "{}: the T2 residual {:.3e} exceeds ε_phys {eps:.3e}",
            what(),
            t.residual
        );
    }
}

/// The points `(u, v)` of `[0, 1]²` T2 visits: [`T2_SAMPLES`] uniform draws, then the grid `{0, ¼, ½, ¾, 1}²`, which
/// holds the charts' edges and centre lines, where a chart reaches its clamps (the shape sphere's equator, the
/// collinear shapes, among them).
fn t2_points() -> Vec<(f64, f64)> {
    let mut rng = SplitMix::new(16);
    let mut out: Vec<(f64, f64)> = (0..T2_SAMPLES)
        .map(|_| (uniform(&mut rng, 0.0, 1.0), uniform(&mut rng, 0.0, 1.0)))
        .collect();
    let grid = [0.0, 0.25, 0.5, 0.75, 1.0];
    out.extend(grid.iter().flat_map(|&u| grid.map(|v| (u, v))));
    out
}

/// T2 over `chart` at `R` under `enc`, every sample checked, against the chart's own reference
/// ([`Registered::against`]: `C(x)` on the shape sphere, R-544); the interior samples' count.
fn t2_chart<R: DecodeAt>(chart: &dyn Registered, enc: Enc) -> usize {
    let mut interior = 0;
    for (i, (u, v)) in t2_points().into_iter().enumerate() {
        let x = R::decode_at(chart, u, v).ic;
        let t = round::<R>(enc, &x, chart.against());
        let what = || format!("{} f{} sample {i} at ({u}, {v})", chart.key(), bits::<R>());
        check_t2_sample(&what, &x, &t);
        interior += usize::from(Sides::of(margins(&widen(&x))).interior());
    }
    interior
}

/// T2 on the registered chart `key` at both precisions, at least half of its samples interior.
fn t2(key: &str, enc: Enc) {
    let chart = roundtrip::chart(key).unwrap_or_else(|| panic!("no registered chart {key}"));
    for (n, width) in [
        (t2_chart::<f64>(chart.as_ref(), enc), 64),
        (t2_chart::<f32>(chart.as_ref(), enc), 32),
    ] {
        assert!(
            n >= T2_SAMPLES as usize / 2,
            "{key} f{width}: {n} interior samples of {T2_SAMPLES}, too few for T2"
        );
    }
}

/// One T2 test per registered chart, `<key>::t2_per_chart`, with its control.
macro_rules! t2_per_chart {
    ($key:ident) => {
        mod $key {
            use super::*;

            #[test]
            fn t2_per_chart() {
                t2(stringify!($key), Enc::Defaults);
            }

            negative_control!(
                t2_per_chart,
                "E under every link swapped, D under the defaults, must fail T2",
                expected = "exceeds ε_phys",
                t2(stringify!($key), Enc::Mismatched)
            );
        }
    };
}

validation::registered_charts!(t2_per_chart);

/// The type names `src` declares `trait_` for: `impl<…> trait_ for Name<…>` lines.
fn declared(src: &str, trait_: &str) -> Vec<String> {
    let pattern = format!(" {trait_} for ");
    src.lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with("impl"))
        .filter_map(|l| {
            let rest = &l[l.find(&pattern)? + pattern.len()..];
            rest.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .map(str::to_owned)
        })
        .collect()
}

/// Every chart type: `Authored` in `engine::chart_authoring::charts`, the registered charts' descriptors, and `Chart`
/// in `kernel::chart`'s sources.
fn chart_types() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let read = |p: &Path| {
        std::fs::read_to_string(p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
    };
    let mut types = declared(
        &read(&root.join("../engine/src/chart_authoring/charts.rs")),
        "Authored",
    );
    let dir = root.join("src/chart");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();
    for f in files {
        types.extend(declared(&read(&f), "Chart"));
    }
    types.sort();
    types.dedup();
    types
}

/// Every chart type has a registry entry among `registered`, the entries' type names.
fn check_registry_complete(registered: &[&str]) {
    let types = chart_types();
    assert!(
        types.len() >= 11,
        "read only {} chart types: {types:?}",
        types.len()
    );
    for t in &types {
        assert!(
            registered.contains(&t.as_str()),
            "the chart {t} has no registry entry, so no T2 test (registered: {registered:?})"
        );
    }
}

#[test]
fn t2_per_chart_registry_complete() {
    let charts = registry();
    let names: Vec<&str> = charts.iter().map(|c| c.type_name()).collect();
    check_registry_complete(&names);
}

negative_control!(
    t2_per_chart_registry_complete,
    "a registry without its mixed-axis charts must fail the completeness check",
    expected = "has no registry entry",
    {
        let charts = registry();
        let names: Vec<&str> = charts
            .iter()
            .map(|c| c.type_name())
            .filter(|&n| n != "Mixed")
            .collect();
        check_registry_complete(&names);
    }
);

// ── t2_qmax_boundary (REQ-DEC-029) ──────────────────────────────────────────────────────────────────────────────

/// The draws per precision.
const QMAX_SAMPLES: u32 = 8192;

/// `x` with each body's momentum capped at `q_max`, then the total momentum re-imposed as zero: the rejected cap
/// (philosophy §8.2), the control's decode.
fn capped(x: &Ic<f64>) -> Ic<f64> {
    let q_max = <f64 as ChartConstants>::Q_MAX;
    let mut p = x.p.map(|v| {
        let n = norm(v);
        if n > q_max {
            [v[0] * q_max / n, v[1] * q_max / n]
        } else {
            v
        }
    });
    let total = [p[0][0] + p[1][0] + p[2][0], p[0][1] + p[1][1] + p[2][1]];
    for (pi, mi) in p.iter_mut().zip(x.m) {
        pi[0] -= mi * total[0];
        pi[1] -= mi * total[1];
    }
    Ic { m: x.m, r: x.r, p }
}

/// No per-body cap: `p` is the Jacobi map of the decoded `q` (dd_decoder §3.4), nothing after it, and `|qₖ| ≤ q_max`.
fn check_no_cap<R: Precision>(what: &dyn Fn() -> String, d: &Decoded<R>, p: &[[R; 2]; 3]) {
    let want = to_particles(&d.masses, d.momentum.p_rho, d.momentum.p_lambda);
    let same = want
        .iter()
        .flatten()
        .zip(p.iter().flatten())
        .all(|(&a, &b)| f(a).to_bits() == f(b).to_bits());
    assert!(
        same,
        "{}: the momenta {p:?} are not the Jacobi map {want:?} of q: a per-body cap or another step after it",
        what()
    );
    let q_max: f64 = R::Q_MAX.into();
    for q in d.momentum.q {
        let q: f64 = q.into();
        assert!(q.abs() <= q_max, "{}: |q| = {q} beyond q_max", what());
    }
}

/// What [`qmax`] saw: the samples with a momentum control past its clamp, and the interior samples with a body's
/// momentum above `q_max`, where a per-body cap would bite.
struct Qmax {
    past: usize,
    above: usize,
}

/// `z` with the configuration and mass controls on `[−6, 6]` and the momentum controls on `[−20, 20]`, across their
/// clamps at `|z| ≈ 13.8`; decoded at `R`, capped first where `cap` holds, then checked.
fn qmax<R: Precision>(cap: bool) -> Qmax {
    let mut rng = SplitMix::new(29);
    let mut seen = Qmax { past: 0, above: 0 };
    let q_max: f64 = R::Q_MAX.into();
    for i in 0..QMAX_SAMPLES {
        let z: [f64; 8] = std::array::from_fn(|k| {
            let span = if (2..6).contains(&k) { 20.0 } else { 6.0 };
            uniform(&mut rng, -span, span)
        });
        let d = decode_z(z.map(R::of));
        let x = if cap {
            narrow::<R>(&capped(&widen(&d.ic)))
        } else {
            d.ic
        };
        let what = || format!("f{} sample {i} at z {z:?}", bits::<R>());
        check_no_cap(&what, &d, &x.p);
        let t = trip::<Defaults, R>(&x);
        check_t2_sample(&what, &x, &t);
        let sides = Sides::of(margins(&widen(&x)));
        seen.past += usize::from(sides.past & 0b11_1100 != 0);
        let over = widen(&x).p.iter().any(|&v| norm(v) > q_max);
        seen.above += usize::from(sides.interior() && over);
    }
    seen
}

#[test]
fn t2_qmax_boundary() {
    for (seen, width) in [(qmax::<f64>(false), 64), (qmax::<f32>(false), 32)] {
        assert!(
            seen.past > 0 && seen.above > 0,
            "f{width}: {} samples past the momentum clamps and {} interior samples with a body above q_max: the \
             boundary is not exercised",
            seen.past,
            seen.above
        );
    }
}

negative_control!(
    t2_qmax_boundary,
    "a decode with the rejected per-body cap and CoM re-enforcement must fail the no-cap check",
    expected = "a per-body cap",
    {
        qmax::<f64>(true);
    }
);

// ── T3 (REQ-ENC-005, REQ-DEC-016) ───────────────────────────────────────────────────────────────────────────────

/// The T3 draws per precision.
const T3_SAMPLES: u32 = 2048;

/// A T3 draw: `x₀ = D(z)`, `z` on `[−4, 4]⁸` with every control inside its clamp, so `x₀` is the canonical
/// representative of its orbit with `λ̃_y` clear of the mirror's deadband; and a gauge `g` (dd_encode §3.3) with `s` on
/// `2^[−4, 4)`, `θ` on `[−π, π)`, parity with probability ½, the offset `a` on `s·[−2, 2)²` and the boost `u` on
/// `s^{−1/2}·[−2, 2)²`, each in proportion to the state it moves, so the input carries its state to rounding.
fn t3_draw(rng: &mut SplitMix) -> (Ic<f64>, Gauge) {
    for _ in 0..64 {
        let z: [f64; 8] = std::array::from_fn(|_| uniform(rng, -4.0, 4.0));
        let x0 = decode_z(z).ic;
        if Sides::of(margins(&x0)).interior() {
            let mut g = Gauge::random(rng, 2.0, 4.0);
            g.a = g.a.map(|a| a * g.s);
            g.u = g.u.map(|u| u / g.s.sqrt());
            return (x0, g);
        }
    }
    panic!("no interior z in 64 draws");
}

/// A state's invariants in its own CoM frame (f64): `L_z`, `E`, the shape-sphere point `n` and the orientation
/// `ρ × Λ`, `Λ = m₀(r₂ − r₀) + m₁(r₂ − r₁)`, whose sign is the parity the mirror reads (`kernel::canon`).
struct Invariants {
    lz: f64,
    e: f64,
    n: [f64; 3],
    orientation: f64,
}

fn invariants(x: &Ic<f64>) -> Invariants {
    let m = x.m;
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * x.r[i][k]).sum::<f64>());
    let total = [0, 1].map(|k| (0..3).map(|i| x.p[i][k]).sum::<f64>());
    let r = x.r.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let mut p = x.p;
    for (pi, mi) in p.iter_mut().zip(m) {
        pi[0] -= mi * total[0];
        pi[1] -= mi * total[1];
    }
    let lz = (0..3).map(|i| r[i][0] * p[i][1] - r[i][1] * p[i][0]).sum();
    let k: f64 = (0..3)
        .map(|i| (p[i][0] * p[i][0] + p[i][1] * p[i][1]) / (2.0 * m[i]))
        .sum();
    let u: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| -m[i] * m[j] / norm([r[j][0] - r[i][0], r[j][1] - r[i][1]]))
        .sum();
    let d = |a: [f64; 2], b: [f64; 2]| [b[0] - a[0], b[1] - a[1]];
    let (rho, d02, d12) = (d(r[0], r[1]), d(r[0], r[2]), d(r[1], r[2]));
    let big = [0, 1].map(|k| m[0] * d02[k] + m[1] * d12[k]);
    Invariants {
        lz,
        e: k + u,
        n: shape(r, m),
        orientation: rho[0] * big[1] - rho[1] * big[0],
    }
}

/// The first-order changes to `L_z`, `E` and `n` of a state within `δ` of `x` in `‖·‖_phys` (Part 4: `|Δmᵢ| ≤ δ`,
/// `‖Δrᵢ‖ ≤ δR`, `‖Δpᵢ‖ ≤ δP`, `R` and `P` `x`'s extents floored at 1), `x` on the section (CoM at the origin, `I = 1`):
/// - `|ΔL_z| ≤ δ·Σᵢ (R‖pᵢ‖ + P‖rᵢ‖)`;
/// - `|ΔE| ≤ δ·[Σᵢ (P‖pᵢ‖/mᵢ + ‖pᵢ‖²/(2mᵢ²)) + Σᵢ<ⱼ (2R·mᵢmⱼ/rᵢⱼ² + (mᵢ + mⱼ)/rᵢⱼ)]`;
/// - `‖Δn‖ ≤ 16δR`: `n`'s components are quadratic in `(ρ̃, λ̃)` over `I`, `‖ρ̃‖, ‖λ̃‖ ≤ 1`, each Jacobi vector moving by
///   at most `2δR` times a reduced mass's root, at most ½, and `I` by at most `4δR`.
fn bounds(x: &Ic<f64>, delta: f64) -> (f64, f64, f64) {
    let extent = |v: &[[f64; 2]; 3]| v.iter().map(|&w| norm(w)).fold(1.0f64, f64::max);
    let (big_r, big_p) = (extent(&x.r), extent(&x.p));
    let m = x.m;
    let (np, nr) = (x.p.map(norm), x.r.map(norm));
    let tol_l = delta * (0..3).map(|i| big_r * np[i] + big_p * nr[i]).sum::<f64>();
    let kinetic: f64 = (0..3)
        .map(|i| big_p * np[i] / m[i] + np[i] * np[i] / (2.0 * m[i] * m[i]))
        .sum();
    let potential: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(i, j)| {
            let rij = norm([x.r[j][0] - x.r[i][0], x.r[j][1] - x.r[i][1]]);
            2.0 * big_r * m[i] * m[j] / (rij * rij) + (m[i] + m[j]) / rij
        })
        .sum();
    (tol_l, delta * (kinetic + potential), 16.0 * delta * big_r)
}

/// What a T3 encode returned: `z` and the step-0 and step-3 values the check reads.
#[derive(Clone, Copy)]
struct Out<R> {
    z: [R; 8],
    mirrored: bool,
    lambda: R,
}

/// The kernel's encode.
fn full<R: Precision>(x: &Ic<R>) -> Out<R> {
    let e = encode::<Defaults, R>(x);
    Out {
        z: e.z.z,
        mirrored: e.report.mirrored,
        lambda: e.report.lambda,
    }
}

/// The classic bug (Part 1): `C` with the mirror applied to the positions only, the momenta left unreflected.
#[cfg(feature = "controls")]
fn half_mirror<R: Precision>(x: &Ic<R>) -> Out<R> {
    let mut c = canonicalise(x);
    if c.mirrored {
        for p in c.ic.p.iter_mut() {
            p[1] = -p[1];
        }
    }
    Out {
        z: blocks::invert::<Defaults, R>(&c.ic).z,
        mirrored: c.mirrored,
        lambda: c.lambda,
    }
}

/// `C` with the rotation `R(−φ)` applied to the positions only: the momenta, mirrored as `C` mirrors them, left in the
/// input's orientation, `R(−φ)` undone on them as `Π·R(φ)·Π = R(−φ)` gives it.
#[cfg(feature = "controls")]
fn config_rotation<R: Precision>(x: &Ic<R>) -> Out<R> {
    let mut c = canonicalise(x);
    let phi: f64 = c.phi.into();
    let (s, k) = phi.sin_cos();
    for p in c.ic.p.iter_mut() {
        let v: [f64; 2] = [p[0].into(), p[1].into()];
        *p = [R::of(k * v[0] + s * v[1]), R::of(k * v[1] - s * v[0])];
    }
    Out {
        z: blocks::invert::<Defaults, R>(&c.ic).z,
        mirrored: c.mirrored,
        lambda: c.lambda,
    }
}

/// T3 (Part 8) on the input `x` at `R`, `out` its encode, `reference` the canonical state `D(E(x))` must equal: first the
/// half-mirror catch, `L_z = ±L_z,in·λ^{1/2}` with the sign `lookup_mirrored` gives (Part 1: `L_canon =
/// L_in/I_in^{1/4}`); then `lookup_mirrored` is the input's parity, `E = E_in/λ` (`E_canon = √I_in·E_in`), the masses,
/// the shape `n` with its `w` reflected by the mirror, and `‖D(E(x)) − reference‖_phys ≤ ε_phys`.
fn check_t3<R: Precision>(what: &dyn Fn() -> String, x: &Ic<R>, reference: &Ic<f64>, out: &Out<R>) {
    let back = widen(&decode_z(out.z).ic);
    let input = widen(x);
    let (have, got) = (invariants(&input), invariants(&back));
    let eps = eps_phys::<R>();
    let lambda: f64 = out.lambda.into();
    let (tol_l, tol_e, tol_n) = bounds(&back, eps);
    let sign = if out.mirrored { -1.0 } else { 1.0 };
    let want_l = sign * have.lz * lambda.sqrt();
    assert!(
        (got.lz - want_l).abs() <= tol_l,
        "{}: L_z {:.9e} is not {want_l:.9e}, the input's {:.9e} × λ^½ signed by lookup_mirrored = {} (within {tol_l:.3e})",
        what(),
        got.lz,
        have.lz,
        out.mirrored
    );
    assert_eq!(
        out.mirrored,
        have.orientation < 0.0,
        "{}: lookup_mirrored on an input of orientation {:.3e}",
        what(),
        have.orientation
    );
    let want_e = have.e / lambda;
    assert!(
        (got.e - want_e).abs() <= tol_e,
        "{}: E {:.9e} is not the input's E_in/λ = {want_e:.9e} (within {tol_e:.3e})",
        what(),
        got.e
    );
    for i in 0..3 {
        assert!(
            (back.m[i] - input.m[i]).abs() <= eps,
            "{}: mass {i} is {:e}, the input's {:e}",
            what(),
            back.m[i],
            input.m[i]
        );
    }
    let want_n = [have.n[0], have.n[1], sign * have.n[2]];
    let dn = (0..3)
        .map(|k| (got.n[k] - want_n[k]).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!(
        dn <= tol_n,
        "{}: the shape {:?} is {dn:.3e} from the input's {want_n:?}, reflected by the mirror (within {tol_n:.3e})",
        what(),
        got.n
    );
    let r = phys(reference, &back);
    assert!(
        r <= eps,
        "{}: D(E(x)) is {r:.3e} from its canonical representative in ‖·‖_phys, beyond ε_phys {eps:.3e}",
        what()
    );
}

/// An encode, as T3 runs it.
type EncodeFn<R> = fn(&Ic<R>) -> Out<R>;

/// T3 over random `g·x₀` at `R`, `D(E(x))` against `x₀` (Part 1: `C(g·x₀) = x₀`, the orbit's representative), with
/// `enc` as the encode; the mirrored samples' count.
fn t3_gauge<R: Precision>(enc: EncodeFn<R>) -> usize {
    let mut rng = SplitMix::new(53);
    let mut mirrored = 0;
    for i in 0..T3_SAMPLES {
        let (x0, g) = t3_draw(&mut rng);
        let x = narrow::<R>(&g.act(&x0));
        let out = enc(&x);
        let what = || format!("f{} sample {i}, gauge {g:?}", bits::<R>());
        check_t3(&what, &x, &x0, &out);
        mirrored += usize::from(out.mirrored);
    }
    mirrored
}

#[test]
fn t3_half_mirror() {
    for (n, width) in [
        (t3_gauge::<f64>(full::<f64>), 64),
        (t3_gauge::<f32>(full::<f32>), 32),
    ] {
        assert!(
            n > 0 && n < T3_SAMPLES as usize,
            "f{width}: {n} of {T3_SAMPLES} samples mirrored: parity is not exercised both ways"
        );
    }
}

negative_control!(
    t3_half_mirror,
    "a configuration-only mirror preserves the shape but not L_z, so it must fail the L_z check",
    expected = "L_z",
    {
        t3_gauge::<f64>(half_mirror::<f64>);
    }
);

/// The coincident inner pair, `ρ = (±0, ±0)` by every sign of its zeros (R-437): `r₀ = (+0, +0)` and `r₁` signed zeros,
/// so `ρ = r₁ − r₀` keeps `r₁`'s signs; body 2 on either side of the pair's axis, and nonzero momenta.
fn pole_inputs() -> Vec<Ic<f64>> {
    let mut out = Vec::new();
    for sx in [0.0f64, -0.0] {
        for sy in [0.0f64, -0.0] {
            for r2 in [[1.1, 0.6], [-0.8, -0.5]] {
                out.push(Ic {
                    m: [0.25, 0.35, 0.4],
                    r: [[0.0, 0.0], [sx, sy], r2],
                    p: [[0.3, -0.2], [-0.1, 0.4], [0.05, 0.15]],
                });
            }
        }
    }
    out
}

/// `x` through `C`'s steps 1a, 0, 1b and the mirror `C` decided, with no rotation, in f64: what `C(x)` is when its
/// rotation is the identity.
fn unrotated(x: &Ic<f64>, mirrored: bool) -> Ic<f64> {
    let m = x.m;
    let com = [0, 1].map(|k| (0..3).map(|i| m[i] * x.r[i][k]).sum::<f64>());
    let r = x.r.map(|v| [v[0] - com[0], v[1] - com[1]]);
    let i_in: f64 = (0..3)
        .map(|i| m[i] * (r[i][0] * r[i][0] + r[i][1] * r[i][1]))
        .sum();
    let (d, q) = (i_in.sqrt(), i_in.sqrt().sqrt());
    let r = r.map(|v| [v[0] / d, v[1] / d]);
    let mut p = x.p.map(|v| [v[0] * q, v[1] * q]);
    let total = [0, 1].map(|k| (0..3).map(|i| p[i][k]).sum::<f64>());
    for (pi, mi) in p.iter_mut().zip(m) {
        pi[0] -= mi * total[0];
        pi[1] -= mi * total[1];
    }
    let flip = |v: [f64; 2]| if mirrored { [v[0], -v[1]] } else { v };
    Ic {
        m,
        r: r.map(flip),
        p: p.map(flip),
    }
}

/// At `ρ = 0` (R-437; dd_decoder §3.3): `C` raises `lookup_clamped` and takes `φ = +0`, so its rotation is the identity
/// and `C(x)` is [`unrotated`] within `ε_phys`; encode gives a finite `z` and `lookup_clamped`, the same bits twice.
fn check_pole<R: Precision>(x: &Ic<f64>) {
    let xr = narrow::<R>(x);
    let what = format!("f{} ρ = 0 input {x:?}", bits::<R>());
    let c = canonicalise(&xr);
    let phi: f64 = c.phi.into();
    assert!(c.clamped, "{what}: no lookup_clamped at ρ = 0");
    assert_eq!(phi.to_bits(), 0, "{what}: φ is {phi:e}, not +0");
    let r = phys(&unrotated(&widen(&xr), c.mirrored), &widen(&c.ic));
    let eps = eps_phys::<R>();
    assert!(
        r <= eps,
        "{what}: C(x) is {r:.3e} from x unrotated: the full-state rotation at ρ = 0 is not the identity"
    );
    let (e1, e2) = (encode::<Defaults, R>(&xr), encode::<Defaults, R>(&xr));
    assert!(
        e1.report.lookup_clamped(),
        "{what}: encode raises no lookup_clamped"
    );
    let z1: Vec<u64> = e1.z.z.iter().map(|&z| f(z).to_bits()).collect();
    let z2: Vec<u64> = e2.z.z.iter().map(|&z| f(z).to_bits()).collect();
    assert_eq!(z1, z2, "{what}: encode is not deterministic");
    let back = widen(&decode_z(e1.z.z).ic);
    assert!(
        e1.z.z.iter().all(|&z| f(z).is_finite())
            && back
                .r
                .iter()
                .chain(&back.p)
                .flatten()
                .all(|v| v.is_finite()),
        "{what}: a non-finite z {:?} or decode",
        e1.z.z
    );
}

/// T3 over random-gauge `x` at `R` with `enc` as the encode, `D(E(x))` against `C(x)`, the kernel's canonicaliser at
/// `R`.
fn t3_full<R: Precision>(enc: EncodeFn<R>) {
    let mut rng = SplitMix::new(71);
    for i in 0..T3_SAMPLES {
        let (x0, g) = t3_draw(&mut rng);
        let x = narrow::<R>(&g.act(&x0));
        let c = widen(&canonicalise(&x).ic);
        let what = || format!("f{} sample {i}, gauge {g:?}", bits::<R>());
        check_t3(&what, &x, &c, &enc(&x));
    }
}

#[test]
fn t3_full_state_rigid_ops() {
    t3_full::<f64>(full::<f64>);
    t3_full::<f32>(full::<f32>);
    for x in pole_inputs() {
        check_pole::<f64>(&x);
        check_pole::<f32>(&x);
    }
}

negative_control!(
    t3_full_state_rigid_ops,
    "a configuration-only mirror must fail the L_z check",
    expected = "L_z",
    t3_full::<f64>(half_mirror::<f64>)
);

mod config_only_rotation {
    use super::*;

    negative_control!(
        t3_full_state_rigid_ops,
        "a configuration-only rotation, the momenta left in the input's orientation, must fail the L_z check",
        expected = "L_z",
        t3_full::<f32>(config_rotation::<f32>)
    );
}

// ── encode_reuses_decode (REQ-ENC-001) ──────────────────────────────────────────────────────────────────────────

/// The random states per precision.
const REUSE_SAMPLES: u32 = 4096;

/// A random physical state: masses from weights on `[0.05, 1]`, normalised; positions on `[−3, 3]²`; momenta on
/// `[−1, 1]²`, with their total and the CoM wherever they fall.
fn random_state(rng: &mut SplitMix) -> Ic<f64> {
    let w: [f64; 3] = std::array::from_fn(|_| uniform(rng, 0.05, 1.0));
    let total: f64 = w.iter().sum();
    Ic {
        m: w.map(|x| x / total),
        r: std::array::from_fn(|_| [uniform(rng, -3.0, 3.0), uniform(rng, -3.0, 3.0)]),
        p: std::array::from_fn(|_| [uniform(rng, -1.0, 1.0), uniform(rng, -1.0, 1.0)]),
    }
}

/// `decode(encode(x)) = canonicalise(x)` within `ε_phys`, `z` the encode's.
fn check_reuse<R: Precision>(what: &dyn Fn() -> String, x: &Ic<R>, z: [R; 8]) {
    let c = widen(&canonicalise(x).ic);
    let back = widen(&decode_z(z).ic);
    let r = phys(&c, &back);
    let eps = eps_phys::<R>();
    assert!(
        r <= eps,
        "{}: decode(encode(x)) is {r:.3e} from canonicalise(x) in ‖·‖_phys, beyond ε_phys {eps:.3e}",
        what()
    );
}

/// The random states at `R` whose canonical form is interior, checked; their count. With `skip_c`, the block inverses
/// are run on `x` itself, `C` skipped.
fn reuse<R: Precision>(skip_c: bool) -> usize {
    let mut rng = SplitMix::new(97);
    let mut interior = 0;
    for i in 0..REUSE_SAMPLES {
        let x = narrow::<R>(&random_state(&mut rng));
        if !Sides::of(margins(&widen(&canonicalise(&x).ic))).interior() {
            continue;
        }
        interior += 1;
        let z = if skip_c {
            blocks::invert::<Defaults, R>(&x).z
        } else {
            encode::<Defaults, R>(&x).z.z
        };
        check_reuse(&|| format!("f{} state {i}: {x:?}", bits::<R>()), &x, z);
    }
    interior
}

#[test]
fn encode_reuses_decode() {
    for (n, width) in [(reuse::<f64>(false), 64), (reuse::<f32>(false), 32)] {
        assert!(
            n >= REUSE_SAMPLES as usize / 4,
            "f{width}: only {n} of {REUSE_SAMPLES} states interior"
        );
    }
}

negative_control!(
    encode_reuses_decode,
    "an encode that skips canonicalisation, inverting the raw state, must fail the check",
    expected = "from canonicalise(x)",
    {
        reuse::<f64>(true);
    }
);

// ── encode_report (REQ-ENC-018) ─────────────────────────────────────────────────────────────────────────────────

/// The report on `g·x₀` for an interior `x₀` (Parts 1, 2 and 6): `λ = I_in^{−1/2} = 1/s` (`I_in = s²`) within
/// `ε_phys`, relative; `lookup_mirrored` is `g`'s parity (T3); no clamp (T2's interior); no coincident pair.
fn check_report<R: Precision>(what: &dyn Fn() -> String, rep: &Report<R>, g: &Gauge) {
    let lambda: f64 = rep.lambda.into();
    let eps = eps_phys::<R>();
    assert!(
        (lambda * g.s - 1.0).abs() <= eps,
        "{}: lookup_rescaled's λ {lambda:e} is not 1/s = {:e}",
        what(),
        1.0 / g.s
    );
    assert_eq!(
        rep.mirrored,
        g.mirror,
        "{}: lookup_mirrored against the gauge's parity",
        what()
    );
    assert!(
        !rep.lookup_clamped(),
        "{}: lookup_clamped on an interior state",
        what()
    );
}

/// The report over random `g·x₀` at `R`, `alter` applied to each report before it is checked; and the orbit's decode
/// `D(E(g·x₀))` within `ε_phys` of `x₀` (T1 and T3 in physical units).
fn reports<R: Precision>(alter: fn(&mut Report<R>)) {
    let mut rng = SplitMix::new(113);
    for i in 0..T3_SAMPLES {
        let (x0, g) = t3_draw(&mut rng);
        let x = narrow::<R>(&g.act(&x0));
        let mut e = encode::<Defaults, R>(&x);
        alter(&mut e.report);
        let what = || format!("f{} sample {i}, gauge {g:?}", bits::<R>());
        check_report(&what, &e.report, &g);
        let r = phys(&x0, &widen(&decode_z(e.z.z).ic));
        assert!(
            r <= eps_phys::<R>(),
            "{}: D(E(g·x₀)) is {r:.3e} from x₀",
            what()
        );
    }
}

/// The clamp notices: a momentum control past its clamp sets its bit, and the coincident inner pair sets
/// `lookup_clamped` through `coincident`.
fn clamp_notices<R: Precision>() {
    let mut z = [0.3, -0.2, 0.1, 0.4, -0.3, 0.2, 0.5, -0.4];
    z[2] = 20.0;
    let e = encode::<Defaults, R>(&decode_z(z.map(R::of)).ic);
    assert_eq!(
        e.report.clamped_controls,
        1 << 2,
        "f{}: z_q0 = 20 sets the clamp bits {:08b}",
        bits::<R>(),
        e.report.clamped_controls
    );
    assert!(e.report.lookup_clamped());
    for x in pole_inputs() {
        let e = encode::<Defaults, R>(&narrow::<R>(&x));
        assert!(
            e.report.coincident && e.report.lookup_clamped(),
            "f{}: the coincident pair is not reported",
            bits::<R>()
        );
    }
}

fn keep<R>(_: &mut Report<R>) {}

#[test]
fn encode_report() {
    reports::<f64>(keep::<f64>);
    reports::<f32>(keep::<f32>);
    clamp_notices::<f64>();
    clamp_notices::<f32>();
}

negative_control!(
    encode_report,
    "a report that drops the mirror notice must fail the parity check",
    expected = "lookup_mirrored against the gauge's parity",
    reports::<f64>(|r| r.mirrored = false)
);

// ── conditioning_physical_space (REQ-VAL-013) ───────────────────────────────────────────────────────────────────

/// The near-clamp draws per precision.
const COND_SAMPLES: u32 = 1024;

/// What a near-clamp round trip is held to: Part 4's physical tolerance, or the naive `z` one it forbids.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Criterion {
    Physical,
    #[cfg(feature = "controls")]
    Latent,
}

/// `z` with each momentum control at `±11.5 ± 0.05`, `s(1 − s) ≈ 10⁻⁵`, inside its clamp at `|z| ≈ 13.8` (margin
/// about 10 of its `ε`), and the other controls on `[−2, 2]`: `d logit/ds ≈ 10⁵` (Part 4).
fn near_clamp(rng: &mut SplitMix) -> [f64; 8] {
    std::array::from_fn(|k| {
        if (2..6).contains(&k) {
            let sign = if rng.next_u64() & 1 == 1 { 1.0 } else { -1.0 };
            sign * uniform(rng, 11.45, 11.55)
        } else {
            uniform(rng, -2.0, 2.0)
        }
    })
}

/// The near-clamp round trips at `R` held to `criterion`; each sample interior by the oracle and unclamped. Returns the
/// median of `|Δz|∞` over the physical residual (floored at `R`'s epsilon), and how many samples' `|Δz|∞` exceed
/// `ε_phys`, the naive tolerance.
fn conditioning<R: Precision>(criterion: Criterion) -> (f64, usize) {
    let mut rng = SplitMix::new(131);
    let eps = eps_phys::<R>();
    let mut ratios = Vec::new();
    let mut over = 0;
    for i in 0..COND_SAMPLES {
        let z = near_clamp(&mut rng).map(R::of);
        let x = decode_z(z).ic;
        let t = trip::<Defaults, R>(&x);
        let what = format!("f{} sample {i} at z {z:?}", bits::<R>());
        assert!(
            Sides::of(margins(&widen(&x))).interior() && !t.encoded.report.lookup_clamped(),
            "{what}: the near-clamp sample is not inside its clamps"
        );
        let dz = (0..8)
            .map(|k| (f(t.encoded.z.z[k]) - f(z[k])).abs())
            .fold(0.0, f64::max);
        match criterion {
            Criterion::Physical => assert!(
                t.residual <= eps,
                "{what}: the physical residual {:.3e} exceeds ε_phys {eps:.3e}",
                t.residual
            ),
            #[cfg(feature = "controls")]
            Criterion::Latent => assert!(
                dz <= eps,
                "{what}: a naive z-tolerance fails: |Δz| = {dz:.3e} exceeds ε_phys {eps:.3e}"
            ),
        }
        over += usize::from(dz > eps);
        ratios.push(dz / t.residual.max(R::EPS));
    }
    ratios.sort_by(f64::total_cmp);
    (ratios[ratios.len() / 2], over)
}

#[test]
fn conditioning_physical_space() {
    for ((ratio, over), width) in [
        (conditioning::<f64>(Criterion::Physical), 64),
        (conditioning::<f32>(Criterion::Physical), 32),
    ] {
        assert!(
            ratio >= 1e4 && over >= COND_SAMPLES as usize / 2,
            "f{width}: the z residual is {ratio:.3e}× the physical one at the median and exceeds ε_phys on {over} of \
             {COND_SAMPLES}: not the ~10⁵ conditioning Part 4 names"
        );
    }
}

negative_control!(
    conditioning_physical_space,
    "the naive z-space tolerance Part 4 forbids must fail on the near-clamp samples",
    expected = "a naive z-tolerance fails",
    {
        conditioning::<f64>(Criterion::Latent);
    }
);

// ── encode_golden_pairs (dd_encode §5) ──────────────────────────────────────────────────────────────────────────

/// One golden pair, `fixtures/states/encode_golden.json`.
struct Pair {
    name: String,
    state: Ic<f64>,
    z: [f64; 8],
    nu: Option<f64>,
}

fn pairs() -> Vec<Pair> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/states/encode_golden.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let json: serde_json::Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let num = |v: &serde_json::Value| v.as_f64().expect("a number");
    let pair2 = |v: &serde_json::Value| [num(&v[0]), num(&v[1])];
    json["pairs"]
        .as_array()
        .expect("pairs")
        .iter()
        .map(|p| Pair {
            name: p["name"].as_str().expect("a name").to_owned(),
            state: Ic {
                m: std::array::from_fn(|i| num(&p["m"][i])),
                r: std::array::from_fn(|i| pair2(&p["r"][i])),
                p: std::array::from_fn(|i| pair2(&p["p"][i])),
            },
            z: std::array::from_fn(|k| num(&p["z"][k])),
            nu: p["nu"].as_f64(),
        })
        .collect()
}

/// One pair at `R`: `E(x)` decodes to `D(z)`, the pair's `z` (so `E(x) = z`, asserted in physical units, Part 4);
/// `D(z)` is `C(x)`; and at a pair with `ν`, `D(E(x))` is the Burrau system at that `ν` (the Euclid plane's decode,
/// rest, masses `(c, b, a)/(a + b + c)`).
fn check_pair<R: Precision>(pair: &Pair, z: [f64; 8]) {
    let what = format!("f{} pair {}", bits::<R>(), pair.name);
    let eps = eps_phys::<R>();
    let x = narrow::<R>(&pair.state);
    let at_z = widen(&decode_z(z.map(R::of)).ic);
    let back = widen(&decode_z(encode::<Defaults, R>(&x).z.z).ic);
    let r = phys(&at_z, &back);
    assert!(
        r <= eps,
        "{what}: D(E(x)) is {r:.3e} from D(z) of the pair's z: E(x) is not z"
    );
    let r = phys(&widen(&canonicalise(&x).ic), &at_z);
    assert!(r <= eps, "{what}: D(z) is {r:.3e} from C(x)");
    if let Some(nu) = pair.nu {
        let plane = EuclidPlane {
            m_range: [1.0, 2.0],
            nu_range: [nu - 0.25, nu + 0.25],
        };
        let burrau = plane.decode::<Defaults, f64>(0.5, 0.5).ic;
        let r = phys(&burrau, &back);
        assert!(
            r <= eps,
            "{what}: D(E(x)) is {r:.3e} from the Burrau system at ν = {nu}"
        );
    }
}

#[test]
fn encode_golden_pairs() {
    let all = pairs();
    assert_eq!(
        all.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["z0", "burrau_345"],
        "the golden pairs"
    );
    for p in &all {
        check_pair::<f64>(p, p.z);
        check_pair::<f32>(p, p.z);
    }
}

negative_control!(
    encode_golden_pairs,
    "the Burrau pair's z with z_β moved by 10⁻⁶ is not E(x), so the pair check must fail",
    expected = "E(x) is not z",
    {
        let all = pairs();
        let p = all.iter().find(|p| p.name == "burrau_345").expect("listed");
        let mut z = p.z;
        z[1] += 1e-6;
        check_pair::<f64>(p, z);
    }
);
