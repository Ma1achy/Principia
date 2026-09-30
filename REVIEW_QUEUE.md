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

---

## RQ-176: what R-297's compute fast-math setting means in the browser build, where WebGPU offers no fast-math control *(design, M8, R-297)*

- **File, section:** `docs/notes/principia_gpu_determinism_note.md` § "The mechanism (measured, not inferred — the
  attribution was overturned by a controlled test)": "\"turn off fast-math\" is not a fix (and is not available in a
  browser regardless)". `decisions.md` § "R-297": "Compute shaders (the simulation): fast-math is an explicit, recorded
  setting, off by default and on as an opt-in optimisation; the project never inherits it silently. Off: bit-identity
  and identical branch decisions hold (R-84, REQ-INT-057). … The compute setting is part of the sim key, recorded in
  pxpack, and shown in the profiler and the Run window." `decisions.md` § "R-85": "Real browsers are checked against
  those tolerances with the browser build (M8)." `docs/design/principia_dd_telemetry_and_tiers.md` § "5. The
  artefact: one file, plain text, readable by the sender": the header records "the setting asked for … and the mode
  of each stage, compute, vertex and fragment, as compiled on the running backend".
- **What:** R-297 makes compute fast-math off the default, and on Metal it reaches off through wgpu's passthrough, a
  native API. In the browser build (TASK-M8-37, TASK-M8-40) the shaders go to the browser's WebGPU as WGSL, and
  WebGPU has no fast-math control and no passthrough; the browser's own compiler chooses the math mode, and it isn't
  exposed. So off can't be guaranteed there, the compiled compute mode can't be read for the header, and the Run
  window's control has nothing to act on. The corpus doesn't say what the setting means in the browser. Nothing in M0
  to M7 depends on the answer. It was flagged in R-297's Applied note ("Flagged, not applied") and asked in PR #88.
- **Options seen:**
  1. **The browser's compute mode is recorded as unknown (recommended).** In the browser build the header records
     the setting asked for and the compute mode as compiled "unknown" (likewise vertex and fragment); the Run window
     shows the control disabled, with a note that the browser chooses; browser runs are held to R-85's Tier-N
     tolerances, measured, not bit-exact, as with the setting on. The sim key and pxpack keep the setting, so a view
     made in the browser opens natively with it.
  2. **The browser counts as on.** The browser build records compute fast-math as on, since it can't be shown off,
     and treats browser parity as measured, not exact. Simpler, but it records a mode nobody measured.
  3. **Measure each browser's mode.** A probe like TASK-M0-44's `compute_fast_math` runs in the browser at start-up
     and records off where the probe's divisions are correctly rounded, unknown otherwise. More work, and a probe
     passing shows correct rounding on its inputs, not that fast-math is off.
- **Needed:** which one. Nothing is blocked now: the browser build is M8 (TASK-M8-37, TASK-M8-40).
