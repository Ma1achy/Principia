//! What the editor reads to say what a node needs (`describe::needs`) and what a run lacks (`Needs::lacks`): derived
//! from the node's source, for a shipped node and a custom alike. Its control arm hands the proof a source whose
//! only read of the FTLE decides a branch, with branches forgotten.

use engine::contract::stain::describe::needs;
use engine::contract::stain::graph::{NodeKind, Occupant};
use render::assemble::matters::Lack;
use render::assemble::Tier;
use validation::negative_control;

const FTLE_BRANCH: &str = "fn colour(ctx: Ctx) -> vec3<f32> {
    if (ctx.sample.ftle > 0.0) { return vec3<f32>(1.0); }
    return vec3<f32>(ctx.sample.ensemble_spread);
}";

const NOTHING: &str = "fn colour(ctx: Ctx) -> vec3<f32> { return vec3<f32>(0.5); }";

const NO_FTLE: Tier = Tier {
    has_ftle: false,
    has_word: true,
};

fn lacks(text: &str, tier: Tier, ensemble: bool) -> Vec<Lack> {
    needs(NodeKind::Colour, &Occupant::Custom(text.into()))
        .unwrap_or_else(|e| panic!("{e}"))
        .lacks(tier, ensemble)
}

fn check() {
    // Pass criterion, written before the first run: a custom whose source reads the FTLE only to choose a branch and the
    // ensemble's spread lacks the FTLE on a tier without it and the spread without an ensemble, and nothing on the full
    // tier with one; a shipped view of the FTLE lacks it the same way, whichever the occupant; one that reads
    // nothing lacks nothing anywhere; and the identity is pure.
    assert_eq!(
        lacks(FTLE_BRANCH, NO_FTLE, false),
        vec![Lack::Ftle, Lack::Spread],
        "what a custom that reads the FTLE lacks without it"
    );
    assert_eq!(
        lacks(FTLE_BRANCH, Tier::FULL, true),
        vec![],
        "nothing lacks on the full tier"
    );
    assert_eq!(
        lacks(NOTHING, NO_FTLE, false),
        vec![],
        "a node that reads nothing lacks nothing"
    );
    let shipped = needs(
        NodeKind::Colour,
        &Occupant::Builtin("debug/generated/ftle".into()),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        shipped.lacks(NO_FTLE, true),
        vec![Lack::Ftle],
        "what the shipped FTLE view lacks"
    );
    assert!(
        needs(NodeKind::Colour, &Occupant::None)
            .unwrap_or_else(|e| panic!("{e}"))
            .pure_in_n,
        "the identity is pure"
    );
}

#[test]
fn the_editor_says_what_a_run_lacks() {
    check();
}

negative_control!(
    the_editor_says_what_a_run_lacks,
    "a proof that forgets branches does not find the FTLE that only decides a branch",
    expected = "what a custom that reads the FTLE lacks without it",
    render::assemble::matters::probe::forgetting(
        render::assemble::matters::Forget::Branches,
        check
    )
);
