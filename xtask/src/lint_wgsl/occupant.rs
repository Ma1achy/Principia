//! The occupant rules of `cargo xtask lint wgsl` (TASK-M1-15), in the render contract's Part 4 terms ("The occupant
//! rules `cargo xtask lint wgsl` checks"; R-72), each over naga's IR:
//! - [`shadow`], in every fragment-stage file: no function reads the `r_sh` or `p_sh` member of a `SimState*` but the
//!   generated read side's `sample_read`, which loads the shadow for `ftle` (REQ-RENDER-018; [`Rule::ShadowRead`]);
//! - [`occupant`], in a colour occupant: no payload buffer named ([`Rule::PayloadBuffer`]); the packed word,
//!   `ctx.sample.word`, used only whole as an argument of a generated accessor ([`Rule::RawWord`]); no chart, integrator
//!   or scheduler identifier read ([`Rule::ChartState`], [`Rule::IntegratorState`], [`Rule::SchedulerState`]: seams 5
//!   and 6, REQ-COL-005); and every use of a read of `ftle` or `ensemble_spread` guarded by `has_ftle && s.ftle_valid`
//!   or `has_ensemble()` ([`Rule::Guard`]: REQ-RENDER-015, R-145, R-255), in a debug view or a source every read but
//!   of the sample's own value, `ctx.sample`'s, which it shows literally (R-79) or forwards, so that its aggregates are
//!   held to R-255.
//!
//! **The guard over naga's IR.** naga lowers `a && b` to control flow: a local set to `b` where `a` holds and to
//! `false` where it does not, then read. So the rule asks what a condition implies: [`Guard::facts`] gives the facts a
//! boolean expression implies when it is true, or false, following a local through every store that may reach the load
//! ([`Flow`]), each with the conditions it is stored under. A sample's validity holds only while what it was read from
//! is unchanged: a store into its variable, whole, by a member, through a pointer or by a function given a pointer,
//! after `ftle_valid` was read, ends it for every statement after the store, and in a loop for every statement of the
//! loop; so does the sample's evaluation again, a call's result or another expression's, on a later run of a loop.
//! A use sits under the conditions of the `if`s whose branch holds it, and after an `if`
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
    AddressSpace, BinaryOperator, Block, Expression, Function, Handle, Literal, LocalVariable,
    Module, ScalarKind, Span, Statement, TypeInner, UnaryOperator,
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

/// The stain context's struct (render contract Part 1), whose `sample` is the sample the occupant is given.
const CTX: &str = "Ctx";
const SAMPLE: &str = "sample";
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

/// Every occupant-rule finding in `module`, a colour occupant after its context, but the guard rule's ([`guard`]): the
/// payload-buffer, raw-word and deny-list rules. `accessors` are the generated functions (the unpack layer's and the
/// read side's).
pub(super) fn occupant(
    module: &Module,
    info: &ModuleInfo,
    source: &str,
    accessors: &BTreeSet<String>,
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
    }
    found
}

/// Each function of `module` with its validation info, its handle (`None` for an entry point) and the guard rule over
/// it: where `own` (a debug view's or a source's), passing over the reads of the sample's own value, `ctx.sample`'s
/// ([`contexts`]).
fn guards<'m>(module: &'m Module, info: &'m ModuleInfo, own: bool) -> Vec<Guard<'m>> {
    let contexts = if own {
        contexts(module)
    } else {
        HashSet::new()
    };
    let handles = module
        .functions
        .iter()
        .map(|(h, _)| Some(h))
        .chain(module.entry_points.iter().map(|_| None));
    each_function(module, info)
        .zip(handles)
        .map(|((function, fi), handle)| {
            let own: HashSet<u32> = contexts
                .iter()
                .filter(|&&(f, _)| Some(f) == handle)
                .map(|&(_, i)| i)
                .collect();
            Guard::new(module, function, fi, own)
        })
        .collect()
}

/// The guard rule's findings in `module`, a colour occupant after its context with the tier constants read as
/// overrides (`super::tier_overrides`): every use of a read of `ftle` or `ensemble_spread` outside its guard, but, where
/// `own`, of the sample's own value.
pub(super) fn guard(module: &Module, info: &ModuleInfo, source: &str, own: bool) -> Vec<Finding> {
    guards(module, info, own)
        .iter()
        .flat_map(|g| g.check(source))
        .collect()
}

/// The guard rule's findings in `module`, the occupant as assembled, where it does not validate with its tier constants
/// read as overrides (`why`): it uses one where only a constant may stand, a `const` declaration's initialiser, so the
/// rule cannot see the tier's guard. Each function's first read the rule would check is reported, unchecked.
pub(super) fn unguardable(
    module: &Module,
    info: &ModuleInfo,
    source: &str,
    own: bool,
    why: &str,
) -> Vec<Finding> {
    let why = why.lines().next().unwrap_or("").trim();
    let mut found = Vec::new();
    for g in guards(module, info, own) {
        let fname = g.function.name.as_deref().unwrap_or("(unnamed)");
        let first = g
            .function
            .expressions
            .iter()
            .find_map(|(h, _)| g.read(h).map(|r| (h, r)));
        if let Some((h, read)) = first {
            let what = match read {
                Read::Ftle(_) => "`ftle`",
                Read::Ensemble => "`ensemble_spread`",
            };
            found.push(at(
                Rule::Guard,
                format!(
                    "`{fname}` reads {what}, but its guard cannot be checked: the occupant uses a tier constant \
                     where only a constant may stand, so the lint cannot read it as the guard does ({why}); read \
                     `has_ftle` in the branch condition itself, not through a `const` (REQ-RENDER-015)"
                ),
                g.function.expressions.get_span(h),
                source,
            ));
        }
    }
    found
}

/// The arguments, each a function and an index, that are the stain's context the assembler gives the occupant: every
/// `Ctx` argument of a function that no call in `module` gives anything but such an argument of its caller. A read of
/// `ftle` or `ensemble_spread` on such an argument's `sample` is the sample's own value (render contract Part 4).
fn contexts(module: &Module) -> HashSet<(Handle<Function>, u32)> {
    let is_ctx = |ty: Handle<naga::Type>| module.types[ty].name.as_deref() == Some(CTX);
    let mut set: HashSet<(Handle<Function>, u32)> = HashSet::new();
    for (h, f) in module.functions.iter() {
        for (i, a) in (0u32..).zip(&f.arguments) {
            if is_ctx(a.ty) {
                set.insert((h, i));
            }
        }
    }
    let callers: Vec<(Option<Handle<Function>>, &Function)> = module
        .functions
        .iter()
        .map(|(h, f)| (Some(h), f))
        .chain(module.entry_points.iter().map(|ep| (None, &ep.function)))
        .collect();
    // Each round but the last removes an argument, so the set's size bounds the rounds.
    for _ in 0..=set.len() {
        let mut gone = Vec::new();
        for &(caller, f) in &callers {
            walk(&f.body, &mut Vec::new(), &mut |s, _, _| {
                let Statement::Call {
                    function: callee,
                    ref arguments,
                    ..
                } = *s
                else {
                    return;
                };
                for (j, &a) in (0u32..).zip(arguments) {
                    let passed = match f.expressions[a] {
                        Expression::FunctionArgument(i) => {
                            caller.is_some_and(|c| set.contains(&(c, i)))
                        }
                        _ => false,
                    };
                    if set.contains(&(callee, j)) && !passed {
                        gone.push((callee, j));
                    }
                }
            });
        }
        if gone.is_empty() {
            break;
        }
        for g in gone {
            set.remove(&g);
        }
    }
    set
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

/// The roots a sample's key is read from ([`Guard::key`]): the variables, arguments, call results and other evaluated
/// expressions at the bottom of its accesses. A store into one changes what the key reads.
type Roots = BTreeSet<Handle<Expression>>;

/// A sample's identity ([`Guard::key`]): its text, and the roots it is read from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    text: String,
    roots: Roots,
}

/// What a boolean implies: that this tier has FTLE (`has_ftle`) or an ensemble (`has_ensemble()`), or that the sample
/// keyed so has a valid `ftle` (`s.ftle_valid`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fact {
    HasFtle,
    HasEnsemble,
    FtleValid(Key),
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

/// `facts` without the validity of any sample read from a root in `changed`: a store into it since the facts were
/// taken may have made it another sample.
fn without(facts: Facts, changed: &Roots) -> Facts {
    facts.map(|mut facts| {
        facts.retain(|f| !matches!(f, Fact::FtleValid(k) if !k.roots.is_disjoint(changed)));
        facts
    })
}

/// A read a use of which needs a guard: `ftle` of the sample keyed so, or `ensemble_spread`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Read {
    Ftle(Key),
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

/// A store that may reach a local's load (`None`: the local's initial value), and the roots changed since it ran.
#[derive(Clone, Debug)]
struct Reach {
    store: Option<usize>,
    changed: Roots,
}

/// Roots changed since each load in scope was evaluated: a fact a read through it implies about a sample read from one
/// of them no longer holds.
type Evals = HashMap<Handle<Expression>, Roots>;

/// Where a statement runs: the conditions it runs under; for each local, the stores that may reach it; and the roots
/// changed since each load in scope was evaluated.
#[derive(Clone, Debug, Default)]
struct State {
    path: Path,
    reach: HashMap<Handle<LocalVariable>, Vec<Reach>>,
    evals: Evals,
}

/// `entry` added to `entries`, or its changed roots to the entry for the same store.
fn reach_into(entries: &mut Vec<Reach>, entry: Reach) {
    match entries.iter_mut().find(|r| r.store == entry.store) {
        Some(r) => r.changed.extend(entry.changed),
        None => entries.push(entry),
    }
}

/// The state where control from each of `states` meets: the conditions, which are the same on each, every store
/// reaching on any, and every evaluation in scope on all, with the roots any of them changed; `None` where none
/// reaches.
fn merge(states: Vec<State>) -> Option<State> {
    let mut states = states.into_iter();
    let mut out = states.next()?;
    for s in states {
        out.evals.retain(|h, _| s.evals.contains_key(h));
        for (h, changed) in s.evals {
            if let Some(mine) = out.evals.get_mut(&h) {
                mine.extend(changed);
            }
        }
        for (l, entries) in s.reach {
            let mine = out.reach.entry(l).or_default();
            for e in entries {
                reach_into(mine, e);
            }
        }
    }
    Some(out)
}

/// A statement's identity, while the function is borrowed.
fn id(s: &Statement) -> usize {
    std::ptr::from_ref(s) as usize
}

/// Whether `s` leaves the switch case it ends, so control does not reach the statement after the switch: a `break`
/// ends the case and reaches it.
fn leaves_case(s: Option<&Statement>) -> bool {
    matches!(
        s,
        Some(Statement::Return { .. } | Statement::Kill | Statement::Continue)
    )
}

/// One function's control flow as the guard rule reads it ([`Guard::check`]): each statement, and each store into a
/// local, with the conditions it runs under and the roots changed since each evaluation in scope; and each load of a
/// local with the stores that may reach it, each with the roots changed since it ran.
struct Flow<'a> {
    module: &'a Module,
    function: &'a Function,
    fi: &'a FunctionInfo,
    state: State,
    visits: Vec<(&'a Statement, Span, Path, Evals)>,
    stores: HashMap<usize, (Path, Handle<Expression>, Evals)>,
    loads: HashMap<Handle<Expression>, Vec<Reach>>,
}

impl<'a> Flow<'a> {
    fn new(module: &'a Module, function: &'a Function, fi: &'a FunctionInfo) -> Flow<'a> {
        let reach = function
            .local_variables
            .iter()
            .map(|(l, _)| {
                let initial = Reach {
                    store: None,
                    changed: Roots::new(),
                };
                (l, vec![initial])
            })
            .collect();
        let mut flow = Flow {
            module,
            function,
            fi,
            state: State {
                path: Vec::new(),
                reach,
                evals: Evals::new(),
            },
            visits: Vec::new(),
            stores: HashMap::new(),
            loads: HashMap::new(),
        };
        flow.block(&function.body);
        flow
    }

    /// The variable or argument `pointer` points into ([`Flow::canonical`]).
    fn root(&self, mut pointer: Handle<Expression>) -> Handle<Expression> {
        while let Expression::AccessIndex { base, .. } | Expression::Access { base, .. } =
            self.function.expressions[pointer]
        {
            pointer = base;
        }
        self.canonical(pointer)
    }

    /// `e`, or, where it names a global, the first expression naming it: naga makes one expression per local variable
    /// and per argument, but one per use of a global.
    fn canonical(&self, e: Handle<Expression>) -> Handle<Expression> {
        let Expression::GlobalVariable(g) = self.function.expressions[e] else {
            return e;
        };
        self.function
            .expressions
            .iter()
            .find(|(_, x)| matches!(x, Expression::GlobalVariable(h) if *h == g))
            .map_or(e, |(h, _)| h)
    }

    /// The roots statement `s` itself changes, not the blocks it holds: the expressions it evaluates, an `Emit`'s, a
    /// new value on each run; a store's variable; a call's result, its pointer arguments' variables and every global,
    /// which the callee may store into. Control flow changes nothing itself; any other statement (an atomic, an image
    /// store, a subgroup operation, which no occupant needs) is taken to change every root.
    fn effects(&self, s: &Statement) -> Roots {
        let mut out = Roots::new();
        match *s {
            Statement::Emit(ref range) => out.extend(range.clone()),
            Statement::Store { pointer, .. } => {
                out.insert(self.root(pointer));
            }
            Statement::Call {
                ref arguments,
                result,
                ..
            } => {
                out.extend(result);
                for &a in arguments {
                    if matches!(
                        self.fi[a].ty.inner_with(&self.module.types),
                        TypeInner::Pointer { .. } | TypeInner::ValuePointer { .. }
                    ) {
                        out.insert(self.root(a));
                    }
                }
                out.extend(
                    self.function
                        .expressions
                        .iter()
                        .filter(|(_, e)| matches!(e, Expression::GlobalVariable(_)))
                        .map(|(h, _)| h),
                );
            }
            Statement::Block(_)
            | Statement::If { .. }
            | Statement::Switch { .. }
            | Statement::Loop { .. }
            | Statement::Break
            | Statement::Continue
            | Statement::Return { .. }
            | Statement::Kill
            | Statement::ControlBarrier(_)
            | Statement::MemoryBarrier(_) => {}
            _ => out.extend(self.function.expressions.iter().map(|(h, _)| h)),
        }
        out
    }

    /// The roots `block` changes anywhere.
    fn changed(&self, block: &'a Block) -> Roots {
        let mut out = Roots::new();
        walk(block, &mut Vec::new(), &mut |s, _, _| {
            out.extend(self.effects(s))
        });
        out
    }

    /// Every store into a local in `block`, anywhere: the local and the store.
    fn stores_in(&self, block: &'a Block, out: &mut Vec<(Handle<LocalVariable>, usize)>) {
        walk(block, &mut Vec::new(), &mut |s, _, _| {
            if let Statement::Store { pointer, .. } = *s {
                if let Expression::LocalVariable(l) = self.function.expressions[pointer] {
                    out.push((l, id(s)));
                }
            }
        });
    }

    /// `roots` changed here: every evaluation in scope and every store reaching here was before the change.
    fn change(&mut self, roots: &Roots) {
        for changed in self.state.evals.values_mut() {
            changed.extend(roots.iter().copied());
        }
        for entries in self.state.reach.values_mut() {
            for r in entries {
                r.changed.extend(roots.iter().copied());
            }
        }
    }

    /// Each statement of `block`, recursively, in the order it runs: the conditions of the `if`s whose branch holds
    /// it, and, after an `if` one branch of which leaves the block, the other branch's.
    fn block(&mut self, block: &'a Block) {
        let depth = self.state.path.len();
        for (s, span) in block.span_iter() {
            if !matches!(s, Statement::Loop { .. }) {
                self.visit(s, *span);
            }
            match *s {
                Statement::Emit(ref range) => {
                    for h in range.clone() {
                        // Evaluated again: what a sample keyed by it held is gone, and its own evaluation is new.
                        self.change(&Roots::from([h]));
                        let Expression::Load { pointer } = self.function.expressions[h] else {
                            continue;
                        };
                        if let Expression::LocalVariable(l) = self.function.expressions[pointer] {
                            let entries = self.state.reach.get(&l).cloned().unwrap_or_default();
                            self.loads.insert(h, entries);
                        }
                        self.state.evals.insert(h, Roots::new());
                    }
                }
                Statement::Store { value, .. } => {
                    let at = (self.state.path.clone(), value, self.state.evals.clone());
                    self.stores.insert(id(s), at);
                }
                _ => {}
            }
            if !matches!(s, Statement::Emit(_)) {
                let effects = self.effects(s);
                self.change(&effects);
            }
            match *s {
                Statement::Store { pointer, .. } => {
                    if let Expression::LocalVariable(l) = self.function.expressions[pointer] {
                        let entry = Reach {
                            store: Some(id(s)),
                            changed: Roots::new(),
                        };
                        self.state.reach.insert(l, vec![entry]);
                    }
                }
                Statement::Block(ref b) => self.block(b),
                Statement::If {
                    condition,
                    ref accept,
                    ref reject,
                } => self.branch(condition, accept, reject),
                Statement::Switch { ref cases, .. } => self.switch(cases),
                Statement::Loop {
                    ref body,
                    ref continuing,
                    ..
                } => self.looped(s, *span, body, continuing),
                _ => {}
            }
        }
        self.state.path.truncate(depth);
    }

    /// `s`, at `span`, runs here.
    fn visit(&mut self, s: &'a Statement, span: Span) {
        let at = (s, span, self.state.path.clone(), self.state.evals.clone());
        self.visits.push(at);
    }

    /// An `if`: each branch under its condition; after it, the state where the branches that do not leave meet, and,
    /// where one leaves, the other's condition.
    fn branch(&mut self, condition: Handle<Expression>, accept: &'a Block, reject: &'a Block) {
        let before = self.state.clone();
        let mut ends = Vec::new();
        for (branch, value) in [(accept, true), (reject, false)] {
            self.state = before.clone();
            self.state.path.push((condition, value));
            self.block(branch);
            self.state.path.pop();
            if !leaves(branch) {
                ends.push(self.state.clone());
            }
        }
        self.state = merge(ends).unwrap_or(before);
        if leaves(accept) {
            self.state.path.push((condition, false));
        } else if leaves(reject) {
            self.state.path.push((condition, true));
        }
    }

    /// A `switch`: each case from the state before it, and the one before that where it falls through; after it, the
    /// state where the cases that reach it meet.
    fn switch(&mut self, cases: &'a [naga::SwitchCase]) {
        let before = self.state.clone();
        let mut ends = Vec::new();
        let mut fall: Option<State> = None;
        for case in cases {
            let start = match fall.take() {
                Some(f) => merge(vec![before.clone(), f]),
                None => None,
            };
            self.state = start.unwrap_or_else(|| before.clone());
            self.block(&case.body);
            if case.fall_through {
                fall = Some(self.state.clone());
            } else if !leaves_case(case.body.last()) {
                ends.push(self.state.clone());
            }
        }
        ends.extend(fall);
        self.state = merge(ends).unwrap_or(before);
    }

    /// A loop: its head is reached again after any of its statements ran, so there every root the loop changes has
    /// changed since each evaluation and each store before it, and every store into a local in it may reach, with all
    /// those roots changed. The body, the continuing block and the code after the loop each start from the
    /// head; the loop's `break if` is visited there.
    fn looped(&mut self, s: &'a Statement, span: Span, body: &'a Block, continuing: &'a Block) {
        let mut changed = self.changed(body);
        changed.extend(self.changed(continuing));
        self.change(&changed);
        let mut stores = Vec::new();
        self.stores_in(body, &mut stores);
        self.stores_in(continuing, &mut stores);
        for (l, store) in stores {
            let entry = Reach {
                store: Some(store),
                changed: changed.clone(),
            };
            reach_into(self.state.reach.entry(l).or_default(), entry);
        }
        self.visit(s, span);
        let head = self.state.clone();
        self.block(body);
        self.state = head.clone();
        self.block(continuing);
        self.state = head;
    }
}

/// The guard rule over one function.
struct Guard<'a> {
    module: &'a Module,
    function: &'a Function,
    fi: &'a FunctionInfo,
    flow: Flow<'a>,
    /// The arguments that are the stain's context ([`contexts`]), whose `sample`'s reads the rule passes over: none
    /// but in a debug view or a source.
    own: HashSet<u32>,
    /// The loads whose facts are being found, so that a cycle of stores ends.
    busy: RefCell<Vec<(Handle<Expression>, bool)>>,
}

impl<'a> Guard<'a> {
    fn new(
        module: &'a Module,
        function: &'a Function,
        fi: &'a FunctionInfo,
        own: HashSet<u32>,
    ) -> Guard<'a> {
        Guard {
            module,
            function,
            fi,
            flow: Flow::new(module, function, fi),
            own,
            busy: RefCell::new(Vec::new()),
        }
    }

    /// `e`'s identity as a sample: the same argument, variable or call result and the same member and index path give
    /// the same key, so `ctx.sample.ftle` and `ctx.sample.ftle_valid` read one sample, and so do two loads of one
    /// pointer. naga makes one expression per argument and per local variable, so its handle keys either, and one per
    /// use of a global, so the first keys it ([`Flow::canonical`]). The key's roots are those it is read from: a store
    /// into one, after a fact about the key was taken, ends the fact.
    fn key(&self, e: Handle<Expression>) -> Key {
        match self.function.expressions[e] {
            Expression::AccessIndex { base, index } => {
                let base = self.key(base);
                Key {
                    text: format!("{}.{index}", base.text),
                    roots: base.roots,
                }
            }
            Expression::Access { base, index } => {
                let base = self.key(base);
                let index = self.key(index);
                let mut roots = base.roots;
                roots.extend(index.roots);
                Key {
                    text: format!("{}[{}]", base.text, index.text),
                    roots,
                }
            }
            Expression::Load { pointer } => {
                let pointer = self.key(pointer);
                Key {
                    text: format!("*{}", pointer.text),
                    roots: pointer.roots,
                }
            }
            _ => {
                let e = self.flow.canonical(e);
                Key {
                    text: format!("e{}", e.index()),
                    roots: Roots::from([e]),
                }
            }
        }
    }

    /// The read `e` is, if it reads `ftle` or `ensemble_spread` of a `SimState` other than the sample's own
    /// (`Guard::own`).
    fn read(&self, e: Handle<Expression>) -> Option<Read> {
        let Expression::AccessIndex { base, .. } = self.function.expressions[e] else {
            return None;
        };
        if self.is_own(base) {
            return None;
        }
        match member(self.module, self.function, self.fi, e)? {
            (SIMSTATE, "ftle") => Some(Read::Ftle(self.key(base))),
            (SIMSTATE, "ensemble_spread") => Some(Read::Ensemble),
            _ => None,
        }
    }

    /// Whether `e` is `ctx.sample`, `ctx` an argument that is the stain's context (`Guard::own`).
    fn is_own(&self, e: Handle<Expression>) -> bool {
        let Expression::AccessIndex { base, .. } = self.function.expressions[e] else {
            return false;
        };
        let Expression::FunctionArgument(i) = self.function.expressions[base] else {
            return false;
        };
        self.own.contains(&i)
            && member(self.module, self.function, self.fi, e) == Some((CTX, SAMPLE))
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

    /// The facts `e`, a boolean, implies where it is `value`, at a point where `evals` gives the roots changed since
    /// each evaluation in scope.
    fn facts(&self, e: Handle<Expression>, value: bool, evals: &Evals) -> Facts {
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
            } => self.facts(expr, !value, evals),
            Expression::Load { pointer } => match self.function.expressions[pointer] {
                Expression::LocalVariable(l) => self.load(e, l, value, evals),
                _ if value => self.valid(pointer, e, evals),
                _ => none(),
            },
            Expression::AccessIndex { .. } if value => self.valid(e, e, evals),
            _ => none(),
        }
    }

    /// `{FtleValid}` where `access` reads `ftle_valid` of a `SimState`, keyed by its sample, unless a root of the sample
    /// changed since a load the read `read` goes through was evaluated (`evals`).
    fn valid(&self, access: Handle<Expression>, read: Handle<Expression>, evals: &Evals) -> Facts {
        let Expression::AccessIndex { base, .. } = self.function.expressions[access] else {
            return none();
        };
        let facts = match member(self.module, self.function, self.fi, access) {
            Some((SIMSTATE, "ftle_valid")) => {
                Some(BTreeSet::from([Fact::FtleValid(self.key(base))]))
            }
            _ => none(),
        };
        let mut changed = Roots::new();
        for load in self.loads_in(read) {
            changed.extend(evals.get(&load).into_iter().flatten().copied());
        }
        without(facts, &changed)
    }

    /// The loads `e` goes through: itself, if it is one, and those of its accesses' bases and indices.
    fn loads_in(&self, e: Handle<Expression>) -> Vec<Handle<Expression>> {
        match self.function.expressions[e] {
            Expression::AccessIndex { base, .. } => self.loads_in(base),
            Expression::Access { base, index } => {
                let mut out = self.loads_in(base);
                out.extend(self.loads_in(index));
                out
            }
            Expression::Load { pointer } => {
                let mut out = self.loads_in(pointer);
                out.push(e);
                out
            }
            _ => Vec::new(),
        }
    }

    /// The facts `e`, a load of local `l`, implies where it is `value`: those common to every store that may reach it,
    /// each with the conditions it is stored under, or to the local's initial value (zero, `false`, with no
    /// initialiser) where no store need have run; each without the validity of a sample whose roots changed since,
    /// before the load or after it (`evals`).
    fn load(
        &self,
        e: Handle<Expression>,
        l: Handle<LocalVariable>,
        value: bool,
        evals: &Evals,
    ) -> Facts {
        if self.busy.borrow().contains(&(e, value)) {
            return none();
        }
        self.busy.borrow_mut().push((e, value));
        let mut out = None;
        for r in self.flow.loads.get(&e).into_iter().flatten() {
            let facts = match r.store {
                Some(store) => {
                    let (path, stored, at) = &self.flow.stores[&store];
                    union(self.path(path, at), self.facts(*stored, value, at))
                }
                None => match self.function.local_variables[l].init {
                    Some(init) => self.facts(init, value, evals),
                    None if value => None,
                    None => none(),
                },
            };
            out = intersection(out, without(facts, &r.changed));
        }
        self.busy.borrow_mut().pop();
        match evals.get(&e) {
            Some(changed) => without(out, changed),
            None => out,
        }
    }

    /// The facts the conditions `path` imply, at a point where `evals` gives the roots changed since each evaluation.
    fn path(&self, path: &[(Handle<Expression>, bool)], evals: &Evals) -> Facts {
        path.iter()
            .fold(none(), |acc, &(c, v)| union(acc, self.facts(c, v, evals)))
    }

    /// Every unguarded use of a guarded read in the function.
    fn check(&self, source: &str) -> Vec<Finding> {
        let fname = self.function.name.as_deref().unwrap_or("(unnamed)");
        let mut found = Vec::new();
        for (s, span, path, evals) in &self.flow.visits {
            let reads: BTreeSet<Read> = consumed(s)
                .into_iter()
                .flat_map(|c| self.reads(c))
                .collect();
            if reads.is_empty() {
                continue;
            }
            let facts = self.path(path, evals);
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
                found.push(at(
                    Rule::Guard,
                    format!("`{fname}` uses {what}"),
                    consumed_at(self.function, s, *span),
                    source,
                ));
            }
        }
        found
    }
}
