# TASK-M6-24 — The GUI track (R-390), 1: the mock engine, the contract conformance suite and the app shell

- **Milestone:** M6
- **Closes:** REQ-GUI-165, REQ-GUI-166, REQ-GUI-167, REQ-GUI-168, REQ-GUI-075, REQ-GUI-162, REQ-GUI-176, REQ-GUI-177
- **Depends on:** TASK-M0-16, TASK-M0-20
- **Needs (earlier milestones):** REQ-SYS-002, REQ-SYS-003, REQ-TOOL-134
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~1,000 lines, the pre-flight's decisions (RQ-243 to RQ-256) included

## Goal
The first task of the GUI track (R-390): a dev GUI the human runs on the Mac with `cargo run -p gui --features mock`, on
a mock engine. The engine crate's contract gains the interface the GUI calls (`set_field`, the snapshot, undo and redo as
requests, the events), and the real engine implements it as far as the conformance suite reaches: its first fields are
the playhead `t`, the history's undo and redo depths and an optional GUI-sized frame summary (RQ-243), and its events are
log entries the snapshot carries, adding no membrane crossing (RQ-245). The gui crate holds the mock engine, a test
double of that contract: it serves plausible GUI-sized snapshots, applies `SetField` with undo and redo (R-69), emits
log entries in its snapshots, supplies the deterministic tick the app's clock reads to play Time (RQ-246), and renders the
figure's stand-in, smooth procedural noise, through a canvas trait kept apart from the data contract, on the device and
queue it hands the app (RQ-247). One conformance suite, defined once in the engine crate, runs against both engines. The
app shell looks, feels and behaves as `01_main.png` and the design notes give it: the window, F3 hiding and showing the
egui layer, egui's dark theme with Ubuntu and Ubuntu Mono, the top bar, the footer with the "mock engine" tag and the
console it opens, and the Explore page's regions, with nothing over the figure. gui's headless capture mode (R-274) runs
the app on the mock, so `cargo xtask screenshot` reaches every track screen.

## References
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "12 Console"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_systems_architecture.md` § "7.1 Crate map"
- `decisions.md` § "R-52 — Undo lives in the state contract *(GU-1 (a))*"
- `decisions.md` § "R-54 — The precision warning is event-driven *(GU-3 (b))*"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"
- `decisions.md` § "R-185 — The crate map is confirmed; kernel → ledger is a build-dependency only *(closes TASK-M0-00)*"
- `decisions.md` § "R-187 — kernel and ledger may take validation as a dev-dependency; validation never depends on prin *(closes RQ-129)*"
- `decisions.md` § "R-206 — An unset `PRIN_GPU_BACKEND` defaults by platform"
- `decisions.md` § "R-274 — The screenshot runner reaches `gui` through a headless capture mode it spawns *(closes RQ-166)*"
- `decisions.md` § "R-275 — A control clipped out of the visible surface isn't present *(closes RQ-167)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `decisions.md` § "R-309 — `SimConfig` and `RenderState` have one canonical serialisation; the profiler header's `config` uses it *(closes RQ-181)*"
- `decisions.md` § "R-318 — The canonical serialisation is JCS (RFC 8785) *(amends R-309)*"
- `decisions.md` § "R-329 — The header's core count is `cpu_cores_available`; no `usize` in a serialised type"

- `decisions.md` § "R-396 — Physics reviews TASK-M6-24 for REQ-GUI-176 and REQ-GUI-177 only *(amends R-390)*"
## Deliverables
- `crates/engine/src/contract/`: the interface the GUI calls, as a trait — apply a `SetField`, read the latest
  snapshot, request undo and redo; the events are read from the snapshot — as plain data (gui_state_contract §1), with the real engine's
  implementation: a type holding a `SimConfig` and a `RenderState`, the history and the log entries since the last snapshot, built by a
  constructor that takes the initial state explicitly, since the surfaces carry no default (RQ-254), with the playhead at
  `t = 0.0` (RQ-244 as amended). The fields it needs,
  as gui_state_contract §2 names them (R-390's "Contract fields"; RQ-243): `RenderField::Playhead` setting
  `Playhead { t: f64 }`; `History { undo_depth: u32, redo_depth: u32 }`; the snapshot's frame summary `frame_ms`, `fps`,
  `quad_count` and `live_memory { heap_bytes, gpu_bytes }`, each an `Option`, `None` from the real engine until
  TASK-M8-05 wires its frame loop; the log entry, with the snapshot carrying the entries accumulated since the
  previous snapshot (RQ-245 as amended per code review 5438674593). A second, separate engine-side
  trait for the canvas (the device and queue, and drawing the figure into the app's pass), §1's sanctioned exception,
  outside the data contract and the conformance suite; the real engine's implementation of it is TASK-M8-05's (RQ-247).
  Everything sits under `engine::contract`, so TASK-M8-04's `pub` surface covers it.
- `crates/engine/src/contract/conformance.rs` (or a module of that name): the one conformance suite, `pub` (not
  `cfg(test)`), generic over the interface, with the case list and a runner that names the failing case; engine's tests
  run it against the real engine. Its first cases are state semantics both engines run without a frame loop: a
  `SetField` shows in the next snapshot; undo and redo restore and reapply it; a no-history edit leaves the history
  unchanged; each applied `SetField` logs one `info` entry from `contract`, seen in the next snapshot (RQ-254, RQ-245).
- Doc changes (R-72), written into `docs/contracts/principia_gui_state_contract.md` § 2 beside the fields: the frame
  summary's definitions (REQ-GUI-176: `quad_count` is the frame record's `leaf_count` at the latest frame; `fps` is
  1000 / the mean `frame_ms` of the frames since the previous snapshot; the footer's GPU and heap are `live_memory`'s
  `gpu` and `heap` bytes, the tile cache not added) and the log entry (REQ-GUI-177: `{ severity: error | warn | info,
  at, source: stain | integrator | quadtree | contract | app, message }`, plain data, carried GUI-sized in the snapshot as
  the entries accumulated since the previous snapshot, which the GUI reads from each snapshot it receives, so no
  membrane crossing is added (systems_architecture §6 invariant 10); R-54's flags stay in the snapshot; both engines
  log each applied `SetField` as an `info` entry from `contract`; the footer counts warnings and errors since the
  session began, until TASK-M6-28's "clear" resets them). Schema v1 is unchanged.
- `crates/gui/src/mock/`: the mock engine, always compiled, with the capture path, `MockSide` and `cli::mock`, the
  `mock` feature choosing only which engine `main` runs (RQ-252 as amended per R-369); the fake clock's time
  source, a deterministic tick the app's clock reads, frozen at a fixed `t` in capture mode, its rate and `dt` named
  mock constants (placeholder content, not corpus values; RQ-246); the mock reads no `ViewUI`; its implementation of the
  canvas trait and its stand-in figure (smooth procedural noise, nothing from `workbench/`); a way for tests and the
  capture mode to make it raise a warning and an error.
- `crates/gui/src/main.rs` and `crates/gui/src/app.rs`: the app, in an `eframe` 0.36 window (matching egui 0.36.2) on its
  wgpu renderer; with `--features mock` it runs on the mock. The app is generic over the engine; the engine-side adapter
  in `gui` that constructs the engine says whether it is the mock, and the tag reads that, not a snapshot field
  (RQ-247). The GUI's clock reads `ViewUI`'s transport and advances the playhead through `SetField { Playhead,
  no_history: true }` (R-101, RQ-246). egui-wgpu is built from the device and queue the engine side provides (here the
  mock), as REQ-GUI-070 requires of the real engine. With F3 off the figure keeps the central rect 01_main.png gives it
  and the rest of the window is the clear colour; the window title reads "principia · dev — mock engine" on the mock
  (RQ-248).
- `crates/gui/src/theme.rs` and the fonts under `crates/gui/assets/fonts/`: text keeps egui's own Ubuntu Light; Ubuntu
  Mono Regular, from the Ubuntu font family's upstream release, committed with its UFL 1.0 licence text and a note
  giving its source and SHA-256, is egui's `Monospace` family, used by numeric fields, readouts and the status line
  (RQ-251).
- `crates/gui/src/explore/{top_bar,footer}.rs`: the top bar's status line, the spec's full list in the artboard's format
  and order ("t 12.40 · 60 fps · 4.1 ms · 1 842 quads · undo 2 / redo 0 · F3 hide"), filled from the mock's snapshot,
  the undo and redo depths from the mock's history, "—" for an absent value; the "principia · dev" wordmark as the
  artboard shows it; and the footer's warning and error counts, latest message, memory readout (GPU, heap) and "? keys"
  hint filled from the mock's snapshot and events (applied per R-369, review 5434766412 on PR #157), with the "mock
  engine" tag in the footer's right group before the memory readout, in egui's weak text colour, and no
  passive-logging indicator while logging is off (RQ-255). Every top-bar control 01_main.png shows is drawn where it
  shows it (RQ-249): the menus open (Windows lists the §G5 windows and Console by their spec names, each disabled until
  its task; View holds "F3 hide"; File holds Quit; Help holds "Keys (?)", disabled until TASK-M6-25); the mode switch
  changes a `ViewUI` mode and shows an empty Stain page frame; Overlays ▾ shows its count from the snapshot (0 on the
  mock) and opens an empty menu; Run…, Profiler… and Export… are drawn disabled; the breadcrumb's slot is empty. A
  footer click opens a console window frame listing the entries as plain rows of severity, time, source and message;
  its layout, filters, copy and clear, and opening on an error, are TASK-M6-28's. The Explore page's regions (left
  Manifold view, figure, right Trajectory, bottom compass, Time and Legend) are empty frames that later track tasks
  fill.
- gui's headless capture mode (R-274): an entry point the screenshot runner spawns, which runs the app on the mock,
  renders a named screen offscreen and writes `capture.png` and a names file with each AccessKit name's rect, for R-275;
  the capture is the artboard's 2160 × 1350 pixels, at a pixels-per-point the gui reviewer sets to match the artboard's
  text, recorded in the PR. The runner's `gui` surface kind (RQ-253): a case's `surface` stays a path for a `data`
  case, so `selftest` and `qa_TASK-M0-20` are unchanged, and may instead be an object `{ "kind": "gui", "screen":
  "01_main", "steps": [...] }`, whose steps are a closed list (`f3`, `raise_warning`, `raise_error`, `click_footer`);
  the runner spawns `cargo run --quiet -p gui --features mock -- capture --screen … --steps … --out <case dir>`, as
  `gate` spawns validation's binary, and reads the PNG and the names back. No crate depends on `gui`.
- CI and configuration for the `mock` feature (RQ-252 as amended per R-369, option (c)): none. The mock is always
  compiled, so qa's tests in `crates/gui/tests/`, clippy and cargo-mutants reach it without a flag;
  `.github/workflows/ci.yml` changes no line, `.cargo/mutants.toml` gains no `features` key, and no qa pin on ci.yml's
  text changes. `.config/nextest.toml` puts `screenshot_gui_surface` in the `ci-workspace` profile, and `xtask/tests/nextest.rs`
  still checks that the profiles together run every test. gui's GPU tests read `PRIN_GPU_BACKEND` as R-206 gives it
  and log the backend chosen.
- Screenshot cases `01_main/mock_shell`, `01_main/mock_f3_off`, `01_main/mock_footer`, `01_main/mock_warning`.
- Tests: `mock_engine`, `conformance` (in engine and in gui), `mock_tag`, `f3_toggle_mock`, `mock_status_line`.

## Acceptance tests
- `cargo test -p gui mock_engine` — a SetField shows in the next snapshot; undo and redo restore and reapply it; an edit marked no history leaves the history unchanged; the fake clock, run through the app's clock on the mock's tick, advances the playhead through no-history SetFields while playing and holds it while paused, and the mock reads no `ViewUI` (RQ-246); events arrive only in the snapshot, as the entries since the previous snapshot, one `contract` info entry per applied SetField (RQ-245 as amended). The PR shows `cargo run -p gui --features mock` opening the app window (REQ-GUI-165).
- `cargo test -p engine conformance` and `cargo test -p gui conformance` — the same case list from the one definition (the four state-semantics cases, RQ-254), run against the real engine and the mock, both pass with no case skipped; a deliberately non-conforming double fails it, naming the case (REQ-GUI-166).
- `cargo xtask screenshot 01_main` (mock_footer) and `cargo test -p gui mock_tag` — the footer on the mock shows the "mock engine" tag beside 01_main.png's footer; `mock_tag` runs the app headless once on the mock and once on the real engine's data contract with no figure, and the tag is in the AccessKit names exactly when the engine is the mock (REQ-GUI-167).
- `cargo xtask screenshot 01_main` (mock_shell, mock_f3_off, mock_warning) and `cargo test -p gui f3_toggle_mock` — against 01_main.png with F3 on and off, the stand-in figure identical underneath: the two captures are pixel-identical over the figure's rect (RQ-248); a raised warning and error change the footer's counts and nothing over the figure; a footer click opens the console window frame; the gui reviewer checks the design notes' global rules (REQ-GUI-168).
- `cargo xtask screenshot 01_main` (mock_shell) — screenshot against 01_main.png: dark theme, Ubuntu for text, Ubuntu Mono for numbers/code (REQ-GUI-075).
- `cargo test -p gui mock_status_line` and `cargo xtask screenshot 01_main` (mock_shell, mock_footer) — the top bar's status line shows the mock snapshot's `t`, fps, frame ms and quad count in the artboard's format, and its undo depth reads 2 after two edits on the mock and 1 after an undo, with the redo depth 1; the footer shows the mock's memory readout and the "? keys" hint; against 01_main.png's top bar and footer (REQ-GUI-168).
- `cargo test -p xtask screenshot_gui_surface` and `cargo xtask deps` — a screenshot case whose `surface` is a `gui` object spawns the capture mode for a named screen with its steps and gets its PNG and names with rects back; a `data` case is unchanged; `cargo xtask deps` shows no edge into gui (REQ-GUI-162).
- Review checklist (physics reviewer) of the frame summary's definition — gui_state_contract §2 defines `quad_count`, `fps` and the footer's GPU and heap against the frame record; the mock's snapshot, the status line and the footer follow it (REQ-GUI-176).
- Review checklist (physics reviewer) of the log entry's definition — gui_state_contract §2 gives the entry's shape and that the snapshot carries the entries since the previous snapshot; the conformance suite checks one `contract` info entry per applied SetField in the next snapshot on both engines; the footer's counts follow it (REQ-GUI-177).

## Notes
- **Fetches (dispatch requirement; RQ-251 as amended per R-369).** The implementer may fetch two things and nothing
  else: `eframe` 0.36 through cargo (R-349 counts cargo filling its registry as a build's own cache write) and the
  Ubuntu Mono release from the Ubuntu font family's upstream, committing the upstream release URL, the UFL 1.0 licence
  text and the SHA-256 beside the font (a repo change). Neither waits for the human.
- R-390, the GUI track: this task and TASK-M6-25 to TASK-M6-29 run in parallel with the physics and renderer chain, on
  spare agent slots (`plan/OPERATIONS.md` § "The GUI track (R-390)"), and merge without waiting for the gates of M1 to
  M5 (`plan/WORKFLOW.md` § "Human checkpoints: the milestone gates"). It sits in M6 because TASK-M6-21 and TASK-M6-22
  build on it.
- **What to try.** The PR description has a "What to try" section for the human: the clicks and keys that show what
  changed, on `cargo run -p gui --features mock` (here: F3, a footer click, the mode switch, the menus). It lists only
  what works (RQ-249).
- **Sources (R-390):** `decisions.md`, then the design notes, then the artboards, then render_gui_spec, then
  gui_state_contract; R-68 still makes corpus values win over artboard values. A look, feel or behaviour they leave
  open is decided and recorded in the PR as "applied per R-369". RQ-248 (F3 off and the window title), RQ-251 (the
  fonts) and RQ-255 (the status line and footer) are such decisions, and the human may revise them through GUI design
  rulings.
- REQ-GUI-075 moved here from TASK-M8-05, and REQ-GUI-162 from TASK-M6-22 (R-390); both tasks depend on this one.
  TASK-M8-05 re-runs REQ-GUI-075's acceptance on the real engine.
- The firewall (the engine's internals crate-private, the compile-fail test, `cargo xtask lint-gui`) stays TASK-M8-04's;
  REQ-GUI-070's egui-wgpu on the real engine's device stays TASK-M8-05's. This task draws the whole top bar and footer
  on the mock: the status line (`t`, fps, frame ms, quad count, undo / redo depth) from the mock's snapshot, with the
  undo depth from the mock's history, and the footer's memory readout and "? keys" hint; only their real data source,
  the real engine's snapshot (with `budget-bound`, REQ-GUI-078, the mock never binding), stays TASK-M8-05's, which
  wires the frame summary and may revise REQ-GUI-176's definition through the porting rule (RQ-243). This task keeps to
  both tasks' rules already: the GUI reads snapshots and emits SetFields only, and names no camera and no "Fate".
- The contract's history here is what the conformance suite needs (apply, undo, redo, no-history edits); the full
  undoable set, coalescing and the blast-radius metadata stay TASK-M8-03's, which depends on this task. Cases that need
  the frame loop (the clock, fps, quads, the figure) join the suite only when the real engine has one (R-390).
- **The canonical text changes (RQ-244, as amended per code review 5438674593).** An explicit initial state gives the
  playhead `t = 0.0`, so RenderState's JCS text gains `"playhead":{"t":0}` (R-309, R-318: 0.0 is written `0`).
  - The implementer: `crates/prin/src/profile/run.rs`:231 builds `playhead: Playhead {},` in the synthetic run and
    becomes `Playhead { t: 0.0 }`; the implementer's pins follow (`crates/engine/src/contract/tests/canonical.rs`:438,
    :527, :665; `crates/prin/tests/profile.rs`:144, :305, :306 (the profile header's `config`), :403).
  - qa, under R-290 (every earlier commit to each file is a qa commit; `M` lines, as `a7496d6` changed both the struct
    literal and the text): `crates/engine/tests/qa_TASK-M0-18.rs`:41 (`playhead: Playhead {},` becomes
    `Playhead { t: 0.0 }`) and :49 (the pinned text), and `crates/prin/tests/qa_TASK-M0-18.rs`:165.
  - The implementer's head: the surfaces have no `Default` (qa_TASK-M0-16), so the engine qa_TASK-M0-18 target fails to
    compile until qa's commit, and CI's `cargo nextest archive --workspace` and `cargo clippy --workspace
    --all-targets` are red there; qa's commit follows the implementer's push, as on PR #160, and the PR says so.
  - The code reviewer confirms that the struct literals gain `t: 0.0` and the expected texts gain `"t":0`, and nothing
    else changes.
- **The gui reviewer's § 8 here** (RQ-250): the reviewer checks the shell's own controls and marks (the top bar, the
  footer, the regions' placement, F3, the theme and fonts); the keyboard item applies from TASK-M6-25 on, and the
  artboard-marks item to each mark's own task.
- **physics** is added only to approve the two R-72 definitions, REQ-GUI-176 and REQ-GUI-177: R-72 says a missing
  definition is "reviewed by the physics reviewer before merging", and `plan/WORKFLOW.md` § "Human checkpoints: the
  milestone gates" requires every definition requirement's doc change to merge with the
  physics reviewer's approval. R-390's three reviewers, code, qa and gui, cover everything else. This is the gate rule
  applied alongside R-390 ("Reviewers: code, qa and gui"), flagged for the human (applied per R-369); it changes
  nothing that gets built.
- **R-396** settles it: physics reviews this task for REQ-GUI-176 and -177 only, and code, qa and gui review everything
  else. `plan/tasks.yaml` cites R-396 beside the reviewer list.
- Carrying the log entries in the profiler's schema-v1 stream stays TASK-M8-27's (REQ-GUI-126 on the real engine).
- Pre-flight (R-388): RQ-243 to RQ-256, decided per R-369 (7 Oct 2026), shape this file's Goal, Deliverables,
  acceptance lines, Notes and References.
- Size: the mock, the suite, the shell, the capture mode and the CI edits are one reviewable step, the ruling's first
  ORDER item; it grew with the pre-flight's decisions (RQ-243 to RQ-256).
- R-406 (10 Oct 2026) revises RQ-248's first decision: with F3 off the figure fills the window, showing more of the
  field at the same scale, each point where the shown layout puts it, so the captures stay pixel-identical over the
  shown figure's rect and the rest of the window is no longer the clear colour (REQ-GUI-179, TASK-M6-30). The window
  title's "mock engine" stands.
