> # ARCHIVED — COMPLETE — experiments run, most conclusions overturned
>
> These experiments were run across four external rounds. **Most of their conclusions did not survive**
> the substrate fixes: they were measured with the alpha criterion, `error(B)` in render space, and
> before the step-control corrections.
>
> **What survives:** the methodology, and the corrections themselves — which are the more valuable
> half. See `principia_ARCHIVE_dd_refinement_criterion_v0.md` §7.
>
> **Evidence:** `prin-rs` — `FINDINGS.md`, `README.md`, `results/`.

# Brief: outstanding experiments on the Principia refinement criterion

**Audience:** an agent with **no prior context on this project**. Everything needed is in this file
plus the seven Python modules listed in §2. Read §1 and §2 before running anything.

**Deliverable:** results for the three experiments in §5, in the reporting format of §6, plus a short
statement for each of whether it **confirms or falsifies** the specific claim named in §5.

**Do not refactor the harness.** It is research code that has been debugged against known-answer
controls; silent "improvements" have already caused two false conclusions (§4). Add new files rather
than editing the existing ones.

---

## 1. Context — what this is and why these runs matter

### 1.1 The project

Principia is a browser-based instrument for exploring the **planar three-body problem's initial
condition space**. It renders a 2D slice of an 8D manifold of initial conditions: each **pixel is one
simulation**, integrated forward, and coloured by some property of its trajectory (its outcome, its
Lyapunov exponent, its energy, and so on).

Because the three-body problem is chaotic, that colouring is fractal at every scale. So the renderer
uses an **adaptive quadtree**: it starts coarse and subdivides ("refines") regions where more
resolution would reveal more structure.

**The whole question this brief serves: how does the renderer decide which regions to refine?**

### 1.2 Vocabulary (used throughout, and in the code)

| term | meaning |
|---|---|
| **quad** | a square patch of IC space; the unit of scheduling. Refining halves its width |
| **sample** / **footprint** | one pixel — one nominal initial condition |
| **ensemble copies** | `E` extra simulations per sample, jittered *within* that pixel's footprint. Their disagreement measures whether the pixel's colour is well-defined |
| **jitter fraction** (`jf`) | size of that jitter, as a fraction of the cell size. **Scales with the cell**, so refining halves it too |
| **playhead** (`t`) | all samples advance to the same physical time in lockstep; `t` is the current time |
| **`ensemble_spread`** | one scalar per footprint: how much the copies disagree. Displayed, stored, and used by the scheduler |
| **exponent** (`alpha`) | how `ensemble_spread` scales with perturbation size: `spread ~ delta^alpha`. **This is the refinement criterion** |

### 1.3 The criterion, in one paragraph

Refining a quad halves the perturbation size `delta`. If spread scales as `delta^alpha`, then
`alpha ≈ 1` means refining halves the uncertainty (**worth doing**) and `alpha ≈ 0` means refining
changes nothing (**futile — the region is irreducibly chaotic**). So the exponent, estimated by
comparing a parent quad against its children, is the refinement signal. An earlier proposal — a
*ratio* of within-pixel to between-pixel spread — was tested and **rejected**: the chaotic
amplification cancels in the quotient, leaving only the anti-aliasing setting.

### 1.4 What has been established so far

- `ensemble_spread` is a **`max` over a small fixed set of fields**, each normalised to `[0,1]`.
  Current set: **shape n̂**, **event class ⊕ detail**, **kinetic energy**.
- **Excluded on evidence:** total energy (conserved — its spread measures only that the ICs differ,
  never that the flow amplified them), FTLE spread, `d_min`, `t_end`, diffusion.
- **Total energy is used as a per-quad sanity field.** Its exponent is *known analytically to be
  exactly 1.0* (its spread is `|grad E| · delta`), in every region at every playhead. This makes it a
  free, always-available ground truth: **any estimator that cannot return 1.0 for energy is broken**,
  and the deviation is a direct error measure. **This control is used in every experiment below.**
- **Softening (`eps`) was removed.** A fixed softening length breaks the scale-invariance of Newtonian
  gravity, contaminating results by up to 1.66× under a pure rescaling. Close encounters are now
  handled by **Aarseth–Zare global regularisation** instead (§2.2).
- **A per-trajectory drift gate is mandatory.** Trajectories with energy drift `> 1e-3` are excluded
  *before* reducing. Ungated, 2% of failed trajectories moved a measured exponent from 1.0 to 6.7.

### 1.5 Why the outstanding runs matter

**Everything above was measured on one configuration: Burrau, `m = (3,4,5)`, released from rest.**
Two of its properties are load-bearing and unrepresentative of the wider chart:

- **`L = 0`** (released from rest). Non-zero angular momentum forbids triple collision; `L = 0` does
  not. This is why one test region is pathological.
- **`E < 0` always.** Bound system, so **ionisation is impossible** and escapes are rare — which is
  why one candidate field showed 100% censoring.

The real chart has **free momenta**, so most of it has `L ≠ 0` and reachable `E > 0`. The runs below
test whether the conclusions survive there.

---

## 2. The code

Seven modules, pure NumPy (`numpy >= 1.20`), no other dependencies. All are batched: a "run" is an
array of many ICs integrated simultaneously, shape `(S, 3, 2)` = samples × bodies × (x,y).

| file | contents |
|---|---|
| `tb.py` | **core.** Softened leapfrog, `energy`, `pair_dists`, `classify`, and `burrau_grid` (constructs a quad's samples + ensemble copies). Module globals **`tb.M`** (masses) and **`tb.R0`** (initial positions) |
| `tb_az.py` | **Aarseth–Zare global regularisation** — the production integrator for this work |
| `tb_all_az.py` | `integrate_all_az(...)` — AZ plus every candidate field in one pass (FTLE via Benettin shadows, diffusion, `d_min`, `t_end`, event class) |
| `sweep_az.py` | the region × playhead × jitter sweep |
| `estimators.py` | six exponent estimators; `ESTIMATORS` dict |
| `est_run.py` | multi-scale spread measurement |
| `refine_test.py` | `shape_vec` (shape-sphere map), `disagree`, dispersion measures |

Also read **`principia_dd_refinement_criterion.md`** — §2 is the full methodology, §7.9 is the result
being extended. It is long; §1, §2 and §7.9 suffice.

### 2.1 The key entry point

```python
import numpy as np, tb, tb_all_az as AA, refine_test as R

# a quad centred at (cx,cy), half-width 0.05, 4x4 samples, 7 ensemble copies each
r0, v0, gid, _, _ = tb.burrau_grid(4, 4, cx, cy, 0.05, body=bd, ens=7,
                                   jitter_frac=0.5, seed=0)
res = AA.integrate_all_az(r0, v0, t_max=13.0, n_sync=32, eta=0.01)

good = res['drift'] < 1e-3          # MANDATORY drift gate
for k in range(16):                 # gid groups copies by their parent sample
    s = np.nonzero((gid == k) & good)[0]
    if len(s) < 3:                  # require >=3 survivors per footprint
        continue
    ...                             # spread over this footprint's copies
```

`body=bd` selects **which body's position the slice varies** (0, 1 or 2) — three different slices of
the same system. `res` contains `r, v, ftle, diffusion, dmin, t_end, censored, binary_id, drift`.

### 2.2 Why AZ, and its one limitation

Newtonian gravity diverges as two bodies approach; naive integration fails. **Regularisation** is a
coordinate change (`u = sqrt(separation)` plus a time transformation) that makes the singular term
*constant* — the trajectory passes through close approach at machine precision. Aarseth–Zare
regularises **two pairs at once**, leaving only the longest side of the triangle unregularised.

**It cannot handle a genuine triple encounter** (all three bodies converging), which is provably
non-regularisable. In that case drift explodes. This is expected, and the drift gate exists partly to
catch it. Region `deep interior` (centre `(0,0)`, body 0) is such a case — **skip it**; it costs
>190 s per probe against ~5 s for the rest, and fails anyway.

### 2.3 Cost

A single probe (16 samples × 8 copies, `t=13`, `eta=0.01`, `n_sync=32`) is **~5 s**. Budget
accordingly; the experiments below are sized for a few hours total. **Save incrementally to JSON
after each region** — long unbroken runs have been lost to timeouts.

---

## 3. Mandatory conventions

Violating any of these has already produced a false conclusion in this project.

1. **Always apply the drift gate** (`drift < 1e-3`) before computing any spread, and require **≥3
   surviving copies** per footprint. Non-negotiable — see §1.4.
2. **Always carry the total-energy control.** Report `alpha_E` alongside every exponent. If
   `|alpha_E − 1| > 0.05`, **the other exponents from that quad are untrustworthy and must be
   reported as such**, not quietly averaged in.
3. **Jitter must scale with cell size.** `burrau_grid` already does this. Do not substitute a fixed
   perturbation — it makes spreads drift under refinement for a purely trivial reason.
4. **Never compare across different `eps` or different integrators.** Use AZ throughout.
5. **Report `n`.** Several earlier conclusions flipped between 2 regions and 8. State how many
   regions, samples and copies underlie every number.

---

## 4. Two cautionary notes

**Bugs found here failed *silently* and looked like physics.** Both took a while to catch:

- **Sign errors in the regularised equations of motion.** Symptom: energy drift ~1 that **would not
  fall when the step size was reduced**. That signature — accuracy insensitive to step size — means a
  *wrong equation*, never a step-size problem. Caught by finite-differencing the Hamiltonian.
- **A step-size rule that duplicated the method.** The AZ time transformation already shrinks the
  physical step at close approach; shrinking the fictitious step too drove `dt → 1e-13` and exhausted
  the step budget. This produced a "the physics is intractable here" conclusion that was simply wrong.

**If a result looks physically impossible, check the machinery before believing it.** The energy
control exists for exactly this.

---

## 5. The experiments

### Experiment 1 — cross-system: does the conclusion survive `L ≠ 0` and `E > 0`? **(highest value)**

**Claim under test:** *kinetic energy is the dominant contributor to `ensemble_spread`, winning the
`max` in 69–100% of footprints.*

**Why it may fail:** measured only on free-fall-from-rest (`L = 0`, `E < 0`). KE's dominance may be a
property of that configuration rather than of determinacy.

**Method.** `tb.M` and `tb.R0` are module globals — patch them, don't edit the file:

```python
import numpy as np, tb
tb.M  = np.array([1.0, 1.0, 1.0])                       # equal masses
tb.R0 = np.array([[0.0, 1.0], [-1.0, -0.5], [1.0, -0.5]])
import refine_test as R
R.MT, R.MTOT = tb.M, tb.M.sum()                          # refine_test caches these — must patch too
```

Then add **initial velocities**, which `burrau_grid` does not provide (it starts from rest). After
calling `burrau_grid`, overwrite `v0`:

- **`L ≠ 0`, `E < 0` (rotating, bound):** rigid rotation about the centre of mass,
  `v = omega × (r − R_com)` with `omega ≈ 0.3`. Verify `L ≠ 0` and `E < 0` before running.
- **`E > 0` (unbound / ionising):** same rotation with a larger `omega`, or add outward radial
  velocity, until `tb.energy(r0, v0, 0.0) > 0`. Verify.

Run **≥4 quads per configuration**, spread across the slice, at `t = 13`, `jf = 0.5`. Compute per
footprint, normalised to `[0,1]`:

| contributor | normalisation |
|---|---|
| shape n̂ | `spread / 2.0` (chord bound on the unit sphere) |
| event `class ⊕ detail` | `disagree / (1 − 1/(E+1))` |
| kinetic energy | `spread / 2.0` (provisional; report raw magnitudes too) |
| *(control)* total energy | report `alpha_E` only, not a contributor |

**Record:** mean normalised value per contributor; **the fraction of footprints where each wins the
`max`**; and `alpha_E` per quad.

**Falsified if:** KE's winner-share drops below ~50% at `L ≠ 0` or `E > 0`, or another field
dominates. That would mean the contributing set is configuration-dependent and must be re-derived.

### Experiment 2 — long horizon: is `t_end` permanently excluded, or just mis-scoped?

**Claim under test:** *`t_end` (time to escape) carries no usable signal — it was 100% censored in
three of four regions at `t = 13`.*

**Method.** Escape fraction vs horizon, `t ∈ {13, 30, 60, 120}`, on ≥3 regions.
`res['censored']` is `True` where no escape was detected. Use `eta = 0.02` and scale `n_sync` with
`t` (keep the sync interval roughly constant, ~0.4 time units).

Where escape fraction exceeds ~0.3, also compute the `t_end` spread over the **escaped copies only**
and its exponent, parent (`half=0.05`) vs child (`half=0.025`).

**Record:** escaped fraction per region per horizon; and where meaningful, `alpha` for `t_end` with
its `alpha_E` control.

**Falsified if:** escape fraction becomes substantial (>0.3) by `t ≈ 30` and `t_end`'s exponent is
then well-estimated — meaning it should be reinstated for longer-horizon renders rather than excluded
outright.

### Experiment 3 — calibrate the drift gate **(cheap; do this first)**

**Claim under test:** *`1e-3` is a reasonable gate threshold.* It was chosen by eye and is heading
into a specification, so it should be calibrated.

**Method.** Sweep the threshold over `{1e-2, 1e-3, 1e-4, 1e-5, 1e-6}` on ≥3 regions, at 4–5
perturbation scales (`jf ∈ {0.125, 0.25, 0.5, 1.0, 2.0}`). For each threshold record:

- `|alpha_E − 1|` — the control error (**lower is better**)
- the fraction of trajectories retained (**higher is better**)

There is a trade-off: too loose admits bad data and corrupts the exponent; too tight discards good
data and biases the sample. **Report the knee.**

**Also report:** whether the choice interacts with `eta` (the integrator tolerance) — a tighter `eta`
should let a tighter gate retain more.

---

## 6. Reporting format

For each experiment: a **markdown table of raw numbers** (no plots needed), then a one-line
**confirms / falsifies** verdict against the claim named in §5, then any anomalies.

Include throughout:

- `alpha_E` for every quad, and flag any with `|alpha_E − 1| > 0.05` as untrustworthy
- number of regions, samples per quad, copies per sample
- median and max drift, and the fraction gated out
- for any configuration change: `L` and `E` at `t = 0`, to confirm the intended regime

**Save all raw measurements to JSON** — the derived tables are less useful than the numbers behind
them, and re-running is expensive.

**Report negative and messy results.** Three conclusions in this project were overturned by better
sampling; a clean-looking summary that hides scatter is worse than useless. If something looks
physically impossible, say so and check the machinery (§4) rather than reporting it as a finding.
