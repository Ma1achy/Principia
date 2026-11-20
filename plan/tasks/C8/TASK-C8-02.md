# TASK-C8-02 — Profiler › Run benchmark…: the dialog, the run, the results sheet and Apply suggested tier

- **Checkpoint:** C8. **After:** the Profiler window (TASK-M8-28); needs the runner (TASK-C8-01).

## Goal
The benchmark from the GUI, as `docs/design/principia_benchmark.md` §5.1 and `docs/gui/principia_render_gui_spec.md`
§G5 › Profiler › "Run benchmark…" describe it: "Profiler window › Run benchmark… opens a short dialog" with the mode
(Quick, Full or Soak, each with its estimated time), the privacy statement and the plug-in prompt; the run on a
fixed-size offscreen surface with the chrome hidden, a progress strip, other input ignored, and Esc writing the file
marked `complete: false`; the results sheet (p50, p95, p99 per scenario and a suggested tier) with "Save file…",
"Copy summary" and "Apply suggested tier" ("Applying is one undo entry"); and the first-launch prompt that offers the
Quick suite once ("nothing runs unless the user accepts").

## Touches
The Profiler window's code (TASK-M8-28's) and a new dialog beside it; the runner from TASK-C8-01, never a second one.

## Tests
The dialog's states and the undo entry, each with a control; Esc's incomplete file; the first-launch prompt shown once.
Screenshot in the PR.
