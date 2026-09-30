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

## RQ-178: TASK-M0-11 generates `QuadReduction` in M0; its member order, packing and histogram N are M5's (R-113), and its f16 members have no §3.8 type *(plan, TASK-M0-11, R-113, R-18)*

- **File, section:**
  - `plan/tasks/M0/TASK-M0-11.md` § "Deliverables": "`crates/ledger/src/gen/rust.rs` — the table and `fgw_*` emitters,
    and the `QuadReduction` struct." § "Goal": "Generation-root §3.7's `QuadReduction` member list is transcribed —
    `spread_event` stored as f16 (R-18) and no `ensemble_outcome_agreement`. … `QuadReduction`'s sizing and alignment,
    and the fixed-size-scalars check, are M5's (TASK-M5-01, R-113)." § "Notes": "QuadReduction's sizing (REQ-PAY-089)
    and REQ-PAY-006 move to TASK-M5-01 with its member order, packing and histogram N (REQ-PAY-075, REQ-PAY-077)", and
    "§3.8's vector rule (TASK-M0-07) treats a `u-bits` component at a scalar index as a full u32, so §3.7's
    `class_histogram[N]` (u8 × N) doesn't fit it. This task places it or files it in REVIEW_QUEUE."
  - `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger": "| `class_histogram[N]` |
    u8 × N |", "| `dominant_outcome` | packed | `class ⊕ detail`, 5 bits |", "| `spread_event` | f16 |",
    "| `spread_winner` | 2 bits |", and eleven more f16 members.
  - The same file § "3.8 Metadata schema": "type: u-bits | f32 | f16-pair | fixed16 | vector(type, k)"; "At a packed
    location, `f16-pair` and `fixed16` take exactly 16 bits"; "A field without a complete entry fails generation
    loudly."
  - `decisions.md` § "R-113": "RQ-93, QuadReduction: option (b), sizing at M5." `plan/requirements.yaml` REQ-PAY-075
    (N and bin width, M5) and REQ-PAY-077 ("member order, the packing of its 2-bit and 5-bit members and its final
    aligned size", M5). REQ-PAY-019's verify detail: "Generated payload has spread_event: f16 and no
    ensemble_outcome_agreement".
- **What:** four things stop TASK-M0-11 generating the struct as written without choosing something M5 owns.
  1. `class_histogram: [u8; N]` needs a concrete N, which is REQ-PAY-075's (M5). A const-generic
     `QuadReduction<const N: usize>` avoids choosing N, but `crates/ledger/tests/payload_ledger.rs`'s `check_fields`
     parses each `pub struct NAME {` line of the generated file and compares NAME with `Struct::name`, so a generic
     header fails it. That file isn't among the task's Deliverables.
  2. A §3.8 entry for a member needs a location: a scalar index (the member order) or a word and offset (the packing).
     Both are REQ-PAY-077's (M5), so the task can't write complete entries for the members, and generation refuses an
     incomplete one.
  3. §3.8 has no plain `f16` type. An f16 member fits only as `f16-pair` at a packed 16-bit location, which is packing
     again. Stable Rust (rustc 1.98.1) has no `f16` type either (`error[E0658]: the type f16 is unstable`), so a
     generated struct can't literally declare `spread_event: f16`.
  4. `u8 × N` fits no §3.8 type at a scalar index (the task's note).
- **Options seen:**
  1. **M0 transcribes the member list as ledger data; the struct waits for M5 (recommended).** `payload.rs` lists
     §3.7's members, each with its name and §3.7 type (`u8`, `u8 × N`, 5-bit packed, `f16`, 2-bit, `f32`, `u16`), as
     data that isn't a §3.8 entry and isn't emitted. The REQ-PAY-019 test checks it: `spread_event` is `f16`, and
     there's no `ensemble_outcome_agreement`. The generated struct, the members' §3.8 entries (typed as item 3's
     answer gives) and `class_histogram`'s placement move to TASK-M5-01 along with REQ-PAY-075 and REQ-PAY-077.
     TASK-M0-11's Deliverables line and REQ-PAY-019's verify detail change to say "the ledger's member list".
  2. **Generate a struct with no fixed layout now.** No `repr(C)`; `QuadReduction<const N: usize>` with
     `class_histogram: [u8; N]`; f16 members stored as binary16 bits (`u16`, under a `pub type F16 = u16` alias); the
     5- and 2-bit members as `u8`. The struct check exempts its members until M5 places them. `payload_ledger.rs`'s
     parser has to accept the generic header, so that file joins the task's Deliverables.
  3. **Option 2 without `class_histogram`.** It's left out until M5 sets N, so no generic header and no test change.
     The member list is incomplete in M0.
  - For item 3, whichever option is chosen, M5's entries need either `f16-pair` at packed 16-bit locations (so every
    f16 member is packed) or a new §3.8 type, `f16`, which is a docs change to §3.8 first.
- **Needed:** which option, and for item 3, `f16-pair` packing or a new `f16` type. **Blocks TASK-M0-11.**

---

## RQ-179: `continuation_index` has no value where `next` is `prev`'s inverse *(payload §3, TASK-M0-11, TASK-M0-13)*

- **File, section:** `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`":
  "**Small fixed shader tables:** `inverse(s)`; `continuation_index(prev,s)→{0,1,2}`; …"; "`continuation_index` is
  derived by inverting `cont_symbol` (`continuation_index[prev][next]` = the digit `e` with
  `cont_symbol[e][prev]==next`). **The Rust kernel/host and the WGSL fragment side MUST use this identical generated
  table** — it is frozen in the Rust layout definition (emitted to both targets) and any change is a binary-format
  version change: the table is hashed with the ledger (R-36)". `docs/design/principia_dd_generation_root.md` § "3.3":
  "`continuation_index(prev,s)→{0,1,2}`".
  `plan/tasks/M0/TASK-M0-11.md` § "Goal": "payload §3's frozen continuation table (`inverse = [1,0,3,2]`,
  `cont_symbol[0..2]`, `continuation_index` derived) are ledger entries, emitted to Rust".
- **What:** as a table indexed `[prev][next]`, `continuation_index` has 16 cells. The derivation fills 12 of them. The
  4 cells where `next = inverse(prev)` have no digit, because every `cont_symbol[e]` excludes the inverse (payload §3:
  "no continuation equals `inverse(prev)`"), and the stated codomain `{0,1,2}` has no "none" value. The append never
  reads those cells, because it pops before it pushes. But the table is emitted identically to Rust and WGSL and
  hashed into the schema version, so whatever those cells hold is a frozen value, not an implementation detail.
- **Options seen:**
  1. **The four cells hold 3, which isn't a digit (recommended).** This matches `dmin_pair`, whose code 3 means
     "unset/invalid" (payload §2). The generated Rust `debug_assert!`s that a push never reads 3. Payload §3's codomain
     becomes "`{0,1,2}`, and 3 where `next = inverse(prev)`".
  2. **The four cells hold a digit, 0**, so the codomain stays `{0,1,2}`. A misuse then silently pushes a valid-looking
     digit.
  3. **`continuation_index` isn't a table.** Only `inverse` and `cont_symbol` are emitted, and the push takes the digit
     from a function that searches `cont_symbol`. That function still needs a return value, or a trap, at the inverse,
     which is the same question.
- **Needed:** the value of the four cells, or option 3 with its value. **Blocks TASK-M0-11.** TASK-M0-13's WGSL half has
  to match whatever is chosen.
