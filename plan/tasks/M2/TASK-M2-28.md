# TASK-M2-28 — Shape-sphere display: the whole sphere, the exponential map's one-to-one cap and the equal-area projection (R-544)

- **Milestone:** M2
- **Closes:** REQ-CHART-002, REQ-RENDER-026, REQ-CHART-047, REQ-CHART-075, REQ-CHART-078
- **Depends on:** TASK-M2-08, TASK-M2-14, TASK-M2-43
- **Needs (earlier milestones):** REQ-GUI-001, REQ-SYS-009
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~300 lines

## Goal
**Re-scoped by R-544 (21 Oct 2026); its open PR #271 is redone against this scope.** The shape sphere chart is the
whole sphere (R-544): it renders both hemispheres, pole to pole, every pixel its own system, with no hemisphere label,
no redundancy flag and no one-hemisphere view; the spherical map's `system_image` is bijective (TASK-M2-43 sets the
descriptor; this task's golden shows the render). It defaults to the equirectangular projection and offers an
equal-area alternative (Mollweide or Hammer–Aitoff) for quantitative area comparisons, where the Lagrange poles aren't
compressed. The exponential map computes its `system_image` from its cap: bijective over the cap, whether or not it
straddles the equator, its radius bounded at π and the `r > π` pixels the domain's end (R-503, as R-544 amends it),
and the pixels exactly on `r = π`, which all map to `−n0`, counted as REQ-CHART-078 defines.

## References
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-104 — The new `system_image` value is `DoubleCover` *(closes RQ-64)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `decisions.md` § "R-157 — `DoubleCover` stays, for the full-range Burrau chart *(closes RQ-127, amends R-141)*"
- `decisions.md` § "R-503 — The shape sphere's exponential map takes its `system_image` from its cap: bijective within one closed hemisphere, a per-pixel fold when the cap straddles the equator; its radius is bounded at π, past it the domain's end *(closes RQ-371; amends R-141 for the exponential map)*"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `decisions.md` § "R-110 — What CI runs, where, and against which goldens *(closes RQ-79)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- The projection option on the shape-sphere chart (`crates/kernel/src/chart/shape_sphere.rs`, carried in the render;
  the GUI selector is REQ-GUI-161, M8), with no hemisphere label and no redundancy flag.
- The exponential map's `system_image` computed from its cap, bijective over it, with the `r > π` domain's end and the
  `r = π` pixels as REQ-CHART-078 defines (`crates/kernel/src/chart/shape_sphere.rs`; REQ-CHART-075), replacing PR
  #252's `NToOne(2)` for that map.
- Golden images under `fixtures/golden/shape_sphere/`, rendered with native wgpu offscreen (R-110): one per projection,
  and one of the whole sphere on each map, an exponential cap straddling the equator among them.

## Acceptance tests
- `cargo xtask golden shape-sphere` — the shape-sphere render draws the whole sphere, pole to pole, with no hemisphere
  label and no redundancy flag; controls: a render of one hemisphere, and one striping the lower half as redundant,
  each fail; `cargo test -p engine chart_descriptors` reports the spherical map bijective (R-544) (REQ-CHART-002).
- `cargo xtask golden shape-sphere-projection` — one golden per projection (equirectangular default, the equal-area
  alternative) of the shape-sphere chart, both hemispheres drawn (REQ-RENDER-026; the M8 selector is REQ-GUI-161,
  TASK-M8-06).
- `cargo test -p kernel exp_map_system_image` — caps inside `w ≥ 0`, inside `w ≤ 0`, touching the equator from one
  side, and a straddling cap on a grid symmetric under `w → −w` (`n0` on the equator, `e2 = ŵ`, an odd resolution `N`,
  every pixel centre at `|r − π| > 10⁻⁶`) each report bijective, the straddling cap's counted total `N²` when
  `s√2 < π`, every pixel of both halves and the `w = 0` row counted once; with `s√2·(N−1)/N > π` on the same grid the
  total equals the number of pixels with `r ≤ π`, enumerated from `r` alone; controls: R-503's per-pixel fold (a pixel
  counting 0 when its mirror is in view), which undercounts, a fixed n = 2, and a count including the pixels with
  `r > π`, each fail (REQ-CHART-075).
- Definition: how pixels exactly on `r = π` (all mapping to `−n0`) count, written into chart_reference §3.3 with no
  appeal to a fold, and REQ-CHART-075's count follows it; physics reviewer approves (REQ-CHART-078).
- Definition: the equal-area projection and its keying written into chart_reference §3.3 and approved by the physics
  reviewer (REQ-CHART-047).

## Notes
- **Redone after R-544 (21 Oct 2026; the human's reply: "Redo #271 to match the whole-sphere chart").** PR #271 (head
  65aa9752) was built under R-141's 2-to-1 cover and R-503's per-pixel fold: its goldens `shape-sphere/one_hemisphere`
  and `shape-sphere/both_flagged` (the redundant hemisphere striped), its `exp_map_system_image` counts and its
  `r = π` definition ("by the per-pixel rule applied to `−n₀`") are measured or written under the fold. They are kept
  as the record, with R-544 beside them, not re-measured by the port; this task replaces them. The equal-area
  projection's definition (Hammer, a display remap, not on the sim key) and its goldens do not turn on the hemisphere
  and may be carried over, re-reviewed. It now depends on TASK-M2-43, which removes R-141's gauge from the decode and
  sets the spherical map's descriptor.
- **qa-file exception (R-544 forces it; applied per R-369).** `crates/engine/tests/qa_TASK-M2-14.rs:884` —
  `check_descriptor(shape(ShapeMap::Exponential), plain(NToOne(2), false))` — becomes the exponential map's value
  computed from its cap, bijective; qa changes it in this task's qa commit, the implementer does not (R-290), and the PR
  lists the `M` with this reason. Line 883, the spherical map, is TASK-M2-43's.
- *Superseded by R-544 (below, kept as the record):* the notes on R-141's n-to-1, the hemisphere label and toggle, the
  representative of a mirrored pair and the tilted fold describe the task before R-544.
- RQ-371 ruled: R-503 (option (b)); the representative of a mirrored pair is its `w > 0` pixel (applied per R-369).
- RQ-71 ruled: R-141 — the φ hemispheres are reflection-equivalent, so the shape sphere is n-to-1 (n = 2); `DoubleCover` is the full-range Burrau chart's (R-157).
- R-113 settles Gap G5: at M2 the two requirements are verified by golden image (one per projection and label); the projection selector and hemisphere toggle live in the Manifold view's Chart section at M8, shown when the chart is the shape sphere (REQ-GUI-161, TASK-M8-06).
- Gap G6: which equal-area projection (Mollweide or Hammer–Aitoff), and whether the projection is a different chart map (sim key, re-integrates) or a display remap.
- RQ-95 ruled: R-113 (Gap G5 above).
- Closes, for gaps the corpus leaves open: REQ-CHART-047 and REQ-CHART-078 (R-72 definitions) (REQ-CHART-047's classification accepted by R-132).
- **Untilted only (physics review 5593301205 of PR #255, P1; applied per R-369, flagged for the human's veto as it changes area statistics; accepted by R-559):** the values this task builds, the spherical map's n-to-1 with n = 2 and the exponential map's cap and per-pixel fold, are the untilted chart's. Under a tilt (R-473) two pixels fold only when their full `(a, b, d)` decode to the same system, a pixel's mirror being the pixel with the mirrored `(a, b)` and the same `d`: the mirror pair keeps its fold iff the tilt adds no hidden offset that depends on the mirrored axis (`N_b = 0` for `N ∝ (R·e_a) × (R·e_b)` from the active tilt map's `R`, decided exactly on the stored angles, per map as chart_reference §3.3 gives it; *was:* "computed from the active tilt map's `R` with `N = R·e₃`"), and otherwise the tilted view is bijective there (*was*, physics review 5593414323 of PR #255, P4: "`N_b = 0` for the tilted plane's normal `N = R·e₃`: `τ₂ = 0` on the spherical map, the analogous condition on an exponential frame aligned so the mirror is `(u, v) ↔ (u, 1 − v)`") (chart_reference §3.3; physics review 5593377792, P3; *was:* "a tilted view being generically bijective"); TASK-M2-36 builds and tests that, after this task.
