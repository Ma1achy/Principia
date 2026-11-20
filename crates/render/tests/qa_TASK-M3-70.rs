//! QA's tests for TASK-M3-70, written from REQ-TOOL-227 (R-928, R-929; render contract Part 5, "The neutral band"):
//! `symlog`'s neutral band on a current drift scales with the rounding itself, `k'·2⁻²³·(K + |V|)` for the energy and
//! `k'·2⁻²³·S` for `L_z`, `S = Σ(|xᵢp_{y,i}| + |yᵢp_{x,i}|)`, with the CPU reference's `k'` (15 and 5), never with the
//! size of the reference; it is applied wherever the drift is read, the field ramp on `energy_drift` or `Lz_drift`
//! included, and `compact_symlog` itself is unchanged; `lin` and `log` keep no band, a non-finite operand passes first,
//! and `dbg_suspect` is handed the drift.
//!
//! **Domain** (pitfalls §10): fresh samples only, each reference its own state's `H(r, p)` or `L_z(r, p)` in f64 rounded
//! once to f32, a correctly rounded CPU reference, as the contract's `k'` of 15 and 5 assumes. The near-zero states are
//! built here, independently of the implementation's tests: a random state's momenta scaled so that `K = |V|` (E ≈ 0),
//! a random state with body 2's `p_y` solved for `L_z = 0`, and dd_integrator test 2's figure-eight at its start.

use render::colour::field_ramp::{
    Compaction, FieldRamp, Lut, Map, Ramp, Reads, ScalarField, Shown as RampShown,
};
use render::present::{self, Rgb, DRIFT_LIN, DRIFT_LOG, DRIFT_SYMLOG};
use render::registry;
use validation::golden_scene::{row_scene, Colouring, DebugCase, Look, Scene};
use validation::gpu::GpuHarness;
use validation::negative_control;

type Vec2s = [[f32; 2]; 3];

/// One ULP's relative size at most, `2⁻²³`.
const ULP: f64 = 1.0 / 8_388_608.0;

/// R-928's `k'` for a CPU reference: 15 for the energy, 5 for `L_z`.
const K_E: f64 = 15.0;
const K_LZ: f64 = 5.0;

/// R-652's band before R-928, `k·2⁻²³` of `|reference|` with `k = 128`, the controls'.
#[cfg(feature = "controls")]
const K_RELATIVE: f64 = 128.0;

// ── states ───────────────────────────────────────────────────────────────────────────────────────────────────────────

/// A small deterministic generator (xorshift64*), so the sets are the same on every run.
struct Rng(u64);

impl Rng {
    fn unit(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let v = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d);
        (v >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[−a, a]`.
    fn sym(&mut self, a: f64) -> f64 {
        a * (2.0 * self.unit() - 1.0)
    }
}

/// A random state, `(r, p, m)`: masses positive, summing to 1 in f64, positions in `[−1, 1]²` no two closer than 0.1,
/// momenta in `[−1, 1]²`.
fn random_state(g: &mut Rng) -> (Vec2s, Vec2s, [f32; 3]) {
    loop {
        let w = [0.2 + g.unit(), 0.2 + g.unit(), 0.2 + g.unit()];
        let total: f64 = w.iter().sum();
        let m = w.map(|x| (x / total) as f32);
        let r = [(); 3].map(|_| [g.sym(1.0) as f32, g.sym(1.0) as f32]);
        let p = [(); 3].map(|_| [g.sym(1.0) as f32, g.sym(1.0) as f32]);
        let far = [(0, 1), (0, 2), (1, 2)].iter().all(|&(a, b)| {
            let dx = f64::from(r[a][0]) - f64::from(r[b][0]);
            let dy = f64::from(r[a][1]) - f64::from(r[b][1]);
            dx.hypot(dy) > 0.1
        });
        if far {
            return (r, p, m);
        }
    }
}

/// `(K, |V|, H, L_z, S)` of the f32 state, in f64.
fn invariants(r: Vec2s, p: Vec2s, m: [f32; 3]) -> (f64, f64, f64, f64, f64) {
    let f = f64::from;
    let k: f64 = (0..3)
        .map(|j| (f(p[j][0]).powi(2) + f(p[j][1]).powi(2)) / (2.0 * f(m[j])))
        .sum();
    let v: f64 = [(0, 1), (0, 2), (1, 2)]
        .iter()
        .map(|&(a, b)| f(m[a]) * f(m[b]) / (f(r[a][0]) - f(r[b][0])).hypot(f(r[a][1]) - f(r[b][1])))
        .sum();
    let lz: f64 = (0..3)
        .map(|j| f(r[j][0]) * f(p[j][1]) - f(r[j][1]) * f(p[j][0]))
        .sum();
    let s: f64 = (0..3)
        .map(|j| (f(r[j][0]) * f(p[j][1])).abs() + (f(r[j][1]) * f(p[j][0])).abs())
        .sum();
    (k, v, k - v, lz, s)
}

/// `state`'s momenta scaled so that `K = |V|`: a near-parabolic state, `E` 0 but for rounding.
fn parabolic((r, p, m): (Vec2s, Vec2s, [f32; 3])) -> (Vec2s, Vec2s, [f32; 3]) {
    let (k, v, ..) = invariants(r, p, m);
    let scale = (v / k).sqrt();
    (r, p.map(|q| q.map(|c| (f64::from(c) * scale) as f32)), m)
}

/// `state` with body 2's `p_y` solved for `L_z = 0` in f64, then rounded: `L_z` 0 but for rounding, `S` not.
fn zero_lz((r, mut p, m): (Vec2s, Vec2s, [f32; 3])) -> Option<(Vec2s, Vec2s, [f32; 3])> {
    let f = f64::from;
    if f(r[2][0]).abs() < 0.1 {
        return None;
    }
    let l01: f64 = (0..2)
        .map(|j| f(r[j][0]) * f(p[j][1]) - f(r[j][1]) * f(p[j][0]))
        .sum();
    p[2][1] = ((f(r[2][1]) * f(p[2][0]) - l01) / f(r[2][0])) as f32;
    Some((r, p, m))
}

/// dd_integrator test 2's figure-eight at its start.
fn figure_eight() -> (Vec2s, Vec2s, [f32; 3]) {
    let ic = validation::drift::figure_eight().ic;
    let f = |a: [[f64; 2]; 3]| a.map(|v| v.map(|c| c as f32));
    (f(ic.r), f(ic.p), ic.m.map(|c| c as f32))
}

/// The near-zero set: the figure-eight, then 24 near-parabolic and 24 zero-`L_z` random states.
fn near_zero_set() -> Vec<(Vec2s, Vec2s, [f32; 3])> {
    let mut g = Rng(0x9e37_79b9_7f4a_7c15);
    let mut out = vec![figure_eight()];
    while out.len() < 25 {
        out.push(parabolic(random_state(&mut g)));
    }
    while out.len() < 49 {
        if let Some(s) = zero_lz(random_state(&mut g)) {
            out.push(s);
        }
    }
    out
}

/// A generic set: 64 random states, no reference near zero by construction.
fn generic_set() -> Vec<(Vec2s, Vec2s, [f32; 3])> {
    let mut g = Rng(0x2545_f491_4f6c_dd1d);
    (0..64).map(|_| random_state(&mut g)).collect()
}

// ── the band's scale (R-928) ─────────────────────────────────────────────────────────────────────────────────────────

/// Each mirror band is `k'·2⁻²³` of its scale computed here in f64: within the f32 evaluation's own rounding, the CPU
/// reference's bound of render contract Part 5 (`K` 3 and `|V|` 4 ULP of the scale; `L_z` 2), plus the band's one
/// multiply's ½ and a ½ for second-order terms, so 5 and 3 ULP of the band. `k_e` and `k_lz` are the factors expected.
fn check_scale(k_e: f64, k_lz: f64) {
    for (r, p, m) in generic_set().into_iter().chain(near_zero_set()) {
        let (k, v, _, _, s) = invariants(r, p, m);
        let want_e = k_e * ULP * (k + v);
        let got_e = f64::from(present::drift_band_energy(r, p, m));
        assert!(
            (got_e - want_e).abs() <= 5.0 * ULP * want_e,
            "the energy band {got_e} is not {k_e}·2⁻²³·(K + |V|) = {want_e} at r {r:?} p {p:?} m {m:?}"
        );
        let want_l = k_lz * ULP * s;
        let got_l = f64::from(present::drift_band_lz(r, p));
        assert!(
            (got_l - want_l).abs() <= 3.0 * ULP * want_l,
            "the L_z band {got_l} is not {k_lz}·2⁻²³·S = {want_l} at r {r:?} p {p:?}"
        );
    }
}

#[test]
fn qa_tool227_band_is_k_prime_of_the_rounding_scale() {
    check_scale(K_E, K_LZ);
}

negative_control!(
    qa_tool227_band_is_k_prime_of_the_rounding_scale,
    "the GPU reference's k', 22 and 6, is not the CPU reference's band",
    expected = "is not 22",
    check_scale(22.0, 6.0)
);

/// The band's edges of scale: at rest (`K = 0`, `S = 0`) the energy band is `15·2⁻²³·|V|` and the `L_z` band empty,
/// where the drift is exact; and neither depends on the reference (the mirror takes none). `band_e` and `band_lz` are
/// the bands under test, `(r, p, m)` to the half-width.
fn check_rest(
    band_e: &dyn Fn(Vec2s, Vec2s, [f32; 3]) -> f32,
    band_lz: &dyn Fn(Vec2s, Vec2s) -> f32,
) {
    let (r, _, m) = random_state(&mut Rng(7));
    let p = [[0.0_f32; 2]; 3];
    let (_, v, ..) = invariants(r, p, m);
    let e = f64::from(band_e(r, p, m));
    assert!(
        (e - K_E * ULP * v).abs() <= 5.0 * ULP * e,
        "at rest the energy band {e} is not 15·2⁻²³·|V|"
    );
    assert!(e > 0.0, "the energy band is empty at rest");
    assert_eq!(band_lz(r, p), 0.0, "S = 0 and the band is not");
    // An exact drift of 0 on an empty band is drawn as 0.
    assert_eq!(present::dbg_drift_shown(0.0, DRIFT_SYMLOG, 0.0), 0.0);
}

#[test]
fn qa_tool227_band_at_rest_and_without_reference() {
    check_rest(&present::drift_band_energy, &present::drift_band_lz);
}

negative_control!(
    qa_tool227_band_at_rest_and_without_reference,
    "R-652's band, 128·2⁻²³ of |reference|, the reference H = −|V| at rest",
    expected = "at rest the energy band",
    check_rest(
        &|r, p, m| {
            let (.., h, _, _) = invariants(r, p, m);
            (K_RELATIVE * ULP * h.abs()) as f32
        },
        &present::drift_band_lz
    )
);

#[cfg(feature = "controls")]
mod qa_tool227_band_at_rest_and_without_reference_lz {
    use super::*;

    negative_control!(
        qa_tool227_band_at_rest_and_without_reference,
        "an `L_z` band floored at the energy's scale, not empty at S = 0",
        expected = "S = 0 and the band is not",
        check_rest(&present::drift_band_energy, &|r, p| {
            let m = [1.0 / 3.0_f32; 3];
            present::drift_band_lz(r, p).max(present::drift_band_energy(r, p, m) / 3.0)
        })
    );
}

// ── a fresh sample's drift is inside the band, references near zero included ────────────────────────────────────────

/// A fresh sample's current drifts as a fragment may compute them in f32, `H(r, p) − E_0` and `L_z(r, p) − Lz_0`, in
/// several association orders and with a fused multiply-add, the CPU reference `E_0`, `Lz_0` rounded once.
fn fresh_drifts(r: Vec2s, p: Vec2s, m: [f32; 3]) -> (Vec<f32>, Vec<f32>, f32, f32) {
    let (.., h, lz, _) = invariants(r, p, m);
    let (e_0, lz_0) = (h as f32, lz as f32);
    let kin = |j: usize| (p[j][0] * p[j][0] + p[j][1] * p[j][1]) / (2.0 * m[j]);
    let pot = |a: usize, b: usize| {
        let d = [r[a][0] - r[b][0], r[a][1] - r[b][1]];
        m[a] * m[b] / (d[0] * d[0] + d[1] * d[1]).sqrt()
    };
    let (k, v) = ([kin(0), kin(1), kin(2)], [pot(0, 1), pot(0, 2), pot(1, 2)]);
    let hs = [
        (k[0] + k[1] + k[2]) - (v[0] + v[1] + v[2]),
        k[0] + k[1] + k[2] - v[0] - v[1] - v[2],
        -v[2] - v[1] - v[0] + k[2] + k[1] + k[0],
        (k[0] - v[0]) + (k[1] - v[1]) + (k[2] - v[2]),
    ];
    let term = |j: usize| r[j][0] * p[j][1] - r[j][1] * p[j][0];
    let fused = |j: usize| r[j][0].mul_add(p[j][1], -(r[j][1] * p[j][0]));
    let ls = [
        term(0) + term(1) + term(2),
        term(2) + term(1) + term(0),
        fused(0) + fused(1) + fused(2),
        r[0][0] * p[0][1] + r[1][0] * p[1][1] + r[2][0] * p[2][1]
            - (r[0][1] * p[0][0] + r[1][1] * p[1][0] + r[2][1] * p[2][0]),
    ];
    (
        hs.iter().map(|h| h - e_0).collect(),
        ls.iter().map(|l| l - lz_0).collect(),
        e_0,
        lz_0,
    )
}

/// Every fresh drift of every state of `set` is drawn as 0 under `symlog` with the band `band(r, p, m, reference)`
/// (`energy`) as the mirror's `dbg_drift_shown` draws it.
fn check_fresh(
    set: &[(Vec2s, Vec2s, [f32; 3])],
    band_e: &dyn Fn(Vec2s, Vec2s, [f32; 3], f32) -> f32,
    band_l: &dyn Fn(Vec2s, Vec2s, f32) -> f32,
) {
    for (n, &(r, p, m)) in set.iter().enumerate() {
        let (des, dls, e_0, lz_0) = fresh_drifts(r, p, m);
        let (be, bl) = (band_e(r, p, m, e_0), band_l(r, p, lz_0));
        for x in des {
            assert_eq!(
                present::dbg_drift_shown(x, DRIFT_SYMLOG, be),
                0.0,
                "state {n}: a fresh energy drift {x:e} is not inside the band {be:e} (E_0 {e_0:e})"
            );
        }
        for x in dls {
            assert_eq!(
                present::dbg_drift_shown(x, DRIFT_SYMLOG, bl),
                0.0,
                "state {n}: a fresh L_z drift {x:e} is not inside the band {bl:e} (Lz_0 {lz_0:e})"
            );
        }
    }
}

/// The near-zero set's references are near zero against their scales: each near-parabolic `|E_0|` and each zero-`L_z`
/// `|Lz_0|` under 2⁻²⁰ of its band's scale, where a band of `128·2⁻²³` of the reference is under 2⁻¹³ of the rounding.
fn check_near_zero(set: &[(Vec2s, Vec2s, [f32; 3])]) {
    let (.., lz8, s8) = invariants(set[0].0, set[0].1, set[0].2);
    assert!(
        lz8.abs() < s8 / 1_048_576.0,
        "the figure-eight's L_z {lz8} is not near zero"
    );
    for (r, p, m) in &set[1..25] {
        let (k, v, h, ..) = invariants(*r, *p, *m);
        assert!(
            h.abs() < (k + v) / 1_048_576.0,
            "a near-parabolic E {h} is not near zero"
        );
    }
    for (r, p, m) in &set[25..] {
        let (.., lz, s) = invariants(*r, *p, *m);
        assert!(
            lz.abs() < s / 1_048_576.0,
            "a zero-L_z state's L_z {lz} is not near zero"
        );
    }
}

#[test]
fn qa_tool227_fresh_drift_inside_the_band_at_a_zero_reference() {
    let near = near_zero_set();
    check_near_zero(&near);
    let set: Vec<_> = generic_set().into_iter().chain(near).collect();
    check_fresh(
        &set,
        &|r, p, m, _| present::drift_band_energy(r, p, m),
        &|r, p, _| present::drift_band_lz(r, p),
    );
}

negative_control!(
    qa_tool227_fresh_drift_inside_the_band_at_a_zero_reference,
    "the band relative to |E_0| and |Lz_0| (k = 128): the near-zero states' rounding shows",
    expected = "is not inside the band",
    check_fresh(
        &near_zero_set(),
        &|_, _, _, e_0| (K_RELATIVE * ULP) as f32 * e_0.abs(),
        &|_, _, lz_0| (K_RELATIVE * ULP) as f32 * lz_0.abs(),
    )
);

// ── the band's edge, its compactions and its non-finite operands ────────────────────────────────────────────────────

/// `shown` (the mirror's `dbg_drift_shown`) draws `|x| ≤ band` as 0 under `symlog`, the edge included, and the next
/// f32 past the edge as itself; `lin` and `log` pass a drift inside the band; a non-finite `x` passes through, and a
/// non-finite band passes `x`.
fn check_edges(shown: fn(f32, u32, f32) -> f32) {
    for (r, p, m) in near_zero_set() {
        for band in [
            present::drift_band_energy(r, p, m),
            present::drift_band_lz(r, p),
        ] {
            let past = f32::from_bits(band.to_bits() + 1);
            assert_eq!(
                shown(band, DRIFT_SYMLOG, band),
                0.0,
                "the band's edge {band:e} is not drawn as 0"
            );
            assert_eq!(
                shown(-band, DRIFT_SYMLOG, band),
                0.0,
                "the band's edge −{band:e} is not drawn as 0"
            );
            assert_eq!(
                shown(past, DRIFT_SYMLOG, band),
                past,
                "a drift just past the band moved"
            );
            assert_eq!(
                shown(-past, DRIFT_SYMLOG, band),
                -past,
                "a drift just past the band moved"
            );
            let inside = band / 2.0;
            assert_eq!(shown(inside, DRIFT_LIN, band), inside, "`lin` has a band");
            assert_eq!(shown(inside, DRIFT_LOG, band), inside, "`log` has a band");
            for compaction in [DRIFT_SYMLOG, DRIFT_LIN, DRIFT_LOG] {
                for x in [f32::INFINITY, f32::NEG_INFINITY] {
                    assert_eq!(
                        shown(x, compaction, band),
                        x,
                        "a non-finite drift did not pass through"
                    );
                }
                assert!(
                    shown(f32::NAN, compaction, band).is_nan(),
                    "a NaN drift did not pass through"
                );
            }
            for b in [f32::INFINITY, f32::NAN] {
                assert_eq!(
                    shown(inside, DRIFT_SYMLOG, b),
                    inside,
                    "a non-finite band made a drift neutral"
                );
            }
        }
    }
}

#[test]
fn qa_tool227_band_edge_compactions_and_non_finite() {
    check_edges(present::dbg_drift_shown);
}

negative_control!(
    qa_tool227_band_edge_compactions_and_non_finite,
    "a band strict at its edge, `|x| < band`",
    expected = "is not drawn as 0",
    check_edges(|x, c, b| if c == DRIFT_SYMLOG && x.abs() < b {
        0.0
    } else {
        x
    })
);

#[cfg(feature = "controls")]
mod qa_tool227_band_edge_compactions_and_non_finite_more {
    use super::*;

    negative_control!(
        qa_tool227_band_edge_compactions_and_non_finite,
        "a band under `log` too",
        expected = "`log` has a band",
        check_edges(|x, c, b| if c != DRIFT_LIN && x.abs() <= b {
            0.0
        } else {
            x
        })
    );
}

#[cfg(feature = "controls")]
mod qa_tool227_band_edge_compactions_and_non_finite_inf {
    use super::*;

    negative_control!(
        qa_tool227_band_edge_compactions_and_non_finite,
        "an infinite band, not tested first, makes a drift neutral",
        expected = "a non-finite band made a drift neutral",
        check_edges(|x, c, b| {
            if x.is_finite() && c == DRIFT_SYMLOG && x.abs() <= b {
                0.0
            } else {
                x
            }
        })
    );
}

// ── the field-ramp path (R-929) ─────────────────────────────────────────────────────────────────────────────────────

fn ramp_of(field: &str, compaction: Compaction) -> FieldRamp {
    FieldRamp::new(
        ScalarField::payload(field).unwrap_or_else(|e| panic!("{e}")),
        compaction,
        Map::ramp(Ramp::Lut(Lut::Coolwarm)),
    )
}

/// The diverging (symlog) compaction at a floor of the drift's own order.
const X0: f64 = 1e-6;

/// The field ramp on a drift has the band under `diverging`, and only there; a ramp on any other field compacted by
/// the symlog has none, `compact_symlog` itself unchanged: `E_0` and `Lz_0` among them. `banded(field)` says whether a
/// ramp on `field` is expected to band.
fn check_ramp_band(banded: &dyn Fn(&str) -> bool) {
    for field in ["energy_drift", "Lz_drift", "E_0", "Lz_0"] {
        let ramp = ramp_of(field, Compaction::Diverging { x0: X0 });
        let band = 1e-6_f32;
        let inside = ramp.shown_banded(band / 2.0, true, Reads::HEADER, band);
        let past = ramp.shown_banded(band * 2.0, true, Reads::HEADER, band);
        if banded(field) {
            assert_eq!(
                inside,
                RampShown::Ramp(0.5),
                "a ramp on `{field}` drew a drift inside the band off neutral"
            );
            assert!(
                matches!(past, RampShown::Ramp(t) if t > 0.5),
                "a ramp on `{field}` moved a drift past the band: {past:?}"
            );
            let wgsl = ramp.wgsl();
            assert!(
                wgsl.contains("dbg_drift_banded("),
                "the ramp's WGSL on `{field}` has no band"
            );
        } else {
            assert!(
                matches!(inside, RampShown::Ramp(t) if t > 0.5),
                "a ramp on `{field}`, no drift, banded a value: {inside:?}"
            );
            assert!(
                !ramp.wgsl().contains("dbg_drift_banded("),
                "the ramp's WGSL on `{field}`, no drift, has a band"
            );
        }
        // Under `log` no field has a band.
        let log = ramp_of(field, Compaction::Log { eps: X0 });
        assert!(
            !log.wgsl().contains("dbg_drift_banded("),
            "a `log` ramp on `{field}` has a band"
        );
        assert_ne!(
            log.shown_banded(band / 2.0, true, Reads::HEADER, band),
            log.shown_banded(0.0, true, Reads::HEADER, band),
            "a `log` ramp on `{field}` drew a value inside the band as 0"
        );
    }
}

#[test]
fn qa_tool227_field_ramp_band_on_drifts_only() {
    check_ramp_band(&|f| f == "energy_drift" || f == "Lz_drift");
}

negative_control!(
    qa_tool227_field_ramp_band_on_drifts_only,
    "a ramp on Lz_drift expected without the band",
    expected = "banded a value",
    check_ramp_band(&|f| f == "energy_drift")
);

#[cfg(feature = "controls")]
mod qa_tool227_field_ramp_band_on_drifts_only_reference {
    use super::*;

    negative_control!(
        qa_tool227_field_ramp_band_on_drifts_only,
        "a ramp on E_0, a reference, expected to band",
        expected = "drew a drift inside the band off neutral",
        check_ramp_band(&|f| f != "Lz_0")
    );
}

// ── the views hand `dbg_suspect` the drift ───────────────────────────────────────────────────────────────────────────

const VIEW_IDS: [&str; 3] = [
    "debug/generated/energy_drift",
    "debug/generated/Lz_drift",
    "debug/derived/energy_drift",
];

fn view_source(id: &str) -> String {
    registry::registry()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("no registry entry `{id}`"))
        .source
}

/// Each drift view hands `dbg_suspect` the drift `x`, not the banded value, and draws the banded one.
fn check_suspect(source: &dyn Fn(&str) -> String) {
    for id in VIEW_IDS {
        let s = source(id);
        assert!(
            s.contains("dbg_suspect(rgb, x,"),
            "`{id}` does not hand `dbg_suspect` the drift"
        );
        assert!(
            s.contains("dbg_drift(shown,"),
            "`{id}` does not draw the banded drift"
        );
    }
}

#[test]
fn qa_tool227_views_hand_the_suspect_styling_the_drift() {
    check_suspect(&view_source);
}

negative_control!(
    qa_tool227_views_hand_the_suspect_styling_the_drift,
    "a view handing `dbg_suspect` the banded value",
    expected = "does not hand `dbg_suspect` the drift",
    check_suspect(&|id| view_source(id).replace("dbg_suspect(rgb, x,", "dbg_suspect(rgb, shown,"))
);

// ── on the GPU: the near-zero states fresh, and past the band ────────────────────────────────────────────────────────

/// The GPU set: the figure-eight, two near-parabolic and one zero-`L_z` state, each fresh (samples 0–3) and then with
/// both references off by 3 bands, a drift past the band (samples 4–7).
fn gpu_states() -> [(Vec2s, Vec2s, [f32; 3]); 4] {
    let set = near_zero_set();
    [set[0], set[1], set[2], set[25]]
}

fn scene(colouring: Colouring) -> Scene {
    let mut s = row_scene("qa drift band", 8, colouring).unwrap_or_else(|e| panic!("{e}"));
    validation::golden_scene::showcase(&mut s.set, 0);
    for (i, (r, p, m)) in (0..8u32).zip(gpu_states().iter().cycle()) {
        s.set.sample(i).r(*r).p(*p);
        let ic = s.set.ic(i);
        (ic.m0, ic.m1, ic.m2) = (m[0], m[1], m[2]);
    }
    for i in 0..8u32 {
        let read = s.read(i);
        let m = s.masses(i);
        let (.., h, lz, _) = invariants(read.r, read.p, m);
        let off = if i >= 4 { 3.0 } else { 0.0 };
        let de = off * f64::from(present::drift_band_energy(read.r, read.p, m));
        let dl = off * f64::from(present::drift_band_lz(read.r, read.p));
        s.set
            .sample(i)
            .E_0((h - de) as f32)
            .Lz_0((lz - dl) as f32)
            .drift_max(0.0, 0.0)
            .state(ledger::display::State::Bounded.code());
    }
    s
}

fn render(h: &GpuHarness, s: &Scene) -> Vec<Rgb> {
    s.render(h.device(), h.queue())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|p| [p[0], p[1], p[2]].map(f64::from))
        .collect()
}

fn bytes(c: Rgb) -> [u8; 3] {
    c.map(|v| (v * 255.0).round() as u8)
}

/// Samples 0–3 render exactly `neutral`, by their 8-bit values; samples 4–7 render off it, on the warm side.
fn check_gpu(s: &Scene, image: &[Rgb], neutral: Rgb, what: &str) {
    let width = s.size().0;
    for i in 0..8u32 {
        for (x, y) in s.pixels(i) {
            let got = image[(y * width + x) as usize];
            if i < 4 {
                assert_eq!(
                    bytes(got),
                    bytes(neutral),
                    "{what}: fresh sample {i} is not neutral at ({x}, {y}): {got:?}"
                );
            } else {
                assert!(
                    bytes(got) != bytes(neutral) && got[0] > got[2],
                    "{what}: sample {i}, past the band, is not on the warm side: {got:?}"
                );
            }
        }
    }
}

fn view(id: &str) -> Colouring {
    Colouring::Debug(Box::leak(Box::new(DebugCase {
        name: format!("qa {id}"),
        id: id.to_owned(),
        params: Vec::new(),
        nudge: 0,
    })))
}

fn ramps() -> [FieldRamp; 2] {
    [
        ramp_of(
            "energy_drift",
            Compaction::Diverging {
                x0: ledger::constants::EPS_E_DEFAULT.number(),
            },
        ),
        ramp_of(
            "Lz_drift",
            Compaction::Diverging {
                x0: ledger::constants::EPS_L_DEFAULT.number(),
            },
        ),
    ]
}

fn gpu() -> GpuHarness {
    GpuHarness::new().unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn qa_tool227_gpu_near_zero_fresh_neutral_and_past_band_shows() {
    let h = gpu();
    for id in VIEW_IDS {
        let s = scene(view(id));
        check_gpu(&s, &render(&h, &s), present::drift_ramp(0.5, false), id);
    }
    for ramp in ramps() {
        let what = ramp.field.field;
        let s = scene(Colouring::Ramp {
            ramp,
            override_on: false,
        });
        check_gpu(
            &s,
            &render(&h, &s),
            Ramp::Lut(Lut::Coolwarm).colour(0.5),
            what,
        );
    }
}

/// The field ramp on `field` with its band replaced by R-652's, `128·2⁻²³·|reference|`.
#[cfg(feature = "controls")]
fn relative_ramp(field: &str) -> Colouring {
    let ramp = ramps()
        .into_iter()
        .find(|r| r.field.field == field)
        .unwrap_or_else(|| panic!("no ramp on `{field}`"));
    let band = ramp.field.band.unwrap_or_else(|| panic!("no band"));
    let reference = ledger::gen::catalogue::drift_reference(field);
    let source = ramp.wgsl();
    assert!(source.contains(band), "the ramp has no `{band}`");
    Colouring::Probe(
        source.replace(
            band,
            &format!("(1.52587890625e-5 * abs(ctx.sample.{reference}))"),
        ),
        Vec::new(),
    )
}

negative_control!(
    qa_tool227_gpu_near_zero_fresh_neutral_and_past_band_shows,
    "a field ramp on Lz_drift with the band relative to |Lz_0| (k = 128)",
    expected = "is not neutral",
    {
        let s = scene(relative_ramp("Lz_drift"));
        check_gpu(
            &s,
            &render(&gpu(), &s),
            Ramp::Lut(Lut::Coolwarm).colour(0.5),
            "Lz_drift",
        );
    }
);

#[cfg(feature = "controls")]
mod qa_tool227_gpu_near_zero_fresh_neutral_and_past_band_shows_energy {
    use super::*;

    negative_control!(
        qa_tool227_gpu_near_zero_fresh_neutral_and_past_band_shows,
        "a field ramp on energy_drift with the band relative to |E_0| (k = 128)",
        expected = "is not neutral",
        {
            let s = scene(relative_ramp("energy_drift"));
            check_gpu(
                &s,
                &render(&gpu(), &s),
                Ramp::Lut(Lut::Coolwarm).colour(0.5),
                "energy_drift",
            );
        }
    );
}

// ── the scene's CPU twin of the ramp reads the band (R-929) ─────────────────────────────────────────────────────────

/// The CPU twin `place` of the field ramp on `energy_drift` places the near-zero GPU set's fresh samples (0–3) at the
/// ramp's neutral, 0.5, as its WGSL draws them, and the samples 3 bands past it (4–7) on the warm side, above 0.5.
/// (`Lz_drift` has no CPU twin: a scene's look of it is an error.)
fn check_twin(place: &dyn Fn(&Scene, &FieldRamp, u32) -> f64) {
    let [ramp, _] = ramps();
    let s = scene(Colouring::Ramp {
        ramp: ramp.clone(),
        override_on: false,
    });
    for i in 0..8u32 {
        let t = place(&s, &ramp, i);
        if i < 4 {
            assert_eq!(
                t, 0.5,
                "the twin places fresh sample {i} at {t}, not neutral"
            );
        } else {
            assert!(
                t > 0.5,
                "the twin places sample {i}, past the band, at {t}, not warm"
            );
        }
    }
}

#[test]
fn qa_tool227_scene_twin_ramp_reads_the_band() {
    check_twin(
        &|s, _, i| match s.look(i).unwrap_or_else(|e| panic!("{e}")) {
            Look::Ramp { t, .. } => t,
            other => panic!("sample {i}'s look is {other:?}, not a ramp"),
        },
    );
}

negative_control!(
    qa_tool227_scene_twin_ramp_reads_the_band,
    "the twin through `shown_in`, the ramp without the band",
    expected = "not neutral",
    check_twin(&|s, ramp, i| {
        let (v, gate) = s.value(i).unwrap_or_else(|e| panic!("{e}"));
        match ramp.shown_in(v, gate, Reads::HEADER, s.set.forbidden(i), s.read(i).state) {
            RampShown::Ramp(t) => t,
            other => panic!("sample {i} shows {other:?}"),
        }
    })
);
