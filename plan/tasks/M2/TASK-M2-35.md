# TASK-M2-35 — Every gallery chart swept over [0,1]²: 0 forbidden and 0 non-finite pixels, with an absolute-E chart as the control arm (R-467)

- **Milestone:** M2
- **Closes:** REQ-CHART-058
- **Depends on:** TASK-M2-08, TASK-M2-11, TASK-M2-12, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~250 lines, the test charts included

## Goal
A property test sweeps every chart `crates/kernel/src/chart` ships, at its default parameters, over a 257 × 257 grid on `[0,1]²`, its edges and corners included, through the real chart map, `validate` and the shared decoder, at f64 and at the f32 instantiation: no pixel is forbidden (`validate`'s layers 2 and 3, or a failing `Chart::map`) and no chart output or decoded state is non-finite (R-467, clause 7). The decoder's `DEGENERATE` tags are counted and printed per chart, not asserted (F5). The control arm is a deliberately broken test chart that sets `E` absolutely, so the check is shown to fire, and a test chart whose map is non-finite on one column.

## References
- `decisions.md` § "R-467 — Forbidden pixels and non-finite values are debug-only: a presentation view draws a pixel that is not part of the chart as the canvas background, with no hatch, key or share, and every failure and non-finite value `#141418`; one switchable debug view, View › Debug view, shows the hatch, `debug_invalid` and a NaN / Inf style, each with its key and count *(amends R-446, and R-407 clause 2 as it applies to display; closes RQ-319)*"
- `decisions.md` § "R-26 — Every chart declares its domain function *(CD-6)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-446 — "Degenerate" leaves the outcome view: black `#141418` means nothing has happened yet, up to the playhead, and bounded, running and every failure draw it; failures show in the debug views; forbidden chart regions keep the chart contract's hatch, decided from the chart's validity *(supersedes R-96, R-399 and R-441 as they apply to the outcome view)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"

## Deliverables
- `crates/kernel/tests/gallery_sweep.rs`: the chart list (the latent chart and the flat and body-plane slices, the shape sphere's spherical and exponential maps, `(L_z, E)` and `(L_z, K)`, the mass simplex, the Burrau family's Euclid plane, `(ν, K)`, acute-angle charts and both strips, the mixed-axis charts), each at its own default parameters; the sweep at f64 and f32; the per-chart tag counts printed.
- The control charts, test-only: an absolute-energy chart (`E = E_lo + s·(E_hi − E_lo)` over a fixed range on a slice whose potential `U` lies inside it) and a chart non-finite on one column.

## Acceptance tests
- `cargo test -p kernel gallery_sweep_unit_square` — each gallery chart, at f64 and f32: 0 forbidden, 0 non-finite, tag counts printed; control arm: the absolute-energy chart reports exactly the pixels with `E < U` forbidden, and a sweep whose `validate` accepts every pixel passes it and so fails the control; the one-column chart is caught as non-finite (REQ-CHART-058).
- Review checklist (physics reviewer): the chart list is every chart the kernel ships and each runs at its own default parameters (REQ-CHART-058).

## Notes
- **"The real engine"** is the kernel's own chart, decode and validate code, the code the GPU kernel monomorphises (R-313), run on the CPU (R-467 D11, applied per R-369).
- **The tags are reported, not asserted** (R-467 F5): R-446 keeps the decoder's tags from marking a pixel forbidden, and the invariant warp's `INVARIANT_INFEASIBLE` can arise inside the square only from rounding at the feasibility boundary. The physics reviewer may ask for more.
- **A Chart-builder chart** is not swept here: whether it may fail inside the square is RQ-372.
