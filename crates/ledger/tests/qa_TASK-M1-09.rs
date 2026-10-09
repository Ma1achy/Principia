//! QA tests for TASK-M1-09 on the ledger side, written from the requirements and their sources, not from the
//! implementation. Which fields are numeric is decided here from the ledger's §3.8 metadata alone (a scalar field of
//! scale `lin`, `log`, `cyclic` or `diverging` with no ledger `floor`; the drifts, which have one, keep R-381's view),
//! never from the emitter's own classification.
//! - REQ-RENDER-022 (render_gui_spec §10.1; R-114, R-136; RQ-231): every numeric view's checked-in `colour()` is the
//!   template: `let raw`, the bitcast guard against the canonical quiet NaN (lowering Part 3a) returning
//!   `debug_invalid(ctx.frag_xy)`, only a stored sentinel's line between, then `ramp(range_norm(raw', lo, hi,
//!   RANGE_AUTO, u_range))`; no self-comparison and no `isnan` anywhere; `[lo, hi]` the ledger range's finite ends, a
//!   step index's `[0, horizon_steps]`, symmetric for a diverging field; the ramp `ramp_twilight` for a cyclic field,
//!   else `ramp_viridis`; `RANGE_AUTO` defaults to 1 exactly for an unbounded range (a cyclic field on its fixed
//!   [0, 1] aside); setting the node's `RANGE_AUTO` param writes `@uniform RANGE_AUTO: u32 = <0|1> [0, 1]`, which
//!   `Declaration::parse` gives back, and nothing else in the view changes.
//! - REQ-TOOL-160 (R-71): the log fields without a ledger `floor` are exactly the statement's six, and each view
//!   places raw at `dbg_log`'s form `1 − 1/(1 + ln(1 + |x|/ε))` on the fixed [0, 1], with the proposed ε = 2⁻²⁴.
//! - REQ-TOOL-161 (R-72): render_gui_spec §10.1 gives the cyclic period, 2π, and the cyclic view places raw at
//!   `fract(raw / 2π)` on the fixed [0, 1] with `ramp_twilight`.
//! - REQ-GEN-012 / REQ-TOOL-137 (R-136, R-271, R-280, R-343 item 5): a stored sentinel's line shows the compacted
//!   value through `dbg_sentinel` (R-381); `d_min`'s unset value, f16 +∞, is tested by its bits and drawn in
//!   `DBG_NOT_YET`, and no other view draws the grey.
//!
//! Each test has a registered negative control (R-176).

use std::collections::BTreeSet;
use std::path::PathBuf;

use ledger::gen::catalogue::numeric_view;
use ledger::schema::{Bound, Entry, FieldType, Scale};
use validation::{negative_control, Declaration, UniformType};

/// Lowering Part 3a's canonical quiet NaN.
const QNAN: u32 = 0x7FC0_0000;

/// f32 +∞'s bits: f16 +∞ (0x7C00, R-271) widened by `unpack2x16float`.
const F32_INF: u32 = 0x7F80_0000;

/// REQ-TOOL-160's statement: the log-scaled numeric fields with no ledger `floor`.
const LOG_FIELDS: [&str; 6] = [
    "d_min",
    "dE_max",
    "dLz_max",
    "closure_min",
    "rho_ratio",
    "r_min_pair_0",
];

/// REQ-TOOL-160's proposed ε: 2⁻²⁴.
fn eps() -> f32 {
    2f32.powi(-24)
}

fn entries() -> Vec<Entry> {
    ledger::gen::validate(&ledger::layout()).unwrap_or_else(|e| panic!("{e}"))
}

/// Whether the template colours `e`, from its §3.8 metadata alone.
fn is_numeric(e: &Entry) -> bool {
    !matches!(e.ty, FieldType::Vector { .. })
        && e.floor.is_none()
        && matches!(
            e.scale,
            Scale::Lin | Scale::Log | Scale::Cyclic | Scale::Diverging
        )
}

fn finite(b: Bound) -> Option<f64> {
    match b {
        Bound::Closed(v) | Bound::Open(v) if v.is_finite() => Some(v),
        _ => None,
    }
}

/// A step index (render_gui_spec §10.1: "a `u16` step index `[0, horizon_steps]`").
fn is_step_index(e: &Entry) -> bool {
    e.name.ends_with("_step")
}

fn generated_path(field: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../render/frag/debug/generated")
        .join(format!("{field}.wgsl"))
}

/// The checked-in view of `field`.
fn checked_in(field: &str) -> String {
    let p = generated_path(field);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// `s` without whitespace.
fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
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

/// The text of the call `name(...)`'s arguments, split at its top-level commas.
fn call_args(text: &str, name: &str) -> Option<Vec<String>> {
    let at = text.find(&format!("{name}("))? + name.len() + 1;
    let mut depth = 0i32;
    let mut args = vec![String::new()];
    for c in text[at..].chars() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(args.iter().map(|a| a.trim().to_owned()).collect()),
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        args.last_mut().unwrap().push(c);
    }
    None
}

/// Every comparison in `wgsl` whose two operands are the same text: a self-comparison, which fast-math may fold away
/// (R-114).
fn self_comparisons(wgsl: &str) -> Vec<String> {
    let b = wgsl.as_bytes();
    let mut out = Vec::new();
    for op in ["!=", "=="] {
        for (at, _) in wgsl.match_indices(op) {
            let mut depth = 0i32;
            let mut l = at;
            while l > 0 {
                let c = b[l - 1];
                match c {
                    b')' => depth += 1,
                    b'(' if depth == 0 => break,
                    b'(' => depth -= 1,
                    b',' | b'&' | b'|' | b'{' | b';' | b'\n' if depth == 0 => break,
                    _ => {}
                }
                l -= 1;
            }
            let mut depth = 0i32;
            let mut r = at + 2;
            while r < b.len() {
                let c = b[r];
                match c {
                    b'(' => depth += 1,
                    b')' if depth == 0 => break,
                    b')' => depth -= 1,
                    b',' | b'&' | b'|' | b'{' | b';' | b'\n' if depth == 0 => break,
                    _ => {}
                }
                r += 1;
            }
            let (lhs, rhs) = (squash(&wgsl[l..at]), squash(&wgsl[at + 2..r]));
            if !lhs.is_empty() && lhs == rhs {
                out.push(format!("{lhs} {op} {rhs}"));
            }
        }
    }
    out
}

/// A WGSL f32 literal's value.
fn lit(s: &str) -> f32 {
    s.trim()
        .trim_end_matches('f')
        .parse::<f32>()
        .unwrap_or_else(|e| panic!("`{s}` is no f32 literal: {e}"))
}

/// The `RANGE_AUTO` default `e`'s view must carry: 1 for an unbounded ledger range, 0 for a bounded one, and 0 for a
/// cyclic field, which sits on its fixed [0, 1] (RQ-231; render_gui_spec §10.1 as built).
fn range_auto_default(e: &Entry) -> f64 {
    let bounded = finite(e.range.lo).is_some() && finite(e.range.hi).is_some();
    if bounded || e.scale == Scale::Cyclic {
        0.0
    } else {
        1.0
    }
}

/// Checks `wgsl`, the view of `e`, against the template (REQ-RENDER-022) and its scale's `raw` (RQ-231;
/// REQ-TOOL-160, REQ-TOOL-161).
fn check_template(e: &Entry, wgsl: &str) {
    let f = e.name;
    let selfs = self_comparisons(wgsl);
    assert!(
        selfs.is_empty(),
        "`{f}`'s view has a self-comparison: {selfs:?}"
    );
    assert!(!wgsl.contains("isnan"), "`{f}`'s view tests NaN with isnan");

    // The header: RANGE_AUTO and u_range, and nothing else.
    let decl = Declaration::parse(wgsl).unwrap_or_else(|e| panic!("`{f}`: {e:?}"));
    let names: Vec<&str> = decl.uniforms.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, ["RANGE_AUTO", "u_range"], "`{f}`'s view's uniforms");
    let ra = &decl.uniforms[0];
    assert!(
        ra.ty == UniformType::U32 && ra.range == Some((0.0, 1.0)),
        "`{f}`'s RANGE_AUTO is not a u32 in [0, 1]: {ra:?}"
    );
    assert_eq!(
        ra.default,
        vec![range_auto_default(e)],
        "`{f}`'s RANGE_AUTO default for its ledger range {:?}",
        e.range
    );
    assert_eq!(
        decl.uniforms[1].ty,
        UniformType::Vec2,
        "`{f}`'s u_range type"
    );

    // The body: `let raw`, the guard, a sentinel's line, the ramp.
    let lines = body(wgsl);
    assert!(
        lines.len() >= 3 && lines[0].starts_with("let raw = "),
        "`{f}`'s colour() does not bind raw first: {lines:?}"
    );
    assert_eq!(
        squash(&lines[1]),
        squash(&format!(
            "if (bitcast<u32>(raw) == {QNAN:#010x}u) {{ return debug_invalid(ctx.frag_xy); }}"
        )),
        "`{f}`'s first line after raw is not the bitcast NaN guard returning the hatch"
    );
    let last = lines.last().unwrap();
    let ramp = if e.scale == Scale::Cyclic {
        "ramp_twilight"
    } else {
        "ramp_viridis"
    };
    assert!(
        last.starts_with(&format!("return {ramp}(range_norm(")),
        "`{f}`'s last line is not `return {ramp}(range_norm(…))`: {last}"
    );
    let args = call_args(last, "range_norm").unwrap_or_else(|| panic!("`{f}`: {last}"));
    assert_eq!(args.len(), 5, "`{f}`'s range_norm arguments: {args:?}");
    assert!(
        args[3].contains("RANGE_AUTO"),
        "`{f}`'s range_norm is not switched by RANGE_AUTO: {args:?}"
    );
    let middle = &lines[2..lines.len() - 1];
    let lets: Vec<&String> = middle.iter().filter(|l| l.starts_with("let ")).collect();
    let meas_ok = args[4].contains("u_range") || lets.iter().all(|l| l.contains("u_range"));
    assert!(meas_ok, "`{f}`'s measured range is not u_range's: {args:?}");

    // raw' and the fixed [lo, hi], by scale.
    let raw = squash(&args[0]);
    match e.scale {
        Scale::Log => {
            let want = "1.0-1.0/(1.0+log(1.0+abs(raw)/";
            assert!(
                raw.starts_with(want) && raw.ends_with("))"),
                "`{f}`'s log view does not place raw at dbg_log's form: {raw}"
            );
            let e_lit = &raw[want.len()..raw.len() - 2];
            assert_eq!(
                lit(e_lit).to_bits(),
                eps().to_bits(),
                "`{f}`'s log floor ε is {e_lit}, not the proposed 2^-24"
            );
            assert!(
                lit(&args[1]) == 0.0 && lit(&args[2]) == 1.0,
                "`{f}`'s log view is not on the fixed [0, 1]: {args:?}"
            );
        }
        Scale::Cyclic => {
            let want = "fract(raw/";
            assert!(
                raw.starts_with(want) && raw.ends_with(')'),
                "`{f}`'s cyclic view does not place raw at fract(raw / period): {raw}"
            );
            let p = &raw[want.len()..raw.len() - 1];
            assert_eq!(
                lit(p).to_bits(),
                (std::f64::consts::TAU as f32).to_bits(),
                "`{f}`'s cyclic period is {p}, not 2π"
            );
            assert!(
                lit(&args[1]) == 0.0 && lit(&args[2]) == 1.0,
                "`{f}`'s cyclic view is not on the fixed [0, 1]: {args:?}"
            );
        }
        Scale::Lin => {
            assert_eq!(raw, "raw", "`{f}`'s lin view compacts raw");
            if is_step_index(e) {
                assert!(
                    lit(&args[1]) == 0.0 && squash(&args[2]).contains("horizon_steps"),
                    "`{f}`, a step index, is not on [0, horizon_steps]: {args:?}"
                );
            } else {
                for (k, (b, axis)) in [(e.range.lo, "x"), (e.range.hi, "y")]
                    .into_iter()
                    .enumerate()
                {
                    match finite(b) {
                        Some(v) => assert_eq!(
                            lit(&args[1 + k]),
                            v as f32,
                            "`{f}`'s fixed end {k} is not its ledger range's {v}: {args:?}"
                        ),
                        None => assert!(
                            squash(&args[1 + k]).contains(&format!("u_range.{axis}")),
                            "`{f}`'s unbounded end {k} is not the measured one: {args:?}"
                        ),
                    }
                }
            }
        }
        Scale::Diverging => {
            assert_eq!(raw, "raw", "`{f}`'s diverging view compacts raw");
            let (lo, hi) = (squash(&args[1]), squash(&args[2]));
            assert!(
                lo == format!("-{hi}") || lo == format!("-({hi})"),
                "`{f}`'s diverging range is not symmetric about 0: {args:?}"
            );
        }
        _ => unreachable!(),
    }

    // A stored sentinel's line (R-136, R-381) or d_min's unset line (R-280), and nothing else.
    let mut want_middle = 0;
    match e.sentinel {
        Some(s) if s.is_finite() => {
            want_middle += 1;
            let line = middle
                .iter()
                .find(|l| l.contains("dbg_sentinel("))
                .unwrap_or_else(|| {
                    panic!("`{f}`'s sentinel {s} has no dbg_sentinel line: {lines:?}")
                });
            let sarg = call_args(line, "dbg_sentinel").unwrap();
            assert_eq!(
                squash(&sarg[0]),
                raw,
                "`{f}`'s sentinel is not fed the compacted value (R-381): {line}"
            );
            assert!(
                line.contains(&format!("{s}")),
                "`{f}`'s sentinel line does not test its sentinel {s}: {line}"
            );
        }
        Some(_) => {
            want_middle += 1;
            let line = middle
                .iter()
                .find(|l| l.contains("DBG_NOT_YET"))
                .unwrap_or_else(|| {
                    panic!("`{f}`'s unset value is not drawn in the grey: {lines:?}")
                });
            assert_eq!(
                squash(line),
                squash(&format!(
                    "if (bitcast<u32>(raw) == {F32_INF:#010x}u) {{ return DBG_NOT_YET; }}"
                )),
                "`{f}`'s unset value is not tested by its bits (R-343 item 5)"
            );
        }
        None => {}
    }
    let others: Vec<&String> = middle.iter().filter(|l| !l.starts_with("let ")).collect();
    assert_eq!(
        others.len(),
        want_middle,
        "`{f}`'s colour() has lines beyond the template's: {lines:?}"
    );
    if f != "d_min" {
        assert!(
            !wgsl.contains("DBG_NOT_YET"),
            "`{f}`'s view draws the grey, which is d_min's unset value's alone"
        );
    }
}

/// Checks each numeric field's checked-in view, and that it is what the generator writes, `views` the views by field.
fn check_all(views: &[(Entry, String)]) {
    assert!(views.len() >= 20, "only {} numeric fields", views.len());
    for (e, wgsl) in views {
        check_template(e, wgsl);
    }
}

fn numeric_views() -> Vec<(Entry, String)> {
    let l = ledger::layout();
    let es = entries();
    es.iter()
        .filter(|e| is_numeric(e))
        .map(|e| {
            let gen = numeric_view(&l.words, &es, e.name, None)
                .unwrap_or_else(|| panic!("`{}` is numeric but has no numeric view", e.name));
            let file = checked_in(e.name);
            assert_eq!(
                gen, file,
                "`{}`'s checked-in view is not the generator's",
                e.name
            );
            (e.clone(), file)
        })
        .collect()
}

#[test]
fn qa_numeric_view_template_every_numeric_view_is_the_template() {
    check_all(&numeric_views());
}

negative_control!(
    qa_numeric_view_template_every_numeric_view_is_the_template,
    "a view guarded by `raw != raw` instead of the bitcast is caught",
    expected = "has a self-comparison",
    {
        let views: Vec<(Entry, String)> = numeric_views()
            .into_iter()
            .map(|(e, w)| {
                let w = w.replace("bitcast<u32>(raw) == 0x7fc00000u", "raw != raw");
                (e, w)
            })
            .collect();
        check_all(&views)
    }
);

/// More controls: each leg of the template can fail.
#[cfg(feature = "controls")]
mod qa_numeric_view_template_controls {
    use super::*;

    fn mutated(field: &str, from: &str, to: &str) -> Vec<(Entry, String)> {
        numeric_views()
            .into_iter()
            .map(|(e, w)| {
                let w = if e.name == field {
                    assert!(w.contains(from), "`{field}` has no `{from}`");
                    w.replace(from, to)
                } else {
                    w
                };
                (e, w)
            })
            .collect()
    }

    mod guard_returns_ramp {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "a NaN guard that returns the ramp's start rather than the hatch is caught",
            expected = "is not the bitcast NaN guard",
            check_all(&mutated(
                "ftle",
                "return debug_invalid(ctx.frag_xy);",
                "return ramp_viridis(0.0);"
            ))
        );
    }

    mod log_eps {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "a log view with ε = 2^-23 is caught",
            expected = "not the proposed 2^-24",
            check_all(&mutated("closure_min", "5.9604645e-8", "1.1920929e-7"))
        );
    }

    mod cyclic_period {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "a cyclic view with period π is caught",
            expected = "not 2π",
            check_all(&mutated("rho_angle", "6.2831855", "3.1415927"))
        );
    }

    mod cyclic_ramp {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "a cyclic view on viridis is caught",
            expected = "is not `return ramp_twilight",
            check_all(&mutated("rho_angle", "ramp_twilight", "ramp_viridis"))
        );
    }

    mod unset_float_compare {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "d_min's unset value tested by a float comparison is caught",
            expected = "is not tested by its bits",
            check_all(&mutated(
                "d_min",
                "bitcast<u32>(raw) == 0x7f800000u",
                "raw == 1.0 / 0.0"
            ))
        );
    }

    mod step_range {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "t_end_step on its ledger [0, 65535] rather than [0, horizon_steps] is caught",
            expected = "is not on [0, horizon_steps]",
            check_all(&mutated(
                "t_end_step",
                "f32(ctx.params.horizon_steps)",
                "65535.0"
            ))
        );
    }

    mod masked {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "a view masking a failed state (validity masking, R-79) is caught",
            expected = "has lines beyond the template's",
            check_all(&mutated(
                "dE_max",
                "    return ramp_viridis(",
                "    if (ctx.sample.state == 4u) { return debug_invalid(ctx.frag_xy); }\n    return ramp_viridis("
            ))
        );
    }

    mod range_auto_default {
        use super::*;
        negative_control!(
            qa_numeric_view_template_every_numeric_view_is_the_template,
            "an unbounded field whose RANGE_AUTO defaults to 0 is caught",
            expected = "RANGE_AUTO default for its ledger range",
            check_all(&mutated(
                "ftle",
                "@uniform RANGE_AUTO: u32 = 1",
                "@uniform RANGE_AUTO: u32 = 0"
            ))
        );
    }
}

// ── REQ-TOOL-160: the log fields, ε ───────────────────────────────────────────────────────────────────────────────

/// The numeric log fields are exactly REQ-TOOL-160's six, and the drifts (with a ledger floor) are no numeric view.
fn check_log_fields(es: &[Entry]) {
    let got: BTreeSet<&str> = es
        .iter()
        .filter(|e| is_numeric(e) && e.scale == Scale::Log)
        .map(|e| e.name)
        .collect();
    let want: BTreeSet<&str> = LOG_FIELDS.into_iter().collect();
    assert_eq!(
        got, want,
        "the log fields with no ledger floor are not REQ-TOOL-160's six"
    );
    let l = ledger::layout();
    for drift in ["energy_drift", "Lz_drift"] {
        assert!(
            numeric_view(&l.words, es, drift, None).is_none(),
            "`{drift}` takes the template, not R-381's view"
        );
    }
}

#[test]
fn qa_numeric_view_template_log_fields_are_req_tool_160s() {
    check_log_fields(&entries());
}

negative_control!(
    qa_numeric_view_template_log_fields_are_req_tool_160s,
    "a seventh log field is caught",
    expected = "are not REQ-TOOL-160's six",
    {
        let mut es = entries();
        let k = es.iter().position(|e| e.name == "ftle").unwrap();
        es[k].scale = Scale::Log;
        check_log_fields(&es)
    }
);

// ── REQ-TOOL-161: render_gui_spec §10.1 gives the period ──────────────────────────────────────────────────────────

/// render_gui_spec §10.1's text.
fn spec_10_1() -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/gui/principia_render_gui_spec.md");
    let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let start = text
        .find("### 10.1 The shared prelude library")
        .expect("§10.1");
    let rest = &text[start + 4..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end].to_owned()
}

fn check_period_defined(section: &str) {
    let flat = section.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        flat.contains("cyclic period is one turn, 2π") && flat.contains("radians"),
        "render_gui_spec §10.1 does not give the cyclic period 2π, in radians"
    );
    assert!(
        flat.contains("REQ-TOOL-161"),
        "render_gui_spec §10.1's period does not cite REQ-TOOL-161"
    );
}

#[test]
fn qa_numeric_view_template_spec_defines_the_cyclic_period() {
    check_period_defined(&spec_10_1());
}

negative_control!(
    qa_numeric_view_template_spec_defines_the_cyclic_period,
    "§10.1 without the period is caught",
    expected = "does not give the cyclic period",
    check_period_defined(&spec_10_1().replace("2π", "τ/2"))
);

// ── REQ-RENDER-022: RANGE_AUTO, node param ↔ code ─────────────────────────────────────────────────────────────────

/// For every numeric field and each value of the param, the generated header's `RANGE_AUTO` default is the param, read
/// back by `Declaration::parse`; the two views differ in that line alone.
fn check_range_auto(gen: impl Fn(&str, Option<bool>) -> Option<String>) {
    let es = entries();
    for e in es.iter().filter(|e| is_numeric(e)) {
        let f = e.name;
        let mut views = Vec::new();
        for on in [false, true] {
            let w = gen(f, Some(on)).unwrap_or_else(|| panic!("`{f}` has no view"));
            let d = Declaration::parse(&w).unwrap_or_else(|e| panic!("`{f}`: {e:?}"));
            let ra = d
                .uniforms
                .iter()
                .find(|u| u.name == "RANGE_AUTO")
                .unwrap_or_else(|| panic!("`{f}` declares no RANGE_AUTO"));
            assert_eq!(
                ra.default,
                vec![f64::from(u8::from(on))],
                "`{f}`'s RANGE_AUTO param {on} is not the code's default"
            );
            views.push(w);
        }
        let differ: Vec<(&str, &str)> = views[0]
            .lines()
            .zip(views[1].lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert!(
            views[0].lines().count() == views[1].lines().count()
                && differ.len() == 1
                && differ[0].0.contains("@uniform RANGE_AUTO"),
            "`{f}`: toggling RANGE_AUTO changes {differ:?}"
        );
    }
}

#[test]
fn qa_numeric_view_template_range_auto_param_round_trips() {
    let l = ledger::layout();
    let es = entries();
    check_range_auto(|f, on| numeric_view(&l.words, &es, f, on));
}

negative_control!(
    qa_numeric_view_template_range_auto_param_round_trips,
    "a generator ignoring the param is caught",
    expected = "is not the code's default",
    {
        let l = ledger::layout();
        let es = entries();
        check_range_auto(|f, _| numeric_view(&l.words, &es, f, Some(true)))
    }
);
