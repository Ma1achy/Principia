//! The decode stage's hold on `L_z` in the production module (`kernel::decode::hold`): its exact tests, which hold all
//! of its loops, are one function, out of line, reached from a chart's decode entry only under a branch on the
//! uniform's `L_z` bit. Every pipeline parses the whole module, so a hold copied into each decode entry slows every
//! pipeline made from it, decode or not; and a hold reached with the switch off runs its exact tests on every sample.
//!
//! The hold is found as the one function of its exact tests' signature, `(v, w) → bool` over three bodies each,
//! holding a loop. A chart whose `L_z` is an axis or held by construction never holds, so its entry reaches none. It reads `target/spirv/kernel.spv`, so `cargo xtask build-kernel` runs first.
//! The controls are the same module with the branch's condition made `true`, and with one entry's calls moved to a
//! copy of the hold.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "controls")]
use naga::Span;
use naga::{
    AddressSpace, ArraySize, BinaryOperator, Block, Expression, Function, Handle, Literal,
    LocalVariable, Module, Scalar, Statement, Type, TypeInner,
};
use validation::negative_control;
use xtask::build_kernel::{to_wgsl, SPV};

/// The stage's `stage_decode_` entries that decode no chart.
const HELPERS: [&str; 3] = [
    "stage_decode_describe",
    "stage_decode_link_tanh",
    "stage_decode_sine_cosine",
];

/// The charts' decode entries the stage has.
const CHART_ENTRIES: usize = 13;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The production module, parsed.
fn module() -> Module {
    let spv = fs::read(root().join(SPV))
        .expect("target/spirv/kernel.spv: run `cargo xtask build-kernel` first");
    let wgsl = to_wgsl(&spv).expect("the kernel translates");
    naga::front::wgsl::parse_str(&wgsl).expect("the production WGSL parses")
}

/// Every statement of `block`, nested blocks included, with the `If` conditions it sits under.
fn walk<'a>(
    block: &'a Block,
    under: &mut Vec<Handle<Expression>>,
    visit: &mut dyn FnMut(&'a Statement, &[Handle<Expression>]),
) {
    for s in block.iter() {
        visit(s, under);
        match s {
            Statement::Block(b) => walk(b, under, visit),
            Statement::If {
                condition,
                accept,
                reject,
            } => {
                under.push(*condition);
                walk(accept, under, visit);
                walk(reject, under, visit);
                under.pop();
            }
            Statement::Switch { cases, .. } => {
                for c in cases {
                    walk(&c.body, under, visit);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                walk(body, under, visit);
                walk(continuing, under, visit);
            }
            _ => {}
        }
    }
}

/// The functions `f` calls directly, and whether it holds a loop.
fn calls(f: &Function) -> (BTreeSet<Handle<Function>>, bool) {
    let mut out = BTreeSet::new();
    let mut looped = false;
    walk(&f.body, &mut Vec::new(), &mut |s, _| match s {
        Statement::Call { function, .. } => {
            out.insert(*function);
        }
        Statement::Loop { .. } => looped = true,
        _ => {}
    });
    (out, looped)
}

/// The functions `f` reaches through calls.
fn reach(m: &Module, f: &Function) -> BTreeSet<Handle<Function>> {
    let mut seen = BTreeSet::new();
    let mut todo: Vec<_> = calls(f).0.into_iter().collect();
    while let Some(h) = todo.pop() {
        if seen.insert(h) {
            todo.extend(calls(&m.functions[h]).0);
        }
    }
    seen
}

/// The local variable `e` points into, if any.
fn local(f: &Function, e: Handle<Expression>) -> Option<Handle<LocalVariable>> {
    match f.expressions[e] {
        Expression::LocalVariable(l) => Some(l),
        Expression::Access { base, .. } | Expression::AccessIndex { base, .. } => local(f, base),
        _ => None,
    }
}

/// The values `e` is computed from: its operands, and for a load from a local every value stored there and the
/// conditions each store sits under.
fn operands(
    f: &Function,
    stores: &HashMap<Handle<LocalVariable>, Vec<Handle<Expression>>>,
    e: Handle<Expression>,
) -> Vec<Handle<Expression>> {
    match &f.expressions[e] {
        Expression::Access { base, index } => vec![*base, *index],
        Expression::AccessIndex { base, .. } => vec![*base],
        Expression::Splat { value, .. } => vec![*value],
        Expression::Swizzle { vector, .. } => vec![*vector],
        Expression::Compose { components, .. } => components.clone(),
        Expression::Load { pointer } => {
            let mut v = vec![*pointer];
            if let Some(l) = local(f, *pointer) {
                v.extend(stores.get(&l).into_iter().flatten());
            }
            v
        }
        Expression::Unary { expr, .. } => vec![*expr],
        Expression::Binary { left, right, .. } => vec![*left, *right],
        Expression::Select {
            condition,
            accept,
            reject,
        } => vec![*condition, *accept, *reject],
        Expression::Relational { argument, .. } => vec![*argument],
        Expression::Math {
            arg,
            arg1,
            arg2,
            arg3,
            ..
        } => [Some(*arg), *arg1, *arg2, *arg3]
            .into_iter()
            .flatten()
            .collect(),
        Expression::As { expr, .. } => vec![*expr],
        _ => Vec::new(),
    }
}

/// Every value stored into each local of `f`, with the conditions of the `If`s around the store.
fn stores(f: &Function) -> HashMap<Handle<LocalVariable>, Vec<Handle<Expression>>> {
    let mut out: HashMap<_, Vec<_>> = HashMap::new();
    walk(&f.body, &mut Vec::new(), &mut |s, under| {
        if let Statement::Store { pointer, value } = s {
            if let Some(l) = local(f, *pointer) {
                let v = out.entry(l).or_default();
                v.push(*value);
                v.extend(under);
            }
        }
    });
    out
}

/// Whether `e` is computed from a uniform buffer.
fn from_uniform(
    m: &Module,
    f: &Function,
    st: &HashMap<Handle<LocalVariable>, Vec<Handle<Expression>>>,
    e: Handle<Expression>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut todo = vec![e];
    while let Some(e) = todo.pop() {
        if !seen.insert(e) {
            continue;
        }
        if let Expression::GlobalVariable(g) = f.expressions[e] {
            if m.global_variables[g].space == AddressSpace::Uniform {
                return true;
            }
        }
        todo.extend(operands(f, st, e));
    }
    false
}

/// Whether `e` is computed from the uniform's `L_z` bit: a `& 2` of a uniform's word.
fn reads_lz_bit(
    m: &Module,
    f: &Function,
    st: &HashMap<Handle<LocalVariable>, Vec<Handle<Expression>>>,
    e: Handle<Expression>,
) -> bool {
    let two =
        |e: Handle<Expression>| matches!(f.expressions[e], Expression::Literal(Literal::U32(2)));
    let mut seen = BTreeSet::new();
    let mut todo = vec![e];
    while let Some(e) = todo.pop() {
        if !seen.insert(e) {
            continue;
        }
        if let Expression::Binary {
            op: BinaryOperator::And,
            left,
            right,
        } = f.expressions[e]
        {
            if (two(right) && from_uniform(m, f, st, left))
                || (two(left) && from_uniform(m, f, st, right))
            {
                return true;
            }
        }
        todo.extend(operands(f, st, e));
    }
    false
}

/// Whether `ty` is three bodies' two components, `array<array<f32, 2>, 3>`.
fn bodies(m: &Module, ty: Handle<Type>) -> bool {
    let array = |ty: Handle<Type>, n: u32| match m.types[ty].inner {
        TypeInner::Array {
            base,
            size: ArraySize::Constant(len),
            ..
        } if len.get() == n => Some(base),
        _ => None,
    };
    array(ty, 3)
        .and_then(|row| array(row, 2))
        .is_some_and(|x| matches!(m.types[x].inner, TypeInner::Scalar(Scalar::F32)))
}

/// The hold in `m`: the one function of the exact tests' signature, `(v, w)` three bodies each to a `bool`, that holds
/// a loop, reached from a chart's decode entry. A chart whose `L_z` is an axis or held by its construction never holds,
/// so its entry reaches none.
fn the_hold(m: &Module) -> Handle<Function> {
    let entries: Vec<_> = m
        .entry_points
        .iter()
        .filter(|e| e.name.starts_with("stage_decode_") && !HELPERS.contains(&e.name.as_str()))
        .collect();
    assert_eq!(
        entries.len(),
        CHART_ENTRIES,
        "the decode stage's chart entries"
    );
    let reached: Vec<_> = entries.iter().map(|e| reach(m, &e.function)).collect();
    let tests = |f: &Function| {
        f.arguments.len() == 2
            && f.arguments.iter().all(|a| bodies(m, a.ty))
            && f.result
                .as_ref()
                .is_some_and(|r| matches!(m.types[r.ty].inner, TypeInner::Scalar(Scalar::BOOL)))
    };
    let shared: Vec<_> = m
        .functions
        .iter()
        .filter(|(_, f)| tests(f) && calls(f).1)
        .map(|(h, _)| h)
        .collect();
    assert_eq!(
        shared.len(),
        1,
        "the hold is not one function shared by the decode entries: {} functions of its tests' signature",
        shared.len()
    );
    let hold = shared[0];
    let reaching = reached.iter().filter(|r| r.contains(&hold)).count();
    assert!(reaching > 0, "no decode entry reaches the hold");
    hold
}

/// The hold in `m` is one function, and every call to it sits under a branch on the uniform's `L_z` bit.
fn check_gated(m: &Module) {
    let hold = the_hold(m);
    let mut sites = 0;
    let all = m
        .functions
        .iter()
        .map(|(_, f)| f)
        .chain(m.entry_points.iter().map(|e| &e.function));
    for f in all {
        let st = stores(f);
        walk(&f.body, &mut Vec::new(), &mut |s, under| {
            if let Statement::Call { function, .. } = s {
                if *function == hold {
                    sites += 1;
                    assert!(
                        under.iter().any(|c| reads_lz_bit(m, f, &st, *c)),
                        "a decode entry reaches the hold with L_z off: no branch on the uniform's L_z bit around it"
                    );
                }
            }
        });
    }
    assert!(sites > 0, "nothing calls the hold");
}

#[test]
fn hold_gated_in_the_production_module() {
    check_gated(&module());
}

#[test]
fn hold_once_in_the_production_module() {
    the_hold(&module());
}

#[cfg(feature = "controls")]
/// Every `If` in `block` around a call to `hold` made unconditional, its condition `yes`.
fn ungate(block: &mut Block, hold: Handle<Function>, yes: Handle<Expression>) -> bool {
    let mut calls_hold = false;
    for s in block.iter_mut() {
        match s {
            Statement::Call { function, .. } if *function == hold => calls_hold = true,
            Statement::Block(b) => calls_hold |= ungate(b, hold, yes),
            Statement::If {
                condition,
                accept,
                reject,
            } => {
                let inside = ungate(accept, hold, yes) | ungate(reject, hold, yes);
                if inside {
                    *condition = yes;
                }
                calls_hold |= inside;
            }
            Statement::Switch { cases, .. } => {
                for c in cases {
                    calls_hold |= ungate(&mut c.body, hold, yes);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => calls_hold |= ungate(body, hold, yes) | ungate(continuing, hold, yes),
            _ => {}
        }
    }
    calls_hold
}

#[cfg(feature = "controls")]
/// The production module with every branch around the hold's calls made `true`: the hold reached with `L_z` off.
fn ungated() -> Module {
    let mut m = module();
    let hold = the_hold(&m);
    for (_, f) in m.functions.iter_mut() {
        let yes = f
            .expressions
            .append(Expression::Literal(Literal::Bool(true)), Span::default());
        ungate(&mut f.body, hold, yes);
    }
    m
}

negative_control!(
    hold_gated_in_the_production_module,
    "the hold's branch made unconditional",
    expected = "reaches the hold with L_z off",
    check_gated(&ungated())
);

#[cfg(feature = "controls")]
/// Every call to `from` in `block` sent to `to`.
fn redirect(block: &mut Block, from: Handle<Function>, to: Handle<Function>) {
    for s in block.iter_mut() {
        match s {
            Statement::Call { function, .. } if *function == from => *function = to,
            Statement::Block(b) => redirect(b, from, to),
            Statement::If { accept, reject, .. } => {
                redirect(accept, from, to);
                redirect(reject, from, to);
            }
            Statement::Switch { cases, .. } => {
                for c in cases {
                    redirect(&mut c.body, from, to);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                redirect(body, from, to);
                redirect(continuing, from, to);
            }
            _ => {}
        }
    }
}

#[cfg(feature = "controls")]
/// The production module with the latent chart's decode calling its own copy of the hold: the hold copied per entry.
fn copied() -> Module {
    let mut m = module();
    let hold = the_hold(&m);
    let copy = m
        .functions
        .append(m.functions[hold].clone(), Span::default());
    let latent = m
        .entry_points
        .iter()
        .find(|e| e.name == "stage_decode_latent_defaults")
        .expect("the latent chart's decode entry");
    let mut todo: Vec<_> = calls(&latent.function).0.into_iter().collect();
    let mut seen = BTreeSet::new();
    while let Some(h) = todo.pop() {
        if seen.insert(h) {
            todo.extend(calls(&m.functions[h]).0);
        }
    }
    for h in seen {
        if h != hold {
            let mut body = std::mem::take(&mut m.functions[h].body);
            redirect(&mut body, hold, copy);
            m.functions[h].body = body;
        }
    }
    m
}

negative_control!(
    hold_once_in_the_production_module,
    "the hold copied into one decode entry",
    expected = "the hold is not one function shared by the decode entries",
    {
        the_hold(&copied());
    }
);
