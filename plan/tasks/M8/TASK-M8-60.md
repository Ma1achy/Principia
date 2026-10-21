# TASK-M8-60 — Listen, 3: horizon and dwell, the spectrum, spectrogram and readout, sweep a row and the known orbits (R-540 S10 to S12)

- **Milestone:** M8
- **Closes:** REQ-GUI-321, REQ-GUI-322, REQ-GUI-323
- **Depends on:** TASK-M8-12, TASK-M8-49
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~800 lines, the tests included

## Goal
**This task waits on RQ-440 and RQ-442** (R-540 H14). Listen's horizon and dwell, display and gestures, `principia_listen.md` §10 to §12: the moving hover's run extended on dwell or pin to Listen's `Horizon`, the trace and the sound switching together; the spectrum, the spectrogram (display only, dB above its local mean), its event lane and the readout; sweep a row and the four known orbits, the tests' calibration set.

## References
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"
- `docs/design/principia_listen.md` § "10. Horizon and dwell"
- `docs/design/principia_listen.md` § "11. Display"
- `docs/design/principia_listen.md` § "12. Sweep and known orbits"
- `docs/design/principia_listen.md` § "13. Tests (each with the arm that fires)"
- `docs/design/principia_scratchpad_pointer_channels.md` § "3. UPDATE TO THE RESPONSIVENESS MODEL — hover is affordable, and the earlier caution was wrong"
- `decisions.md` § "R-538 — The trace's markers and icon sets, as `principia_trace_markers.md` gives them: a star at the start, each end event its own icon in its outcome's colour, "?" for an incomplete run, rings where the state leaves and re-enters the chart's reach, syzygies o–o–o and off by default, off-screen icons pinned faint to the edge, every icon repeated in every image of the chart, never drawn through or started in a pixel that is not a system *(amends R-476)*"

## Deliverables
- The dwell extension in the hover integration's caller, the spectrum and spectrogram painters and the readout, the sweep and the known orbits (`crates/gui/src/listen/`).
- Tests `listen_dwell`, `listen_display_only` and `listen_known_orbits`.

## Acceptance tests
- cargo test -p gui listen_dwell: a hover held 280 ms (a pin at once) on a run that reached the moving horizon without an event is integrated on to Horizon, the trace and the sound switching in the same frame (cross-fade 260 ms); a run ended by its event is not extended; a pointer that moves on cancels it; the map's quads are unchanged; controls: the sound switching without the trace, or an extension of an ended run, each fail (REQ-GUI-321).
- cargo test -p gui listen_display_only (R-540 S13 Spectrogram is display-only): the voiced peaks are identical with the spectrogram on and off and under any display mode; a column's values equal §11's contrast formula and bins 0 to 3 are blank; the readout names the reason for a silence; control: the contrast step leaking into the analysis fails (REQ-GUI-322).
- cargo test -p gui listen_known_orbits: each known orbit loads §12's masses, positions and velocities, total mass 1 and G = 1, pinned and run to the horizon at once; a sweep's pointer positions are §12's and it sends no SetField; pressing it again stops it; control: a known orbit's IC off §12's digits fails (REQ-GUI-323).

## Notes
- **Waits on RQ-440 and RQ-442** (R-540 H14). The task does not start until they are ruled; their rulings may change this file's deliverables and acceptance (a port of the ruling does so first).
- **Applied meanwhile (R-540 H8):** RQ-464 (a): the moving hover to the map's own horizon under pointer_channels §3's step budget, on to Listen's `Horizon` on dwell or pin, the map unchanged. A ruling other than this is a follow-up change.
- Where the spectrum and the spectrogram sit in the GUI is TASK-M8-61's and TASK-M8-63's (R-541 G4, G7); this task draws them.
