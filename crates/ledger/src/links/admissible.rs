//! Part 2.5's three hard requirements, tested on the registry's own trees (chart_decoder_contract § "Three hard
//! requirements on any registered link"; dd_generation_root §3.9, "Tunable parameters", the check; R-481 G3,
//! REQ-GEN-041). Each entry is evaluated at f32 and f64 by [`eval`], which evaluates a tree as the emitter's code does
//! (`crate::gen::links`): the same operators in the same order, each `num` and parameter at the float type as the
//! emitter writes it (π/2 and π rounded toward zero at f32, every other value rounded to nearest), and §3.9's `tanh`
//! as the host evaluates it, where its argument bound and select never act.
//!
//! Under a seeded fuzz of the control that includes saturating controls ([`controls`]):
//! 1. **constraint-preserving**: the forward's output lies in the codomain's closed range at the float type, the
//!    simplex's masses each in `[0, 1]` and their sum within `ε_phys` of 1 (its rounding, held to the encode's
//!    tolerance in canonical units, `M = 1`);
//! 2. **a conditioned inverse**: where the forward is not saturated, `z` strictly inside each control's clamp image
//!    (the inverse of the forward of a saturating control), the forward of the inverse of the forward returns the
//!    forward's output within the encode's tolerance, `ε_phys = k·ε_R` (REQ-ENC-024's round trip, in physical units);
//! 3. **C¹**: the derivative from the log-det, `exp(log_det)`, is finite and agrees with a central difference of the
//!    forward (for the simplex, the area element `√det(JᵀJ)` of the differenced Jacobian) at the step
//!    `h = ε_R^(1/3)·max(1, |z|)`, within the difference's own error: its truncation, estimated by Richardson's
//!    `|D(h) − D(2h)|` (the central difference's error is `O(h²)`, so `D(2h)` errs four times as far), plus the
//!    forward's and the log-det's rounding, each held to `ε_phys` (relative for the log-det's exponential).
//!
//! Requirement 1 is tested on every entry at its declared values, so an entry whose forward leaves the codomain it
//! claims is refused (R-482 C1); all three are tested on an entry with a tunable parameter at both ends of the
//! parameter's range and at 16 seeded interior values, uniform over it (R-481 G3's "at least 16").
//!
//! The check's only tolerance is the register's `ε_phys`; its fuzz and its sample are ledger data, the check's own
//! inputs ([`controls`], [`range_values`]), not register constants (REQ-SYS-082).

use std::f64::consts::{FRAC_PI_2, PI};
use std::ops::{Add, Div, Mul, Neg, Sub};
use std::sync::Mutex;

use super::{Codomain, Expr, Link, Op, Param, Tunable};
use crate::constants::EPS_PHYS;

/// A float type the check evaluates at, f32 or f64, with the operations the emitter's code calls.
pub trait Fl:
    Copy
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    /// Its name, for a refusal.
    const NAME: &'static str;
    /// Its machine epsilon, `ε_R`.
    const EPS: f64;
    /// `v` rounded to nearest.
    fn of(v: f64) -> Self;
    /// `v` as the emitter writes a literal: π/2 and π rounded toward zero at f32 (R-417), every other value nearest.
    fn lit(v: f64) -> Self;
    fn f64(self) -> f64;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn ln_1p(self) -> Self;
    fn exp_m1(self) -> Self;
    fn tanh(self) -> Self;
    fn atanh(self) -> Self;
    fn cosh(self) -> Self;
    fn abs(self) -> Self;
    fn max(self, o: Self) -> Self;
    fn min(self, o: Self) -> Self;
}

impl Fl for f64 {
    const NAME: &'static str = "f64";
    const EPS: f64 = f64::EPSILON;
    fn of(v: f64) -> Self {
        v
    }
    fn lit(v: f64) -> Self {
        v
    }
    fn f64(self) -> f64 {
        self
    }
    fn exp(self) -> Self {
        f64::exp(self)
    }
    fn ln(self) -> Self {
        f64::ln(self)
    }
    fn ln_1p(self) -> Self {
        f64::ln_1p(self)
    }
    fn exp_m1(self) -> Self {
        f64::exp_m1(self)
    }
    fn tanh(self) -> Self {
        f64::tanh(self)
    }
    fn atanh(self) -> Self {
        f64::atanh(self)
    }
    fn cosh(self) -> Self {
        f64::cosh(self)
    }
    fn abs(self) -> Self {
        f64::abs(self)
    }
    fn max(self, o: Self) -> Self {
        f64::max(self, o)
    }
    fn min(self, o: Self) -> Self {
        f64::min(self, o)
    }
}

impl Fl for f32 {
    const NAME: &'static str = "f32";
    const EPS: f64 = f32::EPSILON as f64;
    fn of(v: f64) -> Self {
        v as f32
    }
    fn lit(v: f64) -> Self {
        crate::gen::links::f32_image(v)
    }
    fn f64(self) -> f64 {
        f64::from(self)
    }
    fn exp(self) -> Self {
        f32::exp(self)
    }
    fn ln(self) -> Self {
        f32::ln(self)
    }
    fn ln_1p(self) -> Self {
        f32::ln_1p(self)
    }
    fn exp_m1(self) -> Self {
        f32::exp_m1(self)
    }
    fn tanh(self) -> Self {
        f32::tanh(self)
    }
    fn atanh(self) -> Self {
        f32::atanh(self)
    }
    fn cosh(self) -> Self {
        f32::cosh(self)
    }
    fn abs(self) -> Self {
        f32::abs(self)
    }
    fn max(self, o: Self) -> Self {
        f32::max(self, o)
    }
    fn min(self, o: Self) -> Self {
        f32::min(self, o)
    }
}

/// The values a tree's `param(name)` reads: the entry's clamps and parameters, the active values overriding them.
#[derive(Clone, Copy, Debug)]
pub struct Values<'a> {
    pub declared: &'a [Param],
    pub active: &'a [Param],
}

impl Values<'_> {
    /// The value of `name`, active first; NaN for a name neither holds (a tree the structural check refuses).
    pub fn get(&self, name: &str) -> f64 {
        self.active
            .iter()
            .chain(self.declared)
            .find(|p| p.name == name)
            .map_or(f64::NAN, |p| p.value)
    }
}

/// `e` at the float type `F`, its `input(i)` `x[i]` (NaN past `x`), as the emitter's code evaluates it.
pub fn eval<F: Fl>(e: &Expr, x: &[F], values: &Values) -> F {
    let one = F::of(1.0);
    match e {
        // An index past the argument is the structural check's refusal (`entry_problems`); here it reads NaN.
        Expr::Input(i) => x.get(*i as usize).copied().unwrap_or(F::of(f64::NAN)),
        Expr::Param(name) => F::lit(values.get(name)),
        Expr::Num(v) => F::lit(*v),
        Expr::Op(op, args) => {
            // Each argument evaluated as it is read, left to right, as the emitter's `let`s are; an argument a
            // malformed tree lacks reads NaN (its arity is the version's refusal, `crate::version`).
            let arg = |k: usize| args.get(k).map_or(F::of(f64::NAN), |a| eval(a, x, values));
            let fold = |f: fn(F, F) -> F| {
                args[1..]
                    .iter()
                    .fold(arg(0), |s, t| f(s, eval(t, x, values)))
            };
            match op {
                Op::Add if !args.is_empty() => fold(|a, b| a + b),
                Op::Mul if !args.is_empty() => fold(|a, b| a * b),
                Op::Add | Op::Mul => F::of(f64::NAN),
                Op::Sub => arg(0) - arg(1),
                Op::Div => arg(0) / arg(1),
                Op::Neg => -arg(0),
                Op::Exp => arg(0).exp(),
                Op::Log => arg(0).ln(),
                // §3.9's `tanh` clamps its argument to `±tanh_sat` and selects ±1 past 1 in magnitude; on the host
                // neither changes the result, the clamp acting only where `tanh` already rounds to ±1 at f32 and
                // f64 and the select never (dd_generation_root §3.9), so the host's `tanh` is the operator's here.
                Op::Tanh => arg(0).tanh(),
                Op::Artanh => arg(0).atanh(),
                Op::Sigmoid => one / (one + (-arg(0)).exp()),
                Op::Logit => {
                    let a = arg(0);
                    (a / (one - a)).ln()
                }
                Op::Softplus => {
                    let a = arg(0);
                    a.max(F::of(0.0)) + (-a.abs()).exp().ln_1p()
                }
                Op::InvSoftplus => {
                    let a = arg(0);
                    a + (-(-a).exp_m1()).ln()
                }
                Op::Sech2 => {
                    let c = arg(0).cosh();
                    one / (c * c)
                }
                Op::Clamp => arg(0).max(arg(1)).min(arg(2)),
            }
        }
    }
}

/// Each tree of `trees` at `x`.
fn eval_all<F: Fl>(trees: &[Expr], x: &[F], values: &Values) -> Vec<F> {
    trees.iter().map(|t| eval(t, x, values)).collect()
}

/// The seeded generator of the fuzz and the interior sample: splitmix64, a fixed seed per use.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform on `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// The fuzz of a link with `n` controls (one or two): 64 seeded draws uniform on `[−8, 8]ⁿ`, every saturating control
/// (each pair of them for two), and for two each saturating control beside a drawn one, each way. The saturating
/// controls, with 0 and ±½, lie past where every link's `tanh`, σ or softsign rounds to its bound at f32 and f64, and
/// far past it. Ledger data: the check's fuzz (REQ-SYS-082).
pub fn controls(n: u32) -> Vec<Vec<f64>> {
    let mut g = Seeded(0x5EED_0481);
    let mut out = Vec::new();
    for _ in 0..64 {
        out.push((0..n).map(|_| 16.0 * g.unit() - 8.0).collect());
    }
    let sat = [
        0.0, 0.5, -0.5, 20.0, -20.0, 40.0, -40.0, 1e3, -1e3, 1e8, -1e8,
    ];
    if n == 1 {
        out.extend(sat.iter().map(|&s| vec![s]));
    } else {
        for &a in &sat {
            for &b in &sat {
                out.push(vec![a, b]);
            }
            let d = 16.0 * g.unit() - 8.0;
            out.push(vec![a, d]);
            out.push(vec![d, a]);
        }
    }
    out
}

/// `[lo, hi]`'s two ends and 16 seeded values uniform inside it (R-481 G3's "at least 16"). Ledger data: the check's
/// sample.
pub fn range_values(lo: f64, hi: f64) -> Vec<f64> {
    let mut g = Seeded(0x0481_6A6E);
    let mut v = vec![lo, hi];
    v.extend((0..16).map(|_| lo + (hi - lo) * g.unit()));
    v
}

/// Requirement 1 at `F`: the forward's output at `x` in `codomain`'s closed range, its bounds at `F` as the emitter
/// writes them; the simplex's masses each in `[0, 1]`, their sum within `ε_phys` of 1.
pub fn constraint<F: Fl>(
    link: &Link,
    codomain: Codomain,
    x: &[F],
    values: &Values,
) -> Result<(), String> {
    let y = eval_all(link.forward, x, values);
    let ok = match codomain {
        Codomain::Mass => {
            let sum: f64 = y.iter().map(|m| m.f64()).sum();
            y.iter().all(|m| m.f64() >= 0.0 && m.f64() <= 1.0)
                && (sum - 1.0).abs() <= EPS_PHYS.number() * F::EPS
        }
        _ => {
            let (lo, hi) = bounds::<F>(codomain, values);
            y.iter().all(|v| v.f64() >= lo && v.f64() <= hi)
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "its forward leaves codomain `{}` (output {:?})",
            codomain.name(),
            floats(&y)
        ))
    }
}

/// An interval codomain's closed bounds at `F`, as the emitter writes them: `α`'s and `β`'s upper ends are π/2 and π
/// images, rounded toward zero at f32, the momenta's `±q_max` at the active value; a line's are infinite and the
/// half-line's lower end 0.
pub fn bounds<F: Fl>(codomain: Codomain, values: &Values) -> (f64, f64) {
    let at = |v: f64| F::lit(v).f64();
    match codomain {
        Codomain::Alpha => {
            let a = values.get("alpha_min");
            (at(a), (F::lit(FRAC_PI_2) - F::lit(a)).f64())
        }
        Codomain::Beta => (0.0, at(PI)),
        Codomain::Momentum => {
            let q = at(values.get("q_max"));
            (-q, q)
        }
        Codomain::HalfLine => (0.0, f64::INFINITY),
        Codomain::Mass | Codomain::RealLine => (f64::NEG_INFINITY, f64::INFINITY),
    }
}

fn floats<F: Fl>(v: &[F]) -> Vec<f64> {
    v.iter().map(|x| x.f64()).collect()
}

/// Each control's clamp image `[lo_k, hi_k]`: the inverse of the forward of the most saturating control, `∓10³` in
/// every component, which the inverse's ε clamp moves.
pub fn clamp_images<F: Fl>(link: &Link, n: usize, values: &Values) -> Vec<(f64, f64)> {
    let image = |s: f64| {
        let y = eval_all(link.forward, &vec![F::of(s); n], values);
        floats(&eval_all(link.inverse, &y, values))
    };
    let (lo, hi) = (image(-1e3), image(1e3));
    lo.into_iter().zip(hi).collect()
}

/// Requirement 2 at `F`: each control's clamp image finite and not empty, and where `x` lies strictly inside every
/// one, the forward of the inverse of the forward within `ε_phys` of the forward, in the max norm.
pub fn round_trip<F: Fl>(
    link: &Link,
    x: &[F],
    images: &[(f64, f64)],
    values: &Values,
) -> Result<(), String> {
    if let Some((lo, hi)) = images
        .iter()
        .find(|(lo, hi)| !(lo.is_finite() && hi.is_finite() && lo < hi))
    {
        return Err(format!(
            "its inverse takes the saturated forward to [{lo}, {hi}], not a finite clamp image"
        ));
    }
    let saturated = x
        .iter()
        .zip(images)
        .any(|(z, (lo, hi))| !(z.f64() > *lo && z.f64() < *hi));
    if saturated {
        return Ok(());
    }
    let y = eval_all(link.forward, x, values);
    let back = eval_all(link.inverse, &y, values);
    let again = eval_all(link.forward, &back, values);
    let tol = EPS_PHYS.number() * F::EPS;
    let err = y
        .iter()
        .zip(&again)
        .map(|(a, b)| (a.f64() - b.f64()).abs())
        .fold(0.0, f64::max);
    if err <= tol {
        Ok(())
    } else {
        Err(format!(
            "its inverse returns z = {:?} as {:?}, whose forward is {err:e} from the forward's, past ε_phys = {tol:e}",
            floats(x),
            floats(&back)
        ))
    }
}

/// The forward's central difference along control `k` at `x`, at the step `h` times `k_h`: the column, the step
/// actually taken and the largest output magnitude at its two ends.
fn column<F: Fl>(
    link: &Link,
    x: &[F],
    k: usize,
    k_h: f64,
    values: &Values,
) -> (Vec<f64>, f64, f64) {
    let z = x[k].f64();
    let h = F::of(k_h * F::EPS.cbrt() * z.abs().max(1.0));
    let (mut up, mut down) = (x.to_vec(), x.to_vec());
    up[k] = x[k] + h;
    down[k] = x[k] - h;
    let step = up[k].f64() - down[k].f64();
    let (a, b) = (
        eval_all(link.forward, &up, values),
        eval_all(link.forward, &down, values),
    );
    let col = a
        .iter()
        .zip(&b)
        .map(|(p, m)| (p.f64() - m.f64()) / step)
        .collect();
    let scale = a
        .iter()
        .chain(&b)
        .map(|v| v.f64().abs())
        .fold(0.0, f64::max);
    (col, step, scale)
}

/// The differenced volume factor at the step `h` times `k_h`, `|f'|` for one control and `√det(JᵀJ)` for two, and its
/// rounding: each forward value within `ε_phys·max(1, |f|)` (the module's note).
pub fn volume<F: Fl>(link: &Link, x: &[F], k_h: f64, values: &Values) -> (f64, f64) {
    let eps = EPS_PHYS.number() * F::EPS;
    let cols: Vec<(Vec<f64>, f64, f64)> = (0..x.len())
        .map(|k| column(link, x, k, k_h, values))
        .collect();
    let norm = |c: &[f64]| c.iter().map(|v| v * v).sum::<f64>().sqrt();
    let round = |(_, step, scale): &(Vec<f64>, f64, f64)| 2.0 * eps * scale.max(1.0) / step;
    if cols.len() == 1 {
        (norm(&cols[0].0), round(&cols[0]))
    } else {
        let (a, b) = (&cols[0].0, &cols[1].0);
        let dot: f64 = a.iter().zip(b).map(|(p, q)| p * q).sum();
        let gram = (norm(a) * norm(a) * norm(b) * norm(b) - dot * dot).max(0.0);
        let (ea, eb) = (round(&cols[0]), round(&cols[1]));
        (gram.sqrt(), ea * norm(b) + norm(a) * eb + ea * eb)
    }
}

/// Requirement 3 at `F`: `exp(log_det)` finite and within the central difference's own error of the differenced
/// volume factor (the module's note).
pub fn c1<F: Fl>(link: &Link, x: &[F], values: &Values) -> Result<(), String> {
    let ld = eval(&link.log_det, x, values).f64();
    let from_ld = ld.exp();
    if ld.is_nan() || !from_ld.is_finite() {
        return Err(format!(
            "its log-det at z = {:?} is {ld}, not a finite derivative",
            floats(x)
        ));
    }
    let (fd, round) = volume(link, x, 1.0, values);
    let (fd2, round2) = volume(link, x, 2.0, values);
    let tol = (fd - fd2).abs() + round + round2 + EPS_PHYS.number() * F::EPS * from_ld;
    if (from_ld - fd).abs() <= tol {
        Ok(())
    } else {
        Err(format!(
            "its log-det's derivative {from_ld:e} at z = {:?} is not the forward's central difference {fd:e} \
             (within {tol:e})",
            floats(x)
        ))
    }
}

/// The problems of entry `link` onto `codomain` at the float type `F`, its tree parameters `values`: requirement 1 at
/// every fuzzed control, and with `all`, requirements 2 and 3 too. Each line names the requirement; the caller names
/// the entry and the parameter's value.
fn at<F: Fl>(link: &Link, codomain: Codomain, values: &Values, all: bool) -> Vec<String> {
    let n = codomain.controls() as usize;
    let images = clamp_images::<F>(link, n, values);
    let mut bad = Vec::new();
    for z in controls(codomain.controls()) {
        let x: Vec<F> = z.iter().map(|&v| F::of(v)).collect();
        let mut fails = vec![(
            "constraint-preserving",
            constraint(link, codomain, &x, values),
        )];
        if all {
            fails.push((
                "a conditioned inverse",
                round_trip(link, &x, &images, values),
            ));
            fails.push(("C¹", c1(link, &x, values)));
        }
        for (what, r) in fails {
            if let Err(e) = r {
                bad.push(format!("{what} fails at {}: {e}", F::NAME));
            }
        }
        if !bad.is_empty() {
            break;
        }
    }
    bad
}

/// The refusal line of entry `entry`, its requirement failure `why`, with its tunable parameter's value if any.
fn refusal(entry: &str, param: Option<(&str, f64)>, why: &str) -> String {
    match param {
        Some((name, v)) => format!(
            "link `{entry}`: with `{name}` = {v}, {why} (R-481; chart_decoder_contract Part 2.5)"
        ),
        None => format!("link `{entry}`: {why} (R-482 C1; chart_decoder_contract Part 2.5)"),
    }
}

/// The problems of entry `l` onto `c` under its declarations `rows` ([`problems`]).
fn entry(l: &Link, c: Codomain, rows: &[Tunable]) -> Vec<String> {
    let declared = [l.clamps, l.params].concat();
    let values = Values {
        declared: &declared,
        active: &[],
    };
    let mut bad = Vec::new();
    if rows.is_empty() {
        for why in at::<f64>(l, c, &values, false)
            .into_iter()
            .chain(at::<f32>(l, c, &values, false))
        {
            bad.push(refusal(l.name, None, &why));
        }
        return bad;
    }
    for t in rows {
        for v in range_values(t.lo, t.hi) {
            let active = [Param {
                name: t.param,
                value: v,
            }];
            let values = Values {
                active: &active,
                ..values
            };
            let found: Vec<String> = at::<f64>(l, c, &values, true)
                .into_iter()
                .chain(at::<f32>(l, c, &values, true))
                .collect();
            if let Some(why) = found.first() {
                bad.push(refusal(l.name, Some((t.param, v)), why));
                break;
            }
        }
    }
    bad
}

/// An entry, its codomain and its declarations, with the problems [`entry`] found in them.
type Checked = ((Link, Codomain, Vec<Tunable>), Vec<String>);

/// The problems of the built registry `built` under the tunable declarations `tunables`: requirement 1 for every entry
/// at its declared values, and the three requirements for each declared range's ends and interior sample, at f32 and
/// f64 (the module's note). Each line names the entry, the parameter and its value, and the requirement. Each entry's
/// problems are a function of the entry, its codomain and its declarations alone, so they are kept and reused when the
/// same three are checked again, as every edited registry a generation test builds is, in all its other entries.
pub fn problems(built: &[(Link, Codomain)], tunables: &[Tunable]) -> Vec<String> {
    static CHECKED: Mutex<Vec<Checked>> = Mutex::new(Vec::new());
    let mut bad = Vec::new();
    for (l, c) in built {
        let rows: Vec<Tunable> = tunables
            .iter()
            .filter(|t| t.entry == l.name)
            .copied()
            .collect();
        let key = (*l, *c, rows);
        let kept = CHECKED.lock().ok().and_then(|k| {
            k.iter()
                .find(|(k, _)| *k == key)
                .map(|(_, found)| found.clone())
        });
        let found = kept.unwrap_or_else(|| {
            let found = entry(l, *c, &key.2);
            if let Ok(mut k) = CHECKED.lock() {
                k.push((key, found.clone()));
            }
            found
        });
        bad.extend(found);
    }
    bad
}
