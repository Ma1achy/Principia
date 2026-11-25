//! The compile-discipline lint: the kernel's march layers held to the GPU compile discipline (gpu_determinism_note
//! § "The discipline"; integrator_contract § "Rules the new kernel must hold by construction"), read from their source
//! with syn, so every later GPU build compiles a kernel that already obeys it.
//!
//! **The layers** are the march's: `physics/`, `driver/`, `occupant/`, `regularisation/` and `detect/` under
//! `crates/kernel/src`, every file, but for what the SPIR-V build does not take: an item or module under
//! `#[cfg(not(target_arch = "spirv"))]` (the host's `N_sub` table generator, `driver/nsub_table.rs`, among them) or
//! `#[cfg(test)]`. The stage code the SPIR-V build compiles today, `decode/`, `stage/`, `canon.rs`, `word/` and
//! `shape.rs` ([`STAGE`]), is held to every rule but two: the index and cast rules. The plan is unclear here; chose to
//! leave those two out there because the task names "the kernel's physics and driver layers", and the stage entry
//! points index their storage buffers by invocation id and cast that id, which no constant index or clamp replaces.
//!
//! **The rules**, one test each, each with a seeded violation as its negative control:
//! - `discipline_lint_no_break`: no `break` or `continue` (rule 5: the exit is a flag in the `while` condition).
//! - `discipline_lint_no_exit_from_a_loop_body`: no `return` or `?` inside a `while` body, the other mid-body exit.
//! - `discipline_lint_no_bare_loop`: no `loop {`; every loop is a `while` whose condition holds its count bound.
//! - `discipline_lint_no_counted_for`: no `for` at all, so no `for s in 0..N_sub` with a mid-body exit.
//! - `discipline_lint_no_non_finite_literal`: no `INFINITY`, `NEG_INFINITY` or `NAN` constant, no `infinity()`,
//!   `neg_infinity()` or `nan()`, and no division by a zero literal (rule 4); a stored `±∞` is formed from its bits.
//! - `discipline_lint_no_dynamic_index`: an array index is an integer literal, a constant (an upper-case path) or a
//!   range of them (rule 5: implicit bounds checks are compiler-injected multi-level exits).
//! - `discipline_lint_clamped_casts`: every cast to an integer type is in [`CASTS`], each classified: an integer
//!   operand, or a float one clamped into range before the cast (rule 3), which the lint checks (below).
//! - `discipline_lint_precision_constants`: no `Real`-dependent constant in an `if`, `while` or match-guard condition
//!   (integrator_contract rule 1): an associated constant whose `f32` and `f64` impls give different values (an `f32`
//!   literal that is not the `f64` one rounded), `epsilon`/`EPSILON`, `min_positive_value`/`MIN_POSITIVE`, or an
//!   `at_width(single, double)` whose two values differ. `TINY` is the rule's one enumerated exception.
//! - `discipline_lint_scan`: the scanner's own cases, the host-only items skipped among them.
//!
//! **What a source scan cannot decide**, left to the physics reviewer's audit of every branch condition: a
//! `Real`-dependent constant reaching a branch through a `let`, a field or a function (only one written in the
//! condition is seen); and whether a cast's operand is a float, which [`CASTS`] records by hand, so a new cast fails
//! until someone classifies it. A float cast's clamp is checked only in its shape: the operand a plain name, bound before
//! the cast by a `.clamp(..)` or by both a lower and an upper select, `let x = if x > lo { x } else { lo }` and
//! `let x = if x < hi { x } else { hi }`. syn does not parse inside a macro invocation; the layers hold no
//! `macro_rules!` and call only `panic!`, `write!` and `matches!`, so no step, loop or branch hides in one today.
//!
//! **Known violations** are in [`ALLOWED`], each by file, rule and exact count with its reason: a count that grows or
//! shrinks fails, so the list only ever shrinks with the code.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use validation::negative_control;

/// The march's layers, directories under `crates/kernel/src` (the module's note).
const LAYERS: [&str; 5] = ["physics", "driver", "occupant", "regularisation", "detect"];

/// The stage code the SPIR-V build compiles, held to every rule but the index and cast rules (the module's note): a
/// directory (ending `/`) or a file under `crates/kernel/src`.
const STAGE: [&str; 5] = ["decode/", "stage/", "canon.rs", "word/", "shape.rs"];

/// The integer types a cast may target.
const INTS: [&str; 12] = [
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// The names of the non-finite constants and constructors.
const NON_FINITE: [&str; 6] = [
    "INFINITY",
    "NEG_INFINITY",
    "NAN",
    "infinity",
    "neg_infinity",
    "nan",
];

/// The precision-dependent names every `Real` carries, whatever the kernel's own impls hold.
const FLOAT_LIMITS: [&str; 4] = ["epsilon", "EPSILON", "min_positive_value", "MIN_POSITIVE"];

/// The rule's one enumerated exception (integrator_contract rule 1): the floor, one value per working precision.
const EXCEPTED: &str = "TINY";

/// One discipline rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    Break,
    Exit,
    Loop,
    For,
    NonFinite,
    Index,
    Cast,
    Precision,
}

impl Rule {
    /// The rule's message, which its negative control expects.
    fn message(self) -> &'static str {
        match self {
            Rule::Break => "discipline: `break` or `continue` in a march layer",
            Rule::Exit => "discipline: `return` or `?` inside a `while` body",
            Rule::Loop => "discipline: a bare `loop` in a march layer",
            Rule::For => "discipline: a `for` loop in a march layer",
            Rule::NonFinite => "discipline: a non-finite literal in a march layer",
            Rule::Index => "discipline: a runtime index in a march layer",
            Rule::Cast => "discipline: an integer cast not classified as clamped",
            Rule::Precision => "discipline: a Real-dependent constant in a branch condition",
        }
    }
}

/// One finding: the file below `src/`, the line, the rule, the enclosing function and the offending expression.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    file: String,
    line: usize,
    rule: Rule,
    func: String,
    text: String,
    /// For a cast: whether its operand is a plain name clamped before it (the module's note).
    clamped: bool,
}

/// How a cast in [`CASTS`] is classified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cast {
    /// The operand is already an integer: no float reaches the cast.
    Int,
    /// The operand is a float, clamped into the target's range before the cast (rule 3).
    Clamped,
}

/// Every integer cast in the march layers, by file, enclosing function and the cast as quote prints it, with its class.
const CASTS: &[(&str, &str, &str, Cast)] = &[
    // `Count::whole`, the float count to a whole u32: NaN to 0, then clamped into `[0, U32_TOP]` by two selects.
    ("driver/advance.rs", "whole", "x as u32", Cast::Clamped),
    // The readout's pole code and step count, u32 to the stored u16s.
    ("driver/shape.rs", "store", "self . code as u16", Cast::Int),
    (
        "driver/shape.rs",
        "store",
        "self . closure . step as u16",
        Cast::Int,
    ),
    // The `_reserved` code, a u32 constant, to its u16 field.
    (
        "driver/failure.rs",
        "decode_failed",
        "NO_REFERENCE as u16",
        Cast::Int,
    ),
    // The force evaluations, u32 counts widened to u64.
    (
        "occupant/mod.rs",
        "force_evaluations",
        "total_substeps as u64",
        Cast::Int,
    ),
    (
        "occupant/mod.rs",
        "force_evaluations",
        "self . force_evals as u64",
        Cast::Int,
    ),
    // The floor code: an i32 exponent, clamped, biased, to its u32 code; and back.
    (
        "detect/collision.rs",
        "floor_code_of_exponent",
        "((e - 23) . clamp (FLOOR_EXP_MIN , FLOOR_EXP_MAX) + FLOOR_CODE_BIAS) as u32",
        Cast::Int,
    ),
    (
        "detect/collision.rs",
        "floor_of_code",
        "code as i32",
        Cast::Int,
    ),
    // The Chazy test's escaper, a body index (`usize`), to the detail's u32 (both casts).
    (
        "detect/escape_rule.rs",
        "chazy_test",
        "most_separated (& ph) as u32",
        Cast::Int,
    ),
];

/// The known violations: file below `src/`, rule, exact count, and why it stands.
const ALLOWED: &[(&str, Rule, usize, &str)] = &[
    (
        "driver/cadence.rs",
        Rule::NonFinite,
        1,
        "`R::nan` in `reload`'s widening: fixed in TASK-M3-34's file after it merges",
    ),
    (
        "driver/benettin.rs",
        Rule::NonFinite,
        1,
        "`ftle_read`'s invalid FTLE, `R::nan()`: the read of a stored sample, which the host calls, not the march; to \
         form from its bits when a GPU path reads it",
    ),
    (
        "regularisation/az.rs",
        Rule::Index,
        10,
        "the step-7 state scan's `xs[i]` and the RK4 weights' `f(k[_])[i]`, bounded indices over fixed arrays: to \
         unroll when the march is first compiled to SPIR-V (the harness and flat-grid tasks, TASK-M4-02, TASK-M4-05)",
    ),
    (
        "regularisation/heggie.rs",
        Rule::Index,
        2,
        "the step-7 state scan's `xs[i]`, a bounded index over a fixed array: to unroll when the march is first \
         compiled to SPIR-V (TASK-M4-02, TASK-M4-05)",
    ),
    (
        "regularisation/logh.rs",
        Rule::Index,
        2,
        "the step-7 state scan's `xs[i]`, a bounded index over a fixed array: to unroll when the march is first \
         compiled to SPIR-V (TASK-M4-02, TASK-M4-05)",
    ),
    (
        "regularisation/pairs.rs",
        Rule::Index,
        10,
        "the pair table, host-side interface data read by the selector, the status line and the export, never by a \
         step",
    ),
];

/// Whether `attrs` hold a cfg the SPIR-V build does not take: `not(target_arch = "spirv")`, alone or among others, or
/// `test`.
fn host_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        let text: String = a
            .meta
            .to_token_stream()
            .to_string()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        text == "cfg(test)"
            || text.contains("not(target_arch=\"spirv\")")
            || text.contains("not(any(target_arch=\"spirv\"")
    })
}

fn item_attrs(i: &syn::Item) -> &[syn::Attribute] {
    use syn::Item as I;
    match i {
        I::Const(x) => &x.attrs,
        I::Enum(x) => &x.attrs,
        I::ExternCrate(x) => &x.attrs,
        I::Fn(x) => &x.attrs,
        I::ForeignMod(x) => &x.attrs,
        I::Impl(x) => &x.attrs,
        I::Macro(x) => &x.attrs,
        I::Mod(x) => &x.attrs,
        I::Static(x) => &x.attrs,
        I::Struct(x) => &x.attrs,
        I::Trait(x) => &x.attrs,
        I::TraitAlias(x) => &x.attrs,
        I::Type(x) => &x.attrs,
        I::Union(x) => &x.attrs,
        I::Use(x) => &x.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(i: &syn::ImplItem) -> &[syn::Attribute] {
    match i {
        syn::ImplItem::Const(x) => &x.attrs,
        syn::ImplItem::Fn(x) => &x.attrs,
        syn::ImplItem::Type(x) => &x.attrs,
        syn::ImplItem::Macro(x) => &x.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(i: &syn::TraitItem) -> &[syn::Attribute] {
    match i {
        syn::TraitItem::Const(x) => &x.attrs,
        syn::TraitItem::Fn(x) => &x.attrs,
        syn::TraitItem::Type(x) => &x.attrs,
        syn::TraitItem::Macro(x) => &x.attrs,
        _ => &[],
    }
}

/// `e` printed by quote, its tokens space-separated.
fn text<T: ToTokens>(e: &T) -> String {
    e.to_token_stream().to_string()
}

/// The line `s` starts on.
fn line<T: Spanned>(s: &T) -> usize {
    s.span().start().line
}

/// Whether the index `e` is constant: an integer literal, an upper-case path (a constant), or a range of them.
fn constant_index(e: &syn::Expr) -> bool {
    match e {
        syn::Expr::Lit(l) => matches!(l.lit, syn::Lit::Int(_)),
        syn::Expr::Path(p) => {
            p.qself.is_none()
                && p.path.segments.last().is_some_and(|s| {
                    let s = s.ident.to_string();
                    s.chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                })
        }
        syn::Expr::Paren(p) => constant_index(&p.expr),
        syn::Expr::Range(r) => {
            r.start.as_deref().is_none_or(constant_index)
                && r.end.as_deref().is_none_or(constant_index)
        }
        _ => false,
    }
}

/// The name a plain-name expression reads, if `e` is one.
fn plain_name(e: &syn::Expr) -> Option<String> {
    match e {
        syn::Expr::Path(p) if p.qself.is_none() && p.path.segments.len() == 1 => {
            Some(p.path.segments[0].ident.to_string())
        }
        syn::Expr::Paren(p) => plain_name(&p.expr),
        _ => None,
    }
}

/// The bounds the binding `let name = init` places on `name`, as (lower, upper): a `.clamp(..)` both; a select
/// `if name > c { name } else { c }` (`>` or `>=`) the lower, `if name < c { name } else { c }` (`<` or `<=`) the upper.
fn bounds(name: &str, init: &syn::Expr) -> (bool, bool) {
    if let syn::Expr::MethodCall(m) = init {
        if m.method == "clamp" {
            return (true, true);
        }
    }
    let syn::Expr::If(i) = init else {
        return (false, false);
    };
    let syn::Expr::Binary(b) = &*i.cond else {
        return (false, false);
    };
    if plain_name(&b.left).as_deref() != Some(name) {
        return (false, false);
    }
    let keeps = i.then_branch.stmts.len() == 1
        && matches!(&i.then_branch.stmts[0], syn::Stmt::Expr(e, None) if plain_name(e).as_deref() == Some(name));
    if !keeps || i.else_branch.is_none() {
        return (false, false);
    }
    match b.op {
        syn::BinOp::Gt(_) | syn::BinOp::Ge(_) => (true, false),
        syn::BinOp::Lt(_) | syn::BinOp::Le(_) => (false, true),
        _ => (false, false),
    }
}

/// Collects the bounds `let` bindings of `name` before `before` (a line) place on it, anywhere in a function body.
struct Clamps<'n> {
    name: &'n str,
    before: usize,
    lower: bool,
    upper: bool,
}

impl<'ast> Visit<'ast> for Clamps<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        let bound = match &l.pat {
            syn::Pat::Ident(p) => p.ident == self.name,
            syn::Pat::Type(t) => matches!(&*t.pat, syn::Pat::Ident(p) if p.ident == self.name),
            _ => false,
        };
        if bound && line(l) < self.before {
            if let Some(init) = &l.init {
                let (lo, hi) = bounds(self.name, &init.expr);
                self.lower |= lo;
                self.upper |= hi;
            }
        }
        visit::visit_local(self, l);
    }
}

/// Collects the `Real`-dependent constants a condition holds.
struct CondScan<'p> {
    precision: &'p BTreeSet<String>,
    hits: Vec<String>,
}

impl<'ast> Visit<'ast> for CondScan<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        let segs = &p.path.segments;
        let generic = p.qself.is_some()
            || (segs.len() >= 2 && !matches!(segs[0].ident.to_string().as_str(), "f32" | "f64"));
        if let Some(last) = segs.last() {
            let name = last.ident.to_string();
            if generic && self.precision.contains(&name) {
                self.hits.push(text(p));
            }
        }
        visit::visit_expr_path(self, p);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if self.precision.contains(&m.method.to_string()) {
            self.hits.push(text(m));
        }
        visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*c.func {
            let at_width = p
                .path
                .segments
                .last()
                .is_some_and(|s| s.ident == "at_width");
            if at_width && c.args.len() == 2 && !same_value(&c.args[0], &c.args[1]) {
                self.hits.push(text(c));
            }
        }
        visit::visit_expr_call(self, c);
    }
}

/// The f64 value of a numeric literal, perhaps negated, perhaps cast (`X as f32` reads `X`'s name, not a value).
fn literal(e: &syn::Expr) -> Option<f64> {
    match e {
        syn::Expr::Lit(l) => match &l.lit {
            syn::Lit::Float(f) => f.base10_digits().parse().ok(),
            syn::Lit::Int(i) => i.base10_digits().parse().ok(),
            _ => None,
        },
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => literal(&u.expr).map(|v| -v),
        syn::Expr::Paren(p) => literal(&p.expr),
        _ => None,
    }
}

/// Whether `single`, an f32 value, and `double`, an f64 one, are the same constant: two literals where the f32 one is
/// the f64 one rounded, or the same expression once an `as f32` is dropped (`X as f32` and `X`).
fn same_value(single: &syn::Expr, double: &syn::Expr) -> bool {
    if let (Some(s), Some(d)) = (literal(single), literal(double)) {
        return (s as f32) == (d as f32);
    }
    let single = match single {
        syn::Expr::Cast(c) => &*c.expr,
        e => e,
    };
    text(single) == text(double)
}

/// The associated constants whose `f32` and `f64` impls, anywhere in `files`, give different values, with
/// [`FLOAT_LIMITS`], less [`EXCEPTED`].
fn precision_names(files: &[(String, String)]) -> BTreeSet<String> {
    use std::collections::BTreeMap;
    let mut by: BTreeMap<(String, String), (Option<syn::Expr>, Option<syn::Expr>)> =
        BTreeMap::new();
    for (_, src) in files {
        let Ok(file) = syn::parse_file(src) else {
            continue;
        };
        for item in &file.items {
            let syn::Item::Impl(imp) = item else { continue };
            let Some((_, trait_path, _)) = &imp.trait_ else {
                continue;
            };
            let ty = text(&imp.self_ty);
            if ty != "f32" && ty != "f64" {
                continue;
            }
            for it in &imp.items {
                if let syn::ImplItem::Const(c) = it {
                    let key = (text(trait_path), c.ident.to_string());
                    let slot = by.entry(key).or_default();
                    if ty == "f32" {
                        slot.0 = Some(c.expr.clone());
                    } else {
                        slot.1 = Some(c.expr.clone());
                    }
                }
            }
        }
    }
    let mut names: BTreeSet<String> = by
        .into_iter()
        .filter_map(|((_, name), v)| match v {
            (Some(s), Some(d)) if !same_value(&s, &d) => Some(name),
            _ => None,
        })
        .collect();
    names.extend(FLOAT_LIMITS.iter().map(|s| s.to_string()));
    names.remove(EXCEPTED);
    names
}

/// The lint's walk over one file.
struct Lint<'a> {
    file: &'a str,
    skip_host: bool,
    precision: &'a BTreeSet<String>,
    func: Vec<(String, Option<&'a syn::Block>)>,
    in_while: usize,
    found: Vec<Finding>,
}

impl<'a> Lint<'a> {
    fn push(&mut self, rule: Rule, at: usize, text: String) {
        self.push_cast(rule, at, text, false);
    }

    fn push_cast(&mut self, rule: Rule, at: usize, text: String, clamped: bool) {
        let func = self.func.last().map_or(String::new(), |f| f.0.clone());
        self.found.push(Finding {
            file: self.file.to_owned(),
            line: at,
            rule,
            func,
            text,
            clamped,
        });
    }

    fn condition(&mut self, cond: &syn::Expr) {
        let mut scan = CondScan {
            precision: self.precision,
            hits: Vec::new(),
        };
        scan.visit_expr(cond);
        for hit in scan.hits {
            self.push(Rule::Precision, line(cond), hit);
        }
    }

    fn in_fn(&mut self, name: String, body: Option<&'a syn::Block>, f: impl FnOnce(&mut Self)) {
        let depth = std::mem::replace(&mut self.in_while, 0);
        self.func.push((name, body));
        f(self);
        self.func.pop();
        self.in_while = depth;
    }
}

impl<'a> Visit<'a> for Lint<'a> {
    fn visit_item(&mut self, i: &'a syn::Item) {
        if !(self.skip_host && host_only(item_attrs(i))) {
            visit::visit_item(self, i);
        }
    }

    fn visit_impl_item(&mut self, i: &'a syn::ImplItem) {
        if !(self.skip_host && host_only(impl_item_attrs(i))) {
            visit::visit_impl_item(self, i);
        }
    }

    fn visit_trait_item(&mut self, i: &'a syn::TraitItem) {
        if !(self.skip_host && host_only(trait_item_attrs(i))) {
            visit::visit_trait_item(self, i);
        }
    }

    fn visit_local(&mut self, l: &'a syn::Local) {
        if !(self.skip_host && host_only(&l.attrs)) {
            visit::visit_local(self, l);
        }
    }

    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        self.in_fn(f.sig.ident.to_string(), Some(&f.block), |s| {
            visit::visit_item_fn(s, f)
        });
    }

    fn visit_impl_item_fn(&mut self, f: &'a syn::ImplItemFn) {
        self.in_fn(f.sig.ident.to_string(), Some(&f.block), |s| {
            visit::visit_impl_item_fn(s, f)
        });
    }

    fn visit_trait_item_fn(&mut self, f: &'a syn::TraitItemFn) {
        self.in_fn(f.sig.ident.to_string(), f.default.as_ref(), |s| {
            visit::visit_trait_item_fn(s, f)
        });
    }

    fn visit_expr_break(&mut self, b: &'a syn::ExprBreak) {
        self.push(Rule::Break, line(b), text(b));
        visit::visit_expr_break(self, b);
    }

    fn visit_expr_continue(&mut self, c: &'a syn::ExprContinue) {
        self.push(Rule::Break, line(c), text(c));
        visit::visit_expr_continue(self, c);
    }

    fn visit_expr_loop(&mut self, l: &'a syn::ExprLoop) {
        self.push(Rule::Loop, line(l), "loop".to_owned());
        visit::visit_expr_loop(self, l);
    }

    fn visit_expr_for_loop(&mut self, f: &'a syn::ExprForLoop) {
        self.push(
            Rule::For,
            line(f),
            format!("for {} in {}", text(&f.pat), text(&f.expr)),
        );
        visit::visit_expr_for_loop(self, f);
    }

    fn visit_expr_while(&mut self, w: &'a syn::ExprWhile) {
        self.condition(&w.cond);
        self.visit_expr(&w.cond);
        self.in_while += 1;
        self.visit_block(&w.body);
        self.in_while -= 1;
    }

    fn visit_expr_closure(&mut self, c: &'a syn::ExprClosure) {
        let depth = std::mem::replace(&mut self.in_while, 0);
        visit::visit_expr_closure(self, c);
        self.in_while = depth;
    }

    fn visit_expr_return(&mut self, r: &'a syn::ExprReturn) {
        if self.in_while > 0 {
            self.push(Rule::Exit, line(r), text(r));
        }
        visit::visit_expr_return(self, r);
    }

    fn visit_expr_try(&mut self, t: &'a syn::ExprTry) {
        if self.in_while > 0 {
            self.push(Rule::Exit, line(t), text(t));
        }
        visit::visit_expr_try(self, t);
    }

    fn visit_expr_path(&mut self, p: &'a syn::ExprPath) {
        // A qualified path (`f32::INFINITY`, `R::nan`, `<R as Float>::infinity`) or an upper-case constant imported
        // bare (`INFINITY`); a lower-case bare name is a local (`let nan = ..`).
        let named = |s: &syn::PathSegment| {
            let name = s.ident.to_string();
            NON_FINITE.contains(&name.as_str())
                && (p.qself.is_some()
                    || p.path.segments.len() > 1
                    || name.chars().all(|c| !c.is_ascii_lowercase()))
        };
        if p.path.segments.last().is_some_and(named) {
            self.push(Rule::NonFinite, line(p), text(p));
        }
        visit::visit_expr_path(self, p);
    }

    fn visit_expr_method_call(&mut self, m: &'a syn::ExprMethodCall) {
        if NON_FINITE.contains(&m.method.to_string().as_str()) {
            self.push(Rule::NonFinite, line(m), text(m));
        }
        visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_binary(&mut self, b: &'a syn::ExprBinary) {
        if matches!(b.op, syn::BinOp::Div(_)) && literal(&b.right) == Some(0.0) {
            self.push(Rule::NonFinite, line(b), text(b));
        }
        visit::visit_expr_binary(self, b);
    }

    fn visit_expr_index(&mut self, i: &'a syn::ExprIndex) {
        if !constant_index(&i.index) {
            self.push(Rule::Index, line(i), text(i));
        }
        visit::visit_expr_index(self, i);
    }

    fn visit_expr_cast(&mut self, c: &'a syn::ExprCast) {
        let to_int = matches!(&*c.ty, syn::Type::Path(t) if t.qself.is_none() && t.path.segments.len() == 1
            && INTS.contains(&t.path.segments[0].ident.to_string().as_str()));
        if to_int {
            let at = line(c);
            let clamped = match (plain_name(&c.expr), self.func.last().and_then(|f| f.1)) {
                (Some(name), Some(body)) => {
                    let mut clamps = Clamps {
                        name: &name,
                        before: at,
                        lower: false,
                        upper: false,
                    };
                    clamps.visit_block(body);
                    clamps.lower && clamps.upper
                }
                _ => false,
            };
            self.push_cast(Rule::Cast, at, text(c), clamped);
        }
        visit::visit_expr_cast(self, c);
    }

    fn visit_expr_if(&mut self, i: &'a syn::ExprIf) {
        self.condition(&i.cond);
        visit::visit_expr_if(self, i);
    }

    fn visit_arm(&mut self, a: &'a syn::Arm) {
        if let Some((_, guard)) = &a.guard {
            self.condition(guard);
        }
        visit::visit_arm(self, a);
    }
}

/// The directory a module declared in the file `rel` lives in: `rel`'s own for a `mod.rs` or `lib.rs`, otherwise `rel`
/// without its `.rs`.
fn module_dir(rel: &str) -> String {
    let path = Path::new(rel);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let parent = path
        .parent()
        .map_or(String::new(), |p| p.to_string_lossy().replace('\\', "/"));
    let dir = if name == "mod.rs" || name == "lib.rs" {
        parent
    } else {
        rel.trim_end_matches(".rs").to_owned()
    };
    if dir.is_empty() {
        dir
    } else {
        format!("{dir}/")
    }
}

/// The files of the modules `files` declare host-only (`#[cfg(not(target_arch = "spirv"))] mod x;`), below `src/`.
fn host_modules(files: &[(String, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (rel, src) in files {
        let Ok(file) = syn::parse_file(src) else {
            continue;
        };
        for item in &file.items {
            if let syn::Item::Mod(m) = item {
                if m.content.is_none() && host_only(&m.attrs) {
                    let dir = module_dir(rel);
                    out.insert(format!("{dir}{}.rs", m.ident));
                    out.insert(format!("{dir}{}/mod.rs", m.ident));
                }
            }
        }
    }
    out
}

/// Whether the file `rel` below `src/` is in a march layer.
fn in_layers(rel: &str) -> bool {
    LAYERS.iter().any(|l| rel.starts_with(&format!("{l}/")))
}

/// Whether the file `rel` below `src/` is stage code ([`STAGE`]).
fn in_stage(rel: &str) -> bool {
    STAGE.iter().any(|s| {
        if s.ends_with('/') {
            rel.starts_with(s)
        } else {
            rel == *s
        }
    })
}

/// Every finding in the march layers of `files` (every `.rs` file below `src/`, by its path there), host-only items and
/// modules skipped where `skip_host`.
fn lint(files: &[(String, String)], skip_host: bool) -> Vec<Finding> {
    let precision = precision_names(files);
    let host = if skip_host {
        host_modules(files)
    } else {
        BTreeSet::new()
    };
    let mut found = Vec::new();
    for (rel, src) in files {
        if !(in_layers(rel) || in_stage(rel)) || host.contains(rel) {
            continue;
        }
        let file =
            syn::parse_file(src).unwrap_or_else(|e| panic!("{rel}: syn could not parse it: {e}"));
        let mut walk = Lint {
            file: rel,
            skip_host,
            precision: &precision,
            func: Vec::new(),
            in_while: 0,
            found: Vec::new(),
        };
        walk.visit_file(&file);
        found.extend(walk.found);
    }
    found.sort();
    found
}

/// The kernel's source, every `.rs` file below `crates/kernel/src`, by its path there.
fn sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack: Vec<PathBuf> = vec![root.clone()];
    let mut files = Vec::new();
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("a source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let rel = path
                    .strip_prefix(&root)
                    .expect("below src")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((rel, fs::read_to_string(&path).expect("a source file")));
            }
        }
    }
    files.sort();
    files
}

/// Panics, with `rule`'s message, where the findings of `rule` in `files` are not exactly those [`ALLOWED`] admits,
/// or, for [`Rule::Cast`], not exactly [`CASTS`], each float cast clamped.
fn check(rule: Rule, files: &[(String, String)]) {
    let found: Vec<Finding> = lint(files, true)
        .into_iter()
        .filter(|f| f.rule == rule)
        .filter(|f| in_layers(&f.file) || !matches!(rule, Rule::Index | Rule::Cast))
        .collect();
    let mut wrong: Vec<String> = Vec::new();
    if rule == Rule::Cast {
        for f in &found {
            match CASTS
                .iter()
                .find(|c| c.0 == f.file && c.1 == f.func && c.2 == f.text)
            {
                None => wrong.push(format!(
                    "{}:{}: `{}` in `{}` is not in CASTS",
                    f.file, f.line, f.text, f.func
                )),
                Some(c) if c.3 == Cast::Clamped && !f.clamped => wrong.push(format!(
                    "{}:{}: `{}` in `{}` is a float cast with no clamp before it",
                    f.file, f.line, f.text, f.func
                )),
                Some(_) => {}
            }
        }
        for c in CASTS {
            // An entry for a file not in the tree is one another branch adds; it is held once its file lands.
            if !files.iter().any(|(rel, _)| rel == c.0) {
                continue;
            }
            if !found
                .iter()
                .any(|f| c.0 == f.file && c.1 == f.func && c.2 == f.text)
            {
                wrong.push(format!(
                    "CASTS lists `{}` in {}'s `{}`, which is gone",
                    c.2, c.0, c.1
                ));
            }
        }
    } else {
        let mut files_hit: BTreeSet<&str> = found.iter().map(|f| f.file.as_str()).collect();
        files_hit.extend(ALLOWED.iter().filter(|a| a.1 == rule).map(|a| a.0));
        for file in files_hit {
            let here: Vec<&Finding> = found.iter().filter(|f| f.file == file).collect();
            let allowed = ALLOWED
                .iter()
                .find(|a| a.0 == file && a.1 == rule)
                .map_or(0, |a| a.2);
            if here.len() != allowed {
                wrong.push(format!(
                    "{file}: {} found, {allowed} allowed: {}",
                    here.len(),
                    here.iter()
                        .map(|f| format!("line {} `{}` in `{}`", f.line, f.text, f.func))
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{}:\n{}",
        rule.message(),
        wrong.join("\n")
    );
}

#[cfg(feature = "controls")]
/// The kernel's source with `seeded` added as a file of the driver layer, `driver/seeded.rs`.
fn seeded(seeded: &str) -> Vec<(String, String)> {
    let mut files = sources();
    files.push(("driver/seeded.rs".to_owned(), seeded.to_owned()));
    files
}

#[test]
fn discipline_lint_no_break() {
    check(Rule::Break, &sources());
}

negative_control!(
    discipline_lint_no_break,
    "a seeded `break` out of a while loop",
    expected = "discipline: `break` or `continue` in a march layer",
    check(
        Rule::Break,
        &seeded("pub fn f(n: u32) -> u32 {\n    let mut i = 0;\n    while i < n {\n        if i == 3 {\n            break;\n        }\n        i += 1;\n    }\n    i\n}\n"),
    )
);

#[test]
fn discipline_lint_no_exit_from_a_loop_body() {
    check(Rule::Exit, &sources());
}

negative_control!(
    discipline_lint_no_exit_from_a_loop_body,
    "a seeded `return` from inside a while body",
    expected = "discipline: `return` or `?` inside a `while` body",
    check(
        Rule::Exit,
        &seeded("pub fn f(n: u32) -> u32 {\n    let mut i = 0;\n    while i < n {\n        if i == 3 {\n            return i;\n        }\n        i += 1;\n    }\n    i\n}\n"),
    )
);

#[test]
fn discipline_lint_no_bare_loop() {
    check(Rule::Loop, &sources());
}

negative_control!(
    discipline_lint_no_bare_loop,
    "a seeded bare `loop {`",
    expected = "discipline: a bare `loop` in a march layer",
    check(
        Rule::Loop,
        &seeded("pub fn f(mut i: u32) -> u32 {\n    let mut done = false;\n    loop {\n        i += 1;\n        done = done || i > 3;\n        if done { return i; }\n    }\n}\n"),
    )
);

#[test]
fn discipline_lint_no_counted_for() {
    check(Rule::For, &sources());
}

negative_control!(
    discipline_lint_no_counted_for,
    "a seeded `for s in 0..N_sub`",
    expected = "discipline: a `for` loop in a march layer",
    check(
        Rule::For,
        &seeded("pub fn f(n_sub: u32) -> u32 {\n    let mut k = 0;\n    for s in 0..n_sub {\n        k += s;\n    }\n    k\n}\n"),
    )
);

#[test]
fn discipline_lint_no_non_finite_literal() {
    check(Rule::NonFinite, &sources());
}

negative_control!(
    discipline_lint_no_non_finite_literal,
    "a min-fold seeded from `f32::INFINITY`",
    expected = "discipline: a non-finite literal in a march layer",
    check(
        Rule::NonFinite,
        &seeded("pub fn f(x: [f32; 3]) -> f32 {\n    let m = f32::INFINITY;\n    let m = if x[0] < m { x[0] } else { m };\n    m\n}\n"),
    )
);

#[test]
fn discipline_lint_no_dynamic_index() {
    check(Rule::Index, &sources());
}

negative_control!(
    discipline_lint_no_dynamic_index,
    "a seeded `r[i]` with a runtime `i`",
    expected = "discipline: a runtime index in a march layer",
    check(
        Rule::Index,
        &seeded("pub fn f(r: [f32; 3], i: usize) -> f32 {\n    r[i]\n}\n"),
    )
);

#[test]
fn discipline_lint_clamped_casts() {
    check(Rule::Cast, &sources());
}

negative_control!(
    discipline_lint_clamped_casts,
    "`Count::whole` with its upper clamp removed",
    expected = "discipline: an integer cast not classified as clamped",
    {
        let mut files = sources();
        let advance = files
            .iter_mut()
            .find(|(rel, _)| rel == "driver/advance.rs")
            .expect("driver/advance.rs");
        let clamp = "let x = if x < U32_TOP_F64 { x } else { U32_TOP_F64 };";
        assert!(
            advance.1.contains(clamp),
            "advance.rs has no f64 upper clamp to remove"
        );
        advance.1 = advance.1.replacen(clamp, "", 1);
        check(Rule::Cast, &files)
    }
);

#[test]
fn discipline_lint_precision_constants() {
    check(Rule::Precision, &sources());
}

negative_control!(
    discipline_lint_precision_constants,
    "a seeded `SYNC_EPS`, 1e-6 at f32 and 1e-15 at f64, in a branch",
    expected = "discipline: a Real-dependent constant in a branch condition",
    check(
        Rule::Precision,
        &seeded("pub trait Eps { const SYNC_EPS: Self; }\nimpl Eps for f32 { const SYNC_EPS: f32 = 1e-6; }\nimpl Eps for f64 { const SYNC_EPS: f64 = 1e-15; }\npub fn landed<R: Eps + PartialOrd>(gap: R) -> bool {\n    if gap < R::SYNC_EPS { true } else { false }\n}\n"),
    )
);

/// The scanner's cases: a source, and the rules it must find there, in order.
const CASES: &[(&str, &[Rule])] = &[
    ("fn f() { while a { if b { break; } } }", &[Rule::Break]),
    ("fn f() { while a { continue; } }", &[Rule::Break]),
    ("fn f() -> u32 { while a { return 1; } 0 }", &[Rule::Exit]),
    (
        "fn f() -> Option<u32> { while a { g()?; } None }",
        &[Rule::Exit],
    ),
    (
        "fn f() -> u32 { while a { let g = || { return 1; }; } 0 }",
        &[],
    ),
    ("fn f() -> u32 { if a { return 1; } 0 }", &[]),
    ("fn f() { loop { } }", &[Rule::Loop]),
    ("fn f() { for i in 0..3 { } }", &[Rule::For]),
    ("fn f() -> f32 { f32::INFINITY }", &[Rule::NonFinite]),
    ("fn f<R: Float>() -> R { R::nan() }", &[Rule::NonFinite]),
    (
        "fn f<R: Float>(x: Option<R>) -> R { x.unwrap_or_else(R::infinity) }",
        &[Rule::NonFinite],
    ),
    ("fn f() -> f64 { 1.0 / 0.0 }", &[Rule::NonFinite]),
    ("fn f(x: f32) -> f32 { x / 2.0 }", &[]),
    ("fn f() -> f32 { f32::from_bits(0x7f80_0000) }", &[]),
    ("fn f(r: [f32; 3]) -> f32 { r[0] + r[N] }", &[]),
    (
        "fn f(r: [f32; 3], i: usize) -> f32 { r[i] }",
        &[Rule::Index],
    ),
    ("fn f(r: [f32; 3]) -> f32 { r[1..].len() as f32 }", &[]),
    ("fn f(x: f32) -> f32 { x as f32 }", &[]),
    ("fn f(n: u32) -> u64 { n as u64 }", &[Rule::Cast]),
    (
        "fn f<R: Float>(x: R) -> u32 { if x < R::epsilon() { 1 } else { 0 } }",
        &[Rule::Precision],
    ),
    (
        "fn f<R: Float>(x: R) -> u32 { while x < R::MIN_POSITIVE { } 0 }",
        &[Rule::Precision],
    ),
    (
        "fn f<R: Float>(x: R) -> u32 { if x < R::TINY { 1 } else { 0 } }",
        &[],
    ),
    (
        "fn f(x: f32) -> u32 { if x < f32::EPSILON { 1 } else { 0 } }",
        &[],
    ),
    (
        "fn f<R: Width>(x: R) -> u32 { if x < R::at_width(1e-6, 1e-15) { 1 } else { 0 } }",
        &[Rule::Precision],
    ),
    (
        "fn f<R: Width>(x: R) -> u32 { if x < R::at_width(X as f32, X) { 1 } else { 0 } }",
        &[],
    ),
    (
        "fn f<R: Width>(x: R) -> u32 { if x < R::at_width(0.1, 0.1) { 1 } else { 0 } }",
        &[],
    ),
    (
        "fn f<R: Float>(x: R) -> R { let e = R::epsilon(); x * e }",
        &[],
    ),
    (
        "fn f() -> (bool, bool) { let nan = false; let infinity = nan; (nan, infinity) }",
        &[],
    ),
    ("fn f() -> f32 { INFINITY }", &[Rule::NonFinite]),
    (
        "fn f(n: u32) -> u32 { match n { k if k < R::EPSILON => 0, _ => 1 } }",
        &[Rule::Precision],
    ),
    (
        "#[cfg(not(target_arch = \"spirv\"))]\nfn f() { for i in 0..3 { } }",
        &[],
    ),
    ("#[cfg(test)]\nmod tests { fn f() { loop { } } }", &[]),
    (
        "impl A { #[cfg(not(target_arch = \"spirv\"))] fn f() { loop { } } }",
        &[],
    ),
    (
        "#[cfg(target_arch = \"spirv\")]\nfn f() { loop { } }",
        &[Rule::Loop],
    ),
    ("// loop { break; }\n/// f32::INFINITY\nfn f() {}", &[]),
];

/// Panics where the scan, host-only items skipped as `skip_host` says, finds other rules than a case lists.
fn check_scan(skip_host: bool) {
    for &(src, want) in CASES {
        let files = vec![("driver/case.rs".to_owned(), src.to_owned())];
        let got: Vec<Rule> = lint(&files, skip_host)
            .into_iter()
            .map(|f| f.rule)
            .collect();
        assert_eq!(got, want, "scan of `{src}`");
    }
    let lines = lint(
        &[(
            "driver/case.rs".to_owned(),
            "fn f() {}\n\n\nfn g() { loop {} }\n".to_owned(),
        )],
        skip_host,
    );
    assert_eq!(lines.len(), 1, "scan of the line case");
    assert_eq!(
        (lines[0].line, lines[0].func.as_str()),
        (4, "g"),
        "scan of the line case"
    );
    // The scope: the wrapper, an occupant, a regularised occupant, the detector and the force are read; the host's
    // table generator, a host-only module, is not.
    let files = sources();
    let read: BTreeSet<&str> = files
        .iter()
        .map(|(rel, _)| rel.as_str())
        .filter(|rel| {
            (in_layers(rel) || in_stage(rel)) && !(skip_host && host_modules(&files).contains(*rel))
        })
        .collect();
    for rel in [
        "driver/wrapper.rs",
        "occupant/rk4.rs",
        "regularisation/heggie.rs",
        "detect/collision.rs",
        "physics/force.rs",
        "decode/latent.rs",
        "stage/canon.rs",
        "canon.rs",
    ] {
        assert!(read.contains(rel), "scan of the layers: {rel} is not read");
    }
    assert!(
        !read.contains("driver/nsub_table.rs"),
        "scan of the layers: the host's driver/nsub_table.rs is read"
    );
    assert!(
        !read.contains("chart/tilt.rs"),
        "scan of the layers: chart/tilt.rs is read"
    );
    // The cast's text as quote prints it, the form `CASTS` keys on.
    let cast = lint(
        &[(
            "detect/escape_rule.rs".to_owned(),
            "fn chazy_test() -> u32 { most_separated(&ph) as u32 }".to_owned(),
        )],
        skip_host,
    );
    assert_eq!(
        cast.iter().map(|f| f.text.as_str()).collect::<Vec<_>>(),
        ["most_separated (& ph) as u32"],
        "scan of a cast's text"
    );
}

#[test]
fn discipline_lint_scan() {
    check_scan(true);
}

negative_control!(
    discipline_lint_scan,
    "a scan that reads host-only items as the SPIR-V build's",
    expected = "scan of",
    check_scan(false)
);
