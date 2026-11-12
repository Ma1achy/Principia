# TASK-M7-75 — The margin's map is settled: drop "provisional pending RQ-788" (R-962)

- **Milestone:** M7
- **Closes:** REQ-GUI-436
- **Depends on:** TASK-M7-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~90 lines (two comments in margin.wgsl; three doc comments, two renames, three new assertions and two new
  controls in stain_graph.rs)

## Goal
The built-in margin's map, the greyscale ramp of `top1 − top2` clamped to `[0, 1]`, white where `top1` is far above
`top2`, is the owner's (R-962, RQ-788 (a)). TASK-M7-12 built it so and marked it provisional. This task drops the mark,
renames the test that holds the map, citing R-962, and tests the map's whole range: the map itself does not change.

## References
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/gui/principia_render_gui_spec.md` § "5. Sources & multi-source"
- `decisions.md` § "R-962 — RQ-788, the built-in margin's colour map: option (a), the greyscale ramp of `top1 − top2` clamped to [0, 1], white where `top1` is far above `top2`, as built; "provisional" is dropped by a task *(closes RQ-788)*"
- `decisions.md` § "R-813 — A margin node's two field inputs sit on a `colour` or `post` occupant declaring two `// @input`s; render_gui_spec § 5's example is reworded and a `source` keeps no input *(closes RQ-675)*"

## Deliverables
- `crates/render/shaders/wgsl/frag/colour/margin.wgsl`: the comments at lines 7-10 ("**The map is provisional, pending
  RQ-788.** …") and line 42 ("provisional, pending RQ-788") say instead that the map is R-962's.
- `crates/render/tests/stain_graph.rs`, every place that still names RQ-788 or calls the map provisional:
  - line 13, the module doc ("The margin's colour map is not settled (RQ-788): one test holds the provisional grey
    ramp, named so"): the map is R-962's;
  - lines 615-616, `check_margin`'s doc ("not settled (RQ-788; [`check_margin_provisional_map`])"): cites R-962 and
    the renamed check;
  - lines 688-717: `check_margin_provisional_map` renamed `check_margin_map`, and the test
    `stain_fanout_margin_provisional_map_is_the_grey_ramp` renamed `stain_fanout_margin_map_is_the_grey_ramp`, the doc
    comment and messages citing R-962, not RQ-788.
- The check keeps its two assertions (the margin 0.375 draws OKLab (0.375, 0, 0); the wires swapped, −0.375, draw
  black) and adds three: a margin above 1 draws white, and the margins 0 and 1 exactly draw OKLab (0, 0, 0) and
  (1, 0, 0) (REQ-GUI-436).
- Controls (R-176): the existing hue-wheel occupant stays, its description and `expected` reworded to name it truly (a
  hue wheel, not a diverging map) without RQ-788; two are added, a diverging map through a neutral grey and an
  unclamped ramp (which fails only the margin above 1), each failing the check.

## Acceptance tests
- `cargo test -p render stain_fanout_margin_map_is_the_grey_ramp` — REQ-GUI-436's verify: OKLab `L` equal to the
  margin on `[0, 1]` (0.375 as now), black below 0 (the wires swapped), white above 1, at 0 and 1 exactly; controls:
  the hue wheel, a diverging map through a neutral grey and an unclamped ramp each fail.
- `git grep -n "RQ-788" -- crates` prints nothing, and `git grep -n -i "provisional" --
  crates/render/shaders/wgsl/frag/colour/margin.wgsl crates/render/tests/stain_graph.rs` prints nothing.

## Notes
- New (11 Nov 2026), from R-962 (RQ-788 (a)), applied per R-369 (task scoping): the wording is code, so the rulings PR
  changes none of it. Until this merges the map is drawn as now; only its label is stale.
- The test file's history is TASK-M7-12's implementer's, not qa's, so the rename is the implementer's (R-290).
