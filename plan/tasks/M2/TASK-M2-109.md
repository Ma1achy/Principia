# TASK-M2-109 — REQ-DEC-060's seam clause: the seam measurement across `z_α = 0` under the criterion R-1022 rules, the seed conversion's other clauses built by TASK-M2-93 (R-934)

- **Milestone:** M2
- **Closes:** REQ-DEC-060
- **Depends on:** TASK-M2-93
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** one test's criterion, read as R-1022 rules it; little new code

## Goal
TASK-M2-93 built the expansion seed `w = r/√I`, the conversion of every image it changes and the Lagrange check (R-934). It also ran the seam measurement REQ-DEC-060 asks for, the decoded velocity's step across `z_α = 0` on the kernel's `(L_z, E)` chart. Under the seed in force the written criterion, "no neighbouring pair of pixels across the line differs in decoded velocity by more than the largest neighbouring difference within 8 pixels of it on either side", does not hold, though there is no seam: the step across the line is 1.0000610 times the largest nearby, from the `α` link's own warp, `1 + h²/4` at `h = 1/64` (RQ-820). This task closes REQ-DEC-060 once RQ-820 is ruled: the seam measurement read by the criterion the ruling gives, with its control, the same measurement under R-82's legacy rule, failing it.

Split from TASK-M2-93 (applied per R-369, 15 Nov 2026, task scoping: the task re-scoped to match what holds; no requirement dropped or deferred). The orchestrator decided it in the fix round on PR #547 (the physics review's F1). REQ-DEC-060 moved here from TASK-M2-93. RQ-820 stays open for the owner. If it is ruled otherwise, this task follows the ruling.

## References
- `decisions.md` § "R-934 — RQ-764, the momentum seed: option (a), the expansion seed `w = r/√I` takes the place of the largest-norm rule of R-82 everywhere, the invariant charts and Burrau's included; the seam is not physics; every image it changes is a recorded conversion, updated under R-931; R-82's rule is kept as a named option with its seam declared *(closes RQ-764; amends R-82)*"
- `decisions.md` § "R-1022 — RQ-820, the seed conversion's seam check: option (b), the explanation identity is the criterion; every neighbouring pair's step in decoded velocity equals `√(2K*)·2 sin(|Δα|/2)` to a relative 1e-9, and R-82's legacy rule, the control, fails it across the line *(closes RQ-820)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"
- `docs/design/principia_dd_decoder.md` § "3.4 Momentum"

## Deliverables
- `crates/kernel/tests/seed.rs`'s `seed_conversion_seam` read by R-1022's criterion, the explanation identity: every neighbouring pair of pixels, the pair across the line included, differs in decoded velocity by `√(2K*)·2 sin(|Δα|/2)` in the mass-weighted norm, to a relative 1e-9 (the identity TASK-M2-93's test already asserts), judged as the check; its control, the same measurement under R-82's legacy rule, failing it across the line on every row (else the result is void). The first criterion's numbers are printed as before, an explained divergence, and judge nothing. *Was (17 Nov 2026, R-1022):* "read by the criterion RQ-820 rules, written before the run as the ruling gives it, its control, the same measurement under R-82's legacy rule, failing it on every row (else the result is void)."

## Acceptance tests
- `cargo test -p kernel seed_conversion` — REQ-DEC-060's acceptance as TASK-M2-93 states it, the seam measurement read by R-1022's criterion, the explanation identity, in place of the one TASK-M2-93 wrote (*was:* "by the criterion RQ-820 rules"); the conversion of every invariant and mixed-energy chart image where `K* > K_min` and the Lagrange check as TASK-M2-93 built them, PASS required (REQ-DEC-060).

## Notes
- Entered 15 Nov 2026 in the fix round of PR #547 (applied per R-369: task scoping).
- **Waited for RQ-820**, the owner's reading of the seam criterion, ruled 17 Nov 2026 by R-1022 ("RQ-820: (b)."): option (b), the explanation identity; this task is ready to dispatch. *Was:* "**Waits for RQ-820** (`REVIEW_QUEUE.md`, open), the owner's reading of the seam criterion: this task isn't dispatched before it is ruled."
- What TASK-M2-93 measured, 64 columns at `h = 1/64` about `z_α = 0`, 6 rows: under the seed in force the pair across the line is 1.000061 times the largest beside it on every row; under R-82's legacy rule, the control, the pairs across the line are 0.8, 1.8, 1.039234, 1.120006, 1.200020 and 0.572364, each `2√(K* − K_min)`, ratios of 172 and more; every neighbouring pair under the seed in force is `√(2K*)·2 sin(|Δα|/2)` to a relative 1e-9.
