# M2 chart re-plan, addendum: chart extensions and computed axes

**Status: proposed, 10 Nov 2026.** This document plans; it builds nothing and dispatches nothing. It adds to the M2
chart re-plan (`plan/charts/m2-chart-system-replan.md`, merged from PR #473; "the re-plan" below), from the owner's two
directions of 10 Nov 2026 afternoon, which are **directions, not rulings**: nothing in force changes until the owner
rules on the questions in § D. `plan/tasks.yaml`, the task files and `plan/requirements.yaml` are not edited here (applied
per R-369, the orchestrator's two-way decisions: task scoping): the tasks in § C are proposed, named N12 to N18 so they
slot into the re-plan's § B beside its N1 to N11, and land in `tasks.yaml` once this addendum merges and the one-way
items of § D are answered.

**Sources.** The owner's directions, quoted verbatim in § 0 from the addendum
`orch/charts/prototype-2026-11-10/addendum-extensions-axes-2026-11-10.md`, on the orchestrator's drive, cited by path and
never copied. The prototype it describes, `orch/charts/prototype-2026-11-10/chart-slice.html`, is now version 21
(the addendum's § 3 lists versions 14 to 21); it is referenced by path only, and its quadtree (its design's § 9) is still
a stand-in, version 15's patch for zoomed-out gaps included, so nothing here copies it. The re-plan, its queue entries
and its two rulings (R-925 and R-926), read first from PR #473's branch `origin/plan/chart-system-2026-11-10` at
ac706b8d and again, as merged, on `origin/main` 58160bb5. The orchestrator's surveys
(`orch/charts/replan-2026-11-10/survey-*.md`). File and line evidence below was read on `origin/main` a5f0aa88.

**Every number from the prototype is a toy-map figure** from its no-WebGPU fallback (the addendum's own words: "Numbers
come from the four toy maps, not the kernel's charts"); none is quoted here as measured on a real chart, and every
measurement in § C runs on the kernel's charts before anything rests on it (R-649, targets are measurements, never
gates; R-650, replication is scored in its own milestone).

**The re-plan's ids, by number.** PR #473 has merged, so its two rulings and its queue entries are entries on `main`
and are cited here by number (a record correction, applied per R-369: naming; earlier heads of this document named
them by topic, since `plan/check_plan.py` requires every ruling and queue number in a plan file to resolve and they did
not yet). **R-926** is the ruling of the owner's eleven chart-system decisions (a chart is a
map composed with a flat window; time the last coordinate; a morph a blend of two charts' states; the lock one
constraint; the CPU evaluates charts and the GPU only integrates; per-quad forward tables; a backwards table for the
fragment; no refusals; three slope tiers; table debug views; the one-cell patch), cited "R-926 item 3" and so on;
**R-925** is the ruling of the owner's six prototype decisions; and the re-plan's queue entries are cited by id, each
with its topic ("RQ-762, the window's per-coordinate scale").

## 0. The owner's direction, verbatim (not a ruling)

From the addendum, each "Direction (owner):" sentence as written:

1. § 1, every chart defined on every pixel: "a chart never has a blank area. The 2D slice should always be looking at
   systems, so every edge of a chart's primary range has a declared extension. This follows from "shaders work on every
   chart", and it means a blend of two charts has no gaps either."
2. § 1.2, selectable per edge: "where a chart has no fully valid extension (the mass simplex, L × E), the extension kind
   is a choice per edge, with a default. The owner picks the defaults by looking."
3. § 2, computed axes: "axis names, tick positions and tick values are derived algorithmically, and stay correct through
   pan, zoom, rotation, tilt, slice, blend and morph. An axis that is no longer a pure coordinate says what it is made
   of. One axis per edge, including in a blend."
4. § 2.2, the domain in the ticks: "no "wraps", "folds back", "saturated" or "one system" text, and no "its maximum"
   style names. The numbers and ticks carry it."

The owner wants these in the plan; they have not ruled them. Directions 1 and 2 meet several rulings in force (§ E); the
one-way questions are RQ-771 to RQ-781 (§ D), and the addendum's § 1.2 table is RQ-779, proposed defaults for the owner
to confirm by eye, not decided.

## 0.1 Ids used here, each with a plain-English gloss (R-693, every id glossed for the owner)

**Rulings.** R-26, every chart declares its domain function (`validate`); R-27, both Burrau charts kept, each labelled
with its quotient; R-71, a missing value becomes a calibration; R-72, a missing definition is written by the task that
needs it; R-82, the momentum seed rule; R-92, what the sim key holds of navigation (pan and zoom re-address, tilt and
slice re-integrate); R-369, the orchestrator's two-way decisions; R-407, each axis extends past the square by a declared
type, bounded the default, no invented continuations; R-408, statistics count each system once through the types'
primary ranges; R-437, `atan2` is 0 at the poles; R-459, the compass cube's axis labels are dimension buttons; R-466,
the section resets and the Home button in the canvas's corner; R-467, a not-a-system pixel is background in a
presentation view; R-469, a chart never leaves valid systems, edits stop at the
last valid value; R-470, a not-a-system pixel draws blank, the hatch only an optional overlay; R-471, energy-type axes
warped per pixel by default; R-472, tilted, sliced and saved charts extend by their axes' types; R-474, the two tilt maps,
Tip as sheet the default; R-476, the line system draws axes, ticks and labels; R-503, the exponential map's radius
bounded at π; R-506, every physical-quantity axis warped by default; R-507, tilt units (an axis's full range, a slider
span); R-508, the energy chart's top side continues, its other sides bounded; R-509, the line system's open questions
ruled; R-537, the extension reached by pan and zoom, "never any gaps"; R-538, the trace's markers (a start inside the
collision radius a collision at `t = 0`; a seam tick); R-544, the shape sphere is the whole sphere; R-552, R-537's
extension questions (the primary-square outline, repeats at full strength); R-554, the exponential map past `r = π` the
domain's end by design; R-570, latent charts have no pan or zoom-out limit, the far field saturating by the decoder's
links; R-583, every mark length in base px; R-599, collapse judged per axis, the readout naming a saturated axis; R-613,
no fold anywhere on the shape sphere; R-649, targets are measurements; R-650, replication tiers and M9; R-655, the
tilted sphere's lattice of repeats; R-693, every id glossed; R-791, the energy axis labelled Newtonian energy under
softening; R-819, that reaching every axis that reads the potential; R-834, the four axis kinds stay; R-842, time as an
axis is a chart mode; R-850, one path abstraction for view edits; R-879, final-condition charts with backward runs, after
the M8 charts; R-898, latent `β` periodic over the full circle; R-916, M2 is the whole non-visual chart system; R-917,
the build order (charts, integrator, visuals); R-920, the slider spans; R-923, integrator work before charts, for now
(the pause of the chart tasks); R-925, the owner's six prototype decisions; R-926, the owner's eleven chart-system
decisions.

**Review queue.** RQ-771, the invariant charts' second sheet and nearest edge system; RQ-772, which edges have a genuine
continuation; RQ-773, amending R-407, R-408, R-470 and R-554 for "no blank", and the count; RQ-774, marking clamped or
saturated areas; RQ-775, where the per-edge choice lives and the sim key; RQ-776, the backwards table on an unfolded
chart; RQ-777, mixed-axis values and units; RQ-778, the share's scale and the 2% drop; RQ-779, the defaults table, by
eye; RQ-780, computed axes against the line system's primary scale and the no-ticks-mid-morph rule; RQ-781, where the
view's centre may go. RQ-734, final-condition charts (ruled by R-879, archived). The re-plan's: RQ-760, what a state
blend leaves open, folds no axis declares among it; RQ-762, the window's per-coordinate scale; RQ-764, the momentum
seed; RQ-767, the table route's accuracy outside a morph.

**Tasks and requirements** are glossed in their rows of § A the first time they appear.

## A. What exists, marked against the two directions

Each table is both § A (what exists, glossed) and § B (the mark): **KEEP** (stands), **CHANGE** (stands, rewritten),
**DROP** (retired) or **NEW**. Where the mark waits on an owner's answer the RQ is named and the mark is this plan's
recommendation. "Direction 1" is extensions on every edge, "direction 2" computed axes.

### A.1 Extensions: rulings

| Ruling | What it says | Mark | Why |
|---|---|---|---|
| R-407 | Four types, affine, periodic, pole-crossing, bounded (the default, blank past the edge); "no invented continuations"; a custom chart inherits from its axes | CHANGE (RQ-773) | Direction 1 adds unfold, clamp and saturate and leaves blank only where declared; each new declaration still under its clause 3's physics review. Its inheritance from axes stays (RQ-775) |
| R-408 | Count each system once: a pixel counts only inside every axis's primary range | CHANGE (RQ-773) | The rule stands; the new kinds need primary ranges (an unfold sheet counted, a clamp not) |
| R-467, R-470 | A pixel that is not a system draws blank; the hatch an optional overlay | KEEP | Still governs validity failures and any edge declared blank |
| R-469 | Every pixel a system or explicitly not one; edits stop at the last valid value | KEEP (RQ-781) | The stop stands for true ends; whether the centre crosses onto a sheet is RQ-781 |
| R-471, R-506 | Physical-quantity axes warped by default, per pixel | KEEP | The warp makes the energy chart's edges straight (`s = 0, 1`; `t = 0`), which makes unfold and clamp simple there (§ A.11) |
| R-472 | Tilted, sliced, saved charts extend by their axes' types | KEEP | New kinds act on the chart parameters after the tilt as today's do |
| R-474 | Two tilt maps, Tip as sheet the default | KEEP | Extensions and computed axes read the window under either map |
| R-437 | `atan2` defined at the poles | KEEP | The addendum's § 1.3 fix (pole shapes are systems) is the corpus's already |
| R-508 | Energy chart: `t > 1` continues; `t < 0` and both `s` edges bounded | CHANGE (RQ-771, RQ-779) | The `s` edges unfold, `t < 0` clamps to the rest start, if ruled |
| R-537 | Extension reached by pan and zoom, "never any gaps"; EX2 and EX4, "the only blank pixels are not-a-system pixels" | CHANGE (RQ-773) | The owner's "never any gaps" stands; EX2 and EX4's allowance for blank narrows to declared blank and validity failures |
| R-538 | A start inside `r_coll` is a collision at `t = 0`; the trace's optional seam tick at a θ wrap | KEEP | The addendum's § 1.3 second fix is the corpus's already; the seam tick is the mark language direction 2 asks for |
| R-544, R-613, R-655 | The whole sphere, no fold; the tilted lattice of repeats | KEEP | Unaffected |
| R-552 | Repeats at full strength; the primary-square outline, on by default | KEEP | The outline already shows where a chart's own range ends (RQ-774) |
| R-554 | Exponential map past `r = π` blank by design | CHANGE (RQ-772) | Past `r = π` the cap repeats the sphere; a declared repeat, if ruled |
| R-570 | Latent charts: no limits; the far field saturates by the links, nothing clamps or fades | KEEP | It is the addendum's "saturate" on the latent chart, already the behaviour, with no new kind |
| R-599 | Collapse judged per axis; the readout names a saturated axis | KEEP | Where saturation is is judged from the field, not a hand-picked point |
| R-834 | Four axis kinds stay | KEEP | Extension kinds are per edge, not axis kinds |
| R-898 | Latent `β` periodic | KEEP | The addendum's "β wrap" is this |
| R-92 | Pan and zoom re-address; tilt and slice re-integrate | KEEP (RQ-775) | Decides which kinds can stay off the sim key |
| R-879 | Final-condition charts with backward runs | KEEP | The only backward integration designed; a time axis before 0 would be the second (RQ-772) |

### A.2 Extensions: requirements

| Requirement | What it asks (gloss) | Mark | Why |
|---|---|---|---|
| REQ-CHART-054 | Each axis declares one of four types; the fallback; the existing charts' types | CHANGE (RQ-773) | The kinds per edge (N12) |
| REQ-CHART-055 | The Burrau family's types defined (built) | KEEP | Stands as written; N12 revisits its edges under RQ-772 |
| REQ-CHART-056 | Statistics count each system once through primary ranges | CHANGE (RQ-773) | Primary ranges for the new kinds |
| REQ-CHART-057, REQ-CHART-083 | The sphere's statistics over the whole sphere | KEEP | Unaffected |
| REQ-CHART-058 | Every gallery chart has 0 not-a-system pixels inside `[0,1]²` | KEEP | Inside the square nothing changes |
| REQ-CHART-060 | One generic navigation stop | KEEP (RQ-781) | As R-469 |
| REQ-CHART-066 | A tilted chart extends by its types after the tilt | KEEP | New kinds likewise |
| REQ-CHART-067, REQ-CHART-068 | Energy axes warped by default; linear an option in the sim key | KEEP | The edge's shape depends on the mapping (RQ-771) |
| REQ-CHART-076 | Under Tip as sheet a pixel whose `(a, b)` leave the square is past the edge | KEEP | The new kinds then act on it |
| REQ-CHART-079, REQ-CHART-080 | Statistics unchanged past the square; a mixed periodic axis is affine | KEEP | Unaffected |
| REQ-CHART-087 | Latent: every pixel a system; the far field flat by the links, nothing clamps, rescales, wraps or fades | KEEP | Better than the prototype's saturation: no new kind, no threshold |
| REQ-CHART-088 | The saturation tolerance, a calibration | KEEP | Replaces the prototype's hand-picked saturation points (8 and 4) wherever saturation is marked |
| REQ-CHART-096 | One registry row per primitive, with its natural extension type | CHANGE (RQ-775) | Per-edge kinds allowed and a default per row |
| REQ-CHART-104 | The four warps and the pair maps | KEEP | A warp's implied type becomes its edges' default |
| REQ-ENC-030 | The metric of "the nearest valid point" for lookup and lock | KEEP | Defines the warped energy charts' nearest edge system everywhere (fixed `E`, `L_z` onto the parabola; below `t = 0` the rest start): the clamp reuses it there. Under the linear option and on `Φ_θ,L_z` it rejects `K* ≤ 0` with `L_z ≠ 0` (a class change), so there the clamp is RQ-771 (ii)'s |
| REQ-GUI-180 | F3 fills the window by the types | CHANGE (RQ-773) | By the new kinds |
| REQ-GUI-268 | The figure draws the types past the square, blank past a bounded edge, no gaps | CHANGE (RQ-773) | Blank only where declared or invalid |
| REQ-GUI-269, REQ-GUI-270 | Every interaction past the square; the view state's unwrapped centre | KEEP | A readout may still name a repeat: readouts are not axes |
| REQ-GUI-348 | The primary-square outline, on by default | KEEP | Shows where the extension starts (RQ-774) |
| REQ-SCHED-101, REQ-REF-054 | The quadtree tiles the plane at canonical addresses; neighbours across seams | CHANGE (RQ-775) | A clamp re-addresses to its edge's quads; an unfold sheet has its own addresses; neighbours cross a fold as they cross a pole |
| REQ-TOOL-182 | Deep zoom in repeats | KEEP | An unfold sheet is a real region, zoomed as one |
| REQ-COL-108, REQ-COL-109 | Presentation and debug drawing of not-a-system pixels | KEEP | The debug view also names each pixel's kind (N14) |
| NEW (N12) | The extension kinds, per edge, each with its primary range | NEW | Direction 1 |
| NEW (N13) | The invariant construction's second sheet, defined (R-72) and tested | NEW | RQ-771 |
| NEW (N14) | Per-edge extensions in the CPU evaluator and the count | NEW | Direction 1 |
| NEW (N15) | The backwards table on an extended chart, measured | NEW | RQ-776 |
| NEW (N18) | Marking clamped and saturated areas, only if ruled | NEW | RQ-774 |

### A.3 Extensions: tasks

| Task | What it builds (gloss) | Mark | Why |
|---|---|---|---|
| TASK-M2-11 | Acute-angle Burrau charts and their axes' types (built, PR #241) | KEEP | Built; N12 revisits its edges under RQ-772 |
| TASK-M2-28 | The whole-sphere display, the exponential map's cap (built) | KEEP | Built |
| TASK-M2-35 | Every gallery chart swept over `[0,1]²` (built) | KEEP | Inside the square nothing changes |
| TASK-M2-36 | Tilt and slice on the latent-control charts, Tip as sheet (built) | KEEP | Built |
| TASK-M2-37 | Energy axes warped by default, the extension per mapping (built) | KEEP | Built; N14 extends `energy::sides` |
| TASK-M2-38 | The generic navigation stop | CHANGE (RQ-781) | Sheet-aware `validate`; the fuzz extended (N14) |
| TASK-M2-39 | Under Tip as sheet, past the edge is past the edge | KEEP | Unaffected |
| TASK-M2-58 | The gate: rulings applied to the composition design (the re-plan: CHANGE) | CHANGE | Also carries the answers to RQ-771 to RQ-781 |
| TASK-M2-59 | The composition in the contracts (the re-plan: CHANGE) | CHANGE | With N12 |
| TASK-M2-66 | The primitive registry (the re-plan: CHANGE) | CHANGE (RQ-775) | Each row's edges: kinds allowed and a default |
| TASK-M2-67 | CPU decode and encode through the composition (the re-plan: KEEP) | KEEP | N14 applies the kinds before it |
| TASK-M2-71 | Warps and pair maps | KEEP | Its warps' implied types become defaults through N14 |
| TASK-M8-87 | The real engine's extension types, statistics and F3 fill | CHANGE (RQ-773) | The new kinds |
| TASK-M8-55 | Past the square with the controls shown, no gaps | CHANGE (RQ-773) | Blank only where declared |
| TASK-M8-56 | The quadtree past the square | CHANGE (RQ-775) | Clamp re-addressing; sheet addresses |
| TASK-M8-57 | Deep zoom in repeats; the far field's saturation tolerance | KEEP | Unaffected |
| TASK-M8-44 | The sphere's statistics over the whole sphere | KEEP | Unaffected |
| TASK-M2-17 | Encode by axis kind, where REQ-ENC-030's metric is defined (built) | KEEP | Its nearest point is the clamp's |
| N12 to N15, N18 | **NEW** (§ C) | NEW | Directions 1 and 1.2 |

### A.4 Extensions: contracts and designs

| Where | What it says | Mark | Why |
|---|---|---|---|
| `principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)" (line 179) | The four types, the fallback, blank, the count, repeats and the canonical point | CHANGE (RQ-773) | N12 rewrites it per the answers |
| `principia_chart_reference.md` § 5.4 (line 1116) | Each shipped chart's types and counts | CHANGE (RQ-772) | Per-edge table (§ A.11) |
| `principia_chart_reference.md` § 2.1 (line 284) | The warp's `s` bounded, `t` bounded below 0 | CHANGE (RQ-771) | The sheet and the clamp |
| `principia_chart_reference.md` § 2.2 (iii) (line 461) | `a = √(2·(K* − Lz²/(2I)))`, always the outward sheet | CHANGE (RQ-771) | A sheet sign, defined under R-72 |
| `principia_inverse_encode_contract.md` § "Chart-aware validation", "The nearest valid point" (line 469), "A qualitatively different IC" (line 481; the reject at 489–494) | Fixed energy, `L_z` onto the parabola; below the apex the rest start where `L_z = 0`; `K* = 0` or `K* < 0` with `L_z ≠ 0` rejected; a warped chart point below `t = 0` projected to the rest start; other charts clamp in their own coordinates | KEEP | The defined nearest edge system the prototype lacks, on the warp everywhere; under the linear option and on `Φ_θ,L_z` not where it rejects (RQ-771 (ii)) |
| `principia_chart_composition.md` § 3 (line 183), "extension: the natural type" | One type per registry row | CHANGE (RQ-775) | Per edge, kinds and a default |
| `principia_chart_composition.md` § 4 (line 470), "Extension it implies" | Each warp's type | CHANGE | Becomes the edge's default |
| `principia_chart_composition.md` § 5.1, pair maps | Polar's periodic and pole-crossing | KEEP | Unaffected |
| `principia_chart_composition.md` § 18, time | Time from `t = 0` | CHANGE (RQ-772) | Before `t = 0` is silent |
| `docs/gui/principia_render_gui_spec.md` § G2, "Past the square with the controls shown" (line 415) | Blank past bounded edges | CHANGE (RQ-773) | With N14's kinds |

### A.5 Extensions: code

| Item | What it is | Mark | Why |
|---|---|---|---|
| `crates/kernel/src/chart/energy.rs:124`, `ExtensionType` | The four types | CHANGE | The kinds of N12 |
| `energy.rs:137`, `Sides { below, above }` | A type on each side of an axis | KEEP | **Already per edge**: direction 1.2's "a choice per edge" needs no new structure |
| `energy.rs:162`, `sides(quantity, mapping)` | Each quantity's types by mapping | CHANGE (RQ-779) | The defaults |
| `energy.rs:192`, `count` | R-408's classification | CHANGE (RQ-773) | The new kinds' counts |
| `crates/kernel/src/decode/invariant.rs:12`, `:532` | The construction; `a` the non-negative root | CHANGE (RQ-771) | A sheet input (N13), sheet 1 bitwise today's |
| `invariant.rs:273`, `infeasible` | The exact feasibility test | KEEP | Sheet-independent: it decides on `(L_z, K*)` alone |
| `crates/kernel/src/chart/mod.rs:176`, `ValidationResult::{Project, Clamp}` | Lookup's nearest point and per-coordinate clamp | KEEP | Reused for a pixel's clamp |
| `crates/kernel/src/chart/physical.rs:385`, `clamp_vertical` | Validation clamps a point to a bounded edge before deciding | KEEP | A validation step, not a drawn extension |
| `crates/kernel/src/chart/shape_sphere.rs:504`, `:506` | On `r = π` one system; past it the domain's end | CHANGE (RQ-772) | Past `r = π` a repeat, if ruled |
| `crates/kernel/src/chart/tilt.rs:227`, `TiltSpec::strip`; `:276`, `Strip` | The lattice of repeats and its canonical point | KEEP | Unaffected |

### A.6 Axes: rulings

| Ruling | What it says | Mark | Why |
|---|---|---|---|
| R-476, R-509 | The line system draws the frame, ticks, labels and titles, styled and preset | KEEP | Better than the prototype's single style: styles, presets, secondary scales, base px |
| R-583 | Every mark length, ticks included, in base px | KEEP | Seam and fold ticks take base px too |
| R-459 | The compass cube's H, V, D labels are dimension buttons | KEEP | The compass, not the figure's axes |
| R-791, R-819 | The energy axis named "Newtonian energy" under softening | KEEP | A name, not one of direction 2.2's domain words |
| R-842 | Time as a chart mode; ticks in canonical time units | CHANGE | Already amended by R-926 item 2 (time the last coordinate of every map); its ticks become a computed axis (N16) |
| R-507, R-920 | Units: an axis's full range at zoom 1, a slider span | KEEP | The `scale_k` of the prototype's share rule, completed by RQ-762, the window's per-coordinate scale (RQ-778) |
| R-850 | One path abstraction for every view edit, the morph its general form | KEEP | Unaffected; the no-ticks line is the composition design's § 17 text, marked in § A.9 (RQ-780) |
| R-538 | The trace's seam tick, "a short tick at both ends of the jump" | KEEP | The precedent for direction 2.2's taller seam tick |
| R-599 | The readout names a saturated axis | KEEP | A readout line, not axis text |

### A.7 Axes: requirements

| Requirement | What it asks (gloss) | Mark | Why |
|---|---|---|---|
| REQ-GUI-084 | Axis labels: the short name and the range at each end | CHANGE (RQ-780) | Name and range computed from the window, mixed names included |
| REQ-GUI-246 | The figure's axes as the line system's: sides, primary and secondary scales, warped ticks at round values, presets | CHANGE (RQ-780) | The primary scale becomes the computed axis; seam, fold, clamp and saturate marks; secondary scales, presets and styles kept |
| REQ-RENDER-118 | Mark lengths in base px | KEEP | Unaffected |
| REQ-GUI-422 | Time-axis mode: ticks and a badge | CHANGE | Time ticks from the computed axis |
| REQ-GUI-397 | The readout's "saturated along" line | KEEP | Readout text, outside direction 2.2 |
| REQ-GUI-416 | The softened energy axis's label (TASK-M9-39) | KEEP | A name |
| REQ-CHART-113 | A morph's ends selected exactly | KEEP | Gives "at either end … exactly that chart's coordinate" (N16) |
| NEW (N16) | Computed axes: terms, name, ticks and marks from the window | NEW | Direction 2 |
| NEW (N17) | The figure draws the computed axes | NEW | Direction 2 |

### A.8 Axes: tasks

| Task | What it builds (gloss) | Mark | Why |
|---|---|---|---|
| TASK-M6-26 | The mock GUI's axis labels (built) | KEEP | The mock stays; the real engine's are N17 |
| TASK-M8-06 | The Manifold view panel and the figure's axis labels (REQ-GUI-084) | CHANGE | Labels from N16 |
| TASK-M8-50 | Lines ▾, the axes and presets (REQ-GUI-246) | CHANGE | Primary scale from N16 |
| TASK-M7-70 | Base px for every mark | KEEP | Unaffected |
| TASK-M8-115 | Time-axis mode in the GUI | CHANGE | Ticks from N16 |
| TASK-M8-58 | The trace's markers, seam ticks included | KEEP | Unaffected |
| TASK-M9-39 | The softened force law and the energy axis's name | KEEP | Unaffected |
| N16, N17 | **NEW** (§ C) | NEW | Direction 2 |

### A.9 Axes: contracts and designs

| Where | What it says | Mark | Why |
|---|---|---|---|
| `principia_lines.md` § 8b (line 490) | Sides; primary scale the chart's own coordinates; warped ticks at round values; secondary scales monotonic, solved through the map | CHANGE (RQ-780) | Primary from N16; warped and per-pixel quantities keep § 8b's solve; an unfold's turning point marked, not refused |
| `principia_lines.md` § 12 | Exactness: a contour of `h` lies on the ticks, control `h + 0.001` | KEEP | N16 reuses it on a mixed axis |
| `docs/gui/principia_render_gui_spec.md` § G2, "Figure" (line 464) | "Axis labels carry the short axis name and the range at each end" | CHANGE | Computed |
| `principia_chart_composition.md` § 17 (line 1946) | A morph frame's "axes carry no ticks, only "A, B, `s`"" | CHANGE (RQ-780) | Direction 2 asks for ticks through a blend |
| `principia_chart_composition.md` § 18 (line 2458) | Time ticks in canonical time units; "`paper scale differs`" on the axis | CHANGE | Ticks computed; the caveat is about scale, not domain, and is read as outside direction 2.2 (applied per R-369: a reading, listed in § E) |

### A.10 Axes: code

| Item | What it is | Mark | Why |
|---|---|---|---|
| `crates/gui/src/explore/axis_labels.rs:29`, `axes_over` | The mock's labels: the dominant latent coordinate's value at each end (`dominant(q)`) | CHANGE | On a turned or tilted view the axis is a mix and the label names one term; N16's terms replace it on the real engine |
| `crates/gui/src/explore/time.rs:169`, `ticks` | The scrubber's five ticks | KEEP | Not a figure axis |
| `crates/kernel/src/chart/mod.rs:434`, `AxisMeta` | The blocks an axis writes; annotation-only axes | KEEP | Read by the statistics, not the labels |

### A.11 The second sheet, and which real edges have one

**The kernel's exact feasibility test gives no second sheet.** `infeasible` (`decode/invariant.rs:273`) decides
`2·I·K* < L_z²` exactly and is binary. The construction then takes `a = √max(2K* − L_z², 0)` (`invariant.rs:532`,
`I = 1`), the non-negative root, so it always builds `vᵢ = vᵢ^(L) + a·wᵢ`. The prototype's "contracting sheet" is
`−a·wᵢ`. Its "radial rate" is the seed's. Under today's rule (R-82) `w` is the inner pair's dilation
`(ρ̇, λ̇) = (ρ, 0)` where `z_α ≤ 0` and the third body's `(0, λ)` elsewhere (`invariant.rs:455`), so "contracting" names a
different pair on either side of `z_α = 0`. RQ-764, the momentum seed (does the expansion seed `w = r/√I` replace
R-82's rule; the re-plan's N10, "the seed conversion", and its § D item 24, "The seed `w = r/√I`"), may change that: under its
options (a) and (b) `w` is the uniform expansion of the whole system (on `Φ_θ,L_z` under (a) only), and "contracting" names the whole system's
dilation, `I` decreasing, on both sides of `z_α = 0`. The identity below holds for any seed `w` that does not depend
on `L_z`, which both rules satisfy. Since `vᵢ^(L)` is linear in `L_z` and `a` depends on `L_z²`, **sheet 2 at `L_z` is
exactly the time reversal `p → −p` of sheet 1 at `−L_z`**: on the warp, unfolding past `s = 1` shows the time-reversed
chart shifted by one in `s`. Those are real, distinct systems (their forward fates are sheet 1's pasts), and the
identity gives an exact test (N13) and a backwards-table shortcut (RQ-776). The two sheets meet at the fold, where
`a = 0`.

**The edge's shape.** On the default warp (R-471) the edges are straight in the chart's own `(s, t)`: the spin edges
are `s = 0, 1` at every `t` (`L_z = (2s − 1)·L_max(t)`, chart_reference §2.1, line 264), and the `t = 0` row is the
rest start (`L_max(0) = 0`): on an untilted, unsliced chart, where the configuration and masses are held across it,
one system; under a tilt or slice that moves the configuration or masses (R-471 E3, the warp per pixel), a line of
different rest systems, one per pixel, which a clamp below `t = 0` smears downward (the addendum's "streaks"). Only the
linear option (R-471 E2) has the curved, shape-dependent edge the addendum describes, the parabola
`|L_z| = √(2I(E − U))` with each pixel's own `U` and `I` (R-471 E3).

**Where "nearest edge system" is defined.** REQ-ENC-030's nearest valid point keeps `E` and moves `L_z` onto the
parabola, keeping `L_z`'s sign, where `K* > 0`; where `K* < 0` it projects to the rest start only a pair with
`L_z = 0`, and it rejects `K* = 0` or `K* < 0` with `L_z ≠ 0`, since the rest start's `L_z = 0` is another class
(`docs/contracts/principia_inverse_encode_contract.md` § "A qualitatively different IC", lines 489–494: "At `K* = 0`
with `L_z ≠ 0` … the pair is rejected (`L_z`'s class would change). At `K* < 0` the apex keeps `L_z`'s class only where
`L_z = 0` … every other pair there is rejected"). On the warp that reject never arises: past `s = 0, 1` at `t > 0`,
`K* > 0` and the projection is the rigid rotation at `±L_max(t)`, keeping the sign; at `t = 0`, `L_z = 0`; and below
`t = 0` the contract's own last sentence there holds, "`L_z(s, t)` has no real value there. So projecting it to `t = 0`
changes no class, and the projection stands". So on the warped charts the clamp is REQ-ENC-030's projection on every
pixel. Under the linear option, and on `Φ_θ,L_z` (affine in `L_z`), the region `K* ≤ 0`, `L_z ≠ 0` (below the
parabola's floor at a non-zero `L_z`) has no REQ-ENC-030 projection: a clamp to the rest start there would draw the
`L_z = 0` class, the only one that can reach a triple collision (Sundman), over pixels whose own `L_z` is not zero.
What a clamp does there is RQ-771 (ii)'s choice.

**The edges of the real charts**, proposed for RQ-772 and the physics reviewer; "today" is chart_reference §5.4:

| Chart | Edge | Today | Past the edge, physically | Genuine continuation? | Kinds available |
|---|---|---|---|---|---|
| Latent, flat slices, `BodyPlane` | none | affine; the links saturate (R-570) | every `z` decodes | no edge | none needed: already a system everywhere |
| Shape sphere, spherical map | θ at 0 and 1 | periodic | the same shapes | a repeat | wrap (existing) |
| | φ at the poles | pole-crossing | the same shapes, θ + π | a repeat over the pole | pole-crossing (existing): the prototype's "unfold over the pole" |
| Shape sphere, exponential map | the square's edges | bounded | shapes out to `r = π` the square doesn't show (contract Part 3) | yes: the formula continues | affine to `r = π` (a declaration) |
| | `r = π` | the domain's end, blank (R-554) | the sphere again, `r ↦ 2π − r` in the opposite direction | a repeat | a new repeat map (physics review), clamp to `−n₀` (one system), blank |
| `(L_z, E)`, `(L_z, K)`, warped | `s = 0, 1` | bounded | infeasible on sheet 1; sheet 2 | only by RQ-771's definition | unfold, clamp (the rigid rotation at `±L_max(t)`, a line), blank |
| | `t < 0` | bounded | `K < 0`, no system | no | clamp (the rest start: **one** system untilted and unsliced; per pixel under a tilt or slice that moves the configuration or masses), blank |
| | `t > 1` | affine (R-508) | `K > K_max` | yes, existing | affine |
| `(L_z, E)`, `(L_z, K)`, linear | the parabola | affine, infeasible blank | per pixel, curved | as the warp, at fixed `E` | unfold (reflect `L_z` at fixed `E`, flip the sheet), clamp (REQ-ENC-030 where `K* > 0`; RQ-771 (ii) where `K* ≤ 0`, `L_z ≠ 0`), blank |
| Lone or mixed energy axis at a held `L_z` | below the floor | bounded below `t = 0` | below the least kinetic energy | no | clamp (the rigid rotation, a line where the other axis varies), blank |
| Mass simplex | each side | affine, negative masses blank | a negative mass | no | clamp in the chart's own coordinates (a line of edge systems; one system at a corner), saturate (masses below the edge's `ε_m`, a definition), blank |
| Burrau `ν` | past its range | bounded | real systems to `ν = 0` or 1, twins past the leg swap (R-27), then a zero mass | partly | affine to the domain's end (a declaration), clamp, blank |
| Burrau `θ` | past `π/4`; past `θ_min` | bounded | twins of the fold's own (a repeat); thin triangles then a zero mass | partly | a reflection (new), affine to the end, clamp, blank |
| Burrau `m` (annotation) | both | bounded | the same `ν` family again | a repeat | a repeat, clamp |
| Burrau `K` | `t < 0` | bounded | `K < 0` | no | clamp (the rest row, a line of shapes), blank |
| Burrau `Φ_θ,L_z`'s `L_z` | past `±√(2IK)` | affine, then blank | as the invariant charts | by RQ-771's sheet | unfold, clamp (as the linear option's), blank |
| Burrau δm strip | past either end | affine, then a zero mass | no system | no | clamp, saturate (a definition), blank |
| Region D `(ξ, η)` | `η = 0`, `ξ = 0` | bounded | the mirror; the relabelling (composition § 8, line 1029) | repeats | a reflection (new), clamp, blank |
| Composed charts (the registry) | per primitive | the row's natural type | per row | per row | each row declares, physics review |
| Time axis | before `t = 0` | silent | the trajectory's past | yes, by a backward run (new compute; R-879's is the only one designed) | clamp to the start (a column of equal rows), a backward run, blank |
| | past `T` | the march continues (R-842) | the future | yes | existing |

"Saturate", outside the latent chart's own links, has no definition anywhere in the corpus: each is a definition under
R-72 and R-407's clause 3.

### A.12 Counts

Rows of § A.1 to § A.10 (a row may group several ids). Extensions: rulings KEEP 15, CHANGE 5; requirements KEEP 16,
CHANGE 6, NEW 5; tasks KEEP 11, CHANGE 7, NEW 1 (N12 to N15, N18); contracts and designs KEEP 2, CHANGE 8; code KEEP 5,
CHANGE 5. Axes: rulings KEEP 8, CHANGE 1; requirements KEEP 4, CHANGE 3, NEW 2; tasks KEEP 4, CHANGE 3, NEW 1 (N16,
N17); contracts and designs KEEP 1, CHANGE 4; code KEEP 2, CHANGE 1. Total: KEEP 68, CHANGE 43, DROP 0, NEW 9. Each row
carries one mark; a source whose parts are marked differently is split into one row per part.

## B. Against the two directions: where the existing design is better, and what is new

The marks are in § A. Plainly, the existing design is better than the prototype's sketch in these places, and the plan
keeps it:

1. **Per edge already.** `energy::Sides` (`energy.rs:137`) types each side of an axis separately; direction 1.2 needs
   no new structure, only more kinds and a default per edge.
2. **The nearest edge system is defined on the warp.** REQ-ENC-030's metric (fixed energy, `L_z` onto the parabola;
   below the apex the rest start where `L_z = 0`; elsewhere a clamp in the chart's own coordinates) is a reviewed
   definition, and on the warped charts it projects every pixel (§ A.11); the prototype's "nearest system on the edge"
   is not stated in any metric. Under the linear option and on `Φ_θ,L_z` it rejects `K* ≤ 0` with `L_z ≠ 0`, so there
   the existing design is not ahead: that region needs RQ-771 (ii).
3. **Saturation is the decoder's, judged, not hand-picked.** On the latent chart it is the links' (R-570,
   REQ-CHART-087: "nothing may clamp, rescale, wrap or fade"), where it is reached is judged per axis (R-599), and its
   tolerance is a calibration (REQ-CHART-088); the prototype's saturation points, 8 for shape and momentum and 4 for
   mass, are hand-picked (the addendum says so).
4. **Ticks of a warped quantity sit at its round values** (lines § 8b), placed by solving along the edge; the
   prototype's exact linear arithmetic holds only for a coordinate linear in the window, so a warped `E`, or `E` read
   with each pixel's own `U` under a tilt, needs § 8b's solve.
5. **The line system** has styles, presets (Screen, Print, Ends only), secondary scales and base px (R-476, R-509,
   R-583); the prototype has one style.
6. **Each system counted once** through primary ranges (R-408) and repeats at canonical addresses (R-537 EX7); the
   prototype has no count for an extended pixel and a stand-in quadtree.
7. **The default warp makes the energy chart's edges straight** (§ A.11): unfold is `s' = 2 − s` on sheet 2 and clamp
   below `t = 0` is the rest start, literally one system on an untilted, unsliced chart (under a tilt or slice that moves
   the configuration or masses, a line of different rest systems smeared downward); the prototype's rectangle is the
   linear option.
8. **The prototype's two fixes** (pole shapes are systems; a start inside the collision radius is a collision at
   `t = 0`) are already the corpus's (R-437, R-538).

**New:** the unfold sheet (RQ-771), clamp and saturate as declared kinds (RQ-773), per-edge defaults chosen by eye
(RQ-779), the axis as a computed sum of window terms through blends (RQ-777, RQ-778, RQ-780), and seam, fold, clamp and
saturate marks on the axis in place of words. Seven tasks, § C.

## C. The new tasks, in dependency order, slotted into the re-plan's § B

Each gives its slot in the re-plan's § B, its acceptance test, the measurement that informs any open choice with its
criterion **written now, before any run** (R-649), and its door: **two-way** (decided by the measurement, recorded
"applied per R-369") or **one-way** (the owner's; the RQ named). No tolerance is tuned to pass; a replication or
prototype figure is a measurement, never a gate (R-649, R-650). Visual work (N17, N18) waits for M2's gate (R-917);
the rest is non-visual and so M2's (R-916).

1. **N12, the extension kinds in the contracts.** *Slot:* § B.2 item 5, with TASK-M2-59 and TASK-M2-60 (chart paths in
   the contracts), after TASK-M2-58 has carried the answers to RQ-771 to RQ-781. *Acceptance:*
   `principia_chart_decoder_contract.md` Part 3 and `principia_chart_reference.md` § 5.4 rewritten from the answers,
   each replaced line kept as *Was*: the kinds (affine, periodic, pole-crossing, unfold, clamp, saturate, bounded, and,
   where RQ-772 declares one, a reflection or a new repeat map, each a new kind under R-407's clause 3), declared per
   edge, each with its primary range and its place in R-408's classification order; each shipped chart's per-edge table
   (§ A.11 as ruled), physics-reviewed under R-407's clause 3; the second sheet defined (R-72) in § 2.2; REQ-ENC-030's
   metric named as the clamp's; REQ-CHART-054 and REQ-CHART-056 changed and the NEW requirements of § A.2 added through
   `plan/tools/reqio.py`. *Measurement:* none. *Door:* one-way, RQ-771, RQ-772, RQ-773, RQ-775, RQ-779, RQ-781.

2. **N13, the invariant construction's second sheet.** *Slot:* § B.4, after item 12 (TASK-M2-66, TASK-M2-67), only under
   RQ-771 (a) or (b). *Order with the re-plan's N10* (the seed conversion, built only under (a) or (b) of RQ-764, the
   momentum seed): N10 sits in that plan's § B.1, before TASK-M2-61's baseline, so N13 comes after it. Under RQ-764 (a)
   or (b), N13's "sheet 1 bitwise today's" baseline is the construction as N10 leaves it, with the seed `w = r/√I` on
   the charts N10 converts; under its (c), R-82's construction as `main` has it now. Either way the baseline is `main`'s
   construction at N13's start, and N13 does not start while RQ-764 is unanswered. *Acceptance:* one source for f32
   and f64 (as `invariant.rs` is): a sheet input, sheet 1 bitwise the baseline on every shipped chart (the control,
   sheet 2 as default, must fail that check); on sheet 2, chart_reference §2.2's three checks (`Σpᵢ = 0`, `L_z(p)`,
   `K(p)`) to machine precision; the feasibility branch identical on both sheets for the same `(L_z, K*)`; **the
   time-reversal identity**, bitwise: sheet 2 is built by negating `a` in the same expression,
   `pᵢ = mᵢ(spin·J rᵢ + (−a)·wᵢ)`, and then, since `(−L_z)·J r` is an exact negation, `a` comes from
   `mul_add(−L_z, L_z, 2K*)`, which is even in `L_z`, and round-to-nearest commutes with negation, sheet 2 at `L_z`
   decodes to positions bitwise equal to, and momenta bitwise equal to the negation of, sheet 1 at `−L_z`, on both
   backends; the one exception is a component whose sum is exactly zero, where `x − y` gives `+0` and `−((−x) + y)`
   gives `−0`, so a zero component is compared by `==` (control: a sheet 2 built as sheet 1 mirrored in `s`, a repeat,
   must fail it). *Measurement M-C*, criterion written now: on `InvariantLE` at the register's defaults, 256 × 256
   samples over `s ∈ [1, 2]`, `t ∈ [0, 1]`, the share of pixels whose decoded state equals a state of the square's
   256 × 256 grid; positions are bitwise equal (`==` per component, the held configuration on both), so a pixel
   matches a grid state when `‖Δp‖₂ ≤ 10⁻¹²·‖p‖₂`, the Euclidean norm over all six momentum components, `p` the grid state's (a zero `p`, the
   rest start, matches only an exactly equal one); PASS if only pixels on the fold columns (`s = 1, 2`) and the `t = 0`
   row match; the control (sheet 1 mirrored) must match 100%, else the measurement cannot see a repeat and the result is
   void. *Precondition*, checked before the run and reported: the register's default configuration has a qualifying
   seed, so its outcome is not `Outcome::Degenerate`. Under a degenerate seed the construction sets `a = 0`
   (`invariant.rs:519-536`), so the two sheets are equal by construction, and, the configuration being held across the
   chart, on every pixel at once; a degenerate default configuration makes the result void, not PASS (under RQ-764
   (a) or (b) it cannot arise for `I > 0`). *Door:* one-way whether it is built (RQ-771); the construction
   two-way.

3. **N14, per-edge extensions in the CPU evaluator and the count.** *Slot:* § B.4, after N13 and TASK-M2-66, before item
   16 (N7). *Acceptance:* each registry row and each shipped chart declares, per edge, the kinds allowed and its default
   (RQ-779); the view state's override as RQ-775 rules; the kinds applied before `Φ` in TASK-M2-67's evaluator, in
   R-407's order; `energy::count` extended per RQ-773; inside `[0,1]²` every chart bitwise unchanged (REQ-CHART-058's
   sweep still 0); per kind, samples past each edge: periodic, `s` and `s + 1` equal bitwise; pole-crossing as today;
   unfold, `s = 1 + δ` equal to sheet 2 at `1 − δ` bitwise; clamp, equal bitwise to the REQ-ENC-030 projection where it
   projects (every pixel of the warped energy charts; under the linear option and on `Φ_θ,L_z`, the pixels with
   `K* > 0`, or `K* < 0` with `L_z = 0`), to RQ-771 (ii)'s ruled rule on the rest (`K* ≤ 0` with `L_z ≠ 0`, or blank if
   so ruled), and to the clamped chart point elsewhere; saturate, monotone and inside its declared band; blank as
   today; each kind but blank with a control that swaps that kind's arm for blank and must fail; blank with its own
   two controls, each of which must fail: its arm swapped for clamp (a pixel past a blank edge drawn as a system), and
   a blank pixel counted by `energy::count`; the debug view's per-pixel kind; the navigation stop per RQ-781
   (TASK-M2-38's fuzz extended: no edit leaves the centre on a clamped or saturated pixel; a pan across the fold lands
   on sheet 2 with `validate` passing); the sim key per RQ-775 (switching a re-addressing kind requests no new quad).
   *Measurement:* none. *Door:* one-way where RQ-773, RQ-775, RQ-779 and RQ-781 decide; the rest two-way.

4. **N15, the backwards table on an extended chart.** *Slot:* § B.4, after item 16 (N7) and N14. *Acceptance:* N7's
   construction on `InvariantLE` with the spin edges unfolded, the sheet carried as RQ-776 rules; every forward node of
   both sheets returns to its own cell or, where cells decode to equal states, to a cell whose state equals its own by
   M-C's match rule (positions bitwise equal, `==` per component, the held configuration; `‖Δp‖₂ ≤ 10⁻¹²·‖p‖₂`),
   criterion written now: the untilted `t = 0` row (every cell the rest start) and the fold columns (sheet 2 at
   `s = 2` equal to sheet 1 at `s = 0`, `s = −1` to `s = 1`), M-C's predicted matches, only there: an equal-state
   return outside those cells fails; the round-trip error in cells below is measured to the nearest such cell.
   *Measurement*,
   criterion written now: N7's set-up (256 × 256 samples, table 64, scan 16) over `s ∈ [−1, 2]`, at `t = 0` of the run
   and after it: the wrong share (a worse match than an exhaustive search of the same table) and the round-trip error
   in cells, reported separately for pixels within 2 pixels of a fold and for the rest; PASS if the rest's wrong share
   is at most the same run's figure over the square alone; EXPLAINED DIVERGENCE if worse with a named cause; else
   UNEXPLAINED. The near-fold share is reported, not judged. *Door:* two-way for the construction; one-way for the
   accuracy, RQ-776 and RQ-767, the table route's accuracy outside a morph.

5. **N16, computed axes, the maths.** *Slot:* § B.5, after TASK-M2-61 (the chart-path object, whose window `(c, Q)` the
   re-plan rewrites), N5 (the blend) and N14 (the kinds), beside TASK-M2-81 (path re-timing and alignment). Non-visual, in
   the engine, on the CPU. *Acceptance:* a function from the window `(c, Q)`, the chart or blend and its share, each
   coordinate's scale (RQ-762, the window's per-coordinate scale) and the edges' kinds, to, per screen edge: the terms (coordinate,
   coefficient, sign), the name, the value along the edge, the tick positions and values, and the marks (a wrap's seam,
   an unfold's fold, a clamp's constant run, a saturated stretch). Tests, each with a control: (1) **ends exact**: at a
   morph's `s = 0` and `1`, selected by REQ-CHART-113's exact comparison, the name and every tick value bitwise the
   unblended chart's (control: the arithmetic blend at `s = 1`, which differs on some pairs); (2) **linear
   coordinates**: the chart's own coordinate, evaluated by the CPU evaluator at each tick's screen position, equals the
   tick's round value within 4 ulps of `max(1, |v|)` (control: ticks offset by one pixel); (3) **warped and per-pixel
   quantities** (`E` on `InvariantLE` under a tilt, R-471 E3): ticks placed by § 8b's solve, the residual within the
   solve's tolerance, which lines § 8b does not state: a calibration requirement (R-71), **N16's tick tolerance**,
   one-way, set by the owner and never by the implementer (RQ-780 (iv)); test (3) waits for it (control: placement
   linear in `t`); (4) **navigation**: a pan
   moves each linear coordinate's tick values by the pan's component along it, a zoom by 2 halves the spacing, a
   rotation and a tilt under either R-474 map give the terms of `Q`'s columns; (5) **marks**: a seam where the value
   crosses a period, a fold at `L_z = ±L_max`, a clamp run of constant values, a saturated stretch only where
   REQ-CHART-088's tolerance or R-599's judgement puts it, never a hand-picked point (control: each mark suppressed is
   detected); (6) **exactness on a mixed axis**: a contour of the axis's own expression lies on its major ticks (lines §
   12; control `h + 0.001`); (7) **terms** as RQ-778 rules: with no drop, the named shares sum to 1. *Measurement*,
   criterion written now: over a 64-frame morph from the shape sphere's exponential map to `InvariantLE` and a tilt
   swept 0° to 90° in 1° steps under each R-474 map, the terms per edge and the longest name in characters, reported
   only (it informs RQ-777, RQ-778 and N17's layout). *Door:* one-way where RQ-777, RQ-778 and RQ-780 decide, and for
   N16's tick tolerance (R-71, RQ-780 (iv)); the rest two-way.

6. **N17, the figure draws the computed axes.** *Slot:* § B.6, after M2's gate (R-917); it changes TASK-M8-06 and
   TASK-M8-50 rather than adding a parallel axis path. *Acceptance:* the line system's primary scale on each side is
   N16's axis; a mixed name drawn as N16 gives it; a taller tick at a seam and at a fold, constant numbers on a clamp
   run, a thicker axis line on a saturated stretch, each term's marks fainter by its coefficient, every length in base
   px (R-583); the Ends only preset reads N16; no domain words on any axis. Screenshot tests, each with a control: the
   latent chart at home, at ×1000, at a 30° rotation, at a 40° tilt with a rotation; `InvariantLE` with its spin edges
   unfolded; a morph paused part-way; a long mixed name never overlaps the Home button's rect (R-466; control: the name
   drawn untruncated does). *Measurement:* none. *Door:* one-way where RQ-777 and RQ-780 decide; the rest two-way.

7. **N18, marking clamped and saturated areas** (only under RQ-774 (b) or (c)). *Slot:* § B.6, after N14 and M2's gate.
   *Acceptance:* an Overlays › Chart toggle, off by default under (b), dimming or hatching exactly the pixels N14
   classes clamped or saturated, drawn over the field as R-470's forbidden-region overlay is (control: a toggle that
   also dims a system pixel inside the square fails). *Measurement:* none. *Door:* one-way, RQ-774.

The order: N12 (contracts) → N13 → N14 → N15, and N16 after N14, N5 and TASK-M2-61; N17 and N18 after M2's gate.
Nothing here is dispatched while the pause of the chart tasks holds (R-923, integrator work before charts, for now).

## D. Questions for the owner

Each is a `REVIEW_QUEUE.md` entry with its file, section, quoted text, options and consequences, recommendation and the
measurement that would inform it.

| RQ | In plain English |
|---|---|
| RQ-771 | The energy chart's "second sheet" (the system's in-and-out motion reversed) is not in the kernel: adopt it as the time-reversed chart, and is "the nearest edge system" the existing lookup rule? |
| RQ-772 | Which edges of which charts have something real beyond them, and which can only stop, smear, squash or stay empty? |
| RQ-773 | "No blank areas" needs the extension rulings changed: do they change, and how do the new areas count in statistics? |
| RQ-774 | Should smeared or squashed areas look different, or only on a toggle? |
| RQ-775 | Is each edge's behaviour part of the chart, the view or both, and does switching it throw away computed work? |
| RQ-776 | Looking a system up on a two-sheet chart: which sheet does it land on? |
| RQ-777 | Midway through a morph an axis can be part angle, part momentum: is a summed number acceptable, and how are units shown? |
| RQ-778 | Which coordinates an axis's name lists, and whether the prototype's "drop under 2%" stays |
| RQ-779 | The default behaviour of each edge, for the owner to pick by looking |
| RQ-780 | Do computed axes replace the line system's own scale and the "no ticks mid-morph" rule? |
| RQ-781 | May the view's centre cross onto the second sheet, or into a smeared area? |

## E. Contradictions

Everything in the addendum that contradicts the repository, the contracts or the rulings, plainly. "Addendum" is
`orch/charts/prototype-2026-11-10/addendum-extensions-axes-2026-11-10.md`.

1. **"A chart never has a blank area"** (addendum § 1) against R-407's default (`decisions.md:7727`: "bounded, the
   default"), R-470's blank (`decisions.md:13797`), R-508's bounded `s` edges and `t < 0` (`decisions.md:16658`;
   `docs/design/principia_chart_reference.md:284`), R-554's "past `r = π` is the domain's end by design, blank and
   unwarned" (`decisions.md:22226`), and R-537's EX2 and EX4, "The only blank pixels are not-a-system pixels"
   (`decisions.md:18130`). The owner's own words in R-537 are "Never any gaps" (`decisions.md:18047`); EX4 read them as
   coverage, with not-a-system pixels blank. The direction is the stronger reading of the same words, and needs a ruling
   (RQ-773).
2. **Clamp and saturate are invented continuations** under R-407 clause 3: "**No invented continuations.** An axis gets
   anything beyond bounded only by declaring it, and any new extension type needs physics review"
   (`decisions.md:7818`; `docs/contracts/principia_chart_decoder_contract.md:263`); R-407's applied text says "P4 (a
   clamped extension) does not arise, since no axis is clamped" (`decisions.md:7919`). Each needs a declaration under
   physics review (RQ-773).
3. **Clamp's "line of systems smeared over an area"** (addendum § 1.1) against R-408's "count a visible pixel only if
   every axis is inside its primary range" (`decisions.md:7972`): no primary range is defined for clamp, so a clamped
   area would count its edge systems again unless declared outside it (RQ-773). It also sits beside R-469's "every pixel
   it draws is a valid three-body system or explicitly "not a system"" (`decisions.md:13643`): a clamped pixel is a valid
   system but not the one its coordinates name, a third case R-469 does not have.
4. **Every edge has an extension, and R-469's stop.** R-537's EX3: "**Bounded charts** … the centre stops at the
   domain's edge, but the window may still show blank past it" (`decisions.md:18122`); REQ-CHART-060 stops an edit "at
   the last valid value" (`plan/requirements.yaml:2992`). The direction does not say whether the centre may follow the
   picture onto a sheet or into a clamp (RQ-781).
5. **Saturation's hand-picked points** (addendum § 2.3: "the saturation point is a hand-picked value (8 for shape and
   momentum, 4 for mass), not measured") against R-599's per-axis judgement (`decisions.md:26217`) and REQ-CHART-088's
   calibrated saturation tolerance (R-71). On the latent chart "saturate" is the links' own behaviour, and
   REQ-CHART-087 (`plan/requirements.yaml:3612`) forbids any extension that would "clamp, rescale, wrap or fade" it.
6. **"Unfold onto the contracting sheet (radial rate changes sign)"** (addendum § 1.2) against the kernel: there is no
   second sheet; `a` is the non-negative root (`crates/kernel/src/decode/invariant.rs:12`, `:532`;
   `docs/design/principia_chart_reference.md:461`). The sheet is a new definition (R-72) and type (R-407 clause 3); its
   "radial rate" is, under R-82's seed, the inner pair's for `z_α ≤ 0` and the third body's otherwise
   (`invariant.rs:455`), and under (a) or (b) of RQ-764, the momentum seed (the re-plan's N10, the seed conversion; its
   § D item 24, the seed `w = r/√I`), the whole system's dilation (RQ-771).
7. **"The feasibility edge depends on shape and is curved"** (addendum § 1.4) holds only under the linear option. On the
   default warp the edges are straight lines `s = 0, 1` and the row `t = 0` (`principia_chart_reference.md:261`,
   `:264`); the prototype's rectangle `L_z × E` is the linear option's picture, not the default chart's.
8. **"Nearest edge system" is undefined in the addendum and defined in the contracts only in part**: REQ-ENC-030's
   nearest valid point (`docs/contracts/principia_inverse_encode_contract.md:469`, `:476`, `:489`): fixed `E`, `L_z`
   onto the parabola where `K* > 0`; below the apex the rest start where `L_z = 0`; `K* = 0` or `K* < 0` with `L_z ≠ 0`
   rejected, a class change. On the warp it covers every pixel (below `t = 0` `L_z` has no real value, so "the
   projection stands"), and "Clamp to K = 0" there is that apex, one system on an untilted, unsliced chart and a line of
   different rest systems under a tilt or slice that moves the configuration or masses. Under the linear option and on
   `Φ_θ,L_z`, `K* ≤ 0` with `L_z ≠ 0` has no projection: the addendum's clamp there needs a definition (RQ-771 (ii)).
9. **"Time before 0: clamp to the starting state"** (addendum § 1.2) meets a silence, not a contradiction:
   composition § 18 starts integration at 0 (`docs/design/principia_chart_composition.md`, § 18's table, "`t_lo` is
   new; integration still starts at 0"). The genuine continuation, the trajectory's past, is a backward run, designed
   only for final-condition charts (R-879) (RQ-772).
10. **Ticks through a morph** (addendum § 2, direction) against composition § 17: a morph frame's "axes carry no ticks,
    only "A, B, `s`"" (`docs/design/principia_chart_composition.md:1946`) (RQ-780).
11. **"Linear in s, so ticks sit at round values by exact arithmetic, with no sampling"** (addendum § 2.1) against lines
    § 8b, "A warped axis's ticks fall at round values of the quantity, so they are unevenly spaced"
    (`docs/design/principia_lines.md:496-497`), and R-471 E3's per-pixel warp (`principia_chart_reference.md`, § 2.1): a
    warped quantity, or `E` with each pixel's own `U` under a tilt, is not linear along an edge, so its ticks need
    § 8b's solve (RQ-780).
12. **"Terms under 2% are dropped"** (addendum § 2.1) against the direction's own "An axis that is no longer a pure
    coordinate says what it is made of": an 8° tilt has a share of `sin²8° ≈ 1.9%`, so the axis is named pure while the
    view moves a second coordinate. The 2% is a hand-set tolerance (R-71). The exactness test of lines § 12 still passes
    on the named expression, so it does not catch this (RQ-778).
13. **An unfold's numbers "count up to the fold and back down"** (addendum § 2.2) against lines § 8b, "A secondary
    scale is refused where the quantity is not monotonic along the edge" (`principia_lines.md:500`): stated for
    secondary scales, and § 8b's solve assumes a monotonic quantity (RQ-780).
14. **"No … text"** (addendum § 2.2) meets axis text already ruled: composition § 18's time axis "carries `paper scale
    differs`" (`principia_chart_composition.md:2458`) and R-791's "labelled as Newtonian energy". Both are names or scale
    caveats, not one of the domain words the direction lists; read as outside it (applied per R-369: a reading), listed
    here for the owner's veto.
15. **Per-chart defaults picked by eye** (addendum § 1.2) against R-407's "A custom chart inherits its extension from its
    axes, so nothing is written per chart" and R-472's "nothing written per chart": defaults per registry row (an axis)
    keep the inheritance, defaults per chart would not (RQ-775, RQ-779).
16. **The mock's axis labels name one coordinate** (`crates/gui/src/explore/axis_labels.rs:29`, `dominant(q)`): on a
    turned or tilted view that names one term of a mix. Not a ruling's contradiction (an R-369 choice of TASK-M6-26), but
    direction 2 supersedes it on the real engine (N17).
17. **Not contradictions, recorded:** the addendum's § 1.3 fixes agree with the corpus (R-437; R-538's "a start already
    inside `r_coll` ends as a collision at `t = 0`", `decisions.md:18597`), and its quadtree, version 15's patch
    included, is a stand-in and not copied, as R-926 and the re-plan hold.

## F. With the re-plan

The re-plan (`plan/charts/m2-chart-system-replan.md`, merged from PR #473) gains N12 to N18 in its § B at the slots
given in § C once this addendum merges; its § A.1 then reads
TASK-M2-38, TASK-M8-87, TASK-M8-55, TASK-M8-56, TASK-M8-06, TASK-M8-50 and TASK-M8-115 as CHANGE for the reasons in
§ A here, and its § E gains RQ-771 to RQ-781. Its open item on a fold no extension declares (its O6, the owner's under RQ-760 (iv)) meets RQ-773: a
blend's fold is not an edge extension and stays its question. Nothing here edits its files.
