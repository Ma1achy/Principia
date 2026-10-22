# TASK-M7-55 — Print graph determinism, units and stability: seeded streams, `ref_px` / `device_px` / `fraction`, the screen-anchored sheet and `boil`, with the §11 harnesses 3, 4 and 5 (R-490)

- **Milestone:** M7
- **Closes:** REQ-RENDER-109, REQ-RENDER-111, REQ-RENDER-112, REQ-RENDER-113
- **Depends on:** TASK-M7-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~600 lines, the tests included

## Goal
What every node is ported against: seeded random streams only (a `seed` parameter or a wired stream; never the clock, the frame counter or a platform random); lengths scaled by unit at render time (`ref_px` by `output_width / 1024`, `fraction` by the figure width, `device_px` as is); the sheet anchored to the screen and marks seeded from screen-anchored coordinates and the graph's seed; `boil` a graph setting, off by default. The three harnesses (§11 tests 3, 4 and 5) run on fixture nodes here and on each category in its port task. The scale tolerance is proposed here (REQ-RENDER-112). At render_scale ≠ 1, `device_px` and the anchoring wait on RQ-538 (TASK-M7-69); everything here is tested at render_scale 1.

## References
- `docs/design/principia_print_graph.md` § "6.1 Determinism"
- `docs/design/principia_print_graph.md` § "6.2 Units: a medium must look the same at every export size"
- `docs/design/principia_print_graph.md` § "6.3 Anchoring and stability under motion and playback"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `decisions.md` § "R-490 — The Print graph is deterministic, seeded only; every length is in `ref_px`, `device_px` or `fraction`; the sheet is anchored to the screen; `boil` is off by default"

## Deliverables
- `crates/render/src/print/units.rs`, `rng.rs` (seeded streams, screen-anchored seeding), `boil.rs`.
- `crates/render/tests/print_harness/` — the determinism, scale-invariance and stability harnesses, reusable by the port tasks.
- Tests `print_determinism`, `print_scale_invariance`, `print_stability`; golden suite `print-determinism`.

## Acceptance tests
- `cargo test -p render print_determinism` and `cargo xtask golden print-determinism` — REQ-RENDER-109's verify (the same graph rendered twice gives the same bytes; control: changing only `seed` changes the bytes, a fixture node that reads the frame counter fails a two-frame comparison of an unchanged figure, a fixture node that reads the wall clock fails a two-render comparison taken 10 ms apart, and a fixture node that draws from the platform random (the OS generator) fails a two-render comparison; across backends, the golden suite renders a heavy preset and a slot-chain graph with native wgpu on Metal (the human's Mac, R-186) and on lavapipe (CI), each against the one baseline in `fixtures/golden/print/` within REQ-VAL-138's cross-backend tolerance as ruled (R-110, R-269); control: a fixture node whose draw is seeded from the adapter's name exceeds that tolerance on one backend and fails).
- `cargo test -p render print_scale_invariance` — REQ-RENDER-111's verify (the harness renders each registered `ref_px` and `fraction` node at 512² and at 2048² downsampled and compares within REQ-RENDER-112; the `device_px` exemption list is printed and equals the registry's; control: a fixture node deliberately left in `device_px` but not listed fails).
- Calibration: the PR proposes the metric and threshold with the measured spread over the Analyse and Generate nodes and the fixture that must fail; a reviewer checks it; the human confirms it at the M7 gate (REQ-RENDER-112).
- `cargo test -p render print_stability` — REQ-RENDER-113's verify (at render_scale 1, two frames with an unchanged figure give identical output for every `temporally_stable` node, and a sheet-only graph's output is unchanged by a pan of the figure; under `boil: every N` (N = 3, a test value) the frames within one window of N frames are identical and the frames either side of a window boundary differ; a default Print graph, and a fresh `RenderState`'s, has `boil` off; a graph with `boil: every N` written to `principia.print/1` and read back keeps `boil` and its N; controls: `boil: per frame` differs between the two frames, a fixture sheet seeded from figure coordinates moves under the pan and fails, `every N` run as per frame differs within a window and fails, a default with `boil` on fails the default arm, and a writer that drops `boil` fails the round trip).

## Notes
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
- **Fix round 8 of PR #281 (qa review 5608745757, findings 2, 3, applied per R-369):** the acceptance lines follow REQ-RENDER-109's and REQ-RENDER-113's new verifies; a golden-image verify runs under `cargo xtask golden <suite>` with its baselines in `fixtures/golden/print/` (R-146, R-110). *Was:* "Tests `print_determinism`, `print_scale_invariance`, `print_stability`."; "`cargo test -p render print_determinism` — REQ-RENDER-109's verify (the same graph rendered twice gives the same bytes; control: changing only `seed` changes the bytes, and a fixture node that reads the frame counter fails a two-frame comparison of an unchanged figure)."; "`cargo test -p render print_stability` — REQ-RENDER-113's verify (at render_scale 1, two frames with an unchanged figure give identical output for every `temporally_stable` node, and a sheet-only graph's output is unchanged by a pan of the figure; controls: `boil: per frame` differs between the two frames, and a fixture sheet seeded from figure coordinates moves under the pan and fails).".
