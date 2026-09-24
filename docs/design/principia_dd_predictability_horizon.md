# The predictability horizon

---

> ### ⚠ DEFAULT CHANGED — Heggie, not Aarseth–Zare
>
> This document treats **AZ** as the regularisation. **The default is now Heggie global
> regularisation** (`principia_integrator_contract.md` Part 2b, `principia_spec_pending_changes.md`
> change 8).
>
> **Measured:** 31 of 32 cases, `err>10` **3916 → 73**, AZ's worst decile fixed on 100% of pixels. AZ
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

> ### ⚠ Read §7 first — external replication corrected most of this document
> `lambda` is **0.6–0.8, not 1** (so `t_f64 ≈ 52`, `t_f32 ≈ 23`, and `t` is *crossing times*, not
> e-foldings). The **f32 figure is falsified**: f32 AZ fails at `t ≈ 1–2`, and the cause is a
> **conditioning defect in the regularised Hamiltonian**, not precision — `Gamma = A·B·(H − E)` is
> identically zero on-shell, so it is catastrophic cancellation by construction. **A float32 GPU path
> must not run Aarseth–Zare.** The measurement-vs-representability experiment (§6.1) was structurally
> incapable of answering its question. §5.2's treadmill identity is false.

**Status:** derived and empirically confirmed. This is a **physics limit on the instrument**, not an
engineering problem, and it constrains the dual-precision architecture, the CPU↔GPU cross-check, and
what the renderer can honestly display. Supporting measurements in
`principia_dd_refinement_criterion.md` §7.20.

---

## 1. The result

Error of *any* origin grows as `e^(λt)` in a chaotic system. Machine epsilon is a floor below which
no error can be reduced. So round-off alone reaches the size of the system at

```
t_max  =  ln(1/eps) / lambda
```

| precision | `eps` | `t_max` | in crossing times |
|---|---|---|---|
| f16 | 9.8e-04 | 6.9 | 7.2 |
| **f32 — the GPU kernel** | 1.2e-07 | **15.9** | **16.5** |
| **f64 — the CPU kernel** | 2.2e-16 | **36.0** | **37.3** |
| f128 | 1e-34 | 78.3 | 81.1 |

Beyond `t_max` the integration is propagating amplified round-off. The result is not *inaccurate*; it
is **uninformative** — a different rounding would have produced a different, equally valid-looking
trajectory.

### Units, so the numbers mean something

Canonical units for Burrau (`G=1`, `m=(3,4,5)`, `M=12`):

| quantity | value |
|---|---|
| hyperradius `R` | 2.236 |
| total energy `E` | −12.82 |
| crossing time `sqrt(R³/M)` | **0.965** |
| free-fall time | 1.072 |
| Lyapunov time `1/lambda` (measured) | ~1.0 |

The crossing time is ≈1, so **`t` reads directly as both crossing times and e-foldings**. `t=13` is
13.5 crossings; `t=40` is 41. For reference the classical Pythagorean problem disrupts around `t≈60`,
which is **beyond the f64 horizon** — the disruption of Burrau is not fully resolvable in double
precision from a single trajectory.

---

## 2. Why it cannot be engineered away

The horizon is set by **round-off**, not truncation. Truncation error is reduced by taking smaller
steps; round-off is not. Confirmed directly — the energy control (`alpha_E`, true value 1.0) against
integrator tolerance:

| t | eta=0.02 | eta=0.01 | eta=0.005 |
|---|---|---|---|
| 13 | 1.0014 | 1.0214 | 1.0206 |
| 25 | 1.0043 | 0.9394 | 1.0018 |
| **40** | **−13.24** | **0.14** | **0.56** |

**Quadrupling the tolerance does not move the wall**, and the observed transition (works at 13 and 25,
fails at 40) brackets the predicted 36. **More substeps buy nothing.**

Only **mantissa bits** extend the horizon, and only **logarithmically**: doubling the horizon requires
squaring the precision. This is the standard reason chaotic systems are hard — the cost of linear gains
in predictability is exponential in representation.

---

## 3. Two horizons, and they are not the same

There are **two** distinct exponential walls, and they have different causes:

| | reached when | formula | depends on |
|---|---|---|---|
| **Measurement horizon** | the deliberate ensemble jitter `delta` amplifies to system scale | `ln(R/delta) / lambda` | the **sampling** (jitter size) |
| **Representability horizon** | machine epsilon amplifies to system scale | `ln(R/eps) / lambda` | the **arithmetic** |

The measurement horizon comes **first**, since `delta >> eps`. Past it, `ensemble_spread` saturates —
every copy is uniformly distributed over the accessible region and the measure can no longer
distinguish "very chaotic" from "extremely chaotic". This is the saturation documented at
`principia_dd_refinement_criterion.md` §5.

**Which one binds at `t=40` is not yet established.** The `eta`-independence test above does *not*
distinguish them, because **both are eta-independent**. The distinguishing experiment is to **vary the
jitter `delta`**: the measurement horizon moves with it, the representability horizon does not. Worth
running before either is quoted as *the* cause.

The two are separated by `ln(delta/eps)/lambda`, which for a typical jitter and f64 is on the order of
30 time units — so they are far apart and should not be conflated.

---

## 4. Architectural consequences

### 4.1 The two kernels have different horizons

Principia runs **one physics kernel compiled to f32 (GPU) and f64 (CPU)**. Their predictability
horizons differ by **2.3×**: ~16 crossing times against ~37.

**The CPU↔GPU cross-check is only meaningful below the f32 horizon.** Past `t ≈ 16` the two paths are
propagating different amplifications of different round-off; a disagreement there is **not** evidence
of a bug, and an agreement is **not** evidence of correctness. The cross-check should be gated on
`t < t_max(f32)` and reported as *not applicable* beyond it, rather than silently producing
divergences that look like defects.

### 4.2 The horizon is a field, not a constant

`lambda` varies across IC space — that is the whole subject of the instrument — so `t_max` does too.
Since `lambda` is **already computed and stored per sample as FTLE**, the horizon is available for
free:

```
t_max(IC) = ln(1/eps) / ftle(IC)
```

Two uses follow. As a **display**: a horizon map over IC space, showing where the instrument can see
far and where it goes blind early. As a **scheduler input**: a quad whose playhead exceeds its own
`t_max` should not be refined — no resolution recovers information that no longer exists.

### 4.3 It bounds what the renderer may claim

Rendering past `t_max` is not an error so much as a **category mistake**: the image is a picture of
amplified round-off, stable under re-rendering but not a property of the system. The instrument should
**declare its horizon** rather than let the user infer it from the images looking plausible.

This is the honest resolution of the "horizon problem" logged in the refinement work (§7.15g, §7.19a):
not a gate to calibrate, nor a criterion to improve, but **a statement about where the answers stop
meaning anything**.

### 4.4 It explains a result that looked like a defect

The refinement criterion validates at `t=13` and fails at `t=40`. That was recorded as a limitation of
the criterion. **It is not** — `t=40` is past the f64 predictability horizon, so *no* criterion could
succeed there. The criterion works throughout the range in which the underlying trajectories carry
information. **Further work on the criterion cannot change this.**

---

## 5. Theoretical framing, and its use in the papers

The derivation is the classical predictability-horizon argument (Lorenz; Lighthill), which is
textbook. What is worth reporting is its **application to this instrument**: a per-configuration,
computed horizon rather than a global rule of thumb, and its consequences for a dual-precision
implementation.

**Paper 1 (the instrument).** Justifies a stated horizon as a design property rather than an
omission — the instrument knows and reports where it stops being truthful, which is a virtue, not a
caveat. It also supplies the reason the CPU/GPU cross-check is horizon-limited.

**Paper 2 (quantitative characterisation).** `t_max(IC) = ln(1/eps)/lambda(IC)` is a **map derivable
from the FTLE field at no extra cost** — the same measurement, re-expressed as "how long can this
initial condition be predicted at all". It is a natural companion to the fractal-dimension work and
frames FTLE in operational rather than abstract terms.

**A caveat that must travel with any such figure:** `lambda` estimated over a finite time is not the
asymptotic exponent, and the ensemble-based estimator **saturates** — reporting `lambda ≈ 0` for the
*most* chaotic regions once their spread has filled the accessible space (`principia_dd_refinement_
criterion.md` §5.2). A horizon map built from a saturated `lambda` would show the most chaotic regions
as the most predictable. **Use the Benettin/FTLE estimator, which renormalises and does not saturate,
never the ensemble spread.**

---

## 5.1 Retrospective: which existing measurements are inside the horizon

Applied to the refinement work (`principia_dd_refinement_criterion.md` §7.21):

- **Every field verdict is safe** — all contributor rankings were measured at `t <= 13`. The
  two-contributor set (`shape`, `event`) is optimal in the strong sense that shape exceeds diffusion
  and `d_min` at *every* playhead tested, so under a `max` aggregation neither can ever win.
- **The long-horizon escape claims are not.** Results quoted at `t = 240` sit at ~6.7× the f64
  horizon, and `t = 60` is already past it. Those describe amplified round-off. The coarse conclusion
  ("these regions eventually ionise") is plausibly robust, since escape is an absorbing outcome and
  ensemble *statistics* may shadow better than individual trajectories — but the specific fractions
  and timings are not measurements.

**A general rule follows for this project: any result quoted past `t ≈ 36` (f64) or `t ≈ 16` (f32)
needs either higher precision or a shadowing argument before it is treated as a measurement.**

## 5.2 The refinement treadmill is the same relationship

The refinement work found that holding a pixel below a display threshold requires
`delta < tau · e^(−lambda·t)` — the required spacing **halves every 0.69 time units**. The measurement
horizon above is `t = ln(R/delta) / lambda`.

**These are the same equation.** The treadmill therefore has a second, more useful reading:
**halving the sample spacing buys 0.69 additional time units of validity.** Refinement extends the
measurement horizon logarithmically — *the instrument sees further in time precisely by refining in
space*. The exponential cost of refinement and the logarithmic gain in horizon are one fact, not two,
and the trade is explicit: **each factor of 4 in samples (halving spacing in 2D) buys ~0.69 in `t`.**

## 6. Open

1. **Which horizon binds at `t≈40`** — measurement or representability (§3). Distinguishing test:
   vary the jitter `delta` and see whether the failure point moves.
2. **Whether `t_max` should gate refinement**, or merely annotate it. Refusing to refine past a quad's
   own horizon is defensible, but `lambda` is itself uncertain there, so the gate would rest on a
   quantity measured in the regime where it is least reliable.
3. **f32 horizon verification.** The f32 figure (~16) is derived, not measured — all experiments here
   were f64. It should be confirmed against the GPU kernel directly, since it is the tighter of the
   two constraints and the one users will meet first.

---

## 7. Corrections from external replication (report 4)

Four experiments, 29 min, verified against raw JSON (`principia_dd_refinement_external_report4.md`,
`xp_results4/`). **Most of §1–§5 needs amending.**

### 7.1 `alpha_E` is structurally blind to the measurement horizon — my experiment could not work

The distinguishing test in §6.1 (vary the jitter, see whether the failure point moves) returned
**representability**: `ln(delta)` coefficient `−0.0090 ± 0.0151` (`t = −0.60`) over 120 probes across a
16× jitter span. The predicted 2.77-time-unit shift is absent.

**But that result was guaranteed by the instrument, not discovered.** Each trajectory conserves its own
energy exactly, so `sigma_E` across an ensemble is the spread of the *initial* energies — fixed at
`t=0` and unchangeable however completely the ensemble decorrelates. A footprint whose copies have
flown to entirely different outcomes still carries exactly its starting energy spread. **`alpha_E`
measures integration error only.** I pointed an integration-error meter at a decorrelation question.
(Same failure mode as the `L_z` meter: a diagnostic structurally incapable of seeing what it was
aimed at.)

Measured on the **shape vector** instead, same probes: `alpha_shape = 0.411` at `t=13` in near-field,
where `alpha_E` reads 1.003 and stays near 1.0 to `t=40`.

**So the measurement horizon was crossed before the earliest playhead either of us sampled**, and the
representability horizon is twenty-plus time units further out. **The near wall binds — and it is the
one the instrument is blind to.** §3's framing survives; its implied ordering of concern does not.

### 7.2 `lambda` is 0.6–0.8, not 1 — and `t` is not e-foldings

Measured from ensemble growth in the bound phase (`t = 0–20`): **0.708, 0.779, 0.617**. The later
figure (0.18–0.32) is the bound rate diluted by escapers — escape fraction reaches 0.36–0.57 over that
window and an ionised system is not chaotic — so treat it as a floor, not an estimate.

**Revised horizons at `lambda = 0.7`: `t_f64 ≈ 52`, `t_f32 ≈ 23`.**

And a unit error of mine: the crossing time 0.965 makes `t` read as **crossing times**, but one
e-folding is `1/lambda ≈ 1.4` time units — so `t=40` is **~28 e-foldings, not 40**. Any budget treating
them as interchangeable is off by ~40%. Correct §1 accordingly.

### 7.3 The f32 horizon is falsified — and the cause is AZ, not precision

**`t ≈ 1–2`, against the derived ~16.** Verified with a float-width-parametric driver reusing
`AZSystem` verbatim, reproducing the harness bit-for-bit at f64 (`max|dr| = 0.000e+00`).

The decisive decomposition: **rounding the initial state to f32 and evolving in f64 costs nothing to
`t=25`** (three-decimal agreement in all 27 cells), while **f32 arithmetic destroys the control by
`t=2`**. So representability is not the constraint even in f32.

**And it is AZ specifically, not gravity.** Softened leapfrog in f32 holds `alpha_E = +0.983` at `t=5`
where f32 AZ is at `+0.002`, degrading gracefully from `t≈8`. Supporting signature: **shrinking `eta`
makes f32 AZ *worse*** (0.073 → 0.150) — round-off, not truncation.

**The mechanism is structural.** `Gamma = A·B·(H − E)`, and on the physical manifold `H = E`, so
**`Gamma` is identically zero and its terms cancel to zero by construction**. That is catastrophic
cancellation by design, and f32's ~6.9 decimal digits cannot absorb it. **AZ is inherently
ill-conditioned in single precision, independent of any horizon argument.**

> ### ✅ §7.3 IS RESOLVED — the cause was the Levi-Civita branch cut, not arithmetic
> A Rust kernel (`github.com/Ma1achy/prin-rs`) settled this natively in f32, with no emulation gap.
> **Neither candidate was right.** Not `Gamma` conditioning (my fabricated mechanism, withdrawn
> below) and not reference-body switching (the external hypothesis, run both ways and demoted).
>
> The LC inverse `u0 = sqrt((|rho| + rho.x)/2)` catastrophically cancels when `rho` points along −x,
> and `u1 = rho.y/(2 u0)` amplifies the damage. **The Burrau default sits exactly on that cut**: pair
> (1,2) separation is `[3, 0]` — angle 0.0° — at `t=0`, before anything moves. Registration happens at
> every sync boundary, so the error is injected `n_sync` times per trajectory.
>
> | | f64 @179.9° | f32 @179.9° |
> |---|---|---|
> | reference (unstable) | 3.5e-9 | **2.2e-2** |
> | conditioned | ~1e-16 | 5.96e-8 (= f32 eps) |
>
> **This reproduces the reported symptom exactly**: at f32 the unstable branch inflates
> `spread_shape` by an order of magnitude and produces NaN pixels **while single-trajectory energy
> drift stays superficially fine** — which is why one measurement said "f32 AZ is fine" and another
> said "the ensemble diagnostic breaks early", and *both were right*. The conditioned branch tracks
> the f64 answer to ~1%.
>
> **A consequence worth stating separately:** a fixed branch cut means numerical accuracy depends on
> the **absolute orientation** of a configuration in the coordinate frame. The physics is
> rotationally invariant; the unstable implementation is not.
>
> **`reference/tb_lc.py` had the same defect**, so every f32 conclusion drawn from the NumPy harness
> inherits a 2.2e-2 relative loss at `t=0` on the default configuration. It is patched, with the
> unstable form retained behind a flag for reproducing prior results.
>
> Also corrected there: conditioning both sides took the `t=13` cross-check from **1.930e-10 to
> 2.718e-13**, so §2's "the wall is round-off, confirmed by eta-independence" needs one caveat — the
> growth curve distinguishes *wrong algebra* from *amplified ulp noise*, but **not** amplified noise
> from **a small error injected repeatedly**. Branch-cut error at each of 32 registrations produces
> the same exponential-from-a-small-intercept signature. The curve was under-determined and was read
> as conclusive.

> ### ⚠ (superseded) §7.3's mechanism is withdrawn and its conclusion is UNVERIFIED
> **My cancellation explanation was wrong.** I claimed `Gamma = A·B·(H − E)` is identically zero
> on-shell so its terms cancel catastrophically. The equations of motion use **`dGamma/du`, not
> `Gamma`** — the derivative of a function vanishing on a manifold is not itself zero. The mechanism
> was fabricated.
>
> **And an independent check contradicts the conclusion.** f32 max `|dE/E|`, AZ vs softened leapfrog:
> t=1 `1.4e-06` vs `5.7e-07`; t=2 `9.6e-07` vs `3.8e-05` (**AZ 40× better**); t=5 `6.0e-05` vs
> `6.9e-05`; t=8 `1.2e-06` vs `9.2e-04` (**AZ 800× better**). f32 AZ integrates fine and is
> truncation-dominated at the *same value as f64* by t=5–10.
>
> **But the report measured `alpha_E`, not drift**, and that is far more delicate — a ratio of ensemble
> energy spreads between parent and child. A specific mechanism would break it while leaving drift
> healthy: **AZ chooses a reference body per trajectory, per sync**, so two copies of one pixel can
> take different references and accumulate differently-*structured* error, which drift cannot see but
> `sigma_E` across copies can.
>
> Two constraints that hold regardless: **`sigma_E` halves per refinement level while f32 evaluation
> noise is fixed** (1.0e-2 against `|E|·eps32` = 1.5e-6), so `alpha_E` is unmeasurable in f32 after
> ~12 levels *however well the integration went*; and f32 AZ drift reaches ~1e-2 *absolute* near t≈4,
> i.e. `sigma_E` scale, which is roughly where the control was seen to fail.
>
> **Standing position: the f32 *diagnostic* failing early is probably real; the f32 *integration*
> being unusable is not established. Do not put "f32 must not run AZ" in the corpus until §7.7 resolves
> it.**

> **Superseded claim: a float32 GPU path must not run Aarseth–Zare.** The choice costs an
> order of magnitude of playhead, and precision is not the lever that recovers it. §4.1's "the two
> kernels have different horizons" is wrong in its reasoning — the gap is not `ln(eps)` scaling but a
> conditioning defect specific to the regularised formulation. Softened leapfrog, or a different
> regularisation, is the f32 option.

### 7.4 The `t=40` failure is a rate, and mostly an aggregation artefact

**Nothing jumps 0 → 1 at any `t`.** Footprints fail individually at **+1.89 pp per time unit**
(`t = +10.5`). Any threshold crossing is manufactured by choosing an aggregate and a tolerance.

And the aggregation dominates. `sigma_E` is computed per footprint then combined; **the brief did not
specify how**, and a mean is moved by one bad footprint:

| region | mean | median |
|---|---|---|
| near-field t=40 | −1.778 | **+0.979** |
| mid-field t=40 | −2.294 | **+1.005** |
| body-2 core t=20 | −4.035 | **+1.001** |

**But the second half matters more than the first.** At `t=40` near-field the median-aggregated
`alpha_E` reads **+0.979 while 50–81% of footprints are individually broken**. It survives by ignoring
them. **`alpha_E` is a property of the aggregation, not of the trajectories, and must not certify a
frame.**

`error_ratio_max` is what tracks the damage: Spearman **+0.956** against exponent damage versus +0.599
for the median, range 1.003 → 1.0e7 against 0.9997 → 1.588. **This vindicates the max-aggregation
requirement** (`principia_dd_refinement_criterion.md` §7.19c) and strengthens it from a detection
preference to a correctness condition.

### 7.5 §5.2's treadmill identity is wrong

Pooling 128 deduplicated cells, which separates the two variables because Experiment 1 moves `delta`
at fixed quad width:

| term | coefficient | t |
|---|---|---|
| `ln(delta)` | +0.0142 ± 0.0142 | +0.99 (**null**) |
| `ln(half)` | +0.0522 ± 0.0255 | +2.03 |

**One halving buys ≈2.3 time units, not 0.69 — and through the quad width, not the copy spacing.**
`delta` is not the controlling variable, so the rule **cannot** be derived from `delta < tau·e^(−λt)`;
§5.2's "same equation, rearranged" is false. Refinement helps because **a smaller quad covers less
pathological territory**, which is a different mechanism.

Both effects are marginal and near-field is non-monotone in `half`. **Not worth further spend.**

### 7.6 Escape statistics are shadowing-robust to `t=80`, but `t=240` is untested

Escape fraction is **bit-reproducible at `t=20` and `t=40`** across five round-off seeds (sd exactly
0.0000) and 2–3% relative at `t=80`. Escaper identity survives too (96–100% agreement, majority
seed-stable in 94–100%).

**The catch is honest and important:** seeded at 1.5e-15, the ensemble has spread only to ~3e-3 by
`t=80` — still 2.5 orders below system size, i.e. **not yet decorrelated**, which is why it reproduces.
So `t=240` stays **indicative — because it has not been tested there**, not because it is past a
horizon at 36. Twenty more probes would settle it.

