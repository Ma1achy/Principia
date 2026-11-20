# Past the square, with the controls shown: navigation, interaction, rendering, quadtree and deep zoom

*Written 20 Oct 2026 for the orchestrator. It is the human's instruction; quote §0 verbatim in whatever rulings it
becomes, and port the rest as the applied text. The working demo is `demo/trace_markers.html` in this bundle (tabs: latent
`z_α × z_β`, shape sphere, mass simplex, latent momenta). Read against `origin/main` at 2dc67e4 and the open rulings
branch `rulings/R-469` (R-469 to R-477, R-506 to R-511).*

---

## 0. My words

> "Doesn't the first latent chart also have valid extensions?"

> "Also on things like the shape sphere with the domain wrap all of the icons etc should repeat"

> "Ok this stuff is more general not necessary to do with the trace: you need to be able to interact with the
> extended / domain wrapped regions.
> You should be able to zoom / pan into the extended / wrapped regions even when you haven't explicitly hidden the gui
> with f3."

> "Also yeah by if the viewport extends into a domain wrapped regions / extended region it renders. Never any gaps."

> "Also quad tree and refinement needs to work with the extended and wrapped regions correctly. Along with deep zoom.
> Ensure there's no issues / gaps"

---

## 1. What this changes

Today the corpus reaches past `[0,1]²` **only when the chrome is hidden**:

- R-406 and R-407: the extension appears "with the egui layer hidden (F3) or in present mode".
- The coordinate note: UV is "beyond `[0,1]` only in the window's extension".
- The deep-zoom doc's "Past `[0,1]²` (R-407)" bullet: "With the figure filling the window (F3 off or present mode …)".
- The decoder contract, Part 4: "Filling the window is not a gesture."

This instruction removes that scoping. **The extension is a property of the chart, not of F3.** Pan and zoom reach it
with the controls shown. Wherever the view lands it renders, and it never has gaps. The quadtree, refinement and deep
zoom work there exactly as they do inside the square.

What stays as it is:

| Ruling | What stays |
|---|---|
| R-407 | The four extension types and the validity fallback |
| R-408 | The area statistics: each system counted once, through the primary ranges |
| R-412 | The hemisphere toggle |
| R-468 / R-472 | F3's two modes |
| R-469 | Navigation stops at the edge, judged on the centre |
| R-470 | Not-a-system pixels are blank; the hatch is an overlay |

| Ruling or doc | Change |
|---|---|
| R-406, R-407 clause 2's context, R-472 X2 | The extension is drawn wherever the view reaches, chrome shown or hidden. F3 Extend only adds window area; it is no longer the only way to see past the square. |
| decoder contract Part 3, "Past the unit square" | The opening sentence ("with the egui layer hidden (F3) or in present mode") becomes "wherever the view reaches past `[0,1]²`: by pan, by zoom out, or by F3 Extend". |
| decoder contract Part 4 | Pan and zoom may take the view's window past `[0,1]²` on every chart. The only stop is R-469's, on the centre. "Filling the window is not a gesture" stands for F3. |
| coordinate note, deep_zoom "precision split" | UV and quad indices outside `[0,1]` / the depth-0 root arise from navigation too, not only from F3. |
| caching contract Part 4 | The baseline cover is the coarsest cover of the whole viewport, extension included (EX4). |
| `principia_lines.md` §8 | Traces repeat in every image of the chart (see `principia_trace_markers.md`). |

---

## 2. Requirements

**EX1. The view is a window onto the chart's extended parameter plane.** With the controls shown or hidden, pan
moves the window and zoom scales it, past `[0,1]²` in any direction.

- F3 Extend shows more of the same window at the same scale (R-406).
- F3 Fit scales the window's square to the screen (R-468). Its "square" is the view's own square, wherever the view
  sits, not the chart's `[0,1]²`.
- Nothing about what a pixel means changes with F3.

**EX2. What is drawn past the square is the axes' types, as now.**

- **Affine** (the latent charts, flat slices, body planes, tilted or not): the plane continues, and every pixel is a
  system. This is the latent chart's case. It has always had a valid extension; the demo only lacked a map there.
- **Periodic** (θ): the chart repeats.
- **Pole-crossing** (φ): the chart continues over the pole, with θ shifted by half its period.
- **The hemisphere's fold at the equator** (R-412): the mirror hemisphere, the same systems.
- **Bounded, or a validity failure**: not a system. It is blank in every view (R-470); the hatch appears only through
  Overlays › Chart › forbidden region, which is off by default.

**EX3. The only navigation limit is R-469's.**

- Pan, zoom, tilt, slice and lock are refused only when the **centre** would fail `validate`.
- **Periodic and pole-crossing axes:** `validate` is evaluated on the centre after the types have folded it, so a pan
  across a seam or a pole never stops.
- **Affine latent axes:** no stop at all.
- **Bounded charts** (the warp, the mass simplex, the Burrau family): the centre stops at the domain's edge, but the
  window may still show blank past it. In the demo's simplex tab, a drag toward a corner stops the centre at `(0, 1)`
  and the far side is blank.

**EX4. Never any gaps.** Every visible point of the window is drawn, at whatever resolution is available.

- Until a quad is computed it shows its nearest computed ancestor, as caching contract Part 4 and Part 5 already
  require for the square ("panning never blanks").
- The only blank pixels are not-a-system pixels (EX2).
- Raw canvas background where a system exists is a bug, never a loading state.
- The baseline cover (caching contract Part 4) is the coarsest cover of the whole viewport, extension included. Its
  ancestor chain is evict-exempt there as everywhere.

**EX5. Every interaction works at every visible pixel.** That means hover, click and pin (R-453), lock, the IC
inspector, the trace, keep, readouts, comments and the console's pick.

- **A pixel in a repeat** (periodic, pole, fold) resolves to its **canonical** chart point, the one inside the
  primary range, and so to the same system.
  - The readout shows the canonical coordinates and labels the pixel a repeat.
  - The trajectory, outcome and inspector are those of the canonical system.
- **A pixel on an affine extension** is simply that point of the plane.
- **A click on a not-a-system pixel** integrates nothing and draws a red ✕ where it was clicked.
  - The ✕ is a transient pick marker, not a pixel colour, so it is outside R-470.
  - The readout says "not a system", names the check that failed (for example "a mass ≤ 0") and gives the label
    (`decode_failed`, its detail).
  - Hover over such a pixel shows the same readout and no trace.

**EX6. The trace repeats.**

- In every image of the chart that is in view, the hovered, pinned and kept traces are drawn with every marker and
  icon (`principia_trace_markers.md` §7).
- Repeats are drawn fainter and are never pinned to the view's edge.

**EX7. The quadtree covers the extended plane, and one system is computed once.**

- **(a) Roots tile the plane.** The level-0 cells of the root lattice tile the whole parameter plane. Every cell the
  view touches has a root, so every visible point has an ancestor and EX4 can hold.
  - Quad indices outside the depth-0 root are negative or exceed `2^L − 1`, as the coordinate note already allows.
  - The address stays in the slice plane's own frame (R-97).
- **(b) The canonical address.** A quad in a repeat maps to a quad in the primary range by the type group. That quad
  is the one requested, computed, cached and decided; the repeat draws it.
  - On the shape sphere at level `L`, with `n = 2^L` and R-14's `φ = π(1 − t)`:
    - **θ period:** `ix' = ix mod n`.
    - **Pole `φ = 0` (`t = 1`):** `iy' = 2n − 1 − iy`, and `ix' = (ix + n/2) mod n`.
    - **Pole `φ = π` (`t = 0`):** `iy' = −1 − iy`, and `ix' = (ix + n/2) mod n`.
    - **Hemisphere fold at the equator** (R-412, equator at `t = 0`): `iy' = −1 − iy`, `ix` unchanged.
  - Compose these until the address is inside the root.
  - Every map is quad-exact at `L ≥ 1`. At `L = 0` the half-period shift splits the root, so **a level-0 repeat
    across a pole is drawn from its two level-1 halves** (Q5).
  - Reflections reverse the order of the footprint grid along the reflected axis. `N` is a power of two (R-398) and
    the sample positions `t = (i + ½)/N` are symmetric, so the reflected grid lands exactly on the canonical one. The
    repeat therefore draws the canonical samples mirrored and re-integrates nothing.
- **(c) Scheduling reads the union of images.** A canonical quad is in view if any of its images is. Its screen size
  is its images' size; all images share the zoom, so the screen floor reads the same number.
  - It is decided once. Its children are shared by every image.
  - The frame quota counts it once.
  - Payload, timeline and playhead are shared, so every image shows the same frame.
- **(d) Neighbours cross seams, poles and folds through the same map.** This applies to 2:1 balance, the criterion's
  neighbour arms (the agreement arm's eight neighbours, the class-coherence arm), contours, class edges and smoothing.
  - Past a bounded edge there is no neighbour: the domain ends.
  - An affine neighbour is the real neighbour.
  - A footprint on the root's edge must never be treated as having fewer neighbours than one in the interior. Today
    it would be, and the agreement arm would read the edge as structure.
- **(e) Affine extension is ordinary.** Quads past the root on an affine chart are quads of the same plane. They are
  scheduled, refined, floored and cached like any other.
- **(f) Not-a-system is never integrated.**
  - A quad wholly past a bounded edge is not requested.
  - A quad straddling a validity boundary integrates its valid footprints only. The rest are labelled `decode_failed`
    and drawn blank.
- **(g) The area statistics are unchanged** (R-408): each system is counted once, at its canonical address.

**EX8. Deep zoom works in the extension and in repeats.**

- **(a) In a repeat** the zoom is on the canonical quad. Its f64 centre, half-width, `x₀` and `J_D` are the canonical
  ones (deep_zoom §1 and §2), so precision is identical to zooming into the primary. Only the screen placement is
  mirrored or shifted, and a reflected axis flips the sign of `δ`.
- **(b) On an affine extension far from the root,** the GPU's f32 placement must not be the absolute UV.
  - deep_zoom §1's "misplaced globally by less than one quad width" holds only while `ulp_f32(c_u)` is under a quad
    width. A view panned to `u ~ 10³` breaks it about ten levels sooner.
  - The quad's f32 centre is therefore taken relative to an anchor near the view (its root cell, or the plane anchor
    re-set when the view is far from it). The IC is still `x₀ + J_D·δ`, with `x₀` in f64.
  - The standing prin-rs finding applies: *the deep-zoom floor is a property of where you zoom*. Quote the
    coordinate magnitude with any floor depth.
- **(c) The sliding depth bound and the screen floor** are view-relative and unchanged.
- **(d) Seams between images are rasterised with a shared-edge rule.** Two adjacent images meet with no background
  pixel between them at any zoom. The demo overdraws by half a pixel.

**EX9. Round trip.** The view state (centre, zoom, and on a periodic axis the unwrapped centre, Q1) is recorded as it
is. An export, the JSON and the steganographic tier 1 reproduce the same window, repeats included (round-trip rule,
RT1).

---

## 3. Tests (each with the arm that fires)

| # | Test | The arm that fires |
|---|---|---|
| T1 | With the controls shown, pan the shape sphere's view across θ = 2π and over a pole; the drag never stops and the view stays continuous | a scoping to F3, or a clamp to `[0,1]²`, stops the drag at the seam |
| T2 | Every image of a canonical quad is drawn from one computation: a counter of integrations per canonical address reads 1 with three images in view | computing a repeat by its own address reads 3 |
| T3 | No gaps: render the sphere at a zoom with 6+ images and the latent chart panned 10 units off the root, and count canvas-background pixels | a missing root cell or an unmapped repeat leaves background |
| T4 | Not-a-system is blank, never integrated: on the mass simplex past its edge, the integration count over the blank area is 0, and the Overlays toggle hatches exactly those pixels | integrating then discarding counts > 0 |
| T5 | A click on a blank (not-a-system) pixel draws the ✕ and starts no `computeIC` | a pick that falls through to the nearest valid pixel |
| T6 | Pick in a repeat: hover a pixel in the θ-shifted repeat and in the pole repeat. The IC is bitwise the canonical pixel's, and the readout says "repeat" | an unfolded decode gives a different IC |
| T7 | Neighbours across the seam: a footprint on the root's θ = 0 edge has 8 neighbours, the θ = 2π side's; its agreement arm matches an interior footprint's on a field that is smooth across the seam | treating the edge as missing neighbours gives a different value |
| T8 | 2:1 balance across a pole: an unbalanced pair straddling the pole is balanced | per-root balance misses it |
| T9 | Deep zoom in a repeat: zoom 40 levels into a pole repeat and into its canonical point; the ICs agree bitwise | precision lost to an uncanonicalised centre |
| T10 | Deep zoom on an affine extension at `u = 10³`: distinct ICs hold to the same depth as at the root ± the stated tolerance | an absolute f32 `c_u` collapses ~10 levels early |
| T11 | Seams: zero background pixels along every image boundary at 20 random zooms | a half-pixel rasterisation gap |
| T12 | R-469 on a bounded chart: drag the simplex's centre past its corner; the centre stops at the last valid value, and the view shows blank beyond | no clamp moves the centre into not-a-system |
| T13 | Statistics: "forbidden in view" and the W7 counts are unchanged by panning the sphere one period | counting repeats doubles them |

---

## 4. Queue, don't rule

- **Q1.** On a periodic axis, is the view centre stored unwrapped (continuous pan, unbounded number) or canonicalised
  after each gesture (bounded, with an invisible jump of one period)? *Recommendation:* unwrapped in the view state,
  canonical in keys and addresses.
- **Q2.** Are repeats of the field drawn at full strength? *Recommendation:* yes; they are the same systems. Repeats
  of a trace are fainter, as in the demo.
- **Q3.** An outline of the chart's primary square when the view goes past it. *Recommendation:* an Overlays › Chart
  toggle, off by default and on in the debug view. The demo has it on.
- **Q4.** A zoom-out limit on periodic charts (how many periods may show). *Recommendation:* none beyond the frame
  budget and baseline cover. The demo caps at 3× for its fixed maps.
- **Q5.** The level-0 pole repeat: draw it from its two level-1 halves, or raise the bootstrap to `L ≥ 1` on every
  chart with a pole-crossing axis? *Recommendation:* draw from the children; the bootstrap is already ≥ 2 in practice.
- **Q6.** Does a comment pinned in a repeat attach to the canonical point, so that it appears in every image?
  *Recommendation:* yes.

---

## 5. About the demo

Nothing in it is the app's code. What it shows that the app must do:

- **Pan and zoom past the square with the controls shown:** drag, wheel, pinch, and the − / + / Home buttons.
- **Repeats:** the sphere tiled by its group. B is `(x + πi, (−1)^i y + 2πj)`; C, with the fold, is
  `(x + πi, (−1)^(i+j) y + πj)`.
- **A pick in a repeat** resolves to the canonical system.
- **The mass simplex:** blank past its edge, the overlay toggle, the centre clamp, and the ✕ on a not-a-system click.
- **Traces with all their icons** in every repeat.

Where it differs from the app:

- **Fixed-resolution maps.** Its backgrounds are precomputed with prin-rs at one resolution:
  - latent `z_α × z_β`: 614² over ±4.8;
  - sphere: 768 × 384 over θ, φ;
  - simplex: 320² over `[−0.3, 1.3]²`;
  - momenta: 400² over ±0.48.

  Zooming in shows their pixels, and the latent and momenta tabs stop panning where their maps end. The app refines
  and has no such limit; that limit is the demo's, not R-469's.
- **No quadtree.** EX7 and EX8 are not exercised here.
- **Its own maps.** The sources are in `reference/prin_rs_examples/`. The sphere and momenta maps integrate the
  demo's own decode, so the background and the hovered trace are the same system. On the overlap the sphere map
  matches the latent chart's map on 100% of pixels where the field is smooth; the 20% that differ are all in the
  fractal regions, sampled on different grids.
