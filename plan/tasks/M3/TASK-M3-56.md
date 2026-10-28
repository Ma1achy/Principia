# TASK-M3-56 — Burrau and the Pythagorean family: Chitan et al.'s sixteen triangles, Boekholt et al.'s lifetime and τ_L, Mikkola's bounce orbit and the f64 run against R-33's reference (CH-1, B-1, B-2, V-6, PL-8's first half)

- **Milestone:** M3
- **Closes:** REQ-VAL-242, REQ-VAL-243, REQ-VAL-244, REQ-VAL-245, REQ-VAL-246, REQ-VAL-247, REQ-VAL-248, REQ-VAL-249, REQ-VAL-250, REQ-VAL-251
- **Depends on:** TASK-M3-27, TASK-M3-25, TASK-M3-14, TASK-M3-34, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~450 lines

## Goal
The replication suite's targets CH-1, B-1, B-2, V-6, PL-8's first half run as engine tests (`principia_dd_replication.md`; R-594, R-629). For each of the 16 Pythagorean triangles with hypotenuse below 100, the Newtonian lifetime and the two-body encounter count must be computed and compared with Chitan, Mylläri & Haque's per-triangle values structurally (rule 8: theirs is not a converged code), with both escape criteria recorded (rule 7) (CH-1). Burrau's problem's Newtonian lifetime must be measured and compared with Boekholt, Moerman & Portegies Zwart 2021's plotted plateau, log₁₀ T_l ≈ 1.8 (56 to 71), to the figure's resolution, with their escape criterion (conditions held for 100 time units, from the centre of mass) recorded beside ours (rule 9) (B-1). The Lyapunov exponent of Burrau's problem, measured with the CPU FTLE, must be compared loosely with λ₁₀ = 0.101 ± 0.005 (τ_L = 4.3 ± 0.2), recorded as a fit over every ζ, relativistic runs included, and not a Newtonian-only number (B-2). Mikkola's near-Pythagorean bounce orbit must be shot from the digitised starts of Valtonen et al. 2016's Fig. 3.16 and compared with the figure's tracks (V-6). The nominal Burrau pixel integrated at f64 under the default occupant must be compared with R-33's independent arbitrary-precision reference to t ≈ 60, and the time at which the two part recorded against f64's horizon of about 52 (PL-8's first half; it needs no precision tier). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.6 Burrau and the Pythagorean family"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.5 Burrau itself"
- `decisions.md` § "R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/chitan_2022/newtonian.csv`, transcribed twice, the copies and their diff kept, each row with its page.
- `crates/validation/src/gates/pythagorean.rs`; the book's ×2 remap as a tested function.
- The calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target CH-1` — the Newtonian table transcribed twice (rule 2); per triangle outcome, escaper, lifetime and encounter count beside theirs; agreement counted within REQ-VAL-243, every disagreement listed and queued with both numbers (rule 4); lifetimes past 200 compared only over the systems that end before it (rule 18); control: the triangles' masses permuted must change the escaper (REQ-VAL-242).
- `cargo xtask gate replication --target CH-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-243).
- `cargo xtask gate replication --target B-1` — the lifetime under both criteria; within REQ-VAL-245; the figure reading is the record's (TARGETS B-1), not re-read; control: a run with a 1e-3 perturbation of one position must be reported separately, not averaged in (REQ-VAL-244).
- `cargo xtask gate replication --target B-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-245).
- `cargo xtask gate replication --target B-2` — our FTLE over a window stated with it, beside theirs; within REQ-VAL-247; control: an FTLE on the stable figure-eight must read near zero (REQ-VAL-246).
- `cargo xtask gate replication --target B-2 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-247).
- `cargo xtask gate replication --target V-6` — the digitised markers and tracks (docs/incoming/2026-10-27_papers/valtonen2016/data/) remapped by a tested function (the book's ×2 scale, rule 11); the shot orbit's bounce against the tracks within REQ-VAL-249; the comparison with the figure reviewed by physics; control: the Pythagorean start itself must not bounce (REQ-VAL-248).
- `cargo xtask gate replication --target V-6 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-249).
- `cargo xtask gate replication --target PL-8a` — the chord between the f64 run and the reference sampled through t ≈ 60; the parting time against REQ-VAL-251; control: the f32 run must part earlier than the f64 run (REQ-VAL-250).
- `cargo xtask gate replication --target PL-8a --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-251).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-243, REQ-VAL-245, REQ-VAL-247, REQ-VAL-249, REQ-VAL-251.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
