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

*10 Nov 2026 (R-926).* The chart map and the decode no longer lower into the compute pipeline: the CPU evaluates them in f64 and sends each quad its forward table, from which the integrator's records are formed, and the compute pipeline integrates those records through the wrapper and the occupant, with no chart code in the integrator (R-926, items 5 and 6). The fragment pipeline keeps the chart's generated decode and encode where the chart has closed-form shader code, and reads CPU-built tables where it has none (R-925). The paragraph above is kept as written for what it says of the fragment side and of `SimState` and `ICDescriptor`; its compute-side list is *Was*.

*13 Nov 2026 (R-989, RQ-797 Q1 (a)):* the records are formed on the CPU: an exact quad's from each sample's f64 evaluation, narrowed once, and past R-90's switch a patched quad's from each sample's `x₀ + J_D·δ` expanded in f64 and narrowed once (R-990's check deciding which quads take the patch); a tier-3 map's record holds the sample's own f64 evaluation, narrowed once (Q2 (i)). The GPU reads only finished records, and no `x₀` or `J_D` reaches the integrator.

### Composed charts — the CPU evaluation that was "CPU batch", and the cap

*15 Nov 2026 (TASK-M2-59; REQ-CHART-094): ported from `principia_chart_composition.md` § 7 and § 11, as R-837, R-839,
R-872, R-925, R-926, R-935 and R-1012 rule them.* The composition design gave a composed pair three ways to run:
**uniform affine** (both axes latent directions, `Latent`'s uniforms), a **baked entry** (the gallery's pairs and those
inside the cap, each a compute entry) and **CPU batch** (everything else: f64 on the CPU, each start rounded once to
f32, the IC buffer filled, then the integrator, a re-render per view, labelled). Under R-926 (items 5 and 6) the third is
how **every** view is filled, live or not, the coarse cover in motion and refinement at rest: the CPU evaluates every
chart in f64 and the integrator reads the narrowed records, with no chart code in the integrator and no inverse on that
path (Part 1's notes above). So the three are no longer three integrator routes. What stays of the first two is on the
fragment side: a chart with closed-form code takes its generated WGSL, one with none the table route, never a refusal
(R-925 item 1), and the chart's label is that fragment route, `closed form` or `table` (R-935, RQ-765 (a);
`principia_chart_decoder_contract.md` Part 5). `axis kinds` never reach the kernel, and nothing dispatches on them
(REQ-CHART-004).

- **CPU batch's records.** The records the CPU writes are the words `stage::decode` writes, `write_ic`'s `(m, r, p)`
  per sample, "the stage's only output to the integrator side", as R-669's Path B batch writes BZ-1C's starts. A
  composed chart's record adds a `branch` word per sample, closed-form or numerical
  (`principia_chart_reference.md` § 6.7), read by the resolve pass to flag a copy that straddles a seam
  (`principia_dd_refinement_policy.md` § 1): a schema change, which bumps the payload schema version and invalidates
  cached payloads, its layout set where the record is defined (R-72). A chart of today's `kernel::chart::REGISTRY` carries
  no `branch` and is bitwise today's. *15 Nov 2026 (TASK-M2-87; REQ-INT-115):* the record's full content, where it is
  formed and which quads take the patch are Part 3's "The record and table contract (CPU to GPU)"; this bullet stands.
- **Seam 3.** R-839 (RQ-719 (a), 7 Nov 2026) made a labelled CPU-batch view core seam 3's one stated exception, writing
  the IC buffer the decode stage writes and never reached by a gesture. R-926 supersedes it: every view is filled from
  the CPU's f64 evaluation narrowed to the integrator's records, so no CPU-filled view is a separate route
  (`principia_core_design.md` § 3).
- **The browser.** Every chart runs as natively: filled from the CPU's f64 evaluation in the engine's worker, the same
  Rust compiled to WebAssembly, a coarse cover in motion and the exact view at rest, with no route label of its own
  (R-1012, RQ-817 (a); R-889's "labelled and not live" reads "as natively"). The share of charts whose wasm evaluation of a
  coarse cover fits one frame at 60 Hz is measured and reported, labelling nothing (TASK-M8-102, REQ-CHART-155).
- **The cap** (R-837, RQ-716 (a)). The GPU decode, canonicalise and encode entries stay as test entries, the cross-backend
  parity checks and the agreement presets that compare the fragment's generated code (R-935, RQ-765 (a)). Their cap is 4
  baked entries beyond the gallery, counted as entries (a pair under two link sets is two), decode-only, created on first
  use; a first use may take a compile, accepted as an exception to REQ-PERF-006's "every chart at startup". `SuvakovPlane`
  counts as one of the 4. The value lives in one ledger constant, tested by `baked_cap`. Its criterion is the composition
  design's § 11, C1 to C5, written before the first measurement; C3's 2 s start-up limit is the Mac's, issue #427 setting
  any Vulkan and Windows budget from its own measurements (R-872). R-837's "past the cap a pair runs as CPU batch,
  labelled" is moot under R-926, every chart being evaluated on the CPU. The cap and REQ-PERF-116 are retired as moot
  once the fragment pipeline's compile time and module size with every generated chart section have been measured, on
  the Mac and on lavapipe, against C3's limit, the criterion as R-837's measurement wrote it; until that measurement is
  reported the cap stands (R-935).

---

## Part 2 — Two assembly mechanisms (the substrate split; the old "one mechanism" claim retires)

> **Superseded by the substrate decision (`principia_spike_brief.md`: whole engine in Rust → wasm, physics kernel shared CPU/GPU via rust-gpu).** The two pipelines no longer share an *assembly mechanism*. They never shared *code* — the compute side is physics, the fragment side is colour — only the string-assembler; and the shared-source Rust kernel means the compute side is **not** assembled from WGSL strings at all. This is a real cost booked honestly (the previous elegance of one hot-reload path for both), bought back by something worth more: the physics kernel is single-sourced, so CPU/GPU logic *cannot drift* (parity contract §6). The split is chosen precisely so the parity-critical side gets shared source while the parity-free side keeps its runtime devkit.

**Compute pipeline — monomorphised Rust, not string assembly.** The chart map Φ, decode, canonicalise, wrapper, and occupant (its seam is `ADVANCE`, integrator contract Part 2a, R-19) are one Rust kernel generic over the float type and over chart/occupant (Rust generics + `#[cfg]`/trait selection), compiled by rust-gpu to SPIR-V and — the *same source* — to the CPU-f64 reference. "Charts lower, they do not interpret" (axiom 4) is served *better* by monomorphisation than by string-splicing: a chart is a type parameter the compiler specialises, not a snippet concatenated. Generated pack/unpack (from the layout table) and link maps (from the registry) remain generated, now as Rust the kernel calls rather than WGSL prelude. **Consequence — no runtime-authored custom compute occupants:** you cannot compile user Rust in the browser, so a user's experimental integrator/Φ is a build-time variant, not a text-box occupant. This is an accepted loss (niche — few users write their own symplectic integrator; the custom-*colour* devkit, which is the centrepiece, is preserved below).

**Fragment pipeline — unchanged: hand-WGSL keyed-snippet assembly.** The colour side stays the render doc's flow — the stain graph's keyed WGSL snippets (one function per node) concatenated with the shared prelude and a generated `shade()` that walks the graph (R-64; render_gui_spec Part II §10.1), hashed, compiled async, cached, last-valid-on-failure — because colour has **no f64/f32 parity stakes** (parity contract §6) and *needs* its runtime path: this is what keeps the custom-shader devkit (GUI §3–5, lowering Part 3a) alive, where built-ins dogfood the custom path as one mechanism. The one exception to hand-written WGSL on this side is the decode/encode the fragment recomputes with: it is generated from the one Rust source (rust-gpu → SPIR-V → WGSL translation), never hand-written, and the agreement presets check that translation (R-116).
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

<!-- retired-terms -->
| Degree of freedom | Baked / uniform | Why |
|---|---|---|
| **Chart type** (axis kinds, Φ structure) | **BAKED** | genuinely different code (raw assignment vs invariant solve vs curve embed vs physical-frame entry). Axiom 4 |
| **Link selection** per block | **BAKED** (monomorphised) | different transcendentals; registry is small; the link *function* comes from the registry (generated Rust) |
| **Integrator occupant** | **BAKED** | never branch per-step on occupant; profiles differ structurally |
| **Tier co-computations** (FTLE shadow) | **BAKED** | it adds per-thread *state* (each sample's tangent vector) — present-but-unused state costs registers/occupancy even when branched off |
| **Ensemble copy** (`copy_index`) | **UNIFORM** | not a baked variant: each copy is the same kernel dispatched again, with `copy_index` as a uniform (R-102, R-89) — no per-thread state added |
| **Decode mode** (full vs linearised) | **UNIFORM** (per-quad flag) | the branch is workgroup-uniform (whole quad shares the mode) → free on GPU; halves the variant set; matches the `QuadRequest.flags` design (scheduler contract Part 5). *Promotion path documented:* if the dead full-decode path measurably hurts deep-quad occupancy via register pressure, it becomes a baked variant — a one-line key change under the assembler. (Note: a workgroup-uniform two-path branch is **not** a runtime axis-kind interpreter; axiom 4 is untouched) |
| Kernel bring-up mode (the one kernel debug mode — `principia_colour_composition.md` Appendix A; R-75) | **BAKED** (pre-built SPIR-V; pipeline created on demand — no runtime source compile, Part 4) | different code entirely (writes a known pattern instead of physics), but debugging is not gesture-critical → async pipeline creation on first use. UV / DECODE / ROUNDTRIP are fragment presets (colour_composition §6), not kernel variants |
| Wrapper config (`T, dt, N_max, r_*, eps_*, n_renorm`) | **UNIFORM** (`SimUniforms`) | sim-key *values*, not code. The `N_sub` threshold table is generated from them on the host, per sim key, and travels in `SimUniforms` as exact f32 bit patterns, read by constant index (R-502; integrator dd §3.3). (`n_renorm` is the Benettin renorm interval — the old checkpoint count `M` no longer exists; integrator dd §6) |
| Chart params (`z₀, q₁, q₂`, slice values, curve tangent `γ'(ν₀)`, invariant targets) | **UNIFORM** | **navigation is uniform edits** — the entire Part-4 navigation contract depends on this; no gesture ever compiles |
| Per-quad (`c, h, x₀, J_D`, quad `T`, decode flag) | **PER-QUAD UNIFORM** | deep-zoom doc |
| Mass source | *(simplified away)* | mass is **always produced by the decode** (in the Rust kernel) — from per-pixel axis values or from uniform z-slice components; one code path. `requires_per_pixel_mass` survives only as CPU-side metadata saying whether `SimUniforms.m[3]` is trustworthy for CPU consumers |
<!-- /retired-terms -->

*10 Nov 2026 (R-926).* Under R-926 the rows above that concern the chart (chart type, link selection, decode mode, chart params and the per-quad `x₀`, `J_D`) govern the CPU's f64 evaluation and the fragment's generated code, not the compute pipeline: navigation is a re-evaluation of the chart on the CPU and an upload of the quads' forward tables, never a compile; the per-quad values travel with the quad as its forward table, a one-cell table (a centre and slopes) being the linearised decoder (R-926, items 6 and 11). The rows themselves are kept as written until the plan's tasks rewrite this Part; the integrator's rows (occupant, tier co-computations, ensemble copy, bring-up mode, wrapper config) stand.

*15 Nov 2026 (TASK-M2-87; REQ-INT-115; R-989, RQ-797 Q1 (a)).* The per-quad values reach the compute pipeline only as the records the CPU forms from them: no `x₀` or `J_D` is uploaded for the integrator (the record and table contract, below). So navigation uploads, for the integrator, the quads' records; for the compute pipeline the row "Per-quad (`c, h, x₀, J_D`, quad `T`, decode flag)" reads: the quad's records, `T` staying the per-dispatch uniform (R-935 under RQ-761); and the row "Mass source" reads: the record's `m`, formed on the CPU. Both rows stand as written for the CPU's evaluation. *Was (the note above, its two clauses):* "an upload of the quads' forward tables" and "the per-quad values travel with the quad as its forward table, a one-cell table (a centre and slopes) being the linearised decoder (R-926, items 6 and 11)."

### The record and table contract (CPU to GPU)

*15 Nov 2026 (TASK-M2-87; REQ-INT-115; R-926 items 5 and 6, R-935, R-985, R-989, R-990, R-995, R-1006).* The compute pipeline's one input from the chart is a buffer of **records**, one per sample, which the CPU writes. The integrator's entry reads records, never decodes and holds no chart code (R-926 item 5: "The CPU evaluates charts in f64. The GPU only integrates records. No chart code in the integrator."). That the built entry does so is checked in code by REQ-INT-116, not by this text.

**The record's content.** Each record holds:

- REQ-DEC-005's `(m, r, p)`, the canonical masses, positions and momenta in `G = M = I = 1`, the integrator's one input type whichever route formed it (`principia_chart_decoder_contract.md` Part 2, "the one hard output rule");
- the descriptor and tag words;
- a branch word, closed-form or numerical (Part 1's "CPU batch's records"; `principia_chart_reference.md` § 6.7);
- the stop index `k_stop ≤ horizon_steps`, a `u32`, the host mapping each sample's target time to the latest boundary at or before it in integers on the occupant's grid, `T` staying the per-dispatch uniform and `bounded` the horizon's (R-935 under RQ-761). On a view tilted toward time, with the playhead running, only this word is rewritten each frame, from the pixel and the playhead on the host, and the record's other words are unchanged (R-985; scheduler contract Part 7);
- its route, exact or patch, and for a patch the quad's level and address, so that a re-run re-forms the patch and reproduces exactly what was simulated, on every machine with R-991's pinned maths library (R-995, RQ-801 (a)). Whether a record made before R-926's route re-runs bit for bit is RQ-783, open (R-956). *17 Nov 2026 (R-1023, RQ-783 (a)):* it does: a record written before the route lands recreates through today's f32 decode, kept as product code for that recreation (a named exception to R-926 item 5, on the CPU), bit for bit, and each record notes which route wrote it, how being an implementation choice under R-521 (TASK-M7-45).

This fixes the record's content, not its word order or packing, which are set where the record type is built (R-72; Part 1's "CPU batch's records").

**`SimState` is a separate buffer.** The record is the sample's starting state, never its running one: the march updates each sample's `SimState` in place, with no double-buffering (REQ-SCHED-012), and a re-boot from `t = 0` starts again from the record (R-565, bitwise; R-935 under RQ-761).

**Where a record is formed: on the CPU, in f64, narrowed once** (R-926 item 5; R-989). Every record is formed on the CPU in f64 and narrowed once to f32; the GPU reads only finished records.

- **An exact sample's** record is its own f64 evaluation of the chart, narrowed once.
- **A patched sample's** record, past R-90's switch on a quad R-990's check admits, is its `x₀ + J_D·δ` expanded in f64 and narrowed once (RQ-797 Q1 (a)), the correctly rounded value of its f64 sum. It carries the quad centre's descriptor, tag and branch words, which are right on every patched quad, since a quad whose samples could differ from the centre's in tag, branch or sheet fails R-990's check and is exact per sample. No `x₀` or `J_D` is uploaded for the integrator; collapse and `AT_F32_FLOOR` are read on the CPU from the narrowed records, with nothing read back (R-142; scheduler contract Part 4).
- **A tier-3 map's** sample (a map with no exact or implicit slopes, R-926 item 9) takes no one-cell patch: its record holds the sample's own f64 evaluation, narrowed once, as an exact record does (RQ-797 Q2 (i)). The CPU may form the sample's chart input quad-locally, the quad's centre plus the sample's offset, in f64, before the evaluation; the difference from the centre enters the CPU's evaluation, never the record or the GPU.

**Which quads take the patch** (R-990, RQ-798 (a″); REQ-DEC-063). Past R-90's switch a quad takes the one-cell patch only where this check admits it, and is exact per sample elsewhere, at any depth:

- at its four corners and four edge midpoints, the exact f64 evaluation and the patch's f64 prediction differ, in every component, by at most 4/5 of the bound, the bound being half an f32 ulp of the component's own value, so that admitted records are within one f32 step of exact; and the two agree in tag, branch and sheet;
- its footprint meets no site REQ-DEC-058 flags (REQ-CHART-129's per-footprint flag and REQ-CHART-130's `Seam` mask, extended to the switch's footprint test);
- where a component crosses zero inside the quad, that component is compared at the f32 ulp of the quad's smallest value of that component, not at the checked points' own values, that value and how a crossing is detected being REQ-DEC-064's definition; and where a component's samples drop a binade below the checked points' without crossing zero, or the checked points themselves span two binades, that component is compared at the f32 ulp of the quad's smallest sample magnitude (R-1006, RQ-811 (b)), so that every admitted sample is within one f32 step of exact.

**The forward table** (R-926 items 6 and 11; R-935 under RQ-758 and RQ-766). Each quad's forward table, window position to state, is built on the CPU, and no inverse is anywhere on that path. Above R-90's switch it has one exact node per sample; past it, on a quad R-990's check admits, it is one cell, the centre `x₀` and the slopes `J_D`, which is the linearised decoder, the slopes a tier-1 (exact, dual numbers) or tier-2 (implicit) map's, with central differences at R-633's step only their check. R-90's "adjacent samples give bitwise-identical ICs" compares the narrowed f32 records, the starting states the integrator reads. What the integrator receives from the table is the records formed from it (R-989).

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
| `sample.ftle` | accessor computes `finalise(S,δ,n)/(n·dt)`, and NaN whenever `ftle_valid` is false (R-254) | accessor returns **NaN** (const) | **zero** — derived at read, never stored |
| `sample.ensemble_spread` | resolve-stage value from the E+1 samples | E=0 → **NaN** (absent; `has_ensemble` false — a *computed* zero spread is a real value only when E ≥ 1) | **zero** — derived at resolve, never a stored field |
| `sample.word` | reads the bound word buffer | buffer unbound → returns the **empty/sentinel word** | **zero** — the binding varies, not the struct; unbound = unallocated |

The expensive *state* is still baked out exactly as Part 3 says — the Benettin **shadow trajectory** (48 B, marched every step) and the **word buffer binding** are absent in the feature-off variant, preserving the occupancy/memory win; the **ensemble copies** are not a variant — at E = 0 they are simply not dispatched (R-102). What Part 3a keeps uniform is only the cheap *read-side result* — a computed expression or a naming convention, never resurrected state. **Fixing the read-side interface inflates nothing**, because the always-present fields were never stored bytes to begin with.

*Note (R-343):* the stored buffers this Part's read side is filled from are bound by the generated unpack layer (render contract Part 5, "Unpack layer"): `SimStateFTLE` at `@group(1) @binding(0)` and the word buffer at `@group(1) @binding(1)`, group 0 reserved for the assembler's per-frame uniforms, the numbers generated constants from one ledger table. Every read of either buffer goes through `sample_state(i)` or `sample_word(i)`; the read-side `sample` fields above are filled from those. The generated layer declares the full tier; the feature-off variants (`SimStateBase`, no word binding) and the `has_<feature>` consts are this Part's per-tier assembly, M1's. An absence or unset test on a value read here compares bit patterns, never `isinf` or `isnan`, as the bitcast comparison below does, because fast-math (R-297) may optimise those away.

*Note (R-378):* the read side loads only the stored members each field needs (`simstate_buffer[i].packed_a`, `simstate_buffer[i].S`, …), never the whole stored struct in one load, so unused data is never fetched on any backend; R-378 replaces the whole-struct `sample_state(i)` above, and the fixed read-side type with plain members stands. A word field likewise loads only the components of `word_buffer[i]` it needs (applied per R-369).

**2. NaN is the sentinel, and it makes graceful degradation the default.** A tier-absent (derived) scalar reads as **NaN** at unpack, documented — storage never holds NaN (R-79) — for two reasons that fall out of IEEE arithmetic for free:

- **Unguarded use degrades loudly, not silently — as best-effort behaviour, not a guarantee.** `mix(a, b, sample.ftle)` with `ftle = NaN` propagates → the pixel renders visibly wrong (black/garbage) on real hardware, so the author *sees* the feature is off rather than shipping plausible-but-fake output. This is why NaN beats `-1.0`: a negative FTLE is a *plausible number* a shader might quietly use — the fake-looking-real hazard a research instrument must avoid. **The sentinel's job is to make absence undeniable, and NaN is the only value that can't be mistaken for a measurement.** The honest WGSL caveat (render contract Part 4): fast-math may legally assume no-NaN, so **propagation is loud-by-default, never load-bearing** — no correctness logic may depend on it or on `isnan()`. The reliable mechanisms are the consts and predicates below; the NaN is the default for authors who use neither.
- **Guarded fallback is clean, free, and reliable.** Alongside each feature the assembler bakes a **`const bool has_<feature>`** (`has_ftle`, `has_word`) — compile-time, dead-code-eliminated, zero runtime cost, and *not* subject to the fast-math caveat. **`has_ensemble` is the exception: the fragment side reads it as a uniform, like the compute side's `copy_index` (R-102), so toggling E never re-bakes; E = 0 still reads `ensemble_spread` as NaN (R-145).** An author who wants a graceful visual writes `if (has_ftle) { … } else { …fallback… }`; an author who does nothing gets the loud NaN. (If the absence sentinel itself must ever be tested at runtime, it is an exact **bitcast comparison** against the canonical quiet-NaN bit pattern — reliable where `isnan()` is not.)

**The sentinels' values (R-72; REQ-RENDER-077).** The canonical quiet NaN is the f32 bit pattern **`0x7FC00000`**:
sign 0, exponent all ones, the quiet bit (bit 22) set and the rest of the significand zero, IEEE 754's default quiet NaN
and Rust's `f32::NAN`. A tier-absent derived scalar reads exactly these bits at unpack (`ftle` in a no-FTLE variant,
`ensemble_spread` at E = 0), and so does an invalid read (`ftle` whenever `ftle_valid` is false, R-254; `diffusion` for
`n < 2`, R-245); the bitcast absence test compares against them. The generated read side writes the pattern from its
bits (`CANONICAL_QNAN_BITS`), never as the result of arithmetic, whose NaN sign and payload IEEE leaves to the
implementation. An unbound word buffer reads the word **`vec4<u32>(0u, 0u, 0u, 0xFE000000u)`** (`FGW_UNBOUND`): payload
zero and `length_raw` 127, the truncation sentinel (payload §3). Every word-derived validity predicate reads it as
invalid (`fgw_reduced_length_valid` and `sd_last_symbol_valid` are false), so an absent word cannot read as a measured
one, as the empty word (length 0, no net crossings, a real result) would. `has_word` tells absence from truncation.

**Two orthogonal questions, two reliable mechanisms — no sentinel collision.** Tier-absence surfaces as NaN in the *value*; per-sample failure surfaces as the defined failed-state values, never NaN (R-79) — except `ftle`, which reads NaN whenever `ftle_valid` is false, a failed sample included (R-254). Every aggregate over `ftle` (footprint means and spreads, quad reductions, histograms, statistics) excludes samples by `ftle_valid` explicitly and never relies on NaN propagation: one NaN poisons a sum or mean, and WGSL's `min`/`max`/`clamp` with NaN operands are implementation-defined, a parity hazard (R-255). Both mean "don't use this", and each has its own reliable, non-NaN answer: **`has_<feature>`** answers "is this feature computed at this tier?" (compile-time const), and the **descriptor/validity predicates** answer "did *this sample* produce a usable value?" (`sd_is_failed` for sim/decode failure (canonical name, payload §6); the generated read-time predicates `ftle_valid`, slope-valid, etc. — kernel-set flags at the point of computation are the primary invalid-signal, per the render contract's storage rule). Full correctness is `has_ftle && ftle_valid(sample)` — **never `isnan()`**. An author doing nothing degrades loudly in both cases, which is correct; an author doing it properly never touches NaN at all.

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
                    // quad.collapsed: the per-quad flag set when the full decoder's adjacent samples give
                    //   bitwise-identical ICs (R-90; scheduler Part 5, DECODE_MODE)
    workgroups: samplesPerQuad(sk.tier),           // N×N per quad (memory-tiers §1)
  }));

  return { computeKey, computeUniforms, fragmentKey, fragmentUniforms, dispatch };
}
```

*12 Nov 2026 (R-926 item 5; the chart audit's F7):* the CPU evaluates the chart in f64 and the integrator reads records; the compute-side lines here (`chartUniformsFrom` among the compute uniforms, the per-quad `x0`, `J` and `decodeMode`) are *Was* until TASK-M2-59 and TASK-M2-60 rewrite this part.

**The stain graph's canonical form (R-72; REQ-RENDER-075; TASK-M1-04).** `canonical(rc.stainGraph)` is the graph as
it lowers, whatever order it was built in: node ids, the order nodes and wires were added in and canvas positions are
not in it, so two constructions of one graph have one form.
- **Live nodes.** The form holds the nodes `OUT` depends on. Walking back from `OUT`: its colour input is the chain's
  last post or the combiner, and each post's colour input the post before it or the combiner; the combiner's two
  inputs are the colour and the brightness nodes wired to them; a colour, brightness or post node's field inputs are
  the sources wired to them. A node that is the identity (render_gui_spec §13; colour_composition §4.1), its occupant
  None or one of its field inputs absent or fed by a None source, is not in the form: the input it would feed reads as
  absent, and its own inputs are not followed. An identity post drops out of the chain, and a post's or `OUT`'s absent
  colour input reads the combiner, the chain before it empty. A node nothing live reads cannot change a pixel and is
  not in the form.
- **Order.** The sources, each once, in order of first use, reading the field inputs in port order of the colour, the
  brightness, then each post in chain order; then the colour, the brightness, the combiner, the posts from the
  combiner to `OUT`, and `OUT`. Every wire runs forward in this order.
- **Each node** is `{"kind", "occupant", "inputs"}`. `kind` is `source`, `colour`, `brightness`, `combiner`, `post` or
  `out`. `occupant` is `{"field": <member>}` for the built-in source of a read-side field, `{"builtin": <id>}`,
  `{"custom": <text>}` with the custom WGSL exactly as written, or `null` for `OUT`. `inputs` has one entry per in-port
  in port order (a post's colour first, then its field inputs; the combiner's colour, then its brightness): the
  feeding node's position in the order, or `null` where the input is absent.
- **The post chain** is encoded by the posts' positions, in chain order, each post's first input the position of the
  node before it; it holds at most 8 posts (colour_composition §4.2).
- **Text and hash.** The form is the JSON object `{"nodes": [...]}` in the one canonical serialisation, JCS
  (gui_state_contract §2, R-318). `fragmentKey` is the 64-bit FNV-1a hash of its UTF-8 bytes, R-36's hash.
- **The tier is not in it.** The fragment key identifies the graph; a compiled pipeline is identified by the fragment
  key plus the tier bits, `has_ftle` and `has_word`, which the assembled source bakes (Part 3a) and which choose the
  read side's stored struct and whether the word buffer is bound. `has_ensemble` is excluded: the fragment side reads
  it as a uniform (R-145).
- **The params are not in it.** A node's params are the values of its occupant's `uniformSchema` (gui_state_contract
  §3), uniforms (Part 3, fragment side), so a slider edit rebinds and never changes the fragment key. They are in the
  render key (render contract Part 3).

Chart validation (well-posedness, flags, feasibility declarations) runs **before** resolution, CPU-side, per the chart contract Part 5 — `resolve` only ever sees valid charts.

---

## Appendix — worked enumeration of the current chart set

The rules above, applied. Columns: axis kinds → which Φ map the chart monomorphises to and its extra uniforms; flags → what the CPU declares.

| Chart | Axis kinds | Φ map lowers to | Extra uniforms | Flags / lowering notes |
|---|---|---|---|---|
| **Latent affine slice** | raw × raw | affine map `z₀ + (2s−1)q₁ + (2t−1)q₂` | — | the base case; every navigation gesture is its uniforms |
| **Shape sphere (θ, φ)** | derived-in-block × derived-in-block (config) | block inverse-free direct: (s,t)→(θ,φ) by R-14's map (`principia_chart_reference.md` §3.3: θ = 2π·s, φ = π·(1 − t)) → n → (ρ̃, λ̃) by §3.2 (R-117) | — | `system_image: bijective`, the whole sphere, each pixel its own system, its decode ending in canonicalise with step 3, the mirror, not taken (R-544; physics review 5603309678 of PR #273, P3; *was:* "`system_image: n-to-1`, n = 2 (2-to-1 over the φ hemispheres, which decode to the same system, R-141)"); residual none (both config DOF swept) |
| **(L_z, E)** | invariant × invariant | domain warp → invariant construction (rigid `v^(L)` + seeded `a·w`) | frozen config, feasibility consts | `forbids_energy_normalisation`, `has_feasibility_boundary` — infeasible pixels **write tagged payloads** in-kernel |
| **(L_z, K)** | ditto | ditto with `K(t)=K_max t^{γ_K}` warp | `γ_K` | ditto |
| **Ternary mass** | raw × raw (mass block) | triangle warp → mass controls | frozen config/momentum slice | standard; no Burrau-specific code (lock supplies ν₀) |
| **Euclid ν plane** | curve × **annotation** | `ν(v)` → embed `γ: ν→(α,β,μ₁,μ₂)`; u-axis emits nothing | landmark params (display) | `annotation_axis`; 1-swept + 7-frozen; per-ν statistics only |
| **θ × K strip** | curve (or derived-config if decoupled) × invariant | `θ(s)` → embed (or config-only) + K solve | `γ_K`, coupling flag | curve tilt requires lock (tangent uniform `γ'` supplied at resolution) |
| **Burrau int lattice** | quantised curve × quantised curve | cell quantiser → validity check → `γ(n/m)` | `m_max` | `quantised`: **per-cell dispatch** (one decode+integrate per cell, broadcast), floor-guard exempt, counting measure, no tilt — lock-and-exit |
| **Anosova physical-frame** | derived-in-block × derived-in-block (config): the third-body pair `(ξ, η)`, residual none, as the shape sphere's row (R-834; `principia_chart_reference.md` § 6.6; *15 Nov 2026, TASK-M2-59; was:* "physical × physical") | physical (m,r,p) construction → **canonicalise entry** → standard pipeline | — (R-871: no region-D mask uniform, the full disc unmasked; *was:* "region D bounds") | enters through the one seam; integrator unaware, as always; region D's third-body pair, lowered by TASK-M4-34 through `region_d_variant` with the signed canonicalisation, never folded to `(ξ, −η)` (R-914; REQ-CHART-165) |

Every row ends in the same `canonicalise → (m,r,p) → wrapper(occupant)`. The rows differ only in the Φ map and its uniforms — which is the whole point.

*12 Nov 2026 (R-926 item 5; the chart audit's F7):* the CPU evaluates the chart in f64 and the integrator reads records; the compute-side lines here (the appendix's Φ maps lowered into the compute kernel, and the invariant charts' "infeasible pixels **write tagged payloads** in-kernel") are *Was* until TASK-M2-59 and TASK-M2-60 rewrite this part.

---

*One resolution function, two pipelines — a monomorphised Rust compute kernel and an assembled WGSL fragment shader — one payload between them. Code that differs gets baked; values that move get uniforms; branches that are workgroup-uniform are free. Instant things precompile; everything that re-integrates hides its pipeline creation inside the integration. A gesture can never reach a compiler, and no kernel ever learns what an axis kind is.*
