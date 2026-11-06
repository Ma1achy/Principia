# Principia — chart composition: axes from primitives, in the Chart builder

**Status: Proposed, 6 Nov 2026, on the owner's orders of that day. Not a ruling. Nothing here is implemented until the
rulings report is answered.** No code is written by it. Its one-way doors are not decided here: each is marked **door**,
and waits on the rulings item named beside it (`RQ-NEW-n`). Two are agreed in principle by the owner, and still not
rulings: no fifth axis kind (RQ-NEW-1) and outside region D blank and uncounted (RQ-NEW-3).
Where this document meets a ruling, the ruling wins. Ids `RQ-NEW-n`, `REQ-NEW-n` and `TASK-NEW-n` are placeholders; the id
pass replaces them once the rulings branches of 4 and 5 Nov have merged. Those branches and PR #388 are read here as
they stood on 6 Nov: R-694 to R-720, R-791 to R-809 and TASK-M2-52's branch. PR #387, TASK-M2-37, has merged to `main`,
which this document's base predates: its code and its queue entries, RQ-682, RQ-700 and RQ-701, are read from `main`.

**The order, in a line.** Charts are built from primitives and combinations of primitives, each axis independent and
customisable, composing with what exists and adding no chart type. Any axis may be paired with any other. The only
refusals are genuine contradictions, each with a named reason. Reduced capability is a label, not a refusal.

**What this document finds, in five lines.**

1. Nearly all of it is already there: the four kinds, `Mixed`, the warps, the validator, the links, tilt and Save view
   as chart. The new things are a registry, a third lowering (CPU batch), a constraint (hold `L_z`) and two pair maps.
2. Every primitive the order names fits one of the four kinds, so no fifth kind is needed (§ 2), but the virial ratio
   fits only by a ruling already asked: RQ-682.
3. Region D's axis pair `(ξ, η)` is the **stereographic projection of the mass-weighted shape sphere from the collision
   of bodies 0 and 1** (§ 8.2). So it is closed-form at every mass, needs no `atan2`, has no pole in view, and it carries
   the angle and side-ratio pairs with it.
4. The virial axis is built by TASK-M2-37 (`AxisVirial`), with its warp written. What it lacks is RQ-682's
   answer, so "then virial" is a descriptor and a ruling, not a build (§ 9).
5. A baked axis pair costs little to build and more in the module, which every pipeline parses whole. 4 passes the
   measured criterion on one Mac and 8 doesn't, so **4 is provisional** until the Windows Vulkan run is in, and it holds
   only while decode stays its own entry (§ 11).
6. A view is a point, and every edit a path, but morph is not the general form of tilt and zoom in one formula: no single
   blend reproduces them bitwise. The path type does, with tilt and zoom as arms running their own code (§ 17).
7. Of the 6 Nov primitives (§ 3.1), most are axes on CPU batch or aliases of existing rows. Orbital elements and
   Mardling–Aarseth aren't axes, since a ruling parks them; the rejected list is stated there.

---

## 0. The order's rules, checked

Each rule was read against the code, `decisions.md` and both rulings branches. A conflict is the ruling's, and goes to § 14.

| Item | Result |
|---|---|
| 1. Extend the builder | §G7, R-471, R-506, R-407, R-408, R-475, R-513, R-514, R-478 to R-482, §0.6 and R-670 are as the order cites them. "A new axis kind" meets "a closed set": RQ-NEW-1. R-798's words are about PA-1: RQ-NEW-1 |
| 2. Two layers | Holds, but a feasibility warp is not per-axis (§ 5.5, row 3). Hold `E` exists; hold `L_z` is new physics: RQ-NEW-4 |
| 3. Registry | R-407 clause 3 is as cited; `I = 1` at every decode is dd_decoder §5's identity 1. "CPU batch only" meets seam 3: RQ-NEW-8. Side ratios meet R-798: RQ-NEW-2 |
| 4. Gaps | "Today only the TR-1 engine scan" is not so: no code has it (§ 10.1). TASK-M7-43 and R-476 are as cited. Rule 6 is as cited. The already-covered list holds |
| 5. Region D, then virial | Defined in dd_validation_orbits §1.6. REQ-CHART-070's virial warp is written by TASK-M2-37: § 9. Virial's kind is RQ-682 |
| 6. Tests | § 12 |
| 7. Doors | § 13, § 14 |
| Any pair | §1.3 and REQ-CHART-008 hold. R-669's refused charts: RQ-NEW-7. R-594 item 5: RQ-NEW-9 |

---

## 1. What exists, and is kept

The model is the Chart builder (`principia_render_gui_spec.md` §G7). None of this is rebuilt, and each row was checked
against the code.

| Exists | Where | Used here as |
|---|---|---|
| Four axis kinds, "a closed set"; "no fifth" | chart_decoder_contract Part 3; `chart_authoring::axis::AxisKind`; REQ-CHART-004 | every primitive declares one (§ 3) |
| Axis kinds lower, never interpreted | core_design seam 2; lowering Parts 1 to 4; REQ-CHART-004 | a pair runs on the GPU only as a baked entry (§ 7) |
| `Mixed<H, V, P>`, any two quantities, the other six frozen | `kernel::chart::mixed`; chart_reference §1.3; REQ-CHART-008 | the pair's home |
| Warps and the linear option | R-471, R-506, R-508; `kernel::chart::energy::{Mapping, Mapped}`, TASK-M2-37's | the warp slot of each axis (§ 4) |
| Extension types, primary ranges | R-407, R-408, R-472, R-537; chart_reference §5.4 | each primitive's natural type (§ 3) |
| Tilt, slice, Save view as chart | R-473, R-474, R-475, R-513, R-514, R-801 | composition with a composed chart: `Tilted<C, P>` wraps it |
| Domain preview | §G7; `validate(u, v)`, R-26 | shows why (§ 6.3) |
| Links | R-478 to R-482 | every axis reads the active link; a preset may pin (R-516) |
| `forbids_energy_normalisation` | chart_reference §0.6; R-25; R-670 | the pattern hold `L_z` copies (§ 5.3) |
| Well-posedness validator | `chart_authoring::validate`; contract Part 5 | gains refusals, loses none (§ 6) |
| Annotation axes, a 1-swept family in 2D | contract Part 5; the Euclid plane | the 1D chart (§ 10.1) |
| Point imports | `crates/validation/src/import.rs`, TASK-M2-20 | region D's points (§ 8.5) |

**Already covered, not rebuilt (the order's list, confirmed):** symmetry reduction by canonicalisation; the shape
sphere's two halves kept (R-544) and relabelling kept distinct; landmark-centred presets (R-798's among them); pivot
through Save view as chart; tilted configuration–momentum planes (R-473).

**Existing presets and saved charts reopen bitwise.** Every new field defaults to what is built today, and a default
field is not serialised, so the canonical form (gui_state_contract §2; R-484) and the sim key do not move. § 12 tests it.

---

## 2. The two layers

```
Axis    (independent): primitive or combination · warp · extension type · what it holds fixed
Chart   (optional, couples both axes): pair map · constraints (hold E, hold L_z)
Lower   (one of three): uniform affine · baked entry · CPU batch        ← a label, never a refusal
```

An axis written alone is independent. § 5 says each place where that is not so. In the CPU authoring layer:

```rust
struct AxisSpec   { primitive: Combination, warp: Warp, extension: Extension, holds: Residual }
struct ChartSpec  { axes: [AxisSpec; 2], pair_map: PairMap, constraints: Constraints, slice: Slice }
```

`ChartSpec` is data, authored on the CPU, and the kernel never sees it (REQ-CHART-004). `validate` turns it into a
`ValidatedChart`, as today, and `resolve` lowers it.

**The kinds stay four.** The order's item 1 says "a new axis kind, warp or pair map"; the contract says "a closed set"
and "no fifth". Read together: a primitive is a new *entry* under one of the four, with its metadata. This is
**door** RQ-NEW-1. In this reading the lowering appendix's Anosova row, "physical × physical", is kind 2 × kind 2 on the
configuration block, as the shape sphere's row is, residual none: the row is conformed, not a new kind.

---

## 3. The primitive registry

One table, `chart_authoring::registry`, CPU-side. Every row declares:

- **forward**: state to value, always; the Inspector, the contours and every test read it.
- **inverse class**: `closed` (live on the GPU where baked), `numerical` (CPU batch only, never in the kernel) or `none`
  (overlay only). A closed inverse may need a partner: it is then registered on the pair.
- **kind**: one of the four.
- **extension**: the natural type. Angle-like rows are flagged: a periodic or pole-crossing declaration, or affine on an
  angle-like axis, needs physics review (R-407 clause 3; chart_reference §5.4, "an angle-like quantity declared affine
  would redraw systems and count them again").
- **scale**: `free` (a shape or a ratio), `gauge` (fixed by `I = 1`, so a paper's value needs the recorded rescale,
  dd_validation_orbits §2) or `size-only`, refused as an axis, since `I = 1` at every decode (dd_decoder §5, identity 1)
  makes it constant.

In the formulas `r_ij = ‖r_i − r_j‖`, `γ_k` is the interior angle at body `k`, and `(ξ, η)` is § 8's plane. Bodies are
0-based (R-22), shown 1-based (R-449).

| Primitive | Forward | Inverse | Kind | Extension | Angle-like | Scale |
|---|---|---|---|---|---|---|
| latent direction `q` | `z·q` | closed, assigned | 1 | affine | no | free |
| `α`, `β` | decoder §0.2 | closed, the link's | 2 | bounded | **yes** | free |
| shape `θ`, `φ` | §3.1 | closed on the pair, §3.2 | 2 | periodic, pole-crossing (existing) | **yes** | free |
| Jacobi `p_ρ`, `p_λ` components | §0.3 | closed, the link's | 2 | affine | no | gauge |
| third body `(ξ, η)` | § 8.2 | closed on the pair | 2 | `ξ` bounded, `η` bounded | no | free |
| interior angles `γ₀, γ₁` (any two of three) | `acos` of the law of cosines | closed on the pair, § 8.3 | 2 | bounded | **yes** | free |
| side ratios `r₀₂/r₀₁`, `r₁₂/r₀₁` | `r_ij` | closed on the pair, § 8.3 | 2 | bounded | no | free |
| Jacobi ratio `‖λ‖/‖ρ‖` | §0.2: `tan α·√(μ_ρ/μ_λ)` | closed, residual `β` | 2 | bounded | no | free |
| mass ratio `m_b/m_a` | `m` | closed, through the link (R-506 W3) | 2 | bounded warped, affine linear | no | free |
| mass control `z_μ` | `z` | closed, assigned | 1 | affine | no | free |
| Burrau `ν`, `θ` | §4.3 | closed, the embed | 4 | bounded | `θ` **yes** | free |
| `E` | `K + U` | closed, solve into momentum | 3 | warped bounded (R-471) | no | gauge |
| `K`, `KE` | §2.2 | closed, solve into momentum | 3 | bounded, `t > 1` continues (R-508) | no | gauge |
| `L_z` | `Σ m r ∧ v` | closed, solve into momentum | 3 | bounded warped, affine linear | no | gauge |
| virial ratio `Q = 2K/|U|` | dd_decoder §3.6 | closed; at `L_z = 0`, `K* = Q·|U|/2`; at a held `L_z ≠ 0` the warped axis reads above the floor, § 9 | **3, by RQ-682** | bounded both sides | no | free |
| area, perimeter | triangle | numerical on the pair (figure's Tier 2c) | 2 | bounded | no | gauge |
| `PE` (`U`) | §0.6 | **none**: REQ-CHART-070, no chart offers it until a warp and a residual are written | 3 | — | no | gauge |
| `min(r_ij)/max(r_ij)`, any `min` or `max` of primitives | piecewise | **none** | — | — | no | free |
| pair binding energies | `½μv² − m_i m_j/r_ij` | **none** | — | — | no | gauge |
| `I`, `R`, `ℓ` | §0.5 | refused: **size-only** | — | — | no | size-only |

**Combinations.** A combination is an expression over primitives, in the stain's expression syntax, the subset §4.1 of
`principia_lines.md` needs (R-509). It inherits the weakest inverse class of its parts unless a closed form is registered
for it. Its `angle-like` is true if any part is. Its scale is computed, not declared. On the GPU no expression is ever
interpreted (lowering Part 1): a combination reaches the GPU only as a registered, baked row, and otherwise runs as CPU
batch, where an interpreter is allowed, since it is not a kernel.

**Scale-size refusal is a computed property too.** The `size-only` column is declared, and a test checks the declaration:
it samples decodes and fails any row whose forward is constant on them and is not declared `size-only` (§ 12).

**Mass-weighted shape.** The shape sphere is mass-weighted (§3.1). Geometric angles and side ratios are not: they invert
onto the sphere with the masses in the formula (§ 8.3). So an axis states which it means, in its `holds`: `(γ₀, m₂)`
holding a geometric triangle while the mass moves is a different family from `(θ, m₂)` holding a point on the sphere.
Both are legal and each is labelled.

**What is not an axis in the registry, and why (R-669).** R-669: "no binary-plus-single chart is built", and TS-2 reads "not
scored: no fixed-k velocity chart". So no row with an inverse names a binary phase, an eccentricity or an impact
parameter, and none is a fixed-`k` prescription; forward-only overlay rows may (§ 3.1). A builder option can't quietly reverse
a ruling. **Door** RQ-NEW-7 asks what a user-built pair that lands on one of those planes by other means counts as;
meanwhile nothing built here scores those targets.

### 3.1 The primitives of 6 Nov

The owner's order of 6 Nov, part C. Each identity stated is checked to 10⁻¹³ over 20,000 random states
(`orch/scratch/chartcomp/measure/prim_check.py`, not committed). **Lowering: all CPU batch, labelled `CPU batch: not baked`**,
since the cap's slots are taken (§ 11); only `SuvakovPlane` is baked, by its own ruling. Each closed row is a case of
`prim_round_trip` (§ 12), which tests `forward(inverse(x)) = x`, with a sign flipped as its control.

| Primitive | Forward (`I = 1`) | Inverse | Extension | Holds | Contradicts | Targets | Verdict |
|---|---|---|---|---|---|---|---|
| Jacobi momentum polar: `‖p_ρ‖, ∠p_ρ`, `‖p_λ‖, ∠p_λ` | `hypot`, `atan2` on `(q₀, q₁)`, `(q₂, q₃)` | closed on the pair; it's polar on the pair map (§ 5.1) | `‖·‖` bounded; `∠` periodic, **angle-like** | the other vector | a component axis of the same vector with two of these | none named | **axis**, no new primitive: a pair map |
| body `i`'s speed, heading in the canonical frame | `‖pᵢ‖/mᵢ`, `atan2(pᵢ)` | closed, the other Jacobi vector frozen | bounded; periodic, **angle-like** | `p_λ` for bodies 0, 1; `p_ρ` for body 2 | body 2 is `‖p_λ‖/m₂`, `∠p_λ`: **an alias**, `SameQuantity` | none named | **axis**; body 2 aliased. Heading is relative to `ρ̃` on `+x`, so under a shape axis it turns with the frame |
| flight-path angle `γ_ρ`, `γ_λ` | `atan2(x × p, x·p)` per Jacobi vector | closed, config downstream | periodic, **angle-like** | the speed | `γ_ρ = ∠p_ρ`, since `ρ̃` is on `+x`: **an alias**. `γ_λ = ∠p_λ − β` | none named | **axis** for `γ_λ`; `γ_ρ` aliased |
| third body's `v₂/v_esc` | `v_esc² = 2(m₀/r₀₂ + m₁/r₁₂)`, **a definition this document proposes; the corpus has none** | closed, direction frozen | bounded | direction, `p_ρ` | `‖p_λ‖` at a frozen shape, `SameQuantity` | possibly TS-2 | **door RQ-NEW-14**: it may be R-669's fixed-`k` prescription, which the corpus doesn't give. Overlay until ruled |
| `K`, `PE` | § 3 | `K` closed; `PE` **none** (REQ-CHART-070) | § 3 | § 3 | § 5.5 | as § 3 | **existing**; `PE` stays no axis until a warp and residual are written |
| `K_ρ`, `K_λ` | `‖p‖²/2μ` per Jacobi vector | closed, scale of that vector | bounded | direction | two of `K_ρ, K_λ, K` fix the third: `K = K_ρ + K_λ` | none named | **axis**, gauge |
| `L_ρ`, `L_λ` | `x × p` per Jacobi vector | closed, one component | affine | the rest of that vector | two of `L_ρ, L_λ, L_z`: `L_z = L_ρ + L_λ` | none named | **axis**, gauge; "inner" is the pair 0, 1 by label |
| `h`, the dilation rate | `Σ m r·v`, RQ-680's `(Σ m r·v)/I · I^¾` at `I = 1` | closed: `v ← v + ((h* − h₀)/I)·r`, `L_z` kept | bounded | `L_z`, shape momenta | `K_dil` | state map's targets, REQ-COL-129 | **axis, door RQ-NEW-13**, free |
| Saari split `K = K_dil + K_rot + K_shape` | `h²/2I`, `L²/2I`, the rest: `r` and `Jr` are `m`-orthogonal | `K_shape` closed, scale of the rest; `K_dil` loses `h`'s sign; `K_rot` is `L_z` | bounded | the other two terms | three of the four; `(K_rot, L_z)`; `(K_dil, h)` | none named | `K_shape` **axis**; `K_rot`, `K_dil` **aliases** of `L_z`, `h` |
| `L²E` | scale-free combination | closed, `E` or `L_z` frozen; pole at `L_z = 0` | bounded | the frozen one | two of `L_z, E, L²E` | none named | **axis** at `ε = 0`. `ε ≠ 0` sets a length, so it isn't scale-free there: label. A both-held chart is one value of it (§ 5.3) |
| interior angles at A, B, C, any two | § 3 | closed on the pair | bounded, **angle-like** | the shape's other DOF | § 5.5 row 1; any other shape pair | none named | **in the registry**; `γ₀ + γ₁ ≥ π` blank |
| angle of C about AB's midpoint | `atan2(η, ξ)` | closed with a radius: polar on § 8's pair | bounded, **angle-like** | radius | `β` at equal masses, `d = 0`: `SameQuantity` | none named; P-1's polar view is about a body (§ 5.1) | **axis**, a pair map; no new primitive |
| angle of C about AB's COM | `β` | | | | | | **is `β`**: reuse |
| bodies' polar angles about the COM | `φ₁ − φ₀`, `φ₂ − φ₀` | closed on the pair: a 2 × 2 solve from `Σ m r = 0`, then `I = 1` | periodic, **angle-like** | masses | other shape pairs | none named | **axis**. Collinear through the COM: all angles 0 or π, a line of shapes on one point, a declared seam and `ray-degenerate` |
| signed area | `½ ρ × λ = n_w/(4√(μ_ρ μ_λ))` | closed, with an azimuth | bounded | azimuth | `n_w`; elongation | none named | **axis**; through zero is collinear, the signed `n̂` (R-544) |
| distance to one named shape | `acos(n̂·n̂_k)` | closed with azimuth: the radius of § 5.2's exponential map | bounded, **angle-like** | azimuth | | none named | **axis**, a pair map about `n̂_k`. Euler `n̂_k` needs a solve per mass |
| distance to the *nearest* of a set | `min` of the above | **none** (§ 3's `min` row) | | | | none named | **overlay only**: not an axis, seams at the bisectors |
| elongation, principal-moment ratio | `I₁,₂ = (1 ∓ √(1 − n_w²))/2`: `I₁I₂ = n_w²/4` | closed, one orientation | bounded | azimuth | **signed area's size: one quantity at `I = 1`** | none named | **alias** of `\|n_w\|`, warped |
| inner share of `L_z` | `L_ρ/L_z` | closed, `L_z` frozen; pole at `L_z = 0`, tagged | affine | `L_z` | two of the three `L`s | V-11, parked | **axis**; sign is co- or counter-rotating |
| inner, outer elements `a, e, ω, f` | osculating, pairs by label | closed on a pair, a radial-sign branch; mean anomaly numerical | | | | V-11, parked; SL-1, TR-1, MK-1 "no chart" | **overlay only, door RQ-NEW-12** |
| periapsis ratio | `a_out(1 − e_out)/a_in` | none | | | | V-11, parked | **overlay only**; undefined at `e_out ≥ 1` |
| distance from Mardling–Aarseth | not in the corpus | **none** | | | | V-11, parked | **not an axis** |
| distance from Hill stability (Marchal, Saari, Zare) | `L²\|E\|/(L²\|E\|)_crit(m) − 1`; the threshold at the Euler central configuration, `25/972` at equal masses, `M = 1`; no corpus source | closed through `L²E`; a mass solve | bounded | as `L²E` | as `L²E` | none named | **axis** at `E < 0`, `ε = 0`, `L_z ≠ 0`; else blank, labelled `no Hill threshold`. The source is owed |
| `SuvakovPlane`, two velocity parameters | REQ-CHART-091 | closed, baked by R-669 | its definition | `x = (−1, 1, 0)`, `v₁ = v₂`, `v₃ = −2v₁` | a shape axis: collinear | HU-1, HU-2, AT-D1, AT-D3, OC-4, SD-1's closure | **stays one M9 task** (TASK-M9-40): a reference here, nothing built |
| isosceles families with matched velocities | **no definition in the corpus** | | | | | none named | **door RQ-NEW-14**; a definition requirement |

**The verdicts that aren't as asked.**

- **Mardling–Aarseth isn't an axis.** Its form isn't in the corpus; it is a fit for hierarchical systems, it needs a bound inner and outer orbit, and it has
  no `t = 0` definition without a clear hierarchy. Hierarchical triples are parked (R-668 item 5).
- **Elements aren't results, but they aren't axes yet.** Osculating elements exist at `t = 0` for any state, by the labelled
  pair. They mean an orbit only with a clear hierarchy, and choosing the pair by nearness adds a seam. As axes they would
  be the parked hierarchical slice, and the R-669 binary-plus-single chart by another name. As overlays they're forward only,
  and build no chart.
- **Hill stability is sound under `I = 1`,** since `L²E` is scale-invariant, and has no meaning under softening, where the
  threshold's theorem doesn't apply.
- **Saari against RQ-680.** `h` **is** RQ-680's dilation rate at `I = 1`, and `K_dil = h²/2I`. So the split supplies the
  definition RQ-680's option (b) lacks, and SM1's targets only project states, so they need `h`'s forward and no baked
  entry. **Against REQ-CHART-051:** no conflict. `d_E` removes `Jr` only, so it raises `K_dil` and `K_shape` together; `d_h ∝ r`
  would be a new named direction, defined the same way.

**Rejected as axes**, as the order states them, each checked:

| Rejected | Why | Check |
|---|---|---|
| total momentum | zero by COM | `C`'s boost; `Σp = 0` by construction |
| absolute direction | rotation gauge | `C`'s rotation |
| anything size-only | `I = 1` | `SizeOnly`; a computed test (§ 12) |
| `z` components | planar | no third coordinate exists |
| discrete labels | colourings, not axes | no inverse: `none`, overlay; the third-body pair's fold stays a label |
| anything measured after `t = 0` | results, not axes | an escape time or a final eccentricity is a coloured output |

---

## 4. The axis layer: warps

The warp is `t ∈ [0,1]` to the axis's value. Four, closed:

| Warp | Formula | Extension it implies | Exists |
|---|---|---|---|
| linear | `lo + (hi − lo)·t` | affine | yes, `Mixed` and R-471 E2 |
| power | `lo + (hi − lo)·t^γ` | bounded below `t = 0` | yes, §2.1's `t^γ_K` |
| log | `lo·(hi/lo)^t`, `lo, hi > 0` | bounded | yes, the Euclid plane's `m(u)`, the mass ratio's `q(t)` |
| feasibility | the axis's own (R-471, R-506, R-708) | the axis's own | yes |

**R-506 stands over all of them.** A physical-quantity axis is warped to its feasible range by default, linear as an
option. "Warped" there means the *feasibility* warp. Power and log are options an axis takes on top where its own warp
is written; on an energy-type axis they are not offered until written (REQ-CHART-070's pattern).

**A warp is not lowered as a kind.** Its choice is a uniform word, as `ShapeMap` is (TASK-M2-24's precedent; lowering
Part 3, "workgroup-uniform"): reversible, V1 in § 13. The linear arm is the expression `Mixed` computes today, the
same operations in the same order, so an existing chart's bits do not move.

---

## 5. The chart layer, and where two axes can't be independent

### 5.1 Pair maps

`(a, b) = M(u, v)` sits before the chart's axes. The default is affine, the identity on the axes, and `M` is part of the
chart's type, baked with it: it is `Φ` structure (lowering Part 3). A generic wrapper, `Paired<C, M>`, follows `Tilted<C, P>`.

| Map | Formula | Extension | Note |
|---|---|---|---|
| affine | `(a, b) = (u, v)` | the axes' | the default; bitwise today's |
| polar about `c` | `a = c_a + r cos ϕ`, `b = c_b + r sin ϕ` | `ϕ` periodic, `r` pole-crossing, partner `ϕ`, or bounded | no new type (R-407 clause 3's review); P-1's rings as a view, not their sampling |
| log-polar | `r = r₀ e^{t}` | `r` bounded | the same, no origin in view |
| exponential | §3.3's form, about `n₀` | bounded | generalised off the sphere, § 5.2 |

Polar and log are general options, so they apply to any pair, and a polar view of `(ξ, η)` about a body is P-1's picture.

### 5.2 Curved slices off the shape sphere

§3.3's exponential map needs a point on a 2-sphere and two tangents. It does not need the shape sphere. Two other
2-spheres sit in the state:

- **Momentum.** At fixed configuration and masses, `{p : L_z = L*, K = K*}` is generically 2-dimensional
  (inverse_encode_contract Part 5, kind 3). It is `v = v^(L) + a·w`, `a` fixed and `w` a unit vector in the 3-space
  orthogonal to `Jr` in `⟨·,·⟩_m`: a 2-sphere, and §2.2's seeded direction is one point on it. The map sweeps `w`.
- **Mass.** `(√m₀, √m₁, √m₂)` is a unit vector, so the simplex is an octant of a 2-sphere. The map sweeps it, every
  pixel's masses positive and summing to 1 by construction.

Each is a definition under physics review, closed by TASK-NEW-7 (R-72). Neither is a new chart type: each is
`ExpMap<Target>`, the shape sphere's being `ExpMap<Shape>`, its bits unchanged.

The momentum sphere's frame is built from R-82's seed, which flips with the configuration. With the configuration frozen
across the chart, the frame is constant. Tilted into a configuration direction it can flip, and that is a seam (§ 5.5).

### 5.3 Constraints: hold `E`, and new: hold `L_z`

**Hold `E` is `η_E`, unchanged:** `pᵢ ← √((E* − U)/K₀)·pᵢ` (§0.6). It is a projection after the decode, and it scales
`L_z` too.

The physics reviewer's note of 6 Nov (PR #422, review 5803342655) checked this section and amended it; the amendments are
folded in below, and its script is `orch/scratch/422-physics/hold.py`.

**The split.** With `⟨a, b⟩_m = Σ mᵢ aᵢ·bᵢ` and `J(x, y) = (−y, x)`, `L_z(v) = ⟨Jr, v⟩_m` and `‖Jr‖²_m = I`, so every
velocity splits orthogonally:

```
v = (L₀/I)·J r + δv        ⟨Jr, δv⟩_m = 0        K = L₀²/2I + K_δ        K_δ = ½‖δv‖²_m
```

**Hold `L_z` (`L*`)** is `η_E`'s twin, after the decode and before the derived invariants, in the same place. It replaces
the first term:

```
v ← v + ((L* − L₀)/I)·J r            with I = 1 at every decode; Σp stays 0 because the CoM is at the origin
```

That is the `m`-orthogonal projection of `v` onto `{L_z = L*}`, the smallest velocity change `‖Δv‖_m` that reaches `L*`.
It is not a minimal-kinetic-energy state: `δv` stays. (inverse_encode_contract Part 5's theorem, that `v ∝ Jr` minimises
`K` at given `L_z`, is about the fibre's bottom, not this step.)

**Both held.** `η_E` alone would break the held `L_z`. `{L_z = L*, K = K*}`, with `K* = E* − U`, is a sphere of radius
`a = √(2K* − L*²/I)` about `(L*/I)·Jr` in the 3-space orthogonal to `Jr`, and the nearest point to `v` keeps `δv`'s direction:

```
v ← (L*/I)·J r + η·δv        η = √((K* − L*²/2I)/K_δ)        feasibility: 2K* ≥ L*²  (I = 1)
```

This is inverse_encode_contract Part 5's `v^(L) + a·w` with `w = δ̂v`, and the curve `v(s) = v⁽ᴸ⁾ + eˢ(v − v⁽ᴸ⁾)` that
chart_decoder_contract gives for `d_E` (REQ-CHART-051), with `η = eˢ`: the corpus's own decomposition, twice. Checked over
20,000 random states in f64: both targets and `Σp = 0` within 1.1·10⁻¹⁴. With hold `L_z` off this path is not entered, the
choice being on the switch, never on `L* = L₀`, and `η_E`'s bits do not move. This is new physics, off by default:
**door** RQ-NEW-4. Three definitions (R-72) go with that door: REQ-NEW-19 to REQ-NEW-21.

**The computed form is not the formula above.** Near a rigid rotation `K₀ − L₀²/2I` cancels, goes negative and makes `η`
non-real; and the Lagrange and Euler relative equilibria are such states, so a chart locked on one sits there. `L_z(δv)` is
rounding-sized, not 0, and `η` multiplies it. So `K_δ` is computed from `δv` itself, in dd_decoder §3.7's scaled form, and
`Jr` is projected out a second time after normalising:

```
δv ← v − (L₀/I)·J r
s  ← largest component of |δv|                       s = 0 is exact, an outcome below
δ̂  ← δv/s      δ̂ ← δ̂ − (⟨Jr, δ̂⟩_m/I)·J r            the second projection
K̂  ← ½‖δ̂‖²_m
v  ← (L*/I)·J r + √((K* − L*²/2I)/K̂)·δ̂
```

The reviewer's f32 figures, worst `|ΔL_z|`, one pass against two: `‖δv‖/‖v‖ = 10⁻¹`: 2.9·10⁻⁶ against 1.8·10⁻⁷; `10⁻³`:
3.2·10⁻⁴ against 1.9·10⁻⁷; `10⁻⁵`: 2.8·10⁻² against 1.4·10⁻⁷; `10⁻⁷`: 1.3 against 4.2·10⁻⁷. A near-rigid pixel stays
ill-conditioned in direction, as `η_E` is near rest: f32 and f64 can differ there by `O(1)` in `δ̂` with no tag. The tag
is the exact test `s = 0`, not `‖δv‖²_m ≤ ε_w‖v‖²_m`, which would make a branch tag depend on rounding, against the
comparison-only rule.

**The outcomes, in this order** (REQ-NEW-20). dd_decoder §3.7's list doesn't cover them, and §2's cause set is closed:

| # | Test | Outcome |
|---|---|---|
| 1 | the collision pole | `ENERGY_UNSCALABLE` |
| 2 | `E* < U` | `ENERGY_INFEASIBLE` |
| 3 | `2K* < L*²`, by the existing exact `infeasible` routine, never `E* < U + L*²/2I`, which rounds at the decision | **new**: `ENERGY_INFEASIBLE`, widened, since this stage raises it. §2's table and REQ-DEC-041 need the words, and §3.7's "every `E* ≥ 0` is feasible" is false with `L*` held |
| 4 | `s = 0` at `2K* = L*²`: a rest or exactly rigid `v` on the floor | no tag, and **the rotation still applies**: a rest pixel with `L* ≠ 0` becomes `(L*/I)·Jr`. §3.7's "momenta untouched" can't be copied |
| 5 | `s = 0` above the floor | `ENERGY_UNSCALABLE`. No seed: §3.7 tags the rest start and doesn't seed it, and `d_E`'s seed fallback is a tangent, not a state |
| 6 | otherwise | apply; the floor with `s ≠ 0` gives `η = 0`, the rigid rotation |

A tagged pixel keeps its momenta as decoded, with neither hold applied (§2.2's `a = 0` is the construction's, not this
step's). A non-finite `L*` is refused as a non-finite `E*` is, with `physical.rs`'s treatment of an `L_z²` that has no
finite float as §2.1's exception. Every outcome is a label on the decode, never a refusal.

**Lowering.** `L*` is a uniform. `DecodeUniforms.energy` has two spare words and an unused bit, so the switch and the
value fit with no new row, and with the switch off the uniform's bytes are today's. `sample_with` is the one place the
step goes, and the fragment decode calls it. "No new variant" holds only with no tilt target on `L*`, as below.

**Tilt and slice: two different things.**

- *A chart holding `L_z` by construction* (`(ν, K)`, `Φ_θ,K`, `Φ_strip,K`): R-720 applies, by value, `L* + d`, and to
  slice as well: "The same rule governs slicing the held L_z as well as tilting it." The hold is read inside the chart,
  as `held`, and `d` reaches it there. For `MixedEnergy` it is RQ-701's.
- *A free chart* (latent, `Mixed`, `MixedMassRatio`, the shape sphere, region D's eq. 3 family): `L*` is not a hidden
  coordinate. It is a view setting like `E*`, and `E*` has no tilt. A `z` tilt leaves `L*` and `E*` fixed at every pixel,
  which is what "held" means, and the hold runs per pixel on the tilted `r`, `U` and `v`. **Recommended: no tilt target
  on a free chart**; the `L*` slider is a slice, by value. A by-value `L* + d` varying over the window would be a new tilt
  target, like R-554's `p_λ`, and `d` doesn't reach the decode stage: `Shift` targets control indices and is consumed in
  `Tiltable::decode_shifted`, while the hold sits after it.
- **RQ-NEW-4 closes the held-`L_z` half of RQ-701 only.** RQ-701's held-`K` half, `K_max` on a lone `L_z` axis, is
  untouched; RQ-679, the slider span, stays as it is.
- Infeasible pixels under a tilt are labels, per pixel. No hold is a warped axis, so R-708's "every pixel a system" doesn't
  bind here.

**Encode (REQ-NEW-21).** inverse_encode_contract Part 5 says encode reuses decode, and Part 6 step 4 tests
`‖D(Φ(c(x))) − C(x)‖_phys`. Under a hold neither is automatic: where `D` is unheld a state lands on a pixel that shows a
different system, and the kernel's encode reads no `e_star` today, so hold `E` has the same gap. So step 4 uses the held
decode, and a state off `{L_z = L*}` gets a stated outcome. Which one is a definition REQ-NEW-21 owes (R-72); this
document doesn't pick it.

**Same refusal rules** (R-25, R-670): the flag `forbids_lz_hold`, true where `L_z` is an axis (`(L_z, E)`, `(L_z, K)`,
`Φ_θ,L_z`, an `L_z`-axis `MixedEnergy`); every `Some(L*)` refused there, `Some(0)` included (§ 6.1). `(ν, K)`, `Φ_θ,K`, the
strips and the `K`, `KE`, `E` and virial `MixedEnergy` charts forbid `E*` and read `L*` as their held value (V10), so the
rotation never runs after a construction and can't break a `K` axis: R-708's floor keeps every pixel a system. Both held is
reachable only on free charts, and no chart was found where holding both contradicts what the chart holds. A softened
run (`ε ≠ 0`) keeps hold `L_z`, since `L_z` does not read `ε`; hold `E` stays refused there (R-670).

**What else it means.** `I = 1` holds where the step runs, after `chart.decode`, so after `C` and the scale gauge; on charts
bypassing the canonical frame, `L*` is read in `C`'s frame, after any mirror. `E·L²` is scale-invariant, so at `I = 1` both
held fixes `L²|E|` over the window even where each paper-unit value varies with `ℓ`; a held `L*` or `E*` on region D takes
the `paper scale differs` label. On the shape sphere under `L* ≠ 0`, `(θ, φ)` and `(θ, π − φ)` aren't mirror images even
for a mirror-symmetric `p_fixed`, since the mirror has `−L*`: that qualifies chart_reference §3's sentence. Area statistics
under a hold stay `measure: chart area`: the projection is not Liouville.

### 5.4 What the constraints do to a momentum axis

Hold `E` already moves a chart's momentum axis off its label: `q_k` is what the axis swept, not what is integrated. That
is today's behaviour on `Mixed`. Hold `L_z` does the same. It is reduced capability, so it is a label: "axis reads the
value before the hold", which also names a hidden `p_fixed` or `p_λ` tilt target.

**Rank loss has two cases**, both `ray-degenerate` (§ 5.5, row 9). (i) Hold `L_z`: the swept plane contains `Jr`, which is
an axis plane at a real configuration, `(p_ρ,y, p_λ,y)` at a collinear shape; singular values 1.35 and 2·10⁻¹⁰. (ii) Hold
`E` on, with or without `L_z`: **any plane through a point with `δv = 0`**. The radial projection sends the plane to a
circle, so the chart is rays: both held, 5.3 and 2·10⁻¹⁰; through a rigid rotation, 5.3 and 6·10⁻¹⁰; hold `E` alone through
rest, today, 2.5 and 10⁻¹⁰. That is the default `z₀`, so a momentum-momentum plane at rest under both holds is one curve of
systems. A generic plane keeps rank 2 (6.2 and 0.45).

### 5.5 Every case where two axes can't be independent

The order: "Document every case where two axes cannot be independent instead of hiding it". They fall in three groups:
the same quantity (refused, § 6.1), coupled by a map (the pair is legal, and it says so), and coupled by the state.

| # | Case | Why | What the chart does |
|---|---|---|---|
| 1 | Two of `γ₀, γ₁, γ₂` | `Σγ = π`, so the third is fixed | any two are the shape's two DOF; a third is no axis |
| 2 | Side ratios | `(r₀₂/r₀₁)·(r₁₂/r₀₂) = r₁₂/r₀₁` | two free; the triangle inequality couples their domain: a non-triangle is blank |
| 3 | `(L_z, E)`, `(L_z, K)` | `|L_z| ≤ √(2IK)`, §2.1 | the feasibility warp is a **pair** warp: `L_z`'s line reads `t`. Not per-axis, and it can't be |
| 4 | A held `L_z` under a `K` axis | `K ≥ L_z²/2I` | R-708's additive floor: the axis's line reads the held value |
| 5 | Pole-crossing | R-407: "the partner axis shifted by half its period" | the extension couples the axes past the square |
| 6 | Polar at `r = 0`, exponential at `r = π` | the angle has no meaning there | `ray-degenerate` row or disc; `−n₀` counted once (REQ-CHART-078) |
| 7 | Kind 4 | one `ν` slaves config and mass | a config or mass axis beside it is a same-block collision, `Coupling`'s flag; the slice can't pin those blocks |
| 8 | Axes in one block, generic | `(θ, α)`, `(γ₀, θ)`, `(U, r₀₂/r₀₁)` | independent except on a curve where the Jacobian drops rank: numerical, and the curve is the seam |
| 9 | Two momentum axes under a hold | the swept plane contains `Jr` (hold `L_z`), or passes through a point with `δv = 0`: rest or a rigid rotation (hold `E`, alone or with `L_z`) | `ray-degenerate` where it does; a label (§ 5.4) |
| 10 | Region D's edge | eq. 8 couples `ξ` and `η` | not a product domain: § 8.4 |
| 11 | Mass axis and a shape held | `m` moves `U` and the geometric shape | the axis states what it holds (§ 3) |
| 12 | Raw `z_μ` axis | softmax couples all three masses | contract Part 4: the line is "identical except one mass logit" |
| 13 | Hold `E` and hold `L_z` | `2K* ≥ L*²`, `K* = E* − U` | a feasibility curve across the chart; a label per pixel |
| 14 | Interior angles at a collinear point | `γ₀ = γ₁ = 0` is every point between the two bodies | the angle pair can't tell collinear shapes apart: that corner is `ray-degenerate`, and a label says so |
| 15 | Any `gauge` axis against the paper's quantity | `I = 1` rescales per pixel | a row of `E` here is not a row of a paper's `E`: § 8.6 |
| 16 | Frozen hidden coordinates | six are frozen, each read as fixed | only `gauge` axes change meaning under a slice |
| 17 | Splits of `K` and `L_z` (§ 3.1) | `K = K_ρ + K_λ = K_dil + K_rot + K_shape`; `L_z = L_ρ + L_λ` | any terms that fix the sum, or sum and all but one: refused, `SameQuantity` |
| 18 | `L_z`, `E`, `L²E`; `L_ρ`, `L_z`, the inner share | each is two DOF of the third | any two fix the third: refused |
| 19 | Signed area, elongation | `I₁I₂ = n_w²/4` at `I = 1` | one quantity: refused. At a frozen mass, likewise with `n_w` |
| 20 | Body 2's speed or heading, `‖p_λ‖` or `∠p_λ`; `γ_ρ`, `∠p_ρ`; `h`, `K_dil`; `K_rot`, `L_z`; `v₂/v_esc`, `‖p_λ‖` at a frozen shape | aliases | refused |
| 21 | `h` and `L_z` under hold `E` | `r` and `Jr` are orthogonal, so independent, but `K_dil + K_rot ≤ K` | feasible region, tagged, as row 3 |
| 22 | Polar angles about the COM at a collinear shape | a line of shapes on one point | `ray-degenerate`, a seam |

---

## 6. Refusals and labels

**Any axis may be paired with any other, of any kind** (§1.3's mixed-axis rule, extended). The list below is closed.

### 6.1 Refusals: genuine contradictions only

Each is a variant of `chart_authoring::validate::Refusal`, in code, with a named reason.

| Refusal (new unless marked) | The contradiction | Example |
|---|---|---|
| `AxisAndHold { axis, quantity }` | an axis and a constraint on one quantity | `L_z` axis + hold `L_z`; `E`, `K`, `KE` or `Q` axis + hold `E` (generalises R-25) |
| `SameQuantity { a, b }` | two axes fix one quantity | `(K, KE)`; `(E, K)`, `(E, Q)` or `(K, Q)` at a frozen configuration; `(α, ‖λ‖/‖ρ‖)`; `(m_b/m_a, m_a/m_b)`; § 5.5 rows 17 to 20 |
| `NoJointInverse { a, b }` | the pair has no inverse | either axis's class is `none` |
| `SizeOnly { axis }` | it would be constant | `I`, `R` |
| `HoldAgainstConvention { hold, convention }` | **not settled: RQ-NEW-11.** A rest chart under hold `L*` is a well-defined family, each pixel `(L*/I)·Jr`, and reachable anyway through the latent chart at `z_q = 0`, so this is no contradiction in the states; it is the chart's identity | hold `L_z` on a chart declared at rest, by a declared flag: the mass simplex, the Euclid plane, `Φ_strip,δm`, region D's exact-rest arm |
| `View`, existing | R-25, R-670 | a `forbids_energy_normalisation` chart + any `E*`; `ε ≠ 0` + any `E*` |
| `NoWarpWritten`, TASK-M2-37's | no warp written (R-471 H2, R-506 W2) | `PE` |
| `InvariantNotDownstream`, `UnderDetermined`, `DoubleWritten`, `DependentDirections`, `NonFiniteDirection`, `NoSweptAxis`, `Flag`, existing | contract Part 5 | unchanged |

**`UnderDetermined`, `InvariantNotDownstream` and `NoWarpWritten` are not contradictions in the order's sense**, but they
are the contract's check and R-506's words, and those win. In the builder the first can't be reached, since the slice
always fills the DOF an axis leaves.

A chart that holds `L_z` by construction, `(ν, K)` and its strips, is not refused: hold `L_z` there **is** its held value.
`Q` and `K` at a *moving* configuration are still one quantity under hold `E`, since `K = E − U`.

Two axes fixing one quantity is decided by the registry's table, and a test proves the table complete: it estimates the
pair's rank on a grid, and fails any pair of rank below 2 everywhere that the table does not refuse, and any refused pair
of rank 2 (§ 12).

### 6.2 Labels: reduced capability, never a refusal

| Label | Meaning |
|---|---|
| `live` | uniform affine, or a baked entry |
| `CPU batch: not baked` | closed-form, past the cap (§ 11) or not in the baked set |
| `CPU batch: numerical` | a numerical inverse: its branch rule and seam shown |
| `ray-degenerate` | a line of pixels is one system; areas are not system fractions (contract Part 5) |
| `1-swept family` | an annotation axis (§ 10.1) |
| `measure: chart area` | area is not Liouville measure (contract § "Links carry a measure") |
| `branch: <rule>`, `seam` | § 7.3 |
| `axis reads before the hold` | § 5.4, a hidden `p_fixed` or `p_λ` tilt target included |
| `one orientation` | a chart reaching `η ≥ 0` only, as the third-body pair and the angle pair do (the shape sphere's whole-sphere rule, R-544, stays that chart's) |
| `paper scale differs` | a `gauge` axis at `I = 1` (§ 8.6), or a held `L*` or `E*` on region D (§ 5.3) |

**R-506 W1 unchanged.** A chart with any linear axis logs no decoder-bug warning; a chart whose physical axes are all warped
does. A chart with a mask or a seam logs none on the pixels the mask or seam explains.

### 6.3 The Domain preview shows why

§G7's preview gains one line under the hatch. A refused pair shows its reason's name and text, with no quick render. A
labelled pair shows its label, and a numerical axis its seam, drawn. It reads the `Refusal` and the label, so nothing
is written twice.

---

## 7. Lowering: three ways a pair runs

| Way | Which pairs | What runs | Navigation |
|---|---|---|---|
| **Uniform affine** | both axes latent directions | `Latent`, its `z₀, q₁, q₂` | live, uniforms only |
| **Baked entry** | the gallery's, and the pairs inside the cap | a `#[spirv(compute)]` entry, lowering Part 4 | live, uniforms only |
| **CPU batch** | everything else | f64 on the CPU, each start rounded once to f32, the IC buffer filled, then the integrator | not live: a re-render per view, labelled |

### 7.1 What runs on the GPU

`axis kinds` never reach the kernel, and nothing dispatches on them (REQ-CHART-004). A `DecodeVariant` wrapper, as in
`stage::decode`, is what a baked pair is. The composed chart's `resolve` looks its `(H, V, M)` up in the baked table; a
hit returns the entry, a miss returns `CpuBatch`.

### 7.2 CPU batch

`stage::decode` writes each sample's `(m, r, p)` to a buffer of its own, "the stage's only output to the integrator side".
So the integrator already reads a buffer it does not know the origin of, and CPU batch writes the same words
(`write_ic`'s layout), as R-669's Path B batch does for BZ-1C's starts.

**Core design seam 3 says** "*Not* a pre-decoded IC buffer — pan/zoom/tilt change every pixel's IC every frame, so precompute
is impossible at scale". That reason holds for a live view, and a batch view isn't one: the label says so, and a gesture
never reaches it. The owner's "CPU batch only" is seam 3's exception: **door** RQ-NEW-8.

### 7.3 A numerical inverse: branch and seam

Quads resolve in any order, so a solve can't continue from its neighbour. **A branch rule is a pure function of the pixel.**
So a rule is continuous only away from where branches fold, and **a rule continuous everywhere can't be promised**: the
figure's Tier 2c shows why. Two triangles share an `(area, perimeter)`, a chart must pick one, and where they merge, the
rule's choice flips. That curve is the seam.

- **Solve:** Newton in f64 from a fixed list of starts, the pixel's own coordinates as the only input.
- **Rule:** declared per registered inverse, and recorded on the sample (`branch` in the IC record), with R-82's pattern
  for a tie: a fixed `δ`, and within it, one named choice.
- **Seam:** the set where the solver's Jacobian changes sign, or where `branch` differs across a pixel edge. It is drawn on
  the Domain preview and offered as an overlay.
- **Failure:** no root is `decode_failed`, blank (R-470), the label naming the solver.
- **Round trip:** `forward(inverse(x)) = x` at every sample, within the tolerance, or the sample is `decode_failed`.

---

## 8. First customer: region D

### 8.1 The definition

From dd_validation_orbits §1.6, the port of R-592; the M9 targets name it through "the Anosova physical-frame entry"
(replication § 2 and its rows), and the lowering appendix gives it a row.

- Bodies at `A₁ = (−½, 0)`, `A₂ = (+½, 0)`, `A₃ = (ξ, η)`, equal masses `m = 1`.
- **Region D** (eq. 8): `ξ ≥ 0`, `η ≥ 0`, `(ξ + ½)² + η² ≤ 1`. A point outside it is not a region-D start.
- Velocities, one of two: **exact rest**; or **eq. 3**, `ẋ₁ = ẋ₂ = +5·10⁻⁵`, `ẋ₃ = −10⁻⁴`, every `ẏᵢ = 0`, near-rest.
- Units: `G = 6.67·10⁻⁸` in the papers' units, length 1 as stated there. Energy, eq. 4.

The whole of D is one fundamental domain under relabelling and the mirror: its three edges are the isosceles line `ξ = 0`,
the collinear line `η = 0`, and the arc `r₀₂ = r₀₁`. Its corners: `(0, 0)`, `(½, 0)` a binary collision of bodies 1 and 2,
and `(0, √3/2)` the equilateral triangle.

**This definition is the door: RQ-NEW-3.** Four things it needs ruled are in § 8.4 to § 8.6.

### 8.2 The closed form

`A₁, A₂, A₃` are bodies 0, 1, 2. In dd_decoder §3.2's terms `ρ = A₂ − A₁ = (1, 0)` and `λ` runs from the inner pair's
centre of mass to `A₃`. That centre sits at `(d, 0)`, not at the origin, unless `m₀ = m₁`:

```
d = (m₁ − m₀)/(2 M₀₁)                        0 at equal masses
λ = (ξ − d, η)
ℓ² = μ_ρ + μ_λ ((ξ − d)² + η²)               I before the gauge, = 1/6 + (2/9)(ξ² + η²) at equal masses
ρ̃ = √μ_ρ (1, 0)/ℓ          λ̃ = √μ_λ (ξ − d, η)/ℓ
```

The plane stays the literal one, `A₃`'s position, so the geometric axes of § 8.3 keep their meaning at every mass.

That is all of it: no `atan2`, no pole at `(0, 0)`, the canonical frame by construction, `C` a no-op, the mirror not
taken at `η = 0` (the deadband, R-82). The lowering appendix's "physical (m, r, p) construction → canonicalise entry" is
this, and `ShapeSphere`'s explicit-configuration path carries it.

**It is the shape sphere, stereographically.** With §3.1's `n = (u, v, w)`:

```
(ξ − d, η) = √(μ_ρ/μ_λ)·(n_v, n_w)/(1 + n_u)
```

the projection from `n = (−1, 0, 0)`, §3.1's collision of bodies 0 and 1. So the plane is conformal to the mass-weighted
sphere, and the figure's Tier 1 circles are circles because of it. At equal masses D's apex is the pole `L⁺`, and the
collinear row `η = 0` is the equator. Frames of other papers are affine maps of this plane. Checked numerically at random
masses, to 6·10⁻¹³, against the identity in `λ` (`orch/scratch/chartcomp/regiond_check.py`, not committed; a tested function
in TASK-NEW-3). That check took `λ = (ξ, η)` as given, so it could not see `d`: the offset comes from dd_decoder §3.2, not from it.

### 8.3 The pairs it carries

Each inverts into `(ξ, η)`, in the frame above, so one baked entry could serve all three, its warp word choosing:

```
angles:       η = 1/(cot γ₀ + cot γ₁)           ξ = −½ + η cot γ₀
side ratios:  ξ = (a² − b²)/2                   η = √(a² − (ξ + ½)²)     a = r₀₂/r₀₁, b = r₁₂/r₀₁
```

Whether they share one entry is V5. Each is `forward(inverse(x)) = x` (§ 12), and R-798's remap of PA-1's side
ratios onto the shape sphere is this line followed by § 8.2's: one source, as REQ-CHART-093 asks. **R-798 keeps "There is
no `SideRatio` chart"**; whether a side-ratio axis in the builder is one is **door** RQ-NEW-2, and PA-1's route is not
touched either way.

### 8.4 What the chart is: the domain

D is not a product of two ranges. Three ways to draw it:

- **(a) A mask.** Axes `ξ ∈ [0, ½]`, `η ∈ [0, √3/2]`, linear; `validate` refuses a pixel outside eq. 8 as a named layer-2
  constraint, blank, out of the count (R-470, R-408), no warning (R-506 W1). The picture is the papers', and pixels are
  uniform in `(ξ, η)`, the sampling measure region-D statistics assume.
- **(b) The plane.** No mask: the outside is real systems, relabelled copies, and D is a contour. Statistics then filter.
- **(c) A warp onto the square,** polar about `A₁`, `v ↦ r ∈ [1/(2 cos γ), 1]`, every pixel inside D. Non-uniform measure, the
  apex a ray-degenerate edge.

**Recommended (a), as a preset of the third-body pair**, whose own axes are `ξ` bounded and `η` bounded. The lowering
appendix's "region D bounds" uniform is the mask. **Door** RQ-NEW-3.

A mask has a cost the ruling must meet: contract Part 3 says "**Inside `[0,1]²` every gallery chart is valid by
construction** … so a pixel hatched there is a decoder bug", and REQ-CHART-058 sweeps for it. R-554 made the same exception
for the exponential map past `r = π`: "the domain's end by design", blank, unwarned. Option (a) extends that exception by
name. A masked pixel is then the domain's end, out of R-408's count and total, not "forbidden". R-506's "warped to its
feasible range by default" is met trivially: every `(ξ, η)` is a system, so the feasible range is the whole range.

**Extension.** Both bounded, the default: past `η = 0` the mirror, past `ξ = 0` the relabelling, and neither is a
continuation anybody has defined. R-407's rule: "Nothing ever invents a continuation". The plane chart without the mask
declares nothing more.

### 8.5 Which M9 target needs which part

The "Anosova entry" of the replication rows is three things. Point import exists; the plane does not.

| Part | Exists | Targets |
|---|---|---|
| **Points**: a published start through the import | yes, TASK-M2-20 | LL-1, LL-2, LL-3 (TASK-M9-06, -07), TM-1 (-09), AA-6 (-10), AA-2, AA-3, AA-5, AA-7, AN-1 (-19), BR-1 (-22) |
| **Samples**: many points in D, drawn | the membership test, `in_region_d` | Q-1 (-18), Q-2 |
| **The plane**: a pixel grid over D | **no** | L-1, L-2, L-3, L-4, L-5 (-20, -26, -31), V-9, MT-1 (-21), GG-1's 1/64 grid (-22), TS-1, V-1/3/7 (-28), and TASK-M4-06's golden (REQ-RENDER-082) |

Velocities: exact rest for AA-6 and every free-fall map; eq. 3 for AA-2 and AA-7, and AA-3 and AA-5 read paper I, whose runs
are eq. 3's, though their rows do not say so (flagged for RQ-NEW-3). Masses: equal, but AN-1 and the LL unequal sets at a mass
point: the plane takes the masses as frozen.

**Frames.** Lehto's `(0, 0), (1, 0)` is a translation. Tanikawa's D11 mirror, Lehto's lens, Breen's frame and the book's ×2
scale are each a target's tested function, already in TASK-M9-20, -22 and -28's deliverables; **the corpus does not write
them out**, and this document does not guess them (R-72 keeps them with those tasks). Each ends in `(ξ, η)` or in a
point, so the plane does not depend on them.

### 8.6 The momentum, and the rescale per pixel

**Rest** is `q = 0`, exact. **Eq. 3** is not uniform at `I = 1`: the import's rescale makes it depend on the pixel.
With `p̂ᵢ = (mᵢ/M)·vᵢ/√(GM)` (the import's `unit_state`), then `C`'s `pᵢ ← √ℓ·pᵢ`:

```
q(ξ, η) = √ℓ(ξ, η)·q̄      q̄ = the Jacobi form of p̂      for eq. 3, q̄ = (0, 0, −10⁻⁴/(3√(3G)), 0) = (0, 0, −0.0745, 0)
```

`q̄` is a uniform, at most four floats. The sum matches eq. 3 and eq. 4 to rounding (checked as above: relative energy
1.2·10⁻¹⁴). This is a residual convention on the momentum block, which reads the configuration: the pattern of kind 3.

**Every `gauge` quantity carries that `ℓ`, and so does time:**

```
E_paper = G M² E_c/ℓ      L_paper = √G M^{3/2} √ℓ L_c      t_paper = ℓ^{3/2} t_c/√(GM)
```

So over D, `ℓ` runs from `0.408` to `0.577`, and **one sim-key `T` is a horizon in paper time that varies by a factor of
1.68 across the chart.** A fixed paper horizon needs the run to reach `T_c = T_p √(GM)/ℓ_min^{3/2}` and the comparison to cut
each pixel at its own. The scored comparison reads `ℓ` as a tested function (rule 11); nothing in the kernel moves.
The horizon in M9 is a replication question, flagged in RQ-NEW-3.

### 8.7 Where it lands

Before TASK-M4-06's golden. TASK-NEW-3 builds the pair and its entry; TASK-M4-06 depends on it and lowers its row; and
every plane target above gains TASK-NEW-3. It is not "built ahead of" a target (R-594 item 5): its first customer is
M4's own golden.

---

## 9. Then virial

Built by TASK-M2-37: `kernel::chart::physical::AxisVirial`, and chart_reference §2.1's paragraph, which warps `Q = 2K/|U|`.
REQ-CHART-070's debt is paid there.

**What the axis reads.** At `L_z = 0` it is the virial ratio. At a held `L_z ≠ 0` it is not: its line is
`K* = L_z²/2I + Q(t)·|U|/2`, R-708's additive floor, so, as chart_reference §2.1 says, "The pixel's own virial ratio
`2K*/|U|` is `Q(t) + L_z²/(I·|U|)`". The axis reads the excess above the rotational floor, `Q(t) = 2(K − L_z²/2I)/|U|`, and
the state's own ratio is larger by `Q_min(U) = L_z²/(I|U|)`, which varies with `U` across a row. So § 3's `K* = Q·|U|/2`
is the `L_z = 0` line and the registry carries both; the label at `L_z ≠ 0` is `Q above the floor`, with the state's own
`Q` in the readout. Whether the floor should sit under the rise only is **RQ-700**.

What is missing is its descriptor. RQ-682 (open, in the queue): "Which of the four axis kinds a virial-ratio axis is:
R-471 H3 says it is not kind 3", and "No kind fits, and the set is closed". This document needs option (a) of RQ-682: kind 3,
`InvariantQuantity` gaining `Q`, solving into the momenta, reading config and mass. H3's sentence is about B1's extension,
which R-506 superseded. RQ-700 (where a virial bottom row sits, `Q₀ > 0` at a held `L_z ≠ 0`) stays open. It blocks nothing today only because
RQ-682 keeps the axis off every builder chart; once RQ-682 lets the axis in, RQ-700 decides what a row of it means, so
TASK-NEW-5 waits on it for `L_z ≠ 0`, and with `(a)` applied the label above says so.

The second pair is `(<configuration axis>, Q)`, a `MixedEnergy`, baked as TASK-NEW-5, for the virial pair the owner names.
`Q` at exact-rest region D is 0, and `Q` varying over D is the eq. 3 family's own reading. TS-2's "virial ratio 0 to 0.7"
stays "not scored: no fixed-k velocity chart" (R-669).

---

## 10. The gaps, each as a builder option

| Gap (item 4) | Covered by | Task |
|---|---|---|
| 1D charts in the GUI | § 10.1 | TASK-NEW-11 |
| Curved slices off the shape sphere | § 5.2 | TASK-NEW-7 |
| Hold `L_z` on any chart | § 5.3 | TASK-NEW-4 |
| Polar and log as general options | § 4, § 5.1 | TASK-NEW-6 |
| Tier 1 overlays of any primitive | § 10.2 | TASK-NEW-9 |

### 10.1 1D charts

The order says "today only the TR-1 engine scan". That scan isn't built: `TASK-M9-23` describes it, R-669 parks it with
"no binary-plus-single chart", and no code has it. The nearest existing thing is the Euclid plane, a 1-swept chart with an
annotation axis (contract Part 5). A 1D chart is that: one swept axis, the other annotation-only, well-posed as 1 + 7,
labelled `1-swept family`, statistics per value along the axis only. It lowers as its 2D twin does, the annotation
coordinate unread, so a pair is not used up. The GUI draws it as a strip, and a line of the chosen field beside it.

### 10.2 Tier 1 overlays

A contour of any registered primitive is its `forward`, in the line system (R-476, TASK-M7-43), display-only and never
scored (replication rule 6: "the conversion is a tested function, not a figure overlay"). §4.1's variables gain the
interior angles and the pair binding energies, and its functions gain `acos`, which the stain's expression syntax needs too
(R-509 Y1). TASK-M7-43 is unmerged, so its list grows by a note, and the rest is TASK-NEW-9.

An overlay of a primitive evaluates on the decode, so every registered forward needs f32 evaluation on the GPU or the CPU
fallback: §4.2 already says "a GPU pass on the shared decode, with a CPU fallback at reduced resolution". The Domain
preview draws the same contours.

---

## 11. The axis cap

**The number here, 4, is provisional** until the Windows Vulkan run is in (below). No primitive of § 3.1 takes a slot: they
stay CPU batch with the label, and `SuvakovPlane` is R-669's own.

The order asks for a number with its measured cost (item 5). The bound's duty is REQ-PERF-007's: "bounded to the
active/plausible variant set, not the full cross-product". The value is a one-way door, RQ-NEW-5.

**The criterion, written before the first measurement.** `P` is the baked pairs added to the kernel of main 9188686f,
each its own compute entry in the one module. The cap passes all of:

| | Test | Limit | Why |
|---|---|---|---|
| C1 | `cargo xtask build-kernel` after a kernel edit | `T(P) − T(0)` ≤ 30 s and `T(P)/T(0)` ≤ 1.25 | it runs on every kernel edit and in CI |
| C2 | size of the WGSL module | `wgsl(P)/wgsl(0)` ≤ 1.25 | every pipeline parses the whole module |
| C3 | the eager set, 12 decode entries plus `P`, one after another on this Mac; and one creation | ≤ 2 s; ≤ 1.25 × baseline | "Chart switching must be instant" (lowering Part 4); the 2 s is a design number, for veto |
| C4 | naga validates; a pair's parameters fit `DecodeUniforms.chart` | 24 floats | the uniform's size is a layout |
| C5 | C1 to C3 again under the multiplier the plan implies | as C1 to C3 | `(chart × occupant × tier × links)`, lowering Part 4 |

The cap is the largest `P` passing, not above what the known customers need. The stand-in pair is at least as heavy as
region D's: the mass-weighted Jacobi from `(ξ, η)` with `sqrt`, `atan`, `atan2` and the near-rest momentum, each entry a
distinct monomorphisation. Each time is the median of three.

**What was measured.** On an Apple M3 Pro, Metal, under the queue's other work (load average 4 to 13 for this table):

| `P` | build (s) | SPIR-V (bytes) | WGSL (bytes) | naga (ms) | one creation (ms) | eager set (ms) |
|---|---|---:|---:|---|---|---|
| 0 | 10.1 | 766,248 | 1,004,550 | 38.9 | 83 | 1012 |
| 1 | 11.3 | 784,332 | 1,030,342 | 43.8 | 89 | 1790 |
| 4 | 11.2 | 838,676 | 1,107,749 | 41.0 | 85 | 1993 |
| 8 | 13.4 | 911,140 | 1,211,100 | 44.5 | 91 | 2656 |
| 16 | 14.9 | 1,056,100 | 1,419,411 | 50.0 | 117 | 4361 |
| 16, cheapest pair (`Mixed<H, V>`) | 21.9 | 1,029,104 | 1,381,194 | 52.4 | 114 | 4405 |
| 4, integrator inlined, one pair × 4 occupants | 18.9 | 997,940 | 1,441,010 | 73.3 | 172 | 1940 warm |

A pair is about 18 KB of SPIR-V and 26 KB of WGSL. The cheapest pair is about 23 KB of WGSL, so the size is the inlined
decoder and not the pair's own maths: a cheaper pair doesn't buy a higher cap.

A second run, built to force Metal's first compile (load average 21 to 42, so its times are for comparison between its
own passes only): eager set 12: 998 to 1116 ms; eager set 16: first pass 4424 ms, then 1610 and 1494; eager set 20: first
pass 3459, then 2218 and 2213; four integrator-inlined entries: first pass 3323, then 1942 and 1931, and a first sample
of 8817 at load 13. The first compile of a pair entry is thus between about 150 and 700 ms, and of an integrator-inlined
entry between about 350 and 1900; the spread is the load, and those are ranges, not estimates of a mean.

**Against the criterion:**

| `P` | C1 ratio (limit 1.25) | C2 ratio | C3 eager (limit 2 s) | C3 one creation | C4 |
|---|---|---|---|---|---|
| 4 | 1.11 pass | 1.10 pass | 1.99 s, a bare pass; 1.5 to 1.6 warm in the second run | 1.02 pass | pass |
| 8 | 1.33 **fail** | 1.21 pass | 2.66 s **fail** | 1.10 pass | pass |
| 16 | 1.48 **fail** | 1.41 **fail** | 4.36 s **fail** | 1.40 **fail** | pass |

So **the owner's "about 4" passes, and 8 does not.** `P` of 5, 6 and 7 weren't built; a line through the table puts C3's
eager limit at about 4 and C1's at about 6. The 30 s test in C1 passes throughout and never decides.

**C4.** Region D's pair needs at most 8 frozen floats, 4 ranges, 3 masses and `q̄`, which fit the 24; naga validated every
arm. `DecodeUniforms` needs no new field for a pair of that weight.

**C5: the multiplier decides the design.** One pair, integrator inlined, across 4 occupants fails C2 outright (1.43): an
integrator-inlined entry costs about 109 KB of WGSL and 58 KB of SPIR-V, four times a decode-only one, so C2's budget of
251 KB is spent after two entries. The cap of 4 therefore holds **only because decode is its own entry** (V2), writing the
initial conditions the integrator reads, which keeps chart and occupant additive. TASK-M4-06's "chart × link × occupant ×
FTLE" has to stay additive for a pair, or no useful cap exists. The cap counts **entries**, not pairs: a pair under two
link sets is two, and a tilt of a baked pair (`Tilted<C, P>`) baked as its own entry is one more. A tilted pair past the
cap runs as CPU batch, with the label.

**Start-up objects are not this design's.** The kernel creates 12 compute pipeline objects at start-up against the
lowering contract's "~10". That is issue #427, filed on its own at the owner's word, with no ruling yet. The facts this
design needs: eager pairs are what fail C3, so pairs are created on first use (V7), and if #427 changes the eager set V7
is re-read against it. The customers need four, and the re-measure below allows no more:

| Entry | Customer |
|---|---|
| 1 | region D's third-body pair (§ 8), which the angle and side-ratio axes share (V5) |
| 2 | the virial pair (§ 9) |
| 3 | R-669's `SuvakovPlane`, if RQ-NEW-1 leaves it baked as one chart. It is ruled as its own chart, so its entry is in the module whatever the cap says, and counting it here is the cautious reading |
| 4 | one spare, for the first pair a replication target wants live; `stage_blend` takes it if live paths are adopted (§ 17.12) |

**Proposed, provisionally: 4 baked entries beyond the gallery, created lazily, decode-only.** Everything past it is `CPU
batch: not baked`, a label and never a refusal (§ 6). **4 is provisional:** it rests on one Mac, on Metal. The owner has asked
for the Windows runner's Vulkan result on an RTX 2080 SUPER before it's ruled, and that result is owed.

**The re-measure, with the real pair.** Criterion written first (`orch/scratch/chartcomp/cap2-criterion.md`): C1 to C4 as above,
unchanged, plus C3′: the 12 gallery pipelines at `P` within 1.25 × at `P = 0`, since every pipeline parses the whole module,
so baked pairs cost start-up even when none is created. The pair is region D's own, `d` included, eq. 8's mask and eq. 3's
momentum, baked in distinct copies, since the others aren't built. This Mac, Metal, arms interleaved three times so load
drift falls on all of them, load average 12 to 28. Sizes are exact. A first pass with arms in a row gave times too noisy
to read, and had a slip in the eager set, so it isn't used. Times below are the least of the warm passes, the least
contaminated by other jobs; the build is the median of three.

| `P` | SPIR-V | WGSL | WGSL ratio | build (s) | 12 gallery (ms) | one creation, ratio | eager `12 + P` (ms) | a pair's first creation (ms) |
|---|---:|---:|---|---|---|---|---|---|
| 0 | 766,248 | 1,004,550 | | 21.5 | 958 | | 958 | |
| 1 | 813,344 | 1,064,947 | 1.06 | 20.1 | 1019 | 1.06 | 1101 | 325 to 885 |
| 4 | 954,368 | 1,246,645 | **1.24** | 22.4 | 1178 | **1.23** | 1591 | 239 to 255 |
| 8 | 1,142,400 | 1,489,065 | 1.48 **fail** | 22.4 | 1410 **fail** | 1.47 **fail** | 2345 **fail** | 243 to 1006 |

**The real pair costs more than twice the stand-in**: 60 KB of WGSL and 47 KB of SPIR-V each, against 26 and 18. Module tests
are exact, so **`P = 4` passes C2 at 1.24 and a fifth pair would fail it at 1.30.** Four is the ceiling, not a comfortable
margin, and C1's limit, read in the first run as about 6, isn't what binds. C1 passes at every `P` on this run, with a spread
from 14 to 38 s between repeats, so that ratio can't be read finely here. C3 passes at 1.6 s. C3′ passes at 1.23. Each pair adds
about 56 ms to start-up across the gallery, built or not.

**So 4 still stands, more tightly, and stays provisional.** The Windows arm (Vulkan, RTX 2080 SUPER) is owed, on the same
commit, under the same criterion: if the machines disagree, the lower number stands in the proposal. `stage_blend`, if
live paths are adopted (§ 17.12), is one more entry in the same module, and at about a pair's size it takes the fourth slot.

**What this can't say.** One Mac. Times are under the queue's other jobs, whose load ranged from 4 to 42; sizes are exact
and times aren't, so C1 and C3's small ratios carry that noise, and `P` = 4's eager pass is a bare one. Metal alone:
nothing here says what Vulkan, DX12 or WebGPU pay; wgpu and naga's parse is shared, the driver's compile isn't, and on
Metal the disk cache hides the first compile unless defeated, which the second run did with salted entries. A pipeline's
GPU run time wasn't measured, nor was CI's own build time. The first run's stand-in was a heavy region-D pair; the re-measure
uses the real pair, in copies, since the other three aren't built. The measurement branch is a throwaway, not in the repo.

---

## 12. Tests that can fail

Each with a control (R-176), named here and owned by a task.

| Test | Asserts | Control | Task |
|---|---|---|---|
| `presets_bitwise` | every `kernel::chart::REGISTRY` chart, each gallery preset and a saved chart of each kind (latent oblique, curved with tilt) decodes bit-for-bit to a digest fixture written **before** the first change; `SimConfig`'s canonical form unchanged | one mantissa bit flipped in a digest; a default field serialised | TASK-NEW-2 |
| `closed_inverse_round_trip` | each closed row, each pair: `forward(inverse(x)) = x` over its domain, f64 and f32 at their tolerances | a row with a sign flipped in its inverse | TASK-NEW-3 |
| `size_only_refused` | `I`, `R`, `ℓ` refused by name; the sampled check of § 3 passes | a registered row constant on decodes but declared `free` fails | TASK-NEW-2 |
| `pair_table_complete` | § 6.1's rank test agrees with the table in both directions | a table row removed; a rank-2 pair added to it | TASK-NEW-2 |
| `constraint_axis_refused` | every row of § 6.1, in code, by name; `Some(0)` included; the view through `resolve` is unreachable | the flag cleared on `(L_z, E)` lets a chart through and the arm fails | TASK-NEW-4 |
| `numerical_branch_seam` | a numerical pair reports its rule, per-pixel branch and seam; the rule is a pure function: the same pixel in two orders gives one bit pattern | a rule reading a neighbour fails; the area-perimeter pair with no rule | TASK-NEW-8 |
| `region_d_plane` | every Table 1 row's marked pixel decodes to the import's state; eq. 8's edges; eq. 3 momenta at `√ℓ`; `I = 1` | body 0 and 1 swapped; `q` not scaled by `√ℓ` | TASK-NEW-3 |
| `stereographic_identity` | § 8.2's line at random masses, against a state built from `A₁, A₂, A₃` by dd_decoder §3.2 | `μ_ρ/μ_λ` inverted; `d` dropped | TASK-NEW-3 |
| `hold_lz` | `L_z = L*` to tolerance on every chart allowing it, a near-rigid fixture included (`‖δv‖/‖v‖` from 10⁻¹ to 10⁻⁷ at f32); `Σp = 0`; both held: `E*` too; off: bitwise | `η_E` applied alone with both on fails `L_z`; one projection fails the near-rigid fixture | TASK-NEW-4 |
| `hold_outcomes` | § 5.3's six outcomes in order, each on an exact fixture; a tagged pixel's momenta unchanged; a rest pixel on the floor becomes `(L*/I)·Jr` | the order swapped at 2 and 3 | TASK-NEW-4 |
| `hold_rank` | a momentum plane containing `Jr`, and one through rest and through a rigid rotation under hold `E`, each report `ray-degenerate`; a generic plane doesn't | the label removed | TASK-NEW-4 |
| `hold_encode` | Part 6 step 4 reads the held decode; a state off `{L_z = L*}` gets the stated outcome | step 4 on the unheld decode lands on a wrong pixel | TASK-NEW-4 |
| `baked_cap` | baked pairs in `stage::decode` at or under the ledger's constant | one entry over | TASK-NEW-2 |
| `excluded_primitives` | no row with an inverse names a binary phase, eccentricity, impact parameter or fixed `k` (R-669) | one added | TASK-NEW-2 |
| `prim_round_trip` | each closed row of § 3.1, `forward(inverse(x)) = x`, f64 and f32; Saari's split sums and is orthogonal; `I₁I₂ = n_w²/4`; `h`'s step keeps `L_z`; the aliases equal | a sign flipped in an inverse | TASK-NEW-17 |
| `prim_rejected` | each row of "Rejected as axes" has no axis; `L²E` and Hill's distance label `ε ≠ 0`; Hill's distance blank at `E ≥ 0`; pair table rows 17 to 20 refused | one accepted | TASK-NEW-17 |
| Golden | `cargo xtask golden decode-preset`: region D through the DECODE preset, TASK-M4-06's acceptance, unchanged | as that task's | TASK-M4-06 |

---

## 13. Reversible choices: decided, for veto

Each states its criterion, written first, then the result. Any may be vetoed; each is recorded here and changes no ruling.

| # | Choice | Criterion | Decision | Evidence |
|---|---|---|---|---|
| V1 | A warp's kind: a uniform word, not a baked variant | bitwise on the linear arm; the branch taken before the decode, as `ShapeMap`; TASK-NEW-6 measures decode time, and past +2% it becomes baked | uniform | precedent; the measurement is that task's |
| V2 | Keep decode an entry of its own, apart from the integrator | the variant count must stay additive, § 11 | keep it | § 11's numbers |
| V3 | Region D by explicit configuration, not `atan2` plus the link | the lower worst f32 error against f64 over D; a tie goes to the lowering appendix's row | explicit | worst `1.28·10⁻⁷` against `2.18·10⁻⁷`, 400,000 samples, `orch/scratch/chartcomp/regiond_f32.py` |
| V4 | The registry on the CPU, `chart_authoring` | `kernel` must not learn a kind (REQ-CHART-004) | CPU | by construction |
| V5 | Angles and side ratios: their own entries, or a warp word on the third-body entry | fewest baked entries, with `closed_inverse_round_trip` green at f32 | decided by TASK-NEW-3's test | not measured here |
| V6 | Pair-table completeness by a rank test on a grid | a table alone can drift; a test finds a missing row | table and test | § 12 |
| V7 | Pair pipelines created on first use, not eager at start-up | § 11's C3 | on first use | § 11; re-read if #427 changes the eager set |
| V8 | A 1D chart is an annotation axis | no new lowering; statistics rule already written | annotation | contract Part 5 |
| V9 | Hold `L_z` solves by the rigid rotation, not in the hidden momentum directions | the least velocity change `‖Δv‖_m`; closed form; independent of which axes are swept | rigid | § 5.3; the physics note |
| V10 | A chart holding `L_z` by construction reads the hold as its held value | no double write; R-720's by-value | as such | § 6.1 |
| V11 | Numerical solves: Newton from a fixed start list, no continuation | a pure function of the pixel | as such | § 7.3 |
| V12 | A morph into a polar map takes the angle's orientation-keeping sense | fewest folded pixels at `s = ½` | reversed `θ` | 13% against 32%, § 17.6 |
| V13 | Orthogonal planes pair `q₁ → q₁′`, `q₂ → q₂′`; the in-plane rotation eases on the same `s` | deterministic per segment; ends bit for bit | as such | § 17.7 |
| V14 | Alignment by Procrustes on the 16 × 16 probe grid | a pure function of `(A, B)`, recorded; travel not worse than none | as such | § 17.10; not measured on nonlinear ends |
| V15 | A frame is `degenerate` with a probe below `10⁻³` of its median Gram factor | flags sphere to plane and flat to polar, and no affine rotation | `10⁻³` | a design value; TASK-NEW-13 tests it on those fixtures |
| V16 | Export doesn't cache each end's blocks per pixel | cache only if the CPU side passes 1 s | no cache | 0.11 s, § 17.12 |
| V17 | Re-timing: 33 knots, 16 × 16 probes, on the decoded blocks | spread at most 1.25 on the saturated fixture | as such | 1.12, § 17.9 |
| V18 | Live target: 4 ms for one coarse cover's chart side | written before measuring | 4 ms; missed on the stand-in, about 15 ms | § 17.12 |
| V19 | Re-centre once per end and segment, not per frame, unless the anchor moves | a pure function, so a repeat gives the same bits | once | § 17.8 |
| V20 | A saved mid frame reopens as a record, with no tilt or slice | it has no hidden coordinate; labelled | as such | § 17.5 |
| V21 | A pixel blank at either end is blank mid-path | no invented state (R-470) | as such | § 17.11 |
| V22 | The dilation axis is signed `h`, not `K_dil` | an axis has to invert | `h` | `K_dil` loses the sign, § 3.1 |
| V23 | Aliases are one row, not two: body 2, `γ_ρ`, `K_rot`, `K_dil`, elongation, midpoint angle | one quantity, one row; `pair_table_complete` agrees | aliased | § 3.1, identities to 10⁻¹³ |
| V24 | Every § 3.1 row is CPU batch | the cap's slots are taken | CPU batch | § 11 |
| V25 | Hill's distance blank at `E ≥ 0`, `L_z = 0`, `ε ≠ 0`, labelled, not refused | undefined there, not a contradiction | label | § 3.1 |

---

## 14. Doors: the rulings items

Each is `RQ-NEW-n`, written out with its file, section, quoted text and options in the queue entry the orchestrator holds.
Nothing below is decided here.

| Item | The door | Recommendation |
|---|---|---|
| RQ-NEW-1 | "A new axis kind" against "a closed set"; R-798's reading: does "no new chart type" reach beyond PA-1? | keep four kinds (agreed in principle by the owner, not a ruling); yes, every new capability is a builder option |
| RQ-NEW-2 | A side-ratio axis in the builder against "There is no `SideRatio` chart" | allow the axis; PA-1 keeps its remap |
| RQ-NEW-3 | Region D's definition: domain (a), (b) or (c); eq. 3 for AA-3 and AA-5; the horizon | (a), outside D blank and uncounted (agreed in principle by the owner, not a ruling); yes; the comparison cuts by pixel |
| RQ-NEW-4 | Hold `L_z`: the physics of § 5.3, and the held half of RQ-701 | (a) amended, as § 5.3, on the physics reviewer's note |
| RQ-NEW-11 | Whether a rest chart refuses hold `L_z`, or labels it "not at rest" | the owner's choice; the physics reviewer leans to a declared flag |
| RQ-NEW-5 | The axis cap | 4, provisional until the Windows Vulkan run is in; § 11. The start-up budget itself is issue #427 |
| RQ-NEW-6 | The virial kind: RQ-682 | RQ-682 (a) |
| RQ-NEW-7 | R-669: a user-built plane that lands on a refused one | it may be built, is never scored, and the row stays |
| RQ-NEW-8 | Seam 3: a CPU-filled IC buffer | an exception for labelled batch views |
| RQ-NEW-9 | R-594 item 5: "Do not build charts ahead of it", against general polar, log and curved options | it governs charts built for a target; the options are the owner's |
| RQ-NEW-10 | Chart paths: does a path type meet the bitwise constraint; export contract Part 3 and REQ-TOOL-075 against morphing; `z` or blocks; the default segment; live | a path type, tilt and zoom as arms; the rules give way; blocks; morph the default for a new segment; R-520 and R-669 stand |
| RQ-NEW-12 | Inner and outer elements, Mardling–Aarseth: axes against R-668's parking and R-669 | forward-only overlays; no axis |
| RQ-NEW-13 | The Saari split and RQ-680's dilation rows | define `h`, RQ-680 (b), forward first |
| RQ-NEW-14 | `v₂/v_esc` and symmetric families against R-669 | overlay until the prescription is read; `SuvakovPlane` stays one M9 task |

**New extension types: none proposed.** Polar's `ϕ` and `r` declare existing types, under R-407 clause 3's physics review.

---

## 15. Plan

Tasks, milestones chosen by what depends on them. `TASK-M4-06` gains TASK-NEW-3 in Depends on.

| Task | Milestone | What | Waits |
|---|---|---|---|
| TASK-NEW-1 | M4 | the docs: contract, reference, lowering, §G7, §4.1; § 5.3's three definitions | RQ-NEW-1 to 5, 8 |
| TASK-NEW-2 | M4 | registry, refusals, labels, bitwise digest, cap test | TASK-NEW-1 |
| TASK-NEW-3 | M4 | the third-body pair, region D's preset, angles and side ratios | TASK-NEW-2 |
| TASK-NEW-4 | M4 | hold `L_z` | TASK-NEW-2, RQ-NEW-4, RQ-NEW-11 |
| TASK-NEW-5 | M4 | the virial pair's descriptor and entry | TASK-NEW-2, RQ-NEW-6; RQ-700 at `L_z ≠ 0` |
| TASK-NEW-6 | M8 | warps, polar and log pair maps | TASK-NEW-2 |
| TASK-NEW-7 | M8 | curved slices: momentum and mass | TASK-NEW-6 |
| TASK-NEW-8 | M8 | CPU batch, numerical inverses | TASK-NEW-2, TASK-M4-06 |
| TASK-NEW-9 | M7 | contours of any primitive | TASK-M7-43, TASK-NEW-2 |
| TASK-NEW-10 | M8 | the builder's panels | TASK-M8-18, TASK-NEW-3, -4, -6, -8 |
| TASK-NEW-11 | M8 | 1D charts | TASK-M8-18, TASK-NEW-2 |
| TASK-NEW-12 | M8 | chart paths in the contracts | RQ-NEW-10, TASK-NEW-1 |
| TASK-NEW-13 | M8 | the path type, the line, export through CPU batch | TASK-NEW-12, TASK-NEW-8, TASK-M8-31 |
| TASK-NEW-14 | M8 | live: the blend entry, the coarse cover | TASK-NEW-13 |
| TASK-NEW-15 | M8 | the Sequence's morph segment | TASK-NEW-13, TASK-M8-77, TASK-M8-78 |
| TASK-NEW-16 | M8 | paths under a lock: the stored anchor | TASK-NEW-13, TASK-M2-23 |
| TASK-NEW-17 | M8 | the primitives of 6 Nov, axes and overlays | TASK-NEW-2, TASK-NEW-6, TASK-NEW-9, RQ-NEW-12 to 14 |

---

## 16. What this does not settle

- Vulkan pipeline costs are owed from the Windows runner; DX12 and WebGPU aren't planned. § 11 measured Metal, on one Mac.
- The `D11`, lens and Breen frames: not in the corpus (§ 8.5).
- Whether a composed chart's linearised deep-zoom decode needs its own derivative: `J_D` on a baked pair follows the shared
  path at M6; a CPU batch view has no deep zoom until a task gives it one.
- § 3.1: the Mardling–Aarseth form, Hill's threshold source and TS-2's prescription are not in the corpus, so those rows
  owe a source. No round trip of § 3.1 is built; identities only.
- Chart paths: § 17's whole-frame time, integration included, a built `stage_blend`, and any nonlinear-end alignment:
  not measured. A faster CPU-only live path is a goal with no task. The linearised decoder's tangent planes wait on M6.
- RQ-701, RQ-700 and RQ-682 stand open in the queue; this document assumes nothing of them but where it says so.
  RQ-682's number needs checking at the id pass.

---

## 17. Chart paths

The owner's consolidated order of 6 Nov: "a view is a point in chart space; every edit is a path." This section takes it
clause by clause, each checked against the code, the corpus and the rulings branches of 4 and 5 Nov, which change nothing
here. Every numeric claim comes from `orch/scratch/chartcomp/measure/morph_check.py` and `path_check.py`, not committed, on a
Python replica of chart_reference §0, unless it says otherwise. All of it is **door** RQ-NEW-10.

**Where the order can't hold as stated:**

1. **Morph is not the general form of tilt and zoom, as one formula.** No single blend reproduces them bitwise (§ 17.3). A
   *path type* does, with tilt and zoom as arms running their own code. Under the owner's rule the one-formula
   generalisation is **not adopted**; the path type is the proposal, and whether that satisfies the rule is the owner's.
2. **"Pixel → latent `z`" can't be literal.** The shape sphere's lower half has no `z`, and neither has a momentum past
   `q_max` (§ 17.2). The blend runs on the 8D manifold `z` parametrises.
3. **"The anchor IC bitwise fixed" can't come from inverse-encode** on a nonlinear end. It holds from stored words (§ 17.8).
4. **"Every intermediate frame is a valid chart"** holds as reproducible and decodable, not as contract Part 5's
   well-posedness (§ 17.5).
5. **"Live" conflicts with four written rules**, and a changed link id stays a cut under R-520 (§ 17.12). Its frame-time
   target, written first, is missed on the stand-in (about 15 ms against 4), and stands.

### 17.1 The framing

It fits the contract, which already has the classes: "Anchor-preserving (basis edits): tilt, zoom, chart-mode switch" and
"Anchor-excursion (centre edits): slice, and slider moves in locked mode". The order adds morph to the first, and it
belongs there under a lock. Three things the framing leaves out: **pan** is a centre edit in the operation table, and it
re-addresses (R-92), so it is no excursion; the classes describe **locked** behaviour, and free of a lock a basis edit has
no anchor but `z₀`; and a **lock** is no path at all, since it sets the anchor and changes neither key (R-92).

Sequence keyframes are waypoints and easing its timing: export contract Part 3 already reads so, with one `s` per frame.

### 17.2 The mechanism

**The rule: blend at the last layer where the two ends differ, and run everything downstream once, on the blend.** It is
the one statement that covers the order's cases:

| The ends differ in | Arm | Blended quantity | Downstream, unchanged |
|---|---|---|---|
| tilt angles | `Path::Tilt` | `τ(s)`, its own map (R-474) | `Tilted<C, P>`, then the chart |
| zoom | `Path::Zoom` | `exp(ln a + s(ln b − ln a))` | the view; no sim-key change (R-92) |
| the plane of an affine chart | `Path::PlaneRotation` | the basis, on the geodesic (§ 17.7) | the latent decode |
| one axis | `Path::AxisBlend` | that axis's value or control | the other axis's own construction, then holds |
| the pair map | `Path::PairMap` | `(a, b)` | both axes, then holds |
| the chart | `Path::Line` | the state's blocks, below | the decode, then any common hold |

**Why not literal `z`.** Tested:

| Question | Result |
|---|---|
| Is the blended `z` on the shape sphere, `I = 1`? | Yes: `D`'s image is the sphere at `I = 1` for every `z`; worst 8.9·10⁻¹⁶ over 20,000 blends. **No reprojection is needed.** A blend of `n̂` leaves it (worst `‖n‖ − 1` of 0.99) and a blend of positions leaves `I = 1` (worst 0.998), so neither is an option |
| Does `z` reach every end? | **No.** `β = π·σ(z_β)` puts every `z` on `n₂ ≥ 0` (held at 5,000 `z`). The shape sphere's lower half is its own systems (R-544), and encode mirrors it (`D(E(x)) = C(x)`). Leaving such an end by `s = 10⁻⁶` jumps by 1.75 median, 3.3 worst |
| Does `z` reach a held momentum? | **No,** past `q_max`: `q_k = q_max(2σ − 1)` is bounded. Under hold `E` at equal masses: 0.2% of feasible sphere pixels at `E* = −0.5`, near collisions; 0.005% at `E* = 0`. Clamped, with `lookup_clamped` |
| Is the blend's speed even? | **No,** near a saturated control. From a collinear end, 10% of the angle takes `s = 0.79`: speed spread 247 to 1 |

So `Path::Line` blends the 8D manifold of contract Part 1, `2 config ⊕ 4 momentum ⊕ 2 mass`, block by block, on each end's
own signed state (`canon::canonicalise_signed`):

```
mass      z_μ   a + s(b − a)                    the logits, as z
shape     n̂     the great circle from n̂_A to n̂_B   stays on the sphere; signed; no reprojection
momentum  q     a + s(b − a)                    unbounded; body frame, as held_momentum
```

then `place`, `to_particles` and the signed `C`: the shape sphere's own decode, so `I = 1` by construction and no new
physics. Leaving a lower-hemisphere end by `s = 10⁻⁶` now moves by 2.3·10⁻⁶. Inside the hypercube and upper half this is a
reparametrisation of the literal line, not the same line: the order says `z`, so **the block form is a door**, RQ-NEW-10.

**What the blend means.** A morph is of *addresses*, not of systems: at one pixel, A's system and B's are unrelated. So a
mid frame is a continuous picture of a family of ICs, and its structure is not a landscape of either chart. Where fibres
differ nothing special happens: each end has one state at a pixel, by its own construction (§2.2's seed included), and the
fibre is not blended. Where **branches** differ, a numerical end's state jumps across its seam, so the frame has that seam
for every `s > 0`, with a jump of `s` times the end's, at both ends' seams at once, each drawn. **The great circle has its own
seam:** at `n̂_B = −n̂_A` it isn't unique, so pixels on either side of that curve go opposite ways round. Pure function of
the pixel, R-82's pattern for the tie, a declared seam. A pair of charts whose bodies are labelled differently is blended
by index: relabelling stays distinct, so the path is not a physical one.

### 17.3 The constraint: bitwise through the mechanism

The order: tilt, zoom, lock and Save-view-as-chart "reproduce bitwise through the general mechanism, a test per operation, or
the generalisation isn't adopted." Tested through two readings of "the general mechanism":

| Operation | One formula: blended `z`, or a general geodesic | The path type (§ 17.2) | Test |
|---|---|---|---|
| **Tilt** | **Fails.** On a curved chart a tilt is `(a, b, d) = R·(u, v, 0)` on the chart's own parameters, so the blend misses it: 0.016 median, 0.124 worst in `n̂`, over 3,000 sphere pixels. On an affine chart the general geodesic reaches Tip-as-sheet's and two-target tilts' plane to 10⁻¹⁵ but not their bits: 20 of 64 projector entries equal. **Lean each axis isn't a geodesic at all,** 0.02 median, and it isn't orthonormal | **Passes,** by being the same code: `τ(s)` goes through `Tilted<C, P>`, so a frame is the view at that `τ`, bit for bit | `path_tilt_bitwise` |
| **Zoom** | **Fails.** A blend of `q` isn't log-stepped, and a frame carrying `(A, B, s)` would change the sim key where R-92 says a zoom only re-addresses | **Passes:** the arm writes the view's own field, so the sim key, the quads and the image are the direct zoom's | `path_zoom_bitwise` |
| **Lock** | Nothing to blend: a lock is no path. Equal ends do pass, except `−0`: `−0 + s·0` is `+0` | **Passes,** by not running: equal ends take the end rule (§ 17.4); `lock::kernel_chart` is unchanged | `path_lock_bitwise` |
| **Save view as chart** | Same: nothing differs | **Passes** for an existing view: a saved chart reopens as itself, R-513 and R-514's composition untouched. **A saved mid frame is new** (§ 17.5) | `path_save_view_bitwise` |

**The verdict, without softening.** Morph as the general formula of which tilt and zoom are special cases is **not adopted**:
its test fails on tilt and on zoom. What passes is narrower: a path *type* whose arms call the existing code, with morph one
arm among them. That keeps the framing, the record and the timing, and it doesn't unify the mathematics. Whether it meets the
order's constraint is RQ-NEW-10's first question.

### 17.4 `s = 0` and `s = 1`

**Ends never run the blend.** `s` is compared exactly: `0.0` takes A's own path and `1.0` B's, the keyframe rule of Part 3
("Each keyframe's own frame takes that keyframe's `SimConfig` and `RenderState` bit for bit"). It can't be left to arithmetic:
`a + (b − a) ≠ b` on 23% of random f64 pairs, and any reprojection, alignment or retiming in the path is rounding on top.
`u` is exact at its ends and REQ-TOOL-197's curves have `s(0) = 0`, `s(1) = 1`; a retiming table is pinned at both (§ 17.9).
So alignment can't be applied to B's own frame: it fades to the identity at `s = 1` (§ 17.10). The price is a step between
`s = 1 − ε` and `s = 1` of rounding size, inside the declared tolerance, not a seam.

### 17.5 A frame: valid, reproducible, recorded

**Recorded, by value:** `(A, B, path, s)`, with `A` and `B` the whole chart and view, never a name; `s` as f64 bits, in the
canonical serialisation (R-309); the retiming table's hash, the alignment and, under a lock, the anchor's words. Round trip
per R-484. Absent, the record's bytes are today's. For a `Path::Tilt` or `Path::Zoom` frame, the triple stays in the spec
only: the frame **is** a plain view and its `SimConfig` is that view's. Only `Line`, `AxisBlend`, `PairMap` and a general
`PlaneRotation` put a morph in the sim key, a new `ChartSpec` arm, so an old file's key doesn't move.

**"Valid" holds in this sense:** every pixel decodes to a system or is tagged, and one record gives one image at one
lowering. **It does not hold as contract Part 5's well-posedness**: "all 8 DOF are determined exactly once" is a check on swept
axes and blocks, and a frame has neither; a blend touches every block. Part 5's check runs on A and B. A frame's
`system_image` is unknown, since a blend may fold (§ 17.6), so `measure` is `none`, no statistic is admitted
(`admit_statistic` refuses), and axes carry no ticks, only "A, B, `s`". Contours of a primitive still work: they read the
decoded state (§ 10.2). **Saving a mid frame** gives a frame record, reopened as CPU batch, with no tilt or slice: its chart
has no hidden coordinate to turn. That is reduced capability, a label.

**R-669.** A frame is never a target's chart and never scored (replication rule 6, R-649). A morph doesn't make a refused
chart reachable as a chart: each end has to be built, and `excluded_primitives` still holds. It can pass close to a refused
plane, with the same answer as RQ-NEW-7: it is a user's picture, and the row stays "not scored".

### 17.6 Per-axis and pair-map morphs

They compose by § 17.2's rule, and that rule is what makes one axis stay fixed. If `V` is a latent axis its control is
untouched. If `V` is a constructed axis, `E` or `K`, it runs on the blended configuration, so its value is kept wherever
feasible. A chart-level blend wouldn't: between two pixels at `E* = −0.1`, the blended state misses by 0.054 median and 2.1
worst, against 1.4·10⁻¹⁶ with the step run after the blend.

**A pair-map morph folds.** Flat to polar (`θ = 2πu`, `r = v`) through the plane: folded pixels 0.3% at `s = 0.25`, 32% at
`0.5`, 94% at `0.75`, because that polar map has the opposite orientation; reversing `θ` gives 0.2%, 13%, 1.7%. Either way a
mid frame is not one-to-one: the wrap-around has to pass through a fold. V12 picks the reversed sense. Frames with folds
are flagged (§ 17.11), and the order's "valid chart" is the sense of § 17.5.

### 17.7 Plane to plane

A linear blend squashes: planes swapped give area 0 at `s = ½`; orthogonal planes 0.5; random planes 0.513 median, 0.149 worst.
The geodesic keeps the frame orthonormal to 1.3·10⁻¹⁵ and reaches B's plane to 2.2·10⁻¹⁵. Export contract Part 3 and
REQ-TOOL-075 already rule it; nothing builds it yet. Three things the existing words leave open:

- **In-plane rotation.** The geodesic reaches B's plane, not B's `(q₁, q₂)`. R-92 counts in-plane orientation. The remaining
  rotation is eased on the same `s`.
- **Handedness.** If B's frame has the opposite orientation in the plane, no rotation reaches it. The path takes the second
  principal angle's far side, `π − θ₂`.
- **Ties.** Orthogonal planes, all angles 90°, have no unique geodesic, and that's the common case, `(z_α, z_β)` to
  `(z_μ₁, z_μ₂)`. The rule: `q₁ → q₁′`, `q₂ → q₂′`. The choice is per segment, not per pixel, so it's no seam.

Zoom is log-lerped and `z₀` eased, as Part 3. This arm is general `PlaneRotation`, for ends that aren't tilts of one another.

### 17.8 Lock: a based homotopy

**What the code holds.** `lock::centre` is `z(½,½) = z₀`, and "the kernel's chart is `(z₀, q₁, q₂)` and nothing of the lock".
On a nonlinear chart the rebuilt chart's centre decodes the clicked state bit for bit through `Φ`, while `z_locked`'s own
decode is only within `ε_phys`, or at a clamp's image. So today's bitwise guarantee is on `z₀`, and REQ-GUI-005's anchor test
is a test on it.

**Does "the anchor IC bitwise fixed" hold?**

| Ends | Verdict |
|---|---|
| Both affine | **Holds, by construction.** The path moves `q₁, q₂` only, and `z(½,½) = z₀` at every `s` |
| Any nonlinear end, through inverse-encode | **Doesn't.** `E(D(z)) = z` bitwise on 0 of 20,000 at f64, and `D(E(D(z)))` is off by up to 1.3·10⁻¹⁵; lookup's tolerance is `ε_phys`, and kind 4 and invariant axes solve or project. A clamp moves it further. Iteration doesn't converge to bits |
| Across the f32/f64 boundary | **Doesn't.** A CPU batch rounds an f64 state once; the GPU computes at f32, and a component of the two agrees on 58% of samples. So bits hold within one lowering, never across lowerings |
| With stored words | **Holds.** The anchor is a state: at the path's start the current view's own lowering gives the centre's IC words, they are recorded, and a sample at chart coordinate exactly the centre takes them by a comparison |

So the design is **a based homotopy by stored anchor**. Each end is re-centred on the anchor by inverse-encode, once per end
and segment, since the answer doesn't change between frames unless the anchor does: the order's "per frame" is needed only
then. Each pixel blends its *offset* from that end's centre, added to the anchor, so `H_s(c) = anchor` to rounding and nearby
pixels converge on it; offset alone isn't bitwise (`−0 + 0 = +0`, and the decode's rounding), and the select is what gives
the bits. **The end rule and the anchor rule meet at one pixel:** `s = 1` must be B, and B's own centre differs from the words
by `ε_phys`. They agree only if a locked B, as a view, carries the words too: a chart-mode switch under a lock then records
them. That's a change to the lock's state (`SimConfig`'s lock group gains an IC) and it adds nothing to the GPU's chart. The
test reads a probe at `(½, ½)`, since an even grid has no pixel there.

**Default correspondence:** the anchor, pixel to pixel by offset. **With an excursion** (`δ ≠ 0`): the pin is the current
`z₀`, as "turns about the pin"; the words are that point's; `z_locked` and `δ` are not touched, so snap-back, `z₀ ← z_locked`,
stays bit for bit. **Deep zoom:** REQ-DEC-033's linearised decoder lands in M6. `J_D` at the anchor doesn't change along the
path, since the anchor doesn't; what changes is the chart's tangent plane, per frame. At depth a frame is locally a plane
morph, so those tangent planes follow § 17.7's geodesic, not a linear blend of Jacobians, which would squash. CPU work per
frame, f64, not built, owed to the M6 task.

### 17.9 Path shape and timing

**Easing** is R-576's, untouched: `s = ease_k(u)`. **Path shape is separate**, a field of the segment beside easing and
transition: `segments: [(easing, transition, path)]`. Waypoint charts are keyframes, so a chain of segments is already that.
Control charts bend a segment: de Casteljau's construction with each lerp replaced by the arm's own interpolation, so
planes are built from rotations, shapes from great circles. It passes through neither control, and needs both ends'
pixel coverage: each control's blank region is blank on that path.

**The named default** appears in every export's planner row and record: `plane rotation` for two affine ends, `per-axis
quantity blend` for one axis differing, otherwise `latent line`, meaning § 17.2's.

**Re-timing for constant visual speed.** A table `g` from arc length to path parameter, then `s_path = g(ease(u))`, so easing
governs timing and `g` only removes the saturation's distortion. Arc length is measured on a fixed 16 × 16 probe grid at 33
knots, as chord length of the decoded blocks, in f64 on the CPU: a pure function of `(A, B, path)`, its hash recorded, pinned
at `g(0) = 0`, `g(1) = 1`. On the worst case above: max over median speed falls from 247 to 1.12. **It conflicts with Part
3:** "No field takes `u` directly and none has a second curve". Only the blend takes `g`; every eased field keeps `s`. In a
chart-change segment those are the playhead and render uniforms, so the zoom-against-pan reason doesn't reach it, but it's
a second curve, and RQ-NEW-10 says so.

### 17.10 Without a lock: alignment

An alignment `T` in the window's similarity group, rotation and offset, chosen to minimise `Σ‖Y_A(p) − Y_B(T p)‖²` over the
probe grid, Procrustes on the grid (V14), pure and recorded; or the user picks matching points: one fixes the offset, two a
similarity. Under a lock the anchor is the correspondence, so `T` turns about it, or is off. **`T` can't be applied to B
outright**, since `s = 1` has to be B: `B` is evaluated at `T_s p`, `T_s` along the group's geodesic from `T` at `s = 0` to the
identity at `s = 1`, so at `s = 0` it has no weight and at `s = 1` it's gone. Per pixel it's cheap and adds no seam.

### 17.11 Honesty

- **Seams and degenerate frames.** The Gram factor exists: `tilt.rs`'s per-pixel area weight, `√(|û₁|²|û₂|² − (û₁·û₂)²)`, with
  `ray-degenerate` at its zero (R-554, R-474 R7). The general form is `√det(JᵀJ)`, `J = ∂Y/∂(u, v)`, by differences on the
  probe grid per frame. **Flag:** a frame with any probe below `10⁻³` of its median, a value for V15, is `degenerate`, and a
  frame-to-frame jump in the fraction below is `seam`. Unsigned: a fold reads as a zero crossing, which a coarse grid can
  step over, so it's a flag and not a proof. Charts of different topology, the sphere against a plane, must pass through such frames.
- **Out of feasible.** Tagged, never dropped (contract, "tagged, never dropped"). **A blank can open mid-path**: at
  `E* = −0.3`, equal masses, 1,173 of 1,748 pixels feasible at both ends are infeasible at `s = ½`, since the blended shape
  has a higher `U`.
- **Blank or forbidden at an end.** A pixel in a morph frame is in only if it's in both ends. No state exists for a blank end
  to blend from, and substituting the other end invents a continuity (R-470, R-467). So the blank is held for every `0 < s <
  1` and resolves at the end frame, a step at each end. Region D's mask behaves so: outside D blank at every `s < 1`.
- **Held quantities mid-morph.**

| Held | Mid-morph |
|---|---|
| The same `E*` (or `L*`) at both ends | the step runs on the blend, so it **is** held, within feasibility |
| Different targets, both on | the target is eased, like a slice value; held at the eased value |
| On at one end only | **not held**: a switch has no middle. The end's own state carries it; label `hold off mid-path` |
| Either end forbids it | not held; `E` drifts, label as above |
| Held by construction (`K`, `(ν, K)`) | held if that axis is the fixed one of a per-axis morph; else not |
| `I = 1` | always, by `place` and `C` |

  A `gauge` axis's paper scale has no meaning mid-path, so `t` is canonical time only, and region D's `t_paper` isn't drawn.
- **One colour window per morph.** The window is `RenderState.frozen_ranges` (R-581 to R-583), frozen once for the segment,
  from `s = 0`. Measured windows would re-range per frame and manufacture motion.
- **Optional simultaneous playhead change.** Part 3's policies, unchanged: hold, advance, set. Cost per Part 2 (b).

### 17.12 Performance

**Live: on the GPU with no baked pair.** A frame doesn't need one. Baking `(A, B)` would be `N²` entries, each with two decoders
inlined, about 52 KB of WGSL, so the module test's 251 KB would be spent on five. Instead: **three dispatches**, A's existing
entry and B's writing `(m, r, p)` to their own buffers, then one generic entry, `stage_blend`, reading both, blending on
blocks, writing the IC buffer the integrator reads. The kernel stays chart-agnostic, `(m, r, p)` in and out, and needs one entry
in all, with `s` a uniform. **So mid-morph frames don't count against the cap**, but both ends do need a GPU entry, and an
unbaked end is CPU batch: that's "on the GPU when both ends are closed-form", narrowed to "and baked". `stage_blend` costs the
module about one pair, an estimate, so under § 11's C2 it uses the cap's fourth slot.

It runs under the two-regime policy (caching contract): in motion the coarse cover, at rest refinement. A frame is a new
identity, as a tilt's is, its covers flotsam. It isn't re-addressable: pan and zoom inside a morph frame re-integrate, unlike
R-92's plane, except on `PlaneRotation`.

**The target, written before measuring** (`orch/scratch/chartcomp/frame-criterion.md`): one coarse cover's chart side
(1920×1080 at 1/8 linear, 32,400 samples) within 4 ms on this Mac, a quarter of a 60 Hz frame, so a morph leaves the
integrator what a tilt leaves it. The corpus has no number: `frame budget` is a setting.

**The measurement.** Real entries, `DecodeStage::run`, 32,400 samples, p50 of nine, on this Mac under load: one cover's decode
takes 2.4 to 4.2 ms typically, with outliers to 9.2; 0.2 to 0.4 of it is fixed overhead, and all of it includes the readback a
live frame wouldn't do. Five decode-sized steps is **about 15 ms. The target is missed, by a factor of about 3.4, on this stand-in.**
That number overstates by the readback, unmeasured, and it isn't a built entry. It is not loosened. In the in-motion regime a
miss means coarser or rarer covers ("coarse live glimpses updating every few frames"), not a failure to run. **What's owed:** a
dispatch-only time for a built `stage_blend`, the whole frame with integration, and the Windows arm: TASK-NEW-14. The target
is V18's, the owner's to veto.

**The CPU side, for export.** Criterion in `orch/scratch/chartcomp/morph-criterion.md`: at 1920×1080, three decodes per sample
on five cores, at most 1 s. f64 `sample`, single thread, 83 to 87 ns per sample under load: 0.53 s on one thread, **0.11 s on five.
Passes.** A closed-form end's CPU side doesn't limit export; a frame's march does, and no frame's total is measured here. A
numerical end's solve wasn't measured.

**Conflicts with the written rules, all for RQ-NEW-10:**

| Rule | Words | The order |
|---|---|---|
| export contract Part 3 | "chart *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types"; Part 2 (b) itself says "the chart morphs" | morph |
| REQ-TOOL-075 | "treat chart switches as cuts or dissolves, never parameter interpolation" | morph |
| principia_sequence § 3.4; REQ-GUI-379 | "live playback shows a dissolve as a cut at the second keyframe, as it shows every chart switch" | live first-class |
| R-520 F1; REQ-TOOL-175 | "A changed link id is a cut everywhere … there is nothing between two links to interpolate" | **stands**: the ruling wins. `Line` could cross it, its blocks being link-free, but a morph across a link id stays a cut unless ruled |
| export contract Part 3 | "none has a second curve" | re-timing |

**CPU-only ends.** Export works from day one: each frame is CPU batch, `CPU batch: path`, f64 on the CPU, rounded once, seam 3's
exception (RQ-NEW-8) extended. Live is allowed and not yet fast, a re-render per frame, labelled. Faster CPU-only live is a
recorded goal, with no task: a numerical solve per pixel per frame can't reach the target. Caching each end's blocks per
pixel would help only while the view stays put, and export doesn't need it (V16).

**Export.** Part 2's class (b): each frame its own blocking march `0 → t_frame`. So a morph of `n` frames costs `n` renders
against a dissolve's two, plainly, and the planner says so. A frame is cached under its sim key like any other; no frame
reuses another's quads.

### 17.13 The Sequence

**Today.** "A segment whose chart or any link id changes shows a **cut mark** at its second keyframe", and a chart-change
segment offers "cut or dissolve". TASK-M8-77 (mock engine) and TASK-M8-31 (spec and planner) write them; neither is built in
`crates/`: no `Dissolve` or cut-mark code exists.

**A morph segment.** `transition` gains `morph { path, retime, align }`. On the timeline, a morph segment shows a band across
the Chart lane instead of the mark at its second keyframe, labelled with the path's name. The band lights Chart; the segment
is class (b); the frame count is unchanged. The planner's row gains "a morph: every frame a different decoder", the
artefact the flag beside its spec, R-520 F5's pattern, each frame's record holding its own `SimConfig`. Physics review (⚑)
where it's built.

**"Replaces" can't be total.** A file's segments reopen bitwise (R-484), so an existing `cut` stays a cut. A changed link id
stays a cut (R-520). And a morph's cost is `n` renders, so cut stays on offer, with dissolve. **Recommended default for a new
chart-change segment: morph**, a default change, so it's the owner's. Cut marks remain only on what's actually a cut. `Play
preview` then drives the morph live rather than cutting, and the banner's words change. Reel takeover (REQ-GUI-383) on a
morph frame loads that frame, then R-624's hold-and-cut.

REQ-GUI-374, REQ-GUI-376, REQ-GUI-379, REQ-GUI-381 and REQ-TOOL-075 change only after the ruling.

### 17.14 Tests

Each with a control (R-176). `path_lock_centre` is TASK-NEW-16's, `blend_entry_counted` TASK-NEW-14's, the rest TASK-NEW-13's.

| Test | Asserts | Control |
|---|---|---|
| `path_ends_bitwise` | `s = 0.0`, `1.0`: A's and B's IC words and image hash, on every arm, with alignment, re-timing and control charts on | an end run through the blend fails on 23% of a fixture |
| `path_tilt_bitwise` | a `Path::Tilt` frame's sim key and IC words equal the view at that `τ`, on Sheet, Lean, two targets, curved | the general geodesic in its place |
| `path_zoom_bitwise` | sim key, quad addresses and image equal the direct zoom | a triple in the key |
| `path_lock_bitwise` | a path with equal ends leaves every word, `−0` included; `kernel_chart` unchanged | blending equal ends |
| `path_save_view_bitwise` | a saved chart, tilted and untilted, reopens to one digest through a path | none blended: saved chart's `R` dropped |
| `path_lock_centre` | a probe at `(½, ½)` reads one set of words on every frame, ends included, affine and nonlinear, `δ = 0` and `δ ≠ 0`; snap-back bit for bit | the select removed |
| `path_small_step` | `‖ΔIC‖ ≤ L·ds` away from declared seams, `L` from the arm; a step at a seam is declared | a frame with an undeclared seam |
| `path_record_round_trip` | `(A, B, path, s)` round-trips bitwise and re-renders one image at one lowering; absent, old bytes | `s` written at f32 |
| `path_gram_flags` | sphere to plane flags a degenerate frame; flat to polar flags folds | the flag removed |
| `path_retime` | speed spread at most 1.25 on the saturated fixture; `g` pinned at both ends | the table off |
| `path_blank_ends` | a pixel blank at either end is blank for `0 < s < 1`, tagged, never dropped | substituting the other end |
| `path_hold_table` | each row of § 17.11's table | the step before the blend |
| `path_links_cut` | a changed link id is a cut, live and export | a morph across it |
| `blend_entry_counted` | `stage_blend` is the only morph entry; the cap test unchanged | a baked `(A, B)` entry |
