# Chart reference — the maths, for implementation

**Scope:** the shared decoder, the chart views and the Burrau family, written for implementation.
This is a transcription for implementation, not a new derivation. *15 Nov 2026 (TASK-M2-59):* § 6 adds the composed
charts: the registry's formulas, the warps and pair maps, the constraints, and the third-body pair, region D's chart.

**One-line summary of the architecture.** A chart is a map `Φ : [0,1]² → Y` into some intermediate
space, followed by a **shared decoder** `D` and a **canonicaliser** `C`. Every chart feeds the same
`D ∘ C`. **The integrator is chart-agnostic** — it receives `(mᵢ, rᵢ, pᵢ)` and never knows which
chart produced them. So adding a chart means adding a `Φ`, nothing else.

```
(u,v) ∈ [0,1]²  --Φ-->  chart space  --D-->  (m, r, p)  --C-->  canonical IC  --> integrator
```

**Indexing note.** This document is **0-based throughout**: inner pair `(0,1)`, outer body `2`. The
Burrau family is conventionally written 1-based (bodies 1–3, inner pair 1–2), and §4.2 translates it.
The project-wide convention is 0-based throughout (R-22): `ICDescriptor` names `m0 m1 m2`, and pair id `k` is the side
opposite body `k` (payload §2). What a person reads is 1-based (R-449): internal body `k` is shown as body `k + 1`, a
pair by its two bodies. The shown numbers follow that map, not the Burrau literature's numbering: §4.2 gives which mass
each body has. R-449's map applies everywhere, Burrau included, and the Burrau presets' legends and the Inspector
show each body's mass beside its number, e.g. "body 1 (m = 5/12)" (R-549, RQ-335).

---

## 0. The shared decoder `D` — every chart ends here

### 0.1 Masses

```
μ_k = μ_max · tanh(z_μk)                      k = 1,2      (logit saturation)
(m0, m1, m2) = softmax(0, μ1, μ2)
M01 = m0 + m1        M = m0 + m1 + m2 = 1
```

If `M01 < ε` emit `DEGENERATE(M01_TINY)`.

**Chart constants.** `μ_max = 5` and `q_max = 2` (settled, R-10; defined in `principia_dd_decoder.md` §3). Since R-481 they are the defaults of tunable link parameters, chosen in the Decoder window, and every chart reads the active value.
`α_min = 0` (R-21): full-sphere coverage.

### 0.2 Configuration — hyperspherical mass-weighted Jacobi

Reduced masses:

```
μ_ρ = m0·m1 / M01              μ_λ = m2·M01        (M = 1)
```

**Canonical-frame decode** (the recommended path — it fixes rotation and scale gauge by
construction, so the canonicaliser is a no-op away from the seam):

```
ρ̃ = R̃ cos α · (1, 0)                        R̃ = 1        (scale gauge)
λ̃ = R̃ sin α · (cos β, sin β)                β ∈ S¹, read modulo 2π   (both orientations)

α = α_min + (π/2 − 2α_min)·σ(z_α)            σ = logistic sigmoid
β = z_β + π/2                                  (periodic_beta, the offset link; R-898, R-919, R-1020)
```

*14 Nov 2026 (R-865, R-898, R-919; TASK-M2-107), the target TASK-M2-57 builds, in force when it merges under R-898's conversion test; until then the Was describes the built decoder on `main`, and R-82, R-438 and R-544 stand:* `β` is the periodic angle (`principia_dd_decoder.md` § 3.2). *Was:* "`λ̃ = R̃ sin α · (cos β, sin β)
β ∈ [0, π]   (mirror fixed)`" and "`β = π·σ(z_β)`".

*17 Nov 2026 (R-1020, RQ-824 (b)):* the periodic link is the offset `β = z_β + π/2`, so `z = 0` stays the golden IC, the Lagrange triangle at the chart centre, and the Euler states sit at `z_β = −π/2` and `π/2` (`principia_dd_decoder.md` § 3.2, § 5). *Was (14 Nov 2026):* "`β = z_β                                        (periodic_beta, R-898, R-919)`".

`α_min = 0` (R-21). It is not a numerical guard; a non-zero value would only excise a polar cap, and the poles are
fenced by the collision detector, the conditioning readout and the saturation flags. **Note the orientation, which is easy to get
backwards:** `‖ρ̃‖ = cos α`, so **small α is a LARGE inner-pair separation**; `α → π/2` is a tight
inner pair with a distant third body (hierarchical).

**Unweight and reconstruct positions:**

```
ρ = ρ̃ / √μ_ρ                  λ = λ̃ / √μ_λ

r01 = −m2·λ
r2  =  M01·λ
r0  = r01 − (m1/M01)·ρ
r1  = r01 + (m0/M01)·ρ
```

COM is at the origin by construction. `I = ‖ρ̃‖² + ‖λ̃‖² = R̃² = 1`.

### 0.3 Momentum — free Jacobi momenta

```
q_k = q_max·(2σ(z_qk) − 1)        k = 0..3
p_ρ = (q0, q1)                    p_λ = (q2, q3)

p0 = −p_ρ − (m0/M01)·p_λ
p1 =  p_ρ − (m1/M01)·p_λ
p2 =  p_λ
```

> **Transcription hazard.** The `m0` and `m1` factors are **crossed** relative to the position
> reconstruction (positions take `−m1/M01` on `r0`; momenta take `−m0/M01` on `p0`). It matches
> `principia_dd_decoder.md` §3.4. Transcribe it, then verify by asserting the conjugacy `μ_ρ(p₁/m₁ − p₀/m₀) = p_ρ` (or the kinetic-energy identity) at
> unequal masses: the test that catches a swap. `Σpᵢ = 0` catches only a one-sided swap (R-547, RQ-283). *Was:*
> "verify by asserting `Σpᵢ = 0` to machine precision — the test that catches a swap."

### 0.4 Canonicalisation `C`

*[mirror-step removal (R-865, R-898, R-919): rewritten 14 Nov 2026 by TASK-M2-107 to the decoder TASK-M2-57 builds, with R-898's periodic `β` and R-919's one `β` link; in force when TASK-M2-57 merges, having passed R-898's conversion test; until then each *Was* below describes the built decoder on `main`, and R-82, R-438 and R-544 stand. In the target, `C` has no mirror step, latent `β` is a periodic angle, `β = z_β` read modulo 2π, a lower-half state encodes to `β ∈ (π, 2π)`, and every chart, the shape sphere included, decodes by one path. 17 Nov 2026 (R-1020, RQ-824 (b)): the periodic link is the offset `β = z_β + π/2` (`principia_dd_decoder.md` § 3.2), so `z = 0` stays the golden IC; the identity just stated is its Was. This section is rewritten to that target, each changed passage keeping its *Was*; the definitions are `principia_dd_decoder.md` § 3.2 (REQ-DEC-056). Was (this marker): "mirror-step removal, ruled 8 Nov 2026 (R-865, on RQ-739): the decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the owner's bitwise check (every preset, saved chart and golden with `β ∈ [0, π]` reproducing unchanged), which meets the link `β = π·σ(z_β)`: RQ-750, on which the task waits. This section changes when TASK-M2-57 lands, and stands as written until then. Was: "[mirror-step proposal, 7 Nov 2026 (R-854; RQ-739): the owner's proposal to remove the decoder's mirror step, latent `β` over the full circle, would touch this section. A proposal, not applied; nothing here changes until it is ruled.]""]*

With the canonical-frame decode it is a no-op, across the `λ̃_y = 0` seam as away from it. Implement anyway, for
charts that bypass it:

```
φ = atan2(ρ_y, ρ_x)     rotate all rᵢ, pᵢ by R(−φ)
```

There is no mirror step (R-865): `λ̃_y` keeps its sign. *14 Nov 2026 (R-865, R-898, R-919; TASK-M2-107), the target TASK-M2-57 builds, in force when it merges under R-898's conversion test; until then the Was describes the built decoder on `main`, and R-82, R-438 and R-544 stand:* *Was:* "With the canonical-frame decode both
steps are no-ops away from the seam." and the two lines "`if λ̃_y < −δ_λ:          mirror (δ_λ = 1e−12; λ̃ = √μ_λ·λ)`"
and "`|λ̃_y| ≤ δ_λ:            no mirror (deterministic tie-break — R-82)`".

**`φ` at the pole (R-437).** At `ρ = 0` (the collision of bodies 0 and 1) `φ` is 0, whatever the signs of the zeros,
so the rotation is the identity there: a select on `ρ_x = 0` and `ρ_y = 0`, not `atan2(0, 0)`, which GLSL.std.450 and
WGSL leave undefined (`principia_dd_decoder.md` §3.2, "Every `atan2` at a possible pole").

**`λ̃_y` of either sign.** The encode's `β` is the signed angle of `λ̃` onto `[0, 2π̂]`, `β ∈ (π, 2π)` for a
lower-half state (`principia_dd_encode.md` §3.2). *Was:* "**After the mirror test, `λ̃_y ≥ −δ_λ`, not `λ̃_y ≥ 0`
(R-438).** The deadband is left unmirrored (R-82), so the encode's `β = atan2(|λ̃_y|, λ̃_x)` reads `|λ̃_y|`, and
`β ∈ [0, π]` holds because it does, not by the mirror alone (`principia_dd_encode.md` §3.1)."

### 0.5 Scale gauge

```
ℓ = √I        rᵢ ← rᵢ/ℓ        pᵢ ← √ℓ·pᵢ
```

No-op when `R̃ = 1` was enforced; encode's `C` skips it where `|I_in − 1| ≤ τ_I` and otherwise rescales in two steps
(`principia_dd_encode.md` §3.2; R-547). **Note the asymmetric powers** — positions divide by `ℓ`,
momenta multiply by `√ℓ`. That is what makes the transformation canonical.

### 0.6 Energy normalisation — optional, and forbidden on some charts

```
η_E = √((E* − U)/K0)          pᵢ ← η_E·pᵢ          feasibility: E* ≥ U
```

**Each chart carries a `forbids_energy_normalisation` flag.** It must be `true` for `(Lz,E)` and
`(Lz,K)`, where energy is a chart coordinate or is enforced by the momentum construction — applying
it there would collapse the energy axis. **Enforce in code, not prose:** the validation pass
refuses a chart with the flag set combined with any `E*` override — every `Some(E*)`, including
`Some(0)` (R-25). On a chart that allows it, the outcome at `K0 = 0` and on `E* < U` is a label on the
decode, never a refusal (`principia_dd_decoder.md` §3.7). *2 Nov 2026 (R-670):* the validation pass also refuses
`ε ≠ 0` combined with any `Some(E*)`, on every chart.

### 0.7 Degeneracy — every pixel gets a label

`DEGENERATE(reason)` for numerical failure; `COLLISION_T0(pair)` with `t_event = 0` if
`r_min(0) < r_coll`. **No pixel is ever rejected.**

---

## 1. The 8D latent chart — the reference coordinate system

```
z = ( z_α, z_β | z_q0, z_q1, z_q2, z_q3 | z_μ1, z_μ2 ) ∈ ℝ × S¹ × ℝ⁶
      config       momentum                 mass
```

*[RQ-750 (a) + (c), ruled 8 Nov 2026 (R-898): `z_β` becomes a periodic angle over the full circle, so the latent space
is `ℝ × S¹ × ℝ⁶`, `z_β` on the circle, not `ℝ⁸`; every stored latent view, saved chart and golden is converted once.
Rewritten 14 Nov 2026 by TASK-M2-107 to the target TASK-M2-57 builds, in force when it merges, the Was describing `main` until then: the line above reads `∈ ℝ × S¹ × ℝ⁶`, `z_β` stored unreduced on the lift and read
modulo `2π` (`principia_dd_decoder.md` § 3.2); "8D" below counts dimensions, `z_β` one of them. Was: "`∈ ℝ⁸`" and
"This section stands as written until TASK-M2-57 merges, which rewrites it."]*

**No latent coordinate is spent on gauge.** Rotation and scale are fixed by the canonical-frame
decode, which is why the chart is 8D and not 10D.

### 1.1 Affine slices — the "no tilt" case

Slice centre `z0` and two direction vectors `q_a`:

```
z(u, v) = z0 + (2u − 1)·q_1 + (2v − 1)·q_2
```

The slice scale lives in `q`: zoom is a common scale on `q_1, q_2` (`principia_chart_decoder_contract.md` Part 4;
R-83). There are no per-axis factors.

**Axis-aligned (no tilt):** `q_1 = ê_i`, `q_2 = ê_j` for basis vectors of ℝ⁸. There are `C(8,2) =
28` such planes. The interesting named ones:

| slice | `q_1, q_2` | what it varies |
|---|---|---|
| **shape** | `ê_α, ê_β` | triangle geometry at fixed mass and momentum |
| **inner momentum** | `ê_q0, ê_q1` | `p_ρ` — the inner pair's relative momentum |
| **outer momentum** | `ê_q2, ê_q3` | `p_λ` — the third body against the inner COM |
| **mass** | `ê_μ1, ê_μ2` | the mass simplex at fixed geometry |
| **mixed** | `ê_α, ê_q2` | one config against one momentum axis |

**Arbitrary / oblique:** `q_1, q_2` any orthonormal pair in ℝ⁸. Generate by taking two random
Gaussian vectors and Gram–Schmidt. **Report the pair used**, or the slice is not reproducible.

> **A tilt is a rotation of the 2-plane, not a re-centering.** A 2-plane in 8D has 12 tilt
> axes (6 hidden dimensions × 2 basis vectors). Raw tilts **replace** rather than compose — the
> chart constructor is the commit mechanism.

**Twelve is the full count; Save view as chart commits (R-475).** "Twelve tilt directions" (6 hidden coordinates × 2
axes) is the full count. The UI offers two at a time, through the Tilt section's target dropdowns ("tilt τ₁ into
[X]", "tilt τ₂ into [Y]", R-457). A plane in which an axis mixes several hidden coordinates is reached by saving and
tilting again, or by a custom chart. **Save view as chart** (render_gui_spec §G2) is the commit mechanism above: it
commits the current view as a named custom chart, which can then be tilted, sliced and saved like any other, so with it
tilts compose and any plane is reachable by navigating. A latent view saves `(z₀, q₁, q₂)` as an oblique chart with
affine axes, the pair reported as this section asks; a curved chart saves its parent chart, its slice centre and its
tilt rotation `R` with its targets and map (`principia_chart_decoder_contract.md` Part 4). Reopened with zero tilt, a
saved chart reproduces the saved view bitwise.
A new tilt `R′` of a saved curved chart into its own saved target composes the rotations, `(a, b, d) = R·R′·(u, v,
0)`, the saved `R` acting on `(u′, v′, d′)`; into another target the new offset is added to it, and the saved target is
offered with the others (R-513). A latent view saved at a tilt offers as its targets the named dimensions other than
its first parent's axes, each projected off the saved plane and normalised, the vanishing ones dropped; both tilt maps
act in the orthonormal frame `(q̂₁, q̂₂⊥, t̂)` that Gram–Schmidt makes of the stored pair and the target, and carry the
stored pair itself, so it is kept and the view reopens bitwise (R-514).
*Ruled 8 Nov 2026 (R-895, RQ-746 (a)):* where the saved tilt is Lean each axis with one target and both angles set, its
axes skewed, `R`'s third input is the saved plane's unit normal, `t̂ = û₁ × û₂` normalised, its sign toward `+e_X`, and
the composition is R-513's `R·R′` literally, not a rotation (the cross product's own sign at Lean's edge, no frame at
`|τ₁| = |τ₂| = 90°`). TASK-M2-56 builds it; until it merges the kernel refuses that one case.

### 1.2 Geometry + momentum in a fundamental domain

Pin $\boldsymbol\rho = (\rho_0, 0)$, sweep $\beta(s) = \pi s$, and fix $\boldsymbol\lambda = \lambda_0(\cos\beta, \sin\beta)$. For
the momentum, vary the direction of $\mathbf p_\lambda$ and fix $\mathbf p_\rho$.

### 1.3 Mixed-axis charts

The two axes need not come from the same block. Any pair of coordinates can be the horizontal and vertical
axes, with the other six frozen at stated values.

| horizontal | vertical | what it shows |
|---|---|---|
| shape $\theta$ | $L_z$ | how basin structure varies with shape and angular momentum |
| shape $\varphi$ | $K$ | azimuthal orientation vs kinetic energy |
| $\nu$ (Burrau) | $K$ | triangle shape vs energy (the bifurcation strip, §4.5) |
| mass $m_1$ | shape $\theta$ | mass–shape coupling at fixed momentum |
| $L_z$ | shape $\theta$ | angular momentum vs triangle elongation |
| $\alpha$ (Jacobi) | $p_\rho$ | the classical phase-space portrait |

**Implementation.** Each axis states which latent coordinate or derived quantity it maps and the warp from
$[0,1]$ to its physical range. The chart convention fixes the other six (for example "Burrau masses, rest
start"). Then $\Phi_{\mathrm{mixed}}(u,v) = D(\mathbf z(u,v))$, where $\mathbf z(u,v)$ takes coordinate $i$ from $u$,
coordinate $j$ from $v$, and the rest from the frozen values. It composes with the shared decoder and needs
no new GPU logic. When one axis is a configuration coordinate and the other its conjugate momentum, the
render is a Poincaré section.

---

## 2. Invariant-momentum charts `(Lz, E)` and `(Lz, K)`

These fix geometry and mass, and use the two axes for **conserved quantities** rather than raw
controls. `forbids_energy_normalisation = true`.

**The fixed geometry and masses** (defined per R-72, REQ-CHART-048; TASK-M2-08). They are the slice pin `z₀`'s: its
configuration block through the shared `D_cfg` (§0.2, the canonical-frame decode, so `I = 1`) and its mass block
through the shared `D_mass` (§0.1), under the chart's link selection. Only the momentum block is built per pixel, by
§2.2 from the pixel's `(L_z, K*)`; `z₀`'s momentum block is not read. The chart's parameters are the four frozen
controls `(z_α, z_β, z_μ₁, z_μ₂)`. They are set from `z₀`'s configuration and mass blocks when the chart is entered,
or from the lock where one is set, and edited like any chart parameter (Design axiom 6) (R-549, RQ-350).

### 2.1 Feasibility, and the warp that makes every pixel valid

`E = U + K` with `K ≥ 0`, so `E ≥ U`, and

```
|Lz| ≤ √(2·I·K) = √(2·I·(E − U))
```

The feasible region is the interior of a parabola with apex at `(Lz, E) = (0, U)` — the rest start.
**Domain warp** (choose `K_max > 0`, exponent `γ_K ≥ 1`):

```
K(t)     = K_max · t^γ_K
E(t)     = U + K(t)
L_max(t) = √(2·I·K(t))
Lz(s,t)  = (2s − 1)·L_max(t)
```

with `(s,t) = (u,v) ∈ [0,1]²`. **This maps the unit square onto the feasible interior, so no pixel
is infeasible by construction** — which is why the warp exists rather than clamping.

*5 Nov 2026 (R-791, closing RQ-693 (b)):* at `ε ≠ 0` the `(Lz, E)` chart stays available, and its `E` is the Newtonian
energy: `U` here, in the feasible set, the warp and §2.2's `K*`, is `V` at `ε = 0`, never the softened `V₀` of
`principia_dd_decoder.md` §3.6. So a pixel's start is the same at every `ε`, and toggling softening changes only the
physics. The run's own `E₀` is softened, so it isn't the axis value: the axis is labelled as Newtonian energy whenever
softening is on, and the Inspector shows the softened energy beside it (`principia_render_gui_spec.md` §G2, §G8). Encode
reads the same Newtonian `U` on this chart, so entering a pixel's `(Lz, E)` returns that pixel's `z` at any `ε`. Whether
this reaches the other axes that read `U`, the lone, mixed and Chart-builder `E` axes and the virial ratio, was RQ-709.
*7 Nov 2026 (R-819, RQ-709 (a)):* it reaches all of them. Every axis that reads `U` reads the Newtonian `U` at `ε ≠ 0`,
each labelled, the softened value beside it in the Inspector; on the virial ratio the axis's Newtonian ratio differs
from the stored, softened `virial_ratio`, and the Inspector shows that disagreement. TASK-M9-39 builds it.

For `(Lz, K)`: identical, with `K* = K(t)` directly. Simpler, since `K ≥ 0` is the natural
constraint.

**Past the unit square (R-407, R-508, R-932, R-937): each `s` edge unfolds, `t` clamps below `t = 0` and continues
past `t = 1`** (§5.4's table; TASK-M2-95; *was:* "`s` is bounded, and `t` bounded below `t = 0` and continuing past
`t = 1`", and before that "both axes are bounded"). Outside `[0,1]²` the warp has no feasibility guarantee of its own:
`s ∉ [0,1]` gives `|L_z| = |2s − 1|·L_max(t) > L_max(t)`, so `K* < K_min`, infeasible on sheet 1 for every `γ_K`; `t < 0`
has no real value for a non-integer `γ_K` (an odd integer gives `K < 0`, an even one folds back onto `t > 0`). So each
`s` edge declares **unfold**, the default (R-932, R-937): past either edge the chart shows the construction's second
sheet, `s' = 2 − s` past `s = 1` and `s' = −s` past `s = 0`, period 2 in `s`, sheet 2 at `L_z` being the time reversal
of the chart's own system at `−L_z` (§2.2 (iv)); clamp (the rigid rotation at `±L_max(t)`, REQ-ENC-030's projection,
§2.2 (vi)) and blank are the kinds the edge may be switched to. Below `t = 0` the axis declares **clamp**: the rest
start, REQ-ENC-030's projection to `t = 0` (§2.2 (vi)), at the pixel's own configuration under a tilt or slice that
moves it. Its `t > 1` side (`K > K_max`) stays feasible, and it **continues**, affine (R-508, ruling RQ-376, the
declaration under physics review R-407 clause 3 asks for): past `t = 1` the formula runs on, `K = K_max·t^γ_K > K_max`
and `L_max(t)` growing, so every pixel with `s ∈ [0,1]` is a system, counted as R-408 counts one; the `t` axis's primary
range is `t ≥ 0`. *Was:* "So the warp is bounded past `s`'s edges and below `t = 0` (§5.4)."; "Past `t < 0`, and the
`s` axis past either edge, the warp stays bounded." *Was (before R-508):* "So the warp declares nothing beyond bounded
and is hatched on every side (§5.4). Its `t > 1` side (`K > K_max`) stays feasible; declaring it would be a later
declaration, needing physics review (R-407, clause 3), and is not made. Whether to declare it affine is queued: RQ-376
(R-469, §11)."

**Under the linear option** (E2, below) the edge is the parabola `|L_z| = √(2·I·(E − U))` at each pixel's own `U` and
`I`, curved and inside the window: where `K* > 0` past it the axis unfolds, `L_z` reflected about the parabola at the
pixel's own `E` and the sheet flipped (§2.2 (iv)), or, switched to clamp, takes REQ-ENC-030's projection (§2.2 (vi));
below the floor at a non-zero `L_z` (`K* ≤ 0`, `L_z ≠ 0`), where REQ-ENC-030 rejects, the drawn extension is the clamp
below the floor, the rigid rotation at the pixel's own `L_z`, `E' = U + L_z²/(2I)` (`K' = L_z²/(2I)` on `(L_z, K)`),
its metric `|ΔE|` at fixed `L_z` (§2.2 (v), R-937, a definition under R-72 with the physics reviewer's conditions C1 to
C7); and where `K* < 0` with `L_z = 0`, the rest start. The seam along `E = U` is declared in §5.4's table.
*Was (the dated note this paragraph replaces, 10 Nov 2026):* "the paragraph above is amended, and built by the chart
addendum's N12 to N14. Past either `s` edge the warp unfolds onto the construction's second sheet (§ 2.2): `s' = 2 − s`
on sheet 2, period 2 in `s`, the time reversal of the chart's own system at `−L_z`. Below `t = 0` it clamps to the rest
start, REQ-ENC-030's projection. Under the linear option the parabola reflects `L_z` at the pixel's own `E` and flips
the sheet; and below the floor at a non-zero `L_z` (`K* ≤ 0`, `L_z ≠ 0`), where REQ-ENC-030 rejects, the drawn
extension is the rigid rotation at the pixel's own `L_z`, a vertical clamp to `E = U + L_z²/(2I)` (`K = L_z²/(2I)` on
`(L_z, K)`), each pixel reading its own Newtonian `U` and its own `I`: a new definition (R-72), its metric `|ΔE|` at
fixed `L_z`, beside REQ-ENC-030's `|ΔL_z|` at fixed `E`, with this section's held-`L_z` floor clamp and § 4.5's held-`K`
floor clamp as its precedent. On `Φ_θ,L_z` (§ 4.5) the same rule clamps the held `K` to `L_z²/(2I)`. The physics
reviewer's conditions on it are in R-937." The chart addendum's N13 and N14 build it; until they land the built charts
draw as before (R-932).

**Every energy-type axis is warped by default (R-471).** Axes of kind 3 (`E`, `L_z`, `K`, `KE`, `PE`;
`principia_chart_decoder_contract.md` Part 3) take this warp by default, wherever they appear: on these charts, the
Burrau family's `K` axes, a mixed-axis chart (§1.3) and a Chart-builder axis (render_gui_spec §G7), superseding R-408's
port B1 (a physical axis mapped linearly, affine) as the default. Alone, or beside a non-energy axis, an energy-type
axis takes its own line of the warp: `K` (`KE`) `= K_max·t^γ_K`, `E = U + K_max·t^γ_K`, `L_z = (2s − 1)·L_max` with
`L_max = √(2·I·K)` at the kinetic energy the chart holds fixed (as §4.5's `Φ_θ,L_z`). A `PE` axis has no warp written
yet: REQ-CHART-070 (R-72), and no chart offers one until it is. **The warp is evaluated per pixel,** with that pixel's
own `U`, `I` and masses: a tilt or slice that moves a hidden coordinate (masses, shape) changes `U` and `I` across the
window, and a warp computed once at the centre would put some pixels outside the feasible region; evaluated per pixel,
every pixel stays feasible under every tilt (R-471, E3; R-473, T5; at a held `L_z` on a Burrau `K` axis through the floor
of R-708, §4.5), each pixel whose own chart parameters lie in
`[0,1]²`: under Tip as sheet with one target and both angles set a window pixel's `(a, b)` can leave the square, the
axes' types then acting on it (§5.4), and such a pixel is past the edge, blank past a bounded one (R-512, ruling
RQ-402; R-471, H9). *Was:* "and what such a pixel draws is RQ-402 (R-471, H9)." Untilted, they are `z₀`'s at every pixel (§2, the
fixed geometry and masses), so nothing changes. **A linear mapping is an option per axis** (E2): the axis is affine,
mapped onto the warp's range at the slice centre (`K ∈ [0, K_max]`, `E ∈ [U₀, U₀ + K_max]`, `L_z ∈ ±√(2 I₀ K_max)`, a
builder axis onto its own range); its infeasible pixels in the window take their edge's declared kind (R-932, R-937;
the paragraph above and §5.4's table: on the energy charts the unfold past the parabola where `K* > 0` and the clamp
below the floor where `K* ≤ 0`, `L_z ≠ 0`), with no decoder-bug warning, and the navigation stop (R-469) keeps the
centre on valid ground, which an unfold sheet is and a clamped area is not (R-942). The mapping is recorded in the view
state and the export's sidecar. Under the linear option the axis is R-408's B1 axis, affine past its edge, its
feasibility edge taking the declared kinds; warped, its edges are as above: unfold past `s`'s, clamp below `t = 0`,
affine past `t = 1` (R-508). *Was:* "its infeasible pixels in the window are not a system and draw blank (R-470), with
no decoder-bug warning, and the navigation stop (R-469) keeps the centre feasible."; "warped, it is bounded, as above,
but past `t = 1`, where it continues (R-508)."

**Every physical-quantity axis (R-506).** A Chart-builder physical-quantity axis that is not energy-type, the virial
ratio, the mass ratio and the rest of render_gui_spec §G7's list, follows the same rule (R-506, ruling RQ-372):
warped to its feasible range by default, so every pixel of `[0,1]²` is feasible, bounded past its edge; linear as an
option, B1's affine axis, its pixels that are not a system blank. The mass ratio warps through the mass link, the
active one (§0.1's softmax at the defaults; since R-478 and R-481 the Decoder window's registry entry at its active
parameters, merge of the links port, applied per R-369): its axis moves the active mass link's inputs, so every pixel's
masses are positive and sum to 1. *Was:* "The mass ratio warps through the mass link (§0.1's softmax): its axis
moves the mass logits, so every pixel's masses are positive and sum to 1." The warps this section
does not write (the virial ratio's, a ratio's range in the logits) are REQ-CHART-070's definition (R-72), written here
under physics review before a chart offers them. The decoder-bug warning (R-467, clause 4) is for a chart whose
physical-quantity axes are all warped.

**The `PE` axis: no chart offers it (REQ-CHART-070, R-471 H2).** `PE = U` is a function of the configuration and the
masses alone, so a `PE` axis would solve into the configuration block, not the momenta (kind 3, reading the masses).
At the masses and scale gauge (§0.5) it is unbounded below, toward a collision; a level `U = U*` is a curve in the
configuration's DOF, so an axis of it needs a residual convention to pick a point on that curve, as well as a warp of
`t` onto the range. This
section writes neither, so no chart offers a `PE` axis: the well-posedness validator refuses one (TASK-M2-37). A warp
for it is a new paragraph here, under physics review, before any chart offers it.

**The virial ratio's warp (REQ-CHART-070, R-506 W2).** For `Q = 2K/|U|` (`U < 0` at positive masses), the axis keeps
its own asked range `[Q_lo, Q_hi]`, as a Chart-builder axis does (R-471 H4; render_gui_spec §G7). Warped, it runs
over the range's feasible part `[Q₀, Q_hi]`, `Q₀ = max(Q_lo, 0)`: `Q(t) = Q₀ + (Q_hi − Q₀)·t^γ_K`, where `γ_K` is this
section's exponent, so the range `[0, Q_max]` gives `Q_max·t^γ_K`; a range with `Q_hi ≤ 0` has no feasible part, and
no chart offers it. The kinetic energy at the pixel is `K* = Q(t)·|U|/2`, using the pixel's own `U` (E3). `K*` is
built by §2.2's construction at the chart's held `L_z`. At a held `L_z = 0` every pixel of `[0,1]²` is feasible,
since `K* ≥ 0`; at a held `L_z ≠ 0` the next paragraph adds the floor. The axis is bounded past both of its edges. Under the linear option the axis is
`Q(t) = Q_lo + (Q_hi − Q_lo)·t`, affine, over the asked range, and its pixels with `Q < 0` or `K* < L_z²/2I` are not a
system.

**A warped `t` axis at a held `L_z ≠ 0` stands on the floor (the owner's ruling of 4 Nov 2026 on RQ-683).** A lone
`K`, `KE`, `E` or virial axis holds `L_z` at the chart's value. Warped, its line adds the floor `L_z²/2I`, the least
kinetic energy that spin allows: `K* = L_z²/2I + K_max·t^γ_K` on `K` and `KE`, `E = U + K*` with that `K*` on `E`, and
`K* = L_z²/2I + Q(t)·|U|/2` on the virial ratio, the pixel's own `U` and `I`. So every pixel of `[0,1]²` is a system.
On `K`, `KE` and `E`, and on the virial ratio with `Q₀ = 0`, the bottom row `t = 0` is the floor itself, §2.2's rigid
rotation with `a = 0`. The top row exceeds `K_max` on `K`, `KE` and `E`, and `Q_hi` on the virial ratio: each is a
range, and `K_max` is passed past `t = 1` too (R-508). The pixel's own virial ratio `2K*/|U|` is
`Q(t) + L_z²/(I·|U|)`. On `K`, at `I = 1`, the floor is constant, so a row is a level set of `K`. On the virial ratio
the offset varies with `U`, so a row is no level set of `2K/|U|`, and a readout of the axis shows the pixel's own
ratio. **With `Q₀ > 0` the floor sits under the rise only (R-817, ruling RQ-700 (b)):** with `Q_min(U) = L_z²/(I·|U|)`,
the pixel's ratio is `max(Q₀, Q_min(U)) + (Q_hi − Q₀)·t^γ_K`, `K*` its `|U|/2` multiple, so the bottom row is the asked
bottom `Q₀` where `Q₀` is feasible and on the floor elsewhere, and at `Q₀ = 0` this is the additive form above, bit for
bit. TASK-M2-70 builds it. *Was:* "With `Q₀ > 0` the additive form is applied until the owner answers RQ-700: the asked
bottom `Q₀` is not drawn even where it is feasible, and `a ≠ 0` on the bottom row." At a held
`L_z = 0` the floor is 0 and the lines above stand unchanged. The bottom row must not blank on rounding: the floor is
the least normal float §2.2's exact test finds feasible at the held `L_z`, `L_z²/2` at `I = 1` rounded, raised one
float where it rounded below, and never subnormal, so a backend that flushes subnormals keeps it; adding a non-negative
term to it cannot fall below it. Under the linear option the axis
keeps its fixed values and no floor is added: its pixels with `K* < L_z²/2I` are not a system and draw blank (R-470).
The held `L_z`, and the held `K` on an `L_z` axis, is this chart's hidden coordinate under R-633, as the physical
parameter of the construction that holds its momentum block. No task builds its tilt or its slice: whether and how
it tilts and slices is RQ-701's. The one exception to every pixel being a system: a held `L_z` whose `L_z²` has no
finite float at the chart's own precision has no floor, and every pixel of that chart is not a system,
§2.2's exact test labelling it and `validate` agreeing at that precision. *Was:* "**A warped `t` axis at a held `L_z ≠
0` is not offered (R-471 E1, R-506; RQ-683).** A lone `K`, `KE`, `E` or
virial axis holds `L_z` at the chart's value. At a held `L_z ≠ 0` its pixels with `K* < L_z²/2I`, near `t = 0`, would
be non-systems inside the square, and no warp written here makes them feasible. So no chart offers that axis warped at
a held `L_z ≠ 0`: `validate` refuses every point of it (applied per R-369, TASK-M2-37). Whether such an axis should
exist, and under what warp, is RQ-683's. Under the linear option it is offered, its pixels with `K* < L_z²/2I` not a
system."

**The mass ratio's range in the link's inputs (REQ-CHART-070, R-506 W3).** For two bodies `a` and `b`, the axis is the
ratio `q = m_b/m_a`, with the third body's mass `m_c` held at its slice-centre value. Then `m_a = (1 − m_c)/(1 + q)`
and `m_b = q·m_a`. The axis writes the active mass link's inputs, `z_mass = L⁻¹(m)`, and the decode reads the masses
back through the link, `m = L(z_mass)`, so every pixel's masses are positive and sum to 1.

A ratio is realised when the link's inverse followed by its forward returns its masses `(m_a, m_b, m_c)`, `m_c` at
its held value, and the ratio `q`, each to within `ε_μ`, relative, since the inverse saturates past what the link
reaches. The ratio is read as well because each mass within `ε_μ` bounds it only to about `2·ε_μ`, and `q` is what the
axis labels. Where the logit that sets `m_c` saturates first, the round trip keeps the ratio and moves `m_c`, and
that ratio is not realised.
`ε_μ = 10⁻⁶` is the link clamp (chart_decoder_contract § "Three hard requirements on any registered link", item 2;
inverse_encode_contract Part 3), reused here as the round trip's tolerance, not a new constant.

Warped, `q(t) = q₀·(q₁/q₀)^t`, geometric, over `[q₀, q₁]`: the part of the asked range the link realises at `m_c` and
at its active parameters. A chart decoded at `f32` holds `m_c` narrowed to `f32`, and rounding it moves the two small
masses by a relative `δ·m_c/(1 − m_c)`, so the range is the part realised at `m_c` both as held and as narrowed, and
where those share no ratio, as can happen at a held mass near 1, the chart is not offered. The `ε_μ` test runs at f64
arithmetic on the narrowed values, and an `f32` decode's own rounding is held to Tier N (parity_contract §4). The axis
is bounded past both of its edges.

The search for `[q₀, q₁]` holds a ratio to `ε_μ/2`, half the tolerance above, and that margin is the search's alone: a
ratio still counts as realised at `ε_μ`, so `[q₀, q₁]` can be shorter than the part the link realises at `ε_μ`, by as much as the
measured worst, 5.48 in `ln q`, at an extreme ratio, and a chart realised at `ε_μ` can be refused. The reason is measured. The range's ends are narrowed to `f32`, and that rounding
must stay inside the `ε_μ` a decoded pixel is held to: at the full `ε_μ`, 62% of narrowed `softmax_tanh` charts had a
refused row. The need is `1 − 2⁻²⁴/ε_μ ≈ 0.94` of `ε_μ`. The `1/2` is chosen with room, not derived (R-810, closing
RQ-707 (i)).

Linear, `q(t) = q_lo + (q_hi − q_lo)·t`, affine, over the asked range. A pixel whose ratio is at or below 0, or is not
realised, is not a system.

### 2.2 Deterministic momentum construction

Given target `Lz` and `K*`, construct `pᵢ` with `Σpᵢ = 0`, `K = K*`, `Lz` as specified. Work in
velocities `vᵢ = pᵢ/mᵢ`; let `J(x,y) = (−y, x)` and `⟨a,b⟩_m = Σ mᵢ aᵢ·bᵢ`.

**(i) Minimal-energy rigid rotation realising `Lz`:**

```
ω = Lz / I              vᵢ^(L) = ω·J rᵢ              K_min = Lz²/(2I)
```

**(ii) A direction field that adds energy without changing `Lz`.** Deterministic seed family, in
this order (the tie-break order):

```
primary:    (ρ̇, λ̇) = (ρ, 0)
fallbacks:  (0, λ) ; (Jρ, 0) ; (0, Jλ)
```

Convert each to particle velocities:

```
v0 = −(m2/M)·λ̇ − (m1/M01)·ρ̇
v1 = −(m2/M)·λ̇ + (m0/M01)·ρ̇
v2 =  (M01/M)·λ̇
```

Then project out COM drift and angular momentum:

```
w⁽⁰⁾ = v
c = Σ mᵢ w⁽⁰⁾ᵢ                    w⁽¹⁾ᵢ = w⁽⁰⁾ᵢ − c/M
β_L = L(w⁽¹⁾)/I                   w⁽²⁾ᵢ = w⁽¹⁾ᵢ − β_L·J rᵢ        where L(w) = Σ mᵢ(rᵢ × wᵢ)_z
wᵢ = w⁽²⁾ᵢ / √(‖w⁽²⁾‖²_m)
```

**Seed selection (R-82):** among the seeds with `‖w⁽²⁾‖²_m > ε_w` (default `1e−10`), **take the
largest `‖w⁽²⁾‖_m`**, for conditioning; break ties by seed order. Emit `DEGENERATE` only if no seed
qualifies.

**On the canonical-frame decode the rule is decided on `z_α`** (TASK-M2-09). There `I = 1`, and the four norms are
exactly `cos²α`, `sin²α`, `cos²α·sin²α` and `cos²α·sin²α`: the largest is `(ρ, 0)` where `α ≤ π/4` (a tie keeps it)
and `(0, λ)` elsewhere, and it is at least ½, so no decoded configuration is `DEGENERATE` by the seed rule. Every
registered α link maps `z_α = 0` to `π/4` and increases, so `α ≤ π/4` exactly where `z_α ≤ 0`, and the decoder reads
that sign from the control's bits: one input takes one seed at every precision and on every backend
(`principia_dd_decoder.md` §2, §5 test 12). Comparing the computed norms would fork near `α = π/4`, where the two seeds
are orthogonal and the momenta would differ at O(1).

**(iii) Mix to the target kinetic energy:**

```
K* = E − U          (or K(t) directly on the (Lz,K) chart)
if K* < K_min:  terminal
a = √(2·(K* − Lz²/(2I)))
vᵢ = vᵢ^(L) + a·wᵢ                pᵢ = mᵢ·vᵢ
```

On the `(Lz, E)` chart at `ε ≠ 0`, this `U` is §2.1's Newtonian `U` (R-791).

*10 Nov 2026 (R-934, R-937, the owner's answers on RQ-764 and RQ-771):* two changes are ruled here and built by the
chart plans' N10 and N13; until they land the construction is as this section gives it. The seed: the expansion seed `w
= r/√I` is the seed on every chart that uses this construction, Burrau's included, in place of (ii)'s largest-norm rule
(R-82), which stays as a named option with its seam declared; every image it changes is a recorded conversion, updated
under R-931. The second sheet and the clamps of the chart's edges are defined below, (iv) to (vi) (TASK-M2-95,
REQ-CHART-195; *was:* "The second sheet: `vᵢ = vᵢ^(L) − a·wᵢ`, the same expression with `a` negated, sheet 1 bitwise the
construction above; sheet 2 at `L_z` is the time reversal (`p → −p`) of sheet 1 at `−L_z`, and with the seed `w = r/√I`
it is the whole system contracting, `I` decreasing.").

**(iv) The second sheet** (defined per R-72, REQ-CHART-195; R-937 (i), its meaning read from R-934's seed). The
construction takes a sheet word `σ ∈ {1, 2}` beside `(L_z, K*)`:

```
vᵢ = vᵢ^(L) + a·wᵢ     (σ = 1)           vᵢ = vᵢ^(L) − a·wᵢ     (σ = 2)           pᵢ = mᵢ·vᵢ
```

with `vᵢ^(L)`, `w` and `a` exactly as (i) to (iii). Sheet 1 is (iii), bit for bit; sheet 2 negates the one product
`a·wᵢ` and changes nothing else. Both meet the three checks below: `w` carries no momentum and no `L_z` after (ii)'s
projections, so `Σpᵢ = 0` and `L_z(p) = L_z` on either sheet, and `⟨v^(L), w⟩_m = ω·L(w) = 0`, so
`K = K_min + a²/2 = K*` whichever sign `a` takes. Because `v^(L)` is linear in `L_z`, `a` depends on `L_z²` and `w` does
not depend on `L_z`, **sheet 2 at `L_z` is exactly the time reversal of sheet 1 at `−L_z`**: the same positions, `p → −p`.
These are real systems, distinct from sheet 1's (their forward fates are sheet 1's pasts). With R-934's seed `w = r/√I`,
the uniform dilation, `dI/dt = 2Σmᵢ rᵢ·vᵢ = ±2a·√I` (the rotation adds nothing, `rᵢ·J rᵢ = 0`): sheet 1 is the whole
system expanding, `I` increasing, and sheet 2 the whole system contracting, `I` decreasing, on both sides of `z_α = 0`.
Under R-82's legacy rule, kept as a named option (R-934), "contracting" names the inner pair's dilation where `z_α ≤ 0`
and the third body's elsewhere. The two sheets meet where `a = 0` (`K* = K_min`): that is the **fold**, where the unfold
turns. Wherever `a = 0` is selected (the infeasible arm below; the clamp below the floor, (v)), the sheet word is not
read and the momenta are bitwise the same whichever sheet is asked for (R-937, C1). The sheet word joins the pixel's
record and the decode's inputs (R-937); inside `[0,1]²` every pixel is sheet 1, so nothing inside the square changes.

*The unfold on the chart* (§5.4's **unfold** kind). On the warp (§2.1), a pixel's `s` maps to its sheet's `s'` by the
triangle wave of period 2: with `r = s − 2⌊s/2⌋ ∈ [0, 2)`, `s' = r` on sheet 1 where `r ≤ 1`, and `s' = 2 − r` on sheet
2 where `r > 1`; so past `s = 1` the chart shows `s' = 2 − s` on sheet 2, and past `s = 0` `s' = −s` on sheet 2. Under
the linear option, and on `Φ_θ,L_z` (§4.5) at its held `K`, the same map acts on `s̃ = (1 + L_z/L_max)/2` at the pixel's
own `L_max = √(2·I·K*)` (`K* = E − U` with the pixel's own Newtonian `U` and `I`, R-471 E3, R-791; the held `K` on
`Φ_θ,L_z`), and `L_z' = (2s̃' − 1)·L_max`: `L_z` is reflected about the parabola at the pixel's own `E` and the sheet
flipped, the same period, 2 in `s̃`, which is 4 in `L_z/L_max`. It applies only where `K* > 0`; below the floor it is
(v)'s. The primary range of an unfolded axis is one period, two sheets, centred on the square as R-655 centres a repeat:
`s ∈ [−½, 3/2)` (`s̃` likewise), whose sheet-2 parts `[−½, 0)` and `(1, 3/2)` show sheet 2's `s' ∈ (0, ½]` and
`(½, 1)` once each; a pixel past it is a repeat of one inside it (§5.4), on a tilted view only where the tilt adds no
hidden offset along the unfolded axis, `N_⊥ = 0` for that axis's component of §3.3's `N` (`N_a` where the unfolded axis
is `a`, `N_b` where it is `b`), decided exactly on the stored angles; elsewhere it is its own system, counted (§5.4,
"The unfold's period on a tilted view"; *was:* "a pixel past it is a repeat of one inside it (§5.4).", on every view;
*was (physics review 5913253688 of PR #546, F2):* "hidden offset along the unfolded axis, `N_a = 0`, decided exactly on the stored angles").

**(v) The clamp below the floor** (defined per R-72, REQ-CHART-195; R-937 (ii) and (iii)). Under the linear option on
`(L_z, E)` and `(L_z, K)`, and on `Φ_θ,L_z`, a pixel with `K* ≤ 0` and `L_z ≠ 0` lies below the floor at a non-zero
spin, where REQ-ENC-030 rejects (it would change `L_z`'s class). There the drawn extension is **the rigid rotation at
the pixel's own `L_z`**: `L_z` is kept exactly and the energy is moved to the floor,

```
E' = U + L_z²/(2I)        (K' = L_z²/(2I) on (L_z, K); on Φ_θ,L_z the held K is set to K' = L_z²/(2I))
a = 0 selected directly,  pᵢ = mᵢ·vᵢ^(L),  vᵢ^(L) = (L_z/I)·J rᵢ
```

each pixel reading its own `U` and `I` (R-471 E3), `U` the Newtonian `U` (R-791), and on `Φ_θ,L_z` its own shape. Its
**metric is `|ΔE|` at fixed `L_z`** (equivalently `|ΔK*|`): of the feasible region's points at the pixel's own `L_z`,
the floor `E = U + L_z²/(2I)` is the nearest in `E`. It is written beside REQ-ENC-030's metric (`|ΔL_z|` at fixed `E`,
(vi)), never as that metric; its precedents are §2.1's held-`L_z` floor and §4.5's held-`K` floor clamp (R-709), which it
extends to a non-zero `L_z`. The clamp arm takes no sheet input, and its momenta are bitwise equal, at f32 and at f64, to
the infeasible arm's (`spin = L_z`, `a = 0`) at the same `L_z` and configuration (C1); it meets this section's three
checks to machine precision with `K* = K'` (C2); the `L_z = 0` column decodes to `p = 0`, compared by `==`, which is
REQ-ENC-030's apex projection, and every clamped pixel keeps its own `sign(L_z)`, read through REQ-ENC-036's zero band
(C3). **Membership** is decided on the computed `K*`'s sign together with the exact test below (`2·I·K* < L_z²`, no
rounding at the decision), so each input takes one branch at each precision and on each backend (C4). On a chart with
an `L_z` axis (`(L_z, E)`, `(L_z, K)`, `Φ_θ,L_z`): infeasible with `K* ≤ 0` and `L_z ≠ 0` is this clamp; infeasible with
`K* > 0` is past the parabola, the unfold's or (vi)'s; `K* < 0` with `L_z = 0` is the rest start, (vi)'s apex, which
this clamp also gives at `L_z = 0`. **On a chart whose `L_z` is held** (a lone or mixed `K`, `KE`, `E` or virial axis,
§2.1; §4.5's linear-mode `K` axis at a held `L_z`), every infeasible pixel, `0 < K*` included, takes this clamp at the
held `L_z`: there is no `L_z` axis to unfold along, and (vi) would move the held `L_z`; R-709's held-`L_z` floor is its
precedent (physics review 5910557865 of PR #546, F4). *Was:* "(C4): infeasible with `K* ≤ 0` and `L_z ≠ 0` is this
clamp; infeasible with `K* > 0` is past the parabola, the unfold's or (vi)'s; `K* < 0` with `L_z = 0` is the rest
start, (vi)'s apex, which this clamp also gives at `L_z = 0`." Under a tilt that moves the
shape a clamped pixel is the construction at its own configuration (C5). A clamped pixel is a system and is drawn and
integrated as one; the Inspector and hover show its own `E' = U + L_z²/(2I)` and class, never the row's `E`, with a
"clamped to the floor" flag, and the debug view shows the clamp kind (C6); it stays out of the count (§5.4; R-932); and
a click or re-centre on it never locks the drawn system while showing the row's `E`: the lock is refused, as
REQ-ENC-030 refuses, a clamped area being no ground for the centre or a lock (C7; R-942). REQ-ENC-030's class rule binds
lookups and the lock, never this drawn extension (`principia_inverse_encode_contract.md` § "Chart-aware validation"):
where `L_z² ≥ −2IU` the clamp draws a system with `E' ≥ 0` under a row labelled `E < 0`, and since no bound system with
that spin exists at that shape, none is misrepresented. The conditions C1 to C7 are the physics reviewer's, recorded
in full in R-937. **The seam along `E = U`** is real, forced and declared (§5.4's table): just above it the unfold or
the projection shows near-rest systems, just below it the clamp a rigid rotation with kinetic energy `L_z²/(2I)`; it
grows with `|L_z|` and vanishes at the apex. Just above `E = U` the unfold's period in `L_z` falls below a pixel, so a
band of outcome noise is expected there at large `|L_z|`; no pixel is averaged across sheets. A pixel whose `L_z²` has
no finite float at the chart's precision stays not a system, §2.1's one exception, a validity failure.

**(vi) The clamp where REQ-ENC-030 projects** (R-937; REQ-ENC-030's metric named as the clamp's here). Where an edge
declares **clamp** and REQ-ENC-030 projects, the clamped pixel is that projection, the nearest valid point in its
metric, `|ΔL_z|` at fixed `E`: where `K* > 0`, `L_z` onto the parabola keeping its sign, `L_z' = sign(L_z)·√(2·I·K*)`,
the rigid rotation at `±L_max` with `a = 0`; where `K* < 0` and `L_z = 0`, the rest start (the apex); and on the warp
below `t = 0`, `t` projected to 0, the rest start at the pixel's own configuration (`L_max(0) = 0`, so no class changes,
the contract's own sentence). On the warp's `s` edges at `t > 0` the projection is `s` clamped to `[0, 1]` at the
pixel's own `t`. These are the clamps of §5.4's table wherever they project; (v) is the clamp where REQ-ENC-030 rejects.

*14 Nov 2026 (R-934; TASK-M2-93):* the seed has landed. The construction takes the expansion seed `w = r/√I` on every
chart that uses it, Burrau's included; the seed selection paragraph and its `z_α` paragraph above are R-82's named
legacy option, its seam declared (`principia_dd_decoder.md` §2, §3.4). The second sheet waits for N13.

**The infeasible pixel's label** (defined per R-72). A pixel with `K* < K_min` is tagged
`DEGENERATE(INVARIANT_INFEASIBLE)`, one of `DEGENERATE`'s enumerated causes (§0.7; `principia_dd_decoder.md` §2, "From
totality"), as `η_E`'s `E* < U` is `DEGENERATE(ENERGY_INFEASIBLE)` (`principia_dd_decoder.md` §3.7). Its
`sample_descriptor` (`principia_dd_simstate_payload.md` §2) records the reason as `state = decode_failed` (5),
`detail = 3` (other): no momentum realises `L_z` with less kinetic energy than the rigid rotation's, so the decoder
produces no physical IC there, which is what `decode_failed` records, and none of `detail`'s codes 0–2 (non-finite
output, degenerate configuration, invalid mass construction) describes it. It is a decode failure, not a dynamical
outcome: the sample is terminal at decode, kept, never dropped. Its momenta are finite, `a = 0` selected, so
`pᵢ = mᵢ·vᵢ^(L)`; nothing integrates them. The exception: where `L_z²` or `2K*` has no finite float at the decode's own
precision (§2.1's exception, §4.5), nothing bounds `L_z·r`, so `L_z` is selected out too and `pᵢ = 0`, finite as every
decode of a finite input is (applied per R-369, the physics reviewer's call on #388; *was:* no exception). Such a pixel
is always infeasible, so no feasible pixel is touched. The label is the decode's, for an infeasible input reaching the
construction (a lookup, an edge switched to blank); an edge that declares unfold or clamp maps its pixel before the
decode (§5.4), so the decode it reaches is feasible and untagged (R-932, R-937). The exact test is exactly `K* < L_z²/(2I)`, decided as
`2·I·K* < L_z²` with no rounding at the decision, so one input takes one branch of that test at every precision and on
every backend; `K* = K_min` is feasible, with `a = 0`. The bound on `2K*` is each precision's own range, not a common
cap: `K*` in `[2¹²⁷, 2¹²⁸)` has no construction at f32 and has one at f64, as §2.1's exception is at the chart's own
precision (*was:* "so one input takes one branch at every precision and on every backend").

**Its precedence.** The decode runs mass → config → momentum, and a decode an earlier block has already tagged keeps
that tag (`principia_dd_decoder.md` §3.7): a pixel the mass block tags `DEGENERATE(M01_TINY)` keeps that tag, whatever
its `(L_z, K*)`. Within the momentum stage, infeasibility is decided on `(L_z, K*)` alone and takes precedence over the
seed rule's `DEGENERATE`, which a pixel carries only where its `(L_z, K*)` is feasible.

**Verification that can fail:** assert `Σpᵢ = 0`, `Lz(p) = Lz_target`, and `K(p) = K*`, all to
machine precision, over random `(u,v)`. Three independent constraints; the construction should hit
all three exactly.

---

## 3. Shape-sphere chart `(θ, φ)`

*[mirror-step removal (R-865, R-898, R-919): rewritten 14 Nov 2026 by TASK-M2-107 to the decoder TASK-M2-57 builds, with R-898's periodic `β` and R-919's one `β` link; in force when TASK-M2-57 merges, having passed R-898's conversion test; until then each *Was* below describes the built decoder on `main`, and R-82, R-438 and R-544 stand. In the target, `C` has no mirror step, latent `β` is a periodic angle, `β = z_β` read modulo 2π, a lower-half state encodes to `β ∈ (π, 2π)`, and every chart, the shape sphere included, decodes by one path. 17 Nov 2026 (R-1020, RQ-824 (b)): the periodic link is the offset `β = z_β + π/2` (`principia_dd_decoder.md` § 3.2), so `z = 0` stays the golden IC; the identity just stated is its Was. This section is rewritten to that target, each changed passage keeping its *Was*; the definitions are `principia_dd_decoder.md` § 3.2 (REQ-DEC-056). Was (this marker): "mirror-step removal, ruled 8 Nov 2026 (R-865, on RQ-739): the decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the owner's bitwise check (every preset, saved chart and golden with `β ∈ [0, π]` reproducing unchanged), which meets the link `β = π·σ(z_β)`: RQ-750, on which the task waits. This section changes when TASK-M2-57 lands, and stands as written until then. Was: "[mirror-step proposal, 7 Nov 2026 (R-854; RQ-739): the owner's proposal to remove the decoder's mirror step, latent `β` over the full circle, would touch this section. A proposal, not applied; nothing here changes until it is ruled.]""]*

The quotient of configuration space by translation, rotation and scale. `Φ_S²(u,v) = (n(θ,φ),
m_fixed, p_fixed)` — masses and momenta held, only the **shape** varies.

**The held values** (defined per R-72, REQ-CHART-048; TASK-M2-08). `m_fixed` and `p_fixed` are the slice pin `z₀`'s:
its mass block through the shared `D_mass` (§0.1) and its momentum block through the shared `D_mom` (§0.3), under the
chart's link selection; `z₀`'s configuration block is not read, the shape replacing it. `p_fixed` is a body-frame
momentum, the Jacobi momenta `(p_ρ, p_λ)` relative to the frame with `ρ̃` on `+x`, unmirrored, `λ̃` in either
half-plane (R-544; R-538 J9's frame rule: the full shape sphere is read unmirrored, `ρ` along `+x`), and `Φ` expresses
it in the gauge of the configuration it builds, the inverse of the gauge the decode removes, decided on that
configuration (§0.4, §0.5): `(p_ρ, p_λ)` turned by `R(φ)` with `C`'s `φ = atan2(ρ_y, ρ_x)` (0 at `ρ = 0`, R-437), and
scaled by `I^{−1/4}`, the particle momenta then rebuilt by §0.3's map. So `p_fixed` is taken after the fibre-phase
rotation, which the decode then undoes. The decode runs the shared `C`, which has no mirror step (R-865; *was:* "with its
mirror step not taken (R-544): `C(Φ(u, v))` so taken has"), the one decode path of every chart: `C(Φ(u, v))` has masses `m_fixed` and momenta `p_fixed` at every pixel, to rounding, and its shape is the
pixel's own signed `n̂`. A point and its mirror `(θ, π − φ)` decode to two systems: mirror images of each other
exactly where `p_fixed` is its own mirror image (`p_ρ,y = p_λ,y = 0`, released from rest included), and otherwise
different systems (R-544 items 2 and 5). `I` and the fibre phase `φ_f` (§3.2) are chart parameters and pure gauge:
they change `Φ`'s output, never the decode's, except at `ρ = 0` (`BC₀₁`, `n = (−1, 0, 0)`), `C`'s seam, where R-437's
`φ = 0` leaves `φ_f` in the decode's configuration, turned by `φ_f` against momenta that are not, so `L_z` depends on
`φ_f` there. Their defaults are `I = 1` and `φ_f = 0`, at which `Φ`'s configuration is on the decode's section, `I = 1`
and `ρ̃` on `+x`, on either hemisphere. *Was (R-141's gauge, removed for the shape sphere by R-544):* "`p_fixed` is a
canonical-frame momentum, the Jacobi momenta `(p_ρ, p_λ)` relative to the body frame (`ρ̃` on `+x`, `λ̃` in the upper
half-plane)"; "`(p_ρ, p_λ)` mirrored `y → −y` where `C` mirrors"; "`C(Φ(u, v))` has masses `m_fixed` and momenta
`p_fixed` at every pixel, to rounding, and a point and its mirror `(θ, π − φ)` decode to one system (R-141)"; "at which
`Φ`'s configuration is on `C`'s section on the upper hemisphere".

### 3.1 Forward map (already implemented as `shape_vec`)

```
a = ‖ρ̃‖²    b = ‖λ̃‖²    I = a + b
p = ρ̃ₓ λ̃ₓ + ρ̃_y λ̃_y                 (dot)
q = ρ̃ₓ λ̃_y − ρ̃_y λ̃ₓ                 (standard 2D cross, ρ̃ ∧ λ̃ — R-14)
n = (u, v, w)/I = ( (a−b)/I , 2p/I , 2q/I ) , normalised
```

This is the project's one shape-sphere convention (R-14), the IC Inspector's: `u = ‖ρ̃‖² − ‖λ̃‖²`, `v = 2ρ̃·λ̃`,
`w = 2(ρ̃ ∧ λ̃)`. `L⁺` (`w = +1`) is the equilateral triangle with bodies 0 → 1 → 2 anticlockwise, and the collision of bodies
0 and 1 (`ρ̃ = 0`) is at `n = (−1, 0, 0)`. An earlier version of this section took `q` as the negative of the cross, which put
`L⁺` and `L⁻` at opposite poles from `principia_dd_integrator.md` §3.7. A `shape_vec` built to that version has its third
component negated relative to this one.

### 3.2 Inverse — closed form

Given `n` on the sphere, moment of inertia `I` and fibre phase `φ_f`:

```
a = I(1 + n₀)/2      b = I(1 − n₀)/2      p = I·n₁/2      q = I·n₂/2
                                          (p² + q² = ab holds identically)

ρ̃ = √a·(cos φ_f, sin φ_f)
λ̃ = √b·(cos ψ,   sin ψ)        with  ψ = φ_f + atan2(q, p)
```

**Evaluating the smaller of `a` and `b`** (applied per R-369, TASK-M2-08). It is computed as
`I(n₁² + n₂²)/(2(1 + |n₀|))`, which equals `I(1 − |n₀|)/2` on the unit sphere (`1 − n₀² = n₁² + n₂²`); the larger is
`I(1 + |n₀|)/2` as written. Near `n₀ = ±1`, `n₀ = cos r` rounds to `±1` once the polar radius `r` is below about
`10⁻⁸`, so `1 − |n₀|` loses `r` while `n₁` and `n₂` keep it: evaluated as written, the round trip misses by up to `r`
there, about `10⁻⁸`, against the `10⁻¹³` bound (R-181). The form through `n₁² + n₂²` keeps `ab = p² + q²` to rounding.

**The `atan2(q, p)` sign is correct given §3.1's `q`** (the standard cross). Under the earlier negative-cross `q` it
was `atan2(−q, p)`. Verified to machine precision (round trip over 2,000 random masses and shapes, max error 3e−15).
`φ_f` is the fibre phase, not the polar angle `φ` of §3.3.

**`ψ` at the poles (R-437).** At `n₀ = ±1`, `ab = 0`, so `p = q = 0` and `atan2(q, p)` is at its pole: there the term is
0, whatever the signs of the zeros, so `ψ = φ_f`, chosen by a select on `p = 0` and `q = 0`, not obtained from
`atan2(0, 0)` (`principia_dd_decoder.md` §3.2, "Every `atan2` at a possible pole"). At `n₀ = 1` (`b = 0`) `ψ` does not
reach the positions; at `n₀ = −1` (`a = 0`, the collision `ρ̃ = 0`) it sets `λ̃`'s direction.

Then unweight (`ρ = ρ̃/√μ_ρ`, `λ = λ̃/√μ_λ`) and reconstruct positions per §0.2.

**Round-trip test that can fail:** `shape_vec(decode(u,v)) == n(u,v)` to ~1e−14.

### 3.3 The chart map

Two options; state which is used.

**Spherical coordinates** (simple, has poles). Chart coordinates are written `(s, t) ∈ [0,1]²` here, because `u`
and `v` are shape components in this section. `(s, t)` is the `(u, v)` of `Φ` elsewhere in this document, with the
bottom-left origin and Y-up (`principia_coordinate_conventions_note.md`). By R-14:
```
θ = 2π·s          azimuth in the (u, v) plane, horizontal axis, 0..2π
φ = π·(1 − t)     polar angle from +w, vertical axis, 0..π — L⁺ (φ = 0) at the top
n = (sin φ cos θ, sin φ sin θ, cos φ)
```

**Exponential map about a centre `n0`** (no poles in view; the trig is where curvature lives):
```
n(u,v) = cos(r)·n0 + sin(r)·(d/‖d‖)      d = (2u−1)·s·e1 + (2v−1)·s·e2 ,  r = ‖d‖
```
with `(n0, e1, e2)` an orthonormal frame. **This is the nonlinear chart** — use it wherever a
linearised decoder is being tested, since an affine chart makes the curvature term identically
zero. **The radius is bounded at π (R-503, closing RQ-371):** a pixel with `r = ‖d‖ > π`, which `[0,1]²` reaches
where `s√2 > π`, is past the antipode of `n0`, where the map wraps; it is the domain's end, not a system, out of the
count and the total (§5.4). Inside `[0,1]²` such a pixel is the domain's end by design, as is one exactly on `r = π` if
REQ-CHART-078 makes it so: it draws blank and logs no decoder-bug warning (R-554, ruling RQ-470). *15 Nov 2026 (R-932,
R-938; TASK-M2-95):* past `r = π` the map is no longer the domain's end: the edge declares §5.4's repeat map, the
formula continuing, the point at `(r, ê)` being the one at `(2π − r, −ê)`, a repeat resolved to its canonical point in
`r ≤ π` and not counted, inside the square and past it; past the square the formula continues, affine, to `r = π`. The
chart addendum's N14 builds it; until then the sentences above describe what is built and are its *Was*. The pixels with `r ≤ π` are the map's **cap**, the part of the sphere in view. Every pixel
exactly on `r = π` maps to the one point `−n0`; how such pixels count is REQ-CHART-078's definition (R-72), "Pixels on
`r = π`" below: they are a system, not the domain's end, so they do not draw blank. The fixtures avoid that circle.
*Was:* "how such pixels count is a definition TASK-M2-28 writes (REQ-CHART-078, R-72), and the fixtures avoid that
circle."

**No polar buffer (R-12, premise corrected by R-14).** An alternative map kept a buffer $\varepsilon$ off each pole,
$\theta(u) = \varepsilon + (\pi - 2\varepsilon)u$ and $\varphi(v) = 2\pi v$ in its own naming (θ polar, on the horizontal axis),
"to avoid the collision singularities". It is rejected. Under this convention the poles are the Lagrange
configurations at equal masses (R-549, RQ-355), which are regular points of the flow, and every binary collision lies on the equator ($w = 0$, §3.4).
So nothing singular sits at a pole, and the map runs the full $\varphi \in [0, \pi]$.

**The whole sphere (R-544).** The chart is the whole sphere, everywhere: Explore, the Atlas, traces, Listen, the
state map, prints and the batch job. Pixel `(θ, φ)` decodes to the system whose **signed** `n̂` is that point, and
R-14's `φ = π·(1 − t)`, pole to pole, is the only φ span. The two hemispheres are the triangle's two orientations and
the equator its collinear configurations, so every pixel is its own system, and a trajectory crosses the equator at
every syzygy. The mirror `(θ, φ) ↦ (θ, π − φ)`, `w → −w`, is a symmetry of fates, not an identification: where every
coordinate the chart holds fixed is its own mirror image (released from rest is), the mirror-invariant fields (fate,
`t_end`, spread) are exactly symmetric across the equator and the signed ones (`L_z`, the winding) exactly
antisymmetric, bitwise where the integration is mirror-exact, else to rounding (R-544 §4): the run from the exactly
mirrored state `Π·x` is the run mirrored, bitwise, while the two pixels of a pair decode to mirror images only to the
decode's rounding, so on a chaotic run their fates are compared on a named fixture the test shows to stay close over
its horizon, and the statistics' "counted twice" holds for mirror-invariant classes, a signed class mapping to its
mirror class (applied per R-369; physics review 5603309678 of PR #273, P2); where one is not (a held `p_fixed` with a non-zero `y`-component, or a fixed `L_z ≠ 0`), the
hemispheres are different systems and the symmetry does not apply (REQ-CHART-082). No hemisphere is drawn alone, there
is no hemisphere toggle, and nothing is folded, visible or not. The spherical map's `system_image` is **bijective**:
θ's period and the poles are its only identifications, R-407's periodic and pole-crossing types on edges that no pixel
centre lies on, not a fold (`principia_chart_decoder_contract.md` Part 5); `DoubleCover` is the full-range Burrau
chart's (§4.5, R-157). The mirror twin survives only as information, in the initial-condition constructor
(`principia_render_gui_spec.md` §G8). *Was (R-141, R-113 and R-412, withdrawn for the shape sphere by R-544):* the
chart a 2-to-1 cover, `(θ, φ) ∼ (θ, π − φ)`, the canonical decode keeping `w ≥ 0`; one hemisphere drawn and said so,
or both with the redundancy flagged, through the hemisphere toggle in the Manifold view's Chart section, with one
hemisphere drawn φ's span remapped to end at the equator; the spherical map's `system_image` n-to-1 with n = 2.

**The frame's winding** (the signed field beside `L_z` above; `kernel::winding`). The net turn, in radians, of
`ρ = r₁ − r₀`, the separation of bodies 0 and 1, in the inertial plane. It is not `θ̃` (dd_integrator §3.7), the shape
sphere's unwrapped longitude, which the mirror leaves unchanged (`u` and `v` are mirror-invariant, only `w` turns).
- **Start:** `W = +0` at the IC.
- **Cadence:** every state the march leaves, each internal step's (what the wrapper's cadence sees after its
  projection), from the IC, then one closing step to the state the `SimState` stores (the occupant hands that state
  back, and it can differ from the last one the cadence saw by a rounding). The sum telescopes to the net turn of `ρ` whatever the
  cadence, so long as no step turns `ρ` by `π` or more; the finest cadence the run has aliases least. It is read from a
  run's states, not stored in the `SimState`.
- **Per step**, from `ρ_prev` to `ρ`: `c = ρ_prev × ρ` and `d = ρ_prev · ρ`. Where `c ≠ 0`, the step adds
  `atan2(c, d)`, in `[−π, π]` (±π only by rounding, for a tiny `c` against a negative `d`), with a zero `d` taken as `+0` (pitfalls §11). Where `c = 0`, of either sign, the step
  adds `+0`. That covers three cases: `ρ` did not turn (`d > 0`); `ρ_prev` or `ρ` vanished, bodies 0 and 1 together
  (`d = 0`); or the step turned `ρ` by exactly `±π` (`d < 0`). That last one is the tie, which has no sign of its own.
- **At a collision or an escape** the march stops at its latched state, the one the `SimState` stores, and the
  winding is that state's.
- **Under the mirror `Π`** the run from `Π·x` winds `−W(x)`, bitwise where nonzero and `+0` in both where zero, with
  no case excluded. `Π` negates `c` exactly where it is nonzero (a zero `c` is `+0` in both runs) and leaves `d`'s bits
  unchanged; `atan2` is odd in its first operand; and a sum of negated terms is negated, an exact-zero sum `+0` in both.
  A difference of principal-value angles is not odd. On the negative `x`-axis `ρ_y = +0` in both runs, so the angle
  is `+π` in both, and the mirrored deltas differ from the negated ones in their last bits. A tie left at
  `atan2(+0, d < 0) = +π`, the wrap that dd_integrator §3.7 closes at `+π`, winds `+π` in both runs.

**The exponential map's `system_image` is computed from its cap (R-503, closing RQ-371; R-544).** The cap is the
part of the sphere in view, `[0,1]²`'s pixels with `r ≤ π`; past `r = π` is a repeat, and past the square the formula
continues to `r = π` (§5.4's table, R-932, R-938; *was:* "past `r = π` is the domain's end, and past the square the map
is bounded (§5.4)"); the value is the square's cap's, the extension adding no multiplicity (R-408). Every pixel of the cap is its own system and its mirror `w → −w` another (R-544), so the value
is **bijective** whether the cap lies within one hemisphere or straddles the equator; the pixels exactly on `r = π`,
which all map to `−n0`, count as REQ-CHART-078 defines. The chart computes the value whenever `n0`, `e1`, `e2` or `s`
change, and its label shows it. *Was (R-503's per-pixel fold, withdrawn by R-544):* where the cap straddles the
equator, n-to-1 with a per-pixel fold, a pixel counting 1 if its `w ≥ 0` or its mirror lies outside the cap, else 0,
its representative the pixel with `w > 0`; bijective only within one closed hemisphere.

**Pixels on `r = π` (defined per R-72, REQ-CHART-078; TASK-M2-28).** A pixel whose computed `r` is π, the float type's
π exactly, the `r = ‖d‖` the map computes from the pixel's `(u, v)`, lies in the cap, which is `r ≤ π`, and is a
system: R-503 bounds the radius at π and makes only what lies past it the domain's end. Every such pixel maps to the one
point `−n0`, one system, so together they count **once**, as each system counts once (`principia_chart_decoder_contract.md`
Part 5): a counted total over a grid of pixels is the number of its pixels with `r < π`, plus 1 where any of its pixels
lies on `r = π`, and nothing for the pixels past it. Each of them draws that one system, the decode at its own
`(u, v)`, which reaches `−n0` to rounding (`sin r` at the float type's π is not 0). The circle is the cap's own edge,
where every direction of `d` reaches the one antipode, as each of the spherical map's pole rows reaches one pole: an
identification on a set of measure zero, not a fold, with no mirror in it (R-544), so the value stays bijective. Pixel
centres land on the circle only for a measure-zero set of `s`; the rule decides that set, and REQ-CHART-075's fixtures
keep every pixel centre more than `10⁻⁶` from it. *Was (PR #271, under R-503's fold, kept as the record):* the group
counting 1 or 0 "by the per-pixel rule applied to `−n0`".

**Pixels on `r = π` under a tilt (defined per R-72, REQ-CHART-090; TASK-M2-36; physics review 5719740344 of PR #349,
P1).** The count above is the untilted view's. A tilted pixel lies on the circle when the `r` computed from its tilted
`(a, b)` is π, and it keeps its own hidden offsets `d`, so two such pixels share the shape `−n0` and are one system only
if their `d` agree: a tilted view's pixels on `r = π` count **once per distinct `d`**. Where `N_a = 0` and `N_b = 0`,
decided exactly on the stored angles as the repeats are (`principia_chart_decoder_contract.md` § "Repeats and the
canonical point"), the view adds no `d` that varies, so they count once together, whatever rounding leaves in the
computed offsets. Elsewhere two pixels' `d` are compared exactly as computed at the float type, every target's, with no
tolerance on them or on a computed `N`, as `r` itself is compared with π: two pixels on a chord along the plane's level
line, whose computed `d` are equal, stay one system, and `+0` equals `−0`. Pixels on the circle at different `d` have
different masses or momenta, so they are different systems (R-408).

**Under a tilt (R-473 T1, T2, J1; R-544).** Two pixels fold only when their full `(a, b, d)` decode to the same
system (`principia_chart_decoder_contract.md` Part 5's definition of `system_image`). Under a nonzero tilt a pixel at
window offset `(u, v)` maps to `(a, b, d) = R·(u, v, 0)`, and `d` is added to a momentum control or a mass logit (J1).
On the shape sphere a pixel and its mirror are distinct systems at any tilt, as they are untilted, so a tilted view is
one-to-one on either map, beyond the untilted chart's own identifications, and its pixels in view are the tilted view's
own (blank past a bounded edge, R-512). The chart recomputes the value whenever `n0`, `e1`, `e2`, `s`, `τ₁`, `τ₂` or the
tilt's targets change. The human's ruling on RQ-472, recorded as R-554, kept an equator fold on a tilted view where
`N_b = 0`; R-544 wins (R-613, closing RQ-508, as applied meanwhile): no fold anywhere, no equator fold and no pixel fold
on any view, tilted or not, and RQ-472's ruling keeps only its pole maps. *Was:* "that clash with R-544 is RQ-508,
R-544 applied meanwhile and R-554 leaving the clause unapplied." A view
whose map `(u, v) → (a, b, d)` loses rank (Lean each axis with one target at `|τ₁| = |τ₂| = 90°`) reports
`ray-degenerate` (R-554, ruling RQ-471 (a)): no fold is applied, there being no normal, every area weight is 0 (the
Gram factor), and the legend shows each share as undefined ("—"), never 0%; TASK-M2-36 builds and tests it. *Was:*
"The human's ruling on RQ-472 in `rulings_2026-10-21.md`, not yet recorded, keeps …" and "is RQ-471's: TASK-M2-36
neither builds nor tests that corner until it is ruled." *Was (applied per R-369 from PR #255's reviews, accepted by
R-559, withdrawn for the shape sphere by R-544):* the mirror pair of a tilted view, the pixel with the mirrored `(a, b)` and the same `d`,
kept its fold iff the tilt added no hidden offset that depends on the mirrored axis, `N_b = 0` for
`N ∝ (R·e_a) × (R·e_b)`, decided exactly on the stored angles (Tip as sheet `τ₂ == 0` or `m == 180`, Lean each axis
`τ₁ == ±90` or `τ₂ == 0`, two targets `τ₂ ∈ {0, ±180}`), and was otherwise bijective; a single `τ₁` kept n-to-1 with
n = 2, or the per-pixel fold.

**Projection.** The default is equirectangular: $\theta$ and $\varphi$ map linearly to the axes. It
distorts area near the poles but keeps coordinates readable. For quantitative area comparisons, offer an
equal-area alternative (Mollweide or Hammer–Aitoff), chosen with the projection selector in the Manifold view's Chart
section (render_gui_spec §G2, R-113). This matters because the Lagrange configurations sit
at the poles at equal masses and can be compressed to invisibility in equirectangular.

**The equal-area projection is Hammer's (Hammer–Aitoff), a display remap (defined per R-72, REQ-CHART-047;
TASK-M2-28).** Of the two the paragraph above offers, Hammer's is taken because both its maps are closed form;
Mollweide's forward map solves `2γ + sin 2γ = π sin ℓ` by iteration. It is a **display remap** of the spherical map's
render, not a chart map: it moves where each system is drawn, never which system a chart point `(s, t)` decodes to, so
switching projection changes nothing on the sim key and re-integrates nothing, and the chart's descriptors, its
`system_image` and what R-408 counts are the spherical map's. The equirectangular display is the identity, display
point `(x, y) = (s, t)`. Under Hammer's, with `(x, y) ∈ [0,1]²` the display point, bottom-left origin and Y-up
(`principia_coordinate_conventions_note.md`), its longitude `λ = θ − π` (θ = π on the vertical centre line) and its
latitude `ℓ = π/2 − φ` (`L⁺` at the top):

```
inverse (display → chart):  X = 2√2·(2x − 1),   Y = √2·(2y − 1)
                            shown only where q = (2x − 1)² + (2y − 1)² ≤ 1      (the 2:1 ellipse)
                            z = √(1 − q/2)                                      (= √(1 − X²/16 − Y²/4))
                            λ = 2·atan2(z·X, 2·(1 − q)),   ℓ = asin(z·Y)
                            (s, t) = ((λ + π)/2π, ℓ/π + ½)
forward (chart → display):  λ = 2π·s − π,   ℓ = π·(t − ½),   D = √(1 + cos ℓ·cos(λ/2))
                            X = 2√2·cos ℓ·sin(λ/2)/D,   Y = √2·sin ℓ/D
                            (x, y) = ((X/(2√2) + 1)/2, (Y/√2 + 1)/2)
```

Inside the ellipse `2(2z² − 1) = 2(1 − q) ≥ 0`, so `atan2` stays on its principal branch and `λ ∈ [−π, π]`. The map is
equal-area: every display area element covers the same solid angle, `4π` over the ellipse's area `π/4` of the display
square, `dΩ = 16 dx dy`, so equal areas of the display are equal solid angles of the shape sphere, the polar caps
included. A display point outside the ellipse shows no chart point: it is no pixel of the chart, drawn blank (R-470),
and never counted. A display pixel shows the sample whose tile holds its `(s, t)`, with no interpolation, one sample
per tile as the raster draws every view. The exponential map has no projection: it has no θ or φ axis.

**Past the unit square (R-407).** The spherical map's θ is **periodic** (it wraps, period 2π) and its φ
**pole-crossing**: past a pole it comes back down the other side with θ shifted by π, half its period (for `t > 1`,
`φ → −φ`; for `t < 0`, `φ → 2π − φ`; θ then wraps). So the sphere continues past `[0,1]²` with real neighbouring states,
each a system `[0,1]²` already holds; area statistics count each system once, so these pixels, outside θ's or φ's
primary range, are not counted (R-408, §5.4). The exponential map has no θ or φ axis, so its axes declare nothing and
are bounded (§5.4). There is no one-hemisphere view (R-544): φ is
pole-crossing at both edges, `t = 1` and `t = 0`, and a pixel at `(θ, π − φ)` inside the square is its own system,
never a repeat; TASK-M8-44 checks it (REQ-CHART-057). *Was (R-412, withdrawn by R-544):* "With the hemisphere toggle
drawing one hemisphere, φ's `[0,1]` span is remapped to run from the pole to the equator, and φ stays pole-crossing at
its pole edge; past the equator, the edge that is not a pole, its formula continues into the mirror hemisphere, whose
pixels are real systems, repeated (the fold, R-141). Its primary range is pole to equator, the chart's `[0,1]` span,
so that continuation is not counted (R-412, closing RQ-264; built by TASK-M8-44, REQ-CHART-057)."

**The shape sphere as phase portrait.** Here the rendering surface and the configuration space are the
same object. Each pixel is a starting shape $\mathbf n(0) \in S^2$, and its trajectory $\mathbf n(t)$
crosses the very surface being rendered. The collision points are singular points of the flow, the Euler
configurations are equilibria on the equator, and the Lagrange configurations are the poles at equal masses. The full
reduced phase space is $T^*S^2$, so different momenta at the same shape give crossing trajectories on the
map. A mixed-axis chart (§1.3) pairing a shape coordinate with a momentum lifts that degeneracy. Drawing a
trajectory on this chart is the hover trace and inspector of `principia_trajectory_viewing.md`.

### 3.4 Landmarks at known fixed coordinates

Useful as overlays and as tests. **Collision singularities** (two bodies coincident) are three
points on the equator; **Euler configurations** (collinear) lie on the equator between them;
**Lagrange configurations** (equilateral) are the two poles at equal masses; under the current masses `L±` sit at
`w = ±√(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)`, the poles only at equal masses (R-549, RQ-355). Their exact coordinates depend on the
mass ratios — compute them from §3.1 rather than hard-coding. The Euler configurations are the Euler central
configurations: roots of Euler's quintic in the mass ratios, mapped through §3.1 (R-126).

---

## 4. The Burrau family chart `(ν, K)` and friends

### 4.1 The discrete family — Euclid's parametrisation

For coprime `m > n > 0` with `m − n` odd:

```
a = m² − n²          b = 2mn          c = m² + n²
```

Every primitive Pythagorean triple arises exactly once, up to leg swap. Non-primitive triples $(ka, kb, kc)$ are
dynamically equivalent under the similarity symmetry, so only primitive ones give new shapes.

| (m,n) | triple | angles | a/b |
|---|---|---|---|
| (2,1) | (3,4,5) | 36.9°, 53.1° | 0.75 |
| (3,2) | (5,12,13) | 22.6°, 67.4° | 0.42 |
| (4,1) | (15,8,17) | 61.9°, 28.1° | 1.88 |
| (4,3) | (7,24,25) | 16.3°, 73.7° | 0.29 |

### 4.2 Positions and masses — Burrau convention

Right angle at the origin, normalised by hypotenuse, **each mass equal to its opposite side**:

```
r0 = (0,     0    )      m0 = c/(a+b+c)
r1 = (a/c,   0    )      m1 = b/(a+b+c)
r2 = (0,     b/c  )      m2 = a/(a+b+c)
```

Jacobi vectors for inner pair `(0,1)`:

```
ρ = ( a/c , 0 )
λ = ( −ab/(c(b+c)) , b/c )
```

**Sanity check that can fail:** at `(m,n) = (2,1)` this must give the classical Burrau
configuration, `(m0,m1,m2) = (5,4,3)/12`. Note this is the **normalised** form; the historical
convention uses masses `(3,4,5)` with a differently-scaled triangle. **State which is in use** —
they are the same system up to the scale gauge, but they are not the same numbers.

### 4.3 Continuous interpolation — the `ν` axis

Euclid's formulae hold for all real `m > n > 0`, so with `ν := n/m ∈ (0,1)`:

```
a ∝ 1 − ν²          b ∝ 2ν          c ∝ 1 + ν²
acute angle:  θ(ν) = arctan( (1 − ν²) / (2ν) )
```

Everything — positions, Jacobi vectors, masses — varies **smoothly** with `ν`. Primitive triples
sit at a countable set of `ν` values; the chart sweeps between them.

**The `(ν, K)` chart:** `ν` on one axis (triangle shape, Burrau masses following it), kinetic
energy `K` on the other via §2's construction. This is the *bifurcation strip* (§4.5) — the
right-triangle Burrau configurations are a **1D curve** inside a 2D map, which answers directly
whether the right-angle constraint is dynamically special or merely convenient.

### 4.4 Relaxations — each gives another chart

- **Mass simplex.** Fix the triangle, sweep `Δ₂ = {(m0,m1,m2) : Σmᵢ = 1, mᵢ > 0}`. The Burrau point
  `(c,b,a)/(a+b+c)` is one distinguished location. 2D by construction — barycentric coordinates map
  straight onto `[0,1]²` with a shear.
- **Rest start relaxed.** Replace zero momenta with §0.3 or §2.2.
- **Right angle relaxed.** Sweep the apex angle away from `π/2` at fixed side ratio.

### 4.5 The Burrau-family chart maps

Every Burrau view is an ordinary chart map $\Phi : [0,1]^2 \to Y_{\mathrm{Burrau}}$ into the shared
decoder, with $Y_{\mathrm{Burrau}} \cong Y_{\mathrm{shape}} \times Y_{\mathrm{mass}} \times Y_{\mathrm{mom}}$. The shape
coordinate is $\nu = n/m \in (0,1)$, with the normalised Euclid formulae
$a(\nu) = 1 - \nu^2$, $b(\nu) = 2\nu$, $c(\nu) = 1 + \nu^2$. There is no Burrau-specific integrator or
renderer: $(\nu, \mathbf m, \mathrm{mom}) \mapsto (\boldsymbol\rho, \boldsymbol\lambda, \mathbf m, \mathrm{mom}) \mapsto (\mathbf r_i, \mathbf p_i, m_i)$
through the shared canonicaliser.

**The Euclid plane.** With $m \in [m_{\min}, m_{\max}]$ (default $[1, 32]$) and $\nu \in [\nu_{\min}, \nu_{\max}]$
(default $[1/32, 31/32]$):

$$m(u) = m_{\min}\left(\frac{m_{\max}}{m_{\min}}\right)^{u}, \qquad \nu(v) = \nu_{\min} + (\nu_{\max} - \nu_{\min})\,v,
\qquad \Phi_{\mathrm{Euclid}}(u,v) = \big(\nu(v),\ \mathbf m_{\mathrm{Burrau}}(\nu(v)),\ \mathrm{rest}\big).$$

The decoder uses only $\nu$. $m$ is an **annotation axis**, a display and landmark coordinate for the
integer lattice and primitive-triple overlays (`principia_chart_decoder_contract.md` Part 5). Primitive
triples sit where $m, n \in \mathbb Z$, $m > n$, $\gcd(m,n) = 1$, $m - n$ odd.

**The acute-angle axis.** The acute angle $\theta \in (0, \pi/4]$ parameterises right-triangle shape
directly, and $\nu(\theta) = \sec\theta - \tan\theta$. For $(\theta, K)$, with
$\theta(u) = \theta_{\min} + (\theta_{\max} - \theta_{\min})u$ and $K(v) = K_{\max} v^{\gamma_K}$:

$$\Phi_{\theta,K}(u,v) = \big(\nu(\theta(u)),\ \mathbf m_{\mathrm{Burrau}}(\nu(\theta(u))),\ (L_z = 0,\ K(v))\big).$$

The variants are $\Phi_{\theta,L_z}$ (fix $K$, sweep $L_z$) and $\Phi_{\theta,\delta m}$ (below).

**The acute-angle charts, as built** (defined per R-72, REQ-CHART-024; TASK-M2-11). $\theta(u)$ is horizontal and
the parameter vertical, as on the strips below. $\nu(\theta) = \sec\theta - \tan\theta$ is evaluated as
$\cos\theta/(1 + \sin\theta)$, the same function (multiply through by $1 + \sin\theta$), which cancels nowhere on
$(0, \pi/4]$; it inverts $\theta(\nu)$ (§4.3) there, mapping $(0, \pi/4]$ onto $[\sqrt2 - 1, 1)$: the fold's end
$\theta = \pi/4$ is $\nu = \sqrt2 - 1$, the isosceles triangle ($a = b$), and $\theta \to 0$ is $\nu \to 1$ ($a \to 0$).
The system at $\nu(\theta(u))$ is the $\nu$ axis's embed (§4.3, §4.2). $\Phi_{\theta,K}$'s $K_{\max}$ and $\gamma_K$
are the invariant warp's (REQ-CHART-044, R-113), and its momenta §2.2's construction at $(L_z = 0, K(v))$, as on the
$(\nu, K)$ chart. $\Phi_{\theta,L_z}$ holds $K$ fixed and sweeps $L_z$ linearly over a band symmetric about 0, inside
the feasible $|L_z| \le \sqrt{2IK}$ ($I = 1$, §0.5):

$$L_z(v) = (2v - 1)\,\lambda_L\,L_{\max}(K), \qquad \lambda_L \in (0, 1], \qquad
\Phi_{\theta,L_z}(u,v) = \big(\nu(\theta(u)),\ \mathbf m_{\mathrm{Burrau}}(\nu(\theta(u))),\ (L_z(v),\ K)\big),$$

with $L_{\max}(K) = \sqrt{2IK}$ held to $L_{\max}^2 \le 2K$ in floating point as §2.1's warp holds it, so every
$(u, v) \in [0,1]^2$ is feasible, the edges included. $\theta_{\min}$ and $\theta_{\max}$ are REQ-CHART-045's
calibration; $K$ and $\lambda_L$, which give the $L_z$ range $[-\lambda_L\sqrt{2IK}, \lambda_L\sqrt{2IK}]$, are
REQ-CHART-046's. Both charts' energy is the construction's, so both forbid energy normalisation (§0.6).

**The two shape axes survey different sets (R-27: both charts kept, applied before any Burrau statistic).** The acute-angle axis $\theta \in (0, \pi/4]$ folds out
the leg swap ($\theta$ and $\pi/2 - \theta$ are the same triangle with the legs swapped). The Euclid plane's default
$\nu \in [1/32, 31/32]$ maps to $\theta \in (0, \pi/2)$, both leg orderings. Under the Burrau mass convention ($m_i$ =
opposite side) swapping the legs swaps two masses, so it is a body relabelling, a distinct labelled system. Keeping
both orderings surveys shape × labelling; folding surveys each shape once. Neither is wrong, but the two charts
disagree silently. Both are kept (R-27): label each chart with the quotient it covers (in its
`system_image` descriptor — the full range covers each shape twice, as two labelled systems: `DoubleCover`, R-27,
R-104), and don't take shape fractions from the full-range chart, which double-counts.

**The charts' `system_image` values** (defined per R-72, REQ-CHART-025; TASK-M2-11). The acute-angle charts
($\Phi_{\theta,K}$, $\Phi_{\theta,L_z}$, $\Phi_{\theta,\delta m}$), their $\theta$ range inside $(0, \pi/4]$, cover each
shape once under one labelling: **bijective**, the quotient "shape only". A $\nu$-axis chart (the Euclid plane, the
$(\nu, K)$ chart) is **`DoubleCover`**, the quotient "shape × labelling", where its $\nu$ range reaches both leg
orderings, $\nu_{\min} < \sqrt2 - 1 < \nu_{\max}$ (the leg swap is $\nu \mapsto (1 - \nu)/(1 + \nu)$, whose fixed point
$\sqrt2 - 1$ is the isosceles triangle), so that a shape appears in it twice, as two labelled systems; the Euclid
plane's default range does. A $\nu$ range on one side of $\sqrt2 - 1$ covers each of its shapes once: bijective. Each
chart's label shows its value and its quotient. The statistics guard refuses a shape fraction on a `DoubleCover` chart
(R-27), and on a ray-degenerate one, whose areas are not system fractions (`principia_chart_decoder_contract.md` Part
5); a bijective or n-to-1 chart admits it.

**Ternary mass plot.** Fix the geometry at $\nu_0$. Map $(u,v)$ to the simplex: $x = u$, $y = (1-u)v$,
$m_0 = 1 - x - y$, $m_1 = x$, $m_2 = y$ (0-based, R-22). Shrink toward the
barycentre by an interior buffer $\varepsilon_m$ (default $10^{-4}$):
$\mathbf m \leftarrow (1 - 3\varepsilon_m)\mathbf m + \varepsilon_m(1,1,1)$. Then
$\Phi_{\mathrm{mass}}(u,v) = (\nu_0,\ (m_0, m_1, m_2),\ \mathrm{rest})$, with the Burrau point
$(c, b, a)/(a+b+c)$ as an overlay marker.

**Bifurcation strips.** Shape on the horizontal axis, one parameter on the vertical. For $(\theta, K)$,
$\Phi_{\mathrm{strip},K} = \Phi_{\theta,K}$ above. For $(\theta, \delta m)$, blend the masses toward a target
(default equal masses):

$$\mathbf m(u,v) = (1 - v)\,\mathbf m_{\mathrm{Burrau}}(\nu(\theta(u))) + v\,\mathbf m_{\mathrm{target}}, \qquad
\Phi_{\mathrm{strip},\delta m}(u,v) = \big(\nu(\theta(u)),\ \mathbf m(u,v),\ \mathrm{rest}\big).$$

(Defined per R-72, REQ-CHART-027; TASK-M2-11.) The default target, equal masses, is
$\mathbf m_{\mathrm{target}} = (1, 1, 1)/3$. The triangle is the Burrau one at $\nu(\theta(u))$, its positions §4.2's
$\mathbf r_i$; only the masses are blended, and the Jacobi vectors are that triangle's under the blended masses,
$\boldsymbol\rho = \mathbf r_1 - \mathbf r_0 = (a/c, 0)$ and
$\boldsymbol\lambda = \mathbf r_2 - \mathbf r_{01} = \big(-\tfrac{m_1}{m_0 + m_1}\,\tfrac{a}{c},\ \tfrac{b}{c}\big)$, which
at $v = 0$ are §4.2's. $\Sigma m_i = 1$ on every row, as both ends sum to 1. The momenta are zero (rest), so the chart
allows energy normalisation, as the Euclid plane does.

**Past the unit square (R-407).** Each Burrau-family axis's extension type is a definition (R-72, REQ-CHART-055),
written in §5.4 by TASK-M2-11 and physics-reviewed; until it is written each axis is bounded, the default. (`ν` past
`(0,1)` gives `a = 1 − ν² ≤ 0` or `b ≤ 0`, a zero or negative mass, and the acute angle past `(0, π/4]` the same.) The
ternary mass plot's axes are affine in R-407's sense (its formula continues; the map itself, `m₁ = u`,
`m₂ = (1−u)v`, `m₀ = (1−u)(1−v)`, is bilinear), and its negative-mass area is not a system, drawn blank (R-470; §5.4). *Was:* "and the fallback hatches its
negative-mass area (§5.4)" (stale value, replaced by R-470).
The types are now written in §5.4 (TASK-M2-11, REQ-CHART-055). The acute angle's axis has its edges at `θ_min` and
`θ_max`, inside the domain `(0, π/4]`, so past each edge the formula first runs through real systems the chart does not
show: below `u = 0` the thin triangles of `θ ∈ (0, θ_min)`, and above `u = 1`, where `θ_max < π/4`, the shapes of
`(θ_max, π/4]`. The parenthesis above holds only past `θ = 0`: there `ν ≥ 1`, so `a ≤ 0`, a zero or negative mass. Past
`θ = π/4` the formula continues into the other leg ordering (`ν < √2 − 1`), whose masses stay positive up to `θ = π/2`
(`ν = 0`, `b = 0`): real systems, on `Φ_θ,K` and `Φ_θ,L_z`, and on the δm strip for a target symmetric in bodies 1 and
2 (the default), the leg-swapped twins of the fold's own, which the fold exists to leave out; on a δm strip with an
asymmetric target they are new systems. The acute angle's edges are now declared per edge (§5.4's table; R-932,
R-938): affine to the domain's end, the zero-mass end at `θ = 0` saturating, and past `π/4` a reflection onto the
fold's own side where the systems there are twins, or, on a δm strip with an asymmetric target, affine to `θ = π/2`
and saturating there. *Was:* "On both sides the acute angle is bounded anyway (§5.4): one type per axis, and the twins
side decides it."

**Hidden coordinates under tilt and slice (R-633, ruling RQ-603).** These charts hold their blocks by a construction,
not by latent controls, so their hidden coordinates are the construction's physical parameters, tilted and sliced by
value as R-554 L5's `p_λ` (`principia_chart_decoder_contract.md` § "Tilt (basis edit)", "Tilt on every chart"), but a
held `K` in warped mode, which R-708 tilts in its warp coordinate and which still slices by value, below the table (R-720;
*was:* "a held energy-type coordinate in warped mode"):

| Chart | Hidden coordinates | At `d = 0` |
|---|---|---|
| Ternary mass plot | `ν₀`, the held shape (a physical value); the four momentum controls from 0 (the rest start, `z_mom = 0`) | the untilted chart, exactly |
| The Euclid plane, the δm strip `Φ_strip,δm` | the four momentum controls from 0 (the rest start) | the untilted chart |
| `(ν, K)`, `Φ_θ,K` (and `Φ_strip,K`, the same map) | the held `L_z` (0 untilted), a physical value | the untilted chart |
| `Φ_θ,L_z` | the held `K`, a physical value | the untilted chart |

A momentum control is tilted from 0 and its momenta are `D_mom` of it under the pixel's masses, every registered
momentum link mapping 0 to 0. With a held `L_z` or `K` tilted, the momenta are §2.2's construction at the tilted value
(§2.2 takes any `L_z` with `K ≥ L_z²/2I`). The rest follows R-471's two modes (R-708, ruling RQ-678; *was:* "and a cell
with no construction there (`K < L_z²/2I`) reads as refused: no system, area weight 0", in every mode).

- **Warped mode, the default.** Every pixel inside the chart's square is a system under every tilt, no cell refused.
  `(ν, K)`, `Φ_θ,K` and `Φ_strip,K` floor the axis at the pixel's own held `L_z`, $K = L_z^2/2I + K_{\max} t^{\gamma_K}$,
  which may pass `K_max` at the top, a range as R-508's side continues. `Φ_θ,L_z`'s held `K` is tilted in its warp
  coordinate, `K = K_max·t^γ_K`, `t = t₀ + d`, `t₀ = (K₀/K_max)^(1/γ_K)`, `d` in units of `t`, so it stays ≥ 0, its
  `K_max` and `γ_K` a calibration of this chart's own (R-71), with `K_max` at the chart's calibrated `K` (REQ-CHART-046),
  so `t₀ = 1`, the value proposed at the gate (R-833, ruling RQ-710 (c) with (b) proposed); it takes
  no floor, the band above giving `L_z²/2I ≤ K` at every `K ≥ 0`. Rounding at the floor must not blank the bottom row,
  as §2.1 holds `L_max² ≤ 2K`. The held `L_z` moves by value, tilted and sliced alike, `L_z = L_z₀ + d`, with no sign
  constraint: the floor keeps every pixel a system at any `L_z` (§2.1's one exception), and a column above `K_max` at large
  tilt is accepted, the practical bound the slider span, RQ-679 (R-720, ruling RQ-699; *was:* "How the held `L_z` itself
  moves is RQ-699."). A slice of a held `K` stays by value in both modes, `K₀ + d`: R-708 amends the tilt only,
  REQ-CHART-060's stop holds the centre at `K₀ ≥ 0`, and `t₀` is recomputed from the new `K₀` (applied per R-369).
  A second exception of §2.1's kind: a pixel whose `K*` has no finite `2K*` at the decode's own precision, reached only
  at an extreme `γ_K` or `d`, or on a held `L_z` where the floor plus the line passes it, has no construction. §2.2's
  construction labels it `INVARIANT_INFEASIBLE`, with `a = 0` and momenta 0, as at an `L_z` with no finite square, on
  `K*`'s exponent bits beside the exact test, `validate` refuses it at the chart's own precision, and its area weight
  is 0. No `K` is saturated into a system and nothing is mirrored (applied per R-369,
  the physics reviewer's call on #388).
- **If the implementer confirms it arises (R-709).** A held energy's warp coordinate past `t < 0` clamps at `t = 0`, held
  `K` at its floor, `L_z²/2I` (on `Φ_θ,L_z`, where `L_z` is read off `K`, both are 0: the rest start at the pixel's own
  shape). Each clamped pixel is a real system, flagged "held at floor" in the Inspector and the sidecar, and the test
  output reports the clamped-pixel share. There is no mirror.
- **Linear mode, the option per axis (R-471 E2).** Rows keep fixed physical values, a held coordinate tilts by value,
  and a cell with no construction there (`K < L_z²/2I`) takes its edge's declared kind (§5.4's table; R-932, R-937):
  on a `K` axis at a held `L_z` the clamp below the floor at the pixel's own `L_z` (§2.2 (v)), on `Φ_θ,L_z`'s `L_z`
  the unfold past `±√(2IK)` and the clamp below the floor at a held `K ≤ 0`; a clamped cell is a system, out of the
  count. *Was:* "is no system, area weight 0, blank (R-470)."
- **The f32 band at the floor (R-876, 8 Nov 2026, RQ-740).** "Every pixel is a system" allows an f32 chart (`HeldLz`,
  `HeldK` and the Burrau charts R-708 covers) to refuse a pixel inside the band one float wide at the warp's floor, where
  f32 rounding, not the chart, decides: such a refusal is labelled as rounding, never drawn as the chart's edge, and no
  pixel that isn't a system is ever accepted. The edge walk is repeated on the GPU when the shader decodes these charts
  (TASK-M4-06). *The band, defined (TASK-M2-66, under physics review, R-72; `kernel::chart::band`):* it is measured in
  the pixel's own coordinates `(u, v)` at the chart's precision, the row coordinate `t = v` among them, and it is one
  float wide. An f32 refusal at `(u, v)` is a rounding refusal iff the chart as authored at f64, whose narrowing the f32
  chart is, accepts the same pixel (its f32 coordinates widened exactly; a parameter's rounding is f32 rounding too) and a pixel one f32 float away, `(u ± 1 ulp, v)` or `(u, v ± 1 ulp)`, is accepted at
  f32. The width is one float as measured on the charts below, not derived for every chart: a check set by a parameter
  narrowed to f32, or an operand whose map from `(u, v)` is flat (a power warp near `t = 0`), can put a rounding refusal
  many floats from f32-accepted ground, where it would read as unexplained, a false alarm and never a wrong picture;
  the GPU's edge walk (TASK-M4-06) repeats the measurement. A refusal its f64 twin shares is the chart's edge; one its twin does not share, with no f32-accepted
  pixel one float away, is no rounding effect and takes R-506's decoder-bug warning. Measured on the CPU edge walk
  (`f32_band_label`, 2,121,000 pixels of `HeldLz` on `(ν, K)` and `(θ, K)` and of `HeldK`, warped and linear): no
  rounding refusal at the warp's floor, which `k_floor` keeps feasible at the chart's own precision; the 24,134 rounding
  refusals are all on the acute charts' `θ` axis at `u = 1`, where `θ_max = π/4` rounded to f32 lies above `π/4`;
  against the f32 chart's own parameters widened there are none, so the per-pixel arithmetic never decides alone.
- **The slider spans of the held coordinates (R-920, ruling RQ-679; a definition, R-72, REQ-CHART-188).** R-507 U1
  divides a hidden offset by the coordinate's slider span. A span is the full range at zoom 1 of the screen axis that
  carries the same coordinate on the registry's own charts, read at that chart's home window, the preset's: a length,
  fixed per chart, the same under pan, zoom, slice centre and lock, stored with a saved chart (R-507 U3), and invariant
  under the scale gauge (§0.5). So a 90° tilt turns a screen axis into that sibling axis at its home extent.
  - *`ν₀`* (the ternary mass plot): `Δν = ν_max − ν_min`, the `ν` axis's range on the Euclid plane and the `(ν, K)`
    chart, `[1/32, 31/32]` (REQ-CHART-023), so `15/16`; rail `[ν_min, ν_max]`, centred on the default `ν₀ = 1/2`,
    linear; under a lock the anchor `± Δν/2`. Not `(0, 1)`, whose ends are no systems. A tilted `ν₀ + d` outside
    `(0, 1)` is refused per cell, as above (blank, R-470; area weight 0), never clamped by the rail; REQ-CHART-060's stop
    holds the slice.
  - *The held `L_z`* (`(ν, K)`, `Φ_θ,K`, `Φ_strip,K`; by value, R-720): `2√(2·I·K_max)`, `I = 1` after the scale gauge,
    `K_max` the chart's own `K` axis's (REQ-CHART-044), read at the window's top, `K = K_max`: the `(L_z, K)` chart's top
    row, `L_z = (2s − 1)·√(2IK)` at `t = 1` (§2.1). Rail `[−√(2K_max), +√(2K_max)]`, centred on 0; under a lock the anchor
    `± √(2K_max)`. Warped, the floor keeps every pixel a system, and inside the zoom-1 window `L_z²/2` stays at most
    `K_max` under a single 90° tilt; linear, a single 90° tilt draws the `(L_z, K)` chart's feasible parabola.
  - *The held `K`* (`Φ_θ,L_z`): warped, 1 in its warp coordinate `t`, the warped axis's zoom-1 range, `K = K_max·t^γ_K`,
    `K_max` and `γ_K` this chart's own (REQ-CHART-141, R-833); the rail fixed per preset at `t ∈ [t₀ − ½, t₀ + ½]`, `t₀`
    the preset's, shifted to `[0, 1]` if `t₀ < ½`, never below `t = 0`; under a lock the anchor `± ½` in `t`. Linear,
    `K_max` in `K`, the axis's `[0, K_max]`. One slider, two units: the handle is in `t`, the rail drawn evenly in `t`, the
    number box shows `K = K_max·t^γ_K`; a slice writes `K₀` by value (R-720) and `t₀ = (K₀/K_max)^(1/γ_K)` is recomputed,
    which moves the handle, not the rail; a tilt reads the rail's length, 1 in `t`, `ΔK = K_max·[(t₀ + Δt)^γ_K − t₀^γ_K]`.
  - *A `MixedEnergy` chart's held `L_z`* (R-818) on a lone virial axis has no chart-constant `K_max` (`K = Q·|U|/2`,
    R-817): its span is RQ-701's, written by TASK-M8-48 at M8 (R-920), as are R-818's other lone axes.
  - *The checks* (written before any run): at a single 90° tilt in the zoom-1 window the tilted axis's hidden values
    equal the sibling axis's at the same `u`, to rounding (`ν₀` the Euclid plane's `ν(v)`; the held `L_z`
    `±√(2K_max)·(2s − 1)`; the held `K` `K_max·t^γ_K` over `t₀ ± ½`), a span of 2 or one read at the view's centre
    failing; a pan at fixed `τ` leaves every span bitwise unchanged, a centre-read span moving; a saved tilted chart
    reopens bitwise with its stored span.

Built by TASK-M2-52 on TASK-M2-36's tilt path, the linear option on TASK-M2-37's.

*Ruled 8 Nov 2026 (R-875, RQ-745 (b)):* the three acute-angle charts are re-expressed over the full acute range, both
labellings of each shape shown, the leg swap shown once only by the symmetry filter (R-855); R-27's "shape only" fold
charts give way. TASK-M2-85 builds it once the filter's maths (TASK-M2-84, R-916; *was* TASK-M8-120) exists; until it merges they stand as built, and
REQ-CHART-045's range comes back to the owner at the M2 gate.

### 4.6 The central hypothesis

**Hypothesis.** The fractal basin structures seen in the Burrau–Pythagorean family are cross-sections of
codimension-$k$ structures ($k < 6$) in the full 8D IC manifold, not artefacts of the constrained
submanifold.

**Test.** Lock onto a basin boundary in a Burrau slice. Rotate the slice basis continuously into the
unconstrained dimensions (the Burrau-to-unconstrained morph, `principia_chart_decoder_contract.md` Part 4).
Watch whether the boundary persists, deforms or dissolves.

---

## 5. Implementation plan

### 5.1 One trait, one dispatch

```rust
pub trait Chart {
    fn map<F: Float>(&self, u: F, v: F) -> ChartOut<F>; // Φ : [0,1]² → chart space, generic over the float type
    fn forbids_energy_normalisation(&self) -> bool { false }
    fn name(&self) -> &str;                          // goes in every dump header
    fn validate(&self, u: f64, v: f64) -> ValidationResult; // the domain function (R-26), CPU-side f64
}
```

Φ is generic over the float type (lowering Part 2, R-118): the one source is monomorphised as the f32 kernel and the
f64 CPU reference. `validate(u, v)` stays a CPU-side f64 check (`principia_inverse_encode_contract.md`, chart-aware
validation).

**The chart's parameter type** (refining R-118; applied per R-369, TASK-M2-05). Each chart declares an associated
`type Param: Copy`, the float type its parameters are held at (f32 in the kernel's uniform, f64 on the CPU), and `map`'s
bound is `F: Float + From<Self::Param>`: `map` is generic over every float type the parameters widen into, so a chart
held at f32 maps at f32 in the kernel and one held at f64 maps at f64 on the CPU, from the one source. Narrowing f64
parameters to f32 is the CPU's own step, never `map`'s; the bound has no fallible conversion (as `NumCast`'s
`Option` would be) in the kernel. Every chart implements the trait so.

**`ValidationResult`** (R-26; defined per R-72, REQ-CHART-043). `validate(u, v)` checks the chart point against
`principia_inverse_encode_contract.md` § "Chart-aware validation"'s layers 2 (chart feasibility) and 3 (decode sanity);
layer 1, the hypercube bounds, is the encode path's, on lookup and lock, never `validate`'s
(`principia_chart_decoder_contract.md` Part 3, R-407). It returns one of four variants:

| variant | meaning |
|---|---|
| `Valid` | `(u, v)` passes every layer-2 and layer-3 constraint of the chart |
| `Project { u, v, failed }` | `(u, v)` fails the constraint `failed`; the `(u, v)` returned is the nearest point of the chart's feasible region (for `(Lz,E)`, on the parabola), where lookup and lock commit, and notify |
| `Clamp { u, v, moved, failed }` | `(u, v)` fails `failed`; each out-of-range coordinate is snapped to its boundary, giving the `(u, v)` returned, and `moved` says which of `u`, `v` moved, for `lookup_clamped` and the display of the moved coordinates |
| `Reject { failed }` | `(u, v)` is refused with an error naming `failed`, only where projecting or clamping would give a qualitatively different IC from the one intended |

On a failure the chart returns the first of project, clamp, reject that keeps the IC the one intended (the contract's
order of preference). **The failed constraint** `failed` names one of the contract's constraints, layer 2 then layer
3: the `(Lz,E)` parabola `|Lz| ≤ √(2I(E − U))`; `(Lz,K)`'s `K ≥ 0` and `K ≥ Lz²/2I`; the mass simplex's `mᵢ > 0`,
`Σmᵢ = 1` and interior buffer `ε_m`; the shape sphere's `θ ∈ [0, 2π)` and `φ ∈ [0, π]`; the Burrau charts' `ν ∈ (0,1)`
and the acute-angle charts' `θ ∈ (0, π/4]`, `Φ_θ,L_z` adding `(Lz,K)`'s `K ≥ Lz²/2I` and the δm strip the mass simplex's
`mᵢ > 0` on its blended masses (§4.5; TASK-M2-11);
then decode sanity's decoded masses positive, CoM at the origin and total momentum zero. A mixed-axis chart names its
axes' constraints; a chart that adds a constraint adds it to this list. Every chart's `validate` checks both layers.
Past an edge (§5.4) three of these bounds are the chart's own ranges, which the edge's declared kind extends past, and
not a system's validity: the mass simplex's interior buffer `ε_m`, below which a saturated mass lies; the acute
angle's `θ ≤ π/4`, past which an affine pixel lies; and the Burrau charts' `ν ∈ (0,1)`, past which the δm strip's
mirrored triangles lie, their blended masses positive (physics review 5911915550 of PR #546). The classification reads
them there as the primary range's, never as a layer-2 failure (`principia_chart_decoder_contract.md` Part 3, step 3;
R-932, R-941), and on lookup and lock they bind as written, as REQ-ENC-030's class rule does (R-937): `ε_m` and
`ν ∈ (0,1)` in `principia_inverse_encode_contract.md` § "Chart-aware validation"'s layer 2, `θ ∈ (0, π/4]` in this
list and, on encode, `principia_dd_encode.md` §3.1's fold (REQ-ENC-022) (Codex review of PR #546; qa review 5911919550,
F3). *Was:* "two of these bounds", "and the acute angle's `θ ≤ π/4`, past which an affine pixel lies; the
classification", "and on lookup and lock they bind as written, as REQ-ENC-030's class rule does (R-937)".
The latent chart (§1.1) adds no layer-2 constraint, and layer 3 holds for it by the shared decoder's construction: the
masses are positive under every registered mass link (§0.1), the CoM is at the origin (§0.2) and `Σpᵢ = 0` (§0.3),
the residuals rounding within REQ-ENC-027's tolerance. So its `validate` returns `Valid` at every `(u, v)`. A chart
that bypasses the canonical-frame decode can fail layer 3, and its `validate` checks it.

`ChartOut` is whatever `D` consumes — masses, `(α,β)` or an explicit `(ρ,λ)`, and momenta or a
`(Lz,K)` request. **`D` and `C` are shared and written once.** The integrator never sees a chart.

**Charts to implement**, in this order — each is a `Φ`, and the shared decoder is unchanged:

| # | chart | why this order |
|---|---|---|
| 1 | `Latent { z0, q1, q2 }` | the general case; axis-aligned and oblique are both instances |
| 2 | `ShapeSphere { n0, e1, e2, I, phase }` | the nonlinear one — needed for the linearised-decoder test |
| 3 | `Burrau { nu_range, k_range }` | the project's namesake family |
| 4 | `InvariantLE / InvariantLK` | most machinery, most verification |
| 5 | `MassSimplex` | cheap once the decoder exists |

`BodyPlane` (today's slice) stays as a chart and must reproduce **bit-for-bit** — it is the
Python cross-check's anchor.

**`BodyPlane`'s `Φ`** (defined by TASK-M3-36; `kernel::chart::BodyPlane`). Parameters: the moved body `b`, the window's
centre `(c_x, c_y)` and half-width `h`, the masses (`Σmᵢ = 1`) and the held positions. `Φ(u, v)` puts body `b` at
`((c_x − h) + (2h)·u, (c_y − h) + (2h)·v)`, in that operation order, the other bodies at their held positions, every
body at rest; the decode is `C` (CoM, `I = 1`, `ρ` onto `+x`, no mirror) and the rest-start blocks on the resulting
Jacobi vectors. The map is affine on the whole plane: both axes extend affine on both sides, and every finite `(u, v)`
is a system (a binary collision is a system with `V₀ = −∞`). `Φ` reproduces the reference's initial conditions, not a
field at a common physical time: the decode's gauge gives each pixel its own length and time unit, so only gauge-free
quantities (the shape vector, `E·√I/M^{5/2}`, `r_min/R`) compare across the two.

**The reference it matches.** The Python reference imported at `docs/reference/prin-rs/reference/` (`tb.burrau_grid`,
Burrau's masses `(3, 4, 5)` at `(1, 3)`, `(−2, −1)`, `(1, −1)`, at rest), its output recorded as the fixture
`fixtures/golden/bodyplane/reference.tsv` by `crates/validation/xcheck/bodyplane_ref.py` over the five Burrau regions
(9 × 9) and the cross-check's 3 × 3 window; `cargo xtask golden bodyplane` compares. Bitwise where the operation order
is identical (the placement of the body at the reference's own position); otherwise within the stated tolerance: `Φ`'s
pixel position against `numpy.linspace`'s node within 6 ulp of `max(|c ± h|)`, derived from the two orders' roundings;
the gauge-free algebra within `1e-13`. The reference's shape vector takes `q = ρ̃_y λ̃_x − ρ̃_x λ̃_y`, so its `n₂` is
this document's `−n₂` (R-14's standard cross): a convention, not a tolerance. `Φ` is bitwise today's slice (the
re-registration gate's `ics`). The Python cross-check of the integrator is the change-10 re-run
(`cargo xtask gate change10-rerun`; `principia_dd_predictability_horizon.md` §7.8).

### 5.2 Tests that can fail

- **`Σpᵢ = 0`** to machine precision, every chart, random `(u,v)` — catches the crossed-mass
  transcription hazard in §0.3
- **`I = 1`** after the canonical-frame decode
- **Shape round-trip** `shape_vec(decode(u,v)) == n(u,v)` to ~1e−14
- **`(Lz,E)`:** the constructed momenta hit `Lz` **and** `K` to machine precision, over random
  `(u,v)`, including near the parabola boundary where `K* → K_min`
- **Feasibility:** no `(u,v)` in `[0,1]²` produces `K* < K_min` under the warp — the warp's whole
  purpose
- **Seed fallback:** force the primary seed to degenerate (e.g. `ρ = 0`) and assert a fallback is
  selected rather than `DEGENERATE`
- **Burrau at `ν = 1/2`** reproduces `(3,4,5)`, and the classical configuration to a stated
  tolerance
- **`forbids_energy_normalisation`** is enforced — a config combining `(Lz,E)` with any
  `Some(E*)`, including `Some(0)`, is **refused** (R-25), and a test asserts the refusal
- **Axis-aligned latent slice with `q1 = ê_α, q2 = ê_β`** equals a direct `(α,β)` sweep
  (defined per R-72, REQ-CHART-050). The **direct `(α,β)` sweep** about the slice centre `z0` maps pixel `(u, v)` to `α
  = α-link(z0_α + (2u − 1))` and `β = β-link(z0_β + (2v − 1))`, through the selection's α and β links (§0.2's `α = α_min
  + (π/2 − 2α_min)·σ(z_α)` and `β = z_β`, `periodic_beta`, `β`'s one link, R-898, R-919 (the offset `β = z_β + π/2` from 17 Nov 2026, R-1020); *14 Nov 2026 (R-865, R-898, R-919; TASK-M2-107; physics review 5903954379 of PR #533, P3), the target TASK-M2-57 builds, in force when it merges; until then the Was describes the built decoder:* *was:* "and `β =
  π·σ(z_β)` under the defaults"), and builds the IC from `(α, β)` by §0.2: `ρ̃ = cos
  α·(1, 0)`, `λ̃ = sin α·(cos β, sin β)`, unweighted and reconstructed with the masses of `z0`'s mass block (§0.1), and
  the momenta of `z0`'s momentum block (§0.3). The comparison is per pixel (REQ-CHART-030), so the sweep is the one
  whose `(u, v) → (α, β)` map is the slice's own; a sweep linear in angle has the same `(α, β)` set at other pixels and
  is not it. The equality asserted: at each pixel, inside `[0,1]²` and past it, at f64 and at f32 and under every
  registered link, the slice's decode `D(z(u, v))` and the direct sweep give bitwise-equal `α`, `β` and `(m, r, p)`
  (`==`, so `+0 = −0`): the slice's `z(u, v)` is `z0` with `z_α = z0_α + (2u − 1)` and `z_β = z0_β + (2v − 1)`, exactly,
  up to the sign of a zero. "Every registered link" is every `α` link, with `periodic_beta` the one `β` link (R-919).
- **`BodyPlane` bitwise unchanged**, and the Python cross-check green

### 5.3 What to report per chart

Leaf count, `alpha` distribution, tree shape. **But leaf counts are slice-conditional to 4.3× —
compare within a chart, never across.** The `alpha` distribution is the safer cross-chart quantity.

**And the standing caution applies with extra force here:** a chart that produces a prettier
picture is not a better chart. The measurement is whether the criterion behaves consistently
across charts, not which chart looks best.

### 5.4 Past the unit square — each axis's extension type (R-407)

*10 Nov 2026 (R-932, R-937, R-938, R-941, R-942); rewritten 15 Nov 2026 by TASK-M2-95 (REQ-CHART-194):* the owner's
rule on chart edges, no blank areas, and every chart's edges built now. Each edge of each chart declares a kind, its
default per R-932 and R-941 (saturate past a true edge, unfold onto a second sheet, wrap where the coordinate repeats;
the mechanism on each edge the physics reviewer's call, saturating wherever the edge system itself is not a valid
system), each declaration physics-reviewed under R-407's clause 3. The per-edge table below is the text in force; the
chart addendum's N13 and N14 build it, and until they land the built charts draw as the table before it, kept below as
*Was*, describes (R-932). Every default but the mass simplex's (R-941) is confirmed or changed by the owner by eye when
the kernel's charts render (RQ-779, open). Under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` (R-597)
saturates in place of drawing as not a system, the physics reviewer's call under the rule; how it saturates is RQ-827
(below). *17 Nov 2026 (R-1024, RQ-827 (a)):* such a pixel is clamped to the fence, not saturated: `M₀₁` held at `ε_M01`, the inner ratio `m₀ : m₁` kept, `m₂ = 1 − ε_M01`, the configuration and momenta decoded at those masses, a system the decoder accepts (the clamped `M₀₁`, formed as the decoder forms it, at or above `ε_M01`), not counted, and on the sim key, its system not one the square's samples compute (R-1025). A pixel energy normalisation cannot reach (R-598) is not reached by the rule: R-598 stands, such a pixel a
validity failure, not an edge, drawn as not a system with the reason in the readout (R-955, the owner's answer on
RQ-784, 11 Nov 2026). *Was:* "the owner's rule on chart edges, no blank areas, amends this section, and every chart's
edges are built now: each edge in the table below takes a declared extension (saturate past a true edge, unfold onto a
second sheet, wrap where the coordinate repeats; the mechanism per edge is the physics reviewer's call), the invariant
charts' spin edges unfolding onto the time-reversed sheet (R-937, § 2.1, § 2.2), the mass simplex's sides saturating
(R-941), the time axis before 0 showing the starting state."; "The table is the text in force for what is built today;
the chart addendum's N12 rewrites it per edge, each replaced line kept as *Was*
(`plan/charts/m2-chart-addendum-extensions-axes.md` § A.11)."; "whether the rule reaches η_E's unreachable pixels
(R-598): it does not, R-598 standing, … (R-955 …; *was:* "is RQ-784, R-598 standing meanwhile")".

The rule is `principia_chart_decoder_contract.md` Part 3's (§ "Past the unit square — each axis's extension type
(R-407)"): **each edge declares a kind**, **affine** (the formula continues), **wrap** (R-407's periodic: the coordinate
repeats), **pole-crossing** (continues over the pole, the partner axis shifted by half its period), **unfold** (onto the
second sheet, §2.2 (iv)), **clamp** (the edge system, the nearest point of the primary range in the edge's declared
metric), **saturate** (the coordinate squashed toward a true end it never reaches) or **blank** (R-407's "bounded": not
a system past the edge, drawn blank), and, where a declaration below makes one, a **reflection** or a **repeat map**,
each a new kind under R-407's clause 3. Each kind has its primary range and its place in R-408's classification order
(the contract's). An affine edge may run to a true end and take that end's kind past it ("affine to the zero-mass end,
then saturate"). No edge's default is blank; every edge may be switched to blank, and each row lists the other kinds
its edge may be switched to (R-942: the row declares the kinds allowed and the default, the view state an override per
edge). Whether a clamped pixel re-addresses to the primary range's quads, off the sim key, as R-942 has it, or is its
own system with its own quads, as R-937's C5 and §2.2 (v) and (vi) make most clamped pixels, is RQ-828; meanwhile both
rulings' words stand and the table's clamps are defined as written (code review 5910534528 of PR #546, F1). *17 Nov 2026 (R-1025, RQ-828 (c)):* split: a clamp whose every pixel shows a system the primary range's samples already compute (the Euclid plane's `m`, where the decoder ignores `u`; a time axis before 0, every row the start) re-addresses to those quads, off the sim key, as R-942 has it; every other clamp (§2.2 (v)'s per-pixel tilt, C5; a spin past the square's range; §2.2 (vi)'s exact edge points; the `M₀₁` fence, R-1024) is on the sim key, each clamped pixel its own system, requested and integrated at its own address, and switching it to blank drops the clamped area's quads from the request set. Which clamp is a repeat each row declares per edge, TASK-M2-105's definition under physics review; the clamp definitions and C5 stand. Outside the primary range a pixel whose `Φ` fails to evaluate, or whose state fails `validate(u, v)`, is not a
system, drawn blank (R-470; the hatch only under Overlays › Chart › forbidden region and in the debug views), and
labelled `decode_failed`; inside `[0,1]²` nothing changes but where a true edge lies inside the square (the exponential
map past `r = π`, R-503 amended by R-932; the linear option's infeasible pixels, R-471 E2 amended by R-932 and R-937). *Was:* "each axis declares **affine** (continues), **periodic** (wraps), **pole-crossing**
(continues over the pole, the partner axis shifted by half its period) or **bounded** (not a system past its edge, drawn
blank, the default); outside `[0,1]²` a pixel whose `Φ` fails to evaluate, or whose state fails `validate(u, v)`, is not
a system, drawn blank (R-470; …), and labelled `decode_failed`; inside `[0,1]²` nothing changes." *Was (before R-470):*
"**bounded** (hatched past its edge, the default)" and "is hatched as forbidden and labelled `decode_failed`" (stale
values, replaced by R-470; applied per R-369). The `Chart` trait (§5.1) declares each edge's kind, its default and the
kinds allowed beside `map` (*was:* "declares each axis's type beside `map`"). Each shipped chart's edges:

| chart | edge | default kind | past the edge | may also be | count (R-408, R-932) |
|---|---|---|---|---|---|
| `Latent` (§1.1), every flat or axis-aligned slice, `BodyPlane` | none: the plane | affine | `z₀ + (2s−1)·q₁ + (2t−1)·q₂` continues; the links saturate far out (R-570, REQ-CHART-087); no pixel is clamped (*17 Nov 2026 (R-1024, RQ-827 (a)):* except a pixel whose inner masses sum below `ε_M01`, clamped to the fence, `M₀₁` held at `ε_M01`, § 5.4) | — | every visible pixel counts (unbounded); a validity failure is forbidden, η_E's unreachable pixel among them (R-598, R-955) |
| | a latent `z_β` axis, once R-898's periodic `β` is in force (TASK-M2-57) | wrap, period `2π` in `β` | the same systems | blank | one period |
| | the `M₀₁` fence under the edge-reaching simplex link (`M₀₁ < 10⁻⁵`, R-597), inside the plane | clamp, to the fence (R-1024, RQ-827 (a); *was:* "saturate (R-932; physics review 5851141196 of PR #478, F3)") | the fence system: `M₀₁ = ε_M01`, `m₀ : m₁` kept, `m₂ = 1 − ε_M01`, the rest decoded there (*was:* "its definition is RQ-827; until it is ruled R-597's arm stands, not a system with the reason in the readout") | — | not counted, as a clamp; on the sim key (R-1025) (*was:* "as RQ-827 rules") |
| `ShapeSphere`, spherical map (§3.3) | θ at `s = 0, 1` | wrap (periodic) | the same shapes | blank | one period, the chart's `[0,1]` span |
| | φ at the poles `t = 0, 1` | pole-crossing (θ shifted by π) | the same shapes over the pole | blank | pole to pole, the chart's `[0,1]` span, the whole sphere (R-544) |
| `ShapeSphere`, exponential map (§3.3) | the square's edges | affine to `r = π` (a declaration) | the shapes out to `r = π` the square does not show | blank | counted: the disc `r ≤ π` is the primary range |
| | `r = π`, inside the square or past it | repeat map: the formula continues, the point at `(r, ê)` being the one at `(2π − r, −ê)` (§3.3; reduced mod `2π`), on a tilted view only where `N_a = 0` and `N_b = 0`, decided exactly on the stored angles; elsewhere affine (the repeat on a tilted view, below the table) | the sphere again, from the antipode `−n₀` back toward `n₀`; on the affine branch at offsets `d` the image in `r ≤ π` does not carry | clamp (to `−n₀`, one system), blank | not counted: a repeat, resolved to its canonical point in `r ≤ π` (R-537); on the affine branch counted, each pixel its own system, but where its computed `d` equals its image's exactly (below the table) |
| `InvariantLE / InvariantLK`, warped (§2.1) | `s = 0, 1` | unfold (§2.2 (iv)) | sheet 2: `s' = 2 − s` past 1, `s' = −s` past 0, the time reversal of the chart's own system at `−L_z` | clamp (the rigid rotation at `±L_max(t)`, §2.2 (vi)), blank | sheet 2 counted inside `s ∈ [−½, 3/2)`; past it a repeat, not counted; on a tilted view only where `N_⊥ = 0`, the unfolded axis's component of `N` (`N_a` in the `a` position, `N_b` in the `b` position), else counted (below the table; *was:* "only where `N_a = 0`") |
| | `t < 0` | clamp (§2.2 (vi)): the rest start, at the pixel's own configuration | a column of rest starts (one system untilted and unsliced) | blank | not counted |
| | `t > 1` | affine (R-508) | `K > K_max`, systems the square does not show | blank | counted (`t ≥ 0`) |
| `InvariantLE / InvariantLK`, linear option (§2.1, R-471 E2) | the parabola `\|L_z\| = √(2I(E − U))` where `K* > 0`, at each pixel's own `U`, `I` | unfold (§2.2 (iv)): `L_z` reflected at the pixel's own `E`, the sheet flipped | sheet 2 | clamp (REQ-ENC-030's projection, §2.2 (vi)), blank | sheet 2 counted inside `s̃ ∈ [−½, 3/2)`; past it a repeat; on a tilted view only where `N_⊥ = 0`, the unfolded axis's component of `N` (`N_a` in the `a` position, `N_b` in the `b` position), else counted (below the table; *was:* "only where `N_a = 0`") |
| | below the floor, `K* ≤ 0`, `L_z ≠ 0` | clamp below the floor (§2.2 (v), R-937; metric `\|ΔE\|` at fixed `L_z`) | the rigid rotation at the pixel's own `L_z`, `E' = U + L_z²/(2I)` | blank | not counted (C6) |
| | `K* < 0`, `L_z = 0` | clamp (§2.2 (vi), REQ-ENC-030's apex) | the rest start | blank | not counted |
| | **the seam along `E = U`** (declared, R-937) | — | above it the unfold or the projection shows near-rest systems; below it the clamp a rigid rotation with kinetic energy `L_z²/(2I)`, growing with `\|L_z\|`, vanishing at the apex; just above it the unfold's period falls below a pixel, a band of outcome noise at large `\|L_z\|`, no pixel averaged across sheets; shown in the debug kind map | — | — |
| | the square's edges | affine (the axes are linear) | the kinds above act wherever the pixel's `(L_z, E)` lands | blank | as the rows above |
| A lone or mixed energy axis at a held `L_z` (`K`, `KE`, `E`, the virial ratio; §2.1), warped | `t < 0` | clamp to the `t = 0` row at the pixel's own configuration: the floor, §2.2's rigid rotation at the held `L_z` (the rest start at `L_z = 0`), on `K`, `KE`, `E`, and on the virial ratio at `Q₀ = 0`; on the virial ratio with `Q₀ > 0`, `max(Q₀, Q_min(U))` (R-817), `Q_min(U) = L_z²/(I·\|U\|)` (physics review 5910557865, F6; *was:* "clamp to the `t = 0` row: the floor, §2.2's rigid rotation at the held `L_z` (the rest start at `L_z = 0`), at the pixel's own configuration") | a line of `t = 0` row systems | blank | not counted |
| | `t > 1` | affine (R-508) | `K` past `K_max` (`Q` past `Q_hi`) | blank | counted |
| the same axis, linear option | below the floor, `K* < L_z²/(2I)` | clamp below the floor (§2.2 (v)) at the held `L_z`, metric `\|ΔE\|` (`\|ΔK*\|`) at fixed `L_z` | the rigid rotation at the held `L_z` | blank | not counted |
| A lone `L_z` axis at a held `K` (§2.1), warped or linear | `\|L_z\| = √(2IK)` | unfold, as on `Φ_θ,L_z` | sheet 2 | clamp (§2.2 (vi)), blank | as the linear option's unfold |
| A warped mass-ratio axis (§2.1, R-506) | `t < 0`, `t > 1` | affine: the geometric `q(t) = q₀·(q₁/q₀)^t` continues through real systems, as R-508's `t > 1` does on the energy axes, up to the end the link realises at the pixel's own `m_c` (§2.1's `ε_μ` test); past that end, clamp to it (physics review 5910557865, F7; *was:* "clamp to the edge row, `q₀` or `q₁` at the pixel's own `m_c`") | ratios beyond the asked range, then the realised end | blank | the affine part counted; the clamped part not (*was:* "not counted") |
| the same axis, linear option | `q ≤ 0`, or a ratio not realised | clamp to the nearest realised ratio of the asked range, at the pixel's own `m_c` | a line of edge systems | blank | not counted |
| `MassSimplex`, the ternary plot (§4.5) | each side | saturate (R-941; the definition below) | masses below `ε_m`, approaching `ε_M01` and never reaching it | clamp (the side's system in the chart's own coordinates), blank | counted: a system with a mass below `ε_m` is not in the square |
| Burrau family: the Euclid plane (§4.5) | `m` (the annotation axis `u`), both edges | clamp to the edge column (identical to the formula continuing: the decoder ignores `u`) | the row's own system | blank | not counted (a repeat of the row) |
| | `ν`, both edges | affine to the zero-mass end, then saturate (below) | real shapes up to `ν = 1` (`a → 0`) and down to `ν = 0` (`b → 0`) | clamp, blank | counted: each pixel its own labelled system (R-27; below) |
| Burrau family: the `(ν, K)` chart (§4.3, §4.5) | `ν`, both edges | as on the Euclid plane | as there | clamp, blank | as there |
| | `K`, `t < 0` | clamp to the `t = 0` row: `K = 0`, the rest start of each shape, or the floor `L_z²/(2I)` at a held `L_z` (R-720) | a line of the row's systems | blank | not counted |
| | `K`, `t > 1` | affine (R-508) | `K > K_max` | blank | counted |
| Burrau family: `Φ_θ,K`, the strip `Φ_strip,K` (§4.5) | `θ` below `θ_min` | affine to the zero-mass end `θ = 0` (`a → 0`), then saturate | the thin triangles of `(0, θ_min)` | clamp, blank | counted |
| | `θ` above `θ_max` | affine to `π/4` where `θ_max < π/4`; past `π/4` reflection, `θ' = π/2 − θ` (a new kind; the reflection and its conditions below), and past `π/2`, where `θ' < 0`, the reflection then the `θ = 0` edge's kinds on `θ'` | past `π/4` the leg-swapped twins of the fold's own (R-27) | clamp, blank | up to `π/4` counted; a reflected pixel not counted, a repeat of its canonical `θ'` |
| | `K` | as on `(ν, K)` | as there | as there | as there |
| Burrau family: `Φ_θ,L_z` (§4.5) | `θ`, both edges | as on `Φ_θ,K`, but the reflection flips the `L_z` axis: `(θ, v) ↦ (π/2 − θ, 1 − v)` | as there | as there | as there |
| | `L_z` past `±√(2IK)` (past the square's edge first, affine, where `λ_L < 1`) | unfold (§2.2 (iv), R-937, at the held `K`) | sheet 2 | clamp (REQ-ENC-030's projection, §2.2 (vi)), blank | as the linear option's unfold |
| | below the floor, held `K ≤ 0` at `L_z ≠ 0` | clamp below the floor (§2.2 (v)): the held `K` set to `L_z²/(2I)` at the pixel's own `L_z` and shape | the rigid rotation | blank | not counted |
| Burrau family: the δm strip `Φ_strip,δm` (§4.5) | `θ`, both edges | as on `Φ_θ,K` for a target symmetric in bodies 1 and 2 (the default); for an asymmetric target, past `π/4` affine to the zero-mass end `θ = π/2` (`b → 0`), then saturate | twins (symmetric target) or new systems (asymmetric) | clamp, blank | as on `Φ_θ,K`; asymmetric, past `π/4` counted |
| | `θ` at the triangle's collision end on a row with `v > 0` (`θ = 0`; `θ = π/2` for an asymmetric target, or past the reflection) | affine through the collision, then saturate (RQ-829, settled by physics review 5911915550 of PR #546 under R-932 and R-407 clause 3, option (a)): the collision is no true edge, `ν(θ)` being smooth and monotone through it with real systems on both sides, so past `θ = 0` (`ν > 1`, `a < 0`) and past `θ = π/2` (`ν < 0`, `b < 0`) the formula continues through the mirrored right triangles under the blended masses until a blended mass reaches `ε_m`, then the saturation below, which on every row with `v < 1` starts before the second collision (`θ = −π/2`, bodies 0 and 2; for the `θ = π/2` end `θ = π`, bodies 0 and 1); the column exactly at a collision is not a system, a validity failure of measure zero (coincident bodies, layer 3; R-955); on the row `v = 1` exactly every pixel has the target's masses and none reaches `ε_m`, so the continuation stays affine up to the second collision, and only on that row the pixels at or past it are not a system, of measure zero | the mirrored triangles under the blended masses, new labelled systems (no row inside the square shows them) | clamp, blank | counted (affine); the saturated part as below |
| | δm (`v`), both edges | affine to the zero-mass end, then saturate | the blend continues, `Σmᵢ = 1` kept, until a mass approaches 0 | clamp, blank | counted |
| mixed-axis (§1.3), Chart builder (render_gui_spec §G7) | each axis's edges | its own row above: a latent-coordinate axis affine; an energy-type axis as the invariant or lone rows; a mass ratio as its row; a Burrau dimension as the Burrau rows; any other axis is not offered until each of its edges declares a kind under clause 3 (*was:* "any other axis declares nothing and is blank until declared under clause 3") | per edge | per edge | per edge |
| any chart with a time axis | before `t = 0` | clamp to the starting state (R-938): a column of equal rows | the start, every row | blank | not counted |
| | past `T` | affine: the march continues (R-842) | the future | blank | counted |
| the third-body pair `(ξ, η)`, region D's chart (§6.6; *15 Nov 2026, TASK-M2-59*): the preset's window holds the lens unmasked, the mirror and the body swaps inside it (R-871) | the window's edges in `ξ` and `η` (each axis's natural type bounded, §6.6.6) | RQ-779's, still open (R-941, R-963), never blank by default (R-932, R-938) | the kind each edge declares under physics review (R-407 clause 3), no continuation invented (§6.6.6) | blank | inside the window every pixel is a system and counts, D the symmetry filter's output (§6.6.4); past an edge as its declared kind's count rule says (R-932) |

*Was (the δm strip's collision row while RQ-829 was open):* "`θ` at the triangle's end on a row with `v > 0` (`θ = 0`, and `θ = π/2` for an asymmetric target or past the reflection) | **RQ-829**: the blended masses stay above `ε_m` to the end, where bodies 0 and 1 (0 and 2) coincide, so the saturation above has no start before it, or squashes through it; the row above stands on the rows with `v ≤ 0` (Codex review of PR #546) | as RQ-829 rules | — | as RQ-829 rules" (applied until physics review 5911915550 of PR #546 settled RQ-829).

**The reflection past `π/4`, and its partner** (physics review 5910557865 of PR #546, F1). Swapping the legs is the
reflection across `y = x` together with relabelling bodies 1 and 2 (§4.2: the right angle at the origin, body 1 on +x,
body 2 on +y). A reflection reverses the sense of rotation, and §2.2's construction is equivariant under it with
`L_z → −L_z` (`J` anticommutes with a reflection; `w = r/√I` is equivariant), so the twin of the system at `(θ, L_z)`
is the one at `(π/2 − θ, −L_z)`. The reflection kind therefore maps the pixel's whole state onto the twin's, as a
pole-crossing axis shifts its partner: on `Φ_θ,L_z` it maps `(θ, v) ↦ (π/2 − θ, 1 − v)`, `L_z` being `(2v − 1)·λ_L·L_max`,
with the held `K` unchanged; on `Φ_θ,K` and `Φ_strip,K` it needs the held `L_z` to map to `−L_z`, and on the δm strip
the four momentum controls to map by the mirror and the relabel. Untilted those hidden coordinates are 0 (the held
`L_z`, R-633's table) or the rest start's, which the map leaves as they are, so the reflection is a repeat. Under a tilt
or slice that moves one of them (R-472 X3: the kinds act on the chart's own `(a, b)` only), the canonical point at
`π/2 − θ` does not carry the twin's hidden coordinate, the reflection is no repeat, and the edge is **affine** instead:
past `π/4` the formula continues through the other leg ordering, real systems, counted, to the zero-mass end at
`θ = π/2` (`b → 0`), then saturates. Which applies is decided exactly on the stored tilt and slice (the hidden
coordinate's offset and its tilt coefficients all zero, or not), never by a tolerance on a computed value, as R-554
decides the pole maps. **Past `θ = π/2`** the reflection gives `θ' = π/2 − θ < 0`, past the zero-mass end: the kinds
compose in order, the reflection first, then the `θ = 0` edge's kinds on `θ'` (affine down to where the smallest mass
is `ε_m`, then the saturation below), so the pixel is a repeat of the system that saturated `θ'` shows, and is not
counted (physics review 5910557865, F5). On the affine branch above (a tilt the reflection does not carry), past
`θ = π/2` is the `b → 0` end's saturation itself.

**The repeat past `r = π` on a tilted view** (Codex review of PR #546; *was*, the row's default and count cells: "repeat map: the
formula continues, the point at `(r, ê)` being the one at `(2π − r, −ê)` (§3.3; reduced mod `2π`)" and "not counted: a
repeat, resolved to its canonical point in `r ≤ π` (R-537)", on every view). A tilted pixel keeps its own hidden
offsets `d` (§3.3, "Pixels on `r = π` under a tilt"), linear in its `(a, b)`, and two pixels are one system only where
their full `(a, b, d)` decode to the same system (§3.3, "Under a tilt"). The repeat map moves `(a, b)` to the point at
radius `2π − r` opposite, the same shape, so it leaves `d` unchanged at every pixel only where the tilt adds no offset
along `(a, b)`, `N_a = 0` and `N_b = 0` for §3.3's `N`. As the decoder contract decides the pole maps (§ "Repeats and the
canonical point": on a tilted view a repeat is the same system only where the map leaves `d` unchanged, the pole maps
applying iff `N_a = 0` and `N_b = 0`, decided exactly on the stored angles; R-554, R-663), the repeat map applies iff
`N_a = 0` and `N_b = 0`, decided exactly on the stored angles, and otherwise the edge is **affine**: past `r = π` the
formula continues over the sphere again, each pixel the system its own `(a, b, d)` decodes, counted. A pixel whose
computed `d` equals exactly, at the float type and for every target, the `d` computed at its image point
`(2π − r, −ê)` (on the plane's level line through `n₀`, where the move is along no offset) is the same system as that
point and a repeat of it, not counted, `d` compared as REQ-CHART-090 compares it on `r = π`, with no tolerance and
`+0` equal to `−0` (R-408: each system once).

**The unfold's period on a tilted view** (Codex review of PR #546; *was*, the invariant rows' count cells: "sheet 2
counted inside `s ∈ [−½, 3/2)`; past it a repeat, not counted" and "sheet 2 counted inside `s̃ ∈ [−½, 3/2)`; past it a
repeat", on every view, which the lone `L_z` axis and `Φ_θ,L_z` inherit). The unfold folds `s` (or `s̃`) by its period
2, so the pixels at `(s, t)` and `(s + 2, t)` show the same `s'` on the same sheet; but a tilted pixel keeps its own
hidden offsets `d`, linear in its `(a, b)` (§3.3, "Under a tilt": `d = −(N_a·a + N_b·b)/N_d`), and the shift by a
period moves the unfolded axis: `a` on the invariant `(L_z, E)` and `(L_z, K)` charts, warped and linear (by 2 in `s`;
under the linear option by `4·L_max` in `L_z` at the pixel's own `L_max`, 2 in `s̃ = (1 + L_z/L_max)/2`), and `b` on
`Φ_θ,L_z` (§4.5's vertical axis, `s̃ = (1 + (2v − 1)·λ_L)/2`, so the period 2 in `s̃` is `2/λ_L` in `v`, `4·L_max` in
`L_z`); a lone `L_z` axis moves whichever of `a`, `b` it occupies. So `d` changes by `N_⊥` times that move, `N_⊥` the
unfolded axis's component of `N`: `N_a` where it is `a`, `N_b` where it is `b`. As the decoder contract decides θ's
period on the spherical map (§ "Repeats and the canonical point": on a tilted view a repeat is the same system only
where the map leaves `d` unchanged; `N_a = 0`, θ's period, decided exactly on the stored angles; R-554, R-655), a pixel
past the unfold's period is a repeat of its canonical pixel iff `N_⊥ = 0`, decided exactly on the stored angles, never
on a computed `N_⊥` (physics review 5913253688 of PR #546, F1; *was:* "and the shift by a period moves `a` (by 2 in `s`; under the linear
option by `4·L_max` in `L_z` at the pixel's own `L_max`, 2 in `s̃ = (1 + L_z/L_max)/2`), so `d` changes by `N_a` times
that move", and "a pixel past the unfold's period is a repeat of its canonical pixel iff `N_a = 0`, decided exactly on
the stored angles, never on a computed `N_a`"). Otherwise it is the system its own `(a, b, d)` decodes, the folded
`s'` and sheet at its own offsets, and it is counted (R-408): `t` is not periodic, so there is no lattice of exact
repeats as R-655's. A pixel whose computed `d` equals exactly, at the float type and for every target, the `d` computed
at its canonical pixel (a tiny non-zero stored `N_⊥` can round the move away; *was:* "`N_a`", physics review 5913253688 of PR #546, F1) is the same system as that pixel and a
repeat of it, not counted, `d` compared as REQ-CHART-090 compares it on `r = π`, with no tolerance and `+0` equal to
`−0`, as "The repeat past `r = π` on a tilted view" above has it (physics review 5912673263 of PR #546, F1 and F2;
*was:* "under the linear option by `2·L_max` in `L_z`", and "no pixel of the period shows it (the move along the period
changes `d` by a non-zero multiple of `N_a`, and `t` is not periodic, so there is no lattice of exact repeats as
R-655's), and it is counted (R-408)."). The unfold within the period, sheet 2 on `[−½, 0)` and `(1, 3/2)`, is unchanged: those pixels are their own
systems at any tilt.

**Saturate on a zero-mass end** (defined per R-72; R-932, R-938, R-941; physics-reviewed under R-407's clause 3). An
edge whose continuation ends where a mass reaches 0, a system that is not valid, saturates there: its formula continues,
affine, until the smallest mass reaches `ε_m`, the ternary plot's interior buffer (validation layer 2); past that point
the axis's coordinate is squashed toward the point where that mass would reach `ε_M01`, the `M01_TINY` fence
(`principia_dd_decoder.md` §3.1, REQ-DEC-039), and never reaches it. The band is `(ε_M01, ε_m)`, both ends existing
constants, so no new constant is introduced, and every saturated mass stays above the fence, so `M₀₁` does too,
where the edge's own masses are above it (the general case below) (physics review 5910557865 of PR #546, F3; *was:*
"and every saturated mass stays above the fence, so `M₀₁` does too", physics review 5913512312 of PR #546, F2). With `σ` the edge's outward sign, `+1` on an edge past the axis's maximum and `−1` on one past its minimum (`θ` below
`θ_min` and `θ'` below 0, `ν` below `ν_min`, δm's `v` below 0), every coordinate below is measured outward, `σ` times
the axis's coordinate, so that it increases away from the square on every edge. With `p_s` the outward coordinate
where the squash starts (where the smallest mass is `ε_m`, or the square's edge if that mass is already below `ε_m`
there) and `p₀` the outward coordinate where it would end, the first point past `p_s` on the continuation at which a
mass above `ε_M01` at `p_s` would reach `ε_M01`, a mass at or below `ε_M01` at `p_s` would reach 0, or `M₀₁`, if above
`ε_M01` at `p_s`, would reach `ε_M01`; `Δ = p₀ − p_s`, positive on every edge whose system is valid (every mass
positive there), `p₀` lying past `p_s` outward; and `d ≥ 0` the pixel's outward distance past `p_s`, `σ` times its
affine coordinate less the start's, the pixel shows the chart's formula at the axis coordinate `σ·p`, with

```
p = p_s + Δ·d/(Δ + d)
```

C¹ at `p_s` (`dp/dd = Δ²/(Δ + d)²`, 1 at `d = 0`: value and slope continue), strictly monotone in `d`, with
`p₀ − p = Δ²/(Δ + d) > 0` at every finite `d`, approaching `p₀` as `1/d`. Every mass, and `M₀₁`, is continuous in `p`,
and `p₀` is the first floor any of them crosses, so on `[p_s, p₀)` none has reached its floor: every mass stays
positive, a mass above `ε_M01` at the edge stays above it, and `M₀₁` stays above `ε_M01` wherever it is above it at the
edge. Where the edge's smallest mass is above `ε_M01`, the first condition binds and the band is `(ε_M01, ε_m)` as
above. Where it is at or below `ε_M01` (a δm target with a mass below `ε_M01`, valid because the strip checks only
`mᵢ > 0`, §5.1), the band is `(0, m_e)`, `m_e` that mass at the edge. Where bodies 0 and 1 are both small and `M₀₁`
reaches `ε_M01` first, the squash stops short at that point. No new constant is introduced.

A saturated pixel's masses are the chart's formula at `σ·p`, evaluated from its remaining distance `δ = Δ²/(Δ + d)` to
`p₀` and not from the rounded `p`, which reaches `p₀` at finite `d`. Each mass is its value at `p₀` plus its increment
from `p₀` to `p`. The increment is written in closed form as a product with the axis step `p − p₀ = −δ` (in the axis's
coordinate, `σ·p − σ·p₀ = −σ·δ`), so no difference of nearly equal values is formed. The binding quantity's value at
`p₀` is its floor exactly. This is the formula itself, not a linearisation: at `d = 0` it is the system at `p_s`.

Where a mass is affine in the coordinate, the increment is `−δ` times its constant rate. That holds on the mass
simplex, on the δm axis (the blend `(1 − v)·m_Burrau + v·m_target` being linear in `v`), and for `m₂ = (1 − ν)/2` on
`ν`. On the `ν` and `θ` axes the masses are not affine (§4.2, §4.5): in `ν`, `m₀ = (1 + ν²)/(2(1 + ν))`,
`m₁ = ν/(1 + ν)` and `m₂ = (1 − ν)/2`; with `t = tan(θ/2)`, `ν(θ) = (1 − t)/(1 + t)`, so `m₀ = (1 + t²)/(2(1 + t))`,
`m₁ = (1 − t)/2` and `m₂ = t/(1 + t)`. With `ν₀`, `θ₀`, `t₀` at `p₀` and `ν`, `θ`, `t` at the pixel, `ν − ν₀ = −σ·δ`
on a `ν` edge and `θ − θ₀ = −σ·δ` on a `θ` edge, the increments are:

- `m₁(ν) − m₁(ν₀) = (ν − ν₀)/((1 + ν₀)(1 + ν))`
- `m₀(ν) − m₀(ν₀) = (ν − ν₀)·(ν₀ν + ν₀ + ν − 1)/(2(1 + ν₀)(1 + ν))`
- `t − t₀ = sin((θ − θ₀)/2)/(cos(θ/2)·cos(θ₀/2))`, and from it `m₂(θ) − m₂(θ₀) = (t − t₀)/((1 + t₀)(1 + t))`,
  `m₁(θ) − m₁(θ₀) = −(t − t₀)/2` and `m₀(θ) − m₀(θ₀) = (t − t₀)·(t₀t + t₀ + t − 1)/(2(1 + t₀)(1 + t))`
- on the δm strip's `θ` edges, a blended mass's increment is `(1 − v)` times its Burrau mass's.

So the mass on the floor-0 arm is its increment, never 0 short of underflow, and a far pixel shows a system with a mass
that is small and above its floor. *Was (Codex review of PR
#546, thread PRRT_kwDOUpUQ5c63uuZn; physics review of head 1d272be):* "`p₀` the coordinate where it would be `ε_M01`,
`Δ = p₀ − p_s` and `d ≥ 0` the pixel's distance past `p_s` in the axis's own coordinate", "`p = p_s + Δ·d/(|Δ| + d)`",
"C¹ at `p_s` (value and slope continue), strictly monotone in `d`, inside the band between `p_s` and `p₀` at every
finite `d`, approaching `p₀` as `1/d`, so a far pixel shows a system with a mass that is small, above the fence." *Was (Gemini review of PR #546, thread
PRRT_kwDOUpUQ5c63vLV_; Codex review of PR #546, thread PRRT_kwDOUpUQ5c63vKx7; physics review of head 6055052):*
"With `p_s` the coordinate where the squash starts", "`p₀` the coordinate where it would end, the first point past `p_s`
on the affine continuation", "`d ≥ 0` the pixel's distance past `p_s` in the axis's own coordinate", "Every mass, and
`M₀₁`, is affine in `p`, so on `[p_s, p₀)` each lies between its values at `p_s` and `p₀` and none has crossed its
floor", "each as its value at `p₀` plus `δ` times its outward rate of decrease", "So the mass on the floor-0 arm is `δ`
times its rate".

On the mass simplex the squash acts on each raw barycentric mass of §4.5's bilinear map that is below 0, the side being
exactly where its buffered mass is `ε_m` (layer 2's narrowed simplex): with `dᵢ = −mᵢ` and
`Δ = (ε_m − ε_M01)/(1 − 3ε_m)`, `mᵢ' = −Δ·dᵢ/(Δ + dᵢ)`, so its buffered mass
`(1 − 3ε_m)·mᵢ' + ε_m = ε_M01 + (ε_m − ε_M01)·Δ/(Δ + dᵢ)` lies in `(ε_M01, ε_m]`; the raw
masses at or above 0 are scaled by the one factor `(1 − Σmⱼ')/(1 − Σmⱼ)`, the sums over the saturated masses, so the
raw masses still sum to 1 and a mass at 0 stays at 0. Past a side one mass saturates and the other two keep their ratio
(the pixel slides toward the side along the line through the opposite corner); past a corner two saturate; the map is
continuous across those regions and is the identity on the sides. Where two masses saturate (past a corner) their
sum is above `2ε_M01`, so past the corner `m₂ = 1` `M₀₁ > 2ε_M01`; where one saturates, the masses not saturated stay
buffered at or above `ε_m`, so `M₀₁ ≥ ε_m` there: no saturated pixel reaches the `M01_TINY` fence, and none rounds to a
zero mass. *Was (the first draft's band `(0, ε_m)`, replaced per physics review 5910557865, F3):* "until the smallest
mass reaches `ε_m`, the ternary plot's interior buffer (validation layer 2), reused as the band's width and not a new
constant; past that point the axis's coordinate is squashed toward the zero-mass end"; "`p₀` the zero-mass end"; "a
mass that is small, not zero"; "`Δ = ε_m/(1 − 3ε_m)`, … `ε_m·Δ/(Δ + dᵢ)` lies in `(0, ε_m)`"; "A pixel whose squashed
mass rounds to 0 at the chart's precision is a validity failure, not a system, as §2.1's one exception. Past the corner
`m₂ = 1` the two saturated masses' sum `M₀₁` can fall below `ε_M01` (`principia_dd_decoder.md` §3.1), where the decode
tags `DEGENERATE(M01_TINY)`: that region is RQ-827's, with the edge-reaching link's." **The count:** a saturated system has a
mass below `ε_m`, which no pixel of the square has on the simplex, and on the one-parameter Burrau axes lies past the
axis's range, so it is counted (R-932: counted where its system is not shown inside the square). *Was:* "…, but a
leg-swap twin." (physics review 5910557865, F2).

**Leg-swap twins past `ν`'s range are their own systems** (physics review 5910557865 of PR #546, F2). Past `ν_min` the
continuation runs through the leg-swapped twins `ν̃ = (1 − ν)/(1 + ν)` of shapes the square shows. Swapping the legs is
a body relabelling, "a distinct labelled system" (R-27, §4.5), and the `ν` charts are `DoubleCover`, shape × labelling:
inside the square both labellings of a shape whose `ν` and `ν̃` lie in range are counted. So past the range each pixel
is the labelled system it decodes, affine, with its own readout, trajectory and outcome, and it is counted; it is not a
repeat of its twin. *Was:* "Past `ν_min` the continuation runs through the leg-swapped twins `ν̃ = (1 − ν)/(1 + ν)` of
shapes (R-27): a pixel past `ν`'s range whose `ν` or `ν̃` lies in `[ν_min, ν_max]` is a repeat of the square's system,
not counted; of two pixels past the range that are each other's twins, the one with `ν ≥ √2 − 1` is canonical and
counted. At the defaults (`ν ∈ [1/32, 31/32]`) the shapes past `ν_max` are new, their twins below `1/63`, and past
`ν_min` some are twins of the square's and some new (the reasoning below)."

*Was (the table before R-932, the text in force for what is built until N13 and N14 land; TASK-M2-95 replaces it with the per-edge table above):*

| chart | horizontal axis | vertical axis | past `[0,1]²` | area statistics past `[0,1]²` (R-408) |
|---|---|---|---|---|
| `Latent` (§1.1), every flat or axis-aligned slice, `BodyPlane` | affine | affine | `z₀ + (2s−1)·q₁ + (2t−1)·q₂` continues; what fails validation is not a system, blank (R-470) | every visible pixel counts (primary ranges unbounded); a validity failure is forbidden |
| `ShapeSphere`, spherical map (§3.3) | θ periodic | φ pole-crossing (θ shifted by π) | the sphere continues over its poles; every pixel is a system `[0,1]²` holds | not counted: each such pixel is outside θ's or φ's primary range, the chart's `[0,1]` span; there is no one-hemisphere view, and a pixel inside the square at `(θ, π − φ)` is its own system (R-544). *Was (R-412, withdrawn by R-544):* "with one hemisphere drawn, φ's span is pole to equator and its continuation past the equator, the mirror hemisphere, is not counted" |
| `ShapeSphere`, exponential map (§3.3) | bounded (declares nothing) | bounded (declares nothing) | blank (R-470; the hatch only under Overlays › Chart › forbidden region and in the debug views); inside `[0,1]²` a pixel past `r = π` draws blank too, the domain's end (R-503), as does a tilted pixel whose `(a, b)` leave the square (R-512). *Was:* "hatched" | out of the count and the total (the domain's end); inside `[0,1]²`, a pixel past `r = π` is the domain's end too, and every pixel of the cap counts once, its mirror pixels distinct systems, straddling or not and at any tilt (R-544, §3.3). *Was (R-503's per-pixel fold and the tilted fold, withdrawn by R-544):* "a straddling cap counts a mirrored pair once"; under a tilt the mirror pair kept its fold iff `N_b = 0` |
| `InvariantLE / InvariantLK` (§2.1, the warp) | bounded | bounded below `t = 0`; past `t = 1` it continues, affine (R-508) | blank past `s`'s edges and below `t = 0`; past `t = 1` its systems are drawn (§2.1) | past a bounded edge out of the count and the total (the domain's end); past `t = 1` counted (R-508) |
| `MassSimplex`, the ternary plot (§4.5) | affine (R-407's sense: the formula continues; the map is bilinear) | affine (likewise) | the map continues; the negative-mass area is not a system, blank (R-470), by validation layer 2's narrowed simplex, buffered `mᵢ ≥ ε_m` (`(1−3ε_m)m + ε_m`), which is exactly raw `mᵢ ≥ 0`, so the blank starts at the edge of `[0,1]²` with no band | every visible pixel counts; its negative-mass states fail validity and count as forbidden, so "forbidden in view" rises with F3 off |
| Burrau family: the Euclid plane (§4.5) | `m` bounded (declares nothing) | `ν` bounded (declares nothing) | blank on every side (R-470) | out of the count and the total (the domain's end) |
| Burrau family: the `(ν, K)` chart (§4.3, §4.5) | `ν` bounded (declares nothing) | `K` bounded below `t = 0`, continuing past `t = 1` (the warp's `K(t)`, §2.1; R-508) | blank past `ν`'s edges and below `t = 0`; past `t = 1` its systems are drawn | past a bounded edge out of the count and the total; past `t = 1` counted (R-508) |
| Burrau family: `Φ_θ,K`, the strip `Φ_strip,K` (§4.5) | `θ` bounded (declares nothing) | `K` as on `(ν, K)` (R-508) | blank past `θ`'s edges and below `t = 0`; past `t = 1` its systems are drawn | past a bounded edge out of the count and the total; past `t = 1` counted (R-508) |
| Burrau family: `Φ_θ,L_z` (§4.5) | `θ` bounded (declares nothing) | `L_z` affine (linear at fixed `K`) | blank past `θ`'s edges (R-470); past `L_z`'s the formula continues and `L_z² > 2IK` is not a system, blank, by validation layer 2's `K ≥ L_z²/2I`; at the default `λ_L = 1` the blank starts at the edge of `[0,1]²`, with no band | past `θ`'s edges out of the count and the total; past `L_z`'s every visible pixel counts, its infeasible states forbidden |
| Burrau family: the δm strip `Φ_strip,δm` (§4.5) | `θ` bounded (declares nothing) | δm affine (the blend is linear in `v`) | blank past `θ`'s edges (R-470); past δm's the blend continues, `Σmᵢ = 1` kept, and the zero- or negative-mass area is not a system, blank, by validation layer 2's `mᵢ > 0` | past `θ`'s edges out of the count and the total; past δm's every visible pixel counts, its zero- or negative-mass states forbidden |
| mixed-axis (§1.3) | each axis its own: a latent-coordinate axis affine, a physical-quantity axis mapped linearly onto its range affine (R-408's port, B1; for an energy-type axis its linear option, R-471, whose default warp is bounded, §2.1), any other bounded unless declared | as horizontal | per axis | per axis, by its primary range |

*Was (the "past `[0,1]²`" column; stale values, replaced by R-470, applied per R-369; the hatch is only the optional
overlay Overlays › Chart › forbidden region and the debug views'):* `Latent`, "the fallback hatches what fails
validation"; `MassSimplex`, "the fallback hatches the negative-mass area … so the hatch starts at the edge of `[0,1]²`
with no band"; the Euclid plane, "hatched on every side"; `Φ_θ,L_z`, "hatched past `θ`'s edges; … the fallback
hatches `L_z² > 2IK` …; at the default `λ_L = 1` the hatch starts at the edge of `[0,1]²`"; the δm strip, "hatched
past `θ`'s edges; … the fallback hatches the zero- or negative-mass area".

**`Latent`, the flat slices and `BodyPlane` at any pan or zoom (R-570).** The table's first row holds wherever the view
goes: these charts have no limit on pan or zoom-out (`principia_chart_decoder_contract.md` Part 4, "Latent and flat
charts: no limit on pan or zoom-out"). Their `validate` accepts every `z`, so every pixel is a system at any pan or zoom
(at the decoder's default links with `μ_max` inside REQ-GEN-038's range, which is capped at 10, η_E off (`None`); R-597 and R-598, closing RQ-536 and RQ-552: under the edge-reaching simplex link a pixel with `M₀₁ < 10⁻⁵` is genuinely degenerate, and with η_E on a pixel whose target energy cannot be reached has no system at that energy; each draws as not a system, with the reason in the readout; *was:* "(at the decoder's default links, η_E off (`None`); RQ-536, RQ-552)"), and the row's "what fails validation" is empty for them. *15 Nov 2026 (R-932, R-955; TASK-M2-95):* the η_E pixel stays as written, a validity failure (R-955); the `M₀₁ < 10⁻⁵` pixel saturates under R-932, the table's third row, how it saturates being RQ-827, and until that is ruled it draws as R-597 has it. *17 Nov 2026 (R-1024, RQ-827 (a)):* it is clamped to the fence, not saturated, and not counted (the table's third row). Far from the origin
§0's links saturate (`σ` onto α and β, `q_max·(2σ − 1)` onto the momenta, `μ_max·tanh` onto the mass logits), so
neighbouring pixels decode to nearly the same system and the field goes flat. That is correct, not a bug: nothing
clamps, rescales, wraps or fades it, and each pixel is the system it decodes to (REQ-CHART-087). The saturation is per
coordinate, a consequence of §0 and not a new decision: a view goes flat only where every coordinate its plane varies is
saturated, so along each in-plane axis a strip a few units of `z` wide stays structured out to any distance, and a far
zoom-out shows thin structured bands along the axes between flat quadrants; that too is correct.

**The Burrau family's types** (defined per R-72, REQ-CHART-055; R-407 clause 3, physics-reviewed; TASK-M2-11). One
type per axis of every Burrau-family chart (§4.5), with the reasoning. *15 Nov 2026 (R-932, R-938; TASK-M2-95):* the
kinds are now per edge, in the table above, and the reasoning below is what they rest on; each bullet's choice of
bounded, which the twins side decided for both edges of an axis ("one type per axis"), gives way to the table's per-edge
kinds: `ν` and `θ` affine up to the domain's end (R-938's genuine continuation), the zero-mass end saturating, the
twins past `π/4` a reflection and those past `ν`'s range their own labelled systems, counted (*was:* "counted once"),
`m` clamped, `K` clamped below `t = 0`. The
bullets' "bounded", "not systems, drawn blank" and "fall out of the count and the total with the domain's end" are each
*was* where the table now differs.

- **`ν`, bounded** (the Euclid plane's vertical axis, the `(ν, K)` chart's horizontal). Past `(0, 1)` `a = 1 − ν² ≤ 0`
  or `b = 2ν ≤ 0`, a zero or negative mass: the domain's end. Between the chart's range and those ends the formula
  runs through real systems the chart does not show: past `ν_max` (default `31/32`) new shapes up to `ν = 1`, not
  twins, since their leg-swapped partners `(1 − ν)/(1 + ν) < 1/63` lie below the default `ν_min = 1/32` too; past
  `ν_min` shapes down to `ν = 0`, at the default some twins of shapes inside and some new. Inside `(0, 1)` but past the
  chart's range, the formula would continue through the leg swap's fixed point `√2 − 1` onto the leg-swapped twins of
  shapes a one-sided range already shows, changing the chart's `system_image` past `[0,1]²`, where it must add no
  multiplicity (R-408). Bounded is chosen anyway: one type per axis, and the twins side decides it; the real systems
  past the range are not systems, drawn blank (R-470), and fall out of the count and the total with the domain's end.
  Nothing is declared. *Was:* "are hatched and fall out" (stale value, replaced by R-470).
- **`m`, bounded** (the Euclid plane's horizontal axis). An annotation axis behind the exponential warp
  `m(u) = m_min·(m_max/m_min)^u`, a quantity behind a nonlinear warp, which declares nothing: its continuation would
  draw the same `ν` family again, adding no system (the decoder ignores `u`), and below `u = 0` `m < m_min`, where the
  default `m_min = 1` leaves no lattice point.
- **`K`, bounded below `t = 0` and continuing past `t = 1`** (R-508; *was:* "`K`, bounded"; the `(ν, K)` chart's and
  `Φ_θ,K`'s vertical axis). It is §2.1's `K(t) = K_max·t^γ_K`, bounded for
  §2.1's reasons: `t < 0` has no real value for a non-integer `γ_K` (an odd integer gives `K < 0`, an even one folds back
  onto `t > 0`). Its `t > 1` side (`K > K_max`, feasible at `L_z = 0`) continues, affine, as on the warp (R-508, ruling
  RQ-376), and is counted. *Was:* "is left undeclared, as on the warp (R-407, A5)". The table's rows for the warp, the
  `(ν, K)` chart and `Φ_θ,K` follow; *was:* "bounded" (the warp's vertical axis), "`K` bounded (declares nothing; the
  warp's `K(t)`, §2.1)", "`K` bounded (declares nothing; as on `(ν, K)`)", "hatched on every side; the `t > 1` side is
  not declared (§2.1)", "hatched on every side" and "out of the count and the total (the domain's end)". *5 Nov 2026
  (R-720):* on a floored axis it is `L_z²/2I + K(t)`, and a `K` above `K_max` inside the square is counted, as the `t > 1`
  side is (REQ-CHART-056).
- **`θ`, bounded** (every acute-angle chart's horizontal axis). The axis's edges are `θ_min` and `θ_max`
  (REQ-CHART-045), inside the domain `(0, π/4]`, so past each edge the formula first runs through real systems the
  chart does not show: below `u = 0` the thin triangles of `θ ∈ (0, θ_min)`, and above `u = 1`, where `θ_max < π/4`,
  the shapes of `(θ_max, π/4]`. Past `θ = 0` `ν ≥ 1`, so `a ≤ 0`, a zero or negative mass: the domain's end. Past
  `θ = π/4` the formula continues into the other leg ordering, real systems up to `θ = π/2` (then `b < 0`), on `Φ_θ,K`
  and `Φ_θ,L_z`, and on the δm strip for a target symmetric in bodies 1 and 2 (the default), the leg-swapped twins of
  the fold's own: continuing would undo the fold the axis exists for and count each shape twice (R-27, R-408). On a δm
  strip with an asymmetric target the blended masses do not swap with the legs, and the systems past `π/4` are new.
  Bounded is chosen anyway: one type per axis, and the twins side decides it; the real systems between the axis's
  edges and the domain's ends are not systems, drawn blank (R-470), and fall out of the count and the total with the
  domain's end. *Was:* "are hatched and fall out" (stale value, replaced by R-470).
- **`L_z`, affine** (`Φ_θ,L_z`'s vertical axis). At fixed `K`, `L_z(v) = (2v − 1)·λ_L·L_max(K)` is linear in `v`: a
  physical-quantity axis mapped linearly onto its range (R-408's port, B1, the Chart builder's `L_z` axis). Past its edge
  the formula continues, with `λ_L < 1` through feasible systems (new `L_z`, not seen in `[0,1]²`) up to
  `|L_z| = √(2IK)`; beyond, `K < K_min`, forbidden, not a system, drawn blank (R-470). *Was:* "and the fallback
  hatches it as forbidden" (stale value, replaced by R-470).
- **δm, affine** (the δm strip's vertical axis). The blend `(1 − v)·m_Burrau + v·m_target` is linear in `v` and keeps
  `Σmᵢ = 1`, as the ternary plot's map continues (R-407, A7): past `v = 1` it runs on beyond `m_target`, past `v = 0`
  back beyond `m_Burrau`, through new mass vectors on the column's own triangle, until a mass reaches 0; the zero-
  or negative-mass area is not a system, drawn blank (R-470; *was:* "the fallback hatches the zero- or negative-mass
  area", a stale value, replaced by R-470). No pixel past δm's edge is a twin of one inside: the leg swap changes the
  triangle, which the column holds fixed (at `θ = π/4`, where it does not, `m₁ = m₂` on the whole column for a target
  symmetric in bodies 1 and 2, the default; with an asymmetric target the column's swapped masses lie off the column).

Every edge of every Burrau-family chart now declares a kind (the table above; R-932, R-938), each needing the physics
reviewer's approval (R-407, clause 3); `L_z` past `±√(2IK)` unfolds onto R-937's sheet, and the δm strip's and the
acute angle's zero-mass ends saturate. *Was:* "Only `L_z` and δm are given more than bounded; each needs the physics
reviewer's approval (R-407, clause 3)." The kinds are declared in code with the extension mechanism, by the chart
addendum's N14 and TASK-M8-87 (REQ-CHART-054; *was:* "The types are declared in code with the extension mechanism, by
TASK-M8-05 (REQ-CHART-054).").

A custom chart (the Chart builder, render_gui_spec §G7) takes its axes' types: a latent direction is affine, a
physical-quantity axis mapped linearly onto its range is affine, a Burrau dimension takes the Burrau family's types.
B1 covers §G7's non-periodic quantities (energy, `L_z`, virial ratio, mass ratio); an angle-like quantity declared
affine would redraw systems and count them again (R-408), so it is typed only by a declaration under physics review.
Such a physical-quantity axis is linear in its quantity over its range, so past its edge its formula continues and what cannot be reached is not a system, drawn
blank (R-470; *was:* "the fallback hatches what cannot be reached", a stale value, replaced by R-470) (energy beyond
what is reachable); under R-408 its primary range is unbounded, so those pixels fail validity and count
as forbidden. (R-407's port first made it bounded, by clause 1's default; corrected in R-408's port, B1, applied per
R-369 and flagged to the human.) **Under R-471 an energy-type axis (energy, `L_z`) is warped by default** (§2.1); B1's affine axis is its
linear option. Under R-506 the virial ratio and the mass ratio follow the same rule, warped to their feasible ranges by default (§2.1),
B1 their linear option. *Was:* "The virial ratio and the mass ratio are not energy-type
axes, and B1 stands for them." Their edges take the table's kinds (R-932, TASK-M2-95): a warped energy-type axis
clamps below `t = 0` and continues past `t = 1`, its `L_z` edges unfolding; a warped mass ratio continues, affine, to the end the link realises and clamps there (*was:* "a warped mass ratio
clamps to its edge rows");
under the linear option the feasibility edge takes the table's kinds (the unfold, the clamp below the floor, the
mass ratio's clamp to its nearest realised ratio), so "what cannot be reached" is no longer drawn blank or counted
forbidden on these axes. *Was:* "and so bounded past its edge, its pixels there the domain's end, out of the count and
the total" (the energy-type axis, warped); "and bounded past their edges" (the virial ratio and the mass ratio,
warped); the paragraph's "past its edge its formula continues and what cannot be reached is not a system, drawn blank
… so those pixels fail validity and count as forbidden" stands only for a linear physical-quantity axis whose edge has
no declared kind, which no axis of the Chart builder's list now is. A quantity behind a nonlinear warp (§1.3) is not
offered until each of its edges declares a kind under physics review (R-932: blank is no edge's default; physics
review 5910557865, F8; *was:* "declares nothing and is blank until a declaration under physics review", and before it
"declares nothing and is bounded"). A new
declaration beyond blank, or a new kind, needs physics review (R-407, clause 3; *was:* "A new declaration beyond
bounded, or a new extension type").

**Primary ranges and area statistics (R-408, R-932).** Each kind's primary range: wrap (periodic), one period of that axis (for the
shape sphere's `θ = 2π·s`, the chart's `[0,1]` span); pole-crossing, pole to pole (for the shape sphere's φ, the chart's
`[0,1]` span, the whole sphere, R-544); affine, unbounded, or up to the true end where the edge declares one; unfold,
one period of the two sheets, centred on the square (`s ∈ [−½, 3/2)`, §2.2 (iv)); reflection and a repeat map, the side
the chart shows (`θ ≤ π/4`; the disc `r ≤ π`); clamp, the edge itself, a clamped pixel lying outside; saturate, the
band, counted where the edge declares its systems not shown inside the square, as every saturating edge in the table
does; blank, the domain's end. *Was:* "Each type's primary range: periodic, … affine, unbounded; bounded, its domain." A new periodic or pole-crossing
declaration states its own primary range under physics review (R-407, clause 3); RQ-264 was that case for φ with one
hemisphere drawn, which R-412 ruled (pole to equator) and R-544 withdraws with the hemisphere toggle. *Was:* "for the
shape sphere's φ with both hemispheres drawn, the chart's `[0,1]` span"; "and R-412 rules it: pole to equator, the
chart's `[0,1]` span". "Forbidden in view" and the area statistics count a visible pixel only if every axis is inside its primary
range, so each system counts once; a pixel hatched where the domain ends (a bounded axis past its edge, or a `Φ` that
fails) leaves both the count and the total; only states that fail the validity check count as forbidden; inside
`[0,1]²` nothing changes. A clamped pixel, a repeat and a saturated pixel whose system the square shows leave the count
and the total, as a repeat does (R-932). The classification order is `principia_chart_decoder_contract.md` Part 3's.

**Blank, and tilted charts (R-470, R-472).** A pixel this section calls hatched draws blank, the view's background, in
every view, the outcome view included, and is never a palette colour; the debug views hatch it, and elsewhere the hatch
is only the optional overlay Overlays › Chart › forbidden region (R-470). A tilted, sliced or saved chart extends by its
edges' kinds exactly as an untilted one (*was:* "by its axes' types"), the view state holding an override per edge,
saved with the view (R-942): under a tilt the types act on the chart's own parameters `(a, b) =` R's first
two rows times `(u, v)`, the hidden coordinate's offset being a latent control's or a mass logit's, affine
(`principia_chart_decoder_contract.md` Part 4, R-473); nothing is written per chart (R-472, X3).

**How these pixels are drawn (R-467) and where the extension matters (R-468).** In this section and its table
"hatched" names the classification, a pixel that is not a system. A presentation view draws such a pixel as the plain
canvas background, with no hatch, key or share; only the debug views hatch it, View › Debug view under its "forbidden
(chart)" key (`principia_chart_decoder_contract.md` Part 3, "How a pixel that is not a system is drawn"). Inside
`[0,1]²` every chart of this reference is valid by construction (§2.1's warp, §4.5's simplex map, the decoder accepting
any `z`): a hatched pixel there is a decoder bug, warned in the console, and REQ-CHART-058's sweep checks every chart for
none. The exponential map's pixels past `r = π` (§3.3) are now the table's repeat (R-932, R-938), drawn and
not counted, so the sweep's exception for them lasts only until N14 draws them (REQ-CHART-058 is not changed here). *Was:* "none, the exponential map's pixels past `r = π` (§3.3)
excepted: R-503's domain's end by design, blank and unwarned (R-554, ruling RQ-470)." Extension is free for the affine charts (`Latent`, every flat or axis-aligned slice, `BodyPlane`, tilted or
rotated: planes, the square only the current window onto them), which never produce a forbidden pixel; the types and
the fallback matter for the charts whose square is their whole domain (the warp, the mass simplex, the Burrau family,
the shape sphere's spherical map, whose periodic and pole-crossing types redraw its own systems) and for a custom
chart's physical-quantity axes (R-468, clause 4). The exponential map's square covers a disc of radius `s√2` about `n₀`; past it the
formula continues to `r = π` and past `r = π` repeats (the table), so its pixels there are systems. *Was:* "The
exponential map is bounded, per the table: its square covers a disc of radius `s√2` about `n₀`, and past it a pixel is
classified not a system and draws the background, though the sphere holds systems there." *Was (R-468 as first
ported):* "the shape sphere's redraws".

---

## 6. Composed charts — the registry's formulas, warps, pair maps, constraints and the third-body pair

*15 Nov 2026 (TASK-M2-59; REQ-CHART-094, REQ-CHART-095): ported from `principia_chart_composition.md` § 3, § 4, § 5,
§ 7.3 and § 8, as the rulings named at each place rule them. The registry's fields, the refusals and the labels are
`principia_chart_decoder_contract.md` Part 3 ("The primitive registry") and Part 5 ("Refusals and labels of a composed
chart"); this section gives the maths they declare. Bodies are 0-based (R-22) and shown 1-based (R-449); `r_ij =
‖r_i − r_j‖`, `γ_k` is the interior angle at body `k`, and `(ξ, η)` is § 6.6's plane.*

Under R-926 (items 5 to 8) and R-925 every composed chart is evaluated on the CPU in f64 and sent to the integrator as
records, the fragment taking the chart's generated closed-form code where it has some and the table route where it has
none, never a refusal (`principia_lowering_contract.md` Part 1). The inverse class below therefore says how a chart
point is computed, not which integrator route it takes.

### 6.1 The registry's rows

Each row is one entry of `chart_authoring::registry`. "Extension" is the row's natural type; under R-932 and R-942 each
row declares per edge the kinds allowed and a default, the defaults RQ-779's (R-963). An angle-like row's declaration
needs physics review (R-407 clause 3).

| Primitive | Forward | Inverse | Kind | Extension | Angle-like | Scale |
|---|---|---|---|---|---|---|
| latent direction `q` | `z·q` | closed, assigned | 1 | affine | no | free |
| `α` | §0.2 | closed, the link's | 2 | bounded | **yes** | free |
| `β` | §0.2: `β = z_β`, read modulo 2π (`periodic_beta`, R-898, R-919) | closed, the link's | 2 | periodic (§0.2's target, in force when TASK-M2-57 merges; until then bounded on `[0, π]`, `β = π·σ(z_β)`, the built decoder on `main`) | **yes** | free |
| shape `θ`, `φ` | §3.1 | closed on the pair, §3.2 | 2 | periodic, pole-crossing (existing) | **yes** | free |
| Jacobi `p_ρ`, `p_λ` components | §0.3 | closed, the link's | 2 | affine | no | gauge |
| third body `(ξ, η)` | § 6.6.2 | closed on the pair | 2 | `ξ` bounded, `η` bounded | no | free |
| interior angles `γ₀, γ₁` (any two of three) | `acos` of the law of cosines | closed on the pair, § 6.6.3 | 2 | bounded | **yes** | free |
| side ratios `r₀₂/r₀₁`, `r₁₂/r₀₁` | `r_ij` | closed on the pair, § 6.6.3 | 2 | bounded | no | free |
| Jacobi ratio `‖λ‖/‖ρ‖` | §0.2: `tan α·√(μ_ρ/μ_λ)` | closed, residual `β` | 2 | bounded | no | free |
| mass ratio `m_b/m_a` | `m` | closed, through the link (R-506 W3) | 2 | bounded warped, affine linear | no | free |
| mass control `z_μ` | `z` | closed, assigned | 1 | affine | no | free |
| Burrau `ν`, `θ` | §4.3 | closed, the embed | 4 | bounded | `θ` **yes** | free |
| body position, `BodyPlane` | this reference's | closed, the chart's | 2 | affine | no | free |
| `E` | `K + U` | closed, solve into momentum | 3 | warped bounded (R-471) | no | gauge |
| `K`, `KE` | §2.2 | closed, solve into momentum | 3 | bounded, `t > 1` continues (R-508) | no | gauge |
| `L_z` | `Σ m r ∧ v` | closed, solve into momentum | 3 | bounded warped, affine linear | no | gauge |
| virial ratio `Q` | by mapping, `principia_dd_decoder.md` §3.6: linear, or at `L_z = 0`, `2K/\|U\|`; warped at a held `L_z ≠ 0`, `2(K − L_z²/2I)/\|U\|` | closed; at `L_z = 0`, `K* = Q·\|U\|/2`; at a held `L_z ≠ 0` the warped axis reads above the floor (§2.1, R-817, R-877) | 3 (R-816, R-838) | bounded both sides | no | free |
| area, perimeter | the triangle's | numerical on the pair | 2 | bounded | no | gauge |
| `PE` (`U`) | §0.6 | **none**: no chart offers it until a warp and a residual are written (REQ-CHART-070) | 3 | — | no | gauge |
| `min(r_ij)/max(r_ij)`, any `min` or `max` of primitives | piecewise | **none** | — | — | no | free |
| pair binding energies | `½μv² − m_i m_j/r_ij` | **none** | — | — | no | gauge |
| `I`, `R`, `ℓ` | §0.5 | refused: **size-only** | — | — | no | size-only |

*15 Nov 2026 (TASK-M2-59; Codex review of PR #545): the `β` row follows §0.2, R-898 and R-919, which make `β` the periodic angle in the target TASK-M2-57 builds. Where the golden IC sits under periodic `β` is RQ-824's, and this row doesn't settle it. *17 Nov 2026 (R-1020, RQ-824 (b)):* settled: the link is the offset `β = z_β + π/2`, `z = 0` the golden IC; the row's `β = z_β` reads `β = z_β + π/2`, kept as written as its *Was*. Was:* "| `α`, `β` | §0.2 | closed, the link's | 2 | bounded | **yes** | free |".

**Classes** (`primitive_class`; `principia_chart_space_model.md` § 6b). Conserved, exact: the mass ratio, `z_μ`, `E` at
`ε = 0`, `L_z`, `L²E` and Hill's distance at frozen masses, Routh's `γ`. Conserved, approximate (labelled
`approximate`): the inner and outer elements `a, e, ω`, the periapsis ratio, the distance from Mardling–Aarseth,
Donnison's pairs, `v_∞/v_crit`, `b/b_max` and the parabolic `(q/a, apsidal angle)`. Unclassified, so a definition
requirement and no axis: the isosceles families with matched velocities, the orientation `u`, BHH's `(a, c; L)` and
Mikkola–Hietarinta's `(R, θ)`. Every other row of § 6.1 and § 6.2 is a state function. `I`, `R` and `ℓ` are state
functions fixed at `t = 0` by the gauge and not conserved. At `ε ≠ 0` the `E` axis is the Newtonian energy, which
softened dynamics doesn't conserve (R-791). No row with an inverse reads a trajectory functional.

### 6.2 The builder's longer list: axis rows

The owner's orders of 6 Nov 2026 (part C). Each closed row is a case of `prim_round_trip`, which tests
`forward(inverse(x)) = x` and reads back the row's holds. Only `SuvakovPlane` is a baked GPU pair, by its own ruling
(R-669); every row is evaluated on the CPU (R-926). An alias is refused beside what it aliases (`SameQuantity`).

| Primitive | Forward (`I = 1`) | Inverse | Verdict |
|---|---|---|---|
| Jacobi momentum polar `‖p_ρ‖, ∠p_ρ`, `‖p_λ‖, ∠p_λ` | `hypot`, `atan2` on `(q₀, q₁)`, `(q₂, q₃)` | closed on the pair: polar on the pair map (§ 6.3) | axis; `∠` periodic, angle-like |
| body `i`'s speed, heading in the canonical frame | `‖pᵢ‖/mᵢ`, `atan2(pᵢ)` | closed, the other Jacobi vector frozen; two body speeds as a pair are a circle intersection, two roots or none | axis; body 2 is `‖p_λ‖/m₂`, `∠p_λ`, an alias; heading is relative to `ρ̃` on `+x` |
| flight-path angle `γ_λ` | `atan2(x × p, x·p)` on `λ`, `p_λ` | closed, the configuration downstream | axis; `γ_ρ = ∠p_ρ`, an alias; `γ_λ = ∠p_λ − β` |
| third body's speed against escape | `√chazy_qk[2]`, `= √(K_λ‖λ‖/μ_λ)` at `I = 1`, `M = 1`: below 1 bound, 1 parabolic | closed, direction frozen | axis; not `k` |
| `K_ρ`, `K_λ` | `‖p‖²/2μ` per Jacobi vector | closed, scale of that vector | axis, gauge |
| `L_ρ`, `L_λ` | `x × p` per Jacobi vector | closed, one component | axis, gauge; "inner" is the pair 0, 1 by label |
| `h`, the dilation rate | `Σ m r·v` on the `I = 1` gauged state | closed: `v ← v + ((h* − h₀)/I)·r`, `L_z` kept | axis (R-848), its range a definition under physics review |
| `K_shape` | `K − h²/2I − L²/2I`, `r` and `Jr` being `m`-orthogonal | closed, scale of the rest | axis; `K_rot`, `K_dil` aliases of `L_z`, `h` |
| `L²E` | scale-free | closed, `E` or `L_z` frozen; pole at `L_z = 0`; with `E` frozen it loses `L_z`'s sign | axis at `ε = 0`; at `ε ≠ 0` a label |
| angle of C about AB's midpoint | `atan2(η, ξ)` | closed with a radius: polar on § 6.6's pair | axis, a pair map |
| bodies' polar angles about the COM | `φ₁ − φ₀`, `φ₂ − φ₀` | closed on the pair: a 2 × 2 solve from `Σ m r = 0`, then `I = 1` | axis on the quarter of the torus where every gap is under `π`, the COM inside the triangle; blank elsewhere; each corner of that region a line of shapes, a declared seam and `ray-degenerate` |
| signed area | `½ ρ × λ = n_w/(4√(μ_ρ μ_λ))` | closed, with an azimuth | axis; through zero is collinear, on the signed path |
| distance to one named shape | `acos(n̂·n̂_k)` | closed with azimuth: the exponential map's radius about `n̂_k` | axis, a pair map |
| elongation, principal-moment ratio | `I₁,₂ = (1 ∓ √(1 − n_w²))/2` | closed, one orientation | an alias of `\|n_w\|`, warped |
| inner share of `L_z` | `L_ρ/L_z` | closed, `L_z` frozen; pole at `L_z = 0`, tagged | axis |
| inner, outer elements `a, e, ω, f` | osculating, pairs by label | closed on a pair, a radial-sign branch; mean anomaly numerical | axes at a labelled pair, plus overlays (R-847) |
| Hill stability's distance | `c = L²\|E\|` against `f_k = IU²/2` at the Euler configuration with body `k` in the middle; the chart's labelled pair `(i, j)` is cut off from the third iff `c > max(f_i, f_j)` (R-844), `25/972` at equal masses, `M = 1` | at a frozen mass a warp of `L²E`; moving masses numerical | an alias of `L²E` at a frozen mass, a definition (R-72); blank at `E ≥ 0` and at `ε ≠ 0` (R-844), `L_z = 0` reading −1; Hill-stable means no exchange, not bound |
| isosceles vertex angle, a 1-D family | `γ₂ = 2 atan(1/(2η))` on `ξ = 0` | closed | axis of a 1-D chart |
| focal coordinates `Z`, `σ` | `Z = \|r₁ − r₂\|/(r₁ + r₂)`, `σ = (r₁ + r₂)/r₁₂`, foci `A₁`, `A₂` | closed on the pair: `τ = ±Zσ`, `ξ = στ/2`, `η = ½√((σ² − 1)(1 − τ²))`, `ξ ≥ 0` in D fixing the sign | axis, a pair map; `σ = 1` is the segment `A₁A₂`, a fold of the map |
| Routh `γ = Σ m_i m_j/M²` | threshold `1/27` | closed with one mass ratio held, a two-root branch | axis of mass |
| Tanikawa–Mikkola `(α₂, β₁)` | `V₁ = −2α₂R₁ + β₁R₂`, `V₂ = α₂R₁ − 2β₁R₂`, `V₃ = −V₁ − V₂`, `Rᵢ` from the true centre of mass | closed, linear | axis pair; `L = 0` at any mass; § 6.6.5's rescale per pixel |
| Li–Li–Liao 2020 collinear `(x₁, v₁)`, `(m₁, m₂)` | `r = (x₁, 1, 0)` on `x`, perpendicular `v`, `v₂ = −m₁v₁x₁/m₂`, `v₃` by `P = 0` | closed | axis pair, unscored |
| isosceles choreography `c₁…c₄` | `x₁ = −2c₁, x₂ = x₃ = c₁, y₂ = −y₃ = c₂, u₁ = 0, u₂ = −u₃ = c₃, v₁ = −2c₄, v₂ = v₃ = c₄` | closed on a 2-plane | axis pair on a named 2-plane, any two of the three |
| Orlov collinear `(k, φ)` | `k = K/\|U\|` and a velocity angle as the central body passes through the COM | closed | planar: a polar view of the Šuvakov plane at equal masses; rectilinear: a pair on the collinear momentum block |
| the scattering set (`v_∞/v_crit`, `b/b_max`, the binary phase, `u`, `d/a`, parabolic `(q/a, apsidal angle)`) | the scattering chart's | closed, its pull-in a stated convention | axes of the binary-plus-single chart (R-847, TASK-M2-79), their definitions filled at build under physics review |

Forward-only overlays, with no inverse: the distance to the nearest of a set of shapes, the periapsis ratio (outside
its domain at `a_in ≤ 0` or `e_out ≥ 1`), the distance from Mardling–Aarseth (outside its domain unless `a_in > 0`,
`a_out > 0` and `e_out < 1`; R-668 parks hierarchical triples), the binding ratio, the tidal ratio and the homology
radius outside D. Aliases: `K_rot` of `L_z`, `K_dil` of `h`, `Ψ` at `t = 0` of `∠p_λ`, the hierarchy ratio `ρ₂` of
`‖λ‖/‖ρ‖`, the homology radius in D of `r₁₂/r₀₁`, the McGehee-type coordinates of the shape arc and `h`. Definition
requirements (R-72), no axis until written: the isosceles families with matched velocities, the elliptic Lagrange
`(e, γ)` preset's start, Donnison's pairs, BHH's `(a, c; L)` and Mikkola–Hietarinta's `(R, θ)`.

### 6.3 Warps and pair maps

**Warps** (R-471, R-506, R-508): `t ∈ [0, 1]` to the axis's value; four, a closed set.

*Here, as in §6.1, "Extension" is the row's natural type in R-407's words ("bounded" there is not §5.4's blank); under
R-932 and R-942 each edge declares its kinds and default separately, drawn as §5.4's table has them (an energy warp's
`t < 0` side is §5.4's clamp) (physics review 5912756365 of PR #546, F1).*

| Warp | Formula | Extension it implies |
|---|---|---|
| linear | `lo + (hi − lo)·t` | affine |
| power | `lo + (hi − lo)·t^γ` | bounded below `t = 0` |
| log | `lo·(hi/lo)^t`, `lo, hi > 0` | bounded |
| feasibility | the axis's own (R-471, R-506, R-708; §2.1) | the axis's own |

R-506 stands over all of them: a physical-quantity axis is warped to its feasible range by default, linear as an
option. Power and log are options an axis takes on top where its own warp is written; on an energy-type axis they are
not offered until written (REQ-CHART-070's pattern). A warp is not lowered as a kind: its choice is a uniform word, as
`ShapeMap` is, and the linear arm is the expression `Mixed` computes today, the same operations in the same order, so an
existing chart's bits do not move.

**Pair maps.** `(a, b) = M(u, v)` sits before the chart's axes. The default is affine, the identity, and `M` is part of
the chart's type: `Paired<C, M>`, as `Tilted<C, P>` wraps a chart. Polar and log maps are general options and apply to
any pair; they are builder options built for no target (R-840).

*Here, as in §6.1, "Extension" is the row's natural type in R-407's words ("bounded" there is not §5.4's blank); under
R-932 and R-942 each edge declares its kinds and default separately, drawn as §5.4's table has them (an energy warp's
`t < 0` side is §5.4's clamp) (physics review 5912756365 of PR #546, F1).*

| Map | Formula | Extension | Note |
|---|---|---|---|
| affine | `(a, b) = (u, v)` | the axes' | the default; bitwise today's |
| polar about `c` | `a = c_a + r cos ϕ`, `b = c_b + r sin ϕ` | `ϕ` periodic, `r` pole-crossing with partner `ϕ`, or bounded | each declaration under physics review (R-407 clause 3); P-1's rings as a view |
| log-polar | `r = r₀ eᵗ` | `r` bounded | no origin in view |
| exponential | §3.3's form, about `n₀` | per edge: on the shape sphere §5.4's rows, the square's edges affine to `r = π` and `r = π` a repeat map (R-932, R-938); on the momentum and the mass sphere each edge's kind as TASK-M2-72 declares it under physics review (R-407 clause 3), its default RQ-779's (R-941, R-963), never blank by default (R-932) (physics review 5912756365 of PR #546, F1; *was:* "bounded") | generalised off the shape sphere: the momentum sphere `{p : L_z = L*, K = K*}` at a fixed configuration, `v = v^(L) + a·w`, and the mass sphere `(√m₀, √m₁, √m₂)`, each `ExpMap<Target>` and each a definition TASK-M2-72 writes under physics review (R-72), the momentum sphere's frame seeded by R-934's `w = r/√I` |

### 6.4 Constraints: hold `E`, hold `L_z`, and what they do to a momentum axis

Hold `E` is `η_E` (§0.6), a projection after the decode that scales `L_z` too. Hold `L_z` and both held, with their
computed form and their outcomes in order, are `principia_dd_decoder.md` §3.7 (R-836); the refusals are
`principia_chart_decoder_contract.md` Part 5. Neither is a warped axis, so R-708's "every pixel a system" does not bind:
an infeasible pixel under a hold, tilted or not, is a label per pixel.

- **`I = 1` and the paper scale.** `E·L²` is scale-invariant, so at `I = 1` both held fixes `L²|E|` over the window
  even where each paper-unit value varies with `ℓ`; a held `L*` or `E*` on region D takes `paper scale differs`.
- **The mirror under a held `L* ≠ 0`.** On the shape sphere `(θ, φ)` and `(θ, π − φ)` aren't mirror images, even for a
  mirror-symmetric `p_fixed`, since the mirror has `−L*`; §3's statement about mirror pairs is qualified so. Area
  statistics under a hold stay `measure: chart area`, the projection not being Liouville, and a statistic on a
  shape-sphere chart under a held `L* ≠ 0` counts two different systems' halves, so its legend must not read as a share
  of one whole sphere.
- **A momentum axis under a hold** reads the value it swept, not what is integrated: hold `E` already moves `q_k` off its
  label on `Mixed`, and hold `L_z` does the same. The label is `axis reads before the hold`, which also names a hidden
  `p_fixed` or `p_λ` tilt target.
- **Rank loss, three cases**, each `ray-degenerate`: (i) hold `L_z`, alone or with hold `E`, where the swept plane
  contains `Jr` (an axis plane at a real configuration, `(p_ρ,y, p_λ,y)` at a collinear shape); (ii) hold `E` alone on a
  plane through rest, since `η_E` is a radial projection centred on rest and sends the plane to rays; (iii) both held on
  a plane through a point with `δv = 0`, rest or a rigid rotation, since the scale is centred on `(L*/I)·Jr`. Rest is the
  default `z₀`, so a momentum–momentum plane at rest under both holds is one curve of systems. A plane through a rigid
  rotation that misses rest keeps rank 2 under hold `E` alone; a generic plane keeps rank 2; hold `L_z` alone through rest
  keeps it, being a shift and not a scale.

### 6.5 Where two axes can't be independent

Three groups: the same quantity (refused, `SameQuantity`), coupled by a map (the pair is legal, and says so), and
coupled by the state.

| # | Case | Why | What the chart does |
|---|---|---|---|
| 1 | Two of `γ₀, γ₁, γ₂` | `Σγ = π` | any two are the shape's two degrees of freedom; a third is no axis |
| 2 | Side ratios | `(r₀₂/r₀₁)·(r₁₂/r₀₂) = r₁₂/r₀₁` | two free; the triangle inequality couples their domain |
| 3 | `(L_z, E)`, `(L_z, K)` | `\|L_z\| ≤ √(2IK)`, §2.1 | the feasibility warp is a pair warp: `L_z`'s line reads `t` |
| 4 | A held `L_z` under a `K` axis | `K ≥ L_z²/2I` | R-708's additive floor: the axis's line reads the held value |
| 5 | Pole-crossing | R-407: the partner axis shifted by half its period | the extension couples the axes past the square |
| 6 | Polar at `r = 0`, exponential at `r = π` | the angle has no meaning there | `ray-degenerate` row or disc; `−n₀` counted once (REQ-CHART-078) |
| 7 | Kind 4 | one `ν` slaves configuration and mass | a configuration or mass axis beside it is a same-block collision, `Coupling`'s flag |
| 8 | Axes in one block, generic | `(θ, α)`, `(γ₀, θ)`, `(U, r₀₂/r₀₁)` | independent except on a curve where the Jacobian drops rank: the curve is the seam |
| 9 | Two momentum axes under a hold | § 6.4's three cases | `ray-degenerate` where it does; a label |
| 10 | Region D's edge | eq. 8 couples `ξ` and `η` | not a product domain: § 6.6.4 |
| 11 | Mass axis and a shape held | `m` moves `U` and the geometric shape | the axis states what it holds |
| 12 | Raw `z_μ` axis | softmax couples all three masses | the line is "identical except one mass logit" |
| 13 | Hold `E` and hold `L_z` | `2K* ≥ L*²`, `K* = E* − U` | a feasibility curve across the chart; a label per pixel |
| 14 | Interior angles at a collinear point | `γ₀ = γ₁ = 0` is every point between the two bodies | that corner is `ray-degenerate`, labelled |
| 15 | Any `gauge` axis against the paper's quantity | `I = 1` rescales per pixel | a row of `E` here is not a row of a paper's `E`: § 6.6.5 |
| 16 | Frozen hidden coordinates | six are frozen, each read as fixed | only `gauge` axes change meaning under a slice |
| 17 | Splits of `K` and `L_z` | `K = K_ρ + K_λ = K_dil + K_rot + K_shape`; `L_z = L_ρ + L_λ` | any two are free; refused only: two terms under a hold on their sum (`SameQuantity`) |
| 18 | `L_z`, `E`, `L²E`; `L_ρ`, `L_z`, the inner share | each of three is a function of the other two | any two are free, off `L_z = 0`; refused only: the pair under a hold on the third |
| 19 | `\|n_w\|`, elongation; signed area, elongation | `I₁I₂ = n_w²/4` at `I = 1` | `\|n_w\|` and elongation: one quantity, always, refused; signed area and elongation: one at a frozen mass, refused there |
| 20 | Body 2's speed or heading, `‖p_λ‖` or `∠p_λ`; `γ_ρ`, `∠p_ρ`; `h`, `K_dil`; `K_rot`, `L_z`; `v₂/v_esc`, `‖p_λ‖` at a frozen shape | aliases | refused |
| 21 | `h` and `L_z` under hold `E` | `r` and `Jr` are orthogonal, but `K_dil + K_rot ≤ K` | a feasible region, tagged, as row 3 |
| 22 | Polar angles about the COM at a collinear shape | a line of shapes on one point | `ray-degenerate`, a seam |
| 23 | The mass sphere's octant edge | `Σ mᵢ = 1` with every `mᵢ > 0` | past the edge the pixel takes its edge's declared kind, never blank by default (R-932, R-941); at the edge the mass simplex's narrowed simplex, `mᵢ ≥ ε_m` |

### 6.6 The third-body pair `(ξ, η)` and region D (REQ-CHART-095)

#### 6.6.1 The definition

From `principia_dd_validation_orbits.md` §1.6, the port of R-592: bodies at `A₁ = (−½, 0)`, `A₂ = (+½, 0)`,
`A₃ = (ξ, η)`, equal masses `m = 1` in the papers' equal-mass runs. **Region D** (eq. 8): `ξ ≥ 0`, `η ≥ 0`,
`(ξ + ½)² + η² ≤ 1`. At equal masses and `L_z = 0` the whole of D is one fundamental domain under relabelling and the
mirror: its edges are the isosceles line `ξ = 0`, the collinear line `η = 0` and the arc `r₀₂ = r₀₁`; its corners
`(0, 0)`, `(½, 0)`, the binary collision of bodies 1 and 2 (shown 2 and 3), and `(0, √3/2)`, the equilateral triangle.
`A₁, A₂, A₃` are bodies 0, 1, 2.

**The plane is geometric at every mass** (R-843, RQ-728 (a)): `(ξ, η)` is body 2's position with bodies 0 and 1 at
`(∓½, 0)`, centred on the pair's midpoint, not on its centre of mass. At unequal masses D holds one of six labellings,
one sector, and is labelled so: a statistic over D there says `one labelling`. "Blank and uncounted" outside D reached
equal masses at `L_z = 0` only, and no mask is built (R-871). AN-1 is its own preset, a half-disc of radius 10, with
`A₁A₂ = 1` not the longest side, the equal masses at `A₁, A₂` and `C` the third.

#### 6.6.2 The closed form, and its stereographic identity

In §0.2's terms `ρ = A₂ − A₁ = (1, 0)` and `λ` runs from the inner pair's centre of mass to `A₃`, which sits at `(d, 0)`:

```
d  = (m₁ − m₀)/(2 M₀₁)                        0 at equal masses
λ  = (ξ − d, η)
ℓ² = μ_ρ + μ_λ ((ξ − d)² + η²)               I before the gauge; 1/6 + (2/9)(ξ² + η²) at equal masses
ρ̃  = √μ_ρ (1, 0)/ℓ          λ̃ = √μ_λ (ξ − d, η)/ℓ
```

No `atan2` and no pole at `(0, 0)`; the canonical frame by construction, so `C` is a no-op. Past `η = 0` the decode takes
the signed `C`, so `η < 0` stays its own system and is never folded to `(ξ, −η)` (R-871; REQ-CHART-165). `d` stays: it is
the map into the geometric frame, entering only on the way to the Jacobi `λ`, and the chart's axes never carry it. For
bodies 0, 1, 2 in any frame, positions read as complex numbers, `ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`, and

```
ξ + iη = d + λ/ρ = d + √(μ_ρ/μ_λ)·(n_v + i n_w)/(1 + n_u)
```

The ratio is invariant under translation, rotation and scale; `η`'s sign is the triangle's orientation, the signed
`n̂`. The chart reads back by the ratio form, which is well conditioned; the sphere form is equal but loses digits at
large radius. **It is the shape sphere, stereographically:** with §3.1's `n = (u, v, w)`,
`(ξ − d, η) = √(μ_ρ/μ_λ)·(n_v, n_w)/(1 + n_u)`, the projection from `n = (−1, 0, 0)`, §3.1's collision of bodies 0 and 1.
So the plane is conformal to the mass-weighted sphere; at equal masses D's apex is the pole `L⁺` and the collinear row
`η = 0` the equator. The read-back tolerance is absolute, `|Δ| ≤ 4 eps·max(1, |ξ|, |η|)` in f64 and f32, read from the
blocks `ρ̃, λ̃`; read from particle positions it takes a measured tolerance (R-71).

#### 6.6.3 The pairs it carries

Each inverts into `(ξ, η)`, in the frame above:

```
angles:       η = 1/(cot γ₀ + cot γ₁)           ξ = −½ + η cot γ₀
side ratios:  ξ = (a² − b²)/2                   η = √(a² − (ξ + ½)²)     a = r₀₂/r₀₁, b = r₁₂/r₀₁
```

Each is `forward(inverse(x)) = x`. R-798's remap of PA-1's side ratios onto the shape sphere is this line followed by
§ 6.6.2's, one source (REQ-CHART-093, R-835). The angle pair's axes are unsigned, so it keeps `reduced domain` with no
filter (R-878).

#### 6.6.4 The domain

D is not a product of two ranges. The chart is **the plane, unmasked** (R-871, RQ-714 (b), with RQ-738's conflict 3
read as its (c)): the full disc rendered literally, any window holding the lens `(|ξ| + ½)² + η² ≤ 1` with the mirror
and all three body swaps declared, so the symmetry filter leaves exactly region D with no mask; the filter is on by
default for the paper-frame presets only (R-855). Its check, the owner's: at equal masses and rest, the filtered area
is 0.307, D's own (REQ-CHART-176). No region-D mask uniform is built, and R-554's exception is not extended. At unequal
masses and under eq. 3, D is a contour on the half the mirror leaves, not a filter's output (R-878, conflicts 4 and 9).

#### 6.6.5 The momentum, and the rescale per pixel

Two velocity presets: **exact rest**, `q = 0`, the default, and **eq. 3** the second preset (R-851): `ẋ₁ = ẋ₂ =
+5·10⁻⁵`, `ẋ₃ = −10⁻⁴`, every `ẏᵢ = 0`, an equal-mass near-rest start, its 19% spread at `I = 1` stated beside the
preset, never "fixed". Eq. 3 is not uniform at `I = 1`: the import's rescale makes it depend on the pixel. With
`p̂ᵢ = (mᵢ/M)·vᵢ/√(GM)` (the import's `unit_state`), then `C`'s `pᵢ ← √ℓ·pᵢ`:

```
q(ξ, η) = √ℓ(ξ, η)·q̄      q̄ = the Jacobi form of p̂      for eq. 3 at equal masses, q̄ = (0, 0, −10⁻⁴/(3√(3G)), 0) = (0, 0, −0.0745, 0)
```

`q̄` is at most four floats. At other masses `q̄` is the Jacobi form of `p̂` at those masses, after `C`'s boost (eq. 3 has
`P ≠ 0` there), a residual convention on the momentum block, which reads the configuration: kind 3's pattern. No
unequal-mass target runs eq. 3. **Every `gauge` quantity carries that `ℓ`, and so does time:**

```
E_paper = G M² E_c/ℓ      L_paper = √G M^{3/2} √ℓ L_c      t_paper = ℓ^{3/2} t_c/√(GM)
```

Over D at equal masses `ℓ` runs from 0.408 to 0.577, so one sim-key `T` is a horizon in paper time that varies by a
factor of 1.68 across the chart, and other masses differ. A paper's fixed horizon is read by running to the longest
pixel's horizon, `T_c = T_p √(GM)/ℓ_min^{3/2}`, `ℓ_min` at the run's masses, and cutting each pixel at its own, a tested
function; the kernel is untouched (R-845).

#### 6.6.6 Extension types

Both axes bounded, the row's natural type: past `η = 0` the mirror and past `ξ = 0` the relabelling, both now inside the
preset's window (§ 6.6.4), and neither a continuation anyone has defined (R-407: "Nothing ever invents a continuation").
Under R-932 and R-938 no chart leaves a blank area by default: past the window each edge takes the kind it declares,
under physics review (R-407 clause 3), its default RQ-779's (R-963), and an undeclared edge is not a default.

### 6.7 A numerical inverse: branch and seam

Quads resolve in any order, so a solve can't continue from its neighbour: **a branch rule is a pure function of the
pixel**, continuous only away from where branches fold, so a rule continuous everywhere can't be promised. Two
triangles share an `(area, perimeter)`, a chart must pick one, and where they merge the rule's choice flips: that curve
is the seam.

- **Solve:** Newton in f64 from a fixed list of starts, the pixel's own coordinates the only input.
- **Rule:** declared per registered inverse and recorded on the sample (`branch` in its record), with R-82's pattern for
  a tie: a fixed `δ`, and within it one named choice.
- **Seam:** the set where the solver's Jacobian changes sign, or where `branch` differs across a pixel edge; drawn on the
  Domain preview and offered as an overlay.
- **Failure:** no root is `decode_failed`, the label naming the solver; convergence is reported per pixel and every
  failure tagged, never dropped (R-849; REQ-CHART-145).
- **Round trip:** `forward(inverse(x)) = x` at every sample, within the tolerance, or the sample is `decode_failed`.

A copy that straddles a seam is flagged and kept, its spread read as `seam-contaminated`, and the seam masked from the
refinement criterion (`principia_dd_refinement_policy.md` § 1, § 6).
