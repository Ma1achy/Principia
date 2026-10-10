# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-262: with F3 hiding the GUI, the figure fills the window at the same scale (R-406), but the corpus defines the view only on `(s,t) ∈ [0,1]²` and a nonlinear chart's `Φ` only there *(definition, physics, R-406, REQ-GUI-180, TASK-M8-05)*

- **File, section:**
  - `decisions.md` § "R-406 — With the egui layer hidden by F3, the figure fills the window, showing more of the field
    at the same scale rather than stretching; showing the layer returns the layout", the human's words: "It just
    renders extra tiles, so you can actually see further in every direction", "rather than, like, stretching".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts": "`z(s,t) = z₀ + (2s−1) q₁ + (2t−1) q₂
    (s,t) ∈ [0,1]²`" and "The `(s,t) ∈ [0,1]²` here is the **unsigned addressing space** of the current view (Y-up
    after the single framebuffer flip) — it chooses which quads are asked for".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Part 4 — Navigation is chart construction (pan, slice,
    zoom, tilt, lock)": "**there is no "view" separate from "chart."** The view state `(z₀, q₁, q₂)` *is* the chart",
    and Zoom: "`q₁, q₂` | common scale factor (log-stepped) | same plane, narrower/wider window".
  - `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)": "**UV / quad
    addressing** — `[0,1]²`, **unsigned, Y-up** after a single flip `v = 1 − frag_coord.y/H`. UV locates a sample *in
    the current view*". `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they
    nest; each is right for its job)" gives the same table.
  - `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch": "`fn map<F: Float>(&self, u: F, v: F)
    -> ChartOut<F>; // Φ : [0,1]² → chart space`", and § "2.1 Feasibility, and the warp that makes every pixel valid":
    "`K(t) = K_max · t^γ_K`" … "with `(s,t) = (u,v) ∈ [0,1]²`. **This maps the unit square onto the feasible
    interior, so no pixel is infeasible by construction**".
  - `docs/gui/principia_render_gui_spec.md` § "G1. Rules that hold everywhere": "**Every preview of the slice keeps the
    viewport's aspect ratio** — square for the slice." The shown layout's figure rect is 1056 × 977 of 01_main.png's
    2160 × 1350 pixels (`crates/gui/src/layout.rs`, from TASK-M6-24), not square either.
- **Silence:**
  1. **How the view extends past `[0,1]²`.** R-406 keeps the scale and adds the field around the shown rect in every
     direction (applied per R-369: each point keeps its screen position), so the hidden window's samples lie at `(s,t)`
     outside `[0,1]²` on both axes. The corpus says the view *is* `[0,1]²` of the chart. For the affine slice the
     placement formula extends as it stands; whether it is meant to, how UV is taken from the framebuffer when the
     viewport is not the square `[0,1]²` fills (`v = 1 − frag_coord.y/H` and its `u` partner assume the view spans the
     viewport), and whether the quadtree requests quads outside the depth-0 root (quad addresses are in the slice
     plane's own frame, R-97) are not given. The same question already stands, smaller, for the shown rect, which is
     not square.
  2. **What a chart shows outside its domain.** A nonlinear chart's `Φ` is defined on `[0,1]²` only: the invariant
     warp `K_max · t^γ_K` has no real value at `t < 0` for a non-integer `γ_K`, and its feasibility guarantee is for
     `[0,1]²`; the sphere and Burrau charts are bounded parameterisations. Whether the extra area outside the domain is
     drawn as the forbidden region (R-26's domain function, as the Overlays menu's "forbidden region" draws it), left
     the clear colour, extended by a per-chart rule, or not shown (the hidden view stopping at the domain's edge), is
     not given.
- **Options seen:**
  1. The view's `[0,1]²` stays where the shown rect puts it; outside it the window's samples take `(s,t)` beyond
     `[0,1]` at the same scale; the affine slice extends by its formula and the quadtree requests the extra addresses;
     a nonlinear chart evaluates `Φ` nowhere outside `[0,1]²` and draws that area as the forbidden region (hatched,
     R-26), so no pixel is ever infeasible.
  2. As option 1, but each nonlinear chart defines its own extension (clamped, periodic in longitude for the sphere, or
     none), a definition (R-72) written by the task that needs it and physics-reviewed.
  3. The hidden view fills the window only up to the chart's domain: the affine slice fills the window; a nonlinear
     chart's view stops at its domain's edge, the rest the clear colour.
- **Applied meanwhile:** on the mock (TASK-M6-30, REQ-GUI-179) the figure fills the window: the stand-in is noise
  defined everywhere, with no chart mapping, so nothing there waits. On the real engine, REQ-GUI-180 carries
  `rq: RQ-262`.
- **Waits:** TASK-M8-05's REQ-GUI-180 line (the real engine's fill with F3 off). Nothing else is blocked; memory and
  the frame budget need no new value (R-406's applied note).
