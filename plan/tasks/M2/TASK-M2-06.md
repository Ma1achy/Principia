# TASK-M2-06 — The kernel decode stage: monomorphised per chart, uniforms, GPU ICDescriptor

- **Milestone:** M2
- **Closes:** REQ-DEC-005, REQ-DEC-007, REQ-CHART-001, REQ-TOOL-030, REQ-DEC-043
- **Depends on:** TASK-M2-05, TASK-M1-11
- **Needs (earlier milestones):** REQ-GEN-004, REQ-PAY-002, REQ-TOOL-013, REQ-RENDER-001, REQ-SYS-004
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-10, PIT-9
- **Size:** ~480 lines

## Goal
The decoder runs in the kernel: a decode-stage entry compiled to f32 SPIR-V → WGSL (rust-gpu), monomorphised per chart and link set (the chart a type parameter, no runtime axis-kind or chart-id switch), reading z₀, q₁, q₂ and slice values as uniforms and DECODE_MODE as a per-quad workgroup-uniform flag, emitting the canonical (m, r, p) — the integrator's one input type — and writing the `ICDescriptor`. A native in-process wgpu test dispatch reads it back and compares against the CPU-f64 decode (`decodeOnly`), and the f32-eps scale factor of decoder test 12 is proposed from that measurement.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `docs/contracts/principia_chart_decoder_contract.md` § "Design axioms (the six that must survive contact with a code agent)"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `docs/design/principia_chart_reference.md` § "Chart reference — the maths, for implementation"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/contracts/principia_canonical_spec.md` § "3. The physics & manifold model *(authoritative: `chart_decoder_contract`, `dd_decoder`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_core_design.md` § "5. Canonicalisation is the one seam to (m, r, p)"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the defining decision)"
- `docs/design/principia_core_design.md` § "2. Authoring layer lowers to specialised kernels — by monomorphisation"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_lowering_contract.md` § "Part 3 — The baking rules (compile-time vs uniform)"
- `docs/contracts/principia_parity_contract.md` § "1. The principle"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"

- `decisions.md` § "R-118 — Φ is generic over the float type *(closes RQ-86)*"
## Deliverables
- `crates/kernel/src/stage/decode.rs`: the decode-stage entry generic over `Chart` and links; the (m, r, p) seam type as its only output to the integrator side; the per-quad DECODE_MODE flag (full path only — the linearised path lands in M6).
- `crates/engine/src/decode_dispatch.rs`: a native wgpu decode-only dispatch over a (u, v) grid for tests and the gates, reading back (m, r, p), the DEGENERATE tag and `ICDescriptor`.
- The CPU-f64 `decode_only(chart, uv, sim_key)` counterpart (parity contract §1's `decodeOnly`).
- Tests `crates/engine/tests/decode_stage_gpu.rs`; gate `cargo xtask gate decode-f32-eps` with its fixture in `fixtures/gates/decode_f32_eps/`.

## Acceptance tests
- Review (physics): the integrator-side entry takes only (m, r, p); no chart id or chart parameter is reachable from code past the seam; `cargo test -p engine decode_stage_identities` runs the identity suite (decoder §5 tests 1–4) on the kernel's actual output (REQ-DEC-005).
- Review (code + perf): the kernel has one monomorphised variant per chart/link set; no runtime axis-kind interpreter; `cargo test -p engine decode_uniforms_no_recompile` shows changing z₀ / q / slice values issues no pipeline creation; DECODE_MODE is a per-quad workgroup-uniform flag (REQ-DEC-007).
- Review (code): the compute kernel contains no runtime dispatch on axis kind or chart id; every chart is a generic instantiation (REQ-CHART-001).
- `cargo test -p engine decode_stage_descriptor` — GPU `ICDescriptor` fields (masses, virial_ratio, rho_ratio, rho_angle, K_0, V_0, r_min_pair_0) against the f64 decode, within the REQ-DEC-043 tolerance (REQ-TOOL-030).
- `cargo xtask gate decode-f32-eps` — measures f64-vs-f32 decode differences on the golden IC and fuzzed interiors and reports the multiple of f32 eps that covers them; the calibration proposal is reviewer-checked, confirmed by the human at the M2 gate and recorded in `decisions.md` (REQ-DEC-043).

## Notes
- The Tier-N tolerances are set later by the M4 measurement (REQ-VAL-064); REQ-DEC-043 is decoder test 12's own factor. The CPU-f64 Precision-ring surface as a whole (`computeIC` etc.) is REQ-SYS-017 (M3); this task delivers only `decodeOnly`.
- PIT-10: the measurement holds inputs fixed; state its domain (decode only, no trajectory).
- RQ-86 ruled: R-118 — the chart map is generic over the float type, so the decode stage instantiates each chart's `map` at f32 in the kernel and at f64 in `decode_only`.
