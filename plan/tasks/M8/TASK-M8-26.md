# TASK-M8-26 — Telemetry: the full calibration record, binding resource, throttling inference, passive mode

- **Milestone:** M8
- **Closes:** REQ-TOOL-092, REQ-TOOL-089, REQ-TOOL-090, REQ-TOOL-088
- **Depends on:** TASK-M8-05, TASK-M5-24
- **Needs (earlier milestones):** REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-050, REQ-TOOL-052, REQ-TOOL-057, REQ-SCHED-044, REQ-PERF-050
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The calibration record carries every field telemetry § "Collect everything relevant, in one file" lists beyond the frame record and session block (device incl. unified vs discrete and memory, os incl. thermal state, power and low-power mode, runtime limits, per-frame memory pressure, load incl. backgrounding, cores vs pool and ceiling, in-flight queue depth and overrun dispatches, the thermal signal, binding axis per frame, and slice with sea_fraction). Each run names one binding resource (samples, bandwidth, memory or dispatch overhead). Any exposed throttling signal is logged, else throttling is inferred from a monotone fps decline at constant settings and scene, and tiers are derived from sustained performance. Passive telemetry is an explicit opt-in mode, switched in the Profiler, with a visible indicator in the footer (R-129); nothing is automatic.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Collect everything relevant, in one file"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "4. Deriving the tier instead of choosing it"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "Thermal is the honest budget"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "1.2 Passive telemetry — what interaction actually looks like"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "8. What this is not"

- `decisions.md` § "R-129 ✱ — Where the surfaces with no artboard live *(closes RQ-105)*"
## Deliverables
- `crates/engine/src/telemetry/{record,binding,thermal}.rs` — record fields, per-run binding resource, throttling inference.
- `crates/gui/src/telemetry_toggle.rs` — the passive-mode switch in the Profiler (off by default) and its footer indicator (R-129).
- Tests: `campaign_fields`, `binding_resource`; bench `throttle-inference`; screenshot case `01_main/passive_telemetry` (presence only, R-129).

## Acceptance tests
- `cargo test -p engine campaign_fields` — a campaign file contains every listed field (REQ-TOOL-092).
- `cargo test -p engine binding_resource` — the report names one binding resource per run (REQ-TOOL-089).
- `cargo xtask bench throttle-inference` — a long constant-scene run flags a throttling inference when fps declines monotonically (REQ-TOOL-090).
- `cargo xtask screenshot 01_main` (presence, R-129; indicator on; off by default) — the Profiler switch exists; the footer indicator is visible while passive logging is on; logging is off by default (REQ-TOOL-088).

## Notes
- RQ-105 ruled: R-129 — passive logging is a Profiler switch with its indicator in the footer; with no artboard both are checked by presence, not layout.
