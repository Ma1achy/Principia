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

**The `DEGENERATE` cause set** *(definition, R-72; REQ-DEC-038, written by TASK-M2-24)*. The set is closed: five
reasons, each raised by one block, and none other. Each is recorded in payload §2's `sample_descriptor` as
`state = decode_failed` (5) with the `detail` code below (`principia_dd_simstate_payload.md` §2, "`detail` failure
enum"):

| Reason | Raised by, and when | `decode_failed` `detail` |
|---|---|---|
| `M01_TINY` | the mass block: `M₀₁ < ε_M01` (§3.1), on the emitted masses of every chart, a chart that sets its masses directly included | 2, invalid mass construction |
| `NO_QUALIFYING_SEED` | the invariant-momentum construction: feasible targets, but no seed has `‖w⁽²⁾‖²_m > ε_w` (§3.4, R-82) | 1, degenerate configuration |
| `INVARIANT_INFEASIBLE` | the invariant-momentum construction: `K* < L_z²/(2I)` (`principia_chart_reference.md` §2.2) | 3, other |
| `ENERGY_INFEASIBLE` | `η_E`: `E* < U` (§3.7) | 3, other |
| `ENERGY_UNSCALABLE` | `η_E`: the rest start at `E* > U`, or the collision pole `U = −∞` (§3.7) | 3, other |

- **Why these codes.** `M01_TINY` is the mass simplex's boundary, a mass → 0, which is code 2's own example.
  `NO_QUALIFYING_SEED` fires only where every seed's projected norm is at most `ε_w`; at `I = 1` the largest is at
  least ½ (`principia_chart_reference.md` §2.2), so it is reached only on a configuration collapsed toward a point,
  which is code 1's degenerate configuration. The other three are a target the decoded configuration cannot meet, not
  a defect of the configuration or the masses: `INVARIANT_INFEASIBLE` is code 3 by `principia_chart_reference.md`
  §2.2, and the two `η_E` reasons are the same kind of failure, so code 3 too. `ENERGY_UNSCALABLE` at the collision pole
  is code 3, not 1: the pole without a target is no decode failure at all, but a valid IC already collided
  (payload §2, "A valid t=0 terminal is a real outcome"), so what fails there is the target.
- **Code 0 has no reason.** A non-finite decode output is not a `DEGENERATE` reason: the decode of a finite input
  inside the chart's domain is finite, and that is asserted, not tagged (`crates/kernel/tests/decode_totality.rs`).
  Code 0 is what the payload records for a decode whose output is non-finite all the same.
- **Precedence.** One decode carries at most one reason, the first block's: the mass block's, then the momentum
  stage's (`INVARIANT_INFEASIBLE` over `NO_QUALIFYING_SEED`, `principia_chart_reference.md` §2.2), then `η_E`'s, which
  runs after the momentum block (§3.7), and which no chart with a momentum construction allows.
- **Every pixel carries a label.** A decode is either tagged with one reason or untagged and goes on to integration,
  so no pixel is dropped and none is unlabelled (`principia_chart_reference.md` §0.7). A tagged decode keeps its
  outputs: its `(m, r, p)` finite, its descriptor never NaN, and infinite only at R-436's poles, `rho_ratio`'s `+∞`
  and `V_0`'s `−∞` (the collision pole).

**The decoder's branch inputs** *(enumerated per R-34; REQ-VAL-017, written by TASK-M2-24)*. Every decision the
decode makes that test 12 compares bit for bit across backends, with the value it reads and how that value is formed:

| Decision | Branch input, and its form | Where |
|---|---|---|
| `M01_TINY` | `M₀₁ = fma(m₁, 1, m₀)` against the constant `ε_M01`, from the emitted masses (§3.1, "The branch input"; its premise is RQ-298's) | `kernel::decode::mass` |
| `INVARIANT_INFEASIBLE` | the sign of `2·I·K* − L_z²`, exactly, from the operands' bits: binades and limb products, every product exact, no `fma`, no `√`, no division (applied meanwhile under RQ-298, which holds the "explicit `fma`" form) | `kernel::decode::invariant::infeasible` |
| `L_max`'s four steps down (the warp's edge columns) | the same exact sign at each step | `kernel::chart::invariant::l_max` |
| the seed, on a decoded configuration | `z_α ≤ 0`, read on the control's bits | `kernel::decode::invariant::decoded_seed` |
| the seed, on the Burrau family | `3ν ≤ 1`, by a two-sum of `2ν + ν`, comparisons only | `kernel::chart::burrau::seed` |
| `NO_QUALIFYING_SEED` | each seed's computed `‖w⁽²⁾‖²_m` against `ε_w`, strict comparisons in seed order; reached only through the rule on computed norms, on a configuration not from the canonical decode, whose choice near a tie is not claimed branch-identical (§3.4) | `kernel::decode::invariant::choose` |
| `η_E`'s outcome | `r_min_pair_0 = 0`, `E* < V₀`, `E* = V₀` and `s = 0`, comparisons of emitted values with no arithmetic at the decision (§3.7) | `kernel::decode::energy_norm::normalise` |
| `η_E`'s switch | the energy target's `Option`: on the GPU a uniform flag beside `E*` (R-25), the same for the whole dispatch | `kernel::stage::decode::target` |
| the mirror in `C` (the shape sphere's decode) | `λ̃_y < −δ_λ`, decided in exact mantissa-exponent arithmetic from the state's pair differences (§3.3; R-82). It writes no tag: its branch is compared bit for bit across backends by TASK-M2-07's `canon_gpu` (`canonicaliser_seam_sweep` at the seam states, `canonicaliser_mirror_tie` at the tie), and test 12 compares the shape sphere's decode it feeds | `kernel::canon::mirror_decision` |

The selects that choose a pole's value (§3.2's `atan2` poles, §3.6's `rho_angle` cases and R-436's pole values) read
comparisons against the constant 0 or `±π̂` of values already computed; they choose a stored value, not a tag.

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

**ε is the chart constant `ε_M01`** (REQ-DEC-039, a calibration, R-71). TASK-M2-02 proposes `ε_M01 = 10⁻⁵`, provisional
until the human confirms it at the M2 gate. It sits in a measured gap between the reachable `M₀₁` floors at
`μ_max = 5`: the edge-reaching simplex link's corner `m₂ → 1` piles up at `ε_μ − ε_μ²/4`, below it; the ternary plot's
floor `2ε_m = 2×10⁻⁴` and `softmax_tanh`'s `(1 + e^−μ_max)/(1 + e^−μ_max + e^μ_max) ≈ 6.7×10⁻³` lie above it. So the
corner fires with a tenfold margin at either precision, and the tag's onset falls where `M₀₁` still resolves with `z`
at f32, so the formula, not rounding, decides it (`fixtures/gates/m01_tiny/measurement.toml`).

**The branch input (R-34; test 12).** `M₀₁` is the one rounded sum of the emitted masses `m₀ + m₁`, compared against the
constant `ε_M01`. Under the edge-reaching simplex link, `M₀₁ = m₀ + m₁` written as an add is a multi-op branch input:
inlined, the link's last step `m₁ = t₇·t₁₃` meets the add as `m₀ + t₇·t₁₃`, which a contracting backend may fuse. That
fusion flips the tag where `M₀₁` lies within one rounding of `ε_M01`: TASK-M2-02 bisected the onset crossing on 20 000
lines `z₁ ∈ [6.3, 20.3]`, and fusion flips 77 of the 40 000 crossing points at f64 and 45 at f32. So the block forms
`M₀₁` as the explicit `fma(m₁, 1, m₀)` (R-34; gpu_determinism_note rules 2 and 6): the product `m₁·1` is exact, so it is
the same single rounding of `m₀ + m₁`, and no add is left for a backend to fuse. It reads only the emitted masses, so a
chart that sets masses directly gets the same test.

**Its premise.** The form holds only on a backend that keeps an explicit `fma` fused and does not fold the `×1`. Folding
`fma(x, 1, y)` to `x + y` is exact and standard (LLVM's InstCombine and Mesa's NIR both do it); after it, a contracting
backend can rebuild `fma(t₇, t₁₃, m₀)` and flip the tag as above. Whether the kernel's SPIR-V keeps GLSL.std.450 `Fma`
fused, which TASK-M2-02's qa review observes needs the `NoContraction` decoration, and whether rust-gpu emits that
decoration, is a check for the second-backend parity run, not settled here. So `M₀₁` under the edge-reaching simplex
link, at its onset crossings, is a branch input for the parity gate's no-fork test (R-34, REQ-VAL-017, REQ-VAL-079).
That test compares tags on identical emitted masses: a disagreement under `z` fuzz is a fork only where the two
backends' masses are bit-identical; where the masses differ, it is precision (REQ-VAL-079).

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

Both `α`-poles are **represented, not excised** — the old `α_min` cap is removed for full-sphere coverage. `α→0` puts body 2 at the inner-pair CoM (`‖λ̃‖→0`, azimuth `β` undefined *at the point*); `α→π/2` collapses the inner pair (`‖ρ̃‖→0` — a binary collision, `U→−∞`, and the rotation pin undefined). These are *coordinate/collision* degeneracies the pipeline already fences — the collision detector, the conditioning readout, and the saturation flags — not a range the chart carves out. In exact arithmetic finite `z` never reaches the poles (`(π/2)σ(z_α) ∈ (0, π/2)`); in floating point the range is closed, `α ∈ [0, π/2]` and `β ∈ [0, π]` (R-417). The lower poles, `α = 0` and `β = 0`, are reached where the link's output underflows, at f32 and f64 (σ where `exp(−z)` overflows, the tanh links where `tanh` rounds to −1, the softsign links where `x/(2 + |x|)` rounds to −1). The upper images stay inside the bounds: the f32 images of π/2 and π are generated rounded toward zero (dd_generation_root §3.9), and the f64 nearest values already lie below them, so no saturated control passes `π/2` or `π`. A config at a pole, reached at underflow or ingested there, is caught by those flags, and an angle at an exact pole has a defined value, 0, rather than a NaN: it is not obtained from `atan2(0, 0)`, which is the CPU's 0 but undefined in GLSL.std.450 `Atan2` and so in WGSL, but chosen branch-free by a select on the operands against 0, so that CPU and GPU agree (R-437; §3.3, §3.6 and the encode's inverses).

**Every `atan2` at a possible pole (R-437).** Where both operands of an `atan2` can be 0 together, the value there is
defined, not left to `atan2`: the `atan2` term is 0, the CPU's `atan2(+0, +0)`, and the same 0 for every sign of the
zeros, so that `−0` operands, whose `atan2(±0, −0) = ±π` on the CPU, give it too. The test is a comparison of each
operand with the constant 0 (true for `+0` and `−0` alike), and the value a select between 0 and the `atan2` already
computed, never a branch (the GPU determinism note's discipline). Its places: §3.3's rotation angle `φ` at `ρ = 0`
(the `α = π/2` collision pole), so `R(−φ)` is the identity there; the encode's `β = atan2(|λ̃_y|, λ̃_x)` (R-438)
at `λ̃ = 0` (the `α = 0` pole; `principia_dd_encode.md` §3.1, `principia_inverse_encode_contract.md` Part 3); the shape-sphere
inverse's `atan2(q, p)` at `p = q = 0` (`principia_chart_reference.md` §3.2), so `ψ = φ_f` there; and §3.6's
`rho_angle`, defined case by case. The encode's `α = atan2(‖λ̃‖, ‖ρ̃‖)` is not among them: after the scale gauge
`‖ρ̃‖² + ‖λ̃‖² = I = 1`, so its operands are never both 0.

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

**`φ` at the pole (R-437).** At `ρ = 0`, the `α = π/2` collision pole, `φ` is 0, whatever the signs of the zeros, so
the rotation is the identity there; it is chosen by a select on `ρ_x = 0` and `ρ_y = 0`, not obtained from
`atan2(0, 0)` (§3.2, "Every `atan2` at a possible pole").

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

The stored `ICDescriptor` is the ledger's field list (`principia_dd_generation_root.md` §3.6: 64 B at f32, explicit padding; its width follows `Real`, R-313).
`E₀` is **derived** from it (`K₀ + V₀`), not stored (R-86).

`E₀` here must agree with the kernel's `E_0` at t=0 — that agreement is a cross-check view, not an assumption.

**The six other fields** *(definition, R-72; REQ-PAY-088, written by TASK-M2-03)*. Each is read from the decoded
`(m, r, p)` alone, after the blocks and before integration, so it is the same for every chart (§2: the integrator's
input is `(m, r, p)` and nothing else). With §3.2's reduced masses and Jacobi vectors, written from the positions:

```
M₀₁ = m₀ + m₁        μ_ρ = m₀m₁ / M₀₁        μ_λ = m₂ · M₀₁
ρ = r₁ − r₀          λ = r₂ − (m₀r₀ + m₁r₁) / M₀₁          ρ × λ = ρ_x λ_y − ρ_y λ_x

q_mass       = min(m₀, m₁) / max(m₀, m₁)                      ∈ (0, 1]
rho_mag      = ‖ρ̃‖ = √μ_ρ · ‖ρ‖                               ∈ [0, 1]
lambda_mag   = ‖λ̃‖ = √μ_λ · ‖λ‖                               ∈ [0, 1]
rho_ratio    = ‖ρ̃‖ / ‖λ̃‖                                      ∈ [0, ∞]
rho_angle    = atan2(ρ × λ, ρ · λ)                             ∈ (−π, π]
r_min_pair_0 = min(‖r₁ − r₂‖, ‖r₂ − r₀‖, ‖r₀ − r₁‖)             ∈ [0, ∞)
```

- **`rho_mag` and `lambda_mag` are the mass-weighted magnitudes** `‖ρ̃‖` and `‖λ̃‖`, not the unweighted `‖ρ‖` and `‖λ‖`
  (R-62 names the fields; this settles which norms). So `rho_mag² + lambda_mag² = I = 1`, each lies in `[0, 1]` (the
  `|ρ| ∈ [0,1]` of render_gui_spec §10.1), and in the canonical frame `rho_mag = cos α` and `lambda_mag = sin α`.
- **`rho_ratio` is the mass-weighted ratio** `‖ρ̃‖/‖λ̃‖`, `cot α` in the canonical frame and 1 at the golden IC: the
  ratio that `rho_ratio`'s provisional log floor assumes (R-419), so the floor stands in its units (REQ-TOOL-163). It
  reads `+∞` only where `‖λ̃‖` is exactly 0. Where `α = 0` is reached at underflow (§3.2), `‖λ̃‖` is the rounding left
  in the reconstruction or 0. `rho_mag = 1` there, so the ratio is never `0/0`.
- **`rho_angle` is the signed angle from `ρ` to `λ`**, in radians on `(−π, π]`: a full-turn angle, for the cyclic view's
  period of 2π (render_gui_spec §10.1; REQ-TOOL-161). The mass weights are positive scalars, so `ρ̃` and `λ̃` give the
  same angle. In the canonical frame (`ρ̃` on `+x`, `λ̃_y ≥ 0` on the decode's output; after the encode's
  canonicalisation `λ̃_y ≥ −δ_λ`, R-82's deadband, which is why the encode's `β` reads `|λ̃_y|`, R-438) it is `β ∈ [0, π]`. Its value is defined case by case,
  not left to `atan2` (measured by TASK-M2-03), with `π̂` §3.2's image of π (rounded toward zero at f32, the nearest
  value at f64; R-417), so that the range is `(−π̂, π̂]` at each precision:
  - **Antiparallel** (`ρ × λ = 0`, `ρ · λ < 0`): `π̂`, whatever the sign of a zero `ρ × λ`. Collinear bodies whose `y`
    are exact zeros give `ρ × λ = −0`, and `atan2(−0, x < 0) = −π`, outside the range.
  - **Parallel, or the pole** (`ρ × λ = 0`, `ρ · λ ≥ 0`): 0. At the pole, `ρ × λ = ρ · λ = 0`, GLSL.std.450 `Atan2`
    is undefined, so the CPU's `atan2(0, 0) = 0` (§3.2) does not carry to the GPU: the value is the constant 0.
  - **Elsewhere:** `atan2(ρ × λ, ρ · λ)`, a value above `π̂` taken down to `π̂` (f32's nearest π lies above π), and a
    value at or below `−π̂`, which stands for the half turn `−π`, read as `π̂`, the half turn the range includes.

  Each case is chosen by a comparison against the constant 0 or `±π̂`, selecting a value already computed: no branch
  (the GPU determinism note's discipline).
- **`q_mass` is the inner pair's mass ratio**, lighter over heavier: 1 at equal masses, unchanged when bodies 0 and 1
  swap labels, and greater than 0 wherever `m₀` and `m₁` both are. Its ledger range is declared `[0, 1]` from this
  formula (REQ-PAY-097, R-426), a range that does not span zero, so its numeric view is on `ramp_viridis` (R-400).
- **`r_min_pair_0` is the closest pair's separation at `t = 0`**, a distance in `G = M = I = 1`. The `_0` is the
  initial-time subscript of `K_0`, `V_0` and `E_0`, not a pair id (R-22's pair ids name a side; this field names
  none).
- **The collision pole reaches 0** (measured by TASK-M2-03). In exact arithmetic `α < π/2` keeps `‖ρ̃‖ > 0`, so
  `rho_ratio` and `r_min_pair_0` stay above 0. In floating point, where `α` saturates at the image of π/2 (§3.2),
  `‖ρ‖ = cos α/√μ_ρ` can fall under the rounding of `r₀₁` in the reconstruction. At f64, `cos α ≈ 6×10⁻¹⁷` there, so
  the emitted `r₀` and `r₁` can be equal: then `rho_mag`, `rho_ratio` and `r_min_pair_0` read 0, `V_0` reads `−∞`
  and `virial_ratio` 0. This is §3.2's binary collision, `U → −∞`, which the pipeline fences; the two log fields'
  ranges are `[0, ∞]` and `[0, ∞)`, and their views place 0 at the bottom of the ramp.
- **The ledger declares the log fields' lower bound only.** Its `[0, ∞)`, in the payload ledger and the headers of the
  views generated from it, means that no upper bound is declared, not that `+∞` is excluded: `rho_ratio` reaches `+∞`
  at the `α = 0` pole, where `‖λ̃‖` is 0. Its view draws that `+∞` in the invalid colour, on the CPU and the GPU
  alike, not at the top of the ramp (R-436, below).
- **The poles' infinite values are stored as computed (R-436).** `rho_ratio`'s `+∞` at `α = 0`, and `V_0`'s `−∞` at
  the collision pole (measured at f64; at f32 too, wherever the reconstruction makes a pair coincide), with
  `E₀ = K₀ + V₀ = −∞` there, are the true values: storage holds them, as it never holds
  NaN (R-79). Every consumer, on the CPU and the GPU alike, colours a non-finite input as invalid: each view that reads
  a stored `±∞` (`rho_ratio`'s, `V_0`'s, `E_0`'s), each agreement preset that differences against `E₀`
  (`principia_colour_composition.md` §6) and the energy-agreement view (`principia_render_contract.md` § "Cross-check
  views (the seams)"). The test compares the stored value, on its bits, as R-114's NaN guard does, before any
  arithmetic, `K_0 + V_0` included; it never does arithmetic on `±∞` and never writes an infinite literal, since WGSL
  lets an implementation assume there are no infinities (`principia_gpu_determinism_note.md` § "The discipline", rule
  4). A declared sentinel or unset value keeps the style its own ruling gives it (`d_min`'s unset `+∞`, R-280, R-399).
- **Each producer selects its pole value (R-436).** The descriptor follows `Real` (R-313), so on the GPU path the
  decode stage computes it at f32, and the fragment decode (R-116) forms its own `V` and `E`; the poles are reached at
  f32 as at f64 (§3.2, R-417). WGSL lets an implementation assume there are no infinities, so a GPU-computed `1/0`, or
  arithmetic on `∞`, is not pinned to `±∞`, and a finite indeterminate result would pass the consumers' bit test. So
  each pole value is chosen by comparing its divisor with 0 and a select, as §3.2's `atan2` poles and `rho_angle` are,
  never by the arithmetic:
  - `rho_ratio = ‖ρ̃‖/‖λ̃‖`: the `+∞` bits where `‖λ̃‖ = 0`;
  - `V_0`: the `−∞` bits wherever a pair separation `‖rᵢ − rⱼ‖` is 0, for the whole sum, not a sum with an infinite
    term;
  - `virial_ratio = 2K₀/|V₀|`: 0 where `V_0` is non-finite, tested on its bits.

  The bits are written by a bitcast of the integer pattern (`0x7F80_0000` and `0xFF80_0000` at f32,
  `0x7FF0_0000_0000_0000` and `0xFFF0_0000_0000_0000` at f64), as R-271's packer writes `d_min`'s `+inf`, never by an
  infinite literal or by `1/0`; the quotient the select discards is never read. The CPU path runs the same source and
  stores the same bits. The fragment-side agreement preset tests the fragment decode's `V` on its bits before it
  forms `E` (`principia_colour_composition.md` §6).
- **At rest only `K_0` and `virial_ratio` are 0** (R-421): at the golden IC (§5) the six fields above read 1, `1/√2`,
  `1/√2`, 1, `π/2` and `√3`.

### 3.7 Energy normalisation (optional; keep-or-drop is decision B9)

A post-momentum rescale that enforces a target energy $E^*$. Compute $K_0$, check feasibility $E^* \ge U$, then

$$\mathbf p_i \leftarrow \eta_E\,\mathbf p_i, \qquad \eta_E = \sqrt{\frac{E^* - U}{K_0}}.$$

"Off" is an explicit `None`; $E^* = 0$ is a real target, the energy set to zero (R-25). The rescale follows the
momentum block and precedes the derived invariants, so the `ICDescriptor` describes the rescaled IC ($K_0 = E^* - U$
to rounding).

**The outcome at $K_0 = 0$ and on $E^* < U$ (R-72; REQ-DEC-041): a label, never a refusal.** $U$ is the decoded
configuration's potential, so whether $E^*$ is reachable is known only after the decode, pixel by pixel; the outcome
is a label on that decode. The one refusal is the view-level one below, a forbidding chart with any `Some(E*)`. Here
$U = V_0$ and $K_0$ are §3.6's, of the decoded $(m, r, p)$, and $E^*$ is the view's finite value. In order:

- **The collision pole, $U = -\infty$** (a pair separation is 0, §3.6, R-436): $E^* - U$ is infinite and no finite
  $\eta_E$ exists. The decode is tagged `DEGENERATE(ENERGY_UNSCALABLE)`.
- **$E^* < U$:** $\eta_E$ is not real, since the kinetic energy would be negative. The decode is tagged
  `DEGENERATE(ENERGY_INFEASIBLE)`. As $U < 0$, only a bound target $E^* < 0$ can be infeasible; every $E^* \ge 0$,
  $E^* = 0$ included, is feasible.
- **$K_0 = 0$, the rest start** (every $\mathbf p_i = 0$): there is no direction to scale. At $E^* = U$ the IC is
  already at the target: $\eta_E$ is not applied, the momenta stay 0, and the decode is not tagged. At $E^* > U$ the
  decode is tagged `DEGENERATE(ENERGY_UNSCALABLE)`; the golden IC, `z = 0` at rest, is so tagged under any $E^* > U$.
- **Otherwise** ($E^* \ge U$, some $\mathbf p_i \ne 0$, every separation $> 0$) $\eta_E$ is applied; $E^* = U$ gives
  $\eta_E = 0$, a rest start.

A tagged decode keeps its momenta as decoded: it is kept, never dropped and never NaN, and, as every `DEGENERATE`, it
is terminal (§2). A decode the mass block has already tagged keeps that tag. `ENERGY_INFEASIBLE` and
`ENERGY_UNSCALABLE` belong to `DEGENERATE`'s enumerated cause set (§2).

**$K_0 = 0$ means every $\mathbf p_i$ is 0**, not that the rounded $K_0$ is 0: a non-zero momentum whose squares
underflow is scaled. The rescale is formed on $\hat{\mathbf p}_i = \mathbf p_i / s$, with $s$ the largest
$|p_{i,x}|, |p_{i,y}|$:
$\eta_E\,\mathbf p_i = \sqrt{(E^* - U)/\hat K}\;\hat{\mathbf p}_i$, with $\hat K = K_0/s^2 \ge \tfrac12$ the
kinetic energy of the $\hat{\mathbf p}_i$ (one component is 1 and every $m_i \le 1$). The two are equal in exact
arithmetic, and the second stays finite where $(E^* - U)/K_0$ overflows or $K_0$ rounds to 0.

**The branch inputs** (§2's comparison-only rule; R-436): $E^* < V_0$, $E^* = V_0$, $s = 0$ and
`r_min_pair_0` $= 0$, each a comparison of the emitted values with no arithmetic at the decision, as $M_{01}$'s
test reads the emitted masses (§3.1). The collision pole is tested on the separation, never by arithmetic or a
comparison on $\infty$.

**Where it is allowed.** Only on charts that decode arbitrary momenta without enforcing an energy: the
free-Jacobi-momentum chart and the latent chart. **It must be disabled** on the invariant-momentum charts
$(L_z, E)$ and $(L_z, K)$, where energy is a chart coordinate or is fixed by the momentum construction.
There it would fight the invariant construction near the feasibility boundary, or, for $(L_z, E)$, collapse
the energy axis entirely. Each chart declares a boolean `forbids_energy_normalisation`, and chart-aware
validation (`principia_inverse_encode_contract.md`) refuses any view that combines such a chart with an
$E^*$ override — every `Some(E*)`, including `Some(0)`, since $E^* = 0$ is a real target and "off" is `None` (R-25).

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

Golden anchor: **`z = 0` decodes to the canonical golden IC** — equal masses `(⅓,⅓,⅓)`, `α = π/4` (‖ρ̃‖ = ‖λ̃‖), `β = π/2`, rest (`q = 0`), `I = 1`. Assert its every derived value in f64 and f32. Each is asserted against its hand-computed value, the exact value rounded to that precision (R-421: "hand-computed at f32" means the exact value rounded to f32): bit-exact for the fields that are 0 at rest (`K_0` and `virial_ratio`), and within per-precision tolerances for the others, proposed with evidence by TASK-M2-03 and confirmed at the M2 gate (REQ-PAY-095).

1. **Identities (every decode, fuzzed z):** `Σmᵢ = 1`; total CoM `= 0`; `Σpᵢ = 0`; `I = 1` — all to precision-appropriate tolerance.
2. **Factorisation independence:** perturbing `z_μ` leaves `(α, β)` and `(p_ρ, p_λ)` bit-identical (and each block likewise) — the blocks genuinely do not couple except through the join.
3. **Range respect:** `α ∈ [0, π/2]`, `β ∈ [0, π]`, `|qₖ| ≤ q_max`, for all finite z including ±∞-ish saturation, the range closed (R-417): `α = 0` and `β = 0` are reached where the link underflows, at f32 and f64; `qₖ` reaches `±q_max` at saturation; `α` and `β` never pass `π/2` and `π`, compared at the exact bounds.
4. **Degenerate tagging:** `DEGENERATE(M01_TINY)` fires iff `M₀₁ < ε` and nothing else in the mass block does; degenerate outputs are tagged, never NaN, never dropped.
5. **Canonical no-op:** for canonical-frame decodes, saturated controls included, applying §3.3 changes nothing (rotation angle ≈ 0, mirror not taken) away from the `λ̃_y = 0` seam; at the seam, the deadband makes the choice deterministic (same result twice). An exception covers only a measured case (R-417): the task that measures the no-op lists here each case where it fails, with its `z`, precision and measurement, reviewed by physics; there is no blanket exception.
6. **Reconstruction inverse:** from the emitted `(m, rᵢ)`, recompute `ρ̃, λ̃` via §3.2's definitions and recover `(α, β, R̃=1)` to tolerance — the decode and its own definitional inverse agree.
7. **Round-trip (T2, physical units):** `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys` over interior z; clamp-adjacent samples assert the flag, not the residual.
8. **Invariant construction:** for a grid of feasible `(L_z, K*)` at fixed config: `L_z(v) = L_z` and `K(v) = K*` to tolerance; infeasible pairs refused/tagged exactly on `K* < L_z²/2`; the largest-norm qualifying seed is taken (ties by seed order), and the degenerate-seed path fires only where no seed has `‖w⁽²⁾‖²_m > ε_w` (R-82).
9. **α-orientation:** `α → 0 ⇒ ‖r₁ − r₀‖` is *maximal* (and `‖λ‖` minimal); monotone crossover at `π/4`. Pins the gotcha.
10. **Scale-gauge idempotence:** §3.5 applied to canonical output is identity; applied to a deliberately scaled input yields `I = 1` and transforms `E, L_z` per the encode contract's table.
11. **Linearised decode:** at quad centre, `x₀ + J_D·0 = x₀ =` full decode exactly; at half-width, error vs full decode shrinks `∝ h²` across depths; identical behaviour for an affine chart and a curve chart (chart-agnosticity).
12. **Cross-backend:** the CPU-f64 and GPU-f32 instantiations (and browser-GPU-via-WGSL) agree on the golden IC and fuzzed interiors to f32 eps-scaled tolerance on continuous values; the degenerate/feasibility **branch tags are bit-identical** across all backends (the comparison-only rule — decoder branch inputs, like feasibility `K* ≥ L_z²/2`, must be comparisons against constants / single-rounded, never a runtime transcendental at the decision; `principia_gpu_determinism_note.md`).

---

## 6. Deferred / flagged

- **Body indexing — settled, 0-based (R-22).** The decode uses bodies **0, 1, 2** (softmax reference = body 0), and `ICDescriptor` names `m0, m1, m2`; pair id `k` is the side opposite body `k` (payload §2). The flag was raised because this is exactly the off-by-one that survives until a collision-pair label is wrong on screen. What a person reads is 1-based (R-449): the screen's labels come from the one display function (payload §2, "What a person reads"), so the conversion happens in one place.
- **Energy normalisation `η_E`**: specified in §3.7, its outcomes at `K₀ = 0` and on `E* < U` included (REQ-DEC-041).
  It is kept, with an explicit `Option` off switch (R-25), which settles audit decision B9.
- **Quantised checkpoint storage** — moot under lockstep (no stored trajectory; temporal note, ratified).
- **KS-regularised state representation (v2)** — changes the decoder's output type; explicitly out of scope until then.

---

*One rung, three homes (f32 kernel, f64 CPU, inverted in encode), one set of numbers. Small α is a wide pair. Body zero is the reference. z = 0 is the golden IC. Everything invalid is tagged, and every backend takes the same branches (comparison-only rule), while continuous values diverge honestly.*
