# TASK-M8-43 — The compute fast-math setting in the GUI: the Run window control and the Profiler's per-stage modes

- **Milestone:** M8
- **Closes:** REQ-GUI-163, REQ-GUI-164
- **Depends on:** TASK-M8-24, TASK-M8-28, TASK-M4-08
- **Needs (earlier milestones):** REQ-SCHED-097, REQ-TOOL-141
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
The compute shaders' fast-math setting is on the sim key and shown in the Profiler and the Run window (R-297). The Run
window shows it as a control under its `SimConfig` name, off by default; changing it is marked as re-integrating, and
while it is on the window says that parity is measured, not exact. The Profiler shows each shader stage's fast-math
mode, compute, vertex and fragment, from the session header. Neither has an artboard, so both are checked by presence
only (R-129).

## References
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"

## Deliverables
- `crates/gui/src/windows/run.rs` — the compute fast-math control, its re-integrate marking and the measured-not-exact
  note.
- `crates/gui/src/windows/profiler/` — the per-stage fast-math line, read from the session header.
- Tests `run_fast_math` and `profiler_fast_math`; screenshot cases `04_windows/run_fast_math` and
  `04_windows/profiler_fast_math`.
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo xtask screenshot 04_windows` (run_fast_math) and `cargo test -p gui run_fast_math` — the control is present in the Run window, reading off by default (presence only, no artboard, R-129); toggling it emits one SetField on SimConfig, is marked re-integrating, and shows the measured-not-exact note while it is on (REQ-GUI-163).
- `cargo xtask screenshot 04_windows` (profiler_fast_math) and `cargo test -p gui profiler_fast_math` — the three modes appear in the Profiler (presence only, R-129); a session-header fixture with compute on shows compute on, and one with it off shows it off (REQ-GUI-164).

## Notes
- TASK-M8-42's screen sweep, which checks every schema field is exposed, depends on this task.
