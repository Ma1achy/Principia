# TASK-M1-12 — Derived, live-shape, accumulator, word and ICDescriptor views

- **Milestone:** M1
- **Closes:** REQ-TOOL-010, REQ-TOOL-024, REQ-TOOL-025, REQ-VAL-010, REQ-VAL-011, REQ-VAL-122, REQ-TOOL-154, REQ-TOOL-155, REQ-TOOL-156, REQ-TOOL-157
- **Depends on:** TASK-M1-02, TASK-M1-09, TASK-M1-11
- **Needs (earlier milestones):** REQ-PAY-001, REQ-PAY-010, REQ-GEN-006
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** PIT-3
- **Size:** ~550 lines

## Goal
The fragment-computed views exist over the synthetic payload: live shape views (`u_mode` 0: ½(n+1) direction cosines of the current derived n; 1: Twilight of θ̃; 2: |n|−1 error), accumulator views (running S/t, the Welford slope C_ty/C_tt, drift running-max vs final), derived views (orbit_count / retrograde from θ̃, the reduced crossing count, finalised ftle, current drift H(r,p) − E₀), the word inspector (reduced length invalid-styled when truncated, symbol-at-k slider, truncated flag, whole-word hash) and the ICDescriptor views. `n` joins the ledger as generation-root §3.8's worked entry (`derived`, `vector(f32, 3)`), so its occupants are generated: for every vector field, `n` and the k = 6 fields `r` and `p` alike, they offer '‖·‖ as scalar' beside 'as direction-cosines', with a test asserting ‖n‖ = 1 to a calibrated tolerance (RQ-235). The four renderings the corpus does not give — the ternary masses colour, the whole-word hash of a `uint4`, the invalid-styled reduced length (RQ-236) and the colour of a k ≠ 3 vector's direction cosines (RQ-235 as amended per code review 5438179638) — are written here as R-72 definitions. The t_dmin round-trip view certifies the 16-bit step index.

## References
- `docs/contracts/principia_render_contract.md` § "Live-state & array inspection"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_debug_tooling_plan.md` § "C. Payload field views — `times` (u32), f16-packed scalars & `free_group_word` (separate buffer)"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/design/principia_debug_tooling_plan.md` § "E. Payload field views — `ICDescriptor` (64 B) & the live-state block"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*"
- `decisions.md` § "R-153 — The debug and live-march views have golden images of their own *(closes RQ-123)*"
- `decisions.md` § "R-158 — The six readings are accepted *(closes RQ-128)*"
- `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)"
- `decisions.md` § "R-389 — `θ̃` starts at 0, and below a pole radius `r_pole` it holds with a frozen reference, adding the wrapped exit-minus-entry longitude on exit *(closes RQ-223)*"
- `decisions.md` § "R-392 — An IC that starts inside `θ̃`'s pole radius adds no delta at its first exit; `θ̃` counts from the exit longitude *(closes RQ-225)*"
- `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug view)"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_chart_reference.md` § "3.1 Forward map (already implemented as `shape_vec`)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-136 — `debug_invalid(frag_xy)` draws the hatch; sentinels show their value *(closes RQ-114)*"
- `decisions.md` § "R-245 — An invalid diffusion fit reads NaN, by a validity predicate; no −1.0 sentinel *(amends R-17, R-136; closes RQ-152)*"
- `decisions.md` § "R-369 — Standing rule on autonomy: no size gate; decide and continue; ask the human only for the five kinds listed *(supersedes R-234 and R-367; amends R-175, R-204, R-208, R-211, R-264, R-283, R-290 and R-357)*"

## Deliverables
- `crates/ledger`: `n` in `ledger::payload` as §3.8's worked entry (`derived(from: [r, m0, m1, m2])`, `vector(f32, 3)`, lin per component, [−1, 1] per component); the read side generates its WGSL from the same source as `kernel::shape`, with a twin test, as it does `hamiltonian`; the catalogue (TASK-M1-08's generator) offers both reductions, '‖·‖ as scalar' and 'as direction-cosines', for every `vector(type, k)` (generation-root §3.8): for k = 3 the direction cosines draw as ½(v̂ + 1) in RGB, and for k ≠ 3 (`r`, `p`) by REQ-TOOL-157's definition (RQ-235, as amended per code review 5438179638).
- `crates/render/shaders/wgsl/frag/debug/` (hand-written, registry-scanned): live-shape, accumulator, derived and word-inspector views. The Hamiltonian is the generated read side's `hamiltonian` (TASK-M1-01), not a new helper.
- Generated ICDescriptor views (ternary masses, per-scale scalars) via the catalogue (TASK-M1-08).
- The three R-72 definitions (RQ-236), each written into the render contract and approved by the physics reviewer: the ternary masses colour, `(m0, m1, m2)` as linear RGB scaled by `1/max(mᵢ)`, in Part 5 (REQ-TOOL-154; if TASK-M1-08 generated a combined masses view, that view's rendering is the one written); the whole-word hash, `h = pcg(w ^ pcg(z ^ pcg(y ^ pcg(x))))` by `dbg_pcg` then `dbg_hash_u32`'s byte-to-RGB step, beside `dbg_hash_u32` (REQ-TOOL-155); the invalid-styled reduced length, `debug_invalid(frag_xy)` where `fgw_reduced_length_valid` is false, in Part 6's word views, the raw `length` view still showing 127 literally (REQ-TOOL-156). A fourth, from RQ-235 as amended per code review 5438179638: the colour rendering of a k ≠ 3 vector field's direction cosines (`r` and `p`, `vector(f32, 6)`), in Part 5 (REQ-TOOL-157).
- The VAL-122 proposal: ‖n‖ − 1 measured over a test render at f32, with the stated tolerance, attached to the PR.
- Golden fixtures `fixtures/golden/debug-views/` (RQ-237), as RQ-229's harness cases, one per view in the registry's `category: debug` entries at this task's merge; each reference recorded in the PR with a `BASELINES.md` row citing R-153 as "proposed, confirmed at the M1 gate", as R-376 confirmed M0's rows.

## Acceptance tests
- `cargo xtask golden debug-views` — each view in the registry's `category: debug` entries renders on a synthetic payload and matches its own golden image, its row proposed in this PR and confirmed at the M1 gate (R-153); the |n|−1 view is flat zero on the synthetic payload (REQ-TOOL-010; the real-march check and the live effort heatmap are REQ-TOOL-132, TASK-M3-22).
- `cargo test -p render derived_views_match_cpu` — orbit_count/retrograde, reduced crossing count, finalised ftle and current drift, each computed in the fragment, match a CPU reference on synthetic states: the integers and booleans exactly, with synthetic θ̃ kept away from multiples of 2π so `floor` is stable; `ftle` and the drift within the derived WGSL-accuracy bound of `crates/kernel/tests/derived.rs`, which is not a calibration; rendered through RQ-229's harness (REQ-TOOL-024).
- `cargo test -p render word_hash_and_symbol_at_k` — distinct synthetic words hash to distinct colours by REQ-TOOL-155's fold; symbol-at-k matches the appended sequence (REQ-TOOL-025).
- `cargo test -p render norm_n_views` — the registry lists both views, '‖·‖ as scalar' and 'as direction-cosines', for every vector field (`n`, `r` and `p`); ‖n‖ − 1 is within tolerance: REQ-VAL-122 (calibrated) over a test render (REQ-VAL-010).
- `cargo test -p ledger catalogue_equals_ledger` — TASK-M1-08's check, re-run: the catalogue's field list, `n` included, equals `ledger::layout().entries` (REQ-VAL-010).
- `cargo test -p ledger t_dmin_roundtrip` — pack/unpack `t_dmin_step` round-trips exactly for all u16 values (REQ-VAL-011).
- Review checklist (physics): the proposal measures ‖n‖ − 1 over a test render at f32 and states the tolerance; the human confirms it at the M1 gate and it is recorded in decisions.md (REQ-VAL-122).
- Definition (physics): render contract Part 5 gives the ternary masses colour, and the masses view draws equal masses white and each vertex its primary (REQ-TOOL-154).
- Definition (physics): render contract Part 5 gives the whole-word fold beside `dbg_hash_u32`, and the word-hash view draws it (REQ-TOOL-155).
- Definition (physics): render contract Part 6's word views give the invalid-styled reduced length; a truncated synthetic word's reduced-length view draws the hatch and its raw length view draws 127 on the ramp (R-136) (REQ-TOOL-156).
- Definition (physics): render contract Part 5 gives the colour rendering of a k ≠ 3 vector field's direction cosines (vᵢ/‖v‖), and the direction-cosines views of `r` and `p` draw it on synthetic states (REQ-TOOL-157).

## Notes
- REQ-VAL-122 is a calibration (R-71); REQ-VAL-010's acceptance uses the proposed value until the human confirms it.
- Not available at M1 (milestone Gaps): no artboard shows the debug views, so R-153 gives them golden images of their own, recorded at the gate. The ledger knows n is a vector (gui_state_contract §4) through generation-root §3.8's `vector(type, k)` and `n`'s worked entry, which this task adds to the ledger's code (RQ-235).
- REQ-TOOL-025's "distinct colours" is asserted over a fixed fixture set of words.
- RQ-229, RQ-235 to RQ-237 and RQ-241, decided per R-369 (7 Oct 2026): the harness case kind (RQ-229); `n` in the ledger, and both reductions for every vector field with the k ≠ 3 direction-cosines colour a definition, per code review 5438179638 (RQ-235); the three definitions (RQ-236); the suite, its references, the picker's scope and the comparison bound (RQ-237); the References (RQ-241). TASK-M1-10 and TASK-M1-13 add their own cases to `debug-views`.
- RQ-93 ruled: R-113 — the screenshot runner is TASK-M0-20's (split from TASK-M0-06 by R-183).
- RQ-94 ruled: R-113 — REQ-TOOL-010 renders every view on a synthetic payload at M1; the real-march |n|−1 check and the live effort heatmap are a new M3 requirement (REQ-TOOL-132, TASK-M3-22).
- RQ-101 ruled: R-122 — the Twilight data for the θ̃ view is the published matplotlib table (the prelude's, TASK-M1-03).
- RQ-123 ruled: R-153 — the debug views are checked against golden images of their own, recorded at the M1 gate, not against artboards (REQ-TOOL-010). RQ-237 reads "recorded at the gate" as M0's rows were read: the reference is recorded in the PR and its row confirmed at the gate (R-376's precedent).
- RQ-128 ruled: R-158 — the readings taken while applying R-141 to R-156 are accepted.
