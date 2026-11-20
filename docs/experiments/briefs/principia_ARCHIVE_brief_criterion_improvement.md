> # ARCHIVED — SUPERSEDED
>
> Briefs improving the alpha-exponent criterion and the `error(B)` metric. Both replaced.
>
> **Superseded by:** `principia_dd_refinement_policy.md`. The criterion is `Policy::Tolerance`; the
> metric is payload-space.
>
> **Evidence:** `prin-rs` — `FINDINGS.md`, `README.md`, `results/`.

# Build brief: improving the refinement criterion

**Deliverable:** a corrected criterion, a metric to judge criteria by, and five candidate signals
measured against it. Plus the production colour scheme, the specced-but-missing temporal
accumulators, and slippy-map panning.

**Read §1 first.** It is a design error in the criterion itself, it very likely explains
`deep interior`, and it changes what everything downstream is measuring.

---

## 1. The criterion measures the wrong disagreement

### 1.1 What is implemented

```
spread_shape = mean distance of the COPIES' shape_vec from their centroid, / 2
spread_event = fraction of COPIES not sharing the modal (state,detail), / (1 - 1/(E+1))
ensemble_spread = max(spread_shape, spread_event)
```

Both are statistics over the `E+1` **copies of one footprint**. `QuadReduction` then aggregates
those `N²` per-footprint numbers (mean / median / p90).

**So the quad's number is an aggregate of WITHIN-footprint disagreement. Between-footprint
disagreement is never computed anywhere.**

### 1.2 Why that is backwards

From the design record (`principia_dd_refinement_criterion.md` §1.3), the governing observation:

> *"Sampling more finely only helps when the variation is **between** points. Variation **within** a
> point is irreducible by definition — the ICs there are identical up to perturbation, so no
> resolution resolves it."*

Splitting a quad buys **more footprints** — finer *spatial* sampling. What that can reveal is
structure **between** footprints. Within-footprint spread answers a different question: *is this
pixel's own value well-determined?*

Two distinct questions, currently conflated:

| question | measured by | what it tells you |
|---|---|---|
| Is each pixel trustworthy? | **within**-footprint spread (implemented) | is the image honest here |
| Would finer sampling reveal more? | **between**-footprint variation (**absent**) | should this quad split |

### 1.3 Why this explains `deep interior`

At a clean basin boundary, each footprint is individually **well-determined** — its copies agree,
because they all land the same side of the boundary. So within-footprint spread is **low**. But
adjacent footprints **disagree with each other**, because the boundary runs between them.

**Median of within-footprint spreads sees nothing.** That is a better explanation than
"median is blind to a thin filament", and it predicts the observed behaviour more precisely: the
fraction of footprints actually straddling a boundary **shrinks as you refine** (the boundary is 1-D,
footprints tile 2-D), so within-footprint spread *falls* with refinement even where structure
remains. Between-footprint disagreement stays high as long as the quad contains the boundary at all.

It also explains why mean and p90 descend where median does not: they are more sensitive to the few
straddling footprints, so they partially recover a signal the statistic was never designed to carry.

### 1.4 What to add

```
between_shape = spherical dispersion of the N^2 footprints' NOMINAL shape_vec (copy 0)
              = 1 - |mean resultant|,  or mean distance from centroid / 2 for consistency
between_event = fraction of the N^2 NOMINAL footprints not sharing the modal (state,detail)
              / (1 - 1/N^2)
```

**Use copy 0 only** — it is the un-jittered nominal, so between-footprint variation is not
contaminated by within-footprint jitter. Both bounded by achievable maxima, as required.

**Do not replace `ensemble_spread`.** Both quantities are wanted and they answer different
questions: within-footprint spread is the **trust / display-honesty** signal, between-footprint is
the **refinement** signal. Carry both; report both; and report their correlation, because if it is
high the distinction is academic and if it is low the criterion has been reading the wrong one.

**This is the highest-priority item in this brief.** Everything in §3 is a refinement of a signal;
this is a correction to which signal.

---

## 2. A metric to judge criteria by — build this second, before any candidate

Nothing so far measures whether a criterion is *good*. Leaf counts, depths and α distributions
describe what a criterion did, not whether it was right.

**And the screen floor changed the question.** It stops 61.2% of near-field's leaves, so the
criterion is not primarily deciding *when to stop* — it is deciding **which quads get the budget
first, within what is displayable.** That is a **ranking** problem, and ranking tolerates noise that
would wreck a threshold.

### 2.1 Definition: image change per quad spent

```
1. Render the region at a high uniform resolution -> REFERENCE image (ground truth)
2. Run the scheduler under a budget B, render adaptively -> IMAGE(B)
3. error(B) = mean per-pixel distance between IMAGE(B) and REFERENCE, in OKLab
4. Sweep B. A criterion is better if error(B) falls faster.
```

**Report the whole curve, not one number.** A criterion can win at small budget and lose at large.

**Two controls that must be run, or the metric is uninterpretable:**

- **random ranking** — split uniformly at random. Any criterion must beat this.
- **oracle ranking** — split the quad whose refinement most reduces `error` (computable, expensive,
  offline). This is the ceiling. **The gap between a criterion and the oracle is the improvement
  available**, and without it "better than random" says nothing about how much is left.

**Reference-resolution caveat:** the reference must be finer than any tree under test, and its own
`E` must be high enough that it is not itself under-resolved (§1's `E` finding: `E+1=2` sees 54% of
the spread). State the reference parameters explicitly.

---

## 3. Candidate signals — measure all against §2

Implement each as a selectable `Criterion`, all dumped every run so they can be compared offline
without re-running.

### 3.1 `frac_above_tau` — fraction of footprints hot, not a quantile

The evidence says the useful signal is in the **tail** of a quad's footprint spreads: median stalls
at depth 4, mean and p90 both reach 7. So the fix is not p90 specifically — it is *not the centre*.

The direct form of *"does this quad contain a boundary?"* is **the fraction of footprints above τ**,
not a quantile of the distribution. A quad with 5% hot footprints has a filament; one with a high
median is uniformly blurred. Those want opposite decisions and every current aggregate conflates
them.

Compute for both within- and between-footprint variants.

### 3.2 Spatial layout — free information currently discarded

A quad computes `N²` footprint values and keeps one scalar. **Where the hot footprints sit**
distinguishes the two cases that matter:

- a **boundary** shows as a *connected, thin* structure
- **chaos** shows as *scattered* hot footprints

Cheapest useful summaries, all O(N²) on data already in hand:

```
n_hot            = count of footprints above tau
n_components     = connected components of the hot set (4-connectivity)
largest_component= size of the biggest
perimeter_ratio  = boundary length of the hot set / n_hot     (thin -> high, blobby -> low)
```

A thin connected structure (`largest_component` large, `perimeter_ratio` high) is a boundary →
**split**. Scattered hot footprints (`n_components` ~ `n_hot`) is chaos → **floor**. This is a more
direct answer to the split/floor question than α, and it needs no second level.

### 3.3 Neighbour contrast

Interesting regions are where spread **changes**, not where it is high — a uniformly chaotic quad
and a uniformly smooth one are both boring; the boundary between them is the structure.

```
contrast = max over the 4-neighbours of |spread_self - spread_neighbour|
```

No new computation; the neighbours' reductions exist. **And it sidesteps τ**, which the vertical
slice showed is now the dominant parameter (64× on `far`). A gradient-driven criterion has no
absolute threshold to get wrong.

Caveat: neighbours may be at different levels. Compare against the neighbour's value at the *same*
level where available, or up-tree, and state which.

### 3.4 Sibling-spread reliability — carried forward, still uncharacterised

Already implemented (`alpha_sibling_spread`, the range of four children's α). Flagged twice as
needing its own noise characterised, still not done. **Characterise it here**: the range of four
samples is a noisy statistic, and if its own scatter is comparable to the signal it is not usable.

### 3.5 Escape-time gradient — cheap, and worth a look

`t_end` varies smoothly except at boundaries, so its **spatial gradient across the quad's footprints**
is a boundary detector needing no ensemble at all. Censoring applies (`t_end` pinned at the horizon
where nothing escapes), so compute it only over the escaped subset and report the escaped fraction
beside it. Likely useless in `near-field` at `t=13` where nothing escapes — which is itself worth
recording rather than assuming.

---

## 4. `QuadReduction` — yes, it needs extending

Currently: aggregates of per-footprint spreads, α, trust fields. Add:

| field | why |
|---|---|
| `between_shape`, `between_event` | §1 — the signal the criterion should have been reading |
| `frac_above_tau_within`, `frac_above_tau_between` | §3.1 |
| `n_hot`, `n_components`, `largest_component`, `perimeter_ratio` | §3.2 |
| `t_end_gradient`, `escaped_fraction` | §3.5 |
| `running_max_divergence`, `divergence_trend`, `first_divergence_t` | §5 |

All are O(N²) over data already computed, so the marginal cost is negligible against 512
trajectories per quad. **Dump everything every run** — the point is to compare criteria offline
without re-integrating.

`contrast` (§3.3) is **not** a field: it is computed from neighbours at decision time, and storing it
would freeze a relative quantity, which is the mistake the screen floor's *"never cached as a quad
fact"* rule exists to prevent.

---

## 5. The temporal accumulators are specced and missing

The scheduler contract:

> *"refinement splits on live spatial disagreement **or latched temporal accumulators**"*

Only the first half exists. `running_max_divergence`, `divergence_trend` and `first_divergence_t`
are in the `QuadReduction` spec and unimplemented — **half the trigger is absent.**

`first_divergence_t` matters most: it is the one signal that **cannot saturate**. Every
instantaneous spread saturates once copies fill the accessible space — and then reports `λ ≈ 0` for
the *most* chaotic regions. A crossing time cannot do that.

Implement all three, accumulated at sync boundaries, latching (max-updated, never decayed).
`running_max_divergence` is empirically justified: spread was observed to **fall 6×** between `t=6`
and `t=8` in one region, so an instantaneous read genuinely misses divergence that has occurred.

**Then measure whether they add anything at `t=13`** — they may not, since the march is short. A
clean null is a fine result and should be reported as one.

---

## 6. The production colour scheme — and why it is a criterion question

Renders are currently coloured by outcome class or a log-scaled spread ramp. Both are **diagnostic**
colourings. The production scheme is **bivariate**: **hue from the shape sphere, lightness from a
scalar** (diffusion, FTLE, or spread).

**This is not presentation.** The criterion asks *"would splitting change what we display?"*, so
**what is displayed determines what the criterion should measure.** `spread_shape` maps to hue, so
that half is aligned. **If lightness carries diffusion or FTLE, the criterion is currently blind to
changes in it** — a quad could be uniform in shape and structured in diffusion, and nothing would
refine it.

Implement:
- hue from `shape_vec` (a direction-to-colour map on the sphere; simplest defensible version is
  fine — state which)
- lightness from a selectable scalar: `diffusion`, `ftle`, `ensemble_spread`
- render every region under it, and **check whether the tree matches the visible structure** — the
  §2 metric measured in this colouring rather than the diagnostic one

**Then answer the coupling question:** does `error(B)` under the §2 metric change when lightness
switches from spread to diffusion? If it does, the criterion needs a term for the lightness field
and currently has none.

---

## 7. The slippy map — panning, not just zooming

The zoom ladders test the screen floor's view-relativity, which is real. **Panning is untested**, and
that is where the caching contract lives.

- pan the camera across a region in steps; does the tree persist for quads that stay in view?
- what fraction is recomputed per step that need not be?
- does a quad floored at one camera position stay consistent when it re-enters view?
- and the interaction: **the playhead is fixed within a run, but a pan changes which quads are
  live.** State what happens to a quad's reduction when it leaves and re-enters view.

No eviction policy — just measure what *would* be evictable and what is recomputed.

---

## 8. Two more, lower priority

**Cost-aware priority.** Some quads are far more expensive (close encounters, more substeps).
Ranking by `spread / compute_cost` spends the budget better if costs vary widely. `total_substeps`
is already available. Cheap to test; report the cost distribution first, since if it is narrow the
idea is moot.

**Anisotropic splitting.** A boundary running diagonally gets four children, three of them mostly
wasted. Splitting 2-way along the disagreement direction — which §3.2 already measures — is
strictly cheaper. **Costing only in this round**: how many splits produce children that immediately
`keep`? If that fraction is large the idea is worth building; if small it is not.

---

## 9. Order, and the constraints

**Order:** §1 (the correction) → §2 (the metric) → §3 (candidates) → §6 (colour) → §5 (temporal) →
§7 (panning) → §8 (costing only).

§1 and §2 before anything else: §1 because the criterion is reading the wrong quantity, and §2
because without it "improvement" has no definition.

**Two constraints on every result:**

- **Validate on `deep interior`.** It is the one region where the current mechanism demonstrably
  fails. A change that only improves `near-field` is tuning.
- **Leaf counts are slice-conditional to 4.3×.** Comparisons must be **within** a slice, never
  across. The α distribution is stable (0.172–0.289) and is the safer quantity to compare.

**And carry forward:** p90 under the veto on `deep interior` — mean and p90 reach depth 7 and the
floor sits at 6, so the fix and the floor may collide, and that is the configuration production runs.

**Cautions.** Do not tune to a nice-looking tree — the §2 metric is the result, the picture is a
diagnostic. Report negative and messy results. Every PR here has corrected something stated with
more confidence than it deserved, including this brief's §1, which corrects the criterion itself.
