# TASK-M2-31 — Non-finite values in the numeric views: the invalid colour on the CPU twin and in WGSL, and the measured range over finite values (R-436)

- **Milestone:** M2
- **Closes:** REQ-COL-070
- **Depends on:** TASK-M2-03, TASK-M1-09
- **Needs (earlier milestones):** REQ-RENDER-022
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~250 lines, the regenerated views included

## Goal
**R-436 (RQ-284, option (a)):** the descriptor's infinite values at the poles are stored as computed, and every
consumer, CPU and GPU alike, colours a non-finite input as invalid. The numeric field views are consumers: the
generated `rho_ratio` view reads `+∞` at the `α = 0` pole, and `V_0`'s and `E_0`'s read `−∞` at the f64 collision
pole (dd_decoder §3.6). Today the template's guard sends only NaN to `debug_invalid(frag_xy)`: on the CPU twin `+∞`
compacts to raw 1, the top of the ramp, and on the GPU the result is unpinned, since WGSL lets an implementation assume
no infinities; and under `RANGE_AUTO = 1` the measured range takes `−∞` as an end. This task extends the guard to a
stored `±∞` that is no declared sentinel, tested on the value's bits before any arithmetic, and takes the measured range
over the samples whose stored value is finite. A declared sentinel or unset value keeps its own style: `d_min`'s unset
`+∞` stays "not yet" (R-280, R-399), tested first.

## References
- `decisions.md` § "R-436 — The descriptor's `±∞` at the poles are stored as computed; every consumer, CPU and GPU alike, colours a non-finite input as invalid, tested by comparing the stored value, never by arithmetic on `∞` *(closes RQ-284; extends R-79)*"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `decisions.md` § "R-114 — The debug NaN guard is the bitcast test *(closes RQ-82)*"
- `decisions.md` § "R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*"
- `decisions.md` § "R-280 — An unset `d_min` renders in the neutral "not yet" grey *(closes RQ-170)*"
- `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- `crates/ledger/src/gen/numeric.rs`: the guard's bit test widened from the canonical quiet NaN to a non-finite value
  (an exponent field of all ones, `(bits & 0x7F80_0000) == 0x7F80_0000`, NaN included), after a declared sentinel's or
  unset value's own line, before any compaction; its CPU twin (`NumericView::shown`) the same; the module doc citing
  R-436. No `isinf`, no float comparison against the largest finite value and no infinite literal (R-114; the GPU
  determinism note's rule 4).
- The views regenerated (`cargo xtask codegen`), `crates/render/frag/debug/generated/*.wgsl`; no other generated output
  changes.
- The measured range (`u_range`, the host's min/max over the synthetic buffer, TASK-M1-03) taken over the samples whose
  stored value is finite.
- `crates/render/src/colour/field_ramp.rs`'s `FieldRamp` the same, where it reads a stored value.
- The tests that pin `±∞` on the ramp follow the ruling: TASK-M1-09's `numeric_view_template.rs` (`check_unset`'s
  `f32::NEG_INFINITY` "on the ramp") and `field_ramp.rs`; each `M` line listed with the assertion it changes.
- A GPU test or golden case drawing `rho_ratio` at `+∞` and `V_0` and `E_0` at `−∞`.

## Acceptance tests
- `cargo test -p ledger numeric_view_template` — every generated numeric view with no declared sentinel draws `+∞` and `−∞` (f32 bits `0x7F80_0000`, `0xFF80_0000`) invalid on its CPU twin, before any compaction; `d_min`'s unset `+∞` is still drawn "not yet"; the generated WGSL carries the bit test and no infinite literal; control: a guard that tests NaN alone draws `rho_ratio`'s `+∞` at the top of the ramp (REQ-COL-070).
- `cargo test -p render` with a GPU render on gpu-metal and gpu-lavapipe — `rho_ratio` at `+∞` and `V_0` and `E_0` at `−∞` draw the hatch, pixel-identical to a NaN's, on both backends (REQ-COL-070).
- The measured range of `V_0` over a buffer holding `−∞` and finite values is the finite values' `(min, max)` (REQ-COL-070).
- `cargo xtask codegen` leaves the tree clean, and `cargo xtask golden debug-views` and `cargo xtask golden m1-numeric` pass; a golden whose reference changes is re-rendered and its `BASELINES.md` row cites R-436 (REQ-COL-070).

## Notes
- New task, applied per R-369 in R-436's port: TASK-M1-09, which built the template, has merged, so the generator
  change, the regenerated views and the tests that pinned `±∞` on the ramp need their own PR, as R-417 added
  TASK-M2-30.
- The goldens' showcase values are finite (RQ-284's "Waits"), so no existing reference is expected to change; the new
  case is the first to draw `±∞`.
- A qa test that pins `±∞` on the ramp is a ruling-forced change (R-436): qa makes it, and the PR lists it with the
  assertion it changes (R-290, R-369).
- TASK-M2-25's agreement preset needs the same bit test on its operands; a shared WGSL helper in the presentation layer
  is this task's to offer, if it is the first to merge.
