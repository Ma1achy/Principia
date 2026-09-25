# Principia — quality presets & device characterisation

*Working note for a genuinely new subsystem — the one that came out of the temporal/sampling rework. Frames the boot probe correctly: it is **not** a boot-only event, it is the implementation of the **"auto" quality preset**, which happens to run at boot because auto is the default. This unifies boot-profiling, the quality presets, and the GUI knobs into one coherent object, and it resolves two previously-open questions (cache budget; ensemble E-during-live-march gating).*

---

## The reframe: quality is a preset selector populating one settings struct

Quality is a small enumerated selector. Its three modes are three ways of **filling the same underlying settings object** — the same pattern as the colour presets (pure data, one struct, three editors):

```
QualitySettings = {
  N,                     // SAMPLES_PER_QUAD_AXIS (samples per quad axis) — sim key
  max_rel_depth,         // MAX_REL_DEPTH sliding window — scheduler knob
  render_scale,          // internal render resolution ÷ display (0.25–2.0) — render/refinement-side, NOT sim key (memory-tiers §2)
  lock_to_native,        // pins render_scale = 1.0, greys the slider (default on)
  E,                     // ensemble/SSAA copies per nominal sample — live, NOT sim key (copies cached per copy_index, R-89)
  ftle,                  // Benettin shadow on/off — sim key (tier-gated: Medium up)
  word,                  // free-group word buffer on/off — sim key
  e_motion_gating,       // full E at rest vs reduced during live march
  checkerboard_mode,     // user-owned three-state (permanent/motion-only/off) — arbiter READS (cost model) but never writes (checkerboard contract §3)
  cache_cap,             // current-state cache ceiling
  ...
}
```

- **Auto** — runs **device characterisation** (below), solves the cost model, sets every knob. Re-runnable: re-selecting auto (or a "re-detect" button) **re-measures**. Auto is a *measurement*, not a fixed tier — that's the whole point, and why re-running it is meaningful (device state changed, plugged in an eGPU, first probe ran hot).
- **Named tiers** (Potato / Low / Medium / High / Ultra / Extreme — the canonical six, `principia_memory_tiers.md`) — fixed hardcoded knob values; skip the probe. For users who want predictability over measurement ("I know my machine, give me High"). Still clamped to hard limits.
- **Custom** — exposes the knobs directly (N, MAX_REL_DEPTH, render_scale 0.25–2.0 with the lock-to-native toggle, E, FTLE, motion-gating). User drives each (arbiter off — §10).

Underneath it is **one struct**; the preset selector is only *how it gets populated*: auto *computes* the values, a named tier *looks them up*, custom *lets the user set them*. Drops into the existing GUI/state philosophy with no new mechanism.

---

## "Auto" = device characterisation (three legs: limits · probe · info)

Each leg covers the others' blind spot. **Limits gate (anti-crash), probe measures (anti-stutter), info sanity-checks (anti-lying-probe), headroom protects (thermal).**

```
1. LIMITS — hard ceilings, AUTHORITATIVE
   read adapter.limits (maxStorageBufferBindingSize, maxBufferSize,
   maxComputeWorkgroupsPerDimension…), navigator.deviceMemory, adapter type.
   → HARD CAPS on cache size, live-set size, quad resolution, render-target size, buffers.
   Nothing below may exceed these. This is the ALLOCATION-FEASIBILITY axis the
   probe structurally cannot see: the probe measures speed, limits decide what
   can exist at all (the hard-failure / crash axis vs the probe's soft-failure
   / stutter axis).

2. PROBE — throughput, the real per-device measurement
   warm up (throwaway dispatches — discard: first dispatches measure shader
   COMPILATION + cold GPU, not steady state) → then measure
   cost-per-dt-per-sample on a SPREAD of quads (cheap interior + expensive
   Burrau-adjacent), at REAL trajectory count (~2(E+1)/pixel, or base × known
   factor), over ~a few hundred dt (so per-step cost dominates dispatch
   overhead), using WebGPU timestamp queries where available else batched
   wall-clock around onSubmittedWorkDone. → robust HIGH-PERCENTILE number
   (size for the worst the user will hit, not the mean).

3. INFO — soft prior / cross-check (NOT a lookup key)
   adapter.info vendor/architecture → expected device class. If the probe is
   wildly inconsistent with the class (fast probe + mobile iGPU, or slow probe
   + known-discrete) → distrust the probe, fall back to conservative defaults.
   NOT a device database: the name strings are unreliable/masked for privacy
   ("Apple GPU", blank architecture), a lookup table is a maintenance treadmill,
   and the same silicon runs the kernel differently across drivers/browsers.
   The name is a HINT that vetoes a bad measurement — never the number itself.

4. SOLVE
   cost model: samples affordable per frame = frame_budget /
   (cost_per_dt_per_sample × steps_per_frame). Bounds N (samples per quad axis), E,
   live-set size, and MAX_REL_DEPTH (deeper zoom = larger frontier) TOGETHER
   from ONE base measurement — a calculation, not a per-knob search.
   → CLAMP every solved knob to the step-1 hard limits.
   → apply thermal headroom (~60–70% of raw cold measurement — the device only
     gets slower as it heats; better under-promise with headroom than max out
     cold and stutter when warm).
   → sane default. User can override (custom).
```

### The five ways the probe can lie (guard each)
1. **Cost is wildly non-uniform** across the manifold (smooth interior vs Burrau close-encounter = 1–2 orders of magnitude). A random quad is not representative → profile a **spread** (cheap + expensive), calibrate to a **high percentile**, size for the worst.
2. **Under-counting co-computations** — probe must run the **real trajectory count** (`2(E+1)`, shadows + ensemble), or probe base × known factor.
3. **Cold-start / compilation skew** — first dispatches measure compilation + cold GPU. **Warm up, discard, then measure**, over enough dt that fixed overhead is amortised.
4. **GPU timing is hard in WebGPU** — dispatch is async; wall-clock includes queue latency. Use **timestamp queries** if available; else batched wall-clock over a big batch. Heuristic must tolerate coarse timing (Safari spotty).
5. **Thermal / throttle / shared-GPU** — boot is a best-case snapshot; device only slows. **Size conservatively** (the headroom in step 4).

---

## The auto → custom "show your work" flow

- When a **named tier or auto** is selected, the **custom fields still display the resulting values** (read-only / greyed) — the user can *see* what auto chose ("16² samples, depth budget 6, E=3 [4× SSAA], FTLE on" — a High-tier readout).
- The moment the user **touches any field**, it flips to **custom with those values as the starting point** — custom is "auto's answer, now editable," not a blank slate.
- So the probe's output is **visible and inspectable**, never a black box, and the user tunes *from* a sensible baseline rather than guessing absolute numbers. This is the difference between a usable custom mode and an unusable one.

---

## Hard limits clamp EVERY preset (including custom)

The device's `adapter.limits` are authoritative for **all** modes — custom *requests* values, but allocation feasibility cannot be overridden:

- Custom fields are **clamped (or validated with a warning) against the hard caps**: a slider maxes out at the device ceiling, with a tooltip explaining why. (Lean clamp-with-explanation over silent-fail or allow-and-crash.)
- **Named tiers get the same treatment** — a "High" preset on a weak device clamps down to what fits rather than trying to allocate and crashing. This **preserves graceful-degradation-not-hard-failure** (the reach argument): the crash axis is closed by the clamp, uniformly.
- The hard limits are a **floor under the whole quality system**, not just under auto, because they are about *what can exist on this device*, which no preset can change.

---

## Consequence: quality changes are (mostly) sim-key changes

- **Sample count (`N`) is a sim-key parameter** (with `ftle` and `word`) — changing it changes *what is computed*. So re-running auto, switching presets, or editing `N`/FTLE/word in custom is a **sim-key change → invalidates cached quads → the march re-boots from `t=0`**, masked by the normal blur-stale-while-recomputing backdrop (caching contract). Correct and unavoidable (different resolution = genuinely different samples); not free, but honestly masked.
- **Separate the axes (SETTLED):** knobs that change *what's computed* (`N`, `ftle`, `word`) are **sim-key** and invalidate. **Depth and `E` are not on the sim key (R-89):** depth (`max_rel_depth`) is a scheduler knob, and `E` changes live — copies are cached per `copy_index` and the nominal's key excludes `E`, so copies drop and respawn without invalidating the nominal. **`render_scale` is NOT sim-key and does NOT invalidate** — by payload purity (`SimState = f(IC, sim key)`), render resolution is not in the sim key. It moves the *refinement target* (the screen-space floor drives quads to render-pixel size, so lowering render_scale means existing deep quads are simply deeper than needed — still valid; raising it computes new deeper quads, masked by blur). This is exactly why it can be the medium-timescale continuous slider (§5): it slides freely without ever re-booting the march. `N` (SAMPLES_PER_QUAD_AXIS) by contrast genuinely changes the sample lattice — sim-key, invalidates, re-boots.

---

## What this subsystem resolves (two previously-open questions)

- **Cache budget** (temporal note, was "deferred — fraction of detected VRAM, never fixed"): **set here.** Step 1 (limits) gives the hard ceiling; the solve sizes the cache cap within it. One characterisation phase sets compute *and* memory knobs together — not two phases.
- **Ensemble E during the live march** (temporal note, still-open item 3): **decided here.** The cost model answers "can I afford `2(E+1)`/pixel at interactive resolution within the frame budget?" — if not, the heuristic sets `e_motion_gating` to reduce E (→0/1) during an active march and reserve full E for at-rest/export. The probe is the missing input that question was waiting for.

---

## Auto is a continuous controller — the adaptive control architecture

*Extends "auto" from a one-shot boot measurement to a live controller tracking the device over the session. This section is the design; it supersedes an intermediate "two controllers with anti-conflict rules" framing — two controllers fighting over shared knobs is the tempting wrong shape, and the correction is structural: one writer, many sensors.*

### 1. One arbiter, many sensors
The steady-state measurement and the transition measurement are **sensors, not controllers**. They feed one shared model; **a single policy owns every knob write**. Fighting is impossible because there is no second writer — the conflict problem is dissolved structurally, not managed with min-rules.

### 2. Model-based, not reactive
Maintain one slowly-varying scalar — **device throughput (cost-per-dt-per-sample)** — initialised by the boot probe, continuously refined by both sensors. Knobs are a **pure function of the model**: given throughput + frame budget, *solve* for settings (the boot solve, reused live). Measurements update the model; the model determines the knobs. Converges instead of hunting: you estimate a physical quantity that changes slowly (thermal drift, minutes), and when it moves you jump *directly* to the predicted-correct setting rather than stepping toward it through a noisy feedback chain.

### 3. The quality ladder
Collapse the knob space: design offline an ordered ladder of **~8–12 rungs**, each a complete preset (N, E, depth, render_scale, FTLE/word gating, live-set size) with monotonic cost and roughly just-noticeable perceptual steps — degrading what eyes notice least first (E/edge quality, render_scale) before structural sample density. **The policy's whole output is one integer: the rung.** Kills multi-knob interaction dead, makes hysteresis trivial, persists and debugs as one number. The perceptual craft lives in the ladder's *design*, done once offline — not in live logic.

### 4. Resting rung + motion offset (one ladder, two setpoints)
The settled sensor calibrates the **resting rung**; the transition sensor calibrates a **motion offset** — rungs dropped during gestures/catch-up, plus fallback-staleness tolerance. `motion_rung = resting_rung − offset`, so motion ≤ resting is **structural**, not enforced. The offset is **playhead-depth-aware**: transition cost scales with `t` (catch-up at t=190 ≫ t=5), so the sensor learns "painful past t≈120 on this device" and widens the offset as the playhead deepens — the lockstep-specific smartness, expressed as one learned curve on one ladder. (This is the *tuning input* for the two-regime/PREVIEW_MODE policy the scheduler already has — not a new mechanism bolted on.)

### 5. Timescale cascade (each layer absorbs what's too fast for the one above)
- **Fast — per frame — the frame loop itself:** transient hitches absorbed by existing slack (dropped render frame, sim-steps flex, blur). **No settings change, ever.**
- **Medium — during motion — the motion regime:** render_scale slides *continuously and instantly* under gestures (unless locked to native, in which case E and the refinement floor slide instead — memory-tiers §6) (game dynamic-resolution; invisible while the view moves). Snaps back at rest.
- **Slow — minutes, settled only — rung changes:** the policy re-plans the resting rung only during sustained quiescence (no navigation, no chart change, no catch-up, no in-flight quality change), from the model.

Classic cascade control: inner loops handle disturbances faster than outer loops re-plan. Most "oscillation" in naive designs is a slow loop trying to fix what a fast layer should have absorbed. Settled periods are simultaneously the only *valid* place to measure steady-state (no catch-up confound — navigation *is* interaction, so gating on quiescence excludes bursts by definition) and the least *obtrusive* place to adjust. **The frame loop absorbs transients; the controller tracks sustained drift.** Two mechanisms, two timescales, no overlap.

### 6. Change under cover (the perceptual magic)
Only change quality at moments the user cannot perceive it — the architecture is full of free cover:
- **Motion** — drop render_scale/E while panning; eyes tracking motion can't resolve it; restore at rest. (Checkerboard, when the user has it enabled, is *also* firing here — the cost model counts its ~2× per-frame saving during advance; the arbiter never toggles it.)
- **Blur** — the fallback already blurs transitions; a rung change beneath blur is invisible *by the existing staleness grammar* (blur means "arriving"; arriving at a different rung is still just arriving).
- **Natural invalidations — the key one for expensive knobs:** sim-key rungs (`N`, FTLE, word) trigger a re-boot, so a reactive controller must never touch them. But chart changes, restarts, loop-wraps, and explicit re-detects **already invalidate everything** — piggyback sim-key rung changes onto those moments and they're literally free. **Rule: render-key knobs adjust under motion/blur cover; sim-key knobs adjust only at natural invalidation moments. The controller never *causes* a visible recompute; it *rides* the ones the user caused.**

### 7. Hysteresis + failed-rung memory
**Drop fast** (one bad settled window → down a rung, applied at the next covered moment; render_scale drops instantly under motion). **Rise slow** (sustained headroom for minutes → up one rung, at a covered moment). **Dead zone** between the thresholds (e.g. drop above ~20 ms-equivalent, rise only below ~12 sustained; do nothing between). And — borrowed from TCP congestion control, which solved exactly this hunting problem — **remember the rung that failed**: if rung 7 caused sustained overrun, require stronger, longer evidence before re-attempting it, with the caution decaying over tens of minutes (thermal conditions change). This one memory kills the classic slow-oscillation of climbing to the rung that hurts, getting hurt, descending, repeat.

### 8. Sense what the user feels, not what the GPU reports
- **Steady-state signal: sim-time debt** — is the march holding the requested dt-per-wall-second? An *integrating* signal (debt accumulates smoothly; immune to single-frame noise) — the analogue of buffer occupancy in adaptive-bitrate streaming, and the reason streaming switched to it. Secondary: **95th-percentile frame time** over the settled window (stutter hurts, not the mean).
- **Transition signal: time-to-resync** (how long revealed quads took to promote) and **fallback-visible duration** — the felt quantities of a transition.

### 9. Persist the model, not just the rung
Save `{throughput estimate, resting rung, motion-offset curve, failed-rung memory, provenance signature}` (localStorage/IndexedDB). On reload with **matching provenance** (adapter.info + key limits + probe/kernel version — the identity/validity discipline applied to the device profile), start at the right rung *instantly*: no probe, no settling-in. The magic cold start — the second session just opens correct. Escape hatches so persistence can never trap: provenance mismatch → re-probe; explicit **re-detect** → force fresh probe, overwrite; and the live model keeps correcting a stale/pessimistic estimate anyway (a bad boot measurement gets raised back up within a session, and the correction persists). **Two distinct things are remembered:** the user's *intent* (which preset; custom values) — always honoured verbatim — and, for auto, the *measurement* (a cache, valid only under its signature).

### 10. Two sanctities: the user, and observability
- **Named tier or custom → the arbiter is OFF, entirely.** Auto means "keep it comfortable for me"; the others mean "give me exactly this." Silently overriding explicit intent is how magic becomes haunted.
- **A debug overlay for the arbiter** (debug-tooling-first ethos): estimated throughput, current rung, headroom, recent decisions *and why*. Magic you can inspect is trust; magic you can't is suspicion.

*One breath: sensors estimate one slowly-varying capability scalar; one policy solves it into a rung on one designed ladder, with a playhead-depth-aware motion offset as the second setpoint; the frame loop absorbs transients, the motion regime slides under cover of motion, rung changes land only at settled moments — sim-key rungs only when something already invalidated; drops fast, rises slow through a dead zone, failed rungs remembered; signals are sim-time debt and resync time; the model persists under a provenance signature so the second boot is instant; user override switches the arbiter off; and a debug view shows its reasoning. Fighting is structurally impossible (one writer), oscillation structurally damped (model + cascade + hysteresis + memory), and the seamlessness is real magic's actual mechanism: adaptation the user never catches happening.*

---

## Open sub-questions (settle at implementation)

- The ladder itself: rung count (~8–12) and the exact preset at each rung — the offline perceptual-design task.
- Exact named-tier values (named tiers are *pinned rungs* of the same ladder — Potato/Low/Medium/High/Ultra/Extreme as named presets over the ~8–12 finer internal rungs (memory-tiers doc)).
- Frame-budget target(s) — 16ms interactive; export ignores it (blocking).
- Probe quad-set: *which* cheap and expensive regions (golden IC neighbourhood + a Burrau-adjacent patch is the natural pair) and how many.
- Timestamp-query availability fallback quality — how coarse is acceptable.
- Controller thresholds: the dead-zone band, the settled-window length, the rise-evidence duration, the failed-rung memory decay — all empirical, all one-number tunables on a working system.
- The motion-offset curve parameterisation (offset as a function of playhead depth) — learn a simple monotone curve, don't over-model.

---

*Quality is a preset selector filling one settings struct. "Auto" is device characterisation — limits gate (anti-crash), a warmed-up spread-of-quads probe measures throughput (anti-stutter), adapter.info sanity-checks (anti-lying-probe, a hint not a lookup key), thermal headroom protects — solving all coupled knobs from one measurement and clamping to hard limits. Named tiers look the knobs up; custom lays them bare, pre-filled with the current preset so you tune from a baseline and can see what auto chose. Limits clamp every preset (feasibility is authoritative — graceful degradation, never crash). Sample-count (`N`) changes are sim-key → invalidate → masked by the staleness backdrop; depth and `E` move without invalidating (R-89). It closes the cache-budget and E-motion-gating questions for free, because characterising the device is exactly where both are answered.*
