# Dev GUI (egui) — design notes for the artboards

*Written 25 Sep 2026 by the reviewer, from the design sessions. The PNGs show the layout; this file holds the decisions
the pictures can't. Where the two disagree, this file wins. Where either disagrees with a ruling in decisions.md, the ruling wins.*

## Rules that hold everywhere

- **egui is a toggleable debug layer (F3) over the wgpu render.** It is the dev GUI, not the final UI, but it has the same
  structure and every feature.
- **Contract first.** Every control reads a `Snapshot` and sends a typed `SetField`. Nothing touches simulation internals,
  and data flows one way (UI → SetField → core → snapshot → UI). Undo and redo live in the contract (GU-1).
- **Navigation is chart construction.** There is no camera object: pan, zoom and slice edit z₀ and the basis
  (chart_decoder_contract rule 6).
- **Every preview of the slice keeps the viewport's aspect ratio** — square for the slice. No preview may stretch or crop it.
- **The figure is never covered.** No toasts or labels over the plot. Warnings go to the footer console.
- **Dark default egui theme, Ubuntu / Ubuntu Mono.** It's a dev tool; no styling beyond egui's own.
- **Not in the dev GUI:** Chazy subtitles, and the object-based stain desk. The Stain panel is the plain node-graph editor.

## 01 Explore — the everyday view

- **Top bar:** mode switch (Explore / Stain); Overlays ▾ (with a count of how many are on); Run…, Profiler…, Export…; the
  keyboard breadcrumb; status (t, fps, quads, undo depth).
- **Left: "Manifold view" is ONE group** — chart, navigation, centre z₀, slice and tilt, rotation. They are one thing: how
  you view the manifold.
  - **Chart:** preset (named by its axes, e.g. "z_α × z_β" — never nicknames), the two basis vectors, Chart builder…
  - **Navigate:** centre (u, v), zoom (log₂), all eight z₀ values editable by drag or typing.
  - **Depth readout, and a precision warning** raised by events (GU-3: `DECODE_SWITCHOVER` on visible quads, `AT_F32_FLOOR`),
    not fixed depths.
  - **Lock:** a "● locked at z_locked" badge with unlock. **Sliders are RE-BASED to the anchor, not frozen**: each shows
    anchor + offset, and moving one is a deliberate excursion.
- **Figure:** axis labels (short axis names, range at each end). The orbit's path is drawn from the cursor's IC.
  - **The path is z(t) through the manifold, projected onto this chart's q₁, q₂** — not real space. It's solid near the slice
    and faint where it leaves the plane. It's exact in the shape-sphere (θ, φ) chart, where the map IS the state space.
  - **The hover label names a fate only once it's decided.** Before that: "undecided · still interacting at t of t_max".
- **Right: the Trajectory panel,** for the IC under the cursor or a kept orbit (tabs). Real space and the shape sphere side by
  side; F₂ word, substeps, min separation, |ΔE/E|; playhead; **listen** (sonification, can follow the cursor); kept orbits below.
  - **The shape sphere turns slowly, with visible x, y, z axes,** and switches to an unwrapped (equirectangular) view.
- **Bottom:**
  - **Compass** (the nav cube), bottom left under the view controls. It shows the slice plane inside the chart and
    **switches mode by itself**: touching a slice slider shows slicing, touching a tilt shows tilting. When locked it carries
    a gold pin at the pivot, and the plane turns about the pin. Dragging the plane tilts; dragging the cube orbits.
  - **Time:** play, step, a scrubber. **Scrubbing back re-integrates** to that time, so the figure refines progressively. It
    is not instant, and says so.
  - **Legend, generated from the stain** (see 06).
- **Footer:** warning and error counts, the latest message, memory. Click to open the console (12).
- **Run settings are NOT on the page** (rarely changed): horizon, integrator, escape settings, quality, budget,
  recompute/cancel live in the Run window (04), under their contract names — there is no integrator "tolerance" field
  *(conformed to R-68)*.

## 02 Stain — the plain node-graph editor

Library (search, all modes, groups, your own stains; New, Rename, Import, Export). Graph with typed pins (field / colour /
brightness) and wires. Pipeline and per-node WGSL; problems with auto-recompile. Preview keeps its aspect. The node
inspector holds every setting the old dev GUI had.

## 03 Chart builder

- **Each axis has a kind:** latent direction (any mix of the eight z components, normalised), physical quantity (energy, L_z,
  virial ratio, mass ratio…), or Burrau dimension. **Presets are saved pairs of axes and are editable.**
- **A physical-quantity axis makes the chart nonlinear (Φ).** Lock then replays Φ and the decoder on the CPU.
- **The Domain preview:** the chart's admissible region in its own coordinates, the forbidden region hatched, the current view
  as a rectangle, and "forbidden in view: N%". Each chart supplies its domain function (CD-6). **Both previews are square.**

## 04 Windows

- **Profiler:** frame-time trace; where the frame goes (donut); stacked per-frame bars; substeps-per-pixel histogram; flame
  chart of nested scopes; GPU timestamps per pass; memory (heap, GPU, tile cache) over time; live allocations by type with
  their change over 60 s; a leak detector that flags steady growth while idle.
  - **For agents, the same data structured:** `prin profile --scenario … --json`, `prin profile diff base new --threshold`,
    `prin profile query … --live`. Schema v1 (GU-5): telemetry §2's frame record and five stages at the top, with nested scopes,
    GPU passes, allocations and events beneath. JSON. Leak flags and hot-path summaries precomputed.
- **Export & share:** PNG with pxpack (and optionally the stain's WGSL), snapshot JSON, share link, present mode.
- **Display:** fixed order — SimState *(conformed to R-111)* → stain → style → display scale → gamut clamp → colour-vision simulation → screen; the
  simulation sees the final in-gamut colours *(conformed to R-67)*. Style is optional and applies to the
  figure only; scientific checks run with plain.
- **Run:** as listed under Explore.

## 05 Inspector — one IC, its trajectory, one timeline

The IC Inspector and the trajectory viewer are ONE window. Pane 1, the IC: drag bodies and velocity arrows, with ghost markers
at the playhead. Pane 2: the canonical representative, bodies or shape sphere (turning, with axes, or unwrapped). Pane 3: the
trajectory in real space. **One timeline scrubs all three; editing the IC re-integrates the orbit live.** Reset, Randomise,
Burrau-ish, Rotate, Scale, Boost demonstrate the gauge quotient. Tabs: **Inspect** and **Create & locate** (build an IC, see
where it lands in z and its distance from the slice, then lock the view on it, centre a slice through it, mark it, or keep it).

## 06 Legend — generated by evaluating the stain

Walk back from OUT; sample each colour and brightness node over its declared input domain (GU-2); push through the real
combiner. **Each dimension gets its own key:** categorical swatches (with shares), then a separate bar for what brightness
(or any second channel) encodes. Number ramps get ticks placed through the curve; direction fields a sphere key; coordinate maps
their colour square; post operations line samples. It's called "Legend", never "Fate".

## 07 Keyboard — design note, not a screen

The GUI is a tree of scopes. Tab / Shift+Tab between big scopes (numbered order); Enter drills in; Esc backs out; arrows move
within a scope or adjust the focused value; Shift ×10, Alt ×0.1; held keys use the delay-then-repeat (DAS / ARR) model; `?`
shows shortcuts. What the user sees: a focus ring and the top-bar breadcrumb, nothing else.

## 08 Lock — the reticle and the pin

Locking (K, or right-click → lock here) recentres the view on that point and marks it with a gold reticle at the centre. Every
tilted plane passes through it, so it's the one point that stays still while the picture turns. The compass shows the same point
as a gold pin. Sliders are re-based (see 01). Affine charts compute z_locked directly; nonlinear charts replay Φ and D on the CPU.

## 09 Import picture · saved views · record a sweep

Drop a Principia PNG: read its pxpack, show what differs from now, offer restore / only its stain / open side by side. Saved
views as pxpack snapshots. **Recording a time sweep integrates each frame to its own t**, so it's exact, unlike scrubbing.

## 10 Measure — a tool on the figure

Drag a square region on the plot itself; sampled pairs show on it. The side panel shows the method (uncertainty exponent or box
count), the log-log fit and D. The resolution hazard is two buttons: a threshold sweep, and a matched N / 2N pair. **The
measurement path owns `FULL_RETENTION`** (uniform grid, every sample kept) — ruling PL-4.

## 11 Research — first pass (v2)

Periodic-orbit seeding from spiral cores (checked against the Šuvakov–Dmitrašinović catalogue); continuation along a parameter;
Poincaré return map; side by side with a linked cursor and navigation.

## 12 Console

The footer opened: severity, time, source, message; filters; copy and clear. The same stream as the profiler's telemetry.
Errors open it automatically.
