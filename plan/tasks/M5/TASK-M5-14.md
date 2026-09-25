# TASK-M5-14 — Dispatch queue: epochs, in-flight limit, positional readback and the measurement path

- **Milestone:** M5
- **Closes:** REQ-SCHED-033, REQ-SCHED-021, REQ-SCHED-022, REQ-PAY-066, REQ-SCHED-031, REQ-SCHED-087
- **Depends on:** TASK-M5-04, TASK-M5-09
- **Needs (earlier milestones):** REQ-PERF-010, REQ-PERF-012, REQ-SYS-021, REQ-SCHED-003
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The engine dispatches quads through a queue that keeps 2–4 jobs in flight, skips subsequent passes for quads no
longer visible and tags every job with an epoch, so a result for a superseded epoch is dropped or filed as flotsam (the
first eviction victim) and never painted. The `QuadReduction` readback is fire-and-forget, one in flight, latest wins,
indexed positionally by the quads requested; `onSubmittedWorkDone` is never awaited in the loop. The measurement path
dispatches a uniform grid with `FULL_RETENTION` set, no adaptive subdivision, and returns every per-sample result.

## References
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- `docs/design/principia_dd_generation_root.md` § "Identity"
- `docs/contracts/principia_scheduler_contract.md` § "`QuadRequest.flags`"
- `docs/contracts/principia_scheduler_contract.md` § "Part 2 — Refinement density is not probability density (enforced here, because this is where it would break)"
- `docs/gui/principia_render_gui_spec.md` § "G10. Measure — a tool on the figure (`10_measure.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "10 Measure — a tool on the figure"
- `docs/gui/principia_render_gui_spec.md` § "G14. Settled by the notes (record)"
- `decisions.md` § "R-39 — `FULL_RETENTION` keeps bit 4, owned by the measurement path *(PL-4, amended)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/engine/src/dispatch/{queue.rs, epoch.rs, readback.rs, measure.rs}`.
- Tests `crates/engine/tests/dispatch.rs`: pan away mid-dispatch; in-flight bound; positional readback of k quads;
  Measure dispatch sets bit 4 and returns N² results per quad, interactive dispatches leave it clear.

## Acceptance tests
- `cargo test -p engine dispatch_epoch_inflight` — pan away mid-dispatch; the returned result carries an old epoch and is not written to the cache; in-flight count never exceeds the configured limit in [2,4] (REQ-SCHED-033).
- `cargo test -p engine stale_epoch_flotsam` — stale-epoch job result is not composited; flotsam is the first eviction victim (REQ-SCHED-021).
- Review checklist (code) — grep the loop for awaited map/onSubmittedWorkDone (REQ-SCHED-022).
- `cargo test -p engine readback_positional` — readback of k requested quads maps to them by position (REQ-PAY-066).
- `cargo test -p engine measure_full_retention` — a Measure-tool dispatch sets bit 4, uses no adaptive subdivision, and returns N² per-sample results per quad; interactive dispatches leave bit 4 clear (REQ-SCHED-031).
- Proposal: the in-flight job limit in [2, 4] with its evidence (throughput and pan-away latency at each candidate); the human confirms it at the M5 gate (REQ-SCHED-087).

## Notes
- The in-flight limit is a range (2–4) in scheduler Part 6; the value within it is not given (see Gaps).
- Closes, for gaps the corpus leaves open: REQ-SCHED-087 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
