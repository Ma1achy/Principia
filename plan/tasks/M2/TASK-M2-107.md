# TASK-M2-107 — The periodic β and the decoder without its mirror step, in the docs and the plan: REQ-DEC-056's definitions, the marked sections rewritten, one β link, fifteen requirements (R-865, R-898, R-919)

- **Milestone:** M2
- **Closes:** REQ-DEC-056
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines of docs and plan, no code

## Goal
The docs half of TASK-M2-57, split from it docs first (applied per R-369, 14 Nov 2026: the order is docs, then the
plan, then the code). R-865 removes the decoder's mirror step and runs latent `β` over the full circle; R-898 makes
`z_β` a periodic angle, every stored value converted once; R-919 gives `β` one periodic link. This task writes the
definitions TASK-M2-57 builds to (REQ-DEC-056, R-72), rewrites the sections R-865 marked to the decoder TASK-M2-57
builds, each keeping its *Was*, rewrites the link registry's `β` rows, restates the fifteen requirements R-865 names,
and files REQ-VAL-022's restatement for a ruling. TASK-M2-57 depends on it and builds the code.

## References
- `decisions.md` § "R-865 — The decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the bitwise check: every preset, saved chart and golden with `β ∈ [0, π]` reproduces unchanged, and if anything doesn't, the list comes back to the owner before the removal proceeds *(on RQ-739, which stays open)*"
- `decisions.md` § "R-898 — Latent `β` becomes a periodic angle over the full circle; every stored latent view, saved chart and golden is converted once, the bitwise check replaced for this change by a conversion test; the saved-file format is versioned, old files converted on load; not a hidden half-flag *(closes RQ-750; amends R-865)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_chart_reference.md` § "0.4 Canonicalisation `C`"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/design/principia_chart_composition.md` § "6.6 The symmetry rule"

## Deliverables
- REQ-DEC-056's definitions in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2, under physics review: the periodic link
  and its inverse, its seam, how a syzygy crossing reads on a latent chart, how a stored value converts and why a
  `β ∈ [0, π]` start keeps its bits at f64, whether a converted `z_β` is stored reduced or unreduced and where a wrap
  enters, the pole value at `λ̃ = 0`, and the signed encode.
- The sections marked "[mirror-step removal, ruled 8 Nov 2026 (R-865 …)]" rewritten to the decoder TASK-M2-57 builds,
  each keeping its *Was*, the R-898 dated lines on `ℝ⁸` with them; chart_composition § 6.6 saying how its markers read.
- dd_generation_root § 3.9: `periodic_beta`, `β`'s one link and default; the three bounded `β` maps conversion sources
  only, on a codomain no block takes; each with its *Was*.
- Through `plan/tools/reqio.py`, each keeping a *Was*: REQ-ENC-003, REQ-ENC-005, REQ-ENC-014, REQ-ENC-007,
  REQ-ENC-002, REQ-ENC-015, REQ-ENC-034, REQ-DEC-002, REQ-DEC-016, REQ-DEC-017, REQ-DEC-018, REQ-CHART-002,
  REQ-CHART-020, REQ-COL-006, and REQ-GEN-013's `β` codomain and default.
- REQ-VAL-022's restatement proposed in REVIEW_QUEUE.md for a ruling before TASK-M2-57 merges (R-865), never changed
  here.

## Acceptance tests
- `python3 plan/check_plan.py` passes.
- Review: the definitions approved by the physics reviewer (REQ-DEC-056; R-72).
- Review: every rewritten passage keeps its old text as a *Was*, and every commit carries its "Removed lines" note.

## Notes
- Split from TASK-M2-57 on 14 Nov 2026 by the orchestrator (applied per R-369: task scoping). Its reviewers are code
  and physics, with qa because `plan/check_plan.py` requires code and qa on every task. REQ-DEC-056 moves here;
  TASK-M2-57 keeps REQ-DEC-055 and depends on this task.
- The docs describe the decoder TASK-M2-57 builds; until it merges, `main`'s code is the decoder with its mirror step.
