> # ARCHIVED — COMPLETE — `dom_KE` REJECTED
>
> Answered: **`dom_KE` decided its own verdict** and no value worked in more than one configuration. It
> is not in the shipped design. Recorded in `principia_00_philosophy.md` §4.2 as one of the four
> picked-constant failures.
>
> **Evidence:** `prin-rs` — `FINDINGS.md`, `README.md`, `results/`.

# Brief 2: choosing `dom_KE` — is the `max` criterion well-posed?

**Prerequisite:** you have already run Brief 1 (`principia_brief_refinement_experiments.md`) and
produced `REPORT.md` + `xp_results/`. This brief follows directly from finding **1(c)** of that
report. Everything here reuses machinery you built. **Read §1 before running anything** — the
question has changed shape since Brief 1, and the framing matters more than the numbers.

**Two harness changes since Brief 1 — take the new files:**

1. **`tb_az.py` is patched.** Non-finite trajectories now exit the step loop. This removes the
   `max_steps` burn you diagnosed (354 s against ~3 s nominal) that truncated 43% of Experiment 2.
   Verified to give identical drift and FTLE on a clean probe. **Use the new copy.**
2. Nothing else changed. Your `xp_*.py` files should work unmodified.

**Adopt from your own Brief 1 findings:** gate `1e-4` (not `1e-3`), tolerance `eta = 0.01`. Report
`alpha_E` with everything, flag `|alpha_E − 1| > 0.05` as untrustworthy, save raw JSON incrementally,
skip `deep interior`.

---

## 1. Why this is the blocking question

`ensemble_spread` is defined as a **`max` over contributors**, each normalised to `[0,1]`:

```
ensemble_spread = max( shape_n̂ / 2.0,
                       event_disagreement / (1 − 1/(E+1)),
                       KE_spread / dom_KE )
```

Two of those normalisations are **principled**: `shape` is a distance on the unit sphere so 2.0 is
the chord bound; `event` is a disagreement fraction so `1 − 1/(E+1)` is its attainable maximum. Both
are *achievable maxima of a bounded quantity*.

**`dom_KE` is not.** Kinetic energy is unbounded, so `2.0` was a placeholder. Your Brief 1 finding
1(c) showed this is not cosmetic — **it decides the verdict**:

| KE divisor | burrau | eq_rot (L≠0, E<0) | eq_fast (E>0) |
|---|---|---|---|
| /1.0 | 0.92 | 0.69 | 1.00 |
| **/2.0** | **0.92** | **0.49** | **0.93** |
| /4.0 | 0.90 | 0.24 | 0.55 |

`eq_rot` crosses the 50% falsification line between /1.0 and /2.0. So *"KE dominance is falsified at
`L≠0`"* is true at the specified constant and not robust to it. **The `max` is comparing a bounded
quantity against an unbounded one on an arbitrary scale, which is a defect in the criterion rather
than a fact about the physics.**

**Physically-derived scales were tried and are worse.** Normalising KE by `|E_0|` or by the virial
scale `GM/R`, then sweeping a residual multiplier 0.5× → 4×, gives winner-share ranges of **0.77 and
0.69** against **0.04** for the plain constant. They change KE's magnitude but not its unboundedness,
and push it near the crossover where the multiplier dominates. **Do not pursue that direction.**

### The hypothesis to test

The rest of the system normalises every field by its **declared display domain** — the range the
colour ramp spans. That is the principled bound here too, on this reasoning: *a field's spread
matters exactly insofar as it would change the displayed colour*, so `spread / dom_KE` is the
fraction of the colour ramp the uncertainty covers.

If so, `dom_KE` must be chosen on **display** grounds — and the question becomes whether a
display-sensible choice also makes the criterion behave.

**This experiment asks: does such a value exist?**

- **If yes** — some `dom_KE` gives a stable winner-share across configurations *and* renders sensibly
  — the criterion is well-posed, and the `L≠0` verdict resolves one way or the other.
- **If no** — no value satisfies both — that is strong evidence **KE should not be a contributor at
  all**, and the set reduces to the two bounded fields. That is an equally clean and equally useful
  answer.

**Both outcomes are publishable results. Do not try to rescue KE.**

---

## 2. Experiment A — the `dom_KE` sweep (main run)

**Configurations.** The same three as your Brief 1 Experiment 1: `burrau` (L=0, E<0), `eq_rot ω=0.3`
(L≠0, E<0), `eq_fast ω=1.2` (E>0). Use the same quads. Note your own finding that `eq_rot`'s KE
win-share tracks **radius** within a single configuration (0.19 → 1.00) — so keep the
intermediate-radius quads you added, and report per-quad, not only per-configuration.

**Sweep `dom_KE` over at least:** `{0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0}`, extended if the
interesting behaviour is at an end.

**For every `dom_KE` value, record:**

| quantity | why |
|---|---|
| KE winner-share, per quad and per config | the criterion behaviour |
| shape and event winner-shares | what wins instead |
| **fraction of KE values that clip** (`KE_spread/dom_KE > 1`) | display sanity — heavy clipping means the domain is too small |
| **median `KE_spread/dom_KE`** | display sanity — if this is ~0.001 the ramp is wasted |
| `alpha_E` per quad | trust |

The two display-sanity columns are what make this more than a repeat of your sensitivity table. **A
domain is display-sensible if typical values occupy a usable part of the ramp without heavy
clipping** — as a rough guide, median in roughly `0.05 … 0.5` and clipping under ~5%. Use your
judgement and state the criterion you applied.

**Deliverable:** a table of `dom_KE` against (winner-shares × 3 configs, clip fraction, median
occupancy), and a verdict on whether any value is simultaneously display-sensible and
configuration-stable.

### What "configuration-stable" means here

Not that the share is identical across configurations — it should not be, since these are physically
different regimes. **It means the share does not cross the 50% decision boundary within the range of
display-sensible domains.** If the answer to "does KE dominate" changes depending on which sensible
domain you picked, the criterion is ill-posed regardless of which value you choose.

---

## 3. Experiment B — estimator, on the same runs (cheap, no extra integration)

Your finding 1(b): `ols2` fails the control at every threshold in the stressed `eq_rot` core while
**Theil–Sen returns 0.86–1.05 on the same data**. Production uses exactly `ols2`.

**On the runs from Experiment A**, compute `alpha_E` with **both** `ols2` and `theil`
(`estimators.py` has both) and report:

- how often `ols2` fails the trust bar (`|alpha_E − 1| > 0.05`) while `theil` passes
- whether that co-occurs with anything detectable at runtime — low retention, high max drift, bimodal
  drift, high spread scatter

**The point:** if `ols2`-fails-while-`theil`-passes is a *recognisable* signature, production can
detect the condition and fall back rather than needing Theil–Sen everywhere. Say plainly whether it
is recognisable, and from what.

---

## 4. Experiment C — `t_end` re-scoped (only if A and B leave time)

Brief 1 finding: `t_end`'s resolution *is* the sync interval, because `tb_all_az.py` assigns
`t_end = t_now` at sync boundaries only. Your fine-sync re-run gave `alpha ≈ 0.76–1.37` where the old
settings gave `nan`. **You diagnosed a real defect in the harness, not in the field.**

The proper fix decouples the **escape-test cadence** from the **sync interval**: test for escape more
often than the ensemble syncs, and record the actual time of first detection rather than the boundary
it was noticed at. Better still, bisect within the interval where escape is detected.

**If you attempt it:** add a new module, do not edit `tb_all_az.py`. Report `alpha` for `t_end` with
its control on ≥3 regions. **Carry forward your own caveat** — restricting to escaped copies makes
the surviving subset depend on perturbation size, which breaks the linearity the energy control
assumes, so those controls sit on the same biased subset.

**This is optional.** A and B are the blocking questions.

---

## 5. Reporting

As Brief 1: raw-number markdown tables, explicit verdicts, anomalies section, raw JSON saved.

Two things I specifically want, in your own words:

1. **A recommended `dom_KE`, or a recommendation that KE be dropped.** Whichever the data supports. If
   the honest answer is "no value works", say that — it settles the contributing set at two fields and
   is a better outcome than a fudged constant.
2. **Whether the `max` formulation itself is sound.** You identified that mixing bounded and unbounded
   contributors is the underlying defect. If, having swept it, you think `max`-of-normalised is the
   wrong aggregation — that some other rule (rank-based, or bounded-contributors-only, or a squashing
   function) is better posed — **say so and why.** That is a design question you now have more
   evidence on than I do, and I would rather have the objection than a number that hides it.

Your Brief 1 report was materially better for flagging the KE normalisation defect rather than just
reporting the falsification. Same instinct here: **the framing findings are worth more than the
tables.**
