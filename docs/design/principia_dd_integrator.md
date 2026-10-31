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

From the **integrator contract**: occupant = `ADVANCE(state, t_now, t_target, params) → state'`, nothing else (KDK/Yoshida implement it as the wrapper loop around their `STEP`; R-19); wrapper owns everything shared; **cadence pinned per-`STEP`** (projection, monitoring, detection after every step; shape readout on the macro schedule; nothing stored — lockstep); capability profile `{order, force_evals, symplectic, step_reversible, run_reversible}` read by all consumers (R-500; *was:* `{…, reversible}`); occupant on the sim key; Euler is a debug tool; units `G = M = I = 1`, `T` physical, default 50, raised or lowered (R-575; `∈ [0.4, 200]`, R-610, the floor one closure window and the cap 200; *was:* "`∈ (0, 200]` meanwhile, RQ-555"; *was:* "`T ∈ [50, 200]` physical (default 50)"), `dt_macro = max(1e-3, T/65535)` (R-132), schedule length `⌈T/dt_macro⌉` deterministic; **values may diverge by precision, wrapper branches may not on identical inputs** (R-84).

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
w₁ = 1 / (2 − 2^{1/3}) = 1.3512071919596575         (correctly rounded to f64)
w₀ = 1 − 2w₁           = −1.702414383919315          (computed in f64; w₀ + 2w₁ = 1 exactly)
drift coefficients d = [w₁, w₀, w₁]
kick  coefficients c = [w₁/2, (w₀+w₁)/2, (w₀+w₁)/2, w₁/2]
```
i.e. `KDK(w₁·dt) ∘ KDK(w₀·dt) ∘ KDK(w₁·dt)`. The negative middle step is real, and the composition is still reversible at a fixed step: it is symmetric, so `step_reversible` holds whatever the sign of `w₀` (test 3, §5). What breaks naive time-reversal of a run is the wrapper's adaptive substepping and COM projection (integrator contract Part 6; R-500, closing RQ-369). *Was:* "The negative middle step is real and is why naive time-reversal fails (integrator contract Part 6)."
*Citation (R-501):* Yoshida, H. (1990), "Construction of higher order symplectic integrators", *Phys. Lett. A* 150,
262–268: the fourth-order symmetric triple composition, whose weights are given in closed form there, `w₁ = 1 / (2 −
2^{1/3})` and `w₀ = −2^{1/3} · w₁`. *The digits (R-515, closing RQ-405):* `w₁` is the closed form's correctly rounded f64
value (exact `1.351207191959657634…`, relative difference `6.2·10⁻¹⁷`); `w₀` is `1 − 2w₁` as computed in f64, a
subtraction that is exact, so that `w₀ + 2w₁ = 1` holds exactly in f64, as Yoshida-6's `w₀` is computed from its
consistency condition. It differs from the closed form's `w₀ = −1.702414383919315268…` by `9.9·10⁻¹⁷` relative, and the
cubic order condition's residual `|2w₁³ + w₀³| / Σ|w|³` is `5.5·10⁻¹⁷` (§5, test 1a). Each printed value is the shortest
decimal that parses to the shipped f64. *Was (RQ-405's (a), applied meanwhile):* `w₁ = … = 1.3512071919596578`,
`w₀ = −2^{1/3} · w₁ = −1.7024143839193157 (w₀ + 2w₁ = 1)`, digits that were neither the closed form's correctly rounded
f64 values nor gave `w₀ + 2w₁ = 1` in f64 (`0.9999999999999998`; physics review 5587467237 of PR #258, P2).

**Yoshida-6** (order 6, 7 force evals, symplectic) — Yoshida (1990), **solution A**:
```
w₁ = −1.17767998417887
w₂ =  0.235573213359357
w₃ =  0.784513610477560
w₀ =  1 − 2(w₁ + w₂ + w₃) = 1.31518632068391
drift sequence d = [w₃, w₂, w₁, w₀, w₁, w₂, w₃]
kick sequence  cᵢ = (d_{i−1} + dᵢ)/2  with  d₀ = d₈ = 0
```
*Citation (R-501):* Yoshida, H. (1990), "Construction of higher order symplectic integrators", *Phys. Lett. A* 150,
262–268, Table 1, solution A (`w₁`, `w₂`, `w₃` as printed above; `w₀` from the consistency condition). The paper is not
added to the corpus: the citation stands with the digits, and the transcription is checked by a convergence-order test
(§5, test 1a), which fails on a transcription error. Like Yoshida-4's, the composition is symmetric, so it is
reversible at a fixed step despite its negative weight `w₁` (test 3; R-500). *Was:* "(Transcription check: verify the
three w's against Yoshida 1990 Table 1 solution A — the paper is in the lit-review set — before they enter the shared
physics source.)" (R-57; R-501 replaces the check against the paper with the citation and the test.)

**RK4** (order 4, 4 force evals, **not** symplectic — Validation cross-check occupant; the inspector's *adaptive* high-precision reference is the separate RK45, integrator contract §6): the standard tableau on the full state `y = (r, p)`, `ẏ = (p/m, F(r))`:
```
k₁ = f(y);  k₂ = f(y + dt/2·k₁);  k₃ = f(y + dt/2·k₂);  k₄ = f(y + dt·k₃)
y' = y + (dt/6)(k₁ + 2k₂ + 2k₃ + k₄)
```

These are the steppers with regularisation `none`, where the Hamiltonian is separable. Every stepper also runs under
every regularisation (R-561); on a non-separable one (Aarseth–Zare, Heggie) KDK and the Yoshida steps are the explicit
composition integrator contract Part 2b defines (the sub-steps in this section's drift order, their adjacent half-kicks
not merged: the merged kick sequence above holds on a separable Hamiltonian only), and the pairs measured not to work
carry a warning (its table). The default occupant is Heggie × RK4 (R-560).

### 3.3 Substep law — and the determinism pin, made concrete

```
N_sub(r_min) = clamp( ⌈ (r_sub / r_min)^{γ_sub} ⌉ , 1 , N_max )    [defaults r_sub = 0.05, γ_sub = 1.5, N_max = 64; each on the sim key]
```

<!-- retired-terms -->
This formula *specifies* `N_sub`; it does **not** compute it at runtime. `N_sub` is a **branch decision**, and the determinism rule for branch decisions is stronger and different from what an earlier draft assumed (which had `N_sub` computed as the f32 evaluation with `Math.fround`-per-stage on the CPU to match the GPU). **The spike (`principia_spike_brief.md` findings, 2026-07) measured that rule forking** — 5/272 boundary states — and a **controlled test overturned the cause**: it is **not** fast-math and **not** FMA contraction, but *inherent cross-implementation transcendental latitude*. A GPU's `pow` and libm's `powf` legitimately disagree by 1–3 ulp (e.g. `pow(4.000000477, 1.5)` → exactly `8.0` on Metal vs `8.000001907` in libm), **in both fast and safe math modes** — so *any* runtime transcendental feeding a `ceil` at an integer boundary can fork, and no amount of matched rounding fixes it because the two libraries are each entitled to their own result.
<!-- /retired-terms -->

**The rule (this is the determinism pin, replacing the f32-evaluation framing):** a branch decision may depend only on **comparisons against compile-time constants, or against values every backend receives bit-identically** (the sim key's threshold table, R-502; *was:* "against compile-time constants") and on **single-rounded arithmetic** — never on a runtime `pow`/`sqrt`/`div`. Concretely:

```
N_sub  =  min( bucket( d² ), N_max )  via a generated comparison tree over the sim key's table
          THR[n] = round_f32( ( r_sub · n^{−1/γ_sub} )² )   for n = 1 … L_max     (generated per sim key on the host; L_max the maximum length)
          — at γ_sub = 1.5, THR[n] = round_f32( ( r_sub / n^{2/3} )² );
          — smaller d² ⇒ smaller r_min ⇒ larger N_sub;  no runtime sqrt / div / pow / loop / dynamic index
```

The transcendental (`pow`) still exists — it moves from *runtime, every substep, every backend* (where it forks) to *once per sim key, on the host, into a table* (where it cannot). *Was:* "to *once, at table-generation time, into a constant* … The generator may use f64 libm freely; only the frozen f32 output ships", with the code block's "(frozen at build time)" and "over 64 constants".

**The table is generated per sim key (R-502, closing RQ-370).** `r_sub`, `gamma_sub` and `N_max` are user-adjustable
sim-key values (integrator contract Part 3), so the thresholds are generated on the host for the sim key's values,
`THR[1..=L_max]` by the one formula (monotone, so the tree is; `min(·, N_max)` caps the result), and shipped in `SimUniforms` as exact f32 bit patterns; the comparison tree reads them by constant
index, its shape generated once over the table's maximum length. The branch still compares the one quantised float,
`d²`, against values both pipelines receive bit-identically, so the determinism pin holds; a change of `r_sub`,
`gamma_sub` or `N_max` is a sim-key change and re-integrates, as before. The host computes each threshold in f64 with a
**pinned pure-software libm** (no platform libm, no compiler intrinsic), rounding once to f32, so the native host and
the wasm host give identical bits. The table's **maximum length** is a calibration value (R-71): an `N_max` above it is
refused, never truncated. A test asserts that the default configuration (`r_sub = 0.05`, `γ_sub = 1.5`, `N_max = 64`)
reproduces the build-time table, generated by `cargo xtask codegen` and checked in, bit for bit.

**The one value feeding the comparison must itself be built identically.** `d²` is a scrap of arithmetic (`dx² + dy²`), and if one backend contracts it to an `fma` and another does not it can differ by a ulp — re-forking at a bucket edge. So `d²` is **position-quantised**: cast positions to f32, then lone-subtract, and form the squares and their sum only from operations every IEEE backend rounds alike, with no intermediate that can be subnormal (R-547; gpu_determinism_note rules 2 and 6), in a form TASK-M4-03 writes and proves (REQ-INT-066). *Was:* "then lone-subtract, lone-multiply, and an **explicitly written** `fma(dy, dy, dx*dx)` (explicit, so every backend is forced onto the identical fused op rather than choosing)". Then the sole float feeding the sole branch is bit-identical everywhere, and the branch cannot fork.

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

Under a regularised occupant the energy-suspect predicate reads the **regularised** energy, and `E` above, the
Cartesian energy, is a diagnostic read only from runs that did not stop at an event (R-566; integrator contract Part 1,
"Under a regularised occupant, two energies, named (R-566)"). Both drifts are stored (R-605, closing RQ-528): the
regularised drift latches in payload fields of their own (REQ-PAY-111), read by the suspect predicates and the drift
view under a regularised occupant, the Cartesian ones kept beside them for diagnostics only. *Was:* "; where the
regularised drift is stored is RQ-528".

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
  unregularised occupants and at sync boundaries for regularised ones (R-95), on the fixed grid `Δ_sync`, the window
  stated in physical time and never as a count of boundaries (R-565).
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

*Superseded by R-30, kept for the record (R-339)* — the order below was R-6's pin until the order was decided. R-30 orders terminal events by time (REQ-EVT-020): sim_failed first; then whichever event came first, a same-time tie going to collision; triple ejection as a detail of escape; then running; then bounded. Labels stay mutually exclusive, and there is still no timeout state.

**Terminal taxonomy & priority (pinned here — confirm/veto):** labels are mutually exclusive; when multiple could fire in one `STEP`, the deterministic order is
```
SIM_FAILED (non-finite state)  >  COLLISION  >  ESCAPE  >  BOUNDED (t ≥ T — horizon reached; there is NO separate timeout state, payload §2)
```

`MAX_SUBSTEPS` is **not in this ordering — it is not a terminal condition.** Hitting the substep cap `N_max` is a statement about the *integrator* (this step was too hard to resolve in budget), not the *dynamics* (the trajectory has not ended). Terminating on it would falsely paint every hard-but-survivable close encounter as terminal — i.e. exactly the close-encounter regions (basin boundaries, the near-collision manifold, the Burrau neighbourhood) that are the most scientifically interesting. **Instead: on hitting the cap, advance the trajectory with the best-available (under-resolved) state; the march continues to its real dynamical outcome.** The **`saturated`** descriptor bit (a *stored* sticky flag — payload §2) records that `N_sub == N_max` occurred at some step; the exact cumulative work lives in the `total_substeps` u32 (from which the `total_substeps_log2` complexity proxy is *derived* at read). The cap still bounds work-per-step (protecting the frame-loop barrier from stalling on one pixel), but it triggers *advance-and-flag*, not *terminate*. `saturated` is a **confidence annotation**, not an outcome class — the terminal outcome is always dynamical (escape / collision / bounded; there is no `timeout` state — reaching the horizon without escape or collision IS `bounded`, payload §2), and the flag marks how trustworthy that outcome is.

**Determinism (parity):** the cap must be on substep *count* (`N_max`), never wall-clock — so the *decision to cap* stays a bit-identical shared branch and the *capped step* is computed identically on both pipelines. Reduced accuracy, but reproducible: both sides take the same under-resolved step and flag it. (A wall-clock cap would be nondeterministic and break parity — forbidden.)
*The ordering this paragraph argues for is superseded by R-30, kept for the record (R-339); that the order is deterministic, identical CPU/GPU and in the shared physics source still holds (REQ-EVT-020).* A deterministic priority is *required* by the shared-branch rule; collision-over-escape because a collision is a singular event that invalidates further gating. This ordering is a pin this drill-down introduces — it must be identical CPU/GPU and belongs in the shared physics source. **(Amended: `MAX_SUBSTEPS` was removed from this chain — it is a non-terminal saturation flag, above.)**

### 3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)

**Nothing is stored.** Each macro-step, the shape vector `n(t)` is *derived from the current state* (the map below — math unchanged from the checkpoint era) and read by the render stage directly; the unwrapped phase `θ̃` persists as a **running accumulator** in `SimState`. There is no checkpoint array and no schedule — the old `float4(n(t_m), θ̃(t_m)) × M` write is deleted.

```
Shape map (Montgomery), from mass-weighted Jacobi (ρ̃, λ̃), I = ‖ρ̃‖² + ‖λ̃‖²:
   u = ‖ρ̃‖² − ‖λ̃‖²          v = 2 (ρ̃ · λ̃)          w = 2 (ρ̃ ∧ λ̃)   (signed area)
   n = (u, v, w)/I ∈ S²      — w = ±1 at the Lagrange (equilateral) poles at equal masses (R-549)
```

**The physics-overlay convention** (Montgomery's shape sphere, equal masses). Three classes of special
configuration sit at fixed points:

| symbol | configuration | location | count |
|---|---|---|---|
| $BC_{01}, BC_{12}, BC_{20}$ | binary collisions | equator, 120° apart | 3 |
| $E_0, E_1, E_2$ | Euler collinear, $E_j$ with body $j$ in the middle | equator, between the collisions | 3 |
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
These labels are internal. Shown on the overlay, they name the bodies 1-based (R-449): `BC₀₁`, `BC₁₂` and `BC₂₀` read
`BC₁₂`, `BC₂₃` and `BC₁₃`, a pair's bodies smaller first. `E_j` is the collinear configuration with body `j` in the
middle, and it is shown as `E_{j+1}` through R-449's one conversion function, so `E₀, E₁, E₂` read `E₁, E₂, E₃`;
its antipodality to the other two bodies' binary collision (`ê_j = −b̂_j`, `b̂_j` the collision of pair `j`, the side
opposite body `j`) is stated for equal masses only (R-549, RQ-334). *Was:* "The table's `E₀, E₁, E₂` index is not
defined beyond `ê_j = −b̂_j`; how they are shown is RQ-334."

**Unwrapped phase** `θ̃`: the equatorial longitude `atan2(n_v-axis, n_u-axis)` accumulated continuously — per step, add the principal-value delta (∈ (−π, π]) so no 2π jumps enter; `orbit_count = ⌊|θ̃|/2π⌋` and `retrograde = sign(θ̃) < 0` are **derived at read from the running accumulator**, at any playhead. **Terminal latch:** on termination the state stops advancing and all accumulators freeze at their terminal values (the latch policy the winding cross-check certifies).

**Unwrapped phase — its start and the poles (R-389).** `θ̃` starts at 0, `θ̃(0) = 0`: it counts the net turning since the IC, so `orbit_count` is the completed revolutions and `retrograde` the net clockwise motion, independent of the starting longitude. At the poles (`n_u = n_v = 0`, `w = ±1`) the longitude is undefined, so `θ̃` **holds below a radius, with a frozen reference**: on entering `√(n_u² + n_v²) < r_pole`, the last longitude (the last one outside the radius) is stored; no delta is added while inside; on exit, `wrap(exit longitude − stored longitude)` is added, wrapped into (−π, π], the interval of the per-step principal-value delta. **The exact-π case:** a longitude difference of exactly ±π, per step or on exit, is added as **+π** — the wrap convention's interval is closed at +π and open at −π. A run that terminates inside the radius adds nothing for that passage (the terminal latch above freezes `θ̃` as it stands). `r_pole` is a calibration (R-71; REQ-INT-086), proposed with its evidence by the task that builds `θ̃` and confirmed by the human at the M1 gate. **Confirmed at the M1 gate (R-433): `r_pole` = 0.01**, a constants-register entry (dd_generation_root §3.8) marked provisional until M3 and read from the register, never a constant of its own; M3 re-confirms it against real orbits' near-pole arcs (TASK-M3-11). An IC that starts inside the radius has no stored longitude, and **its first exit adds no delta** (R-392): `θ̃` is still 0 at that exit, and it starts counting from the exit longitude, the first longitude the hold trusts. A longitude inside `r_pole` is never trusted, the IC's included; the turning before the first exit is not counted.

**Unwrapped phase — the hold across a resumed march (R-397).** R-389 and R-392 stand as written; this says where the hold keeps its state. The stored longitude lives in `SimState`'s `_reserved` as a u16 code in steps of `2π/65535`, with **0xFFFF for "no reference"**, R-392's IC that starts inside the radius (payload §2 gives the encode, the decode and the lifecycle). No descriptor bit is used. **"Inside the hold" is not stored:** a march that resumes from a stored `SimState` recomputes it from the current state, `√(n_u² + n_v²) < r_pole`, by the same test as the uninterrupted march. The reference is quantised when it is stored, on entry, and every exit differences against the decoded code, so `θ̃` is bit-identical however the march is split into dispatches. **The quantisation:** "the stored longitude" in R-389's exit delta is the decoded code, which differs from the last longitude outside the radius by at most half a step, `π/65535 ≈ 4.8e-5` rad, plus the encode's round-off at the kernel's `Real`, bounded by `4·ulp_Real(2π)` (1.9e-6 rad at f32, 3.6e-15 at f64), whatever the evaluation order: at most ~5e-5 rad of `θ̃` per pole passage. The per-step deltas outside the radius are not quantised. The exact-π case applies to the difference against the decoded reference.

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
   1a. **Convergence order of the transcribed coefficients (R-501):** on the same fixture, the fitted quantity is the
   maximum relative energy error over one orbit, `max_t |E(t) − E₀| / |E₀|`, fitted against `dt` on a log–log ladder;
   Yoshida-4's fitted order rounds to 4 and Yoshida-6's to 6. *Controls:* each free coefficient (Yoshida-4's `w₁`;
   Yoshida-6's `w₁`, `w₂`, `w₃`) with its **4th significant digit** changed, `w₀` recomputed from the consistency
   condition so that only the higher order conditions break, must give a fitted order that rounds lower. A relative
   change `δ` adds an error term of about `δ·dt²`, which the ladder resolves only above a **detection floor**, estimated at
   about `3·10⁻⁷` relative for Yoshida-4 (`δ² ≳ 10⁻¹³`) and about `10⁻⁹` for Yoshida-6 (`δ^{3/2} ≳ 10⁻¹³`) at f64, so
   about the 7th and the 10th significant digits; the test measures the floor (the smallest relative change it
   detects) and prints it. **The later digits** are covered by REQ-INT-037's equality to the printed digits and by an
   **algebraic check** on the shipped f64 constants, evaluated exactly (in rational or 50-digit decimal arithmetic) on
   their f64 values, of the order conditions of a symmetric composition of a symmetric second-order step,
   `S(h) = exp(hA₁ + h³A₃ + h⁵A₅ + …)`: `|Σw − 1|` and `|Σw³| / Σ|w|³` for both, and for Yoshida-6 the two fifth-order
   conditions, `|Σw⁵| / Σ|w|⁵` and the nested one, the coefficient of `[A₁,[A₁,A₃]]` in the composition's
   Baker–Campbell–Hausdorff logarithm at `h⁵` (taken exactly in the free algebra truncated at degree 5), divided by the
   same coefficient computed with `|w|`. With the nested condition the set is complete to order 6 (the degree-5 Lie
   elements are `A₅` and `[A₁,[A₁,A₃]]`), which physics confirms; *as first ported:* `Σw⁵` alone, a necessary subset
   (qa review 5587775086 B). The residuals are at most `3·10⁻¹⁵` (Yoshida-4, f64 rounding) and `5·10⁻¹⁴` (Yoshida-6,
   Table 1's 15-digit truncation). Measured on the printed digits: Yoshida-4 `|Σw − 1| = 0` and `5.5·10⁻¹⁷` (R-515; *was* `2.9·10⁻¹⁶`, on RQ-405's interim digits), Yoshida-6 `5.1·10⁻¹⁵` (`Σw⁵`;
   the nested condition `4.0·10⁻¹⁶`). Its controls: each coefficient (Yoshida-6's `w₀` recomputed) changed by one unit of its 15th significant digit (Yoshida-4) or its 13th
   (Yoshida-6), in each direction, must exceed the bound (measured: at least `9.9·10⁻¹⁵` and `2.8·10⁻¹³`; R-515, *was* `9.8·10⁻¹⁵` on RQ-405's interim digits). Past those
   positions a change can lower a residual (Table 1's own truncation sets Yoshida-6's, f64 rounding Yoshida-4's), and a
   last-digit change of Yoshida-4 can parse to the same f64, so the last printed digits are REQ-INT-037's alone. It
   stands in for checking the digits against the paper, which is cited, not held (§3.2). *As first ported:* "the
   control is each coefficient with one printed digit changed, … which … must fail the test" (code review 5587492719
   C5, physics review 5587467237 P1, qa review 5587466555 item 1 of PR #258: unmeetable for the late digits).
2. **Symplectic vs not:** long bounded orbit — KDK/Yoshida energy error *oscillates* about E₀ with no secular trend; RK4 and Euler drift secularly; **Euler's violent drift lighting up `SUSPECT_ENERGY` is the pass condition** (the debug-occupant test).
   *The long bounded orbit (TASK-M3-05's definition, R-72):* the Chenciner–Montgomery figure-eight
   (`principia_dd_validation_orbits.md` §1.2), equal masses, `L_z = 0`, from its published initial conditions in
   `G = 1`, `mᵢ = 1`, `x₁ = −x₂ = (0.97000436, −0.24308753)`, `x₃ = 0`, `v₃ = (−0.93240737, −0.86473146)`,
   `v₁ = v₂ = −v₃/2`, brought to the sim's units (integrator contract Part 5): the masses `1/3`, the velocities scaled by
   `√(1/3)` so the orbit keeps its shape, then canonicalised, and run to the horizon cap `T = 200` at the default sim key
   (`dt_macro = max(10⁻³, T/65535)`, the default substep law). Bounded, stable to small perturbations and never close,
   so a symplectic occupant's energy error stays at its oscillation and Euler's secular drift has the whole horizon to
   grow (`validation::drift::figure_eight`).
3. **Step reversibility (occupant-level):** for symplectic occupants, one `STEP(dt)` then momentum-negate then `STEP(dt)` then negate returns the state to precision — *without* wrapper (projection/substep excluded); documents exactly where reversibility lives and where it breaks. It is what the profile's `step_reversible` means (integrator contract Part 2, R-500): asserted at rounding level for KDK, Yoshida-4 and Yoshida-6, negative weights included; for the regularised occupants it is a measurement, not an assertion: the residual of each built one (Aarseth–Zare + RK4, Heggie, logH in each time mode) is recorded, and its profile's `step_reversible` is the measured verdict. On a regularised occupant's extended state, "momentum-negate" is that occupant's time-reversal map (which regularised momenta are negated, how its time variable and conjugate, and TTL's auxiliary `W`, are mapped), a definition TASK-M3-08 writes (REQ-INT-095, R-72); the residual and its norm are taken on that full extended state. "Rounding level" is a calibrated criterion (REQ-VAL-191, R-71), proposed by TASK-M3-02 and used by both; the measurement's control is a known-asymmetric stepper at the same `dt` (RK4, and REQ-VAL-032's asymmetric Yoshida-4), which must read not reversible.
<!-- retired-terms -->
4. **Substep determinism:** over fuzzed states straddling **bucket edges** (§3.3 threshold table), **all backends** — CPU-f64, CPU-f32, native-GPU, browser-GPU-via-WGSL — produce the identical `N_sub` integer, always (0 forks; the CPU-fround approach this replaced forked 5/272).
<!-- /retired-terms -->
5. **Detector — escape:** a synthetic hyperbolic ejection fires `ESCAPE` once `|Δn̂|` over the window falls below `tau` with `E_rel > 0`; a grazing near-escape that turns back never fires; a settled bound hierarchy (closure small, `E_rel < 0`) never fires. *(Superseded form, change 11: passes G1–G3 and terminates at exactly `c = k_esc` steps after gates hold; the grazing case exercises the −1 decrement.)*
6. **Detector — collision & priority:** head-on pair crosses `r_coll` → `COLLISION(pair)` with the correct pair id; two pairs below `r_coll` in the same step → `COLLISION` with `detail = 3`, never a binary label (change 7); a contrived same-step collision+escape resolves per §3.6 priority, identically on both precisions (R-30's order by time, a same-time tie going to collision; REQ-EVT-020, R-339).
7. **Drift shape:** a close-encounter IC shows `max|ΔE| ≫ |ΔE_final|` (spike that recovered) — validates storing both and the cross-check view's premise.
   *The close-encounter IC (TASK-M3-05's definition, R-72):* a hierarchical triple whose inner binary's pericentre
   passages are the close encounters: masses `(0.45, 0.45, 0.1)`, bodies 0 and 1 a Kepler binary of semi-major axis 1
   and eccentricity 0.9 started at apocentre, body 2 on a circular orbit of radius 4 about the binary's centre of mass,
   in `G = 1`, the whole at rest in its centre of mass, canonicalised, and run to the default horizon `T = 50` at the
   default sim key (`validation::drift::close_encounter`). Its pericentre in the canonical units, about 0.067, stays
   above the substep law's `r_sub = 0.05`, so every step is one substep and each passage is resolved by the step
   itself: the energy error spikes at each passage and recovers after it, the shape this test asks for. `≫` is read as
   at least ten times (applied per R-369). At f32 Yoshida-4 reads 1.1× and Yoshida-6 2×, under that reading: their spikes (4.9e-6
   and 4.7e-9 at f64) sit at or below f32's round-off, so their final drift is round-off too, and the shape is asked of KDK
   at either precision and Yoshida-4 at f64 (physics review 5722380920 of PR #360, F4). The Burrau IC, the golden anchor, is not this test's fixture: its encounters
   are deep enough that the default step does not resolve them, and its drift at f32 neither recovers nor stays at
   round-off in `L_z`.
8. **Winding & terminal latch:** on a circulating bounded orbit, `θ̃` is continuous (no 2π jumps), `orbit_count` matches a hand-counted winding, and `retrograde` (`θ̃`'s sign) matches the hand-counted direction of travel along the equator, the hand count signed and built independently of the march's own `atan2(v, u)`, from the order in which the orbit crosses the three binary-collision half-meridians, each the half of the meridian on `b̂ₖ`'s side (positive dot product with `b̂ₖ`), not the whole great circle, whose far half meets the equator at `−b̂ₖ`, a collinear point in the arc that holds the Euler point `Eₖ` and `Eₖ` itself only at equal masses (physics review 5635696210 of PR #276, non-blocking; *was:* "whose far half runs through the Euler points") (test 9's azimuths; applied per R-369, physics review 5635567722 of PR #276, so that the reversed-sense control can fail, and physics review 5635643767, non-blocking; *was:* "from the order in which the orbit passes the binary-collision meridians"; *was:* "the hand count signed;"); the run from the mirrored state `Π·x` (`y → −y` on every position and momentum) has a `θ̃(t)` and a `retrograde` bitwise identical to the run from `x`'s, and an `L_z` opposite to it (R-580), bitwise (`−L_z` exactly) at every sampled step, on a fixture whose `|L_z|` exceeds a bound stated in the test (on an `L_z = 0` orbit "opposite" holds vacuously); post-event slots are frozen at the terminal state (finalisation). Its controls, each failing: a march with `θ̃`'s sense reversed (`u` and `v` swapped, or `v` negated) fails the direction check while `orbit_count` and the mirror pair pass; `retrograde` computed from the sign of `L_z` (`retrograde` differs across the pair); `θ̃` read from `w` (`θ̃` not identical across the pair); a positions-only `Π` on an initial state whose momenta have nonzero y-components (`L_z` not opposite). Applied per R-369 (qa review 5635487490, Q1 and Q2, and physics review 5635496144, P1, of PR #276): after R-580's drop nothing checked `retrograde`'s value on a real orbit, since `orbit_count` is unsigned and the mirror pair compares two runs that can be wrong the same way; the direction check is R-580's own reading, "`retrograde` is the direction of travel along the shape sphere's equator", and does not reinstate the `L_z` clause. *Was (R-580's port):* "`θ̃` is continuous (no 2π jumps) and `orbit_count` matches a hand-counted winding;" and "and an `L_z` opposite to it (R-580);". `retrograde` is the direction of travel along the shape sphere's equator and `L_z` the rotation in the plane, different quantities: `Π` leaves `u` and `v`, and so `θ̃ = atan2(v, u)` and `retrograde`, unchanged, and negates `w` and `L_z`, so no clause tying `retrograde` to the sign of `L_z` can hold for both members of a mirror pair (R-580, RQ-537). *Was:* "`orbit_count` matches a hand-counted winding, `retrograde` matches the L_z sign;".
9. **Shape-map identities:** `‖n‖ = 1` always; equilateral configs → `n_w = ±1` (poles) at equal masses, and `n_w = ±√(3m₀m₁m₂)/(m₀m₁ + m₀m₂ + m₁m₂)` for every mass set (R-549, RQ-355); collinear (Euler) configs → `n_w = 0` (equator); binary-collision limits approach the b̂ points. **Numeric landmarks (R-14):** `BC₀₁ → (−1, 0, 0)` for any masses; `L⁺` (bodies 0 → 1 → 2 anticlockwise, equilateral) `→ (0, 0, +1)` at equal masses; all three binary collisions at `w = 0` for random masses; equal masses put the collisions 120° apart (azimuths 180°, 60°, 300°). **Cross-check:** `n` against the IC Inspector's JS (`shapePoint` after its canonicalise, `docs/gui/reference/ic_inspector.html`) on random ICs, agreeing to f64 round-off once the mirror fold is applied (the Inspector canonicalises to `w ≥ 0`, so compare against `|w|`). Checked at step 3: 5,000 random ICs, max difference 1.2e−15.
10. **Benettin invariance:** halving `δ₀` and doubling `n_renorm` (within the linear regime) leaves λ_T unchanged within tolerance; λ_T ≈ 0 on the quasi-regular Kepler-embedded orbit, large on Burrau.
11. **Ensemble sanity:** deep basin-interior pixel → `H = 0`, tiny σ²; a straddling boundary pixel → `H > 0`; and each sample's **Benettin shadow is excluded from the colour/spread pool** (asserting the shadow-is-not-a-sample separation structurally).
12. **Burrau smoke (literature-anchored):** the classical rest start resolves to binary + escaper within the default horizon, qualitatively matching Szebehely & Peters (1967); asserted as outcome-class + coarse `t_end` window, not trajectories.
13. **Firewall/purity:** test in seam table — same quad, two schedules, byte-identical.

---

## 6. Deferred / flagged

<!-- retired-terms -->
- ~~**Priority-order pin (§3.6)**~~ — **settled by R-30** (by time; REQ-EVT-020): the pin above is superseded and kept in §3.6 for the record (R-339). Was: introduced here because the shared-branch rule demands *some* deterministic order; confirm or veto it as decision B4 on the step-5 sheet (R-6). There is no older rule to check it against. Must land in the shared physics source either way.
- **Naming: `n_renorm`** — the Benettin renorm interval keeps this name (the old `M`-vs-checkpoint-count collision is moot: checkpoints are gone under lockstep).
- ~~**Shape-map axis assignment**~~ — **settled by R-14:** `n = (u, v, w)/I` with the standard cross, θ azimuthal in `(u, v)`, φ polar from `+w` (§3.7, chart_reference §3.1 and §3.3).
- **Yoshida-6 coefficients** — *settled by R-501:* the coefficients carry their citation (Yoshida 1990, Table 1, solution A; §3.2) and test 1a's convergence-order check; the paper is not added. Was: verify the three w's against Yoshida (1990) Table 1 solution A before they enter the shared source (paper already in the lit set).
- **Reversibility replay** — bounded and deferred per the integrator contract Part 6; nothing here forecloses it. Regularisation is v1 (R-562); Kustaanheimo–Stiefel is not built, Part 2b's four being the regularisations and a later method added by its own ruling (R-607; *was:* "whether Kustaanheimo–Stiefel has a place is RQ-530"). *Was:* "**Reversibility replay & KS regularisation** — bounded and deferred per the integrator contract Part 6; nothing here forecloses either."
<!-- /retired-terms -->

---

*Five occupants, one wrapper, one table-bucketed substep integer. Collision beats escape on a same-time tie (R-30), and both precisions agree on it. Closure and energy together decide escape; the accumulators latch at the end; a sample's Benettin shadow is never itself a sample. Euler exploding is a test passing.*
