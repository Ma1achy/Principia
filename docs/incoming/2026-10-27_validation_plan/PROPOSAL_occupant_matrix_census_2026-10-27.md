# Proposal: the occupant matrix census

27 October 2026. **Status: proposed, not in force.** It is meant for `docs/incoming/2026-10-27_validation_plan/` as
DRAFT-VAL-08 (the next free number; PLAN.md uses 01 to 07), with a review-queue entry for the owner's ruling. Nothing here changes a default, a gate or a task.

## 0. In one paragraph

Run the validation corpus we have gathered through every combination of precision, stepper, regularisation, step
control and build that Principia can build, and record what works where. Start small: show the default works on the
cases we care about, then cross a small set of settings over cheap fixtures. Grow it in the background, in CI and on
the owner's Mac, to the whole corpus and every map replication. The output is a measured map of the configuration
space: an R-71 calibration that the gates and defaults cite, never a gate in its own right. Its scientific product is
knowing which features of a three-body map are physics and which are an integrator's.

## 1. Why

The project's own record says one verdict per occupant is not enough:

- **The best setting depends on the region.** Heggie wins 31 of 32 gallery cases and AZ wins `far` on every pixel
  (`frac better 0.000`, a flat 0.7–0.9 decades).
- **The axes interact, so changing one knob at a time misses things.** Heggie alone is 19x better on drift and the
  predictive step limit alone 29x, but together they are 753x. The `dtau` fix and the landing clamp each made the
  images worse on their own.
- **The settings travel together.** `refine_flagged`, `escape_rule` and `k_frac` all changed results while nothing
  printed them. A census that crosses only precision × stepper × regularisation would repeat that failure.
- **One reading is never enough.** Energy drift was blind to the overshoot defect. A comparison against a ground
  truth saturated at 1.0000 for a correct mode and a broken one alike. The median hid a 1900x improvement on the worst
  decile.
- **Pictures that look like physics can be numerical.** The pale wedges in `config_stability` were the step limit's
  absence. The ribbon banding turned out to be physics, and it took six excluded mechanisms to show it.

The plan already checks each occupant against its own targets: REQ-VAL-170/-045/-030/-041, H-1/H-2, D-2, D-4 and
PL-1 to PL-12. Only PL-9 crosses two axes. Nothing maps the product.

## 2. The corpus

### 2.1 Periodic orbits: the answer key is the orbit itself

Integrate one period. The distance back to the start is pure error, with no reference trajectory and no chaos in the
way. This is the figure-eight result (order 2.08 clamped and 1.06 unclamped under AZ; 2.40 and 1.03 under Heggie)
applied to thousands of orbits.

| set | rows | in repo | what it spans |
|---|---|---|---|
| Three Body Orbits atlas (OC-1, OC-2) | 3,916 with closure < 1e-9 | `2026-10-26_orbit_catalogues/` | periods, `rmin`, families |
| Hristov et al. (PL-6) | 937 at 97–180 digits | `2026-10-27_hp_ics/` (R-631) | above f64 |
| Li & Liao free-fall (LL-5, FF) | 234 + 24,582 | `2026-10-27_hp_ics_freefall/` | brake orbits, time symmetry |
| Central-symmetry, Euler | 4,860; 12,431 | `2026-10-27_hp_ics_freefall/` | symmetric families |
| Belgrade gallery (BG-1), Hudomal thesis (HU-3) | 206; 38 | `2026-10-26_*` | the Šuvakov–Dmitrašinović sequences |
| Simó choreographies | 344 | `2026-10-27_hp_ics/` | nothing above f64 (OC-1 note) |

Each orbit's floor is the f64 rounding of its stored ICs amplified along the orbit, never the catalogued closure.

### 2.2 Analytic and gate fixtures

These are the two-body radial collision, gauge invariance, the Burrau constants, the H-3 Lagrange collapse and the
`xcheck` against `reference/tb_az.py`. Each has an exact answer.

### 2.3 Maps: the best subjects

Periodic orbits test accuracy. Maps test whether a picture is physics or integrator, which is the question
Principia's renders actually raise.

| id | map | chart | needs |
|---|---|---|---|
| L-1 to L-5 | Lehto lifetime map ("Maasai shield"), homogeneity, ξ | Anosova entry / ShapeSphere at rest | M3, M4; L-2 M6 |
| V-1/V-3/V-7, V-2/V-8 | Valtonen partition, entropy map, zoom windows | as L-1 | M4; M6 for zooms |
| GG-1 | Boekholt et al. converged lifetime grid over D | Anosova entry | M3 |
| TS-1 | Tanikawa symbol-sequence maps | Anosova entry (D11) | M3 |
| AA-2 to AA-8 | Agekyan & Anosova decay times | Anosova entry | M3 |
| P-1 to P-6 | Burrau escaper rings, regular regions | BodyPlane polar rings | M3 |
| HU-1, HU-2 | return-proximity maps | SuvakovPlane (not built) | new chart |
| PA-1 | escapers near Lagrange | SideRatio (not built) | new chart |
| TR-1 | islands of regularity, ι = 0 row | BinaryPlusSingle (not built) | new chart |
| LLL-1 | stability over the mass plane | MassSimplex | IC tables not fetched |
| TS-2 | first symbols at non-zero L | new velocity prescription | new chart |
| our own | `near-field`, `far`, `deep interior`, `body2 core`, `mid-field`, `config_stability`, `config_basin`, `preset_shape_h1` | existing | now |

For each map and each occupant that survived stage 1 (§4), record:

- **Robust features**, ones that survive every working occupant: region fractions, P-5's escaper shares, outcome-class
  fractions, the boundary's box dimension, the lifetime-tail exponent (V-9).
- **Fragile features**, ones that move with the integrator. These are numerical, or chaotic at the pixel level and
  only meaningful in aggregate (BZ-1).
- **Disagreements with the paper.** Where several of our occupants agree with each other and not with the paper, the
  paper's integrator becomes the suspect, and the census says so with both numbers (rule 4).

L-5 is already a census question: Lehto et al. say halving the step only sharpens boundaries and an unregularised run
reproduces the main structures.

## 3. The axes

### 3.1 Crossed fully: integration

| axis | values now | values later |
|---|---|---|
| precision | f32, f64 | df32, qf32, dd64, mp (each when RQ-577 builds it) |
| device | CPU; GPU on lavapipe (cloud); GPU on Metal (owner's Mac) | Vulkan hardware if one is available |
| stepper | RK4, KDK leapfrog | Taylor (PL-3, PL-4, PL-9); an embedded RK pair and IAS15 (§3.2) |
| regularisation | none, AZ, Heggie, logH (TTL) | — |
| `dtau` mode | `PerStepInterval`, `FixedPerInterval`; `PerStepRemaining` as a named control only | — |
| landing | `clamp_final_step` on/off; `land_iterate` | — |
| step limit | `None`, `Predictive` over a ladder of its `f` (`step_limit_f`), `Global` (`f` scales `eta`); `AbGrowth` as a control | — |
| `eta` | a six-rung ladder per stepper | — |
| build | §6 | — |

A combination that cannot be built (logH has no coordinate transformation and no reference body, so AZ's
shared-reference flag means nothing there) is recorded with its reason and is never silently missing.

### 3.2 Tolerance and divergence

**Integration tolerance.** Every shipped stepper takes a fixed step in fictitious time, so `eta` (and the step
limit's `f`) is the accuracy knob; there is no embedded error estimate to set a tolerance on. Error-controlled
steppers join as a stepper value with a **tolerance ladder** as their axis:

- an embedded Runge–Kutta pair (RKF4(5) or Dormand–Prince 8(5,3); Heggie 1974 used RKF7(8), so H-1/H-2 can then
  compare like for like rather than as a ratio under one stepper);
- IAS15 (adaptive, tolerance-free in practice), already the drift reference in the logH comparison, promoted to a
  census column.

For these, `eta` is replaced by the tolerance, and the reading is error against tolerance (an adaptive stepper whose
error does not track its tolerance is the finding). Cost comparisons stay at matched force evaluations.

**Divergence.** Each cell records a **divergence horizon**: the time at which this cell's trajectory and a reference
separate past a threshold δ in shape-sphere chord. Three references, each its own reading:

- the same cell one `eta` rung finer (resolution horizon);
- the default occupant (occupant horizon);
- the next precision up, where it exists (precision horizon; PL-7's per-tier horizon is this at fixed occupant; R-33's
  convergence reference for the Burrau pixel to t ≈ 60).

δ is a **ladder** (1e-12 to 1e-1, one rung per decade), never one value, because a fixed threshold fails on both
sides of a distribution. The horizon curve over δ is the reading; its slope is a Lyapunov estimate to check against
B-2 (`τ_L = 4.3 ± 0.2`). Recorded on periodic orbits too, where a horizon shorter than the period means the orbit is
not closed by that cell.

**The census's own thresholds.** The verdict bands in §5 are swept over a factor-of-ten ladder each way on stage 1,
and the summary reports how many verdicts flip. A verdict that flips under a small change of its own threshold is
reported as `borderline`, not as `works` or `fails`.

### 3.3 Held at production and swept separately

These are swept only where a target reads them. Each run states their values in its sidecar.

- the escape rule (`Reference`, `Distance(r)`, `Closure`), `escape_every`, `escape_confirm`, `stop_on_escape`;
- `r_coll` and `epsilon` (canonical, fixed at `t = 0`);
- the ensemble size `E+1` and the offset scheme (Halton, `Pcg`);
- AZ's shared-reference flag (the open question of BRIEF §3);
- `refine_flagged`: **always off** for the census. It is batch-only and removes the population being measured.
- `stop_on_event`: **off** for drift readings and **on** for outcome readings. The two passes are separate, because
  termination parks a trajectory at a close approach and corrupted five conclusions in a row.

## 4. Stages

### Stage 0: show Principia works

The default occupant (Heggie × RK4) at f64 on the CPU and f32 on the GPU passes the targets the project leans on: the
collision, figure-eight order, gauge invariance, the Burrau constants, the cross-check, OC-1 on a short-period
subset, and D-2/D-4. This is the existing plan. **The census starts only once stage 0 is green.**

### Stage 1: small and sane, at M3

- **Settings:** every shipped stepper and regularisation × f32/f64 × the step-control settings in §3.1, at the
  production build.
- **Fixtures:**
  - the figure-eight;
  - the two-body collision, run once per pair, as the Heggie pair-independence test does;
  - a Burrau pixel to t = 16;
  - about 100 atlas orbits stratified by `rmin` and period, with the smallest-`rmin` rows included on purpose;
  - `config_stability` at 64², drift only.
- **Size:** roughly 2 × 2 × 4 × 6 × 6 ≈ 600 cells per fixture. Each is a single trajectory or a small grid, so hours,
  not days.
- **Output:** a verdict per cell (§5) and a survivors list for stage 2.

### Stage 2: the corpus, in the background

- **Periodic orbits:** every row in §2.1 × every stage-1 survivor.
- **Maps:** every buildable map in §2.3 × the survivors. Render at **1024²**, the shipping resolution, for anything
  read as a chord or a per-pixel ratio. A coarse grid misstated a chord ratio by 26x one way and a max by 8x the
  other.
- **Device:** the GPU columns run on lavapipe in the cloud and on Metal on the owner's Mac. They are kept as separate
  columns, never pooled.

### Stage 3: grow

- Add a precision column per RQ-577 tier as it is built, and the Taylor stepper when it lands.
- Add a chart's maps when the chart is built.
- Add the build flags (§6).
- Add a new occupant the day it ships.

## 5. What each cell records

| reading | how | what it catches |
|---|---|---|
| fitted order | endpoint slope over the `eta` ladder, never per-rung two-point estimates | a wrong landing, a wrong stepper (1.06 against 2.08) |
| closure floor | `\|state(T) − state(0)\|` against the predicted f64-rounding floor | where a tier runs out |
| drift, Cartesian and regularised | both, always | the readout's conditioning against the integration (280x on the collision) |
| `error_ratio` | max deviation, aggregated by max | damaged pixels; absolute meaning, since it is 1.0 under exact dynamics |
| `dt_max`, `ab_floored`, cap hits | from `PixelOut` | an unbounded step (the `2.209e128` case) |
| non-finite, budget-exhausted | counted separately, never discarded | undetermined, not missing |
| cost | steps and force evaluations; **never wall-clock** | the accuracy-per-work frontier |
| divergence horizon | per δ rung, against the finer rung, the default occupant and the next precision (§3.2) | when a cell stops meaning anything, and why |
| error against tolerance | adaptive steppers only | an error controller that does not control |
| outcome labels | against the default occupant, chaotic fixtures only | flips; read with the per-pixel distribution, never a count alone |
| gain map | `log10(drift_A/drift_B)` on a fixed diverging ramp | **where** a difference lives, which no aggregate can say |

**Verdicts:** `works` · `degraded` (order or floor within a stated factor of nominal) · `borderline` (flips under the §3.2 threshold sweep) · `fails` · `not applicable` ·
`not built`. The thresholds are R-71 calibrations, each set once with its reason in the census's README.

## 6. Build flags

Flags change floating-point results, so the build is an axis.

- **GPU:** FMA contraction on or off; fast-math and reassociation; the translation path (WGSL through Naga to SPIR-V
  or to MSL); Metal's math mode.
- **CPU:** `target-cpu` (with or without FMA); the opt level; `libm` (AZ calls `powf` deliberately to match NumPy's
  last ulp, and up to 28% of its per-call cost is that fidelity).

This matters most for the precision ladder. df32 and qf32 rely on error-free transforms, which fast-math or
reassociation silently destroy. PL-1's canary ("a reassociated build fails") is the seed; the census runs every cell ×
the flag set and records which results depend on the build. Flags join at stage 3, starting with the fixtures where
they can matter: the closure floors and PL-1/PL-2.

## 7. Controls and guardrails

Every one of these comes from a mistake on the record.

- **A control arm per stage.** A deliberately broken occupant (a sign-flipped derivative; the unclamped landing; no
  step limit on `config_stability`) must **fail**. If it passes, the stage has no teeth and its results are void.
- **A liveness arm per axis.** Each knob must be shown to move at least one cell. Golden matrices have been blind four
  ways before. "Balance moves 0 of 160" is a finding; "18 distinct" is not.
- **Distributions, not medians.** Per-orbit and per-pixel distributions with the moved count and the max. "No effect"
  is never concluded from an aggregate.
- **State the regime.** Every number carries its region, resolution, horizon, `eta`, and which pass (drift or
  outcome) it came from.
- **Matched work.** Accuracy comparisons are at matched force evaluations, or they say they are not.
- **Saturation checks.** Count distinct values and the modal share before reading any curve, and check that a ground
  truth is not saturated before quoting a comparison against it.
- **No writes into `results/`.** Census output goes to its own root, passed as an argument, never hardcoded.

## 8. Running it

### 8.1 Cloud CI

| job | trigger | contents |
|---|---|---|
| census-smoke | every PR touching the kernel, stepper or regularisation | stage 0, plus the default's immediate neighbours on stage-1 fixtures |
| census-stage1 | nightly | the full stage 1, CPU and lavapipe |
| census-stage2 | weekly, sharded by fixture | the corpus and the maps, CPU and lavapipe, cached |

- Linux runners are free on a public repo. A matrix job allows 256 jobs, each with a six-hour limit, so shard by
  fixture and orbit block.
- **Cache key:** kernel hash + full config hash + fixture id. Only cells whose inputs changed are rerun. When the
  kernel moves, every dependent cell is marked stale automatically, which is the scheduler corpus's staleness lesson
  built into the cache.
- **lavapipe caveat:** it is LLVM on a CPU, so it catches shader bugs, non-determinism and translation errors, but it
  is not a real GPU's arithmetic. Its column is labelled `gpu-lavapipe` and never pooled with Metal.

### 8.2 The owner's Mac

- It is registered as a self-hosted runner, labelled `metal`, running the GPU columns weekly or on demand.
- The same harness, cache and output format, so the Metal column drops into the same table.
- It only runs when the Mac is awake and online. A missed week is recorded as not run, never as a pass.

### 8.3 Output

- **One table** (Parquet, plus CSV for reading), one row per cell × fixture. Its columns: every axis value, every
  reading, the verdict, the commit, the config hash and the build flags.
- **A provenance sidecar per render**, from `overrides_vs_production`.
- **A generated summary page** per run: the verdict matrix per fixture, the survivors list, what changed since the
  last run, and gain maps for the map fixtures.
- Kept outside `results/` (proposed: `census/`), with a README stating the thresholds and their reasons.

## 9. The questions it answers

1. **Where is regularisation needed?** Closure error against `rmin` for none, AZ, Heggie and logH, over thousands of
   orbits.
2. **Where does f64 run out?** The evidence for ruling RQ-577, and for which tier is worth building first.
3. **Long horizons:** does the leapfrog's bounded error beat RK4's secular drift on the orbits and horizons Principia
   renders?
4. **Which orbits are hard for everything?** Intrinsically unstable orbits, as opposed to ones that defeat a
   particular integrator.
5. **What does accuracy cost?** The error-per-evaluation frontier per occupant, and where the step limit rather than
   `eta` sets the cost.
6. **Which map features are physics?** The structures invariant under every working occupant, against the ones that
   move.
7. **Which results depend on the build?** Per flag, per tier.

Questions 1, 3, 5 and 6 would stand as a write-up of their own.

## 10. Dependencies and blockers

| needs | for |
|---|---|
| M3 (steppers, regularisations, outcome classifier) | stage 1; most of stage 2 |
| M4 (GPU compute, raster) | the GPU columns, L-3, the 1024² maps on the GPU |
| M6 (deep zoom) | L-2, V-2/V-8's zooms, PL-11/PL-12 |
| RQ-577 (the precision ladder) | the df32/qf32/dd64/mp columns; PL-1 to PL-12 |
| new charts (SuvakovPlane, SideRatio, BinaryPlusSingle) | HU-1/HU-2, PA-1, TR-1/MK-1/SL-1 |
| missing papers (Tanikawa & Mikkola 2021; Standish 1970; Yasko & Orlov 2015; Valtonen & Karttunen 2006; Mylläri et al. 2018) | TM-1, TASK-M3-47/-49, V-11 |
| self-hosted runner registration on the Mac | the Metal column |

## 11. For the owner to rule

1. Adopt the census as proposed (DRAFT-VAL-08), with stage 0 as the precondition.
2. Agree the CI cadence in §8.1 and the Metal runner in §8.2.
3. Agree that census verdict thresholds are R-71 calibrations, set once and stated, and never gates.
4. Agree that `census/` sits outside `results/`.
5. Decide whether stage-1 results feed the RQ-577 ruling, or that ruling stays after M3 regardless (R-630).
