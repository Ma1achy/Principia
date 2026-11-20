//! QA's tests for TASK-M7-03's REQ-COL-011 (colour_composition §1): a colour occupant is an expression tree whose
//! every node outputs `vec3` or `f32`, and only the root is constrained by the slot's signature, `vec3` for `colour`
//! and `f32` for `brightness`. Verify: a tree with an `f32` node feeding a `vec3` root type-checks; an `f32` root in
//! the colour slot fails type-checking. Each test has its negative control (R-176).

use engine::contract::stain::occupant::{Expr, Primitive, Slot, Tree, ValueType};
use render::codegen::{schema::MapParam, Arg, Call, Param};
use validation::negative_control;

/// The tests' operations, each typed by §1's two output types.
#[derive(Clone, Debug, PartialEq)]
enum Op {
    /// `→ f32`: a scalar leaf.
    Scalar,
    /// `→ vec3`: a colour leaf.
    Colour,
    /// `f32 → vec3`: a ramp.
    Ramp,
    /// `vec3 → f32`: a lightness of a colour.
    Lightness,
    /// `vec3, f32 → vec3`: a colour scaled by a scalar.
    Scale,
}

impl Primitive for Op {
    fn output(&self) -> ValueType {
        match self {
            Op::Scalar | Op::Lightness => ValueType::F32,
            Op::Colour | Op::Ramp | Op::Scale => ValueType::Vec3,
        }
    }

    fn inputs(&self) -> Vec<ValueType> {
        match self {
            Op::Scalar | Op::Colour => vec![],
            Op::Ramp => vec![ValueType::F32],
            Op::Lightness => vec![ValueType::Vec3],
            Op::Scale => vec![ValueType::Vec3, ValueType::F32],
        }
    }

    fn call(&self) -> Call {
        let (name, function, args, params) = match self {
            Op::Scalar => (
                "scalar",
                "f32",
                vec![Arg::Param(0)],
                vec![Param::ranged("s", MapParam::S, 0.5)],
            ),
            Op::Colour => (
                "colour",
                "oklch_to_linear",
                vec![Arg::Param(0), Arg::Param(1), Arg::Param(0)],
                vec![
                    Param::ranged("L", MapParam::L, 0.6),
                    Param::ranged("C", MapParam::C, 0.1),
                ],
            ),
            Op::Ramp => ("ramp", "ramp_grey", vec![Arg::Input(0)], vec![]),
            Op::Lightness => ("lightness", "length", vec![Arg::Input(0)], vec![]),
            Op::Scale => (
                "scale",
                "mix",
                vec![Arg::Input(0), Arg::Input(0), Arg::Input(1)],
                vec![],
            ),
        };
        Call {
            name: name.into(),
            function: function.into(),
            args,
            params,
        }
    }
}

fn leaf(id: u32, op: Op) -> Expr<Op> {
    Expr::new(id, op, vec![])
}

/// Every tree in `ok` type-checks in its slot; every tree in `refused` does not.
fn check(ok: Vec<(Slot, Expr<Op>)>, refused: Vec<(Slot, Expr<Op>)>) {
    for (slot, root) in ok {
        if let Err(e) = Tree::new(slot, root.clone()) {
            panic!("a well-typed tree in the {slot:?} slot was refused: {e}\n{root:?}");
        }
    }
    for (slot, root) in refused {
        assert!(
            Tree::new(slot, root.clone()).is_err(),
            "an ill-typed tree type-checked in the {slot:?} slot: {root:?}"
        );
    }
}

/// An `f32` node directly under a `vec3` root: ramp(scalar).
fn f32_feeds_vec3_root() -> Expr<Op> {
    Expr::new(1, Op::Ramp, vec![leaf(0, Op::Scalar)])
}

/// `f32` nodes at every depth under a `vec3` root: scale(ramp(lightness(colour)), lightness(scale(colour, scalar))).
fn mixed_deep() -> Expr<Op> {
    Expr::new(
        9,
        Op::Scale,
        vec![
            Expr::new(
                3,
                Op::Ramp,
                vec![Expr::new(2, Op::Lightness, vec![leaf(1, Op::Colour)])],
            ),
            Expr::new(
                8,
                Op::Lightness,
                vec![Expr::new(
                    7,
                    Op::Scale,
                    vec![leaf(5, Op::Colour), leaf(6, Op::Scalar)],
                )],
            ),
        ],
    )
}

/// An `f32` root over `vec3` nodes: lightness(scale(colour, scalar)).
fn f32_root_over_vec3() -> Expr<Op> {
    Expr::new(
        3,
        Op::Lightness,
        vec![Expr::new(
            2,
            Op::Scale,
            vec![leaf(0, Op::Colour), leaf(1, Op::Scalar)],
        )],
    )
}

#[test]
fn qa_occupant_typecheck_only_the_root_is_constrained() {
    check(
        vec![
            (Slot::Colour, f32_feeds_vec3_root()),
            (Slot::Colour, mixed_deep()),
            (Slot::Colour, leaf(0, Op::Colour)),
            (Slot::Brightness, f32_root_over_vec3()),
            (Slot::Brightness, leaf(0, Op::Scalar)),
        ],
        vec![
            (Slot::Colour, leaf(0, Op::Scalar)),
            (Slot::Colour, f32_root_over_vec3()),
            (Slot::Brightness, f32_feeds_vec3_root()),
            (Slot::Brightness, mixed_deep()),
            (Slot::Brightness, leaf(0, Op::Colour)),
        ],
    );
}

negative_control!(
    qa_occupant_typecheck_only_the_root_is_constrained,
    "an f32 root in the colour slot is refused",
    expected = "was refused",
    check(vec![(Slot::Colour, f32_root_over_vec3())], vec![])
);

#[test]
fn qa_occupant_typecheck_inner_types_still_checked() {
    // A slot constrains only the root, but each node's inputs are still its operation's: a `vec3` fed where `f32`
    // is taken, an `f32` where `vec3` is, and a missing input are refused, whatever the root.
    check(
        vec![],
        vec![
            (
                Slot::Colour,
                Expr::new(1, Op::Ramp, vec![leaf(0, Op::Colour)]),
            ),
            (
                Slot::Brightness,
                Expr::new(1, Op::Lightness, vec![leaf(0, Op::Scalar)]),
            ),
            (
                Slot::Colour,
                Expr::new(2, Op::Scale, vec![leaf(0, Op::Colour)]),
            ),
            (
                Slot::Colour,
                Expr::new(2, Op::Scale, vec![leaf(0, Op::Scalar), leaf(1, Op::Colour)]),
            ),
        ],
    );
}

negative_control!(
    qa_occupant_typecheck_inner_types_still_checked,
    "a well-typed inner wiring is not refused",
    expected = "an ill-typed tree type-checked",
    check(
        vec![],
        vec![(
            Slot::Colour,
            Expr::new(2, Op::Scale, vec![leaf(0, Op::Colour), leaf(1, Op::Scalar)]),
        )],
    )
);

/// A checked tree whose root fills the slot generates the slot's function with the slot's signature.
fn check_generates(slot: Slot, root: Expr<Op>, want: &str) {
    let tree = Tree::new(slot, root).unwrap_or_else(|e| panic!("{e}"));
    let g = tree.generate().unwrap_or_else(|e| panic!("{e}"));
    assert!(
        g.source.contains(want),
        "the generated slot function is not `{want}`:\n{}",
        g.source
    );
}

#[test]
fn qa_occupant_typecheck_root_sets_the_slot_function() {
    check_generates(
        Slot::Colour,
        mixed_deep(),
        "fn colour(ctx: Ctx) -> vec3<f32>",
    );
    check_generates(
        Slot::Brightness,
        f32_root_over_vec3(),
        "fn brightness(ctx: Ctx) -> f32",
    );
}

negative_control!(
    qa_occupant_typecheck_root_sets_the_slot_function,
    "the colour slot's function is not an f32",
    expected = "is not",
    check_generates(Slot::Colour, mixed_deep(), "fn colour(ctx: Ctx) -> f32")
);
