# TASK-M3-03 — Substep control: the frozen N_sub bucket table and total_substeps

- **Milestone:** M3
- **Closes:** REQ-INT-027, REQ-INT-028, REQ-TOOL-033, REQ-TOOL-034
- **Depends on:** TASK-M3-01
- **Needs (earlier milestones):** REQ-GEN-007, REQ-PAY-013, REQ-PAY-027, REQ-TOOL-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~400 lines

## Goal
`N_sub` is chosen by a generated comparison tree of the position-quantised `d²` against 64 build-time-frozen f32 thresholds `THR[n] = round_f32((r_sub/n^{2/3})²)`, with no runtime sqrt, div, pow, loop or dynamic index, and it equals the specification `clamp(⌈(r_sub/r_min)^{γ_sub}⌉, 1, N_max)`. The substepper exposes the live `N_sub` per macro-step, keeps the exact `total_substeps` u32, and sets the sticky `saturated` descriptor bit iff `N_sub == N_max` occurs.

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 3 — Parameter ownership (who owns what)"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/design/principia_dd_integrator.md` § "3.3 Substep law — and the determinism pin, made concrete"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `xtask`/`build.rs` table generator: computes `THR[1..=64]` in f64 libm and emits only the f32 bit patterns as a generated Rust source (`crates/kernel/src/generated/nsub_thresholds.rs`) plus the comparison tree (no loop, no indexing).
- `crates/kernel/src/driver/substep.rs` — `d2_quantised` (f32 positions, lone-sub, lone-mul, explicit `fma(dy, dy, dx*dx)`), the clamp-in-f32-before-cast rule, `bucket(d2) -> u32`, and `SubstepState { live_n_sub, total_substeps, saturated }`.
- `fixtures/gates/nsub-bucket/` — fuzzed `d²` straddling every bucket edge (±1 ulp and exact).

## Acceptance tests
- `cargo xtask gate nsub-bucket` — fuzz `d²` across and straddling every bucket edge: `N_sub` bit-identical on CPU-f64 and CPU-f32 (0 forks); the table is generated offline in f64 libm and only the f32 output ships; the tree equals `clamp(⌈(r_sub/r_min)^{γ_sub}⌉, 1, N_max)` away from edges (REQ-INT-028).
- `cargo test -p kernel nsub_live_readout` — the per-macro-step `N_sub` readout equals the bucket value used for that macro-step (REQ-INT-027).
- `cargo test -p kernel saturated_sticky` — a `d²` sequence that hits the cap once then relaxes keeps `saturated = 1`; with a non-power-of-two `N_max` the flag sets exactly when `N_sub` reaches `N_max` (REQ-TOOL-033).
- `cargo test -p kernel total_substeps_proxy` — `total_substeps` is an exact u32 equal to Σ N_sub; the proxy ⌊log₂ Σ N_sub⌋ via countLeadingZeros equals ⌊log₂⌋ of the exact count at powers of two and their neighbours, and 0 for totals 0 and 1 (REQ-TOOL-034).

## Notes
- The domain of the 0-fork result is identical inputs per step (pitfalls §10); state it in the gate's output.
- RQ-97 ruled: R-113 — REQ-INT-028 is split: this task proves CPU-f64 and CPU-f32 only; the native-GPU leg is REQ-VAL-059's `N_sub` (M4, TASK-M4-03) and the browser WGSL leg is REQ-VAL-144 (M8, TASK-M8-40).
