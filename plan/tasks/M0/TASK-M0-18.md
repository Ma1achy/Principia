# TASK-M0-18 — `prin profile`: a fixed scenario headless, and the regression diff

- **Milestone:** M0
- **Closes:** REQ-TOOL-002, REQ-TOOL-006, REQ-TOOL-007, REQ-TOOL-119
- **Depends on:** TASK-M0-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
`prin profile --scenario NAME --frames N --json PATH` runs a registered, fixed, deterministic scenario headless and writes one self-describing, plain-text v1 JSON file — the session header block (with the build hash and the full config) then the frame records (telemetry §5). `prin profile diff BASE NEW --threshold P%` reads two such files and exits non-zero when NEW regresses on BASE by more than P% (render_gui_spec §G5 "For agents, the same data structured").

## References
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/prin/src/profile/{mod,run,diff}.rs` — the `profile` subcommand (clap), scenario registry, headless run, JSON write; `diff` over per-scope p95.
- A synthetic scenario registered (`synthetic_frames`: emits frame records with a fixed scope and event sequence, no physics); `deep_zoom_03` is defined and registered in M5 (REQ-TOOL-130, TASK-M5-28, R-113).
- `crates/prin/tests/profile.rs` and trace fixtures.

## Acceptance tests
- `cargo test -p prin profile_file` — the written file parses as JSON against schema v1; the header holds the build hash and the config as the M0 contract skeleton (TASK-M0-16) serialises it; `prin profile` reads it back (REQ-TOOL-002; the dev GUI profiler's read and the provenance-object header are REQ-TOOL-098's verify, M8).
- `cargo test -p prin profile_scenario` — run the synthetic scenario for N frames twice: the same frame count and the same scope / event sequence; an unregistered scenario name is refused (REQ-TOOL-006).
- `cargo test -p prin profile_diff` — the diff of a trace against a copy with one scope's p95 raised 6% exits non-zero at `--threshold 5%` and zero at `--threshold 10%` (REQ-TOOL-007).
- Definition: the diff's compared statistic, scope set and missing-scope rule written into render_gui_spec § "Profiler" and approved by the physics reviewer (REQ-TOOL-119).

## Notes
- One format, one parser, one percentile code: `prin` and, later, the interactive path share the frame record (telemetry §5.5).
- See Gaps: which statistic the diff compares (REQ-TOOL-119).
- RQ-93 ruled: R-113 — REQ-TOOL-006 is split (M0: a registered synthetic scenario; M5: `deep_zoom_03` for 600 frames) and REQ-TOOL-002's verify is split (M0: schema, header and `prin profile`; M8: the dev GUI profiler, in REQ-TOOL-098).
- Closes, for gaps the corpus leaves open: REQ-TOOL-119 (R-72 definition) (classification accepted by R-132).
