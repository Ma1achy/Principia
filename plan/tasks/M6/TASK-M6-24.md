# TASK-M6-24 — The GUI track (R-390), 1: the mock engine, the contract conformance suite and the app shell

- **Milestone:** M6
- **Closes:** REQ-GUI-165, REQ-GUI-166, REQ-GUI-167, REQ-GUI-168, REQ-GUI-075, REQ-GUI-162
- **Depends on:** TASK-M0-16, TASK-M0-20
- **Needs (earlier milestones):** REQ-SYS-002, REQ-SYS-003, REQ-TOOL-134
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~700 lines

## Goal
The first task of the GUI track (R-390): a dev GUI the human runs on the Mac with `cargo run -p gui --features mock`, on
a mock engine. The engine crate's contract gains the interface the GUI calls (`set_field`, the snapshot, undo and redo as
requests, the events), and the real engine implements it as far as the conformance suite reaches. The gui crate holds the
mock engine, a test double of that contract: it serves plausible GUI-sized snapshots, applies `SetField` with undo and
redo (R-69), emits events through the contract's own channels, runs a fake clock, and renders the figure's stand-in,
smooth procedural noise, on the device and queue it hands the app. One conformance suite, defined once in the engine
crate, runs against both engines. The app shell looks, feels and behaves as `01_main.png` and the design notes give it:
the window, F3 hiding and showing the egui layer, egui's dark theme with Ubuntu and Ubuntu Mono, the top bar, the footer
with the "mock engine" tag and the console it opens, and the Explore page's regions, with nothing over the figure. gui's
headless capture mode (R-274) runs the app on the mock, so `cargo xtask screenshot` reaches every track screen.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `decisions.md` § "R-274 — The screenshot runner reaches `gui` through a headless capture mode it spawns *(closes RQ-166)*"
- `decisions.md` § "R-275 — A control clipped out of the visible surface isn't present *(closes RQ-167)*"

## Deliverables
- `crates/engine/src/contract/`: the interface the GUI calls — apply a `SetField`, read the latest snapshot, request undo
  and redo, receive events — as plain data (gui_state_contract §1), with the real engine's implementation; the fields it
  needs added as the corpus names them (R-390's "Contract fields").
- `crates/engine/src/contract/conformance.rs` (or a module of that name): the one conformance suite, generic over the
  interface, reachable by gui's tests; engine's tests run it against the real engine.
- `crates/gui/src/mock/`: the mock engine, compiled under the `mock` feature and in gui's tests; its fake clock; its
  stand-in figure (smooth procedural noise, nothing from `workbench/`); a way for tests and the capture mode to make it
  raise a warning and an error.
- `crates/gui/src/main.rs` and `crates/gui/src/app.rs`: the app; with `--features mock` it runs on the mock. egui-wgpu is
  built from the device and queue the engine side provides (here the mock), as REQ-GUI-070 requires of the real engine.
- `crates/gui/src/theme.rs` and the fonts under `crates/gui/assets/fonts/`; `crates/gui/src/explore/{top_bar,footer}.rs`:
  the top bar's status line (`t`, fps, frame ms, quad count, undo / redo depth, "F3 hide") filled from the mock's
  snapshot, the undo depth from the mock's history, and the footer's warning and error counts, latest message, memory
  readout (GPU, heap) and "? keys" hint filled from the mock's snapshot and events (applied per R-369, review
  5434766412 on PR #157); and the Explore page's regions (left Manifold view, figure, right Trajectory, bottom compass, Time and Legend) as empty
  frames that later track tasks fill.
- gui's headless capture mode (R-274): an entry point the screenshot runner spawns, which runs the app on the mock,
  renders a named screen offscreen and writes the PNG and its AccessKit names, with rects for R-275; the runner's `gui`
  surface kind that spawns it. No crate depends on `gui`.
- Screenshot cases `01_main/mock_shell`, `01_main/mock_f3_off`, `01_main/mock_footer`, `01_main/mock_warning`.
- Tests: `mock_engine`, `conformance` (in engine and in gui), `mock_tag`, `f3_toggle_mock`, `mock_status_line`.

## Acceptance tests
- `cargo test -p gui mock_engine` — a SetField shows in the next snapshot; undo and redo restore and reapply it; an edit marked no history leaves the history unchanged; the fake clock advances the playhead while playing and holds it while paused; events arrive only through the contract's channels. The PR shows `cargo run -p gui --features mock` opening the app window (REQ-GUI-165).
- `cargo test -p engine conformance` and `cargo test -p gui conformance` — the same case list from the one definition, run against the real engine and the mock, both pass with no case skipped; a deliberately non-conforming double fails it, naming the case (REQ-GUI-166).
- `cargo xtask screenshot 01_main` (mock_footer) and `cargo test -p gui mock_tag` — the footer on the mock shows the "mock engine" tag beside 01_main.png's footer; the tag is drawn exactly when the engine is the mock (REQ-GUI-167).
- `cargo xtask screenshot 01_main` (mock_shell, mock_f3_off, mock_warning) and `cargo test -p gui f3_toggle_mock` — against 01_main.png with F3 on and off, the stand-in figure identical underneath; a raised warning and error change the footer's counts and nothing over the figure; a footer click opens the console; the gui reviewer checks the design notes' global rules (REQ-GUI-168).
- `cargo xtask screenshot 01_main` (mock_shell) — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075).
- `cargo test -p gui mock_status_line` and `cargo xtask screenshot 01_main` (mock_shell, mock_footer) — the top bar's status line shows the mock snapshot's `t`, fps, frame ms and quad count, and its undo depth reads 2 after two edits on the mock and 1 after an undo; the footer shows the mock's memory readout and the "? keys" hint; against 01_main.png's top bar and footer (REQ-GUI-168).
- `cargo test -p xtask screenshot_gui_surface` and `cargo xtask deps` — a screenshot case with surface kind `gui` spawns the capture mode for a named window and gets its PNG and names back; `cargo xtask deps` shows no edge into gui (REQ-GUI-162).

## Notes
- R-390, the GUI track: this task and TASK-M6-25 to TASK-M6-29 run in parallel with the physics and renderer chain, on
  spare agent slots (`plan/OPERATIONS.md` § "The GUI track (R-390)"), and merge without waiting for the gates of M1 to
  M5 (`plan/WORKFLOW.md` § "Human checkpoints: the milestone gates"). It sits in M6 because TASK-M6-21 and TASK-M6-22
  build on it.
- **What to try.** The PR description has a "What to try" section for the human: the clicks and keys that show what
  changed, on `cargo run -p gui --features mock` (here: F3, a footer click, the mode switch, the menus).
- **Sources (R-390):** `decisions.md`, then the design notes, then the artboards, then render_gui_spec, then
  gui_state_contract; R-68 still makes corpus values win over artboard values. A look, feel or behaviour they leave
  open is decided and recorded in the PR as "applied per R-369".
- REQ-GUI-075 moved here from TASK-M8-05, and REQ-GUI-162 from TASK-M6-22 (R-390); both tasks depend on this one.
  TASK-M8-05 re-runs REQ-GUI-075's acceptance on the real engine.
- The firewall (the engine's internals crate-private, the compile-fail test, `cargo xtask lint-gui`) stays TASK-M8-04's;
  REQ-GUI-070's egui-wgpu on the real engine's device stays TASK-M8-05's. This task draws the whole top bar and footer
  on the mock: the status line (`t`, fps, frame ms, quad count, undo / redo depth) from the mock's snapshot, with the
  undo depth from the mock's history, and the footer's memory readout and "? keys" hint; only their real data source,
  the real engine's snapshot (with `budget-bound`, REQ-GUI-078), stays TASK-M8-05's. This task keeps to both tasks' rules
  already: the GUI reads snapshots and emits SetFields only, and names no camera and no "Fate".
- The contract's history here is what the conformance suite needs (apply, undo, redo, no-history edits); the full
  undoable set, coalescing and the blast-radius metadata stay TASK-M8-03's, which depends on this task.
- Size: the mock, the suite, the shell and the capture mode are one reviewable step, the ruling's first ORDER item.
