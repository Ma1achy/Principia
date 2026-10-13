//! The occupant rules of `cargo xtask lint wgsl` (TASK-M1-15), in the render contract's Part 4 terms ("The occupant
//! rules `cargo xtask lint wgsl` checks"; R-72), each over naga's IR:
//! - [`shadow`], in every fragment-stage file: no function reads the `r_sh` or `p_sh` member of a `SimState*` but the
//!   generated read side's `sample_read`, which loads the shadow for `ftle` (REQ-RENDER-018; [`Rule::ShadowRead`]);
//! - [`occupant`], in a colour occupant: no payload buffer named ([`Rule::PayloadBuffer`]); the packed word,
//!   `ctx.sample.word`, used only whole as an argument of a generated accessor ([`Rule::RawWord`]); no chart, integrator
//!   or scheduler identifier read ([`Rule::ChartState`], [`Rule::IntegratorState`], [`Rule::SchedulerState`]: seams 5
//!   and 6, REQ-COL-005); and, in one that is not a debug view, every use of a read of `ftle` or `ensemble_spread`
//!   guarded by `has_ftle && s.ftle_valid` or `has_ensemble()` ([`Rule::Guard`]: REQ-RENDER-015, R-145, R-255).
//!
//! **The guard over naga's IR.** naga lowers `a && b` to control flow: a local set to `b` where `a` holds and to
//! `false` where it does not, then read. So the rule asks what a condition implies: [`Guard::facts`] gives the facts a
//! boolean expression implies when it is true, or false, following a local through every store to it, each with the
//! conditions it is stored under. A use sits under the conditions of the `if`s whose branch holds it, and after an `if`
//! one branch of which leaves the block (`return`, `discard`, `break`, `continue`), under the other branch's. Every
//! statement's consumed expressions are uses: a `return`'s value, a store's value, a call's arguments and a branch's
//! condition. `has_ftle` is the prelude's tier constant, which the lint presents as an override
//! (`super::tier_overrides`), so that naga does not fold `!has_ftle` into a literal. An expression carries the reads its
//! operands carry, but a local's load carries none, since every store into it was checked as a use, and the absence
//! test, a `bitcast<u32>` compared by `==` or `!=` with the canonical quiet NaN's bits, carries none (lowering Part
//! 3a).

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, HashSet};

use naga::valid::{FunctionInfo, ModuleInfo};
use naga::{
    AddressSpace, BinaryOperator, Block, Expression, Function, Handle, Literal, Module, ScalarKind,
    Span, Statement, TypeInner, UnaryOperator,
};

use super::{Finding, Rule};

/// The canonical quiet NaN's f32 bits, which the absence test compares against (lowering Part 3a, "The sentinels'
/// values"), the ledger's.
const QNAN_BITS: u32 = ledger::payload::canonical_qnan_bits();

/// The read-side `SimState` (lowering Part 3a), whose `ftle`, `ftle_valid`, `ensemble_spread` and `word` the rules
/// read, and the prefix of every `SimState*` struct, stored or read-side, whose shadow the shadow rule reads.
const SIMSTATE: &str = "SimState";

/// The shadow's members (dd_generation_root §3.5).
const SHADOW: [&str; 2] = ["r_sh", "p_sh"];

/// The stain context's quad lane (render contract Part 1) and its members that are not the scheduler's: the raster's
/// addressing and the quad's frame in the slice plane, the chart's (render contract Part 4).
const QUAD_LANE: &str = "CtxQuad";
const QUAD_ADDRESSING: [&str; 3] = ["index", "uv", "sample_count"];
const QUAD_FRAME: [&str; 3] = ["tl", "centre", "half_width"];

/// The read side's integrator parameters (render contract Part 4): the macro-step, and Benettin's δ₀ and
/// renormalisation interval, the arguments `ftle` and `diffusion` are derived with.
const READ_PARAMS: &str = "ReadParams";
const INTEGRATOR: [&str; 3] = ["dt_macro", "delta_0", "n_renorm"];

/// The rule a read of `structure.member` breaks in a colour occupant (render contract Part 4): the chart lane
/// (`Ctx.chart`) and the quad's frame are the chart's; the integrator's parameters the integrator's; every other member
/// of the quad lane, and of `RenderQuad`, the scheduler's.
fn denied(structure: &str, member: &str) -> Option<Rule> {
    match structure {
        "Ctx" if member == "chart" => Some(Rule::ChartState),
        QUAD_LANE if QUAD_FRAME.contains(&member) => Some(Rule::ChartState),
        QUAD_LANE if !QUAD_ADDRESSING.contains(&member) => Some(Rule::SchedulerState),
        "RenderQuad" => Some(Rule::SchedulerState),
        READ_PARAMS if INTEGRATOR.contains(&member) => Some(Rule::IntegratorState),
        _ => None,
    }
}

/// Every function and entry point of `module`, with its validation info.
fn each_function<'m>(
    module: &'m Module,
    info: &'m ModuleInfo,
) -> impl Iterator<Item = (&'m Function, &'m FunctionInfo)> {
    module.functions.iter().map(|(h, f)| (f, &info[h])).chain(
        module
            .entry_points
            .iter()
            .enumerate()
            .map(|(i, ep)| (&ep.function, info.get_entry_point(i))),
    )
}

/// A finding of `rule` at `span`'s line in `source`.
fn at(rule: Rule, what: String, span: Span, source: &str) -> Finding {
    Finding {
        rule,
        what,
        line: span.is_defined().then(|| span.location(source).line_number),
    }
}

/// The struct and member names `e` reads, where it is a member access on a struct value or a pointer to one.
fn member<'m>(
    module: &'m Module,
    function: &Function,
    fi: &FunctionInfo,
    e: Handle<Expression>,
) -> Option<(&'m str, &'m str)> {
    let Expression::AccessIndex { base, index } = function.expressions[e] else {
        return None;
    };
    let ty = match fi[base].ty {
        naga::proc::TypeResolution::Handle(h) => h,
        naga::proc::TypeResolution::Value(TypeInner::Pointer { base, .. }) => base,
        naga::proc::TypeResolution::Value(_) => return None,
    };
    let ty = match module.types[ty].inner {
        TypeInner::Pointer { base, .. } => base,
        _ => ty,
    };
    let t = &module.types[ty];
    let TypeInner::Struct { ref members, .. } = t.inner else {
        return None;
    };
    Some((
        t.name.as_deref()?,
        members.get(index as usize)?.name.as_deref()?,
    ))
}

/// The expressions `e` reads.
fn operands(e: &Expression) -> Vec<Handle<Expression>> {
    match *e {
        Expression::Access { base, index } => vec![base, index],
        Expression::AccessIndex { base, .. } => vec![base],
        Expression::Splat { value, .. } => vec![value],
        Expression::Swizzle { vector, .. } => vec![vector],
        Expression::Compose { ref components, .. } => components.clone(),
        Expression::Load { pointer } => vec![pointer],
        Expression::Unary { expr, .. }
        | Expression::Derivative { expr, .. }
        | Expression::As { expr, .. } => vec![expr],
        Expression::Binary { left, right, .. } => vec![left, right],
        Expression::Select {
            condition,
            accept,
            reject,
        } => vec![condition, accept, reject],
        Expression::Relational { argument, .. } => vec![argument],
        Expression::Math {
            arg,
            arg1,
            arg2,
            arg3,
            ..
        } => [Some(arg), arg1, arg2, arg3]
            .into_iter()
            .flatten()
            .collect(),
        // `arrayLength`'s operand, a pointer to a storage buffer's runtime-sized array, is never a `SimState` member
        // nor the word: none of the others reads one.
        _ => Vec::new(),
    }
}

/// The expressions statement `s` consumes itself, not those of the blocks it holds.
fn consumed(s: &Statement) -> Vec<Handle<Expression>> {
    match *s {
        Statement::If { condition, .. } => vec![condition],
        Statement::Switch { selector, .. } => vec![selector],
        Statement::Loop { break_if, .. } => break_if.into_iter().collect(),
        Statement::Return { value } => value.into_iter().collect(),
        Statement::Store { value, .. } => vec![value],
        Statement::Call { ref arguments, .. } => arguments.clone(),
        _ => Vec::new(),
    }
}

/// Where statement `s`, at `span`, consumes its expressions: a loop's at its `break if` condition, which naga spans
/// at the loop's head, every other statement at its own span.
fn consumed_at(function: &Function, s: &Statement, span: Span) -> Span {
    match *s {
        Statement::Loop {
            break_if: Some(e), ..
        } => function.expressions.get_span(e),
        _ => span,
    }
}

/// A branch condition and the value it has where a statement runs.
type Path = Vec<(Handle<Expression>, bool)>;

/// Whether `block` always leaves the block holding it: its last statement returns, discards, breaks or continues.
fn leaves(block: &Block) -> bool {
    matches!(
        block.last(),
        Some(Statement::Return { .. } | Statement::Kill | Statement::Break | Statement::Continue)
    )
}

/// Each statement of `block`, recursively, with its span and the conditions it runs under, to `visit`: those of the
/// `if`s whose branch holds it, and, after an `if` one branch of which leaves the block, the other branch's.
fn walk<'a>(block: &'a Block, path: &mut Path, visit: &mut dyn FnMut(&'a Statement, Span, &Path)) {
    let depth = path.len();
    for (s, span) in block.span_iter() {
        visit(s, *span, path);
        match *s {
            Statement::Block(ref b) => walk(b, path, visit),
            Statement::If {
                condition,
                ref accept,
                ref reject,
            } => {
                for (branch, value) in [(accept, true), (reject, false)] {
                    path.push((condition, value));
                    walk(branch, path, visit);
                    path.pop();
                }
                if leaves(accept) {
                    path.push((condition, false));
                } else if leaves(reject) {
                    path.push((condition, true));
                }
            }
            Statement::Switch { ref cases, .. } => {
                for case in cases {
                    walk(&case.body, path, visit);
                }
            }
            Statement::Loop {
                ref body,
                ref continuing,
                ..
            } => {
                walk(body, path, visit);
                walk(continuing, path, visit);
            }
            _ => {}
        }
    }
    path.truncate(depth);
}

/// Each shadow read in `module` (REQ-RENDER-018): a read of `r_sh` or `p_sh` on a `SimState*` in any function but
/// `exempt`, the read side's `sample_read`, where the module holds it.
pub(super) fn shadow(
    module: &Module,
    info: &ModuleInfo,
    source: &str,
    exempt: Option<&str>,
) -> Vec<Finding> {
    let mut found = Vec::new();
    for (function, fi) in each_function(module, info) {
        let fname = function.name.as_deref().unwrap_or("(unnamed)");
        if exempt == Some(fname) {
            continue;
        }
        for (h, _) in function.expressions.iter() {
            let Some((s, m)) = member(module, function, fi, h) else {
                continue;
            };
            if s.starts_with(SIMSTATE) && SHADOW.contains(&m) {
                found.push(at(
                    Rule::ShadowRead,
                    format!(
                        "`{fname}` reads `{s}.{m}`, the Benettin shadow, as a sample: a render path reads `ftle`, \
                         the shadow's one product (REQ-RENDER-018; dd_generation_root §3.5)"
                    ),
                    function.expressions.get_span(h),
                    source,
                ));
            }
        }
    }
    found
}

/// Every occupant-rule finding in `module`, a colour occupant after its context: the payload-buffer, raw-word and
/// deny-list rules, and, where `guarded`, the guard rule. `accessors` are the generated functions (the unpack layer's
/// and the read side's).
pub(super) fn occupant(
    module: &Module,
    info: &ModuleInfo,
    source: &str,
    accessors: &BTreeSet<String>,
    guarded: bool,
) -> Vec<Finding> {
    let mut found = Vec::new();
    for (function, fi) in each_function(module, info) {
        let fname = function.name.as_deref().unwrap_or("(unnamed)");
        let span = |h| function.expressions.get_span(h);
        for (h, e) in function.expressions.iter() {
            if let Expression::GlobalVariable(g) = *e {
                let global = &module.global_variables[g];
                if matches!(global.space, AddressSpace::Storage { .. }) {
                    let name = global.name.as_deref().unwrap_or("(unnamed)");
                    found.push(at(
                        Rule::PayloadBuffer,
                        format!(
                            "`{fname}` names the payload buffer `{name}`: a colour occupant reads the payload \
                             through the generated accessors, `ctx.sample` and `ctx.ic` (seam 5, REQ-COL-005)"
                        ),
                        span(h),
                        source,
                    ));
                }
            }
            let Some((s, m)) = member(module, function, fi, h) else {
                continue;
            };
            if let Some(rule) = denied(s, m) {
                let whose = match rule {
                    Rule::ChartState => "the chart's",
                    Rule::IntegratorState => "the integrator's",
                    _ => "the scheduler's",
                };
                found.push(at(
                    rule,
                    format!(
                        "`{fname}` reads `{s}.{m}`, {whose} state: no colour equation references chart, integrator \
                         or scheduler state (seam 6, REQ-COL-005)"
                    ),
                    span(h),
                    source,
                ));
            }
        }
        found.extend(raw_word(module, function, fi, source, accessors));
        if guarded {
            found.extend(Guard::new(module, function, fi).check(source));
        }
    }
    found
}

/// The uses of the packed word in `function` other than whole, as an argument of a generated accessor (seam 5): an
/// expression reading it (indexing, masking, shifting), a non-accessor call given it, or another statement consuming it.
fn raw_word(
    module: &Module,
    function: &Function,
    fi: &FunctionInfo,
    source: &str,
    accessors: &BTreeSet<String>,
) -> Vec<Finding> {
    let fname = function.name.as_deref().unwrap_or("(unnamed)");
    let is_word =
        |h: Handle<Expression>| member(module, function, fi, h) == Some((SIMSTATE, "word"));
    let pointer = |h: Handle<Expression>| {
        matches!(
            fi[h].ty.inner_with(&module.types),
            TypeInner::Pointer { .. } | TypeInner::ValuePointer { .. }
        )
    };
    // The pointers to the word or into it, a limb's included: an expression's operands precede it, so one pass in
    // handle order finds them all.
    let mut into: HashSet<Handle<Expression>> = HashSet::new();
    for (h, e) in function.expressions.iter() {
        let derived = match *e {
            Expression::AccessIndex { base, .. } | Expression::Access { base, .. } => {
                into.contains(&base)
            }
            _ => false,
        };
        if pointer(h) && (is_word(h) || derived) {
            into.insert(h);
        }
    }
    let words: Vec<Handle<Expression>> = function
        .expressions
        .iter()
        .filter_map(|(h, e)| match *e {
            Expression::AccessIndex { .. } if is_word(h) && !pointer(h) => Some(h),
            Expression::Load { pointer } if is_word(pointer) => Some(h),
            _ => None,
        })
        .collect();
    // A load through a pointer into the word, a limb's, decodes it as `ctx.sample.word.x` does.
    let limbs = function.expressions.iter().filter(|(_, e)| {
        matches!(**e, Expression::Load { pointer } if into.contains(&pointer) && !is_word(pointer))
    });
    let what = |how: String| {
        format!(
            "`{fname}` {how} `SimState.word`: the packed word is decoded only by a generated accessor, given it \
             whole (seam 5, REQ-COL-005)"
        )
    };
    let mut found: Vec<Finding> = limbs
        .map(|(h, _)| {
            let span = function.expressions.get_span(h);
            at(Rule::RawWord, what("decodes".into()), span, source)
        })
        .collect();
    if words.is_empty() {
        return found;
    }
    for (h, e) in function.expressions.iter() {
        if operands(e).iter().any(|o| words.contains(o)) {
            found.push(at(
                Rule::RawWord,
                what("decodes".into()),
                function.expressions.get_span(h),
                source,
            ));
        }
    }
    walk(&function.body, &mut Vec::new(), &mut |s, span, _| {
        if !consumed(s).iter().any(|c| words.contains(c)) {
            return;
        }
        let how = match *s {
            Statement::Call {
                function: callee, ..
            } => {
                let name = module.functions[callee].name.as_deref().unwrap_or("");
                if accessors.contains(name) {
                    return;
                }
                format!("hands `{name}`, not a generated accessor,")
            }
            _ => "uses".into(),
        };
        found.push(at(
            Rule::RawWord,
            what(how),
            consumed_at(function, s, span),
            source,
        ));
    });
    found
}

/// For each load of a local in `block`, whether a store has certainly overwritten the local's initial value there:
/// `stored` holds the locals stored on every path to the statement; an `if` keeps those both its branches store, or
/// those of the one that does not leave; a loop's or a switch's stores count inside it only.
fn definitely_assigned(
    function: &Function,
    block: &Block,
    stored: &mut BTreeSet<Handle<naga::LocalVariable>>,
    out: &mut HashMap<Handle<Expression>, bool>,
) {
    let local = |h: Handle<Expression>| match function.expressions[h] {
        Expression::LocalVariable(l) => Some(l),
        _ => None,
    };
    for s in block.iter() {
        match *s {
            Statement::Emit(ref range) => {
                for h in range.clone() {
                    if let Expression::Load { pointer } = function.expressions[h] {
                        if let Some(l) = local(pointer) {
                            out.insert(h, stored.contains(&l));
                        }
                    }
                }
            }
            Statement::Store { pointer, .. } => {
                if let Some(l) = local(pointer) {
                    stored.insert(l);
                }
            }
            Statement::Block(ref b) => definitely_assigned(function, b, stored, out),
            Statement::If {
                ref accept,
                ref reject,
                ..
            } => {
                let mut a = stored.clone();
                definitely_assigned(function, accept, &mut a, out);
                let mut r = stored.clone();
                definitely_assigned(function, reject, &mut r, out);
                *stored = match (leaves(accept), leaves(reject)) {
                    (true, false) => r,
                    (false, true) => a,
                    _ => a.intersection(&r).copied().collect(),
                };
            }
            Statement::Switch { ref cases, .. } => {
                for case in cases {
                    definitely_assigned(function, &case.body, &mut stored.clone(), out);
                }
            }
            Statement::Loop {
                ref body,
                ref continuing,
                ..
            } => {
                let mut inside = stored.clone();
                definitely_assigned(function, body, &mut inside, out);
                definitely_assigned(function, continuing, &mut inside, out);
            }
            _ => {}
        }
    }
}

/// What a boolean implies: that this tier has FTLE (`has_ftle`) or an ensemble (`has_ensemble()`), or that the sample
/// keyed so has a valid `ftle` (`s.ftle_valid`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fact {
    HasFtle,
    HasEnsemble,
    FtleValid(String),
}

/// The facts that hold somewhere; `None` for every fact, where nothing runs (a `false` taken as true).
type Facts = Option<BTreeSet<Fact>>;

fn none() -> Facts {
    Some(BTreeSet::new())
}

fn union(a: Facts, b: Facts) -> Facts {
    match (a, b) {
        (Some(mut a), Some(b)) => {
            a.extend(b);
            Some(a)
        }
        _ => None,
    }
}

fn intersection(a: Facts, b: Facts) -> Facts {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect()),
        (a, None) => a,
        (None, b) => b,
    }
}

/// A read a use of which needs a guard: `ftle` of the sample keyed so, or `ensemble_spread`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Read {
    Ftle(String),
    Ensemble,
}

impl Read {
    /// Whether `facts` guard a use of the read.
    fn guarded_by(&self, facts: &Facts) -> bool {
        let Some(facts) = facts else {
            return true;
        };
        match self {
            Read::Ftle(key) => {
                facts.contains(&Fact::HasFtle) && facts.contains(&Fact::FtleValid(key.clone()))
            }
            Read::Ensemble => facts.contains(&Fact::HasEnsemble),
        }
    }
}

/// The guard rule over one function.
struct Guard<'a> {
    module: &'a Module,
    function: &'a Function,
    fi: &'a FunctionInfo,
    /// Each local's stores: the value and the conditions it is stored under.
    stores: HashMap<Handle<naga::LocalVariable>, Vec<(Path, Handle<Expression>)>>,
    /// Each load of a local: whether a store has certainly overwritten its initial value where it is loaded.
    assigned: HashMap<Handle<Expression>, bool>,
    /// The locals whose facts are being found, so that a cycle of stores ends.
    busy: RefCell<Vec<(Handle<naga::LocalVariable>, bool)>>,
}

impl<'a> Guard<'a> {
    fn new(module: &'a Module, function: &'a Function, fi: &'a FunctionInfo) -> Guard<'a> {
        let mut stores: HashMap<_, Vec<_>> = HashMap::new();
        walk(&function.body, &mut Vec::new(), &mut |s, _, path| {
            if let Statement::Store { pointer, value } = *s {
                if let Expression::LocalVariable(l) = function.expressions[pointer] {
                    stores.entry(l).or_default().push((path.to_vec(), value));
                }
            }
        });
        let mut assigned = HashMap::new();
        definitely_assigned(
            function,
            &function.body,
            &mut BTreeSet::new(),
            &mut assigned,
        );
        Guard {
            module,
            function,
            fi,
            stores,
            assigned,
            busy: RefCell::new(Vec::new()),
        }
    }

    /// `e`'s identity as a sample: the same argument, variable or call result and the same member path give the same
    /// key, so `ctx.sample.ftle` and `ctx.sample.ftle_valid` read one sample, and so do two loads of one pointer. naga
    /// makes one expression per argument and per local variable, so its handle keys either.
    fn key(&self, e: Handle<Expression>) -> String {
        match self.function.expressions[e] {
            Expression::AccessIndex { base, index } => format!("{}.{index}", self.key(base)),
            Expression::Load { pointer } => format!("*{}", self.key(pointer)),
            _ => format!("e{}", e.index()),
        }
    }

    /// The read `e` is, if it reads `ftle` or `ensemble_spread` of a `SimState`.
    fn read(&self, e: Handle<Expression>) -> Option<Read> {
        let Expression::AccessIndex { base, .. } = self.function.expressions[e] else {
            return None;
        };
        match member(self.module, self.function, self.fi, e)? {
            (SIMSTATE, "ftle") => Some(Read::Ftle(self.key(base))),
            (SIMSTATE, "ensemble_spread") => Some(Read::Ensemble),
            _ => None,
        }
    }

    /// Whether `left` and `right` are the absence test: one the `bitcast<u32>` of a value, the other the canonical quiet
    /// NaN's bits, `0x7fc00000u` or `CANONICAL_QNAN_BITS` (lowering Part 3a, "The sentinels' values").
    fn absence_test(&self, left: Handle<Expression>, right: Handle<Expression>) -> bool {
        let bitcast = |h: Handle<Expression>| {
            matches!(
                self.function.expressions[h],
                Expression::As {
                    kind: ScalarKind::Uint,
                    convert: None,
                    ..
                }
            )
        };
        (bitcast(left) && self.qnan_bits(right)) || (bitcast(right) && self.qnan_bits(left))
    }

    /// Whether `e` is the canonical quiet NaN's bits: the literal, or a constant whose value is it.
    fn qnan_bits(&self, e: Handle<Expression>) -> bool {
        let literal = |e: &Expression| matches!(*e, Expression::Literal(Literal::U32(QNAN_BITS)));
        match self.function.expressions[e] {
            Expression::Constant(c) => {
                literal(&self.module.global_expressions[self.module.constants[c].init])
            }
            ref other => literal(other),
        }
    }

    /// The guarded reads `e` carries: its own, and its operands'; none through a local's load, a call's result or the
    /// absence test.
    fn reads(&self, e: Handle<Expression>) -> BTreeSet<Read> {
        if let Some(read) = self.read(e) {
            return BTreeSet::from([read]);
        }
        let expr = &self.function.expressions[e];
        match *expr {
            Expression::Binary {
                op: BinaryOperator::Equal | BinaryOperator::NotEqual,
                left,
                right,
            } if self.absence_test(left, right) => BTreeSet::new(),
            _ => operands(expr)
                .into_iter()
                .flat_map(|o| self.reads(o))
                .collect(),
        }
    }

    /// The facts `e`, a boolean, implies where it is `value`.
    fn facts(&self, e: Handle<Expression>, value: bool) -> Facts {
        match self.function.expressions[e] {
            Expression::Literal(Literal::Bool(b)) if b != value => None,
            Expression::Override(o)
                if value && self.module.overrides[o].name.as_deref() == Some("has_ftle") =>
            {
                Some(BTreeSet::from([Fact::HasFtle]))
            }
            Expression::CallResult(f)
                if value && self.module.functions[f].name.as_deref() == Some("has_ensemble") =>
            {
                Some(BTreeSet::from([Fact::HasEnsemble]))
            }
            Expression::Unary {
                op: UnaryOperator::LogicalNot,
                expr,
            } => self.facts(expr, !value),
            Expression::Load { pointer } => match self.function.expressions[pointer] {
                Expression::LocalVariable(l) => {
                    let assigned = self.assigned.get(&e).copied().unwrap_or(false);
                    self.local(l, value, assigned)
                }
                _ if value => self.valid(pointer),
                _ => none(),
            },
            Expression::AccessIndex { .. } if value => self.valid(e),
            _ => none(),
        }
    }

    /// `{FtleValid}` where `e` reads `ftle_valid` of a `SimState`, keyed by its sample.
    fn valid(&self, e: Handle<Expression>) -> Facts {
        let Expression::AccessIndex { base, .. } = self.function.expressions[e] else {
            return none();
        };
        match member(self.module, self.function, self.fi, e) {
            Some((SIMSTATE, "ftle_valid")) => {
                Some(BTreeSet::from([Fact::FtleValid(self.key(base))]))
            }
            _ => none(),
        }
    }

    /// The facts local `l` implies where its load is `value`: those common to every store into it, each with the
    /// conditions it is stored under, and, unless a store has certainly overwritten it (`assigned`), to its initial
    /// value (zero, `false`, with no initialiser).
    fn local(&self, l: Handle<naga::LocalVariable>, value: bool, assigned: bool) -> Facts {
        if self.busy.borrow().contains(&(l, value)) {
            return none();
        }
        self.busy.borrow_mut().push((l, value));
        let mut out = match self.function.local_variables[l].init {
            _ if assigned => None,
            Some(init) => self.facts(init, value),
            None if value => None,
            None => none(),
        };
        for (path, stored) in self.stores.get(&l).into_iter().flatten() {
            out = intersection(out, union(self.path(path), self.facts(*stored, value)));
        }
        self.busy.borrow_mut().pop();
        out
    }

    /// The facts the conditions `path` imply.
    fn path(&self, path: &[(Handle<Expression>, bool)]) -> Facts {
        path.iter()
            .fold(none(), |acc, &(c, v)| union(acc, self.facts(c, v)))
    }

    /// Every unguarded use of a guarded read in the function.
    fn check(&self, source: &str) -> Vec<Finding> {
        let fname = self.function.name.as_deref().unwrap_or("(unnamed)");
        let mut found = Vec::new();
        walk(
            &self.function.body,
            &mut Vec::new(),
            &mut |s, span, path| {
                let reads: BTreeSet<Read> = consumed(s)
                    .into_iter()
                    .flat_map(|c| self.reads(c))
                    .collect();
                if reads.is_empty() {
                    return;
                }
                let facts = self.path(path);
                for read in reads.iter().filter(|r| !r.guarded_by(&facts)) {
                    let what = match read {
                    Read::Ftle(_) => {
                        "a read of `ftle` outside `has_ftle && s.ftle_valid` for its sample `s`: FTLE views guard \
                         with both, and an aggregate over `ftle` excludes samples by `ftle_valid`, never by NaN \
                         propagation (REQ-RENDER-015, R-255)"
                    }
                    Read::Ensemble => {
                        "a read of `ensemble_spread` outside `has_ensemble()`, the uniform's guard (REQ-RENDER-015, \
                         R-145)"
                    }
                };
                    let span = consumed_at(self.function, s, span);
                    found.push(at(
                        Rule::Guard,
                        format!("`{fname}` uses {what}"),
                        span,
                        source,
                    ));
                }
            },
        );
        found
    }
}
