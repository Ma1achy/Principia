# Principia — scheduler contract

*Seventh doc. The CPU brain: which quads exist, in what order they refine, what's cached, what's evicted. The *policy* — priority formula, split/keep/merge, eviction, the frame loop — is recorded in Part 6 and Part 7 and treated as settled (the refinement criterion itself is `principia_dd_refinement_policy.md`; see the note at the head of Part 6). This doc contracts the seams that make that policy safe against everything else: the non-determinism firewall, the density-is-not-probability rule, preview/refine caching, the depth model for infinite zoom, and terminal-vs-refinable tied to the two floors.*

---

## Part 1 — The firewall: the scheduler is arbitrary about *what* it looks at, never about *what* it sees

Every prior doc held one invariant: the physics is deterministic; only precision differs. **The scheduler is the first component that is legitimately, deeply non-deterministic — and that is fine, provided it is walled off.** Which quads exist, their refinement order, cache contents, eviction history all depend on camera path, frame budget, idle timing. **Two users exploring the same region get different quadtrees.** That must stay cosmetic, never scientific.

**The load-bearing rule:**

> A quad's `SimState` is a pure function of `(IC, sim key, playhead t)` — the march is fixed-`dt` deterministic (frame loop, Part 7), so the state at any `t` is identical whether reached by continuous marching, catch-up after reveal, or re-boot after eviction; at frame 3 or frame 300; on any device. The scheduler decides *which* ICs get computed and *when*. It must never influence what any given IC computes *to*.

This is the CPU-brain twin of the CPU/GPU divergence rule. It makes the non-determinism a matter of *the map filling in differently on screen*, never *different answers*. The violations an agent will reach for, all forbidden:

- **No seeding integration from a cached nearby state.** Every IC integrates from its own decoded `(m,r,p)`, never from a neighbour's endpoint or a parent quad's trajectory. Warm-starting would make the result depend on visit order.
- **No reusing a neighbour's `SimState`** as a stand-in for an unfinished quad. Ancestor *fallback* is a display trick (draw the parent upscaled) — it never writes a child's payload.
- **No carrying scheduler state into the payload** — not cache age, not compute order, not priority, not which frame it landed on. The payload compatibility signature (integrator/render docs) contains sim-key fields only; scheduler state is categorically excluded.

If a payload's contents could differ between two sessions that scheduled differently, the firewall is broken.

---

## Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)

The chart-decoder contract states it ("Links carry a measure"); the render contract echoes it. **The scheduler is where the violation would actually happen**, because the entire priority system *deliberately* over-samples boundaries.

> The quadtree is a rendering / compute-allocation structure. Its leaf density is `1 − coherence`-driven — high where boundaries are filamentary, low in smooth basins. This is **not** physical probability density, and it must never feed a quantitative claim.

There are three distinct notions of density, and they are not the same thing. *Exploration density* is what makes interaction smooth and useful. *Refinement density* is where the renderer spends compute because the image or the dynamics are complicated. *Physical/statistical density* is what a statement like "fraction of IC space showing X" needs. The quadtree concentrates compute at basin boundaries because they are visually and dynamically interesting, not because those ICs are more probable. Unless stated otherwise, the default goal is smooth, controllable exploration, not uniform sampling.

Two instruments, kept separate:

- **Adaptive quadtree — for *looking*.** Concentrates compute where the picture is interesting. Leaf density reflects visual/dynamical complexity, not how probable those ICs are.
- **Uniform re-sampling — for *measuring*.** Basin fractions, island prevalence, fractal dimension come from a **flat grid at fixed resolution** (the offline/export path: `SAMPLES_PER_QUAD_AXIS` over a uniform dispatch, no adaptive subdivision), or from a known measure with the link/warp Jacobian correction (chart contract Part 2.5).

This is the scheduler-side member of the same family as the link-measure honesty rule and the chart `system_image` descriptor: **adaptive structure for viewing, uniform sampling for statistics.** Counting quadtree leaves to estimate "fraction of IC space doing X" is the canonical error, and it is the scheduler's job not to make that count look meaningful.

---

## Part 3 — Depth: infinite zoom by default

**Mandelbrot-style infinite zoom is the default.** Absolute reachable depth is **emergent from physics and precision, not imposed by a constant.** Three orthogonal concerns that an earlier single `MAX_DEPTH` constant conflated:

| Concern | Governed by | Nature |
|---|---|---|
| How much detail *below the current view* | `MAX_REL_DEPTH` | performance toggle, slides with the camera |
| How deep you *can* zoom | decode floor + integration floor | physics/precision, emergent |
| Whether to split *this* quad | `Policy::Tolerance` (`principia_dd_refinement_policy.md`, R-15) | scientific — the adaptive part |

### `MAX_REL_DEPTH` (renamed from `MAX_DEPTH`)

A **view-relative performance/quality window**: refine at most N levels below the current camera depth. It *follows the camera down*, so infinite zoom works — descend, the window slides with you, always a bounded amount of work below the view. Sensible relative budget is small (≈ 4–8 below the view; the earlier defaults of 8 / 10 / 12–14 were written as *absolute* and conflated "how deep total" with "how much detail below the view").

**This rewrites the split predicate — the critical correctness point.** The earlier absolute form was:

```
split(C) ⟺ S_quad > τ(ℓ)  ∧  ℓ < MAX_DEPTH        # absolute — caps infinite zoom at ~14
```

Under the sliding interpretation it becomes:

```
split(C) ⟺ in_view(C) ∧ tile_size(C) > pixel_size                       # must split above the screen floor (policy §0.1, R-88) — the at-rest target (R-108)
         ∨ policy_splits(C)  ∧  ℓ < camera_depth + MAX_REL_DEPTH           # policy_splits: Policy::Tolerance (R-15); the cap binds in view below the floor and off screen (R-98)
```

These are **different gates**. An agent working from the old absolute form will silently cap the zoom. The rename forces the predicate to be rewritten.

**`MAX_REL_DEPTH` is not on the sim key.** It is view-relative scheduler state, exactly like `frame_budget` and the in-flight-job limit. Lowering it while zoomed invalidates *no payload* — it just stops *scheduling* deeper quads; already-computed quads stay valid and cached. It sits cleanly on the "what we look at" side of the Part 1 firewall, never the "what we compute" side. It belongs with the scheduler knobs, never with `T` / `dt_macro` / thresholds.

### The absolute floor is emergent

How deep you *can* go is answered by the two floors (integrator + deep-zoom docs), not a magic number: the **decode floor** (`AT_F32_FLOOR`, pushed to ~depth 50 by the linearised decoder) and the **integration floor** (substep-saturation-dominated, the v1 KS-less limit — a confidence gradient, non-terminal). The cap on absolute depth *emerges* from these; it is not a policy constant.

---

## Part 4 — Terminal vs refinable, tied to the two floors

The `ReadyRefinable` lifecycle state is the whole slippy-map trick: it distinguishes **"done enough to display"** from **"done forever."** The scheduler revisits refinable quads when budget allows; it never revisits terminal ones. So "done forever" needs a precise definition — but first, the thing that governs refinement in *normal* use is not a floor at all:

**Whether a quad splits is `Policy::Tolerance`'s decision (R-15; `principia_dd_refinement_policy.md` §0.1, §1).** Split iff any footprint in the quad is unresolved against the single tolerance `eps`. In view, the camera decides depth (a quad may stop only once its texel is at or below one screen pixel) and the criterion decides the *order* the complete tree arrives in. Off screen, the criterion decides depth. This Part defines the floors that decision runs inside.

**The everyday boundary is the screen-space floor (view-relative, not a terminal):**

- **Screen-space floor — `tile_size(quad, zoom) ≤ pixel_size`.** Above it, every in-view quad **must split** (policy §0.1, R-88): the camera decides depth. **This is the at-rest target (R-108):** during a gesture the frame budget governs — ancestors show, so there are never blanks — and completeness resumes at rest (caching Part 6's two regimes). At and below it, splitting produces *sub-pixel* samples that cannot be displayed distinctly, and the criterion may **supersample** there where a footprint is unresolved (§0.1; R-88). This is the **everyday** refinement boundary: in normal exploration you hit it far shallower than any precision floor. **It is view-relative, evaluated live against the current zoom — NOT cached as a quad fact and NOT terminal:** zoom in and the same IC-space patch covers more screen, its tiles regrow above pixel size, and refinement *resumes* (real new samples). So a screen-floored quad is "done *at this zoom*," not "done forever." (`MAX_REL_DEPTH` caps **every split beyond the screen floor** — in-view supersampling below it and off-screen policy splits alike (R-98) — a budget lever, never a stop above it in view: `MAX_REL_DEPTH ≥ screen floor` always, R-88.)

The **true terminal floors** (done forever — genuinely no more information extractable, cached as quad facts) apply only when someone zooms *past* pixel-matching into extreme zoom:

- **Decode switchover, NOT a stop — `DECODE_SWITCHOVER`.** When the *full nonlinear decoder's* adjacent samples collapse to bitwise-identical ICs (~depth 20–23, or shallower with tiny `q₁,q₂`), that is **not** the floor — it is the trigger to **switch to the linearised decoder** (deep-zoom note §2, "Switchover"), with depth `ℓ_switch = 20` as an upper bound, whichever comes first (R-90; lowering's `SWITCH` = `ℓ_switch`), which anchors the precision-critical part in f64 on the CPU and restores the distinction. Refinement **continues** on the linear path (depth ~23 → ~50). The sample-collapse symptom on the *full* decoder means *switch decoders*, never *stop*. (This is the fix that the old single "AT_F32_FLOOR = stop" reading would have strangled — it fired the stop exactly where the linear decoder should engage.)
- **Decode floor proper — `AT_F32_FLOOR` (linear-decoder only).** Only when the *linearised* decoder's samples collapse to bitwise-identical ICs (~depth 50+) is f32 genuinely exhausted — the quad-local offsets `δ` are too small for f32 to represent distinct neighbours, and no fix exists short of higher precision (deferred KS/arbitrary-precision route). **This** is the true decode floor: stop, terminal. It is a deep-zoom *backstop*, not an everyday mechanism — most sessions never reach it (the screen floor stopped them long before). The same visible symptom (sample collapse) means *switch* on the full decoder and *stop* on the linear decoder — the response keys off **which decoder is active**.
- **Integration floor:** substep-saturation-dominated — the samples are *distinct* but their outcomes are f32-integration-limited, under-resolved (integrator doc Part 6). Orthogonal to decode precision: a quad whose `suspect_fraction` stays high and whose samples are **substep-saturated** (the confidence flag, not a sample-terminal — the trajectories *did* reach outcomes, just under-resolved) is at the integration floor — **stop refining** (further splitting won't resolve what the integrator can't), flag it floor-limited, don't re-queue as refinable. A *refinement-stop*, not a *sample-terminal*: samples have real outcomes with low confidence, the quad is done subdividing.

So the stop hierarchy, by how often it fires: **screen-space floor** (everyday, view-relative — in view, quads above it must split; below it the criterion may supersample) → **`MAX_REL_DEPTH`** (caps every split beyond the screen floor — that supersampling and off-screen policy splits, R-88, R-98) → **`DECODE_SWITCHOVER`** (extreme zoom — switches decoders, not a stop) → **`AT_F32_FLOOR`** on the linear path + **integration floor** (extreme/deep — true terminals). In view above the screen floor a quad splits (the at-rest target; during a gesture the frame budget governs, R-108); otherwise refinement happens iff the policy splits **AND** no cap or terminal floor has fired (R-88).

Everything else that is `Ready` but kept by the policy, or screen-floored at the current zoom, is **refinable**: displayable now, revisited if budget frees up (or on zoom-in, for screen-floored quads). Only the true precision/integration floors are **terminal** (done forever); refinable quads are paused.

---

## Part 5 — Preview vs refine is a second sim key

`PREVIEW_MODE` (`QuadRequest.flags` bit 3, table below) = reduced horizon + coarse integration during interaction; full quality on idle. Reduced horizon is a **different `T`**, and `T` is **on the sim key** (integrator doc). Therefore:

- **A preview payload and its refined payload are different payloads of the same quad.** The payload compatibility signature already covers horizon, so the cache distinguishes them correctly — but the contract must state it: **preview and refined are distinct cache entries.**
- **"Sharpen on idle" is a recompute, not an in-place upgrade.** The refined quad is a fresh integration at full `T`; it *replaces* the preview in what's displayed, it does not edit it.
- **The cache must never serve a preview payload where full quality was asked** (or vice versa) — and a quantitative export must never read a preview quad. Preview is for interaction responsiveness only; it is scientifically inert.

This is preview/refine living correctly on the *compute* side of the firewall: they are genuinely different computations (different `T`), so they are different payloads — not two views of one payload.

### `QuadRequest.flags`

The per-quad dispatch request carries a bit-packed `flags` word:

| Bit(s) | Name | Meaning |
|---|---|---|
| 0 | `DECODE_MODE` | full (0) or linearised (1) decoder (deep-zoom note §2). Linearised once `quad.collapsed` — the per-quad flag set when the full decoder's adjacent samples give bitwise-identical ICs — or past `ℓ_switch`, whichever first (R-90) |
| 1 | `ENSEMBLE_ENABLED` | dispatch `E` jittered copies per grid position |
| 2 | `FTLE_ENABLED` | compute the full Benettin FTLE (tier-gated) |
| 3 | `PREVIEW_MODE` | reduced horizon, coarse integration (this Part) |
| 4 | `FULL_RETENTION` | skip reduction and keep every per-sample result. **Owner: the measurement path** (R-39) — the Measure tool and matched N / 2N renders, a uniform grid with every sample retained (`principia_render_gui_spec.md` §G10) |
| 5 | `COMPUTE_IC_DESCRIPTOR` | write the `ICDescriptor` buffer alongside the results (dd_decoder §3.6) |
| 6–7 | reserved | not `DEBUG_MODE`, which is a baked kernel variant (R-41) |

When `DECODE_MODE` is set, the reference IC `x₀` and the Jacobian `J_D` travel in a **separate uniform buffer**, bound only for linear-path quads, not inside the request. They are about 45 floats and unused at shallow zoom. The request also carries the quad's centre and half-width, computed on the CPU in f64 and passed as f32. The GPU computes sample positions from them as `u = centre + half·(2t − 1)` (deep-zoom note §1), never from the min/max bounds, which are kept for CPU-side scheduling, culling and debugging.

---

## Part 6 — The settled policy

Recorded so the contract is self-contained. **The split decision is `Policy::Tolerance`'s** (R-15, `principia_dd_refinement_policy.md`); this Part keeps the scheduling mechanics around it.

**Priority.** `P_tile = w_v·P_visible + w_z·P_zoom + w_c·P_complexity + w_f·P_focus`, defaults `w_v=10, w_z=2, w_c=3, w_f=1`. Visibility dominates (never compute off-screen); complexity (`1 − coherence`) drives adaptive refinement; zoom-match is a tiebreaker; focus (inverse distance to the **pointer** while it is in view, to the viewport centre otherwise — cursor bias, R-55; its weight and decay are still to be written) is subtle. Weights exposed in research mode.

**Split/keep/merge.** Decided by `Policy::Tolerance` (R-15): split iff any footprint is unresolved (`spread_shape > eps`, the copies disagree on event class, or the footprint is undetermined — policy §1); the stop rule is `alpha_area` (§2); merging is the split rule read backwards (§3); the decision variants are in §6. The scheduler applies it **and** `ℓ < camera_depth + MAX_REL_DEPTH`; in view above the screen floor the quad splits regardless (R-88). Guards checked first: terminal (Part 4) or offscreen → stop.
*Superseded (R-15), kept for the record: split if any spread/impurity threshold was exceeded (`outcome impurity`, `S_n`, `S_t`, `S_L`, `S_f` when `FTLE_VALID`, `S_D`, low ensemble agreement — now `spread_event`, R-18 — persistent parent-child disagreement); tiebreakers `retrograde_fraction ≈ 0.5`, high `mean_orbit_count` spread, near the locked pixel; keep coarse if dominant purity high, all spreads low, summary visually stable, already finer than screen demand; merge/deprioritise if offscreen, overresolved, indistinguishable from ancestor, or under cache pressure; default keep.*

**Eviction.** Cost-weighted LRU: eviction resistance ∝ `computeCostMs`. Expensive (deep, close-encounter, high-substep) quads resist eviction; high-coherence smooth quads are cheap to recompute and evicted first. This directly serves the firewall — evicting and recomputing is *safe* precisely because the payload is pure (Part 1), so LRU can be aggressive without scientific consequence.

**Cancellation.** WebGPU dispatches can't be cancelled mid-flight; the scheduler skips *subsequent* passes for quads no longer visible. In-flight limit 2–4 jobs; an epoch counter discards stale results (a result for a quad the camera has left is dropped, not written — consistent with Part 1: dropping a computation changes nothing about any payload).

**Resolution controls (quality = sample density, NOT viewport):** `SAMPLES_PER_QUAD_AXIS` (`N` — the sample grid inside a quad; each sample is a full simulation, so this is *the* quality/memory/compute driver — too low misclassifies a quad as coherent by undersampling itself) and `MAX_REL_DEPTH` (view-relative refinement window — how deep the quadtree subdivides below the camera). **Both are tier-gated with a Custom override.** Per-tier values in the memory-tiers table (`N` ranges 8–32 across the six tiers; rel-depth budget likewise per tier). **There is no sample↔pixel interpolation knob** — a sample rasterises directly to its screen-space footprint (a *tile*); **one sample, one tile, no interpolation**. (`render_scale` — memory-tiers §2 — is a different thing: it sets how many *render pixels exist* (internal raster scale, upscaled to display); the screen floor then pins one sample per *render* pixel exactly as stated here. A raster-scale knob, not a sample-density or interpolation one.) (sharpness comes from subdividing quads — real new samples — not from interpolating a sparse sample grid to more pixels, which would fabricate/smooth over the filamentary structure). The only sample↔pixel combining is the honest direction: SSAA ensemble resolve (many sub-pixel samples → one pixel colour, Part 9). The quality tier also gates the resident co-computations (ensemble copies + per-sample Benettin shadows).

---

## Part 7 — The frame loop (lockstep presentation)

*The one genuinely new object of the temporal-architecture reversal (ratified). Replaces the pull-based/on-demand loop: a first-class fixed-timestep loop with a global playhead. Same shape as networked-game lockstep and the standard fixed-timestep game loop.*

**The principle: never present mixed time.** Every temporal glitch (frozen rectangle in a live field, region snapping forward, children popping in behind the playhead) is one sin — presenting pixels from different moments as one moment. The barrier makes it unrepresentable. *(The one deliberate, bounded exception is checkerboard motion acceleration — a uniform one-`dt` half-frame skew only while the playhead advances, self-erasing at every rest/pause/endpoint, off in export — `principia_checkerboard_contract.md` §6.)*

**Two tiers — barrier over the LIVE SET, catch-up OFF-LOOP** (a hard barrier over everything would let one slow reveal freeze the loop):

```
LIVE SET     quads at the playhead. March together, barrier-synced,
             presented coherently every frame. Cheap — one dt/frame each.
CATCHING-UP  newly-revealed / newly-split quads marching 0→playhead (or
             resume-point→playhead, caching Part 7) in the BACKGROUND.
             Shown via the moving fallback (blurred live ancestor, animating).
             On reaching the playhead → PROMOTED at the next barrier, atomically.

frame loop (fixed timestep):
  advance every LIVE-SET quad one fixed dt        // cheap, uniform
  BARRIER over the live set only                   // coherence
  promote catching-up quads with quad.t == playhead // atomic, synced
  present                                           // one shared t
  advance playhead                                  // written by the GUI's clock: a SetField marked "no history" (R-101)
```

- **Fixed `dt` per sim-step, decoupled from render frames** (accumulate wall-clock, take fixed steps, render when ready). Playback speed is a `dt`-per-second setting (`ViewUI` transport, read by the GUI's clock, R-101) — never device frame rate — so two runs reproduce and the parity determinism firewall holds.
- **Promotion is gated on time-sync (`quad.t == playhead` at a barrier), never on completion** — the playhead is momentarily stationary at the barrier, so the child-chasing-a-moving-target race cannot occur.
- **Transport controls are free from statelessness:** pause freezes the *playhead*, not the compute (reveals still catch up to the frozen `t` and promote); restart = playhead→0 + discard state; loop = auto-restart at `t_end`. Pausing at `t_end` = the full integration = equal-or-cheaper than the old eager system, at every earlier frame strictly cheaper.
- **Three failure modes, three owners:** *glitch* → the barrier; *stall* → off-loop catch-up; *lie* → blur. This unifies the blur grammar: spatially stale, temporally behind, and being-refined are all one honest signal — **sharp is real, fuzzy is arriving.**
- **Export runs this same loop in BLOCKING mode** (hard barrier over *all* visible quads per captured frame — no fallback in exported pixels); interactive runs progressive. Same loop, two barrier policies (export contract).
- **Two-regime amendment retained:** during an active gesture the in-motion regime dispatches only the coarse cover (priority scoring bypassed, `PREVIEW_MODE`); catch-up compute is what's being scheduled. At-rest resumes on debounce. The loop never awaits GPU work (responsiveness invariant), and the loop itself runs in the render-loop worker (caching Part 6a).

## Part 8 — Continuous refinement & the live-to-live handoff

*Refinement now runs during playback against evolving state, not once against final outcomes — strictly richer signal (end-state impurity misses trajectories that diverge mid-flight and reconverge).*

**Split is `Policy::Tolerance`'s one test (R-15), fed by two orthogonal signals under the same `eps`:** a footprint is unresolved if its spread exceeds `eps` now, or its latched running maximum ever did (R-91).

- **Spatial coherence (instantaneous, two-way):** at the playhead, the quad's samples disagree — different classes (incl. `RUNNING` vs terminal) or `spread_shape > eps`. The existing Part 6 predicate, evaluated live. **Symbolic spread (`S_word`) is a diagnostic, not a split input (R-99)** — a footprint reduction over the E+1 copies' free-group words (derived at resolve, sampling/SSAA note; the words are fetched from the **separate word buffer**, not `SimState` — they're the cold field that lives apart, ledger §3.3a), which fires *earlier* than outcome-impurity because words branch before fates differ; it is shown and measured, never read by the split.
- **Temporal accumulators (fixed-size, latching — NEVER a time-series):** the **running max divergence** is the latch, and it is **per footprint** (largest bundle spread the footprint has shown so far — catches diverge-then-reconverge; the one accumulator that feeds "unresolved", against `eps`, R-91; a quad is unresolved if any of its footprints is, R-99). The **running mean divergence** and the **first-divergence time** (write-once) are diagnostics, not split inputs (R-99). The divergence trend and the separate thresholds `θ_s`, `θ_max`, `θ_trend` are dropped (R-91). O(1), updated in place on the GPU each step, reduce-before-evaporate — the CPU sees scalars, never history.

**The temporal signal latches:** running-max only grows; a quad proven interesting *stays* refined even if currently calm — correct for crystallisation (boundary structure accumulates, doesn't flicker). **The latch lives with the resident quad (R-99):** when the cache evicts or merges the quad, its footprints' latches go too, so the latch never pins memory; a region revisited after eviction re-discovers its structure.

**The live-to-live handoff (refinement must never interrupt a region's animation):**
1. Quad flagged at playhead `t`. **The parent keeps marching and rendering** — no freeze (the moving fallback).
2. Children spawn and catch up off-loop, chasing the advancing playhead.
3. Swap parent→children **atomically at a barrier, gated on time-sync** — both at the same `t`, seamless, at higher resolution.

Transiently both parent and children integrate (bounded by the mid-split count) — a small compute bump, never a memory one. The failure this prevents: frozen rectangles clustering exactly where structure is (you only split where the user is looking).

---

## Part 9 — Ensemble / SSAA sampling (dispatch rules)

*The scheduler-side of the sampling/SSAA model (full rationale in `principia_sampling_msaa_note.md`). The data/render treatment is uniform; the dispatch treatment has two rules.*

**Ensembles only on nominal samples — the no-recursion rule.** A nominal grid sample (`SAMPLES_PER_QUAD_AXIS²` per quad) spawns E ensemble copies when `ENSEMBLE_ENABLED`. **A copy is a full uniform `SimState` (identical to the render graph) but a scheduler *leaf* — it never spawns its own ensemble.** `ENSEMBLE_ENABLED` is checked only for nominal samples; copies dispatch ensembles-off, by construction. This is *structural uniformity ≠ role uniformity*: a copy is a normal pixel at the data/render level (where they're identical) and a leaf at the dispatch level. One flag-check bounds `E → E`, never `E² → E³`. (Same pattern as the FTLE shadow one level down: a trajectory, not a sample.)

**Offsets are a fixed low-discrepancy prefix.** A footprint has E+1 samples, `copy_index` 0..E: copy 0 is the un-jittered centre, and copies 1..E take points 1..E of a **Halton (2,3) sequence**, centred (minus ½) and scaled to the footprint (R-80) — *not* live-stochastic, *not* a naive grid. Fixed because lockstep marches the scene (no cross-frame Monte-Carlo accumulation; a new sample costs O(t) not O(1)), so Principia lives in the low-sample-count regime where a fixed well-distributed set wins; low-discrepancy for best coverage at low E. Deterministic ⇒ parity-clean (no shared RNG), reproducible spread, no shimmer (time excluded). No per-cell rotation: copy positions are fixed by the pixel alone, which keeps recreate-from-image exact (R-100).

**The copies serve two consumers (double duty).** Same E+1 samples feed (a) the **render-side SSAA resolve** (their colours → the pixel's anti-aliased colour) and (b) the **data-side spread reduction** (their classified outcomes → the footprint spread scalar). Both are per-footprint reductions at the **resolve stage**, twins of each other (colours vs outcomes). Two firewalls: colours resolve terminally on the render side (never read as data); outcomes reduce on the data side. The footprint spread is **derived at resolve, not stored per-sample** — consumed live for display and **aggregated into `QuadReduction`** (the quad-level `ensemble_spread` / agreement the scheduler reads for splitting, Part 6). So spread is a split signal *and* a live-colourable footprint quantity, but it lives at the footprint/quad level, never as a `SimState` field.

**Motion tier-gating (set by the quality controller).** Copies are full sims that march every frame, so live playback costs ~`2(E+1)` marching trajectories/pixel (base + shadow, ×(E+1)). The **quality/device controller** (`principia_quality_device_note.md`) sets `e_motion_gating` from its cost model: **E reduced/off (→0/1) during an active live march, full E at rest and in export** (where the blocking barrier already accepts cost). The boot probe answers "can this device afford `2(E+1)`/pixel at interactive resolution within the frame budget?" — the previously-open question (temporal note item 3) is resolved by the characterisation phase, which is exactly where the frame-loop cost first becomes measurable. E is one knob on the quality ladder; the motion offset drops it during gestures/catch-up.

---

*The scheduler is arbitrary about what it looks at and exact about what it sees: states are pure functions of `(IC, sim key, t)` under a fixed-`dt` march, so eviction, re-order, catch-up, and re-boot are all free. Adaptive density is for viewing; uniform sampling is for measuring. Infinite zoom is the default; `MAX_REL_DEPTH` is a view-relative window that rewrites the split gate; absolute depth is emergent from the two floors. The frame loop presents only the barrier-synced live set; catch-up and refinement run off-loop and promote in atomically; refinement splits where a footprint is unresolved — its spread over `eps` now, or its latched running maximum ever (R-91) — and hands off live-to-live — the screen is never mixed-time, never frozen, never lying.*
