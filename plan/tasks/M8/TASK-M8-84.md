# TASK-M8-84 — Saving an IC: the `principia.ic/1` record, copy, paste, Save IC… and Open IC…, the twin on reopening, marks in `Lines`, and the trajectory CSV with its bodies labelled 1-based (R-586, R-587)

- **Milestone:** M8
- **Closes:** REQ-TOOL-211, REQ-GUI-393, REQ-GUI-394, REQ-RENDER-119, REQ-GUI-395, REQ-TOOL-212, REQ-TOOL-213
- **Depends on:** TASK-M8-17, TASK-M8-83, TASK-M8-67, TASK-M8-58, TASK-M8-53
- **Needs (earlier milestones):** REQ-GUI-188
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~700 lines, the tests included

## Goal
An IC is saved, shared and reopened as the `principia.ic/1` record (JCS, bitwise at f64, −0 normalised): Ctrl+C and Copy as text, Ctrl+V, File › Save IC… and Open IC… into Create & locate with three outcomes and a residual; a reopened IC sits in the hemisphere it was built in, its twin opposite; a marked IC is a `Trace` of kind `Marked` whose `ic` is an `IcRef`, travelling with the view; Export trajectory… writes the CSV, one row per step, its body columns `body1_x` … (R-586, R-587).

## References
- `docs/design/principia_ic_save.md` § "3.1 The IC record"
- `docs/design/principia_ic_save.md` § "3.2 Copy, save, open"
- `docs/design/principia_ic_save.md` § "3.3 The mirror twin"
- `docs/design/principia_ic_save.md` § "3.5 In views: marks"
- `docs/design/principia_ic_save.md` § "3.6 Trajectory export (R-587)"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_lines.md` § "3. The data model"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `decisions.md` § "R-586 — An initial condition is saved, shared and reopened: the `principia.ic/1` record in JCS, bitwise at f64 with −0 normalised, copied, pasted, saved and opened into Create & locate in its as-built frame with three outcomes; a state-in entry, `computeICState`, beside `computeIC`; the mirror twin shown on reopening; a marked IC a `Trace` whose `ic` is an `IcRef`, travelling in `Lines` *(queues RQ-563 and RQ-564)*"
- `decisions.md` § "R-587 — The trajectory CSV: one row per step as taken, never resampled, its body columns labelled 1-based (`body1_x` …) as bodies are always displayed, the preamble saying so; the record's own fields and the outcome's `detail` stay 0-based with a 1-based gloss *(amends R-449 for the trajectory CSV's column labels)*"
- `decisions.md` § "R-449 — What a person reads is 1-based: the bodies are 1, 2, 3 and a pair is named by its two bodies, never by its pair id, through one conversion function; storage, computation and machine-readable exports stay 0-based and say so *(supersedes R-22 as it applies to display labels; amends R-444, R-445, R-447 and R-448 as they name the classes)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-553 — The trace's queued questions are ruled as applied meanwhile: repeats at half strength; the star at the start, Pictures' triangle; the end event over a syzygy; chevrons and edge pins separate; a presentation readout says "not a system" and names the check, `decode_failed`, its detail and "forbidden" only in a debug view; a non-finite step ends the run `sim_failed`; the continuous θ branch only; Sunset the ramp's default; three bug classes, no updated bundle; the trace leaves at `|q₃| ≥ q_max`; a curve axis's samples past its edge counted apart; T18's fade arm only where a distance is defined; TF3 the default length's; a kept orbit's `Trace` in `Lines`; TM8 on the shown axes *(closes RQ-412 to RQ-415, RQ-417, RQ-418 and RQ-421 to RQ-429; amends R-537 and R-538)*"
- `decisions.md` § "R-484 — The round-trip rule: importing any export is the identity on `SimConfig` and `RenderState`, bitwise in the canonical serialisation, on every path but the image's low-bit embedding, which drops whole tiers from the bottom (custom source, then colour) and never tier 1, physics, writing nothing rather than a partial record; the degrading is reported by the writer, the record and the reader, and badged; every field is assigned a tier in one table, exhaustive at compile time; four tests keep it from drifting; splitting tier 2 is queued (RQ-386)"
- `decisions.md` § "R-521 — Each recorded link carries its own registry-entry hash beside the build hash and the schema version: a mismatch refuses, naming exactly that link, and a match recreates across an unrelated schema change *(closes RQ-385; amends R-483)*"
- `decisions.md` § "R-318 — The canonical serialisation is JCS (RFC 8785) *(amends R-309)*"

## Deliverables
- The IC record's type, writer and reader (`crates/engine/src/ic_record.rs`), with the −0 normalisation and the non-finite refusal.
- The Inspector's Copy, Copy as text, Paste, File › Save IC… and Open IC… (and a drop), the three outcomes and the residual; the twin on reopening.
- `IcRef` in `Trace.ic` and the `Marked(id)` kind (`crates/render/src/lines/`), Mark and Mark all kept; the record's measured size written into `principia_ic_save.md` §3.5.
- Export trajectory… and the CSV writer; the columns after `Lz` defined in `principia_ic_save.md` §3.6 (REQ-TOOL-213).
- Tests: `ic_record`, `ic_copy_save_open`, `ic_reopen_twin`, `ic_marks_travel`, `ic_kept_only`, `traj_csv`, `traj_csv_columns`.

## Acceptance tests
- `cargo test -p engine ic_record` — save, open and re-save give a byte-identical file; as_built is bitwise over 10⁴ random ICs, −0 included, a −0 written 0; a record with a NaN or an infinity is refused naming the field; the re-derived canonical z equals the stored z for a recovered record (controls: a lossy number format, a −0 that survives, and p stored as primary, each fail) (REQ-TOOL-211).
- `cargo test -p gui ic_copy_save_open` — Ctrl+C puts the record's JSON on the clipboard and Copy as text a table naming bodies 1, 2, 3; Ctrl+V of that JSON opens the IC in Create & locate bitwise; Save IC… writes <label>.principia-ic.json and Open IC… reads it back as recovered with its residual shown; a record with one link's entry hash changed opens as physics only, as_built loaded and locate refused naming that link; truncated JSON opens as corrupt (controls: a default link standing in, a table numbering bodies 0, 1, 2, and a corrupt record that loads, each fail) (REQ-GUI-393).
- `cargo test -p gui ic_reopen_twin` — an IC built in the lower hemisphere, saved and reopened, has its dot in the lower hemisphere and the twin marker in the upper, and its record's hemisphere −1; one built on the seam reopens with no twin marker (control: an open that loads the canonical, folded form fails) (REQ-GUI-394).
- `cargo test -p render ic_marks_travel` — a view with one OnChart and one Built mark (an off-slice IC) round-trips bitwise through snapshot JSON, a saved view, a share link, the animation spec, the pxpack at 1024² and the tEXt chunk; at 64² tier 2 drops whole and the tEXt chunk still carries both marks; Mark all kept with three kept orbits adds three Marked traces; the record's size is measured and written into principia_ic_save.md §3.5 (controls: marks held in ViewUI, and a Built mark that round-trips only its z, each fail) (REQ-RENDER-119).
- `cargo test -p gui ic_kept_only` — under RQ-564's ruling, a view with one marked IC and one kept-only IC exported through snapshot JSON, a saved view, the tEXt chunk and a share link carries the mark and either omits the kept-only IC's Trace (the draft) or carries it (R-553), as ruled; the kept-orbit tabs never travel (control: the kept-orbit tabs in an export fail) (REQ-GUI-395).
- `cargo test -p gui traj_csv` — the CSV's rows equal the Inspector's steps one for one, bitwise at f64; a run cut by the step budget carries "incomplete: step budget"; for one collision of bodies 1 and 2 the preamble's detail is 2 (0-based) with the gloss collision 1–2, as the GUI labels it; the header row's body columns are body1_x to body3_vy in that order, and the preamble says they are 1-based; a stride of 3 keeps every third step and carries its label (controls: resampled rows, a silent cut, a skipped or doubled conversion, and a body column labelled body0_x, each fail) (REQ-TOOL-212).
- Definition: §3.6 names each column after Lz with its quantity, unit and name and the rule each name follows; physics reviewer approved; cargo test -p gui traj_csv_columns: the header row after Lz equals the defined names in order (control: a column named with a 0-based body or pair index fails) (REQ-TOOL-213).

## Notes
- R-586 and R-587 (21 Oct 2026, gap sweep PR 2c), from `ic_save.md`, with the human's one change: the CSV's body columns are 1-based. The `Marked` kind and the columns' definition are applied per R-369.
- **RQ-564 is open** (a kept-only IC against R-553): REQ-GUI-395 waits on it; meanwhile R-553 stands, a kept orbit's `Trace` round-tripping in `Lines`. Nothing else waits.
- **RQ-563 is open** (the frame integrated): TASK-M8-83's interim applies; the CSV's preamble names the frame.
- R-449 holds for every other export: the record's fields and the outcome's `detail` are 0-based and say so; only the CSV's body column labels are 1-based (R-587).
