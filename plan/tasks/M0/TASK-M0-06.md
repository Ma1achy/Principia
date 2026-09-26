# TASK-M0-06 — The golden-image runner, with one-variable reproduction

- **Milestone:** M0
- **Closes:** REQ-VAL-003, REQ-VAL-009, REQ-VAL-138
- **Depends on:** TASK-M0-01, TASK-M0-04
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.2, PIT-1.3, PIT-3, PIT-8
- **Size:** ~400 lines

## Goal
`cargo xtask golden <suite>` renders each case of a suite headless through the harness, compares it with the reference image in `fixtures/golden/<suite>/` to the tolerance its requirement gives, and writes a diff report. It also has the reproduction mode an image artefact is diagnosed with (philosophy §4.3a; pitfalls §1.3): `cargo xtask golden repro <case> --vary <field>=<a>,<b>` renders two arms whose configurations differ in exactly one field — a pair differing in more is refused — and reports, per arm, the RGB values along a named line (to tell a structure from the ~3-step 8-bit staircase) and each named symptom in its own column, so a fix is closed only against the symptom columns it moved (pitfalls §8). Goldens render with native wgpu offscreen, from M1 on; the M8 Playwright browser suite later checks against the same baselines, and no baseline is re-made without a gate decision (R-110). The screenshot runner, once built here too, is TASK-M0-20 (R-183).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.3a Do not reason about an image — reproduce it"
- `docs/read_first/principia_01_pitfalls.md` § "1.2 Four wrong theories, in order"
- `docs/read_first/principia_01_pitfalls.md` § "1.3 What settled it — a controlled experiment, not an argument"
- `docs/read_first/principia_01_pitfalls.md` § "3. Standing rules earned in this sequence"
- `docs/read_first/principia_01_pitfalls.md` § "8. TWO ARTEFACTS ARE NOT ONE DEFECT"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/design/principia_colour_composition.md` § "7. Preset table & golden-image obligation"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-183 — TASK-M0-06 is split *(closes S1; reverses R-156's size exemption)*"

## Deliverables
- `xtask/src/golden.rs` — `cargo xtask golden <suite>`, `cargo xtask golden --all` (registered in `cargo xtask ci`, run on every commit, R-110), `cargo xtask golden repro`. Cases render through native wgpu offscreen.
- Case format `fixtures/golden/<suite>/<case>/case.json`: render input, reference image, tolerance given as a requirement or calibration-requirement id (the runner refuses a bare number).
- Diff output under `target/golden/`: per-pixel difference image and summary.
- Repro report: the arms' configurations with the single differing field; the RGB profile along the named line per arm; one column per symptom metric per arm.
- `fixtures/golden/selftest/` — a WGSL fragment writing an analytic gradient, with its analytically computed reference.
- `fixtures/golden/BASELINES.md` — the rule that a baseline image changes only with a recorded gate decision (R-110); the golden runner refuses a case whose reference changed without an entry naming that decision.

## Acceptance tests
- `cargo xtask golden selftest` — the analytic gradient matches its reference; the same case against a reference shifted by one 8-bit step fails (the runner can fire).
- `cargo test -p xtask golden_repro` — a repro pair whose configurations differ in two fields is refused; a one-field pair reports the RGB profile along the line for each arm (REQ-VAL-003).
- `cargo test -p xtask golden_repro_columns` — the report tabulates each symptom in its own column per arm; on a fixture where the change moves one symptom and leaves the other unchanged, the unchanged column is reported as unchanged, not merged (REQ-VAL-009).
- Review checklist (physics §3): an artefact investigation record made with this mode shows the one-variable-changed reproduction and the measurement that killed or confirmed each hypothesis (REQ-VAL-003); an ablation closes a fix only against the columns it moved (REQ-VAL-009).
- Proposal: the golden-image diff metric and default tolerance with evidence (same-backend re-render on native wgpu offscreen passes, a one-variable change fails); the human confirms it at the M0 gate (REQ-VAL-138).

## Notes
- No golden-image requirement exists before M1 (REQ-RENDER-024 is the first); the runner is built before them (MILESTONES M0: every verify method has its runner first).
- RQ-79 ruled: R-110 — native golden suites run on every commit, GUI screenshots on GUI PRs and at the gates; goldens render with native wgpu offscreen from M1, and M8's Playwright suite reuses the baselines.
- RQ-93 ruled: R-113 — the screenshot runner was built here, beside the golden-image runner; R-183 splits it into TASK-M0-20.
- Closes, for gaps the corpus leaves open: REQ-VAL-138 (R-71 calibration) (classification accepted by R-132).
