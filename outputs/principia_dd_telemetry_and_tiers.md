# Telemetry, profiling, and deriving the quality tiers from real hardware

**The point: the tier constants are currently picked. With measurements from real devices they can
be DERIVED.** That is the same standard §4.2 of the philosophy applies everywhere else — *a quantity
is admissible if it is bounded by its own achievable maximum, fixed by a conservation law, or
expressed in canonical units* — and the tiers currently fail it.

---

## 1. The two modes, and why both are needed

**They answer different questions and neither substitutes for the other.**

### 1.1 The fixed suite — comparability across devices

A scripted sequence run identically on every device: a fixed set of slices, a fixed zoom ladder, a
fixed pan path, a fixed playhead march, each at several quality settings. **No user input, no
variation.** This is the only thing that lets two devices be compared, because it is the only thing
where the *work* is held constant.

**It must sweep, not sample.** A single setting tells you pass/fail on that device; a sweep tells
you *where the device's boundary is*, which is what a tier needs.

### 1.2 Passive telemetry — what interaction actually looks like

Log while someone uses it normally. **The benchmark tells you what the device can do; this tells you
what people ask of it**, and those are usually nothing alike. If the suite spends its time on smooth
zooms and real users hammer the playhead, the tier tuned on the suite is tuned for the wrong thing.

---

## 2. What to record — and the rule that makes it useful

> **A frame time is meaningless without the work it did.** Every frame record carries what was asked
> of it, or the whole log is uninterpretable.

Per frame:

```
frame_ms           wall clock, the number that matters
quads_computed     new quads this frame
quads_reused       cache hits
samples            quads x N^2 x (E+1) — the actual integration work
substeps_total     the honest cost measure; steps are not comparable across steppers
playhead_dt        how far time moved
camera_delta       pan/zoom magnitude — 0 for a static frame
tree_depth_max     and leaf count
stage_ms           breakdown: integrate / reduce / colour / upload / present
```

**`stage_ms` is the load-bearing one.** Without it you know a frame was slow and not *which resource
ran out* — and different devices will bottleneck differently. Integrated graphics is bandwidth-bound;
a 5090 at small workloads may be latency- or dispatch-bound and never approach its throughput at all.
A tier that assumes everything is compute-bound will be wrong on both.

Per session, once:

```
device            GPU/CPU model, core counts, VRAM or unified memory size
backend           Metal / Vulkan / DX12 / WebGPU, and driver version
precision         f32 / f64 support and reported f64 rate
build             commit hash, release profile, feature flags
display           resolution, refresh rate, DPI scale
```

**Unified memory needs its own field.** On Apple silicon there is no VRAM/RAM split, so a tier
derived from discrete-GPU memory limits will be nonsense there. 18 GB shared on the M3 and 64 GB on
the M5 are *different tiers on the same architecture*, which is exactly the case a
VRAM-threshold rule gets wrong.

---

## 3. Percentiles, not means — and the specific thresholds

**A mean frame time hides stutter, and stutter is what makes something feel broken.** Report p50,
p95, p99, max.

| target | frame budget | what it means |
|---|---|---|
| 60 fps | **16.7 ms** | the goal |
| 30 fps | 33.3 ms | comfortable |
| **24 fps** | **41.7 ms** | **the HARD FLOOR, not a goal — below this it is not interactive** |

**60 fps is the target. 24 is the floor.** A tier that lands at 24 has failed to find a good setting,
not succeeded at finding an acceptable one. The distinction matters for the inversion in §4: solve
for **16.7 ms first**, and only fall back toward 41.7 ms when nothing fits — recording that it had
to, because a device that can only reach the floor is telling you something about either the device
or the settings ladder.

**The headline number is `frac_frames_over_41.7ms`, measured DURING MOTION.** Static frames can take
longer without anyone minding; a dropped frame mid-pan is immediately visible. So the telemetry must
separate moving frames from still ones — `camera_delta > 0` is the discriminator and it is free.

---

## 3.5 THE TIER IS THREE COUPLED AXES, NOT ONE — v0.5

**`eps` replaced resolution as the quality knob** (`principia_dd_refinement_policy.md`). That is a
better knob — a tolerance is *a guarantee about the answer* rather than *a resource allocation*, and
the same number means the same thing on every slice.

**But a tolerance is a quality promise and a tier is a cost bound, and they conflict.** Reaching a
given `eps` costs **5–37× less than uniform on Burrau regions and about the same on arbitrary latent
slices** — the cost is a property of the field, not of the setting. **So `eps` cannot be fixed and the
cost bounded at the same time.**

### The three axes

| axis | what it is | what happens when it binds |
|---|---|---|
| **`eps`** | the **convergence target** — where the tree stops refining | the image is *done*; nothing further to compute |
| **frame budget** | the **per-frame cap** — how much work per frame | the image is *still arriving*; it keeps improving next frame |
| **hard cap** | quads / bytes — **memory safety** | the image **stops improving short of `eps`**, and must say so |

**They are independent because refinement is progressive.** A frame does not have to converge; the
application has to stay responsive. `eps` says what *done* means, the budget says what you get *this
frame*, and the two do not have to agree. **The old tiers conflated them** — resolution and max-depth
set both the final quality *and* the time to reach it, one axis where there should be three.

**The hard cap is not optional and is new.** Under the old model, resolution fixed the quad count and
memory was predictable. Under `eps` the tree size is **slice-dependent and unbounded**, so a
pathological slice OOMs without one.

### ALWAYS REPORT WHICH AXIS BOUND

```
converged      reached eps            -> nothing more to do
budget-bound   still refining         -> normal during motion; suspicious if it persists at rest
cap-bound      stopped short of eps   -> the image is NOT what was asked for, and must say so
```

**This is the whole point of the v0.5.** A tier that silently misses its target is the failure mode
this project exists to eliminate — and *which* constraint bound is exactly what telemetry needs to
improve the numbers.

### v0.5 means the NUMBERS are guesses; the SHAPE is not

Ship with plausible values per tier and **calibrate from real devices** (§1, §4). The numbers are
expected to be wrong. What must be right now is that there are **three axes**, that the **binding one
is reported**, and that the **hard cap exists** — those are hard to retrofit; the constants are a
config change.

**And `sea_fraction` (`principia_dd_refinement_policy.md` §5.1) is computable before descending**, so
the system can *predict* what a given `eps` will cost on this slice and pick sensibly. The old model
had no way to do that. **A cheap estimator is the named next step.**

---

## 4. Deriving the tier instead of choosing it

With measured throughput per device, the tier becomes an **inversion** rather than a guess:

```
given   measured samples/second and bytes/second on this device
and     a 16.7 ms budget (fall back toward 41.7 only if nothing fits, and RECORD that it had to)
and     sea_fraction(eps) for the slice in hand
solve   for the three axes: eps, per-frame budget, hard cap
```

**Solve for `eps` first, then the budget, then the cap** — quality target, then responsiveness, then
safety. And **report which one bound**, per §3.5.

**That makes the tier a function of measured capability**, and it means a device nobody tested still
gets a sensible answer by running the suite once on first launch. The alternative — a table of device
names — is unmaintainable and wrong for anything unlisted.

**Report which constraint binds**, per device: samples, bandwidth, memory, or dispatch overhead.
A tier that lowers `N` on a bandwidth-bound device is optimising the wrong axis.

---

## 5. The artefact: one file, plain text, readable by the sender

**It is going to friends over Discord. It must be a single file they can open and read.**

- **One self-describing file.** Header block with the session info, then the frame records. No
  binary, no separate manifest, nothing that needs a tool to inspect.
- **Readable.** Anyone sending it should be able to see exactly what they are sending. That is both
  a courtesy and the reason they will actually send it.
- **Self-contained.** It must carry the build hash and the full config, or it cannot be interpreted
  six weeks later — the same provenance rule that `refine_flagged` propagation made non-negotiable.
- **Bounded size.** A long session at 60 fps is 200k+ frame records. Either downsample on write
  (keep every frame during motion, every Nth while idle) or roll up idle stretches into summaries.

---

## 5.5 Profiling is FIRST-CLASS, not a debug mode

**Instrumentation that can be compiled out will be**, and the numbers it produces will not match the
build people actually run. `stage_ms` and the frame record are part of the render loop's contract,
always present, with the *reporting* toggleable rather than the *measurement*.

This is the same lesson as `ab_floored` and `ab_min`: they were computed on every march and read by
nothing, and **a sticky bit nothing reads is indistinguishable from one that never fires.** The
inverse failure is equally bad — a counter that only exists in a debug build measures the debug
build.

**Consequences:**

- The overhead must be small enough to leave on. A timestamp per stage and a counter increment per
  frame is nanoseconds against a 16.7 ms budget; it does not need to be conditional.
- **`prin` and the interactive path share the frame record.** A batch render emits the same
  structure with `camera_delta = 0` and no present stage. One format, one parser, one set of
  percentile code.
- The provenance line from §5 is part of it, not a separate concern.

---

## 6. Failing gracefully — and what "gracefully" means here

**The instrument's first commitment is to say where it stops knowing (§4.1 of the philosophy). A
crash says nothing; a wrong picture says something false. Neither is acceptable, and the second is
worse.**

### 6.1 No suitable GPU, or no GPU at all

**Fall back to the CPU path and say so, loudly and permanently in the UI.** The kernel is already
generic over `Real` and the CPU path exists — it is the reference. It will be slow, and that is a
legitimate state to be in as long as it is *visible*: a CPU-rendering session at 2 fps that presents
itself as normal is a bug report waiting to happen.

**Record the reason in the telemetry.** "No adapter", "adapter lacks required features", "backend
init failed" are different problems and the log must distinguish them, or the device spread in §1
teaches nothing.

### 6.2 Memory pressure — CONTINUOUS, not an error condition

**The budget measured at startup is a snapshot, not a contract.** Another application takes memory,
the OS reclaims, a browser tab is throttled. The number available at launch is not the number
available at frame 10,000, and any design treating it as one **will fail on a real device**.

**On Apple silicon this is the normal case, not the edge case.** Unified memory means the GPU
allocation competes with everything else running — there is no separate VRAM to be safely inside.
Expect pressure; do not treat it as exceptional.

#### Three states, not two

```
comfortable   below the cap, refining freely
pressured     approaching it -- STOP GROWING THE TREE, keep serving what is cached,
              report cap-bound (§3.5). The image STOPS IMPROVING; it does not degrade.
reclaiming    over it -- evict, deepest quads first
```

**The middle state is the one that matters and the one most likely to be missed.** A design with only
*fine* and *OOM* oscillates between them.

#### Eviction order, and the trap in it

Drop the **deepest** cached quads first — cheapest to lose, easiest to recompute.

> **Never drop the coarse ancestors.** They are what the fill draws during motion, so losing them
> turns a memory problem into **a blank screen**. That is the difference between degrading and
> breaking.

#### And test it deliberately

Run with an **artificially low cap** and assert the system stays responsive, keeps drawing, reports
cap-bound, and never blanks. **An OOM path that has never executed is an OOM path that does not
work** — this project has already found a sticky bit nothing read and a diagnostic that folded from
`0.0`.

### 6.2a Budgeting before allocating

**This is the one that must not be handled by crashing, because it is predictable.** The payload
size is known in advance: `quads x N^2 x (E+1) x sizeof(SimState)`. So:

- **Budget before allocating.** Query the adapter's reported limits, compute the requirement, and if
  it does not fit, **reduce the tier and report it** rather than attempting the allocation.
- **On an allocation failure mid-session** — eviction pressure, another application taking memory —
  drop the deepest cached quads first, since they are the cheapest to lose and the easiest to
  recompute. Never drop the coarse ancestors: those are what the fill uses during motion, and
  losing them turns a memory problem into a blank screen.
- **Unified memory has no separate budget.** On Apple silicon the GPU allocation competes with
  everything else on the machine, so the "reported limit" is not a promise. Treat OOM as
  *expected* there, not exceptional.

### 6.3 Device loss, timeouts, driver resets

A long compute dispatch can trip a watchdog. **Bound the dispatch, do not bound the work** — split
into chunks small enough to return, and carry the state between them. That is the same structure the
tiled prebake (§7.2, philosophy) already needs, so it is not a special case.

On an actual device loss: **the payload is a pure function of `(IC, sim key, playhead t)`**, so
everything is reconstructible. Reinitialise, rebuild from the cache metadata, and say what happened.

### 6.4 Compute pressure — being a good citizen, not just a fast one

Everything above asks **how much can we get away with**. The other question is **how much should we
take**, and it is not the same question.

**A renderer that saturates every core and the GPU makes the whole DEVICE unusable, not just itself.**
Scrolling stutters elsewhere, the compositor drops frames, a laptop fan spins up, a phone gets hot in
the hand. The user does not experience *"Principia is fast"* — they experience *"my computer got
worse"*.

**And it is self-defeating.** On a phone or laptop the OS will thermally throttle, so hogging loses
over any session longer than a burst.

#### CPU

**Leave cores.** Do not size the rayon pool to all available cores — leave at least one, more on
small-core devices, configurable. The compositor, the browser and the OS all need to run. **Measure
the marginal return**: if core `N+1` buys 3%, it is not worth the stutter it causes elsewhere.

**Drop priority on background work.** The prebake, the export job and the catch-up march are all lower
priority than the frame loop *and* than the rest of the system. A long batch render should be
pre-emptible without a fight.

**Yield.** Long-running work returns to the scheduler regularly rather than holding a core for a
second. Same requirement as the watchdog bound in §6.3 and should share a mechanism.

#### GPU

**Bound the dispatch, not just the total work.** The tiled prebake already needs this, so it is not a
special case: split into chunks that *return*, and carry state between them. A long dispatch blocks
the compositor and on some platforms trips a driver reset.

**Do not queue ahead.** Submitting several frames deep keeps the GPU saturated and makes the
application **unresponsive to input**, because a gesture cannot jump a queue already committed. Keep
the in-flight depth shallow — it costs throughput and buys responsiveness, which is the correct trade
here. *This is the one most likely to be got wrong: it improves every throughput benchmark and
directly damages the thing that matters, which is that a tilt feels immediate.*

**Respect backgrounding.** Window loses focus or tab hidden → **stop**, not throttle. A background tab
burning a GPU is the most user-hostile thing this application could do, and the telemetry should
record that it happened at all.

#### Thermal is the honest budget

Peak throughput is a burst measurement; **what matters is what the device sustains.** A phone holding
60 fps for ten seconds and 30 for ten minutes should be tiered on the thirty — and §7's campaign has
to run long enough to see it.

If a throttling signal is exposed, log it. **If not, infer it**: a monotone decline in achieved fps at
constant settings and constant scene *is* thermal, and is measurable without platform APIs.

#### A deliberate ceiling, user-visible

Offer a **target utilisation**, not only a target framerate. *"Use up to N cores"* and *"cap at 30
fps"* are legitimate things to want — on battery, in a lecture, while compiling something else. **The
application must not assume it is the only thing running.**

**Default to leaving headroom.** Full utilisation is opt-in, and the telemetry records which was in
force so the calibration is not polluted by mixing the two.

---

### 6.5 The rule underneath all of these

> **A failure is a measurement outcome, not an absence.** The no-discard rule (§4.3) already says
> this about trajectories that will not integrate; it applies identically to devices that will not
> render. Log it, show it, carry it in the telemetry — do not let a degraded session look like a
> healthy one.

---

## 7. The calibration campaign — collect everything, decide nothing yet

**The tiers ship with defaults keyed on crude specs** — VRAM/unified memory, core counts, backend.
Good enough to put a device in roughly the right bracket. **The boundaries between tiers are then
found from data, not chosen.**

### Push every device past where it is comfortable

**Do not only run each device at its assigned tier.** Run it across the whole ladder, including
settings it will obviously fail, because **the boundary is what is being measured** and you cannot see
a boundary from one side of it.

> *What does an iPhone actually do at maximum settings?* is a more useful measurement than *does it
> hold 60 fps at Potato?* — the second confirms a guess; the first locates the edge.

### Collect everything relevant, in one file

Beyond §2's per-frame record and session block:

```
device        model, SoC/GPU, core counts, unified vs discrete, total and AVAILABLE memory
os            version, thermal state if exposed, power source, low-power mode
runtime       backend + driver, f64 support and rate, max workgroup/buffer limits
pressure      memory pressure state per frame, evictions, bytes resident, peak resident
load          CPU utilisation, GPU utilisation if exposed, whether the app was backgrounded
cores         pool size vs cores available, and the utilisation ceiling in force (§6.4)
queue         in-flight GPU depth, and any dispatch that overran its bound
thermal       any throttling signal available -- a device that sustains 60 fps for ten seconds
              and 30 fps for ten minutes is a different device
binding axis  converged / budget-bound / cap-bound, per frame (§3.5)
slice         which chart, and sea_fraction -- the SAME device on TWO SLICES is two data points,
              and the cost is a property of the field
```

**Thermal is the one most likely to be forgotten and most likely to matter on phones and laptops.** A
burst benchmark measures a device that does not exist after two minutes.

### Then invert

With that corpus: for each device, **the highest `eps` and budget that held 16.7 ms** — and where it
fell back toward 41.7. **That is the tier boundary, measured.** Until then the spec-keyed defaults are
placeholders and should be labelled as such in the config.

**A device nobody tested still gets a sensible answer** by running the fixed suite once on first
launch (§1.1) — which is why the suite must exist even after the tiers are calibrated.

---

## 8. What this is not

**Not a benchmark score.** The output is a set of settings that fit a budget on that device, not a
number to compare against other people's.

**Not automatic telemetry.** Explicit, opt-in, run when asked. The suite is a thing someone chooses
to run, and passive logging is a mode with a visible indicator.

**And not a substitute for the tier table having a rationale.** Measured data tells you where a
device's boundary is; it does not tell you which side of it the default should sit. That is still a
judgement about how much quality to trade for how much smoothness, and it should be written down
rather than fitted.
