//! QA tests for TASK-M2-21, REQ-VAL-024 (validation ground-truth note § "Link variation as a conditioning diagnostic
//! (not a default change)"; inverse-encode contract Part 7), written from the requirement: for each registry link the
//! diagnostic inverts the canonicalised IC and reports that link's residual and conditioning; a per-validation link
//! choice is recorded in provenance; validation never changes the default link. Each test registers its negative
//! control (R-176). The requirement's verify method is the physics reviewer's checklist; these tests back it.
//!
//! - `qa_link_variation_reports_each_links_own`: at a moving IC in a foreign frame (nonzero momenta, so the momentum
//!   rows are exercised, which an IC at rest cannot do: every momentum link's inverse of 0 is 0), every row's
//!   `residual_L` is `‖D_L(z_L) − C(x)‖_phys` and its `κ_L` is `κ` under that row's own selection, recomputed here bit
//!   for bit; `z_L` is `E_L(x)`; the rows of different links on one block give different `z_L` in that block's controls;
//!   and inside the simplex every row reaches the IC within `ε_phys`.
//! - `qa_link_variation_provenance_and_default`: each row's links name its own link on its block and the defaults
//!   elsewhere, pass the registry's check and travel through the ViewState record unchanged (the per-validation choice
//!   recorded in provenance); the diagnostic is pure (two runs agree bitwise) and leaves `Links::defaults()` and the
//!   default selection's link ids as they were, which are the registry slots' defaults.
//!
//! The ICs are test data (REQ-SYS-082).

use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, Window,
};
use engine::contract::view_state::{export, restore};
use kernel::decode::{decode, Ic};
use kernel::encode::ingest::kappa;
use kernel::encode::{encode, SimplexClamp};
use kernel::generated::links::{
    Defaults, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ, SoftmaxTanh, SoftsignAlpha,
    SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};
use validation::link_variation::{diagnose, Row};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

type WMass<M> = Select<M, SigmoidAlpha, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WAlpha<A> = Select<SoftmaxTanh, A, SigmoidBeta, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WBeta<B> = Select<SoftmaxTanh, SigmoidAlpha, B, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>;
type WMom<Q> = Select<SoftmaxTanh, SigmoidAlpha, SigmoidBeta, Q, Q, Q, Q>;

/// A moving scalene triangle, masses `(0.2, 0.35, 0.45)`, rotated and offset, momenta not summing to zero (`C` removes
/// the drift): a foreign frame.
fn moving() -> Ic<f64> {
    Ic {
        m: [0.2, 0.35, 0.45],
        r: [[2.1, -0.4], [-0.7, 1.3], [0.9, -1.6]],
        p: [[0.31, -0.12], [-0.08, 0.27], [0.15, 0.05]],
    }
}

/// `(z, residual, κ)` of `ic` under `S`, from encode, decode and `κ` directly.
fn own<S>(ic: &Ic<f64>) -> ([u64; 8], u64, u64)
where
    S: Selection,
    S::MassLink: SimplexClamp,
{
    let e = encode::<S, f64>(ic);
    let r = phys(&e.canonical.ic, &decode::<S, f64>(&e.z).ic);
    (
        e.z.z.map(f64::to_bits),
        r.to_bits(),
        kappa::<S, f64>(ic).to_bits(),
    )
}

/// The row's own `(z, residual, κ)`, by its block and link.
fn expected(block: &str, link: &str, ic: &Ic<f64>) -> ([u64; 8], u64, u64) {
    match (block, link) {
        ("mass", "softmax_tanh") => own::<WMass<SoftmaxTanh>>(ic),
        ("mass", "stick_breaking") => own::<WMass<StickBreaking>>(ic),
        ("alpha", "sigmoid_alpha") => own::<WAlpha<SigmoidAlpha>>(ic),
        ("alpha", "tanh_alpha") => own::<WAlpha<TanhAlpha>>(ic),
        ("alpha", "softsign_alpha") => own::<WAlpha<SoftsignAlpha>>(ic),
        ("beta", "sigmoid_beta") => own::<WBeta<SigmoidBeta>>(ic),
        ("beta", "tanh_beta") => own::<WBeta<TanhBeta>>(ic),
        ("beta", "softsign_beta") => own::<WBeta<SoftsignBeta>>(ic),
        ("momenta", "sigmoid_q") => own::<WMom<SigmoidQ>>(ic),
        ("momenta", "tanh_q") => own::<WMom<TanhQ>>(ic),
        ("momenta", "softsign_q") => own::<WMom<SoftsignQ>>(ic),
        other => panic!("a row on {other:?}, which no registry entry gives"),
    }
}

fn controls(block: &str) -> &'static [usize] {
    match block {
        "mass" => &[6, 7],
        "alpha" => &[0],
        "beta" => &[1],
        "momenta" => &[2, 3, 4, 5],
        other => panic!("no block {other}"),
    }
}

fn check_own(rows: &[Row], ic: &Ic<f64>) {
    assert_eq!(
        rows.len(),
        11,
        "the rows are not the registry's 11 block links"
    );
    for r in rows {
        let (z, res, k) = expected(r.block, r.link(), ic);
        assert_eq!(
            (
                r.z.map(f64::to_bits),
                r.residual.to_bits(),
                r.kappa.to_bits()
            ),
            (z, res, k),
            "the row of {} on {} does not report its own link's z, residual and κ",
            r.link(),
            r.block
        );
        assert!(
            !r.clamped && r.residual <= eps_phys::<f64>() && r.kappa.is_finite() && r.kappa > 0.0,
            "the row of {} on {} misses the interior IC: clamped {}, residual {:e}, κ {:e}",
            r.link(),
            r.block,
            r.clamped,
            r.residual,
            r.kappa
        );
    }
    // Different links on one block place the IC at different controls of that block.
    for block in ["mass", "alpha", "beta", "momenta"] {
        let on: Vec<&Row> = rows.iter().filter(|r| r.block == block).collect();
        for (i, a) in on.iter().enumerate() {
            for b in &on[i + 1..] {
                assert!(
                    controls(block)
                        .iter()
                        .any(|&k| a.z[k].to_bits() != b.z[k].to_bits()),
                    "{} and {} place the IC at the same {block} controls",
                    a.link(),
                    b.link()
                );
            }
        }
    }
}

#[test]
fn qa_link_variation_reports_each_links_own() {
    let ic = moving();
    check_own(&diagnose(&ic), &ic);
}

negative_control!(
    qa_link_variation_reports_each_links_own,
    "rows whose results are swapped between two links of one block do not report their own link's",
    expected = "does not report its own link's",
    {
        let ic = moving();
        let mut rows = diagnose(&ic);
        let (i, j) = (
            rows.iter().position(|r| r.link() == "tanh_q").unwrap(),
            rows.iter().position(|r| r.link() == "softsign_q").unwrap(),
        );
        let (zi, ri, ki) = (rows[i].z, rows[i].residual, rows[i].kappa);
        rows[i].z = rows[j].z;
        rows[i].residual = rows[j].residual;
        rows[i].kappa = rows[j].kappa;
        rows[j].z = zi;
        rows[j].residual = ri;
        rows[j].kappa = ki;
        check_own(&rows, &ic)
    }
);

fn view(links: Links) -> SimConfig {
    SimConfig {
        chart: Chart {
            window: Window::SQUARE,
        },
        plane: Plane {
            z0: [0.0; 8],
            q1: [0.0; 8],
            q2: [0.0; 8],
        },
        slice: Slice {},
        lock: Lock {
            locked: false,
            z_locked: [0.0; 8],
        },
        links,
        integrator: Integrator {},
        kernel_variant: KernelVariant::Physics,
        horizon: Horizon { t: 100.0 },
        collision: Collision {},
        quality: Quality {},
    }
}

fn check_provenance(rows: &[Row]) {
    let defaults = Links::defaults();
    for r in rows {
        r.links.check().unwrap_or_else(|e| {
            panic!(
                "the row of {} records links the registry refuses: {e}",
                r.link()
            )
        });
        for ((name, got), (_, d)) in r.links.blocks().into_iter().zip(defaults.blocks()) {
            match name == r.block {
                true => assert!(
                    got.id == r.link(),
                    "the row's provenance does not record its link on {name}"
                ),
                false => assert!(
                    got == d,
                    "the row of {} on {} records {} on {name}, not the default",
                    r.link(),
                    r.block,
                    got.id
                ),
            }
        }
        let record = export(&view(r.links.clone())).expect("a record");
        let back = restore(&record).expect("the row's provenance restores");
        assert!(
            back.links == r.links,
            "the row of {} on {} does not travel through the record",
            r.link(),
            r.block
        );
    }
}

#[test]
fn qa_link_variation_provenance_and_default() {
    let defaults = Links::defaults();
    let ids = Defaults::LINK_IDS;
    let slots = ledger::links::slots();
    for (s, id) in slots.iter().zip(ids) {
        assert_eq!(
            s.default, id,
            "the default selection is not the registry's slot defaults"
        );
    }
    let ic = moving();
    let rows = diagnose(&ic);
    check_provenance(&rows);
    let again = diagnose(&ic);
    assert!(rows == again, "the diagnostic is not pure: two runs differ");
    assert_eq!(
        Links::defaults(),
        defaults,
        "the diagnostic changed the default links"
    );
    assert_eq!(
        Defaults::LINK_IDS,
        ids,
        "the diagnostic changed the default selection"
    );
    assert!(
        rows.iter().filter(|r| r.links == defaults).count() == 4,
        "the defaults' row is not one per block"
    );
}

negative_control!(
    qa_link_variation_provenance_and_default,
    "a row on α recording a swapped β link as well does not record the defaults elsewhere",
    expected = "not the default",
    {
        let mut rows = diagnose(&moving());
        let i = rows.iter().position(|r| r.link() == "tanh_alpha").unwrap();
        let j = rows.iter().position(|r| r.link() == "tanh_beta").unwrap();
        rows[i].links.beta = rows[j].links.beta.clone();
        check_provenance(&rows)
    }
);
