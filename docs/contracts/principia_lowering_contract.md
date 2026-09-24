# Principia — lowering contract

*Eleventh doc. The compiler layer: how a configuration becomes a concrete GPU pipeline. The other contracts say what must be true; this one says what gets built. Three agreed principles govern it: (a) **two pipelines** — a compute **kernel** (Rust) and a fragment **shader** (WGSL) — meeting only at the payload buffer; (b) **rules, not enumeration** — the table is a generation contract, with the current chart set worked as an appendix; (c) **two mechanisms** (Part 2, superseding the earlier 'one mechanism' framing) — the compute pipeline is **monomorphised Rust** (single-sourced CPU/GPU, no runtime assembly); the fragment pipeline is **assembled from keyed WGSL snippets** and hot-recompiled.*

---

## Part 1 — What lowering is

```
resolve(ViewState, SimKey, RenderConfig) →
    { compute_key,  compute_uniforms,        // chart+decode+integrator side
      fragment_key, fragment_uniforms,       // stain-graph render side
      dispatch_plan }                        // quad list, per-quad bindings, workgroups
```

This is axiom 4 ("charts lower; they do not interpret") made literal: the resolution function selects and parameterises *specialised* pipelines. There is no runtime interpreter of axis kinds, links, occupants, or slots anywhere on the GPU.

**The factoring that keeps the space finite:** the configuration tuple does *not* lower into one shader. Chart map + decode + canonicalise + wrapper + occupant lower into the **compute pipeline**; the stain graph + compositor lower into the **fragment pipeline**; the two meet *only* through `SimState`/`ICDescriptor`. Two small keyed sets that compose, never one product.

---

## Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)

> **Superseded by the substrate decision (`principia_spike_brief.md`: whole engine in Rust → wasm, physics kernel shared CPU/GPU via rust-gpu).** The two pipelines no longer share an *assembly mechanism*. They never shared *code* — the compute side is physics, the fragment side is colour — only the string-assembler; and the shared-source Rust kernel means the compute side is **not** assembled from WGSL strings at all. This is a real cost booked honestly (the previous elegance of one hot-reload path for both), bought back by something worth more: the physics kernel is single-sourced, so CPU/GPU logic *cannot drift* (parity contract §6). The split is chosen precisely so the parity-critical side gets shared source while the parity-free side keeps its runtime devkit.

**Compute pipeline — monomorphised Rust, not string assembly.** The chart map Φ, decode, canonicalise, wrapper, and occupant (its seam is `ADVANCE`, integrator contract Part 2a, R-19) are one Rust kernel generic over the float type and over chart/occupant (Rust generics + `#[cfg]`/trait selection), compiled by rust-gpu to SPIR-V and — the *same source* — to the CPU-f64 reference. "Charts lower, they do not interpret" (axiom 4) is served *better* by monomorphisation than by string-splicing: a chart is a type parameter the compiler specialises, not a snippet concatenated. Generated pack/unpack (from the layout table) and link maps (from the registry) remain generated, now as Rust the kernel calls rather than WGSL prelude. **Consequence — no runtime-authored custom compute occupants:** you cannot compile user Rust in the browser, so a user's experimental integrator/Φ is a build-time variant, not a text-box occupant. This is an accepted loss (niche — few users write their own symplectic integrator; the custom-*colour* devkit, which is the centrepiece, is preserved below).

**Fragment pipeline — unchanged: hand-WGSL keyed-snippet assembly.** The colour side stays the render doc's flow — the stain graph's keyed WGSL snippets (one function per node) concatenated with the shared prelude and a generated `shade()` that walks the graph (R-64; render_gui_spec Part II §10.1), hashed, compiled async, cached, last-valid-on-failure — because colour has **no f64/f32 parity stakes** (parity contract §6) and *needs* its runtime path: this is what keeps the custom-shader devkit (GUI §3–5, lowering Part 3a) alive, where built-ins dogfood the custom path as one mechanism.
```
[prelude]            context, colour spaces, vMF, unpack helpers (generated WGSL)
[node functions]     one per stain-graph node: source / colour / brightness / combiner / post  ← occupants (built-in | debug | custom, runtime-authored)
[shade()]            GENERATED — walks the graph: sources → colour? / brightness? → combiner → (post)* → OUT
```

So provenance now splits by *side*: the **compute** side is generated + shared-Rust, monomorphised at build time; the **fragment** side is generated + authored + user WGSL, assembled and hot-reloaded at runtime. A compute change recompiles the Rust kernel (re-integrates anyway — sim key); a fragment change re-assembles a WGSL pipeline (render key, imperceptible). Part 3a's uniform read-side interface applies to the fragment side, where custom and built-in occupants read `SimState`; the compute side, being one Rust source, has no read-side-drift hazard to resolve.

---

## Part 3 — The baking rules (compile-time vs uniform)

The central tension: bake too much → variant explosion and compile stalls; uniform too much → dynamic branching in the hot loop. The rulings, with reasons:

### Compute side

| Degree of freedom | Baked / uniform | Why |
|---|---|---|
| **Chart type** (axis kinds, Φ structure) | **BAKED** | genuinely different code (raw assignment vs invariant solve vs curve embed vs physical-frame entry). Axiom 4 |
| **Link selection** per block | **BAKED** (monomorphised) | different transcendentals; registry is small; the link *function* comes from the registry (generated Rust) |
| **Integrator occupant** | **BAKED** | never branch per-step on occupant; profiles differ structurally |
| **Tier co-computations** (FTLE shadow, ensemble) | **BAKED** | they add per-thread *state* (each sample's tangent vector; the E ensemble copies) — present-but-unused state costs registers/occupancy even when branched off |
| **Decode mode** (full vs linearised) | **UNIFORM** (per-quad flag) | the branch is workgroup-uniform (whole quad shares the mode) → free on GPU; halves the variant set; matches the `QuadRequest.flags` design (scheduler contract Part 5). *Promotion path documented:* if the dead full-decode path measurably hurts deep-quad occupancy via register pressure, it becomes a baked variant — a one-line key change under the assembler. (Note: a workgroup-uniform two-path branch is **not** a runtime axis-kind interpreter; axiom 4 is untouched) |
| Kernel bring-up mode (the one kernel debug mode — `principia_colour_composition.md` Appendix A; R-75) | **BAKED** (pre-built SPIR-V; pipeline created on demand — no runtime source compile, Part 4) | different code entirely (writes a known pattern instead of physics), but debugging is not gesture-critical → async pipeline creation on first use. UV / DECODE / ROUNDTRIP are fragment presets (colour_composition §6), not kernel variants |
| Wrapper config (`T, dt, N_max, r_*, eps_*, n_renorm`) | **UNIFORM** (`SimUniforms`) | sim-key *values*, not code. (`n_renorm` is the Benettin renorm interval — the old checkpoint count `M` no longer exists; integrator dd §6) |
| Chart params (`z₀, q₁, q₂`, slice values, curve tangent `γ'(ν₀)`, invariant targets) | **UNIFORM** | **navigation is uniform edits** — the entire Part-4 navigation contract depends on this; no gesture ever compiles |
| Per-quad (`c, h, x₀, J_D`, quad `T`, decode flag) | **PER-QUAD UNIFORM** | deep-zoom doc |
| Mass source | *(simplified away)* | mass is **always produced by the decode** (in the Rust kernel) — from per-pixel axis values or from uniform z-slice components; one code path. `requires_per_pixel_mass` survives only as CPU-side metadata saying whether `SimUniforms.m[3]` is trustworthy for CPU consumers |

### Fragment side

| Degree of freedom | Baked / uniform | Why |
|---|---|---|
| **Stain graph** (node occupants + wiring) | **BAKED** = the fragment key; changing a node or a wire = async recompile with last-valid fallback (imperceptible; render switching is not on the gesture path) |
| Debug field views | **BAKED, generated, on demand** | one tiny source per field from the catalogue generator; compiling a mega-switch over heterogeneous field types would be the interpreter anti-pattern |
| Slot uniforms (κ, C, swatches, L-range, invert) | **UNIFORM** | schema-driven, already specced |
| View-only display state | **UNIFORM** | render key; free to animate (the playhead is the frame loop's clock — sim-side march, not a render uniform; the time scrubber sets the display time and re-integrates, R-66) |
| Compositor (backdrop render target, separable blur ×2, composite) | **FIXED SHADERS** | above the stain graph; precompiled always; never varies |

---

## Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)

Part 3 bakes tier co-computations *out* for cost (line: the shadow state and ensemble copies cost occupancy even when branched off). That creates a hazard the compiler layer must resolve, because **the built-in slot occupants are themselves custom shaders through the same assembler** (Part 2) — they are the proof the devkit works. If built-ins read raw `SimState` fields and customs get a degraded API, the built-ins no longer dogfood the custom path, and there are two shader classes. So the read-side interface must be **identical** for built-in and custom, and it must not break when a tier bakes a feature out.

The resolution has two halves, and it rests on a property the payload already has: **the hot read-side fields are mostly *derived at read*, not stored** (`ftle = S_final/(n·dt)`, ensemble spread computed at the resolve stage, the substep-log proxy from `total_substeps`, drift from `E_0`). A derived field's "presence" is *whether its accessor computes a value or returns a sentinel* — which costs **zero bytes** either way.

**1. The read-side `SimState` type is fixed across all variants — plain field syntax, no getters.** Every field is always declared; authors (built-in and custom alike) write `sample.ftle`, `sample.ensemble_spread`, `sample.d_min` — plain member access, uniform everywhere. There is no `sample_ftle(ctx)` getter pattern (gross for what is conceptually a field read, and it would infect even always-present fields). What varies per variant is not the *type* but what *fills* each field:

| Field | FTLE/ensemble/word ON | feature OFF (baked variant) | memory cost of "always present" |
|---|---|---|---|
| `sample.ftle` | accessor computes `finalise(S,δ,n)/(n·dt)` | accessor returns **NaN** (const) | **zero** — derived at read, never stored |
| `sample.ensemble_spread` | resolve-stage value from the E+1 samples | E=0 → **NaN** (absent; `has_ensemble` false — a *computed* zero spread is a real value only when E ≥ 1) | **zero** — derived at resolve, never a stored field |
| `sample.word` | reads the bound word buffer | buffer unbound → returns the **empty/sentinel word** | **zero** — the binding varies, not the struct; unbound = unallocated |

The expensive *state* is still baked out exactly as Part 3 says — the Benettin **shadow trajectory** (48 B, marched every step), the **ensemble copies**, the **word buffer binding** are all absent in the feature-off variant, preserving the occupancy/memory win. What Part 3a keeps uniform is only the cheap *read-side result* — a computed expression or a naming convention, never resurrected state. **Fixing the read-side interface inflates nothing**, because the always-present fields were never stored bytes to begin with.

**2. NaN is the sentinel, and it makes graceful degradation the default.** A tier-absent (derived) scalar reads as **NaN** at unpack, documented — storage never holds NaN (R-79) — for two reasons that fall out of IEEE arithmetic for free:

- **Unguarded use degrades loudly, not silently — as best-effort behaviour, not a guarantee.** `mix(a, b, sample.ftle)` with `ftle = NaN` propagates → the pixel renders visibly wrong (black/garbage) on real hardware, so the author *sees* the feature is off rather than shipping plausible-but-fake output. This is why NaN beats `-1.0`: a negative FTLE is a *plausible number* a shader might quietly use — the fake-looking-real hazard a research instrument must avoid. **The sentinel's job is to make absence undeniable, and NaN is the only value that can't be mistaken for a measurement.** The honest WGSL caveat (render contract Part 4): fast-math may legally assume no-NaN, so **propagation is loud-by-default, never load-bearing** — no correctness logic may depend on it or on `isnan()`. The reliable mechanisms are the consts and predicates below; the NaN is the default for authors who use neither.
- **Guarded fallback is clean, free, and reliable.** Alongside each feature the assembler bakes a **`const bool has_<feature>`** (`has_ftle`, `has_ensemble`, `has_word`) — compile-time, dead-code-eliminated, zero runtime cost, and *not* subject to the fast-math caveat. An author who wants a graceful visual writes `if (has_ftle) { … } else { …fallback… }`; an author who does nothing gets the loud NaN. (If the absence sentinel itself must ever be tested at runtime, it is an exact **bitcast comparison** against the canonical quiet-NaN bit pattern — reliable where `isnan()` is not.)

**Two orthogonal questions, two reliable mechanisms — no sentinel collision.** Tier-absence surfaces as NaN in the *value*; per-sample failure surfaces as the defined failed-state values, never NaN (R-79). Both mean "don't use this", and each has its own reliable, non-NaN answer: **`has_<feature>`** answers "is this feature computed at this tier?" (compile-time const), and the **descriptor/validity predicates** answer "did *this sample* produce a usable value?" (`sd_is_failed` for sim/decode failure (canonical name, payload §6); the generated read-time predicates `ftle_valid`, slope-valid, etc. — kernel-set flags at the point of computation are the primary invalid-signal, per the render contract's storage rule). Full correctness is `has_ftle && ftle_valid(sample)` — **never `isnan()`**. An author doing nothing degrades loudly in both cases, which is correct; an author doing it properly never touches NaN at all.

**Net effect on the "should low tiers disable features at all?" question:** yes, and it carries no downstream cost. No shader forks (one read-side type), no getter ugliness (plain field syntax), no memory inflation (derived fields), and graceful degradation is the *default* (NaN is loud; `has_<feature>` is there for the careful). The built-ins use the identical interface, so they genuinely prove the custom-shader path. Feature-gating by tier is free at the interface exactly because it was already free at the payload — the derive-at-read design serves interface uniformity as a second dividend.

---

## Part 4 — The precompile rule

The responsiveness invariant demands no compilation on the gesture path. Under the substrate split (Part 2) the two sides reach this differently:

- **Compute side — no runtime source compilation exists.** Every `(chart × occupant × tier × links)` variant is monomorphised and compiled to SPIR-V by rust-gpu **at build time**; at runtime "precompile" means only creating the wgpu pipeline *object* from that pre-built SPIR-V (cheap — no source compiler runs). So the gesture-never-reaches-the-compiler invariant is *trivially* true for compute: there is no compute source compiler at runtime to reach. The cost this rule used to manage (runtime WGSL assembly + compile) has moved to **build time**, and reappears there as the **monomorphisation-set question** — which variant combinations to generate ahead of time (a real build-side budget, not a runtime one; bound it to the active/plausible set, not the full cross-product).
- **Fragment side — runtime WGSL compilation, as before.** Custom colour occupants and slot changes compile WGSL from source at runtime, so the async / last-valid-fallback machinery below applies **here**.

The rule that keeps the (fragment) precompile set tiny:

> **Anything whose change already costs a re-integration may compile lazily inside that cost. Only changes that must feel instant need precompilation.**

- **Chart switching must be instant** → the compute variant is already built (SPIR-V); create its pipeline object for the *active* (occupant, tier, links) at startup — ~10 objects.
- **Occupant / tier / link / threshold changes re-integrate anyway** (sim key) → their pipeline-object creation hides inside seconds of sim work; lazy. (No source compile — the SPIR-V already exists.)
- **Render presets** precompile (a handful); other slot combinations, all debug views, and all customs **compile WGSL on selection**, async, last-valid fallback (fragment side).
- **Compositor shaders** always precompiled (fixed WGSL set).
- **Startup budget:** ~10 compute pipeline objects + ~6 fragment + 3 compositor — precisely the window the spotlight reel covers.

Tilt, pan, slice, zoom, lock, playhead: **uniform writes only, by construction of Part 3.** A gesture can never reach a compiler — and on the compute side there is no runtime compiler to reach at all.

---

## Part 5 — The resolution function (the "switch", concretely)

*Illustrative pseudocode — `resolve` runs **engine-side (Rust/wasm)**, not in TS; the shape is what matters. `computeKey` selects a **pre-built monomorphised SPIR-V variant** (Part 4), not a runtime-assembled shader.*

```
function resolve(vs: ViewState, sk: SimKey, rc: RenderConfig): Lowered {
  const computeKey = hash(vs.chart.type, sk.links, sk.occupant, sk.tierBits);
  const fragmentKey = hash(canonical(rc.stainGraph));   // canonical graph form: defined by the task that needs it (R-72)

  const computeUniforms = {
    sim:   simUniformsFrom(sk),                    // T, dt, N_max, thresholds, eps, n_renorm
    chart: chartUniformsFrom(vs),                  // z₀, q₁, q₂, slice values,
                                                   // curve tangent / invariant targets if kinds demand
  };
  const fragmentUniforms = slotUniformsFrom(rc);   // schema-driven + playhead t

  const dispatch = quadsFor(vs).map(quad => ({
    pipeline: pipelines.get(computeKey),           // pre-built monomorphised SPIR-V variant (Part 4)
    quadUniforms: { c: quad.c, h: quad.h, x0: quad.x0, J: quad.J,
                    T: quad.T, decodeMode: (quad.collapsed || quad.depth > SWITCH) ? LIN : FULL },
                    // R-90: LIN once the full decoder's adjacent samples give bitwise-identical ICs
                    //   (quad.collapsed), or past SWITCH = ℓ_switch = 20, an upper bound — whichever first
    workgroups: samplesPerQuad(sk.tier),           // N×N per quad (memory-tiers §1)
  }));

  return { computeKey, computeUniforms, fragmentKey, fragmentUniforms, dispatch };
}
```

Chart validation (well-posedness, flags, feasibility declarations) runs **before** resolution, CPU-side, per the chart contract Part 5 — `resolve` only ever sees valid charts.

---

## Appendix — worked enumeration of the current chart set

The rules above, applied. Columns: axis kinds → which Φ map the chart monomorphises to and its extra uniforms; flags → what the CPU declares.

| Chart | Axis kinds | Φ map lowers to | Extra uniforms | Flags / lowering notes |
|---|---|---|---|---|
| **Latent affine slice** | raw × raw | affine map `z₀ + (2s−1)q₁ + (2t−1)q₂` | — | the base case; every navigation gesture is its uniforms |
| **Shape sphere (α, β)** | derived-in-block × derived-in-block (config) | block inverse-free direct: (s,t)→(α,β) ranges | — | `system_image: 2-to-1`; residual none (both config DOF swept) |
| **(L_z, E)** | invariant × invariant | domain warp → invariant construction (rigid `v^(L)` + seeded `a·w`) | frozen config, feasibility consts | `forbids_energy_normalisation`, `has_feasibility_boundary` — infeasible pixels **write tagged payloads** in-kernel |
| **(L_z, K)** | ditto | ditto with `K(t)=K_max t^{γ_K}` warp | `γ_K` | ditto |
| **Ternary mass** | raw × raw (mass block) | triangle warp → mass controls | frozen config/momentum slice | standard; no Burrau-specific code (lock supplies ν₀) |
| **Euclid ν plane** | curve × **annotation** | `ν(v)` → embed `γ: ν→(α,β,μ₁,μ₂)`; u-axis emits nothing | landmark params (display) | `annotation_axis`; 1-swept + 7-frozen; per-ν statistics only |
| **θ × K strip** | curve (or derived-config if decoupled) × invariant | `θ(s)` → embed (or config-only) + K solve | `γ_K`, coupling flag | curve tilt requires lock (tangent uniform `γ'` supplied at resolution) |
| **Burrau int lattice** | quantised curve × quantised curve | cell quantiser → validity check → `γ(n/m)` | `m_max` | `quantised`: **per-cell dispatch** (one decode+integrate per cell, broadcast), floor-guard exempt, counting measure, no tilt — lock-and-exit |
| **Anosova physical-frame** | physical × physical | physical (m,r,p) construction → **canonicalise entry** → standard pipeline | region D bounds | enters through the one seam; integrator unaware, as always |

Every row ends in the same `canonicalise → (m,r,p) → wrapper(occupant)`. The rows differ only in the Φ map and its uniforms — which is the whole point.

---

*One resolution function, two pipelines — a monomorphised Rust compute kernel and an assembled WGSL fragment shader — one payload between them. Code that differs gets baked; values that move get uniforms; branches that are workgroup-uniform are free. Instant things precompile; everything that re-integrates hides its pipeline creation inside the integration. A gesture can never reach a compiler, and no kernel ever learns what an axis kind is.*
