# Principia: export details — the colour window, the viewer's settings and mark sizes

*Adopted by R-581, R-582 and R-583 (gap sweep PR 2c, 21 Oct 2026), from the human's design `export_details.md`
(`docs/incoming/2026-10-21_gap_sweep_exports_picture_ic/designs/`), its "Decided" section his words: "All three accepted
as proposed". Where the draft gave a choice with a recommendation (its §5), the recommendation is what was accepted.
Queued from it: RQ-562 (the export's frozen window and `base_width`, written into the exported `RenderState`, against
R-484's round trip), since ruled by R-588 (24 Oct 2026): the identity holds on the record.*

## 1. What exists

Export & share offers an image at multiples of the view, with the pxpack and Styled / Plain
(`principia_render_gui_spec.md` § "Export & share"); Record a time sweep sets a range, frames, a size and a format (§G9).
`range_norm` takes a declared `[lo, hi]`, or under `RANGE_AUTO` the min and max "measured over the current draw"
(render GUI spec Part II § "10.1 The shared prelude library"). The embedding carries "THE RAMP WINDOW", "not optional"
(`principia_dd_image_embedding.md` §6). The display chain ends gamut clamp → lines drawn over → colour-vision
simulation, which "models the viewer" (`principia_colour_composition.md` §4.3; R-123); the state contract names no home
for it (`principia_gui_state_contract.md` §2). Sizes follow three rules: `ScreenPx | PrintPt` (`principia_lines.md` §3,
§5), the Print graph's `ref_px` × `output_width / 1024` and `device_px` (`principia_print_graph.md` §6.2, R-490). Icons,
arrows, label offsets and the smart-ink radius are "px" with no rule (lines §3, §6, §7).

## 2. The gap

- **(a) The window.** A 4× export measures its window over different pixels from the screen's; a recording re-measures
  every frame, and its colours flicker. Nothing says whether the window travels as numbers or as "auto".
- **(b) The viewer's settings.** Nothing says whether the colour-vision simulation or `render_scale` reach a file; the
  simulation is neither recorded nor excluded.
- **(c) Sizes.** `PrintPt` needs a DPI no export has (REQ-RENDER-088 tests "the export's DPI"); `ScreenPx` and `ref_px`
  disagree unless the export's width happens to be a multiple of the view's.

## 3. The design

### 3.1 The window is frozen at export (R-581)

- **An image export freezes every measured range.** When Export is pressed, each `range_norm` window in use whose source
  is measured (`RANGE_AUTO = 1`: a stain node's, and a Print graph field input's, which reads the stain's window rule,
  `principia_print_graph.md` §4.4) is written, as the `(lo, hi)` of the `u_range` the screen is drawing with at that
  moment, into `RenderState.frozen_ranges: { node_id → (lo, hi) }`, f32 as `u_range` is. The exported record carries
  that `RenderState`. A node with an entry reads it instead of measuring: `range_norm` gains a third source, **frozen**,
  beside declared and measured (render GUI spec Part II §10.1). A declared range is never frozen; it is already fixed.
- **Clipping is stated.** The export records, per frozen node, the share of the exported pixels it clamped at each end,
  `clipped: { node_id → (lo, hi) }`, each the fraction, in `[0, 1]`, of the export's pixels the chart classifies as a
  system (R-467) whose value fell below `lo` or above `hi`; a pixel that is not a system counts at neither end (applied
  per R-369).
- **A recording uses one window, frozen when Record is pressed** (the human's words: "a recording's colour range is
  frozen when Record is pressed (clipping reported)"): Record a time sweep freezes the window on screen at that moment,
  as an image export does, and every frame of the recording reads it; the clipped shares are recorded per frame.
  **Per frame** is an explicit choice in the Record dialog, off by default, labelled "re-ranges every frame: frames not
  comparable", and recorded as `range_policy = per_frame`; the default is `range_policy = frozen`. Under `per_frame` no
  node is frozen and each frame measures its own window, as before.
- **A pre-pass, opt-in** (the draft's open question 1, "(ii) a cheap low-resolution pre-pass over all frames, union of ranges" … "*Recommend (i)*, clipping reported; (ii) opt-in.", accepted as proposed; ported by fix round 3 of PR #287, code review 5638122561, C1): the Record
  dialog offers **Pre-pass**, off by default, labelled "window from a quick pass over every frame". Picked, before the
  recording starts every frame is rendered once at a reduced resolution and each measured node's window is measured
  per frame; the union of those windows (the smallest `lo`, the largest `hi`) is frozen for the whole recording and
  recorded as `range_policy = prepass_union`, and the clipped shares are still recorded per frame, at the recording's
  own resolution. The pre-pass's resolution is a calibration value (R-71, REQ-TOOL-217). The Sequence's planner
  offers it as Record does (applied per R-369, as for `per_frame`).
- **The Sequence's export** (`principia_sequence.md` §3.7) is a recording too, made of keyframes that may show different
  views. It freezes, when its Start is pressed, the window on screen for every measured node the view on screen draws,
  for the whole export; a measured node no keyframe shares with that view is frozen at the first export frame that
  draws it, measured over that frame at the export's size, and the planner and the record name each such node. Its
  planner offers `per_frame` as Record does (applied per R-369: the human's "frozen when Record is pressed" names the
  sweep's button; this is the same rule for the other recording).
- **On import** a frozen node reads "frozen at export" in its inspector and the Legend's key for it; **Re-measure** in
  the node's inspector removes its entry, one `SetField` on `RenderField::FrozenRanges` (the whole map), one undo entry
  (R-69). A snapshot JSON or share link is not an image export: it carries `frozen_ranges` as the view has it and
  freezes nothing (applied per R-369).
- **The record is the identity (R-588, RQ-562 (a)).** "An export is a record of what was drawn" (the human): import
  gives back, bitwise, the `RenderState` the export wrote, its frozen windows and `base_width` included, so the import
  reproduces the image that was drawn and never re-measures a live window; R-484's identity holds on that record. The
  export reports the fields it froze, in its export block's `froze` (§3.4). R-484's T1 runs on the headless paths,
  which freeze nothing.
- **An entry goes with its node** (applied per R-369, fix round 1 of PR #287, gui review 5638052211, G6): a stain-graph edit that removes a node with
  an entry (Delete node, render GUI spec Part II §7) or a preset replacing the whole graph (Part II §11) drops the
  entry of every node no longer in the graph in the same undoable command, one history key and one undo entry
  (§G2's "one undoable command"), and undo restores both; so a new node that reuses an id never inherits a stale
  freeze.

### 3.2 The viewer's settings stay with the viewer (R-582)

- **The colour-vision simulation lives in `ViewUI`**, `ViewUI.cvd: Off | Deuteranopia | Protanopia | Tritanopia |
  Achromatopsia` (R-123's five), reaching the renderer as a uniform the GUI sets, as the canvas background does
  (`principia_gui_state_contract.md` §2). It is never undoable, never written to any record and never restored from
  one, and it is remembered across sessions in the GUI's own settings (the draft's recommendation to its open question
  2, accepted).
- **An export never includes it** unless the user picks **"As seen with: <mode>"** in Export & share's Image section, a
  checkbox off by default, offered while `ViewUI.cvd` is not Off. Picked, the export's pixels are the simulation's; the
  record carries `cvd_simulated: <mode>` (and `cvd_simulated: null` in every other export); the file name gains the
  suffix `__as_seen_<mode>` (for example `__as_seen_deuteranopia`); and wherever the app shows such an image (Import
  picture's preview, §G9), its header strip carries the badge "As seen with <mode>" (R-523). Import picture's report
  names it, and restoring the view does not turn the simulation on: the restored `ViewUI` is the user's own.
- **`render_scale` is 1 in every export**, as the checkerboard is forced off (`principia_export_animation_contract.md`
  Part 4): the exporter renders at the export's own resolution, whatever the view's `render_scale`.
- **A screenshot is the screen.** The app writes no screenshot today; any image the app writes of the window as shown
  records the `render_scale` and the `ViewUI.cvd` mode it captured, beside its record, and is never an export (applied
  per R-369: the rule for a feature not yet designed, so that it cannot pass for an export).

### 3.3 One size rule: base px (R-583)

- **Every mark length is in base px**: a stroke's width, a dash's lengths, an outline's width, a label's size, offset,
  halo width and spacing, an icon's and an arrow's size, a tick's length, smart ink's radius, and the Print graph's
  marks (what `ref_px` sized). `LineStyle`'s `units` field is gone: there is one unit. **`PrintPt` is retired.**
- **`RenderState.base_width: u32`** is the figure's width in logical px (the display's scale factor removed) when the
  view is saved or exported, written by the writer into the record as the figure's width at that moment. A spec with no
  view (the reel's builder, a headless render) keeps the `base_width` its state carries; a record without the field,
  from before R-583, reads 1024.
- **A rendered length is the length × `output_width / base_width`**: 1 on screen (on screen the renderer draws at a
  scale of 1, whatever `base_width` says, so a mark keeps its size when the window is resized), `k` for a `k×` export,
  and the same ratio for any size. **Label placement runs at base size and is then scaled**: spacing, collisions and
  offsets are decided in base px, so a 4× export places the same labels as the screen.
- **On screen 1 base px is one logical px** (applied per R-369, fix round 1 of PR #287, gui review 5638052211, G5): at a display scale factor `s` a mark
  of `L` base px is drawn `L × s` physical px long, as every other GUI length is. `base_width` and the view's size are
  in logical px, and Export & share's "multiples of the view" count logical px: a `k×` export is `k × base_width` px
  wide, so its ratio is `k` on every display.
- **`device_px` stays the one exception**, listed (`principia_print_graph.md` §6.2): a node that models the display
  itself keeps true screen pixels, and an export's record lists every `device_px` node it drew.
- **Physical size** is an optional export field, a width in mm and a DPI, which sets `output_width =
  round(width_mm / 25.4 × DPI)` and writes the PNG's `pHYs` chunk (pixels per metre `round(DPI / 0.0254)`, unit 1); the
  style editor then shows each length's size in pt beside its base px, `pt = base px × (width_mm / 25.4 × 72) /
  base_width`, as a readout, never a second unit (the draft's recommendation to its open question 3, accepted).
- **The Print graph on screen** (the draft's open question 4, its recommendation accepted): `ref_px` is base px, with
  1024 the base where a state has none, so on screen a medium keeps its size when the window is resized, and R-490's
  export-size invariance still holds: two exports of one state, at any two sizes, scale by the same `output_width /
  base_width` rule.

### 3.4 What every image records

Every exported image, and every recording, carries beside `SimConfig` and `RenderState` an **export block**, a sidecar
outside the round-trip identity as `f3_fill` is (R-484 J3), in the low-bit record's tier 2 with `f3_fill` and in the
`tEXt` chunk always (applied per R-369):

```
export { range_policy: frozen | per_frame | prepass_union,
         clipped: { node_id → (lo, hi) }                 an image; a recording: one map per frame, in frame order
         cvd_simulated: null | <mode>,
         render_scale: 1,
         device_px_nodes: [node_id],
         froze: { nodes: [node_id], base_width: null | u32 } }
```

*Was:* `export { range_policy: frozen | per_frame,` (fix round 3 of PR #287, code review 5638122561, C1, adds `prepass_union`).
*Was:* `         device_px_nodes: [node_id] }` (qa review 5638292369 of PR #287, Q1, adds `froze`, R-588).

**`froze` is the export's report of the fields it froze (R-588)** (applied per R-369: where the report lives, fix round
of PR #287's rulings port, qa review 5638292369, Q1): `nodes` lists, in `node_id` order, every node whose
`frozen_ranges` entry this export wrote, a node the view had already frozen not among them, and `base_width` is the
value the export wrote into `RenderState.base_width`; a headless export, which freezes nothing and keeps the state's
`base_width`, reports `nodes: []` and `base_width: null`, and a recording under `per_frame` reports `nodes: []`. It is
in the export block, outside the identity, so the `RenderState` an import gives back is the one the export wrote and
`froze` says which of its fields the export set.

`frozen_ranges` and `base_width` are `RenderState`'s, so they are in the identity and in tier 2 as the ramp window is.

## 4. Fit with rulings

- Embedding §6 holds literally: the window travels as numbers. `range_norm` gains a third source, frozen.
- Export contract Part 4 gains "`render_scale` = 1; the colour-vision simulation off unless chosen".
- State contract §2 gains `ViewUI.cvd`, `RenderState.frozen_ranges` and `RenderState.base_width`.
- **Amends** REQ-RENDER-088 (`PrintPt`), lines §3 and §5's units and R-476 as it adopted them, and print_graph §6.2 and
  R-490: `ref_px` becomes base px with 1024 the default base; R-490's export-size invariance still holds.
- The export writes `frozen_ranges` and `base_width` into the `RenderState` it records, and R-484's "import of an
  export is the identity" holds on that record (R-588, closing RQ-562; §3.1, §3.3). *Was:* "which R-484's "import of
  an export is the identity" reads against the live view's state: RQ-562 (§3.1, §3.3)."

## 5. Decided

The human, 21 Oct 2026 (`export_details.md`, "Decided", verbatim):

> All three accepted as proposed: a recording's colour range is frozen when Record is pressed (clipping reported);
> colour-blindness simulation is never baked into an export unless asked, and is recorded when it is; line, label and
> mark sizes scale with the export by one rule ("base px").

`DECISIONS_2026-10-21.md`, verbatim: "Export details: yes to all three (recording colour range frozen at Record;
colour-blindness simulation not baked in unless asked; line and label sizes scale with the export)."

## 6. Tests

Each with what makes it fail (the draft's §6):

- **frozen_range_roundtrip**: export, import, render: bitwise equal. *Fails if* import re-measures.
- **recording_one_window**: 10 frames, an auto node: equal `u_range` in every frame's record. *Fails if* any differs.
  Control: `per_frame` gives at least two.
- **clipped_share_counts**: a field forced past the window reports `clipped_hi > 0`. *Fails if* it reads 0.
- **cvd_not_in_export**: the simulation on gives the same pixels as off. *Fails if* any differ. Control: "As seen with"
  differs and is recorded.
- **render_scale_forced**: an export at screen `render_scale = 0.5` equals one at 1. *Fails if* not.
- **base_px_invariance**: 1× against 4× box-downsampled: every mark within tolerance, identical placed-label sets.
  *Fails if* any mark keeps a screen size; `device_px` nodes must be listed in the sidecar.
