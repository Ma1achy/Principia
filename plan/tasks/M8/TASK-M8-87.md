# TASK-M8-87 — Past `[0,1]²` on the real engine: the axis extension types, the area statistics and the F3 fill (split from TASK-M8-05 by R-627)

- **Milestone:** M8
- **Closes:** REQ-GUI-180, REQ-CHART-054, REQ-CHART-056, REQ-CHART-071
- **Depends on:** TASK-M8-88, TASK-M7-21, TASK-M2-32, TASK-M2-34, TASK-M6-48
- **Needs (earlier milestones):** REQ-RENDER-069, REQ-COL-102
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
Split from TASK-M8-05 (R-627; applied per R-369), on the real engine after TASK-M8-88 wires the shell: with F3 hiding the layer the figure fills the window, showing more of the field at the same scale (R-406); past the chart's `[0,1]²` each axis extends by the extension type it declares (affine, periodic, pole-crossing or bounded, the default), and a pixel whose `Φ` fails or whose state fails validation is hatched as forbidden, labelled `decode_failed` and never integrated (R-407). Area statistics over the window count each system once, through the axis types' primary ranges; a pixel hatched where the domain ends leaves both the count and the total, and only validity failures are forbidden (R-408). Its text is TASK-M8-05's goal's, moved.

## References
- `decisions.md` § "R-627 — M8's GUI tasks are built on the mock engine, as R-390's track is, each followed by a small task wiring it to the real engine; TASK-M8-01 is re-planned on the contract that exists; TASK-M8-05 goes first; a v1 web skeleton is deployed to GitHub Pages, by hand only *(extends R-390; the skeleton measures for RQ-550 and RQ-551 and settles neither)*"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "Rules that hold everywhere"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field at the same scale rather than stretching; showing the layer returns the layout"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `decisions.md` § "R-446 — "Degenerate" leaves the outcome view: black `#141418` means nothing has happened yet, up to the playhead, and bounded, running and every failure draw it; failures show in the debug views; forbidden chart regions keep the chart contract's hatch, decided from the chart's validity *(supersedes R-96, R-399 and R-441 as they apply to the outcome view)*"
- `decisions.md` § "R-467 — Forbidden pixels and non-finite values are debug-only: a presentation view draws a pixel that is not part of the chart as the canvas background, with no hatch, key or share, and every failure and non-finite value `#141418`; one switchable debug view, View › Debug view, shows the hatch, `debug_invalid` and a NaN / Inf style, each with its key and count *(amends R-446, and R-407 clause 2 as it applies to display; closes RQ-319)*"
- `decisions.md` § "R-468 — With the egui layer hidden, F3 fills the window by one of two modes: Extend, the default and R-406's, more of the field at the same scale, or Fit, the view's square scaled to the window's shorter side, never stretched, the rest canvas background; chosen in View › "F3 fills by: Extend / Fit" for the session, a view change and never an edit *(amends R-406)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `decisions.md` § "R-508 — An energy chart's `t > 1` side continues: past `t = 1` the warp's `K > K_max` is drawn and counted as an affine side, while past `t < 0`, and the `s` axis past either edge, the warp stays bounded *(closes RQ-376; amends R-471)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"

## Deliverables
- The figure filling the window with F3 off (R-406): the canvas given the shown layout's placement of the field, UV
  continuing past the shown figure's rect at the same scale (the coordinate note, A4 of R-407), and the area past the
  chart's `[0,1]²` drawn by each axis's extension type with the hatch fallback (R-407). No quad request set is fixed
  beyond what R-407 implies: a hatched pixel needs no integration.
- The axis extension types (REQ-CHART-054; chart_decoder_contract Part 3, chart_reference §5.4): each axis's declared
  type on the `Chart` trait (`crates/kernel/src/chart/`), bounded the default; the types acting before `Φ` and
  `validate`; the fallback hatching, labelled `decode_failed`; the existing charts' types (latent and flat affine, the
  sphere's spherical map θ periodic and φ pole-crossing, its exponential map and the invariant warp bounded, the mass
  simplex affine in R-407's sense, its map bilinear; a Chart-builder physical-quantity axis mapped linearly onto its
  range, one of §G7's non-periodic quantities (energy, `L_z`, virial ratio, mass ratio), affine, and one behind a
  nonlinear warp declaring nothing, so bounded, R-408's port, B1). The pixel's validity check is inverse_encode layers 2
  and 3 only (layer 1 is the encode path's). Tests `chart_extension`, `f3_fill_labels`; screenshot cases
  `01_main/f3_off_sphere`, `01_main/f3_off_invariant`.
- The area statistics' counting (REQ-CHART-056; chart_decoder_contract Part 3, R-408): each type's primary range on the
  `Chart` trait beside its type, and a classifier in `crates/kernel/src/chart/` that, for a visible pixel, gives the
  domain's end, not counted, forbidden or a counted system, in chart_decoder_contract Part 3's order, reading the chart
  (types, primary ranges, `validate`, `Φ`), not the payload label. TASK-M8-18's `forbidden_fraction` uses it. Test
  `area_stats_primary_range`.

## Acceptance tests
- `cargo xtask screenshot 01_main` (f3_off, f3_off_sphere, f3_off_invariant) on the real engine and `cargo test -p gui f3_fill_labels` — with a stain that reads no screen-lane field, on the latent chart, the shape sphere's spherical map and an invariant chart: the F3-off capture is pixel-identical to the F3-on one over the shown figure's rect; past `[0,1]²` the latent chart continues its formula, the sphere wraps in θ and crosses its poles with θ shifted by π, and the invariant chart is forbidden on every side, the canvas background in the presentation capture (R-467); every window pixel carries a labelled output, each forbidden one `decode_failed`, none dropped; F3 again restores the shown capture; the undo depth is unchanged and no SetField is sent; controls: the shown view stretched to the window fails the check over the rect, and a window pixel left unlabelled fails the label check (REQ-GUI-180, R-407).
- `cargo test -p kernel chart_extension` and Review checklist (physics reviewer) — per chart, samples past each edge of `[0,1]²`: the latent chart's z continues `z₀ + (2s−1)q₁ + (2t−1)q₂`; the sphere's `s` and `s + 1` decode equal, and `t = 1 + δ` decodes as `t = 1 − δ` with θ + π (likewise below `t = 0`); the invariant warp is hatched past every edge; the mass simplex continues and its negative-mass pixels are hatched by layer 2's narrowed simplex (buffered `mᵢ ≥ ε_m`, exactly raw `mᵢ ≥ 0`), at every sample past the edge, including one within `ε_m` of it, so the hatch starts at the edge with no band; control: a check of buffered `mᵢ > 0` alone (which passes the band) fails; a custom chart's undeclared axis is hatched past its edge; every hatched pixel is `decode_failed` and none is integrated; inside `[0,1]²` every chart's output is bit-identical to before; controls: a periodic axis built as affine, a pole-crossing axis without the half-period shift, and an undeclared axis that continues each fail; the physics reviewer checks the types and the fallback against R-407 (REQ-CHART-054). Run on the whole
  shape sphere (R-544), R-14's full `φ = π·(1 − t)` span; a custom chart's physical-quantity axis continues
  linearly and its unreachable pixels are hatched by `validate`, with a physical-quantity axis built as bounded as a
  further control (R-408's port, B1).
- `cargo test -p kernel area_stats_primary_range` and Review checklist (physics reviewer) — on the shape sphere's
  spherical map (the whole sphere), a window covering `[0,1]²` and reaching past it in `s` (a periodic duplicate) and
  in `t` (past a pole, θ shifted by π) gives the same forbidden count and the same total as `[0,1]²` alone, so each
  periodic duplicate is counted once; on the invariant warp, a window reaching past an edge gives `[0,1]²`'s count and
  total, the domain-hatched band leaving both; on the mass simplex, the window's negative-mass pixels count as forbidden
  and the forbidden share rises (R-408, B2); a custom chart's unreachable physical-quantity pixels count as forbidden;
  B5's order, on a test chart where no shipped chart has the case: a pixel where layer 2 rejects and `Φ` fails
  (non-finite) counts as forbidden, and a pixel where `Φ` fails and layer 2 accepts is the domain's end, out of the
  count and the total; inside `[0,1]²` every chart's statistic is bit-identical to before; controls: counting the
  periodic duplicate, counting a pixel past a pole, counting the domain-hatched band as forbidden, dropping it from the
  count but not the total, and a classifier running `Φ` before `validate` each fail; the physics reviewer checks the primary ranges and the classification order against R-408
  (REQ-CHART-056).
- `cargo test -p kernel chart_extension` (the outcome view) — past `[0,1]²`, every pixel the types or the fallback hatch is marked forbidden in REQ-COL-102's field, and the outcome view draws it as the canvas background, not `#141418` and not the hatch, the debug view hatching it (R-467); the field is Part 3's hatching, the primary ranges playing no part: a shape-sphere pixel past its pole draws its outcome colour, not the background, and on a test chart with a periodic axis and an affine physical-quantity axis, a pixel past one period whose state fails `validate` draws the background (physics review 5548113767 on PR #226, P1); controls: a pixel past the invariant warp's edge drawn from its `decode_failed` detail alone draws black, and a field filled from the area statistics' classifier, which leaves that pixel "not counted", each fail (REQ-CHART-054, R-446, R-467).
- `cargo test -p kernel lean_gram_factor` — REQ-CHART-071's verify (R-474 R7, the factor's control).

## Notes
- R-627 (27 Oct 2026): split from TASK-M8-05 in the plan first (applied per R-369): these are kernel physics drawn on the real figure, not a screen the mock can show, so they stay behind the engine chain. The deliverables and acceptance lines above are TASK-M8-05's, moved verbatim; its Notes on R-406, R-407, R-408, R-412, R-446, R-467, R-468, R-470 to R-472, R-474 R7, R-508, R-537, R-544 and its review rounds are their record and apply here. Its dependencies for them come with them: TASK-M7-21, TASK-M2-32, TASK-M2-34 and TASK-M6-48 (`tilt_map`, for REQ-CHART-071).
- **⚑ Physics review is required** for REQ-CHART-071 and B1's retirement (R-471, R-474), as TASK-M8-05's note says: this task does not merge on the code reviewer's word alone.
- The tasks that depended on TASK-M8-05 for the real figure (TASK-M8-18's `forbidden_fraction`, TASK-M8-44, TASK-M8-55 and TASK-M8-56 among them) depend on this task, which reaches TASK-M8-88 and TASK-M8-05.
