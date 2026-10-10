# TASK-M6-31 — Compass modes and keyboard navigation/interaction modes in the mock (R-409)

- **Milestone:** M6
- **Closes:** REQ-GUI-181, REQ-GUI-182, REQ-GUI-183, REQ-GUI-184
- **Depends on:** TASK-M6-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
On the mock engine, the compass and the keyboard work as the human asked after trying TASK-M6-26 (R-409). The compass
has no orbit: it is a fixed orthographic view at its default angle. It has a Tilt and a Slice button beside it, Tilt the
default; a click on one switches the mode, touching a slice or tilt slider still switches it to match, and the buttons
always show the current mode. In tilt mode a drag on the compass tilts the plane; in slice mode a drag anywhere on it
moves the slice plane along its normal, the tilt fixed. The keyboard has two modes in every scope: navigation mode, with
an orange focus ring, where the arrows move between siblings; and interaction mode, with a blue ring, where the focused
element takes the arrows, while Tab, Ctrl+Z, `?` and the shortcuts keep their meaning (R-409 G1). Every landing is in
navigation mode: Enter into a scope lands on its first element, and a Tab on the big scope itself, the whole scope
highlighted; Enter on an element starts interaction, Enter on a button acts as its click, Esc in interaction mode
returns to navigation mode on the same element, and Esc in navigation mode goes up one scope. The arrows need Enter: in
navigation mode they always move between siblings, and a scope's arrow actions act only in interaction mode, while its
letter and Space shortcuts act in either mode. In navigation mode the compass is one element, beside its Tilt and Slice
buttons; Enter on it starts interaction, where the arrows tilt or move the slice by its mode, Shift ×10 and Alt ×0.1 as
everywhere (R-409's follow-up answers).

## References
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-404 — #174's look choices and key-repeat timings are confirmed as built: the focus ring, the breadcrumb, the `?` overlay, the 500 ms delay and 40 ms interval, and the base steps"
- `decisions.md` § "R-405 — The footer and the console join the keyboard's scope tree, as the top bar does: the footer is big scope 8, and Enter on it opens the console"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "07 Keyboard — design note, not a screen"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"

## Deliverables
- `crates/gui/src/explore/compass.rs` — the orbit removed: no `Drag::Orbit`, no `Compass::orbit`, no yaw and pitch
  state, no Shift+arrows orbit (today at :36, :54, :83–86, :152–167 and :215–231); the view fixed at the code's default
  angle (`ORBIT`, kept as the fixed view's angle, renamed if the implementer chooses; a look choice under R-390, R-409
  point 1). The mode labels (today at :245–256) become two buttons, Tilt and Slice, each a click target that switches
  the mode and shows whether it is the current one; Tilt the default (`Compass::default`, today Slice). A drag in tilt
  mode tilts, as today's plane drag does, wherever it starts on the compass. A drag in slice mode moves `z₀` along the
  unit normal of the plane as drawn (perpendicular to both tilted in-plane directions in the compass's frame), one
  `SetField` on `z₀` coalesced per drag (R-96), the basis unchanged; locked, it is an excursion along that line through
  the anchor, as the slice step's edit is (R-409 A3). The hint under the buttons is reworded for the two modes.
- `crates/gui/src/explore/manifold_view/{mod,slice_tilt,centre}.rs` — touching a slice or tilt slider still sets the
  compass's mode (`Touch`), so the buttons follow it.
- `crates/gui/src/keyboard/scopes.rs` — the two modes replacing `browsing` (today at :324–419): every Tab landing and
  every Enter into a scope lands in navigation mode on the first element; in navigation mode the arrows move between
  siblings, and on a big scope itself between the big scopes in Tab order (R-409 G2); Enter on an adjustable element
  (`Arrows::Adjust`, the compass) starts interaction mode, on the Figure, which has nothing inside, starts interaction
  mode, on a read-only scope with nothing inside (the Legend) does nothing (R-409 A2, G3), and on an `activates` scope
  acts as its click and stays in navigation mode; in interaction mode the arrows adjust the element, and Tab and
  Shift+Tab, Ctrl+Z, `?` and the shortcuts keep their meaning, except while a text field is being typed in (R-409 G5),
  Tab or Shift+Tab leaving for the next or previous big scope in navigation mode, or, inside a window, for the window's
  next or previous section, staying in the window (R-409 G1, G6); Esc or Enter in interaction mode returns to navigation
  mode on the same element, and Esc in navigation mode pops one level. A text field lands in navigation mode; Enter on
  it starts typing (interaction mode), in which every key goes to it, Ctrl+Z undoing the text edit and `?` typing a
  character, but Esc and Enter (back to navigation mode, text kept) and Tab (moving on) (R-409 G4, G5); the console's
  text filter is built on this by TASK-M6-28. The mode is held with the focus, in `ViewUI`'s keyboard focus scope (R-409
  A5); a contract field added re-runs the conformance suite on both engines.
- `crates/gui/src/keyboard/{keymap,mod}.rs` — the mode carried through the key handling, so a screen's arrow actions
  (the Figure's pan, the compass's tilt and slice) act in interaction mode only, and its letter and Space shortcuts
  (the Figure's + / −, Space, L and K) whenever the focus is in its scope, in either mode (R-409 F1). A Tab lands on
  the big scope itself, in navigation mode (R-409 F4).
- The Compass scope (big scope 5) holding three siblings in navigation mode, in this order: the compass, then Tilt, then
  Slice (R-409 A2, F4); from the compass → reaches Tilt and → again Slice, and ← goes back. Enter on Tilt or Slice sets
  the mode; Enter on the compass starts interaction, in which the arrows tilt (`StepKind::Tilt`, 1°, R-404) in tilt mode
  and move the slice along the normal (REQ-GUI-184's step) in slice mode, Shift ×10 and Alt ×0.1 in either mode, as
  everywhere (R-409 F3).
- `crates/gui/src/keyboard/scopes.rs`, `StepKind` — `Orbit` removed (R-404's row retired by R-409); a slice-along-the-
  normal kind added with its proposed base step, Shift ×10 and Alt ×0.1 applying to it as to every kind; no separate
  fine step (R-409 F3).
- `crates/gui/src/explore/breadcrumb.rs` — the ring drawn in the orange in navigation mode and in `hyperlink_color`
  (R-404) in interaction mode (today one colour, :22–52); the orange a look choice under R-390, as an egui colour or a
  theme colour, recorded in the PR; the breadcrumb's colour per R-409 A6, recorded in the PR.
- The `?` overlay's rows reworded: for the compass, no orbit, Enter to interact, Shift ×10 and Alt ×0.1; the
  arrows needing Enter and the letter and Space shortcuts acting in either mode (R-409 F1, F3).
- Calibration proposal: the base step of the compass's slice-mode arrows (REQ-GUI-184), with its reasoning, used
  provisionally until the human confirms it at the M8 gate (R-182).
- Tests `mock_compass_modes`, `mock_keyboard_modes`, `mock_compass_keys`, and `mock_keyboard` and `mock_manifold_view`
  updated for the modes; screenshot cases `07_keyboard/mock_focus_navigation`, `07_keyboard/mock_focus_interaction`, and
  `01_main/mock_compass_tilt`, `01_main/mock_compass_slice` re-captured with the buttons.

## Acceptance tests
- `cargo test -p gui mock_compass_modes` and `cargo xtask screenshot 01_main` (mock_compass_tilt, mock_compass_slice) — the compass opens in Tilt; a click on Slice shows Slice and a click on Tilt shows Tilt; touching a slice slider shows Slice and touching τ₁ shows Tilt, the buttons following; in tilt mode a drag on the plane and a drag off it each emit one SetField on the basis and none on z₀; in slice mode a drag anywhere on the compass emits one SetField on z₀ whose change is along the drawn plane's normal (its components along the plane's in-plane directions zero within 1e-12), the basis unchanged; locked, the slice-mode drag moves z₀ along the normal through the anchor; a drag anywhere and every key leave the compass's view angle unchanged; controls: a slice-mode drag that tilts, a drag that orbits, a slice move along the depth axis on a tilted plane, and a button that does not follow a slider each fail; the screenshots against 01_main.png, read with R-409 (render_gui_spec §G13) (REQ-GUI-181).
- `cargo test -p gui mock_keyboard_modes` and `cargo xtask screenshot 07_keyboard` (mock_focus_navigation, mock_focus_interaction) — Tab to Manifold view lands on Manifold view itself, in navigation mode, → there moves to the Figure and ← back to Manifold view, and Enter lands on Chart in navigation mode; Enter on Centre z₀ lands on its first slider in navigation mode, ↓ moves to the second slider and changes no value; Enter on it starts interaction mode and ↑ raises it by its base step; ↓ in interaction mode adjusts it and does not move the focus; Esc returns to navigation mode on the same slider, and a second Esc goes up to Centre z₀; Enter on the slider again and then Enter returns to navigation mode on it; in interaction mode on the slider, Tab lands on the Figure in navigation mode, and Ctrl+Z and ? act as in navigation mode; Tab lands on the Figure in navigation mode, where the arrows move to the next big scope and emit no SetField, + and − zoom and K locks, and Enter starts interaction mode, where the arrows pan and + / − and K still act; Enter on the Legend changes nothing; Tab lands on the Compass scope itself in navigation mode and Enter goes in onto the compass, still in navigation mode (R-409 A2, F4); Enter on a button (Tilt, a menu entry) acts as its click and stays in navigation mode; the ring is the orange in navigation mode and `hyperlink_color` in interaction mode; controls: an Enter into a scope that lands in interaction mode, a Tab that lands on a big scope's first element, an Esc in interaction mode that goes up a scope, an Enter in interaction mode that stays in it, a Tab in interaction mode captured by the element, Enter on the Figure landing in navigation mode, Enter on the Legend starting interaction mode, arrows on a big scope that do nothing, a Figure shortcut (+, −, K) that acts in one mode only, a ring of one colour in both modes and the pre-R-409 `browsing` model (arrows adjusting a value reached by Enter on its section) each fail; the two captures differ only in the ring's colour (and the breadcrumb's, per R-409 A6) (REQ-GUI-182).
- `cargo test -p gui mock_compass_keys` — Tab lands on the Compass scope and Enter goes in onto the compass, in navigation mode; from the compass → moves to Tilt and → again to Slice, and ← from Slice moves back to Tilt and ← again to the compass; Enter on Slice shows Slice and Enter on Tilt shows Tilt, staying in navigation mode; Enter on the compass starts interaction mode; in tilt mode ← → tilt τ₁ and ↑ ↓ τ₂ by the confirmed Tilt step (R-404), one SetField on the basis each; in slice mode each arrow emits one SetField on z₀ along the plane's normal by REQ-GUI-184's step; in each mode Shift+arrows give 10 times and Alt+arrows 0.1 times the arrow's step, and no key turns the view; Esc returns to navigation mode on the compass; controls: Shift+arrows that orbit, Shift+arrows that give a step smaller than the arrow's, a sibling order other than compass, Tilt, Slice (→ from the compass reaching Slice), an arrow on the compass in navigation mode that tilts, and a slice-mode arrow that tilts each fail (REQ-GUI-183).
- Review checklist (gui reviewer) of the slice-mode base-step proposal — the proposal gives the base step (absolute, or relative to the slice step's range or the view) with its reasoning; a reviewer checks it; the human confirms it at the M8 gate and it is recorded in `decisions.md` (REQ-GUI-184).
- `cargo xtask screenshot 01_main` (mock_compass_slice, mock_compass_tilt) and `cargo xtask screenshot 08_lock` (mock_locked) — screenshots against 01_main.png after touching a slice slider, after touching a tilt and after clicking each of the Tilt and Slice buttons, and against 08_lock.png when locked, the buttons and the fixed view read with R-409 (REQ-GUI-091, closed by TASK-M6-26, re-checked here as R-409 words it).
- `cargo test -p gui mock_manifold_view` and `cargo xtask screenshot 01_main` (mock_manifold_view) — every Manifold view control still emits a SetField on z₀ or the basis when used from the keyboard in interaction mode; after lock a slider reads anchor plus offset; the compass switches by its buttons and by the slider touched, with no orbit (REQ-GUI-170, closed by TASK-M6-26, re-checked here as R-409 words it).
- `cargo test -p gui mock_keyboard` and `cargo xtask screenshot 07_keyboard` (mock_focus_before, mock_focus_after, mock_shortcuts, mock_focus_manifold_view) — Tab, Shift+Tab, Enter, Esc, the arrows and the Shift and Alt steps move focus and values as §G3 gives, in each mode; `?` opens the shortcuts over everything and Esc closes it; screenshots before and after moving focus differ only in the ring and the breadcrumb, and before and after Enter on a value only in the ring's colour (and the breadcrumb's, per R-409 A6), against 07_keyboard.png read with R-409 (REQ-GUI-169 and REQ-GUI-098, closed by TASK-M6-25, re-checked here as R-409 words them).
- `cargo run -p gui --features mock`, `cargo test -p engine conformance` and `cargo test -p gui conformance` — the app launches on the mock and the conformance suite passes on both engines (REQ-GUI-165, REQ-GUI-166, closed by TASK-M6-24, re-run here).

## Notes
- **What to try.** The PR description has a "What to try" section: drag on the compass in Tilt, click Slice and drag
  again, touch a slice slider and a tilt and watch the buttons; Tab to Manifold view, Enter, ↓ through the sections,
  Enter on Centre z₀, ↓ between its sliders, Enter on one and ↑ ↓, Esc, Esc; Tab to the Compass, Enter (on the
  compass), → to Tilt, → to Slice, Enter, ← to Tilt, ← back to the compass, Enter, the arrows, Shift+arrows and
  Alt+arrows, Esc; on the Figure, + / − and K before and after Enter, and the arrows only after it; Tab out of
  interaction mode; on the Legend, Enter does nothing; watch the ring turn orange and blue; on
  `cargo run -p gui --features mock`.
- **Why a new task.** TASK-M6-26 (merged) built the compass with its orbit and TASK-M6-25 (merged) the `browsing`
  model; R-409 changes both. This task follows TASK-M6-26, and TASK-M6-27 depends on it, so Time, the Legend, the
  Trajectory panel, the windows, the console and the Stain editor are built on the two modes (R-409 A7). It is a GUI-
  track task (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1 to M5.
- **Look choices (R-390), recorded in the PR as "applied per R-369":** the navigation mode's orange; the buttons' look
  and labels; the hint's wording; the fixed view's angle (the code's default, R-409
  point 1); and the breadcrumb's colour (R-409 A6). **They hold (R-416):** like TASK-M6-26's, they are neither
  confirmed nor changed until the human has tried this task's mock; the PR lists them for that.
- **Calibration (R-71) proposed here:** REQ-GUI-184, the slice-mode arrows' base step. It is used provisionally
  (R-182) and confirmed by the human at the M8 gate; an unconfirmed one blocks that gate. R-404's other base steps stand;
  its `Orbit` row is retired.
- **qa's files (R-290, forced by R-409; R-409 A9).** `crates/gui/tests/qa_TASK-M6-25.rs` (e.g.
  `qa_mock_keyboard_arrows_adjust_a_focused_value_shift_x10_alt_x0_1`, whose value adjusts with no Enter on it and
  whose kinds list `StepKind::Orbit`), `crates/gui/tests/qa_TASK-M6-26.rs` (e.g.
  `qa_mock_compass_arrows_tilt_and_shift_arrows_orbit`, `qa_mock_compass_switches_mode_by_the_slider_touched` if the
  labels change, and its keyboard flows that adjust with no Enter) and `crates/gui/tests/qa_TASK-M6-25_recheck.rs`,
  where it relies on the same, assert behaviour R-409 changes. Every earlier commit of each is qa's, so this task's qa
  reviewer changes those assertions in the task's qa commit, and only those; the implementer never edits them, and
  until that commit those tests fail on the implementer's head, as the PR says. The PR lists each `M` with its reason,
  and the code reviewer confirms that no assertion was weakened but the ones R-409 changes.
- **R-409's follow-up answers (10 Oct 2026).** The human settled four of the first port's open conflicts: the arrows
  need Enter, and letter and Space shortcuts act in either mode (F1); the console's entry list scrolls only after Enter
  (F2, TASK-M6-28's); on the compass Shift is ×10 and Alt ×0.1, as everywhere, so there is no separate fine step and
  no look choice for one (F3; REQ-GUI-184 stays a calibration); and a Tab lands on the big scope itself (F4). Whether
  the slice-mode drag along the drawn normal changes any result is the physics reviewer's, on PR #186 (F5).
- **The console and the footer** are TASK-M6-28's (REQ-GUI-178, R-405); they land in navigation mode under this task's
  model, which TASK-M6-28 builds on through TASK-M6-27.
- **The real engine.** TASK-M8-07 re-runs REQ-GUI-091 and TASK-M8-13 REQ-GUI-096, REQ-GUI-097 and REQ-GUI-098 there,
  as R-409 words them.
- Sources and silences as TASK-M6-24's Notes give them (R-390).
