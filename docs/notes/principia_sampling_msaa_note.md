# Principia — sampling, ensemble & SSAA model

*Working note before amending the colouring drill-down and render contract. Resolves: how mixed/boundary pixels are coloured, how ensemble copies relate to SSAA, where FTLE and ensemble-spread sit, and how none of it recurses or breaks the "any render graph" guarantee. Supersedes the majority-class / entropy-desaturation machinery entirely.*

---

## The core move: colour per sample, average last

The mixed-pixel problem is **anti-aliasing, not data semantics.** Samples ≠ pixels: a quad computes `SAMPLES_PER_QUAD_AXIS²` ICs (each a full `SimState`), and at a fractal boundary disagreeing samples fall under one display pixel. The resolution:

> **Two distinct mechanisms — do not conflate them.** *Resolution* (how much manifold detail) comes from **subdividing quads** — zoom in, a quad splits into 4, real new samples at finer IC-space spacing, keeping tiles roughly matched to pixels (one sample ≈ one tile ≈ one pixel at the refinement steady state). *Anti-aliasing* (smoothing boundaries) comes from the **ensemble** — E jittered copies within each sample's **IC-space footprint**, sub-pixel SSAA, always-on regardless of the quad-to-pixel ratio. The base `N×N` grid is NOT itself a sub-pixel supersampler — sharpness is subdivision's job, not cranking N to oversample. "Footprint" is always **IC-space** — the patch of initial-condition space a sample owns — never a screen-pixel region. Where multiple samples *do* fall under one pixel (a quad zoomed in past its N before subdivision, or the ensemble copies), that is the honest SSAA direction (many real samples → one pixel colour), never one-sample-fabricated-into-many-pixels (which would smooth over the filamentary structure the instrument exists to reveal).

**Each sample is coloured independently through the full render pipeline → a render-side resolve pass averages the sample *colours* into the pixel colour.** This is SSAA. Ordering is load-bearing: `post(combine(colour(ctx), brightness(ctx)))` runs **per sample**, producing sample colours; only then are colours averaged.

> **Why SSAA, not MSAA — the distinction is load-bearing for the cost model.** Hardware **MSAA** runs the *shading* **once per pixel** and samples only coverage/depth at N sub-points — its entire reason for existing is to *avoid* per-sample shading, which is why it's cheap. **SSAA** runs the *full shading computation independently at every sub-sample* and averages — N× the work, no reuse. Principia is unambiguously the latter, and then some: each sub-sample is not just a re-shade but a **completely independent simulation** — its own initial condition, trajectory, FTLE shadow, classification, and colour through the whole pipeline. There is no shading-reuse to amortise; there is no shared anything. So the cost scales *linearly in samples* (the `2(E+1)` trajectory count) exactly because it's SSAA — calling it "MSAA" would imply a shading-amortisation that does not exist and make the tier costs look far cheaper than they are. It is arguably *beyond* image-space SSAA: standard SSAA supersamples the *rasterisation* of one scene, whereas here each sample is a different *point of IC-space* → a different *physical trajectory*. We are supersampling the **dynamical system itself** at sub-pixel resolution, not an image. (The consumer-familiar "16×" labels on the tiers are just that — labels; the mechanism is SSAA and the E count need not be a power of two — see the sampling-pattern section.)

**Category-averaging is structurally impossible** because classification→colour happens per-sample *before* any averaging. A boundary pixel that's 75% escape-samples / 25% bounded-samples renders 75/25 blended sRGB — honest spatial AA of the footprint, not an invented "class 2". Two distinct operations, never conflated:
- **coverage/colour averaging (fine, desirable):** blend the colours of the actual outcomes present, by area fraction — ordinary edge anti-aliasing.
- **category averaging (forbidden, and now impossible):** invent a new outcome *value* from a colormap position. Never happens — nothing averages data.

**Deleted:** the majority-class rule and the `C·(1 − H/ln k)` entropy-desaturation. They solved a non-problem and baked a display choice into a data occupant (the objection: fuzzing a shader choice into the data). Gone.

---

## Ensemble copies ARE the SSAA samples

The ensemble machinery (E jittered copies per grid position, deterministic offsets in the pixel's IC-space footprint, fixed by the pixel alone (R-100) — see the sampling-pattern section below; `principia_dd_integrator.md` §3.8 uses the same Halton (2,3) prefix) doubles as the SSAA sample pool. Ensemble spread answers a finite-scale question: if I take the patch of nearby ICs this pixel covers, how mixed are their outcomes? It is not the infinitesimal stretching rate (that is FTLE), and it is resolution-dependent on purpose — zoom in and the footprint shrinks, so a pixel that read as mixed may separate into clean subregions. The **same E copies** feed two consumers:

1. **Render side — colour → SSAA resolve.** Each copy colours through the active graph; the resolve averages the E+1 colours → the pixel's anti-aliased colour.
2. **Data side — outcomes → spread metric.** The copies' classified outcomes reduce to a spread/agreement scalar (`spread_event`, f16, stored in `QuadReduction`, `principia_dd_generation_root.md` §3.7; any agreement value is derived from it on the fly — R-18. `ensemble_outcome_agreement` is a retired name).

So SSAA is **free wherever ensemble is enabled**, and AA quality scales with tier exactly where it's needed (boundaries are where E goes up *and* where AA matters). Potato/Low (E=0) → one sample/pixel, no AA, fast. The two outputs never cross the waist: colours resolve on the render side (terminal, display-only, never readable as data); outcomes reduce on the data side (into a field). Same firewall as always.

---

## Uniform samples: every sample is a full, normal SimState

**Decision (supersedes the earlier "FTLE base-only" and "partial-record copy" directions):** stop special-casing. Every sample — base and all E ensemble copies — is a **full, structurally identical `SimState`**, computing every field **including its own FTLE** (its own Benettin shadow). Rationale:

- **Uniformity beats the micro-saving.** One kind of thing: a pixel is a pixel. No base-only fields, no inheritance, no overlay logic, no "which fields are per-copy" invariant, no partial records. SSAA resolve = "colour these E+1 identical-shaped structs, average." The render graph is trivially agnostic because there is genuinely nothing special about any sample.
- **FTLE wants AA too.** Earlier claim ("FTLE is smooth, needs no AA") was too glib — FTLE spikes near boundaries and close-encounters, so an FTLE colouring *has* edges. If it's a field the graph can colour by, it should anti-alias like anything else → copies carry their own FTLE.
- **The cost is linear in E and the tiers own it.** The "FTLE-per-copy ~doubles trajectory count" objection scales with E — and that is now priced *per tier* rather than bounded globally: **E is the tier's SSAA knob (free-valued, 0–15; High = 3, Extreme = 15 — memory-tiers)**. The sweet-spot tiers keep E small (2×/4× SSAA, plenty of AA for most viewing); Extreme pays `2(E+1) = 32` trajectories/pixel knowingly ("melts current top-end GPUs" is its stated character). So the trade stays flipped the same way: **keep every copy uniform at whatever E the tier sets**, rather than strip FTLE from copies to make high E cheaper. Lockstep makes each `SimState` small, so E+1 of them scales cleanly.

**Per-pixel trajectory accounting** (uniform): `(E+1)` full sims, each with its own Benettin FTLE shadow ⇒ ~`2(E+1)` trajectories — 8/pixel at High (E=3), 32/pixel at Extreme (E=15). Of these, E+1 are clean coloured samples; E+1 are their FTLE shadows (out of the colour/resolve pool — renormalisation corrupts endpoints; the "shadow is not a sample" rule holds per-sample now).

*(The partial-record optimisation — copies carry only boundary fields, overlaid on base — stays available if profiling later shows E=4 shadows hurt on weak devices. Start uniform/simple; optimise only if measured. Same "profile first" principle as the cache budget.)*

---

## No recursion: structural uniformity ≠ role uniformity

Hazard: if a copy is "just a normal pixel" and a normal pixel spawns E copies, then `E → E² → E³…`. Stopped by a **role distinction that does not touch the data/render level:**

- **Nominal sample** (base at a grid position) — *can* have an ensemble; spawns E jittered copies when `ENSEMBLE_ENABLED`.
- **Ensemble copy** — a full `SimState` (classified, coloured, FTLE'd, resolved) but a **scheduler leaf**: never spawns its own ensemble. One level deep, by construction.

**Structural uniformity (same struct, same render treatment) ≠ role uniformity (same dispatch behaviour).** A copy is a normal pixel to the render graph and the resolve (data level — where they *are* identical); a leaf to the scheduler (dispatch level). Same pattern as the FTLE shadow one level down: the shadow is "a trajectory" but not "a sample" (doesn't get coloured); the copy is "a sample" but not "a nominal sample" (doesn't spawn ensembles). **Roles are a dispatch concept, structure is a data concept; keeping them distinct makes it finite *and* preserves render-graph-agnosticism.**

Rule: `ENSEMBLE_ENABLED` applies only to nominal samples; copies dispatch with ensembles-off. One flag check, no recursion.

---

## Stability metrics: FTLE and diffusion are SimState fields; spread is a resolve-stage reduction

Auxiliary computations, and where each lives:
- **FTLE:** each sample's own Benettin shadow produces scalar `S` → the sample's `ftle` (derived at read: `S_final/(n·dt)`). Per-sample.
- **Diffusion:** running regression moments → the `diffusion` slope (derived at read). Per-sample.
- **Ensemble spread:** the footprint's E+1 samples' outcomes reduce **at the resolve stage** to a spread scalar — a *footprint* quantity, **not** a stored `SimState` field (next paragraph).

FTLE and diffusion are per-sample `SimState` fields, visualisable through any render graph directly. **Ensemble spread is different: it is NOT a per-sample field** — it is a footprint quantity *derived at the resolve stage* from a footprint's E+1 samples (the data-side twin of the SSAA colour resolve). It's consumed live for display and aggregated into the quad reduction for refinement, but it carries no `SimState` field — storing a group property on each sample is wrong-shaped and costs an f32 ×`2(E+1)`×viewport for nothing. No per-sample "is ensemble" tag either: grouping is structural (E+1 per footprint), `copy_index` marks nominal vs copy, ensemble-on/off is a quad/tier flag.

### The point-quantity vs footprint-quantity distinction (intrinsic, not an optimisation)

- **FTLE is a *point* quantity** — FTLE-of-this-exact-IC is well-defined per copy → each copy computes its own → it **anti-aliases** (copies differ, colours blend at edges). ✓
- **Ensemble spread is a *footprint* quantity** — spread-of-the-neighbourhood only exists at the neighbourhood level; a single copy has nothing to spread over. So it's **one value per nominal sample, shared by its copies for display** → spread-colouring does **not** sharp-edge anti-alias. And that's **correct**, because spread is defined over the footprint — there is nothing sub-footprint to resolve.

So "ensemble spread can't have SSAA" is not a limitation — it's what the quantity *means*. Both FTLE and spread are fields, visualised identically; they differ only in whether copies compute-their-own (point → AA) or share-the-nominal's (footprint → no AA). The distinction is intrinsic to the physics of the quantity, not a pipeline choice.

---

## The sampling pattern: coordinate-seeded deterministic offsets

**Not stochastic (live RNG), not a naive regular grid — a fixed low-discrepancy pattern.** The ensemble/SSAA offsets within a pixel's IC-space footprint are the **2-D Halton sequence in bases (2, 3)** — one base per sub-pixel axis:

```
offset(0) = (0, 0)                                                   # copy 0: the un-jittered footprint centre
offset(k) = ( radical_inverse(2, k) − ½ , radical_inverse(3, k) − ½ )  # k = copy_index = 1…E: Halton(2,3) points 1…E,
                                                                     #   centred, then scaled to the footprint (R-80)
# no per-cell rotation: copy positions are fixed by the pixel alone, which keeps recreate-from-image exact (R-100)
```

**Committed to Halton (2,3), not Sobol.** For E ≤ ~16 in two dimensions the two are near-identical in coverage quality, so the decision is made on *implementation*: Halton's radical inverse (`radical_inverse(b, k)` = reflect k's base-b digits about the point) is a few lines with **no direction-vector tables**, computes on the fly identically on CPU and GPU, and needs no state. Sobol's theoretical edge only appears at high dimensions (dozens), which is irrelevant here (2-D sub-pixel offsets). If a future need arises (e.g. jointly low-discrepancy across *more* than the 2 sub-pixel axes), revisit — but for this, Halton (2,3) is the pick. (Bases 2 and 3 are the standard first-two-primes choice; they are coprime, so the two axes don't correlate.)

- **quad address** — `(depth, tx, ty)`: the cell at refinement depth in the **slice plane's own frame**, relative to the plane anchor (`z₀`'s value at the last re-integrating event); in-plane pan and zoom change which addresses are requested, never the addresses (R-97). **No `chart_id` term** — a chart is *continuous* (`z₀, q₁, q₂, slices, links`); it cannot be reduced to an integer and does not belong in a sample's position. The offset pattern is a property of *the grid cell being rendered*: stable under pan (cells keep identity), correctly view-relative under tilt (the cell's manifold *meaning* changes, but its sampling stays fixed — sampling is a rendering op on the current grid, not a manifold-absolute one). **NOT screen position** (the cell address is in the slice plane's frame, which is already navigation-stable, R-97). Where "chart" *does* live: the **cache**, as a continuous validity signature (exact-match; change the chart → different ICs → invalidate) — the caching contract's identity/validity split. That is a separate mechanism from the sample positions; do not conflate them.
- **pixel_index_in_quad** — `(i, j)`, `0 ≤ i,j < SAMPLES_PER_QUAD_AXIS`: which nominal sample within the quad's sample grid.
- **copy_index** — `0 … E`: which of the footprint's **E+1** samples (R-80). Copy 0 is the un-jittered nominal, at the footprint centre; copies 1…E are **Halton (2,3) points 1…E**, centred (minus ½) and scaled to the footprint — *not* a hash-to-uniform and *not* a regular grid. Because they're a *sequence prefix*, **any E works** (you take points 1…E, whatever E is — this is why E need not be a power of two). At the low sample counts Principia runs (`E+1 ≤ 16`), a low-discrepancy prefix has lower variance / better sub-pixel coverage than either random or a naive grid.

**Why this is the synthesis (beats both poles):**
- **Deterministic** → reproducible (same spec → same image) and **parity-clean without a shared RNG stream**: CPU and GPU both evaluate the same fixed Halton points at the same integer `copy_index`. The earlier jitter-determinism-for-parity gap **closes for free** — there is no RNG *state* to synchronise; the positions follow from the pixel both pipelines already hold (R-100).
- **Looks random** → well-distributed, no axis-aligned lattice, so it dodges the moiré-against-regular-structure concern that motivated wanting stochastic (the "structurally honest / one less naive quantisation" property is kept).
- **Temporally stable → no shimmer.** The positions are *spatial* — fixed by the pixel and `copy_index` (R-100), never temporal — `t`/frame is excluded — so a static pixel gets the same offsets every frame. Temporal stability by construction. (Frame-seeding would reintroduce shimmer.)
- **Reproducible spread metric.** Because offsets are a function of the pixel's manifold coordinates, "the spread at this pixel" is a deterministic number, not a noisy estimate — spread stays a *measurement*, not a sample. (This was the decisive objection to live-stochastic for a research instrument.)
- **Preserves the one-pool unification** — one deterministic pattern serves both the SSAA colour resolve and the spread metric.

**Why a *fixed* low-discrepancy prefix and not live-accumulating stochastic Monte Carlo:** a pixel's colour *is* an integral over the footprint's sub-domain, and progressive stochastic sampling would converge to it unbiased — **but only under accumulation over a static scene** (the path-tracer case). Under lockstep the "scene" is *marching*: each sample is a full trajectory integrated to the current `t`, and a sample at `t` and a sample at `t+dt` are samples of *two different integrals*, so cross-frame Monte-Carlo accumulation does not apply — and adding a new stochastic sample mid-march would mean integrating a fresh trajectory `0→t` (O(t), "recomputing shitloads"), not O(1) as in a path tracer. So Principia lives permanently in the **low, non-accumulated sample-count regime**, and there a *fixed* well-distributed set beats random. The low-discrepancy sequence is what makes that fixed set well-distributed. (Determinism, reproducible spread, parity-cleanliness, and no shimmer all still hold — it's a fixed pattern; the sequence just gives it better coverage than a hashed-uniform pattern would.)

**Cautions:**
1. **Integer-domain, exact.** The Halton points are fixed constants (identical CPU/GPU by construction), indexed by the integer `copy_index`. There is **no per-cell rotation** (R-100): the same E-point prefix is used in every footprint, so copy positions are fixed by the pixel alone, which keeps recreate-from-image exact.
2. **Address the slice-plane grid cell, not screen position.** The quad address `(depth, tx, ty)` is a cell of the slice plane's own frame (R-97), not a screen coordinate — so it is already navigation-stable (pan and zoom preserve cell identity). Placing the footprints by it makes the sample pattern a property of *the render grid cell*, reproducible regardless of scroll position. When a quad splits, children are new addresses with smaller footprints → correctly finer patterns.

**Mental model:** quad address = *which cell of the slice plane's grid* (R-97); pixel index = *which sample in that cell*; copy index = *which jittered neighbour of that sample* — the offsets are copy 0 at the centre plus Halton (2,3) points 1…E, centred, indexed by copy_index (R-80) — deterministic and well-distributed, stable across time, pan, and CPU-vs-GPU; a fixed pattern that looks random — which is what a research instrument wants.

---

## Symbolic spread — the free-group word as a third resolve-stage reduction

**The word is per-copy** (each of the E+1 samples carries its own free-group word — it's a stored per-sample field, genuinely irreducible: the symbolic encounter *sequence* isn't derivable from current state). Per-copy costs `~uint4 × (E+1)` × viewport — a real memory commitment — but it *earns* the cost by enabling a divergence signal no other metric captures.

**Why it's sharper than outcome/position spread:** outcome-class and position spread measure where trajectories *end up*. The word captures the *topological route* — the sequence of close encounters. Two copies can share an outcome (both escape) and similar final positions yet have **different words** — topologically distinct paths. And symbolic divergence happens *earlier*: nearby trajectories share a word *prefix*, then a close encounter branches their words, and only *later* does that manifest as different outcomes. **The word branches before the fate differs** — so a word-based signal fires *ahead* of outcome-impurity, in the region where trajectories are starting to diverge but haven't reached different fates. It is shown and measured as a diagnostic; it is not a split input (R-99).

**Symbolic spread is a footprint quantity, derived at the resolve stage** — the *third* reduction over the grouped E+1 copies, alongside the existing two:
- colours → SSAA pixel colour (render side)
- outcomes/positions → spread scalar (data side)
- **words → symbolic-spread scalar (data side)** ← new: shared-prefix-length / edit-distance / distinct-word-count across the E+1 words

All three are per-footprint reductions at the same stage; symbolic spread joins the spread family as a new `S_word` channel — a **diagnostic, not a split input** (R-99): the split reads only policy §1's `unresolved(f)`, whose latch is per footprint. The marginal *compute* cost is low — the copies' words are accumulated anyway; this retains and reduces them rather than discarding the copies'.

**Measure once built (empirical, like truncation rate):** whether symbolic spread is *additive* to outcome-impurity — does it fire earlier/differently (theory says yes), or in the same places (then it adds no diagnostic signal, though the word still earns its place as the core symbolic-dynamics feature). Build it per-copy, add the reduction, measure the marginal signal; it stays out of the split (R-99).

---

## The two firewalls (unchanged, restated for this model)

1. **Colour vs data:** the E copies' *colours* resolve on the render side (terminal, display-only, never a data source); their *outcomes* reduce on the data side (into `ensemble_spread`). The averaged pixel colour is never read back as data. (Same waist firewall.)
2. **Shadow vs sample:** Benettin shadows (one per sample now) are ingredients of each sample's `ftle` field — not pixels, out of the colour/resolve pool, because renormalisation corrupts their endpoints. (The N+2 → per-sample generalisation of the stability-metrics rule.)

---

## Amendments this makes

- **Colouring drill-down §3.7 (mixed-pixel):** strike majority-class + entropy-desaturation; replace with colour-per-sample → render-side SSAA resolve; ensemble copies are the SSAA pool; point-vs-footprint AA distinction. (This also retires the earlier symlog-adjacent `1−H/ln k` pin — vetoed, as agreed.)
- **Render contract Part 4 (Averaging):** rewrite — averaging is of *colours* (render-side resolve), never data; category-averaging structurally impossible; note the resolve is display-only/terminal.
- **Render contract Part 1 / generation-root ledger:** ensemble spread is **derived at the resolve stage** from the footprint's E+1 samples — a first-class *visualisable* quantity (colourable live at resolve, aggregated into `QuadReduction` for refinement) but **NOT a stored per-sample `SimState` field** (wrong-shaped; costs f32 ×`2(E+1)`×viewport for nothing). FTLE is per-sample (every sample computes its own) — consistent with uniform samples. *(Landed in both docs.)*
- **Scheduler:** the `ENSEMBLE_ENABLED`-only-on-nominal-samples rule (copies are leaves) — one flag-check, the no-recursion guarantee. E is the tier's SSAA knob (free-valued 0–15; High = 3).
- **Sampling pattern (new decision):** offsets are copy 0 at the footprint centre plus **Halton (2,3) points 1…E**, centred and scaled to the footprint (R-80), indexed by `copy_index` 0…E, time excluded, **no chart_id** (chart is continuous → lives in the cache as a validity signature, not in the sample positions); no per-cell rotation — copy positions are fixed by the pixel alone (R-100). Chosen over live-stochastic Monte-Carlo because lockstep marches the scene (no cross-frame accumulation; a new sample costs O(t) not O(1)) → Principia lives in the low-sample-count regime where a fixed well-distributed set wins. Closes the jitter-determinism-for-parity gap for free. Belongs in the render/scheduler contract as the sample-offset rule.
- **Stability-metrics reconciliation:** per-pixel is now `(E+1)` uniform sims each with a Benettin shadow ≈ `2(E+1)` trajectories, superseding "N+2, one Benettin". The "shadow is never a sample" rule now applies per-sample.

---

*Colour per sample, average last — so category-averaging can't happen and boundaries anti-alias honestly. The ensemble copies you already compute for spread ARE the SSAA samples; keep E modest and every sample a full uniform SimState (own FTLE included), so the render graph sees one kind of thing. Copies are structurally pixels but scheduler-leaves — uniform where it matters, distinct where it must be, so nothing recurses. Every stability metric is just a field; FTLE anti-aliases because it's a point quantity, spread doesn't because it's a footprint quantity — and that's exactly right.*
