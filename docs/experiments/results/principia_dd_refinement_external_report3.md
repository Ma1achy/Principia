# Brief 3: validating the no-discard architecture

**Run:** 96 no-gate probes (15360 trajectories, 33 non-finite = 0.21%) + 24 gauge runs, 42 min.
Every footprint retained all `E+1` copies in all 96 probes — nothing was discarded anywhere. Raw JSON in `xp_results3/`.
`eta = 0.01` unless stated, **no gate anywhere**, `deep interior` skipped.
Quads are 4x4 = 16 samples; copies per footprint `E+1 = 8` (`ens=7`) and `16` (`ens=15`).
New files only: `xp_nogate.py`, `xp_scale.py`, `xpC_{closed,meters,scale,roundtrip}*.py`.

**Headline: the architecture is sound in principle and works at `t=13`, but it does not
survive to `t=40`, and the two proposed meters have a structural gap. Details below — this is
a partial negative and I am reporting it as one.**

---

## Experiment A — does the closed loop survive with nothing discarded?

Parent `half=0.05` vs child `half=0.025`, exponent from the realised spread ratio. 48 probes:
6 regions x 2 horizons x 2 copy counts x 2 quad sizes.

### A1. Pass rate against the trust bar `|alpha_E − 1| <= 0.05`

| measure | t=13 | t=40 | ens=7 | ens=15 | overall |
|---|---|---|---|---|---|
| `std` | 8/12 | 4/12 | 6/12 | 6/12 | 12/24 |
| **`MAD`** | **10/12** | 5/12 | 7/12 | 8/12 | **15/24** |
| `trim` | 10/12 | 4/12 | 7/12 | 7/12 | 14/24 |

Copy count barely matters (ens=7 → 7/12, ens=15 → 8/12 for MAD). **Horizon dominates
everything**: MAD goes 10/12 → 5/12 between t=13 and t=40.

### A2. Ranking preservation — same probes, gated vs ungated-MAD reduction

| horizon | field | Spearman(gated, ungated-MAD) | n |
|---|---|---|---|
| t=13 | **shape** | **+0.943** | 6 |
| t=13 | event | undefined (only 2 defined values) | 2 |
| t=13 | E | −0.371 | 6 |
| t=40 | shape | +0.657 | 6 |
| t=40 | event | +0.300 | 5 |

Side by side at t=13 (`alpha_shape`): ungated-MAD is systematically *lower* than gated but ranks
almost identically — 2.303→1.932, 1.943→1.407, 1.082→0.888, 1.006→1.009, 0.125→−0.076,
2.039→1.876. The ordering is what the scheduler uses, and it survives.

**Verdict: CONFIRMED at t=13, FALSIFIED at t=40.**
At t=13, MAD over the full uniform sample recovers `alpha_E` in 10/12 cases and preserves the
`shape` ranking at Spearman +0.943. The two failures are both `eq_rot b0 core`, which the error
meter flags loudly (see A3). At t=40 no measure recovers the control — best is MAD at 5/12 — so
the loop as specified does not survive to the longer horizon.

### A3. The meter must aggregate the way the reduction does

The reduction takes a **mean** over footprints; a median-aggregated meter cannot see a single bad
footprint move that mean. Scoring "flag if ratio > 1.05" against "is `alpha_E` actually wrong":

| meter aggregation | caught | missed | false alarms |
|---|---|---|---|
| median over footprints | 6 | **3** | 1 |
| **max over footprints** | **9** | **0** | 3 |

**Use the max.** With median aggregation the meter misses three real failures — all at t=40, all
reading `error_ratio` ≈ 1.00–1.04 while `alpha_E` is 0.883, 0.902, 1.534.

Conditional pass rate, max-aggregated:

| error_ratio (worse of parent/child) | n | MAD `alpha_E` passes |
|---|---|---|
| < 1.05 | 17 | 14/17 |
| 1.05 – 2 | 6 | 1/6 |
| > 2 | 1 | 0/1 |

**Anomaly — the max is a flag, not a measurement.** Its magnitude is unstable: tightening `eta`
from 0.01 to 0.005 makes it *worse* in several cases (`body2 core` t=40: 1561 → 1.80e+04;
`near-field` t=80: 2742 → 2.88e+04) while median drift improves by one to two orders of magnitude.
The max over 16 footprints of a heavy-tailed quantity is not a stable statistic. Treat
`> 1.05` as a boolean "distrust this quad" and do not read the number.

---

## Experiment B — are the two meters independent, and do they track error?

36 probes, 6 regions x `t ∈ {13,40,80}` x `eta ∈ {0.01,0.005}`.

### B1. The Lz meter does not exist in the regime the project was built on

| config | probes | Lz meter defined | why |
|---|---|---|---|
| **burrau** | 18 | **0/18** | released from rest: `v=0` for every copy, so `Lz ≡ 0` and `sigma_Lz(0) = 0` — the ratio is 0/0 |
| eq_rot | 12 | 12/12 | rotating: copies differ in Lz |
| eq_fast | 6 | 6/6 | rotating: copies differ in Lz |

**This is structural, not incidental.** The ratio formulation requires the conserved quantity to
*vary across the ensemble* at `t=0`. Jittering a body's position always changes its potential
energy, so `sigma_E(0) > 0` always. It does not change `Lz` at all when the system starts from
rest. **Every from-rest configuration — the whole Burrau baseline of Briefs 1–2 — has no Lz meter.**

### B2. Where both exist, they are largely redundant

| pair | correlation (log–log) | n |
|---|---|---|
| error_ratio_max vs median drift | +0.774 | 36 |
| Lz_max vs median drift | +0.879 | 18 |
| **error_ratio_max vs Lz_max** | **+0.910** | 18 |
| round-trip vs median drift | +0.186 | 12 |
| round-trip vs error_ratio_max | +0.617 | 12 |

Both track integration error. They correlate +0.910 with each other — Lz is slightly the better
drift proxy where it exists, but it is not independent information.

**Verdict: one meter suffices, and it must be the energy ratio.** `error_ratio` is always defined;
`error_ratio_lz` is unavailable in the entire from-rest regime and adds +0.910-correlated
information where it is available. Carry Lz opportunistically if free, but nothing may depend on it.

### B3. `Gamma` is not a third meter

`Gamma = A·B·(H − E)` by construction. Measured on a batch:

- `max |Gamma − A·B·(H−E)|` = **1.27e-13**
- `corr(Gamma, A·B × energy error)` = **+1.0000000000**

Recovered from the Cartesian state it is the energy error multiplied by a per-trajectory factor
`A·B`. **It carries nothing the energy meter does not already carry.** It would only be independent
if accumulated in the regularised variables through the integration, which the harness does not do.

---

## Experiment C — canonical-unit length scales

Softened leapfrog (AZ carries no softening), `c_eps = 0.05`, `dt = 1e-4`, `t = 5`,
`alpha ∈ {0.25, 1, 4}` — a 16x size range. Every comparison is rescaled-vs-not within one
integrator, so the never-compare-across-integrators rule holds; the result is about the gauge
argument, not the production integrator.

### C1. Gauge invariance of scale-quotiented measurements (mid-field)

| mode | alpha=0.25 | alpha=1 | alpha=4 | gauge held? |
|---|---|---|---|---|
| **absolute** `E·alpha` | −10.2092216334 | −10.3733803581 | −10.3839913216 | **NO — 2nd decimal** |
| **absolute** `n̂₀` | −0.0070621252 | +0.0238637727 | −0.0401862718 | **NO — sign flips** |
| **initial** `E·alpha` | −10.3733715961 | −10.3733715961 | −10.3733715961 | **yes — 10 d.p.** |
| **initial** `n̂₀` | +0.0238825780 | +0.0238825780 | +0.0238825780 | **yes — 10 d.p.** |
| **comoving** `E·alpha` | −10.6909880452 | −10.6909880452 | −10.6909880452 | yes (but see C2) |

**Confirmed, and better than the ~5 decimals expected: `initial` holds the gauge to 10 decimal
places across a 16x rescaling. A fixed absolute length destroys it** — `n̂₀` does not merely shift,
it changes sign, so the measured shape is a different point on the shape sphere.

### C2. Why co-moving is prohibited — and it is not an accuracy problem

| mode | `dt = 1e-4` | `dt = 2e-5` | improves with step size? |
|---|---|---|---|
| absolute | 1.184e-09 | 4.737e-11 | yes (25x) |
| **initial** | 1.184e-09 | 4.735e-11 | **yes (25x)** |
| **comoving** | **3.062e-02** | **3.062e-02** | **NO — identical** |

`error_ratio` under co-moving: **1.227083** at `dt=1e-4` and **1.227084** at `dt=2e-5`.

Co-moving `eps` is **seven orders of magnitude** worse than `initial`, and **completely insensitive
to step size**. That is the §4 wrong-equation signature from Brief 1: accuracy that does not improve
when you integrate more carefully is not an integration error. Rescaling `eps` by the instantaneous
`I(t)` makes the Hamiltonian explicitly time-dependent, so energy is genuinely not conserved.
`error_ratio` reports it directly (1.227 against 1.000). **That is the number justifying the
prohibition.**

Note the two properties are independent: co-moving *is* gauge-covariant (identical across `alpha`)
and still fails, because covariance and conservation are different requirements.

---

## The three questions

### 1. Recommended dispersion measure: **MAD — with the horizon caveat stated up front**

MAD over the full uniform sample, non-finite values mapped to `+inf`. It recovers the control in
10/12 quads at `t=13` and preserves the `shape` ranking at Spearman +0.943 — the loop closes.
It beats `std` (8/12) and matches `trim` while needing no trim fraction, which is one fewer free
constant — and given this project's history with free constants, that matters.

But **no measure survives `t=40`** (MAD 5/12, trim 4/12, std 4/12). The honest statement is: the
no-discard reduction works at short playhead and degrades with horizon, in the same way and for the
same reason the gate did in Brief 1. Whatever replaces the gate does not remove the horizon problem;
it only stops the sample shrinking.

**Non-finite values are the one place the architecture cannot be literally satisfied.** A NaN has no
rank, so no dispersion measure can absorb it — `np.median` of anything containing NaN is NaN. Mapping
to `+inf` preserves the `E+1` count and keeps the rule uniform, and MAD and trimmed-std both survive
a minority of them (observed worst case 4.7% at t=240). `std` returns NaN, which is the honest answer
for `std`. This is a real limit, not a detail: **"never discard" holds for the reduction but not for
the number line.**

### 2. Both meters needed? **No. Energy only.**

`error_ratio_lz` is **undefined in 18/18 Burrau probes** because from-rest ensembles have
`sigma_Lz(0) = 0` exactly. Where it is defined it correlates +0.910 with the energy meter. It is
neither generally available nor independent. Use the energy ratio, **max-aggregated over footprints**,
as a boolean flag at `> 1.05`; carry Lz only opportunistically.

### 3. Invariants — one you have is redundant, one is being destroyed, and one you are missing

**`E` and `Lz` exhaust the non-trivial first integrals** of the planar three-body problem in the COM
frame. The remainder are total linear momentum and COM position — and `tb_az.AZSystem.to_cartesian`
**re-centres both to zero every sync**. Measured after integration: `|COM|` max 1.48e-16,
`|P_total|` max 8.88e-16. They are structurally masked, not conserved: they *cannot* drift, so they
*cannot* report. **That is a free correctness check the architecture currently destroys**, and it
could be recovered by evaluating them before the re-centring line rather than after.

I found this the hard way: my first round-trip implementation differenced a COM-centred return
against a lab-frame start and read a constant 0.627 at every horizon from `t=0.5` to `t=80`, with
drift at 1e-15. The re-centring is invisible until something depends on it.

**`Gamma` is not a third meter** — `corr = +1.0000000000` with `A·B ×` energy error (B3).

**The one I would add: time-reversal round-trip error.** Integrate to `t`, reverse the velocities,
integrate back, measure `||r_return − r_0||` in canonical units, both sides COM-centred. Not a first
integral, but parameter-free — and it sees what the conserved quantities structurally cannot:

| config | region | t | error_ratio MAX | median drift | **round-trip** |
|---|---|---|---|---|---|
| eq_rot | b0 outer | 13 | 1.000000 | 2.08e-11 | **3.17e-03** |
| eq_fast | b0 core | 13 | 1.000000 | 1.42e-12 | **4.71e-02** |
| eq_rot | b0 outer | 40 | 1.000000 | 5.19e-11 | **5.80e-03** |
| eq_fast | b0 core | 40 | 1.000000 | 1.32e-12 | **1.05e-02** |

Every conserved quantity reads *exactly* 1.000000 with drift at machine precision, while the
trajectory has moved 0.3%–4.7% of a system radius from where it should be. **Phase error along the
trajectory conserves `E` and `Lz` exactly and is therefore invisible to both proposed meters.**
`corr(round-trip, drift) = +0.186` — it is genuinely independent information.

### And the reason that matters — the absorption argument is not certified by the meters

The architecture's justification for tolerating contamination is that a slightly-inaccurate
trajectory is the exact trajectory of a slightly-different IC, which the ensemble already samples.
That holds only while the integration displacement stays **below the deliberate jitter it is
supposed to hide inside**. The round-trip measures exactly that displacement, so the claim is
directly testable:

| config | region | t | jitter (canonical) | round-trip | ratio | absorbed? |
|---|---|---|---|---|---|---|
| burrau | near-field | 13 | 7.45e-03 | 9.44e-06 | 0.00 | yes |
| burrau | mid-field | 13 | 4.98e-03 | 8.57e-05 | 0.02 | yes |
| burrau | body2 core | 13 | 7.45e-03 | 2.57e-05 | 0.00 | yes |
| **eq_fast** | **b0 core** | **13** | **1.54e-02** | **4.71e-02** | **3.05** | **NO** |
| eq_rot | b0 core | 13 | 1.54e-02 | 1.67e-01 | 10.81 | NO |
| eq_rot | b0 outer | 13 | 9.05e-03 | 3.17e-03 | 0.35 | yes |
| burrau | (all three) | 40 | — | — | 0.13–0.33 | yes |
| eq_fast | b0 core | 40 | 1.54e-02 | 1.05e-02 | 0.68 | yes |
| eq_rot | b0 core | 40 | 1.54e-02 | 2.49e+00 | 161.2 | NO |
| eq_rot | b0 outer | 40 | 9.05e-03 | 5.80e-03 | 0.64 | yes |

**Absorption holds in 9 of 12 cases, and in Burrau it holds with two orders of margin.** The
argument is sound where it matters most. But `eq_fast b0 core` at t=13 violates it by 3x **while
`error_ratio_max` reads exactly 1.000000 and drift is 1.4e-12** — so the meters certify a quad
whose integration error exceeds the perturbation the ensemble is deliberately applying. The two
`eq_rot b0 core` violations are caught (error_ratio 1.8e+04), so the gap is narrow, but it is real.

**Caveat on the round-trip, stated because it weakens the above.** It is an *upper bound* on
trajectory error, not a clean integrator-quality meter: it also contains chaotic amplification of
roundoff and AZ's reference-body switching, which is state-dependent and need not choose the same
reference on the reverse path. Its non-monotonicity for `eq_fast` (4.71e-02 at t=13 but 1.05e-02 at
t=40) points at exactly that. Read the "NO" rows as *not demonstrated to be absorbed* rather than
proven violation.

---

## Anomalies and negative results

- **No dispersion measure survives `t=40`** (best 5/12). The horizon problem is not a property of
  the gate; removing the gate does not remove it.
- **The meter, as specified, fails on the footprints it exists to flag.** A std-based
  `sigma_E(t)/sigma_E(0)` returns NaN as soon as one copy is non-finite — precisely the pathological
  footprint. Under exact dynamics the two *sets* are identical, so any statistic gives exactly 1.0
  and the calibration does not depend on the choice: **use MAD inside the meter too.**
- **Meter magnitude is unstable under `eta`** — tightening the tolerance made `error_ratio_max`
  worse in 3 cases while median drift improved 10–100x. Flag, don't measure.
- **`trim10` remains a no-op at 8 copies** (`floor(8 × 0.1) = 0`); 12.5% was used at ens=7. Raising
  to 16 copies changed almost nothing (MAD 7/12 → 8/12), so **the copy count is not the lever** —
  worth knowing before paying 2x for it.
- **The `event` exponent is frequently undefined** — `nan` in 4 of 6 regions at t=13, because event
  disagreement is identically zero in both parent and child. It contributes nothing at short horizon.
- **`alpha_E` ranking is not preserved** (Spearman −0.371 at t=13) even though `shape` ranking is
  (+0.943). That is expected — `alpha_E` should be 1.0 everywhere, so its ranking is noise ranking —
  but it means the control must be read as a pass/fail bar, never as an ordering.
