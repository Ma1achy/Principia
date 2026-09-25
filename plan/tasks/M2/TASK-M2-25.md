# TASK-M2-25 — Portable WGSL decode and encode, the DECODE preset and the agreement preset

- **Milestone:** M2
- **Closes:** REQ-COL-006, REQ-RENDER-027, REQ-TOOL-029
- **Depends on:** TASK-M2-16, TASK-M2-24, TASK-M1-14
- **Needs (earlier milestones):** REQ-COL-003, REQ-RENDER-005, REQ-RENDER-008, REQ-TOOL-011, REQ-TOOL-019, REQ-RENDER-001, REQ-COL-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.7, PIT-9, PIT-5
- **Size:** ~480 lines

## Goal
The chart decode and encode are available as portable WGSL to the fragment stage, so presets recompute decode(z) and encode(decode(z)) from `ctx.chart.z`. The DECODE preset — fragment-side decode, coloured (z components, masses as ternary, body positions) — certifies the decode rung before any physics, for every chart without special-casing, and its decoded (m, r, p) match the f64 `decodeOnly()`. The agreement preset |E(fragment-decode) − ctx.payload.E₀| is a live cross-implementation check that also exposes write-addressing faults (a deliberate dispatch scramble shows as spatial disagreement). Debug presets ship locked.

## References
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `decisions.md` § "R-41 — `DEBUG_MODE` uses the baked variants, not flag bits *(RS-1 (b))*"
- `docs/design/principia_debug_tooling_plan.md` § "A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `docs/design/principia_dd_decoder.md` § "1. What it is"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"

## Deliverables
- `crates/render/wgsl/decode.wgsl` and `crates/render/wgsl/encode.wgsl` as snippets for the fragment assembler (their provenance per Gap G1).
- The DECODE and agreement presets in the preset library (`crates/render/src/presets/`), locked.
- Golden suite `fixtures/golden/decode_preset/` (CPU-decoded reference image per chart); gate `cargo xtask gate decode-agreement`; test `crates/render/tests/decode_preset.rs`.

## Acceptance tests
- `cargo xtask gate decode-agreement` — the agreement preset |E(fragment-decode) − ctx.payload.E0| is at f32 noise on a healthy survey; a deliberate dispatch scramble shows spatial disagreement (REQ-COL-006).
- `cargo xtask golden decode-preset` — the DECODE preset rendered for each chart matches a CPU-decoded reference image (REQ-RENDER-027).
- `cargo test -p render decode_preset_vs_decode_only` — fragment-decoded (m, r, p) against the f64 `decodeOnly()` within the Tier-N tolerance (Gap G4); Σm = 1, CoM = 0 and I = 1 hold (REQ-TOOL-029).

## Notes
- Gap G1: canonical_spec §1 / dd_decoder §1 / REQ-SYS-015 allow no second transcription of the decode logic, but colour_composition §6 speaks of "the two decode ports" (WGSL-decode vs Rust-decode) and debug_tooling_plan of "the WGSL decode port". Whether the fragment WGSL is emitted from the shared Rust source (rust-gpu/naga) or hand-ported decides this task's deliverable and what the agreement preset certifies.
- Gap G2: `ctx.chart.z` ("the full 8-D latent at this pixel, chart triple applied") is not defined for nonlinear charts (shape sphere, invariant, Burrau), whose Φ doesn't pass through z — whether Φ is ported to WGSL too.
- Gap G3: "every chart in the lowering appendix" includes the Burrau int lattice and the Anosova physical-frame chart, which no M2 requirement builds (REQ-CHART-037, M4).
- Gap G4: the Tier-N tolerance is set by the M4 native-wgpu measurement (REQ-VAL-064).
- Gap G20: `ctx.payload.E0` is `SimState.E_0`, written by the integrating kernel (M4); at M2 only the decode stage's E₀ = K_0 + V_0 exists; "f32 noise" and "a healthy survey" are unquantified.
- Plan note: M1's REQ-TOOL-011 already lists the DECODE preset in the M1 catalogue, though the preset needs this task's WGSL decode.
