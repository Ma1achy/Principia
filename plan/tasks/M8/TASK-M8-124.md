# TASK-M8-124 — The chart-path object, and today's pan, zoom, tilt, slice, lock, chart switch and Save view as chart re-expressed through it, bitwise (R-863 stage 1, R-864)

- **Milestone:** M8
- **Closes:** REQ-CHART-147
- **Depends on:** TASK-M4-37, TASK-M2-14, TASK-M2-22, TASK-M2-23, TASK-M2-40, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~600 lines

## Goal
The owner's stage 1 (R-863): one chart-path object in `engine`, a view a point in chart space and every edit a path
through it (R-864), and today's operations re-expressed as its named families, each reproducing today's result bit for
bit through it, before any new capability. Once an operation runs through the object its old route is deleted, not kept
as a fallback (R-864 item 2).

## References
- `decisions.md` § "R-864 — The chart-path work is one object: core types designed first as one module, a type sketch with a one-page rationale reviewed by physics and code and brought to the owner before any implementation; today's operations re-expressed as named families of it and their old routes deleted; bitwise reproduction first; special cases are named families, never if-branches; the kernel/engine split stays; restructuring `engine/nav`, `engine/lock.rs` and `engine/chart_authoring` is approved *(the architecture of PR #422's chart-path work; refines no ruling on `main`)*"
- `decisions.md` § "R-863 — When the chart-composition design's tasks are released, they are built in five stages, each merging only with the previous stage's tests green: the path object with today's operations bitwise through it; locked paths; the primitive registry and composed axes; morph families, easing and Sequence segments; time-axis charts and the symmetry filter *(the build order of PR #422's tasks; refines no ruling)*"
- `docs/design/principia_chart_space_model.md` § "3. The existing operations are paths of named families"
- `docs/design/principia_chart_composition.md` § "17.3 The constraint: bitwise through the mechanism"
- `docs/design/principia_chart_composition.md` § "17.4 `s = 0` and `s = 1`"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"

## Deliverables
- **First, the gate (R-864 item 1):** the type sketch of the core types (Chart, Path and its families, Lock and anchor,
  Axis and primitive, PairMap, Hold) as one module, with a one-page rationale, in a design note under `docs/design/`
  named in the PR, on the PR's first commit; reviewed there by the physics and code reviewers against the chart-space
  model, then taken to the owner through `/rulings-report`.
- After the owner has approved the sketch: the module in `crates/engine/src/`, and `engine/nav`, `engine/lock.rs` and
  `engine/chart_authoring` restructured to fit it (approved by R-864); pan, zoom, tilt, slice, lock, chart switch and
  Save view as chart each a constructor of the object, a path of a named family; each operation's old route deleted.
- **Stage 1's fence (R-863, "before any new capability").** The sketch designs every core type; the module implements
  only what today's operations use. Composed Axis and primitive and PairMap (stage 3's), Hold (TASK-M8-118's, unplaced:
  RQ-744 (C)) and any family beyond today's operations are sketched only, and carry no new behaviour until their
  stage's task.
- The bitwise baseline, today's results recorded from today's code on its own commit before the re-expression.
- Chart maps and decode stay in `kernel` (R-864 item 5): nothing here changes a kernel bit.

## Acceptance tests
- Review: the type sketch and its one-page rationale approved by the physics and code reviewers, then approved by the
  owner, the answer recorded as a ruling, before any implementation commit: no commit on the branch touches `crates/`
  before it (REQ-CHART-147).
- `cargo test -p engine -- path_pan_bitwise path_zoom_bitwise path_tilt_bitwise path_slice_bitwise path_lock_bitwise path_chart_switch_bitwise path_save_view_bitwise path_presets_bitwise` —
  each operation through the object against the recorded baseline: sim key, IC words and image hash bit for bit, on
  latent, curved and saved charts; every preset and saved chart today's code builds, reopened through the object;
  controls: one floating-point operation reordered in a family, the general geodesic in place of the tilt family, a path
  triple in a zoom's sim key, each failing (REQ-CHART-147).
- Review: no operation's old route left beside the object, and no special case an if-branch outside a named family
  (R-864 items 2 and 4); no family, pair map or hold beyond today's operations given behaviour (R-863 stage 1)
  (REQ-CHART-147).

## Notes
- **Build order (R-863):** stage 1, the path object and the bitwise tests, before any new capability. Stage 2,
  TASK-M8-117, depends on this task and merges only with its tests green.
- **Gate (R-864 item 1):** dispatched first for the sketch only. No implementation commit lands before the owner has
  approved the sketch, the answer recorded as a ruling; the rest of the task then goes ahead on the same branch.
- **Routes covered.** The bitwise baseline and the deletion cover every route of the seven operations on `main` when
  this task starts, the PR naming each: at least `engine/nav`'s `pan`, `zoom`, `slice`, `tilt` and `tilt_pair`,
  `engine/lock.rs`'s lock, and the chart switch's route. Save view as chart has no code on `main` yet: its routes are
  TASK-M6-49's and TASK-M6-51's (the mock) and TASK-M8-48's (the real engine, curved views; R-475), none merged and none
  in Depends on. Which comes first, and so whether this task's baseline holds their routes or they are built through
  the object, is RQ-744 (E); until it is answered, a Save view route merged before this task starts is in the baseline
  and the deletion like the rest.
- If an operation can't reproduce bitwise as a family of the object, or seems to need a special case outside one, stop
  and bring it back (R-864 item 4): never patch around it.
- REQ-CHART-113's tests (`path_tilt_bitwise`, `path_zoom_bitwise`, `path_lock_bitwise`, `path_save_view_bitwise`),
  TASK-M8-106's, overlap this task's; which task keeps them is RQ-744 (B).
- Waits on RQ-721 (chart paths; answered by R-850, with its rider R-849, the owner's reply of 7 Nov 2026) and, like
  every task of the design, on TASK-M4-37.
