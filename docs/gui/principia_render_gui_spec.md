# Principia — Dev GUI (egui / F3)

*Status: canonical. Rewritten in step 6 (24 Sep 2026). This is the single source of truth for the **developer GUI**: the
egui debug layer over the wgpu render, toggled with F3. It is not the final UI, but it has the same structure and every
feature. The production (TS) GUI is out of scope; it re-skins the same operations through the same contract
(`principia_gui_state_contract.md` §1, §7).*

*Sources, in order of authority: `decisions.md` rulings, then `gui/design/GUI_DESIGN_NOTES.md` (the reviewer's design
notes), then the twelve artboards in `gui/design/`. The notes win over the pictures, and a ruling wins over both. Where the
notes and the corpus disagreed, the entries (RQ-20 to RQ-24) are ruled by R-64 to R-68. §G13 lists where a picture is
overridden.*

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
  label and the lock reticle are the figure's own marks, not notifications.
- **Dark default egui theme, Ubuntu / Ubuntu Mono.** It's a dev tool; there is no styling beyond egui's own.
- **Not in the dev GUI:** Chazy subtitles, and the object-based stain desk. The Stain mode is the plain node-graph editor
  (Part II).
- **Sim key or not.** Each control knows whether its field re-integrates (sim key) or is live (render key), and warns per
  field from the caching blast-radius table (gui_state_contract §2).

## G2. Explore — the everyday view (`01_main.png`)

**Top bar.** Menus (File, View, Windows, Help); the mode switch **Explore / Stain**; **Overlays ▾**, with a count of how
many are on; **Run…**, **Profiler…**, **Export…**; the keyboard breadcrumb (§G3); status: `t`, fps, frame ms, quad count,
`budget-bound` when the frame budget binds, undo / redo depth, and "F3 hide".

**Overlays ▾** holds the toggles, grouped, each with an Alt+digit shortcut, plus **all off** and **save as default**:
- Quadtree: tile bounds, depth colouring, priority heatmap, `refine_flagged`, split / merge activity.
- Integration: substep density, substep-cap hits, NaN / invalid pixels, energy drift.
- Chart: forbidden region (the chart's domain function, R-26), grid, Burrau lattice (primitive triples).
- Stain: class edges, `t_end` contours.

Each entry is sorted by Part II §12.1's rule into a post node (Tier 1, derived from the address), a sim shader (Tier 2,
already resident) or a tile debug shader (Tier 3, a scheduler verdict). The Overlays menu and the Display window (§G5)
replace the corpus's global display bar (R-67).

**Left: "Manifold view" is ONE group.** Chart, navigation, centre `z₀`, slice and tilt, and rotation are one thing: how you
view the manifold.
- **Chart:** the preset, named by its axes (e.g. "z_α × z_β", never a nickname); the two basis vectors `q₁`, `q₂`, each with
  an edit button; **Chart builder…** (§G7); the chart's kind (affine or nonlinear). When the chart is the shape sphere,
  the section also shows the **projection selector** (equirectangular, or an equal-area alternative) and the **hemisphere
  toggle** (one hemisphere, or both with the redundancy flagged) — `principia_chart_reference.md` §3.3 (R-113).
- **Navigate:** centre `(u, v)`; zoom (log₂); all eight `z₀` values, editable by drag or by typing.
- **Depth readout** (`2^-k`, quad level) **and a precision warning**. The warning is **raised by events** (R-54):
  `DECODE_SWITCHOVER` firing on visible quads, and `AT_F32_FLOOR`. It is never tied to fixed depths. The artboard's readout
  is "f64 · 7 digits left · linearise decode".
- **Lock:** a "● locked at z_locked" badge with **unlock**. Sliders are **re-based to the anchor, not frozen** (§G4).
- **Centre z₀:** eight sliders, `z_α, z_β, z_q0…z_q3, z_μ1, z_μ2`.
- **Slice & tilt:** the slice step, the tilt angles `τ₁`, `τ₂`, and the rotation `γ`.

**Figure.** Axis labels carry the short axis name and the range at each end. The orbit's path is drawn from the cursor's
IC:
- **The path is `z(t)` through the manifold, projected onto this chart's `q₁`, `q₂`** — not real space. This is
  `principia_trajectory_viewing.md` §3's hover trace (the full CPU state, `computeIC`, projected onto the chart's actual
  basis). It is **solid near the slice and faint where it leaves the plane.** The notes add that it is exact in the
  shape-sphere `(θ, φ)` chart, where the chart's coordinates are the shape itself.
- **The hover label names a fate only once it's decided.** Before that it reads "undecided · still interacting at `t` of
  `t_max`". A decided fate is a terminal `state` (escape, collision) or `bounded` at the horizon (payload §2); its labels
  use 0-based bodies and pairs (R-22).

**Right: the Trajectory panel,** for the IC under the cursor or a kept orbit (tabs: "under cursor", `#1`, `#2`, `+`):
- a one-line summary (fate, `t_end`, FTLE, `E₀`);
- real space (CoM frame) and the shape sphere side by side. **The shape sphere turns slowly, with visible x, y, z axes,** and
  switches to an unwrapped (equirectangular) view; a "turn" checkbox stops it;
- the F₂ word, substeps, minimum separation, `|ΔE/E|`;
- a playhead for this orbit;
- **listen**: sonification (`principia_scratchpad_pointer_channels.md` §4, its one normative part — R-109; `θ(t), φ(t)` → spectrum), which can follow the
  cursor. The mapping is the corpus's; the artboard's selector ("separations → pitch") is illustrative (R-68);
- **Open full viewer…** and **IC Inspector…** (both open §G8);
- **Kept orbits** below, each with its fate and time, removable.

**Bottom.**
- **Compass** (the nav cube), bottom left under the view controls. It shows the slice plane inside the chart and **switches
  mode by itself**: touching a slice slider shows slicing, and touching a tilt shows tilting. When locked it carries a gold
  pin at the pivot, and the plane turns about the pin (§G4). Dragging the plane tilts; dragging the cube orbits. It reads
  out the tilt and rotation angles.
- **Time:** play, step, a scrubber, speed. Transport (play / pause / speed / loop) is `ViewUI` state: not undoable, not
  on the sim key (R-96). The GUI's clock advances `RenderState`'s playhead each frame through a `SetField` marked "no
  history"; a manual scrub is one coalesced undo entry (R-101). **Scrubbing back re-integrates** to that time, so the figure refines
  progressively. It is not instant, and it says so ("refining · 72%"). The scrubber sets the display time and never replays
  stored frames; the export contract's "no scrub" applies to exported animations only (R-66).
- **Legend, generated from the stain** (§G6).

**Footer.** Warning and error counts, the latest message, memory (GPU, heap), and "? keys". Clicking it opens the console
(§G12).

**Run settings are NOT on the page** (they're rarely changed). Horizon, integrator, escape settings, quality, budget, and
recompute / cancel live in the Run window (§G5).

## G3. Keyboard — a design note, not a screen (`07_keyboard.png`)

The GUI is a tree of scopes. The big scopes, in Tab order: 1 top bar · 2 Manifold view · 3 Figure · 4 Trajectory ·
5 Compass · 6 Time · 7 Legend. Manifold view's sub-scopes (Chart, Navigate, Centre z₀, Slice & tilt) are reached with Enter.

| key | action |
|---|---|
| Tab / Shift+Tab | next / previous big scope, in the numbered order |
| Enter | into the focused scope |
| Esc | back out one level |
| arrows | between siblings; adjust a focused value |
| Shift · Alt | ×10 · ×0.1 steps |
| held keys | delay, then repeat (the DAS / ARR model) |
| Ctrl+Z | undo, from the contract's history (R-52) |
| ? | shortcuts, over everything |

In scope: Figure — arrows pan, + / − zoom, Space keeps the orbit, L listens, K locks. Trajectory — Enter reaches the
playhead, listen and kept orbits. Compass — arrows tilt, Shift+arrows orbit. Time — Space plays, ← → step. Legend is
read-only.

What the user sees: a focus ring on the current scope and the breadcrumb in the top bar (e.g. "Manifold view › Navigate ›
zoom"). Nothing else changes on screen.

## G4. Lock — the reticle and the pin (`08_lock.png`)

Locking (K, or right-click → lock here) recentres the view on that point and marks it with a gold reticle at the centre.
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

### Profiler

Tabs: Timeline, Flame, GPU, Memory, Counters; live / pause / Capture. It shows:
- the frame-time trace, with frame ms, p50, p95, p99 and the worst frame;
- where the frame goes (a donut), stacked per-frame bars for the last 60 frames;
- a substeps-per-pixel histogram, with the cap marked (the long tail near close encounters is the GPU divergence);
- a flame chart of nested scopes on the main thread; GPU timestamps per pass;
- memory (heap, GPU, tile cache) over time; live allocations by type, with their change over 60 s;
- a **leak detector** that flags steady growth while idle.

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
Scenarios are deterministic. Buttons: Export trace (JSON), Open in Tracy, Headless render….

### Export & share

- **Image:** size (multiples of the view), format; **embed the view (pxpack)**, and optionally **the stain's WGSL**. The
  picture carries its own settings, so opening it recreates the view exactly (§G9).
- **State:** copy snapshot (JSON), save snapshot, load; a share link (`principia://view?…`). The spec is the object and the
  picture is its shadow (`principia_export_animation_contract.md` Part 6).
- **Present:** hide all chrome; Esc returns.

### Display — the last stages

**Order is fixed (R-67):** SimState → stain → style → display scale → gamut clamp → colour-vision simulation → screen (R-111). The stain colours the data, the style draws it,
the display scale and gamut clamp finish it, and colour-vision simulation shows how the finished picture is seen: the
simulation sees the final in-gamut colours.
- **Style** is optional and applies to the figure only. Scientific checks run with **plain**. Presets: plain, watercolour
  & pencil, print · Poster78, more; with paper grain and press misregistration.
- **Colour-vision simulation:** off, deuteranopia, protanopia, tritanopia.
- **Overlays:** grid, class edges, `t_end` contours, cursor crosshair.

The display stage stays global and outside the pipeline (Part II §12). This window and the Overlays menu (§G2) replace the
corpus's top display bar (R-67).

### Run — from the top bar

Rarely changed, so it lives in a window, not on the page. Every field is a `SimConfig` field (gui_state_contract §2).
**The Run window exposes the parameters the contracts define, under their contract names (R-68).**
- **Integration:** integrator_contract Part 3's parameters — `T_horizon` (physical time, `∈ [50, 200]`, Part 5),
  `dt_macro`, `N_max` (default 64), `r_sub` / `gamma_sub`, `r_coll` (a user-exposed sim key, Part 7), `r_close`,
  `eps_E` / `eps_L` — and the integrator occupant (stepper × regularisation; Heggie with KDK leapfrog is the general
  default, Aarseth–Zare is kept for benchmarks; Part 2b).
- **Escape:** the criterion is shape closure + energy sign (R-29): `tau` and the escape window (0.4 time units,
  provisional).
  There is no persistence count (change 11).
- **Refinement:** quality (the preset selector — Auto, named tiers, Custom; gui_state_contract §6), frame budget (ms), max
  depth (`MAX_REL_DEPTH`), ensemble `E` (samples per pixel).
- **Recompute** and **Cancel**, with progress.

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

It's called **"Legend"**, never "Fate". The outcome legend uses colour_composition §1.4's canonical palette (R-77) and 0-based
labels (R-22).

## G7. Chart builder (`03_chartbuilder.png`)

- **Each axis has a kind:** latent direction (any mix of the eight `z` components, normalised; a normalise button shows
  `|q|`), physical quantity (energy, `L_z`, virial ratio, mass ratio…, with a range and what is **held fixed**, e.g. "z₀ along
  the other 7" — the kind-2 residual convention, chart_decoder_contract Part 3), or Burrau dimension (Euclid `(m, n)` → the
  primitive triple, and the neighbourhood swept, e.g. mass × momentum).
- **Presets are saved pairs of axes and are editable:** Save, Duplicate, Delete.
- **A physical-quantity axis makes the chart nonlinear (Φ).** Pixels map through Φ, then the decoder. Lock replays Φ and the
  decoder on the CPU instead of the affine `z₀ + s·q₁ + t·q₂`.
- **The Domain preview:** the chart's admissible region in its own coordinates, the forbidden region hatched, the current view
  as a rectangle, the boundary's formula, and "forbidden in view: N%". **Each chart supplies its domain function** (R-26:
  `validate(u, v)` on the `Chart` trait).
- **A quick render** (e.g. 64 × 64 at a short horizon), at the view's aspect. **Both previews are square.**
- The footer checks `q₁ · q₂ = 0` and lists the hidden directions (and the residual). Revert / Apply.

## G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)

**The IC Inspector and the trajectory viewer are ONE window (R-65).** trajectory_viewing §4's panels are hosted here and in
Explore's Trajectory panel (§G2); trajectory_viewing §4 says where each lives. The standalone IC Inspector tool is absorbed;
its HTML (`docs/gui/reference/ic_inspector.html`) is prior art.

- **Pane 1, the IC:** drag bodies (0, 1, 2) and their velocity arrows, with ghost markers at the playhead. **Right-click a
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
- **Saved views:** pxpack snapshots, each with a thumbnail and "go"; "Save this view".
- **Record a time sweep:** a time range, a frame count, the quality each frame is refined to, a size at the view's aspect,
  GIF / PNG frames / MP4, overlays on or off, pxpack in every frame. **Recording integrates each frame to its own `t`**, so
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
- **Continuation** along a parameter (e.g. a mass ratio), with a step, marking folds where stability changes.
- **Poincaré return map** on a chosen section, for a kept orbit.
- **Side by side** with a linked cursor and navigation, and a difference view. The linked views are a separate
  `ViewUI` item, specified with these v2 research tools (R-106) — not the chart's link ids in `SimConfig`.

## G12. Console (`12_console.png`)

The footer, opened: severity, time, source, message; filters (all, warnings, errors, info, text); copy and clear. It is the
same stream as the profiler's telemetry. Errors open it automatically. Sources include the stain, the integrator, the
quadtree, the contract (each `SetField` is logged) and the app.

## G13. Where the artboards are overridden

The pictures are layouts, and **their values are illustrative: corpus values win, and the Run window exposes the parameters
the contracts define, under their contract names (R-68).** Where a detail in one differs from a ruling, the notes or the
corpus, the spec follows the authority:
- **Body and pair labels** are 0-based (R-22). The artboards show "body 1 escapes", "collision 1–2", "body 3 crosses".
- **Escape has no persistence count** (R-29, change 11). The Run artboard shows "persistence 8".
- **The profiler's top level is telemetry §2's five stages** (R-56). The donut shows other categories.
- **No camera** (notes, G1). The Research artboard shows "linked camera", and the console shows "SetField CameraZoom".
- **"Legend", never "Fate"** (notes). The Stain artboard shows "Time of fate", and the Display artboard shows "fate edges".
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
7. The compass switches mode by itself; lock re-bases the sliders and pins the compass.
8. The legend is generated by evaluating the stain, with one key per dimension.
9. Keyboard: a tree of scopes, a focus ring and a breadcrumb, nothing else on screen.
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
- **`DEBUG_NAN : vec3<f32>`** — the reserved invalid-pixel colour (the validity-first invariant, §13):
  a NaN reads as "no data", never as a value.

Each numeric debug field therefore generates a two-line `colour()` — the NaN guard, an exact **bitcast comparison** of
`raw` against the canonical quiet-NaN bits that returns `DEBUG_NAN` (`principia_render_contract.md` Part 2's rule; never a
self-comparison or `isnan()`, which fast-math may fold away — R-114), then `ramp( range_norm(raw, lo, hi, RANGE_AUTO, u_range) )` — where `RANGE_AUTO` is the
fixed↔auto flag, editable identically in the node inspector, on the node in the graph, and in the
code (§9, §10). **Debug fields are raw** — the stated exception to §13's validity-first rule (R-79): apart from the NaN
guard there is no validity masking — a failed-state sentinel (e.g. `0.0`) is shown as its literal value, cross-checked
against the raw `state` field, not silently recoloured. NaN still goes to the invalid colour.

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
  invalid-pixel colour — a NaN / sentinel must read as "no data", not as a value (composition spec
  §3, §6; R-79). Debug fields are the stated exception: they show literal stored values, and NaN still goes to the
  invalid colour (§10.1).

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
- **Standing composition-spec gaps** (to reconcile on that doc's next pass, tracked in
  `principia_colour_composition.md`): colour-source-as-axis, overlays-as-post-chain-over-configured-
  base, physics-as-overlay-op, gradient-unifies-the-ramp, per-footprint vs quad-aggregate spread,
  the debug taxonomy, the addressing channel convention (UV=RG / TL=RB / centre=BG / index=grey), and
  boundaries-as-overlay. This GUI spec assumes those resolutions.
