# Principia — parity & determinism testing (working note)

> **SUPERSEDED — this note became `principia_parity_contract.md`; read that instead.** (Doubly so since the substrate move, `principia_spike_brief.md`: the "two complete copies, TS/f64 + WGSL/f32" framing below is retired — it is now **one shared Rust kernel compiled twice** (CPU-f64, GPU-f32), so logic equality is definitional, not tested; the branch-determinism rule is the threshold table, not f32-evaluation; and the harness is a native `#[test]`, not Dawn-in-Node. All current in the contract.) The principle, tiers, and compare-before-accumulate discipline below carried over intact (plus a Tier B for integer/packed fields). **Every open question below is now answered in the contract:** Q1 → the TS pipeline ships whole as the Precision ring (`computeIC`/`computeQuad`/`stepOnce`/`decodeOnly`, contract §1); Q2 → colour is out of parity scope (golden images + property tests, §6/§7); Q3 → the aggregate statistic's exact form is pinned in the validation phase (§5/§7); Q4 → native `#[test]` in-process for sim parity (the substrate change above — no Dawn-in-Node), Playwright for visual (§6); Q5 → confirmed, shared *states* incl. synthetic (§3); Q6 → codegen has its own suite row beside parity (§6). Kept as the decision record only.

*Original header: not the contract yet — the decided principle, captured so it isn't lost, plus the open questions to settle before writing the full test doc.*

---

## The principle (decided)

The two pipelines are **identical in logic and algorithm; they differ only in arithmetic precision; precision produces bounded, checkable error only where the computation is non-amplifying.** Chaos is not an exception to this — it is the case where the identical logic *correctly* amplifies a tiny arithmetic difference into a large one. Same code, same decisions, precision as the only input variable; whether outputs stay close is physics, not correctness.

## Three assertion tiers

1. **Logic / algorithm equality — EXACT, no tolerance.** Every discrete decision is bit-equal across precisions: substep count `N_sub`, branch taken, event classification, terminal label, escaper identity, collision pair, loop structure. These are the determinism-pin quantities; a mismatch is a bug, full stop. Most of the suite lives here and has no ε.
2. **Numerical equality where non-amplifying — TIGHT tolerance.** Decode, IC quantities (`E₀`, masses, `(m,r,p)`), a single `STEP` from a shared state, short pre-divergence stretches. Checkable precisely because the computation doesn't blow up small differences. (~1e-5 relative, tier-dependent.)
3. **No pointwise assertion where amplifying — STRUCTURAL only.** Accumulated chaotic trajectories: compared as outcome class (exact), `t_end` (windowed), divergence-onset (consistent), aggregate basin geometry (within sampling noise). The pointwise gap here IS the signal; asserting it small would assert a falsehood.

## The load-bearing discipline

**Never accumulate before comparing.** Feed both pipelines the same state, compare one step — logic exact, arithmetic tight — and chaos never enters, because chaos is a property of *iteration*, not of a step. Trajectory-level agreement is then only ever structural. This single rule is what makes "two identical pipelines" a rock instead of a flaky diff. It is the divergence principle expressed as a test strategy.

## Shape of the eventual doc

The **parity & determinism test contract** — capstone that ties the five drill-down suites together under the two-level rule, specifies per-stage assertions and their tier, and defines the aggregate-survey agreement test that certifies "the picture is the same." References the drill-downs; does not repeat them. Top of the test hierarchy, not a twelfth component.

---

## Open questions — to discuss before writing

*(Parked. These are the things that need deciding first.)*

1. **Does the TS reference implement the *whole* pipeline, or only the parts worth cross-checking?** Earlier we scoped the CPU witness as small (spot-check, not full mirror). "Two complete copies" is a stronger claim. Which is it — full parallel pipeline, or reference-for-the-checkable-stages? This changes the amount of code materially.

2. **Where does the render/colour side sit?** Parity so far is about *sim*. Do we also parity-test colouring (does the TS colour path match the WGSL fragment output pixel-for-pixel), or is colour validated differently (golden images, property tests) since it has no f64/f32 correctness stakes?

3. **What is the aggregate-agreement metric, exactly?** "Basin fractions within sampling noise" needs a concrete statistic and a threshold — and it interacts with the measure-honesty rule (uniform grid, not quadtree leaves). Define it here or defer to the validation work?

4. **Test data / harness reality in TS+WebGPU.** Running WGSL kernels headless for tests (Node vs a browser-based runner; getting f32 results back off the GPU for comparison) is a real harness question. Dawn/node-webgpu? Playwright? This shapes what's even runnable in CI.

5. **The per-step test needs shared *states*, not shared ICs.** To compare one `STEP` mid-trajectory, both sides need to start from the identical intermediate state — which means the harness generates/feeds states, not just initial conditions. Confirm that's the intended mechanism (and that it doesn't require the f64 side to have produced the state — synthetic states are fine and arguably better coverage).

6. **What certifies the *generated* code (pack/unpack/catalogue)?** That's a different kind of test (codegen correctness, GPU self-test dispatch) than pipeline parity — does it fold into this doc or stay in the generation-root drill-down?

---

*Identical logic, precision the only variable, checkable error only where nothing amplifies. Compare before you accumulate. The gap under chaos is data, not error.*
