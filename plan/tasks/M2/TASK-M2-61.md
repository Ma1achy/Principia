# TASK-M2-61 — The chart-path object, and today's pan, zoom, tilt, slice, lock, chart switch and Save view as chart re-expressed through it, bitwise (R-863 stage 1, R-864)

- **Milestone:** M2
- **Closes:** REQ-CHART-147, REQ-CHART-113, REQ-CHART-160
- **Depends on:** TASK-M2-58, TASK-M2-14, TASK-M2-22, TASK-M2-23, TASK-M2-40, TASK-M2-52, TASK-M2-38, TASK-M2-39, TASK-M2-56, TASK-M2-60
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
- `decisions.md` § "R-873 — The build order's leftovers: the milestones are kept and the order across them kept by dispatch; TASK-M8-106 is split, its stage-1 tests to TASK-M8-124; each unplaced task goes in the stage it serves, points of interest one task in stage 3; and the stage-1 path object depends on the Save view tasks *(closes RQ-744)*"
- `decisions.md` § "R-899 — The chart-path brief's nine questions: the lock test reads the recorded state at the pin; snap-back undoes the slice only, auto-snap off; jumps under a lock kept and labelled until the sliding anchor exists; the pin's screen position stored, "assumed" at the centre where none can be derived; a pivot record in the view, untilt (a1), amending R-473; points of interest one task in stage 3; their star filled within the lock's tolerance, the off-slice distance shown; marks mid-morph to pixel resolution, the filter off, labelled; the block line built in stage 2 *(amends R-473, and the snap-back of chart_decoder_contract Part 4 and design § 17.8)*"
- `decisions.md` § "R-901 — Easing and the path curve are a property of each segment of a path, per the chart-space model's Theorems 4 and 5, one representation shared by live morphs, saved paths and export, built with the path object *(amends R-576 and export contract Part 3)*"

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
- `cargo test -p engine -- path_tilt_bitwise path_zoom_bitwise path_lock_bitwise path_save_view_bitwise path_keyframe_bitwise path_ends_bitwise path_record_round_trip` — each against the direct operation's bits; controls: the general geodesic in tilt's place, a triple in a zoom's key, an end through the blend, s at f32 (REQ-CHART-113, moved here from TASK-M2-81 by R-873).
- `cargo test -p engine -- path_easing_timing_only path_concat_join` — re-timing a segment changes when frames are shown and no frame's content, bitwise; two eased segments concatenate with the join at the shared end; a saved path and an export read the same timing as the live morph; controls: an easing applied to a frame's content fails, a second easing beside the segment's fails (REQ-CHART-160).
- Review: each edit that jumps the view under a lock (Home, the resets, presets, steers, capture) labelled as a cut, and the PR stating its non-conformance with R-849 until stage 2's sliding anchor (R-899 Q3 (a); REQ-CHART-147).

## Notes
- **Build order (R-863):** stage 1, the path object and the bitwise tests, before any new capability. Stage 2,
  TASK-M2-65, depends on this task and merges only with its tests green.
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
  TASK-M2-81's, overlap this task's; which task keeps them is RQ-744 (B).
- Waits on RQ-721 (chart paths; answered by R-850, with its rider R-849, the owner's reply of 7 Nov 2026) and, like
  every task of the design, on TASK-M2-58.
- *8 Nov 2026 (R-873, R-899, R-901):* RQ-744 is ruled: (B) (a), REQ-CHART-113's tests are this task's; (E) (i), Save view as chart (TASK-M6-49, TASK-M6-51, TASK-M8-48) and TASK-M2-60 come before stage 1, so they are in Depends on and their routes are in the baseline. The brief's Q3 (a): the edits that jump the view under a lock are kept, labelled as cuts, the non-conformance with R-849 stated. R-901: easing and the path curve are a field of each path segment, built here with the path object, Theorems 4 and 5 stated and tested (REQ-CHART-160). R-894: TASK-M8-129, the consolidation, follows this task and TASK-M8-125, before stage 2. The two notes above on RQ-744 (B) and (E) are overtaken. **Timing:** stage 1 closes when this task and TASK-M8-125 merge with their tests green. *Was:* "- **Closes:** REQ-CHART-147"; "- **Depends on:** TASK-M2-58, TASK-M2-14, TASK-M2-22, TASK-M2-23, TASK-M2-40, TASK-M2-52"
- *9 Nov 2026 (R-916):* moved from M8 to M2 and renumbered: formerly TASK-M8-124; the header follows plan/tasks.yaml. *Was:* "Milestone: M8"; "Depends on: TASK-M2-58, TASK-M2-14, TASK-M2-22, TASK-M2-23, TASK-M2-40, TASK-M2-52, TASK-M6-49, TASK-M6-51, TASK-M8-48, TASK-M2-60".
- *9 Nov 2026 (R-916, R-919):* R-916 supersedes R-873 (E) (i) (F2): this task no longer depends on TASK-M6-49, TASK-M6-51 or TASK-M8-48, which would be forward edges out of M2; Save view as chart on curved views is built through the object by TASK-M2-63, so the "Routes covered" note's wait on RQ-744 (E) is overtaken. TASK-M2-38, TASK-M2-39 and TASK-M2-56 merge before the baseline commit and are in Depends on. R-919: the type sketch (R-864 item 1) is approved when this task brings it, before its baseline commit, starting from type-sketch v10; the owner sees it then, and the gate above reads with that. The baseline writes `fixtures/chart_composition/digest.json`, which TASK-M2-66 is held to (applied per R-369).
