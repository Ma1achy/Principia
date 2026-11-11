# M2 chart re-plan, addition: morph frames and the new chart edges against traces, contours, placement and saving

**Status: proposed, 11 Nov 2026.** This document plans; it builds nothing and dispatches nothing. It is the third of the
chart plans in `plan/charts/`, after the re-plan (`plan/charts/m2-chart-system-replan.md`, merged from PR #473) and its
addendum (`plan/charts/m2-chart-addendum-extensions-axes.md`, merged from PR #476). It does two things. It plans the
four things those two don't cover, traces, contours, one mechanism for "where is this state on this chart", and a saved
mid-morph frame, and it applies R-944 (the owner's principle: a morph frame is a chart, and a stated mid-morph
limitation is a gap to close) to every place the design said a mid-morph frame can't do something. And its PR is the one
place the three plans' tasks enter `plan/tasks.yaml`: § F lists them, N1 to N19 and one measurement task, for that PR's
last commits to enter. R-923 (integrator work before charts, for now) still holds the chart tasks: § F.6.

**Sources.** The owner's words, verbatim in `decisions.md`: R-944 (the principle), R-945 (the backwards direction's list
was not exhaustive), and R-947 to R-952 (their reply of 10 Nov 2026 to the draft of this document). The two merged
plans, cited by section. The design as it stands on this branch, where the same PR's earlier commits brought
`docs/design/principia_chart_composition.md` and `docs/design/principia_lines.md` into line under those rulings; this
document cites those sections and never restates a decision they don't hold. Line numbers aren't cited: sections and ids
are.

**Every number here that is not in a ruling is a fixture size or a criterion written before a run (R-649, targets are
measurements, never gates).** No threshold and no fallback is chosen in this document. Where the draft proposed one that
no ruling adopts, it is kept in § A.5 as a labelled proposal that nothing adopts, with the measurement that would inform
it.

## The owner's answers of 10 Nov 2026 (R-947 to R-952), and the decisions listed for their veto

**Status, 11 Nov 2026.** The draft of this document put eight questions to the owner, lettered (A) to (H). They were
asked in the draft itself and never entered `REVIEW_QUEUE.md`, so no queue entry closes. The owner answered four as
rulings, and gave the orchestrator the other four and one more to decide under R-369 (the orchestrator's two-way
decisions), listed for their veto. They added two things before the trace tasks are built, each a ruling. One new queue
entry is open, RQ-785 (at what picture size P2's and P4's pixels are counted).

| Question | Answer | Ruling or decision |
|---|---|---|
| (A) Where a running or saved state is on a mid-morph frame, and on a chart with no inverse | Option (a): one placement function, two routes chosen by the chart, the existing reading where a chart has an inverse and the nearest point everywhere else, every blend included, refined to the lock's precision on the CPU | R-947 (placement) |
| (B) A trace at an unfold line, and in a clamped or saturated area | (i-a), (ii-a), (iii-a): a state's sheet is set by whether the system is growing or shrinking; the trace jumps at the fold, with an optional fold tick, off by default; nothing is drawn inside a clamped or saturated area. Physics review confirms | R-948 (traces at the new edges) |
| (C) The contour variables `h`, `v`, `s`, `t` and the latent group on a mid-morph frame | Each end chart's own value, the expression drawn once per end at that end's share, so the grid is both ends' grids, fading | applied per R-369, listed for veto |
| (D) Marks and the symmetry filter on a mid-morph frame (R-899's Q8 (a) under R-944) | Marks found by the placement function to the lock's tolerance, the filter on. **Conditional on physics review; not settled until it confirms** | applied per R-369, listed for veto, conditional |
| (E) A hover trace while a morph plays | A run changed by the morph's `s`, not by the pointer, is not a new run and keeps its head's time; a pointer move still restarts | applied per R-369, listed for veto |
| (F) A pixel tagged at an end, under the state blend | A tagged end that carries a state blends as a state; only an end pixel with no state stays blank mid-path. **Conditional on physics review, tag by tag; not settled until it confirms** | applied per R-369, listed for veto, conditional |
| (G) A changed link id mid-morph | Option (a): blend, don't cut; R-520's rule for interpolating link parameters inside one chart stands | R-949 (a changed link id morphs) |
| (H) The lock's leftover jump from a time-axis chart to one with no time axis | Option (a): removed | R-950 (the lock's jump removed) |
| The unlocked pivot of a saved frame's tilt | Each end about its own centre | applied per R-369, listed for veto |
| Added by the owner: a measurement P4 beside P2 | P2 and P4 are reported, not gates, and shown to the owner before TASK-M8-49 (traces on the line system) and TASK-M8-58 (the trace's markers and leaving rule) are built; no threshold or fallback is picked by anyone else | R-951 (the two measurements) |
| Added by the owner: contours on the CPU | Agreed for every chart; the 8.3 ms criterion kept as written; its report says what the owner will see while moving on each of the three test charts | R-952 (contours on the CPU) |
| The draft's § C, RQ-771 (ii), the linear energy chart below its floor at a non-zero spin | Ruled before this document was filed: the rigid rotation at the pixel's own spin, physics review agreeing with conditions C1 to C7 | R-937 (the second sheet); § C |

**The owner's go-ahead for (C), (D), (E), (F) and the pivot** (their reply of 10 Nov 2026, verbatim, under their heading
"THE FOUR YOU DECIDE"):

> Go ahead with (C), (D), (E) and (F) as you proposed, listed for my veto. The unlocked pivot for a
> saved frame (each end about its own centre) is fine too.

That is a go-ahead to decide them, not a ruling on any of them. Each is recorded below as the orchestrator's decision,
in the form R-369 asks for, and each stays open to the owner's veto.

- **applied per R-369: (C),** a Chart or Latent contour variable on a mid-morph frame has one value per end chart and is
  drawn once per end at that end's share. § A.2.
- **applied per R-369: (D),** on a mid-morph frame marks are found to the lock's tolerance and the symmetry filter is
  on. Conditional on physics review, which must confirm the mechanism and the definition of the canonical representative
  on a blend; the condition is written into TASK-M2-84 (the symmetry filter's maths) and, for the marks' standard,
  TASK-M2-80 (points of interest's maths). Until physics confirms, R-899's Q8 (a) is what a test asserts. § B, item B6.
- **applied per R-369: (E),** a change of the hovered system caused by a playing morph does not restart the trace. §
  A.1.
- **applied per R-369: (F),** an end pixel that is tagged and carries a state blends as a state. Conditional on physics
  review, tag by tag for the decoder's five tags; the condition is written into TASK-M2-81 (path re-timing and frame
  flags). Until physics confirms, V21 (a pixel blank or tagged at either end is blank mid-path) stands as written. § B,
  item B1.
- **applied per R-369: the unlocked pivot,** without a lock a tilt or slice of a saved frame turns each end about its
  own centre. § A.4.

**Further decisions of the orchestrator, applied per R-369 and listed for veto.** Each is a record correction, a
wording, a mechanical consequence of a ruling named beside it, or task scoping. None changes what a ruling says.

- **applied per R-369: B2,** no blank opens mid-path under the state blend (a record correction; R-926 item 3, R-943).
  One case is left to physics review in N5.
- **applied per R-369: B4,** a `gauge` end's paper scale is drawn on its own side of the dual ruler (a consequence of
  R-940).
- **applied per R-369: B7,** a pan or zoom on a blend re-addresses the cache as on any chart (a consequence of R-935's
  answer to RQ-770).
- **applied per R-369: B8,** the "CPU batch" label and the GPU blend entry are moot (a record correction; R-926 items 5
  and 6).
- **applied per R-369: B11,** two time windows blend like any two windows, and V29 is retired (a consequence of R-926
  item 3).
- **applied per R-369: B15,** a blend's well-posedness stated in words (wording; R-943's three conditions).
- **applied per R-369: B16,** the chart-space model's summary corrected by a dated note (a record correction).
- **applied per R-369: B17,** where a blend repeats, stated (a consequence: a blend's pixel shows the same system again
  only where both ends do).
- **applied per R-369: task scoping.** All of N1 to N19 enter `plan/tasks.yaml` in this document's PR, as the two merged
  plans say; the placement function, N19, takes the CPU half of TASK-M7-52 (the one projection) into M2; N14 (per-edge
  extensions) is split by declared kind; P2 and P4 live in a new M2 measurement task, N20 here; N5, N7 and N19 stay one
  task each. § F.
- **applied per R-369: the held-`E*` measurement** of the re-plan's § B.3 item 11 is kept as written, with a dated note
  there saying what it measures. § E, item 14.

## 0. Ids used here, each with a plain-English gloss (R-693)

**Rulings.** R-71, a missing value is a calibration requirement; R-72, a missing definition is written by the task that
needs it, under physics review; R-92, which view edits change the cached simulation; R-186, timings run on the owner's
Mac; R-369, the orchestrator's two-way decisions, listed for veto; R-407, how an axis extends past its range, a new kind
needing physics review (its clause 3); R-467, a chart failing validity draws "not a system"; R-474, the two tilt maps;
R-476, the line system (contours and traces) adopted; R-483, a keyframe holds its links, a changed link id a cut (its
X7); R-520, link parameters interpolate only in a blocking export; R-538, the trace's markers and where a trace can
leave (TF1), read per shown axis (TF2); R-542, the state map's "Chart position" colouring and its one projection (H11);
R-569, R-616 and R-617, when a trace restarts at `t = 0`; R-583, one size rule, base px; R-623, a segment offers cut or
interpolate; R-649, targets are measurements, never gates, a criterion written before the run; R-669, a user's picture
is never scored; R-693, every id glossed; R-849, under a lock the anchor slides; R-850, one path abstraction for every
view edit; R-854, points kept as signed states; R-899, the owner's answers on the chart-path brief (Q8: marks mid-morph
"to pixel resolution", the symmetry filter off); R-901, easing per path segment; R-902, the lock is the path's, and a
mid-morph tilt goes to both ends; R-916, M2 is the whole non-visual chart system; R-917, build order charts, integrator,
visuals; R-919, the owner's answers on the chart plan (F18: the trajectory fixtures); R-923, integrator work before
charts, for now, no new chart task meanwhile; R-925, the owner's six chart decisions of 10 Nov; R-926, the owner's
eleven chart-system decisions of 10 Nov; R-931, a provably wrong golden is fixed; R-932, chart edges, no blank areas,
the kinds; R-933, bodies starting collided are a collision at `t = 0`; R-934, the momentum seed `w = r/√I`; R-935, the
re-plan's eleven answers; R-936, what a state blend leaves open; R-937, the invariant charts' second sheet and the clamp
below the floor; R-938, every chart's edges are built now; R-939, marking clamped and saturated areas, a toggle; R-940,
the dual ruler; R-941, each edge's default per R-932, the rest confirmed by eye; R-942, the addendum's five remaining
answers; R-943, the general rule for a morph; R-944, the principle, a morph frame is a chart; R-945, the backwards
direction serves anything that places a state on a chart; R-947 to R-952, the owner's answers on this document, glossed
in the table above.

**Review queue.** RQ-759, what the lock's re-solve guarantees; RQ-760, what a state blend leaves open; RQ-764, the
momentum seed; RQ-765, what the GPU's decode entries become; RQ-767, how accurate the backwards table must be; RQ-770,
whether pan and zoom keep the cache; RQ-771, the energy chart's second sheet and the clamp below its floor; RQ-775,
where the per-edge choice lives; RQ-776, the backwards table on a two-sheet chart; RQ-779, each edge's default kind,
still open; RQ-781, where the view's centre may go; RQ-783, whether a record made before the new decode route recreates
bit for bit, still open; RQ-784, whether "No blank areas" reaches a pixel energy normalisation cannot reach, still open;
RQ-785, at what picture size P2's and P4's pixels are counted, new and open.

**Tasks and requirements** are glossed where they first appear in a section, and in § F's tables. **N1 to N11** are the
re-plan's proposed tasks, **N12 to N18** the addendum's, **N19** (the one placement function) and **N20** (the
measurement task for P2 and P4) this document's. None has a `TASK-` id until this PR's last commits enter it.

## 0.1 The owner's principle, verbatim (R-944)

> THE PRINCIPLE. Every pixel of a morph frame is a valid system, so a morph frame is a chart and every
> feature that works on a chart works on a morph frame: traces, contours, marks, hover, tilt, slice, save.
> Where the design says a mid-morph frame can't do something, treat that as a gap to close, not a
> limitation to label. The only real exception I've accepted is area statistics on a mid-morph frame.

So "not drawn mid-morph" is never an item of this plan. The one exception is area statistics (R-936, what a state blend
leaves open: no area statistic on a mid-morph frame, each end keeping its own).

---

## A. The four items

Each item is a plan item: what is built, by which task, and its acceptance, with every criterion written now, before any
run (R-649, targets are measurements, never gates). The design text each item rests on is already in the design; the
section is named.

### A.1 Traces (hover, pinned and kept), on any chart, a blend included, and at the new edges

**Ruled by** R-947 (placement) and R-948 (traces at the new edges); decided per R-369 (the orchestrator's two-way
decisions) for (E). **Design:** `docs/design/principia_lines.md` § "8. Traces" (the bullets "On a mid-morph frame" and
"At the new edges", the markers table's Fold row, and the note under "Animation");
`docs/design/principia_trajectory_viewing.md` § "3. Hover trace — projection onto the current (tilted, sliced) plane" (a
dated note).

**What is built.**

1. **A trace on a blend.** Each trace sample is placed by N19's placement function (§ A.3). On a blend that is its
   nearest-point route, seeded by the previous sample's place, or by the backwards table where there is no previous one
   or the solve fails from it. The first vertex of a hover trace is the hovered pixel itself. The fade is the route's
   residual, in place of the hidden coordinates' distance. A sample leaves where the solve reports failure, with the
   rings. At `s = 0` and `s = 1`, selected exactly (REQ-CHART-113, a path's ends selected exactly), the trace is the end
   chart's own, bitwise. Where the frame folds and more than one pixel shows the state within the lock's tolerance
   (REQ-CHART-158, the lock's tolerance, a calibration), the trace takes the one nearest its previous sample.
2. **A hover trace while a morph plays** (applied per R-369: (E)). The hovered pixel's system changes with the morph's
   `s`, so each frame integrates another system. A run changed by the morph and not by the pointer is not a new run for
   R-569's rule (a new trace while playing restarts at `t = 0`): it keeps its head's time. A pointer move still
   restarts. Pinned and kept traces are fixed systems, re-placed each frame.
3. **At an unfold** (R-948 (i-a) and (ii-a)). A sample is placed on the sheet its own state lies on: the sign of the
   state's momentum along the construction's seed, which under R-934's seed `w = r/√I` is the sign of the dilation rate,
   sheet 1 while the system grows and sheet 2 while it shrinks. Read per shown axis, the trace keeps its `L_z` and its
   place reflects across the fold line, `s ↦ 2 − s`. Away from the fold that is a jump: the line breaks, and a fold
   tick, the seam tick's twin, marks both ends, optional and off by default. Where the sheet changes on the fold the
   line runs on.
4. **At a clamp or a saturate** (R-948 (iii-a)). A sample is never placed inside the area: a real state is placed at its
   own point inside the primary range, or it leaves by lines § 8's rule, with the rings.
5. **At a wrap:** as a periodic axis today, in every image, with the seam tick.

**Physics review confirms** the sheet reading and the clamp and saturate rule (R-948: "Physics review confirms"). They
are physics definitions (R-72) under R-407's clause 3. The confirmation is the physics review of this document's PR and
of the task that builds each; a disagreement goes back to the owner.

**Tasks.**

- **N19 (new, § A.3):** the placement function, returning the sheet.
- **TASK-M8-49 (traces on the line system), change:** every trace placed through N19 on every chart; its acceptance
  gains `line_traces_blend`. It is not built until the owner has seen P2 and P4 (R-951; § A.5).
- **TASK-M8-58 (the trace's markers and leaving rule), change:** the fold tick, the sheet in the per-axis placement,
  "never inside a clamp or saturate area", the leaving rule on a blend (a failed solve) and the branch rule at a blend's
  fold; its acceptance gains `trace_fold`. REQ-GUI-272 (the trace's markers and icon sets) gains the fold tick. Not
  built until the owner has seen P2 and P4. The addendum's § A.8 marks this task "KEEP, Unaffected"; under R-948 it
  changes (§ E, item 10).
- **TASK-M8-09 (the hover integration), change:** `computeIC` takes a blend chart, its IC the blend's CPU decode at the
  hovered pixel (N5).
- **TASK-M8-10 (the hover trace on the figure), change:** calls N19; the playing-morph rule of item 2.
- **N5 (the re-plan's blend chart):** exposes the blend's f64 forward map and decode to N19 and to `computeIC`. **N13
  (the addendum's second sheet):** exposes a state's sheet, the sign of its seed component, from one source with the
  construction.

**Acceptance, criteria written now.**

- `line_traces_blend` (TASK-M8-49), gating. On the two blend pairs, the latent chart to the shape sphere's exponential
  map, and the `(L_z, E)` chart to the shape sphere's exponential map, at `s = 0.25`, `0.5` and `0.75`: a hover trace's
  first vertex is the hovered pixel exactly; every vertex whose state is a pixel's own state (the `t = 0` state of a
  pinned trace) is placed at that pixel with its residual within the lock's tolerance (REQ-CHART-158); at `s = 0.0` and
  `1.0` every vertex equals the end chart's own trace bitwise. Controls, each of which must fail: a trace placed by
  chart A's encode reading at `s = 0.5` misses the first-vertex check; a trace at `s = 0.0` read by the arithmetic blend
  of the ends at `s = 10⁻⁶` fails the bitwise check. **This test checks only points on the surface.** What a trace does
  off it, which is almost every vertex, is measured by P2 and P4, not asserted (R-951).
- `trace_fold` (TASK-M8-58), gating. On `InvariantLE` (the `(L_z, E)` chart) unfolded at the register's defaults, a
  fixture trajectory with at least two turning points of `I`: every vertex's sheet equals the sign of its own `İ`;
  across each turning point the two vertices' places satisfy `s₂ = 2 − s₁` where `L_z` is unchanged; a fold tick at both
  ends of each jump when the option is on. Controls: the sheet read from the sign of `L_z` fails the first check; a
  trace joined across the jump fails the break check. The tolerance of the `s₂ = 2 − s₁` comparison is not given by the
  corpus: a calibration requirement (R-71) on TASK-M8-58, proposed with its evidence by the task and never chosen here
  (the draft's figure is in § A.5's list of proposals nothing adopts).

### A.2 Contours, on the CPU for every chart

**Ruled by** R-952 (contours on the CPU, the 8.3 ms criterion kept); decided per R-369 for (C). **Design:**
`docs/design/principia_lines.md` § "4.1 The expression" (the paragraph "On a mid-morph frame") and § "4.2 Evaluation"
(the Starting-state bullet's dated notes); `docs/design/principia_chart_composition.md` § "10.2 Tier 1 overlays" (a
dated note).

**What is built.**

1. **Where a starting-state contour is evaluated.** On the chart's decode, on a grid at screen resolution, independent
   of the quadtree, as before. The decode is the CPU's f64 evaluation of the chart, the evaluator that fills the quads'
   forward tables, run on the screen grid; the expression is evaluated on the decoded state on the CPU. It is one route
   for every chart, a chart with no shader code and a blend included, with no reduced resolution forced by a chart's
   kind (R-952, following R-926 item 5: the CPU evaluates charts in f64). Playhead contours, read from the resident
   quads' payload, are unchanged. No GPU contour pass is planned: under R-935's answer to RQ-765 (what the GPU's decode
   entries become) those entries stay as test entries only.
2. **The Chart and Latent variables on a blend** (applied per R-369: (C)). A Chart variable (`h`, `v`, `s`, `t`) or a
   Latent variable has one value per end chart, the pixel's place in that end's own coordinates. An expression that
   names one is drawn once per end, that end's values substituted, at that end's share of opacity (`1 − s` and `s`). So
   the Grid preset draws each end's own grid at its own ruler's ticks (R-940, the dual ruler). An expression that names
   neither is drawn once, on the blend's own state. At `s = 0` and `1`, selected exactly, only that end's contours are
   drawn, bitwise as on the end chart. The share-weighted sum of the two ends' values is not used: it is the summed
   number R-940 rejects.

**Tasks.**

- **TASK-M7-43 (contours), change:** `contour_eval` evaluates on the CPU's f64 screen grid; the per-end evaluation of
  Chart and Latent variables on a blend. Its dependence on TASK-M2-25 (the generated WGSL decode) is dropped; it depends
  on TASK-M2-67 (the CPU evaluator) and N5 (the blend chart). REQ-RENDER-093 (where contours are evaluated) and
  REQ-RENDER-092 (the contour expression's variables) change. It carries R-952's timing, below, so it gains a benchmark.
- **TASK-M7-72 (contours of any registered primitive), change, small:** each registered forward is evaluated on the
  CPU's decoded state; REQ-RENDER-121's verify reads so. Its control, an angle off by 1°, stays.
- **TASK-M8-50 (Lines ▾, the axes and the Grid preset), change:** the Grid preset on a blend draws both ends' grids,
  reading N16's dual ruler (the addendum's computed axes), so one source gives the ruler's ticks and the grid's.

**Acceptance, criteria written now.**

- `contour_eval` (TASK-M7-43), gating: its control stays (a contour read from the quads' payload moves with refinement).
  Its depth-30 check against "the CPU f64 decode's level set" becomes an identity once the contour is itself the CPU's
  f64 evaluation, so it gains a check at a depth where f64 itself rounds, reported with the level reached and never
  asserted against a fixed depth (as the re-plan's open item O8 reports the f32 floor).
- `contour_blend_ends` (TASK-M7-43), gating: at `s = 0.0` and `1.0` every contour set's polylines equal the end chart's
  bitwise; at `s = 0.5` a contour of `h` drawn for end A lies on A's ruler ticks and for end B on B's (lines § 12's
  exactness test); opacity `1 − s` and `s`. Controls: `h + 0.001` per end misses the ticks; an `h` evaluated as the
  summed axis value misses both ends' ticks.
- **The timing, R-952's criterion, kept as written and not adjusted after the run.** On the Mac (R-186): the
  starting-state contour pass, the CPU's f64 evaluation, the expression and the marching squares together, on all cores,
  the slowest of five warm runs, on three test charts: the latent chart, the shape sphere's exponential map, and a blend
  of the two at `s = 0.5`. PASS if the half-resolution grid, 960 × 540, takes at most 8.3 ms, half a 60 Hz frame, in
  motion. If it misses, the in-motion grid steps down to quarter resolution, lines § 4.2's own degrade-in-motion rule,
  recorded with the numbers. At rest the full grid is reported, not judged.
- **The report says what the owner will see.** For each of the three test charts the report says plainly what the owner
  will see while moving: contours at half resolution, or at quarter resolution (R-952). R-952 gives no rule for a
  quarter-resolution grid that also misses 8.3 ms: if that happens the report says so with the numbers and it is the
  owner's to decide, with nothing stepped down further meanwhile.

### A.3 One mechanism for "where is this state on this chart": N19

**Ruled by** R-947 (placement), read with R-945 (the backwards direction serves anything that places a running or saved
state on a chart). **Design:** `docs/design/principia_chart_composition.md` § "10.6 Placement: one function for "where
is this state on this chart"", new, and § "10.5 Points of interest" (a dated note: this is the definition it owed).

**What is built: N19, the one placement function.** `place(state, chart) → {point, sheet, residual, route, status}` is
one Rust source in `crates/kernel`, beside the encode, as R-542's H11 has the one projection, and every consumer calls
it, never a second implementation. Its input is a signed state (R-854); its chart is any chart, a blend or a saved blend
included.

- **Two routes, chosen by the chart, never by the consumer.** *The encode reading,* where the chart has a closed-form or
  registered numerical inverse: the encode, read on the shown axes, the hidden coordinates giving the off-slice
  distance. It is R-542 H11's one projection and R-538 TF2's per-axis reading, unchanged on every chart that has it.
  *The nearest point,* everywhere else, every blend included: the chart point whose decoded state is nearest the state,
  found by the lock's solver (N11) on the chart's own f64 forward map, seeded by the caller's hint (a trace's previous
  sample, a lock's previous centre) or by the backwards table (N7). Its residual is the off-slice distance.
- **Refined to the lock's precision on the CPU.** On the CPU every route refines to the lock's tolerance
  (REQ-CHART-158), mid-morph included. R-926 item 7's "Approximate is acceptable during a morph" binds the fragment's
  table route only (R-947), which N9 (the fragment table route) measures against this function.
- **What it returns.** `status` is `placed`, `left` (the leaving rule, or a failed solve) or `ill-conditioned` (below
  the floor R-935 has under RQ-767 (a)). On a chart with a second sheet, `sheet` names it. Where a blend folds and more
  than one point lies within the lock's tolerance, every such point is returned and the consumer applies its own rule: a
  trace takes the one nearest its previous sample, a mark draws a star at each, the lock follows its local minimiser
  (R-935, under RQ-759 (a)). At `s = 0` and `s = 1`, selected exactly, the end chart's own route answers, bitwise.
- **Two definitions are still owed (R-72), and N19's task writes them under physics review, the physics reviewer
  approving each.** *The metric "nearest" is measured in:* design § 10.6 says where it starts (REQ-CHART-124's
  mass-weighted metric on the `I = 1` state); R-947 fixes that there is one function and which route a chart takes, not
  the metric's weights. *Which nearest point answers for a state off the chart's image:* as R-935 has it under RQ-767
  (a). Neither is chosen here. N19 carries a definition requirement for the two, and REQ-GUI-427 (the definitions points
  of interest owe) reads its "nearest chart point" and its off-slice metric from § 10.6 once they are written.
- **Not taken** (R-947): a blend's trace placed by the share-weighted mix of the two ends' readings; the nearest point
  on every chart; a mechanism per consumer.

**Consumers.**

| Consumer | Under N19 |
|---|---|
| Chart position and the state map (TASK-M7-47; the re-plan's N9) | unchanged in kind; the fragment's generated code and its table route are held to `place` on the CPU |
| Hover, pinned and kept traces (TASK-M8-10, TASK-M8-49, TASK-M8-58) | `place`, the route by chart; on a blend the nearest-point route (§ A.1) |
| Points of interest's marks (TASK-M2-80, TASK-M8-126) | `place`'s point and residual; mid-morph to the lock's tolerance under (D), conditional on physics review |
| The symmetry filter's partner check (TASK-M2-84, TASK-M8-120) | the partner is `place(g·D(φ(u)), chart)`; on a blend under (D), conditional on physics review |
| The lock's re-solve (the re-plan's N11, TASK-M2-65) | N11 is the nearest-point route's solver; the lock's own rules stay R-935's |
| The decode and round-trip presets | unchanged; the table route where a chart has no closed form (R-925) |
| The time-axis IC Inspector's "off this chart" (design § 18.4) | `place`'s status |

**Tasks.**

- **N19, new, M2, non-visual (R-916).** Slot: the re-plan's § B.4, after N5 (the blend chart), N7 (the backwards table
  on the CPU), N11 (the lock's solver core), N13 (the second sheet, for `sheet`) and TASK-M2-67 (the CPU evaluator), and
  before TASK-M2-80 and TASK-M2-84. It takes TASK-M7-52's CPU source into M2 and widens it (applied per R-369: task
  scoping; R-542 H11's "never a second implementation" is kept).
- **TASK-M7-52 (the one projection), change:** its scope narrows to the fragment's generated half of N19, the WGSL
  generation and its native-against-WGSL test; it depends on N19.
- **TASK-M2-80 (points of interest's maths), change:** the nearest chart point and the off-slice distance are `place`'s
  point and residual. REQ-GUI-427's clause "how marks are found on a path frame with no inverse" is defined by design §
  10.6. REQ-CHART-172 (points of interest's maths) changes under (D), with its condition.
- **TASK-M2-84 (the symmetry filter's maths) and TASK-M8-120 (the filter in the GUI), change:** the partner by `place`;
  on a blend under (D), with its condition.
- **TASK-M8-10, TASK-M8-49, TASK-M8-58, change:** call `place` (§ A.1).
- **N7 and N11 (the re-plan's):** both serve `place`; N7's scope is every consumer, not the fragment's list (R-945).

**Acceptance, criteria written now.**

- `place_routes` (N19), gating. For each kernel chart of the re-plan's § C (the latent chart, the shape sphere's
  exponential map, the `(L_z, E)` chart) and each of § A.1's two blend pairs at `s = 0.25`, `0.5` and `0.75`, on a 64 ×
  64 pixel grid: `place` of each pixel's own decoded state returns that pixel with its residual within the lock's
  tolerance (REQ-CHART-158), or, where the blend folds, a set containing it; at `s = 0.0` and `1.0` the end chart's own
  answer, bitwise; on a chart with an inverse the encode route's answer equals TASK-M7-52's one projection bitwise.
  Controls: a `place` that returns its table seed unrefined fails the residual check on at least one blend (if none
  fails, the table already meets the tolerance and the control is reported void, not passed); an answer one pixel off
  fails.
- The two definitions owed are written into design § 10.6 and approved by the physics reviewer before `place_routes` is
  read as passing on a nearest-point route.
- `place_routes` checks only states on the chart's image. What `place` does off it is P2's and P4's to report (§ A.5).

### A.4 A saved mid-morph frame: tilt, slice, and the unlocked pivot

**Ruled by** R-944 (the principle), which amends R-902's "§17.5 stands as written" for a saved frame; decided per R-369
for the unlocked pivot. **Design:** `docs/design/principia_chart_composition.md` § "17.5 A frame: valid, reproducible,
recorded", item 8 of its list, and § 13, where V20 (a saved mid frame reopens with no tilt or slice) is retired.

**What is built.** A saved mid frame is a frame record that holds both ends whole. Reopened, it is a blend chart like a
live one, at its saved `s`, and every chart feature works on it, area statistics excepted. A tilt or a slice of it goes
to both end charts, each in its own hidden coordinates and its own R-474 map (the two tilt maps), as R-902 has it for a
live morph: no hidden coordinate is invented for the blend. Under a lock the edit is about the pinned state. **Without a
lock it is about each end's own centre,** the pixel at the window's centre, as a tilt of that chart alone turns (applied
per R-369: the unlocked pivot). An end that can't take the tilt holds, and under a lock the anchor slides (R-849). The
tilted frame saves again as a record of the tilted ends at the same `s`. A saved frame may also be morphed on: a path
from it is a blend whose end is a blend (N5's nesting).

**Tasks.**

- **TASK-M2-65 (paths under a lock), change:** `path_mid_morph_tilt` gains a saved-frame arm, `saved_blend_tilt_slice`.
  REQ-CHART-161 (the lock across blends and the mid-morph tilt) reads "a tilt or slice mid-morph, on a live path or a
  saved frame reopened".
- **TASK-M2-81 (path re-timing and frame flags), change, small:** REQ-CHART-114 (a path frame's flags) carries no "no
  tilt or slice" label.
- **TASK-M8-48 (tilt, slice and Save view in the real GUI), change:** the tilt and slice controls are live on a reopened
  saved blend, not greyed with a label.
- **TASK-M2-61 (the chart-path object): keep.** Its record already stores both ends whole; it gains nothing.

**Acceptance, criterion written now.** `saved_blend_tilt_slice` (TASK-M2-65), gating: a frame saved at `s = 0.4` on the
chart pairs of REQ-CHART-161, reopened, then tilted by 20° and sliced by 0.1 of the slider span: its pixels' decoded
states equal bitwise the live morph's at `s = 0.4` after the same tilt and slice (the same leaf with the same inputs),
under a lock and without; without a lock each end has turned about its own centre; the tilted frame saved and reopened
again equals itself bitwise. Controls, each of which must fail: a tilt of the blended state's own encode (a hidden
coordinate invented for the blend); the reopened frame refusing the tilt; without a lock, both ends turned about one
shared point.

### A.5 The measurements: P2 and P4, reported and not gates; and the proposals nothing adopts

**Ruled by** R-951 (the two measurements). **Why:** the owner's words, verbatim, from their reply of 10 Nov 2026, item 1
under "TWO THINGS TO ADD BEFORE THE TRACE TASK IS BUILT":

> 1. A trajectory is almost always off the picture's surface. On a blend the nearest pixel to an
>    off-surface state can snap from one place to another, so a trace may break into pieces
>    mid-morph, and the whole trace may jump as a morph starts or ends. Your tests only check
>    points that are on the surface. Add a measurement next to P2, call it P4:
>    - on the same fixtures and blend pairs, at s = 0.25, 0.5 and 0.75, the share of neighbouring
>      trace vertices that land more than 8 pixels apart on the blend, set against the same
>      share on each end chart's own trace;
>    - also the median and worst residual (how far off the surface each vertex is).
>    P2 and P4 are reported, not gates. Show me both before TASK-M8-49 and TASK-M8-58 are built.
>    If the numbers are bad I will decide then; the likely fallback is fading the trace by its
>    residual, which your draft already has. Do not pick thresholds or fallbacks yourself.

**The set-up both share.**

- **The fixtures:** the trajectory fixtures of R-919's F18, built by TASK-M2-82 (the trajectory interface and its exact
  fixtures): the figure-eight and the Lagrange equilateral solution.
- **The blend pairs,** two: the latent chart to the shape sphere's exponential map, and the `(L_z, E)` chart to the
  shape sphere's exponential map.
- **16 seeded pixels per pair.** The seed is written into N20's task file before any run.
- **The placement:** N19's `place`, on the end charts by their own routes and on the blend by its nearest-point route.

**P2.** The on-screen jump between an end chart's own trace (`s = 0`) and the blend's (`s = 10⁻⁶`), median and worst, in
pixels per vertex.

**P4.** In the owner's words above: at `s = 0.25`, `0.5` and `0.75`, the share of neighbouring trace vertices that land
more than 8 pixels apart on the blend, set against the same share on each end chart's own trace; and the median and
worst residual, how far off the surface each vertex is.

**Reported, not gates.** Neither has a pass line, and none is written here. Each is a measurement in R-649's sense, its
numbers shown as measured. The owner sees both before TASK-M8-49 and TASK-M8-58 are built, and decides then if the
numbers are bad. **No threshold and no fallback is picked by the implementer, the orchestrator or a reviewer.** Fading
the trace by its residual is the fallback the owner names as likely; it is not adopted. (A trace on a blend does fade by
its residual in place of the hidden coordinates' distance, § A.1 item 1: that is the trace's ordinary fade, not a remedy
for a trace that breaks.)

**Where they live: N20, a new M2 measurement task** (applied per R-369: task scoping). It depends on N19 (the placement
function), N5 (the blend chart) and TASK-M2-82 (the trajectory fixtures). Its deliverable is the report, shown to the
owner through the rulings report. TASK-M8-49 and TASK-M8-58 depend on N20, and their task files say they are not built
until the owner has seen the report.

**The picture size waits on RQ-785.** "More than 8 pixels apart" and P2's "in pixels" name no picture size, and the
measurements run headless, with no window. That is a missing definition (R-72), the owner's to state: RQ-785 (at what
picture size P2's and P4's pixels are counted) asks it, and N20 carries a definition requirement for it. Nothing is
assumed meanwhile, and N20 does not run until RQ-785 is answered.

**What this document does not settle about the set-up, for N20's task file to state before any run and nothing here to
guess:** what a seeded pixel contributes (the draft has "over the trajectory fixtures … and 16 seeded pixels per chart
pair" and says no more), and whether P2 is also read at the other end, `s = 1` against `s = 1 − 10⁻⁶` (R-951 defines it
at `s = 0` only, and the owner's words have "as a morph starts or ends").

**Proposals nothing adopts.** Each was in the draft. None is a criterion of any task, and none is to be applied by an
implementer or a reviewer. Each names the measurement that would inform it.

| Proposal, as the draft had it | Status | The measurement that would inform it |
|---|---|---|
| **P3's pass line and degrade rule:** placing a 5,000-vertex trace on a blend frame within a quarter of a frame, and if it misses "every k-th vertex in motion, all at rest, k recorded" | Not adopted. A fallback for a slow trace is the owner's, as R-951 has it for a trace that breaks | The time to place a trace of a stated length on a blend frame, on the Mac (R-186), one thread, slowest of five warm runs, reported as a number with no pass line. No task carries it yet |
| **Fading a trace by its residual** as the remedy for a trace that breaks or jumps on a blend | Not adopted (R-951: the owner decides on the numbers) | P2 and P4 |
| **(a′), a blend's trace placed by the share-weighted mix of the two ends' readings,** if P2's worst jump is large | Not taken (R-947) | P2 |
| **The tolerance of `trace_fold`'s `s₂ = 2 − s₁`:** the draft had 4 ulps of `max(1, \|s\|)` | Not adopted: a calibration requirement (R-71) on TASK-M8-58 | The spread of `s₂ − (2 − s₁)` over the fixture's turning points, at f32 and f64, proposed with the task |
| **The canonical representative of a mirror pair on a blend:** the partner whose pixel comes first in the frame's row-major order | A definition owed (R-72), proposed in design § 10.4 for the physics reviewer to approve where TASK-M2-84 builds it; not settled | The filter's mask test on a blend of two mirror-declaring charts: a pair blanked only where the partner check passes; control, a blend that breaks the mirror blanks nothing |
| **A GPU contour pass on closed-form charts** as an optional speed path | Not planned: R-952 rules the CPU for every chart, and the GPU decode entries are test entries only (R-935, under RQ-765 (a)) | R-952's timing; if it steps down to quarter resolution, that is reported and the owner decides |

---

## B. The sweep: the 17 stated mid-morph limitations, each closed or kept (R-944)

R-944 (the principle: a morph frame is a chart) makes each place the design said a mid-morph frame can't do something a
gap to close. The draft found 17, B1 to B17. Each is closed by a named item or kept, and each matches what
`docs/design/principia_chart_composition.md` now says, in § "17.5 A frame: valid, reproducible, recorded" (its list of
ten items) and the sections named.

| # | The limitation, as the design had it | Outcome | Closed by, or why kept | Design, as it now reads |
|---|---|---|---|---|
| B1 | A pixel blank or tagged at either end is blank mid-path | **Closed, conditionally** | (F), applied per R-369 (the orchestrator's two-way decisions): a tagged end that carries a state blends as one; only a no-state end stays blank. TASK-M2-81 (path re-timing and frame flags), `path_blank_ends`. Conditional on physics review, tag by tag; until it confirms, V21 stands | § 17.11, the dated note; § 13, V21; § 17.14, `path_blank_ends` |
| B2 | "A blank can open mid-path" | **Closed** (a record correction, applied per R-369) | That count was a hold run on a blended configuration; the state blend runs no hold (R-926 item 3; R-943, the general rule for a morph). N5 (the blend chart) adds a fixture whose path crosses a binary-collision point of the shape sphere, its outcome a collision at `t = 0` (R-933), physics review confirming | § 17.11, the dated note; `principia_chart_space_model.md` § 0 |
| B3 | Held quantities are not held mid-morph | **Kept: not a limitation** | It says which systems a frame shows. Every feature that reads a held quantity works mid-morph and reads the true value. A path that holds a quantity is one of R-943's other paths, allowed and not built now. TASK-M2-61 (the chart-path object) must not assume the default path: a review point | § 17.11, the dated note under the table |
| B4 | A `gauge` axis's paper scale has no meaning mid-path | **Closed** (applied per R-369) | The dual ruler (R-940): the paper scale is drawn on its own end's side, fading with its share. N16 and N17 (computed axes), TASK-M8-115 (time-axis mode in the GUI) | § 17.11, the dated note |
| B5 | A morph frame's axes carry no ticks | **Closed** | R-940 (the dual ruler) and R-942 (the computed axis is the primary scale on every frame). N16, N17 | § 17.5, item 3 |
| B6 | Marks "to pixel resolution", and the symmetry filter off | **Closed, conditionally** | N19's placement; (D), applied per R-369: marks to the lock's tolerance, the filter on, the frame declaring the symmetries either end declares. TASK-M2-80, TASK-M2-84, TASK-M8-126, TASK-M8-120. Conditional on physics review; the canonical representative on a blend is a definition owed (R-72) | § 17.5, item 6; § 10.4 and § 10.5, the dated notes; § 17.14 |
| B7 | A morph frame is not re-addressable: pan and zoom re-integrate | **Closed** (applied per R-369) | A blend is a chart; a pan or zoom on it re-addresses the cache as on any chart (R-935, under RQ-770 (a)). It changes cost, not results. No task changes for it | § 17.12, the dated note |
| B8 | "CPU batch" frames, the GPU blend entry, "live allowed and not yet fast" | **Closed** (a record correction, applied per R-369) | Every chart, every blend included, is evaluated on the CPU and sent as forward tables (R-926 items 5 and 6). TASK-M8-107 (live paths): its `stage_blend` entry and `blend_entry_counted` are moot | § 17.12, the dated note; § 17.14 |
| B9 | A changed link id is a cut, never a morph | **Closed** | R-949 (a changed link id morphs). TASK-M2-81 (`path_links_morph` in place of `path_links_cut`), TASK-M8-108 (the Sequence's morph segment: a cut mark only on a cut) | § 17.12's conflicts table; § 17.13; § 17.14 |
| B10 | Under a lock, time-axis to IC-only charts keep a labelled jump | **Closed** | R-950 (the jump removed). TASK-M2-65 (REQ-CHART-161's time-axis pair: the pinned state within the lock's tolerance on every frame, the control the jump), TASK-M2-83 (time as an axis, the maths) | the status block; § 18.4; `principia_chart_space_model.md` § 9 |
| B11 | Time to time with different windows is a cut | **Closed** (applied per R-369) | Time blends linearly (R-926 item 3); V29 is retired. TASK-M2-83, `time_morph_rules`: a time-to-time morph with different windows runs, its ends exact; the control a cut | § 18.4; § 13, V29; § 18.6 |
| B12 | A frame is never scored | **Kept: under the one exception** | A replication score is an area statistic on a paper's chart, and no paper's chart is a blend (R-669) | § 17.5, item 10 |
| B13 | Tracking across morph frames does not work | **Kept: not a feature** | A speed idea that measured badly (the re-plan's open item O12). N19 does not rely on it: its seeds are the previous sample and the backwards table | none needed |
| B14 | "Approximate is acceptable during a morph" | **Closed** | R-947: it binds the fragment's table route only; every placement on the CPU refines to the lock's tolerance | § 10.6, "Accuracy" |
| B15 | A frame is not well-posed in contract Part 5's sense | **Kept as a fact; nothing is refused by it** (wording, applied per R-369) | A blend is well-posed when both ends are and its path meets R-943's three conditions | § 17.5, item 2 |
| B16 | The chart-space model's summary: "Time-to-IC morphs are possible but cut by choice" | **Closed** (a record correction, applied per R-369) | Gives way to R-926 and R-950; "Held quantities aren't preserved mid-morph" stands (B3) | `principia_chart_space_model.md` § 9, the dated note |
| B17 | Where a blend repeats: stated nowhere | **Closed** (applied per R-369) | A blend repeats where both ends repeat with the same lattice in pixel space; elsewhere it is affine past its square, its traces drawn in the images it has | § 17.5, item 9 |
| | **Area statistics on a mid-morph frame** | **Kept: the one accepted exception** | R-944 and R-936 (under RQ-760 (iii-a)): `measure` is `none`, no statistic is admitted, the two end charts keep their own | § 17.5, item 1 |

**The count.** Closed: 13 (B1 and B6 conditionally, on physics review). Kept: 4, of which B12 falls under the one
exception, B3 and B13 are not limitations, and B15 is a fact that refuses nothing. The four items of § A close the rest
of § 17.5's list: contours (item 4), traces (item 5), tilt and slice on a live path (item 7, R-902's, unchanged) and
saving a mid frame (item 8).

**Read and not limitations,** as the draft found: design § 17.2's "A morph is of *addresses*, not of systems"; § 17.11's
Gram flags and seams, which are flags and refuse nothing; "One colour window per morph"; § 17.13's reel takeover;
export's `n` renders.

---

## C. RQ-771 (ii): the linear energy chart below its floor at a non-zero spin

Ruled, and not this document's. The draft's § C set out eight points for physics review on the owner's option (ii-2).
R-937 (the invariant charts' second sheet) closes RQ-771 with the physics reviewer's call: below the floor at a non-zero
spin the drawn extension is the rigid rotation at the pixel's own spin, everywhere there, on the linear `(L_z, E)` and
`(L_z, K)` charts and on Burrau `Φ_θ,L_z`, with conditions C1 to C7, and REQ-ENC-030's class rule (the "qualitatively
different IC" rule) binds lookups and the lock, never a drawn extension. The addendum's answers section carries C1 to C7
for N12 to N14. Nothing here adds to it.

## D. The draft's questions

The draft's § D held eight questions in queue form, (A) to (H). All are answered: the table and the lists under "The
owner's answers of 10 Nov 2026" above give each its ruling or its decision per R-369 (the orchestrator's two-way
decisions).

---

## E. Contradictions, each resolved or still open

The draft listed fifteen. Each is resolved by a ruling, by a design section this PR's earlier commits changed, or by
this document; none is left as a claim. Glosses: R-944, the principle; R-945, the backwards direction's list not
exhaustive; R-947 to R-952, the owner's answers above; R-369, the orchestrator's two-way decisions.

1. **R-902 ("No hidden coordinate is invented for the blend, so §17.5 stands as written") against a saved blend tilting
   and slicing.** Resolved by R-944, which amends R-902: the first half stands, and "§17.5 stands as written" no longer
   keeps the "no tilt or slice" sentence. Design § 17.5, item 8; § 13, V20 retired. § A.4.
2. **R-899's Q8 (a) (marks "to pixel resolution", the filter off) against the principle.** Resolved in part by R-944,
   which amends R-899, and by design § 10.4 and § 10.5's dated notes. That the marks meet the lock's tolerance mid-morph
   with the filter on is (D), applied per R-369 and conditional on physics review: not settled until it confirms. If the
   physics reviewer disagrees, it goes to the owner as a queue entry.
3. **R-926 item 7's "ONLY … for the fragment-side needs" against R-542's H11 and the re-plan's TASK-M2-80 row.**
   Resolved by R-945 and R-947.
4. **Design § 17.5's "axes carry no ticks" against the dual ruler.** Resolved by R-940 and R-942. Design § 17.5, item 3.
5. **Design § 17.11's "blank or tagged at either end" and "a blank can open mid-path" against "every in-between frame is
   a valid system" (R-943).** The blank that opens: resolved by § 17.11's dated note (a record correction, applied per
   R-369). The tagged end: (F), applied per R-369 and conditional on physics review, tag by tag; until it confirms V21
   stands, and a disagreement goes to the owner as a queue entry.
6. **`principia_chart_space_model.md` § 0's "Corollary 1 is withdrawn" against R-943.** Resolved by that section's dated
   note: Corollary 1 holds again for the state blend.
7. **R-520's F1, "nothing between two links to interpolate", against R-926 item 3's state blend.** Resolved by R-949,
   which amends R-520 and R-483.
8. **R-849's time-axis jump against R-926.** Resolved by R-950, which amends R-849 and R-901; R-935 had already applied
   it.
9. **Design § 18.4's "Different windows: a cut" and V29 against R-926 item 3.** Resolved by § 18.4's and § 18.6's dated
   notes and V29's retirement (applied per R-369).
10. **The addendum's § A.8 marks TASK-M8-58 (the trace's markers) "KEEP, Unaffected".** Resolved by R-948: the task
    changes (§ A.1, § F.4). The addendum's answers section now points here.
11. **The re-plan's § A.1 does not list TASK-M8-107 (live paths), the contour tasks, the trace tasks or TASK-M7-52 (the
    one projection), each of which R-926 changes.** Resolved by § F.4 here, which lists each with its change. The
    re-plan's answers section now points here.
12. **Lines § 4.2's and REQ-RENDER-093's "GPU pass on the shared decode" against R-926 item 5.** Resolved by R-952 and
    lines § 4.2's dated notes. REQ-RENDER-093 changes with TASK-M7-43 (§ F.5).
13. **REQ-CHART-172's two standards,** "within the lock's tolerance" on a chart and "to pixel resolution" on a mid-morph
    frame. Resolved with item 2: one standard under (D), conditional on physics review; the requirement changes with
    TASK-M2-80 and carries the condition (§ F.5).
14. **The re-plan's § B.3 item 11 measures "the held-`E*` miss mid-morph (§ 17.6's 0.054 median)".** The draft held that
    under the state blend the measurement has no subject. Read against the design it has one: § 17.6 gives the 0.054
    median as the miss of "a chart-level blend", the blended state between two pixels at one `E*`, against 1.4·10⁻¹⁶
    "with the step run after the blend". The state blend is that chart-level blend, so the miss is N5's own, and the arm
    that holds `E*` survives as a map, not a morph (R-936, under RQ-760 (i-a)). So the measurement is kept as written,
    on N5, reported, labelled and never a gate, and a dated note in the re-plan's item 11 says what it measures (applied
    per R-369: a record correction, listed for veto). Nothing is relabelled onto another arm.
15. **The owner's request placed "decision 7" possibly in another PR.** Not a contradiction of the corpus: it is R-926
    item 7, and R-945 cites it there.

**Still open,** none of them among the fifteen: RQ-785 (the picture size P2's and P4's pixels are counted at), which
holds N20; RQ-779 (each edge's default kind), under which N14's rows are built to R-932 as R-941 says; RQ-784 (whether
"No blank areas" reaches a pixel energy normalisation cannot reach); RQ-783 (pre-switch records).

---

## F. The tasks and requirements this document's PR enters

The two merged plans leave their tasks out of `plan/tasks.yaml` and say they land in this document's PR. So that PR
enters all of N1 to N19, the measurement task N20, the changes to existing tasks and the requirement changes below
(applied per R-369, the orchestrator's two-way decisions: task scoping). Each new task gets its `TASK-` id when it is
entered; ids are permanent, and requirements change only through `plan/tools/reqio.py`.

**Reviewers** follow `plan/WORKFLOW.md`'s rule: `code` and `qa` on every task; `physics` where a task touches decode,
encode, the charts, a calibration (R-71) of a physical value or a definition (R-72); `gui` where it closes a GUI-area
requirement or a GUI screenshot; `perf` where it closes a benchmark. **Sizes:** a kernel or physics task is kept under
about 300 mutants where it splits naturally. Where the plans give no natural split the note says so, and the implementer
splits before review if the diff would give more than about 500 mutants.

**How to read "Depends on".** An edge is listed where a plan names it ("after", "with", "from"). Where a plan gives only
the task's place in its ordered list, the edge is marked †: the list is in dependency order, but the plan does not say
the task needs the one before it, or the edge is this document's reading of what the acceptance uses. Each † edge is
confirmed or dropped when the task is entered (applied per R-369: task scoping).

### F.1 The new tasks, N1 to N13 and N15 to N20

| N | Title | Defined in | Milestone | Depends on | Reviewers | Acceptance paragraph | Size note |
|---|---|---|---|---|---|---|---|
| N1 | The first slice: one real chart through CPU f64, per-quad table, records and the integrator, against today's decode | the re-plan, § C and § B.1 item 2 | M2 | TASK-M2-58† (the gate; first in § B.1) | code, qa, physics, perf (its PASS reads a rate on the Mac) | the re-plan § C, "The criterion, written before any run", with "Compared bit for bit" and "The control arm" | a measurement and its harness, three charts; no split |
| N2 | The CPU-to-GPU record and table contract | the re-plan, § A.1 and § B.2 item 5 | M2 | TASK-M2-58; with TASK-M2-59 and TASK-M2-60 | code, qa, physics | the re-plan § B.2 item 5 | contract text, no code |
| N3 | Slopes in three tiers: exact (dual numbers), implicit or none, a flag at each kink site | the re-plan, § B.4 item 13 | M2 | TASK-M2-66†, TASK-M2-67† | code, qa, physics | the re-plan § B.4 item 13, with measurement M-B | kernel; splits by tier if over the limit |
| N4 | The per-quad forward-table builder on the CPU, and measurement M-A | the re-plan, § B.4 item 14 | M2 | N1 (the re-plan's O1), N2† | code, qa, physics, perf (M-A is a benchmark on the Mac) | the re-plan § B.4 item 14: the acceptance, M-A's one-thread criterion and the whole-screen criterion | one builder and one benchmark; no split |
| N5 | The blend chart: the state blend, nested blends, both ends navigated, the ends exact, a signed fold indicator defined (R-72) | the re-plan, § B.3 item 11 | M2 | TASK-M2-61† | code, qa, physics | the re-plan § B.3 item 11, with this document's § A.1 (it exposes its forward map and decode) and § B, B2 (the collision fixture) | **one task: the plan gives no natural split; the implementer splits before review if the diff would give more than about 500 mutants** |
| N6 | Time as the last coordinate: the map's time coordinate, the playhead the window's centre, a target time to an integer index | the re-plan, § B.5 item 19 | M2 | TASK-M2-82, TASK-M2-61†; with TASK-M2-83 | code, qa, physics | the re-plan § B.5 item 19 | kernel and engine; no split named |
| N7 | The backwards table on the CPU: bake, key, fold flag, search | the re-plan, § B.4 item 16 | M2 | N1 (the re-plan's O1), N4†; N14a to N14d (the addendum § C item 3 slots N14 before it) | code, qa, physics | the re-plan § B.4 item 16; its scope every consumer (R-945) | **one task: no natural split; split before review over about 500 mutants** |
| N8 | The five table debug views | the re-plan, § B.6 item 22 | after M2's gate (R-917); **milestone not named by the plan** | N4†, N7†, N9† | code, qa, physics, gui if its verify is a screenshot | the re-plan § B.6 item 22, "Acceptance for each view" | render; one view per commit |
| N9 | The fragment table route: the GPU side of N7 for Chart position and the presets | the re-plan, § B.6 item 22 | after M2's gate; **milestone not named by the plan** | N7, N19 (this document's § A.3: it is measured against `place`) | code, qa, physics, perf (the backwards pass's cost on Metal) | the re-plan § B.6 item 22, "Measurement"; this document's § A.3, "Refined to the lock's precision" | render; no split named |
| N10 | The seed conversion: `w = r/√I` everywhere, every changed golden and reference image a recorded conversion (R-931) | the re-plan, § B.1 item 4 | M2 | none named; before TASK-M2-61's baseline | code, qa, physics | the re-plan § B.1 item 4. Its PR owes the list of images by file, and rewrites `principia_dd_decoder.md` § 2 and § 3.4, the inverse encode contract's seed passage and REQ-DEC-021 | kernel plus goldens; the image list may be its own commit |
| N11 | The lock's solver core, from TASK-M2-73 | the re-plan, § B.3 item 10 | M2 | TASK-M2-61† | code, qa, physics | the re-plan § B.3 item 10 | kernel; no split named |
| N12 | The extension kinds in the contracts: the kinds per edge, each shipped chart's per-edge table, the second sheet and the clamp's metric defined | the addendum, § C item 1, with its answers section | M2 | TASK-M2-58; with TASK-M2-59 and TASK-M2-60 | code, qa, physics | the addendum § C item 1, and its answers section's N12 bullet (R-932, R-937, R-938) | contract text and requirements, no code |
| N13 | The invariant construction's second sheet, and a state's sheet from one source with it | the addendum, § C item 2 | M2 | N12, N10, TASK-M2-66, TASK-M2-67 | code, qa, physics | the addendum § C item 2 with measurement M-C; R-937's C1 to C7 where they bind it; this document's § A.1 (the sheet reading) | kernel; no split named |
| N15 | The backwards table on an extended chart: sheet 1 only, a state looked up as itself and as its time reverse | the addendum, § C item 4 | M2 | N7, N13, N14a | code, qa, physics | the addendum § C item 4 | kernel; no split named |
| N16 | Computed axes, the maths: terms, name, ticks and marks from the window; the dual ruler in a blend | the addendum, § C item 5 | M2 | TASK-M2-61, N5, N14a to N14d; beside TASK-M2-81 | code, qa, physics | the addendum § C item 5 and its answers section's N16 bullet (R-940, R-942); N16's tick tolerance a calibration (R-71) | engine; splits by its seven tests if over the limit |
| N17 | The figure draws the computed axes | the addendum, § C item 6 | after M2's gate; **milestone not named by the plan** (it changes TASK-M8-06 and TASK-M8-50) | N16, TASK-M8-06†, TASK-M8-50† | code, qa, gui, physics | the addendum § C item 6; this document's § B, B4 and B5 | gui; screenshots |
| N18 | Marking clamped and saturated areas: a toggle, off by default | the addendum, § C item 7 | after M2's gate; **milestone not named by the plan** | N14b, N14c, N14e | code, qa, gui, physics | the addendum § C item 7 and its answers section's N18 bullet (R-939: dim or hatch shown to the owner as a pair) | gui; small |
| N19 | The one placement function, `place(state, chart) → {point, sheet, residual, route, status}`, and the two definitions it owes (R-72) | this document, § A.3 | M2 | N5, N7, N11, N13, TASK-M2-67 | code, qa, physics | this document's § A.3, "Acceptance" (`place_routes`), with design § 10.6 | **one task: no natural split; split before review over about 500 mutants** |
| N20 | The measurements P2 and P4, reported to the owner | this document, § A.5 | M2 | N19, N5, TASK-M2-82; waits on RQ-785 | code, qa, physics | this document's § A.5; R-951 | a measurement and its report; no pass line |

### F.2 N14, split by declared kind

The addendum's § C item 3 defines N14 (per-edge extensions in the CPU evaluator and the count) as one task. Every
chart's edges are built now (R-938), which makes it too large for one, so it is split by the kinds R-932 declares
(applied per R-369: task scoping). Each part's acceptance is the clause of the addendum's § C item 3 that names its
kind, with that kind's control (its arm swapped for blank must fail) and "inside `[0,1]²` every chart bitwise
unchanged". A row whose default is still open under RQ-779 (each edge's default kind) is built to R-932's rule as R-941
says: the mass simplex saturates, and the owner confirms the rest by eye when the charts render.

| N | Title | Depends on | Reviewers | Acceptance, from the addendum § C item 3 | Size note |
|---|---|---|---|---|---|
| N14a | Unfold in the CPU evaluator | N13, TASK-M2-66, TASK-M2-67 | code, qa, physics | "unfold, `s = 1 + δ` equal to sheet 2 at `1 − δ` bitwise" | kernel; one kind |
| N14b | Clamp in the CPU evaluator, the clamp below the floor included | N13, TASK-M2-66, TASK-M2-67 | code, qa, physics | "clamp, equal bitwise to the REQ-ENC-030 projection where it projects …", and R-937's ruled rule on the rest, with its conditions C1 to C7 | kernel; the largest of the five; splits at the clamp below the floor if over the limit |
| N14c | Saturate in the CPU evaluator | TASK-M2-66, TASK-M2-67 | code, qa, physics | "saturate, monotone and inside its declared band"; the mass simplex saturates (R-941), and under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` saturates (R-932, the addendum's answers section) | kernel; one kind |
| N14d | Wrap in the CPU evaluator | TASK-M2-66, TASK-M2-67 | code, qa, physics | "periodic, `s` and `s + 1` equal bitwise; pole-crossing as today" | kernel; small |
| N14e | The count, the navigation stop and the sim key | N14a, N14b, N14c, N14d; TASK-M2-38 (the generic navigation stop) | code, qa, physics | "`energy::count` extended per RQ-773"; "the navigation stop per RQ-781 (TASK-M2-38's fuzz extended …)"; "the sim key per RQ-775 (switching a re-addressing kind requests no new quad)" | engine; three parts, one commit each |

**Three parts of the addendum's paragraph that the split by kind does not place, placed here (applied per R-369: task
scoping, listed for veto):** the per-edge declarations ("each registry row and each shipped chart declares, per edge,
the kinds allowed and its default") go with each kind's own task, for the rows that take that kind; the view state's
override goes with N14e, beside the sim key, since RQ-775 rules both; blank's two controls and the debug view's
per-pixel kind go with N14e, which is the first task to see every kind at once.

### F.3 The measurement task, N20

Its row is in § F.1. It closes two new requirements: the report of P2 and P4, with no pass line (R-951); and a
definition requirement (R-72) for the picture size the pixels are counted at, which waits on RQ-785. It runs nothing
until RQ-785 is answered. Its acceptance is that the report exists with every number § A.5 names, at the size the owner
states, and has been shown to the owner. TASK-M8-49 and TASK-M8-58 gain it as a dependency.

### F.4 Changes to existing tasks

| Task | What it builds (gloss) | Change | Depends on, after the change | Reviewers | Source of the change |
|---|---|---|---|---|---|
| TASK-M2-58 | the gate: the owner's rulings applied to the composition design | Also carries R-944, R-945, R-947 to R-952 and this document's R-369 decisions from the design's dated notes into its body, each with its *Was*; (D) and (F) only once physics has confirmed them | unchanged | code, qa, physics | § A, § B here; the design's status block |
| TASK-M2-65 | paths under a lock | `saved_blend_tilt_slice`; the time-axis pair holds the pin with no jump (R-950) | gains N11 and N5 (the re-plan § B.3) | code, qa, physics | § A.4; § B, B10 |
| TASK-M2-80 | points of interest's maths | The nearest chart point and the off-slice distance are `place`'s; mid-morph marks to the lock's tolerance under (D), **with the condition written in: physics review confirms, else R-899's Q8 (a) is what the test asserts** | gains N19 | code, qa, physics, gui | § A.3; § B, B6 |
| TASK-M2-81 | path re-timing, alignment and frame flags | `path_links_morph` in place of `path_links_cut` (R-949); `path_blank_ends` under (F), **with the condition written in: physics review confirms tag by tag, else V21 as written**; no "no tilt or slice" flag | unchanged by this document | code, qa, physics | § B, B1 and B9; § A.4 |
| TASK-M2-83 | time-axis chart mode's maths | `time_morph_rules`: a time-to-time morph with different windows runs, its ends exact; time to IC-only morphs, and under a lock no jump stays | with N6 (the re-plan § B.5) | code, qa, physics | § B, B10 and B11 |
| TASK-M2-84 | the symmetry filter's maths | The partner by `place`; the filter on a blend under (D), **with the condition written in: physics review confirms the mechanism and approves the canonical representative's definition (R-72), else the filter is off mid-morph as R-899's Q8 (a) has it** | gains N19 | code, qa, physics | § A.3; § B, B6 |
| TASK-M7-43 | contours | Evaluation on the CPU's f64 screen grid (R-952); Chart and Latent variables per end on a blend (C); `contour_blend_ends`; R-952's timing and its report | drops TASK-M2-25; gains TASK-M2-67 and N5 | code, qa, gui, physics, and perf for the benchmark | § A.2 |
| TASK-M7-52 | the one projection | Narrows to the fragment's generated half of N19: the WGSL generation and the native-against-WGSL test. **Its CPU half moves into M2, as N19** | gains N19 | code, qa, physics | § A.3 |
| TASK-M7-72 | contours of any registered primitive | Each registered forward evaluated on the CPU's decoded state | unchanged | code, qa, gui, physics | § A.2 |
| TASK-M8-49 | traces on the line system | Placed through `place` on every chart; `line_traces_blend`. **Not built until the owner has seen P2 and P4** | gains N19 and N20 | code, qa, gui, physics | § A.1; § A.5 |
| TASK-M8-58 | the trace's markers and leaving rule | The fold tick; the sheet in the per-axis placement; never inside a clamp or saturate area; the leaving and branch rules on a blend; `trace_fold`, its tolerance a calibration (R-71). **Not built until the owner has seen P2 and P4** | gains N19, N20, N13, and N14a to N14c† | code, qa, gui, physics | § A.1; § A.5 |
| TASK-M8-107 | live paths: the blend entry and the coarse cover | Its `stage_blend` entry and `blend_entry_counted` are moot (R-926 items 5 and 6); what stays is the coarse cover's timing for a blend, read with the re-plan's M-A | gains N4† and N5† | code, qa, physics, perf | § B, B8 |
| TASK-M8-108 | the Sequence's morph segment | A changed link id may morph; the cut mark only on what is a cut (R-949) | unchanged | code, qa, gui, perf | § B, B9 |

**Also touched, by a note in the task file and no change of scope:** TASK-M8-09 (the hover integration: `computeIC`
takes a blend chart); TASK-M8-10 (the hover trace: calls `place`; the playing-morph rule, (E)); TASK-M8-48 (tilt and
slice live on a reopened saved blend); TASK-M8-50 (the Grid preset on a blend); TASK-M8-115 (time-axis mode's ticks, a
`gauge` end's paper scale on its side); TASK-M8-120 and TASK-M8-126 (the filter and the marks on a blend, under (D) with
its condition); TASK-M2-61 (keep; its path type must not assume the default path).

**The two merged plans' own marks on existing tasks** (the re-plan's § A.1, the addendum's § A.3 and § A.8) are entered
as those tables have them, each read with its plan's answers section. The edges they bring: TASK-M2-57 (latent `β`
periodic) loses its edge to TASK-M2-65 and lands before TASK-M2-61's baseline, as N10 does; TASK-M2-73 (numerical
inverses) gives its solver core to N11; TASK-M4-05 (flat-grid compute) reads N2's records.

### F.5 Requirements

| Requirement | Gloss | Change | With |
|---|---|---|---|
| REQ-GUI-427 | the definitions points of interest owe | "How marks are found on a path frame with no inverse" is defined by design § 10.6; its nearest chart point and off-slice metric are read from N19's two definitions | TASK-M2-80 |
| REQ-CHART-172 | points of interest's maths | "On a mid-morph frame the point found to pixel resolution" gives way to `place` and the lock's tolerance, under (D), the condition carried | TASK-M2-80 |
| REQ-GUI-272 | the trace's markers and icon sets | Gains the fold tick, optional and off by default (R-948) | TASK-M8-58 |
| REQ-TOOL-175 | a keyframe's state, a changed link id a cut | "A change of a link id between keyframes must be a cut everywhere" gives way: it may morph (R-949); the rule for link parameters stands | its closing task, with TASK-M2-81 and TASK-M8-108 |
| REQ-CHART-161 | the lock across blends and the mid-morph tilt | A tilt or slice on a live path or a saved frame reopened; the time-axis pair with no jump (R-950) | TASK-M2-65 |
| REQ-RENDER-093 | where contours are evaluated | The CPU's f64 evaluation of the chart for every chart, in place of "a GPU pass on the shared decode with a CPU fallback at reduced resolution" (R-952) | TASK-M7-43 |
| REQ-RENDER-121 | contours of any registered primitive | Its verify reads "each registered forward evaluates on the CPU's decoded state" | TASK-M7-72 |
| REQ-RENDER-092 | the contour expression's variables | The Chart and Latent variables' meaning on a blend, (C) | TASK-M7-43 |
| REQ-CHART-114 | a path frame's flags | "Keep a changed link id a cut" gives way (R-949); "blank or tagged at either end blank mid-path" under (F), the condition carried; `path_links_cut` becomes `path_links_morph` | TASK-M2-81 |
| REQ-GUI-426 | points of interest drawn | Its mid-morph label "to pixel resolution" and "the filter off", under (D), the condition carried | TASK-M8-126 |
| REQ-PERF-117 | a live path by one generic blend entry | The blend entry and "a CPU-only end … labelled live" are moot; the coarse cover's timing stays | TASK-M8-107 |
| REQ-COL-134 | the one projection, one Rust source compiled to WGSL | Stays with TASK-M7-52; the Rust source it names is N19's `place` | TASK-M7-52 |
| REQ-COL-133, REQ-COL-163 | the leaving rule's definition; later states read signed | Their subject is the CPU source, which N19 now builds. **Not placed by the sources:** they move to N19 or stay with TASK-M7-52, placed and recorded when the tasks are entered | N19 or TASK-M7-52 |
| REQ-INT-070 | decode in the kernel, never an IC buffer | Retired through `reqio`, replaced by N2's record requirement (the re-plan § A.2) | N2 |
| REQ-DEC-021 | the momentum seed | Brought into line with R-934 | N10 |
| REQ-CHART-054, REQ-CHART-056 | each axis's extension type; statistics count each system once | Brought into line with R-932 | N12 |
| REQ-CHART-112, REQ-CHART-131 | the path type and its arms; the blend's blocks | Brought into line with R-936 and R-943 | N5, with TASK-M2-60 |
| REQ-DEC-034, REQ-DEC-036 | `J_D` by central differences | Brought into line with R-935 (under RQ-758 (a): exact or implicit slopes, central differences the check) | N3, with their closing tasks |
| REQ-GUI-084, REQ-GUI-246 | axis labels; the figure's axes | Brought into line with R-940 and R-942 | N16, N17 |
| **New** | one per new task, authored from the acceptance paragraph § F.1 and § F.2 name, in the plan's own words and nothing added | A value the paragraph lacks is a calibration requirement (R-71); a definition it lacks, a definition requirement (R-72) | each N |
| **New, named here** | N19's placement function (`place_routes`); N19's two definitions (R-72: the metric, and the answer off the image); N20's report; N20's picture size (R-72, RQ-785); TASK-M7-43's contour timing (a benchmark, R-952); TASK-M8-58's `trace_fold` tolerance (R-71); TASK-M2-84's canonical representative on a blend (R-72); N5's signed fold indicator (R-72, the re-plan § A.2); N16's tick tolerance (R-71, the addendum § C item 5) | | the task named |

The other requirement marks of the two merged plans (the re-plan's § A.2, the addendum's § A.2 and § A.7) are entered as
those tables have them.

### F.6 R-923's hold stands

R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, with TASK-M2-82 (the
trajectory interface and its fixtures) excepted, and allows no new chart task meanwhile. **Entering tasks in
`plan/tasks.yaml` does not lift it.** N1 to N20 are chart tasks: each is entered held, and none is dispatched until the
owner lifts the hold. N20 waits on RQ-785 as well, and TASK-M8-49 and TASK-M8-58 on N20's report having been shown to
the owner.
