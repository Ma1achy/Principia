# TASK-M8-74 — Result, the job: one job.json run in the app or by `prin batch`, f32 or f64, the result folder and its payload, cancel and resume (R-572)

- **Milestone:** M8
- **Closes:** REQ-SYS-087, REQ-SYS-088, REQ-SYS-089, REQ-SYS-090, REQ-TOOL-196
- **Depends on:** TASK-M8-02, TASK-M4-17, TASK-M7-44
- **Needs (earlier milestones):** REQ-SYS-027, REQ-TOOL-181
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** PIT-3
- **Size:** ~600 lines, the tests included

## Goal
A Result runs a view's region at full precision, end to end, as one batch job (R-572): one `job.json` (`SimConfig` and `RenderState` in their canonical serialisation, a `run` block, the build hash) that the app runs in the background and `prin batch --job job.json` runs with the same code; f64 by default (the shared kernel on CPU threads) or f32 (the survey kernel on the GPU, TASK-M4-17's uniform path), nothing else (R-265); a uniform `nx × ny` grid with refinement off and the view's integrator occupant. Rows are pure and take their cell width from the global grid, so Cancel keeps what finished and Resume continues bitwise. It writes the result folder: `result.json` with failed and undetermined pixels counted, `payload.prin` at each stored time (the horizon plus the user's playheads), f64 wide at both precisions, the nominal copy with the reductions or all `E + 1` copies, `image.png` with its tier-1 `run` block, and `log.txt`. The payload's field list and layout are a definition this task writes (R-72).

## References
- `docs/design/principia_results.md` § "3.2 Where it runs: the job"
- `docs/design/principia_results.md` § "3.3 Output: the result folder"
- `docs/design/principia_results.md` § "4. Fit with rulings"
- `docs/design/principia_results.md` § "6. Tests"
- `docs/read_first/principia_00_philosophy.md` § "7.2 Tiled prebake"
- `docs/read_first/principia_00_philosophy.md` § "7.3 Dump tiers — and the default is not an image"
- `docs/design/principia_dd_image_embedding.md` § "6. What travels"
- `decisions.md` § "R-572 — Result: a view's region run at full precision, f32 or f64 only, on a uniform grid, as a batch job in the app or by `prin batch`, written to a result folder that reopens as a view drawn from what was stored; the user chooses the stored times, and one copy per pixel or all `E+1` copies is an option *(amends REQ-SYS-007 and philosophy §2; unparks philosophy §7.3's reduced and per-copy tiers as the Result's payload)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- `crates/engine/src/result/` — the job (`job.json` read and written in the canonical serialisation), the row scheduler (rows independent, cell width from the global grid, lower priority than the frame loop, REQ-SCHED-043), cancel and resume from the rows finished, and the writer of the result folder.
- `crates/prin/src/cmd/batch.rs` — `prin batch --job job.json`, calling the same engine code.
- `payload.prin`'s field list and layout written into `principia_results.md` § 3.3 (REQ-SYS-090), physics-reviewed.
- The `run` block in `crates/engine/src/contract/tiers.rs`, tier 1.

## Acceptance tests
- `cargo test -p engine result_f64_bitwise` — the same f64 job on 1 and on 4 threads, rows in shuffled order, gives a bitwise-equal payload.prin; control: a reduction accumulated in row-completion order differs under the shuffle and fails; a precision other than f32 or f64 is refused (REQ-SYS-087).
- `cargo test -p prin result_app_and_cli` — the in-app job and `prin batch --job job.json` give byte-identical payload.prin files; control: a second code path that rounds a field differently fails (REQ-SYS-087).
- `cargo test -p engine result_resume` — a job cancelled after a third of its rows, then resumed, is bitwise the uninterrupted run, and the cancelled folder's result.json reads its share done; control: a row taking its cell width from its own extent offsets its samples by a half cell and fails (REQ-SYS-088).
- `cargo test -p engine result_payload_complete` — a 16 × 16 job with one pixel injected to run out of its step budget holds 256 records at each stored time, that pixel's present with its reason and counted in result.json; control: a writer that filters failed pixels holds 255 and fails (REQ-SYS-089).
- `cargo test -p engine result_payload_widths` — an f32 and an f64 job write the same field list at the same widths; control: a width following the precision fails (REQ-SYS-089).
- `cargo test -p engine result_stored_times` — with playheads 10 and 30 added at T = 50 the payload holds exactly t = 10, 30 and 50, and with all copies (E + 1)·nx·ny records per time; control: a writer storing the horizon only fails (REQ-SYS-089).
- Review checklist (physics reviewer) — `principia_results.md` § 3.3 lists every payload field with its type and source (the SimState payload, R-135's reduction fields), the file's layout and schema version, and every payload input of the stain library is present (REQ-SYS-090).
- `cargo test -p engine result_image_run_block` — a result's image squeezed to tier 1 only still carries its run block and imports as the identity on SimConfig and RenderState; control: a run block assigned to tier 2 is dropped at that size and fails; the tier table stays exhaustive (REQ-TOOL-196).

## Notes
- R-572 (21 Oct 2026). REQ-SYS-007 is amended by it: §7.3's reduced tier and its per-copy tier are unparked for this payload only; §7.2's VRAM tiling stays parked.
- The pilot estimate and the window are TASK-M8-75's.
