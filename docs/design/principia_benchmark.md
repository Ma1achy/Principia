# Principia: the benchmark

**Adopted by R-493 to R-499 (20 Oct 2026, the human's B1 to B7).** This document is part of the corpus from those
rulings on, read with them; it is telemetry §1.1's fixed suite made concrete (`principia_dd_telemetry_and_tiers.md`).
Its §10's three open questions are not ruled: they are RQ-392 to RQ-394. Read with the review queue's entries on points
where this document and an earlier ruling or doc disagree: RQ-399 (the result file's schema version, §6) and RQ-400
(the run's window size and the work, §5.1, §8). Its §4 times are estimates until the first Full run on the reference
machine replaces them (R-493, R-495).

*Design document. Goes to `docs/design/principia_benchmark.md`. It makes telemetry §1.1's "fixed suite" concrete, and
gives it a button, a file and a report. It changes nothing in the telemetry doc's frame record, percentiles, tier axes
or failure handling; it uses them.* *(Its status as delivered; R-493 to R-499 adopt it, above.)*

---

## 1. What exists, and what this adds

**Exists** (origin/main `d7e6f50`):

*Checked when adopted (R-493): each item below holds on the base the rulings were written on.*

- **The telemetry design** (`principia_dd_telemetry_and_tiers.md`):
  - §1.1 the fixed suite (fixed slices, zoom ladder, pan path, playhead march, at several quality settings, sweeping
    rather than sampling);
  - §2 the frame record;
  - §3 percentiles, never means;
  - §3.5 the three tier axes and "always report which axis bound";
  - §5 the one-file artefact, "going to friends over Discord", profiler schema v1;
  - §6.4 thermal and the utilisation ceiling;
  - §7 the calibration campaign, "push every device past where it is comfortable";
  - §8 "not a benchmark score".
- **`prin profile <scenario>`**: runs a registered scenario headless, streams a schema v1 trace, and diffs two traces
  against a threshold. One scenario is registered: `synthetic_frames`.
- **`cargo xtask bench`**: developer regression benches with baselines in `fixtures/bench/`. One bench:
  `trivial-kernel`.
- **The Profiler window task**: passive logging, live view and the leak detector.

**Adds:**

- **A scenario format:** data, not code.
- **The suite:** the scenarios that make up §1.1, written out.
- **Three run modes:** Quick, Full and Soak.
- **"Run benchmark"** in the GUI, and `prin bench` on the command line.
- **The result file:** one file a friend sends back.
- **`prin bench report`:** turns a result into something an agent can act on.
- **Comparison** across devices.

**One runner.** `prin profile`, `cargo xtask bench` and the GUI's button run scenarios through **one** scenario
runner and **one** scenario registry. They differ in which scenarios they select, not in how a scenario runs, so a
regression bench and a friend's benchmark measure the same thing the same way.

---

## 2. A scenario is data

A scenario is a file in `bench/scenarios/`, read by the runner. Adding one needs no code.

```json
{
  "format": "principia.bench.scenario/1",
  "id": "pan-nearfield",
  "title": "Pan across Burrau near-field",
  "measures": "frame time during motion; the scheduler's churn while the camera moves",
  "requires": ["gpu"],
  "start": { "sim": { "...": "SimConfig, canonical" }, "render": { "...": "RenderState, canonical" } },
  "warmup_frames": 60,
  "timeline": {
    "unit": "frames",
    "keyframes": [
      { "at": 0,   "set": { "camera.cx": -0.4 } },
      { "at": 600, "set": { "camera.cx":  0.4 }, "ease": "linear" }
    ]
  },
  "settle": { "after": true, "max_frames": 1200 },
  "sweep": ["tier"]
}
```

- **`start`** is a full `SimConfig` and `RenderState` in their canonical serialisation, so a scenario starts from a
  state the round-trip rule can reproduce. No scenario depends on a user's settings.
- **`timeline`** reuses the export contract's keyframe spec (Part 3) with one difference: time is in **frames**, not
  wall-clock seconds. A slow device does the **same work** in more time, which is what makes devices comparable
  (§1.1: "the only thing where the work is held constant"). Edits go through `SetField`, exactly as the GUI's own
  edits do.
- **`settle`** keeps running after the timeline until the view converges, or until `max_frames`. Time-to-settled is a
  measurement, so the cap is recorded when it bites, never treated as success.
- **`sweep`** names the axes the suite crosses this scenario with (§4).
- **`requires`** declares `gpu`, `window` (the scenario must present) or nothing. A runner that cannot meet a
  requirement **skips the scenario and records the skip**, never a pass.
- **Validation at load.** Unknown keys are refused (`deny_unknown_fields`). Every `SetField` path is checked against
  the contract's schema, so a scenario cannot name a field that no longer exists.
- **The suite has a hash**, the content hash of its scenario files. Two results are comparable only when their suite
  hashes match, and `compare` says so when they don't.

---

## 3. The suite

Each entry exists because it isolates one cost the tier has to bound. "Bound by" is the §3.5 axis the scenario is
expected to stress; the measurement decides.

| id | What it does | Measures | Expected to bind |
|---|---|---|---|
| `cold-start` | launch to the first presented frame, and to settled, on the default chart | shader compile and pipeline creation (separately), first frame, time to settled | budget |
| `settle-<chart>` | a static view settling, one per chart in the set below | time to converge; `sea_fraction` recorded, since the cost is a property of the field | eps / budget |
| `pan-nearfield` | a fixed pan across Burrau near-field | `frame_ms` during motion, frac over 41.7 ms, churn | budget |
| `zoom-ladder` | zoom in by powers of two to depth 44 and back, on an O(1) chart coordinate | frame time per depth; the f32 → linearised switch; the deep-zoom floor | budget / cap |
| `playhead-march` | fixed view, playhead 0 → `T_horizon` at normal speed | per-frame integration cost under lockstep | budget |
| `scrub` | fixed view, jumps back and forth across the playhead | re-reduction cost of a playhead change (`reproj`) | budget |
| `tilt-drag` | a two-angle tilt drag (re-integrates every frame) | worst case for the cache | budget / cap |
| `stain-switch` | cycle six stains on a settled view | colour-only cost; asserts zero substeps | none |
| `print-<class>` | one light, one medium and one heavy Print preset, each still and then panned | `stage_ms.post`, the degrade level, settle | budget |
| `lines` | contours, an animated trace and axes on, playhead running | the line system's cost | budget |
| `memory-ramp` | resolution and depth raised until the first eviction, then held | peak resident, evictions, pressure state | cap |
| `export-1024`, `export-4096` | a blocking figure export | wall time per megapixel | none (batch) |
| `soak` | `pan-nearfield` and `playhead-march` looped for ten minutes | sustained against burst frame time; throttling | thermal |

**The chart set** for `settle-*`, chosen to span the field's cost (refinement policy: "5–37× less than uniform on
Burrau regions, about the same on arbitrary latent slices"):

- `near-field` and `deep-interior` (Burrau, structure localised and everywhere);
- `far` (smooth: the control);
- `config_stability` (latent);
- `preset_shape_h1` (the sea).

**Every scenario records which axis bound, per frame** (§3.5). A summary that reports a fast frame time without
saying the image was cap-bound is the failure §3.5 exists to prevent.

---

## 4. The sweep, and the three run modes

**The tier ladder is swept, including tiers the device will fail** (§7: "you cannot see a boundary from one side of
it"). During a swept row the tier is **fixed**: the quality arbiter is switched off, because a controller adapting
during the measurement moves the thing being measured. One additional row per scenario runs with the arbiter **on**,
and that row measures the arbiter.

| Mode | What runs | Time on a mid laptop (to be measured) | For |
|---|---|---|---|
| **Quick** | `cold-start`, `settle-near-field`, `pan-nearfield`, `playhead-march`, `print-heavy`; arbiter row plus two tiers either side of the suggested one | ~2 min | a first look; first-launch tier suggestion (§7) |
| **Full** | every scenario except `soak`, across the whole tier ladder | ~15–20 min | sending results back; calibration |
| **Soak** | Full, then `soak` | Full + 10 min | laptops and phones; §7: "a burst benchmark measures a device that does not exist after two minutes" |

The times are estimates. The first Full run on the reference machine replaces them with measurements in this table.

---

## 5. Running it

### 5.1 From the GUI

**Profiler window › Run benchmark…** opens a short dialog:

- **Mode:** Quick, Full or Soak, with the estimated time.
- A **plain statement of what is recorded:** device, OS, driver, build, the scenarios' settings and the timings. Then:
  "Nothing about you or your files. You can read the file before sending it." (§5: readable by the sender, and the
  reason they will send it.)
- **Plug in** (for Soak, and recommended for Full on battery). Power source and low-power mode are recorded (§7).

During the run:

- The **chrome is hidden** and the figure fills the window (present mode, R-406), so every device draws the same
  surface.
- A **thin progress strip** shows the scenario name, `n of N`, and the time remaining.
- **Esc aborts.** The file is still written, marked `"complete": false`, with every finished scenario intact.
- **Input is ignored except Esc.** A stray mouse move would be a camera edit, and it would change the work.

Afterwards a **results sheet** shows:

- per scenario: p50, p95 and p99 frame time during motion, frac over 41.7 ms, time to settle, and which axis bound;
- the suggested tier (§7: "the highest `eps` and budget that held 16.7 ms").

It has three buttons: **Save file…**, **Copy summary** and **Apply suggested tier**. Applying is one undo entry, and
never automatic (§8: "not automatic telemetry").

### 5.2 From the command line

```
prin bench run   [--mode quick|full|soak] [--scenario ID...] [--out FILE]
prin bench show  FILE [--pretty]
prin bench report FILE [FILE...] [--format md|json]
prin bench compare A B [--threshold ...]
```

- **`run`** is headless where every selected scenario allows it. Scenarios requiring `window` open one.
- **`compare`** is `prin profile diff` applied per scenario, with the suite-hash check (§2).

### 5.3 In the browser build

The same dialog and runner. The result is saved through the browser's download. The header records the browser and
its WebGPU adapter info, and marks fields the browser does not expose as `null` (the R-308 / R-303 pattern), never
guessed.

---

## 6. The result file

**One file**, `principia-bench-<device-slug>-<YYYYMMDD-HHMM>.jsonl`, profiler schema v1 **plus** a benchmark block. It
stays JSON Lines and plain text (§5).

1. **Line 1: the session header** (§2's session block, as today), plus `bench`:
   - `mode`, `suite_hash` and `complete`;
   - the list of scenarios with `ran | skipped(reason) | aborted`;
   - power source, low-power mode and thermal state at start.
2. **Per scenario:**
   - a `scenario_begin` line with its canonical `start` state and its sweep row;
   - the frame records (§2), downsampled by §5's rule (every frame during motion, every Nth while idle);
   - a `scenario_end` line with that row's **summary**: frame-time percentiles split by `camera_delta > 0` and
     `= 0`, frac over 16.7 and 41.7 ms, settle frames and time, bound-axis counts, `stage_ms` percentiles per stage
     including `post`, peak resident bytes, evictions, throttling signals and `post_level` counts.
3. **Last line:** the run summary, the suggested tier, and the inversion table it came from.

**Size target:** under 20 MB for Full, so it travels over Discord uncompressed. If measurement shows it doesn't,
Full writes `.jsonl.gz` and the dialog says so.

**Never a score** (§8). The summary is a table of settings that held the budget and the measurements behind them.
There is no single number to put on a leaderboard.

---

## 7. Feeding it back: `prin bench report`

This is the part that turns a friend's file into work for the agents.

`prin bench report FILE` writes a Markdown report, fixed in structure so an agent can rely on it:

1. **Device and build:** one block, from the header.
2. **Where the budget went:** per scenario, the stage that dominated p95 frame time (`integrate`, `reduce`, `colour`,
   `post`, `upload`, `present`), and the bound axis.
3. **The worst five frames,** each with its full work record (§2: "a frame time is meaningless without the work it
   did"), so a slow frame can be reproduced.
4. **Anomalies:**
   - a scenario cap-bound at rest;
   - a stain switch that integrated;
   - a still view that left `post_level: full`;
   - sustained frame time more than 1.5× burst (thermal);
   - any `dmin_nan_unset` or `dmin_negative_floored` above zero.
5. **Against the reference:** if a baseline is given, the regression diff per scenario.
6. **Reproduce:** the exact `prin bench run --scenario ...` line for each anomaly.

With several files, `report` adds a **device comparison table**: per scenario, each device's p95 during motion and its
bound axis. That table is what tells the agents whether a bottleneck is universal or one device's.

---

## 8. Determinism of the work

The work a scenario asks for must be identical across runs and devices; only the time may differ.

- **Timelines step in frames** (§2).
- **No wall-clock-driven behaviour inside a swept row.** The arbiter is off, there are no frame-time-adaptive
  decisions, and degrade-while-moving uses its policy, not a measured budget, so the work is fixed. Its adaptive form
  is measured in the arbiter row.
- **Fixed seeds** everywhere (ensemble offsets already are; the Print graph is by §6.1 of its doc).
- **Warm-up frames** are excluded from the summary, and pipeline compile time is recorded on its own line, because on
  the web it can dominate first-frame cost and would otherwise smear across `cold-start`.

---

## 9. Tests (each with an arm that can fail)

1. **The work is deterministic.** Run Quick twice on one machine: the work counters (`quads_computed`, `samples`,
   `substeps_total`, per frame) are identical. Timings may differ. Control: changing one scenario's seed must change
   them.
2. **One runner.** The same scenario through `prin profile`, `cargo xtask bench` and the GUI runner produces identical
   work counters.
3. **Abort.** Aborting mid-run writes a valid file with `complete: false` and every finished scenario's summary.
   `report` and `compare` accept it and say it is incomplete (the existing "session incomplete" behaviour).
4. **Skip, not pass.** A runner without a window skips `requires: window` scenarios and records the skip. Control: a
   scenario that reports a summary without having run must fail validation.
5. **Validation.** A scenario with an unknown key or a stale `SetField` path is refused at load, naming the file and
   key.
6. **Suite hash.** `compare` on two files with different suite hashes says so and compares only the scenarios whose
   definitions match.
7. **The summary is the records.** For every scenario, the summary's percentiles recomputed from the file's own frame
   records match. Mutation arm: a summary computed over all frames, not split by motion, must fail.
8. **Readable.** The file opens as plain text, line 1 is the header, and no line exceeds a bound.
9. **Report structure.** `report` on a fixture file produces every section in order, including anomalies seeded into
   the fixture. Control: a clean fixture lists no anomalies.

---

## 10. Open questions (for the review queue)

*Queued as RQ-392 to RQ-394, one per question, in order (the human's point 6); what is built meanwhile is each
entry's.*

1. **First launch.** §7 says a device nobody tested gets a sensible answer "by running the fixed suite once on first
   launch". Opt-in prompt, or silent Quick with the result shown? §8 says "explicit, opt-in, run when asked", so the
   default reading is a prompt.
2. **Collecting results.** Files come back by hand (Discord). Should there ever be an upload? §8 rules out automatic
   telemetry; an explicit "Send to Malachy" would still need hosting. Out of scope unless ruled in.
3. **Phones.** The browser build on a phone cannot hide the browser chrome or prevent sleep. Should a phone run record
   the visual-viewport size and request a wake lock, and mark the run "mobile, best effort"?
