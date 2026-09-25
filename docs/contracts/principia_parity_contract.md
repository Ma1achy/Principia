# Principia — parity & determinism test contract

*The capstone of the test hierarchy. Not a component — the rule under which the two *instantiations* of the one shared kernel (CPU-f64 reference, GPU-f32 survey — `principia_spike_brief.md`) are held equivalent, and the per-stage assignment of that rule. (Since the substrate move to a single Rust kernel compiled to both, logic equality is definitional; what remains to test is precision divergence, branch determinism, and backend miscompilation — see §6.) References the five drill-down suites; does not repeat them. Where a drill-down says "test X", this doc says "and here is the tier X is asserted at, and why."*

---

## 1. The principle

The two instantiations are **identical in logic and algorithm — one shared Rust source, compiled twice, so logic equality is structural, not tested. They differ only in arithmetic precision (and in what each backend's shader compiler does with that source). Precision produces bounded, checkable error only where the computation is non-amplifying.**

Chaos is not an exception to this — it is the case where the identical logic *correctly* amplifies a tiny arithmetic difference into a large one. Same code, same decisions, precision as the only input variable; whether outputs stay close is physics, not correctness. A parity suite that ignores this either misses real bugs (tolerance too loose to catch a logic error) or cries wolf (tolerance applied where chaos guarantees divergence). The three-tier rule below is what keeps it a rock.

**The dual instantiation is a feature, not test scaffolding.** The CPU-f64 build ships — it is the Precision ring: the hover dwell-tier, the click inspector, the offline uniform-survey certifier. (The *same* kernel run at f64 is the **precision** reference — f64 removes the numerical ambiguity that would muddy a ground-truth check — but it shares source, so it is *not* independent; the deliberately-**independent** convergence reference, shared-bug-immune, is a *separate* Brutus-style CPU arbitrary-precision integrator with convergence gating — raise the precision and tighten the tolerance until the result stops changing; double-double is a fast screen only (R-33) — validation-ground-truth §correctness-factoring, systems-architecture §1 — spike criterion 4.) Its public surface:

```
computeIC(chart, uv, simKey)      → SimState              // one IC, full f64
computeQuad(chart, quadID, simKey) → SimState[]           // a quad's samples, full f64  (was computeTile — 'tile' is retired for quadtree nodes, memory-tiers §1)
stepOnce(state, dt, params)        → state'                // one STEP, either occupant  (parity workhorse)
decodeOnly(chart, uv, simKey)      → (m, r, p, ICDescriptor)
```

The parity suite is this subsystem diffed against the headless WGSL kernel. It was going to exist regardless; the tests are therefore nearly free.

---

## 2. The three tiers

| Tier | Applies to | Assertion | Tolerance |
|---|---|---|---|
| **L — Logic / algorithm** | the **rules**, and decisions **given identical inputs** | one source + no fork on fixed inputs | none (see corrected scope below) |
| **N — Numerical, non-amplifying** | decode, ICs, one step, pre-divergence stretches | elementwise close | tight (§4) |
| **S — Structural, under amplification** | accumulated chaotic trajectories | distributional / categorical match | windowed / statistical |

### Tier L — exact, no tolerance (the bulk of the suite)

These are the determinism-pin quantities. A mismatch is a bug, full stop, every time:

- substep count `N_sub` (the frozen-threshold bucket lookup — integrator dd §3.3; achieved by the comparison-only rule, §2 Tier B)
- terminal label, and its priority resolution when several could fire (integrator dd §3.6)
- event classification: the `state` enum (the old separate `class` is merged into it — payload §2), escaper identity, collision pair (`detail`)
- feasibility branch taken (invariant charts), degenerate branch + reason
- mirror-tie choice at the deadband, coincident-pair rejection (encode dd §3.2)
- decode-mode selection, loop iteration structure

**CORRECTED SCOPE — Tier L is about the RULE, not the OUTCOME.**

The previous wording said *"branch decisions must be identical across precisions"*. **That is false and
cannot be made true.** A branch on a float comparison forks whenever the value straddles the
threshold: `d² < r_coll²` with `d²` at `0.9999999999` lands on opposite sides in f32 and f64, same rule,
different answer. **The inputs differ, so the decision differs. That is precision, it is expected, and
suppressing it would defeat the point of running on the GPU at all.**

What Tier L actually guarantees, in two parts:

1. **STRUCTURAL IDENTITY — there is ONE implementation, not two.** The kernel is generic over `Real`
   and monomorphised; the GPU build and the CPU build are **the same source**. So *"the logic is
   identical"* is not something to verify by comparing outputs — **it is true by construction, because
   there is one copy of it.** There is no GPU regularisation and a separate CPU one; there is one,
   instantiated twice. Verified by **inspection of the source**, never by agreement of results.

2. **STATELESS DETERMINISM — given IDENTICAL inputs, the same decision.** This is what the
   comparison-only rule buys, and it is genuinely testable: the spike measured **705 boundary states,
   0 forks** across f64, f32, Metal (naga→MSL) and lavapipe (naga→SPIR-V), against a control —
   runtime `ceil((r_sub/√d²)^1.5)` — that forked **105 times on Metal and 82 on lavapipe** on the same
   inputs.

**The distinction that matters:** a decision that differs *because the inputs differed* is chaos and is
reported. A decision that differs *on identical inputs* means the operation itself disagreed — two
backends computing `pow` to 1–3 ulp apart and crossing an integer boundary. **Only the second is a
bug**, and the comparison-only rule is what excludes it.

**And no backend certifies another.** The spike's control forked on **crossing sets that do not nest** —
26 shared, 57 Metal-only, 34 lavapipe-only. Every backend gets states right that the other gets wrong,
so a suite pinned to one passes while a second disagrees on inputs it never sampled. **The rule must
hold by construction; it cannot be established by testing.**

### Tier B — integer-exact *given the same branch decisions* (integer & packed fields)

> **The name was misleading and the claim was too strong.** "Binary-exact" invited the reading that
> these fields match CPU↔GPU on a real trajectory. **They do not and cannot** — they are functions of
> the trajectory, and trajectories diverge by precision (see Tier L above). What is exact is the
> **integer arithmetic given its inputs**: no rounding, no tolerance, so a mismatch *on the same
> inputs* is a logic bug rather than permitted error.

Distinct from the numerical tiers: certain payload fields are **integer or integer-packed** and must match **bit-for-bit** CPU↔GPU (they have no floating-point tolerance — they either match or there is a logic bug). Per the SimState payload spec:
- **integer step indices** `t_end_step`, `t_dmin_step`; the exact `total_substeps` u32;
- **all packed descriptor fields** (`state`, `detail`, `saturated`, `dmin_pair`, `last_symbol`) and their bit offsets;
- the **free-group word arithmetic** — the mixed-radix `W` is an *integer* computed by integer multiply-add/div-mod, so the word (and `fgw_length_raw` — incl. the 127 truncation sentinel; `fgw_retained_prefix_length` derives from it) must be bit-exact **given bit-exact crossing-detection decisions** (which are Tier-L branch decisions). Note: word *content* is only as deterministic as the crossing-detection branches feeding it — the simultaneous-both-cut tie-break must be deterministic (integrator contract) or the word can diverge on FP test order.

Tier B is exact **conditionally**: given the same branch decisions and the same integer inputs, the
result is exact — integer logic has no permitted error. **It is not a claim that the fields match
across a full trajectory**, which they will not. The f16-packed *display* scalars (`d_min`, `dE_max`, `dLz_max`) are **not** Tier B — they derive from floating-point maxima and belong to Tier N/S with the underlying quantity.

**How Tier B is *achieved* — the comparison-only rule (spike-established, supersedes the old "compute the branch in f32 on both sides" pin).** A branch decision derived from a runtime transcendental **cannot** be made bit-exact across backends: a GPU's `pow`/`sqrt`/`div` and libm's differ by 1–3 ulp *in every math mode* (not a fast-math artefact — controlled test, `principia_spike_brief.md`), so a `ceil` at an integer boundary forks. Tier B therefore holds **only because branch inputs are reduced to comparisons against compile-time constants and single-rounded arithmetic** — `N_sub` is a frozen-threshold bucket lookup, collision is `d² < r_coll²`, horizon is an integer counter, the crossing tie-break is a fixed rule (integrator dd §3.3, `principia_gpu_determinism_note.md`). Without that rule these fields are *not* bit-exact and Tier B is a false assertion; with it, the spike measured 0 forks across five instantiations (CPU-f64, CPU-f32, native-GPU, browser-GPU-via-WGSL, double-double). **The scope line, stated once and corrected.** The rule applies to the **decision given its inputs**,
not to the decision at `t = 50`:

| | guarantee | verified by |
|---|---|---|
| the **rules** — bucket table, `d² < r_coll²`, tie-breaks, packing | one source, instantiated | **inspection**, not comparison |
| a **decision given identical inputs** | no fork | the stateless test (705 states, 0 forks) |
| a **label on a real trajectory** | **none — it will differ** | reported by the inspector, not suppressed |
| everything **continuous** | Tier N/S envelope | loose tolerance, **not to be tightened** |

**A `state` that differs at the horizon is chaos**, the same as a position that differs — it is what
the pixel inspector exists to show. **A `state` that differs on identical inputs is a logic bug.** The
first is the design working; only the second is excluded.

### Tier N — tight numerical (checkable because nothing amplifies)

- **decode → `(m, r, p)`, `ICDescriptor`, `E₀`**: the whole Matter rung (decoder dd §5).
- **one `STEP` from a shared state**: the parity workhorse. Feed both occupants the identical state, compare one step. Chaos cannot enter — it is a property of *iteration*, not of a step — so this is always tight and always meaningful, for regular and chaotic pixels alike (integrator dd §5.1–5.3).
- **short pre-divergence trajectory**: up to the divergence-onset time, accumulated error is bounded; compare to a growing-but-bounded envelope.
- **encode round-trip in physical units**: T2 (encode dd §5.1).

### Tier S — structural only (asserting the pointwise gap would assert a falsehood)

A chaotic trajectory integrated to `t_end` **cannot** match pointwise across precisions, and it is not a bug — positive Lyapunov exponent means a 1-ulp difference is uncorrelated by `t_end`. So:

- **never** assert `‖state_cpu − state_gpu‖ < ε` at `t_end` on a chaotic pixel.
- **do** assert: same **outcome class** (Tier L, exact) **on non-chaotic fixtures only** — on a chaotic trajectory the label may differ across precisions (Tier B table, R-84); `t_end` within a window; **divergence-onset time** consistent between the two runs; and the **aggregate survey** agrees (§5).

The pointwise gap here *is* the reversibility/chaos signal the whole instrument exists to show. Tier S is the divergence principle stated as a test strategy.

---

## 3. The load-bearing discipline: never accumulate before comparing

**For any per-step or per-decode comparison, feed both pipelines the same input and compare immediately.** This is what makes the suite robust:

- `stepOnce` comparisons take a **shared state**, not a shared IC — so the harness feeds states, including *synthetic* ones (deep near-collision, high-substep, near-feasibility-edge) that a real trajectory reaches rarely. Synthetic states are legitimate and give better coverage; the f64 side need not have produced them.
- Trajectory-level agreement is then **only ever Tier S** — because the only place accumulation happens is a full trajectory, and full trajectories are compared structurally.

The rule in one line: *tight and elementwise up to and including a single step; structural-only beyond it.*

---

## 4. Tolerance — and the cross-backend reality

Tier-N tolerance absorbs **two** sources of difference, not one:

1. **f32 vs f64** — the intended comparison.
2. **cross-backend f32 variance** — the kernel run through native `wgpu` vs the user's browser/driver can produce slightly different f32 results for the same kernel; shader translation and driver math differ across backends.

Therefore Tier-N tolerances are **pinned empirically, not guessed**: **native in-process `wgpu` sets them** (R-85) — run the kernel there, measure the spread on the decode/step quantities, set the tolerance to comfortably cover it. Dawn CI is dropped. Real browsers are checked against those tolerances with the browser build (M8). A tolerance validated only against f64 will flake across drivers. (For a solo web artefact the browser matrix is small — Chrome stable and Safari (R-110) — but it is not zero.)

Relative tolerance, per quantity class: IC/decode ~1e-5; one-step state ~1e-5 scaled by force magnitude; monitored `E₀`/`L_z` at t=0 ~1e-6. These are starting points to be replaced by measured values.

---

## 5. The aggregate-survey agreement test (what certifies "the picture is the same")

Per-pixel Tier-S is weak on its own; the test that actually certifies the two pipelines produce the same *science* is aggregate:

- Over a **uniform grid** at fixed resolution — **not** quadtree leaves (measure honesty; scheduler contract Part 2) — compute the same region on both pipelines.
- Assert agreement on: **outcome-class fractions** (fraction escaping / bounded / collision), **boundary location** (the class-boundary set matches within a pixel or two), and where computed, **island prevalence** and **decay-time distribution shape**.
- Threshold is **sampling-noise-bounded**: the two surveys are two finite samplings of the same underlying map, so they agree to within the statistical noise of the grid resolution, not exactly.

This is the validation-scratchpad's "CPU proves the method" reframed as "CPU and GPU agree on the survey" — and it is the test that matters for a shareable artefact, because it certifies the shipped GPU picture is the true one. (The exact statistic and threshold — Q3 in the working note — is pinned in the validation phase, alongside literature reproduction.)

---

## 6. The harness

**What shared source changed, and what it did *not*.** Under the Rust-kernel substrate (`principia_spike_brief.md` findings → rust-gpu) the CPU and GPU physics are **one source** compiled twice, so **transcription drift — the historical reason Tier-L existed — is structurally impossible**: there is no second implementation to fall out of sync. That retires Tier-L's "did someone update one side and not the other" burden entirely. It does **not** retire the parity suite, and the spike proved why: the *same source* still passes through a GPU shader compiler **entitled to silently miscompile** — a triangular nested loop translated through naga→MSL summed one body-pair of three, a **wrong answer with no error and no crash**. So the suite's job is *re-aimed*, not removed: it no longer guards transcription drift (gone), it guards **backend miscompilation and branch-determinism**. This is load-bearing in CI **permanently**.

| Suite | Runner | Mechanism | Frequency |
|---|---|---|---|
| Sim parity (Tier B branch words; Tier N/S continuous) | **native `#[test]`, in-process** | instantiate the one kernel CPU-f64 and dispatch the GPU build via `wgpu` (native), read the `SimState` buffer back, diff in the same process — Tier B bit-exact, Tier N/S within envelope | every commit / CI |
| Codegen (pack/unpack/catalogue) | native `#[test]` + a **GPU self-test dispatch** | pack∘unpack round-trip on host and on-device — generation-root dd §5 | every commit |
| Aggregate survey (§5) | native, headless (no canvas) | uniform-grid region on both instantiations, statistic compared | nightly / pre-release (heavier) |
| Colour / visual | **native `wgpu` offscreen** from M1; at M8 the **Playwright** browser suite checks against the same baselines within tolerance (R-110) | golden-image diff of the fragment output (colour side stays hand-WGSL — no parity stakes); no re-baselining without a gate decision | every commit (native goldens, R-110) (out of parity scope — Q2) |

**The other suites, and the CI hardware (R-110).** Unit, property, numerical-gate and native golden suites run on every commit; benchmarks run nightly and at each milestone gate; GUI screenshots run on GUI PRs and at the gates. GPU CI is a self-hosted Apple-silicon runner (Metal), plus **lavapipe** as the second backend on every commit.

Native in-process parity is *simpler* than the old Dawn-in-Node harness (one process, one language, no buffer marshalling across a runtime boundary) and *more* necessary (it is the only thing that catches a silent shader miscompile). **Run the gate on more than one GPU backend once available** — the spike's evidence is Metal-only (both the native and browser legs funnelled to the same MSL compiler, which is *why* even continuous words matched); branch-word bit-identity across a *different* naga backend + driver (Vulkan/D3D12) is the one thing the spike could not witness on an M-series machine. lavapipe on every commit is the second backend and satisfies M4's two-backend check; a run on a **real non-Metal GPU** is the standing pre-Paper-2 action item and gates Paper 2 (R-58, R-110). Parity never touches the canvas; the canvas-bound colour path is the only thing needing a real browser, and it is not a parity test.

---

## 7. What this contract does *not* cover

- **Physics correctness (self-consistency ≠ correctness).** Parity proves the two pipelines *agree* — it does **not** prove they agree with *reality*. A consistently-wrong pipeline (a sign error, a transposed Jacobi coordinate) passes every tier: both precisions compute the same wrong value. Correctness against external truth (analytic solutions, published orbits, Burrau/Lehto benchmarks) is the **ground-truth validation suite** (`principia_validation_ground_truth_note.md`), a *separate* suite. **The two compose:** the ground-truth suite validates the *physics* on the trustworthy **f64 CPU reference** (where precision removes numerical ambiguity), and this parity contract proves **GPU == CPU** — so *correct-CPU × GPU==CPU ⇒ correct-GPU*. Physics-correctness and precision-fidelity are separate properties, separate suites, multiplied; the GPU inherits correctness through parity rather than being independently validated against theory. (The chaotic ground-truth case — Burrau — is compared at **tier-S structural features**, the same discipline this contract uses under chaos.)
- **Colour correctness** — validated by golden images + the colouring-dd property tests; no f32/f64 stakes; explicitly out of scope (§6, Q2).
- **The aggregate statistic's exact form** — deferred to validation (Q3).
- **Per-component internal logic** — lives in the five drill-down suites; this doc assigns their *tier*, it does not restate their *content*.
- **Performance** — a separate concern; parity is about correctness only.

---

## 8. Test index (drill-down suite → tier)

| Source suite | Its tests land at |
|---|---|
| decoder dd §5 | N (identities, ranges, round-trip), L (degenerate branch, canonical no-op branch) |
| generation-root dd §5 | L (disjointness, pack∘unpack exact), plus codegen harness |
| integrator dd §5 | L (substep integer, detector labels, priority), N (order-scaling, one-step, drift), S (Burrau smoke, divergence structure) |
| encode dd §5 | N (T2, scale table), L (mirror tie, coincident rejection, feasibility branch) |
| colouring dd §5 | *(none — out of parity; own golden/property suite)* |

The seam catalogue (architecture §5) remains the **integration**-test index; this contract is the **parity/unit** discipline layered under it. Every seam row is still at least one integration test; every drill-down test now also carries a tier.

---

*Identical logic, precision the only variable, checkable error only where nothing amplifies. Branches are exact on identical inputs; steps are tight; trajectories are structural; the survey agrees within sampling noise. Compare before you accumulate. The gap under chaos is the signal, not the bug — and the second pipeline was always a feature.*
