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

**During a run nothing but the step and this projection touches the state (R-854, the owner's symmetry rule of 7 Nov
2026).** The per-step centre-of-mass projection removes floating-point drift and no physical degree of freedom. Energy
and `L_z` are monitored, never corrected. No rotation removal, rescaling, mirroring or canonicalisation ever happens
during a run; a regularised occupant changes variables internally for accuracy and evolves the same physical system.
Symmetry reductions apply only to how a starting state is described at `t = 0`.

The decode already quotients out the CoM translation and bulk momentum. Runtime projection keeps floating-point
drift from putting them back: without it the state drifts as $\mathbf r_i = \mathbf r_i^{\mathrm{internal}} + \mathbf R_{\mathrm{drift}}$,
spending f32 dynamic range on physically irrelevant bulk motion. It is housekeeping, not a user-facing feature.

**Projection**, after every `STEP`, with $M$ constant (computed once and cached). Position–momentum form:

$$\mathbf R_{\mathrm{com}} = \frac{1}{M}\sum_i m_i\mathbf r_i, \quad \mathbf P_{\mathrm{com}} = \sum_i \mathbf p_i, \qquad
\mathbf r_i \leftarrow \mathbf r_i - \mathbf R_{\mathrm{com}}, \quad \mathbf p_i \leftarrow \mathbf p_i - \frac{m_i}{M}\mathbf P_{\mathrm{com}}.$$

An occupant that evolves velocities uses $\mathbf V_{\mathrm{com}} = \frac{1}{M}\sum_i m_i\mathbf v_i$ and
$\mathbf v_i \leftarrow \mathbf v_i - \mathbf V_{\mathrm{com}}$. The cost is about 20 FLOPs, negligible next to a force evaluation. The projection's arithmetic is that; the wrapper then re-forms the force evaluation at the projected positions (`Geom::new`, so the next step is a function of the stored state alone), so KDK makes 2 force evaluations per `STEP` under the wrapper, not the 1 of Part 2's table, which counts the `STEP` alone (applied per R-369; physics review 5722380920 of PR #360, F3).

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

The force-evaluation counts above are the `STEP`'s (the "1×" and the "about 20 FLOPs" of Part 1 alike): under the wrapper each `STEP` also re-forms one force evaluation at the projected positions (Part 1), so a run's total is `total_substeps · (force_evals + 1)` (applied per R-369).

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

*12 Nov 2026 (R-974, the owner's answer on RQ-793, option (a)):* each occupant gains a **joint step** for the FTLE
shadow's displacement form, marching `(x, δ⃗)` together: `x` bit-identical to its `STEP`, `δ⃗` advanced through the same
stages, the force difference formed from pair separations, never from an absolute `x'`; for KDK, Yoshida-4, Yoshida-6
and RK4, and for the default regularised occupant (Part 2b, R-560). Whether that difference is formed plainly in f32
(a1) or cancellation-free (a2) is chosen by a measurement whose criterion TASK-M3-72 writes before any run (R-649);
TASK-M3-72 builds it and defines the shadow's projection, energy and collision test from `x + δ⃗`, under physics review.
*13 Nov 2026 (TASK-M3-72, applied per R-369):* every stepper's `STEP` gains its joint step, `joint_step(s, δ⃗, dt,
geom) → (s', geom', δ⃗')`, `s'` and `geom'` the `STEP`'s bit for bit, `δ⃗'` through the same stages with the force
difference of dd_integrator §3.8, (a1) or (a2) as the step's type parameter: KDK, Yoshida-4, Yoshida-6 and RK4, as
R-974 names them, and Euler, the debug occupant, so that the slot's every stepper has one. The default regularised
occupant's, Heggie × RK4's, is TASK-M3-74's (re-scoped 13 Nov 2026, applied per R-369: Heggie × RK4 needs
TASK-M3-08); the line above, R-974's list, is unchanged.

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

- **`energy_drift` is only a *dynamics* diagnostic on a symplectic occupant.** On KDK/Yoshida, energy oscillates around `E_0` and drift measures the *physics* (close-encounter stress). On Euler/RK4 it measures the *integrator* — Euler drifts secularly and will light up `SUSPECT_ENERGY` everywhere (read by R-660 over the states where its secular drift has time to act: bound, interacting states, RQ-626 (a)). The suspect flag therefore means different things per occupant; a debug/stats layer must read `symplectic` before interpreting drift.
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
| **the dispatch shape** | **DESIGNED FRESH** | 72 KB per quad (512 × 144 B, D6; 76 KB at 152 B from TASK-M3-41 on, with R-603's and R-605's two u32s) fits no backend; see `principia_systems_architecture.md` §5.5 |

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

*11 Nov 2026 (R-953, the owner's answer on RQ-756):* rule 1 has one enumerated exception, the floor `TINY`: one value
per working precision, a calibration (Part 3, Part 6; R-71, REQ-INT-105). A branch may compare a quantity with `TINY`,
as the regularised occupants do: Aarseth–Zare's `A < TINY` and `B < TINY`, Heggie's `Rᵢ < TINY`, `S < TINY` and
`dt/dτ < TINY`, logH's drift denominator below `TINY` (Part 6), and the Levi-Civita inverse's guards `u₀ > TINY` and
`|u₁| > TINY` (Part 2b, the conditioned inverse). This Part used `TINY` in those branch tests before this note without
saying it was excepted; the owner's answer calls it "the same floor constant the contract already excepts", and R-953
writes the exception down. Rule 1's text stands for every other constant: `SYNC_EPS` and `LAND_EPS_REL` stay forbidden
in a branch.

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

**`dt_max`: its home, and the "no usable step" sentinel.** Rule 4's `dt_max` is defined here (a definition, R-72;
REQ-TOOL-039, REQ-TOOL-125).

- **What it is.** `dt_max` is, for one trajectory, the largest physical step its occupant took: the maximum, over the
  steps of its march, of the physical time each one advanced.
- **Its home is an occupant diagnostic, outside the payload:** `kernel::regularisation::dt_max::DtMax`. It is not a
  ledger field and takes no payload bytes, so nothing here goes through the generation root; it is not on the sim key
  and is not hashed. The occupant that forms the physical steps feeds it one step at a time, in march order, and whoever
  runs the occupant reads it afterwards. Placing it in the payload would be a ledger change and needs a ruling.
- **A usable step is a finite one,** as Part 2b's step 8 folds it ("folded into `dt_max` when finite"). A step whose
  exponent field is all ones, a NaN or `±∞`, is unusable and is skipped. That is decided on the step's bits, before any
  comparison (`IeeeBits::non_finite_bits`; R-114, R-538; `principia_gpu_determinism_note.md` § "The discipline",
  rule 4): never by a float compared with itself, and with no precision-dependent constant in the branch (rule 1). Every
  finite step is usable and is reported as it stands, never judged, clamped or filtered: a zero step, a negative one,
  and the overflow `2.209e128`, which is finite at f64 and is reported as itself. At f32 `2.209e128` is `+∞`, so there
  it is skipped and counted.
- **A skipped step is not reported as `dt_max`; it is reported by a count.** Beside the largest usable step the
  diagnostic reports how many steps it skipped, so a march with a non-finite step reads apart from the same march
  without it, and a skipped step is never invisible (`principia_01_pitfalls.md` §9). The count is a `u32` that
  saturates at its largest value: past that many it says "at least this many".
- **The fold is reseeded from the first usable step** (rule 4): the first usable step becomes the maximum, whatever its
  value, and a later usable step replaces it only where it compares greater (`dt > dt_max`). That comparison only ever
  sees finite operands, the non-finite steps having been skipped on their bits before it. No `0.0`, no infinity and no
  other seed value takes part.
- **"No usable step" is an explicit sentinel, never a number.** A trajectory that took no step, or whose every step was
  unusable, reports `DtMaxReport::NoUsableStep { skipped }`; one with a usable step reports
  `DtMaxReport::Largest { dt_max, skipped }`. So "no usable step" cannot be read as "the largest step was `0.0`", which
  is the report of a trajectory whose largest usable step was zero (`principia_01_pitfalls.md` §9). At f32 a trajectory
  whose every step overflowed reports the sentinel with their count.

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
  *11 Nov 2026 (R-953, the owner's answer on RQ-756 (5) 6):* under Aarseth–Zare the limit reads the relative
  **momenta**, not velocities: `dτ ≤ f·d_min / (|P|_max·A·B)`, `|P|_max` the largest of `|P₁|`, `|P₂|` and `|P₂ − P₁|`,
  as the reference code computes it (the Aarseth–Zare section below, step 4). Heggie's and logH's limits read velocities
  (`heggie/driver.rs:242-245`, `logh/driver.rs:256-259`), and this line stands for them as written.
- **TTL is a prior finding, not an open build:** it is built and validated in prin-rs, and it loses on accuracy,
  monotonically worse with mass ratio (prin-rs `NOTES.md:2573`; R-162).

#### Aarseth–Zare: the equations, the reference body, the Levi-Civita inverse and the step control (transcribed)

*Transcribed from docs/reference per CLAUDE.md § Authority, applied per R-369 (10 Nov 2026); physics-reviewed;
confirmed at the M3 gate.* The precedent is R-160, which put Heggie, logH and TTL in the section above; this one does
the same for Aarseth–Zare (RQ-756). Citation: Aarseth & Zare 1974, Celest. Mech. 10, 185, doi:10.1007/BF01227619. Each
equation is written as the prin-rs reference set computes it (reference, not authority, R-159), each checked line by
line against the cited file: `docs/reference/prin-rs/reference/tb_az.py` (`tb_az.py` below),
`docs/reference/prin-rs/reference/tb_lc.py` (`tb_lc.py`), `docs/reference/prin-rs/reference/tb.py` (`tb.py`) and
`docs/reference/prin-rs/src/integrate/az/driver.rs` (`driver.rs`). The Rust AZ system, its Levi-Civita map and its
reference-body selector (`src/integrate/az/system.rs`, `hamiltonian.rs`, `reference_body.rs`) are not in the imported
set, so the equations of motion, the inverse map and the reference-body rule are the NumPy reference's, which
`driver.rs:4` names as the source it was transcribed from. Planar, `G = 1` (`tb.py:10`). Where the reference and the
rest of the corpus disagree, nothing is chosen here: each such point is in RQ-756 and is marked *(RQ-756)* below.

*10 Nov 2026 (R-927, the owner's answer on RQ-756):* on each point where the reference code and the rest of the corpus
disagree, the reference code's choice stands, the reference code being `driver.rs`, and REQ-INT-049's threshold is
re-measured under that same code (REQ-INT-114, a calibration). So, of the marks below: the reference body is the body
not in the longest pair, and the Part 2b table's wording reads so; the unstable form's loss at 179.9° is `tb_lc.py`'s,
`4.0e-12` at f64 and `2.2e-2` at f32; `Γ`'s coupling is the two-pair one given here; the landing secant is part of the
step control, and its stop is `driver.rs`'s (step 6), not `tb_az.py`'s; and REQ-INT-049's gate runs under step 3's
per-step rule. REQ-INT-054's bound per sync interval is not settled: the option the owner chose for it, the step budget
over one interval, `max(N_min, step_budget_rate·Δ_sync)`, is not a bound Part 3's budget gives, that budget being
cumulative since `t = 0` (R-604: the running total against `max(N_min, step_budget_rate · t_elapsed)`), so R-927 flags
it and applies nothing; Part 3 stands as written, and the bound's formula waits on RQ-756 (4). Three marks stay open in
RQ-756, each needing a Rust file that is not in the imported set: whether `V₁, V₂` are velocities or conjugate momenta
(step 4); what the inverse map tests against in place of the literal `1e-300`; and the order in which the selector takes
the three pairs, its first-maximum `argmax` being the driver's own stated convention and the order given here standing
meanwhile.

*11 Nov 2026 (R-953, the owner's answer on RQ-756, which it closes):* the three Rust files are imported, verbatim, from
prin-rs at commit `b1e6006`, which differs from the set's `8600d45` by the owner's word:
`docs/reference/prin-rs/src/integrate/az/system.rs` (`system.rs` below), `.../az/lc.rs` (`lc.rs`) and
`.../az/reference_body.rs` (`reference_body.rs`). `hamiltonian.rs` is not imported and is not needed: the equations
below stay `tb_az.py`'s, which `system.rs:7` names as its source. What they settle, each now written into the text
below with its *Was*:

- **The predictive limit reads momenta (step 4).** `phys_from_state` returns the relative positions and the relative
  momenta, `P₁ = L₁p₁/(2A)`, `P₂ = L₂p₂/(2B)`, `A` and `B` floored at `TINY` (`system.rs:88-102`), and the limit takes
  the largest of `|P₁|`, `|P₂|` and `|P₂ − P₁|`. The comment above `predictive_dtau` (`driver.rs:607-608`, "a physical
  bound `dt <= f*d_min/|v_rel|_max` … `phys_from_state` returns `(R1, R2, V1, V2)`", and `driver.rs:82`) mislabels them
  velocities: **the comment is wrong and the code is the rule** (the owner having checked the code, R-953). The
  difference from R-160's `|v_rel|` is more than a label (R-927, R-953): the momenta are mass-weighted, `P = K·V`
  (`K` the velocity-space mass matrix above), so `f·d_min/|P|_max` is a length over a momentum, not a crossing time, and
  it is not covariant under a rescaling of the masses: masses `×λ` at fixed lengths scale time by `λ^(−1/2)`, `V` by
  `λ^(1/2)` and `P` by `λ^(3/2)`, so the limit moves by `λ⁻¹` against the dynamical time. In the `M = 1` gauge at equal
  masses `P₁ = (2V₁ − V₂)/9` and `P₂ − P₁ = (V₂ − V₁)/3`, so the limit is three times looser or more than R-160's at
  the same `f`; at Burrau's masses 3, 4, 5 (`M = 12`, `tools/xcheck/cases.py:162`) it is tighter. `P₂ − P₁` is
  proportional to `V₂ − V₁` exactly when `m_b = m_c`. So under Aarseth–Zare `f = 0.02` is tied to `M = 1` (the kernel's
  gauge, R-668): its evidence, FINDINGS § 3.3 on `config_stability`, is a latent decode (`grid.rs:430-461`) whose masses
  sum to 1 (the decoder's `ΣM = 1`, `principia_dd_decoder.md:208`). The reference code's form is the rule here. Nobody
  is to "fix" it to velocities. *Was (corrected 11 Nov 2026 by the physics review of PR #488):* "`P₂ − P₁` is not
  proportional to `v_c − v_b` (at equal unit masses `P₁ = (2V₁ − V₂)/3`). The limit therefore differs from R-160's by a
  mass-dependent factor of order one".
- **The inverse map's guards are `TINY`.** `lc.rs`'s conditioned inverse tests `u0 > T::TINY` (`lc.rs:55`) and
  `u1.abs() > T::TINY` (`lc.rs:63`), and its retained unstable form `u0 > T::TINY` (`lc.rs:78-79`), where `tb_lc.py`
  has the literal `1e-300`. `T::TINY` is the floor `TINY` of Part 3 and Part 6, one value per working precision, a
  calibration (R-71, REQ-INT-105); the owner's two numbers, `1e-37` at f32 and `1e-300` at f64, are its values in
  prin-rs (`src/real.rs:61`, `:77` at `b1e6006`, read by the physics consult, not imported). A branch on `TINY` is
  rule 1's one exception (Part 2c, the dated note there). `system.rs` floors at `TINY` too, where `tb_az.py` floors at
  `1e-300`: `A` and `B` in `phys_from_state` (`system.rs:93-94`) and the three separations in `energy_phys`
  (`system.rs:109-111`).
- **The reference body's pairs are taken in the order `(0,1), (0,2), (1,2)`,** as transcribed, by the owner's word;
  `longest_index` keeps the first maximum by a strict `>` ("Strict `>` only, so the first maximum wins — numpy's
  convention", `reference_body.rs:38-47`), and `choose_reference` returns `THIRD[longest_index(…)]`
  (`reference_body.rs:29-31`), `THIRD` the body opposite each pair. The pair order itself is in `src/physics/mod.rs:17`
  at `b1e6006`, `PAIRS = [(0, 1), (0, 2), (1, 2)]`, read by the physics consult (not imported).
- **REQ-INT-054's bound per sync interval (RQ-756 (4), the owner's (4)(a-2)):** R-604's cumulative step budget, read at
  each boundary: the running total of steps at boundary `t_k` is at most `max(N_min, step_budget_rate·t_k)` (Part 3's
  `step_budget_rate` row, `t_k` the physical time since `t = 0`), with no cap per interval. It adds no behaviour: the
  budget check of Part 6 is unchanged, and the bound is what it already gives.

**The reference body (re-chosen at every sync boundary).** From the Cartesian state at the boundary, the three pair
distances are taken in the order `(0,1), (0,2), (1,2)` (`tb.py:15`, `tb.py:41-47`). The longest is found by `argmax`,
and the reference body `a` is **the body not in the longest pair**: `a = [2, 1, 0][argmax]` (`tb_az.py:145-150`). On a
tie for the longest, `argmax` returns the first maximum in that order, so the earlier pair wins: a tie between `(0,1)`
and `(0,2)` gives `a = 2`, and any tie involving `(0,1)` gives `a = 2`; a tie between `(0,2)` and `(1,2)` alone gives
`a = 1`. The other two bodies are `b < c` in index order: `(a, b, c)` is `(0,1,2)`, `(1,0,2)` or `(2,0,1)`
(`tb_az.py:39`, the triple taken at `tb_az.py:259-263`). The regularised pairs are `(a,b)` and `(a,c)`; the side
`(b,c)` is not regularised. At the boundary where `a` is chosen, `(b,c)` is the longest side, `|R₃| ≥ max(|R₁|, |R₂|)`,
so it can then close only at a triple collision (`tb_az.py:8-15`). Within an interval `a` is not re-chosen: the side
can shorten and close with `a` at a finite distance, and that approach enters through the unregularised term
`−A·B·m_b m_c/|R₃|` until the next boundary re-chooses `a`. Hence the floor on `|R₃|` (`tb_az.py:118`) and `|R₂ − R₁|`
in the predictive limit's `d_min` (`driver.rs:620`). A switch is
counted when `a` differs from the previous boundary's (`tb_az.py:255-257`; `driver.rs:955-961`). `driver.rs` also
carries a hysteresis on this choice, default `0`, which at `0` is the plain `argmax` bitwise (`driver.rs:433-445`,
`driver.rs:937-952`), and a forced-reference override for a shared policy (`driver.rs:499-500`, `driver.rs:933-935`);
neither is part of the rule. The Part 2b table's "reference body from the longest side" is this rule (R-927, RQ-756
(5) 5; *was:* "*(RQ-756, the wording)*"), and its pairs are taken in the order above by the owner's word (R-953, RQ-756
(5) 8; `reference_body.rs:38-47`).

**Coordinates and mass factors.** With `m_a, m_b, m_c` the three masses and `M = m_a + m_b + m_c`
(`tb_az.py:46-51`):

- relative positions and velocities `R₁ = r_b − r_a`, `R₂ = r_c − r_a`, `V₁ = v_b − v_a`, `V₂ = v_c − v_a`, and the
  unregularised side `R₃ = R₂ − R₁ = r_c − r_b` (`tb_az.py:11`, `tb_az.py:62-66`);
- the mass factors `μ₁ = m_a m_b / (m_a + m_b)` and `μ₂ = m_a m_c / (m_a + m_c)` (`tb_az.py:52-53`; `1/μ₁ = 1/m_a +
  1/m_b`, `1/μ₂ = 1/m_a + 1/m_c`, `tb_az.py:20`);
- the momenta conjugate to `R₁, R₂`, from the velocity-space mass matrix `K` (`tb_az.py:54-58`, `tb_az.py:67-68`):
  `P₁ = K₁₁V₁ + K₁₂V₂`, `P₂ = K₁₂V₁ + K₂₂V₂`, with `K₁₁ = m_b(m_a + m_c)/M`, `K₂₂ = m_c(m_a + m_b)/M`,
  `K₁₂ = −m_b m_c/M`;
- the physical energy, held fixed through the interval as `E` (`tb_az.py:72`, `tb_az.py:85-94`):
  `E = |P₁|²/(2μ₁) + P₁·P₂/m_a + |P₂|²/(2μ₂) − m_a m_b/|R₁| − m_a m_c/|R₂| − m_b m_c/|R₃|`, the kinetic part with its
  cross term `P₁·P₂/m_a` because `R₁, R₂` share the body `a` rather than being Jacobi coordinates (`tb_az.py:17-22`).
  Each `|Rᵢ|` in it is floored at `1e-300` (`tb_az.py:86-89`); the reference code, `system.rs`'s `energy_phys`, floors
  each at `TINY` (`system.rs:109-111`; R-953).

**The Levi-Civita map on each pair** (`tb_lc.py:11-13`, `tb_lc.py:31-44`). For `u = (u₀, u₁)`,
`L(u) = [[u₀, −u₁], [u₁, u₀]]`, and

- `R = L(u)u = (u₀² − u₁², 2u₀u₁)` (`tb_lc.py:43-44`), so `|R| = |u|²`, and `dR = 2L(u)du` (`tb_lc.py:13`);
- `L(u)w = (u₀w₀ − u₁w₁, u₁w₀ + u₀w₁)` and `L(u)ᵀw = (u₀w₀ + u₁w₁, −u₁w₀ + u₀w₁)` (`tb_lc.py:31-40`).

On both pairs, `R₁ = L(u₁)u₁`, `R₂ = L(u₂)u₂`, `A = |u₁|² = |R₁|`, `B = |u₂|² = |R₂|` (`tb_az.py:24`). Below,
`L₁ = L(u₁)`, `L₂ = L(u₂)`. The regularised momenta are `p₁ = 2L₁ᵀP₁`, `p₂ = 2L₂ᵀP₂` (`tb_az.py:70-71`), and back,
`P₁ = L₁p₁/(2A)`, `P₂ = L₂p₂/(2B)`, with `A` and `B` floored at `1e-300` there (`tb_az.py:76-83`); the reference
code, `system.rs`'s `phys_from_state`, floors them at `TINY` (`system.rs:92-102`; R-953). These `P₁, P₂` are what the
predictive limit reads (step 4).

**The conditioned inverse, `u` from `R`** (`tb_lc.py:80-106`; the default, `USE_STABLE_LC = True`, `tb_lc.py:55` and
`tb_lc.py:109-111`; in `driver.rs` the field `lc_stable`, default `true`, `driver.rs:285-286`, `driver.rs:483`). With
`r = |R|`, `x = R.x`, `y = R.y`, `a = sqrt(max((r + x)/2, 0))` (`= |u₀|`) and `b = sqrt(max((r − x)/2, 0))`
(`= |u₁|`) (`tb_lc.py:89-94`):

- **`x ≥ 0`:** `u₀ = a`; `u₁ = y/(2a)` if `a > TINY`, otherwise `u₁ = b` (`tb_lc.py:96-98`; `lc.rs:53-59`).
- **`x < 0`:** `u₁ = −b` if `y < 0`, otherwise `u₁ = b` (so `+b` at `y = 0`); `u₀ = y/(2u₁)` if `|u₁| > TINY`,
  otherwise `u₀ = a` (`tb_lc.py:100-103`; `lc.rs:60-67`).

The guards are the floor `TINY` (Part 3, Part 6), as the reference code tests them (`lc.rs:55`, `u0 > T::TINY`;
`lc.rs:63`, `u1.abs() > T::TINY`), where `tb_lc.py` tests the literal `1e-300`; a branch on `TINY` is Part 2c rule 1's
one exception (R-953). *Was:* "`a > 1e-300`" and "`|u₁| > 1e-300`".

The larger component is computed directly and the smaller derived from it: `u₀` is the larger when `x ≥ 0`, `u₁` when
`x < 0` (`tb_lc.py:83-85`). **The sign choice:** both branches give `u₀ ≥ 0` (on `x < 0`, `u₁` takes the sign of `y`,
so `y/(2u₁) = |y|/(2b) ≥ 0`), the same half-plane as the unstable form, and the other root `−u` maps to the same `R`
(`tb_lc.py:85-86`). **The unstable form** it replaces, kept only to reproduce earlier results (`tb_lc.py:58-77`):
`u₀ = sqrt(max((r + x)/2, 0))`, then `u₁ = y/(2u₀)` if `u₀ > TINY`, otherwise `sqrt(max((r − x)/2, 0))` (`lc.rs:74-83`;
*was:* "`u₀ > 1e-300`"); it cancels
catastrophically as `R` turns towards `−x` (relative round-trip loss at 179.9°: `4.0e-12` at f64 and `2.2e-2` at f32,
`tb_lc.py:65-69`; R-927 settles it so, `3.5e-9` being `tb_lc.py`'s f64 figure for 179.99°; *was:* "*RQ-756: § 7.3 and
REQ-INT-044 give `3.5e-9` at f64 for 179.9°, the figure `tb_lc.py` gives for 179.99°*"). The guards are `TINY`, rule 1's
one exception (R-953, RQ-756 (5) 7). *Was:* "The guards `1e-300` are literals in branch conditions *(RQ-756, against
Part 2c rule 1 and `TINY`)*."

**The regularised Hamiltonian** `Γ = A·B·(H − E)` (`tb_az.py:27-30`):

`Γ = B|p₁|²/(8μ₁) + A|p₂|²/(8μ₂) + (L₁p₁)·(L₂p₂)/(4m_a) − m_a m_b B − m_a m_c A − A·B·m_b m_c/|R₃| − E·A·B`

The two regularised singular terms, `−m_a m_b/|R₁|` and `−m_a m_c/|R₂|`, have become `−m_a m_b B` and `−m_a m_c A`;
`Γ` is singular only as `|R₃| → 0` (`tb_az.py:32-33`). It couples positions and momenta through its first three terms,
so it is not separable; Part 2a's "the `|u|^2 |p_lambda|^2` term" is the single-pair Levi-Civita `Γ`'s, and this
two-pair coupling is the rule here (R-927, RQ-756 (5) 4; *was:* "*(RQ-756, against Part 2a's "the `|u|^2 |p_lambda|^2`
term")*"). The derivation of the kinetic
terms: `|L₁p₁|² = A|p₁|²`, so `A·B·|P₁|²/(2μ₁) = B|p₁|²/(8μ₁)`, and `A·B·P₁·P₂/m_a = (L₁p₁)·(L₂p₂)/(4m_a)`.

**The equations of motion in fictitious time `τ`** (`tb_az.py:112-142`), on the state `(u₁, p₁, u₂, p₂, t)` with `E`
held, `A` and `B` here not floored and `|R₃|` floored at `1e-300` (`tb_az.py:114-118`):

- `du₁/dτ = ∂Γ/∂p₁ = B·p₁/(4μ₁) + L₁ᵀ(L₂p₂)/(4m_a)` (`tb_az.py:125`)
- `du₂/dτ = ∂Γ/∂p₂ = A·p₂/(4μ₂) + L₂ᵀ(L₁p₁)/(4m_a)` (`tb_az.py:126`)
- `dp₁/dτ = −∂Γ/∂u₁`, with (`tb_az.py:129-133`)
  `∂Γ/∂u₁ = u₁|p₂|²/(4μ₂) + L(p₁)ᵀ(L₂p₂)/(4m_a) − 2m_a m_c u₁ − m_b m_c (2u₁B/|R₃| + 2(A·B/|R₃|³) L₁ᵀR₃) − 2E·B·u₁`
- `dp₂/dτ = −∂Γ/∂u₂`, with (`tb_az.py:135-139`)
  `∂Γ/∂u₂ = u₂|p₁|²/(4μ₁) + L(p₂)ᵀ(L₁p₁)/(4m_a) − 2m_a m_b u₂ − m_b m_c (2u₂A/|R₃| − 2(A·B/|R₃|³) L₂ᵀR₃) − 2E·A·u₂`
- `dt/dτ = A·B` (`tb_az.py:25`, `tb_az.py:141`).

`L(p₁)ᵀ(L₂p₂)` is the gradient in `u₁` of `(L₁p₁)·(L₂p₂)`, since `L(u₁)p₁ = L(p₁)u₁`; the `R₃` terms carry opposite
signs because `∂R₃/∂u₁ = −2L₁` and `∂R₃/∂u₂ = +2L₂`.

**Registration and return.** At each boundary the Cartesian state is mapped to `(u₁, p₁, u₂, p₂)` with the interval
clock `t = 0` and `E` computed as above (`tb_az.py:61-74`); at the end of the interval it is mapped back
(`tb_az.py:96-109`): `V₁ = (K₂₂P₁ − K₁₂P₂)/det`, `V₂ = (K₁₁P₂ − K₁₂P₁)/det`, `det = K₁₁K₂₂ − K₁₂²`; `r_a = 0`,
`r_b = R₁`, `r_c = R₂`, `v_a = 0`, `v_b = V₁`, `v_c = V₂`; then positions and velocities are shifted to the centre of
mass.

**The step control within one sync interval**, `driver.rs`'s defaults (`driver.rs:472-497`): the per-step interval
rule (`DtauMode::PerStepInterval`, `driver.rs:110-126`), the landing clamp on (`clamp_final_step`), the landing secant
on with at most 4 retakes (`land_iterate`, `land_max_iters`), the hard minimum (`StepBlend::Min`), and no per-step limit
(`StepLimit::None`); the predictive limit is R-160's, above, and enters as the third constraint. `tb_az.py` mirrors
these defaults (`tb_az.py:153-238`, the march at `tb_az.py:253-331`), flooring at `1e-300` where `driver.rs` floors at
`TINY` (Part 6), except for the landing secant, which stops differently (step 6).

1. **The interval.** `dt_left = t_target − t` (`driver.rs:975`), `Δ_sync` on the grid (R-565, Part 2a). An interval
   whose `t ≥ t_target − SYNC_EPS` is skipped (`driver.rs:963-966`).
2. **The entry size.** `dτ_entry = η·dt_left / (max(A₀, TINY)·max(B₀, TINY))`, from the registered state
   (`driver.rs:985-987`).
3. **Each step's size** (`driver.rs:699-758`): `ab = max(A, TINY)·max(B, TINY)` from the current state; the
   constraints `c₀ = η·dt_left/ab`, `c₁ = dτ_entry`, `c₂` the per-step limit (`+∞` when none); the entry-cap hit is
   counted when `c₀ > c₁` (`driver.rs:730-734`, Part 6); `dτ` is the least of the finite, positive constraints
   (`driver.rs:639-642`); then the landing clamp, `dτ = min(dτ, max(dt_left − t_s, 0)/ab)`, `t_s` the interval clock
   and `ab` the same floored product (`driver.rs:752-756`). The floor signal fires when `A < TINY` or `B < TINY`
   (`driver.rs:716`, Part 6).
4. **The predictive limit** (`driver.rs:614-632`): `c₂ = f·d_min/(P_max·ab)` with `d_min = min(|R₁|, |R₂|, |R₂ − R₁|)`
   and `P_max = max(|P₁|, |P₂|, |P₂ − P₁|)`, the pair `(P₁, P₂)` being the relative **momenta**, the second half of the
   system's `phys_from_state` (`driver.rs:619-621`; `system.rs:88-102`, `P₁ = L₁p₁/(2A)`, `P₂ = L₂p₂/(2B)`, `A` and `B`
   floored at `TINY`), and `+∞` when `f ≤ 0`, the denominator is not positive, or `d_min` or the denominator is not
   finite (`driver.rs:627-631`). `f = 0.02` (R-160). **The comment above `predictive_dtau` is wrong** and the code is the
   rule (R-953): `driver.rs:607-608` says "a physical bound `dt <= f*d_min/|v_rel|_max`" and "`phys_from_state` returns
   `(R1, R2, V1, V2)`", and `driver.rs:82` "`f * d_min / |v_rel|_max`", but the values read are momenta, not velocities.
   The difference from R-160's `|v_rel|` is more than a label: momenta are mass-weighted (`P = K·V`), so this limit is
   not covariant under a rescaling of the masses (masses `×λ` move it by `λ⁻¹` against the dynamical time); in the
   `M = 1` gauge at equal masses (`P₁ = (2V₁ − V₂)/9`, `P₂ − P₁ = (V₂ − V₁)/3`) it is three times looser or more than a
   velocity limit at the same `f`, at Burrau's masses 3, 4, 5 tighter, and `f = 0.02` is tied to `M = 1` (the dated
   R-953 paragraph above); this is the rule (R-927, R-953). The kernel reads `P₁, P₂`; it is not to be "fixed" to
   velocities. *Was (corrected 11 Nov 2026 by the physics review of PR #488):* "this limit differs from a velocity
   limit by an order-one, mass-dependent factor (at equal unit masses `P₁ = (2V₁ − V₂)/3`)". *Was:*
   "`c₂ = f·d_min/(v_max·ab)` … and `v_max = max(|V₁|, |V₂|, |V₂ − V₁|)`, the pair `(V₁, V₂)` being the second half of
   the system's `phys_from_state` … *(RQ-756: whether `V₁, V₂` are velocities or the conjugate momenta `P₁, P₂`.)*".
5. **The step.** One classical RK4 step of the equations above, on `(u₁, p₁, u₂, p₂, t)` with `E` held, from the
   saved state (`driver.rs:1044-1049`; `tb_az.py:288-292`).
6. **The landing secant** (`driver.rs:1065-1095`). When the step just taken
   reaches the boundary (`t_s ≥ dt_left − dt_left·LAND_EPS_REL`): `want = dt_left − t_before`, `cur = h`, and up to
   `land_max_iters` (4) times, `got = t_s − t_before`; stop if `got` is not finite, `got ≤ 0`, or
   `|got − want| ≤ dt_left·LAND_EPS_REL`; otherwise `cur = cur·want/got` and the step is retaken from the saved
   state at `cur`, each retake a counted step. The clamp predicts the landing step from `A·B` before the step, a
   first-order predictor, and the secant on `t(dτ)` with the exact point `t(0) = 0` removes that residual
   (`driver.rs:417-424`, `driver.rs:1065-1069`); the secant is part of the step control (R-927, RQ-756 (5) 1; *was:*
   "*(RQ-756: the corpus names the clamp only)*"). `tb_az.py`'s secant
   (`tb_az.py:196-210`, `tb_az.py:294-316`) stops differently: on every retake it re-tests the landing,
   `t_s ≥ dt_left − dt_left·LAND_EPS_REL`, and the finiteness of the whole state (`tb_az.py:303-306`), so a retake that
   lands short of the boundary ends the secant and the march takes another clamped step; `driver.rs` does not re-test
   the landing, and retakes until `|got − want| ≤ dt_left·LAND_EPS_REL`, `got` is not finite or `got ≤ 0`, or the 4
   tries are used (`driver.rs:1084-1094`). The two trajectories differ, and `driver.rs`'s stop is the rule (R-927,
   RQ-756 (5) 9; *was:* "*(RQ-756: which stop is the rule)*").
7. **The exit.** The march ends when `t_s ≥ dt_left − dt_left·LAND_EPS_REL`, or the state is not finite, or the
   budget is spent (`driver.rs:997-1012`; here the step budget of Part 6, R-604). The finiteness test is explicit
   (`driver.rs:1000-1001`, "Test is_finite explicitly"), and it is the guard that catches an entry size that is not
   finite, not the floor: at f32 `dτ_entry = η·dt_left/(max(A, TINY)·max(B, TINY))` (step 2) is not finite whenever that
   floored product is below about `η·dt_left/FLT_MAX` (with `η·dt_left` about `1e-4`, a product below about `3e-43`,
   both factors near `5e-22` for example), neither factor then floored and `ab_floored` not firing; both `A` and `B`
   below `TINY`, `TINY·TINY` underflowing to `0`, is one case of it. The entry size is then infinite and the step's
   state is not finite (Part 6, the Aarseth–Zare bullet; R-953). *Was (corrected 11 Nov 2026 by the physics review of PR
   #488):* "it is the guard that catches a doubly degenerate state, not the floor: with both `A` and `B` below `TINY` at
   f32, `TINY·TINY` underflows to `0`, so `dτ_entry` (step 2) is infinite and the step's state is not finite". A state
   degenerate only mid-interval does not reach it: step 3 skips the non-finite constraint (`driver.rs:640`) and takes a
   finite step, a counted cap hit; the infinite step reaches RK4 when the state is degenerate at the interval's entry.
   On a landing the clock is set to the boundary exactly, `t += dt_left`; otherwise `t += min(t_s, dt_left)`
   (`driver.rs:1186-1194`). A step that carries `t_s` past `2·dt_left` is counted as an overshoot
   (`driver.rs:1101-1114`).
8. **The largest physical step**, recorded only: `dt_took = t_s − t_before` after the secant, folded into `dt_max`
   when finite (`driver.rs:1097-1100`); `driver.rs` seeds it at `0` (`driver.rs:885`), which the kernel does not
   (Part 2c, rule 4; REQ-TOOL-039).

`SYNC_EPS` and `LAND_EPS_REL` are the reference's precision-dependent constants that Part 2c rule 1 forbids in a branch;
their values are not part of this transcription.

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

*12 Nov 2026 (R-974):* the joint step of Part 2's dated line widens this seam as well: an occupant that implements
`ADVANCE` marches the shadow's `(x, δ⃗)` with the sample, `x` bit-identical to its `ADVANCE`, as TASK-M3-72 defines it.
*12 Nov 2026 (R-974, physics review 5875695467 of PR #503, F1, a recorded consequence):* under an occupant that owns
its time mapping (Heggie × RK4, the default, `dτ = S^{3/2}/(R₁R₂R₃) dt`, Part 2b, R-161; logH alike), stepping the
sample and its shadow by the same `Δτ` moves them by different `Δt`, so `δ⃗` gains a component along the flow and is no
longer a separation at equal physical time; the finite-difference estimator, ‖δ₀‖'s meaning and dd test 10 then fail.
How the joint step keeps the sample and its shadow at the same physical time under such an occupant is a definition
(R-72) TASK-M3-72 writes under physics review, its options including: carry `δt` as a component of the displacement
and resynchronise it at each renormalisation, or step the shadow's `τ` so that it lands on the sample's `t`.
*13 Nov 2026 (TASK-M3-72's definition, R-72; dd_integrator §3.8):* the seam gains the joint march,
`ADVANCE_JOINT(state, δ⃗, t_now, t_target, params) → (state', δ⃗')`, with a joint per-substep cadence passed in as
Cost 1 has it (it projects `x` and `δ⃗` and watches `x + δ⃗`): `state'` is `ADVANCE`'s, bit for bit, under the same
per-substep work. A stepper's is its `N_sub` joint steps. An occupant that owns its time mapping carries `δt` in the
displacement, advanced by the same `Δτ`, and resynchronises it to first order wherever the separation is read, so that
at every renormalisation the shadow's elapsed physical time equals the sample's (§3.8 gives it in full); Heggie ×
RK4's is TASK-M3-74's. An occupant with no joint march has no displacement-form shadow: marching one beside it fails
to build.

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
| `T_horizon` | default 50, raised or lowered: ∈ [0.4, 200] (R-132, R-575, R-610): the floor one escape window (0.4 provisional, R-29; the floor follows the window's value), below which the escape arm cannot fire, the cap 200 until long horizons are measured for cost (*was:* "∈ (0, 200] meanwhile (R-132, R-575; its range RQ-555)"; *was:* "∈ [50, 200], default 50 (R-132)") | when to stop (physical time, §5) | wrapper loop bound |
| `dt_macro` | `max(1e-3, T/65535)` (**fixed** for a given `T`, §5; R-132) — so `⌈T/dt_macro⌉ ≤ 65535` and the u16 step indices stay exact (R-86) | base macro step | wrapper loop bound |
| `N_max` | tunable (default 64), at most the threshold table's maximum length (a calibration value, R-502), refused above it | substep cap per macro-step — **user-adjustable** (raising it resolves more close-encounter structure before the integration floor; saturation is the stored sticky `saturated` bit, set when `N_sub == N_max` occurs — payload §2; the exact cumulative work lives in the `total_substeps` u32). The substepper also exposes the **live current `N_sub`** per macro-step (an already-computed intermediate) for the animated effort view — render Part 5 | wrapper substepper |
| `r_sub`, `gamma_sub` | 0.05, 1.5 | produce `N_sub` from `r_min`: with `N_max` they generate the threshold table on the host, per sim key, shipped in `SimUniforms` (Part 4, R-502) | wrapper substepper |
| `r_coll` | — | collision threshold — **user-exposed sim-key** (Part 7, collision: definitional regularisation choice, not precision) | wrapper detector |
| `epsilon` | 0 | Plummer softening length: a fraction of the initial hyperradius `R`, fixed at `t = 0`, never co-moving, on the sim key, where the gauge's `R₀ = √(I₀/M) = 1` (Part 5) makes the fraction and the length one number, read as given and never recomputed from the state; `ε ≠ 0` refused at dispatch until TASK-M9-39 lands (Part 7, R-654) | wrapper dispatch |
| `tau`, escape window | —, 0.4 (provisional) | escape settling threshold (not tuned; to re-measure) and the window `|Δn̂|` is taken over, in time units, sampled at macro-step boundaries (unregularised occupants) or sync boundaries (regularised ones, on the fixed grid `Δ_sync`, R-565) (R-29, R-95) | wrapper detector |
| `Δ_sync` | a calibration value (R-71, REQ-INT-080); 0.1 costs about 2% more steps than the production grid, 0.0125 about 50% (Part 2a) | the sync grid of the regularised occupants: a fixed quantum of physical time, the same for every run, independent of `T` and of frame timing; steps are sized from it and the march stops only on it (R-565, Part 2a § "The sync grid is a fixed quantum of physical time (R-565)") | wrapper loop bound (a constant of the build) |
| `step_budget_rate` | a calibration value starting at about 4e4 steps per unit physical time per copy (R-71, R-564, REQ-INT-103) | a regularised occupant's step budget, a rate: steps per unit physical time, per ensemble copy, so it is defined for a live playhead with no fixed end; running out makes the copy undetermined (Part 6, R-563). Accounted every step (R-604, closing RQ-527): a copy's running total of steps is checked at every step against `max(N_min, step_budget_rate · t_elapsed)`, `t_elapsed` its physical time since `t = 0`, and the copy is exhausted when it exceeds it (*was:* "How the rate is accounted along a run is RQ-527") | wrapper loop bound |
| `N_min` | a calibration value (R-71, R-604, REQ-INT-108), confirmed at the M3 gate | the step budget's floor: the allowance below which the budget never falls, so that an early close encounter is not cut before `step_budget_rate · t_elapsed` has grown | wrapper loop bound |
| `TINY` | a calibration value, one per working precision (R-71, REQ-INT-105); the corpus sets no value, and the reference set names `T::TINY` and states its prin-rs values only in a comment (`driver.rs:244-245`) (*was, corrected 11 Nov 2026 by the review of PR #488:* "in neither the corpus nor the reference set"). *11 Nov 2026 (R-953):* its values in prin-rs, which the owner states as "1e-37 at f32, 1e-300 at f64" (prin-rs `src/real.rs:61` and `:77` at `b1e6006`, read by the physics consult; not imported; the set's own `driver.rs:244-245` states them in a comment), are recorded as the reference's values; the calibration is still proposed with its evidence and confirmed at the M3 gate (REQ-INT-105) | the floor of the regularised occupants' time-transformation denominators: Heggie's `S` and `dt/dτ` (its `Rᵢ` are tested against it, not floored; *was:* "Heggie's `Rᵢ`, `S` and `dt/dτ`"), Aarseth–Zare's `A` and `B`, logH's drift denominator; a step taken at the floor is a counted, non-terminal signal (Part 6, R-563). Also, by the reference code (R-953): the Levi-Civita inverse's guards, `u₀ > TINY` and `|u₁| > TINY` (Part 2b, the conditioned inverse; `lc.rs:55`, `:63`), which compare a component of `u` (a length to the power ½) with it, a different use from a denominator's floor; and Aarseth–Zare's three separations in its energy, floored at it (`system.rs:109-111`). A branch on `TINY` is Part 2c rule 1's one exception (R-953) | occupant (regularisation) |
| `r_close` | 0.01 | close-encounter counting | wrapper detector |
| `r_pole` | 0.01 (R-433; the constants register's `r_pole`, provisional until M3, REQ-INT-086) | the pole disc of the unwrapped phase `θ̃` (dd_integrator §3.7): it decides the stored `θ̃`, `_reserved`, `orbit_count` and `retrograde` for the same IC, so it is on the sim key whatever form TASK-M3-11 gives it, and a change at the M3 gate re-integrates (applied per R-369, PR #210). It is not hashed (dd_generation_root §3.8 "The hash"): it changes values, not what the bits mean | wrapper march |
| `eps_E`, `eps_L` | 10⁻⁶ | relative-drift floors (rest starts) | wrapper monitor |
| `G`, `M_total` | 1, 1 | constants | wrapper |

**3. Occupant + tier flags.** Which `STEP` is bound, plus the co-computation selection (`FTLE_ENABLED` — the forward-pass shadow rides the wrapper). These select **baked variants** (lowering Part 3 — per-thread state costs occupancy even branched off), never runtime uniforms. Also sim key. The ensemble is **not** a baked variant: `copy_index` is a uniform, and each copy is the same kernel dispatched again (R-102, R-89).

The occupant needs almost nothing — `(m,r,p)`, `dt`, `G`. Horizon, substepping, and termination are **wrapper** responsibilities, because adaptive substepping is a wrapper concern: it's what turns one fixed `dt_macro` into `N_sub` calls of `STEP`.

---

*4 Nov 2026 (R-698, closing RQ-639):* the escape window's "provisional" ends at the M3 gate: 0.4 is confirmed by measurement over a window ladder, with the range a carried window may take (REQ-EVT-033, TASK-M3-34).

## Part 4 — Determinism, and the substep as the subtle seam

Core-design axiom 1: physics defined once, compiled twice — *structurally*, one Rust source — so the f32/f64 gap is *only* precision **for continuous values** (branch decisions must match on identical inputs, below) and the CPU/GPU divergence is honest. Making the occupant swappable splits the determinism contract cleanly:

- **The `STEP` is the occupant's business** — its arithmetic differs by precision across CPU/GPU, and that difference is the honest divergence the inspector witnesses.
- **The wrapper must be bitwise-consistent across CPU/GPU regardless of occupant** — because it decides *control flow*, not just values.

The trap, and it is the single highest-value thing in this doc: **`N_sub = min(N_max, max(1, ⌈(r_sub/r_min)^γ_sub⌉))` is a branch decision, and a branch that forks across backends is a divergence source that is neither precision nor chaos — it is two `pow` implementations disagreeing, and it makes the CPU sanity-check a *different discretisation* than the GPU ran.** The naïve reading — "compute it in f32 on both sides so the integers match" — **does not survive real compilers** (spike, `principia_spike_brief.md`): the culprit is not fast-math or FMA but *inherent cross-implementation transcendental latitude* (GPU `pow` vs libm `powf` differ 1–3 ulp at integer boundaries, in every math mode), so no runtime `pow`/`sqrt`/`div` feeding a `ceil` can be made backend-stable by matched rounding.

**Rule (the determinism pin, replacing the f32-evaluation framing — full spec in integrator dd §3.3): a branch decision may depend only on comparisons against compile-time constants, or against values every backend receives bit-identically (the sim key's threshold table, R-502), and single-rounded arithmetic — never a runtime transcendental.** `N_sub` is a **bucket lookup**: `d²` compared against the sim key's f32 thresholds `THR[n] = round_f32((r_sub·n^{−1/γ_sub})²)`, `(r_sub/n^{2/3})²` at the default `γ_sub = 1.5`, via a generated comparison tree — no runtime sqrt/div/pow/loop/dynamic index. The thresholds are generated per sim key on the host from its `r_sub`, `γ_sub` and `N_max`, shipped in `SimUniforms` as exact f32 bit patterns and read by constant index; the host computes the `pow` with a pinned pure-software libm, so the native and the wasm hosts give identical bits; `N_max` above the table's maximum length, a calibration value, is refused (R-502, closing RQ-370; dd §3.3). *Was:* "compared against 64 build-time-frozen f32 thresholds `THR[n] = round_f32((r_sub/n^{2/3})²)` … The `pow` moves to build time (into the constants)". The `pow` leaves the device (into the host's table); the sole float feeding the sole branch is `d²`, built **position-quantised** (f32 positions, lone-sub, then only operations every IEEE backend rounds alike (a condition on the driver, R-690), with no intermediate that can be subnormal; R-547, *was* "lone-mul, explicit `fma`") so it too is bit-identical everywhere. Clamp in f32 *before* the float→int cast (out-of-range `OpConvertFToU` is GPU UB). This is the *same* law as the read-side NaN/predicate rule (render Part 4, lowering Part 3a): **a control-flow decision is frozen at build time or reduced to a comparison — never left hostage to a floating-point op the backend computes its own way** (`principia_gpu_determinism_note.md`). Same discipline for the terminal comparisons whose control flow must not diverge:

- Escape comparisons (`|Δn̂| < tau`, `E_rel > 0`; R-29) — shared branches whose inputs are multi-op arithmetic. The old persistence counter that absorbed single-step flips is gone (change 11), so nothing forgives a rounding flip near threshold; these inputs take rule 6's treatment (R-34). *4 Nov 2026 (R-670; physics review 5771183527 of PR #408, F4):* at `ε ≠ 0`, `E_rel > 0` and the ionisation gate's pair energies take `√(d² + ε²)` in the potential term and are the same kind of branch input, so they are enumerated too, and the `sqrt` is never taken at the decision: both sides are positive, so `E_rel > 0` is decided as `|Δv|⁴·(d² + ε²) > 4·(M_pair + m_b)²`, under R-547's rule for a branch input. The gate's third input, total `E > 0`, which R-654 softens, reads the stored `E_0` (softened at `ε ≠ 0`), its sign decided once per trajectory at `t = 0`, an enumerated exception (R-710, below). *Was:* "follows RQ-697: what it reads, its form and how it is listed are that ruling's."
- `SIM_FAILED` raising, the horizon-reached branch (`t ≥ T` ⇒ `bounded` — there is no separate timeout state, payload §2), and the **substep-cap decision** (`N_sub == N_max`) — shared branches. The cap decision leads to *advance with the best-available (under-resolved) step*, not a terminal; the sticky **`saturated`** bit (descriptor bit 5 — a STORED flag set at the point the cap fires, payload §2; the old peak-`substep_log2`-then-derive mechanism is gone) records that it happened. The capped step is computed identically both sides (count-bound, deterministic — never wall-clock), so parity holds through the saturation, not just up to it. **`N_max` is a sim-key tunable** — both pipelines must use the same value (as with `r_coll`), so the shared-branch decision stays bit-identical.

State the boundary explicitly in code: values may diverge by precision; **branch decisions in the wrapper may not, on identical inputs** — each step's decision is identical given that step's inputs (`principia_parity_contract.md` Tier L, R-84). Along a chaotic trajectory the inputs themselves diverge by precision, so its labels may differ across precisions; that is reported, not suppressed.

---

*4 Nov 2026 (R-694, closing RQ-685):* `E_rel` (R-29) holds a division and a `√` through `d`, and `|Δn̂|` a division through `n̂` and a `√` in its magnitude. Each decision takes a written form that uses no driver division or `√`, for example a squared or multiplied-through comparison: REQ-INT-112, TASK-M4-29. The quantities are unchanged. R-670's softened form above is this form at `ε ≠ 0`. The ionisation gate's total `E > 0` is not among these two (R-710).

*4 Nov 2026 (R-710, closing RQ-697):* the gate's total-energy condition reads the stored `E_0`, its sign decided once per trajectory at `t = 0`, at `ε = 0` and `ε ≠ 0` alike. Live `E` differs only by drift, so a label must not flip on integration error. `E_0`'s terms hold a division and a `√`, so its sign is an enumerated exception in `branch_inputs.rs` to this Part's rule: the decision may differ between backends only within a few ulp of `E_0 = 0` (REQ-INT-113), measured and reported, never reconciled. It is the only exception.

## Part 5 — Units and the horizon (inherited scale gauge)

**Everything runs in one fixed dimensionless N-body unit system: `M = Σmᵢ = 1`, `G = 1`, `I = 1`.** This is not a new choice — `I = 1` is the *same* scale gauge as the config decoder's `R̃ = 1` (chart contract Part 1). The integrator **inherits** the scale gauge; it does not impose one.

*1 Nov 2026 (R-668, RQ-668):* the kernel stays at `G = 1` and `M = 1`. A source in another `G` (or total mass) is
converted at the import boundary, by the recorded rescale (`principia_dd_validation_orbits.md` § 2,
"The recorded rescale", its velocities `vᵢ/√(GM)` the time rescaling), and one that can't be converted is refused there. Dispatch
refuses a `SimUniforms` whose `g` or `m_total` is not 1, with a typed reason, as it refuses `ε ≠ 0` (REQ-SYS-096).

Consequence, worth stating so an agent doesn't add a per-IC time rescale: because the similarity symmetry `r → λr, t → λ^{3/2}t` is gauged out (scale fixed to 1), a **single global horizon `T` is legitimate across every pixel**. Two different-scale ICs would need different physical `T` to reach the same dynamical state — but there are no different-scale ICs; scale is fixed. That is *why* one `T` works for the whole map.

*8 Nov 2026 (R-879, RQ-734):* one labelled exception: a final-condition chart, which works back from a state at time `T` and so meets the rescale to `I = 1` after the flow, where it doesn't commute with it, may use a per-pixel horizon, labelled. Every other chart keeps the single global horizon above.

- **`T` is physical time in these units, not a step count.** Default 50, and it can be raised or lowered (R-575, amending R-132): `T ∈ [0.4, 200]` (R-610, closing RQ-555): the floor is one escape window (0.4 provisional, R-29; the floor follows the window's value), below which the escape arm cannot fire, and the settings say so; the cap stays at 200 until long horizons are measured for cost. *Was:* "`T ∈ (0, 200]` meanwhile, the lowest value and the cap being RQ-555". Tens to a couple hundred dynamical (crossing) times, the window in which Burrau resolves into binary + escaper, at the default and above. *Was:* "`T ∈ [50, 200]`, default 50 (R-132)".
- **The macro-step schedule length is derived and deterministic**: `⌈T / dt_macro⌉` macro-steps (× `N_sub` near encounters) — 5×10⁴ at the default `T = 50`, and never more than 65535, since `dt_macro = max(1e-3, T/65535)` (R-132). With `dt_macro` fixed for a given `T`, this length is identical CPU/GPU — **this is the schedule the reversibility work replays** (§ below).
- **CoM re-projection is part of the deterministic step**: each step is *integrate → re-project to CoM* (removes accumulated centre-of-mass / total-linear-momentum drift). The projection is on the parity surface — CPU and GPU must project with identical arithmetic and order. It manages CoM/linear-momentum only; energy and `L_z` remain genuine conservation diagnostics. The integrator works in **CoM-frame particle coordinates** (3 bodies); Jacobi is the chart/IC representation, converted to particles once at decode.
- **Exact step-index timing invariant**: `t_end_step` stores `step_count` (the completed-macro-step count — running: current, terminal: latched, initial: 0; payload §2), as an exact u16. **Requires `horizon_steps = ⌈T/dt_macro⌉ ≤ 65535`, ENFORCED at dispatch** (single format, no Q0.16 fallback); over-limit `(T, dt_macro)` is rejected — use coarser `dt_macro` or march epochs. Also assert the joint `horizon_steps × N_max ≤ 2³²−1` for the exact `total_substeps` counter.
- **t=0 terminals are real outcomes, not decode failures**: at dispatch, before the first step, evaluate the terminal detectors on the decoded IC. A valid IC already inside `r_coll` → `state=collision, t_end_step=0`; escape has no t = 0 case, because its settling test needs a window of history (R-29), so an IC that is already escaping is classified when its window completes (RQ-19). `decode_failed` is reserved for the decoder failing to produce a valid physical IC (non-finite / degenerate / invalid mass), never a dynamical t=0 terminal.
  *10 Nov 2026 (R-933, the owner's rule on starting collided):* a start whose bodies are already collided, one pair or
  more, a double or a triple, is this `t = 0` collision, and its least separation and its pair are recorded as for any
  collision: `d_min` with its pair, and `detail` (the pair's id, or 3 for two or more pairs inside `r_coll`).
- **Branch-cut crossing detection (free-group word, payload §3)**: run detection at **every accepted adaptive substep** (not per-macro-step) — the word mutation is event-only/cold, but detection must be substep-granular to catch multiple/recrossing events. Order simultaneous crossings **chronologically by crossing fraction** `τ = d₀/(d₀−d₁)` (sort ascending); fixed cut-ID priority is a **tie-break only** when `τ` values are equal within tolerance. **Endpoint-on-cut: half-open sign convention** — count iff `negative → non-negative`; fixed and identical CPU/GPU. These branch decisions are Tier-L/Tier-B (parity contract): word content is only as deterministic as they are.
  **The τ tie rule (R-72, REQ-INT-079; a definition, written by TASK-M3-16, physics-reviewed before merge).** An
  accepted substep is a segment from its start state to its end state. Each cut `c`, cut `a` = 0 and cut `b` = 1 (the
  cut IDs; symbolic-dynamics contract §1), has a signed distance `d₀` at the segment's start and `d₁` at its end,
  formed by §1's cut geometry under the rule for every branch input that must agree across backends (Part 4; R-547).
  Detection runs on every accepted substep's segment, in order, each endpoint shared by the two substeps that meet
  there; at most one crossing per cut per segment, so at most two per substep (`max_crossings_per_substep`).
  - **The sign convention.** A `d` is on the *negative* side iff it is below zero and its exponent field is not all
    zeros: `±0` and a subnormal read as zero, on the cut, the non-negative side, as a backend that flushes subnormals
    reads them. The test is on the bits and a comparison with zero, so no precision-dependent constant enters it (Part
    2c, rule 1). A cut is crossed iff its two endpoints' sides differ: `negative → non-negative` (`d₀ < 0`, then
    `d₁ ≥ 0`) is the **positive sense** and appends the cut's generator (`a`, `b`); `non-negative → negative` is the
    **negative sense** and appends its inverse (`A`, `B`). Which way round each cut's `d` is signed, so that its
    positive sense is the literature's generator, is §1's. Since a side is a function of the endpoint's value alone,
    an endpoint exactly on a cut is counted once, by whichever of its two substeps changes side: `−, 0, +` is one
    crossing, in the first substep; `+, 0, −` one, in the second; `−, 0, −` is a crossing and its recrossing, which
    free reduction cancels (payload §3).
  - **The start of a march.** A march's start is an endpoint like any other: it seeds the detection, and nothing is
    appended for it. A collinear start has `d₀ = 0` on the non-negative side, so leaving into `d > 0` appends nothing,
    and leaving into `d < 0` is the negative sense, named by the start's own segment: `A` with body 0 between, `B` with
    body 2, nothing with body 1. Over a closed period this counts the start's syzygy once, whichever way: a period
    leaving into `d < 0` arrives at its end from `d > 0` and reads nothing there; one leaving into `d > 0` arrives from
    `d < 0` and reads the generator there. If rounding leaves `d₀` a few ulps below zero, the side is the negative one:
    leaving into `d > 0` then reads the generator on the first substep, and leaving into `d < 0` reads nothing there,
    so at an outer body's segment that one letter moves or goes, and it is a deterministic function of `d₀` (Tier B). A
    few ulps above zero reads as `d₀ = 0`. On the Šuvakov plane's starts (Table I's among them) the middle body is ours
    1 (R-630, item 3), whose segment carries no letter, so `d₀`'s rounding changes nothing there
    (`crossing_start_on_cut`).
  - **The sort.** A crossed cut's crossing fraction is `τ = |d₀| / (|d₀| + |d₁|)`, which is `d₀/(d₀ − d₁)` on a
    crossing, lies in `[0, 1]`, and has a nonzero denominator, since one endpoint is negative. `τ` is never computed,
    so no division feeds the branch (Part 4). With both cuts crossed, `τ_b < τ_a` is decided as
    `|d₀,b|·|d₁,a| < |d₀,a|·|d₁,b|` (the term `|d₀,a|·|d₀,b|` common to both sides cancels), and the two crossings
    append in ascending `τ`.
  - **Comparison-only, identical on CPU and GPU.** Each product is formed with no float addition, so no backend can
    contract it, and with no intermediate that can be subnormal: a factor that reads as zero (above) makes the product zero;
    otherwise each factor is split at its bits into its exponent and its significand in `[1, 2)`, the two significands
    are multiplied once (a correctly rounded multiply, in `[1, 4)`), and the product is the pair (the exponents' sum
    plus that multiply's own exponent; the multiply's significand), compared exponent first. Bit and integer operations,
    comparisons and one correctly rounded multiply of normal values: given identical `d`, the decision is identical on
    every IEEE backend (a condition on the driver, R-690) at one `Real` (parity contract, Tier B). The two `Real`s can decide a near-tie differently, even
    on the same `d`, since the tolerance below is the working precision's rounding. *Given identical `d`*: along a
    trajectory the `d` differ by precision, and so may the word (pitfalls §10).
  - **The tie tolerance.** Two `τ` are equal iff their two products compare equal: the tolerance is the working
    precision's rounding of those products, about one ulp relative, and no tuned value. A wider tolerance would put
    near-simultaneous crossings in cut-ID order, the order the `τ`-sort exists to replace (payload §3); the decision is
    deterministic because the comparison is exact, not because of a tolerance.
  - **The tie-break.** On a tie the lower cut ID comes first: `a`'s crossing, then `b`'s.
  - **Cuts that share one `d`: the segment gate (symbolic-dynamics contract §1, RQ-632).** §1's two cuts are segments
    of one great circle, the collision equator, and share one signed function `d`. Each carries a *segment gate*, §1's
    segment classifier: a comparison-only test, at an endpoint, of which segment of the equator a side change there
    belongs to. A cut is crossed iff `d`'s two endpoints' sides differ and the cut's gate is open at the substep's
    non-negative endpoint (the end of a crossing in the positive sense, the start of one in the negative). The gates
    are exclusive, so one substep crosses at most one of §1's cuts: the sort above stays defined, and meets two
    crossings only between cuts with distinct `d`. The gate's comparisons are branch inputs under the same rule as `d`
    (Part 4; R-547).
  - **Substep granularity.** Detection reads each `d`'s side at a substep's two endpoints only. An even number of
    crossings of one `d`'s zero within one substep, a crossing and its recrossing among them, leaves the two sides
    equal and goes unseen; an odd number reads as one. Detection is as fine as the accepted substeps (payload §3:
    every accepted adaptive substep), and no finer. A coarse substep fails in three ways, each a word that is wrong
    without any flag (`crossing_substep_limits`):
    - **Lost crossings:** the even count above.
    - **Misattribution:** the classifier reads the substep's non-negative endpoint, so a substep running past the
      crossing by more than a quarter turn of a close pair names the wrong segment. With `r₀ = (0, 0)`, `r₂ = (1, 0)`
      and `r₁` a small `ε` from `r₀`, turning from −20° to 100°, the true crossing, at 0°, has body 1 between and no
      letter; the detector reads `a`.
    - **`P₀₂`:** under §1's cuts a substep passing `P₀₂` across both cuts flips `d` twice and reads nothing, losing the
      `aB` of that loop. §1's cuts lose this case rather than misorder it, which is the case payload §3's `τ`-sort
      paragraph is about.
  - The word identical CPU/GPU, and the GPU arm of the half-open test, are REQ-VAL-072's at M4 (R-113, R-150).
- **FTLE partial-interval finalisation (payload §5)**: the Benettin sum `S` accumulates only at renormalisation boundaries. At read/terminalisation, **finalise the partial interval** — `S_final = S + log(δ_current/δ₀)`, then `ftle = S_final / (step_count · dt_macro)` (divide by full elapsed). Do NOT divide the boundary-only `S` by full elapsed time (systematically too small). `completed_renorms = step_count / renorm_interval_steps` (derived; uniform schedule).
- **`t_end` (SimState termination time) is in the same units**, so it is directly comparable across pixels, quads, and charts. This is what makes the Lehto decay-time map a straight render of `t_end` — the unit system is what lets "decay time" be a chart at all.

---

## Part 6 — Three boundaries stated honestly (deferred work lands here)

**Reversibility.** The shared Lehto/Trani diagnostic (validation doc) needs a genuinely time-symmetric integrator, and **the current stack is symplectic but not cleanly reversible as a run**: state-dependent adaptive substepping (the substep sequence isn't symmetric under reversal) and per-step COM projection (a non-symplectic nudge) break naive "integrate → negate → integrate." The symmetric steppers (KDK, Yoshida-4, Yoshida-6) do not: a symmetric composition with a negative weight (Yoshida-4's `w₀`, Yoshida-6's `w₁`) is still reversible at a fixed step, which dd_integrator §5 test 3 checks (R-500, closing RQ-369). *Was:* "Yoshida's `w₂ < 0` middle step, state-dependent adaptive substepping …, and per-step COM projection … all break naive …" (R-57 fixed the label; R-500 withdraws the claim). Not a bug for the *survey* (symplecticity is what matters there). But the reversibility diagnostic must either (a) record the macro-step + substep schedule and replay it exactly in reverse — feasible precisely because the schedule is deterministic (§5) — or (b) bind a fixed-step reversible occupant with COM projection disabled. Both routes gate on the `step_reversible` capability flag (§2; *was:* "The `reversible` capability flag (§2) gates which occupants qualify"); `run_reversible` marks the one occupant whose whole run is time-symmetric, logH's TTL time mode (R-162, R-500). Stated now so the deferred work doesn't discover it late.

**Regularisation and the integration floor.** **Regularisation is v1** (R-562): the regularisation occupants of Part 2b (`none`, Aarseth–Zare, Heggie, logH) ship, and Heggie × RK4 is the default (R-560). Kustaanheimo–Stiefel (KS) is not one of them, and is not built (R-607, closing RQ-530): Part 2b's four are the
regularisations, and a later method is added by its own ruling. *Was:* "whether it has a place is RQ-530". **The integration floor follows the occupant the user chose (R-563):**
- **Regularisation off:** the substep machinery is unchanged. Close approaches are handled by substepping capped at `N_max`, and **substep-saturated regions are exactly where the integration floor bites** — the deepest collision-manifold structure is substep-limited, not resolved. This is the concrete cause of the deep-zoom note's second floor. It is a **confidence gradient, not a terminal wall**: saturated trajectories *continue* to their real outcomes (flagged as under-resolved, the sticky `saturated` bit), so the close-encounter structure is *computed* (degraded) rather than *cut off*.
- **Regularised** (Aarseth–Zare, Heggie, logH): there is no `N_sub` and no `N_max`. The bound is a **step budget**, a rate of steps per unit physical time per copy (R-564, Part 3's `step_budget_rate`), so every loop stays bounded (Part 2c, rule 3). **Running out of the budget makes the result undetermined, never discarded:** the copy stops, is not finite, and its footprint reads undetermined (`principia_dd_refinement_policy.md` §1). **The separation floor and the entry-cap hits are counted, non-terminal signals;** each lets the run advance under-resolved, and each is counted, never ending the run. Per occupant, each transcribed from its prin-rs driver (applied per R-369, physics review 5605806092 of PR #277, F3; *was:* "the separation floor fires when a pair separation `Rᵢ` falls below the floor `TINY`, so the time transformation's denominator `R₁R₂R₃` is floored and the step advances anyway (prin-rs `r_floored`), and an entry-cap hit is a step whose size is held at the interval's entry cap, `η·Δ_sync` over `dt/dτ` at the interval's entry (prin-rs `n_cap_hits`)"):
  - **The floor `TINY`** is one value per working precision, a calibration (R-71, REQ-INT-105): the corpus sets no value, and the reference set names it (`T::TINY`) and states its prin-rs values, `1e-300` at f64 and `1e-37` at f32, only in a comment (`driver.rs:244-245`). *Was (corrected 11 Nov 2026 by the review of PR #488):* "it is in neither the corpus nor the reference set, which names it only (`T::TINY`)".
  - **Heggie:** the quantity floored is `dt/dτ`, at `TINY`: `R₁R₂R₃` under Eq. 20, and `R₁R₂R₃ / S^{3/2}` under the default, Eq. 22 (R-161), with `S` itself floored at `TINY` first; `Rᵢ` is tested, never floored. **The signal counts every step on which the floor acts:** it fires when `dt/dτ < TINY` or (under Eq. 22) `S < TINY`, and also when any pair separation `Rᵢ < TINY`, the reference's own test. This differs from the reference, whose `r_floored` fires on `Rᵢ < TINY` only (`heggie/driver.rs`): `dt/dτ` can fall below `TINY` while every `Rᵢ ≥ TINY` (under Eq. 20 with one `Rᵢ` just above `TINY` and the other two's product below 1; under Eq. 22 near a binary collision, where `dt/dτ ≈ R₁/√(R₂R₃)`, whenever `R₂R₃ > 1`), and such a step advances on a fabricated denominator. Counting it keeps "each is counted" true, as logH's bullet does (applied per R-369, physics review 5606990734 of PR #277, P1). *Was:* "the signal fires when any pair separation `Rᵢ < TINY` (prin-rs `r_floored`)".
  - **Aarseth–Zare:** the signal fires when `A < TINY` or `B < TINY` (prin-rs `ab_floored`); each factor is floored at `TINY`, and the denominator is the product `A·B` of the floored factors. At f32 `TINY·TINY` can underflow, so a step with both factors floored is recorded by the proposal of REQ-INT-105. *11 Nov 2026 (R-953, the owner's follow-up on RQ-756):* the guard that catches such a doubly degenerate state is the march's explicit finiteness test (Part 2b, the Aarseth–Zare step control, step 7; `driver.rs:1000-1001`), not the floor: at f32 `dτ = η·dt_left/(A·B)`, `A·B` the product of the floored factors, is not finite whenever that product is below about `η·dt_left/FLT_MAX` (about `3e-43` at `η·dt_left ≈ 1e-4`, both factors near `5e-22` for example), when neither factor is floored and `ab_floored` does not fire; `1e-37 × 1e-37` underflowing to `0` is one case of it. *Was (corrected 11 Nov 2026 by the physics review of PR #488):* "`1e-37 × 1e-37` underflows to `0` at f32, the denominator `A·B` of the floored factors is `0`, and `dτ = η·dt_left/(A·B)` is infinite." Its test arm (REQ-INT-105's, built by TASK-M3-68) exercises that finiteness test on a state degenerate at the interval's entry, since a state degenerate only mid-interval takes a finite step under step 3's per-step rule and never reaches it.
  - **logH:** it floors its own drift denominator at `TINY` (`K + B` in the logH time mode, the carried `W` in the TTL mode), so `dt/ds` is `1 / max(drift, TINY)`. The signal fires when that drift denominator is below `TINY` or either denominator, drift or kick, is not positive (prin-rs records the second, `den_degenerate`; the first is counted too, so every floored step is counted, applied per R-369).
  - **An entry-cap hit**, every regularised occupant: the step-size rule in force sizes each step as `η·Δ_sync / (dt/dτ)` from the current state and caps it at the interval's entry size, `dτ_entry = η·Δ_sync / (dt/dτ at the interval's entry)`, each `dt/dτ` the occupant's floored one above. A hit is counted when the per-step size asks for more than the entry cap (prin-rs `capped = cons[0] > cons[1]`, counted into `n_cap_hits`), before the predictive step limit (Part 2b), which may cut the step further, and before the landing clamp, which is never counted as a hit. prin-rs counts it under Aarseth–Zare only; Heggie's and logH's drivers apply the same cap uncounted, and here it is counted under each.

  **How they are stored and drive the floor (R-603, closing RQ-526: a flag bit plus counters, and the tile doesn't split).** Budget exhaustion sets a sticky flag, `budget_exhausted`, the copy's `state` latched as it stood and its footprint undetermined at read; the `saturated` bit is set when either counted signal fires. The flag and the two counts are kept together in the hot payload word `budget_word`, not in `sample_descriptor`, whose sixteen bits are all assigned (R-545; payload §1; REQ-PAY-110, whose bits are a definition), a schema change. *Was (code review 5662211284 of PR #293, applied per R-369):* "Budget exhaustion sets a sticky descriptor flag, `budget_exhausted`, beside `saturated`, … and the two counts are kept in a payload word (a schema change; the bit's and the counters' layout are REQ-PAY-110, a definition)." An undetermined-by-budget footprint is undetermined: never discarded and never treated as resolved, and it takes the refinement policy's `Undetermined` decision, never a split, since halving the cell does not help it (`principia_dd_refinement_policy.md` §1, §6). Its remedy is finer steps, as an explicit re-run the user asks for, never an automatic one. The scheduler's integration-floor stop reads `saturated` and `suspect_fraction` for every occupant (`principia_scheduler_contract.md` Part 4). *Was:* "How budget exhaustion and the two counts are stored in `SimState`, and how they drive the scheduler's integration-floor stop, is RQ-526." Measured over 35 charts at `t = 13` with a whole-run budget of 480,000 steps (prin-rs `results/step_budget/`): running out of the budget was the only failure mechanism present, every chart cleared at 480,000, and a too-small budget **manufactures work**, truncated footprints reading undetermined and splitting. `error_ratio` is blind to a truncated run (it reads its converged 1.0000), so the budget's outcome is read directly, never inferred from drift. At `t = 50` the same 480,000 left 42 of 576 pixels of `burrau_nu_k` exhausted, and the step rate (p99 over pixels, per unit time) was nearly the same at 13 and at 50: about 1.8e4 on `burrau_nu_k`, 4.5e3 on `latent_mixed_h3`, 1.1e3 on `config_stability` and 3e2 on `latent_shape`, the control that never binds; rates are averaged over a pixel's copies and the budget applies per copy, so the worst copy needs more (prin-rs `budget_horizon_probe`, PR #34). Hence a rate, starting at about 4e4 per unit time per copy, about twice the worst measured pixel mean, confirmed at its gate (R-564).

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
  - **The collision test's floor (a definition; 11 Nov 2026, R-954, the owner's answer on RQ-782).** The test reads the substepper's squared separation, formed from each position cast to f32 (`principia_dd_integrator.md` § 3.3 and § 3.6; R-547), so it resolves a separation only to the f32 spacing of the positions at the pair's coordinates: about `6·10⁻⁸` for `|x|` in `[0.5, 1)` and `1.2·10⁻⁷` in `[1, 2)`, in the `I = 1` gauge (Part 5). **Below that spacing, `r_coll` means "collision at f32 coincidence":** a pair collides when its f32 positions coincide, which can be at a true separation larger than `r_coll`, and `d_min` is resolved only to that spacing. **The slider stays valid to 0,** as above. **The floor is recorded with the run,** beside `r_coll`, so an output says whether its `r_coll` lay below what the test resolves; what is recorded, and how it is computed from the run, is a definition TASK-M3-71 writes into this Part under physics review (R-72; REQ-EVT-035). The default `10⁻³` lies four decades above the floor. The test itself is unchanged: TASK-M3-09's threshold of at least `2⁻¹²⁶` for every `r_coll > 0` stands, so f32-coincident bodies always collide.
    *12 Nov 2026 (R-973, the owner's answer on RQ-792, option (a) in its cautious form):* the form of the record is ruled: a per-sample byte holding the floor's exponent, latched as the largest floor over every state at which the test was near firing (`d_quant(s) < r_coll + floor(s)`) and over the `d_min` state, each output's flag read as `r_coll < floor`. TASK-M3-71 writes the bit-level definition here, under physics review (REQ-EVT-035).

**Softening, `epsilon` (R-654).** Where `ε ≠ 0` the force law is Plummer's, `V = −Σ mᵢmⱼ/√(r² + ε²)` and `Fᵢ = −∂V/∂rᵢ`, a different force law at any `ε` (generation_root § "Open"), tagged and never pooled with `ε = 0`. `ε` is canonical, a fraction of the initial hyperradius `R` fixed at `t = 0`, never co-moving; like `r_coll` it is a **recorded parameter**, in every output header. The gauge (Part 5, `M = I = 1`) has `R₀ = √(I₀/M) = 1`, so the sim-key value is both the fraction and the length: it is read as given, and never recomputed from the state, whose `I` is 1 only to rounding. `E`, `E₀` and the drift meters of Part 1 read the softened `V`. Runs with `ε ≠ 0` are refused at dispatch until TASK-M9-39 lands. *2 Nov 2026 (R-670, closing RQ-656):* under `ε ≠ 0`, the collision test stays on geometric `r`, `min ‖rᵢ − rⱼ‖ < r_coll`; softening is offered on the unregularised occupants only, and a regularised occupant (Part 2b) refuses `ε ≠ 0` with a typed reason; the Chazy classifier's `E` is the softened `E_0`; `E_rel` and the ionisation gate's three pairwise relative energies take the softened potential, `d` read as `√(d² + ε²)` in the potential term only (identical at `ε = 0`; `tau`'s measured gap and `E_rel`'s to-re-measure note were not taken under softening); the decode's `V₀` is softened (`principia_dd_decoder.md` § 3.6); a view refuses `ε ≠ 0` with any `Some(E*)` (§ 3.7); and the substep law stays on the geometric `d²` and `r_min`. *Was:* "The collision test, the regularised occupants, the Chazy classifier, `E_rel` and the ionisation gate, the decode's `V₀`, `η_E`'s `U` and the substep law under `ε ≠ 0` are RQ-656."

*5 Nov 2026 (R-791, closing RQ-693 (b); R-792, closing RQ-694 (a)):* two additions to that list. The Chazy classifier's two-body potentials soften too, `pair_kup`'s and `out_kup`'s `U` reading `√(d² + ε²)`, the separations, `v_r` and the kinetic terms geometric. And the `(L_z, E)` chart reads a Newtonian `U`, not the softened `V₀`, in its axis, feasible set, warp and `K*`, so a pixel's start does not depend on `ε`. The other axes that read `U` are RQ-709.

- **Escape (pending change 11, landed):** `ESCAPE ⟺ |Δn̂| over a window < tau AND E_rel > 0`. The shape vector has settled, and the escaper is unbound. Escape is a limit being approached, not a threshold crossed: as the third body recedes, `n̂` converges with `|dn̂/dt| ~ 1/t³` (`principia_01_pitfalls.md` §2.1). **`tau` is not a tuned constant** *(to re-measure, R-29)*. Closure separates escapers from bound trajectories by 383× (7.04e-05 against 2.70e-02, stable across `t = 25–30`), and any value in the middle two orders gives the same answer. Measured: **100% precision, 96.3% recall** *(before R-29's `E_rel`; to re-validate)*. It fires late (`t≈10` rather than `t≈1.5`), which is right for a *stored* `t_end`. Neither condition alone is enough: closure alone also fires on bound hierarchies that settle, and energy alone flickers during encounters. It evaluates **post-step on the projected state**. Records which body escaped. **Triple ejection** (`detail = 3`, R-32)
is a classification of a firing this rule has already made, behind `E_rel > 0`, never a second way to fire (R-818,
ruling RQ-708 (a)).
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
  **Triple ejection (ionisation)** is `escape` with `detail = 3` (pending change 7): all three bodies mutually unbound, which needs `E > 0`, so it is reachable on the 8D chart but not from rest. The proposed gate is all three pairwise relative energies positive and all three separations growing. Its definition pass is open. *4 Nov 2026 (R-32, R-710):* the gate is those two and total `E > 0`, reading the stored `E_0`, its sign decided once per trajectory at `t = 0`, because `E < 0` forbids total escape.
  *Superseded (change 11): the three-gate persistent detector on the outer Jacobi pair — distance (`‖λ‖ > R_esc`), outward (`λ·v_λ > 0`), outer two-body energy (`E_out > 0`) — with a ±1 persistence counter to `k_esc = 8`. It fired on transients: 0 of 895 escapes were still unbound eight sync boundaries later, which would have overstated the escape fraction 5.8×. Under the new rule `receding` and `d > R_esc` are redundant (identical to the digit), so three tuned constants are gone.*
- **Terminal states** are captured by the `state` enum (ledger §3.1): `escape`/`bounded`/`collision` are the physical outcomes; `sim_failed` (NaN/Inf during integration) and `decode_failed` (the decoder could not produce a valid physical IC — NEVER a valid t=0 terminal, which is a real outcome per §5) are lifecycle states, mutually exclusive with them. **`timeout` is not a separate state** — reaching horizon `T` without escape or collision IS `bounded` (it was merged; a bounded trajectory is one that stayed bounded to the horizon). `sim_failed` means the payload is untrustworthy except the state field. **`MAX_SUBSTEPS` is NOT a terminal label** — hitting the substep cap advances the trajectory with the best-available state and sets the sticky **`saturated`** confidence flag (in `sample_descriptor`, bit 5); the march continues to its real dynamical outcome. The cap bounds work-per-step (frame-loop protection) but never terminates. See Part 4 and `principia_dd_integrator.md` §3.6 for the determinism rule (count-bound, not wall-clock). **Running out of a regularised occupant's step budget stops the copy**, which is undetermined: never discarded, and never a physical-outcome label (Part 6, R-563): its `state` stays as it stood and the sticky `budget_exhausted` flag is set (R-603; *was:* "how it is stored is RQ-526"). *Was:* "Running out of a regularised occupant's step budget is not a terminal label either: the copy is undetermined, never discarded", which read, beside the substep cap, as though the march continued (physics review 5605806092 of PR #277, F4).

The detectors are wrapper machinery (shared, determinism-critical per §4), not occupant machinery — they wrap the step and read the same state regardless of which `STEP` produced it.

---

*One swappable `STEP`, one shared wrapper. The occupant advertises a capability profile; the wrapper owns the loop, the substepping, the projection, the monitoring, and the detectors. Values diverge by precision; wrapper branches do not, on identical inputs. Time is dimensionless and global because scale is gauged. Euler is a debug tool. Reversibility is named, bounded and deferred; regularisation is v1, and the integration floor follows the occupant (R-562, R-563). Every stepper × regularisation pair runs, and the pairs measured not to work say so (R-561). Was: "Reversibility, regularisation, and the integration floor are named, bounded, and deferred."*
