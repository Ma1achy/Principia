# ADR-0001: The mock's value noise addresses and hashes its lattice in integers

**Status: Proposed.** Authority: none until a ruling or an "applied per R-369" record is cited here. Asked for by R-685.
The owner's choice of pattern is RQ-691.

**Relation to `decisions.md`:**
- An ADR carries reasoning and evidence, never a rule.
- It closes nothing and is never cited as a source of a requirement.
- Its Status names the R-n or the "applied per R-369" record that accepts it. Until then it reads Proposed.
- If it conflicts with `decisions.md`, `decisions.md` wins and the ADR is superseded by a new one, not edited to agree.
- The owner's acceptance of the pattern change goes in `decisions.md`, not here.

**Where ADRs live:** `plan/adr/`, the convention applied per R-369. `docs/` would make an ADR normative by location,
since the corpus there is the authority (R-1), and `docs/archive/` and `docs/reference/` can't be cited as normative
(R-159). `plan/` is derived from the docs, and `plan/check_plan.py` reads only `plan/tasks/M*/TASK-*.md` as tasks.

## Context

Sources: issue #390, comment 6632801932 (the RTX) and comment 6633374835 (Metal, lavapipe and the audit).

**Observed:**
- RTX 2080 SUPER, Vulkan, driver 621.01: `hash(i + vec2(1.0, 0.0))` in cell N and `hash(i)` in cell N+1 differ, with equal
  inputs.
- Unequal crossings at ×0.77: 1031 of 7217 (warp 1, octave 0) and 53612 of 136092 (octave 4). Largest difference 0.94.
  Jump lines appear in `warp`. 292 of 293 mid-row mismatched columns lie on one.
- With the `+1` read from a uniform: 0 unequal in all 20 cases.
- Metal (M3 Pro) and lavapipe (llvmpipe, Mesa 26.0.8): 0 of 20 cases unequal, 0 jump lines, on naga 30.0.1, the version
  the Windows program's lock and the repo's lock at 256c90b9 also hold (confirmed by the Windows session).
- `hash(i)` against strict f32: Metal 79.95 to 93.78% equal, lavapipe 100%, RTX about 86 to 92%. Inexact values alone make
  no blocks; only disagreement between cells does.
- Repeats exactly across 2 full runs, with no driver events. The Windows report carries the owner's label "possibly
  faulty hardware"; this result is deterministic, and it is one card.

**Read from source:**
- naga 30.0.1 has no `NoContraction`, so the WGSL path cannot forbid this.
- `hash` assumes four inlined copies round alike. Nothing promises that.

**Modelled here, not measured.** The model is strict f32 in NumPy, at the same views and field points. It makes one
change: each offset corner computes `fract(i*k + o*k)` in place of `fract((i + o)*k)`. That is the rewrite that folds
`1.0 * 127.1`.

| Quantity | RTX (given) | Model |
|---|---|---|
| warp 1, octave 0, unequal | 1031 of 7217 | 1031 of 7217 |
| warp 1, octave 4, unequal | 53612 of 136092 | 53612 of 136092 |
| Largest difference | up to 0.94 | 0.940 |
| Mid-row mismatched columns | 293 | 293 |
| Jump lines ×0.77: warp.x, warp.y (V, H) | 77,62 and 78,63 | 77,62 and 78,63 |
| Jump lines ×1.00: warp.x, warp.y (V, H) | 62,52 and 62,55 | 62,52 and 62,55 |

The unmodified strict form gives 0 in every row.

**Inferred:** the card distributes `(i + c) * k` over a compile-time constant. All twelve numbers matching is strong
evidence, but no compiler output exists. The stage is unknown, and the model does not explain why the card's `hash(i)`
also departs from strict f32. Why Metal and lavapipe do not do it is unknown too. Lavapipe's 100% is one driver's
behaviour, not a Vulkan promise.

**Consequence:** the uniform result fits, since a non-constant gives the folding nothing to gain. It is no guarantee: the
same rewrite on `e` is legal and would reproduce the defect, by the same model.

## Decision

The corner path, from `floor` to the corner's value, holds no float arithmetic that can round:

```wgsl
fn lattice_hash(c: vec2<i32>) -> u32 { let u = vec2<u32>(c); return pcg(u.y ^ pcg(u.x)); }
fn corner(c: vec2<i32>) -> f32 { return f32(lattice_hash(c) >> 8u) * 0x1p-24f; }
fn cell_corners(c: vec2<i32>) -> vec4<f32> { /* corner(c), c + (1,0), c + (0,1), c + (1,1), integer additions */ }
fn value_noise(p: vec2<f32>) -> f32 { let whole = floor(p); let f = p - whole; let c = vec2<i32>(whole); /* ... */ }
```

- `pcg` is the form of `dbg_pcg` (`crates/render/shaders/wgsl/lib/present.wgsl`:103). It is not in the prelude the mock
  includes, so it is copied, citing that line.
- The top 24 bits matter. A 24-bit integer converts to f32 exactly, and multiplying by `2⁻²⁴` is exact, so no rewrite of
  that step can change a bit. That includes a reciprocal fold (compare R-296).
- Corners are therefore bitwise equal across cells and across backends, and equal to a CPU twin.

## What removes the cause, and what does not

- **Integer `+1`:** the offset no longer reaches float arithmetic. Integer addition and multiplication are exact mod 2³²,
  so reassociating them changes nothing. Equal `c` gives equal bits.
- **`i` is still `floor` of a float, but this is one input, not two.** Cell N's `c + 1` equals cell N+1's `c` exactly while
  every integer is representable (below). The float path before `floor` (`q * 2.03 + (17, 9)`, `p + (3.1, 1.7)`) still has
  compile-time constants. A rewrite there moves `p`, and so a cell edge, by an ulp. That is harmless once corners agree,
  because the noise is continuous across the edge and has no `fract` amplifier at that step.
- **After the corners:** `mix` stays float, so ulp-level differences remain, unamplified. `fract(n * 4.0)` stays too: the
  band edge, intended look.
- **`f = p - whole` replaces `fract(p)`,** so `f` and `c` come from one `floor`. If a driver computed `fract` by a different
  route, the two could disagree. Not observed; the change costs nothing.

**So:** the integer `+1` removes every compile-time float rewrite in the corner path. It does not remove them from the
shader as a whole, and this ADR does not say it does.

## Alternatives

| Option | Alone | Removes the cause? | Cost / weakness |
|---|---|---|---|
| A. Integer `+1`, integer hash, exact conversion | **Recommended** | Yes, by construction | Pattern changes. Adds `pcg`. No new binding |
| A1. Integer hash of `vec2<i32>(i)` with `+1` still in float | No | No. `i + 1.0` is a float add before the cast; it rounds nothing here but keeps a float path into the address and invites a `vec2<u32>` mistake | No gain over A |
| B. Corners once per cell, in the shader | No | No. A fragment invocation has no memory across pixels, and the seam is exactly two invocations in different cells. `DontInline` is not emitted by naga's SPIR-V writer (grep) | Useful only as structure: `cell_corners` gives the test the shipped function, so it is kept inside A |
| B2. Corners once, in a host-computed table read by `textureLoad` | Works | Yes, if indexed by integers | New binding across three pipelines, an upload path, periodicity. No gain over A |
| C. Uniform-held offset | Observed cure on one driver | No. Blocks the folding, not the rewrite. `fract` stays. 20 cases, one card, nothing forbids it | Grows `ChartUniform`. A comment is the only guard. Keeps old pixels |
| D. Integer `+1`, then the old float `hash(vec2<f32>(c + o))` | Keeps the pattern | Partly. No float add left to distribute, but the `fract` amplifier stays, and a compiler seeing through `f32(c + 1)` is unexamined | The only way to keep the look. Untested on the RTX. **The fallback if the owner refuses a pattern change** |
| A + C | | | C adds nothing to A |
| A + B | **Chosen** | As A | B is structure only |

## f32 range and the cast

Lattice at octave 4 ≈ 16.98 × (3(z + 1) + offset) + 264, on x. The y axis is about 1.2 times further.

| Lattice | Meaning | Rough `\|z\|` on x |
|---|---|---|
| ≤ 521 | Measured views (modelled across all 20 cases) | under 2 |
| 2²⁰ | `f` has 3 bits; stepped field | 2.1e4 |
| 2²³ | `f` is 0; value noise is `corner(c)`, white noise | 1.6e5 |
| 2²⁴ | Odd integers vanish, so `c + 1` is nobody's `c`; `u = 0`, so that corner has weight 0 and no seam exists | 3.3e5 |
| 2³¹ | i32 limit | 4.2e7 |

**Reachability:**
- The zoom slider's `ZOOM_RANGE` lower end, −4, gives half-span 16, so a zoom-out alone stays in the hundreds to low
  thousands of lattice units.
- Deep zoom never raises `i`.
- Only a far pan reaches these limits, and R-570 sets no pan limit on latent charts.
- Whether the mock's own pan clamps was not traced. `Z0_HALF_RANGE` is a slider range.

**Far-pan behaviour changes (modelled).** At `|z|` = 4e4 the old strict hash has gone flat (`warp.x` mean 0), because
`i * 127.1` loses its fraction. The new one stays noise but steps, and the detector counts 510 lines at that view. That
staircase comes from f32 `p`, not from corners. So the jump-line test stays on the ordinary views, and the corner test
goes further.

**Cast:**
- `floor` gives an integer-valued f32, so `i32()` is exact in range.
- Out of range is defined by naga, not the driver: `FClamp` before `ConvertFToS` (`back/spv/block.rs`:2380-2426) and
  `naga_f2i32` (`back/msl/writer.rs`:6263-6292).
- NaN through `FClamp` is unexamined. A finite chart does not reach it.
- `vec2<u32>(c)` from i32 is a bit reinterpretation.
- **`vec2<u32>(floor(p))` directly is forbidden.** It saturates negatives to 0 and flattens the negative half-plane, which
  the field reaches (modelled ×0.77 `p` from −0.89 before offsets; further with pan).

## Effect on the look

**No fixture holds mock pixels, so nothing is re-baselined:**
- `fixtures/golden/BASELINES.md` has no row for the mock or for noise, so R-110's "no re-baselining without a gate decision"
  is not reached.
- `fixtures/screenshot/*/cases.json`: 50 mock cases, all layout cases. `xtask/src/screenshot.rs` states "the runner compares
  no pixels"; captures go to `target/screenshot/` for the gui reviewer, and nothing is committed.
- `fixtures/screenshot/01_main/compass/reference/*` are compass drawing records, and `selftest/reference.png` is unrelated.

**What still changes:** every noise pixel of `fs_main`, `fs_debug` and `fs_stain`, so every captured figure and the Stain
preview. Same family, different pattern. Modelled, with no GPU run: `n` mean about 0.49 to 0.50, std 0.117 to 0.126; old
`warp.x` std 0.113 to 0.119, new 0.119 to 0.122; new corners uniform (deciles flat, neighbour correlation under 0.004),
the old hash lumpy (deciles 27k to 42k). R-390 allows this: "smooth noise", placeholder content.

**Why no integer hash reproduces the old values:** they are IEEE f32 rounding of products with `127.1` and `311.7` passed
through `fract`, so reproducing them in integers means soft-float multiplication on every corner. It would match lavapipe
only, since Metal's `hash(i)` departs from strict f32 at 6 to 20% of pixels. Only option D keeps the pattern, and that
is not an integer hash.

**No mock golden is added.** Float tails stay backend-dependent (R-686's baseline), and the CPU twin pins the part that can
be pinned, without images.

## Must not claim

- That any real shader or kernel path is safe on NVIDIA. The audit's "No real shader has a float lattice hash" (issue #390,
  comment 6633374835) is not "safe": `canon.rs`'s premise stays unenforced on Vulkan (RQ-690).
- Anything on RQ-685, RQ-686 or RQ-687, or on R-297's meaning. No change to `StageMode::Off`, tolerances or probes.
- A link to #393 or #396. Nothing here connects them. "Same driver" is a hypothesis.
- That the driver's rewrite is proven. It is reproduced by a model; no compiler output exists.
- That integer lattices are a rule for the kernel. This is a mock decision.
- That `NoContraction` or any decoration is available or would help.
