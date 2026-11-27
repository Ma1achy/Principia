//! What can change the picture, proved over naga's IR (node SDK gap 26).
//!
//! The proof is a backward "demand" analysis: starting from what the picture is made of, the returned colour of
//! `shade_at` and `shade_sample` for a stain, or a node's slot function for a shader on its own, it finds which parts of
//! which values can reach it, field by field and lane by lane ([`Demand`]). A value reaches the picture by data (an
//! operand of something that does), by memory (a store into a place a later load reads, through a pointer too, a
//! callee's pointer argument included), by control (the condition of a branch, a switch or a loop exit around a store,
//! a return, a call with effects or a loop) or by an effect (`discard`, an atomic, a store to a buffer).
//!
//! It is one-sided: "cannot change the picture" is proved, "may change it" is not a promise. A value judged dead
//! reaches nothing whatever it holds, NaN and infinity included, because no algebra is used: `0 * x` is not 0 when `x`
//! is NaN, `x - x` counts as reading `x`, and a term is only ever dropped for being unreachable. Anything the walk does
//! not model (a texture, a barrier, a subgroup operation, a pointer it cannot follow) makes the whole answer "may".
//!
//! Three questions are answered from it:
//! 1. [`Proof::new`] with nothing held: what can reach the picture at all. The assembler drops what cannot
//!    ([`super::Dropped`]), and every shipped control must be able to ([`Shader::never`]).
//! 2. The same with each `choice` and `toggle` held at its value ([`Shader::now`]), so a branch on one is followed
//!    only where it goes: a control that cannot matter now, and the setting that would make it matter
//!    ([`Shader::standing`]).
//! 3. Over a stain, which inputs of the frame can change its picture ([`Changes`]).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use naga::valid::{FunctionInfo, ModuleInfo};
use naga::{
    AddressSpace, ArraySize, BinaryOperator, Block, Expression, Function, GlobalVariable, Handle,
    Literal, LocalVariable, Module, Statement, SwitchValue, TypeInner, UnaryOperator,
};

use super::{
    code_text, compile, head, key_text, prefixed, AssembleError, Control, Declaration, Kind,
    Uniform, UniformType,
};
use ledger::gen::read::{self, Tier};

// ── Demand ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// Which parts of a value can reach the picture: none, all, or some of its members, lanes or elements by index.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Demand {
    #[default]
    Nothing,
    Whole,
    Parts(BTreeMap<u32, Demand>),
}

impl Demand {
    pub fn is_nothing(&self) -> bool {
        matches!(self, Demand::Nothing)
    }

    /// The demand on part `i`.
    pub fn part(&self, i: u32) -> Demand {
        match self {
            Demand::Parts(m) => m.get(&i).cloned().unwrap_or_default(),
            d => d.clone(),
        }
    }

    /// The demand on a part whose index is not known: any part's.
    fn any(&self) -> Demand {
        match self {
            Demand::Parts(m) => m.values().fold(Demand::Nothing, |mut a, d| {
                a.join(d);
                a
            }),
            d => d.clone(),
        }
    }

    /// `d` on part `i` alone.
    fn at(i: u32, d: Demand) -> Demand {
        if d.is_nothing() {
            Demand::Nothing
        } else {
            Demand::Parts(BTreeMap::from([(i, d)]))
        }
    }

    /// Adds `other`; whether that changed anything.
    fn join(&mut self, other: &Demand) -> bool {
        match (&mut *self, other) {
            (_, Demand::Nothing) | (Demand::Whole, _) => false,
            (Demand::Nothing, o) => {
                *self = o.clone();
                true
            }
            (_, Demand::Whole) => {
                *self = Demand::Whole;
                true
            }
            (Demand::Parts(a), Demand::Parts(b)) => {
                let mut changed = false;
                for (k, v) in b {
                    changed |= a.entry(*k).or_default().join(v);
                }
                changed
            }
        }
    }
}

/// One step from a value into a part of it: a known index, or one not known until the shader runs, with the number of
/// parts where the type gives it.
#[derive(Clone, Copy, Debug)]
enum Step {
    At(u32),
    Any(Option<u32>),
}

/// The demand on the part at `path`.
fn restrict(d: &Demand, path: &[Step]) -> Demand {
    path.iter().fold(d.clone(), |d, s| match s {
        Step::At(i) => d.part(*i),
        Step::Any(_) => d.any(),
    })
}

/// `d` on the part at `path`, as a demand on the whole.
fn wrap(path: &[Step], d: Demand) -> Demand {
    path.iter().rev().fold(d, |d, s| match (s, d.is_nothing()) {
        (_, true) => Demand::Nothing,
        (Step::At(i), _) => Demand::at(*i, d),
        (Step::Any(Some(n)), _) if *n <= 64 => {
            Demand::Parts((0..*n).map(|i| (i, d.clone())).collect())
        }
        (Step::Any(_), _) => Demand::Whole,
    })
}

// ── Held values ───────────────────────────────────────────────────────────────────────────────────────────────────

/// A scalar the proof can evaluate a condition from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lit {
    Bool(bool),
    U32(u32),
    I32(i32),
    F32(f32),
}

impl Lit {
    fn of(l: &Literal) -> Option<Lit> {
        Some(match *l {
            Literal::Bool(b) => Lit::Bool(b),
            Literal::U32(v) => Lit::U32(v),
            Literal::I32(v) => Lit::I32(v),
            Literal::F32(v) => Lit::F32(v),
            Literal::AbstractInt(v) => Lit::I32(i32::try_from(v).ok()?),
            _ => return None,
        })
    }

    /// A uniform's value as the block holds it, `None` for a vector.
    pub fn uniform(ty: UniformType, v: f64) -> Option<Lit> {
        Some(match ty {
            UniformType::U32 => Lit::U32(v as u32),
            UniformType::I32 => Lit::I32(v as i32),
            UniformType::F32 => Lit::F32(v as f32),
            _ => return None,
        })
    }
}

/// Uniform members held at a value: (the block's global, the member's index) → its value.
pub type Held = HashMap<(Handle<GlobalVariable>, u32), Lit>;

// ── The analysis ──────────────────────────────────────────────────────────────────────────────────────────────────

/// Where a pointer points: a variable, a pointer argument or a global, then a path into it, with the index
/// expressions of the steps whose index is not known.
struct Place {
    root: Root,
    path: Vec<Step>,
    indices: Vec<Handle<Expression>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Root {
    Local(Handle<LocalVariable>),
    Arg(u32),
    Global(Handle<GlobalVariable>),
}

#[derive(Default)]
struct Fun {
    exprs: Vec<Demand>,
    args: Vec<Demand>,
    ret: Demand,
    /// The demand on each local variable's and pointer argument's memory.
    memory: HashMap<Root, Demand>,
    reached: bool,
}

/// The proof over one module from its roots.
pub struct Proof<'m> {
    module: &'m Module,
    info: &'m ModuleInfo,
    funs: Vec<Fun>,
    globals: Vec<Demand>,
    effectful: Vec<bool>,
    held: &'m Held,
    /// False where the walk met something it does not model: then everything may reach the picture.
    sure: bool,
    /// Whether the walk in progress added any demand.
    changed: bool,
}

impl<'m> Proof<'m> {
    /// The proof over `module` whose picture is the result of each function named in `roots`, every part of it, and
    /// every effect of what they call; with each member in `held` at its value.
    pub fn new(
        module: &'m Module,
        info: &'m ModuleInfo,
        roots: &[&str],
        held: &'m Held,
    ) -> Proof<'m> {
        let mut p = Proof {
            module,
            info,
            funs: module
                .functions
                .iter()
                .map(|(_, f)| Fun {
                    exprs: vec![Demand::Nothing; f.expressions.len()],
                    args: vec![Demand::Nothing; f.arguments.len()],
                    ..Fun::default()
                })
                .collect(),
            globals: vec![Demand::Nothing; module.global_variables.len()],
            effectful: Vec::new(),
            held,
            sure: true,
            changed: false,
        };
        p.effectful = p.effects();
        for (h, f) in module.functions.iter() {
            if f.name.as_deref().is_some_and(|n| roots.contains(&n)) {
                let fun = &mut p.funs[h.index()];
                fun.reached = true;
                fun.ret = Demand::Whole;
            }
        }
        loop {
            p.changed = false;
            for (h, f) in module.functions.iter() {
                if !p.funs[h.index()].reached {
                    continue;
                }
                p.block(h, f, &f.body, &mut Vec::new());
                p.changed |= p.expressions(h, f);
            }
            if !p.changed || !p.sure {
                break;
            }
        }
        p
    }

    /// Whether the proof followed everything; where not, every query answers "may".
    pub fn sure(&self) -> bool {
        self.sure
    }

    fn function(&self, name: &str) -> Option<Handle<Function>> {
        self.module
            .functions
            .iter()
            .find(|(_, f)| f.name.as_deref() == Some(name))
            .map(|(h, _)| h)
    }

    /// The demand on the global `name`'s memory.
    pub fn global(&self, name: &str) -> Demand {
        if !self.sure {
            return Demand::Whole;
        }
        self.module
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some(name))
            .map_or(Demand::Nothing, |(h, _)| self.globals[h.index()].clone())
    }

    /// The demand on the result of the function `name`, nothing where it is never called.
    pub fn result(&self, name: &str) -> Demand {
        match (self.sure, self.function(name)) {
            (false, _) => Demand::Whole,
            (true, Some(h)) => self.funs[h.index()].ret.clone(),
            (true, None) => Demand::Nothing,
        }
    }

    /// The demand on argument `k` of the function `name`.
    pub fn argument(&self, name: &str, k: usize) -> Demand {
        match (self.sure, self.function(name)) {
            (false, _) => Demand::Whole,
            (true, Some(h)) => self.funs[h.index()]
                .args
                .get(k)
                .cloned()
                .unwrap_or_default(),
            (true, None) => Demand::Nothing,
        }
    }

    /// Whether the function `name` may do something besides returning: an effect, or a loop, which may not end.
    pub fn has_effects(&self, name: &str) -> bool {
        !self.sure
            || self
                .function(name)
                .is_none_or(|h| self.effectful[h.index()])
    }

    // ── Which functions have effects ──

    /// Each function's: whether it, or anything it calls, discards, writes a buffer, a global or through a pointer
    /// argument, runs an atomic, loops, or calls `note_non_finite`, the debug view's note, counted in both views so an
    /// answer does not change with the view.
    // The plan is unclear here; chose to count `note_non_finite` as an effect in the presentation view too, where it does
    // nothing, because an answer that changed with View › Debug view would dim a control in one view and not the other.
    fn effects(&mut self) -> Vec<bool> {
        let module = self.module;
        let mut direct = Vec::new();
        let mut calls = Vec::new();
        for (_, f) in module.functions.iter() {
            let mut out = Vec::new();
            direct.push(self.direct_effects(f, &f.body, &mut out));
            calls.push(out);
        }
        loop {
            let mut changed = false;
            for (k, cs) in calls.iter().enumerate() {
                if !direct[k] && cs.iter().any(|c: &Handle<Function>| direct[c.index()]) {
                    direct[k] = true;
                    changed = true;
                }
            }
            if !changed {
                return direct;
            }
        }
    }

    fn direct_effects(
        &mut self,
        f: &Function,
        block: &Block,
        calls: &mut Vec<Handle<Function>>,
    ) -> bool {
        let mut any = false;
        for s in block.iter() {
            any |= match s {
                Statement::Block(b) => self.direct_effects(f, b, calls),
                Statement::If { accept, reject, .. } => {
                    let a = self.direct_effects(f, accept, calls);
                    self.direct_effects(f, reject, calls) || a
                }
                Statement::Switch { cases, .. } => {
                    // Every case is walked, not just to the first effect: the walk also gathers the calls.
                    let mut any = false;
                    for c in cases {
                        any |= self.direct_effects(f, &c.body, calls);
                    }
                    any
                }
                Statement::Loop {
                    body, continuing, ..
                } => {
                    self.direct_effects(f, body, calls);
                    self.direct_effects(f, continuing, calls);
                    true
                }
                Statement::Call { function, .. } => {
                    calls.push(*function);
                    self.module.functions[*function].name.as_deref() == Some("note_non_finite")
                }
                Statement::Store { pointer, .. } => match place(self.module, f, None, *pointer) {
                    Some(p) => !matches!(p.root, Root::Local(_)),
                    None => true,
                },
                Statement::Emit(_)
                | Statement::Break
                | Statement::Continue
                | Statement::Return { .. } => false,
                _ => true,
            };
        }
        any
    }

    // ── Statements ──

    /// Demands every enclosing condition, all of it, unless a control forgets branches.
    fn control(&mut self, f: Handle<Function>, ctrl: &[Handle<Expression>]) -> bool {
        if forgets(Forget::Branches) {
            return false;
        }
        let mut changed = false;
        for &c in ctrl {
            changed |= self.funs[f.index()].exprs[c.index()].join(&Demand::Whole);
        }
        changed
    }

    fn whole(&mut self, f: Handle<Function>, e: Handle<Expression>) -> bool {
        self.funs[f.index()].exprs[e.index()].join(&Demand::Whole)
    }

    fn memory(&self, f: Handle<Function>, r: Root) -> Demand {
        match r {
            Root::Global(g) => self.globals[g.index()].clone(),
            r => self.funs[f.index()]
                .memory
                .get(&r)
                .cloned()
                .unwrap_or_default(),
        }
    }

    fn add_memory(&mut self, f: Handle<Function>, r: Root, d: &Demand) -> bool {
        match r {
            Root::Global(g) => self.globals[g.index()].join(d),
            r => self.funs[f.index()].memory.entry(r).or_default().join(d),
        }
    }

    /// Walks `block`, adding what its statements demand; whether control can run past its end.
    fn block(
        &mut self,
        h: Handle<Function>,
        f: &'m Function,
        block: &'m Block,
        ctrl: &mut Vec<Handle<Expression>>,
    ) -> bool {
        let module = self.module;
        for s in block.iter() {
            let mut changed = false;
            // Whether control can reach the next statement; code after a statement that cannot is never run.
            let mut falls = true;
            match s {
                Statement::Emit(_) => {}
                Statement::Block(b) => falls = self.block(h, f, b, ctrl),
                Statement::If {
                    condition,
                    accept,
                    reject,
                } => match self.eval(f, *condition) {
                    Some(Lit::Bool(true)) => falls = self.block(h, f, accept, ctrl),
                    Some(Lit::Bool(false)) => falls = self.block(h, f, reject, ctrl),
                    _ => {
                        ctrl.push(*condition);
                        let a = self.block(h, f, accept, ctrl);
                        let r = self.block(h, f, reject, ctrl);
                        falls = a || r;
                        ctrl.pop();
                    }
                },
                Statement::Switch { selector, cases } => {
                    let value = self.eval(f, *selector);
                    let taken = value.and_then(|v| {
                        let hit = |c: &naga::SwitchCase| match (c.value, v) {
                            (SwitchValue::I32(a), Lit::I32(b)) => a == b,
                            (SwitchValue::U32(a), Lit::U32(b)) => a == b,
                            _ => false,
                        };
                        cases
                            .iter()
                            .position(hit)
                            .or_else(|| cases.iter().position(|c| c.value == SwitchValue::Default))
                    });
                    match taken {
                        // A `break` in a case leaves the switch, so a switch is taken to fall through.
                        Some(mut k) => loop {
                            self.block(h, f, &cases[k].body, ctrl);
                            if !cases[k].fall_through || k + 1 == cases.len() {
                                break;
                            }
                            k += 1;
                        },
                        None => {
                            ctrl.push(*selector);
                            for c in cases {
                                self.block(h, f, &c.body, ctrl);
                            }
                            ctrl.pop();
                        }
                    }
                }
                Statement::Loop {
                    body,
                    continuing,
                    break_if,
                } => {
                    // Whether a loop runs, and when it stops, can change the picture: one that never ends draws none.
                    changed |= self.control(h, ctrl);
                    if let Some(b) = break_if {
                        if !forgets(Forget::Branches) {
                            changed |= self.whole(h, *b);
                        }
                    }
                    self.block(h, f, body, ctrl);
                    self.block(h, f, continuing, ctrl);
                }
                Statement::Break | Statement::Continue | Statement::Kill => {
                    changed |= self.control(h, ctrl);
                    falls = false;
                }
                Statement::Return { value } => {
                    let ret = self.funs[h.index()].ret.clone();
                    if let Some(v) = value {
                        changed |= self.funs[h.index()].exprs[v.index()].join(&ret);
                    }
                    if !ret.is_nothing() || self.effectful[h.index()] {
                        changed |= self.control(h, ctrl);
                    }
                    falls = false;
                }
                Statement::Store { pointer, value } => {
                    let Some(p) = place(module, f, Some(&self.info[h]), *pointer) else {
                        self.sure = false;
                        return true;
                    };
                    let forgotten =
                        forgets(Forget::PointerWrites) && matches!(p.root, Root::Arg(_));
                    let d = match p.root {
                        _ if forgotten => Demand::Nothing,
                        Root::Global(g) if writes_out(&module.global_variables[g]) => Demand::Whole,
                        r => restrict(&self.memory(h, r), &p.path),
                    };
                    if !d.is_nothing() {
                        changed |= self.funs[h.index()].exprs[value.index()].join(&d);
                        for i in p.indices {
                            changed |= self.whole(h, i);
                        }
                        changed |= self.control(h, ctrl);
                    }
                }
                Statement::Atomic {
                    pointer,
                    fun,
                    value,
                    ..
                } => {
                    let Some(p) = place(module, f, Some(&self.info[h]), *pointer) else {
                        self.sure = false;
                        return true;
                    };
                    for i in p.indices {
                        changed |= self.whole(h, i);
                    }
                    changed |= self.whole(h, *value);
                    if let naga::AtomicFunction::Exchange { compare: Some(c) } = fun {
                        changed |= self.whole(h, *c);
                    }
                    changed |= self.add_memory(h, p.root, &Demand::Whole);
                    changed |= self.control(h, ctrl);
                }
                Statement::Call {
                    function,
                    arguments,
                    result,
                } => changed |= self.call(h, f, *function, arguments, *result, ctrl),
                _ => {
                    self.sure = false;
                    return true;
                }
            }
            self.changed |= changed;
            if !falls {
                return false;
            }
        }
        true
    }

    fn call(
        &mut self,
        h: Handle<Function>,
        f: &'m Function,
        callee: Handle<Function>,
        arguments: &[Handle<Expression>],
        result: Option<Handle<Expression>>,
        ctrl: &[Handle<Expression>],
    ) -> bool {
        let module = self.module;
        let g = &module.functions[callee];
        let mut changed = false;
        if !self.funs[callee.index()].reached {
            self.funs[callee.index()].reached = true;
            changed = true;
        }
        if let Some(r) = result {
            let d = self.funs[h.index()].exprs[r.index()].clone();
            changed |= self.funs[callee.index()].ret.join(&d);
        }
        // A call whose result nothing reads and which has no effect gives its arguments nothing, whatever the
        // function's other calls give theirs.
        let read = result.is_some_and(|r| !self.funs[h.index()].exprs[r.index()].is_nothing());
        if !read && !self.effectful[callee.index()] {
            return changed;
        }
        for (k, &a) in arguments.iter().enumerate() {
            let pointer = matches!(
                module.types[g.arguments[k].ty].inner,
                TypeInner::Pointer { .. }
            );
            if !pointer {
                let d = self.funs[callee.index()].args[k].clone();
                changed |= self.funs[h.index()].exprs[a.index()].join(&d);
                continue;
            }
            let Some(p) = place(module, f, Some(&self.info[h]), a) else {
                self.sure = false;
                return changed;
            };
            // What the caller reads after the call is demanded of what the callee writes through the pointer...
            if !forgets(Forget::PointerWrites) {
                let after = restrict(&self.memory(h, p.root), &p.path);
                changed |= self.funs[callee.index()]
                    .memory
                    .entry(Root::Arg(k as u32))
                    .or_default()
                    .join(&after);
            }
            // ...and what the callee reads through it is demanded of what the caller stored there before.
            let read = self.funs[callee.index()]
                .memory
                .get(&Root::Arg(k as u32))
                .cloned()
                .unwrap_or_default();
            if !read.is_nothing() {
                changed |= self.add_memory(h, p.root, &wrap(&p.path, read));
                for i in p.indices {
                    changed |= self.whole(h, i);
                }
            }
        }
        if self.effectful[callee.index()] {
            changed |= self.control(h, ctrl);
        }
        changed
    }

    // ── Expressions ──

    /// Each expression's demand passed to its operands, last to first, so most operands are seen after their users.
    fn expressions(&mut self, h: Handle<Function>, f: &'m Function) -> bool {
        let module = self.module;
        let info = &self.info[h];
        let mut changed = false;
        for (e, x) in f.expressions.iter().rev() {
            let d = self.funs[h.index()].exprs[e.index()].clone();
            if d.is_nothing() {
                continue;
            }
            let give = |p: &mut Proof, o: Handle<Expression>, d: &Demand| {
                p.funs[h.index()].exprs[o.index()].join(d)
            };
            match *x {
                Expression::Literal(_)
                | Expression::Constant(_)
                | Expression::Override(_)
                | Expression::ZeroValue(_)
                | Expression::CallResult(_)
                | Expression::AtomicResult { .. }
                | Expression::LocalVariable(_)
                | Expression::GlobalVariable(_) => {}
                Expression::Compose { ty, ref components } => match &module.types[ty].inner {
                    TypeInner::Vector { .. } => {
                        let mut lane = 0;
                        for &c in components {
                            let width = match *info[c].ty.inner_with(&module.types) {
                                TypeInner::Vector { size, .. } => size as u32,
                                _ => 1,
                            };
                            let part = if width == 1 {
                                d.part(lane)
                            } else {
                                (0..width).fold(Demand::Nothing, |mut a, j| {
                                    a.join(&Demand::at(j, d.part(lane + j)));
                                    a
                                })
                            };
                            changed |= give(self, c, &part);
                            lane += width;
                        }
                    }
                    _ => {
                        for (k, &c) in components.iter().enumerate() {
                            changed |= give(self, c, &d.part(k as u32));
                        }
                    }
                },
                Expression::AccessIndex { base, index } => {
                    if !is_pointer(module, info, base) {
                        changed |= give(self, base, &Demand::at(index, d));
                    }
                }
                Expression::Access { base, index } => {
                    if !is_pointer(module, info, base) {
                        let n = length(module, info, base);
                        changed |= give(self, base, &wrap(&[Step::Any(n)], d));
                        changed |= give(self, index, &Demand::Whole);
                    }
                }
                Expression::Splat { value, .. } => changed |= give(self, value, &Demand::Whole),
                Expression::Swizzle {
                    size,
                    vector,
                    pattern,
                } => {
                    let mut on = Demand::Nothing;
                    for (k, c) in pattern.iter().take(size as usize).enumerate() {
                        on.join(&Demand::at(*c as u32, d.part(k as u32)));
                    }
                    changed |= give(self, vector, &on);
                }
                Expression::FunctionArgument(k) => {
                    changed |= self.funs[h.index()].args[k as usize].join(&d);
                }
                Expression::Load { pointer } => {
                    let Some(p) = place(module, f, Some(info), pointer) else {
                        self.sure = false;
                        return changed;
                    };
                    changed |= self.add_memory(h, p.root, &wrap(&p.path, d));
                    for i in p.indices {
                        changed |= give(self, i, &Demand::Whole);
                    }
                }
                Expression::Unary { expr, .. } => changed |= give(self, expr, &d),
                Expression::Binary { op, left, right } => {
                    let zero = |o: Handle<Expression>| matches!(f.expressions[o], Expression::Literal(l) if is_zero(l));
                    let folds = op == BinaryOperator::Multiply && forgets(Forget::ZeroTimes);
                    for (side, other) in [(left, right), (right, left)] {
                        if folds && zero(other) {
                            continue;
                        }
                        let lanes = same_shape(module, info, e, side);
                        changed |= give(self, side, if lanes { &d } else { &Demand::Whole });
                    }
                }
                Expression::Select {
                    condition,
                    accept,
                    reject,
                } => match self.eval(f, condition) {
                    Some(Lit::Bool(true)) => changed |= give(self, accept, &d),
                    Some(Lit::Bool(false)) => changed |= give(self, reject, &d),
                    _ => {
                        let lanes = same_shape(module, info, e, condition);
                        changed |= give(self, condition, if lanes { &d } else { &Demand::Whole });
                        changed |= give(self, accept, &d);
                        changed |= give(self, reject, &d);
                    }
                },
                Expression::Derivative { expr, .. } => changed |= give(self, expr, &d),
                Expression::Relational { argument, .. } => {
                    changed |= give(self, argument, &Demand::Whole)
                }
                Expression::Math {
                    arg,
                    arg1,
                    arg2,
                    arg3,
                    ..
                } => {
                    for a in [Some(arg), arg1, arg2, arg3].into_iter().flatten() {
                        changed |= give(self, a, &Demand::Whole);
                    }
                }
                Expression::As { expr, .. } => {
                    let lanes = same_shape(module, info, e, expr);
                    changed |= give(self, expr, if lanes { &d } else { &Demand::Whole });
                }
                Expression::ArrayLength(_) => {}
                _ => {
                    self.sure = false;
                    return changed;
                }
            }
        }
        changed
    }

    // ── Held values ──

    /// The value of `e` where it follows from held members and literals alone.
    fn eval(&self, f: &Function, e: Handle<Expression>) -> Option<Lit> {
        if self.held.is_empty() {
            return None;
        }
        self.value(&f.expressions, e, Some(f))
    }

    fn value(
        &self,
        arena: &naga::Arena<Expression>,
        e: Handle<Expression>,
        f: Option<&Function>,
    ) -> Option<Lit> {
        use Lit::{Bool, F32, I32, U32};
        Some(match arena[e] {
            Expression::Literal(ref l) => Lit::of(l)?,
            Expression::Constant(c) => {
                let init = self.module.constants[c].init;
                self.value(&self.module.global_expressions, init, None)?
            }
            Expression::Load { pointer } => {
                let Expression::AccessIndex { base, index } = arena[pointer] else {
                    return None;
                };
                let Expression::GlobalVariable(g) = arena[base] else {
                    return None;
                };
                f?;
                *self.held.get(&(g, index))?
            }
            Expression::Unary { op, expr } => match (op, self.value(arena, expr, f)?) {
                (UnaryOperator::LogicalNot, Bool(b)) => Bool(!b),
                (UnaryOperator::BitwiseNot, U32(v)) => U32(!v),
                (UnaryOperator::BitwiseNot, I32(v)) => I32(!v),
                (UnaryOperator::Negate, I32(v)) => I32(v.wrapping_neg()),
                (UnaryOperator::Negate, F32(v)) => F32(-v),
                _ => return None,
            },
            Expression::Binary { op, left, right } => {
                let (a, b) = (self.value(arena, left, f)?, self.value(arena, right, f)?);
                binary(op, a, b)?
            }
            Expression::As {
                expr,
                kind,
                convert: Some(4),
            } => {
                let v = self.value(arena, expr, f)?;
                let n = match v {
                    Bool(b) => f64::from(u8::from(b)),
                    U32(v) => f64::from(v),
                    I32(v) => f64::from(v),
                    F32(v) => f64::from(v),
                };
                match kind {
                    naga::ScalarKind::Uint if !matches!(v, F32(_)) => U32(n as i64 as u32),
                    naga::ScalarKind::Sint if !matches!(v, F32(_)) => I32(n as i64 as i32),
                    naga::ScalarKind::Float => F32(n as f32),
                    naga::ScalarKind::Bool => Bool(n != 0.0),
                    _ => return None,
                }
            }
            Expression::Select {
                condition,
                accept,
                reject,
            } => match self.value(arena, condition, f)? {
                Bool(true) => self.value(arena, accept, f)?,
                Bool(false) => self.value(arena, reject, f)?,
                _ => return None,
            },
            _ => return None,
        })
    }
}

/// `a op b` on scalars of one kind, where the proof needs it: comparisons, logic and integer arithmetic that cannot
/// trap or wrap differently from WGSL.
fn binary(op: BinaryOperator, a: Lit, b: Lit) -> Option<Lit> {
    use BinaryOperator as B;
    use Lit::{Bool, I32, U32};
    let cmp = |o: std::cmp::Ordering| match op {
        B::Equal => Some(Bool(o.is_eq())),
        B::NotEqual => Some(Bool(o.is_ne())),
        B::Less => Some(Bool(o.is_lt())),
        B::LessEqual => Some(Bool(o.is_le())),
        B::Greater => Some(Bool(o.is_gt())),
        B::GreaterEqual => Some(Bool(o.is_ge())),
        _ => None,
    };
    match (a, b) {
        (Bool(x), Bool(y)) => match op {
            B::LogicalAnd | B::And => Some(Bool(x && y)),
            B::LogicalOr | B::InclusiveOr => Some(Bool(x || y)),
            B::Equal => Some(Bool(x == y)),
            B::NotEqual | B::ExclusiveOr => Some(Bool(x != y)),
            _ => None,
        },
        (U32(x), U32(y)) => match op {
            B::And => Some(U32(x & y)),
            B::InclusiveOr => Some(U32(x | y)),
            B::ExclusiveOr => Some(U32(x ^ y)),
            B::Add => Some(U32(x.wrapping_add(y))),
            B::Subtract => Some(U32(x.wrapping_sub(y))),
            _ => cmp(x.cmp(&y)),
        },
        (I32(x), I32(y)) => match op {
            B::And => Some(I32(x & y)),
            B::InclusiveOr => Some(I32(x | y)),
            B::ExclusiveOr => Some(I32(x ^ y)),
            B::Add => Some(I32(x.wrapping_add(y))),
            B::Subtract => Some(I32(x.wrapping_sub(y))),
            _ => cmp(x.cmp(&y)),
        },
        _ => None,
    }
}

fn is_zero(l: Literal) -> bool {
    match l {
        Literal::F32(v) => v == 0.0,
        Literal::F64(v) | Literal::AbstractFloat(v) => v == 0.0,
        Literal::I32(v) => v == 0,
        Literal::U32(v) => v == 0,
        Literal::AbstractInt(v) => v == 0,
        _ => false,
    }
}

/// Whether a store to `g` leaves the shader: a buffer or a workgroup's memory, not a per-invocation global.
fn writes_out(g: &GlobalVariable) -> bool {
    !matches!(g.space, AddressSpace::Private | AddressSpace::Function)
}

fn is_pointer(module: &Module, info: &FunctionInfo, e: Handle<Expression>) -> bool {
    matches!(
        info[e].ty.inner_with(&module.types),
        TypeInner::Pointer { .. } | TypeInner::ValuePointer { .. }
    )
}

/// Whether `a` and `b` have one shape, so a lane of one is the same lane of the other.
fn same_shape(
    module: &Module,
    info: &FunctionInfo,
    a: Handle<Expression>,
    b: Handle<Expression>,
) -> bool {
    let shape = |e: Handle<Expression>| match *info[e].ty.inner_with(&module.types) {
        TypeInner::Scalar(_) => Some(1),
        TypeInner::Vector { size, .. } => Some(size as u32),
        _ => None,
    };
    shape(a).is_some() && shape(a) == shape(b)
}

/// The number of parts of `e`'s value, or of what it points to: an array's fixed length, a vector's lanes, a matrix's
/// columns.
fn length(module: &Module, info: &FunctionInfo, e: Handle<Expression>) -> Option<u32> {
    let inner = match *info[e].ty.inner_with(&module.types) {
        TypeInner::Pointer { base, .. } => &module.types[base].inner,
        TypeInner::ValuePointer { size: Some(s), .. } => return Some(s as u32),
        ref t => t,
    };
    match *inner {
        TypeInner::Array {
            size: ArraySize::Constant(n),
            ..
        } => Some(n.get()),
        TypeInner::Vector { size, .. } => Some(size as u32),
        TypeInner::Matrix { columns, .. } => Some(columns as u32),
        _ => None,
    }
}

/// Where the pointer `e` points, or `None` where it cannot be followed. Without `info` an index's length is not known.
fn place(
    module: &Module,
    f: &Function,
    info: Option<&FunctionInfo>,
    e: Handle<Expression>,
) -> Option<Place> {
    let leaf = |root| {
        Some(Place {
            root,
            path: Vec::new(),
            indices: Vec::new(),
        })
    };
    match f.expressions[e] {
        Expression::LocalVariable(l) => leaf(Root::Local(l)),
        Expression::GlobalVariable(g) => leaf(Root::Global(g)),
        Expression::FunctionArgument(k) => leaf(Root::Arg(k)),
        Expression::AccessIndex { base, index } => {
            let mut p = place(module, f, info, base)?;
            p.path.push(Step::At(index));
            Some(p)
        }
        Expression::Access { base, index } => {
            let mut p = place(module, f, info, base)?;
            p.path
                .push(Step::Any(info.and_then(|i| length(module, i, base))));
            p.indices.push(index);
            Some(p)
        }
        _ => None,
    }
}

// ── Controls ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// A way a test's control breaks the proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forget {
    /// No condition is demanded: branches, switches and loop exits are forgotten.
    Branches,
    /// A write through a pointer argument is forgotten.
    PointerWrites,
    /// `0 * x` is folded to 0, so `x` is not demanded.
    ZeroTimes,
}

#[cfg(not(feature = "controls"))]
fn forgets(_: Forget) -> bool {
    false
}

#[cfg(feature = "controls")]
fn forgets(f: Forget) -> bool {
    probe::FORGET.with(|c| c.get() == Some(f))
}

/// A test's control's reach into the proof, on this thread only.
#[cfg(feature = "controls")]
pub mod probe {
    use std::cell::Cell;

    use super::Forget;

    thread_local! {
        pub(super) static FORGET: Cell<Option<Forget>> = const { Cell::new(None) };
        pub(super) static CONSTANT: Cell<bool> = const { Cell::new(false) };
    }

    /// `f` run with a dropped uniform's reads given its type's zero as a constant, which naga folds.
    pub fn constant_stand_in<T>(f: impl FnOnce() -> T) -> T {
        let before = CONSTANT.with(|c| c.replace(true));
        let out = f();
        CONSTANT.with(|c| c.set(before));
        out
    }

    /// `f` run with the proof broken as `forget` says.
    pub fn forgetting<T>(forget: Forget, f: impl FnOnce() -> T) -> T {
        let before = FORGET.with(|c| c.replace(Some(forget)));
        let out = f();
        FORGET.with(|c| c.set(before));
        out
    }
}

// ── A stain: fields and the frame ─────────────────────────────────────────────────────────────────────────────────

/// The roots of a stain's picture: what a pixel is given, with and without a raster's lanes.
pub const STAIN_ROOTS: [&str; 2] = ["shade_at", "shade_sample"];

/// The read-side fields the stain in `module` reads that can reach its picture, named as [`super::fields_read`] names
/// them: `syntactic`, that set, less each member no demand reaches.
pub fn live_fields(
    module: &Module,
    proof: &Proof,
    syntactic: BTreeSet<String>,
) -> BTreeSet<String> {
    if !proof.sure() {
        return syntactic;
    }
    let index = |ty: &str, name: &str| {
        super::named_type(module, ty).and_then(|t| match &module.types[t].inner {
            TypeInner::Struct { members, .. } => members
                .iter()
                .position(|m| m.name.as_deref() == Some(name))
                .map(|k| k as u32),
            _ => None,
        })
    };
    let sample = proof.result("sample_read");
    let ic = proof.result(super::IC_READ);
    let mut out = BTreeSet::new();
    for f in syntactic {
        let d = match f.strip_prefix("ic.") {
            Some(m) => index(read::IC_STRUCT, m).map_or(Demand::Whole, |k| ic.part(k)),
            None => {
                let (m, c) = f
                    .split_once('.')
                    .map_or((f.as_str(), None), |(m, c)| (m, Some(c)));
                let d = index("SimState", m).map_or(Demand::Whole, |k| sample.part(k));
                match c.and_then(|c| read::WORD_COMPONENTS.iter().position(|w| *w == c)) {
                    Some(k) => d.part(k as u32),
                    None => d,
                }
            }
        };
        // The plan is unclear here; chose to keep the word as whole as the IR reads it, dropping only a read nothing
        // reaches, because narrowing it to components would change the read side of stains that drop nothing.
        if !d.is_nothing() {
            out.insert(f);
        }
    }
    out
}

/// What can change a stain's picture (question 3), over the whole graph: the read-side fields it reads, which change
/// as the playhead moves and the samples with it; and which of the frame's other inputs reach it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    /// The fields, as [`super::field_set`] names them.
    pub fields: BTreeSet<String>,
    /// The pixel's place on the screen (`ctx.frag_xy`, `ctx.screen`): the camera, the zoom and the window.
    pub pixel: bool,
    /// The sample's quad and tile (`ctx.quad`, `ctx.tile`): the zoom and the quads' layout.
    pub quad: bool,
    /// The chart's classification of the sample (`ctx.validity`).
    pub validity: bool,
    /// The run's read parameters, masses and ensemble spread (`ctx.params`, the read side's arguments).
    pub run: bool,
    /// The prelude's uniforms: the canvas background and whether the ensemble is on.
    pub canvas: bool,
}

impl Changes {
    /// Whether the picture is a function of the sample and the run alone, the same wherever the sample is drawn: what
    /// a bake to a texture needs.
    pub fn bakes(&self) -> bool {
        !self.pixel && !self.quad && !self.validity
    }

    /// Whether nothing in the frame changes it: it is one flat picture.
    pub fn constant(&self) -> bool {
        self.fields.is_empty() && self.bakes() && !self.run && !self.canvas
    }
}

/// Whether the proof over `stain` at `tier` as `view` followed everything: where not, nothing is judged dead.
pub fn followed(
    stain: &super::Stain,
    tier: Tier,
    view: super::View,
) -> Result<bool, AssembleError> {
    let arm = (view == super::View::Debug).then_some(super::DEBUG_ARM);
    let every = super::every_field()?;
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let (source, _) = super::source(
        stain,
        tier,
        &every,
        arm,
        false,
        &super::Dropped::default(),
        true,
    )?;
    let (module, info) = compile(&source)?;
    let held = Held::new();
    Ok(Proof::new(&module, &info, &STAIN_ROOTS, &held).sure())
}

/// Question 3 over `stain` at `tier`, a presentation view: what can change its picture.
pub fn stain_changes(stain: &super::Stain, tier: Tier) -> Result<Changes, AssembleError> {
    let every = super::every_field()?;
    let every: Vec<&str> = every.iter().map(String::as_str).collect();
    let (source, _) = super::source(
        stain,
        tier,
        &every,
        None,
        false,
        &super::Dropped::default(),
        false,
    )?;
    let (module, info) = compile(&source)?;
    let syntactic = super::fields_read(&module, &info);
    Ok(changes(&module, &info, syntactic))
}

/// Question 3 over the stain module `module`, whose syntactic field set is `syntactic`.
pub fn changes(module: &Module, info: &ModuleInfo, syntactic: BTreeSet<String>) -> Changes {
    let held = Held::new();
    let proof = Proof::new(module, info, &STAIN_ROOTS, &held);
    let argument = |name: &str| {
        STAIN_ROOTS.iter().any(|root| {
            let Some((_, f)) = module
                .functions
                .iter()
                .find(|(_, f)| f.name.as_deref() == Some(root))
            else {
                return false;
            };
            f.arguments
                .iter()
                .position(|a| a.name.as_deref() == Some(name))
                .is_some_and(|k| !proof.argument(root, k).is_nothing())
        })
    };
    Changes {
        fields: live_fields(module, &proof, syntactic),
        pixel: argument("frag_xy") || argument("screen"),
        quad: argument("quad") || argument("tile"),
        validity: argument("validity"),
        run: argument("params") || argument("masses") || argument("ensemble_spread"),
        canvas: !proof.global("prelude_uniforms").is_nothing(),
    }
}

// ── One shader on its own: questions 1 and 2 ──────────────────────────────────────────────────────────────────────

/// One node's shader on its own, compiled once per kind and code: what each of its uniforms, inputs and context lanes
/// can do to its result, whatever the graph around it does with that result.
pub struct Shader {
    compiled: Arc<Compiled>,
    kind: Kind,
    declaration: Declaration,
    /// Whether the text reads each declared uniform, by declaration order: a `field` is read as the field in the code, so
    /// it has no member to follow, and a read of it is counted as reaching the result.
    read: Vec<bool>,
}

/// A shader's module, kept under its kind and code.
struct Compiled {
    module: Module,
    info: ModuleInfo,
}

/// The node's reads that can reach its result, by declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach {
    /// Each declared uniform's.
    pub uniforms: Vec<bool>,
    /// Each declared field input's.
    pub inputs: Vec<bool>,
    /// A combiner's colour and brightness, a post's colour: each value in-port's that is not a field.
    pub values: Vec<bool>,
    /// The context's members it reads (`sample`, `frag_xy`, `screen`, …), and the read-side fields under `sample`.
    pub lanes: BTreeSet<String>,
    pub fields: BTreeSet<String>,
    /// Whether the proof followed everything; where not, everything is live.
    pub sure: bool,
}

/// What a node needs the frame to provide, read from its source: each is true only where the proof finds that the
/// node reads it into its result. Where the proof could not follow the code, everything it may read is needed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// The Benettin FTLE (`ftle`, `ftle_valid`), which only a tier with the shadow stores.
    pub ftle: bool,
    /// The ensemble's spread (`ensemble_spread`), which only a run with an ensemble has.
    pub spread: bool,
    /// The symbolic word (`word`), which only a tier with the word buffer stores.
    pub word: bool,
    /// A range the host measures over the picture and writes into a `hidden` control's uniform.
    pub measured_range: bool,
    /// The sample's step times (`t_end_step`, `t_dmin_step` and their fractions): the picture follows the playhead.
    pub time: bool,
    /// The previous frame's class, a lane the context does not have yet.
    pub previous_class: bool,
    /// The sample's quad or tile (`ctx.quad`, `ctx.tile`).
    pub quad: bool,
    /// Whether the picture is a function of the sample alone, the same wherever it is drawn: what a bake to a texture
    /// of n̂ needs. It reads no screen place, quad, tile or chart classification.
    pub pure_in_n: bool,
}

impl Needs {
    fn of(reach: &Reach, declaration: &Declaration) -> Needs {
        if !reach.sure {
            return Needs {
                ftle: true,
                spread: true,
                word: true,
                measured_range: true,
                time: true,
                previous_class: true,
                quad: true,
                pure_in_n: false,
            };
        }
        let field = |names: &[&str]| names.iter().any(|n| reach.fields.contains(*n));
        let lane = |names: &[&str]| names.iter().any(|n| reach.lanes.contains(*n));
        // The plan is unclear here; chose a `hidden` control on a live `vec2<f32>` uniform as the mark of a measured
        // range, because the host writes a hidden control and the shipped views' `u_range` is the only one there is.
        let measured_range = declaration
            .described
            .controls
            .iter()
            .filter(|c| c.control == Control::Hidden)
            .any(|c| {
                declaration
                    .uniforms
                    .iter()
                    .zip(&reach.uniforms)
                    .any(|(u, &live)| live && u.name == c.uniform && u.ty == UniformType::Vec2)
            });
        Needs {
            ftle: field(&["ftle", "ftle_valid"]),
            spread: field(&["ensemble_spread"]),
            word: field(&["word"]),
            measured_range,
            time: field(&[
                "t_end_step",
                "t_dmin_step",
                "t_end_fraction",
                "t_dmin_fraction",
            ]),
            previous_class: lane(&["previous"]),
            quad: lane(&["quad", "tile"]),
            pure_in_n: !lane(&["frag_xy", "screen", "quad", "tile", "validity"]),
        }
    }
}

/// Something a node needs that the run does not provide, for the editor to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lack {
    /// The FTLE, on a tier without the Benettin shadow.
    Ftle,
    /// The symbolic word, on a tier without the word buffer.
    Word,
    /// The ensemble's spread, in a run with no ensemble.
    Spread,
}

impl Lack {
    /// What the editor says about it, in plain words.
    pub fn says(self) -> &'static str {
        match self {
            Lack::Ftle => "needs the FTLE, which this run does not compute",
            Lack::Word => "needs the symbolic word, which this run does not store",
            Lack::Spread => {
                "needs the ensemble's spread, which a run with no ensemble does not have"
            }
        }
    }
}

impl Needs {
    /// What the node needs that a run at `tier`, with an ensemble where `ensemble`, does not provide.
    pub fn lacks(&self, tier: Tier, ensemble: bool) -> Vec<Lack> {
        let mut out = Vec::new();
        if self.ftle && !tier.has_ftle {
            out.push(Lack::Ftle);
        }
        if self.word && !tier.has_word {
            out.push(Lack::Word);
        }
        if self.spread && !ensemble {
            out.push(Lack::Spread);
        }
        out
    }
}

/// Whether a control can matter now.
#[derive(Clone, Debug, PartialEq)]
pub enum Standing {
    /// It can change the picture with the settings as they are.
    Matters,
    /// It cannot change the picture whatever the settings: the shader never reads it into its result.
    Never,
    /// It cannot change the picture with the settings as they are; `needs` is one setting that would let it, where a
    /// change of one choice or toggle does.
    NotNow { needs: Option<Setting> },
}

/// A choice or toggle set to a value.
#[derive(Clone, Debug, PartialEq)]
pub struct Setting {
    /// The uniform the control edits.
    pub uniform: String,
    pub value: f64,
}

impl Shader {
    /// The node's slot function, its uniforms' global and its declarations' uniforms.
    fn slot(&self) -> String {
        format!("n0_{}", self.kind.slot().unwrap_or_default())
    }

    fn global(&self) -> Option<Handle<GlobalVariable>> {
        self.compiled
            .module
            .global_variables
            .iter()
            .find(|(_, g)| g.name.as_deref() == Some("n0_uniforms"))
            .map(|(h, _)| h)
    }

    /// The declarations the shader was compiled with.
    pub fn declaration(&self) -> &Declaration {
        &self.declaration
    }

    /// Question 1: what can reach the result whatever the settings.
    pub fn never(&self) -> Reach {
        self.reach(&Held::new())
    }

    /// Question 2: what can reach the result with each choice and toggle at `value(uniform)`.
    pub fn now(&self, value: &dyn Fn(&Uniform) -> Vec<f64>) -> Reach {
        self.reach(&self.held(value, None))
    }

    /// Each choice and toggle's value, as `value` gives it, but the uniform `free`'s, which is left free.
    /// The member of the uniform block that declared uniform `k` is: a `field` has none, so those before it are not counted.
    fn member(&self, k: usize) -> u32 {
        self.declaration.uniforms[..k]
            .iter()
            .filter(|u| u.ty != UniformType::Field)
            .count() as u32
    }

    fn held(&self, value: &dyn Fn(&Uniform) -> Vec<f64>, free: Option<&str>) -> Held {
        let mut held = Held::new();
        let Some(g) = self.global() else { return held };
        for c in &self.declaration.described.controls {
            if !matches!(c.control, Control::Toggle | Control::Choice(_))
                || free == Some(c.uniform.as_str())
            {
                continue;
            }
            let Some(k) = self
                .declaration
                .uniforms
                .iter()
                .position(|u| u.name == c.uniform)
            else {
                continue;
            };
            let u = &self.declaration.uniforms[k];
            if let Some(l) = value(u).first().and_then(|&v| Lit::uniform(u.ty, v)) {
                held.insert((g, self.member(k)), l);
            }
        }
        held
    }

    fn reach(&self, held: &Held) -> Reach {
        let slot = self.slot();
        let proof = Proof::new(
            &self.compiled.module,
            &self.compiled.info,
            &[slot.as_str()],
            held,
        );
        let block = proof.global("n0_uniforms");
        let uniforms = self
            .declaration
            .uniforms
            .iter()
            .enumerate()
            .map(|(k, u)| match u.ty {
                UniformType::Field => self.read[k],
                _ => !block.part(self.member(k)).is_nothing(),
            })
            .collect();
        let ctx = Ctx::of(&self.compiled.module);
        let (values, context) = match self.kind {
            Kind::Combiner => (
                vec![
                    !proof.argument(&slot, 0).is_nothing(),
                    !proof.argument(&slot, 1).is_nothing(),
                ],
                Demand::Nothing,
            ),
            Kind::Post => (
                vec![!proof.argument(&slot, 1).is_nothing()],
                proof.argument(&slot, 0),
            ),
            _ => (Vec::new(), proof.argument(&slot, 0)),
        };
        let inputs_at = ctx.member("inputs");
        let inputs = (0..self.declaration.inputs.len())
            .map(|k| inputs_at.is_none_or(|m| !context.part(m).part(k as u32).is_nothing()))
            .collect();
        let mut lanes = BTreeSet::new();
        for (k, name) in ctx.members.iter().enumerate() {
            if !context.part(k as u32).is_nothing() && name != "inputs" {
                lanes.insert(name.clone());
            }
        }
        let sample = ctx
            .member("sample")
            .map_or(Demand::Whole, |m| context.part(m));
        let fields = match super::named_type(&self.compiled.module, "SimState")
            .map(|t| &self.compiled.module.types[t].inner)
        {
            Some(TypeInner::Struct { members, .. }) => members
                .iter()
                .enumerate()
                .filter(|(k, _)| !sample.part(*k as u32).is_nothing())
                .filter_map(|(_, m)| m.name.clone())
                .collect(),
            _ => BTreeSet::new(),
        };
        Reach {
            uniforms,
            inputs,
            values,
            lanes,
            fields,
            sure: proof.sure(),
        }
    }

    /// What the node needs from the frame, derived from what its source reads into its result (question 1): never
    /// declared by its author.
    pub fn needs(&self) -> Needs {
        Needs::of(&self.never(), &self.declaration)
    }

    /// Each declared uniform's standing with the uniforms at `value(uniform)` (question 2, with question 1 for
    /// [`Standing::Never`]); for one that cannot matter now, the first change of one choice or toggle, in declaration
    /// order and then value order, that lets it.
    // The plan is unclear here; chose to search changes of one choice or toggle only, because the editor names one
    // setting, and a control that needs two at once is dimmed with none named.
    pub fn standing(&self, value: &dyn Fn(&Uniform) -> Vec<f64>) -> Vec<Standing> {
        let never = self.never();
        let now = self.now(value);
        let us = &self.declaration.uniforms;
        let mut out = Vec::with_capacity(us.len());
        for k in 0..us.len() {
            if !never.uniforms[k] {
                out.push(Standing::Never);
                continue;
            }
            // A choice or toggle is judged free, with the others held: its own branches are what it changes.
            let held = self.declaration.described.controls.iter().any(|c| {
                c.uniform == us[k].name && matches!(c.control, Control::Toggle | Control::Choice(_))
            });
            let free = |value: &dyn Fn(&Uniform) -> Vec<f64>| {
                held && self.reach(&self.held(value, Some(&us[k].name))).uniforms[k]
            };
            if now.uniforms[k] || free(value) {
                out.push(Standing::Matters);
                continue;
            }
            let mut needs = None;
            'search: for c in &self.declaration.described.controls {
                let values = match &c.control {
                    Control::Toggle => 2,
                    Control::Choice(names) => names.len(),
                    _ => continue,
                };
                let Some(set) = us.iter().find(|u| u.name == c.uniform) else {
                    continue;
                };
                for v in 0..values {
                    let v = v as f64;
                    if value(set).first() == Some(&v) {
                        continue;
                    }
                    let tried = |u: &Uniform| {
                        if u.name == set.name {
                            vec![v]
                        } else {
                            value(u)
                        }
                    };
                    if self.now(&tried).uniforms[k] || free(&tried) {
                        needs = Some(Setting {
                            uniform: set.name.clone(),
                            value: v,
                        });
                        break 'search;
                    }
                }
            }
            out.push(Standing::NotNow { needs });
        }
        out
    }
}

/// The members of the stain's `Ctx`, by index.
struct Ctx {
    members: Vec<String>,
}

impl Ctx {
    fn of(module: &Module) -> Ctx {
        let members = match super::named_type(module, "Ctx").map(|t| &module.types[t].inner) {
            Some(TypeInner::Struct { members, .. }) => members
                .iter()
                .map(|m| m.name.clone().unwrap_or_default())
                .collect(),
            _ => Vec::new(),
        };
        Ctx { members }
    }

    fn member(&self, name: &str) -> Option<u32> {
        self.members
            .iter()
            .position(|m| m == name)
            .map(|k| k as u32)
    }
}

/// The shaders compiled, by kind and the hash of their code.
type Kept = HashMap<(Kind, u64), Arc<Compiled>>;

/// The most shaders kept compiled; past it the store starts again.
const KEPT_MOST: usize = 256;

/// The shader of a node of `kind` whose occupant's text is `text`, compiled on its own at the full tier with every
/// field filled. The module is kept under its kind and code, so a label edit compiles nothing; the declarations are
/// read from `text` each time, since the code key leaves the descriptive lines out.
pub fn shader(kind: Kind, text: &str) -> Result<Shader, AssembleError> {
    static KEPT: OnceLock<Mutex<Kept>> = OnceLock::new();
    let declaration = declaration_text(kind, text)?;
    let key = (kind, ledger::version::fnv1a64(key_text(text).as_bytes()));
    let kept = KEPT.get_or_init(Mutex::default);
    let found = kept
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        .cloned();
    let compiled = match found {
        Some(c) => c,
        None => {
            let fields = super::every_field()?;
            let fields: Vec<&str> = fields.iter().map(String::as_str).collect();
            let mut source = head(Tier::FULL, &fields)?;
            let mut extra = Vec::new();
            // A `field` is read as the field it defaults to, as the assembler reads it.
            let chosen: Vec<&Uniform> = declaration
                .uniforms
                .iter()
                .filter(|u| u.ty == UniformType::Field)
                .collect();
            let code =
                super::params::specialise(&code_text(text), &chosen, &|u| u.default[0] as usize)?;
            let block: Vec<&Uniform> = declaration
                .uniforms
                .iter()
                .filter(|u| u.ty != UniformType::Field)
                .collect();
            if !block.is_empty() {
                let types: Vec<UniformType> = block.iter().map(|u| u.ty).collect();
                source.push_str(&super::params::library(&types));
                source.push_str("struct n0_Uniforms {\n");
                for u in &block {
                    source.push_str(&format!("    {}: {},\n", u.name, u.ty.wgsl()));
                }
                source
                    .push_str("}\n@group(0) @binding(1) var<uniform> n0_uniforms: n0_Uniforms;\n");
                extra.push("uniforms");
            }
            source.push_str(&prefixed(&code, "n0_", kind, &extra)?);
            source.push_str("\nfn note_non_finite() {}\n");
            let (module, info) = compile(&source)?;
            let c = Arc::new(Compiled { module, info });
            let mut kept = kept.lock().unwrap_or_else(PoisonError::into_inner);
            if kept.len() >= KEPT_MOST {
                kept.clear();
            }
            kept.insert(key, c.clone());
            c
        }
    };
    let read = declaration
        .uniforms
        .iter()
        .map(|u| super::params::reads(text, &u.name))
        .collect();
    Ok(Shader {
        compiled,
        kind,
        declaration,
        read,
    })
}

/// Every uniform and field input the source of a node of `kind`, the text `text`, declares that can never reach its
/// result: a shipped source with one has a control or a port that does nothing. A colour's or brightness's implicit
/// `field` input, which a source that reads `ctx.sample` itself never declares, is not one.
// The plan is unclear here; chose to leave out a colour's or brightness's implicit `field` input, because a view that
// reads `ctx.sample` itself never declares one, and the graph drops its wire instead.
pub fn never_matters(kind: Kind, text: &str) -> Result<Vec<String>, AssembleError> {
    let s = shader(kind, text)?;
    let reach = s.never();
    if !reach.sure {
        return Ok(vec!["the proof could not follow it".to_owned()]);
    }
    let d = s.declaration();
    let declared = Declaration::parse(text)?.inputs.len();
    let uniforms = d
        .uniforms
        .iter()
        .zip(&reach.uniforms)
        .filter(|(_, &live)| !live)
        .map(|(u, _)| format!("uniform `{}`", u.name));
    let inputs = d
        .inputs
        .iter()
        .zip(&reach.inputs)
        .take(declared)
        .filter(|(_, &live)| !live)
        .map(|(i, _)| format!("input `{}`", i.name));
    Ok(uniforms.chain(inputs).collect())
}

/// The declarations of a node of `kind` with the text `text`, as [`super::declaration`] reads them.
fn declaration_text(kind: Kind, text: &str) -> Result<Declaration, AssembleError> {
    let mut d = Declaration::parse(text)?;
    if matches!(kind, Kind::Colour | Kind::Brightness) && d.inputs.is_empty() {
        d.inputs.push(super::default_input());
    }
    Ok(d)
}

// ── Dropping ──────────────────────────────────────────────────────────────────────────────────────────────────────

/// What `stain`'s assembly may leave out, from `proof` over it assembled with nothing left out: a node the walk calls
/// whose result reaches nothing and which has no effect; a field input its node never reads; a uniform its node never
/// reads, where every read of the node's uniforms is `uniforms.<name>`, so a read of it can be given its zero.
// The plan is unclear here; chose never to drop the combiner, because whether `shade()` calls it depends on the identity
// rules, which must not move with the proof.
pub(super) fn dropped(
    stain: &super::Stain,
    proof: &Proof,
    written: bool,
) -> Result<super::Dropped, AssembleError> {
    let mut out = super::Dropped::default();
    if !proof.sure() {
        return Ok(out);
    }
    let canonical = stain.canonical();
    let stain = canonical.stain();
    let (colour, brightness) = stain.slots();
    let chain = stain.chain();
    let sources = stain.sources();
    let combiner = stain.position(Kind::Combiner);
    let called: Vec<usize> = sources
        .iter()
        .copied()
        .chain(colour)
        .chain(brightness)
        .chain(chain.iter().copied())
        .collect();
    let slot = |i: usize| format!("n{i}_{}", stain.nodes[i].kind.slot().unwrap_or_default());
    for &i in &called {
        let f = slot(i);
        if proof.result(&f).is_nothing() && !proof.has_effects(&f) {
            out.nodes.push(i);
        }
    }
    let ctx_inputs = proof_ctx_inputs(proof);
    for &i in called.iter().filter(|i| !out.nodes.contains(i)) {
        let context = proof.argument(&slot(i), 0);
        for (k, j) in stain.field_inputs(i).into_iter().enumerate() {
            let read = ctx_inputs.is_none_or(|m| !context.part(m).part(k as u32).is_nothing());
            if !read && j.is_some() {
                out.inputs.push((i, k));
            }
        }
    }
    // A source stays where any wire from it is still read.
    let dead = out.nodes.clone();
    out.nodes.retain(|&s| {
        stain.nodes[s].kind != Kind::Source
            || called.iter().all(|&i| {
                dead.contains(&i)
                    || stain
                        .field_inputs(i)
                        .into_iter()
                        .enumerate()
                        .all(|(k, j)| j != Some(s) || out.inputs.contains(&(i, k)))
            })
    });
    for i in called.into_iter().chain([combiner]) {
        let uniforms = &stain.declarations[i].uniforms;
        if uniforms.is_empty() {
            continue;
        }
        let text = stain.text(i).unwrap_or_default();
        let text = if written {
            text.to_owned()
        } else {
            code_text(text).into_owned()
        };
        let gone = out.nodes.contains(&i);
        let block = proof.global(&format!("n{i}_uniforms"));
        if !gone && without_uniforms(&text, &[], "").is_none() {
            continue;
        }
        // A `field` has no member in an assembled block: it is read as the field, in the code.
        for (member, u) in uniforms
            .iter()
            .filter(|u| u.ty != UniformType::Field)
            .enumerate()
        {
            if gone || block.part(member as u32).is_nothing() {
                out.uniforms.push((i, u.clone()));
            }
        }
    }
    Ok(out)
}

/// The index of `inputs` in the stain's `Ctx`.
fn proof_ctx_inputs(proof: &Proof) -> Option<u32> {
    Ctx::of(proof.module).member("inputs")
}

/// `text`, a node's WGSL, with each read `uniforms.<name>` of a uniform in `gone` read from `to` instead; `None` where
/// a node reads its uniforms any other way (whole, through a pointer, or a name it declares itself), so none can be
/// left out.
pub(super) fn without_uniforms(text: &str, gone: &[&Uniform], to: &str) -> Option<String> {
    use crate::lex::Tok;
    let toks = super::lex(text).ok()?;
    let sig: Vec<usize> = (0..toks.len())
        .filter(|&k| !matches!(toks[k].0, Tok::Comment | Tok::Space))
        .collect();
    let at = |k: usize| sig.get(k).map_or("", |&t| &text[toks[t].1..toks[t].2]);
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for k in 0..sig.len() {
        if at(k) != "uniforms" || toks[sig[k]].0 != Tok::Ident || (k > 0 && at(k - 1) == ".") {
            continue;
        }
        let name = at(k + 2);
        let plain = at(k + 1) == "."
            && sig.get(k + 2).is_some_and(|&t| toks[t].0 == Tok::Ident)
            && (k == 0 || at(k - 1) != "&");
        if !plain {
            return None;
        }
        if let Some(u) = gone.iter().find(|u| u.name == name) {
            out.push_str(&text[from..toks[sig[k]].1]);
            #[cfg(feature = "controls")]
            if probe::CONSTANT.with(std::cell::Cell::get) {
                out.push_str(&format!("({}())", u.ty.wgsl()));
                from = toks[sig[k + 2]].2;
                continue;
            }
            let _ = u;
            out.push_str(to);
            from = toks[sig[k]].2;
        }
    }
    out.push_str(&text[from..]);
    Some(out)
}
