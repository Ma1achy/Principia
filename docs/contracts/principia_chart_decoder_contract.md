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

**The shape sphere chart is the whole sphere (R-544):** pixel `(θ, φ)` (θ azimuthal, φ polar from `+w`; R-14, chart reference §3.3) decodes to the system whose **signed** `n̂` is that point. The two hemispheres are the triangle's two orientations, the equator its collinear shapes, and every pixel is its own system: the mirror `(θ, φ) ↦ (θ, π − φ)` is a symmetry of fates that is tested, never an identification, and nothing is folded or hidden. *Was (R-141, removed for the shape sphere by R-544):* "**Shape-sphere redundancy:** the canonical decode gauges the `λ̃_y → −λ̃_y` reflection, i.e. `(θ,φ) ∼ (θ, π−φ)` … The φ hemispheres are reflection-equivalent — the chart is a **2-to-1 cover**. Render one hemisphere or flag the redundancy."

---

## Part 2 — The decoder

*[mirror-step removal (R-865, R-898, R-919): rewritten 14 Nov 2026 by TASK-M2-107 to the decoder TASK-M2-57 builds, with R-898's periodic `β` and R-919's one `β` link; in force when TASK-M2-57 merges, having passed R-898's conversion test; until then each *Was* below describes the built decoder on `main`, and R-82, R-438 and R-544 stand. In the target, `C` has no mirror step, latent `β` is a periodic angle, `β = z_β` read modulo 2π, a lower-half state encodes to `β ∈ (π, 2π)`, and every chart, the shape sphere included, decodes by one path. 17 Nov 2026 (R-1020, RQ-824 (b)): the periodic link is the offset `β = z_β + π/2` (`principia_dd_decoder.md` § 3.2), so `z = 0` stays the golden IC; the identity just stated is its Was. This section is rewritten to that target, each changed passage keeping its *Was*; the definitions are `principia_dd_decoder.md` § 3.2 (REQ-DEC-056). Was (this marker): "mirror-step removal, ruled 8 Nov 2026 (R-865, on RQ-739): the decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the owner's bitwise check (every preset, saved chart and golden with `β ∈ [0, π]` reproducing unchanged), which meets the link `β = π·σ(z_β)`: RQ-750, on which the task waits. This section changes when TASK-M2-57 lands, and stands as written until then. Was: "[mirror-step proposal, 7 Nov 2026 (R-854; RQ-739): the owner's proposal to remove the decoder's mirror step, latent `β` over the full circle, would touch this section. A proposal, not applied; nothing here changes until it is ruled.]""]*

**Latent controls** `z ∈ ℝ × S¹ × ℝ⁶`, `z_β` the circle (R-898). These are *controls*, not physical quantities — each is pushed through a smooth decoder (sigmoid / softmax / warp, and for `z_β` the identity onto the angle) to reach a physical value. Chart coordinates `(s,t) ∈ [0,1]²` map in via a chart map; the validated region is the unit hypercube `[0,1]⁸` on the seven bounded controls, `z_β` having no bound: it is stored unreduced, on the lift, and read modulo `2π` (`principia_dd_decoder.md` § 3.2). *Was (the built decoder until TASK-M2-57 merges; rewritten 14 Nov 2026, TASK-M2-107):* "**Latent controls** `z ∈ ℝ⁸`." "each is pushed through a smooth decoder (sigmoid / softmax / warp) to reach a physical value." "the validated region is the unit hypercube `[0,1]⁸`." (Past `[0,1]²`, where the figure fills the window, an extended pixel's `z` may leave `[0,1]⁸`: the hypercube bound is the encode path's check, inverse_encode layer 1 on lookup and lock, and an extended pixel is checked by layers 2 and 3 only, Part 3, R-407.) **No latent coordinate is spent on a gauge direction** (this is why the old z₂, z₃ Cartesian-Jacobi directions were dropped in the 10D→8D cleanup).

*[RQ-750 (a) + (c), ruled 8 Nov 2026 (R-898): `z_β` becomes a periodic angle over the full circle, so the latent space is `ℝ × S¹ × ℝ⁶`, `z_β` on the circle, not `ℝ⁸`, and every stored latent view, saved chart and golden is converted once, under a conversion test in place of R-865's bitwise check. Rewritten 14 Nov 2026 by TASK-M2-107, above, to the target TASK-M2-57 builds, in force when it merges; until then the Was describes `main`; *was:* "This part stands as written until TASK-M2-57 merges, which rewrites it."]*

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

**Canonicaliser** `C`: translate to CoM, apply the gauge rotation; it has no mirror step (R-865). Every chart converges here, the shape sphere's decode included, by the one path: its state keeps the pixel's signed `n̂` because `C` keeps `λ̃_y`'s sign (R-544's signed sphere, without its skip). *Was:* "**Canonicaliser** `C`: translate to CoM, apply the gauge rotation/mirror. Every chart converges here. The shape sphere's decode runs the same `C` with its mirror step not taken, so its state keeps the pixel's signed `n̂` (R-544); on every other chart `C` is unchanged."

**The one hard output rule:** the integrator's input is `(mᵢ, rᵢ, pᵢ)` and *nothing else, ever*. The compute kernel is chart-agnostic — it reads `(m,r,p)` and never learns which chart produced them. Chart proliferation cannot reach the integrator.

*15 Nov 2026 (TASK-M2-87; REQ-INT-115; R-926 item 5).* The rule stands for the physical input: the integrator integrates `(mᵢ, rᵢ, pᵢ)`, and holds no chart code. It reads them from records the CPU wrote, each formed in f64 and narrowed once, and the record carries, beside them, the descriptor, tag and branch words, the stop index `k_stop` and the route (R-935, R-985, R-989, R-995): the record and table contract, `principia_lowering_contract.md` Part 3 ("The record and table contract (CPU to GPU)"). "Nothing else, ever" reads as no chart code and no decode in the integrator, those ruled words travelling in the record.

---

## Part 2.5 — Link functions & compactification (customisable)

Squashing functions appear in **three distinct roles**. They look alike (all map something unbounded into something constrained) but act at different stages, live in different keys, and must not be conflated.

| Role | Maps | Purpose | Change → re-integrate? |
|---|---|---|---|
| **Link function** | control `ℝ →` constrained physical param | enforce a block's **constraint geometry** | **yes** — changes the IC |
| **Axis warp** | chart coord `[0,1] →` position along axis | shape **sampling density** | **yes** — changes which IC each pixel is |
| **Colour compactification** | diagnostic scalar `→ [0,1]` | fit an unbounded diagnostic into colourmap range | **no** — colour-only, second pass |

The last one is what "decouples palette/compactification changes from integration cost" means: recolour the payload (`SimState` buffer) without re-running the integrator. Links and axis warps are on the **integration** side; colour compactification is on the **colour** side. This is the integrate/colour seam again.

**The links are the decoder's (R-478).** Links belong to the decoder, which every chart passes through
(`principia_chart_reference.md` §0, "every chart ends here"), not to any one chart: changing a link changes what every
chart's pixels mean, and switching chart never switches links, since this Part's robustness test is "same chart,
different link". They are chosen and tuned in the GUI's Decoder window (`principia_render_gui_spec.md` §G7a); the axis
warps are per-axis chart properties, owned by the Chart builder (§G7). A chart preset carries no links (R-478, W4),
unless it pins them: a preset or a saved chart may optionally pin the links it was designed under, and loading it
applies them, with a notice, in one undo entry, never silently (R-516; `principia_render_gui_spec.md` §G7). *Was:*
"whether one may pin them is RQ-380" (stale value, replaced by R-516).

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
2. **Invertible, with a conditioned inverse.** Lock/lookup needs the inverse (`logit`, `artanh`, `log`). It blows up at the boundary → the ε clamps (`ε_z, ε_μ, ε_q = 10⁻⁶`) and the saturation constants `μ_max = 5`, `q_max = 2` (settled, R-10; since R-481 the defaults of two tunable parameters, below). A link without a stable inverse cannot support the encode path.
3. **Smooth (C¹).** The deep-zoom **linearised decoder** replaces the nonlinear decode with a local Jacobian; a non-differentiable link breaks that path.

**Tunable parameters (R-481 ⚑).** Each registry entry declares its tunable parameters, each with a name, a default, an
allowed range and a citation: today `μ_max` on `softmax_tanh` and `q_max` on the momentum links, with defaults 5 and 2
(R-10), `μ_max`'s range capped at 10 (R-597: at 10 the far field's `M₀₁` floor, `(1 + e^−10)/(1 + e^−10 + e^10) ≈ 4.5×10⁻⁵`,
stays above the `M01_TINY` fence's `10⁻⁵`, so no far-field pixel at the default link is tagged); `α_min` stays fixed (R-21; R-518, which rules that it does not become a parameter; *was:* "(R-21; RQ-382)",
a stale value, replaced by R-518). The allowed range keeps **all three** requirements above for **every**
value in it, and the registry's check tests them at the range's ends and on a sampled interior
(`principia_dd_generation_root.md` §3.9). Everything that reads a parameter by name reads the active value: the link
functions, the codomain's bounds, the encode's clamps and its layer-1 range check (`principia_inverse_encode_contract.md`
§ "Chart-aware validation"). The generated code reads the values as a uniform, so the kernel's results at the defaults
are bitwise unchanged (R-481, G7).

### Links carry a measure

A link is **not measure-neutral**. Softmax with saturated logits is smooth but *not uniform* over the simplex; uniform-in-angle ≠ uniform-over-shapes (S² has its own area element); a power-law axis warp `K(t)=K_max·t^{γ_K}` deliberately biases density toward low `K`. Fine for exploration. But for any **quantitative** claim (basin fractions, island prevalence) you must either correct by the link's Jacobian or sample in a known measure. So **each registry entry should carry its log-det Jacobian**, so a quantitative pass can reweight. The log-det is a function of the entry's parameters, and area statistics reweight with the active value, never the default (R-481, A4). This is the one place a customisable link can silently corrupt a result — the freedom is real, but it moves the sampling measure, and the measure has to travel with the link.

**A statistic says what it counts** *(15 Nov 2026, TASK-M2-59; REQ-CHART-094; ported from
`principia_chart_composition.md` § 6.4)*. Refinement density is not probability density: the quadtree "is a rendering
/ compute-allocation structure … **not** physical probability density, and it must never feed a quantitative claim"
(`principia_scheduler_contract.md` Part 2), so a statistic starts from a flat grid, never from a count of leaves. A
composed or warped chart adds a second bias: its pixels aren't equal in any physical measure. So **any statistic over a
composed or warped chart** (outcome fractions, "forbidden in view", replication scores) is one of two things, and there
is no third:

- **Weighted.** Each counted pixel of the flat grid carries the chart's area factor, the Gram determinant
  `√det(JᵀJ)`, `J = ∂Y/∂(u, v)`, into a reference measure the statistic names. `J` is taken on the pixel's own branch,
  never across a seam. A `ray-degenerate` chart has no such factor, and R-554 rules there: the legend shows each share
  as undefined ("—"), never 0%. No label replaces that.
- **Labelled `per pixel of this chart`.** The count is what it says, and claims nothing physical.

**The label is the default**, since `Y`'s metric is a definition still owed (REQ-CHART-124): a statistic is weighted only
once its measure is defined and named. `measure: chart area` stays the chart's label, and `per pixel of this chart` is
the same fact on the statistic. R-408's rule of what counts is unchanged: the weight falls on the pixels it counts, and
pixels that leave the count leave the total too. The default label lifts no area weight a view already carries: under
Lean each axis the Gram weight stands (R-408, R-554). **Replication fractions use each paper's own measure**, not the
area factor: a flat grid in the paper's own coordinates on its domain, or on a warped chart the Jacobian into those
coordinates; the target's row names that measure, one not yet read from the paper is a gap, and a target stays a
measurement, never a gate (R-649). No area statistic is taken on a mid-morph frame (R-936, R-944); the two end charts
keep their own.

### Integrity: the link is part of the experiment

The reason to expose the link is not flexibility — it's that a fixed, hidden link **launders an arbitrary choice into apparent fact**. "The Burrau point is a local maximum of ejection prevalence in the mass simplex" is really "…under softmax with `μ_max = 5`"; if the link is invisible, so is the qualifier — to a reader, and to you six months later. Three requirements make the choice honest:

1. **Swappable within the compatible set.** Any constraint-preserving link for that block (the rows above). Already stated.
2. **Recorded in provenance.** The link id per block lives in the exported ViewState, beside `z₀`, the basis, and the axis warps. A figure whose provenance doesn't pin its links is *not reproducible* — a re-run under a different default samples a different measure and can return a different answer. The link is part of the experiment, so it lives in the record.
3. **Falsifiable by re-measurement.** The real test: does a finding *survive* a link swap? Extremal under softmax **and** under a boundary-reaching simplex map **and** under a different logit saturation → a property of the *dynamics*. Moves when the link changes → a property of the *parametrisation*. This is the **tilt persistence test of Part 4 applied to the measure instead of the plane**: a structure is real iff it's invariant under the arbitrary choices you could have made differently.

**The parameters are part of the record (R-481, R-483).** The parameter values are recorded beside the link id
wherever it is recorded: `SimConfig`'s `Links` holds, per block (masses, α, β, momenta), the link id and every tunable
parameter's value, written in full even at the defaults, so the view state, the sim key and the payload compatibility
signature, every export and every sidecar carry them through the one canonical serialisation
(`principia_gui_state_contract.md` §2). A figure that names its link but not its `μ_max` is not reproducible. A record's
decoder is the reading build's registry plus the record's links: a link id the build lacks, or one whose definition
changed, is refused ("state present, cannot recreate", naming the link), never replaced by a default
(`principia_dd_image_embedding.md` §6; R-483, X4). Each recorded link carries its own registry-entry hash, so the
refusal names exactly the changed link (R-521; *was:* "R-483, X4; RQ-385", a stale value, replaced by R-521).

**What makes an alternative worth offering.** Not any function with the right codomain — one whose Jacobian differs *enough to stress a claim*. Two links inducing near-identical measures agree trivially and prove nothing. The registry should span genuinely different sampling densities (softmax vs. a simplex map that reaches the edges; `t^γ` at spread-apart γ; sigmoid vs. a heavier-tailed bounded map) so that agreement across them *means* something. To make that deliberate, a registry entry carries — beyond forward, inverse, and log-det — a note on **which region of the block it over- and under-samples relative to uniform**, so a robustness sweep picks alternatives that disagree where it matters rather than three flavours of the same bias.

**No new entry until a study needs one (R-519).** The registry's entries stand as they are; an alternative that
disagrees enough to stress a claim (a temperature-softmax, a bounded map heavier-tailed than softsign, a simplex map
uniform over the simplex) is added when a study needs one, by the task that runs that study, as a definition (R-72)
under physics review, through `principia_dd_generation_root.md` §3.9's table and its check.

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
| **3. Cross-block invariant** | `E`, `L_z`, `K`, `KE`, `PE` | *solves* into a downstream sector, upstream sectors read as fixed | must sit **downstream** of every block it depends on; brings **feasibility boundaries** (infeasible pixels tagged, not dropped); two coupled invariants → joint solve; **warped by default** (`principia_chart_reference.md` §2.1, evaluated per pixel), every pixel of `[0,1]²` feasible, a linear mapping an option per axis whose infeasible pixels take their edge's declared kind (R-471, as R-932 and R-937 amend E2: § "Past the unit square" below) |
| **4. Coupled curve** | one scalar `ν` slaving *several* blocks via a fixed nonlinear embed (Burrau/Euclid: `ν → (α,β,μ₁,μ₂)`) | the embed pins config + mass together | tilt is only well-posed **at a point** (Part 4); decouple the coupling → decomposes back to a plain block axis |

*Was (kind 3's last cell, before R-932 and R-937 amended R-471's E2; TASK-M2-95):* "a linear mapping an option per axis
whose infeasible pixels are not a system (R-471)".

*7 Nov 2026 (R-816, ruling RQ-682 (a), and R-838, RQ-717):* the **virial ratio** `Q = 2K/|U|` is kind 3: it solves
into the momenta and reads the configuration and the masses as fixed upstream. R-471 H3's "not kind 3" is read as about
B1's extension only, which R-506 has superseded. The set stays closed at four (R-834, RQ-712 (a)).

Burrau introduces no fifth kind. Kind 4 *degrading* into kind 1/2 when you drop the mass coupling is the signal the taxonomy is at the right level.

**Residual conventions (kind 2)** are a first-class chart property. "The `m₀` axis" is ambiguous until you declare the fate of the other mass DOF: hold `m₁ = m₂`, hold the `m₁:m₂` ratio, or pin `z₇`. Different conventions give genuinely different one-parameter families — all valid, all must be *labelled*.

**Mixed-axis charts:** the two axes need not share a block. Any pair. When one axis is a configuration coordinate and the other its conjugate momentum, the render **is literally a Poincaré section** — it lifts the phase-space degeneracy (same shape, different momentum, different fate) that a config-only chart collapses.

### The primitive registry — every axis an entry under one of the four kinds (R-834)

*15 Nov 2026 (TASK-M2-59; REQ-CHART-094): ported from `principia_chart_composition.md` § 2 and § 3, as R-834 (RQ-712
(a)), R-835 (RQ-713 (a)), R-846, R-847, R-849, R-925, R-926, R-935 and R-1011 rule it; the formulas are
`principia_chart_reference.md` § 6.* The four kinds stay a closed set (REQ-CHART-004): a primitive is a **registry
entry** under one of them, with its metadata, never a fifth kind, and R-798's "no new chart type" is the rule for
anything not already ruled (R-834). `SuvakovPlane` stands as R-669 ruled it. The side ratios are a registered closed
pair, an option in the Chart builder and not a chart type, the same closed form serving as PA-1's remap (R-835); R-798's
"There is no `SideRatio` chart" stands. The virial ratio is kind 3 (R-816, R-838, above). The Anosova row of the
lowering appendix, "physical × physical", is kind 2 × kind 2 on the configuration block, residual none, as the shape
sphere's row is: the row is conformed, not a new kind (`principia_lowering_contract.md`, Appendix).

**Two layers.** An axis written alone is independent: a primitive or a combination of primitives, a warp, an extension
and what it holds fixed. A chart couples its two axes only by what it declares, a pair map and its constraints (hold
`E`, hold `L_z`). In the CPU authoring layer:

```rust
struct AxisSpec   { primitive: Combination, warp: Warp, extension: Extension, holds: Residual }
struct ChartSpec  { axes: [AxisSpec; 2], pair_map: PairMap, constraints: Constraints, slice: Slice }
```

`ChartSpec` is data, authored on the CPU, and no kernel sees it (REQ-CHART-004); `validate` turns it into a
`ValidatedChart`, as today, and `resolve` lowers it (`principia_lowering_contract.md`). Each place two axes can't be
independent is listed in `principia_chart_reference.md` § 6.5. Every new field defaults to what is built today, and a
default field is not serialised, so existing presets and saved charts reopen bitwise: the canonical form
(`principia_gui_state_contract.md` §2; R-484) and the sim key do not move.

**The registry.** One table, `chart_authoring::registry`, CPU-side. Every row declares:

- **forward**: state to value, always. The Inspector, the contours and every test read it.
- **inverse class**: `closed`, `numerical` or `none` (an overlay only). A closed inverse that needs a partner is
  registered on the pair. Under R-925 and R-926 (items 5 to 8) every chart is evaluated on the CPU in f64 and the
  integrator holds no chart code, so the class never chooses an integrator route: on the fragment side a chart with
  closed-form shader code takes its generated WGSL and one with none the table route, never a refusal; a numerical
  inverse runs on the CPU, its branch rule and seam declared (`principia_chart_reference.md` § 6.7).
- **slope tier** (R-926 item 9; R-935 under RQ-758): exact (dual numbers), implicit or none. A map with no exact slopes
  is still a valid chart.
- **kind**: one of the four.
- **extension**: the row's natural type. Under R-932 and R-942 (RQ-775 (a)) the row declares, per edge, the kinds
  allowed (saturate, unfold, wrap, pole-crossing, clamp) and a default, never a blank area by default, the mechanism
  per edge the physics reviewer's call; the view state holds an override per edge. The per-edge defaults are RQ-779's
  (deferred to M2 by R-963; R-941: the mass simplex saturates). An **angle-like** row is flagged: a periodic or
  pole-crossing declaration, or affine on an angle-like axis, needs physics review (R-407 clause 3), since an
  angle-like quantity declared affine would redraw systems and count them again.
- **scale**: `free` (a shape or a ratio); `gauge` (fixed by `I = 1`, so a paper's value needs the recorded rescale,
  `principia_chart_reference.md` § 6.6.5); or `size-only`, refused as an axis, since `I = 1` at every decode
  (`principia_dd_decoder.md` §5, identity 1) makes it constant.
- **class** (`primitive_class`): a state function; a conserved quantity, exact or approximate, an approximate one
  labelled `approximate`; or a trajectory functional, which is never an axis and has no row
  (`principia_chart_space_model.md` § 6b; R-852). A state function read at a fixed `T` may be an axis of a
  final-condition chart; an event read, such as minimum `I`, stays a functional at any `T` (R-879).

**Combinations.** A combination is an expression over primitives, in the stain's expression syntax, the subset
`principia_lines.md` §4.1 needs (R-509). Its forward is the expression. Its inverse class is not read from its parts:
`exp(K) + K` has closed parts and no closed inverse, and `K + L_z` doesn't say which part moves. It is `closed` only
with a registered closed inverse, and `numerical` only with a registered numerical one, naming what it solves for and
its branch rule. A typed combination with no registered inverse, a registered `none` row typed as an expression
included, takes R-849's general numerical solve on the CPU (REQ-CHART-145): distinguishable from a registered numerical
inverse by stating the coordinates it solves for, its branch rule, its seams and its convergence per pixel, tagging
every failure and never dropping it (R-1011, with physics's points (i) to (iii)). The `SizeOnly`, `NoWarpWritten`,
`AxisAndHold` and `SameQuantity` refusals still apply to a typed combination, and `NoJointInverse` is raised only for a
registered `none` row picked as an axis (Part 5). A combination is angle-like if any part is, and its scale is computed,
not declared. No expression is ever interpreted on the GPU (lowering contract Part 1).

**Size-only is a computed property too.** The `size-only` column is declared, and a test checks the declaration: it
samples decodes and fails any row whose forward is constant on them and is not declared `size-only`.

**Mass-weighted against geometric.** The shape sphere is mass-weighted; the interior angles and the side ratios are
geometric and invert onto the sphere with the masses in the formula (`principia_chart_reference.md` § 6.6.3). So an
axis states which it means in its `holds`: `(γ₀, m₂)`, holding a geometric triangle while the mass moves, is a
different family from `(θ, m₂)`, holding a point on the sphere. Both are legal, and each is labelled.

**What a row may name (R-847, R-846, R-1011).** R-669 is reversed: fixed `k` and binary-plus-single scattering are
charts and builder axes like any other, built when their turn comes (TASK-M2-78, TASK-M2-79), their missing
definitions filled at build under physics review (R-72), and inner and outer orbital elements are axes at a labelled
pair, plus overlays. So a row with an inverse may name a binary phase, an eccentricity, an impact parameter or a
fixed-`k` prescription; the guard that refused such rows is retired (R-1011). A plane a user builds is the user's chart
and is never scored (R-846).

**Rejected as axes.** Total momentum (zero by the centre of mass, `C`'s boost); absolute direction (the rotation gauge,
`C`'s rotation); anything size-only (`I = 1`; `SizeOnly`); `z` components (the problem is planar); discrete labels
(colourings, with no inverse: `none`, an overlay); and trajectory functionals and event reads (results, coloured
outputs, R-879).


### Past the unit square — each axis's extension type (R-407)

*10 Nov 2026 (R-932, the owner's rule on chart edges, with R-937 to R-939, R-941 and R-942); rewritten 15 Nov 2026 by
TASK-M2-95 (REQ-CHART-194):* **no blank areas.** Every edge of a chart's primary range declares an extension kind: past
a true edge the chart saturates, where there is a second sheet it unfolds onto it, and where the coordinate repeats it
wraps; which mechanism an edge uses is the physics reviewer's call, saturating wherever the edge system itself is not a
valid system, such as a zero mass. This subsection is rewritten from those answers, each replaced line kept as *Was*;
each shipped chart's per-edge table is `principia_chart_reference.md` §5.4's, and the definitions of the second sheet
and the clamps are its §2.2 (iv) to (vi). The chart addendum's N13 and N14 build it; until they land the built charts
draw as the *Was* lines describe (R-932). The choice lives in the registry row, with a per-edge override in the view
state (R-942); the centre may cross onto an unfold sheet and stops at a clamped or saturated area (R-942); an optional
Overlays › Chart toggle, off by default, marks clamped and saturated pixels (R-939). Under the edge-reaching simplex link
a pixel with `M₀₁ < 10⁻⁵` (R-597) saturates, the physics reviewer's call under this rule, in place of drawing as not a
system; how it saturates is RQ-827, and until that is ruled it draws as R-597 has it. *17 Nov 2026 (R-1024, RQ-827
(a)):* it is clamped to the fence, not saturated: `M₀₁` held at `ε_M01`, the inner ratio `m₀ : m₁` kept, `m₂ = 1 − ε_M01`, the configuration and momenta decoded at those masses, a system the decoder accepts (the clamped `M₀₁`, formed as the decoder forms it, at or above `ε_M01`), not counted, and on the sim key, its system not one the square's samples compute (R-1025). A pixel whose target energy cannot
be reached with η_E on (R-598) is not reached by the rule: R-598 stands, such a pixel a validity failure, not an edge,
drawn as not a system with the reason in the readout, "no blank areas" being about edges and their extensions (R-955,
the owner's answer on RQ-784, 11 Nov 2026). *Was:* "The four types below gain unfold, clamp and saturate as declared
kinds per edge (clause 3's physics review for each); blank ("bounded") is no edge's default, and is drawn only for a
validity failure or an edge switched to blank. The count: an unfold sheet is inside the primary range and counted, a
clamped pixel is not, a saturated pixel only where its system is not shown inside the square."; "Under the edge-reaching
simplex link a pixel with `M₀₁ < 10⁻⁵` (R-597) saturates, the physics reviewer's call under this rule, in place of
drawing as not a system;"; "(…; *was:* "is RQ-784, R-598 standing meanwhile"). The section below is the text in force
for what is built today; the chart addendum's N12 rewrites it from these rulings, each replaced line kept as *Was*, and
N14 builds it (`plan/charts/m2-chart-addendum-extensions-axes.md`)."

The figure can show more than the chart's `[0,1]²`: wherever the view reaches past `[0,1]²`, by pan, by zoom out, or
by F3 Extend (R-537; F3 and present mode fill the window at the same scale, R-406), its samples take `(s,t)` beyond
`[0,1]` on either axis. **The extension is a property of the chart, not of F3** (R-537): it is drawn wherever the view
reaches, the controls shown or hidden, and it never has gaps. **Inside `[0,1]²` nothing changes**, but where a chart's
true edge lies inside its square (the linear option's parabola, R-471 E2; the exponential map's `r = π`, R-503), whose
kind acts there too. *Was (R-407):* "with the egui layer hidden (F3) or in present mode it fills the window at the same
scale (R-406), so where the window reaches past the depth-0 root its samples take". **Each edge of the chart declares
one kind** (R-932; *was:* "Outside it, **each chart axis declares one of four extension types**, and the type decides
what the axis does past its edge:"), and the kind decides what the chart shows past that edge:

| Kind | Past the edge | Primary range (R-408, R-932) | Place in the classification order | Which edges (chart_reference §5.4) |
|---|---|---|---|---|
| **affine** | the formula continues, up to a true end where the edge declares one, whose kind then acts | unbounded, or to the true end | step 0 passes the coordinate through; the pixel is then classified by steps 3 to 6 | basis vectors: the latent and flat slices, every latent direction; the invariant warp past `t = 1` (R-508); the linear option's axes (R-471 E2); `ν`, `θ` and δm to their zero-mass ends; the exponential map's square to `r = π`; a time axis past `T` |
| **wrap** (R-407's **periodic**) | the coordinate repeats: it wraps by its period | one period (the shape sphere's θ, the chart's `[0,1]` span) | step 0 folds it; step 2 takes a pixel outside one period out of the count and the total, a repeat | the shape sphere's θ; latent `β` once R-898's periodic `β` is in force |
| **pole-crossing** | continues over the pole and back down the other side, the partner axis shifted by half its period | pole to pole (the shape sphere's φ, the chart's `[0,1]` span, the whole sphere, R-544) | step 0 folds it and shifts the partner; step 2, a repeat | the shape sphere's φ (θ shifted by π) |
| **unfold** | onto the construction's second sheet, the coordinate reflected about the edge and the sheet flipped (chart_reference §2.2 (iv)); a sheet word joins the pixel's record and the decode's inputs | one period of the two sheets, centred on the square (`s ∈ [−½, 3/2)` on the warp): its sheet-2 pixels are counted | step 0 folds it and sets the sheet; `validate` reads the sheet (R-942); past the period, step 2, a repeat, on a tilted view only where `N_⊥ = 0`, the unfolded axis's component of `N` (`N_a` where it is `a`, `N_b` where it is `b`: `Φ_θ,L_z`'s `L_z`, a lone `L_z` axis in that position), decided exactly on the stored angles, and otherwise its own system, counted (physics review 5913253688 of PR #546, F2; *was:* "only where `N_a = 0`") (chart_reference §5.4, "The unfold's period on a tilted view") | the invariant charts' spin edges, warped and linear, `Φ_θ,L_z`'s and a lone `L_z` axis's (R-937) |
| **clamp** | the edge system: the nearest point of the primary range in the edge's declared metric, REQ-ENC-030's (`\|ΔL_z\|` at fixed `E`, chart_reference §2.2 (vi)) where it projects, `\|ΔE\|` at fixed `L_z` below the floor where REQ-ENC-030 rejects (§2.2 (v), R-937) | ends at the edge; a clamped pixel lies outside it, an edge system shown again, not counted, as a repeat is not | step 0 projects it; step 2 takes it out of the count and the total; whether its quads are re-addressed, off the sim key, or its own is RQ-828 (*17 Nov 2026, R-1025:* re-addressed only where the clamp repeats the primary range's systems, every other clamp on the sim key) | the invariant `t < 0`; the floor of a held `L_z`; below the floor at a non-zero `L_z` (R-937); the Burrau `K` below 0; the Euclid plane's `m`; a mass ratio past the end the link realises; a time axis before 0 (R-938) |
| **saturate** | the coordinate squashed toward a true end it never reaches, monotone and inside its declared band (chart_reference §5.4, "Saturate on a zero-mass end") | the band, counted where its systems are not shown inside the square, the edge declaring which | step 0 squashes it; step 2 takes it out only where the edge declares its systems shown inside the square | the mass simplex's sides (R-941); the zero-mass ends of `ν`, `θ` and δm; the `M₀₁` fence under the edge-reaching simplex link (RQ-827; *17 Nov 2026:* clamped to the fence by R-1024, so a clamp, not a saturate, on the sim key by R-1025) |
| **blank** (R-407's **bounded**, one name) | not a system: blank, the view's background, in every view but the debug views, which hatch it; elsewhere the hatch only as the optional Overlays › Chart › forbidden region (R-467, R-470) | its domain | step 1, the domain's end, out of the count and the total | no edge's default (R-932); every edge may be switched to it. An axis is not offered until each of its edges declares a kind under clause 3, so no edge is blank for want of a declaration (physics review 5910557865 of PR #546, F8; *was:* "an axis that declares nothing until a declaration under clause 3") |
| **reflection** (a new kind under clause 3) | the coordinate reflected about the edge onto the side the chart shows, where the systems past it are twins of that side's, the partner and hidden coordinates mapped onto the twin's as the reflection requires (on `Φ_θ,L_z` `(θ, v) ↦ (π/2 − θ, 1 − v)`, the spin reversed); where a tilt or slice leaves a hidden coordinate that the map does not carry, the edge is affine instead | the side the chart shows | step 0 reflects it; step 2, a repeat | the acute angle `θ` past `π/4` where its systems are twins (R-27; chart_reference §5.4, "The reflection past `π/4`") |
| **repeat map** (a new kind under clause 3) | a declared map onto the point that is the same system; on a tilted view only where the map leaves the tilt's hidden offset `d` unchanged (§ "Repeats and the canonical point"), the exponential map's iff `N_a = 0` and `N_b = 0`, decided exactly on the stored angles, and otherwise the edge is affine (chart_reference §5.4, "The repeat past `r = π` on a tilted view") | the domain the map lands in | step 0 maps it; step 2, a repeat | the exponential map past `r = π`, `(r, ê) ↦ (2π − r, −ê)` |

*Was (the table of four types, before R-932):* "| **affine** | the formula continues | basis vectors: the latent and
flat slices, every latent direction; a physical-quantity axis mapped linearly onto its range (R-408's port, B1), for an
energy-type axis its linear option (R-471, E2; warped, its default, it is bounded) |"; "| **periodic** | the axis wraps
| azimuthal angles: the shape sphere's θ |"; "| **pole-crossing** | continues over the pole and back down the other
side, the partner axis shifted by half its period | polar angles: the shape sphere's φ (its partner θ shifted by π) |";
"| **bounded** | not a system: blank, the view's background, in every view but the debug views, which hatch it;
elsewhere the hatch only as the optional Overlays › Chart › forbidden region (R-467, R-470) | anything with a hard edge
in its domain (the invariant warp's `t^γ_K` at `t < 0`); **the default for an axis that declares nothing** |".

A custom chart inherits its edges' kinds from its axes' rows (R-472, as R-942 amends it: a view may hold an override per
edge, saved with it); nothing else is written per chart. The kinds are a property of each edge beside its axis's kind
(the four kinds above), not a fifth axis kind. Each registry row declares, per edge, the kinds allowed and the default
(R-942, RQ-775 (a)); a kind whose pixels show systems computed elsewhere (wrap, pole-crossing, reflection, repeat map,
clamp) re-addresses to the canonical quads and is off the sim key, as repeats are (R-537 EX7), and a kind that shows new
systems (unfold, saturate, affine) changes the quads' systems and is on it. *15 Nov 2026 (code review 5910534528 of PR
#546, F1):* whether a clamp re-addresses is **RQ-828**: R-937's C5 and chart_reference §2.2 (v) and (vi) make most
clamped pixels systems no quad of the primary range computes (a per-pixel configuration, a spin past the square's
range, an exact edge point); meanwhile R-942's words stand here as written, the clamp definitions stand, and the clamp's
place on or off the sim key waits for the ruling. *17 Nov 2026 (R-1025, RQ-828 (c)):* split: a clamp whose every
pixel shows a system the primary range's samples already compute (the Euclid plane's `m`; a time axis before 0)
re-addresses to those quads, off the sim key; every other clamp (C5's per-pixel tilt, a spin past the square's range,
an exact edge point, the `M₀₁` fence of R-1024) is on the sim key, each clamped pixel its own system, and switching it
to blank drops the clamped area's quads from the request set. Each registry row declares per edge which its clamp is
(TASK-M2-105, under physics review). The kinds table's clamp row reads accordingly, its "whether its quads are
re-addressed, off the sim key, or its own is RQ-828" answered here. **The kinds act first** (step 0): an affine
edge passes its coordinate through, a wrap or pole-crossing one folds it (shifting the partner), a reflection or repeat
map maps it, an unfold folds it and sets the sheet, a clamp projects it in its metric, a saturate squashes it into its
band, and a blank one past its edge makes the pixel not a system. `Φ` and the validity check then run on what the kinds
give. *Was:* "A custom chart inherits its extension from its axes; nothing is written per chart. The types are a
property of each axis beside its kind (the four kinds above), not a fifth kind. The axes' types act first: an affine
axis passes its coordinate through, a periodic one wraps it, a pole-crossing one reflects it over the pole and shifts
its partner, and a bounded one past its edge hatches the pixel. `Φ` and the validity check then run on what the types
give."

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
its infeasible pixels take their edge's declared kind (chart_reference §5.4's table; R-932, R-937) and warn nothing.
*Was:* "under an axis's linear option its infeasible pixels are not a system, blank, and warn nothing." *Was:* "Whether a Chart-builder chart, whose
physical-quantity axis has a range, may fail there is RQ-372." *Was (R-446):* "it is drawn with the render contract's hatch (`debug_invalid`, Part 5). The outcome
view keeps the hatch too, on every pixel this part hatches …".

**Blank, never a colour (R-470).** A pixel that is not a system, past an edge whose kind is blank (switched to it, or an
undeclared axis's), rejected by `validate`'s layer 2 or 3, or where `Φ` fails, draws **blank** (*was:* "past a bounded
edge"; under R-932 blank is never an edge's default, so a blank pixel is a validity failure or an edge switched to
blank), the view's background colour, which is R-467's canvas background. It is not
a palette colour in any view, the outcome view included; the debug views hatch it. The hatch survives outside them only
as the optional overlay Overlays › Chart › forbidden region, off by default, drawn over those pixels only
(`principia_render_gui_spec.md` §G2). NaN, `±∞`, `sim_failed`, `decode_failed` and the classification behind the hatch
show only in the debug views, R-467's View › Debug view among them; a presentation view draws a system's `sim_failed` or
non-finite sample `#141418` as R-467 has it, and a `decode_failed` sample the chart does not classify (a non-finite
decode, η_E's tags, the mass block's `M01_TINY`, the last under the edge-reaching simplex link being RQ-827's, which R-1024 clamps to the fence and so draws as a system) blank,
as a pixel that is not a system (R-511). *Was:* "a system's
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
and for custom physical-quantity axes: past their edges they show their edges' declared kinds (R-932), and only a
validity failure or an edge switched to blank draws the canvas background, the debug view the hatch (R-467). The shape
sphere's exponential map's square covers only a disc of radius `s√2` about `n₀`; past it the formula continues to
`r = π` and past `r = π` repeats (`principia_chart_reference.md` §5.4's table; R-938), so the sphere's systems there are
drawn. *Was:* "past their edge, with F3 in its Extend mode, a presentation view shows the canvas background and the
debug view the hatch (R-467). The shape sphere's exponential map is one of the bounded charts: its axes are bounded
(`principia_chart_reference.md` §5.4's table) and its square covers only a disc of radius `s√2` about `n₀`, so past its
square a pixel is classified not a system and draws the background, though the sphere holds systems there (physics
review 5585225466 of PR #253, P6)." *Was (R-468 as first ported):* "and the shape sphere,
whose periodic and pole-crossing types redraw its own systems". In Fit mode the figure is the view's square alone, the rest of the window the
canvas background (`principia_render_gui_spec.md` §G1).

**No invented continuations.** An edge gets anything beyond blank only by declaring it, and any new kind (a
reflection, a repeat map) and every declaration needs physics review (R-407, clause 3; R-932). Each shipped chart's
edges are in `principia_chart_reference.md` §5.4's per-edge table. *Was:* "An axis gets anything beyond bounded only by
declaring it, and any new extension type needs physics review. The existing charts' types are in
`principia_chart_reference.md` §5.4."

**Area statistics past `[0,1]²` count each system once, through the kinds (R-408, R-932).** Each kind has a **primary
range**: periodic, one period of that axis (for the shape sphere's `θ = 2π·s`, the chart's `[0,1]` span); pole-crossing,
pole to pole (for the shape sphere's φ, the chart's `[0,1]` span, the whole sphere, R-544; *was:* "with both hemispheres drawn"); affine, unbounded;
bounded, its domain; and the new kinds' as the table above gives them: an unfold sheet is inside it and counted, a
clamped pixel outside it and not counted, a saturated pixel counted only where its system is not shown inside the
square, a reflected or repeat-mapped pixel outside it (R-932; *was:* "through the types (R-408)"). A new periodic or pole-crossing declaration states its own primary range, under clause 3's physics
review (R-407). "Forbidden in view" (render_gui_spec §G7) and the area
statistics (W7, `system_image` below) count a visible pixel **only if every axis is inside its primary range**, so a
periodic or pole-crossing redraw is not counted again: a pixel past a pole carries its partner shifted by half a period
and is outside the pole-crossing axis's primary range, so it is excluded. **Pixels hatched because the domain ends
there** (a bounded axis past its edge, or a `Φ` that fails to evaluate) are not systems: they leave both the count and
the total. **Only states that fail the validity check count as forbidden.** A pixel is classified in this order: (0) its edges'
kinds act, as above; (1) past an edge whose kind is blank, the domain's end; (2) outside a primary range, not counted
(a repeat: wrap, pole-crossing, reflection, repeat map, an unfold past its period; a clamped pixel; a saturated pixel
whose edge declares its systems shown inside the square), out of the count and the total; (3) rejected by `validate`'s
feasibility (layer 2, on what the kinds give, the sheet included, before `Φ`), forbidden: past an edge, layer 2 reads
the constraints that make the state a system (each mass positive, `Σmᵢ = 1`, the energy charts' parabola and floor),
and a bound that is the chart's own range, which the edge's declared kind extends past, is read as the primary range's
in step 2, never as a validity failure here: the mass simplex's interior buffer `ε_m`, below which a saturated mass
lies (chart_reference §5.4, "Saturate on a zero-mass end"; R-932, R-941); the acute angle's `θ ≤ π/4`, past which an
affine pixel lies (the δm strip with an asymmetric target, a tilt the reflection does not carry); and the Burrau
charts' `ν ∈ (0,1)`, past which the δm strip's mirrored triangles lie (`ν > 1` past `θ = 0`, `ν < 0` past `θ = π/2`,
their blended masses positive; on a chart with Burrau's own masses a mass is already non-positive there) (physics
review 5911915550 of PR #546); on lookup and lock each binds as written: `ε_m` and `ν ∈ (0,1)` in
`principia_inverse_encode_contract.md` § "Chart-aware validation"'s layer 2 (REQ-ENC-013), `θ ∈ (0, π/4]` in
`validate`'s failed-constraint list (chart_reference §5.1, REQ-CHART-043) and, on encode, the fold of
`principia_dd_encode.md` §3.1 (REQ-ENC-022), as REQ-ENC-030's class rule binds lookups and the lock, never a drawn
extension (R-937) (Codex review of PR #546; qa review 5911919550, F3; *was:* "(… the energy charts' parabola and floor,
`ν ∈ (0,1)`)", "and the acute angle's `θ ≤ π/4`", "on lookup and lock layer 2 binds as written
(`principia_inverse_encode_contract.md` § "Chart-aware validation")"); (4) a `Φ` that fails where
layer 2 accepts, the domain's end; (5) a decoded state that fails layer 3, forbidden; (6) any other, a counted system:
an affine continuation, an unfold sheet inside its period, a saturated pixel whose system the square does not show.
*Was:* "A pixel is classified in this order: past a bounded edge, the domain's end; outside a primary range, not
counted; rejected by `validate`'s feasibility (layer 2, on what the types give, before `Φ`), forbidden; a `Φ` that fails
where layer 2 accepts, the domain's end; a decoded state that fails layer 3, forbidden; any other, a counted system." The statistic reads the chart (the types, the
primary ranges, `validate`, `Φ`), not the payload's `decode_failed` detail, which does not tell the two hatches apart.
**Inside `[0,1]²` every statistic is unchanged.** The shape sphere has no one-hemisphere chart (R-544), and a
one-hemisphere view is only the symmetry filter's, a display option after rendering that removes nothing (R-855; R-878,
RQ-738 conflict 1's reading held; *was:* "The shape sphere has no one-hemisphere view (R-544):"): its statistics are the whole sphere's, each pixel its own system, and where the mirror symmetry holds (REQ-CHART-082) each fate of a mirror-invariant class is counted twice, equally, so those fractions are equal on a fixture whose pairs stay close over the horizon, and elsewhere differ only by the pairs whose fate a rounding-level difference in the decode flips (a signed class maps to its mirror class; physics reviews 5603309678, P2, and 5603529679, P7, of PR #273) (REQ-CHART-083, REQ-CHART-057; TASK-M8-44). *Was (R-412, withdrawn by R-544):* "With one hemisphere of the shape sphere drawn, φ's span is remapped to run from the pole to the equator; past the equator its formula continues into the mirror hemisphere (real systems, repeated), and its primary range is pole to equator, so that continuation is not counted (R-412, closing RQ-264; built by TASK-M8-44, REQ-CHART-057)."

**Repeats and the canonical point (R-537).** A pixel in a repeat (past a periodic axis's period, over a pole; under
R-932 also past a reflection, a repeat map or an unfold's period) is the same system as its **canonical** point, the one inside every axis's primary range, which the
types give by folding it: its pick, readout, trajectory, outcome and inspector are the canonical system's, the readout
naming the pixel a repeat. A pixel on an affine extension is that point of the plane. The quadtree follows the same
fold (EX7): a quad in a repeat maps to its canonical quad by the type group (on the shape sphere at level `L`,
`n = 2^L`, R-14's `φ = π(1 − t)`: θ's period `ix' = ix mod n`; the pole maps at both edges, over the pole at `t = 1`
`iy' = 2n − 1 − iy` and `ix' = (ix + n/2) mod n`, over the pole at `t = 0` `iy' = −1 − iy` and the same shift; there
is no fold at the equator, the mirror being no repeat (R-544); composed until the address is inside the root), which is the one
requested, computed, cached and decided, every image drawing it; neighbours (2:1 balance, the criterion's arms,
contours, class edges, smoothing) cross seams, poles and folds through the same map, an affine neighbour is the real
one, and past a blank edge there is none (*was:* "past a bounded edge"). A clamped quad re-addresses to its edge's
quads, as a repeat does (R-942; whether it may is RQ-828, above; *17 Nov 2026 (R-1025):* only where the clamp repeats
the primary range's systems, every other clamped quad its own address), and an unfold sheet's quads are its own addresses, a neighbour crossing the fold as it crosses
a pole (R-942). A quad wholly past a blank edge is not requested (*was:* "past a bounded edge"); one straddling a
validity boundary integrates its valid footprints only, the rest `decode_failed` and blank. The statistics above are
unchanged: each system is counted once, at its canonical address (R-408). At `L = 0` a repeat across a pole is drawn
from its two level-1 children, as EX7(b) states; the bootstrap is not raised for pole-crossing charts (R-552, RQ-410
(a)). **On a tilted view** a repeat is the same
system only where the type group's map leaves the tilt's hidden offset `d` unchanged: along an axis that mixes θ with
a non-periodic coordinate the chart is affine and draws no image (R-538 TF2.2, J8; R-552, RQ-430 (a); *was:* "RQ-430 (a) applied meanwhile");
the pole maps apply on a tilted view iff `N_a = 0` and `N_b = 0` for chart_reference §3.3's `N`, decided exactly on the
stored angles (Tip as sheet `m == 180`; Lean each axis iff `sin τ₁ = sin τ₂ = 0`, its one-target corner ray-degenerate
with no fold; two targets iff `τ₁, τ₂ ∈ {0, ±180}`; R-663, closing RQ-630; *was:* "Lean each axis only untilted, its
one-target corner ray-degenerate with no fold"), and otherwise the view is affine along that axis and draws no image
(R-554, ruling RQ-472 (a)); there is no equator fold on any view (R-544): R-544 wins and there is no fold anywhere,
the human's RQ-472 ruling's equator clause, which kept one where `N_b = 0`, and its pixel fold for the statistics not
applied (R-613, closing RQ-508, as applied meanwhile; *was:* "is RQ-508 (R-544 applied meanwhile; R-554 leaves that
clause unapplied)").
**The lattice of repeats on a tilted view (R-655, closing RQ-631).** The spherical map repeats every 1 in `s` and
every 2 in `t`, so a pixel `(s + k, t + 2j)` with `N_a·k + 2·N_b·j = 0` for integers `(k, j) ≠ 0` keeps `d` and is the
same system as `(s, t)`: it is counted once, at its canonical point (R-408). The lattice is decided exactly on the
stored angles, never by a tolerance on a computed `N`: `N_a = N_b = 0`, the pole maps and both periods, as above;
`N_a = 0` alone, θ's period, the canonical `s` in `[0, 1]`; otherwise the multiples of one generator `(k₀, j₀)` with
`j₀ ≠ 0`, where one exists: `N_b = 0` alone, `t`'s period 2, `(k₀, j₀) = (0, 1)`; Tip as sheet with both angles set,
`N_a/N_b = τ₁/τ₂`, a ratio of two stored doubles and so always rational, its generator read exactly from their binary
expansions; Lean each axis with one target, `N_a/N_b = tan τ₁/tan τ₂`, which for angles rational in degrees is rational
only where `τ₁ ≡ ±τ₂` (mod 180°), the ratio `±1`, or where one angle is `≡ ±30°` and the other `≡ ±60°` (mod 180°),
the ratio `±1/3` or `±3` (Mann's theorem on vanishing sums of roots of unity); two targets, only `t`'s period, where
`sin τ₂ = 0` and `sin τ₁ ≠ 0`. With such a generator, `w = (k₀, 2j₀)` in `(s, t)`, the canonical point is the one whose
offset from `(½, ½)` along `w` lies in `[−|w|/2, |w|/2)`, a strip that holds the square (applied per R-369, R-655).
Where there is none the view is affine along that axis, as above; at RQ-471's corner no fold is applied.
*Was (R-544 removes the hemisphere's fold from the repeats):* "(past a periodic axis's period, over a pole, past the
hemisphere's fold)"; "with both hemispheres drawn, R-14's `φ = π(1 − t)`, the pole maps at both edges, …; with one
hemisphere drawn, R-412's `φ = (π/2)(1 − t)`, the pole map at `t = 1` and R-412's fold at the equator, `t = 0`,
`iy' = −1 − iy` with `ix` unchanged, past `t = −1` the fold then the `t = 1` pole map (R-537 U10)"; "whether the pole
maps and the equator fold apply on a tilted view, against chart_reference §3.3's tilted fold, is RQ-472". *Was (R-554
rules RQ-472 (a)):* "whether the pole maps apply on a tilted view is RQ-472, and they are not built on a tilted view
until it is ruled"; "as §3.3 decides its fold".
*Was:* "with R-14's `φ = π(1 − t)`: θ's period `ix' = ix mod n`; over the pole at `t = 1`, `iy' = 2n − 1 − iy` and
`ix' = (ix + n/2) mod n`; over the pole at `t = 0`, `iy' = −1 − iy` and the same shift; R-412's fold at the equator,
`t = 0`, `iy' = −1 − iy` with `ix` unchanged; composed" (physics review 5594430380 of PR #263, P3: two maps on one
edge; applied per R-369).
**The named exception on a tilted view (R-665, closing RQ-634).** A pole map's image `(s + ½ + k, −t + 2j)` has the same
`n̂` as `(s, t)`, and where the pole maps do not apply it also keeps `d` on the lines `N_a(½ + k) + N_b(2j − 2t) = 0`:
under Lean `45°/45°`, `(a, ¼)` and `(a + ½, −¼)`; with `N_a = 0`, `N_b ≠ 0`, the pole rows `t = j` at each `d`. These
lines are a named exception of measure zero to R-408, as chart_reference §3.3 states the circle's identification: such
a pixel and its image are one system and each is counted. No fold or test on the computed `t` decides them.

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

**Pan and zoom past the square (R-537).** Pan and zoom may take the view's window past `[0,1]²` on every chart, with
the controls shown or hidden. The only stop is R-469's, on the centre (below): on a periodic or pole-crossing axis
`validate` is evaluated on the centre after the types have folded it, so a pan across a seam or a pole never stops; on
an affine latent axis there is no stop at all; on a chart whose square is its whole domain the centre stops at the
domain's edge, while the window may still show blank past it. "Filling the window is not a gesture" stands for F3, whose
Extend only adds window area.

**Latent and flat charts: no limit on pan or zoom-out (R-570).** On a chart whose axes are affine and whose decode
accepts any `z` (the latent chart and the flat and body-plane slices, tilted or rotated) navigation has no limit. **Pan
is unbounded:** any centre is allowed, because every latent `z` decodes to a valid system, so the stop below never
fires for a pan there. **Zoom-out is unbounded:** no cap from a fixed view range, a multiple of the home view or a
number of chart squares; the only limits are the frame budget and the baseline cover (`principia_caching_contract.md`
Part 4), and a frame's cost is set by the screen, not by the view's extent: where the viewport spans many level-0
roots the cover is taken at a level `L < 0`, coarser than the root (`principia_scheduler_contract.md` Part 3,
"Infinite zoom-out: levels coarser than the root"; RQ-535, decided per R-369). **Every pixel is a system** at any pan or zoom: no hatch, no blank, no
"not a system" (at the decoder's default links with `μ_max` inside REQ-GEN-038's range, which is capped at 10, η_E off (`None`); R-597 and R-598, closing RQ-536 and RQ-552: under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` is genuinely degenerate, and with η_E on a pixel whose target energy cannot be reached has no system at that energy; each draws as not a system, with the reason in the readout; *was:* "(at the decoder's default links, η_E off (`None`); RQ-536, RQ-552)"). **Far out the field goes flat, and that is correct:** the
decoder saturates every coordinate (`principia_chart_reference.md` §0: the configuration through a sigmoid, the momenta
to `±q_max`, the masses through `μ_max·tanh`), so far from the origin neighbouring pixels decode to nearly the same
system; nothing clamps, rescales, wraps or fades the view to hide it, and the readout states the decoded system as for
any pixel. **Precision is not a navigation limit:** at large chart coordinates the f32 floor arrives sooner, and the
precision ladder handles it (`principia_deep_zoom.md` §2 and §4), never a stop on the pan or the zoom. Unchanged: a
chart with a bounded axis, or an affine axis that runs off the physical domain (the mass simplex, Burrau `L_z`, the
Invariant warp), keeps its not-a-system region past the square; periodic and pole-crossing charts keep RQ-409 (a) (R-552), no
zoom-out limit beyond the frame budget and the baseline cover; and the trace's "leaves / re-enters the chart" on a
latent chart is a trajectory state projected back onto the axes, not a navigation limit.

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

**Two tilt maps (R-474, ruling RQ-340).** With one target (`X = Y`) and both angles set the GUI offers two maps, recorded as the view's `tilt_map`. **Tip as sheet**, the default (`sheet`): with `m = √(τ₁² + τ₂²)` the plane `span(e_a, e_b)` is turned rigidly by `m` about the in-plane axis `k = (τ₂ e_a − τ₁ e_b)/m` (Rodrigues' formula, in the 3-space `(e_a, e_b, e_X)`), so the plane tips toward `e_X` (its normal tipping away from `e_X`, toward `−(τ₁ e_a + τ₂ e_b)/m`; R-474 K16) and a single `τ₁` moves `q₁` toward `e_X` (*was:* "so its normal tips toward `e_X`"); the basis stays orthonormal, with no lock at 90°. **Lean each axis**, an option (`lean`), is the formula above per axis, `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_X`, skewed (`u₁·u₂ = sin τ₁ sin τ₂`). The two differ only with `X = Y` and both angles non-zero; a single tilt, or `X ≠ Y`, gives bitwise the same basis. **Over the pole:** Tip as sheet is not clamped at `m = 90°`; the angles are stored as set and the rotation is computed from them, wrapped internally only in total tip at 360° (for `m > 180°`, `τ·(m − 360)/m`, the same rotation), never folded onto the 180° partner `τ·(m − 180)/m`, which is the same plane mirrored. The range `τ ∈ [−90°, +90°]` above is Lean each axis's; under Tip as sheet each angle runs over `[−180°, 180°]`. A comparison of planes (a readout, a cache or dedupe keyed on the plane) compares normals up to sign and is never written back; the sim key keeps the plane and its in-plane orientation (R-92). Under Lean each axis an area statistic weights each pixel by the Gram factor `√(|u₁|²|u₂|² − (u₁·u₂)²)` (R-408); Tip as sheet needs none. At `|τ₁| = |τ₂| = 90°` with one target, both axes wholly in `e_X`, the Gram factor is 0 at every pixel: the chart reports `ray-degenerate`, no fold is applied (there is no normal), every area weight is 0, and the legend shows each share as undefined ("—"), never 0% (R-554, ruling RQ-471 (a)).

**Tilt on every chart (R-473).** The dev GUI tilts every chart, curved ones included. The tilt is a rotation in the 3-space of the two screen axes and the hidden target coordinate, `R(τ₁, τ₂)` from the active map, applied to the chart's **own parameters**, not to a tangent vector: the pixel at window offset `(u, v)` maps to `(a, b, d) = R·(u, v, 0)`; `a`, `b` are the chart's own axis parameters and go through its chart map exactly, so its axes keep their meaning; `d` is added to the hidden coordinate's value at the slice centre. With two targets (`X ≠ Y`) each axis turns toward its own, in the 4-space `(a, b, d_X, d_Y)`. Untilted, `R` is the identity and the chart is bitwise today's; on the latent chart T2 is exactly the basis edit above. A chart's hidden coordinates are the latent controls its axes do not write (Part 5's block-touch sets), and every hidden coordinate is tilted in its link's pre-image, a mass in its logit, never in the physical value, so every pixel stays a system (masses positive, summing to 1). One target kind is the exception: on the shape sphere the physical `p_λ` components (`p_λ,x`, `p_λ,y`, in the sphere's own frame, `ρ` along `+x`, unmirrored) are tilt targets too (R-554, ruling RQ-474), as the trace demo's tab F builds them, the tilt adding `d` to the physical component at the slice centre, not to its latent control, so it never saturates; their definition (how the decode holds the other momenta and the masses, the target's range, what the readout names) is written here by TASK-M8-58 (REQ-CHART-084, R-72) and checked by physics, and until it is the target is not offered. **Charts that hold a block by a construction (R-633, ruling RQ-603; one principle added to R-473 J1).** Where a chart holds a block by latent controls, its hidden coordinates are those controls; where it holds a block by a physical construction, its hidden coordinates are that construction's physical parameters, tilted and sliced by value as the shape sphere's `p_λ` targets are, `d` added to the physical value at the slice centre. So: the mass simplex's configuration hidden coordinate is `ν₀`, the held Burrau-family shape, and `d = 0` reproduces the untilted chart exactly; a rest start (the mass simplex, the Euclid plane, the δm strip) is `z_mom = 0`, and each of the four momentum controls is a hidden coordinate from 0, the momenta `D_mom` of it under the pixel's masses (every registered momentum link maps 0 to 0); the `(ν, K)` chart and `Φ_θ,K` gain the held `L_z`, and `Φ_θ,L_z` the held `K`, as physical hidden coordinates, the momenta §2.2's construction (`principia_chart_reference.md`) at the tilted value. **Those three charts follow R-471's two modes (R-708, ruling RQ-678).** In warped mode, the default, every pixel inside the chart's square is a system under every tilt: the `K` axis takes the pixel's own held `L_z` as its floor, `K = L_z²/2I + K_max·t^γ_K`, past `K_max` at the top if the floor takes it there; a held `K` is tilted in its warp coordinate, not by value, so it stays ≥ 0; and rounding at the floor doesn't blank the bottom row. The held `L_z` moves by value in both modes, tilted and sliced alike, `L_z = L_z₀ + d`, with no sign constraint: R-708's warp coordinate is for a held `K` only, the floor keeps every pixel a system at any `L_z`, and a column above `K_max` at large tilt is accepted, the practical bound the slider span, RQ-679 (R-720, ruling RQ-699). *Was:* "How the held `L_z` itself moves in warped mode is RQ-699." A slice of a held `K` stays by value in both modes, `K₀ + d`: R-708 amends the tilt only, REQ-CHART-060's stop holds the centre at `K₀ ≥ 0`, and `t₀` is recomputed from the new `K₀` (applied per R-369; R-709's clamp does not apply to a slice). If the implementer confirms that a held energy's warp coordinate reaches `t < 0`, it clamps at `t = 0`, held `K` at its floor, each clamped pixel a real system flagged "held at floor" in the Inspector and the sidecar, with no mirror (R-709). In linear mode, R-471 E2's option, rows keep fixed physical values, a held coordinate tilts by value, and a cell with no momentum construction at the tilted value (`K < L_z²/2I`) takes its edge's declared kind (R-932, R-937; `principia_chart_reference.md` §4.5's linear-mode bullet and §5.4's table): on a `K` axis at a held `L_z` the clamp below the floor at the pixel's own `L_z` (§2.2 (v)), on `Φ_θ,L_z`'s `L_z` the unfold past `±√(2IK)` and the clamp below the floor at a held `K ≤ 0`; a clamped cell is a system, integrated, out of the count (R-932). *Was (R-708, amended by R-932 and R-937; Codex review of PR #546):* "reads as refused, through the tilt's existing ray-degenerate path: it is no system, its area weight 0 (R-554), blank per R-470, drawn as validation layer 2's `K ≥ L_z²/2I` draws such a cell, not a system (`principia_chart_reference.md` §5.4)." *Was:* "A cell with no momentum construction at the tilted value (`K < L_z²/2I`) reads as refused, through the tilt's existing ray-degenerate path:", in every mode (replaced by R-708). With these, T1's "every chart" holds as written, and every chart of the registry tilts and slices. Before R-633 the rule above, the latent controls the axes do not write, was the only one, so the mass simplex's configuration had no value at the slice centre and the `(ν, K)` chart, `Φ_θ,K` and `Φ_θ,L_z` offered none. On a curved chart the rotation compares a chart parameter and a hidden coordinate in units that do not depend on zoom (R-507, ruling RQ-375): each screen axis in units of the axis's full range at zoom 1 (the parameter span of the chart preset's window), the hidden coordinate in units of its own slider span; on the latent chart the tilt stays the basis edit above, in the latent coordinates. *Was:* "How a chart parameter and a hidden coordinate share a scale is RQ-375 (the human's proposal applied meanwhile on the curved charts: each screen axis in units of the window's half-width, the hidden coordinate in units of its own slider span)." **The slider spans (R-920, ruling RQ-679).** A hidden coordinate's slider span is the full range at zoom 1 of the screen axis that carries the same coordinate on the registry's own charts, read at that chart's home window, never at the view's centre: fixed per chart, the same under pan, zoom, slice centre and lock, stored with a saved chart (R-507 U3). A latent slider's span is 2, rail `[−1, 1]` (`principia_render_gui_spec.md` §G2). The held `ν₀`'s is `Δν = ν_max − ν_min` over `[1/32, 31/32]` (REQ-CHART-023), `15/16`, its rail `[ν_min, ν_max]`. The held `L_z`'s is `2√(2·I·K_max)`, `I = 1` after the scale gauge, `K_max` the chart's own `K` axis's (REQ-CHART-044), read at the window's top: rail `[−√(2K_max), +√(2K_max)]`, the practical bound R-720 names. The held `K`'s, tilted in its warp coordinate, is 1 in `t`, rail fixed per preset at `t ∈ [t₀ − ½, t₀ + ½]`, `t₀` the preset's (shifted to `[0, 1]` if `t₀ < ½`), `K_max` and `γ_K` the chart's own held-`K` warp (REQ-CHART-141); on a linear axis it is `K_max` in `K`. Under a lock the anchor runs over `±` half the span. The held `K`'s one slider has its handle in `t`, drawn evenly in `t`, and its number box in `K`; a slice writes `K₀` by value and recomputes `t₀`, moving the handle, not the rail; a tilt reads the rail's length. A `MixedEnergy` chart's held `L_z` on a lone virial axis has no chart-constant `K_max`; its span is RQ-701's, written by TASK-M8-48 at M8 (R-920). The definitions are `principia_chart_reference.md` §4.5's (REQ-CHART-188). Under Tip as sheet with one target and both angles set the rotation can take a window pixel inside `[−1, 1]²` to `(a, b)` outside the chart's square, where the axes' types act (Part 3): such a pixel is past the chart's edge, and blank past a bounded one (R-512, ruling RQ-402; past the energy warp's continuing `t > 1` side it is that side's system, R-508). *Was:* "where the axes' types act (Part 3); whether that is meant is RQ-402." A tilted chart extends past `[0,1]²` by its axes' types as an untilted one does (Part 3), and an energy-type axis's warp is re-evaluated per pixel under it (R-471). Slicing moves the slice centre along any hidden coordinate, subject to the stop above.

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
  second order, `δv` is §2.2 (ii)'s unit direction `w`, the seed in force, the expansion `w = r/√I` (R-934; *was:* "its
  seed decided on `z_α` (R-82)", now the named legacy option's): the direction the `(L_z, K)` construction adds energy
  along, so on that construction's image `d_E` is its `K` axis's tangent. `δpᵢ = mᵢ·δvᵢ` in
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

**Setting the lock.** Select the pixel at $(s,t)$ and pin its IC there, **without moving the view** (R-866, the owner's
reply of 8 Nov 2026: "locking must not move the view"): no edit of `z₀`, the basis or a nonlinear chart's window. The pin
is the clicked pixel; its initial in-plane excursion is that pixel's offset from the view's centre, and tilt and zoom
turn about it, so the pinned pixel stays fixed on screen. For an affine chart,
$\mathbf z_{\mathrm{locked}} = \mathbf z_0 + (2s-1)\mathbf q_1 + (2t-1)\mathbf q_2$: CPU arithmetic, with no GPU readback.
For a nonlinear chart, replay $\Phi$ and $D$ on the CPU, or read the GPU buffer back. **Centre on pin** is an optional
action, a navigation edit that brings the pin to the centre, and a setting, off by default, applies it at every lock in
the same command (render_gui_spec §G4). Where this part reads the anchor as the centre pixel (the anchor-preserving
class below, "keeps the anchor at centre", and the nonlinear chart's re-centred window), the anchor is at the pin;
TASK-M2-62 builds it in the engine (R-916; *was* TASK-M8-125), exactly on nonlinear charts or stating why not, an inexact pin's bound a calibration the owner rules on before merge (R-866, R-71; REQ-CHART-166, REQ-GUI-429; *was* "REQ-GUI-425, REQ-GUI-429", REQ-GUI-425 being the GUI's badge, TASK-M8-125's). Until it lands the code
re-centres as below. *Was (before R-866):* "Select the pixel at $(s,t)$ and snap the centre to its IC."

The centre pixel has one special property: `z(½,½) = z₀` **regardless of the basis**. That property sorts the operations into two classes:

- **Anchor-preserving (basis edits): tilt, zoom, chart-mode switch.** The locked IC is the *fixed point* of the operation — the centre pixel's IC never changes; surrounding pixels are `z_locked + offset·q'(τ)`, same in-plane offsets, rotating directions. You watch the neighbourhood deform around an IC that is exactly constant. This is the persistence test, mechanically. Chart-mode switching keeps the anchor at centre while changing which degrees of freedom the surroundings explore ("this IC's neighbourhood under energy variation vs. under geometry variation").
- **Anchor-excursion (centre edits): slice, and slider moves in locked mode.** There is no way to change a hidden coordinate while keeping the point — slicing *necessarily* moves the centre IC. Locked slicing is therefore an **excursion along a line through the anchor**: `z_centre = z_anchor + δ`, with `δ` accumulating slice steps. The centre pixel is always "the anchor's IC except along the excursion direction(s)." The excursion has **memory and a way home**: `δ` is tracked explicitly, the anchor value is ghost-marked on each control, and a snap-back gesture returns `δ → 0` exactly (not approximately — the anchor is stored, not re-derived).

**Sliders are re-based, not frozen.** An earlier design froze the sliders in locked mode. Here, in locked mode, sliders are **re-based to the anchor, not frozen**. Each slider shows `z_anchor + δ` live, with the anchor value marked; moving one is an anchor-excursion along that control's direction. Frozen sliders give only the microscope; re-based sliders additionally give slice-through-the-lock — strictly more capability, same GPU interface. Unlock preserves the current `(z₀, q₁, q₂)` as the new free-mode state.

**Nonlinear charts:** the anchor is preserved by re-centring `Φ` on `z_locked` via the inverse-encode path (two options: CPU replicate-and-evaluate, or per-pixel readback). Feasibility applies: an excursion or tilt can carry the neighbourhood outside the feasible region of an invariant chart — those pixels are tagged, never dropped, exactly as for any chart.

**The lock's construction, defined** (R-72; REQ-CHART-089; TASK-M2-23):

- **The excursion is held by the stored pair.** `z₀` holds the centre and `z_locked` the anchor, and `δ = z₀ − z_locked`:
  a slice step or a slider move edits `z₀` alone, so `δ` accumulates it; snap-back undoes the slice only, clearing the
  hidden part of `δ` and leaving the pin where it is on screen, and "centre on pin" (R-866) clears its in-plane part, the
  two together `z₀ ← z_locked`, bitwise by definition: centre on pin after snap-back assigns `z₀ ← z_locked`. On an
  oblique basis the split of `δ` into in-plane and hidden parts is a projection, so snap-back alone is exact to rounding:
  its rounding bound is derived from the construction TASK-M2-62 defines (of order `ε_R·κ(B)·‖δ‖` for a solve on the
  basis `B = [q₁ q₂ | hidden]`, or a few ulps if built as `z_locked` plus the in-plane part recomputed from the pin's
  stored chart parameters) and stated by that task, its decoded state within REQ-ENC-024's `ε_phys` only where that bound
  and the decoder's conditioning allow; a bound that can't be derived is a calibration (R-71); the test's fixture names
  its `κ(B)` and `‖δ‖`; its construction is a definition TASK-M2-62
  writes here under physics review (R-72) (the chart-path brief's Q2 (a), R-899; the auto-snap setting off by
  default); and `δ` has no field of its own that could disagree with `z₀`. *Was:* "snap-back is `z₀ ← z_locked`, bit
  for bit;". Unlock clears the flag and the anchor and leaves
  `(z₀, q₁, q₂)` as it stands. A basis edit (a tilt, a zoom, a chart-mode switch) turns about the pin
  (render_gui_spec §G4; R-451, "Locked"; R-466 K5): at `δ = 0` the pin is the centre and the edit keeps `z₀`; where `δ`
  has an in-plane part the pin is off-centre, and the edit's `SetField` on `z₀` moves `z₀`, and with it `δ`.
- **The window on a nonlinear chart.** The view is a window on the chart's own parameters,
  `(u, v) = c + ((2s−1)·h_u, (2t−1)·h_v)`, `c` its centre and `h` its half-widths: Part 3's placement of the addressing
  space, applied to the chart's coordinates, the window pan and zoom move past the square (R-537) and whose offsets a
  tilt reads (R-473).
- **Setting the lock on a nonlinear chart.** `Φ` and `D` are replayed on the CPU at the clicked pixel's `(u, v)`, which
  must pass `validate` (R-469); the state they decode goes through lookup's ladder (inverse_encode_contract §
  "Chart-aware validation"), the inverse-encode path, and a lock the ladder refuses sets nothing. The ladder's `z` gives
  `z_locked` the controls the chart's axes set; every other control of `z_locked` is the clicked pixel's own frozen
  control, bit for bit, as the invariant charts' fibre point keeps them. The chart's hidden coordinates are `z₀`'s
  (`principia_chart_reference.md` §2.1), so the chart rebuilt on `z₀ = z_locked` freezes what it froze before. The
  window is re-centred, `c ← (u, v)`, so the rebuilt chart's centre pixel decodes the clicked state bit for bit. Where
  the ladder clamps none of the controls the chart's axes set, `z_locked` decodes the clicked state within lookup's
  `ε_phys`; where it clamps one, `z_locked` holds the clamp's image and the lock carries `lookup_clamped`, its free
  decode as far from the clicked state as the clamp moved it, and the rebuilt chart's centre pixel still decodes the
  clicked state bit for bit. *(R-866, 8 Nov 2026: locking must not move the view, so this re-centring of the window is
  replaced; how the pin is held on a nonlinear chart without it, exactly or with the reason it can't be, is TASK-M2-62's
  definition (R-72; REQ-CHART-166; *was* "REQ-GUI-425", the GUI's badge, TASK-M8-125's), written here. This bullet stands as the built behaviour until then.)* The edits are the
  `SetField` on `z₀ = z_locked` and the lock group's:
  one command, sent in one frame under one history key, so the history keeps one entry and one undo restores both
  (gui_state_contract §2, "A command of several edits is one history entry"; R-466); a re-centre's edits likewise. The
  window is chart construction, held in `SimConfig` (R-69) among "chart id and params"; `SimConfig`'s chart group has no
  fields yet (R-133), and TASK-M2-21 fills it, the ViewState record that must restore any chart exactly being
  `SimConfig`'s serialisation (REQ-SYS-011; R-483 H2). Until then the lock returns the re-centred window beside its
  edits, and undoing a nonlinear lock restores `z₀` and the lock group but not the window.
- **Lock validation on an invariant chart.** Each pixel of the locked view is read as an entered pair `(L_z, y)`, `y`
  the chart's vertical value, `E` or `K`, at its own configuration and masses: the chart frozen at the pixel's hidden
  coordinates, which a tilt or an excursion moves. A pair that fails the chart's layer 2 there is tagged with the
  constraint it failed and its nearest feasible point (inverse_encode_contract § "The nearest valid point"), or with
  none where that projection would change `L_z`'s class; no pixel is dropped. A pixel whose construction finds no seed
  (R-82) is tagged `DEGENERATE`, its label, never dropped; it is not counted infeasible and is offered no re-centre.
  Where any pixel is tagged infeasible, lock validation gives one notice for the view, never one per tagged pixel
  (R-593). It offers the switch to the latent chart, which has no layer-2 constraint, and a re-centre only where the
  lock's own centre has gone infeasible: the view's centre pixel, `(s, t) = (½, ½)`, whose IC is `z₀`, read as its
  entered pair like any other pixel, tagged infeasible with a nearest feasible point. The re-centre is a lock at that
  point, the one "nearest feasible point" (inverse_encode_contract § "Lock validation"; REQ-CHART-010). Where the centre
  is feasible, as after a tilt alone at `δ = 0`, the centre being the anchor, or is tagged with no nearest point, or
  `DEGENERATE`, only the switch is offered. The other tagged pixels' nearest points stay in their tags and are not
  offered. Where no pixel is tagged infeasible there is no notice. *Was:* "the lock offers the re-centre to each tagged
  pixel's nearest feasible point, a lock there, and the switch to the latent chart … which point that is for a view of
  many tagged pixels is open (RQ-572)."
- **A curve's tangent at the lock.** The tangent tilt reads `γ′(ν₀)` pulled back to the latent controls through the
  selected links, at the embed's controls `z(ν₀)`: `dz_α = dα/f_α′(z_α)` and `dz_β = dβ/f_β′(z_β)`,
  `f′ = exp(log_det)`; the momenta's 0, the curve being a rest start; and the mass controls' the derivative of the mass
  link's inverse along `(dμ₁, dμ₂)`, `(m₀, m₁, m₂) = softmax(0, μ₁, μ₂)`, by a central difference of step
  `h = ε^{1/3}·(1 + ‖μ‖)/‖dμ‖`, `ε` f64's machine epsilon, its error of order `ε^{2/3}`, the registry giving no mass
  link's Jacobian. Without a lock the tangent tilt is refused. It is refused too, naming the controls, where a link's
  inverse clamps (inverse_encode_contract Part 3's clamp test, `lookup_clamped`'s): `α`'s or `β`'s at `z(ν₀)`, or the
  mass link's at `μ` or at either difference point `μ ± h·dμ`. A clamped control's map is flat there, so its pullback
  is not `γ′(ν₀)`'s, as `d_E` is unavailable where an `f_k′` underflows to 0 (§ "Directions are axis kinds").

### What the GPU knows about all of this: nothing

The kernel receives `(z₀, q₁, q₂, chart id + params)` — identical in free and locked mode, before and after any tilt, slice, or excursion. Pan, slice, zoom, tilt, lock, snap-back are **all CPU-side edits to the same uniform**. The lock (chart construction, R-69), the `δ` memory, the ghost marks, the direction library — none of it reaches the kernel. If an implementation finds itself adding a "locked" flag or a second code path to the kernel, it has misread this part. A curved chart's tilt (R-473) is a chart parameter like any other: its rotation `R`, its targets' indices and their scales, computed on the CPU from the stored angles and the map and sent as part of "chart id + params".

### Chart paths: a view is a point, every edit a path (R-850, R-925, R-926, R-936, R-943)

*15 Nov 2026 (TASK-M2-60, closing REQ-CHART-112).* A view is a point in chart space, a map composed with a flat
window of a centre and a basis (R-926 item 1), and every edit in this Part is a path through that space, one path
abstraction for every view edit: tilt, zoom, chart switch, lock, Save view as chart and morph (R-850). A path is window
edits plus the state blend: pan, slice, zoom, rotation and tilt edit the window on one map, a window turn on one map
being a tilt (R-925); only a change of map is a blend, and "morph" names that blend only. A morph is defined by its
pairing (the same pixel on both charts), its path (by default the shortest in each block's natural geometry: mass in
`μ_k = ln(m_k/m₀)`, the logs of the mass ratios, linear; shape on the great circle; momentum in `q` and time, linear)
and its easing, timing only (R-943), exact at both ends (R-926 item 3); the per-axis blend, the pair map and the
per-field path are retired as morphs and offered as maps (R-936); another path is allowed, not built now and not
designed out (R-943). Each operation is tested bitwise against today and against its own path family's formula,
never across families (R-850), a morph's family being the block line, the default path (design § 17.3). The definition
in full, with the Sequence's transition and a morph's frame record, is `principia_export_animation_contract.md` Part 3,
"the chart path".

---

## Part 5 — Well-posedness and the validation contract

*[mirror-step removal (R-865, R-898, R-919): rewritten 14 Nov 2026 by TASK-M2-107 to the decoder TASK-M2-57 builds, with R-898's periodic `β` and R-919's one `β` link; in force when TASK-M2-57 merges, having passed R-898's conversion test; until then each *Was* below describes the built decoder on `main`, and R-82, R-438 and R-544 stand. In the target, `C` has no mirror step, latent `β` is a periodic angle, `β = z_β` read modulo 2π, a lower-half state encodes to `β ∈ (π, 2π)`, and every chart, the shape sphere included, decodes by one path. 17 Nov 2026 (R-1020, RQ-824 (b)): the periodic link is the offset `β = z_β + π/2` (`principia_dd_decoder.md` § 3.2), so `z = 0` stays the golden IC; the identity just stated is its Was. This section is rewritten to that target, each changed passage keeping its *Was*; the definitions are `principia_dd_decoder.md` § 3.2 (REQ-DEC-056). Was (this marker): "mirror-step removal, ruled 8 Nov 2026 (R-865, on RQ-739): the decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the owner's bitwise check (every preset, saved chart and golden with `β ∈ [0, π]` reproducing unchanged), which meets the link `β = π·σ(z_β)`: RQ-750, on which the task waits. This section changes when TASK-M2-57 lands, and stands as written until then. Was: "[mirror-step proposal, 7 Nov 2026 (R-854; RQ-739): the owner's proposal to remove the decoder's mirror step, latent `β` over the full circle, would touch this section. A proposal, not applied; nothing here changes until it is ruled.]""]*

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
  - **n-to-1** — a fixed finite number of pixels share each system. Carries the fold so downstream draws/labels one representative. The shape sphere is not n-to-1 (R-544): it is the whole sphere, each pixel its own system, so its `system_image` is **bijective** on either map, untilted and tilted — the spherical map's θ period and poles are its axis types (R-407), on edges no pixel centre lies on, and the exponential map's cap counts every pixel once, its pixels on `r = π` as REQ-CHART-078 defines untilted, and as REQ-CHART-090 defines on a tilted view, past `r = π` the domain's end (R-503) (`principia_chart_reference.md` §3.3). *Was (before REQ-CHART-090, code review 5719919631 of PR #349, F3):* "its pixels on `r = π` as REQ-CHART-078 defines". *Was (R-141, R-503 and the tilted fold applied per R-369 on PR #255, withdrawn for the shape sphere by R-544):* "The shape sphere is n-to-1 with n = 2: 2-to-1 over the φ hemispheres, which are reflection-equivalent (the canonical decode gauges `λ̃_y → −λ̃_y`, Part 1), so both decode to the same system (R-141). That is the spherical map's value. The exponential map's is computed from its cap …: bijective where the cap lies within one closed hemisphere; where it straddles the equator, n-to-1 with a **per-pixel fold**, at most 2 …"; under a tilt the mirror pair kept its fold iff `N_b = 0`.
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

### Refusals and labels of a composed chart — a closed list (R-834 to R-841)

*15 Nov 2026 (TASK-M2-59; REQ-CHART-094): ported from `principia_chart_composition.md` § 6.1 and § 6.2, as R-836,
R-841, R-843, R-849, R-871, R-877, R-878, R-925, R-926, R-935, R-939, R-1011 and R-1012 rule them.* **Any axis may be
paired with any other, of any kind** (Part 3's mixed-axis rule, extended). The only refusals are genuine
contradictions, each a variant of `chart_authoring::validate::Refusal` with a named reason; reduced capability is a
label, never a refusal. The list is closed.

| Refusal | The contradiction | Example |
|---|---|---|
| `AxisAndHold { axis, quantity }` | an axis and a constraint on one quantity | `L_z` axis + hold `L_z`; `E`, `K`, `KE` or `Q` axis + hold `E` (generalises R-25) |
| `SameQuantity { a, b }` | two axes are rank 1 | `(K, KE)`; `(E, K)`, `(E, Q)` or `(K, Q)` at a frozen configuration; `(α, ‖λ‖/‖ρ‖)`; `(m_b/m_a, m_a/m_b)`; `principia_chart_reference.md` § 6.5's rows 17 to 20 as they refuse |
| `NoJointInverse { a, b }` | the pair has no inverse | a registered `none` row picked as an axis; a typed combination with no registered inverse, a registered `none` row typed as an expression included, takes R-849's general solve instead and never reaches it (R-1011) |
| `SizeOnly { axis }` | it would be constant | `I`, `R`, `ℓ` |
| `View`, existing | R-25, R-670 | a `forbids_energy_normalisation` chart + any `E*`; `ε ≠ 0` + any `E*` |
| `NoWarpWritten`, existing | no warp written (R-471 H2, R-506 W2) | `PE` |
| `InvariantNotDownstream`, `UnderDetermined`, `DoubleWritten`, `DependentDirections`, `NonFiniteDirection`, `NoSweptAxis`, `Flag`, existing | this Part's check | unchanged |

`UnderDetermined`, `InvariantNotDownstream` and `NoWarpWritten` are not contradictions in that sense, but they are this
Part's check and R-506's words, and those win; in the Chart builder the first can't be reached, since the slice always
fills what an axis leaves. A rest chart under hold `L*` is not refused: it is a well-defined family, each pixel
`(L*/I)·Jr`, labelled `not at rest` (R-841, RQ-722 (b)); the design's `HoldAgainstConvention` refusal is retired. A
chart that holds `L_z` by construction, `(ν, K)` and its strips, is not refused either: hold `L_z` there **is** its held
value. `Q` and `K` at a *moving* configuration are still one quantity under hold `E`, since `K = E − U`.

**Hold `L_z`'s refusals** (R-25, R-670, R-836). A chart where `L_z` is an axis (`(L_z, E)`, `(L_z, K)`, `Φ_θ,L_z`, an
`L_z`-axis `MixedEnergy`) carries `forbids_lz_hold`, and every `Some(L*)` is refused there, `Some(0)` included. A
non-finite `L*` is refused as a non-finite `E*` is. `(ν, K)`, `Φ_θ,K`, the strips and the `K`, `KE`, `E` and virial
`MixedEnergy` charts forbid `E*` and read `L*` as their held value, so the rotation never runs after a construction and
can't break a `K` axis. A softened run (`ε ≠ 0`) keeps hold `L_z`, since `L_z` does not read `ε`; hold `E` stays refused
there (R-670). The decode-side outcomes are `principia_dd_decoder.md` §3.7's.

**Two axes fixing one quantity are decided by the registry's table**, and a test proves the table complete: it
estimates the pair's rank on a grid, and fails any pair of rank below 2 everywhere that the table does not refuse, and
any refused pair of rank 2.

**The labels** (reduced capability, never a refusal):

| Label | Meaning |
|---|---|
| route: `closed form` or `table` | the fragment's route for the chart (R-935, RQ-765 (a): REQ-CHART-098's label becomes the fragment route, never a refusal) |
| `numerical` | a numerical inverse or R-849's general solve: its branch rule and seam shown, its convergence reported per pixel (REQ-CHART-145) |
| `ray-degenerate` | a line of pixels is one system; areas are not system fractions (`system_image`, above) |
| `1-swept family` | an annotation axis (above) |
| `measure: chart area` | area is not Liouville measure (Part 2.5, "Links carry a measure") |
| `branch: <rule>`, `seam` | a numerical inverse's branch rule and its seam (`principia_chart_reference.md` § 6.7) |
| `axis reads before the hold` | a momentum axis under a hold reads the value it swept, not what is integrated, a hidden `p_fixed` or `p_λ` tilt target included (`principia_chart_reference.md` § 6.4) |
| `reduced domain` | a paper's domain of starts shown as a labelled view: the mirror for `η ≥ 0`, the body swap at equal masses for `ξ ≥ 0`; on the third-body pair it is the symmetry filter's output and the full disc its one click off (R-871); the angle pair's axes can't carry the sign, so it keeps the label with no filter (R-878) |
| `one orientation` | an inverse reaching one orientation by its own coordinates, as `\|n_w\|`'s aliases do; every shape-sphere chart shows the full `θ` and `φ` range (R-544) |
| `mirror pairs shown once: η ≥ 0` | the symmetry filter's statement, one per declared symmetry |
| `paper scale differs` | a `gauge` axis at `I = 1`, a held `L*` or `E*` on region D, or a chart read against a paper that normalises `E`, so `ℓ` moves across it (`principia_chart_reference.md` § 6.6.5) |
| `Q above the floor` | a warped virial axis at a held `L_z ≠ 0` with `Q₀ = 0`: the excess above the rotational floor, the state's own `Q` in the readout; at `Q₀ > 0` the axis is labelled `Q`, the columns where `Q₀` is infeasible marked as the floor is (R-877, RQ-741 (a)) |
| `not at rest` | hold `L*` on a chart declared at rest: each pixel the rigid rotation `(L*/I)·Jr` (R-841) |
| `approximate` | a conserved row steady only in a limit (the registry's class) |
| `per pixel of this chart` | a statistic not weighted into a stated measure (Part 2.5, "A statistic says what it counts") |
| `one labelling` | a statistic over region D at unequal masses, where D holds one of six labellings (R-843) |
| `seam-contaminated` | a pixel's spread or `error_ratio`, where its copies straddle a seam (`principia_dd_refinement_policy.md` § 1) |

The design's `live`, `CPU batch: not baked` and `CPU batch: numerical` give way, under R-926 items 5 to 8, to the
route and `numerical` rows above: every chart is filled from the CPU's f64 evaluation, so no label names a CPU batch.
In the browser the labels are the native ones, with no "not live" or "CPU batch" label of their own (R-1012, RQ-817
(a)). Clamped and saturated areas carry no mark by default; an Overlays › Chart toggle, off by default, marks them
(R-939). **R-506 W1 is unchanged:** a chart with any linear axis logs no decoder-bug warning, a chart whose physical
axes are all warped does, and a chart with a mask or a seam logs none on the pixels the mask or seam explains.

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
