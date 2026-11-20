//! QA tests for TASK-M0-50, round 6, written from REQ-RENDER-083's self-comparison clause: the lint must fail on "any
//! comparison of a float scalar or vector expression with itself (the same expression, or structurally equal reads of
//! the same `let`, argument, variable or buffer element with no store between)" (R-352, R-353), and the task's "with
//! no store to that variable between the two reads".
//!
//! "Between" is read as control flow, as in `qa_TASK-M0-50_paths.rs`: a store lies between two reads when some
//! execution runs it after the first read the comparison uses and before the second. WGSL has no labelled `break`,
//! so the "labelled-style" exits here are an inner loop's `break` that lands on the outer loop's read, or a flag the
//! outer loop breaks on. A call stores to a place when its callee, however deep, writes through the pointer it is
//! given to that place; a callee that only reads through it, or writes through another pointer, stores nothing there.
//! Each firing case has a near miss one step away on the other side, and each test has a registered negative control
//! (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

/// Globals, and a recursion-free chain of callees over pointer parameters, each writing some and only reading the
/// others: `mix1` writes its second; `mix2(a, b)` calls `mix1(b, a)`, so writes its first; `mix3(a, b, c)` calls
/// `mix2(c, b)` and reads `a`, so writes its third; `mix4(a, b, c)` calls `mix3(b, c, a)`, so writes its first and
/// only reads the other two. `rd2` only reads both; `wfield` and `rfield` reach one struct member through a callee; `sp3`
/// writes `g2` through private pointers two calls deep; `local_ptr` writes only its own local. `kill` always
/// discards, itself or (`kill2`) through a call, and `maybe_kill` on one path only; `ret_early`'s store follows its
/// `return`, so never runs, while `w_then_ret`'s and `wif`'s run.
const PREAMBLE: &str = "struct P { a: f32, b: f32 }
var<private> g: f32;
var<private> g2: f32;
fn w1(p: ptr<function, f32>) { *p = 1.0; }
fn r1(p: ptr<function, f32>) -> f32 { return *p; }
fn mix1(a: ptr<function, f32>, b: ptr<function, f32>) { *b = *a; }
fn mix2(a: ptr<function, f32>, b: ptr<function, f32>) { mix1(b, a); }
fn mix3(a: ptr<function, f32>, b: ptr<function, f32>, c: ptr<function, f32>) { mix2(c, b); let t = r1(a); }
fn mix4(a: ptr<function, f32>, b: ptr<function, f32>, c: ptr<function, f32>) { mix3(b, c, a); }
fn rd2(a: ptr<function, f32>, b: ptr<function, f32>) -> f32 { return r1(a) + r1(b); }
fn wfield(p: ptr<function, P>) { w1(&(*p).a); }
fn rfield(p: ptr<function, P>) -> f32 { return r1(&(*p).b); }
fn sp(p: ptr<private, f32>) { *p = 1.0; }
fn sp2(p: ptr<private, f32>) { sp(p); }
fn sp3() { sp2(&g2); }
fn local_ptr() -> f32 { var t = 0.0; w1(&t); return t; }
fn kill() { discard; }
fn kill2() { kill(); }
fn maybe_kill(c: bool) { if c { discard; } }
fn ret_early(p: ptr<function, f32>) { return; *p = 1.0; }
fn w_then_ret(p: ptr<function, f32>) { *p = 1.0; return; }
fn wif(p: ptr<function, f32>, c: bool) { if c { *p = 1.0; } }
";

/// `body` as the body of `f(x, y, i, c, q)`, on a line of its own; and that line.
fn wrap(body: &str) -> (String, u32) {
    let line = u32::try_from(PREAMBLE.lines().count() + 2).unwrap();
    let source = format!(
        "{PREAMBLE}fn f(x: f32, y: f32, i: i32, c: bool, q: ptr<function, f32>) -> bool {{\n    {body}\n}}\n"
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

/// Self-comparisons in nested control flow: every path that stores to the place leaves, or re-reads the first operand,
/// before the second read.
const CFG_SELF: [(&str, &str); 16] = [
    // Nested loops: an inner exit that stores leaves the function; an inner break or the flag carries nothing stored.
    ("inner loop: store then return, outer read", "var v = x; let old = v; loop { loop { if c { v = 1.0; return false; } break; } if old != v { return true; } } return false;"),
    ("inner loop: store then return, a flag breaks the outer", "var v = x; let old = v; var done = false; loop { loop { if c { v = 1.0; return false; } if i > 0 { done = true; break; } } if done { break; } } return old != v;"),
    ("inner loop: store, break to a return, never round the outer", "var v = x; let old = v; loop { loop { if c { if i > 0 { v = 1.0; break; } } if old != v { return true; } } return false; } return false;"),
    ("outer read refreshed each iteration, store after the inner loop", "var v = x; loop { let a = v; loop { if a != v { return true; } if c { break; } } v = 1.0; } return false;"),
    ("store then continue, both reads refreshed after it", "var v = x; loop { let a = v; if c { v = 1.0; continue; } if a != v { return true; } } return false;"),
    // break if.
    ("store after the loop a break-if leaves", "var v = x; let old = v; loop { if old != v { return true; } continuing { break if c; } } v = 1.0; return false;"),
    ("store in continuing, the body read refreshed", "var v = x; loop { let a = v; if a != v { return true; } continuing { v = v + 1.0; break if c; } } return false;"),
    // A store in continuing that no path reaches: the body always leaves.
    ("continuing store, body always breaks, read in body", "var v = x; let old = v; loop { if old != v { return true; } break; continuing { v = 1.0; } } return false;"),
    ("continuing store, body always breaks, read after", "var v = x; let old = v; loop { break; continuing { v = 1.0; break if i > 0; } } return old != v;"),
    ("continuing store reached by continue, body read refreshed", "var v = x; loop { let a = v; if a != v { return true; } if c { continue; } return false; continuing { v = 1.0; } } return false;"),
    // Exits inside nested ifs inside a loop.
    ("nested if: store then return", "var v = x; let old = v; loop { if c { if i > 0 { v = 1.0; return false; } } if old != v { return true; } } return false;"),
    ("nested if: store then discard", "var v = x; let old = v; loop { if c { if i > 0 { v = 1.0; discard; } } if old != v { return true; } } return false;"),
    // Switch fall-through chains (naga's multi-selector cases) and default inside one.
    ("chain 0, 1, 2: store then return", "var v = x; let old = v; switch i { case 0, 1, 2: { v = 1.0; return false; } default: {} } return old != v;"),
    ("chain 0, 1, 2: store then return, else break", "var v = x; let old = v; switch i { case 0, 1, 2: { if c { v = 1.0; return false; } break; } default: {} } return old != v;"),
    ("chain with default: store then return, read in another case", "var v = x; let old = v; switch i { case 3: { return old != v; } case 0, default, 1: { v = 1.0; return false; } } return false;"),
    ("chain in a loop: store then return, default continues", "var v = x; let old = v; loop { if old != v { return true; } switch i { case 0, 1: { v = 1.0; return false; } default: { continue; } } } return false;"),
];

/// Near misses, each one step from a case in `CFG_SELF`: a store on some path between the reads the comparison uses.
const CFG_STORED: [(&str, &str); 16] = [
    ("inner loop: store then break to the outer read", "var v = x; let old = v; loop { loop { if c { v = 1.0; break; } break; } if old != v { return true; } } return false;"),
    ("inner loop: store then break, a flag breaks the outer", "var v = x; let old = v; var done = false; loop { loop { if c { v = 1.0; done = true; break; } break; } if done { break; } } return old != v;"),
    ("inner loop: store, break, round the outer", "var v = x; let old = v; loop { loop { if c { if i > 0 { v = 1.0; break; } } if old != v { return true; } } } return false;"),
    ("outer read before the outer loop, store after the inner loop", "var v = x; let a = v; loop { loop { if a != v { return true; } if c { break; } } v = 1.0; } return false;"),
    ("inner loop: store then continue to the read", "var v = x; let old = v; loop { loop { if c { v = 1.0; continue; } if old != v { return true; } break; } } return false;"),
    ("store in continuing before the break-if, body read", "var v = x; let old = v; loop { if old != v { return true; } continuing { v = v + 1.0; break if c; } } return false;"),
    ("store in continuing, break-if out to the read", "var v = x; let old = v; loop { if c { break; } continuing { v = 1.0; break if i > 0; } } return old != v;"),
    ("continuing store reached by continue, read in body", "var v = x; let old = v; loop { if old != v { return true; } if c { continue; } break; continuing { v = 1.0; } } return false;"),
    ("continuing store reached by continue, read after", "var v = x; let old = v; loop { if c { continue; } break; continuing { v = 1.0; break if i > 0; } } return old != v;"),
    ("continuing store reached by continue, read before the loop", "var v = x; let a = v; loop { if a != v { return true; } if c { continue; } return false; continuing { v = 1.0; } } return false;"),
    ("nested if: store, the other arm returns", "var v = x; let old = v; loop { if c { if i > 0 { v = 1.0; } else { return false; } } if old != v { return true; } } return false;"),
    ("nested if: store, the other arm discards", "var v = x; let old = v; loop { if c { if i > 0 { v = 1.0; } else { discard; } } if old != v { return true; } } return false;"),
    ("chain 0, 1, 2: store", "var v = x; let old = v; switch i { case 0, 1, 2: { v = 1.0; } default: {} } return old != v;"),
    ("chain 0, 1, 2: store then break", "var v = x; let old = v; switch i { case 0, 1, 2: { if c { v = 1.0; break; } return false; } default: {} } return old != v;"),
    ("chain with default: store, read after", "var v = x; let old = v; switch i { case 3: { return false; } case 0, default, 1: { v = 1.0; } } return old != v;"),
    ("chain in a loop: store then continue", "var v = x; let old = v; loop { if old != v { return true; } switch i { case 0, 1: { v = 1.0; continue; } default: { return false; } } } return false;"),
];

/// Self-comparisons across calls given a pointer to the place read, where no callee in the chain writes through it.
const CALL_SELF: [(&str, &str); 10] = [
    ("mix4 given v as its second (read)", "var v = x; var u = y; var w = x; let old = v; mix4(&u, &v, &w); return old != v;"),
    ("mix4 given v as its third (read)", "var v = x; var u = y; var w = x; let old = v; mix4(&u, &w, &v); return old != v;"),
    ("mix3 given v as its first (read)", "var v = x; var u = y; var w = x; let old = v; mix3(&v, &u, &w); return old != v;"),
    ("mix2 given v as its second (read)", "var v = x; var u = y; let old = v; mix2(&u, &v); return old != v;"),
    ("rd2 given v twice", "var v = x; let old = v; let r = rd2(&v, &v); return old != v;"),
    ("mix4 given the argument pointer as its second", "var u = y; var w = x; let old = *q; mix4(&u, q, &w); return old != *q;"),
    ("rfield given the struct", "var v = P(x, y); let old = v.b; let r = rfield(&v); return old != v.b;"),
    ("sp3 writes g2, g read", "let old = g; sp3(); return old != g;"),
    ("local_ptr writes its own first local", "var t = x; let old = t; let r = local_ptr(); return old != t;"),
    ("mix4 in a loop, v only read", "var v = x; var u = y; var w = x; let old = v; loop { if old != v { return true; } mix4(&u, &v, &w); } return false;"),
];

/// Near misses, each one step from a case in `CALL_SELF`: a callee in the chain writes through the pointer.
const CALL_STORED: [(&str, &str); 10] = [
    ("mix4 given v as its first (written)", "var v = x; var u = y; var w = x; let old = v; mix4(&v, &u, &w); return old != v;"),
    ("mix4 given w as its first, w read", "var v = x; var u = y; var w = x; let old = w; mix4(&w, &u, &v); return old != w;"),
    ("mix3 given v as its third (written)", "var v = x; var u = y; var w = x; let old = v; mix3(&u, &w, &v); return old != v;"),
    ("mix2 given v as its first (written)", "var v = x; var u = y; let old = v; mix2(&v, &u); return old != v;"),
    ("mix1 given v twice", "var v = x; let old = v; mix1(&v, &v); return old != v;"),
    ("mix4 given the argument pointer as its first", "var u = y; var w = x; let old = *q; mix4(q, &u, &w); return old != *q;"),
    ("wfield given the struct", "var v = P(x, y); let old = v.a; wfield(&v); return old != v.a;"),
    ("sp3 writes g2, g2 read", "let old = g2; sp3(); return old != g2;"),
    ("w1 given the first local", "var t = x; let old = t; w1(&t); return old != t;"),
    ("mix4 in a loop, v written round the back edge", "var v = x; var u = y; var w = x; let old = v; loop { if old != v { return true; } mix4(&v, &u, &w); } return false;"),
];

#[test]
fn qa_lint_wgsl_self_compare_cfg_fires() {
    check(&CFG_SELF, true);
}

negative_control!(
    qa_lint_wgsl_self_compare_cfg_fires,
    "each near miss has a store on a path between its reads, so the rule must not fire",
    expected = "did not fire",
    check(&CFG_STORED, true)
);

#[test]
fn qa_lint_wgsl_self_compare_cfg_near_misses_are_quiet() {
    check(&CFG_STORED, false);
}

negative_control!(
    qa_lint_wgsl_self_compare_cfg_near_misses_are_quiet,
    "each self-comparison has no store between its reads, so the rule must fire",
    expected = "fired on",
    check(&CFG_SELF, false)
);

#[test]
fn qa_lint_wgsl_self_compare_call_chain_fires() {
    check(&CALL_SELF, true);
}

negative_control!(
    qa_lint_wgsl_self_compare_call_chain_fires,
    "each near miss's callee chain writes through the pointer, so the rule must not fire",
    expected = "did not fire",
    check(&CALL_STORED, true)
);

#[test]
fn qa_lint_wgsl_self_compare_call_chain_near_misses_are_quiet() {
    check(&CALL_STORED, false);
}

negative_control!(
    qa_lint_wgsl_self_compare_call_chain_near_misses_are_quiet,
    "no callee in each self-comparison's chain writes through the pointer, so the rule must fire",
    expected = "fired on",
    check(&CALL_SELF, false)
);

/// Self-comparisons where the callee's own control flow keeps its store off every path between the reads: the store
/// in the caller is followed by a call that always discards (naga's `Statement::Kill`, "Aborts the current shader
/// execution", aborts it from a callee as from the caller; the PR's decision, applied per R-369, that `discard` ends a
/// path), and a callee's store that its own `return` makes unreachable.
const CALLEE_FLOW_SELF: [(&str, &str); 3] = [
    (
        "store then a call that always discards",
        "var v = x; let old = v; if c { v = 1.0; kill(); } return old != v;",
    ),
    (
        "store then a call that discards two calls deep",
        "var v = x; let old = v; if c { v = 1.0; kill2(); } return old != v;",
    ),
    (
        "a callee whose store follows its return",
        "var v = x; let old = v; ret_early(&v); return old != v;",
    ),
];

/// Near misses, each one step from a case in `CALLEE_FLOW_SELF`: the callee may return after the caller's store, or
/// its own store runs.
const CALLEE_FLOW_STORED: [(&str, &str); 4] = [
    (
        "store then a call that discards on one path",
        "var v = x; let old = v; if c { v = 1.0; maybe_kill(c); } return old != v;",
    ),
    (
        "store then a call that never discards",
        "var v = x; let old = v; if c { v = 1.0; let r = local_ptr(); } return old != v;",
    ),
    (
        "a callee whose store precedes its return",
        "var v = x; let old = v; w_then_ret(&v); return old != v;",
    ),
    (
        "a callee that stores on one path",
        "var v = x; let old = v; wif(&v, c); return old != v;",
    ),
];

#[test]
fn qa_lint_wgsl_self_compare_callee_flow_fires() {
    check(&CALLEE_FLOW_SELF, true);
}

negative_control!(
    qa_lint_wgsl_self_compare_callee_flow_fires,
    "each near miss's store reaches the second read, so the rule must not fire",
    expected = "did not fire",
    check(&CALLEE_FLOW_STORED, true)
);

#[test]
fn qa_lint_wgsl_self_compare_callee_flow_near_misses_are_quiet() {
    check(&CALLEE_FLOW_STORED, false);
}

negative_control!(
    qa_lint_wgsl_self_compare_callee_flow_near_misses_are_quiet,
    "no store reaches each self-comparison's second read, so the rule must fire",
    expected = "fired on",
    check(&CALLEE_FLOW_SELF, false)
);
