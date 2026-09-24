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

**Latent controls** `z ∈ ℝ⁸`. These are *controls*, not physical quantities — each is pushed through a smooth decoder (sigmoid / softmax / warp) to reach a physical value. Chart coordinates `(s,t) ∈ [0,1]²` map in via a chart map; the validated region is the unit hypercube `[0,1]⁸`. **No latent coordinate is spent on a gauge direction** (this is why the old z₂, z₃ Cartesian-Jacobi directions were dropped in the 10D→8D cleanup).

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
| **3. Cross-block invariant** | `E`, `L_z`, `K`, `KE`, `PE` | *solves* into a downstream sector, upstream sectors read as fixed | must sit **downstream** of every block it depends on; brings **feasibility boundaries** (infeasible pixels tagged, not dropped); two coupled invariants → joint solve |
| **4. Coupled curve** | one scalar `ν` slaving *several* blocks via a fixed nonlinear embed (Burrau/Euclid: `ν → (α,β,μ₁,μ₂)`) | the embed pins config + mass together | tilt is only well-posed **at a point** (Part 4); decouple the coupling → decomposes back to a plain block axis |

Burrau introduces no fifth kind. Kind 4 *degrading* into kind 1/2 when you drop the mass coupling is the signal the taxonomy is at the right level.

**Residual conventions (kind 2)** are a first-class chart property. "The `m₀` axis" is ambiguous until you declare the fate of the other mass DOF: hold `m₁ = m₂`, hold the `m₁:m₂` ratio, or pin `z₇`. Different conventions give genuinely different one-parameter families — all valid, all must be *labelled*.

**Mixed-axis charts:** the two axes need not share a block. Any pair. When one axis is a configuration coordinate and the other its conjugate momentum, the render **is literally a Poincaré section** — it lifts the phase-space degeneracy (same shape, different momentum, different fate) that a config-only chart collapses.

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

### Tilt (basis edit)

Rotating a basis vector *within the 8D*, smoothly interpolating a plane direction toward a chosen hidden direction. For a raw/affine axis:

```
q'(τ) = normalise( cos τ · q + sin τ · d_target )        τ ∈ [−90°, +90°]
```

`τ = 0` → no tilt; `τ = ±90°` → fully committed to the target. **Sign matters** — the manifold is not symmetric under latent sign flips, so `+30°` and `−30°` reveal different cross-sections. The 8D has **6 directions orthogonal** to the current plane; the tilt selector offers those (plus named compound directions, below). Choosing which hidden direction to rotate into is the core hypothesis-testing gesture ("does this boundary persist as I rotate `q₂` toward `L_z`?").

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

This rule retro-explains the curve-axis result: a curve axis has no single basis vector — `γ(ν)` bends — so tilting it means rotating its **tangent at the centre**:

```
q_tilt(τ) = normalise( cos τ · γ'(ν₀) + sin τ · d_target )
```

Free-floating curve-axis tilt is ill-defined only because the tangent changes along the curve. Pin the centre and the tangent is a fixed vector. This is the general pattern — *non-raw directions are tangents; tangents need a point* — of which "curve axes tilt only at a lock" is the special case.

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

The kernel receives `(z₀, q₁, q₂, chart id + params)` — identical in free and locked mode, before and after any tilt, slice, or excursion. Pan, slice, zoom, tilt, lock, snap-back are **all CPU-side edits to the same uniform**. The lock (chart construction, R-69), the `δ` memory, the ghost marks, the direction library — none of it reaches the kernel. If an implementation finds itself adding a "locked" flag or a second code path to the kernel, it has misread this part.

---

## Part 5 — Well-posedness and the validation contract

A chart is **well-posed iff its swept axes + conventions + slice pin all 8 DOF**: no block over- or under-determined. The usual case is 2 swept + 6 frozen. The exception is **annotation axes** (below): a chart may sweep only 1 physical DOF and render it across a 2D canvas, giving 1 swept + 7 frozen, provided the other axis is declared annotation-only.

**Per-axis metadata the validator needs:**

- **block-touch set** — which of {config, momentum, mass} the axis writes into. May be *empty* for an annotation axis.
- **annotation-only?** — if set, the axis drives *no* IC degree of freedom; it exists purely as a display/landmark coordinate (tick marks, lattice highlighting). The decoder ignores it. The Euclid plane (`principia_chart_reference.md` §4.5) is the canonical case: `Φ_Euclid` sweeps `ν` on one axis; the `m` axis is annotation (integer-lattice / primitive-triple landmarks only, "the decoder uses only ν"). A chart with an annotation axis is a **1-swept family rendered in 2D**, well-posed as 1 + 7, and must be *labelled as such* — it supports per-value statistics along the swept axis only, never 2D density claims across the annotation axis.

- **invariant?** — if so, which sector it solves into, its dependency set (must be downstream), **and whether it is `conserved_along_flow`**: `E` and `L_z` are constants of motion (a hover trace pins to a labelled dot); `K` is invariant-*constructed* but not conserved (a trace oscillates as KE↔PE exchanges). Consumers: the hover trace and any along-trajectory rendering.
- **residual convention** — for derived-in-block axes, how the leftover within-block DOF is pinned
- **curve?** — if so, tilt requires a lock; carries an `embed` map and its tangent `γ'`

**Per-chart descriptors:**

- `system_image` — how the address map (what you turn) folds onto distinct systems (what the physics feels). The chart is *always* a genuine 2D thing you explore by two knobs; this only records redundancy. Four values, replacing the old one-off `has_redundant_hemisphere`:
  - **bijective** — every pixel is a distinct system (most charts).
  - **n-to-1** — a fixed finite number of pixels share each system. Carries the fold so downstream draws/labels one representative.
  - **`DoubleCover`** — covers each shape twice, as two labelled systems (R-27, R-104): the shape sphere, 2-to-1 over the φ hemispheres. Carries the fold so downstream draws/labels one representative.
  - **ray-degenerate** — whole lines of pixels map to the same system (the *continuous* `(m,n)` Euclid plane: rays through the origin are similarity classes, so the picture bands along rays). Legitimate and often *pedagogically the point* — it makes the similarity symmetry visible — but the quantitative layer must not read areas as system fractions, and the UI should expect banding.

  The int `(m,n)` lattice is **bijective**: coprimality (`gcd=1`) is the lowest-terms rule, one address per ray, redundancy quotiented out — which is exactly why the discrete survey and the continuous plane are different instruments over the same 1D curve of shapes.

- `forbids_energy_normalisation` — set on invariant charts where `E` is itself a coordinate (`(L_z,E)`, `(L_z,K)`); the validation pass refuses any config combining such a chart with a non-zero `E*` override.
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
