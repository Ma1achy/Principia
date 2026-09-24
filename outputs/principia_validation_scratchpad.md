# Principia — validation vs throughput

> **SUPERSEDED / ABSORBED — the current authorities are `principia_validation_ground_truth_note.md` (the four-tier ground-truth suite, ingestion paths, CPU-validates-GPU-inherits) and `principia_parity_contract.md`.** The CPU/GPU split below survives, with two refinements the later docs made: (1) "the CPU proof runs once, then the GPU is trusted" is now **CPU-correct × GPU==CPU ⇒ GPU-correct**, where the parity half runs as an *ongoing every-commit CI suite* (native `#[test]` in-process — the substrate change; parity contract §6, not Dawn-in-Node) — the ground-truth half is the run-once piece, the parity half is not; (2) "regularisation needed even CPU-side" is refined by the integrator contract Part 6 — **KS regularisation is v2, deferred**; v1 handles close approaches by capped substepping with the definitional `r_coll` recorded per benchmark (ground-truth note pins it per reference). Kept as the decision record for the CPU-validates / GPU-delivers framing.

*Original header: scratchpad — where reproduction of the literature actually lives, and why.*

## The split

Two jobs, two machines. Don't conflate them.

- **CPU = validate the method.** One-time proof that the symmetry reduction (8D chart) + decoder + diagnostics are *valid* — that the reduced parametrisation recovers known findings. Precision-first, small N. A correctness check, not a product feature.
- **GPU = deliver the contribution.** The survey at a scale/resolution nobody has run. Throughput-first, huge N, f32. The novelty is coverage, not per-pixel precision.

Once the targets below pass on CPU, the GPU inherits the validity and is free to be fast and approximate. The reference computation blesses the method — it is **not** an ongoing GPU certification harness.

## Why f32 on GPU is fine

- The scientific value is **survey scale + structure** — basin fractions, island prevalence, boundary geometry — not certifying any single trajectory.
- Structural / statistical features are robust to per-pixel f32 error.
- Where f32 genuinely struggles: the deepest levels near the collision manifold. Those are measure-zero sets; they don't move survey-scale conclusions.
- No one has run a survey this large. Even f32-resolution structure is new.

## CPU validation targets

| Source | Chart (where) | Diagnostic (what) | Recover |
|---|---|---|---|
| Agekian–Anosova / Lehto | rest, L=0, equal-mass config map | decay time + reversibility | fractal decay strips; forward/backward divergence (Arrow of Time) |
| Trani ("isles") | rest-start sampling | reversibility-based regularity | regular islands (~⅓ of phase space) |
| Aarseth (low priority) | Burrau neighbourhood | Lyapunov | known chaoticity |

**Pass condition:** the reduced-pipeline map reproduces the published structure. That *is* the proof the 8D reduction didn't throw anything away.

## Machinery the CPU validator exercises

- **Physical-frame (Anosova) chart** → canonicalise into 8D. Proves a non-canonical gauge lands on the same manifold as the hyperspherical-Jacobi decode.
- **Diagnostics:** decay time; reversibility (forward + reverse, phase-space return distance); Lyapunov; frequency diffusion.
- **Integration modes:** forward→event; forward + reverse; variational.
- **Escape/decay detection:** one body unbound + separated + receding.
- **Regularisation** (Levi-Civita / algorithmic): needed even CPU-side — Burrau is collision-dominated.
- **Convergence gating** (Brutus-style): specifically for the regularity question, so numerical chaos neither fakes nor erases islands.

## Not this

- Not a GPU precision race. Throughput is the point.
- Not a per-pixel oracle running behind the survey. The CPU proof runs once, up front, to establish the reduction is faithful. After that, the GPU is trusted to be fast.

---

*Reproduction is the validation. Subsumption is the argument: the known maps regenerated as dials on one instrument.*
