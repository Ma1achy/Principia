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

## RQ-184: TASK-M0-14 declares `cfg(target_arch, values("spirv"))` for the `unexpected_cfgs` lint in `crates/kernel/build.rs`, which is lint configuration *(code, TASK-M0-14, PR #96, R-197)*

- **File, section:** `decisions.md` § "R-197 — Who may fix, suppress or configure a lint *(closes RQ-134)*": "Changing
  lint configuration (`clippy.toml`, `[lints]` tables) needs a ruling." PR #96 (TASK-M0-14), `crates/kernel/build.rs:8`:
  `cargo::rustc-check-cfg=cfg(target_arch, values("spirv"))`, listed in the PR as "Applied per R-204 — veto?" item 6.
- **What:** the kernel source is compiled twice, for the host and by rust-gpu for SPIR-V, and refers to
  `target_arch = "spirv"`, a value rustc doesn't know. Without a declaration, `cargo clippy --workspace --all-targets --
  -D warnings` fails at `crates/kernel/src/toolchain.rs:47` ("unexpected cfg condition value: spirv"). Cargo documents
  `cargo::rustc-check-cfg` as setting the expected-cfg list the `unexpected_cfgs` lint checks: the same setting as
  `[lints.rust] unexpected_cfgs = { check-cfg = [...] }`, which is rust-gpu's documented fix. So the code reviewer reads
  it as lint configuration that R-197 reserves for a ruling, whichever file it's in. An item-level
  `#[allow(unexpected_cfgs)]` doesn't silence it (the code reviewer tried it); only a module- or crate-level `#![allow]`
  does, and that is broader.
- **Options seen:**
  1. **Accept the declaration (recommended).** It names exactly one expected value, `spirv`, for `target_arch`, and
     leaves the lint on for every other cfg. Whether it sits in `build.rs` (as in #96) or a `[lints.rust]` table in
     `crates/kernel/Cargo.toml` is the human's choice; the effect is the same.
  2. **A module-level `#![allow(unexpected_cfgs)]` with a reason comment**, which R-197 lets the code reviewer approve
     without a ruling. Broader: it silences every unexpected cfg in that module.
- **Needed:** which one. Blocks PR #96's merge (TASK-M0-14).
