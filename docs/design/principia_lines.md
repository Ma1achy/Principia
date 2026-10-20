# Principia — the line system: contours and traces

**Adopted by R-476 (20 Oct 2026, the human's L1: "Build the line system as specified in `principia_lines.md`").**
This document is part of the corpus from R-476 on: where it says "proposed", the proposal is what is built, except
where R-476 or a later ruling says otherwise, and its §11's six open questions are not ruled: they are RQ-377, and
what is built meanwhile is RQ-377's. It is read with `decisions.md` § R-469 to R-477 (charts never leave the manifold,
blank non-systems, warped energy axes, tilt on every chart, the two tilt maps, save view as chart), which its §10
anticipates. Its mock-ups are in `docs/gui/reference/lines/`: `images/` (every picture of the "Principia Lines"
canvas, rendered from real data) and `canvas/` (the canvas's artboards, `.dc.html`, and its index `canvas.json`; they
reference the images as uploaded assets, `/_blob/<id>`, so read them for layout and controls with `images/`
alongside). The bundle's Python scripts and its 737 KB background (`outcome_map_preset_shape_h1_n384.bin`, §13) are
not kept: `docs/gui/reference/` holds mock-ups only, and what made them is described in §13. Like every reference
there, the mock-ups are transcribed, not copied (render_gui_spec §G13); what is mock is listed in §13.

*Design proposal, 20 Oct 2026, for review. Not a ruling. Mock-ups: the "Principia Lines" canvas. Revised the same day:
dash-dot, label position and offset, trace colours, direction arrows, trace animation, and axes (§8b).* *(Its status
as delivered; R-476 adopts it, above.)*

---

## 1. What it is

A single, generic system for drawing styled lines over the figure. It has two consumers from the start:

- **Contours**: user-defined, of any quantity or combination of quantities, at any levels.
- **Traces**: the hover trace, the pinned orbit and kept orbits (`principia_trajectory_viewing.md` §3), drawn as the
  trajectory's path through the manifold, projected onto the current chart.

Both are polylines over the figure, so they share one style library, one renderer, one label engine and one smart-ink
rule. A trace is a contour's sibling, not a separate feature.

**It is separate from the stain system.** Lines are not nodes in the stain graph and do not change any pixel's colour
value. They are drawn after the stain, over the finished figure, and read the figure only for smart ink (§6).

**Display only.** Nothing in this system re-integrates anything. Changing a line, a style or a contour set never
touches the simulation key.

### What it absorbs

| today | becomes |
|---|---|
| Overlays → Chart → grid | a contour preset: contours of the chart's own axes (`h`, `v`) |
| Overlays → Stain → `t_end` contours | a contour preset: `t_end`, read at the playhead |
| the hover trace's drawing (render_gui_spec §G2) | the hover trace, drawn by this system |

Unchanged: tile bounds and the other quadtree and integration overlays (debug views), class edges (a stain-derived
overlay), and the forbidden-region hatch toggle.

---

## 2. Where it sits

```
stain  →  display chain  →  [ LINES ]  →  egui layer (chrome, hover label, reticle)
```

- Lines belong to the **figure**, not the chrome: they stay visible with F3 and present mode, and they are in exports.
- With F3 Extend, lines continue over the extension wherever the field exists. Where a pixel is not a system (past a
  chart's edge), no contour passes through it and a trace breaks.
- One menu: **Lines ▾** beside Overlays ▾, with a count, `Alt+L` to show or hide all.

**Over the Print graph, or through it (R-487; `principia_print_graph.md` §2).** Lines are drawn **over** the Print
graph by default, so they stay crisp and readable on any medium. A line set whose style has `through_medium` (§3; a
line may set it as an override in its `LineRef`) is instead composited onto the stain's output **before** the Print
graph, so the medium draws it: a pencil contour comes out as watercolour pencil, a trace as a linocut line. Axes are a
line set, so the same switch applies. The two passes are this renderer called twice with complementary filters on
`through_medium`. Where the pass drawn over sits against the display chain, before the display scale (the Print graph
doc §2) or after the gamut clamp (R-476 M2), is RQ-395; so is what smart ink (§6) reads for each pass.
Each line set's style panel shows "Through the medium", off by default (design doc §7.3).

---

## 3. The data model

```
LineStyle {                      // a named entry in the library
  stroke:  { colour, width, opacity, cap, join }
  dash:    Solid | Dashed | Dotted | DashDot | Pattern([on, off, …])   // dotted = round caps on a zero-length dash
  outline: Option<{ colour, width }>      // drawn under the stroke, following the same dashes
  ink:     Fixed | Smart { radius, hysteresis, mode: LightDark | KeepHue }
  label:   LabelStyle
  units:   ScreenPx | PrintPt
  through_medium: bool            // R-487: drawn before the Print graph, through the medium; default false, drawn over
}

LabelStyle { font, size, format, halo: Option<{ colour, width }>, position: OnLine | Above | Below, offset_px,
             gap_under_label: bool, upright: bool, spacing }

LineRef { style: StyleName, overrides: partial LineStyle }   // a line names a style and may override fields

ContourSet {
  name, visible,
  expr:    Expression,                    // §4
  read_at: StartingState | Playhead,
  levels:  Count(n) | Step(start, step) | List([..]),  spacing: Linear | Log,  follow_view: bool
  line:    LineRef
}

Trace {
  kind:    Hover | Pinned | Kept(id),
  ic:      the decoded IC (chart, uv, sim key),
  line:    LineRef,
  length:  ToPlayhead | Window(t0, t1) | LastDt(dt),
  fade:    Option<{ floor }>,             // opacity per vertex from distance to the slice
  ticks:   Option<{ every, labelled }>,
  markers: bool,                          // start ●, collision ×, escape →, horizon ▪
  arrows:  Option<{ shape: Filled | Chevron | Barb | Notched | Dot, size_px, outline: bool,
                    every: Px(n) | Dt(dt) | PerTrace(n) }>,
  colour:  Fixed | AutoFromPalette | AlongTime(Ramp),
  animate: Static | WithPlayhead | OwnClock { speed, repeat: Once | Loop | BackAndForth },
  tail:    Option<{ dt, ghost_rest: bool, head_marker: bool }>,   // for the two animated modes
  break_at_chart_exit: bool
}

Axes {
  sides:   { bottom, left, top, right }   // each: Off | Primary | Secondary(quantity)
  frame:   Option<LineRef>
  major:   { every | count | list,  length_px, direction: In | Out | Across,  line: LineRef }
  minor:   Option<{ per_major, length_px, line: LineRef }>
  labels:  LabelStyle + { format, units: Latent | Physical }
  titles:  LabelStyle + text per side
}

Lines { styles: Library, contours: [ContourSet], traces: [Trace], axes: Axes }   // part of the exported view state
```

**Rules for styles.**

- A line refers to a style by name and may override single fields without forking it.
- Editing a style updates every line that uses it.
- The library ships with **Screen contour**, **Dashed secondary**, **Trace**, **Grid** and **Print**, and the user can
  add, duplicate and rename styles.
- `PrintPt` sizes a style in points for exports, where screen pixels mean nothing.

---

## 4. Contours

### 4.1 The expression

A small, total expression language, evaluated at every point of the slice. It has three parts.

**Variables.**

| group | names | meaning |
|---|---|---|
| Chart | `h`, `v` | the axis values, in each axis's own units (a warped energy axis gives energy, not the fraction) |
| | `s`, `t` | the raw chart coordinates in `[0,1]` |
| Latent | `z_α z_β z_q0 z_q1 z_q2 z_q3 z_μ1 z_μ2` | the 8D point at the pixel, slice and tilt applied |
| Starting state | `E K U Lz I virial m1 m2 m3 r12 r13 r23 area α β θ φ` | from the decoded IC; the payload's decoded-IC quantities (colour_composition §1) plus cheap geometry |
| Playhead | `t_end state d_min drift ftle …` | payload fields at the current playhead |

**Operators and functions.** `+ − × / ^`, `abs sqrt log exp min max atan2`, and comparisons, which give 0 or 1.

**Totality.** A pixel where the expression is not finite (division by zero, log of a negative, a non-system pixel) has
no value. No contour passes through it.

Examples: `E`; `m1/m2`; `Lz^2/(2*I*K)`; `min(r12, r13, r23)/max(r12, r13, r23)`; `h` (the grid).

A contour of a quantity that varies out of the plane simply shows where its level sets cut the slice, so lines "from
the other direction" need nothing extra. Contours of the chart's own axes on a tilted chart show how the axes have
turned.

### 4.2 Evaluation

- **Starting state (the default).** Evaluated on the chart's decode only, on a grid at screen resolution, independent of
  the quadtree. So it is exact at any zoom and any depth of refinement, and it updates live during slice, tilt, pan and
  zoom.
  - It uses the same precision path as the decode (the linearised decoder at depth).
  - It runs as a GPU pass on the shared decode, with a CPU fallback at reduced resolution.
  - During motion it may drop to half resolution and refine at rest: the same degrade-in-motion, converge-at-rest rule
    the coarse-ancestor fill uses.
- **Playhead.** Read from the resident quads' payload, so its resolution is the displayed texels, and it breaks up in
  chaotic regions, as the `t_end` mock shows. That is the field, not a drawing fault. The editor says which resolution a
  set is drawn at.

### 4.3 Levels

- **Count:** `n` levels evenly across the values in view.
- **Step:** a start value and a step.
- **List:** explicit values.
- **Spacing:** linear or log.
- **Follow the view's range:** recompute the levels as the view changes. Off by default, so levels stay put when you
  pan, and a level means the same thing everywhere you look.
- The editor shows the range in view and how many levels fall inside it.

### 4.4 Extraction

Marching squares on the evaluation grid, with linear interpolation along cell edges. Saddle cells are resolved by the
cell-centre value. A cell with a non-finite corner yields no segment. Segments are joined into polylines before
styling, so that dashes and labels run continuously along the whole line.

---

## 5. Drawing a line

- **Polylines in screen space, dashes by arc length.** The dash pattern continues along the whole line instead of
  restarting per segment. The mock-up's first cut restarted it, and dashed lines rendered solid.
- **Outline first, then stroke,** with the same dash pattern and phase. Dash lengths do not scale with width, so a wider
  outline stays aligned with its stroke.
- **Opacity per vertex** (traces): consecutive vertices of equal ink and quantised opacity are drawn as one run, which
  keeps dashes continuous.
- **Units:** screen px on screen, pt in a print export.

---

## 6. Smart ink

An option on any style, for both lines and labels.

1. Take the figure **under the lines** (stain plus display chain) and compute its WCAG relative luminance.
2. Average it over a disc of `radius` px around each vertex, so one stray pixel never flips the ink.
3. Compare in log contrast against the tie point, where light and dark ink give equal contrast. For the default inks
   `#F5F4F0` and `#111114` that is relative luminance 0.179.
4. Switch only when the contrast favours the other ink by more than `hysteresis`, so the ink does not flicker on a
   mottled edge or while panning.
5. The outline takes the opposite ink.

**Modes.**

- **LightDark** (shown in the mock-ups): two fixed inks.
- **KeepHue:** keep the line's own hue and move only its OKLab lightness to reach a target contrast. For coloured
  contour sets that must stay distinguishable.

Measured on the mock-up's background: white on the green collision basin is 2.5:1, which is why plain white lines
weaken there. Smart ink draws dark there and light over the black and blue regions.

---

## 7. Labels

- **Placement.** Placed along each line every `spacing` px of arc length. Each label is rotated to the local tangent and
  kept upright.
- **Collisions.** A label that would overlap one already placed is skipped. Placement runs across all contour sets in
  list order, so the top set wins.
- **Position.** On the line, above it or below it, with an offset in px. "Above" is measured from the label's upright
  frame, so it stays on the same side as the text turns. A plain style with no halo needs an offset to be readable:
  the mock-up shows digits crossed by the line when they sit on it.
- **Gap under the label.** An option that breaks the line under its label, so text and line do not cross. Only
  meaningful for labels on the line.
- **Format.** Format string, precision and units, with a proper minus sign.
- **Halo.** Colour and width.
- **Trace labels.** A trace's time ticks are labels in the same engine.

---

## 8. Traces

The trajectory comes from `computeIC`: the full CPU f64 state per step (`principia_trajectory_viewing.md` §1). Drawing
it:

- **Projection.** Each state is run through the chart's forward map and read in the chart's own coordinates (§3 there).
  On a shape chart this is exact: the state's shape maps back onto `(z_α, z_β)`.
- **Fade off the plane.** Opacity per vertex falls with the state's distance from the slice, down to a floor (0.18 in
  the mock). See §11 Q3 for what "distance" should be in general.
- **Breaks.** The line breaks where the path leaves the chart and resumes where it comes back. It is never joined across
  the outside.
- **Markers.** Start ●, collision ×, escape →, horizon ▪. Optional time ticks every `Δt`, labelled.
- **Colour.** One colour; or automatic, where each new pinned or kept trace takes the next colour of a trace palette,
  any of them settable by hand; or a ramp along time from start to end.
- **Direction arrows.** Optional arrowheads along the path. Shape: filled triangle, open chevron, half barb, notched or
  dot. Size in px. They follow the line's outline. Frequency every N px of path, every Δt of time, or N per trace.
  Spacing them in time makes them bunch where the motion is slow, so speed can be read off the line.
- **Animation.**
  - Static: the whole path at once.
  - With the playhead: a head marker sits at the playhead and moves as time is scrubbed or played, using the Time
    panel's own play and loop.
  - Own clock: the trace plays over its own window at its own speed while the figure stays put: once, looped, or back
    and forth.
  - Both animated modes take a tail (a bright stretch of Δt behind the head), an option to show the rest of the path
    faintly, and a head marker labelled with its time.
- **Length.** To the playhead (the default), a fixed window, or the last `Δt`.
- **Incomplete runs.** When the hover's step budget runs out (the pointer-channels note), the drawn part is marked
  incomplete, e.g. its tail dashed, rather than drawn as if it were the whole trajectory.
- **Kinds.**
  - **Hover:** follows the cursor.
  - **Pinned:** the clicked IC (R-453).
  - **Kept orbits:** each has its own line, sharing a style by default, colour overridable.

---

## 8b. Axes

The frame, ticks, tick labels and axis titles are lines and labels too, so they take styles from the same library and
can be restyled like any contour.

- **Sides.** Each side is off, the primary scale, or a secondary scale in another quantity.
- **Primary scale.** The chart's own coordinates: latent units on a latent chart, physical units on a physical axis. A
  warped axis's ticks fall at round values of the quantity, so they are unevenly spaced.
- **Secondary scale.** Any quantity that varies monotonically along that edge, placed by solving the chart map. The
  mock-up's print panel puts the shape angles α and β on the top and right: 30°–60° and 60°–120°, unevenly spaced because
  the link is a sigmoid. A secondary scale is refused where the quantity is not monotonic along the edge.
- **Ticks.** Major ticks by step, count or list; inward, outward or across the edge; with their own length. Optional minor
  ticks, given as a number per major tick. A label stride labels every n-th major tick, so density and legibility are
  set separately. Labels can sit outside the figure or inside it on a halo.
- **Frame and labels.** The frame is optional. Tick labels take a format and a style; titles are labels with text.
- **Presets.** Screen (the dev default: no frame, outward ticks, mono labels), Print (frame, inward and minor ticks,
  paper-halo ticks, titles), and Ends only (today's GUI: just the range at each end and the axis name). The canvas's
  variants board shows eight settings: sparse, default, dense minor, count with label stride, four sides framed, top and
  right across, inside the figure, and ends only.
- **Grid.** The Grid preset (§1) can take its spacing from the major ticks, so grid lines and ticks always agree.
- **Chrome.** Axes hide with the chrome under F3 and present mode by default. An option keeps them in exports.

---

## 9. Persistence, undo, provenance

- **Saved with the view.** Contour sets, traces' settings and the style library used are part of the exported view
  state, so a print reproduces its lines.
- **Undo.** Edits are undoable view edits with no re-integration. Hovering is never recorded.
- **Personal library.** The style library is also kept per user. A shared view embeds the styles it uses, so it renders
  the same for someone without them.

---

## 10. How it fits today's other decisions

- **Charts never leave the manifold.** Contours never pass through non-system pixels, and traces break there.
- **Warped energy and Lz axes.** A constant-energy contour on a warped chart is how you read absolute energy. This is the
  "linear reading" without a linear axis.
- **Tilt and slice everywhere (option (b)).** Starting-state contours re-evaluate live because they need only the decode,
  and traces re-project.
- **F3 Extend and Fit.** Lines are part of the figure in both modes.
- **RQ-340.** Unaffected: lines read whatever plane the chart defines.

---

## 11. Open questions

*Queued as RQ-377, parts (1) to (6), one per question (R-476, L5: "Queue them as RQ entries; do not rule on them"); what is built meanwhile is RQ-377's.*

1. **Expression language.** A purpose-built mini-language (proposed; total, sandboxed, GPU-compilable), or reuse the
   stain graph's Code view syntax?
2. **Playhead quantities.** Allow any payload field, or a curated list?
3. **Off-plane distance for the trace fade.** The general definition is the distance in z from the trajectory's encoded
   point to the slice plane, which needs the encode at every step (and the metric RQ-341 raises). The mock-up uses a
   proxy: the scale-normalised Jacobi momentum magnitude, which this shape chart holds at zero.
4. **Label collisions.** Between contour labels and the trace's tick labels: which wins?
5. **Class edges.** Should they move onto the line system as "contours of a categorical field", or stay in the stain?
6. **Default style for new contour sets.** Screen contour (white with outline), or smart ink on by default?

---

## 12. Tests (each with an arm that can fail)

- **Exactness.** A contour of `h` lies exactly on the axis ticks. Control: a deliberately offset expression `h + 0.001`
  does not.
- **Totality.** No contour segment is emitted in a cell with a non-finite corner. Control: a field with an injected NaN
  cell must show a gap there.
- **Dash continuity.** Measured dash lengths along a long dashed contour are constant. Control: the per-segment
  renderer fails it, as the mock-up's first cut did.
- **Smart ink.** Over a split background, the ink differs on the two sides and stays put within a hysteresis band.
  Control: hysteresis 0 flickers on a mottled strip.
- **Trace projection.** On the shape chart, a trace's first vertex lands on the hovered pixel exactly.
- **Provenance.** A print re-opened reproduces its contour sets and styles bitwise.
- **No re-integration.** Changing any line setting leaves the simulation key and quad requests unchanged.

---

## 13. About the mock-ups

**Real:**

- **Background.** prin-rs `outcome_map` on `preset_shape_h1` (latent `z_α × z_β`, centre 0, half 1, equal masses,
  released from rest), 384² at `t_max = 20`, Heggie, `r_coll = 1e-3`, closure escape, in the agreed outcome palette.
- **Contours.** Computed from the decoder's own formulas: E is the potential at rest; area is the shape sphere's signed
  area coordinate.
- **Trace.** An RK4 integration of the IC at one pixel, mapped back onto `(z_α, z_β)` through the inverse of the angle
  links.
- **Smart ink.** Uses the WCAG rule above.

**Mock:**

- The panels and controls.
- The trace's off-plane proxy (§11 Q3).
- The renderer, which is Python and matplotlib, not the engine.
