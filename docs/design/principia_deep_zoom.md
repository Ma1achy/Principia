# Principia — deep zoom & adaptive sampling

*The mechanism — quad-local coordinates, linearised decoder, three-layer quadtree, sliding depth bound — and how it couples back to the chart & decoder contract. Sits under that contract; shares its Jacobian and its CPU/GPU split.*

---

## The precision split (the CPU/GPU seam, decode side)

Deep zoom is the **decode-side instance** of "CPU high-fidelity, GPU throughput." The global/nonlinear precision lives in **f64 on the CPU**; the GPU does only **local, relative, f32** arithmetic. Two mechanisms implement it: quad-local coordinates for the UV→sample step, and the linearised decoder for the sample→IC step.

| Component | Precision | Job |
|---|---|---|
| CPU (quadtree manager) | f64 | quad bounds, centre/half-width, `x₀` and `J_D`, scheduling |
| GPU (per-quad uniforms) | f32 | `c, h, x₀, J_D` — exact half-widths, slight centre offset |
| GPU (kernel) | f32 | quad-local sample positions, IC via `x₀ + J_D·δ`, integration |
| Pixel inspector | f64 (CPU) | single-trajectory witness at full precision |

**Coordinate conventions (the CPU/GPU seam is also a Y-convention seam — see `principia_coordinate_conventions_note.md`).** Three spaces nest, and the CPU and GPU must share the same orientation or the quad-local decode is silently mirrored:

- **Screen/framebuffer** — `[0,W]×[0,H]`, top-left, **Y-down** (frag_coord, mouse events, output image).
- **UV / quad addressing** — `[0,1]²`, **unsigned, Y-up** after a single flip `v = 1 − frag_coord.y/H`. UV locates a sample *in the current view*; it chooses which quads are asked for. The quad address itself — `(depth, tx, ty)`, i.e. `(level, i, j)` — and the quadtree key are taken in the **slice plane's own frame**, relative to the plane anchor (`z₀`'s value at the last re-integrating event, R-92): in-plane pan and zoom change which addresses are requested, never the addresses themselves (R-97).
- **IC / chart space** — **signed real**, centred on `z₀`, Y-up, graph-like (the decoder input; any axis readout). Placement (chart/decoder contract): `z = z₀ + (2s−1)q₁ + (2t−1)q₂` — the `(2s−1)` maps `[0,1]→[−1,1]`, so the plane is signed and centred on `z₀`. The golden IC is `z=0`; ICs on either side are genuinely `±`.

**One orientation (Y-up), one flip (framebuffer↔UV, mirrored once at export).** Everything downstream reads the post-flip coordinate, so a mirrored image is always a wrong-flip-count bug at that one seam — never a reason to add a compensating flip. **Critically for this contract: the CPU (f64 quad centres/half-widths) and the GPU (quad-local sample coord) must agree on Y-up**, or `x₀ + J_D·δ` produces mirror-image ICs *within each quad* — a silent decode corruption at the CPU/GPU seam. "Bottom-left, allow negatives" decomposes as *Y-up orientation* (the flip) + *signed values in IC-space* (the placement) — two facts, two layers; per-axis signedness varies (config/momentum signed; mass fractions/radii constrained), and the decoder mediates.

Nothing here contradicts the contract's rule that the integrator has one input type: `x₀ + J_D·δ` still produces `(m, r, p)`, and the integrator never learns whether the IC came from the full decoder or the linear one.

---

## 1. Quad-local coordinates — UV precision

**Problem.** At quadtree depth `ℓ`, a quad spans `~2^−ℓ` of the `[0,1]²` UV domain. Computing global UV on the GPU as `u = u_min + (u_max−u_min)·t` in f32 burns most of f32's ~7 significant digits on the *offset from the origin* — at depth 30 the quad is `~10⁻⁹` wide, so there's nothing left for within-quad resolution.

**Fix.** The CPU (f64) computes per-quad **centre `c_u`** and **half-width `h_u`** (and `c_v, h_v`), passed to the GPU as f32 uniforms. The kernel works *relative to the centre*:

```
u = c_u + h_u · (2t − 1)        t = (i + 0.5)/N   (quad-local sample coord)
```

**Why it works.** `h_u = 2^−(ℓ+1)` is an exact power of two, exactly representable in f32 at any depth up to the exponent limit (`~2⁻¹²⁶`). `c_u` has finite f32 precision, so the quad is misplaced *globally* by less than one quad width — invisible in the image. Within-quad precision is full f32 at any depth. **`t = (i+0.5)/N` and `δ` are in the post-flip Y-up convention** (coordinate note): the same orientation the CPU used to compute `c, h` and the Jacobian, so the quad-local offset is not mirrored relative to the decode. A wrong Y here mirrors ICs within the quad silently.

---

## 2. Linearised decoder — IC precision

**Problem.** At shallow zoom the GPU runs the full nonlinear decode per sample (sigmoid, softmax, trig, canonicalise). At deep zoom this hits its own f32 floor at depth **~20–23**: adjacent samples decode to indistinguishable ICs.

**Fix.** Linearise the decode about the quad centre. Error is `O(h²)` — at depth 30, `h ~ 10⁻⁹`, so error `~ 10⁻¹⁸`, far below anything representable.

- **CPU precompute (f64), per deep quad:** evaluate the full decode at the centre, `x₀ = D(c_u, c_v)` in f64 (the reference IC); compute `J_D = ∂D/∂(u,v)` at the centre by **central differences in f64** (two decode evals per axis). Pass `x₀` (→f32) and `J_D` (→f32) as uniforms.
- **GPU kernel (f32):** `x = x₀ + J_D · δ`, where `δ` is the quad-local UV offset. A few fused multiply-adds (continuous — the decode is not a branch input, so FMA latitude here is honest divergence, not a determinism concern); the nonlinear pipeline never runs on the GPU.

**Why the precision survives.** The nonlinear precision lives in `x₀` (computed in f64, so its f32 cast is the best possible representation of the true centre IC). `J_D` is *rates of change* — `O(1)`, no precision lost in f32. Floor moves from ~23 to **~50+**.

**Also faster.** The linear path is cheaper than the full decode, and deep zoom is exactly where quad count and compute pressure peak. Smooth interior quads are coarsened early and never reach this regime; the linear path targets the expensive boundary quads.

**Switchover.** **Switch to the linearised decoder when the full decoder's adjacent samples give bitwise-identical ICs, with `ℓ_switch = 20` as an upper bound — whichever comes first** (R-90; lowering's `SWITCH` = `ℓ_switch`), via a `DECODE_MODE` flag in `QuadRequest.flags`. Output is identical either side — same `SimState`, same reduction, same render. This is the key point for the scheduler's floor logic: **sample-collapse on the *full* decoder is a switchover trigger, NOT a refinement stop** — it means "the full decoder's f32 pipeline is out of precision, hand off to the linear path," and refinement *continues*. Only sample-collapse on the *linearised* decoder is the true `AT_F32_FLOOR` stop. Same visible symptom, opposite response, keyed to which decoder is active. (The adaptive trigger also covers the corner case where microscope zoom has shrunk `q₁, q₂` to tiny magnitudes at *shallow* quadtree depth — tiny `q` loses precision in the full decode even before the quad pyramid is deep, so the switchover can fire early; still a switch, never a stop.)

Jacobian cost: microseconds per quad on the CPU, amortised over `N²` samples. Negligible.

---

## 3. Three-layer quadtree

Built in three layers, each a working system; later layers add capability without changing earlier contracts. This is the build order.

| Layer | Adds | Contracts / behaviour |
|---|---|---|
| **0 — flat grid** | no cache | Defines the three contracts: `SimState` (compute output), decoder (quad-local UV → IC), fragment shader (`SimState` → colour) |
| **1 — quad cache + ancestor fallback** | slippy map | Panning never blanks — a cached ancestor always shows; zoom shows blurry ancestors that sharpen as children finish. Data flow identical to L0; CPU tracks cache membership only; **no sim data flows GPU→CPU**; FIFO queue |
| **2 — adaptive refinement** | budget allocation | CPU→GPU adds the linearised-decoder uniforms; refinement driven by trajectory coherence / ensemble spread (FTLE is on from Medium up — memory-tiers doc) |

**The quadtree is adaptive scientific sampling, not just LOD.** It allocates simulation budget by *local dynamical complexity*: smooth basin interiors stay coarse even at large screen area; filamentary boundaries refine even when visually small. "Resolution" means both screen resolution and dynamical-complexity resolution.

**Consequence that ties to Part 2.5:** refinement density is **not** physical probability density. The quadtree concentrates compute at boundaries because they're interesting, not because those ICs are more probable. So refinement structure must never be read as a measure — quantitative claims use `|det J_D|` (below), not quad density.

---

## 4. Sliding depth bound

The refinement cap is a **sliding window**: refine to `current_depth + max_depth`. The window follows the camera down, so there are always a bounded number of adaptive levels below the view and the work is never unbounded. A trivial scalar in the scheduler.

This sidesteps the floor question at design time — you don't need to *know* where any precision floor sits in order to stop; you just don't refine past a fixed budget below the view.

**Stopping conditions, in effect (by how often they fire):** (i) the **screen-space floor** — `tile_size ≤ pixel_size` — the *everyday* boundary: in view, every quad above it must split, and below it the criterion may supersample where a footprint is unresolved (policy §0.1, R-88); view-relative, un-floors on zoom-in; (ii) the sliding `MAX_REL_DEPTH` budget cap — it caps only that supersampling depth (`MAX_REL_DEPTH` ≥ the screen floor, R-88); (iii) the **decode switchover** (not a stop) — if the *full* decoder's `N²` samples collapse to bitwise-identical ICs, or at `ℓ_switch` if that comes first (R-90), **switch to the linearised decoder** and keep refining (see below); (iv) the **decode floor proper** (`AT_F32_FLOOR`) — only if the *linearised* decoder's samples collapse (~depth 50+), genuinely terminal, a deep-zoom backstop; (v) the integration floor — reserved (see end). The screen floor governs normal use; (iii)–(v) only matter when zooming *past* pixel-matching into extreme zoom.

---

## Couplings to the contract

**a. The linearised object is the full chart→IC composite, not the decoder `D` alone.** What varies with `(u,v)` is the whole map: quad UV → `Φ` → z → decode → canonicalise → physical. Because `J_D` is taken by **central differences in f64**, the deep-zoom path is chart-agnostic — it linearises whatever the composite is, including nonlinear axis warps, the invariant downstream solve, and the Burrau curve embed. The four axis kinds need **zero** special-casing here. The one thing not to do: linearise only the innermost decoder and drop the chart map's contribution.

**b. `|det J_D|` is the shared measure weight from Part 2.5.** `J_D = ∂physical/∂(u,v)` factors through the link derivatives (softmax Jacobian, sigmoid slope, curve tangent). Its determinant *is* the local sampling-measure factor that Part 2.5 requires to travel with a link for quantitative honesty. The CPU already computes `J_D` per deep quad for the linear decode — take `|det J_D|` and the per-quad measure weight is free. Integrity machinery and deep-zoom machinery share one Jacobian; compute once, use for both.

**c. The f64-anchored decode isolates CPU/GPU divergence to the integrator.** Once the decode is anchored in f64 (`x₀`, `J_D` both f64-computed), the GPU's IC is the best f32 representation of the true IC, so the *decode* barely diverges between CPU and GPU. The divergence the pixel inspector then shows is almost purely **integration** divergence — exactly the quantity worth witnessing. The linearised decoder doesn't only buy depth; it cleans the divergence signal.

---

## Reserved: the integration floor

There is a second floor the decode guard doesn't catch — adjacent ICs *distinct* in physical coordinates, but f32 integration error exceeds their difference, so outcomes differ by rounding noise rather than real structure (Trani's numerical chaos, at pixel scale), and/or trajectories saturate the substep cap at close encounters. This floor is a **confidence gradient, not a terminal wall** (integrator contract Part 6, regularisation and the integration floor): saturated/suspect trajectories still reach real dynamical outcomes — they're *flagged* (the `saturated` descriptor bit; a high `suspect_fraction`), not terminated — so the structure is *computed with low confidence*, not cut off. The `saturated` bit and the derived suspect predicates exist in the payload (§2), so nothing is foreclosed; the scheduler uses saturation + high `suspect_fraction` as the **integration floor refinement-stop** (stop subdividing what the integrator can't resolve), not a sample-terminal. The sliding `MAX_REL_DEPTH` and the screen-space floor bound the work regardless of where this floor sits. Not a section — a reserved concern.

---

*Global precision on the CPU, relative arithmetic on the GPU. The linear decode is chart-agnostic, shares the integrity Jacobian, and isolates divergence to the integrator. Depth is bounded by budget, not by knowing the floor.*
