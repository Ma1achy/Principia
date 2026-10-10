# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-264: with the shape sphere's hemisphere toggle drawing one hemisphere, what the pole-crossing φ does past the edge that is not a pole, and what its primary range is, is not given *(definition, physics, R-408, R-407, R-113, R-141, TASK-M8-05)*

- **File, section:**
  - `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic,
    pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*", the human's
    words: "pole-crossing (continues over the pole, partner axis shifted by half its period)" and "sphere: θ periodic,
    φ pole-crossing".
  - `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts
    only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity
    failures are forbidden *(closes RQ-263)*", the human's words: "pole-crossing = pole to pole".
  - `docs/design/principia_chart_reference.md` § "3.3 The chart map": "Draw one hemisphere and say so, or draw both and
    flag the redundancy; the hemisphere toggle lives in the Manifold view's Chart section (render_gui_spec §G2, R-113)".
  - `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)": "the **hemisphere
    toggle** (one hemisphere, or both with the redundancy flagged)".
  - `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set", the
    shape-sphere row: "(s,t)→(θ,φ) by R-14's map
    (`principia_chart_reference.md` §3.3: θ = 2π·s, φ = π·(1 − t))", the full pole-to-pole span over `t ∈ [0,1]`.
- **Silence:** R-14's map puts a pole at each end of the chart's `[0,1]` span on φ, so R-407's pole-crossing and
  R-408's "pole to pole" primary range fit it. With the toggle drawing one hemisphere, the corpus says neither (1)
  whether the chart's `[0,1]` span on φ is remapped to run from a pole to the equator, or the full pole-to-pole span is
  kept and the other hemisphere set aside (masked, or drawn and labelled), nor, if the span ends at the equator, (2)
  what the φ axis does past that edge, which is not a pole, and (3) what its primary range is there (the chart's `[0,1]`
  span, pole to equator, or pole to pole). Past the equator the formula's continuation shows the other hemisphere,
  which decodes to the same systems (R-141's fold), so a reflection there would draw the same pixels.
- **Options seen:**
  - (a) The span is remapped to end at the equator; φ stays pole-crossing at its pole edge, and past the equator its
    formula continues into the other hemisphere (the same systems as the mirror, R-141); its primary range is the
    chart's `[0,1]` span, pole to equator, so that continuation is not counted.
  - (b) As (a), but the equator edge is a bounded edge for this chart: past it the pixels are hatched as the domain's
    end, out of the count and the total.
  - (c) The toggle does not change `Φ`: the chart's `[0,1]` span stays pole to pole, one hemisphere is drawn by setting
    the other aside, and R-407's and R-408's rules hold as for both hemispheres.
- **Applied meanwhile:** with both hemispheres drawn, R-407 and R-408 apply as written. Statistics agree under (a) and
  (b) (the pixels past the equator are out of the count and the total either way); only what is drawn there differs,
  and under (c) the primary range is pole to pole.
- **Waits:** only TASK-M8-44, a leaf task (nothing depends on it) that builds and checks the shape sphere's extension
  and counting with one hemisphere drawn; its REQ-CHART-057 carries `rq: RQ-264`. TASK-M8-05 runs with both
  hemispheres and does not wait.
