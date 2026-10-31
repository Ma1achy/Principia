# Principia — render pipeline contract

*Fifth doc. Sits beside the chart & decoder contract; consumes its output. Covers: the render payload, the stain graph (a fixed backbone with a free, typed interior), cache tiers and the recompute rule, semantic rules, the struct-inspection SDK, and the debug catalogue — which is the first thing built.*

---

## Part 1 — The payload (render input)

**One canonical payload.** The compute kernel maintains a `SimState` per sample — **the live state of a marching simulation at the playhead**, not a completed trajectory record (temporal-architecture note, ratified). The decode stage emits `ICDescriptor` per sample; the CPU supplies quad metadata. The fragment shader binds all of it. There is **no stored trajectory representation at all** — no checkpoint array, no ×M history. The shape vector `n(t)` is a *derived readout of the current state*, recomputed each step; everything temporal that persists is a **fixed-size running accumulator**.

Physical structs (the generation-root ledger is the single source of truth):

- **`SimState`** (tier-sized, O(1) in time — never grows with `t`; **8-byte aligned** now the word is out): the live phase state (`r, p` — 12 f32, **CoM-frame particle coords**, re-projected to CoM each step), **one tier-gated Benettin shadow** (12 f32 — resident under lockstep, marches too; the ingredient of this sample's own `ftle`), the running accumulators (Benettin `S`, unwrapped phase `θ̃`, **Welford diffusion** — per-sample `mean_y, C_ty`, with time-only `mean_t, C_tt, n` **derived closed-form, not a shared global**), the scalar block (`E_0, Lz_0` in f32 — refs for the drift-max latch, current drift *derived* as `H(r,p)−E_0`; `d_min, dE_max, dLz_max` in **f16**, packed into u32 words with the descriptor), packed `times` (`t_end_step, t_dmin_step` as **exact u16 macro-step indices**), the exact **`total_substeps` u32** (resumable cumulative-work counter — the log proxy is derived at read), and one bit-packed `sample_descriptor` (**10 bits used**: **`state`** enum [escape/bounded/collision/running/sim_failed/decode_failed], `detail`, `saturated` [stored sticky bit, set when `N_sub == N_max` occurred], `dmin_pair`, `last_symbol` [bits 8–9]; bits 10–15 reserved — payload §2, R-86; `trajectory_stats` eliminated — fields derived or moved). **The `free_group_word` is NOT in `SimState`** — it lives in a separate **word buffer** (§3.3/3.3a in ledger), indexed identically (per-copy), because it is the only *cold* field (per-crossing append + resolve/inspect read, never per-step); moving it out drops `SimState` to 8-align, removes 12 B of vec4-forced tail padding, and shrinks the hot per-sample footprint; with the 8 B closure pair (`closure_min` f32, `closure_step` u16 — payload §1) it is **144 B (FTLE-on) / 96 B (FTLE-off)** effective (recomputed from 136 / 88, R-40 / D6), as generated until TASK-M3-41, and **152 B / 104 B** from TASK-M3-41 on, with R-603's and R-605's two u32s (payload §1). **Every sample is this same full struct** (base and each ensemble copy — uniformity, sampling/SSAA note); ensemble copies are E *additional* such samples per nominal sample (Halton-(2,3)-offset, scheduler-leaves), not extra fields inside one struct. **Ensemble spread is NOT a per-sample field** — it is a footprint quantity derived at the resolve stage from a footprint's E+1 samples (the data-side twin of the SSAA colour resolve), consumed live for display and aggregated into `QuadReduction` for the scheduler; storing it per-sample would be wrong-shaped and cost an f32 ×`(E+1)`×viewport. On termination the sample **latches**: state stops advancing, accumulators freeze, `state` takes its terminal value. **Escape is the one terminal whose march may continue** (pitfalls §2.4's checks): once it fires, `state` reads escape and `t_end` is fixed, time averages (FTLE's `S/T` and the like) freeze at `t_esc`, and the further march writes nothing else (R-95).
- **`ICDescriptor`** (64 B at f32 — 12 × f32 plus explicit padding, R-86; its width follows `Real`, R-313): `m0 m1 m2, q_mass, rho_mag, lambda_mag, rho_ratio, rho_angle, K_0, V_0, virial_ratio, r_min_pair_0`. `E₀` is derived (`K_0 + V_0`), not stored (R-86). Written once post-decode, pre-integration. What the IC *is*, vs. what the trajectory *did*.
- **`RenderQuad`** (CPU-written): `quad_depth, quad_state, coherence_score, outcome_impurity, ensemble_spread, suspect_fraction, priority_score, ancestor_gap, cache_age`.
- **`RenderContext`**: `{sample: SimState, ic: ICDescriptor, quad: RenderQuad, uv, screen_uv, time}`.

**Physical vs logical — no unpack pass.** The render design doc's `RenderSample` with loose fields (`outcome_class: u32, …`) is the *logical* view. It is realised as **accessor functions over the physical structs**, not as a materialised unpacked struct. Never build an unpack-copy compute pass — it doubles memory for nothing. "Stable logical context" = the accessor layer (Part 5).

`SimState` and `ICDescriptor` never leave the GPU in normal operation; `QuadReduction` (one per quad; 176 B, aligned to 4, with u32 histogram bins: dd_generation_root §3.7, R-410) is the sole **automatic** GPU→CPU return of simulation data (the adaptive layer). R-288's per-frame telemetry readback is not simulation data (R-332). User-initiated pulls — the decoded export, and the click inspector's optional **on-demand single-IC f32 GPU trace** for the divergence overlay (`trajectory_viewing.md` §5: a one-pixel dispatch of the survey kernel writing a dense n(t) series) — are sanctioned, tiny, async operations outside that loop. (The hover/click **trace** is *not* a payload pull: it re-integrates the IC on the CPU via `computeIC`, touching no GPU buffer — Part 7.)

---

## Part 2 — Fixed pipeline, swappable slots

The stain is a **free, typed node graph** (R-64; `principia_render_gui_spec.md` Part II §3–§4, `principia_gui_state_contract.md` §5) over a fixed backbone. `combiner` and `OUT` are fixed singletons; any number of `source`, `colour`, `brightness` and `post` nodes are wired freely, subject only to port types and acyclicity; the tail is a variable-length post chain:

```
source(s) → colour?     ┐
                        ├→ combiner → (post)* → OUT
source(s) → brightness? ┘
```

| Node kind | Signature | Occupants |
|---|---|---|
| colour | `colour(ctx) → vec3` (linear RGB) | event classification, shape-sphere maps, diagnostic palettes, **debug views**, **custom source** |
| brightness | `brightness(ctx) → f32` (nominal [0,1]) | fixed, event time, diffusion, FTLE, ensemble spread, BC proximity, debug, custom |
| combiner | `combine(rgb, b) → vec3` | Principia default (OKLab replace-L), multiply, custom |
| post | `post(rgb) → vec3` (a chain of zero or more) | invert, structural overlays, custom (colour-vision simulation is not a post node — it is in the display stage, R-67) |

**A post node may read a render-side texture bound to it** (R-548, RQ-320): the Chazy "what just changed" view's
previous-class texture, written each frame, is the one such texture (Chazy drill-down §10.8.2). It is render-side
state, never a payload field; how the post node reads it beside this frame's coarse class is written here by the task
that builds the view (TASK-M7-40; R-72).

**Custom is not a node kind — it is an occupant.** Swapping a node's shader source is the *only* extension mechanism. Built-in, debug, and custom occupants share one compile path: full fragment source regenerated from the shared prelude + one function per node + the generated `shade()` that walks the graph (render_gui_spec Part II §10.1); schema-driven uniforms; hot reload with per-node failure isolation (a broken custom keeps the last valid source in that node; the other nodes are untouched). **Backbone order is constitutional** — sources feed colour and brightness, they meet only at the combiner, and the post chain runs after it; nobody feeds post back into colour (the graph is acyclic). **The built-in occupants are themselves customs through this path** — they are the proof it works, which is *why* the read-side interface below is identical for both.

**Reading `SimState` at any tier (the author contract).** The read-side `SimState` type is **fixed across all tiers** — every field is always present, read by plain member access (`sample.ftle`, `sample.ensemble_spread`), never a getter (lowering contract Part 3a). A tier that bakes a feature out does **not** remove the field; it makes the field read **NaN**. So:
- **You may read any field at any tier.** No guards are required to *compile* or *run* — a custom written against FTLE works at every tier.
- **Absent features read NaN, and unguarded use degrades loudly — as best-effort, not as a guarantee.** `mix(a, b, sample.ftle)` at a no-FTLE tier propagates NaN → visibly wrong pixel on real hardware. This is intentional loudness (absence is undeniable, never plausible-but-fake colour; NaN, not `-1`, precisely so it cannot be mistaken for a measurement) — **but WGSL does not guarantee it** (fast-math may assume no-NaN), so *no correctness logic may rely on NaN propagation or on `isnan()`* (Part 4's WGSL caveat). The reliable mechanisms are the next two bullets; the NaN is the loud default for authors who use neither.
- **Guard absence with the baked `has_<feature>` const** — compile-time, dead-code-eliminated, always reliable: `if (has_ftle) { …FTLE visual… } else { …plain visual… }`. `has_ftle`/`has_word` are baked consts; `has_ensemble` is a uniform, read the same way, so toggling E never re-bakes (R-145; E = 0 still reads `ensemble_spread` as NaN).
- **Guard per-sample validity with the descriptor predicates, never `isnan`:** `sd_is_failed(d)` (sim/decode failure) and the generated read-time validity predicates (`ftle_valid` — completed renorms > 0; `diffusion` slope-valid — n ≥ 2). Full correctness is `has_ftle && ftle_valid(sample)`. An aggregate over `ftle` (footprint mean or spread, quad reduction, histogram, statistic) excludes samples by `ftle_valid` explicitly, never by NaN propagation: one NaN poisons a sum, and WGSL's `min`/`max`/`clamp` with NaN operands are implementation-defined (R-255). (Testing the absence sentinel itself, if ever needed, is an exact **bitcast comparison** against the canonical quiet-NaN bits — reliable where `isnan()` is not.) Two separate questions, two reliable mechanisms: `has_<feature>` = computed at this tier?; descriptor/validity predicate = did *this sample* produce a usable value?

Internally, built-ins may work in OKLab/OKLCH; the public slot contract stays RGB + scalar.

**Presets are pure data:** a preset is a whole serialised graph — nodes, wires, per-node params and overlay toggles (render_gui_spec Part II §11). Selecting one replaces the stain wholesale. The GUI mock's preset chips are rows of this table.

---

## Part 3 — Cache tiers and the recompute rule

**The headline contract: changing a render mode never recomputes sim data.** Render changes only reinterpret the current `SimState`. Two clocks, cleanly split: **advancing the playhead is sim work** (the frame loop marches the live set — scheduler contract); **changing how the state is coloured is render work** (free). The playhead is a live clock, not a render uniform: the dev GUI's time scrubber sets the display time and re-integrates progressively, never replaying stored frames; "no scrub" applies to exported animations only (R-66).

| Tier | Contents | Invalidated by (its key) | Cost |
|---|---|---|---|
| **Sim buffers** | `SimState[]` (live, at the playhead), `ICDescriptor[]` per quad | **sim key**: chart id+params, the slice plane (z₀'s out-of-plane part, span{q₁, q₂}, the in-plane orientation — in-plane pan and zoom re-address, lock changes neither key, R-92), warps, link ids, integrator config, the kernel variant (physics, or colour_composition Appendix A's bring-up mode, "the only sim-key item" of colour_composition §0; RQ-221), the compute shaders' fast-math setting (off by default, R-297), T, event thresholds, the quality tier's sim-key fields (not depth, not `E` — the nominal's key excludes `E` and copies are cached per `copy_index`, R-89), payload schema version (a content hash of the ledger, R-36) (= the payload compatibility signature). A sim-key change resets the march (state re-boots from `t = 0`) | expensive — integration |
| **Baked texture** | equirect `GPUTexture` for colour occupants that are pure `f(n̂)` (vMF, LUTs, patterns). The physics overlay is **not** baked: it is a per-fragment occupant (its sites are functions of the decoded masses); hoisting its sites to uniforms when masses are constant over the slice is an allowed optimisation, not a bake tier (R-121; colour_composition §2) | **bake key**: colour-node source + its uniforms | ~ms, JS, debounced (~120 ms); preview canvas *is* the uploaded texture — zero preview/render drift by construction |
| **Frame** | composited output | **render key**: hash of the stain graph (nodes, their sources, wires, per-node params — the canonical graph form is defined by the task that needs it, R-72) + uniform values + overlay set | per-frame: one `textureSample` (or direct `colour(ctx)`) + L-override + post |

**The render key's graph part (R-72; REQ-RENDER-075; TASK-M1-04).** The canonical graph form is lowering Part 5's: the
live nodes in canonical order, each its kind, its occupant and its inputs by position, in JCS. The render key hashes
that form with each live node's per-node params added, as `"params"`: each uniform its occupant's `uniformSchema`
declares (gui_state_contract §3), by name, its value a list of numbers, one per component, the schema's default where
none is set. The hash is the 64-bit FNV-1a of the text's UTF-8 bytes, as the fragment key's. The fragment key hashes
the form without the params, which are uniforms, so a param edit recolours without a recompile.
A `colour` node's custom-stop ramp (colour_composition §9.4; gui_state_contract §2; TASK-M7-11), where it has one, is
added beside its params as `"ramp"`, its stops and `cyclic`, so the render key changes with it; the fragment key
leaves it out, as it leaves out the params.

The bake is an *implementation strategy* for the f(n̂) subset, not a contract change — publicly the occupant is still `colour(ctx)`. Occupants that read dynamical fields (event class, diffusion) skip the bake tier and evaluate per-fragment.

**Field-availability gating.** `ftle` (per-sample) exists only when the tier computed it (`has_ftle`; per-sample validity via the read-time `ftle_valid` predicate — payload §6); ensemble spread (footprint-derived at resolve) exists only when `contains-ensemble` — but it is not a per-sample field, so "availability" means the resolve stage runs the spread reduction, gated on the tier flag. Binding brightness to an unpopulated field **must not trigger recompute**: the UI greys out unavailable sources; reading anyway reads **NaN** at unpack (a tier-absent field is derived, never stored — Part 2), and every colouring maps it to its invalid colour (R-79). The *only* path to recompute is an explicit quality-tier change by the user. (This is the render-side analogue of "don't reconcile CPU/GPU" — the helpful violation an agent will attempt.)

---

## Part 4 — Semantic rules

**L-ownership (reserved lightness).** When a brightness metric is bound, L belongs to brightness; the colour occupant contributes hue/chroma only. Consequence: monotone-L LUTs (Viridis, Cividis, Magma) carry their information *in* L, so Replace-L destroys their primary channel — a semantic conflict, not a bug, resolved by combiner choice (Replace default; Multiply preserves base L). The house pattern is the bivariate family n̂ × ⟨field⟩ (`principia_colour_composition.md` §4.1): hue = shape-sphere position, L = a named dynamical magnitude (FTLE, diffusion, ensemble spread, `t_end`). Stability × Hue is deleted (R-76).

**Averaging (SSAA colour resolve).** Samples ≠ pixels; at boundaries several disagreeing samples fall under one pixel. **Each sample is coloured independently through the pipeline; a render-side resolve pass averages the sample *colours* into the pixel colour** — anti-aliasing, not data-averaging (sampling/SSAA note). Category-averaging is therefore **structurally impossible**: classification → colour happens per-sample before any averaging, so a boundary pixel blends the colours of the *actual outcomes present* by area fraction (honest spatial AA), never an invented class. The **ensemble copies are the SSAA sample pool** (same E copies as the spread metric; fixed low-discrepancy Halton (2,3) offsets). Continuous S² occupants may still consume means of *current* shape vectors (averaging unit vectors then mapping is legitimate — `n` is Cartesian). A *point* quantity (class, `n`, `ftle`) anti-aliases; the *footprint* quantity (ensemble spread, **derived at resolve** from the E+1 samples, not stored per-sample) is one value per nominal sample and does not sharp-edge AA (correct — nothing sub-footprint to resolve). **The resolve is display-only and terminal** — it averages colours, is never read back as data; the quad's outcome statistics come from the data-side reduction of the copies' classified outcomes. A `RUNNING` sample colours like any class. *(Deleted: the old majority-class + entropy-desaturation rule — it baked a display choice into a data occupant; uncertainty marking, if wanted, is an optional independent slot binding on the exposed spread/entropy field.)*

**Two rotations, never crossed.** `sph_uv` eats the **config-space** normal (the frame the BC/Euler/Lagrange points are fixed in). View orbit is display-only and never changes which UV a physics point maps to. Pattern auto-rotate is a UV offset *inside* the colour function — changes what colour a point shows, consistently, not which point a pixel represents. The click-to-inspect shape-sphere trajectory view answers the same question the same way.

**Composite order (constitutional):** baked base → combine (L-override) → the post chain: the physics overlay (`site_overlay`, evaluated per fragment, never baked — R-121, colour_composition §4.2) and the structural/debug overlays → `OUT`; then the display stage, outside the graph (R-67): style → **display scale** (when `render_scale ≠ 1`: bilinear upscale below native, box downsample above — the step onto the display-sized swap chain; all prior stages run at render resolution) → gamut clamp → colour-vision simulation → screen. The simulation sees the final in-gamut colours. **The line system** (R-476; `principia_lines.md`: contours, traces,
axes, grids) draws its own overlay pass, outside the graph and after the gamut clamp, before the colour-vision
simulation, which sees the lines too; it changes no pixel's colour value and never touches the sim key. Gradient-magnitude-on-the-*map* is a post occupant (screen-space finite difference over the colour buffer) — distinct from the baked |∇c| on the sphere texture.

**The Print graph stage (R-485 to R-492; `principia_print_graph.md`).** In the display stage, where R-67 had style, the
Print graph runs on the finished colours: SimState → stain → [the line sets marked `through_medium`] → **Print graph** →
display scale → **Print graph's screen pass** → gamut clamp → [the line sets drawn over] → colour-vision simulation →
screen (R-487; R-531, ruling RQ-395: the pass drawn over keeps the line system's place above, after the gamut clamp).
The screen pass (R-618, closing RQ-538, option (b)) runs the graph's `device_px` nodes and the screen-anchored sheet
after the display scale, at display resolution, so a `device_px` length is a screen pixel at every `render_scale`; the
rest of the graph runs at render resolution before it, and with neither a `device_px` node nor a sheet in the graph
the screen pass is the identity. At export a display pixel is an export pixel (`principia_print_graph.md` §6.2).
The identity is **no pass and no target**: with neither in the graph the screen pass is not dispatched and allocates
nothing, so it costs nothing, as an overlay that is off (render_gui_spec §12.1, REQ-RENDER-055); a pass-through copy
at display resolution is not the identity, however bitwise its output. When it runs it is a per-frame pass at display
resolution, whatever `render_scale`; its time is recorded on the Mac (REQ-PERF-113) and its targets are counted in
`k_d` (memory_tiers §8, REQ-PERF-069) (perf review 5661999637 of PR #293, P2, applied per R-369).
*Was:* "→ **Print graph** → display scale → gamut clamp → [the line sets drawn over] → colour-vision simulation →
screen (R-487; R-531, …)", with no screen pass. *Was:* "→ **Print graph** →
[the line sets drawn over] → display scale → gamut clamp → colour-vision simulation → screen (R-487; where the pass
drawn over sits, before the display scale or after the gamut clamp as above, is RQ-395, and it keeps the place above
until RQ-395 is ruled)". It is a node graph of its own, outside the stain graph, with its own registry and WGSL; its
default, Plain, is the bitwise identity; it re-integrates nothing and touches no sim key (`RenderState.print`, R-491).
It reads the stain's output and payload fields only as read-only inputs, normalised by the stain's own window rule,
never writing them (design doc §4.4); no readback, statistic, legend, readout or golden reads its output (R-489).
**Debug views bypass it** (R-487): View › Debug view, a baked variant of the pipeline (R-41, R-467), and every stain
whose colour occupant is a debug view (Part 6) are drawn with Plain, never styled, because a debug view exists to show
the data. At a pixel that is not a system, blank above (R-470, R-511), it prints the sheet (its Ground, Age and
Viewing slots) and never a mark (its Palette, Medium, On the ground and Unfinished slots), so no palette colour reaches
that pixel (R-533, ruling RQ-398). It is a stage of the frame record, the print stage (`stage_ms.post`; the human's ruling on RQ-497, 21 Oct 2026), and degrades while the camera
moves (R-488).

**Sentinels, not NaN, in storage.** `diffusion` is derived at read, and an invalid streaming slope (`n < 2`) reads NaN by the predicate `n ≥ 2` (R-245, amending R-17's sentinel; `principia_dd_simstate_payload.md`, and the payload ledger, `principia_dd_generation_root.md` §3.4); **never NaN in storage buffers** (WGSL NaN behaviour is implementation-defined; `isNan` is unreliable under fast-math). A blown-up sample stores the defined failed-state values (payload §1: its f16 latches hold `0.0`), not NaN; only a tier-absent *derived* field reads NaN, at unpack. Every colouring maps NaN or a sentinel to its invalid treatment — by default `#141418` in a presentation view and the hatched invalid pattern, `debug_invalid(frag_xy)`, only in the debug views (R-79, R-132, R-467; applied per R-369). *Was:* "by default the hatched invalid pattern (R-79, R-132)" (stale value, replaced by R-467). The kernel-set `state` enum (`sim_failed`/`decode_failed`) and sticky `saturated` bit are the primary point-of-computation invalid-signals; the drift suspect gates are read-time predicates over the stored latches (payload §5); bitcast pattern tests in debug views are best-effort garnish.

**The occupant rules `cargo xtask lint wgsl` checks (R-72; REQ-RENDER-015, REQ-RENDER-018, REQ-COL-005; TASK-M1-15).**
Part 2's validity rules, the shadow rule of `principia_dd_generation_root.md` §3.5 and the colouring's seams 5 and 6
(`principia_dd_colouring.md` §4) are checked by the WGSL lint (Part 5, "Unpack layer"), in these terms:
- **A colour occupant** is the occupant of any stain node but `OUT` (Part 2): a source, colour, brightness, combiner or
  post, built in, generated or written at runtime. That is every file under `crates/render/shaders/wgsl/frag/`, the
  debug catalogue's views and reductions under `crates/render/frag/debug/`, and the WGSL the render side and the presets
  write at runtime (a field ramp's, a preset's), whose output is checked in under `crates/render/frag/runtime/` and kept
  equal to its writers by a test. A **debug view** is a colour occupant under a `debug/` directory (Part 6: every debug
  view is an ordinary slot occupant). A **source** is a source node's occupant; a built-in one is a file under
  `crates/render/shaders/wgsl/frag/source/`. The unpack layer, the read side, the stain's context, the shared library
  and the compositor are not occupants.
- **The generated accessors** are the read side's `SimState` and `ICDescriptor` members as `ctx.sample` and `ctx.ic` hold
  them, filled by the generated `sample_read` and `ic_read` (lowering Part 3a: plain member access, no getters), and the
  functions of the generated unpack layer and read side (`crates/render/frag/generated/`). **A non-accessor payload
  read** (seam 5) is a colour occupant's use of a payload buffer, any `var<storage>` global (the `SimState`, word,
  `ICDescriptor` and `RenderQuad` buffers), or a use of the packed word, `ctx.sample.word`, other than whole as an
  argument of a generated accessor (`fgw_length_raw(ctx.sample.word)`): indexing, masking or shifting it, or handing it
  to any other function, decodes the payload by hand.
- **Chart, integrator and scheduler identifiers** (seam 6) are, in the fragment's context: the chart's, the chart lane
  (`ctx.chart`; colour composition §3: the chart id, `z` and the position in the slice plane) and the quad's frame in
  the slice plane, `ctx.quad.tl`, `ctx.quad.centre` and `ctx.quad.half_width` (deep zoom §1's per-quad `c` and `h`); the
  integrator's, the read side's integrator parameters `ctx.params.dt_macro`, `ctx.params.delta_0` and
  `ctx.params.n_renorm` (the macro-step, and Benettin's δ₀ and renormalisation interval, which the read side derives
  `ftle` and `diffusion` from); and the scheduler's, `ctx.quad`'s `RenderQuad` members, every one but the frame and the
  raster's own addressing (depth, state, coherence, impurity, spread, suspect fraction, priority, cache age, ancestor
  gap and dominant outcome: what the CPU scheduler writes, Part 6's quad fields). They are not the payload's fields and
  predicates, the validity lane's predicates (`ctx.validity`, the sample's chart classification `forbidden` among them,
  colour composition §3, REQ-COL-102), `ctx.params.horizon_steps` (the step scale the display fractions are derived
  with, Part 5), the raster's
  addressing (`ctx.frag_xy`, `ctx.screen.uv`, `ctx.tile.uv`, and `ctx.quad`'s `index`, `uv` and `sample_count`), the
  prelude's tier constants and `has_ensemble()`, or a node's own uniforms.
- **Reading `r_sh` or `p_sh` as a sample** (REQ-RENDER-018) is any read of the `r_sh` or `p_sh` member of a stored or
  read-side `SimState` in fragment-stage WGSL but the generated read side's `sample_read`, which loads the shadow for
  `ftle`'s `benettin_delta` (the shadow as `ftle`'s ingredient, dd_generation_root §3.5) and fills the read side.
- **Guarded** (REQ-RENDER-015; Part 2; R-255): in a colour, brightness, combiner, post or source occupant, a debug view
  among them, every use of a read of `ftle` (returning it, storing it, passing it to a function, branching on it, an
  aggregate's accumulation among them) sits where `has_ftle && s.ftle_valid` holds for the sample `s` it was read from:
  in the branch an `if` on that conjunction takes, with any further conjuncts, or after an `if` on its negation whose
  branch leaves the block (`return`, `discard`, `break` or `continue`). A read of `ensemble_spread` is guarded by the
  uniform, `has_ensemble()` (R-145). The one use allowed unguarded is the absence test, the value's `bitcast<u32>`
  compared by `==` or `!=` with the canonical quiet NaN's bits, `0x7FC00000` (`CANONICAL_QNAN_BITS`; lowering Part 3a,
  "The sentinels' values"). So an aggregate over `ftle` that excludes samples by `ftle_valid` is guarded, and one that
  relies on NaN propagation, or on `min`, `max` or `clamp` with NaN operands, is not (R-255). A source forwards its own
  sample's field and is not held to the guard for that value, `ctx.sample`'s, where `ctx` is the context the source is
  given; every other sample's `ftle` or `ensemble_spread` it reads is held to it, so an aggregate over `ftle` in a
  source still excludes samples by `ftle_valid` (R-255). A debug view shows its own sample's value literally, NaN to the
  invalid colour by R-114's bitcast guard (R-79), and is not held to the guard for that value, `ctx.sample`'s, where
  `ctx` is the context the view is given; every other sample's `ftle` or `ensemble_spread` it reads, a `sample_read`
  result's among them, is held to it, so an aggregate over `ftle` in a debug view still excludes samples by `ftle_valid`
  (R-255). The guard is read where `has_ftle` itself is read: an occupant that uses `has_ftle` in a `const` declaration
  is not seen to be guarded, and the lint reports the first read of `ftle` or `ensemble_spread` in each of its functions
  as unchecked.
- **The exceptions**, each to the one rule it names, as Part 5, Part 6 or R-79's debug exception requires them: the
  field views of `r_sh` and `p_sh` and their direction-cosine reductions read the shadow (Part 6: a view per field,
  every struct); the structural views `debug/s_*` and the posts `fallback_tint` and `pending_hatch` read the scheduler's
  quad fields (Part 6: the quad fields and the structural overlays); the FTLE-running and diffusion-slope views read
  `ctx.params.dt_macro` (Part 5, "Live-state & array inspection": `S/t` and `C_ty/C_tt`); the whole-word hash view hands
  the word to `dbg_hash_word` (Part 6's word views: `dbg_hash` of the uint4); and the coordinate presets read the quad's
  frame (Part 6's UV view; debug tooling plan §F's UV passthrough). These are the views built so far. A later view that
  Part 5, Part 6 or R-79's debug exception lets read what a rule forbids is excepted by the task that builds it, which
  adds it here, to the one rule it reads past, with the reason, and to the lint's list, as these were: so, when they are
  built, the DECODE view, its agreement presets and ROUNDTRIP, which read `ctx.chart.z` (the chart's; Part 6's field
  views), the impurity-mask cross-check, which reads the quad's `dominant_outcome` (the scheduler's; Part 6's
  cross-checks), and the diffusion (Welford) cross-check, which reads `ctx.params.dt_macro` to recompute `C_tt(n)` (the
  integrator's; dd_generation_root §3.5). Chart position, the state map's source, is one too (R-558, ruling RQ-489):
  when built, it reads the target chart, its parameters and its inverse, generated to WGSL from the one Rust source
  (R-116; `principia_state_map.md` SM2), past seam 6's chart rule, and TASK-M7-47 adds it here and to the lint's list.

---

## Part 5 — Struct inspection SDK

**One layout table generates everything, to two targets.** The bit layouts exist in several places — the payload drill-down's tables, the kernel's packing/unpacking, the fragment side's WGSL unpack helpers, the CPU export decoder ("Decoded" export mode). Hand-written, they *will* drift. So: a single **Rust layout definition** (mirroring the payload drill-down's tables verbatim) **generates** (a) **Rust** pack/unpack for the **kernel** (which is Rust → SPIR-V under the substrate — lowering Part 2, so the kernel packs in Rust, not WGSL), (b) **WGSL** unpack helpers for the **fragment side** (which reads `SimState` in WGSL to colour it), (c) the CPU (Rust host) export decoder, (d) the debug-view catalogue entries (WGSL fragment views). Two targets, one source; the compute-side pack and the fragment-side unpack are the same layout emitted twice. The spec table is the human-readable rendering of the same table.

### Unpack layer (generated, one accessor per named field)

```wgsl
// generated WGSL (fragment-side unpack) — do not edit. source: the Rust layout definition (mirrors payload §2, §6 — R-86)
// stored buffers (R-343): group 0 is the assembler's per-frame uniforms; the group/binding numbers are generated
// constants from one ledger table (SIMSTATE_GROUP = 1, SIMSTATE_BINDING = 0, WORD_GROUP = 1, WORD_BINDING = 1)
@group(1) @binding(0) var<storage, read> simstate_buffer: array<SimStateFTLE>;  // the full tier; M1's per-tier assembly owns the feature-off variants (lowering Part 3a)
@group(1) @binding(1) var<storage, read> word_buffer: array<vec4<u32>>;
fn sample_state(i: u32) -> SimStateFTLE { return simstate_buffer[i]; } // the ONLY reads of either buffer (R-343)
fn sample_word(i: u32) -> vec4<u32>     { return word_buffer[i]; }    // the same i as the state, per copy
// R-378: the read side loads only the stored members each field needs (simstate_buffer[i].packed_a, simstate_buffer[i].S, …),
// never the whole stored struct in one load, so the whole-struct sample_state(i) above is replaced; a word field loads only the components it needs
// schema version (R-343): WGSL has no u64 — .x the low 32 bits of the Rust u64, .y the high 32 (unpack2x16float's order)
// const PAYLOAD_SCHEMA_VERSION: vec2<u32> = vec2<u32>(<low 32>, <high 32>);
// closure (R-343): WGSL has no u16 — one member closure_step_reserved: u32, closure_step bits 0–15, _reserved bits 16–31
fn closure_step(w: u32) -> u32        { return extractBits(w,  0u, 16u); } // w = sample_state(i).closure_step_reserved
// sample_descriptor (low 16 of packed_a) — 10 bits used (bits 10-15 reserved; total_substeps is a separate exact u32)
fn sd_state(w: u32) -> u32            { return extractBits(w,  0u, 3u); } // 0 escape,1 bounded,2 collision,3 running,4 sim_failed,5 decode_failed
fn sd_detail(w: u32) -> u32           { return extractBits(w,  3u, 2u); } // 4-state union (payload §2): escape→body, collision→pair, sim/decode_failed→failure category
fn sd_saturated(w: u32) -> bool       { return extractBits(w,  5u, 1u) == 1u; } // N_sub == N_max occurred at some macro-step (sticky confidence flag, R-86)
fn sd_dmin_pair(w: u32) -> u32        { return extractBits(w,  6u, 2u); } // which pair achieved d_min (latched; NOT from the word); pair ids per payload §2 (R-22)
fn sd_last_symbol(w: u32) -> u32       { return extractBits(w,  8u, 2u); } // final word symbol (a=0,A=1,b=2,B=3); a redundant cache of the sidecar word
fn sd_last_symbol_valid(len: u32) -> bool { return len >= 1u && len != 127u; } // len = fgw_length_raw(word_buffer[i]) — validity gates on the word
// NOTE: bits 10–15 are reserved. The complexity proxy is DERIVED from the exact u32 — floor(log2), 0 for a total ≤ 1 (R-86):
fn total_substeps_log2(total: u32) -> u32 { return select(0u, 31u - countLeadingZeros(max(total, 1u)), total > 1u); } // on SimState.total_substeps, NOT the descriptor
// state predicates (canonical names — payload §6). "Finished" (stop marching) ≠ "resolved outcome" (a real result):
fn sd_is_resolved_outcome(w: u32) -> bool { return sd_state(w) <= 2u; }      // escape/bounded/collision
fn sd_is_running(w: u32) -> bool          { return sd_state(w) == 3u; }      // skip in reductions (hasn't voted)
fn sd_is_failed(w: u32) -> bool           { return sd_state(w) >= 4u; }      // sim_failed/decode_failed (+reserved 6/7)
fn sd_is_finished(w: u32) -> bool         { return !sd_is_running(w); }      // scheduler gates re-dispatch on THIS —
                                                                             // gating on is_resolved_outcome re-marches failed samples forever (payload §6)
// DERIVED, not extracted here: orbit_count/retrograde (from theta), encounter_count/enc_XY/dominant_pair
//   (topological read of the word, §3.3), ftle=S_final/elapsed (finalise partial renorm, §5), current substeps (live off the march), current drift (H(r,p)-E_0).

// times (u32)
fn tm_t_end_step(w: u32) -> u32       { return extractBits(w,  0u, 16u); }  // completed-step count at ALL times, latched at termination (payload §2)
fn tm_t_dmin_step(w: u32) -> u32      { return extractBits(w, 16u, 16u); }  // EXACT absolute macro-step of closest approach
// display fractions derived with a horizon_steps uniform: f32(t_end_step)/f32(horizon_steps)
// exact u16 is the only format: dispatch refuses a configuration with horizon_steps = ceil(T/dt_macro) > 65535 (R-86)

// f16-packed scalars — unpack2x16float (core WGSL, NO shader-f16 feature needed; returns vec2<f32>: .x low, .y high)
// pack side must clamp/canonicalise to binary16 finite range first (indeterminate otherwise); failed states pack 0.0
// packed_a = descriptor(bits 0–9 used, 10–15 reserved) in low 16 + d_min(f16) in high 16  → d_min = unpack2x16float(packed_a).y
// packed_b = dE_max(f16) low + dLz_max(f16) high
fn pa_d_min(pa: u32) -> f32           { return unpack2x16float(pa).y; }
const PA_D_MIN_UNSET: u32 = 0x7c00u;  // d_min's unset value, f16 +inf (R-271)
fn pa_d_min_is_unset(pa: u32) -> bool { return extractBits(pa, 16u, 16u) == PA_D_MIN_UNSET; } // by its bits, never a float test (R-271, R-343)
fn pb_dE_max(pb: u32) -> f32          { return unpack2x16float(pb).x; }
fn pb_dLz_max(pb: u32) -> f32         { return unpack2x16float(pb).y; }
// NB: descriptor occupies the LOW 16 bits of packed_a; the sd_* accessors above take that same u32 (bits 0–9 used; 10–15 reserved).


// trajectory_stats — ELIMINATED (fields derived or moved; see ledger). Its former contents:
//   t_dmin -> now in `times` (16-bit macro-step); total_substeps -> its own exact u32 (log proxy DERIVED at read);
//   orbit_count/retrograde -> DERIVED from theta at read; dmin_pair -> descriptor bits 6–7.

// free_group_word: uint4 in a SEPARATE BUFFER (word_buffer[sample_index]), NOT inline in SimState.
//   Mixed-radix log2(3) packing (76 symbols in 121 bits = x,y,z + .w[0:24]); length 7 bits at .w[25:31].
//   Truncation is the LENGTH SENTINEL length_raw == 127 — there is NO flag bit (bit 24 is payload; payload §3).
//   Decode is O(length) SEQUENTIAL (pop base-3 tail, residue is base-4 head), NOT random-access.
//   Fetched only at resolve (symbolic-spread) and inspect (display) — never in the march. See payload §3 / ledger 3.3a.
fn fgw_length_raw(word: vec4u) -> u32 { return extractBits(word.w, 25u, 7u); }        // 0…76 valid; 127 = truncated
fn fgw_truncated(word: vec4u) -> bool { return fgw_length_raw(word) == 127u; }
// REDUCED crossing count — valid ONLY when not truncated (payload §5/§6); never expose 127 as a count
fn fgw_reduced_length_valid(word: vec4u) -> bool { return !fgw_truncated(word); }
fn fgw_reduced_length(word: vec4u) -> u32 { return fgw_length_raw(word); }   // use only when fgw_reduced_length_valid
// RETAINED prefix length — debug/export only, NOT the reduced count (payload §6 name, R-86)
fn fgw_retained_prefix_length(word: vec4u) -> u32 { return select(fgw_length_raw(word), 76u, fgw_truncated(word)); } // clamps 127
// fgw_decode(word) -> array of symbols: sequential base-3 unpack + continuation-table replay (cold path)
```

**WGSL traps (one line each, they all bite):** use the **u32** overload of `extractBits` — the i32 overload sign-extends. WGSL has **no f64**. f16-packed pairs read via `unpack2x16float`. **`SimState` is now 8-byte aligned** (the `vec4` word moved to its own buffer — largest remaining member is `array<vec2>`); pack the live-state block in vec2 groupings for `r, p` and shadow. The word buffer is separately bound and indexed identically to samples (per-copy).
**Bindings (R-343):** `SimStateFTLE` at `@group(1) @binding(0)`, the word buffer at `@group(1) @binding(1)`; group 0 is reserved for the assembler's per-frame uniforms. The group and binding numbers are generated constants from one ledger table, never literals written by hand. Both buffers are read only through `sample_state(i)` and `sample_word(i)`, the same `i` for both; no other code indexes either buffer (a `word_buffer[i]` in a comment above reads as `sample_word(i)`). **`closure_step` (R-343):** WGSL has no u16, so `closure_step` and `_reserved` are one member, `closure_step_reserved: u32` (`closure_step` bits 0–15, `_reserved` bits 16–31), read through `closure_step(w)`. **Schema version (R-343):** WGSL has no u64, so it is `const PAYLOAD_SCHEMA_VERSION: vec2<u32>`, `.x` the low 32 bits and `.y` the high 32. The layer only reads: no WGSL setter is emitted (R-343). **Unset-checks test bit patterns (R-343):** an unset-check on a value read in a fragment shader tests its bit pattern (`pa_d_min_is_unset`, against `PA_D_MIN_UNSET = 0x7c00u`, R-271), never `isinf` or `isnan` (nor a float comparison standing in for them, such as `x != x` or `x > 65504.0`), because fast-math (R-297) may optimise those away. **The lint (R-351, R-352):** `cargo xtask lint wgsl` fails on `isinf` or `isnan`, on a comparison against an inf or NaN constant, on a float compared with itself (`x != x`, `x == x`, and `<`, `<=`, `>` or `>=` of an expression with itself), or on a comparison against a finite-max stand-in used as an inf check (65504.0, f16's largest finite value, or 3.40282347e38, f32's, of either sign, in any spelling or as a `bitcast<f32>` of its bit pattern), in any WGSL file under `crates/render/frag/`; the fix is a bit-pattern test. The review checklist's grep stays as a backup. (The `crates/render/frag/` scope and the rule's place in `lint wgsl` are applied per R-204, accepted by R-352; the stand-in list and the ordered self-comparisons are applied per R-204, accepted by R-353.)

*Note (TASK-M1-03, applied per R-369):* the float rules also hold in every WGSL file of the fragment stage's shared
library under `crates/render/shaders/` (render_gui_spec Part II §10.1). The generated prelude is linted alone, and every
other library file as the prelude's continuation, since it follows the prelude at assembly.

**Per-member loads (R-378):** the generated read side loads only the stored members each field needs (`simstate_buffer[i].packed_a`, `simstate_buffer[i].S`, …), never the whole stored struct in one load, so unused data is never fetched on any backend; this replaces the whole-struct `sample_state(i)` above, and each buffer is still read only by the generated layer, at the same `i` for both. A word field likewise loads only the components of `word_buffer[i]` it needs (applied per R-369). The physics review confirms, from the compiled shader output, that a stain reading one field loads only that field's words.

**`closure_step_reserved` bits 16–31 (R-397):** the `_reserved` half of the u32 above is `θ̃`'s frozen pole reference, a u16 code in steps of `2π/65535` with 0xFFFF for "no reference" (payload §2; dd_integrator §3.7). It is march state, read only by the kernel when a march resumes; as a `_`-named member it has no ledger field entry, so no accessor is emitted for it, the read side never loads it, and no debug view shows it. `closure_step(w)` still reads bits 0–15 only.

### Presentation layer (hand-written, small, reused by every debug view)

```wgsl
fn dbg_cat(i: u32, n: u32) -> vec3f      // categorical: Okabe–Ito cycle ≤8 classes, golden-angle beyond (reuses palette system §9.1)
fn dbg_lin(x: f32, lo: f32, hi: f32) -> vec3f   // scalar, viridis ramp
fn dbg_log(x: f32, eps: f32) -> vec3f           // scalar, log-compressed
fn dbg_flag(b: bool) -> vec3f                   // boolean: green / red
fn dbg_hash_u32(v: u32) -> vec3f                // raw word → hashed colour ("is it changing at all")
fn dbg_hash_word(w: vec4u) -> vec3f             // whole free_group_word → hashed colour (R-72; REQ-TOOL-155)
fn dbg_ternary(m: vec3f, frag_xy: vec2f) -> vec3f        // the three masses → ternary colour (R-72; REQ-TOOL-154)
fn dbg_dircos3(v: vec3f, frag_xy: vec2f) -> vec3f        // a k = 3 vector's direction cosines, ½(v̂ + 1)
fn dbg_dircos6(v: array<vec2f, 3>, frag_xy: vec2f) -> vec3f // a k = 6 vector's direction cosines (R-72; REQ-TOOL-157)
fn dbg_sentinel(x: f32, frag_xy: vec2f) -> vec3f // absence-NaN (exact bitcast test) → debug_invalid(frag_xy), the hatch (R-136); a stored sentinel such as the word length's 127 shows as its literal value on the ramp (R-79); suspect styling: an extension point, applied on this output by the drift views from the drift suspect predicates (TASK-M3-05, R-379)
fn dbg_drift(x: f32, compaction: u32, viridis: bool, lin_r: f32, eps: f32, frag_xy: vec2f) -> vec3f // a drift view's compacted drift on its ramp (TASK-M3-05; R-381)
fn dbg_suspect(rgb: vec3f, x: f32, energy: bool, lz: bool, frag_xy: vec2f) -> vec3f // the suspect styling on a drift view's ramp output (TASK-M3-05; R-379)
```

**The renderings (R-72; REQ-TOOL-122; TASK-M1-03).** The helpers live in `crates/render/shaders/wgsl/lib/present.wgsl`
and follow the shared prelude at assembly (render_gui_spec Part II §10.1), whose ramps, colour-space maps and
`debug_invalid` they call. Each returns linear RGB, the colour slot's space (Part 2). An "8-bit sRGB" colour is decoded
with dd_colouring §3.1's sRGB transfer.
- **`dbg_cat(i, n)`:** for `n ≤ 8`, the Okabe–Ito palette (Okabe & Ito 2002), cycling `i mod 8`, as 8-bit sRGB in
  its published order: black `#000000`, orange `#E69F00`, sky blue `#56B4E9`, bluish green `#009E73`, yellow `#F0E442`,
  blue `#0072B2`, vermillion `#D55E00`, reddish purple `#CC79A7`. For `n > 8`, every class takes the golden angle
  (dd_colouring §3.7): OKLCH with L = 0.75 and C = 0.12, hue `frac(i·φ_g)` turns, `φ_g = (√5 − 1)/2`. The shader
  computes the hue as `i · round(φ_g·2³²)` mod 2³², over 2³², so it is within `(i + 1)·2⁻³²` turns of `frac(i·φ_g)`
  before the f32 rounding. At L = 0.75 the sRGB gamut holds a chroma of 0.1275 at every hue, so C = 0.12 is in gamut
  at every hue.
- **`dbg_lin(x, lo, hi)`:** `ramp_viridis(range_norm(x, lo, hi, false, ·))`, the fixed range, clamped.
- **`dbg_log(x, eps)`:** `s = ln(1 + |x|/eps)`, then `ramp_viridis(1 − 1/(1 + s))`. Below `eps` the map is nearly
  linear (`t ≈ |x|/eps`), above it logarithmic; 0 maps to the ramp's start and `|x| → ∞` to its end; `|x| = eps·(e − 1)`
  maps to its middle. `eps > 0` is the caller's argument, the magnitude where compression sets in; the helper gives it
  no default. The sign of `x` is not shown.
- **`dbg_flag(b)`:** true is Okabe–Ito's bluish green `#009E73`, false its vermillion `#D55E00`, a green and red that
  the common colour-vision deficiencies still tell apart.
- **`dbg_hash_u32(v)`:** the PCG hash (Jarzynski & Olano 2020, "Hash Functions for GPU Rendering", JCGT 9(3),
  `pcg_hash`): `state = v·747796405 + 2891336453`, `word = ((state >> ((state >> 28) + 4)) ^ state)·277803737`,
  `h = (word >> 22) ^ word`, all wrapping u32. Its low three bytes, low first, are the 8-bit sRGB red, green and blue.
- **`dbg_hash_word(w)`, the whole-word hash (R-72; REQ-TOOL-155; TASK-M1-12):** the `free_group_word`, a `uint4`
  `(x, y, z, w)` (payload §3), folds through `dbg_hash_u32`'s PCG hash, `pcg`, limb by limb from `x`:
  `h = pcg(w ^ pcg(z ^ pcg(y ^ pcg(x))))`, all wrapping u32. `h` then takes `dbg_hash_u32`'s byte-to-RGB step: its low
  three bytes, low first, are the 8-bit sRGB red, green and blue. Every bit of the word, its length included, enters
  the fold, so the word-hash view (Part 6) colours whole words, not one limb. Like `dbg_hash_u32` it can give any
  colour, never a pattern. The word-hash view draws it only where `fgw_reduced_length_valid` holds: a truncated
  word's retained prefix is not the reduced word, and every word derivation is invalid for it (payload §3), so there
  the view draws `debug_invalid(frag_xy)`, the hatch, as the reduced-length and symbol-at-k views do (applied per
  R-369; the physics reviewer approves this R-72 definition).
- **`dbg_ternary(m, frag_xy)`, the ternary masses colour (R-72; REQ-TOOL-154; TASK-M1-12):** the `ICDescriptor`'s
  masses `(m0, m1, m2)` as linear RGB scaled by `1/max(mᵢ)`: equal masses draw white, and each vertex, one mass alone,
  its primary, `m0` red, `m1` green, `m2` blue. Where no mass is positive, or one is the absence NaN, the colour is
  undefined and `debug_invalid(frag_xy)` is drawn.
- **The direction cosines of a vector field (generation-root §3.8; TASK-M1-12):** for every `vector(type, k)` field the
  catalogue offers two views, '‖·‖ as scalar', the field's generated view of `‖v‖`, and 'as direction-cosines', drawn
  by the helper of its `k`. Where `‖v‖ > 0` fails, or a component is the absence NaN, the direction is undefined and
  `debug_invalid(frag_xy)` is drawn.
  - **`k = 3`, `dbg_dircos3(v, frag_xy)`** (`n`): `½(v̂ + 1)` as linear RGB, `v̂ = v/‖v‖`, the live shape view's
    mode 0 (below).
  - **`k ≠ 3`, `dbg_dircos6(v, frag_xy)` (R-72; REQ-TOOL-157; RQ-235 as amended per code review 5438179638):** the
    direction cosines `cᵢ = vᵢ/‖v‖` of the `k` components, each squared, are summed into channel `⌊3i/k⌋`; the three
    sums, which total 1, are linear RGB scaled by `1/max`, as the ternary masses are. For `k = 6`, `r`, `p` and their
    shadows, the components are `(x, y)` per body, so channel `j` is body `j`'s share of `‖v‖²`: red body 0, green
    body 1, blue body 2 (internal ids; the view's legend reads body 1, 2, 3, R-449), equal shares white. The scale cancels `‖v‖²`, so the colour is `wⱼ/max(wⱼ)` with `wⱼ` body
    `j`'s squared norm. A k = 6 vector has no three-channel sign, so the signs of the cosines are not shown; the
    field's per-component views show them.
- **Drift max-vs-final (TASK-M1-12; applied per R-369):** the accumulator view of the drift shape shows the current
  drift's magnitude over its running maximum, `|H(r, p) − E_0| / dE_max` (`u_quantity` 0) or `|L_z(r, p) − Lz_0| /
  dLz_max` (1), by `dbg_lin` over `[0, 1]`: near 1 the loss is secular, still at its maximum; near 0 it was a transient
  spike that recovered (R-246). Where the maximum is not positive, before any drift is latched, the ratio is undefined
  and `debug_invalid(frag_xy)` is drawn.
- **`dbg_sentinel(x, frag_xy)`:** the absence NaN, tested by its exact bits against the canonical quiet NaN (`0x7FC00000`,
  lowering Part 3a), draws `debug_invalid(frag_xy)`, the hatch below. Any other value, a stored sentinel such as the word length's 127
  included, shows as its literal value on the viridis ramp at `t = 0.5 + 0.5·x/(1 + |x|)` (`dbg_literal`), with `x`
  first clamped to ±1e30, which needs no range: 0 maps to the middle, −1 to a quarter, 1 to three quarters, and every
  finite value to its own place (R-79, R-136). The suspect styling is an extension point, not an argument:
  `dbg_sentinel` takes no suspect flag, and the view that reads the drift suspect predicates (Part 4; the energy-drift
  and L_z-drift views) applies the styling on `dbg_sentinel`'s output. TASK-M3-05 defines the styling here, its look
  and which predicate drives which style (R-72, the physics reviewer approves; REQ-TOOL-149), and builds it (R-379).
  The drift views' ramp is the preset's (RQ-208, decided per R-369, option 3), and each compaction (R-381) has its own
  placement `t` on it; the absence NaN draws `dbg_sentinel`'s hatch under every compaction. `symlog`, the default:
  the compacted value takes `dbg_sentinel`'s place `t = 0.5 + 0.5·x/(1 + |x|)` on `f_edrift`'s `diverging(c−,c0,c+)`,
  `0` at the neutral `c0`, or on viridis through the palette swap (colour_composition §6), where the pixel is
  `dbg_sentinel`'s output. `lin`: `t = range_norm(x, −R, R, false, ·)` (render_gui_spec §10.1), the fixed range,
  clamped, on the `diverging` ramp, `0` at the neutral `c0`, or on viridis through the palette swap; a drift beyond
  `±R` saturates to the ramp's end, `c−` or `c+`. `log`: the drift's magnitude `|x|`, unsigned, placed as `dbg_log`
  places it, `t = 1 − 1/(1 + s)` with `s = ln(1 + |x|/ε)` the floored log value, on the sequential viridis ramp,
  `dbg_sentinel`'s, never the diverging one, since colour_composition §1.2 sends positive fields to sequential; the view
  states that it shows `|drift|`. The `diverging` ramp carries only the signed compactions, `symlog` and `lin`. The
  styling applies on that ramp's output, and TASK-M3-05's definition gives the three colours (R-72; REQ-TOOL-149).
- **The drift views and the suspect styling (TASK-M3-05's definition, R-72; R-379, R-381; REQ-TOOL-149; the physics
  reviewer approves):** the energy-drift and L_z-drift views (the generated `energy_drift` and `Lz_drift` views and
  the derived `energy_drift` view) colour the current drift `x`, `energy_drift` or `Lz_drift`, in two steps, both in
  `crates/render/shaders/wgsl/lib/drift.wgsl`, which follows the compactions and ramps at assembly.
  - **The value and its place, `dbg_drift(x, compaction, viridis, R, ε, frag_xy)`.** The value each view passes on
    is the preset's compacted value, chosen by the view's `COMPACTION` uniform: 0 `symlog`, the default, 1 `lin`, 2
    `log` (R-381). The absence NaN, by its exact bits, draws `debug_invalid(frag_xy)` under every compaction; any other
    value is clamped to ±1e30, as `dbg_literal` clamps it, and placed: `symlog` at `t = dbg_literal(s)`, `s =
    sign(x)·ln(1 + |x|/ε)` the compacted value, so `t = 0.5 + 0.5·s/(1 + |s|)`, 0 at the middle; `lin` at `t =
    range_norm(x, −R, R, false, ·)`, clamped; `log` at `t = 1 − 1/(1 + ln(1 + |x|/ε))`, `dbg_log`'s place of `|x|`.
    `ε` is the field's floor, the sim key's `ε_E` for `energy_drift` and `ε_L` for `Lz_drift` (10⁻⁶ by default,
    integrator contract Part 3), each a view uniform, `EPS_E` and `EPS_L`. `R` is the view's `LIN_R` uniform, its
    default the constants register's `drift_lin_r_E` or `drift_lin_r_Lz` (REQ-TOOL-150). `ln(1 + a/ε)` is
    `log1p_ratio`'s, which forms no overflowing quotient.
  - **The non-finite guard (R-436; render_gui_spec §10.1; R-79).** Each view tests its reference, `E_0` or `Lz_0`, and
    then its drift `x` on their bits, an exponent of all ones (`dbg_drift_non_finite`), before reading either as a
    number, and a non-finite one draws `debug_invalid(frag_xy)` under every compaction, unstyled. So what reaches
    `dbg_drift` from a view is finite, and nothing rests on the bits of `∞ − ∞`, which WGSL doesn't pin: a stored `±∞`
    reference (`E_0`'s `−∞` at the collision pole) hatches with `x` whatever its bits, and `x = ±∞` on a finite
    reference hatches rather than being clamped.
  - **The neutral band (R-652; the owner's ruling (d) of 31 Oct 2026; REQ-TOOL-149), `dbg_drift_shown(x, compaction,
    reference)`, in `drift.wgsl`.** The current drifts are computed in the fragment, `H(r, p) − E_0` and `L_z(r, p) −
    Lz_0` (dd_generation_root §3.8; payload §5), so a fresh sample's drift is rounding, not physics, and differs by
    backend. Under `symlog` a drift with `|x| ≤ k·2⁻²³·|reference|`, `reference` `E_0` or `Lz_0`, `k = 128`, is drawn
    as 0, at `c0` exactly (or viridis' middle under the palette swap); any other `x` keeps its own place, so a drift
    past the band is unchanged. `lin` and `log` have no band, and a non-finite `x` or `reference` passes through first,
    on the bits, so the band never makes a non-finite operand neutral (`|x| ≤ k·2⁻²³·|∞|` holds for every `x`). The value
    passed to `dbg_suspect` stays `x`, and the predicates read the latches, so a band never hides a lit predicate.
    **Why `k = 128`:** in units of `2⁻²³`, the relative size of one ULP at most, the difference of a fragment's value
    from the stored reference on a fresh sample is bounded by summing, to first order, each operation's error in
    `H = Σ dot(pᵢ, pᵢ)/(2mᵢ) − Σ mᵢmⱼ/√dot(rᵢⱼ, rᵢⱼ)`. In the fragment, under WGSL's accuracy (`+ − ×` ½ ULP, `/` 2.5
    ULP, `sqrt` 4.5 ULP), with any reassociation of the six-term sum (five additions, each ½ ULP of a partial sum at most
    `K + |V|`): a kinetic term 3.5, so `K` carries 6; a potential term 0.5 + (2 of the dot, halved by the root, + 4.5) +
    2.5 = 8.5, so `|V|` carries 11. In a correctly rounded CPU reference, `K` 3 and `|V|` 4. So
    `|x| ≤ 2⁻²³·(9K + 15|V|)`, `k_E = (9K + 15|V|)/|E_0|`, and for a bound state `|V| = K + |E_0|`, `k_E = 24·K/|E_0| + 15`.
    For `L_z`, six products and five additions in the fragment, 3 of `S = Σ(|xᵢp_{y,i}| + |yᵢp_{x,i}|)`, and 2 in the
    reference: `k_L = 5·S/|Lz_0|`. In a GPU-computed reference, the one kernel source at f32 under the same accuracy, the
    production path, the reference carries the fragment's own `K` 6 and `|V|` 11, and `L_z` 3: `|x| ≤ 2⁻²³·(12K + 22|V|)`,
    so `k_E = 34·K/|E_0| + 22`, and `k_L = 6·S/|Lz_0|`. `k = 128` holds the bound for `K/|E_0|` up to 4.7 on a bound state
    and `S/|Lz_0|` up to 25.6 with a CPU reference, and up to 3.1 and 21.3 with a GPU one. The golden showcase's fresh
    sample has `k_E` 63.3 to 64.5 and `k_L` 5.08 with a CPU reference, and `k_E` up to 92 and `k_L` 6.1 with a GPU one;
    Metal's observed differences, +5 ULP of `E_0` and −1 ULP of `Lz_0`, are 2.6 and 0.7 in these units, inside the bound
    and not what sets it. **128 is** the first power of two above the showcase's worst case with either reference, and a
    power of two is one number in f32 and f64; 64 fails even the CPU form, at 64.5. The band reads 2⁻¹⁶ of the reference,
    far under the energy-suspect threshold's 2⁻⁸. No GPU kernel writes real samples until M4, so
    `drift_view_neutral_fresh_sample_renders_exactly_neutral` takes its references from the CPU read side, and
    TASK-M4-05 inherits the check on a GPU-computed reference.
    **The edge is hard.** A colour past the band keeps its own place, so no other tile moves, and `t` jumps at the band's
    edge: from 0.5 to 0.75 at `|E_0| = 0.1168` (dd test 2) and to 0.87 at `|E_0| = 1`, under `ε_E = 10⁻⁶`, and to the
    mirror places on the negative side. So a drift oscillating through the edge reads as on and off, and a pixel within
    rounding of the edge can differ between backends by that whole jump: the band moves this noise to the edge, where a
    fresh sample isn't, rather than removing it.
    **At or near a zero reference** the band narrows to nothing, and is empty at `reference = 0`, where only an exact 0
    is neutral. The rounding there is still relative to `K + |V|` or `S`, so a fresh sample with `E_0` or `Lz_0` near a
    cancellation (a near-parabolic start, or a start with `L_z ≈ 0` and moving bodies, the figure-eight among them) can
    show a rounding-level drift off `c0`. It errs toward showing: the band never hides more than `k·2⁻²³·|reference|`.
  - **The ramp.** `symlog` and `lin` draw on `diverging(c−, c0, c+)`, `c−` `#3B4CC0`, `c0` `#DDDDDD` and `c+`
    `#B40426` in 8-bit sRGB, Moreland's cool-warm ends and neutral, mixed in linear RGB by `ramp_diverging`, `c0` at
    `t = 0.5`; under the palette swap, the view's `PALETTE` uniform 1, they draw on viridis, so `symlog`'s pixel is
    `dbg_sentinel(s, frag_xy)`. `log` draws on viridis whatever `PALETTE` holds, never on `diverging`, and the view's
    header states that it shows `|drift|`.
  - **The suspect styling, `dbg_suspect(rgb, x, energy, lz, frag_xy)`,** on the ramp's output `rgb`: lines one pixel
    wide every fourth pixel from `p = ⌊frag_xy⌋`, in framebuffer pixels as the hatch's stripes are, drawn in ink,
    black where `rgb`'s relative luminance (Rec. 709's weights, `0.2126 R + 0.7152 G + 0.0722 B`, linear) is at least
    0.18 and white below, so the lines stay visible on every part of either ramp. **Which predicate drives which
    style:** the energy-suspect predicate (`suspect_energy`, relative `δE`) draws rows, where `p.y & 3 = 0`; the
    L_z-suspect predicate (`suspect_lz`, absolute `ΔL_z`) draws columns, where `p.x & 3 = 0`; both, a grid. Both
    views apply both styles, each from its own predicate, so either view shows which predicate a sample lights. A
    pixel off the lines, and every pixel of a sample neither predicate lights, is `rgb` unchanged; the hatch is never
    styled. The predicates are the generated read side's, read at read time over the stored latches, `dE_max`, `E_0`
    and `dLz_max` (dd_integrator §3.5; payload §5), the energy one at the view's `EPS_E`; no suspect bit is stored and
    none reaches `dbg_sentinel`, whose signature is unchanged (R-379). Lines, not a tint, so the ramp colour between
    them still reads as the drift's place, and rows and columns, not two colours, so the two predicates are told apart
    without colour vision. What a lit predicate means depends on the occupant: on a symplectic one the energy drift
    measures the dynamics (close-encounter stress), on any other the integrator's error (integrator contract Part 2),
    which the read side's `drift_suspect` carries beside the predicates as the occupant's `symplectic` flag.
- **The hatch, `debug_invalid(frag_xy)` (proposed, R-71; REQ-COL-055; R-132, R-136):** diagonal stripes 4 px wide
  across `x + y`, at pixel `p = ⌊frag_xy⌋`: `((p.x + p.y) >> 2) & 1` selects violet `#9B00FF` (0) or cyan
  `#48FFFF` (1), 8-bit sRGB. Neither colour collides with a palette entry. In OKLab, violet is 0.142 from its nearest
  entry (plasma, near its stop 71) and cyan 0.112 from its nearest (Turbo, between its stops 83 and 84). The palettes
  measured are the outcome palette with `#E034C6` (colour_composition §1.4), the `dbg_*` palettes, every LUT of
  colour_composition §7.1 (Viridis, Cividis, Plasma, Magma, Inferno, Twilight, Cool-warm, Principia, Cubehelix and
  Turbo, each as its ramp draws it, between its stops too), the grey ramp and the OKLCH hue circle at L 0.75, C 0.12.
  The flat magenta R-16 kept, `#FF00FF`, is 0.101 from `#E034C6`. The two colours are 0.35 apart in OKLab lightness, so
  the stripes stay visible without colour vision. `dbg_hash_u32` can
  give any colour, but never a pattern. The human confirms the pattern and colours at the M1 gate.
  **Confirmed at the M1 gate as proposed (R-428):** violet `#9B00FF`, cyan `#48FFFF`, stripes 4 px wide.
  With R-445's triple ejection, purple `#611597`, in the outcome palette, the nearest entries are unchanged: purple is
  0.193 from violet and 0.580 from cyan, and R-447's OKLab-uniform swatches at least 0.235 and 0.196
  (colour_composition §1.4; REQ-COL-104, REQ-COL-105).
  **Under each colour-vision simulation (R-429; REQ-COL-061, R-386)** the hatch stays distinguishable from every
  palette entry above, by the criterion confirmed beside it: τ is the OKLab distance from `#FF00FF` to `#E034C6`,
  0.101; under each simulation it passes by colour, each stripe colour farther than τ from every palette entry, both
  simulated, or by the pattern, when the two simulated stripes are more than τ apart. The pattern argument holds at
  1:1 device pixels only; a downscaled export blends the stripes to a flat colour, which under achromatopsia sits on
  the grey ramp (R-429).
- **The "not yet" stipple, `debug_not_yet(frag_xy)` (R-399; proposed at the M1 gate, R-71; REQ-COL-053,
  REQ-COL-064):** "not yet", an unset `d_min` (R-271, R-280) and a `running` sample (R-96) alike, is the neutral grey
  `DBG_NOT_YET` with a fine dot stipple computed from the pixel position `p = ⌊frag_xy⌋`, as the hatch is. It is dots,
  not the hatch's diagonal stripes, so the two never read as one another, and it is never one flat colour, so no ramp
  value, greyscale included, collides with it. The grey's value (REQ-COL-053, proposed by TASK-M1-09) and the
  stipple's pattern (REQ-COL-064, proposed by TASK-M1-17) are confirmed by the human at the M1 gate. It lives in `present.wgsl`
  beside `DBG_NOT_YET`; every view that shows "not yet" calls it. The outcome view does not: there `running` samples
  draw bounded's `#141418` (R-446; colour_composition §1.4).
  **Confirmed at the M1 gate:** the grey `#4E4E4E` (R-433), and the stipple as a 2 × 2 px dot of `#909090` in each
  4 × 4 physical-pixel cell, where bit 1 of `p.x` and of `p.y` are both 0, its dots changed from the proposed `#808080`
  (R-434). No flat colour of any palette comes within the hatch's τ = 0.101 of every pixel of a 4 × 4 block (the
  nearest, 0.115), and the dots are 0.229 lighter than the grey in OKLab, visible without colour vision.
- **Presentation views draw no hatch (R-467).** A presentation view,
  every stain whose colour occupant is not a debug view (Part 6: a `debug/` occupant), never draws `debug_invalid`: a
  pixel the chart classifies as not a system (`principia_chart_decoder_contract.md` Part 3) draws the canvas
  background, a uniform the GUI sets to the colour its window is cleared to, and a `sim_failed` or `decode_failed`
  sample, a reserved `state` code, a non-finite value and a ramp's invalid value draw `#141418`, unless a node overrides
  its invalid colour. The "not yet" stipple is unchanged. The hatch stays the debug views': View › Debug view
  (`principia_render_gui_spec.md` §G2) draws it for a forbidden pixel and for a decode or sim failure, and a NaN / Inf
  style of its own for a non-finite value, a calibration (REQ-COL-110; R-71), each with a legend key and a count. The
  task that needs the style proposes it; it is not magenta, which sits at τ from escape body 2 (R-505).
- **Blank in every view, the hatch an optional overlay, the failures debug-only (R-470).** The background a pixel that
  is not a system draws is blank, the view's background colour, never a palette colour in any view, the outcome view
  included. The hatch is drawn outside the debug views only by the optional overlay Overlays › Chart › forbidden region
  (off by default), `debug_invalid(frag_xy)` over exactly the pixels the chart classifies as not a system and the
  `decode_failed` samples (R-511), and nothing else (REQ-COL-111). NaN, `±∞`, `sim_failed`, `decode_failed` and the chart's classification are shown only in the
  debug views. View › Debug view is one of them, built as a baked variant of the render's pipeline (R-41: a variant,
  never a flag bit): `RenderState`'s `debug_view` selects the variant compiled with the debug arm, and the presentation
  variant is compiled without it, so a presentation view cannot draw a debug style. A `decode_failed` sample the chart
  does not classify draws blank in a presentation view, as a pixel that is not a system, the overlay drawing over it;
  `sim_failed` keeps `#141418` (R-511, ruling RQ-379). *Was:* "A presentation view's `#141418` for a `decode_failed`
  sample the chart does not classify is RQ-379's to confirm."

### Live-state & array inspection

- **Live shape views** (`u_mode`): mode 0 → `0.5·(n+1)` direction cosines RGB of the *current* derived `n`; mode 1 → cyclic map (Twilight) of the running unwrapped phase `θ̃`; mode 2 → `|n|−1` as error view (normalisation damage made visible — should be flat zero, since `n` is derived fresh each step).
  Mode 0 is `dbg_dircos3(n, frag_xy)` (Part 5). Where `n` is undefined (a coincident configuration, `I = 0`, gives
  `0/0`) or `θ̃` is not finite, the mode reading it draws `debug_invalid(frag_xy)`, the hatch, as every colouring maps
  NaN or a sentinel to its invalid colour (Part 4; TASK-M1-12, applied per R-369).
- **Accumulator views**: Benettin `S/t` (FTLE-running — a live approximation; the finalised read is `S_final/(n·dt)` with the partial renorm interval closed, payload §5), diffusion slope = C_ty/C_tt from the Welford accumulators, drift running-max vs running-final.
- **Word inspector**: `fgw_reduced_length` as scalar view (styled invalid when truncated); symbol-at-slot-k via a slot slider; `fgw_truncated` (the 127-sentinel) as flag view.
- **ICDescriptor views**: masses as ternary colour, `virial_ratio`, `rho_ratio`, `rho_angle`, `r_min_pair_0` as scalar views — these certify the *decoder*, independent of any integration.

---

## Part 6 — The debug catalogue (first build target)

**Every debug view is an ordinary slot occupant** — a colour source of the form `present(unpack(ctx))`. The catalogue is **generated from the layout table**, so it is exhaustive *by construction*: a new struct field automatically gains a view (or fails to compile — also a signal). The UI debug picker is auto-populated.

**The catalogue is a test suite where the display is the assertion.** Two kinds of view: **field views** certify a single producer; **cross-check views** certify the *contract between* two producers. When a view looks wrong, you know which side — or which seam — to suspect.

**Generation is fully mechanical because the layout table carries per-field presentation metadata**: `{scale: lin|log|cyclic, range, sentinel?, categorical_n?, tier_gate?}`. So the drift fields `energy_drift` and `Lz_drift` default to `diverging · symlog` with the field's floor (`eps_E`, `eps_L`), the preset `f_edrift`'s, and `lin` and `log` are available through the compaction override (`principia_colour_composition.md` §1.2, §6; R-381); `t_end` → lin [0, T], `rho_angle` → cyclic, `diffusion` → lin, NaN (invalid fit) hatched (R-245). **A field without metadata fails generation loudly** — coverage of every struct member is enforced, not hoped for.

### Field views (one per field, every struct)

| View group | Reads | Certifies |
|---|---|---|
| UV view | fragment preset: `ctx.screen.uv` or `ctx.quad.uv` → RG (`principia_colour_composition.md` §6; R-75) | quad-local coordinate addressing. Smooth gradient = healthy; **banding = f32 precision loss made visible** |
| DECODE view | fragment preset: fragment-side decode of `ctx.chart.z` (colour_composition §6; R-75) | chart map + decoder before any physics: z components, masses (ternary), positions — Phase-0d "raw IC colouring". Its agreement presets (fragment-decode `E` against `ctx.payload.E₀`, and the like) check the WGSL decode (translated from the one Rust source, R-116) against the kernel's, within a stated tolerance, a calibration requirement, not bit-exactly (R-297) |
| `ICDescriptor` — all 12: masses as ternary, `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio`, `rho_angle` (cyclic), `K_0`, `V_0`, `virial_ratio`, `r_min_pair_0` | `ctx.ic` | decoder + canonicaliser per sample, field by field |
| `SimState` scalars — `d_min`, `ftle` (S_final/elapsed; NaN whenever `ftle_valid` is false, R-253, R-254), `diffusion` (NaN when `n < 2`, R-245), `dE_max`, `dLz_max`, `E_0`, `Lz_0`, `S`, `theta` (cyclic, R-423), `mean_y`, `C_ty`, `closure_min` (log — periodic orbits read as holes) and `closure_step` (the period label; families with related periods band together; pending change 9), and the exact `total_substeps` u32 | `ctx.sample` | the integration kernel. **NaN is a deliberate sentinel** for a tier-absent feature only — read at unpack, never stored (lowering Part 3a / author contract above); a blown-up sample stores the defined failed-state values and is marked by its `state` (R-79). The "no-NaN in a *valid computed* value" convention still holds for finite results, and sentinel styling flags the NaN and stored-sentinel cases distinctly so a NaN pixel reads as "unavailable", not as data |
| `sample_descriptor` sub-fields — 10 bits: `state` (6-valued enum incl. running/sim_failed/decode_failed), `detail` (union keyed by state), `saturated` (stored sticky bit), `dmin_pair`, `last_symbol` (bits 8–9); bits 10–15 reserved (R-86) | unpack layer | **the bit packing itself** — pack and unpack generated from one table, so a garbage view means table ≠ kernel write, caught day one. `saturated` is a **stored** sticky bit (`N_sub==N_max` occurred), NOT a derived predicate. The `total_substeps_log2` complexity proxy is **derived** at read (⌊log₂ `total_substeps`⌋ via `countLeadingZeros`, 0 for a total ≤ 1 — R-86), not a descriptor field. The **live current-substep count** animates separately (live view below), read off the march |
| `times` (t_end_step, t_dmin_step — EXACT u16 indices) + derived (orbit_count/retrograde from theta; total_substeps_log2 from the u32) | unpack/derive layer | round-trip packed fields; derived match live source; fraction via horizon_steps uniform |
| Live shape & accumulator views: current `n` → direction cosines / running `θ̃` → cyclic / \|n\|−1 error (flat-zero expected); FTLE-running `S/t`; diffusion slope from moments; drift max-vs-final; **live current-substep count → effort heatmap (animates: close encounters propagate in time)** | live state + accumulators | the live march itself — shape derivation, phase unwrapping, accumulator bookkeeping, substepper effort |
| Word views: `fgw_reduced_length` (invalid-styled when truncated), symbol-at-k (scrubber), truncated sentinel, **whole-word hash** (`dbg_hash` of the uint4) | `free_group_word` | symbol packing — and the hash view renders **topological basins**: word boundaries are finer than outcome boundaries, so this is the Burrau topological-boundary diagnostic, free at debug level |
| Ensemble views (tier-gated): outcome agreement, spread — **derived at resolve** from the footprint's E+1 samples (not a stored field), consumed live & aggregated to the quad | the ensemble/SSAA machinery (E Halton-(2,3)-offset copies per nominal sample) |
| Quad fields — all 9 (depth, state enum, coherence, impurity, spread, suspect fraction, priority, cache age, ancestor gap) + payload/status flags (sim-failed, cache-valid, contains-ensemble, contains-FTLE, schema version) | `ctx.quad` | the **CPU scheduler** and the payload compatibility signature — CPU-written, so a wrong view here exonerates the GPU |
| Structural overlays (quadtree boundaries, active leaf outlines, fallback tint, pending hatch, visible-set, locked/stale) | post node + quad + `ctx.uv` | the quad/instance render path and cache behaviour |
| **Uniform echo** — flat swatches of `quality_tier`, `n_renorm` (R-111), thresholds *as currently bound* | `SimUniforms` | the CPU→GPU binding path — catches "slider moved but nothing rebound" |

**The uniform echo row and the Fourier row (R-420).** The uniform echo row is not on the M1 checklist (REQ-TOOL-011):
it is built and ticked by TASK-M5-31, after TASK-M5-02 (`quality_tier`) and TASK-M3-06 (`SimUniforms`), against a real
binding (REQ-TOOL-164). That task names the thresholds the echo shows, and the binding it reads `quality_tier`
from, since `SimUniforms` holds no tier (REQ-TOOL-165, R-72). The Optional Fourier block row (`|a_k|` per k and ω,
from the quad payload, certifying "the truncated-Fourier path when enabled") is removed from this table: nothing
current defines the path, and it is parked as a possible future feature (`principia_00_philosophy.md` §7.6a).

**The quad fields row's flags (R-424).** At M1 no flag is a `RenderQuad` member (dd_generation_root §3.7a), and
the checklist (REQ-TOOL-011) checks contains-ensemble against the prelude's `has_ensemble()` (Part 2; R-145), as the
spread view reads "present iff contains-ensemble", contains-FTLE against `has_ftle`, and the schema version against
the build's generated `PAYLOAD_SCHEMA_VERSION`. sim-failed is defined and shown by TASK-M5-17, the `QuadReduction` task
(REQ-PAY-096, R-72), and cache-valid by TASK-M5-08, the M5 cache task (REQ-SCHED-099, R-72), which also decides
whether a per-quad schema version, the drawn payload's own (caching contract Part 1), belongs with cache-valid: it
can differ from the build's only once quads can come from an older build's cache.

**The visible-set and locked/stale overlays (R-425).** The structural overlays row's visible-set and locked/stale
overlays are not on the M1 checklist: TASK-M5-29 defines them (REQ-TOOL-166, R-72), in debug_tooling_plan §F,
reusing the corpus's existing meanings where they exist (for example, `quad_state` 4, dd_generation_root §3.7a, for
"stale") rather than defining new ones, and draws them on the Tier 3 path (render_gui_spec §12.1).

**The word views' renderings (TASK-M1-12).**
- **The invalid-styled reduced length (R-72; REQ-TOOL-156):** where `fgw_reduced_length_valid` is false, the word
  truncated, the reduced-length view draws `debug_invalid(frag_xy)`, the hatch; elsewhere it draws
  `fgw_reduced_length` by `dbg_lin` over `[0, fgw_capacity]`, `[0, 76]`. The raw `length` view still shows the stored
  127 literally, on the ramp by `dbg_sentinel` (R-79, R-136).
- **Symbol-at-k:** the slot `k`, `0` to `fgw_capacity − 1`, is the view's slider uniform; the symbol there,
  `fgw_symbol(word, k)` (payload §3), draws by `dbg_cat(s, 4)`. A slot at or past the word's length, and every slot of
  a truncated word, holds no symbol and draws the hatch.
- **Truncated:** `fgw_truncated` by `dbg_flag`.
- **Whole-word hash:** `dbg_hash_word` of the word (Part 5, REQ-TOOL-155); a truncated word draws the hatch.
- **Last symbol:** `last_symbol` has no in-band "none" code; it is meaningful iff `length ≥ 1 && length ≠ 127`
  (payload §2), so its view draws `dbg_cat(s, 4)` there and the hatch for the empty word and a truncated word
  (applied per R-369).

### Cross-check views (the seams)

Field views are unit tests; these are the integration tests. Each compares a value against an independent computation of the same thing:

| View | Compares | Certifies |
|---|---|---|
| **Invariant-chart gradient** | `E_0` (or `Lz_0`) rendered on an (L_z,E)/(L_z,K) chart | **the invariant downstream solve**: must be a perfect axis-aligned gradient; any deviation = broken solve or feasibility handling. The strongest single test in the catalogue — one glance certifies the hardest chart machinery |
| Energy agreement | `E_0` (SimState) vs `K_0 + V_0` (ICDescriptor), log \|Δ\| | decode-time vs kernel-time energy computation across the two structs |
| Winding consistency | derived `orbit_count = ⌊|θ̃|/2π⌋`, `retrograde = θ̃<0` vs the running `θ̃` accumulator | the derivation vs its live source; the terminal-latch policy |
| Diffusion (Welford) | slope `C_ty/C_tt` in the fragment vs a reference computed the same way | the moment bookkeeping — an independent *fit* needs history (gone under lockstep); the algebraic re-derivation certifies the accumulators instead. Full fit-vs-fit lives in the CPU parity suite via `computeIC` |
| Word bookkeeping | `fgw_reduced_length` vs the derived reduced-crossing count (topological word read) vs `fgw_truncated` | the symbol-append rule vs crossing derivation (word buffer) |
| t_dmin round-trip | `tm_t_dmin_step` absolute macro-step (no longer needs t_end) | the 16-bit timing quantisation |
| Impurity mask | per-sample `class ⊕ detail` ≠ the quad's `dominant_outcome`, at the joint grain (`principia_dd_generation_root.md` §3.7; **no `majority_class` field** — R-20; `RenderQuad` exposes `dominant_outcome`) | sample↔quad reduction agreement; the mask's spatial mean must equal `outcome_impurity` |
| Drift shape | `energy_drift` (final) vs `delta_E_max_abs` (max); same pair for L_z | secular loss vs a transient spike that recovered — the *reason* both are kept (the final derived at read, §3.1; the max an accumulator; R-246) |
| Round-trip residual | `z → D → E → D`, physical-space `‖x − x′‖` log-scaled — a fragment preset, `encode(decode(z))` recomputed in the fragment (R-75) | the **encode path** (inverse-encode contract): block inverses, numerics, encode-reuses-decode on invariant charts — the encode-side sibling of the invariant-chart gradient. Bright at clamps/feasibility edges/mirror tie = *expected*, tagged |

**A non-finite operand is invalid (R-436).** The energy agreement's operands reach `±∞` at the poles:
`K_0 + V_0` is `−∞` where `V_0` is (the f64 collision pole, `principia_dd_decoder.md` §3.6). The view tests each
stored operand, `E_0`, `K_0` and `V_0`, on its bits before any arithmetic, `K_0 + V_0` included, and draws a pixel
with a non-finite one in the invalid colour, on the CPU and the GPU alike, never `log|(−∞) − (−∞)|`; every
cross-check view and agreement preset that reads a stored value does the same.

**The kernel keeps one debug mode (R-75)** — `principia_colour_composition.md` Appendix A's bring-up mode: the kernel writes a known pattern instead of physics, for when payload writes are too broken for the fragment-recompute presets to run. It is a baked kernel variant (lowering contract Part 3), not a dispatch flag (R-41). It skips integration and writes into the **existing payload slots** — no new buffers. UV, DECODE and ROUNDTRIP are fragment presets (the rows above), not kernel modes.

**Why first, concretely:** (1) you cannot write the quad-depth heatmap until `RenderContext.quad` is plumbed — building debug first *forces the contract into existence*; (2) the whole substrate is testable with **synthetic CPU-filled payloads** before one line of physics exists — the direct fix for the original black-screen failure; (3) the sub-field views are the visual unit test of the bit layout. Build order stands: contract + SDK → debug catalogue → flat compute → quads → adaptive.

---

## Part 7 — The hover trace (per-IC trajectory overlay)

> **Full spec: `principia_trajectory_viewing.md`.** This part is a pointer + the render-side facts (compositor placement, the pull's sanctioned status). The mechanism below supersedes an earlier tiered/checkpoint-readback draft — there is now **no** checkpoint-based hover path and **no** per-axis tier table.

**The interaction:** hovering an IC pixel overlays the trajectory that IC follows, **projected onto the current chart's own `(q₁, q₂)` plane** (whatever tilt/slice made it), drawn on the survey. Click escalates to the full inspector surface (`trajectory_viewing.md` §4).

**One mechanism, no tiering.** Hover **always integrates the hovered IC on the CPU** — one f64 `computeIC` (the Precision-ring path shared with parity and the click inspector), returning the full state `r(t), p(t)`, which is then projected onto `(q₁, q₂)` and drawn. There is deliberately **no** "instant from the GPU payload" path: under lockstep `SimState` holds live state only — no trajectory history exists to draw from — and a single three-body trajectory is cheap and CPU-side, so the old tiered/checkpoint optimisation was removed as needless complexity that drew from data the payload never truly contained. The full CPU state supports **any** projection uniformly — config, momentum, real space, tilted combinations — so there is no case analysis.

- **Tilt deforms the trace, slice repositions it.** Projecting onto the *actual* basis vectors (never a "dominant named coordinate") means a conserved direction contributes zero motion and a live one contributes its share: rotate a conserved axis toward a live one and the trace goes dot → line → curve, live. (Details in `trajectory_viewing.md` §3.)

**Discipline.** One integration per settled hover, in a **dedicated inspector worker** (separate from the render-loop worker so its heavy f64 integration cannot stall the frame loop), **debounced + cancel-on-move** (generation counter, latest-wins), never awaited — it never blocks the main thread or the GPU survey. This is a *user-initiated* action outside the automatic refinement loop (it does not even read the payload — it re-integrates from the decoded IC).

**Compositor placement (render-side fact).** The trace is an overlay layer above the stain graph, pure display state, drawn by the line system (R-476; `principia_lines.md` §8: projection, fade, breaks, markers and styles), which the orbit viewer reuses; it breaks where the path leaves the chart, never joined across a pixel that is not a system. **No trace over the backdrop:** if there is no current-identity payload under the cursor (a stale/blurred region), there is no IC to stand behind the cursor, so no trace — consistent with blur meaning not-current.

---

## Part 8 — Errata against the older design docs

1. **`TileUniforms { cu: f64, … }`** in the render/quadtree design doc (the old name — now the per-QUAD uniforms, lowering Part 5): **WGSL has no f64.** Correct form (already in the deep-zoom note): CPU computes centre/half-width in f64, uploads **f32** uniforms; shader arithmetic is quad-local.
2. **`ftle` in the payload vs "Lyapunov = separate kernel"** in core design: reconciled — forward-pass co-computations (each sample's own Benettin shadow; the E ensemble copies) are gated by tier flags; "separate kernels" means separate *protocols* (reversibility's forward+reverse). Core-design doc amended.
3. **"Custom nodes"** phrasing anywhere: custom is not a node kind; it is an occupant of a node (Part 2). The stain graph is free inside a fixed backbone (R-64).

---

*One payload, one typed stain graph on a fixed backbone. Render changes never touch sim buffers. One layout table generates pack, unpack, export, and the catalogue. The debug views are the assertions — built first, against synthetic data, before any physics exists.*
