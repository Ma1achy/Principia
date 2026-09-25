# TASK-M8-15 — Inspector pane 2: the canonical representative, the gauge buttons, the round-trip ghost, the JS oracle

- **Milestone:** M8
- **Closes:** REQ-GUI-057, REQ-GUI-058, REQ-GUI-059, REQ-GUI-060, REQ-GUI-064, REQ-GUI-065, REQ-GUI-114, REQ-GUI-118, REQ-VAL-109
- **Depends on:** TASK-M8-14, TASK-M2-27
- **Needs (earlier milestones):** REQ-DEC-002, REQ-VAL-022, REQ-CHART-036, REQ-INT-003, REQ-VAL-023, REQ-ENC-020, REQ-SYS-015
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-9
- **Size:** ~500 lines

## Goal
The Jacobi frame (ρ, λ), the interior vertex angles and the labelled CoM are always drawn on both the editing and canonical panes, with no toggles, and both panes auto-fit. Pane 2 shows the canonical representative (CoM at the origin, ρ̃ on +x, R̃ = 1, CoM frame) recomputed on every edit, as bodies with momentum arrows (the chart's Jacobi q, not velocity) or as the shape sphere; the unwrap is the mass-weighted equirect with θ horizontal, φ vertical, L⁺ at the top, collinear shapes on the equator, landmarks from the actual masses, and the lower half shaded mirror-folded (R-14, R-50). "Ghost the gauge transform" animates translate → boost → rotate → scale. Reset, Randomise, Burrau-ish, Rotate 15°, Scale × 1.25 and Boost → leave the canonical pane and z unchanged. The round-trip ghost overlays decode(z) pushed back through g⁻¹ and reports the error. The Inspector calls the engine's actual Rust decode and encode, and the standalone JS tool is kept as an independent oracle cross-checked on random ICs.

## References
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "05 Inspector — one IC, its trajectory, one timeline"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/notes/ic_inspector_scratchpad.md` § "Physical canvas — interaction"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `docs/notes/ic_inspector_scratchpad.md` § "Canonical window (separate panel)"
- `docs/notes/ic_inspector_scratchpad.md` § "The quotient set (the thing being audited)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/notes/ic_inspector_scratchpad.md` § "Whole-system handles — the invariance audit"
- `docs/notes/ic_inspector_scratchpad.md` § "Build notes"
- `decisions.md` § "R-8 — The IC Inspector copies"

## Deliverables
- `crates/gui/src/widgets/body_canvas.rs` — shared by panes 1 and 2: Jacobi frame, angles, CoM, auto-fit.
- `crates/gui/src/windows/inspector/pane_canonical.rs` — bodies / sphere toggle, gauge ghost animation, √I = 1 stamp, round-trip ghost.
- `crates/gui/src/windows/inspector/gauge_buttons.rs`.
- `crates/validation/src/js_oracle.rs` + `crates/validation/js/run_oracle.mjs` — runs `docs/gui/reference/ic_inspector.html`'s decode maths under Node on random ICs and compares n with the Rust.
- Tests: `canonical_rep`, `unwrap_landmarks`, `roundtrip_ghost` (including a deliberately broken decode that must show an offset), `gauge_invariance` (proptest), `js_oracle_n`; screenshot cases `05_inspectors/pane2_bodies`, `05_inspectors/pane2_sphere`, `05_inspectors/ghost_stage{1..4}`.

## Acceptance tests
- `cargo xtask screenshot 05_inspectors` (canonical pane) — screenshot: canonical arrows are labelled/drawn as momenta (REQ-GUI-057).
- `cargo xtask screenshot 05_inspectors` (both panes) — screenshot of both panes against 05_inspectors.png (REQ-GUI-058).
- `cargo xtask screenshot 05_inspectors` (large and small configurations; √I = 1 stamp) — screenshot with a large and a small configuration: both fit; the stamp is present (REQ-GUI-059).
- `cargo test -p gui unwrap_landmarks` — landmarks for masses (1,1,1) and (1,2,3) match the mass-weighted formula; L⁺ maps to the top edge; collinear ICs map to φ = 90° (REQ-GUI-060).
- `cargo test -p engine canonical_rep` — after random edits the canonical representative has CoM = 0, P = 0, ρ̃_y = 0 with ρ̃_x > 0, and R̃ = 1 (REQ-GUI-064).
- `cargo test -p engine roundtrip_ghost` — for random ICs the ghost coincides with the dragged config to the round-trip bound; a deliberately broken decode shows a visible offset (REQ-GUI-065).
- `cargo xtask screenshot 05_inspectors` (both views and the four ghost stages) — screenshots of both views and of the ghost animation's four stages against 05_inspectors.png (REQ-GUI-114).
- `cargo test -p engine gauge_invariance` — random ICs: Rotate, Scale and Boost leave the canonical pane's representative and z unchanged (see the gauge gate) (REQ-GUI-118).
- `cargo test -p validation js_oracle_n` — random ICs: Rust n and the JS tool's n agree to the round-trip bound; the Inspector links no second decode implementation (REQ-VAL-109).

## Notes
- RQ-71 (whether the shape sphere is a double cover or a 2-to-1 fold) touches the mirror-folded shading of REQ-GUI-060; no requirement closed here carries it, but the physics reviewer checks the shading against whatever RQ-71 rules.
