# TASK-M0-18 — `prin profile`: a fixed scenario headless, and the regression diff

- **Milestone:** M0
- **Closes:** REQ-TOOL-002, REQ-TOOL-006, REQ-TOOL-007, REQ-TOOL-119, REQ-TOOL-139, REQ-TOOL-144, REQ-TOOL-145
- **Depends on:** TASK-M0-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, perf, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
`prin profile --scenario NAME --frames N --json PATH` runs a registered, fixed, deterministic scenario headless and writes one self-describing, plain-text v1 JSON Lines file — the session header on the first line (with the build hash and the full config), then one compact frame record per line (telemetry §5, R-286). The run opens no GPU, so the header writes `backend.api` "none" with the GPU's fields null (R-308), and `config` is `{"scenario", "frames", "sim", "render"}`, `SimConfig` and `RenderState` in their canonical serialisation, which this task defines (R-309, R-72). `prin profile show PATH --pretty` prints a trace indented for reading (R-286). `prin profile diff BASE NEW --threshold P%` reads two such files and exits non-zero when NEW regresses on BASE by more than P% (render_gui_spec §G5 "For agents, the same data structured").

## References
- `docs/gui/principia_render_gui_spec.md` § "Profiler"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "3. Percentiles, not means — and the specific thresholds"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-286 — Profiler traces are JSON Lines: the header, then one compact frame record per line"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-308 — A session that opens no GPU writes `api: "none"`, its GPU fields null *(closes RQ-180)*"
- `decisions.md` § "R-309 — `SimConfig` and `RenderState` have one canonical serialisation; the profiler header's `config` uses it *(closes RQ-181)*"
- `decisions.md` § "R-310 — The "veto?" items on #94 and #95, and physics on TASK-M0-18, stand"

## Deliverables
- `crates/prin/src/profile/{mod,run,diff,show}.rs` — the `profile` subcommand (clap), scenario registry, headless run, JSON Lines write; `diff` over per-scope p95; `show --pretty` (R-286).
- A synthetic scenario registered (`synthetic_frames`: emits frame records with a fixed scope and event sequence, no physics); `deep_zoom_03` is defined and registered in M5 (REQ-TOOL-130, TASK-M5-28, R-113).
- `crates/prin/tests/profile.rs` and trace fixtures.
- `crates/engine/src/contract/{sim_config,render_state}.rs` — `SimConfig`, `RenderState` and their groups serialisable, in the canonical serialisation (R-309).
- `crates/engine/src/contract/profile.rs` and `crates/engine/src/contract/schema/profile_v1.json` (TASK-M0-17's files) — `backend.api` takes "none", and `backend.driver`, `device.gpu`, `device.memory` and `precision` may be null, as telemetry §5 now reads (R-308).
- `docs/contracts/principia_gui_state_contract.md` §2 — the canonical serialisation's key order and number formatting (REQ-TOOL-145, R-72), with the commit's "Removed lines" note.

## Acceptance tests
- `cargo test -p prin profile_file` — the written file is JSON Lines, each line parsing against its schema v1 line type (R-286); the header holds the build hash and the config as the M0 contract skeleton (TASK-M0-16) serialises it: `{"scenario", "frames", "sim", "render"}`, `SimConfig` and `RenderState` in the canonical serialisation (R-309); `prin profile` reads it back (REQ-TOOL-002; the dev GUI profiler's read and the provenance-object header are REQ-TOOL-098's verify, M8).
- `cargo test -p prin profile_scenario` — run the synthetic scenario for N frames twice: the same frame count and the same scope / event sequence; an unregistered scenario name is refused (REQ-TOOL-006).
- `cargo test -p prin profile_diff` — the diff of a trace against a copy with one scope's p95 raised 6% exits non-zero at `--threshold 5%` and zero at `--threshold 10%` (REQ-TOOL-007).
- `cargo test -p prin profile_show` — `prin profile show PATH --pretty` prints each line of a trace indented, and the printed JSON parses to the same values as the file; without `--pretty` it prints the file's lines unchanged (REQ-TOOL-139).
- Definition: the diff's compared statistic, scope set and missing-scope rule written into render_gui_spec § "Profiler" and approved by the physics reviewer (REQ-TOOL-119).
- `cargo test -p prin profile_no_gpu` — the `synthetic_frames` header has `backend.api` "none" and `backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and `precision` null; `prin` has no GPU adapter crate in its dependency tree; the typed form and `profile_v1.json` accept that header and still reject an `api` outside the five values (REQ-TOOL-144, R-308).
- Definition: the canonical serialisation's key order and number formatting written into gui_state_contract §2 and approved by the physics reviewer; serialising the same `SimConfig` and `RenderState` twice gives the same bytes, and a written number reads back to the same value (REQ-TOOL-145, R-309).

## Notes
- One format, one parser, one percentile code: `prin` and, later, the interactive path share the frame record (telemetry §5.5).
- See Gaps: which statistic the diff compares (REQ-TOOL-119).
- RQ-93 ruled: R-113 — REQ-TOOL-006 is split (M0: a registered synthetic scenario; M5: `deep_zoom_03` for 600 frames) and REQ-TOOL-002's verify is split (M0: schema, header and `prin profile`; M8: the dev GUI profiler, in REQ-TOOL-098).
- Closes, for gaps the corpus leaves open: REQ-TOOL-119 (R-72 definition) (classification accepted by R-132).
- RQ-180 ruled: R-308 — a session that opens no GPU writes `api: "none"`, its GPU fields null; never open an adapter
  just to fill the header. RQ-181 ruled: R-309 — the skeleton becomes serialisable, `config` is
  `{"scenario", "frames", "sim", "render"}`, in one canonical serialisation that snapshot JSON, share links and pxpack
  share (they are held to it by REQ-TOOL-146, TASK-M8-32).
- R-310: physics joins the Reviewers, because REQ-TOOL-119's and REQ-TOOL-145's definitions need the physics reviewer's
  approval (R-72).
- Size: the two rulings add the contract files' serde derives, the header's no-GPU form in TASK-M0-17's files and a
  definition; the ~400-line estimate is the orchestrator's to weigh at the PR (R-264).
