# TASK-M9-40 — The Šuvakov-plane chart, at equal masses and at fixed m₃, defined and built (R-669)

- **Milestone:** M9
- **Closes:** REQ-CHART-091
- **Depends on:** TASK-M2-14, TASK-M4-06, TASK-M4-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~400 lines

## Goal
One M9 task builds the chart five targets wait on (R-669, RQ-608 and RQ-618 (a)): `SuvakovPlane`, over the plane of Šuvakov & Dmitrašinović's initial conditions, at equal masses and at fixed m₃, in RQ-582's labelling (R-630 item 3: the middle body is body 1, at β = 0, Hudomal eq. (18)'s). The chart reference has no section for it, so the task defines it first (R-72), then builds it as a chart of the one chart trait, its decode a tested function. OC-4 on the plane, AT-D1, AT-D3 (TASK-M9-35), HU-1 and HU-2 (TASK-M9-27) run on it.

## References
- `decisions.md` § "R-669 — The owner adopts the recommendations on the fifteen questions of 1 Nov but RQ-656: the Šuvakov-plane chart is one M9 task, its five targets waiting on it whole; a syzygy-symbol field only; N_scramble only; no periodic-orbit continuation, after the SJTU partial check; TR-1, MK-1 and SL-1 stay in M9, not scored, with no chart; the census's CI cadence as § 8.1 and § 8.2 write it, lavapipe in the smoke job; REQ-VAL-199 keeps 256 starts; GG-1's panels by like ε; the drift predicates read the stored latch, live values only for control flow; BZ-1C's starts by Path B, a list-of-starts chart the fallback; Hénon 1974 joins the rotating-frame set; Heggie & Sweatman 1991 in M9, not scored *(closes RQ-593, RQ-595, RQ-596, RQ-597, RQ-608, RQ-617, RQ-618, RQ-620, RQ-636, RQ-647 and RQ-654; amends R-636, R-637 and R-639; refines R-642)*"
- `decisions.md` § "R-636 — The 28 Oct update: the Šuvakov-plane targets stay closures and no chart is built yet; targets past the horizon cap are "not evaluated within cap", never failures, and a raised-cap run is a later task; the weekly consistency audit is adopted, report-only, in the cloud; the session's R-369 decisions stand; #326 merges on the Mac's own GPU check; the five-chart build is held; the Mac keeps GPU work, two local agents at most *(closes RQ-583, RQ-592, RQ-602 and RQ-605; supersedes R-391; amends R-262, R-277 and R-357; refines R-633)*"
- `decisions.md` § "R-630 — The verification plan's §9, every recommendation accepted: PLAN.md is the single plan, its drafts are M3 requirements, the Šuvakov plane labels its middle body body 1, RQ-576 takes option (d), RQ-579 asks the authors, the three fetches go ahead, Li & Liao's pages are recorded only, tier D promotes by name, and RQ-577 is ruled after M3 *(closes RQ-576, RQ-579, RQ-582, RQ-584, RQ-585, RQ-586 and RQ-587; restates PR #309's and PR #303's rulings)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "4.3 The atlas's slice families"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"

## Deliverables
- The chart's section in `docs/design/principia_chart_reference.md`: its axes and ranges, at equal masses and at fixed m₃, what it holds fixed, its labelling, and its decode to the shared decoder `D`; approved by the physics reviewer before any code (R-72).
- The chart in `crates/kernel`, behind the one chart trait, with its descriptor and well-posedness entry (TASK-M2-14).
- Its row in `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set", and its baked compute variant: its entry in the variant key and pipeline table of `crates/engine/src/variants.rs` (TASK-M4-06), since TASK-M9-35 scores on the GPU explorer's f32 output and builds no chart.

## Acceptance tests
- Review checklist (physics reviewer) — the chart's section is approved, its axes, ranges and labelling stated (REQ-CHART-091).
- `cargo test -p kernel suvakov_plane` — each atlas orbit on the plane at equal masses and at each m₃ decodes from its marked pixel to its catalogue initial conditions within the decode's tolerance; at random (u,v), Σpᵢ = 0 to machine precision, I = 1 after the canonical-frame decode, and shape_vec(decode(u,v)) equals the shape the chart's section states for that pixel to ~1e−14; control: a decode with bodies 0 and 1 swapped misses the markers (REQ-CHART-091).
- `cargo test -p engine suvakov_plane_variant` — the chart selects its own baked variant from the pipeline table, and a pan on it changes only uniform buffers; control: a key with the chart left out selects no pipeline (REQ-CHART-091).
- Physics + code reviewers: the chart's row of the lowering appendix checked against the chart registry and the kernel variants (REQ-CHART-091).

## Notes
- Added by R-669 (1 Nov 2026, RQ-608 and RQ-618 (a); the port, 2 Nov 2026). A build task, not a target: it is not scored, its tests block its own merge, and TASK-M9-37 does not wait on it; TASK-M9-27 and TASK-M9-35 do.
- The GPU variant and § 5.2's general tests: code review 5771184172 of PR #408, C6. § 5.2's other tests name other charts. *Was:* Depends on TASK-M2-14 and TASK-M4-17, ~350 lines, the chart in `crates/kernel` only, `suvakov_plane` the one test.
- R-636's "no chart build yet" gives way to this one task (R-669). The atlas data are used under CC BY 4.0, credited to https://www.threebodyorbits.com; `atlas-params.json` never enters the repo.
