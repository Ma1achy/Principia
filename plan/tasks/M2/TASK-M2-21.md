# TASK-M2-21 — Link provenance: the ViewState record, link swap and the link-variation diagnostic

- **Milestone:** M2
- **Closes:** REQ-SYS-011, REQ-CHART-033, REQ-VAL-024, REQ-SYS-083, REQ-SYS-084
- **Depends on:** TASK-M2-05, TASK-M2-15, TASK-M2-20, TASK-M2-41
- **Needs (earlier milestones):** REQ-GEN-008
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~380 lines

## Goal
The exported ViewState records (z₀, q₁, q₂), the chart id and params, the axis warps and the link id per block, so any chart — tilted or not — saves and restores exactly and a figure's provenance pins its links (R-106: the link ids are the chart's per-block registry entries). Decode and encode consume only registry links, a link swap swaps both directions and changes provenance, and the link-variation diagnostic inverts a canonicalised IC under each registry link and reports residual_L and κ_L without ever changing the default link.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Integrity: the link is part of the experiment"
- `docs/contracts/principia_chart_decoder_contract.md` § "The operation table"
- `docs/design/principia_dd_generation_root.md` § "4. Seams (obligations → integration tests)"
- `docs/design/principia_dd_generation_root.md` § "2. Consolidated contract"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Link variation as a conditioning diagnostic (not a default change)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 7 — Ground-truth ingestion (the validation programme's demand)"
- `decisions.md` § "R-106 — The link ids are the chart's link functions *(closes RQ-66)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"

## Deliverables
- The ViewState provenance record in `crates/engine/src/contract/view_state.rs` (serialise / restore).
- `crates/validation/src/link_variation.rs` (the diagnostic; per-IC link choice recorded in provenance).
- Tests `crates/engine/src/contract/tests/view_state.rs`, `crates/kernel/tests/link_swap.rs`.
- `SimConfig`'s `Links { mass, alpha, beta, momenta }`, each a `LinkChoice { id, params }`, written in full at the defaults (R-483 X1, X2, H1); tests `links_filled`, `links_written_in_full`.

## Acceptance tests
- `cargo test -p engine viewstate_roundtrip` — export a tilted view with a non-default link, reload, assert bitwise-identical z per pixel and identical link ids (REQ-SYS-011).
- `cargo test -p kernel link_swap` — property: T2 round-trips via the registry inverses; swapping a link changes the provenance and swaps the decode and encode directions together (REQ-CHART-033; the sim-key change and re-integration are REQ-CHART-052, TASK-M4-08).
- Review (physics): the diagnostic reports residual_L and κ_L per link; provenance records the link used; the default link is unchanged by validation code (REQ-VAL-024).
- `cargo test -p engine links_filled` — REQ-SYS-083's verify.
- `cargo test -p engine links_written_in_full` — REQ-SYS-084's verify.
- REQ-SYS-011's verify gains a non-default link parameter and C6's control, the export with the parameter stripped failing to reproduce (R-482).
- `cargo test -p engine lock_window_undo` — once `SimConfig`'s chart group holds a nonlinear chart's window (`engine::lock::Window`), `engine::lock::set_on_chart`'s window `SetField` joins the lock's command under the same history key, and one undo restores z₀, the lock group and the window (R-69; gui_state_contract §2, R-466; REQ-SYS-011; gui review 5642898583 of PR #290, G5, applied per R-369).

## Notes
- RQ-95 ruled: R-113 — REQ-CHART-033 is split: this task asserts registry-only links, provenance and the two-direction swap; "a link swap changes the sim key, recompiles and re-integrates from t = 0" is REQ-CHART-052 (TASK-M4-08). This settles Gap G23.
- R-483 H2 (20 Oct 2026): the ViewState record is `SimConfig`'s serialisation, not a copy of the links.
- qa files that must change, in this task's qa commit and only there (R-290, forced by R-483 X1; decided per R-369, R-483 H10): `crates/engine/tests/qa_TASK-M0-18.rs` (`Links {}` at :34; the texts at :59 and :116), `crates/engine/tests/qa_TASK-M6-28.rs` (:45), `crates/engine/tests/qa_TASK-M2-22.rs` (:111), `crates/engine/tests/qa_TASK-M6-24.rs` (:54), `crates/engine/tests/qa_TASK-M1-11.rs` (:294), `crates/engine/tests/qa_TASK-M6-29.rs` (:52), `crates/gui/tests/qa_TASK-M6-26.rs` (:1157) and `crates/gui/tests/qa_TASK-M6-24.rs` (:91). The PR lists each `M` line with its reason and the code reviewer confirms no other assertion is weakened. The implementer's own: `crates/engine/src/contract/sim_config.rs`, its `tests/canonical.rs` and `tests/conformance.rs`, `crates/engine/tests/nav.rs`, `crates/engine/tests/bringup.rs`, `crates/prin/tests/profile.rs` (:141, :407), `crates/prin/src/profile/run.rs`, `crates/gui/src/mock/mod.rs`, `crates/gui/src/tests/support.rs` and `crates/validation/src/decode_f32_eps.rs`. The lists are a survey; the qa reviewer confirms them against the build.
- R-484's T2 fires on `Links {}` until this task fills it; T2 is TASK-M8-53's, after this task (R-484 J9).
- **The nonlinear lock's window (code review 5642862326 of PR #290, C4; applied per R-369):** a nonlinear chart's view window (`engine::lock::Window`, its centre and half-widths) is chart construction among "chart id and params", which this task holds in `SimConfig`'s chart group (REQ-SYS-011; chart_decoder_contract § "The lock (projective microscope)", "The lock's construction, defined"). Once the chart group holds it, `engine::lock::set_on_chart`'s re-centred window joins the lock's one command as a `SetField`, so one undo restores it with `z₀` and the lock group (R-69; gui_state_contract §2, R-466). Until then TASK-M2-23's lock returns the window beside its edits.
