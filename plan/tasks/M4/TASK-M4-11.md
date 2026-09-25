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
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/engine/src/checkerboard.rs`: the parity mask (render-pixel granularity), applied to the live-set march only (never to catch-up), per-pixel for the whole bundle.
- Kernel/dispatch plumbing: the parity bit as a per-dispatch uniform; no new buffer.

## Acceptance tests
- `cargo test -p engine checkerboard_parity` — over two frames every pixel advances exactly once; the per-frame trajectory count halves (REQ-RENDER-032).
- `cargo test -p engine checkerboard_bundle` — within any pixel all bundle members carry the same t after every frame (REQ-RENDER-033).
- `cargo test -p engine checkerboard_e0` — E = 0 with checkerboard on: the mask is applied (REQ-RENDER-035; that Potato is the E = 0 tier is REQ-PERF-014's, M5).
- `cargo test -p engine checkerboard_memory` — GPU allocation is identical with checkerboard on and off; no extra or half-size buffer; every stale SimState stays resident (REQ-PERF-005; the estimator's no-saving credit is REQ-PERF-018's, TASK-M5-10).

## Notes
- RQ-98 ruled: R-113 — REQ-PERF-005 keeps the allocation half here; "the estimator must credit it with no memory saving" joins REQ-PERF-018 (M5, TASK-M5-10). REQ-RENDER-035's verify is "E = 0 with checkerboard on"; the tier name is checked by REQ-PERF-014 (M5). This settles the gap.
