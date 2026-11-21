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

**Decision:** the parametrisation stays **open `[0,1]`, endpoints excluded** (no endpoint-clamping — no landing machinery), and there are **no artificial guards** against approaching degenerate configs. This is not something to *build* — it's what the open parametrisation already does (sigmoid/softmax approach the boundary as `z → ±∞`, so `[0,1]` gets arbitrarily close but, in exact arithmetic, never lands; floating point is R-417's, below). You decline two features (endpoint-landing; artificial exclusion), keeping only Path B for exact-solution validation.

**In floating point the range is closed (R-417).** The parametrisation is open in exact arithmetic, but the links reach some endpoints by their own rounding: the lower poles `α = 0` and `β = 0` where a link's output underflows, at f32 and f64, and `qₖ = ±q_max` at saturation. The upper images stay inside the bounds, since the f32 images of π/2 and π are generated rounded toward zero (dd_decoder §3.2; dd_generation_root §3.9). There is still no endpoint clamping and no artificial guard: what is reached is reached by rounding, not by landing machinery. So the Euler collinear configuration at `β = 0` is reached exactly at underflow, while `β → π` is approached from below.

Two kinds of boundary, and "approach as limit" is correct for **both**, for different reasons:
- **Singular configs** (collision `α→π/2`, `‖ρ̃‖→0` in dd_decoder §3.2's orientation, R-417; mass→0) — approaching as a limit is *physically correct*: there is no IC to land on (the ODE has no right-hand side there — a mass-zero system is a different problem; zero separation is a potential singularity). Landing would construct an *invalid* IC. Watch it degenerate (e.g. `E → −∞` as `α → π/2`); don't land. ✓
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

*Corrected 30 Oct 2026 (R-649, the owner's clarification 2 of 29 Oct 2026: "don't delete the ground-truth note,
correct it. GPU-vs-CPU agreement within a stated f32 tolerance is an INTERNAL check and stays continuous and blocking,
measured not assumed. What goes is the claim that GPU correctness follows from the CPU's.").* The CPU reference and the
parity suite below stand. What changes is the composition: the GPU's agreement with the CPU is **measured**, within
each check's stated f32 tolerance, and blocks a merge like any internal correctness test (R-650); it is not a proof
that the GPU is correct. An external target binds the tier it declares (R-649 item 2): the CPU IC inspector (b) for an
exact target, where a GPU miss is expected and not a defect; the GPU explorer (a) for a statistical target, scored
there, statistically or qualitatively only (the owner's confirmation of 30 Oct 2026, R-649: "statistical targets are
scored on tier (a), exact targets on tier (b)"). *Was:* "the GPU explorer (a) is expected to agree statistically or
qualitatively only, on the statistical targets." The heading's "inherit on GPU" is kept as the section's citation key;
read it as this paragraph says. *31 Oct 2026, R-661:* that tier (a)'s f32 statistics agree is a hypothesis Principia will
measure, and no paper establishes it.

Don't validate Burrau/figure-eight on *both* precisions independently. **Validate the *physics* on the trustworthy f64 CPU reference** (against analytic/periodic/literature), and **separately the parity suite measures GPU against CPU**, within stated f32 tolerances. *Was:* "and **separately the parity suite proves GPU==CPU**. Compose:"

> **Substrate nuance (shared source):** under the Rust-kernel substrate the "f64 CPU reference" is the *same shared kernel* run on the CPU at f64 — its value here is **precision** (f64 removes the numerical ambiguity that would muddy a ground-truth comparison), **not independence**. Because it shares source with the survey, a *shared-source logic bug* would sit in both — but the ground-truth check catches exactly that, since it compares against **external** truth (theorems, published orbits), not against the survey. Where genuine *independence* is wanted — the integration-floor falsifiability probe, a Burrau reference that cannot inherit the survey's bugs — that is the **separate, deliberately-independent high-precision convergence integrator** (Brutus-style: CPU arbitrary precision with convergence gating — raise the precision and tighten the tolerance until the result stops changing; double-double is a fast screen only, R-33; Precision ring, systems-architecture §1), *not* the shared kernel. Two references, two jobs: the shared kernel at f64 for precision-removes-ambiguity ground truth, the independent integrator for shared-bug-immune convergence checks.

> CPU correct (ground-truth suite), **and** GPU agreeing with CPU within each check's stated f32 tolerance (parity:
> an internal check, continuous and blocking, measured, not assumed). GPU correctness does **not** follow from the
> CPU's: on an external target the GPU explorer is held to its tier (a), statistical or qualitative agreement, and a
> GPU miss on an inspector-tier (b) target is expected, not a defect (R-649).

*Was:* "> CPU correct (ground-truth suite) **×** GPU==CPU (parity) **⇒** GPU correct." (replaced by R-649's
clarification 2, 29 Oct 2026).

Physics-correctness and precision-fidelity are **separate properties, separate suites**. The expensive "is it right" checks run on the f64 reference where precision removes numerical ambiguity; the GPU's agreement with that reference is measured by the parity suite, within stated f32 tolerances, and is not taken as proof of the GPU's correctness. *Was:* "separate properties, separate suites, multiplied" and "the GPU inherits correctness through parity rather than being independently validated against theory" (R-649).

---

## New demands this places on the inverse-encode contract

Written for self-round-trip (`decode(encode(x))=x` for x's the decode produces); now must additionally:
1. **Ingest foreign points** — canonicalise an *external* physical IC into the fixed gauge (CoM, orientation, scale, body order) before inverting. The canonicalisation is the same operation the reduction is built on — so this reuses *and independently tests* it.
2. **Report a residual** — `|decode(encode(x)) − x|` as the bug signal (small = faithful; large = located defect).
3. **Report conditioning** — `κ` of the encode Jacobian at the point, to distinguish "hard but representable" (ill-conditioned, link-fixable) from "fundamental."
4. An implementer building the inverse encode for the round-trip property *alone* would make none of these robust — so they must be stated, or the validation suite can't be built on it.

---

## Path B's entry, defined (TASK-M3-24)

The lower harness entry is `validation::ground_truth::path_b::inject`:

```
inject::<A>(m: [f64; 3], r: [[f64; 2]; 3], p: [[f64; 2]; 3], key: &MarchKey64) -> Result<PathB, Refused>
```

- **Input:** masses, positions and momenta in the sim's units (`G = 1`, `Σmᵢ = 1`), in the canonical gauge: centre of
  mass at the origin and at rest. The gauge is **checked, never imposed**: a start off it by more than 16 ulp of its own
  extent, a mass that is not positive, masses not summing to 1, or a value that is not finite is refused, so the march
  sees the closed form's own numbers. `I = 1` is not required (the march is scale-covariant). `A` is the occupant, a
  type; `key` carries the horizon, the macro-step and the collision radius.
- **Bypassed:** the chart (no `(u, v)`, no window), the decode `D` (no `z`, no link selection), the encode `E`, and
  canonicalisation `C`.
- **Kept:** the dispatch-time label of the start, the wrapper's march at f64 with its cadence, detectors, monitor and
  shape readout (`engine::precision::integrate_ic`, the Precision ring's own), and the shape map `n(r, m)`, read on the
  start and where the march stopped.
- **Routed here:** an exactly degenerate start. Euler's equal-mass collinear configuration with the middle body
  labelled 2 sits at the inner pair's centre of mass, `λ̃ = 0`, the `α = 0` pole: the encode reaches it only by clamping
  (`lookup_clamped`, measured: residual `1.8·10⁻⁶`, `κ = 8.2·10⁵`), so it enters by Path B. Under the periodic `β`
  link a collinear state with `λ̃ ≠ 0` is an interior point, and Path A takes it.

## The fixture set (TASK-M3-24)

`fixtures/ground_truth/fixtures.json` holds each start with its source, construction and criteria; the golden anchors
are `fixtures/anchors/anchors.json`. Every case runs at CPU-f64 through the Precision ring; each reference is
independent of the kernel's stepper (a closed form, a published value, or the RK45 reference,
`engine::precision::rk45`). Times are canonical unless a unit is named.

| fixture | tier | path | source | asserted |
|---|---|---|---|---|
| `conservation_figure_eight` | 1 | A | Noether, on `figure_eight`'s march | `\|ΔE/E\| ≤ 10⁻⁹`, `L_z` and `P` within `10⁻¹¹` of their scales, each computed here from the final state, never from the f16 latches |
| `lagrange_equilateral` | 2 | A | Lagrange 1772, masses (0.2, 0.3, 0.5), `ω² = M/a³` | CHECK 1 unclamped within `ε_phys`; the pixel's decode is `D(z)` bit for bit; the march within `10⁻⁸` (phys) of the rigid rotation at half a turn |
| `euler_collinear` | 2 | B | Euler 1767, equal masses, middle body 2, `ω² = 5m/(4a³)` | the encode clamps it; the march within `10⁻⁸` of the rigid rotation at half a turn; the shape point at the Euler point `E₂ = (1, 0, 0)` within `10⁻⁸`; a start off the gauge refused |
| `figure_eight` | 3 | A | Chenciner & Montgomery 2000, Simó's digits, `T₈ = 6.32591398`; relabelled so the middle body is in the inner pair (the published labelling is on the `α = 0` pole) | CHECK 1 unclamped within `ε_phys`; closure after the period within `10⁻⁶` (the reference's own closure from the published digits is `4.1·10⁻⁸`); the march within `10⁻⁸` of the reference at the period |
| `burrau` | 4 | A | Burrau 1913; Szebehely & Peters 1967: masses (3, 4, 5) at rest at (1, 3), (−2, −1), (1, −1) | CHECK 1 unclamped within `ε_phys`; the march ends in `escape`, the lightest body, labelled between 50 and 100 of S&P's time units (dd_validation_orbits §1.5's coarse window); the reference's escaper the same; provenance carries `r_coll` |

The anchors, `z0` and `burrau_345` (dd_integrator §5), are held as `z` and their f64 decode bit for bit; `z0` is also
checked against Lagrange's homothetic collapse, and `burrau_345` against the Burrau criterion above.

**Measured, at the first run** (TASK-M3-24's PR gives the output): every case but the Burrau label passes. Burrau's
physics is right on both trajectories, the march's and the reference's (the lightest body leaves between 60 and 65 of S&P's
units and is unbound, ten binary separations away, at 100), but the escape **label** is `bounded` at 100: the shape
chord over a window stays above `tau = 10⁻³` because the inner binary keeps turning, its envelope `≈ 0.85/d` (slope
`−0.97` against the escaper's distance `d`), the mechanism `plan/KNOWN.md` records as "escape from a binary fires late".
The label comes near 158 of S&P's units, far past §1.5's window. The anchor fails as written; it is reported, not moved: its two tests, `golden_anchors_burrau_integrator` and
`entry_paths_burrau`, are ignored until TASK-M3-34 fixes the label's lag, and `entry_paths_burrau_ejects_on_time`
holds the physics meanwhile (the lightest body unbound from a time in 60–65 of S&P's units on, on both trajectories,
and escaped at 100; at 0.05 resolution the reference's onset is 59.6, which the test's 0.5 grid reads as 60.0).

## The benchmark `r_coll` (proposed, TASK-M3-24)

**Proposed: `r_coll = 0` for the Burrau benchmark** (`validation::ground_truth::BENCHMARK_R_COLL`). Szebehely & Peters
integrated with Levi-Civita regularisation and a fifth-order Runge–Kutta method (S&P pp. 881–883, as
`docs/incoming/2026-10-27_papers/valtonen2016/README.md` records): point masses, every binary encounter regularised and
integrated through, none terminated at a separation. So no separation may end the benchmark's march: `r_coll = 0`, no
collision terminal, with the default occupant, Heggie × RK4, regularising as Levi-Civita did. The default `10⁻³` would
not be theirs: the march's least separation is `d_min = 1.85·10⁻⁴` (measured), so at the default the run would end at
that encounter as a collision. Every benchmark run carries the `r_coll` it ran at in its provenance
(`validation::ground_truth::Provenance`).

## The independent convergence reference: its protocol (TASK-M3-25)

The Brutus-style reference of "The correctness factoring" above is `validation::reference::brutus`. This section is its
definition: the ladder, the stop rule, when it runs and how a harness calls it.

**What it integrates.** Three point masses in the plane, Newton's law with any `G`, Cartesian positions and velocities
in the frame the start gives: no regularisation, no softening, no change of time. The method is Bulirsch–Stoer: Gragg's
modified midpoint over a step `H` in `n = 2, 4, 6, …` substeps, then Neville's extrapolation to `h → 0` in `h²`. A step
is accepted at the first column `k ≥ 2` whose error estimate `‖T_kk − T_k,k−1‖` is within the rung's tolerance. The
error is measured as `max_i max(|Δrᵢ|/R, |Δvᵢ|/V)`, where `R` is the largest distance of a body from the centre of mass
and `V` the largest speed relative to it, floored at `√(GM/R)`. The next step is chosen by the least work per unit time
over the columns tried. Each output time is landed on exactly.

**Independence.** The arithmetic is `dashu-float` (MIT or Apache-2.0), binary floats rounded to nearest-even at the
rung's precision, and the kernel does not depend on it. The source uses `std` and `dashu_float` only. The gate's
independence input checks both: a scan of the source for any other path, and `cargo tree` on `kernel`'s and `engine`'s
normal and build dependencies. Each check has a control: a kernel import injected into the source, and a crate the
kernel does use (`libm`) named as forbidden.

**The ladder.** A rung of `d` digits has:
- tolerance `ε = 10⁻ᵈ`;
- working precision `⌈d·log₂10⌉ + 64` bits;
- `clamp(4 + d/2, 8, 24)` extrapolation columns, so the order rises with the digits.

The rungs run `d₀, d₀ + Δ, d₀ + 2Δ, …`, up to a set number of rungs. The default is `d₀ = 12`, `Δ = 4`, up to ten
rungs. A rung whose precision cannot resolve its tolerance (fewer than `⌈d·log₂10⌉ + 8` bits) is refused, not run.

**The stop rule.** After each rung past the first, the disagreement `δ` with the rung before is measured at every output
time, at full precision, in the error's scaled measure. The ladder stops at the first `δ ≤ 10⁻ᵖ`, and the finer rung of
the two is the result. There is no result when:
- the ladder is spent without agreement;
- a rung fails (collision, step underflow, budget, or an unresolvable precision);
- a rung is no finer than the one before;
- a rung's march took exactly the steps and force evaluations of the rung before. Agreement then shows only that it was
  the same run.

A refused ladder never returns its last rung in place of a result.

**Choosing `p`.** The consumer sets the agreement digits `p` and states them: at least two more than the digits it then
reads from the result. The gate uses `p = 20`, below f64's rounding, so a ladder held at 53 bits cannot pass. The Burrau
ejection test uses `p = 6`: it reads a two-body energy's sign on a 0.5 grid.

**When it runs.** Never per pixel, never in the app. It runs:
- in CI, the gate `reference-convergence` (seconds): the Lagrange triangle over half a turn and the figure-eight over
  its period, each converged and checked against a truth that does not come from convergence, with the controls;
- in the test suite (`crates/validation/tests/brutus.rs`, seconds): RK45 and the kernel's march at f64 measured against
  it at the figure-eight's period;
- on demand, as the reference for the integration-floor probe and Burrau's ground truth. The converged Burrau run to 100
  of Szebehely & Peters' units is an ignored test, `brutus_burrau` (84 s in a debug build).

A converged result holds for its start, its output times and its `p` alone. Convergence on one start says nothing of
another's, so each start runs its own ladder.

**Harness interface.**

```
brutus::converge(start: &Start, times: &[f64], ladder: &Ladder) -> Result<Converged, NotConverged>
Start { g, m: [f64; 3], r: [[f64; 2]; 3], w: [[f64; 2]; 3], motion: Velocity | Momentum }
Converged::accepted().outputs[k] = { t, r, v, energy_error, lz_error }   // at times[k], rounded to f64
```

Every `f64` of the start is taken as the exact binary number it is. A kernel state enters through
`brutus_gate::from_ic`, which keeps its momenta and divides by the mass at the rung's precision, so a comparison with the
kernel's march starts from the kernel's own bits. A published start enters through `brutus_gate::from_published`, with
its velocities and its own `G`. A comparison with a regularised march is read at the sync boundary where the march
stops (`⌊t/Δ_sync⌋·Δ_sync`), not at the time asked for. `brutus::run` runs one rung, for a consumer that wants the
ladder's table.

**Measured, at the first run** (TASK-M3-25's PR gives the output):

| case | accepted rung | `δ` with the rung before | truth |
|---|---|---|---|
| Lagrange, half a turn | 24 digits | `2.7·10⁻²²` | `3.2·10⁻¹⁶` from the rigid rotation, the closed form's own f64 rounding |
| figure-eight, one period | 28 digits | `1.6·10⁻²³` | closure `4.102·10⁻⁸`, the published digits' |

At the figure-eight's period, measured against the reference:
- RK45 (`rtol = 10⁻¹³`): `phys` `1.8·10⁻¹²`;
- Yoshida-4's march: `3.4·10⁻¹²`;
- Heggie × RK4's march, at its boundary 14.75: `2.3·10⁻¹³`.

Burrau to 100 of Szebehely & Peters' units at `p = 6` converges at the 24-digit rung, `δ = 3.1·10⁻⁷` with the
20-digit rung (the 12- and 16-digit rungs disagree by `4.6`, chaos at work). On it the lightest body is unbound from
60.0 on, in the window 60–65, and has escaped at 100.

## Open sub-questions (settle at implementation)

*Settled by TASK-M3-24 (the sections above):* the analytic ICs (Lagrange at masses (0.2, 0.3, 0.5); Euler, by Path B);
the figure-eight first; the Burrau feature set (escaper, outcome class, the coarse window); the pinned `r_coll`; the
lower entry point; the tolerances (exact cases by closed form or reference, at f64; Burrau structurally).

## Open sub-questions (settle at implementation)

- Exact analytic ICs to encode: Lagrange (equal-mass equilateral, known ω), Euler (collinear, via Path B), plus a couple of central configurations.
- Which periodic orbits: figure-eight first (the canonical, visually striking one), then a handful from the Hudomal catalogue spanning free-group classes.
- Burrau feature-set: the specific outcome-scale features to assert (escaper identity, close-encounter sequence, approximate ejection time) — read them from Szebehely / the recomputations. **Pin `r_coll`** to match the regularisation the reference computation assumed (collision radius is user-exposed and definitional — integrator contract Part 7, collision — so a benchmark comparison must record and fix it).
- The lower harness entry point (physical `(m,r,p)` → integrator+shape) — a validation-only API, parallel to `computeIC`.
- Tolerances: exact-case tolerances (tight, f64) vs structural-case feature-matching (outcome-class exact, timing windowed).

---

*Self-consistency proves the pipeline agrees with itself; this proves it agrees with the universe. Four tiers — conservation (smoke detector) → analytic solutions (exact theorems, Lagrange/Euler) → periodic orbits (whole-pipeline, shape-sphere-visual, figure-eight) → literature benchmarks (Burrau/Lehto, structural because chaos). Two entry paths — chart-encode for generic ICs (end-to-end, canonicalise→invert→round-trip→forward), direct-physical-inject for exactly-degenerate configs (dynamics-only, chart bypassed). Everything is representable, so a failed round-trip is always a located bug (gauge/convention/conditioning), and the residual + conditioning number are the diagnostics — which also independently test the canonicalisation and build the measure-robustness registry. The parametrisation stays open and unguarded; degenerate configs are approached as limits (correct for singular ones, exact-enough for regular ones), which dissolves the can-we-chart-them question. Validate the physics on the f64 CPU; inherit GPU correctness through parity. Correct-CPU × GPU==CPU = correct-GPU.*
