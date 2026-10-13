//! QA tests for TASK-M1-17 on the ledger side, written from R-400, R-403 and the requirements they add, not from the
//! implementation:
//! - REQ-GEN-033 (R-403; dd_generation_root §3.8): every `diverging` entry of the payload ledger declares a range that
//!   spans zero, `lo < 0 < hi`, either end possibly unbounded, and those entries are R-403's four (`energy_drift`,
//!   `Lz_drift`, `E_0`, `Lz_0`); `K_0` and `V_0` are `lin` over `[0, ∞)` and `(−∞, 0]`; the payload ledger itself,
//!   one entry retagged `diverging` over a range that does not span zero (`[0, ∞)`, `(−∞, 0]`, `[−1, 0]`, `(0, ∞)`),
//!   fails generation naming the field and its range, and retagged over a range that spans zero (`(−∞, ∞)`, `[−1, 1]`)
//!   generates (`qa_diverging_range_*`).
//! - REQ-RENDER-084 (R-400, R-403; render_gui_spec §10.1): each numeric view's checked-in `colour()`, evaluated here by
//!   a small interpreter of its own WGSL (not by the emitter's CPU twin) with `range_norm` as §10.1 defines it, puts 0
//!   at `t = ½` on `ramp_coolwarm` for a field whose declared range spans zero, in both modes and over an asymmetric
//!   measured range; a field that can't be negative is on `ramp_viridis`; `K_0` and `V_0` are on `ramp_viridis`,
//!   increasing in the value in both modes, both headers defaulting to `RANGE_AUTO = 1`, `t = 0` the measured minimum
//!   under auto and `K_0`'s 0 or `V_0`'s measured minimum under fixed, a positive `V_0` clamped to `t = 1`
//!   (`qa_numeric_view_template_*`).
//! - REQ-RENDER-084's ranges: the fields the corpus makes non-negative (render_gui_spec §10.1's `mᵢ ∈ [0,1]` and
//!   `|ρ| ∈ [0,1]`; dd_generation_root §3.4's `d_min > 0`, `closure_min`, `delta_E_max_abs` and `delta_Lz_max_abs`
//!   `≥ 0`; §3.6's log fields; dd_decoder §3.6's `K₀ = Σ‖pᵢ‖²/2mᵢ`, `virial_ratio = 2K₀/|V₀|`; R-62's magnitudes)
//!   declare `lo ≥ 0` and each is cited (`qa_declared_ranges_*`).
//!
//! Each test has a registered negative control (R-176).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ledger::gen::catalogue::numeric_view;
use ledger::schema::{Bound, Entry, FieldType, Ledger, Range, Scale};
use validation::{negative_control, Declaration};

// ── Shared ──────────────────────────────────────────────────────────────────────────────────────────────────────────

fn entries_of(ledger: &Ledger) -> Vec<Entry> {
    ledger::gen::validate(ledger).unwrap_or_else(|e| panic!("{e}"))
}

fn entries() -> Vec<Entry> {
    entries_of(&ledger::layout())
}

/// R-400's "spans zero": `lo < 0 < hi`, either end possibly unbounded; an end at 0, open or closed, does not.
fn spans_zero(r: &Range) -> bool {
    let below = match r.lo {
        Bound::Closed(v) | Bound::Open(v) => v < 0.0,
        Bound::Unbounded => true,
    };
    let above = match r.hi {
        Bound::Closed(v) | Bound::Open(v) => v > 0.0,
        Bound::Unbounded => true,
    };
    below && above
}

fn by_name<'a>(es: &'a [Entry], name: &str) -> &'a Entry {
    es.iter()
        .find(|e| e.name == name)
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"))
}

// ── REQ-GEN-033: the `diverging` tag agrees with the declared range ─────────────────────────────────────────────────

/// R-403's list of the `diverging` entries left once `K_0` and `V_0` are `lin`.
const DIVERGING: [&str; 4] = ["energy_drift", "Lz_drift", "E_0", "Lz_0"];

fn check_payload_tags(es: &[Entry]) {
    let mut seen = BTreeSet::new();
    for e in es.iter().filter(|e| e.scale == Scale::Diverging) {
        assert!(
            spans_zero(&e.range),
            "`{}` carries `diverging` over {:?}, a range that does not span zero (R-403)",
            e.name,
            e.range
        );
        seen.insert(e.name);
    }
    assert_eq!(
        seen,
        DIVERGING.into_iter().collect::<BTreeSet<_>>(),
        "the `diverging` entries are not R-403's four"
    );
    let k = by_name(es, "K_0");
    assert_eq!(k.scale, Scale::Lin, "`K_0`'s scale (R-403)");
    assert_eq!(
        (k.range.lo, k.range.hi),
        (Bound::Closed(0.0), Bound::Unbounded),
        "`K_0` is not declared [0, ∞) (R-403)"
    );
    let v = by_name(es, "V_0");
    assert_eq!(v.scale, Scale::Lin, "`V_0`'s scale (R-403)");
    assert_eq!(
        (v.range.lo, v.range.hi),
        (Bound::Unbounded, Bound::Closed(0.0)),
        "`V_0` is not declared (−∞, 0] (R-403)"
    );
}

#[test]
fn qa_diverging_range_payload_tags_agree_with_ranges() {
    check_payload_tags(&entries());
}

negative_control!(
    qa_diverging_range_payload_tags_agree_with_ranges,
    "the pre-R-403 tag, K_0 diverging over [0, ∞), is caught",
    expected = "a range that does not span zero",
    {
        let mut es = entries();
        es.iter_mut().find(|e| e.name == "K_0").unwrap().scale = Scale::Diverging;
        check_payload_tags(&es)
    }
);

/// The payload ledger with entry `name` retagged `diverging` over `range`.
fn retagged(name: &str, range: Range) -> Ledger {
    let mut l = ledger::layout();
    let e = l
        .entries
        .iter_mut()
        .find(|e| e.name == Some(name))
        .unwrap_or_else(|| panic!("no ledger entry `{name}`"));
    e.scale = Some(Scale::Diverging);
    e.range = Some(range);
    l
}

/// `s` with the minus sign, infinity and spacing written one way, to read a range's notation.
fn norm(s: &str) -> String {
    s.replace('−', "-")
        .replace("inf", "∞")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Generation of `l` fails, and its error names `field` and, where given, the range in interval notation.
fn check_refused(l: &Ledger, field: &str, range: Option<&str>) {
    let err = match ledger::gen::validate(l) {
        Ok(_) => panic!("generation was not refused for `{field}` over {range:?}"),
        Err(e) => e.to_string(),
    };
    let line = err
        .lines()
        .find(|line| line.contains(field) && line.contains("diverging"))
        .unwrap_or_else(|| panic!("the refusal does not name `{field}` and `diverging`: {err}"));
    if let Some(r) = range {
        assert!(
            norm(line).contains(&norm(r)),
            "the refusal of `{field}` does not name its range {r}: {line}"
        );
    }
}

const fn range(lo: Bound, hi: Bound) -> Range {
    Range { lo, hi }
}

#[test]
fn qa_diverging_range_one_signed_is_refused() {
    use Bound::{Closed, Open, Unbounded};
    check_refused(
        &retagged("K_0", range(Closed(0.0), Unbounded)),
        "K_0",
        Some("[0, ∞)"),
    );
    check_refused(
        &retagged("V_0", range(Unbounded, Closed(0.0))),
        "V_0",
        Some("(−∞, 0]"),
    );
    // The boundary: an end at 0, closed or open, does not span zero.
    check_refused(
        &retagged("q_mass", range(Closed(-1.0), Closed(0.0))),
        "q_mass",
        None,
    );
    check_refused(
        &retagged("q_mass", range(Open(0.0), Unbounded)),
        "q_mass",
        None,
    );
    check_refused(
        &retagged("q_mass", range(Unbounded, Open(0.0))),
        "q_mass",
        None,
    );
}

negative_control!(
    qa_diverging_range_one_signed_is_refused,
    "a diverging entry over (−∞, ∞) spans zero and is not refused",
    expected = "generation was not refused",
    check_refused(
        &retagged("K_0", range(Bound::Unbounded, Bound::Unbounded)),
        "K_0",
        None
    )
);

fn check_generates(l: &Ledger, what: &str) {
    if let Err(e) = ledger::gen::validate(l) {
        panic!("{what} was refused: {e}");
    }
}

#[test]
fn qa_diverging_range_spanning_generates() {
    use Bound::{Closed, Open, Unbounded};
    check_generates(
        &retagged("q_mass", range(Unbounded, Unbounded)),
        "q_mass diverging over (−∞, ∞)",
    );
    check_generates(
        &retagged("q_mass", range(Closed(-1.0), Closed(1.0))),
        "q_mass diverging over [−1, 1]",
    );
    check_generates(
        &retagged("q_mass", range(Open(-1.0), Unbounded)),
        "q_mass diverging over (−1, ∞)",
    );
}

negative_control!(
    qa_diverging_range_spanning_generates,
    "a diverging entry over [0, 1] is refused",
    expected = "was refused",
    check_generates(
        &retagged("q_mass", range(Bound::Closed(0.0), Bound::Closed(1.0))),
        "q_mass diverging over [0, 1]"
    )
);

// ── REQ-RENDER-084: the declared non-negative ranges ────────────────────────────────────────────────────────────────

/// The fields the corpus or their definition make non-negative, as QA reads the sources (not the implementation's
/// list): render_gui_spec §10.1 (`mᵢ ∈ [0,1]`, `|ρ| ∈ [0,1]`); dd_generation_root §3.4 (`d_min > 0`, `closure_min ≥ 0`,
/// `delta_E_max_abs ≥ 0`, `delta_Lz_max_abs ≥ 0`) and §3.6 (log fields, so `> 0`: `rho_ratio`, `r_min_pair_0`);
/// dd_decoder §3.6 (`K₀ = Σ‖pᵢ‖²/2mᵢ ≥ 0`, `virial_ratio = 2K₀/|V₀| ≥ 0`); R-62 (`lambda_mag`, a magnitude).
const NON_NEGATIVE: [&str; 13] = [
    "m0",
    "m1",
    "m2",
    "rho_mag",
    "lambda_mag",
    "d_min",
    "closure_min",
    "dE_max",
    "dLz_max",
    "rho_ratio",
    "r_min_pair_0",
    "K_0",
    "virial_ratio",
];

fn lower(e: &Entry) -> Option<f64> {
    match e.range.lo {
        Bound::Closed(v) | Bound::Open(v) => Some(v),
        Bound::Unbounded => None,
    }
}

fn check_non_negative(es: &[Entry], cited: &[(&str, &str)]) {
    for name in NON_NEGATIVE {
        let e = by_name(es, name);
        let lo = lower(e);
        assert!(
            lo.is_some_and(|v| v >= 0.0),
            "`{name}`, non-negative by the corpus, declares no lower bound ≥ 0: {:?}",
            e.range
        );
        assert!(
            !spans_zero(&e.range),
            "`{name}`'s declared range spans zero, so its view would be on cool-warm"
        );
        let c = cited
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| *c)
            .unwrap_or_else(|| panic!("`{name}`'s declared lower bound has no citation"));
        assert!(
            c.contains('§') || c.contains("R-"),
            "`{name}`'s citation names no section or ruling: {c}"
        );
    }
    // The mass fractions' whole declared domain is [0, 1] (render_gui_spec §10.1).
    for m in ["m0", "m1", "m2"] {
        let e = by_name(es, m);
        assert_eq!(
            (e.range.lo, e.range.hi),
            (Bound::Closed(0.0), Bound::Closed(1.0)),
            "`{m}` is not declared [0, 1]"
        );
    }
    // V_0 can't be positive (dd_decoder §3.6: `V₀ = −Σ mᵢmⱼ/‖rᵢ − rⱼ‖`).
    let v = by_name(es, "V_0");
    assert!(
        matches!(v.range.hi, Bound::Closed(h) | Bound::Open(h) if h <= 0.0),
        "`V_0` declares no upper bound ≤ 0: {:?}",
        v.range
    );
}

#[test]
fn qa_declared_ranges_non_negative_fields() {
    check_non_negative(&entries(), &ledger::payload::NON_NEGATIVE);
}

negative_control!(
    qa_declared_ranges_non_negative_fields,
    "the ledger's unbounded default left on virial_ratio is caught",
    expected = "`virial_ratio`, non-negative by the corpus, declares no lower bound",
    {
        let mut es = entries();
        es.iter_mut()
            .find(|e| e.name == "virial_ratio")
            .unwrap()
            .range = range(Bound::Unbounded, Bound::Unbounded);
        check_non_negative(&es, &ledger::payload::NON_NEGATIVE)
    }
);

// ── REQ-RENDER-084: the views, evaluated ────────────────────────────────────────────────────────────────────────────

/// A value of the small WGSL subset the generated views' bodies use.
#[derive(Clone, Copy, Debug, PartialEq)]
enum V {
    S(f64),
    V2(f64, f64),
    B(bool),
}

impl V {
    fn s(self) -> f64 {
        match self {
            V::S(x) => x,
            v => panic!("not a scalar: {v:?}"),
        }
    }
}

/// render_gui_spec §10.1: `clamp((x − l)/(h − l), 0, 1)`, `(l, h) = select((lo, hi), meas, auto)`, a degenerate
/// range reading 0.
fn range_norm_spec(x: f64, lo: f64, hi: f64, auto: bool, meas: (f64, f64)) -> f64 {
    let (l, h) = if auto { meas } else { (lo, hi) };
    if h == l {
        0.0
    } else {
        ((x - l) / (h - l)).clamp(0.0, 1.0)
    }
}

struct Eval<'a> {
    toks: Vec<String>,
    at: usize,
    env: &'a BTreeMap<String, V>,
}

fn tokens(s: &str) -> Vec<String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit()
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < c.len()
                && (c[i].is_ascii_alphanumeric()
                    || c[i] == '.'
                    || ((c[i] == '-' || c[i] == '+') && matches!(c[i - 1], 'e' | 'E')))
            {
                i += 1;
            }
            out.push(c[start..i].iter().collect());
        } else if ch.is_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_' || c[i] == '.') {
                i += 1;
            }
            out.push(c[start..i].iter().collect());
        } else if (ch == '!' || ch == '=' || ch == '<' || ch == '>') && c.get(i + 1) == Some(&'=') {
            out.push(c[i..i + 2].iter().collect());
            i += 2;
        } else {
            out.push(ch.to_string());
            i += 1;
        }
    }
    out
}

impl Eval<'_> {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.at).map(String::as_str)
    }

    fn next(&mut self) -> String {
        let t = self.toks[self.at].clone();
        self.at += 1;
        t
    }

    fn expect(&mut self, t: &str) {
        let got = self.next();
        assert_eq!(got, t, "WGSL: expected `{t}`");
    }

    fn cmp(&mut self) -> V {
        let a = self.sum();
        match self.peek() {
            Some("!=") => {
                self.next();
                V::B(a.s() != self.sum().s())
            }
            Some("==") => {
                self.next();
                V::B(a.s() == self.sum().s())
            }
            _ => a,
        }
    }

    fn sum(&mut self) -> V {
        let mut a = self.product();
        while let Some(op @ ("+" | "-")) = self.peek() {
            let op = op.to_owned();
            self.next();
            let b = self.product().s();
            a = V::S(if op == "+" { a.s() + b } else { a.s() - b });
        }
        a
    }

    fn product(&mut self) -> V {
        let mut a = self.unary();
        while let Some(op @ ("*" | "/")) = self.peek() {
            let op = op.to_owned();
            self.next();
            let b = self.unary().s();
            a = V::S(if op == "*" { a.s() * b } else { a.s() / b });
        }
        a
    }

    fn unary(&mut self) -> V {
        if self.peek() == Some("-") {
            self.next();
            return V::S(-self.unary().s());
        }
        self.primary()
    }

    fn args(&mut self) -> Vec<V> {
        self.expect("(");
        let mut out = Vec::new();
        if self.peek() == Some(")") {
            self.next();
            return out;
        }
        loop {
            out.push(self.cmp());
            match self.next().as_str() {
                "," => {}
                ")" => return out,
                t => panic!("WGSL: unexpected `{t}` in arguments"),
            }
        }
    }

    fn primary(&mut self) -> V {
        let t = self.next();
        if t == "(" {
            let v = self.cmp();
            self.expect(")");
            return v;
        }
        if t.chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '.')
        {
            let n = t.trim_end_matches(['u', 'i', 'f']);
            return V::S(
                n.parse()
                    .unwrap_or_else(|e| panic!("WGSL literal `{t}`: {e}")),
            );
        }
        if t == "vec2" {
            for k in ["<", "f32", ">"] {
                self.expect(k);
            }
            let a = self.args();
            return V::V2(a[0].s(), a[1].s());
        }
        if self.peek() == Some("(") {
            let a = self.args();
            return match t.as_str() {
                "abs" => V::S(a[0].s().abs()),
                "max" => V::S(a[0].s().max(a[1].s())),
                "min" => V::S(a[0].s().min(a[1].s())),
                "f32" => V::S(a[0].s()),
                "clamp" => V::S(a[0].s().clamp(a[1].s(), a[2].s())),
                "range_norm" => {
                    let V::B(auto) = a[3] else {
                        panic!("range_norm's auto is not a bool: {:?}", a[3])
                    };
                    let V::V2(ml, mh) = a[4] else {
                        panic!("range_norm's meas is not a vec2: {:?}", a[4])
                    };
                    V::S(range_norm_spec(
                        a[0].s(),
                        a[1].s(),
                        a[2].s(),
                        auto,
                        (ml, mh),
                    ))
                }
                f => panic!("WGSL: no `{f}` in the interpreter"),
            };
        }
        if let Some(v) = self.env.get(&t) {
            return *v;
        }
        if let Some((base, c)) = t.rsplit_once('.') {
            if let Some(V::V2(x, y)) = self.env.get(base) {
                return V::S(match c {
                    "x" => *x,
                    "y" => *y,
                    _ => panic!("WGSL: `{t}`"),
                });
            }
        }
        panic!("WGSL: unknown name `{t}`")
    }
}

fn eval(expr: &str, env: &BTreeMap<String, V>) -> V {
    let mut e = Eval {
        toks: tokens(expr),
        at: 0,
        env,
    };
    let v = e.cmp();
    assert_eq!(e.at, e.toks.len(), "WGSL: trailing tokens in `{expr}`");
    v
}

/// The statements of `colour()`'s body, trimmed.
fn body(wgsl: &str) -> Vec<String> {
    let start = wgsl
        .find("fn colour(ctx: Ctx) -> vec3<f32> {")
        .unwrap_or_else(|| panic!("no colour() in:\n{wgsl}"));
    wgsl[start..]
        .lines()
        .skip(1)
        .map(str::trim)
        .take_while(|l| *l != "}")
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A numeric view, run at one value: the ramp it returns and the place on it, for `raw = x`, `RANGE_AUTO = auto`,
/// `u_range = meas`, the guard lines skipped (`x` is finite and no sentinel).
fn run(wgsl: &str, x: f64, auto: bool, meas: (f64, f64)) -> (String, f64) {
    let mut env = BTreeMap::new();
    env.insert("raw".to_owned(), V::S(x));
    env.insert(
        "uniforms.RANGE_AUTO".to_owned(),
        V::S(f64::from(u8::from(auto))),
    );
    env.insert("uniforms.u_range".to_owned(), V::V2(meas.0, meas.1));
    env.insert("ctx.params.horizon_steps".to_owned(), V::S(1000.0));
    for line in body(wgsl).iter().skip(1) {
        if line.starts_with("if ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("let ") {
            let (name, expr) = rest.split_once('=').expect("a let binding");
            let v = eval(expr.trim().trim_end_matches(';'), &env);
            env.insert(name.trim().to_owned(), v);
            continue;
        }
        let ret = line
            .strip_prefix("return ")
            .and_then(|r| r.strip_suffix(';'))
            .unwrap_or_else(|| panic!("unexpected line `{line}`"));
        let (ramp, inner) = ret.split_once('(').expect("a ramp call");
        let inner = inner.strip_suffix(')').expect("the ramp call's close");
        return (ramp.to_owned(), eval(inner, &env).s());
    }
    panic!("colour() has no return")
}

fn checked_in(field: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../render/frag/debug/generated")
        .join(format!("{field}.wgsl"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// Every scalar field the template colours on a `lin` or `diverging` scale with no sentinel: its checked-in view,
/// which must be the generator's.
fn signed_or_lin_views() -> Vec<(Entry, String)> {
    let l = ledger::layout();
    let es = entries_of(&l);
    es.iter()
        .filter(|e| {
            !matches!(e.ty, FieldType::Vector { .. })
                && e.floor.is_none()
                && e.sentinel.is_none()
                && matches!(e.scale, Scale::Lin | Scale::Diverging)
        })
        .filter_map(|e| {
            let gen = numeric_view(&l.words, &es, e.name, None)?;
            let file = checked_in(e.name);
            assert_eq!(
                gen, file,
                "`{}`'s checked-in view is not the generator's",
                e.name
            );
            Some((e.clone(), file))
        })
        .collect()
}

const T_TOL: f64 = 1e-12;

/// Each view's ramp follows its declared range: a zero-spanning field is on `ramp_coolwarm` with 0 at ½ in both
/// modes over an asymmetric measured range, ±M at the ends; any other is on `ramp_viridis`, increasing.
fn check_ramps(views: &[(Entry, String)]) {
    let mut n_cool = 0;
    for (e, wgsl) in views {
        let f = e.name;
        if spans_zero(&e.range) {
            n_cool += 1;
            let meas = (-1.0, 4.0);
            for auto in [true, false] {
                let (ramp, t0) = run(wgsl, 0.0, auto, meas);
                assert_eq!(
                    ramp, "ramp_coolwarm",
                    "`{f}` spans zero but is on {ramp} (R-400)"
                );
                assert!(
                    (t0 - 0.5).abs() < T_TOL,
                    "`{f}`'s 0 is at t = {t0}, not ½ (auto {auto}; R-400)"
                );
                let (_, hi) = run(wgsl, 4.0, auto, meas);
                let (_, lo) = run(wgsl, -4.0, auto, meas);
                assert!(
                    (hi - 1.0).abs() < T_TOL && lo.abs() < T_TOL,
                    "`{f}`'s ±M are at {lo}, {hi}, not 0 and 1 over [−M, M] (auto {auto})"
                );
            }
        } else {
            let (ramp, _) = run(wgsl, 0.5, true, (0.0, 1.0));
            assert_eq!(
                ramp, "ramp_viridis",
                "`{f}`, whose range {:?} does not span zero, is on {ramp}",
                e.range
            );
            let ts: Vec<f64> = [0.1, 0.4, 0.8]
                .iter()
                .map(|&x| run(wgsl, x, true, (0.0, 1.0)).1)
                .collect();
            assert!(
                ts[0] < ts[1] && ts[1] < ts[2],
                "`{f}`'s view does not increase in the value: {ts:?}"
            );
        }
    }
    assert!(
        n_cool >= 3,
        "only {n_cool} zero-spanning lin or diverging views"
    );
}

#[test]
fn qa_numeric_view_template_ramp_follows_declared_range() {
    check_ramps(&signed_or_lin_views());
}

negative_control!(
    qa_numeric_view_template_ramp_follows_declared_range,
    "E_0 placed on its measured range rather than the symmetric one is caught",
    expected = "`E_0`'s 0 is at t =",
    {
        let views: Vec<(Entry, String)> = signed_or_lin_views()
            .into_iter()
            .map(|(e, w)| {
                let w = if e.name == "E_0" {
                    assert!(w.contains("vec2<f32>(-m, m)"));
                    w.replace("vec2<f32>(-m, m)", "uniforms.u_range")
                } else {
                    w
                };
                (e, w)
            })
            .collect();
        check_ramps(&views)
    }
);

/// `K_0` and `V_0` (R-403): `ramp_viridis`, both headers defaulting to `RANGE_AUTO = 1`, increasing in the value in
/// both modes; auto: `t = 0` the measured minimum; fixed: `K_0`'s declared 0 and `V_0`'s measured minimum, a positive
/// `V_0` clamped to `t = 1`.
fn check_k0_v0(k0: &str, v0: &str) {
    for (f, w) in [("K_0", k0), ("V_0", v0)] {
        let d = Declaration::parse(w).unwrap_or_else(|e| panic!("`{f}`: {e:?}"));
        let ra = d
            .uniforms
            .iter()
            .find(|u| u.name == "RANGE_AUTO")
            .unwrap_or_else(|| panic!("`{f}` has no RANGE_AUTO"));
        assert_eq!(ra.default, vec![1.0], "`{f}`'s RANGE_AUTO default (R-403)");
    }
    let at = |w: &str, x: f64, auto: bool, meas: (f64, f64)| {
        let (ramp, t) = run(w, x, auto, meas);
        assert_eq!(ramp, "ramp_viridis", "a one-signed view on {ramp} (R-403)");
        t
    };
    let close = |a: f64, b: f64| (a - b).abs() < T_TOL;
    // K_0, measured over [0.3, 2.3].
    let km = (0.3, 2.3);
    let (a0, a1, ah) = (
        at(k0, 0.3, true, km),
        at(k0, 2.3, true, km),
        at(k0, 1.3, true, km),
    );
    assert!(
        close(a0, 0.0) && close(a1, 1.0) && close(ah, 0.5),
        "`K_0` under auto: t(min) = {a0}, t(max) = {a1}, t(mid) = {ah}, not 0, 1, ½"
    );
    let (f0, f1, fh) = (
        at(k0, 0.0, false, km),
        at(k0, 2.3, false, km),
        at(k0, 1.15, false, km),
    );
    assert!(
        close(f0, 0.0) && close(f1, 1.0) && close(fh, 0.5),
        "`K_0` under fixed: t(0) = {f0}, t(max) = {f1}, t(max/2) = {fh}, not 0, 1, ½"
    );
    // V_0, measured over [−3, −1].
    let vm = (-3.0, -1.0);
    let (a0, a1, ah) = (
        at(v0, -3.0, true, vm),
        at(v0, -1.0, true, vm),
        at(v0, -2.0, true, vm),
    );
    assert!(
        close(a0, 0.0) && close(a1, 1.0) && close(ah, 0.5),
        "`V_0` under auto: its most negative value is at t = {a0}, its max at {a1}, mid {ah}; not 0, 1, ½"
    );
    let (f0, f1, fh, fp) = (
        at(v0, -3.0, false, vm),
        at(v0, 0.0, false, vm),
        at(v0, -1.5, false, vm),
        at(v0, 0.5, false, vm),
    );
    assert!(
        close(f0, 0.0) && close(f1, 1.0) && close(fh, 0.5),
        "`V_0` under fixed: t(min) = {f0}, t(0) = {f1}, t(min/2) = {fh}, not 0, 1, ½"
    );
    assert!(
        close(fp, 1.0),
        "`V_0` = +0.5 under fixed reads t = {fp}, not clamped to 1"
    );
}

#[test]
fn qa_numeric_view_template_k0_v0_viridis_both_modes() {
    check_k0_v0(&checked_in("K_0"), &checked_in("V_0"));
}

negative_control!(
    qa_numeric_view_template_k0_v0_viridis_both_modes,
    "V_0 mapped decreasing, its most negative value at the bright end, is caught",
    expected = "`V_0` under auto",
    {
        let v0 =
            checked_in("V_0").replace("ramp_viridis(range_norm(", "ramp_viridis(1.0 - range_norm(");
        check_k0_v0(&checked_in("K_0"), &v0)
    }
);

#[cfg(feature = "controls")]
mod qa_numeric_view_template_k0_on_coolwarm {
    use super::*;
    negative_control!(
        qa_numeric_view_template_k0_v0_viridis_both_modes,
        "K_0 on the pre-R-403 diverging view is caught",
        expected = "a one-signed view on ramp_coolwarm",
        {
            let k0 = checked_in("E_0").replace("ctx.sample.E_0", "ctx.ic.K_0");
            check_k0_v0(&k0, &checked_in("V_0"))
        }
    );
}
