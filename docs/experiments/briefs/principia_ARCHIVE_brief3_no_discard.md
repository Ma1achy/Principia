> # ARCHIVED — COMPLETE — validated, and the rule has since been extended
>
> The no-discard architecture was validated and is now a standing commitment
> (`principia_00_philosophy.md` §4.3).
>
> **Extended since, in ways worth knowing:** a `filter(is_finite)` in a *reduction* is a silent discard
> one layer up — 26 sites classified, three classes, only one a bug. And a quad where **nothing
> integrated** reports `ensemble_spread` **5.6× smaller** than a healthy one, reading as *better
> resolved*, with `error_ratio_max` at exactly its converged 1.0000. Hence
> `footprint_undetermined` and `Decision::Undetermined`.
>
> **Evidence:** `prin-rs` — `FINDINGS.md`, `README.md`, `results/`.

# Brief 3: validate the no-discard architecture

**Prerequisite:** Briefs 1 and 2, and your `REPORT.md` / `REPORT2.md`. This one **overturns a
convention both of those relied on** — the drift gate — so read §1 before reusing any pipeline code.

**Deliverable:** whether the corrected design works end-to-end, and if not, where it breaks. This is
the last check before the design goes into a specification. **A clean negative is a good outcome.**

---

## 1. What changed, and why

Briefs 1–2 both applied a **drift gate**: exclude trajectories with `|dE/E| >` threshold before
computing any spread, requiring ≥3 survivors per footprint. You calibrated it to `1e-4`.

**That whole approach is now judged wrong.** Not the threshold — the act of discarding. Five reasons,
three of them measured in your own reports:

1. **It is chaos-selective.** High drift is genuinely integration error (energy is conserved, so if it
   moves the integrator failed) — but integration difficulty is *caused by close encounters*, and
   close encounters are what chaos is made of. **Excluding by drift excludes by chaoticity**, on an
   instrument built to measure chaos. You measured this: escape understated by up to 0.19 because
   escapers carry higher drift.
2. **It turns a constant into a variable.** `event` is normalised by its attainable maximum
   `1 − 1/(E+1)`. If `E` varies per pixel because copies were removed, that "constant" is a per-pixel
   free parameter — **the same defect you identified in `dom_KE`.**
3. **Image quality decays with the playhead.** Failures are absorbing, so the effective copy count
   falls monotonically as `t` advances. Unacceptable in a lockstep instrument where the playhead is
   scrubbed.
4. **It violates a stated sampling principle**: *"a pixel is a pixel; there is genuinely nothing
   special about any sample."*
5. **It produces confident-looking answers from biased subsets.** At `t=80`, near-field, the gated
   pipeline reports `alpha_E = 0.8607` computed on 44% of the data. That reads as an answer.

**And there is no "failed" category to classify on either.** Calling the threshold a *classifier*
rather than a *filter* just relocates the arbitrary constant. Numerical error here is a continuum:
non-finite values are 0–4% while the finite drift tail spans **ten orders of magnitude** (median
1.6e-07, p99 2.9e+03 at `t=40`).

**The physical point underneath:** in a chaotic system a slightly-inaccurate trajectory *is* the exact
trajectory of a slightly-different initial condition — which is precisely what the ensemble is already
sampling. Below some scale, integration error is not contamination; it is absorbed into the signal.
Measured at `t=13`: `sigma_E(t)/sigma_E(0) = 1.0000` in two regions.

### The replacement

**Nothing is ever discarded. Every footprint always carries exactly `E+1` copies.**

Trust is *measured* rather than enforced, using conserved quantities:

```
error_ratio    = sigma_E(t)  / sigma_E(0)      -- exactly 1.0 under exact dynamics
error_ratio_lz = sigma_Lz(t) / sigma_Lz(0)     -- same construction, independent
```

Each trajectory conserves its own `E` and `L_z` exactly, so the ensemble's *spread* of them is fixed
at `t=0` and must stay constant forever. **Any departure from 1.0 is accumulated integration error,
with no threshold and no tuned constant.** These are independent: symplectic integrators conserve
angular momentum exactly but energy only approximately.

Note the symmetry: the property that *disqualifies* conserved quantities as spread contributors (no
dynamics — their spread only reports that ICs differ) is exactly what *qualifies* them as error
meters.

**Contamination is tolerated deliberately.** Bad values enter the reduction and nothing removes them.
This is defensible because the exponent is used **ordinally** (it ranks quads, it does not forecast
`2^-alpha`), so contamination must be systematically *rank*-changing to matter — and because the
failure direction is **conservative**: contamination inflates spread, pushing toward *refine*, which
wastes budget rather than losing structure. **The design does not need to be clean, it needs to be
justified and safe when wrong.**

---

## 2. Experiments

Use `eta = 0.01`. **Apply no gate anywhere.** Report `error_ratio` and `error_ratio_lz` with every
measurement. Save raw JSON incrementally. Skip `deep interior`.

### Experiment A — does the closed loop survive with no discarding? **(the blocking one)**

**Claim under test:** *with no gate, uniform `E+1` copies, and a robust dispersion measure, the
criterion still works: the energy control recovers `alpha_E ≈ 1.0` and the exponent still
discriminates between regions.*

**Method.** Repeat the Brief-1 closed loop — parent (`half=0.05`) vs child (`half=0.025`), exponent
from the realised spread ratio — but with **no trajectory ever excluded**. Compare **three dispersion
measures** over the full uniform sample:

| measure | note |
|---|---|
| `std` | outlier-sensitive; the baseline that failed ungated in Brief 1 |
| `MAD` (`1.4826 × median|x − median|`) | robust to a minority; **known to fail past ~50% corruption** |
| trimmed std (drop top/bottom 10% **by value**, not by drift) | keeps the sample uniform in *rule* even though it drops points per-footprint — a middle option |

For ≥4 regions and ≥2 horizons (`t = 13` and `t = 40`), record: `alpha_E` per measure,
`alpha` for `shape` and `event` per measure, `error_ratio`, `error_ratio_lz`.

**Confirms if:** some measure gives `|alpha_E − 1| < 0.05` at `t=13` **and** preserves the region
ranking for `shape`/`event` that the gated Brief-1/2 runs produced.

**Falsified if:** no measure recovers the control without discarding. **That is a real possible
outcome** — say so plainly. It would mean either the gate must return with an explicit honesty cost,
or the reduction needs a different formulation entirely.

### Experiment B — are the two error meters actually independent, and do they track error?

**Claim under test:** *`error_ratio` and `error_ratio_lz` are parameter-free trust measures, and they
catch different failures.*

**Method.** Across regions and horizons (`t = 13, 40, 80`) and ≥2 tolerances (`eta = 0.01, 0.005`),
record both ratios per footprint. Test:

- **Do they track integration quality?** Both should approach 1.0 as `eta` tightens, and depart as `t`
  grows. Report the correlation between each and the mean per-trajectory drift.
- **Are they independent?** Report their correlation with each other. If they are ~identical, one is
  redundant and should be dropped. **If they disagree, report which failures each catches** —
  angular momentum should be the more sensitive of the two on a symplectic scheme, but AZ+RK4 is
  **not** symplectic, so this may not hold. That would itself be worth knowing.
- **Is `Gamma` a third meter?** The regularised Hamiltonian is autonomous in `tau`, so it is conserved
  on the AZ path. If it is cheap to extract, report it alongside. Optional.

### Experiment C — canonical-unit length scales **(cheap; independent of A and B)**

**Claim under test:** *a length constant expressed in canonical units is gauge-covariant, so an
optional softening `epsilon` and a collision radius `r_coll` are admissible where a fixed absolute
length is not.*

Brief-1 context: softening was removed because a fixed `eps` broke the scale gauge (1.66× under a
pure rescaling). But the same test showed that **scaling `eps` with the configuration preserved the
gauge to five decimals across a 16× size range**. Principia canonicalises `sqrt(I) = 1`, so a constant
in canonical units rescales along with the configuration automatically.

**Method.** Rescale the configuration (`r → alpha·r`, `v → alpha^-0.5·v`, `t → alpha^1.5·t`) for
`alpha ∈ {0.25, 1, 4}`, with `eps` expressed as a fraction of the canonical scale (i.e. `eps` scaled
by `alpha`), and confirm scale-quotiented measurements are invariant. Then repeat with `eps`
**fixed at `t=0` in canonical units** but the configuration allowed to evolve — the intended
production semantics — and confirm the gauge still holds.

**Also test the constraint that forces the design:** a **co-moving** `eps` (scaled by the
*instantaneous* `I` rather than the initial one) should make the Hamiltonian time-dependent and
**destroy energy conservation**. Confirm `error_ratio` departs from 1.0 in that case while the fixed
version holds. If confirmed, that is the reason co-moving is prohibited, and it deserves a number.

---

## 3. Reporting

As before: raw-number tables, explicit verdicts, anomalies, raw JSON.

Three things specifically:

1. **A recommended dispersion measure**, or a statement that none works without discarding.
2. **Whether both error meters are needed**, or one suffices.
3. **Any invariant I have missed.** You have now spent more time inside this system than I have. The
   design turns on conserved quantities being free correctness checks — `E`, `L_z`, and `Gamma` on
   the AZ path. **If there are others worth carrying, or if one of these is unsound as a meter, say
   so.** That is more valuable than the tables.

Your last two reports were better for the objections than the numbers — the `dom_KE` normalisation
defect and the Theil–Sen degeneracy both changed the design. Same instinct here.
