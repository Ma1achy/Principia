# TASK-M1-03 — The shared prelude: range_norm, DEBUG_NAN, has_<feature> consts and the dbg_* presentation layer

- **Milestone:** M1
- **Closes:** REQ-RENDER-014, REQ-RENDER-020, REQ-RENDER-021, REQ-TOOL-009
- **Depends on:** TASK-M1-01
- **Needs (earlier milestones):** REQ-GEN-001, REQ-GEN-002, REQ-RENDER-001
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
The fixed shared prelude every fragment node (built-in, debug or custom) can call exists and is emitted from the one Rust layout definition: the ramps the M1 views need (`ramp_viridis`, `hue_wheel`, Twilight, a greyscale ramp), `range_norm(x, lo, hi, auto, meas)`, the reserved invalid colour `DEBUG_NAN`, and the baked `const bool has_ftle / has_ensemble / has_word` per variant. Beside it, the small hand-written presentation layer the debug views reuse: `dbg_cat`, `dbg_lin`, `dbg_log`, `dbg_flag`, `dbg_hash_u32`, `dbg_sentinel`.

## References
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/contracts/principia_gui_state_contract.md` § "3. The registry is the scanned filesystem — for the *fragment* side; compute occupants are Rust build variants"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `decisions.md` § "R-16 — The colour PDF's map lists are ported *(closes RQ-14)*"
- `docs/gui/principia_render_gui_spec.md` § "13. Invariants"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `docs/design/principia_colour_composition.md` § "3. The `ctx` contract"

## Deliverables
- `crates/ledger`: prelude emission (`shaders/wgsl/lib/prelude.wgsl`, generated) — `range_norm`, `DEBUG_NAN`, the ramps, and per-variant `has_<feature>` consts derived from the variant's tier bits.
- `crates/render/shaders/wgsl/lib/present.wgsl` (hand-written): the six `dbg_*` helpers, with a Rust CPU mirror in `crates/render/src/present.rs` for the assertions.
- Tests: `crates/render/tests/prelude.rs` (GPU compile + CPU-vs-shader evaluation).

## Acceptance tests
- `cargo test -p ledger prelude_has_consts` — the generated prelude contains `has_ftle`, `has_ensemble`, `has_word` with values matching the variant's tier bits (REQ-RENDER-014).
- `cargo test -p render prelude_generated` — the prelude is produced by the layout build step (not a hand file), and a custom node calling `ramp_viridis` compiles (REQ-RENDER-020).
- `cargo test -p render range_norm` — CPU reference vs shader for x in and out of range with auto on and off; out-of-range clamps under fixed (REQ-RENDER-021).
- `cargo test -p render dbg_helpers` — each helper returns the stated colour for fixture inputs; `dbg_sentinel(-1.0)` == magenta; NaN bits → hatch pattern (REQ-TOOL-009).

## Notes
- Open RQ-75 (REQ-RENDER-014): whether the fragment side keeps a baked `has_ensemble` or reads it as a uniform (R-102 made `copy_index` a compute uniform). The task emits `has_ftle` and `has_word` now; `has_ensemble` waits on the ruling.
- Not given by the corpus (milestone Gaps): the invalid-colour magenta value `DEBUG_NAN` (R-16: a plain default, no source), the hatch pattern for NaN in `dbg_sentinel`, the Okabe–Ito swatch values and the golden-angle L/C for `dbg_cat`, the forms of `dbg_log` and `dbg_hash_u32`, and the flag green/red values. The LUT data (Viridis, Twilight) is taken from the reference artefacts colour_composition §7 names as the oracle.
- `meas` for `auto = true` comes from `QuadReduction` in production (M5); at M1 the test supplies it as a uniform from a CPU min/max over the synthetic buffer.
