# Principia — coordinate conventions

*A boundary contract — how coordinates work, stated once, at the seams. This is foundational: it wants to exist before the decoder or the picking code is written, because both failure modes here (a mirrored Y, a confused addressing-vs-value space) are **invisible until they corrupt a specific feature**, and then the symptom shows up in the wrong subsystem and costs a day. The rule is: name the conventions, name the single flip, derive everything from the post-flip coordinate.*

---

## The one-line rule

**There is exactly one internal orientation — bottom-left origin, Y-up — and exactly one flip, at the framebuffer↔UV boundary (`v = 1 − frag_coord.y / H`), mirrored once at image export. Mouse picking, quad addressing, and the CPU/GPU quad uniforms all read the *post-flip* Y-up coordinate, so they agree by construction. A mirrored image is *always* a wrong-number-of-flips bug at that one seam — never a reason to add a compensating flip elsewhere.**

---

## Why this is a footgun: WebGPU has multiple disagreeing Y-conventions

- **NDC / clip space: Y-up** (+Y toward top — like OpenGL, unlike D3D/Metal-native). Already Y-up.
- **Framebuffer / texture coords: Y-down** — texel (0,0) is top-left; `@builtin(position)` (frag_coord) is framebuffer space, so `frag_coord.y = 0` is the **top** row.
- **Viewport: origin top-left.**
- **Canvas / DOM / mouse events: Y-down, top-left.**

So NDC is Y-up but frag_coord is Y-down. It is easy to flip in one place, not notice another path already had the opposite convention, and end up **double-flipped** (right by accident until you touch the other path) or **half-flipped** (picking mirrored relative to render). Worse: **a vertical flip is often not visually obvious** on an unfamiliar/roughly-symmetric manifold image — it passes visual inspection and only surfaces when the locked pixel / inspector / hover maps to the wrong place, i.e. the symptom appears in the wrong subsystem.

---

## The three coordinate spaces (they nest; each is right for its job)

Conflating "bottom-left origin" (orientation) with "allow negative coordinates" (signed values) is the core confusion — they live at **different layers**. There are three spaces:

| Space | Range / sign | Origin & orientation | Used for |
|---|---|---|---|
| **Screen / framebuffer** | `[0,W]×[0,H]`, pixels | top-left, **Y-down** | rasterisation, mouse events, output image |
| **UV / quad addressing** | `[0,1]²`, **unsigned** | bottom-left, **Y-up** (post-flip) the sample position *in the current view*, which chooses the quads asked for; the quad identity `(depth,tx,ty)` and the quadtree are taken in the **slice plane's own frame**, relative to the plane anchor (`z₀` at the last re-integrating event) — pan and zoom change which addresses are requested, never the addresses (R-97) |
| **IC / chart space** | **signed real**, physical scales | centred on `z₀`, **Y-up**, graph-like | the decoder input; any axis display / readout / scale bar — *a normal graph* |

**The map between the last two is the chart placement:**
```
z(s,t) = z₀ + (2s−1)·q₁ + (2t−1)·q₂        # the chart placement (chart/decoder contract)
#   (2s−1) maps [0,1]→[−1,1], so the plane is signed & centred on z₀.
#   Equivalently z₀ + 2(uv−½)·q — same map, one authoritative form.
```
UV is the unsigned `[0,1]` **address**; converting it places it as a **signed offset from the chart centre**, scaled by zoom. So:

- **UV stays `[0,1]` unsigned** — because "where in the current view" is naturally a `[0,1]` index *regardless of where the view sits* in signed IC-space, and quad addresses want clean non-negative integers. The addresses themselves live in the slice plane's frame, anchored at the plane anchor, not in the view (R-97). You never want negative quad indices; the signedness lives in the **placement** (`z₀` can be anywhere), not in the quad index.
- **IC-space is signed and centred** — the chart is a plane *centred on `z₀`*; a displacement from centre is naturally `±` (left/below negative, right/above positive). The golden IC is `z = 0`; ICs on either side are genuinely `±`. The plane has no natural corner-origin — it has a natural *centre*, and coordinates are signed offsets from it. This is the thing that "behaves like a normal graph."

**So "origin bottom-left, allow negatives" decomposes as:** *Y-up orientation everywhere* (the single flip) **+** *signed values in IC-space specifically* (the placement layer) — set in **two different places** (the framebuffer flip; the UV→IC placement transform). They are not the same fact.

---

## Per-axis signedness: "if it makes sense for that axis"

The navigation plane is **always** signed (you can pan either way from `z₀`), but whether a given *physical manifold axis* admits negative values depends on what it is:

- **Signed / symmetric:** a configuration displacement, a momentum component — negative is meaningful.
- **Constrained:** a mass fraction (simplex, `[0,1]`-ish, can't go negative); `‖ρ̃‖ = cos α` (bounded); a radius (positive).

So the **chart-plane offset** from `z₀` is always signed; the **manifold quantity it maps to** may be constrained. **The decoder is the boundary that translates one to the other** — it maps signed chart coordinates through mass→config→momentum, respecting each quantity's domain. The signedness of the *navigation* plane (always signed, centred) and the domain of each *physical axis* (varies) are, again, two different things, mediated by the decoder.

---

## Code paths that must honour the single convention (each is a separate path)

The point of naming *one* boundary is that every coordinate consumer reads from *after* the flip, so they all agree. The paths:

1. **Render sample coordinate** (frag_coord → UV → decoder) — the primary flip site: `v = 1 − frag_coord.y/H`, commented as *the* convention flip.
2. **Mouse / pointer picking** (lock, hover, click-inspect) — canvas events are Y-down top-left; flip them **the same way** before computing the picked quad / `z`. **Most likely to be forgotten**; symptom (clicking the top inspects the bottom) is the classic wrong-subsystem bug.
3. **Quad address `(tx, ty)`** — if `ty` derives from screen rows it inherits the flip; the address `(depth, tx, ty)` must be computed against the **Y-up** frame so a manifold cell has stable identity regardless of the flip (else CPU- and GPU-computed addresses could disagree on Y, and the "sample pattern is a property of the cell" property breaks).
4. **CPU quadtree ↔ GPU uniforms** — the CPU (f64) computes quad centres/half-widths; if CPU thinks Y-up and GPU samples Y-down (or vice versa) the quad-local coordinate is mirrored **within each quad**, and the linearised decoder's `x₀ + J_D·δ` produces mirror-image ICs. A CPU/GPU seam where a silent Y disagreement corrupts the decode — the convention must be shared explicitly.
5. **Exported image** — if internal is Y-up and the output format is Y-down, the encode step flips back **once** (the mirror of the input flip), equally single-and-named.

---

## The catch: a UV-passthrough debug view (debug-first)

Turn the invisible bug visible. A debug mode that renders the raw post-flip `(u,v)` as a gradient (e.g. `u→red, v→green`) makes a wrong flip **instantly obvious** — the gradient runs the wrong way, or picking a corner lights the wrong channel. This converts "mirrored manifold you don't notice for a week" into "the gradient is upside down, fix the one seam." Belongs in the debug tooling catalogue as a structural (non-payload) view, validated before any physics. Pair it with a picking cross-check: click a known corner, assert the reported UV/IC coordinate is the expected corner.

---

## Where this lands in the contracts

- **Deep-zoom / decoder contract** — the UV/quad/CPU-GPU coordinate machinery lives here; the three-space structure, the placement transform, and the CPU/GPU-seam convention-sharing rule belong in its coordinate section.
- **Debug tooling plan** — the UV-passthrough view + the picking cross-check as the catch.
- **GUI/trajectory-viewing** — picking (lock/hover/inspect) must flip the same way; note it at the pointer-event boundary.

---

*One orientation (Y-up), one flip (framebuffer↔UV, mirrored at export) — everything reads the post-flip coordinate, so a mirrored image is always a wrong-flip-count bug at that seam. Three spaces, not one: screen (Y-down pixels) → flip → UV (`[0,1]` unsigned, Y-up — addressing) → placement `z₀ + 2(uv−½)·q` → IC-space (signed, Y-up, graph-like — decoder input & axis display). "Bottom-left, allow negatives" = Y-up orientation (the flip) + signed values in IC-space (the placement) — two facts, two layers. Per-axis signedness varies; the decoder mediates. A UV-gradient debug view makes the whole thing self-checking.*
