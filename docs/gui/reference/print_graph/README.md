# Print graph: reference material

**Reference, not code to build on, and not normative.** The design is `docs/design/principia_print_graph.md`, adopted by
R-485 to R-492; this folder is what it was drawn from. In the human's words (20 Oct 2026, point 3 of the message R-485
quotes): "It is reference, not code to build on: port the lab's nodes and take the prototype's data format, as the
design doc §1 says." Cite the design doc and the rulings, never a file here (R-159's pattern for `docs/reference/`).
Nothing here is run by the build, CI or any test.

| Path | What it is | How the port uses it |
|---|---|---|
| `reference/print_graph_lab/index.html` | the lab, self-contained (the "Principia Print Graph" artifact): 78 node types in WebGL2/GLSL, 103 presets in 11 families, the slot chain. A `<meta charset>` was added to the delivered copy; without it its labels serve as mojibake | **which nodes exist and what they look like**: each node's GLSL is translated to WGSL (design doc §10, R-486) |
| `reference/print_graph_lab/heros-bold.otf` | the lab's display font, TeX Gyre Heros Bold; licence in `LICENSE-heros-bold.md` beside it | the lab's own page only |
| `reference/fx_prototype/` | the Python prototype: every medium as a JSON graph of small nodes (`presets/`), the engine (`graph.py`), the nodes (`nodes.py`, catalogued in `NODES.md`), `verify.py` | **the data format** (design doc §4, R-486): settings, `$setting` references, presets usable as nodes, explicit seeded streams. Its originals and scanned paper, which `verify.py` and some nodes read, are not in the bundle |
| `catalogue/node_catalogue.json` | the lab's node registry as data: 78 types (the source `image`, labelled Render, and `output` among them), category, label, doc, inputs, parameter ranges, pass count | **the porting checklist** (design doc §4.3) |
| `catalogue/presets.json`, `catalogue/preset_families.json` | every preset's graph in the lab's own form (nodes, params, wires), and the 11 families the gallery groups them into | the preset list and families; the port records them in the design doc §4's format |
| `images/nodes/`, `images/presets/` | 76 node renders (every type less `image` and `output`) and 103 preset renders, each a 512² JPEG at its defaults on one Principia render, each folder with an `index.json` | **the port's visual references** (design doc §10, R-492): each node and preset matched within the tolerance the first port task sets; a deviation the design requires is listed in the registry entry, never tuned away |
| `images/contact_sheet_nodes.jpg`, `images/contact_sheet_presets.jpg` | the same renders on one sheet each, by category | an overview |
| `images/source_config_stability.jpg` | the input every reference image was rendered from | the port renders the same graph on the same image |
| `images/lines_over_through.jpg` | the same contours drawn over three presets, and through them | R-487's two line passes |
| `tools/` | the Playwright scripts that made every image here (`capture_nodes.js`, `capture_presets.js`, `capture_slot_chain.js`) and exported the catalogue (`export_catalogue.js`) | regenerating the references as exact PNGs at any size, below |
| `boards/*.png`, `boards/source/` | the five design boards (`pipeline`, `slots`, `panel`, `degrade`, `bench`) and the HTML and images that draw them | the look of the design; the degrade board's reduced frames are downsamples of the full render, an illustration, not a run; the bench board shows layout only, no value in it is a measurement; where the boards differ from the rulings the spec follows the rulings (render_gui_spec §G13): the panel board's canvas corner badge is the figure's header strip (R-523), its "103 in 11 families" is 111 in 12 (R-532), and the boards' IBM Plex is not the dev GUI's Ubuntu and Ubuntu Mono (§G1) |

**Regenerating the reference images** (the bundle's own recipe, for a port task on the human's Mac; nothing in CI runs
it). From this folder, serve the lab and run the capture scripts with Chromium under SwiftShader
(`--use-gl=swiftshader`), which needs no GPU and gives the same pixels with the same Chromium:

```
cd reference/print_graph_lab && python3 -m http.server 8766 &
cd ../.. && PORT=8766 node tools/capture_nodes.js out/nodes      # every node at defaults
PORT=8766 node tools/capture_presets.js                          # every preset → presets/
PORT=8766 node tools/capture_slot_chain.js                       # the slots board's six frames
```

Lab nodes with extra inputs are wired automatically: `flow` ← a Flow node; `paper` ← Paper; `density`, `height`,
`mask`, `field` ← Brightness; `ground` ← Ground. That is why Strokes shows only its strokes.

**The lab's quirks are recorded, not inherited** (design doc §10): its screen-pixel units, frame-seeded noise and
stand-ins for payload fields are where the port follows the design doc instead (R-490, R-492; the stand-ins, R-536, ruling RQ-401).
