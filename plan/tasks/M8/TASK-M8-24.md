# TASK-M8-24 — Run window: integration, escape and refinement settings under their contract names (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-094, REQ-GUI-102, REQ-GUI-103, REQ-GUI-104, REQ-GUI-142, REQ-GUI-047, REQ-GUI-351, REQ-GUI-352
- **Depends on:** TASK-M8-05, TASK-M6-28, TASK-M3-39
- **Needs (earlier milestones):** REQ-INT-026, REQ-INT-014, REQ-EVT-015, REQ-GUI-011, REQ-GUI-014, REQ-GUI-015, REQ-PERF-065, REQ-COL-002, REQ-INT-098
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-2.2
- **Size:** ~450 lines

## Goal
Run settings are off the Explore page and in the Run window, every field a SimConfig field under its contract name (R-68, R-107): integrator_contract Part 3's parameters (T_horizon default 50, raised or lowered, in [0.4, 200] (R-610), the window saying beside it that below one closure window (0.4) the escape arm cannot fire (*was:* "in (0, 200] meanwhile (R-575, RQ-555)", and before it "clamped to [50, 200], default 50"), dt_macro shown read-only as derived, with its rule max(10⁻³, T/65535) (R-132, R-138), N_max default 64, r_sub / gamma_sub, r_coll, r_close, eps_E / eps_L) and the occupant (stepper × regularisation, Heggie × KDK by default, Aarseth–Zare available), with no "tolerance" field; escape as tau and the escape window, with no persistence count; refinement as the quality preset selector, frame budget, MAX_REL_DEPTH and E, with Recompute and Cancel with progress. Selecting a named tier or Auto shows the Custom fields read-only; touching one switches to Custom with those values. Corpus values win over the artboard's (palette hex codes, N_max 64).

## References
- `decisions.md` § "R-575 — The horizon stays 50 by default and can be raised or lowered, not floored at 50 *(amends R-132 and the Run window's `T_horizon ∈ [50, 200]`; queues RQ-555)*"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "Run — from the top bar"
- `decisions.md` § "R-107 — Apply the RQ-67 follow-ups; GUI_DESIGN_NOTES may be conformed *(closes RQ-67)*"
- `docs/gui/principia_render_gui_spec.md` § "G13. Where the artboards are overridden"
- `decisions.md` § "R-68 — Artboard values are illustrative; corpus values win *(closes RQ-24)*"
- `docs/contracts/principia_gui_state_contract.md` § "6. Quality settings — preset selector over one struct (see `principia_quality_device_note.md`)"
- `docs/design/principia_quality_device_note.md` § "The auto → custom "show your work" flow"
- `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*"

- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-560 — The default occupant is Heggie × RK4, the combination every Heggie result was measured with; REQ-GUI-102's default and the Run window change with it"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
- `decisions.md` § "R-610 — The horizon's floor is one closure window, 0.4, below which the escape arm cannot fire, and the settings say so; the cap stays at 200 until long horizons are measured for cost: `T_horizon ∈ [0.4, 200]`, default 50 *(closes RQ-555; amends R-575's interim range)*"
- `decisions.md` § "R-615 — R-561's "Problems panel" is the status line and the Run window: a pair measured not to work shows its warning there *(closes RQ-525; reads R-561)*"

## Deliverables
- `crates/gui/src/windows/run.rs` — the field list generated from the SimConfig schema paths.
- `crates/gui/src/windows/quality.rs` — the show-your-work flow.
- Tests: `run_fields`; screenshot cases `04_windows/run`, `04_windows/run_high_tier`, `04_windows/run_custom_edit`, `01_main/no_run_fields`.

## Acceptance tests
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 04_windows` (Run window) — screenshot against 01_main.png shows none of these fields; all are in the Run window (04_windows.png) (REQ-GUI-094).
- `cargo test -p gui run_fields` — the Run window's field list equals the contract parameter names; defaults N_max = 64 and occupant = Heggie×RK4 (R-560; control: a default of Heggie×KDK, the default before R-560, fails); T_horizon defaults to 50, takes 10, 0.4 and 200, clamps 201 to 200 and refuses 0.39 and 0 with the reason "below one closure window (0.4) the escape arm cannot fire", the line beside the field saying so (R-610), and a press of ↓ (with or without Shift or Alt) or a drag that would take T below 0.4 sets T to 0.4 if T is above it and otherwise leaves T unchanged: Shift+↓ at T = 10 gives 0.4 (control: 20, 0.2 or a refusal fails), ↓ at T = 0.4 leaves 0.4 (control: a press that raises T fails), and no press goes below 0.4 (control: PR #283's interim, which reaches the Alt step, 0.2, fails) (R-575, R-610; *was (R-575's interim):* "refuses 0 with the reason, … to the Alt step, 0.2, … Shift+↓ at T = 10 gives 0.2 (control: 20, or a refusal, fails), ↓ at T = 0.2 leaves 0.2 …, and no press reaches 0 (control: the mock's clamp to lo = 0, which reaches 0, fails)"; control: the clamp to [50, 200] before R-575 turns 10 into 50 and fails); no field named tolerance; dt_macro is read-only, shows its rule and tracks T (R-138) (REQ-GUI-102; gui review 5627122883 of PR #283, G8; gui review 5627280803, N3). *Was:* "`cargo test -p gui run_fields` — the Run window's field list equals the contract parameter names; defaults N_max = 64 and occupant = Heggie×RK4 (R-560; control: a default of Heggie×KDK, the default before R-560, fails; *was:* "occupant = Heggie×KDK"); T_horizon defaults to 50, takes 10 and 200, and refuses 0 and 201 with the reason (R-132, R-575; control: the clamp to [50, 200] before R-575 turns 10 into 50 and fails; *was:* "T_horizon is clamped to [50, 200] with default 50 (R-132)"); no field named tolerance; dt_macro is read-only, shows its rule and tracks T (R-138) (REQ-GUI-102)." (fix round 1 of PR #283).
- `cargo xtask screenshot 04_windows` (escape section) — screenshot against 04_windows.png's Run window shows tau and window, no 'persistence' field (REQ-GUI-103).
- `cargo xtask screenshot 04_windows` and `cargo test -p gui recompute_cancel` — screenshot against 04_windows.png; Cancel during a recompute stops it and the progress resets (REQ-GUI-104).
- `cargo xtask screenshot 04_windows` (contract names; N_max reads 64; palette hex equals colour_composition §1.4) — Run window screenshot lists contract parameter names; substep cap default reads 64; palette colours equal corpus hex codes (REQ-GUI-142).
- `cargo xtask screenshot 04_windows` (High tier; edit E → Custom) — select High: fields show 16² samples, depth 6, E = 3, FTLE on greyed; edit E → mode shows Custom with the other values kept (REQ-GUI-047).
- `cargo test -p gui occupant_pair_warning` — selecting Heggie × KDK shows the integrator contract's Part 2b warning text beside the selectors and the run proceeds; selecting Heggie × RK4 shows none; every pair of the production steppers is reachable, and the Euler pairs only in the debug selection (REQ-INT-015); controls: a selector that disables a measured-bad pair fails, a warning shown for a not-measured pair fails, and a production selector offering an Euler pair fails (REQ-GUI-351; R-561; qa review 5607548278 of PR #277, Q6, applied per R-369).
- `cargo test -p gui status_pair_warning` — with Heggie × KDK bound the status line shows the pair's warning, and with Heggie × RK4 it shows none; controls: a binding change that leaves the old warning shown fails, and the warning added to the stain editor's Problems pane fails (REQ-GUI-352; R-561, R-615). *Was:* "`cargo test -p gui problems_pair_warning` — with Heggie × KDK bound the Problems panel RQ-525 names lists the pair's warning, and with Heggie × RK4 it lists none; control: a binding change that leaves the old warning listed fails (REQ-GUI-352; R-561; waits on RQ-525, the rest of the task does not)."

## Notes
- R-138: dt_macro is derived, not editable; the Run window shows it read-only with its rule max(1e-3, T/65535).
- The escape window's 0.4 is marked provisional in §G5; the field exposes whatever SimConfig holds.
- R-390: TASK-M6-28 builds the Run window's frame on the mock engine; this task depends on it and binds every field to its SimConfig field on the real engine, keeping every requirement it closes.
- R-575 (21 Oct 2026): the horizon is no longer floored at 50. The mock's `T_RANGE` (`crates/gui/src/windows/run.rs`, TASK-M6-28, merged with `(50.0, 200.0)`) changes here with the field: [0.4, 200] (R-610, RQ-555), a typed value below 0.4 refused with the reason "below one closure window (0.4) the escape arm cannot fire" (applied per R-369). *Was:* "(0, 200], RQ-555's options (a) and (i) applied meanwhile, a value at or below 0 refused with its reason."
- R-610 (25 Oct 2026, RQ-555): the floor is one escape window (0.4 provisional, R-29; the floor follows the window's value); ↓ and the drag stop at 0.4, replacing the interim below (applied per R-369). R-615 (RQ-525 (c)): R-561's "Problems panel" is the status line and the Run window, so a measured-bad pair's warning shows in the status line too (REQ-GUI-352).
- Fix round 1 of PR #283 (gui review 5627122883, G8; applied per R-369): above 200 the field clamps to 200, as before R-575, and only a value at or below 0 is refused; a press of ↓ (with or without Shift or Alt) or a drag that would take T to 0 or below sets T to the Alt step, 0.2, if T is above it, and otherwise leaves T unchanged; ↓ never raises T (fix round 3, gui review 5627280803, N3). The mock's clamp (`crates/gui/src/windows/run.rs:315`, `.clamp(lo, hi)` with `lo = T_RANGE.0`) and its test `mock_windows_open_run_t_horizon_steps_in_its_range` ("At the bottom of the range ↓ stays" at `lo`) change here.
