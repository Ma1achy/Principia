//! QA tests for TASK-M0-50, round 6, written from REQ-RENDER-083's self-comparison clause: the lint must fail on "any
//! comparison of a float scalar or vector expression with itself (the same expression, or structurally equal reads of
//! the same `let`, argument, variable or buffer element with no store between)" (R-352, R-353), and the task's "with
//! no store to that variable between the two reads".
//!
//! "Between" is read as control flow, as in `qa_TASK-M0-50_cfg.rs`: a store lies between two reads when some
//! execution runs it after the first read and before the second. A call runs its callee's store, as the caller sees
//! it, only when some path through the callee runs the store and then returns to the caller; a path that discards
//! ("Aborts the current shader execution") or never leaves a loop never comes back. This file probes that across the
//! callee's own control flow: callees that return on some paths and discard on others, loops in callees (one that
//! can never return), stores through struct members and array elements, and a store reachable only by `continue`.
//! WGSL forbids recursion and pointers inside arrays or structs; the last test checks the lint's parser refuses them,
//! so the callee-first summary order the lint relies on always holds. Each firing case has a near miss one step away
//! on the other side, and each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

/// A struct and an array, a global, and callees over pointer parameters whose own control flow decides what the
/// caller sees after the call.
const PREAMBLE: &str = "struct P { a: f32, b: f32 }
var<private> g: f32;
fn ret_or_die(p: ptr<function, f32>, c: bool) { if c { *p = 1.0; return; } *p = 2.0; discard; }
fn die_both(p: ptr<function, f32>, c: bool) { if c { *p = 1.0; discard; } *p = 2.0; discard; }
fn clean_ret_or_set_die(p: ptr<function, f32>, c: bool) { if c { return; } *p = 2.0; discard; }
fn set_ret_or_clean_die(p: ptr<function, f32>, c: bool) { if c { *p = 1.0; return; } discard; }
fn g_ret_or_die(c: bool) { if c { g = 1.0; return; } discard; }
fn g_die_or_ret(c: bool) { if c { g = 1.0; discard; } }
fn loop_set(p: ptr<function, f32>, n: i32) { var i = 0; loop { if i >= n { break; } *p = 1.0; i++; } }
fn loop_set_break_if(p: ptr<function, f32>, c: bool) { loop { *p = 1.0; continuing { break if c; } } }
fn loop_set_return(p: ptr<function, f32>, c: bool) { loop { *p = 1.0; if c { return; } } }
fn spin(p: ptr<function, f32>) { loop { *p = 1.0; } }
fn spin_c(p: ptr<function, f32>) { loop { *p = 1.0; continuing { } } }
fn loop_set_die(p: ptr<function, f32>, c: bool) { loop { *p = 1.0; if c { discard; } } }
fn loop_clean_ret_or_set_die(p: ptr<function, f32>, d: bool) { var n = 0; loop { if n > 0 { return; } *p = 1.0; n++; if d { discard; } } }
fn set_member(p: ptr<function, P>) { (*p).a = 1.0; }
fn set_member_die(p: ptr<function, P>) { (*p).a = 1.0; discard; }
fn set_elem(p: ptr<function, array<f32, 4>>, k: i32) { (*p)[k] = 1.0; }
fn set_elem_die(p: ptr<function, array<f32, 4>>, k: i32, c: bool) { if c { (*p)[k] = 1.0; discard; } }
fn cont_set(p: ptr<function, f32>, c: bool, d: bool) { loop { if c { break; } if d { continue; } return; continuing { *p = 1.0; } } }
fn cont_set_never_back(p: ptr<function, f32>, c: bool, d: bool) { if c { return; } loop { if d { continue; } discard; continuing { *p = 1.0; } } }
fn cont_set_on(p: ptr<function, f32>, c: bool, d: bool) { cont_set(p, c, d); }
fn cont_set_never_back_on(p: ptr<function, f32>, c: bool, d: bool) { cont_set_never_back(p, c, d); }
";

/// `body` as the body of `f(x, y, i, c, d)`, on a line of its own; and that line.
fn wrap(body: &str) -> (String, u32) {
    let line = u32::try_from(PREAMBLE.lines().count() + 2).unwrap();
    let source = format!(
        "{PREAMBLE}fn f(x: f32, y: f32, i: i32, c: bool, d: bool) -> bool {{\n    {body}\n}}\n"
    );
    (source, line)
}

/// Panics, naming each case on which the self-compare rule's firing on the body's line is not `fires`.
fn check(cases: &[(&str, &str)], fires: bool) {
    let mut wrong = Vec::new();
    for (name, body) in cases {
        let (source, line) = wrap(body);
        let found = check_fragment(&source)
            .unwrap_or_else(|e| panic!("case `{name}` does not parse/validate: {e}\n{source}"));
        let hit = found
            .iter()
            .any(|f| f.rule == Rule::SelfCompare && f.line == Some(line));
        if hit != fires {
            wrong.push(*name);
        }
    }
    if fires {
        assert!(
            wrong.is_empty(),
            "self-compare did not fire on {} case(s): {wrong:?}",
            wrong.len()
        );
    } else {
        assert!(
            wrong.is_empty(),
            "self-compare fired on {} near miss(es): {wrong:?}",
            wrong.len()
        );
    }
}

/// Self-comparisons across a call whose every store through the pointer (or to the global) is on no path that returns
/// to the caller: the callee discards after it, or loops for ever, or the store is reachable only on a path that
/// discards.
const SUMMARY_SELF: [(&str, &str); 10] = [
    ("mixed callee: the store on each path, every path discards", "var v = x; let old = v; if c { die_both(&v, c); } return old != v;"),
    ("mixed callee: returns clean, discards after its store", "var v = x; let old = v; clean_ret_or_set_die(&v, c); return old != v;"),
    ("mixed callee: global stored only on the discarding path", "let old = g; g_die_or_ret(c); return old != g;"),
    ("callee loops for ever, storing", "var v = x; let old = v; if c { spin(&v); } return old != v;"),
    ("callee loops for ever through an empty continuing", "var v = x; let old = v; if c { spin_c(&v); } return old != v;"),
    ("callee's loop leaves only by discard", "var v = x; let old = v; if c { loop_set_die(&v, c); } return old != v;"),
    ("struct member stored, then discard", "var v = P(x, y); let old = v.a; if c { set_member_die(&v); } return old != v.a;"),
    ("array element stored, then discard", "var v = array<f32, 4>(x, y, x, y); let old = v[1]; set_elem_die(&v, 1, c); return old != v[1];"),
    ("store reachable only by continue, never back to the caller", "var v = x; let old = v; cont_set_never_back(&v, c, d); return old != v;"),
    ("the same, one call deeper", "var v = x; let old = v; cont_set_never_back_on(&v, c, d); return old != v;"),
];

/// Near misses, each one step from a case in `SUMMARY_SELF`: some path through the callee runs the store and returns.
const SUMMARY_STORED: [(&str, &str); 11] = [
    (
        "mixed callee: the store on each path, one returns",
        "var v = x; let old = v; ret_or_die(&v, c); return old != v;",
    ),
    (
        "mixed callee: stores then returns, else discards clean",
        "var v = x; let old = v; set_ret_or_clean_die(&v, c); return old != v;",
    ),
    (
        "mixed callee: global stored on the returning path",
        "let old = g; g_ret_or_die(c); return old != g;",
    ),
    (
        "callee loop stores, then breaks out",
        "var v = x; let old = v; loop_set(&v, i); return old != v;",
    ),
    (
        "callee loop stores, leaves by break if",
        "var v = x; let old = v; if c { loop_set_break_if(&v, c); } return old != v;",
    ),
    (
        "callee loop stores, leaves by return",
        "var v = x; let old = v; if c { loop_set_return(&v, c); } return old != v;",
    ),
    (
        "callee loop stores, returns on the next iteration",
        "var v = x; let old = v; loop_clean_ret_or_set_die(&v, d); return old != v;",
    ),
    (
        "struct member stored, returns",
        "var v = P(x, y); let old = v.a; set_member(&v); return old != v.a;",
    ),
    (
        "array element stored, returns",
        "var v = array<f32, 4>(x, y, x, y); let old = v[1]; set_elem(&v, 1); return old != v[1];",
    ),
    (
        "store reachable only by continue, then breaks back",
        "var v = x; let old = v; cont_set(&v, c, d); return old != v;",
    ),
    (
        "the same, one call deeper",
        "var v = x; let old = v; cont_set_on(&v, c, d); return old != v;",
    ),
];

#[test]
fn qa_lint_wgsl_self_compare_callee_summary_fires() {
    check(&SUMMARY_SELF, true);
}

negative_control!(
    qa_lint_wgsl_self_compare_callee_summary_fires,
    "each near miss's callee runs its store and returns on some path, so the rule must not fire",
    expected = "did not fire",
    check(&SUMMARY_STORED, true)
);

#[test]
fn qa_lint_wgsl_self_compare_callee_summary_near_misses_are_quiet() {
    check(&SUMMARY_STORED, false);
}

negative_control!(
    qa_lint_wgsl_self_compare_callee_summary_near_misses_are_quiet,
    "no path through each self-comparison's callee stores and returns, so the rule must fire",
    expected = "fired on",
    check(&SUMMARY_SELF, false)
);

/// WGSL forms the lint can never meet, because the parser refuses them: recursion, direct or mutual (the callee-first
/// order of naga's function arena, on which the summaries rest), and pointers held in an array or a struct.
const REFUSED: [(&str, &str); 4] = [
    ("direct recursion", "fn r(p: ptr<function, f32>) { r(p); }\n@fragment fn main() {}\n"),
    ("mutual recursion", "fn a(p: ptr<function, f32>) { b(p); }\nfn b(p: ptr<function, f32>) { a(p); }\n@fragment fn main() {}\n"),
    ("array of pointers", "fn h(p: ptr<function, f32>) { var arr: array<ptr<function, f32>, 2>; }\n@fragment fn main() {}\n"),
    ("struct holding a pointer", "struct S { p: ptr<function, f32> }\n@fragment fn main() {}\n"),
];

/// A source each `REFUSED` case is one step from, which parses.
const ACCEPTED: [(&str, &str); 4] = [
    ("a call chain", "fn r2(p: ptr<function, f32>) { *p = 1.0; }\nfn r(p: ptr<function, f32>) { r2(p); }\n@fragment fn main() {}\n"),
    ("a chain of two", "fn b(p: ptr<function, f32>) { *p = 1.0; }\nfn a(p: ptr<function, f32>) { b(p); }\n@fragment fn main() {}\n"),
    ("array of floats", "fn h(p: ptr<function, f32>) { var arr: array<f32, 2>; }\n@fragment fn main() {}\n"),
    ("struct holding a float", "struct S { p: f32 }\n@fragment fn main() {}\n"),
];

/// Panics, naming each case whose refusal by the lint's parser is not `refused`.
fn check_refused(cases: &[(&str, &str)], refused: bool) {
    let wrong: Vec<&str> = cases
        .iter()
        .filter(|(_, src)| check_fragment(src).is_err() != refused)
        .map(|(name, _)| *name)
        .collect();
    assert!(
        wrong.is_empty(),
        "refusal is not {refused} on {} case(s): {wrong:?}",
        wrong.len()
    );
}

#[test]
fn qa_lint_wgsl_refuses_recursion_and_stored_pointers() {
    check_refused(&REFUSED, true);
}

negative_control!(
    qa_lint_wgsl_refuses_recursion_and_stored_pointers,
    "each accepted source is valid WGSL, so it must not be refused",
    expected = "refusal is not true",
    check_refused(&ACCEPTED, true)
);

#[test]
fn qa_lint_wgsl_accepts_the_near_misses_of_refused_forms() {
    check_refused(&ACCEPTED, false);
}

negative_control!(
    qa_lint_wgsl_accepts_the_near_misses_of_refused_forms,
    "each refused source is invalid WGSL, so it must be refused",
    expected = "refusal is not false",
    check_refused(&REFUSED, false)
);
