# TASK-M7-68 — Print graph cost classes measured: every node at 1024² on the reference machine, classed from the numbers, and each reduced node's factor (R-488)

- **Milestone:** M7
- **Closes:** REQ-PERF-096, REQ-PERF-097
- **Depends on:** TASK-M7-56, TASK-M7-57, TASK-M7-58, TASK-M7-59, TASK-M7-60, TASK-M7-61, TASK-M7-62, TASK-M7-63, TASK-M7-64, TASK-M7-65, TASK-M7-66
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~300 lines, the bench and the table included

## Goal
Time every registered node at 1024² on the reference machine (the human's Mac, R-186), at its registry defaults and at the heaviest value of each parameter any of the 111 presets sets, as the median of 60 timed runs after 10 warm-up runs, the class taken from the heavier; propose the thresholds dividing `light`, `medium` and `heavy` and each `reduced` node's factor (½ or ¼), and replace every provisional class in the registry with the measured one. The table is what the arbiter's post level is solved from (TASK-M8-73).

## References
- `docs/design/principia_print_graph.md` § "8.3 Cost classes"
- `docs/design/principia_print_graph.md` § "8.1 Degrade while moving"
- `docs/design/principia_print_graph.md` § "4.3 Node types are a registry"
- `decisions.md` § "R-488 — Degrade while moving: each Print graph node runs by its motion policy (`full`, `reduced`, `bypass`), the whole graph falls back a level while over budget, and full quality returns on settle; the playhead advancing is not motion"
- `decisions.md` § "R-557 — A Print graph field input takes the stain's window when the stain colours by the same field, else the field's own `range_norm` default; the print stage's time is recorded but never moves the Auto rung, and its post level is the arbiter's motion-regime lever, under Auto only; the human approves PR #268 with its follow-ups *(closes RQ-478, RQ-479 and RQ-495; amends R-488)*"

## Deliverables
- `cargo xtask bench print_cost_classes` and its table in the PR and in `crates/render/src/print/nodes/` (each entry's class and factor).
- Test `print_cost_class_matches`.

## Acceptance tests
- `cargo xtask bench print_cost_classes`, on the human's Mac (R-186) — REQ-PERF-096's verify (every registered node timed at 1024² at its defaults and at its presets' heaviest settings (median of 60 timed runs after 10 warm-up runs, GPU timestamps), the class from the heavier, the table recorded in the PR and in the registry; controls: a node classed from its defaults where a preset's heavier setting crosses a threshold fails, and a registry entry whose class disagrees with its measured time fails `cargo test -p render print_cost_class_matches`).
- Calibration: the PR proposes the thresholds and each `reduced` node's factor from REQ-PERF-096's table, with the gaps in the measured distribution they sit in; a reviewer checks them; the human confirms them at the M7 gate (REQ-PERF-097).

## Notes
- §8.3 says the port's first task measures every node; a node can be timed only once it exists, so this task follows the port tasks, whose classes are provisional until it lands (applied per R-369).
- Benchmarks run on the human's Mac (R-186).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 3 of PR #281 (perf review 5608473498, F6, applied per R-369):** nodes are timed at their defaults and at the heaviest setting any preset uses, the class from the heavier, as the median of 60 timed runs after 10 warm-up runs (the profiler's 60-frame window; percentiles, not means). *Was:* "`cargo xtask bench print_cost_classes`, on the human's Mac (R-186) — REQ-PERF-096's verify (every registered node timed at 1024² (median of repeated runs), the table recorded in the PR and in the registry; control: a registry entry whose class disagrees with its measured time fails `cargo test -p render print_cost_class_matches`)."; "me every registered node at 1024² on the reference machine (the human's Mac, R-186), propose the thresholds dividing `light`, `medium` and `heavy` and each `reduced` node's factor (½ or ¼), and replace every provisional class in the registry with the measured one. The table is what the arbiter's post level is solved from (TASK-M8-73).".
