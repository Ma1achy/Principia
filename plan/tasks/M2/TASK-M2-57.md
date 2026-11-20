# TASK-M2-57 — The decoder's mirror step removed, latent β over the full circle, under the owner's bitwise check (R-865) ⚑

- **Milestone:** M2
- **Closes:** REQ-DEC-055
- **Depends on:** TASK-M2-40, TASK-M2-107
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-865, on RQ-739: the decoder's mirror step goes, and latent `β` runs over the full circle, so every chart, the shape
sphere included, uses one decode path with no special case, and a lower-half state encodes to `β ∈ (π, 2π)` in place
of a mirrored state. It is conditional on the owner's bitwise check: every existing preset, saved chart and golden with
`β ∈ [0, π]` reproduces unchanged through the new decoder. If anything doesn't, the task stops and brings the list back
to the owner before proceeding. **It waited on RQ-750:** `β = π·σ(z_β)` already covers `(0, π)`, so any full-circle
link changes every stored latent `z`, and the owner chooses the way out before the link is defined or anything is built. R-898 answered it, and R-919
answers RQ-754: one periodic `β` link, the presets and goldens whose plane runs along `z_β` redefined in the new
coordinates, and an unpinned old saved chart converted from `sigmoid_beta` with a notice.

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
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"

## Deliverables
- **First, on its own commit, before the decoder changes:** the baseline, every existing preset, saved chart and golden
  with `β ∈ [0, π]` recorded through today's decoder (IC words, sim key and, for a golden, its image hash), the PR
  listing each.
- **On the same baseline commit, before any link changes: the retired-link fixture (R-921; physics review 5836519618
  of PR #469, applied per R-369),** `fixtures/retired_beta_links/pre_m2_57.json`, written through today's decoder: for
  each of `sigmoid_beta`, `tanh_beta` and `softsign_beta` as the `β` link, the pixels' latent coordinates, the four
  blocks' links and each pixel's decoded starting state as f32 bits, the kernel's f32 decode, with the commit and the
  command that wrote it recorded in the file and its row in `fixtures/README.md`. The precision is f32 because that is
  where conversion can change the last bits: at f64 the converted `z_β` is the `β` the old decode computed, so the
  converted decode is bitwise the old one (dd_decoder § 3.2, "How a stored value converts"; physics review 5903954379 of
  PR #533; *17 Nov 2026 (R-1020):* that reason was the identity's, see the R-1020 note under Notes; the f32 precision
  stands), and an f64 fixture would leave the second arm of `retired_link_fixture` unexercisable by construction (qa
  review 5903950227 of PR #533, Q4). Its pixels span `z_β`: for each link, 257 values, `z_β` the link's
  inverse of `β = π·k/258`, `k = 1 … 257`, at the fixture's other latent coordinates. Every bit in it comes from the
  baseline commit's decoder and none is edited by hand: it may be written again only by running the baseline commit's
  decoder, never through any later decoder, and only as the bounded extension the acceptance line gives.
  REQ-TOOL-172's retired-link arm (TASK-M7-45) reads it.
- Built to the full-circle link and the signed encode TASK-M2-107 defines in dd_decoder § 3.2 and § 3.3 and dd_encode
  § 3.2 (REQ-DEC-056, closed there); this PR corrects them only where the built code differs, under physics review,
  each correction keeping its *Was*. *Was (14 Nov 2026, moved to TASK-M2-107):* "The full-circle link and the signed
  encode, defined in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2 (R-72; REQ-DEC-056), under physics review: the link
  and its inverse, its seam, how a syzygy crossing reads on a latent chart, why a `β ∈ [0, π]` start keeps its bits,
  and the pole value at `λ̃ = 0`."
- The decoder without its mirror step, in `crates/kernel` (its shared CPU/GPU source, the generated WGSL regenerated
  from it), the encode to `β ∈ (π, 2π)` for a lower-half state, and the shape sphere through the one decode path.
- The sections marked "[mirror-step removal …]" are rewritten by TASK-M2-107 to the decoder this task builds, in force
  when it merges; this PR corrects them only where the built code differs, each correction keeping its *Was*.
  *Was (14 Nov 2026, moved to TASK-M2-107):* "The sections marked "[mirror-step removal, ruled 8 Nov 2026 (R-865 …)]"
  rewritten to the built decoder, each keeping its old text as a "Was": canonical_spec § 3, chart_decoder_contract
  Part 2 and Part 5, inverse_encode_contract Part 1, chart_reference § 0.4 and § 3, dd_decoder § 3.3, dd_encode § 3.2,
  and principia_lines.md § 8's `z_β` row; and chart_composition's markers (§ 6.6's list) read with it."
- In the PR: every stored `z`, IC save file or test that relied on the old mirrored encode, and the effect on default
  windows of twice the area in `β` (the proposal's items 4 and 5).
- **One periodic `β` link (R-919, RQ-754 Q1 (c)):** the full-circle link is the `β` block's only link. dd_generation_root
  § 3.9's `β` rows and codomain, and REQ-GEN-013's `β` codomain and default, are rewritten to it by TASK-M2-107; the
  registry in `crates/ledger` is built to match. *Was (14 Nov 2026, moved to TASK-M2-107):* "dd_generation_root
  § 3.9's `sigmoid_beta`, `tanh_beta` and `softsign_beta` rows and its `β ∈ [0, π]` codomain, and REQ-GEN-013's `β`
  codomain and default, rewritten to it, each keeping its *Was*; the registry in `crates/ledger` to match." The three
  bounded maps are not deleted: they stay as conversion sources only, their forward maps kept for Q3 (a)'s conversion
  and the `latent_conversion` control "a value converted by the wrong link fails", not selectable on any block and not
  counted as `β` links (physics review 5834538186 of PR #465). The PR lists
  what the single `β` link changes in built code (REQ-VAL-024's 54 link selections become 18). REQ-GEN-014's two `β`
  densities are not met meanwhile, by R-919: no second `β` link is added (R-519).
- **Presets and goldens whose plane runs along `z_β` redefined, not converted (R-919, RQ-754 Q2 (b)):** each an affine
  plane in the new coordinates, a deliberate change exempt from the conversion test, the PR recording each with why;
  the goldens among them regenerated (R-898). No non-affine latent chart kind is defined for them.
- **An old saved chart's `β` link (R-919, RQ-754 Q3 (a)):** converted from the `β` link it pins where it pins one
  (R-516), else from `sigmoid_beta`, the default, with a notice on load naming the assumption.

## Acceptance tests
- Before the decoder changes, on its own commit: the baseline of every preset, saved chart and golden with
  `β ∈ [0, π]`, recorded through today's decoder (IC words, sim key and, for a golden, its image hash), the PR listing
  each (REQ-DEC-055).
- On the same baseline commit: `fixtures/retired_beta_links/pre_m2_57.json` written through today's decoder, before
  any link changes, for `sigmoid_beta`, `tanh_beta` and `softsign_beta`: latent coordinates, the four blocks' links and
  decoded starting states as f32 bits, the writing commit and command recorded, every bit from the baseline commit's
  decoder and none edited by hand (R-921; REQ-TOOL-172's retired-link arm).
- `cargo test -p kernel retired_link_fixture` — once the periodic link and the conversion exist: every fixture pixel
  decoded through its retained retired link equals the fixture's starting state bit for bit; and, for each of the three
  links, at least one fixture pixel whose `z_β`, converted to the periodic link and decoded, differs from the fixture's
  state in at least one bit. **The search is bounded, its budget fixed here and never raised after seeing results.**
  If a link has none among its 257 pixels, the fixture is written once more, by running the baseline commit's decoder,
  with that link's pixels extended to 4097 (`z_β` the link's inverse of `β = π·k/4098`, `k = 1 … 4097`), never
  edited by hand; that is the whole budget. Why these sizes: if conversion moves a bit at even 1 value in 250, 4097
  evenly spread values miss it only by chance well under one in a million, and 3 × 4097 states as bits keep the fixture
  a few megabytes; 257 is the first write, a sixteenth of it. If a link still has no bit-changing pixel within the
  budget, the task does not loop: the PR lists the link; TASK-M7-45's converting-reader control is marked not
  exercisable for it, with the evidence (the pixel count and the conversion's largest difference over the budget, in
  units in the last place of each state word); and a REVIEW_QUEUE entry goes to the owner as a report against R-921's
  premise that conversion "can change the starting state's last bits" (R-812). The second arm then checks such a link,
  listed in the test with its queue entry, the other way: no budget pixel changes a bit. It is reported, never counted
  as passing. Controls: a fixture state with one bit flipped fails the first arm; a fixture holding only pixels where
  conversion is exact, for a link not so listed, fails the second; and a listed link with a pixel where conversion
  changes a bit fails the second (R-921; REQ-TOOL-172).
- `cargo test -p kernel mirror_step_removed` — a lower-half state off the collinear axes (`|λ̃_y|` well above rounding,
  as near the `−x` half-axis the encode can give just below `π̂`, dd_encode § 3.2) encodes to `β ∈ (π, 2π)` and decodes
  back to itself, not its mirror image; a latent-chart trace through a syzygy at `β = 0` and at `β = π` continuous; controls: the mirror
  step left in fails, a σ-type link that jumps at `β = 0` fails (REQ-DEC-055).
- `cargo test -p kernel latent_conversion` — every converted `z` decodes to the same system as its old `z`
  within REQ-ENC-024's `ε_phys` on the decoded state; a saved file of the old version converted on load, one of an unknown kind refused by `Saved::from_words`;
  the converted words marked with a new kind value; an old saved chart converted from the `β` link it pins, an unpinned
  one from `sigmoid_beta` with the assumption reported for the notice on load; controls: a value converted by the wrong
  link fails, an unpinned chart converted with no assumption reported fails (R-898; R-919; REQ-DEC-055).
- `cargo test -p gui mirror_step_presets` and `cargo test -p engine mirror_step_presets` — every chart preset each crate
  holds (the gui's in `crates/gui/src/explore/manifold_view/chart.rs`) whose plane doesn't run along `z_β`, converted,
  against the baseline by the conversion test; each whose plane runs along `z_β` an affine plane in the new
  coordinates, listed in the PR with why (R-898; R-919; REQ-DEC-055).
- `cargo test -p kernel saved_chart` — the saved charts, converted, against the baseline by the conversion test (R-898;
  REQ-DEC-055).
- `cargo test -p render --test debug_views`, `cargo test -p render --test qa_TASK-M2-32`, `cargo test -p validation --test
  decode_preset_golden` and `cargo test -p validation --test qa_TASK-M2-28` — the golden suites re-run with every
  golden's outcome classes identical and its regenerated hash recorded with the reason, in the GPU jobs that run them;
  a golden whose plane runs along `z_β` (the latent decode golden's `q₁[1]`, `q₂[1]`) redefined in the new coordinates,
  exempt, its regenerated hash recorded with why (R-898; R-919; REQ-DEC-055).
- `cargo test -p kernel --test roundtrip` — with the assertions the ruling changes: T1 as the equivariance `E(Π·x)` = the
  mirrored `z` (`z_β → π − z_β` under R-1020's offset link, compared by its distance on the circle, `d`, dd_decoder
  § 3.2, as REQ-ENC-003 states it; the momentum controls reflected, `L_z` negated) in place of `E(Π·x) = E(x)`, and
  `t3_half_mirror` removed under REQ-ENC-005's change (REQ-DEC-055). *17 Nov 2026 (R-1020):* *Was:* "(`z_β → 2π̂ −
  z_β` compared by its distance on the circle, `d`, dd_decoder § 3.2, as REQ-ENC-003 states it;".
- `cargo test -p engine --test canon_gpu canonicaliser_seam_sweep` and `cargo test -p engine --test encode_gpu`, in the
  `gpu-kernel` job after `cargo xtask build-kernel` — the signed encode's `β` case (the sign and zero tests on `λ̃_x`
  and `λ̃_y`, built per R-547) equal bit for bit between CPU-f32 and the GPU on Metal and lavapipe over collinear states
  in random gauge whose rotated `λ̃_y` is a rounding residue of either sign, subnormal residues included; control: the
  plain float `λ̃` (`Canonical::lambda_t_y`, the superseded form), evaluated on the CPU under an unfused or flush-to-zero
  emulation, decides the case differently from the unflushed CPU on at least one input
  (REQ-DEC-002, REQ-COL-006; W11; physics review 5903954379 of PR #533, P4).
- `cargo test -p kernel --test ingest kappa_on_beta_seam` — the criterion, written here before the test runs (R-649),
  asked for by physics review 5907871580 of PR #533 (P7) and approved, the control's factor `10³` included, in physics
  review 5908031552 of PR #533 (P7). **The seam state:** equal masses, `α = π/4`, `β = 0`
  (`λ̃` on the `+x` half-axis, `λ̃_y = 0`, off the `α`-pole), `I = 1`, momenta a fixed, recorded `(p_ρ, p_λ)` with no
  component zero and none above 1 in size, so `hₖ = ε^{1/3}·max(1, |xₖ|)` lies between `6.1·10⁻⁶` and about
  `8.3·10⁻⁶` at f64 (the largest `|xₖ|`, a position, about 1.37); it is put in a random gauge (a fixed, recorded
  turn), so that `C`'s `λ̃_y` is a rounding residue. **The off-seam states:** the same state with `λ̃` turned by
  `β = ±η`, `η = 10⁻³` rad, so `|λ̃_y| = sin(π/4)·sin η ≈ 7.1·10⁻⁴`, over 80 `hₖ`: no `hₖ` step, which moves
  `λ̃_y` by about one `hₖ` at most, crosses the seam from them. **Asserted:** `κ` at the seam state is finite and agrees
  with `κ` at each off-seam state to a relative tolerance of `10⁻²`, `|κ_seam − κ_off| ≤ 10⁻²·κ_seam` (`κ` varies by
  about `η` relative across `±η`; the difference's truncation and rounding are about `hₖ² ≈ 4·10⁻¹¹` and `ε/hₖ ≈
  4·10⁻¹¹`): `kernel::encode::ingest::weighted_encode_jacobian` takes `z_β`'s component of the central difference by
  the signed difference on the circle `δ` (dd_decoder § 3.2, "The distance between two `z_β`"; inverse_encode_contract
  Part 4, "How it is evaluated"). **The mass columns' three-point stencil:** a second seam state, `m₀ = 3·10⁻⁶ ≤ hₖ`,
  `m₁ = m₂`, its positions set so `α = π/4` at `I = 1` and `β = 0`, in a gauge the test takes from a fixed, recorded
  list of turns, the first whose three `E[β]` values in `m₀`'s column (`E(x)`, `E(x + hₖe_{m₀})`, `E(x + 2hₖe_{m₀})`)
  straddle the seam (not all within `π` of each other on the line), the test asserting that it found one: the `z_β`
  component of that column's unweighted three-point difference, unwrapped by `δ` against `E(x)[β]`
  (inverse_encode_contract Part 4, "The mass columns"), is at most `10⁻⁴` in size, the state staying collinear as
  `m₀` moves, so its true value is 0 and its computed value rounding: positions of about 400 (`‖ρ‖ ≈ cos(π/4)/√m₀`)
  times `ε`, over `‖λ̃‖` and `hₖ`, about `10⁻⁷`. **Controls (R-176):** the same
  Jacobian with `z_β` subtracted on the line fails: at the first seam state its `κ` exceeds the off-seam states' `κ`
  (their own, which no step crosses the seam to change) by a factor of at least `10³` at f64 (`π/(200·hₖ)` at the
  largest `hₖ`, `8.3·10⁻⁶`, is `1.9·10³`), the expected factor being about `π/(2hₖ) ≈ 2.6·10⁵` for an off-seam `κ` of
  order 1, the margin of about 100 covering that `κ`'s size, and the measured factor reported; and the mass stencil combined on the line gives a `z_β` component of
  at least `π/(2hₖ) ≈ 2.6·10⁵` in size, the straddle putting at least `2π` on one of its three terms (R-898;
  REQ-DEC-055; the physics reviewer's ruling on Codex thread 4573289687 of PR #533).
- `cargo test -p engine --test lookup latent_plane_lift` — on the oblique latent plane `z₀ + u·q₁ + v·q₂` with
  `z₀ = 0`, `q₁ = 4e_β + 4e_q0` and `q₂ = e_q1` (a unit vector orthogonal to `e_β` and `e_q0`), the state whose
  encode is the plane's point at `u = 1, v = 0` (`z_β = z_q0 = 4`, every other component 0) lands on the chart with
  `|u − 1| ≤ 10⁻¹⁰` and `|v| ≤ 10⁻¹⁰`, its lift taken by least plane residual (dd_decoder § 3.2, "Stored reduced or
  unreduced"): the bound is written before the test runs (R-649), against `E∘D`'s recovery at f64, the precision the
  lookup runs at, of about `10⁻¹³` in the ball (REQ-VAL-022's measured 1.2·10⁻¹³) and the `q` link's `1/(σ(1 − σ))
  ≈ 57` at `z_q0 = 4`; control: the lift nearest the centre, `z_β = 4 − 2π`, leaves the plane, its residual nonzero
  and its `u = 1 − π/4 ≈ 0.21`, and fails (R-898; REQ-DEC-055; the physics reviewer's ruling on Codex thread
  4573289690 of PR #533; physics review 5907871580 of PR #533, P8).
- **The restated requirements' verifies (TASK-M2-107; qa review 5903950227 of PR #533, Q1).** Each runs as its
  requirement now states it, in the file named, the implementer's test files among them this task's to change:
  - `cargo test -p kernel --test canon` — REQ-DEC-002 (a lower-half state keeps `λ̃_y < 0` through canonicalise; the
    section's three conditions and idempotence); REQ-DEC-017 (`λ̃_y` at `±δ_λ`, `±0` and straddling keeps its sign,
    the same output twice; the source check that nothing in `C` or the encode reads `δ_λ`; controls: `C` with the
    mirror step left in fails, a `C` reading `δ_λ` fails the source check); REQ-DEC-018 (`canonical_noop` with `z_β`
    over the full circle).
  - `cargo test -p engine --test canon_gpu`, in the `gpu-kernel` job — REQ-DEC-002 and REQ-DEC-017
    (`canonicaliser_seam_sweep` without the mirror fork, the line above); REQ-ENC-015
    (`canonicaliser_canonicalise_order` with a lower-half input; control: the mirror step left in fails it).
  - `cargo test -p kernel --test encode_core` — REQ-ENC-007 (`encode_block_inverses` with `z_β` on both halves, read
    on the circle; `encode_collinear_beta`'s f64 sides, f32 expectation and pole landmark; `β` sets no
    `lookup_clamped`); REQ-ENC-003 (`encode_t1_gauge` as the equivariance, at its built bound, `z_β` on the circle;
    control: an encode with the mirror step left in fails); REQ-ENC-014 (`encode_notices`: a lower-half state raises
    none, bit 1 set on no input).
  - `cargo test -p engine --test encode_gpu`, in the `gpu-kernel` job — REQ-COL-006 and REQ-ENC-007 (the generated
    encode's signed `β` against the CPU's on both halves within REQ-DEC-043's factor on the circle, `π̂` on the `−x`
    half-axis with `±0` operands, the pole's 0).
  - `cargo test -p kernel --test roundtrip` — REQ-ENC-005 and REQ-DEC-016 (T3: `D(E(g·x)) = C(g·x)` and
    `L_z(D(E(g·x))) = det(Π_g)·L_z(C(x))` (`C(x)` at `I = 1` in the CoM frame, so the rescale's `L_canon = L_in/I_in^{1/4}` and the CoM's orbital term are taken out); controls: a configuration-only rotation in `C`, and a test-side mirror
    reflecting the `rᵢ` only, each fail), with the line above.
  - `cargo test -p kernel --test encode_by_kind encode_shape_sphere_signed` — REQ-ENC-034 with its three controls.
  - `cargo test -p kernel --test shape_sphere` — REQ-CHART-020 (the canonical form, `Φ` then the one `C`, on both
    hemispheres, a lower-hemisphere point keeping `n₂ < 0`).
  - `cargo nextest run -p ledger --test all link_registry::` — REQ-GEN-013 (`periodic_beta` is `β`'s default and
    resolves by name; no selection can name `sigmoid_beta`, `tanh_beta` or `softsign_beta`; control: a selection naming
    one is refused). *Was (TASK-M0-80, 19 Nov 2026: ledger's tests became one binary, `all`, its files under `tests/all/`; code review 5984285992 of PR #565, F1):* `cargo test -p ledger --test link_registry`.
  - `cargo test -p render --test decode_preset` — REQ-COL-006's fragment arms.
  - Review: REQ-ENC-002 (every listed consumer calls the one encode entry point) and REQ-CHART-002 (unchanged, its
    shape sphere through the one path).
  - REQ-CHART-100 (its statement and `region_d_variant`'s control) and REQ-CHART-171 (its statement and `region_d_plane`'s verify),
    and `docs/design/principia_chart_composition.md` § 8.2 (l.1206), restated through `plan/tools/reqio.py` (the doc by a
    dated rewrite), each with its *Was*, before this task merges, to the target wording in the Notes' 15 Nov 2026 entry
    (physics review 5908031552 of PR #533, P9; physics review 5908113445, P11). The restated verifies are what
    TASK-M2-68 (`cargo test -p kernel region_d_plane`, REQ-CHART-171) and TASK-M4-34 (`cargo test -p engine
    region_d_variant`, REQ-CHART-100, M4) build and run; neither is a dependency of this task, so this task runs either
    only if it is on `main` by then, reporting a test count above 0 (qa review 5908119719 of PR #533). REQ-CHART-100's
    existing control, a mantissa bit flipped in its uniform fails, is kept alongside the new fold control (applied per
    R-369: no control is dropped; physics re-checks it).
- Any preset, saved chart or golden (other than a preset or golden redefined under R-919) whose converted value doesn't decode to the same system within REQ-ENC-024's `ε_phys` on the decoded state, or
  whose outcome classes change, stops the task: the list goes to the owner (REQ-DEC-055).
- Review: for every saved oblique latent chart whose `q₁[1]` or `q₂[1]` is nonzero, the PR states which form it
  takes, the curved chart under the old link read through the new inverse or a planar approximation, and why: under the
  new `β` link such a plane becomes a curved surface, so a planar approximation is kept only where it passes the
  conversion test, and one that fails it stops the task like any other (brief item 15; R-898; REQ-DEC-055).

## Notes
- Added by R-865 (8 Nov 2026), the owner's reply 1 to the code map of that day, on RQ-739 (open on its item 3, the
  optional "mirror pairs counted once" statistic, which this task doesn't build).
- **Waits on RQ-750** (`rq: RQ-750` on REQ-DEC-055 and REQ-DEC-056): physics review 5817348709 of PR #449, P1. Not
  dispatched until it is ruled.
- **Requirements that change when this merges (R-865; physics review 5817348709, P2).** TASK-M2-107 restated
  them to the decoder this task builds, each keeping a "Was", binding from this task's merge; this task builds the code
  that meets them and runs their verifies (the acceptance lines above), correcting a statement only where the built code
  differs, under physics review. *Was (14 Nov 2026, moved to TASK-M2-107):* "Each carries a note now; this task rewrites
  their statements to the built decoder, each keeping a "Was"": REQ-ENC-003 (T1, to the equivariance
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
- *Fix round of PR #455 (P2 of physics review 5820410914; code review 5820413299 finding 5):* "to rounding precision" is bounded by REQ-ENC-024's `ε_phys`, applied per R-369; brief item 15 is an acceptance line. *Was (acceptance, the clauses):* "decodes to the same system as its old `z` to rounding precision;" and "doesn't decode to the same system to rounding precision, or".
- *9 Nov 2026:* **Waits on RQ-754**, filed from this task's implementer's stop report: R-898's periodic `z_β` against the `β` link registry (`sigmoid_beta`, `tanh_beta`, `softsign_beta` onto `β ∈ [0, π]`; REQ-GEN-013, REQ-GEN-014), the presets and goldens whose plane runs along `z_β` (flat, they fail the conversion test; curved, no chart kind defines them), and the `β` link an unpinned saved chart was made under (R-478, R-516). Not dispatched past its baseline commit until it is ruled; the baseline commit is not blocked.
- *9 Nov 2026 (R-919):* **RQ-754 is ruled,** the owner: "RQ-754: Q1 (c); Q2 (b); Q3 (a)." Q1 (c), one periodic `β` link,
  the second `β` density waiting for a study (R-519), REQ-GEN-014 not met on `β` meanwhile; Q2 (b), the presets and
  goldens whose plane runs along `z_β` redefined as affine planes in the new coordinates, exempt from the conversion
  test and recorded with why; Q3 (a), an unpinned old saved chart converted from `sigmoid_beta` with a notice on load.
  The wait is over; the deliverables and acceptance above carry it. The conversion test still holds for every other
  preset, stored latent view and saved chart, and the Review line on saved oblique latent charts stands. **Depends on
  gains TASK-M2-65** (applied per R-369): R-916 puts this task in stage 3, after stage 2's TASK-M2-65, as TASK-M2-55
  is. *Was (Depends on):* "TASK-M2-40". *Was (Goal):* "**It waits on RQ-750:**". *Was (acceptance):* "holds (the gui's
  in `crates/gui/src/explore/manifold_view/chart.rs`), converted, against the baseline by the
  conversion test (R-898; REQ-DEC-055)." "golden's outcome classes identical and its regenerated hash recorded with the
  reason, in the GPU jobs that run them (R-898; REQ-DEC-055)." "- Any preset, saved chart or golden whose converted value". "the converted words marked with a new kind value; control:
  a value converted by the wrong link fails (R-898; REQ-DEC-055)."
- *9 Nov 2026 (physics review 5834538186 of PR #465):* Q3 (a) needs the old forward maps (`sigmoid_beta`, and
  `tanh_beta` and `softsign_beta` for charts that pin them), as does the `latent_conversion` control "a value converted
  by the wrong link fails", so the one-periodic-link deliverable keeps the three as conversion sources only: not
  selectable, not counted as `β` links (R-919's Q1 (c) bullet; dd_generation_root § 3.9's R-919 line).
  *Was (Deliverables):* "the registry in `crates/ledger` to match; the PR lists".
- *10 Nov 2026 (R-921; physics review 5836519618 of PR #469, P1 and P2; applied per R-369: the pre-M2-57 fixture):*
  REQ-TOOL-172's retired-link arm (TASK-M7-45) needs the starting states a pre-TASK-M2-57 build decodes from a record
  made under `sigmoid_beta`, `tanh_beta` or `softsign_beta`; the record writer is built in M7, so the only independent
  source is a fixture written here, on the baseline commit, before any link changes. The Deliverables and acceptance
  tests gain its lines, and `retired_link_fixture` checks that the fixture has, for each link, pixels where converting
  `z_β` moves a bit, so the converting reader's control cannot pass trivially. Added, nothing removed. Paused under
  R-923 like the rest of this task; this edits its plan text only.
- *10 Nov 2026 (physics review 5836599062 of PR #469, P3; applied per R-369: the fixture's bounded search):* the
  second arm of `retired_link_fixture` had no exit if conversion is exact for a link, as it could be for a periodic
  link, still undefined, whose round trip is exact on `[0, π]`. The search is now bounded, 257 pixels per link and one
  extension to 4097, written before running; a link with none is reported to the owner, its converting-reader control
  marked not exercisable, never looped on. The Deliverables and the acceptance line now say the same thing about
  rewriting: every bit from the baseline commit's decoder, none edited by hand. *Was (Deliverables):* "Its pixels span
  `z_β`, and include, for each link, pixels where converting `z_β` to the periodic link before decoding changes at
  least one bit of the decoded state. It is never regenerated after the baseline commit;". *Was (acceptance):*
  "decoded starting states as bits, the writing commit and command recorded (R-921; REQ-TOOL-172's retired-link arm)."
  "state in at least one bit; if a link has none, the fixture is written again through the baseline commit's decoder
  with more pixels, never edited by hand; controls: a fixture state with one bit flipped fails the first arm, and a
  fixture holding only pixels where conversion is exact fails the second (R-921; REQ-TOOL-172)."
- *11 Nov 2026 (the re-plan's § A.1 and § B.1 item 3, KEEP; the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4):* its edge to TASK-M2-65 is dropped so it lands before TASK-M2-61's baseline, as TASK-M2-93 does, and the baseline is frozen once (applied per R-369: task scoping). *Was (Depends on):* "TASK-M2-40, TASK-M2-65".
- *14 Nov 2026 (applied per R-369: task scoping, the orchestrator):* split docs first. TASK-M2-107 writes REQ-DEC-056's
  definitions, the marked sections' rewrites, dd_generation_root § 3.9 and the fifteen requirements, and files
  REQ-VAL-022's restatement (RQ-821); this task builds the code to them, depends on TASK-M2-107, and keeps
  REQ-DEC-055. Its merge also waits on RQ-821's ruling. Code review 5903950686 of PR #533, F2: the Deliverables
  and Notes items that moved are pointed to TASK-M2-107, each keeping its *Was*, and the acceptance line "Review: the
  full-circle link and the signed encode …" is dropped, REQ-DEC-056 being TASK-M2-107's. *Was (that acceptance
  line):* "Review: the full-circle link and the signed encode in dd_decoder § 3.2 and § 3.3 and dd_encode § 3.2, written to the owner's answer to RQ-750 (R-898: `z_β` a periodic angle, each stored value converted once) and approved by the physics reviewer, with whether a converted `z_β` is stored reduced to one period or unreduced and where a wrap enters (R-72; REQ-DEC-056)." The 32 qa files the removal breaks at compile are named
  exceptions under R-237 and R-369 (a ruling, R-865 with R-916 and R-919, forces the change), edited in this task's qa
  commit only (R-290). *Was (Closes):* "REQ-DEC-055, REQ-DEC-056". *Was (Depends on):* "TASK-M2-40".
- *14 Nov 2026 (qa review 5903950227 of PR #533, Q4; applied per R-369):* the retired-link fixture's decoded states
  are f32 bits, where the conversion's last bits can differ; at f64 the conversion is exact. *Was (Deliverables):*
  "blocks' links and each pixel's decoded starting state as bits, with the commit and the command that wrote it recorded
  in the file and its row in `fixtures/README.md`." *Was (acceptance):* "decoded starting states as bits,".
- *14 Nov 2026 (qa review 5903950227 of PR #533, m3; applied per R-369):* `mirror_step_removed`'s lower-half state is
  off the collinear axes. *Was (acceptance):* "- `cargo test -p kernel mirror_step_removed` — a lower-half state encodes
  to `β ∈ (π, 2π)` and decodes back to itself,".
- *14 Nov 2026 (physics review 5904074113 of PR #533, P8; applied per R-369):* the roundtrip line's mirror rule reads as
  REQ-ENC-003's. *Was (acceptance):* "mirrored `z` (`β → 2π − β`, the momentum controls reflected, `L_z` negated)".
- *14 Nov 2026 (physics review 5904074113 of PR #533, P9):* after this task merges, `lookup_mirrored` is never set, so
  the IC record's `hemisphere` and `mirrored` (`principia_ic_save.md` § 3.1, REQ-TOOL-211) need a new definition; it is
  owed by TASK-M8-84 as REQ-TOOL-236 (R-72), not by this task.
- *14 Nov 2026 (physics review 5907871580 of PR #533, P7; R-649):* `kappa_on_beta_seam`'s criterion is written in the
  task file before it runs, for physics approval in PR #533 (given 15 Nov 2026 in physics review 5908031552, P7, the
  control's factor `10³` included; code review 5908084689, F2): the relative tolerance `10⁻²`, the off-seam offset
  `η = 10⁻³` beyond the stencil's reach, the control's factor `10³`, and a seam state with `m₀ ≤ hₖ` so the mass columns'
  three-point stencil runs, its bound `10⁻⁴`. *Was (acceptance):* "`cargo test -p kernel --test ingest kappa_on_beta_seam` — `κ` at a state on the `+x` half-axis of `λ̃` (`λ̃_y = 0`, `λ̃_x > 0`, off the `α`-pole) is finite and agrees with `κ` at the nearby off-seam states either side of the axis (`λ̃_y` of each sign), the relative tolerance stated in the test before it runs and approved in physics review (R-649): `kernel::encode::ingest::weighted_encode_jacobian` takes `z_β`'s component of the central difference, and of the mass columns' three-point stencil, by the signed difference on the circle `δ` (dd_decoder § 3.2, "The distance between two `z_β`"; inverse_encode_contract Part 4, "How it is evaluated"); control: the same Jacobian with `z_β` subtracted on the line fails, its `κ` of order `π/hₖ` (R-898; REQ-DEC-055; the physics reviewer's ruling on Codex thread 4573289687 of PR #533)."
- *14 Nov 2026 (physics review 5907871580 of PR #533, P8):* `latent_plane_lift` names `q₂` and states its bounds.
  *Was (acceptance):* "`cargo test -p engine --test lookup latent_plane_lift` — on the oblique latent plane with centre `z_β = z_q0 = 0` and `q₁ = 4e_β + 4e_q0`, the state whose encode is the plane's point at `u = 1` (`z_β = z_q0 = 4`) lands on the chart at `u = 1`, its lift taken by least plane residual (dd_decoder § 3.2, "Stored reduced or unreduced"); control: the lift nearest the centre, `z_β = 4 − 2π`, leaves the plane and fails (R-898; REQ-DEC-055; the physics reviewer's ruling on Codex thread 4573289690 of PR #533)."
- *14 Nov 2026 (physics review 5907871580 of PR #533, P2; Codex thread PRRT_kwDOUpUQ5c63jmxP):* **Waits on RQ-824**,
  where the golden IC sits once `β = z_β`: under the target, `z = 0` decodes to `β = 0`, a collinear state, not the
  golden IC, and building `β = z_β` changes what the built anchor and landmark tests decode (REQ-VAL-018,
  REQ-VAL-020, REQ-VAL-029; TASK-M2-03's `ic_descriptor` golden test and `chart_landmarks_lagrange_euler`). Not
  dispatched past its baseline commit until RQ-824 is ruled; its merge waits on it too, with RQ-821.
- *15 Nov 2026 (physics review 5908031552 of PR #533, P9; applied per R-369, physics's minimum option, so no
  requirement's demand changes before this task builds it):* REQ-CHART-100 and REQ-CHART-171 stand unchanged in
  TASK-M2-107's PR as `main`'s description, and this task restates them, and chart_composition § 8.2, through
  `plan/tools/reqio.py` (the doc by a dated rewrite), each with its *Was*, before it merges. After it merges, these
  clauses have no referent:
  - REQ-CHART-171's "bit-identical to C with the mirror step" and "canonicalise_signed, R-544's path";
  - REQ-CHART-100's "C with its step 3, the mirror, not taken" (with "canonicalise_signed, R-544's path" and "the
    decode must be bit-identical to C with the mirror step");
  - REQ-CHART-100's verify control, "the variant decoding through canonicalise (step 3 taken)", which can no longer be
    built from production code;
  - REQ-CHART-171's verify (`cargo test -p kernel region_d_plane`): "every pixel with η ≥ 0 is bit-identical to the
    decode ending in canonicalise" and its last control, "a decode ending in canonicalise (step 3 taken) fails", which
    production code can no longer build either (physics review 5908113445 of PR #533, P11);
  - `docs/design/principia_chart_composition.md` § 8.2 (l.1206): "the mirror not taken at `η = 0` (the deadband,
    R-82); past `η = 0` the decode takes the signed `C`, `canonicalise_signed`".

  The physics they demand carries over; the target wording, physics's: "ending in C (no mirror step, R-865): a pixel
  at η < 0 decodes to its own system, never folded onto (ξ, −η), Π of the (ξ, −η) pixel's state where the held
  velocities are their own mirror image"; "on η ≥ 0, D included, bit-identical to main's decode, R-898's conversion
  test"; and the control: "a test-side fold of every η < 0 pixel onto its (ξ, −η) partner fails". For REQ-CHART-171's
  verify (P11), physics's targets: "every pixel with η ≥ 0, D included, is bit-identical to main's decode (R-898's
  conversion test)", and the control "a test-side fold of every η < 0 pixel onto its (ξ, −η) partner fails"; its
  signed-half check ("every pixel with η < 0 decodes to Π of the (ξ, −η) pixel's state … λ̃_y < 0 kept") carries
  over unchanged. REQ-CHART-100's mantissa-bit-flip control is kept alongside the new fold control (applied per
  R-369: no control is dropped; physics re-checks it). The requirements' text stays unchanged in TASK-M2-107's PR
  (P9's minimum option). *Was (acceptance, the region-D line's last clause):* "`cargo test -p engine region_d_variant`
  and `cargo test -p kernel region_d_plane` then run as restated." (qa review 5908119719 of PR #533: those tests are
  TASK-M2-68's and TASK-M4-34's, so the line matched no test of this task's.)
- *17 Nov 2026 (R-1018, R-1019, R-1020; the owner's reply of 17 Nov 2026):* RQ-821, RQ-822 and RQ-824 are ruled, so this
  task waits on none of them. **R-1020 (RQ-824 (b)):** the periodic link is the offset `β = z_β + π/2`, not the
  identity, so `z = 0` stays the golden IC; this task builds it, writes the offset's floating-point form into
  dd_decoder § 3.2 under physics review (R-72), and restates the identity's worked values there and in dd_encode § 3.2
  and § 3.3 (they read shifted by `−π/2` until it does); REQ-VAL-020 (Euler at `z_β = ∓π/2`, interior points),
  REQ-ENC-003 (the mirror map `z_β → π − z_β`) and REQ-ENC-007 (`z_β = β − π/2`) are restated, and REQ-VAL-018 and
  REQ-VAL-029 stand. **R-1018 (RQ-821 (a) + (a2)):** REQ-VAL-022's gate is restated; this task runs it on its decoder
  and proposes, with its criterion written before the run, (a1)'s collinear tolerance and its measure under REQ-VAL-121
  (R-71, R-649), the members judged bit for bit those whose `C(x)` has `λ̃_y` exactly `±0`. **R-1019 (RQ-822 item 1
  (a)):** REQ-ENC-019's fixtures read the outer-pair coincidence as unclamped, a `t = 0` collision state; the
  outer-pair test that fails as written is restated to it. **R-1020's arithmetic, not a choice:** under the offset a
  converted `z_β` is `L(z_β) − π/2`, rounded once, and the decode adds `π/2` back, so the Deliverables' reason for the
  fixture's f32 precision ("at f64 the converted `z_β` is the `β` the old decode computed, so the converted decode is
  bitwise the old one") was the identity's: an f64 converted decode need not be bitwise the old one either. The
  fixture's f32 precision and `retired_link_fixture`'s two arms stand; `latent_conversion` judges every converted value
  within REQ-ENC-024's `ε_phys`, as R-898 has it (REQ-DEC-055, REQ-TOOL-172). T1's mirror map is restated above.
- *17 Nov 2026 (physics review 5944675263 of PR #560, an observation for this task's R-72 definition, not a ruling):*
  the exact values the port asserts (the old golden converting to `z_β′ = 0`, `z = 0` decoding to the old golden's
  `β` bits, REQ-VAL-020's "decode at `z_β = π/2` → `β = π̂` at f32 and f64") hold only if the offset constant is
  `c = π̂/2` at each precision, the same `c` in encode and decode. At f64 that is automatic (`fl(π/2) = fl(π)/2`); at
  f32 it is the rounded-toward-zero image 1.5707962513 (dd_generation_root § 3.9's rule for `π̂`), not the nearest f32
  1.5707963705, with which `z = 0` decodes one ulp above the old f32 golden `β` and `2c` passes `π`. REQ-VAL-020's
  verify already forces it; this task's physics review confirms the definition states it.
