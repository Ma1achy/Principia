# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-173: TASK-M5-30 allows only `QuadReduction` to be read back per frame; R-288 reads two counters back each frame *(plan, TASK-M5-30, R-288)*

- **File, section:** `plan/tasks/M5/TASK-M5-30.md` § "Acceptance tests", the checklist line for REQ-SYS-033 (also
  REQ-SYS-033's verify detail in `plan/requirements.yaml`): "audit every buffer map/readback: only QuadReduction is read
  back per frame; other readbacks are behind a user action and async". REQ-SYS-033's statement: "QuadReduction must be
  the sole automatic GPU-to-CPU crossing". `decisions.md` § "R-288": "Two per-frame atomic u32 counters … in telemetry
  §2. They ride on the existing profiler/telemetry readback, not a new GPU→CPU channel (QuadReduction stays the sole
  automatic return of simulation data, R-142), and are read back asynchronously with a frame or two of latency, never
  stalling the frame."
- **What:** R-288's counters come back every frame, on the profiler/telemetry readback, with no user action. Read
  literally, TASK-M5-30's checklist line fails on them. R-288's own words arguably settle it: they aren't a new
  GPU→CPU channel, and `QuadReduction` stays the sole automatic return of *simulation data*. REQ-SYS-033's statement
  and checklist don't draw that line. The same task's Goal ("the only automatic GPU → CPU crossing is the
  `QuadReduction` readback") and REQ-SYS-036's line ("finds only the reduction readback and the two sanctioned pulls")
  read the same way.
- **Options seen:**
  1. **Reword REQ-SYS-033 to "simulation data" (recommended).** `QuadReduction` is the only automatic per-frame return
     of simulation data; the profiler/telemetry readback (R-288) is not simulation data. REQ-SYS-033's statement and
     verify detail change through `plan/tools/reqio.py`, and TASK-M5-30's checklist line follows.
  2. **Leave it as written.** TASK-M5-30 reconciles it when it builds the membrane audit, citing R-288.
- **Needed:** which one. Nothing is blocked now: TASK-M5-30 is an M5 task.
- **See also:** R-294 and RQ-174 (archived, ruled by R-294) concern the same per-frame readback of R-288's counters.
  R-294 bears on the options but doesn't choose between them. It keeps the counters automatic and per frame (an atomic
  u32 buffer bound and reset each frame), so TASK-M5-30's checklist line, read literally, still fails on them. It also
  puts their readback "with the telemetry readback", not on `QuadReduction` or a channel of its own, which is the line
  option 1 draws between simulation data and telemetry. TASK-M5-28 now builds that readback (R-294), and TASK-M5-30
  doesn't reach TASK-M5-28 through its Depends on, so under option 2 its membrane audit may run before the counters'
  readback exists.

## RQ-180: A headless `prin profile` run opens no GPU, but the session header requires a GPU, a graphics API and its driver *(docs, TASK-M0-18, REQ-TOOL-002, REQ-TOOL-006)*

- **File, section:** `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The artefact: one file, plain text,
  readable by the sender", the session header: "device     gpu (model), cpu (model), cpu_cores, gpu_cores (null when
  not reported), memory: {"unified": {bytes}} or {"discrete": {vram_bytes, ram_bytes}}", "backend    api ("metal" /
  "vulkan" / "dx12" / "webgpu"), driver (its version)", "precision  f32, f64 (supported: true / false), f64_rate …",
  "display    width_px, height_px, refresh_hz, dpi_scale; null for a headless run", and "Every object below has
  exactly the keys listed, all required: an absent value is `null`, never a missing key." The typed form
  (`engine::contract::profile::SessionHeader`) and `profile_v1.json` follow it: `backend.api` is one of the four, and
  `device.gpu`, `device.memory`, `backend` and `precision` are never `null`. `plan/tasks/M0/TASK-M0-18.md`
  § "Deliverables": "A synthetic scenario registered (`synthetic_frames`: emits frame records with a fixed scope and
  event sequence, no physics)"; § "Goal": it "runs a registered, fixed, deterministic scenario headless".
- **What:** §5 makes only `display` nullable "for a headless run". The M0 `prin` opens no GPU adapter (no crate in
  its dependency tree does, and the synthetic scenario does no GPU work), so it has no GPU model, no graphics API, no
  driver version, no GPU memory split and no f64 support to report, yet the header requires all of them and the API
  must be one of the four. Writing, say, `"metal"` on a Mac would record a backend the run never used; a placeholder
  string can't be written for `api` at all. The corpus doesn't say what a run that opens no GPU writes, nor that
  `prin profile` must open one. The CPU model and the memory size are also not read by anything in the workspace
  yet, but they exist on every host; the GPU fields don't exist for this run.
- **Options seen:**
  1. **`prin profile` opens the GPU adapter it would render with, and records it, even when the scenario uses no GPU
     (recommended if the header describes the machine).** The header then describes the device the trace was taken
     on, as §2's "Per session, once" reads. It adds a GPU dependency (wgpu) to `prin` at M0 and needs an adapter on
     every machine that runs `cargo test -p prin profile_file`, CI's included.
  2. **Amend §5 and schema v1: `backend` and the GPU half of `device` and `precision` are `null` for a run that opens
     no GPU**, as `display` is for a headless run. A docs change first (the porting rule), then the typed form and
     `profile_v1.json` (TASK-M0-17's files), then TASK-M0-18.
  3. **Something else the human names** (for example a fifth `api` value, `"none"`).
- **Needed:** which one, and, under 1, whether CI must provide an adapter. Blocks TASK-M0-18 (REQ-TOOL-002's header,
  REQ-TOOL-006's run).

## RQ-181: The header's config "as the M0 contract skeleton serialises it", but the skeleton has no serialisation *(plan, TASK-M0-18, REQ-TOOL-002)*

- **File, section:** `plan/tasks/M0/TASK-M0-18.md` § "Acceptance tests" (and REQ-TOOL-002's verify detail in
  `plan/requirements.yaml`): "the header holds the build hash and the config as the M0 contract skeleton (TASK-M0-16)
  serialises it". `docs/design/principia_dd_telemetry_and_tiers.md` § 5: "config     the run's full configuration, a
  JSON object", and "It must carry the build hash and the full config, or it cannot be interpreted six weeks later".
  `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface":
  "Provenance already serialises `SimConfig + RenderState`". The task's § "Deliverables" lists only
  `crates/prin/src/profile/…`, the synthetic scenario, and `crates/prin/tests/profile.rs` with its fixtures.
- **What:** the M0 skeleton (`crates/engine/src/contract/{sim_config,render_state}.rs`) declares `SimConfig`,
  `RenderState` and their groups as empty structs with no serde derive and no serialised form, so there is nothing
  that "serialises it" to compare the header against. Three things are open: (a) whether the skeleton gains
  `Serialize` (a change to `crates/engine/src/contract/`, outside TASK-M0-18's Deliverables); (b) the shape of
  `config`: which top-level keys hold `SimConfig` and `RenderState` (serde's field names give `{"chart": {}, …}` for
  each, but not the keys above them); (c) whether "the run's full configuration" of a `prin profile` run also holds
  the scenario name and the frame count, which neither struct has, and which the synthetic scenario, having no
  physics, is the whole of.
- **Options seen:**
  1. **TASK-M0-18 adds `#[derive(Serialize)]` to the skeleton's `SimConfig`, `RenderState` and their groups, and
     writes `config` as `{"scenario": NAME, "frames": N, "sim": SimConfig, "render": RenderState}`, serde's field
     names beneath (recommended).** Its Deliverables gain the two contract files. The header then holds the
     skeleton's serialisation, as REQ-TOOL-002's verify reads, and the scenario that produced the trace.
  2. **`config` holds only the scenario and the frame count at M0**; REQ-TOOL-002's verify detail drops "as the M0
     contract skeleton (TASK-M0-16) serialises it" until a task gives the skeleton a serialised form.
  3. **Another shape the human names.**
- **Needed:** which one, and the key names if not option 1's. Blocks TASK-M0-18 (REQ-TOOL-002).
