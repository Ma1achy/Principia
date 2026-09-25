> # ARCHIVED — SUPERSEDED — ITS PREMISE IS VOID
>
> This briefs auditing every signal **against DP labels scored by `error(B)`**. That metric is now known
> to be **void**: it scored OKLab distance under an auto-ranged colouring, so a smooth region's 1e-8
> residual counted as error at every depth and **breadth-first came out near-optimal by construction.**
>
> So the audit's ranking machinery measured the colour map. **Any re-run must score payload-space** —
> `shape_vec` and event class against a fixed `eps` — per `principia_dd_refinement_policy.md` §4.
>
> **What survives:** §2's argument that Spearman separates *"does not know"* from *"knows but is drowned
> out"*; §1's account of how diffusion was excluded on a **scale artefact** rather than an information
> result; and the standing rule against pre-filtering the signal list.
>
> **Evidence:** `prin-rs` — `FINDINGS.md`, `README.md`, `results/`.

# Brief: audit every signal against the DP labels

**Prerequisite:** PR #22 (`Rank::Uniform`, `Cache::dp_optimal`). This brief exists because the DP
gives, for the first time, a **ground-truth label per quad** — and every previous exclusion was
decided without one.

**Deliverable:** a ranked table of how much each available quantity knows about what is actually
worth splitting, plus a test of whether combining them beats the best single one.

---

## 1. Why the earlier exclusions do not stand

`principia_dd_refinement_criterion.md` §7.13 excluded **diffusion** because *"it wins the `max` in
6% of footprints in one region and 0% elsewhere"*. That is a statement about **scale**, not about
**information**. `ensemble_spread = max(...)`, so a field that lives an order of magnitude lower
can carry real signal and never win. **That is a normalisation artefact recorded as an information
result.**

Three further reasons those measurements do not transfer:

- **Measured with KE present**, which won 69–100% of footprints and was later *dropped*. Diffusion
  was competing against a field that no longer exists.
- **Measured on softened physics**, before AZ and before the Levi-Civita branch-cut fix.
- **Measured at n = 64**, the sample size that flipped three separate conclusions elsewhere in this
  project.

**And decisively: diffusion and FTLE have never been scored by `error(B)` at all.** The `Rank` list
is within/mean/p90, between, max_of_both, frac_hot ×2, layout, term_grad, running_max, first_div,
contrast. Neither appears. **They were excluded by a weak test and never given the good one.**

---

## 2. The measurement — Spearman against the DP's decision

The DP knows, for every quad and every budget, whether the **exact optimum** splits it. That is a
label, and it is free.

For each candidate signal `s` and each region, at several budgets:

```
label(k)  = 1 if the DP optimum splits quad k at this budget, else 0
score(s)  = Spearman( s(k), label(k) )   over all quads in the frontier
```

**Why Spearman and not `error(B)`:** it is **scale-free**. A signal that correlates strongly with
the optimum but never wins a `max` is being **wasted by the aggregation**, not uninformative —
which is precisely the failure that excluded diffusion. Rank correlation separates *"does not know"*
from *"knows but is drowned out"*, and nothing in the corpus currently can.

**Report `error(B)` too**, for every signal, as the operational number. The two together say whether
a signal is worth *rescaling* (high Spearman, poor `error(B)`) or worth *dropping* (low both).

---

## 3. Audit everything, not a shortlist

Every scalar on `QuadReduction`, plus every derived quantity already computed. **Do not pre-filter
on a hunch** — the point is that the previous filter was wrong.

| group | signals |
|---|---|
| spread arms | `spread_mean`, `spread_median`, `spread_p90`, `spread_shape_median`, `spread_event_median` |
| between arms | `between_shape`, `between_event`, `between_spread`, `between_matched`, `within_pooled` |
| **never scored** | **`diffusion`**, **`ftle`** — add them |
| layout, abs mask | `n_hot`, `n_components`, `largest_component`, `perimeter_ratio`, `frac_above_tau` (within + between) |
| layout, rel mask | the `layout_rel_*` twins, `grad_rms_within`, `grad_rms_between` |
| temporal | `running_max_divergence`, `divergence_trend`, `first_divergence_median`, `frac_diverged` |
| outcome | `terminated_fraction`, `escape_fraction`, `t_end_gradient` |
| trust | `error_ratio_max`, `worst_energy_drift`, `n_nonfinite` |
| cost | `total_substeps` |
| structural | `level`, `cell_width`, `alpha`, `alpha_sibling_spread` |
| derived | `contrast(s)` for each of the above, and each divided by `total_substeps` |

**Also audit the aggregation, not just the signal.** Each footprint-level quantity has mean / median
/ p90 / frac-above-quantile forms, and §7.13's whole diffusion verdict turned on a `max`. Report the
best aggregation per signal and **say which**, because that is a second axis that has never been
swept.

---

## 4. Then test whether MORE INFORMATION HELPS

This is the question that motivated the audit and it is answerable from the same labels.

**Fit a logistic regression** on the standardised signals against the DP label — the cheap version
of the learned-criterion idea, no new integration, runs entirely on the cache.

- **If the combination beats the best single signal on `error(B)`**, the criterion is
  **information-limited** and a learned version has room. Report the fitted weights: they say
  *which* signals carry the complementary information, which is more useful than the score.
- **If it does not**, the signals are **redundant** and no amount of extra fields helps. That is a
  clean negative and it closes the question rather than leaving it as an intuition.

**Report the coefficient of multiple correlation between signals** as well. If everything is
collinear — which is plausible, since most are functions of the same footprint spreads — that
explains a null before it is mistaken for "the criterion cannot be improved".

**Guard, and it matters here:** fit on some regions, score on the held-out one. A logistic fit on
21,845 quads with ~30 features will fit near-perfectly in-sample and tell you nothing.

---

## 5. Two things to check while the labels are out

**Does the DP's decision depend on budget in a way a static signal cannot capture?** If the optimal
label for a given quad flips between `B = 767` and `B = 6143`, then **no budget-independent
signal can be optimal at both**, and the per-level `captured` result (−9.38 at level 2, +0.53 at
level 6) is a symptom of that rather than of a bad signal. Report label stability across budgets
before concluding anything about signal quality.

**And the cost axis.** Every `error(B)` in the corpus has **B in quads**, but quads vary **100× in
`total_substeps`** (p1 2.04e4, p50 4.09e4, p99 2.05e6, bimodal). Re-plot with **B in substeps**.
A cost-aware criterion can lose on error-per-quad and win decisively on error-per-second, and
**nothing has ever measured the second** — `greedy_oracle/cost` ranks by gain-per-cost but is still
plotted against a quad budget, i.e. scored in the wrong units.

That one is cheap, uses data already in the cache, and could change the standing conclusion outright.

---

## 6. What would change the standing conclusion

PR #22's honest reading is *"no criterion reliably clears breadth-first"*. Three ways that could be
wrong, in order of how cheaply they are tested:

1. **Wrong axis** — adaptive wins on cost, not on quad count (§5).
2. **Wrong normalisation** — a signal knows, but is drowned out by the `max` (§2).
3. **Wrong signal** — a combination beats every single one (§4).

**If all three come back negative, that is a real and publishable result**: on this manifold,
adaptive refinement does not beat uniform sampling at equal cost, and the honest instrument is
uniform-plus-budget. That is a better outcome than a criterion that looks clever and is not.

---

## 7. Cautions

**Do not pre-filter the signal list.** The last filter excluded a signal on a scale artefact and it
has taken three months to notice.

**Report negative results.** A signal with Spearman ≈ 0 against the optimum is a genuine finding and
should be stated, not omitted for tidiness.

**Held-out regions for anything fitted.** In-sample fits on 21,845 quads prove nothing.

**And do not change the default on this run.** The audit says which signals carry information; it
does not by itself say what the criterion should be. That is a separate decision with the sweep
behind it.
