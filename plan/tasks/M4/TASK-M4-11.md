# TASK-M4-11 — Checkerboard march: a per-pixel phase mask over whole sample bundles

- **Milestone:** M4
- **Closes:** REQ-RENDER-032, REQ-RENDER-033, REQ-RENDER-035, REQ-PERF-005
- **Depends on:** TASK-M4-10, TASK-M4-06
- **Needs (earlier milestones):** REQ-RENDER-004, REQ-PAY-025
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
Checkerboard motion acceleration's compute half: while the playhead advances, only the samples of half the render pixels march each frame — a per-render-pixel mask whose parity flips each frame — and the other half hold their value from t − dt. A pixel's whole bundle (nominal, E copies, every Benettin shadow) shares one phase; the mask is per pixel, never per trajectory. It runs at every E including 0, allocates nothing extra, and keeps every stale `SimState` resident.

## References
- `docs/contracts/principia_checkerboard_contract.md` § "1. What this is (and what it is NOT)"
- `docs/contracts/principia_checkerboard_contract.md` § "2. The mechanism: reconstruct-from-previous, resolved through SSAA"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact"
- `docs/contracts/principia_checkerboard_contract.md` § "3. The three-state control"

## Deliverables
- `crates/engine/src/checkerboard.rs`: the parity mask (render-pixel granularity), applied to the live-set march only (never to catch-up), per-pixel for the whole bundle.
- Kernel/dispatch plumbing: the parity bit as a per-dispatch uniform; no new buffer.

## Acceptance tests
- `cargo test -p engine checkerboard_parity` — over two frames every pixel advances exactly once; the per-frame trajectory count halves (REQ-RENDER-032).
- `cargo test -p engine checkerboard_bundle` — within any pixel all bundle members carry the same t after every frame (REQ-RENDER-033).
- `cargo test -p engine checkerboard_e0` — E = 0 (the Potato setting) with checkerboard on: the mask is applied (REQ-RENDER-035).
- `cargo test -p engine checkerboard_memory` — GPU allocation is identical with checkerboard on and off; the estimator's figure is unchanged (REQ-PERF-005).

## Notes
- Gap: the memory estimator REQ-PERF-005 names is built in M5 (REQ-PERF-018).
- Waits on RQ-98 (`REVIEW_QUEUE.md`): M3 and M4 requirements that name later surfaces.
