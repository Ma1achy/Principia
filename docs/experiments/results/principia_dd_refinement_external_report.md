# Results: three refinement experiments

**Run:** 248 probes, 31744 trajectories, 0 failures after the Brief 2 re-run.
See `REPORT2.md` for the follow-up study. All raw per-trajectory
measurements in `xp_results/*_raw.json`; derived tables in `xp_results/*_tables.md`.

**Harness untouched.** New files only: `xp_common.py`, `xp_reduce.py`, `xp_probe.py`,
`xp_driver.py`, `xp{1,2,3}_*.py`.

**Global gate accounting** (all experiments pooled, 31744 trajectories):
median drift 2.31e-08. Fraction gated out: **12.1% @1e-3**, 15.4% @1e-4.
(Higher than the pre-Brief-2 figure because the completed long horizons, which drift
most, are now included rather than timing out.)

Every quad is 4x4 = 16 samples with 7 ensemble copies each (8 per footprint,
128 trajectories per probe). `deep interior` (0,0) body 0 skipped throughout.

---

## Experiment 3 — drift gate calibration

6 regions (5 Burrau + 1 equal-mass rotating), 5 jitter scales
`jf in {0.125,0.25,0.5,1.0,2.0}`, 3 tolerances `eta in {0.02,0.01,0.005}`,
5 thresholds applied **offline** to stored drift arrays. 90 probes, 0 failures.

### Pooled, at production tolerance eta=0.01

| thr | median \|a_E−1\| (ols2) | median \|a_E−1\| (theil) | mean retained | worst-region err |
|---|---|---|---|---|
| 1e-02 | 0.0247 | 0.0005 | 0.940 | 0.2728 |
| 1e-03 | 0.0083 | 0.0005 | 0.923 | 0.1266 |
| **1e-04** | **0.0015** | 0.0011 | **0.909** | **0.0602** |
| 1e-05 | 0.0046 | 0.0040 | 0.890 | 0.2343 |
| 1e-06 | 0.0163 | 0.0083 | 0.840 | 0.1619 |

**The knee is at 1e-4, not 1e-3.** Control error is minimised there (0.0015 vs
0.0083) and the cost is 1.4 percentage points of retention (0.909 vs 0.923).
Below 1e-4 the error rises again — that is the "discards good data and biases
the sample" arm, and it is real, not monotone noise.

### Interaction with integrator tolerance (pooled)

| eta | 1e-2 | 1e-3 | 1e-4 | 1e-5 | 1e-6 | retained @1e-4 |
|---|---|---|---|---|---|---|
| 0.02 | 0.0890 | 0.0228 | 0.0172 | 0.0295 | 0.0711 | 0.873 |
| 0.01 | 0.0247 | 0.0083 | 0.0015 | 0.0046 | 0.0163 | 0.909 |
| 0.005 | 0.0011 | 0.0011 | 0.0011 | 0.0021 | 0.0021 | 0.932 |

**Yes, and strongly.** At `eta=0.005` the control error is flat at 0.0011 across
the whole threshold range — the gate stops mattering — while retention is
uniformly higher. Tighter `eta` lets a tighter gate retain more, exactly as
predicted. At `eta=0.02` no threshold reaches the 0.05 trust bar reliably.

### Per-region, eta=0.01 (the scatter the pooled table hides)

| region | 1e-2 | 1e-3 | 1e-4 | 1e-5 | 1e-6 | retained @1e-3 |
|---|---|---|---|---|---|---|
| near-field | 0.0053 | 0.0053 | 0.0059 | 0.0100 | 0.0153 | 0.998 |
| mid-field | 0.0018 | 0.0018 | 0.0018 | 0.0013 | 0.0173 | 1.000 |
| body2 core | 0.0469 | 0.0113 | 0.0012 | 0.0068 | 0.0071 | 0.986 |
| body2 mid | 0.0002 | 0.0002 | 0.0002 | 0.0002 | 0.0002 | 1.000 |
| body1 far | 0.0442 | 0.0371 | 0.0002 | 0.0025 | 0.0269 | 0.995 |
| **b0 core (eq_rot)** | **0.2728** | **0.1266** | **0.0602** | **0.2343** | **0.1619** | **0.559** |

**Verdict: CONFIRMS that 1e-3 is reasonable, but 1e-4 is strictly better.**
1e-3 is defensible — it is within 0.01 of truth in 5 of 6 regions. 1e-4 dominates
it on control error at negligible retention cost and should be the specified value.

### Anomalies

- **`b0 core` (equal-mass rotating) fails the control at every threshold** —
  \|a_E−1\| never falls below 0.06, and the trend is non-monotone
  (0.27 → 0.13 → 0.06 → 0.23 → 0.16). No gate setting rescues it. But `theil`
  on the same data returns 0.86–1.05. **In stressed regions the weak joint is the
  two-point estimator, not the gate.** Production uses exactly ols2.
- Retention there falls 0.65 → 0.31 as the gate tightens, and surviving
  footprints fall 12.8 → 6.4 of 16. This is the sample-bias arm made visible.
- `body2 core` carries max drift 4.11e+02 alongside median 3.04e-09 — the
  distribution is bimodal, so median drift alone is a misleading health metric.

### Machinery check (brief §4)

Drift falls ~1 order of magnitude per halving of `eta` in **all six regions**
(e.g. `b0 core`: 2.26e-02 → 1.42e-04 → 2.34e-06). Accuracy is step-size
sensitive, so the wrong-equation signature described in §4 is absent.

---

## Experiment 1 — cross-system, L≠0 and E>0

3 configurations, 25 quads, 100 probes, 0 failures. Contributors read at
`jf=0.5`, `t=13`, gate 1e-3; `alpha_E` fitted over `jf in {0.125,0.25,0.5,1.0}`.

| config | regime | L₀ | E₀ range | quads (trustworthy/total) |
|---|---|---|---|---|
| burrau | L=0, E<0 | 0.00 | −13.02 … −7.83 | 7/7 |
| eq_rot ω=0.3 | L≠0, E<0 | +0.99 … +2.96 | −1.51 … −0.57 | 9/12 |
| eq_fast ω=1.2 | L≠0, E>0 | +3.98 … +11.83 | +0.73 … +6.50 | 6/6 |

### Winner shares (trustworthy quads only)

| config | footprints | mean shape | mean event | mean KE | win shape | win event | **win KE** |
|---|---|---|---|---|---|---|---|
| burrau | 112 | 0.0137 | 0.0287 | 0.7817 | 0.00 | 0.08 | **0.92** |
| eq_rot | 144 | 0.1480 | 0.2059 | 0.3377 | 0.25 | 0.29 | **0.46** |
| eq_fast | 96 | 0.0037 | 0.0000 | 0.0114 | 0.07 | 0.00 | **0.93** |

Including untrustworthy quads: burrau 0.92, eq_rot 0.49, eq_fast 0.93.

**Verdict: FALSIFIED for L≠0, E<0. Survives for E>0.**
KE's winner-share is 0.46 at `L≠0, E<0` — below the ~50% falsification line —
with shape (0.25) and event (0.29) together taking the majority. The contributing
set is configuration-dependent and must be re-derived.

### Why I do not trust that verdict as far as it looks — read this before acting

The KE normalisation (`spread/2.0`) is flagged provisional in the brief, and
**it decides the verdict**:

| KE divisor | burrau | eq_rot | eq_fast |
|---|---|---|---|
| /0.5 | 0.96 | 0.84 | 1.00 |
| /1.0 | 0.92 | 0.69 | 1.00 |
| **/2.0 (brief)** | **0.92** | **0.49** | **0.93** |
| /4.0 | 0.90 | 0.24 | 0.55 |
| /8.0 | 0.80 | 0.12 | 0.50 |
| /20.0 | 0.46 | 0.09 | 0.42 |

Burrau is robust across a 16× range of the divisor (0.96 → 0.80). `eq_rot`
crosses the 50% line between /1.0 and /2.0. **The falsification is real at the
specified constant but is not robust to it.** Unlike shape (chord bound 2.0) and
event (attainable maximum 1−1/n), KE has no principled bound, so the `max` is
comparing a bounded quantity against an unbounded one on an arbitrary scale.
That is a defect in the criterion, not a fact about the physics.

### Per-quad scatter

| config | quads | min | median | max |
|---|---|---|---|---|
| burrau | 7 | 0.69 | 1.00 | 1.00 |
| eq_rot | 12 | 0.19 | 0.53 | 1.00 |
| eq_fast | 6 | 0.62 | 1.00 | 1.00 |

`eq_rot` KE win-share runs 0.19 to 1.00 **within one configuration**, and it is
not random — it tracks radius. `b0 r1.6` gives 1.00, `b0 r2.2` gives 0.25.
Dominance is a property of the region, not of the configuration.

### Anomalies

- **The `E>0` result is dominance over near-zero.** `eq_fast` mean KE is 0.0114
  against Burrau's 0.7817 — two orders of magnitude smaller — and event spread is
  identically 0.0000 in all six quads. Everything ionises cleanly, so every
  contributor is negligible and KE "wins" a contest between near-zeros. The 0.93
  share is arithmetically true and physically nearly vacuous: these regions are
  determinate, so the refinement question does not arise there.
- **Gate-induced sampling bias.** In the first pass all three `eq_rot` *core*
  quads failed the control and all three *outer* quads passed, so the trustworthy
  sample was biased to large radius. I added 6 intermediate-radius quads; that
  moved the answer from 0.50 to 0.46. **Reporting the first 6 quads alone would
  have given a different verdict.**
- The three untrustworthy `eq_rot` quads (a_E = 0.89, 0.93, 1.13) are precisely
  the close-encounter cores, retaining 11/16, 14/16, 11/16 footprints.

### Configuration change I made, and why

**Equal-mass-at-rest was dropped as the L=0 arm.** `R0=[[0,1],[-1,-0.5],[1,-0.5]]`
has sides 1.803/1.803/2.000 — near-equilateral — so equal masses released from
rest collapse homothetically toward a **triple collision**, which §2.2 says AZ
provably cannot regularise. Measured: drift NaN, retention 0.17–0.26, 235–354 s
per probe against ~3 s nominal. It is `deep interior`'s pathology spread over the
whole slice. Burrau is the correct L=0 baseline regardless — it is the
configuration the 69–100% claim was measured on. The L≠0 and E>0 arms are
unaffected, and both were verified in regime before use (table above).

---

## Experiment 2 — long horizon (t=13, 30, 60, 120, 240) — **SUPERSEDED, see below**

> **This section was rewritten after Brief 2.** The `tb_az.py` patch removed the non-finite
> step-budget burn that caused 17 of 40 probes to time out here. All 40 now complete (0 failures),
> plus a t=240 horizon. **The original conclusion was wrong** and is corrected below. Full detail in
> `REPORT2.md` §D.

50 probes across 5 regions x 2 quad sizes x 5 horizons, **0 failures**. Gate `1e-4`.

### Escape fraction vs horizon (parent quad)

| region | t=13 | t=30 | t=60 | t=120 | t=240 |
|---|---|---|---|---|---|
| near-field | 0.00 | 0.01 | 0.49 | 0.86 | **1.00** |
| mid-field | 1.00 | 1.00 | 1.00 | 1.00 | 1.00 |
| far r~10 | 0.00 | **1.00** | 1.00 | 1.00 | 1.00 |
| body2 mid | 0.98 | 1.00 | 1.00 | 1.00 | 1.00 |
| body1 slice | 0.00 | 0.06 | 0.50 | 0.95 | **1.00** |

**Verdict: the claim is FALSIFIED.** Every region ionises completely by t=240, and `far r~10` by
t=30. "100% censored in three of four regions" was an artefact of the 13-unit horizon. **My earlier
reading in this section — that those regions "are bound, and waiting does not help" — was wrong;
waiting helps in every one of them.** The timeouts that forced that reading were the harness bug,
not the physics.

### But the drift gate collapses before the escapes arrive

| region | retained t=13 | t=30 | t=60 | t=120 | t=240 | footprints with >=3 copies @ t=240 |
|---|---|---|---|---|---|---|
| near-field | 0.98 | 0.95 | 0.43 | 0.16 | 0.10 | **0/16** |
| mid-field | 0.98 | 0.98 | 0.77 | 0.55 | 0.34 | 8/16 |
| far r~10 | 1.00 | 0.29 | 0.19 | 0.10 | 0.09 | 2/16 |
| body2 mid | 1.00 | 0.98 | 0.77 | 0.50 | 0.34 | 11/16 |
| body1 slice | 1.00 | 0.93 | 0.38 | 0.15 | 0.12 | 1/16 |

For near-field there is no horizon where escape is measurable *and* the data survives: t=60 gives
escape 0.49 at retention 0.43; t=120 gives 0.86 at 0.16. **The binding constraint on long-horizon
rendering is the gate, not the integrator** — which sharpens the Experiment 3 finding that a
threshold calibrated at t=13 does not transfer.

### `t_end` — the exclusion rested on a measurement artefact

`t_end` was assigned at sync boundaries only (`tb_all_az.py:75`), so its resolution *was* the sync
interval, 0.4. All 128 trajectories in mid-field t=30 carried `t_end = 6.8` exactly = 17 x 0.4:
within-footprint spread zero **by construction**, exponent undefined.

Brief 2 rebuilt this in a new module (`xp_tend.py`) with the escape test decoupled from sync and
bisected inside the firing window — resolution `1.95e-04`, a factor of 2048. Result: **16/16
footprints resolved, and `alpha_t_end` = 0.9464 (mid-field) and 0.8588 (body2 mid), both with
trustworthy energy controls.** `t_end` carries usable signal and should be re-scoped, not excluded.

The escaped-only caveat is now quantified: it moves `alpha_E` by 0.0000 where escape is complete and
by 0.0537 where escape is partial (near-field t=60, escape 0.76) — just across the trust bar. **Trust
`t_end` exponents only where the escape fraction is ~1.0.**

### Anomaly — the gate biases *against* escape

Escape is powered by close encounters, where AZ works hardest, so escaped trajectories carry higher
drift (near-field t=30: median 3.6e-02 escaped vs 2.6e-07 censored). The gate removes escapers
preferentially and **under**-states escape fraction by up to 0.19 (near-field and body1 slice at
t=60). The table above is therefore conservative.

## Cross-cutting: a cost pathology worth fixing

When the AZ state overflows to non-finite, `done = s[:,8] >= dt_left` is never
satisfied (NaN comparisons are False), so `integrate_az` burns its entire
`max_steps` budget instead of bailing. Measured: 354 s against ~3 s nominal;
at t=60, 3/128 trajectories never reach `t_max` and take the probe from ~10 s to
46 s. **Results stay valid** — those trajectories carry NaN drift and the gate
excludes them — but cost blows up by up to 100×, and it is why Experiment 2's
long horizons are incomplete.

I did not change the harness. I ran every probe in a subprocess under a hard
wall-clock timeout instead (`xp_driver.py`). A one-line non-finite check in the
`integrate_az` step loop would remove the pathology, but that is your call to
make on research code.

**A related trap, not triggered here but worth recording:** the obvious
workaround — lowering `max_steps` — would corrupt results silently. Trajectories
truncated mid-interval have their playhead left behind (`t[sel] += min(s[:,8],
dt_left)`) while their energy stays conserved, so **the drift gate would not
catch them** and they would be compared at the wrong physical time.
