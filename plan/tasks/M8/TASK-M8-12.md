# TASK-M8-12 — Listen, 1: the signal, spectrum, pitch, windows, channels and loudness from the one computeIC integration, with the audio out (R-540 S1 to S4, S6, S7) ⚑

- **Milestone:** M8
- **Closes:** REQ-GUI-049, REQ-GUI-050, REQ-GUI-090, REQ-GUI-143, REQ-GUI-144, REQ-VAL-102, REQ-VAL-103, REQ-VAL-113, REQ-VAL-114, REQ-SYS-060, REQ-GUI-309, REQ-GUI-310, REQ-GUI-311, REQ-GUI-312, REQ-GUI-313, REQ-GUI-314, REQ-GUI-316
- **Depends on:** TASK-M8-09, TASK-M8-11, TASK-M3-28
- **Needs (earlier milestones):** REQ-SYS-017, REQ-CHART-036, REQ-TOOL-028, REQ-VAL-041
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-3
- **Size:** ~1,300 lines, the tests included (~500 before R-540)

## Goal
**This task waits on RQ-440 (the audio backend) and RQ-442 (`RMS_FLOOR` against prin-rs's integrator)**, which the human's message says need answers before Listen is built. R-540 makes listen **Listen**, `docs/design/principia_listen.md`, transcribed from its demo's code: this task builds its analysis and its audio out, §1 to §4, §6 and §7 — the signal from the Jacobi vectors with no canonical frame (`w·e^{iθ}`, `φ`, and the rotation `e^{iψ}` under the default), the complex spectrum whose sign is the stereo side, the peaks with the 1.5-period cut and the mean-rate fallback, so every moving run sounds, the pitch `f·t_c·f_ref·2^shift`, the windows and frames following the head or the whole run as one chord, silence past the end, the three channel modes, and the loudness with its floor. TASK-M8-59 voices it (§5, §8, §9) and TASK-M8-60 extends and displays it (§10 to §12). As first planned: listen (Trajectory panel and Inspector) sonifies the corpus's mapping — θ(t), φ(t) → spectrum → audible range — from the same computeIC result as the trace, following the cursor, cross-fading between hovered ICs and never retriggering per pixel. The audible range is set by mapping 1/t_c to a calibrated reference pitch so frequency ratios hold across ICs. The open choices (θ alone or stereo θ / φ; whole-trajectory or windowed spectrum) are defined in pointer_channels §4; the chaotic anchor IC is defined in trajectory_viewing §1; the minimum trajectory length for a usable spectrum is measured; the spectral-entropy anchors (0.076 figure-eight, 0.177 chaotic) are checked to a calibrated tolerance. The spectrum runs in the inspector worker and costs the frame loop nothing. The artboard's "separations → pitch" selector is not built (R-68).

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
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"
- `docs/design/principia_listen.md` § "1. What is heard"
- `docs/design/principia_listen.md` § "2. The spectrum"
- `docs/design/principia_listen.md` § "3. Pitch"
- `docs/design/principia_listen.md` § "4. Follow the head, or the whole run"
- `docs/design/principia_listen.md` § "6. Channels"
- `docs/design/principia_listen.md` § "7. Loudness"
- `docs/design/principia_listen.md` § "9. Events and effects"
- `docs/design/principia_listen.md` § "13. Tests (each with the arm that fires)"
- `docs/design/principia_listen.md` § "14. What the demo is not"

## Deliverables
- `crates/engine/src/inspector/sonify.rs` — spectrum of θ(t), φ(t), 1/t_c scaling, spectral entropy; runs in the inspector worker. Under R-540: the signal (§1), the spectrum, peaks, fallback, entropy and interval (§2), the pitch (§3), the windows and frames (§4), the channels (§6) and the loudness (§7), as `principia_listen.md` transcribes the demo's code; and §9's four effect inputs, R/R₀, Ṙ t_c/R, H and √(L_z²|E|), computed here for REQ-GUI-313's scale arm, their mappings and the audio graph TASK-M8-59's (code review 5595371779 of PR #267, F3; applied per R-369).
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
- `cargo test -p engine listen_signal` — REQ-GUI-309's verify (R-540 S13 Wrap; Lagrange (default signal)).
- `cargo test -p engine listen_winding_sign` — REQ-GUI-310's verify (R-540 S13 Winding sign).
- `cargo test -p engine listen_spectrum` — REQ-GUI-311's verify (R-540 S13 Figure-eight).
- `cargo test -p engine listen_no_silent_run` — REQ-GUI-312's verify (R-540 S13 No silent run).
- `cargo test -p engine listen_scale_invariance` — REQ-GUI-313's verify (R-540 S13 Scale invariance).
- `cargo test -p engine listen_follow` — REQ-GUI-314's verify (R-540 S13 Silence after the end).
- `cargo test -p engine listen_loudness` — REQ-GUI-316's verify (R-540 S13 Lagrange (angles only)).
- `cargo xtask bench hover-listen-frame` — frame times with continuous hover + listen equal the no-hover baseline within noise; the work runs on the inspector worker (REQ-SYS-060).

## Notes
- R-109, amended by R-144, makes pointer_channels §3 and §4 normative; §3's per-frame-during-motion budget is TASK-M8-09's.
- Calibrations (R-71) proposed here: REQ-GUI-143, REQ-VAL-114. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- Definitions (R-72) written here: REQ-GUI-144, REQ-VAL-113. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- **Waits on RQ-440 and RQ-442** (R-540 H14). The task does not start until they are ruled; their rulings may change this file's deliverables and acceptance (a port of the ruling does so first). Native audio out is RQ-440's backend.
- **⚑ Physics review is required** on R-540 S1 to S3 and S7 (REQ-GUI-309 to REQ-GUI-313, REQ-GUI-316: the signal, the pitch's scale invariance, the floor); it does not merge on the code reviewer's word alone.
- **Applied meanwhile (R-540):** RQ-451 (the default adds the rotation), RQ-450 (the fallback; REQ-VAL-103's measured minimum sets the readout's warning), RQ-453 (REQ-VAL-102 stays a check of θ(t)'s spectrum), RQ-455 and RQ-458 (the code's last-half window and channels), RQ-443 (the selector's row stale), and RQ-441 (a), the analysis on the CPU in the inspector worker, unless it is ruled first. REQ-GUI-144's choices are defined by R-540; this task's doc change to pointer_channels §4 is made (R-540's port), so its doc review checks the build against it. A ruling other than these is a follow-up change.
- The S3 measurements are the demo's, evidence, not values (R-540 H12); the tests measure prin-rs.
- **RQ-465, (a) applied meanwhile** (found merging main, 7daf689b): paused, Follow the head plays the whole run as one chord whatever the trace's length; a `Window` or `LastDt` trace is drawn as chosen (RQ-427 (a), PR #263). A ruling other than (a) is a follow-up change.
- **RQ-467, (a) applied meanwhile** (physics review 5595386853 of PR #267, P2): the code's silences, a bound run with small |E| out of the fitted band and runs too short to analyse, stand; REQ-GUI-312's test does not assert them.
- **PR #267's physics review (applied per R-369):** REQ-GUI-309's Lagrange arm states t ≤ 60 and REQ-GUI-316's horizon waits on RQ-442 (P1); REQ-GUI-310 asserts the side levels (P3); REQ-GUI-313 gains the known-answer arm, 181.94 Hz (P4). RQ-466 (a): listen in the Trajectory panel only, none in the Inspector.
