# TASK-M0-18 — `prin profile`: a fixed scenario headless, and the regression diff

- **Milestone:** M0
- **Closes:** REQ-TOOL-002, REQ-TOOL-006, REQ-TOOL-007
- **Depends on:** TASK-M0-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
`prin profile --scenario NAME --frames N --json PATH` runs a named, fixed, deterministic scenario headless and writes one self-describing, plain-text v1 JSON file — the session header block (with the build hash and the full config) then the frame records (telemetry §5). `prin profile diff BASE NEW --threshold P%` reads two such files and exits non-zero when NEW regresses on BASE by more than P% (render_gui_spec §G5 "For agents, the same data structured").

## References
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"

## Deliverables
- `crates/prin/src/profile/{mod,run,diff}.rs` — the `profile` subcommand (clap), scenario registry, headless run, JSON write; `diff` over per-scope p95.
- Scenario `deep_zoom_03` registered (see Gaps for its content at M0).
- `crates/prin/tests/profile.rs` and trace fixtures.

## Acceptance tests
- `cargo test -p prin profile_file` — the written file parses as JSON against schema v1; the header holds the build hash and the config; `prin profile` reads it back (REQ-TOOL-002; the dev GUI profiler's read is REQ-TOOL-098, M8).
- `cargo test -p prin profile_scenario` — run `deep_zoom_03` for 600 frames twice: the same frame count and the same scope / event sequence (REQ-TOOL-006).
- `cargo test -p prin profile_diff` — the diff of a trace against a copy with one scope's p95 raised 6% exits non-zero at `--threshold 5%` and zero at `--threshold 10%` (REQ-TOOL-007).

## Notes
- One format, one parser, one percentile code: `prin` and, later, the interactive path share the frame record (telemetry §5.5).
- See Gaps: the scenario `deep_zoom_03` and what runs at M0 before any frame loop; which statistic the diff compares; the dev GUI half of REQ-TOOL-002.
