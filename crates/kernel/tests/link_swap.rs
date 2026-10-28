//! Link swap (REQ-CHART-033; dd_generation_root §4, seam 2/8, and §2; dd_decoder §4, seams 2, 8 and 13; dd_encode §2
//! and §4, seam 2/13; R-113: the sim key's change and the re-integration are REQ-CHART-052's, TASK-M4-08): decode and
//! encode consume only the registry's link functions and inverses, the provenance records each block's link, and a
//! link swap swaps both directions together.
//!
//! - `link_swap_t2_registry_inverses` (a property under `validation::prop`, `CASES` = 256, REQ-VAL-151): under the
//!   default selection and two others, T2 holds in physical units, `‖x − D(E(x))‖_phys ≤ ε_phys` at `x = D(z)`, and
//!   every control encode returns is its registry entry's own inverse of the intermediate it read, bit for bit.
//! - `link_swap_swaps_both_directions`: swapping one block's link (α's `sigmoid_alpha` for `tanh_alpha`, and every
//!   block at once) changes the provenance in that block and no other, as the kernel records it, the selection's link
//!   ids (`Selection::LINK_IDS`, each its entry's `Link::NAME`); the encode's control of that block changes and no
//!   other, bit for bit; T2 holds after the swap; and a swap of one direction only, encode under the old link and
//!   decode under the new, misses `ε_phys`. The provenance as `SimConfig`'s `Links` records it (R-483) and its
//!   canonical text is the other half, `crates/validation/tests/link_swap.rs`: `kernel` takes no dependency on `engine`
//!   (systems_architecture §7, R-185, R-187).
//!
//! The samples are latent `z` with every control in `[−1.5, 1.5]`, clear of every link's clamp; the box is test data,
//! not a register constant (REQ-SYS-082).

use kernel::decode::{decode, Ic, Latent};
use kernel::encode::{encode, Encoded, SimplexClamp};
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidBeta, SigmoidQ, SoftmaxTanh, SoftsignAlpha,
    StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};
use proptest::prelude::*;
use validation::negative_control;
use validation::prop::run;
use validation::roundtrip::{eps_phys, phys, trip_with};

/// The defaults with `α`'s link swapped for `tanh_alpha`.
type AlphaSwapped =
    Select<SoftmaxTanh, TanhAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;

/// Every block swapped: the edge-reaching simplex, softsign for `α`, tanh for `β` and the momenta.
type AllSwapped = Select<StickBreaking, SoftsignAlpha, TanhBeta, TanhQ, TanhQ, TanhQ, TanhQ>;

/// The latent box the samples come from.
fn latents() -> impl Strategy<Value = [f64; 8]> {
    prop::array::uniform8(-1.5f64..1.5)
}

fn bits<const N: usize>(v: [f64; N]) -> [u64; N] {
    v.map(f64::to_bits)
}

/// Encode's controls are the registry entries' own inverses of the intermediates it read, bit for bit: `α`, `β`, the
/// four momenta and the masses each through `S`'s link for its block.
fn check_registry<S>(e: &Encoded<f64>)
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let b = &e.blocks;
    let q = [
        b.jacobi.p_rho[0],
        b.jacobi.p_rho[1],
        b.jacobi.p_lambda[0],
        b.jacobi.p_lambda[1],
    ];
    let [mu1, mu2] = <S::MassLink as Link<2, 3>>::inverse(e.canonical.ic.m);
    let want = [
        <S::AlphaLink as Link<1, 1>>::inverse([b.alpha])[0],
        <S::BetaLink as Link<1, 1>>::inverse([b.beta])[0],
        <S::Q0Link as Link<1, 1>>::inverse([q[0]])[0],
        <S::Q1Link as Link<1, 1>>::inverse([q[1]])[0],
        <S::Q2Link as Link<1, 1>>::inverse([q[2]])[0],
        <S::Q3Link as Link<1, 1>>::inverse([q[3]])[0],
        mu1,
        mu2,
    ];
    assert_eq!(
        bits(e.z.z),
        bits(want),
        "encode's controls are not the registry inverses of {:?}",
        S::LINK_IDS
    );
}

/// T2 at `x = D_S(z)` under `S`: `E_S` reads the registry inverses ([`check_registry`]), and `D_S(E_S(x))` is within
/// `ε_phys` of `x`. `Err` with the residual where it is not.
fn check_t2<S>(z: [f64; 8]) -> Result<(), TestCaseError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let x: Ic<f64> = decode::<S, f64>(&Latent::new(z)).ic;
    let e = encode::<S, f64>(&x);
    prop_assert!(
        !e.report.lookup_clamped(),
        "a sample in the box clamped under {:?}",
        S::LINK_IDS
    );
    check_registry::<S>(&e);
    let back = decode::<S, f64>(&e.z).ic;
    let r = phys(&x, &back);
    prop_assert!(
        r <= eps_phys::<f64>(),
        "T2 misses under {:?}: residual {r:e} at z = {z:?}",
        S::LINK_IDS
    );
    Ok(())
}

#[test]
fn link_swap_t2_registry_inverses() {
    run(&latents(), |z| {
        check_t2::<Defaults>(z)?;
        check_t2::<AlphaSwapped>(z)?;
        check_t2::<AllSwapped>(z)
    });
}

negative_control!(
    link_swap_t2_registry_inverses,
    "an encode whose α control is read through the default link while the selection's is tanh_alpha is not the \
     registry's inverse",
    expected = "encode's controls are not the registry inverses",
    {
        let x = decode::<AlphaSwapped, f64>(&Latent::new([0.7, -0.4, 0.2, 0.1, -0.3, 0.5, 0.4, -0.2])).ic;
        check_registry::<AlphaSwapped>(&encode::<Defaults, f64>(&x))
    }
);

/// The selection's link ids, each checked to be its control's entry's `Link::NAME`.
fn link_ids<S: Selection>() -> [&'static str; 7] {
    let names = [
        <S::MassLink as Link<2, 3>>::NAME,
        <S::AlphaLink as Link<1, 1>>::NAME,
        <S::BetaLink as Link<1, 1>>::NAME,
        <S::Q0Link as Link<1, 1>>::NAME,
        <S::Q1Link as Link<1, 1>>::NAME,
        <S::Q2Link as Link<1, 1>>::NAME,
        <S::Q3Link as Link<1, 1>>::NAME,
    ];
    assert_eq!(
        S::LINK_IDS,
        names,
        "the link ids are not the entries' names"
    );
    S::LINK_IDS
}

/// The swap from `A` to `B` (`slots` the selection's controls it changes, in `LINK_IDS`' order: mass, `α`, `β`, `q0` …
/// `q3`; `controls` the latent controls they set): the provenance, the selection's link ids, changes in those slots
/// and no other, encode's controls change there
/// and no other at a sample where the two links differ, T2 holds under `B`, and a swap of the encode alone (encode under
/// `A`, decode under `B`) misses `ε_phys`.
fn check_swap<A, B>(slots: &[usize], controls: &[usize])
where
    A: Selection,
    A::MassLink: SimplexClamp,
    B: Selection,
    B::MassLink: SimplexClamp,
{
    let (a, b) = (link_ids::<A>(), link_ids::<B>());
    let changed: Vec<usize> = (0..7).filter(|&k| a[k] != b[k]).collect();
    assert_eq!(
        changed, slots,
        "the swap did not change the provenance in exactly its slots"
    );
    let z = [0.7, -0.4, 0.2, 0.1, -0.3, 0.5, 0.4, -0.2];
    let x = decode::<B, f64>(&Latent::new(z)).ic;
    let (ea, eb) = (encode::<A, f64>(&x), encode::<B, f64>(&x));
    let moved: Vec<usize> = (0..8).filter(|&k| ea.z.z[k] != eb.z.z[k]).collect();
    assert_eq!(
        moved, controls,
        "the swap did not move encode's controls in exactly its blocks"
    );
    check_t2::<B>(z).expect("T2 after the swap");
    let half = trip_with::<A, B, f64>(&x).residual;
    assert!(
        half > eps_phys::<f64>(),
        "a swap of the encode alone round-trips ({half:e}): the directions are not tied"
    );
}

#[test]
fn link_swap_swaps_both_directions() {
    check_swap::<Defaults, AlphaSwapped>(&[1], &[0]);
    check_swap::<AlphaSwapped, Defaults>(&[1], &[0]);
    check_swap::<Defaults, AllSwapped>(&[0, 1, 2, 3, 4, 5, 6], &[0, 1, 2, 3, 4, 5, 6, 7]);
}

negative_control!(
    link_swap_swaps_both_directions,
    "a swap to the same links changes no provenance",
    expected = "the swap did not change the provenance",
    check_swap::<Defaults, Defaults>(&[1], &[0])
);
