# TASK-M7-50 — Replace-L's keep in gamut option: chroma scaled by min(1, L/0.75, (1 − L)/0.35), off by default (R-542 SM15, R-558)

- **Milestone:** M7
- **Closes:** REQ-COL-132
- **Depends on:** TASK-M7-04, TASK-M7-48
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
SM15's chroma scaling as Replace-L's option **keep in gamut**, off by default, so every brightness level stays in gamut where it is on; Replace-L as ruled stays the default, so dd_colouring §3.5, its test 6 and colour composition §4.1's truth table hold and every saved stain renders bitwise as before (R-558, RQ-496 (a)). *Was:* "SM15's chroma scaling under Replace-L, so every brightness level stays in gamut. **It waits on RQ-496**: it changes Replace-L for every stain against dd_colouring §3.5, its test 6 and colour composition §4.1's truth table; main's Replace-L stands meanwhile." **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM15. In the stain graph editor"
- `docs/design/principia_state_map.md` § "SM12. Tests (each with the arm that fires)"
- `docs/design/principia_dd_colouring.md` § "3.5 Combiners"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `decisions.md` § "R-558 — The state map's twenty entries are ruled: any chart may be a target, its inverse generated beside the view's; a tilted target the branch nearest the slice; never a criterion input; the vMF colouring kept and stated not one-to-one; the nominal copy; an ended run held; no history fields; the prototype's constants the starting values; Chart position a named seam-6 exception, with no fixed t; Replace-L's chroma scaling an option, "keep in gamut", off by default; Shape n̂ stays `vector`; the code's reading of the mirror and the clamp; the file's preset names and the trace's fade; "Edit stain…"; no figure ring under the trace; one colour node; `α_min = 0`; "none" a setting of the node *(closes RQ-480 to RQ-489 and RQ-496 to RQ-505; amends R-542)*"

## Deliverables
- The option, a parameter of the Replace-L combiner, off by default (`crates/render/`; R-558 S2); its inspector checkbox is TASK-M7-51's.
- Test `replace_l_chroma`.

## Acceptance tests
- `cargo test -p render replace_l_chroma` — REQ-COL-132's verify.

## Notes
- **RQ-496 is ruled (a) (R-558):** an option, off by default; this task no longer waits. *Was:* "**Waits on RQ-496** (Replace-L as ruled, R-77; R-542 H9)." and "If RQ-496 rules (c), no scaling, this task is retired with REQ-COL-132."
