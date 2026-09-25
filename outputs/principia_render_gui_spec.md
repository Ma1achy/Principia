# Principia — Render / Colour GUI (dev)

*Status: canonical. Single source of truth for the **developer (egui / F3) colour-and-render GUI** —
the surface that authors, edits, and inspects the fragment-side composition. Scope is the
**colour-maps / render-modes / debug** part of the dev GUI only; the production (TS) GUI, the
IC-inspector tool, and non-render dev panels are out of scope here.*

*Conforms to `principia_colour_composition.md` (the composition algebra — sources, occupants,
combiner, post chain, codegen/eject, ctx contract, defaults). This document specifies the **GUI**
over that algebra; where the two overlap, the composition spec is authoritative on semantics and
this spec is authoritative on interaction.*

*Related prototypes map directly onto slots defined here: `principia_colour_explorer.html` is the
**colour-node inspector** (its site-blend/field-ramp faces become the type-driven editors of §9);
`principia_render_modes.html` is the **library panel** (§11); the catalogue's per-mode default
composition **is** the serialized preset graph.*

---

## 0. The core claim

**There is one object — a composition — and every layer of the GUI is a view onto it at a different
zoom.** The library *picks* a composition, the canvas *wires* it, the node inspector *edits one
stage* of it, and Advanced *drops to that stage's code*. Customisation and abstraction are the same
structure seen at different depths: a user can stay at "pick a preset" or descend continuously to
hand-WGSL, and it is one path, not four tools.

Consequences: there is no separate "render-mode GUI", "colour-map GUI", and "debug GUI" — they are
one GUI over one piece of state (the composition graph + display settings), differing only in
*whether you are picking a preset, editing the graph, or viewing a locked one*.

---

## 1. Layout — four surfaces

```
┌───────────────── global display bar ─────────────────┐
├──────────┬─────────────────────────────┬─────────────┤
│ library  │        graph canvas         │    node     │
│ drawer   │   (Graph | Code toggle)     │  inspector  │
│(catalogue│                             │             │
│ presets) │  source→colour/brightness   │ morphing    │
│          │    →combiner→post→OUT        │ editor +    │
│          │                             │ Advanced    │
└──────────┴─────────────────────────────┴─────────────┘
```

- **Library drawer** (collapsible, left) — the catalogue, grouped + filterable + glyphed; loads a
  whole graph on selection (§11).
- **Graph canvas** (centre) — the pipeline as a node graph, with a corner `Graph | Code` view toggle
  (§10).
- **Node inspector** (right) — the editor for the selected node; morphs to the node/source type;
  Advanced-WGSL at the bottom (§9).
- **Global display bar** (top) — gamut / CVD / render-scale / boundary-overlay; applied to `OUT`,
  *outside* the pipeline (§12).

A live **preview** (shape-sphere or illustrative slice, toggle) renders the current `OUT`.

---

## 2. The pipeline

The composition is a fixed-shape flow with pluggable occupants:

```
source(s) → colour?     ┐
                        ├→ combiner → post(s) → OUT
source(s) → brightness? ┘
```

- `colour?` and `brightness?` are **optional** (None = identity, §13).
- `combiner` is **required** (Replace-L / Multiply).
- `post(s)` is a **variable-length ordered chain** (`vec3 → vec3`), possibly empty.
- The **display stage is not part of this pipeline** (§12).

The graph *is* the composition; codegen walks it to produce the fragment shader (`principia_colour_
composition.md` §5).

---

## 3. The node graph — model

- **Node** = `{ id, kind, inputs[], outputs[], params }`. Kinds: `source`, `colour`, `brightness`,
  `combiner`, `post`, `OUT`.
- **Port** = a typed, directional (in/out) connection point. Wire-carried types:
  - `field` — a ctx field, **tagged with a subtype**: `scalar` / `vector` / `categorical`.
  - `vec3` — a colour.
  - `f32` — a lightness.
- **Wire** = `out-port → in-port`, type-checked (§6).

### 3.1 Node kinds

| kind         | inputs                          | output | notes |
|--------------|---------------------------------|--------|-------|
| `source`     | —                               | `field` | one param: **which ctx field** (dropdown); out-port subtype follows the field |
| `colour`     | one or more `field`             | `vec3` | inspector editor chosen by the **primary field input's subtype** (§9) |
| `brightness` | one `field`                     | `f32`  | inspector = gradient / compaction editor |
| `combiner`   | `colour: vec3?` + `brightness: f32?` | `vec3` | **fixed singleton**; Replace-L / Multiply |
| `post`       | `vec3` (+ optional `field` ins) | `vec3` | one op (overlay / band-mask / tone); chained |
| `OUT`        | `vec3`                          | —      | **fixed singleton**; cannot be deleted |

---

## 4. The backbone — fixed vs free

The pipeline has an **invariant skeleton** and a **free interior**.

- **Fixed singletons:** `OUT` and `combiner`. They cannot be added, deleted, or duplicated. This
  makes the two-slot backbone literal and removes an entire class of invalid states ("no combiner",
  "two combiners", "OUT unreachable-because-deleted").
- **Free interior:** any number of `source`, `colour`, `brightness`, and `post` nodes, wired subject
  only to **type compatibility** (§6) and **acyclicity**.
- **The tail** is `… → combiner → (post)* → OUT` — zero or more post nodes between the combiner and
  OUT.

So the invariant is `… → combiner → (post)* → OUT`; the free part is everything feeding the
combiner's two inputs.

---

## 5. Sources & multi-source

- A `source` node is `{ field dropdown } → typed out-port`; picking the field sets the out-port's
  subtype (`scalar`/`vector`/`categorical`).
- **Fan-out is free:** one `source` out-port may wire to many consumers. This is how *the same field
  drives both hue and lightness* — a single `shape n̂` source wired into both a `colour` node and a
  `brightness` node is a **visible wire**, not a coincidence. (Bivariate `n̂ × ⟨field⟩` is authored
  exactly this way — see §11 presets.)
- **Multi-source mappings expose multiple input ports** on the consuming node (e.g. a derived source
  taking two `field` ins for a `top1 − top2` margin, or a `gradient-magnitude` node). This is
  distinct from multi-wire-per-port (which is disallowed, §6).

---

## 6. Port typing & wire rules

- A wire is accepted iff the out-port type matches the in-port type (`field`→`field`, `vec3`→`vec3`,
  `f32`→`f32`). A type mismatch is **rejected** — the wire will not attach and snaps back.
- **Subtype within `field` is *not* a hard gate:** dropping a `categorical` field into a `colour`
  node currently in gradient mode is **allowed**, and the node's inspector **morphs** to the new
  subtype's editor (→ palette), carrying over what params it can and flagging what it cannot.
- **In-port arity: exactly one wire.** Dropping a new wire onto an occupied in-port **replaces** the
  old (last-write-wins).
- **Out-port arity: unbounded** (fan-out, §5).

---

## 7. Canvas interactions

- **Select:** click a node → highlight + open its inspector. Click empty canvas → deselect
  (inspector shows graph/global). **Marquee-drag** → multi-select.
- **Move:** drag a node body → reposition; wires follow. Multi-selected nodes move together.
  (Snap-to-grid optional.)
- **Wire (create):** press on an out-port → drag a wire → release on a compatible in-port. While
  dragging, **compatible in-ports highlight and incompatible ones dim**. Releasing on **empty
  canvas** opens a **node-create menu filtered to nodes that accept the dragged type** (drag-off-pin
  authoring — the primary accelerator; it also teaches the type system by only offering valid nodes).
- **Wire (delete / redirect):** grab near a wire's head and drag off to detach, or select the wire +
  `Delete`. (In-port replacement, §6, is the redirect path.)
- **Add node:** right-click canvas → categorized node palette (sources grouped by ctx-group;
  colour / brightness / post). *The library is not a node source* — it loads whole graphs; single
  nodes come from the canvas palette or drag-off-pin.
- **Delete node:** select + `Delete`. `OUT` and `combiner` **refuse**. Deleting a mid-graph node
  leaves dangling in-ports, which render as **None** and evaluate via identity (§13) — the pipeline
  stays renderable.
- **Pan / zoom:** space-drag or middle-drag to pan; scroll to zoom.

---

## 8. Node visuals — glyph-forward

Each node box carries:
- a **title bar** (kind + source/field name),
- a **state glyph** — a gradient chip / mini shape-sphere / palette dots / op icon — so the graph is
  readable at a glance without opening inspectors,
- **typed ports** on left (in) and right (out), **colour-coded by type** (`field` / `vec3` / `f32`),
- a small **`{ }` badge** when the node's code has been hand-edited (§10),
- an **accent outline** when selected.

---

## 9. Node inspector — morphing editors

Clicking a node fills the right inspector with **that node's editor**, chosen by node kind and, for
`colour`/`brightness`, by the **wired source subtype**:

| node / input                     | inspector editor |
|----------------------------------|------------------|
| `colour` ← `vector` (n̂)          | **shape-sphere / SiteBlend editor** (sites × kernel × swatch-or-gradient × blend-space) |
| `colour` ← `scalar`              | **gradient editor** (built-in LUT or custom stops, cyclic) |
| `colour` ← `categorical`         | **palette-swatch editor** (e.g. event palette) |
| `brightness` ← any               | **gradient / compaction editor** (greyscale + polarity) |
| `combiner`                       | Replace-L / Multiply |
| `post`                           | that op's controls (grid freq, physics κ/masses, band, tone, …) |
| `source`                         | field dropdown (which ctx field) |

The source-type-morph is **automatic**: the wire *is* the type, so there is no manual site-blend /
field-ramp toggle (unlike the standalone explorer prototype — the toggle collapses into the wire).

**Advanced ▸ edit shader** sits at the bottom of every node's inspector: reveals the generated WGSL
**for that node**, editable in place (per-node eject, `principia_colour_composition.md` §5). Editing
respects the node's function boundary; the node then shows the `{ }` badge and its graphical controls
grey out; **revert-to-generated** restores them.

---

## 10. `Graph | Code` — a view toggle, not an eject

A corner control on the canvas toggles **how the pipeline is represented**, not what it is:

- **Graph view** — nodes + wires.
- **Code view** — the assembled fragment WGSL for the whole pipeline (all node functions + shared
  library calls + the `shade()` that walks the graph).

The pipeline is the source of truth; both are renderings of it. **Graph view is offered iff the
pipeline still has graph structure:**

- No hand edits, or only per-node code edits that **preserve node function boundaries** → the toggle
  is lossless (flip freely; the edited node shows a `{ }` glyph in graph view).
- A **whole-pipeline code edit** that dissolves node boundaries into a form with no graph
  decomposition → **only code view can be rendered faithfully**, so the toggle stops offering graph
  view and stays on code.

This is **not a commit or an "eject with ceremony"** — there is no one-way dialog or revert prompt.
Plaintext simply becomes the only faithful representation once code has been written that has no
graph form. The rule is uniform: **graph view is available iff the pipeline still has graph
structure.**

The eject ladder, top to bottom, is therefore just *representation availability*: whole-pipeline →
whole-slot (a colour/brightness occupant) → single node. At every level, **peeking is free; editing
that removes structure removes the graph rendering at that level.**

### 10.1 The shared prelude library

The assembled WGSL is three layers: a **fixed shared prelude** (emitted / shipped once) → the
**per-node functions** → the `shade()` that walks the graph. The prelude is a library of general
helpers **any** node function may call — including hand-authored custom occupants (§10). It carries
the colour-space maps (`srgb_to_oklab` / `oklab_to_srgb` …), the ramps (`ramp_viridis`,
`ramp_coolwarm`, `hue_wheel`, `lut_sample` …), the reserved-colour constant, and value mapping.
Like everything else, it is emitted from the one Rust layout definition (canonical spec §8) — the
fragment side of the generation root, not hand-maintained per shader.

Two prelude members underpin the generated debug catalogue and are documented here because custom
shaders share them:

- **`range_norm(x, lo, hi, auto, meas) -> f32`** — min–max normalise `x` to `0..1` for ramp lookup:
  `clamp((x − l)/(h − l), 0, 1)` where `(l,h) = select((lo,hi), meas, auto)`. The `auto` flag chooses
  the range source. **`auto = false`** uses the **fixed `[lo,hi]`** — a per-field domain **declared in
  the manifest** from what is known *a priori*: physics/constraints (mass fraction `mᵢ ∈ [0,1]`;
  `|ρ| ∈ [0,1]`, `√I = 1` under `I = 1` canonicalisation; virial ≈ 1) or the encoding (a `u16` step
  index `[0, horizon_steps]`; an `f16` latch's range; `d_min ∈ [0, ~2]`). **`auto = true`** ignores
  the declared range and uses the **measured min/max** of the field over the current draw, supplied in
  `meas` (`meas.x = lo`, `meas.y = hi`) — a per-draw reduction uniform off the same machinery that
  produces `QuadReduction`. Trade: fixed is **absolute / stable / comparable across frames** but must
  know the range and **clamps out-of-range**; auto is **always full-contrast** with no prior knowledge
  but is **relative** (the mapping shifts with the data — absolute values are not readable, renders are
  not comparable). `range_norm` is a general helper, not debug-specific.
- **`DEBUG_NAN : vec3<f32>`** — the reserved invalid-pixel colour (the validity-first invariant, §13):
  a NaN reads as "no data", never as a value.

Each numeric debug field therefore generates a two-line `colour()` — `if (raw != raw) { return
DEBUG_NAN; }` then `ramp( range_norm(raw, lo, hi, RANGE_AUTO, u_range) )` — where `RANGE_AUTO` is the
fixed↔auto flag, editable identically in the node inspector, on the node in the graph, and in the
code (§9, §10). **Debug fields are raw:** apart from the NaN guard there is no validity masking — a
failed-state sentinel (e.g. `0.0`) is shown as its literal value, cross-checked against the raw
`state` field, not silently recoloured.

---

## 11. Presets = whole graphs

- The **catalogue is the library**. Its entries (production groups up top; the debug groups —
  addressing / quad-structure / decoded-IC / validity / diagnostic / profiling — below) are the
  browsable rows, each showing its **mapping glyph** (gradient chip / mini-sphere / palette dots).
  A **filter box** covers the ~50+ entries.
- **A preset is an entire serialized graph** — sources + colour/brightness + combiner + post + all
  per-node params. Selecting one **replaces the current graph wholesale**.
- **The catalogue's "default composition" IS the preset payload** — the ctx source(s), the
  gradient/polarity/palette, validity handling. It is not merely documentation; it is what expands
  into the graph nodes on load. *Catalogue and preset definitions are the same data, not two things
  to maintain.*
- **Debug presets load locked** — the graph is read-only; the first edit **forks a custom copy**
  (`principia_colour_composition.md` §6 locked-preset discipline). Production presets load editable.
- The relation at load is **one-way**: preset → graph. After loading you edit a graph instance; the
  preset is only where it started. (Saving a graph back as a new named preset is a later question;
  "duplicate to custom" suffices for the dev GUI.)
- **Bivariate presets** (`n̂ × FTLE`, `Outcome-class × diffusion`, …) load a graph with **both**
  combiner inputs occupied — one `source→colour`, one `source→brightness` — i.e. the second slot
  filled. There is no special bivariate UI; the family is just *the brightness slot occupied*.

---

## 12. Display stage — global, outside the pipeline

Applied to `OUT` after the graph, as **settings, never nodes** (no OUT-downstream graph):

- **gamut clamp**, **CVD simulation** (models the viewer, not the visualisation), **render→display
  scale**.

### 12.1 Structural overlays and tile debug shaders

Overlays are **not** one category. What separates them is **where the value comes from**, and that
alone decides whether an overlay needs any plumbing at all. Three tiers:

| Tier | Value is | Path | Examples |
|---|---|---|---|
| **1 · derived** | a function of **position/address** | ordinary **post node** — zero data | quad/tile **boundaries**, Morton |
| **2 · resident** | already on the GPU | **sim shader** — no new crossing | `QuadReduction` aggregates: depth, impurity, priority, coherence, valid-count |
| **3 · scheduler verdict** | a CPU decision, nowhere else | **tile debug shader** — small CPU→GPU buffer, postprocess | refine-decision, dirty, resident, generation, budget, ancestor gap |

**Tier 1 — boundaries are a post node, not a special case.** This is what the render contract already
says: *structural overlays → post slot + quad + `ctx.uv`*. Boundary-ness is a pure function of
**position within the quad**, so the fragment shader can draw it from `ctx.quad.uv` / `ctx.tile.uv`
with no data source, no buffer, no separate pass, and no CPU involvement:

```wgsl
fn edge_line(uv: vec2<f32>, width_px: f32) -> f32 {
  let d = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));   // distance to nearest edge, cell-local
  let w = fwidth(d);                                            // cell size in UV per pixel
  return 1.0 - smoothstep(0.0, w * width_px, d);                // constant px width, antialiased free
}
```

`fwidth` is what makes this work for **any** quad: it converts a UV distance into a *pixel* distance,
so a line is the same width at any quad size, any depth, any zoom — and it antialiases for free.
Thresholding UV directly (`u < 0.01`) would instead give fat borders on coarse quads and hairlines on
deep ones. The same expression on `ctx.tile.uv` gives tile boundaries; one node can composite both at
different widths and opacities.

Being an ordinary post node, the boundary overlay **serialises with the graph**, has editable width /
opacity / colour / level, chains in post order, and composes over any field — with no display-bar
special case and no bespoke pass. The rest of that render-contract list (**fallback tint**, **pending
hatch**, **visible-set**, **locked/stale**) is Tier 3 by this test: those are *values attached to a
quad*, not functions of position within it.

**Tier 3 — tile debug shaders.** Scheduler verdicts exist only on the CPU. When such an overlay is
active the **CPU packs per-visible-quad data into a small buffer (~16 KB for a typical viewport)** and
a **tile debug shader reads it in the postprocess stage**; **zero cost when off**. These share the
compilation pipeline and prelude (§10.1) with the sim shaders, differing in grain (visible quad, not
sample), data source, and composite stage. Compositing in postprocess is what lets them layer over any
sim-shader field and drive the "watch-the-frontier-grow" animation from live state.

**The membrane law is not in tension with Tier 3.** "Big data never crosses" governs *per-sample*
records (millions per viewport). This is *per-visible-quad* metadata — hundreds of entries, a fixed
~16 KB, uploaded only while the overlay is on. The quantity that must never cross is the `SimState`
array, not a handful of scalars per visible quad. That ~16 KB is nonetheless the **design budget** for
this catalogue: it bounds the payload to a few scalars per visible quad, and it is the criterion for
whether a scheduler quantity earns a slot or goes to **inspector drill-down** (click a quad → its
metadata as text).

**Sorting rule.** Ask *"can the fragment shader compute this from the address alone?"* → Tier 1, post
node. Else *"is it already GPU-resident?"* → Tier 2, sim shader. Only if neither → Tier 3, and only
then does it cost a buffer. (`leaf/internal` was dropped: redundant with depth, near-constant per
pixel.)

Per-quad detail too fine to earn a slot in the ~16 KB budget goes to **inspector drill-down** (click
a quad → its metadata as text). That budget is the real design constraint on this catalogue: it
bounds the payload to a few scalars per visible quad.

Display-stage settings live in the top display bar and apply uniformly to the main render and the
preview; tile debug shaders are toggles in the same bar but are *shaders*, not settings.

---

## 13. Invariants

- **Always-renderable.** The graph is evaluated on every change (cheap — it selects the fragment
  shader). A graph is *always* renderable: **dangling / absent inputs fall back to occupant identity
  (None)**, so you can never wire yourself into a black screen. A genuinely invalid graph (should be
  unreachable given the fixed OUT+combiner and identity rules) shows a **defined fallback** (flat
  grey / error tint), never a crash.
- **Occupant identity (None):** `colour`-None → **greyscale of brightness**; `brightness`-None →
  **colour's own L** (pass-through); **both**-None → flat mid-grey. (Matches `principia_colour_
  composition.md` §4.1.)
- **Validity-first.** Every field carries its validity lane; every colouring has an explicit
  invalid-pixel colour — a NaN / sentinel must read as "no data", not as a value (composition spec
  §3, §6).

---

## 14. Prototype → GUI mapping

| prototype | role in this GUI |
|-----------|------------------|
| `principia_colour_explorer.html` | the **colour-node inspector** — its site-blend/field-ramp faces are the type-driven editors of §9 (the toggle becomes the wired source type) |
| `principia_render_modes.html`    | the **library panel** (§11) — its groups, glyphs, and default-composition metadata are the preset rows / payloads |
| `principia_colour_composition.md`| the **algebra** this GUI edits — sources, occupants, combiner, post, codegen/eject, ctx, defaults |

---

## 15. Settled decisions (record)

1. **Source is first-class** — a `source` node with a field param; not implied only by presets.
2. **Explicit source nodes** (not folded into colour/brightness).
3. **Free-ish graph** with a **fixed OUT + combiner backbone**; acyclic; type-checked.
4. **combiner and OUT are fixed singletons.**
5. **Drag-off-pin creates a type-filtered node** on empty-canvas release.
6. **In-ports take one wire** (last-write-wins); **multi-source = multiple input ports**, not
   multi-wire-per-port.
7. **Node inspector = side panel**, morphing by node/source type; per-node Advanced-WGSL at the
   bottom.
8. **`Graph | Code` is a representation toggle**, not an eject; graph offered iff structure survives.
9. **Whole-pipeline Advanced** = the code view of the whole graph; sticky only because the graph
   can't be reconstructed from dissolved code.
10. **Presets are whole graphs**; catalogue default-composition = preset payload; debug locked →
    fork-on-edit.
11. **Display stage (gamut / CVD / scale / boundary overlay) is global**, outside the pipeline.

---

## 16. Open / next

- **Build the interactive dev-GUI mock** — library → graph (drag / wire / drag-off-pin, backbone
  constraints, glyph nodes) → click-node → inspector (real sphere / gradient / palette) →
  `Graph | Code` → live preview.
- **Node palette contents** — the concrete list of source fields (from the ctx contract) and post
  ops surfaced in the right-click palette.
- **Preview** — sphere vs illustrative-slice toggle; which is default.
- **Standing composition-spec gaps** (to reconcile on that doc's next pass, tracked in
  `principia_colour_composition.md`): colour-source-as-axis, overlays-as-post-chain-over-configured-
  base, physics-as-overlay-op, gradient-unifies-the-ramp, per-footprint vs quad-aggregate spread,
  the debug taxonomy, the addressing channel convention (UV=RG / TL=RB / centre=BG / index=grey), and
  boundaries-as-overlay. This GUI spec assumes those resolutions.
