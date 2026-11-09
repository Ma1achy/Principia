# TASK-M2-57 — The decoder's mirror step removed, latent β over the full circle, under the owner's bitwise check (R-865) ⚑

- **Milestone:** M2
- **Closes:** REQ-DEC-055, REQ-DEC-056
- **Depends on:** TASK-M2-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-865, on RQ-739: the decoder's mirror step goes, and latent `β` runs over the full circle, so every chart, the shape
sphere included, uses one decode path with no special case, and a lower-half state encodes to `β ∈ (π, 2π)` in place
of a mirrored state. It is conditional on the owner's bitwise check: every existing preset, saved chart and golden with
`β ∈ [0, π]` reproduces unchanged through the new decoder. If anything doesn't, the task stops and brings the list back
to the owner before proceeding. **It waits on RQ-750:** `β = π·σ(z_β)` already covers `(0, π)`, so any full-circle
link changes every stored latent `z`, and the owner chooses the way out before the link is defined or anything is built.

## References
- `decisions.md` § "R-865 — The decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the bitwise check: every preset, saved chart and golden with `β ∈ [0, π]` reproduces unchanged, and if anything doesn't, the list comes back to the owner before the removal proceeds *(on RQ-739, which stays open)*"
- `decisions.md` § "R-854 — Symmetry reductions apply only to how a starting state is described at `t = 0`: trajectories evolve and are shown in full physical space, nothing is folded during a run, and the only operation besides the step is the per-step centre-of-mass projection, energy and `L_z` monitored, never corrected"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/design/principia_lines.md` § "8. Traces"
- `decisions.md` § "R-898 — Latent `β` becomes a periodic angle over the full circle; every stored latent view, saved chart and golden is converted once, the bitwise check replaced for this change by a conversion test; the saved-file format is versioned, old files converted on load; not a hidden half-flag *(closes RQ-750; amends R-865)*"

## Deliverables
- **First, on its own commit, before the decoder changes:** the baseline, every existing preset, saved chart and golden
  with `β ∈ [0, π]` recorded through today's decoder (IC words, sim key and, for a golden, its image hash), the PR
  listing each.
- The full-circle link and the signed encode, defined in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2 (R-72;
  REQ-DEC-056), under physics review: the link and its inverse, its seam, how a syzygy crossing reads on a latent chart,
  why a `β ∈ [0, π]` start keeps its bits, and the pole value at `λ̃ = 0`.
- The decoder without its mirror step, in `crates/kernel` (its shared CPU/GPU source, the generated WGSL regenerated
  from it), the encode to `β ∈ (π, 2π)` for a lower-half state, and the shape sphere through the one decode path.
- The sections marked "[mirror-step removal, ruled 8 Nov 2026 (R-865 …)]" rewritten to the built decoder, each keeping
  its old text as a "Was": canonical_spec § 3, chart_decoder_contract Part 2 and Part 5, inverse_encode_contract Part 1,
  chart_reference § 0.4 and § 3, dd_decoder § 3.3, dd_encode § 3.2, and principia_lines.md § 8's `z_β` row; and
  chart_composition's markers (§ 6.6's list) read with it.
- In the PR: every stored `z`, IC save file or test that relied on the old mirrored encode, and the effect on default
  windows of twice the area in `β` (the proposal's items 4 and 5).

## Acceptance tests
- Before the decoder changes, on its own commit: the baseline of every preset, saved chart and golden with
  `β ∈ [0, π]`, recorded through today's decoder (IC words, sim key and, for a golden, its image hash), the PR listing
  each (REQ-DEC-055).
- `cargo test -p kernel mirror_step_removed` — a lower-half state encodes to `β ∈ (π, 2π)` and decodes back to itself,
  not its mirror image; a latent-chart trace through a syzygy at `β = 0` and at `β = π` continuous; controls: the mirror
  step left in fails, a σ-type link that jumps at `β = 0` fails (REQ-DEC-055).
- `cargo test -p kernel latent_conversion` — every converted `z` decodes to the same system as its old `z` to rounding
  precision; a saved file of the old version converted on load, one of an unknown kind refused by `Saved::from_words`;
  the converted words marked with a new kind value; control: a value converted by the wrong link fails (R-898;
  REQ-DEC-055).
- `cargo test -p gui mirror_step_presets` and `cargo test -p engine mirror_step_presets` — every chart preset each crate
  holds (the gui's in `crates/gui/src/explore/manifold_view/chart.rs`), converted, against the baseline by the
  conversion test (R-898; REQ-DEC-055).
- `cargo test -p kernel saved_chart` — the saved charts, converted, against the baseline by the conversion test (R-898;
  REQ-DEC-055).
- `cargo test -p render --test debug_views`, `cargo test -p render --test qa_TASK-M2-32`, `cargo test -p validation --test
  decode_preset_golden` and `cargo test -p validation --test qa_TASK-M2-28` — the golden suites re-run with every
  golden's outcome classes identical and its regenerated hash recorded with the reason, in the GPU jobs that run them
  (R-898; REQ-DEC-055).
- `cargo test -p kernel --test roundtrip` — with the assertions the ruling changes: T1 as the equivariance `E(Π·x)` = the
  mirrored `z` (`β → 2π − β`, the momentum controls reflected, `L_z` negated) in place of `E(Π·x) = E(x)`, and
  `t3_half_mirror` removed under REQ-ENC-005's change (REQ-DEC-055).
- Any preset, saved chart or golden whose converted value doesn't decode to the same system to rounding precision, or
  whose outcome classes change, stops the task: the list goes to the owner (REQ-DEC-055).
- Review: the full-circle link and the signed encode in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2, written to the
  owner's answer to RQ-750 (R-898: `z_β` a periodic angle, each stored value converted once) and approved by the physics
  reviewer, with whether a converted `z_β` is stored reduced to one period or unreduced and where a wrap enters (R-72;
  REQ-DEC-056).

## Notes
- Added by R-865 (8 Nov 2026), the owner's reply 1 to the code map of that day, on RQ-739 (open on its item 3, the
  optional "mirror pairs counted once" statistic, which this task doesn't build).
- **Waits on RQ-750** (`rq: RQ-750` on REQ-DEC-055 and REQ-DEC-056): physics review 5817348709 of PR #449, P1. Not
  dispatched until it is ruled.
- **Requirements that change when this merges (R-865; physics review 5817348709, P2).** Each carries a note now; this
  task rewrites their statements to the built decoder, each keeping a "Was": REQ-ENC-003 (T1, to the equivariance
  above), REQ-ENC-005, REQ-ENC-014, REQ-ENC-007, REQ-ENC-002, REQ-ENC-015, REQ-DEC-002, REQ-DEC-016, REQ-DEC-017,
  REQ-DEC-018, REQ-ENC-034, REQ-CHART-002, REQ-CHART-020 and REQ-COL-006. **REQ-VAL-022** is a numerical gate read on
  `C(x)` "where C's mirror puts L⁻ at w = +1": its restatement is proposed here under physics review and recorded by a
  ruling before this task merges, never changed silently in its PR.
- **qa's files (R-290, R-237; the exception a ruling forces, R-369).** The qa files that assert the mirror change in this
  task's qa commit and only there, the PR listing each `M` line with its reason and the code reviewer confirming no
  other assertion is weakened: `crates/kernel/tests/qa_TASK-M2-07.rs`, `qa_TASK-M2-07_edge.rs` and `qa_TASK-M2-15.rs`
  among them, with every other qa file the build shows asserting the mirror. `crates/kernel/tests/roundtrip.rs` is
  TASK-M2-16's, the implementer's to change.
- **The condition is a stop, not a gate to tune.** If any preset, saved chart or golden with `β ∈ [0, π]` doesn't
  reproduce, the task stops before the removal merges and the list goes to the owner (the orchestrator files it on the
  review queue). Nothing is re-baselined, loosened or special-cased to pass.
- **Traces at syzygies.** The owner's words say this also fixes the latent-chart traces' jump at syzygies. Under a σ-type
  link only the syzygies at `β = π` become continuous, one at the seam `β = 0` still jumping unless `z_β` is periodic
  (RQ-750, which corrects RQ-739's 7 Nov note); a trace that still jumps goes on the list.
- **Not here:** the proposal's optional "mirror pairs counted once" statistic (its item 3) is not ruled, and no task
  builds it: RQ-739 stays open on it.
- **Applied per R-369:** in M2, the decoder's milestone; it depends on TASK-M2-40 so that the saved charts it builds are
  among those checked. R-82's mirror test, R-438's `|λ̃_y|` and R-544's skip go with the step once this merges (their
  forward lines).
- *8 Nov 2026 (R-898):* RQ-750 is ruled (a) + (c): `z_β` is a periodic angle over the full circle, every stored latent view, saved chart and golden converted once, and for this deliberate change R-865's bitwise check is replaced by a conversion test (each converted value decoding to the same system to rounding precision, the goldens' outcome classes identical and regenerated with a record of why). The wait above is over, and the Goal's bitwise condition reads with R-898. The saved-file format: no version field, a new kind value instead, `Saved::from_words` refusing unknown kinds (applied per R-369). The curved form is recommended for oblique latent charts whose `q₁[1]` or `q₂[1]` is nonzero; the PR lists the affected charts. Whether a converted `z_β` is stored reduced or unreduced is a definition this task writes (R-72). *Was (acceptance):* "- `cargo test -p kernel mirror_step_removed` — a lower-half state encodes to `β ∈ (π, 2π)` and decodes back to itself, not its mirror image; a latent-chart trace through a syzygy continuous, or the reason it can't be on the list brought back; controls: the mirror step left in fails, one golden's `β` moved by one ulp fails the check (REQ-DEC-055)." "holds (the gui's in `crates/gui/src/explore/manifold_view/chart.rs`), through the new decoder against the baseline, bit for bit (REQ-DEC-055)." "- `cargo test -p kernel saved_chart` — the saved charts, against the baseline, bit for bit (REQ-DEC-055)." "decode_preset_golden` and `cargo test -p validation --test qa_TASK-M2-28` — the golden suites re-run with every golden unchanged, in the GPU jobs that run them (REQ-DEC-055)." "- Any preset, saved chart or golden that doesn't reproduce stops the task: the list goes to the owner, nothing re-baselined (REQ-DEC-055)." "owner's answer to RQ-750 and approved by the physics reviewer (REQ-DEC-056)."
