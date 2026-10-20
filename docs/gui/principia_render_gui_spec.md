# Principia — Dev GUI (egui / F3)

*Status: canonical. Rewritten in step 6 (24 Sep 2026). This is the single source of truth for the **developer GUI**: the
egui debug layer over the wgpu render, toggled with F3. It is not the final UI, but it has the same structure and every
feature. The production (TS) GUI is out of scope; it re-skins the same operations through the same contract
(`principia_gui_state_contract.md` §1, §7).*

*Sources, in order of authority: `decisions.md` rulings, then `gui/design/GUI_DESIGN_NOTES.md` (the reviewer's design
notes), then the twelve artboards in `gui/design/`. The notes win over the pictures, and a ruling wins over both. Where the
notes and the corpus disagreed, the entries (RQ-20 to RQ-24) are ruled by R-64 to R-68. §G13 lists where a picture is
overridden.*

*Building the dev GUI on the mock engine (R-390), the sources rank `decisions.md`, then the design notes, then the
artboards, then this spec, then `principia_gui_state_contract.md`; the higher wins where they disagree. R-68 still makes
corpus values win over artboard values, as §G13 lists; the order governs layout, look, feel and behaviour.*

*Part I covers the GUI as a whole. Part II is the stain editor, the node-graph editor over the composition algebra; it keeps
its section numbers (§0–§16). It conforms to `principia_colour_composition.md`: where the two overlap, the composition spec
is authoritative on semantics and this spec on interaction.*

**Vocabulary.** A **stain** is a composition in `principia_colour_composition.md`'s sense: the graph that colours the
data. The two words name one object. The **figure** is the rendered slice.

| artboard | screen | section |
|---|---|---|
| `01_main.png` | Explore, the everyday view | §G2 |
| `02_stain.png` | Stain, the node-graph editor | Part II |
| `03_chartbuilder.png` | Chart builder | §G7 |
| `04_windows.png` | Profiler, Export & share, Display, Run | §G5 |
| `05_inspectors.png` | Inspector: one IC, its trajectory, one timeline | §G8 |
| `06_legend.png` | Legend, generated from the stain | §G6 |
| `07_keyboard.png` | Keyboard (a design note, not a screen) | §G3 |
| `08_lock.png` | Lock: the reticle and the pin | §G4 |
| `09_importrecord.png` | Import picture, saved views, record a sweep | §G9 |
| `10_measure.png` | Measure, a tool on the figure | §G10 |
| `11_research.png` | Research, first pass (v2) | §G11 |
| `12_console.png` | Console | §G12 |

---

# Part I — The dev GUI

## G1. Rules that hold everywhere

- **egui is a toggleable debug layer (F3) over the wgpu render.** egui-wgpu shares the engine's `wgpu` context and paints
  onto the same surface (`principia_gui_state_contract.md` §1).
- **With the layer hidden, the figure fills the window (R-406), by one of two modes (R-468).** **Extend**, the
  default and R-406's fill: F3 hiding egui makes the figure fill the whole window, showing more of the field at the
  same scale in every direction, never the shown view stretched: each point of the field keeps the screen position it
  has in the shown layout, so the figure's rect there is unchanged, and nothing already computed is invalidated.
  **Fit**: the view's square, `UV ∈ [0,1]²` at the current `(z₀, q₁, q₂)`, is scaled uniformly to the window's shorter
  side and centred, never stretched, its extent ratio kept, and the rest of the window is the canvas background; the
  larger screen scale asks for finer quads (the screen floor, REQ-REF-011). Either way it is a view change, not an
  edit: hiding and showing send no `SetField` and add no undo entry; showing the layer returns the normal layout, in
  either mode, with nothing moved. The mode is chosen in View › **F3 fills by: Extend / Fit** (§G2's top bar) and kept
  for the session, each launch starting at Extend; present mode follows it, and an image export, a snapshot and a
  share link name it in their sidecar, `f3_fill=extend|fit` (R-472). Past the chart's `[0,1]²` each axis extends by its declared type
  (affine, periodic, pole-crossing or bounded, the default), and a pixel whose `Φ` fails or whose state fails
  validation is classified forbidden (R-407, `principia_chart_decoder_contract.md` Part 3), drawn as the canvas
  background in a presentation view and with the hatch in the debug view (R-467; next bullet). Extension is free for
  every affine chart, the latent chart and the flat and body-plane slices, tilted or rotated, planes onto which the
  square is only the current window; the types and the fallback matter for the charts whose square is their whole
  domain (the warp, the mass simplex, the Burrau family) and for custom physical-quantity axes (R-468, clause 4). The
  screen lane is the one exception to "nothing moves": `ctx.screen.uv` and `ctx.screen.pixel` are taken over the
  figure area as shown, the full window with the layer hidden in Extend and the scaled square in Fit, so a stain that
  reads them (the UV-view debug preset among them) draws differently over the shown rect when F3 toggles; in Extend the
  figure is identical over the shown rect for a stain that reads no screen-lane field (R-407, A4;
  `principia_coordinate_conventions_note.md`). *Was (R-406, R-407):* "a pixel whose `Φ` fails or whose state fails
  validation is hatched as forbidden"; "the full window with the layer hidden"; "the figure is identical over the shown
  rect"; one mode.
- **Presentation views draw no hatch (R-467).** The outcome view, the Chazy stains, every other stain that is not a
  debug view, the figure filling the window with F3 and present mode are presentation views. In them a pixel that is
  not part of the chart (past `[0,1]²` a state that fails validity or an axis beyond its bounded edge; inside the
  square a validity failure, a decoder bug, §G12) draws the plain canvas background, the colour the window is cleared
  to, with no hatch, legend key or share, as the area outside a map is blank; and NaN, `±∞`, `sim_failed` and
  `decode_failed` are never a category: they draw R-446's black `#141418`. The debug views show them, View › Debug view
  all three at once (§G2), and the Chart builder's Domain preview keeps its hatch and its "forbidden in view: N%", a
  diagnostic for building charts (§G7). **R-470:** such a pixel is blank, the view's background, in every view, never a
  palette colour, the outcome view included; the hatch outside the debug views is only the optional Overlays › Chart ›
  forbidden region, over those pixels alone; NaN, `±∞`, `sim_failed`, `decode_failed` and the classification behind
  the hatch show only in the debug views (whether a `decode_failed` sample the chart does not classify is blank instead
  of `#141418` is RQ-379).
- **A chart never leaves the manifold (R-469).** Every pixel is a valid three-body system or explicitly not one, and no
  chart, tilt, slice or extension integrates a state that is not a system. Navigation stops at the edge (§G2).
- **Contract first.** Every control reads a `Snapshot` and sends a typed `SetField`. Nothing touches simulation internals,
  and data flows one way: UI → `SetField` → core → snapshot → UI (gui_state_contract §1, §2). **Undo and redo live in the
  contract** as a history of typed `SetField` edits, shared by every GUI (R-52). A drag coalesces into one entry (R-96).
  Playback never enters undo: the GUI's clock advances the playhead through a `SetField` marked "no history" (R-101).
  The top bar shows the undo depth; Ctrl+Z undoes from the contract's history.
- **Navigation is chart construction.** There is no camera object: pan, zoom and slice edit `z₀` and the basis
  (`principia_chart_decoder_contract.md`, design axiom 6; canonical_spec §9, invariant 4). No control, window or log line is named
  after a camera.
- **Every preview of the slice keeps the viewport's aspect ratio** — square for the slice. No preview may stretch or crop it.
- **The figure is never covered.** No toasts or labels over the plot. Warnings go to the footer console (§G12). The hover
  label and the lock reticle are the figure's own marks, not notifications. The one control over the figure is the
  **Home** button in its top-right corner, by the human's words (§G2's Figure, R-466 K7).
- **Dark default egui theme, Ubuntu / Ubuntu Mono.** It's a dev tool; there is no styling beyond egui's own.
- **Not in the dev GUI:** Chazy subtitles, and the object-based stain desk. The Stain mode is the plain node-graph editor
  (Part II).
- **Sim key or not.** Each control knows whether its field re-integrates (sim key) or is live (render key), and warns per
  field from the caching blast-radius table (gui_state_contract §2).

## G2. Explore — the everyday view (`01_main.png`)

**Top bar.** Menus (File, View, Windows, Help; View holds, in order, **Home**, its first entry, R-466, a separator,
**F3 hide**, **F3 fills by: Extend / Fit**, one row of two radio options, R-468, **Debug view**, a check entry, R-467,
**Save view as chart**, R-475, and **reset layout**, R-462); the mode switch **Explore / Stain**; **Overlays ▾**, with a count of how
many are on; **Lines ▾** beside it, with a count, `Alt+L` showing or hiding all lines (R-476); **Run…**, **Profiler…**, **Export…**; the keyboard breadcrumb (§G3); status: `t`, fps, frame ms, quad count,
`budget-bound` when the frame budget binds, undo / redo depth, and "F3 hide".

**Overlays ▾** holds the toggles, grouped, each with an Alt+digit shortcut, plus **all off** and **save as default**:
- Quadtree: tile bounds, depth colouring, priority heatmap, `refine_flagged`, split / merge activity.
- Integration: substep density, substep-cap hits, NaN / invalid pixels, energy drift.
- Chart: forbidden region (the chart's domain function, R-26), Burrau lattice (primitive triples).
- Stain: class edges.

*Was (before R-476):* "- Chart: forbidden region (the chart's domain function, R-26), grid, Burrau lattice (primitive
triples)." and "- Stain: class edges, `t_end` contours." The grid and the `t_end` contours are presets of the line
system now, under Lines ▾ (R-476, L2).

Each entry is sorted by Part II §12.1's rule into a post node (Tier 1, derived from the address), a sim shader (Tier 2,
already resident) or a tile debug shader (Tier 3, a scheduler verdict). The Overlays menu and the Display window (§G5)
replace the corpus's global display bar (R-67). The Chart group's "forbidden region" and the Integration group's
"NaN / invalid pixels" are diagnostics the person turns on, off by default, beside View › Debug view (R-467, applied per
R-369); whether they stay beside it, go, or show only while it is on was RQ-374, ruled by R-470: **forbidden region**
stays, off by default, and draws the hatch over exactly the pixels the chart classifies as not a system (blank pixels)
and nothing else, in any view; **NaN / invalid pixels** draws only while a debug view is the figure (View › Debug view
on, or a debug-catalogue stain), dimmed with a tooltip saying so otherwise (RQ-374's option (c), applied per R-369).

**The debug view (R-467).** View › **Debug view** switches the figure, over whatever stain is active, to the one view
that shows what the presentation views leave out: every system pixel as the stain draws it; a pixel the chart
classifies as not a system with the hatch, `debug_invalid`; a decode or sim failure with `debug_invalid`; NaN and `±∞`
with a style of their own (REQ-COL-110). The legend gains a "Debug" key, its three entries "forbidden (chart)",
"decode or sim failure" and "NaN / Inf", each with its count of the figure's pixels and their share, drawn always
(§G6; colour_composition §6). It is a render-side toggle sent as a `SetField`, undoable as an Overlays ▾ toggle is
(R-69), re-integrating nothing (gui_state_contract §2; applied per R-369). *Was (R-467 D5 as first ported):* "marked
'no history': a view, not an edit, adding no undo entry". It is off at launch, keeps its state through
F3, present mode and both fill modes, and has no key of its own; the View menu reaches it from the keyboard (§G3). On
the mock its forbidden pixels are the region the test hook flags (`PRIN_MOCK_INVALID`, R-467 D8).

**Lines ▾ (R-476).** The line system of `docs/design/principia_lines.md`, a separate system, not a node of the stain
graph, drawn over the finished figure: contours of any expression over chart, latent and payload quantities, the hover,
pinned and kept traces, the axes (frame, ticks, labels, titles), grids and style presets. The menu lists the contour
sets, traces and axes with their visibility, opens the contour-set editor and the style library, and holds the presets,
among them **Grid** (contours of the chart's own axes) and **t_end** (`t_end` at the playhead), which were Overlays
toggles. Lines belong to the figure: they stay with F3 and present mode and are in exports; under F3's Extend they
continue wherever the field exists and never pass through a pixel that is not a system. Editing a line is one undoable
`SetField` that re-integrates nothing. Mock-ups: `docs/gui/reference/lines/`. Its six open questions are RQ-377.

**Left: "Manifold view" is ONE group.** Chart, navigation, centre `z₀`, slice and tilt, and rotation are one thing: how you
view the manifold.
- **Chart:** the preset, named by its axes (e.g. "z_α × z_β", never a nickname); the two basis vectors `q₁`, `q₂`, each with
  an edit button; **Chart builder…** (§G7); the chart's kind (affine or nonlinear). When the chart is the shape sphere,
  the section also shows the **projection selector** (equirectangular, or an equal-area alternative) and the **hemisphere
  toggle** (one hemisphere, or both with the redundancy flagged) — `principia_chart_reference.md` §3.3 (R-113); with one
  hemisphere drawn, φ's span runs from the pole to the equator (R-412).
  **Save view as chart** (R-475), a button after Chart builder… (and on View ›), commits the current view as a named
  custom chart: a latent view as the oblique chart `(z₀, q₁, q₂)`, a curved one as its parent chart, slice centre and
  tilt (map, angles, targets). It is one undo entry, makes the saved chart active (reproducing the view bitwise at zero
  tilt) and adds it to the preset selector's **Saved** group, named "Saved <n>", renamed inline, its provenance (parent
  chart, map, angles, targets, and a latent view's `q₁`, `q₂`) shown beside it; it is then tilted, sliced and saved like
  any chart, so tilts compose. Saved charts are the GUI's own settings, kept across sessions, deleted as a preset is
  (§G7). With an energy-type axis (`E`, `L_z`, `K`), the section offers each such axis's **mapping**, warped (the
  default, every pixel feasible) or linear (an affine axis, its infeasible pixels blank), one undoable edit that
  re-integrates (R-471).
- **Navigate:** centre `(u, v)`; zoom (log₂). With the depth readout, the precision warning and the lock row (the next
  two bullets) that is all it holds, with the reset beside its heading (R-466): no grid of `z₀` values, which repeated
  the Slice centre sliders (R-457). Its **reset** returns the centre `(u, v)` to the chart preset's centre and the zoom
  to the preset's window, which the zoom row reads as × 1.00 (R-466).
- **Depth readout** (`2^-k`, quad level) **and a precision warning**. The warning is **raised by events** (R-54):
  `DECODE_SWITCHOVER` firing on visible quads, and `AT_F32_FLOOR`. It is never tied to fixed depths. The artboard's readout
  is "f64 · 7 digits left · linearise decode".
- **Lock:** a "● locked at z_locked" badge with **unlock**. Sliders are **re-based to the anchor, not frozen** (§G4).
- **Slice centre** (named "Centre z₀" before R-457): eight sliders, `z_α, z_β, z_q0…z_q3, z_μ1, z_μ2`, each with its
  number box beside it. Moving the slice along its normal is the slider of the normal's dimension, the cube's D axis
  (R-457, R-459). Its **reset**, beside its heading, returns every out-of-plane coordinate of `z₀` to the chart preset's
  base point, the six coordinates off the preset's axes, tilted or not (R-466, K2). On every chart (R-473, T6) the
  sliders of the coordinates its axes do not set are its hidden coordinates and move the slice centre along them, each
  through its latent control (a mass through its logit), the physical value at the centre read out beside it (the
  masses on the shape-sphere chart, applied per R-369); the stop (below) holds.
- **Tilt** (named "Slice & tilt" before R-457): "tilt `τ₁` into [X]" and "tilt `τ₂` into [Y]", each a slider with its
  number box and a dropdown that picks the latent dimension the angle tilts into, and the rotation `γ`. `τ₁` tilts the
  horizontal axis `q₁` toward `e_X` and `τ₂` the vertical axis `q₂` toward `e_Y`: `u₁ = cos τ₁ e_a + sin τ₁ e_X`,
  `u₂ = cos τ₂ e_b + sin τ₂ e_Y`, then `γ` turns them as before (`View::basis`). A dropdown offers only the dimensions
  not in the plane (not `e_a`, `e_b`); both default to the slice normal, the cube's D axis, which is today's one shared
  target, so the defaults draw what the code drew before. Changing a target while its angle is non-zero re-tilts into
  the new dimension, one undoable edit. There is no step slider: it set `z₀`'s component on the slice axis, which the
  Slice centre slider for that dimension sets (R-457). Its **reset**, beside its heading, sets `τ₁`, `τ₂` and `γ` to 0;
  it changes neither tilt target nor the H, V and D choice (R-466).
  **The map (R-474):** "map: Tip as sheet / Lean each axis", two radio options after `γ`, Tip as sheet the default.
  With one target (`X = Y`) and both angles set, Tip as sheet turns the plane rigidly by `m = √(τ₁² + τ₂²)` about the
  in-plane axis perpendicular to `(τ₁, τ₂)`, orthonormal, and Lean each axis is the formula above; otherwise the two are
  the same. Under Tip as sheet each angle runs over ±180°, not clamped at 90°; under Lean each axis over ±90°. The map is
  `SimConfig`'s `tilt_map`, one undoable edit, and neither the Tilt reset nor Home changes it. **Every chart (R-473):**
  the section tilts every chart, curved ones included, the targets offering the chart's hidden coordinates, the
  rotation acting on the chart's own parameters (`principia_chart_decoder_contract.md` Part 4).

**Resets and Home (R-466).** "Reset" means the active chart preset's own values, never literal zero: the masses `z_μ1`,
`z_μ2` are slice coordinates, and zeroing them would change the physical system. Each reset reads its target from the
preset's definition, which carries, with its pair of axes, a base point, a centre `(u, v)` and a window (the scale at
which the zoom reads × 1.00), defined here and in §G7 by TASK-M6-45 (R-72, REQ-GUI-224); no value is hardcoded. The
active preset is the one Chart names, tilted or not; with a basis edited out of every preset's frame there is none, and
the resets and Home are dimmed. All three sections are read in the active preset's coordinates, its axes `e_a`, `e_b`
and the six dimensions off them, tilted or not (applied per R-369, R-466 K2): Slice centre's values are `z₀`'s six
coordinates off `e_a` and `e_b`; Navigate's are `z₀`'s two coordinates on them and the zoom `σ`; Tilt's are `τ₁`, `τ₂`,
`γ`. The Slice centre reset sets the six to the base point's; the Navigate reset sets the two to the preset's centre
and the zoom to the window, the angles kept; the Tilt reset edits the basis only, turning the plane about the view's
centre `z₀`, the zoom kept. Untilted, these are the chart decoder contract's split (Part 4): a move across the plane, a
pan within it and a zoom, and a basis edit. Each reset sets only its own values, so it leaves the other two sections
untouched at every tilt, and the three commute: in any order they compose to Home. Navigate's centre row reads `z₀` in
the current plane, along `q̂₁`, `q̂₂`, the coordinates its pan edits; untilted they are the section's values, and tilted
the dimming and the checks compare the section's values, not the row's figures. **Home** is all three resets in
one command: the button in the figure's top-right corner (below), the Home key in both keyboard modes (§G3) and
**View › Home**. It leaves alone the chart preset and its axes (Chart's `q₁` and `q₂` axes, the cube's H and V), the H,
V and D choice and the tilt targets, the time playhead, the stain, and the panel and window layout, and also the lock,
the compass's mode and view, the overlays and the Trajectory panel's pin and kept orbits (R-466 K4). Each reset, and
Home, is **one undoable command**: its edits are sent in one frame under one history key, so one Ctrl+Z restores all it
set, and a command with nothing to change sends nothing. A section's reset is **dimmed** (egui's disabled style, no
click) while the section is already at its home values, within 10⁻¹², and Home, its button and its menu entry while all
three are. A reset drives the compass, its readout, the sliders and the figure live, as any change to the slice does
(R-458), and switches no compass mode. Locked, the edits go through the lock's rule for their kind (§G4), and the lock
and its anchor stay (R-466 K5): a locked Tilt reset, or a locked Navigate reset's zoom, turns about the pin and so may
move `z₀`, so the checks that a reset leaves the other two sections untouched run unlocked. The commands are named
"Reset Slice centre", "Reset Tilt", "Reset Navigate" and "Home" (tooltips, the menu and the `?` overlay).

**Navigation stops at the edge (R-469).** A pan, slice, zoom, tilt or lock that would take the chart's centre out of
`validate`'s layers 2 and 3 stops at the last valid value: a dragged slider's handle stops there and goes no further,
the drag staying live up to it, the compass shows the clamped plane, and the compass's readout adds "stopped at the
edge: <check> (validate layer <n>)" until the next edit. Undo and the contract hold the clamped value.

**Figure.** Axis labels carry the short axis name and the range at each end. *(R-476: the axes are the line system's,
its Screen preset the dev default; this line is its Ends only preset, offered.)* A **Home** button, a house icon 24 × 24
points, sits in the canvas's top-right corner, inset 8 points (R-466; its look applied per R-369, K7): a click on it is
Home, never a click on the figure (it pins nothing and starts no pan); it is dimmed while all three sections are home,
and it hides with the layer (F3, present mode). The orbit's path is drawn from the cursor's IC:
- **The path is `z(t)` through the manifold, projected onto this chart's `q₁`, `q₂`** — not real space. It is drawn by
  the line system (R-476, `principia_lines.md` §8), as the pinned and kept orbits are. This is
  `principia_trajectory_viewing.md` §3's hover trace (the full CPU state, `computeIC`, projected onto the chart's actual
  basis). It is **solid near the slice and faint where it leaves the plane.** The notes add that it is exact in the
  shape-sphere `(θ, φ)` chart, where the chart's coordinates are the shape itself.
- **The hover label names a fate only once it's decided.** Before that it reads "undecided · still interacting at `t` of
  `t_max`". A decided fate is a terminal `state` (escape, collision) or `bounded` at the horizon (payload §2); its labels
  name the bodies 1, 2, 3 and a pair by its two bodies, "collision 1–2", never a pair id, from the one display function
  (R-449; payload §2, "What a person reads").

**Right: the Trajectory panel,** for the IC under the cursor or a kept orbit (tabs: "under cursor", `#1`, `#2`, `+`):
- **a click on the figure pins that sample's trajectory here (R-453).** A primary click with no drag pins the sample
  under it (a drag still pans); the pin holds the sample's IC, so a later pan, zoom or slice edit leaves it, and a
  click on another point moves it. Hover never replaces a pinned trajectory, and **the cursor moving into the panel
  never clears it**; without a pin, the panel keeps the last sample hovered over the figure until the pointer hovers
  another point. An **unpin** button returns the first tab to following the cursor. The path drawn on the figure and
  the hover label keep following the pointer, pinned or not (the Figure, above); the pinned orbit is the panel's. The
  pin, like the kept orbits, is `ViewUI` state, never undoable; the first tab reads "pinned" while a pin is held
  (R-453, D1 to D3, D5);
- a one-line summary (fate, `t_end`, FTLE, `E₀`);
- real space (CoM frame) and the shape sphere side by side. **The shape sphere turns slowly, with visible x, y, z axes,** and
  switches to an unwrapped (equirectangular) view; a "turn" checkbox stops it. **Both views are interactive (R-453):**
  on real space (and the unwrapped view) a drag pans and the wheel zooms about the pointer; on the sphere a drag orbits
  the view (yaw, and pitch clamped to ±90°) and the wheel zooms, a drag turning "turn" off; a double-click restores a
  view's default. The orbit turns the view of the sphere, never the data (trajectory_viewing §4), and is the panel's
  own `ViewUI` state; it is not the compass's, which has none (R-409);
- the F₂ word, substeps, minimum separation, `|ΔE/E|`;
- a playhead for this orbit;
- **listen**: sonification (`principia_scratchpad_pointer_channels.md` §4, normative with §3 — R-109, R-144; `θ(t), φ(t)` → spectrum), which can follow the
  cursor. The mapping is the corpus's; the artboard's selector ("separations → pitch") is illustrative (R-68);
- **Open full viewer…** and **IC Inspector…** (both open §G8);
- **Kept orbits** below, each with its fate and time, removable.

**Bottom.**
- **Compass** (the nav cube), bottom left under the view controls. It shows the slice plane inside the chart, in a fixed
  orthographic view at its default angle: it has **no orbit** (R-409). It has two modes, **Tilt** and **Slice**, with a
  button for each beside it; Tilt is the default, and the buttons always show the current mode. A click on a button
  switches the mode, and the compass still **switches mode by itself** too: touching a slice slider switches it to
  Slice, and touching a tilt to Tilt (R-409). In tilt mode, dragging on it tilts the plane; in slice mode, dragging
  anywhere on it moves the slice plane along its normal, the tilt fixed (R-409). When locked it carries a gold pin at the
  pivot, and the plane turns about the pin (§G4). It reads out the tilt and rotation angles. From the keyboard it is one
  element, beside its Tilt and Slice buttons (§G3). It is drawn as `docs/gui/reference/principia_nav_cube.html` draws
  its cube, isometric, the slice its true cross-section polygon (R-451), and it has four **views**, Cube (the default),
  Plücker, Jacobi and Shape, chosen on the compass and independent of its Tilt and Slice modes (R-452); the next
  subsection gives both. Its drags are live in both directions: a drag on it moves the sliders, its readout and the
  figure every frame, and a drag on a Slice centre or Tilt slider switches its mode and moves its plane every frame,
  each drag one undo entry (R-458). Its cube's H, V and D axis labels are buttons that choose the slice's dimensions
  (R-459). A section reset or Home drives it live too, the cube easing to the new plane as after a click, its mode
  unchanged (R-466).
- **Time:** play, step, a scrubber, speed. Transport (play / pause / speed / loop) is `ViewUI` state: not undoable, not
  on the sim key (R-96). The GUI's clock advances `RenderState`'s playhead each frame through a `SetField` marked "no
  history"; a manual scrub is one coalesced undo entry (R-101). **Scrubbing back re-integrates** to that time, so the figure refines
  progressively. It is not instant, and it says so ("refining · 72%"). The scrubber sets the display time and never replays
  stored frames; the export contract's "no scrub" applies to exported animations only (R-66). While the scrubber is
  dragged the display time follows the drag every frame, the whole drag one undo entry (R-458).
- **Legend, generated from the stain** (§G6).

**Footer.** Warning and error counts, the latest message, memory (GPU, heap), "? keys", and the passive-logging
indicator while logging is on (R-129). Clicking it opens the console
(§G12).

**Run settings are NOT on the page** (they're rarely changed). Horizon, integrator, escape settings, quality, budget, and
recompute / cancel live in the Run window (§G5).

**Every drag is live (R-458).** Dragging any slider (or the number box beside it, or the compass) applies its value
every frame while the pointer moves, at most once a frame, pointer events coalesced, so the figure, the other controls
showing the value and the compass follow the drag, not the release; the whole drag is one undo entry (R-96) and one
console line. A sim-key field re-integrates progressively as it is dragged, as a scrub back does.

**Layout: every panel boundary is a splitter (R-462).** The Manifold view, the Trajectory panel, the Compass, Time and
Legend each meet their neighbours at a draggable splitter, and the figure's canvas takes the space that remains. Each
panel has a minimum size; a double-click on a splitter resets that splitter to its default; the layout is remembered
across sessions, and **View › reset layout** returns every splitter to its default, which is `01_main.png`'s layout.
Resizing the canvas re-frames the figure by R-406's rule (each point of the field keeps its screen position, the
canvas showing more or less of the field at the same scale) and sends no `SetField`; it never re-integrates the view,
only computing what it newly exposes.

### The compass: look and views (R-451, R-452)

*References for look and behaviour, transcribed, not copied (§G13): `docs/gui/reference/principia_nav_cube.html`
("Principia — navigation cube"), `docs/gui/reference/principia_plucker_compass.html` ("Principia — orientation
compass") and `docs/gui/reference/principia_compass_charts.html` ("Principia — compass, position & chart transform").
Their light cream palette is copied nowhere: every colour is the dev GUI's dark theme (§G1).*

**The Cube view (R-451).** The cube `[−1,1]³` in the frame of the preset's axes, Width = chart x (`e_a`), Height = chart
y (`e_b`), Depth = the slice axis (`e_d`), drawn as the reference draws it:
- **Projection:** isometric, `P(x,y,z) = (CX + S·√3/2·(x − z), CY + S·(½(x + z) − y))`, `S = min(0.78·w/(2√3),
  0.78·h/4)`, centred.
- **Edges** depth-sorted by `x + z`, nearer ones more opaque and thicker (opacity `0.28 + 0.55·near`, width
  `0.8 + 1.0·near` px, `near = (x + z + 2)/4` at the edge's middle); the slice is sorted among them by its own mean
  `x + z`, so edges in front of it draw over it.
- **The slice** is the true cross-section polygon of the plane through the cube, 3 to 6 vertices: a rectangle
  untilted, rectangles to hexagons as it tilts. Through the cube's central axis (the plane through a point `(0, 0, d)`,
  `|d| < 1`) it has 4 to 6, never a triangle, a pentagon possible off the centre; off that axis a plane can cut off one
  corner, a triangle, and at `|d| = 1` a tilted plane can be one (at `τ₁ = τ₂ = −45°` the plane `x + y + z = 1` through
  `(0, 0, 1)`), which the 3-to-6 bound covers (R-451 B2). Its fill is Lambert-shaded from the slice's own normal,
  two-sided (`0.2 + 0.8·max(0, n·L)`, `n` flipped to face the view `(1,1,1)`, light `(0.35, 1.0, 0.55)` normalised, a
  depth term `±0.05`), at opacity 0.6; an in-plane grid (spacing 0.5) is clipped to the polygon; an outline (1.7 px)
  closes it.
  Both modes draw the decoder's plane: its normal `n` is that of `View::basis`'s `(q₁, q₂)` in the cube's frame, and
  its offset is `n·p`, `p` the point whose coordinates are `z₀`'s components on `e_a`, `e_b` and `e_d`, each over the
  slice step's half-range, unclamped, so a pan, which moves `z₀` within the plane, leaves the drawing unchanged. Only
  the offset is bounded, clamped to `±|n|₁`; at the bound the plane touches the cube, and a face is drawn as the
  polygon, an edge or a corner as the outline alone. The modes differ in the drag, the labels and the readout, never
  in the plane drawn, and untilted the plane is the reference's slice-mode rectangle at its depth (R-451 B2). The three
  axes share one scale, so the drawn tilt is the latent angle; the cube is [−1, 1] on each axis in units of the
  half-range, absolute, free or locked (free, the sliders' range; locked, the sliders span the anchor ± the
  half-range), and the chart window's extent is not drawn. The drawing follows the decoder's tilt mapping, not the
  reference's axis-angle map; RQ-340 holds the difference (R-451). *R-474 (ruling RQ-340):* it draws the active map,
  `View::basis` under `tilt_map`, Tip as sheet (the reference's axis-angle map) by default.
  All of this holds where the compass draws the plane, tested by `s = 2|q₁ × q₂| / (|q₁|² + |q₂|²)`, `q₁`, `q₂` in
  the cube's frame, `s ≥ 10⁻³` (under the decoder's map `s = √(1 − sin²τ₁ sin²τ₂)` for every `γ` and `σ`). The cut
  is the compass's own, numerical, set by the 1e-12 normal check (the normal's error is about `ε/s`): only at the
  four corner points `τ₁ = ±τ₂ = ±90°`, where `q₁` and `q₂` both lie along `e_d`, is there no plane, and within
  about 0.057° of them a plane exists that the compass does not draw. Where `s < 10⁻³` no slice is drawn (no polygon,
  fill, grid or outline), only the cube's edges, labels and corner numbers, and, locked, the gold pin at the anchor
  (R-451 B7); the readout says so and a slice-mode drag sends no edit (R-451 B11; RQ-340).
  *R-474:* B11 holds under Lean each axis only. Under Tip as sheet `q̂₁`, `q̂₂` are orthonormal at every tilt, so
  `s = 1` and the plane is always drawn, at the corners `τ₁ = ±τ₂ = ±90°` (`m = 127.3°`) too.
- **Labels:** each axis name centred on its edge, pushed 20 px outward from the cube. Slice mode names Width
  (bottom-left edge), Depth (bottom-right) and Height (left vertical), with −1.0 and 1.0 at the corners, one number per
  corner, 12 px outward. Tilt mode names only the tilt target, on the bottom-right edge: under the decoder's mapping q₁
  and q₂ share one target, the slice axis, and the other edges are left unnamed (R-451 B1). From R-459 the three
  edges' labels are the H, V and D buttons below, in both modes, and the third axis names what the gesture tilts into.
- **Readout:** slice mode, "plane ⟂ <axis>" untilted, or the tilt as tilt mode reads it when a tilt is set, and
  "<axis> = +0.25 of range" (`z₀` on the slice axis over the slice step's half-range); tilt mode, q₁'s and q₂'s target
  and their angles in degrees, one decimal. Both tilt targets are always named, each with its angle ("τ₁ → z_q3
  +12.0°", "τ₂ → z_μ1 0.0°"), whichever the third axis shows (R-457, R-459).
  Where `s < 10⁻³` (above), either mode adds the line "no plane drawn: s < 10⁻³ at τ₁ = <τ₁>°, τ₂ = <τ₂>°
  (RQ-340)", the angles signed, one decimal (R-451 B11). *R-474:* that line, under Lean each axis only, ends
  "(Lean each axis)". **The readout's lines under R-474**, in either mode: each screen axis's components, "H = 0.72 e_a −
  0.28 e_b + 0.63 e_X" and "V = …", the unit axes after `γ`, two decimals, terms below 0.005 left out (R9); past `m =
  90°` under Tip as sheet, "same plane as (τ₁′, τ₂′), mirrored", `τ′ = τ·(m − 180)/m` (R4); under Lean each axis,
  "dead zone: |τ₁| = 90°, τ₂ does not move the plane" (or with the indices swapped) while an angle reads ±90.0°, and
  "axes at <θ>°", `cos θ = sin τ₁ sin τ₂`, whenever it does not read 90.0° (R8); and R-469's "stopped at the edge" line.
- **Drag:** slice mode, the drag moves the slice along the drawn plane's normal, the decoder's (R-409 A3's), by its
  2-D displacement projected onto that normal's direction on the screen (`ŝ`, along `P(n̂) − P(0)`), so the drawn
  plane moves with the pointer at every tilt, the component across `ŝ` doing nothing, a drag of the cube's full width
  along `ŝ`, `2√3·S`, spanning the whole range (R-465). Untilted, `ŝ` points to the lower left, so a drag to the lower left moves
  the plane toward `+e_d` and a drag to the right moves it back, up and right. Where the drawn unit normal's screen
  image is shorter than `0.25·S` (the normal near the view axis `(1, 1, 1)`, as at `τ₁ = τ₂ = −45°`), the drag reads
  its vertical component instead, up moving the plane toward the viewer, at the same rate, the whole range per `2√3·S`
  of vertical travel (R-465 N3). Tilt mode, unchanged by R-465: a
  horizontal drag sets τ₁ and a vertical one τ₂ (up raising it), the full width
  or height 180°, each clamped to ±90° (under Lean each axis; under Tip as sheet to ±180°, nothing clamping the tip at
  90°, R-474). One undo entry per drag (R-96), the edit applied every frame of the drag so the
  sliders, the readout and the figure follow it (R-458). The displayed values ease toward their targets,
  `v ← v + (target − v)·(1 − 0.0016^dt)`, `dt` capped at 0.05 s, in the drawing only; the contract holds the target
  (R-451 B3, B4), so during a drag only the cube's drawing trails the pointer (R-465 N1). Slice mode and tilt mode
  stay two separate modes (R-465).
  Where `s < 10⁻³`, a slice-mode drag sends no edit, the compass drawing no normal to move along; a tilt-mode drag
  acts as anywhere else, free or locked: off the corner points a plane exists there, and locked the drag turns about
  the pin (§G4), its pivot and the figure's samples kept. The GUI's guards for a degenerate basis have their own cut,
  not the compass's: they decline where their sine test fires or `s < 10⁻⁶` on the plane's own basis (the corner
  points, and `s`'s own rounding floor), and there no `z₀` edit is sent and the figure shows nothing new (R-451 B11;
  REQ-GUI-203).
- Kept from R-409: no orbit, the Tilt and Slice buttons and the switch by the slider touched, Tilt the default, the
  gold pin when locked, §G3's keys. The reference's own controls (axis selectors, sliders, Animate) are not on the
  compass; the Manifold view owns them.
- **Live, both ways (R-458).** A drag on the cube applies its edit every frame, at most once a frame, so the Manifold
  view's sliders, the readout and the figure follow the pointer during the drag; the whole drag is one undo entry.
  The other way, a drag on a Slice centre slider switches the compass to Slice and a drag on a Tilt slider (`τ₁`,
  `τ₂`, `γ`) to Tilt, from the drag's first frame, and the drawn plane moves or tilts with the drag every frame (a
  Slice centre slider moves the plane where its dimension is one the cube draws, by B2's rule; a pan along `e_a` or
  `e_b` of an untilted plane, or a move along a dimension the cube does not draw, leaves the drawing where it is).
- **Axis buttons (R-459).** The cube's three edge labels are buttons: **H** (the bottom-left edge, chart x, the same as
  Chart's "horizontal q₁"), **V** (the left vertical edge, chart y, Chart's "vertical q₂") and **D** (the bottom-right
  edge, the slice normal, the dimension the slice moves through). Each reads its letter and its dimension
  ("H z_α") and opens a dropdown of the latent dimensions not on the other two, so no dimension appears twice across
  H, V and D. Picking H or V sets the plane to the new axis pair, the tilts and `γ` reset to 0, the zoom and `z₀` kept
  (locked, about the pin), as the preset does, and Chart's rows follow; an edit in Chart moves H and V with it.
  Picking D sets the slice normal; a tilt target that was the old D follows it. Each pick is one undoable edit. The
  slice normal and the two tilt targets are `SimConfig`'s slice frame (gui_state_contract §2), read by no kernel.
- **The third axis (R-459).** In Slice mode the bottom-right edge shows D, the slice normal. In Tilt mode it shows
  what the gesture tilts into: X while `τ₁` is being dragged and Y while `τ₂` is; when nothing is being dragged, X if
  `τ₁ ≠ 0`, else Y if `τ₂ ≠ 0`, else the slice normal. A compass drag that moves both angles shows the one it has
  moved further since it began, `τ₁` on a tie. The label takes the active gesture's colour (slice, `τ₁` or `τ₂`). The
  cube draws the plane of that gesture exactly: `View::basis` with the other angle at 0, in the frame `(e_a, e_b,
  e_T)`, `T` the dimension the third axis shows. While the third axis shows X or Y it is that target's dropdown, the
  same choice as the Tilt section's. When both angles are non-zero and X ≠ Y the plane leaves any one cube, and the
  cube shows a badge, **"4D: see Plücker"**; a click on it switches the compass to the Plücker view, which is exact
  (R-452). Both targets are always named in the readout.

**The views (R-452).** Four buttons on the compass, **Cube**, **Plücker**, **Jacobi** and **Shape**, Cube the default,
the current one shown. A view sets what the compass draws; Tilt and Slice set what a drag on the Cube does; the two are
independent, and in a grid view a drag does nothing. The view is the compass's own GUI state, not undoable.
- **Plücker (the latent heading).** The slice plane `span(q₁, q₂)` in the eight latent coordinates, from the `q₁`, `q₂`
  the decoder uses, orthonormalised, never from a reconstructed tilt: `p_ij = q₁ᵢ q₂ⱼ − q₁ⱼ q₂ᵢ` (i < j) as the upper
  triangle of an 8 × 8 grid, labelled with the chart's coordinate names on both edges, the diagonal empty. `Σ p² = 1`,
  and the grid is unchanged by any rotation of `q₁`, `q₂` within the plane. A cell's colour is its sign in one of two
  dark-mode hues, its intensity `|p|`, zero the cell background; no number in a cell, its value on hover ("z_i × z_j
  +0.123", a tooltip beside the compass). The dominant cell is outlined (when `|p| > 0.02`) and the readout gives it
  and `Σ p²`: "strongest: z_a × z_b +0.993 · Σ p² 1.000". It shows heading, not position.
- **Position readout,** under the Plücker view: `z₀` as one signed bar per latent coordinate, the slice axis
  highlighted (`principia_compass_charts.html`, "position gauge"). A readout, not a view.
- **Jacobi and Shape (the decoded heading).** The plane pushed through the decoder's Jacobian at `z₀`, or at the
  hovered pixel while the pointer is over the figure: `u₁ = J q₁`, `u₂ = J q₂`, orthonormalised, drawn as the same
  grid over eight physical coordinates; the readout gives the dominant pair and the area stretch
  `√(|J q₁|²|J q₂|² − (J q₁ · J q₂)²)`. Jacobi's eight are `ρ̃ₓ, ρ̃ᵧ, λ̃ₓ, λ̃ᵧ, p_ρₓ, p_ρᵧ, p_λₓ, p_λᵧ`, as the decoder
  writes them; Shape's `n₁, n₂, n₃, R, ṅ₁, ṅ₂, ṅ₃, Ṙ`. Both sets, and which Jacobian (the corpus's `J_D` is
  `∂physical/∂(u, v)`, `principia_deep_zoom.md`), are under physics review and wait on RQ-341 to RQ-343, RQ-345 and
  RQ-346; the
  reference's Jacobian is a synthetic placeholder and is not copied. The help text says that these views change as the
  slice moves, because the Jacobian depends on `z₀`, while the Plücker view does not.
- **Click to steer.** A click on Plücker cell (i, j) sets the slice plane to `z_i × z_j`, the tilts and `γ` reset to 0,
  the zoom and `z₀` kept. A click on a physical cell (i, j) sets it to the latent preimage at `z₀`, `q₁ ∝ J⁺ e_i`,
  `q₂ ∝ J⁺ e_j`, Gram–Schmidt orthonormalised, and is refused, with the reason logged to the console, where the
  preimage is no 2-plane; the construction waits on RQ-344 to RQ-346. Every steer is one `SetField` on the basis through the
  contract: undone by one Ctrl+Z, logged in the console, the Manifold view's sliders following.
- **Keys:** the Compass scope's siblings are the compass, Tilt, Slice, Cube, Plücker, Jacobi and Shape (§G3).

## G3. Keyboard — a design note, not a screen (`07_keyboard.png`)

The GUI is a tree of scopes. The big scopes, in Tab order: 1 top bar · 2 Manifold view · 3 Figure · 4 Trajectory ·
5 Compass · 6 Time · 7 Legend · 8 footer (R-405). Manifold view's sub-scopes (Chart, Navigate, Slice centre, Tilt; named Centre z₀ and Slice & tilt before R-457) are reached with Enter.
The reset of Navigate, Slice centre and Tilt is its sub-scope's last element, so Enter into a sub-scope still lands on
its first; Enter on a reset acts as its click, and on a dimmed one does nothing, the order unchanged (R-466). The Home
button over the figure is not an element; the Home key does its work.

| key | action |
|---|---|
| Tab / Shift+Tab | next / previous big scope, in the numbered order, landing on the scope itself in navigation mode, from either mode; inside a window, its next / previous section, in navigation mode, staying in the window (R-409) |
| Enter | in navigation mode: on a scope, into it, on its first element, but on the Figure, which has nothing inside, start interaction, and on a read-only scope with nothing inside (the Legend), nothing; on an element, start interaction; on a button, its click. In interaction mode: back to navigation mode on the same element (R-409) |
| Esc | in interaction mode, back to navigation mode on the same element; in navigation mode, up one scope (R-409) |
| arrows | in navigation mode, always between siblings, and on a big scope itself between the big scopes in Tab order; in interaction mode, adjust the focused element (R-409) |
| Shift · Alt | ×10 · ×0.1 steps, everywhere, the compass included (R-409) |
| letters, Space | a scope's shortcuts (below), whenever the focus is in that scope, in either mode, except while a text field is being typed in (R-409) |
| held keys | delay, then repeat (the DAS / ARR model) |
| Ctrl+Z | undo, from the contract's history (R-52), in either mode, except while a text field is being typed in, when it undoes the text edit (R-409) |
| ? | shortcuts, over everything, in either mode, except while a text field is being typed in, when it types a character (R-409) |
| Home | Home: Slice centre, Tilt and Navigate reset to the chart preset's values, one undoable command, wherever the focus is, in either mode, except on the console's log in interaction mode (its first entry, R-450) and while a text field is being typed in (the caret to the line's start); Shift and Alt change nothing, Ctrl and Cmd make it no Home; it does not repeat (R-466) |

In scope: a scope's arrow actions need Enter, acting in interaction mode only, and its letter and Space keys are
shortcuts, acting whenever the focus is in the scope, on it or on an element inside it, in either mode, except while a
text field is being typed in (R-409).
Figure — arrows pan; + / − zoom, Space keeps the orbit, L listens, K locks (§G4), P pins the sample under the pointer
in the Trajectory panel (R-453). Trajectory — Enter reaches the
playhead, listen, kept orbits, unpin and its two views; on the real-space view in interaction mode the arrows pan by
R-404's `Pan` step and + / − zoom by its `ZoomLog2` step, and on the sphere the arrows orbit by its `Angle` step (1°)
and + / − zoom; the unwrapped view, while shown in the sphere's place, is that element and takes the real-space view's
keys (R-453, D6). Compass — in interaction mode, arrows tilt in tilt mode and move the slice along the
plane's normal in slice mode, Shift ×10 and Alt ×0.1 as everywhere (R-409; there is no orbit, and no separate fine
step). Time — Space plays; ← → step, in interaction mode on its scrubber (applied per R-369, R-409 F1). Legend is
read-only.

What the user sees, while keyboard navigation is on (R-460): a focus ring on the current scope, orange in navigation mode and blue in interaction mode (R-409),
and the breadcrumb in the top bar (e.g. "Manifold view › Navigate › zoom"). Nothing else changes on screen.

**Keyboard navigation is on or off, apart from the focus (R-460, R-464).** The focus (where keys go) and keyboard navigation
(the ring, and R-409's two modes below) are separate. Keyboard navigation starts only on keyboard input: any key press
turns it on and shows the ring where the focus is once the key has acted, a modifier pressed alone excepted (R-460,
R-464; there is no navigation shortcut today). Tab, Shift+Tab and the arrows act as always, from the focus where it
is; Esc, the letter and Space shortcuts, Ctrl+Z and `?` act as always; in a text field being typed in every key acts as
typing does; Enter in navigation mode, while it is off, only turns it on and shows the ring where the focus is, so
nothing unseen is pressed (applied per R-369, R-460 H1, R-464 M2). Opening a
window with the mouse (a click on the footer, a menu entry, a top-bar button or a panel's button) gives the window the
focus, on its landing as below, but shows no ring and leaves keyboard navigation off, whatever it was; opening one from
the keyboard turns it on, landing as below with the ring. A menu opened with the mouse takes the focus the same way,
with no ring. After any mouse click or drag, whatever was clicked or dragged, clicking off to close something
included, keyboard navigation is off and the ring hidden; the click moves the focus only as the click rules below say
(R-450 A3, R-456), and the next key press shows the ring again where the focus already is (R-464, amending R-450 A3 and
R-456 only as to whether the ring is drawn). A scroll or a hover is neither a click nor a drag and leaves the ring as
it is (applied per R-369, R-464 M3). The GUI starts with it off. It is the GUI's own state, beside the focus, never
undoable and never read by the engine.

**Two modes: navigation and interaction (R-409).** While keyboard navigation is on (R-460), the keyboard is always in one of two modes. In **navigation mode**
the ring is orange and the arrows move between siblings; on a big scope itself, the big scopes are the siblings, and
the arrows move between them in Tab order. In **interaction mode** the ring is blue and the focused element takes the
arrows, with Shift ×10 and Alt ×0.1; Tab and Shift+Tab, Ctrl+Z, `?` and the letter and Space shortcuts keep their
meaning in both modes, except while a text field is being typed in, and Tab or Shift+Tab from interaction mode lands
in navigation mode on the next or previous big scope, or, inside a window, on the window's next or previous section,
staying in the window (applied per R-369, R-409 G1, G2, G5, G6). Every landing is in navigation mode: a Tab on a big scope lands on the scope
itself, the whole scope highlighted, the Figure and the Compass included, as REQ-GUI-095 has it; Enter into a scope
lands on its first element; and a window opens on its first section, as the console opens on its header bar scope (R-450);
opened with the mouse it takes the focus there with no ring (R-460).
The landings above are the keyboard's; a click on a text field is not one (R-456). Enter
on an element starts interaction with it, and Enter on a scope goes in, with two exceptions: the Figure, which has no
elements inside, takes Enter as the start of interaction, and a read-only scope with nothing inside, such as the
Legend, takes Enter as nothing (R-409 A2, G3). Esc or Enter in interaction mode returns to navigation mode on the same
element, and Esc in navigation mode goes up one scope (R-409, G1). Enter on a button acts as its click and stays in
navigation mode (applied per R-369, R-409 A1). The blue is egui's dark theme's `hyperlink_color` (R-404); the orange
is a look choice (R-390). The Compass scope holds the compass, then its Tilt and Slice buttons, then its view buttons
Cube, Plücker, Jacobi and Shape, then the cube's axis buttons H, V and D and, while it is shown, the "4D: see Plücker"
badge (R-459; Enter on an axis button opens its dropdown, ↑ ↓ choose, Enter picks and Esc closes it unchanged; Enter on
the badge acts as its click), in that order, as siblings in navigation mode (R-409, the human's follow-up answer;
A2; R-452): Tab lands on the Compass scope, Enter goes in onto the compass, → moves to Tilt, Slice and on through the
views, Enter on the compass starts interaction with it, Enter on Tilt or Slice sets the mode, and Enter on a view
button switches the view. In the Cube view interaction is as above; in a grid view (Plücker, Jacobi, Shape) the arrows
move a highlighted cell over the grid's upper triangle, Space steers to it as a click does, and Esc or Enter returns to
navigation mode (R-452). A text field, such as the console's search field, lands in navigation mode (a keyboard
landing; a click on it is not one, R-456); Enter on it starts
typing, which is interaction mode, and while typing every key goes to the field but Esc, Enter and Tab, Ctrl+Z
undoing the text edit and `?` typing a character: Esc and Enter return to navigation mode on the field and keep the
text, and Tab and Shift+Tab leave it and move on, to the window's next or previous section inside a window (applied
per R-369, R-409 G4, G5, G6). A click on a text field starts typing at once, in interaction mode, the ring blue, the
caret where the click lands (the ring hidden from the click until the next key press: the first character typed goes
into the field and shows the ring blue, and Esc, Enter or Tab shows it orange where the focus then is, R-464); from there the typing rules above hold, Esc, Enter and Tab included. A click anywhere else
ends any typing and leaves the keyboard in navigation mode (the mode keys find; the click hides the ring until the
next key press, R-460, R-464); it moves the keyboard's focus only where a rule already says
a click does, as a click on a console control does (R-450 A3), landing in navigation mode on what was clicked, so a
click on the Figure that pins a sample (R-453) ends typing and leaves the focus where it was (R-456, amending R-450 A3
and R-409 G4 for text fields; its scope applied per R-369). The footer and the console follow the same two modes (R-405, R-409).

**The footer and the console are scopes too (R-405).** The footer, the bar at the bottom of the window, is big scope 8,
after Legend, so Tab runs 1 top bar · … · 7 Legend · 8 footer and wraps to the top bar. Enter on the footer opens the
console (§G12), as a click does, with the focus inside it. The console is a window with exactly two scopes, top to
bottom (R-450, amending R-405 and R-409): the **header bar** (the filters All, Warnings, Errors, Info, then the search
field, Copy, Clear and the close button ×, in that visual order) and the **log** (the entry list). Opened by the
footer or from Windows › Console, it takes the focus, landing in navigation mode on the header bar scope itself, the
whole scope highlighted when it was opened from the keyboard; opened with the mouse it lands there with no ring (R-460,
R-464). In navigation mode on a scope, ↑ and ↓ move between the header bar and the log, and Tab and
Shift+Tab do the same, staying in the window. Enter on the header bar goes in, onto All; ← and → move across every
header element in visual order, the search field, Copy, Clear and × included, stopping at the row's ends; Esc goes
back up to the header bar scope. The search field is a text field, as below. × is an ordinary focusable button: Enter
on it, or a click, closes the console and returns the focus to whatever opened it, the footer or Windows › Console;
Esc from the window's top level, on the header bar or the log scope, closes it too. Enter on the log starts
interaction: ↑ and ↓ scroll it one entry, Page Up and Page Down a page (the entries the log shows), Home and End go to
the first and last entry, and Shift gives ×10, as everywhere; the whole filtered log is reachable, not only the rows
drawn; Esc returns to navigation mode on the log scope. A click on a console control moves the keyboard's focus onto
it, in navigation mode, but a click on the search field starts typing, in interaction mode (R-456). The ring and the breadcrumb follow ("Console › Header › Copy", "Console › Log"). Nothing about this order depends
on state: an empty filtered list, a hidden row, or how the console was opened (R-450, A1 to A6). The console opening
by itself on an error does not take the focus.

The ring's, the breadcrumb's and the `?` overlay's look, the held-key delay and repeat (500 ms, 40 ms) and the base
steps are confirmed as #174 built them, and recorded in R-404.

## G4. Lock — the reticle and the pin (`08_lock.png`)

Locking (K, a shortcut with the focus in the Figure, in either keyboard mode (§G3, R-409); or right-click → lock
here) recentres the view on that point and marks it with a gold reticle at the centre.
Every tilted plane passes through it, so it's the one point that stays still while the picture turns. The compass shows the
same point as a gold pin.
- **Sliders are re-based, not frozen:** each shows the anchor plus an offset, and moving one is a deliberate excursion from
  the anchor. The lock flag, the anchor and the excursion `δ` are `SimConfig` — lock is chart construction, so locking and unlocking
  are undoable (R-69; gui_state_contract §2); recentring is a `SetField` on `z₀`.
- **Still works:** tilt, rotation, zoom and chart changes all turn about the pin.
- **Unlocking:** the view as it stands becomes the new free start.
- **The point:** affine charts compute `z_locked = z₀ + (2s−1)q₁ + (2t−1)q₂` directly (chart_decoder_contract Part 3, the
  affine chart); nonlinear charts replay Φ and D on the CPU.
- The lock badge offers **unlock** and **open in Inspector** (§G8).

## G5. Windows (`04_windows.png`)

**Every window moves and resizes (R-461).** The Profiler, Export & share, Display, Run, the Chart builder (§G7) and the
Inspector (§G8) are windows, not overlays: each is dragged by its title bar and resized from its edges and corners, down
to a minimum size that keeps its title bar and controls usable. Each stays at least partly on screen: a drag or a
change of the GUI's size never leaves less than its title bar's height of it, and 64 points of its title bar's width,
inside the GUI. Each window's position and size are remembered across sessions, in the GUI's own settings, never the
contract; with none remembered a window opens where the artboards put it. The console is docked along the bottom
instead, its height dragged (§G12).

### Profiler

Tabs: Timeline, Flame, GPU, Memory, Counters, **Arbiter** (the arbiter's debug overlay — estimated throughput, current
rung, headroom, recent decisions and why; `principia_quality_device_note.md`, R-129); live / pause / Capture; a
**passive logging** switch (telemetry §1.2), with its indicator in the footer (R-129). It shows:
- the frame-time trace, with frame ms, p50, p95, p99 and the worst frame;
- where the frame goes (a donut), stacked per-frame bars for the last 60 frames;
- a substeps-per-pixel histogram, with the cap marked (the long tail near close encounters is the GPU divergence);
- a flame chart of nested scopes on the main thread; GPU timestamps per pass;
- memory (heap, GPU, tile cache) over time; live allocations by type, with their change over 60 s;
- a **leak detector** that flags steady growth while idle;
- the compute shaders' fast-math setting, from the session header, with the compute stage's mode as compiled beside it
  where the two differ (telemetry §5; R-297).

**Schema v1 (R-56):** telemetry §2's frame record and its **five stages** (integrate / reduce / colour / upload / present)
at the top level; nested scopes, GPU passes, allocations and events beneath them. JSON. Leak flags and hot-path summaries
are precomputed, so an agent reads conclusions, not raw traces. The artboard's finer categories (quadtree, stain + style,
IC decode, readback, egui) are scopes nested under the five stages.

**For agents, the same data structured:**
```
prin profile --scenario deep_zoom_03 --frames 600 --json out/prof.json   # a fixed scenario, headless
prin profile diff out/base.json out/prof.json --threshold 5%            # exits non-zero on regression
prin profile query "top 10 scopes by p95" --live                        # query while running, same schema
```
Scenarios are deterministic. Buttons: Export trace (JSON; it also writes the Chrome Trace Event format for Perfetto
and `chrome://tracing`, R-207), Open in Tracy, Headless render….

**What `prin profile diff` compares (REQ-TOOL-119, R-72).** Each scope's p95 in NEW against its p95 in BASE. A scope
regresses when NEW's p95 is more than P% above BASE's, `(p95_NEW − p95_BASE) / p95_BASE × 100 > P`, or, where BASE's
p95 is 0, when NEW's is above 0. The diff exits 1 when any scope regresses, 0 when none does, and 2 when it cannot read
either file as schema v1.
- **The statistic.** A scope's p95 is the nearest-rank 95th percentile of its per-frame ms over the file's frames: the
  samples sorted ascending, the ⌈0.95 n⌉-th, counted from 1 (telemetry §3: percentiles, not means). `prin` and the
  interactive path share the percentile code (telemetry §5.5).
- **The scope set.** The frame (its `frame_ms`); each of the five stages (its `stage_ms`, in the frames where it is not
  `null`); each CPU scope, named by its stage and the names from the stage down to it (`integrate/quadtree`); and each
  GPU pass, by its stage and name. A scope that occurs more than once in a frame gives that frame the sum of its ms; a
  frame where it does not occur gives it no sample.
- **A missing scope.** A scope in only one of the two files is listed, as only in BASE or only in NEW, and is not
  compared; it does not by itself make the diff exit non-zero. The output names each one, so a scope renamed or gone
  is shown, not passed over in silence.
- **The threshold, exactly (R-323).** P is a percentage ≥ 0 written as a plain decimal (`5%`, `5`, `7.5%`), and is
  read as the decimal it is written as; each p95 is the double the file gives. The test is decided exactly, as
  `100 × p95_NEW > (100 + P) × p95_BASE` in rationals, not in rounded floating-point arithmetic: a p95 of 100 → 107 is
  a rise of exactly 7%, so it is not a regression at `--threshold 7%`.
- **A file with no frames (R-323, R-328).** A NEW trace with no frame records has no p95 to compare,
  and the diff exits 2, as it does for a file it cannot read, saying that NEW has no frame records (R-323). A BASE
  trace with no frame records is treated the same way, saying that BASE has no frame records: with nothing in BASE the
  gate could never fail (R-328, confirming physics's finding 2 on TASK-M0-18).
- **A cut-off or incomplete trace (R-323, R-298, R-299, R-358).** When either file is a session that ended before its
  summary line, its last line perhaps cut off, the diff first prints, for that file, "session incomplete" and the number
  of bytes the reader dropped from a cut-off last line (0 when none). It then compares that file's frames and exits as
  it would for a complete one, so an incomplete trace is never compared over fewer frames in silence. When either file
  is a complete session whose last line, after its summary line, was cut off (R-356), the diff first prints, for that
  file, the number of bytes the reader dropped after the summary line, without "session incomplete", since the session
  is complete; it then compares the file as any complete trace. The bytes are printed before a refusal for no frame
  records too, so they are always reported and nothing is compared silently (R-358).

### Export & share

- **Image:** size (multiples of the view), format; **embed the view (pxpack)**, and optionally **the stain's WGSL**. The
  picture carries its own settings, so opening it recreates the view exactly (§G9).
- **State:** copy snapshot (JSON), save snapshot, load; a share link (`principia://view?…`). The spec is the object and the
  picture is its shadow (`principia_export_animation_contract.md` Part 6).
  Snapshot JSON, share links and pxpack carry `SimConfig` and `RenderState` in their one canonical
  serialisation (`principia_gui_state_contract.md` §2, R-309).
  That record is the export's sidecar: with `SimConfig` it carries the tilt map (`tilt_map=sheet|lean`, R-474), a
  curved chart's tilt (R-473), each energy-type axis's mapping (R-471) and a saved chart's definition (R-475), and
  beside it F3's fill mode (`f3_fill=extend|fit`, R-472) and the lines (`Lines`, R-476).
- **Present:** hide all chrome; Esc returns.
  With the chrome hidden the figure fills the window, as with F3 (R-406), in the mode View › F3 fills by sets (R-468;
  applied per R-369).

### Display — the last stages

**Order is fixed (R-67):** SimState → stain → style → display scale → gamut clamp → colour-vision simulation → screen (R-111). The stain colours the data, the style draws it,
the display scale and gamut clamp finish it, and colour-vision simulation shows how the finished picture is seen: the
simulation sees the final in-gamut colours.
- **Style** is optional and applies to the figure only. Scientific checks run with **plain**. The styles are the poster's
  (`workbench/principia_poster_both_sides.html` and its press module; ✱ R-130). **Watercolour & pencil** is its painted
  treatment (`window.stainPaint`, :2375–2388: a slight blur and desaturation, a soft graphite line where the colour
  changes, and the paper grain when Paper is on). **Print** is paper.design's CMYK halftone shader (Paper Shaders,
  Apache-2.0; :2872) with the poster's patches (:2876–2892): each plate slips as a whole (per-plate misregistration,
  :2881–2888), the finest screen is a cell ceiling (`u_maxCells`, :2879, :2892), and the base dot is controllable (:2878).
  v1 offers plain, watercolour & pencil, and the seven print presets, named as the file names them (:2875): Default,
  Drops, Newspaper, Vintage, Poster78, Riso, Duotone.
- **Colour-vision simulation:** off, deuteranopia, protanopia, tritanopia, achromatopsia (R-123).
- **Overlays:** class edges, cursor crosshair. *Was (before R-476):* "grid, class edges, `t_end` contours, cursor
  crosshair"; the grid and the `t_end` contours are Lines ▾ presets (§G2; R-476, L2).

The display stage stays global and outside the pipeline (Part II §12). This window and the Overlays menu (§G2) replace the
corpus's top display bar (R-67).

### Run — from the top bar

Rarely changed, so it lives in a window, not on the page. Every field is a `SimConfig` field (gui_state_contract §2).
**The Run window exposes the parameters the contracts define, under their contract names (R-68).**
- **Integration:** integrator_contract Part 3's parameters — `T_horizon` (physical time, `∈ [50, 200]`, default 50, Part 5; R-132),
  `dt_macro` (derived, not editable: shown read-only with its rule `max(1e-3, T/65535)`, R-138), `N_max` (default 64), `r_sub` / `gamma_sub`, `r_coll` (a user-exposed sim key, Part 7), `r_close`,
  `eps_E` / `eps_L` — and the integrator occupant (stepper × regularisation; Heggie with KDK leapfrog is the general
  default, Aarseth–Zare is kept for benchmarks; Part 2b).
- **Compute fast-math** (R-297): off by default; on is an opt-in optimisation for speed, and while it is on, parity
  is measured, not exact (`principia_parity_contract.md` §4). It is on the sim key, so changing it re-integrates. In
  the browser build the control is shown disabled, with a note that the browser chooses the mode (R-303).
- **Escape:** the criterion is shape closure + energy sign (R-29): `tau` and the escape window (0.4 time units,
  provisional).
  There is no persistence count (change 11).
- **Refinement:** quality (the preset selector — Auto, named tiers, Custom; gui_state_contract §6), with the **Custom
  quality fields** under "quality: Custom" (R-129); frame budget (ms), max depth (`MAX_REL_DEPTH`), ensemble `E` (samples
  per pixel); the **target-utilisation ceiling** (telemetry §6.4, "A deliberate ceiling, user-visible"; R-129).
- **Recompute** and **Cancel**, with progress.

The surfaces with no artboard (the Custom quality fields, the utilisation ceiling, the Profiler's Arbiter tab, the
passive-logging switch and indicator, the compute fast-math control and the Profiler's fast-math line, R-297) are checked by presence only, not layout, until the M8 dev GUI (R-129).

## G6. Legend — generated by evaluating the stain (`06_legend.png`)

Walk back from OUT; sample each colour and brightness node over its **declared input domain** (R-53: the node interface
declares it, inheriting the manifest's per-field domain by default); push the samples through the real combiner. **Each
dimension gets its own key:**
- categorical: swatches, with their shares of the view;
- then a separate bar for what brightness (or any second channel) encodes;
- number ramps: ticks placed through the curve (a log curve gets log-spaced ticks);
- direction fields: a sphere key (the node's own sphere colouring, sampled);
- coordinate maps: their colour square (e.g. `u → red`, `v → green`);
- post operations: line samples (quad boundaries, grid).

It's called **"Legend"**, never "Fate". The outcome legend uses colour_composition §1.4's canonical palette (R-77) and
its 1-based class names (R-449, superseding R-22's 0-based display labels): "collision 1–2", "collision 1–3", "collision
2–3", "triple collision", "escape body 1", "escape body 2", "escape body 3", "triple ejection", each from the one display
function (payload §2, "What a person reads"); no key shows "body 0" or a pair id. Its keys are the nine classes,
physical outcomes only (R-444), each a flat swatch: the black key reads "bounded / nothing yet", which `running` and
every failure share in the outcome view (R-446); it has no "degenerate", "collision @ t=0", `running` or `sim_failed`
key. **Each key's name is the outcome class's name from the same display module** (applied per R-369): the black
class's is "bounded / nothing yet", the other eight's the function's names for their `state` and `detail`.
**No forbidden-region key (R-467, amending R-446).** A presentation legend has no "forbidden region" key and no
share for it: a pixel REQ-COL-102's lane marks forbidden, either of the chart contract's hatches (the domain's end and
the universal fallback; colour_composition §1.4), is not part of the chart and draws the canvas background, unkeyed.
**The nine classes' shares are area statistics over the chart's own pixels** (chart_decoder_contract Part 3; §G7): a
visible pixel counts only if every axis is inside its primary range, and a pixel that is not a system, a domain's end
or a state that fails the validity check, leaves every share and the total, so the shares sum to 100 % of the
counted systems (applied per R-369). Inside `[0,1]²` the shares are unchanged. **The debug view's key** (§G2, View ›
Debug view) comes after the stain's own keys: "Debug", three entries, "forbidden (chart)" in the hatch style, "decode
or sim failure" in `debug_invalid`'s style and "NaN / Inf" in its own style (REQ-COL-110), each compared as the style,
not as one colour (R-136), each drawn always with its count, the number of the figure's pixels drawn in it, and its
share of the figure's pixels, raw pixel counts, not R-408's statistic. *Was (R-446, applied per R-369):* "A forbidden
chart region keeps the hatch, and the outcome legend keys it with one more key after the nine, labelled "forbidden
region", drawn in the hatch style (`debug_invalid`) …; it is drawn always, its share 0 where the view holds no
forbidden pixel"; "Its share and the nine classes' shares are area statistics over the view …; a pixel hatched because
the domain ends there … is drawn with the key's hatch but leaves every share and the total; the forbidden key's share
is that of the states that fail the validity check, §G7's "forbidden in view".".

**A free-fall slice under `η_E` is almost all `decode_failed`, drawn black, and that is expected (R-441, reworded by
R-446; dd_decoder §3.7).** Energy
normalisation scales the momenta, `pᵢ ← η_E·pᵢ`; with zero momenta it cannot reach any `E*` above the potential, since
scaling zero momenta cannot raise the kinetic energy. So with `η_E` on, every pixel of a zero-momentum (free-fall)
slice whose `E*` is above its potential `U = V₀` is tagged `DEGENERATE(ENERGY_UNSCALABLE)`, and any pixel whose `E*` is
below it `DEGENERATE(ENERGY_INFEASIBLE)`: these are `decode_failed`, drawn black `#141418` in the outcome view, the
bounded / nothing yet key of the outcome legend (colour_composition §1.4; R-446). A pixel the mass block has already
tagged, such as `DEGENERATE(M01_TINY)`, keeps that tag rather than an energy one, but it is `decode_failed` too and
drawn the same black. The one exception to the colour is where `E* = V₀` exactly: the IC is already at the target, the
decode is untouched and untagged, and the pixel integrates, drawn in its outcome's colour once its event fires.
`V₀` varies over the slice and is negative, so only an `E* < 0` inside its range has such a contour
`V₀ = E*`, between the `ENERGY_INFEASIBLE` and `ENERGY_UNSCALABLE` regions; on a sampled grid it shows only as
scattered pixels where a sample lands on it exactly, not as a drawn line. `η_E` is then a no-op on every free-fall
preset, and the near-uniform black, with that contour its only exception, is expected, not a bug. The Chazy stains
draw the same samples black too, Chazy B's `#141418`, as every presentation view draws a failure (R-467, ruling
RQ-319; dd_chazy_final_motion §7), so the same slice reads black there; View › Debug view shows them as "decode or sim
failure". In both systems the black key's share counts these samples, so there it reads near 100 % and is not a
bound fraction; the debug view's counts separate them (dd_chazy_final_motion §10.10; R-467, applied per R-369). *Was (RQ-319, decided per R-369):* "The Chazy stains draw the same samples with the render contract's hatch
(`debug_invalid`), as they draw `sim_failed`, not black, …, so the same slice reads hatched there." The
tags show in the η_E debug view (R-446), the debug views that keep the invalid hatch and their own palettes: today
the `state` view shows `decode_failed` and the `detail` view its failure category (debug_tooling_plan §B); which view
the ruling's "η_E debug view" is, is RQ-333. *Was (R-441):* drawn degenerate's white `#ECECF0`.

## G7. Chart builder (`03_chartbuilder.png`)

- **Each axis has a kind:** latent direction (any mix of the eight `z` components, normalised; a normalise button shows
  `|q|`), physical quantity (energy, `L_z`, virial ratio, mass ratio…, with a range and what is **held fixed**, e.g. "z₀ along
  the other 7" — the kind-2 residual convention, chart_decoder_contract Part 3), or Burrau dimension (Euclid `(m, n)` → the
  primitive triple, and the neighbourhood swept, e.g. mass × momentum).
- **Presets are saved pairs of axes and are editable:** Save, Duplicate, Delete. Each preset also carries the values
  the resets and Home read (§G2, R-466): a base point, a centre `(u, v)` and a window, defined by TASK-M6-45 (R-72,
  REQ-GUI-224); a saved preset stores them with its axes.
- **A physical-quantity axis makes the chart nonlinear (Φ).** Pixels map through Φ, then the decoder. Lock replays Φ and the
  decoder on the CPU instead of the affine `z₀ + s·q₁ + t·q₂`.
- **Each axis carries its extension type past `[0,1]²` (R-407):** a latent direction is affine; a physical-quantity axis
  mapped linearly onto its range is affine, the fallback hatching what fails validation, and one behind a nonlinear
  warp declares nothing and is bounded (R-408's port, B1: first made bounded, by R-407's default). B1 covers the
  non-periodic quantities listed above (energy, `L_z`, virial ratio, mass ratio); an angle-like quantity declared affine
  would redraw systems and count them again, so it takes its type only by a declaration under physics review. A Burrau
  dimension takes the Burrau family's types (`principia_chart_reference.md` §5.4). The chart inherits its extension
  from its axes. **R-471:** an energy-type physical quantity (energy, `L_z`) is warped by default
  (`principia_chart_reference.md` §2.1, per pixel), every pixel of `[0,1]²` feasible and the axis bounded past its edge;
  its range mapped linearly is the option, B1's affine axis, whose infeasible pixels draw blank and log no decoder-bug
  warning (§G12).
- **The Domain preview:** the chart's admissible region in its own coordinates, the forbidden region hatched, the current view
  as a rectangle, the boundary's formula, and "forbidden in view: N%". **Each chart supplies its domain function** (R-26:
  `validate(u, v)` on the `Chart` trait). **"Forbidden in view" counts each system once (R-408):** a visible pixel
  counts only if every axis is inside its type's primary range (periodic, one period; pole-crossing, pole to pole;
  affine, unbounded; bounded, its domain); a pixel hatched where the domain ends (a bounded axis past its edge, or a
  failed `Φ`) leaves both the count and the total; only states that fail the validity check count as forbidden. Inside
  `[0,1]²` the share is unchanged (`principia_chart_decoder_contract.md` Part 3). The preview keeps its hatch and its
  share under R-467: it is a diagnostic for building charts, not a presentation view (R-467, clause 6).
- **A quick render** (e.g. 64 × 64 at a short horizon), at the view's aspect. **Both previews are square.**
- The footer checks `q₁ · q₂ = 0` and lists the hidden directions (and the residual). Revert / Apply.

## G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)

**The IC Inspector and the trajectory viewer are ONE window (R-65).** trajectory_viewing §4's panels are hosted here and in
Explore's Trajectory panel (§G2); trajectory_viewing §4 says where each lives. The standalone IC Inspector tool is absorbed;
its HTML (`docs/gui/reference/ic_inspector.html`) is prior art.

- **Pane 1, the IC:** drag bodies 1, 2, 3 (internally 0, 1, 2; R-449) and their velocity arrows, with ghost markers at the playhead. **Right-click a
  body** for its properties popover (mass, position, velocity, momentum, distances, per-body share of P / L / E — all
  editable, in sync with dragging); each body's disc radius is ∝ ∛m (R-96; `ic_inspector_scratchpad.md`).
- **Pane 2, the canonical representative:** bodies or the shape sphere (turning, with axes, or unwrapped); "ghost the gauge
  transform" shows what the gauge buttons did.
- **Pane 3:** the trajectory in real space (CoM frame).
- **One timeline scrubs all three; editing the IC re-integrates the orbit live.** Integration is `computeIC` on the CPU in
  f64 (trajectory_viewing §1); the engine is reached only through it (§6). **listen** is here too.
- **Reset, Randomise, Burrau-ish, Rotate 15°, Scale × 1.25, Boost →** demonstrate the gauge quotient: the canonical pane
  doesn't move.
- **Readouts:** latent `z`, `α, β`, masses, `E · L_z`, outcome, F₂ word, round-trip error, gauge error; plots of energy error
  and pair separations.
- **Tabs: Inspect** and **Create & locate**: build an IC, see where it lands in `z` and its distance from the slice, then
  **lock the view on it**, **centre a slice through it**, **mark it in the figure**, or **keep it**.

## G9. Import picture · saved views · record a sweep (`09_importrecord.png`)

- **Import picture:** drop a Principia PNG (or File › Import picture…). It reads the pxpack, shows what's stored (chart,
  centre, zoom, tilt, stain, time, when it was made) and **what differs from now**, and offers **restore this view**, **only
  its stain**, or **open side by side**.
- **Saved views:** pxpack snapshots, each with a thumbnail and "go"; "Save this view". A saved view reopens under the
  tilt map it was saved with, one saved before R-474 under Lean each axis (R-474). A saved view is a whole view; a
  saved chart (§G2's Save view as chart, R-475) is a chart in the picker.
- **Record a time sweep:** a time range, a frame count, the quality each frame is refined to, a size at the view's aspect,
  GIF / PNG frames / MP4 (MP4 through a system `ffmpeg` when present; in the browser, PNG frames zipped, GIF via a wasm
  encoder, MP4/WebM through WebCodecs where supported — export_animation Part 4, R-131), overlays on or off, pxpack in every frame. **Recording integrates each frame to its own `t`**, so
  it's exact, unlike scrubbing. That is the export contract's blocking mode (export_animation Part 4: a hard barrier per
  captured frame).

## G10. Measure — a tool on the figure (`10_measure.png`)

Drag a square region on the plot itself; the sampled pairs show on it, with the pairs that disagree marked. Esc leaves the
tool. The side panel shows:
- the method: **uncertainty exponent** or **box count**; samples (pairs per ε), the ε range, and what classifies a pair
  (e.g. outcome class);
- the log-log fit, `α ± error`, and `D = 2 − α` for this slice;
- **the resolution hazard as two buttons:** a threshold sweep, and a matched **N / 2N** pair;
- Export CSV, Keep region.

**The measurement path owns `FULL_RETENTION`** (scheduler_contract Part 5, bit 4): a uniform grid with every sample kept
(R-39).

## G11. Research — first pass (v2) (`11_research.png`)

- **Periodic-orbit seeding** from spiral cores, where the winding number diverges; Newton-refine from each; residual and
  period per seed; compared with the Šuvakov–Dmitrašinović catalogue (`principia_dd_validation_orbits.md` §1.4).
- **Continuation** along a parameter (e.g. a mass ratio), with a step, marking folds where stability changes. Stability
  is read from the monodromy matrix's **Floquet multipliers**: a **fold** is where a multiplier crosses +1;
  **period-doubling**, where one crosses −1 (R-127).
- **Poincaré return map** on a chosen section, for a kept orbit. The sections offered: **syzygy crossings** (w = 0), a
  chosen **shape-sphere great circle**, and a **Jacobi-coordinate hyperplane** (R-127). Transcribed with citations,
  physics-reviewed, confirmed at the gate.
- **Side by side** with a linked cursor and navigation, and a difference view. The linked views are a separate
  `ViewUI` item, specified with these v2 research tools (R-106) — not the chart's link ids in `SimConfig`.

## G12. Console (`12_console.png`)

The footer, opened: severity, time, source, message; filters (all, warnings, errors, info, text); copy and clear. It is the
same stream as the profiler's telemetry. Errors open it automatically. Sources include the stain, the integrator, the
quadtree, the contract (each `SetField` is logged) and the app.

The keyboard reaches it through the footer, big scope 8 (§G3, R-405). Its entry list scrolls with ↑ ↓ only after
Enter on it, in interaction mode, and Esc returns to navigation mode on the list (§G3, R-409).
It is reached from Windows › Console too, and Esc returns the focus to whichever opened it.

**Its keyboard model is two scopes (R-450):** the header bar (All, Warnings, Errors, Info, the search field, Copy,
Clear and ×) and the log. It opens on the header bar scope, in navigation mode; ↑ ↓ and Tab move between the two;
Enter on the header bar goes in and ← → cross every header control, × included, Enter on × closing the console; Enter
on the log scrolls it by entry (↑ ↓), page (Page Up, Page Down) and to its ends (Home, End), Shift ×10, over the whole
filtered log. Every visible control is reachable, whatever the console's state (§G3). A click on the search field
starts typing, in interaction mode; a click on any other console control lands in navigation mode on it (R-456).

**Its height and a maximise toggle (R-463).** The console is docked along the bottom of the GUI, over the bottom row.
Its height is dragged from its top edge, between a minimum of its header bar and three log rows and the height below
the top bar, and is remembered with the layout (R-462). A **maximise** button in the header bar, between Clear and ×,
and the console's shortcut **M** (§G3's scope shortcuts, not while the search field is being typed in) toggle it
between its height and the full height below the top bar, covering the panels and the figure until toggled back; the
user chose it, so it is no notification over the figure (§G1). Opening and closing it each complete within one frame
at 60 fps (16.7 ms), measured on the human's Mac before and after the fix, with where the time went.

**A decoder bug inside `[0,1]²` is warned here (R-467, clause 4).** Inside the square every gallery chart is valid by
construction, so a pixel the chart classifies as not a system there (a validity failure, or a `Chart::map` that fails)
is a decoder bug: it draws the canvas background in a presentation view and the hatch in the debug view, and the
console logs one warning per chart and view, severity warn, source `app`, "decoder bug: chart <name> fails validity
inside [0,1]² at pixel (<x>, <y>), (s, t) = (<s>, <t>); <n> pixels in this view", the first such pixel in row order
from the bottom-left, logged again only when the view or the chart changes (applied per R-369). The real engine should
log none (REQ-CHART-058's sweep). Whether a Chart-builder chart's in-square failure warns is RQ-372. An energy-type
axis under its linear option (R-471, E2) has infeasible pixels inside the square by design: they draw blank and warn
nothing.

## G13. Where the artboards are overridden

The pictures are layouts, and **their values are illustrative: corpus values win, and the Run window exposes the parameters
the contracts define, under their contract names (R-68).** Where a detail in one differs from a ruling, the notes or the
corpus, the spec follows the authority:
- **Body and pair labels** are 1-based, as the artboards show them ("body 1 escapes", "collision 1–2", "body 3
  crosses"): R-449 supersedes R-22's 0-based display labels, which overrode the artboards here. A pair is named by its
  two bodies, smaller first, never by its pair id; storage, computation and machine-readable exports stay 0-based
  (payload §2, "What a person reads").
- **Escape has no persistence count** (R-29, change 11). The Run artboard shows "persistence 8".
- **The profiler's top level is telemetry §2's five stages** (R-56). The donut shows other categories.
- **The compass references** (R-451, R-452): `docs/gui/reference/principia_nav_cube.html`,
  `principia_plucker_compass.html` and `principia_compass_charts.html` are references for look and behaviour,
  transcribed through §G2's compass subsection. Where they differ, the spec follows the rulings: their light cream
  palette is not copied (the dark theme, §G1); the cube opens in Tilt, not Slice (R-409); their own controls (axis
  selectors, sliders, Animate) are not on the compass; the cube's tilt follows the decoder's mapping, not the
  reference's axis-angle map (RQ-340); the cube's slice drag follows the pointer along the drawn normal's screen
  direction, not the reference's horizontal `dx` along its z axis (R-465); the grids draw no in-cell numbers; the
  physical compass's Jacobian and its body position labels are a synthetic placeholder, replaced by the decoder's
  Jacobian over R-452's coordinates.
- **The console's header** (R-450). The console artboard's row is "copy · clear · console ▾"; the header bar adds a
  focusable × after Clear, which closes the console, and the console is two scopes, the header bar and the log (§G12).
  R-463 adds the maximise button between Clear and ×.
- **The Navigate section and the slice controls** (R-457). Where `01_main.png` shows the eight `z₀` values under
  Navigate and a step under the slice and tilt, the GUI has neither: the `z₀` values are the Slice centre sliders,
  and the sections are named Slice centre and Tilt, each tilt angle with its target dropdown.
- **The resets and Home** (R-466). `01_main.png` shows no reset beside the Manifold view's headings, no Home button
  on the figure and no View › Home; the GUI has all three. The Home button is the one control drawn over the figure,
  in its top-right corner, by the human's words; nothing else covers it (GUI_DESIGN_NOTES, "The figure is never
  covered").
- **The View menu's fill modes and the debug view** (R-467, R-468). No artboard shows View › "F3 fills by: Extend /
  Fit" or View › Debug view; the GUI has both. `06_legend.png` shows no forbidden-region key, and the presentation
  legends have none (R-467); the debug view's "Debug" key is not drawn there.
- **The layout and the windows' places** (R-461, R-462). The artboards' panel sizes and window positions are the
  defaults; splitters and the windows' title bars and edges change them, and View › reset layout returns the panels to
  the artboards'.
- **No camera** (notes, G1). The Research artboard shows "linked camera", and the console shows "SetField CameraZoom".
- **"Legend", never "Fate"** (notes). The Stain artboard shows "Time of fate", and the Display artboard shows "fate edges".
- **The compass and the focus ring** (R-409). The main artboard shows the compass's "slicing / tilting" labels and "drag
  the cube to orbit"; the compass has Tilt and Slice buttons and no orbit. The keyboard artboard shows one blue ring; the
  ring is orange in navigation mode and blue in interaction mode.
- **The reference HTML files' event palette** (R-444 to R-446). `principia_render_modes.html` and
  `principia_dev_gui.html` label the outcome classes "coll @ t=0" and "degenerate", and the render-mode catalogue's
  outcome row gives "t=0 orange, degenerate white"; the outcome view has neither class, and its classes and colours are
  colour_composition §1.4's.
- **Artboard values are illustrative; corpus values win (R-68):** the outcome palette's hex values, the substep cap
  (`N_max`, default 64), and the sound mapping (`θ(t), φ(t)` → spectrum). The Run window exposes the parameters the
  contracts define, under their contract names — so the artboard's integrator "tolerance" field has no counterpart.

## G14. Settled by the notes (record)

1. The dev GUI is a debug layer (F3) with the final UI's structure and every feature.
2. Explore and Stain are the two modes; everything else is a window or a tool.
3. "Manifold view" is one group: chart, navigation, centre, slice and tilt, rotation.
4. Run settings live in a window, not on the page.
5. The figure is never covered; warnings go to the footer console.
6. Every slice preview keeps the viewport's aspect.
7. The compass switches mode by itself, and by its Tilt and Slice buttons, with no orbit (R-409); lock re-bases the
   sliders and pins the compass.
8. The legend is generated by evaluating the stain, with one key per dimension.
9. Keyboard: a tree of scopes, a focus ring and a breadcrumb, nothing else on screen; the ring is orange in navigation
   mode and blue in interaction mode (R-409).
10. The measurement path owns `FULL_RETENTION` (R-39).
11. Users keep their own stains in the library (New, Rename, Import, Export) — Part II §11's "later question".

---

# Part II — The stain editor

*The Stain mode (`02_stain.png`): the plain node-graph editor over the composition algebra. Sections §0–§16 keep their
numbers. The stain is a free, typed node graph (R-64); `principia_gui_state_contract.md` §5 describes the same object.*

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

*The artboard (`02_stain.png`) shows the same four surfaces: library left, graph canvas centre with
`Graph · Pipeline WGSL · Node WGSL`, preview and node inspector right; the assembled code and a **Problems** pane (compile
status, notes, auto-recompile) sit under the canvas. The global display bar is replaced by the Display window and the
Overlays menu (R-67).*

```
┌──────────── top bar (Overlays ▾ · Display…) ─────────┐
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
- **Display window + Overlays menu** (R-67; was the global display bar, top) — style / render-scale / gamut / CVD; applied to `OUT`,
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
- **`debug_invalid(frag_xy: vec2<f32>) -> vec3<f32>`** — the reserved invalid-pixel treatment (the validity-first
  invariant, §13): it draws the hatch from the pixel position (R-136), a pattern that collides with no palette entry, its
  exact pattern a calibration (REQ-COL-055, R-71; R-132), confirmed at the M1 gate as proposed (R-428). Only NaN, and
  a stored `±∞` (below), get it: a stored sentinel such as −1.0 shows as its
  literal value on the ramp (R-79's exception, R-136). A NaN reads as "no data", never as a value. A stored `±∞` that
  is no declared sentinel or unset value gets it too, on the CPU and the GPU alike (R-436): `rho_ratio`'s `+∞` at the `α = 0` pole,
  `V_0`'s and `E_0`'s `−∞` at the collision pole (`principia_dd_decoder.md` §3.6).

Each numeric debug field therefore generates a two-line `colour()` — the NaN guard, an exact **bitcast comparison** of
`raw` against the canonical quiet-NaN bits that returns `debug_invalid(frag_xy)` (since R-436, the non-finite guard: an
exponent of all ones, which also masks a `±∞` that is no declared sentinel or unset value, below) (`principia_render_contract.md` Part 2's rule; never a
self-comparison or `isnan()`, which fast-math may fold away — R-114), then `ramp( range_norm(raw, lo, hi, RANGE_AUTO, u_range) )` — where `RANGE_AUTO` is the
fixed↔auto flag, editable identically in the node inspector, on the node in the graph, and in the
code (§9, §10). **Debug fields are raw** — the stated exception to §13's validity-first rule (R-79): apart from the NaN
guard (the non-finite guard, R-436) there is no validity masking — a failed-state sentinel (e.g. `0.0`) is shown as its literal value, cross-checked
against the raw `state` field, not silently recoloured. NaN still goes to the invalid pattern, and so does a stored `±∞`
that is no declared sentinel or unset value (R-436).

**The template as built (TASK-M1-09; RQ-231, decided per R-369; R-72 for the cyclic period, R-71 for the log
floor).** `ledger::gen::numeric` emits it for every numeric field but the drifts, which keep R-381's `symlog` view.
`raw` is the field's value compacted per its ledger scale: `lin` and `diverging` the identity; `log`
`1 − 1/(1 + ln(1 + |x|/ε))`, `dbg_log`'s place, on the fixed `[0, 1]`; `cyclic` `fract(x / period)` on the fixed
`[0, 1]`, on `ramp_twilight`. Every other ramp is `ramp_viridis`, except for a field whose declared range spans zero,
which is on `ramp_coolwarm` centred at zero (R-400; TASK-M1-17). **The cyclic period is one turn, 2π:** a cyclic
field is an angle in radians (REQ-TOOL-161). `rho_angle` and `theta` are the cyclic fields: `rho_angle`'s formula is
REQ-PAY-088's, and `theta`, the unwrapped longitude `θ̃` in radians, is cyclic because it is an angle, although its
range spans zero (R-423; REQ-RENDER-085). `fract(x) = x − ⌊x⌋`, WGSL's `fract`, so `raw ∈ [0, 1)` for a negative
angle too (R-423); that half-open range holds in exact arithmetic. In f32 or f64, a negative `x / period` close enough to
0 gives `1 − |x / period|` rounded to exactly 1.0 (in f32, any `|x / period| ≤ 2⁻²⁵`), as WGSL's `fract` allows; for a cyclic
quantity 1.0 is the same turn as 0, so it is no error (applied per R-369). **The log floor** of a field with no ledger `floor` is that field's own,
`ε_f` in its units, a calibration proposed with its evidence for the M1 gate (R-401, REQ-TOOL-160; TASK-M1-18),
`rho_ratio`'s and `r_min_pair_0`'s provisionally, re-confirmed at M2 once their formulas are defined (R-419,
REQ-TOOL-163): `rho_ratio`'s assumes the mass-weighted ‖ρ̃‖/‖λ̃‖ and says so, and for `r_min_pair_0` only the
evidence is provisional; TASK-M1-09 built one ε for every such field, 2⁻²⁴, f16's smallest positive subnormal,
which R-401 replaces. The M1 gate confirmed the six as proposed (R-428): `d_min` 2⁻¹⁴, intended, `dE_max` and
`dLz_max` 2⁻²⁰, `closure_min` 2⁻²³, and, provisionally until M2, `rho_ratio` 2⁻²³ and `r_min_pair_0` 2⁻¹⁴.
`[lo, hi]` is the ledger range's finite ends; a step index's is
`[0, horizon_steps]`; a `diverging` field's range, and under R-400 every range that spans zero, is symmetric about 0.
**An undeclared range is a ledger gap, not one that spans zero (R-426; dd_generation_root §3.8):** a field is on
`ramp_coolwarm` only when its declared range spans zero; `q_mass` and `mean_y` stay there provisionally, recorded as
provisional at the M1 gate, until TASK-M2-03 and TASK-M3-13 declare their ranges and the physics reviewer re-checks
each ramp. **Each log floor is a constants-register entry** (dd_generation_root §3.8), marked provisional until the M1
gate and read from the register like any other, never a function in its place (R-427). Since the M1 gate, four are
confirmed entries citing R-428, and `rho_ratio`'s and `r_min_pair_0`'s are marked provisional until M2 (R-419, R-428).
A field carries the `diverging` scale only when its declared range spans zero (R-403; dd_generation_root §3.8), so
the scale and the ramp agree; the one-signed `K_0` `[0, ∞)` and `V_0` `(−∞, 0]` are `lin`, on `ramp_viridis`, mapped
monotonically, `V_0`'s most negative value at the dark end.
An end with no finite bound takes the measured end, and a field whose ledger range
is unbounded defaults to `RANGE_AUTO = 1`, except a cyclic one, on its fixed `[0, 1]`. `RANGE_AUTO` and `u_range` are
the view's uniforms, declared in its header (gui_state_contract §3): `// @uniform RANGE_AUTO: u32 = <0|1> [0, 1]`,
whose default the generator writes from the node's param and the assembler's declaration parser reads back, and
`// @uniform u_range: vec2<f32> = (lo, hi)`, the measured `(min, max)` of `raw`.
**A non-finite value is invalid (R-436).** The guard sends a stored `±∞` that is no declared sentinel or unset value to
`debug_invalid(frag_xy)` as it sends NaN, on the CPU twin and in the WGSL alike, by a comparison on the value's bits
before any arithmetic (an exponent of all ones; never `isinf`, an infinite literal or arithmetic on `±∞`, which WGSL
lets an implementation assume away; R-114, gpu_determinism_note rule 4); and the measured range is taken over the
samples whose stored value is finite, since the others are not on the ramp. Before the guard (the order below), a
stored sentinel shows as its literal value through `dbg_sentinel`, fed the compacted value (R-136, R-381), and
`d_min`'s unset value, f16 +∞ tested by its bits, is drawn in the "not yet" style of running samples,
`debug_not_yet(frag_xy)`: the neutral grey `DBG_NOT_YET` with a fine dot stipple from the pixel position (R-271,
R-280, R-399; REQ-COL-053, REQ-COL-064; the grey `#4E4E4E` confirmed at the M1 gate, R-433, and the stipple, its dots
`#909090`, R-434).

**The order of a view's lines (R-436; applied per R-369).** The generator emits a view's lines in the order this
section gives them, and the widened guard, an exponent of all ones, would catch `d_min`'s unset `+∞` if it came
first. So a numeric view's `colour()` tests, in this order:
1. a declared sentinel's or unset value's own bit test (`d_min`'s unset `+∞`, drawn "not yet"; a finite sentinel,
   shown through `dbg_sentinel`; a `±∞` sentinel, below);
2. the non-finite guard: NaN, and every stored `±∞` that is no declared sentinel or unset value, to `debug_invalid`;
3. the ramp.

**A `±∞` sentinel is tested on its bits (R-436; applied per R-369).** R-418's identities, `escape_time_min`'s `+∞`
(binary16 `0x7c00`) and `escape_time_max`'s `−∞` (`0xfc00`), "no escape", are declared sentinels. Such a sentinel is
tested on its bits, never against a literal: the literal-sentinel line `if (raw == <literal>)` would need an infinite
literal, which naga rejects (gpu_determinism_note rule 4), and it is tested before the guard and before any
arithmetic, never through the compaction or `range_norm`, which would do arithmetic on `±∞`. Its drawn style is not
defined by the corpus: it is a definition requirement (R-72), REQ-COL-071, written here by TASK-M5-17, the resolve
stage that first stores those bits and adds quad fields to the field-view catalogue, and reviewed by the gui reviewer.

**The prelude as built (TASK-M1-03; R-72 for the definitions, R-369 for the rest).** The ledger emits it into
`crates/render/shaders/wgsl/lib/prelude.wgsl`, the full tier's. The assembler emits it per variant
(`ledger::gen::prelude::wgsl(tier)`). Its members, beyond the two above:
- **The tier's features:** `const has_ftle: bool` and `const has_word: bool`, baked from the variant's tier bits: the
  stored variant is `SimStateFTLE` exactly when `has_ftle`, and the word buffer is bound exactly when `has_word`.
  `has_ensemble` is read from a uniform (R-145), so authors write `has_ensemble()`. WGSL has no bool in a uniform and
  no module-scope value from one. The uniform is `PreludeUniforms { has_ensemble: u32, … }`, 16 bytes, 1 when E ≥ 1,
  bound at `@group(0) @binding(0)`, in group 0 with the assembler's per-frame uniforms. The ledger writes the numbers
  as `PRELUDE_UNIFORMS_GROUP` and `PRELUDE_UNIFORMS_BINDING`. The read side's `sample_read` takes `has_ensemble()` as
  its argument.
- **`is_absent_nan(x)`:** the absence test, `bitcast<u32>(x) == 0x7FC00000u`, the canonical quiet NaN's bits
  (lowering Part 3a), never `isnan` (R-114, R-297).
- **`range_norm`:** `range_norm(x, lo, hi, auto_range, meas)`. `auto` is a WGSL reserved word, so the argument is
  named `auto_range`. A degenerate range, `h = l`, reads 0, not the implementation-defined clamp of `0/0`.
- **Colour space:** `srgb_to_linear` and `linear_to_srgb` (dd_colouring §3.1's transfer), `oklab_to_linear` (§3.1's
  inverse), and `oklch_to_linear(l, c, turns)`. The hue is in turns and reduced to [−½, ½) before `cos` and `sin`,
  whose WGSL error is bounded on [−π, π].
- **The ramps return linear RGB**, the colour slot's space (render contract Part 2). `ramp_viridis(t)` and
  `ramp_twilight(t)` read matplotlib's published tables (R-122), checked in under `crates/ledger/data/lut/` with their
  source named: matplotlib 3.8.0, `_cm_listed.py`, 256 and 510 stops. Stop `k` sits at `t = k/(N − 1)`, with `t`
  clamped to [0, 1]. The ramp interpolates linearly between stops in the tables' sRGB encoding, then decodes. A cyclic
  caller passes `fract(·)` to `ramp_twilight`. `ramp_magma(t)` reads the same release's `_magma_data` table, 256
  stops, under the same rules: the ramp of the `s_impurity` view (colour_composition §6; TASK-M1-13).
  `ramp_grey(t)` is OKLab `(t, 0, 0)`, `t` clamped: the colour slot's
  None grey (colour_composition §4.1). `hue_wheel(t)` is OKLCH with L = 0.75 and C = 0.12 at hue `t` turns, periodic
  in `t`, and in gamut at every hue.
- **`debug_invalid(frag_xy)`** draws the hatch defined in the render contract's presentation layer (Part 5;
  proposed, R-71, REQ-COL-055; confirmed at the M1 gate, R-428).

**Added by R-399 and R-400 (TASK-M1-17).** `ramp_coolwarm(t)` reads Moreland's cool-warm table (R-122), checked in
under `crates/ledger/data/lut/` with its source named, as `ramp_viridis` and `ramp_twilight` read theirs, and returns
linear RGB; its midpoint, `t = ½`, is the table's neutral, where a zero-spanning field's 0 lands.
`debug_not_yet(frag_xy)` draws the "not yet" style, `DBG_NOT_YET` with a fine dot stipple from the pixel position,
defined beside the hatch in the render contract's presentation layer (proposed, R-71; REQ-COL-053, REQ-COL-064;
confirmed at the M1 gate, R-433 and R-434: the grey `#4E4E4E`, 2 × 2 px dots of `#909090` in each 4 × 4 px cell).

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
  "duplicate to custom" suffices for the dev GUI.) **Settled by the notes:** the library has a **Your stains** group,
  with New, Rename, Import and Export (§G14 item 11).
- **Bivariate presets** (`n̂ × FTLE`, `Outcome-class × diffusion`, …) load a graph with **both**
  combiner inputs occupied — one `source→colour`, one `source→brightness` — i.e. the second slot
  filled. There is no special bivariate UI; the family is just *the brightness slot occupied*.

---

## 12. Display stage — global, outside the pipeline

*Order (R-67): stain → style → display scale → gamut clamp → colour-vision simulation → screen (§G5, Display). The simulation sees the final in-gamut colours.*

Applied to `OUT` after the graph, as **settings, never nodes** (no OUT-downstream graph):

- **style** (optional, figure only), **render→display scale**, **gamut clamp**, **CVD simulation** (models the viewer,
  not the visualisation) — in that order (R-67). *(Was: gamut clamp, CVD simulation, render→display scale.)*

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

*As built (TASK-M1-13; REQ-RENDER-024; applied per R-369):* the occupant
`crates/render/shaders/wgsl/frag/post/edge_line.wgsl` converts to pixels with `fwidth(uv)` rather than `fwidth(d)`, each
derivative taken modulo the cell (`dpdx(uv) − round(dpdx(uv))`, and so for `dpdy`), and divides each axis's edge
distance by its own pixel size before taking the nearer. `fwidth(d)` reads 0 where a 2 × 2 pixel block straddles a cell
edge, since `d` is equal on both sides, and so draws no line there. The wrapped derivative reads the pixel's size
wherever a cell is at least 2 px wide. The width, `width_px`, is the line's full width across the edge at half
coverage. The fallback tint and the pending hatch are post occupants reading `ctx.quad` at M1 (RQ-239); their styling
is debug_tooling_plan §F's (REQ-TOOL-124), and the Tier-3 path below is TASK-M5-29's.

Being an ordinary post node, the boundary overlay **serialises with the graph**, has editable width /
opacity / colour / level, chains in post order, and composes over any field — with no display-bar
special case and no bespoke pass. The rest of that render-contract list (**fallback tint**, **pending
hatch**, **visible-set**, **locked/stale**) is Tier 3 by this test: those are *values attached to a
quad*, not functions of position within it.
The visible-set and locked/stale overlays are TASK-M5-29's, defined there in debug_tooling_plan §F with the
corpus's existing meanings where they exist (`quad_state` 4 for "stale") (R-425; REQ-TOOL-166).

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

Display-stage settings live in the Display window and apply uniformly to the main render and the
preview; tile debug shaders are toggles in the Overlays menu but are *shaders*, not settings (R-67).

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
  invalid-pixel treatment, by default the hatched invalid pattern (R-132) in a debug view and `#141418` in a
  presentation view, which draws no hatch (R-467) — a NaN / sentinel must read as "no data", not as a value (composition spec
  §3, §6; R-79). *Was:* "by default the hatched invalid pattern (R-132)". Debug fields are the stated exception: they show literal stored values, and NaN still goes to the
  invalid pattern (§10.1), as does a stored `±∞` that is no declared sentinel or unset value (R-436).

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
11. **Display stage (style / scale / gamut / CVD, in that order — R-67) is global**, outside the pipeline; its controls are
    the Display window and the Overlays menu. *(Was: "gamut / CVD / scale / boundary overlay"; boundaries are a post node, §12.1.)*

---

## 16. Open / next

- **Build the interactive dev-GUI mock** — library → graph (drag / wire / drag-off-pin, backbone
  constraints, glyph nodes) → click-node → inspector (real sphere / gradient / palette) →
  `Graph | Code` → live preview.
- **Node palette contents** — the concrete list of source fields (from the ctx contract) and post
  ops surfaced in the right-click palette.
- **Preview** — sphere vs illustrative-slice toggle; which is default.
- ~~**The GUI questions RQ-20 to RQ-24**~~ **ruled by R-64 to R-68** (step 6): the stain is a graph, one Inspector
  window, the scrubber stays, the display chain and its controls, and artboard values are illustrative.
- ~~**Standing composition-spec gaps** (to reconcile on that doc's next pass, tracked in
  `principia_colour_composition.md`): colour-source-as-axis, overlays-as-post-chain-over-configured-
  base, physics-as-overlay-op, gradient-unifies-the-ramp, per-footprint vs quad-aggregate spread,
  the debug taxonomy, the addressing channel convention (UV=RG / TL=RB / centre=BG / index=grey), and
  boundaries-as-overlay. This GUI spec assumes those resolutions.~~ **Closed (R-72; REQ-COL-054; TASK-M7-01):** each
  resolution is written in `principia_colour_composition.md` §9, "The stain editor's composition resolutions". Its open
  parts are in `REVIEW_QUEUE.md`: RQ-280 (the addressing convention's unnamed channel and index), RQ-281 (the
  diagnostic and profiling groups' membership) and RQ-282 (a ramp of custom stops).
