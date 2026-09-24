# Principia — toolchain spike brief (shared-source Rust engine)

*The go/no-go probe for the substrate decision. Not Principia — a throwaway spike answering one question: does a shared-source Rust kernel give clean CPU/GPU parity through a real toolchain, and survive into a browser? Native-first by risk sequencing: prove parity + branch-determinism natively (fast, no browser), then push the rust-gpu kernel through wasm + SPIR-V→WGSL and confirm it holds. Scoped to kill four unknowns and nothing else. The output is a decision, not code — delete the spike when done.*

---

>**Outcome (the spike ran — this brief is retained as the plan of record; the findings are propagated across the corpus).** rust-gpu was chosen on all four criteria (uniform Rust including the kernel; the browser SPIR-V→WGSL hop runs clean in Chrome; f64 + double-double headroom confirmed — criterion 4, which also confirms the capability the *separate, independent* high-precision reference integrator uses). **The determinism rule stated below was overturned — this is the load-bearing finding.** The f32-evaluation/quantise hypothesis (`N_sub` *defined* as the f32 evaluation, computed with f32 rounding at each stage) **forked** across backends. The cause is **inherent transcendental latitude** — a GPU `pow` and libm `powf` differ 1–3 ulp in *every* math mode (**not** FMA contraction, **not** fast-math; a controlled test ruled both out) — so no matched f32-quantise can make it bit-exact. `N_sub` is instead a **frozen-threshold bucket lookup** (comparison against build-time constants; no runtime `pow`/`sqrt`/`div`). The resulting one-line law, the six discipline rules, and the Metal-only caveat live in `principia_gpu_determinism_note.md`; every "f32-evaluation / f32-quantise" phrasing in this brief is the **hypothesis the spike rejected**, kept for the record.

---

## The decision this unblocks

Direction chosen: **whole CPU engine in Rust → wasm, thin TS GUI shell.** The wasm/JS boundary becomes the firewall as a fact of the binary (the engine cannot import the GUI — different linker). The key invariant is **CPU logic/physics impl == GPU**, made structural by a single shared kernel source: the GPU is f32-locked (WebGPU has no f64), the CPU runs f64 (or higher), and the *only* differences permitted are FP-continuous ones — which the project is unapologetically honest about.

Two candidate toolchains, and the spike's job is to decide between them:
- **rust-gpu** — uniform Rust, the physics kernel *included*, marked for SPIR-V. Cleaner for a whole-Rust engine and keeps higher CPU precision open (double-double on the same kernel). Risk: the browser path is SPIR-V→WGSL, an extra translation hop.
- **CubeCL** — `#[cube]` proc-macro DSL (a Rust *subset* island inside otherwise-plain Rust), targets wgpu natively (smoother browser), has a CPU backend, but caps at ~f64.

The discriminator is entirely **the browser hop**. Everything else favours rust-gpu's uniformity; if its SPIR-V→WGSL output won't run clean in a browser, CubeCL's wgpu-native path wins despite the dialect island. That is the one thing neither reasoning nor a chat window settles — only a real compile does.

## Invariant under test (sharper than "same source")

Shared source guarantees identical *source*, not identical *execution at branch points*. A continuous field diverging across backends is expected and honest. A **branch decision that forks** — an integer, an enum, a terminal label — is categorically worse: the CPU then sanity-checks a *different discretisation* than the GPU ran, and the physics-impl-equality invariant is technically satisfied while the executed algorithm has silently split. So verify the strong form:

> **Every branch decision (substep integer, state enum, terminal) is bit-identical across all backends; only continuous values diverge.**

## Scope — IN (build exactly this)

- **KDK leapfrog** `STEP(state, dt) -> state'` — kick-drift-kick, nothing more.
- **Pairwise Newtonian force**, 3 planar bodies, dimensionless units `G = M = I = 1`.
- **Minimal `SimState`**: the three `(r_i, p_i)`, a `state` enum `{running, collision, bounded, failed}` (escape optional — skip the three-gate detector), the substep count `N_sub` / cumulative `total_substeps`, current energy `H` (conservation sanity only). No bit-packing — that is ledger-codegen, a separate concern; the kernel writes plain struct fields.
- **Generic over `F: Float`** — one kernel, instantiated **CPU-f64** and **GPU-f32**.
- **The precision-sensitive branch** — `N_sub = min(N_max, max(1, ceil((r_sub / r_min)^γ)))`, defaults `r_sub = 0.05`, `γ = 1.5`, `N_max = 64`. This is the whole point of the branch test.
- **The determinism rule** — `N_sub` is *defined* as the f32 evaluation: the CPU computes it with f32 rounding applied at each stage (`r_min`, the quotient, the power) so it selects the GPU's integer. Physics stays f64; the *branch input* is f32-quantised. Verify this rule actually holds through two real compilers.
- **Collision terminal** `min_{i<j}‖r_i − r_j‖ < r_coll` (pick a small shared `r_coll`, e.g. `1e-2`) and **horizon** `t ≥ T ⇒ bounded` — enough to produce a terminal `state` to diff.
- **wgpu compute dispatch** (native *and* browser) over the input states — required to run on GPU at all; keep it minimal (one bind group, a states-in / states-out buffer pair).
- **One native parity `#[test]`** diffing the two instantiations, with the branch-determinism assertion.
- **Then the browser hop** — rust-gpu kernel through wasm + SPIR-V→WGSL, run in Chrome, diff against native.

## Scope — OUT (do not build; it is creep)

- Quadtree, scheduler, cache, the allocation ring.
- Payload bit-packing / the ledger codegen.
- Colour, fragment pipeline, compositor, any rendering.
- A GUI of any kind — the spike diffs numbers, it does not draw.
- Charts, decoder, encode — feed physical `(m, r, p)` in directly.
- Adaptive-substep wrapper machinery beyond the minimal `N_sub` the branch test needs.
- Deep zoom, linearised decoder, quad-local coordinates.

## The three golden inputs

1. **Equal-mass rest start** — the easy continuous case. One step should agree to ~f32 tolerance; a short multi-step run (≈100 steps) should show drift accumulate *sanely* (FP-scale, not a logic-bug jump). Baseline: if this diverges wildly, something is broken beyond precision.
2. **Near-collision state, `N_sub` swept across the `ceil()` boundary** — the branch-determinism case, and the sharpest test. Feed *identical bits* to every backend, run one step, assert the same integer `N_sub` and the same `state`. Sweep `r_min` through the values where `ceil()` flips — a fork hides exactly at those boundaries.
3. **Burrau / Pythagorean** (masses ∝ 3,4,5 at the 3-4-5 triangle, rest) — precision-hostile, collision-dominated. **Structural** comparison only (which body ejects, encounter sequence, approximate ejection time). Do **not** assert pointwise at long time — chaos forbids it.

## Pass/fail, ordered by pain-if-wrong

1. **Browser survival (make-or-break).** The rust-gpu kernel compiles to SPIR-V, translates to WGSL, runs in Chrome, and gives the *same one-step answer as native-GPU-f32*. Prove native parity **first**; the browser hop is the last step. If it fails or is miserable → finding: fall back to CubeCL's wgpu-native path.
2. **Branch determinism across all three backends.** `N_sub` and terminal `state` bit-identical across **CPU-f64-quantised, native-GPU-f32, and browser-GPU-f32-via-WGSL**. Continuous fields may diverge; branches may not. If they fork: determine whether **FMA contraction** in the branch expression is the culprit (GPU fusing `a*b+c`, CPU not), and whether the f32-quantise rule fixes it. This is the finding that most affects the design — it says whether the single-sourced quantise rule survives two real compilers.
3. **Uniform-Rust-including-kernel.** Confirm the physics function is plain Rust marked for SPIR-V (rust-gpu's promise, and the reason to prefer it for a whole-Rust engine), not a dialect island. If the kernel fights you → a point back toward CubeCL.
4. **Precision headroom.** The f64 CPU instantiation is trivial (near-certain — confirm). Double-double on the *same* kernel via `twofloat` runs (a five-minute check; a rust-gpu-specific payoff — the separate, independent high-precision reference integrator would use this).

## Deliverable — a decision, not code

A short findings note answering all four with *evidence*: does it compile, does it run in Chrome, what is the parity delta on each golden input, did any branch fork, did quantise fix it? Ending in the rule:

> **rust-gpu** if the browser SPIR-V→WGSL hop is clean — uniform Rust including the kernel is the cleaner whole-engine architecture and keeps double-double open. **CubeCL** if the browser hop is broken or miserable — accept the `#[cube]` dialect island and the ~f64 ceiling for the smoother wgpu-native browser path.

Toolchain state moves fast — check the *current* rust-gpu / CubeCL browser story rather than trusting a fixed version; part of the spike is learning where these actually are today, not where they were.

---

*One kernel, two precisions, three backends. FP diverges; logic and branches do not — verify that, or find exactly where it breaks. Native proves the physics; the browser hop proves the toolchain. The output is "rust-gpu or CubeCL, and why," backed by a parity number and a branch that either held or forked. Then delete it.*
