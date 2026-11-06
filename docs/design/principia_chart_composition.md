# Principia — chart composition: axes from primitives, in the Chart builder

**Status: a design, 6 Nov 2026, on the owner's order of that day. Not a ruling.** No code is written by it. Its
one-way doors are not decided here: each is marked **door**, and waits on the rulings item named beside it (`RQ-NEW-n`).
Where this document meets a ruling, the ruling wins. Ids `RQ-NEW-n`, `REQ-NEW-n` and `TASK-NEW-n` are placeholders; the id
pass replaces them once the rulings branches of 4 and 5 Nov have merged. Those branches, and PRs #387 and #388, are read
here as they stood on 6 Nov: R-694 to R-720, R-791 to R-809, TASK-M2-37's branch and TASK-M2-52's.

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
4. The virial axis is built on TASK-M2-37's branch (`AxisVirial`), with its warp written. What it lacks is RQ-682's
   answer, so "then virial" is a descriptor and a ruling, not a build (§ 9).
5. A baked axis pair costs little to build and more in the module, which every pipeline parses whole: 4 pass the
   measured criterion and 8 don't, and that holds only while decode stays its own entry (§ 11).

---

## 0. The order's rules, checked

Each rule was read against the code, `decisions.md` and both rulings branches. A conflict is the ruling's, and goes to § 14.

| Item | Result |
|---|---|
| 1. Extend the builder | §G7, R-471, R-506, R-407, R-408, R-475, R-513, R-514, R-478 to R-482, §0.6 and R-670 are as the order cites them. "A new axis kind" meets "a closed set": RQ-NEW-1. R-798's words are about PA-1: RQ-NEW-1 |
| 2. Two layers | Holds, but a feasibility warp is not per-axis (§ 5.5, row 3). Hold `E` exists; hold `L_z` is new physics: RQ-NEW-4 |
| 3. Registry | R-407 clause 3 is as cited; `I = 1` at every decode is dd_decoder §5's identity 1. "CPU batch only" meets seam 3: RQ-NEW-8. Side ratios meet R-798: RQ-NEW-2 |
| 4. Gaps | "Today only the TR-1 engine scan" is not so: no code has it (§ 10.1). TASK-M7-43 and R-476 are as cited. Rule 6 is as cited. The already-covered list holds |
| 5. Region D, then virial | Defined in dd_validation_orbits §1.6. REQ-CHART-070's virial warp is written on TASK-M2-37's branch: § 9. Virial's kind is RQ-682 |
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
| Warps and the linear option | R-471, R-506, R-508; `kernel::chart::energy::{Mapping, Mapped}` on TASK-M2-37's branch | the warp slot of each axis (§ 4) |
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
| virial ratio `Q = 2K/|U|` | dd_decoder §3.6 | closed, `K* = Q·|U|/2` | **3, by RQ-682** | bounded both sides | no | free |
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

**What is not in the registry, and why (R-669).** R-669: "no binary-plus-single chart is built", and TS-2 reads "not
scored: no fixed-k velocity chart". So the registry holds no binary phase, eccentricity or impact parameter, and no
fixed-`k` prescription. A builder option can't quietly reverse a ruling. **Door** RQ-NEW-7 asks what a user-built pair
that lands on one of those planes by other means counts as; meanwhile nothing built here scores those targets.

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

**Hold `L_z` (`L*`)** is its twin, after the decode and before the derived invariants, in the same place. It adds the
missing rigid rotation, which is the minimal-kinetic-energy realisation of a change of `L_z` (inverse_encode_contract
Part 5, kind 3):

```
v ← v + ((L* − L₀)/I)·J r            with I = 1 at every decode; Σp stays 0 because the CoM is at the origin
```

**Both held.** `η_E` alone would break the held `L_z`, so with both on, the scale takes only the part orthogonal to the
rotation, in `⟨·,·⟩_m`:

```
v⁽ᴸ⁾ = (L₀/I)·J r       δv = v − v⁽ᴸ⁾       K_δ = K₀ − L₀²/2I
v ← (L*/I)·J r + η·δv    η = √((E* − U − L*²/2I)/K_δ)       feasibility: E* ≥ U + L*²/2I
```

With hold `L_z` off this path is not entered, and `η_E`'s bits do not move. Its outcomes follow §0.6: on a chart that
allows it, `K_δ = 0` or an infeasible target is a label on the decode (dd_decoder §3.7), never a refusal. This is new
physics, off by default: **door** RQ-NEW-4.

**Lowering.** `L*` is a uniform. `DecodeUniforms.energy` has two spare words and an unused bit, so the switch and the
value fit with no new row, and with the switch off the uniform's bytes are today's. No new variant.

**Tilt.** A held `L*` moves by value, `L* + d`, as R-720 rules for `(ν, K)`, `Φ_θ,K` and `Φ_strip,K`. RQ-701 is open for
`MixedEnergy`, on the same question; RQ-NEW-4 carries it.

**Same refusal rules** (R-25, R-670): the flag `forbids_lz_hold`, true where `L_z` is an axis; every `Some(L*)`
refused there, `Some(0)` included (§ 6.1). A softened run (`ε ≠ 0`) keeps hold `L_z`, since `L_z` does not read `ε`; hold
`E` stays refused there (R-670).

### 5.4 What the constraints do to a momentum axis

Hold `E` already moves a chart's momentum axis off its label: `q_k` is what the axis swept, not what is integrated. That
is today's behaviour on `Mixed`. Hold `L_z` does the same. It is reduced capability, so it is a label: "axis reads the
value before the hold". Two momentum axes under a hold can lose rank where the swept plane contains the rotation
direction `Jr`, and the label is `ray-degenerate` there (§ 5.5, row 9).

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
| 9 | Two momentum axes under a hold | the swept plane may contain `Jr` | `ray-degenerate` where it does; a label |
| 10 | Region D's edge | eq. 8 couples `ξ` and `η` | not a product domain: § 8.4 |
| 11 | Mass axis and a shape held | `m` moves `U` and the geometric shape | the axis states what it holds (§ 3) |
| 12 | Raw `z_μ` axis | softmax couples all three masses | contract Part 4: the line is "identical except one mass logit" |
| 13 | Hold `E` and hold `L_z` | `E* ≥ U + L*²/2I` | a feasibility curve across the chart; a label per pixel |
| 14 | Interior angles at a collinear point | `γ₀ = γ₁ = 0` is every point between the two bodies | the angle pair can't tell collinear shapes apart: that corner is `ray-degenerate`, and a label says so |
| 15 | Any `gauge` axis against the paper's quantity | `I = 1` rescales per pixel | a row of `E` here is not a row of a paper's `E`: § 8.6 |
| 16 | Frozen hidden coordinates | six are frozen, each read as fixed | only `gauge` axes change meaning under a slice |

---

## 6. Refusals and labels

**Any axis may be paired with any other, of any kind** (§1.3's mixed-axis rule, extended). The list below is closed.

### 6.1 Refusals: genuine contradictions only

Each is a variant of `chart_authoring::validate::Refusal`, in code, with a named reason.

| Refusal (new unless marked) | The contradiction | Example |
|---|---|---|
| `AxisAndHold { axis, quantity }` | an axis and a constraint on one quantity | `L_z` axis + hold `L_z`; `E`, `K`, `KE` or `Q` axis + hold `E` (generalises R-25) |
| `SameQuantity { a, b }` | two axes fix one quantity | `(K, KE)`; `(E, K)`, `(E, Q)` or `(K, Q)` at a frozen configuration; `(α, ‖λ‖/‖ρ‖)`; `(m_b/m_a, m_a/m_b)` |
| `NoJointInverse { a, b }` | the pair has no inverse | either axis's class is `none` |
| `SizeOnly { axis }` | it would be constant | `I`, `R` |
| `HoldAgainstConvention { hold, convention }` | a convention pins what the hold moves | hold `L_z` on a chart with a rest convention (the mass simplex, the Euclid plane) |
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
| `axis reads before the hold` | § 5.4 |
| `one orientation` | a chart reaching `η ≥ 0` only, as the third-body pair and the angle pair do (the shape sphere's whole-sphere rule, R-544, stays that chart's) |
| `paper scale differs` | a `gauge` axis at `I = 1` (§ 8.6) |

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

`A₁, A₂, A₃` are bodies 0, 1, 2. In §0.2's terms `ρ = (1, 0)` and `λ = (ξ, η)` before the gauge, at any masses:

```
ℓ² = μ_ρ + μ_λ (ξ² + η²)                     I before the gauge, = 1/6 + (2/9)(ξ² + η²) at equal masses
ρ̃ = √μ_ρ (1, 0)/ℓ          λ̃ = √μ_λ (ξ, η)/ℓ
```

That is all of it: no `atan2`, no pole at `(0, 0)`, the canonical frame by construction, `C` a no-op, the mirror not
taken at `η = 0` (the deadband, R-82). The lowering appendix's "physical (m, r, p) construction → canonicalise entry" is
this, and `ShapeSphere`'s explicit-configuration path carries it.

**It is the shape sphere, stereographically.** With §3.1's `n = (u, v, w)`:

```
(ξ, η) = √(μ_ρ/μ_λ)·(n_v, n_w)/(1 + n_u)
```

the projection from `n = (−1, 0, 0)`, §3.1's collision of bodies 0 and 1. So the plane is conformal to the mass-weighted
sphere, and the figure's Tier 1 circles are circles because of it. At equal masses D's apex is the pole `L⁺`, and the
collinear row `η = 0` is the equator. Frames of other papers are affine maps of this plane. Checked numerically at random
masses, to 6·10⁻¹³ (`orch/scratch/chartcomp/regiond_check.py`, not committed; a tested function in TASK-NEW-3).

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

Built on TASK-M2-37's branch: `kernel::chart::physical::AxisVirial`, and chart_reference §2.1's paragraph, `Q = 2K/|U|`
warped, `K* = L_z²/2I + Q·|U|/2`. REQ-CHART-070's debt is paid there, if #387 merges as it stood on 6 Nov.

What is missing is its descriptor. RQ-682 (open, on #387's branch): "Which of the four axis kinds a virial-ratio axis is:
R-471 H3 says it is not kind 3", and "No kind fits, and the set is closed". This document needs option (a) of RQ-682: kind 3,
`InvariantQuantity` gaining `Q`, solving into the momenta, reading config and mass. H3's sentence is about B1's extension,
which R-506 superseded. RQ-700 (where a virial bottom row sits, `Q₀ > 0` at a held `L_z ≠ 0`) stays open and blocks nothing:
`(a)` is applied.

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

**The start-up budget.** The baseline already breaks the contract's own number: 12 eager entries against "~10 objects"
(lowering Part 4; REQ-PERF-068), about 1 s on this Mac. Adding pairs to the eager set is what fails C3, so pairs are
created lazily on Apply (V7): that takes them out of C3's first test, and the cap's cost is then the build's and the
module's. It also departs from REQ-PERF-006's "eagerly … of every chart at startup" if a composed chart counts as a chart;
RQ-NEW-5 asks that. Under lazy creation the measured limit rises to about 6 (C1), so the customers set the number:

| Entry | Customer |
|---|---|
| 1 | region D's third-body pair (§ 8), which the angle and side-ratio axes share (V5) |
| 2 | the virial pair (§ 9) |
| 3 | R-669's `SuvakovPlane`, if RQ-NEW-1 leaves it baked as one chart. It is ruled as its own chart, so its entry is in the module whatever the cap says, and counting it here is the cautious reading |
| 4 | one spare, for the first pair a replication target wants live |

**Proposed: 4 baked entries beyond the gallery, created lazily, decode-only.** Everything past it is `CPU batch: not
baked`, a label and never a refusal (§ 6).

**What this can't say.** One Mac. Times are under the queue's other jobs, whose load ranged from 4 to 42; sizes are exact
and times aren't, so C1 and C3's small ratios carry that noise, and `P` = 4's eager pass is a bare one. Metal alone:
nothing here says what Vulkan, DX12 or WebGPU pay; wgpu and naga's parse is shared, the driver's compile isn't, and on
Metal the disk cache hides the first compile unless defeated, which the second run did with salted entries. A pipeline's
GPU run time wasn't measured, nor was CI's own build time. The stand-in is a heavy region-D pair, not the built one.
The measurement branch was a throwaway; its scripts are not in the repo.

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
| `stereographic_identity` | § 8.2's line at random masses | `μ_ρ/μ_λ` inverted | TASK-NEW-3 |
| `hold_lz` | `L_z = L*` to tolerance on every chart allowing it; `Σp = 0`; both held: `E*` too; off: bitwise | `η_E` applied alone with both on fails `L_z` | TASK-NEW-4 |
| `baked_cap` | baked pairs in `stage::decode` at or under the ledger's constant | one entry over | TASK-NEW-2 |
| `excluded_primitives` | no row names a binary phase, eccentricity, impact parameter or fixed `k` (R-669) | one added | TASK-NEW-2 |
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
| V7 | Pair pipelines lazy on Apply, not eager at start-up | § 11's C3 | lazy | § 11 |
| V8 | A 1D chart is an annotation axis | no new lowering; statistics rule already written | annotation | contract Part 5 |
| V9 | Hold `L_z` solves by the rigid rotation, not in the hidden momentum directions | minimal kinetic energy; closed form; same as §2.2 (i) | rigid | inverse_encode_contract Part 5 |
| V10 | A chart holding `L_z` by construction reads the hold as its held value | no double write; R-720's by-value | as such | § 6.1 |
| V11 | Numerical solves: Newton from a fixed start list, no continuation | a pure function of the pixel | as such | § 7.3 |

---

## 14. Doors: the rulings items

Each is `RQ-NEW-n`, written out with its file, section, quoted text and options in the queue entry the orchestrator holds.
Nothing below is decided here.

| Item | The door | Recommendation |
|---|---|---|
| RQ-NEW-1 | "A new axis kind" against "a closed set"; R-798's reading: does "no new chart type" reach beyond PA-1? | keep four kinds; yes, every new capability is a builder option |
| RQ-NEW-2 | A side-ratio axis in the builder against "There is no `SideRatio` chart" | allow the axis; PA-1 keeps its remap |
| RQ-NEW-3 | Region D's definition: domain (a), (b) or (c); eq. 3 for AA-3 and AA-5; the horizon | (a); yes; the comparison cuts by pixel |
| RQ-NEW-4 | Hold `L_z`: the physics of § 5.3, and by value under tilt | as § 5.3 |
| RQ-NEW-5 | The axis cap, and REQ-PERF-006's "every chart at startup" against REQ-PERF-068's "about 10" | § 11 |
| RQ-NEW-6 | The virial kind: RQ-682 | RQ-682 (a) |
| RQ-NEW-7 | R-669: a user-built plane that lands on a refused one | it may be built, is never scored, and the row stays |
| RQ-NEW-8 | Seam 3: a CPU-filled IC buffer | an exception for labelled batch views |
| RQ-NEW-9 | R-594 item 5: "Do not build charts ahead of it", against general polar, log and curved options | it governs charts built for a target; the options are the owner's |

**New extension types: none proposed.** Polar's `ϕ` and `r` declare existing types, under R-407 clause 3's physics review.

---

## 15. Plan

Tasks, milestones chosen by what depends on them. `TASK-M4-06` gains TASK-NEW-3 in Depends on.

| Task | Milestone | What | Waits |
|---|---|---|---|
| TASK-NEW-1 | M4 | the docs: contract, reference, lowering, §G7, §4.1 | RQ-NEW-1 to 5, 8 |
| TASK-NEW-2 | M4 | registry, refusals, labels, bitwise digest, cap test | TASK-NEW-1 |
| TASK-NEW-3 | M4 | the third-body pair, region D's preset, angles and side ratios | TASK-NEW-2 |
| TASK-NEW-4 | M4 | hold `L_z` | TASK-NEW-2, RQ-NEW-4 |
| TASK-NEW-5 | M4 | the virial pair's descriptor and entry | TASK-NEW-2, RQ-NEW-6 |
| TASK-NEW-6 | M8 | warps, polar and log pair maps | TASK-NEW-2 |
| TASK-NEW-7 | M8 | curved slices: momentum and mass | TASK-NEW-6 |
| TASK-NEW-8 | M8 | CPU batch, numerical inverses | TASK-NEW-2, TASK-M4-06 |
| TASK-NEW-9 | M7 | contours of any primitive | TASK-M7-43, TASK-NEW-2 |
| TASK-NEW-10 | M8 | the builder's panels | TASK-M8-18, TASK-NEW-3, -4, -6, -8 |
| TASK-NEW-11 | M8 | 1D charts | TASK-M8-18, TASK-NEW-2 |

---

## 16. What this does not settle

- Vulkan, DX12 and WebGPU pipeline costs: § 11 measured Metal, on one Mac.
- The `D11`, lens and Breen frames: not in the corpus (§ 8.5).
- Whether a composed chart's linearised deep-zoom decode needs its own derivative: `J_D` on a baked pair follows the shared
  path at M6; a CPU batch view has no deep zoom until a task gives it one.
- RQ-701, RQ-700 and RQ-682 stand open on #387's branch; this document assumes nothing of them but where it says so.
