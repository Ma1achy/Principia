# TASK-M8-12 — listen: sonification of θ(t), φ(t) from the one computeIC integration

- **Milestone:** M8
- **Closes:** REQ-GUI-049, REQ-GUI-050, REQ-GUI-090, REQ-GUI-143, REQ-GUI-144, REQ-VAL-102, REQ-VAL-103, REQ-VAL-113, REQ-VAL-114, REQ-SYS-060
- **Depends on:** TASK-M8-09, TASK-M8-11, TASK-M3-28
- **Needs (earlier milestones):** REQ-SYS-017, REQ-CHART-036, REQ-TOOL-028, REQ-VAL-041
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-3
- **Size:** ~500 lines

## Goal
listen (Trajectory panel and Inspector) sonifies the corpus's mapping — θ(t), φ(t) → spectrum → audible range — from the same computeIC result as the trace, following the cursor, cross-fading between hovered ICs and never retriggering per pixel. The audible range is set by mapping 1/t_c to a calibrated reference pitch so frequency ratios hold across ICs. The open choices (θ alone or stereo θ / φ; whole-trajectory or windowed spectrum) are defined in pointer_channels §4; the chaotic anchor IC is defined in trajectory_viewing §1; the minimum trajectory length for a usable spectrum is measured; the spectral-entropy anchors (0.076 figure-eight, 0.177 chaotic) are checked to a calibrated tolerance. The spectrum runs in the inspector worker and costs the frame loop nothing. The artboard's "separations → pitch" selector is not built (R-68).

## References
- `docs/design/principia_scratchpad_pointer_channels.md` § "4. Audio-specific constraints, which are NOT the frame budget"
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-109 — Only pointer_channels §4 is normative *(closes RQ-70)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-144 — pointer_channels §3 is normative too *(closes RQ-74, amends R-109)*"

## Deliverables
- `crates/engine/src/inspector/sonify.rs` — spectrum of θ(t), φ(t), 1/t_c scaling, spectral entropy; runs in the inspector worker.
- `crates/gui/src/listen.rs` — audio out with cross-fade (native: an audio-output backend; browser: WebAudio in TASK-M8-38's shell).
- `crates/validation/src/sonify_anchor.rs` — the entropy gate and the minimum-t measurement.
- Doc changes: `docs/design/principia_scratchpad_pointer_channels.md` §4 (the two open choices); `docs/design/principia_trajectory_viewing.md` §1 (the chaotic IC and horizon).
- Calibration proposals: the reference pitch (pitch range for the figure-eight and a set of chaotic ICs), and the entropy tolerance (spread under windowing and length choices).

## Acceptance tests
- `cargo test -p engine sonify_pitch_ratio` — two ICs with the same dynamical frequency ratio produce the same pitch interval (REQ-GUI-049).
- `cargo test -p gui listen_crossfade` — the audio buffer across an IC change has no discontinuity (no click) (REQ-GUI-050).
- `cargo test -p engine sonify_input_is_theta_phi` — the audio path's input is the θ(t), φ(t) series of the hovered IC's computeIC trajectory; no separations-based mapping exists (REQ-GUI-090).
- Review checklist (physics reviewer) of the pitch proposal — the proposal shows the resulting pitch range for the figure-eight and a set of chaotic ICs stays within the audible range; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-GUI-143).
- Doc review of `docs/design/principia_scratchpad_pointer_channels.md` § "4. Audio-specific constraints, which are NOT the frame budget" — pointer_channels §4 states both choices; the reviewer checks they keep the trace and the sound one measurement (§2); the physics reviewer approves the doc change before merge (REQ-GUI-144).
- `cargo xtask gate sonify-entropy` (tolerance: REQ-VAL-114, calibrated) — spectral entropy for the figure-eight ≈ 0.076 and for the chaotic reference ≈ 0.177; periodic < chaotic (REQ-VAL-102).
- `cargo xtask gate sonify-min-length` — a recorded measurement of spectral line resolution vs t gives the minimum t used by listen (REQ-VAL-103).
- Doc review of `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism" — trajectory_viewing §1 gives the chaotic IC (as z or physical state) and the horizon of the measurement; the physics reviewer approves the doc change before merge (REQ-VAL-113).
- `cargo xtask gate sonify-entropy` — the proposal shows the entropy's spread under the spectrum's windowing and trajectory-length choices and a tolerance that still separates the two orbits; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-VAL-114).
- `cargo xtask bench hover-listen-frame` — frame times with continuous hover + listen equal the no-hover baseline within noise; the work runs on the inspector worker (REQ-SYS-060).

## Notes
- R-109, amended by R-144, makes pointer_channels §3 and §4 normative; §3's per-frame-during-motion budget is TASK-M8-09's.
- Calibrations (R-71) proposed here: REQ-GUI-143, REQ-VAL-114. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- Definitions (R-72) written here: REQ-GUI-144, REQ-VAL-113. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
