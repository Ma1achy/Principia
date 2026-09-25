# TASK-M5-22 — Transport: pause, restart, loop and the playhead clock

- **Milestone:** M5
- **Closes:** REQ-SCHED-036, REQ-TOOL-046, REQ-TOOL-047, REQ-TOOL-048, REQ-TOOL-049
- **Depends on:** TASK-M5-21
- **Needs (earlier milestones):** REQ-SCHED-011, REQ-SCHED-013, REQ-SCHED-084
- **Reviewers:** code, qa, perf
- **Pitfalls:** PIT-1
- **Size:** ~300 lines

## Goal
Transport works on the lockstep loop: the global playhead advances by fixed dt with every visible pixel's
`SimState` marching with it and no stored temporal data; pause freezes the playhead but not compute (revealed quads catch
up to the frozen t and promote); restart sets the playhead to 0 and discards and re-decodes the live set; loop restarts
at t_end, the horizon T or when every visible sample has latched terminal. The GUI's clock advances `RenderState`'s
playhead each frame through a `SetField` marked "no history"; transport is `ViewUI` state and invalidates nothing.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 7 — The frame loop (lockstep presentation)"
- `docs/design/principia_temporal_architecture_note.md` § "Transport controls (free from statelessness)"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/design/principia_temporal_architecture_note.md` § "Footguns (all re-applications of disciplines already established)"
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"
- `docs/contracts/principia_export_animation_contract.md` § "Part 1 — Playback is the temporal mechanism"
- `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*"
- `decisions.md` § "R-101 — Transport lives in `ViewUI`; the clock writes the playhead without history *(closes RQ-61)*"

## Deliverables
- `crates/engine`: transport fields in `ViewUI`; the "no history" `SetField` marking.
- `crates/engine/src/frame/transport.rs`.
- Tests `crates/engine/tests/transport.rs`.

## Acceptance tests
- `cargo test -p engine transport_semantics` — pause then pan: new quads promote at the frozen t; restart zeroes t and clears live state; loop wraps at t_end; play/pause/speed/loop leave the sim key and every cache entry unchanged; playback adds no undo entry (REQ-SCHED-036).
- `cargo test -p engine playhead_fixed_dt` — no per-pixel history buffer exists; the playhead advances dt per step (REQ-TOOL-046).
- `cargo test -p engine pause_catch_up_promotes` — pause, pan: new quads catch up to the frozen t and promote; the playhead stays fixed (REQ-TOOL-047).
- `cargo test -p engine restart_redecodes` — after restart all visible SimStates equal fresh decodes at t = 0 (REQ-TOOL-048).
- `cargo test -p engine loop_restarts_on_latch` — a region that fully latches before T restarts at the latch time (REQ-TOOL-049).

## Notes
- The undo stack itself is M8 (gui_state_contract); this task asserts that a playback `SetField` carries the
  "no history" mark and adds no entry to the engine-side edit log.
