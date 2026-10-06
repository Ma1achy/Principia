# TASK-M7-20 — Colour-vision simulation: real Viénot and Brettel

- **Milestone:** M7
- **Closes:** REQ-COL-042, REQ-COL-045, REQ-COL-061
- **Depends on:** TASK-M7-02
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
Colour-vision simulation as a display-stage function on linear sRGB: real Viénot simulation for protan and deutan and real Brettel simulation for tritan, through LMS space, with matrices and golden values from a published reference implementation named with its version in dd_colouring §3.8 (R-78); achromatopsia multiplies the linear triplet by §3.8's M_achrom. The modes offered in the Display window are off, deuteranopia, protanopia, tritanopia and achromatopsia (R-123).

The reference implementation is pinned by R-383: DaltonLens-Python at commit `3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d`, its Viénot and Brettel simulators on its Smith–Pokorny LMS model, implementing Viénot, Brettel & Mollon 1999 and Brettel, Viénot & Mollon 1997. The goldens in `fixtures/cvd/` are generated from it. With the simulation built, the task also proposes and checks REQ-COL-061 (R-386): the hatch's colours stay distinguishable from the palette under each simulation, confirmed by the human at the M1 gate.

## References
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)"
- `decisions.md` § "R-78 — Real Viénot and Brettel colour-vision simulation *(closes RQ-29)*"
- `docs/gui/principia_render_gui_spec.md` § "Display — the last stages"
- `decisions.md` § "R-123 — Achromatopsia is a fifth Display mode *(closes RQ-91)*"
- `decisions.md` § "R-383 — The colour-vision reference is DaltonLens-Python at commit `3cba5e6`: its Viénot 1999 and Brettel 1997 simulators, on its Smith–Pokorny LMS model, generate TASK-M7-20's goldens *(closes RQ-209)*"
- `decisions.md` § "R-386 — At the M1 gate, the hatch colours are checked distinguishable from the palette under each colour-vision simulation, so TASK-M7-20 merges before the M1 gate"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/shaders/wgsl/compositor/cvd.wgsl` (a fixed display-stage pass, not scanned) and the Rust mirror `crates/render/src/display/cvd.rs`.
- `fixtures/cvd/` — the reference implementation's golden values.
- `docs/design/principia_dd_colouring.md` §3.8 — the reference implementation named with its version (R-78 says it is named when this task lands), with the "Removed lines" note.
- **The pinned source (R-383).** Fetch read-only into the task's scratch directory, and install nothing: `https://github.com/DaltonLens/DaltonLens-Python` at commit `3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d` (version 0.1.6 in its `setup.cfg`; MIT). Transcribe from `daltonlens/convert.py` (`LMSModel_sRGB_SmithPokorny75`, `XYZJuddVos_from_linearRGB_BT709`, `LMS_from_XYZJuddVos_Smith_Pokorny_1975`, `linearRGB_from_sRGB`, `sRGB_from_linearRGB`) and `daltonlens/simulate.py` (`plane_projection_matrix`, `Simulator_Vienot1999` for protan and deutan, `Simulator_Brettel1997` for tritan with its defaults `use_white_as_neutral=True` and `use_vischeck_anchors=False`); R-383 cites each by line. The methods' sources: Viénot, Brettel & Mollon 1999, "Digital video colourmaps for checking the legibility of displays by dichromats", and Brettel, Viénot & Mollon 1997, "Computerized simulation of color appearance for dichromats".
- `fixtures/cvd/` — generated from the pinned implementation in linear sRGB, through its simulators' linear-RGB path (`_simulate_dichromacy_linear_rgb`) at full dichromacy, never its 8-bit path (R-383); the generating script and the commit recorded beside the values.
- The R-71 proposal for REQ-COL-061: the criterion (each hatch colour's OKLab distance to its nearest palette entry, both simulated), its threshold, and how the two-stripe pattern counts where one stripe colour falls within it, as under achromatopsia against the grey ramp, with the measured distances per mode (R-386).

## Acceptance tests
- `cargo test -p render cvd_simulation` — dd_colouring unit test 10: protan, deutan and tritan match the named reference implementation's golden values; achrom yields R = G = B exactly; applying any simulation pre-linearisation produces a detectable difference (REQ-COL-042).
- `cargo test -p render cvd_modes` — each of off / deuteranopia / protanopia / tritanopia / achromatopsia transforms a test colour set by its Viénot (protan, deutan) or Brettel (tritan) simulation, matching the reference golden values, and achromatopsia by M_achrom (R = G = B); off is identity (REQ-COL-045).
- `cargo test -p render cvd_hatch_distinct` — under each of deuteranopia, protanopia, tritanopia and achromatopsia, the hatch's violet `#9B00FF` and cyan `#48FFFF` and every palette entry REQ-COL-055 lists, all simulated, meet the calibrated criterion; the test prints each hatch colour's least OKLab distance per mode (REQ-COL-061).
- Review (code, qa): the REQ-COL-061 proposal gives the criterion and threshold with the measured distances per mode; confirmed by the human at the M1 gate, beside REQ-COL-055 (REQ-COL-061).

## Notes
- Gap: R-78 leaves the reference implementation (and version) to be named when the task lands; no requirement carries that choice to the human, so it is flagged in the milestone report.
- RQ-209 ruled: R-383 names the reference implementation and its version, DaltonLens-Python at commit `3cba5e6a7c8f0e8199c8f83f1afb58eb6dab7a3d`, closing the gap above; the goldens are generated from it.
- R-386: the M1 gate does not pass until REQ-COL-061 is green on `main`, so this task merges before the M1 gate (`plan/MILESTONES.md` § "M1 — The synthetic payload and the eyes"). REQ-COL-061 sits in M7 because this task closes it.
- Calibration (R-71): REQ-COL-061.
- RQ-91 ruled: R-123 — achromatopsia is offered in the Display window as a fifth mode (render_gui_spec conformed in step 7).
