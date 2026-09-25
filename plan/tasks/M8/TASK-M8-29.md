# TASK-M8-29 — Failing gracefully: no-GPU fallback, device loss, and every degradation shown and logged

- **Milestone:** M8
- **Closes:** REQ-SYS-057, REQ-SYS-058, REQ-TOOL-091
- **Depends on:** TASK-M8-26, TASK-M8-27
- **Needs (earlier milestones):** REQ-TOOL-001, REQ-SCHED-042, REQ-SCHED-015, REQ-PERF-023, REQ-SYS-017
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
With no suitable GPU the application falls back to the CPU path, says so loudly and permanently in the UI, and records the reason (no adapter / adapter lacks required features / backend init failed). On device loss it reinitialises, rebuilds from the cache metadata with byte-identical payloads, and says what happened. Every failure or degradation is logged, shown and carried in telemetry, so a degraded session never looks like a healthy one.

## References
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.1 No suitable GPU, or no GPU at all"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.3 Device loss, timeouts, driver resets"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6.5 The rule underneath all of these"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "6. Failing gracefully — and what "gracefully" means here"

## Deliverables
- `crates/engine/src/device/{select,loss}.rs` — the three failure reasons; rebuild from cache metadata.
- `crates/gui/src/explore/degraded_banner.rs` — the permanent CPU-path indicator (outside the figure).
- A degradation-path index (`crates/engine/src/telemetry/degradations.rs`) that each fallback registers in, checked by a test that every registered path emits an event and a UI indicator.
- Tests: `no_gpu_fallback`, `device_loss_rebuild` (proptest), `degradation_paths_reported`.

## Acceptance tests
- `cargo test -p engine no_gpu_fallback` — simulate each failure: CPU path runs, UI banner shows, telemetry reason distinguishes the three (REQ-SYS-057).
- `cargo test -p engine device_loss_rebuild` — inject device loss: payloads rebuilt are byte-identical and a notice is shown (REQ-SYS-058).
- Review checklist (code reviewer), with `cargo test -p engine degradation_paths_reported` — each fallback/failure path emits a telemetry event and a UI indicator (REQ-TOOL-091).

## Notes
- none
