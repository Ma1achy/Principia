# TASK-M2-93 — The seed conversion: `w = r/√I` everywhere, every changed golden and reference image a recorded conversion (R-934, R-931)

- **Milestone:** M2
- **Closes:** REQ-DEC-021, REQ-DEC-060
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** kernel plus goldens; the image list may be its own commit

## Goal
The momentum seed of the deterministic construction becomes the uniform expansion `w = r/√I` on every chart that uses it, the invariant charts and Burrau's included (R-934, RQ-764 (a)); R-82's largest-norm rule stays as a named legacy option, its seam declared. Every golden, reference image, fixture and baseline the change moves is rebuilt as a recorded conversion under R-931, before TASK-M2-61 records its baseline.

## References
- `decisions.md` § "R-934 — RQ-764, the momentum seed: option (a), the expansion seed `w = r/√I` takes the place of the largest-norm rule of R-82 everywhere, the invariant charts and Burrau's included; the seam is not physics; every image it changes is a recorded conversion, updated under R-931; R-82's rule is kept as a named option with its seam declared *(closes RQ-764; amends R-82)*"
- `decisions.md` § "R-931 — Goldens: a golden or reference image that is provably wrong is fixed, everything that depends on it is updated, and tests run against the fixed version; an incorrect thing is never treated as correct, and a bug is never replicated for the sake of matching it *(no RQ; refines R-110)*"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "3.4 Momentum"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 3 — Block inverses (closed forms, with their forward mates)"

## Deliverables
- The seed `w = r/√I` in `crates/kernel`'s momentum construction, R-82's rule a named option.
- Every image the seed changes, rebuilt as a recorded conversion (R-931: a provably wrong golden is fixed, its dependents updated, tests run against the fixed version), the proof stated in the PR and the `BASELINES.md` row. **The PR owes the list of images by file.**
- `docs/design/principia_dd_decoder.md` § 2 and § 3.4 and the inverse encode contract's seed passage (Part 3, Kind 3: "the largest-norm qualifying seed is taken, ties by seed order — R-82") rewritten to the seed in force, each replaced line kept as *Was*.

## Acceptance tests
- `cargo test -p kernel seed_selection` — under the seed in force the construction takes `w = r/√I` on every chart that uses it, `NO_QUALIFYING_SEED` unreachable for `I > 0`; under the named legacy option (R-82's rule) a fixture where several seeds qualify selects the largest-norm one, not the first, an exact-tie fixture selects the earlier seed, forcing the primary seed to degenerate (ρ = 0) selects a fallback, not DEGENERATE, and DEGENERATE fires only where no seed has ‖w⁽²⁾‖²_m > ε_w (REQ-DEC-021).
- `cargo test -p kernel seed_conversion` — the conversion test of every invariant and mixed-energy chart image where `K* > K_min`; the legacy rule a named option. Measurement: the seam jump in the decoded velocity across `z_α = 0` on the kernel's `(L_z, E)` chart; criterion, written before any run: after the change no neighbouring pair of pixels across the line differs in decoded velocity by more than the largest neighbouring difference within 8 pixels of it on either side. Control (the measurement shown able to fire): the same measurement under R-82's legacy rule, the named option, must fail the same criterion; if it passes, the criterion cannot see the seam and the result is void. Under the legacy rule the decoded velocity is predicted to jump by `2√(K*−K_min)` at each pixel on the line (1.9545 at the chart review's one point, a prototype figure, R-649); no threshold depends on that number. With it (R-934): at the invariant charts' default pin, the equal-mass Lagrange triangle, every pixel starts a homographic orbit whose break-up is set by rounding, labelled (REQ-DEC-060).

## Notes
- Entered 11 Nov 2026 as N10 of the chart re-plan (`plan/charts/m2-chart-system-replan.md`) (applied per R-369: task scoping); its acceptance is the re-plan's § B.1 item 4. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold.
- No edge is named for this task; it lands before TASK-M2-61's baseline, so TASK-M2-61 depends on it (applied per R-369: task scoping).
- REQ-DEC-021 (the momentum seed) was closed by TASK-M2-09, which is built and is not rewritten: the requirement is brought into line with R-934 and moves here, the task that builds the new behaviour (11 Nov 2026).
- TASK-M2-96 (the second sheet) starts from the construction as this task leaves it; the first slice (TASK-M2-86) names the seed in force when it runs.
- The re-plan's open item O10 (the seed not ruled) is closed by R-934.
