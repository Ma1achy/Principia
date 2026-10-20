# Principia: the Print graph (post-processing)

**Adopted by R-485 to R-492 (20 Oct 2026, the human's P1 to P8).** This document is part of the corpus from those
rulings on: where it says "proposal", the proposal is what is built, except where R-485 to R-492 or a later ruling says
otherwise. Its §12's four open questions are not ruled: they are RQ-388 to RQ-391. Its reference material is in
`docs/gui/reference/print_graph/` (reference, not code to build on; its README): the paths below that start
`reference/`, `catalogue/`, `images/`, `tools/` or `boards/` are relative to that folder. Read with the review queue's
entries on points where this document and an earlier ruling or doc disagree, each named where it applies: RQ-395 (where
the lines drawn over sit, §2), RQ-396 (R-130's styles as presets, §1), RQ-397 (the badge, §3), RQ-398 (pixels that are
not a system, §3) and RQ-401 (the lab's stand-in, §4.4).

*Design document. Goes to `docs/design/principia_print_graph.md`. Supersedes R-130's v1 style list and replaces the
Display window's "Style" with the Print graph. Everything here is a proposal for the rulings in the accompanying
instruction; nothing here overrides a ruling until it is written into `decisions.md`.* *(Its status as delivered;
R-485 to R-492 adopt it, above.)*

---

## 1. What it is

The **Print graph** turns the finished, coloured figure into a picture made in a medium: watercolour, offset press,
linocut, a CRT, cross-stitch. It is a node graph, like the Stain, but it answers a different question.

| | Stain | Print graph |
|---|---|---|
| Question | what colour is this system? | what is the picture made of? |
| Reads | the payload (SimState) | the stain's finished colours, and optionally payload fields as **read-only** maps |
| Output | the figure's colours, which every measurement reads | a styled image, which **no** measurement reads |
| Default | the chart's stain | **Plain** (the identity) |
| Re-integrates? | never | never |

The design has two prototypes behind it, both in this bundle:

- **The Print graph lab** (`reference/print_graph_lab/`, published as the "Principia Print Graph" artifact): 78 node
  types in WebGL2/GLSL, 103 presets in 11 families, and the slot chain (§5). **This is the reference for which nodes
  exist and what they look like.**
- **The Python prototype** (`reference/fx_prototype/`): every medium as a JSON graph of small nodes, with settings,
  `$setting` references, presets usable as nodes, explicit random streams and pixel-exact verification. **This is the
  reference for the data format (§4).**

The port takes the lab's nodes and the prototype's format. It does not take either codebase.

## 2. Where it sits

*The lines drawn over sit after the Print graph; whether before the display scale, as drawn below, or after the gamut
clamp, as R-476 M2 placed the line system, is RQ-395 (R-487).*

R-67's chain, with the Print graph where "style" was and the line system placed explicitly:

```
SimState ─► Stain ─► [line sets marked "through the medium"] ─► Print graph ─► [line sets drawn over] ─►
          display scale ─► gamut clamp ─► colour-vision simulation ─► screen
```

- **Lines are drawn over by default.** Contours, traces, axes and grids are composited **after** the Print graph, so
  they stay crisp and readable on any medium.
- **A line set can opt in to being rendered through the medium.** Its `LineStyle` gains `through_medium: bool`
  (default `false`). Marked sets are composited onto the stain's output **before** the Print graph, so a pencil
  contour comes out as watercolour pencil, a trace as a linocut line. Axes are a line set too, so the same switch
  applies. The two passes are the same line renderer called twice with complementary filters.
- **Colour-vision simulation sees the styled picture**, as R-67 already orders.
- **Debug views bypass it.** A debug view (R-41's baked variants) is never styled, because it exists to show the data.

## 3. What never changes because of it

- **Measurement reads the stain, never the print.** The Inspector, Measure, the legend, the colour readout under the
  cursor, area statistics and every scientific check read the pre-print colours or the payload. A Print graph cannot
  change any number the instrument reports.
- **Goldens and scientific checks run under Plain.** A test that renders a figure sets Plain explicitly. It does not
  inherit the user's Print graph.
- **A styled figure says so.** While any Print graph other than Plain is active, the canvas corner badge names it
  ("Print: Linocut"). Figure export keeps embedding's existing obligation: a scientific figure is the renderer's
  pixels. So the export dialog offers **Styled** and **Plain** and defaults to Plain for figure export, as embedding
  §9 already does for the low-bit record.
- **No re-integration, no sim key.** The Print graph lives in `RenderState`, edits are colour-side and coalesce like
  any other `SetField` (R-96), and a change never touches the cache.

## 4. The data model

### 4.1 A graph is data

```json
{
  "format": "principia.print/1",
  "name": "Linocut",
  "settings": { "seed": 3, "depth": 0.7 },
  "nodes": {
    "img":  { "type": "render" },
    "fl":   { "type": "flow",    "params": { "sigma": 5 },        "in": { "in": "img" } },
    "pap":  { "type": "paper",   "params": { "colour": "#efe9dc" } },
    "cut":  { "type": "linocut", "params": { "depth": "$depth", "seed": "$seed" },
              "in": { "in": "img", "flow": "fl", "paper": "pap" } },
    "out":  { "type": "output",  "in": { "in": "cut" } }
  },
  "out": "out"
}
```

- **`nodes`**: id → `{ type, params, in }`. `in` maps each input port to `node` or `node.port`.
- **`settings`**: a graph's public knobs. A parameter written `"$name"` or `"$name+2"` reads a setting. The simple
  panel (§7.1) shows settings, not node parameters.
- **Presets are graphs, and a graph can use a preset as a node**: `{ "type": "preset:Polaroid", "in": { "in": "x" } }`
  runs that preset's graph as a sub-graph. This is how "Light painting, on a Polaroid" is two nodes (the Python
  prototype's rule).
- **Editor positions are not part of the graph.** They live in `ViewUI` beside it, so moving a node is not an undo
  entry and two equal graphs serialise equal.
- **Serialisation is the canonical one** (R-309, JCS by R-318). Node ids are sorted by JCS and carry no meaning.

### 4.2 Port types

Every port is typed, and the editor refuses a wire between mismatched types. This is the lab's implicit convention
made explicit.

| Type | Contents | Produced by, for example |
|---|---|---|
| `image` | linear RGB + alpha, f16 | `render`, every medium |
| `scalar` | one channel, `[0,1]` unless stated | `brightness`, `fringe density`, `noise`, a payload field |
| `direction` | a unit tangent field and its coherence | `flow` |
| `ground` | surface colour plus tooth height | `paper`, `ground` |
| `mask` | one channel, `{0,1}` | thresholds |

An unconnected input takes the default its node type declares (the lab's `defaults: { ground: "paper", field:
"white" }`).

### 4.3 Node types are a registry

Node types are a **registry**, as the link functions are (dd_generation_root §3.9). Each entry declares:

- `id`, `label`, `category`, a one-paragraph `doc` (the lab's `doc` strings are the starting text);
- typed inputs with their defaults, and typed outputs;
- parameters, each with a type, default, range, step and **unit** (§6.2);
- its passes, in WGSL;
- a **cost class** (`light`, `medium`, `heavy`) and a **motion policy** (§8);
- whether it is **temporally stable** (§6.3).

The registry is checked at build time: every parameter has a range and a unit, every input a type, every preset
references only registered types and in-range values. A preset is data, so adding a medium needs no code when its
nodes exist.

`catalogue/node_catalogue.json` in this bundle is the lab's registry exported as data: 78 types, their categories,
docs, inputs and parameter ranges. It is the porting checklist.

### 4.4 Payload fields as inputs

*The lab's caption names fringe density as the stand-in, and the lab's four graphs use brightness, fringe density or
both; the deviation each port lists is RQ-401's (R-492).*

A graph may read a payload field as a `scalar` input: `{ "type": "field", "params": { "name": "t_end" } }`. This is
the lab's "Driven by the data" family (Relief of lifetime, Wet where chaotic, Craquelure, Unfinished where uncertain),
where the medium follows the physics: the relief is the lifetime, the wash is wet where the spread is high.

- **Read-only.** The field is the same value the stain reads, normalised over the visible range with the stain's own
  window rule. The Print graph can never write it.
- **Absent is a defined state.** If the field is not in the payload (a tier that does not compute it), the input
  takes the node's declared default and the panel says "field not available at this tier". It is never silently
  replaced by brightness. In the lab these four presets use brightness as a stand-in, which is why they carry a `*`;
  in the port they read the real field.

## 5. The slot chain: palette and medium compose

The lab's answer to "a limited palette composed with watercolour pencil" is a fixed chain of slots around a medium:

```
render ─► Palette ─► Medium ─► On the ground ◄─ Ground
                                    │
                                    ▼
                               Unfinished ─► Age ─► Viewing ─► output
```

| Slot | What it holds | Lab parameters |
|---|---|---|
| **Palette** | nearest-colour mapping onto a named palette, by lightness or by class | mode, amount, wheel |
| **Medium** | one medium preset (any graph with one `image` in and one out) | the preset's settings |
| **Ground** | the surface: 23 grounds, from hot-press watercolour to a brick wall | type, tooth |
| **On the ground** | how the surface takes the medium: absorbency, tooth | type, absorb, tooth |
| **Unfinished** | how far into the picture the artist got, beyond which only the underdrawing shows; can be driven by a field | progress, from, rag |
| **Age** | what happened afterwards: fading, yellowing, foxing, folds, varnish | years, drive, ... |
| **Viewing** | how the light falls on it, always full frame | mode, light, elevation, strength |

**The slot chain is itself a graph.** It is the template the simple panel edits (§7.1). Each slot holds a sub-graph
(`preset:...`), and an empty slot is the identity. So "limited palette + watercolour pencil + cold-press + 40 years"
is four slot choices, and the advanced editor shows the same thing as nodes.

## 6. Determinism, scale and time

### 6.1 Determinism

The same `RenderState` must give the same pixels.

- **Same backend:** bitwise.
- **Across backends:** within the parity contract's colour tolerance.

Every random draw comes from a seeded stream: a `seed` parameter, or a stream wired between nodes as in the
prototype. Nothing reads the clock, the frame counter or `Math.random`. This is what lets an embedded image or a
share link reproduce a styled figure (the round-trip rule, tier 2).

### 6.2 Units: a medium must look the same at every export size

The lab measures stroke widths, dot pitches and cell sizes in **screen pixels**. A 4096² export would then be a
different picture: strokes a quarter as wide relative to the figure. So every length parameter declares a unit:

- **`ref_px`** (the default for marks: strokes, dots, cells, stitches): pixels on a 1024-wide figure, scaled by
  `output_width / 1024` at render time.
- **`device_px`**: true screen pixels. Only for effects that model the display itself: CRT scanlines, LCD subpixels,
  LED pitch.
- **`fraction`**: a fraction of the figure width, for broad effects such as vignette and lighting.

The acceptance test renders a preset at 512² and 2048², downsamples the second, and requires agreement within
tolerance for every `ref_px` and `fraction` node. `device_px` nodes are exempt and listed.

### 6.3 Anchoring and stability under motion and playback

- **The sheet is anchored to the screen.** Paper tooth, canvas weave, grain and the ground stay fixed while the figure
  pans and zooms beneath them. The picture is redrawn each frame, but the sheet it is drawn on does not move.
- **Marks are stable where the picture is.** Noise-driven marks (dither thresholds, stipple, stroke seeds) are seeded
  from screen-anchored coordinates and the graph's seed, never from the frame. Where the figure does not change
  between frames, neither do the marks: a still region of a moving playhead does not shimmer.
- **Boil is a setting, off by default.** Hand-made animation re-draws its marks every frame. `boil: off | per frame |
  every N frames` is a graph setting for animation export, recorded with the graph.

Not every node can be stable: a node whose look depends on its whole neighbourhood (Kuwahara, stained-glass cells)
changes where its neighbourhood does. Each registry entry declares `temporally_stable`, and the panel marks the
unstable ones.

## 7. The GUI

### 7.1 The Print panel (replaces Display › Style)

- **Plain / Print** toggle at the top. Plain is the default and the identity.
- **Slots**, top to bottom as §5, each a dropdown with a small live thumbnail of the current view in that choice:
  Palette, Medium (the preset families as groups), Ground, On the ground, Unfinished, Age, Viewing. Each slot has its
  graph's **settings** beneath it, not its node parameters.
- **Preset gallery**: the 103 presets as live thumbnails of the current view, grouped by the lab's 11 families.
  Picking one fills the Medium slot. Thumbnails render at low resolution, lazily, at most a few per frame.
- **Edit graph…** opens the advanced editor (§7.2).
- **Reset** returns to Plain, and is one undo entry.

### 7.2 The graph editor

The Stain's node editor component, reused: same wiring, same typed ports, same undo, same keyboard scopes. It edits
the same graph the slots show. Custom nodes (user WGSL) are the Stain's occupant mechanism (render contract: "Custom is
not a node kind, it is an occupant") and are tier 3 in the round-trip rule.

### 7.3 Lines

Each line set's style panel gets "Through the medium" (§2), off by default.

## 8. Cost and motion

### 8.1 Degrade while moving

This is Malachy's ruling. While the camera moves (the frame record's `camera_delta > 0`), or a drag is in progress:

1. Each node runs according to its **motion policy**:
   - `full`: light nodes;
   - `reduced`: run at half or quarter resolution and upsample; most nodes;
   - `bypass`: replaced by the identity while moving; nodes whose reduced form would look wrong, such as ASCII, where
     half-resolution characters are a different picture.
2. If the post stage still exceeds its share of the frame budget, the **whole graph** drops a level: reduced, then
   Plain.
3. **On settle** (no camera change for `settle_ms`, default 250 ms, with hysteresis), the graph renders at full
   quality once and holds it.

The playhead advancing is **not** motion for this purpose. The view is still, so the medium renders at full quality
and stability (§6.3) keeps it from shimmering. If full quality cannot hold the playback rate, the frame record says so
and the tier controller treats the post stage like any other stage.

### 8.2 The budget

The post stage is a stage in the telemetry frame record (`stage_ms.post`). It is not exempt from the tier
controller's accounting; the tier simply has a post-stage share. Which degrade level ran is recorded per frame
(`post_level: full | reduced | plain`), so a profile shows when degradation happened and the benchmark (see
`principia_benchmark.md`) measures it per node class.

### 8.3 Cost classes

Measured, not guessed. The port's first task measures every node at 1024² on the reference GPU and assigns its class
from the numbers. The lab's structure predicts:

- **heavy**: Kuwahara, strokes, stained glass, stereogram, thread painting and other multi-pass neighbourhood nodes;
- **light**: the per-pixel adjust nodes.

The measurement decides.

## 9. Provenance

- The Print graph is a `RenderState` field, `print: PrintGraph`, and goes everywhere `RenderState` goes (snapshot,
  share link, pxpack, embedded record, `tEXt` chunk, animation spec).
- **By value, always.** A preset is recorded as the graph it expanded to, plus its preset id as a label. Changing a
  built-in preset in a later build cannot change what an old record reproduces.
- **Round-trip tiers:** the graph with built-in nodes is **tier 2** (colour); a custom WGSL node's source is **tier 3**
  and is the first thing dropped when an embedded record does not fit. A record that dropped tier 3 recreates with the
  custom node shown as missing and the figure rendered without it, badged.
- **Interpolating across keyframes:** numeric settings interpolate; a change of graph structure or preset is a cut at
  the keyframe.

## 10. Porting from the lab

- **One node at a time, against the lab's own output.** For each node, the reference is the lab's render of that node
  at its defaults on the bundled source image (`images/nodes/`). The port renders the same graph on the same image and
  must match within a perceptual tolerance (ΔE in OKLab, median and p99, thresholds set in the first port task and
  then fixed). The lab is run headless by `tools/capture_nodes.js`, which is what made these images, so the agent can
  regenerate them at any size.
- **Presets the same way**, against `images/presets/`.
- **GLSL → WGSL is a translation, not a redesign.** Where the lab cheats (screen-pixel units, frame-seeded noise, a
  brightness stand-in for a field), the port follows this document instead, and the deviation is listed in the
  node's registry entry, so a mismatch against the lab is explained rather than tuned away.
- **The lab's quirks are recorded, not inherited.** One example: the lab page has no `<meta charset>`, so served
  locally its labels render as mojibake (`8Ã—8`). The bundled copy adds the meta tag.

## 11. Tests (each with an arm that can fail)

1. **Plain is bitwise identity.** Control: any non-Plain preset must differ.
2. **Measurement is blind to print.** With a heavy preset active, the Inspector, the colour readout, area statistics
   and a stain golden read bitwise what they read under Plain. Mutation arm: a readout wired after the print stage
   must fail.
3. **Determinism.** The same graph twice gives the same bytes. Control: changing only `seed` changes the bytes.
4. **Scale invariance (§6.2)** for every `ref_px` and `fraction` node. Control: one node deliberately left in
   `device_px` must fail it.
5. **Stability (§6.3).** Two frames with an unchanged figure give identical output for every `temporally_stable` node.
   Control: `boil: per frame` must differ.
6. **Lines over and through.** A contour set renders identically with and without a Print graph when over. Marked
   through, it differs.
7. **Motion degrade.** A scripted pan records `post_level` dropping and returning to `full` within `settle_ms` of the
   last camera change. Control: a still view never leaves `full`.
8. **Registry.** A preset referencing an unknown node type or an out-of-range value fails the build, with the name.
9. **Port fidelity.** Every node and preset is within tolerance of its lab reference, or listed with its reason (§10).
10. **Round trip.** A non-Plain graph with settings, a sub-preset and a field input survives every export path bitwise
    (the round-trip rule's T1).

## 12. Open questions (for the review queue, not the agent to settle)

*Queued as RQ-388 to RQ-391, one per question, in order (the human's point 6: "Do not rule on either doc's open
questions"); what is built meanwhile is each entry's.*

1. Should **Palette** be allowed to run *before* the stain's lightness channel is composed, so a limited palette can
   quantise hue while keeping the stain's lightness field exact? The lab's palette slot acts on the finished colour.
2. The **Viewing** slot's lighting assumes a sheet. Should it be disabled for screen media (CRT, LCD), where a raking
   light is meaningless?
3. **Field inputs at deep zoom.** A field normalised over the visible range changes its normalisation while zooming,
   so a "Relief of lifetime" relief rescales as you zoom. Normalise over the visible range (consistent with the stain),
   or over a fixed window recorded with the graph?
4. Should the **Export Styles** frames (Polaroid border, banknote, poster) become a later "Frame" stage after the
   Print graph? Ruled out of scope for this bundle.
