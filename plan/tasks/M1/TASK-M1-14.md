# TASK-M1-14 — The synthetic golden suite: sub-field views, adversarial fixtures and colour independence

- **Milestone:** M1
- **Closes:** REQ-TOOL-016, REQ-TOOL-018, REQ-VAL-012, REQ-PAY-024
- **Depends on:** TASK-M1-07, TASK-M1-10, TASK-M1-11, TASK-M1-12, TASK-M1-13, TASK-M1-17, TASK-M0-06
- **Needs (earlier milestones):** REQ-PAY-018, REQ-VAL-003, REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.2, PIT-1.7, PIT-3, PIT-9
- **Size:** ~450 lines

## Goal
Step 0b's exit: every payload surface is validated against CPU-filled synthetic buffers — a set state, a known escaper, a ramp in energy_drift, a bitwise-adversarial descriptor, a forced-failure sample — asserting both that the accessor returns the value and that the shader renders it, with each sub-field view rendered on a fixture quad (the visual bit-layout test). And the colour-independence check exists: one fixture slice rendered in event, greyscale and hue modes from the same payload gives an identical edge map, so a structure that survives every map is a physics/logic artefact.

## References
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_debug_tooling_plan.md` § "Principle"
- `docs/design/principia_debug_tooling_plan.md` § "Build order within Phase 0 (the only forced staggering)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/read_first/principia_01_pitfalls.md` § "1.2 Four wrong theories, in order"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/read_first/principia_01_pitfalls.md` § "1.3 What settled it — a controlled experiment, not an argument"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "H. Codegen self-test (the tooling that tests the tooling)"
- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-271 — `d_min`'s unset value is +inf; stored values never reach 0.0 *(closes RQ-163, amends payload §1)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `fixtures/golden/m1-synthetic/`: the named fixture set (set state, known escaper, energy_drift ramp, bitwise-adversarial descriptor, forced-failure sample) with expected images; `cargo xtask golden m1-synthetic`.
- `fixtures/golden/m1-subfields/`: one image per sub-field view on a fixture quad.
- `crates/render/tests/colour_independence.rs`: the event / greyscale / hue render of one slice and the edge-map comparison.

## Acceptance tests
- `cargo xtask golden m1-subfields` — the visual bit-layout test renders each sub-field view on a fixture quad (REQ-TOOL-016).
- `cargo xtask golden m1-synthetic` — the synthetic buffers render the expected golden images and the paired accessor tests pass (REQ-TOOL-018).
- `cargo test -p render colour_independence` — a fixture slice rendered in event, greyscale and hue modes from the same payload has an identical edge map of the payload-derived field across modes; a control that perturbs the payload between modes must fail (REQ-VAL-012).
- Review checklist (code): a grep of the generated WGSL/Rust finds no `isnan` in validity logic; `cargo test -p engine stored_values_no_nan` — no stored value of the synthetic and forced-failure samples is NaN, and the only non-finite stored value is R-271's unset f16 +inf (`0x7C00`) (REQ-PAY-024).

## Notes
- VAL-012 is the regression of pitfalls §1.2 (c): colouring is a pure function applied after the physics. The event mode is the outcome palette (TASK-M1-10); greyscale and hue are prelude ramps over the same field.
- Each golden is shown able to fail (VAL-007, PIT-3): the bitwise-adversarial descriptor carries a contaminated bit the unpack would mask, and the test must catch it (PIT-9).
- RQ-229, decided per R-369 (7 Oct 2026): `m1-subfields` and `m1-synthetic` read the payload, so their cases are `cargo xtask golden`'s harness case kind, which TASK-M1-09 builds (RQ-229, as amended per code review 5438179638); each case's `BASELINES.md` row cites R-369 and RQ-229, proposed and confirmed at the M1 gate.
- Depends on TASK-M1-17 (R-399, R-400 and R-403, applied per R-369), so the suite renders the views in their conformed
  looks. R-401's per-field log floors are TASK-M1-18's, split from TASK-M1-17 (applied per R-369; physics review
  5482528309 of PR #194); this task does not depend on it, as it reads no floor value: if this task merges first,
  TASK-M1-18 re-renders any of its goldens that draws one of the six log views.
- Split, applied per R-369 (TASK-M1-14's pre-flight): REQ-TOOL-011, the render_contract Part 6 catalogue checklist,
  moves to TASK-M1-19, which depends on this task, with its deliverable and its qa checklist line. RQ-273 holds
  REQ-TOOL-011 for two rows the corpus gives no way to build in M1 (the uniform echo and the Optional Fourier block);
  it holds TASK-M1-19 only, not this task.
- REQ-PAY-024's acceptance line is conformed to R-271, applied per R-369: a failed sample's `d_min` half, and any
  sample's before its first step, holds f16 +inf (bits `0x7C00`), so "every stored value … is finite" was stale; R-79
  and payload §1 say only that storage never holds NaN. The test name follows (`stored_values_no_nan`, was
  `stored_values_finite`).
- Two gaps are R-72 definitions, which this task writes as doc changes, reviewed by physics (R-72): "greyscale mode"
  and "hue mode", which only `docs/read_first/principia_01_pitfalls.md` § "1.2 Four wrong theories, in order" (line 40)
  names; and REQ-VAL-012's "edge map" (of the payload-derived field), which no doc defines.
- The PR lists which of this task's cases draw one of R-401's six log views (`d_min`, `dE_max`, `dLz_max`,
  `closure_min`, `rho_ratio`, `r_min_pair_0`), since TASK-M1-18 re-renders them (REVIEW_QUEUE.md RQ-271, Waits).
