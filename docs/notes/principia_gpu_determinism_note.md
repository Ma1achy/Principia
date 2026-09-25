# Principia — GPU determinism & backend-compilation discipline

*A boundary contract, stated once, at the seam where a shared kernel meets two float precisions and several shader compilers. Foundational: every rule here was earned by a **measured failure** in the toolchain spike (`principia_spike_brief.md` findings), not by assertion — and each failure mode is invisible until it silently corrupts a specific result. The governing law is one sentence; the rest is the enumerated ways to obey it and the failures that prove why. Authoritative for `principia-gpu` / `principia-numerics`; the integrator dd §3.3, integrator contract Part 4/Part 1, and parity contract §Tier-B/§6 point here.*

---

## The one-line law

> **Anything a control-flow decision depends on is frozen at build time or reduced to a comparison — never left hostage to a floating-point operation the backend is free to compute its own way.**

This is the *same* law as the read-side NaN/validity rule (render contract Part 4, lowering Part 3a): there, validity is a compile-time `const` or a stored flag, never a runtime `isnan()` a fast-math compiler may optimise away. Here, a branch edge is a build-time constant, never a runtime transcendental a shader compiler may round its own way. One principle, two instances — because a control-flow decision that rides on backend-elective floating point is the shared failure mode behind both.

---

## Why branches are special (continuous divergence is fine; branch divergence is not)

The project is unapologetically honest that CPU (f64) and GPU (f32) trajectories **diverge** — that divergence *is* the science (the inspector's divergence overlay, "divergence is the observable"). So it is worth being exact about which divergence is welcome and which is corrosive, because they happen at different points and have opposite character:

- **Continuous divergence** — positions, momenta, energy, the trajectory itself drifting apart at FP scale and then exponentially under chaos, and the **outcome class** (escape/collision/bounded) it eventually decides near a basin boundary. This is *irreducible and correct*: a 1-ulp difference at t=0 is amplified by a positive Lyapunov exponent to O(1) by the horizon. Do **not** try to make it match; trying would corrupt the science. This is the honest-divergence case.
- **Branch divergence at an instantaneous decision** — `N_sub` (the substep count for *this* step), collision, horizon. These are decided *before any accumulation*, from the current state: pure functions of one bit-identical input. Chaos is nowhere near them. If they fork, it is **not** chaos and **not** precision-in-the-honest-sense — it is two library implementations of the same function disagreeing.

The reason the second kind is corrosive: **the CPU's entire job is to sanity-check the GPU by running the *same* algorithm at higher precision.** A forked `N_sub` means the two sides integrate on *different time grids* from that step on — they are no longer two precisions of one integrator, they are two different numerical schemes, and when they later disagree on outcome you can no longer attribute the disagreement to chaos rather than to the grid mismatch you injected. The branch fork **destroys the falsifiability of the CPU check**, and (for Paper 2) injects grid-noise into the very basin boundary whose fractal structure is being measured — making "is this filigree real or f32 hash noise?" unanswerable. Continuous divergence you flaunt; branch divergence you eliminate, and it happens to be nearly free to eliminate.

---

## The mechanism (measured, not inferred — the attribution was overturned by a controlled test)

The spike's as-briefed rule (`N_sub` = f32 evaluation of `⌈(r_sub/r_min)^1.5⌉`, with matched per-stage rounding CPU-side) **forked** — 5/272 boundary states. The initial guess was fast-math; a controlled test (vendored shader compiler with an env-gated safe-math switch, `pow`'s raw result bits read back per sample) **overturned it**:

- Disabling fast-math (Metal `MTLMathMode::Safe`) did **not** restore determinism — still 5/272, a *different* set. **Fast-math is a secondary perturbation, not the cause**, and "turn off fast-math" is not a fix (and is not available in a browser regardless).
- The load-bearing cause is **inherent cross-implementation transcendental latitude**: a GPU's `pow` and libm's `powf` legitimately differ by 1–3 ulp *in every math mode* (measured: `pow(4.000000477, 1.5)` → exactly `8.0` on the GPU vs `8.000001907` in libm — a 2-ulp gap landing exactly on the `ceil` boundary → `N_sub` 8 vs 9). No matched rounding can close this: each library is entitled to its own result.

**Consequence:** *no runtime transcendental feeding a branch is safe on any backend.* The fix is categorical, not numerical — remove the transcendental from the branch path entirely.

---

## The discipline (each rule = one measured failure)

1. **Branch inputs are comparison-only, against compile-time constants.** No runtime `pow`/`sqrt`/`div` decides control flow.
   - `N_sub` is a **bucket lookup**: `d²` against 64 frozen f32 thresholds `THR[n] = round_f32((r_sub/n^{2/3})²)`, as a generated comparison tree — no runtime sqrt/div/pow/loop/index. The `pow` moves to *table-generation time* (f64 libm, run once, frozen into constants); every backend then compares against the identical bit-pattern.
   - **Collision** is `d² < r_coll²` against a constant (not `r_min < r_coll` via a runtime `sqrt`).
   - **Horizon** is an **integer step counter** (not `t += dt` then `t ≥ T` — accumulated float time drifts and forks the test).

2. **The one float feeding a branch is built identically on every backend.** `d²` is `dx² + dy²` — a scrap of arithmetic a compiler may contract to an `fma` on one backend and not another, re-forking at a bucket edge. So `d²` is **position-quantised**: cast positions to f32, then lone-subtract, lone-multiply, and an **explicitly written** `fma(dy, dy, dx*dx)` (explicit `fma` forces every backend onto the identical fused op instead of choosing). Then the sole float feeding the sole branch is bit-identical everywhere.

3. **Clamp in f32 *before* any float→int cast.** Out-of-range `OpConvertFToU` is **undefined behaviour on the GPU** (Rust's `as` saturates instead) — a fork source independent of rounding. Clamp the value/index into range in f32 first. (Pre-empted from day one in the spike; never fired — remains a discipline rule, not an observed bug.)

4. **No non-finite literals.** `f32::INFINITY` (e.g. as a min-fold seed) compiles to SPIR-V but is **rejected by naga validation** ("Float literal is infinite" — WGSL cannot express it). Seed folds with the first element.

5. **Loop shape is load-bearing, not stylistic — the wgpu#4449 hazard.** Terminal/early exit is via a **flag tested in the loop condition, zero `break` statements**, and loops are `while`, never a counted `for s in 0..N_sub` with a mid-body exit. A counted loop with an interior exit lowers, through rust-gpu → SPIR-V → naga → WGSL, to a multi-level-exit shape that **miscompiles** (wgpu#4449). Relatedly, **implicit bounds checks are compiler-injected multi-level exits**: rust-gpu lowers every runtime-indexed `a[i]` to `if in_bounds {…} else {break-to-panic}`, reintroducing the same shape. Measured consequence: a **triangular nested loop silently truncated through naga→MSL — the GPU summed one body-pair of three** (H = −0.577 vs −1.732), a *wrong answer with no error and no crash*. **Fix:** constant indices / unrolled pairs where the iteration count is small (elides the checks); the flag-in-`while`-condition shape for the adaptive wrapper (spike-validated at 100 macro-steps through SPIR-V→MSL *and* SPIR-V→WGSL→Tint). This is the canonical wrapper shape in integrator contract Part 1.

6. **Explicit `fma` wherever the fused path must be identical across backends** (see rule 2). Written as `a*b + c`, one compiler contracts and another does not; written as `fma(a, b, c)`, every backend takes the fused path.

---

## The parity consequence (what shared source does and does not buy)

Single-sourcing the kernel (one Rust source, compiled to CPU-f64 and GPU-f32 — the substrate decision) makes **transcription/logic drift structurally impossible**: there is no second implementation to fall out of sync, so the parity suite's historical Tier-L "did someone update one side only" burden is **retired**. It does **not** retire the parity suite, because the *same source* still passes through a shader compiler **entitled to silently miscompile** (rule 5's truncation is the proof — a wrong answer, no crash). So the suite is **re-aimed, permanent**: it guards **backend miscompilation and branch determinism**, not transcription. It runs as a **native in-process `#[test]`** (instantiate the kernel CPU-f64, dispatch the GPU build, diff `SimState` in one process) — simpler than a cross-runtime harness, and the only thing that catches a silent miscompile. Branch words (`state`, `N_sub`, `total_substeps`, terminals) are asserted **bit-exact on identical inputs, per step**; continuous words are loose and honest (parity contract §Tier-L/§Tier-B). Along a chaotic trajectory the inputs diverge by precision, so its labels may differ across precisions — that is reported, not a bug (R-84).

**Standing caveat — one GPU backend is not enough.** The spike's bit-identity evidence is **Metal-only**: on the test machine both the native and the browser legs funnelled to the same MSL compiler, which is *why* even continuous words matched. Branch-word bit-identity across a **different naga backend + driver** (Vulkan / D3D12) is the one thing the spike could not witness on an M-series machine. The comparison-only rule is *designed* to be robust across backends (it removes the compiler's freedom), so it should hold — but "should" is not "did." Run the parity gate on a real non-Metal GPU before trusting the survey to produce Paper-2 numbers; it is a standing pre-Paper-2 action item, not a doc edit. (lavapipe, the second backend on every commit, satisfies M4's two-backend check; R-58, R-110.)

---

*One law: a branch is frozen or a comparison, never backend-elective floating point. Continuous values diverge — that is the physics, and it is honest. Branch words do not, on identical inputs — that is the fix, and it is nearly free. The transcendental lives at build time, inside a constant; the loop exits on a flag, never a break; the cast clamps first; and the parity suite, now guarding the compiler rather than the programmer, stays alive forever and on more than one backend. Every rule here cost a real, silent, measured failure to learn — which is exactly why it will hold.*
