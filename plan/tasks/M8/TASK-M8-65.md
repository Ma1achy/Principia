# TASK-M8-65 — Listen in Explore, 2: the Listen… window (R-541 G6)

- **Milestone:** M8
- **Closes:** REQ-GUI-329
- **Depends on:** TASK-M8-64, TASK-M8-12, TASK-M8-59, TASK-M8-60
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
The `Listen…` window of R-541 G6, split from TASK-M8-62: every sound setting of `principia_listen.md` in two columns with the readout row, the instrument menu grouped by family, R-555's defaults (the code's, sound off at launch and Sine (pure), but reverb, Doppler and width on), nothing on the sim key. It waited on RQ-440 and RQ-442 through TASK-M8-12; R-555 rules both.

## References
- `decisions.md` § "R-541 — The trace and Listen in the dev GUI's Explore mode, as `principia_trace_listen_gui.md` places them: everything about one trajectory in the Trajectory panel, each kept trajectory a tab with its own previews, trace and sound, its Trace, Listen and Numbers sections opening `Trace…` and `Listen…`; a Trace group in Overlays ▾; no trace control in the left panel or in Lines ▾; Listen adding to the bottom band's Time column and nothing else *(amends R-476 and R-538)*"
- `docs/gui/principia_render_gui_spec.md` § "Listen… — every sound setting (R-541 G6)"
- `docs/gui/principia_render_gui_spec.md` § "G5. Windows (`04_windows.png`)"
- `docs/gui/principia_render_gui_spec.md` § "G3. Keyboard — a design note, not a screen (`07_keyboard.png`)"
- `docs/design/principia_listen.md` § "8. Instruments"
- `decisions.md` § "R-540 — Listen, the trajectory heard as sound, as `principia_listen.md` gives it and its demo's code transcribes it: the spectrum of the motion of the triangle's shape plus its rotation, complex so that the winding's sign is the stereo side, pitched by the crossing time so that it is scale-free, following the head or played whole, every moving run heard; the instrument voices it and changes nothing measured; events and effects; its build waits on RQ-440 and RQ-442 *(extends R-109)*"

## Deliverables
- The `Listen…` window (`crates/gui/`), moving and resizing as every window does (R-461).
- The instrument menu's family assignment, written here (R-72; RQ-459 (a), R-556: the 39 instruments, the counts met, reviewed by gui), into `docs/design/principia_listen.md` §8 with the porting rule's "Removed lines" note.
- Test `listen_window`; screenshot `listen_window`.

## Acceptance tests
- cargo xtask screenshot listen_window (instrument menu open): every control of §G5's Listen… table with its default (§G5's table: the code's, but reverb, Doppler and width on, R-555), the two columns, the readout row and the grouped menu, each family holding its count; cargo test -p gui listen_window: each control sets its Listen field and none touches the sim key, Horizon included; only Horizon starts an integration, of the shown trajectory through computeIC; Tab and Enter reach the controls column by column (Sound, Loudness, Listen; then Pitch & analysis, Events, Effects), group by group in §G5's order, in navigation mode, each changed only in interaction mode (§G3, R-409); controls: a sound setting of principia_listen.md missing, a Listen edit that changes the sim key or the quad requests, a non-Horizon Listen edit that calls computeIC (the call counter, as REQ-GUI-269 reads it), and a control out of that order or changed in navigation mode, each fail (REQ-GUI-329).

## Notes
- **Split (code review 5595371779 of PR #267, F2; the orchestrator's decision, applied per R-369):** from TASK-M8-62, which keeps `Trace…`.
- **Ruled (21 Oct 2026):** RQ-454, the defaults Sine (pure) and sound off at launch, with reverb, Doppler and stereo width on at ×1.40, ×1.50 and ×1.50 and bit-crush off (R-555, with the human's Listen effects follow-up, which overrides the code's unchecked three); RQ-459 (a), this task writes the families, ruled so by R-556. *Was:* "**Applied meanwhile (R-541):** RQ-454, the code's defaults; RQ-459 (a), this task writes the families. A ruling other than these is a follow-up change."
- The gui reviewer checks the family assignment against the counts (pure 5, keys 3, struck 10, bowed and blown 7, voices 4, synths 8, textures 2).
- REQ-GUI-329's sim-key arm and keyboard arm are gui review 5595359543 of PR #267's G3 and G2 (applied per R-369): Listen never touches the sim key, Horizon included.
- **gui review 5595467390 of PR #267, H2: a non-Horizon Listen edit calling computeIC fails** (applied per R-369). *Was:* "cargo xtask screenshot listen_window (instrument menu open): every control of §G5's Listen… table with the code's default, the two columns, the readout row and the grouped menu, each family holding its count; cargo test -p gui listen_window: each control sets its Listen field and none touches the sim key, Horizon included; only Horizon starts an integration, of the shown trajectory through computeIC; Tab and Enter reach the controls column by column (Sound, Loudness, Listen; then Pitch & analysis, Events, Effects), group by group in §G5's order, in navigation mode, each changed only in interaction mode (§G3, R-409); controls: a sound setting of principia_listen.md missing, a Listen edit that changes the sim key or the quad requests, and a control out of that order or changed in navigation mode, each fail (REQ-GUI-329)."
- Fix round 1 of PR #274 (gui G1, applied per R-369): the acceptance line's default reads §G5's table's, the code's but reverb, Doppler and width on (R-555). *Was:* "every control of §G5's Listen… table with the code's default".
