# TASK-M2-25 — Portable WGSL decode and encode, the DECODE preset and the agreement preset

- **Milestone:** M2
- **Closes:** REQ-COL-006, REQ-RENDER-027, REQ-TOOL-029, REQ-COL-057
- **Depends on:** TASK-M2-16, TASK-M2-24, TASK-M1-14
- **Needs (earlier milestones):** REQ-COL-003, REQ-RENDER-005, REQ-RENDER-008, REQ-TOOL-011, REQ-TOOL-019, REQ-RENDER-001, REQ-COL-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-1.7, PIT-9, PIT-5
- **Size:** ~480 lines

## Goal
The chart decode and encode are available to the fragment stage as WGSL generated from the one Rust source (rust-gpu → SPIR-V → WGSL, R-116; never hand-written), so presets recompute decode(z) and encode(decode(z)) from `ctx.chart.z`. The DECODE preset — fragment-side decode, coloured (z components, masses as ternary, body positions) — certifies the decode rung before any physics, for every chart built by M2 without special-casing, and its decoded (m, r, p) match the f64 `decodeOnly()`. The agreement preset |E(fragment-decode) − E₀| is a live check of the translation that also exposes write-addressing faults (a deliberate dispatch scramble shows as spatial disagreement). Debug presets ship locked.

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
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-116 — The fragment decode and encode are generated from the one source *(closes RQ-84)*"
- `decisions.md` § "R-117 — The lowering appendix's shape-sphere row uses (θ, φ) *(closes RQ-85)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"
## Deliverables
- `crates/render/wgsl/decode.wgsl` and `crates/render/wgsl/encode.wgsl` as snippets for the fragment assembler, emitted by the rust-gpu → SPIR-V → WGSL translation of the shared decode source (R-116); the build fails if either is edited by hand.
- The DECODE and agreement presets in the preset library (`crates/render/src/presets/`), locked.
- Golden suite `fixtures/golden/decode_preset/` (CPU-decoded reference image per chart); gate `cargo xtask gate decode-agreement`; test `crates/render/tests/decode_preset.rs`.

## Acceptance tests
- `cargo xtask gate decode-agreement` — the agreement preset |E(fragment-decode) − E₀|, with E₀ the decode stage's K₀ + V₀ (R-86), is within REQ-DEC-043's calibrated f32 decode factor on a healthy survey (R-133); a deliberate dispatch scramble shows spatial disagreement (REQ-COL-006).
- `cargo xtask golden decode-preset` — the DECODE preset rendered for each M2 chart (latent affine slice, shape sphere, (L_z, E), (L_z, K), ternary mass, Euclid ν plane, θ × K strip) matches a CPU-decoded reference image (REQ-RENDER-027; the Burrau int lattice and Anosova are REQ-RENDER-082, TASK-M4-06).
- `cargo test -p render decode_preset_vs_decode_only` — fragment-decoded (m, r, p) against the f64 `decodeOnly()` within REQ-DEC-043's calibrated f32 decode factor (R-133; Tier N is set at M4); Σm = 1, CoM = 0 and I = 1 hold (REQ-TOOL-029).
- Definition: `ctx.chart.z` on charts whose Φ does not produce z, written into colour_composition §3 and approved by the physics reviewer (REQ-COL-057).

## Notes
- R-116 settles Gap G1: the fragment WGSL is generated from the shared Rust source, never hand-ported; the agreement presets check the translation (colour_composition §3 and §6 were conformed in step 7).
- Gap G2: `ctx.chart.z` ("the full 8-D latent at this pixel, chart triple applied") is not defined for nonlinear charts (shape sphere, invariant, Burrau), whose Φ doesn't pass through z — whether Φ is ported to WGSL too.
- R-113 settles Gap G3: REQ-RENDER-027 covers the M2 charts; the Burrau int lattice and Anosova are REQ-RENDER-082 (TASK-M4-06).
- R-133 settles Gaps G4 and G20: until REQ-VAL-064 sets Tier N (M4) the tolerance is REQ-DEC-043's calibrated f32 decode factor, and the agreement preset's E₀ is the decode stage's K₀ + V₀, not `SimState.E_0`. "A healthy survey" stays unquantified.
- Plan note: M1's REQ-TOOL-011 already lists the DECODE preset in the M1 catalogue, though the preset needs this task's WGSL decode.
- RQ-84 ruled: R-116 (above). RQ-85 ruled: R-117 — the shape-sphere DECODE reference uses R-14's (θ, φ) map. RQ-94 ruled: R-113 — REQ-TOOL-011's DECODE row is this task's REQ-RENDER-027. RQ-95 ruled: R-113 (Gap G3 above).
- Closes, for gaps the corpus leaves open: REQ-COL-057 (R-72 definition) (classification accepted by R-132).
