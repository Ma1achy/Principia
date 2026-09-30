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

## RQ-175: in-shader quantisation doesn't make R-269's half-way fixture byte-identical: Metal's fast-math division moves R by one ulp across the tie *(physics, golden, calibration, TASK-M0-43, R-287)*

- **File, section:** `plan/tasks/M0/TASK-M0-43.md` § "Acceptance tests": "the R-269 fixture renders identical bytes
  in the `gpu-metal` and `gpu-lavapipe` jobs (max step 0 against one reference)". `plan/requirements.yaml`
  REQ-VAL-176, verify detail: "R-269's half-way fixture renders identical bytes on gpu-metal and gpu-lavapipe (max step
  0) with in-shader quantisation". `decisions.md` § "R-287": "so every backend writes identical bytes. … per-backend
  references stay the fallback if any case still differs". `docs/contracts/principia_parity_contract.md` § "4.
  Tolerance — and the cross-backend reality": "**Golden images are byte-exact across backends by construction
  (R-287).**"
- **What:** TASK-M0-43 builds the quantisation as R-287 says. The runner draws each case into an `Rgba32Float`
  target, and its own shader scales each channel to 0..255, rounds half to even (written out with `floor`, not
  `round`), and stores `k / 255` in the `Rgba8Unorm` target. Run 36736481929 (branch
  `measure/m043-lavapipe-halfway`, R-272, deleted afterwards) rendered R-269's fragment on hosted Metal and lavapipe:
  - The control (automatic conversion): Metal against lavapipe is max step 1 on 32768 of 65536 pixels, every even x,
    in R only. That is R-269's result, reproduced.
  - Quantised in the shader: Metal against lavapipe is still max step 1, on 24064 of 65536 pixels (94 of the 256
    columns, all even x), in R only. G and B are identical.
  - The rounding itself agrees. Lavapipe's quantised render is byte-identical to its automatic one, and at the columns
    where both backends' f32 `R × 255` is exactly x + 0.5, both round to even. Hosted Metal matches a local M3 Pro
    byte for byte.
  - The remaining difference is in the fragment's own arithmetic, before any rounding. On lavapipe, R =
    `(x + 0.5) / 255.0` matches correctly rounded f32 division. On Metal, it matches `(x + 0.5) × f32(1/255)`: wgpu
    30 compiles MSL with the default `MTLCompileOptions`, which has fast-math on, and offers no switch to turn it off.
    On 94 columns the two values are one ulp apart, on opposite sides of x + 0.5, so the rounded levels differ by one.
  - The fixture is made of exact ties, so any one-ulp difference changes the byte. The same exposure applies to any M1
    golden whose value lands within an ulp of a half-way point, and Metal's fast-math also approximates `exp`, `pow`
    and the like. Quantising in the shader removes the conversion's tie-break difference, but not "by construction"
    every cross-backend difference.
- **Options seen:**
  1. **The fixture takes the fallback (R-287's own words).** `quantise/halfway` keeps one reference per backend, as
     the control does. The PR names it (R-287: "the PR names it"). The acceptance line and REQ-VAL-176's verify detail
     change: the quantised fixture is max step 0 per backend against its own reference, and the evidence that
     quantisation works becomes "lavapipe's quantised bytes equal its automatic ones; at exact f32 ties both backends
     round to even". Parity §4's "by construction" sentence is qualified.
  2. **Make the fixture's R value backend-independent.** Compute R as `(x + 0.5) × 0.003921569` (an f32 multiply,
     correctly rounded on every backend) instead of `/ 255.0`. The quantised render should then be identical, and the
     66 columns where `R × 255` is still exactly x + 0.5 (x = 0, 2, 4, 5, 8, 9, …) still test the tie. The control
     still shows max step 1. This changes R-269's fragment, and it shows only that the rounding is identical, not
     that goldens are (see the last point under **What**).
  3. **Take Metal's fast-math out of the golden path.** Not possible through wgpu 30's API: the compile options are
     fixed inside wgpu-hal. It would need an upstream change or a raw-MSL passthrough, both outside M0.
- **Needed:** which one, and whether parity §4's "by construction" stands. TASK-M0-43 waits. Its implementation is
  committed locally on `task/TASK-M0-43` (1cd8cc2), not pushed, with no PR: runner, fixture, control references
  (Metal and lavapipe), tests and CI steps. Under option 1 or 2 only the fixture, its references and the acceptance
  wording change.
