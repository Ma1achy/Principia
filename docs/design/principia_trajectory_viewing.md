# Principia — trajectory viewing (hover trace + click inspector)

*Supersedes the render-contract Part 7 hover mechanics and the earlier inspector-surface draft's tiering. One rule underneath both features: **hover or click integrates that IC on the CPU (f64, `computeIC`); the two features differ only in what you project the trajectory onto and where you draw it.** No tiering, no block-span analysis — a single three-body trajectory is cheap, CPU-side, and off the GPU survey's back. Under lockstep (temporal note, ratified) `SimState` holds no trajectory at all — only live state — so the CPU integration was always the only honest source for a trace.*

---

## 1. The single mechanism

```
on hover / click over pixel →
    uv  = flip_y(pointer_to_uv(event))       // Y-up convention: v = 1 − event.y/H
    IC  = decode(chart, uv, simKey)          // the CPU decode
    traj = computeIC(chart, uv, simKey)      // one f64 integration, full state r(t), p(t)
    project(traj) and draw
```

**Picking honours the single Y-flip (coordinate note).** Pointer events are Y-down top-left; they must be flipped to the post-flip Y-up UV *the same way the render does* before decoding — else the classic wrong-subsystem bug (clicking the top of the screen inspects the bottom of the manifold). The picking cross-check debug view (debug plan §F) certifies this path agrees with the render flip.

`computeIC` is the Precision-ring path already built for parity and used by nothing new here — trajectory viewing is its third consumer. Because it returns the **full state** at every step, it supports **any** projection: onto the current chart's tilted plane, onto the shape sphere, into real space. That is why there is no case analysis — the full state can be projected any way; the survey's `SimState` (live state only, no history under lockstep) never could.

Responsiveness: cancel-on-move by generation counter, as everywhere.

> **SUPERSEDED — see `principia_scratchpad_pointer_channels.md` §3.** "One integration per settled
> hover" was too conservative. Measured: one trajectory is **6.1 ms on a single core at `t = 50`,
> 1.6 ms at `t = 13`** — it fits inside a 60 fps frame and can run *per frame during motion*, not
> only on settle. The budget exists for the **tail** (`total_substeps` is bimodal, ~100× p1→p99), not
> the average: give the hover a **step budget**, and if it exhausts it, **report what it got and mark
> it incomplete** rather than reducing quality. Dwell releases the bound. Same degrade-during-motion,
> converge-on-rest pattern as the coarse-ancestor fill.

There is also a **third consumer of the same integration: sonification** — `θ(t), φ(t)` → spectrum →
audible range. The trace and the sound are the *same measurement*: the loop drawn on the shape sphere
IS `θ(t), φ(t)`, and the sound is its spectrum. Measured, figure-eight vs chaotic: **spectral entropy
0.076 vs 0.177**, one line with harmonics against four comparable peaks. They fail complementarily —
a dense winding is visual mush and a clear high note. Scratchpad §2, §4. It runs in the dedicated inspector worker (separate from the render-loop worker — caching Part 6a, render Part 7); it never blocks the GPU survey, the frame loop, or the main thread.

---

## 2. Two features = two projections of the one trajectory

| | **Hover trace** | **Click inspector** |
|---|---|---|
| Where drawn | **overlaid on the survey** you're already viewing | a **separate multi-panel surface** |
| Projected onto | the **current chart's own `(q₁, q₂)` plane** | **absolute** shape sphere + real space |
| Answers | "how does this IC move *through the space I'm currently viewing*" | "what *is* this trajectory" (shape + real, in absolute terms) |
| Weight | light — one overlay | heavier — three linked panels |
| Trigger | hover (transient) | click (persistent until closed) |

Same integration; different lens. They're complementary, not redundant.

---

## 3. Hover trace — projection onto the current (tilted, sliced) plane

The trace at each step is the trajectory state projected onto the chart's **actual basis vectors**:

```
trace_x(t) = project(state(t), q₁)        trace_y(t) = project(state(t), q₂)
```

— i.e. run the state through the chart's forward map and read where it lands in the `(q₁, q₂)` plane. **Coordinate-free in the way that matters:** it never asks whether `q₁` is "pure θ" or "0.6·θ + 0.8·mass" — it projects onto the vector, whatever tilting made it. "Combination of axes" is the *general* case; a single clean axis is the degenerate one.

Two behaviours follow, and both are diagnostic value that exists *because* the axes are combinations:

- **Tilt deforms the trace, live.** Rotate a conserved direction (`E`, `L_z`) toward a live one and the trace goes from a **dot** (conserved axis, no motion) → a **line** → a **curve**, as the live component's share grows. The trace's motion along a tilted axis is a direct visual readout of *how much of a live direction is mixed into that axis*. A conserved component contributes exactly zero; the live component contributes its projection. (This is why the projection must be onto the real `q`, never onto "the axis's dominant named coordinate" — the shortcut would freeze a trace that should move.)
- **Slice repositions the trace, live.** Slicing moves `z₀` (the plane's *position* in the hidden dimensions) without changing its orientation — so the same IC's trace shifts as you slice, because you're projecting onto the same plane anchored at a different offset. Tilt changes the plane's orientation; slice changes its position; the trace responds to both.

Works in **any** chart, any tilt, any slice — uniformly, because the full CPU state supports the projection regardless.

---

## 4. Click inspector — the three-panel absolute surface

Opens on click; shows the **same trajectory three ways at once**, linked by one time cursor. Absolute coordinates (shape + real), not the current chart's axes — the complement to hover.

| Panel | Shows | Source | Comparison? |
|---|---|---|---|
| **3D shape sphere** (rotatable) | `n(t)` polyline on S², landmarks (3 BC equatorial, Lagrange poles) | `computeIC` trajectory → shape | **yes** (§5) |
| **2D UV unwrap** | same `n(t)`, equirect (θ, φ): θ azimuth horizontal, φ polar vertical (R-14) | same | **yes** (§5) |
| **Real space** | the three bodies in the plane, `r_i(t)` | `computeIC` trajectory | **no** — CPU-f64 sugar |
| **Scalar readout** | `t_end_step`, `state`, escaper, `d_min`, drifts @ cursor | `SimState` + `computeIC` | classification-level (§5) |

**Where each panel lives (R-65).** The click inspector's panels are hosted in the **Inspector window**
(`principia_render_gui_spec.md` §G8), which absorbs the standalone IC Inspector, and in **Explore's Trajectory side panel**
(§G2):

| Panel | Inspector window | Explore's Trajectory panel |
|---|---|---|
| 3D shape sphere / 2D UV unwrap | pane 2, one pane toggled "sphere / unwrapped" (it can also show the canonical bodies) | beside real space, the same toggle |
| Real space | pane 3 | beside the sphere |
| Scalar readout | the readout row (latent `z`, `α, β`, masses, `E · L_z`, outcome, F₂ word, round trip, gauge) | the summary line and the readouts (F₂ word, substeps, min separation, `\|ΔE/E\|`) |
| *the IC itself, editable* | pane 1 (from the absorbed IC Inspector) | — |

The 3D sphere and the 2D unwrap now share one pane and are toggled; they stay in the same config-space frame, so they still
agree. The sphere turns slowly, with visible axes, and can be stopped ("turn").

- **Real space is CPU-f64 intuition sugar** — "see what this IC looks like", no comparison claimed. There is no GPU real-space trajectory to compare against (the survey never produced one), and that's fine — visualisation, not parity.
- **Config-space frame (both sphere panels).** Both feed `sph_uv` in the config-space frame (two-rotations rule). The 3D orbit control rotates the **camera, not the data** — so the 3D view and the 2D unwrap always agree. Landmarks fixed in the same frame.
- **Linked brushing.** One `ViewUI.t_cursor` shared across panels: scrub in one → all highlight the same instant; hover a sphere point → real space jumps to that configuration. Pure `ViewUI` state, no engine involvement. This is what makes three panels one instrument. The inspector's cursor is independent of the survey's global playhead (it examines one frozen IC).

---

## 5. What is honestly comparable (click inspector)

The one comparison worth drawing — and the reason it's honest — is on the shape sphere, because it's the only place the GPU and CPU both produced the same quantity.

- **Shape divergence overlay (optional; off by default, on for the curious and paper figures).** On opening the overlay, dispatch an **on-demand single-IC f32 GPU trace**: a one-pixel compute job re-running the clicked IC through the *survey kernel* (the same Rust-sourced compute kernel at f32 — in the browser it reaches WebGPU as WGSL exactly as the survey does, via `rust-gpu → SPIR-V → naga`) and writing a dense `n(t)` series — one trajectory's history is trivial memory, so the no-history discipline is not violated (it is a sanctioned pull, sized like the hover pull). Draw it over the CPU f64 curve, sampled at shared times. Coincident → agreement; peeling apart → divergence, and the peel-off time reads the local Lyapunov time. The shape sphere is **bounded**, so this is cleaner than a real-space comparison would be: escaping real-space paths both shoot to infinity and swamp the signal, whereas on S² the shape trajectories stay compact and genuinely separate. *(Replaces the old checkpoint-skeleton overlay — checkpoints no longer exist; this is a fair f32-vs-f64 comparison of the same kernel, on demand.)*
- **Classification agreement.** Both report outcome class / escaper / `t_end`. In regular regions, agreement certifies the pipeline (disagreement = a real bug). Near a boundary, outcome disagreement is the honest signal that *that pixel's fate is genuinely uncertain* (the two-floor "numerically suspect" story).

**The reading: slightly more trustworthy, not more correct.** In regular regions the CPU integration agrees with the map and certifies it. In chaotic regions CPU (f64) and GPU (f32) diverge exponentially — as any two numerical integrations of a chaotic system must — and neither is the true path; the divergence measures local unpredictability. The inspector certifies the tool where the physics is predictable and reveals the unpredictability where it isn't.

**Footnote (tool + paper), honest form:** *The click inspector integrates each trajectory independently on the CPU in double precision. Where regular, this agrees with the map to high accuracy; where chaotic, the CPU (f64) and GPU (f32) trajectories diverge exponentially — as any two numerical integrations of a chaotic system must — and neither is the "true" path. The divergence is itself a measure of local unpredictability.* (Not "more accurate, minor differences" — that falsely privileges the inspector.)

---

## 6. Placement

- **No new sim machinery.** Both features compose `computeIC` (Precision ring / parity) + the chart projection (hover) or the shape-sphere widget + equirect + real-space draw (click).
- **GUI/state:** both are `ViewUI` surfaces reading the engine only via `computeIC(chart, uv, simKey)`; the engine never knows they exist. Hover overlay + inspector panels + `t_cursor` are all `ViewUI`.
- **Hosts (R-65):** the Inspector window (render_gui_spec §G8) and Explore's Trajectory side panel (§G2). The standalone IC Inspector tool is absorbed into the Inspector window; its HTML stays in `docs/gui/reference/` as prior art.
- **Supersedes** render-contract Part 7's hover tiering table and the earlier inspector draft's block-span tiering — replaced by "always integrate the IC on CPU, project, draw."

---

*One integration, two lenses. Hover projects it onto the plane you're looking through — tilt deforms it, slice repositions it. Click projects it onto absolute shape and real space, three panels, one cursor. Real space is sugar; the shape divergence is the honest comparison. Always CPU, no tiers — a single trajectory was never worth optimising.*
