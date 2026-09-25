# Drill-down — the Integrator (Physics rung)

---

> ### ⚠ DEFAULT CHANGED — Heggie, not Aarseth–Zare
>
> This document treats **AZ** as the regularisation. **The default is now Heggie global
> regularisation** (`principia_integrator_contract.md` Part 2b, `principia_spec_pending_changes.md`
> change 8).
>
> **Measured:** 31 of 32 cases, `err>10` **3915 → 74** at prin-rs `8600d45` (the original run at `70cfbc4` gave 3916 → 73; R-165), AZ's worst decile fixed on 100% of pixels. AZ
> retains exactly one win — **`far`**, where sustained hierarchy means it **never re-registers**.
>
> **The mechanism:** doubling the sync-boundary **re-registration count** at fixed step size moves the
> drift field **0.444 decades**, against 2.5e-6 for the LC branch choice and 7.5e-5 for the
> reference-body rule. *It is not which chart is chosen; it is how often the state is passed through
> one.*
>
> **What still holds here:** anything about the *horizon*, `t_max = ln(1/eps)/lambda`, and precision —
> those are properties of the arithmetic, not of the regularisation. Anything AZ-specific
> (reference-body selection, the LC branch cut, the two-pair structure) is now one occupant's detail
> rather than the design.
>
> **Evidence:** `prin-rs` `FINDINGS.md` §5, `results/integrator_gallery_1024/`.

*Third drill-down. Occupant coefficients, the substep law, detector state machines, the live shape readout, and the co-computation algorithms — the exact maths the one shared kernel must produce at either precision (under the substrate there is one Rust source, not two implementations; the maths still has to be pinned, and its branch decisions held bit-identical on identical inputs, per step, by the comparison-only rule — §3.3). The wrapper/occupant architecture, capability profiles, cadence, units, and determinism rules are the integrator contract's; consolidated in §2, not re-argued.*

---

## 1. What it is

The **Physics rung**: `(m, r, p) → integrate to horizon → classify → pack`. One swappable `STEP` occupant inside a fixed wrapper (loop, substep, project, monitor, detect, state readout). Three homes: the f32 survey kernel, the f64 inspector/hover witness, and — deferred — the recorded-schedule reverse replay.

---

## 2. Consolidated contract

From the **integrator contract**: occupant = `ADVANCE(state, t_now, t_target, params) → state'`, nothing else (KDK/Yoshida implement it as the wrapper loop around their `STEP`; R-19); wrapper owns everything shared; **cadence pinned per-`STEP`** (projection, monitoring, detection after every step; shape readout on the macro schedule; nothing stored — lockstep); capability profile `{order, force_evals, symplectic, reversible}` read by all consumers; occupant on the sim key; Euler is a debug tool; units `G = M = I = 1`, `T ∈ [50, 200]` physical (default 50), `dt_macro = max(1e-3, T/65535)` (R-132), schedule length `⌈T/dt_macro⌉` deterministic; **values may diverge by precision, wrapper branches may not on identical inputs** (R-84).

From **core design / precision ring**: one physics definition, compiled twice — *structurally* (one Rust source), so logic equality is definitional; divergence (continuous) exposed, never reconciled; branch decisions held bit-identical on identical inputs, per step (comparison-only rule, §3.3; labels on chaotic trajectories may differ across precisions — `principia_parity_contract.md` Tier L, R-84); match-integrator mode for honest inspector comparison.

From the **scheduler (firewall)**: every IC integrates from its own decoded state — **no warm starts, ever**; payload pure of scheduling.

From the **render contract**: the Benettin co-computation rides the forward pass gated by a baked tier bit, and each ensemble copy is the same kernel dispatched again with `copy_index` as a uniform, not a baked variant (R-102); sentinels, never NaN, in storage; the kernel-set `state` enum and sticky `saturated` bit are the point-of-computation invalid-signals, while the drift suspect gates are read-time predicates over the stored latches (payload §5).

From **stability-metrics.md**, reconciled to the uniform-sample / ensemble-as-SSAA model (sampling/SSAA note, ratified): **every sample is a full `SimState` carrying its own Benettin shadow**, and a nominal grid sample has **E ensemble copies** (each also a full sample with its own shadow). So per nominal sample: `(E+1)` samples × (1 trajectory + 1 Benettin shadow) ≈ **`2(E+1)` trajectories**. **A Benettin shadow can never double as an SSAA sample** — renormalisation makes its endpoint an algorithmic artefact, not a physical trajectory — so the shadows are never in the colour/spread pool; the `E+1` *samples* are. (The older "N free shadows + 1 Benettin = N+2" framing is superseded: the ensemble copies are full samples, not shadows-of-the-base, and FTLE is per-sample not base-only.)

From the **deep-zoom contract**: substep-saturation-dominated quads are the integration floor — now a *confidence gradient* (saturation flag, non-terminal), not a terminal wall.

---

## 3. The maths

### 3.1 Force (G = 1)

```
aᵢ = Σ_{j≠i} mⱼ (rⱼ − rᵢ) / ‖rⱼ − rᵢ‖³        (3 pairs; ‖·‖³ shared with r_min)
```

### 3.2 Occupants

**Euler** (explicit — debug occupant; not symplectic, drifts secularly *on purpose*):
```
r' = r + dt·v ;   v' = v + dt·a(r)
```

**KDK leapfrog** (order 2, 1 force eval, symplectic):
```
v½ = v + (dt/2)·a(r) ;   r' = r + dt·v½ ;   v' = v½ + (dt/2)·a(r')
```

**Yoshida-4** (order 4, 3 force evals, symplectic) — composition of three KDK steps with weights
```
w₁ = 1 / (2 − 2^{1/3}) = 1.3512071919596578
w₀ = −2^{1/3} · w₁     = −1.7024143839193157        (w₀ + 2w₁ = 1)
drift coefficients d = [w₁, w₀, w₁]
kick  coefficients c = [w₁/2, (w₀+w₁)/2, (w₀+w₁)/2, w₁/2]
```
i.e. `KDK(w₁·dt) ∘ KDK(w₀·dt) ∘ KDK(w₁·dt)`. The negative middle step is real and is why naive time-reversal fails (integrator contract Part 6).

**Yoshida-6** (order 6, 7 force evals, symplectic) — Yoshida (1990), **solution A**:
```
w₁ = −1.17767998417887
w₂ =  0.235573213359357
w₃ =  0.784513610477560
w₀ =  1 − 2(w₁ + w₂ + w₃) = 1.31518632068391
drift sequence d = [w₃, w₂, w₁, w₀, w₁, w₂, w₃]
kick sequence  cᵢ = (d_{i−1} + dᵢ)/2  with  d₀ = d₈ = 0
```
*(Transcription check: verify the three w's against Yoshida 1990 Table 1 solution A — the paper is in the lit-review set — before they enter the shared physics source.)*

**RK4** (order 4, 4 force evals, **not** symplectic — Validation cross-check occupant; the inspector's *adaptive* high-precision reference is the separate RK45, integrator contract §6): the standard tableau on the full state `y = (r, p)`, `ẏ = (p/m, F(r))`:
```
k₁ = f(y);  k₂ = f(y + dt/2·k₁);  k₃ = f(y + dt/2·k₂);  k₄ = f(y + dt·k₃)
y' = y + (dt/6)(k₁ + 2k₂ + 2k₃ + k₄)
```

### 3.3 Substep law — and the determinism pin, made concrete

```
N_sub(r_min) = clamp( ⌈ (r_sub / r_min)^{γ_sub} ⌉ , 1 , N_max )    [r_sub = 0.05, γ_sub = 1.5, N_max = 64]
```

<!-- retired-terms -->
This formula *specifies* `N_sub`; it does **not** compute it at runtime. `N_sub` is a **branch decision**, and the determinism rule for branch decisions is stronger and different from what an earlier draft assumed (which had `N_sub` computed as the f32 evaluation with `Math.fround`-per-stage on the CPU to match the GPU). **The spike (`principia_spike_brief.md` findings, 2026-07) measured that rule forking** — 5/272 boundary states — and a **controlled test overturned the cause**: it is **not** fast-math and **not** FMA contraction, but *inherent cross-implementation transcendental latitude*. A GPU's `pow` and libm's `powf` legitimately disagree by 1–3 ulp (e.g. `pow(4.000000477, 1.5)` → exactly `8.0` on Metal vs `8.000001907` in libm), **in both fast and safe math modes** — so *any* runtime transcendental feeding a `ceil` at an integer boundary can fork, and no amount of matched rounding fixes it because the two libraries are each entitled to their own result.
<!-- /retired-terms -->

**The rule (this is the determinism pin, replacing the f32-evaluation framing):** a branch decision may depend only on **comparisons against compile-time constants** and on **single-rounded arithmetic** — never on a runtime `pow`/`sqrt`/`div`. Concretely:

```
N_sub  =  bucket( d² )  via a generated comparison tree over 64 constants
          THR[n] = round_f32( ( r_sub / n^{2/3} )² )   for n = 1 … N_max     (frozen at build time)
          — smaller d² ⇒ smaller r_min ⇒ larger N_sub;  no runtime sqrt / div / pow / loop / dynamic index
```

The transcendental (`pow`) still exists — it moves from *runtime, every substep, every backend* (where it forks) to *once, at table-generation time, into a constant* (where it cannot). The generator may use f64 libm freely; only the frozen f32 output ships, and every backend then compares against the identical bit-pattern.

**The one value feeding the comparison must itself be built identically.** `d²` is a scrap of arithmetic (`dx² + dy²`), and if one backend contracts it to an `fma` and another does not it can differ by a ulp — re-forking at a bucket edge. So `d²` is **position-quantised**: cast positions to f32, then lone-subtract, lone-multiply, and an **explicitly written** `fma(dy, dy, dx*dx)` (explicit, so every backend is forced onto the identical fused op rather than choosing). Then the sole float feeding the sole branch is bit-identical everywhere, and the branch cannot fork.

Two further branch-path rules the spike earned by real failure:
- **Clamp in f32 *before* the float→int cast.** Out-of-range `OpConvertFToU` is UB on the GPU (Rust's `as` saturates instead) — a fork source independent of rounding. Clamp `d²`/the bucket index into range in f32 first.
- **Horizon is an integer step counter**, never accumulated float time (`t += dt` drifts and can fork the `t ≥ T` test); **collision is `d² < r_coll²`** against a constant, not `r_min < r_coll` via a runtime `sqrt`.

Verified: with the table rule, the branch decisions (`N_sub`, collision, horizon) are **bit-identical across CPU-f64, CPU-f32, native-GPU-f32, browser-GPU-via-WGSL, and CPU-double-double** on identical inputs, per step — 0 forks. That is the guarantee (R-84): a decision is identical given its step's inputs. Over a chaotic trajectory the inputs diverge by precision, so `state`, `total_substeps` and terminal labels may differ across precisions (parity contract Tier L/B). Continuous values (positions, momenta, energy, the trajectory) diverge freely and honestly; only the branch words are pinned. See `principia_gpu_determinism_note.md` for the general law this instances.

### 3.4 COM projection (per `STEP`; the policy is integrator contract Part 1)

```
R_com = Σ mᵢrᵢ / M ;   P_com = Σ pᵢ
rᵢ ← rᵢ − R_com ;      pᵢ ← pᵢ − (mᵢ/M)·P_com
```

Linear, exact-symmetry housekeeping; keeps f32 dynamic range on internal structure (the time-domain twin of quad-local coordinates). Energy and L_z are **never** projected — monitored only.

**Per-trajectory, not per-pixel.** The projection acts on *one* physical system's `{rᵢ, pᵢ}`, and under the uniform-sample model (sampling/SSAA note) a pixel carries several independent trajectories — the base, its Benettin FTLE shadow, and (per nominal sample) E ensemble copies each with their own shadow. **Each is projected independently, every step**, because each is an independent integration whose own CoM drifts under its own roundoff. In particular a **shadow trajectory must be CoM-projected on its own** — if it inherited the base's projection instead of computing its own, its uncorrected drift would corrupt the Benettin separation and thus the FTLE. The wrapper runs the full per-`STEP` sequence (project, monitor, detect) on every marching trajectory it owns; "per `STEP`" means per step *of each trajectory*.

### 3.5 Invariant monitoring (per `STEP`, post-projection)

```
E  = Σ ‖pᵢ‖²/2mᵢ − Σ_{i<j} mᵢmⱼ/rᵢⱼ          L_z = Σ (xᵢ p_{y,i} − yᵢ p_{x,i})
ΔE = E − E₀ ;   δE = ΔE / max(ε_E, |E₀|)      (ε_E = ε_L = 10⁻⁶ — rest-start floors)
accumulate:  ΔE_final, max|ΔE|, ΔL_z final, max|ΔL_z|

**Diffusion (Welford streaming, per macro-step).** Slope of spread `y` on time `t` via centered co-moments, not raw sums (avoids catastrophic cancellation — payload §4 mini-spec). The **time-only** part (`n, mean_t, C_tt`) is **DERIVED, not a shared mutable global** — a per-macro-step shared update is impossible in a GPU dispatch (synchronisation is workgroup-scoped; no dispatch-wide barrier — the same on Rust → SPIR-V as it was on WGSL). For the uniform schedule it is closed-form: `n = step_count`, `mean_t = (n+1)h/2`, `C_tt(n) = h²·n(n²−1)/12`. The **per-sample** part (`mean_y, C_ty`) updates in the march using the **OLD-mean time deviation** `δ_t = 0.5·n·h` (NOT `t − mean_t` post-insertion): `dy = y − mean_y; mean_y += dy/n; C_ty += δ_t·(y − mean_y)` — two f32 accumulators (precision-sensitive, no f16). Slope `= C_ty / C_tt(n)` at read, **invalid for `n < 2`** (sentinel, never divide-by-zero).
suspect predicates (READ-TIME, over the stored drift latches — no stored suspect bits, payload §5): energy-suspect on relative δE ; L_z-suspect on **absolute** ΔL_z
```

Final *and* max are both stored because the *shape* of drift is the diagnostic: bounded oscillation ≈ fine; secular ≈ concerning; a spike that recovers = close-encounter stress (the Drift-shape cross-check view).

### 3.6 Detectors (per `STEP`, on the projected state)

**Collision:** `min_{i<j} ‖rᵢ − rⱼ‖² < r_coll²` → `COLLISION(pair)` (pair id per payload §2, R-22) — the **squared** comparison (no runtime `sqrt` in the branch, per §3.3's comparison-only rule). Uses the same squared min-separation the substepper buckets on and `d_min` displays (as its √) — one value, three consumers.
**Count before classifying (pending change 7, landed):** count the pairs with `‖rᵢ − rⱼ‖² < r_coll²`. Exactly 1 → `COLLISION(pair)`; 2 or more → `COLLISION`, `detail = 3` (triple collision, terminal and non-continuable). Testing "any pair" first steals genuine triples into the binary arm. `d_min` is the primary stored quantity and `r_coll` a recorded parameter (integrator contract Part 7).

**Escape (pending change 11, landed):**

```
ESCAPE  ⟺  |Δn̂| over a window < tau    AND    E_rel > 0
```

The shape vector has settled and the escaper is unbound. `tau` was measured in a 383× gap and not tuned (integrator contract
Part 7; `principia_01_pitfalls.md` §2); the gap is to re-measure with R-29's `E_rel`. Defined by R-29:

- `E_rel = ½|Δv|² − (M_pair + m_b)/d` is the relative two-body energy of the candidate escaper `b` about the centre of
  mass of the other two: `Δv` and `d` are `b`'s velocity and distance relative to that centre of mass, and `M_pair` is the
  pair's mass (`G = 1`). It uses the **total** mass. An `M_pair`-only form (prin-rs) biases toward escape.
- **The window:** `|Δn̂|` is taken over 0.4 time units (**provisional**), sampled at macro-step boundaries for
  unregularised occupants and at sync boundaries for regularised ones (R-95).
- **The escaper** is the body with `E_rel > 0` and the largest separation from the other two: its distance `d` to their
  centre of mass, the same `d` as in `E_rel` (R-61).
- **To re-measure:** precision, recall and the `tau` gap were measured before `E_rel` was fixed. Re-validate them with
  this `E_rel`, against check 2's independent ground truth (pitfalls §2.4), keeping the legacy `t = 30` set as a
  comparison (R-95).
- **After escape fires (R-31, R-95, R-103):** `state` reads `escape` and `t_end` is fixed. Time averages (FTLE's `S/T` and
  the like) freeze at `t_esc`. In production `done` is set when escape fires and the loop ends. The post-escape march
  for the checks of pitfalls §2.4 runs only in the validation harness, which keeps its own state; the payload never
  sees it.

Triple ejection is `ESCAPE` with `detail = 3`; its gate is ruled by R-32, applied later.

*Superseded by change 11, kept for the record* — for each candidate body `k` (outer of the Jacobi split against the remaining pair), three gates and a persistence counter:

```
G1  ‖λ‖   > R_esc                    (far)
G2  λ·λ̇   > 0                        (outward)
G3  E_out = ½ μ_λ ‖λ̇‖² − m_k·M_pair/‖λ‖  > 0     (unbound w.r.t. the pair; G = 1)

counter:  all three pass → c_k += 1 ;  any fails → c_k = max(0, c_k − 1)
terminal: c_k ≥ k_esc (= 8)  →  ESCAPE(body k)
```

The ±1 counter with floor is what absorbs single-step gate flicker near thresholds — the sanctioned honest divergence of the integrator contract. *(This detector fired on transients: 0 of 895 escapes were still unbound eight sync boundaries later.)*

**Terminal taxonomy & priority (pinned here — confirm/veto):** labels are mutually exclusive; when multiple could fire in one `STEP`, the deterministic order is
```
SIM_FAILED (non-finite state)  >  COLLISION  >  ESCAPE  >  BOUNDED (t ≥ T — horizon reached; there is NO separate timeout state, payload §2)
```

`MAX_SUBSTEPS` is **not in this ordering — it is not a terminal condition.** Hitting the substep cap `N_max` is a statement about the *integrator* (this step was too hard to resolve in budget), not the *dynamics* (the trajectory has not ended). Terminating on it would falsely paint every hard-but-survivable close encounter as terminal — i.e. exactly the close-encounter regions (basin boundaries, the near-collision manifold, the Burrau neighbourhood) that are the most scientifically interesting. **Instead: on hitting the cap, advance the trajectory with the best-available (under-resolved) state; the march continues to its real dynamical outcome.** The **`saturated`** descriptor bit (a *stored* sticky flag — payload §2) records that `N_sub == N_max` occurred at some step; the exact cumulative work lives in the `total_substeps` u32 (from which the `total_substeps_log2` complexity proxy is *derived* at read). The cap still bounds work-per-step (protecting the frame-loop barrier from stalling on one pixel), but it triggers *advance-and-flag*, not *terminate*. `saturated` is a **confidence annotation**, not an outcome class — the terminal outcome is always dynamical (escape / collision / bounded; there is no `timeout` state — reaching the horizon without escape or collision IS `bounded`, payload §2), and the flag marks how trustworthy that outcome is.

**Determinism (parity):** the cap must be on substep *count* (`N_max`), never wall-clock — so the *decision to cap* stays a bit-identical shared branch and the *capped step* is computed identically on both pipelines. Reduced accuracy, but reproducible: both sides take the same under-resolved step and flag it. (A wall-clock cap would be nondeterministic and break parity — forbidden.)
A deterministic priority is *required* by the shared-branch rule; collision-over-escape because a collision is a singular event that invalidates further gating. This ordering is a pin this drill-down introduces — it must be identical CPU/GPU and belongs in the shared physics source. **(Amended: `MAX_SUBSTEPS` was removed from this chain — it is a non-terminal saturation flag, above.)**

### 3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)

**Nothing is stored.** Each macro-step, the shape vector `n(t)` is *derived from the current state* (the map below — math unchanged from the checkpoint era) and read by the render stage directly; the unwrapped phase `θ̃` persists as a **running accumulator** in `SimState`. There is no checkpoint array and no schedule — the old `float4(n(t_m), θ̃(t_m)) × M` write is deleted.

```
Shape map (Montgomery), from mass-weighted Jacobi (ρ̃, λ̃), I = ‖ρ̃‖² + ‖λ̃‖²:
   u = ‖ρ̃‖² − ‖λ̃‖²          v = 2 (ρ̃ · λ̃)          w = 2 (ρ̃ ∧ λ̃)   (signed area)
   n = (u, v, w)/I ∈ S²      — w = ±1 at the Lagrange (equilateral) poles
```

**The physics-overlay convention** (Montgomery's shape sphere, equal masses). Three classes of special
configuration sit at fixed points:

| symbol | configuration | location | count |
|---|---|---|---|
| $BC_{01}, BC_{12}, BC_{20}$ | binary collisions | equator, 120° apart | 3 |
| $E_0, E_1, E_2$ | Euler collinear | equator, between the collisions | 3 |
| $L^+, L^-$ | Lagrange equilateral | north and south poles | 2 |

**The landmarks are computed from the shape map above, not hard-coded (R-14).** A binary collision's $\hat{\mathbf b}$ is
`n` evaluated at a configuration with that pair coincident, using the current masses. The collision of bodies 0 and 1
($\tilde\rho = 0$) is always at $\hat{\mathbf b}_{01} = (-1, 0, 0)$. With equal masses the other two land at
$\hat{\mathbf b}_{12} = \left(\tfrac12, \tfrac{\sqrt3}{2}, 0\right)$ and $\hat{\mathbf b}_{20} = \left(\tfrac12, -\tfrac{\sqrt3}{2}, 0\right)$, 120° apart, and
$$\hat{\mathbf e}_j = -\hat{\mathbf b}_j \ \text{(equal masses)}, \qquad \hat{\mathbf l}^{\pm} = (0, 0, \pm 1),$$
where $L^+$ ($w = +1$) is the equilateral triangle with bodies 0 → 1 → 2 anticlockwise. Every binary collision lies on
the equator ($w = 0$) for any masses. With unequal masses the three collisions are not 120° apart, and the overlay
marks them at their mass-weighted positions (R-50). The Euler landmarks $\hat{\mathbf e}_j$ are the **Euler central
configurations** — the collinear relative equilibria, roots of Euler's quintic in the mass ratios — mapped through the
shape map; equal masses reduce to the antipodes $-\hat{\mathbf b}_j$ above. The quintic is transcribed with citation by the
task that builds the landmarks, physics-reviewed and confirmed at the gate (R-126).

The table's labels are 0-based (R-22): `BC₀₁` is bodies 0 and 1, i.e. $\hat{\mathbf b}_{01}$, which is pair 2 in the payload's
pair-id map (pair `k` is the side opposite body `k`). Axis assignment follows this convention: the form of `n` above is
fixed, and the component→axis order is R-14's, the same as the IC Inspector's (`principia_chart_reference.md` §3.1, §3.3).

**Unwrapped phase** `θ̃`: the equatorial longitude `atan2(n_v-axis, n_u-axis)` accumulated continuously — per step, add the principal-value delta (∈ (−π, π]) so no 2π jumps enter; `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = sign(θ̃) < 0` are **derived at read from the running accumulator**, at any playhead. **Terminal latch:** on termination the state stops advancing and all accumulators freeze at their terminal values (the latch policy the winding cross-check certifies).

### 3.8 Co-computations (tier-gated, ride the forward pass)

**Benettin FTLE** (1 shadow; verbatim from stability-metrics.md, renorm interval renamed `n_renorm` — see §6):
```
init: x' = x₀ + δ₀ (arbitrary direction, ‖δ₀‖ small);  S = 0
every n_renorm steps:  δ⃗ = x' − x ;  δ = ‖δ⃗‖ ;  S += log(δ/δ₀) ;  x' ← x + δ₀·(δ⃗/δ)
λ (read at any playhead / termination) = S_final / (step_count · dt_macro)
   where S_final = S + log(δ_current/δ₀)   — finalise the PARTIAL renorm interval first
   (plain S/T systematically under-reads between renorm boundaries — payload §5)
```
Renormalisation is what makes λ an intrinsic flow quantity; it is also exactly why the Benettin endpoint is not a valid neighbourhood sample.

**Ensemble sensitivity** (E copies per nominal sample, full samples jittered within the footprint at fixed Halton (2,3) offsets — copy 0 the un-jittered centre, copies 1..E at Halton points 1..E centred and scaled to the footprint, R-80 — no intervention):
```
outcome entropy   H = −Σᵢ pᵢ log pᵢ            (categorical final diagnostics)
spread            σ²_T = (1/(E+1)) Σₖ ‖Φ_T(x₀⁽ᵏ⁾) − x̄(T)‖²     (over the footprint's E+1 samples, at resolve)
```
Deliberately resolution-dependent (footprint shrinks with zoom) — a **footprint** quantity (one value per nominal sample, shared by its copies; does not sharp-edge AA), vs FTLE the **point** quantity (per-sample, anti-aliases). Total per nominal sample at full tier: **~2(E+1)** integrations (E+1 samples, each + its Benettin shadow).

---

## 4. Seams (obligations → integration tests)

| Seam | Obligation | Test |
|---|---|---|
| **3** decoder → integrator | consume `(m,r,p)` only; never learn the chart | every chart in the lowering appendix drives the same kernel unmodified |
| **4** occupant ↔ wrapper | occupant is pure `STEP`; wrapper identical across occupants | swap occupants → only §3.2 numbers change; wrapper branch trace identical |
| **5** producer of the payload | every field of §generation-root ledger written, per its metadata | the field debug views live and sane; sentinel/suspect conventions honoured |
| **1** precision ring | f32/f64 same branches on identical inputs, values differ by precision only | branch equality on identical per-step inputs over fuzzed states (R-84); divergence view shows smooth growth, no branch cliffs |
| **9** firewall | no warm starts; no scheduling state in outputs | schedule the same quad twice (different orders/frames) → byte-identical payloads |

---

## 5. Unit tests

Golden anchors: **`z = 0`** (equal-mass, α = π/4, β = π/2, rest) and the **Burrau IC** (via encode from the (3,4,5) construction).

1. **Order scaling (Kepler-embedded):** set `m₂ → ε` so bodies 0–1 form a near-Kepler pair; energy error over one orbit scales `∝ dt²` (KDK), `∝ dt⁴` (Yoshida-4, RK4), `∝ dt⁶` (Yoshida-6) across a dt ladder.
2. **Symplectic vs not:** long bounded orbit — KDK/Yoshida energy error *oscillates* about E₀ with no secular trend; RK4 and Euler drift secularly; **Euler's violent drift lighting up `SUSPECT_ENERGY` is the pass condition** (the debug-occupant test).
3. **Step reversibility (occupant-level):** for symplectic occupants, one `STEP(dt)` then momentum-negate then `STEP(dt)` then negate returns the state to precision — *without* wrapper (projection/substep excluded); documents exactly where reversibility lives and where it breaks.
4. **Substep determinism:** over fuzzed states straddling **bucket edges** (§3.3 threshold table), **all backends** — CPU-f64, CPU-f32, native-GPU, browser-GPU-via-WGSL — produce the identical `N_sub` integer, always (0 forks; the CPU-fround approach this replaced forked 5/272).
5. **Detector — escape:** a synthetic hyperbolic ejection fires `ESCAPE` once `|Δn̂|` over the window falls below `tau` with `E_rel > 0`; a grazing near-escape that turns back never fires; a settled bound hierarchy (closure small, `E_rel < 0`) never fires. *(Superseded form, change 11: passes G1–G3 and terminates at exactly `c = k_esc` steps after gates hold; the grazing case exercises the −1 decrement.)*
6. **Detector — collision & priority:** head-on pair crosses `r_coll` → `COLLISION(pair)` with the correct pair id; two pairs below `r_coll` in the same step → `COLLISION` with `detail = 3`, never a binary label (change 7); a contrived same-step collision+escape resolves per §3.6 priority, identically on both precisions.
7. **Drift shape:** a close-encounter IC shows `max|ΔE| ≫ |ΔE_final|` (spike that recovered) — validates storing both and the cross-check view's premise.
8. **Winding & terminal latch:** on a circulating bounded orbit, `θ̃` is continuous (no 2π jumps), `orbit_count` matches a hand-counted winding, `retrograde` matches the L_z sign; post-event slots are frozen at the terminal state (finalisation).
9. **Shape-map identities:** `‖n‖ = 1` always; equilateral configs → `n_w = ±1` (poles); collinear (Euler) configs → `n_w = 0` (equator); binary-collision limits approach the b̂ points. **Numeric landmarks (R-14):** `BC₀₁ → (−1, 0, 0)` for any masses; `L⁺` (bodies 0 → 1 → 2 anticlockwise, equilateral) `→ (0, 0, +1)`; all three binary collisions at `w = 0` for random masses; equal masses put the collisions 120° apart (azimuths 180°, 60°, 300°). **Cross-check:** `n` against the IC Inspector's JS (`shapePoint` after its canonicalise, `docs/gui/reference/ic_inspector.html`) on random ICs, agreeing to f64 round-off once the mirror fold is applied (the Inspector canonicalises to `w ≥ 0`, so compare against `|w|`). Checked at step 3: 5,000 random ICs, max difference 1.2e−15.
10. **Benettin invariance:** halving `δ₀` and doubling `n_renorm` (within the linear regime) leaves λ_T unchanged within tolerance; λ_T ≈ 0 on the quasi-regular Kepler-embedded orbit, large on Burrau.
11. **Ensemble sanity:** deep basin-interior pixel → `H = 0`, tiny σ²; a straddling boundary pixel → `H > 0`; and each sample's **Benettin shadow is excluded from the colour/spread pool** (asserting the shadow-is-not-a-sample separation structurally).
12. **Burrau smoke (literature-anchored):** the classical rest start resolves to binary + escaper within the default horizon, qualitatively matching Szebehely & Peters (1967); asserted as outcome-class + coarse `t_end` window, not trajectories.
13. **Firewall/purity:** test in seam table — same quad, two schedules, byte-identical.

---

## 6. Deferred / flagged

<!-- retired-terms -->
- **Priority-order pin (§3.6)** — introduced here because the shared-branch rule demands *some* deterministic order; confirm or veto it as decision B4 on the step-5 sheet (R-6). There is no older rule to check it against. Must land in the shared physics source either way.
- **Naming: `n_renorm`** — the Benettin renorm interval keeps this name (the old `M`-vs-checkpoint-count collision is moot: checkpoints are gone under lockstep).
- ~~**Shape-map axis assignment**~~ — **settled by R-14:** `n = (u, v, w)/I` with the standard cross, θ azimuthal in `(u, v)`, φ polar from `+w` (§3.7, chart_reference §3.1 and §3.3).
- **Yoshida-6 coefficients** — verify the three w's against Yoshida (1990) Table 1 solution A before they enter the shared source (paper already in the lit set).
- **Reversibility replay & KS regularisation** — bounded and deferred per the integrator contract Part 6; nothing here forecloses either.
<!-- /retired-terms -->

---

*Five occupants, one wrapper, one table-bucketed substep integer. Collision beats escape, and both precisions agree on it. Closure and energy together decide escape; the accumulators latch at the end; a sample's Benettin shadow is never itself a sample. Euler exploding is a test passing.*
