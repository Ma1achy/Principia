# TASK-M8-31 — Keyframes, the export planner and the spec object: serialise, share, re-render

- **Milestone:** M8
- **Closes:** REQ-TOOL-074, REQ-TOOL-075, REQ-TOOL-076, REQ-TOOL-095, REQ-TOOL-096, REQ-TOOL-083, REQ-TOOL-084, REQ-VAL-100
- **Depends on:** TASK-M8-30, TASK-M7-31
- **Needs (earlier milestones):** REQ-SYS-011, REQ-SCHED-048, REQ-TOOL-065, REQ-VAL-057, REQ-VAL-064, REQ-GUI-002
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-10
- **Size:** ~500 lines

## Goal
A keyframe animation serialises as {keyframes: [(t_wall, ViewState, RenderState, playhead policy)], easing, fps, duration}, provenance-complete, URL-encodable and re-renderable; everything a result is conditional on serialises with it. Interpolation eases z₀ componentwise, log-lerps zoom, moves the basis along the Grassmannian geodesic re-orthonormalised per frame, eases slices, lerps render uniforms per their scale metadata, and treats chart switches as cuts or dissolves. The planner classifies each segment as fixed-view time playback, view animation or render-only animation, prices each and reports the estimate before the job runs; the ladder re-runs per frame wherever the sim key moves. Exported artefacts embed or link the spec, and receiving it re-renders bit-comparable playback. The spotlight reel is built by the blocking exporter from a curated spec list. The same spec re-renders bit-identically on one backend and within parity tolerance across backends, labels flipping only on chaotic trajectories (R-84).

## References
- `docs/contracts/principia_export_animation_contract.md` § "Part 2 — The two playback types and their costs"
- `docs/contracts/principia_export_animation_contract.md` § "Part 3 — The keyframe system (retained)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/contracts/principia_export_animation_contract.md` § "Part 7 — The spotlight reel (attract mode)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 5 — Determinism & reproducibility"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"

## Deliverables
- `crates/engine/src/contract/animation.rs` — the spec object and its URL encoding.
- `crates/engine/src/export/{keyframes,planner}.rs` — interpolation and segment classification with estimates.
- `xtask/src/reel.rs` — `cargo xtask reel <spec-list>` builds the spotlight reel through the exporter.
- Golden suite `spec-rerender` (same backend bit-identical; second backend within parity tolerance, flips reported).
- Tests: `planner_segments`, `keyframe_interp` (proptest), `spec_url_roundtrip`, `spec_rerender_payload`, `keyframe_sim_key_reintegrates`, `export_embeds_spec`.

## Acceptance tests
- `cargo test -p engine planner_segments` — a three-segment spec is classified (a) / (b) / (c) and an estimate is shown before start; the (c) segment triggers no integration (REQ-TOOL-074).
- `cargo test -p engine keyframe_interp` — random keyframe pairs: every interpolated basis is orthonormal; zoom midpoint equals the geometric mean; a chart change yields no interpolated chart (REQ-TOOL-075).
- `cargo test -p engine spec_url_roundtrip` — serialise, URL-encode, decode, re-render: the frames match the original (REQ-TOOL-076).
- `cargo test -p engine spec_rerender_payload` — serialise → URL → deserialise → re-render reproduces the payload bytes (REQ-TOOL-095).
- `cargo test -p engine keyframe_sim_key_reintegrates` — a keyframed sim-key change triggers re-integration for the affected frames (REQ-TOOL-096).
- `cargo test -p engine export_embeds_spec` — an exported video's embedded spec re-renders identical frames (REQ-TOOL-083).
- Review checklist (code reviewer) of `xtask/src/reel.rs` and the reel player's click-to-load — the reel build script invokes the exporter over the spec list; clicking a segment loads its spec (REQ-TOOL-084).
- `cargo xtask golden spec-rerender` — render a spec twice on one backend: bit-identical frames; on a second backend: label / class images identical outside chaotic regions (flips there reported), values within parity tolerance (REQ-VAL-100).

## Notes
- The URL encoding of the spec (and the `principia://view?…` link TASK-M8-32 uses) is not specified (raised as a gap for a REVIEW_QUEUE entry).
