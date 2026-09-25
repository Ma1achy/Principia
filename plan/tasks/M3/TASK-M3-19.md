# TASK-M3-19 — The Precision ring: computeIC, computeQuad, stepOnce, decodeOnly and the RK45 inspector reference

- **Milestone:** M3
- **Closes:** REQ-SYS-017, REQ-INT-055, REQ-GUI-008
- **Depends on:** TASK-M3-06, TASK-M3-17, TASK-M2-25
- **Needs (earlier milestones):** REQ-SYS-015, REQ-DEC-005, REQ-CHART-028, REQ-PAY-034, REQ-TOOL-029
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
The CPU-f64 instantiation of the shared kernel ships as the Precision ring in `crates/engine`, exposing parity §1's `computeIC(chart, uv, simKey)`, `computeQuad(chart, quadID, simKey)`, `stepOnce(state, dt, params)` and `decodeOnly(chart, uv, simKey)`. The IC Inspector integrates a clicked IC through the shared kernel at f64 on the CPU — one Inspector (R-65), not a separate viewer — and its independent adaptive reference integration is RK45 in f64 (R-33), held outside the occupant system.

## References
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `decisions.md` § "R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*"
- `docs/design/principia_trajectory_viewing.md` § "1. The single mechanism"
- `docs/read_first/principia_00_philosophy.md` § "The IC inspector — the prebake standard at a single point"
- `decisions.md` § "R-65 — One Inspector window; the standalone IC Inspector is absorbed *(closes RQ-21)*"

## Deliverables
- `crates/engine/src/precision/mod.rs` — the four entry points over the kernel instantiated at f64.
- `crates/engine/src/precision/rk45.rs` — the RK45 reference integrator (f64, adaptive), used by the inspector path only.
- `crates/engine/src/inspector.rs` — the engine-side Inspector model: one type serving the IC Inspector and the trajectory viewer, integrating through `computeIC`.

## Acceptance tests
- `cargo test -p engine precision_ring_entry_points` — each of the four entry points called from a native test: computeIC returns a full-f64 SimState; stepOnce performs one STEP for any bound occupant; decodeOnly returns (m, r, p, ICDescriptor); computeQuad returns the quad's samples (REQ-SYS-017).
- Review (physics): computeIC's inspector path uses the RK45 integrator in f64 (REQ-INT-055).
- `cargo test -p engine inspector_shared_kernel` — the Inspector integrates a clicked IC via the shared kernel instantiated at f64 on CPU; the IC Inspector and trajectory viewer are one Inspector type (R-65) (REQ-GUI-008).

## Notes
- Gap: REQ-GUI-008's 'hosted in the one Inspector window' — the window is the M8 dev GUI; M3 can only test the engine-side Inspector. The M8 GUI tasks must keep it one window.
- Waits on RQ-98 (`REVIEW_QUEUE.md`): M3 and M4 requirements that name later surfaces.
