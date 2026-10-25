# TASK-M8-32 — Export & share window: image with pxpack, state snapshot and share link, present mode (04_windows.png)

- **Milestone:** M8
- **Closes:** REQ-TOOL-102, REQ-TOOL-103, REQ-GUI-100, REQ-TOOL-146, REQ-TOOL-167, REQ-TOOL-168, REQ-TOOL-169, REQ-GUI-353
- **Depends on:** TASK-M8-30, TASK-M8-31, TASK-M8-05, TASK-M7-32, TASK-M6-28, TASK-M2-34, TASK-M6-47, TASK-M8-46, TASK-M6-48, TASK-M2-37, TASK-M2-40, TASK-M7-45, TASK-M3-39
- **Needs (earlier milestones):** REQ-TOOL-059, REQ-TOOL-065, REQ-TOOL-067, REQ-TOOL-070, REQ-TOOL-071, REQ-TOOL-072, REQ-INT-098
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The Export & share window offers image export (size in multiples of the view, format, embed the view as pxpack, optionally the stain's WGSL) so that opening the picture recreates the view exactly; state export (copy snapshot JSON, save snapshot, load, and a `principia://view?…` share link), each recreating the view; and present mode, which hides all chrome until Esc, its figure filling the window as with F3 (R-406), past the chart's `[0,1]²` by each axis's extension type with the hatch fallback (R-407).

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `decisions.md` § "R-309 — `SimConfig` and `RenderState` have one canonical serialisation; the profiler header's `config` uses it *(closes RQ-181)*"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "04 Windows"
- `docs/gui/principia_render_gui_spec.md` § "Export & share"
- `docs/gui/principia_render_gui_spec.md` § "G9. Import picture · saved views · record a sweep (`09_importrecord.png`)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 6 — Sharing: the spec is the object, the video is its shadow"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"

- `docs/contracts/principia_integrator_contract.md` § "Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)"
- `decisions.md` § "R-561 — Every stepper × regularisation pair is allowed, regularisation off included; a pair measured not to work carries a warning beside the selector, in the Problems panel and in each export's settings, and is still defined exactly so its results reproduce; a table marks each pair measured good, measured bad or not measured"
## Deliverables
- `crates/gui/src/windows/export_share.rs` — calls the exporter (TASK-M8-30) and the M7 embedding.
- `crates/gui/src/present.rs`.
- Tests: `png_pxpack_roundtrip`, `state_share_roundtrip`; screenshot cases `04_windows/export`, `01_main/present`, `01_main/present_sphere`, `01_main/present_esc`, `01_main/present_fit`.
- Present mode's fill on the real engine (R-406, R-407, R-467, R-468): the figure given the window as F3 off gives it, in the mode View › F3 fills by sets: in Extend TASK-M8-05's extension per axis type, a forbidden pixel drawn as the canvas background (TASK-M2-34's render); in Fit TASK-M8-46's scaled square, the rest the canvas background.
  *Was: "using TASK-M8-05's extension per axis type and hatch fallback". Stale, replaced by R-467 and R-468.*

## Acceptance tests
- `cargo test -p gui png_pxpack_roundtrip` — export a PNG with pxpack, import it: the restored SimConfig + RenderState equal the exported ones field for field (REQ-TOOL-102).
- `cargo test -p gui state_share_roundtrip` — round-trip each of JSON copy, saved file and share link: the state is equal after load (REQ-TOOL-103).
- `cargo test -p gui state_canonical_text` — for one state, the `SimConfig` and `RenderState` text in the snapshot JSON, in the decoded share link and in the extracted pxpack record is byte-equal to the canonical serialisation (gui_state_contract §2) (REQ-TOOL-146, R-309).
- `cargo xtask screenshot 01_main` (present, present_sphere, present_fit; Esc restores the layout) and Review checklist (physics reviewer) on the fill — as REQ-GUI-100's verify: screenshot in present mode shows only the figure, filling the window; in Extend, over the shown figure's rect it is pixel-identical to the shown layout's capture (with a stain that reads no screen-lane field), and the window outside it is covered with the extension R-407 gives (on the latent chart its formula continued; on the shape sphere θ wrapped and the poles crossed), a forbidden pixel the canvas background (R-467) and every other pixel never the clear colour, never the shown view stretched; in Fit (`present_fit`) the capture equals F3's Fit capture (REQ-GUI-230); Esc restores 01_main.png's layout; controls: a present mode left in the shown rect fails the coverage check, and one that hatches a forbidden pixel fails; the physics reviewer checks the fill against R-407 (REQ-GUI-100).
  *Was: "`cargo xtask screenshot 01_main` (present, present_sphere; Esc restores the layout) and Review checklist (physics reviewer) on the fill — screenshot in present mode shows only the figure, filling the window: over the shown figure's rect it is pixel-identical to the shown layout's capture (with a stain that reads no screen-lane field), and the window outside it is covered with the extension R-407 gives (on the latent chart its formula continued; on the shape sphere θ wrapped and the poles crossed), never the clear colour or the shown view stretched; Esc restores 01_main.png's layout; control: a present mode left in the shown rect fails the coverage check; the physics reviewer checks the fill against R-407 (REQ-GUI-100).". Stale, replaced by R-467 and R-468.*
- `cargo test -p gui export_tilt_map` — REQ-TOOL-167's verify.
- `cargo test -p gui export_f3_fill` — REQ-TOOL-168's verify.
- `cargo test -p gui export_chart_params` — REQ-TOOL-169's verify.
- `cargo test -p gui export_pair_warning` — an image, a snapshot and a share link exported under Heggie × KDK carry the occupant and the warning text in their sidecar, and importing each shows the warning; a recording made under Heggie × KDK carries the occupant and the warning in its spec object (its `SimConfig`, TASK-M8-31) and a receiver shows it; under Heggie × RK4 they carry the occupant and no warning; control: an export whose sidecar drops the warning under a measured-bad pair fails (REQ-GUI-353; R-561; qa review 5607548278 of PR #277, Q7, applied per R-369).

## Notes
- R-390: TASK-M6-28 builds the Export & share window's frame on the mock engine; this task depends on it and wires it to the real export, keeping every requirement it closes.
- R-406 (10 Oct 2026), applied per R-369: present mode hides the chrome as F3 does, so its figure fills the window the
  same way, each point of the field where the shown layout puts it (REQ-GUI-100's note; flagged in R-406, since the
  human spoke of F3). On the real engine the area past the chart's `[0,1]²` follows RQ-262's ruling, as TASK-M8-05's
  REQ-GUI-180 does.
- R-407 (10 Oct 2026) rules RQ-262: present mode's fill on the real engine is built here, using TASK-M8-05's extension
  (this task already depends on TASK-M8-05). The acceptance line for REQ-GUI-100 checks that Present fills the window,
  and the physics reviewer joins for that line (applied per R-369, R-407, A2). Nothing here computes an area statistic
  over the extended window; R-408 (RQ-263 ruled) counts each system once, through the axis types (REQ-CHART-056,
  TASK-M8-05).
- R-467 and R-468 (20 Oct 2026): present mode's fill is the F3-off figure, so it follows R-468's F3 fill setting
  (View › F3 fills by: Extend / Fit; Extend, the default, is TASK-M8-05's extension; Fit draws the chart's own square,
  scaled to fit, the rest the canvas background), and where it falls back past the formula's validity the pixel
  draws the canvas background, not the hatch (R-467, which amends R-407 clause 2 as it applies to display). "Hatch
  fallback" in the Deliverables line reads "background fallback"; the debug view, when on, shows the hatch in present
  mode too (TASK-M8-46, REQ-GUI-227 and REQ-GUI-230).
- R-467 D13 (code review 5585240188 of PR #253, F4; applied per R-369): this task depends on TASK-M2-34 (the
  background), TASK-M6-47 (the mode's row) and TASK-M8-46 (Fit on the real engine); the note above's "background
  fallback" reading is now the Deliverables and acceptance lines.
- **R-471, R-472, R-473, R-474, R-475 (20 Oct 2026):** the sidecar (the pxpack, the snapshot JSON, the share link) carries `tilt_map=sheet|lean`, `f3_fill=extend|fit`, each energy-type axis's mapping, a curved chart's tilt and a saved chart's definition; present mode follows F3's fill mode (R-472 X2c).
- R-484 RT2, RT3 (20 Oct 2026): the window's tier report is TASK-M8-53's (REQ-GUI-263); the sidecar carries the links through `SimConfig` (R-483).
- **R-521 H2, H5 (20 Oct 2026; code review F1 of PR #265, applied per R-369):** snapshot JSON and the share link carry, beside the build hash and the schema version, the four entry hashes of their links (dd_generation_root §3.9, "Each entry's own hash"); TASK-M8-53's `every_destination_links` (REQ-TOOL-176) checks them. This task depends on TASK-M7-45, which generates the hashes (code review F9 of PR #265, applied per R-369).
