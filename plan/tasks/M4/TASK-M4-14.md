# TASK-M4-14 — Seam cross-check views on GPU payloads, and the live substep heatmap

- **Milestone:** M4
- **Closes:** REQ-VAL-066, REQ-VAL-067, REQ-VAL-068, REQ-VAL-069, REQ-VAL-077, REQ-VAL-112, REQ-TOOL-045
- **Depends on:** TASK-M4-10, TASK-M3-22
- **Needs (earlier milestones):** REQ-INT-001, REQ-INT-041, REQ-PAY-030, REQ-PAY-073, REQ-PAY-033, REQ-TOOL-035, REQ-VAL-015, REQ-VAL-117, REQ-INT-027, REQ-TOOL-024, REQ-PAY-050, REQ-VAL-033, REQ-TOOL-018, REQ-SYS-017
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~480 lines

## Goal
debug_tooling_plan §G's integration tests with a display, now on payloads the GPU wrote: winding consistency (derived `orbit_count`/`retrograde` vs the running θ̃, terminal latch respected), the Welford diffusion slope vs a reference computed the same way (and fit-vs-fit in the CPU parity suite via `computeIC`), word bookkeeping (`fgw_reduced_length` vs the derived reduced-crossing count vs `fgw_truncated`), and drift shape (final vs max). The agreement tolerances of the invariant-chart gradient and the Welford slope are calibrated (R-71). A live view renders an animated heatmap of the current macro-step's substep count read off the marching state.

## References
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/design/principia_debug_tooling_plan.md` § "G. Cross-check views (certify a *seam*, not a field — integration tests with a display)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"

## Deliverables
- `crates/render`: the four cross-check views as fragment presets over the generated accessors; the live substep heatmap view.
- `crates/validation/tests/cross_checks.rs`: each cross-check on synthetic payloads and on GPU-written golden-IC payloads; the Welford fit-vs-fit via `computeIC`.
- `xtask` gate `cross-check-tolerances`: the measured f32 disagreement of the invariant-chart gradient and the Welford slope views on a healthy survey and on a deliberately broken case each tolerance must flag.
- Golden fixtures `fixtures/golden/drift-shape/`, `fixtures/golden/substep-heatmap/`.

## Acceptance tests
- `cargo test -p render winding_consistency` — synthetic θ̃ values straddling multiples of 2π give the expected `orbit_count`/`retrograde` (REQ-VAL-066).
- `cargo test -p render welford_crosscheck` — the fragment slope equals the CPU-recomputed slope from the same accumulators bit for bit (or to f32 rounding), and `cargo test -p validation welford_fit_vs_fit` runs the full fit-vs-fit through `computeIC` (REQ-VAL-067).
- `cargo test -p render word_bookkeeping` — synthetic words with known crossings: lengths agree when not truncated; truncated words flagged (REQ-VAL-068).
- `cargo xtask golden drift-shape` — a synthetic payload with a recovered transient spike shows max > final; secular loss shows max ≈ final (REQ-VAL-069).
- `cargo test -p validation cross_checks_gpu` — each cross-check (winding with the terminal latch, Welford, drift shape) passes on synthetic and GPU-written golden-IC payloads (REQ-VAL-077).
- `cargo xtask gate cross-check-tolerances` — the proposal shows the measured f32 disagreement of each view on a healthy survey and a deliberately broken case each tolerance must flag; checked by the physics reviewer and confirmed by the human at the M4 gate (REQ-VAL-112, calibrated).
- `cargo xtask golden substep-heatmap` — a close-encounter IC (REQ-VAL-033's fixture) shows a hot spot that moves over playback (REQ-TOOL-045).

## Notes
- REQ-VAL-112 is a calibration (R-71): the PR carries the proposed value, its evidence and the reviewer's check, marked pending; the human confirms it at the M4 gate and it is then recorded in decisions.md.
- Each cross-check is shown able to fail (a deliberately broken accumulator turns it red) before its green result is read (pitfalls §3, §9).
