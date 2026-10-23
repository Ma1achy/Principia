# TASK-M8-78 — Sequence: Play preview, the proof strip, the planner's estimate and its basis, and export with the spec beside it (R-577)

- **Milestone:** M8
- **Closes:** REQ-GUI-379, REQ-GUI-380, REQ-GUI-381, REQ-PERF-110, REQ-TOOL-200, REQ-GUI-387
- **Depends on:** TASK-M8-77, TASK-M8-30, TASK-M5-28, TASK-M8-32
- **Needs (earlier milestones):** REQ-TOOL-050
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~450 lines, the tests included

## Goal
Play preview runs the sequence through the live frame loop from the timeline cursor, link parameters and dissolves as cuts (R-520 F3), under its banner, its frames entering no undo (R-101). The proof strip renders one small frame per keyframe and per segment midpoint through the blocking exporter at its lowest tier, each bitwise the exported frame. Export… opens the planner first, one row per segment with class, frames, cost, flags and an estimate that carries its basis (a range from this machine's frame records, or "no measurement on this machine yet", never a number), then the export's parameters, Start in the background with progress, ETA, thumbnail and Cancel; the `.principia-seq.json` is written beside every export (R-577).

## References
- `docs/design/principia_sequence.md` § "3.6 Preview and the proof strip"
- `docs/design/principia_sequence.md` § "3.7 The planner and export"
- `docs/gui/principia_render_gui_spec.md` § "Sequence — keyframed view animations (R-576, R-577)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 2 — The two playback types and their costs"
- `docs/contracts/principia_export_animation_contract.md` § "Part 4 — Export: the frame loop in blocking mode"
- `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text, readable by the sender"
- `decisions.md` § "R-577 — The Sequence window, as `keyframes_reel.md` gives it: whole keyframes on a timeline with read-only lanes, segments coloured by class, easing and transitions per segment, Play preview with cuts, a proof strip of exported frames, a planner whose estimate carries its basis and never guesses, and the spec saved as `.principia-seq.json` and written beside every export *(queues RQ-559 and RQ-560)*"
- `decisions.md` § "R-520 — Link parameters interpolate across keyframes in a blocking export only, eased linearly with every frame re-integrated, and the export is flagged; live playback keeps the cut, and a changed link id is a cut everywhere *(closes RQ-384; amends R-483)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"
- `decisions.md` § "R-489 — The instrument stays honest: measurement never reads the print, goldens and scientific checks run under Plain, a styled figure is badged, and figure export defaults to Plain"

## Deliverables
- Play preview in the Sequence window: the live sequence player, sending each frame's state with "no history"; TASK-M8-79's takeover continues in it.
- The proof strip through TASK-M8-30's blocking exporter at the lowest export quality tier.
- The planner (rows from TASK-M8-31's classification, the estimate from the frame records, proof frames timed into the measurement) and the export dialog, the job TASK-M8-30's.
- The `.principia-seq.json` written beside every export.
- Tests: `preview_cuts_links`, `proof_equals_export`, `sequence_planner`, `planner_no_guess`, `export_writes_seq`, `preview_holds_explore`.

## Acceptance tests
- `cargo test -p gui preview_cuts_links` — a segment easing a link parameter previews holding the first keyframe's value until the second keyframe and its value from there, the banner shown while it plays and the snapshot's preview set; the preview sends no SetField, and after Stop, and after the spec's end, SimConfig and RenderState equal those before the preview bitwise and the undo depth is unchanged; a pan while it plays ends the preview and pans the user's view from before it, one history entry, and an undo while it plays ends it and undoes the user's last entry, the preview's frames untouched; a no-history Hover SetField sent while it plays does not end it; Esc with the focus in the Sequence's timeline ends it, the focus still on the timeline in the same mode, and a second Esc then acts as §G3 has it; the same segment's export is eased and flagged (controls: a preview that interpolates the parameter, one that sends its frames as SetFields, one that leaves the view at its last frame, one whose pan lands on the preview's state, one ended by a Hover SetField, one that an undo does not end, and an Esc that stops it and also moves the focus, fail) (REQ-GUI-379).
- `cargo test -p gui proof_equals_export` — a three-keyframe spec gives five proof frames, each equal bitwise to the frame an export at the same time, size and tier writes (control: a proof frame taken from the live loop fails) (REQ-GUI-380).
- `cargo test -p gui sequence_planner` — a three-segment spec of classes (a), (b) and (c) gives three rows whose costs are one march to the segment's end, one 0 → t_frame march per frame and none, a link-easing segment flagged and a dissolve flagged as two renders; nothing integrates until Start; Start runs off the GUI thread with progress, ETA and the last frame's thumbnail updating, and Cancel stops it (controls: a planner that starts the job on opening, and one that prices a (c) segment above zero, fail) (REQ-GUI-381).
- `cargo test -p gui planner_no_guess` — with no frame records the estimate reads "no measurement on this machine yet" and contains no digit; with records of throughputs 2 and 4 units per second, a segment costing 8 units reads 2 s to 4 s; after a proof strip its frames' times are in the measurement (controls: an estimate from a default throughput with no records, and one ignoring the slowest record, fail) (REQ-PERF-110).
- `cargo test -p gui export_writes_seq` — an export as PNG frames and one as GIF each leave a .principia-seq.json beside the output, bitwise the file Save… writes for the spec (control: an export with no file beside it fails) (REQ-TOOL-200).
- `cargo test -p gui preview_holds_explore` — during a preview of a spec whose stain and chart differ from the user's view, the header strip shows the preview badge and no decoder badge, the Legend shows only "Preview: the legend returns when it stops", t reads "—", a cursor moved over the figure sends no Hover and draws no sample, and the Manifold view shows the user's z₀; after Stop the Legend, the decoder badge and t return (controls: a Legend or a decoder badge that describes the user's view during the preview, a number shown for t, a hover sample drawn, and Manifold values taken from the preview's frame, each fail) (REQ-GUI-387).

## Notes
- R-577 (21 Oct 2026, gap sweep PR 2b). The estimate's unit (samples times physical time marched, per wall second from the frame record's `samples`, `playhead_dt` and `frame_ms`) and the range's ends are applied per R-369.
- How a GIF, MP4 or WebM carries the spec in itself is the video carrier's design (gap sweep PR 2c), not this task's.
- Perf reviews the planner's measurement and the preview in the frame loop.
- **Fix round 1 of PR #286 (gui review 5631060675, F2; applied per R-369):** Play preview is a transient view the engine plays from a preview request carrying the spec (the request and the snapshot's `preview` are `principia_gui_state_contract.md` §2's); no SetField is sent, the view returns bitwise when it stops or ends, and a user edit ends it first (REQ-GUI-379). Acceptance line regenerated from the verify.
- **Fix round 2 of PR #286 (gui review 5631227164, G1, G2, G4; applied per R-369):** a preview ends at an edit that enters the history, or undo or redo, not at the hover; Esc in the Sequence stops it and nothing else (REQ-GUI-379); what Explore shows during a preview is REQ-GUI-387 (new). Acceptance lines regenerated from the verifies.
