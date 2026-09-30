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

## RQ-182: how M5 types `QuadReduction`'s f16 members: `f16-pair` at packed 16-bit locations, or a new §3.8 `f16` type *(docs, M5, TASK-M5-01, R-306)*

- **File, section:** `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger":
  "| `spread_event` | f16 |" and eleven more f16 members. The same file § "3.8 Metadata schema": "type: u-bits | f32 |
  f16-pair | fixed16 | vector(type, k)"; "At a packed location, `f16-pair` and `fixed16` take exactly 16 bits"; "A field
  without a complete entry fails generation loudly." `decisions.md` § "R-306": "RQ-178's sub-question, how M5 types the
  f16 members (`f16-pair` at packed 16-bit locations, or a new §3.8 `f16` type), was not ruled; it is filed as RQ-182
  for M5". Archived RQ-178 (`docs/archive/review_queue/M0.md`), item 3.
- **What:** §3.8 has no plain `f16` type; an f16 value fits a §3.8 entry only as `f16-pair` at a packed 16-bit location.
  Stable Rust has no `f16` type (`error[E0658]: the type f16 is unstable`, rustc 1.98.1), so the generated struct stores
  f16 values as binary16 bits either way. TASK-M5-01 writes `QuadReduction`'s §3.8 entries, member order and packing
  (REQ-PAY-077), and needs a type for each f16 member.
- **Options seen:**
  1. **Every f16 member is `f16-pair` at a packed 16-bit location.** No schema change; the member order and packing
     (REQ-PAY-077) place each f16 in half of a 32-bit word.
  2. **§3.8 gains a type `f16`** (a docs change to §3.8 first, then the ledger's type set and both emitters), usable at
     a scalar index or a packed 16-bit location.
- **Needed:** which one, before TASK-M5-01 starts. Nothing in M0 is blocked.

---

## RQ-183: `n_unresolved` is carried by `QuadReduction` but has no §3.7 row or type *(docs, M5, TASK-M5-01, R-142, R-306)*

- **File, section:** `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger",
  "Temporal accumulators": "`QuadReduction` carries only the verdict: the count of the quad's unresolved footprints,
  latched ones included (policy §1's `n_unresolved`). `QuadReduction` stays the sole automatic return." The same
  section's "Refinement" row for `alpha_area`: "An empty mask is told from a full one by `n_unresolved`". No §3.7
  table has an `n_unresolved` row, so §3.7 gives it no type. `decisions.md` § "R-306": "TASK-M0-11 transcribes
  generation-root §3.7's `QuadReduction` members … each with its name and §3.7 type".
- **What:** TASK-M0-11 records §3.7's member list as ledger data (R-306). It lists `n_unresolved`, since §3.7 says
  `QuadReduction` carries it, with its type recorded as not given (`ty: None` in `crates/ledger/src/payload.rs`'s
  `QUAD_REDUCTION`). TASK-M5-01 builds the struct and writes each member's §3.8 entry, and needs a type for it.
- **Options seen:**
  1. **A §3.7 row for `n_unresolved`, typed `u16`,** as `valid_sample_count` ("decoded of N²") is: a count of a quad's
     footprints is at most N².
  2. **A §3.7 row with another type** (`u32`, or a width fixed from N).
  3. **`n_unresolved` is not a member,** and the verdict reaches the CPU some other way; §3.7's sentence changes.
- **Needed:** which one, before TASK-M5-01 starts. Nothing in M0 is blocked: the M0 list records the member without a
  type, and REQ-PAY-019's test does not read it.
