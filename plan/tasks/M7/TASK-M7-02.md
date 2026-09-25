# TASK-M7-02 — Colour spaces: sRGB transfer, OKLab and OKLCH, transcription-checked against Ottosson

- **Milestone:** M7
- **Closes:** REQ-COL-033, REQ-COL-049
- **Depends on:** TASK-M1-04
- **Needs (earlier milestones):** REQ-RENDER-020, REQ-RENDER-016
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The shared WGSL library and its Rust mirror carry dd_colouring §3.1's exact transforms — the sRGB transfer (0.04045 / 0.0031308 thresholds), linear sRGB → OKLab through M₁, the cube root and M₂, the closed inverse coefficients and M₁⁻¹, and the OKLCH polar form — each coefficient checked against Ottosson's reference implementation before it enters the shared source (R-51). The round-trip tolerance of dd_colouring unit test 1 is proposed with its measured evidence (R-71) for confirmation at the M7 gate.

## References
- `docs/design/principia_dd_colouring.md` § "3.1 Colour spaces (exact transforms — transcription-check against Ottosson before entering the shared source)"
- `docs/design/principia_dd_colouring.md` § "6. Deferred / flagged"
- `decisions.md` § "R-51 — The OKLab coefficients are checked against Ottosson *(CO-2)*"
- `open-questions.md` § "Open questions"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"

## Deliverables
- `crates/render/shaders/wgsl/lib/colour_space.wgsl` — `srgb_to_linear`, `linear_to_srgb`, `srgb_to_oklab`, `oklab_to_srgb`, `oklab_to_oklch`, `oklch_to_oklab` (the prelude members render_gui_spec §10.1 names); replaces or confirms the M1 prelude's colour-space maps.
- `crates/render/src/colour/space.rs` — the same transforms in Rust (f32 and f64) for tests and CPU-side generators (swatches, palettes).
- `crates/render/tests/oklab_transcription.rs` — every §3.1 coefficient compared with Ottosson's published values; anchors (white, primaries); gamut-lattice round trips on the CPU and through the M0 GPU test dispatch of the WGSL.
- `fixtures/gates/oklab-roundtrip/` and the `oklab-roundtrip` gate: the gamut lattice and the measured f32 round-trip error.
- The R-71 proposal in the PR: measured maximum error, proposed tolerance, margin.
- Only if a §3.1 digit is wrong: the dd_colouring correction under R-51, with its "Removed lines" note.

## Acceptance tests
- `cargo test -p render oklab_transcription` — dd_colouring unit test 1: round trips across a gamut lattice within tolerance REQ-COL-049 (calibrated); white → (1, 0, 0); primaries match Ottosson's published values; WGSL and Rust agree (REQ-COL-033).
- `cargo xtask gate oklab-roundtrip` — the proposal shows the measured f32 round-trip error over the gamut lattice and the margin the tolerance leaves above it; confirmed by the human at the M7 gate and recorded in decisions.md (REQ-COL-049).

## Notes
- Calibration (R-71): REQ-COL-049's value is proposed here, checked by the physics reviewer, confirmed at the gate. Until the M7 gate confirms it, CI runs `oklab_transcription` and the gate against the proposed value, marked provisional (R-182).
- PIT-3 ("check the measurement can fire"): the gate must show a deliberately perturbed coefficient fails it.
- Name the Ottosson reference (URL and revision) in the test file.
