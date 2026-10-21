# Chart reference — the maths, for implementation

**Scope:** the shared decoder, the chart views and the Burrau family, written for implementation.
This is a transcription for implementation, not a new derivation.

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
λ̃ = R̃ sin α · (cos β, sin β)                β ∈ [0, π]   (mirror fixed)

α = α_min + (π/2 − 2α_min)·σ(z_α)            σ = logistic sigmoid
β = π·σ(z_β)
```

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

With the canonical-frame decode both steps are no-ops away from the seam. Implement anyway, for
charts that bypass it:

```
φ = atan2(ρ_y, ρ_x)     rotate all rᵢ, pᵢ by R(−φ)
if λ̃_y < −δ_λ:          mirror (δ_λ = 1e−12; λ̃ = √μ_λ·λ)
|λ̃_y| ≤ δ_λ:            no mirror (deterministic tie-break — R-82)
```

**`φ` at the pole (R-437).** At `ρ = 0` (the collision of bodies 0 and 1) `φ` is 0, whatever the signs of the zeros,
so the rotation is the identity there: a select on `ρ_x = 0` and `ρ_y = 0`, not `atan2(0, 0)`, which GLSL.std.450 and
WGSL leave undefined (`principia_dd_decoder.md` §3.2, "Every `atan2` at a possible pole").

**After the mirror test, `λ̃_y ≥ −δ_λ`, not `λ̃_y ≥ 0` (R-438).** The deadband is left unmirrored (R-82), so the
encode's `β = atan2(|λ̃_y|, λ̃_x)` reads `|λ̃_y|`, and `β ∈ [0, π]` holds because it does, not by the mirror alone
(`principia_dd_encode.md` §3.1).

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
decode, never a refusal (`principia_dd_decoder.md` §3.7).

### 0.7 Degeneracy — every pixel gets a label

`DEGENERATE(reason)` for numerical failure; `COLLISION_T0(pair)` with `t_event = 0` if
`r_min(0) < r_coll`. **No pixel is ever rejected.**

---

## 1. The 8D latent chart — the reference coordinate system

```
z = ( z_α, z_β | z_q0, z_q1, z_q2, z_q3 | z_μ1, z_μ2 ) ∈ ℝ⁸
      config       momentum                 mass
```

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

For `(Lz, K)`: identical, with `K* = K(t)` directly. Simpler, since `K ≥ 0` is the natural
constraint.

**Past the unit square (R-407, R-508): `s` is bounded, and `t` bounded below `t = 0` and continuing past `t = 1`**
(*was:* "both axes are bounded"). Outside `[0,1]²` the warp has no feasibility guarantee:
`s ∉ [0,1]` gives `|L_z| = |2s − 1|·L_max(t) > L_max(t)`, so `K* < K_min`, infeasible for every `γ_K`; `t < 0` has no
real value for a non-integer `γ_K` (an odd integer gives `K < 0`, an even one folds back onto `t > 0`). So the warp
is bounded past `s`'s edges and below `t = 0` (§5.4). Its `t > 1` side (`K > K_max`) stays feasible, and it
**continues** (R-508, ruling RQ-376, the declaration under physics review R-407 clause 3 asks for): past `t = 1` the
formula runs on, `K = K_max·t^γ_K > K_max` and `L_max(t)` growing, so every pixel with `s ∈ [0,1]` is a system, an
affine side, counted as R-408 counts one; the `t` axis's primary range is `t ≥ 0`. Past `t < 0`, and the `s` axis
past either edge, the warp stays bounded. *Was:* "So the warp declares nothing beyond bounded and is hatched on
every side (§5.4). Its `t > 1` side (`K > K_max`) stays feasible; declaring it would be a later declaration, needing
physics review (R-407, clause 3), and is not made. Whether to declare it affine is queued: RQ-376 (R-469, §11)."

**Every energy-type axis is warped by default (R-471).** Axes of kind 3 (`E`, `L_z`, `K`, `KE`, `PE`;
`principia_chart_decoder_contract.md` Part 3) take this warp by default, wherever they appear: on these charts, the
Burrau family's `K` axes, a mixed-axis chart (§1.3) and a Chart-builder axis (render_gui_spec §G7), superseding R-408's
port B1 (a physical axis mapped linearly, affine) as the default. Alone, or beside a non-energy axis, an energy-type
axis takes its own line of the warp: `K` (`KE`) `= K_max·t^γ_K`, `E = U + K_max·t^γ_K`, `L_z = (2s − 1)·L_max` with
`L_max = √(2·I·K)` at the kinetic energy the chart holds fixed (as §4.5's `Φ_θ,L_z`). A `PE` axis has no warp written
yet: REQ-CHART-070 (R-72), and no chart offers one until it is. **The warp is evaluated per pixel,** with that pixel's
own `U`, `I` and masses: a tilt or slice that moves a hidden coordinate (masses, shape) changes `U` and `I` across the
window, and a warp computed once at the centre would put some pixels outside the feasible region; evaluated per pixel,
every pixel stays feasible under every tilt (R-471, E3; R-473, T5), each pixel whose own chart parameters lie in
`[0,1]²`: under Tip as sheet with one target and both angles set a window pixel's `(a, b)` can leave the square, the
axes' types then acting on it (§5.4), and such a pixel is past the edge, blank past a bounded one (R-512, ruling
RQ-402; R-471, H9). *Was:* "and what such a pixel draws is RQ-402 (R-471, H9)." Untilted, they are `z₀`'s at every pixel (§2, the
fixed geometry and masses), so nothing changes. **A linear mapping is an option per axis** (E2): the axis is affine,
mapped onto the warp's range at the slice centre (`K ∈ [0, K_max]`, `E ∈ [U₀, U₀ + K_max]`, `L_z ∈ ±√(2 I₀ K_max)`, a
builder axis onto its own range); its infeasible pixels in the window are not a system and draw blank (R-470), with no
decoder-bug warning, and the navigation stop (R-469) keeps the centre feasible. The mapping is recorded in the view
state and the export's sidecar. Under the linear option the axis is R-408's B1 axis, affine past its edge; warped, it
is bounded, as above, but past `t = 1`, where it continues (R-508).

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

**The infeasible pixel's label** (defined per R-72). A pixel with `K* < K_min` is tagged
`DEGENERATE(INVARIANT_INFEASIBLE)`, one of `DEGENERATE`'s enumerated causes (§0.7; `principia_dd_decoder.md` §2, "From
totality"), as `η_E`'s `E* < U` is `DEGENERATE(ENERGY_INFEASIBLE)` (`principia_dd_decoder.md` §3.7). Its
`sample_descriptor` (`principia_dd_simstate_payload.md` §2) records the reason as `state = decode_failed` (5),
`detail = 3` (other): no momentum realises `L_z` with less kinetic energy than the rigid rotation's, so the decoder
produces no physical IC there, which is what `decode_failed` records, and none of `detail`'s codes 0–2 (non-finite
output, degenerate configuration, invalid mass construction) describes it. It is a decode failure, not a dynamical
outcome: the sample is terminal at decode, kept, never dropped. Its momenta are finite, `a = 0` selected, so
`pᵢ = mᵢ·vᵢ^(L)`; nothing integrates them. The test is exactly `K* < L_z²/(2I)`, decided as `2·I·K* < L_z²` with no
rounding at the decision, so one input takes one branch at every precision and on every backend; `K* = K_min` is
feasible, with `a = 0`.

**Its precedence.** The decode runs mass → config → momentum, and a decode an earlier block has already tagged keeps
that tag (`principia_dd_decoder.md` §3.7): a pixel the mass block tags `DEGENERATE(M01_TINY)` keeps that tag, whatever
its `(L_z, K*)`. Within the momentum stage, infeasibility is decided on `(L_z, K*)` alone and takes precedence over the
seed rule's `DEGENERATE`, which a pixel carries only where its `(L_z, K*)` is feasible.

**Verification that can fail:** assert `Σpᵢ = 0`, `Lz(p) = Lz_target`, and `K(p) = K*`, all to
machine precision, over random `(u,v)`. Three independent constraints; the construction should hit
all three exactly.

---

## 3. Shape-sphere chart `(θ, φ)`

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
rotation, which the decode then undoes. The decode runs the shared `C` with its mirror step not taken (R-544):
`C(Φ(u, v))` so taken has masses `m_fixed` and momenta `p_fixed` at every pixel, to rounding, and its shape is the
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
REQ-CHART-078 makes it so: it draws blank and logs no decoder-bug warning (R-554, ruling RQ-470). The pixels with `r ≤ π` are the map's **cap**, the part of the sphere in view. Every pixel
exactly on `r = π` maps to the one point `−n0`; how such pixels count is a definition TASK-M2-28 writes (REQ-CHART-078,
R-72), and the fixtures avoid that circle.

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

**The exponential map's `system_image` is computed from its cap (R-503, closing RQ-371; R-544).** The cap is the
part of the sphere in view, `[0,1]²`'s pixels with `r ≤ π`; past `r = π` is the domain's end, and past the square the
map is bounded (§5.4). Every pixel of the cap is its own system and its mirror `w → −w` another (R-544), so the value
is **bijective** whether the cap lies within one hemisphere or straddles the equator; the pixels exactly on `r = π`,
which all map to `−n0`, count as REQ-CHART-078 defines. The chart computes the value whenever `n0`, `e1`, `e2` or `s`
change, and its label shows it. *Was (R-503's per-pixel fold, withdrawn by R-544):* where the cap straddles the
equator, n-to-1 with a per-pixel fold, a pixel counting 1 if its `w ≥ 0` or its mirror lies outside the cap, else 0,
its representative the pixel with `w > 0`; bijective only within one closed hemisphere.

**Under a tilt (R-473 T1, T2, J1; R-544).** Two pixels fold only when their full `(a, b, d)` decode to the same
system (`principia_chart_decoder_contract.md` Part 5's definition of `system_image`). Under a nonzero tilt a pixel at
window offset `(u, v)` maps to `(a, b, d) = R·(u, v, 0)`, and `d` is added to a momentum control or a mass logit (J1).
On the shape sphere a pixel and its mirror are distinct systems at any tilt, as they are untilted, so a tilted view is
one-to-one on either map, beyond the untilted chart's own identifications, and its pixels in view are the tilted view's
own (blank past a bounded edge, R-512). The chart recomputes the value whenever `n0`, `e1`, `e2`, `s`, `τ₁`, `τ₂` or the
tilt's targets change. The human's ruling on RQ-472, recorded as R-554, keeps an equator fold on a tilted view where
`N_b = 0`; that clash with R-544 is RQ-508, R-544 applied meanwhile and R-554 leaving the clause unapplied. A view
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
asymmetric target they are new systems. On both sides the acute angle is bounded anyway (§5.4): one type per axis, and
the twins side decides it.

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
  + (π/2 − 2α_min)·σ(z_α)` and `β = π·σ(z_β)` under the defaults), and builds the IC from `(α, β)` by §0.2: `ρ̃ = cos
  α·(1, 0)`, `λ̃ = sin α·(cos β, sin β)`, unweighted and reconstructed with the masses of `z0`'s mass block (§0.1), and
  the momenta of `z0`'s momentum block (§0.3). The comparison is per pixel (REQ-CHART-030), so the sweep is the one
  whose `(u, v) → (α, β)` map is the slice's own; a sweep linear in angle has the same `(α, β)` set at other pixels and
  is not it. The equality asserted: at each pixel, inside `[0,1]²` and past it, at f64 and at f32 and under every
  registered link, the slice's decode `D(z(u, v))` and the direct sweep give bitwise-equal `α`, `β` and `(m, r, p)`
  (`==`, so `+0 = −0`): the slice's `z(u, v)` is `z0` with `z_α = z0_α + (2u − 1)` and `z_β = z0_β + (2v − 1)`, exactly,
  up to the sign of a zero.
- **`BodyPlane` bitwise unchanged**, and the Python cross-check green

### 5.3 What to report per chart

Leaf count, `alpha` distribution, tree shape. **But leaf counts are slice-conditional to 4.3× —
compare within a chart, never across.** The `alpha` distribution is the safer cross-chart quantity.

**And the standing caution applies with extra force here:** a chart that produces a prettier
picture is not a better chart. The measurement is whether the criterion behaves consistently
across charts, not which chart looks best.

### 5.4 Past the unit square — each axis's extension type (R-407)

The rule is `principia_chart_decoder_contract.md` Part 3's (§ "Past the unit square — each axis's extension type
(R-407)"): each axis declares **affine** (continues), **periodic** (wraps), **pole-crossing** (continues over the pole,
the partner axis shifted by half its period) or **bounded** (not a system past its edge, drawn blank, the default);
outside `[0,1]²` a pixel whose `Φ` fails to evaluate, or whose state fails `validate(u, v)`, is not a system, drawn
blank (R-470; the hatch only under Overlays › Chart › forbidden region and in the debug views), and labelled
`decode_failed`; inside `[0,1]²` nothing changes. *Was:* "**bounded** (hatched past its edge, the default)" and "is
hatched as forbidden and labelled `decode_failed`" (stale values, replaced by R-470; applied per R-369). The `Chart` trait (§5.1) declares each axis's type beside `map`. The
existing charts:

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

**The Burrau family's types** (defined per R-72, REQ-CHART-055; R-407 clause 3, physics-reviewed; TASK-M2-11). One
type per axis of every Burrau-family chart (§4.5), with the reasoning:

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
  not declared (§2.1)", "hatched on every side" and "out of the count and the total (the domain's end)".
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

Only `L_z` and δm are given more than bounded; each needs the physics reviewer's approval (R-407, clause 3). The types
are declared in code with the extension mechanism, by TASK-M8-05 (REQ-CHART-054).

A custom chart (the Chart builder, render_gui_spec §G7) takes its axes' types: a latent direction is affine, a
physical-quantity axis mapped linearly onto its range is affine, a Burrau dimension takes the Burrau family's types.
B1 covers §G7's non-periodic quantities (energy, `L_z`, virial ratio, mass ratio); an angle-like quantity declared
affine would redraw systems and count them again (R-408), so it is typed only by a declaration under physics review.
Such a physical-quantity axis is linear in its quantity over its range, so past its edge its formula continues and what cannot be reached is not a system, drawn
blank (R-470; *was:* "the fallback hatches what cannot be reached", a stale value, replaced by R-470) (energy beyond
what is reachable); under R-408 its primary range is unbounded, so those pixels fail validity and count
as forbidden. (R-407's port first made it bounded, by clause 1's default; corrected in R-408's port, B1, applied per
R-369 and flagged to the human.) **Under R-471 an energy-type axis (energy, `L_z`) is warped by default** (§2.1) and
so bounded past its edge, its pixels there the domain's end, out of the count and the total; B1's affine axis is its
linear option. Under R-506 the virial ratio and the mass ratio follow the same rule, warped to their feasible ranges by default (§2.1)
and bounded past their edges, B1 their linear option. *Was:* "The virial ratio and the mass ratio are not energy-type
axes, and B1 stands for them." A quantity behind a nonlinear warp (§1.3) declares nothing and is bounded. A new
declaration beyond bounded, or a new extension type, needs physics review (R-407, clause 3).

**Primary ranges and area statistics (R-408).** Each type's primary range: periodic, one period of that axis (for the
shape sphere's `θ = 2π·s`, the chart's `[0,1]` span); pole-crossing, pole to pole (for the shape sphere's φ, the chart's
`[0,1]` span, the whole sphere, R-544); affine, unbounded; bounded, its domain. A new periodic or pole-crossing
declaration states its own primary range under physics review (R-407, clause 3); RQ-264 was that case for φ with one
hemisphere drawn, which R-412 ruled (pole to equator) and R-544 withdraws with the hemisphere toggle. *Was:* "for the
shape sphere's φ with both hemispheres drawn, the chart's `[0,1]` span"; "and R-412 rules it: pole to equator, the
chart's `[0,1]` span". "Forbidden in view" and the area statistics count a visible pixel only if every axis is inside its primary
range, so each system counts once; a pixel hatched where the domain ends (a bounded axis past its edge, or a `Φ` that
fails) leaves both the count and the total; only states that fail the validity check count as forbidden; inside
`[0,1]²` nothing changes. The classification order is `principia_chart_decoder_contract.md` Part 3's.

**Blank, and tilted charts (R-470, R-472).** A pixel this section calls hatched draws blank, the view's background, in
every view, the outcome view included, and is never a palette colour; the debug views hatch it, and elsewhere the hatch
is only the optional overlay Overlays › Chart › forbidden region (R-470). A tilted, sliced or saved chart extends by its
axes' types exactly as an untilted one: under a tilt the types act on the chart's own parameters `(a, b) =` R's first
two rows times `(u, v)`, the hidden coordinate's offset being a latent control's or a mass logit's, affine
(`principia_chart_decoder_contract.md` Part 4, R-473); nothing is written per chart (R-472, X3).

**How these pixels are drawn (R-467) and where the extension matters (R-468).** In this section and its table
"hatched" names the classification, a pixel that is not a system. A presentation view draws such a pixel as the plain
canvas background, with no hatch, key or share; only the debug views hatch it, View › Debug view under its "forbidden
(chart)" key (`principia_chart_decoder_contract.md` Part 3, "How a pixel that is not a system is drawn"). Inside
`[0,1]²` every chart of this reference is valid by construction (§2.1's warp, §4.5's simplex map, the decoder accepting
any `z`): a hatched pixel there is a decoder bug, warned in the console, and REQ-CHART-058's sweep checks every chart for
none, the exponential map's pixels past `r = π` (§3.3) excepted: R-503's domain's end by design, blank and unwarned
(R-554, ruling RQ-470). Extension is free for the affine charts (`Latent`, every flat or axis-aligned slice, `BodyPlane`, tilted or
rotated: planes, the square only the current window onto them), which never produce a forbidden pixel; the types and
the fallback matter for the charts whose square is their whole domain (the warp, the mass simplex, the Burrau family,
the shape sphere's spherical map, whose periodic and pole-crossing types redraw its own systems) and for a custom
chart's physical-quantity axes (R-468, clause 4). The exponential map is bounded, per the table: its square covers a
disc of radius `s√2` about `n₀`, and past it a pixel is classified not a system and draws the background, though the
sphere holds systems there. *Was (R-468 as first ported):* "the shape sphere's redraws".
