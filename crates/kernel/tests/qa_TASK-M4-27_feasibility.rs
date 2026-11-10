//! QA (TASK-M4-27; REQ-VAL-347): the read-back's `INVARIANT_INFEASIBLE` outcome is the decision the production
//! decode takes. The task's deliverable: "No parity-only physics: each branch's input and decision move into a function
//! that its production site and the entry both call, with no change of behaviour". The decode's construction decides
//! `INVARIANT_INFEASIBLE` by `no_construction(K*, L_z)`: `K* < L_z²/2`, or a `K*` with no finite `2K*`
//! (`decode/invariant.rs`, the module's note "A `K*` with no finite `2K*`"; chart_reference §4.5). So the read-back's
//! outcome word must be that predicate's on every `(K*, L_z)`, the no-finite-double binade included, or a backend's
//! read-back reports a decision the stage never takes. Written from the deliverable, not from the read-back's body.

use kernel::decode::invariant::no_construction;
use kernel::stage::branch_read as read;
use validation::negative_control;

/// `(K*, L_z)` pairs on both sides of each half of the decision: the feasibility test's edge, and `K*` below, at and
/// past the first binade whose double has no finite f32 (`2^127`), `±∞` and NaN.
fn pairs() -> Vec<(f32, f32)> {
    let edge = f32::from_bits(0x7f00_0000); // 2^127
    let below = f32::from_bits(0x7eff_ffff); // the greatest f32 below 2^127
    let mut v = vec![
        (1.0, 1.0),
        (0.125, 0.5),
        (0.0, 0.0),
        (-0.5, 1.0),
        (1.0, 2.0),
    ];
    for k in [below, edge, f32::MAX, f32::INFINITY, f32::NAN] {
        for lz in [0.0, 1.0, -1.0] {
            v.push((k, lz));
        }
    }
    v
}

/// Each pair's outcome word, as `outcome` writes it from word `at`, against the production decode's decision.
fn check(outcome: impl Fn(f32, f32) -> u32) {
    for (k, lz) in pairs() {
        let want = u32::from(no_construction(k, lz));
        assert_eq!(
            outcome(k, lz),
            want,
            "the read-back's INVARIANT_INFEASIBLE outcome is not the decode's at K* = {k:e} ({:#010x}), L_z = {lz}",
            k.to_bits()
        );
    }
}

/// The read-back's outcome word: word 1 of `write_feasibility`'s slot.
fn read_back(k: f32, lz: f32) -> u32 {
    let mut out = vec![0u32; 9];
    read::write_feasibility(&mut out, 0, k, lz);
    assert_eq!(out[0], 1, "the slot is not marked written");
    out[1]
}

#[test]
fn qa_branch_read_feasibility_is_the_decodes_decision() {
    check(read_back);
}

negative_control!(
    qa_branch_read_feasibility_is_the_decodes_decision,
    "a read-back that drops the no-finite-2K* half of the decision",
    expected = "is not the decode's",
    check(|k, lz| u32::from(kernel::decode::invariant::infeasible(1.0, k, lz)))
);
