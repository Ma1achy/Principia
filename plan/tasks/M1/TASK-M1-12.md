# TASK-M1-12 — Derived, live-shape, accumulator, word and ICDescriptor views

- **Milestone:** M1
- **Closes:** REQ-TOOL-010, REQ-TOOL-024, REQ-TOOL-025, REQ-VAL-010, REQ-VAL-011, REQ-VAL-122
- **Depends on:** TASK-M1-02, TASK-M1-09, TASK-M1-11
- **Needs (earlier milestones):** REQ-PAY-001, REQ-PAY-010, REQ-GEN-006
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~500 lines

## Goal
The fragment-computed views exist over the synthetic payload: live shape views (`u_mode` 0: ½(n+1) direction cosines of the current derived n; 1: Twilight of θ̃; 2: |n|−1 error), accumulator views (running S/t, the Welford slope C_ty/C_tt, drift running-max vs final), derived views (orbit_count / retrograde from θ̃, the reduced crossing count, finalised ftle, current drift H(r,p) − E₀), the word inspector (reduced length invalid-styled when truncated, symbol-at-k slider, truncated flag, whole-word hash) and the ICDescriptor views. For the vector field n the generated occupants offer '‖·‖ as scalar' beside 'as direction-cosines', paired with a test asserting ‖n‖ = 1 to a calibrated tolerance. The t_dmin round-trip view certifies the 16-bit step index.

## References
- `docs/contracts/principia_render_contract.md` § "Live-state & array inspection"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"

## Deliverables
- `crates/render/shaders/wgsl/frag/debug/` (hand-written, registry-scanned): live-shape, accumulator, derived and word-inspector views; WGSL `shape_n` / Hamiltonian helpers in the prelude library with Rust twins from the kernel.
- Generated ICDescriptor views (ternary masses, per-scale scalars) via the catalogue (TASK-M1-08).
- The VAL-122 proposal: ‖n‖ − 1 measured over a test render at f32, with the stated tolerance, attached to the PR.
- Golden/screenshot fixtures `fixtures/golden/m1-derived/`.

## Acceptance tests
- `cargo xtask screenshot debug-views` — each view in the debug picker renders on a synthetic payload; the |n|−1 view is flat zero on the synthetic payload (the real-march form waits on M3) (REQ-TOOL-010).
- `cargo test -p render derived_views_match_cpu` — orbit_count/retrograde, reduced crossing count, finalised ftle and current drift, each computed in the fragment, equal a CPU reference on synthetic states (REQ-TOOL-024).
- `cargo test -p render word_hash_and_symbol_at_k` — distinct synthetic words hash to distinct colours; symbol-at-k matches the appended sequence (REQ-TOOL-025).
- `cargo test -p render norm_n_views` — the registry lists both views for n; ‖n‖ − 1 is within tolerance: REQ-VAL-122 (calibrated) over a test render (REQ-VAL-010).
- `cargo test -p ledger t_dmin_roundtrip` — pack/unpack `t_dmin_step` round-trips exactly for all u16 values (REQ-VAL-011).
- Review checklist (physics): the proposal measures ‖n‖ − 1 over a test render at f32 and states the tolerance; the human confirms it at the M1 gate and it is recorded in decisions.md (REQ-VAL-122).

## Notes
- REQ-VAL-122 is a calibration (R-71); REQ-VAL-010's acceptance uses the proposed value until the human confirms it.
- Not available at M1 (milestone Gaps): REQ-TOOL-010's "|n|−1 flat zero on a real march" and the live current-substep effort heatmap both need a marching integrator (M3); the GUI-screenshot runner compares against `docs/gui/design/NN_*.png`, and no artboard shows the debug views; the ledger metadata schema (generation root §3.8) has no vector type, so "the ledger knows n is a vector" (gui_state_contract §4) has nothing to key on.
- `dbg_hash_u32`'s hash is not specified (TASK-M1-03's gap); REQ-TOOL-025's "distinct colours" is asserted over a fixed fixture set of words.
- Waits on RQ-93 (`REVIEW_QUEUE.md`): M0 requirements that need things M0 doesn't have.
- Waits on RQ-94 (`REVIEW_QUEUE.md`): M1 requirements that need M2, M3, M5 or M8.
- Waits on RQ-101 (`REVIEW_QUEUE.md`): The colour golden oracle and the LUT data live outside the corpus.
