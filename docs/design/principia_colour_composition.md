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
`flag`. Every ramp/compaction carries an explicit **invalid-pixel treatment/value** — by default `#141418` in a
presentation view and the hatched invalid pattern, `debug_invalid(frag_xy)`, only in the debug views (§3, §6; R-132,
R-467; applied per R-369). *Was:* "by default the hatched invalid pattern (§3, §6; R-132)" (stale value, replaced
by R-467).

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

**Signed means a range that spans zero (R-400).** A field is signed, for these defaults and for the numeric debug
views, when its declared ledger range spans zero; it then takes a diverging ramp centred at zero, the prelude's
`ramp_coolwarm` for the debug views, and a field that can't be negative keeps its sequential ramp (viridis in the
debug views). No signed-or-not mark is added to the ledger. **The `diverging` tag agrees with the range (R-403):** a
field carries the ledger's `diverging` scale only when its declared range spans zero, and a field that can't be
positive, as `V_0` can't, keeps its sequential ramp too (viridis in the debug views), mapped monotonically, its most
negative value at the dark end. **"Not yet" sits on no ramp (R-399):** an unset `d_min` and
a running sample are drawn in the stippled "not yet" style (§1.4), never a flat grey, so a greyscale default or a
user's ramp never collides with it.
**An undeclared range is not signed (R-426):** where the ledger declares no range for a field, that is a gap in the
ledger (dd_generation_root §3.8), not a range that spans zero, so it never makes the field signed.

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
its nine classes carry structure worth encoding in the colours themselves. **The outcome view shows physical outcomes
only (R-444):**

| class | payload | colour | sRGB |
|-------|---------|--------|------|
| bounded / nothing yet | `bounded`, `running`, `sim_failed` and a reserved `state` code (R-446); a `decode_failed` sample draws blank (R-511; *was:* "`bounded`, `running` and every failure") | black | `#141418` |
| collision 1–2 | collision, `detail` = pair 2 (bodies 0, 1) | red | `#DE2D2D` |
| collision 1–3 | collision, `detail` = pair 1 (bodies 2, 0) | green | `#2EBC4E` |
| collision 2–3 | collision, `detail` = pair 0 (bodies 1, 2) | blue | `#3462E0` |
| triple collision | collision, `detail` = 3 | orange | `#F29620` |
| escape body 1 | escape, `detail` = 0 | yellow | `#F0DE32` |
| escape body 2 | escape, `detail` = 1 | magenta | `#E034C6` |
| escape body 3 | escape, `detail` = 2 | cyan | `#30C8DC` |
| triple ejection | escape, `detail` = 3 | purple | `#611597` |

**The class names are what a person reads, 1-based (R-449):** the bodies are 1, 2, 3 and a pair is named by its two
bodies, smaller first, never by its pair id. The payload column keeps the internal 0-based ids (R-22: body `k` is
`detail` `k`, pair `k` the side opposite body `k`), which the code, the WGSL and every machine-readable export use.
Every legend, label and swatch name takes the class's name from the one display function (payload §2, "What a person
reads"). R-444 to R-448 wrote these classes 0-based ("collision 0–1", "escape body 0"); the classes and their colours
are unchanged, only the names as shown.

The assignment is a **mnemonic, not arbitrary**: **collisions are additive primaries keyed by the
colliding pair** (1–2 → R, 1–3 → G, 2–3 → B); **escapes are subtractive primaries keyed by the
escaping body** (1 → Y, 2 → M, 3 → C; as shown, R-449; internally `detail` 0, 1 and 2, R-22). The two event *families* (collision vs escape) are therefore
separable at a glance while the pair/body identity stays legible; the three classes outside the primaries are
bounded / nothing yet (black), triple collision (orange) and triple ejection (purple). The nine classes read from the
`state` enum **plus the `detail` union** — collision → pair id or 3, escape → body id or 3 (so the R/G/B/Y/M/C
assignment lands on `detail`, not on a second field). **How the table reads the payload (R-444 to R-446):** from
`state` and `detail` alone. `t_end_step` plays no part: a collision at `t_end_step == 0` is an ordinary pair or triple
collision and draws in that colour (R-445); timing is carried by lightness when a stain wants it, the outcome ×
`t_end` bivariate preset (§4.1), white = early. **Black means "nothing has happened yet, up to the playhead"
(R-446):** `bounded`, `running`, `sim_failed` of each detail (NaN in state, Inf or overflow in state, a non-finite
derived quantity, payload §2: the human's "NaN, infinity") and a reserved `state` code (applied per R-369) draw
`#141418` in the outcome view; a `decode_failed` sample, whatever its `detail` or tag (η_E's `ENERGY_UNSCALABLE` and
`ENERGY_INFEASIBLE`, the mass block's `M01_TINY`, a non-finite decode), draws blank, as a pixel that is not a system,
with no key or share (R-511, ruling RQ-379; below). *Was (R-446):* "`bounded`, `running` and every failure draw
`#141418` in the outcome view: `sim_failed` of each detail (…), `decode_failed` whatever its `detail` or tag (η_E's
`ENERGY_UNSCALABLE` and `ENERGY_INFEASIBLE`, the mass block's `M01_TINY`, a non-finite decode), and a reserved `state`
code". The outcome view reads only `state`
and `detail`, integer fields, so it never receives a NaN itself; storage holds none (R-79). **`#141418` is the outcome
palette's explicit invalid treatment**, which §3 ("Overridable per node") and render_gui_spec §13 ("every colouring
has an explicit invalid-pixel treatment") allow, so R-79's rule that NaN or a sentinel reads as "no data", and
R-436's, are met in the outcome view by black, not by the hatch. A view starts black and fills
in as events fire. Failures are shown by the debug views, which keep the invalid hatch and their own palettes (the
`state` and `detail` views, `principia_debug_tooling_plan.md` §B), and by View › Debug view, which shows them over
the stain with a key and a count each (§6; R-467). **A forbidden chart region is not part of the chart (R-467,
amending R-446):** the outcome view draws it as the plain canvas background, the colour the window is cleared to, with
no hatch, legend key or share; only the debug views hatch it, `debug_invalid(frag_xy)`
(`principia_chart_decoder_contract.md` Part 3). It is decided from the chart alone, never from the payload's
`decode_failed` detail, which cannot tell the two hatches apart (R-446). **A pixel draws the background where
chart_decoder_contract Part 3 hatches it**, on every pixel, inside `[0,1]²` and past it: after the axes' extension
types act, a bounded axis past its edge, a state `validate` rejects (layer 2, or layer 3 on a finite decoded state), or
a `Φ` that fails to evaluate (non-finite, or outside its domain). Both of Part 3's hatches draw it, the domain's end and
the universal fallback (applied per R-369). *Was (R-446):* "Forbidden chart regions keep the chart contract's
forbidden-region treatment in the outcome view, the hatch `debug_invalid(frag_xy)`"; "A pixel keeps the hatch where
chart_decoder_contract Part 3 hatches it"; "Both hatches keep it". **`Φ` here is the
chart map, `Chart::map`** (`principia_chart_reference.md` §5.1), not the shared decoder after it: the decoder's own
outcomes, its `DEGENERATE` tags (`M01_TINY`, `ENERGY_UNSCALABLE`, `ENERGY_INFEASIBLE`, and any tag added later) and a
non-finite decode, never mark a pixel forbidden; such a pixel draws blank in a presentation view, as a pixel that is
not a system does (R-511, ruling RQ-379), the lane still unset, as R-446's list of failures says (applied per R-369).
*Was:* "such a pixel draws black unless the types or `validate` hatch it". **The axes' primary ranges hatch nothing:** R-446's words list them
among the chart's inputs ("types, primary ranges, validate, Φ"), quoting word for word chart_decoder_contract Part 3's
sentence on what the area statistic reads ("The statistic reads the chart (the types, the primary ranges, `validate`,
`Φ`)"), but under Part 3, whose treatment the ruling keeps,
they decide what the area statistics count (R-408), not what is hatched (applied per R-369, flagged), so a pixel
outside a primary range that the chart does not hatch (the shape sphere past its pole, a periodic redraw) is a system
and draws its outcome, and one whose state fails `validate` there draws the background (R-467). Every other
`decode_failed` sample draws blank (R-511; *was:* "draws black"). **Blank is not a colour (R-470).** The canvas background a not-a-system pixel draws
is blank, in every view, the outcome view included: no palette, ramp, compaction or legend holds it, it is never
counted in a legend's shares, and the only thing drawn over it outside the debug views is the optional Overlays › Chart
› forbidden region hatch (render_gui_spec §G2). NaN, `±∞` and `sim_failed` keep their presentation
treatment above (`#141418`), a `decode_failed` sample the chart does not classify draws blank, with the
forbidden-region hatch over it (R-511, ruling RQ-379), and all of them show as themselves only in the debug views
(§6). *Was:* "NaN, `±∞`, `sim_failed` and `decode_failed` keep their presentation treatment above (`#141418`) and show
as themselves only in the debug views (§6); whether a `decode_failed` sample the chart does not classify should draw
blank instead is RQ-379." *Was (R-446):* "one whose state fails `validate` there is hatched". How a sample's chart classification reaches the outcome view is REQ-COL-102's definition (R-72), written in §3
by the task that needs it. *Was (R-96, superseded in part by R-444 to R-446):*
"degenerate", `decode_failed`, drawn white `#ECECF0`; "collision @ t=0", a collision with `t_end_step == 0`, drawn
orange; `running` in the neutral grey and `sim_failed` in the invalid pattern. There is **no separate `escaper` field**: the
escaping body *is* `detail | state=escape`, so “which body escaped” is already carried by this map's
escape colours. A standalone escaper view is therefore this map **filtered to the escape classes** — a
**categorical filter** (`show class ∈ {…}, mute the rest`), which is a general operation any categorical
mode admits (“just collisions”, “just body 3's escape”, R-449), not a distinct render mode.

**The "not yet" style (R-399).** The neutral grey of "not yet" is not a flat swatch: "not yet", for running samples of a
field that holds no value for them and for an unset `d_min` alike, is the neutral grey (REQ-COL-053) with a fine dot
stipple from the pixel position, drawn by
`debug_not_yet(frag_xy)` (render contract, presentation layer), distinct from the invalid hatch's stripes. A ramp draws
one flat colour per value, so no ramp, greyscale included, can collide with it. The grey's value and the stipple's
pattern are proposed for the M1 gate (REQ-COL-053, REQ-COL-064). **Confirmed at the M1 gate:** the grey `#4E4E4E`
(R-433), and the stipple as a 2 × 2 px dot in each 4 × 4 physical-pixel cell over it, its dots changed from the proposed
`#808080` to `#909090`, so that no flat palette colour comes within the hatch's τ = 0.101 of every pixel of a 4 × 4
block: the nearest is 0.115 from its farthest pixel (R-434). **Not in the outcome view (R-446):** there `running`
samples draw black, bounded / nothing yet, not the style. The style and its calibrations stand for every other field
that uses them, an unset `d_min` among them.

**The two triple outcomes (R-444, R-445; REQ-COL-062 superseded, REQ-COL-104).** **Triple collision**, a collision with
`detail = 3`, and **triple ejection**, an escape with `detail = 3` (payload § `sample_descriptor`, "3 means all three";
R-30; REQ-EVT-006), are classes of the table. Triple collision takes the orange `#F29620`, which the removed
"collision @ t=0" class had. Triple ejection is purple `#611597` (OKLab L 0.401, C 0.190, hue 305°), between blue and
magenta in hue (collision 2–3 at 265°, escape body 2 at 335°) and clear of the hatch's violet `#9B00FF`. *Was:* lavender
`#D6A1FF` and navy `#000097`, proposed by TASK-M1-10 as the 8-bit colours farthest in OKLab from the classes, the
running grey and the hatch's colours, and confirmed at the M1 gate (R-433, RQ-234 decided per R-369). Navy shares
collision 2–3's hue (264° against 265°) and becomes indistinguishable from it once lightness carries another field
(R-445).
**Separation (R-71; REQ-COL-104), measured as REQ-COL-055 measures** (Euclidean distance in OKLab, dd_colouring §3.1's
transforms): purple is 0.192 from the "not yet" grey `#4E4E4E`, 0.193 from collision 2–3 and from the hatch's violet,
0.272 from escape body 2, 0.277 from bounded, 0.316 from the stipple's `#909090` dot and 0.580 from the hatch's cyan
`#48FFFF`, so it clears every entry and the hatch and stipple colours by at least 0.19, as R-445 says; the orange is
0.188 from the dot. **The palette's closest pair is 0.175** (R-546, RQ-331): triple collision `#F29620` against escape body 1 `#F0DE32`,
with the swatches as ruled. With the t = 0 class gone it stays 0.175 (the same two colours as the old t = 0 pair);
0.185 holds only with the orange left out. *Was:* "The human's figure for the palette's closest pair is 0.185,
collision 1–3 against escape body 3, "with the t = 0 class gone"; measured, the closest pair is 0.175, triple collision
`#F29620` against escape body 1 `#F0DE32`, the pair that was collision @ t=0's, since the orange stays as triple
collision (0.185 is the closest pair with the orange left out). Transcribed as ruled, the measured value beside it:
RQ-331."

**The OKLab-uniform variant (R-447; REQ-COL-103, REQ-COL-105).** A second preset of the outcome palette: each chromatic
class keeps its own OKLab hue at L = 0.72 and C = 0.123, so only hue varies and any brightness-slot field reads
cleanly; bounded stays `#141418`. **The sRGB palette above stays the default.**

| class | OKLab hue | sRGB |
|-------|-----------|------|
| collision 1–2 | 27° | `#E7857A` |
| collision 1–3 | 147° | `#6EB976` |
| collision 2–3 | 265° | `#7FA2F2` |
| triple collision | 65° | `#D99349` |
| escape body 1 | 103° | `#B3A841` |
| escape body 2 | 335° | `#D486C3` |
| escape body 3 | 208° | `#09B9CD` |
| triple ejection | 305° | `#B790E2` |

The hues are the sRGB classes' own, rounded to a degree. C = 0.123 is in gamut at every listed hue (the narrowest,
208°, admits 0.1238) (R-546, RQ-332); the largest chroma in gamut at every hue at L = 0.72 is 0.1224, near 199.8°,
recorded, not adopted. *Was:* "R-447 gives C = 0.123 as "the largest chroma in gamut at every hue at that L";
measured, C = 0.123 is in gamut at every listed hue (the narrowest, 208°, allows 0.1238), but the largest chroma in
gamut at every hue at L = 0.72 is 0.1224, near 199.8°." The swatches as ruled measure L 0.719 to
0.721, C 0.122 to 0.124 and each hue within 0.5° of its row; `#6EB976`, `#D99349` and `#09B9CD` differ by one or two
8-bit steps in one channel from the rounding of (0.72, 0.123, hue), `#6DB976`, `#D9934A` and `#07B9CD`. Those three
roundings are recorded, not adopted: the ruled hex values stand (R-546, RQ-332 (a)). *Was:* "Transcribed as ruled:
RQ-332." Against each class's own OKLab hue (the sRGB class's, before R-447 rounds it to a degree) every
swatch is within 0.5°; escape body 3's `#09B9CD` measures 208.54°, 0.54° from its row's 208° and 0.04° from its class's
208.5°, and stays: R-455 (RQ-366) reads the 0.5° against the class's own hue, a tolerance below hex quantisation at this
chroma (one green or blue step moves the hue about 0.7°). **Its closest pair, measured, is 0.063**, escape body 2 against triple ejection, 30° apart; four more pairs are
under 0.101 (collision 1–2 and triple collision, 0.079; triple collision and escape body 1, 0.081; collision 2–3 and
triple ejection, 0.084; collision 1–3 and escape body 1, 0.091). It is closer than the sRGB palette by construction, eight hues at
one L and C. Each swatch is at least 0.139 from the stipple's dot and 0.196 from the hatch's colours.

Like every colour assignment in the system, **this is a default, not a fixed mapping** — the
class→colour swatch-set is user-editable. It is the canonical default the render-mode catalogue's
outcome-state row inherits (that catalogue is out of scope here; this palette is the one piece of it
that is settled). It governs the outcome palette (`state ⊕ detail`) only: the raw `state` debug view keeps a six-colour
`dbg_cat` palette (`principia_debug_tooling_plan.md` §B, R-115).

**The Chazy final-motion fields (R-442; `principia_dd_chazy_final_motion.md`).** A separate categorical system from
the outcome class: `chazy_final`, `chazy_status` and `chazy_structure`, derived at read from the live state (drill-down
§5), with their own stains. R-442 gives them canonical default palettes as well, so `state` is no longer the only
categorical field with one (this amends "the one categorical field" above): the seven-class final-motion palette is
`chazy_final`'s default (drill-down §10.2), beside its four-family and Okabe–Ito variants (§10.3–§10.4); the status
palette is `chazy_status`'s (§10.6); the structure hues are `chazy_structure`'s (§10.5.2). Two swatches hold in every
Chazy stain: B is this table's bounded `#141418`, and collision, any pair or triple, is magenta `#DA85B6` (§10.1).
The categorical filter above applies to them unchanged (§10.7). The palettes, their measured separations and their
calibration are in the drill-down, not here.
R-443 adds `chazy_escaper`, the escaping body's index, and the one-body views that follow one body (the body lens,
its energy form, the escaper map and its mass-rank forms), with their palettes, in the drill-down's §10.11. The "no
separate `escaper` field" above concerns the event outcome, whose escaping body is `detail`; `chazy_escaper` belongs to
the separate Chazy system (R-443) and does not change it, and the escaper map is a Chazy stain, not this map filtered.

### 1.5 A point on a chart: Chart position's colour slot (R-542)

*Added by R-542 (`principia_state_map.md` SM11, SM14). Shape n̂'s nodes above are unchanged.*

Beside n̂, a colour occupant may read **a point on a chart's quotient**: Chart position's `point⟨q⟩`, q ∈ sphere ·
plane · cylinder · simplex (render GUI spec Part II §3), the place the sample's system has reached on a target chart,
with its status (state map SM1, SM5). Its colour slot is a **base** and an ordered list of **layers**, the composition
of §1.1 to §1.3:

- **Gradient**: Family B over the point. The 2D and 3D gradients are one-to-one on the quotient (state map SM6); the 1D
  gradients run one three-stop OKLab ramp; round θ is the hue circle.
  The Torus gradient (R-543), on the plane and the cylinder, is periodic in chart units and one-to-one within a period
  (state map SM6).
- **Site blend**: Family A. On the sphere it is the §2 static sets, one code path with Shape n̂'s Site blend; on a flat
  point, §2's lattice sites, with flat distance `−|x − p|²`, the same kernels and the same blend spaces.
- **Palette**: categorical by tile. Each tile of a tiling (square, hexagons, triangles, brick, stripes; on the sphere
  latitude × longitude cells, Fibonacci cells, meridian wedges, latitude bands) takes one swatch, by index or by golden
  angle.
- **Picture**: an image pulled back through the flow (§3's image input; state map SM8).
  Past its square it follows its wrap mode (Repeat, Mirror or Clamp) and a separate dim setting, Mirror and dimmed by
  default (R-543).
- **Layers**: `bandmask` with band patterns (grid lines, checkerboard, tile edges, stripes, rings; on the sphere its
  own), and `site_overlay` for site dots, in order, each with its spacing, colour and opacity.

The definitions, values and tests are state map SM6, SM8, SM11 and SM12. The tiling defaults are starting calibrations
(R-71): the prototype's values, confirmed at the M7 gate (RQ-487 (a), R-558).

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

**Flat lattice sites (R-542).** For a flat point (§1.5) the sites are a square or hexagonal lattice in the chart's tile
coordinates, `n` across, a whole number of them round θ on a cylinder. They are static and uniform-hoistable, compared
by flat distance `−|x − p|²` (state map SM11), and feed only Chart position's Site blend and its site-dot layer. The
sphere's sets above are unchanged.

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
| **quad**      | `index`, `depth`, `tl` (slice coords), `centre` (slice coords), `half_width` (slice coords, vec2), `uv` (within-quad vec2), `state` (enum), and summary stats: `impurity`, `spread`, `suspect_frac`, `priority`, `cache_age`, `sample_count` |
| **tile/sample** | `tile_index` (within quad), `sample_index`, `N` (samples per quad axis: a quad holds N × N, canonical spec §8), `E` (ensemble), `uv` (within-tile vec2) |
| **payload**   | every per-pixel field written by the kernel — `state`, `ftle`, `energy_drift`, `Lz_drift`, `diffusion`, `d_min`, `word`/hash, `t_end`, decoded-IC quantities, masses, … |
| **validity**  | the sentinel/predicate lane paired with **every** field: `ftle_valid`, the diffusion predicate `n ≥ 2` (R-245), `sd_is_failed`, out-of-chart / saturated flags, `forbidden` (bool, the sample's chart classification, REQ-COL-102), `ftle_valid` etc. |

**The chart lane's `ctx.chart.slice_uv` (R-394; REQ-COL-063).** The sample's position in the slice plane's own frame,
relative to the plane anchor (R-97): a vec2 in [0, 1]², post-flip Y-up, built from its quad's centre `c`
(`ctx.quad.centre`) and half-width `h` (`ctx.quad.half_width`, defined below: `2^−(ℓ+1)` per axis, in slice coords)
as the CPU computes them in f64, `ctx.chart.slice_uv = ctx.quad.centre + ctx.quad.half_width · (2 · ctx.quad.uv − 1)`,
the pattern of deep_zoom §1's `u = c + h·(2t − 1)`. In-plane pan and zoom never change it for a given sample; they
change only which quads are drawn. It is not a screen or view position: screen-relative positions are the screen
lane's fields, `ctx.screen.uv` and `ctx.screen.pixel`. Formed in f32 it is an absolute coordinate, which R-395 keeps
free of banding only up to the deep-zoom switchover (ℓ_switch, R-90); past it a fragment works in the quad's local
coordinates, `ctx.quad.centre` and the offset `δ = ctx.quad.half_width · (2 · ctx.quad.uv − 1)` from it.
Where the view reaches past the chart's depth-0 root, by navigation with the controls shown or with the figure
filling the window (F3 off or present mode, R-406, R-407, R-537), `ctx.chart.slice_uv` lies outside [0, 1]² there; the
formula is unchanged, and inside [0, 1]² nothing changes. *Was:* "Where the figure fills the window past the chart's
depth-0 root (F3 off or present mode, R-406, R-407),"

**The validity lane's `ctx.validity.forbidden`, the sample's chart classification (R-72; REQ-COL-102; R-446).** How a
sample's chart classification reaches the outcome view, so that it draws a forbidden chart region with the hatch from
the chart and not from the payload (§1.4). **Type and values:** a `bool`, one per sample, read by every pixel that shows
the sample. `true`, forbidden, is "hatched by chart_decoder_contract Part 3": on every pixel, inside `[0,1]²` and past
it, the sample's chart point is hatched as forbidden after the axes' extension types act, where a bounded axis is past
its edge, where `validate`'s layer 2 or layer 3 rejects the state (R-26; `principia_inverse_encode_contract.md` §
"Chart-aware validation"; layer 3 on a finite decoded state), or where `Φ` fails to evaluate (non-finite, or outside its
domain), `Φ` being `principia_chart_reference.md` §5.1's `Chart::map`, the chart map before the shared decoder. `false`
is every other sample. **What plays no part:** the axes' primary ranges, which decide only what R-408's area statistics
count, so a pixel outside a primary range that passes is a system and draws its outcome; the shared decoder's own
outcomes, its `DEGENERATE` tags (`M01_TINY`, `ENERGY_UNSCALABLE`, `ENERGY_INFEASIBLE`, and any added later) and a
non-finite decoder output, which never set it, so such a sample is `decode_failed` and draws blank (R-511; *was:*
"draws black unless the types or `validate` hatch it"), a non-finite decode staying `decode_failed`/0; and the payload, whose `decode_failed` detail cannot
tell the two hatches apart (R-446). **Who fills it, and from what:** the CPU, from the chart (R-26), per sample at its
chart point, never the kernel and never the payload. Inside `[0,1]²` it is filled from the chart's `validate` and
`Chart::map` (the `Chart` trait, chart_reference §5.1): forbidden where `validate` returns anything but `Valid` or
`Chart::map`'s output has a non-finite component. Past `[0,1]²` the extension types act first, and that fill is
TASK-M8-05's (REQ-CHART-054). It reaches the fragment as one bit per sample, in a buffer bound beside the per-quad
frames; a pass that binds no classification marks no sample forbidden. The outcome palette's occupant tests it first and
draws `debug_invalid(frag_xy)` where it is set, in every mode (§1.4, §4.1). *Applied per R-369 (TASK-M2-32): a member of
the validity lane, beside the out-of-chart flag, as a validity predicate of the sample, which the colour occupants may
read, and not a member of the chart lane, which they may not (REQ-COL-005, seam 6; render contract Part 4: the chart
lane holds the chart id, `z` and the position in the slice plane); a member of its own, not the out-of-chart flag, since
a pixel past `[0,1]²` on a periodic or affine axis is out of the square but not forbidden (qa review 5557530841, Q1).*

**The within-cell coordinates `ctx.quad.uv` and `ctx.tile.uv` (R-72; REQ-COL-056).** Each is a vec2 in [0, 1]², with
its origin at its cell's bottom-left corner, `u` increasing rightward and `v` upward on the slice: the post-flip, Y-up
convention of the debug tooling plan's UV passthrough (§F). The quad's cell is the quad; a tile's is the tile, one of
the quad's N × N, which its one sample rasterises to (canonical spec §8). So, for the tile in column `i` and row `j` of
its quad, both counted from the quad's bottom-left from 0, `ctx.quad.uv = (vec2(i, j) + ctx.tile.uv) / N`.
`ctx.tile.uv` is what render GUI spec §12.1 draws tile boundaries from, as it draws quad boundaries from `ctx.quad.uv`.

**The quad's half-width `ctx.quad.half_width` (R-72; TASK-M1-07).** A vec2 in slice coords: the quad's half-width on
each axis, deep_zoom §1's `h = 2^−(ℓ+1)` at depth `ℓ`, as the CPU computes it in f64 and the GPU reads it in f32,
beside `ctx.quad.centre`, deep_zoom §1's `c`. With them a fragment reconstructs a sample's UV coordinate from its
quad-local coordinate, `ctx.quad.centre + ctx.quad.half_width · (2 · ctx.quad.uv − 1)` (deep_zoom §1), and its
quad-local offset `δ = ctx.quad.half_width · (2 · ctx.quad.uv − 1)` (deep_zoom §2), as the coordinate view and its δ
mode do (debug_tooling_plan §F).

**Validity is not optional.** Every `ScalarField` returns `(value, valid)`. Every `Ramp`/`Compaction`
has an explicit **invalid treatment/value**. Without this, debug views silently lie at exactly the
pixels they exist to expose (a NaN FTLE would ramp to *some* colour and look like data). Invalid pixels
render in a **hatched pattern** that collides with no palette entry; its exact pattern is a calibration (REQ-COL-055,
R-71; R-132), confirmed at the M1 gate as proposed (R-428). Overridable per node. **The hatch is the debug views'
(R-467):** a presentation view, every stain that is not a debug view (§6), draws no hatch, so there a ramp's or
compaction's invalid value, a NaN and a non-finite input draw bounded / nothing yet's `#141418` unless the node
overrides its invalid colour, and a pixel the chart classifies as not a system draws the canvas background (§1.4). The
invalid treatment stays explicit; only its presentation default is black. The background there is blank, never a
palette colour (R-470).

**Fragment-side recompute.** Because `ctx.chart.z` is present and the decode/encode are available in
WGSL — generated from the one Rust source (rust-gpu → SPIR-V → WGSL translation), never hand-written (R-116) — the fragment stage can *recompute* cheap quantities (decode `z` → shape/energy; `encode(decode
(z))` residual). This is what dissolves most of the §A kernel modes (§6) and enables **agreement
presets** (fragment-decode vs kernel-payload) as live checks of that translation.

**Chart position's reads (R-542; `principia_state_map.md` SM2).** Chart position computes its point at read, from the
payload lane's live state at the playhead and its masses, through the encode `ROUNDTRIP` already runs; it adds no
field to any lane's buffer. A Picture base reads one image, a texture bound in a render-side bind group, never a sim
struct. The uploaded picture is a `RenderState` field, assigned its tier in R-484's table by the task that builds it
(applied per R-369). Chart position also reads the target chart and its inverse, which seam 6 and render contract
Part 4 keep from a colour occupant; it is a named exception to seam 6, as ROUNDTRIP and DECODE are, listed in render
contract Part 4 and in the lint, the target's inverse generated to WGSL from the one Rust source (R-116; R-558,
RQ-489 (a)). *Was:* "that is RQ-489, and the source waits on it."

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
the brightness role. The outcome × `t_end` preset, the outcome class in the colour slot and `t_end` in the brightness
slot, white = early, is how a stain shows an outcome's timing, which the outcome palette does not carry (R-445). It is therefore a *row-family* of the render-mode catalogue, generated by the
backbone, not a set of hand-authored modes.
The Chazy stains with an energy channel are of this family: `chazy_final` or `chazy_structure` as the hue carrier
and `chazy_q` in the brightness slot under Replace-L's range form (`principia_dd_chazy_final_motion.md` §10.5, R-442).
So is the body lens with energy: the lens's roles as the hue carrier and `chazy_qk[k]` in the brightness slot
(`principia_dd_chazy_final_motion.md` §10.11.2, R-443).

**The three modes of pitfalls §1.2 (c): event, greyscale and hue (R-72; REQ-COL-067, REQ-COL-068).** Pitfalls §1.2 (c)
renders one slice "in event mode, greyscale and hue mode" from one payload, to tell a structure of the physics from one
of the colour map. Each mode is a stain on the backbone above, its combiner Replace-L, and the three differ only in
their occupants: the payload, its read path and the field are the same in all three. **Event mode** is the outcome
palette (§1.4) in the colour slot, the brightness None; its field is the outcome class, read from `state` and the
`detail` union (§1.4, R-444; `t_end_step` plays no part, R-445). Greyscale and hue mode render a field on the prelude's ramps
(render GUI spec §10.1), the same field as event mode where they are compared with it.

- **Greyscale mode (REQ-COL-067)** renders a field through the truth table's None-colour row: the colour slot None and
  the field in the brightness slot, so that under Replace-L each sample is `OKLab(L = B, 0, 0)`, the "greyscale of
  the brightness field", the prelude's `ramp_grey(B)`. `B` is the field compacted to [0, 1]. A magnitude field takes
  its §1.2 compaction with its per-field polarity, its salient end bright (FTLE white = high; `t_end` white = low).
  A categorical field has classes and no magnitude: its class of rank `r` among its `K` classes takes
  `B = (r + ½)/K`, so each class has its own lightness, `1/K` from the next, and none is pure black or white. The
  outcome class is ranked bounded first (bounded / nothing yet), so that bounded is the darkest, as §1.2 and §1.4 keep
  it, then collision 1–2, 1–3 and 2–3, triple collision, escape body 1, 2 and 3, and triple ejection: `K = 9` (R-448;
  the names as shown, R-449).
  The Chazy fields' ranks are in `principia_dd_chazy_final_motion.md` §10.9 (R-442).
- **Hue mode (REQ-COL-068)** renders a hue-carrier field, the colour role of the canonical bivariate family above (a
  categorical field, or a field of direction or phase), in the colour slot through the prelude's hue wheel,
  `hue_wheel(t)`: OKLCH at L = 0.75 and C = 0.12, the hue `t` turns, in gamut at every hue (render GUI spec §10.1).
  The brightness slot is None, so the truth table's "C, None" row keeps the wheel's own lightness: the field is
  carried by hue alone, at one lightness, which a reading of lightness alone cannot see. A categorical field's class
  of rank `r`, ranked as in greyscale mode, sits at `t = r/K`: the wheel is periodic in `t`, so `t = 1` would be
  `t = 0`, and `r/K` for `r < K` gives each class its own hue, `1/K` of a turn from the next. A field of phase sits at
  its phase in turns. A magnitude field is not a hue carrier: the bivariate family puts it in the brightness slot,
  and greyscale mode is its reading.
- **What every mode keeps (R-399; §3).** A sample the field holds no value for keeps its own style in every mode,
  never a place on the greyscale or the wheel: a `running` sample of a field that holds no value for it, the "not
  yet" style, `debug_not_yet(frag_xy)`, and a `sim_failed` sample, a reserved `state` code or a NaN `#141418` in a
  presentation view (R-467) and the invalid pattern, `debug_invalid(frag_xy)`, in the debug views, as they draw them.
  **The outcome class is the exception (R-446):** it holds a value for every sample but a `decode_failed` one,
  `running`, `sim_failed` and a reserved code being its bounded / nothing yet class, so in every mode they take that
  class's place, rank 0, as the outcome palette draws them black (§1.4), and a `decode_failed` sample draws blank in
  every mode (R-511; *was:* "it holds a value for every sample, `running` and every failure being its bounded / nothing
  yet class"); a forbidden chart region draws the canvas background in every mode, decided from the chart's
  validity, not from the payload (§1.4; R-467). *Was (R-446, R-399):* "a `sim_failed` sample, a reserved `state` code or
  a NaN the invalid pattern, `debug_invalid(frag_xy)`, as the debug views draw them"; "only a forbidden chart region
  keeps the hatch, in every mode". A field that holds a value for a `running` sample, as FTLE does while the march runs, renders that value in
  every mode, so a progressive render shows the field as it fills. Hue mode's colour occupant draws the styles
  itself, as the palette does. Greyscale mode's brightness slot carries a lightness and can draw neither pattern, so
  the mode ends in one post (§4.2) that draws each style at those samples over the combiner's output and passes every
  other sample's colour on unchanged.
  The Chazy fields hold a value for a `running` sample, so they never draw the "not yet" style
  (`principia_dd_chazy_final_motion.md` §7, R-442).

**Chart position as a colour occupant (R-542; state map SM2, SM6, SM11).** Its 2D and 3D gradients carry position in
all three OKLab axes, lightness included, which is what makes them one-to-one on the quotient. A brightness node after
an L-spending base (a 1D gradient, the sphere's 3D gradient or the cylinder's 2D gradient), or under lightness layers,
raises a Problems warning (render GUI spec Part II §1). The plane's 2D gradient, the sphere's longitude gradient, Site
blend and the index Palette hold L fixed, so a brightness node can take it. SM2's "the brightness slot is None" means that a
base spending L on position takes a brightness node only with that Problems warning, Multiply keeping its L, and
SM15's Replace-L chroma scaling is an option of the Replace-L combiner, "keep in gamut", off by default and set on by
the Displacement and Mixing presets, so the truth table above stands for every stain at the default (R-558, RQ-496
(a)). *Was:* "SM2's "the brightness slot is None" meets the combiner choice of render contract Part 4 and SM11, and
SM15's Replace-L chroma scaling meets the truth table above (RQ-496); meanwhile the truth table stands for every
stain, Replace-L unscaled." Its statuses keep their own style
in every mode, as above: a left or past-the-end sample under "none" its grey, `sim_failed` `#141418`, `decode_failed`
and not a system blank (R-511, R-470); with a brightness node the stain ends in the one post that draws each style over
the combiner's output, as greyscale mode does (applied per R-369).

**The vMF shape colouring is not one-to-one (R-558, RQ-483 (a)).** It stays the hue carrier it is, and nothing in it
changes: at a fixed L no continuous colouring of the sphere is one-to-one, and the vMF blend gives the antipodal
shapes `±(1,1,1)/√3` one grey bin at κ = 1, 3 and 6 (`principia_state_map.md` SM6; `principia_dd_colouring.md`
§3.2). The one-to-one sphere map is Chart position's 3D gradient, a separate node; it is not offered as a shape
colouring.

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
[lines through]  →  print graph  →  render→display scale  →  gamut_clamp  →  [lines over]  →  cvd_sim(mode)  →  screen
```

*(Was, before R-531: `[lines through]  →  print graph  →  [lines over: RQ-395]  →  render→display scale  →  gamut_clamp  →  cvd_sim(mode)  →  screen`.)*

*(Was, before R-485 and R-487: `style  →  render→display scale  →  gamut_clamp  →  cvd_sim(mode)  →  screen`.)*

**The Print graph (R-485 to R-492; `principia_print_graph.md`)** stands where style was. It is a node graph of its own,
after the stain and separate from it: its registry, its codegen and its WGSL are its own, the stain's codegen never sees
it, and it is no stain occupant or preset; its settings are `RenderState.print` (R-491). Its default, **Plain**, is the
bitwise identity. It reads the stain's finished colours and, as read-only maps, payload fields normalised by the stain's
own window rule (`principia_print_graph.md` §4.4); nothing reads its output but the stages after it, so measurement,
the legend, the readout and every scientific check read the stain (R-489). Debug views bypass it (R-487 K1: View › Debug
view and every `debug/` occupant), and are never styled.

**The two line passes (R-487).** One line renderer is called twice with complementary filters on `LineStyle.through_medium`
(`principia_lines.md` §3): the line sets marked through are composited onto the stain's output **before** the Print
graph, so the medium draws them; the rest are drawn **after** it, crisp on any medium, in R-476 M2's place, the next
paragraph's: after the gamut clamp and before the colour-vision simulation (R-531, ruling RQ-395, the design doc §2's
diagram corrected to match). Smart ink for a line drawn over reads the figure to the clamp, the Print graph's output
included; for a line drawn through, the stain's output (`principia_lines.md` §6). A pixel that is not a system takes the
Print graph's sheet, never its marks (R-533). *Was:* "Where the pass drawn over sits is RQ-395: the design doc §2 puts
it before the display scale, R-476 M2 after the gamut clamp; until RQ-395 is ruled it keeps R-476 M2's place, the next
paragraph's."

**Order (R-67):** stain → style → display scale → gamut clamp → colour-vision simulation → screen. The simulation sees the final in-gamut colours. **The line system (R-476)** draws its lines after the gamut clamp and
before the colour-vision simulation, which sees them as it sees the rest of the figure (`principia_lines.md` §2;
applied per R-369). **Style** is optional and applies to
the figure only; scientific checks run with plain. *(R-485: style is the Print graph, above; its default is Plain,
and goldens and scientific checks run under Plain, R-489.)* *(Was: `gamut_clamp → cvd_sim(mode) → render→display scale`, with no style.)*

**CVD is here, not in the pipeline, and this is a category correction, not a convenience.** CVD
simulation asks "what does this *finished encoding* look like to a deuteranope/protanope/…?" — it
models the **viewer**, not the visualisation. That question is only well-posed on the output, so
anything applied after it is meaningless and anything that lets it be misplaced makes a diagnostic
that can lie. It is an **accessibility/display setting** alongside `render_scale`, and it therefore
applies uniformly to *everything* — main render, sphere preview, equirect unwrap, node thumbnails —
which is exactly what an accessibility audit wants: check the whole instrument at once. The CVD
simulation is real Viénot (protan, deutan) and Brettel (tritan) through LMS from linear sRGB, with matrices and golden
values from a published reference implementation, named with its version when the task lands (R-78; dd_colouring §3.8).
The reference is DaltonLens-Python at commit `3cba5e6`, named by R-383 in dd_colouring §3.8.

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
  transcription (R-116), running on every debugged frame. The fragment shaders may compile with fast-math and the
  compute shaders don't, by default (R-297), so an agreement preset compares within a stated tolerance, a calibration
  requirement (R-71), not bit-exactly. ROUNDTRIP's residual is computed wholly in the fragment, and its stated
  tolerance is ε_phys (`principia_inverse_encode_contract.md` Part 4; `principia_debug_tooling_plan.md` §A).
  **A non-finite operand is invalid (R-436).** `E₀ = K₀ + V₀` reads `−∞` at the f64 collision pole, as computed
  and stored (`principia_dd_decoder.md` §3.6), where the difference would be `(−∞) − (−∞)`, NaN on the CPU and
  unpinned on the GPU. An agreement preset tests each stored operand (`K_0`, `V_0`, and any other it reads) on its
  bits before any arithmetic, `K₀ + V₀` included, and draws a pixel with a non-finite operand in the invalid colour,
  on the CPU and the GPU alike; it never does arithmetic on `±∞`, since WGSL lets an implementation assume there are
  no infinities. The fragment decode's own `V`, whose pole value it selects as the decode stage does
  (`principia_dd_decoder.md` §3.6, "Each producer selects its pole value"), is tested on its bits too, before the
  preset forms `E(fragment-decode) = K + V`.

**Discipline 1 — debug presets ship locked.** Their diagnostic value is that ROUNDTRIP-red means the
same thing every time. Editing a debug preset **forks it to custom** via the same one-way eject; it
never mutates the named preset.

**Discipline 2 — validity first (see §3).** Every debug field carries its validity lane and every
ramp an explicit invalid-pixel treatment (the hatched invalid pattern, R-132), or the views lie at the pixels they exist to expose. Debug fields are the
stated exception to masking (R-79): they show literal stored values (a failed-state `0.0` reads as `0.0`, cross-checked
against `state`), and NaN still goes to the invalid pattern. So does a stored `±∞` that is no declared sentinel, on
the CPU and the GPU alike, tested on its bits, never by arithmetic on it (R-436, extending R-79); a declared sentinel
or unset value keeps its own style (`d_min`'s unset `+∞`, R-280, R-399).

**The debug view, View › Debug view (R-467).** One switchable view over whatever stain is active, a render-side
toggle (`RenderState`'s `debug_view`, gui_state_contract §2), not a preset: every system pixel is drawn as the stain
draws it; a pixel the chart classifies as not a system (§1.4) draws the hatch, `debug_invalid(frag_xy)`; a non-finite
sample (a `sim_failed` with a non-finite detail, a `decode_failed` of detail 0, a non-finite value the stain reads)
draws the NaN / Inf style, a prelude pattern of its own (REQ-COL-110, a calibration, R-71), proposed by the task
that needs it and not magenta: R-16's flat magenta `#FF00FF` sits at τ (0.101, R-429) from escape body 2's `#E034C6`,
so each of the style's colours lies farther than τ from `#FF00FF` (R-505); any other `sim_failed` or
`decode_failed` sample, a reserved `state` code included, draws `debug_invalid`. The chart is read first, then
non-finiteness, then the failure. Its legend adds one key after the stain's, "Debug": "forbidden (chart)", "decode or
sim failure" and "NaN / Inf", each in its style, with the number of the figure's pixels drawn in it and their share,
each drawn always (render_gui_spec §G6). It is the one view that shows all three; the presentation views show none of
them (§3). *R-470:* it is one of the debug views, built as a baked pipeline variant selected by `debug_view` (R-41's
baked variants): the presentation variant has no debug arm, and `debug_view` is never a flag bit read per pixel.

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
| Grid / contours | `bandmask(vmf-base, θφ-grid / iso-hue, band, lineCol)` (a colouring that bands pixels; the stroked grid and contours over the figure are the line system's, R-476, `principia_lines.md`) |
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

**The Print graph's goldens (R-489, R-492).** Every stain golden and every scientific check renders with the Print graph
set to **Plain** explicitly, never the user's. The Print graph's own nodes and presets have a golden obligation of their
own, against the lab rather than the two HTML files: each node at its defaults and each preset on the bundled source
image, matched to `docs/gui/reference/print_graph/images/nodes/` and `images/presets/` within the ΔE (OKLab, median
and p99) tolerance the first port task sets, each deviation the design requires listed in its registry entry, never
tuned (`principia_print_graph.md` §10, §11 test 9).

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

## 9. The stain editor's composition resolutions (R-72; REQ-COL-054)

Render GUI spec §16 lists eight "standing composition-spec gaps" whose resolutions the stain editor assumes. Each is
written here from the passages that already carry it. The parts the passages left open were ruled by R-546 and are written in: RQ-280 (the addressing convention's third
channels and its indexes, §9.7), RQ-281 (the debug groups' membership, §9.6) and RQ-282 (gradient magnitude, §9.4; a
ramp of custom stops is defined apart). *Was:* "The open parts are raised in `REVIEW_QUEUE.md`, not written in: RQ-280
(the addressing convention's unnamed channel and its index), RQ-281 (the debug groups' membership past the four the
passages settle) and RQ-282 (a ramp of custom stops)."

**9.1 Colour source as an axis.** Where a value comes from is one axis, independent of the slot it fills and of the
mapping that colours it. In the graph a source is a `source` node with one parameter, the ctx field it reads; picking
the field sets its out-port's subtype, `scalar`, `vector` or `categorical` (render GUI spec §3.1, §5). The fields it
offers are §1.2's `ScalarField` sources and the shape position `n̂`, all read through `ctx` (§3), each with its
validity lane. A `vector` source (`n̂`) feeds Family A (§1.1); a `scalar` or `categorical` source feeds Family B (§1.2)
or a categorical palette (§1.4). Any source may feed either slot, since the channel is independent of the source and the
only constraint is the output signature, `vec3` or `f32` (§4.1); one source may feed both, as one visible fan-out wire
(render GUI spec §5). The bivariate space is the product of the two slots' sources, ⟨hue carriers⟩ × ⟨brightness
fields⟩ (§4.1). The concrete list a node palette shows stays render GUI spec §16's "Node palette contents" item.

**9.2 Overlays are a post chain over a configured base.** An overlay is a post node, `vec3(+ctx) → vec3` (§4.2), drawn
from the combinators of §1.3. In the post chain its `base` is the colour arriving at it: the output of `combine` over
the user's configured occupants, or of the post before it. The order is the render contract's (Part 4, "Composite
order"): baked base → `combine` → the post chain, the physics overlay and the structural and debug overlays → `OUT`, and
the display stage after it, outside the graph (§4.3). So grid, contours, the checker (the overlay of §7.1's "Checker +
VMF"), the physics overlay and the quad boundaries lie over whatever stain is configured, not over a fixed base map, and
are inserted and reordered as data, at most 8 (§4.2). A combinator may still sit inside an occupant's expression tree (§1), where its
`base` is the sub-tree wired to it.

**9.3 Physics is an overlay op.** The physics overlay is `site_overlay(base, SiteBlend{ sites = physics(m), kernel =
vmf(κ), colours = per-site })` with per-blob strength `s` (§1.3): Family A used as a combinator, not a distinct node.
Its sites are §2's physics generators, `BC(m)`, `Euler(m)` and `Lagrange(m)`, evaluated per pixel from the decoded
IC's masses and hoisted to uniforms only when no slice axis or tilt touches a mass dimension (§2). Its preset is the
post step with κ = 11 for the binary collisions and 9 for the Euler and Lagrange points (§7, §7.1), blended as a
sequential clamped mix (§1.3, R-122). It is evaluated per fragment, never baked (render contract Part 4, R-121).

**9.4 Gradient magnitude is a ramp over a derived field.** The "Gradient magnitude" map is
`FieldRamp{ gradient_magnitude(map), lerp }` (§7): `gradient_magnitude` is §1.2's derived operator, the finite
difference of another colour node from three evaluations at ±ε on the sphere, and the ramp is an ordinary §1.2 ramp,
so it is a field through a ramp, not a mode of its own. The gradient magnitude of the rendered *map* is a different
thing: a post occupant, a screen-space finite difference over the colour buffer, distinct from the |∇c| evaluated on
the sphere (render contract Part 4). §16's item is this reading alone (R-546, RQ-282 (a)). Render GUI spec §9's
custom-stop ramp, the gradient editor's "built-in LUT or custom stops, cyclic", is defined separately, here, by
TASK-M7-11 when its graph model needs it (R-72, REQ-COL-140): its stops, their positions, its interpolation space and
how `cyclic` applies. *Was:* "Whether the item also means that the GUI's gradient editor, "built-in LUT or custom
stops, cyclic" (render GUI spec §9), unifies §1.2's ramps, and so what a ramp of custom stops is, is RQ-282."

**9.5 Spread per footprint and per quad.** Ensemble spread is a footprint quantity: the resolve stage reduces a
footprint's E + 1 samples to one spread, one value per nominal sample shared by its copies for display, and no
`SimState` field holds it (sampling note, "Stability metrics"). It is therefore not anti-aliased, and that is what it
means: there is nothing within the footprint to resolve. That per-footprint value is the ensemble-spread field of §1.2
(white = high) and of the bivariate `ensemble-spread × n̂` (§4.1). `ctx.quad.spread` (§3) is a different field: the
footprints' spreads aggregated into the quad reduction for refinement (sampling note), one value
per quad, as `ctx.quad.impurity` is.

**9.6 The debug taxonomy.** The library's debug section is the six groups of render GUI spec §11: addressing,
quad-structure, decoded-IC, validity, diagnostic and profiling. Every entry is a locked debug preset over the one
colouring system, reading `ctx` from one of §6's three data sources (§6, Discipline 1). The passages settle four, and
R-546 (RQ-281) the other two and the cross-checks:
- **addressing**: the presets that colour a position, `ctx.screen.uv → RG` (fragment addressing) and `ctx.quad.uv →
  RG` (structural addressing) (§6), in the channel convention of §9.7;
- **quad-structure**: §6's `ctx.quad` presets, `s_depth`, `s_impurity` and `s_state`;
- **decoded-IC**: the decoded-IC payload quantities (§1.2), the `ICDescriptor` views of debug tooling plan §E (its
  masses, scalar and `E₀` rows), which "certify the **decoder**, independent of any integration" (debug tooling plan
  §E), and the DECODE view, the fragment-side decode of `ctx.chart.z` (§6); and ROUNDTRIP and the agreement
  presets, because they certify the decoder (R-546, RQ-281);
- **validity**: the validity lane of §3, every field's sentinel or predicate (§6, Discipline 2);
- **diagnostic**: the §B–§D payload field views (`f_edrift`, `f_wordhash`, `f_state`) and §E's live-shape and
  accumulator views (R-546, RQ-281);
- **profiling**: views of per-sample cost (`total_substeps`, the substep-log proxy) and of quad cache age (R-546,
  RQ-281). These colour measurements that are always taken, so telemetry §5.5 stands: profiling's measurement is
  first-class, and only its views are filed here.

*Was:* "Which presets the diagnostic and profiling groups hold, and the groups of ROUNDTRIP, the agreement presets, the
§B–§D payload field views and §E's live-shape and accumulator views, is RQ-281."

**9.7 The addressing channel convention.** An addressing preset writes a position into colour channels as render GUI
spec §16 names them, each pair read in order as `UV=RG` is, `u` → red and `v` → green (§6; debug tooling plan §F):
- **UV = RG**: `ctx.screen.uv` or `ctx.quad.uv`, `u` → R, `v` → G, B = 0. The UV passthrough's quad-local δ mode
  draws "`u`'s in red and `v`'s in green, blue 0", and "in exact arithmetic it draws `t`, as `ctx.quad.uv → RG` does,
  so the two agree unless the δ path is wrong" (debug tooling plan §F), which holds only with the RG view's blue 0
  too; §6's UV view is one preset over either source, so `ctx.screen.uv → RG` takes the same;
- **TL = RB**: `ctx.quad.tl` (slice coords), `x` → R, `y` → B, G = 0 (R-546, RQ-280);
- **centre = BG**: `ctx.quad.centre`, `x` → B, `y` → G, R = 0 (R-546, RQ-280);
- **index = grey**: an index written as one value on all three channels, one grey preset per index: `ctx.quad.index`,
  `ctx.tile.tile_index` and `ctx.tile.sample_index` (R-546, RQ-280). Each maps its index to grey in [0, 1] by a
  stated rule, written here by TASK-M7-17 (R-72, REQ-COL-139).

*Was:* "The channel TL = RB and centre = BG leave unnamed (G and R), which index "index" is (`ctx.quad.index`,
`ctx.tile.tile_index` or `ctx.tile.sample_index`), and how it maps to [0, 1] are RQ-280."

**9.8 Boundaries are an overlay.** Quad and tile boundaries are Tier-1 overlays (render GUI spec §12.1): a function of
the position within the cell, drawn by an ordinary post node from `ctx.quad.uv` or `ctx.tile.uv`, with no data
source, no buffer, no separate pass and no CPU work. It is §1.3's `bandmask` on the distance to the cell's edge, as
§6 says, with that distance taken in pixels, so that a line is the same width at any quad size, depth and zoom and is
antialiased, as `edge_line` draws it (render GUI spec §12.1, as built by TASK-M1-13). Being a post node it serialises
with the graph, has editable width, opacity, colour and level, chains in post order, and lies over any stain; it is not
a display-stage setting (render GUI spec §15, item 11).

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

**The pattern and its slots (R-72; REQ-TOOL-123; TASK-M1-11).** The known pattern is a fixed ramp over the sample's
index `i` in the `SimState` buffer (on the synthetic harness's flat layout, the flat grid's index), with
`j = i mod 2¹⁶`. The mode writes every member of the sample's `SimStateFTLE` and nothing else: the word buffer,
`ICDescriptor` and `RenderQuad` are not written, and no buffer is added.

| Slots | Pattern |
|---|---|
| each real slot, numbered `k` in the ledger's member order: `r` 0–5 (`r[b][c]` is `2b + c`), `p` 6–11, `r_sh` 12–17, `p_sh` 18–23, `S` 24, `theta` 25, `mean_y` 26, `C_ty` 27, `E_0` 28, `Lz_0` 29, `closure_min` 30 | `32·i + k` |
| `packed_a`, through the generated setters | `state = i mod 6`, `detail = ⌊i/2⌋ mod 4`, `saturated = ⌊i/8⌋ mod 2 = 1`, `dmin_pair = ⌊i/16⌋ mod 4`, `last_symbol = ⌊i/64⌋ mod 4`, `d_min` unset (R-271); reserved bits 10–15 zero |
| `packed_b` | `0`: `dE_max` and `dLz_max` both +0 |
| `times` | `t_end_step = j`, `t_dmin_step = 65535 − j` |
| `total_substeps` | `i` |
| `closure_step`, `_reserved` (`θ̃`'s frozen pole reference, R-397) | `j`, `0` (the code of the reference 0 rad: a valid code, not a fresh sample's 0xFFFF) |

`_reserved` keeps `0` under R-397 (applied per R-369): the pattern is not physics (its `theta` slot holds `32·i + 25`,
not a phase), 0 is a valid reference code (payload §2), and the bring-up mode never runs the pole hold, so nothing
reads it as a reference. The synthetic harness uploads whatever `SimState` its test builds, `_reserved` included.

Every real slot's value is an integer below 2²⁴ for `i < 2¹⁹`, so it is exact in f32: the f32 SPIR-V variant and the
f64 native one write the same values, and the dispatch refuses more than 2¹⁹ samples. A slot written one place off
holds another slot's value, so the readback fails on it. The `f16` fields hold a fresh sample's values (`d_min` unset,
the drift maxima 0), so the GPU writes no `f16` conversion. The readback test decodes the buffer through the normal
unpack path (`sim_state_from_ftle`, lowering Part 3a) and compares each field with this table, and compares the raw
packed words too, `packed_a` by `roundtrip_ctl`, so a reserved bit the unpack masks off also fails (pitfalls §9).
