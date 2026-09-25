# TASK-M1-14 — The synthetic golden suite: sub-field views, adversarial fixtures and colour independence

- **Milestone:** M1
- **Closes:** REQ-TOOL-016, REQ-TOOL-018, REQ-TOOL-011, REQ-VAL-012, REQ-PAY-024
- **Depends on:** TASK-M1-07, TASK-M1-10, TASK-M1-11, TASK-M1-12, TASK-M1-13, TASK-M0-06
- **Needs (earlier milestones):** REQ-PAY-018, REQ-VAL-003, REQ-VAL-007
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.2, PIT-1.7, PIT-3, PIT-9
- **Size:** ~450 lines

## Goal
Step 0b's exit: every payload surface is validated against CPU-filled synthetic buffers — a set state, a known escaper, a ramp in energy_drift, a bitwise-adversarial descriptor, a forced-failure sample — asserting both that the accessor returns the value and that the shader renders it, with each sub-field view rendered on a fixture quad (the visual bit-layout test). The catalogue is ticked against render_contract Part 6's field-view table. And the colour-independence check exists: one fixture slice rendered in event, greyscale and hue modes from the same payload gives an identical edge map, so a structure that survives every map is a physics/logic artefact.

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

## Deliverables
- `fixtures/golden/m1-synthetic/`: the named fixture set (set state, known escaper, energy_drift ramp, bitwise-adversarial descriptor, forced-failure sample) with expected images; `cargo xtask golden m1-synthetic`.
- `fixtures/golden/m1-subfields/`: one image per sub-field view on a fixture quad.
- `crates/render/tests/colour_independence.rs`: the event / greyscale / hue render of one slice and the edge-map comparison.
- The catalogue coverage checklist (a table in the PR, one row per render_contract Part 6 row).

## Acceptance tests
- `cargo xtask golden m1-subfields` — the visual bit-layout test renders each sub-field view on a fixture quad (REQ-TOOL-016).
- `cargo xtask golden m1-synthetic` — the synthetic buffers render the expected golden images and the paired accessor tests pass (REQ-TOOL-018).
- Review checklist (qa): each row of render_contract Part 6's field-view table ticked against the generated catalogue on a synthetic payload (REQ-TOOL-011).
- `cargo test -p render colour_independence` — a fixture slice rendered in event, greyscale and hue modes from the same payload has an identical edge map of the payload-derived field across modes; a control that perturbs the payload between modes must fail (REQ-VAL-012).
- Review checklist (code): a grep of the generated WGSL/Rust finds no `isnan` in validity logic; `cargo test -p engine stored_values_finite` — every stored value of the synthetic and forced-failure samples is finite (REQ-PAY-024).

## Notes
- REQ-TOOL-011's table includes rows M1 can't show (milestone Gaps): the DECODE preset (lands in M2, R-75), the tier-gated ensemble views (M5), the live current-substep heatmap (needs a march, M3), and the uniform echo, whose row names `M` — the retired checkpoint count (REQ-SYS-002).
- VAL-012 is the regression of pitfalls §1.2 (c): colouring is a pure function applied after the physics. The event mode is the outcome palette (TASK-M1-10); greyscale and hue are prelude ramps over the same field.
- Each golden is shown able to fail (VAL-007, PIT-3): the bitwise-adversarial descriptor carries a contaminated bit the unpack would mask, and the test must catch it (PIT-9).
- Waits on RQ-80 (`REVIEW_QUEUE.md`): Retired terms still live in the docs, and the vocabulary lint's doc scope.
- Waits on RQ-94 (`REVIEW_QUEUE.md`): M1 requirements that need M2, M3, M5 or M8.
