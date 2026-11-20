//! Link swap, the provenance as `SimConfig` records it (REQ-CHART-033; R-483: the provenance's links are `SimConfig`'s
//! `Links`, ids and parameter values): a swap of a block's link changes `Links` in that block and no other, each block
//! naming its selection's registry entry, and changes the canonical text every export carries. The kernel half, T2
//! through the registry inverses and the two directions swapped together, is `crates/kernel/tests/link_swap.rs`
//! (`cargo test -p kernel link_swap`); this half lives here because `kernel` takes no dependency on `engine`
//! (systems_architecture §7, R-185, R-187).

use engine::contract::canonical;
use engine::contract::sim_config::Links;
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidBeta, SigmoidQ, SoftmaxTanh, SoftsignAlpha,
    StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};
use validation::negative_control;

/// The defaults with `α`'s link swapped for `tanh_alpha`.
type AlphaSwapped =
    Select<SoftmaxTanh, TanhAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// Every block swapped.
type AllSwapped = Select<StickBreaking, SoftsignAlpha, TanhBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

/// `S`'s links name, per block, its entries: mass, `α`, `β` and the momenta (the first momentum control's, which the
/// four share).
fn check_names<S: Selection>(links: &Links) {
    let want = [
        <S::MassLink as Link<2, 3>>::NAME,
        <S::AlphaLink as Link<1, 1>>::NAME,
        <S::BetaLink as Link<1, 1>>::NAME,
        <S::Q0Link as Link<1, 1>>::NAME,
    ];
    let got = links.blocks().map(|(_, c)| c.id.clone());
    assert_eq!(got, want, "the Links do not name the selection's entries");
}

/// The swap from `A` to `B` changes `Links` in exactly `blocks`, and its canonical text.
fn check_swap<A: Selection, B: Selection>(blocks: &[&str]) {
    let (a, b) = (
        Links::of::<A>().expect("one momenta link"),
        Links::of::<B>().expect("one momenta link"),
    );
    check_names::<A>(&a);
    check_names::<B>(&b);
    let changed: Vec<&str> = a
        .blocks()
        .into_iter()
        .zip(b.blocks())
        .filter(|((_, x), (_, y))| x != y)
        .map(|((name, _), _)| name)
        .collect();
    assert_eq!(
        changed, blocks,
        "the swap did not change the provenance in exactly its blocks"
    );
    assert_ne!(
        canonical::to_string(&a).expect("a text"),
        canonical::to_string(&b).expect("a text"),
        "the swap did not change the provenance's text"
    );
}

#[test]
fn link_swap_changes_the_links() {
    check_swap::<Defaults, AlphaSwapped>(&["alpha"]);
    check_swap::<AlphaSwapped, Defaults>(&["alpha"]);
    check_swap::<Defaults, AllSwapped>(&["mass", "alpha", "beta", "momenta"]);
}

negative_control!(
    link_swap_changes_the_links,
    "a swap to the same links changes no provenance",
    expected = "the swap did not change the provenance",
    check_swap::<Defaults, Defaults>(&["alpha"])
);
