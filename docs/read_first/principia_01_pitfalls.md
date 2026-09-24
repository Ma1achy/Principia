# Pitfalls — failures that looked like results

**Companion to `principia_00_philosophy.md` §4.** That file states the commitments; this one records
the failures that earned them, in enough detail to be recognised again.

**Every entry has the same shape:** what was observed, what it was assumed to be, what it actually
was, and the measurement that settled it. The middle two are the useful part — a pitfall you only
know the answer to is a fact; one you know the *wrong* answer to is a warning.

---

## 1. THE PATCHWORK — the longest-running one, and the most instructive

### 1.1 What was observed

Rendered charts showed structures that **started and stopped**. Ribbons that ran continuously in an
older implementation would extend, terminate, and reappear further on. Around the central fractal
region there were **hard geometric boundaries** — a semicircular dome, a triangular tent, radiating
straight-edged wedges, and pale outlines that appeared to *envelope and truncate* the detail inside.
Elsewhere, fine **dotted concentric arcs** across smooth gradients.

**The observation that made it findable, and it came from the user, not the analysis:** *the ribbons
match where they are well behaved.* An unregularised leapfrog and a regularised Aarseth–Zare
integrator agreeing in the tame regions means **the physics core is right**, so anything
discontinuous has to be logic.

### 1.2 Four wrong theories, in order

**(a) It is `t_end` quantisation.** Escape was tested only at sync boundaries, so `t_end` took 32
values at `n_sync = 32`. Plausible — and *partly true*, since the escape cadence was genuinely
coarse. But decoupling it left the rendered image **bitwise identical**. `t_end` moved on 99.7% of
pixels and `shape_vec` on **none**.

**(b) It is 8-bit output quantisation.** Proposed after (a) failed. Killed by one measurement: a line
crossing was `(120,57,35) → (245,171,146) → (122,57,36)` — a **two-pixel spike of +350 in RGB sum**,
returning to background. 8-bit quantisation steps by **~3** and produces monotone staircases, never
isolated bright lines on a smooth field. **Wrong by two orders of magnitude.**

**(c) It is the colour map — the ensemble-spread lightness channel saturating at basin boundaries.**
Refuted by the user from prior knowledge: the same slices rendered in event mode, greyscale and hue
mode all show the same structures. **Colouring is independent of the physics; a structure that
survives every colour map is not a colouring artefact.**

**(d) It is the chart geometry — energy and angle thresholds.** A circle in momentum space *is* an
energy contour, and wedges *are* angle thresholds, so the shapes were right. But the predicted
`E = 0` radius was 0.16 of the half-width against an observed ~0.25–0.30, and the dome was offset
rather than centred. **Right family, wrong instance.**

### 1.3 What settled it — a controlled experiment, not an argument

The reference GLSL decode and integrator were ported to NumPy and the same slice rendered twice,
**with exactly one variable changed**:

| | result |
|---|---|
| integrate all to `t_max`, colour by `theta` | **continuous ribbons, no domes, no hard edges** |
| identical code, freeze `shape_vec` at escape termination | **the domes, tents, wedges and seams appear** |

**The reference implementation grew the artefact when given one behaviour of the new one.** Same
physics, same integrator, same decode, same slice.

### 1.4 The actual bug

`stop_on_event: true` is the default. A terminated trajectory's `shape_vec` is **the state at its own
`t_end`**, and termination happened at a sync boundary — so there were ~32 possible stopping times.

**The rendered field was a patchwork of ~32 time strata stitched at hard seams.**

A ribbon is a level set of `theta` **at a common time**. Where neighbouring pixels froze at different
times, the ribbon breaks and resumes. The pale outlines are the seams. The boundaries are *smooth
curves* because `t_end` is a smooth function — which is exactly why they looked wrong: **a real basin
boundary is sharp but ragged; a stratum seam is sharp and clean.**

**Exposure is what made it visible here.** The reference terminates on collision in every mode but on
escape **only in basin mode**, where the colour *is* the outcome and freezing is correct. On the same
slice the reference freezes **0.15%** of pixels; the new implementation froze **99.4%**. Same
mechanism, **660× the exposure**.

### 1.5 The root cause underneath the root cause

The escape gate was **missing a condition the reference has**:

```
reference:   dist > r_esc  &&  outward > 0  &&  E_out > 0      THREE conditions
ours:                          dr·dv > 0    &&  spec  > 0      TWO
```

**No distance gate at all.** Escape was declared the instant relative energy went positive and the
body was receding — **at any distance, including mid-encounter**, where two-body energy goes
transiently positive and then re-binds. Independently measured: of 895 trajectories escaping under
the fine cadence, **0 were still unbound at +1, +2, +3, +4 or +8 boundaries. Every one re-bound.**
An escape fraction about to be overstated **5.8×**.

`r_esc` is a persistence guard done **geometrically** rather than temporally — and a better one,
because distance is monotone on a real escape whereas a time window is a heuristic.

### 1.6 The fix — see §2 for the criterion

Escape is now `|Δn̂| < tau ∧ E_rel > 0` (§2). Under it the toggle **barely matters**: freezing a
*converged* trajectory is nearly a no-op, so `stop_on_escape` on and off give near-identical images.

**The lesson generalises past this bug: the patchwork was never caused by stopping. It was caused by
stopping while the displayed quantity was still moving.**

### 1.7 What to recognise next time

- **Smooth, clean geometric boundaries in a chaotic field.** Fractal boundaries are ragged. Circles,
  straight edges and smooth arcs are **level sets of something smooth** — usually a termination time.
- **Structures that start and stop.** A continuous feature broken into segments is a field sampled at
  inconsistent times, not a field with gaps.
- **Two implementations agreeing where tame and diverging where not.** That localises the fault to
  logic, not to physics, and it is the strongest single clue available.

---

## 2. THE ESCAPE CRITERION — what replaced it, and why

### 2.1 Escape is a limit, not a threshold

For a hierarchical escape the binary separation stays bounded while the third body's `lambda` grows
linearly. Since `|rho~| = R cos alpha` and `|lambda~| = R sin alpha`:

```
tan alpha ~ t   =>   alpha -> pi/2   =>   n_0 = cos 2 alpha -> -1,  error ~ 1/t^2
                                     =>   |dn/dt| ~ 1/t^3
```

**The shape vector converges to a pole.** Escape is a **limit being approached**, not a threshold
being crossed — and closure measures exactly that, in the quotient space where translation, rotation
and scale are already removed.

Measured decay from `t = 27 → 30`: **1.2** against a predicted 1.4. The shortfall is expected (real
escapers are still decelerating, so `lambda` is not yet perfectly linear).

### 2.2 The criterion

```
ESCAPE  <=>  |Δn̂| over a window < tau    AND    E_rel > 0
```

**Defined (R-29, 24 Sep 2026):**

- `E_rel = ½|Δv|² − (M_pair + m_b)/d` is the relative two-body energy of the candidate escaper `b` about the centre of
  mass of the other two: `Δv` and `d` are `b`'s velocity and distance relative to that centre of mass, and `M_pair` is the
  pair's mass (`G = 1`). It uses the **total** mass. An `M_pair`-only form (prin-rs) biases toward escape.
- **The window:** `|Δn̂|` is taken over 0.4 time units (**provisional**), sampled at macro-step boundaries for
  unregularised occupants and at sync boundaries for regularised ones (R-95).
- **The escaper** is the body with `E_rel > 0` and the largest separation from the other two: its distance `d` to their
  centre of mass, the same `d` as in `E_rel` (R-61).
- **To re-measure:** precision, recall and the `tau` gap were measured before `E_rel` was fixed. Re-validate them with
  this `E_rel`.

Measured on the config chart, ground truth = unbound and receding at `t = 30`. *These numbers predate R-29's `E_rel` (prin-rs's
implementation uses `M_pair` only). To re-validate against check 2's independent ground truth (§2.4), with this
legacy `t = 30` set kept as a comparison (R-95).*

| criterion | fires | **precision** | recall | median `t` |
|---|---|---|---|---|
| `E > 0 ∧ receding` (the old one) | 83.8% | 97.9% | 100% | 1.50 |
| + `d > 5` (the reference) | 82.7% | 99.0% | 99.8% | 2.00 |
| **closure alone** | 95.4% | **82.8%** | 96.3% | 9.00 |
| **closure ∧ `E > 0`** | 79.0% | **100.0%** | 96.3% | 10.00 |
| + receding | 79.0% | **100.0%** | 96.3% | 10.00 |
| + `d > 3` | 79.0% | **100.0%** | 96.3% | 10.00 |

**Two conditions, not four.** Receding and `d > r_esc` add **literally nothing** — identical to the
digit — because once closure and energy both hold, the body **is** receding and far away by
construction. **Three tuned constants eliminated.**

**Neither condition alone is sufficient**, and the failure modes are complementary: closure alone is
82.8% because *settling* also happens for bound hierarchies; energy alone is 97.9% because it
**flickers** during encounters.

**`tau` is not a tuned constant** *(to re-measure, R-29: prin-rs reports the gap at best 6.8× in its build)*. Closure separates escapers from bound trajectories by **383×**
(7.04e-05 against 2.70e-02), stable across `t = 25–30`. Any value in the middle two orders gives the
same answer.

### 2.3 The 96.3% recall is the right failure direction

~4% of genuine escapes have not settled enough to fire by `t = 30`. They fire later. **Late rather
than wrong** — which matters because `t_end` is a *stored, reported* quantity feeding the escape-time
gradient, the outcome label and the colouring. A criterion firing at `t = 1.5` with 0.5% false
positives writes **wrong timestamps into the payload permanently**; at 10^6 pixels that is thousands
of them, off by as much as 7 time units.

**Firing early is only a virtue if you are saving compute. For a measurement it is a liability.**

### 2.4 Should it terminate?

**Argument for:** if the state is genuinely stationary past `t_end`, `state(t_end) = state(t)` for
all later `t`, so stopping **preserves** purity rather than breaking it. And for **time-averaged**
fields continuing is actively wrong — `FTLE = S/T` past escape adds nothing to `S` while growing `T`,
**diluting the measurement**. Carrying a settled trajectory as `running` also misrepresents what is
known.

**Argument against:** the compute saving is small (fires at `t≈10` of 16, on 26% of pixels, and
escaped trajectories are the *cheap* ones), and stopping bakes a heuristic into a payload that is
meant to outlive it.

**Ruled (R-31, R-95, R-103).** Once escape fires, `state` reads escape and `t_end` is fixed. Time averages
(FTLE's `S/T` and the like) freeze at `t_esc`, so the dilution above cannot happen. In production `done`
is set when escape fires and the loop ends. The post-escape march for the three checks below runs only
in the validation harness, which keeps its own state; the payload never sees it. The checks are outstanding, and
they are the ones that killed the previous criterion; each one's pass threshold, horizon and fixture
are set by calibration (R-71):

1. **Integrate 2–3× past firing** and confirm nothing re-binds — the direct analogue of the 0-of-895
   test.
2. **Independent ground truth** — separation growing without bound, not an energy sign. The current
   ground truth *shares a term* with the criterion.
3. **`deep interior`**, where the transient population actually lives. The config chart's base rates
   are too high to discriminate.

**Collision stays terminal regardless** — it is a **singularity**, not a heuristic. There is
genuinely nothing to integrate past.

---

## 3. Standing rules earned in this sequence

**RUN IT, DO NOT REASON ABOUT IT.** Four wrong theories were produced by reading code and reasoning.
A ten-minute port settled what an hour of argument could not. **When an image looks wrong, reproduce
it with one variable changed.**

**A QUANTITY THAT DOES NOT CONVERGE UNDER REFINEMENT IS MEASURING THE SAMPLING, NOT THE SYSTEM.**
Escape fraction went 0.0947 → 0.2153 → 0.4423 → 0.5494 at strides 0, 32, 4, 1 — **largest relative
step at the finest stride**. That is the wrong shape for a resolved quantity, and it was read as
"converging". Same signature as drift-not-falling-with-step-size, one level up.

**A DIAGNOSTIC MUST REUSE THE ORIGINAL DISCRETISATION.** A persistence check re-ran trajectories with
`n_sync` rescaled per window, making every window a different discretisation. It produced
0.162, 0.219, 0.011, 0.083, 0.335 and read exactly like a flickering condition. **The project's own
`n_sync`/`t_max` trap, inside a diagnostic written to catch traps.**

**A DISCRIMINATOR CAN BE POISONED BY ITS OWN SUBJECT.** `d_min` was used to test whether new escapes
were spurious mid-encounter firings, on the grounds that the re-labelled population had *larger*
separations. But **a run stopped early never reaches its close approach** — its `d_min` is larger
*because it terminated early*. It was measuring the termination, not the encounter.

**A THRESHOLD ON A QUANTITY SPANNING DECADES MUST BE RELATIVE.** Closure was first dismissed using an
absolute cutoff of 2e-3, which sits *inside* the bound population's range. The same defect as
`tau_display` at the 0.4th percentile. **Set thresholds from the observed distribution, or from a
measured gap.**

**CHECK THE MEASUREMENT CAN FIRE BEFORE READING IT.** A proposed test — "does `frac_hot_between`'s
distinct-value count go from 45 to thousands" — was **arithmetically impossible**: it is a fraction
over `N² = 64` footprints, so at most 65 values by construction. Written while cataloguing exactly
that defect.

---

## 4. RE-REGISTRATION COUNT — the mechanism that had to be found twice

### 4.1 The finding

Passing the state through a coordinate chart costs accuracy, and **the cost scales with how often
you do it, not with which chart you pick.**

Measured by doubling the sync-boundary re-registration count **at fixed step size** (`eta` adjusted
so `steps p50` stays flat within 6%, so this is not a step-size result wearing another label):

```
  LC branch unconditioned              2.5e-6 decades
  hysteresis, switches 17.8 -> 6.7     7.5e-5 decades
  re-registration x2, fixed step       4.4e-1 decades    <- 6000x and 175000x
```

> **It is not which chart is chosen. It is how often the state is passed through one.**

### 4.2 What it caused

Pale, straight-edged wedges cutting through otherwise continuous ribbon structure on
`config_stability`, with magenta speckle at their cores. Reported as *"chaos does not cut and
resume; a threshold does"* — broadly right, and specifically wrong in almost every particular,
across **eleven hypotheses of which nine were refuted.**

### 4.3 The control that found it

**An independent integrator on the same initial conditions produces an unrelated drift field.**
Leapfrog drift tracks FTLE at **+0.305** — what physics looks like. AZ drift tracks FTLE at
**−0.082** and leapfrog drift at **−0.096**.

**Keep a regularisation-free occupant permanently for this reason.** It shares no coordinate
machinery with the others, so it is the only arm that can adjudicate when they disagree.

### 4.4 The remedy, and its own falsification test

Heggie 1974 global regularisation: three relative vectors on equal footing, **no reference body to
re-choose and therefore no re-registration at all.** Measured **31 of 32 cases**, `err>10`
**3916 → 73**, AZ's worst decile fixed on 100% of pixels.

**And the loss is the strongest evidence.** `far` is AZ's only win and it is total — all 65,536
pixels — because there one body stays distant, AZ's reference choice is ideal, and **it never
re-registers.** A mechanism that predicts its own exception is worth more than one that only wins.

**The outstanding falsification:** logH (algorithmic regularisation) has *no coordinate
transformation at all* — a strictly stronger form of the property Heggie's win is attributed to.
**If the mechanism is real, logH should match or beat Heggie. If it does not, the mechanism is
wrong.** Run it under the same RK4 and step control, or the comparison scores the integrator
instead.

### 4.5 THIS WAS ALREADY ON RECORD

From this project's own earlier notes, months before the wedge investigation:

> *"LC inverse injects error `n_sync` times per trajectory. Registration is at every sync
> boundary."*

**Known, written down, and then rediscovered over four days.** A finding that has to be found twice
is a filing problem, not a discovery problem. **A mechanism belongs in this document, not only in
the investigation record that found it** — investigation records are read once; pitfalls are read
before the next investigation.

---

## 5. MEASURING IN THE WRONG SPACE — `error(B)` in OKLab

**The refinement metric scored the colour map, not the field, and nobody noticed for months.**

`error(B)` measured OKLab distance against a reference image, under the shipping colouring — whose
lightness is **auto-ranged per region**. So a smooth region's `1e-8` residual counted as error at every
depth, and **breadth-first came out near-optimal BY CONSTRUCTION OF THE METRIC.**

Every "nothing beats uniform" conclusion in this project was an artefact of that. Scored on the
**payload** instead — `shape_vec` and event class against a fixed `eps` — the same footprints give
**137 quads against uniform's 5449** on `near-field`.

**The general form: a metric that shares a stage with the thing under test is not independent of it.**
The colouring was downstream of the criterion, and scoring in its space made the criterion's failures
invisible.

**Remedy, standing:** the criterion is shader-agnostic by design, so **its evaluation must be too.**
Any criterion work scored in render space is void before it starts.

---

## 6. READ `dp/u` BEFORE `tol/u`

**A low saving has two causes and only the exact optimum separates them:**

- a field with **nothing to find** — the optimum is also ~1×
- a policy **failing to find it** — the optimum is much better

Both occur on the same chart one decade of `eps` apart. Without the DP ceiling, `tol/u ≈ 1.0` reads as
a broken policy when it is a featureless field — which is what happened, twice, before it was measured.

On the sea charts the ceiling is **1.08–1.44×** over breadth-first in all 24 cells and the policy sits
at 1.10–1.45× of *that*. **It is not underperforming; the optimum is breadth-first.**

---

## 7. A STOP-REASON BREAKDOWN NEEDS WEIGHTING BY WHAT IT FORECLOSES

The standing rule was: never quote a leaf count without its stop-reason breakdown. **That is not
enough.**

Only **4 leaves** read `floor` in the `near-field` `t=50` case — **at level 2, each foreclosing a
quarter of the frame**, suppressing a descent that reaches 829 quads at zero error. **A decisive stop
reads as a rounding error in a leaf count.**

**Weight each stop by the subtree it prevents**, not by the leaf that carries it.

---

## 8. TWO ARTEFACTS ARE NOT ONE DEFECT

The magenta pixels and the wedges were treated as one phenomenon for days. The ablation:

```
arm            non-finite   wedge density
none              1153         0.0026
dtau only           32         0.0026     <- magenta gone, WEDGES UNTOUCHED
clamp only        1531         0.0024     <- alone it makes non-finite WORSE
limit only           0         0.0001     <- reproduces the all-three arm on every column
all three            0         0.0001
```

**The unchanged column — `0.0026 → 0.0026` — is the finding.** Hunting a single cause for two
co-located symptoms cost most of a week. **When a fix removes one symptom and leaves another
numerically identical, that is two defects, and the identical column is the evidence.**

---

## 9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN

`roundtrip_ctl` — the descriptor pack/unpack round-trip test — **passed on a deliberately
contaminated value.** `from_bits` masks to bits 2–4; the fork landed in bits 0–1. **A parity check
placed at the decode passes on a wrong answer.**

Same family as `alpha_area` returning exactly `0.0000` for both an empty mask and a full one, and as
`dt_max` folding from `0.0` so an all-unusable pixel reported *"the largest step it took was zero"* —
from the diagnostic built to catch `2.209e128`.

**The general form: a check whose reachable output set does not include the failure it guards
against.** It cannot fail, so it tells you nothing, and it does so while reading green.

**And the companion finding from the same audit:** `packed` and `packed_ctl` are *the same
shift-and-mask expression* fed by different buckets — **0 forks against 83**. So **a descriptor is
exactly as deterministic as the branches feeding it**, and auditing the packing finds nothing. Audit
the inputs, not the encoding.

---

## 10. GENERALISING A STATELESS RESULT TO A TRAJECTORY

The lowering spike measured **705 boundary states, 0 forks** — one comparison each, inputs held
fixed. I read that as evidence that outcome labels would agree across backends on a real trajectory.

**It is not, and cannot be.** A label is a function of the trajectory; the trajectory accumulates;
FMA contraction and rounding order differ; so `d²` eventually lands on the other side of `r_coll²`
and the label flips. **The test held the inputs fixed, which is exactly the condition that does not
obtain after ten thousand steps.**

What the result *does* establish is narrower and still worth having: **given identical inputs, the
operation itself does not disagree.** So when a decision differs, it is because the inputs differed —
which is precision, which is expected, which is reported. **Not because two backends implemented
`ceil(pow(...))` differently.**

**The rule: a test that holds a variable fixed says nothing about the regime where that variable
moves.** State the domain of a determinism result as loudly as the result.
