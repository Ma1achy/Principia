# Principia — temporal architecture reversal

***RATIFIED*** *— and propagated: the rename and the contract edits below have been executed (render, scheduler Parts 7–8, caching Part 7, export rewrite, ledger, integrator §3.7). This note remains as the decision record and its reasoning. This is a significant change: it obsoletes the checkpoint-as-animation model and touches the render / scheduler / caching / export contracts. The physics drill-downs (decoder/integrator/encode/colouring) are **untouched** — this is all above the payload waist.*

---

## The decision

**Reverse the temporal model from eager-in-time to lazy-in-time (lockstep-live-march).**

- **Was (V2):** integrate every pixel 0→`t_end` up front, store the trajectory as an M-knot checkpoint array per pixel, animate by replaying the stored knots. Enables random-access scrub.
- **Now:** a global playhead marches all in-view pixels forward one `dt_macro` at a time; render the **live state** at each step; **store no history** — discard each step's state after rendering. No scrub; playback only. (Scrubbing in the GUI re-integrates; see R-66.)

**Why (the load-bearing reason): Principia is memory-bound, and only because eager-in-time forced it to be.** The three-body integration is compute-light (a few flops/substep/pixel). The large thing was the checkpoint storage — pixels × M × float4, resident, plus the write/read bandwidth to fill and replay it. That footprint existed *solely* to service scrub. Remove scrub → remove eager → remove stored history → footprint collapses to the **visible working set** (one live state per on-screen pixel, constant, viewport-bounded, does not grow with `t`).

**The V2-on-M3 diagnosis:** V2 felt slow on an M3's excellent bandwidth. That's the signature of *memory-traffic-bound*, not compute-bound and not bandwidth-ceiling — writing full trajectories out and streaming them back per frame saturates even great bandwidth, plus allocation pressure. Direct evidence the eager-checkpoint approach was the bottleneck.

**Device reach (why this matters for the goal):** the artefact is *a thing people open in a browser*. VRAM is the harshest low-end constraint (integrated GPUs, laptops, phones have compute but little VRAM and choke on WebGPU allocations). Eager footprint (hundreds of MB–GB) → runs on discrete GPUs only. Lockstep footprint (a few MB, viewport-scale) → runs on anything with WebGPU. **Converting a memory problem into a compute problem moves the load onto the resource the low end actually has.** Failure modes are asymmetric: lockstep degrades to *slower* on weak hardware (graceful); the memory design *fails to allocate and crashes* (hard). Graceful-slowdown beats hard-failure decisively for reach.

**Latency is also better, not just memory.** Current per-quad reveal cost is always `0→t_end` (the max). Lockstep reveal (catch-up) is `0→playhead`, and `playhead ≤ t_end` always, usually `≪`. So average reveal latency is **lower**, and total work is less (lazy in space *and* time — never computes unwatched future). The only thing eager bought was scrub, which we don't want. At `t_end` (if paused there) the two do identical integration — so **"pause at t_end" is equal-or-faster than current, never slower** — and every earlier frame is strictly cheaper.

---

## The rename

**`SimResult` → `SimState`.** The struct is no longer a completed *result* of a full trajectory; it is the *current state* of a marching simulation at the playhead. `SimState` names it correctly. (11 docs reference it — render 7, systems-arch 6, integrator/debug/parity 4/4/3, deep-zoom 3, others 1–2 — a mechanical global rename once this note is ratified. `ICDescriptor`, `QuadReduction` unchanged.)

---

## What changes vs what doesn't

| System | Change | Severity |
|---|---|---|
| Quadtree topology | **none** — subdivides the IC plane, time-agnostic | none |
| Slippy map / fallback | **none** in mechanism — shorter waits; fallback now *animates* (below) | improves |
| Payload | checkpoint **array** → tiny live `SimState`; shape `n(t)` is now a *derived quantity of current state*, not stored | **big simplification** |
| Stability metrics | checkpoint-stored running values → **plain running accumulators** (below) | simplifies |
| Refinement | one-shot on final outcomes → **continuous on evolving state** (below) | **the real new work** |
| Caching | unbounded history buffer → small **viewport-scale current-state** cache | simplifies |
| Render pipeline / swapping | **none** — reads `SimState` known fields; see decoupling below | none |
| Export/animation contract | **rewrite** — it assumed the scrub/checkpoint model | moderate |
| Physics drill-downs | **none** — above the waist | none |

The scary-sounding systems (quadtree, slippy map) are the *least* affected — they're spatial. The data architecture *simplifies*. The one genuine new design piece is continuous refinement.

---

## Stability metrics are just running accumulators (they get simpler)

These are *naturally* "summarise the trajectory as it goes" quantities; the checkpoint array was an imposed storage layer fighting the grain of the math. Under lockstep they are what they always were:

- **Shape position `n(t)`** — not stored at all now; *derived from the current state every step*. The checkpoint array's whole purpose (replay shape over time) is served by the live march. Biggest simplification.
- **FTLE (Benettin)** — already a running sum `S += log(δ/δ₀)` + renorm. Read at the playhead as `λ = S_final/(n·dt)` **with the partial renorm interval finalised** (plain `S/t` systematically under-reads — payload §5). One scalar + the shadow's live state.
- **Ensemble spread** — a footprint reduction over the **E+1 ensemble copies'** current positions/outcomes, **derived at the resolve stage** (never a stored field; the Benettin shadows are never in the spread pool — sampling/SSAA note supersedes the old "N free shadows" framing). Instantaneous from current state; time-avg/max live in `QuadReduction` accumulators.
- **Diffusion** — ⚠ **the one caveat.** It's a *fit* (regress spread vs time, take the slope). A fit is O(1) **only if** accumulated as **running regression state** — the **Welford centered co-moments** (`mean_y, C_ty`, with the time-only moments closed-form; payload §4 / integrator dd §3.5), from which the slope falls out at read. (The naive raw sums `Σt, Σt², Σy, Σty, Σy²` are O(1) too but cancel catastrophically — payload §4 uses the centered form for exactly this reason.) Collect (t,y) *points* to fit later → history grows → the trap. Must be implemented as running co-moments, never stored points.

General rule (same as the drift accumulators already use): **fixed-size, O(1), updated in place each step, never a time-series.** Everything worth animating fits in a small fixed set of scalars per pixel — nothing like the checkpoint array.

---

## Continuous refinement — temporal accumulators + spatial coherence

Refinement now runs *during* playback against evolving state, not once against final outcomes. The split is `Policy::Tolerance`'s (R-15), and two orthogonal signals feed its one test, under the one knob `eps` (R-91):

**Spatial coherence (kept, instantaneous):** at the playhead, do the quad's pixels disagree (different classes / far apart in state)? The classic "boundary through this quad → split." Live, two-way (can un-flag).

**Temporal behaviour (new, fixed-size running accumulators — NOT stored history):**
- **running max divergence** — the latch: the largest bundle spread a **footprint** has shown so far (catches mid-flight divergence that reconverges — invisible to end-state impurity). One float per footprint, `max`-updated (R-99).
- **running mean divergence** — time-averaged spread (distinguishes mild-constant from explosive-occasional). One float. A diagnostic, not a split input (R-99).
- **first-divergence time** — `t` at which spread first crossed `eps`; write-once; proxy for how fast chaos manifests here. A diagnostic, not a split input (R-99).

```
unresolved(f)  ⟺  spread(f, now)          > eps     // boundary now
               ∨  running_max_spread(f)   > eps     // flew apart at some point (latched)
split(quad)    ⟺  any footprint f in quad is unresolved          // R-91; θ_s, θ_max, θ_trend dropped
```

Costs one float per footprint for the latch, held with the resident quad (R-99), plus the diagnostics; O(1) in-place, same reduce-before-evaporate pattern — GPU distills history to a scalar in-thread; CPU sees the scalar, never the history.

**The temporal signal latches (one-way):** running-max only grows (and first-divergence, a diagnostic, is write-once) — so a footprint proven interesting keeps its quad refined even if it currently looks calm (R-99). This is *correct* for the crystallisation movie: boundary structure **accumulates and sharpens** as time reveals it, rather than flickering. Distinct from the spatial signal, which is instantaneous and two-way.

---

## Refinement must be a live-to-live handoff (or it looks broken)

**The failure mode:** playhead marching, a quad splits, children catch up 0→`t`; if the parent *freezes* during that catch-up, the user sees a dead rectangle in a live field — and the freezes cluster exactly where structure is (you only split there), i.e. exactly where the user is looking. Refinement stutter in the most-watched regions.

**The rule:** refinement never interrupts a region's animation.
1. Quad flagged to split at playhead `t`. **Parent keeps marching and rendering** — no freeze.
2. Children spawn, catch up in the background *while the parent keeps advancing* — so children chase a **moving target** (must catch up to where the playhead *is when they finish*, re-syncing to current `t`, not where it was at spawn).
3. Swap parent→children **atomically at a single frame, both already at the same `t`.** Seamless: the region animated as the parent throughout, continues as the children at higher resolution.

So the slippy-map fallback becomes a **moving fallback** — the parent is a live lower-resolution version that keeps marching until the higher-resolution version is ready to take over *in time*. Handoff gated on **time-sync** (`children.t == playhead`), not merely "children computed." Transient: briefly both parent and children integrate (bounded by how many quads are mid-split) — a small compute bump, not a memory one.

---

## Staleness semantics strengthen (a free win)

Blur previously meant "spatially stale (wrong zoom/position)." Under lockstep it *also* means "frozen in time — not marching with the playhead." These **unify**: sharp = live-and-in-time; blurred = stale-and-frozen. One honest "not current," now with a temporal dimension. Stale quads can't animate → the blur conveying "this isn't live" is *reinforced*, not contradicted. No new signal needed — it rides the blur already specced.

---

## Render swapping is unaffected — the struct is the boundary

**The render pipeline reads a `SimState` struct with known fields. That contract is unchanged, so render swapping is unaffected, full stop.** Colour occupants bind to *fields of the struct* and don't know *how* the field was filled — checkpoint replay (old) or running accumulator at the playhead (new) is invisible to them. The change is entirely on the **production** side (upstream of the struct); the render side only consumes the finished struct.

This is the payload-as-waist firewall doing its designed job: the struct separates "how state is computed" from "how state is coloured." Lockstep changes the former; render swapping lives in the latter; the struct between them doesn't move. Because occupants were *never* allowed to reach around the struct and read raw trajectory/checkpoints directly (the struct is the sole interface), changing what fills the struct **cannot** break them. The **read-side** struct type always has its fixed field set — at every tier, by the uniform read-side interface (lowering Part 3a: a tier that bakes a producer out makes the field read the NaN sentinel; the *stored* struct is tier-sized, 144/96 B). Render always reads the full known type; swapping among render modes is free and causes **no recomputation**, during playback included. (The "colour only by what you accumulate" idea from discussion dissolves twice over: layout is decided once, and Part 3a makes availability a *value* question, never a *type* question.)

---

## Transport controls (free from statelessness)

Play / pause / restart / loop are trivial because there's no stored history to manage: restart = reset playhead to 0 + discard state; loop = auto-restart at `t_end`; pause = stop advancing the playhead (render mode swapping and even panning still work at the frozen `t`). Pausing at `t_end` = the full integration = equal-or-faster than current.

---

## The one accepted cost (stated plainly)

**Revisiting costs recompute.** No retained history → a quad that scrolls off and returns must catch up again (re-march 0→playhead). Mitigated by a small **bounded viewport-scale cache** of recent quads' current-`t` state (panning back to a just-left region is cheap), but genuinely-long-absent quads re-boot. This is the flip side of lazy-in-both; it's bounded and maskable (moving fallback), and it's *compute* the device can do — versus the memory design's hard allocation failure. Right side of the trade for a browser artefact.

---

## The frame loop — the one genuinely new object (and what makes lockstep clean)

The old design was pull-based / on-demand with no global clock. Lockstep needs a **first-class fixed-timestep frame loop** with a **barrier**, and this is the object that turns "lockstep with a pile of sync edge-cases" into one coherent presentation discipline. It is the same shape as networked-game lockstep (don't render tick N until all peers submitted) and the standard fixed-timestep game loop — well-understood territory, not novel. **The loop runs in a Web Worker (OffscreenCanvas), not on the main thread** — the main thread is only an input pump + DOM-GUI host, so the loop (and all the scheduler/physics behind it) can never hitch a gesture or freeze the GUI (caching contract Part 6a). The playhead, the barrier, and all sim state live worker-side.

**The principle: never present mixed time.** Every glitch in the naive design (frozen rectangle in a live field, region snapping forward, children popping in behind the playhead) is the *same sin* — presenting pixels from different moments as if they shared a moment. The barrier makes that **unrepresentable**: the only thing that reaches the screen is a set known to be at one `t`. The glitch class is designed out, not patched. **The one deliberate, bounded exception is checkerboard motion acceleration** (`principia_checkerboard_contract.md`): while the playhead *advances*, half the pixels may lag by exactly one `dt` (a uniform, bounded skew — categorically unlike the arbitrary unbounded mismatches above), measured-imperceptible and self-erasing at every rest/pause/endpoint. It is opt-out and forced off in export; the strict single-`t` invariant holds unchanged at every quiescent moment.

**Two-tier: barrier over the LIVE SET, catch-up OFF-LOOP.** A hard barrier over *everything* would reintroduce the stall (one slow reveal freezes the loop — the classic lockstep "one lagging peer stalls the match"). So:

```
LIVE SET     quads already at the playhead. March together, barrier-synced,
             presented coherently every frame. Cheap — one dt/frame each.

CATCHING-UP  newly-revealed / newly-split quads, marching 0→playhead in the
             BACKGROUND (does NOT gate presentation). Shown via the moving
             fallback (blurred live parent/ancestor, itself animating).
             Reaches playhead → PROMOTED into the live set at the next frame
             barrier, atomically, already synced.

frame loop (fixed timestep):
  target_t = playhead
  advance every LIVE-SET quad until quad.t == target_t     // one dt, cheap
  BARRIER — wait until the LIVE SET is at target_t          // only waits on the cheap set
  promote any catching-up quads that reached target_t       // atomic, synced
  present                                                    // one shared t, coherent
  advance playhead by fixed dt
```

The barrier only ever waits on the live set (already synced, one `dt` closes it). Expensive catch-up runs *beside* the loop and never gates presentation; a quad joins the barrier-synced set only when it can do so without stalling it. **Coherence is the live set's job (kills glitches); responsiveness is the background catch-up's job (kills stalls); the blurred fallback is the honest bridge (kills the lie).**

**Three failure modes, three owners:**
- *glitch* (mixed time on screen) → prevented by the barrier (present only the synced live set)
- *stall* (freeze waiting on a slow quad) → prevented by off-loop background catch-up
- *lie* (showing not-current as current) → prevented by blur (catching-up region shown blurred through its animating live parent)

**This unifies the blur grammar completely.** Every kind of not-currentness is one signal: spatially stale (wrong zoom) → blur; temporally behind (catching up) → blur; being refined (children not ready) → blur (parent shown, animating). One rule — **sharp is real, fuzzy is arriving** — covers space, time, and refinement uniformly. The temporal dimension slotted into the existing blur discipline for free; no new vocabulary.

**Promotion-at-barrier resolves the desync footguns by construction** (see footguns 2, 5, 6 below): a catching-up quad promotes only when `quad.t == live_set.t`, checked *at* a barrier where the playhead is momentarily stationary — so there is no moving target within a frame and the child-chasing-a-moving-playhead race cannot occur. Pausing freezes the playhead (live set holds coherently at that instant; reveals still catch up to it in the background and promote). Refinement thrash is off-loop and never visible; the split/latch decision is CPU scheduler metadata (persists across visits) separate from the live/catching-up *state* (discarded and re-booted).

**Fixed-timestep = determinism (footgun 3).** The playhead advances by a **fixed `dt` per sim-step**, and sim-steps are **decoupled from render-frames** (accumulate wall-clock, take fixed sim-steps, render when ready — the standard fixed-timestep loop). Playback speed is a `dt`-per-second setting, independent of device frame rate, so two runs reproduce and the parity contract's determinism firewall holds. Frame-rate-dependent marching would make "playback speed" = "device speed" and break parity — the loop is where this is prevented.

**Export runs the SAME loop in BLOCKING mode (footgun 7).** Export wants the *hard* barrier — a captured frame must be fully caught up, no fallback, no progressive fill ("exported frames fully converged"). Since export renders to a file, latency doesn't matter and correctness does, so it accepts the stall: hard barrier over *all* visible quads, capture, advance. Interactive playback runs the loop in *progressive* mode (live-set barrier + background catch-up). **Same loop, two barrier policies** — the interactive-vs-export distinction the rewritten export contract must state.

---

## Decisions — DECIDED (ratified in conversation; recorded here so they don't evaporate)

- **The reversal + `SimState` rename** — ratified, and the contract edits below **executed** (render, scheduler Parts 7–8, caching Part 7, export rewrite, ledger, integrator §3.7). ✓
- **Frame-loop cadence** — **adjustable playback speed; default = 1 minute to `t_end`** (so `steps_per_frame = ⌈T / (60 × fps × dt_macro)⌉` at 1×, speed multiplier ~0.1×–10× on top). Invariant locked: fixed `dt` per sim-step, decoupled from wall-clock frame time (determinism). The *number* is derived from the 60s default; the *feel* is tuned live. ✓
- **Latch persistence** — the refinement latch is **per footprint, lives with the resident quad, and goes when the cache evicts or merges the quad** (R-99), so it never pins memory. Bounded by the resident set, not by session length. An evicted or merged region correctly re-discovers. ✓
- **Checkpoints — deleted, confirmed.** No surviving consumer: survey animation (lockstep), replay scrub (removed — the GUI scrubber re-integrates, R-66), hover/inspector trace (CPU `computeIC`), divergence overlay (on-demand single-IC f32 GPU trace), static outcome map (never needed them). The concept is obsolete; the ledger reflects it. ✓
- **Viewport-cache budget** — policy locked (hard cap, current-state-only, cost-weighted LRU incl. `t_cached`); the *number* is set by the **device-characterisation phase** (`principia_quality_device_note.md`; caching Part 7) — a fraction of a detected VRAM budget, never a fixed constant, scaling down on weak devices alongside the compute knobs. One characterisation sets compute *and* memory settings together. ✓

## Still open (settle at implementation, or next edit pass)

1. **Contracts edit pass — DONE (both waves).** The temporal edits landed, and the **sampling/SSAA amendments subsequently landed too**: colouring dd §3.7 (colour-per-sample → SSAA resolve), render Part 4 (averaging rewrite), scheduler Part 9 (ensembles-only-on-nominal + Halton (2,3) offsets), and the ledger/render statements that ensemble spread is **derived at resolve, not a stored field** (the earlier "colourable per-sample field" phrasing here was the pre-reversal framing). ✓
2. **Catch-up scheduling** — background, non-blocking, generation-cancelled, promote-at-barrier; reuses the existing refinement-compute path. Confirm at implementation.
3. **Ensemble E during the live march — RESOLVED by the quality/device controller.** Ensemble copies march every frame (~`2(E+1)` trajectories/pixel; at E=4 ~10× base). The device-characterisation cost model sets `e_motion_gating`: **E reduced/off during an active march, full E at rest and in export.** The boot probe is the measurable-frame-loop-cost input this was waiting for (`principia_quality_device_note.md`; scheduler Part 9). Threshold is a controller tunable, settled on a working system.
4. **Milestone plan** — written in step 7 (`plan/MILESTONES.md`; canonical_spec §11, R-74). Assembly, not design. ✓

---

## Blast radius, sorted by kind of change (is this a nightmare? — no)

**The firewall did its job:** the change is contained to *one side of the struct*. Physics (above the waist) doesn't change; render/colour/GUI (below) only renames. Mostly *subtractive* (delete checkpoint array, scrub, history buffer) + *one additive piece* (continuous refinement + frame loop) + *one rewrite* (export).

- **Category 1 — pure rename, zero semantics** (`SimResult`→`SimState`, 11 docs): find-and-replace, one pass.
- **Category 2 — physics does NOT change**: integrator wrapper still marches the occupant (`ADVANCE`, R-19) identically; only `maybe_write_checkpoint` → expose-current-state-and-discard. Shape-map math, winding, FTLE/spread/diffusion accumulators all already per-step. Decoder/encode/colouring-math untouched.
- **Category 3 — genuinely changing (3 docs)**: render (delete-heavy, simplifies), scheduler (additive — the new design + frame loop), export (the one rewrite).
- **Category 4 — simplified, small**: caching, ledger, lowering/systems-arch/gui incidental.

**Verdict: fairly clean, mostly subtractive.** Four docs with real edits, one rewrite, rest rename-or-trivial. A weekend of careful doc surgery, not a month. The change *deletes a concept and simplifies the data model* — the good kind of re-architecture.

## Footguns (all re-applications of disciplines already established)

Cluster in two themes: **(A) the memory problem sneaking back** (guard with hard invariants), **(B) determinism/timing under a now-live march** (guard with the fixed-`dt` firewall the parity contract already demands). Most B footguns are *dissolved by the frame loop above*.

1. **Diffusion regression-moments trap** (A, sneakiest) — "collect (t,y) points and fit later" silently reintroduces per-pixel history → OOM at scale (V2 failure in a corner). MUST be running co-moments (the **Welford centered** form — `mean_y, C_ty` + closed-form time moments, payload §4 — *not* the naive raw sums `Σt,Σt²,…`, which cancel catastrophically). Named invariant in ledger + integrator dd.
2. **Children chasing a moving playhead** (B) — *dissolved by promotion-at-barrier*: promote only when `quad.t == live_set.t` at a barrier where the playhead is stationary. Time-sync gate, not completion gate.
3. **Frame-rate-dependent marching breaks determinism** (B) — *dissolved by fixed-timestep loop*: fixed `dt`/sim-step, decoupled from render frames. Correctness issue, not just UX.
4. **Viewport cache regrowing into a history buffer** (A) — hard budget cap; current-state-only; the "cache a bit of history for smoothness" temptation is the V2 siren. Guard with a ceiling.
5. **Refinement thrash / latch** (A+B) — *off-loop via frame loop, so never visible*; the latch lives with the resident quad and goes with it (R-99), state re-boots.
6. **Pan-while-paused** (B) — *clarified by frame loop*: pause freezes the playhead, not the compute; reveals still catch up to frozen `t`. Honest, not a bug — state it.
7. **Export determinism** (B) — *handled by blocking barrier mode*: export runs the frame loop with a hard barrier (fully caught up before capture), interactive runs progressive. Same loop, two policies.

---

*Eager-in-time was the cleverness that made it slow — precompute-and-store the whole future to enable a scrub nobody wanted, at a VRAM cost that would lock out half the audience. Lazy-in-time (lockstep-live-march) is more accurate (true `n(t)`, no M-knot crudeness), lighter (viewport working set, runs on a phone), and simpler (stability metrics revert to the accumulators they always were). The struct is the waist; render never notices. Refinement gets richer (temporal accumulators, latching) and hands off live-to-live. The frame loop is the one new object, and it's the piece that makes it clean: present only the barrier-synced live set, catch up and refine off-loop, promote in atomically at a barrier, blur the difference — so the screen is never mixed-time (no glitch), never frozen (no stall), never lying (blur is honest). One principle wearing many hats: present-what's-synced, compute-the-rest-off-loop, mark-the-difference. The one cost — recompute on revisit — is bounded, maskable, and the graceful side of every failure. Not a nightmare: mostly subtractive, one rewrite, footguns that are all re-applications of disciplines already in the contracts.*
