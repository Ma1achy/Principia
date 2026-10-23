# TASK-M6-21 — The event-driven precision warning: DECODE_SWITCHOVER and AT_F32_FLOOR in the snapshot

- **Milestone:** M6
- **Closes:** REQ-GUI-010, REQ-GUI-012
- **Depends on:** TASK-M6-08, TASK-M6-26
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-033
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~340 lines

## Goal
The snapshot carries, GUI-sized, whether `DECODE_SWITCHOVER` has fired on visible quads and whether `AT_F32_FLOOR` has been hit, and the Explore view's precision warning is raised by those events (R-54) — never by fixed depth thresholds.

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_scheduler_contract.md` § "Part 4 — Terminal vs refinable, tied to the two floors"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `decisions.md` § "R-54 — The precision warning is event-driven *(GU-3 (b))*"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `docs/gui/design/GUI_DESIGN_NOTES.md` § "01 Explore — the everyday view"
- `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound"
- `decisions.md` § "R-570 — Latent and flat charts have no limit on pan or zoom-out: every latent `z` decodes to a system, so a pan never stops there and R-469's stop never fires for it; zoom-out is limited only by the frame budget and the baseline cover, a frame's cost set by the screen, not by the view's extent; every pixel is a system at any pan or zoom; far out the field goes flat by the decoder's saturation, which is correct and is never clamped, rescaled, wrapped or faded; precision is never a navigation limit *(states what R-407, R-467, R-469 and RQ-409 (a) imply; carries RQ-409 (a)'s "no zoom-out limit" from periodic charts to every chart whose axes are affine)*"

## Deliverables
- `crates/engine/src/contract/snapshot.rs`: two event fields fed from TASK-M6-08's event stream, filtered to visible quads.
- `crates/gui`: the depth readout's precision warning bound to those fields.
- `crates/engine/src/deep/saturation.rs` (new): RQ-553's per-direction test. TASK-M6-08's collapse comparison in `crates/engine/src/deep/switchover.rs` is extended to record, per in-plane direction, which adjacent pairs collapsed and whether each pair's f64 sample points are distinct; `saturation.rs` reads the quad's f64 `J_D` (`crates/engine/src/deep/linearise.rs`, TASK-M5-04 and TASK-M6-07, the same `J_D` the linear uniform carries) and classifies the quad saturated iff every collapsed pair lies along a direction whose `J_D` column is 0 with its points distinct. The result travels as a `saturated` bit on the quad's `DECODE_SWITCHOVER` and `AT_F32_FLOOR` events, and `snapshot.rs`'s two fields count only events without it, so the events, the ladder and the cache facts are unchanged.

## Acceptance tests
- `cargo test -p engine snapshot_precision_events` — snapshot fields for both events exist and flip when the scheduler reports the event on a visible quad (REQ-GUI-010).
- `cargo test -p engine precision_warning_event_driven` — zoom a visible region through the switchover and to the linear floor; the warning event is raised exactly at each event and not at any fixed depth when the events do not fire (e.g. early switchover at shallow depth with tiny q); (a) `z₀ = 10⁴·d` set by SetField on the latent chart at home zoom (default links, `E* = None`) raises no warning while the events fire, and a never-suppressing executor fails; (b) at `z₀ = 10⁴·ê_k` a band quad collapsed along `ê_k` only raises none and a quad also collapsed along the other axis at the genuine floor still warns, an all-columns executor and an any-pair executor each failing; (c) an unsaturated linear-floor quad with bitwise-identical adjacent f64 decodes and `J_D ≠ 0` still warns, the identical-decodes executor failing; (d) an unsaturated linear-floor quad whose f64 sample points coincide, `J_D` reading 0, still warns, an executor without the distinct-points clause failing; each arm asserts its precondition, (b) to (d) on synthetic quad facts (REQ-GUI-012; RQ-553 (a), per direction).

## Notes
- R-390: the Manifold view's depth readout and its precision warning are built on the mock engine by TASK-M6-26, on which this task depends; this task binds the warning to the real engine's event fields.
- **RQ-553 (a), decided per R-369** (23 Oct 2026, by the orchestrator on PR #279, not a human ruling; physics review 5624721489, P4): a saturation collapse, f64 `J_D`'s columns along the view's plane 0 at a quad whose f64 sample points are distinct, raises no precision warning; the genuine floor still does (deep_zoom §4; REQ-GUI-012). The saturation test needs f64 `J_D` at the quad, which the switchover computes (deep_zoom §2 a). *Was:* "`cargo test -p engine precision_warning_event_driven` — zoom a visible region through the switchover and to the linear floor; the warning event is raised exactly at each event and not at any fixed depth when the events do not fire (e.g. early switchover at shallow depth with tiny q) (REQ-GUI-012)."
- **Fix round 3 of PR #279** (physics review 5624781454, P5; gui review 5624788928, G5 to G7; applied per R-369): RQ-553's test is per direction (the bands along the axes covered), the genuine floor is two cases whose controls can fail, the saturated view is set by SetField, and the classification's file and route to the snapshot are named (the `saturation.rs` deliverable). Size ~220 → ~340 lines. *Was (acceptance):* "`cargo test -p engine precision_warning_event_driven` — zoom a visible region through the switchover and to the linear floor; the warning event is raised exactly at each event and not at any fixed depth when the events do not fire (e.g. early switchover at shallow depth with tiny q); a saturated far-field view on the latent chart at home zoom (default links, `E* = None`) raises no warning while the events fire there, and a genuine-floor view with distinct, unsaturated f64 decodes still warns; controls: suppressing on identical f64 decodes drops the genuine-floor warning and fails, and never suppressing warns in the saturated view and fails (REQ-GUI-012; RQ-553 (a))." *Was (Size):* "~220 lines"
