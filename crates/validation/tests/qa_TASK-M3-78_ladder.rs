//! QA tests for TASK-M3-78's re-check (qa review 5910349708, F1), written from REQ-VAL-045 and REQ-VAL-357, not from
//! the implementation:
//!
//! - the "does not fall with η" control reads a *halving* ladder of `η` only (REQ-VAL-045's "over a halving ladder of
//!   η"): a ladder that does not halve exactly, rung by rung, coarsest first, is refused outright, never given a verdict;
//! - REQ-VAL-357's proposal states the minimum with its evidence: `radial-collision --propose` over the contract's
//!   ladder `η = 0.02` to `0.0003125` prints each occupant's passage at every rung and proposes 4, the minimum the PR
//!   and integrator contract Part 1 record.
//!
//! Each test registers its negative control (R-176).

use std::path::{Path, PathBuf};

use serde_json::json;
use validation::gate::{self, Gate};
use validation::heggie::{propose_regions, RadialCollisionGate, RADIAL_NAME};
use validation::negative_control;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Each ladder is refused by the gate, under both occupants, with an error naming the halving ladder, not run to a
/// verdict of any kind.
fn check_refused(ladders: &[&[f64]]) {
    let c = gate::config(&root(), RADIAL_NAME).expect("gate.json");
    for occupant in ["heggie", "aarseth-zare"] {
        for ladder in ladders {
            let input = json!({
                "description": "qa", "occupant": occupant, "reader": "regularised", "eta": 0.01, "ladder": ladder
            });
            match RadialCollisionGate.run(&c, "qa", &input) {
                Ok(r) => panic!(
                    "{occupant}: the ladder {ladder:?} is not refused: verdict {:?}",
                    r.verdict
                ),
                Err(e) => assert!(
                    e.contains("halving ladder"),
                    "{occupant}: {ladder:?} refused for another reason: {e}"
                ),
            }
        }
    }
}

#[test]
fn qa_radial_non_halving_ladder_refused() {
    check_refused(&[
        // one rung: no fall to read
        &[0.01],
        // nearly halving, but a rung off by a part in a thousand
        &[0.01, 0.005, 0.0025 * 1.001, 0.00125],
        // thirding, not halving
        &[0.01, 0.01 / 3.0, 0.01 / 9.0],
        // finest first
        &[0.0025, 0.005, 0.01],
        // a rung repeated
        &[0.01, 0.005, 0.005, 0.0025],
        // a step of zero or below
        &[0.0, 0.0],
        &[-0.01, -0.005],
    ]);
}

negative_control!(
    qa_radial_non_halving_ladder_refused,
    "a ladder that halves exactly, offered as one to refuse",
    expected = "is not refused",
    check_refused(&[&[0.01, 0.005, 0.0025, 0.00125]])
);

/// `--propose`'s output over `ladder`: both occupants' passages at every rung, and the minimum `expected`.
fn check_proposal(ladder: &[f64], expected: usize) {
    let out = propose_regions(ladder, 0.01);
    assert!(out.contains("REQ-VAL-357"), "{out}");
    for occupant in ["## Heggie + RK4", "## AarsethZare + RK4"] {
        assert!(out.contains(occupant), "no {occupant} in {out}");
    }
    for e in ladder {
        assert!(
            out.lines()
                .filter(|l| l.starts_with(&format!("| {e} |")))
                .count()
                == 2,
            "the rung η = {e} is not printed under both occupants: {out}"
        );
    }
    let want = format!("**Proposed:** a region minimum of {expected}.");
    assert!(out.contains(&want), "the proposal is not {expected}: {out}");
}

#[test]
fn qa_radial_proposal_is_four() {
    check_proposal(
        &[0.02, 0.01, 0.005, 0.0025, 0.00125, 0.000625, 0.0003125],
        4,
    );
}

negative_control!(
    qa_radial_proposal_is_four,
    "the proposal over a ladder stopping at η = 0.005, which cannot show four rungs from the gate's η",
    expected = "the proposal is not 4",
    check_proposal(&[0.02, 0.01, 0.005], 4)
);
