# TASK-M7-33 — Pipeline precompile at startup and the render-target counts

- **Milestone:** M7
- **Closes:** REQ-PERF-068, REQ-PERF-069
- **Depends on:** TASK-M7-14, TASK-M7-17, TASK-M7-21, TASK-M5-10, TASK-M5-27
- **Needs (earlier milestones):** REQ-RENDER-040, REQ-RENDER-006, REQ-PERF-031
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Render presets and compositor shaders are precompiled at startup; other slot combinations, debug views and customs compile on selection, asynchronously with last-valid fallback, within a startup budget of about 10 compute pipeline objects, 6 fragment and 3 compositor pipelines. With the render graph built, the render-target counts k_d (display-res) and k_r (render-res) are measured and recorded.

## References
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"
- `docs/design/principia_memory_tiers.md` § "8. Caveats"

## Deliverables
- `crates/engine/src/pipelines/precompile.rs` — the startup set (the render presets, the compositor set).
- `xtask` benches `pipeline-startup` and `render-targets`.
- `docs/design/principia_memory_tiers.md` §8 — the measured k_d, k_r added beside the estimates (not replacing them), with the "Removed lines" note.

## Acceptance tests
- `cargo xtask bench pipeline-startup` — counts pipeline objects created at startup (≈10 compute, 6 fragment, 3 compositor) and measures startup time; no other fragment pipeline compiles before selection (REQ-PERF-068).
- `cargo xtask bench render-targets` — counts display-res and render-res targets allocated by the built render graph and records k_d, k_r (REQ-PERF-069).

## Notes
- The corpus gives no startup-time threshold; the bench records it.
- Which render presets form the precompiled "handful" follows the preset library's production section.
