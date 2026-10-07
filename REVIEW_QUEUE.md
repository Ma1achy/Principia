# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

*Found by the physics review of PR #158 (TASK-M1-11, review 5436669140), raised first by qa there. Nothing is chosen.*

## RQ-226: θ̃'s frozen reference and its has-reference flag have no storage across a resumed march *(physics, ledger, R-389, R-392, TASK-M3-11, REQ-INT-082)*

- **File, section:**
  - `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep,
    ratified)", R-389's paragraph: "on entering `√(n_u² + n_v²) < r_pole`, the last longitude (the last one outside
    the radius) is stored; no delta is added while inside; on exit, `wrap(exit longitude − stored longitude)` is
    added", and R-392's: "An IC that starts inside the radius has no stored longitude, and **its first exit adds no
    delta**".
  - `docs/design/principia_dd_simstate_payload.md` § 2, on resumption: "on a **cached-state resume** (a quad
    continuing its march from cached `SimState` at `t_cached`, which lockstep does)".
  - The same file, § 1 and § 2, on the free space: `_reserved : u16, // free under alignment (4 B and 8 B cost the
    same); do not spend`, and descriptor bits 10–15: "Reserved, decode as zero, never opportunistically reused".
    `SimStateFTLE` is "144 B actual / 144 B effective".
- **The silence:** the corpus says the longitude is "stored" but gives it no slot. The hold needs two values that
  persist between macro-steps, the frozen longitude and a has-reference flag (set outside the radius, clear for an IC
  that starts inside). The previous shape point can be recomputed from the stored positions; these two cannot. Kept
  only in kernel locals, they are lost at a cached-state resume, so θ̃ would depend on how the march was split into
  dispatches. Every place to keep them is protected by the payload document, so any choice is a ledger change.
- **Options** (from the physics review, which prefers (a)):
  - **(a)** A new f32 `theta_ref`, plus the flag as a versioned assignment of descriptor bit 10. `SimStateFTLE` goes
    from 144 B to 152 B (+8 B per sample with padding). Implements R-389 exactly.
  - **(b)** The reference as a u16 longitude in `_reserved` (steps of about 9.6e-5 rad), plus the bit-10 flag. No
    size change, but the exit delta and the exact-π case become quantised.
  - **(c)** Rebuild the reference from θ̃ and the longitude θ̃ counts from. It still needs storage for R-392's
    IC-inside case, and it changes R-389's arithmetic.
  - **(d)** Drop the hold at dispatch boundaries. Physics rejects it: θ̃ would depend on the dispatch split.
- **Needed:** a ruling on where the two values live, before TASK-M3-11 (REQ-INT-082) runs θ̃ in the kernel's march.
  TASK-M1-11 computes θ̃ in a single call and is not blocked.

*Found by the physics review of PR #160 (TASK-M1-07, review 5438322479). Nothing is chosen; it goes to the human with
REQ-TOOL-152's calibration at the M1 gate.*

## RQ-242: REQ-TOOL-019's "no banding" has no depth range, and an f32 u bands at depth *(physics, REQ-TOOL-019, REQ-TOOL-152, REQ-DEC-031, TASK-M1-07, M5)*

- **File, section:**
  - `plan/requirements.yaml`, REQ-TOOL-019: "The UV preset … must reconstruct each sample's UV coordinate as deep_zoom
    §1 writes it, u = c_u + h_u·(2t − 1) and v likewise, … for sampled quads with no banding (adjacent-sample deltas
    smooth, not step-quantised, by the criterion REQ-TOOL-152 calibrates)."
  - `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision": "Within-quad precision is full
    f32 at any depth."
  - `plan/requirements.yaml`, REQ-DEC-031: "… the GPU must compute sample positions as u = centre + half·(2t − 1) with
    t = (i + 0.5)/N, never from the quad's min/max bounds." REQ-SCHED-067 uses the same centre-plus-half-width pattern.
- **What was measured (PR #160):** on M1's flat grid every form is exact (departure 0). At depth 30, the f32 sum
  c + h·(2t − 1), which is the coordinate view and the form REQ-DEC-031 names, bands like the global form (departure
  1). c (f64) + δ stays exact (departure 0). qa's sweep (N = 6) shows the f32 sum's departure doubling per level
  (N·2^(ℓ−24)), so at the proposed bound of 1/16 it reads "banded" from about ℓ = 19, just inside ℓ_switch = 20 (R-90).
  §1's "full f32 at any depth" holds for t and δ, but not for a u formed in f32. The decoder is correct only because
  §2 consumes δ, never u.
- **The question (physics; it changes what M5 builds):**
  - (a) Does REQ-TOOL-019's "no banding" apply to an f32 u only down to the depth where REQ-TOOL-152's bound is first
    exceeded, or to the c (f64) + δ path at every depth?
  - (b) Is §1's u carried on the GPU as (c, δ), and never as a global f32 u, past ℓ_switch? That would bear on
    REQ-DEC-031's wording, REQ-SCHED-067 and TASK-M5-04's per-quad uniforms.
- **Not blocked:** TASK-M1-07 meets REQ-TOOL-019's verify on M1's flat grid. The answer shapes M5.

*Found by the physics review of PR #160 (TASK-M1-07, review 5438875182), with a point added by its code review
(5439037360). Nothing is chosen; it goes to the human.*

## RQ-257: `ctx.chart.slice_uv` has two readings, the position in the view or in the slice plane *(physics, R-97, TASK-M1-06, TASK-M2-25, REQ-COL-057)*

- **File, section:**
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract", the chart row of the lane table: "`slice_uv`
    (vec2 in [0,1]²), `z` (the full 8-D latent at this pixel, chart triple applied), `chart_id`". That is all the
    corpus says of the lane. (The phrase "on a flat grid tiling the slice" is from a qa test comment, not the docs.)
  - `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for
    its job)", the UV row: "**UV / quad addressing** | `[0,1]²`, **unsigned** | bottom-left, **Y-up** (post-flip) the
    sample position *in the current view*, which chooses the quads asked for; the quad identity `(depth,tx,ty)` and the
    quadtree are taken in the **slice plane's own frame**, relative to the plane anchor (`z₀` at the last
    re-integrating event) — pan and zoom change which addresses are requested, never the addresses (R-97)"; and below
    it, "`z(s,t) = z₀ + (2s−1)·q₁ + (2t−1)·q₂        # the chart placement (chart/decoder contract)`", with "UV is the
    unsigned `[0,1]` **address**; converting it places it as a **signed offset from the chart centre**, scaled by
    zoom."
  - `decisions.md` § "R-97 — Quad addresses live in the slice plane *(closes RQ-57)*": "Quad addresses are
    `(level, i, j)` in the slice plane's own frame, relative to the plane anchor: `z₀`'s value at the last
    re-integrating event (R-92). In-plane pan and zoom change which addresses are requested, never the addresses
    themselves. Conform deep_zoom §1."
  - `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision": "`u = c_u + h_u · (2t − 1)
    t = (i + 0.5)/N   (quad-local sample coord)`", where "The CPU (f64) computes per-quad **centre `c_u`** and
    **half-width `h_u`**".
- **The code:** TASK-M1-06 (merged) fills the lane in `crates/render/src/bind.rs` as
  `(vec2<f32>(r.quad_xy) + r.quad_uv) / vec2<f32>(ctx_uniforms.quads)`, the grid tiling (lines 267–271 on main at
  844b27f; lines 273–277 on PR #160's head 613cfda, unchanged by it). No TASK-M1-07 deliverable or acceptance test
  reads it; the UV preset, the δ mode and `uv_preset_reconstruction` use `ctx.quad.*` and `ctx.screen.uv`.
- **The two readings** (from the physics review, which chooses neither):
  - **(a) The position in the current view.** The coordinate note's UV row defines UV as "the sample position *in the
    current view*", and the chart placement z(s,t) = z₀ + (2s−1)·q₁ + (2t−1)·q₂ maps it, "scaled by zoom". Under this
    reading the grid-tiling formula is correct for the harness, whose grid is the view.
  - **(b) The position in the slice plane.** R-97 puts quad addresses "in the slice plane's own frame, relative to the
    plane anchor", and deep_zoom §1's u = c + h·(2t − 1) is the sample's coordinate in that frame. Under this reading
    the lane must be `centre + half_width·(2·quad.uv − 1)` (REQ-TOOL-019 calls this "each sample's UV coordinate").
  - The lane's name ("slice") and the existence of a separate `ctx.screen.uv` point toward (b); the coordinate note's
    UV row points toward (a).
- **Where they differ:** on any set that does not tile the slice from its origin. On `flat_at(.., 5, [7, 2])` (3 × 3
  quads at depth 5 from origin cell (7, 2)), (b) gives the quad in column i a centre u of (7 + i + ½)/32, and (a)
  gives (i + ½)/3. They agree only when the grid is exactly the slice's cells at its depth from the origin.
- **Why it is the human's:** the docs do not define the lane, so neither formula can be required, and calling the
  present one correct would choose (a). The choice decides where `ctx.chart.z` is placed per pixel in M2, so it is
  physics that changes results (CLAUDE.md "When to ask the human"). It is not TASK-M1-07's to define under R-72: the
  lane is TASK-M1-06's, and no TASK-M1-07 deliverable needs it.
- **No test pins the lane where it matters** (code review 5439037360): qa's TASK-M1-06 test
  (`crates/engine/tests/qa_TASK-M1-06.rs`, `ctx_lanes_qa_tile_and_quad_uv_are_y_up_and_exact`) now uses a square 4 × 4
  grid at depth 2 from the origin, where (a) and (b) agree. No test checks `slice_uv` on a non-square or offset grid,
  so a swapped width and height would go unnoticed. On a non-square grid the frames cannot tile the slice at any
  depth, so no such fixture has one expected value until the meaning is settled.
- **Needed:** a ruling on which space `ctx.chart.slice_uv` lives in, written into colour_composition §3. Then: the
  lane conformed to it if (b), and a test of `slice_uv` on a non-square, offset grid (such as `flat_at(.., 5, [7, 2])`
  with unequal columns and rows) added with the ruling.
- **Waits:** TASK-M2-25, the first task whose fragment presets read `ctx.chart.z` per pixel (the DECODE and agreement
  presets, and REQ-COL-057's definition of `ctx.chart.z` in colour_composition §3). At M1, `ctx.chart.z` is the
  screen's single z (RQ-216, REQ-GUI-001's note), so nothing in M1 waits. TASK-M1-07 is not blocked.
