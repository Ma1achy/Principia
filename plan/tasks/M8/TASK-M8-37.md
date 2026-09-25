# TASK-M8-37 — Browser build: the wasm engine in a Web Worker on an OffscreenCanvas, the TS shell, coi-serviceworker

- **Milestone:** M8
- **Closes:** REQ-SYS-039, REQ-SYS-047, REQ-SYS-049, REQ-SYS-056, REQ-SYS-045, REQ-SYS-042, REQ-COL-047
- **Depends on:** TASK-M8-02, TASK-M8-04, TASK-M7-16
- **Needs (earlier milestones):** REQ-SYS-020, REQ-SYS-031, REQ-SYS-034, REQ-SCHED-072, REQ-RENDER-058
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The browser product exists: the whole frame loop (WebGPU device, scheduler, cache, quadtree, compute and render, playhead and barrier) runs in the wasm engine inside a Web Worker on an OffscreenCanvas transferred once at startup; the main thread is a separate TS binary — an input pump and DOM-GUI host that owns no simulation state. Resize is observed on the main thread and sent as one message that reconfigures the swapchain in the worker. Cross-origin isolation on GitHub Pages comes from a self-hosted `coi-serviceworker` beside `index.html`. The equirect bake is debounced (120 ms) via `copyExternalImageToTexture` and never lands inside the frame callback.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"

## Deliverables
- `web/` — `package.json`, `index.html`, `coi-serviceworker.js` (vendored), `src/main.ts` (shell), `src/engine.worker.ts` (loads the wasm engine), `src/resize.ts`.
- `crates/engine/src/wasm/entry.rs` — the worker entry, surface from the transferred OffscreenCanvas.
- `crates/engine/src/wasm/bake.rs` — the debounced bake upload.
- Tests: `resize_one_message`, `bake_outside_raf`.

## Acceptance tests
- Review checklist (code reviewer) — build produces two binaries; main-thread bundle contains no engine state (REQ-SYS-039).
- Review checklist (code reviewer) — the engine crate builds to wasm and contains all CPU-side state; the TS shell holds no scheduler/cache/physics state (code review of the TS package) (REQ-SYS-047).
- Review checklist (code reviewer) — the main-thread bundle contains no engine state; two worker wasm instances exist (engine, inspector) (REQ-SYS-049).
- Review checklist (code reviewer) — review the TS startup path: one transferControlToOffscreen / postMessage transfer, no later canvas messages carrying state (REQ-SYS-056).
- `npm --prefix web test -- resize_one_message` — resize event yields one message and one reconfigure (REQ-SYS-045).
- Review checklist (code reviewer) — deployment contains the script; no CDN (REQ-SYS-042).
- `npm --prefix web test -- bake_outside_raf` — bake upload scheduled outside the rAF callback (REQ-COL-047).

## Notes
- `web/` has no test runner in the plan conventions; this task uses `npm --prefix web test` (raised as a gap for a REVIEW_QUEUE entry).
- The second (inspector) wasm instance that REQ-SYS-049 names is started here and wired in TASK-M8-38.
