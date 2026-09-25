# TASK-M8-24 — Run window: integration, escape and refinement settings under their contract names (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-GUI-094, REQ-GUI-102, REQ-GUI-103, REQ-GUI-104, REQ-GUI-142, REQ-GUI-047
- **Depends on:** TASK-M8-05
- **Needs (earlier milestones):** REQ-INT-026, REQ-INT-014, REQ-EVT-015, REQ-GUI-011, REQ-GUI-014, REQ-GUI-015, REQ-PERF-065, REQ-COL-002
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-2.2
- **Size:** ~450 lines

## Goal
Run settings are off the Explore page and in the Run window, every field a SimConfig field under its contract name (R-68, R-107): integrator_contract Part 3's parameters (T_horizon clamped to [50, 200], default 50, dt_macro = max(10⁻³, T/65535) (R-132), N_max default 64, r_sub / gamma_sub, r_coll, r_close, eps_E / eps_L) and the occupant (stepper × regularisation, Heggie × KDK by default, Aarseth–Zare available), with no "tolerance" field; escape as tau and the escape window, with no persistence count; refinement as the quality preset selector, frame budget, MAX_REL_DEPTH and E, with Recompute and Cancel with progress. Selecting a named tier or Auto shows the Custom fields read-only; touching one switches to Custom with those values. Corpus values win over the artboard's (palette hex codes, N_max 64).

## References
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
## Deliverables
- `crates/gui/src/windows/run.rs` — the field list generated from the SimConfig schema paths.
- `crates/gui/src/windows/quality.rs` — the show-your-work flow.
- Tests: `run_fields`; screenshot cases `04_windows/run`, `04_windows/run_high_tier`, `04_windows/run_custom_edit`, `01_main/no_run_fields`.

## Acceptance tests
- `cargo xtask screenshot 01_main` and `cargo xtask screenshot 04_windows` (Run window) — screenshot against 01_main.png shows none of these fields; all are in the Run window (04_windows.png) (REQ-GUI-094).
- `cargo test -p gui run_fields` — the Run window's field list equals the contract parameter names; defaults N_max = 64 and occupant = Heggie×KDK; T_horizon is clamped to [50, 200] with default 50 (R-132); no field named tolerance (REQ-GUI-102).
- `cargo xtask screenshot 04_windows` (escape section) — screenshot against 04_windows.png's Run window shows tau and window, no 'persistence' field (REQ-GUI-103).
- `cargo xtask screenshot 04_windows` and `cargo test -p gui recompute_cancel` — screenshot against 04_windows.png; Cancel during a recompute stops it and the progress resets (REQ-GUI-104).
- `cargo xtask screenshot 04_windows` (contract names; N_max reads 64; palette hex equals colour_composition §1.4) — Run window screenshot lists contract parameter names; substep cap default reads 64; palette colours equal corpus hex codes (REQ-GUI-142).
- `cargo xtask screenshot 04_windows` (High tier; edit E → Custom) — select High: fields show 16² samples, depth 6, E = 3, FTLE on greyed; edit E → mode shows Custom with the other values kept (REQ-GUI-047).

## Notes
- Gap: R-132 derives dt_macro from T; whether the Run window keeps dt_macro as an editable field is not settled by R-132.
- The escape window's 0.4 is marked provisional in §G5; the field exposes whatever SimConfig holds.
