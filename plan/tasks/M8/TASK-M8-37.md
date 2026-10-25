# TASK-M8-37 — Browser build (v1 web): the wasm engine in a Web Worker on an OffscreenCanvas, the main-thread shell, coi-serviceworker

- **Milestone:** M8
- **Closes:** REQ-SYS-039, REQ-SYS-047, REQ-SYS-049, REQ-SYS-056, REQ-SYS-045, REQ-SYS-042, REQ-COL-047, REQ-TOOL-143
- **Depends on:** TASK-M8-02, TASK-M8-04, TASK-M7-16
- **Needs (earlier milestones):** REQ-SYS-020, REQ-SYS-031, REQ-SYS-034, REQ-SCHED-072, REQ-RENDER-058
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~530 lines

## Goal
v1 web's build exists (R-571: the whole native app, compiled to run in the browser, its GUI the native app's): the whole frame loop (WebGPU device, scheduler, cache, quadtree, compute and render, playhead and barrier) runs in the wasm engine inside a Web Worker on an OffscreenCanvas transferred once at startup; the main thread holds the GUI's own wasm context (egui), a loader and the input pump, owns no simulation state and hosts no DOM GUI (R-620, RQ-551 (b)); the figure is drawn on the worker's `OffscreenCanvas` and the GUI on a main-thread canvas above it, two surfaces with no compositing step (applied per R-369). *Was:* "the main thread is a separate binary that owns no simulation state and hosts no DOM GUI, the GUI being the native app's egui (where egui runs, and so what else the main thread holds, is RQ-551)." Resize is observed on the main thread and sent as one message that reconfigures the swapchain in the worker. Cross-origin isolation on GitHub Pages comes from a self-hosted `coi-serviceworker` beside `index.html`. The equirect bake is debounced (120 ms) via `copyExternalImageToTexture` and never lands inside the frame callback.

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
- `decisions.md` § "R-146 — The crate layout is confirmed *(closes RQ-76)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-303 — In the browser build, each stage's compiled fast-math mode is "unknown" *(closes RQ-177; amends R-297)*"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `decisions.md` § "R-620 — In v1 web egui runs in its own wasm context on the main thread, and the engine stays in its worker *(closes RQ-551)*"

## Deliverables
- `web/` — `package.json`, `index.html`, `coi-serviceworker.js` (vendored), `src/main.ts` (shell), `src/engine.worker.ts` (loads the wasm engine), `src/resize.ts`.
- `crates/engine/src/wasm/entry.rs` — the worker entry, surface from the transferred OffscreenCanvas.
- `crates/engine/src/wasm/bake.rs` — the debounced bake upload.
- The session header's browser rule (R-303): the header writer takes the running backend's fast-math capability, and the browser build (`crates/engine/src/wasm/entry.rs`) passes one that exposes no control, so each stage's compiled mode is written as "unknown".
- Tests: `resize_one_message`, `bake_outside_raf`, `session_header_browser_fast_math`.

## Acceptance tests
- Review checklist (code reviewer) — the build produces the engine's worker module and the GUI's main-thread wasm context; the main-thread bundle contains no engine state (REQ-SYS-039). *Was:* "build produces two binaries; main-thread bundle contains no engine state" (R-620).
- Review checklist (code reviewer) — the engine crate builds to wasm and contains all CPU-side state; the browser build's GUI is the gui crate's, and web/ holds no GUI and no scheduler/cache/physics state (code review of the web package); the GUI's wasm context runs on the main thread and the engine's in its worker (R-620) (REQ-SYS-047).
- Review checklist (code reviewer) — the main-thread bundle contains no engine state; two worker wasm instances exist (engine, inspector); the GUI's wasm context runs on the main thread (R-620) (REQ-SYS-049).
- Review checklist (code reviewer) — review the v1 web startup path: one transferControlToOffscreen / postMessage transfer of the figure's canvas, no later canvas messages carrying state, the GUI on its own main-thread canvas (REQ-SYS-056). *Was:* "review the TS startup path: one transferControlToOffscreen / postMessage transfer, no later canvas messages carrying state" (R-620).
- `npm --prefix web test -- resize_one_message` — resize event yields one message and one reconfigure (REQ-SYS-045).
- Review checklist (code reviewer) — deployment contains the script; no CDN (REQ-SYS-042).
- `npm --prefix web test -- bake_outside_raf` — bake upload scheduled outside the rAF callback (REQ-COL-047).
- `cargo test -p engine session_header_browser_fast_math` — a header written for a backend with no fast-math control, as the browser build's WebGPU is, records the compute setting as asked for, off and on, and the compute, vertex and fragment modes as "unknown"; a native header never records "unknown" (REQ-TOOL-143).
- Review checklist (code reviewer) — the browser entry (`crates/engine/src/wasm/entry.rs`) passes the no-control fast-math capability that the header writer and the Run window read (REQ-TOOL-143).

## Notes
- R-303 (closes RQ-177): in the browser build each shader stage's compiled fast-math mode is recorded as "unknown", since WebGPU offers no fast-math control (REQ-TOOL-143).
- R-146: `web/` unit tests run under Vitest (`npm --prefix web test` runs `vitest run`); the browser suites run under Playwright.
- The second (inspector) wasm instance that REQ-SYS-049 names is started here and wired in TASK-M8-38.
- RQ-99 ruled: R-113 — the M5 native loop runs on a dedicated render thread (REQ-SYS-034); the wasm-engine worker clause of REQ-SYS-034 and REQ-RENDER-045 is REQ-SYS-039/049, closed here.
- R-571 (21 Oct 2026): the browser build is v1 web, the whole native app compiled to run in the browser, with the same interface, features and stains; its GUI is the native app's egui (REQ-SYS-047 restated), and a TS DOM GUI is v2 web's. R-620 (25 Oct 2026) rules RQ-551 (b): egui in its own wasm context on the main thread, the engine in its worker; nothing waits. *Was:* "Where egui runs in the browser and what the main thread holds is RQ-551: REQ-SYS-039, REQ-SYS-045, REQ-SYS-047, REQ-SYS-049 and REQ-SYS-056 carry `rq: RQ-551`, so this task waits on it." *Was (title):* "the TS shell"; *was (goal):* "The browser product exists" and "the main thread is a separate TS binary — an input pump and DOM-GUI host that owns no simulation state" (gui review 5608379011 of PR #282, applied per R-571: none of RQ-551's options has a DOM GUI); *was (acceptance, REQ-SYS-047):* "the TS shell holds no scheduler/cache/physics state (code review of the TS package)".
