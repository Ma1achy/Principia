# Principia toolchain spike — findings

**Verdict: rust-gpu.** The browser SPIR-V→WGSL hop is clean — not merely
tolerable but bit-identical to native on every golden input — so the brief's
decision rule selects rust-gpu: uniform Rust including the kernel, and the
double-double path confirmed open. Date: 2026-07-17. Machine: Apple M3 Pro
(Metal), Chrome 149, macOS 25.2.

Toolchain as tested: rust-gpu / spirv-std / cargo-gpu **0.10.0-alpha.1**
(pinned nightly-2026-04-11), wgpu / naga **30.0.0**, wasm-bindgen 0.2.126.

The four criteria, in the brief's pain order:

---

## 1. Browser survival — PASS, bit-identical

Pipeline: `kernel` (plain Rust) → `spirv-unknown-naga-wgsl` target (official
in 0.10.0-alpha.1) → naga 30 `spv` → `wgsl` → wasm + wgpu → Chrome 149
headless WebGPU (Tint → MSL), driven by `wasm-pack test --chrome --headless`.

Result over all 274 golden states (golden 1 at k=1 and k=100, the full
boundary sweep): **0 branch forks and worst continuous delta 0.0e0 — the
browser output is bit-identical to the native GPU output on every word.**
Both paths end at Metal on this machine and the two MSL compilers
(naga's and Tint's) round this kernel identically. On other OS/GPU pairs the
continuous fields would be expected to differ at FP scale; the branch words
should not.

It did not run clean *on the first attempt* — three landmines, all fixable in
source and none fatal (detail in §3): a non-finite literal rejected by naga's
validator, and a genuine naga→MSL loop miscompile avoided by unrolling.
Harness papercuts: wasm-pack pins a cached chromedriver (151) that 404s
against installed Chrome (149) — the matching chromedriver must be swapped in
manually; `webdriver.json` needs the WebGPU flags.

## 2. Branch determinism — the headline finding

**The f32-quantise rule as briefed does not survive real compilers; a
comparison-only reformulation survives them bit-exactly.** Three formulations
were measured on the golden-2 sweep (272 states straddling every reachable
`ceil()` boundary and the collision boundary, ±8 ulps, axis-aligned and
oblique):

| Rule formulation | native-GPU vs CPU forks |
|---|---|
| As briefed: f32 `sqrt`→`div`→`powf(1.5)`→`ceil`→clamp | 5/272, all within ±1 ulp of exact boundaries |
| Algebraic: `powf` → `q*sqrt(q)` | 7/272 (different samples — not an improvement, a reshuffle) |
| **Threshold table: `d²` vs 64 precomputed f32 constants** | **0/272 — and 0 everywhere: CPU-f64 vs CPU-f32 vs native GPU vs browser GPU, twofloat included** |

**FMA contraction is not the culprit.** The axis-aligned forks (`dy = 0`, d²
an exact lone square, sqrt correctly rounded on both sides) corner the cause
inside the transcendental pipeline itself: **wgpu's Metal backend compiles
MSL with fast-math defaults** (`MTLCompileOptions` is created with only
language version and `preserveInvariance` set — wgpu-hal-30.0.0
`metal/device.rs:272`). Under fast-math, division may lower to
reciprocal-multiply, sums may contract or reassociate, and `pow` carries no
rounding guarantee — so *no multi-op float expression is ulp-stable through
that compiler*, and swapping one transcendental for another just moves the
forks (5→7).

*[Corrected by controlled test — see "Criterion 2 — mechanism, controlled"
below: fast-math is a contributor but not the load-bearing cause; disabling
it does not restore determinism. The conclusion that only comparison-only
branch inputs are safe is unchanged and strengthened.]*

The fix is categorical, not numerical: **feed branches nothing a fast-math
compiler may legally perturb.**

- `N_sub` = comparison of `d²` against a compile-time table of 64 bit-exact
  f32 thresholds `t_n = round_f32((r_sub/n^⅔)²)`, as a generated binary
  comparison tree — no sqrt, no division, no pow, no loop, no runtime
  indexing (`kernel/src/nsub_table.rs`, generator in `scripts/`).
- Collision = `d² < r_coll²` against a constant.
- Horizon = integer step counter, never accumulated float time.
- Clamp in f32 *before* any float→int cast (`OpConvertFToU` is UB
  out-of-range in SPIR-V; Rust's `as` saturates — a silent fork source
  independent of rounding).
- The only arithmetic feeding a branch is `d²` itself. Two variants measured:
  quantise-at-`d²` (the brief's "quantise late" spirit — CPU computes d² in
  F, rounds once) held 272/272 on this sweep but can still leak 1 ulp on
  oblique geometry (f64-sum-rounded-once vs f32-per-op; observed pre-table as
  a real n_sub 5-vs-4 fork). The **position-quantised variant** — cast
  positions to f32, then lone-sub, lone-mul, explicit `fma(dy,dy,dx²)` —
  is identical-op-sequence on every backend and is deterministic by
  construction. **Recommendation for Principia: the position-quantised form.**
  (Explicit fma matters: written as `dx*dx + dy*dy`, one compiler may
  contract and another not; writing `fma` makes every backend take the fused
  path.)

With the table rule, branch decisions (`state`, `n_sub`, `n_sub_alt`,
`steps_done`, `total_substeps`) are **bit-identical across all five
instantiations tested** — CPU-f64-quantised, CPU-f32, native-GPU-f32,
browser-GPU-f32-via-WGSL, and CPU-double-double — on every golden input.

Continuous parity (honest FP divergence, reported not asserted tight):
golden 1 one-step GPU vs CPU-f64: 4.7e-10; 100 steps: 1.8e-7 vs CPU-f32,
8.3e-7 vs CPU-f64; f64 energy drift over 100 steps: 3.9e-5 (KDK at dt=0.01).
Golden 3 (Burrau, dt=1e-3): all three backends reach COLLISION at t=1.8790,
step 1879, substep 1883 — structurally identical to the substep. (dt=0.01
blows through the first Burrau encounter unresolved — energy +5287 from
−12.8 — hence 1e-3 for that input.)

## 3. Uniform Rust including the kernel — PASS, with a discipline list

The physics kernel is one plain `#![no_std]` Rust crate, generic over
`num_traits::Float` + a 2-method `Quant` trait; the rust-gpu entry point is a
20-line skin (`shader/src/lib.rs`). The identical source compiles under
stable rustc (f32, f64, twofloat) and under rust-gpu's pinned nightly to
SPIR-V. No dialect island. Criterion met — but "plain Rust" earned three
naga-path rules, found by real failures:

1. **No non-finite literals.** `f32::INFINITY` as a min-fold seed compiles
   to SPIR-V fine and is rejected by naga validation ("Float literal is
   infinite" — WGSL cannot express it). Seed folds with the first element.
2. **Implicit bounds checks are compiler-injected multi-level loop exits.**
   The user-level discipline (all `while`, no `break`, flag-checked
   conditions) held — but rust-gpu lowers every runtime-indexed `a[i]` to
   `if in_bounds {} else { break-to-panic }`, visibly reintroducing the
   wgpu#4449 shape in the emitted WGSL. Consequence observed: the triangular
   nested energy loop **silently truncated through naga→MSL — the GPU summed
   one pair of three** (H = −0.577 vs −1.732) while rectangular nested loops
   (forces, drift) translated correctly. Silent wrong-answer, not a hang.
   Fix: constant indices / unrolled pairs where iteration count is small,
   which elides the checks entirely.
3. **The two-level adaptive structure survives when written safe.** Outer
   macro-step loop × inner substep loop with terminal exits via
   state-flag-in-loop-condition (zero `break`s) ran correctly through
   SPIR-V→MSL and SPIR-V→WGSL→Tint at k_steps up to 100 in one dispatch.
   This shape drops into Principia's real wrapper.

Ecosystem papercuts (half a day total): crates.io `cargo-gpu` resolves to a
0.1.0 placeholder — install `--version 0.10.0-alpha.1` explicitly; spirv-std
declares `glam >= 0.30.8` but breaks against 0.33 (pin + `cargo update
--precise 0.30.8`); Homebrew's rustup wrapper defeats proxy-by-argv0, so
`cargo +nightly` shims must symlink the real Cellar binary.

## 4. Precision headroom — PASS, trivially

`twofloat::TwoFloat` (double-double, ~106-bit) implements
`num_traits::Float`; an optional kernel feature adds an 8-line `Quant` impl,
and the *same kernel source* runs: 100 golden-1 steps and the full sweep,
zero branch forks vs f64. The independent high-precision reference
integrator can share the kernel, as hoped. (CubeCL's ceiling here would be
its MLIR CPU runtime at f64.)

---

## The decision

> rust-gpu if the browser SPIR-V→WGSL hop is clean — it is, bit-identically
> so, through an official compile target. **rust-gpu.**

CubeCL remains a credible fallback (researched, not built: wgpu runtime runs
on wasm via Burn's plumbing, `#[cube]` covers this kernel's needs), but it
loses on both differentiators: its `#[cube]` fns panic on the host at math
intrinsics (CPU path = heavyweight vendored MLIR/LLVM JIT, no wasm), and it
caps at f64 where rust-gpu's plain-Rust kernel just ran at double-double.

**What changes Principia's architecture (the criterion-2 answer):** the
single-sourced f32-quantise rule *as a float pipeline* does not survive two
real compilers — one of them runs fast-math and is entitled to. The invariant
survives only when branch inputs are reduced to: integers, comparisons
against compile-time constants, and single-rounded ops (lone sub/mul,
explicit fma) on f32-quantised operands. That is a stronger, buildable rule —
"branch inputs are comparison-only over a constant table" — and with it, the
strong form of the invariant held bit-exactly across five instantiations,
two shader compilers, and a browser.

*Caveats: one machine (everything funnels to Metal on M3 Pro — a
Vulkan/D3D12 box would add independent evidence); alpha toolchain (naga
targets not conformance-tested, and the silent loop truncation in §3 is the
supporting argument for keeping the parity suite alive in CI); goldens
regenerate with `cargo run -p host --bin gen_goldens`.*

---

## Criterion 2 — mechanism, controlled (addendum, 2026-07-17)

The fast-math attribution above was identified from source, not flipped and
re-measured. Controlled test: vendored wgpu-hal 30.0.0 with an env-gated
`MTLMathMode::Safe` switch (`vendor/wgpu-hal`, `METAL_SAFE_MATH=1`); kernel
feature `powf-rule` restores the as-briefed pipeline verbatim and writes
`pw`'s raw bits into the spare diagnostic word, so the GPU's pw is read back
exactly. Same build, same sweep, math mode the only variable
(`host/tests/mechanism.rs`).

| Condition | branch forks | pw-bit mismatches (GPU vs CPU) |
|---|---|---|
| Fast (stock default) — control | 5/272 (reproduces original set exactly) | 194/272 |
| `MTLMathMode::Safe` | **5/272** (different set: 2 vanished, 2 appeared, 3 identical) | 183/272 |

**Conclusion: the load-bearing mechanism is inherent transcendental
latitude, not fast-math.** Metal's `pow` disagrees with libm's `powf` by
1–3 ulp on ~70% of sweep inputs *in both math modes*; with Safe-mode
division correctly rounded, the persisting forks isolate to `pow` itself
(e.g. `pow(4.000000477, 1.5)`: Metal exactly 8.0, libm 8.000001907 — 2 ulp).
Fast-math is a real secondary perturbation — it moved which
boundary-adjacent samples fork — but disabling it does not restore
determinism, so "turn off fast-math" is **not** a fix. The comparison-only
rule is the fix under either mechanism (and measured 0/272 in both modes).

Per-fork classification of the original 5 (control run, pw values measured):
`nsub2/axis+0` GPU pw 2.000000238 (+1 ulp over 2) vs CPU 2.0 → n 3 vs 2;
`nsub3/axis-1` GPU 2.999999762 vs CPU 3.000000477 → 3 vs 4 (both vanish
under Safe → fast-math-sensitive); `nsub4/obl-1` GPU 4.000000954 vs CPU-f32
4.0 / CPU-f64 4.000000477 (the oblique d² representational leak, visible at
bit level) → 5 vs 4/5; `nsub8/axis-1` and `nsub8/obl-1` GPU exactly 8.0 vs
CPU 8.000001907 → 8 vs 9 (all three persist under Safe → pure pow
latitude). **All five pw values lie in [2, 9], far inside the f32-clamped
cast range: 5/5 transcendental-rounding, 0/5 float→int cast.** (The
OpConvertFToU hazard was pre-empted by clamp-before-cast from day one and
never fired; it remains a discipline rule, not an observed mechanism.)

Build profiles: all originally reported CPU results ran the `dev` profile
(opt-level 0); shader binaries always release (cargo-gpu default); wgpu/naga
stock 30.0.0 via `ShaderSource::SpirV` (naga spv-in → MSL), no passthrough;
browser leg naga-cli 30 + Chrome 149. Re-run under `--release`: mechanism
counts (5/272, 194 and 183 pw mismatches) and the table-rule suites (0/272,
all green) are **identical bit-for-bit** — CPU opt-level ruled out as a
contributor. The vendored wgpu-hal control run reproduced the original 5/272
with the identical sample set, so the vendor copy itself is non-perturbing.
