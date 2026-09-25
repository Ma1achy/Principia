# Principia — ground-truth physics validation

*The scientific spine that's been missing. Everything built so far proves the pipeline is **self-consistent** (CPU==GPU parity; fields match sources; reductions match samples). None of it proves the physics is **correct** — a consistently-wrong pipeline passes every self-consistency check (CPU and GPU agree on the same wrong number; the reduction faithfully reduces it; the colour faithfully shows it). Correctness is a different kind of test: comparison against **truth external to the system** — theorems and the published literature, which you didn't write. This is what makes Principia a scientific instrument rather than a self-consistent renderer, and it's what a paper reviewer looks for first.*

---

## Self-consistency ≠ correctness (why this is a separate suite)

A sign error in the integrator, a transposed Jacobi coordinate, masses decoded to the wrong bodies — *every* existing check still passes (parity: both precisions compute the same wrong value; cross-checks: fields match their wrong sources). Only an **external ground truth** catches it. So the validation suite compares Principia's output against answers that come from **outside** — closed-form solutions and the field's published record.

**Conservation laws are necessary, not sufficient.** Energy/momentum/`L_z` drift (the accumulators you have) catches *gross* errors — but a symplectic integrator conserves energy beautifully while integrating the *wrong Hamiltonian* if a force term is sign-flipped symmetrically. Conservation is the smoke detector, not the audit.

---

## Four tiers of ground truth (increasing in what they prove)

1. **Conservation** (have it, always-on, weakest) — `E, p, L_z` constant to within expected drift. Catches gross errors. Necessary, not sufficient.
2. **Analytic solutions** (exact, unambiguous — *do first*) — the **Lagrange** equilateral config (equal masses at triangle vertices, rigid rotation at a known rate) and the **Euler** collinear solutions. Closed-form: put in the exact IC, check the trajectory does *exactly* what the theorem says. No numerical reference to argue with — if the engine can't keep the Lagrange triangle rigid, the physics is broken, full stop. Non-chaotic, so checkable **precisely**.
3. **Known periodic orbits** (validates the *whole* pipeline, visually provable, directly in-domain) — the **figure-eight** (Chenciner–Montgomery), the Broucke–Hadjidemetriou–Hénon families, the published catalogue (Hudomal thesis). Published precise ICs → known periodic behaviour with **known signatures on the shape sphere** (the figure-eight traces a specific curve). If the shape sphere shows the figure-eight tracing its known path, that's a *visual* proof the whole decode→integrate→shape pipeline is correct — beautiful for the paper. Non-chaotic (periodic), so checkable **precisely**.
4. **Literature benchmarks** (matches the field's canonical references, reviewer-facing) — **Burrau/Pythagorean** (masses 3,4,5 at the 3-4-5 triangle; Szebehely's computation, recomputed many times, outcome agreed) and **Lehto et al.** specific numbers (the paper you build on). The "agrees with the published record" test — the shared reference everyone knows. **Chaotic**, so checkable only **structurally** (see below).

---

## The ingestion problem: known ICs are physical; the pipeline is latent

Known ICs are published in **physical coordinates** (positions/velocities, or masses + Jacobi vectors). Principia's pipeline entry is **latent `z ∈ [0,1]⁸`** (or `(s,t)` chart coords). So validating "does Principia reproduce the figure-eight" requires finding *what `z` decodes to the figure-eight's physical IC* — running the decode **backwards**. This is the `inverse_encode` path, but ingesting an **external** point (one you didn't generate), which is a **new, harder demand** than the round-trip-consistency property it was written for.

### Everything is representable in principle — so a failed round-trip is a BUG signal
The 8D reduced manifold *is* the complete space of planar three-body ICs modulo the quotiented symmetries. So there is no valid planar three-body IC *physically* outside it. Therefore **a failure to round-trip a known IC is always a located bug** — never "that IC isn't representable." The bug lives in one of:
- **Canonicalisation** — published ICs come in arbitrary frames (orientation, CoM velocity, scale, body labelling). The reduction fixed a **gauge** (CoM-centred, canonical orientation, `I=1` scale, body ordering). Ingesting an external IC must **first canonicalise it into that same gauge**, *then* invert the decode. Skip this and a figure-eight published in a rotated frame won't match — a **gauge mismatch masquerading as a physics error** (the same trap as the coordinate-flip bug). ⚠ This is the most dangerous failure: it can *round-trip fine* (decode(encode(x))=x for the wrong x) and only fail the *forward-behaviour* check, pointing at the wrong subsystem.
- **Convention translation** — a paper's Jacobi weighting / body order / scale normalisation (energy=−1 vs `I=1`) / rotating frame. Recoverable by canonicalising into your gauge *if correctly identified*; misidentified → encodes to a valid-but-wrong `z`.
- **Boundary / conditioning** — see the degenerate-config section.

**So the round-trip residual is a pure bug signal:** it should always be small for a correctly-handled known IC; when it isn't, you've found a real defect in the encode/gauge machinery. And ingesting external ICs **independently tests the canonicalisation itself** — the gauge-fixing, currently only checked by self-consistency, gets exercised on foreign points.

---

## Two entry paths (routed by interior-vs-degenerate)

**Path A — chart-encode (generic/interior known ICs; the full end-to-end test):**
```
known physical IC (paper / analytic)
  → canonicalise into Principia's fixed gauge (CoM, orientation, I=1 scale, body order)
  → invert the decode → z   (inverse_encode ingesting a foreign point)
  → CHECK 1 (round-trip): decode(z) == canonicalised IC?  residual small ⇒ faithful; large ⇒ located bug
  → run z through the GENERAL FORWARD pipeline (exactly as a user's pixel would)
  → CHECK 2 (behaviour): does the trajectory reproduce the known result?
```
This validates **encode + decode + canonicalisation + integrator + shape-projection simultaneously, as integrated, through the real pixel path** — a far stronger statement than isolated unit tests. Use for figure-eight (non-degenerate framing), generic periodic orbits, Burrau.

**Path B — direct physical inject (exactly-degenerate analytic solutions):**
For the *exactly*-degenerate configs (Euler collinear, anything at a singularity), construct the IC **directly in physical `(m,r,p)`** from the closed form and feed it to the **integrator + shape pipeline directly**, via a lower harness entry point, **bypassing the `z → decode` front end**. This validates the *dynamics* without needing to encode a degenerate config into `z` (which the chart can't cleanly reach exactly — see below). The `computeIC`-style path takes chart+uv; the harness exposes a lower entry taking physical `(m,r,p)`.

**Routing:** interior/generic ICs → Path A (end-to-end). Exactly-degenerate analytic configs → Path B (dynamics-only, chart bypassed). The chart-singularity question only ever touches the design *here*, and only as "use Path B for these" — a harness concern, not a product one.

---

## Degenerate configs: open & unguarded, approach as a limit

**Decision:** the parametrisation stays **open `[0,1]`, endpoints excluded** (no endpoint-clamping — no landing machinery), and there are **no artificial guards** against approaching degenerate configs. This is not something to *build* — it's what the open parametrisation already does (sigmoid/softmax approach the boundary as `z → ±∞`, so `[0,1]` gets arbitrarily close but never lands). You decline two features (endpoint-landing; artificial exclusion), keeping only Path B for exact-solution validation.

Two kinds of boundary, and "approach as limit" is correct for **both**, for different reasons:
- **Singular configs** (collision `α→0`; mass→0) — approaching as a limit is *physically correct*: there is no IC to land on (the ODE has no right-hand side there — a mass-zero system is a different problem; zero separation is a potential singularity). Landing would construct an *invalid* IC. Watch it degenerate (e.g. `E → −∞` as `α → 0`); don't land. ✓
- **Regular-but-boundary configs** (Euler collinear — a *valid, non-singular* state that's only at a *coordinate* boundary) — approaching to ~10⁻⁵⁰ (deep-zoom f64 machinery) is *exact enough* for any visualisation/measurement; the exact point is validated via **Path B** (direct inject) if ever needed. ✓

**This dissolves "can we construct charts to reach them?"** — under approach-as-limit + Path-B-for-exact, you never need a chart centred on a degenerate config, so the coordinate-singular-vs-intrinsic-orbifold analysis becomes **optional-interesting, not required**. (For the record: coordinate-singular configs *are* chart-reachable via a re-oriented basis + f64 `z₀` to ~50-digit residual; intrinsic orbifold singularities of the reduced manifold are reachable by *no* chart — but you need neither result, because Path B bypasses the chart for the exact cases.)

---

## Link variation as a conditioning diagnostic (not a default change)

The hard boundary/symmetric ICs are ill-conditioned partly *because the default compactification squashes them to the edge of `[0,1]`*. A different link (edge-reaching simplex map, heavier-tailed bounded map, `t^γ`) — from the **link registry you already have for measure-robustness** — might place the same IC at a comfortable interior `z`. Use this as a **diagnostic bisection**:
```
for each registry link L:  z_L = invert(canonicalised IC, L);  residual_L;  conditioning_L = κ(encode Jacobian)
  clean under some L  ⇒ representable; the difficulty was CONDITIONING (link-fixable). Report best link.
  bad under all L     ⇒ FUNDAMENTAL (true boundary / gauge bug / intrinsic singularity) — investigate.
```
The **conditioning-number-across-links/charts *is* the test** for coordinate-artefact vs intrinsic singularity — you *measure* which regime an IC is in rather than knowing in advance.

⚠ **Firewall: link variation informs the *diagnostic* and the *registry*, NOT the global default.** The compactification is a **sim-key parameter that sets the sampling measure** (provenance-recorded, because different links give different quantitative answers — fractal dimension, basin densities). Optimising the *default* link for validation-reachability would **corrupt the measure the science depends on**, for *all* ICs. So:
- **Diagnostic** (classify difficulty) — pure, changes nothing. ✓
- **Per-validation link choice** (encode a given known IC under whichever link reaches it best, forward-check under *that* link, **record it in provenance**) — legitimate: you're validating the *machinery* (which is link-parametric) under the link that best exercises the target. ✓
- **Enriching the registry** with conditioning guidance ("this link near the mass-simplex edges") — this **directly serves the measure-robustness programme** (which needs to know which links disagree where). Validation *builds* the robustness toolkit. ✓
- **Changing the default** for reachability — ✗ never; the tail (a few known ICs) must not wag the dog (the reported measure).

---

## Chaos forbids pointwise comparison (exact vs structural)

- **Non-chaotic cases (analytic, periodic — tiers 2–3):** stable/periodic, so checkable **precisely** — the Lagrange triangle stays rigid to tolerance; the figure-eight closes and repeats; the shape-sphere curve matches.
- **Chaotic benchmark (Burrau — tier 4):** you **cannot** check "matches Szebehely at t=60" pointwise — chaos diverges any method difference exponentially, so your trajectory and his differ in detail even if both are correct. Compare **robust outcome-scale features that survive chaos**: *which* body escapes, the *sequence* of close encounters, the *approximate* ejection time, the interaction topology. This is the **same discipline as the parity contract's tier-S** assertions (outcome-class / structure, not pointwise state).

---

## The correctness factoring: validate on CPU, inherit on GPU

Don't validate Burrau/figure-eight on *both* precisions independently. **Validate the *physics* on the trustworthy f64 CPU reference** (against analytic/periodic/literature), and **separately the parity suite proves GPU==CPU**. Compose:

> **Substrate nuance (shared source):** under the Rust-kernel substrate the "f64 CPU reference" is the *same shared kernel* run on the CPU at f64 — its value here is **precision** (f64 removes the numerical ambiguity that would muddy a ground-truth comparison), **not independence**. Because it shares source with the survey, a *shared-source logic bug* would sit in both — but the ground-truth check catches exactly that, since it compares against **external** truth (theorems, published orbits), not against the survey. Where genuine *independence* is wanted — the integration-floor falsifiability probe, a Burrau reference that cannot inherit the survey's bugs — that is the **separate, deliberately-independent high-precision convergence integrator** (double-double/arbitrary; Precision ring, systems-architecture §1), *not* the shared kernel. Two references, two jobs: the shared kernel at f64 for precision-removes-ambiguity ground truth, the independent integrator for shared-bug-immune convergence checks.

> CPU correct (ground-truth suite) **×** GPU==CPU (parity) **⇒** GPU correct.

Physics-correctness and precision-fidelity are **separate properties, separate suites, multiplied**. The expensive "is it right" checks run on the f64 reference where precision removes numerical ambiguity; the GPU inherits correctness through parity rather than being independently validated against theory.

---

## New demands this places on the inverse-encode contract

Written for self-round-trip (`decode(encode(x))=x` for x's the decode produces); now must additionally:
1. **Ingest foreign points** — canonicalise an *external* physical IC into the fixed gauge (CoM, orientation, scale, body order) before inverting. The canonicalisation is the same operation the reduction is built on — so this reuses *and independently tests* it.
2. **Report a residual** — `|decode(encode(x)) − x|` as the bug signal (small = faithful; large = located defect).
3. **Report conditioning** — `κ` of the encode Jacobian at the point, to distinguish "hard but representable" (ill-conditioned, link-fixable) from "fundamental."
4. An implementer building the inverse encode for the round-trip property *alone* would make none of these robust — so they must be stated, or the validation suite can't be built on it.

---

## Open sub-questions (settle at implementation)

- Exact analytic ICs to encode: Lagrange (equal-mass equilateral, known ω), Euler (collinear, via Path B), plus a couple of central configurations.
- Which periodic orbits: figure-eight first (the canonical, visually striking one), then a handful from the Hudomal catalogue spanning free-group classes.
- Burrau feature-set: the specific outcome-scale features to assert (escaper identity, close-encounter sequence, approximate ejection time) — read them from Szebehely / the recomputations. **Pin `r_coll`** to match the regularisation the reference computation assumed (collision radius is user-exposed and definitional — integrator contract Part 7, collision — so a benchmark comparison must record and fix it).
- The lower harness entry point (physical `(m,r,p)` → integrator+shape) — a validation-only API, parallel to `computeIC`.
- Tolerances: exact-case tolerances (tight, f64) vs structural-case feature-matching (outcome-class exact, timing windowed).

---

*Self-consistency proves the pipeline agrees with itself; this proves it agrees with the universe. Four tiers — conservation (smoke detector) → analytic solutions (exact theorems, Lagrange/Euler) → periodic orbits (whole-pipeline, shape-sphere-visual, figure-eight) → literature benchmarks (Burrau/Lehto, structural because chaos). Two entry paths — chart-encode for generic ICs (end-to-end, canonicalise→invert→round-trip→forward), direct-physical-inject for exactly-degenerate configs (dynamics-only, chart bypassed). Everything is representable, so a failed round-trip is always a located bug (gauge/convention/conditioning), and the residual + conditioning number are the diagnostics — which also independently test the canonicalisation and build the measure-robustness registry. The parametrisation stays open and unguarded; degenerate configs are approached as limits (correct for singular ones, exact-enough for regular ones), which dissolves the can-we-chart-them question. Validate the physics on the f64 CPU; inherit GPU correctness through parity. Correct-CPU × GPU==CPU = correct-GPU.*
