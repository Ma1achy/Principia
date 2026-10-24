# TASK-M8-83 — The Precision ring's state-in entry, `computeICState(state, simKey)`, beside `computeIC`, and the frame the constructor integrates (R-586)

- **Milestone:** M8
- **Closes:** REQ-INT-107
- **Depends on:** TASK-M3-19, TASK-M2-43
- **Needs (earlier milestones):** REQ-SYS-017, REQ-INT-055, REQ-CHART-081
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines, the tests included

## Goal
The Precision ring gains `computeICState(state, simKey) → SimState`, the integration `computeIC` runs after its decode, from a full state given, on the CPU in f64 under R-33's RK45, so a built, off-slice IC can be integrated bitwise; the constructor integrates `C(as_built)` with `C`'s mirror step not taken (RQ-563's option (a)). A picked IC still enters through `computeIC` (R-586).

## References
- `docs/design/principia_ic_save.md` § "3.4 Integration"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `decisions.md` § "R-586 — An initial condition is saved, shared and reopened: the `principia.ic/1` record in JCS, bitwise at f64 with −0 normalised, copied, pasted, saved and opened into Create & locate in its as-built frame with three outcomes; a state-in entry, `computeICState`, beside `computeIC`; the mirror twin shown on reopening; a marked IC a `Trace` whose `ic` is an `IcRef`, travelling in `Lines` *(queues RQ-563 and RQ-564)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*"
- `decisions.md` § "R-589 — The IC constructor integrates `C(as_built)` with steps 1a, 0, 1b and 2 taken and step 3, the mirror, not taken: the hemisphere kept, as the shape sphere chart's own decode does, never the mirror twin; the trajectory CSV stays in that frame *(closes RQ-563; settles R-586's interim)*"

## Deliverables
- `crates/engine/src/` (beside `computeIC`): `computeICState`, sharing `computeIC`'s integration after the decode; the constructor's canonicalisation with step 3 not taken.
- Test `compute_ic_state`.

## Acceptance tests
- `cargo test -p engine compute_ic_state` — the test first asserts that its built fixture's state is changed by decode(encode(state)), so the re-decode control can fail; for a picked IC, computeICState on computeIC's decoded state gives a SimState bitwise equal to computeIC's; for a built IC in the lower hemisphere the integrated initial state is bitwise the shared C's step-3-not-taken result on as_built (REQ-CHART-081): the centre of mass at the origin and at rest, I = 1, ρ̃ on +x, its λ̃_y sign and its L_z sign kept; the entry runs RK45 in f64 (controls: an entry that folds the mirror, giving the twin's trajectory, one that re-decodes z, which is not bitwise, one that skips the rescale, and one that skips the boost, each fail); review (physics) (REQ-INT-107).

## Notes
- R-586 (21 Oct 2026, gap sweep PR 2c): the human accepted the draft's state-in entry, which amends `computeIC`'s contract (parity contract §1).
- **RQ-563 is ruled** by R-589 (24 Oct 2026; physics), option (a): the constructor integrates `C(as_built)` with steps 1a, 0, 1b and 2 taken and the mirror step not taken, the hemisphere kept, never the twin. *Was:* "**RQ-563 is open** (the frame integrated, against R-544; physics): meanwhile its option (a), `C(as_built)` with the mirror step not taken, the hemisphere kept; nothing waits. A ruling for (b) would fold the mirror here."
- TASK-M8-16 depends on this task: its Inspector reaches the engine through `computeIC` and `computeICState` (REQ-GUI-117).
- **Fix round 2 of PR #287 (qa review 5638084688, Q8, applied per R-369):** the acceptance lines follow the new verifies. *Was:* "`cargo test -p engine compute_ic_state` — for a picked IC, computeICState on computeIC's decoded state gives a SimState bitwise equal to computeIC's; for a built IC in the lower hemisphere the state integrated keeps its hemisphere (its λ̃_y sign) and its L_z sign, the rotation and centre of mass canonicalised; the entry runs RK45 in f64 (controls: an entry that folds the mirror, giving the twin's trajectory, and one that re-decodes z, which is not bitwise, each fail); review (physics) (REQ-INT-107)."
- **Fix round 3 of PR #287 (code review 5638122561, C2, applied per R-369):** this task depends on TASK-M2-43, whose `canon.rs` gives the shared `C` its step-3-not-taken entry (REQ-CHART-081). *Was:* "- **Depends on:** TASK-M3-19"; "- **Needs (earlier milestones):** REQ-SYS-017, REQ-INT-055".
- **Fix round 4 of PR #287 (physics review 5638148080, P2, applied per R-369):** the acceptance lines follow the new verifies. *Was:* "`cargo test -p engine compute_ic_state` — the test first asserts that its built fixture's state is changed by decode(encode(state)), so the re-decode control can fail; for a picked IC, computeICState on computeIC's decoded state gives a SimState bitwise equal to computeIC's; for a built IC in the lower hemisphere the state integrated keeps its hemisphere (its λ̃_y sign) and its L_z sign, the rotation and centre of mass canonicalised; the entry runs RK45 in f64 (controls: an entry that folds the mirror, giving the twin's trajectory, and one that re-decodes z, which is not bitwise, each fail); review (physics) (REQ-INT-107)."
