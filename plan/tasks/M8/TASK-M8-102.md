# TASK-M8-102 — CPU batch: numerical inverses, their branch rule and seam

- **Milestone:** M8
- **Closes:** REQ-CHART-129, REQ-CHART-130, REQ-CHART-155
- **Depends on:** TASK-M2-73, TASK-M4-06, TASK-M6-03, TASK-M4-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The third lowering: an unbaked or numerical pair fills the integrator's IC buffer from f64 on the CPU, labelled and not live.

## References
- `docs/design/principia_chart_composition.md` § "7.2 CPU batch"
- `docs/design/principia_chart_composition.md` § "7.3 A numerical inverse: branch and seam"
- `docs/design/principia_chart_composition.md` § "7.4 Seams: jitter, spread and refinement"
- `docs/design/principia_dd_refinement_policy.md` § "1. The split rule — no exponent in the split test"
- `docs/design/principia_core_design.md` § "3. Kernel eats chart params, decodes in-kernel"
- `docs/design/principia_dd_refinement_policy.md` § "0.1 In view, the camera decides depth and the criterion decides ORDER"
- `docs/design/principia_dd_refinement_policy.md` § "6. Decision variants, and the second budget line"
- `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `decisions.md` § "R-889 — The browser build gets a WASM CPU route too: numerical axes, unbaked closed forms and CPU-only morphs run as CPU batch in the engine's worker, labelled and not live *(closes RQ-735)*"

## Deliverables
- `crates/engine/src/chart_batch.rs`.
- The solver, its rule and seam; the branch in the IC record: one word per sample, on CPU batch only, for every CPU batch chart, closed-form or numerical, beside `write_ic`'s words, whose `decode_failed` also carries a chart mask.
- The `seam_straddled` flag in the resolve pass (R-142), for baked charts and from `branch` in the IC record for CPU batch ones; `seam-contaminated`; the mask on `Policy::Tolerance`'s `unresolved`; the decision `Seam` and `n_seam`, a slot added to `QuadReduction`'s 44 (REQ-PAY-077). Both layouts are a schema change, as R-603's was: the payload schema version bumps.

## Acceptance tests
- `cargo test -p engine seam_jitter_flagged` — copies at the sampling note's offsets in `(u, v)`; a footprint across a numerical seam, a closed-form CPU batch row's two-root branch or mask, polar's branch line or a baked chart's mask, flagged, all `E + 1` copies kept, spread and `error_ratio` labelled `seam-contaminated`; controls: a copy dropped at the seam, the flag read from a neighbour (REQ-CHART-129).
- `cargo test -p engine seam_refinement_masked` — a flagged footprint leaves `n_unresolved`, a mask-blanked copy included; a quad with any flagged footprint and none unresolved takes `Seam`, a mixed quad included, `n_seam` counted, no priority from `coherence`; a non-finite copy still fires; an unflagged neighbour still splits; today's registry bitwise; controls: the mask removed, the mask on an unflagged footprint, `Keep` for `Seam`, `Seam` only when every footprint is flagged (REQ-CHART-130).
- The CPU batch route built for `wasm32` and its tests run under the wasm target in CI; a numerical chart's solve run native and wasm on one fixture, compared bit for bit, the result reported in the PR; controls: a numerical chart refused in the browser fails, a CPU batch view not labelled fails (REQ-CHART-155).

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- Shares R-669's Path B batch where that exists.
- Waits on its rulings item: RQ-719. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *7 Nov 2026 (R-849, R-839):* RQ-733 is ruled (b), and RQ-719 (a): a labelled CPU-batch view is seam 3's stated exception.
- *8 Nov 2026 (R-873, R-889):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M8-117, now TASK-M2-65), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M8-106 (now TASK-M2-81) and TASK-M8-108, stage 4, depend on it. R-889 (RQ-735 (b)): in the browser build, numerical axes, closed forms past the cap or unbaked and morphs with a CPU-only end run as CPU batch in the engine's worker, Rust compiled to WebAssembly, labelled, not live (REQ-CHART-155). *Was:* "- **Closes:** REQ-CHART-107, REQ-CHART-129, REQ-CHART-130, REQ-CHART-145"; "- **Depends on:** TASK-M4-37, TASK-M4-33, TASK-M4-06, TASK-M6-03, TASK-M4-34, TASK-M8-100"
- *9 Nov 2026 (R-916):* the line above is a record: stage 2 (TASK-M8-117 then, TASK-M2-65 now) is no longer in Depends on; this task comes after it through TASK-M2-73, and after M2's gate by R-917's hold. TASK-M2-81 (formerly TASK-M8-106) is M2's and no longer depends on this task; TASK-M8-108 still does.
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-107, REQ-CHART-129, REQ-CHART-130, REQ-CHART-145, REQ-CHART-155"; "Depends on: TASK-M4-37, TASK-M4-33, TASK-M4-06, TASK-M6-03, TASK-M4-34, TASK-M8-100, TASK-M8-117".
- *9 Nov 2026 (R-916):* split: the numerical inverse's rule, branch and seam and the general numerical solve (REQ-CHART-107, REQ-CHART-145) are TASK-M2-73's, `crates/engine/src/chart_batch.rs`'s solver among them; this task keeps the seam's jitter and refinement, the payload's slots and the browser build. *Was (Acceptance):* the `numerical_branch_seam` and `general_numerical_solve` lines, now at TASK-M2-73.
