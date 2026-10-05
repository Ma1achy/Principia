//! The occupant algebra's expression tree (colour_composition §1; REQ-COL-011): a tree with an `f32` node feeding a
//! `vec3` root type-checks; an `f32` root in the colour slot fails type-checking; a duplicated id, a wrong input count
//! and an input of the wrong type are refused; and a checked tree lowers to one generated function per node. The
//! operations here are the tests' own, each a call into the shared WGSL library: the algebra's are TASK-M7-05 to -10's.
//! Each test registers its negative control (R-176).

use render::codegen::{self, schema::MapParam, Arg, Call, Param};

use crate::contract::stain::occupant::{Expr, Primitive, Slot, Tree, ValueType};

/// The tests' operations.
#[derive(Clone, Debug, PartialEq)]
enum T {
    /// An `f32` leaf: the lightness `L`, `f32(L)`.
    Lightness(f64),
    /// `f32 → vec3`: the greyscale ramp, `ramp_grey`.
    Grey,
    /// A `vec3` leaf: OKLCH at `L`, `C` and a hue in turns, `oklch_to_linear`.
    Lch(f64, f64, f64),
    /// `vec3, vec3 → vec3`: a constant-weight blend at strength `s`, `mix`.
    Mix(f64),
}

impl Primitive for T {
    fn output(&self) -> ValueType {
        match self {
            T::Lightness(_) => ValueType::F32,
            _ => ValueType::Vec3,
        }
    }

    fn inputs(&self) -> Vec<ValueType> {
        match self {
            T::Lightness(_) | T::Lch(..) => vec![],
            T::Grey => vec![ValueType::F32],
            T::Mix(_) => vec![ValueType::Vec3, ValueType::Vec3],
        }
    }

    fn call(&self) -> Call {
        let call = |name: &str, function: &str, args: Vec<Arg>, params: Vec<Param>| Call {
            name: name.into(),
            function: function.into(),
            args,
            params,
        };
        match *self {
            T::Lightness(l) => call(
                "lightness",
                "f32",
                vec![Arg::Param(0)],
                vec![Param::ranged("L", MapParam::L, l)],
            ),
            T::Grey => call("grey", "ramp_grey", vec![Arg::Input(0)], vec![]),
            T::Lch(l, c, hue) => call(
                "oklch",
                "oklch_to_linear",
                vec![Arg::Param(0), Arg::Param(1), Arg::Param(2)],
                vec![
                    Param::ranged("L", MapParam::L, l),
                    Param::ranged("C", MapParam::C, c),
                    Param::free("hue", render::assemble::UniformType::F32, vec![hue]),
                ],
            ),
            T::Mix(s) => call(
                "mix_const",
                "mix",
                vec![Arg::Input(0), Arg::Input(1), Arg::Param(0)],
                vec![Param::ranged("s", MapParam::S, s)],
            ),
        }
    }
}

fn leaf(id: u32, op: T) -> Expr<T> {
    Expr::new(id, op, vec![])
}

/// `mix_const(oklch, grey(lightness))`: the `f32` lightness feeds the `vec3` grey, under the `vec3` root.
fn f32_under_vec3_root() -> Expr<T> {
    Expr::new(
        3,
        T::Mix(0.25),
        vec![
            leaf(0, T::Lch(0.7, 0.12, 0.3)),
            Expr::new(2, T::Grey, vec![leaf(1, T::Lightness(0.6))]),
        ],
    )
}

// ----- an f32 node feeding a vec3 root type-checks -----

fn check_accepted(slot: Slot, root: Expr<T>) {
    match Tree::new(slot, root.clone()) {
        Ok(tree) => assert_eq!((tree.slot(), tree.root()), (slot, &root)),
        Err(e) => panic!("the tree did not type-check: {e}"),
    }
}

#[test]
fn occupant_typecheck_f32_node_feeds_vec3_root() {
    check_accepted(
        Slot::Colour,
        Expr::new(1, T::Grey, vec![leaf(0, T::Lightness(0.5))]),
    );
    check_accepted(Slot::Colour, f32_under_vec3_root());
    // The slot constrains only the root: an f32 root fills the brightness slot.
    check_accepted(Slot::Brightness, leaf(0, T::Lightness(0.5)));
}

validation::negative_control!(
    occupant_typecheck_f32_node_feeds_vec3_root,
    "the f32 lightness wired straight into mix_const's vec3 input must not type-check",
    expected = "the tree did not type-check",
    check_accepted(
        Slot::Colour,
        Expr::new(
            2,
            T::Mix(0.5),
            vec![leaf(0, T::Lch(0.7, 0.1, 0.0)), leaf(1, T::Lightness(0.5))]
        )
    )
);

// ----- an f32 root in the colour slot fails type-checking -----

fn check_refused(slot: Slot, root: Expr<T>, why: &str) {
    match Tree::new(slot, root) {
        Ok(_) => panic!("the tree type-checked; it must be refused for `{why}`"),
        Err(e) => assert!(
            e.to_string().contains(why),
            "the tree was refused for `{e}`, not for `{why}`"
        ),
    }
}

#[test]
fn occupant_typecheck_f32_root_in_colour_slot_fails() {
    check_refused(
        Slot::Colour,
        leaf(0, T::Lightness(0.5)),
        "the root, node 0, outputs f32; the colour slot takes vec3<f32>",
    );
    check_refused(
        Slot::Brightness,
        f32_under_vec3_root(),
        "the root, node 3, outputs vec3<f32>; the brightness slot takes f32",
    );
}

validation::negative_control!(
    occupant_typecheck_f32_root_in_colour_slot_fails,
    "a vec3 root in the colour slot type-checks, and must fail the refusal",
    expected = "the tree type-checked",
    check_refused(
        Slot::Colour,
        f32_under_vec3_root(),
        "the colour slot takes vec3<f32>"
    )
);

// ----- the other refusals -----

#[test]
fn occupant_typecheck_refuses_malformed_trees() {
    check_refused(
        Slot::Colour,
        Expr::new(1, T::Grey, vec![leaf(1, T::Lightness(0.5))]),
        "node 1: the id is used twice in the tree",
    );
    check_refused(
        Slot::Colour,
        Expr::new(0, T::Grey, vec![]),
        "node 0 takes 1 input(s); 0 given",
    );
    check_refused(
        Slot::Colour,
        Expr::new(1, T::Grey, vec![leaf(0, T::Lch(0.7, 0.1, 0.0))]),
        "node 1's input 0 takes f32; node 0 outputs vec3<f32>",
    );
}

validation::negative_control!(
    occupant_typecheck_refuses_malformed_trees,
    "a well-formed tree must fail the refusal",
    expected = "the tree type-checked",
    check_refused(
        Slot::Colour,
        Expr::new(1, T::Grey, vec![leaf(0, T::Lightness(0.5))]),
        "the id is used twice"
    )
);

// ----- a checked tree lowers to one function per node -----

fn check_lowered(root: Expr<T>, ids: &[u32]) {
    let tree = Tree::new(Slot::Colour, root).unwrap_or_else(|e| panic!("{e}"));
    let generated = tree.generate().unwrap_or_else(|e| panic!("{e}"));
    let got: Vec<u32> = generated.functions.iter().map(|f| f.id).collect();
    assert_eq!(got, ids, "the functions are not one per node in post-order");
    for f in &generated.functions {
        assert_eq!(f.name, codegen::function_name(f.id));
        assert!(generated.source.contains(&f.text));
    }
    assert!(generated
        .source
        .contains("fn colour(ctx: Ctx) -> vec3<f32> {\n    return node_3(ctx);\n}"));
}

#[test]
fn occupant_typecheck_lowers_to_one_function_per_node() {
    check_lowered(f32_under_vec3_root(), &[0, 1, 2, 3]);
}

validation::negative_control!(
    occupant_typecheck_lowers_to_one_function_per_node,
    "the pre-order is not the emission order",
    expected = "the functions are not one per node in post-order",
    check_lowered(f32_under_vec3_root(), &[3, 0, 2, 1])
);
