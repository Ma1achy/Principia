# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-263: the shape sphere's extension past `[0,1]²` (R-407) redraws systems already in `[0,1]²`; whether area statistics count the window or only `[0,1]²` is not given *(physics, R-407, R-406, R-141, TASK-M8-05)*

- **File, section:**
  - `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic,
    pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*", the human's
    words: "periodic (wraps), pole-crossing (continues over the pole, partner axis shifted by half its period)" and
    "sphere: θ periodic, φ pole-crossing".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract",
    `system_image`: "**n-to-1** — a fixed finite number of pixels share each system. Carries the fold so downstream
    draws/labels one representative. The shape sphere is n-to-1 with n = 2", and, for a ray-degenerate chart, "the
    quantitative layer must not read areas as system fractions".
  - `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison
    checklist)", W7: "**Measure honesty** — every arbitrary choice carries its Jacobian or is barred from quantitative
    claims".
  - `docs/gui/principia_render_gui_spec.md` § "G7. Chart builder (`03_chartbuilder.png`)": "**The Domain preview:** the
    chart's admissible region in its own coordinates, the forbidden region hatched, the current view as a rectangle, the
    boundary's formula, and "forbidden in view: N%"".
- **Silence:** past `[0,1]²`, the shape sphere's θ wraps and its φ crosses the poles with θ shifted by π (R-407), so
  every pixel past `[0,1]²` shows a system that `[0,1]²` already holds, beside the 2-to-1 hemisphere fold the
  `system_image` descriptor records. The descriptor's multiplicity and the area statistics read from it (W7's measure
  honesty; §G7's "forbidden in view: N%") are defined for `[0,1]²`; whether a statistic taken while the window is
  extended counts the window or only `[0,1]²`, and how it counts the redrawn systems, is not given. The same question
  holds for any chart with a periodic or pole-crossing axis.
- **Options seen:**
  - (a) Statistics are taken over `[0,1]²` only; the extension is drawn and never counted.
  - (b) Statistics are taken over the window, duplicates included: a redrawn system counts once per pixel that shows
    it.
  - (c) Statistics are taken over the window, duplicates counted once: each system counts once however many window
    pixels show it, as `system_image` carries the hemisphere fold.
- **Applied meanwhile:** nothing computes an area statistic on the extended window. TASK-M8-05 has no such line, and
  §G7's "forbidden in view" is drawn in the Chart builder, a window of the egui layer, which shows only while the figure
  is in its rect, the view's `[0,1]²`.
- **Waits:** only a line of TASK-M8-05 that computes such a statistic on the extended window; it has none, so nothing
  is blocked today. A later task that computes one over the window waits.
