//! A colour occupant's expression tree (colour_composition §1): a tree over the two primitive families, site-blend
//! (§1.1) and field-ramp (§1.2), and the combinators (§1.3). Every node outputs `vec3` or `f32` ([`ValueType`]), and
//! only the root is constrained, by the slot's signature: `vec3` for `colour`, `f32` for `brightness`
//! ([`Slot::signature`]). So an `f32` node may feed a `vec3` node anywhere below the root.
//!
//! A tree is built checked ([`Tree::new`]): its node ids are distinct, each node has the inputs its operation takes,
//! each input's output is the type its operation takes there, and the root's output is the slot's. A checked tree
//! lowers to the render crate's codegen ([`Tree::generate`]; colour_composition §5), whose text is the occupant's
//! WGSL for the one assembler.
//!
//! A node's operation is a [`Primitive`]: what it outputs, what it takes and the library call it lowers to. The
//! occupant algebra's operations, the families' and the combinators' node variants, are TASK-M7-05 to -10's, each a
//! [`Primitive`]; the tree, its type-checker and its lowering are the same for every one.

use std::collections::BTreeSet;
use std::fmt;

use render::codegen::{self, CodegenError, Generated};
use serde::{Deserialize, Serialize};

pub use render::codegen::{Slot, ValueType};

/// A tree node's id: unique in its tree and stable across edits, so its generated function keeps its name
/// ([`codegen::function_name`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExprId(pub u32);

/// A node's operation: its output type, the types of the inputs it takes, in order, and the shared-library call it
/// lowers to (colour_composition §5).
pub trait Primitive {
    /// The node's output type.
    fn output(&self) -> ValueType;
    /// The inputs it takes, each its type, in input order.
    fn inputs(&self) -> Vec<ValueType>;
    /// The call it lowers to: its name, its library function, the call's arguments and its parameters.
    fn call(&self) -> codegen::Call;
}

/// One node of an expression tree: its id, its operation and its inputs, each a subtree, in input order.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr<P> {
    pub id: ExprId,
    pub op: P,
    pub inputs: Vec<Expr<P>>,
}

impl<P: Primitive> Expr<P> {
    /// A node of `op` with `inputs`.
    pub fn new(id: u32, op: P, inputs: Vec<Expr<P>>) -> Expr<P> {
        Expr {
            id: ExprId(id),
            op,
            inputs,
        }
    }

    /// The node's output type, its operation's.
    pub fn output(&self) -> ValueType {
        self.op.output()
    }

    /// Whether the subtree is well typed: its ids not in `seen` and distinct, each node's inputs the number its
    /// operation takes, each of the type it takes there.
    fn check(&self, seen: &mut BTreeSet<ExprId>) -> Result<(), TypeError> {
        if !seen.insert(self.id) {
            return Err(TypeError(format!(
                "node {}: the id is used twice in the tree",
                self.id.0
            )));
        }
        let want = self.op.inputs();
        if want.len() != self.inputs.len() {
            return Err(TypeError(format!(
                "node {} takes {} input(s); {} given",
                self.id.0,
                want.len(),
                self.inputs.len()
            )));
        }
        for (k, (&ty, input)) in want.iter().zip(&self.inputs).enumerate() {
            if input.output() != ty {
                return Err(TypeError(format!(
                    "node {}'s input {k} takes {}; node {} outputs {}",
                    self.id.0,
                    ty.wgsl(),
                    input.id.0,
                    input.output().wgsl()
                )));
            }
            input.check(seen)?;
        }
        Ok(())
    }

    /// The subtree as the codegen takes it.
    fn lower(&self) -> codegen::Node {
        codegen::Node {
            id: self.id.0,
            output: self.output(),
            call: self.op.call(),
            inputs: self.inputs.iter().map(Expr::lower).collect(),
        }
    }
}

/// A type-checked occupant: an expression tree filling a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree<P> {
    slot: Slot,
    root: Expr<P>,
}

impl<P: Primitive> Tree<P> {
    /// `root` as `slot`'s occupant, or why it does not type-check: a node's id used twice, a node with other than
    /// the inputs its operation takes or an input of another type, or a root whose output is not the slot's.
    pub fn new(slot: Slot, root: Expr<P>) -> Result<Tree<P>, TypeError> {
        root.check(&mut BTreeSet::new())?;
        if root.output() != slot.signature() {
            return Err(TypeError(format!(
                "the root, node {}, outputs {}; the {} slot takes {}",
                root.id.0,
                root.output().wgsl(),
                slot.kind().name(),
                slot.signature().wgsl()
            )));
        }
        Ok(Tree { slot, root })
    }

    /// The slot it fills.
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// The root.
    pub fn root(&self) -> &Expr<P> {
        &self.root
    }

    /// The occupant's WGSL (colour_composition §5; [`codegen::generate`]).
    pub fn generate(&self) -> Result<Generated, CodegenError> {
        codegen::generate(self.slot, &self.root.lower())
    }
}

/// Why a tree did not type-check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeError(pub String);

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TypeError {}
