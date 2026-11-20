# The state map: every pixel coloured by where its system is now

*21 Oct 2026. New. A colouring, not a computation: each sample is coloured by its own system's position, at the
playhead, on a chart. The node is **Chart position**. It works on every chart, the shape sphere included, and is coloured by a base plus
layers (SM11). Shape n̂ is a different node and stays exactly as it is. The prototype `demo/state_flow.html` is the reference; where this file and it disagree, the
prototype's code is the transcription source and the disagreement is queued. Quote §0 verbatim in the ruling.*

---

## 0. My words

> "we take this idea of where we're sort of coloring the initial condition based upon where it ends up in some sort of
> chart. So in the shape sphere thing, we are coloring over time where the pixel trajectory is on the shape sphere
> chart and evolving over time. Now, what if we could generalize that to any chart? So you would almost see, for
> example, on a chart of, say, momentum, you would see how the pixel's momentum evolves over time, if that makes sense.
> Or you would do it with, maybe, any other sort of, like, latent parameter in a way. Because you could take a slice
> even and do wacky stuff like just say this entire slice area right now is an image, and then you could like see how
> stuff almost like flows. … I think from memory, people do this with pendulum sort of fractal stuff, so we could
> probably do that here."

> "I just don't want it to completely break everything, so I want you to like actually consider everything carefully
> here. Because it also has interacts, you know, properly as well, of like the extension stuff"

> "We can't have this changing any of the sim structs btw ? Can it work with the current data the shaders have access
> to ?"

> "Can this be made a sort of generalisation of the shape sphere colour map system that allowed us to do a lot of
> possible ways to colour the shape sphere? So we could have the same sort of tiling patterns + layers + combinations
> of stuff? Maybe could have more stuff?"

> "Just stop where it last was if it collided etc? That's cleaner?"

> "I wouldn't store the exact last position, I don't want to change the structs or anything that side"

> "Also I think the artefact doesn't really have a generalisation of the shape sphere colour map stuff idk. Its
> confusing. Also need you to design art boards on how the gui for this will look in the stain graph editor. Plus how
> this will work in the stain graph editor."

> "Idk still feel like we need the shape n mode. This generalisation is still assuming like a spherical coordinate system
> / unwrap.. which is weird? We need to tile & colour the plane differently in these modes"

> "Agreed" — to: Shape n̂ kept as its own mode; a colour vocabulary per quotient (a linear position map and real tilings
> on the plane, θ round the hue circle on the cylinder, triangles in the masses); tilings laid in chart units by default.

> "Why in this are you confusing the rest of the sting graph? I just want it to be... You can choose to color the plane a
> certain way. Either with a gradient, either with a site blend, either with a palette. And then you can also
> additionally add, you know, extra lines, be it grid lines, checkerboard patterns, tiles, etc. Or you can just choose to
> be purely tiled, you know, there's like the layers to it, like we did the shapesphere thing. Again, you're sort of
> confusing a lot of the different bits of the system into this one thing here."

> "Yep" — to: on a flat chart, the colour slot is a base (Gradient · Site blend · Palette · Picture) plus optional layers
> on top (grid lines, checkerboard, tile edges, stripes, contours, a site overlay), the same composition as the shape
> sphere's; Shape n̂, the brightness slot, Replace-L and the post chain unchanged; the state map only supplies the point.

> "Shape sphere chart can use this mode also btw, just shape n node is different to this node, understood? What are
> you calling this node? We need a good name for it that is simple and clear"

> "Chart position" — chosen for the node, one node with a time setting (playhead, t = 0 or a fixed t) in place of the
> two working names "Where now" and "Where at t = 0".

---

## SM1. Definition

For a sample with live state `x(t)` at the playhead, and a **target chart** `T`:

```
colour(sample, t) = C_T( fold_T( P_T( E( x(t) ) ) ) )
```

- `E` is the encode (`principia_dd_encode.md`), the quotient map onto the decode's canonical section: scale and
  rotation are removed, mirror images identified.
- `P_T` is the target chart's projection, the one the trace uses (`principia_trajectory_viewing.md` §3; trace
  follow-up F1). It returns one of four results, listed in SM5.
- `fold_T` maps the point onto `T`'s **quotient**, its group of images (R-407 types): θ mod 2π, the shape sphere's
  `n̂`, the canonical mirror.
- `C_T` is a colour map, **one-to-one and continuous on that quotient** (SM6).

**The target is the view's own chart by default.** Then, at `t = 0`, every sample sits where it started, and the
picture is the colour map itself, i.e. the legend. As the playhead moves, it flows.

**Image mode** replaces `C_T` with a look-up in a picture laid on `T`'s square (SM8). Each pixel shows the colour of
the picture at the place its system has reached. This is the "slice as an image" in §0, and it is a pull-back: one
gather per pixel, with no scatter.

## SM2. It changes no sim struct and reads only what the fragment already has

| Needed | Where it already is |
|---|---|
| the live state `r, p` at the playhead | `SimState`, "the live state of a marching simulation at the playhead" (render contract; payload §1; generation root) |
| the masses | `ICDescriptor`, the payload lane's decoded-IC quantities |
| ended / failed | the `state` lane. Storage never holds NaN (R-79); a run stopped by its event holds its end state |
| the encode, including the chart inverses | WGSL generated from the one Rust source (R-116); `ROUNDTRIP` already runs it per pixel (colour composition §3, "Fragment-side recompute") |
| `fold_T` and `C_T` | pure maths in the stain node: no history, no buffer |

It is a **derived field, computed at read** (payload §5), like the existing current-shape `n`. It needs no new
`SimState` field, no new accumulator and no kernel change. It is live-compatible: only the playhead state is read.

Three things it adds, none of them a sim struct:

1. **A target other than the view's chart** needs that chart's inverse in the fragment pipeline as well (Q1).
2. **Image mode** needs one texture binding for the picture: a render-side bind group.
3. **The colour map owns lightness** (SM6), so under L-ownership (render contract) it is a full-colour occupant: the
   brightness slot is None.

## SM3. Source side: the extension comes for free, because the colour is a property of the system

The colour depends only on the sample's system, never on where its pixel sits. So:

- **Wrapped copies** (θ period, pole crossings, the hemisphere's fold; R-407, R-412) decode to the same system, and get
  the same colour by construction. No seam handling is needed or allowed. **But only if a copy reuses its canonical
  quad's samples (EX7's canonical addressing) and never re-decodes from its own coordinate.** `x + 2π` folded back
  is not `x` in the last bits: re-decoding each pixel from its own coordinate, as the prototype first did, gave ICs
  differing at the 12th decimal in 1 of 400 random copy pairs. **Whether that shows is a question of horizon, and at
  the prototype's it does not.** Measured on the shape sphere, released from rest, at `t = 20`: a relative IC
  perturbation of 1e-15, 1e-12 or 1e-9 moves the shape by more than 0.02 on **0 of 300** systems; 1e-6 on 2; 1e-3 on 61.
  The prototype's colour test of copies reads 0 differing pairs **with and without** the fix (0 of 1024 on the sphere,
  0 of 768 on θ × dilation), so at this horizon it cannot fire and proves nothing about the fix. The requirement rests
  on the IC identity, which can be tested bitwise, and on longer horizons, where it will show.
- **Never a gap while navigating.** The prototype keeps a coarse **base grid per chart** over its whole reachable
  domain. On a repeating chart that is the square, drawn through every image (R-407's group); on an affine chart it is
  the navigation limit. It is drawn under everything, then the last finished view grid, then the new one.
  - The first prototype restarted its integration on every frame of a drag and drew nothing until a grid finished, so
    the extension went black. That was a bug in the prototype, now fixed.
  - Measured after the fix, black fraction of the canvas while dragging into the extension: 0 at every sample, on the
    latent chart and on the sphere.
  - In the app this is the quadtree's own coarse-ancestor fill, not a new mechanism.
- **Affine extension** past `[0,1]²` holds new systems, coloured like any other.
- **Not a system** (forbidden; R-470) is the canvas background, never integrated.
- **The quadtree, refinement and deep zoom are unchanged.** It is a colouring: the scheduler's criterion does not read
  it (the firewall, scheduler contract Part 1). A coarse leaf shows its footprint's colour.
  - In chaotic regions the field is rough at every scale, so a leaf the criterion keeps coarse looks blocky. That is
    the coarse-ancestor fill doing its job, not a fault.
  - Whether the state map should feed the criterion is Q3.

## SM4. Destination side: why it must be a quotient, not a branch

The trace picks each sample's branch by continuity with the previous one: θ ± 2π, the pole crossings
(`tiltProject`, `thetaHProject`). A per-pixel field has no previous sample, so it **cannot pick branches**. It must be
defined on the target's quotient, and `C_T` must be invariant under the target's identifications:

| Target axis | Quotient | What `C_T` must do |
|---|---|---|
| periodic (θ) | the circle | carry θ round a closed hue circle |
| the shape sphere (θ, φ with pole crossing) | the sphere, via `n̂` | be a function of `n̂`, never of (θ, φ) |
| the hemisphere (R-412) | the sphere mod the mirror | be a function of the canonical (folded) `n̂` |
| affine (latent `z`, a physical quantity) | the line | compactify (SM6) |
| bounded (ν, a mass) | the interval | map linearly |
| a tilted axis (E, F) | not periodic and multi-valued in θ | take the branch nearest the slice (smallest out-of-plane `w`); this is a choice, Q2 |

## SM5. The four results of a projection

| Result | When | Drawn as |
|---|---|---|
| **on the chart** | the projection exists | `C_T` |
| **left the chart** | a coordinate entering a shown axis with non-zero weight has no value (F1) | **at the edge it left by** (default, SM5a); or no value, in its own style |
| **past a bounded edge** | the nearest point lies past a bounded axis's end (F1's curve-axis row) | at that end (default, SM5a), counted apart from left; or no value |
| **undetermined** | the run failed (R-79's failed values; `sim_failed`) | the presentation black `#141418` (R-467) |

### SM5a. Left the chart: drawn at the edge it left by

A system that leaves a chart is drawn **at the edge it went through**, not grey. On the latent chart it leaves because
α passes the decoder's limit, so `z_α` is `±∞`, and the side is known from the shape (`cos 2α = n₁`). The other
coordinate keeps its value now. The compaction sends `±∞` to the colour map's edge. The latent momenta (`|q|` past
`q_max`), the tilt's `z_q3`, and a curve axis's nearest point past its end clamp the same way. **This uses only the
current state**, so it needs no stored field (SM2).

**It is not the same as "where it last was", and the difference is measured.** The true last on-chart position needs
history, and storing it is **ruled out by §0** (no struct change). It was measured once, offline, to know what the
clamp gives up:

On the latent chart at `t = 8`, 400 random systems, 129 of them left, true exit found at Δt = 0.005, compared in the
compacted coordinates `s ∈ [−1, 1]`:
- **The axis it left by agrees.** |Δs| median 0.033, and it falls with Δt (0.267 at the prototype's Δt = 0.25), so in
  the limit the clamp *is* the exit point on that axis.
- **The other axis is where they differ.** |Δs| median 0.08, p90 0.54, max 1.25. The system keeps moving after it
  leaves: into the collision, or back round a close approach. The clamp shows that coordinate now; the last position
  shows it at the exit.

So the clamp is exact in direction and current in the other coordinate. Half the left pixels differ little from the
last position; the tail differs a lot. Grey (no value) remains an option.

**Ended runs hold their end state (absorbing).** A run stopped by its event sits at its last state. Option: *after
its end*, paint the outcome palette instead.

Measured in the prototype at `t = 8` (64²–96² grids, the demo's integrator):
- On the shape sphere, the collided systems sit at the three binary-collision points and paint **three flat bands**.
  This is correct, and it shows the collision basins at a glance.
- On the latent chart, 3186 of 9216 have **left** (their α passed the decoder's limits near a collision).
- On the ν × dilation chart, 4854 of 6432 are **past a bounded edge**: most states are far from the right-triangle
  curve and project to its ends.

## SM6. Colour maps: one-to-one on the quotient, so they own lightness

**No continuous one-to-one map from the sphere into a plane exists** (invariance of domain). So any fixed-lightness
colouring of `n̂` — including the vMF six-pole blend (`principia_dd_colouring.md` §3.2) — gives some distinct shapes
the same colour. That is acceptable for a hue carrier, and **not for this field**: two systems at different places
must not look the same. So the sphere map uses all three OKLab axes, linear in `n̂`. Measured (colours binned at 2e-3 in OKLab over a 300² grid of the sphere): the vMF blend at fixed L puts **antipodal shapes** (chord 2.000) in one bin at κ = 1, 3 and 6 — the pair `±(1,1,1)/√3`, both neutral grey — while the linear map's worst same-bin pair is 0.025 apart, the bin's own width. The state map is therefore a
**full-colour occupant**, and the vMF blend stays the shape colouring it already is (Q4).

Per target, as prototyped:

| Target quotient | Map (OKLab) |
|---|---|
| sphere `n̂` | `L = 0.64 + 0.20 n₃`, `a = 0.095 n₁`, `b = 0.095 n₂`: linear, so one-to-one |
| circle × line (θ × dilation) | `a, b = 0.09 (cos θ, sin θ)`, `L = 0.66 + 0.12 tanh(h/κ)`: a cylinder |
| simplex (masses) | `(a, b) = 0.15 Σ mᵢ ûᵢ` for three hue directions 120° apart, `L = 0.70`: an affine image of the triangle |
| line × line, interval × line | **linear**: `(a, b) = 0.095 (sₓ, s_y)`, `L = 0.75`. Revised (SM11): the first version put `x` along a 270° arc of hue, a polar unwrap of a flat chart, to avoid a grey centre. §0 rules out the unwrap; the grey centre is the chart origin and is accepted |
| circle × circle (none in the demo) | `x` round the hue circle, `y` round a circle in (L, C): a torus |

Rules:
- **Compactification:** `s = tanh((z − c)/κ)` with `κ` the chart's home half-range (latent 3, momenta 0.3, the tilt's
  `s` π, dilation 0.6), so the home view spans about `s ∈ [−0.76, 0.76]`.
- **Chroma is fixed per map, never clipped per pixel** (clipping breaks linearity and injectivity), at the largest
  value that stays in sRGB gamut over the map's whole range. That was checked on a 200² grid for every map above: 0
  out-of-gamut samples.

## SM7. Colour frame and deep zoom

- **Chart (default)**: absolute. The same colour means the same place in every view, so views are comparable.
- **View (option, the view's own chart only)**: `s = tanh((x − view centre)/view half-width)`.
  - It is needed at depth: in the absolute frame, the `t = 0` picture at a deep zoom is one flat colour, because the
    whole window is inside one colour step.
  - At `t > 0` most systems leave the window and saturate the relative map. That is expected and stated.
- **Precision**: the state is f32. The `t = 0` identity holds to **4e-7 (latent), 1.5e-6 (sphere), 5e-6 (simplex),
  2.7e-8 (momenta), 7e-8 (θ × h), 1.9e-6 (tilted E, F)** in chart units. On ν × h it holds to **4.9e-4**, which is the
  demo's 1/1000 curve table, not the method.

## SM8. Image mode

- The picture is laid on `T`'s home square (equirect on the sphere; θ across on the cylinder).
- Past the square on an affine axis it is **mirror-repeated and dimmed**, so it stays continuous and says it is outside.
- The picture can be built in (checkerboard, grid, rings) or the user's own, uploaded.
- Played over time, a picture laid on the chart is stretched along the flow: smooth on regular islands, shredded where
  the dynamics are chaotic. That is the FTLE's meaning, seen directly; it is not a measurement of it.

## SM9. Legend and the hovered pixel

The legend is the colour map (or the picture) over `T`'s square, with the hovered pixel's system marked where it is
now. On the view's own chart, the marker sits where the hovered pixel's current colour sits in the legend. The trace
of the same pixel would run to the same place: the trace is this field for one pixel. The prototype draws no trace,
only a ring on the hovered pixel.

The marker is computed with the grid's own integrator settings, so it agrees with its pixel exactly.

## SM10. Time, ensemble and fade

- **Time**: it is the field at the playhead. Scrubbing back is re-integration, as for every live field (temporal
  architecture note). The prototype instead keeps states every Δt = 0.25 to the horizon, so it can scrub instantly;
  that is a demo device, not a design.
- **Ensemble**: the nominal copy (copy 0). An OKLab mean over the `E+1` copies would turn undetermined regions grey,
  which is informative but not one-to-one (Q5).
- **Fade off the slice**: opt-in, off by default, view's own chart only (R-509's distance, as the trace).
  A fade spends lightness, so it is stated as breaking injectivity.

## SM11. Colouring Chart position: a base, then layers

*Revised three times on 21 Oct 2026 (§0's last five quotes). The first version put one sphere kernel on every chart;
the second invented separate node kinds and mixed the source and brightness nodes into them; the third tied the sphere
to Shape n̂. This one: Chart position is one node, on any chart including the shape sphere, and its colour slot is
colour composition §1 as it stands — a base and layers — with the chart's point where Shape n̂ has n̂.*

**Two different nodes.** *Shape n̂* is the existing node: the shape of the system, coloured by its own nodes (Site blend,
Bands, its position map), exactly as now. *Chart position ⟨chart⟩* is the new one: where the system is on a chosen
chart. On the shape sphere chart the two read the same point, and are still two nodes: Shape n̂ keeps its colouring,
Chart position takes a base and layers. The backbone is unchanged: one colour slot, one brightness slot, Replace-L or
Multiply, the post chain.

**Chart position ⟨chart⟩** — chart (default: the view's own) · at (playhead · t = 0 · a fixed t) · when it leaves the
chart (clamp to the edge · none) · after its run ends (hold · outcome colour). Stores nothing.

**Its colour slot is a base plus layers** — the same shape as the shape sphere's colouring (a Family A or B occupant,
then `bandmask` / `site_overlay` combinators, colour composition §1.1–§1.3):

*Base, one of:*

| Base | Colour composition | Shape sphere | On the plane | Cylinder (θ × s) | Simplex (masses) |
|---|---|---|---|---|---|
| **Gradient** | Family B: a field over the point through a ramp | **3D**: `L = 0.64 + 0.20 n₃`, `(a, b) = 0.095 (n₁, n₂)`, one-to-one · round the longitude (hue circle) · pole to pole | **2D**: `(a, b) = 0.095 (sₓ, s_y)` at `L = 0.75`, one-to-one, L free · along x · along y · out from the centre | 2D: θ round the hue circle, `L = 0.66 + 0.12 tanh(s/0.6)` · round θ (hue circle) · along s | 2D: three primaries mixed by the masses at `L = 0.70` · along m₂ · along m₃ |
| **Site blend** | Family A: nearest / top-k / vMF, blend in OKLab or RGB | the sphere's site sets (`axes6`, `corner8`, `ico12`, `fib(N)`, `ring(N)`), the same code path as Shape n̂'s Site blend (agreement exactly 0) | sites on a square or hex lattice, plain flat distance `−|x − p|²` | the same, a whole number of sites round θ | the same lattice laid on the masses |
| **Palette** | categorical: each tile one palette colour — purely tiled | latitude × longitude cells (a multiple of 6 round θ) · Fibonacci cells · meridian wedges · latitude bands | square, hexagons, triangles, brick, stripes x / y | the same, closing round θ | triangles in the masses; stripes in m₂ / m₃ |
| **Picture** | the image pulled back through the flow (SM8) | equirectangular | over the square | wrapped in θ | over the triangle |

1D ramps run one fixed three-stop OKLab ramp (indigo → teal → yellow); round θ uses the hue circle, the one place hue is
native. Site colours and the palette are swatch sets, as on the sphere: **by index** — `(i mod 3, j mod 3)` picks one of
nine OKLab swatches at `L = 0.72`, `(a, b) = 0.08 (i mod 3 − 1, j mod 3 − 1)`, so every neighbour differs — or
**golden-angle** hues by tile.

*Layers on top, in order, each optional* — every one a `bandmask` (a line or cell colour where its pattern is in band,
mixed in at its opacity), or for site dots a `site_overlay`:

| Layer | In band where |
|---|---|
| grid lines | within 0.045 of a whole x or y |
| checkerboard | ⌊x⌋ + ⌊y⌋ is odd |
| hexagon / triangle / brick edges | a point 0.045 away lies in a different tile |
| stripes along x / y | ⌊x⌋ or ⌊y⌋ is odd |
| rings round the centre | within 0.045 of a whole distance from the chart's centre |
| site dots | within 0.13 of a lattice site |

On the shape sphere the layers are the sphere's own: latitude / longitude lines, checkerboard (lat × lon), meridian
stripes, latitude bands, Fibonacci cell edges (a point 0.045 of the cell spacing away, along the surface, is in another
cell), site dots on the Fibonacci sites. Flat tile edges and rings are not offered there.

Each layer has its own spacing (`n` across, 4–24), colour (dark, light, black, white) and opacity (25–100%).

**Spacing on a flat chart is in chart units by default**: `n` across the chart's square, the same size continuing past it, and as many
down as keeps cells square on screen. *The squashed square* lays `n × n` over the compactified `[−1, 1]²`, the whole
unbounded chart, cells shrinking toward infinity. A system that has left the chart takes the cell at the navigation
edge it left by (SM5a). On the cylinder every count round θ is a multiple of 6, so every pattern closes across the seam.

**Lightness.** The plane's 2D gradient, the sphere's longitude gradient, Site blend and the index palette hold L fixed, so a brightness node can take L
without losing position. The 1D ramps, the sphere's 3D gradient and the cylinder's 2D gradient spend L on position, and a brightness node after
them raises the Problems warning, as it does after Shape n̂'s position map and Bands. Layers are mostly lightness, and
under Replace-L their lines fade; that is a second, separate warning. The plane's 2D gradient is grey at the chart
origin, which on the latent charts is the Lagrange landmark (Q9).

**Brightness, unchanged in meaning** and outside the colour slot: displacement `|e(t) − e(0)|`, a straight-line distance
on the quotient (`e` = n̂, `(cos θ, sin θ, s)`, `(sₓ, s_y)` or the masses: a distance only, never a colour); *left its
starting tile*, which under Chart position compares the Palette's tile now and at `t = 0`, and under Shape n̂ the
nearest site.

**Only the 2D and 3D gradients must be one-to-one (SM6).** Site blends, palettes and layers are classifications, many-to-one by
design.

**Fields that need history stay out of scope without a struct change** (Q7).

## SM12. Tests (each with the arm that fires)

| Test | Passes when | Fires on |
|---|---|---|
| Identity at `t = 0` | on every chart, target = view, every sample's projected point equals its own chart point to f32 round-trip (the prototype's figures, SM7) | a wrong projection, a missed fold, a mirror slip |
| Copies agree | every copy's sample is the canonical quad's sample: its IC is **bitwise** equal (the control, re-decoding from the copy's own coordinate, differs on 1 of 400 pairs). A colour comparison at `t = 20` is not the test: it reads 0 differing pairs with and without the fix | a copy re-decoded from its own coordinate; a colour computed from screen position |
| One-to-one | colours binned at 2e-3 over a 300² grid of each target's quotient: no bin holds points more than a bin's width apart. Controls that must **fail**: the vMF blend (antipodes, 2.000) and a closed hue circle on a line (1.993) | a fixed-lightness sphere map; a hue circle on a non-periodic axis |
| Invariance | `C_T` equal on identified points (θ and θ + 2π; `n̂` across a pole crossing) | a map written on (θ, φ) |
| Gamut | 0 out-of-gamut samples per map | per-pixel clipping |
| Edge clamp | for left systems, the clamp agrees with the exit point found at fine Δt on the axis it left by, and the gap falls with Δt (0.267 → 0.033 from Δt 0.25 to 0.005) | a clamp to the wrong side, or a finite clamp on an infinite coordinate |
| Never a gap | the canvas has no uncovered pixel during a drag or zoom into the extension (black fraction 0 at every sample) | the old grid dropped before the new one lands |
| Conserved axes | on the mass simplex the field is identical at every `t` | a mass leaking into the march |
| Four results | a latent-momentum target has left-the-chart pixels where `|q| > q_max`; ν targets have past-the-edge pixels; neither is drawn as a colour | left drawn as a clamped colour |
| No struct change | the build changes no `SimState`, payload or descriptor layout (`git diff` on the layout definition is empty) | a stored field |
| Firewall | the scheduler's decisions are identical with the state map on and off | the field read by the criterion |
| Shape n̂ unchanged | the Shape n̂ node, every one of its colour nodes (Position, Site blend over axes6 / fib / ico12 / corner8 with each kernel, Bands) gives bitwise the values it gave before the flat vocabularies were added: 7200 values over 400 random shapes × 6 settings, **0** differ (old build against new). The brightness slot is not in this test: Replace-L now scales chroma (below), which changes a sphere stain *with* a brightness node | a flat node's code reaching the sphere path |
| Same site blend on the sphere | Chart position ⟨shape sphere⟩ with Site blend (axes6 · Full · vMF κ 3 · OKLab) against the Shape n̂ node's Site blend at the same settings: worst difference **exactly 0** over 300 random shapes. Two nodes, one code path | a second implementation of the sphere's site blend |
| Neighbours differ | Palette by index (and Site blend's index swatches): across every tile boundary, sampled 2.5–8.6 thousand pairs per tiling, **0** pairs share a colour for square, hexagons, triangles, brick and both stripes | an index not stepping by ±1 to a neighbour; a hexagon or brick coloured by a 2-colouring |
| Layers in order | grid over checkerboard differs from checkerboard over grid wherever both are in band; a layer at 0% opacity changes nothing | layers applied as an unordered set |
| Whole periods | on the cylinder, the tile count round θ is a multiple of 6, so every base and layer closes: θ and θ + 2π give the same tile colour — **0 of 7200** differ, over 6 patterns × 6 counts | a count that is not a multiple of 3 (index colours) or 2 (checker), which draws a seam |
| Plane map one-to-one | binned at 2e-3 over a 300² grid of `[−1, 1]²`: worst same-bin pair **0.028** apart in s, inside the bin's own diagonal (0.030) | an angle or unwrap on a flat chart |
| Replace-L in gamut | chroma scaled by `min(1, L/0.75, (1 − L)/0.35)`: **0 of 99,710** out-of-gamut samples over every flat colour node at every brightness level used (0.18, 0.30, 0.50, 0.78, 0.84). Without the scaling, the 0.30 level clipped to near-black in the prototype, the undetermined colour | per-pixel clipping |

## SM13. Queue, don't rule

- **Q1. A target other than the view's chart.** Recommendation (a): allowed, with the target's inverse generated into
  the fragment pipeline beside the view's. (b) The view's own chart only.
- **Q2. Tilted targets.** Recommendation (a): the branch nearest the slice, as prototyped. (b) Not offered as a
  target.
- **Q3. The state map and the criterion.** Recommendation (a): never a criterion input (the firewall). (b) A spread
  over it as a candidate signal, measured first.
- **Q4. The vMF shape colouring.** It is not one-to-one, by SM6's theorem. Recommendation (a): keep it as the hue
  carrier it is, and state that in colour composition §4.1. (b) Offer SM6's sphere map as an alternative shape
  colouring.
- **Q5. Ensemble colour.** Recommendation (a): the nominal copy. (b) The OKLab mean over copies, labelled as not
  one-to-one.
- **Q6. After its end.** Recommendation (a): hold the end state. (b) The outcome palette.
- **Q7. History fields** (winding on any chart, time in a tile, first passage). Recommendation (a): out of scope; each
  needs its own accumulator, which is a struct change. (b) Specify the accumulators, measured for cost first.
- **Settled by §0, not queued: the exact last position is not stored.** The edge clamp (SM5a) is the behaviour; no
  struct changes for it.
- **Q8. Tiling defaults** (SM11): pattern, tiles across (6) and the index palette chroma (0.08) are calibration
  values (R-71). Recommendation (a): adopt the prototype's as the starting values. Chart units as the default is
  settled by §0, not queued.
- **Q9. The plane's grey origin.** The linear map is achromatic at the chart origin. Recommendation (a): accept it — on
  the latent charts the origin is the Lagrange landmark, a named place, and it is far in L from the undetermined and
  no-value colours. (b) Offset the map so the achromatic point falls outside the home square.
- **Settled by §0, not queued: Shape n̂ stays its own mode.** Its nodes are unchanged and nothing is migrated.

## SM14. What this changes

| Doc | Change |
|---|---|
| `principia_dd_simstate_payload.md` §5 | the derived table gains "chart position at the playhead", from `r, p`, the masses and the target chart; stores nothing |
| `principia_colour_composition.md` §3, §4.1 | the state map as a full-colour occupant (owns L; brightness None); image mode's texture input |
| `principia_render_gui_spec.md` | SM15: the `point⟨q⟩` field subtype, the two Where sources, the four colour nodes and two brightness nodes, the Problems entries, the presets, and the 2D legend |
| `principia_dd_colouring.md` §3.2 | Q4's sentence |
| `principia_colour_composition.md` §1–§2 | Shape n̂'s nodes unchanged. Added (SM11): a flat point as an input beside n̂; its Gradient fields; lattice site sets (square, hex) with flat distance; the Palette (categorical by tile); band patterns for layers (grid, checker, tile edges, stripes, rings, site dots) |
| none of the contracts' layouts | (SM2) |

## SM15. In the stain graph editor

The state map adds one source and a few occupants to the existing stain graph (render GUI spec §0–§9). It does not add a
mode, and it does not touch the backbone. Art boards: `boards/` (1 editor, 2 base and layers, 3 inspectors, 4 ports,
5 presets, 6 Explore).

```
Chart position ⟨chart⟩ at the playhead ─→ colour slot:  base ─→ layer 1 ─→ layer 2 ─→ … ─┐
Chart position at the playhead, at t = 0 ─→ brightness slot (unchanged) ─────────────────┴→ Replace-L / Multiply → post* → OUT
Shape n̂ ─→ its own colouring (unchanged), an alternative source in the same colour slot
```

**One new field subtype.** `field` gains `point⟨q⟩`, q ∈ sphere · plane · cylinder · simplex: a position on the
quotient plus the SM5 status (ok, clamped, past edge, undetermined). Port types and wire colours are unchanged. Shape
n̂ is a `point⟨sphere⟩`, and its nodes accept exactly that, as now.

**The node.** *Chart position ⟨chart⟩* — chart (default: the view's own), at (playhead · t = 0 · a fixed t), when it
leaves the chart (clamp · none), after its run ends (hold · outcome); stores nothing. At t = 0 it is the pixel's own IC
through the projection, with no march. It is one node; a brightness field that compares two times takes two copies of
it. *Shape n̂* is a separate node and is unchanged; on the shape sphere chart both are offered.

**Colour slot.** From Shape n̂: its own nodes, unchanged. From Chart position, on any chart: a **base** (Gradient · Site blend ·
Palette · Picture) and an ordered list of **layers** (grid lines, checkerboard, hexagon / triangle / brick edges,
stripes, rings, site dots; on the sphere its own: lat / lon lines, checkerboard, stripes, Fibonacci cell edges, site
dots), each with spacing, colour and opacity (SM11). Layers are the existing `bandmask` and
`site_overlay` combinators with flat band patterns; the inspector shows them as a stack under the base, reorderable,
each removable.

**Brightness slot and combine.** Unchanged: Displacement and Left its starting tile are brightness occupants like any
other, and Replace-L scales chroma by `min(1, L/0.75, (1 − L)/0.35)` along with L, so every level stays in gamut (0 of
99,710 samples out). Before that, the 0.30 level clipped to near-black, the undetermined colour.

**Inspector.** The library and the drag-off-a-pin menu offer only what the wired source takes. Swapping the source
between Shape n̂ and Chart position puts each base's counterpart in its place (Position ↔ Gradient, Site blend ↔ Site
blend, Bands ↔ Palette); changing Chart position's chart re-offers the bases and layers that exist on the new one. Every
setting stays on the graph, so switching back restores it exactly.

**Problems.** Warning where a brightness node replaces L that the base spends on position (Shape n̂'s position map and
Bands; a 1D gradient; the sphere's 3D and the cylinder's 2D gradient). Warning where a brightness node replaces L under lightness layers.
Info per compile: left the chart (clamped), ended (held), past a bounded edge, undetermined, as counts; no SimState,
payload or descriptor change.

**Presets** (prototype `smPreset`): Classic shape colour (the Shape n̂ node → Site blend, as today); and on Chart
position, on any chart: Where (Gradient 2D, 3D on the sphere); Graph paper (Gradient + checkerboard + grid lines); Basins
(Palette: hexagons, Fibonacci cells on the sphere); Displacement (Gradient + Displacement); Flow (Picture); Mixing
(Palette: square, lat × lon on the sphere, + Left its starting tile). A preset inserts an ordinary editable graph.

**Explore.** The state map is a stain picked like any other. Compass, play bar and chart controls are unchanged. The
legend is the colour slot drawn over the target chart (base and layers), with a ring at the hovered system, drawn on the
edge when it has left.

**Backwards compatibility.** Nothing to migrate: every saved stain loads and renders as before.

**WGSL sketch.** The source compiles to calls that already exist: `encode(r, p)` (ROUNDTRIP), the target chart's
inverse, `fold_q`, the status. The base and each layer are functions of `Point` only.

```wgsl
struct Point { u: f32, v: f32, s: vec2<f32>, q: u32, status: u32 }   // chart coords, squashed coords, quotient, SM5 status
fn where_now(px: Px) -> Point { let z = encode(px.r, px.p); return fold(target_inverse(z)); }
fn gradient_2d_plane(pt: Point) -> vec3<f32> { return vec3(0.75, 0.095 * pt.s.x, 0.095 * pt.s.y); }   // OKLab
fn layer(base: vec3<f32>, pt: Point, kind: u32, n: f32, col: vec3<f32>, alpha: f32) -> vec3<f32> {
  return select(base, mix(base, col, alpha), in_band(kind, cell_xy(pt, n))); }                       // bandmask
```

**Tests.** (1) Every saved stain renders bitwise the same with the state map in the build. (2) That arm fires: removing
Shape n̂'s registration changes the render. (3) A mixed quotient on Displacement is an error, not a silent cast.
(4) Switching the source chart and back restores the base's settings exactly. (5) Layers compose in order: grid over
checkerboard differs from checkerboard over grid wherever both are in band.
