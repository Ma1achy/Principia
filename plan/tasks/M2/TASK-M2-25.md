# TASK-M2-25 — Portable WGSL decode and encode, the DECODE preset and the agreement preset

- **Milestone:** M2
- **Closes:** REQ-COL-006, REQ-RENDER-027, REQ-TOOL-029, REQ-COL-057, REQ-SYS-085
- **Depends on:** TASK-M2-16, TASK-M2-24, TASK-M1-14, TASK-M1-16, TASK-M1-19
- **Needs (earlier milestones):** REQ-COL-003, REQ-RENDER-005, REQ-RENDER-008, REQ-TOOL-011, REQ-TOOL-019, REQ-RENDER-001, REQ-COL-001, REQ-COL-063
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

- `decisions.md` § "R-394 — `ctx.chart.slice_uv` is the sample's position in the slice plane, stable under pan and zoom; a screen-relative position is a separate field *(closes RQ-257)*"
- `decisions.md` § "R-436 — The descriptor's `±∞` at the poles are stored as computed; every consumer, CPU and GPU alike, colours a non-finite input as invalid, tested by comparing the stored value, never by arithmetic on `∞` *(closes RQ-284; extends R-79)*"
- `decisions.md` § "R-437 — The value of every `atan2` at a possible pole of the decode, encode and canonicalisation is defined: 0, for every sign of the zeros, chosen branch-free by a select on the operands against 0, so CPU and GPU agree *(closes RQ-285)*"
- `decisions.md` § "R-628 — The descriptor's `rho_angle` is read from the configuration block's own `(ρ̃, λ̃)`, `Φ`'s vectors, not from the Jacobi vectors re-formed from the re-centred positions; the other descriptor fields keep their positional forms *(amends dd_decoder §3.6's "Each is read from the decoded `(m, r, p)` alone" for `rho_angle`; asked directly, no RQ)*"

## Deliverables
- `crates/render/wgsl/decode.wgsl` and `crates/render/wgsl/encode.wgsl` as snippets for the fragment assembler, emitted by the rust-gpu → SPIR-V → WGSL translation of the shared decode source (R-116); the build fails if either is edited by hand.
- The DECODE and agreement presets in the preset library (`crates/render/src/presets/`), locked.
- Golden suite `fixtures/golden/decode_preset/` (CPU-decoded reference image per chart); gate `cargo xtask gate decode-agreement`; test `crates/render/tests/decode_preset.rs`.

## Acceptance tests
- `cargo xtask gate decode-agreement` — the agreement preset |E(fragment-decode) − E₀|, with E₀ the decode stage's K₀ + V₀ (R-86), is within REQ-DEC-043's calibrated f32 decode factor on a healthy survey (R-133); a deliberate dispatch scramble shows spatial disagreement (REQ-COL-006).
- `cargo xtask golden decode-preset` — the DECODE preset rendered for each M2 chart (latent affine slice, shape sphere, (L_z, E), (L_z, K), ternary mass, Euclid ν plane, θ × K strip) matches a CPU-decoded reference image (REQ-RENDER-027; the Burrau int lattice and Anosova are REQ-RENDER-082, TASK-M4-06).
- `cargo test -p render decode_preset_vs_decode_only` — fragment-decoded (m, r, p) against the f64 `decodeOnly()` within REQ-DEC-043's calibrated f32 decode factor (R-133; Tier N is set at M4); Σm = 1, CoM = 0 and I = 1 hold (REQ-TOOL-029).
- `cargo test -p render read_side_shape_generated` — `read_side.wgsl`'s `shape()` is a build output of the rust-gpu → SPIR-V → WGSL translation of `kernel::shape`, not a string literal in `crates/ledger/src/gen/read.rs`; REQ-SYS-015's full-reach scan (`.wgsl` included) finds no hit; the twin test against `kernel::shape` still passes; control: a hand-written `mu_rho` formula planted in a `.wgsl` file, and one planted in a string in `crates/ledger/src/gen/read.rs`, are each found by the scan (REQ-SYS-085; R-547, RQ-365 (c)).
- Definition: `ctx.chart.z` on charts whose Φ does not produce z, written into colour_composition §3 and approved by the physics reviewer (REQ-COL-057).

## Notes
- R-116 settles Gap G1: the fragment WGSL is generated from the shared Rust source, never hand-ported; the agreement presets check the translation (colour_composition §3 and §6 were conformed in step 7).
- Gap G2: `ctx.chart.z` ("the full 8-D latent at this pixel, chart triple applied") is not defined for nonlinear charts (shape sphere, invariant, Burrau), whose Φ doesn't pass through z — whether Φ is ported to WGSL too.
- R-113 settles Gap G3: REQ-RENDER-027 covers the M2 charts; the Burrau int lattice and Anosova are REQ-RENDER-082 (TASK-M4-06).
- R-133 settles Gaps G4 and G20: until REQ-VAL-064 sets Tier N (M4) the tolerance is REQ-DEC-043's calibrated f32 decode factor, and the agreement preset's E₀ is the decode stage's K₀ + V₀, not `SimState.E_0`. "A healthy survey" stays unquantified.
- Plan note: M1's REQ-TOOL-011 already lists the DECODE preset in the M1 catalogue, though the preset needs this task's WGSL decode.
- RQ-84 ruled: R-116 (above). RQ-85 ruled: R-117 — the shape-sphere DECODE reference uses R-14's (θ, φ) map. RQ-94 ruled: R-113 — REQ-TOOL-011's DECODE row is this task's REQ-RENDER-027. RQ-95 ruled: R-113 (Gap G3 above).
- Closes, for gaps the corpus leaves open: REQ-COL-057 (R-72 definition) (classification accepted by R-132).
- R-297: the fragment may compile with fast-math and the compute kernel doesn't, so the agreement preset compares within a stated tolerance, not bit-exactly. TASK-M2-29 calibrates it (REQ-COL-060) and moves the agreement gate to it; until then this task's gate uses REQ-DEC-043's factor. R-297 amends R-133 here, and only here: `decode_preset_vs_decode_only` keeps REQ-DEC-043's factor.
- R-394 (RQ-257): `ctx.chart.slice_uv` is the sample's position in the slice plane, `c + h·(2·ctx.quad.uv − 1)`,
  stable under pan and zoom (REQ-COL-063). TASK-M1-16 conforms the lane, so this task, the first to place `ctx.chart.z`
  per pixel, depends on it.
- Depends on TASK-M1-19, applied per R-369 (TASK-M1-14's pre-flight): REQ-TOOL-011, which this task needs, moved from
  TASK-M1-14 to TASK-M1-19, split off so that RQ-273 holds only it. R-420 ruled RQ-273, so TASK-M1-19 waits on nothing.
- **R-436 (RQ-284):** the agreement preset tests each stored operand, `K_0` and `V_0` (and any other it reads), on
  its bits before any arithmetic, `K₀ + V₀` included, and draws a pixel with a non-finite one in the invalid colour on
  the GPU and the CPU twin alike; `E₀ = K₀ + V₀` is `−∞` at the f64 collision pole (dd_decoder §3.6). No arithmetic on
  `±∞` and no infinite literal (colour_composition §6; the GPU determinism note's rule 4). TASK-M2-31 builds the same
  test for the numeric views; this task may share its WGSL helper.
- **R-437 (RQ-285):** the generated WGSL decode and encode inherit the Rust source's selects at every `atan2` pole
  (dd_decoder §3.2 "Every `atan2` at a possible pole"); the agreement and DECODE checks include pole inputs with `±0`
  operands, where CPU and GPU agree. The holds RQ-284 and RQ-285 are gone.
- **R-436, the producers (physics review 5531809281 P1, applied per R-369):** the fragment decode forms its own `V`
  and `E`, so its `V` takes the decode stage's select (`−∞` bits wherever a pair separation is 0, written by bitcast,
  never `1/0`), inherited from the shared source (TASK-M2-06), and the agreement preset tests that `V` on its bits
  before it forms `E(fragment-decode) = K + V` (colour_composition §6; REQ-COL-006).
- **R-438, the `−x` half-axis (physics review 5532062124 P1, applied per R-369):** the generated WGSL encode inherits
  the Rust source's select of `β = π̂` where `λ̃_y` compares equal to 0 and `λ̃_x < 0` (TASK-M2-15; dd_encode §3.1),
  since WGSL may ignore a zero's sign through `atan2`; the ROUNDTRIP and agreement checks include that half-axis with
  `±0` operands, where CPU and GPU agree (REQ-COL-006).
- **R-628 (27 Oct 2026, the human's answer on PR #303):** the descriptor's `rho_angle` is read from the configuration
  block's own `(ρ̃, λ̃)`, `Φ`'s vectors, not from the re-centred positions (dd_decoder §3.6), so the generated decode
  takes it from them too, and the shape-pole test holds descriptor word 7 strictly, with near-pole points at `φ_f = 0`
  and a generic phase against the CPU's f32 decode and `Φ`'s f64 angle (physics review 5669268864, P1).
