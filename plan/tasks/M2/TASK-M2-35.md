# TASK-M2-35 — Every gallery chart swept over [0,1]²: 0 forbidden and 0 non-finite pixels, with an absolute-E chart as the control arm (R-467)

- **Milestone:** M2
- **Closes:** REQ-CHART-058
- **Depends on:** TASK-M2-08, TASK-M2-11, TASK-M2-12, TASK-M2-13
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~350 lines, the test charts and parameter sets included

## Goal
A property test sweeps every chart `crates/kernel/src/chart` ships, at the parameter sets this task names, over a 257 × 257 grid on `[0,1]²`, its edges and corners included, through the real chart map, `validate` and the shared decoder, at f64 and at the f32 instantiation: no pixel is forbidden (`validate`'s layers 2 and 3, or a failing `Chart::map`), no chart output `z` or decoded `(m, r, p)` is non-finite, and the warp charts raise no `INVARIANT_INFEASIBLE` (R-467, clause 7; F5 and D11 as revised on physics review 5585225466 of PR #253). The other `DEGENERATE` tags and the grid points landing on a collision pole are counted and printed per chart. The control arm is a deliberately broken test chart that sets `E` absolutely, so the check is shown to fire, a warp with `L_max` one ulp too large, and a test chart whose map is non-finite on one column.

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
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `decisions.md` § "R-436 — The descriptor's `±∞` at the poles are stored as computed; every consumer, CPU and GPU alike, colours a non-finite input as invalid, tested by comparing the stored value, never by arithmetic on `∞` *(closes RQ-284; extends R-79)*"
- `decisions.md` § "R-13 — Lookup has no coincident-bodies rejection *(closes RQ-11)*"
- `decisions.md` § "R-60 — No t = 0 escape outcome *(closes RQ-19)*"

## Deliverables
- `crates/kernel/tests/gallery_sweep.rs`: the chart list (the latent chart and the flat slices; the shape sphere's spherical and exponential maps; `(L_z, E)` and `(L_z, K)`; the mass simplex; the Burrau family's Euclid plane, `(ν, K)`, `Φ_θ,K`, `Φ_θ,L_z`, `Φ_θ,δm` and both strips; the mixed-axis charts), each at the parameter sets below; the sweep at f64 and f32; the per-chart tag counts and collision-pole counts printed.
- **The parameter sets** (no chart has a `Default`; physics review P4). Where the register (`crates/ledger/src/constants.rs`) holds a value the sweep uses it, and where that value is a pending calibration the one the chart ships today, the test saying so beside it:
  - the invariant charts and the Burrau `K` charts: `K_max = 1`, `γ_K = 2` (REQ-CHART-044, proposed, provisional until the M2 gate), and the edge set `γ_K = 1`; `z₀`'s configuration and mass controls at 0 and at one seeded draw from `[−1, 1]⁴`;
  - the mass simplex: `ε_m = 10⁻⁴` (the register's `mass_simplex_eps_m`, chart_reference §4.5) and `ν₀ = 1/2`;
  - the Euclid plane: `m ∈ [1, 32]`, `ν ∈ [1/32, 31/32]` (the register's `euclid_*`, chart_reference §4.5); `(ν, K)` on the same `ν` range;
  - the acute-angle charts: `θ ∈ [θ_min, θ_max]` = `[0.0317433662712574, 0.7853981256484985]` (REQ-CHART-045, proposed); `Φ_θ,L_z`'s `K = 1/8` and `λ_L = 1` (REQ-CHART-046, proposed; `λ_L = 1` is also the edge set); `Φ_θ,δm`'s equal-mass target;
  - the latent chart and the flat slices: `z₀ = 0` with `q₁`, `q₂` two coordinate axes, and one seeded `z₀` with a tilted orthonormal pair;
  - the shape sphere: `I = 1`, `φ_f = 0`, `z₀`'s mass and momentum blocks 0 and one seeded draw; the exponential map about `n₀ = (0, 0, 1)`, `e₁ = (1, 0, 0)`, `e₂ = (0, 1, 0)`, at `s = π/4` and `s = π/2`;
  - the mixed-axis charts: each pair the module defines, frozen `z = 0`, on a range inside each axis's codomain and, as the edge set, a range reaching its link's closed codomain edge (`α = π/2`, `|p| = q_max`), where the link inverse can make `z` non-finite.
- The control charts, test-only: an absolute-energy `(L_z, E)` chart (`E = E_lo + v·(E_hi − E_lo)` over a fixed range on a slice whose potential `U` lies inside it), a warp with `L_max` one ulp too large, and a chart non-finite on one column.

## Acceptance tests
- `cargo test -p kernel gallery_sweep_unit_square` — each gallery chart at each named parameter set, at f64 and f32: 0 forbidden, 0 non-finite (`z` and `(m, r, p)`), `INVARIANT_INFEASIBLE` = 0 on `(L_z, E)`, `(L_z, K)`, `(ν, K)`, `Φ_θ,K`, `Φ_θ,L_z` and the `K` strip; the other tag counts and the collision-pole counts printed; control arm: the absolute-energy chart reports exactly layer 2's forbidden set `|L_z| > √(2I(E − U))`, `E < U` included, and a sweep whose `validate` accepts every pixel passes it and so fails the control; the one-ulp warp fires `INVARIANT_INFEASIBLE` on the `s ∈ {0, 1}` columns; the one-column chart is caught as non-finite (REQ-CHART-058).
- Review checklist (physics reviewer): the chart list is every chart the kernel ships and each runs at the parameter sets named above (REQ-CHART-058).

## Notes
- **"The real engine"** is the kernel's own chart, decode and validate code, the code the GPU kernel monomorphises (R-313), run on the CPU (R-467 D11, applied per R-369).
- **One tag is asserted, the others reported** (R-467 F5 as revised on physics review P2): `INVARIANT_INFEASIBLE` is decided on `(L_z, K*)` alone, the warp's own claim (chart_reference §2.1: "no pixel is infeasible by construction"), so inside the square it is clause 4's in-square failure and would escape D7's warning, which reads the f64 `validate` lane; R-446 keeps the other tags from marking a pixel forbidden.
- **`V₀` and `E₀` are outside the non-finite count** (physics review P5): at an exact binary collision they are R-436's defined `−∞`, a `t = 0` collision, a system (R-13, R-60), not a failure; the collision-pole count says how many grid points land there.
- **BodyPlane is not swept here**: it is not built at M2; TASK-M3-36, which builds it, adds it to this sweep (REQ-CHART-059; physics review P3).
- **A Chart-builder chart** is not swept here: whether it may fail inside the square is RQ-372.
- **R-506 (20 Oct 2026) rules RQ-372:** a builder chart's physical-quantity axes are warped to their feasible range by default, and under linear its infeasible pixels are not a system; the warps are TASK-M2-37's (REQ-CHART-067, REQ-CHART-070) and the warning TASK-M8-47's, so the question is no longer open, and this task's sweep is unchanged. The note above is kept as history (code review 5587720942 of PR #255, F7).
- *Was (as first ported):* the chart list held "the body-plane slices", each chart ran "at its own default parameters", the tags were "reported, not asserted", and the control "reports exactly the pixels with `E < U` forbidden". Stale, replaced by R-467 D11 and F5 as revised on physics review 5585225466 of PR #253.
