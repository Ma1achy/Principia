# Drill-down — the Decoder (Matter rung)

*Template drill-down. Shape: what it is → consolidated contract → the maths (implementation-independent formulae only) → seams as integration tests → unit tests → deferred. No WGSL, no struct layouts, no code structure — §3 is the implementation-independent maths, and the **one shared Rust source** (its CPU-f64 and GPU-f32 instantiations) must produce these numbers to precision (branch tags bit-exactly); everything else (WGSL accessors, struct layouts, code structure) is the agent's.*

---

## 1. What it is

The **Matter rung**: `z → mass × config × momentum → canonicalise → (m, r, p)`. It turns a point of the 8D manifold into three bodies. It is the first real artefact after the debug substrate — if it is wrong, everything downstream is noise. It runs from **one Rust source** in three roles: the f32 compute **kernel** (Rust → SPIR-V), the f64 (or higher) CPU path (lock, inspector, deep-zoom `x₀`/`J_D`), and — inverted — inside encode. Under the substrate (`principia_spike_brief.md`) these are not three transcriptions of one definition but one source monomorphised/instantiated three ways, so decode-logic drift between them is *structurally impossible*.

---

## 2. Consolidated contract (every clause that binds this component)

From the **chart & decoder contract**: the decoder factorises, `Y ≅ Y_mass × Y_cfg × Y_mom` (2×2×4=8); block ordering fixed `z[0:2]=config, z[2:6]=momentum, z[6:8]=mass`; decode order **mass → config → momentum → derived invariants** (load-bearing: invariant axes solve downstream); links are registry entries — constraint-preserving, invertible with conditioned inverse, C¹, carrying log-det Jacobian; **no latent coordinate on a gauge direction**.

From the **core design + integrator contracts**: physics defined once, compiled twice — now *structurally* (one Rust source), so the decoder's f32/f64 difference is precision only **for continuous values**; its **branch decisions** (the `DEGENERATE` tags, feasibility gates) must be bit-identical across backends by the comparison-only rule (`principia_gpu_determinism_note.md`) — a decoder that tagged `DEGENERATE` on one backend and not another would fork the outcome. Output is `(m, r, p)` in the dimensionless system `G = M = I = 1`; the integrator never learns which chart produced its input (canonicalise is the one seam).

From the **lowering contract**: link selection and chart type are **baked** (monomorphised Rust variants — lowering Part 2, not WGSL snippets); `z₀, q₁, q₂`, slice values are **uniforms**; `DECODE_MODE` (full vs linearised) is a per-quad workgroup-uniform flag; mass is **always produced by the decode snippet** — `SimUniforms.m[3]` is advisory only.

From the **deep-zoom contract**: the linearised path replaces the decode with `x₀ + J_D·δ`; `J_D` is central differences **of the full chart→IC composite** in f64 — chart-agnostic, no per-axis-kind special-casing; error `O(h²)`; `|det J_D|` doubles as the measure weight.

From the **inverse-encode contract**: decoded states are **already canonical** (T2 exercises block inverses only); the block decodes must be invertible with the closed forms of §3; the invariant momentum construction *is* the canonical fibre choice (encode reuses decode).

From the **render contract**: the decode stage writes `ICDescriptor` (what the IC *is*) post-decode, pre-integration; the DECODE view — a fragment preset, the fragment-side decode of `ctx.chart.z` (`principia_colour_composition.md` §6; R-75), not a kernel mode — certifies this rung before any physics exists.

From **totality** (scheduler/render): invalid decodes are **tagged, never dropped** — `DEGENERATE(reason)` is a decode-time terminal label with a narrow, enumerated cause set.

---

## 3. The maths

Exact. The one shared source, at either precision, must produce these numbers; continuous values agree to precision, branch tags bit-exactly.

**Chart constants.** `μ_max = 5` and `q_max = 2` (settled, R-10). The formulae below keep the symbols.
`α_min = 0` (R-21): full-sphere coverage.

### 3.1 Mass

```
(m₀, m₁, m₂) = softmax(0, μ₁, μ₂),      μₖ = μ_max · tanh(z_μk)
M₀₁ = m₀ + m₁ ;   if M₀₁ < ε  →  DEGENERATE(M01_TINY)
```

Body 0 is the softmax reference (logit 0). Equal masses ⇔ `z_μ = 0`. Burrau point: `μ₁ = log(b/c)`, `μ₂ = log(a/c)`.

### 3.2 Configuration — hyperspherical mass-weighted Jacobi

Mass-weighted Jacobi vectors and reduced masses:

```
ρ̃ = √μ_ρ · (r₁ − r₀)                          μ_ρ = m₀m₁ / M₀₁
λ̃ = √μ_λ · (r₂ − (m₀r₀ + m₁r₁)/M₀₁)           μ_λ = m₂ · M₀₁
I  = ‖ρ̃‖² + ‖λ̃‖²
```

**Canonical-frame decode** (the recommended path — rotation and scale gauged by construction):

```
ρ̃ = R̃ cos α · (1, 0)         λ̃ = R̃ sin α · (cos β, sin β)
R̃ = 1  (scale gauge = I = 1)      β ∈ [0, π]  (mirror fixed by construction)

α = (π/2) · σ(z_α)          β = π · σ(z_β)
```

**`α_min = 0` (R-21).** The general link is $\alpha = \alpha_{\min} + \left(\tfrac{\pi}{2} - 2\alpha_{\min}\right)\sigma(z_\alpha)$.
It is not a numerical guard: nothing divides by α, and a non-zero value would only excise a polar cap. The form above is
its $\alpha_{\min} = 0$ case, the one in force. The paragraph below describes it.

Both `α`-poles are **represented, not excised** — the old `α_min` cap is removed for full-sphere coverage. `α→0` puts body 2 at the inner-pair CoM (`‖λ̃‖→0`, azimuth `β` undefined *at the point*); `α→π/2` collapses the inner pair (`‖ρ̃‖→0` — a binary collision, `U→−∞`, and the rotation pin undefined). These are *coordinate/collision* degeneracies the pipeline already fences — the collision detector, the conditioning readout, and the saturation flags — not a range the chart carves out. Finite `z` never reaches the exact poles (`α = (π/2)σ(z_α) ∈ (0, π/2)` strictly); a config ingested *at* a pole is caught by those flags, and exact-pole `atan2(0,0)=0` yields a deterministic canonical value rather than a NaN.

> **⚠ Orientation gotcha (agents will assume the opposite):** `‖ρ̃‖ = cos α`, so **small α = LARGE inner-pair separation**; `α → π/2` = tight inner binary + distant third body (hierarchical). The α-sphere colouring depends on this. Test 5.9 pins it.

**Unweight and reconstruct** (produces CoM = 0 identically):

```
ρ = ρ̃ / √μ_ρ          λ = λ̃ / √μ_λ
r₀₁ = −m₂ λ            r₂ = M₀₁ λ
r₀  = r₀₁ − (m₁/M₀₁) ρ      r₁ = r₀₁ + (m₀/M₀₁) ρ
```

(Identities: `r₁ − r₀ = ρ`; inner CoM `= r₀₁`; total CoM `= M₀₁ r₀₁ + m₂ r₂ = 0`; `r₂ − r₀₁ = (M₀₁+m₂)λ = λ` since ΣM = 1.)

### 3.3 Canonicalise

Only needed when the input did **not** come through the canonical frame (physical-frame charts, encode):

```
rotate all rᵢ, pᵢ by R(−φ),  φ = atan2(ρ_y, ρ_x)
mirror (full state: every rᵢ AND every pᵢ, y → −y)  iff λ̃_y < −δ_λ   (λ̃ = √μ_λ·λ, δ_λ = 10⁻¹²)
   |λ̃_y| ≤ δ_λ → no mirror   (the one mirror test, in encode's frame — R-82)
```

With the canonical-frame decode both are no-ops away from the seam — and **that no-op property is a test** (5.5).

### 3.4 Momentum

**Free decode** (4 controls → physical Jacobi momenta `(p_ρ, p_λ) ∈ ℝ⁴`):

```
qₖ = q_max · (2σ(z_qk) − 1)
```

**Jacobi-to-particle** (Σpᵢ = 0 identically):

```
p₀ = −p_ρ − (m₀/M₀₁) p_λ
p₁ = +p_ρ − (m₁/M₀₁) p_λ
p₂ = p_λ
```

Momenta receive the **same** rotation/mirror as positions (§3.3 — the full-state rule).

**Invariant construction** (the momentum stage under `(L_z, E)` / `(L_z, K)` charts; also the canonical fibre for encode): with `I = 1` at the canonical scale,

```
v = v⁽ᴸ⁾ + a·w
v⁽ᴸ⁾ = L_z · J r        (rigid rotation — the minimal-KE realisation of L_z)
a    = √( 2(K* − L_z²/2) )        feasible iff K* ≥ L_z²/2
w    = deterministic unit direction (mass-weighted norm 1, zero CoM drift, zero L_z),
       from the seeded fallback family: among seeds with ‖w⁽²⁾‖²_m > ε_w take the largest
       norm, ties by seed order; no seed qualifies → DEGENERATE   (R-82)
```

### 3.5 Scale gauge

```
rᵢ ← rᵢ / ℓ,     pᵢ ← √ℓ · pᵢ,     ℓ = √I
```

No-op when `R̃ = 1` was enforced (canonical frame). This is the same similarity map as encode's step 0 (`√ℓ = I^{1/4}` ✓ consistent with the inverse-encode contract).

### 3.6 ICDescriptor derived quantities (decode-time, pre-integration)

```
K₀ = Σᵢ ‖pᵢ‖² / 2mᵢ        V₀ = − Σ_{i<j} mᵢmⱼ / ‖rᵢ − rⱼ‖        (G = 1)
E₀ = K₀ + V₀ ;   virial_ratio = 2K₀ / |V₀| ;   ρ-magnitudes, ρ_ratio, ρ_angle, r_min_pair₀
```

`E₀` here must agree with the kernel's `E_0` at t=0 — that agreement is a cross-check view, not an assumption.

### 3.7 Energy normalisation (optional; keep-or-drop is decision B9)

A post-momentum rescale that enforces a target energy $E^*$. Compute $K_0$, check feasibility $E^* \ge U$, then

$$\mathbf p_i \leftarrow \eta_E\,\mathbf p_i, \qquad \eta_E = \sqrt{\frac{E^* - U}{K_0}}.$$

**Where it is allowed.** Only on charts that decode arbitrary momenta without enforcing an energy: the
free-Jacobi-momentum chart and the latent chart. **It must be disabled** on the invariant-momentum charts
$(L_z, E)$ and $(L_z, K)$, where energy is a chart coordinate or is fixed by the momentum construction.
There it would fight the invariant construction near the feasibility boundary, or, for $(L_z, E)$, collapse
the energy axis entirely. Each chart declares a boolean `forbids_energy_normalisation`, and chart-aware
validation (`principia_inverse_encode_contract.md`) refuses any view that combines such a chart with a
non-zero $E^*$ override.

---

## 4. Seams (its side of each — the integration-test list)

| Seam | The decoder's obligation | Integration test |
|---|---|---|
| **2** chart → decoder | accept any well-posed chart's `z` (or chart-map output); apply only registered links | every chart in the lowering appendix decodes without special-casing; DECODE preset view per chart |
| **3** decoder → integrator | emit exactly `(m, r, p)`, canonical, `G=M=I=1` | identities test-suite (5.1–5.4) run on the kernel's actual input |
| **8** encode ↔ decode | be invertible per block; the invariant construction is the fibre; decoded states are canonical | T2 round-trip in physical units; ROUNDTRIP preset view dark everywhere except clamps/feasibility/mirror-tie |
| **13** generated sources | consume generated link functions (Rust, from the link registry — never inline transcendentals) | link swap recompiles and re-integrates; `|det J_D|` matches analytic link derivatives where closed forms exist |
| **1** precision (ring law) | one definition, f64 and f32 builds | linearised-vs-full agreement at quad centres; `O(h²)` error scaling with depth |

---

## 5. Unit tests

Golden anchor: **`z = 0` decodes to the canonical golden IC** — equal masses `(⅓,⅓,⅓)`, `α = π/4` (‖ρ̃‖ = ‖λ̃‖), `β = π/2`, rest (`q = 0`), `I = 1`. Assert its every derived value in f64 and f32.

1. **Identities (every decode, fuzzed z):** `Σmᵢ = 1`; total CoM `= 0`; `Σpᵢ = 0`; `I = 1` — all to precision-appropriate tolerance.
2. **Factorisation independence:** perturbing `z_μ` leaves `(α, β)` and `(p_ρ, p_λ)` bit-identical (and each block likewise) — the blocks genuinely do not couple except through the join.
3. **Range respect:** `α ∈ (0, π/2)`, `β ∈ [0, π]`, `|qₖ| ≤ q_max`, for all finite z including ±∞-ish saturation.
4. **Degenerate tagging:** `DEGENERATE(M01_TINY)` fires iff `M₀₁ < ε` and nothing else in the mass block does; degenerate outputs are tagged, never NaN, never dropped.
5. **Canonical no-op:** for canonical-frame decodes, applying §3.3 changes nothing (rotation angle ≈ 0, mirror not taken) away from the `λ̃_y = 0` seam; at the seam, the deadband makes the choice deterministic (same result twice).
6. **Reconstruction inverse:** from the emitted `(m, rᵢ)`, recompute `ρ̃, λ̃` via §3.2's definitions and recover `(α, β, R̃=1)` to tolerance — the decode and its own definitional inverse agree.
7. **Round-trip (T2, physical units):** `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys` over interior z; clamp-adjacent samples assert the flag, not the residual.
8. **Invariant construction:** for a grid of feasible `(L_z, K*)` at fixed config: `L_z(v) = L_z` and `K(v) = K*` to tolerance; infeasible pairs refused/tagged exactly on `K* < L_z²/2`; the largest-norm qualifying seed is taken (ties by seed order), and the degenerate-seed path fires only where no seed has `‖w⁽²⁾‖²_m > ε_w` (R-82).
9. **α-orientation:** `α → 0 ⇒ ‖r₁ − r₀‖` is *maximal* (and `‖λ‖` minimal); monotone crossover at `π/4`. Pins the gotcha.
10. **Scale-gauge idempotence:** §3.5 applied to canonical output is identity; applied to a deliberately scaled input yields `I = 1` and transforms `E, L_z` per the encode contract's table.
11. **Linearised decode:** at quad centre, `x₀ + J_D·0 = x₀ =` full decode exactly; at half-width, error vs full decode shrinks `∝ h²` across depths; identical behaviour for an affine chart and a curve chart (chart-agnosticity).
12. **Cross-backend:** the CPU-f64 and GPU-f32 instantiations (and browser-GPU-via-WGSL) agree on the golden IC and fuzzed interiors to f32 eps-scaled tolerance on continuous values; the degenerate/feasibility **branch tags are bit-identical** across all backends (the comparison-only rule — decoder branch inputs, like feasibility `K* ≥ L_z²/2`, must be comparisons against constants / single-rounded, never a runtime transcendental at the decision; `principia_gpu_determinism_note.md`).

---

## 6. Deferred / flagged

- **Body indexing — settled, 0-based (R-22).** The decode uses bodies **0, 1, 2** (softmax reference = body 0), and `ICDescriptor` names `m0, m1, m2`; pair id `k` is the side opposite body `k` (payload §2). The flag was raised because this is exactly the off-by-one that survives until a collision-pair label is wrong on screen.
- **Energy normalisation `η_E`**: specified in §3.7. Whether to keep it (flag-gated) or drop it is audit decision B9.
- **Quantised checkpoint storage** — moot under lockstep (no stored trajectory; temporal note, ratified).
- **KS-regularised state representation (v2)** — changes the decoder's output type; explicitly out of scope until then.

---

*One rung, three homes (f32 kernel, f64 CPU, inverted in encode), one set of numbers. Small α is a wide pair. Body zero is the reference. z = 0 is the golden IC. Everything invalid is tagged, and every backend takes the same branches (comparison-only rule), while continuous values diverge honestly.*
