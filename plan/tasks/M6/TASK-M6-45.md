# TASK-M6-45 — Resets for Slice centre, Tilt and Navigate, and Home (button, key, View › Home): the chart preset's own values, one undoable command each, dimmed when home, the compass live (R-466)

- **Milestone:** M6
- **Closes:** REQ-GUI-218, REQ-GUI-219, REQ-GUI-220, REQ-GUI-221, REQ-GUI-222, REQ-GUI-223, REQ-GUI-224
- **Depends on:** TASK-M6-38, TASK-M6-39
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~550 lines

## Goal
On the mock engine, each of the Manifold view's Slice centre, Tilt and Navigate sections gets a "reset" beside its heading that returns that section, and only it, to the active chart preset's own values, never literal zero, and Home does all three in one command, from a house button in the figure canvas's top-right corner, the Home key in both keyboard modes and View › Home (R-466). First, as a definition (R-72, REQ-GUI-224), this task writes what the corpus lacks: each chart preset's base point, centre `(u, v)` and window (the scale at which the zoom reads × 1.00), in render_gui_spec §G2 and §G7, the mock's values placeholder content (R-390), and the mock's presets carry them; the resets read them and hardcode nothing. Slice centre's reset moves `z₀` across the plane only, Tilt's edits the basis only (`τ₁ = τ₂ = γ = 0`, `z₀`, the zoom, the targets and H, V, D kept), Navigate's pans within the plane and sets the zoom (R-466 K2). Each reset and Home is one undoable command (one history key, R-458's mechanism), dimmed while already home, and drives the compass live without switching its mode. Home leaves the preset and its axes, H, V, D and the targets, the playhead, the stain, the layout, the lock and the rest of `ViewUI` alone.

## References
- `decisions.md` § "R-466 — Slice centre, Tilt and Navigate each have a "reset" beside the heading that returns the section to the active chart preset's own values, never literal zero; Home does all three in one command, from a button in the canvas's corner, the Home key in both keyboard modes and View › Home; each is one undoable command, dimmed when already home, and drives the compass live *(extends R-409, R-457, R-458 and R-462)*"
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `decisions.md` § "R-458 — Drags are live in both directions: a drag on the compass or on any slider applies its edit every frame, at most once a frame, so the sliders, the compass, its readout and the figure follow the drag, and the whole drag is one undo entry; a drag on a Slice centre slider switches the compass to Slice and one on a Tilt slider to Tilt, its plane moving with the drag *(amends R-409 A3 and R-451 B3 as they send one `SetField` per drag)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-450 — The console has two scopes, its header bar and its log: opening it lands on the header bar, Enter goes in and ← → cross every header control, × included; Enter on the log scrolls it entry by entry, the whole log reachable *(amends R-405 and R-409 as they apply to the console's structure)*"
- `decisions.md` § "R-456 — A click on a text input starts typing, in interaction mode; a click anywhere else lands in navigation mode *(amends R-450 A3, and R-409's landing rule and G4, for text inputs)*"
- `decisions.md` § "R-460 — Focus and keyboard navigation are separate: a window opened with the mouse takes the focus but shows no ring and leaves keyboard navigation off; keyboard navigation starts only on keyboard input; a test per entry point, with a keyboard control arm *(amends R-409 point 4's "always in one of two modes" and R-450's landing "the whole scope highlighted", for a window opened with the mouse; reads R-456's blue ring on a clicked text field as shown only while keyboard navigation is on)*"
- `decisions.md` § "R-464 — After any mouse click or drag the ring is hidden, the focus moving exactly as R-450 A3 and R-456 say; the next key press shows it again where the focus is; a window or the console opened with the mouse takes the focus with no ring, opened from the keyboard with it *(closes RQ-367; amends R-450, R-456 and R-460 as to whether the ring is drawn)*"
- `decisions.md` § "R-462 — Every panel boundary is a draggable splitter, the canvas taking the space left; each panel has a minimum size, a double-click resets a splitter, the layout is remembered and View › reset layout restores it; resizing the canvas re-frames the figure and never re-integrates it"
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 4 — Navigation is chart construction (pan, slice, zoom, tilt, lock)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"

## Deliverables
- The definition (REQ-GUI-224): render_gui_spec §G2 and §G7 give each chart preset's base point, centre and window, the zoom row reading `log₂(σ_h / σ)` and `σ_h / σ`; the mock's presets (`crates/gui/src/explore/manifold_view/mod.rs`, `PRESETS`) carry them, placeholder content (R-390), the test preset's base point with non-zero `z_μ1`, `z_μ2` and a centre off `(0, 0)`, and a second preset with other values; a "Removed lines" note for any line changed.
- The three resets (REQ-GUI-218): a "reset" button at the right of each section's heading row (`navigate.rs`, `centre.rs`, `slice_tilt.rs`), each the last element of its sub-scope, Enter acting as its click; tooltips "Reset Slice centre …", "Reset Tilt …", "Reset Navigate …" (R-466 K6, K9).
- Home (REQ-GUI-219, REQ-GUI-220): the 24 × 24 point house button in the canvas's top-right corner, inset 8 points, painted with egui strokes, hidden with the layer, its click no figure click (K7); the Home key as a global command in the keymap (`crates/gui/src/keyboard/keymap.rs`) and the `?` overlay's row, both modes, except on the console's log in interaction mode and in a text field being typed in, Shift and Alt leaving it Home, Ctrl and Cmd not, no repeat (K1); View › Home the View menu's first entry with "Home" as its shortcut (K8).
- One undoable command each (REQ-GUI-221): the edits sent in one frame under one fresh history key (TASK-M6-39's mechanism), nothing sent at home (K10); locked, each edit through the lock's rule for its kind, Home's `z₀` edit last (K5).
- Dimming (REQ-GUI-222): egui's disabled style at home within 10⁻¹², the order kept, and with no active preset (K3, K11).
- The compass live (REQ-GUI-223): the new state in the next frame, the cube easing as after a click, no mode switch (K12).
- Tests `mock_manifold_resets`, `mock_home`, `mock_home_leaves_alone`, `mock_reset_undo`, `mock_reset_dimming` and `mock_reset_compass_live`.

## Acceptance tests
- cargo test -p gui mock_manifold_resets: for each reset, all three sections moved first (a Slice centre slider and z_μ1's number box, τ₁, τ₂ and γ non-zero with a target changed, a pan and a zoom); the section reads exactly the preset's values (within 10⁻¹²; untilted, bit for bit) and the other two as before (the Tilt reset: `z₀` bit for bit and the zoom); controls: a reset that does nothing, one to literal zero, one that also resets another section, and one hardcoding the first preset's values (re-run under a second preset) each fail (REQ-GUI-218).
- cargo test -p gui mock_home: from the button, the key in navigation and in interaction mode, and View › Home, the home view; controls: a Home of one or two sections, and a button click that pins or pans, fail; Home on the console's log in interaction mode goes to the first entry and in a typed field moves the caret, resetting nothing; Ctrl+Home resets nothing; F3 hides the button (REQ-GUI-219).
- cargo test -p gui mock_home_leaves_alone: the preset and its axes, the slice frame, the playhead, the stain graph, the layout, the lock, the compass's mode and view and the overlays unchanged by Home; control: a Home touching the playhead, a target, D or the layout fails (REQ-GUI-220).
- cargo test -p gui mock_reset_undo: each command raises the undo depth by exactly 1, one Ctrl+Z restores all it set bit for bit, one redo sets it again; control: two entries fail; at home the depth is unchanged (REQ-GUI-221).
- cargo test -p gui mock_reset_dimming: disabled at home, enabled after its section moves, Home enabled while any is off home, all disabled with no active preset; controls as in REQ-GUI-222.
- cargo test -p gui mock_reset_compass_live: the compass's target plane and readout follow in the next frame, the mode kept, in Tilt and in Slice mode; controls as in REQ-GUI-223.
- Review checklist (physics reviewer, REQ-GUI-224): §G2 and §G7 define the base point, centre and window of every preset, the zoom row relative to the window, and the mock reads them.

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: move a Slice centre slider, tilt and pan and zoom; click Tilt's "reset": only the angles return to 0 and the compass follows; click Slice centre's and Navigate's; each dims once home. Move all three again and press Home, then Ctrl+Z: one step back restores everything. Try the house button and View › Home. Open the console, Enter on the log and press Home: it goes to the first entry.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **Physics reviews REQ-GUI-224 only,** the definition (R-72; applied per R-369, R-466 K14, as R-396 for TASK-M6-24); code, qa and gui review everything else (R-390).
- **Flagged readings (R-466, applied per R-369, the human may veto):** the Tilt reset keeps `z₀`, so the centre `(u, v)` is read anew in the untilted plane (K2); "the chart preset and basis" is the preset and its axes (K4); a reset switches no compass mode (K12); Home on the console's log and in a typed field keeps their meaning (K1).
- **qa's files (R-290, R-466 K15).** No qa assertion is known to assert what R-466 changes; one that asserts a section's heading row, the View menu or the canvas holds nothing more is changed by this task's qa reviewer only.
- **Early start (R-388).** It may start off TASK-M6-39's approved head once TASK-M6-38 has merged.
