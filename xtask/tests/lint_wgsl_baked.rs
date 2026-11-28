//! A baked uniform has no member in the block the lint declares for a node (`xtask::lint_wgsl::uniform_block`): the
//! assembler writes its value into the code, as it does a `field`. Pass criterion: for a source declaring one baked
//! uniform and one plain, the block holds the plain one and not the baked one. Control: the same source with the
//! `baked` word taken off puts it in the block.

use validation::negative_control;
use xtask::lint_wgsl::uniform_block;

fn check_block(baked: &str) {
    let source = format!(
        "// @uniform curve: u32 = 1 [0, 4]{baked}\n// @uniform gain: f32 = 0.5 [0, 1]\nfn colour(ctx: Ctx) -> vec3<f32> {{ return vec3<f32>(uniforms.gain); }}\n"
    );
    let block = uniform_block(&source).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        block.contains("gain: f32"),
        "the plain uniform is in the block: {block}"
    );
    assert!(
        !block.contains("curve"),
        "a baked uniform is in the block: {block}"
    );
}

#[test]
fn a_baked_uniform_is_not_in_the_block() {
    check_block(" baked");
}

negative_control!(
    a_baked_uniform_is_not_in_the_block,
    "the same uniform without the `baked` word is in the block",
    expected = "a baked uniform is in the block",
    check_block("")
);
