//! QA tests for TASK-M2-21, REQ-CHART-033 (dd_generation_root §2 and §4; dd_decoder §4; dd_encode §2 and §4; R-113),
//! written from the requirement: decode and encode consume only the registry's links and inverses, the link ids are
//! recorded in provenance (`SimConfig`'s `Links`, R-483), and a link swap swaps both directions. Each test registers
//! its negative control (R-176).
//!
//! Every selection the registry offers one block at a time is exercised: the default selection and, per block, each
//! other entry onto that block's codomain (two mass links, three each for `α`, `β` and the momenta: 7 swaps).
//!
//! - `qa_link_swap_t2_every_registry_link` (property, `validation::prop`, `CASES`): under each selection, at
//!   `x = D_S(z)`, encode's eight controls are, bit for bit, the selection's registry inverses of the intermediates it
//!   read, and T2 holds in physical units, `‖x − D_S(E_S(x))‖_phys ≤ ε_phys` (REQ-VAL-151's `ε_phys`).
//! - `qa_link_swap_provenance_and_directions`: for each swap from the defaults, the provenance (`Links::of`) records the
//!   selection's own link names (`Link::NAME`), passes the registry's check (each id an entry on its block's codomain),
//!   and differs from the defaults' in that block alone; encode's controls move in that block's controls alone; and a
//!   swap of one direction only, in either order (encode under one selection, decode under the other), misses
//!   `ε_phys`, while the swap of both holds T2.
//!
//! It lives in `validation`'s tests, which may use `kernel` and `engine` both: `kernel` may not depend on `engine`,
//! not even as a dev-dependency (systems_architecture §7, R-187; `cargo xtask deps`).
//!
//! The latent box `[−1.5, 1.5]⁸` keeps every control clear of every link's clamp; it is test data (REQ-SYS-082).

use engine::contract::sim_config::Links;
use kernel::decode::{decode, Latent};
use kernel::encode::{encode, Encoded, SimplexClamp};
use kernel::generated::links::{
    Defaults, Link, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh,
    SoftsignAlpha, SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};
use proptest::prelude::*;
use validation::negative_control;
use validation::prop::run;
use validation::roundtrip::{eps_phys, phys};

type WMass<M> = Select<M, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WAlpha<A> = Select<SoftmaxTanh, A, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WBeta<B> = Select<SoftmaxTanh, SigmoidAlpha, B, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WMom<Q> = Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, Q, Q, Q, Q>;

fn bits(v: [f64; 8]) -> [u64; 8] {
    v.map(f64::to_bits)
}

/// The registry inverses of the intermediates encode read, under `S`: `α`, `β`, the four Jacobi momenta, the masses.
fn registry_inverses<S: Selection>(e: &Encoded<f64>) -> [f64; 8] {
    let b = &e.blocks;
    let [mu1, mu2] = <S::MassLink as Link<2, 3>>::inverse(e.canonical.ic.m);
    [
        <S::AlphaLink as Link<1, 1>>::inverse([b.alpha])[0],
        <S::BetaLink as Link<1, 1>>::inverse([b.beta])[0],
        <S::Q0Link as Link<1, 1>>::inverse([b.jacobi.p_rho[0]])[0],
        <S::Q1Link as Link<1, 1>>::inverse([b.jacobi.p_rho[1]])[0],
        <S::Q2Link as Link<1, 1>>::inverse([b.jacobi.p_lambda[0]])[0],
        <S::Q3Link as Link<1, 1>>::inverse([b.jacobi.p_lambda[1]])[0],
        mu1,
        mu2,
    ]
}

fn t2<S>(z: [f64; 8]) -> Result<(), TestCaseError>
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let x = decode::<S, f64>(&Latent::new(z)).ic;
    let e = encode::<S, f64>(&x);
    prop_assert!(
        !e.report.lookup_clamped(),
        "clamped under {:?} at {z:?}",
        S::LINK_IDS
    );
    prop_assert_eq!(
        bits(e.z.z),
        bits(registry_inverses::<S>(&e)),
        "encode's controls are not the registry inverses under {:?}",
        S::LINK_IDS
    );
    let r = phys(&x, &decode::<S, f64>(&e.z).ic);
    prop_assert!(
        r <= eps_phys::<f64>(),
        "T2 misses under {:?}: {r:e} at {z:?}",
        S::LINK_IDS
    );
    Ok(())
}

#[test]
fn qa_link_swap_t2_every_registry_link() {
    run(&prop::array::uniform8(-1.5f64..1.5), |z| {
        t2::<Defaults>(z)?;
        t2::<WMass<StickBreaking>>(z)?;
        t2::<WAlpha<TanhAlpha>>(z)?;
        t2::<WAlpha<SoftsignAlpha>>(z)?;
        t2::<WBeta<TanhBeta>>(z)?;
        t2::<WBeta<SoftsignBeta>>(z)?;
        t2::<WMom<TanhQ>>(z)?;
        t2::<WMom<SoftsignQ>>(z)
    });
}

negative_control!(
    qa_link_swap_t2_every_registry_link,
    "controls read through softsign while the selection's β link is tanh are not its registry inverses",
    expected = "encode's controls are not the registry inverses",
    {
        let z = [0.3, -0.9, 0.4, -0.2, 0.6, -0.5, 0.2, 0.7];
        let x = decode::<WBeta<TanhBeta>, f64>(&Latent::new(z)).ic;
        let e = encode::<WBeta<SoftsignBeta>, f64>(&x);
        assert_eq!(
            bits(e.z.z),
            bits(registry_inverses::<WBeta<TanhBeta>>(&e)),
            "encode's controls are not the registry inverses"
        );
    }
);

/// The swap from the defaults to `B`, which changes the block `block` (its latent controls `controls`).
fn check_swap<B>(block: &str, controls: &[usize])
where
    B: Selection,
    B::MassLink: SimplexClamp,
{
    let (a, b) = (
        Links::of::<Defaults>().expect("links"),
        Links::of::<B>().expect("links"),
    );
    b.check().unwrap_or_else(|e| {
        panic!(
            "the provenance of {:?} is not the registry's: {e}",
            B::LINK_IDS
        )
    });
    let names = [
        <B::MassLink as Link<2, 3>>::NAME,
        <B::AlphaLink as Link<1, 1>>::NAME,
        <B::BetaLink as Link<1, 1>>::NAME,
        <B::Q0Link as Link<1, 1>>::NAME,
    ];
    let recorded = [&b.mass.id, &b.alpha.id, &b.beta.id, &b.momenta.id].map(|s| s.as_str());
    assert_eq!(
        recorded, names,
        "the provenance does not record the selection's links"
    );
    let changed: Vec<&str> = a
        .blocks()
        .into_iter()
        .zip(b.blocks())
        .filter(|((_, x), (_, y))| x != y)
        .map(|((n, _), _)| n)
        .collect();
    assert_eq!(
        changed,
        [block],
        "the swap's provenance changed {changed:?}"
    );
    let z = [0.45, -0.7, 0.9, -0.35, 0.55, -1.1, 0.25, -0.6];
    let x = decode::<B, f64>(&Latent::new(z)).ic;
    let (ea, eb) = (encode::<Defaults, f64>(&x), encode::<B, f64>(&x));
    let moved: Vec<usize> = (0..8)
        .filter(|&k| ea.z.z[k].to_bits() != eb.z.z[k].to_bits())
        .collect();
    assert_eq!(
        moved, controls,
        "the swap moved encode's controls {moved:?}"
    );
    let both = phys(&x, &decode::<B, f64>(&eb.z).ic);
    assert!(
        both <= eps_phys::<f64>(),
        "the swap of both directions misses T2: {both:e}"
    );
    let encode_only = phys(&x, &decode::<B, f64>(&ea.z).ic);
    assert!(
        encode_only > eps_phys::<f64>(),
        "a swap of the decode alone round-trips ({encode_only:e}): the directions are not tied"
    );
    let xa = decode::<Defaults, f64>(&Latent::new(z)).ic;
    let decode_only = phys(&xa, &decode::<Defaults, f64>(&encode::<B, f64>(&xa).z).ic);
    assert!(
        decode_only > eps_phys::<f64>(),
        "a swap of the encode alone round-trips ({decode_only:e}): the directions are not tied"
    );
}

#[test]
fn qa_link_swap_provenance_and_directions() {
    check_swap::<WMass<StickBreaking>>("mass", &[6, 7]);
    check_swap::<WAlpha<TanhAlpha>>("alpha", &[0]);
    check_swap::<WAlpha<SoftsignAlpha>>("alpha", &[0]);
    check_swap::<WBeta<TanhBeta>>("beta", &[1]);
    check_swap::<WBeta<SoftsignBeta>>("beta", &[1]);
    check_swap::<WMom<TanhQ>>("momenta", &[2, 3, 4, 5]);
    check_swap::<WMom<SoftsignQ>>("momenta", &[2, 3, 4, 5]);
}

negative_control!(
    qa_link_swap_provenance_and_directions,
    "a swap to the defaults' own links changes no provenance",
    expected = "the swap's provenance changed",
    check_swap::<WAlpha<SigmoidAlpha>>("alpha", &[0])
);
