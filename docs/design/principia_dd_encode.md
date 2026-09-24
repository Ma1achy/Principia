# Drill-down — Encode (the Question rung's upward door)

*Fourth drill-down. The inverse-encode contract is already maths-dense; this drill-down's value is the assembly: every forward map (decoder dd §3) placed beside its inverse **with the closure verified**, the canonicalisation as an exact ordered procedure, the gauge group action written explicitly (so T1 is a concrete sweep, not a slogan), and the projection metric for curve axes pinned. Consolidation where the contract sufficed; new maths only where the two *precisions* of the one shared kernel could otherwise diverge (encode runs the decode inverted, in the same single source — `principia_spike_brief.md`).*

---

## 1. What it is

The **only upward edge on the ladder**: `physical state → z`, the quotient map onto the decode's canonical section. Consumers: the lock's re-centre, direct physical IC entry, Burrau `(m,n)` entry, invariant-pair lookup, **literature IC import** (how the validation programme's Anosova/Lehto/Burrau starts get loaded), the T-tests, and the `ROUNDTRIP` debug kernel.

---

## 2. Consolidated contract

From the **inverse-encode contract** (all of it binds; headline clauses): `E = (block/chart inverses) ∘ C`; **T1** gauge-invariance (deterministic tie-breaks), **T2** right-inverse on the interior, **T3** left-inverse modulo gauge; rigid operations act on the **full phase-space state**; the canonicalisation order is **1a CoM → 0 scale → 1b boost → 2 rotate → 3 mirror** (R-23); the invariant fibre point is the chart's own forward construction (**encode reuses decode** — pending change 3 supersedes "smallest latent norm"); tolerances asserted in **physical units**; everything discarded is **reported** (`lookup_rescaled`, `lookup_mirrored`, `lookup_clamped`).

From the **chart contract / generation root**: block inverses come from the **link registry** (never inlined); ε clamps are registry data (`ε_μ = ε_z = ε_q = 10⁻⁶`, `μ_max = 5`, `q_max = 2`).

From the **decoder dd**: decoded states are already canonical (§3.3 no-op property), so T2 exercises the block inverses and numerics only; body 0 is the softmax reference.

From **navigation (Part 4)**: the lock on nonlinear charts prefers CPU forward replication; encode is the path for *entered* states, not clicked pixels.

---

## 3. The maths

### 3.1 Forward ↔ inverse pairing table — closures verified

| Block | Forward (decoder dd §3) | Inverse (encode) | Closure check |
|---|---|---|---|
| **Mass** | `mₖ = e^{μₖ}/(1+e^{μ₁}+e^{μ₂})`, `μ₀ = 0`; `μₖ = μ_max·tanh(z)` | `μₖ = log(mₖ/m₀)`; `z = artanh(μₖ/μ_max)`, clamp `(1−ε_μ)μ_max` | `m₁/m₀ = e^{μ₁}` ⟹ log-ratio recovers `μ₁` exactly ✓ |
| **Config** | `ρ̃ = cos α (1,0)`, `λ̃ = sin α (cos β, sin β)`; `α = (π/2)σ(z_α)`, `β = πσ(z_β)` | `α = atan2(‖λ̃‖, ‖ρ̃‖)`, `β = atan2(λ̃_y, λ̃_x)`; `z = logit(s)` with the range maps inverted | `atan2(sin α, cos α) = α` on `(0, π/2)` ✓; `β ∈ [0,π]` guaranteed by the mirror ✓ |
| **Momentum** | `p₀ = −p_ρ − (m₀/M₀₁)p_λ`, `p₁ = p_ρ − (m₁/M₀₁)p_λ`, `p₂ = p_λ`; `qₖ = q_max(2σ(z)−1)` | `p_λ = p₂`; `p_ρ = p₁ + (m₁/M₀₁)p₂`; `s = ½(q/q_max + 1)`, `z = logit(s)`, clamp | substitute forward into inverse: `p₁ + (m₁/M₀₁)p₂ = p_ρ` identically ✓; `Σpᵢ = 0` consumed, not assumed |
| **Invariant fibre** *(momentum stage of (L_z,E)/(L_z,K) charts)* | `v = L_z·Jr + a·w`, `a = √(2(K*−L_z²/2))` (I = 1) | chart coords = evaluate `L_z(x)`, `E(x)` directly; latent = **run the same construction**, then free-momentum-invert the constructed `p = m·v` | closure is *by construction* — one code path, so lookup and clicked pixel land on the same z |
| **Curve (Burrau)** | `γ: ν ↦ (α, β, μ₁, μ₂)`; `θ(ν)` monotone | on-curve: `ν(θ) = sec θ − tan θ` from the canonical leg ratio; masses cross-check `μ₁ = log(b/c)`, `μ₂ = log(a/c)` | `sec θ − tan θ` inverts `θ(ν)` in closed form on `θ ∈ (0, π/2)` ✓ |

### 3.2 Canonicalisation `C` — the exact ordered procedure

Every operation acts on the **full state** (all `rᵢ` and all `pᵢ`). Order is load-bearing.

```
1a COM        r ← r − R_com                            (translations gauged; I is CoM-frame-defined)
0  SCALE      I_in = Σ mᵢ‖rᵢ‖²  (CoM frame)          → r ← r/√I_in ;  p ← I_in^{1/4}·p
              record λ = I_in^{−1/2} ; notice lookup_rescaled
              (E_canon = √I_in·E_in ;  L_canon = L_in/I_in^{1/4} ;  t_canon = t_in/I_in^{3/4})
1b BOOST      p ← p − mᵢ·P_tot/M                       (boosts gauged)
2  ROTATE     φ = atan2(ρ_y, ρ_x) ;  apply R(−φ) to every rᵢ AND every pᵢ ;  ρ̃ lands on +x
              ρ = 0 → reject (coincident inner pair)
3  MIRROR     if λ_y < −δ_λ : apply diag(1,−1) to every rᵢ AND every pᵢ ; notice lookup_mirrored
              |λ_y| ≤ δ_λ = 10⁻¹² : deterministic no-mirror (T1 depends on this tie-break)
4  INVERT     per §3.1, into the active chart where possible, else latent z (always possible)
5  FIBRE      chart's own construction first ; smallest-latent-norm only where none exists
6  VALIDATE   hypercube → chart feasibility → decode sanity ; project / clamp / reject ; surface flags
```

The order is normative (R-23): **1a** subtract CoM → **0** rescale → **1b** subtract boost → **2** rotate → **3** mirror. These steps commute with nothing; *CoM subtraction precedes the `I` computation* because `I` is CoM-frame-defined. The inverse-encode contract (Part 6) uses the same numbering.

### 3.3 The gauge group action, explicit (this is what T1 sweeps)

For random `g = (a, u, θ, s, Π)` — translation `a ∈ ℝ²`, boost `u ∈ ℝ²`, rotation `θ`, scale `s > 0`, mirror `Π ∈ {𝟙, diag(1,−1)}`:

```
rᵢ' = s · R(θ) · Π · rᵢ + a
pᵢ' = s^{−1/2} · R(θ) · Π · pᵢ + mᵢ · u
```

**T1 asserts `E(g·x) = E(x)` for all such g** — including g's that land `λ_y` inside the mirror deadband (the tie-break sweep). The scaling law on momenta (`s^{−1/2}`) is the similarity symmetry; using `s^{+1/2}` or forgetting `Π` on `p` are the two errors this explicit form exists to prevent.

### 3.4 Curve projection (off-curve input) — metric pinned

Generic input is not on the Burrau embed. Projection:

```
ν* = argmin_ν  d²(ν) = ‖ n(α,β) − n(α_γ(ν), β_γ(ν)) ‖²  +  ‖ m − m_γ(ν) ‖²
```

— shape-sphere **chordal** distance ⊕ mass-simplex **Euclidean** distance, unit weights (both O(1)-normalised). 1-D minimisation over a smooth curve (Brent/golden-section grade); report `d(ν*)`; beyond tolerance → the curve chart cannot represent the point → fall back to latent z. **The metric and weights are a pin of this drill-down — confirm/veto.** On-curve inputs bypass the argmin via the closed form (§3.1) — and the two paths agreeing *is a test* (5.7).

### 3.5 Conditioning (numbers, and the tolerance rule)

`d logit/ds = 1/(s(1−s)) ≤ 1/ε = 10⁶` at the clamps; `artanh` likewise at saturation. Therefore every assertion is in **physical space**: `‖D(z) − D(E(D(z)))‖_phys ≤ ε_phys`, never a z-residual near saturation. Clamp events are *expected and flagged*, not failures.

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **8** physical → manifold | quotient onto the section; full-state rigid ops; everything discarded reported | T1/T3 suites (§5); the three `lookup_*` notices fire iff their operation did |
| **2/13** link registry | block inverses are registry inverses, never re-derived inline | swap a link → encode's inverse swaps with it; analytic vs numeric Jacobian agreement inherited from the registry tests |
| **fibre / (L_z,E) chart** | encode reuses decode's construction — one code path | enter `(L_z, E)` numerically vs click the corresponding pixel → identical z to precision (the pending-change-3 test) |
| **Observation** | `ROUNDTRIP` kernel = `D → E → D`, physical residual | residual view dark except clamps / feasibility edges / mirror tie — each *tagged expected* |
| **validation import** | literature ICs enter through this door | Anosova region-D and Burrau rest starts encode → decode → invariants match the papers' stated values after the recorded rescale |

---

## 5. Unit tests

Golden pairs: **z = 0** (equal-mass golden IC) and **Burrau** (physical `(3,4,5)` construction ↔ its known ν and mass point).

1. **T2, per chart:** interior z samples → `D → E`, assert physical residual ≤ `ε_phys`; clamp-adjacent samples assert `lookup_clamped`, not a tight residual.
2. **T1 gauge sweep:** random `g` per §3.3 (including deadband-straddling rotations) → `E(g·x) = E(x)` bit-comparably after float tolerance; the mirror tie produces the identical output twice.
3. **T3 with the half-mirror catch:** random-gauge x → `E → D` → equals `C(x)`: shapes match, masses match, `E` matches after the recorded rescale, `|L_z|` matches **with sign consistent with `lookup_mirrored`** — a config-only mirror fails exactly this check.
4. **Scale step:** feed the Burrau state at 2× scale → `lookup_rescaled` with `λ = 1/2`... precisely `I_in = 4·I_canon` ⇒ recorded `λ = 1/2`, `E_canon = 2·E_in`, timing conversion factor `I_in^{3/4}` — assert the whole table row.
5. **Burrau closure:** physical `(3,4,5)` → encode → masses `(5,4,3)/12`, right angle recovered, `ν` matches `ν(θ) = sec θ − tan θ`; decode back → same triangle to `ε_phys`.
6. **Infeasible invariant entry:** `(L_z, E)` with `K* < L_z²/2` → projected/clamped/rejected per the validation ladder, flagged, never silent, never NaN.
7. **Curve projection consistency:** exact on-curve input — argmin and closed form agree on `ν` to tolerance; off-curve input — `d(ν*)` reported, latent fallback taken beyond tolerance, and `E ∘` fallback still satisfies T2.
8. **Conditioning formulation:** a near-clamp input where the z-residual is ~10⁵× the physical residual — asserting that the *physical* tolerance passes while a naive z-tolerance would fail (the test that keeps future contributors honest about §3.5).
9. **Coincident-pair rejection:** `ρ → 0` inputs reject at step 2 with the correct reason, both precisions, same branch.

---

## 6. Deferred / flagged

- **Projection metric pin (§3.4)** — chordal-⊕-Euclidean, unit weights: confirm or veto; whatever wins goes in the shared source with the curve embeds.
- ~~**Operational order of steps 0/1 (§3.2)**~~ — **settled by R-23:** 1a → 0 → 1b → 2 → 3 is normative, in the contract's numbering as here; pinned in the **shared source** so both instantiations (CPU-f64, GPU-f32) inherit the identical sequence.
- **Batch import ergonomics** (CSV of literature ICs → encode → tagged report per row) — a validation-phase tool, not contract; noted so the import seam test has a harness to live in.

---

*Every forward has its inverse beside it, and the closure is checked on paper before it is checked in code. The group action is written out so T1 is a loop, not a hope. One construction serves the pixel and the lookup. And the residual that matters is measured in the world, not in z.*
