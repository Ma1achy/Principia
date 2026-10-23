# TASK-M8-76 — Where precision runs out: four runs of the clicked point, the chord's two marks, the trust line, and nothing changed in the atlas (R-574)

- **Milestone:** M8
- **Closes:** REQ-VAL-196, REQ-VAL-197, REQ-VAL-198, REQ-VAL-199, REQ-GUI-369, REQ-GUI-370, REQ-GUI-371
- **Depends on:** TASK-M8-17, TASK-M4-16
- **Needs (earlier milestones):** REQ-TOOL-127, REQ-TOOL-040
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-10
- **Size:** ~600 lines, the tests included

## Goal
The click inspector runs the clicked point four ways (R-574): f32, the survey kernel's one-pixel GPU dispatch, bitwise what coloured the pixel; f64 by `computeIC`; and each nudged by one ulp per component of its decoded IC. It reads the shape-sphere chord between them at the sync boundaries for f32–f64, f64–f64′ and f32–f32′, and marks "faithful until" (10⁻³) and "parted at" (0.1), the human's values, shown and editable, with the self-check. It shows the runs (f64 solid, f32 dashed), the separation plot, the marks, the starting gap, the fates side by side, and, when a trajectory is clicked, the trust line about the atlas pixel. The atlas does not change.

## References
- `docs/design/principia_honesty_precision.md` § "3.1 Four runs of one IC"
- `docs/design/principia_honesty_precision.md` § "3.2 The chord and the two marks"
- `docs/design/principia_honesty_precision.md` § "3.3 What the user sees"
- `docs/design/principia_honesty_precision.md` § "3.4 Where it lives"
- `docs/design/principia_honesty_precision.md` § "3.5 Nothing changes in the atlas"
- `docs/design/principia_honesty_precision.md` § "6. Tests (each with what makes it fail)"
- `docs/design/principia_trajectory_viewing.md` § "5. What is honestly comparable (click inspector)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `decisions.md` § "R-574 — Honesty as a measurement: the click inspector runs the clicked point four ways, f32, f64 and each nudged by one ulp, and marks "faithful until" at a shape-sphere chord of 10⁻³ and "parted at" at 0.1, both measured, shown and editable; the trust line shows in the trajectory inspector panel when a trajectory is clicked; nothing changes in the atlas; the README's Honesty line is reworded"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/inspector/precision.rs` — the four runs (the nudge, the GPU dispatches of TASK-M4-16's trace for f32 and nudged f32, `computeIC` for f64 and nudged f64), the chord series at the sync boundaries, the marks and the self-check.
- `crates/gui/src/inspector/precision.rs` — the dashed f32 run in panes 2 and 3 and the Trajectory panel's previews, the separation plot, the marks (`ViewUI`, editable), the starting gap, the fates' wording and the trust line, in the Inspector by default and in the Trajectory panel for a pinned or kept trajectory.
- `cargo xtask gate parting-self-check --propose` and `cargo xtask gate parting-ratio --propose`.

## Acceptance tests
- `cargo test -p engine f32_run_is_the_atlas` — the f32 run's outcome and t_end equal the atlas pixel's nominal copy bitwise; control: a CPU f32 run of the same IC differs on a chaotic fixture, so emulation fails (REQ-VAL-196).
- `cargo test -p engine nudge_is_one_ulp` — each of the twelve components of each nudged IC is next_up of its run's own; control: a nudge by ε·|x| fails on a component just below a power of two (REQ-VAL-196).
- `cargo test -p engine parting_marks` — same_run_never_parts (f64 against itself, the chord exactly 0; control: a harness adding a difference fails); regular_orbit_does_not_part (the figure-eight, "did not part by T" for every pair at 0.1); chaotic_orbit_parts (Burrau, t_part(f32) and t_part(f64) found, f32's earlier); marks_follow_edits (the parted mark edited to 10⁻² moves to the first boundary above 10⁻² on a recorded chord series; control: a constant mark fails) (REQ-VAL-197).
- `cargo xtask gate parting-self-check --propose` — the gap against ln(100)/λ over the parting probe's four regions, the band proposed to clear the chaotic regions' p10–p90 and flag the noise floor and saturation; and `cargo test -p engine self_check_unreliable`: marks one boundary apart at a large FTLE read "unreliable" (control: the check removed reads a time and fails); recorded in decisions.md after the human confirms it at the M8 gate (REQ-VAL-198, calibrated).
- `cargo xtask gate parting-ratio --propose` — at T = 200 over at least 256 chaotic ICs where both pairs part, the count reported, the median t_part(f32)/t_part(f64) against the value RQ-557 rules, the tolerance proposed from the spread; recorded in decisions.md after the human confirms it at the M8 gate (REQ-VAL-199, calibrated; waits on RQ-557).
- `cargo test -p gui precision_view` — agreement_wording: the same fate latched after parting reads "not evidence" (mutation: the before/after check dropped reads "settled" and fails); a run out of budget shows as such with its end time; three chord series in the plot; the four runs run for a pin and not for the cursor; an exported Inspector figure's settings name all four runs (REQ-GUI-369).
- `cargo test -p gui trust_line` — on a recorded fixture with t_part(f32) = 18.4 and t_part(f64) = 41, the line reads the first form at t_p = 10, the second (naming t = 18.4) at t_p = 30 and the third at t_p = 45 (control: a line keyed on the faithful mark reads the second form at t_p = 10 and fails); the FTLE estimate beside it; no line while the panel follows the cursor (REQ-GUI-370).
- `cargo test -p engine atlas_unchanged` — the atlas frame is bitwise the same with the viewer's four runs on and off, before and after a mark edit; control: a viewer that writes its parting into the atlas's colour inputs fails (REQ-GUI-371).

## Notes
- R-574 (21 Oct 2026). The marks 10⁻³ and 0.1 are the human's values (prin-rs PR #34, `parting_probe`; `docs/incoming/2026-10-21_gap_sweep_results_honesty/`, a record), not calibrations.
- REQ-VAL-198 and REQ-VAL-199 are calibrations (R-71): the PR carries each proposed value, its evidence and the reviewer's check, marked pending; the human confirms them at the M8 gate. REQ-VAL-199 waits on RQ-557.
- RQ-558 (the runs past a terminal event): its option (a), as measured, applied meanwhile.
- RQ-556 (what the header strip states) is TASK-M8-41's; nothing here touches the atlas or its chrome.
