# TASK-M6-48 — The GUI track (R-390): RQ-340 ruled — Tip as sheet the default, Lean each axis an option, over the pole, the compass drawing the active map, the readout's lines (R-474) ⚑

- **Milestone:** M6
- **Closes:** REQ-GUI-231, REQ-GUI-232, REQ-GUI-233, REQ-GUI-234, REQ-GUI-235, REQ-GUI-236, REQ-GUI-237, REQ-GUI-238
- **Depends on:** TASK-M6-38, TASK-M6-39, TASK-M6-40, TASK-M6-45
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
On the mock: `View::basis` gains Tip as sheet (Rodrigues about `k = (τ₂e_a − τ₁e_b)/m`), the default, with Lean each axis (today's) an option, selected by `SimConfig`'s `tilt_map`; the Tilt section's map selector after `γ`; Tip as sheet over the pole to ±180°, wrapped at 360°, never folded; same-plane comparisons up to sign, never written back; the compass draws the active map, B11 under Lean only; the readout's components, Lean's warnings and the mirrored-partner line; exactly the `X = Y`, both-angles views re-integrate (C10).

## References
- `decisions.md` § "R-474 — RQ-340 is ruled: two tilt maps, "Tip as sheet", the nav cube's axis-angle map and the default, and "Lean each axis", today's, an option, differing only with one target and both angles set; Tip as sheet carries on over the pole, wrapped only in total tip at 360° and never folded onto its 180° partner; planes are compared up to sign and never written back; the map is recorded everywhere as `tilt_map`; the compass draws the active map, B11 revisited; Lean's Gram factor and warnings; every readout shows each axis's components *(closes RQ-340; amends R-451, R-457 and R-459)*"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `decisions.md` § "R-459 — The cube's H, V and D axis labels are buttons, each a dropdown of latent dimensions, no dimension twice, each pick one undoable edit, H and V in step with Chart's horizontal q₁ and vertical q₂; in Tilt mode the third axis shows what the gesture tilts into, and a "4D: see Plücker" badge appears when both angles are set into different targets *(amends R-451 B1's labels)*"
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"

## Deliverables
- `crates/gui/src/explore/manifold_view/mod.rs` (`View::basis`, both maps), `slice_tilt.rs` (the selector, the ranges: `TILT_RANGE` ±180° under Sheet), `compass.rs` (`Drag::moved`/tilt, the active map, the readout).
- `SimConfig`'s `tilt_map` in the contract (`crates/engine/src/contract/sim_config.rs`), default Sheet, its absence read as Lean.
- The mock capture `mock_compass_cube_tilt_hex` regenerated: `fixtures/screenshot/01_main/compass/decoder/mock_compass_cube_tilt_hex.json` and `fixtures/screenshot/01_main/compass/mock_compass_cube_tilt_hex.json` (R-474 K12; a fixture of the mock's capture, listed in the PR, not a golden of `fixtures/golden/`); a capture case `mock_tilt_map_selector`.
- Tests `tilt_maps`, `tilt_over_the_pole`, `same_plane_readout`, `tilt_map_state`, `compass_active_map`, `lean_warnings`, `readout_components`, `reintegration_scope`.

## Acceptance tests
- `cargo test -p gui tilt_maps` and `cargo xtask screenshot 01_main` (`mock_tilt_map_selector`) — REQ-GUI-231's verify (C2, C3, C5).
- `cargo test -p gui tilt_over_the_pole` — REQ-GUI-232's verify (C4).
- `cargo test -p gui same_plane_readout` — REQ-GUI-233's verify.
- `cargo test -p gui tilt_map_state` — REQ-GUI-234's verify.
- `cargo test -p gui compass_active_map` and `cargo xtask screenshot 01_main` — REQ-GUI-235's verify.
- `cargo test -p gui lean_warnings` — REQ-GUI-236's verify.
- `cargo test -p gui readout_components` — REQ-GUI-237's verify.
- `cargo test -p gui reintegration_scope` — REQ-GUI-238's verify (C10).

## Notes
- **⚑ Physics review is required** (the instruction's "How to work this", 4): this task does not merge on the code reviewer's word alone; its reviewer list includes physics. The tilt maths (Tip as sheet, the wrap, the plane comparison) is physics-reviewed.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green. With the ⚑ above, physics must approve too.
- **qa's files (R-290, R-474 K14; applied per R-369, a ruling forcing the change).** These must change in this task's qa commit and only there, the PR listing each `M` line with its reason, the code reviewer confirming no other assertion is weakened (a test meaning the Lean map may set `tilt_map = Lean` and keep its assertions): `crates/gui/tests/qa_TASK-M6-33.rs` (helpers ~:95-135 and the doc :11-16; :275-353, its axis-angle control inverted; :427-500; :559-591; :593-604; :606-700; :1064-1286; :1359-1418; :1422-1470; :1522-1634, the 90° clamp; :1636-), `crates/gui/tests/qa_TASK-M6-37.rs` (:98-116; :215-625; :705-) and `xtask/tests/qa_TASK-M6-33.rs` (:370-404 and the doc :11-13; :406-424; :558-563). The implementer's own tests (`crates/gui/src/tests/compass_cross_section.rs`, `mock_compass_cube.rs`, `mock_compass.rs`, `mock_manifold_view.rs`, `degenerate_tilt_guards.rs`, `compass_support.rs`) are theirs to change. The qa reviewer confirms the list against the build.
- **The views that re-integrate (R10):** exactly those with `X = Y` and both angles set; no golden of `fixtures/golden/` holds a tilted view.
- **Review round 1 of PR #255 (applied per R-369):** two more qa files, `crates/engine/tests/qa_TASK-M0-18.rs` (`SIM_TEXT` :57-61) and `crates/prin/tests/qa_TASK-M0-18.rs` (`expected_config_text` :162-185), pin `SimConfig`'s canonical text that `tilt_map` changes (code review F2; R-474 K14, which also names the `slice: Slice {}` struct literals that change only if the new field breaks them); choosing Lean each axis clamps an angle past ±90° in the same history entry (gui review G1; R-474 K15, REQ-GUI-232); the map radios follow `γ` and precede the Tilt reset in the keyboard order (G7; K7, REQ-GUI-231). The contract's restatement of R1 says the plane, not its normal, tips toward `e_X` (physics review F5; K16).
