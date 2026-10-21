# Principia — chart & decoder contract

*The core of the whole design. Physics → decoder → charts → slice/tilt → well-posedness. Read this before the core-design seams and validation notes; those two sit underneath this one.*

---

## Part 1 — The physics: what the 8D actually is

Planar three-body problem. Degrees of freedom, accounted honestly:

| Stage | DOF | |
|---|---|---|
| Full phase space | 12 | 3 bodies × 2D × (position + momentum) |
| After CoM removal | 8 | −2 CoM position, −2 CoM momentum |
| + mass simplex | (2) | masses normalised Σmᵢ = 1 → 2 DOF |
| **Before gauge fixing** | **10** | 4 config + 4 momentum + 2 mass |
| Fix rotation (pin ρ̃ → +x) + scale (R̃ = 1) | −2 | removes 2 configuration DOF |
| **Symmetry-reduced manifold** | **8** | **2 config ⊕ 4 momentum ⊕ 2 mass** |

**The load-bearing correction:** the masses are *inside* the 8, not an external context bolted on. The 8D is **not** pure phase space. It is `2 config ⊕ 4 momentum ⊕ 2 mass`. Anything you might want to vary — mass ratio included — is a direction in this one space.

**What each block is:**

- **Configuration (2 DOF).** After rotation + scale are gauged away, what remains is the pure *shape* of the triangle — a point on the **shape sphere S²**, coordinates `(α, β)` (equivalently `(θ, φ)`). Scale is gauged legitimately because Newtonian gravity has the similarity symmetry `r → λr, t → λ^{3/2}t`: different-size ICs are time-rescalings of one another, dynamically equivalent. Rotation is gauged because planar orientation is physically irrelevant (only `L_z` matters, and that lives in momentum).
- **Momentum (4 DOF).** Two planar Jacobi momentum vectors `(p_ρ, p_λ)`. The rest start sits at the origin. `L_z`, `E`, `KE`, `PE` are *derived* from this block (with masses/positions for `E`, `PE`), not independent axes.
- **Mass (2 DOF).** The 2-simplex `Δ² = {(m₀,m₁,m₂) : Σ = 1, mᵢ > 0}`. Two controls (softmax logits) cover it. Burrau `(c,b,a)/(a+b+c)` is one point; equal mass `(⅓,⅓,⅓)` the barycentre.

**Shape-sphere redundancy:** the canonical decode gauges the `λ̃_y → −λ̃_y` reflection, i.e. `(θ,φ) ∼ (θ, π−φ)` (θ azimuthal, φ polar from `+w`; R-14, chart reference §3.3). The φ hemispheres are reflection-equivalent — the chart is a **2-to-1 cover**. Render one hemisphere or flag the redundancy.

---

## Part 2 — The decoder

**Latent controls** `z ∈ ℝ⁸`. These are *controls*, not physical quantities — each is pushed through a smooth decoder (sigmoid / softmax / warp) to reach a physical value. Chart coordinates `(s,t) ∈ [0,1]²` map in via a chart map; the validated region is the unit hypercube `[0,1]⁸`. (Past `[0,1]²`, where the figure fills the window, an extended pixel's `z` may leave `[0,1]⁸`: the hypercube bound is the encode path's check, inverse_encode layer 1 on lookup and lock, and an extended pixel is checked by layers 2 and 3 only, Part 3, R-407.) **No latent coordinate is spent on a gauge direction** (this is why the old z₂, z₃ Cartesian-Jacobi directions were dropped in the 10D→8D cleanup).

**Block ordering (convention — fix once, then hold):**

```
z[0:2]  → configuration controls  → (α, β)              [config block]
z[2:6]  → momentum controls (4)   → (p_ρ, p_λ)          [momentum block]
z[6:8]  → mass controls (2)       → softmax → (m₀,m₁,m₂) [mass block]
```

**Factorised decode** `D = D_mass × D_cfg × D_mom`, i.e. `Y ≅ Y_mass × Y_cfg × Y_mom` with `2 × 2 × 4 = 8`:

- `D_mass`: 2 controls → softmax → simplex point.
- `D_cfg`: 2 controls → `(α, β)` → hyperspherical Jacobi → COM-frame shape (canonical gauge: ρ̃ on +x, R̃ = 1). *(Exact hyperspherical formulae: `principia_dd_decoder.md` §3.2. The contract only fixes the interface.)*
- `D_mom`: 4 controls → Jacobi momenta, either free (4 DOF) or produced under an invariant construction.

**Decode order is load-bearing:** `mass → config → momentum → derived invariants`. Derived quantities (`E`, `L_z`, `K`) are computed *after* the blocks they depend on. This is what makes invariant chart axes well-posed (Part 3, kind 3).

**Canonicaliser** `C`: translate to CoM, apply the gauge rotation/mirror. Every chart converges here.

**The one hard output rule:** the integrator's input is `(mᵢ, rᵢ, pᵢ)` and *nothing else, ever*. The compute kernel is chart-agnostic — it reads `(m,r,p)` and never learns which chart produced them. Chart proliferation cannot reach the integrator.

---

## Part 2.5 — Link functions & compactification (customisable)

Squashing functions appear in **three distinct roles**. They look alike (all map something unbounded into something constrained) but act at different stages, live in different keys, and must not be conflated.

| Role | Maps | Purpose | Change → re-integrate? |
|---|---|---|---|
| **Link function** | control `ℝ →` constrained physical param | enforce a block's **constraint geometry** | **yes** — changes the IC |
| **Axis warp** | chart coord `[0,1] →` position along axis | shape **sampling density** | **yes** — changes which IC each pixel is |
| **Colour compactification** | diagnostic scalar `→ [0,1]` | fit an unbounded diagnostic into colourmap range | **no** — colour-only, second pass |

The last one is what "decouples palette/compactification changes from integration cost" means: recolour the payload (`SimState` buffer) without re-running the integrator. Links and axis warps are on the **integration** side; colour compactification is on the **colour** side. This is the integrate/colour seam again.

### Link functions — the mass-simplex / logit question

A decoder stage reaches a *constrained* physical quantity from an unbounded control via a **link**. The choice is customisable, but bounded by the block's constraint type — a link is only valid if it *cannot* leave the constraint set.

| Block constraint | Physical target | Natural links | Inverse (encode/lookup) |
|---|---|---|---|
| Simplex Δ² | masses `(m₀,m₁,m₂)`, Σ=1 | softmax; temperature-softmax; pre-saturate logits with `μ_max·tanh` (the current default, `principia_dd_decoder.md` §3.1) | log-ratios → `artanh` |
| Bounded interval `(a,b)` | config angles `α, β`; capped momenta | scaled/shifted **sigmoid**; scaled **tanh** | `logit` / `artanh` |
| Positive half-line `(0,∞)` | any positive unbounded param | **softplus**, **exp** | `log` / inverse-softplus |
| Symmetric cap `(−c,c)` | signed capped param | `c·tanh`, `c·(2σ−1)` | `artanh` |
| Unbounded `ℝ` | free param | identity | identity |

The current decoder (`principia_dd_decoder.md` §3) fixes one link per block (mass = softmax ∘ `μ_max·tanh` saturation; config = sigmoid; free momentum = sigmoid). The generalisation you want: make the link a **registry entry selected per block/control**, constrained to be type-compatible with that block's codomain. `sigmoid ↔ tanh` are interchangeable on a bounded interval up to reparametrisation (`tanh x = 2σ(2x) − 1`); they differ only in slope profile, so the choice is a *sampling* preference there, not a constraint one.

### Three hard requirements on any registered link

1. **Constraint-preserving.** Output satisfies the block's constraint for *all* inputs — a simplex link lands in Δ², a bounded link stays in range. This is what "compactification" buys; a link that can escape the constraint is not admissible.
2. **Invertible, with a conditioned inverse.** Lock/lookup needs the inverse (`logit`, `artanh`, `log`). It blows up at the boundary → the ε clamps (`ε_z, ε_μ, ε_q = 10⁻⁶`) and the saturation constants `μ_max = 5`, `q_max = 2` (settled, R-10). A link without a stable inverse cannot support the encode path.
3. **Smooth (C¹).** The deep-zoom **linearised decoder** replaces the nonlinear decode with a local Jacobian; a non-differentiable link breaks that path.

### Links carry a measure

A link is **not measure-neutral**. Softmax with saturated logits is smooth but *not uniform* over the simplex; uniform-in-angle ≠ uniform-over-shapes (S² has its own area element); a power-law axis warp `K(t)=K_max·t^{γ_K}` deliberately biases density toward low `K`. Fine for exploration. But for any **quantitative** claim (basin fractions, island prevalence) you must either correct by the link's Jacobian or sample in a known measure. So **each registry entry should carry its log-det Jacobian**, so a quantitative pass can reweight. This is the one place a customisable link can silently corrupt a result — the freedom is real, but it moves the sampling measure, and the measure has to travel with the link.

### Integrity: the link is part of the experiment

The reason to expose the link is not flexibility — it's that a fixed, hidden link **launders an arbitrary choice into apparent fact**. "The Burrau point is a local maximum of ejection prevalence in the mass simplex" is really "…under softmax with `μ_max = 5`"; if the link is invisible, so is the qualifier — to a reader, and to you six months later. Three requirements make the choice honest:

1. **Swappable within the compatible set.** Any constraint-preserving link for that block (the rows above). Already stated.
2. **Recorded in provenance.** The link id per block lives in the exported ViewState, beside `z₀`, the basis, and the axis warps. A figure whose provenance doesn't pin its links is *not reproducible* — a re-run under a different default samples a different measure and can return a different answer. The link is part of the experiment, so it lives in the record.
3. **Falsifiable by re-measurement.** The real test: does a finding *survive* a link swap? Extremal under softmax **and** under a boundary-reaching simplex map **and** under a different logit saturation → a property of the *dynamics*. Moves when the link changes → a property of the *parametrisation*. This is the **tilt persistence test of Part 4 applied to the measure instead of the plane**: a structure is real iff it's invariant under the arbitrary choices you could have made differently.

**What makes an alternative worth offering.** Not any function with the right codomain — one whose Jacobian differs *enough to stress a claim*. Two links inducing near-identical measures agree trivially and prove nothing. The registry should span genuinely different sampling densities (softmax vs. a simplex map that reaches the edges; `t^γ` at spread-apart γ; sigmoid vs. a heavier-tailed bounded map) so that agreement across them *means* something. To make that deliberate, a registry entry carries — beyond forward, inverse, and log-det — a note on **which region of the block it over- and under-samples relative to uniform**, so a robustness sweep picks alternatives that disagree where it matters rather than three flavours of the same bias.

---

## Part 3 — Charts

**A chart is a 2D projection *through* the 8D manifold.** It is a mathematical object: a choice of 2D plane (plus fixed values for the other 6 directions). It is **not** a camera, **not** a view mode, **not** UI validation. Constraints like "this coordinate is non-negative" are *properties of the manifold in that chart*, not input checks.

**General interface:** `Φ : [0,1]² → Y` (decode parameter space) `→ z → (m,r,p)`. The **affine slice** is one implementation of `Φ`, not the definition:

```
z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂        (s,t) ∈ [0,1]²
```

`z₀` = slice centre (8D); `q₁, q₂` = basis vectors spanning the 2D plane. Nonlinear charts (curve, derived, physical-frame) need the general `Φ` and cannot be written affinely.

**Coordinate layers (coordinate note).** The `(s,t) ∈ [0,1]²` here is the **unsigned addressing space** of the current view (Y-up after the single framebuffer flip) — it chooses which quads are asked for; quad identity itself is taken in the slice plane's own frame (R-97), and no hash seed exists (R-100). The `(2s−1), (2t−1)` factors are exactly the **placement into signed IC-space**: they map `[0,1] → [−1,1]`, centred on `z₀`, so the *navigable plane is signed and centred* (pan either way from centre; the golden IC sits at the `z=0` centre). So `[0,1]` addressing and signed-centred IC values are one formula apart, by design — the `[0,1]` for indexing, the signed offset for the physical coordinate. Per-axis: the *plane offset* is always signed, but whether a given decoded quantity admits negatives varies (config/momentum signed; mass simplex / bounded params constrained) — the decoder maps signed plane coordinates to each axis's actual domain.

### The four axis kinds — a closed set

| Kind | What the axis is | How it pins DOF | Cost / caveat |
|---|---|---|---|
| **1. Raw latent** | a single `z_k` | direct assignment | trivial |
| **2. Derived-in-block** | scalar inside one block: `m₀`, `θ`, `φ` | inverts into its block | **under-determines the 2D block** → chart must declare a *residual convention* for the leftover within-block DOF |
| **3. Cross-block invariant** | `E`, `L_z`, `K`, `KE`, `PE` | *solves* into a downstream sector, upstream sectors read as fixed | must sit **downstream** of every block it depends on; brings **feasibility boundaries** (infeasible pixels tagged, not dropped); two coupled invariants → joint solve; **warped by default** (`principia_chart_reference.md` §2.1, evaluated per pixel), every pixel of `[0,1]²` feasible, a linear mapping an option per axis whose infeasible pixels are not a system (R-471) |
| **4. Coupled curve** | one scalar `ν` slaving *several* blocks via a fixed nonlinear embed (Burrau/Euclid: `ν → (α,β,μ₁,μ₂)`) | the embed pins config + mass together | tilt is only well-posed **at a point** (Part 4); decouple the coupling → decomposes back to a plain block axis |

Burrau introduces no fifth kind. Kind 4 *degrading* into kind 1/2 when you drop the mass coupling is the signal the taxonomy is at the right level.

**Residual conventions (kind 2)** are a first-class chart property. "The `m₀` axis" is ambiguous until you declare the fate of the other mass DOF: hold `m₁ = m₂`, hold the `m₁:m₂` ratio, or pin `z₇`. Different conventions give genuinely different one-parameter families — all valid, all must be *labelled*.

**Mixed-axis charts:** the two axes need not share a block. Any pair. When one axis is a configuration coordinate and the other its conjugate momentum, the render **is literally a Poincaré section** — it lifts the phase-space degeneracy (same shape, different momentum, different fate) that a config-only chart collapses.


### Past the unit square — each axis's extension type (R-407)

The figure can show more than the chart's `[0,1]²`: with the egui layer hidden (F3) or in present mode it fills the
window at the same scale (R-406), so where the window reaches past the depth-0 root its samples take `(s,t)` beyond
`[0,1]` on either axis. **Inside `[0,1]²` nothing changes.** Outside it, **each chart axis declares one of four
extension types**, and the type decides what the axis does past its edge:

| Type | Past the edge | Which axes |
|---|---|---|
| **affine** | the formula continues | basis vectors: the latent and flat slices, every latent direction; a physical-quantity axis mapped linearly onto its range (R-408's port, B1), for an energy-type axis its linear option (R-471, E2; warped, its default, it is bounded) |
| **periodic** | the axis wraps | azimuthal angles: the shape sphere's θ |
| **pole-crossing** | continues over the pole and back down the other side, the partner axis shifted by half its period | polar angles: the shape sphere's φ (its partner θ shifted by π) |
| **bounded** | not a system: blank, the view's background, in every view but the debug views, which hatch it; elsewhere the hatch only as the optional Overlays › Chart › forbidden region (R-467, R-470) | anything with a hard edge in its domain (the invariant warp's `t^γ_K` at `t < 0`); **the default for an axis that declares nothing** |

A custom chart inherits its extension from its axes; nothing is written per chart. The types are a property of each
axis beside its kind (the four kinds above), not a fifth kind. The axes' types act first: an affine axis passes its
coordinate through, a periodic one wraps it, a pole-crossing one reflects it over the pole and shifts its partner, and a
bounded one past its edge hatches the pixel. `Φ` and the validity check then run on what the types give.

**The universal fallback** needs no declaration: outside `[0,1]²`, a pixel whose `Φ` fails to evaluate (non-finite,
outside its domain), or whose state fails the existing validity check (R-26's `validate`;
`principia_inverse_encode_contract.md` § "Chart-aware validation", layers 2 and 3: outside the chart's feasible region,
a non-positive mass), is hatched as forbidden. The pixel's check is layers 2 and 3 only: layer 1 (the hypercube bounds,
"After encoding") is the encode path's, on lookup and lock, and never applied to a pixel inside `[0,1]²` either, so an
affine latent axis whose `z` leaves `[0,1]⁸` past the edge is not hatched for that. This covers physical-quantity axes
automatically: energy beyond what is reachable hatches. A hatched pixel is a labelled output, never dropped (Part 5): it is `decode_failed` (payload §2), it
is not integrated, and it is drawn as the next paragraph says (R-467).

**How a pixel that is not a system is drawn (R-467, amending R-446 and this part's clause 2 as it applies to
display).** In this part "hatched" names the classification, a pixel that is not a system, forbidden or the domain's
end; the types and the fallback decide it as above, and only its drawing is R-467's. **In a presentation view** (the
outcome view, the Chazy stains and every other stain that is not a debug view, in the shown layout, with the figure
filling the window under F3, in present mode and in an export) it draws the plain canvas background, the colour the
window is cleared to, with no hatch, legend key or share, as the area outside a map is blank. That holds on every pixel
this part hatches (the types, `validate`'s layers 2 and 3, and a failed `Φ`, read there as the chart map `Chart::map`,
`principia_chart_reference.md` §5.1), inside `[0,1]²` and past it, decided from the chart through REQ-COL-102's lane and
never from the payload's `decode_failed` detail; the primary ranges below decide only what is counted. **Only the debug
views draw it with the render contract's hatch** (`debug_invalid`, Part 5): View › Debug view
(`principia_render_gui_spec.md` §G2) hatches it under its "forbidden (chart)" key, with a count. Every other
`decode_failed` sample, the decoder's `DEGENERATE` tags and a non-finite decode included, draws blank, the canvas
background, in a presentation view, as a pixel that is not a system does, and the forbidden-region overlay draws over
it; `sim_failed` keeps `#141418` (R-511, ruling RQ-379; colour_composition §1.4). *Was (R-467):* "draws black
`#141418` in a presentation view (R-446, R-467; colour_composition §1.4)". **Inside `[0,1]²` every gallery chart is valid by
construction** (the feasibility warp, the simplex map, the decoder accepting any `z`), so a pixel hatched there is a
decoder bug: it shows only in the debug view and logs a console warning naming the chart and the pixel (R-467,
clause 4; `principia_render_gui_spec.md` §G12), and a sweep of every gallery chart over `[0,1]²` checks that the real
engine produces none (REQ-CHART-058). A Chart-builder chart follows R-471 on every
physical-quantity axis (R-506, ruling RQ-372): warped to its feasible range by default, so valid inside the square,
its failure there a decoder bug, warned, when all its physical-quantity axes are warped; under an axis's linear option
its infeasible pixels are not a system, blank, and warn nothing. *Was:* "Whether a Chart-builder chart, whose
physical-quantity axis has a range, may fail there is RQ-372." *Was (R-446):* "it is drawn with the render contract's hatch (`debug_invalid`, Part 5). The outcome
view keeps the hatch too, on every pixel this part hatches …".

**Blank, never a colour (R-470).** A pixel that is not a system, past a bounded edge, rejected by `validate`'s layer 2
or 3, or where `Φ` fails, draws **blank**, the view's background colour, which is R-467's canvas background. It is not
a palette colour in any view, the outcome view included; the debug views hatch it. The hatch survives outside them only
as the optional overlay Overlays › Chart › forbidden region, off by default, drawn over those pixels only
(`principia_render_gui_spec.md` §G2). NaN, `±∞`, `sim_failed`, `decode_failed` and the classification behind the hatch
show only in the debug views, R-467's View › Debug view among them; a presentation view draws a system's `sim_failed` or
non-finite sample `#141418` as R-467 has it, and a `decode_failed` sample the chart does not classify (a non-finite
decode, η_E's tags, the mass block's `M01_TINY`) blank, as a pixel that is not a system (R-511). *Was:* "a system's
failed or non-finite sample `#141418` as R-467 has it, and whether a `decode_failed` sample the chart does not
classify (a non-finite decode, η_E's tags) is instead not a system, blank, is RQ-379". R-408's distinction between the domain's end
and forbidden stays in the payload and the statistics below, unchanged (N4). Every pixel a chart draws is a valid
three-body system or explicitly not a system, and no chart, tilt, slice or extension integrates a state that is not a
system (R-469, P1). *Was (R-467), the bounded row's second cell:* "forbidden: the canvas background in a
presentation view, the hatch in the debug view (R-467)".

**Where the extension matters (R-468, context).** Extension past `[0,1]²` is free for every affine chart: the latent
chart and the flat and body-plane slices, tilted or rotated, are planes, and the square is only the current window
onto them; their axes are affine and the decoder accepts any `z`, so they never produce a forbidden pixel. The types
and the fallback matter for the charts whose square is their whole domain (the invariant warp, the mass simplex, the
Burrau family, and the shape sphere's spherical map, whose periodic and pole-crossing types redraw its own systems)
and for custom physical-quantity axes: past their edge, with F3 in its Extend mode, a presentation view shows the
canvas background and the debug view the hatch (R-467). The shape sphere's exponential map is one of the bounded
charts: its axes are bounded (`principia_chart_reference.md` §5.4's table) and its square covers only a disc of radius
`s√2` about `n₀`, so past its square a pixel is classified not a system and draws the background, though the sphere
holds systems there (physics review 5585225466 of PR #253, P6). *Was (R-468 as first ported):* "and the shape sphere,
whose periodic and pole-crossing types redraw its own systems". In Fit mode the figure is the view's square alone, the rest of the window the
canvas background (`principia_render_gui_spec.md` §G1).

**No invented continuations.** An axis gets anything beyond bounded only by declaring it, and any new extension type
needs physics review. The existing charts' types are in `principia_chart_reference.md` §5.4.

**Area statistics past `[0,1]²` count each system once, through the types (R-408).** Each type has a **primary
range**: periodic, one period of that axis (for the shape sphere's `θ = 2π·s`, the chart's `[0,1]` span); pole-crossing,
pole to pole (for the shape sphere's φ with both hemispheres drawn, the chart's `[0,1]` span); affine, unbounded;
bounded, its domain. A new periodic or pole-crossing declaration states its own primary range, under clause 3's physics
review (R-407). "Forbidden in view" (render_gui_spec §G7) and the area
statistics (W7, `system_image` below) count a visible pixel **only if every axis is inside its primary range**, so a
periodic or pole-crossing redraw is not counted again: a pixel past a pole carries its partner shifted by half a period
and is outside the pole-crossing axis's primary range, so it is excluded. **Pixels hatched because the domain ends
there** (a bounded axis past its edge, or a `Φ` that fails to evaluate) are not systems: they leave both the count and
the total. **Only states that fail the validity check count as forbidden.** A pixel is classified in this order: past
a bounded edge, the domain's end; outside a primary range, not counted; rejected by `validate`'s feasibility (layer 2,
on what the types give, before `Φ`), forbidden; a `Φ` that fails where layer 2 accepts, the domain's end; a decoded
state that fails layer 3, forbidden; any other, a counted system. The statistic reads the chart (the types, the
primary ranges, `validate`, `Φ`), not the payload's `decode_failed` detail, which does not tell the two hatches apart.
**Inside `[0,1]²` every statistic is unchanged.** With one hemisphere of the shape sphere drawn, φ's span is remapped
to run from the pole to the equator; past the equator its formula continues into the mirror hemisphere (real systems,
repeated), and its primary range is pole to equator, so that continuation is not counted (R-412, closing RQ-264;
built by TASK-M8-44, REQ-CHART-057).

---

## Part 4 — Navigation is chart construction (pan, slice, zoom, tilt, lock)

**The single fact this part rests on: there is no "view" separate from "chart."** The view state `(z₀, q₁, q₂)` *is* the chart. Every navigation gesture is an edit to that triple — nothing else exists to edit. An agent that builds a "camera" or "view mode" distinct from the chart has already gone wrong.

### The operation table

| Operation | Edits | Constraint | What the user sees |
|---|---|---|---|
| **Pan** | `z₀` | `Δz₀ ∈ span(q₁, q₂)` — move *within* the plane | same slice, different window |
| **Slice** | `z₀` | `Δz₀ ⊥ span(q₁, q₂)` — move *across* the plane, along a hidden direction | a parallel plane: every pixel's IC changes by the same hidden-coordinate step |
| **Zoom** | `q₁, q₂` | common scale factor (log-stepped) | same plane, narrower/wider window |
| **Tilt** | `q₁` or `q₂` | rotate toward a hidden direction (below) | the plane itself rotates through the 8D |

Pan and slice are the **same operation** — move `z₀` — decomposed by the plane. A free-mode slider sets one component of `z₀`, which is in general a pan+slice *mixture* (its basis vector is rarely exactly in or exactly orthogonal to the plane). Tilt and zoom are the **same kind** of operation — edit the basis. Because every tilted position is a full first-class chart, tilted charts serialise, save, and restore for free: ViewState already stores `(z₀, q₁, q₂)`.

**What each gesture does to the keys (R-92).** The sim key holds the **slice plane**: `z₀`'s out-of-plane part, `span{q₁, q₂}`, and the in-plane orientation. **In-plane pan and zoom re-address** (the same plane, different quads asked for); **slicing out of the plane, tilting and rotating re-integrate** (a new plane changes every quad's ICs); **the lock changes neither**.

**Filling the window is not a gesture (R-406, R-407).** With F3 or present mode hiding the chrome, the figure shows more
of the same chart, each axis past `[0,1]²` by its extension type (Part 3): `(z₀, q₁, q₂)` is unchanged and nothing is
edited.

**Navigation stops at the edge (R-469).** Pan, slice, zoom, tilt and lock are accepted only if the chart's centre (the
view's centre pixel, `z(½,½)`) still passes `validate`'s layers 2 and 3 after the edit. An edit that would cross stops
at the last valid value along its own path (bisection on the edit's fraction to adjacent f64 values), the drag staying
live up to there and going no further. It is one generic rule in the edit path, a function of the `Chart` trait and
`validate`, applied to every chart; no chart writes its own. Undo and the GUI state contract record the clamped value,
not the requested one, and the GUI's slider stops there while the readout names the check that stopped it
(`principia_render_gui_spec.md` §G2). Tilt and zoom are anchor-preserving (the lock, below), so they never move the
centre and pass unless they come with a centre edit; pan, slice, lock and an edit of a chart parameter that moves the
centre's state are the edits that stop.

### Tilt (basis edit)

Rotating a basis vector *within the 8D*, smoothly interpolating a plane direction toward a chosen hidden direction. For a raw/affine axis:

```
q'(τ) = normalise( cos τ · q + sin τ · d_target )        τ ∈ [−90°, +90°]
```

`τ = 0` → no tilt; `τ = ±90°` → fully committed to the target. **Sign matters** — the manifold is not symmetric under latent sign flips, so `+30°` and `−30°` reveal different cross-sections. The 8D has **6 directions orthogonal** to the current plane; the tilt selector offers those (plus named compound directions, below). Choosing which hidden direction to rotate into is the core hypothesis-testing gesture ("does this boundary persist as I rotate `q₂` toward `L_z`?").

In the dev GUI each tilt angle has its own target, picked from those directions: `τ₁` tilts `q₁` toward the latent axis `e_X` and `τ₂` tilts `q₂` toward `e_Y`, each chosen in the Tilt section ("tilt τ₁ into [X]", "tilt τ₂ into [Y]"), both defaulting to the slice normal, the one shared target the GUI used before (R-457; `principia_render_gui_spec.md` §G2). With `X = Y` the GUI's map is the code's one-target map that RQ-340 described, which R-474 rules (below); with `X ≠ Y` it is this section's two-target form, and the two tilted vectors are orthonormal. *Was (R-457):* "that RQ-340 describes, still open there".

**Two tilt maps (R-474, ruling RQ-340).** With one target (`X = Y`) and both angles set the GUI offers two maps, recorded as the view's `tilt_map`. **Tip as sheet**, the default (`sheet`): with `m = √(τ₁² + τ₂²)` the plane `span(e_a, e_b)` is turned rigidly by `m` about the in-plane axis `k = (τ₂ e_a − τ₁ e_b)/m` (Rodrigues' formula, in the 3-space `(e_a, e_b, e_X)`), so the plane tips toward `e_X` (its normal tipping away from `e_X`, toward `−(τ₁ e_a + τ₂ e_b)/m`; R-474 K16) and a single `τ₁` moves `q₁` toward `e_X` (*was:* "so its normal tips toward `e_X`"); the basis stays orthonormal, with no lock at 90°. **Lean each axis**, an option (`lean`), is the formula above per axis, `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_X`, skewed (`u₁·u₂ = sin τ₁ sin τ₂`). The two differ only with `X = Y` and both angles non-zero; a single tilt, or `X ≠ Y`, gives bitwise the same basis. **Over the pole:** Tip as sheet is not clamped at `m = 90°`; the angles are stored as set and the rotation is computed from them, wrapped internally only in total tip at 360° (for `m > 180°`, `τ·(m − 360)/m`, the same rotation), never folded onto the 180° partner `τ·(m − 180)/m`, which is the same plane mirrored. The range `τ ∈ [−90°, +90°]` above is Lean each axis's; under Tip as sheet each angle runs over `[−180°, 180°]`. A comparison of planes (a readout, a cache or dedupe keyed on the plane) compares normals up to sign and is never written back; the sim key keeps the plane and its in-plane orientation (R-92). Under Lean each axis an area statistic weights each pixel by the Gram factor `√(|u₁|²|u₂|² − (u₁·u₂)²)` (R-408); Tip as sheet needs none.

**Tilt on every chart (R-473).** The dev GUI tilts every chart, curved ones included. The tilt is a rotation in the 3-space of the two screen axes and the hidden target coordinate, `R(τ₁, τ₂)` from the active map, applied to the chart's **own parameters**, not to a tangent vector: the pixel at window offset `(u, v)` maps to `(a, b, d) = R·(u, v, 0)`; `a`, `b` are the chart's own axis parameters and go through its chart map exactly, so its axes keep their meaning; `d` is added to the hidden coordinate's value at the slice centre. With two targets (`X ≠ Y`) each axis turns toward its own, in the 4-space `(a, b, d_X, d_Y)`. Untilted, `R` is the identity and the chart is bitwise today's; on the latent chart T2 is exactly the basis edit above. A chart's hidden coordinates are the latent controls its axes do not write (Part 5's block-touch sets), and every hidden coordinate is tilted in its link's pre-image, a mass in its logit, never in the physical value, so every pixel stays a system (masses positive, summing to 1). On a curved chart the rotation compares a chart parameter and a hidden coordinate in units that do not depend on zoom (R-507, ruling RQ-375): each screen axis in units of the axis's full range at zoom 1 (the parameter span of the chart preset's window), the hidden coordinate in units of its own slider span; on the latent chart the tilt stays the basis edit above, in the latent coordinates. *Was:* "How a chart parameter and a hidden coordinate share a scale is RQ-375 (the human's proposal applied meanwhile on the curved charts: each screen axis in units of the window's half-width, the hidden coordinate in units of its own slider span)." Under Tip as sheet with one target and both angles set the rotation can take a window pixel inside `[−1, 1]²` to `(a, b)` outside the chart's square, where the axes' types act (Part 3): such a pixel is past the chart's edge, and blank past a bounded one (R-512, ruling RQ-402; past the energy warp's continuing `t > 1` side it is that side's system, R-508). *Was:* "where the axes' types act (Part 3); whether that is meant is RQ-402." A tilted chart extends past `[0,1]²` by its axes' types as an untilted one does (Part 3), and an energy-type axis's warp is re-evaluated per pixel under it (R-471). Slicing moves the slice centre along any hidden coordinate, subject to the stop above.

### Slice (centre edit, orthogonal component)

Fixing the six non-displayed coordinates to constants; changing the slice = stepping `z₀` along a hidden direction `d`: `z₀ ← z₀ + t·d`. Coordinate-agnostic; works in every chart. Every pixel's IC shifts by the identical hidden step — the plane translates parallel to itself.

### Directions are axis kinds — the unifying rule

**A slice direction or tilt target is the same object as a chart axis: one of the four kinds of Part 3, evaluated at a point.**

- **Raw latent** directions (`e_k`) are constant vectors — the same everywhere, no anchor needed.
- **Derived, invariant, and coupled** directions are *tangent vectors to constrained curves* — they depend on where you are, so they must be **evaluated at the chart centre**. "Increase E at fixed L_z" is a different vector at every point (it's a tangent *field*); the named compound directions (below) are exactly these, and this is why they must be recomputed when the centre changes. "Vary m₀ holding m₁:m₂" is a tangent to a curve through the centre, carrying the same **residual convention** as the corresponding axis kind.

Consequence, stated once and inherited everywhere: **the caveat "identical ICs except the one that varies" is true in *controls*, not automatically in *physical quantities*.** Slicing along raw `z₆` moves **all three masses** (the softmax couples them) — the line is "identical except one mass logit." Physical one-quantity lines (only `m₀`, only `E`) are derived/invariant *directions* with conventions, i.e. curves, not raw latent lines. The UI must label which it is showing.

**Named compound directions.** Predefined $\mathbf q$ vectors for physically meaningful orientations, used
as slice directions or tilt targets:

- **Mass perturbation away from Burrau:** the direction in the mass block from the Burrau logits toward equal masses.
- **Energy increase at fixed $L_z$:** a configuration-dependent direction in the momentum block.
- **Burrau-to-unconstrained morph:** tilt one basis vector from a configuration direction into a mass or
  momentum direction. This is the direct test of the Burrau hypothesis (`principia_chart_reference.md` §4.6).

**Their latent vectors** (defined per R-72, REQ-CHART-051; TASK-M2-22). Each is evaluated at the chart centre `z₀`,
under the chart's link selection, on the shared decode before any energy normalisation (`principia_chart_reference.md`
§0.1–§0.3, §0.6), and recomputed whenever `z₀` changes. Each is a unit vector of ℝ⁸ (Euclidean norm 1 in the latent
controls), zero outside the block named; where its construction gives no direction, the library reports it
unavailable at that centre rather than returning a vector.

- **Mass perturbation away from Burrau,** `d_m`, in the mass block `z[6:8]`: `d_m ∝ z_eq − z₀[6:8]`, where `z_eq` is the
  selected mass link's inverse at equal masses `(⅓, ⅓, ⅓)` (`(0, 0)` under the default softmax ∘ `μ_max·tanh`). It is
  the straight latent line from the centre's mass controls, the Burrau logits when the centre is a Burrau IC, to equal
  masses, which a slice step of `‖z_eq − z₀[6:8]‖` along it reaches exactly. Unavailable where `z₀[6:8] = z_eq`: the
  centre already has equal masses.
- **Energy increase at fixed `L_z`,** `d_E`, in the momentum block `z[2:6]`. Decode `z₀` to `(mᵢ, rᵢ, pᵢ)` and take
  `vᵢ = pᵢ/mᵢ`. The physical tangent is `δv = v − v⁽ᴸ⁾`, `v⁽ᴸ⁾ᵢ = (L_z/I)·J rᵢ`: the velocity with its rigid rotation
  projected out in `⟨·,·⟩_m` (`principia_chart_reference.md` §2.2 (i) and (ii)'s projection). It is the tangent at
  `s = 0` of the curve `v(s) = v⁽ᴸ⁾ + eˢ·(v − v⁽ᴸ⁾)`, along which `L_z` is exactly constant and
  `K = K_min + e²ˢ·(K − K_min)` increases: the steepest ascent of `K`, and of `E = U + K` at the centre's fixed
  configuration and masses, on the `L_z` level set in the kinetic metric. Where `‖δv‖²_m ≤ ε_w·‖v‖²_m` (`ε_w = 10⁻¹⁰`,
  §2.2's), at rest and on the rigid rotation `K = K_min`, where every direction on the level set raises `K` alike at
  second order, `δv` is §2.2 (ii)'s unit direction `w`, its seed decided on `z_α` (R-82): the direction the `(L_z, K)`
  construction adds energy along, so on that construction's image `d_E` is its `K` axis's tangent. `δpᵢ = mᵢ·δvᵢ` in
  Jacobi form is `δq = (δp_ρ, δp_λ)`, `δp_ρ = (m₀·δp₁ − m₁·δp₀)/M₀₁`, `δp_λ = δp₂`; it is pulled back through the
  selected momentum links, `δz_qk = δq_k / f_k′(z_qk)`, `f_k′ = exp(log_det_k)` (each registered momentum link
  increases), and normalised. Unavailable where an `f_k′` underflows to 0 (a saturated control).
- **Burrau-to-unconstrained morph,** a tilt `(source, target)`. The source is the basis vector with the larger share of
  its length in the configuration block, `‖q[0:2]‖/‖q‖`, `q₁` on a tie: the plane's configuration direction, on a
  Burrau slice its `ν` axis's tangent `γ′(ν₀)`. It is refused where neither basis vector has a configuration component.
  The target is into mass, `d_m`, or into momentum, `d_E`, with its component in `span(q₁, q₂)` projected out and
  renormalised, so the rotation leaves the plane: `τ = 0` is the slice the test starts from, `τ = ±90°` commits the
  source to the unconstrained direction, and `z(½,½) = z₀` throughout, the anchor-preserving persistence test of
  `principia_chart_reference.md` §4.6. Unavailable where that direction is unavailable or lies in the plane.

This rule retro-explains the curve-axis result: a curve axis has no single basis vector — `γ(ν)` bends — so tilting it means rotating its **tangent at the centre**:

```
q_tilt(τ) = normalise( cos τ · γ'(ν₀) + sin τ · d_target )
```

Free-floating curve-axis tilt is ill-defined only because the tangent changes along the curve. Pin the centre and the tangent is a fixed vector. This is the general pattern — *non-raw directions are tangents; tangents need a point* — of which "curve axes tilt only at a lock" is the special case.

**For the dev GUI's tilt this is amended (R-473):** the GUI tilts a curved chart, a coupled-curve (kind 4) axis
included, by the rotation on the chart's own parameters (§ "Tilt (basis edit)", "Tilt on every chart"), which needs no
tangent and so no lock; "curve axes tilt only at a lock" and "free-floating curve-axis tilt is ill-defined" hold for the
tangent construction above, which stays where this contract uses it: the lock and the direction library's
tangent-vector targets.

### The lock (projective microscope)

Every chart has a centre; point-dependent directions are always evaluated there. **The lock is simply the gesture that sets the centre to a chosen IC and pins it.** It is CPU-side chart construction, held in `SimConfig` (R-69) — nothing about the chart maths changes.

**Setting the lock.** Select the pixel at $(s,t)$ and snap the centre to its IC. For an affine chart,
$\mathbf z_{\mathrm{locked}} = \mathbf z_0 + (2s-1)\mathbf q_1 + (2t-1)\mathbf q_2$: CPU arithmetic, with no GPU readback.
For a nonlinear chart, replay $\Phi$ and $D$ on the CPU, or read the GPU buffer back.

The centre pixel has one special property: `z(½,½) = z₀` **regardless of the basis**. That property sorts the operations into two classes:

- **Anchor-preserving (basis edits): tilt, zoom, chart-mode switch.** The locked IC is the *fixed point* of the operation — the centre pixel's IC never changes; surrounding pixels are `z_locked + offset·q'(τ)`, same in-plane offsets, rotating directions. You watch the neighbourhood deform around an IC that is exactly constant. This is the persistence test, mechanically. Chart-mode switching keeps the anchor at centre while changing which degrees of freedom the surroundings explore ("this IC's neighbourhood under energy variation vs. under geometry variation").
- **Anchor-excursion (centre edits): slice, and slider moves in locked mode.** There is no way to change a hidden coordinate while keeping the point — slicing *necessarily* moves the centre IC. Locked slicing is therefore an **excursion along a line through the anchor**: `z_centre = z_anchor + δ`, with `δ` accumulating slice steps. The centre pixel is always "the anchor's IC except along the excursion direction(s)." The excursion has **memory and a way home**: `δ` is tracked explicitly, the anchor value is ghost-marked on each control, and a snap-back gesture returns `δ → 0` exactly (not approximately — the anchor is stored, not re-derived).

**Sliders are re-based, not frozen.** An earlier design froze the sliders in locked mode. Here, in locked mode, sliders are **re-based to the anchor, not frozen**. Each slider shows `z_anchor + δ` live, with the anchor value marked; moving one is an anchor-excursion along that control's direction. Frozen sliders give only the microscope; re-based sliders additionally give slice-through-the-lock — strictly more capability, same GPU interface. Unlock preserves the current `(z₀, q₁, q₂)` as the new free-mode state.

**Nonlinear charts:** the anchor is preserved by re-centring `Φ` on `z_locked` via the inverse-encode path (two options: CPU replicate-and-evaluate, or per-pixel readback). Feasibility applies: an excursion or tilt can carry the neighbourhood outside the feasible region of an invariant chart — those pixels are tagged, never dropped, exactly as for any chart.

### What the GPU knows about all of this: nothing

The kernel receives `(z₀, q₁, q₂, chart id + params)` — identical in free and locked mode, before and after any tilt, slice, or excursion. Pan, slice, zoom, tilt, lock, snap-back are **all CPU-side edits to the same uniform**. The lock (chart construction, R-69), the `δ` memory, the ghost marks, the direction library — none of it reaches the kernel. If an implementation finds itself adding a "locked" flag or a second code path to the kernel, it has misread this part. A curved chart's tilt (R-473) is a chart parameter like any other: its rotation `R`, its targets' indices and their scales, computed on the CPU from the stored angles and the map and sent as part of "chart id + params".

---

## Part 5 — Well-posedness and the validation contract

A chart is **well-posed iff its swept axes + conventions + slice pin all 8 DOF**: no block over- or under-determined. The usual case is 2 swept + 6 frozen. The exception is **annotation axes** (below): a chart may sweep only 1 physical DOF and render it across a 2D canvas, giving 1 swept + 7 frozen, provided the other axis is declared annotation-only.

**Per-axis metadata the validator needs:**

- **block-touch set** — which of {config, momentum, mass} the axis writes into. May be *empty* for an annotation axis.
- **annotation-only?** — if set, the axis drives *no* IC degree of freedom; it exists purely as a display/landmark coordinate (tick marks, lattice highlighting). The decoder ignores it. The Euclid plane (`principia_chart_reference.md` §4.5) is the canonical case: `Φ_Euclid` sweeps `ν` on one axis; the `m` axis is annotation (integer-lattice / primitive-triple landmarks only, "the decoder uses only ν"). A chart with an annotation axis is a **1-swept family rendered in 2D**, well-posed as 1 + 7, and must be *labelled as such* — it supports per-value statistics along the swept axis only, never 2D density claims across the annotation axis.

- **invariant?** — if so, which sector it solves into, its dependency set (must be downstream), **and whether it is `conserved_along_flow`**: `E` and `L_z` are constants of motion (a hover trace pins to a labelled dot); `K` is invariant-*constructed* but not conserved (a trace oscillates as KE↔PE exchanges). Consumers: the hover trace and any along-trajectory rendering.
- **residual convention** — for derived-in-block axes, how the leftover within-block DOF is pinned
- **curve?** — if so, tilt requires a lock; carries an `embed` map and its tangent `γ'` (the tangent tilt at a lock; the dev GUI's tilt of a curved chart is Part 4's rotation on the chart's own parameters, which needs none, R-473)
- **extension type** — affine, periodic, pole-crossing or bounded (the default when none is declared): what the axis does past `[0,1]²` (Part 3, R-407)

**Per-chart descriptors:**

- `system_image` — how the address map (what you turn) folds onto distinct systems (what the physics feels). The chart is *always* a genuine 2D thing you explore by two knobs; this only records redundancy. Four values, replacing the old one-off `has_redundant_hemisphere`:
  - **bijective** — every pixel is a distinct system (most charts).
  - **n-to-1** — a fixed finite number of pixels share each system. Carries the fold so downstream draws/labels one representative. The shape sphere is n-to-1 with n = 2: 2-to-1 over the φ hemispheres, which are reflection-equivalent (the canonical decode gauges `λ̃_y → −λ̃_y`, Part 1), so both decode to the same system (R-141). That is the spherical map's value. The exponential map's is computed from its cap (R-503, closing RQ-371; `principia_chart_reference.md` §3.3): bijective where the cap lies within one closed hemisphere; where it straddles the equator, n-to-1 with a **per-pixel fold**, at most 2: a pixel counts once unless its mirror `w → −w` is in view, a mirrored pair counting once (its `w > 0` pixel the representative), and an equator pixel counts once. Its radius is bounded at π; past it is the domain's end. *Was:* "The shape sphere is n-to-1 with n = 2", for both maps. Both are the untilted chart's values: under a tilt (R-473) two pixels fold only when their full `(a, b, d)` decode to the same system, so a tilted shape-sphere view is generically bijective, its fold taken over the tilted view's pixels (`principia_chart_reference.md` §3.3; applied per R-369, flagged for the human's veto).
  - **`DoubleCover`** — covers each shape twice, as two labelled systems (R-27, R-104, R-157): the full-range Burrau chart, where the leg swap relabels the bodies (`principia_chart_reference.md` §4.5). Carries the fold so downstream draws/labels one representative. *Was (R-104): the shape sphere's value too; R-141 made the shape sphere n-to-1, and R-157 keeps `DoubleCover` for the Burrau chart.*
  - **ray-degenerate** — whole lines of pixels map to the same system (for example, a chart sweeping both `m` and `n` continuously: rays through the origin are similarity classes, so the picture bands along rays). §4.5's Euclid plane is not this case: it sweeps only `ν`, its `m` axis annotation-only (above), and its full range is `DoubleCover` (R-27, R-157). Legitimate and often *pedagogically the point* — it makes the similarity symmetry visible — but the quantitative layer must not read areas as system fractions, and the UI should expect banding.

  The multiplicity is `[0,1]²`'s. Past `[0,1]²` (the window's extension, Part 3) it adds none: an area statistic counts a
  pixel only if every axis is inside its extension type's primary range, so a periodic or pole-crossing redraw is never
  counted again, and a pixel hatched where the domain ends leaves both the count and the total (R-408).

  The int `(m,n)` lattice is **bijective**: coprimality (`gcd=1`) is the lowest-terms rule, one address per ray, redundancy quotiented out — which is exactly why the discrete survey and a continuous `(m,n)` chart are different instruments over the same 1D curve of shapes.

- `forbids_energy_normalisation` — set on invariant charts where `E` is itself a coordinate (`(L_z,E)`, `(L_z,K)`); the validation pass refuses any config combining such a chart with an `E*` override — every `Some(E*)`, including `Some(0)` (R-25).
- `has_feasibility_boundary` — invariant charts; infeasible pixels are *tagged labelled outputs*, never dropped.
- `coupling` — kind-4 charts tie multiple blocks; flag same-block axis collisions.

**The check, in one line:** given each axis's block-touch set (empty for annotation), conventions, invariant/curve status, and the slice, confirm all 8 DOF are determined exactly once — counting annotation axes as contributing none. Refuse anything that leaves a block under-determined or double-writes one.

---

## Design axioms (the six that must survive contact with a code agent)

1. **No global parameter.** Everything variable lives in the 8D. Mass ratio is directions 6–7 — not special, not external. The chart picks two directions; the integrator reads all physics from the coordinate vector it receives.
2. **The decoder factorises.** `z⁸ → mass × config × momentum → (m,r,p)`. This is the first real artefact after the debug shaders; if it is wrong, everything downstream is noise.
3. **The integrator is stateless and has one input type.** `(m,r,p)`, forever. Everything converges through canonicalisation before it.
4. **Charts lower; they do not interpret.** The four axis kinds are a CPU authoring layer that **monomorphises** to a specialised decode+map per chart (a chart is a type parameter the Rust kernel specialises — lowering Part 2, core-design seam 2) — never a runtime axis-kind interpreter. Monomorphisation *is* the lowering; there is no WGSL interpreter to avoid on the compute side because the compute side is Rust → SPIR-V.
5. **No latent coordinate on a gauge direction.** Every slice direction is a meaningful physical variation.
6. **Navigation is chart construction.** There is no view/camera object separate from the chart: pan and slice edit `z₀`, zoom and tilt edit the basis, the lock pins the centre — all CPU-side edits to one uniform. Non-raw directions are tangents evaluated at the centre. The GPU has no locked mode, no view mode, no second code path.

---

*One manifold, projected many ways. Mass is a direction, not a mode. Slice moves the centre; tilt rotates the basis; the lock pins the centre; the GPU never learns any of it happened. The integrator never learns the chart's name.*
