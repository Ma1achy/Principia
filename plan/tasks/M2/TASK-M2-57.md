# TASK-M2-57 — The decoder's mirror step removed, latent β over the full circle, under the owner's bitwise check (R-865) ⚑

- **Milestone:** M2
- **Closes:** REQ-DEC-055, REQ-DEC-056
- **Depends on:** TASK-M2-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-865 closes RQ-739: the decoder's mirror step goes, and latent `β` runs over the full circle, so every chart, the shape
sphere included, uses one decode path with no special case, and a lower-half state encodes to `β ∈ (π, 2π)` in place
of a mirrored state. It is conditional on the owner's bitwise check: every existing preset, saved chart and golden with
`β ∈ [0, π]` reproduces unchanged through the new decoder. If anything doesn't, the task stops and brings the list back
to the owner before proceeding.

## References
- `decisions.md` § "R-865 — The decoder's mirror step is removed and latent `β` runs over the full circle, conditional on the bitwise check: every preset, saved chart and golden with `β ∈ [0, π]` reproduces unchanged, and if anything doesn't, the list comes back to the owner before the removal proceeds *(closes RQ-739)*"
- `decisions.md` § "R-854 — Symmetry reductions apply only to how a starting state is described at `t = 0`: trajectories evolve and are shown in full physical space, nothing is folded during a run, and the only operation besides the step is the per-step centre-of-mass projection, energy and `L_z` monitored, never corrected"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_dd_decoder.md` § "3.3 Canonicalise"
- `docs/design/principia_dd_encode.md` § "3.2 Canonicalisation `C` — the exact ordered procedure"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/design/principia_lines.md` § "8. Traces"

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
- `cargo test -p kernel mirror_step_removed` — every preset, saved chart and golden with `β ∈ [0, π]` through the new
  decoder against the recorded baseline, bit for bit; a lower-half state encodes to `β ∈ (π, 2π)` and decodes back to
  itself, not its mirror image; a latent-chart trace through a syzygy continuous, or the reason it can't be on the list
  brought back; controls: the mirror step left in fails, one golden's `β` moved by one ulp fails the check (REQ-DEC-055).
- `cargo test -p kernel --test roundtrip` and `cargo test -p kernel saved_chart` — re-run, passing as before (REQ-DEC-055).
- Review: the full-circle link and the signed encode in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2, approved by
  the physics reviewer (REQ-DEC-056).

## Notes
- Added by R-865 (8 Nov 2026), the owner's reply 1 to the code map of that day, closing RQ-739.
- **The condition is a stop, not a gate to tune.** If any preset, saved chart or golden with `β ∈ [0, π]` doesn't
  reproduce, the task stops before the removal merges and the list goes to the owner (the orchestrator files it on the
  review queue). Nothing is re-baselined, loosened or special-cased to pass.
- **Traces at syzygies.** The owner's words say this also fixes the latent-chart traces' jump at syzygies. RQ-739's
  physics note recorded that a crossing at `β = 0` would still wrap under `β ∈ (0, 2π)`; the link this task defines
  decides it, and a trace that still jumps goes on the list.
- **Not here:** the proposal's optional "mirror pairs counted once" statistic (its item 3) is not ruled, and no task
  builds it.
- **Applied per R-369:** in M2, the decoder's milestone; it depends on TASK-M2-40 so that the saved charts it builds are
  among those checked. R-82's mirror test, R-438's `|λ̃_y|` and R-544's skip go with the step once this merges (their
  forward lines).
