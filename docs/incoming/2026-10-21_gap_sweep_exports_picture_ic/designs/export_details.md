# Export details: colour windows, the viewer's settings, mark sizes

*Draft proposal against main @45caf8be.*

## 1. What exists today

Export & share offers an image at "multiples of the view", with pxpack and Styled/Plain (`principia_render_gui_spec.md`
§G5, :873-899); Record a time sweep sets range, frames, size, format (§G9, :1229-1233). `range_norm` uses a declared
`[lo, hi]` or, under `RANGE_AUTO`, the min/max "measured over the current draw" (§10.1, :1680-1691, :1739-1743). The
embedding record carries "THE RAMP WINDOW" in tier 2: "not optional" (`principia_dd_image_embedding.md` §6, :238,
:311). The display chain ends gamut clamp → lines drawn over → colour-vision simulation, which "models the viewer"
(gui spec :902-905, :1845; R-123, :935); the state tiers name no home for it (`principia_gui_state_contract.md` §2,
:46-56). Sizes follow three rules: `ScreenPx | PrintPt` (`principia_lines.md` §3 :88, :151; §5 :222), `ref_px` ×
`output_width/1024` and `device_px` (`principia_print_graph.md` §6.2, :244-256). Icons, arrows, label offsets and the
smart-ink radius are "px" with no rule (lines §3 :105-115, §6 :232).

## 2. The gap

- **(a)** A 4× export measures its window over different pixels than the screen; a recording re-measures each frame
  and flickers. Nothing says whether the window travels as numbers or as "auto".
- **(b)** Nothing says whether CVD or `render_scale` reach a file or screenshot; CVD is neither recorded nor excluded.
- **(c)** `PrintPt` needs a DPI no export has (REQ-RENDER-088 tests "the export's DPI"); `ScreenPx` and `ref_px`
  disagree unless the export width happens to be a multiple of the view.

## 3. Proposed design

### (a) Freeze the window

- **Export freezes every measured range**: each `u_range` in use is written to `RenderState.frozen_ranges
  { node_id → (lo, hi) }`; a node with one reads it instead of measuring. The frozen window is the one on screen when
  Export is pressed.
- **Clipping is stated**: the sidecar records per node the share of exported pixels clamped at each end.
- **Recordings use one window** frozen at Record, with clipped shares per frame. **Per frame** is an explicit choice,
  labelled "re-ranges every frame: frames not comparable", recorded as `range_policy=per_frame`.
- **On import** the node shows "frozen at export"; **Re-measure** clears it, one undo entry.

### (b) The viewer's settings stay with the viewer

- **CVD lives in `ViewUI`**, reaching the renderer as a GUI-set uniform, as the canvas background does (state
  contract §2, :111). Never undoable, never restored from a record.
- **Exports never include it** unless the user picks **"As seen with: [mode]"**, which records `cvd_simulated`,
  suffixes the file name and badges the header strip.
- **`render_scale` = 1 in every export**, as checkerboard is forced off (export contract Part 4, :76). A
  **screenshot** is the screen: its record names the `render_scale` and CVD mode it captured.

### (c) One rule: base px

- **Every mark length is in base px**: stroke, dash, outline, label size, offset, halo and spacing, icon and arrow
  size, tick length, smart-ink radius, and the Print graph's `ref_px` marks.
- **`RenderState.base_width`** is the figure's width in logical px (DPI scale removed) when saved or exported; a spec
  with no view (reel, headless) and older records take 1024.
- **Rendered length = length × `output_width / base_width`**: 1 on screen, k for a k× export, the same ratio for any
  size. Label placement runs at base size and is scaled, so a 4× export places the same labels.
- **`device_px`** stays the one listed exception.
- **Physical size** (mm, DPI) becomes an optional export field that sets `output_width` and writes PNG `pHYs`; the
  style editor reads widths in pt for it.

## 4. Fit with existing rulings

- Embedding §6 holds literally: the window travels as numbers. `range_norm` gains a third source, frozen.
- Export contract Part 4 gains "`render_scale` = 1; CVD off unless chosen".
- State contract §2 gains `ViewUI.cvd`, `RenderState.frozen_ranges` and `RenderState.base_width`.
- **Amends** REQ-RENDER-088 (`PrintPt`), lines §3/§5 units, and print_graph §6.2 / **R-490**: `ref_px` becomes base
  px with 1024 the default base, so on screen a medium keeps its size when the window is resized; R-490's
  export-size invariance still holds.
- Every image carries `range_policy`, clipped shares, `cvd_simulated`, `base_width`, and a screenshot's
  `render_scale`.

## 5. Open questions

1. **Which window does a recording freeze?** (i) the screen's at Record; (ii) a cheap low-resolution pre-pass over
   all frames, union of ranges; (iii) the final frame's. *Recommend (i)*, clipping reported; (ii) opt-in.
2. **Where does CVD live?** (i) `ViewUI` plus uniform; (ii) `RenderState`, travelling in records; (iii) an app
   preference. *Recommend (i)*, remembered across sessions.
3. **`PrintPt`**: (i) retire it, base px plus a pt readout; (ii) keep it as base px × 72/DPI, DPI required;
   (iii) keep both. *Recommend (i).*
4. **Print graph on screen**: (i) base px; (ii) keep `/1024` and scale lines the same way. *Recommend (i)*; (ii)
   shrinks labels in small windows.

## 6. Tests (each with what makes it fail)

- **frozen_range_roundtrip**: export, import, render: bitwise equal. *Fails if* import re-measures.
- **recording_one_window**: 10 frames, an auto node: equal `u_range` in every frame record. *Fails if* any differs.
  Control: `per_frame` gives at least two.
- **clipped_share_counts**: a field forced past the window reports `clipped_hi > 0`. *Fails if* it reads 0.
- **cvd_not_in_export**: CVD on gives the same pixels as off. *Fails if* any differ. Control: "As seen with" differs
  and is recorded.
- **render_scale_forced**: an export at screen `render_scale = 0.5` equals one at 1. *Fails if* not.
- **base_px_invariance**: 1× against 4× box-downsampled: every mark within tolerance, identical placed-label sets.
  *Fails if* any mark keeps a screen size; `device_px` nodes must be listed in the sidecar.


## Decided (Malachy, 21 Oct 2026)

All three accepted as proposed: a recording's colour range is frozen when Record is pressed (clipping reported);
colour-blindness simulation is never baked into an export unless asked, and is recorded when it is; line, label and
mark sizes scale with the export by one rule ("base px").
