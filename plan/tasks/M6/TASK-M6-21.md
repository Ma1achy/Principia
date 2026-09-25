# TASK-M6-21 — The event-driven precision warning: DECODE_SWITCHOVER and AT_F32_FLOOR in the snapshot

- **Milestone:** M6
- **Closes:** REQ-GUI-010, REQ-GUI-012
- **Depends on:** TASK-M6-08
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-033
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~220 lines

## Goal
The snapshot carries, GUI-sized, whether `DECODE_SWITCHOVER` has fired on visible quads and whether `AT_F32_FLOOR` has been hit, and the Explore view's precision warning is raised by those events (R-54) — never by fixed depth thresholds.

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `decisions.md` § "R-54 — The precision warning is event-driven *(GU-3 (b))*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"

## Deliverables
- `crates/engine/src/contract/snapshot.rs`: two event fields fed from TASK-M6-08's event stream, filtered to visible quads.
- `crates/gui`: the depth readout's precision warning bound to those fields.

## Acceptance tests
- `cargo test -p engine snapshot_precision_events` — snapshot fields for both events exist and flip when the scheduler reports the event on a visible quad (REQ-GUI-010).
- `cargo test -p engine precision_warning_event_driven` — zoom a visible region through the switchover and to the linear floor; the warning event is raised exactly at each event and not at any fixed depth when the events do not fire (e.g. early switchover at shallow depth with tiny q) (REQ-GUI-012).

## Notes
- None.
