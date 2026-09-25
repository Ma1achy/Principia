# Principia — Colour Composition

*Status: canonical. Single source of truth for the **colour occupant** — the internally-compositional
system that produces the `colour` and `brightness` values consumed by the render pipeline's
`combine` stage. Supersedes the implementation sections (§5–§9) of the retired shape-sphere colour-map
PDF and the mode-enumeration in `principia_debug_tooling_plan.md` §B–§G. The outer 4-slot pipeline
framing in `principia_gui_state_contract.md` §4 and the colour drill-down `principia_dd_colouring.md`
are amended to defer here (§8).*

*Design thesis: the retired PDF enumerated **products** where the system has a few **factors**. Nearly every
named colour map is one primitive family under different parameters; the LUT-spheres, the vMF map,
Voronoi, soft-Voronoi, basin-blend, the physics overlay, and custom N-pole are **the same primitive**.
Enumerating them is a maintenance liability and a second colouring path that does a strict subset of
what composition does. This document replaces the catalogue with an algebra, and re-expresses the
catalogue — and the entire debug-view set — as a **preset table over that algebra**.*

---

## 0. Scope & membrane position

Colour lives entirely on the **runtime-authorable hand-WGSL side** of the GPU↔CPU membrane. The
compute kernel (monomorphised Rust → SPIR-V/f64) is **untouched** by anything in this document;
determinism-law constraints do not apply here. Everything specified is **render-key**: editing it
never invalidates the survey cache and never re-integrates. (The one exception — a kernel-side
bring-up pattern — is quarantined to Appendix A and is the only sim-key item.)

The subsystem has three data sources, one read interface (§3 `ctx`), one primitive algebra (§1),
one pipeline shape (§4), one codegen path (§5). Debug views are presets over exactly this (§6).

---

## 1. Primitive algebra

A colour occupant is an expression tree over two primitive **families** and a small set of
**combinators**. Each node's output type is `vec3` (a colour) or `f32` (a scalar/lightness); the
slot's required output signature (`vec3` for `colour`, `f32` for `brightness`) constrains only the
root.

### 1.1 Family A — site-blend  →  `vec3`

The workhorse. Regenerates ~20 of the PDF modes.

```
SiteBlend {
  sites  : SiteSet          // §2 — an ordered set of unit directions p_i on S²
  kernel : Kernel           // how per-site weights are formed from d_i = n̂·p_i
  colours: SiteColours       // one colour per site
  space  : BlendSpace        // where the weighted mean is taken
}
→ blend( colours , weights(kernel, {n̂·p_i}) , space )
```

**Kernel** = `support × temperature`. This is the continuum the redesign asserts: hard assignment,
soft-Voronoi, and vMF blend are one primitive at different temperatures.

| kernel        | support   | weight law                                             | temperature |
|---------------|-----------|--------------------------------------------------------|-------------|
| `vmf(κ)`      | all sites | `w_i = exp(κ·(d_i − d_max))`                            | κ ∈ [0.5,12] |
| `topk(k,ks)`  | nearest k | softmax over the k largest `d_i`, sharpness `ks`       | ks ∈ [1,20] |
| `nearest`     | 1         | `w_i = [i = argmax_i d_i]`                              | — (= κ→∞ = topk(1,∞)) |

`nearest` is the κ→∞ / ks→∞ limit but **must be a discrete code path** (`exp` overflows). Fork (c)
is settled: the UI presents temperature as one **blend-sharpness dial with a hard detent at the top
edge** that snaps to the `nearest` path; `support` (all-site vMF vs top-k Voronoi) is the discrete
choice that distinguishes the vMF family from the Voronoi family. `d_max = max_i d_i` is subtracted
for numerical stability (shifts weights, not their ratios).

**BlendSpace** — an **explicit parameter**, not an accident of implementation:
- `oklab` — blend in OKLab (a,b) at the site colours' (L,C). Weighted-mean chroma **shrinks toward
  the boundary between sites**, so uncertainty reads as desaturation. This is a *feature* and is the
  perceptual default. It is a property of the blend space, **not** of any "hue table."
- `rgb` — blend in linear-ish sRGB. Used where site colours come from a perceptual LUT already
  (the LUT-sphere construction, §7) and further OKLab mixing is undesirable.

**SiteColours** — fork (b) is settled: **hue tables are dissolved.** A site's colour is *always* a
**swatch** (any OKLab colour). There is exactly one colour-assignment concept. The Full-OKLAB and
Okabe–Ito "hue tables" become **preset swatch-sets** (six swatches at fixed L, C, and the tabulated
hues). Colours may be authored directly, drawn from a palette generator (golden-angle, OI-cycle,
gradient A→B), or **sampled from a LUT at `i/N`** — the last of which is precisely how a LUT-sphere
is built (§7). Fidelity to the map list (§7.1) is pinned by golden-image tests (§7), not by a special type.

### 1.2 Family B — field-ramp  →  `vec3` or `f32`

For everything that is *not* a directional blend: a scalar field mapped through a ramp.

```
FieldRamp {
  field : ScalarField        // → f32 (+ a validity lane, §3)
  ramp  : Ramp               // f32 → vec3   (colour)   OR   Compaction : f32 → f32 (lightness)
}
```

**ScalarField** sources (all read through `ctx`, §3):
- **payload** — any per-pixel kernel output: `state`, `ftle`, `energy_drift`, `Lz_drift`,
  `diffusion`, `d_min`, `word`/hash, decoded-IC quantities (E₀, K₀, V₀, L_z, virial, mass ratios,
  Jacobi ρ magnitudes/ratio/angle, min-pair-dist, …).
- **geometry-of-n̂** — `n_z`, `‖n̂‖`, azimuth/polar, `stability` = ½(1 − max_j n̂·b̂ⱼ) (BC distance),
  spherical harmonic `Yℓm(n̂)`, the 3-fold Turing wave-triple, 3-D value-noise octaves.
- **ctx lanes** — `quad.depth`, `quad.impurity`, `quad.spread`, `quad.priority`, `quad.cache_age`,
  `screen.uv`, `quad.uv`, etc. (§3). This is what dissolves the structural debug views.
- **derived operators** — `gradient_magnitude(map)` (finite-difference of another colour node —
  three evaluations at ±ε), `topk_margin(sites)` = `d_1 − d_2` (drives soft-Voronoi/basin edges).

**Ramp** (scalar → colour): `lut(name)`, `lerp(c0,c1)`, `diverging(c−,c0,c+)` (through a neutral),
`bands(field, n, line_col, base)` (a **band-mask**: line colour where the field falls in a periodic
band, base colour elsewhere — this is `grid`, `contours`, and the lattice classifiers `checker`,
`lat/lon-stripes`, `truchet` on (θ,φ)). **Compaction** (scalar → lightness, for the `brightness`
slot or before a ramp): `lin`, `log`, `symlog` (signed, through the midpoint), `cyclic` (phase),
`flag`. Every ramp/compaction carries an explicit **invalid-pixel treatment/value** — by default the hatched invalid
pattern (§3, §6; R-132).

**Default ramps by field role.** Signed fields (energy, L_z, drifts) default to
diverging-through-neutral so the zero-crossing is a legible contour; positive fields to sequential;
angles to cyclic. **Magnitude / diagnostic fields default to *greyscale*, not a sequential LUT** —
because a greyscale magnitude field *is* a lightness, so the same field doubles as a natural
**brightness occupant** (§4.1). Polarity is **per-field**, set so the *salient* end is bright, and
is not uniform:

- **FTLE, diffusion, ensemble spread → white = high** (the *magnitude* pops — chaos, phase-space
  spreading, or ensemble dispersion; high value = bright, the standard convention, so for spread
  white = low certainty and black = high certainty);
- **time-to-event (`t_end`) → white = low / early** (quick-resolving pops; late and bounded darken,
  keeping bounded = black consistent with §1.4).

The payoff is composition: `combine(colour = shape-sphere map, brightness = FTLE-greyscale,
Replace-L)` modulates the position map's lightness by chaos (unstable brightens, regular darkens) —
a clean bivariate encoding (§4.1). Every default here is customisable; the greyscale and its polarity
are only the defaults, chosen so these fields compose well as the brightness channel.

### 1.3 Combinators  →  `vec3`

Compose sub-results. All are `vec3(+ctx) → vec3`.

- `mix_const(a, b, t)` — constant-weight blend (the "blend with second map" control).
- `mix_field(a, b, field)` — blend weight from a scalar field.
- `bandmask(base, field, band, line)` — overlay lines/tiles where a field is in-band (grid/contours
  over a base map; **quad-boundary overlay** on the normal render is this with
  `field = distance-to-quad-edge`).
- `site_overlay(base, SiteBlend*)` — site blobs over a base, blended as a sequential clamped mix (the oracle's form,
  `principia_dd_colouring.md` §3.4, R-122). **The physics overlay is
  exactly this**: `site_overlay(base, SiteBlend{ sites = physics(m), kernel = vmf(κ), colours =
  per-site })`. It was never a distinct node; it is Family A used as a combinator, with a physics
  site generator (§2) and per-blob strength `s`.

*The whole PDF catalogue is: two primitive families + four combinators. Adding a new map is wiring,
not a new pixel function.*

### 1.4 Categorical colour-assignment — the outcome-state default palette

Categorical fields (outcome `state`, encounter-word hash, F₂ conjugacy class) map through a
**palette**: one colour per class, hue carrying *identity, not magnitude*. Most categorical palettes are
**generated** — golden-angle for the many-class word/basin field (adjacent basins stay hue-separated),
an Okabe–Ito cycle for small class counts. This is the categorical case of §1.1's dissolved colour
assignment: a class→colour map is a swatch-set.

The outcome **`state`** field is the one categorical field with a **canonical default palette**, because
its nine terminal classes carry structure worth encoding in the colours themselves:

| class | colour | sRGB | | class | colour | sRGB |
|-------|--------|------|-|-------|--------|------|
| collision 0–1 (pair 2) | red     | `#DE2D2D` | | body 0 escape   | yellow  | `#F0DE32` |
| collision 0–2 (pair 1) | green   | `#2EBC4E` | | body 1 escape   | magenta | `#E034C6` |
| collision 1–2 (pair 0) | blue    | `#3462E0` | | body 2 escape   | cyan    | `#30C8DC` |
| bounded        | black   | `#141418` | | collision @ t=0 | orange  | `#F29620` |
| degenerate     | white   | `#ECECF0` | |                 |         |           |

The assignment is a **mnemonic, not arbitrary**: **collisions are additive primaries keyed by the
colliding pair** (0–1 → R, 0–2 → G, 1–2 → B); **escapes are subtractive primaries keyed by the
escaping body** (0 → Y, 1 → M, 2 → C; 0-based, R-22). The two event *families* (collision vs escape) are therefore
separable at a glance while the pair/body identity stays legible; the three non-generic outcomes are
bounded (black), the t=0 collision (orange), and degenerate (white). The nine classes read from the
`state` enum **plus the `detail` union** — collision → pair id, escape → body id (so the R/G/B/Y/M/C
assignment lands on `detail`, not on a second field). **How the table reads the payload (R-96):** "degenerate" is
`decode_failed`; "collision @ t=0" is a collision with `t_end_step == 0`. The two states the table has no row for:
`running` shows neutral grey, and `sim_failed` shows the invalid pattern (§3). There is **no separate `escaper` field**: the
escaping body *is* `detail | state=escape`, so “which body escaped” is already carried by this map's
escape colours. A standalone escaper view is therefore this map **filtered to the escape classes** — a
**categorical filter** (`show class ∈ {…}, mute the rest`), which is a general operation any categorical
mode admits (“just collisions”, “just body-2 escape”), not a distinct render mode.

Like every colour assignment in the system, **this is a default, not a fixed mapping** — the
class→colour swatch-set is user-editable. It is the canonical default the render-mode catalogue's
outcome-state row inherits (that catalogue is out of scope here; this palette is the one piece of it
that is settled). It governs the outcome palette (`state ⊕ detail`) only: the raw `state` debug view keeps a six-colour
`dbg_cat` palette (`principia_debug_tooling_plan.md` §B, R-115).

---

## 2. Site-set kinds

Sites feed Family A and `site_overlay`. Two kinds, distinguished **in the type**, because one is
pure geometry and one depends on the decoded IC.

**Static generators** — functions of parameters only; **uniform-hoistable** (computed once, bound as
a uniform array):
- `axes6` — the six ±axis poles.  `corner8` — sign-octant corners.  `ico12` — icosahedron vertices.
- `fib(N)` — golden-angle Fibonacci lattice of N points.
- `ring(N, tilt, rot)` — N points on a rotatable great circle (custom N-pole).

**Physics generators** — functions of the **decoded IC** (the mass point), evaluated per pixel from
`ctx.payload` masses; **not bakeable**:
- `BC(m)` — binary-collision loci.  `Euler(m)` — the Euler central configurations (the collinear relative
  equilibria: roots of Euler's quintic in the mass ratios, mapped through the shape map; equal masses reduce to the
  antipodes of b̂; R-126).  `Lagrange(m)` — equilateral poles. On the mass-weighted shape sphere every one of these **moves with (m₀,m₁,m₂)**, and when a
  slice axis (or a tilt) touches a `z_μ` dimension the masses are **per-pixel state**, so there is no
  per-slice constant to bake even in principle.

**Hoist optimisation.** When neither basis axis nor any active tilt touches a mass dimension, masses
are constant across the slice; the codegen detects this and hoists physics sites to uniforms —
**semantics per-pixel, cost per-slice** when possible, per-pixel otherwise.

**Preview corollary.** A shape-sphere / equirect preview is a single sphere and therefore renders at
**one mass point**. Default: the slice-centre masses `z₀`. When a pixel is inspected, the inspected
pixel's masses. The preview must **state which mass point it is showing**, because physics landmarks
(and any physics-dependent colouring) are only meaningful relative to it.

---

## 3. The `ctx` contract

`ctx` is a **first-class read interface** — the single contract that production colouring, the
codegen, the egui panel, and every debug preset program against. Making it rich is what lets debug
views be presets rather than a parallel system (§6), and what makes a working debug render a **live
test of the production read path** (same buffers, same bindings, same codegen). All lanes are
render-key.

| lane          | fields |
|---------------|--------|
| **screen**    | `pixel` (ivec2), `uv` (screenspace, vec2), `target_dims` (ivec2) |
| **chart**     | `slice_uv` (vec2 in [0,1]²), `z` (the full 8-D latent at this pixel, chart triple applied), `chart_id` |
| **quad**      | `index`, `depth`, `tl` (slice coords), `centre` (slice coords), `uv` (within-quad vec2), `state` (enum), and summary stats: `impurity`, `spread`, `suspect_frac`, `priority`, `cache_age`, `sample_count` |
| **tile/sample** | `tile_index` (within quad), `sample_index`, `N` (samples/quad), `E` (ensemble) |
| **payload**   | every per-pixel field written by the kernel — `state`, `ftle`, `energy_drift`, `Lz_drift`, `diffusion`, `d_min`, `word`/hash, `t_end`, decoded-IC quantities, masses, … |
| **validity**  | the sentinel/predicate lane paired with **every** field: `ftle_valid`, the diffusion `−1` sentinel, `sd_is_failed`, out-of-chart / saturated flags, `ftle_valid` etc. |

**Validity is not optional.** Every `ScalarField` returns `(value, valid)`. Every `Ramp`/`Compaction`
has an explicit **invalid treatment/value**. Without this, debug views silently lie at exactly the
pixels they exist to expose (a NaN FTLE would ramp to *some* colour and look like data). Invalid pixels
render in a **hatched pattern** that collides with no palette entry; its exact pattern is a calibration (REQ-COL-055,
R-71; R-132). Overridable per node.

**Fragment-side recompute.** Because `ctx.chart.z` is present and the decode/encode are available in
WGSL — generated from the one Rust source (rust-gpu → SPIR-V → WGSL translation), never hand-written (R-116) — the fragment stage can *recompute* cheap quantities (decode `z` → shape/energy; `encode(decode
(z))` residual). This is what dissolves most of the §A kernel modes (§6) and enables **agreement
presets** (fragment-decode vs kernel-payload) as live checks of that translation.

---

## 4. Pipeline shape

Two-tier mutability, and it is deliberate: **the backbone is a fixed typed topology with switchable
occupants; the post chain is a variable-length ordered list.** Different mutability because they are
different kinds of thing.

```
            ┌─────────────┐
 colour  →  │             │
(Option)    │   combine   │ → post[0] → post[1] → … → post[k]  ═╡ display stage ╞═→ canvas
brightness→ │ (Replace-L/ │      (ordered chain, ≤ 8,           (fixed terminal,
(Option)    │  Multiply)  │       each vec3→vec3 +ctx)           settings not nodes)
            └─────────────┘
```

### 4.1 Backbone & `Option` occupants

`colour` and `brightness` are **`Option<Occupant>`**. Topology is **fixed** — occupants are switched
off, never structurally deleted. Fork settled: **ship None occupants; node "removal" sets None and
renders the node ghosted, not vanished** (keeps the topology legible — you can see where to click to
restore — while offering the gesture of removal). In the Rust state contract this is literally
`Option<Occupant>`: a free enum variant for typed `SetField`, serialisation, and undo. Structural
deletion would make topology variable and break the "occupants are data" invariant for zero semantic
gain.

**`None` is the identity element of `combine`.** Truth table (combiner-dependent):

| colour | brightness | Replace-L result | Multiply result | meaning |
|--------|------------|------------------|-----------------|---------|
| C      | B          | `OKLab(L=B, a=Cₐ, b=C_b)` | `C · B` | normal |
| C      | **None**   | `C` (colour's own L kept) | `C · 1` | "just the colour map" (default) |
| **None** | B        | `OKLab(L=B, 0, 0)` | `white · B` | **greyscale of the brightness field** (also the most CVD-robust encoding possible) |
| **None** | **None** | flat mid-grey `OKLab(0.6,0,0)` | flat mid-grey | well-defined, harmless, instantly visible |

Replace-L here is `L = B`: the range form `L_min + (L_max − L_min)·B` (`principia_dd_colouring.md` §3.5) at its defaults `L_min = 0`, `L_max = 1` (R-77).

This gives, for free, exactly the "use anything as colour, anything as brightness, or neither"
requirement: any field can occupy either slot (channel is independent of source — the *only*
constraint is the output signature, `vec3` vs `f32`), and either slot can be empty.

**The canonical bivariate family — and the honest replacement for "Stability × Hue".** The most
useful bivariate encodings are `colour = hue-carrier × brightness = magnitude-field`. The archetype
is **n̂ × ⟨field⟩**: shape-sphere position in the hue, a real dynamical magnitude in the lightness —
`FTLE × n̂`, `diffusion × n̂`, `ensemble-spread × n̂`, `time-to-event (t_end) × n̂`. Because those
magnitude fields default to greyscale-white=high (§1.2), the field drops into the brightness slot and
composes directly under Replace-L: chaotic / high-diffusion regions brighten the position map and
regular regions sink to black (and for `t_end`, with its inverted polarity, early-event regions
brighten while late / bounded regions darken). This **replaces the deleted "Stability × Hue"** — and it
replaces it with several encodings rather than one, because **"stability" is not a single metric**:
each brightness partner here is a distinct, named, *real* quantity (Lyapunov divergence, phase-space
diffusion, ensemble dispersion, time to the terminal event). `t_end` is deliberately **time to the
terminal event** — escape *or* collision *or* any class-ending event — not "escape time"; bounded
orbits reach no event within the window (sentinel).

Generally, the bivariate space is the **product ⟨hue carriers⟩ × ⟨brightness fields⟩**, not a fixed
list: categorical hue carriers compose the same way (outcome-state × FTLE), and any scalar can take
the brightness role. It is therefore a *row-family* of the render-mode catalogue, generated by the
backbone, not a set of hand-authored modes.

### 4.2 Post chain

Replaces the single post slot. An **ordered array of post nodes** (data), each `vec3(+ctx) → vec3`,
drawn from the combinator set (§1.3) and the post families: `site_overlay` (physics blobs, IC-dependent
per §2), `bandmask` (contours / quad-boundaries), `mix_const` / `mix_field` (blend-with-map),
fuzziness (ensemble-spread modulation), invert, tone. Codegen composes them in declaration order; the
node editor renders them as a linear run you can **insert into and reorder**. Bounded at **≤ 8** to
keep compile time and per-pixel cost sane. This is what unlocks "overlay *and* fuzziness *and* invert
simultaneously," which the single slot could not express.

### 4.3 Display stage (terminal, outside the pipeline)

A **fixed terminal stage** applied to the finished output, **as settings, not nodes** — the codegen
never sees them and they are not part of any occupant or preset:

```
style  →  render→display scale  →  gamut_clamp  →  cvd_sim(mode)  →  screen
```

**Order (R-67):** stain → style → display scale → gamut clamp → colour-vision simulation → screen. The simulation sees the final in-gamut colours. **Style** is optional and applies to
the figure only; scientific checks run with plain. *(Was: `gamut_clamp → cvd_sim(mode) → render→display scale`, with no style.)*

**CVD is here, not in the pipeline, and this is a category correction, not a convenience.** CVD
simulation asks "what does this *finished encoding* look like to a deuteranope/protanope/…?" — it
models the **viewer**, not the visualisation. That question is only well-posed on the output, so
anything applied after it is meaningless and anything that lets it be misplaced makes a diagnostic
that can lie. It is an **accessibility/display setting** alongside `render_scale`, and it therefore
applies uniformly to *everything* — main render, sphere preview, equirect unwrap, node thumbnails —
which is exactly what an accessibility audit wants: check the whole instrument at once. The CVD
simulation is real Viénot (protan, deutan) and Brettel (tritan) through LMS from linear sRGB, with matrices and golden
values from a published reference implementation, named with its version when the task lands (R-78; dd_colouring §3.8).

---

## 5. Codegen & eject

**Graph → readable WGSL.** The occupant tree compiles to deterministic, readable WGSL:
- **one named function per node**, with stable identity across recompiles;
- comments carrying the node name and its parameter values;
- parameters bound as **uniforms**, so slider tweaks rebind rather than recompile;
- a small shared WGSL library the codegen calls into: `vmf_weight`, `nearest`, `topk`, `oklab↔srgb`,
  `lut_sample`, the static site arrays, the field functions, the decode/encode (translated from the one Rust source,
  never hand-written — R-116). (This library
  is what replaces the 33 bespoke pixel functions.)

**View code.** Every pipeline stage exposes its generated WGSL snippet for reading. The graph *is* a
legible derivation of the shader — which doubles as pedagogy.

**Eject (fork (a), settled: per-node primary, whole-slot also available).** "Edit the shader" ejects
generated code into the existing **custom-WGSL occupant**:
- **per-node** — eject a single node's function to editable text; the rest of the slot stays
  graph-composed and live. Requires the codegen to expose **stable per-node function boundaries** and
  to splice an edited node back among generated peers, with **dependency tracking** so an upstream
  edit does not silently orphan a downstream node's inputs.
- **whole-slot** — eject the entire occupant function as one blob (the simpler mechanism; identical
  to today's custom-WGSL occupant).
- Eject is **one-way** (no decompilation). An ejected node/slot flags itself custom; its parameter
  widgets **grey out** (the code no longer derives from them); **live-compile with last-valid
  fallback** applies (already specified in `dd_colouring`); **"revert to generated"** is the way
  back. Same peek-vs-commit discipline as chart tilt vs promotion.

This is precisely the Unreal material-graph → HLSL relationship.

---

## 6. Debug views as presets

There is **one** colouring system. Debug views are presets over it (§1) reading `ctx` (§3), across
three data sources. `NORMAL` disappears — it was only ever "the user's pipeline."

**§B–§E payload field views → presets.** A field view *is* `colour = FieldRamp{field, recommended
ramp}, brightness = None, chain = []`. `f_edrift` = `energy_drift · diverging · symlog`; `f_wordhash`
= `word-hash · categorical`; `f_state` = `state · categorical palette`. As presets they inherit
compaction override, palette swap, the post chain, and per-stage shader visibility for free. The
"debug mode" dropdown becomes a **section of the preset library**.

**§F structural views → `ctx.quad` presets.** `s_depth` = `ctx.quad.depth → Viridis`; `s_impurity`
= `ctx.quad.impurity → Magma`; `s_state` = `ctx.quad.state → categorical`. Quad **boundary lines**
become a **post-chain `bandmask` step** on `distance-to-quad-edge` — strictly better than a mode,
because you can now overlay quad boundaries on the *normal* render.

**§A kernel modes → mostly presets, via fragment-side recompute (§3).** With `ctx.chart.z` present
and the decode/encode translated to WGSL from the one Rust source (R-116):
- **UV view** = `colour = ctx.screen.uv → RG` (fragment addressing) or `ctx.quad.uv → RG` (structural
  addressing).
- **DECODE view** = fragment-side decode of `ctx.chart.z`, coloured.
- **ROUNDTRIP** = fragment-side `encode(decode(z))` residual, ramped.
- **Agreement presets** (new, and better than the originals) = `|E(fragment-decode) − ctx.payload.E₀|`
  and friends: WGSL-decode vs Rust-decode. These simultaneously test **write-addressing** (a dispatch
  scramble shows as spatial disagreement) and are a **live check between two compilation paths of one
  source** — the kernel's rust-gpu build and its SPIR-V → WGSL translation — so they check the translation, not a
  transcription (R-116), running on every debugged frame.

**Discipline 1 — debug presets ship locked.** Their diagnostic value is that ROUNDTRIP-red means the
same thing every time. Editing a debug preset **forks it to custom** via the same one-way eject; it
never mutates the named preset.

**Discipline 2 — validity first (see §3).** Every debug field carries its validity lane and every
ramp an explicit invalid-pixel treatment (the hatched invalid pattern, R-132), or the views lie at the pixels they exist to expose. Debug fields are the
stated exception to masking (R-79): they show literal stored values (a failed-state `0.0` reads as `0.0`, cross-checked
against `state`), and NaN still goes to the invalid pattern.

**Net:** one colouring system · three data sources (sample payload · quad attributes · fragment
recompute) · presets all the way down. `debug_tooling_plan` §B–§G are re-expressed as a preset table
(§7); only Appendix A remains kernel-side.

---

## 7. Preset table & golden-image obligation

Every currently-specified map (the colour maps, the patterns, special modes,
the physics overlay, listed in full in §7.1) and every debug view is **recreated as a composition preset**. Representative
rows (schematic — full table lives with the preset library):

| preset | family / expression |
|--------|---------------------|
| VMF OKLAB | `SiteBlend{ axes6, vmf(κ=3), swatches=FullOKLAB-set, oklab }` |
| VMF Okabe–Ito | `SiteBlend{ axes6, vmf(κ), swatches=OI-set, oklab }` |
| Viridis (LUT-sphere) | `SiteBlend{ ring(16)+2poles, vmf(κ=4), colours=lut(viridis, i/N), rgb }` |
| Voronoi 6 | `SiteBlend{ axes6, nearest, swatches }` |
| Soft-Voronoi / basin-blend | `SiteBlend{ axes6 or fib(N), topk(2,ks), swatches }` |
| Octant / hemispheres / icosa / fibonacci | `SiteBlend{ corner8 / axes6 / ico12 / fib(N), nearest, swatches }` |
| Dot lattice | `SiteBlend{ fib(N), nearest, swatches }` masked by `bands(topk_margin…)`, `bgCol` base |
| Custom N-pole | `SiteBlend{ ring(N,tilt,rot), vmf(κ), swatches }` |
| Direction cosines | `FieldRamp` per channel `½(1+n)` |
| Checker / stripes / truchet | `bandmask(base, lattice-classifier(θ,φ), band, tileCol)` |
| Grid / contours | `bandmask(vmf-base, θφ-grid / iso-hue, band, lineCol)` |
| Gradient magnitude | `FieldRamp{ gradient_magnitude(vmf-map), lerp }` |
| Perlin / harmonics / Turing | `FieldRamp{ noise / Yℓm / wave-triple, lerp or diverging }` |
| Physics overlay | post step `site_overlay(base, SiteBlend{ physics(m), vmf(κ=11 BC / 9 EL), per-site colours }, s)` |
| `s_depth`, `f_ftle`, ROUNDTRIP, … | §6 presets over `ctx` |

**Verification obligation.** The **two reference HTML files are the golden oracle**
(`docs/gui/reference/principia_colour_explorer.html` and `docs/gui/reference/principia_colour_presets.html`; R-122),
and formulas follow the oracle. Every recreated
preset ships with a **golden-image test** against the corresponding reference output. This is a
**cross-implementation check in the project's established style** (like the shared-kernel-vs-
independent-integrator convergence reference): the composition engine and the reference artefact are
two implementations of the same maps, and agreement to tolerance certifies the port. A preset is not
"done" until its golden image matches.

**Pinned to §7.1 (R-16).** The golden-image suite is complete when every entry of §7.1 has a preset and a passing
golden test. §7.1 is the checklist; the two reference HTML files are the oracle for each entry (R-122).

### 7.1 The complete map list (R-16)

Ported from the retired shape-sphere colour-map PDF (R-3, R-16). Parameter ranges are in §8. The vMF engine (Eq. 5),
the LUT sphere, the CVD method (R-78) and the physics overlay's blob blend are in `principia_dd_colouring.md` §3.

**Colour maps — `principia_colour_explorer.html`** (R-139; Direction cosines is in `principia_colour_presets.html`).

| map | definition |
|---|---|
| VMF OKLAB | six vMF poles at $\{\pm\hat x, \pm\hat y, \pm\hat z\}$, full-OKLab hue table (dd_colouring §3.2) |
| VMF Okabe–Ito | the same engine with the Okabe–Ito CB-safe hue table |
| LUT spheres: Viridis, Cividis, Plasma, Magma, Inferno, Twilight, Cool-warm, Principia, Cubehelix | the seamless LUT sphere: 16 LUT samples as equatorial poles, the LUT endpoints at the north and south poles, blended as Eq. 5 in RGB. Twilight is cyclic. Cool-warm is diverging. The Principia palette is indigo → teal → gold. Cubehelix is generated analytically (hue spirals, lightness monotone increasing). **LUT data (R-122):** the published matplotlib tables for Viridis, Cividis, Plasma, Magma, Inferno and Twilight; Cubehelix's reference is the analytic form with dd_colouring §3.8's parameters (s = 0.5, λ = 1.5, h = 1), and matplotlib's cubehelix function, called with the same parameters, is a cross-check only (R-151); Moreland's table for Cool-warm; the Principia palette's stops are the explorer's (`principia_colour_explorer.html` :108, `LUT.principia`, eight stops). |
| Turbo | a 1-D colour LUT from Google's published Turbo table (Mikhailov 2019, Apache-2.0; R-139), shown among the additional colour map modes |
| Direction cosines | each Cartesian component of $\hat{\mathbf n}$ to its own RGB channel (lightness is not uniform) |

Global controls on every colour map: **Invert** ($v \mapsto 255 - v$), **Blend** (a linear mix of any two modes),
**Auto-rotate**.

**Patterns and special modes — `principia_colour_presets.html`** (R-139). Every pattern has the signature
$(\hat{\mathbf n}, \text{params}, \text{palette}) \mapsto [R, G, B]$.

| group | map | definition |
|---|---|---|
| Voronoi-type | Octant | partition by sign: index $= 4[n_x \ge 0] + 2[n_y \ge 0] + [n_z \ge 0]$ |
| | Voronoi 6 | nearest axis pole, $i^* = \arg\max_i \hat{\mathbf n}\cdot\hat{\mathbf p}_i$ |
| | Hemispheres | the dominant axis sets the colour, two shades per axis for the sign |
| | Icosahedral | 12 Voronoi cells about the icosahedron vertices $\{(0, \pm1, \pm\phi), (\pm1, \pm\phi, 0), (\pm\phi, 0, \pm1)\}/\lVert(0, 1, \phi)\rVert$, $\phi = (1+\sqrt5)/2$ |
| | Soft Voronoi | sigmoid blend between the two nearest poles, $t = \sigma(k_s(d_1 - d_2))$ |
| Lattices | Fibonacci lattice | $N$ golden-angle points, $n_{z,i} = 1 - 2i/(N-1)$, $r_i = \sqrt{1 - n_{z,i}^2}$, $\phi_i = \pi(\sqrt5 - 1)\,i$; golden-angle hue spacing so adjacent cells contrast |
| | Dot lattice | the Fibonacci points drawn as coloured dots of angular radius $\rho = \cos(1.4/\sqrt N)$ on a dark background |
| Stripes | Checkerboard | with $\varphi = \arccos n_z$ (polar), $\theta = \operatorname{atan2}(n_y, n_x) + \pi$ (azimuth; R-14's names): even $= (\lfloor f\varphi/\pi\rfloor + \lfloor f\theta/2\pi\rfloor) \bmod 2$; seam-free for integer $f$ |
| | Latitude stripes | $\cos(f \arccos n_z) > 0$ (no atan2) |
| | Longitude stripes | $\sin(f\,\operatorname{atan2}(n_y, n_x)) > 0$ (seamless for integer $f$) |
| | Truchet mosaic | each patch cell $(c_i, c_j)$ gets a deterministic diagonal split from $h = \operatorname{frac}(\sin(127.1c_i + 311.7c_j)\cdot 43758.5)$; colour by the side of the diagonal |
| Overlays | Grid overlay | pixels within $\epsilon = 0.04$ of a grid line take the line colour; the base map is unchanged elsewhere |
| | Iso-hue contours | lines of constant vMF hue $\psi = \operatorname{atan2}(b_{\mathrm{vmf}}, a_{\mathrm{vmf}})$, uniformly spaced in hue |
| | Gradient magnitude | $\lvert\nabla c\rvert \approx \tfrac12\sqrt{\lVert c(\hat{\mathbf n} + \varepsilon\hat x) - c(\hat{\mathbf n})\rVert^2 + \lVert c(\hat{\mathbf n} + \varepsilon\hat y) - c(\hat{\mathbf n})\rVert^2}$ |
| | Perlin noise | 4-octave 3-D value noise in Cartesian coordinates (no seam), $v = \sum_{k=0}^{3} 2^{-k}\omega(2^k s\,\hat{\mathbf n}) \big/ \sum_{k=0}^{3} 2^{-k}$ |
| | Checker + VMF | the checkerboard over the vMF map |
| Special | Real spherical harmonics | $v = Y_{\ell m}/\max\lvert Y_{\ell m}\rvert$, blended between the positive- and negative-lobe colours in proportion to $\lvert v\rvert$, grey on the nodal lines. Forms for $\ell \in \{1,2,3\}$ include $Y_{10} = \sqrt{3/4\pi}\,n_z$, $Y_{11} = \sqrt{3/4\pi}\,n_x$, $Y_{20} = \sqrt{5/16\pi}\,(2n_z^2 - n_x^2 - n_y^2)$, $Y_{22} = \sqrt{15/16\pi}\,(n_x^2 - n_y^2)$, $Y_{33} = \sqrt{35/32\pi}\,n_x(n_x^2 - 3n_y^2)$ |
| | Turing-like standing waves | $v = \tfrac13\left[\sin(f n_x) + \sin\!\left(f(\tfrac12 n_x + \tfrac{\sqrt3}{2} n_y)\right) + \sin\!\left(f(\tfrac12 n_x - \tfrac{\sqrt3}{2} n_y)\right)\right]$ |
| | Custom N-pole VMF | $N$ poles on a tilted great circle, $\hat{\mathbf p}_i = (\cos(\varphi_0 + 2\pi i/N)\cos\psi, \sin(\varphi_0 + 2\pi i/N)\cos\psi, \sin\psi)$, $\psi = \text{tilt}\cdot\pi/2$ |
| | Basin blend | soft interpolation between the two nearest Fibonacci cells, $c = t\,\text{pal}[i_1] + (1-t)\,\text{pal}[i_2]$, $t = \sigma(k_s(d_1 - d_2 - 0.04))$ |
| Physics | Physics overlay | vMF blobs at the binary collisions, Euler and Lagrange points (dd_colouring §3.4, blob blend; $\kappa = 11$ BC, 9 Euler/Lagrange), strength $s$; landmark positions per R-14, mass-weighted (R-50) |

---

## 8. Amendments to other docs

- **`principia_gui_state_contract.md` §4–§5** — "fixed 4-slot pipeline; occupants are data" becomes
  **"fixed typed *backbone* (`colour`/`brightness` → `combine`) with `Option` occupants · variable-
  length ordered *post chain* · fixed terminal *display stage* (style → scale → gamut → CVD, settings not
  nodes; R-67)."** The occupant model (source+reduction / channel / mapping, channel independent of source)
  is retained and points here for the compositional interior. CVD moves out of the pipeline.
- **`principia_dd_colouring.md`** — the vMF engine, LUT-sphere, physics overlay (Eq. 11), and
  combiner/compaction forms are retained as the *primitives* of §1 and cross-referenced; the
  mode-by-mode presentation is superseded by the preset table (§7). The `combine` L-ownership rules
  (Replace-L / Multiply) are unchanged and referenced by §4.1.
- **The shape-sphere colour-map PDF §5–§9** (implementation) — superseded by this document. The PDF
  is retired and archived; its Eq. 5 is in `principia_dd_colouring.md` §3.2, its CVD matrices are replaced by real Viénot/Brettel (§3.8, R-78), and its parameter ranges
  (L∈[0.35,0.90], C∈[0.05,0.22], κ∈[0.5,12], f∈[2,14], N∈[12,96], ks∈[1,20], s∈[0,1]) are adopted.
- **`principia_debug_tooling_plan.md` §B–§G** — re-expressed as the debug preset table (§6). §A is
  reduced to Appendix A.

---

## Appendix A — kernel-side bring-up mode

The **only** debug item that is *not* a render-key preset and *does* touch the kernel. A single
minimal mode — **"kernel writes a known pattern (e.g. `ctx`-derived UV or a fixed ramp) instead of
physics"** — for the bring-up situation where payload writes are so broken that the agreement /
fragment-recompute presets (§6) cannot even run (they depend on a trustworthy payload). It is
sim-key (it changes what the kernel computes), monomorphised on the Rust side under the determinism
law, and demoted from the debug catalogue to this appendix precisely because it is the sole exception
to "debug is presets over a shared data layer." Once payload writes are trusted, everything else in
§6 supersedes it.
