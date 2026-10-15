//! QA tests for TASK-M2-31 on the ledger side, written from REQ-COL-070's statement and verify detail and their
//! sources (R-436; R-79, R-114; render_gui_spec §10.1; dd_decoder §3.6), not from the implementation:
//! - every generated numeric view's CPU twin (`NumericView::shown`) draws every non-finite f32, each NaN payload,
//!   quiet and signalling, of either sign, and `±∞`, in the invalid colour, in fixed and auto range alike, unless it is
//!   the field's declared unset value (`d_min`'s `+∞`, f32 bits `0x7F80_0000`, R-280, R-399), which keeps its "not yet"
//!   style; and no finite f32 (`±0`, `±` the smallest and largest subnormal, the smallest normal, `±` the largest
//!   finite, and every exponent below all ones) is drawn invalid or "not yet": the test is an exponent of all ones,
//!   no wider and no narrower (`qa_non_finite_twin_*`);
//! - the measured range (`u_range`, `NumericView::measured`) is taken over the finite samples: a buffer with `±∞` and
//!   NaN mixed in measures as its finite part alone, a `lin` field's as the finite values' own `(min, max)`, and a
//!   buffer of non-finite values only as an empty one, never with an infinite end (`qa_non_finite_measured_*`);
//! - every generated view's checked-in WGSL carries the bit test, `(bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u`
//!   returning `debug_invalid(ctx.frag_xy)`, exactly once, after the declared sentinel's or unset value's own line and
//!   before the ramp, with no arithmetic on `raw` before it; and no view carries `isinf`, an infinite literal (a float
//!   literal beyond f32's range, or a division by zero) or the largest finite value (`qa_non_finite_wgsl_*`).
//!
//! The sentinel each field declares is read from the ledger's own entry (`Entry::sentinel`), never from the emitter's
//! classification. Each test registers its negative control (R-176).

use ledger::gen::catalogue::numeric_view;
use ledger::gen::numeric::{NumericView, Shown};
use ledger::schema::{Entry, Scale};
use validation::negative_control;

/// f32 `+∞`'s bits: `d_min`'s unset value, f16 `+∞` (0x7C00, R-271) widened.
const POS_INF: u32 = 0x7F80_0000;
/// f32 `−∞`'s bits: `V_0`'s and `E_0`'s value at the collision pole (dd_decoder §3.6).
const NEG_INF: u32 = 0xFF80_0000;
/// An f32's exponent field (IEEE 754 binary32): all ones is non-finite.
const EXP: u32 = 0x7F80_0000;
const SIGN: u32 = 0x8000_0000;
const MANT: u32 = 0x007F_FFFF;

/// The read side's `horizon_steps` the tests draw with (the golden scenes' context).
const HORIZON: u32 = 1000;

fn entries() -> Vec<Entry> {
    ledger::gen::validate(&ledger::layout()).unwrap_or_else(|e| panic!("{e}"))
}

/// Every field the catalogue generates a numeric view for, with its CPU twin and its checked-in WGSL.
fn views() -> Vec<(Entry, NumericView, String)> {
    let l = ledger::layout();
    let es = entries();
    let out: Vec<(Entry, NumericView, String)> = es
        .iter()
        .filter_map(|e| {
            let gen = numeric_view(&l.words, &es, e.name, None)?;
            let n = NumericView::of(e)
                .unwrap_or_else(|| panic!("`{}` has a numeric view but no twin", e.name));
            let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../render/frag/debug/generated")
                .join(format!("{}.wgsl", e.name));
            let file =
                std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert_eq!(
                gen, file,
                "`{}`'s checked-in view is not the generator's",
                e.name
            );
            Some((e.clone(), n, file))
        })
        .collect();
    assert!(out.len() >= 30, "only {} numeric views", out.len());
    out
}

/// The non-finite f32 bit patterns probed: `±∞`, and NaN payloads of both signs, quiet (mantissa's top bit set) and
/// signalling (clear, mantissa non-zero), at the mantissa's edges and on a stride through it.
fn non_finite_bits() -> Vec<u32> {
    let mut m: Vec<u32> = vec![
        0, // ±∞
        1,
        2,
        0x0020_0000,
        0x003F_FFFF, // the largest signalling payload
        0x0040_0000, // the canonical quiet NaN
        0x0040_0001,
        0x0055_5555,
        0x007F_FFFE,
        MANT,
    ];
    m.extend((1..MANT).step_by(7919));
    m.iter().flat_map(|&m| [EXP | m, SIGN | EXP | m]).collect()
}

/// Finite f32 bit patterns at the edges: `±0`, `±` the smallest and largest subnormal, `±` the smallest normal,
/// `±` the largest finite and the one below it, and, for every exponent below all ones, its smallest and largest
/// mantissa of each sign.
fn finite_bits() -> Vec<u32> {
    let mut v: Vec<u32> = [0, 1, MANT, 0x0080_0000, 0x7F7F_FFFF, 0x7F7F_FFFE]
        .iter()
        .flat_map(|&b| [b, SIGN | b])
        .collect();
    for e in 0..255u32 {
        for m in [0, MANT] {
            v.push(e << 23 | m);
            v.push(SIGN | e << 23 | m);
        }
    }
    v
}

/// What `field`'s view must draw for a stored non-finite `bits`, from the ledger's entry alone: its unset value's
/// "not yet" style when `bits` is the field's declared non-finite sentinel, `d_min`'s `+∞` (R-280, R-399), else
/// the invalid colour (R-436).
fn want_non_finite(e: &Entry, bits: u32) -> Shown {
    match e.sentinel {
        Some(s) if !s.is_finite() && (s as f32).to_bits() == bits => Shown::NotYet,
        _ => Shown::Invalid,
    }
}

/// The `u_range`s the twin is drawn with: the default unit interval, and wide, narrow and reversed ones.
const U_RANGES: [[f64; 2]; 4] = [[0.0, 1.0], [-1e30, 1e30], [0.25, 0.25], [3.0, -2.0]];

/// `shown` the CPU twin under test: `NumericView::shown`, or a control's wrong one.
type Twin = fn(&NumericView, f32, bool, [f64; 2], u32) -> Shown;

fn twin(n: &NumericView, x: f32, auto: bool, u: [f64; 2], h: u32) -> Shown {
    n.shown(x, auto, u, h)
}

/// Checks `shown` over every numeric view: every non-finite value invalid but the declared unset value, which is
/// "not yet"; every finite one on the ramp (or its literal sentinel), at a finite `t` in `[0, 1]`.
fn check_twin(shown: Twin) {
    let nf = non_finite_bits();
    let fin = finite_bits();
    for (e, n, _) in views() {
        let f = e.name;
        for auto in [false, true] {
            for u in U_RANGES {
                for &b in &nf {
                    let x = f32::from_bits(b);
                    assert_eq!(
                        shown(&n, x, auto, u, HORIZON),
                        want_non_finite(&e, b),
                        "`{f}` draws the non-finite {b:#010x} (auto {auto}, u_range {u:?})"
                    );
                }
                for &b in &fin {
                    let x = f32::from_bits(b);
                    match shown(&n, x, auto, u, HORIZON) {
                        Shown::Ramp { t, .. } => assert!(
                            t.is_finite() && (0.0..=1.0).contains(&t),
                            "`{f}` places the finite {b:#010x} at t = {t}"
                        ),
                        Shown::Literal(_) if e.sentinel == Some(f64::from(x)) => {}
                        other => panic!(
                            "`{f}` draws the finite {b:#010x} ({x:e}) as {other:?}, not on its ramp"
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn qa_non_finite_twin_every_view_every_payload() {
    check_twin(twin);
}

/// The ±∞ sentinels the views declare are exactly `d_min`'s unset `+∞`: a new one has no drawn style here
/// (REQ-COL-071 defines R-418's), so it must not silently take the "not yet" style.
#[test]
fn qa_non_finite_twin_the_only_infinite_sentinel_is_d_mins_unset() {
    let got: Vec<(&str, u32)> = views()
        .iter()
        .filter_map(|(e, ..)| {
            e.sentinel
                .filter(|s| !s.is_finite())
                .map(|s| (e.name, (s as f32).to_bits()))
        })
        .collect();
    assert_eq!(got, [("d_min", POS_INF)], "the views' non-finite sentinels");
}

negative_control!(
    qa_non_finite_twin_every_view_every_payload,
    "a twin whose guard tests the canonical NaN alone (TASK-M1-09's) draws +inf on the ramp",
    expected = "draws the non-finite 0x7f800000",
    check_twin(|n, x, auto, u, h| {
        if x.to_bits() != 0x7FC0_0000 && x.is_nan() {
            // Keep every other NaN invalid, so the control isolates ±∞.
            return Shown::Invalid;
        }
        if x.is_infinite()
            && !matches!(n.sentinel, Some(ledger::gen::numeric::Sentinel::Unset { bits }) if bits == x.to_bits())
        {
            // ±∞ reaching the ramp, as the old guard let it: the compaction of the largest finite value.
            return n.shown(f32::MAX.copysign(x), auto, u, h);
        }
        n.shown(x, auto, u, h)
    })
);

/// More controls: the other ways the twin can be wrong.
#[cfg(feature = "controls")]
mod qa_non_finite_twin_controls {
    use super::*;

    mod guard_before_unset {
        use super::*;
        negative_control!(
            qa_non_finite_twin_every_view_every_payload,
            "a twin testing the non-finite guard before d_min's unset value draws its +inf invalid",
            expected = "`d_min` draws the non-finite 0x7f800000",
            check_twin(|n, x, auto, u, h| {
                if x.to_bits() & EXP == EXP {
                    return Shown::Invalid;
                }
                n.shown(x, auto, u, h)
            })
        );
    }

    mod guard_too_wide {
        use super::*;
        negative_control!(
            qa_non_finite_twin_every_view_every_payload,
            "a guard that also catches the largest finite value (a float comparison against it) is caught",
            expected = "draws the finite 0x7f7fffff",
            check_twin(|n, x, auto, u, h| {
                if x.to_bits() == 0x7F7F_FFFF {
                    return Shown::Invalid;
                }
                n.shown(x, auto, u, h)
            })
        );
    }

    mod signalling_nan_missed {
        use super::*;
        negative_control!(
            qa_non_finite_twin_every_view_every_payload,
            "a guard that misses a signalling NaN of negative sign is caught",
            expected = "draws the non-finite 0xff800001",
            check_twin(|n, x, auto, u, h| {
                if x.to_bits() == 0xFF80_0001 {
                    return n.shown(0.0, auto, u, h);
                }
                n.shown(x, auto, u, h)
            })
        );
    }
}

// ── The measured range over the finite samples ──────────────────────────────────────────────────────────────────

/// `measured` under test: `NumericView::measured`, or a control's wrong one.
type Measure = fn(&NumericView, &[f32]) -> [f64; 2];

fn measure(n: &NumericView, xs: &[f32]) -> [f64; 2] {
    n.measured(xs)
}

/// A field's finite sample values, of both signs, a subnormal among them.
const FINITE: [f32; 6] = [0.75, -1.5, 2.25, 0.0, 9.5, 1e-40];

/// `a` and `b` interleaved, the rest of the longer appended.
fn interleave(a: &[f32], b: &[f32]) -> Vec<f32> {
    (0..a.len().max(b.len()))
        .flat_map(|k| a.get(k).into_iter().chain(b.get(k)).copied())
        .collect()
}

/// Checks `measure` over every view: the buffer `FINITE` with every non-finite probe interleaved measures as `FINITE`
/// alone, bit for bit; a `lin` view's as `FINITE`'s own `(min, max)`, and, with `±∞` beside `∓` the largest finite
/// value, as those finite ends; a buffer of non-finite values only, as an empty one; and no end is ever non-finite.
fn check_measured(measure: Measure) {
    let nf: Vec<f32> = non_finite_bits()
        .into_iter()
        .step_by(37)
        .chain([POS_INF, NEG_INF])
        .map(f32::from_bits)
        .collect();
    for (e, n, _) in views() {
        let f = e.name;
        let fin: Vec<f32> = FINITE
            .into_iter()
            .filter(|&x| e.sentinel != Some(f64::from(x)))
            .collect();
        let mixed = interleave(&fin, &nf);
        assert_eq!(mixed.len(), fin.len() + nf.len(), "`{f}`: the mix");
        let (a, b) = (measure(&n, &mixed), measure(&n, &fin));
        assert!(
            a.iter().all(|v| v.is_finite()),
            "`{f}`'s measured range over finite values and ±inf has a non-finite end: {a:?}"
        );
        assert_eq!(
            a.map(f64::to_bits),
            b.map(f64::to_bits),
            "`{f}`'s measured range with ±inf and NaN mixed in is {a:?}, not the finite values' {b:?}"
        );
        if e.scale == Scale::Lin {
            let lo = fin.iter().copied().fold(f32::INFINITY, f32::min);
            let hi = fin.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert_eq!(
                a,
                [f64::from(lo), f64::from(hi)],
                "`{f}`, lin, measures the mix as {a:?}, not its finite values' (min, max)"
            );
            let edge = [
                f32::from_bits(NEG_INF),
                -f32::MAX,
                0.5,
                f32::MAX,
                f32::from_bits(POS_INF),
                f32::NAN,
            ];
            let got = measure(&n, &edge);
            assert_eq!(
                got,
                [-f64::from(f32::MAX), f64::from(f32::MAX)],
                "`{f}`, lin, measures -inf, -max, 0.5, max, +inf, NaN as {got:?}, not the finite values' (min, max)"
            );
        }
        let none = measure(&n, &nf);
        assert!(
            none.iter().all(|v| v.is_finite()),
            "`{f}`'s measured range over non-finite values only has a non-finite end: {none:?}"
        );
        assert_eq!(
            none.map(f64::to_bits),
            measure(&n, &[]).map(f64::to_bits),
            "`{f}`'s measured range over non-finite values only is {none:?}, not an empty buffer's"
        );
    }
}

#[test]
fn qa_non_finite_measured_over_the_finite_samples() {
    check_measured(measure);
}

negative_control!(
    qa_non_finite_measured_over_the_finite_samples,
    "a measured range that takes -inf as its low end is caught",
    expected = "has a non-finite end",
    check_measured(|n, xs| {
        let mut r = n.measured(xs);
        if xs.iter().any(|x| x.to_bits() == NEG_INF) {
            r[0] = f64::NEG_INFINITY;
        }
        r
    })
);

#[cfg(feature = "controls")]
mod qa_non_finite_measured_controls {
    use super::*;

    mod nan_payload_counted {
        use super::*;
        negative_control!(
            qa_non_finite_measured_over_the_finite_samples,
            "a measured range that counts a NaN payload's compaction as a sample is caught",
            expected = "not the finite values'",
            check_measured(|n, xs| {
                let counted: Vec<f32> = xs
                    .iter()
                    .map(|&x| if x.is_nan() { 1e6 } else { x })
                    .collect();
                n.measured(&counted)
            })
        );
    }
}

// ── The generated WGSL ──────────────────────────────────────────────────────────────────────────────────────────

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

/// The float literals in `code` (a token of digits with a `.` or an exponent), with their values.
fn float_literals(code: &str) -> Vec<(String, f64)> {
    let b = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let starts = b[i].is_ascii_digit()
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'));
        if !starts {
            i += 1;
            continue;
        }
        let s = i;
        while i < b.len()
            && (b[i].is_ascii_alphanumeric()
                || b[i] == b'.'
                || ((b[i] == b'-' || b[i] == b'+') && matches!(b[i - 1], b'e' | b'E')))
        {
            i += 1;
        }
        let tok = &code[s..i];
        if tok.starts_with("0x") || tok.starts_with("0X") {
            continue;
        }
        let t = tok.trim_end_matches(['f', 'h', 'u', 'i']);
        if t.contains('.') || t.contains('e') || t.contains('E') {
            if let Ok(v) = t.parse::<f64>() {
                out.push((tok.to_owned(), v));
            }
        }
    }
    out
}

/// The guard line R-436 gives every numeric view.
fn guard() -> String {
    squash("if ((bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u) { return debug_invalid(ctx.frag_xy); }")
}

/// Checks one view's WGSL `wgsl` for the field `e`.
fn check_wgsl(e: &Entry, wgsl: &str) {
    let f = e.name;
    let code: String = wgsl
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    // No isinf, no infinite literal, no comparison against the largest finite value (R-114; rule 4).
    assert!(!code.contains("isinf"), "`{f}`'s view tests isinf");
    assert!(!code.contains("isnan"), "`{f}`'s view tests isnan");
    for (tok, v) in float_literals(&code) {
        assert!(
            (v as f32).is_finite(),
            "`{f}`'s view carries the infinite literal `{tok}`"
        );
        assert!(
            (v as f32).abs() != f32::MAX,
            "`{f}`'s view compares against the largest finite value `{tok}`"
        );
    }
    let sq = squash(&code);
    for z in ["/0.0", "/0.", "/-0.0", "/0f", "/0.0f"] {
        assert!(
            !sq.contains(&format!("{z})")) && !sq.contains(&format!("{z};")),
            "`{f}`'s view divides by zero (an infinite value): {z}"
        );
    }

    // The order: let raw; the declared sentinel's or unset value's line; the guard, exactly once; the ramp.
    let lines = body(wgsl);
    assert!(
        lines[0].starts_with("let raw = "),
        "`{f}` does not bind raw first: {lines:?}"
    );
    let g = guard();
    let at: Vec<usize> = (0..lines.len())
        .filter(|&k| squash(&lines[k]) == g)
        .collect();
    assert_eq!(
        at.len(),
        1,
        "`{f}`'s view does not carry the non-finite bit test exactly once: {lines:?}"
    );
    let at = at[0];
    let before = &lines[1..at];
    let want_before = usize::from(e.sentinel.is_some());
    assert_eq!(
        before.len(),
        want_before,
        "`{f}`'s non-finite guard is not right after its declared sentinel's or unset value's line: {lines:?}"
    );
    match e.sentinel {
        Some(s) if !s.is_finite() => assert_eq!(
            squash(&before[0]),
            squash(&format!(
                "if (bitcast<u32>(raw) == {:#010x}u) {{ return debug_not_yet(ctx.frag_xy); }}",
                (s as f32).to_bits()
            )),
            "`{f}`'s unset value is not tested by its bits ahead of the guard"
        ),
        Some(_) => assert!(
            before[0].contains("dbg_sentinel(") && before[0].starts_with("if ("),
            "`{f}`'s line before the guard is not its sentinel's: {}",
            before[0]
        ),
        None => {}
    }
    // No arithmetic on raw before the guard: before it, `raw` appears only bitcast or compared with the sentinel.
    for l in before {
        let cond = &l[..l.find('{').unwrap_or(l.len())];
        let stripped = squash(cond)
            .replace("bitcast<u32>(raw)", "")
            .replace("raw==", "");
        assert!(
            !stripped.contains("raw"),
            "`{f}`'s line before the guard reads raw in arithmetic: {l}"
        );
    }
    let last = lines.last().unwrap();
    assert!(
        at < lines.len() - 1 && last.starts_with("return ramp_"),
        "`{f}`'s guard is not before the ramp: {lines:?}"
    );
}

fn check_all_wgsl(views: &[(Entry, NumericView, String)]) {
    for (e, _, w) in views {
        check_wgsl(e, w);
    }
}

#[test]
fn qa_non_finite_wgsl_every_view_bit_test_in_order() {
    check_all_wgsl(&views());
}

/// Each view with `from` replaced by `to` in `field`'s.
#[cfg(feature = "controls")]
fn mutated(field: &str, from: &str, to: &str) -> Vec<(Entry, NumericView, String)> {
    views()
        .into_iter()
        .map(|(e, n, w)| {
            let w = if e.name == field {
                assert!(w.contains(from), "`{field}` has no `{from}`");
                w.replace(from, to)
            } else {
                w
            };
            (e, n, w)
        })
        .collect()
}

negative_control!(
    qa_non_finite_wgsl_every_view_bit_test_in_order,
    "rho_ratio's view guarding the canonical NaN alone is caught",
    expected = "`rho_ratio`'s view does not carry the non-finite bit test",
    check_all_wgsl(&mutated(
        "rho_ratio",
        "(bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u",
        "bitcast<u32>(raw) == 0x7fc00000u"
    ))
);

#[cfg(feature = "controls")]
mod qa_non_finite_wgsl_controls {
    use super::*;

    const D_MIN_UNSET: &str =
        "    if (bitcast<u32>(raw) == 0x7f800000u) { return debug_not_yet(ctx.frag_xy); }\n";
    const D_MIN_GUARD: &str = "    if ((bitcast<u32>(raw) & 0x7f800000u) == 0x7f800000u) { return debug_invalid(ctx.frag_xy); }\n";

    mod guard_before_unset {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "d_min's guard ahead of its unset line is caught",
            expected = "`d_min`'s non-finite guard is not right after",
            check_all_wgsl(&mutated(
                "d_min",
                &format!("{D_MIN_UNSET}{D_MIN_GUARD}"),
                &format!("{D_MIN_GUARD}{D_MIN_UNSET}")
            ))
        );
    }

    mod isinf {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "a view testing isinf is caught",
            expected = "`V_0`'s view tests isinf",
            check_all_wgsl(&mutated(
                "V_0",
                "    return ramp_viridis(",
                "    if (isinf(raw)) { return debug_invalid(ctx.frag_xy); }\n    return ramp_viridis("
            ))
        );
    }

    mod infinite_literal {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "a view comparing with an infinite literal is caught",
            expected = "carries the infinite literal `1e39`",
            check_all_wgsl(&mutated(
                "E_0",
                "    return ramp_coolwarm(",
                "    if (abs(raw) == 1e39) { return debug_invalid(ctx.frag_xy); }\n    return ramp_coolwarm("
            ))
        );
    }

    mod max_compare {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "a view comparing with the largest finite value is caught",
            expected = "compares against the largest finite value",
            check_all_wgsl(&mutated(
                "rho_ratio",
                "    return ramp_viridis(",
                "    if (abs(raw) > 3.4028235e38) { return debug_invalid(ctx.frag_xy); }\n    return ramp_viridis("
            ))
        );
    }

    mod divide_by_zero {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "a view forming +inf as 1.0 / 0.0 is caught",
            expected = "divides by zero",
            check_all_wgsl(&mutated(
                "K_0",
                "    return ramp_viridis(",
                "    if (raw == 1.0 / 0.0) { return debug_invalid(ctx.frag_xy); }\n    return ramp_viridis("
            ))
        );
    }

    mod arithmetic_before_guard {
        use super::*;
        negative_control!(
            qa_non_finite_wgsl_every_view_bit_test_in_order,
            "length's sentinel line doing arithmetic on raw ahead of the guard is caught",
            expected = "reads raw in arithmetic",
            check_all_wgsl(&mutated(
                "length",
                "if (raw == 127.0)",
                "if (raw - 127.0 == 0.0)"
            ))
        );
    }
}
