//! QA tests for TASK-M2-41 on the generated link code, written from the requirements, not from the implementation
//! (R-481; R-482 C2, C4, C5; chart_decoder_contract § "Three hard requirements on any registered link", § "Links carry
//! a measure"; inverse_encode_contract § "Chart-aware validation"). They live in `validation` because they read both
//! the generated links (`kernel`) and the register's `ε_phys` and the registry's trees (`ledger`).
//! - REQ-GEN-040 (C2 on the link functions): `LinkParams::DEFAULT` is the chart constants (5, 2) by bits; each tunable
//!   entry's `forward_at`, `inverse_at` and `log_det_at` at the defaults equal, bit for bit at f32 and f64, the
//!   registry's own trees evaluated with the chart constants by name (`ledger::links::eval`, an evaluator independent
//!   of the emitter), so the parametrised code is today's at the defaults; at non-default values they equal the trees
//!   at those values, so every tunable read comes from `p`; the generated file reads no `R::MU_MAX` or `R::Q_MAX`; and
//!   a parameter one ulp off its default moves an output somewhere in the sample (`qa_m241_default_bitwise`).
//! - REQ-GEN-041 (C4), on the generated code: at each range's ends and 16 interior values, under fuzzed `z` with
//!   saturating controls, at f32 and f64, the output is in its codomain at the active value, the round trip holds within
//!   `ε_phys·ε_R` in physical units wherever the encode's own clamp test says the value is not clamped, and the log-det is
//!   finite (`qa_m241_range_admissible_generated`).
//! - REQ-GEN-039 (C5): at each range's ends and its quartiles, the generated log-det equals `log √det(JᵀJ)` of the
//!   generated forward, by central differences at f64, within the difference's own error; the mutation arm, the log-det
//!   left at the default while the forward runs at `mu_max = 4` (or `q_max = 1.5`), fails (`qa_m241_logdet_params`).
//! - REQ-DEC-048: at `q_max = 1.5` the decode keeps `|q| ≤ 1.5` under saturating controls for every momentum link, and
//!   encode clamps a momentum of 1.8 to the active bound and reports it in layer 1's clamp bits, which it does not at
//!   the default; at `mu_max = 4` the mass ratio's cap is `(1 − ε_μ)·4` in encode and `4` in decode; at the defaults
//!   decode and encode are today's bit for bit (`qa_m241_active`).
//!
//! **Domain** (pitfalls §10): stateless link evaluations, decode and encode; no trajectory.
//!
//! Each test has a registered negative control (R-176).

use kernel::decode::{decode, decode_at, Latent};
use kernel::encode::blocks::clamps_at;
use kernel::encode::{encode, encode_at, SimplexClamp};
use kernel::generated::constants::ChartConstants;
use kernel::generated::links::{
    Defaults, Link, LinkParams, LinkReal, Select, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh,
    SoftsignQ, TanhQ,
};
use ledger::constants::EPS_PHYS;
use ledger::links::{eval, registered, tunables, Fl, Param, Values};
use validation::negative_control;

/// A float type both sides evaluate at.
trait Both: LinkReal + Fl {}
impl Both for f32 {}
impl Both for f64 {}

fn of<R: Both>(v: f64) -> R {
    <R as Fl>::of(v)
}

fn wide<R: Both>(v: R) -> f64 {
    <R as Fl>::f64(v)
}

/// The register's `ε_phys·ε_R` at `R`: the encode's tolerance in canonical units (REQ-ENC-024).
fn eps_phys<R: Fl>() -> f64 {
    EPS_PHYS.number() * R::EPS
}

/// splitmix64 from a fixed seed: the fuzz.
struct Seeded(u64);

impl Seeded {
    fn unit(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Saturating controls: past where every bounded link rounds to its bound at f32 and f64, and far past it.
const SATURATING: [f64; 10] = [20.0, -20.0, 40.0, -40.0, 1e3, -1e3, 1e8, -1e8, 1e30, -1e30];

/// `n`-control samples: 48 seeded draws on `[−span, span]ⁿ`, 0, and with `saturating` each saturating control in
/// each slot beside drawn ones.
fn sample(n: usize, span: f64, saturating: bool, seed: u64) -> Vec<Vec<f64>> {
    let mut g = Seeded(seed);
    let mut out: Vec<Vec<f64>> = (0..48)
        .map(|_| (0..n).map(|_| span * (2.0 * g.unit() - 1.0)).collect())
        .collect();
    out.push(vec![0.0; n]);
    if saturating {
        for s in SATURATING {
            for k in 0..n {
                let mut z: Vec<f64> = (0..n).map(|_| span * (2.0 * g.unit() - 1.0)).collect();
                z[k] = s;
                out.push(z);
            }
            out.push(vec![s; n]);
        }
    }
    out
}

/// A tunable parameter's declared range `(lo, hi)` on `entry`.
fn range(entry: &str) -> (f64, f64) {
    let r = tunables()
        .iter()
        .find(|t| t.entry == entry)
        .unwrap_or_else(|| panic!("no tunable row on `{entry}`"));
    (r.lo, r.hi)
}

/// The tunable parameter of `entry`, by name.
fn param_of(entry: &str) -> &'static str {
    tunables().iter().find(|t| t.entry == entry).unwrap().param
}

/// `LinkParams` at `R` with `name` set to `v` (rounded to nearest, as R-481 G7 has the f32 uniform), the other field
/// its default.
fn params<R: Both>(name: &str, v: f64) -> LinkParams<R> {
    let mut p = LinkParams::<R>::DEFAULT;
    match name {
        "mu_max" => p.mu_max = of(v),
        "q_max" => p.q_max = of(v),
        other => panic!("no field `{other}`"),
    }
    p
}

// ── REQ-GEN-040: the defaults bitwise today's ───────────────────────────────────────────────────────────────────

/// The registry's trees of `L`, evaluated at `R` with the chart constants by name and `active` over them.
fn trees<R: Both>(name: &str, which: usize, x: &[R], active: &[Param]) -> Vec<R> {
    let (l, _) = registered()
        .iter()
        .find(|(l, _)| l.name == name)
        .unwrap_or_else(|| panic!("no entry `{name}`"));
    let declared = [l.clamps, l.params].concat();
    let values = Values {
        declared: &declared,
        active,
    };
    match which {
        0 => l.forward.iter().map(|t| eval(t, x, &values)).collect(),
        1 => l.inverse.iter().map(|t| eval(t, x, &values)).collect(),
        _ => vec![eval(&l.log_det, x, &values)],
    }
}

fn bits<R: Both>(v: &[R]) -> Vec<u64> {
    v.iter().map(|x| wide(*x).to_bits()).collect()
}

/// Each of `L`'s three functions at `p` against the registry's trees at `active`, over the sample with saturating
/// controls, the inverse fed the forward's outputs and values past the bound; a mismatch's message ends "(bitwise)".
fn check_link<const C: usize, const P: usize, L: Link<C, P>, R: Both>(
    p: &LinkParams<R>,
    active: &[Param],
) {
    let mut inputs: Vec<[R; P]> = Vec::new();
    for z in sample(C, 8.0, true, 0x0A41_0001) {
        let x: [R; C] = core::array::from_fn(|k| of(z[k]));
        let y = L::forward_at(x, p);
        assert_eq!(
            bits(&y),
            bits(&trees::<R>(L::NAME, 0, &x, active)),
            "`{}` forward at {z:?} (bitwise)",
            L::NAME
        );
        assert_eq!(
            bits(&[L::log_det_at(x, p)]),
            bits(&trees::<R>(L::NAME, 2, &x, active)),
            "`{}` log-det at {z:?} (bitwise)",
            L::NAME
        );
        inputs.push(y);
        inputs.push(core::array::from_fn(|k| y[k] * of(1.25)));
    }
    for y in inputs {
        assert_eq!(
            bits(&L::inverse_at(y, p)),
            bits(&trees::<R>(L::NAME, 1, &y, active)),
            "`{}` inverse at {:?} (bitwise)",
            L::NAME,
            y.iter().map(|v| wide(*v)).collect::<Vec<_>>()
        );
    }
}

/// Every tunable entry at `p`, against the trees at `active`.
fn check_all<R: Both>(p: &LinkParams<R>, active: &[Param]) {
    check_link::<2, 3, SoftmaxTanh, R>(p, active);
    check_link::<1, 1, SigmoidQ, R>(p, active);
    check_link::<1, 1, TanhQ, R>(p, active);
    check_link::<1, 1, SoftsignQ, R>(p, active);
}

/// `LinkParams::DEFAULT` is the chart constants (R-10's 5 and 2) by bits.
fn check_default_fields<R: Both>() {
    let d = LinkParams::<R>::DEFAULT;
    assert_eq!(
        wide(d.mu_max).to_bits(),
        wide(R::MU_MAX).to_bits(),
        "DEFAULT.mu_max"
    );
    assert_eq!(
        wide(d.q_max).to_bits(),
        wide(R::Q_MAX).to_bits(),
        "DEFAULT.q_max"
    );
    assert_eq!((wide(d.mu_max), wide(d.q_max)), (5.0, 2.0), "R-10's values");
}

/// `name` one ulp above its default at `R` moves some output of `L`'s forward, inverse or log-det in the sample.
fn moves<const C: usize, const P: usize, L: Link<C, P>, R: Both>(name: &str) -> bool {
    let d = LinkParams::<R>::DEFAULT;
    let mut p = d;
    match name {
        "mu_max" => p.mu_max = next_up(d.mu_max),
        _ => p.q_max = next_up(d.q_max),
    }
    sample(C, 8.0, true, 0x0A41_0002).iter().any(|z| {
        let x: [R; C] = core::array::from_fn(|k| of(z[k]));
        let y = L::forward_at(x, &d);
        bits(&L::forward_at(x, &p)) != bits(&y)
            || bits(&[L::log_det_at(x, &p)]) != bits(&[L::log_det_at(x, &d)])
            || bits(&L::inverse_at(y, &p)) != bits(&L::inverse_at(y, &d))
    })
}

/// The next representable value above `v` at its own width.
fn next_up<R: Both>(v: R) -> R {
    if R::EPS == f64::EPSILON {
        of(f64::from_bits(wide(v).to_bits() + 1))
    } else {
        of(f64::from(f32::from_bits((wide(v) as f32).to_bits() + 1)))
    }
}

fn check_ulp<R: Both>() {
    assert!(
        moves::<2, 3, SoftmaxTanh, R>("mu_max"),
        "mu_max one ulp off moves nothing"
    );
    assert!(
        moves::<1, 1, SigmoidQ, R>("q_max"),
        "q_max one ulp off moves nothing in sigmoid_q"
    );
    assert!(
        moves::<1, 1, TanhQ, R>("q_max"),
        "q_max one ulp off moves nothing in tanh_q"
    );
    assert!(
        moves::<1, 1, SoftsignQ, R>("q_max"),
        "q_max one ulp off moves nothing in softsign_q"
    );
}

/// Non-default values: each range's ends and midpoint, both parameters set, at `R`.
fn check_non_default<R: Both>() {
    let (mlo, mhi) = range("softmax_tanh");
    let (qlo, qhi) = range("tanh_q");
    for (mu, q) in [
        (mlo, qhi),
        (mhi, qlo),
        (0.5 * (mlo + mhi), 0.5 * (qlo + qhi)),
    ] {
        let p = LinkParams::<R> {
            mu_max: of(mu),
            q_max: of(q),
        };
        let active = [
            Param {
                name: "mu_max",
                value: mu,
            },
            Param {
                name: "q_max",
                value: q,
            },
        ];
        check_all::<R>(&p, &active);
    }
}

#[test]
fn qa_m241_default_bitwise() {
    check_default_fields::<f32>();
    check_default_fields::<f64>();
    check_all::<f32>(&LinkParams::DEFAULT, &[]);
    check_all::<f64>(&LinkParams::DEFAULT, &[]);
    check_non_default::<f32>();
    check_non_default::<f64>();
    check_ulp::<f32>();
    check_ulp::<f64>();
    let text = include_str!("../../kernel/src/generated/links.rs");
    for name in ["R::MU_MAX", "R::Q_MAX"] {
        assert!(
            !text.contains(name),
            "the generated links read {name} by name"
        );
    }
}

negative_control!(
    qa_m241_default_bitwise,
    "the links at mu_max and q_max one ulp off their defaults are not the trees at the defaults",
    expected = "(bitwise)",
    {
        let p = LinkParams::<f64> {
            mu_max: next_up(5.0f64),
            q_max: next_up(2.0f64),
        };
        check_all::<f64>(&p, &[])
    }
);

// ── REQ-GEN-041 on the generated code: the ranges admissible ───────────────────────────────────────────────────

/// `[lo, hi]`'s ends and 16 interior values, evenly spaced strictly inside it.
fn range_sample(lo: f64, hi: f64) -> Vec<f64> {
    let mut v = vec![lo, hi];
    v.extend((1..=16).map(|k| lo + (hi - lo) * k as f64 / 17.0));
    v
}

/// The three requirements for a momentum link `L` run at `q_max = q`, its codomain judged at `q_max = judge`, at `R`;
/// a failure's message ends "(admissible)".
fn check_momentum_judged<L: Link<1, 1>, R: Both>(q: f64, judge: f64) -> usize
where
    L::Codomain: kernel::encode::blocks::Ends,
{
    let p = params::<R>("q_max", q);
    let bound = wide(params::<R>("q_max", judge).q_max);
    let mut tripped = 0;
    for z in sample(1, 8.0, true, 0x0A41_0003) {
        let x = [of::<R>(z[0])];
        let [y] = L::forward_at(x, &p);
        assert!(
            wide(y).abs() <= bound,
            "`{}` at q_max = {q}, z = {z:?}: {} leaves [−q_max, q_max] (admissible)",
            L::NAME,
            wide(y)
        );
        assert!(
            wide(L::log_det_at(x, &p)).is_finite() || z[0].abs() >= 20.0,
            "`{}` at q_max = {q}, z = {z:?}: the log-det is not finite (admissible)",
            L::NAME
        );
        if !clamps_at::<L, R>(y, &p) {
            tripped += 1;
            let [back] = L::forward_at(L::inverse_at([y], &p), &p);
            let err = (wide(back) - wide(y)).abs();
            assert!(
                err <= eps_phys::<R>(),
                "`{}` at q_max = {q}, z = {z:?}: the round trip is {err:e} off, past ε_phys (admissible)",
                L::NAME
            );
        }
    }
    tripped
}

/// [`check_momentum_judged`] judged at the value it runs at.
fn check_momentum<L: Link<1, 1>, R: Both>(q: f64) -> usize
where
    L::Codomain: kernel::encode::blocks::Ends,
{
    check_momentum_judged::<L, R>(q, q)
}

/// The three requirements for the mass link at `mu_max = mu`, at `R`.
fn check_mass<R: Both>(mu: f64) -> usize {
    let p = params::<R>("mu_max", mu);
    let mut tripped = 0;
    for z in sample(2, 8.0, true, 0x0A41_0004) {
        let x = [of::<R>(z[0]), of::<R>(z[1])];
        let m = SoftmaxTanh::forward_at(x, &p);
        let sum: f64 = m.iter().map(|v| wide(*v)).sum();
        assert!(
            m.iter().all(|v| (0.0..=1.0).contains(&wide(*v)))
                && (sum - 1.0).abs() <= eps_phys::<R>(),
            "softmax_tanh at mu_max = {mu}, z = {z:?}: {m:?} is no simplex point (admissible)"
        );
        if <SoftmaxTanh as SimplexClamp>::clamps_at(m, &p) == 0 {
            tripped += 1;
            let back = SoftmaxTanh::forward_at(SoftmaxTanh::inverse_at(m, &p), &p);
            let err = m
                .iter()
                .zip(&back)
                .map(|(a, b)| (wide(*a) - wide(*b)).abs())
                .fold(0.0, f64::max);
            assert!(
                err <= eps_phys::<R>(),
                "softmax_tanh at mu_max = {mu}, z = {z:?}: the round trip is {err:e} off (admissible)"
            );
        }
    }
    tripped
}

fn check_ranges<R: Both>() {
    let (mlo, mhi) = range("softmax_tanh");
    for mu in range_sample(mlo, mhi) {
        assert!(
            check_mass::<R>(mu) > 0,
            "no unclamped mass sample at mu_max = {mu}"
        );
    }
    for (entry, f) in [
        (
            "sigmoid_q",
            check_momentum::<SigmoidQ, R> as fn(f64) -> usize,
        ),
        ("tanh_q", check_momentum::<TanhQ, R>),
        ("softsign_q", check_momentum::<SoftsignQ, R>),
    ] {
        let (lo, hi) = range(entry);
        for q in range_sample(lo, hi) {
            assert!(f(q) > 0, "no unclamped `{entry}` sample at q_max = {q}");
        }
    }
}

#[test]
fn qa_m241_range_admissible_generated() {
    check_ranges::<f32>();
    check_ranges::<f64>();
}

negative_control!(
    qa_m241_range_admissible_generated,
    "tanh_q's forward run at q_max = 3 and judged at the default's [−2, 2] leaves its codomain",
    expected = "(admissible)",
    {
        check_momentum_judged::<TanhQ, f64>(3.0, 2.0);
    }
);

// ── REQ-GEN-039 (C5): the log-det at the active value ───────────────────────────────────────────────────────────

/// The forward's central difference along control `k` at `x`, step `m·h`, `h = ε^(1/3)·max(1, |x_k|)`: the column and
/// its rounding bound, each forward value within `ε_phys·ε·max(1, |f|)`.
fn column<const C: usize, const P: usize, L: Link<C, P>>(
    x: [f64; C],
    k: usize,
    m: f64,
    p: &LinkParams<f64>,
) -> ([f64; P], f64) {
    let h = m * f64::EPSILON.cbrt() * x[k].abs().max(1.0);
    let (mut up, mut down) = (x, x);
    up[k] += h;
    down[k] -= h;
    let step = up[k] - down[k];
    let (a, b) = (L::forward_at(up, p), L::forward_at(down, p));
    let col = core::array::from_fn(|i| (a[i] - b[i]) / step);
    let scale = a.iter().chain(&b).fold(1.0f64, |s, v| s.max(v.abs()));
    (col, 2.0 * eps_phys::<f64>() * scale / step)
}

/// `√det(JᵀJ)` of the differenced Jacobian at step multiple `m`, and its rounding bound.
fn volume<const C: usize, const P: usize, L: Link<C, P>>(
    x: [f64; C],
    m: f64,
    p: &LinkParams<f64>,
) -> (f64, f64) {
    let norm = |c: &[f64]| c.iter().map(|v| v * v).sum::<f64>().sqrt();
    if C == 1 {
        let (c, r) = column::<C, P, L>(x, 0, m, p);
        (norm(&c), r)
    } else {
        let (a, ra) = column::<C, P, L>(x, 0, m, p);
        let (b, rb) = column::<C, P, L>(x, 1, m, p);
        let dot: f64 = a.iter().zip(&b).map(|(u, v)| u * v).sum();
        let (na, nb) = (norm(&a), norm(&b));
        let gram = (na * na * nb * nb - dot * dot).max(0.0);
        // Each column's error, ra or rb in each of its P components, bounds the area's by Cauchy–Schwarz.
        let (ea, eb) = (ra * (P as f64).sqrt(), rb * (P as f64).sqrt());
        (gram.sqrt(), ea * nb + na * eb + ea * eb)
    }
}

/// `L`'s log-det read at `ld_p` against the forward's differenced volume at `f_p`, at `x`: whether they agree within
/// the difference's own error, Richardson's `|V(h) − V(2h)|` (three times the `O(h²)` error of `V(h)`) plus both
/// steps' rounding and the log-det's own, `ε_phys·ε` relative.
fn agrees<const C: usize, const P: usize, L: Link<C, P>>(
    x: [f64; C],
    ld_p: &LinkParams<f64>,
    f_p: &LinkParams<f64>,
) -> (bool, f64, f64) {
    let from_ld = L::log_det_at(x, ld_p).exp();
    let (v1, r1) = volume::<C, P, L>(x, 1.0, f_p);
    let (v2, r2) = volume::<C, P, L>(x, 2.0, f_p);
    let tol = (v1 - v2).abs() + r1 + r2 + eps_phys::<f64>() * from_ld;
    (
        from_ld.is_finite() && (from_ld - v1).abs() <= tol,
        from_ld,
        v1,
    )
}

/// Every sample point agrees at the parameter value `v` of `name`, the log-det read at `ld_v`; returns how many
/// disagree, and with `strict` panics on the first.
fn logdet_fails<const C: usize, const P: usize, L: Link<C, P>>(
    name: &str,
    v: f64,
    ld_v: f64,
    strict: bool,
) -> usize {
    let (f_p, ld_p) = (params::<f64>(name, v), params::<f64>(name, ld_v));
    let mut bad = 0;
    for z in sample(C, 6.0, false, 0x0A41_0005) {
        let x: [f64; C] = core::array::from_fn(|k| z[k]);
        let (ok, ld, fd) = agrees::<C, P, L>(x, &ld_p, &f_p);
        if !ok {
            bad += 1;
            assert!(
                !strict,
                "`{}` at {name} = {v}, z = {z:?}: exp(log-det) {ld:e} is not the differenced volume {fd:e} (logdet)",
                L::NAME
            );
        }
    }
    bad
}

/// The ends and quartiles of `entry`'s range.
fn logdet_values(entry: &str) -> Vec<f64> {
    let (lo, hi) = range(entry);
    vec![
        lo,
        hi,
        lo + 0.25 * (hi - lo),
        lo + 0.5 * (hi - lo),
        lo + 0.75 * (hi - lo),
    ]
}

fn check_logdet() {
    for v in logdet_values("softmax_tanh") {
        logdet_fails::<2, 3, SoftmaxTanh>("mu_max", v, v, true);
    }
    for (entry, f) in [
        (
            "sigmoid_q",
            logdet_fails::<1, 1, SigmoidQ> as fn(&str, f64, f64, bool) -> usize,
        ),
        ("tanh_q", logdet_fails::<1, 1, TanhQ>),
        ("softsign_q", logdet_fails::<1, 1, SoftsignQ>),
    ] {
        assert_eq!(param_of(entry), "q_max");
        for v in logdet_values(entry) {
            f("q_max", v, v, true);
        }
    }
}

#[test]
fn qa_m241_logdet_params() {
    check_logdet();
    // C5's mutation arm: the log-det left at the default while the forward runs elsewhere disagrees.
    assert!(
        logdet_fails::<2, 3, SoftmaxTanh>("mu_max", 4.0, 5.0, false) > 0,
        "a log-det at mu_max = 5 passes against the forward at 4"
    );
    for f in [
        logdet_fails::<1, 1, SigmoidQ> as fn(&str, f64, f64, bool) -> usize,
        logdet_fails::<1, 1, TanhQ>,
        logdet_fails::<1, 1, SoftsignQ>,
    ] {
        assert!(
            f("q_max", 1.5, 2.0, false) > 0,
            "a log-det at q_max = 2 passes against the forward at 1.5"
        );
    }
}

negative_control!(
    qa_m241_logdet_params,
    "the log-det at the default mu_max = 5 against the forward at 4 fails the strict check",
    expected = "(logdet)",
    {
        logdet_fails::<2, 3, SoftmaxTanh>("mu_max", 4.0, 5.0, true);
    }
);

// ── REQ-DEC-048: the active value everywhere ────────────────────────────────────────────────────────────────────

type TanhMomenta = Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, TanhQ, TanhQ, TanhQ, TanhQ>;
type SoftsignMomenta =
    Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, SoftsignQ, SoftsignQ, SoftsignQ, SoftsignQ>;

/// The largest `|q|` the decode under `S` gives at `p` over the 8-control sample with saturating controls.
fn max_q<S: kernel::generated::links::Selection, R: Both + kernel::payload::PayloadReal>(
    p: &LinkParams<R>,
) -> f64 {
    sample(8, 8.0, true, 0x0A41_0006)
        .iter()
        .map(|z| {
            let lat = Latent::new(core::array::from_fn(|k| of::<R>(z[k])));
            let d = decode_at::<S, R>(&lat, p);
            d.momentum
                .q
                .iter()
                .fold(0.0f64, |s, v| s.max(wide(*v).abs()))
        })
        .fold(0.0, f64::max)
}

/// `|q| ≤ q` under every momentum link at `R`, reached to within the sample's resolution.
fn check_decode_q<R: Both + kernel::payload::PayloadReal>(q: f64) {
    let p = params::<R>("q_max", q);
    for (name, got) in [
        ("sigmoid_q", max_q::<Defaults, R>(&p)),
        ("tanh_q", max_q::<TanhMomenta, R>(&p)),
        ("softsign_q", max_q::<SoftsignMomenta, R>(&p)),
    ] {
        assert!(
            got <= 1.5,
            "`{name}` decodes |q| = {got} past q_max = 1.5 (active)"
        );
        assert!(
            got > 1.4,
            "`{name}`'s |q| reaches only {got} under saturating controls"
        );
    }
}

/// `z` whose sigmoid_q control gives `q₀ = 1.8` at the default `q_max = 2`: `2(2σ(z) − 1) = 1.8`, `σ = 0.95`.
fn z_1_8() -> Latent<f64> {
    let mut z = [0.0; 8];
    z[2] = 19.0f64.ln();
    Latent::new(z)
}

/// Encode at `q_max = q` of a state with a momentum of 1.8: whether layer 1's momentum clamp bits fire, and the
/// largest `|q|` the encoded `z` decodes to at the same `q`.
fn encode_1_8(q: f64) -> (bool, f64) {
    let p = params::<f64>("q_max", q);
    let ic = decode::<Defaults, f64>(&z_1_8()).ic;
    let e = encode_at::<Defaults, f64>(&ic, &p);
    let momentum_bits = (e.report.clamped_controls >> 2) & 0b1111;
    let back = decode_at::<Defaults, f64>(&e.z, &p);
    let got = back.momentum.q.iter().fold(0.0f64, |s, v| s.max(v.abs()));
    (momentum_bits != 0, got)
}

fn check_encode_q(q: f64) {
    let q0 = decode::<Defaults, f64>(&z_1_8()).momentum.q[0];
    assert!(
        (q0 - 1.8).abs() <= eps_phys::<f64>(),
        "the fixture's momentum is {q0}, not 1.8"
    );
    let (clamped, got) = encode_1_8(q);
    assert!(
        clamped,
        "encode at q_max = {q} does not report the momentum 1.8 in layer 1 (active)"
    );
    // Clamped to the active bound: within the inverse's ε_q clamp of it, `q_max·2ε_q` for σ, from below.
    let floor = q * (1.0 - 2.0 * f64::EPS_Q) - eps_phys::<f64>();
    assert!(
        got <= q && got >= floor,
        "encode at q_max = {q} takes 1.8 to {got}, not to the active bound (active)"
    );
    let (clamped, got) = encode_1_8(2.0);
    assert!(
        !clamped,
        "encode at the default q_max = 2 reports the momentum 1.8 as clamped"
    );
    assert!(
        (got - 1.8).abs() <= 1e-9,
        "encode at the default does not keep 1.8: {got}"
    );
}

/// `ln(m₁/m₀) = μ₁`, `μ₂ = 0`: the simplex point.
fn masses(mu1: f64) -> [f64; 3] {
    let e = mu1.exp();
    let m0 = 1.0 / (2.0 + e);
    [m0, e * m0, m0]
}

fn check_mass_cap(mu: f64) {
    let p = params::<f64>("mu_max", mu);
    let cap = (1.0 - f64::EPS_MU) * mu;
    // Encode's layer 1: a ratio between the cap and mu_max is flagged, one under the cap is not.
    let above = masses(mu * (1.0 - 0.5 * f64::EPS_MU));
    let below = masses(mu * (1.0 - 2.0 * f64::EPS_MU));
    assert_ne!(
        SoftmaxTanh::clamps_at(above, &p),
        0,
        "a ratio past (1 − ε_μ)·{mu} is not flagged (active)"
    );
    assert_eq!(
        SoftmaxTanh::clamps_at(below, &p),
        0,
        "a ratio under (1 − ε_μ)·{mu} is flagged (active)"
    );
    // Clamped to the cap: a ratio of e^(mu + 1) returns as e^cap.
    let back = SoftmaxTanh::forward_at(SoftmaxTanh::inverse_at(masses(mu + 1.0), &p), &p);
    let ratio = (back[1] / back[0]).ln();
    assert!(
        (ratio - cap).abs() <= eps_phys::<f64>() * cap,
        "the mass ratio clamps to {ratio}, not (1 − ε_μ)·{mu} = {cap} (active)"
    );
    // Decode: saturating controls reach mu_max and never pass it.
    let worst = sample(2, 8.0, true, 0x0A41_0007)
        .iter()
        .map(|z| {
            let m = SoftmaxTanh::forward_at([z[0], z[1]], &p);
            (m[1] / m[0]).ln().abs().max((m[2] / m[0]).ln().abs())
        })
        .fold(0.0, f64::max);
    assert!(
        worst <= mu * (1.0 + eps_phys::<f64>()) && worst >= mu * (1.0 - eps_phys::<f64>()),
        "the decode's largest mass ratio is e^{worst}, not e^{mu} (active)"
    );
}

/// At the defaults, decode and encode at `LinkParams::DEFAULT` are today's bit for bit.
fn check_defaults_unchanged() {
    for z in sample(8, 8.0, true, 0x0A41_0008) {
        let lat = Latent::new(core::array::from_fn(|k| z[k]));
        let (a, b) = (
            decode::<Defaults, f64>(&lat),
            decode_at::<Defaults, f64>(&lat, &LinkParams::DEFAULT),
        );
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "decode at the defaults moved at {z:?}"
        );
        if a.degenerate.is_none() {
            let (ea, eb) = (
                encode::<Defaults, f64>(&a.ic),
                encode_at::<Defaults, f64>(&a.ic, &LinkParams::DEFAULT),
            );
            assert_eq!(
                format!("{ea:?}"),
                format!("{eb:?}"),
                "encode at the defaults moved at {z:?}"
            );
        }
    }
}

#[test]
fn qa_m241_active() {
    check_decode_q::<f64>(1.5);
    check_decode_q::<f32>(1.5);
    check_encode_q(1.5);
    check_mass_cap(4.0);
    check_defaults_unchanged();
}

negative_control!(
    qa_m241_active,
    "at the default q_max = 2 the decode passes 1.5",
    expected = "(active)",
    check_decode_q::<f64>(2.0)
);
