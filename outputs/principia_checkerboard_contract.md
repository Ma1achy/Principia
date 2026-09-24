# Principia — checkerboard motion acceleration (contract)

*A motion-time throughput optimisation: during playhead advance, compute half the live pixels per frame and reconstruct the other half from their previous state. Measured to be imperceptible at 8-bit display precision when SSAA-resolved (see §7). This contract specifies the mechanism, the three-state control, how it composes with SSAA/ensembles and render-scale, the self-erasing catch-up at rest, and why it is a **compute** lever and not a memory one. It relaxes the single-playhead invariant to "single playhead at every quiescent moment" — see §6.*

---

## 1. What this is (and what it is NOT)

**What it is: a per-frame compute halver during playhead advance.** Under lockstep, each render pixel's nominal sample (and its bundle — see below) is marched one `dt` per frame. Checkerboard marches the samples of **half** the render pixels each frame (a checkerboard mask over render pixels), leaving the other half showing their value from the previous frame — one `dt` stale. Next frame the parity flips: the previously-stale half marches, the previously-fresh half holds. So every pixel advances every *other* frame, and per-frame integration cost is **~2×** lower. This raises frame rate during motion/playback — directly attacking the stutter and high-resolution responsiveness problem (the reason the render loop moved to a worker; caching Part 6a).

**Bundle atomicity (load-bearing):** a render pixel's *entire* sample bundle — the nominal sample, its E ensemble copies, and every Benettin shadow — **shares one checkerboard phase**: all march or all hold, together. The copies exist to be resolved and spread-reduced *against each other at one `t`*; letting them straddle phases would put mixed time *inside a single pixel's resolve* — corrupting SSAA, spread, and FTLE at once. The mask is per-pixel, never per-trajectory.

**What it is NOT: a memory optimisation.** This is load-bearing and must not be misread.

| Resource | Effect of checkerboard |
|---|---|
| **Sim memory** (`SimState` payload) | **no saving.** The stale half is not gone — it is *not advanced this frame*, but its `SimState` stays fully resident (required, so it can be caught up one `dt` later — §5). The whole payload is present regardless. |
| **Texture / render-target memory** | **no saving.** The framebuffer is full-resolution; stale pixels occupy their slot showing last frame's colour. No half-size buffer is allocated. |
| **Per-frame compute** | **~2× saving.** Half the trajectory integrations per frame. This is the entire point. |
| Overhead added | **near-zero.** Which-half-is-stale is a parity bit; each stale pixel's `SimState` is already resident. No extra buffer. |

> **Checkerboard is a throughput lever, not a footprint lever.** The intuition "skip half the pixels → save memory" is *wrong* here — nothing shrinks, you just do less work per frame. **Memory pressure is handled by a different family** (quality tiers: fewer samples, FTLE off, smaller `E`; and `render_scale`: fewer render pixels + smaller render targets — `principia_memory_tiers.md`). Checkerboard sits with the *throughput* levers (coarser refinement during motion + this). It will not help a device that is OOMing; it helps a device whose frames are too slow.

---

## 2. The mechanism: reconstruct-from-previous, resolved through SSAA

A checkerboard-stale pixel shows its **own value from `t − dt`** — its previous computed colour, held one frame. This is *temporal* reconstruction (reuse this pixel's last value), **not spatial interpolation** (never invent a pixel from its neighbours — that remains forbidden; it would smooth over filamentary structure). The pixel is real; it is simply one `dt` behind.

The reconstruction quality depends entirely on **SSAA/ensembles**, which is why the two are coupled:

- **With SSAA (`E ≥ 1`):** the pixel's E+1 ensemble sub-samples sit at distinct sub-pixel IC offsets (Halton (2,3), sampling note). Near a boundary these sub-samples straddle it — some flip class, some don't — and the SSAA resolve averages them. **Averaging E+1 straddling sub-samples pulls the reconstructed pixel to within a fraction of one 8-bit level of the true value** (§7). SSAA is the mechanism that makes checkerboard imperceptible.
- **Without SSAA (`E = 0`, Potato/Low):** one sample, no averaging, so a stale pixel is cruder (a hard stale value, no boundary softening). Checkerboard **still functions** — it is still a valid ~2× compute win — it is just *rougher* at boundaries. **SSAA is a quality multiplier on checkerboard, NOT a gate:** checkerboard runs at every tier; it is nearly-perfect where ensembles exist and functional-but-rough where they don't. The pure-`E=0` case is the honest fallback, not a disabled state.

**Optional: spread-extrapolation (extra quality where affordable).** Instead of holding the stale value flat, nudge it forward: `value(t) ≈ value(t−dt) + dt · rate`, where `rate` is the pixel's local rate of change (a by-product available from the integration). Measured to cut reconstruction error a further ~69% (§7). A refinement, not required; gated to where the rate is cheaply available.

---

## 3. The three-state control

A single user setting, three states. **Default: On (permanent).** The default is justified by §7 (imperceptible) *and* by §5 (self-erasing at rest — the at-rest frame is always full regardless), so "permanent" carries no fidelity cost at rest.

| State | Behaviour | For |
|---|---|---|
| **On — permanent** *(default)* | Checkerboard whenever the **playhead advances** (camera motion *and* static playback). Auto-fills to a full frame whenever the playhead is stationary (§5). | Everyone — proven imperceptible, and rest/endpoints are always full. |
| **On — motion only** | Checkerboard only during **camera motion** (pan/zoom/tilt/slice/catch-up). **Static playback** (playhead advancing, camera still — watching the system evolve at a fixed view) renders **every frame in full**. | Users who want every *playback* frame pristine and will pay the compute for it. |
| **Off** | Never checkerboard; every frame computes all live pixels. | Purists; debugging; anyone who wants the guarantee off. |

**Scope: checkerboard applies to the live-set march only.** Catch-up marches (newly revealed/split quads racing `0→playhead`, or resumes) are **never checkerboarded** — they are background work, not presented until promoted, so there is nothing to reconstruct; skipping their steps would only delay promotion. Checkerboard halves the cost of advancing the *presented, barrier-synced live set*.

**The trigger is playhead advance, not merely camera motion** — checkerboard staleness comes from the playhead moving between frames (`t` vs `t−dt`), which happens during both camera-driven catch-up *and* playback. So:
- **Permanent** gates on *playhead advances at all* → covers motion and playback; a no-op at true rest.
- **Motion-only** gates on *camera motion specifically* → so static playback (the "watching evolution carefully" moment) stays full. This is the *sole* behavioural difference between Permanent and Motion-only: whether static playback checkerboards.

**Always force-off in export (regardless of the setting).** Export runs the blocking-barrier full path (`principia_export_animation_contract.md` Part 4) — every exported frame is fully caught-up and fully refined. Checkerboard does not apply to export; the setting is ignored there. A screenshot/export is never checkerboarded.

---

## 4. Composition with render-scale (they stack; upscale helps)

Checkerboard applies **at render resolution**, before the render→display upscale. `render_scale` reduces render resolution below display (`principia_memory_tiers.md` §2); checkerboard then halves the pixels *at that render resolution*. They **compose cleanly and independently**:

- **Measured (dt=0.05, 8-bit levels of reconstruction error):** render_scale 1.0 → mean 0.90; 0.75 → 0.84; 0.5 → 0.81. **Error *decreases* slightly as render_scale drops**, because the bilinear render→display upscale **blurs the checkerboard seams** — the stale pixels get averaged with fresh neighbours during upscaling, a free cleanup pass. Max error also drops (110 → 78 levels at 0.5). The earlier worry that upscaling might make the shimmer *chunkier* is wrong: bilinear upscale is a smoothing op and it smooths checkerboard the same as everything else.
- So **both may be on at once** — they are orthogonal motion accelerators (render_scale: fewer render pixels + smaller targets = compute *and* memory; checkerboard: half those render pixels per frame = compute only), and stacking them is strictly cooperative.

---

## 5. Self-erasing: catch-up at rest, endpoints, and pause

The insight that makes "permanent" safe: **a checkerboard-stale pixel is a pixel not computed *yet* this frame — and "yet" becomes "now" the moment there is compute slack.** Checkerboard is therefore a *transient half-frame time-debt*, not a permanent state of the frame, and it **amortises itself to zero whenever the playhead is stationary.**

**Mechanics of the fill-in (cheap — one step, not a re-integration):**
- While advancing, the fresh half is at `t_now`, the stale half at `t_now − dt`.
- When the playhead **stops** (rest / pause), march the stale half **one `dt`** to reach `t_now`. Both halves now at `t_now`; promote at the next barrier. Full frame, single playhead, pristine.
- This is **one `dt` for half the pixels** — tiny, instant at rest, not a march from zero. It reuses the existing catch-up machinery (the *moving fallback* → *promote at a time-sync barrier* pattern, temporal note) at **pixel granularity** — the same code path as quad catch-up after a pan.

**Where the fill-in fires:**
- **At rest** (playhead paused, camera still): stale half catches up → full frame.
- **Pause** (`play/pause`, temporal note): the playhead stops advancing → the debt is paid → the paused frame is full. (Render-mode swaps and panning at the frozen `t` still work; if you then pan, catch-up resumes at pixel granularity.)
- **Timeline endpoints:** at **`t = 0`** (before any marching — nothing is stale) and at **`t_end` / all-terminated** (playhead stationary), the frame is full. The **start and final frames are always complete.**
- **Static playback under Motion-only:** every frame full by definition (that mode does not checkerboard playback).

> **No frame is ever *served* checkerboarded except mid-advance.** At every quiescent moment — rest, pause, both timeline endpoints, and (under Motion-only) static playback — the frame is fully current. So the fake-data hazard is eliminated: a screenshot at rest is full (rest filled it in), an export is full (blocking barrier), the endpoints are full. The stale pixels are **always** either imperceptible (mid-motion, §7) or already filled in (everywhere the playhead is still). There is no configuration that serves a deceptive still frame.

---

## 6. The single-playhead invariant, relaxed (the one architectural concession)

The temporal architecture rests on **never presenting mixed time** — every visible pixel at one `t`, enforced by the barrier (temporal note: "the only thing that reaches the screen is a set known to be at one `t`"). Checkerboard is the **one deliberate, bounded relaxation** of this:

> **Single playhead at every quiescent moment; a bounded one-`dt` half-frame skew only while the playhead advances.** During advance, the screen carries two adjacent times — half at `t`, half at `t − dt`. The skew is **exactly one `dt`** (not unbounded), it exists **only while advancing**, and it **collapses to zero** whenever the playhead stops (§5). At rest, pause, and endpoints the strict single-playhead invariant holds unchanged.

This is a genuine concession and is named as such. Its justification is threefold: the skew is **bounded** (one `dt`), **imperceptible** while it exists (§7 — the moving eye cannot resolve sub-8-bit-level per-pixel differences), and **self-erasing** at every moment fidelity matters (§5). The `Off` state restores the strict invariant unconditionally for anyone who wants it. Export never relaxes it. This mixed-time is categorically different from the *glitches* the barrier designs out (frozen rectangle in a live field, region snapping forward): those were **unbounded, arbitrary** time mismatches presented as coherent; this is a **uniform, bounded, one-`dt`** skew that is measured-imperceptible and dissolves on cue.

---

## 7. Measured imperceptibility (the evidence for the default)

Tested on a planar-3-body IC velocity slice (softened forces, crude classifier, RGB-as-side-lengths stand-in for the real VMF/OKLAB shape colouring — so treat *magnitudes* as indicative, *structure* as robust). Error = reconstruction difference vs the true frame, converted to **8-bit display levels** (one level = 1/255 per channel; below one level, a pixel is *the same colour on screen*). SSAA-resolved, `E+1 = 8` (High tier), reconstructed (stale) pixels only:

| dt | mean error | mean in 8-bit levels | verdict |
|---|---|---|---|
| 0.02 | 0.0052 | **0.77 level** | below quantisation floor — *literally the same on-screen colour* for the typical pixel |
| 0.05 | 0.0125 | **1.84 levels** | imperceptible, especially in motion |
| 0.10 | 0.0230 | 3.39 levels | faintly visible on boundaries at the aggressive `dt` |

**The tail (boundary pixels — the honest caveat):** the mean is sub-level, but not every pixel is. At dt=0.02: 95th percentile ~3 levels, max ~85 levels (a handful of genuine boundary-flip pixels). The high-error pixels are **~2.5% of the frame, on boundaries, one frame, moving** — invisible while moving (the eye integrates across frames and cannot lock onto individual pixels), *findable* only if you freeze the frame and pixel-peep. At rest you cannot freeze a checkerboarded frame — rest fills it in (§5).

**Single-sample (no SSAA) for contrast:** mean error ~0.27 ≈ **~40 levels** — the visible "tearing." **SSAA is the entire difference between unusable and below-the-display-floor.** This is why §2 frames SSAA as the quality mechanism.

**Error scales smoothly with `dt`** (no cliff), which makes a **`dt` ceiling** the clean gate: checkerboard stays comfortably sub-2-levels for `dt ≲ 0.05`. Above that the mean creeps visible. Recommended: **disable (or reduce the checkerboard fraction) when the per-frame playhead advance exceeds ~`0.05`** in the relevant time units — i.e. checkerboard only when one playhead step is small enough that the mean reconstruction error stays ≲ 2 levels. (Exact ceiling is a build-time tune against the real colour mapping and integrator `dt` scaling; the *existence* of a smooth, gateable threshold is the robust finding.)

**Render-scale composition** (§4) and **spread-extrapolation** (§2) both *reduce* error further (upscale blurs seams; extrapolation −69%), so the figures above are the *unaided* SSAA baseline — the floor, not the ceiling, of achievable quality.

---

## 8. Build-time settles (measure on the real system)

- **The exact `dt` ceiling** — §7's ~0.05 is from the proxy; the real VMF/OKLAB colour mapping could amplify small state differences into more levels (a more sensitive mapping → lower ceiling), and the real integrator's `dt` scaling differs. Confirm where the mean crosses ~2 levels on the real render.
- **Checkerboard-fraction ramp vs `dt`** — instead of a hard on/off at the ceiling, optionally *ramp* the stale fraction down as `dt` approaches the ceiling (fewer stale pixels when staleness is riskier). Whether a ramp is worth the complexity over a hard gate is a feel call.
- **Spread-extrapolation on/off default** — the −69% is real but it costs the rate term; decide whether it is on by default or a further opt-in.
- **Catch-up granularity at rest** — filling the whole stale half in one frame at rest is the simple policy; if that one-frame catch-up ever costs a visible hitch on weak devices, spread it over 2–3 frames (still sub-perceptual since the playhead is stopped). Measure.
- **Interaction with the controller's other motion levers** — checkerboard, coarser-refinement-during-motion, and E-reduction-under-motion all fire during motion; confirm they compose without over-degrading. (The arbiter **accounts for** checkerboard's ~2× in its cost model but **never toggles it** — it is a user-owned setting, not an arbiter knob; quality/device note, memory-tiers §5.)

---

*Checkerboard is a bounded, self-erasing, one-`dt` half-frame time-debt taken on only while the playhead advances, resolved through SSAA to below the 8-bit display floor, dissolved to a full frame at every rest/pause/endpoint, forced off in export, and switchable off entirely. It buys ~2× per-frame compute during motion — a throughput lever for the responsiveness problem, saving no memory (that is the tiers' and render_scale's job). Sharp means true; the skew is imperceptible and never outlives the motion that caused it.*
