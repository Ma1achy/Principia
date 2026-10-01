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
In the dev GUI, passive logging is a Profiler switch, with its indicator in the footer (`principia_render_gui_spec.md`
§G5, R-129).

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

- `dmin_nan_unset` — u32, the number of `d_min` packs this frame that stored a NaN as the unset value (R-281, R-288)
- `dmin_negative_floored` — u32, the number of `d_min` packs this frame that clamped a negative value to the floor (R-281, R-288)

Both are atomic counters, counted in release builds as well as debug ones, and read back asynchronously on the
profiler/telemetry readback a frame or two late, never stalling the frame; `QuadReduction` is unchanged and stays the
sole automatic return of simulation data (R-142, R-288).

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
- **The header records the compute fast-math setting and each shader stage's fast-math mode (R-297):** the setting
  asked for, which is the sim key's, off by default; and the mode of each stage, compute, vertex and fragment, as
  compiled on the running backend. The two can differ: where a backend's own path compiles without fast-math and
  offers no switch (Vulkan, on lavapipe), the setting on compiles the compute stage off, and the header shows both. The
  display stages may keep fast-math on (`principia_parity_contract.md` §4). In the browser build, where WebGPU offers
  no fast-math control, the header records the setting asked for and each stage's compiled mode as "unknown" (R-303).
- **Format: JSON Lines, profiler schema v1 (R-56, R-286).** At the top level, §2's frame record and its five stages;
  beneath them, nested scopes, GPU passes, allocations and events. The dev GUI's profiler and `prin profile` read and
  write it (`principia_render_gui_spec.md` §G5). JSON Lines is plain text, so the file stays readable by the sender:
  the header on the first line, then one compact frame record per line. Pretty-printing is on demand
  (`prin profile show --pretty`, or `jq`), never in the file, so the file meets "readable" and "bounded size"
  together (R-286).
- **Also: Chrome Trace Event format (R-207).** Export trace also writes the same capture in the Chrome Trace Event
  format, openable in Perfetto and `chrome://tracing`, alongside schema v1: CPU scopes as complete events, GPU passes
  on their own track, counters as counter events. Schema v1 stays the file `prin profile` and the dev GUI read.
- **Bounded size.** A long session at 60 fps is 200k+ frame records. Either downsample on write
  (keep every frame during motion, every Nth while idle) or roll up idle stretches into summaries.

**Profiler schema v1: the keys and the nesting (R-56, R-72).** This is the definition REQ-TOOL-120 asks for. Its typed
form is `engine::contract::profile`, and its JSON Schema is `crates/engine/src/contract/schema/profile_v1.json`; both
follow it. Every object below has exactly the keys listed, all required: an absent value is `null`, never a missing
key. A key named `ms` or ending in `_ms` is wall-clock milliseconds, a number ≥ 0; counts and sizes are integers ≥ 0.

The ranges, which the typed form and the JSON Schema both hold: `cpu_cores`, `gpu_cores`, `width_px`, `height_px`,
`tree_depth_max`, `dmin_nan_unset` and `dmin_negative_floored` are at most 2^32 − 1, and every other count or size at
most 2^64 − 1. `camera_delta`, `refresh_hz`,
`dpi_scale` and `f64_rate` are ≥ 0 too; `playhead_dt` is signed. Every number is finite. A frame's `stage_ms.present`
and `stages.present` are both `null` or both present; each pool's `bytes` in `live_memory` is the sum of its `by_kind`
bytes; and a pool's `by_kind` has at most one entry for each `kind`, and a stage's `allocations` at most one for each
`kind` and `pool` (all three below). A writer given a value outside its range, NaN or an infinity, or a frame that
breaks any of those three rules, fails rather than write it, and a reader rejects all of them, so every line of a file
the reader accepts validates against the JSON Schema's definition for its place (below). The reverse holds with four
exceptions, which the schema accepts and the reader rejects: a count or size written with a zero fraction
(`"cpu_cores": 4.0`), which JSON Schema's `integer` admits; a key repeated within an object whose keys this section
lists, where the schema sees only the last copy; a
pool whose `bytes` is not the sum of its `by_kind` bytes, a sum JSON Schema cannot express; and two `by_kind` entries
in one pool with the same `kind`, or two `allocations` entries in one stage with the same `kind` and `pool`, a
uniqueness by key that JSON Schema cannot express. A key repeated anywhere inside `config` or inside a leak-flag or
hot-path entry is not an exception: the reader and the schema both keep the last copy. A writer produces none of the
four: it writes every count and size as a JSON integer, each key once, each pool's `bytes` as that sum, one `by_kind`
entry per type in a pool, and one `allocations` entry per kind and pool in a stage.

**The file** is JSON Lines (R-286): one compact JSON object per line. The writer ends every line with a newline, and
a reader also accepts a last line without one, as JSON Lines allows. The header line comes first, then one line per
frame record (none, for a session that recorded no frame), then the summary line, last:

```
header line    {"schema": "principia-profile-v1", "header": the session header}
frame lines    one frame record per line, in the session's order
summary line   {"leak_flags": [leak flag, ...] or null, "hot_paths": [hot-path summary, ...] or null}
```

Each line is one object whose keys this section lists: the header line has exactly `schema` and `header`, a frame line
is exactly a frame record, and the summary line has exactly `leak_flags` and `hot_paths`. A line that is not the object
its place calls for, or a blank line among them, is not schema v1. The writer
writes each line compact, buffered, and never pretty-prints; `prin profile show --pretty`, or `jq`, pretty-prints on
demand (R-286). A writer can stream the frames as the session runs, since nothing before the summary line depends on a
later frame. The JSON Schema defines one line for each place, in `$defs`: `header_line`, `frame` and `summary_line`.
A file's lines are not one JSON document, so the schema checks the file line by line, each line against the
definition for its place.

**A session that ended before its summary line** (R-298), because it crashed or is still running, leaves a trace whose
last line is a frame record, or the header line when it recorded no frame. That trace is valid schema v1. A reader
returns its header and frames, reports `leak_flags` and `hot_paths` as absent with "session incomplete", and never
rejects the file for the missing summary line; `prin profile query --live` reads an in-progress trace this way. Each
line is checked against the definition for the place it holds, so the last line is a `frame` (or the `header_line`).
Only the summary line may be missing, besides the cut-off line below.

**A line cut off at the end** (R-299). A last line with no newline after it that is not one complete JSON value is the
part of a line the session was writing when it stopped, not a line of the trace. A reader drops it, reads the lines
before it as the trace (the line before it holds a frame's place, so it is a frame record or the header line), reports
the session incomplete, with "session incomplete" as above, and states the number of bytes it dropped. A last line with
no newline that is one complete JSON value is read as any last line is. A line that ends in a newline and is not the
object its place calls for is not schema v1, wherever it is, and a reader rejects the file for it. A file whose only
line is cut off has no header line, and a reader rejects it as it does an empty file.

`leak_flags` and `hot_paths` are the precomputed leak flags and hot-path summaries that `principia_render_gui_spec.md`
§ "Profiler" puts in schema v1, so an agent reads conclusions, not raw traces. They summarise the whole session, so
they sit at the file's top level, on the summary line after the frames, where a writer that streams the frames writes
them once the session ends. Each entry is a JSON object, and the task that closes REQ-TOOL-100 (M8)
defines its keys; until then, a writer writes `null`, and a reader accepts any object as an entry.

**The session header** carries §2's per-session fields, and the full config that §5 requires:

```
device     gpu (model), cpu (model), cpu_cores, gpu_cores (null when not reported),
           memory: {"unified": {bytes}} or {"discrete": {vram_bytes, ram_bytes}}
backend    api ("metal" / "vulkan" / "dx12" / "webgpu" / "none"), driver (its version)
precision  f32, f64 (supported: true / false), f64_rate (the reported f64 rate as a fraction of the f32 rate;
           null when not reported)
build      commit (the hash), profile (the release profile), features ([flag, ...])
display    width_px, height_px, refresh_hz, dpi_scale; null for a headless run
config     the run's full configuration, a JSON object, in the canonical serialisation (R-309)
```

Unified memory is its own variant, not a VRAM size of zero (§2).

**A session that opens no GPU (R-308)** writes `backend.api` "none", and `null` for the GPU's own fields:
`backend.driver`, `device.gpu`, `device.gpu_cores`, `device.memory` and `precision`. `device.cpu` and `device.cpu_cores`
are written as always. Readers accept this header. A run never opens a GPU adapter only to fill the header: `prin
profile` running a scenario that does no GPU work (M0's `synthetic_frames`) writes this form.

**`config` (R-309)** holds `SimConfig` and `RenderState` in their one canonical serialisation, the same text snapshot
JSON, share links and pxpack carry (`principia_gui_state_contract.md` §2). A `prin profile` run writes it as
`{"scenario": NAME, "frames": N, "sim": SimConfig, "render": RenderState}`: the scenario it ran, the frame count, and
the two structs.
Written canonically (JCS, R-318), the object's members appear sorted: `frames`, `render`, `scenario`, `sim`.

**The frame record** is §2's, key for key, followed by the five stages' nested sections and the memory live at the
frame's end:

```
frame           the frame's index in the session, from 0 (so downsampled frames keep their place)
frame_ms        wall clock
quads_computed  quads_reused  samples  substeps_total
playhead_dt     how far time moved (signed): the change in the playhead's simulation time t this frame
camera_delta    pan/zoom magnitude: > 0 when the camera moved this frame, 0 when it did not
tree_depth_max  leaf_count
dmin_nan_unset  dmin_negative_floored
stage_ms        {integrate, reduce, colour, upload, present}: each stage's ms
stages          {integrate, reduce, colour, upload, present}: each stage's nested sections
live_memory     {heap, gpu, tile_cache}: each pool's live memory at the frame's end,
                {bytes, by_kind: [{kind (the type), count, bytes}, ...]}
```

`live_memory` is what `principia_render_gui_spec.md` § "Profiler" draws: memory (heap, GPU, tile cache) over time is
each pool's `bytes` frame by frame, and live allocations by type, with their change over 60 s, is each pool's
`by_kind`. A pool's `bytes` is its total live bytes; `by_kind` has one entry for each type with live allocations in the
pool, its live count and bytes; and a pool's `bytes` is exactly the sum of its `by_kind` bytes, so every tracked live
byte has a type. The three pools are disjoint: a tracked allocation counts in exactly one pool, so the three stack
without counting a byte twice. The tile cache lives in heap or GPU memory, and its bytes count in `tile_cache` only,
never also in `heap` or `gpu`: `heap` and `gpu` are the tracked memory outside the tile cache. The same holds for the
stages' `allocations`, whose `pool` names the one pool. The leak detector reads the same figures across idle frames.
It is a snapshot, not a change, so a downsampled file still shows each kept frame's memory. The stages'
`allocations` say which stage made the allocations.

`stage_ms` and `stages` have exactly the five stages as keys, written in that order, and nothing else. A batch render
has no present stage (§5.5), so its `stage_ms.present` and `stages.present` are `null` and the keys stay the same.
The two are `null` together or present together: a frame with a present time and no present sections, or the reverse,
is neither a batch render nor an interactive frame, and is not schema v1.

`dmin_nan_unset` and `dmin_negative_floored` are telemetry §2's two per-frame `d_min` counters (R-288, closing
RQ-171), each a u32 count for the frame: how many `d_min` values the packer received as NaN and stored as unset, and
how many it received negative and clamped to the floor (R-281). They are counted in release builds too, and come back
on the profiler/telemetry readback a frame or two late, never stalling a frame (R-288).

`playhead_dt` is the change in the playhead's simulation time `t` over the frame, in the unit of `T_horizon`
(`principia_integrator_contract.md` Part 3, physical time). It is negative when the playhead moves back and 0 when it
does not move, and it is not wall-clock time. v1 makes only the sign of `camera_delta` normative: `camera_delta > 0`
means the camera moved (panned or zoomed) this frame and 0 means it did not, the moving/still discriminator of §3.
The magnitude's metric is not defined in v1; the task that first consumes the magnitude defines it.

**A stage's nested sections** sit beneath it:

```
scopes       [scope, ...]        CPU scopes, nested: {name, start_ms, ms, children: [scope, ...]}
gpu_passes   [gpu pass, ...]     {name, start_ms, ms}, from the GPU timestamps
allocations  [allocation, ...]   {kind (the type), pool ("heap" / "gpu" / "tile_cache"), count, bytes}: the
                                 allocations the stage made, one entry per kind and pool
events       [event, ...]        {name, at_ms, detail (text, or null)}
```

`start_ms` and `at_ms` count from the start of the frame. A scope exists only beneath a stage, so every scope has one
of the five as its ancestor. The finer categories (quadtree, stain + style, IC decode, readback, egui;
`principia_render_gui_spec.md` § "Profiler") are scopes nested in whichever stage runs them. A scope at the top of a
frame record, beside the stages, is not schema v1.

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
reclaiming    over it -- evict, lowest cost-weighted resistance first (R-120)
```

**The middle state is the one that matters and the one most likely to be missed.** A design with only
*fine* and *OOM* oscillates between them.

#### Eviction order, and the trap in it

Drop the cached quads with the **lowest cost-weighted resistance** first — eviction resistance ∝ `computeCostMs`
(scheduler Part 6, caching Part 7), so the cheapest to recompute go first; expensive quads resist (R-120).

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
  drop the cached quads with the lowest cost-weighted resistance first (R-120), since they are the
  cheapest to recompute. Never drop the coarse ancestors: those are what the fill uses during motion, and
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
force so the calibration is not polluted by mixing the two. In the dev GUI the target-utilisation ceiling is set in the
Run window (`principia_render_gui_spec.md` §G5, R-129).

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
fell back toward 41.7. **That is the tier boundary, measured.** Until then the defaults in the tier tables (§3.5) are
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
