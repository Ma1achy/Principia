# Principia — core design decisions

*The seams and the CPU/GPU contract. This is the thing to hand a code agent — not the big spec.*

## Principle: divergence is the observable

The pixel inspector (click any pixel → trajectories in shape + real space) is a **day-one shipping feature**, not a validation bolt-on. It is the higher-fidelity witness.

- **CPU inspector** = f64 (or higher — double-double on the *same* kernel, spike criterion 4), adaptive stepping, high fidelity. **GPU survey** = f32, fixed step, throughput.
- **Do not force them to agree — on *continuous* values.** Trajectories, positions, energy: they diverge at FP scale and then exponentially under chaos, and the gap *is* the interesting quantity. Trying to reconcile an f32 kernel with an f64 witness hides the exact thing worth seeing.
- The inspector can run f64, run the f32 GPU path on demand (a single-IC GPU trace — spike criterion 1), or **overlay/diff both**. Divergence time + phase-space separation growth is a *free chaos diagnostic*: immediate divergence → near a boundary; long agreement → regular region.
- The one invariant to protect: the **physics *definition* is identical** on both sides — and under the shared-source substrate (Rust kernel, one source compiled to both) this is *structural*, not a discipline. Legitimate differences are purely numerical. **But "numerical" splits in two:** *continuous* values may diverge (that is the honest signal); ***branch decisions* — `N_sub`, collision, terminal — must NOT**, because a forked branch means the two sides integrate on *different grids* and the CPU is no longer checking the *same* computation the GPU ran. So branches are held bit-identical by the comparison-only rule (`principia_gpu_determinism_note.md`), while everything continuous is left to diverge. Numerics is the sole independent variable *for continuous quantities*; branches have no independent variable at all.

## Seams — draw these hard. Each subsystem: one job, one input type.

### 1. Physics defined once, compiled twice — now *structurally*, not by discipline
EoM, regularisation, event detection, decoder: **one Rust source**, compiled by rust-gpu to f32/SPIR-V (GPU) and by rustc to f64 — or double-double (CPU). Under the substrate decision (`principia_spike_brief.md`) a change is not merely "made in one place" — there *is* only one place, so **logic/transcription drift is impossible, not prevented**. The residual CPU/GPU difference is (a) precision (the honest signal), and (b) what each backend's shader compiler does with the shared source — which is *not* nothing (a compiler may silently miscompile; parity §6 re-aims the suite at exactly this). So the seam's value shifts from "prevent drift" (solved structurally) to "catch backend miscompilation and hold branches deterministic."

### 2. Authoring layer lowers to specialised kernels — by monomorphisation
The `Chart` / axis-kind abstraction (raw, derived-in-block, invariant, curve) is a **CPU authoring layer**. It **lowers** to a concrete decode+map per chart — a **monomorphised Rust variant** (a chart is a type parameter the compiler specialises), **never a runtime axis-kind interpreter**. Under the substrate this is served *better* than the old "compiled WGSL variant or `switch(chart_type)`" framing: monomorphisation *is* specialisation, and there is no WGSL interpreter to avoid on the compute side because the compute side is Rust→SPIR-V (lowering Part 2). Adding a chart = a lowering rule + a kernel type/branch, not reconfiguring an engine. This is the decision that most prevents the previous failure (shader trying to do too much).

### 3. Kernel eats chart params, decodes in-kernel
Kernel input = the chart description in a uniform: `z₀`, basis vectors, chart id + params. Per pixel: **map → z → decode → (m, r, p) → integrate**, in the compiled Rust kernel (→ SPIR-V on the GPU, native on the CPU). *Not* a pre-decoded IC buffer — pan/zoom/tilt change every pixel's IC every frame, so precompute is impossible at scale. Per-frame CPU cost stays O(1): write a uniform, dispatch.

### 4. Integrate and colour are separate passes — and now separate *mechanisms*
Integration maintains a **typed per-sample state** (`SimState`: the state enum + live accumulators — the marching state at the playhead, O(1) in time; lockstep, temporal note). Colouring is a second pass over that buffer, at any playhead. The split is now doubled: **compute is monomorphised Rust (parity-critical, single-sourced); colour is hand-WGSL (parity-free, runtime devkit)** — different mechanisms meeting only at the payload (lowering Part 2).
- Switch diagnostic / recolour without re-integrating.
- **Debug shaders come free**: a debug view is just second-pass (WGSL) colouring of a different field — z components, decoded struct fields, quad state, quadtree depth. "Colour is data" becomes structural, not a slogan.
- The CPU trajectory diff is the *same pattern*: the **same kernel at two precisions** (not two integrators), the comparison (divergence time, separation growth) another derived field. No new machinery.
- (Amended twice: diagnostics ride the *forward* pass as tier-gated co-computations — **each sample's own Benettin FTLE shadow, and E ensemble copies that are full uniform samples** (the SSAA pool; sampling/SSAA note — the old "N ensemble shadows" framing is superseded). "Separate kernels" applies only to different integration *protocols*, e.g. reversibility's forward+reverse — and these are build-time Rust variants, not runtime-authored.)

### 5. Canonicalisation is the one seam to (m, r, p)
`canonicalise: physical → canonical (m, r, p)` — the quotient onto the section (CoM frame, scale/rotation fixed), a single function. It is used in **both directions' plumbing**: the decode path runs it after chart-specific construction (`z → decode → canonicalise → (m, r, p)`), and encode is `block inverses ∘ canonicalise` (`physical → canonical → z` — the one upward door; seam 8). Latent charts construct canonically and pass through trivially; physical-frame charts (Anosova etc.) genuinely need it. Everything converges to `(m, r, p)` before the integrator sees anything, so the integrator has exactly **one input type, forever**. Chart proliferation can never reach it.

## Through-line

Hard seams — **physics def | numerics · authoring | GPU · integrate | colour · chart | canonical | integrate**. Each piece has one job and one input. Buildable a seam at a time; an agent can work one seam without the rest bleeding in.

---

*Divergence is exposed, not reconciled. Debug views are first-class and built first. The integrator has one input type. Charts lower; they do not interpret.*
