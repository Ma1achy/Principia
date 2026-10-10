# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-264: with the shape sphere's hemisphere toggle drawing one hemisphere, what the pole-crossing φ does past the edge that is not a pole, and what its primary range is, is not given *(definition, physics, R-408, R-407, R-113, R-141, TASK-M8-44, REQ-CHART-057; context: TASK-M8-05)*

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

---

## RQ-265: the debug views and colouring §3.6 give the `log` and `diverging` scales different forms, and §3.6's `log` needs `lo > 0` where the ledger declares `lo = 0` *(conflict, R-381, R-401, R-403, TASK-M7-05, REQ-COL-032, REQ-COL-039, REQ-COL-065)*

*Found in the R-388 pre-flight of TASK-M7-05 (10 Oct 2026). Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_colouring.md` § "3.6 Compaction (payload scalar → b ∈ [0,1]; forms per ledger `scale`)":
    "log        b = clamp( (ln x − ln lo)/(ln hi − ln lo) ),  x ≤ 0 → 0 with sentinel styling" and "diverging  b = ½ +
    ½·sign(x)·ln(1+|x|/x₀)/ln(1+x_max/x₀)   ★ symlog — PIN, confirm/veto".
  - `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug
    view)": the drift views' "`symlog`, the default: the compacted value takes `dbg_sentinel`'s place `t = 0.5 +
    0.5·x/(1 + |x|)`", the compacted value being R-381's "`symlog`, the signed log through the midpoint with the
    field's floor `eps_E` or `eps_L` (RQ-206's option 1, `x ↦ sign(x)·ln(1 + |x|/ε)`)" (decisions.md § "R-381"); and
    "`dbg_log(x, eps)`: `s = ln(1 + |x|/eps)`, then `ramp_viridis(1 − 1/(1 + s))`".
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library": "`raw` is the field's value compacted
    per its ledger scale: `lin` and `diverging` the identity; `log` `1 − 1/(1 + ln(1 + |x|/ε))`, `dbg_log`'s place, on
    the fixed `[0, 1]`".
  - `docs/design/principia_dd_colouring.md` § "2. Consolidated contract" (line 17): "colour **compaction** is the third
    compactification role — render-key, free, never re-integrates; per-field scale comes from the **ledger metadata**
    (lin | log | cyclic | diverging | categorical | flag)". REQ-COL-032 (sources §2 and § "4. Seams (obligations →
    integration tests)", seam 5) asks that the view re-style from that scale.
  - The ledger declares the log fields with `lo = 0` and an unbounded `hi`: e.g. `closure_min`,
    `.range(from_zero)` (`crates/ledger/src/payload.rs`:135 at `c683714`).
- **Conflict:** one ledger `scale` names two different maps. For `log`, the debug template places `1 − 1/(1 + ln(1 +
  |x|/ε_f))` with a per-field floor (R-401) and needs no range; §3.6 places `(ln x − ln lo)/(ln hi − ln lo)`, which
  needs `0 < lo < hi < ∞`, and the ledger gives `lo = 0` and no finite `hi`. For `diverging`, the drift views (R-381's `symlog`)
  compact `s = sign(x)·ln(1 + |x|/ε)`, with `x₀ = ε` the field's floor (`eps_E` or `eps_L`), and place `t = 0.5 +
  0.5·s/(1 + |s|)`; §3.6 places `½ + ½·sign(x)·ln(1 + |x|/x₀)/ln(1 + x_max/x₀)`. The two share `x₀` and differ only in
  the normalisation: `1 + |s|` against `ln(1 + x_max/x₀)`, so the drift views need no `x_max`; and the template places `E_0` and `Lz_0` (`diverging`) by the identity on `[−M, M]` (R-400). REQ-COL-032
  asks that compaction take each field's scale from the ledger, so a view and a ramp built on the same field disagree on
  where a value sits.
- **Options seen:**
  - (a) The debug views keep their own placement forms (render contract presentation layer, render_gui_spec §10.1),
    distinct from the user-facing §3.6 compaction; the ledger `scale` names the form family, and each layer states its
    own map. §3.6's `log` then needs its `lo` from somewhere other than the ledger range (a declared floor, or a
    measured end).
  - (b) Both layers use §3.6's forms: the debug template's `log` and the drift views' `symlog` move to §3.6's, which
    changes R-381's and R-401's views and their goldens, and §3.6's `log` still needs a finite positive `lo` and `hi`.
  - (c) §3.6 adopts the debug layer's range-free forms: `log` becomes `1 − 1/(1 + ln(1 + |x|/ε_f))` with R-401's
    per-field floor (or the field's `floor?`, R-263), and `diverging` uses `x₀` as that floor with no `x_max` (`b = ½ +
    ½·sign(x)·(1 − 1/(1 + ln(1 + |x|/x₀)))`), so one form per scale serves both layers. This `diverging` form is
    algebraically the drift views' own: with `s = sign(x)·ln(1 + |x|/x₀)`, `s/(1 + |s|) = sign(x)·(1 − 1/(1 + |s|))`,
    so under (c) R-381's `symlog` view is unchanged.
- **Applied meanwhile:** nothing. The debug views stand as built (R-381, R-401).
- **Waits:** all of TASK-M7-05, not single lines (`plan/WORKFLOW.md` § "Escalation": "The task stays open, blocked,
  until the human rules"), and through it its dependents TASK-M7-08, TASK-M7-09 and TASK-M7-23, and every task
  downstream of them in `plan/tasks.yaml`. Three of its requirements carry `rq: RQ-265`: REQ-COL-032
  (`ledger_scale_restyle`, the scale the view re-styles from), REQ-COL-039 (`compaction_forms`: §3.6's `log` cannot be
  built as written for the ledger's log fields, whose `lo` is 0, and the forms are what the ruling picks) and
  REQ-COL-065 (the `diverging` form's rules: whether `x_max` exists at all is what (c) decides). TASK-M7-05 is also held
  by the M1 gate (and the gates before M7).

---

## RQ-266: what `class_histogram` does when a setting's footprint count per quad, N² × (E+1), exceeds its bin width, is not given *(silence, physics, R-398, R-137, R-132, TASK-M5-01, REQ-PAY-075, REQ-PAY-077, REQ-PAY-089)*

*Found in physics review 5480220637 of PR #185 (the R-388 pre-flight of TASK-M5-01). Nothing is chosen.*

- **File, section:**
  - `docs/design/principia_dd_generation_root.md` § "Outcome (all at joint `class ⊕ detail` grain)": "`class_histogram[N]`
    | u8 × N | derives dominant *and* impurity from **one** source, so they cannot disagree", and "`outcome_impurity` |
    f16 | `1 − max(class fraction)`".
  - `docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors": "**Uniformity
    invariant — every footprint carries `E+1` copies at all times; no copy is ever removed from the sample.**"
  - `docs/design/principia_memory_tiers.md` § "4. The six quality tiers": "**Ultra and Extreme cap `N` at 16** (`N² ≤
    256`, the one-workgroup-per-quad invocation ceiling)", Extreme's `E` = 15 "*(provisional, R-137)*", and Medium's
    `N` = 16 with `E` = 1.
  - `docs/design/principia_memory_tiers.md` § "5. Controller levers, ranked by impact": "**Custom mode** exposes
    `render_scale` (0.25–2.0; …), `N`, `MAX_REL_DEPTH`, E, and FTLE directly (arbiter off)", and "**Custom's `N` is a
    power of two (R-398)**, each within the one-workgroup-per-quad thread ceiling (`N² ≤`
    `maxComputeInvocationsPerWorkgroup`, REQ-PERF-011)", so `N` = 32 on a 1024-invocation adapter, and `E` has no bound.
  - `docs/design/principia_memory_tiers.md` § "7. The "are you sure?" safety system (three severities, none blocking)":
    "**Red does not prevent** — … the user has final say".
  - REQ-PAY-075 (TASK-M5-01's definition): "the bin width cannot overflow at the largest footprint count per quad".
- **Silence:** a bin `w` bits wide holds at most `2^w − 1`. §3.7's u8 overflows already at Medium (16² × 2 = 512); at
  Extreme the count is 16² × 16 = 4096; in Custom, with `E` unbounded and `N` up to 32, no fixed width covers every
  setting. The corpus does not say what happens past the width. A bin that wraps silently corrupts `dominant_outcome`
  and `outcome_impurity`, which feed the split decision, so the choice changes results.
- **Options seen:**
  - (a) A fixed bin width chosen so that no setting that can allocate reaches it (e.g. u32 bins: at `N` = 32,
    `E + 1` would have to exceed 2^32/1024 ≈ 4.2 M copies), with the bound stated and checked.
  - (b) The bin width is chosen from the setting (`⌈log₂(N²(E+1) + 1)⌉`, rounded to a packable width), so the
    `QuadReduction` layout follows the sim key's `N` and `E`.
  - (c) A setting with `N²(E+1) > 2^w − 1` is refused or clamped (Custom's `E` capped, and the internal rungs kept
    under it). Against it, §7's "none blocking" for Custom; for it, the corpus's precedent of refusing a configuration
    that exceeds a stored format: `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)",
    `t_end_step`'s row, "**Dispatch refuses a configuration with `horizon_steps=⌈T/dt⌉ > 65535`** (R-86; single format,
    no Q0.16 fallback; long integrations use coarser dt/epochs)".
  - (d) Bins saturate at `2^w − 1` and the quad is flagged (its `outcome_impurity` marked untrustworthy), so a
    saturated quad is never read as a clean one.
- **Applied meanwhile:** nothing.
- **Waits:** all of TASK-M5-01, not single lines (`plan/WORKFLOW.md` § "Escalation": "The task stays open, blocked,
  until the human rules"), and through it its dependent TASK-M5-17, and every task downstream of it in
  `plan/tasks.yaml`. Three of its requirements carry `rq: RQ-266`: REQ-PAY-075 (the definition's overflow clause, and
  `quad_reduction_histogram_capacity` at Extreme's 4096 and at the boundary the ruling sets), and, because under (b)
  the layout follows the setting, REQ-PAY-077 and REQ-PAY-089 (member order, packing, aligned size). TASK-M5-01 is
  also held by the M1 gate (and the gates before M5).
