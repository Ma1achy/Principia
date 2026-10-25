# Principia — integrator contract

*Sixth doc. The producer side of `SimState` — every categorical debug view, every outcome class, every stability scalar bottoms out here. The physics (KDK/Yoshida/RK4, adaptive substepping, COM projection, invariant monitoring, escape/collision detection) is specified in `principia_dd_integrator.md` §3 (COM projection and invariant monitoring in Part 1 here) and treated as settled. This doc contracts the *seams around* the integrator so it slots into everything else: the swappable step slot, the shared wrapper, parameter ownership, units, and the determinism/reversibility/floor boundaries.*

---

## Part 1 — The shape: a swappable `step()` slot inside a fixed wrapper

Same pattern as the render pipeline's swappable colour/brightness slots. The integrator is **not** a monolith; it is one small swappable piece inside a fixed loop. *(The occupant seam is `ADVANCE(state, t_now, t_target, params)`, Part 2a, R-19. For a stepper with no regularisation, `ADVANCE` is the loop below.)*

```
integrate(ic, params):
    state ← ic                      # (m, r, p) from the decoder
    done ← false                    # terminal flag — the ONLY loop-exit mechanism
    macro ← 0
    while macro < N_macro and not done:              # WRAPPER owns the loop
        N_sub ← substep_bucket(d²(state), params)     # WRAPPER — table lookup, §Part 4 / integrator dd §3.3
        s ← 0
        while s < N_sub and not done:
            state ← STEP(state, dt_macro / N_sub, params)   # OCCUPANT — the only swappable line
            state ← project_com(state)          # WRAPPER — per STEP (Part 1, COM projection)
            update_invariants(state)            # WRAPPER — per STEP, post-projection
            done ← detect_terminal(state, params)   # WRAPPER — per STEP; sets the flag on collision or escape (R-103), never breaks
            s ← s + 1
        expose_state_readout(macro)                     # WRAPPER — live n(t) derived, nothing stored
        macro ← macro + 1
    return SimState
```

**Loop shape is load-bearing for the GPU, not a stylistic choice.** Terminal exit is via a **`done` flag tested in each loop condition — zero `break` statements**, and both loops are `while`, never `for s in 0..N_sub` (a counted loop with a mid-body exit lowers, through rust-gpu → SPIR-V → naga → WGSL, to the multi-level-exit shape that **miscompiles** — wgpu#4449). The spike validated *this exact shape* through SPIR-V→MSL and SPIR-V→WGSL→Tint at 100 macro-steps in one dispatch; `break both loops` (the earlier pseudocode) is the shape that fails. See `principia_gpu_determinism_note.md` for the full loop/indexing discipline.

**The occupant is `ADVANCE(state, t_now, t_target, params) → state'` and nothing else (Part 2a, R-19).** KDK/Yoshida implement `ADVANCE` as the loop above, around their `STEP`; a KDK `STEP` is literally the three-line kick-drift-kick. Everything else — the loop, adaptive substepping, COM projection, invariant monitoring, terminal detection, the state readout — is the **shared wrapper**, identical regardless of which occupant is bound.

**Cadence (pinned).** Projection and invariant accumulation run **after every `STEP` invocation** — per substep, not per macro-step: after the final half-kick in KDK, or after all Yoshida stages. Per-step max-accumulation is the point: it catches close-encounter drift spikes that recover before termination — the regime where Burrau is most interesting. Terminal detection is pinned per `STEP` too: collision must break mid-macro-step (integrating on through `r < r_coll` at near-singular forces for the remainder of a macro-step manufactures `SIM_FAILED` noise, and `r_min` is computed every substep anyway). Escape has no persistence counter (change 11, R-29): its window rule is in Part 7. The shape readout stays on the macro schedule (derived live; nothing stored — lockstep).

### COM projection and invariant monitoring

The decode already quotients out the CoM translation and bulk momentum. Runtime projection keeps floating-point
drift from putting them back: without it the state drifts as $\mathbf r_i = \mathbf r_i^{\mathrm{internal}} + \mathbf R_{\mathrm{drift}}$,
spending f32 dynamic range on physically irrelevant bulk motion. It is housekeeping, not a user-facing feature.

**Projection**, after every `STEP`, with $M$ constant (computed once and cached). Position–momentum form:

$$\mathbf R_{\mathrm{com}} = \frac{1}{M}\sum_i m_i\mathbf r_i, \quad \mathbf P_{\mathrm{com}} = \sum_i \mathbf p_i, \qquad
\mathbf r_i \leftarrow \mathbf r_i - \mathbf R_{\mathrm{com}}, \quad \mathbf p_i \leftarrow \mathbf p_i - \frac{m_i}{M}\mathbf P_{\mathrm{com}}.$$

An occupant that evolves velocities uses $\mathbf V_{\mathrm{com}} = \frac{1}{M}\sum_i m_i\mathbf v_i$ and
$\mathbf v_i \leftarrow \mathbf v_i - \mathbf V_{\mathrm{com}}$. The cost is about 20 FLOPs, negligible next to a force evaluation.

**Why CoM, and only CoM.** CoM position and total linear momentum are linear constraints, generated by exact
translation symmetry, dynamically decoupled from the internal motion, and removable by a canonical projection
with no ambiguity. Energy and $L_z$ are none of these. Restoring either after drift means choosing one of many
state corrections, and none is canonical, so a correction would steer the orbit rather than remove a redundant
mode. **The wrapper never projects energy or angular momentum.** It monitors them.

**Monitoring.** Before the loop, evaluate and store

$$E_0 = \sum_i \frac{\|\mathbf p_i\|^2}{2m_i} - G\sum_{i<j}\frac{m_i m_j}{\|\mathbf r_i - \mathbf r_j\|}, \qquad
L_{z,0} = \sum_i (x_i p_{y,i} - y_i p_{x,i}).$$

After each `STEP`, post-projection, evaluate $E(t)$ and $L_z(t)$ and accumulate the worst deviation so far,
$\Delta E_{\max} \leftarrow \max(\Delta E_{\max}, |E(t) - E_0|)$, and likewise $\Delta L_{z,\max}$. The relative forms divide by a floor,
$\delta_E = (E(t) - E_0)/\max(\varepsilon_E, |E_0|)$ and $\delta_L = (L_z(t) - L_{z,0})/\max(\varepsilon_L, |L_{z,0}|)$, so a
near-zero initial invariant (a rest start) doesn't blow up. The energy evaluation reuses the pairwise distances
from the force computation. The marginal cost is the kinetic term plus two comparisons.

**Per step, not start-and-end.** A close encounter can degrade conservation and then recover before the escaper
leaves. Only per-step accumulation catches that, and it's the regime where Burrau is most interesting. The
*shape* of the drift is the diagnostic: bounded oscillation is usually fine, steady secular drift is a concern,
and a sudden spike points at a close encounter or step-size trouble. The exact accumulators and suspect
predicates are in `principia_dd_integrator.md` §3.4–3.5.

**Under a regularised occupant, two energies, named (R-566).** A regularised occupant carries the energy twice: the
**regularised energy**, read from its regularised state (Heggie's and Aarseth–Zare's from the regularised coordinates
and momenta whose Hamiltonian they march; logH's as its transcription gives it, REQ-INT-104), and the **Cartesian
energy**, `E` above, read from the physical state. After a close passage the Cartesian read-out is ill-conditioned,
280× to 7100× worse than the regularised one on the two-body radial collision, while the regularised energy stays at
round-off (prin-rs `HgOut::drift` against `drift_reg`). So:
- **acceptance, and any per-step suspect flag under a regularised occupant, read the regularised energy:** the two-body
  radial collision gate (`principia_dd_validation_orbits.md` §2, REQ-VAL-045) and `SUSPECT_ENERGY`
  (`principia_dd_integrator.md` §3.5);
- **the Cartesian energy is for diagnostics** (the drift fields, `error_ratio`), **read only from runs that did not stop
  at an event**: a run parked at a collision reads its own cancellation, which produced five wrong conclusions in a row
  in prin-rs.
With regularisation off the two are one, `E` above. **Both drifts are stored (R-605, closing RQ-528):** under a
regularised occupant the regularised energy's drift latches sit in payload fields of their own beside the Cartesian
ones (a schema change; their layout is REQ-PAY-111, a definition). The suspect flags and the drift view read the
regularised drift; the Cartesian drift is diagnostic only, since at a close approach it is a cancellation of two large
terms, and it is wrong exactly where a run stopped at an event. A run that stopped at an event draws in the drift view
as now, its readout saying "stopped at event". The energy-suspect threshold is recalibrated on Heggie × RK4's
regularised drift at the M3 gate (REQ-INT-109), measured first, as before the ruling (REQ-INT-106). The recalibrated
value replaces REQ-INT-085's one threshold, applied at every precision to every occupant, so its proposal also reports
what Euler, RK4, KDK, Yoshida-4 and Yoshida-6 light at it on dd tests 2 and 7 at each precision, and whether
REQ-INT-085's and REQ-VAL-031's pass conditions (Euler lighting `SUSPECT_ENERGY` on test 2) still hold. *Was:* "Where the
regularised drift is stored, and how a drift view draws a run that stopped at an event, is RQ-528."

This factoring is load-bearing: it puts all the shared, determinism-critical machinery in one place, and keeps occupants trivially simple and interchangeable. Adaptive substepping lives in the **wrapper**, not the integrator — the wrapper calls `STEP` `N_sub` times at `dt_macro/N_sub`. RK4-as-validation and the inspector's f64 run are just "different occupant + different precision," no special path.

---

## Part 2 — Occupants and the capability profile

The occupants (the last column gives the *default* binding; the user overrides):

| Occupant | Order | Force evals/step | Symplectic | Step-reversible | Run-reversible | Default tier |
|---|---|---|---|---|---|---|
| Euler | 1 | 1× | ✗ | ✗ | ✗ | — (debug only, below) |
| KDK leapfrog | 2 | 1× | ✓ | ✓ | ✗ | Potato–Low (fast, motion) |
| Yoshida-4 | 4 | 3× | ✓ | ✓ | ✗ | Medium–High |
| Yoshida-6 | 6 | 7× | ✓ | ✓ | ✗ | Ultra–Extreme (accuracy) |
| RK4 | 4 | 4× | ✗ | ✗ | ✗ | Validation cross-check (fixed, non-symplectic) — NB the inspector's *adaptive* high-precision reference is the separate **RK45**, §6, which is not a wrapper occupant |

*Was:* one `Reversible` column, ✗ for Euler and RK4 and "(see §4)" for KDK, Yoshida-4 and Yoshida-6; R-500 (closing
RQ-369) splits it in two, defined below.

**The default occupant is Heggie × RK4 (R-560).** The occupant the user gets unless they choose another is the composed
occupant (stepper × regularisation, Part 2b) Heggie 1974 stepped with RK4: RK4 is what every Heggie result was measured
with (Part 2b). The table's last column is each stepper's default binding by tier when the regularisation is `none`;
the default occupant is Heggie × RK4 at every tier (R-606, closing RQ-529): tiers trade resolution and samples, never
the integrator, since a cheaper unregularised default on the low tiers would change results near collisions. The tier
column is the per-stepper binding with regularisation `none` only; it is revisited, as RQ-529's option (c), once more
of Heggie's pairs are measured (R-561's table). *Was:* "the default occupant is Heggie × RK4 at every tier meanwhile,
and whether a tier keeps another default is RQ-529." *Was:* Yoshida-4's row read "Medium–High (default)".

**The two reversibility flags (R-500).** `step_reversible` says the occupant's `STEP` is time-symmetric at a fixed
step: one `STEP(dt)`, momentum negation, `STEP(dt)`, negation returns the state to rounding level, with no wrapper
projection or substepping (dd_integrator §5, test 3). A symmetric composition is step-reversible whatever the sign of
its weights, so KDK, Yoshida-4 and Yoshida-6 are ✓; Euler, RK4 and Aarseth–Zare + RK4 are ✗. `run_reversible` says the
bound occupant's whole run is time-symmetric, as the reversibility test binds it: only logH's TTL time mode (R-162). For
the regularised occupants (Part 2a, Part 2b) `step_reversible` is measured by dd test 3, not asserted: the test runs
each one's `STEP` and records its residual, and the profile carries what it measured; the time-transformed leapfrogs
(logH's TTL time mode, and Aarseth–Zare + time-transformed leapfrog when built) are expected ✓ and Aarseth–Zare + RK4
✗. What breaks reversal of a run is the wrapper's state-dependent adaptive substepping and its per-step COM projection
(Part 6), not the steppers.

**Occupants are not interchangeable — each advertises a capability profile `{order, force_evals, symplectic, step_reversible, run_reversible}` (*was:* `{order, force_evals, symplectic, reversible}`; R-500), and downstream features read the profile rather than assuming.** This is the new load-bearing idea, the integrator analogue of the chart `system_image` and the link measure. Three consequences:

- **`energy_drift` is only a *dynamics* diagnostic on a symplectic occupant.** On KDK/Yoshida, energy oscillates around `E_0` and drift measures the *physics* (close-encounter stress). On Euler/RK4 it measures the *integrator* — Euler drifts secularly and will light up `SUSPECT_ENERGY` everywhere. The suspect flag therefore means different things per occupant; a debug/stats layer must read `symplectic` before interpreting drift.
- **The reversibility diagnostic needs a `step_reversible` occupant** (Part 6; *was:* "a `reversible` occupant (§4)") — it cannot be wired to whatever happens to be bound. The profile flag is the guard: both of Part 6's routes, the replayed schedule and the fixed step with projection off, gate on `step_reversible` (R-500).
- **Order sets the meaning of the `total_substeps_log2` complexity proxy (derived, payload §5) and FTLE confidence** — a 6th-order step is worth more than a Euler step of the same size.

**What the profile's `order` sets (R-72; REQ-INT-078, a definition the physics reviewer approves; TASK-M3-02).**
`total_substeps_log2 = ⌊log₂ total_substeps⌋` (payload §5) counts `STEP` calls. The profile gives a count its
meaning, in two readings, and `order` sets a third thing, the error exponent; each is read from the profile and none
from an occupant's name:
- *Encounter stress, the complexity proxy proper.* For an occupant with `owns_time_mapping: false` the count is the
  wrapper's substep law (`N_sub` from `r_min`, Part 4), not the occupant's, so on one trajectory it is the same for
  every such occupant and `order` does not change it: it reads as how hard the close encounters were. It is compared
  between samples of one sim key, whose occupant is one (Part 3). For `owns_time_mapping: true` the count is the
  occupant's own schedule (Part 2a, Cost 2), read only against samples of the same occupant. It is never a confidence
  grade: under the substep law more substeps answer a shorter local timescale, not a finer resolution of the same one
  (next item).
- *Cost.* A run made `total_substeps · force_evals` force evaluations, so its cost in `log₂` is
  `total_substeps_log2 + log₂ force_evals`, to within the floor's 1. Effort compares across occupants in this form,
  never as the bare count.
- *The error exponent.* dd_integrator §3.3's law, `N_sub = ⌈(r_sub/r_min)^{γ_sub}⌉` clamped to `[1, N_max]`, at the
  default `γ_sub = 1.5` scales as the two-body time at the closest approach, `τ_local = √(r_min³ / (G m_pair))`,
  `m_pair = mᵢ + mⱼ` the masses of the closest pair, so an unsaturated substep `h_sub = dt_macro / N_sub` holds
  `h_sub / τ_local` near the fixed ratio `κ = dt_macro · √(G m_pair / r_sub³)` (below it by the ceiling's rounding, and
  below it where `N_sub = 1`), however deep the encounter. `κ` is fixed per pair, set by the sim key's `dt_macro` and
  `r_sub` and by the pair's mass: mass is a latent direction of each sample, never an external global (canonical_spec
  §1), so `κ` varies across the samples of one sim key on a chart with a mass axis, and within one unequal-mass
  trajectory takes up to three values, one per closest pair. An order-`p` step's relative error per local dynamical time is then `O(κ^p)` at every
  unsaturated substep: `order` is the exponent the law's fixed ratio is raised to, which is how "a 6th-order step is
  worth more than a Euler step of the same size" (`κ < 1`). It ranks occupants at one `κ` only up to each one's error
  constant; a sim key has one occupant (Part 3), so occupants are compared across sim keys at equal `κ`; it is not an error bound, and no threshold is set on it (a threshold would be a calibration,
  R-71). Where the law saturates (`N_sub = N_max`, the stored `saturated` bit, Part 6) `h_sub / τ_local` exceeds `κ`,
  growing as `(r_sub/r_min)^{3/2} / N_max`, and the exponent no longer bounds the step: that is the integration floor,
  a confidence gradient, not a terminal (Part 6; deep_zoom § "Reserved: the integration floor"). At a `γ_sub` other
  than 1.5 the ratio is not fixed, `h_sub / τ_local ∝ r_min^{γ_sub − 3/2}`, and the exponent compares occupants at
  one `r_min` only. *Was (fix round 1, physics re-check 5607961477 P7: `κ` is fixed per pair, not per sim key, and
  `m_pair` was not defined):* "`τ_local = √(r_min³ / (G m_pair))`, so an unsaturated substep `h_sub = dt_macro / N_sub`
  holds `h_sub / τ_local` near the sim key's fixed ratio `κ = dt_macro · √(G m_pair / r_sub³)` (below it by the
  ceiling's rounding, and below it where `N_sub = 1`), however deep the encounter." and "It ranks occupants at one sim
  key's `κ` only up to each one's error constant;".

**FTLE confidence** is the triple `(symplectic, saturated, order)`. The FTLE (payload §5) is read off the Benettin
shadow, integrated by the same occupant. On a `symplectic: true` occupant the step preserves phase-space volume, so its
error does not bias the exponent in kind; on a `symplectic: false` occupant (Euler, RK4) the step's numerical
dissipation or excitation of phase-space volume enters the exponent at the order of its error, so the FTLE is marked
integrator-biased, and is not compared with a symplectic occupant's. A trajectory whose `saturated` bit is set was
stepped past the ratio the law holds, so its FTLE is computed with low confidence, the integration floor's (Part 6);
an unsaturated one was stepped at `κ`, where `order` grades the occupant. `total_substeps_log2` is not part of it: it
reads encounter stress, and a saturated run has the most substeps. Whether the FTLE has a value at all stays
`ftle_valid`'s (payload §6; R-253, R-254). `kernel::occupant::CapabilityProfile` reads them (`force_evaluations`,
`error_exponent`, `ftle_confidence` given the `saturated` bit), with `drift_reads_dynamics` (`symplectic`) and the two
reversibility gates (`step_reversible`, `run_reversible`). *Was (TASK-M3-02's first draft, physics review 5607707487
P3: under the substep law the extra substeps measure encounter depth, so `ρ` ranked a saturated run, the
lowest-confidence state, highest):* "in three readings" (the third): "*Resolution.* A `STEP` of size `h` of an
order-`p` occupant commits a local error `O(h^{p+1})`, and a run of mean step `h̄` a global error `O(h̄^p)`. With
`step_count` macro-steps taken (payload §5), `h̄ = dt_macro · step_count / total_substeps`, so each doubling of the
steps per macro-step is worth `p` bits of accuracy: 1 for Euler, 2 for KDK, 4 for Yoshida-4 and RK4, 6 for Yoshida-6.
The run's **resolution** is `ρ = order · (total_substeps_log2 − ⌊log₂ step_count⌋)`, in bits, at least 0 (a
difference the two floors invert reads 0) and within `order` of `order · log₂(total_substeps / step_count)`. It is
ordinal: it ranks the runs of one occupant by how much finer than the macro-step they were integrated, and compares
occupants only up to each one's error constant; it is not an error bound, and no threshold is set on it (a threshold
would be a calibration, R-71). Bits past the precision's own (24 at f32, 53 at f64) carry nothing." and "**FTLE
confidence** is the pair `(symplectic, ρ)`. … so its error does not bias the exponent in kind, and `ρ` grades the run;
… so the FTLE is marked integrator-biased whatever its `ρ` … `kernel::occupant::CapabilityProfile` reads all three
(`force_evaluations`, `resolution_bits`, `ftle_confidence`)".

**Euler is kept deliberately, as a debug occupant.** It makes energy drift blow up violently and visibly, which makes "energy-drift view + Euler" a *test that the diagnostic plumbing works* — you *want* to see it explode. It is not a production option; it is the integrator-side analogue of a debug shader. The profile (`symplectic: false`) is what stops it being mistaken for one.

### Part 2c — THE GPU KERNEL: what ports, what is rewritten, and the rules it must hold

**Established by the lowering spike. This is a decision about the shipped kernel, not a note about
prin-rs.**

#### The split

| layer | disposition | why |
|---|---|---|
| **step / deriv / Hamiltonians / `physics/`** | **PORTS AS-IS** | measured **0 `Vec`/`Box`/`dyn`/`std`** across both RK4s, both Hamiltonians and `physics/`. Already shader-shaped. Porting is transcription. |
| **the driver layer** | **REWRITTEN to the rules below** | every blocker found is here, and every one is a *rule violation* rather than a bug |
| **the dispatch shape** | **DESIGNED FRESH** | 72 KB per quad (512 × 144 B, D6) fits no backend; see `principia_systems_architecture.md` §5.5 |

**The good news is real and worth stating: no blocker is a physics problem.** No wrong equations, no
bad numerics. It is all driver-layer plumbing — the cheapest kind of bad news, and what a spike is
for.

#### Rules the new kernel must hold by construction

**1. NO PRECISION-DEPENDENT CONSTANT IN A CONTROL-FLOW DECISION.**

prin-rs gives `f32` and `f64` different `SYNC_EPS` (1e-6 / 1e-15) and `LAND_EPS_REL` (1e-5 / 1e-14),
**both sitting directly in branch conditions.** That is not a GPU problem — **the kernel fails the
requirement between its own two builds**, before any backend is involved.

> A threshold that changes with the `Real` type means the two instantiations are running *different
> logic*, not the same logic at different precision. **That is the one thing the single-kernel design
> exists to prevent.**

If a tolerance must scale with precision, it belongs where the *value* is used, never where the
*branch* is taken.

**2. COMPARISON-ONLY IN BRANCH CONDITIONS.** No runtime transcendental deciding a branch. Frozen
threshold tables (frozen per sim key, generated on the host, R-502), integer counters, `d² < r_coll²` rather than `d < r_coll`. The spike's control —
runtime `ceil((r_sub/√d²)^1.5)` — forked **105 times on Metal and 82 on lavapipe**; the
comparison-only form forked **0 of 705** on both.

**`pow` is what bites; `sqrt` is merely unguaranteed.** With the collision threshold straddled at
0 ULP, the `sqrt`-decided branch forked **0 of 705** on both backends. So squaring the collision test
is a *rule for the new kernel*, not an urgent fix to the old one.

**3. NO BARE `loop {`.** Every march loop needs a bound, for the watchdog and for the dispatch
chunking (`principia_dd_telemetry_and_tiers.md` §6.3–6.4). prin-rs has none.

**4. NO `T::infinity()` SEEDS IN FOLDS.** Reseed min-folds from the first element. 27 sites in
prin-rs, and the related failure — `dt_max` folding from `0.0` so an all-unusable pixel reported its
largest step as zero — is in `principia_01_pitfalls.md` §9.

**5. NO ALLOCATION IN THE HOT PATH.** Already true of the physics layer; must stay true of the
driver.

> **These are RULES, not a fix list.** Filed as *"prin-rs has 46 sqrt-decided branches"* they read as
> a backlog for code that is about to be replaced. Filed as rules, the new kernel does not
> reintroduce them — which is the only outcome that matters.

#### And FMA contraction is a live question

A backend that fuses `a*b+c` changes the result, which changes a comparison, which changes a
decision. **Check whether contraction is controllable per backend**, and whether any branch depends
on it not happening. In *continuous* arithmetic FMA latitude is honest divergence and fine; in a
branch input it is not.

---

### Part 2b — Regularisation is a SECOND swappable axis, not a property of the stepper

**DECIDED. The stepper (RK4, KDK, Yoshida-N) and the REGULARISATION are independent choices, and
both are occupant slots.** Four regularisation occupants, each earning its place for a different
reason:

| occupant | chart | re-registers? | why it is kept |
|---|---|---|---|
| **none** | Cartesian | — | **the INDEPENDENT control.** Shares no coordinate machinery with the others, which is what let it catch the wedge bug. Prior findings, not controls (R-164): the FTLE–drift Spearman correlation is **+0.305** for leapfrog and **−0.082** for AZ, and AZ's is a null against its shifted control (−0.102; prin-rs `NOTES.md:2077`). No gate rests on them. Keep permanently. |
| **Aarseth–Zare** | two pairs, reference body from the longest side | **yes, at every sync boundary** | wins on **sustained hierarchy** (`far`: all 65,536 pixels, 0.7–0.9 decades) — precisely because the reference never has to change |
| **Heggie 1974 (global)** | three relative vectors, symmetric | **no reference body at all** | the general default, stepped with RK4: the default occupant is Heggie × RK4 (R-560; *was:* "the general default."). **31 of 32 cases**, `err>10` (`error_ratio > 10`) 3915 → 74 at prin-rs `8600d45` (the original run at `70cfbc4` gave 3916 → 73; R-165), fixes AZ's worst decile on 100% of pixels. Default time transformation: Eq. 22 at n = 3/2 (R-161) |
| **logH (algorithmic)** | **none whatsoever** | n/a | time transformation only, no coordinate transformation. The *strongest* form of the no-chart property. Its **TTL time mode (Mikkola–Tanikawa)** is the reversible occupant (R-162): `run_reversible`, the only one, and `step_reversible` as dd test 3 measures it (R-500) |

**Why this axis exists at all — the measured finding it came from.** Doubling the sync-boundary
**re-registration count** at *fixed step size* moves the drift field by **0.444 decades**, against
**2.5e-6** for the LC branch choice and **7.5e-5** for the reference-body selection rule. Six
thousand and one hundred and seventy-five thousand times respectively.

> **It is not which chart is chosen. It is how often the state is passed through one.**

That is a property of the **regularisation**, not of the stepper — which is why it needs its own
slot rather than being folded into `STEP`.

#### The equations and the step control (R-160, R-161)

The Heggie, logH and TTL equations of motion, their time transformations and their step control are transcribed into
this Part from the prin-rs reference set (`docs/reference/prin-rs/src/integrate/heggie/`, `.../logh/`; reference, not
authority, R-159), with citations: Heggie 1974; Mikkola & Tanikawa 1999; Preto & Tremaine 1999. The transcription is
physics-reviewed and confirmed at the M3 gate (R-160). Settled already:

- **Heggie's default time transformation is Eq. 22 at n = 3/2:** `dτ = S^{3/2}/(R₁R₂R₃) dt`, the configuration measured
  (R-161). Eq. 20 (`dτ = dt/(R₁R₂R₃)`) stays as a selectable time mode. M3's re-run of the 32-case matrix confirms the
  default.
- **The predictive step limit:** `dτ ≤ f·d_min / (|v_rel|_max·A·B)`, with f = 0.02 (R-160). It is what removed the wedges
  (INDEX, the evidence base).
- **TTL is a prior finding, not an open build:** it is built and validated in prin-rs, and it loses on accuracy,
  monotonically worse with mass ratio (prin-rs `NOTES.md:2573`; R-162).

#### Every stepper × regularisation pair runs; the pairs measured not to work carry a warning (R-561)

**The user chooses the regularisation and the stepper freely, and no pair is forbidden.** The regularisation (`none`,
Aarseth–Zare, Heggie, logH) and the stepper (KDK, Yoshida-4, Yoshida-6, RK4; Euler in the debug selection only, Part 2)
are selected separately, regularisation off included, and every pair runs. **A pair measured not to work carries a
warning, never a block:** beside the occupant selector in the Run window, in the status line (R-615, closing RQ-525:
"the Problems panel" is the status line and the Run window; *was:* "in the Problems panel (which panel is RQ-525)")
and in the settings each export records (its sidecar, render GUI spec § "Export & share"), so a result made with it
says so. The warning states what was measured. A run under such a pair that diverges is undetermined, never discarded
(Part 6). *Was:* "every stepper composes with every regularisation" (REQ-INT-006, REQ-INT-017), with no word on which
compositions work.

**Every pair is defined exactly, so its results reproduce.** With regularisation `none` the Hamiltonian is separable
and each stepper is dd_integrator §3.2's. Aarseth–Zare's `Γ` and Heggie's `Γ*` couple position and momentum (Part 2a
for Aarseth–Zare; for both, prin-rs's integrator registry, `src/integrate/mod.rs`, as the gap sweep's design §1 quotes it,
`docs/incoming/2026-10-21_gap_sweep_integrator/`: "neither Hamiltonian is separable"), so on them a stepper is applied to the regularised equations of motion `(u̇, ṗ, ṫ) = F(u, p)` in fictitious time `τ`,
at the step `h` the step control gives:
- **KDK:** the explicit kick–drift–kick. Kick `p½ = p + (h/2)·ṗ(u, p)`; drift `(u₁, t₁) = (u, t) + h·(u̇, ṫ)(u, p½)`;
  kick `p₁ = p½ + (h/2)·ṗ(u₁, p½)`. Three evaluations of `F`, each at the state as it stands at that stage.
- **Yoshida-4 and Yoshida-6:** the same composition with dd_integrator §3.2's coefficients, the KDK step above taken in
  sequence at `wᵢ·h`. **The order** is dd_integrator §3.2's drift sequence: Yoshida-4 takes the KDK step at `w₁·h`,
  then `w₀·h`, then `w₁·h`; Yoshida-6 at `w₃·h`, `w₂·h`, `w₁·h`, `w₀·h`, `w₁·h`, `w₂·h`, `w₃·h`. **Adjacent
  half-kicks are not merged:** each sub-step is the whole three-evaluation KDK above, so Yoshida-4 takes 9 evaluations
  of `F` per step and Yoshida-6 21. dd_integrator §3.2 prints the merged kick sequence (`cᵢ = (d_{i−1} + dᵢ)/2`), which
  equals the KDK sequence only on a separable Hamiltonian; here the trailing kick of one sub-step is evaluated at
  `(uᵢ, p½)` and the leading kick of the next at `(uᵢ, pᵢ)`, so merging them is a different method (applied per R-369,
  physics review 5605806092 of PR #277, F5).
- **RK4 and Euler:** their classical forms on the whole state `(u, p, t)`.

Under logH, whose time transformation is built for a leapfrog, each pair's exact form is written with logH's equations
and checked as the others are (REQ-INT-097). **TASK-M3-39 writes them here**, each pair in each of logH's time modes, from
logH's equations once TASK-M3-08 has transcribed them (R-160, REQ-INT-084, which covers logH's own equations, time
transformations and step control, not its pairs), physics-reviewed before merge (applied per R-369, qa review 5607548278
of PR #277, Q1). *Was:* "when they are transcribed (R-160, REQ-INT-084)", which named no writer for the pair forms.

The explicit composition is not symplectic on a non-separable Hamiltonian, so the composed occupant's profile is the
pair's, not the stepper's row in Part 2: `symplectic` false for these pairs, and `step_reversible` dd test 3's
measurement (R-500). logH's own time modes, logH and its TTL leapfrog, are unchanged (above). A symplectic regularised
occupant, if one is wanted, is an implicit stepper and gets its own row in the table below.

**Which pairs work, as measured (R-561).** Each pair is measured good, measured bad or not measured; only a measured-bad
pair carries the warning. A status changes only by a measurement, recorded here with its evidence; the criterion for
"measured not to work" is a definition (R-72, REQ-INT-099).

**One cell per pair and time transformation.** A regularisation with more than one time transformation (Heggie's Eq. 20
and Eq. 22, R-161; logH's logH and TTL modes, R-162) has a row for each, since each is selectable and is measured on its
own; a pair's status, and its warning, are the cell's for the time transformation in force, and no rule combines
verdicts across them. The design measured Heggie's pairs under both transformations with the same verdict, so the two
Heggie rows agree (applied per R-369, qa review 5607548278 of PR #277, Q2; *was:* one row per regularisation, "Heggie"
and "logH" a row each).

| regularisation (time transformation) \ stepper | Euler (debug) | KDK | Yoshida-4 | Yoshida-6 | RK4 |
|---|---|---|---|---|---|
| `none` | not measured | not measured | not measured | not measured | not measured |
| Aarseth–Zare | not measured | not measured | not measured | not measured | **measured good** (1) |
| Heggie (Eq. 20) | not measured | **measured bad** (2) | not measured | not measured | **measured good** (3) |
| Heggie (Eq. 22, the default) | not measured | **measured bad** (2) | not measured | not measured | **measured good, the default** (3) |
| logH (logH mode) | not measured | not measured | not measured | not measured | not measured |
| logH (TTL mode) | not measured | not measured | not measured | not measured | not measured |

1. Aarseth–Zare + RK4 passes through exact collision at `d_min = 1.35e-11` with energy drift `6.2e-15` (Part 2a); the
   32-case matrix ran it and Heggie under the same RK4 and step control (the comparison discipline, below). This status
   is the port's, applied per R-369, not the human's: the gap sweep measured only Heggie's pairs, and the collision
   passage is a different measurement from REQ-INT-099's criterion, so REQ-INT-098's gate re-measures it by that
   criterion (physics review 5605806092 of PR #277, F7).
2. Heggie × KDK, the explicit composition above, on the figure-eight at a fixed fictitious step with no step control,
   under both of Heggie's time transformations (Eq. 20 and Eq. 22): order 0.8–1.2 at 3.0 calls per step, and the
   regularised energy diverges within 100 orbits (to 1e78 and beyond). Its warning reads: "Heggie's Hamiltonian is not
   separable, so a leapfrog step is first order here and can diverge (figure-eight: order about 1, diverged within 100
   orbits)."
3. Heggie × RK4, the same fixture, under both of Heggie's time transformations (Eq. 20 and Eq. 22; the design §1's
   table, "Tested today on the figure-eight, fixed fictitious step, no step control, both of Heggie's time
   transformations"): order 3.99 at 4.0 calls per step, the regularised energy bounded, 5e-7 to 3e-6, over 100 orbits;
   31 of 32 gallery cases (row above). *Was:* "Heggie × RK4, the same fixture: order 3.99 …", which named no time
   transformation (qa re-check 5607645188 of PR #277).
4. Heggie × the generalised (Störmer–Verlet) leapfrog, **measured bad** (R-567), the same fixture: order 2.0 over one
   orbit at 15–20 calls per step, growing without bound over 100. **Cause, measured:** it does not keep Heggie's
   Eq. (9), `Σ qᵢ = 0`, which is quadratic in the regularised positions (`qᵢ = Qᵢ²`); implicit midpoint keeps every
   quadratic invariant exactly, while a partitioned method like Störmer–Verlet keeps only those mixing positions with
   momenta (Hairer, Lubich & Wanner, ch. IV.2). Implicit midpoint holds the residual at 1e-15–1e-13; the leapfrog's
   grows as h² per orbit, and the energy breaks once it reaches about 1.8e-2, in every run, at every step size, under
   both time transformations. The control, implicit midpoint started deliberately off the constraint, stays bounded at
   1e-3 and 1e-2, and at 3e-2 holds under one time transformation and diverges under the other. Smaller steps break
   later (2.2, 4.4, 8.8 orbits). Its warning reads: "does not keep Heggie's Eq. (9); the energy breaks once its
   residual reaches ~2e-2". The generalised leapfrog is not a column of the table above, being none of Part 2's
   steppers, and neither it nor the implicit midpoint rule is a selectable stepper or built (R-609, closing RQ-532):
   the pair is recorded here as measured bad, for the record, and "stays allowed" means that no pair of the built
   steppers is blocked (*was:* "whether it, and the implicit midpoint rule, become selectable steppers is RQ-532, and
   meanwhile neither is built") (prin-rs `examples/heggie_stepper_probe.rs`, its header and its third section, the causal arm; R-567).

The `none` pairs are separable, so each stepper's own form applies; they are not measured as pairs here. Two implicit
steppers were measured under Heggie on the same fixture and are not occupants: the implicit midpoint rule (order 2.00,
9–10 calls per step, energy bounded at 2e-4, symplectic, about 400× RK4's error at the same step) and the generalised
Störmer–Verlet leapfrog (measured bad, note 4: it does not keep Heggie's Eq. (9); R-567). Neither is a selectable
stepper (R-609; *was:* "Whether either becomes a selectable stepper is RQ-532."). Measured in prin-rs, examples `heggie_stepper_probe` (its PR #34; the gap sweep of 21 Oct
2026, R-561; the causal arm, R-567). *Was:* "(order 2.0 over one orbit, 15–20 calls per step, growing without bound
over 100 orbits and worse at smaller steps, unexplained and probably the probe's implementation; nothing rests on it)";
"worse at smaller steps" was a misread, the maximum taken over runs that had already diverged (R-567).

#### The profile gains a field

`re_registrations: u32` — or at minimum a per-trajectory count in the payload. **This is the
quantity the entire wedge investigation turned on and it is currently invisible.** A method that
re-registers is structurally different from one that does not, and downstream must be able to read
that rather than know it as folklore.

#### The comparison discipline is part of the seam, NOT a convention

With four occupants shippable, someone will eventually compare two of them under different
steppers and get a confident wrong answer. **The harness must refuse mismatched arms**, because a
convention is exactly what failed in the `refine_flagged` propagation.

- **Same stepper, same step control, across every arm.** Comparing a GBS logH against an RK4
  Heggie scores the *integrator*, not the *regularisation*. The AZ/Heggie result is trustworthy
  precisely *because* both ran the same RK4 under the same step control.
- **Fixed colour-ramp constants shared across arms.** An auto-ranged ramp per panel *manufactures
  or hides* the difference it is meant to show — measured, not hypothesised.
- **Every arm prints its full provenance**, including the occupant and its profile.

#### One thing deliberately NOT decided

**Automatic selection between occupants.** If AZ wins on sustained hierarchy and Heggie otherwise,
a selector would need a boundary — and *an imposed choice with a boundary is the exact defect class
that produced the wedges*. The current position: **expose the choice, report which would have been
better, never switch silently.** Revisit only with a measurement showing the boundary is stable.

---

### Part 2a — Widening the slot: `owns_time_mapping`, and the `advance` signature

**DECIDED (change 8, extended by Part 2b; R-19), driven by measurement.** The wrapper's model — `STEP(state, dt)` called `N_sub`
times at `dt_macro/N_sub`, with adaptive substepping owned by the wrapper — was measured **failing on
the regime this instrument exists to explore.** Unregularised adaptive stepping on `deep interior`
did not hang; it returned `|dE/E| = 2.8e+02` in 35k steps against a 2e6 budget. **A wrong number
that looks like a right one**, which is worse than exhaustion.

**Regularisation is the fix, and it does not fit the current slot.** Aarseth–Zare carries its own
time transformation (`dt = |R1||R2| dtau`) and integrates in **fictitious time**, so the occupant —
not the wrapper — controls the mapping to physical time. Validated: passage through exact collision
at `d_min = 1.35e-11` with energy drift **6.2e-15**, where every unregularised alternative fails
(see `principia_dd_refinement_criterion.md` §7.7–7.8).

**The change is one signature and one flag:**

```
STEP(state, dt, params)                   -> state'    # the earlier seam
ADVANCE(state, t_now, t_target, params)   -> state'    # the occupant seam (R-19)
```

The seam moves from *"advance by `dt`"* to *"advance to `t`"*. **The wrapper owns the target; the
occupant owns how it gets there.** KDK/Yoshida implement `ADVANCE` as the loop they already run — a
rename, no behaviour change. AZ implements it by stepping in `tau` until it lands on `t_target`.

The profile gains **`owns_time_mapping: bool`** — the honest name, because the question is not "is it
regularised" but **"does the wrapper still know how far a step goes"**. It is reported for the **composed occupant**
(stepper × regularisation, Part 2b): `true` whenever the regularisation is not `none` (R-19).

**Why widen rather than hold AZ outside the occupant system** (as RK45 is held outside): RK45 sits
outside because it is a *reference* — rare, inspector-only, validation. **AZ is the opposite: it is
what makes the ordinary case work.** Putting the mechanism that handles close encounters outside the
system that handles integration makes the main path the exception, and that leaks.

#### What this preserves, and what it costs

**Preserved — the canonicalise seam.** The integrator still takes `(m, r, p)` and never learns the
chart's name. This change does not come near it.

**Preserved — payload purity, and it gets *cleaner*.** The guarantee is *"`SimState` is a pure
function of `(IC, sim key, playhead t)` — identical whether reached by continuous marching, catch-up,
or re-boot after eviction"*. That is a statement about **targets**, not steps. An `advance-to-t`
signature says the same thing in its own types. Caching, re-addressing under in-plane pan and zoom (R-92) and live-to-live
handoff are unaffected. For a regularised occupant the three reach the same state **bitwise**, because its march stops
only on the fixed sync grid (R-565, below).

**Preserved — determinism**, with the count-bound relocated: *"fixed `dt`, deterministic schedule,
count-bound never wall-clock"* becomes *"fixed `tau`-schedule, count-bound never wall-clock"*. Same
guarantee; the occupant declares its own schedule.

**Preserved — the refinement machinery**, which compares reductions at a shared playhead. The
playhead is exactly what the new signature makes explicit.

**Cost 1 — the per-substep cadence is a callback passed in (R-19).** Projection, invariant accumulation and
terminal detection currently run in the wrapper *after* every `STEP`. Under `ADVANCE` the occupant
must invoke them, so **pass them in rather than doing them after** — otherwise an occupant can
silently skip them and nothing catches it. That cadence is load-bearing: it is what catches
close-encounter drift spikes that recover before termination, which is the regime Burrau lives in.

**Cost 2 — the wrapper can no longer verify `N_sub`.** It must trust the occupant's reported
`total_substeps`. That number is not cosmetic: measured across 68,685 leaves it is **bimodal with a
100× p1→p99 range** (`principia_dd_tree_dump_analysis.md` §7), and cost-aware priority depends on it
meaning the same thing across occupants. **Assert the occupant reports it honestly; do not assume.**

**Caching:** the compatibility signature already includes *"integrator occupant + config"*, so a
different occupant already invalidates payloads — correct as-is. Confirm `owns_time_mapping` is part
of the config that is hashed.

#### The sync grid is a fixed quantum of physical time (R-565)

A regularised occupant lands on **sync boundaries**: the times its march stops at to land, sample and, for
Aarseth–Zare, re-register (Part 2b). **The sync grid is a fixed quantum of physical time, `Δ_sync`, the same for every
run, independent of the horizon `T` and of frame timing.** The boundaries are `t = k·Δ_sync`, never `t_max / n_sync`.
*Was:* the grid was `t_max / n_sync` (REQ-INT-080's "the number of sync boundaries `n_sync` over the horizon"; prin-rs
`EnsembleCfg`), so a run at another horizon was another discretisation.
- **Steps are sized from `Δ_sync`**, never from the target a call asks for: the step control (Part 2b, R-160) sizes each
  interval's steps from the interval `Δ_sync`, and Aarseth–Zare's `dt_left` (REQ-INT-080) is `Δ_sync`.
- **The march stops only on the grid.** `ADVANCE(state, t_now, t_target, params)` advances to the latest boundary at or
  before `t_target`; it never stops between boundaries. So live march, catch-up and re-boot after eviction reach the
  same state at the same grid time, bitwise. The rule governs where `ADVANCE` returns for a copy still marching: a
  terminal event (a collision, a non-finite state, the step budget run out, Part 6 and Part 7) still latches at the
  step it fires on, as before, and the copy's march ends there, between boundaries or not; escape is sampled at the
  boundaries (Part 7). Applied per R-369 (physics review 5605806092 of PR #277, F8).
- **The playhead reads boundary states:** what is shown at playhead `t` is the state at the latest boundary at or
  before `t` (scheduler contract Part 7).
- **Time windows are stated in physical time**, never in boundaries: the escape closure window is 0.4 time units (Part
  7, R-29), sampled at the boundaries inside it.
- **The standing rule** "a diagnostic must reuse the original discretisation" (pitfalls §3; `n_sync` fixed while
  `t_max` varies compares different discretisations) **holds by construction**: there is one grid.
- **`Δ_sync` is a calibration value** (R-71, REQ-INT-080), a constant of the build, read from the constants register
  as `r_pole` is, so a change re-integrates (applied per R-369). Measured under Heggie with the physical step held fixed
  and the closure window held fixed in time, against the production grid (`t_max / n_sync = 13/32`): a grid 4× finer
  (0.10) costs 1–3% more steps, 16× (0.025) 17–25%, 32× (0.0125, about a display frame) 40–52%; the shape changes by
  about 1e-11 to 1e-10 (about 1e-6 on `far`), and 0, at most 1 and at most 4 outcome labels of 1024 flip (prin-rs
  `sync_grid_probe`, PR #34). Heggie never re-registers, so its sensitivity to the boundary cadence is about 10×
  smaller than Aarseth–Zare's.

R-565 is the sync grid only (R-602, closing RQ-524): the unregularised occupants' macro-step grid, `dt_macro =
max(1e-3, T/65535)` (Part 3, R-132), which depends on `T` above `T = 65.535`, stays R-132's, unchanged. *Was:* "Whether
the unregularised occupants' macro-step grid, `dt_macro = max(1e-3, T/65535)` (Part 3, R-132), which depends on `T`
above `T = 65.535`, follows the same rule is RQ-524."

#### The table gains two rows

| Occupant | Order | Force evals/step | Symplectic | Step-reversible | Run-reversible | Owns time mapping | Default tier |
|---|---|---|---|---|---|---|---|
| **Aarseth–Zare + RK4** | 4 | 4× | ✗ | ✗ | ✗ | ✓ | close-encounter regimes; **validation-grade stepper** |
| **Aarseth–Zare + time-transformed leapfrog** | 2 | 1× | ✓ | ✓ (measured, dd test 3) | ✗ | ✓ | **the entry the table wants**; an allowed future occupant, not a v1 requirement (R-168) |

*Was:* one `Reversible` column, ✗ and ✓; R-500 splits it (Part 2). logH's TTL time mode (Part 2b) is step-reversible
(measured, dd test 3) and the one run-reversible occupant (R-162, R-500).

**AZ is currently stepped with RK4, which was chosen to prove the physics rather than to ship** — it
is neither symplectic nor reversible, so binding it makes the regularised default a *validation*
stepper. **Time-transformed leapfrog (Mikkola–Tanikawa)** gives regularisation at leapfrog cost while
staying symplectic and step-reversible (R-500; *was:* "and reversible, and it is what unblocks the reversibility
diagnostic (§4)"). The run-reversible occupant the reversibility test binds is logH's TTL time mode (R-162).

**Specify the slot now even if RK4 fills it initially**, so the profile advertises `symplectic:
false` loudly and nothing downstream quietly assumes otherwise. Note `Gamma` is **not separable**
(the `|u|^2 |p_lambda|^2` term couples position and momentum), so plain KDK does not apply to AZ. *Was: "the
Mikkola–Tanikawa form is required, not optional."* The pair is still allowed, defined as Part 2b's explicit composition
and warned of if it is measured not to work (R-561, Part 2b § "Every stepper × regularisation pair runs; the pairs
measured not to work carry a warning (R-561)"). **A reversible occupant exists: logH's TTL time mode
(Mikkola–Tanikawa), as prin-rs built it; Aarseth–Zare with Mikkola–Tanikawa isn't required (R-162).**

---

**Which occupant is bound is on the sim key** (Part 3). Switching integrator **re-integrates** — unlike switching a render mode, which never does. This is the one-line rule that keeps the CPU/GPU divergence honest: same occupant both sides, precision the only difference **in continuous values** (branch decisions still match on identical inputs, per step, by the comparison-only rule — Part 4; labels on chaotic trajectories may differ across precisions, R-84).

---

## Part 3 — Parameter ownership (who owns what)

"What does the integrator need" is really "who owns each parameter." Three layers, three owners — an agent that passes the wrong parameter to the wrong layer breaks this.

**1. Per-sample, from the decoder — the IC.** `(m, r, p)`. Different every pixel. *Not* a uniform; it comes from `x₀ + J_D·δ` (deep zoom) or the full decode. The occupant reads only this plus `dt` and `G`.

**2. Wrapper config — the loop.** Constant across a dispatch, bound once per frame/tier as `SimUniforms`. **On the sim key** — change any and it re-integrates.

| Parameter | Default | Role | Owner |
|---|---|---|---|
| `T_horizon` | default 50, raised or lowered: ∈ [0.4, 200] (R-132, R-575, R-610): the floor one closure window, 0.4, below which the escape arm cannot fire, the cap 200 until long horizons are measured for cost (*was:* "∈ (0, 200] meanwhile (R-132, R-575; its range RQ-555)"; *was:* "∈ [50, 200], default 50 (R-132)") | when to stop (physical time, §5) | wrapper loop bound |
| `dt_macro` | `max(1e-3, T/65535)` (**fixed** for a given `T`, §5; R-132) — so `⌈T/dt_macro⌉ ≤ 65535` and the u16 step indices stay exact (R-86) | base macro step | wrapper loop bound |
| `N_max` | tunable (default 64), at most the threshold table's maximum length (a calibration value, R-502), refused above it | substep cap per macro-step — **user-adjustable** (raising it resolves more close-encounter structure before the integration floor; saturation is the stored sticky `saturated` bit, set when `N_sub == N_max` occurs — payload §2; the exact cumulative work lives in the `total_substeps` u32). The substepper also exposes the **live current `N_sub`** per macro-step (an already-computed intermediate) for the animated effort view — render Part 5 | wrapper substepper |
| `r_sub`, `gamma_sub` | 0.05, 1.5 | produce `N_sub` from `r_min`: with `N_max` they generate the threshold table on the host, per sim key, shipped in `SimUniforms` (Part 4, R-502) | wrapper substepper |
| `r_coll` | — | collision threshold — **user-exposed sim-key** (Part 7, collision: definitional regularisation choice, not precision) | wrapper detector |
| `tau`, escape window | —, 0.4 (provisional) | escape settling threshold (not tuned; to re-measure) and the window `|Δn̂|` is taken over, in time units, sampled at macro-step boundaries (unregularised occupants) or sync boundaries (regularised ones, on the fixed grid `Δ_sync`, R-565) (R-29, R-95) | wrapper detector |
| `Δ_sync` | a calibration value (R-71, REQ-INT-080); 0.1 costs about 2% more steps than the production grid, 0.0125 about 50% (Part 2a) | the sync grid of the regularised occupants: a fixed quantum of physical time, the same for every run, independent of `T` and of frame timing; steps are sized from it and the march stops only on it (R-565, Part 2a § "The sync grid is a fixed quantum of physical time (R-565)") | wrapper loop bound (a constant of the build) |
| `step_budget_rate` | a calibration value starting at about 4e4 steps per unit physical time per copy (R-71, R-564, REQ-INT-103) | a regularised occupant's step budget, a rate: steps per unit physical time, per ensemble copy, so it is defined for a live playhead with no fixed end; running out makes the copy undetermined (Part 6, R-563). Accounted every step (R-604, closing RQ-527): a copy's running total of steps is checked at every step against `max(N_min, step_budget_rate · t_elapsed)`, `t_elapsed` its physical time since `t = 0`, and the copy is exhausted when it exceeds it (*was:* "How the rate is accounted along a run is RQ-527") | wrapper loop bound |
| `N_min` | a calibration value (R-71, R-604, REQ-INT-108), confirmed at the M3 gate | the step budget's floor: the allowance below which the budget never falls, so that an early close encounter is not cut before `step_budget_rate · t_elapsed` has grown | wrapper loop bound |
| `TINY` | a calibration value, one per working precision (R-71, REQ-INT-105); in neither the corpus nor the reference set | the floor of the regularised occupants' time-transformation denominators: Heggie's `S` and `dt/dτ` (its `Rᵢ` are tested against it, not floored; *was:* "Heggie's `Rᵢ`, `S` and `dt/dτ`"), Aarseth–Zare's `A` and `B`, logH's drift denominator; a step taken at the floor is a counted, non-terminal signal (Part 6, R-563) | occupant (regularisation) |
| `r_close` | 0.01 | close-encounter counting | wrapper detector |
| `r_pole` | 0.01 (R-433; the constants register's `r_pole`, provisional until M3, REQ-INT-086) | the pole disc of the unwrapped phase `θ̃` (dd_integrator §3.7): it decides the stored `θ̃`, `_reserved`, `orbit_count` and `retrograde` for the same IC, so it is on the sim key whatever form TASK-M3-11 gives it, and a change at the M3 gate re-integrates (applied per R-369, PR #210). It is not hashed (dd_generation_root §3.8 "The hash"): it changes values, not what the bits mean | wrapper march |
| `eps_E`, `eps_L` | 10⁻⁶ | relative-drift floors (rest starts) | wrapper monitor |
| `G`, `M_total` | 1, 1 | constants | wrapper |

**3. Occupant + tier flags.** Which `STEP` is bound, plus the co-computation selection (`FTLE_ENABLED` — the forward-pass shadow rides the wrapper). These select **baked variants** (lowering Part 3 — per-thread state costs occupancy even branched off), never runtime uniforms. Also sim key. The ensemble is **not** a baked variant: `copy_index` is a uniform, and each copy is the same kernel dispatched again (R-102, R-89).

The occupant needs almost nothing — `(m,r,p)`, `dt`, `G`. Horizon, substepping, and termination are **wrapper** responsibilities, because adaptive substepping is a wrapper concern: it's what turns one fixed `dt_macro` into `N_sub` calls of `STEP`.

---

## Part 4 — Determinism, and the substep as the subtle seam

Core-design axiom 1: physics defined once, compiled twice — *structurally*, one Rust source — so the f32/f64 gap is *only* precision **for continuous values** (branch decisions must match on identical inputs, below) and the CPU/GPU divergence is honest. Making the occupant swappable splits the determinism contract cleanly:

- **The `STEP` is the occupant's business** — its arithmetic differs by precision across CPU/GPU, and that difference is the honest divergence the inspector witnesses.
- **The wrapper must be bitwise-consistent across CPU/GPU regardless of occupant** — because it decides *control flow*, not just values.

The trap, and it is the single highest-value thing in this doc: **`N_sub = min(N_max, max(1, ⌈(r_sub/r_min)^γ_sub⌉))` is a branch decision, and a branch that forks across backends is a divergence source that is neither precision nor chaos — it is two `pow` implementations disagreeing, and it makes the CPU sanity-check a *different discretisation* than the GPU ran.** The naïve reading — "compute it in f32 on both sides so the integers match" — **does not survive real compilers** (spike, `principia_spike_brief.md`): the culprit is not fast-math or FMA but *inherent cross-implementation transcendental latitude* (GPU `pow` vs libm `powf` differ 1–3 ulp at integer boundaries, in every math mode), so no runtime `pow`/`sqrt`/`div` feeding a `ceil` can be made backend-stable by matched rounding.

**Rule (the determinism pin, replacing the f32-evaluation framing — full spec in integrator dd §3.3): a branch decision may depend only on comparisons against compile-time constants, or against values every backend receives bit-identically (the sim key's threshold table, R-502), and single-rounded arithmetic — never a runtime transcendental.** `N_sub` is a **bucket lookup**: `d²` compared against the sim key's f32 thresholds `THR[n] = round_f32((r_sub·n^{−1/γ_sub})²)`, `(r_sub/n^{2/3})²` at the default `γ_sub = 1.5`, via a generated comparison tree — no runtime sqrt/div/pow/loop/dynamic index. The thresholds are generated per sim key on the host from its `r_sub`, `γ_sub` and `N_max`, shipped in `SimUniforms` as exact f32 bit patterns and read by constant index; the host computes the `pow` with a pinned pure-software libm, so the native and the wasm hosts give identical bits; `N_max` above the table's maximum length, a calibration value, is refused (R-502, closing RQ-370; dd §3.3). *Was:* "compared against 64 build-time-frozen f32 thresholds `THR[n] = round_f32((r_sub/n^{2/3})²)` … The `pow` moves to build time (into the constants)". The `pow` leaves the device (into the host's table); the sole float feeding the sole branch is `d²`, built **position-quantised** (f32 positions, lone-sub, then only operations every IEEE backend rounds alike, with no intermediate that can be subnormal; R-547, *was* "lone-mul, explicit `fma`") so it too is bit-identical everywhere. Clamp in f32 *before* the float→int cast (out-of-range `OpConvertFToU` is GPU UB). This is the *same* law as the read-side NaN/predicate rule (render Part 4, lowering Part 3a): **a control-flow decision is frozen at build time or reduced to a comparison — never left hostage to a floating-point op the backend computes its own way** (`principia_gpu_determinism_note.md`). Same discipline for the terminal comparisons whose control flow must not diverge:

- Escape comparisons (`|Δn̂| < tau`, `E_rel > 0`; R-29) — shared branches whose inputs are multi-op arithmetic. The old persistence counter that absorbed single-step flips is gone (change 11), so nothing forgives a rounding flip near threshold; these inputs take rule 6's treatment (R-34).
- `SIM_FAILED` raising, the horizon-reached branch (`t ≥ T` ⇒ `bounded` — there is no separate timeout state, payload §2), and the **substep-cap decision** (`N_sub == N_max`) — shared branches. The cap decision leads to *advance with the best-available (under-resolved) step*, not a terminal; the sticky **`saturated`** bit (descriptor bit 5 — a STORED flag set at the point the cap fires, payload §2; the old peak-`substep_log2`-then-derive mechanism is gone) records that it happened. The capped step is computed identically both sides (count-bound, deterministic — never wall-clock), so parity holds through the saturation, not just up to it. **`N_max` is a sim-key tunable** — both pipelines must use the same value (as with `r_coll`), so the shared-branch decision stays bit-identical.

State the boundary explicitly in code: values may diverge by precision; **branch decisions in the wrapper may not, on identical inputs** — each step's decision is identical given that step's inputs (`principia_parity_contract.md` Tier L, R-84). Along a chaotic trajectory the inputs themselves diverge by precision, so its labels may differ across precisions; that is reported, not suppressed.

---

## Part 5 — Units and the horizon (inherited scale gauge)

**Everything runs in one fixed dimensionless N-body unit system: `M = Σmᵢ = 1`, `G = 1`, `I = 1`.** This is not a new choice — `I = 1` is the *same* scale gauge as the config decoder's `R̃ = 1` (chart contract Part 1). The integrator **inherits** the scale gauge; it does not impose one.

Consequence, worth stating so an agent doesn't add a per-IC time rescale: because the similarity symmetry `r → λr, t → λ^{3/2}t` is gauged out (scale fixed to 1), a **single global horizon `T` is legitimate across every pixel**. Two different-scale ICs would need different physical `T` to reach the same dynamical state — but there are no different-scale ICs; scale is fixed. That is *why* one `T` works for the whole map.

- **`T` is physical time in these units, not a step count.** Default 50, and it can be raised or lowered (R-575, amending R-132): `T ∈ [0.4, 200]` (R-610, closing RQ-555): the floor is one closure window, 0.4 (R-29), below which the escape arm cannot fire, and the settings say so; the cap stays at 200 until long horizons are measured for cost. *Was:* "`T ∈ (0, 200]` meanwhile, the lowest value and the cap being RQ-555". Tens to a couple hundred dynamical (crossing) times, the window in which Burrau resolves into binary + escaper, at the default and above. *Was:* "`T ∈ [50, 200]`, default 50 (R-132)".
- **The macro-step schedule length is derived and deterministic**: `⌈T / dt_macro⌉` macro-steps (× `N_sub` near encounters) — 5×10⁴ at the default `T = 50`, and never more than 65535, since `dt_macro = max(1e-3, T/65535)` (R-132). With `dt_macro` fixed for a given `T`, this length is identical CPU/GPU — **this is the schedule the reversibility work replays** (§ below).
- **CoM re-projection is part of the deterministic step**: each step is *integrate → re-project to CoM* (removes accumulated centre-of-mass / total-linear-momentum drift). The projection is on the parity surface — CPU and GPU must project with identical arithmetic and order. It manages CoM/linear-momentum only; energy and `L_z` remain genuine conservation diagnostics. The integrator works in **CoM-frame particle coordinates** (3 bodies); Jacobi is the chart/IC representation, converted to particles once at decode.
- **Exact step-index timing invariant**: `t_end_step` stores `step_count` (the completed-macro-step count — running: current, terminal: latched, initial: 0; payload §2), as an exact u16. **Requires `horizon_steps = ⌈T/dt_macro⌉ ≤ 65535`, ENFORCED at dispatch** (single format, no Q0.16 fallback); over-limit `(T, dt_macro)` is rejected — use coarser `dt_macro` or march epochs. Also assert the joint `horizon_steps × N_max ≤ 2³²−1` for the exact `total_substeps` counter.
- **t=0 terminals are real outcomes, not decode failures**: at dispatch, before the first step, evaluate the terminal detectors on the decoded IC. A valid IC already inside `r_coll` → `state=collision, t_end_step=0`; escape has no t = 0 case, because its settling test needs a window of history (R-29), so an IC that is already escaping is classified when its window completes (RQ-19). `decode_failed` is reserved for the decoder failing to produce a valid physical IC (non-finite / degenerate / invalid mass), never a dynamical t=0 terminal.
- **Branch-cut crossing detection (free-group word, payload §3)**: run detection at **every accepted adaptive substep** (not per-macro-step) — the word mutation is event-only/cold, but detection must be substep-granular to catch multiple/recrossing events. Order simultaneous crossings **chronologically by crossing fraction** `τ = d₀/(d₀−d₁)` (sort ascending); fixed cut-ID priority is a **tie-break only** when `τ` values are equal within tolerance. **Endpoint-on-cut: half-open sign convention** — count iff `negative → non-negative`; fixed and identical CPU/GPU. These branch decisions are Tier-L/Tier-B (parity contract): word content is only as deterministic as they are.
- **FTLE partial-interval finalisation (payload §5)**: the Benettin sum `S` accumulates only at renormalisation boundaries. At read/terminalisation, **finalise the partial interval** — `S_final = S + log(δ_current/δ₀)`, then `ftle = S_final / (step_count · dt_macro)` (divide by full elapsed). Do NOT divide the boundary-only `S` by full elapsed time (systematically too small). `completed_renorms = step_count / renorm_interval_steps` (derived; uniform schedule).
- **`t_end` (SimState termination time) is in the same units**, so it is directly comparable across pixels, quads, and charts. This is what makes the Lehto decay-time map a straight render of `t_end` — the unit system is what lets "decay time" be a chart at all.

---

## Part 6 — Three boundaries stated honestly (deferred work lands here)

**Reversibility.** The shared Lehto/Trani diagnostic (validation doc) needs a genuinely time-symmetric integrator, and **the current stack is symplectic but not cleanly reversible as a run**: state-dependent adaptive substepping (the substep sequence isn't symmetric under reversal) and per-step COM projection (a non-symplectic nudge) break naive "integrate → negate → integrate." The symmetric steppers (KDK, Yoshida-4, Yoshida-6) do not: a symmetric composition with a negative weight (Yoshida-4's `w₀`, Yoshida-6's `w₁`) is still reversible at a fixed step, which dd_integrator §5 test 3 checks (R-500, closing RQ-369). *Was:* "Yoshida's `w₂ < 0` middle step, state-dependent adaptive substepping …, and per-step COM projection … all break naive …" (R-57 fixed the label; R-500 withdraws the claim). Not a bug for the *survey* (symplecticity is what matters there). But the reversibility diagnostic must either (a) record the macro-step + substep schedule and replay it exactly in reverse — feasible precisely because the schedule is deterministic (§5) — or (b) bind a fixed-step reversible occupant with COM projection disabled. Both routes gate on the `step_reversible` capability flag (§2; *was:* "The `reversible` capability flag (§2) gates which occupants qualify"); `run_reversible` marks the one occupant whose whole run is time-symmetric, logH's TTL time mode (R-162, R-500). Stated now so the deferred work doesn't discover it late.

**Regularisation and the integration floor.** **Regularisation is v1** (R-562): the regularisation occupants of Part 2b (`none`, Aarseth–Zare, Heggie, logH) ship, and Heggie × RK4 is the default (R-560). Kustaanheimo–Stiefel (KS) is not one of them, and is not built (R-607, closing RQ-530): Part 2b's four are the
regularisations, and a later method is added by its own ruling. *Was:* "whether it has a place is RQ-530". **The integration floor follows the occupant the user chose (R-563):**
- **Regularisation off:** the substep machinery is unchanged. Close approaches are handled by substepping capped at `N_max`, and **substep-saturated regions are exactly where the integration floor bites** — the deepest collision-manifold structure is substep-limited, not resolved. This is the concrete cause of the deep-zoom note's second floor. It is a **confidence gradient, not a terminal wall**: saturated trajectories *continue* to their real outcomes (flagged as under-resolved, the sticky `saturated` bit), so the close-encounter structure is *computed* (degraded) rather than *cut off*.
- **Regularised** (Aarseth–Zare, Heggie, logH): there is no `N_sub` and no `N_max`. The bound is a **step budget**, a rate of steps per unit physical time per copy (R-564, Part 3's `step_budget_rate`), so every loop stays bounded (Part 2c, rule 3). **Running out of the budget makes the result undetermined, never discarded:** the copy stops, is not finite, and its footprint reads undetermined (`principia_dd_refinement_policy.md` §1). **The separation floor and the entry-cap hits are counted, non-terminal signals;** each lets the run advance under-resolved, and each is counted, never ending the run. Per occupant, each transcribed from its prin-rs driver (applied per R-369, physics review 5605806092 of PR #277, F3; *was:* "the separation floor fires when a pair separation `Rᵢ` falls below the floor `TINY`, so the time transformation's denominator `R₁R₂R₃` is floored and the step advances anyway (prin-rs `r_floored`), and an entry-cap hit is a step whose size is held at the interval's entry cap, `η·Δ_sync` over `dt/dτ` at the interval's entry (prin-rs `n_cap_hits`)"):
  - **The floor `TINY`** is one value per working precision, a calibration (R-71, REQ-INT-105): it is in neither the corpus nor the reference set, which names it only (`T::TINY`).
  - **Heggie:** the quantity floored is `dt/dτ`, at `TINY`: `R₁R₂R₃` under Eq. 20, and `R₁R₂R₃ / S^{3/2}` under the default, Eq. 22 (R-161), with `S` itself floored at `TINY` first; `Rᵢ` is tested, never floored. **The signal counts every step on which the floor acts:** it fires when `dt/dτ < TINY` or (under Eq. 22) `S < TINY`, and also when any pair separation `Rᵢ < TINY`, the reference's own test. This differs from the reference, whose `r_floored` fires on `Rᵢ < TINY` only (`heggie/driver.rs`): `dt/dτ` can fall below `TINY` while every `Rᵢ ≥ TINY` (under Eq. 20 with one `Rᵢ` just above `TINY` and the other two's product below 1; under Eq. 22 near a binary collision, where `dt/dτ ≈ R₁/√(R₂R₃)`, whenever `R₂R₃ > 1`), and such a step advances on a fabricated denominator. Counting it keeps "each is counted" true, as logH's bullet does (applied per R-369, physics review 5606990734 of PR #277, P1). *Was:* "the signal fires when any pair separation `Rᵢ < TINY` (prin-rs `r_floored`)".
  - **Aarseth–Zare:** the signal fires when `A < TINY` or `B < TINY` (prin-rs `ab_floored`); each factor is floored at `TINY`, and the denominator is the product `A·B` of the floored factors. At f32 `TINY·TINY` can underflow, so a step with both factors floored is recorded by the proposal of REQ-INT-105.
  - **logH:** it floors its own drift denominator at `TINY` (`K + B` in the logH time mode, the carried `W` in the TTL mode), so `dt/ds` is `1 / max(drift, TINY)`. The signal fires when that drift denominator is below `TINY` or either denominator, drift or kick, is not positive (prin-rs records the second, `den_degenerate`; the first is counted too, so every floored step is counted, applied per R-369).
  - **An entry-cap hit**, every regularised occupant: the step-size rule in force sizes each step as `η·Δ_sync / (dt/dτ)` from the current state and caps it at the interval's entry size, `dτ_entry = η·Δ_sync / (dt/dτ at the interval's entry)`, each `dt/dτ` the occupant's floored one above. A hit is counted when the per-step size asks for more than the entry cap (prin-rs `capped = cons[0] > cons[1]`, counted into `n_cap_hits`), before the predictive step limit (Part 2b), which may cut the step further, and before the landing clamp, which is never counted as a hit. prin-rs counts it under Aarseth–Zare only; Heggie's and logH's drivers apply the same cap uncounted, and here it is counted under each.

  **How they are stored and drive the floor (R-603, closing RQ-526: a flag bit plus counters, and the tile doesn't split).** Budget exhaustion sets a sticky descriptor flag, `budget_exhausted`, beside `saturated`, the copy's `state` latched as it stood and its footprint undetermined at read; the `saturated` bit is set when either counted signal fires, and the two counts are kept in a payload word (a schema change; the bit's and the counters' layout are REQ-PAY-110, a definition). An undetermined-by-budget footprint is undetermined: never discarded and never treated as resolved, and it takes the refinement policy's `Undetermined` decision, never a split, since halving the cell does not help it (`principia_dd_refinement_policy.md` §1, §6). Its remedy is finer steps, as an explicit re-run the user asks for, never an automatic one. The scheduler's integration-floor stop reads `saturated` and `suspect_fraction` for every occupant (`principia_scheduler_contract.md` Part 4). *Was:* "How budget exhaustion and the two counts are stored in `SimState`, and how they drive the scheduler's integration-floor stop, is RQ-526." Measured over 35 charts at `t = 13` with a whole-run budget of 480,000 steps (prin-rs `results/step_budget/`): running out of the budget was the only failure mechanism present, every chart cleared at 480,000, and a too-small budget **manufactures work**, truncated footprints reading undetermined and splitting. `error_ratio` is blind to a truncated run (it reads its converged 1.0000), so the budget's outcome is read directly, never inferred from drift. At `t = 50` the same 480,000 left 42 of 576 pixels of `burrau_nu_k` exhausted, and the step rate (p99 over pixels, per unit time) was nearly the same at 13 and at 50: about 1.8e4 on `burrau_nu_k`, 4.5e3 on `latent_mixed_h3`, 1.1e3 on `config_stability` and 3e2 on `latent_shape`, the control that never binds; rates are averaged over a pixel's copies and the budget applies per copy, so the worst copy needs more (prin-rs `budget_horizon_probe`, PR #34). Hence a rate, starting at about 4e4 per unit time per copy, about twice the worst measured pixel mean, confirmed at its gate (R-564).

*Was:* "Kustaanheimo–Stiefel (KS) regularisation is **v2**. It transforms the integration variable so the $1/r$ singularity is removed analytically, which would allow fixed-step integration through close encounters with bounded error. But it changes the state representation and ripples through decoder/canonicaliser/observables, so it is not a drop-in occupant (it would not satisfy the `STEP(state,…)` signature on the *same* state type). v1 handles close approaches by brute-force substepping capped at `N_max`. Consequence, connecting to the deep-zoom note: **substep-saturated regions are exactly where the integration floor bites** — the deepest collision-manifold structure is substep-limited, not resolved, until KS lands. … KS (v2) *improves the confidence* in that region; it does not *remove a terminal condition*. The **saturation flag** is the honest v1 marker of reduced trust there." (R-562 removes the v2 text; the substep reading is kept above for regularisation off.)

**Integrator-family mismatch in cross-checks.** The inspector caveat: a non-symplectic inspector (RK45) vs symplectic GPU (KDK/Yoshida) will disagree on the energy trace on long bounded orbits — symplectic oscillates around `E_0`, non-symplectic drifts secularly. A non-trivial `|ΔE|` or `|Δλ_T|` between inspector and GPU is therefore **expected** on long horizons even when both are correct — it is *not* a GPU bug. The inspector must offer a **match-integrator mode** (same occupant + coefficients + step size as the active GPU pipeline) for apples-to-apples validation; reserve adaptive RK45 for chasing near-collision singularities at higher precision than the GPU can deliver. This is the divergence principle applied to integrator *choice*: divergence from precision is honest; divergence from a different occupant is a category error dressed as a bug. **Under the default occupant, Heggie × RK4 (R-560), the GPU is not symplectic either:** RK4's energy error drifts secularly, as the inspector's does, so the oscillating-against-drifting contrast above holds when the active occupant is symplectic (KDK or Yoshida with regularisation `none`); under Heggie × RK4 the two drift by different amounts, and the match-integrator mode stands for every occupant (applied per R-369, physics review 5605806092 of PR #277, F9).

---

## Part 7 — Detectors as the `SimState` producer

Escape and collision are where the categorical outcome comes from — every outcome-class debug view bottoms out here.

- **Collision:** `min_{i<j} ‖r_i − r_j‖ < r_coll`. Uses the *same* `r_min` the substepper already computes — one value, two consumers (like `d_min`). Records the pair.
  **Count the pairs before classifying (pending change 7, landed).** Exactly one pair below `r_coll` → `collision`, `detail` = that pair. Two or more → `collision`, `detail = 3`, a **triple collision**. Testing "any pair below `r_coll`" first steals genuine triples into the binary arm. The rule is on the count, not on "all three", because of the triangle inequality: `|AB| < r_coll` and `|AC| < r_coll` give `|BC| < 2·r_coll`, so exactly two pairs below threshold is already a near-triple. A triple collision is terminal and **non-continuable** (it is provably non-regularisable), unlike a binary collision, where stopping is a choice.
  **`d_min` is the primary stored quantity; the label is derived from it.** `d_min` (over all three pairs) is stored, the collision label is read from it, and `r_coll` is a **recorded parameter carried on every output**, not a physical constant. Measured on a 64×64 near-field sweep: the collision fraction runs 0.0000 → 0.0242 → 1.0000 across three decades of `r_coll`, because the whole grid's `d_min/R` spans less than one decade. No threshold in that range is a physical event boundary. The default `1e-3` of `R` separates tail from bulk on the slices measured, and no more.

**`r_coll` is user-exposed (sim-key), not a fixed constant.** It is *definitional* — it sets what counts as a collision, which is a regularisation choice, not a precision. A slider from "regularise aggressively" (large) to "refuse to regularise" (→0), all valid, answering different questions; changing it is a **sim-key change → invalidate → re-boot** (masked by the staleness backdrop), and it must be the **same value on both pipelines** so the collision *branch* stays bit-identical (parity's shared-branch rule — the threshold is shared sim-key, so this holds). Honest framing of the extremes, because neither means what a naive user expects:
  - **Large `r_coll`** — terminates at any moderately-close approach: *cheaper* (stops before the expensive close-encounter integration) and *coarser* (lumps survived-slingshots in with hits); the basin floods with what are really *close-approach events at threshold R*, not physical collisions.
  - **`r_coll → 0` (or 0)** — refuses to regularise: near-collisions are integrated *through*, so the force `∝1/r²` blows up and the substepper drives `N_sub` toward `N_max`. Singular encounters then **saturate the substep cap** — the trajectory advances under-resolved and flagged (not terminated; saturation is non-terminal — Terminal states, below), continuing to whatever dynamical outcome it reaches. So zero is *not* "perfect integration" and it no longer *manufactures terminations* — it *degrades confidence* at the singular structure (high saturation flag) while the trajectories still run. Expensive (the substepper works hardest here), and the true coincidence still can't be integrated through — but the region is *computed-with-low-confidence*, not *cut off*.
  - **Coupling to `N_max`:** the usable minimum `r_coll` is bounded below by what the substepper resolves before the cap — very small `r_coll` fills the field with **substep-saturated (under-resolved, flagged) samples** (the integration floor, Part 6), not terminations. Either surface the coupling in the UI, or let very-small `r_coll` raise `N_max` at a cost the quality controller accounts for. Under a regularised occupant there is no `N_max`; its bound is Part 6's step budget (R-563). *Was:* "This is the same floor KS regularisation (v2) would lift." (R-562).
  - **Benchmarks pin it:** literature comparisons (Burrau especially — it *is* a close-encounter problem) assume a specific regularisation, so the ground-truth validation suite records the `r_coll` used (`principia_validation_ground_truth_note.md`).
- **Escape (pending change 11, landed):** `ESCAPE ⟺ |Δn̂| over a window < tau AND E_rel > 0`. The shape vector has settled, and the escaper is unbound. Escape is a limit being approached, not a threshold crossed: as the third body recedes, `n̂` converges with `|dn̂/dt| ~ 1/t³` (`principia_01_pitfalls.md` §2.1). **`tau` is not a tuned constant** *(to re-measure, R-29)*. Closure separates escapers from bound trajectories by 383× (7.04e-05 against 2.70e-02, stable across `t = 25–30`), and any value in the middle two orders gives the same answer. Measured: **100% precision, 96.3% recall** *(before R-29's `E_rel`; to re-validate)*. It fires late (`t≈10` rather than `t≈1.5`), which is right for a *stored* `t_end`. Neither condition alone is enough: closure alone also fires on bound hierarchies that settle, and energy alone flickers during encounters. It evaluates **post-step on the projected state**. Records which body escaped.
  **Defined (R-29):**
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
    sees it. The harness's march also renders pitfalls §1.6's `stop_on_escape` "off" image, and that regression stands
    there (R-148). Collision stays terminal regardless.
  **Triple ejection (ionisation)** is `escape` with `detail = 3` (pending change 7): all three bodies mutually unbound, which needs `E > 0`, so it is reachable on the 8D chart but not from rest. The proposed gate is all three pairwise relative energies positive and all three separations growing. Its definition pass is open.
  *Superseded (change 11): the three-gate persistent detector on the outer Jacobi pair — distance (`‖λ‖ > R_esc`), outward (`λ·v_λ > 0`), outer two-body energy (`E_out > 0`) — with a ±1 persistence counter to `k_esc = 8`. It fired on transients: 0 of 895 escapes were still unbound eight sync boundaries later, which would have overstated the escape fraction 5.8×. Under the new rule `receding` and `d > R_esc` are redundant (identical to the digit), so three tuned constants are gone.*
- **Terminal states** are captured by the `state` enum (ledger §3.1): `escape`/`bounded`/`collision` are the physical outcomes; `sim_failed` (NaN/Inf during integration) and `decode_failed` (the decoder could not produce a valid physical IC — NEVER a valid t=0 terminal, which is a real outcome per §5) are lifecycle states, mutually exclusive with them. **`timeout` is not a separate state** — reaching horizon `T` without escape or collision IS `bounded` (it was merged; a bounded trajectory is one that stayed bounded to the horizon). `sim_failed` means the payload is untrustworthy except the state field. **`MAX_SUBSTEPS` is NOT a terminal label** — hitting the substep cap advances the trajectory with the best-available state and sets the sticky **`saturated`** confidence flag (in `sample_descriptor`, bit 5); the march continues to its real dynamical outcome. The cap bounds work-per-step (frame-loop protection) but never terminates. See Part 4 and `principia_dd_integrator.md` §3.6 for the determinism rule (count-bound, not wall-clock). **Running out of a regularised occupant's step budget stops the copy**, which is undetermined: never discarded, and never a physical-outcome label (Part 6, R-563): its `state` stays as it stood and the sticky `budget_exhausted` flag is set (R-603; *was:* "how it is stored is RQ-526"). *Was:* "Running out of a regularised occupant's step budget is not a terminal label either: the copy is undetermined, never discarded", which read, beside the substep cap, as though the march continued (physics review 5605806092 of PR #277, F4).

The detectors are wrapper machinery (shared, determinism-critical per §4), not occupant machinery — they wrap the step and read the same state regardless of which `STEP` produced it.

---

*One swappable `STEP`, one shared wrapper. The occupant advertises a capability profile; the wrapper owns the loop, the substepping, the projection, the monitoring, and the detectors. Values diverge by precision; wrapper branches do not, on identical inputs. Time is dimensionless and global because scale is gauged. Euler is a debug tool. Reversibility is named, bounded and deferred; regularisation is v1, and the integration floor follows the occupant (R-562, R-563). Every stepper × regularisation pair runs, and the pairs measured not to work say so (R-561). Was: "Reversibility, regularisation, and the integration floor are named, bounded, and deferred."*
