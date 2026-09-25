# Principia — what it is for

**Read this before anything else, and return to it when a decision feels arbitrary.**

**Companion:** `principia_01_pitfalls.md` records the failures that earned §4's commitments, in
enough detail to be recognised again.

Everything else in this corpus is a contract, a measurement or a brief. This file is the *why*, and
it exists because the *why* was scattered across fifty documents where it could be inferred but not
checked — and a project that has already corrected itself a dozen times needs one place that does
not move.

---

## 1. The one-sentence version

**Principia is a browsing instrument for a space nobody browses, paired with a measuring instrument
for the things browsing finds.**

Two paths. Same physics, same charts, same kernel. Different jobs, and **different standards of
truth**.

---

## 2. The two paths, and why the split is the whole design

### The interactive atlas — a hypothesis generator

The three-body initial-condition manifold is 8-dimensional and chaotic. You cannot survey it
analytically. You cannot render all of it precisely. But you **can** fly around it cheaply and
*notice things* — and noticing is the expensive part of research, not computing.

Nobody does this. Every existing result in the field comes from **targeted numerical continuation**:
pick a family, follow it. That finds what you already suspected. It cannot show you a region you had
no reason to look at.

**So the atlas's job is to be worth investigating, not to be correct.** It runs at f32, its
predictability horizon is `t ≈ 23`, and past that it is showing amplified round-off. **That is
acceptable** — as long as it is *stated*, and as long as nothing downstream mistakes a pointer for a
result.

### The prebake — a measuring instrument

Same kernel, same charts, precision instead of interactivity. Uniform tiles, no scheduler, no camera,
no frame budget. `advance to t`, return, stitch, exit.

CPU at arbitrary precision, or GPU at emulated double/double-double. Hours, not milliseconds. **This
is where results come from.**

### The IC inspector — the prebake standard at a single point

Between "browse" and "commit six hours" there has to be a middle rung, or every prebake is launched
on a hunch.

**Click a pixel. Watch that one initial condition evolve accurately, on CPU.**

It is the **prebake's trade — precision over speed — scoped to a single IC** rather than a slice.
Which is why it is specced as the *shared kernel at f64*, not a separate viewer: same code, more
digits, one trajectory. And why the closure, period and stability measurements land there naturally —
one trajectory, high precision, **no ensemble**, so representability is the only limit.

Its job is to answer *"is this region actually worth the expensive pass?"* before the expensive pass
is paid for.

### The projective microscope — addressing, not computing

Tilting the 2-plane through the 8D chart is **exploration with no truth claim**. It is a stream of
new *identities*, so the cache grows and nothing in it becomes wrong — *"tilting doesn't invalidate,
it re-addresses"*.

It costs nothing because it computes nothing new about anything already computed. Same category as
the atlas: it buys **attention**, not **truth**.

### Three rungs, one kernel, and the standard of truth rises with the cost

| | scope | precision | cost | answers |
|---|---|---|---|---|
| **atlas** (+ microscope) | the manifold | f32 | milliseconds | *where should I look?* |
| **inspector** | one IC | f64 and up | seconds | *is this worth the expensive pass?* |
| **prebake** | one slice | arbitrary | hours | *what is actually true here?* |

**Each is cheap enough to justify the next**, which is what makes it a loop rather than three
separate tools. A discovery walks up the ladder; nothing expensive is spent on a hunch.

### Why using the same kernel is load-bearing

**The thing you saw and the thing you verify are the same code.** If the atlas used a fast
approximation and the prebake a different formulation, every discovery would carry the question *"is
this real, or an artefact of the fast path?"* — and answering it would require a third
implementation.

Sharing the kernel does not make the atlas correct. It makes the atlas's *errors* the prebake's
errors, so the prebake can rule them out by precision alone.

*(The independent convergence integrator in the Precision ring exists for the remaining case: a bug
in the shared derivation itself. Shared kernel buys precision; independence must be bought
separately.)*

---

## 3. The consequence people get wrong

**The interactive path does not need to be truthful past its horizon.** It needs to be *good enough
to be worth looking at*, and *honest about where it stops*.

This is not a lowering of standards; it is the correct allocation of them. **All the work on the
refinement criterion is about making exploration efficient, not about making results correct.**
Results come from the prebake.

That changes what "good enough" means:

| | interactive atlas | prebake |
|---|---|---|
| **standard** | worth investigating | correct |
| **limited by** | the frame budget | precision |
| **refinement** | essential — the budget is finite | **switched off** — uniform is the point |
| **horizon** | `t ≈ 23` at f32 | whatever precision you paid for |
| **when wrong** | wastes attention | **invalidates a result** |

**The two are limited by different things and neither is limited by the other's constraint.** More
wall-clock never buys the atlas more truth; more precision never buys the prebake more speed.

---

## 4. Six commitments, each earned by a measured failure

These are not style. Each one is here because violating it produced a wrong answer that looked like a
right one.

### 4.1 The instrument must say where it stops knowing

`t_max = ln(1/eps)/lambda` — around 23 at f32, 52 at f64. Past it the image is amplified round-off:
stable under re-rendering, and **not a property of the system**. Rendering past the horizon is not an
error so much as a *category mistake*.

Because `lambda` varies across IC space, the horizon is a **field**, not a constant — and `lambda` is
already carried as FTLE, so it can be reported rather than assumed.

### 4.2 Every constant must be derived or absent

The recurring defect in this project, four times over: a number chosen by eye that turns out to
decide the answer.

- the **variance-ratio criterion** measured the anti-aliasing knob, not the physics
- **`dom_KE`** decided its own verdict; no value worked in more than one configuration
- **`tau_display`** sat at the **0.4th percentile** of its own distribution, making the split
  predicate true for 99.6% of quads
- calling a drift threshold a *"classifier"* rather than a *"filter"* relocated an arbitrary constant
  rather than removing it

**A quantity is admissible if it is bounded by its own achievable maximum, fixed by a conservation
law, or expressed in canonical units.** Otherwise it is a tuning knob wearing a measurement's
clothing.

### 4.3 Failure is a measurement, not missing data

Nothing is ever discarded. A trajectory that will not integrate is a *measurement outcome* — "this
could not be determined" — and it is the strongest available statement that a pixel is undetermined.

Discarding is **chaos-selective**: integration difficulty is caused by close encounters, and close
encounters are what chaos *is*. Filtering by drift filters by chaoticity, on an instrument built to
measure chaos. Measured consequence: escape fraction understated by up to **0.19**.

It also makes normalisation constants into per-pixel variables, degrades image quality monotonically
with the playhead, and violates *"a pixel is a pixel"*.

### 4.3a Do not reason about an image — reproduce it

**Four wrong theories in one investigation**, all produced by reading code: `t_end` quantisation,
8-bit output quantisation, the colour map, and chart geometry. Each was plausible and each was wrong.

A ten-minute port of the reference implementation, **with one variable changed**, settled it in a
single run. **When an image looks wrong, reproduce it with a controlled experiment.** Reading the
code tells you what it does; running it tells you what it produces, and those diverge exactly where
the bug is.

Full account: `principia_01_pitfalls.md` §1.

### 4.4 A test that cannot fail is not a test

This family has **eight** confirmed members in this project:

- a scale-invariance test that passed while **nothing terminated**
- a spread image on a linear ramp over a quantity spanning decades — *an instrument that could not
  see its own signal*
- an `L_z` error meter, **structurally undefined** for from-rest initial conditions (`0/0`)
- `alpha_E` pointed at a decorrelation question it is **mathematically incapable** of answering
- a pan sequence producing a **byte-identical tree** at every step, because the camera was never
  wired in
- `n_hot < N²` asserted under a quantile rule, where it holds **by construction**
- an overlay 97.7% one colour
- a curvature term **identically zero** on an affine chart

**The remedy is a control that proves the test can discriminate** — a deliberately sign-flipped
variant, a known-answer field, a comparison that must differ. If a check has never failed, it may not
be able to.

### 4.5 Verify against the formulation, not just the data

Repeatedly, a claim was correct about the numbers and wrong about the system:

- COM "drift" proposed as an error meter — but AZ integrates relative coordinates, so the COM is
  never integrated and `|COM| = 1.5e-16` is the **correct value of a non-dynamical quantity**
- a fabricated cancellation mechanism for an f32 failure whose real cause was a **branch cut in the
  Levi-Civita inverse**, sitting exactly on the Burrau default
- two implementations "disagreeing" over a **documented axis transpose** and a **3× window**

### 4.5a A quantity that does not converge under refinement is measuring the sampling

Escape fraction read 0.0947 → 0.2153 → 0.4423 → 0.5494 at strides 0, 32, 4, 1 — **the largest
relative step at the finest stride**. That is the wrong shape for a resolved quantity, and it was
read as "converging". The escapes were **transient**: 0 of 895 still unbound eight boundaries later.

Same signature as drift-not-falling-with-step-size, one level up. **If refining the sampling keeps
moving the answer, the answer is about the sampling.**

### 4.6 Report the negative, and the messy

Nearly every review in this project corrected something stated with more confidence than it
deserved — and **the corrections were the most valuable output**. A clean summary that hides scatter
is worse than useless here, because the next decision is made on it.

**Conclusions from small samples flip.** One field was excluded on a 1.2× effect, measured 18.8× at
more regions, then 5× at more still. Region count was load-bearing and it did mislead.

---

## 5. What the instrument is actually looking for

Not "pretty fractals". Four concrete things, in rising order of ambition:

**Where the boundaries are, and how tangled.** Basin structure, fractal dimension, the uncertainty
exponent `alpha` — which is *the same computation* as the refinement criterion. The scheduler and the
science share one number.

**Where the periodic orbits are.** The closure field goes to the integration floor on a periodic
orbit and stays O(1) elsewhere — **seven decades of contrast at f32**. Every published family was
found by continuation; this renders the manifold they live in and they appear *because they cannot
not*. Free-fall orbits start at rest, so they live in the `z_q = 0` plane the Config chart already
draws.

**How stable those orbits are.** The *width* of the closure well, not its depth: a broad soft basin
is elliptic, a thin filament hyperbolic. That needs the surrounding manifold, which is exactly what a
list of orbits cannot give you.

**What the chaos is organised around.** The stable and unstable manifolds of hyperbolic orbits *are*
the fractal boundaries. If the render shows filaments emanating from the orbits, that is the skeleton
and the chaos in one image, with the causal relationship **visible rather than asserted**.

And the case that ties it together: Szebehely & Peters (1967) found Burrau's escape *"while at the
same time finding a nearby periodic solution."* Burrau starts at rest, so it sits in the same plane
as all 12,409 known free-fall orbits. **Is Burrau on the unstable manifold of one of them?** That is
not a description of its chaos — it is the *cause*. Dimension says how tangled; the skeleton says
what it is tangled *around*.

---

## 6. How to tell if a decision is drifting

Four questions. If a design choice cannot answer them, it is probably wrong here even if it is
sensible elsewhere.

1. **Which path is this for?** Interactive is optimised for *attention*; prebake for *truth*.
   Optimising the wrong one is the commonest drift.
2. **Where does the constant come from?** If the answer is "it looked right", stop. (§4.2)
3. **Could this measurement have failed?** If not, it is not evidence. (§4.4)
4. **Does the instrument still say where it stops knowing?** A result quoted past its horizon, or a
   pixel whose failure is hidden, breaks the one thing that makes the rest trustworthy. (§4.1, §4.3)

---

## 7. Parked — deliberate post-1.0

**These are recorded so they are not forgotten AND not half-started.** Each is a real idea with a
reason to wait. Nothing here should be built before 1.0; several would be cheap to *enable* later if
one small decision is taken now, and those are flagged.

### 7.1 Extended and arbitrary precision

**Why:** the horizon `t_max = ln(1/eps)/lambda` is around **23 at f32, 52 at f64, 105 at
double-double**. The Burrau disruption established by Szebehely & Peters in 1967 sits at **t ≈ 60** —
**past both current paths.** The instrument cannot presently reach its own system's most famous
published result.

- **GPU emulated double / double-double** — double-f32 gives ~48 bits (horizon 46) and may beat
  native f64 on hardware where f64 runs at 1/32 rate. Double-double needs f64 hardware.
- **CPU true arbitrary** — MPFR-style. Not GPU-viable (heap allocation, branchy, hostile to SIMD),
  but the reference path does not need to be.

**Why it waits:** ~15× slower for double-double, ~60× for quad-double. Fine offline, not now.

**What it buys, precisely.** Not better refinement — the criterion saturates from **ensemble
decorrelation at `t ≈ 3–11`**, which is sampling geometry and precision-independent. It buys
**pixels that are TRUE at high `t`**, which is a different quantity and the one that matters for a
result. It also widens the findable periodic-orbit period band (`T < horizon`) and lowers the closure
floor from 1e-7 to 1e-32, which is the difference between *finding* orbits and *ranking* them.

> **DECIDE NOW, COSTS NOTHING:** keep the payload width a **function of the `Real` type**, never
> hardcoded f32. The kernel is already generic over `Real` with per-precision floors, so a
> `DoubleF64` impl is another row. Hardcode the width and the prebake path forks the payload later,
> which is where the real cost would land.

### 7.2 Tiled prebake

**Why:** a full slice at extended precision will not fit in VRAM, and the memory multiplier is the
**ensemble copies**, not the pixels — a tile is `W × H × (E+1)` live states, so size tiles from
`(E+1) × sizeof(Real)`.

**Why it is easy here:** tiles are **embarrassingly independent**, because `SimState` is a pure
function of `(IC, sim key, playhead t)`. No halo, no ghost cells, no communication. **Purity buys
seamless stitching for free**, and resumability with it — crash on tile 400 of 1024, restart at 400.

**The one trap:** a tile must compute its cell width from the **global** slice geometry, never its
own extent, or the Halton offsets land at a half-cell offset and the stitch has a seam nobody can
explain.

**Free benefit:** progressive output — write each tile as it completes, watch the slice fill in, and
learn at tile 3 rather than tile 1024 that something is wrong.

**Note it is the quadtree's geometry with the criterion switched OFF.** The quadtree answers *"where
is detail worth having"*; tiling answers *"the working set does not fit"*. Same subdivision, opposite
motivation, and the prebake wants uniform depth because it is a **reference**.

### 7.3 Dump tiers — and the default is not an image

**The payload dump is an existing design seam made persistent.** The render contract already
separates computing from colouring; a prebake that emits an *image* has collapsed that seam and
thrown the physics away. **The image is a view of the dump, not an alternative to it** — and a
six-hour run should be re-colourable forever.

Measured sizes, at the current widths:

| tier | 256² | 1080p | 4K |
|---|---|---|---|
| image (a view; regenerate freely) | 192 KB | 5.9 MB | 23.7 MB |
| **reduced per pixel — THE DEFAULT** | 4 MB | 127 MB | **506 MB** |
| full `SimState` per copy | 72 MB | 2.2 GB | 8.9 GB |
| trajectory, 1 copy, every step | 120 GB | 3.7 TB | **14.8 TB** |
| trajectory, all copies | 959 GB | 29.6 TB | **118 TB** |
| trajectory, subsampled 1/256 | 479 MB | 14.8 GB | 59 GB |

**The tier follows the rung** (§2): atlas → image; prebake → reduced payload; inspector → full
trajectory, which is free at one IC. Full-trajectory-per-pixel is not a fourth mode — it is **the
inspector applied to a small grid**, which is exactly the workflow: find something at 4K, crop to
256², dump everything.

> **The reduced tier's field list is a CONTRACT, not a convenience.** Anything omitted can never be
> re-coloured and re-running costs six hours. **Include fields no current shader reads** —
> `closure_step`, `first_divergence_t`, the per-arm spreads — precisely because you cannot know now
> what someone will want to colour by.

Subsampling is what makes trajectory dumps usable, with a caveat: **a subsampled trajectory cannot
resolve close encounters**, which is where the structure is. Log-spaced sampling probably beats
uniform.

### 7.4 Headless dataset generation

**Why:** the prebake **is** headless already — batch, no camera, no GUI. What ML adds is a different
*sampler*: random points across the 8D chart rather than a 2-plane. That is **strictly easier** —
no adjacency, no tiling, no stitching, embarrassingly parallel.

**And the chart is unusually well suited.** The decoder canonicalises translation, rotation and scale,
so **a model never has to learn those symmetries**; the sigmoid parametrisation makes it a bounded
hypercube, so uniform sampling is trivial. That is a cleaner dataset than raw Cartesian ICs, where
most samples are trivial rescalings of each other.

**Labels are unusually rich for multi-task work** — outcome, `t_end`, FTLE, closure, period and the
braid word on the same sample. And the periodic-orbit case has **12,409 known ground-truth solutions**
for a genuine needle-in-haystack benchmark.

> **THE PITFALL, and it is non-obvious:** past `t_max` the labels are **amplified round-off**.
> A model trains on them perfectly happily, because the noise is deterministic and reproducible —
> and learns to predict noise. **Either stay inside the horizon, or ship the trust fields as part of
> the label** (`error_ratio`, `worst_energy_drift`, the per-sample horizon from FTLE). All three are
> already computed; they just have to be in the dump.

Positioning: Payot et al., NeurIPS ML4PS 2023, is already in the reference list.

### 7.5 Regularisation-method comparison as a scientific result

**The observation:** every regularisation comparison in the literature measures **accuracy per step
on one trajectory** — Mikkola's Figure 6, Heggie's own caveat, all of it. That is the right metric
for integrating a star cluster. It is the wrong one for **rendering a field**, and nobody was
rendering fields when these methods were compared.

**Measured here:** doubling the sync-boundary re-registration count at fixed step size moves the
drift field by **0.444 decades**, against 2.5e-6 for the LC branch choice and 7.5e-5 for the
reference-body selection rule. Heggie beats AZ on **31 of 32** cases (`err>10` 3916 → 73), and
**loses precisely on `far`** — where sustained hierarchy means AZ never re-registers.

**Prior art that narrows the claim, and must be cited:** Trani et al. 2024 (arXiv 2403.03247,
*"Isles of regularity in a sea of chaos"*) already establish that numerical error causes **spurious
mixing of phase space** — they call it *numerical chaos* — underestimating the regular phase space
by ~25% while ensemble statistics stay correct to within a percent. **So "numerical error corrupts
a 3BP map" is known.** What appears new is the **attribution to re-registration count** and the
**evaluation axis** (coherence across neighbouring ICs rather than accuracy per trajectory).

**Status: parked, and deliberately.** Two web searches are not a literature review; Aarseth's book
chapter and Mikkola & Aarseth (1993) are the next reads. And the result currently rests on **one
implementation of each method, one stepper, one chart family** — a referee's first question is
whether it generalises, which is exactly what the full matrix would answer.

> **Fairness is not optional here and it is not a convention.** Comparing a GBS logH against an
> RK4 Heggie scores the *integrator*, not the *regularisation*. The AZ/Heggie result is trustworthy
> **because** both ran the same RK4 under the same step control. The harness must refuse mismatched
> arms.

### 7.6 GPU port — the only lever that changes the category

**Measured, not estimated.** A 1024² render at `t = 50` is 1.05M pixels × 8 copies ×
**~106,000 substeps** × 4 force evals ≈ **2.6 × 10¹⁴ FLOP**. The committed 725.9 s render is exactly
what 16 cores of f64 should deliver.

**So the CPU implementation is not slow because of a defect.** `rayon` parallelises the pixel loop,
the kernel is generic over `Real` so it monomorphises, there is **no `dyn` dispatch in the hot
path**, LTO is on. The work is simply large.

| lever | gain |
|---|---|
| **GPU port** | **~85×** — embarrassingly parallel, no cross-pixel communication, f32-tolerant for survey |
| f32 instead of f64 | ~2× on CPU; much more on GPU. Horizon drops 52 → 23 |
| leapfrog instead of RK4 | 4× fewer force evals — **but changes the thing being measured** |
| fewer substeps | linear; 106k steps for `t = 50` is `dt ≈ 5e-4`, and the predictive limit may allow a larger nominal |

**One free CPU win, worth taking whenever the file is next open:** `driver.rs:477-510` allocates
**seven `Vec::with_capacity(n_sync)` per trajectory** — `refs`, `tight`, `escape_flags`,
`tie_ratio`, `boundary_shapes`, `closure_hist`, `unbound_flags`. That is **~59 million heap
allocations per 1024² render**, for diagnostic history a production render never reads. Gate them
as `keep_boundary_shapes` already is.

**And for the matrix specifically: run it at 256².** Sixteen times less work — ~45 s a case against
12 minutes — and the matrix compares *methods*, not fine structure. Run the winner at 1024²
afterwards.

### 7.7 The rule for this section

**A parked idea earns its place by naming what it buys and why it waits.** If either is missing it is
not parked, it is undecided — and undecided things drift into half-built ones. The only work that
should happen on anything here before 1.0 is a **decision that costs nothing now and prevents a fork
later** — §7.1's payload genericity is currently the only one.

---

## 7.8 Sequencing — what is next, and why in this order

1. **logH experiment.** The falsification test for the re-registration mechanism, not a
   confirmation exercise. logH has *no coordinate transformation at all* — strictly stronger than
   Heggie's no-reference-body property. **If the mechanism is real, logH matches or beats Heggie.
   If it does not, the mechanism is wrong.** Same RK4, same step control, or it scores the
   integrator instead.

2. **The refinement mechanism, from scratch.** Nearly the entire corpus was measured on a
   contaminated field — `frac_hot_between` saturating at 45 values, the α distributions, the
   68,685-leaf tree statistics, the `error(B)` curves. All computed on renders carrying the wedges,
   and **wedges are exactly the high-frequency artefact that makes a structure detector saturate.**

   **What survives:** the DP ceiling machinery, `Rank::Uniform` as a baseline, labels-are-monotone
   (structural, not data-dependent), and the blocked-vs-pooled ρ correction (a methodology fix).
   **What must be re-taken:** everything else, *including* "nothing beats breadth-first" — that was
   measured on the contaminated field and is not safe to carry forward.

   That is a better position than it sounds: the instrument is built and validated; only the inputs
   need regenerating.

3. **The GUI, and actually building the thing.** §7.6 and §7.5 wait for this. The full
   implementation is the point; the matrix and the GPU port are what the full implementation makes
   cheap, not prerequisites for it.

---

## 8. Considered and rejected — with the numbers, so they are not reinvented

**Parked (§7) means *decided, deferred, with a reason to wait*. This section means *rejected, with
the cost recorded*.** The distinction matters: a parked idea will be built; one of these will not,
and the entry exists so the same reasoning does not have to be redone.

### 8.1 An 8D BVH over the manifold, instead of a per-slice quadtree

**The idea** (predates the quadtree choice): build the acceleration structure in the **latent space
itself** rather than per-slice. A node is an 8D box carrying a coarse dynamical descriptor; a slice
becomes a **query** against it. Split along the direction of greatest dynamical sensitivity rather
than axis-aligned. The appeal is real — a per-slice quadtree is discarded the moment you rotate the
plane, whereas structure discovered in the manifold could be **reused across every slice through
it**.

**Why it fails: codimension.** A basin boundary is codimension 1 — a *curve* in a 2D slice, a
**7-dimensional sheet** in the 8D manifold. Covering it at level-6 resolution:

| | cells |
|---|---|
| 2D slice (1D curve) | **64** |
| 8D manifold (7D sheet) | **6.9 × 10¹²** |

**Lazy refinement does not rescue it.** Refine only near viewed slices and the six hidden dimensions
stay at level 0 — cell width **1.0 in `z`**, across which `alpha` spans its entire range. The node's
descriptor then summarises dynamics that are completely different at its two ends: meaningless. Tilt
reuse within ±0.125 in `z` costs **262,144×** a single slice.

**Stochastic sampling does not rescue it either.** Sampling estimates *measure*, and a codimension-1
sheet has **measure zero** — uniform sampling hits it with probability zero, and importance sampling
needs to already know where it is. **The structure you want is exactly what sampling cannot find.**

**And the finding that outlives the idea:** the refinement criterion fails because Wada makes
boundary **ubiquitous** — `frac_hot_between` saturates at 45 distinct values with 75% modal, because
nearly every quad contains one, so there is nothing to prioritise. **Wada is dimension-independent.**
A space-filling codimension-1 boundary is space-filling in 8D too. So a higher-dimensional structure
inherits the *identical* saturation over 10¹² cells instead of 4096. **It does not fix the criterion;
it makes the same problem astronomically more expensive.**

*"Maybe the slice is the problem, and a higher-dimensional structure escapes it"* is a natural thing
to wonder. **The answer is no, and the reason is the dynamics rather than the representation.**

**Two things salvaged, both affordable because they stay in 2D:**

- **Anisotropic splitting** — the dominant-sensitivity idea, applied where it is cheap. A boundary
  curve approximated by axis-aligned quads is a **staircase**: four children to resolve a feature
  varying in one direction. Splitting along one axis, chosen by which direction varies most, follows
  the curve with roughly half the cells. **This is live, not parked** — it sits in the refinement
  work awaiting a costing that is already specified (*count splits whose four children immediately
  `keep`*). It is also immune to the Wada saturation, because it changes the **shape** of what is
  spent rather than **where**.
- **The `Delta_k` sensitivity probe** — dynamical disagreement along each latent direction, at
  `1 + 2d = 17` samples per point. Local, cheap, needs no hierarchy, and it answers a genuine
  scientific question: *which latent directions generate the variation*, and *does a boundary survive
  motion along the hidden ones*. That is a **Paper 2 tool**, not a scheduler one.

---

## 9. What this is not

**Not a simulator.** It does not integrate one system well; it integrates millions and asks what
varies between them. The unit of interest is the **manifold**, not the trajectory.

**Not a search.** Nothing is hunted. The manifold is rendered by a field, and the interesting things
appear because the field is near-zero on them.

**Not real-time-first.** Interactivity buys *browsing*, and browsing buys *hypotheses*. The results
come from a batch job that may run for hours. **The pretty version is the front door, not the
building.**

**And not three tools.** One kernel, three ways of looking, with the standard of truth rising as the
cost rises. Nothing in the design is there for its own sake: the atlas exists so you have somewhere
to point the inspector, the inspector exists so the prebake is never launched on a hunch, and the
prebake exists because it is the only one of the three whose output is a **result**.
