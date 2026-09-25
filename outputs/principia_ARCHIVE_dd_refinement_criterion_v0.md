> # ARCHIVED — SUPERSEDED, DO NOT IMPLEMENT FROM THIS
>
> This document records the **variance-ratio and α-exponent era** of the refinement criterion. The
> policy it describes was replaced during the prin-rs vertical slice, and the **metric it is
> measured with is now known to be void**.
>
> **Superseded by:** `principia_dd_refinement_policy.md`
> **Evidence:** `prin-rs` — `FINDINGS.md` §5, `results/tolerance_scaling/README.md`
>
> **What specifically does not survive:**
>
> - **The α-exponent split test.** It split where halving the cell had *already* halved the spread —
>   the **smooth** regions — and floored at discontinuities. Inverted.
> - **`error(B)` scored in OKLab under the shipping colouring**, whose lightness is auto-ranged per
>   region. A smooth region's 1e-8 residual counted as error at every depth, so **breadth-first came
>   out near-optimal by construction of the metric.** Every "nothing beats uniform" conclusion here
>   is an artefact of scoring in render space.
> - **`tau_display`, `alpha_hi`, `k_frac`** as the parameter space. Replaced by a single `eps`.
>
> **What survives and is worth reading:** the methodology, the corrections in §7, the no-discard
> reasoning, and the treadmill argument (§4). The *reasoning about how to measure* is sound; the
> *measurements* were taken on a substrate with known defects.

> ## ⚠ SUPERSEDED IN PART — `prin-rs/RESULTS.md` is authoritative for anything it covers
>
> This document records **how the refinement design was reached**, which is why it is kept intact
> rather than rewritten. But a Rust kernel (`github.com/Ma1achy/prin-rs`) has since re-measured much
> of it at 10^6 samples with a validated, conditioned integrator, and **four classes of claim here are
> known wrong**. Read this for the reasoning; read `RESULTS.md` for the numbers.
>
> | claim here | status |
> |---|---|
> | **The f32 verdict** (§7.19 / horizon doc §7.3) — "f32 AZ fails at t≈1–2", mechanism unresolved | **Resolved, and it was neither of the proposed causes.** The Levi-Civita inverse `u0 = sqrt((\|rho\| + rho.x)/2)` cancels when `rho` points along −x. **The Burrau default sits exactly on that cut** — pair (1,2) separation is `[3, 0]`, angle 0.0°, at `t=0`. At f64 the leak is negligible; at f32 it inflates `spread_shape` by an order of magnitude and produces NaN pixels **while single-trajectory drift stays superficially fine** — the reported symptom exactly. The conditioned branch tracks f64 to ~1%. Neither arithmetic conditioning of `Gamma` (my fabricated mechanism, §7.19 banner) nor reference-body switching (the external hypothesis) was the cause. |
> | **`deep interior` as a near-triple encounter** (§7.7, §7.8) — cited `d_min = 1.6e-07` with the longest side falling to 0.27 | **Wrong. It is an ordinary binary collision.** Pair (0,2) closes to 2.28e-5 while (0,1) and (1,2) never register even at `r_coll = R`. The original figures came from a run with **energy drift of 37** — the unregularised integrator. Under AZ both implementations reach `t=13` in ~1 s at `\|dE/E\| ~ 1.4e-7` and agree to three digits. |
> | **Anything measured at `n = 64`** — most of §5–§7 rests on 16–64 trajectories over 4–8 regions | **Treat as indicative only.** This is the exact sample size the kernel was built to replace, and conclusions in this document flipped repeatedly under resampling (1.2× → 18.8× → 5× for one field across three region sets). |
> | **Event class's "~4 time units earlier" lead time** (§7.7) | **Not reproduced.** Measured under a definition that was then mis-transcribed into the kernel brief (terminal `(state,detail)` instead of tightest-pair identity), so the comparison was never like-for-like. |
>
> **One qualification of a field this document specifies.** `error_ratio = sigma_E(t)/sigma_E(0)`
> **detects spread, not drift.** An ensemble whose copies drift *together* has a low ratio however
> badly they drift — correlated error shifts every copy's energy alike and leaves the spread
> untouched. Measured: after refinement `deep interior` has **zero flagged pixels at 11% energy
> error**. It is therefore blind to precisely the failure a systematically-wrong integrator produces.
> `energy_drift_max` is the field to threshold on when absolute conservation is what matters. Both are
> needed; neither substitutes.
>
> **What survives unchanged:** the structural arguments — the ratio criterion's algebraic
> cancellation (§3.2), the treadmill (§4), the conserved-quantity signature (§6.3), the bounded-
> contributor requirement (§7.15c), and the uniformity invariant (§7.16). Those are algebra and
> design reasoning, not measurements, and re-sampling does not touch them.

# Refinement criterion & ensemble spread — experimental findings

**Status:** empirical. Records *why* the refinement mechanism and `ensemble_spread` are defined the
way they are — including several proposals that were tested and rejected, and several rejections
that were later overturned by better sampling.

**Read §1 and §9 if nothing else.** §1 is what was found; §9 is what remains open.

---

## 1. Summary

### 1.1 The rejected proposal

A refinement criterion was derived from first principles as a **variance ratio**:

```
ratio = between / (between + within)
```

`within` = spread inside one pixel's ensemble bundle (nominally irreducible); `between` = spread
across the quad's samples (nominally reducible by splitting). Measured on Burrau it is **≈0.62 in
every region at every scale**. The failure is **algebraic**: both terms carry the same exponential
factor, which cancels, leaving the ratio of the two *initial* length scales — i.e. **the SSAA jitter
fraction**. A refinement threshold built on it would need recalibrating for every quality tier.

### 1.2 What replaces it

**The scaling exponent**, measured across ≥2 refinement levels, not a single-scale ratio.
Exponent ≈1 → refining reduces uncertainty linearly (**split**); ≈0 → refining changes nothing
(**floor**); no spread at any scale (**keep coarse**). For discrete outcome fields this is exactly
the classical **uncertainty exponent** α (Grebogi–Ott–Yorke), so the basin-boundary dimension
`D = d − α` — the Paper 2 measurement — is the *same computation*. The quad hierarchy already
supplies the two scales, and the scheduler's existing "parent–child disagreement" signal **is** an
exponent estimator.

### 1.3 Two axes untangle "chaos" from "the IC map"

A spread can be large because the flow amplified a perturbation (**dynamics**) or merely because the
field varies smoothly across IC space (**the map**). Two independent diagnostics separate them:

| axis | definition | reads |
|---|---|---|
| **dynamics score** | `d(log spread)/dt` at fixed δ | growth = chaos amplifying; flat = just the map |
| **map exponent** | `d(log spread)/d(log δ)` at fixed t | ≈1 = responds fully to δ; ≈0 = doesn't respond |

**A useful contributor needs both.** Dynamics ≈0 → it measures the IC map, not chaos.
Exponent ≈0 → refining cannot reduce it, so it cannot inform refinement.

The method validates itself on **total energy**: dynamics **0.001**, map exponent **0.963** — exactly
the textbook pure-map signature (`spread = |∇E|·δ`: fully δ-responsive, entirely unamplified).

### 1.4 Other findings

- **Refinement demand grows exponentially with playhead.** Measured λ ≈ 1.01, so the spacing needed
  to hold a pixel below threshold **halves every ≈0.69 time units**. No budget keeps up: the
  integration floor is not a static region property but a **front sweeping IC space as `t` advances**.
- **Free-running ensemble spread and Benettin FTLE are not interchangeable in either direction.**
  Ensembles saturate — and report `λ ≈ 0` for the *most* chaotic regions when they do. FTLE does not,
  because renormalisation *is* the anti-saturation mechanism.
- **Outcome-based fields invert under lockstep.** Early in the march nearly every sample is `RUNNING`,
  so impurity ≈0 and agreement ≈1: maximum apparent confidence exactly when nothing is known.
- **Event spread must be measured at joint `class ⊕ detail` grain** — it strictly dominates either
  grain alone, and using one grain everywhere dissolves a separate spec ambiguity.

---

## 2. Methodology

### 2.1 System

Planar three-body, Newtonian, `G=1`. Two configurations:

- **Burrau (Pythagorean)** — `m = (3,4,5)` released from rest at the vertices of a 3-4-5 triangle.
  Primary system; it is the Paper 2 target.
- **Equal-mass free-fall** — `m = (1,1,1)` from an isoceles triangle. Used once, to test whether
  results were chart-specific (§4.2).

The **IC slice** is the position of one body over a 2D box; which body is perturbed is a parameter,
giving three distinct slices per system.

### 2.2 Integrator, and why its validation mattered

Vectorised KDK leapfrog, all ICs batched as array rows, softened gravity (`eps`) in place of
regularisation. **The harness defaults produced a median relative energy drift of 54** — i.e.
physically meaningless. Parameters were swept before any measurement was taken:

| `eps` | `dt` | median &#124;ΔE/E&#124; | max &#124;ΔE/E&#124; |
|---|---|---|---|
| 0.01 | 2e-3 | 3.8e+01 | 1.7e+02 |
| 0.01 | 5e-4 | 1.9e+00 | 2.4e+01 |
| 0.03 | 2e-3 | 2.1e+00 | 5.7e+00 |
| **0.03** | **5e-4** | **1.0e-05** | **9.7e-04** |
| 0.1 | 2e-3 | 2.1e-07 | 2.3e-04 |

**`eps = 0.03, dt = 5e-4` used throughout** — acceptable drift without softening so heavily that
close encounters are erased. This is a *qualitative design test*, not production physics: `eps=0.03`
smooths exactly the close approaches where Burrau's interesting dynamics live.

### 2.3 Sampling geometry

Mirrors the production arrangement:

- a **quad** is a square patch of IC space, half-width `h`, containing `N×N` **nominal samples**;
- each nominal sample carries `E` **ensemble copies**, jittered within its pixel footprint;
- **jitter scales with cell size** (`jitter_frac × hx`). This is load-bearing: with a *fixed*
  perturbation, spreads would drift under refinement for a purely trivial reason.

Typical: `N=5..8`, `E=3..15`, `jitter_frac=0.5`. Refining a quad halves `h`, which halves both the
sample spacing and the jitter — as it does in production.

### 2.4 What "spread" means, per field type

| field type | measure | range |
|---|---|---|
| unit vector (shape n̂) | mean distance of copies from their centroid | `[0,2]` (chord) |
| continuous scalar | standard deviation (or relative std where noted) | field-dependent |
| discrete class | disagreement fraction `1 − modal/(E+1)` | `[0, 1−1/(E+1)]` |

Two dispersion measures were used for unit vectors and the difference matters: **spherical variance**
(`1 − |resultant|`) is *compressive* at large spreads, and **mean pairwise chord distance** is linear
in angular separation. Where scaling laws are being measured, the linear metric is used — the
compressive one manufactures apparent steepening (§4.2).

**Choosing a bounded metric space can manufacture saturation.** The shape sphere quotients out scale,
so it is bounded by construction and *will* saturate. This caused a real error (§7.1).

### 2.5 Regions

Regions were initially labelled by outcome class, which proved unsound (at `t=20` most samples had
not yet escaped, so labels reflected censoring rather than dynamics). Later work labels regions by
**measured local divergence rate λ**, estimated from pre-saturation ensemble growth.

The final sweep uses **8 regions across 3 slices**: near-field, deep interior, mid-field, far r≈10,
body-2 core, body-2 mid, body-1 slice, body-1 far. Earlier sections use 2–10 regions and say so.

### 2.6 Known methodological weaknesses

1. **Region count was load-bearing and did mislead** — see §7. Any conclusion resting on two regions
   is provisional.
2. **Softening breaks the scale gauge** (§7.5) — the single most serious weakness here. `eps=0.03`
   introduces a fixed length that does not transform under dynamical similarity, contaminating
   scale-quotiented measurements by up to **1.66× on a pure gauge transformation**, and the field
   *ranking* does not survive reducing it. Production Principia uses regularisation, not softening.
3. **Small ensembles** — `E=3..15` means discrete disagreement quantises coarsely, and two-time
   difference estimators amplify noise.
4. **One system** for everything except §4.2.
5. **`score = dynamics × exponent`** (§6) is a heuristic of mine for requiring both axes, not a
   derived or standard quantity.


---

## 3. Result — the variance ratio fails

### 3.1 It is flat

Three regions, four refinement levels (`h` halving each time, `N=8`, `E=3`, `t=20`):

| level | smooth | boundary | mixed |
|---|---|---|---|
| 0 | 0.606 | 0.644 | 0.628 |
| 1 | 0.639 | 0.660 | 0.625 |
| 2 | 0.607 | 0.627 | 0.629 |
| 3 | 0.665 | 0.604 | 0.639 |

**≈0.62 ± 0.03 everywhere.** No dependence on region, none on scale.

It does vary — but with the **playhead**, identically in all three regions:

| `t` | 1 | 2 | 4 | 8 | 16 |
|---|---|---|---|---|---|
| smooth | .991 | .991 | .978 | .795 | .653 |
| boundary | .991 | .991 | .991 | .968 | .655 |
| mixed | .991 | .991 | .991 | .980 | .616 |

**It measures the clock, not the quad.** Under lockstep this is fatal: refine everything early,
nothing late, uniformly across space.

**The components are not the problem.** Over a 5×5 grid of quads at fixed pre-saturation `t`:

| quantity | dynamic range | CV |
|---|---|---|
| `within` | **3844×** | 1.33 |
| `between` | **465×** | 1.04 |
| **ratio** | 0.58 – 0.97 | **0.13** |

Both components carry an order of magnitude more spatial signal than their quotient. **The division
destroys the information.**

### 3.2 Why — the exponential cancels

```
within  ≈ δ_jitter  · e^(λ_local · t)
between ≈ δ_spacing · e^(λ_local · t)

ratio ≈ δ_spacing / (δ_spacing + δ_jitter)      -- dynamics cancel; the SSAA knob remains
```

Confirmed on three independent legs, each matching prediction quantitatively.

**(a) Ratio tracks the jitter knob.** `between` is *exactly* invariant under jitter (a correctness
check on the harness — jitter cannot affect nominal samples); `within` scales ≈×1.85 per doubling:

| jitter frac | smooth | boundary | mixed |
|---|---|---|---|
| 0.10 | 0.916 | 0.995 | 0.999 |
| 0.25 | 0.855 | 0.979 | 0.991 |
| 0.50 | 0.773 | 0.932 | 0.965 |
| 1.00 | 0.639 | 0.836 | 0.879 |

**The ratio moves 0.92 → 0.64 in a fixed region with fixed physics, purely by changing an SSAA
parameter.**

**(b) `within` is exactly linear in spacing** (linear metric, `t=5`, well pre-saturation):

| spacing | 8.0e-2 | 4.0e-2 | 2.0e-2 | 1.0e-2 | 5.0e-3 |
|---|---|---|---|---|---|
| `within` | 1.312e-1 | 5.829e-2 | 2.882e-2 | 1.439e-2 | 7.191e-3 |
| factor | — | ×2.25 | **×2.02** | **×2.00** | **×2.00** |

(With the *spherical-variance* metric the same sweep gives ×2.28…×16.8 — that steepening is the
metric's compressive response, not physics. Hence §2.4.)

**(c) `within` is exponential in playhead** (fixed spacing):

| `t` | 4 | 6 | 8 | 10 | 12 | 14 |
|---|---|---|---|---|---|---|
| `within` | 1.68e-4 | 7.98e-4 | 2.65e-2 | 1.90e-1 | 2.60e-1 | 4.28e-1 |

Fit over pre-saturation points: `log(within) = 1.008·t − 12.55` → **λ ≈ 1.008 per time unit**.

### 3.3 It is not one box, one slice, or one chart

Seven Burrau regions, near-field to r≈25, across two perturbed bodies, labelled by measured λ:

| region | λ | `within(t=6)` | `within(t=12)` | ratio |
|---|---|---|---|---|
| near-field | 0.513 | 1.06e-02 | 2.31e-01 | 0.717 |
| deep interior | 0.002\* | 4.97e-01 | 5.04e-01 | 0.606 |
| mid-field | 0.156 | 2.30e-04 | 5.89e-04 | 0.967 |
| far r≈10 | 0.740 | 2.23e-08 | 1.89e-06 | 0.875 |
| very far r≈25 | 1.057 | 1.29e-10 | 7.29e-08 | 0.673 |
| body-2 slice | 0.384 | 2.93e-02 | 2.94e-01 | 0.681 |
| body-2, far | 0.898 | 9.07e-07 | 1.99e-04 | 0.983 |

λ spans **500×**, `within` spans **nine orders of magnitude**, ratio spans 0.61–0.98, and
**`corr(λ, ratio) = +0.215`** — no relationship.

\* *`λ`≈0 here is itself instructive: this region is already saturated at `t=6`, so the two-time
estimator returns ≈0 for the **most** chaotic region. Even measuring λ requires pre-saturation
sampling.*

**Second system** (equal masses, isoceles): `corr(λ, ratio) = +0.192`. The purest case: `far r≈8`
(λ=0.557) and `far r≈20` (λ=0.079) differ 7× in λ and three orders of magnitude in `within` — and
their ratios are **identical to three decimals, 0.984 both**.

---

## 4. Result — the exponential treadmill

Since `within = δ · e^(λt)`, holding a pixel below a display threshold `τ` requires

```
δ < τ · e^(−λ·t)
```

With measured `λ ≈ 1.008`, **the required spacing halves every ≈0.69 time units of playhead.**

This is architectural, not a tuning problem:

- "refine until pixels are well-defined" is **unreachable** in a chaotic region;
- the integration floor is **not** a static property of a region — it is a **front sweeping through
  IC space as `t` advances**;
- a quad legitimately refinable early becomes legitimately floored later, and the mechanism must
  express that as a function of `t`.

---

## 5. Result — saturation, and what it means for FTLE vs ensembles

### 5.1 They fail differently

- **Ensemble copies are free-running.** Spread grows as `δ·e^(λt)` only while separations stay small;
  once at system scale the measure is pinned. Observed pinning: 0.497 at `t=6`, 0.504 at `t=12`.
- **Benettin does not saturate, by construction.** Renormalisation pulls the shadow back to `d₀` every
  interval and accumulates log-stretch, so the separation never enters the nonlinear regime.

**This is the converse of a rule already in the architecture.** Benettin shadows cannot double as
ensemble samples (renormalisation corrupts endpoints); we now show **ensemble samples cannot double
as a Benettin shadow** (without renormalisation they saturate). `N` free-running copies plus one
renormalised shadow is **two instruments**, not redundancy.

### 5.2 The validity window, measured

λ recovered from a sliding two-time window on ensemble spread (saturation ceiling ≈0.5):

| `t` | 2 | 4 | 6 | 8 | 10 | 12 | 14 | 16 |
|---|---|---|---|---|---|---|---|---|
| **mid-field** spread | 3.7e-8 | 1.5e-6 | 2.3e-4 | 3.9e-5 | 2.8e-4 | 5.9e-4 | 5.7e-4 | 4.5e-4 |
| recovered λ | — | 1.853 | 2.522 | **−0.894** | 0.981 | 0.381 | −0.015 | −0.121 |
| **near-field** spread | 5.5e-6 | 6.0e-3 | 1.1e-2 | 1.6e-1 | 2.0e-1 | 2.3e-1 | 3.0e-1 | 3.3e-1 |
| recovered λ | — | 3.494 | 0.287 | 1.363 | 0.110 | 0.065 | 0.129 | 0.048 |
| **deep interior** spread | 2.1e-2 | 2.9e-1 | 5.0e-1 | 5.1e-1 | 5.7e-1 | 5.0e-1 | 5.4e-1 | 5.3e-1 |
| recovered λ | — | 1.309 | 0.264 | 0.014 | 0.054 | −0.061 | 0.034 | −0.013 |

1. **Saturated estimates are biased, not merely noisy** — a saturated region reports λ ≈ 0, i.e.
   **maximally chaotic reads as perfectly regular**.
2. **The window closes soonest where λ is largest** (`t_sat ≈ (1/λ)·ln(S/δ)`), so the tool fails first
   exactly where the signal is strongest.
3. **The spread is non-monotonic** — mid-field falls 2.3e-4 → 3.9e-5 between `t=6` and `t=8`, a 6×
   *decrease*. Diverge-then-reconverge, observed directly: independent empirical support for
   `running_max_divergence` being a **latching** field.

### 5.3 Consequences

- **Ensemble spread is a decoherence detector, not a chaos magnitude.** Post-saturation it still
  correctly answers *"is this pixel's value still determined?"*; it cannot rank *how* chaotic.
- **FTLE when you want λ itself**, at any playhead.
- **λ ≈ 1 bounds the renormalisation interval**: `d₀ · e^(λ·Δt)` must stay well below system scale.
- Cross-validation: the Benettin shadow gives FTLE ≈ 1.25 against λ ≈ 1.01 from pre-saturation
  ensemble growth — two unrelated methods agreeing on the divergence rate.


---

## 6. Result — field-by-field sweep

### 6.1 The replacement criterion: scaling exponent

For **discrete** fields, `within ≈ δ^α` where α is the classical **uncertainty exponent**, so
`ratio = 1/(1 + jitter_frac^α)`. This model fits the near-field data to **within 1%** at α = 0.488 —
which is *why* the ratio is knob-controlled, and why reading α directly from the scaling is strictly
better than reading it through a ratio.

α measured across regions (outcome disagreement vs perturbation size, four scales):

| region | w(δ=.125) | w(.25) | w(.5) | w(1.0) | **α** | reading |
|---|---|---|---|---|---|---|
| near-field | 0.0750 | 0.0925 | 0.1300 | 0.1750 | 0.416 | partly reducible |
| deep interior | 0.1950 | 0.2025 | 0.2525 | 0.1775 | **−0.009** | **refining futile** |
| mid-field | 0.0000 | 0.0000 | 0.0000 | 0.0275 | — | pure → keep coarse |
| far r≈10 | 0.0000 | 0.0000 | 0.0000 | 0.0000 | — | pure → keep coarse |
| body-2 slice | 0.1175 | 0.1075 | 0.1500 | 0.1925 | 0.262 | partly reducible |
| body-2, far | 0.0000 | 0.0200 | 0.0500 | 0.0975 | **1.143** | **refining works** |

α spans essentially its full theoretical range and lands the three-way decision directly. **α is not
a proxy** — it *is* the exponent governing how fast display uncertainty shrinks with resolution.

### 6.2 Exponents differ by field, and fields hand off

Scaling exponent per field, per playhead (near-field):

| field | t=3 | t=5 | t=7 | t=9 | t=12 | t=16 |
|---|---|---|---|---|---|---|
| shape n̂ | **1.003** | **0.990** | 0.651 | 0.367 | 0.268 | 0.140 |
| kinetic energy | **0.996** | 0.954 | 0.399 | 0.451 | 0.193 | 0.106 |
| max separation | 0.905 | 0.910 | 0.686 | 0.431 | 0.467 | 0.338 |
| `d_min` | 0.516 | 0.516 | 0.384 | 0.084 | 0.067 | 0.086 |
| outcome | *n/a* | *n/a* | *n/a* | 0.514 | 0.416 | 0.295 |

1. **The flow is smooth; the fractality is in the classification.** Smooth state fields start at
   exponent **1.00** — exactly linear response, as a smooth ODE map must give.
2. **Event-derived fields are fractal from `t=0`** — `d_min` sits at **0.516** at `t=3`, because a
   *minimum over the trajectory* inherits near-discontinuity from close encounters.
3. **Every exponent decays toward zero** — universal saturation, field-independent.
4. **The fields hand off.** Outcome is **undefined until `t≈9`**; by then the smooth fields have
   decayed to 0.37. **No single field carries the signal across the whole playhead.**

### 6.3 Amplification separates dynamics from the IC map

`A(t) = spread(t)/spread(t≈0)`:

| field | baseline @t≈0 | A(1) | A(3) | A(5) | A(9) | A(14) |
|---|---|---|---|---|---|---|
| **total energy** | 8.5e-3 | **1.00** | **1.00** | **1.00** | **1.04** | **1.06** |
| `L_z` | 1.0e-16 | 97 | 513 | 707 | 874 | 1760 |
| kinetic energy | 7.3e-6 | 508 | 1.5e4 | 1.6e5 | 1.2e6 | 2.1e6 |
| **shape n̂** | 1.9e-3 | 1.07 | 22.5 | 55.5 | 290 | **350** |
| `d_min` | 2.7e-7 | 743 | 3.8e4 | 3.8e4 | 5.7e4 | 5.7e4 |
| max separation | 6.2e-3 | 1.10 | 5.96 | 23.2 | 75.9 | 393 |

**Energy's amplification is 1.00 at every playhead** — its spread is `|∇E|·δ` and nothing else,
forever. Its scaling exponent is **1.001 at t=3**: a perfect "refining is working" reading produced
by a quantity that cannot change. Late, once integrator error exceeds the IC-induced difference, it
becomes erratic (−0.25, 0.49, 0.80). **False positive early, noise late.** `L_z` is degenerate for a
different reason: free-fall from rest has `L_z ≡ 0`, so its baseline is float noise.

### 6.4 The full sweep — 8 regions, 3 slices, both axes

| field | dynamics | map exponent | score (product) | discrimination | verdict |
|---|---|---|---|---|---|
| **`t_end`** | 0.326 | 0.587 | **0.191** | 585× | strong on both axes |
| **kinetic energy** | **0.422** | 0.448 | **0.189** | 137× | strong on both axes |
| **diffusion** | 0.188 | 0.590 | **0.111** | 1055× | strong on both axes |
| shape n̂ | 0.245 | 0.163 | 0.040 | 143× | usable; weak δ-response |
| event `class⊕detail` | 0.183 | 0.199 | 0.036 | 2× | usable; weak within chaotic regions |
| FTLE | 0.260 | **0.115** | 0.030 | 5× | **barely responds to δ** |
| `d_min` | **0.013** | 0.091 | 0.001 | 13× | **no dynamics — latches** |
| total energy | **0.001** | **0.963** | 0.001 | 14× | **no dynamics — pure map** |

### 6.5 Event grain — joint `class ⊕ detail` strictly dominates

| grain (near-field) | t=4 | t=6 | t=8 | t=10 | t=13 | t=16 |
|---|---|---|---|---|---|---|
| class only (bound vs escaping) | 0 | 0 | 0.0859 | 0.1328 | 0.1562 | 0.2188 |
| detail only (which pair is the binary) | 0.0469 | 0.0469 | 0.1797 | 0.2109 | 0.2578 | 0.3125 |
| **class ⊕ detail (joint)** | **0.0469** | **0.0469** | **0.2500** | **0.2969** | **0.3594** | **0.4609** |
| gain over better single grain | 0 | 0 | +0.0703 | +0.0859 | +0.1016 | **+0.1484** |

Never below `max(class, detail)`, exceeding it by up to **+0.148** (47% over detail alone). Copies
can agree on *whether* something escaped while disagreeing on *which*, and conversely — both are
genuine determinacy failures because both would be coloured differently. **And it does not
over-report**: in the determinate region the joint measure collapses to the class value exactly.

**This dissolves a separate spec ambiguity.** `dominant_outcome` stored at `class|detail` while
`outcome_impurity` was defined on the class fraction required an unsound masked accessor (argmax does
not commute with masking) and a `dominant_class` companion field. **Define every event-derived
reduction field at the joint grain and there is only one grain** — no commuting problem, no companion
field. A deletion, not an addition.

### 6.6 Event class vs outcome, and `t_end`

| field | t=2 | t=4 | t=6 | t=8 | t=10 | t=13 | t=16 |
|---|---|---|---|---|---|---|---|
| **event class** (tightest pair) | 0 | **0.0469** | 0.0469 | 0.1797 | 0.2109 | 0.2578 | **0.3125** |
| **`t_end`** (÷ horizon) | 0 | 0 | 0 | 0.0062 | 0.0467 | 0.0913 | **0.1242** |
| shape n̂ | 0.0058 | 0.1593 | 0.1013 | 0.5126 | 0.6021 | 0.6443 | 0.7199 |
| diffusion | 0.0030 | 0.0018 | 0.0223 | 0.0107 | 0.0197 | 0.0438 | 0.0542 |
| `d_min` | 0.0045 | 0.0100 | 0.0103 | 0.0156 | 0.0150 | 0.0162 | 0.0165 |
| *(escaped fraction)* | 0 | 0 | 0 | 0.156 | 0.430 | 0.484 | 0.602 |

**Event class needs no gate** — defined at every playhead, non-zero from `t=4`, four time units
before outcome. It also **subsumes outcome**: the escaping body is the one not in the tight pair.
**`t_end` is the strongest continuous discriminator** (585× in the full sweep); escape time is the
classic fractal quantity of this problem, and `d_min` was a poor proxy for it.

**The feared seam does not exist.** By `t=8`, when outcome first goes non-zero, n̂ is already at 0.51.
Low early values are **correct** — at `t=2` the copies genuinely have not diverged.


---

## 7. Corrections — verdicts that were overturned

Recorded deliberately. Three of these were **two-region calls that flipped when retested at scale**,
which is the single most important methodological lesson in this document.

### 7.1 A bounded metric manufactured its own saturation

§3–§5 measured a *continuous shape-sphere dispersion*; the spec defines ensemble spread over
**outcomes**, a discrete quantity. The shape sphere quotients out scale, so it is bounded by
construction and *will* saturate regardless of dynamics.

**Rejection of the ratio survived** (both metric families reject it), but the **positive replacement
— the uncertainty exponent — only emerges under the correct definition**. The wrong metric hid the
answer.

### 7.2 FTLE: excluded on 1.2×, which was an unlucky pair

| region set | FTLE spread discrimination |
|---|---|
| 2 regions, 1 slice | 1.2× → *excluded* |
| 10 regions, 3 slices | **18.8×** |
| 8 regions, 3 slices (final sweep) | 5× |

A methodological fix was applied at the same time — every trajectory now uses the **same** shadow
direction, so FTLE differences reflect IC differences rather than the arbitrary perturbation
direction — but it changed almost nothing (1.07× → 1.11× on the original pair). **The flaw was the
sample, not the method.**

**The correct objection to FTLE is different and stronger: map exponent 0.115.** Its spread barely
responds to perturbation size, so **refining a quad cannot reduce it**. It is dominated by intrinsic
finite-time fluctuation rather than by the IC difference. A quantity refinement cannot lower does not
belong in a measure whose purpose is to drive refinement.

**But there is a real counter-argument, and the data does not settle it.** In six of ten regions both
other contributors read **exactly 0** while FTLE spread reads 0.18–0.33. Without it,
`ensemble_spread` declares those pixels fully determined — yet FTLE is a **displayable production
field**, and its value there *is* uncertain.

- **Include** if `ensemble_spread` must cover *every displayable field* — then the floor it imposes
  (≳0.1 across much of the domain) is a true statement, not a defect.
- **Exclude** if it is specifically a *refinement-relevant* determinacy measure — a floor that
  refinement cannot lower is what the integration floor already expresses, and duplicating it inside
  the spread scalar muddies both.

The second matches the stated purpose; the first matches the scalar's *name*. **This is a
specification question, not an experimental one.**

### 7.3 Kinetic energy: excluded for "inverting", also a two-region artefact

Excluded because amplification appeared to invert between regions (2.1e6 near-field vs 1.2e7
mid-field). Across 8 regions it has the **highest dynamics score of any field (0.422)** and ranks
second overall by combined score. **Reconsider as a contributor** — its practical obstacle is the
lack of a bounded domain to normalise against, not its information content.

### 7.4 Confirmed at scale (not overturned)

- **`d_min`** — dynamics score **0.013**, flat in `t` because a running minimum latches. Also its
  magnitude peaks at 0.0165, which after normalisation against `[0,2]` is **0.008**: it can never win
  a `max`. Excluded on both counts.
- **Conserved quantities** — energy's amplification ≡ 1.00 across all regions tested.

### 7.5 Softening breaks the scale gauge — and contaminates the field rankings

**The objection.** Newtonian gravity obeys dynamical similarity (`r → αr`, `t → α^{3/2}t`) — the
scale gauge Principia quotients out. Softened gravity, `(|r|² + ε²)^{-3/2}`, introduces a **fixed
length** that does not transform, so the softened dynamics are **not scale-invariant**. Production
Principia will use regularisation (Levi-Civita / global), never softening.

**Gauge test.** The shape-sphere metric is scale-quotiented, so rescaling the whole configuration
must leave its spreads unchanged. Near-field, `t=10`:

| α | ε scaled with α (gauge kept) | ε fixed (gauge broken) |
|---|---|---|
| 0.50 | **0.58166** | 0.51401 |
| 1.00 | **0.58166** | 0.58166 |
| 2.00 | **0.58166** | 0.85506 |
| 4.00 | **0.58166** | 0.73722 |

With ε scaled the result is **identical to five decimals across an 8× range of system size** — which
confirms three things at once: dynamical similarity is exactly implemented, the shape-sphere metric
is genuinely scale-quotiented, and **softening is the only symmetry breaker in the harness**.

With ε fixed, the same physical system reads **0.514 → 0.855** — a **1.66× swing produced by a pure
gauge transformation**. Direction is as expected: larger α makes ε relatively smaller, resolving
closer encounters, raising sensitivity.

**ε-convergence does not converge smoothly.** Near-field, `t=8`, `dt ~ ε^{3/2}`:

| ε | dt | within | between | ratio | median drift |
|---|---|---|---|---|---|
| 0.030 | 5.0e-4 | 0.46953 | 0.94160 | 0.667 | 4.0e-05 |
| 0.010 | 1.0e-4 | 0.08991 | 0.32580 | 0.784 | 1.2e-09 |
| 0.005 | 3.5e-5 | 0.32107 | 0.79155 | 0.711 | 1.0e-10 |
| 0.003 | 1.6e-5 | 0.01923 | 0.07146 | 0.788 | 1.5e-11 |

Components vary **25×** and non-monotonically; the drift columns show this is *not* an integration
error (1.5e-11 at the smallest ε) but genuine sensitivity — changing ε changes the force law, and in
a chaotic system that yields a different realisation, not a perturbed one. The **ratio** is
comparatively stable (0.667–0.788), consistent with §3.2: it is a geometric constant and does not
care about the dynamics.

**The field ranking does not survive.** Same region, both axes, at two softenings:

| field | score @ ε=0.03 | score @ ε=0.01 |
|---|---|---|
| `t_end` | **0.480** | *undefined — no escapes by t=11* |
| kinetic energy | 0.255 | **0.433** |
| event class | 0.069 | *undefined* |
| shape n̂ | 0.056 | 0.202 |
| diffusion | 0.050 | **0.000** |
| FTLE | 0.007 | 0.000 |
| `d_min` | 0.003 | 0.001 |
| total energy | 0.003 | 0.001 |

**What is robust across ε, and what is not:**

| conclusion | status |
|---|---|
| total energy is a pure map (dynamics ≈0, exponent ≈0.91–0.96) | **robust** — holds at both ε |
| `d_min` has no dynamics (0.054 → 0.002) | **robust** |
| FTLE is weak (0.007 → 0.000) | **robust** |
| kinetic energy is strong (0.255 → 0.433) | **robust** |
| `t_end` / event class / diffusion rankings | **fragile — ε-dependent** |

So the **exclusions** (§8) rest on ε-robust evidence; the **ordering among included fields** does not.
§6.4's ranking must be treated as provisional until rerun with regularisation.

**Also note:** all *structural* results — the algebraic cancellation (§3.2), the treadmill (§4), the
conserved-quantity signature (§6.3) — are ε-independent, because they are algebra rather than
measurement. Softening changes the numbers fed into them, not their form.

### 7.6 Rerun without softening — gauge restored, and three further findings

Open question 8 addressed. Softening was removed entirely (`eps = 0`) and close encounters resolved
by **adaptive timestepping** instead. The criterion

```
dt = eta * min_ij ( r_ij^{3/2} / sqrt(G (m_i + m_j)) )        -- local two-body free-fall time
```

is **scale-covariant**: under `r → αr` it scales as `α^{3/2}`, exactly as `t` does. No fixed length
or time enters, so the gauge survives. Drift converges as ~`eta³` (3.0e-2 → 3.3e-5 for
`eta` 0.05 → 0.005).

**Finding 1 — the gauge is restored to floating-point precision.** Rescaling the configuration with
**no parameter adjusted** (`eta` is dimensionless):

| quantity | gauge violation, softened `eps=0.03` | gauge violation, `eps=0` |
|---|---|---|
| shape spread | **4.9e-01** | **1.2e-09** |
| diffusion | — | **2.4e-12** |
| FTLE | — | **4.0e-04** \* |

\* *residual alternates between exactly two values with α, which is floating-point in the `α^{3/2}`
scaling (0.25/1/4 are exactly representable, 0.5/2 are not) — not physics.*

**Two further gauge breaks were found and fixed in the FTLE implementation itself**, neither related
to softening:

1. **The Benettin separation metric `|Δr|² + |Δv|²` is not scale-covariant** — `Δr ~ α` but
   `Δv ~ α^{-1/2}`, so the naive sum mixes two scalings. Measured swing: **25%**. Fixed by
   normalising to a dimensionless separation using the hyperradius `R` and its velocity scale
   `sqrt(GM/R)`.
2. **The shadow was seeded at an absolute offset `d₀`**, making the first interval contribute
   `−log(α)`. This appeared as a constant *additive* FTLE offset of `log(α)/T` — verified
   quantitatively: observed 0.0866 per doubling, predicted `log(2)/8 = 0.0866`. Fixed by seeding at
   a dimensionless separation.

**Finding 2 — softening cannot simply be removed; regularisation is not optional.** Free-fall-from-
rest ICs give near-radial orbits, so with `eps = 0` several regions hit **genuine collision
singularities**: `dt → 0`, the timestep floor bites, and energy conservation fails outright.

| region | time | median drift | fraction floored |
|---|---|---|---|
| near-field | 4.3s | 5.0e-04 | 2% |
| **deep interior** | **70.7s** | **3.7e+01** | **94%** |
| mid-field | 6.2s | 9.8e-03 | 100% |
| **far r≈10** | 0.3s | **5.1e+07** | 100% |
| body-2 core / mid, body-1 slice / far | 2–6s | ≤1.3e-03 | 0–16% |

**Softening was masking real singularities.** Three of eight regions are unrunnable. This is a direct
argument for the planned Levi-Civita / global regularisation: it is not an accuracy refinement, it is
what makes a general sweep possible at all.

**Finding 3 — the field ranking does not survive, confirming §7.5.** Like-for-like on the five
collision-free regions:

| field | softened dyn | softened exp | softened score | exact dyn | **exact exp** | exact score |
|---|---|---|---|---|---|---|
| kinetic energy | 0.389 | 0.500 | 0.195 | **0.476** | **0.995** | **0.474** |
| shape n̂ | 0.236 | 0.176 | 0.041 | 0.192 | **0.944** | 0.181 |
| diffusion | 0.243 | 0.326 | 0.079 | 0.109 | **1.000** | 0.109 |
| FTLE | 0.120 | 0.145 | 0.017 | 0.132 | 0.757 | 0.100 |
| `d_min` | −0.012 | 0.119 | 0.000 | 0.130 | 0.337 | 0.044 |
| `t_end` | **0.570** | 0.587 | **0.335** | 0.135 | 0.306 | 0.041 |
| total energy | 0.002 | 0.981 | 0.002 | 0.017 | 0.999 | 0.017 |
| event class | 0.183 | 0.241 | 0.044 | *n/a* | 0.371 | *n/a* |

- **Pairwise order agreement: 16/28** — chance is 14/28. The ranking is essentially uncorrelated.
- **Map exponents rise to ≈1 almost universally** without softening (shape 0.18→0.94, KE 0.50→1.00,
  diffusion 0.33→1.00). The fields sit in the **linear-response regime** at `t=13`; softening was
  inflating apparent saturation. Part of that may be numerical — the softened runs used a *fixed* dt
  whose max drift reached 9.2e-3, so under-resolved encounters could masquerade as divergence.
- **`t_end` collapses** (0.335 → 0.041): with exact gravity far fewer escapes occur by `t=13`, so it
  is mostly censored.
- **Event class reads zero disagreement** throughout the clean regions.

**What is robust across softened *and* exact:**

| conclusion | status |
|---|---|
| total energy is a pure map — exclude | **robust** (exponent 0.98 / 1.00, dynamics ≈0 in both) |
| kinetic energy is strong | **robust** — top-ranked in both |
| everything else | **provisional** — awaits regularisation |

**Caveat.** The five comparable regions are the *collision-free* subset, which is biased toward tamer
dynamics. The softened-vs-exact comparison is like-for-like on identical regions and so is valid; but
generalising either ranking to the whole IC space is not, until regularisation makes the singular
regions runnable.

### 7.7 Levi-Civita regularisation — built, validated, and its limit measured

Built in response to §7.6, which showed a general sweep is impossible without regularisation.

**Construction.** The close pair `(i,j)` is regularised; the third body is a perturber.
Coordinates `rho = r_j − r_i`, `lam = r_k − COM(i,j)`. Planar LC is complex squaring,
`rho = L(u)u` with `|rho| = |u|²`, plus the time transformation `dt = |rho| dtau`. The
regularised Hamiltonian `Gamma = |rho|(H − E)` is

```
Gamma = |p_u|²/(8 mu_rho) + |u|²[ |p_lam|²/(2 mu_lam) + V_pert − E ] − G m_i m_j
```

**The singular term `−G m_i m_j/|rho|` has become a constant.** `Gamma` and its derivatives are
finite at `u = 0`, so the pair passes through exact collision. `Gamma` is not separable (the
`|u|²|p_lam|²` term couples `u` and `p_lam`), so RK4 in `tau` is used rather than leapfrog.
Uniform `dtau` is correct here — in LC coordinates the regularised pair *is* a harmonic
oscillator — and `dtau = eta·sqrt(|rho|/Gm_ij)` scales as `sqrt(alpha)`, keeping the gauge.

**Validation — exact two-body radial collision** (equal masses released from rest, so the orbit
falls straight through `r = 0`; analytic collision time `pi/2·sqrt(d³/2mu) = 0.785`):

| n_steps | `d_min` reached | energy drift |
|---|---|---|
| 5 000 | 1.35e-11 | **6.2e-15** |
| 20 000 | 1.35e-11 | 2.0e-14 |
| 80 000 | 1.35e-11 | 9.5e-14 |

Passage through collision at **machine precision**. This is the property no amount of softening
or timestep reduction can buy.

**Single-pair LC is not enough.** It regularises only its own pair; a close approach of either
other pair still enters through the unregularised perturbation term:

| region | unsoftened | LC on pair (1,2) |
|---|---|---|
| far r≈10 | 5.1e+07 | **2.3e-12** ✓ |
| mid-field | 9.8e-03 | **3.2e-07** ✓ |
| near-field | 5.0e-04 | **1.7e+01** ✗ *(made worse)* |
| deep interior | 3.7e+01 | 7.7e+01 ✗ |

**Pair switching** (`tb_lc_switch.py`) holds the state in Cartesian form at sync boundaries,
identifies the closest pair per trajectory, and regroups the batch so each subgroup integrates in
its own pair's LC coordinates:

| region | softened | unsoftened | LC fixed-pair | **LC + switching** | switches |
|---|---|---|---|---|---|
| far r≈10 | n/a | 5.1e+07 | 2.3e-12 | **6.9e-11** | 0 |
| near-field | 4.0e-05 | 5.0e-04 | 1.7e+01 | **1.8e-05** | 6 |
| body-2 core | 5.0e-04 | 5.0e-04 | n/a | **6.4e-05** | 6 |
| mid-field | 9.8e-03 | 9.8e-03 | 3.2e-07 | **nan** | 2 |
| deep interior | n/a | 3.7e+01 | 7.7e+01 | **nan** | 4 |

**The limit, measured.** The two failures overflow in the *perturbation* term (`d1/n1³` with
`n1 → 0`) — the third body reaching the regularised pair. That is a genuine **triple encounter**:
two pairs close simultaneously, which a single regularised pair cannot represent **however you
switch**. Switching handles *sequential* binary encounters; it cannot handle *concurrent* ones.

**This confirms the planned architecture empirically.** The design says "Levi-Civita for the planar
binary; global regularisation for the triple", and that division is exactly what the measurements
show: LC is necessary and sufficient for binary encounters (machine-precision collision passage),
and strictly insufficient for triple encounters. **Global (Aarseth–Zare / chain) regularisation,
which regularises two pairs simultaneously, is required** — not as an optimisation but to make
those regions runnable at all.

Cost note: `deep interior` took 119 s even while failing — the regions with the deepest encounters
are both the most expensive and the ones that need global regularisation most.

### 7.8 Aarseth–Zare global regularisation — built and verified

§7.7 showed single-pair LC cannot survive a triple encounter however you switch. AZ regularises
**two** pairs at once.

**Construction.** Reference body `a` = the one **not in the longest side**, so both regularised pairs
share it: `R1 = r_b − r_a`, `R2 = r_c − r_a`, and the unregularised side is `R3 = r_c − r_b`. Because
`(b,c)` is the longest side, `|R3| ≥ max(|R1|,|R2|)` — so `R3 → 0` **only at genuine triple
collision**, which is provably non-regularisable anyway. Everything else is covered.

`R1, R2` are relative-to-a-common-body, not Jacobi, so the kinetic energy carries a cross term:

```
T = P1²/(2 mu1) + P1·P2/m_a + P2²/(2 mu2),    1/mu1 = 1/m_a + 1/m_b,  1/mu2 = 1/m_a + 1/m_c
```

(derived by inverting the velocity-space mass matrix, determinant `m_a m_b m_c / M`; verified — the
AZ energy matches the Cartesian energy to **0.00e+00** and the coordinate round-trip to 4.4e-16).

LC on both, `dt = |R1||R2| dtau = A·B·dtau` — the time transformation vanishes when **either** pair
closes. The regularised Hamiltonian

```
Gamma = B|p1|²/(8 mu1) + A|p2|²/(8 mu2) + (L1p1)·(L2p2)/(4 m_a)
        − m_a m_b B − m_a m_c A − A B m_b m_c/|R3| − E A B
```

turns **both** `−m_a m_b/|R1|` and `−m_a m_c/|R2|` into terms linear in `B` and `A`. Nothing is
singular unless `|R3| → 0`.

**Results** (t=13, 64 trajectories):

| region | softened | unsoftened | LC + switch | **AZ global** |
|---|---|---|---|---|
| far r≈10 | n/a | 5.1e+07 | 6.9e-11 | **9.0e-08** |
| mid-field | 9.8e-03 | 9.8e-03 | **nan** | **2.8e-09** |
| near-field | 4.0e-05 | 5.0e-04 | 1.8e-05 | **6.0e-11** |
| body-2 core | 5.0e-04 | 5.0e-04 | 6.4e-05 | **8.9e-11** |
| body-2 mid | 6.1e-05 | 6.1e-05 | n/a | **1.4e-08** |
| body-1 slice | 5.5e-04 | 4.9e-04 | n/a | **9.5e-11** |
| body-1 far | 1.3e-03 | 1.3e-03 | n/a | **1.1e-10** |
| deep interior | n/a | 3.7e+01 | **nan** | 3.0e-07 median, **1.0e+03 max** |

**7 of 8 regions clean at 1e-8 to 1e-11** — against 5/8 unsoftened and 3/5 for LC+switching. Drift
converges cleanly with step size (near-field: 4.1e-09 → 2.8e-11 → 1.2e-11).

**Two bugs found, both worth recording.**

1. **Sign errors in `∂Gamma/∂u`.** Drift sat at ~1 and would *not* fall with step size — the signature
   of a wrong equation, not a step-size problem. A finite-difference check against `Gamma` localised
   it exactly: the `du/dtau` terms matched to 1e-10, the `dp/dtau` terms were off by O(1). Both `R3`
   derivative signs were flipped (and swapped between `u1` and `u2`), because `dR3/du1 = −2L1` while
   `dR3/du2 = +2L2`. After the fix, all components match finite differences to ~1e-9 across random
   states. **Verify regularised equations of motion by finite-differencing the Hamiltonian — the
   algebra is error-prone and the failure is silent.**
2. **The step-size rule defeated the regularisation.** `dt = A·B·dtau`, so the `A·B` factor *already*
   shrinks the physical step at close approach — that is precisely what regularisation buys. Shrinking
   `dtau` as well drove `dt → 1e-13` and exhausted the step budget. **The original deep-interior
   failure was this, not the physics.** Fixed by sizing `dtau` so the first physical step is a fixed
   fraction of the interval.

**Residual.** `deep interior` now converges in the median (1.7e-05 → 3.0e-07 as `eta` falls) but a
minority of trajectories still blow up (max drift 1.0e+03). Diagnostics: its closest approach reaches
**1.6e-07** while the longest side falls to **0.27** — a deep near-triple encounter, far tighter than
near-field (8.5e-03 / 2.13) or mid-field (1.3e-04 / 1.22).

**This is where the integrator stops and classification takes over.** Per
`principia_spec_pending_changes.md` change 7, a triple encounter is an **outcome to detect and
terminate on** (`state = collision, detail = 11`), not a trajectory to integrate through. The correct
handling of these samples is to flag them, not to chase them with smaller steps — the residual is a
missing *outcome class*, not a missing integrator.

### 7.9 The sweep, rerun with AZ — the ranking survives

Seven regions across three slices, all at drift **1.8e-09 to 2.2e-07**, gauge intact, encounters
survivable. (`deep interior` excluded: it is the near-triple case pending-change 7 says to classify
and terminate, and it costs >190 s per probe against 5 s for the rest.)

| field | dynamics | map exponent | score | discrimination | softened score | verdict |
|---|---|---|---|---|---|---|
| **`t_end`** | 0.183 | 1.207 | **0.221** | 2× | 0.393 | strong |
| **kinetic energy** | 0.130 | 1.098 | **0.143** | 9258× | 0.228 | strong |
| **diffusion** | 0.107 | 0.983 | **0.105** | 77× | 0.176 | strong |
| shape n̂ | 0.092 | 0.851 | 0.078 | 115× | 0.045 | usable |
| event `class⊕detail` | *n/a* | 0.687 | *n/a* | 2× | 0.044 | usable |
| total energy | **0.000** | **1.000** | 0.000 | 34629× | 0.002 | **no dynamics** |
| FTLE | **−0.127** | 0.198 | 0.000 | 3× | 0.058 | **no dynamics** |
| `d_min` | **−0.019** | 1.004 | 0.000 | 4407× | 0.005 | **no dynamics** |

**The top three are identical to the softened sweep — `t_end` > `KE` > `diffusion`.** Pairwise
agreement 18/28 against 14/28 for chance. The earlier "ranking does not survive" (§7.6, 16/28) was
comparing against the *collision-free subset* of an integrator that was itself failing; with the
gauge intact and encounters handled, the ordering is stable.

**Three exclusions now settled, not provisional:**

- **Total energy** — dynamics **exactly 0.000**, map exponent **exactly 1.000**. The textbook pure-map
  signature, cleaner here than anywhere: it responds perfectly to `delta` and not at all to time.
- **FTLE** — dynamics **negative** (−0.127): its spread does not grow with playhead at all under
  correct integration. §7.2's 18.8× discrimination and §5.10's coverage argument were **softening
  artefacts**. This closes open question 1 empirically rather than by specification.
- **`d_min`** — dynamics −0.019, confirming the latching argument at full accuracy.

**Kinetic energy is reinstated** (§7.3): third-highest dynamics, exponent 1.098, and the widest
discrimination of any dynamical field (9258×). Its only obstacle remains the lack of a bounded domain.

### 7.10 Closed-loop validation — does the exponent predict the refinement gain?

Open question 5. The exponent `alpha` is measured at the **parent** scale; the quad is then actually
refined (half-width halved, which halves both spacing and jitter) and the realised spread ratio is
compared against the prediction `S_child/S_parent = (1/2)^alpha`.

| region | field | `alpha` | predicted | actual | log error |
|---|---|---|---|---|---|
| near-field | shape | 0.839 | 0.559 | 0.472 | **0.168** |
| near-field | KE | 1.098 | 0.467 | 0.394 | **0.170** |
| near-field | diffusion | 0.723 | 0.606 | 0.490 | **0.212** |
| near-field | **E (control)** | 0.998 | 0.501 | 0.500 | **0.002** |
| body-1 slice | shape | 1.121 | 0.460 | 0.380 | **0.191** |
| body-1 slice | diffusion | 0.962 | 0.513 | 0.412 | **0.219** |
| body-1 slice | **E (control)** | 1.000 | 0.500 | 0.500 | **0.000** |
| body-2 mid | shape | 0.887 | 0.541 | 0.428 | **0.235** |
| body-2 mid | diffusion | 0.983 | 0.506 | 0.513 | **0.014** |
| body-2 mid | **E (control)** | 1.000 | 0.500 | 0.500 | **0.001** |
| body-2 mid | `t_end` | 1.000 | 0.500 | **0.000** | 68.4 |
| body-2 core | **E (control)** | **6.691** | 0.010 | 0.000 | 3.52 |
| body-2 core | (all fields) | 2.9–6.7 | — | — | 1.4–68 |

**Verdict: the decision rule works, and the weak link is the exponent *estimator*, not the criterion.**

- **The control validates the methodology.** Total energy must have `alpha = 1` exactly (its spread is
  `|grad E|·delta`), so refinement must halve it. Measured `alpha` = 0.998 / 1.000 / 1.000 and
  predicted-vs-actual agrees to **0.002 / 0.000 / 0.001**. A test that can recover a known answer to
  three decimals is measuring what it claims to.
- **Where the exponent is well estimated, prediction is good** — shape and diffusion land within
  ~20% (log error 0.014–0.235) across three regions.
- **`body-2 core` fails wholesale, and the control says why.** Energy's exponent read **6.691** there
  instead of 1.0 — physically impossible, so the *estimate* is broken, not the prediction. Every
  field's prediction failed with it. Three jitter samples are too few for a stable slope when the
  spreads are non-monotonic.
- **`t_end` fails through censoring** — the realised ratio is exactly 0.000 because no samples escape
  at the child scale. This is open question 3, now demonstrated to break the closed loop rather than
  merely being untidy.

**Consequences.**

1. **The exponent estimator needs to be robust, and is the thing to engineer.** In production it comes
   from a parent–child comparison (two scales) — fewer samples than the three used here, so this is a
   real risk. Options: more levels, a fitted trend across the hierarchy, or rejecting estimates whose
   control-field exponent departs from 1.
2. **Carry a sanity field.** Total energy has a *known* exponent of 1.0. Measuring it alongside gives a
   free, per-quad validity check on the estimator — if `alpha_E` is not ≈1, the other exponents from
   that quad are untrustworthy. **The field excluded for having no dynamics turns out to be the ideal
   control.**
3. **`t_end` should not enter until censoring is resolved**, despite topping the ranking on the static
   sweep.

### 7.11 The exponent estimator — the failure was integration, not fitting

§7.10 left "build a robust estimator" as the top task, after total energy returned `alpha = 6.691`
where its true value is exactly 1.0. Six estimators were compared — 2/3/5-point OLS, Theil–Sen,
median-of-adjacent-ratios, and a clamped 2-point — scored against energy, whose exponent is known
analytically (spread `= |grad E|·delta`, so linear in every region at every playhead) and therefore
needs no reference run.

**Every estimator failed in the same region, which pointed at the data.** The raw energy spreads for
`body-2 core`:

| jitter | 0.125 | 0.25 | **0.5** | 1.0 | 2.0 |
|---|---|---|---|---|---|
| E spread | 5.43e-03 | 1.09e-02 | **3.78e+01** | 1.17e+02 | 1.66e+02 |

A **3400× jump** where linear scaling demands 2×. That is not a fitting problem. Diagnosing further:

| jitter | E spread | max drift | fraction with drift > 1e-3 |
|---|---|---|---|
| 0.125 | 5.43e-03 | 2.4e-05 | 0.00 |
| 0.25 | 1.09e-02 | 4.2e-04 | 0.00 |
| **0.5** | **3.78e+01** | **1.4e+02** | **0.02** |
| 1.0 | 1.17e+02 | 3.1e+02 | 0.02 |

**The spread tracks integration drift, not jitter.** Two percent of trajectories failed to integrate,
and their garbage values dominated the standard deviation. The estimator was working perfectly — it
was faithfully fitting integration failure.

**The fix is a per-trajectory drift gate, not a better fit.** Excluding trajectories with
`drift > 1e-3` *before* reducing (requiring ≥3 survivors per footprint):

| jitter | 0.125 | 0.25 | 0.5 | 1.0 | 2.0 |
|---|---|---|---|---|---|
| E spread, gated | 5.43e-03 | 1.09e-02 | **2.17e-02** | 4.37e-02 | 8.72e-02 |
| fraction kept | 1.00 | 1.00 | 0.98 | 0.98 | 0.97 |

Clean factor-of-2 scaling. **`alpha`: 6.691 → 0.9969.** Dropping 2% of trajectories moved the
estimate by a factor of nearly seven.

**With the gate, every estimator works — including the cheap one.** Error `|alpha − 1|` on energy:

| region | ols2 | ols3 | ols5 | Theil–Sen | ratio-median | clamped2 |
|---|---|---|---|---|---|---|
| near-field | 0.0011 | 0.0005 | 0.0018 | 0.0001 | 0.0005 | 0.0011 |
| body-2 core | 0.0031 | 0.0041 | 0.0010 | 0.0006 | 0.0024 | 0.0031 |
| body-1 slice | 0.0011 | 0.0008 | 0.0005 | 0.0004 | 0.0004 | 0.0000 |
| body-2 mid | 0.0007 | 0.0005 | 0.0003 | 0.0003 | 0.0003 | 0.0007 |
| **worst** | **0.0031** | 0.0041 | 0.0018 | **0.0006** | 0.0024 | 0.0031 |

**The two-point estimator — the only one production can afford — has worst-case error 0.0031.**
Robust fitting is unnecessary once the data is clean; Theil–Sen's marginal advantage (0.0006) does
not justify the extra samples. **Sample hygiene beats estimator sophistication.**

### 7.12 Closed loop, re-run with gating and the production estimator

§7.10 repeated with the drift gate and a strict two-point `alpha` (parent at two jitter scales):

| region | field | `alpha` | `alpha_E` (control) | predicted | actual | log error |
|---|---|---|---|---|---|---|
| near-field | **E** | 1.000 | 1.000 | 0.4999 | 0.5000 | **0.000** |
| near-field | shape | 0.868 | 1.000 | 0.548 | 0.472 | 0.149 |
| near-field | KE | 1.167 | 1.000 | 0.445 | 0.394 | 0.122 |
| near-field | diffusion | 0.633 | 1.000 | 0.645 | 0.490 | 0.274 |
| body-2 core | **E** | 1.011 | 1.011 | 0.4961 | 0.5012 | **0.010** |
| body-2 core | shape | 0.763 | 1.011 | 0.589 | 0.203 | 1.067 |
| body-1 slice | **E** | 1.001 | 1.001 | 0.4998 | 0.4997 | **0.000** |
| body-1 slice | diffusion | 1.279 | 1.001 | 0.412 | 0.412 | **0.001** |
| body-2 mid | **E** | 1.000 | 1.000 | 0.5001 | 0.4997 | **0.001** |
| body-2 mid | KE | 0.454 | 1.000 | 0.730 | 0.102 | 1.971 |

**All four quads now pass the energy sanity check** (`|alpha_E − 1| < 0.05`), against one of four
before gating. Energy's own closed loop is exact to **0.000–0.010** in every region.

**But per-field prediction remains scattered** — median log error 0.219, worst 1.971. So:

- **The sanity field validates the *estimator*, not per-field *predictability*.** A passing
  `alpha_E` means the fit machinery and the sample hygiene are sound; it does not mean an
  individual field's exponent will predict that field's refinement gain. Genuinely chaotic fields
  have non-power-law scaling that no estimator can rescue.
- **Refinement decisions should therefore be treated as ordinal, not quantitative.** "This quad's
  exponent is high, refine it" is supported by the evidence; "refining will reduce spread by exactly
  `2^-alpha`" is not.

**Adopted:**

1. **Per-trajectory drift gate before any reduction.** `drift > 1e-3` excluded, require ≥3 survivors
   per footprint. This is not an accuracy nicety — ungated, 2% bad trajectories moved an exponent by
   a factor of seven. `QuadReduction` already carries `worst_energy_drift`; this makes it *load
   bearing* rather than diagnostic.
2. **Two-point OLS** as the estimator. Worst-case error 0.0031 on the control; no case for anything
   more elaborate.
3. **Total energy as a per-quad sanity field.** Excluded from `ensemble_spread` for having no
   dynamics (§6.3), it is the ideal *control*: known exponent, free to compute, and it detects
   exactly the failure mode that broke §7.10.
4. **Ordinal use of the exponent only** — rank quads by it; do not use `2^-alpha` as a quantitative
   forecast.

### 7.13 Q3, Q4, Q6 — set membership settled

Three remaining questions, all about *which fields belong in `ensemble_spread`* rather than whether
the criterion works. All tested under AZ with the §7.11 drift gate.

**Q3 — `t_end` censoring: the field is unusable, and worse than "untidy".** Three candidate measures
were tried (raw std with censored values pinned at the horizon; std over escaped copies only;
disagreement on *whether* escape occurred). All are undefined, because at `t = 13`:

| region | censored fraction | raw parent | raw child |
|---|---|---|---|
| near-field | **1.00** | 0.0000 | 0.0000 |
| body-2 core | **1.00** | 0.0000 | 0.0000 |
| body-1 slice | **1.00** | 0.0000 | 0.0000 |
| body-2 mid | 0.01 | 0.0116 | 0.0000 |

**Nothing escapes at all** in three of four regions. This is not a measure-design problem — there is
no signal to measure. And it reframes §7.9: `t_end` **topped the static ranking (score 0.221) on
essentially no data**, its 2× discrimination reflecting near-uniform zeros rather than structure.

Longer horizons (`t` = 30, 60) proved too expensive to test here, which is itself the answer for a
*live* signal: a field that needs many multiples of the current horizon before it acquires any
variance cannot inform refinement at the playhead. **`t_end` is excluded** — not deferred pending a
censoring fix.

**Q4 — diffusion does not earn its place.** Per-footprint contributions, normalised to `[0,1]`, with
the fraction of footprints where each field wins the `max`:

| region | level | shape | event | **diffusion** | KE | winner share (shape/event/diff/KE) |
|---|---|---|---|---|---|---|
| near-field | parent | 0.0094 | 0.0000 | 0.0036 | **0.0644** | 0.00 / 0.00 / 0.00 / **1.00** |
| body-2 core | parent | 0.0381 | 0.1199 | 0.0081 | **0.2369** | 0.00 / 0.25 / 0.00 / **0.75** |
| body-1 slice | parent | 0.0163 | 0.0804 | 0.0021 | **0.1863** | 0.00 / 0.31 / 0.00 / **0.69** |
| body-2 mid | parent | 0.0159 | 0.0000 | 0.0112 | **0.3189** | 0.00 / 0.00 / 0.06 / **0.94** |

**Diffusion wins the `max` in 6% of footprints in one region and 0% everywhere else.** Its magnitude
is consistently an order below KE. Since `ensemble_spread` is a `max`, a contributor that never wins
contributes nothing. **Excluded** — it remains a good debug view.

Note this also retires the argument for keeping it (§5.9): it was retained for *graded resolution*
where event class quantises in `1/(E+1)` steps. KE provides that grading, continuously, and wins.

**Q6 — contributor switching is real but benign.** The `max` winner does change between parent and
child (body-2 core 0.75 → 0.81 KE; body-1 slice 0.69 → 0.81), so an exponent computed across levels
can mix contributors. Two things make this tolerable:

- **The switch is between the same two fields** (KE and event class), not a scramble across all four,
  and both are legitimate determinacy signals.
- **§7.12 already concluded the exponent should be used ordinally**, not as a quantitative forecast.
  A mixed-contributor exponent is a worse *number* but usually the same *ranking*.

**Mitigation, cheap:** store which contributor won (2 bits — four contributors), so the scheduler can
compare like with like when it wants to, and so a switch is visible rather than silent.

### Resulting contributing set — SUPERSEDED by §7.15

> **This three-field set was overturned.** `dom_KE` was never chosen, and §7.15 shows **no value
> works**: KE spans 396× within a single configuration, no domain is display-sensible in more than
> one configuration, and `burrau` admits none at all. **KE is dropped.** The final set is the two
> bounded contributors — see §7.15d. Retained here because the reasoning that led to including KE
> (it wins 69–100% of footprints) is correct and is exactly why the `max` formulation was unsound:
> an unbounded contributor wins by virtue of its normalisation, not its information.

```
ensemble_spread = max(
    range_norm( spread(n̂),  0, 2 ),
    disagreement( event_class ⊕ event_detail ) / (1 - 1/(E+1)),
    range_norm( spread(KE), 0, dom_KE )      -- DROPPED, see §7.15
)
```

### 7.14 External replication — three conclusions corrected

An independent run of the three outstanding experiments (238 probes, 28288 trajectories, 6.4 h
compute; full report in `principia_dd_refinement_external_report.md`, raw data in `xp_results/`).
It confirmed the framework and **overturned three of my conclusions, two of them because of bugs in
my own harness.** All three verified here before acceptance.

#### (a) The gate threshold is 1e-4, not 1e-3

§7.11 chose `1e-3` by eye. Swept properly at production tolerance `eta=0.01`, pooled over 6 regions:

| threshold | median control error | mean retained |
|---|---|---|
| 1e-2 | 0.0247 | 0.940 |
| 1e-3 | 0.0083 | 0.923 |
| **1e-4** | **0.0015** | **0.909** |
| 1e-5 | 0.0046 | 0.890 |
| 1e-6 | 0.0163 | 0.840 |

**A genuine knee** — error rises again below 1e-4, which is the sample-bias arm made visible, not
noise. `1e-3` is defensible (within 0.01 of truth in 5 of 6 regions) but **`1e-4` dominates it at
1.4 percentage points of retention. Adopt 1e-4.**

**And the gate interacts strongly with integrator tolerance.** At `eta=0.005` the control error is
flat at 0.0011 across the *whole* threshold range — the gate stops mattering — while retention is
uniformly higher. At `eta=0.02` no threshold reliably reaches the 0.05 trust bar. **Specify the pair
`(eta, gate)` together, not the gate alone.**

#### (b) "Sample hygiene beats estimator sophistication" — true only for clean regions

§7.11 concluded the cheap two-point estimator suffices once data is gated. That held across the
Burrau regions tested there. In the **stressed** equal-mass rotating region, `ols2` fails the control
at *every* threshold (`|alpha_E − 1|` never below 0.06, non-monotone: 0.27 → 0.13 → 0.06 → 0.23 →
0.16) while **Theil–Sen on the same data returns 0.86–1.05**.

**Production uses exactly `ols2`.** So §7.11's conclusion needs qualifying: in stressed regions the
weak joint is the *estimator*, not the gate. Options: Theil–Sen where enough samples exist, or use
the control to *detect* the condition (`ols2` failing while `theil` passes is a specific,
recognisable signature) and fall back.

#### (c) The `max` compares a bounded quantity against an unbounded one

**This is a defect in the criterion I specified, not a fact about the physics**, and it decides the
Experiment 1 verdict. KE's winner-share versus the arbitrary divisor:

| KE divisor | burrau | eq_rot (L≠0, E<0) | eq_fast (E>0) |
|---|---|---|---|
| /0.5 | 0.96 | 0.84 | 1.00 |
| /1.0 | 0.92 | 0.69 | 1.00 |
| **/2.0 (as specified)** | **0.92** | **0.49** | **0.93** |
| /4.0 | 0.90 | 0.24 | 0.55 |
| /8.0 | 0.80 | 0.12 | 0.50 |

Burrau is robust across a 16× range (0.96 → 0.80); `eq_rot` crosses the 50% falsification line
between /1.0 and /2.0. So **"KE dominance is falsified at `L≠0, E<0`" is true at the specified
constant and not robust to it.**

`shape` has a principled bound (chord ≤ 2 on the unit sphere) and `event` has one (attainable maximum
`1 − 1/(E+1)`). **KE has none**, so `max` is comparing incommensurable things on an arbitrary scale.

**Physically-derived scales do not fix it.** Tested here: normalising KE by `|E_0|` or by the virial
scale `GM/R`, then varying the residual multiplier 0.5× to 4×:

| KE scale | winner-share range over a 8× multiplier sweep |
|---|---|
| constant 2.0 | **0.04** |
| `/|E_0|` | 0.77 |
| `/(GM/R)` | 0.69 |

Both are *worse*. They change KE's magnitude but not its unboundedness, and they push it near the
crossover where the multiplier dominates.

**The resolution is already in the architecture.** Every other normalisation in the system uses
`range_norm` against the field's **declared display domain**, and that is the principled bound here
too: a field's spread matters exactly insofar as it would change the displayed colour, so
`spread / dom_KE` is *the fraction of the colour ramp the uncertainty covers*. The outstanding task
was always "KE needs a declared bounded domain" (§7.13) — this shows that constant is
**load-bearing for the criterion, not just for rendering**, and must be chosen on display grounds
rather than picked.

**Until it is chosen, the `L≠0` verdict is not actionable.**

#### (d) `t_end` — my Q3 conclusion measured my own instrument

§7.13 excluded `t_end` because parent and child spreads were **exactly** 0.0000e+00. The cause is in
my harness: `tb_all_az.py` assigns `t_end = t_now` at **sync boundaries only**, so its resolution
*is* the sync interval — 0.4 time units at the settings used. Verified: all 128 trajectories in one
probe carry `t_end = 6.8` exactly = 17 × 0.4. **Within-footprint spread was zero by construction, at
every horizon.**

Re-run with the sync interval cut 32×:

| region | sync | footprints with >1 distinct `t_end` | **alpha** | control |
|---|---|---|---|---|
| mid-field | 0.4 | 1/16 | **nan** | ok |
| mid-field | 0.05 | 14/16 | **1.3697** | ok |
| body-2 mid | 0.4 | 0/16 | **nan** | ok |
| body-2 mid | 0.0125 | 16/16 | **0.7648** | ok |

**`t_end` was never shown to carry no signal — it was measured with an instrument 32× coarser than
the effect.** It should be **re-scoped, not excluded**. Caveat retained from the report: restricting
to escaped copies makes the surviving subset depend on perturbation size, which breaks the linearity
the energy control assumes, so those controls are computed on the same biased subset. Treat
0.76–1.37 as indicative, n=2 regions.

Separately, escape is **later than the horizon, not absent**: near-field goes 0.00 → 0.01 → **0.56**
between `t` = 13, 30, 60. My "100% censored" was a statement about `t=13`.

#### (e) A cost pathology in `tb_az.py` — fixed

When the AZ state overflows to non-finite, `done = s[:,8] >= dt_left` is never satisfied (NaN
comparisons are False), so `integrate_az` burns its entire `max_steps` budget: **354 s against ~3 s
nominal**, and it is why Experiment 2's long horizons are 43% incomplete. Results stayed valid (those
trajectories carry NaN drift and the gate excludes them) but cost blew up ~100×.

**Patched** — non-finite trajectories now exit the loop. Regression-checked: identical drift and FTLE
to the prior version.

The report also flags a trap I would likely have hit: **lowering `max_steps` as a workaround would
corrupt results silently**, because truncated trajectories have their playhead left behind while
their energy stays conserved — so the drift gate would *not* catch them, and they would be compared
at the wrong physical time.

#### (f) The gate does not transfer to long horizons

Calibrated at `t=13`. At `t=120`, in the *best-behaved* region, retention falls to 0.70 (parent) and
0.62 (child) — **and parent and child are censored by different amounts**, so any long-horizon
parent–child exponent is computed on two differently-censored samples.

#### What this does not change

The framework held: the drift gate is necessary, the energy control detects estimator failure, and
the exponent is the criterion. **Every correction above was found *by* the control**, which is the
strongest available evidence that the control works.

### 7.15 `dom_KE` resolved — KE is dropped, and the `max` formulation is the real defect

Second external run (260 probes, 33280 trajectories, 92 min, zero failures; full report in
`principia_dd_refinement_external_report2.md`, raw data in `xp_results2/`). It answers the blocking
question and, in doing so, **falsifies the resolution proposed in §7.14c**.

#### (a) No `dom_KE` works — the reason is structural

No value is display-sensible in more than one configuration, and **`burrau` — the configuration the
original 69–100% claim was measured on — admits none at all.** It is squeezed from both sides: at
`dom_KE = 2` occupancy is acceptable (0.077) but 9% of values clip; at `dom_KE = 4` clipping is fine
but occupancy has fallen to 0.039.

The cause is dynamic range, not a search failure:

| config | field | p90/p10 |
|---|---|---|
| burrau | event | **3.5** |
| burrau | **KE** | **396** |
| eq_rot | KE | 102 |

**KE spans 396× within a single configuration where `event` spans 3.5×.** No single linear domain can
hold the bottom decile off the floor and the top decile off the ceiling across that range. The two
configurations that *do* admit a usable band have **disjoint** bands (0.125–0.25 for `eq_fast`, 2.0
for `eq_rot`).

#### (b) The display-domain hypothesis (§7.14c) is inert — my proposal was wrong

§7.14c argued the principled bound is the field's declared display domain, since a ramp clips rather
than running away. Tested by scoring KE **clamped at 1.0**: winner shares came back **bit-identical
at every domain**.

Verified independently here: over 20 000 synthetic footprints at five domains, clamping changed the
winner in **0.0000** of cases. The reason is immediate once stated — `shape` and `event` are bounded
by their achievable maxima and in practice sit well below 1.0, while a clipped KE equals exactly 1.0,
so it still wins. **Clamping changes the value, never the winner.**

The hypothesis is coherent as display semantics and has **no effect on the criterion**. It renames
the free constant rather than eliminating it.

#### (c) The defect is `max`-over-normalised itself

> *"`max` over normalised contributors makes the normalisation constants the criterion: whichever
> field is normalised most loosely wins nearly every footprint, so the constants do the work of a
> decision threshold while looking like units."*

That is the correct diagnosis, and §7.14c's verdict flipping between `/1.0` and `/2.0` is the same
mechanism showing through. Two further objections, both sound:

- **`max` is winner-take-all**, so it discards the information that *two contributors agree* — which
  is precisely the evidence that a footprint is genuinely indeterminate rather than noisy in one
  channel.
- **It is discontinuous in the constants.** A small change to `dom_KE` near a crossover flips a
  footprint's refinement decision. Not a property a scheduler should have.

**Squashing functions rejected, correctly:** `x/(x + x0)` bounds KE but reintroduces the same free
constant with the same power to decide the verdict — *"it looks principled and isn't."*

#### (d) Adopted: bounded contributors only

```
ensemble_spread = max( spread(n̂) / 2.0,                                  -- chord bound
                       disagreement(class ⊕ detail) / (1 − 1/(E+1)) )     -- attainable maximum
```

**With every contributor bounded by its achievable maximum, the normalisations become facts rather
than choices.** This is the smallest change that is well-posed. Rank/quantile transform is the
fallback if sensitivity proves insufficient.

**The cost, stated plainly:** in `burrau`, at any sensible domain, KE wins 0.90–1.00 of footprints —
so dropping it makes the criterion **materially less sensitive in the free-fall regime**. That is the
price of well-posed over tuned, and it is the right trade: a criterion whose answer depends on an
arbitrary constant is not a criterion.

#### (e) The estimator fallback does not exist — Theil–Sen *is* `ols2` at two scales

§7.14b proposed Theil–Sen as a fallback where `ols2` fails. **At two points there is exactly one
pairwise slope, so median-of-one is that slope.** Verified independently here: worst
`|theil − ols2|` over 5000 random two-point cases is **8.7e-10**.

Theil–Sen's advantage in Brief 1 came from consuming **4–5 scales**, not from robustness. Production
has two. **So the fallback is not a better estimator — it is to acquire a third scale.**

**But the condition is detectable for free.** `ols2` fails while Theil–Sen passes in 4/43 groups, and
**retention alone catches it**: `retained < 0.90` catches 5/6 failures with 1 false alarm, and the
single miss is a rounding-width case (`|alpha_E − 1| = 0.0510` against a 0.05 bar). Retention is
already computed for the gate.

#### (f) `t_end` carries real signal once measured properly

Bisecting inside the interval where escape fires gives resolution **1.95e-04 against 0.4** — resolving
**16/16 footprints** where the sync-boundary method managed 1/16. `alpha = 0.9464` (mid-field) and
`0.8588` (body-2 mid), both with trustworthy controls.

The caveat is now quantified: escaped-only subsetting moves `alpha_E` by **0.0000** where escape is
complete and **0.0537** where partial — just across the trust bar. **Rule: trust `t_end` exponents
only where escape fraction ≈ 1.0.**

#### (g) The long-horizon conclusion was my harness bug, and the gate is the real limit

With the non-finite patch (§7.14e), **all 40 probes succeed where 17 previously failed.** Every region
ionises completely by `t = 240`; `far r≈10` goes **0.00 → 1.00 between `t` = 13 and 30**. The Brief 1
reading that those regions "are bound, and waiting does not help" was an artefact of the timeouts.

**But the gate collapses before the escapes arrive.** At `t = 240`, near-field retains **10%** and
**0/16 footprints** clear the 3-copy minimum. There is no horizon at which escape is measurable *and*
the data survives. **The binding constraint on long-horizon rendering is the gate, not the
integrator** — which reframes §7.14f from a calibration note into a structural limit.

**And the gate biases escape fraction opposite to expectation.** The natural worry is that
easy-to-integrate escapers are over-represented. It runs the other way: escape is powered by close
encounters, exactly where AZ works hardest, so **escapers carry higher drift and are preferentially
removed — understating escape by up to 0.19.** All reported escape fractions are conservative.

### 7.16 The drift gate was architecturally wrong — failure is a measurement, not an absence

§7.11–§7.15 treated the drift gate as a *filter*: exclude trajectories with `|dE/E| >` threshold
before reducing. That is wrong, and the objection is architectural rather than statistical.

#### Why discarding is not admissible

1. **It is chaos-selective.** High drift genuinely is integration error (energy is conserved exactly,
   so if it moves the integrator failed) — but integration difficulty is *caused by close
   encounters*, and close encounters are what chaos is made of. **Excluding by drift is excluding by
   chaoticity**, on an instrument whose purpose is measuring chaos. Measured consequence: escape
   fraction understated by up to 0.19 [§7.15g].
2. **It makes the normalisation constant a variable.** `event` is normalised by its attainable
   maximum `1 − 1/(E+1)`. If `E` varies per pixel because copies were removed, that "constant" is a
   per-pixel free parameter — **exactly the defect that disqualified KE** [§7.15c].
3. **Quality decays with the playhead.** Failures are absorbing: a trajectory that fails stays
   failed. So the effective copy count falls monotonically as `t` advances, and image quality degrades
   as you scrub forward. Unacceptable for a lockstep instrument.
4. **It violates a stated design principle.** The sampling note: *"Uniformity beats the micro-saving.
   One kind of thing: a pixel is a pixel… there is genuinely nothing special about any sample."*
   Gating makes some samples special.
5. **It produces confident-looking answers from biased subsets.** At `t=80`, near-field, the gated
   pipeline reports `alpha_E = 0.8607` computed on **44%** of the data. That reads as an answer. It is
   the exponent of the tame minority that survived.

#### The statistical fix does not work either

Robust dispersion (MAD, no gate) was tested as a way to keep every trajectory:

| horizon | retained | `alpha_E` gated+std | `alpha_E` ungated+MAD |
|---|---|---|---|
| t=13 | 0.99 | 1.0100 | 1.0214 |
| t=40 | 0.82 | 1.0241 | **0.1413** |
| t=80 | 0.44 | 0.8607 | **6.0362** |

MAD is robust to a **minority** of outliers; past a 50% breakdown point the median is itself
corrupted. **Neither approach rescues the long-horizon case, because at long horizon most of the
sample genuinely is unmeasurable.** No statistic recovers information that was never obtained.

#### The correct disposition: a failed trajectory is a measurement outcome

It is not missing data. It has a value — *"this could not be determined"* — which is **information,
not absence**. And it is the strongest possible statement that a pixel's value is not determined,
which is precisely what `ensemble_spread` measures. Under the gate this read **low** (determinate);
it must read **high**.

**So keep the sample uniform and encode the failure in the value:**

```
ensemble_spread = max( spread_shape, spread_event, failed_fraction )
```

- **Every footprint always carries exactly `E+1` copies.** Nothing is ever discarded; the denominator
  is always `E+1`, so the normalisations stay constants.
- **`failed_fraction` is bounded `[0,1]` by construction** — the most principled bound available. It
  joins the bounded-contributor set without reintroducing a free constant.
- **The drift threshold stops being a filter and becomes a classifier.** Same number, entirely
  different disposition: it labels a copy *indeterminate* rather than deleting it.

Measured, near-field:

| t | `failed_fraction` | spread (gated, old) | **spread (uniform, new)** | reading |
|---|---|---|---|---|
| 13 | 0.01 | 0.014 | 0.026 | determinate |
| 40 | 0.18 | 0.496 | 0.496 | partly indeterminate |
| 80 | 0.56 | 0.531 | **0.664** | **indeterminate** |

The failure term takes over exactly where the old pipeline was reporting the surviving minority.

#### It also makes the refinement decision self-consistent

Refining shrinks the jitter, so copies start closer together — but **each trajectory still has the
same close encounter**. Integration failure is therefore *not reducible by refinement*. Under the new
formulation this falls out automatically: as failures accumulate, parent and child spreads both
saturate toward 1, their ratio tends to 1, and `alpha → 0` — which is the **floor** decision.
Correct, and arrived at honestly rather than by a special case.

#### Consequences for the struct

- **`retained_fraction` is no longer a trust flag; `failed_fraction` is a contributor.** (`retained =
  1 − failed`, so the estimator-failure detector of §7.15e still works — it is the same number read
  for a different purpose.)
- **`worst_energy_drift` is not "the gate".** It is the input to a per-copy classifier.
- **A uniformity invariant should be stated explicitly** alongside the struct: *every footprint
  carries `E+1` copies at all times; no copy is ever removed from the sample.*
- **Open question 13 is reframed.** It is not "the gate limits the horizon" — it is **"the instrument
  must report where it stops knowing"**. With `failed_fraction` as a contributor, it does: a
  long-horizon pixel reads *indeterminate* instead of emitting a fabricated exponent. That is a
  reporting property, not a compute limit.

### 7.17 "Failed" was another picked constant — the parameter-free replacement

§7.16 said the drift threshold becomes *"a classifier, not a filter"*. That does not survive scrutiny:
**a classifier with an arbitrary threshold is still an arbitrary constant deciding the answer** — the
same defect that disqualified `dom_KE` (§7.15c), relocated rather than removed. Numerical error in
this system is a continuum, not a binary condition, so "failed" is not a natural category.

#### The physical argument: below some scale, integration error is not contamination

**In a chaotic system a slightly-inaccurate trajectory is the exact trajectory of a slightly-different
initial condition.** The ensemble deliberately perturbs initial conditions. So integration error is
*the same kind of thing as the jitter* and is absorbed into the quantity being measured. It becomes a
problem only when it exceeds the deliberate perturbation — at which point the copy is sampling a
different neighbourhood, not this pixel's.

That gives a **self-scaling comparison** rather than a picked one: drift against the ensemble's own
energy spread `sigma_E`. Measured (p99 drift ÷ `sigma_E`):

| region | t | ratio | reading |
|---|---|---|---|
| near-field | 13 | **4.5e-03** | error 200× below the jitter — **absorbed** |
| near-field | 40 | 2.2e+03 | dominates |
| near-field | 80 | 6.4e+04 | dominates |
| body-2 core | 13 | 1.1e+01 | comparable |

#### NaN/Inf alone is not sufficient

The natural proposal — treat only non-finite values as failures, and prevent those by construction —
is necessary but does not reach the problem:

| region | t | NaN/Inf | median drift | p90 | p99 | max |
|---|---|---|---|---|---|---|
| near-field | 13 | 0.000 | 3.9e-09 | 1.7e-08 | 7.4e-05 | 2.5e-04 |
| near-field | 40 | 0.000 | 1.6e-07 | 8.6e-03 | **2.9e+03** | 9.8e+03 |
| near-field | 80 | 0.042 | 3.1e-04 | 4.5e+01 | **1.2e+07** | 3.6e+07 |

**Non-finite values are 0.0–4.2%, while the finite tail spans ten orders of magnitude.** The damage
is done by finite-but-meaningless values, which no non-finite check catches.

#### The parameter-free diagnostic

Each trajectory conserves its own energy exactly, so **the ensemble's *spread* of energies is fixed at
`t=0` and must remain constant for ever**. Any growth in `sigma_E` is *pure integration error* —
no threshold, no constant, and it is per-footprint:

```
error_ratio = sigma_E(t) / sigma_E(0)        -- exactly 1.0 under exact dynamics
```

| region | t=13 | t=40 | t=80 |
|---|---|---|---|
| near-field | **1.0000** | 80.96 | 9150.29 |
| body-2 core | **1.0000** | 1.0003 | 121902.53 |

**1.0000 at `t=13` in both regions** — the claim that numerical error is harmless there is
*measured*, not assumed. And it diverges cleanly where error genuinely accumulates.

This is the same construction as the `alpha_energy` sanity field (§7.11): a conserved quantity whose
correct value is known analytically, used as a free correctness check. `alpha_energy` validates the
*estimator*; `error_ratio` validates the *integration*. Neither needs a tuned constant.

#### Consequences

- **`failed_fraction` as defined in §7.16 is withdrawn.** It presupposed a binary failure category
  that does not exist.
- **Replace it with `error_ratio = sigma_E(t)/sigma_E(0)`**, a continuous per-footprint quantity with
  a known correct value of 1.0. It answers *"how much of this footprint's apparent spread is
  numerical rather than physical?"* — which is the real question.
- **Non-finite values remain a genuine binary failure** and should be prevented by construction
  (the §7.14e patch removed the one known source). They are rare enough (≤4%) to be handled as an
  explicit exceptional case rather than a statistical one.
- **The uniformity invariant of §7.16 stands and is strengthened** — no copy is discarded, and now
  nothing is classified either. Every copy contributes; `error_ratio` reports how much to trust the
  aggregate.
- **Still open: how `error_ratio` enters the spread.** As a contributor (bounded by construction only
  from below), as a divisor, or as a reported confidence alongside it. It is not bounded above, so it
  cannot naively join a `max` of bounded contributors — the §7.15c objection applies to it too, and
  must be answered before it is specified.

### 7.18 Two closing decisions: contamination tolerance, and length scales in canonical units

#### (a) Contamination is tolerated, and the justification is ordinality + failure direction

Bad values enter the reduction. Nothing removes them — §7.16 rules out discarding on architectural
grounds and §7.17 rules out a "failed" category as another picked constant. **The criterion does not
need them gone; it needs to be justified and good enough.** Two facts make that a principled position
rather than a shrug:

1. **The exponent is used ordinally, not quantitatively** (§7.12). It ranks quads; it does *not*
   forecast the realised gain as `2^-alpha`. So contamination would have to be **systematically
   rank-changing** to matter, and it hits parent and child similarly, so much of it divides out of the
   ratio.
2. **The failure direction is conservative.** Contamination inflates spread, which pushes toward
   *refine*. Over-refining a hard region wastes budget; under-refining a structured one loses
   information. **The criterion fails in the wasteful direction, not the lossy one.**

That is the justification for stopping: not that the estimate is clean, but that it is **safe when it
is wrong**. `error_ratio` (§7.17) reports how contaminated a footprint is, so the condition is visible
rather than hidden. No further tuning is planned.

#### (b) Length scales are admissible in canonical units — and must be fixed at `t=0`

§7.5 condemned softening for breaking the scale gauge. That conclusion was too broad. The gauge test
in that same section shows why:

| α | ε **scaled** with the configuration | ε **fixed** in absolute units |
|---|---|---|
| 0.5 | 0.58166 | 0.51401 |
| 1.0 | 0.58166 | 0.58166 |
| 2.0 | 0.58166 | 0.85506 |
| 4.0 | 0.58166 | 0.73722 |

**Scaling ε with the configuration preserved the gauge exactly, to five decimal places, across a 16×
size range.** The break came from a *fixed absolute* length, not from having a length scale at all.

Principia canonicalises `√I = 1`, so **a constant expressed in canonical units is rescaled by
dynamical similarity along with everything else** and is therefore gauge-covariant by construction.
This applies equally to the collision radius `r_coll` and to an optional softening length `ε`.

**Three constraints on how they are specified.**

- **Fixed at `t=0`, never co-moving.** It is tempting to scale ε with the *instantaneous* `I` so it
  tracks the system as it breathes. That would make the Hamiltonian **time-dependent and destroy
  energy conservation** — and energy conservation is exactly what `error_ratio` and `alpha_energy`
  are built on. A co-moving ε would delete both diagnostics. The constraint is forced, not chosen.
- **Gauge-covariant is not the same as correct.** Canonical units buy *"the same answer regardless of
  arbitrary overall scale"*; they do not buy *"the right answer"*. Softened gravity is a different
  force law at any ε. §7.6 measured the consequence: field rankings between ε=0.03 and ε=0 agree at
  **16/28 pairwise, i.e. chance**. **ε>0 runs must not be pooled with ε=0 runs**, and ε should be
  tagged in the payload so a dataset cannot silently mix them.
- **`r_coll` is the better default mechanism.** Softening makes the dynamics wrong *everywhere*;
  a collision radius leaves them exact and only decides *when to stop*. Since spec pending change 7
  makes collision a first-class outcome (including triple collision at `detail = 11`), terminating
  there records a **result** rather than applying a fudge — and it bounds the substep explosion at
  close approach, which is the actual practical problem (measured: 354 s against ~3 s nominal before
  the §7.14e patch).

**Adopted:** `r_coll` **nonzero by default**, `ε` **default 0 and optional**, both expressed in
**canonical units** and both **fixed at `t=0`**. `r_coll = 0` is permitted and means "no collision
termination" — at which point close approaches are bounded only by regularisation and the substep
budget.

### 7.19 No-discard validated at short horizon; three meter corrections; a fourth invariant

Third external run (96 probes, 15360 trajectories, **nothing discarded anywhere**; full report in
`principia_dd_refinement_external_report3.md`, raw data in `xp_results3/`). Verified here before
acceptance.

#### (a) Partial confirm — and removing the gate does not remove the horizon problem

| horizon | MAD | trimmed std | std |
|---|---|---|---|
| t=13 | **10/12** quads recover `alpha_E` | — | — |
| t=40 | 5/12 | 4/12 | 4/12 |

At `t=13` the loop closes: MAD over the full uniform sample recovers the control **and preserves the
`shape` ranking against the gated baseline at Spearman +0.943**. At `t=40` no measure survives.

**The honest reading:** removing the gate stops the *sample* shrinking — which was the point — but the
reduction still degrades with playhead for the same underlying reason. **Copy count is not the lever:**
8 → 16 copies moved MAD from 7/12 to 8/12, so 2× cost buys almost nothing.

#### (b) `error_ratio_lz` is withdrawn — structurally undefined

**Undefined in 18/18 Burrau probes.** Released from rest, `v = 0` for every copy, so `L_z ≡ 0` and
`sigma_Lz(0) = 0` exactly — the ratio is `0/0`. Verified here directly.

**This is structural, not a sampling accident.** The construction needs the quantity to *vary across
the ensemble at `t=0`*. Jittering a position always changes potential energy, so `sigma_E(0) > 0`
always; it never changes `L_z` from rest, for any positions. **The entire from-rest regime — the whole
baseline of this document — has no `L_z` meter.** Where it does exist it correlates **+0.910** with the
energy meter anyway.

**One meter: energy.** Remove `error_ratio_lz` from the struct.

#### (c) Two corrections to the meter as specified

1. **It must use a robust statistic internally.** A `std`-based `sigma_E(t)/sigma_E(0)` returns **NaN
   the moment one copy is non-finite** — precisely the pathological footprint it exists to flag. Under
   exact dynamics both sets are identical so *any* statistic gives exactly 1.0; calibration does not
   depend on the choice. **Use MAD inside the meter.**
2. **It must aggregate by `max` over footprints, not median.** The reduction takes a *mean* over
   footprints, which one bad footprint moves; a median-aggregated meter cannot see that. Max-aggregated
   catches **9/9 with zero misses** (3 false alarms) against median's 6 caught, 3 missed.
   **But treat it as a boolean flag** — its magnitude is unstable, worsening when `eta` was *tightened*
   in three cases while median drift improved 10–100×.

#### (d) `Gamma` is not a third meter

Correlation **+1.0000000000** with `A·B ×` energy error — exactly as the construction implies. Drop.

#### (e) COM and momentum are *not* meters under AZ — this reported finding does not hold

The report proposed that `to_cartesian`'s COM re-centring **destroys** a free correctness check
(measured `|COM| = 1.48e-16`, `|P_total| = 8.88e-16`), recoverable by evaluating before the re-centring
line. **Checked against the formulation, this is wrong, and it was accepted here without checking.**

Under AZ the integrated state is `(u1, p1, u2, p2)` — **8 phase-space dimensions, exactly the relative
motion**. The full planar three-body problem has 12; the COM's 4 (position and velocity) are **never
integrated at all**. `R1` and `R2` are relative by construction, so the COM position is a *derived*
quantity fixed by `R1, R2` and the masses.

**So `|COM| = 1.5e-16` is not a destroyed check — it is the correct value of a quantity that has no
dynamics in this formulation.** Translation and boost are precisely the gauges AZ quotients out.
Nothing is recoverable by moving the measurement earlier, because nothing is being integrated that
could drift. The claim *would* hold for a **Cartesian** integrator, where each body's absolute position
is integrated and numerical error makes the COM wander; it does not transfer to a relative-coordinate
formulation.

**The re-centring must also stay for a separate reason:** it is the re-projection that keeps
coordinates centred and well-conditioned. Removing it would cost precision and buy nothing.

**What does survive from this finding** is the frame-consistency lesson, which is real and general: a
first round-trip implementation read a constant **0.627** at every horizon from `t=0.5` to `t=80`,
because it differenced a COM-centred return against a lab-frame start. **A meter is only as good as the
frame it is evaluated in** — and that applies to `roundtrip_error` (§7.19f), which *is* a real meter.

#### (f) A fourth invariant, and it is the one that tests the central justification

**Time-reversal round-trip error.** In four cases **every conserved quantity reads exactly 1.000000
with drift at machine precision, while the trajectory has moved 0.3–4.7% of a system radius.**
Phase error conserves `E` and `L_z` exactly and is **structurally invisible** to both proposed meters.
`corr(round-trip, drift) = +0.186` — genuinely independent.

**This makes §7.18a's absorption argument directly testable.** The claim was: integration error is
absorbed while it stays below the jitter the ensemble deliberately applies. Measured, it **holds in
9/12 cases**, and in Burrau with two orders of margin. But **`eq_fast b0 core` at `t=13` violates it by
3× while `error_ratio_max` reads exactly 1.000000** — the meters certify a quad whose integration error
exceeds the perturbation the ensemble is applying.

**Weighted as the report weights it, not overclaimed:** the round-trip is an *upper bound*, also
containing chaotic amplification and AZ's state-dependent reference switching, which need not select
the same reference in reverse. Its non-monotonicity for `eq_fast` (4.71e-02 at `t=13` but 1.05e-02 at
`t=40`) points at exactly that. **Read those cases as "not demonstrated absorbed" rather than "proven
violated."**

Still: the absorption argument is the justification for tolerating contamination (§7.18a), and it is
now **conditional and testable** rather than assumed. The round-trip should be carried as a second
meter precisely because it catches what the energy meter cannot.

#### (g) Canonical units — clean confirm, and co-moving prohibited with a number

**`initial` mode holds the gauge to 10 decimal places** across a 16× rescaling — better than the 5
expected. **`absolute` breaks it in the 2nd decimal**, and `n̂₀` does not merely shift but **changes
sign**, so the measured shape lands elsewhere on the shape sphere entirely.

**Co-moving `epsilon`:** `|dE/E| = 3.062e-02` at `dt=1e-4` **and 3.062e-02 at `dt=2e-5`** — seven
orders worse than `initial` and **completely insensitive to step size**. That is the wrong-equation
signature (§7.14): not an accuracy problem, the Hamiltonian is genuinely time-dependent. `error_ratio`
reports it directly at **1.227 against 1.000**.

**And the sharpest framing in the report:** co-moving is *gauge-covariant and still fails*.
**Covariance and conservation are separate requirements.** A length constant must satisfy both.

### 7.20 What `t=13` means — the horizon is a predictability limit, not an engineering one

> **Promoted to its own document: `principia_dd_predictability_horizon.md`, and substantially
> corrected there (§7).** `lambda` is 0.6–0.8 not 1; the f32 horizon is falsified (f32 AZ fails at
> `t≈1–2` through a conditioning defect, not precision); §7.21's treadmill identity is false. The
> **field verdicts are unaffected** — all were measured at `t <= 13`.

> **Promoted to its own document: `principia_dd_predictability_horizon.md`.** It constrains the
> dual-precision architecture and the CPU↔GPU cross-check, not just refinement, and it is a result
> for the papers. Summary retained here; the architectural consequences and the two-horizon
> distinction are there.

Horizons have been quoted as bare numbers throughout. In canonical units for Burrau
(`G=1`, `m=(3,4,5)`, `M=12`, hyperradius `R=2.236`, `E=−12.82`):

| quantity | value |
|---|---|
| crossing time `sqrt(R³/M)` | **0.965** |
| free-fall time | 1.072 |
| Lyapunov time `1/λ` (measured) | ~1.0 |

**So `t=13` is 13.5 crossing times and 13 e-foldings; `t=40` is 41 crossings and 40 e-foldings.**
`t=13` is not a short run — it is a substantial fraction of the interesting dynamics. (For reference,
the classical Pythagorean problem disrupts around `t ≈ 60`.)

#### The wall is round-off amplified by chaos

Error grows as `e^(λt)` regardless of its source, so machine epsilon alone reaches order unity at

```
t_max = ln(1/eps) / lambda
```

| | `eps` | `t_max` | crossing times |
|---|---|---|---|
| f16 | 9.8e-04 | 6.9 | 7.2 |
| **f32 — the GPU kernel** | 1.2e-07 | **15.9** | **16.5** |
| **f64 — the CPU kernel** | 2.2e-16 | **36.0** | **37.3** |
| f128 | 1e-34 | 78.3 | 81.1 |

At `t=36` in f64, `eps × e^(λt) = 0.96` — **round-off has been amplified to the size of the system.
There is no information left to integrate.**

#### This is confirmed, not merely predicted

If the wall were truncation error, tightening the integrator tolerance would move it. It does not:

| t | eta=0.02 | eta=0.01 | eta=0.005 |
|---|---|---|---|
| 13 | 1.0014 | 1.0214 | 1.0206 |
| 25 | 1.0043 | 0.9394 | 1.0018 |
| **40** | **−13.24** | **0.14** | **0.56** |

**Quadrupling the precision does not move it**, and the observed transition (works at 13 and 25,
fails at 40) brackets the predicted 36 exactly. **More substeps buy nothing.** The horizon is
logarithmic in precision — only *more mantissa bits* extend it, and only by `ln`.

#### Consequences for Principia

1. **The refinement criterion is not the limiting factor.** It validates within the predictability
   horizon and fails outside it. §7.19a's "t=13 works, t=40 fails" is **the physics of the system, not
   a defect of the criterion.** No amount of further work on the criterion changes it.
2. **The f32 GPU kernel has a horizon of ~16 crossing times; the f64 CPU kernel ~36.** These differ by
   **2.3×** — a design-relevant number, since Principia runs both. Beyond `t ≈ 16` the GPU path is
   integrating amplified round-off, and CPU/GPU cross-checks past that point compare two different
   noise realisations rather than disagreeing about physics.
3. **The instrument should declare its horizon.** Rendering past `t_max` is not wrong so much as
   *meaningless* — the image is a picture of amplified round-off. `t_max = ln(1/eps)/lambda` is
   computable per configuration from a quantity (`lambda`) already carried as FTLE, so the horizon can
   be **reported rather than assumed** and will vary across IC space.
4. **This is the honest framing of the "horizon problem"** raised in §7.15g and §7.19a: not a gate to
   calibrate or a criterion to improve, but a **statement the instrument must make about where its
   answers stop meaning anything.**

### 7.21 Horizon audit — which measurements are inside it

Applying §7.20's horizon (`t_max = 36` in f64) retrospectively to everything in this document.

#### Safe: every field verdict

All contributor rankings and exclusions were measured at `t <= 13`, well inside the horizon. **No
field verdict changes.**

**Diffusion's exclusion is stronger than §7.13 stated.** It was rejected there for winning the `max` in
≤6% of footprints — a single-horizon argument. In fact it is dominated across the entire usable window:

| field | t=2 | t=4 | t=6 | t=8 | t=10 | t=13 | t=16 |
|---|---|---|---|---|---|---|---|
| **shape n̂** | **0.0058** | **0.1593** | **0.1013** | **0.5126** | **0.6021** | **0.6443** | **0.7199** |
| event | 0 | 0.0469 | 0.0469 | 0.1797 | 0.2109 | 0.2578 | 0.3125 |
| diffusion | 0.0030 | 0.0018 | 0.0223 | 0.0107 | 0.0197 | 0.0438 | 0.0542 |
| `d_min` | 0.0045 | 0.0100 | 0.0103 | 0.0156 | 0.0150 | 0.0162 | 0.0165 |

**Shape exceeds diffusion at every playhead tested.** Under a `max` aggregation, diffusion can never
contribute anywhere in the usable window — not a marginal call at one horizon but domination
throughout. The same holds for `d_min`. **The two-field set is optimal in the strong sense: nothing
else ever wins.**

Coverage also matches the window well — shape is the only signal before `t≈4`, event takes over from
`t=4`, and the **f32 horizon is ~16**.

#### Not safe: the long-horizon escape claims

§7.15g reported *"every region ionises completely by `t = 240`"* and *"`far r≈10` goes 0.00 → 1.00
between `t` = 13 and 30", used to overturn a Brief-1 conclusion.

**`t = 240` is ~6.7× the f64 predictability horizon.** `t = 30` is at 83% of it and `t = 60` past it.
Those trajectories are propagating amplified round-off, so **the escape fractions are not trustworthy
as stated**. They describe *a* plausible evolution, not *the* evolution of those initial conditions.

Two qualifications, so this is not over-corrected:

- **Escape is a coarse, absorbing outcome.** Once a body is unbound and receding it stays so, and the
  *statistics* of escape across an ensemble may be robust where an individual trajectory is not. The
  claim "these regions eventually ionise" is plausibly right; the specific fractions and timings are
  not measurements.
- **The `t_end` re-scoping (§7.15f) is unaffected** — it was measured at `t = 13`, and the finding was
  about *instrument resolution* (sync-interval quantisation), not about long-horizon physics.

**Any long-horizon claim in this document should be read as indicative.** Establishing them properly
needs either higher precision (f128 buys `t ≈ 78`) or an argument that the specific statistic is
shadowing-robust.

#### The treadmill and the measurement horizon are the same relationship

§4 found that holding a pixel below threshold requires `delta < tau · e^(−lambda·t)` — "the required
spacing halves every 0.69 time units". §7.20's measurement horizon is `t = ln(R/delta)/lambda`.

**These are the same equation, rearranged.** So the treadmill has a second reading:
**halving the spacing buys 0.69 more time units of validity.** Refinement extends the measurement
horizon logarithmically — *the instrument sees further in time precisely by refining in space*. The
exponential cost of the treadmill and the logarithmic gain in horizon are one fact, not two.

---

## 8. Current definition of `ensemble_spread`

**One scalar.** What the shader displays *is* what `QuadReduction` stores *is* what the scheduler
reads. Computed **per footprint, at the resolve stage, over that footprint's `E+1` copies**. No
graph-dependent variant may claim the name; per-field views are **debug** shaders under their own
names (`spread · d_min`, `spread · ftle`, …).

```
ensemble_spread = max over the fixed contributing set F of
                    range_norm( spread_of_field, field.dom )       -- dimensionless 0..1
```

- **`max`, not mean** — the question is *"is this pixel's value determined?"*; one uncertain
  component answers no. A mean would dilute a single decisive disagreement.
- **Fixed set** `F`, a property of the IC neighbourhood, not of the active render graph.
- **Normalised per field** so heterogeneous fields are commensurable before aggregation.

### Contributing set — current best, on the evidence above

| field | spread measure | normalisation | covers | evidence |
|---|---|---|---|---|
| **shape n̂** | mean distance from centroid | `range_norm(·, 0, 2)` | `t < 4` — the only signal that early | exponent 1.00 at t=3; A(14)=350 |
| **event `class⊕detail`** | `1 − modal/(E+1)`, joint grain | ÷ `(1 − 1/(E+1))` → tier-independent | `t ≥ 4`, **ungated** | §6.5, §6.6 |
| **`t_end`** | std of copies' `t_end` | `range_norm(·, 0, horizon)` | `t ≥ 8` | 585× discrimination, score 0.191 |
| **diffusion** | std of copies' slope | `range_norm(·, dom)` | graded where event quantises | 1055× discrimination, score 0.111 |
| *kinetic energy* | std | needs a bounded domain | — | **candidate** — highest dynamics (0.422), and the only strong field robust to softening (§7.3, §7.5) |

> **Provisional.** The *ordering* of the included fields is ε-dependent (§7.5) and must be rerun with
> regularisation before it hardens into spec. The **exclusions** below rest on ε-robust evidence.

**No `terminated_fraction` gate is required** — event class is defined at every playhead, which was
the reason for preferring it over outcome. `t_end` reading ≈0 before any escape is *correct*.

### Excluded, each on evidence

| field | reason | § |
|---|---|---|
| total energy, `L_z` | conserved → amplification ≡1.00; spurious exponent 1.0 early, integrator noise late | 6.3 |
| `d_min` | latches (dynamics 0.013); magnitude ≈0.008 after normalisation — can never win the `max` | 7.4 |
| FTLE | map exponent 0.115 — refining cannot reduce it. **Contested**: coverage argument in §7.2 | 7.2 |
| max separation | unbounded once bodies escape — no domain to normalise against | 6.3 |
| outcome | subsumed by event `class⊕detail`, which is ungated and available 4 time units earlier | 6.6 |
| `S_word` | different complexity class (pairwise LCP, not one-pass); lags divergence | — |

### The refinement signal is derived, not parallel

It is the **scaling exponent of this same scalar** between a parent quad and its children. Exponent
≈1 → **split**; ≈0 → **floor**; spread ≈0 at all scales → **keep coarse**. The quad hierarchy
supplies the two scales; the scheduler's existing "parent–child disagreement" **is** this estimator
and should be specified as one.

---

## 9. Open questions

1. **~~Is `ensemble_spread` for display coverage or for refinement?~~ — moot (§7.9).** It was raised
   to decide FTLE. Under AZ, FTLE's dynamics score is **negative** (−0.127) — its spread does not grow
   with playhead at all. The 18.8× discrimination and the coverage argument were softening artefacts.
   **FTLE is excluded on evidence**, so the specification question no longer has to be answered.
2. **~~Does kinetic energy join the set?~~ — yes (§7.9).** Third-highest dynamics under AZ,
   exponent 1.098, widest discrimination of any dynamical field (9258×). **Remaining task is only to
   declare a bounded domain** for `range_norm`.
3. **~~`t_end` censoring~~ — resolved by exclusion (§7.13).** Not a measure-design problem: **100%
   censored** in three of four regions at `t=13`, so there is no signal. Its top ranking in §7.9 was
   an artefact of near-uniform zeros. **Excluded**, not deferred.
4. **~~Does diffusion earn its place~~ — no (§7.13).** It wins the `max` in 6% of footprints in one
   region and 0% elsewhere, an order of magnitude below KE. A contributor that never wins a `max`
   contributes nothing. **Excluded**; KE supplies the graded resolution it was kept for.
5. **~~Closed-loop validation~~ / ~~robust estimator~~ — both done (§7.10–§7.12).** The estimator
   failure was **integration failure, not fitting**: 2% of trajectories with runaway drift dominated
   the reduction and moved an exponent by 7×. A per-trajectory drift gate fixes it, after which the
   cheap two-point estimator has worst-case error 0.0031 on a known-answer control. Remaining
   nuance: the control validates the estimator, **not** per-field predictability — so the exponent
   should be used **ordinally** (rank quads) rather than as a quantitative forecast. Original note: The rule predicts the
   realised refinement gain to within ~20% wherever the exponent is well estimated, and the
   total-energy control recovers its known answer to **0.002**. But where the estimator breaks
   (`body-2 core`: control exponent read 6.691 instead of 1.0) every prediction breaks with it.
   **New top task: a robust exponent estimator**, since production has only two scales
   (parent–child) against the three used here. Carry total energy as a per-quad **sanity field** —
   its exponent is known to be 1.0, so it validates the estimator for free.
6. **~~Contributor switching~~ — real but benign (§7.13).** The winner does shift between levels
   (0.69 → 0.81 KE share), but only between KE and event class, and §7.12 already restricts the
   exponent to *ordinal* use. **Adopted mitigation: store which contributor won (2 bits)**, so a
   switch is visible rather than silent.
7. **~~Cross-system~~ / ~~`dom_KE`~~ — closed (§7.15). KE is DROPPED.** No domain is
   display-sensible in more than one configuration and `burrau` admits none; KE spans 396× within one
   configuration against `event`'s 3.5×. The §7.14c display-domain resolution was **wrong** — clamping
   leaves winner-shares bit-identical. **The contributing set is the two bounded fields.** Earlier
   note: At `L≠0, E<0`
   KE's winner-share falls to 0.49, below the falsification line — **but the verdict is not robust to
   KE's arbitrary normalisation constant**, which is the real finding. **New top task: choose
   `dom_KE` on display grounds**; until then the `L≠0` result is not actionable. Also reopened:
   `t_end` (§7.14d) was excluded on an instrument artefact and should be re-scoped.
11. **Gate + tolerance must be specified as a pair** (§7.14a) — at `eta=0.005` the gate stops
    mattering; at `eta=0.02` no threshold reaches the trust bar.
12. **~~Estimator fallback~~ — there isn't one (§7.15e).** Theil–Sen *is* `ols2` at two scales
    (verified to 8.7e-10); its Brief-1 advantage came from consuming 4–5 scales. **The fallback is to
    acquire a third scale.** Detection is free: `retained < 0.90` catches 5/6 failures.
13. **~~The gate is the binding constraint on long-horizon rendering~~ — reframed and answered
    (§7.16).** Discarding trajectories was architecturally wrong: chaos-selective, it turns the
    normalisation constant into a per-pixel variable, it degrades image quality monotonically with
    the playhead, and it violates the sampling uniformity principle. **A failed trajectory is a
    measurement outcome, not missing data.** Keep every copy, add `failed_fraction` as a bounded
    contributor. The instrument then *reports* where it stops knowing rather than emitting a
    fabricated exponent from a biased subset.
14. **`t_end` reinstated conditionally** (§7.15f) — `alpha` ≈ 0.86–0.95 once bisected, but trust only
    where escape fraction ≈ 1.0.
8. **~~Rerun without softening~~ — done (§7.6), and it escalated.** The gauge is restored to
   floating-point precision, and two further gauge breaks *inside the FTLE implementation* were found
   and fixed. But three of eight regions hit **genuine collision singularities** with `eps=0`, so a
   full sweep now **requires** Levi-Civita / global regularisation — not as an accuracy refinement
   but to be runnable at all. On the five collision-free regions the ranking agreement with the
   softened sweep is 16/28, i.e. chance. **Only `E`-excluded and `KE`-strong survive both.**
9. **~~Implement global regularisation~~ — built (§7.8); rerun the sweep is the top item now.**
   Aarseth–Zare clears 7/8 regions to 1e-8..1e-11 (vs 5/8 unsoftened). `deep interior`'s residual is
   a near-triple encounter that pending-change 7 says to *classify and terminate*, not integrate.
   **The unsoftened field sweep can now be rerun over the full region set — this is what unblocks the
   provisional ranking.** Previously: §7.7 built and validated Levi-Civita
   (machine-precision passage through exact collision) and added pair switching, which fixes
   sequential binary encounters — near-field 1.7e+01 → 1.8e-05, far r≈10 5.1e+07 → 6.9e-11. But
   **triple encounters (two pairs close at once) still fail**, and those are exactly the regions the
   sweep needs. Aarseth–Zare / chain regularisation is the remaining piece. Everything downstream of
   the field ranking is provisional until it exists.
10. **Why do map exponents rise to ≈1 without softening?** Fields sit in the linear-response regime at
    `t=13`. Distinguish genuine physics from the softened runs' fixed-dt integration error
    (max drift 9.2e-3) masquerading as divergence.

---

## 10. Code

All in this directory; pure NumPy, no dependencies beyond it. Total compute for every result in this
document: **< 10 minutes** on one core.

| file | contents |
|---|---|
| `tb.py` | core: vectorised KDK leapfrog, softened gravity, energy, pair distances, outcome classification, `burrau_grid` (quad + footprint + ensemble-jitter construction) |
| `tb_ftle.py` | Benettin FTLE shadows with renormalisation; diffusion via O(1) regression accumulators; moment of inertia |
| `tb_events.py` | escape detection → `t_end` (censored at horizon); `binary_id` (tightest pair — event class, defined at every `t`) |
| `tb_all.py` | **combined integrator** — every candidate field in one pass, so a probe costs one integration rather than three |
| `refine_test.py` | shape-sphere (Hopf) mapping, spherical variance, disagreement fraction, `probe()`, `refine_series()` |
| `sweep.py` | the 8-region × 3-playhead × 3-jitter matrix used in §6.4 |
| `gauge_test.py` | dynamical-similarity rescaling test — does softening break the scale gauge (§7.5) |
| `tb_exact.py` | **unsoftened** integrator: exact Newtonian force, per-trajectory scale-covariant adaptive dt (§7.6) |
| `tb_all_exact.py` | unsoftened combined integrator — lockstep sync intervals, dimensionless Benettin metric and seeding |
| `sweep_exact.py`, `sweep_exact.json` | the unsoftened sweep over the collision-free regions |
| `tb_lc.py` | **Levi-Civita regularisation** — LC map, regularised Hamiltonian, RK4 in fictitious time (§7.7) |
| `tb_lc_switch.py` | LC with dynamic pair switching at sync boundaries |
| `tb_az.py` | **Aarseth–Zare global regularisation** — two pairs regularised simultaneously (§7.8) |
| `sweep.json` | raw sweep output |

### Reproducing

```bash
# integration quality first -- this mattered (§2.2)
python3 -c "
import numpy as np, tb
r0,v0,_,_,_ = tb.burrau_grid(4,4,1.0,3.0,0.2,ens=0)
for eps in (0.01,0.03,0.1):
  for dt in (2e-3,5e-4):
    res = tb.integrate(r0,v0,t_max=20.0,dt=dt,eps=eps)
    print(eps, dt, f'{np.median(res[\"drift\"]):.1e}')
"

# the ratio is flat (§3.1)
python3 -c "
import refine_test as rt
rt.refine_series('boundary', 1.333, 2.513, 0.10, levels=4,
                 N=8, ens=3, t_max=20.0, dt=5e-4, eps=0.03)
"

# full field sweep, both axes (§6.4) -- ~2 minutes
python3 -c "
import sweep, json
D = {}
for nm,cx,cy,bd in sweep.REGIONS:
    D[nm] = {f't={t}': sweep.probe(cx,cy,t,0.5,body=bd) for t in (3.,7.,13.)}
    D[nm].update({f'jf={j}': sweep.probe(cx,cy,13.,j,body=bd) for j in (0.25,1.0)})
json.dump(D, open('sweep.json','w'))
"
```

### Parameters used throughout

`eps = 0.03`, `dt = 5e-4`, `jitter_frac = 0.5`, `d0 = 1e-8` (Benettin), `renorm_every = 200` steps
(= 0.1 time units; at λ≈1 the shadow grows ×1.1 per interval, comfortably linear).

---

## 11. Production notes — regularisation on the GPU

Design implications of §7.7–§7.8 for the shader integrator. **Nothing here is GPU-benchmarked** —
all timings are vectorised NumPy on one core, so ratios are indicative, not predictive.

### Cost

Per step AZ is roughly **5–10× a leapfrog step** (RK4 = four derivative evaluations, plus the LC
matrix applies and the `R3` term). But it takes **drastically fewer steps** through exactly the
encounters that otherwise cost millions of substeps. Measured end-to-end, near-field at `t=13`:

| integrator | wall time | median drift |
|---|---|---|
| unsoftened adaptive | 4.3 s | 5.0e-04 |
| **AZ global** | **3.8 s** | **6.0e-11** |

Slightly *faster*, at ~9 orders of magnitude better accuracy. In smooth regions AZ is pure overhead;
in encounter-rich regions it wins by a wide margin.

### The GPU concern is divergence, not flops

Mixed lanes (some AZ, some Cartesian) make a warp run both paths; and AZ's **reference body changes
per trajectory over time**, so even within AZ there are three variants. Two viable answers:

- **AZ everywhere, always** — uniform path, zero divergence, some wasted work in smooth regions. On
  GPU this often beats the "smarter" branching version.
- **Dispatch at quad granularity** — classify a quad as encounter-prone and route it to a separate
  kernel, making divergence *inter*-quad and therefore free. Fits the existing architecture, since
  quads are already the dispatch unit. Caveat: encounter-ness is spatially correlated early and less
  so late, as chaos separates samples within a quad.

### It is not "another integrator" in the payload sense

AZ is not merely different stepping — it is a different **state representation**
(`u1,p1,u2,p2` plus a reference-body label, instead of `r,v`). Normally that would be invasive.
**Because the design is lockstep, it need not be:** AZ can live entirely *inside* a sync interval,
with Cartesian as the canonical state at the boundaries — which is how `tb_az.py` is built.

**Consequence: `SimState` does not change.** Regularisation is a per-interval integration strategy,
not a new payload format or a new stored field.

Two things that already fit:

- **Variable work per sample is designed in.** `total_substeps` is a `SimState` field and
  substep-saturation is already a stop condition. AZ needs different numbers of `tau`-steps per
  trajectory to reach the same `t`; the architecture already expects that.
- **LC *improves* f32 conditioning near collisions.** `u = sqrt(r)`, so where `r` is tiny and poorly
  represented, `u` is larger and better represented. Regularisation helps precision exactly where f32
  struggles most — a GPU win, not only an accuracy one.

### Open choice: RK4 is probably the wrong production integrator

RK4 is a poor GPU inner loop — four force evaluations, not symplectic, and **not time-symmetric**,
which matters because the time-reversal measure `xi` depends on it. `Gamma` is not separable, so plain
leapfrog does not apply. But **time-transformed / logarithmic-Hamiltonian leapfrog**
(Mikkola–Tanikawa; Preto–Tremaine) gives collision-regularising behaviour at roughly leapfrog cost
while staying symplectic and time-symmetric. That is likely the better production choice.

**AZ + RK4 was the right tool to prove the physics, not necessarily the one to ship.**

### Verification practices that earned their keep

1. **Finite-difference the regularised Hamiltonian.** Two sign errors in `dGamma/du` produced drift
   that sat at ~1 and *would not fall with step size* — the signature of a wrong equation, not a
   step-size problem. FD localised it to the exact components in one run.
2. **Test the scale gauge by rescaling with no parameter adjusted.** Caught softening's 1.66×
   violation, and two further gauge breaks inside the FTLE implementation that were unrelated to it.
3. **Watch for step-size rules that duplicate what the method already does.** `dt = A·B·dtau` already
   shrinks the physical step at close approach; shrinking `dtau` too drove `dt -> 1e-13`. The original
   "deep interior fails" conclusion was this bug, not physics.
