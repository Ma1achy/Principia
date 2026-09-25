# TASK-M1-05 — The fragment pipeline at runtime: fragment key, async compile cache, last-valid fallback, hot reload and the frame record

- **Milestone:** M1
- **Closes:** REQ-RENDER-003, REQ-RENDER-005, REQ-RENDER-006, REQ-RENDER-011, REQ-TOOL-131
- **Depends on:** TASK-M1-04, TASK-M0-17
- **Needs (earlier milestones):** REQ-TOOL-008
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~420 lines

## Goal
The assembled fragment source becomes a live wgpu pipeline at runtime: hashed into the fragment key, compiled asynchronously, cached, with last-valid-on-failure per node (a node that fails to compile keeps its last valid source while the others take their current sources, and the error is surfaced). A colour snippet supplied at runtime compiles and swaps in without rebuilding the binary and without touching sim buffers. Debug field views are baked one tiny source per view on demand (never a mega-switch); node params and view-only display state are uniforms; the fixed compositor shaders (backdrop, separable blur ×2, composite) are created at startup. This is the first frame loop, so the profiler's measurement struct lands with it (R-56, R-113): each rendered frame fills a schema-v1 frame record.

## References
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_core_design.md` § "4. Integrate and colour are separate passes — and now separate *mechanisms*"
- `docs/contracts/principia_lowering_contract.md` § "Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)"
- `docs/contracts/principia_lowering_contract.md` § "Part 4 — The precompile rule"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/contracts/principia_lowering_contract.md` § "Fragment side"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/design/principia_colour_composition.md` § "5. Codegen & eject"
- `docs/contracts/principia_lowering_contract.md` § "Part 3 — The baking rules (compile-time vs uniform)"
- `decisions.md` § "R-56 — Profiler schema v1 is a superset of telemetry §2, in JSON *(GU-5, amended)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5.5 Profiling is FIRST-CLASS, not a debug mode"

## Deliverables
- `crates/render/src/pipeline_cache.rs`: fragment-key hashing, async compile, cache, per-node last-valid state, error surface.
- `crates/render/src/hot_reload.rs`: runtime snippet ingestion (a changed `.wgsl` occupant file or an in-memory source).
- `crates/render/src/compositor.rs` + `shaders/wgsl/compositor/*.wgsl`: the three fixed compositor pipelines, created at startup.
- `crates/render/src/frame_record.rs`: the render loop fills the M0 schema-v1 `FrameRecord` (TASK-M0-17) per frame — the stages it has at M1 (colour, present) timed, the rest zero — always compiled, with reporting toggleable.
- `crates/render/src/debug_bake.rs`: the entry point that bakes one debug-view source per field id on demand (the catalogue generator plugs into it in TASK-M1-08).
- Tests: `crates/render/tests/pipeline_runtime.rs`.

## Acceptance tests
- `cargo test -p render runtime_snippet` — a colour snippet supplied at runtime compiles, swaps in without rebuilding the binary, and leaves sim buffers byte-identical (REQ-RENDER-003).
- `cargo test -p render fragment_cache` — the same stain produces the same hash and hits the cache; a failed compile keeps the previous pipeline (REQ-RENDER-005).
- Review checklist (code): no debug shader switches over field ids at runtime; compositor pipelines are created at startup (REQ-RENDER-006).
- `cargo test -p render node_failure_isolation` — a syntax error injected into one node's source: the pipeline keeps rendering with that node's previous source and the other nodes' current sources; an error is surfaced (REQ-RENDER-011).
- `cargo test -p render frame_record` — rendering N frames of the synthetic payload with profiling on writes N frame records that validate against schema v1 (REQ-TOOL-131).

## Notes
- REQ-RENDER-003 says "without rebuilding the wasm binary"; the wasm build is M8, so at M1 the test is native (no rebuild of the running binary) and the wasm form re-runs when M8 lands.
- R-113 (RQ-93): R-56's "the measurement struct lands with the first frame loop" is placed here (REQ-TOOL-131), split from REQ-TOOL-008.
- Precompile rule (lowering Part 4): render presets and compositor precompile; debug views and customs compile on selection. No compile on the gesture path.
