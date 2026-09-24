# Principia — debug tooling plan

*Working note before the milestone plan. The organising principle, the four-surface matrix, and the actual catalogue: for each observable, what the **shader shows** and what the **test asserts**. Drawn from the render contract (Parts 5–8), generation-root dd, integrator dd, deep-zoom note, scheduler contract. This is the Phase-0 deliverable enumerated.*

---

## Principle

The debug tooling is not scaffolding — it is **the perception layer for building everything else**, and for an agent that cannot see the screen, it is the *only* sensory organ. It doubles as the test harness. So it is built **first**, before the physics and scheduler it observes, and validated against **synthetic hand-filled buffers** before any real component exists.

**One generated core, four surfaces.** The unpack/accessor layer is generated from the ledger (one source). It feeds four presentation surfaces:

|  | Shader (eyes — spatial/structural bugs) | Unit test (CI — pointwise/logic bugs, agent-perceptible) |
|---|---|---|
| **Payload** (`SimState`, bit-packed, arrays) | field-colouring fragment shaders | assert on unpacked accessors |
| **Structural** (`RenderQuad`, quadtree) | depth/state/coherence heatmaps | assert on the CPU quad structs |

Both modalities on both data sources — neither lens sees everything. A field test can pass (each pixel's `state` individually valid) while the shader reveals them spatially scrambled; a view can look plausible while a test catches an off-by-one in bit 7. The generated accessor (`sd_state` etc.) is called by *both* the shader and the assertion — **WGSL** (fragment) and **Rust** (kernel/host, used by the assertion) versions from the same layout definition — so four surfaces, not four implementations.

**Synthetic-first.** Every payload surface is validated by CPU-filling a `SimState` buffer with known values (a set `state`, a known escaper, a ramp in `energy_drift`, a bitwise-adversarial descriptor) and asserting both the accessor returns it and the shader renders it. This happens before the decoder exists — the direct fix for the black-screen failure.

---

## A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)

`DEBUG_MODE ∈ {NORMAL, UV_PASSTHROUGH, DECODE_PASSTHROUGH, ROUNDTRIP}` — alongside `DECODE_MODE` in dispatch flags. No new buffers; intermediates written into existing slots, reinterpreted on the render side.

| Mode | Shader shows | Test asserts | Certifies |
|---|---|---|---|
| `UV_PASSTHROUGH` | quad-local `u` (and `v`) as a gradient across each quad | reconstructed `(u,v)` for sampled quads = expected `c ± h·(2t−1)`; **no banding** = adjacent-sample deltas are smooth, not step-quantised | quad-local coordinate reconstruction; the linearised-decoder switchover. Banding = f32 precision loss made *visible* |
| `DECODE_PASSTHROUGH` | decoded IC before physics: z components, masses (ternary), body positions | decoded `(m,r,p)` = f64 `decodeOnly()` to Tier-N tol; identities (Σm=1, ΣCoM=0, I=1) | the Matter rung, before any integrator exists (Phase-0d "raw IC colouring") |
| `ROUNDTRIP` | `z → D → E → D` physical residual, log-scaled | residual ≤ `ε_phys` except at clamps/feasibility/mirror-tie (tagged expected) | the encode path — encode-side sibling of the invariant-chart gradient |
| `NORMAL` | (the real pipeline) | — | — |

---

## B. Payload field views — `sample_descriptor` (bit-packed u32)

One view + one test per field. Generated from the ledger; a garbage view here = ledger ≠ kernel-write, caught day one. All accessors use the **u32** `extractBits` overload (i32 sign-extends — the trap).

| Field | Shader shows | Test asserts |
|---|---|---|
| `state` (0–2) | categorical palette, **6 states** (escape/bounded/collision/running/sim_failed/decode_failed) | `sd_state(w)` round-trips 0–5; union legend switches on it; `running`(3) shows in-flight; the three failure/lifecycle states render distinctly |
| `detail` (3–4) | **legend keyed by state** (escape→body, collision→pair, sim/decode_failed→failure category — the 4-state union, payload §2; blank for running) | correct code per state; **the three-colours-bug regression test** — detail is read, not dropped; undefined-when-running handled |
| `saturated` (5) | boolean overlay | set iff the substep exponent hit `⌈log2 N_max⌉` at some point (sticky) |
| `dmin_pair` (6–7) | categorical(3) | round-trips; matches the pair that achieved `d_min` (a latched fact, not the word) |
| `total_substeps_log2` (derived, not a descriptor field) | scalar (log) — cumulative work / complexity | matches `⌈log2 Σ N_sub⌉`, derived at read via `countLeadingZeros` from the exact `total_substeps` u32 (payload §2); the *complexity* proxy (not peak — peak was dropped) |
| `fgw_truncated` (the **length sentinel** `length_raw == 127` in the word buffer's `.w` — NO flag bit; bit 24 is payload, payload §3) | flag view | fires iff a mixed-radix append would exceed the 121-bit budget (76 symbols); word-derived quantities styled invalid once truncated |
| `t_end` / `t_dmin` (in `times`) | scalar ramp (/65535 of horizon) | round-trip within quantisation; `t_dmin` absolute; **bit-identical CPU/GPU quantisation** (parity) |
| `d_min`, `dE_max`, `dLz_max` (f16, packed) | scalar ramp | `unpack2x16float` round-trips within f16 eps; sentinels styled, never scaled |
| **DERIVED views (not descriptor bits):** `orbit_count`/`retrograde` (from `theta`), the **reduced crossing count** (`fgw_reduced_length`, valid iff not truncated) / `enc_XY`/`dominant_pair` (topological word read — symbolic-dynamics contract), `ftle = S_final/(step_count·dt)` (partial renorm finalised — payload §5, never plain `S/t`), current drift (`H(r,p)−E_0`) | computed in the fragment from stored state/word | each matches a reference computed the same way; the word-derived tallies use the generator→pair mapping, not a histogram |
| **live current-substep count** *(live view, not a descriptor bit)* | **animated heatmap of substeps this macro-step** — read off the marching state | the substepper's live effort; close encounters propagate across the manifold in time (Part-5 live views) |
| **whole-word hash** | `dbg_hash(w)` — "is it changing at all" | distinct words → distinct colours | 

---

## C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)

| Field | Shader shows | Test asserts |
|---|---|---|
| `t_end_step`/`t_dmin_step` (in `times`, bits 0–15/16–31) | scalar (via `horizon_steps` uniform) | exact-index round-trip; bit-identical CPU/GPU (Tier B) |
| `total_substeps` (exact u32, separate field) | scalar (log proxy via `countLeadingZeros`) | matches Σ N_sub; the `total_substeps_log2` proxy is derived at read, not stored |
| *(no packed `orbit_count`/`retrograde` rows — derived from `theta`, §B derived views; `dmin_pair` is descriptor bits 6–7, §B)* | | |
| **word length** (`.w` bits 25–31, 7 bits; 127 = truncation sentinel) | scalar | `fgw_reduced_length` = **net reduced** crossings (free reduction pops cancel — NOT raw symbols appended), valid iff `!fgw_truncated` |
| **symbol-at-k** | scrubber `u_k` → symbol colour | `fgw_symbol(word,k)` per slot (sequential decode, cold path) |
| **whole-word hash** | hashed colour → **renders topological basins** (finer than outcome boundaries) | word-boundary set ≠ outcome-boundary set (the free Burrau topological-boundary diagnostic) |

---

## D. Payload field views — `SimState` scalars (11 × f32)

Scale per ledger metadata. **The two drift fields are signed → diverging scale, not sequential-log** (generation-root finding).

| Field | Shader (scale) | Test asserts |
|---|---|---|
| `t_end_step` | lin [0,T] via `horizon_steps` | completed-step count at all times, latched at termination (payload §2) |
| `d_min` | log | > 0; = min over trajectory |
| `ftle` | lin (tier) | ≈0 on Kepler-embedded, large on Burrau |
| `energy_drift` | **diverging** | oscillates (symplectic) vs drifts (Euler/RK4) |
| `diffusion` | lin, **−1 sentinel styled** | sentinel bit-exact; never scaled |
| `delta_E_max_abs` | log | ≥ |final| (spike-that-recovered) |
| `Lz_drift` | **diverging** | absolute-gated suspect |
| `delta_Lz_max_abs` | log | ≥ |final| |
| `E_0` | diverging | **= K₀+V₀** (cross-check) |
| `Lz_0` | diverging | invariant-chart gradient input |

---

## E. Payload field views — `ICDescriptor` (12 × f32) & the live-state block

| Field | Shader | Test |
|---|---|---|
| masses | ternary colour | = decode masses |
| `virial_ratio`, `rho_ratio`, `rho_angle`(cyclic), `K_0`, `V_0`(div), `r_min_pair_0`(log) | scalar per scale | = f64 decode values |
| **live shape views** `u_mode` | mode 0: `½(n+1)` dir-cosines RGB of current derived `n`; mode 1: running phase `θ̃` cyclic; mode 2: `|n|−1` error | `‖n‖=1` (flat-zero — derived fresh each step); `θ̃` continuous (no 2π jumps); terminal-latched samples frozen |
| accumulator views | FTLE-running `S/t` (live approx — the finalised read is `S_final/(step_count·dt)`, payload §5); diffusion slope `C_ty/C_tt(n)` (Welford, derived time-moments); drift max-vs-final | accumulator bookkeeping, live |

*ICDescriptor views certify the **decoder**, independent of any integration.*

---

## F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)

These certify the **CPU brain** — a wrong view here exonerates the GPU and points at the scheduler.

| View | Shader shows | Test asserts |
|---|---|---|
| quad-depth heatmap | depth `ℓ` as colour ramp | subdivision matches complexity + `ℓ < camera_depth + MAX_REL_DEPTH` (sliding cap) |
| quad-state enum | loaded / pending / refinable / **terminal** / stale as discrete colours | state transitions legal; terminal iff a TRUE floor hit (linear-decoder `AT_F32_FLOOR`, or the integration floor — quad-level refinement-stops; the sample `saturated` flag is non-terminal and the screen floor is a view-relative veto, never terminal — scheduler Part 4) |
| coherence / impurity | scalar heatmap | = reduction output; high where boundaries |
| ensemble spread | scalar (tier) | present iff `contains-ensemble` |
| suspect fraction | scalar | flags integration-floor quads |
| priority score | scalar | = `w_v·P_v + w_z·P_z + w_c·P_c + w_f·P_f` |
| cache age / ancestor gap | scalar | LRU + fallback-chain behaviour |
| **UV passthrough (coordinate convention)** | raw post-flip `(u,v)` as a gradient (`u→red, v→green`), + quad-local `δ` as a second mode | **the Y-flip is correct** — the gradient runs bottom-left-origin Y-up (green increases *upward*); a wrong flip is instantly visible as an inverted gradient. Makes the invisible mirror-bug (coordinate note) visible *before any physics*. Structural, validated at 0b |
| **picking cross-check** | click/hover a known corner or centre → mark the reported UV/IC | reported coordinate = expected corner (top-left click → `v≈1`, bottom-left → `v≈0`); certifies pointer-event → post-flip-UV path agrees with the render flip (the wrong-subsystem bug: clicking top inspects bottom) |
| **quadtree boundaries / leaf outlines** | overlay (post slot + `ctx.uv`) | active leaf set correct; pinned chain never evicted |
| fallback ancestor tint / pending hatch | overlay | slippy-map fallback shown, never blank |

---

## G. Cross-check views (certify a *seam*, not a field — integration tests with a display)

| View | Shader shows | Test asserts |
|---|---|---|
| **invariant-chart gradient** | `E_0` / `L_z_0` on an (L_z,E)/(L_z,K) chart | **perfect axis-aligned gradient** — one glance certifies the invariant downstream solve (the strongest single test) |
| energy agreement | `|E_0 − (K_0+V_0)|` log | ≤ tol (decode-time vs kernel-time) |
| winding consistency | derived `orbit_count = ⌊|θ̃|/2π⌋`, `retrograde = θ̃<0` vs the running `θ̃` accumulator | derivation = live source; terminal-latch respected |
| diffusion (Welford) | slope `C_ty/C_tt` vs reference computed identically | the streaming accumulator bookkeeping is correct |
| **impurity mask** | per-sample `state` ≠ quad majority `state` | spatial mean = `outcome_impurity` — **needs the `dominant_outcome` grain fix (pending change 1)** |
| drift shape | `energy_drift` (final) vs `delta_E_max_abs` (max) | distinguishes secular loss from recovered spike |

---

## H. Codegen self-test (the tooling that tests the tooling)

| Test | Mechanism |
|---|---|
| pack∘unpack = id | property-fuzzed per field, **in Rust (host) and via a GPU self-test dispatch** of the WGSL fragment unpack (where the i32-`extractBits` trap lives — the Rust side has no `extractBits`) |
| disjointness/coverage | static: no field overlap; reserved bits explicit; widths fit ranges |
| metadata gate | delete any `scale` → generation fails, field named |
| f16 pairs | round-trip via `unpack2x16float` within f16 eps |
| union semantics | `detail` decodes per `state` |

---

## Build order within Phase 0 (the only forced staggering)

1. **0a — generation root**: ledger + generated pack/unpack (Rust + WGSL) + codegen self-test (§H). Nothing renders yet; tests green.
2. **0b — synthetic payload harness**: CPU-fill `SimState`/`ICDescriptor`, the field-view shaders (§B–E) + their accessor tests. **Screen colours a hand-filled buffer; both surfaces green — before any physics.**
3. **0c — kernel debug modes** (§A): `UV_PASSTHROUGH` first (needs only quad-local coords + a trivial kernel), then `DECODE_PASSTHROUGH` once the decoder lands (Phase 1), `ROUNDTRIP` once encode lands.
4. **structural views (§F)** land the moment `RenderQuad` is *defined* — before the scheduler *logic* that fills it with interesting values, so adaptive refinement is visible as it's built. **The UV-passthrough coordinate view is the earliest of all** — it needs only a rasterised quad and the flip, so it lands before decode/schedule/anything, catching a wrong Y-convention (coordinate note) at the very first pixel rather than a week later in the wrong subsystem.
5. **cross-checks (§G)** land per the seam they certify, as those components arrive.

Everything payload-shaped is testable at 0b against fiction. Everything structural precedes its subsystem. By the time Claude Code writes decode/integrate/schedule, the eyes and the assertions for that component already exist and are green.

---

*One generated core, four surfaces, synthetic-first. The shader catches what a scalar can't; the test is the only thing the agent can see. Build the eyes before the thing seen — for an agent, the eyes are the only sense there is.*
