//! QA tests for TASK-M2-21, REQ-VAL-024 (validation ground-truth note § "Link variation as a conditioning diagnostic
//! (not a default change)"; inverse-encode contract Part 7), written from the requirement: for each registry link the
//! diagnostic inverts the canonicalised IC and reports that link's residual and conditioning; a per-validation link
//! choice is recorded in provenance; validation never changes the default link. "Each registry link" is each selection
//! of the registry's entries across the blocks, the product of every block's entries (physics review 5678259443 of
//! PR #308, P1). Each test registers its negative control (R-176). The requirement's verify method is the physics
//! reviewer's checklist; these tests back it.
//!
//! - `qa_link_variation_covers_every_selection`: the rows' link ids are exactly the product, over the four blocks, of
//!   the registry entries onto each block's slot codomain (`ledger::links`), each selection once; one row is the
//!   default selection.
//! - `qa_link_variation_reports_each_selections_own`: at a moving IC in a foreign frame (nonzero momenta, so every
//!   momentum link's inverse is exercised; at rest each gives 0) and at a moving IC near the simplex's edge (where some
//!   selections clamp), every row's `z_L` is `E_L(x)`, its `residual_L` is `‖D_L(z_L) − C(x)‖_phys`, its `κ_L` is
//!   `κ` and its clamp flag is the encode's, each under that row's own selection as its links name it, recomputed here
//!   bit for bit; rows whose selections differ in exactly one block place the IC at different controls of that block;
//!   and at the interior IC every row reaches the IC unclamped within `ε_phys` (T2).
//! - `qa_link_variation_provenance_and_default`: each row's links pass the registry's check, carry each entry's
//!   tunable parameters at their defaults, and travel through the ViewState record unchanged (the per-validation choice
//!   recorded in provenance); the diagnostic is pure (two runs agree bitwise) and leaves `Links::defaults()` and the
//!   default selection's link ids as they were, which are the registry slots' defaults.
//!
//! The ICs are test data (REQ-SYS-082).

use std::collections::BTreeMap;

use engine::contract::sim_config::{
    Chart, Collision, Horizon, Integrator, KernelVariant, Links, Lock, Plane, Quality, SimConfig,
    Slice, Window,
};
use engine::contract::view_state::{export, restore};
use kernel::decode::{decode, Ic};
use kernel::encode::ingest::kappa;
use kernel::encode::{encode, SimplexClamp};
use kernel::generated::links::{
    Alpha, Beta, Defaults, Link, Mass, Select, Selection, SigmoidAlpha, SigmoidBeta, SigmoidQ,
    SoftmaxTanh, SoftsignAlpha, SoftsignBeta, SoftsignQ, StickBreaking, TanhAlpha, TanhBeta, TanhQ,
};
use validation::link_variation::{diagnose, Row};
use validation::negative_control;
use validation::roundtrip::{eps_phys, phys};

/// The blocks of `Links`, in `Links::blocks`' order, and the registry slot whose codomain each takes (the momenta the
/// first momentum control's, which the four share, R-479 E1).
const BLOCKS: [(&str, &str); 4] = [
    ("mass", "mass"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("momenta", "q0"),
];

/// The latent controls each block sets, in `BLOCKS`' order: mass `z[6:8]`, `α` `z[0]`, `β` `z[1]`, momenta `z[2:6]`.
const CONTROLS: [&[usize]; 4] = [&[6, 7], &[0], &[1], &[2, 3, 4, 5]];

/// A moving scalene triangle, masses `(0.2, 0.35, 0.45)`, rotated and offset, momenta not summing to zero (`C` removes
/// the drift): a foreign frame, inside the simplex.
fn moving() -> Ic<f64> {
    Ic {
        m: [0.2, 0.35, 0.45],
        r: [[2.1, -0.4], [-0.7, 1.3], [0.9, -1.6]],
        p: [[0.31, -0.12], [-0.08, 0.27], [0.15, 0.05]],
    }
}

/// A moving triangle with one mass `10⁻⁵`, near the simplex's edge (`m₂/m₀ ≈ 5.5·10⁴`, past `e^{2μ_max} ≈ 2.2·10⁴` at
/// `μ_max = 5`), with large momenta: the test asserts some selection clamps here.
fn edge() -> Ic<f64> {
    let tiny = 1e-5;
    Ic {
        m: [tiny, 0.45 - tiny / 2.0, 0.55 - tiny / 2.0],
        r: [[1.3, 0.8], [-0.6, -1.1], [0.4, 0.9]],
        p: [[-1.9, 1.4], [0.7, -2.3], [1.1, 0.6]],
    }
}

/// The registry entries onto `slot`'s codomain, as `ledger::links` lists them.
fn entries(slot: &str) -> Vec<&'static str> {
    let codomain = ledger::links::slots()
        .into_iter()
        .find(|s| s.name == slot)
        .unwrap_or_else(|| panic!("the registry has no slot {slot}"))
        .codomain;
    ledger::links::registered()
        .iter()
        .filter(|(_, c)| *c == codomain)
        .map(|(l, _)| l.name)
        .collect()
}

/// A row's link ids, per block.
fn ids(r: &Row) -> [String; 4] {
    r.links.blocks().map(|(_, c)| c.id.clone())
}

/// The rows are the product of the blocks' entries, each selection exactly once, the default selection among them.
fn check_covers(rows: &[Row]) {
    let mut want: Vec<[String; 4]> = Vec::new();
    for m in entries(BLOCKS[0].1) {
        for a in entries(BLOCKS[1].1) {
            for b in entries(BLOCKS[2].1) {
                for q in entries(BLOCKS[3].1) {
                    want.push([m, a, b, q].map(str::to_owned));
                }
            }
        }
    }
    // The momentum controls q1..q3 take q0's codomain, so one momenta link serves all four (R-479 E1).
    for q in ["q1", "q2", "q3"] {
        assert_eq!(
            entries(q),
            entries("q0"),
            "the momentum slots differ in codomain"
        );
    }
    let mut got: Vec<[String; 4]> = rows.iter().map(ids).collect();
    got.sort();
    let n = got.len();
    got.dedup();
    assert_eq!(n, got.len(), "a selection has two rows");
    want.sort();
    assert_eq!(
        got, want,
        "the rows are not every selection of the registry's entries"
    );
    let default: [String; 4] = BLOCKS.map(|(_, s)| {
        ledger::links::slots()
            .into_iter()
            .find(|x| x.name == s)
            .expect("a slot")
            .default
            .to_owned()
    });
    assert_eq!(
        rows.iter().filter(|r| ids(r) == default).count(),
        1,
        "the default selection is not one row"
    );
}

#[test]
fn qa_link_variation_covers_every_selection() {
    let rows = diagnose(&moving());
    check_covers(&rows);
    // Two mass entries and three each for α, β and the momenta in today's registry (dd_generation_root §3.9).
    assert_eq!(
        rows.len(),
        2 * 3 * 3 * 3,
        "the product is not 54 selections"
    );
}

negative_control!(
    qa_link_variation_covers_every_selection,
    "a diagnosis that repeats one selection in place of another does not cover every selection",
    expected = "a selection has two rows",
    {
        let mut rows = diagnose(&moving());
        let last = rows.len() - 1;
        rows[last] = rows[0].clone();
        check_covers(&rows)
    }
);

/// `(z, residual, κ, clamped)`, the first three as bits.
type Own = ([u64; 8], u64, u64, bool);

/// `ic` inverted under `S`, from encode, decode and `κ` directly.
fn own<S>(ic: &Ic<f64>) -> Own
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
        e.report.lookup_clamped(),
    )
}

fn by_q<M, A, B>(q: &str, ic: &Ic<f64>) -> Own
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
    A: Link<1, 1, Codomain = Alpha>,
    B: Link<1, 1, Codomain = Beta>,
{
    match q {
        "sigmoid_q" => own::<Select<M, A, B, SigmoidQ, SigmoidQ, SigmoidQ, SigmoidQ>>(ic),
        "tanh_q" => own::<Select<M, A, B, TanhQ, TanhQ, TanhQ, TanhQ>>(ic),
        "softsign_q" => own::<Select<M, A, B, SoftsignQ, SoftsignQ, SoftsignQ, SoftsignQ>>(ic),
        other => panic!("a momenta link {other} no registry entry gives"),
    }
}

fn by_b<M, A>(b: &str, q: &str, ic: &Ic<f64>) -> Own
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
    A: Link<1, 1, Codomain = Alpha>,
{
    match b {
        "sigmoid_beta" => by_q::<M, A, SigmoidBeta>(q, ic),
        "tanh_beta" => by_q::<M, A, TanhBeta>(q, ic),
        "softsign_beta" => by_q::<M, A, SoftsignBeta>(q, ic),
        other => panic!("a β link {other} no registry entry gives"),
    }
}

fn by_a<M>(a: &str, b: &str, q: &str, ic: &Ic<f64>) -> Own
where
    M: Link<2, 3, Codomain = Mass> + SimplexClamp,
{
    match a {
        "sigmoid_alpha" => by_b::<M, SigmoidAlpha>(b, q, ic),
        "tanh_alpha" => by_b::<M, TanhAlpha>(b, q, ic),
        "softsign_alpha" => by_b::<M, SoftsignAlpha>(b, q, ic),
        other => panic!("an α link {other} no registry entry gives"),
    }
}

/// The selection the links `[mass, α, β, momenta]` name, `ic` inverted under it.
fn expected(l: &[String; 4], ic: &Ic<f64>) -> Own {
    match l[0].as_str() {
        "softmax_tanh" => by_a::<SoftmaxTanh>(&l[1], &l[2], &l[3], ic),
        "stick_breaking" => by_a::<StickBreaking>(&l[1], &l[2], &l[3], ic),
        other => panic!("a mass link {other} no registry entry gives"),
    }
}

/// Every row reports its own selection's `z`, residual, `κ` and clamp, bit for bit; rows one block apart place the IC
/// at different controls of that block.
fn check_own(rows: &[Row], ic: &Ic<f64>) {
    for r in rows {
        let want = expected(&ids(r), ic);
        assert_eq!(
            (
                r.z.map(f64::to_bits),
                r.residual.to_bits(),
                r.kappa.to_bits(),
                r.clamped
            ),
            want,
            "the row of {:?} does not report its own selection's z, residual, κ and clamp",
            ids(r)
        );
    }
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            let (x, y) = (ids(a), ids(b));
            let apart: Vec<usize> = (0..4).filter(|&k| x[k] != y[k]).collect();
            if let [k] = apart[..] {
                assert!(
                    CONTROLS[k]
                        .iter()
                        .any(|&c| a.z[c].to_bits() != b.z[c].to_bits()),
                    "{x:?} and {y:?} place the IC at the same {} controls",
                    BLOCKS[k].0
                );
            }
        }
    }
}

#[test]
fn qa_link_variation_reports_each_selections_own() {
    let ic = moving();
    let rows = diagnose(&ic);
    check_own(&rows, &ic);
    for r in &rows {
        assert!(
            !r.clamped && r.residual <= eps_phys::<f64>() && r.kappa.is_finite() && r.kappa > 0.0,
            "the row of {:?} misses the interior IC: clamped {}, residual {:e}, κ {:e}",
            ids(r),
            r.clamped,
            r.residual,
            r.kappa
        );
    }
    let ic = edge();
    let rows = diagnose(&ic);
    check_own(&rows, &ic);
    assert!(
        rows.iter().any(|r| r.clamped),
        "no selection clamps at the edge IC, so it tests no clamp"
    );
}

negative_control!(
    qa_link_variation_reports_each_selections_own,
    "rows whose results are swapped between two selections do not report their own",
    expected = "does not report its own selection's",
    {
        let ic = moving();
        let mut rows = diagnose(&ic);
        let i = rows
            .iter()
            .position(|r| r.links.momenta.id == "tanh_q" && r.links.mass.id == "stick_breaking")
            .unwrap();
        let j = rows
            .iter()
            .position(|r| r.links.momenta.id == "softsign_q" && r.links.alpha.id == "tanh_alpha")
            .unwrap();
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
        slice: Slice {
            // The slice frame (R-457 E3), at its defaults: the normal and both targets z_q3.
            frame: engine::contract::sim_config::SliceFrame::along(
                engine::contract::sim_config::Z_Q3,
            ),
        },
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

/// Each row's links are a registry selection the record holds: the check passes, each block carries its entry's
/// tunable parameters at their defaults (the selection the row inverted under), and the ViewState record restores them.
fn check_provenance(rows: &[Row]) {
    for r in rows {
        r.links.check().unwrap_or_else(|e| {
            panic!(
                "the row of {:?} records links the registry refuses: {e}",
                ids(r)
            )
        });
        for (block, choice) in r.links.blocks() {
            let want: BTreeMap<String, f64> = ledger::links::tunables()
                .iter()
                .filter(|t| t.entry == choice.id)
                .map(|t| (t.param.to_owned(), t.default))
                .collect();
            assert!(
                choice.params == want,
                "the row of {:?} records {block}'s parameters {:?}, not {want:?}",
                ids(r),
                choice.params
            );
        }
        let record = export(&view(r.links.clone())).expect("a record");
        let back = restore(&record).expect("the row's provenance restores");
        assert!(
            back.links == r.links,
            "the row of {:?} does not travel through the record",
            ids(r)
        );
    }
}

#[test]
fn qa_link_variation_provenance_and_default() {
    let defaults = Links::defaults();
    let ids_before = Defaults::LINK_IDS;
    for (s, id) in ledger::links::slots().iter().zip(ids_before) {
        assert_eq!(
            s.default, id,
            "the default selection is not the registry's slot defaults"
        );
    }
    for ((_, c), (_, s)) in defaults.blocks().into_iter().zip(BLOCKS) {
        let slot = ledger::links::slots()
            .into_iter()
            .find(|x| x.name == s)
            .expect("a slot");
        assert_eq!(
            c.id, slot.default,
            "Links::defaults() is not the slots' defaults"
        );
    }
    let ic = moving();
    let rows = diagnose(&ic);
    check_provenance(&rows);
    let again = diagnose(&ic);
    assert!(rows == again, "the diagnostic is not pure: two runs differ");
    check_provenance(&diagnose(&edge()));
    assert_eq!(
        Links::defaults(),
        defaults,
        "the diagnostic changed the default links"
    );
    assert_eq!(
        Defaults::LINK_IDS,
        ids_before,
        "the diagnostic changed the default selection"
    );
}

negative_control!(
    qa_link_variation_provenance_and_default,
    "a row recording a β link on its α block records links the registry refuses",
    expected = "records links the registry refuses",
    {
        let mut rows = diagnose(&moving());
        rows[5].links.alpha.id = "tanh_beta".to_owned();
        check_provenance(&rows)
    }
);
