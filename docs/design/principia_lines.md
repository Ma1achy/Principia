# Principia — the line system: contours and traces

**Adopted by R-476 (20 Oct 2026, the human's L1: "Build the line system as specified in `principia_lines.md`").**
This document is part of the corpus from R-476 on: where it says "proposed", the proposal is what is built, except
where R-476 or a later ruling says otherwise, and its §11's six open questions are RQ-377, ruled by R-509 (§11).
*Was:* "and its §11's six open questions are not ruled: they are RQ-377, and what is built meanwhile is RQ-377's." Its §3's `Trace` and §8 are amended by R-538 (the trace's markers and icon
sets, syzygies, off-screen pinning, repeats and not-a-system handling, and the trace's defaults). It is read with `decisions.md` § R-469 to R-477 (charts never leave the manifold,
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
- One menu: **Lines ▾** beside Overlays ▾, with a count, `Alt+L` to show or hide all. The traces are not in it: they
  are in Overlays ▾ › Trace, the Trajectory panel and the `Trace…` window (R-541; render_gui_spec §G2, §G5), and
  Lines ▾ holds the contour sets, the axes, the grids and the presets (R-556, RQ-448 (a)).

**Over the Print graph, or through it (R-487; `principia_print_graph.md` §2).** Lines are drawn **over** the Print
graph by default, so they stay crisp and readable on any medium. A line set whose style has `through_medium` (§3; a
line may set it as an override in its `LineRef`) is instead composited onto the stain's output **before** the Print
graph, so the medium draws it: a pencil contour comes out as watercolour pencil, a trace as a linocut line. Axes are a
line set, so the same switch applies. The two passes are this renderer called twice with complementary filters on
`through_medium`. The pass drawn over keeps this section's place, after the gamut clamp and before the colour-vision
simulation (R-476 M2), and the Print graph doc §2's diagram is corrected to match; smart ink (§6) for a line drawn over
reads the figure to the clamp, the Print graph's output included, and for a line drawn through the stain's output
(R-531, ruling RQ-395). *Was:* "Where the pass drawn over sits against the display chain, before the display scale (the
Print graph doc §2) or after the gamut clamp (R-476 M2), is RQ-395; so is what smart ink (§6) reads for each pass."
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
  markers: { icon_set: Principia | Arrows | Pictures | Orbits, icon_size, end_icon: bool, reach_rings: bool,
             syzygy: Option<{ style: FromSet | ThreeInARow | LineAndBead | NumeralTag | CrossingTick | Eclipse,
                              fade_stutters: bool }>,
             edge_pin: bool, window_chevrons: bool, seams: bool },   // §8's markers and icon sets (R-538)
  join_jumps: bool,                       // a dotted line across a break (R-538)
  arrows:  Option<{ shape: Filled | Chevron | Barb | Notched | Dot, size_px, outline: bool,
                    every: Px(n) | Dt(dt) | PerTrace(n) }>,
  colour:  Fixed | AutoFromPalette | AlongTime(TimeRamp),   // TimeRamp: §8's ramp along time (R-538 TF4)
  animate: Static | WithPlayhead | OwnClock { speed, repeat: Once | Loop | BackAndForth },
  tail:    Option<{ dt, ghost_rest: bool }>,   // "Tail only", for the two animated modes (R-538: off by default)
  head_marker: bool,                      // labelled with its time (R-538: on, with or without a tail)
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

*`Trace` as R-538 amends it (§8):* `markers` is the icon set and its toggles, `join_jumps` is new and the head marker
no longer sits under `tail`, since its default is on with the tail off. *Was:* "`markers: bool, // start ●, collision ×,
escape →, horizon ▪`" and "`tail: Option<{ dt, ghost_rest: bool, head_marker: bool }>, // for the two animated modes`".
`AlongTime` takes a `TimeRamp { preset: Sunset | Viridis | Magma | Ice | Ember | Aurora | Mono | Custom([c₀, c₁, c₂]),
reverse: bool }`, the line system's own type, not the stain graph's `Ramp` (R-538 TF4, J12). *Was:* "`colour: Fixed |
AutoFromPalette | AlongTime(Ramp),`".

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

1. Take the figure **under the lines** (stain plus display chain) and compute its WCAG relative luminance. For a line
   drawn over, that is the figure to the gamut clamp, the Print graph's output included; for a line drawn through the
   medium, it is the stain's output, before the Print graph (R-531).
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
  On a shape chart this is exact: the state's shape maps back onto `(z_α, z_β)`. **Per shown axis, in order (R-538
  TF2):** each sample's chart parameters are taken on the branch nearest the previous sample's (θ ± 2π; over a pole
  θ + π with φ reflected), every coordinate of a mix read in the frame the chart's own parameters are defined in (the
  full shape sphere's `(θ, φ)` unmirrored, `ρ` along `+x`; the latent chart's canonical); then the axes are mixed (a
  tilt, R-473); then the point is drawn in every image by the extension types. The trace never runs the inverse
  policy's lookup and lock validation (inverse encode Part 6) on a sample: it encodes the shown coordinates without
  clamping and checks each shown axis on its own. On a tilted chart the θ branches are not repeats of one image: θ
  wraps but the tilt target does not, so the chart is affine along that axis and does not repeat.
- **Fade off the plane.** Opacity per vertex falls with the state's distance from the slice, down to a floor (0.18 in
  the mock). See §11 Q3 for what "distance" should be in general.
- **Breaks.** The line breaks where the path leaves the chart and resumes where it comes back. It is never joined across
  the outside by default; an option, **join jumps** (off), joins a jump with a dotted line (R-538).
- **Markers and icon sets (R-538).** One selector, **Icon set**: **Principia** (the default), Arrows, Pictures and
  Orbits, each giving every marker below; Principia is Arrows with Orbits' two rings for leaving and re-entering the
  reach. Every colour is the outcome palette's (colour_composition §1.4, the active preset), so an icon and the pixel
  under it agree: collisions 1–2 `#DE2D2D`, 1–3 `#2EBC4E`, 2–3 `#3462E0`, triple collision `#F29620`; escapes body 1
  `#F0DE32`, body 2 `#E034C6`, body 3 `#30C8DC`, triple ejection `#611597` (the sRGB default). Icons are drawn over a
  dark halo `#0B0B0E` with light ink `#F5F4F0`, so they read on any field; bodies are shown 1, 2, 3 (R-449).

  | Marker | When | Principia | Arrows | Pictures | Orbits |
  |---|---|---|---|---|---|
  | **Start** | `t = 0` | a five-pointed star in the trace's colour | star | the initial triangle in body colours | star |
  | **Collision** | the run ends in a pair collision | two arrows meeting at a burst in the pair's colour, oriented along the trace | the same | two discs touching | a merger: the pair as one point in a flash ring |
  | **Triple collision** | two or more pairs inside `r_coll` (`detail = 3`) | three arrows into a burst, `#F29620` | the same | three discs touching | three discs in a flash ring |
  | **Escape** | one body escapes | a disc in the escaping body's colour with an arrow leaving it, along the trace | the same | a streaking disc | a bound pair (ellipse) left behind, the body leaving |
  | **Triple ejection** | all three unbound (`detail = 3`) | three discs `#611597`, each with an arrow pointing out | the same | three discs flying apart | three dotted streaks out |
  | **Horizon** | nothing happened by `t_max` | an hourglass | hourglass | hourglass | a closed orbit (an ellipse with a body on it) |
  | **Incomplete** | the hover's step budget ran out | "?"; the last stretch of the line dashed | ? | ? | ? |
  | **Leaves the chart's reach** | a coordinate entering a shown axis with non-zero weight has no value (below, "Where a trace can leave") | a dotted ring; the line dashed until it re-enters | an arrow into a bar | an open ring | a dotted ring |
  | **Re-enters the chart's reach** | it comes back | a ring with a centre dot | a bar, then an arrow out | a filled ring | a ring with a dot |
  | **Leaves / enters the window** (optional, off) | the line crosses the view's edge | a chevron at the edge | | | |
  | **Seam** (optional, off) | θ wraps | a short tick at both ends of the jump | | | |
  | **Syzygy** (off by default) | the three bodies are collinear | o–o–o | o–o–o | three in a row | an eclipse |

  There is no double escape: it is not an outcome class (two bodies leaving a third is triple ejection, `detail = 3`).
  The end event icon can be switched off. *Was:* "**Markers.** Start ●, collision ×, escape →, horizon ▪. Optional time
  ticks every `Δt`, labelled." The leaves row's *Was* (R-538 TF1): "the state leaves what the chart can show (latent: a
  momentum past `q_max`, or α past its limits)".
- **Where a trace can leave (R-538 TF1).** The trace point is the state projected onto the chart's actual axes.
  Hidden coordinates never make a trace leave: anything orthogonal to the chart's plane feeds only the off-slice
  distance, the fade (where it has no value, R-509: no fade, and the GUI says so). A trace leaves only where a
  coordinate entering a shown axis with non-zero weight has no value: ±∞ from a saturated link, NaN, or a bounded
  axis's inverse outside its domain. Moving off the slice is never leaving. So it depends on the chart's axes, not its
  name:

  | Shown axis | Can it make the trace leave? |
  |---|---|
  | θ or φ of the shape sphere, alone or in any mix | No: `n̂` exists for every shape with `I > 0` (`I = 0` is a total collision, which has ended the run); θ's pole singularity is passed as a jump |
  | a mass or mass ratio | No: masses are constant along a run |
  | a physical quantity (`E`, `L_z`, `K`, a momentum, the dilation rate) | No, unless its warp's inverse has no value (`K ≥ 0` always; R-508 continues `t > 1`) |
  | a curve axis (Burrau's ν) | No: the encode projects onto the nearest point of the curve (inverse encode Part 5(b)) and the distance to the curve is fade (R-509). A point whose nearest ν lies past a bounded edge is not drawn there, the line broken, with no ring; it is a real system the axis does not show, counted in the readout's "past the domain's end: N samples", apart from the not-a-system count (RQ-425 (a), R-553) |
  | a latent coordinate behind a saturating link (`z_q` past `q_max`, `z_α` past the decoder's α limits) | Yes, exactly when it has weight in the shown axis: on the latent momenta chart, or a tilt toward `z_q`; never on the latent `z_α × z_β` chart through a momentum |
  | `z_β` at a syzygy | It runs to ±∞ and comes back: a jump at infinity, not a sample off the chart |

  A tilted axis is a mix, a weighted sum of the chart's parameters and the tilt target (R-473), and leaves only through
  a term with non-zero weight: tilted toward a latent momentum `z_q` (weight `sin τ`), the trace leaves while the
  target component's own `|q₃| ≥ q_max` (`= |p_λ,y|`, REQ-DEC-019; RQ-424 (a), R-553); tilted toward the physical momentum, it
  never leaves. *Was:* "the target's `|q| ≥ q_max` (RQ-424)".
- **Time ticks.** Every `Δt`, labelled; by default every 2, labelled (R-538).
- **Syzygies (R-538), off by default.** A syzygy is an instant at which the three bodies are collinear: the
  configuration crosses the shape sphere's equator, `w = 0` (R-14). Each has a middle body; the sequence of middle
  bodies is the syzygy word (Montgomery), and the same letter twice running is a **stutter**, drawn faded unless "fade
  stutters" is off. The marker sits where the line actually crosses `w = 0`, interpolated between the two samples on
  either side, never at a sample. Principia's and Arrows' o–o–o is three discs on a short line in the body colours (each
  body's escape colour, R-538 I3), in the order the bodies lie on the line (the outer pair lowest-numbered first, the
  middle body in the centre), the line `#D6D3CB`, the discs dark-haloed, no other dots and no rings; the other styles
  (Three in a row, Line and bead, Numeral tag, Crossing tick, Eclipse) stay selectable, the selector defaulting to "from
  the icon set". On a chart that carries the y-momenta the canonical frame mirrors at every syzygy, so the trace jumps
  there and a marker is drawn at both ends of the jump.
- **Off-screen icons (R-538).** An icon whose point is outside the view is drawn faint, pinned to the nearest point just
  inside the view's edge, by default; "Off-screen icons: pin to the edge (faint). Untick to hide them." Window
  chevrons (off) mark where the line crosses the view's edge. Pins are events, chevrons the line.
- **Repeats (R-538, R-537).** The trace works wherever the view goes. On an affine chart it runs on past the square
  like the field. On a periodic, pole-crossing or fold chart (the shape sphere, its hemisphere) it is drawn in every
  image of the chart in view, with every marker and icon (start, events, rings, ticks, arrows, head), at half strength (RQ-412 (a), R-553);
  repeats are never pinned to the edge, only the primary copy pins, so the edge does not fill with duplicates. A hover
  or click in a repeat integrates the canonical system, so the trace is identical in every image. With the hemisphere
  toggle on (R-412) the trace folds at the equator and its repeats continue into the mirror hemisphere.
- **Not a system (R-538).** Every sample of the trajectory is encoded into the chart; a sample whose chart point is not
  a system (fails `validate`'s layers 2 and 3, on the shown axes after the types fold the point, never Part 6's ladder on the whole state, RQ-429 (a), R-553) is
  not drawn,
  the line breaking there. A real trajectory cannot enter such
  a region (masses constant, energy conserved), so the readout's count of such samples, "not a system: N samples" (reading "in forbidden: N samples" only
  while a debug view is the figure, RQ-417 (b), R-553 K1; *was:* "in forbidden: N samples" in every view),
  reads 0, and a non-zero count is reported as an encode or chart-map bug, a console warning. A trace is never started
  in such a pixel: a click there integrates nothing and draws a red ✕ where it was clicked (R-537), and hover shows the
  readout and no trace. On the mass simplex a run's chart point never moves (its masses are constant), so its trace is
  a fading dot, the dot → line → curve rule (`principia_trajectory_viewing.md` §3) for a conserved axis.
- **Colour.** One colour; or automatic, where each new pinned or kept trace takes the next colour of a trace palette,
  any of them settable by hand; or a ramp along time from start to end. **By default (R-538)** the hover and pinned
  trace is a fixed `#FFF27A`, and kept traces take the trace palette in turn: `#7FE3FF #FF9EC4 #B7F28A #FFB86B #C9A7FF
  #9EF0D2`. Width 2.2 px; icon size 1.4×.
- **The ramp along time is selectable (R-538 TF4).** Presets: **Sunset** (the default once the ramp is chosen,
  RQ-422 (a), R-553, `#FFF27A #FF8A5B #E0457B #7A4CFF #3BC7FF`), Viridis, Magma, Ice, Ember, Aurora and Mono; **Custom**, three
  colour stops; **Reverse**, which reverses the ramp; and a preview bar that shows it. The stops are evenly spaced and
  interpolated linearly, over `t` from 0 to the horizon `t_max` (R-538 J12). It colours the line and its time ticks,
  and the marks drawn in the trace's colour (the start star, the direction arrows) at their own time; the event icons
  keep the outcome palette. It is part of the trace style, so it is kept with kept orbits, saved with the view and
  travels in exports (the round-trip rule's tier 2; RQ-428 (a), R-553: a kept orbit's `Trace` is `Lines` state, its tab `ViewUI`). It is not the field's palette: a Chazy or outcome palette
  is a stain (the node graph); this ramp is the line system's.
- **Direction arrows.** Optional arrowheads along the path. Shape: filled triangle, open chevron, half barb, notched or
  dot. Size in px. They follow the line's outline. Frequency every N px of path, every Δt of time, or N per trace.
  Spacing them in time makes them bunch where the motion is slow, so speed can be read off the line.
- **Animation.**
  - Static: the whole path at once.
  - With the playhead: a head marker sits at the playhead and moves as time is scrubbed or played, using the Time
    panel's own play and loop. **When the playhead is not playing and is not being scrubbed (R-538 TF3)**, every trace
    of the default length (to the head) is drawn from `t = 0` to its end (its event, or the horizon), with no head
    marker, and a `Window` or `LastDt` length is kept as chosen (RQ-427 (a), R-553); while playing, or
    while the scrubber is dragged, it is drawn to the head, with the head marker. A step while paused is neither
    (R-538 J11). *Was:* "every trace is drawn from `t = 0` to its end" (gui review 5594407176 of PR #263, G2; applied
    per R-369).
  - Own clock: the trace plays over its own window at its own speed while the figure stays put: once, looped, or back
    and forth. It is always animated (R-538 TF3); static draws the whole trace, as above.
  - Both animated modes take a tail (a bright stretch of Δt behind the head), an option to show the rest of the path
    faintly, and a head marker labelled with its time. **By default (R-538)** the tail is off ("Tail only", Δt 3 when
    on), "show the rest faintly" is off, and the head marker, labelled with its time, is on.
- **Length.** To the playhead (the default), a fixed window, or the last `Δt`. The default draws everything from `t = 0`
  to the head while the playhead is played or scrubbed, and to the trace's end otherwise (R-538 TF3); a fixed window
  or the last `Δt` is kept as chosen, and a finished own-clock "Once" holds its last frame, the whole trace (RQ-427 (a),
  R-553). *Was:* "The default draws everything from `t = 0` to the head (R-538)."
- **Incomplete runs.** When the hover's step budget runs out (the pointer-channels note), the drawn part is marked
  incomplete, rather than drawn as if it were the whole trajectory: its end icon is "?" and its last stretch dashed
  (R-538). A step size that is not finite ends the run at once with the label `sim_failed`, as the integrator contract
  says, and its trace ends with the same "?" icon; "incomplete" means only that the hover's step budget ran out
  (RQ-418 (a), R-553). *Was:* "the drawn part is marked incomplete, e.g. its tail dashed, rather than drawn as if it were the whole
  trajectory".
- **Kinds.**
  - **Hover:** follows the cursor.
  - **Pinned:** the clicked IC (R-453).
  - **Kept orbits:** each has its own line, sharing a style by default, colour overridable. Each is a tab of the
    Trajectory panel, with its own previews, trace and sound (R-541 G4).
- **The look (R-541 G9).** Every art board and the build draw the trace one way, these defaults: the line `#FFF27A`,
  width 2.2, over the style's `outline` (§3), dark, of width `w + 2.2`, so it reads on any stain (the demo's halo);
  time ticks every 2, labelled; filled-triangle arrows every Δt 1.0, 8 px; the start a yellow star; the head a white
  disc labelled with its time; icons at 1.4×; syzygies off. Kept traces take R-538's trace palette in turn, each checked against the body colours and the
  stain classes, a failing colour alone replaced; the boards' `#5ab4ff` is never a trace colour (R-556, RQ-447 (b);
  REQ-GUI-333).
  While Listen is on, the hover or tab's trace on the shape-sphere preview is drawn brighter inside Listen's window
  (render_gui_spec §G2; `principia_listen.md` §4).

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

*Queued as RQ-377, parts (1) to (6), one per question (R-476, L5: "Queue them as RQ entries; do not rule on them"), and ruled by R-509 (20 Oct 2026), the human: "1 reuse the stain's expression syntax; 2 any ledger-declared payload field; 3 proxy where defined, else no fade and say so, the real distance waits on RQ-341; 4 trace labels win; 5 class edges move to the line system; 6 Screen contour default, smart ink opt-in." Where §1 keeps class edges in the stain, §4.1's expression is a mini-language, §7 orders labels and §8 fades a trace, R-509 holds. Was: "; what is built meanwhile is RQ-377's."*

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
- **Trace markers (R-538).** `principia_trace_markers.md` §11's tests T1 to T12, each with its arm, are in the
  requirements R-538 adds (REQ-GUI-271 to REQ-GUI-276, REQ-INT-096). The follow-up's T14 to T20 and T16b (R-538 TF6),
  each with its arm, are in REQ-GUI-277 to REQ-GUI-282 and REQ-CHART-080.

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
