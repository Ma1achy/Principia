# Principia — quality tiers & memory model (auto-mode reference)

---

> ### ⚠ THE TIER MODEL IS NOW THREE AXES, NOT ONE
>
> This document describes tiers as **resource allocations** — resolution, `N`, `E+1`, max depth. The
> quality knob is now **`eps`**, a tolerance in `spread_shape`'s own units
> (`principia_dd_refinement_policy.md`).
>
> **A tolerance is a quality promise; a tier is a cost bound; and they conflict** — reaching a given
> `eps` costs **5–37× less than uniform on Burrau and about the same on arbitrary latent slices**,
> because the cost is a property of the field. So the tier becomes three coupled axes:
>
> - **`eps`** — the convergence target, where refinement stops
> - **frame budget** — the per-frame cap, how fast it gets there
> - **hard cap** — quads/bytes, memory safety. **New and not optional**: under `eps` the tree size is
>   slice-dependent and unbounded, so a pathological slice OOMs without one.
>
> **Always report which axis bound** — converged / budget-bound / cap-bound. A tier that silently
> misses its target is the failure mode this project exists to eliminate.
>
> **Full treatment:** `principia_dd_telemetry_and_tiers.md` §3.5. The per-tier numbers here are still
> a reasonable starting point; the *shape* is what has changed.

*Grounded memory estimates and the six named quality tiers the auto-quality controller ladders between. This supersedes an earlier version that mistakenly keyed quality on viewport resolution and included per-quad pixel interpolation — both removed (see the taxonomy and the "no fabricated detail" rule below).*

*Memory model verified against the payload spec: **payload = `bytes × (E+1) × render_px`** (the shadow is inside `SimState`, so `(E+1)`, not `2(E+1)` — that's the compute/trajectory count). Hot `SimState` = **144 B** (FTLE-on) / **96 B** (FTLE-off) (recomputed, R-40 / D6); word = **16 B × (E+1) × render_px** (symbolic features on). **The §4 totals add the render-target term** (the rasterised image buffers: ~3 full RGBA at **display** resolution — the swap chain / final composite are necessarily display-sized — plus ~3 at **render** resolution for backdrop/intermediates); they still exclude bake texture, staging, and driver/browser/OS overhead. `render_px = display_px × render_scale²` — see §2's two-resolution model (sample density is not a separate knob — the screen floor pins one sample per render pixel).*

---

## 1. Taxonomy (locked vocabulary)

The pipeline, IC-space → screen, with one word per level (the old overloaded "tile" is retired):

| Term | What it is | Where |
|---|---|---|
| **QUAD** | a node in the quadtree; subdivides **IC-space**; splits into 4 children where structure is complex | IC-space |
| **SAMPLE** | one of the `N×N` points inside a quad; **each is a full simulation** (`SimState` + trajectory/shadow/word). `N = SAMPLES_PER_QUAD_AXIS` | IC-space (the physics) |
| **TILE** | a sample's **screen-space footprint**, pre-rasterisation — the thing that gets drawn | screen space |
| **PIXEL** | an actual screen pixel | viewport |

**Pipeline:** QUAD (IC-space) → contains N×N SAMPLES (physics) → each SAMPLE is a TILE in screen space → TILE rasterises to PIXELS.

**Renames from earlier docs:** quadtree-node `tile` → **QUAD**; `SAMPLES_PER_TILE_AXIS` → **`SAMPLES_PER_QUAD_AXIS`**; `TileReduction`/`TileSummary` → **`QuadReduction`/`QuadSummary`**; `TILE_PIXEL_RES` → **removed** (it was an interpolation knob — see below). New meaning of **`tile`** (retained): a sample's screen-space footprint, pre-rasterisation.

---

## 2. Two resolutions — display and internal-render (sample density is not a separate knob)

Two distinct quantities, historically muddled as one "resolution":

| Quantity | What it is | Knob? |
|---|---|---|
| **Display resolution** | the monitor / window pixel count | **a given** — not controllable (it's the user's screen) |
| **Internal render resolution** | what is actually rasterised, then upscaled/downscaled to display: `render_px = display_px × render_scale²` | **tier knob + Custom slider** (`render_scale ∈ {0.25, 0.5, 0.75, 1.0}`), or **locked to native** (see below) |

**There is no separate "samples per pixel" dial** — and this is the point that keeps the resolution system coherent. Sample density is **not** an independent quality knob layered on top; it is set by the quadtree's screen-space floor. The quadtree **must** subdivide every in-view quad until its tiles reach pixel size (the **screen-space floor**, scheduler contract; policy §0.1, R-88), and *that is the resolution mechanism*. So at the refinement steady state, **there is at least one real sample per render pixel, by construction** — you cannot hold "half a sample per pixel" as a setting, because the floor keeps subdividing until tiles equal pixels. Below the floor the refinement criterion may supersample where a footprint is unresolved, capped by `MAX_REL_DEPTH`, which caps every split beyond the screen floor, off-screen policy splits included (R-88, R-98); the one-sample-per-render-pixel figures in this doc are that floor's baseline. An earlier draft had an `spp` ("samples per rendered pixel") tier knob; it was **deleted** because it directly contradicts the screen floor (sub-1 spp fights the floor that drives density to 1) and duplicates E (super-1 spp *is* the ensemble). The three real levers are `render_scale` (how many pixels you render), the refinement floor (how far quads subdivide — normally to pixel size), and E (ensemble/SSAA, the only supersampling path).

**"Resolution" in the quality sense = how many pixels you render (`render_scale`) and how finely the quadtree refines** (to the screen floor). Display size is *scale*, not quality — a bigger window carries no more *information*, it just rasterises the same refined structure across more pixels.

**Sharpness comes from subdividing quads, not interpolation.** When a tile is bigger than a pixel (a quad not yet refined to the floor — mid-zoom, or motion), that region is honestly one sample's value (a flat block); to sharpen, the quadtree **splits** — real new samples at finer IC-space spacing, down to the screen floor. There is **no one-sample-fabricated-into-many-pixels** interpolation (it would smooth over the filamentary structure the instrument exists to reveal — scientifically backwards).

**The only legitimate sample↔pixel combining is SSAA via ensembles** — the *opposite* direction: when a tile is smaller than a pixel (deep quad / zoomed past 1:1), multiple real samples fall in one pixel's footprint and are **averaged** to the pixel colour (the E+1 ensemble copies). Honest anti-aliasing of computed data — many-real-samples → one-pixel, never one-sample → many-pixels. Two non-fabricating exceptions to "one sample, one tile": (a) this SSAA resolve; (b) the **transient blurred ancestor fallback** while a quad's samples compute ("sharp is real, fuzzy is arriving") — a loading state, never data.

### What `render_scale` gets you (and its character)

`render_scale < 1` renders fewer pixels than the display and upscales — the low-end survival lever. Because the screen floor pins one sample per *render* pixel, rendering at 0.5× means ¼ the render pixels means ¼ the samples: **the sample saving comes through `render_scale` for free**, no separate density dial required. The image is **uniformly soft** (upscaled) but **structurally honest underneath** — each render pixel is still a real, sharp sample at the screen floor; you're just showing fewer of them, stretched. For a research instrument that is the right kind of degradation: you can still see the fractal boundary is *there*, just at lower effective resolution. (`render_scale > 1`, supersampling, is a Custom-only extra — see §4; it is not used by any tier.)

### "Lock to native" toggle

A UI toggle (**default on for most users**) pins internal render resolution to the actual display resolution — `render_scale = 1.0`, and the render-scale slider is **disabled/greyed**. Most people want native and shouldn't fiddle. Unlocking exposes the slider for the two off-native uses: **below 1.0** (performance — render low, upscale, survive on weak hardware) and **above 1.0** (supersampling — render high, downsample, an *alternative* AA method; Custom only). Auto-mode respects the lock: if locked, it adapts via E and the refinement floor only, never render_scale.

---

## 3. Memory model — payload + render targets

Two separate consumers, both driven by **render** pixels (not display pixels). At the refinement steady state the screen floor gives one sample per render pixel, so live sample count ≈ `render_px`:

```
render_px      = display_px × render_scale²
payload_mem    = (E+1) × bytes × render_px                       (the SimState samples — one per render pixel at the floor)
render_targets = k_d × display_px × 4  +  k_r × render_px × 4    (k_d ≈ 3 display-res: swap chain + final composite;
                                                                  k_r ≈ 3 render-res: backdrop + compositing intermediates)
total          ≈ payload_mem + render_targets + fixed            (fixed = bake texture, staging)
```

> **Why the split matters:** the swap chain / final composite are *necessarily display-sized* (that's what is presented), so they do **not** shrink with `render_scale` — only the render-res intermediates do. An all-at-render-resolution model undercounts targets exactly where the term matters (low `render_scale` on big displays). Corrected here; only the sub-native tiers' totals move.

- **`payload_mem`** — the sample memory. `(E+1)` (shadow is inside `SimState`), `bytes` = 144/96, `render_px` = live sample count (one per render pixel — the screen floor, not a separate spp factor).
- **`render_targets`** — the **rasterised image itself uses VRAM**: ~3 buffers at **display** resolution (swap chain + final composite — fixed regardless of tier) + ~3 at **render** resolution (backdrop + intermediates — scale with `render_scale²`). At native (`render_scale = 1`) the six together ≈ 0.05 GB @1080p / 0.09 @1440p / 0.20 @4K; below native only the render-res half shrinks.
- **`render_scale` is the quadratic lever** — it changes `render_px`, so it cuts payload, render targets, *and* per-frame compute **all at once**. The single most effective performance knob (this is why games lean on dynamic resolution).


**Render targets dominate at the low end.** At Potato@4K, render targets (0.11 GB — mostly the display-sized swap chain, which `render_scale` cannot shrink) are **~68%** of the 0.16 GB total — the image buffers *exceed* the payload. At Extreme@4K they're ~1% (rounding error under 21 GB). So the render-target term matters *exactly* where the payload is smallest: weak devices on big displays, the memory-constrained case the low tiers serve. This is why it can't be dropped from the model — and why the display-vs-render split above can't be either.

---

## 4. The six quality tiers

Quality is `render_scale / E / FTLE / word`. The bottom two tiers (Potato/Low) render **well below native** to survive on weak GPUs (render_scale carries the saving — fewer render pixels means fewer samples at the screen floor) and skip FTLE; **Medium up** turn on FTLE and climb the SSAA ladder, reaching native (`1.0×`) at High. `N~`/`depth~` are indicative — the tier sets the sample grid and refinement budget; the screen floor pins one sample per render pixel.

| Tier | render_scale | E | SSAA | FTLE | N~ | depth~ | Character |
|---|---|---|---|---|---|---|---|
| **Potato** | 0.25× | 0 | 1× | — | 8 | 3 | render quarter-res + upscale, no AA/FTLE/word — just runs (uniformly soft, but real samples underneath) |
| **Low** | 0.5× | 0 | 1× | — | 8 | 4 | render half, word on, no FTLE |
| **Medium** | 0.75× | 1 | 2× | on | 16 | 5 | render ¾, 2× SSAA, FTLE — the runs-on-a-laptop baseline, first tier with the chaos measurement |
| **High** | 1.0× | 3 | 4× | on | 16 | 6 | native, 4× SSAA, FTLE — the sweet spot |
| **Ultra** | 1.0× | 7 | 8× | on | 24 | 7 | native, 8× SSAA, deeper refinement |
| **Extreme** | 1.0× | 15 | 16× | on | 32 | 8 | native, 16× SSAA, max everything — melts current top-end GPUs |

**FTLE is on from Medium up, off for Potato/Low — a *compute* + *fidelity* boundary, not memory.** (render_scale already shrank the sample count so much that FTLE's memory cost is trivial — +6 MB at Potato, +0.1 GB at Medium@1080p — so memory is no longer the reason.) The reasons it stays gated at the bottom two tiers: (1) **compute** — FTLE is a *second full trajectory per sample* (the Benettin shadow), ~2× the integration work, and Potato/Low serve genuinely weak, *compute-bound* GPUs (a phone won't OOM at 0.02 GB — it'll chug on 2× the trajectories); (2) **fidelity match** — FTLE is a quantitative chaos measurement, and Potato/Low render at 0.25×/0.5× and upscale, so the measurement would be computed on a blurry quarter-res canvas where its fine structure can't be read. Medium at 0.75× is close enough to native *and* a laptop-dGPU tier (not a phone tier), so FTLE is both affordable and legible there. (Custom can force FTLE on at *any* render_scale — a curious weak-GPU user can enable it and accept the framerate hit; it's just off by default at the bottom.)

**Extreme is `1.0×` native, not supersampled.** 16× ensemble SSAA already handles the classification-edge aliasing that matters here; supersampling (`render_scale > 1`) on top would only clean second-order raster-grid aliasing at 2.25×+ the memory — not worth it, and it would make the tier's cost display-dependent (48 GB at 4K). So Extreme stays a fixed, predictable native 16×-SSAA preset that allocates cleanly on a 24 GB card even at 4K. Supersampling remains available as a **Custom-only** option for anyone who specifically wants raster-edge AA and has the VRAM (§5 / Custom slider).

**The `SSAA` column is a *label*, not a constraint — E is free-valued.** Hardware MSAA is locked to powers of two (2×/4×/8×/16×) because GPU coverage masks are; *our* SSAA has no such limit — each sample is an independent simulation resolved in a shader, not a fixed-function coverage sample, and the Halton offset sequence (sampling note) gives good sub-pixel coverage at **any** E, not just powers of two. So the controller's fine internal rungs can step E through 2, 3, 4, 5, 6… smoothly; only the six *named tiers* snap E to 1/3/7/15 so the labels read as the familiar "2×/4×/8×/16×". `samples = E + 1`; the "N×" label is just `samples`.

### 4.1 The same six tiers on the three axes

The table above is the **resource ladder** — what each tier spends per frame. Under the three-axis
model (`principia_dd_telemetry_and_tiers.md` §3.5) that is one axis of three, and each tier also sets
the other two:

| Tier | `eps` — convergence target | frame budget | hard cap |
|---|---|---|---|
| **Potato** | `1e-1` | 41.7 ms floor acceptable | 25% of available memory |
| **Low** | `5e-2` | 33 ms | 35% |
| **Medium** | `2e-2` | 16.7 ms goal | 50% |
| **High** | `1e-2` | 16.7 ms | 60% |
| **Ultra** | `5e-3` | 16.7 ms | 70% |
| **Extreme** | `1e-3` | best effort | 80% |

**Every number in this table is a placeholder** and is labelled as one in the config. They are chosen
to be in the right ballpark and ordered correctly, nothing more. **The calibration campaign
(`principia_dd_telemetry_and_tiers.md` §7) sets the real values** by running each device across the
whole ladder, including settings it will obviously fail, because a boundary cannot be seen from one
side of it.

**Three things about the shape that ARE decided:**

- **`eps` tightens with tier.** It is the tolerance in `spread_shape`'s own units and means the same
  thing on every slice, so it is the axis that makes two tiers comparable. The range spans the
  tolerance study's sweep (`1e-3` to `1e-1`).
- **The hard cap is a FRACTION of available memory, not an absolute.** Available memory is a
  snapshot rather than a contract (§6.2 of the telemetry doc), and on Apple silicon the GPU competes
  with everything else running. **A fixed byte count would be wrong on every device but the one it
  was measured on.** The fraction leaves headroom proportionally, which is also the good-citizen rule
  in §6.4.
- **Always report which axis bound** — converged / budget-bound / cap-bound. A tier that silently
  misses its target is the failure mode the three-axis split exists to prevent.

**The target for Potato is "runs acceptably on a phone"**, not a specific handset. That is the v0.5
acceptance criterion: falsifiable, testable on hardware already owned, and independent of the numbers
being right.

### Total GB (payload + render targets) by tier × display resolution

| Tier | 1080p display | 1440p display | 4K display |
|---|---|---|---|
| Potato | 0.04 | 0.07 | 0.16 |
| Low | 0.09 | 0.16 | 0.36 |
| Medium | 0.41 | 0.73 | 1.65 |
| **High** | **1.38** | 2.45 | 5.51 |
| Ultra | 2.70 | 4.81 | 10.82 |
| Extreme | 5.36 | 9.53 | **21.43** |

*(Figures include the render-target term (3 display-res + 3 render-res buffers), at the 144 / 96 B widths plus the 16 B word where the tier has it (R-40 / D6). Split at 4K: Potato 0.05 payload + 0.11 targets — the display-sized buffers dominate the low tiers; High 5.31 + 0.20; Extreme 21.23 + 0.20.)*

**Reading it:** *High @ 1080p ≈ 1.4 GB* is the sweet spot. *Extreme @ 4K ≈ 21 GB* is the deliberate "melts current top-end" corner (fits a 24 GB 4090 at 4K; a later card runs it easily — the rung's already there). The low tiers are cheap because `render_scale < 1` shrinks *both* payload and render targets — Potato@1080p renders at 480×270 and lands at 0.04 GB (most of it the display-sized swap chain), genuinely a "does your potato run it" fallback. Every tier's render pixels are still real, sharp samples at the screen floor; the low tiers just render fewer of them and upscale.

---

## 5. Controller levers, ranked by impact

Under memory/compute pressure, auto-mode pulls in this order (top levers cut **both** memory and compute):

| Lever | Effect | Scaling |
|---|---|---|
| **render_scale** | **quadratic** — cuts render_px, hits payload + render targets + compute together | 1.0→0.5 is 4× on everything (locked-to-native disables this) |
| **E (SSAA)** | `(E+1)` linear on payload + compute `2(E+1)` | 16×→1× is 16× |
| **refinement floor** | stop subdividing coarser than pixel-size (tiles 2× pixels etc.) — fewer live samples + less compute, *under motion only*; snaps back to the pixel floor at rest — must-split above the floor is the at-rest target; during a gesture the frame budget governs (R-108) | view-relative; a motion lever, not a tier setting |
| **checkerboard** | compute half the render pixels per frame while the playhead advances, reconstruct the rest from `t−dt` (SSAA-resolved) — **~2× per-frame compute, saves NO memory**; self-erases at rest (`principia_checkerboard_contract.md`) | throughput-only motion lever; three-state user setting (permanent default / motion-only / off); orthogonal to and stacks with render_scale |
| **FTLE** | ×1.5 (drops the shadow + its compute) | on/off |
| **word** | +11–17% | on/off |
| **display resolution** | *scale, not quality* — a given (the window); not a lever, but sets the baseline everything multiplies against | linear in area |

`render_scale` is the heaviest hammer (quadratic, touches all three costs) — but it's the *uniformly-soft* one, so for a research view the controller may prefer E and the refinement floor first, reaching for render_scale under harder pressure. The **refinement floor** is the motion lever: during an active pan/zoom, let quads stop one or two levels above pixel-size (coarser, fewer samples, cheaper) and snap to the true pixel floor at rest — this is the "get the gist while moving, sharpen on stop" behaviour, and it is *not* a persistent quality setting (it lives in the scheduler's motion adaptation, not the tier). When **locked to native**, render_scale is off the table and adaptation runs on E → refinement-floor → FTLE → word — with the timescale caveat that **E and the refinement floor move live under motion** (copies drop/respawn without invalidating the nominal; the floor is view-relative), while **FTLE and word are sim-key rung components** that change only at natural invalidation moments (chart change, restart, re-detect — quality/device note §6), never mid-march.

> **Memory is the hard clamp; compute is soft.** Available memory / `adapter.limits` sets the ceiling (exceed → allocation failure, crash). Within it, compute sets frame rate (exceed the budget → slower, graceful). Auto-mode picks the highest tier whose **total** (payload + render targets) fits with margin, then pulls **render_scale / E / refinement-floor** under *motion* to hold interactivity, restoring at rest.

**Internal rungs are finer than the six names.** The ladder has ~8–12 rungs in total (R-89; quality/device note §3): the six named tiers are pinned rungs on it, and the rungs between them are unnamed internal steps (moving render_scale, E, and the refinement floor semi-independently) for smooth adaptation — the names are user-facing presets; the fine steps are numbers. **Custom mode** exposes `render_scale` (0.25–2.0; <1 performance, >1 supersampling), `N`, `MAX_REL_DEPTH`, E, and FTLE directly (arbiter off), plus the lock-to-native toggle.

---

## 6. Auto-mode tier selection

**At boot (device characterisation):**
1. Probe memory (`maxBufferSize`, `maxStorageBufferBindingSize`; unified-memory devices get a larger effective budget than discrete VRAM — unified vs discrete heuristic).
2. Detect the **display resolution** (sets the baseline `display_px` everything multiplies against).
3. Pick the **highest tier whose total (payload + render targets, at the display resolution and the tier's render_scale) fits with margin** — leave headroom for the remaining overhead (bake, staging, driver). Budget the *process* estimate, not raw payload. On a big display + weak GPU, a tier's `render_scale < 1` is what makes it fit.
4. Warmed-up compute probe at that tier's `2(E+1)` trajectory count (and its render_px) checks the frame budget; step down if compute-bound (not memory-bound). **This matters most at the Medium boundary** — Medium is the first FTLE tier, so its trajectory count *doubles* vs Low; a weak GPU may fit Medium's memory but fail its compute probe and be held at Low.

**Live:** pull **render_scale / E / refinement-floor** down the sub-rungs under motion; restore at rest. If **locked to native**, render_scale is held at 1.0 and adaptation runs on E and the refinement floor only. Never exceed the memory ceiling (the hard clamp) — live adjustment operates *within* the boot-selected memory tier.

**Examples** (totals at the stated display; memory is rarely the binding constraint above the low end — compute usually is):
- **16 GB M-series Pro, 1440p:** boots **High** (~2.4 GB) comfortably, reaches **Ultra** (~4.8 GB); compute-bound not memory-bound — drops E under motion.
- **8 GB discrete, 1080p:** **High** (~1.4 GB) or **Ultra** (~2.7 GB) with wide memory margin; frame budget decides.
- **2 GB integrated laptop, 4K panel:** memory-tight — **Low** (0.36 GB, renders at 0.5×→1920×1080) is the safe boot; **Medium** (1.65 GB with FTLE) may fit memory but likely fails the compute probe (FTLE's 2× trajectories on a weak iGPU), so auto holds Low.
- **Phone (~1 GB), 1080p:** **Potato** (0.04 GB, 480×270 upscaled) or **Low** (0.09 GB) — both FTLE-off, which is what keeps them affordable on a phone GPU (the gating is doing compute work here, not memory).

---

## 7. The "are you sure?" safety system (three severities, none blocking)

Warnings fire **only on an attempted change** to a setting the profile says the device can't handle well — never on current state, never nagging.

| Severity | Trigger | Dialog |
|---|---|---|
| **Green** | fits memory with margin AND meets frame budget | apply silently |
| **Yellow** | fits memory but compute probe says it'll run poorly (below the playable threshold) | "This will run at ~X fps and cause heavy GPU load / heat. Proceed?" — with a **"use recommended instead"** one-click escape |
| **Red** | total (payload + render targets) exceeds the memory ceiling (will likely fail to allocate) | a **starker** variant — "This needs ~X GB; we estimate your device has ~Y GB. It will probably fail to load. Proceed anyway?" — **still not blocked** |

**Rules:**
- **Red does not prevent** — it's a louder dialog making very clear it will *probably fail*, but the user has final say (workstation owners, or people who want to see it try).
- **"Don't warn me again for this specific choice" checkbox** on the dialog — per-setting suppression (dismissing 16× doesn't silence a future *different* risky choice).
- **Runtime graceful fallback underneath:** even if a forced setting fails to allocate, catch it and drop to a working tier with a message ("Couldn't allocate Extreme, fell back to High") — never a crash or black screen. The warning is the first line; graceful fallback is the second.

> **The warning is only as good as the profile.** The whole system depends on trustworthy boot-time characterisation — under-detect and it warns people off settings they could run (timid); over-detect and it waves through melty ones. Downstream of the device probe.

**Build-time tunes (measure against feel, don't hardcode now):** the yellow/red fps thresholds (an interactive explorer tolerates lower fps than a game; paused exploration tolerates less than active panning); the memory-fit safety margin (compare *process estimate* vs budget, since payload ≠ process).

---

## 8. Caveats

- **Totals ≠ full process budget.** The §4 totals are payload + render targets; real footprint still adds the bake texture, staging buffers, transient export/resize allocations, and driver/browser/OS overhead. Budget the process estimate with margin.
- **Render-target buffer counts are estimates.** `k_d ≈ 3` display-res (swap chain 2–3 + final composite) + `k_r ≈ 3` render-res (backdrop + compositing intermediates) — the real numbers depend on the final compositor design; confirm once the render graph is built. They only move the low-tier figures materially (where targets are the big fraction).
- **Buffers are sharded** (WebGPU 128 MiB/binding, 256 MiB/buffer baseline) — a tier's multi-GB payload is many physical quad-chunk buffers, not one allocation.
- **Live-sample count, not full render frame, at partial refinement.** Figures assume steady-state refinement (screen floor, one sample per render pixel) at the tier's `render_scale`. Under lockstep's lazy-in-space refinement, only refined quads carry full samples — actual resident payload is often below the figure (the figure is the fully-refined ceiling the controller budgets for). Render targets, by contrast, are allocated up front at full render resolution regardless of refinement.
- **Compute is the usual real limiter at High+ on capable GPUs** — memory tiers are generous there; the controller's live render_scale / E / refinement-floor adjustment is doing frame-rate work, not memory work. Memory matters most at the low end (phones, integrated laptops, big displays on weak GPUs), which is exactly where the O(1)-in-time lockstep design and `render_scale < 1` pay off.
