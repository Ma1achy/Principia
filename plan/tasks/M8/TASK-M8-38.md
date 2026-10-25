# TASK-M8-38 — v1 web's membranes: set_field and SAB input in, the ~10 Hz snapshot out, the inspector worker

- **Milestone:** M8
- **Closes:** REQ-SYS-040, REQ-SYS-041, REQ-SYS-043, REQ-SYS-050, REQ-SYS-051, REQ-SYS-055, REQ-SYS-059, REQ-SYS-044, REQ-SYS-091
- **Depends on:** TASK-M8-37, TASK-M8-09
- **Needs (earlier milestones):** REQ-SYS-036, REQ-GUI-010, REQ-SYS-033
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
The engine↔GUI membrane has exactly three crossings for engine state, in v1 web messages between the GUI's wasm context on the main thread and the engine's worker (R-620): `set_field(path, value)` of plain serialised data as the GUI's only write, a GUI-sized snapshot throttled to ~10 Hz (R-94), and the OffscreenCanvas transferred once. Input deltas reach the engine through a SharedArrayBuffer written with Atomics and read at the top of every frame (pointer events coalesced) when cross-origin isolated, otherwise one coalesced postMessage per main-thread rAF tick — chosen once at boot behind one engine interface. No wasm-bindgen object handle reaches JS. Click-to-inspect sends cursor coordinates; the f64 re-integration runs fire-and-forget in the inspector worker (a second wasm context) and posts back. The GUI redraws at frame rate from the last snapshot. v1 web's other crossings are a second, enumerated platform list (systems_architecture §3's table), plain data and never engine-sized, and both lists are checked in CI (R-619); this task defines the browser transport of `prin profile query --live`, the list's row 14 (REQ-SYS-091, R-72). *Was:* "The wasm ↔ JS membrane has exactly three crossings:" (R-619, R-620).

## References
- `docs/contracts/principia_caching_contract.md` § "Part 6a — The threading model: the render loop lives in a worker (and that worker is the wasm engine)"
- `decisions.md` § "R-94 — The GUI snapshot is ~10 Hz *(closes RQ-45)*"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `decisions.md` § "R-571 — The browser: the native app first, then v1 web, the whole native app compiled to run in the browser on the GitHub Pages site, then v2 web, a separate product designed later *(replaces the browser item of the gap sweep's `INSTRUCTION_now.md` backlog; queues RQ-550 and RQ-551)*"
- `decisions.md` § "R-619 — v1 web has two lists of crossings: the three for engine state, unchanged, and a separate, enumerated platform list, both enforced in CI *(closes RQ-550; amends REQ-SYS-059's "exactly three" for v1 web)*"
- `decisions.md` § "R-620 — In v1 web egui runs in its own wasm context on the main thread, and the engine stays in its worker *(closes RQ-551)*"

## Deliverables
- `crates/engine/src/wasm/membrane.rs` — the three exports only, serialised in and out.
- `crates/engine/src/wasm/input.rs` — the SAB and postMessage backends behind one trait, boot selection.
- `web/src/{input,snapshot,inspector}.ts`; `crates/engine/src/wasm/inspector_entry.rs` — the second wasm context running TASK-M8-09's worker.
- Tests: `sab_coalesce`, `boot_selects_backend`, `snapshot_rate`, `transports_equal`.

## Acceptance tests
- `npm --prefix web test -- sab_coalesce` — burst of pointermoves produces one SAB state, not queued messages (REQ-SYS-040).
- `npm --prefix web test -- boot_selects_backend` — boot without isolation selects postMessage; engine code path identical (REQ-SYS-041).
- `npm --prefix web test -- snapshot_rate` — message rate ≤ ~10 Hz; payload contains no quad tree or payload data; the GUI repaints every frame between snapshots from the last one received (REQ-SYS-043).
- `npm --prefix web test -- transports_equal` — both transports deliver the same edit sequence; snapshot rate is capped (REQ-SYS-050).
- Review checklist (code reviewer) — no wasm-bindgen object handle is exported to JS; snapshot payload size is GUI-sized (REQ-SYS-051).
- Review checklist (code reviewer) of the wasm-bindgen layer — review the wasm-bindgen layer: exported functions take/return serialised data only; no exported struct with callable methods reaches the scheduler or other internals (REQ-SYS-055).
- `cargo xtask check` (membrane crossings) — REQ-SYS-059's verify: each wasm module's imports and exports, and the messages between the two contexts, checked against the three crossings and the platform list; controls: an extra export or message carrying engine state, a platform crossing missing from the list, and an engine-sized message, each fail the check (REQ-SYS-059; R-619). *Was:* "Review checklist (code reviewer) — the wasm exports are set_field, snapshot and the one-time canvas init (REQ-SYS-059)."
- Review checklist (the reviewers) — systems_architecture §3's row 14 states how `prin profile query --live` reaches the page's profiler in v1 web, as plain data (REQ-SYS-091).
- Review checklist (perf reviewer) — inspector runs in a separate worker; main thread never blocks (REQ-SYS-044).

## Notes
- R-571 (21 Oct 2026): the browser build is v1 web, the whole native app in the browser, which needs more crossings than the three (downloads, file drops, the clipboard, audio and more; measured in `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)", "v1 web, and what it crosses"). R-619 (25 Oct 2026, RQ-550 (a)): the three crossings stay for engine state, and the platform list is the second list, both enforced in CI; R-620 (RQ-551 (b)): egui in its own wasm context on the main thread, the engine in its worker. Nothing waits. *Was:* "The rule is not redesigned: REQ-SYS-051 and REQ-SYS-059 carry `rq: RQ-550`, and REQ-SYS-040, REQ-SYS-041, REQ-SYS-043, REQ-SYS-044 and REQ-SYS-050 `rq: RQ-551` (where the GUI runs), so this task waits on both."
- Definition written here (R-72; the reviewers approve before merge): REQ-SYS-091. *Was (title):* "Browser membranes".
