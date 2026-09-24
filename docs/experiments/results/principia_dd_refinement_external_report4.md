# Brief 4 — horizons

**Run:** 244 probes (31232 trajectories) + 135 precision points, ~75 min wall.
No gate anywhere; every footprint carried all `E+1` copies in all 244 probes — verified,
not assumed: `len(gid) == (ens+1)*N^2` holds for every record.
`eta = 0.01`. `deep interior` skipped. Raw JSON in `xp_results4/`, tables in
`xp_results4/tables.md`.

New code: `xp_prec.py` (float-width-parametric AZ driver), `xpD_jitter.py`,
`xpD_f32.py`, `xpD_escape.py`, `xpD_spacing.py`, `xpD_report.py`, `xpD_lambda.py`.
Harness untouched. `xp_nogate.run_probe` gained one output field (`cls`).

---

## Before the tables: four things in the framing that did not hold

I would rather lead with these than with the numbers, since that is what the last
three reports were useful for.

### 1. `alpha_E` cannot see the measurement horizon. It is not a matter of resolution.

Experiment 1 asks which of two walls binds and proposes to find out by watching
`alpha_E` depart from 1.0. But each trajectory conserves its own energy, so
`sigma_E(t) = sigma_E(0)` **no matter how thoroughly the ensemble has decorrelated**.
A footprint whose eight copies have flown to completely different outcomes still has
exactly the energy spread it started with. The measurement horizon — jitter growing to
system size — leaves `sigma_E` untouched by construction.

So `alpha_E` is a pure numerical-error meter. Whatever it finds is the representability
side, and the experiment as specified could only ever have returned that answer.

The measurement horizon is visible, just not there. Measured on the shape vector,
same probes, same 16× jitter lever:

| region | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 |
|---|---|---|---|---|---|---|---|
| near-field `alpha_shape` | **+0.411** | +0.525 | +0.517 | +0.566 | +0.478 | +0.729 | +0.860 |
| near-field `alpha_E` | +1.003 | +0.986 | +0.986 | +1.000 | +0.991 | +0.990 | −1.595 |
| mid-field `alpha_shape` | +0.858 | +0.843 | +1.027 | +0.626 | +0.645 | +0.853 | +0.835 |
| body2 core `alpha_shape` | **+0.438** | +0.631 | +0.705 | +0.429 | +0.274 | +0.851 | +0.308 |

`alpha_shape` is already at 0.41 at t=13 in near-field, where `alpha_E` reads 1.003.
**The measurement horizon has been crossed before the earliest horizon either of us has
been sampling, and the representability horizon is more than twenty time units further
out.** The near wall binds, by a wide margin, and it is the one the instrument is blind
to. If a single number is wanted for "how far can the playhead go before refinement stops
buying anything", it comes from `alpha_shape`, and it is well under 13 — not 36.

### 2. There is no wall. There is a rate.

Nothing in 120 probes jumps from working to broken at any t. What happens is that
footprints go bad one at a time. Fraction of the 16 footprints whose own `error_ratio`
leaves `[1/1.05, 1.05]`, regressed on `t` across all regions and jitters:

**+0.0189 per time unit, std err 0.0018, t = +10.5** (n = 120 cells).

About 1.9 percentage points of the image per unit of playhead. At t=13 that is ~4% of
footprints; by t=40 it is 50–80% in near-field. A threshold crossing has to be *invented*
by picking an aggregate and a tolerance, and which t you get depends entirely on those two
choices — which is the next item.

### 3. Most of "fails at t=40" is the footprint aggregator, not the arithmetic.

`sigma_E` is computed per footprint and then combined across the 16 footprints. The brief
does not say how, and the harness reduction takes a **mean**. One bad footprint moves a
mean. The same data, combined by median instead — nothing else changed:

| region | agg | t=13 | t=20 | t=27 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|
| near-field | mean | +1.003 | +0.986 | +1.000 | +0.990 | −1.595 | **−1.778** |
| near-field | median | +1.008 | +0.999 | +1.000 | +0.982 | +1.101 | **+0.979** |
| mid-field | mean | +1.001 | +0.999 | +1.009 | +1.030 | +0.339 | **−2.294** |
| mid-field | median | +1.001 | +1.005 | +1.010 | +0.965 | +1.004 | **+1.005** |
| body2 core | mean | +1.013 | **−4.035** | −3.370 | +1.469 | −2.522 | **−2.205** |
| body2 core | median | +1.030 | **+1.001** | +1.002 | +0.983 | +0.996 | **+0.964** |

Median-aggregated, the known-answer control holds to within 0.04 of 1.0 at **t=40 in all
three regions**. `body2 core` at t=20 moves from −4.035 to +1.001 on the aggregator alone.

This cuts both ways and the second half matters more. The exponent passing does **not**
mean the image is sound: at t=40 near-field the median-aggregated `alpha_E` reads +0.979
while **50–81% of the footprints are individually broken**. The median survives precisely
by ignoring them. `alpha_E` is a property of the aggregation, not of the trajectories, and
it should not be the thing that certifies a frame.

### 4. Experiment 4's lever is the quad width, not `delta`.

`delta = jf · 2·half/(N−1)`, so jitter and quad size move it identically and the brief
treats them as the same knob. They are not. Pooling Experiments 1 and 4 (128 deduplicated
cells) lets the two be separated, because Experiment 1 moves `delta` at fixed `half`:

| term | coefficient | std err | t |
|---|---|---|---|
| `ln(delta)` | +0.0142 | 0.0142 | +0.99 |
| `ln(half)` | **+0.0522** | 0.0255 | **+2.03** |
| `t` | +0.0157 | 0.0017 | +9.15 |

The jitter term is null; the quad-width term is the one carrying signal. Refinement helps
because the quad covers less pathological territory, not because the copies sit closer
together. That is a different mechanism from the one `delta < tau·e^(−lambda t)` describes,
and it means the budgeting rule cannot be derived from the jitter scale.

---

## Experiment 1 — which horizon binds

3 regions × 5 jitter fractions (0.03125 → 0.5, a 16× span) × 8 horizons, `half=0.05`,
ens=7, no gate. **n = 120 probes, 15360 trajectories.**

### The discriminating test

Failure fraction regressed on `ln(delta)` and `t` together, pooled over regions:

| term | coefficient | std err | t | reading |
|---|---|---|---|---|
| `ln(delta)` | **−0.0090** | 0.0151 | **−0.60** | null |
| `t` | +0.0189 | 0.0018 | +10.53 | strong |

Predicted shift if the wall were jitter-set: `ln(16)/lambda = 2.77` time units across the
span. Measured: `−0.0090` per e-fold, equivalent to **1.3 time units total with a standard
error covering zero**. Held at 16× the jitter range, the failure point does not move.

**Verdict: REPRESENTABILITY.** The wall is fixed in the jitter, exactly as your test was
designed to detect. The predicted `ln(delta_2/delta_1)/lambda` shift is not there.

Three qualifications, in decreasing order of how much they should change what you do:

- This is the answer to the question `alpha_E` is capable of answering (framing point 1).
  The measurement horizon exists and binds much earlier; it is just invisible to this meter.
- "The wall" is a rate of 1.9 pp/time unit, not a crossing (point 2).
- Its apparent location moves by more than 20 time units depending on the footprint
  aggregator (point 3).

### The meter that tracks the damage

`error_ratio` as specified (median over footprints) against the max-aggregated version
Brief 3 recommended, both at jf=0.5:

| region | stat | t=13 | t=20 | t=24 | t=27 | t=30 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|---|
| body2 core | median | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.080 | 1.080 |
| body2 core | **max** | 1.065 | 1.759 | 4.269 | **17.44** | **1561** | **4700** | 1561 | 1561 |
| near-field | median | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.000 | 1.042 | 1.042 |
| near-field | **max** | 1.001 | 1.000 | 1.000 | 1.000 | 1.013 | 1.000 | **11.56** | 11.56 |

Across 22 (region, horizon) cells, `error_ratio_max` predicts exponent damage at
**Spearman +0.956**; the median manages **+0.599**. Dynamic range 1.003 → 1.0e7 against
0.9997 → 1.588. This is an independent confirmation of Brief 3's recommendation on new
data, and now with a monotone dose-response rather than a single flag.

---

## Experiment 2 — the float32 horizon

**Prediction was ~16. Measured: t ≈ 1–2 for AZ.** An order of magnitude short, and for a
reason the `ln(R/eps)/lambda` derivation does not contain.

Three arms separate representability from arithmetic, plus the softened leapfrog at both
widths. 3 regions × 9 horizons × 5 arms = **135 points**.

**Driver control:** `xp_prec.integrate_az_prec` at float64 reproduces `tb_az.integrate_az`
**bit-for-bit** — `max|dr| = 0.000e+00` at t=5 and t=13. It reuses `AZSystem.deriv`,
`.to_reg`, `.to_cartesian` and `choose_reference` verbatim; only the bookkeeping loop is
retranscribed, because the harness pins float64 in `to_reg`'s zero column and in the step
loop's `.astype(float)`. I would not have trusted the float32 numbers without this.

`alpha_E`, near-field:

| arm | t=1 | t=2 | t=5 | t=8 | t=10 | t=12 | t=15 | t=20 | t=25 |
|---|---|---|---|---|---|---|---|---|---|
| f64 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +0.985 |
| **f32-IC** | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +1.000 | +0.984 |
| **f32-arith** | **−0.028** | +0.065 | +0.002 | +0.141 | +0.105 | +0.226 | −0.132 | +0.305 | −0.025 |
| lf64 | +1.000 | +0.995 | +0.995 | +1.026 | +1.042 | +1.040 | +0.908 | +1.078 | +0.845 |
| lf32 | +1.000 | +1.034 | +1.011 | +0.898 | +0.945 | +0.891 | +0.772 | +0.762 | +0.604 |

mid-field: f32-arith is +0.083 by t=2. body2 core: +0.086 at t=1.

**`f32-IC` matches `f64` to three decimals in every one of the 27 cells.** Writing the
state at float32 precision and evolving it exactly costs nothing measurable out to t=25 —
so the representability horizon is *not* the binding constraint even in float32. The
prediction fails because the failure is arithmetic.

**It is AZ specifically, not gravity.** The unregularised softened leapfrog in float32
holds `alpha_E` at +0.983 at t=5 and +0.945 at t=10, where AZ in float32 is already at
+0.002. Against its own float64 arm, `lf32` degrades gracefully from about **t=8**.

Two supporting diagnostics, since a figure this far off the prediction wanted checking
before I believed it:

- Reducing the step size makes float32 AZ **worse**, not better (`|dE/E|` 0.073 → 0.150
  when `eta` goes 0.01 → 0.002). That is the round-off-dominated signature: RK4 truncation
  is already far below round-off, so more steps only accumulate more of it.
- The one-time drift jump between the first and second sync appears at **both** widths
  (f64: 3.9e-15 → 4.5e-10; f32: 1.8e-07 → 8.0e-02), with the ratio tracking
  `eps32/eps64`. It is the system leaving rest and starting to do work, not a defect in
  the new driver, and not a close encounter — the nearest approach in that window is 2.66,
  with the real encounter at t≈2.0 (min separation 1.5e-04).

**Verdict: FALSIFIED, and the consequence is larger than the number.** A float32 GPU path
must not run Aarseth–Zare. It could run the softened leapfrog to t≈8, or carry AZ's state
in float64 while keeping the rest of the pipeline narrow. The ~16 figure is safe only as a
statement about *storing* state, which is not where the cost is.

---

## Experiment 3 — is the escape statistic shadowing-robust?

4 regions × 3 horizons × 5 seeds, jitter ~1e-14 (measured `sigma_E(0)` = 3.95e-15),
**n = 60 probes, 7680 trajectories.** Copy 0 is unjittered and therefore bit-identical
across seeds; it is excluded from every cross-seed number, since including it would
manufacture agreement that is not in the data.

| region | t | region frac | across-seed sd | per-footprint frac sd | identity agreement | majority identity stable |
|---|---|---|---|---|---|---|
| near-field | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| near-field | 40 | 0.1250 | 0.0000 | 0.0000 | 0.9821 | 0.94 |
| near-field | 80 | 0.5696 | **0.0131** | 0.0175 | 0.9643 | 1.00 |
| mid-field | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| mid-field | 40 | 0.1250 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| mid-field | 80 | 0.3625 | **0.0091** | 0.0162 | 0.9661 | 0.94 |
| body2 mid | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body2 mid | 40 | 0.1875 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body2 mid | 80 | 0.5000 | **0.0000** | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 20 | 0.0000 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 40 | 0.1250 | 0.0000 | 0.0000 | 1.0000 | 1.00 |
| body1 slice | 80 | 0.4933 | **0.0074** | 0.0151 | 0.9621 | 1.00 |

**Verdict: CONFIRMED to t=80.** At t=20 and t=40 the escape fraction is *bit-reproducible*
across five independent round-off-scale perturbations — sd exactly 0.0000. At t=80 it moves
by 0.007–0.013 on fractions of 0.36–0.57, i.e. **2–3% relative**.

The predicted split did not appear, though. Identity was supposed to scramble while the
fraction held; instead identity also survives — 96–100% of copies agree with their
footprint's majority, and the majority itself is seed-stable in 94–100% of footprints. The
fine claim is in as good shape as the coarse one at these horizons.

**But this does not license the t=240 claims, and the reason is not the one in the brief.**
Seeded at 1.5e-15, the ensemble has spread to only ~3e-3 by t=80 — still two and a half
orders below system size. It is *not yet decorrelated*, which is precisely why escape is
reproducible. Reproducibility at t=80 is evidence about t=80. Full decorrelation arrives
somewhere around t≈105–115 (next section), and t=240 is well past it. The t=240 results
should stay marked indicative — not because they sit 6.7× past a horizon at 36, but because
this test has not been run at that playhead. It extends cheaply: the same 20 probes at
t=240 would settle it.

---

## `lambda` is not 1. It is 0.6–0.8, and every horizon in the brief divides by it.

This was not on the list, but `t_max = ln(1/eps)/lambda` has `lambda` as its only free
parameter, and Experiment 3's ensemble measures it directly for free — seeded at round-off,
so there are many e-folds of clean exponential growth before saturation. Growth of the
shape-vector spread (scale-quotiented, so gauge-independent):

| region | s(0) | s(20) | s(40) | s(80) | λ[0–20] | λ[20–40] | λ[40–80] | esc@40 | esc@80 |
|---|---|---|---|---|---|---|---|---|---|
| near-field | 1.51e-15 | 2.13e-09 | 1.21e-07 | 1.55e-03 | **0.708** | 0.202 | 0.236 | 0.125 | 0.569 |
| mid-field | 7.03e-16 | 4.08e-09 | 2.60e-06 | 2.86e-03 | **0.779** | 0.323 | 0.175 | 0.125 | 0.364 |
| body2 mid | 1.03e-15 | 4.32e-14 | 3.51e-12 | 1.06e-07 | 0.187 | 0.220 | 0.258 | 0.188 | 0.500 |
| body1 slice | 1.99e-15 | 4.53e-10 | 2.75e-07 | 2.90e-03 | **0.617** | 0.320 | 0.232 | 0.125 | 0.486 |

`lambda` is **0.62–0.78 in the bound phase** (t=0–20) for three of four regions, against the
assumed 1.0. It then drops to 0.18–0.32, and I do not think that late figure is the chaotic
rate: the escape fraction reaches 0.36–0.57 over the same interval, and **an ionised system
is not chaotic** — once a body is away, what is left is a regular two-body problem that
stops amplifying. The late-interval number is the bound rate diluted by escapers, so it is a
floor, not an estimate.

At `lambda = 0.7` the derived horizons become **t_f64 ≈ 52** (not 36) and **t_f32 ≈ 23**
(not 16). Both move the same way: further out. Three caveats worth carrying — this is
measured on `n_hat` rather than a phase-space FTLE; it is an ensemble spread rate rather
than a true Lyapunov exponent (they agree only for small separations, which does hold here);
and `body2 mid` at 0.187 shows it is regional, not a system constant.

The practical consequence: **`t` does not read as e-foldings.** Crossing time 0.965 makes
`t` read as crossing times, but at `lambda ≈ 0.7` one e-folding is ~1.4 time units, so
t=40 is about 28 e-foldings and not 40. Any budget computed by treating them as
interchangeable is off by ~40%.

---

## Experiment 4 — what a refinement level buys

2 regions × 4 quad half-widths (0.1 → 0.0125, 8× span) × 8 horizons at fixed jf=0.5.
**n = 64 probes.** Overlaps Experiment 1 exactly at `half=0.05, jf=0.5`; the two agree to
**zero relative difference** across all 15 shared cells, so no parameter drift between the
runners.

Failure fraction, near-field and mid-field:

| region | half | delta | t=13 | t=20 | t=27 | t=33 | t=36 | t=40 |
|---|---|---|---|---|---|---|---|---|
| near-field | 0.1 | 3.33e-02 | 0.062 | 0.250 | 0.312 | 0.500 | 0.812 | 0.812 |
| near-field | 0.05 | 1.67e-02 | 0.062 | 0.000 | 0.000 | 0.000 | 0.500 | 0.562 |
| near-field | 0.025 | 8.33e-03 | 0.000 | 0.062 | 0.000 | 0.062 | 0.438 | 0.500 |
| near-field | 0.0125 | 4.17e-03 | 0.000 | 0.000 | 0.250 | 0.062 | 0.688 | 0.812 |
| mid-field | 0.1 | 3.33e-02 | 0.000 | 0.000 | 0.125 | 0.375 | 0.188 | 0.188 |
| mid-field | 0.05 | 1.67e-02 | 0.062 | 0.000 | 0.062 | 0.188 | 0.250 | 0.438 |
| mid-field | 0.025 | 8.33e-03 | 0.000 | 0.000 | 0.000 | 0.062 | 0.000 | 0.188 |
| mid-field | 0.0125 | 4.17e-03 | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 | 0.125 |

mid-field is close to monotone in `half`. near-field is not — 0.0125 reads 0.812 at t=40,
worse than 0.05's 0.562. The effect is real but weak and the scatter is comparable to it.

**The budgeting rule, from the pooled fit (framing point 4):**

- Halving the **quad half-width** buys **≈ 2.3 time units** of playhead (`ln(half)`
  coefficient +0.0522 ± 0.0255, `t` = +2.03, divided by the failure rate 0.0157/unit).
- Halving the **jitter** buys **≈ 0.63 time units**, but the coefficient is
  +0.0142 ± 0.0142 — **not distinguishable from zero**.

The 0.63 is tantalisingly close to the predicted `ln(2)/lambda = 0.693`. I do not think
that should be leaned on: it is one standard error from zero, and the term that actually
carries signal is the one the prediction does not involve.

**Verdict: the rule holds in direction but not in magnitude or mechanism.** Refinement buys
about **2.3 time units per halving**, roughly 3× the predicted 0.69, and it buys them
through the quad width rather than the copy spacing. At `t` = 1.9 pp of ruined image per
time unit, one refinement level is worth about 3.6 percentage points of footprints saved.
Both marginal — `t` = +2.03 is one region away from nothing, and near-field's
non-monotonicity is a real caution against spending on this.

---

## Answers

**Experiment 1 — which horizon binds?** As posed, and on the evidence `alpha_E` is capable
of giving: **representability**. The failure point is fixed against a 16× jitter range
(`ln(delta)` coefficient −0.0090 ± 0.0151, t = −0.60), and the predicted 2.77-time-unit
shift is absent.

That answer is correct and nearly beside the point. `alpha_E` is structurally blind to the
measurement horizon, because each trajectory conserves its own energy and `sigma_E` is
therefore unmoved by decorrelation. Measured on the shape vector instead, the measurement
horizon has already been crossed at t=13 (`alpha_shape` = 0.411 in near-field, where
`alpha_E` = 1.003). **The measurement horizon binds, by more than twenty time units, and
the experiment could not have seen it.**

**Experiment 2 — verify the f32 horizon.** **Falsified: t ≈ 1–2, not ~16.** The
derivation is right about representability and wrong about what fails. Rounding the state
to float32 and evolving it in float64 costs nothing measurable out to t=25 (`f32-IC`
matches `f64` to three decimals in all 27 cells). float32 *arithmetic* destroys the control
by t=2. It is Aarseth–Zare specifically: the softened leapfrog in float32 holds `alpha_E` at
+0.983 at t=5 and degrades gracefully from t≈8. A float32 GPU path must not run AZ.

**Experiment 3 — is escape shadowing-robust?** **Confirmed to t=80.** Bit-reproducible at
t=20 and t=40 across five round-off-scale seeds; 2–3% relative at t=80. Identity survives
too, so the predicted coarse/fine split did not materialise. This does not reach t=240 —
the ensemble is still 2.5 orders below system size at t=80, which is why it reproduces.
The t=240 claims should stay indicative; twenty more probes would settle them.

**Experiment 4 — the refinement/horizon trade.** **Directionally right, wrong magnitude,
wrong mechanism.** One halving buys **≈ 2.3 time units** (not 0.69), and it buys them
through the *quad half-width* (`ln(half)` +0.0522 ± 0.0255, t = +2.03), not the jitter
(`ln(delta)` +0.0142 ± 0.0142, null). `delta` is not the controlling variable, so the rule
cannot be derived from `delta < tau·e^(−lambda t)`. Both effects are marginal and
near-field is non-monotone in `half` — I would not spend on this yet.

### What I would change

1. **Stop certifying frames with `alpha_E`.** It is a property of the footprint aggregator.
   Median-aggregated it reads +0.979 at t=40 in near-field while 50–81% of the footprints
   underneath are individually broken; mean-aggregated the same data reads −1.778. Report
   the **failure fraction** instead — it needs no aggregator and no tolerance, and it climbs
   at a measurable 1.9 pp per time unit.
2. **Use `error_ratio_max`, not `error_ratio`.** Spearman +0.956 against exponent damage
   over 22 cells versus +0.599 for the median, with dynamic range 1.003→1.0e7 against
   0.9997→1.588. Second independent confirmation of this, now with a dose-response.
3. **Re-derive the horizons with `lambda ≈ 0.7`, not 1.0**, and stop reading `t` as
   e-foldings. Crossing time 0.965 makes `t` crossing times; e-foldings are ~1.4 time units
   apart.
4. **If a float32 path is coming, decide the integrator first.** That choice costs an order
   of magnitude of playhead and precision is not the lever that fixes it.
