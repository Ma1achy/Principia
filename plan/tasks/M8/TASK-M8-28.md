# TASK-M8-28 — Profiler window, leak detector, precomputed summaries and `prin profile query --live` (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-098, REQ-TOOL-099, REQ-TOOL-100, REQ-TOOL-101, REQ-TOOL-114, REQ-TOOL-115, REQ-TOOL-128
- **Depends on:** TASK-M8-26, TASK-M8-27
- **Needs (earlier milestones):** REQ-TOOL-002, REQ-TOOL-005, REQ-TOOL-006, REQ-TOOL-007, REQ-TOOL-008, REQ-TOOL-050, REQ-TOOL-051, REQ-TOOL-053
- **Reviewers:** code, qa, physics, gui, perf
- **Pitfalls:** PIT-3
- **Size:** ~500 lines

## Goal
The Profiler window has Timeline, Flame, GPU, Memory and Counters tabs with live / pause / Capture, and shows §G5's views (frame-time trace with p50 / p95 / p99 / worst, the donut, stacked bars for 60 frames, the substeps-per-pixel histogram with the cap marked at N_max, the main-thread flame chart, GPU timestamps per pass, memory over time, live allocations with their 60 s change) and the Export trace, Open in Tracy and Headless render… buttons. The leak detector flags steady memory growth while idle, with "idle" and the hot-path summary defined in §G5 and the threshold and window calibrated. The exported JSON carries precomputed leak flags and hot-path summaries within schema v1. `prin profile query "…" --live` queries a running app and answers in the same schema.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
## Deliverables
- `crates/gui/src/windows/profiler/{timeline,flame,gpu,memory,counters}.rs`.
- `crates/engine/src/telemetry/{leak,hot_path}.rs` — the detector and summaries written into the v1 export.
- `crates/prin/src/cmd/profile_query.rs` + an engine-side query endpoint.
- Doc change: `docs/gui/principia_render_gui_spec.md` § "Profiler" (idle; the hot-path summary's fields within schema v1).
- Calibration proposal: idle traces with and without an injected leak; the threshold and window.

## Acceptance tests
- `cargo xtask screenshot 04_windows` (Profiler; histogram cap marker at N_max) — screenshot against 04_windows.png's profiler; the histogram's cap marker sits at N_max; `cargo test -p gui profiler_reads_prin_file` — the window reads the profiler file `prin profile` writes, and that file's header config is REQ-GUI-039's provenance object (R-113) (REQ-TOOL-098).
- `cargo test -p engine leak_detector` (threshold and window: REQ-TOOL-115, calibrated) — synthetic idle trace with steady heap growth is flagged; a flat idle trace and growth during activity are not (REQ-TOOL-099).
- `cargo test -p engine profile_summaries` — an exported trace contains leak-flag and hot-path summary sections consistent with its raw scopes (REQ-TOOL-100).
- `cargo test -p prin profile_query_live` — 'top 10 scopes by p95' against a running app returns ten v1-schema scope records (REQ-TOOL-101).
- Doc review of `docs/gui/principia_render_gui_spec.md` § "Profiler" — the Profiler section defines idle and lists the hot-path summary's fields within profiler schema v1; the physics reviewer approves the doc change before merge (REQ-TOOL-114).
- `cargo xtask gate leak-detector` — the proposal shows idle memory traces with and without an injected leak and a threshold and window that flag the leak and not the clean trace; a reviewer checks the proposal and the human confirms the value at the M8 gate, then it is recorded in `decisions.md` (REQ-TOOL-115).
- Definition: the `--live` transport and discovery written into render_gui_spec's Profiler section and approved by the physics reviewer (REQ-TOOL-128).

## Notes
- How `--live` reaches a running app (the transport) is not given.
- Calibrations (R-71) proposed here: REQ-TOOL-115. Each value is confirmed by the human at the M8 gate; an unconfirmed one blocks the gate.
- Definitions (R-72) written here: REQ-TOOL-114. Each doc change carries the porting rule's "Removed lines" note and the physics reviewer's approval.
- RQ-93 ruled: R-113 — REQ-TOOL-002's dev-GUI half (the profiler reading the same file, its header config REQ-GUI-039's provenance object) is verified here by REQ-TOOL-098.
- Closes, for gaps the corpus leaves open: REQ-TOOL-128 (R-72 definition) (classification accepted by R-132).
