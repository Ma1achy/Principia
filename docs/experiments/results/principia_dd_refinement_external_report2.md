# Brief 2: is the `max` criterion well-posed?

**Run:** 260 probes, 33280 trajectories, 92 min compute, **0 failures**. Raw JSON in
`xp_results/` and `xp_results2/`. Gate `1e-4`, `eta = 0.01` (Brief 1 calibration),
`deep interior` skipped. Every quad is 4x4 = 16 samples x 8 copies = 128 trajectories.

**Harness untouched.** New files: `xpB_regress.py`, `xpB_domke.py`, `xpB_domke_report.py`,
`xpB_estimator.py`, `xp_tend.py`, `xpB_tend_run.py`, `xpB_tend_report.py`, `xpB_horizons.py`.

**Regression control passed before anything else was believed.** Experiments A and B are pure
post-hoc reductions of Brief 1's stored per-trajectory arrays, so I first re-derived Brief 1's
published KE win-shares through the new code path: **18/18 cells reproduce** REPORT.md to within
0.005 (`xpB_regress.py`).

**The patched `tb_az.py` reproduces Brief 1 bit-identically** — drift and KE arrays, NaN patterns,
retention, on both a clean probe and the pathological `eq_rot` core. Rows already `done` get
`h = 0`, so the fix changes only *when the loop exits*, never a finite trajectory's value. Brief 1's
raw data is therefore valid under the new harness, and A and B needed no new integration.

---

## Experiment A — the `dom_KE` sweep

25 quads at `jf = 0.5` (burrau 7, eq_rot 12 incl. intermediate radii, eq_fast 6), gate `1e-4`,
`alpha_E` over `jf ∈ {0.125, 0.25, 0.5, 1.0}`.

**Display-sensible** = median occupancy (`KE_spread/dom_KE`) in `[0.05, 0.5]` **and** clip fraction
(`> 1`) under 5%. **Configuration-stable** = KE's win share does not cross 0.5 inside that band.

### A1. Pooled per configuration (trustworthy quads)

| dom_KE | burrau winKE | eq_rot winKE | eq_fast winKE | clip (b/r/f) | occupancy (b/r/f) | display-sensible for |
|---|---|---|---|---|---|---|
| 0.125 | 1.00 | 0.97 | 1.00 | 0.49/0.56/0.00 | 1.233/1.022/0.174 | eq_fast |
| 0.25 | 1.00 | 0.92 | 1.00 | 0.35/0.47/0.00 | 0.616/0.511/0.087 | eq_fast |
| 0.5 | 0.96 | 0.80 | 1.00 | 0.19/0.29/0.00 | 0.308/0.255/0.044 | **none** |
| 1.0 | 0.92 | 0.64 | 1.00 | 0.12/0.14/0.00 | 0.154/0.128/0.022 | **none** |
| 2.0 | 0.92 | 0.46 | 0.93 | 0.09/0.05/0.00 | 0.077/0.064/0.011 | eq_rot |
| 4.0 | 0.90 | 0.22 | 0.55 | 0.07/0.02/0.00 | 0.039/0.032/0.005 | **none** |
| 8.0 | 0.80 | 0.12 | 0.50 | 0.04/0.01/0.00 | 0.019/0.016/0.003 | **none** |
| 16.0 | 0.54 | 0.11 | 0.49 | 0.03/0.01/0.00 | 0.010/0.008/0.001 | **none** |
| 32.0 | 0.22 | 0.10 | 0.03 | 0.01/0.00/0.00 | 0.005/0.004/0.001 | **none** |
| 64.0 | 0.12 | 0.06 | 0.00 | 0.01/0.00/0.00 | 0.002/0.002/0.000 | **none** |

**No `dom_KE` is display-sensible for more than one configuration, and `burrau` has none at all.**
Burrau is squeezed from both sides: at `dom_KE = 2` occupancy is fine (0.077) but clipping is 9%;
at 4 clipping is fine (7%) but occupancy has fallen to 0.039. There is no value in between that
fixes both.

### A2. Verdict on stability

| config | display-sensible values | winKE across them | crosses 0.5? |
|---|---|---|---|
| burrau | **none** | — | — |
| eq_rot | 2.0 only | 0.46 | no (single point) |
| eq_fast | 0.125, 0.25 | 1.00 | no (single regime) |

`eq_rot` and `eq_fast` each admit a band, but the bands are **disjoint**, and `eq_rot`'s single
admissible value puts KE at 0.46 — below the decision boundary — while `eq_fast`'s puts it at 1.00.
Nothing is stable across configurations because nothing is admissible across configurations.

### A3. Why — the dynamic range, which is the whole story

| config | field | p10 | p50 | p90 | max | **p90/p10** |
|---|---|---|---|---|---|---|
| burrau | shape | 7.89e-04 | 1.60e-02 | 5.37e-02 | 6.66e-01 | 68 |
| burrau | event | 1.25e-01 | 2.50e-01 | 4.36e-01 | 5.00e-01 | **3.5** |
| burrau | **KE** | 4.24e-03 | 1.13e-01 | 1.68e+00 | 6.43e+01 | **396** |
| eq_rot | event | 1.64e-01 | 5.00e-01 | 6.25e-01 | 7.50e-01 | **3.8** |
| eq_rot | **KE** | 1.96e-02 | 3.28e-01 | 2.00e+00 | 2.89e+01 | **102** |
| eq_fast | **KE** | 1.38e-02 | 2.17e-02 | 3.38e-02 | 4.49e-02 | 2.5 |

KE spans 396x within a single configuration where `event` spans 3.5x. **A single linear domain
cannot both avoid clipping the top decile and keep the bottom decile off the floor when the field
spans two-and-a-half orders of magnitude.** And the median KE spread differs 15x between `eq_rot`
(0.328) and `eq_fast` (0.022), so a domain tuned on one regime is wrong for the other.

### A4. The display-domain framing does not change the criterion at all

A colour ramp clips; it does not run away. So I scored KE **clamped at 1.0**, which is the
substantive difference from Brief 1's sensitivity sweep. It changes nothing:

| dom_KE | burrau clamped / unclamped | eq_rot | eq_fast |
|---|---|---|---|
| 0.25 | 1.0000 / 1.0000 | 0.9314 / 0.9314 | 1.0000 / 1.0000 |
| 1.0 | 0.9196 / 0.9196 | 0.6857 / 0.6857 | 1.0000 / 1.0000 |
| 4.0 | 0.9018 / 0.9018 | 0.2286 / 0.2286 | 0.5521 / 0.5521 |
| 16.0 | 0.5357 / 0.5357 | 0.0971 / 0.0971 | 0.4896 / 0.4896 |

Identical everywhere, because a clipped KE still equals 1.0, which still beats any bounded
contributor below 1.0. **The hypothesis that `dom_KE` is "the display domain" is coherent as
display semantics but has no effect on which field wins the `max`.** It does not make the
criterion well-posed; it only renames the free constant.

### A5. Per-quad scatter (the pooled numbers hide it)

At `dom_KE = 2.0`, KE's win share by quad: burrau 0.69–1.00; **eq_rot 0.19–1.00**; eq_fast 0.62–1.00.
Within `eq_rot` it still tracks radius — `b0 r1.6` gives 1.00, `b0 r2.2` gives 0.25 — so even at a
fixed domain the answer to "does KE dominate" depends on which quad you look at.

**Verdict: no value of `dom_KE` is simultaneously display-sensible and configuration-stable.
Recommendation: drop KE from the contributing set.** See §5.

---

## Experiment B — is `ols2`-fails-while-`theil`-passes recognisable?

43 quad-groups pooled from `exp1` (25 groups x 4 scales) and `exp3` (18 x 5 scales), gate `1e-4`,
trust bar `|alpha_E − 1| <= 0.05`.

### B1. Cross-tabulation

| | theil passes | theil fails |
|---|---|---|
| **ols2 passes** | 37 | 0 |
| **ols2 fails** | **4** | 2 |

Theil–Sen is never worse than `ols2` in 43/43 groups, and strictly better in 4 (9%).

### B2. The finding that changes the recommendation

**With two scales, Theil–Sen *is* `ols2`.** It is the median of a single pairwise slope. Verified
over 2000 random two-point cases: worst relative difference **5.1e-13**, i.e. the same estimator via
a different floating-point path. Theil–Sen's advantage in B1 comes entirely from consuming 4–5
scales, not from robustness.

Production has a parent quad and its children — two scales. **So "fall back to Theil–Sen" is not
available. The fallback must be to acquire a third perturbation scale.**

### B3. Is the condition detectable at runtime?

| diagnostic | both_ok (median) | ols2 fails, theil ok (median) | AUC |
|---|---|---|---|
| retained fraction | 1.00 | 0.557 | 0.08 (→ 0.92 inverted) |
| min surviving footprints | 16 | 10.5 | 0.14 (→ 0.86) |
| max drift | 7.0e-05 | 9.7e+03 | 0.92 |
| log10-drift IQR | 0.74 | 4.99 | 0.93 |
| log-log fit residual | 6.8e-04 | 4.4e-02 | **0.98** |

The fit residual separates best — but it needs >=3 scales, so **production cannot compute it**
(with two points the fit is exact and the residual is identically zero). Among diagnostics
available from a single probe:

| rule (fires ⇒ distrust ols2) | caught | false alarms | precision | recall |
|---|---|---|---|---|
| **retained < 0.90** | **5/6** | **1/37** | **0.83** | **0.83** |
| log10-drift IQR > 2.0 | 5/6 | 1/37 | 0.83 | 0.83 |
| min footprints < 16 | 5/6 | 1/37 | 0.83 | 0.83 |
| max drift > 1e-2 | 6/6 | 12/37 | 0.33 | 1.00 |

**Yes, it is recognisable — from retention alone.** The single failure retention misses is marginal:
`burrau` near-field at `eta=0.02`, `alpha_ols2 = 0.949`, i.e. `|alpha_E − 1| = 0.0510` against a
0.05 bar. It is a rounding-width miss, not a qualitative one.

**Recommendation:** when a quad retains under 90% of its trajectories, do not trust its two-point
exponent — measure a third scale. Retention is already computed for the gate, so this costs nothing
to detect.

---

## Experiment C — `t_end` with the escape cadence decoupled from sync

New module `xp_tend.py`; `tb_all_az.py` untouched. Escape tested every `dt_test = 0.05`
independent of any sync interval, then **bisected 8 times inside the firing window**. Resolution
`1.95e-04` against the old 0.4 — a factor of 2048.

### C1. Resolution recovered

| config | region | t | fp resolved /16 | esc frac | retained | t_end spread (parent) |
|---|---|---|---|---|---|---|
| burrau | mid-field | 13 | **16/16** | 1.00 | 1.00 | 1.43e-02 |
| burrau | body2 mid | 13 | **16/16** | 1.00 | 1.00 | 4.41e-03 |
| eq_fast | b0/b1/b2 core | 13 | **0/16** | 1.00 | 1.00 | 0.00e+00 |
| burrau | near-field | 60 | 15/16 | 0.76 | 0.90 | 5.18e+00 |

Brief 1's sync-grid method resolved **1/16** at best and 0/16 typically.

### C2. The exponent

| config | region | t | **alpha t_end** | alpha_E control | trust |
|---|---|---|---|---|---|
| burrau | mid-field | 13 | **0.9464** | 1.0014 | ok |
| burrau | body2 mid | 13 | **0.8588** | 1.0009 | ok |
| eq_fast | b0/b1/b2 core | 13 | **nan** | 0.999–1.002 | ok |
| burrau | near-field | 60 | **0.4911** | 0.9418 | **UNTRUSTWORTHY** |

`t_end` has a well-defined exponent of **0.86–0.95** in the two Burrau regions that ionise at t=13,
with trustworthy controls — consistent with, and better resolved than, Brief 1's fine-sync
0.76–0.85. **`t_end` carries usable signal. Its exclusion rested on a measurement artefact.**

### C3. Quantifying the Brief 1 caveat

Restricting to escaped copies makes the surviving subset depend on perturbation size, which breaks
the linearity `alpha_E` assumes. Measured directly:

| region | t | esc frac | alpha_E escaped-only | alpha_E all gated | difference |
|---|---|---|---|---|---|
| mid-field | 13 | 1.00 | 1.0014 | 1.0014 | **0.0000** |
| body2 mid | 13 | 1.00 | 1.0009 | 1.0009 | 0.0000 |
| eq_fast cores | 13 | 1.00 | 0.999–1.002 | same | 0.0000 |
| near-field | 60 | **0.76** | 0.9418 | 0.9955 | **0.0537** |

**The caveat bites exactly and only when escape is partial.** Where every copy escapes the
restriction is a no-op; where 24% do not, it moves the control by 0.054 — just across the 0.05 trust
bar, which is precisely why near-field t=60 fails it. So the rule is clean: **trust `t_end`
exponents only where the escape fraction is ~1.0.**

### Anomaly — `eq_fast` is degenerate, and not for a resolution reason

All three `eq_fast` quads return `t_end = 1.95e-04` for every trajectory — exactly the bisection
floor. Escape fires in the *first* test window: at `E > 0` with `omega = 1.2` the third body is
already unbound and receding at t=0. **These systems are born ionised, so `t_end` is identically
zero and carries no information there.** That is physics, not instrument — the opposite of the
Burrau case, where the zeros *were* the instrument.

---

## Experiment D — Brief 1's Experiment 2, completed, plus t=240

**All 40 probes now succeed. 17 failed in Brief 1; 0 fail now.** near-field t=120 parent went from a
1800 s timeout to 82 s. The patch is what did it.

### D1. Escape fraction vs horizon (gate 1e-4, parent quad)

| region | t=13 | t=30 | t=60 | t=120 | t=240 |
|---|---|---|---|---|---|
| near-field | 0.00 | 0.01 | 0.49 | 0.86 | **1.00** |
| mid-field | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| far r~10 | 0.00 | **1.00** | 1.00 | 1.00 | 1.00 |
| body2 mid | 0.98 | 1.00 | 1.00 | 1.00 | 1.00 |
| body1 slice | 0.00 | 0.06 | 0.50 | 0.95 | **1.00** |

**Every region reaches complete ionisation.** `far r~10` goes 0.00 → 1.00 between t=13 and t=30 —
it was never bound, merely slow. **This overturns Brief 1's Experiment 2 conclusion outright:**
"100% censored in three of four regions" was entirely an artefact of a 13-unit horizon, and my own
Brief 1 verdict that those regions "are bound, and waiting does not help" was wrong. Waiting helps
in every one of them.

### D2. But the gate collapses first — the two curves cross

| region | t=13 | t=30 | t=60 | t=120 | t=240 | footprints with >=3 copies @ t=240 |
|---|---|---|---|---|---|---|
| near-field | 0.98 | 0.95 | 0.43 | 0.16 | 0.10 | **0/16** |
| mid-field | 0.98 | 0.98 | 0.77 | 0.55 | 0.34 | 8/16 |
| far r~10 | 1.00 | 0.29 | 0.19 | 0.10 | 0.09 | 2/16 |
| body2 mid | 1.00 | 0.98 | 0.77 | 0.50 | 0.34 | 11/16 |
| body1 slice | 1.00 | 0.93 | 0.38 | 0.15 | 0.12 | 1/16 |

At t=240 near-field retains 10% and **not a single footprint clears the 3-copy minimum**. So the
horizon at which escape becomes universal is past the horizon at which the data survives the gate.
For near-field there is no horizon where both hold: at t=60 escape is 0.49 with retention 0.43; at
t=120 escape is 0.86 with retention 0.16.

**This sharpens Brief 1's finding that a gate calibrated at t=13 does not transfer.** The binding
constraint on long-horizon rendering is the drift gate, not the integrator.

### Anomaly — the gate biases *against* escape, opposite to the obvious worry

Escapes are powered by close encounters, which is where AZ works hardest, so escaped trajectories
carry *higher* drift (near-field t=30: median 3.6e-02 escaped vs 2.6e-07 censored). The gate
therefore removes escapers preferentially and **under**-states escape fraction:

| region | t | esc (gated) | esc (all trajectories) | difference |
|---|---|---|---|---|
| near-field | 60 | 0.49 | 0.66 | **−0.17** |
| body1 slice | 60 | 0.50 | 0.69 | **−0.19** |
| far r~10 | 30 | 1.00 | 0.98 | +0.02 |

I checked this expecting the opposite bias (that easy-to-integrate escapers would be
over-represented). It runs the other way, by up to 0.19. The D1 numbers are therefore conservative,
which strengthens rather than weakens the conclusion.

---

## 5. The two questions, in my own words

### 5.1 Recommended `dom_KE`: **there isn't one. Drop KE.**

No value is display-sensible for more than one configuration, and `burrau` — the configuration the
original 69–100% claim was measured on — admits **no** display-sensible value at all. The reason is
structural, not a matter of searching harder: KE's spread spans **396x** within one configuration
where `event` spans 3.5x. A single linear domain cannot serve a field with that dynamic range, and
no amount of tuning changes that. The display-domain hypothesis is coherent as display semantics but
**provably does not affect which field wins the `max`** (§A4) — it relocates the free constant
rather than eliminating it.

The contributing set should reduce to the two bounded fields, **shape n̂ and event class ⊕ detail**,
whose normalisations are achievable maxima of bounded quantities and need no tuning.

I want to be straight about what this costs. In `burrau` at any sensible domain KE wins 0.90–1.00 of
footprints, so dropping it will change `ensemble_spread` materially there — shape and event take
over at much lower normalised values (§A2 pooled: at `dom_KE=4`, shape 0.30 / event 0.17). The
criterion becomes less sensitive in the free-fall regime. That is a real loss, and it is the price
of a well-posed criterion rather than a tuned one.

### 5.2 Is `max`-of-normalised the right aggregation? **No, and the reason generalises.**

The defect is not that `dom_KE` was picked badly. It is that **`max` over normalised contributors
makes the normalisation constants the criterion.** Whichever field is normalised most loosely wins
almost every footprint, so the constants are doing the work of a decision threshold while looking
like units. Brief 1's finding — the verdict flipping between `/1.0` and `/2.0` — is that mechanism
showing through, and this brief shows it is not fixable by choosing better constants.

Two further properties make `max` a poor fit here. It is **winner-take-all**, so it discards the
information that two contributors agree — which is exactly the evidence that a footprint is
genuinely indeterminate rather than noisy in one channel. And it is **discontinuous in the
constants**: an arbitrarily small change to `dom_KE` near a crossover flips a footprint's winner and
therefore its refinement decision, which is not a property you want in a scheduler.

What I would put in its place, in order of preference:

1. **Bounded contributors only, aggregated by `max`.** Keep the current rule, drop KE. `max` is
   defensible once every contributor is a bounded quantity normalised by its achievable maximum,
   because then the normalisations are facts rather than choices. This is the smallest change and it
   is well-posed.
2. **Rank/quantile transform, if KE-like information must be retained.** Map each field's spread to
   its empirical quantile within the quad. Bounded in `[0,1]` by construction, free of tuning
   constants, and invariant under any monotone rescaling — so the 396x dynamic range stops
   mattering. The cost is that it is relative to the quad, so a uniformly-uncertain quad and a
   uniformly-certain one look alike; that needs a magnitude gate alongside, exactly as
   `refine_test.py`'s docstring already anticipates for the ratio criterion.
3. **Not a squashing function.** `x/(x+x0)` bounds KE but still needs `x0`, so it reintroduces the
   same free constant with the same power to decide the verdict. It looks principled and isn't.

I would take option 1 now and treat option 2 as the thing to test if dropping KE proves to cost too
much sensitivity in the free-fall regime.

---

## Anomalies and negative results

- **The display-domain hypothesis is provably inert** (§A4). Clamping KE at 1.0 — the honest
  rendering of "the ramp clips" — leaves every winner share bit-identical. This was the brief's
  central hypothesis and it does not survive contact with the arithmetic.
- **`ols2` cannot be rescued by switching to Theil–Sen** (§B2). At two scales they are the same
  estimator to 5e-13. Brief 1's framing of this as an estimator-robustness finding was wrong; it is
  a number-of-scales finding.
- **My Brief 1 Experiment 2 conclusion was wrong** (§D1). I wrote that the censored regions "are
  bound, and waiting does not help". Every one of them ionises completely by t=240, and `far r~10`
  by t=30. The timeouts that forced that reading were the harness bug, not physics.
- **The gate biases against escape, not toward it** (§D anomaly). I checked expecting the opposite
  sign and found up to −0.19.
- **`eq_fast`'s `t_end` zeros are physical, not instrumental** (§C anomaly) — the mirror image of
  Brief 1's mid-field zeros, and worth distinguishing carefully because they look identical in a
  table.
- **`eq_rot` KE win-share still ranges 0.19–1.00 across quads at fixed `dom_KE`.** Even with the
  normalisation question settled, "does KE dominate" has no single answer within one configuration.
