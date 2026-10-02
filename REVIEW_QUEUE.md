# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-196: two choices applied per R-204 in R-360 and R-361: build-kernel in every `xtask-ci` shard, and the display fraction's guard outermost *(R-360, R-361)*

- **File, section:**
  - `decisions.md` § "R-360 — `cargo xtask ci --partition k/n` splits the controls into n = 4 parallel jobs, by a
    stable hash of the control name *(closes RQ-193; amends R-336)*": its "Applied per R-204 — veto?" item.
    `plan/tasks/M0/TASK-M0-45.md` § "Deliverables", the other runners' bullet.
  - `decisions.md` § "R-361 — The generated display fraction is exactly 1 at its endpoint: `select(f32(s)/f32(h), 1.0,
    s == h)` *(closes RQ-195)*": its "Applied per R-204 — veto?" item. `docs/design/principia_dd_simstate_payload.md`
    § "6. Accessors (illustrative of generated output; …)", the comment above `tm_t_end_fraction`.
    `plan/tasks/M0/TASK-M0-15.md` § "Deliverables", the `wgsl.rs` bullet.
- **What:**
  - **A. Applied without asking (R-204), open for a veto:**
    1. R-360: the runners other than `controls` under `--partition`. RQ-193's option 1 ran them "in one shard only".
       Sliced by control name, the kernel's controls fall in every shard, and each reads build-kernel's output, which
       `ci.rs` writes before `controls` runs. So build-kernel runs in every shard, before its controls (16 s of the job
       on `main`'s run of aaa1e40, warm); plan-check, the three lints, gate and golden run in shard 1 only.
       TASK-M0-45 builds it.
       - **Mark:** `decisions.md` — "the runners other than `controls` run in shard 1 of n only"
       - **Mark:** `plan/tasks/M0/TASK-M0-45.md` — "R-360: build-kernel in every shard"
    2. R-361: how the endpoint select composes with the existing `horizon_steps > 0u` guard. The guard stays outermost:
       `select(0.0, select(f32(s) / f32(h), 1.0, s == h), h > 0u)`, so `h = 0` gives 0 even at `s == h == 0`, and
       `s == h > 0` gives exactly 1.0. RQ-195's option 1 placed the select "inside the existing guard", but the
       ruling's words don't settle `h = 0` with `s == h`, where the endpoint select outermost would give 1.0.
       REQ-RENDER-019 ("returning 0 when `horizon_steps` is 0") and payload §6's guard read for this nesting. PR #116
       (TASK-M0-15) builds it.
       - **Mark:** `decisions.md` — "the nesting, R-361's form inside the existing"
       - **Mark:** `docs/design/principia_dd_simstate_payload.md` — "the guard stays outermost"
       - **Mark:** `plan/tasks/M0/TASK-M0-15.md` — "R-361: the nesting"
- **Options seen:**
  1. **Accept A1 and A2 (recommended);** veto either, saying what replaces it.
  2. For A1: build the kernel once, in a job before the 4 shards, and pass its SPIR-V and WGSL to them as an
     artifact; each shard skips build-kernel.
  3. For A2: the endpoint select outermost, `select(select(0.0, f32(s) / f32(h), h > 0u), 1.0, s == h)`, which gives
     1.0 at `s == h == 0` and so amends REQ-RENDER-019.
- **Needed:** accept or veto. Nothing waits on it: TASK-M0-45 builds A1 as written, and PR #116 builds A2.
  When ruled, each mark names the ruling in place of "veto?".

---
