# FTLE shadow storage precision — experiment (rigorous)

*Empirical test of the deferred question: can the Benettin shadow (48 B, the biggest
single chunk of `SimState` when FTLE is on) be stored at f16/bf16 to reach a ~128 B
struct, without corrupting the FTLE (finite-time Lyapunov exponent)?*

**Verdict: NO — keep the shadow at f32.** f32 tangent storage is *exact* (error 1e-6–1e-8);
f16 introduces 0.7–7% variable error; bf16 is catastrophic (9–96%). f32 stays.

## Method (rigorous version)
- **Variational (true-Jacobian) tangent equations:** the tangent vector `w` evolves by
  `w' = Df(x)·w` with the *analytic* 4N×4N Jacobian of the planar N-body vector field —
  the mathematically correct Benettin/Lyapunov method (no finite-difference reconstruction,
  no absolute-position round-trip; the tangent is stored in its natural displacement form).
- **Unsoftened dynamics** (`soft=0`), RK4, fine `dt`, so the physics is the real
  gravitational three-body problem, not a softened proxy.
- **Timestep-convergence check** on the f64 reference (dt, dt/2, dt/4) to confirm λ is a
  real converged exponent, not integration noise.
- **Controlled comparison:** arithmetic always f64; only the resident tangent `w` is
  round-tripped through the test precision each step (real IEEE `float16`/`bfloat16`).
  The integrator error is therefore **common-mode across all precisions and cancels** in
  the relative error — so the reported error IS the storage-precision effect, isolated.
- Cases: equal-mass chaotic ICs (genuine λ ≈ 1–4). Figure-eight periodic orbit attempted
  as a λ≈0 anchor but the integrator wasn't accurate enough to hold it (ref λ≠0), so it was
  used only as a relative-ordering check, not ground truth.

## Result — storage-precision effect on λ (isolated, controlled)

| Precision | seed 1 | seed 2 | seed 3 | verdict |
|---|---|---|---|---|
| **f32**  | 2e-7 | 2e-8 | 9e-6 | **exact — no meaningful error** |
| **f16**  | 3.1% | 0.7% | 7.3% | real, *variable*, too much for a λ instrument |
| **bf16** | 8.8% | 19%  | 96%  | catastrophic, sometimes collapses λ entirely |

## Conclusions
1. **f32 shadow: exact.** Confirmed with the correct variational method on real dynamics —
   storing the tangent at f32 introduces no meaningful FTLE error. **Keep f32.**
2. **f16 shadow: 0.7–7% variable error.** Real and unpredictable per-trajectory. For an
   instrument whose *output* is Lyapunov exponents, several-percent variable error on the
   headline quantity is unsuitable (λ known to ~1 sig fig in the worst case). The 24 B saved
   is not worth it. *(An earlier crude screening run — softened dynamics, finite-difference
   reconstruction, sub-resolution d0 — overstated this as 0.5–23%; the rigorous variational
   test corrects the magnitude to ~7% worst-case, but the decision is unchanged.)*
3. **bf16: catastrophic (9–96%), ruled out.** Confirms f16 > bf16 for bounded normalised
   quantities (bf16 trades mantissa for exponent range that FTLE does not need). Also not in
   web WGSL. Where reduced precision IS safe (latched display scalars `d_min, dE_max, dLz_max`),
   use **f16**, never bf16.

## Design consequences
- **Shadow stays f32.** (Sizes below updated to the current layout — this note's original
  156/108 predated the word-buffer split.) Hot `SimState` = **136 B effective (FTLE on) /
  88 B (FTLE off)**, with the cold word external at 16 B/sample (payload §0). The old ~128 B
  target is beaten outright for FTLE-off (88 B); the FTLE-on struct stays 136 B — shrinking
  it further would need an f16 shadow, which this rigorously rules out on accuracy grounds.
- **Storage FORM finding (from the screening run, still valid):** if reduced-precision shadow
  is ever revisited, it must be stored as a **displacement/tangent vector**, never an absolute
  state — absolute-f16 was ~1000× worse than displacement-f16 (the tiny separation is destroyed
  by quantisation of the large shared position). The rigorous test already uses the displacement
  form and f16 is *still* too coarse, so this is moot unless a future higher-order/better-
  conditioned tangent scheme changes it.

## Note on rigour
The exact f16 percentages from the initial screening run were unreliable (confounded by
softening + finite-difference reconstruction + a sub-resolution renorm separation). The
**decision** (f32 keep, f16/bf16 reject) is robust because it rests on a *controlled relative
comparison* with a large, consistent effect — but the magnitudes required the variational,
unsoftened, convergence-checked version to pin down honestly. f16 worst-case is ~7%, not ~23%.

*Scripts: `ftle_precision.py`, `ftle_precision2.py` (screening — instructive, magnitudes
unreliable), `ftle_rigorous.py` / `ftle_fig8.py` / `ftle_clean.py` (variational, unsoftened,
convergence-checked — the trustworthy numbers).*
