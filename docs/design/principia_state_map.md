# The state map: every pixel coloured by where its system is now

*21 Oct 2026. New. A colouring, not a computation: each sample is coloured by its own system's position, at the
playhead, on a chart. The node is **Chart position**. It works on every chart, the shape sphere included, and is coloured by a base plus
layers (SM11). Shape n̂ is a different node and stays exactly as it is. The prototype `demo/state_flow.html` is the reference; where this file and it disagree, the
prototype's code is the transcription source and the disagreement is queued. Quote §0 verbatim in the ruling.*

*Ported by R-542 (21 Oct 2026) from the bundle as delivered, `docs/incoming/2026-10-21_state_map/principia_state_map.md`,
its headings unchanged. The prototype is `docs/gui/reference/state_map/state_flow.html` and the art boards are
`docs/gui/reference/state_map/boards/` (reference mocks, not ported as code); the bundle's `demo/state_flow.html` and
`boards/` read as those, and its measurement scripts `tests/*.js` stay in `docs/incoming/2026-10-21_state_map/`,
reference only. §0 is the human's words; SM1 to SM15 are the applied text (R-369). Paragraphs headed *Transcribed from
the prototype (R-542)* are the port's: what the prototype's code does where this file is silent. Where the file and the
code disagree the code is carried, the file's words kept, and the disagreement queued (RQ-499). SM13's questions are
RQ-480 to RQ-488; the conflicts the port found with the corpus and the open branches are RQ-489 and RQ-496 to RQ-499.
The prototype was read as source; it was not run in a browser. The trace's "F1" this file cites is R-538's TF1
(`principia_lines.md` §8).*

*Amended by R-543 (21 Oct 2026), the tiling follow-up, `docs/incoming/2026-10-21_state_map_tiling/` (its
`INSTRUCTION.md` the human's ruling): colour maps that tile the infinite plane. §0 gains the follow-up's words; SM6 and
SM11 gain the **Torus** gradient, SM8 and SM11 the Picture's **wrap mode** and **Dim past the square**, SM8 the
**Checkerboard, tileable** picture, SM11's lightness paragraph and SM15's Problems the Torus among the bases that spend
L, SM12 six tests and SM15 the Flow preset's new settings; each addition is marked R-543. No line of the file is changed
or removed: where R-543 replaces a rule the line stays and an *Amended by R-543* line follows it, so the bundle's file
stays a line-for-line subsequence of this one (REQ-COL-112). The reference prototype
`docs/gui/reference/state_map/state_flow.html` is version 9 of the State Flow page from R-543 on, the follow-up's
`demo/state_flow.html`; its diff against version 8 is the tiling only.
Where R-543's text and that code disagree the code is carried and the disagreement queued, except where a queued
entry applies R-543's text meanwhile: RQ-506 (a) carries the ruling's words where the code is silent (the legend's
period, the readout's wrap mode) and the code's period counts. The tile that separates systems a Torus period apart is
RQ-507.*

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

*The follow-up of 21 Oct 2026 (R-543): the human's words, `docs/incoming/2026-10-21_state_map_tiling/INSTRUCTION.md`
§0 verbatim, "here" in its last line being that bundle:*

> > "i geuss what i'm reaching for are colour maps that tile cleanly over the whole infinire plane because the tiling is
> > weird right now ?"
>
> To the proposal of a Torus gradient base and a Repeat wrap mode for the Picture base:
>
> > "yeah"
>
> And on the prototype built from it (`demo/state_flow.html` here, version 9 of the State Flow page):
>
> > "ok add this as follow up instructiosn bc i already sent of f that bundle"

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

*Transcribed from the prototype (R-542; `smWhere`, `smTarget`).* The prototype's targets are its charts: the latent
`z_α × z_β` chart, the shape sphere `(θ, φ)` (both hemispheres; the one-hemisphere view targets the full sphere), the
mass simplex, the latent momenta `z_q1 × z_q3`, `θ × dilation rate`, the right triangles `ν × dilation`, and the sphere
tilted 30° into `z_q3` or into `p_λ,y`. The target "the view's own" resolves to the view's chart, the one-hemisphere
view to the full sphere. Per target, `P_T` reads, from the encode `e` of the state:

| Target | On the chart | Otherwise |
|---|---|---|
| latent `z_α × z_β` | `(z_α, z_β)` when both are finite | left: `z_α = −∞` if `α < π/4` else `+∞`, with `α = ½ acos(n₁)` from the shape; `z_β` its own sign's ±∞ if not finite |
| shape sphere | `(θ, φ)`, with `n̂ = (sin φ cos θ, sin φ sin θ, cos φ)` | — (never left) |
| mass simplex | `(u, v)` from the masses (`u = m₂`, `v = m₃/(1 − m₂)`, on the raw masses) | — (never left) |
| latent momenta | `(z_q1, z_q3)` when both are finite | left: each non-finite one `sign(q_i)·∞` |
| tilted into `z_q3` or `p_λ,y` | `(s, φ)`, `s = a cos 30° + (d/κ) sin 30°` with `a = θ + 2πk − π` on the branch `k ∈ [−4, 4]` with the smallest out-of-plane `|w| = |−a sin 30° + (d/κ) cos 30°|`, `κ = 0.6/π`, `d` the target coordinate (`z_q3`, or `p_λ,y`) in the unmirrored frame | left, when `z_q3` has no value (`|q₃| ≥ q_max`): `s = sign(q₃)·∞`, `φ` kept |
| `θ × dilation` | `(θ, h)`, `h` the scale-free homologous rate `(Σ m r·v)/I · I^¾` | — (never left) |
| `ν × dilation` | `(ν, h)`, `ν` the nearest point of the right-triangle curve (mirror folded) | past a bounded edge when `ν ∉ [1/32, 31/32]`: clamped to that end, `h` kept |

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

*Port's note (R-542).* SM2 is the human's hard constraint: no change to `SimState`, the payload, the `ICDescriptor` or
any layout; a need for a stored field is stopped and queued, never added. Two places the port found where the field
reads what main does not yet give a colour occupant are queued, not resolved: a colour occupant may not reference the
chart (`principia_dd_colouring.md` §4, seam 6; render contract Part 4), where Chart position reads the target chart and
its inverse (the view chart's inverse is already in the encode `ROUNDTRIP` runs, dd_encode §2; only a target other
than the view's chart needs a second one); and SM11's "a fixed t" needs a state other than the playhead's, which `SimState` does not hold (both RQ-489). Item 3's "the brightness slot is None" meets L-ownership
(render contract Part 4: "resolved by combiner choice") and SM11 and SM15, which let a brightness node follow an L-fixed
base (RQ-496). The prototype offers no fixed t; meanwhile Chart position reads the playhead or `t = 0` only.

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

*Transcribed from the prototype (R-542; `smMakeLevel`, `smBase`, `smStart`, `smDraw`).* The prototype's canonical
addressing folds a copy's point onto the chart's square (the shape sphere by its θ period and pole crossings, `θ` mod
2π on `θ × dilation`) and snaps it to a lattice of `2⁻³⁰` before decoding, so that every copy integrates the same IC
bit for bit; in the app that is EX7's canonical quad (R-537), not a lattice. Its base grid is 40 cells on the short
side over the base rectangle; a view grid restarts only after the view has been still for 180 ms, dropping stale work,
and is computed at 24, 48 and then the chosen resolution (64, 96 or 128), coarse levels drawn underneath. A pixel the
chart does not decode, or decodes to a non-finite IC, is not a system: blank (alpha 0), never integrated. *Port's note
(R-542, applied per R-369):* a `decode_failed` sample the chart does not classify as not a system draws blank too, as
R-511 draws it in a presentation view; Chart position tests the `state` lane before it projects, and SM5's undetermined
is `sim_failed` only. On main a copy's canonical address is EX7's for repeats outside the primary range; two mirror
pixels inside the square (the spherical map's hemispheres, R-141; the exponential map's straddling cap, R-503) are each
decoded from their own coordinate and agree to rounding, not bitwise; and on an affine latent axis there is no
navigation limit (R-469; R-537 EX3), so the app's base fill is EX4's coarse ancestor and EX7(a)'s root tiling, the
prototype's base grid a demo device (RQ-498).

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

*Port's note (R-542).* The prototype's code reads the shape sphere target from the signed `n̂`, computed before the
encode canonicalises, over both hemispheres, as this table's shape-sphere row has it; SM1's "mirror images identified"
folds it. The code is applied meanwhile (RQ-498). The code's one-hemisphere view targets the full sphere, where this
table's hemisphere row folds (RQ-504; the code meanwhile). On main R-141's decode gauge puts a lower-hemisphere pixel's
system on the upper hemisphere, so on a both-hemispheres view SM7's identity holds modulo the mirror (RQ-498).

## SM5. The four results of a projection

| Result | When | Drawn as |
|---|---|---|
| **on the chart** | the projection exists | `C_T` |
| **left the chart** | a coordinate entering a shown axis with non-zero weight has no value (F1) | **at the edge it left by** (default, SM5a); or no value, in its own style |
| **past a bounded edge** | the nearest point lies past a bounded axis's end (F1's curve-axis row) | at that end (default, SM5a), counted apart from left; or no value |
| **undetermined** | the run failed (R-79's failed values; `sim_failed`) | the presentation black `#141418` (R-467) |

*Transcribed from the prototype (R-542; `smPaint`).* Under "no value" a left pixel draws `rgb(62, 58, 66)` and a
past-edge pixel `rgb(44, 50, 58)`, each its own grey; an undetermined pixel draws `rgb(20, 20, 24)`, `#141418`, and is
counted apart. The status line counts left, past a bounded edge, ended and undetermined, as SM15's Problems pane does.

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

*Transcribed from the prototype (R-542; `smImageUV`, `smTileXY`, `smLastOn`).* Where a clamped coordinate is `±∞`, a
picture or a tiling takes the edge of the target's navigation rectangle on that side (its base range, or its
navigation limit, or its square), inset by `10⁻⁹` of the range for a tiling; the compaction maps `±∞` to `±1`. "After
its end: outcome colour" paints the outcome palette at 0.78 of its value (the escaping body's colour, the pair's
colour, `#F29620` for a triple collision). The prototype keeps a `last` mode in its code, reached from no control, that
scans back through its stored frames: it is the offline measurement above, not a behaviour; the exact last position is
not stored (§0, R-542).

*Port's note (R-542).* The trace draws a left sample and a past-the-end one as no value (R-538 TF1, `principia_lines.md`
§8: "not drawn there, the line broken"), where SM5a's default clamps; SM9's "the trace is this field for one pixel"
holds for on-chart samples only; "past a bounded edge" is R-470 N1's and R-472's term for a source pixel drawn blank;
and on the latent chart "the navigation edge it left by" (SM11) can only be the compacted edge `s = ±1`. Queued as
RQ-498; meanwhile the clamp is the default, as the file and the prototype have it. The latent-chart figures above are
the prototype's, whose latent chart has `AMIN = 0.05` where R-21 sets `α_min = 0`, so on main a system leaves the latent
chart almost only at a collision (RQ-503); main's encode clamps and never returns a non-finite `z`, so the app's "left"
is read from a shown coordinate's clamp (REQ-COL-133). Whether "none" is an ordinary setting or a debug option is
RQ-505.

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
| circle × circle, realised (R-543) | by the Torus gradient below, on the plane and the cylinder, which it makes periodic |
| line × line, interval × line: the **Torus** gradient (R-543) | periodic in chart units, `n` periods across the home square: `θ = 2πX`, `φ = 2πY` with `(X, Y)` the position in periods, `(a, b) = (R + r cos φ)(cos θ, sin θ)`, `L = L₀ + r_L sin φ`, `R = 0.074`, `r = 0.037`, `L₀ = 0.70`, `r_L = 0.12`: a torus, one-to-one within a period, repeating exactly across periods |
| circle × line (θ × dilation): the **Torus** gradient (R-543) | `X = (θ mod 2π)/2π`, θ once round, and `s` repeating `n` times over the square: the same torus |

Rules:
- **Compactification:** `s = tanh((z − c)/κ)` with `κ` the chart's home half-range (latent 3, momenta 0.3, the tilt's
  `s` π, dilation 0.6), so the home view spans about `s ∈ [−0.76, 0.76]`.
- **Chroma is fixed per map, never clipped per pixel** (clipping breaks linearity and injectivity), at the largest
  value that stays in sRGB gamut over the map's whole range. That was checked on a 200² grid for every map above: 0
  out-of-gamut samples.

*Added by R-543 (the Torus gradient).* The Torus gradient makes the plane periodic, so it tiles the infinite plane
with no seam and no special home square. Each period maps onto a torus inside OKLab, `X, Y` the position in periods in
**chart units** (never the squashed square), `n` periods across the home square; on the plane the `y` period count is
`round(n · H / W)`, as the Palette's tiling already does. On the cylinder `X` is θ itself once round (`wrap(θ)/2π`), and
only `s` repeats. A torus sits in 3-D colour without touching itself (`R > r`), so within one period the map is
one-to-one and continuous; across periods it repeats exactly. Its shape is fixed, never clipped per pixel (the rule
above): the constants are 0 out of gamut on a 240² sweep of (θ, φ), at 0.975 of the largest scale that stays in gamut
(`checks/torus_gam.js`). They are calibration values under R-71, confirmed at the M7 gate with the rest of SM6's. It
spends L on position (SM11). **The honest cost:** a system exactly one period away gets the same colour; the legend
states the period in chart units, and "Left its starting tile" brightness separates such systems (RQ-506, RQ-507). The
table's circle × circle row is realised by it, for the plane, by making the plane periodic.

*Transcribed from the prototype (R-543; `SM_TORUS`, `smTorusXY`, `smTileXY`, `smNTheta`, `smTorusLab`, `SM_GRAD`).*
`SM_TORUS = { L0: 0.70, rL: 0.12, R: 0.074, r: 0.037 }`. On the plane `(X, Y)` are SM11's tile coordinates in chart
units at `n` = Repeats, whatever the tilings' own *squashed square* setting: `X = (u − x_min)/(x_max − x_min)·n`,
`Y = (v − y_min)/(y_max − y_min)·n_y`, `n_y = max(1, round(n·H/W))` (`round` half up), so one period is
`(x_max − x_min)/n` by `(y_max − y_min)/n_y` in chart units, the same size inside the square and past it; a system that
has left takes the navigation edge it left by, as the tilings do (SM11, SM5a). On the cylinder `X = (θ mod 2π)/2π` and
`Y = (v − y_min)/(y_max − y_min)·n`, so `s` repeats `n` times over the square's height whatever its aspect (the code
reaches it as SM11's `Y` at `n_θ` rescaled by `n/max(1, round(n_θ·H/W))`). Then `θ = 2πX`, `φ = 2πY`,
`c = R + r cos φ`, `(L, a, b) = (L₀ + r_L sin φ, c cos θ, c sin θ)`. It is offered on the plane and cylinder quotients
only, not on the sphere or the simplex. The largest in-gamut scale of `(R, r)` at `L₀ = 0.70`, `r_L = 0.12` is 1.026 on
torus_gam.js's 120² sweep (1.0259 on 240²), so the constants sit at 0.9745 of it (R-543's check,
`docs/incoming/2026-10-21_state_map_tiling/checks/r543_tiling_check.py`).

*Transcribed from the prototype (R-542; `smPosLab`, `smCompact`, `oklabToRgb`).* OKLab to sRGB is
`principia_dd_colouring.md` §3.1's, each linear channel clamped to [0, 1] before the transfer, so an in-gamut map is
untouched. The simplex's three hue directions are `ûᵢ = (cos(hᵢ + 0.5), sin(hᵢ + 0.5))`, `hᵢ = 0, 2π/3, 4π/3`, for the
masses `m₁, m₂, m₃` in that order (`0.15 = 1.5 × 0.10`). The cylinder's `κ` is 0.6. The compaction per target: the latent
chart `(tanh(z_α/3), tanh(z_β/3))`, the latent momenta `tanh(·/0.3)` on each, a non-finite result read as 0; the tilted
charts `(tanh(s/π), −(2φ/π − 1))`, `φ` linear; `ν × dilation` `(2(ν − 1/32)/(30/32) − 1, tanh(h/0.6))`, `ν` linear on its
bounded range; the simplex `(2u − 1, 2v − 1)`, linear.

## SM7. Colour frame and deep zoom

- **Chart (default)**: absolute. The same colour means the same place in every view, so views are comparable.
- **View (option, the view's own chart only)**: `s = tanh((x − view centre)/view half-width)`.
  - It is needed at depth: in the absolute frame, the `t = 0` picture at a deep zoom is one flat colour, because the
    whole window is inside one colour step.
  - At `t > 0` most systems leave the window and saturate the relative map. That is expected and stated.
- **Precision**: the state is f32. The `t = 0` identity holds to **4e-7 (latent), 1.5e-6 (sphere), 5e-6 (simplex),
  2.7e-8 (momenta), 7e-8 (θ × h), 1.9e-6 (tilted E, F)** in chart units. On ν × h it holds to **4.9e-4**, which is the
  demo's 1/1000 curve table, not the method.

*Transcribed from the prototype (R-542; `smCompact`).* In the view frame the vertical axis is signed by the view's own
orientation (`−tanh((y − c_y)/h_y) · sign(y_max − y_min)`), so up on the screen is `+s_y` on every chart. The prototype's
frame is a control, "Colour window", with the target chart (absolute) as the default.

## SM8. Image mode

- The picture is laid on `T`'s home square (equirect on the sphere; θ across on the cylinder).
- Past the square on an affine axis it is **mirror-repeated and dimmed**, so it stays continuous and says it is outside.
  - *Amended by R-543:* that is now the default of a wrap mode. Past the square the picture follows its wrap mode,
    **Past its square**: Repeat · Mirror · Clamp, with a separate tickbox, **Dim past the square**, which dims it so it
    says it is outside. Repeat is `s − ⌊s⌋`, Mirror the triangle wave, Clamp the edge colour. **The defaults are Mirror
    with dimming on**, so every saved stain renders bitwise as before. Mirror stays right for photos, which stay
    continuous across the edge. For a picture made of cells it is wrong: a mirror repeats the edge row, so the same cell
    appears on both sides of the square's edge (one bright, one dim) and a checkerboard stops being a checkerboard; and
    past the edge the picture runs backwards, so a system far outside gets the colour of one just inside. Repeat tiles a
    tileable picture over the plane with no seam. The hovered pixel's readout and the graph strip name the wrap mode
    ("Picture · repeat").
- The picture can be built in (checkerboard, grid, rings) or the user's own, uploaded.
- Built in too: **Checkerboard, tileable** (R-543), 8 × 8 cells coloured by the Torus gradient at their centres, so its
  colours wrap and under Repeat it has no seam. The existing Checkerboard stays as it is: under Repeat its parity holds
  but its colour ramp jumps at the seams, which is the picture's property, not the wrap mode's.
- Played over time, a picture laid on the chart is stretched along the flow: smooth on regular islands, shredded where
  the dynamics are chaotic. That is the FTLE's meaning, seen directly; it is not a measurement of it.

*Transcribed from the prototype (R-542; `smImage`, `smImageUV`, `smPaint`).* The picture is 256 × 256 texels, looked up
nearest, at `(u, v)` the point's place in the target's square (`θ/2π, φ/π` on the sphere; `θ/2π` and `(0.6 − h)/1.2` on
the cylinder; the chart's `[x_min, x_max] × [y_min, y_max]` elsewhere), each coordinate mirror-repeated by the triangle
wave `s ↦ s mod 2, 2 − s past 1`, and multiplied by 0.62 outside `[0, 1]` (the cylinder's `θ` never is). The built-ins:
an 8 × 8 checkerboard, its cells at OKLab `L = 0.82` or `0.50`, `(a, b) = 0.12((i + ½)/4 − 1, 1 − (j + ½)/4)`; a 16 × 16
grid of 2 px lines `#f2efe6` on `#101114`, with a red bar `#ff5a4f` along the top half of the top edge and a blue bar
`#4fb6ff` down the top half of the left edge, so orientation reads; and 18 rings, alternately `L = 0.80` and `0.42`,
`(a, b) = 0.1(cos r, sin r)`. The user's picture is scaled to the same 256 × 256. Layers (SM11) apply over a picture as
over any base. The brightness slot does not apply under Picture in the prototype (RQ-499).

*Transcribed from the prototype (R-543; `smWrapFn`, `smImage`'s `tchecker` branch, `smPaint`, `smSyncUI`).* The
paragraph above is the default, Mirror with dimming on. The wrap mode maps each coordinate before the texel look-up
`min(N − 1, ⌊w(s)·N⌋)`: Repeat `w(s) = s − ⌊s⌋`; Mirror the triangle wave above; Clamp `w(s) = max(0, min(1 − 10⁻⁹, s))`.
Dimming multiplies the texel by 0.62 where `(u, v)` lies outside `[0, 1]²` and the tickbox is on, whatever the wrap
mode. At the square's closed far edge `s = 1`, which is inside it, Repeat reads texel 0 and Mirror texel `N − 1`.
**Checkerboard, tileable:** cell `(i, j)` of 8 × 8 is SM6's torus at its centre, `θ = 2π(i + ½)/8`, `φ = 2π(j + ½)/8`,
chroma × 0.8 and `r_L` × 0.8 about `L₀`, then ± 0.07 in `L` by parity: `L = L₀ + 0.8 r_L sin φ + 0.07` where `i + j` is
odd and `− 0.07` where it is even, `(a, b) = 0.8 (R + r cos φ)(cos θ, sin θ)`; taken to 8-bit sRGB as the other
built-ins, its linear channels lie in [0.035, 0.972], 0 of 64 cells at a channel limit (R-543's check). The picture's
menu lists Checkerboard, Checkerboard, tileable, Grid, Rings and Your image…; Past its square offers "Repeat (tiles the
plane)", "Mirror (for photos)" and "Clamp to the edge". The graph strip reads "Picture · repeat" (or mirror, clamp);
the prototype's hovered-pixel readout does not name the wrap mode (RQ-506).

## SM9. Legend and the hovered pixel

The legend is the colour map (or the picture) over `T`'s square, with the hovered pixel's system marked where it is
now. On the view's own chart, the marker sits where the hovered pixel's current colour sits in the legend. The trace
of the same pixel would run to the same place: the trace is this field for one pixel. The prototype draws no trace,
only a ring on the hovered pixel.

The marker is computed with the grid's own integrator settings, so it agrees with its pixel exactly.

*Transcribed from the prototype (R-542; `smLegend`).* The legend keeps the target's aspect ratio; past the simplex's
triangle it is `rgb(18, 19, 21)`; it is drawn with the colour slot's base and layers and no brightness. The marker is a
6 px ring, white when inside the legend's square and grey `#a8a8a8` when past it, pinned 5 px inside the edge; on the
sphere and the cylinder its `θ` is wrapped. A system that has left is marked at its clamped point; with "no value" it is
not marked and the readout says why.

## SM10. Time, ensemble and fade

- **Time**: it is the field at the playhead. Scrubbing back is re-integration, as for every live field (temporal
  architecture note). The prototype instead keeps states every Δt = 0.25 to the horizon, so it can scrub instantly;
  that is a demo device, not a design.
- **Ensemble**: the nominal copy (copy 0). An OKLab mean over the `E+1` copies would turn undetermined regions grey,
  which is informative but not one-to-one (Q5).
- **Fade off the slice**: opt-in, off by default, view's own chart only (R-509's distance, as the trace).
  A fade spends lightness, so it is stated as breaking injectivity.

*Transcribed from the prototype (R-542; `smPaint`).* The fade is applied as the pixel's opacity over the canvas,
`a = max(0.18, 1/(1 + (d/0.35)²))`, `d` the off-slice distance the trace's fade reads (the chart's projection's distance,
or the momentum proxy `|q|`, R-509), a non-finite `d` faded to the floor; the trace's own fade is `max(0.18, e^{−d/1.2})`
(RQ-499). The prototype integrates with Dormand–Prince 5(4) at relative tolerance `10⁻¹⁰`, `h ≤ 0.02`, landing on each
stored time, to the horizon `t = 20`; a failed step stores NaN from there on ("not data: never held"). These are the
demo's, not the app's: the app reads the live `SimState` (SM2).

## SM11. Colouring Chart position: a base, then layers

*Revised three times on 21 Oct 2026 (§0's last five quotes). The first version put one sphere kernel on every chart;
the second invented separate node kinds and mixed the source and brightness nodes into them; the third tied the sphere
to Shape n̂. This one: Chart position is one node, on any chart including the shape sphere, and its colour slot is
colour composition §1 as it stands — a base and layers — with the chart's point where Shape n̂ has n̂.*

*Port's note (R-542).* The prototype's code reads the shape sphere target from the signed `n̂` (payload §5's `n`), so on
the shape sphere the two nodes read the same point, as this section says, and SM12's "Same site blend" holds as
written; SM1's fold, under which they would read mirror points where `n₃ < 0`, is queued against SM4 and the code
(RQ-498, the code applied meanwhile). *Was:* "The encode folds the mirror …, so `E(x)` lies on one hemisphere, while
Shape n̂'s `n` (payload §5) keeps its sign: on the shape sphere the two nodes read the same point only where `n₃ ≥ 0`,
and SM12's "Same site blend" and SM7's identity are measured there until RQ-498 is ruled." Main's Shape n̂ is a `vector` subtype, its colour node the SiteBlend editor (render GUI
spec Part II §3, §6, where a subtype is no hard gate and the inspector morphs) (RQ-497).

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
| **Gradient**, added by R-543 | Family B, as above | — | **Torus**: periodic in chart units, `n` periods across the square, one-to-one within a period, seamless across periods (SM6) | **Torus**: θ round the hue, `s` repeating seamlessly (SM6) | — |
| **Picture**, added by R-543 | past its square by its wrap mode, Repeat · Mirror · Clamp, with Dim past the square; Mirror and dimming on by default (SM8); built in also **Checkerboard, tileable** | — | past the square by the wrap mode | in `s` by the wrap mode | — |

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
The Torus gradient (R-543) spends L on position too, so a brightness node after it raises the same "no longer
one-to-one" warning.

**Brightness, unchanged in meaning** and outside the colour slot: displacement `|e(t) − e(0)|`, a straight-line distance
on the quotient (`e` = n̂, `(cos θ, sin θ, s)`, `(sₓ, s_y)` or the masses: a distance only, never a colour); *left its
starting tile*, which under Chart position compares the Palette's tile now and at `t = 0`, and under Shape n̂ the
nearest site.

**Only the 2D and 3D gradients must be one-to-one (SM6).** Site blends, palettes and layers are classifications, many-to-one by
design.

The Torus gradient (R-543) is one-to-one within a period and repeats exactly across periods: a system one period away
has the same colour, its cost (SM6).

**Fields that need history stay out of scope without a struct change** (Q7).

*Transcribed from the prototype (R-542; `smGradLab`, `smRamp`, `smFSitesLab`, `smSites`, `smSphereBlend`,
`smPattern`, `smTileXY`, `smPaletteLab`, `smLayerOn`, `smApplyLayers`, `smBright`, `smOccupant`).*
- **Tile coordinates.** A point's tile coordinates `(X, Y)` for spacing `n`: on the sphere `X = (θ mod 2π)/2π · n_θ` and
  `Y = φ/π · n_θ/2` with `n_θ = 6·max(1, round(n/6))`; on the simplex `X = n(m₂ + m₃/2)`, `Y = n m₃ √3/2`, the triangle
  itself; on a flat chart in chart units `X = (u − x_min)/(x_max − x_min)·n_x`, `Y = (v − y_min)/(y_max − y_min)·n_y`,
  with `n_x = n` (on the cylinder `n_θ`, `θ` wrapped) and `n_y = max(1, round(n_x·H/W))` from the chart's aspect; in the
  squashed square `X = (sₓ + 1)/2·n`, `Y = (s_y + 1)/2·n` (offered on the plane only).
- **Tilings.** Square `(⌊X⌋, ⌊Y⌋)`; stripes `⌊X⌋` or `⌊Y⌋`; brick: row `j = ⌊Y⌋`, `i = ⌊(X + (j mod 2))/2⌋ − ⌈j/2⌉`;
  triangles: `j' = Y/(√3/2)`, `i' = X − j'/2`, the cell `(⌊i'⌋, ⌊j'⌋)` and its orientation `o = 1` where
  `frac(i') + frac(j') > 1`; hexagons pointy-top, one unit wide, by cube rounding of `q = (√3/3·X − Y/3)/R`,
  `r = (2/3·Y)/R`, `R = 1/√3`. On the sphere, latitude × longitude cells are the square tiling of `(X, Y)`; Fibonacci
  cells are the nearest of `n²` Fibonacci points; on the simplex stripes are `⌊n·m₂⌋` or `⌊n·m₃⌋` (at most `n − 1`).
- **Swatches.** By index: `(L, a, b) = (0.72, 0.08(i mod 3 − 1), 0.08(j mod 3 − 1))`; triangles take `L = 0.75` where
  `o = 1` and `0.69` where `o = 0` in place of 0.72 (RQ-499); Fibonacci cells always take the golden-angle hue of their
  index. Golden-angle: hue `((7919 i' + 104729 j)·137.50776°) mod 360°` with `i' = 2i + o`, at OKLCH `L = 0.72`,
  `C = 0.09`. The 1D ramp's three stops are sRGB `(59, 47, 128)`, `(33, 144, 141)` and `(253, 231, 37)` taken to OKLab
  and interpolated linearly there; "out from the centre" is `|s|/√2`; the hue circle is OKLCH `L = 0.75`, `C = 0.1`.
- **Site blend on a flat point.** The 5 × 5 lattice sites round the point in tile coordinates (square: cell centres;
  hex: `(q + r/2, r√3/2)`), their similarity `−|(X, Y) − p|²`, the site's index taken mod `n_θ` round θ on the
  cylinder; the kernels are the sphere's (nearest; top-3; vMF `w = e^{κ(d − d_max)}`), `κ` 1, 2, **3**, 5 or 8;
  defaults hex lattice, 6 across, index swatches, vMF, OKLab. RGB blending is in linear sRGB.
- **Site blend on the sphere.** The similarity is `n̂·p − 1`; the sets `axes6`, `corner8`, `ico12`, `fib(N)` and
  `ring(N)` (N 8 to 128, 32 by default); swatches golden-angle by default (`L` cycling 0.50, 0.66, 0.80, `C = 0.085`, hue
  steps of 2.399963 rad), the Full and Okabe–Ito hue tables (OKLCH `L = 0.72`, `C = 0.11`) with six sites, or a
  gradient A → B; kernel nearest (the discrete path), top-3, or vMF with `κ` 0.5 to 12 (3 by default). It is one
  function, shared by both nodes.
- **Layers.** The prototype offers three layer slots, each a pattern, `n` (4, **6**, 8, 12, 18, 24), a colour (OKLab
  `L` 0.25 dark, 0.96 light, 0.08 black, 1.00 white, `a = b = 0`) and an opacity (25, **50**, 75, 100 %), mixed in
  OKLab, `c ← (1 − α)c + α·c_layer`, in slot order, wherever the pattern is in band. Rings are centred on the chart's
  centre; dots are within 0.13 of a cell centre (on the sphere, within `0.13·√(4π/N)` of a Fibonacci point,
  `N = n²`); a Fibonacci cell edge is a point `0.045·√(4π/N)` away, along either tangent, in another cell.
- **Brightness.** Displacement: `B = 0.18 + 0.66·min(1, |e(t) − e(0)|/(½·D))`, `D` the quotient's diameter (sphere 2,
  cylinder `2√2`, simplex `√2`, plane `2√2`); Left its starting tile: `B = 0.30` if the tile (Shape n̂: the nearest site)
  is the same at `t` and `t = 0`, else `0.78`. The `t = 0` point is the pixel's own IC through the projection, with no
  march. Replace-L then sets `L = B` and scales `(a, b)` by `min(1, B/0.75, (1 − B)/0.35)` (SM15; RQ-496).
- **The Torus's Repeats** (R-543; `sm_gn`). 1, 2, 3, 4, 6 or 8 across the square, **2** by default, shown only when Torus
  is chosen; on the cylinder θ stays once round whatever it is. Its "no longer one-to-one" warning under a brightness
  node is `smSpendsL`'s, every plane Gradient but 2D and every cylinder Gradient but round θ.
  *Transcribed from the prototype (R-543, version 9; `sm_gn`, `SM_GRAD`).* Repeats offers "Once across the square",
  "2 across", "3 across", "4 across", "6 across" and "8 across", in that order, "2 across" selected. The Gradient
  picker lists the Torus second: on the plane "Torus: repeats seamlessly over the whole plane", after "2D: x → a, y → b
  (one lightness)"; on the cylinder "Torus: θ round the hue, s repeats seamlessly", after "2D: θ round the hue circle,
  s up L".

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
| Torus seam (R-543) | colour at (u, v) equals colour at (u + P, v) and (u, v + P_y) to 1e-12 over a sweep reaching well past the square (prototype: 2.7e-17) | a period computed in squashed or compacted units; an off-by-one in the period count |
| Torus one-to-one (R-543) | within one period, distinct grid points have distinct colours (minimum OKLab separation > 0 on a 200² grid) | R ≤ r, or L and chroma swapped so the tube self-intersects |
| Torus gamut (R-543) | 0 samples out of sRGB on a 240² (θ, φ) sweep, with no clipping in the shader path | constants raised past the gamut, or a per-pixel clamp added |
| Wrap defaults bitwise (R-543) | a stain saved before this change renders bitwise identically (Gradient 2D, Picture, Palette hex; the prototype checks all three) | the default wrap or dim flipped |
| Wrap arm (R-543) | the same Picture under Repeat and under Mirror differs past the square and is identical inside it | a wrap setting that is not wired through |
| Tileable checker seam (R-543) | under Repeat, the cell across each seam has opposite parity and the colour difference across the seam equals that between neighbouring cells inside | the old checkerboard silently substituted |

*Port's note (R-542).* Every test keeps its failing arm as written; each is a requirement's verify (R-542 H-items).
Where an arm meets the corpus or the prototype's code it is queued, not changed: Four results' "neither is drawn as a
colour" against SM5a's default clamp (RQ-498); Shape n̂ unchanged's "Position" and "Bands" against main's Shape n̂
(RQ-497); Replace-L in gamut against main's Replace-L (RQ-496).

*Port's note (R-543).* The six R-543 rows are the follow-up's §5, verbatim, each a requirement's verify. They were run
on a transcription of the prototype, `docs/incoming/2026-10-21_state_map_tiling/checks/r543_tiling_check.py`
(`python3 -I`; its output is in R-543's PR), with each arm shown to fire: Torus seam 1.6e-14 over four charts (the
latent chart, the tilted chart, ν × dilation, the cylinder) and every Repeats, out to ten squares past each edge; Torus
one-to-one 1.16e-3; Torus gamut 0 of 240², none changed by a clamp; Wrap arm 0 differing inside the square and 19,200
of 24,000 past it; Tileable checker seam's steps equal to 6e-17. Two readings, applied per R-369 (R-543 H4, H5): **One-to-one's
arms.** On the 200² grid R = r gives a separation of 0, but R < r and the swap leave the grid's minimum above 0 (2.1e-6
and 1.4e-5), because the tube meets the L axis between grid rows; so the test also asserts, on the map's output, a
signed radial chroma `(a, b)·(cos θ, sin θ)` > 0 at every grid point, the tube clear of the L axis, which fires on every
arm (0.037 at the constants; 0, −0.007 and −0.046 on the arms). **The seam's step along y.** Inside a column the step between
neighbouring cells varies with φ, so the step across the y seam is held to the picture's own rule from cell 7 to
cell 8, which wraps to cell 0; along x every neighbouring step in a row is the same, the seam's included. Wrap defaults
bitwise needs the app and is TASK-M7-49's; on the transcription version 9's lookup at its defaults equals version 8's on
every texel, and flipping either default fires.
*Amended by R-543 H11.* The Wrap arm's "inside" is `[0, 1)²`: at the closed far edge `s = 1`, inside the square (SM8),
Repeat reads texel 0 and Mirror texel `N − 1`, so the test asserts the two modes identical on the half-open square and
that edge rule on a sweep reaching `s = 1` (on the transcription 0 of 25,600 differ on `[0, 1)²`, 321 of 321 far-edge
samples read the stated texels, and the arm that moves the rule fires).

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

*Port's note (R-542).* Q1 to Q9 are RQ-480 to RQ-488, each with its recommendation as option (a), unruled; each (a) is
applied meanwhile, as the prototype has it, and a ruling otherwise is a follow-up change. Q4 records the existing vMF
shape colouring and changes nothing in it.

## SM14. What this changes

| Doc | Change |
|---|---|
| `principia_dd_simstate_payload.md` §5 | the derived table gains "chart position at the playhead", from `r, p`, the masses and the target chart; stores nothing |
| `principia_colour_composition.md` §3, §4.1 | the state map as a full-colour occupant (owns L; brightness None); image mode's texture input |
| `principia_render_gui_spec.md` | SM15: the `point⟨q⟩` field subtype, the two Where sources, the four colour nodes and two brightness nodes, the Problems entries, the presets, and the 2D legend |
| `principia_dd_colouring.md` §3.2 | Q4's sentence |
| `principia_colour_composition.md` §1–§2 | Shape n̂'s nodes unchanged. Added (SM11): a flat point as an input beside n̂; its Gradient fields; lattice site sets (square, hex) with flat distance; the Palette (categorical by tile); band patterns for layers (grid, checker, tile edges, stripes, rings, site dots) |
| none of the contracts' layouts | (SM2) |

*Port's note (R-542).* Each row is applied in the doc it names, citing R-542. "The two Where sources" reads as §0
settles it: one node, Chart position, with a time setting, a brightness field that compares two times taking two
copies of it (SM15).

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
The first warning holds for the Torus gradient too (R-543), which spends L on position.

**Presets** (prototype `smPreset`): Classic shape colour (the Shape n̂ node → Site blend, as today); and on Chart
position, on any chart: Where (Gradient 2D, 3D on the sphere); Graph paper (Gradient + checkerboard + grid lines); Basins
(Palette: hexagons, Fibonacci cells on the sphere); Displacement (Gradient + Displacement); Flow (Picture); Mixing
(Palette: square, lat × lon on the sphere, + Left its starting tile). A preset inserts an ordinary editable graph.
*Amended by R-543:* Flow is Picture over **Checkerboard, tileable**, with **Repeat** and dimming **off**, the one
deliberate visual change of the follow-up.

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

*Transcribed from the prototype (R-542; `smPreset`, `smSyncUI`, `smSpendsL`).* The preset buttons read "Classic shape
colour", "Where now", "Graph paper", "Basin tiles", "Displacement", "Picture flow" and "Mixing" (the file and board 5
name them Where, Basins and Flow; RQ-499). Each preset sets: Classic — the shape sphere chart, the Shape n̂ node, Site
blend `axes6`, Full swatches, vMF κ 3, OKLab, no brightness; Where now — Gradient 2D, the view's own chart; Graph paper
— Gradient 2D, layer 1 checkerboard ×6 white 25 %, layer 2 grid lines ×6 dark 75 %; Basin tiles — Palette, 6 across, by
index, hexagons (Fibonacci cells on the sphere); Displacement — Gradient 2D with Displacement; Picture flow — Picture,
the checkerboard; Mixing — Palette, 6 across, by index, square (lat × lon on the sphere), with Left its starting tile.
Every preset but Classic clears the layers it does not set and takes Chart position as the source. Swapping the
source maps Gradient ↔ Position, Site blend ↔ Site blend, Palette ↔ Bands; Picture stays; the layers are hidden, not
cleared, under Shape n̂. The base spends L, and a brightness node raises the warning, under Shape n̂ for Position and
Bands, and under Chart position for a Gradient other than the plane's 2D, the sphere's longitude and the cylinder's
round θ. The prototype's Shape n̂ node offers Position (linear in n̂, SM6's sphere map), Site blend and Bands over n̂ (a
checkerboard of `2n` cells round θ and `n` down φ in `L` 0.80 / 0.42 with the position map's `(a, b)`); board 4's last
paragraph, where the layers have no sphere form and the flat base takes "its sphere counterpart's place" when Chart
position's chart becomes the sphere, is the third revision's, before SM11 gave Chart position the sphere's own bases and
layers; the file and the code have Chart position keep its bases on the sphere (RQ-497, RQ-499).

*Transcribed from the prototype's version 9 (R-543; `smPreset`).* The Flow preset (its button "Picture flow", RQ-499)
sets Picture, **Checkerboard, tileable**, Past its square **Repeat** and Dim past the square **off**, with no brightness,
in place of version 8's Picture over the Checkerboard at the defaults: the one deliberate visual change of R-543. No
other preset sets the wrap mode or the dimming.
