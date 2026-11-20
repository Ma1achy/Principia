//! QA tests for TASK-M0-50, round 5, written from REQ-RENDER-083's self-comparison clause: the lint must fail on "any
//! comparison of a float scalar or vector expression with itself (the same expression, or structurally equal reads of
//! the same `let`, argument, variable or buffer element with no store between)" (R-352, R-353), and the task's
//! "with no store to that variable between the two reads".
//!
//! "Between" is read as control flow: a store lies between two reads when some execution runs the store after the
//! first read and before the second. So a store on a path that leaves the function or the loop (`return`, `discard`,
//! `break`) before the second read is not between them, and the comparison still fails the lint; a store reached
//! round a loop's back edge, through a `break` that carries it to the second read, inside a callee (however deep), or
//! by another invocation across a barrier is between them, and the comparison is no self-comparison. Each firing case
//! has a near miss one step away, and each test has a registered negative control (R-176).

use validation::negative_control;
use xtask::lint_wgsl::{check_fragment, Rule};

/// Globals in each address space a fragment-stage module can name, and callees that read, write or pass on a place.
/// `set_g_3` is written before the callees it reaches `g` through.
const PREAMBLE: &str = "struct S { v: array<f32, 4> }
@group(0) @binding(0) var<storage, read_write> sw: S;
@group(0) @binding(1) var<storage, read> sr: S;
@group(0) @binding(2) var<uniform> u: vec4<f32>;
var<private> g: f32;
var<private> g2: f32;
var<workgroup> wg: f32;
fn bump(p: ptr<function, f32>) { *p = *p + 1.0; }
fn peek(p: ptr<function, f32>) -> f32 { return *p; }
fn set_g_3() { set_g_2(); }
fn set_g_2() { set_g(); }
fn set_g() { g = 1.0; }
fn set_p(p: ptr<private, f32>) { *p = 1.0; }
fn set_g_via_ptr() { set_p(&g); }
fn set_wg() { wg = 1.0; }
fn set_sw() { sw.v[0] = 1.0; }
fn read_g() -> f32 { return g + g2; }
fn local_only() -> f32 { var t = 1.0; t = t + 1.0; return t; }
";

/// `body` as the body of `f(x, y, i, c)`, on a line of its own; and that line.
fn wrap(body: &str) -> (String, u32) {
    let line = u32::try_from(PREAMBLE.lines().count() + 2).unwrap();
    let source =
        format!("{PREAMBLE}fn f(x: f32, y: f32, i: i32, c: bool) -> bool {{\n    {body}\n}}\n");
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

/// Self-comparisons: no execution runs a store to the place read between the two reads.
const SELF: [(&str, &str); 23] = [
    // Early exits: the store's path leaves before the second read.
    ("store then return, read after the if", "var v = x; let old = v; if c { v = 1.0; return false; } return old != v;"),
    ("store then discard, read after the if", "var v = x; let old = v; if c { v = 1.0; discard; } return old != v;"),
    ("store then return in a switch case", "var v = x; let old = v; switch i { case 0: { v = 1.0; return false; } default: {} } return old != v;"),
    ("store then return in a loop, read in the loop", "var v = x; let old = v; loop { if c { v = 1.0; return false; } if old != v { return true; } } return false;"),
    ("store then break, read in the loop", "var v = x; let old = v; loop { if c { v = 1.0; break; } if old != v { return true; } } return false;"),
    ("store then return, never round the back edge", "var v = x; let old = v; loop { if old != v { return true; } v = 1.0; return false; } return false;"),
    // Loops: the store runs only before the first read or after the second.
    ("break-if in continuing on a body read", "var v = x; loop { let a = v; continuing { break if a != v; } } return false;"),
    ("store in an if before both reads in a loop", "var v = x; loop { if c { v = 1.0; } let a = v; if a != v { return true; } } return false;"),
    ("both reads in an inner loop, store after it", "var v = x; loop { loop { let a = v; if a != v { return true; } break; } v = v + 1.0; } return false;"),
    ("store in a finished loop before both reads", "var v = x; loop { v = v + 1.0; if c { break; } } let old = v; return old != v;"),
    // Switch: a multi-selector case (naga's fall-through) and a leading default.
    ("store in a multi-selector case, read in default", "var v = x; let old = v; switch i { case 0, 1: { v = 1.0; } default: { return old != v; } } return false;"),
    ("store in a leading default, read in a later case", "var v = x; let old = v; switch i { default: { v = 1.0; } case 0, 1: { return old != v; } } return false;"),
    // Calls that write nothing read.
    ("a callee that only reads the global", "let old = g; let r = read_g(); return old != g;"),
    ("a callee that writes only its locals", "let old = g; let r = local_only(); return old != g;"),
    ("a pointer to another local passed on", "var v = x; var w = y; let old = v; bump(&w); return old != v;"),
    ("a pointer passed to a callee that only reads it", "var v = x; let old = v; let r = peek(&v); return old != v;"),
    ("a storage element across a call writing workgroup", "let a = sw.v[0]; set_wg(); return a != sw.v[0];"),
    // Globals in each address space, with no store, call or barrier writing them.
    ("private read twice", "let a = g; return a != g;"),
    ("workgroup read twice", "let a = wg; return a != wg;"),
    ("read-write storage element read twice", "let a = sw.v[i]; return a != sw.v[i];"),
    ("read-only storage element across a barrier", "let a = sr.v[i]; storageBarrier(); return a != sr.v[i];"),
    ("uniform across a barrier", "let a = u.x; workgroupBarrier(); return a != u.x;"),
    ("private across a storage barrier", "let a = g; storageBarrier(); return a != g;"),
];

/// Near misses, each one step from a case in `SELF`: a store on some path between the two reads.
const STORED: [(&str, &str); 20] = [
    ("store in an if, read after", "var v = x; let old = v; if c { v = 1.0; } return old != v;"),
    ("store then break, read after the loop", "var v = x; let old = v; loop { if c { v = 1.0; break; } if i > 0 { break; } } return old != v;"),
    ("store in a multi-selector case, read after", "var v = x; let old = v; switch i { case 0, 1: { v = 1.0; } default: {} } return old != v;"),
    ("store in a switch case in a loop, round the back edge", "var v = x; let old = v; loop { if old != v { return true; } switch i { case 0: { v = 1.0; } default: {} } } return false;"),
    ("store in continuing before the break-if read", "var v = x; loop { let a = v; continuing { v = v + 1.0; break if a != v; } } return false;"),
    ("read in an inner loop, store in the outer after it", "var v = x; let old = v; loop { loop { if old != v { return true; } break; } v = v + 1.0; } return false;"),
    ("for-loop update", "var v = x; let old = v; for (; v < 3.0; v += 1.0) { if old != v { return true; } } return false;"),
    ("while-loop body", "var v = x; let old = v; while (v < 3.0) { if old != v { return true; } v = v + 1.0; } return false;"),
    ("store then return in the loop, after a later store on the back edge", "var v = x; let old = v; loop { if old != v { return true; } v = 1.0; if c { return false; } } return false;"),
    ("a callee three calls deep, written before its callees", "let old = g; set_g_3(); return old != g;"),
    ("a callee writing the global through a private pointer", "let old = g; set_g_via_ptr(); return old != g;"),
    ("a private pointer to the global passed on", "let old = g; set_p(&g); return old != g;"),
    ("a callee writing workgroup", "let old = wg; set_wg(); return old != wg;"),
    ("a callee writing the storage element", "let old = sw.v[0]; set_sw(); return old != sw.v[0];"),
    ("a store to the storage element", "let old = sw.v[i]; sw.v[i] = 1.0; return old != sw.v[i];"),
    ("read-write storage across a storage barrier", "let old = sw.v[i]; storageBarrier(); return old != sw.v[i];"),
    ("read-write storage across workgroupUniformLoad", "let old = sw.v[i]; let t = workgroupUniformLoad(&wg); return old != sw.v[i];"),
    ("workgroup across a storage barrier", "let old = wg; storageBarrier(); return old != wg;"),
    ("a callee in a loop, round the back edge", "let old = g; loop { if old != g { return true; } set_g_3(); } return false;"),
    ("a pointer to the local passed on", "var v = x; let old = v; bump(&v); return old != v;"),
];

#[test]
fn qa_lint_wgsl_self_compare_paths_fire() {
    check(&SELF, true);
}

negative_control!(
    qa_lint_wgsl_self_compare_paths_fire,
    "each near miss has a store on a path between its reads, so the rule must not fire",
    expected = "did not fire",
    check(&STORED, true)
);

#[test]
fn qa_lint_wgsl_self_compare_paths_near_misses_are_quiet() {
    check(&STORED, false);
}

negative_control!(
    qa_lint_wgsl_self_compare_paths_near_misses_are_quiet,
    "each self-comparison has no store between its reads, so the rule must fire",
    expected = "fired on",
    check(&SELF, false)
);
