# TASK-M7-75 — The margin's map is settled: drop "provisional pending RQ-788" (R-962)

- **Milestone:** M7
- **Closes:** REQ-GUI-436
- **Depends on:** TASK-M7-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~30 lines

## Goal
The built-in margin's map, the greyscale ramp of `top1 − top2` clamped to `[0, 1]`, white where `top1` is far above
`top2`, is the owner's (R-962, RQ-788 (a)). TASK-M7-12 built it so and marked it provisional. This task drops the mark
and renames the test that holds the map, citing R-962; the map itself does not change.

## References
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/gui/principia_render_gui_spec.md` § "5. Sources & multi-source"
- `decisions.md` § "R-962 — RQ-788, the built-in margin's colour map: option (a), the greyscale ramp of `top1 − top2` clamped to [0, 1], white where `top1` is far above `top2`, as built; "provisional" is dropped by a task *(closes RQ-788)*"
- `decisions.md` § "R-813 — A margin node's two field inputs sit on a `colour` or `post` occupant declaring two `// @input`s; render_gui_spec § 5's example is reworded and a `source` keeps no input *(closes RQ-675)*"

## Deliverables
- `crates/render/shaders/wgsl/frag/colour/margin.wgsl`: the comments "**The map is provisional, pending RQ-788.** …"
  and "provisional, pending RQ-788" say instead that the map is R-962's.
- `crates/render/tests/stain_graph.rs`: `stain_fanout_margin_provisional_map_is_the_grey_ramp` renamed
  `stain_fanout_margin_map_is_the_grey_ramp`, its doc comment and messages citing R-962, not RQ-788; its assertions and
  controls unchanged (R-176), its boundaries at 0 and 1 tested.

## Acceptance tests
- `cargo test -p render stain_fanout_margin_map_is_the_grey_ramp` — REQ-GUI-436's verify: OKLab `L` equal to the
  margin on `[0, 1]`, black below 0, white above 1, at 0 and 1 exactly; controls: a diverging map and an unclamped ramp
  each fail.
- `git grep -n "RQ-788" -- crates` prints nothing.

## Notes
- New (11 Nov 2026), from R-962 (RQ-788 (a)), applied per R-369 (task scoping): the wording is code, so the rulings PR
  changes none of it. Until this merges the map is drawn as now; only its label is stale.
- The test file's history is TASK-M7-12's implementer's, not qa's, so the rename is the implementer's (R-290).
