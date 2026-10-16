# TASK-M6-37 — Degenerate-tilt guards

- **Milestone:** M6
- **Closes:** REQ-GUI-203
- **Depends on:** TASK-M6-26, TASK-M6-27, TASK-M6-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
On the mock engine, the GUI's tests for a degenerate basis use R-451 B11's scale-free test. Today `in_plane`
(`det = 1 − g² ≤ 1e-12` on the unit directions) and `chart_coords` (`det ≤ 1e-24·g₁₁g₂₂`) are sine-type tests, and at
the four tilt corners `τ₁ = ±τ₂ = ±90°` with `γ ≠ 0` they do not fire: one of `q₁`, `q₂` is rounding alone and the
sine between them is between about 0.27 and 1 depending on `σ`. Locked, `about_the_pin` then takes its pivot from that
rounding (physics review P15 of PR #227: `t ≈ −2.9·10¹⁴` with `z₀ = 0`, `z_locked = (0.1, 0.05, 0.2)`, and a tilt drag
to `(89°, 90°, γ = 45°)` sends a `z₀` edit of about `(−7.1·10¹², 0, 6.2·10¹⁰)`), and the mock's figure, built through
the same guards in `mock/quads.rs` and `mock/sample.rs`, is placed along the rounding direction. This task replaces
both guards with `s = 2√(g₁₁g₂₂ − g₁₂²) / (g₁₁ + g₂₂) < 10⁻³`, `g` the Gram matrix of the plane's own `q₁`, `q₂`, the
cut B11 sets (numerical, from the 1e-12 normal tolerance, not an R-71 calibration). Where `s < 10⁻³` the guards return
none, so locked a basis edit sends no `z₀` edit and passes as it does free (`about_the_pin`'s rule for a degenerate
basis), and the figure shows nothing new.

## References
- `decisions.md` § "R-451 — The compass's cube is drawn as `principia_nav_cube.html` draws it, in the dev GUI's dark colours: isometric, depth-sorted edges, the slice as its true cross-section polygon, Lambert-shaded, with the reference's labels, readout, drags and easing; the drawing follows the decoder's tilt mapping *(amends R-409 point 1, the compass's view angle)*"
- `decisions.md` § "R-409 — The compass has no orbit and two modes, Tilt and Slice, set by its buttons and by the slider touched; the keyboard has a navigation mode (orange ring) and an interaction mode (blue ring), Enter going in and Esc coming out *(amends R-390 and R-404)*"
- `decisions.md` § "R-390 — The GUI track starts now, on a mock engine, in parallel with the physics and renderer chain, which keeps priority for agent slots"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/gui/principia_render_gui_spec.md` § "The compass: look and views (R-451, R-452)"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 3 — Charts"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"

## Deliverables
- One scale-free degeneracy test, `s` from the Gram matrix of a plane's `q₁`, `q₂` with the 10⁻³ cut, used by
  `in_plane` and `chart_coords` in place of their determinant thresholds; off the cut both return what they do today.
- `about_the_pin` and the mock's figure (`mock/quads.rs`, `mock/sample.rs`) unchanged in their own code beyond what
  the guards' new answer gives them: no pivot and so no `z₀` edit where `s < 10⁻³` while locked, and no new sample.
- Test `degenerate_tilt_guards`.

## Acceptance tests
- `cargo test -p gui degenerate_tilt_guards` — at each of the four corners τ₁ = ±τ₂ = ±90°, for γ = 0 and γ = 45° and over a sweep of σ in [0.01, 4], s < 10⁻³ and in_plane and chart_coords return none; over a sweep of tilts with s ≥ 10⁻³ (among them (90°, 89.9°), γ = 0, s = 1.75·10⁻³) they return today's values within 1e-12; s is unchanged by scaling q₁ and q₂ together; locked, with z₀ = 0 and z_locked = (0.1, 0.05, 0.2) on the corner basis (90°, 90°, γ = 45°, σ = 1), a tilt drag to (89°, 90°, γ = 45°) sends no SetField on z₀ and its basis edit passes unchanged, and likewise at each corner, γ = 0 and 45°, over the σ sweep; at a corner the mock's figure places no new sample (mock/quads.rs and mock/sample.rs give none); the failing control: today's guards (in_plane's det ≤ 1e-12, chart_coords' det ≤ 1e-24·g₁₁g₂₂), which at the corners with γ = 45° do not fire, give the pivot t ≈ −2.9·10¹⁴ and the drag's z₀ edit of about (−7.1·10¹², 0, 6.2·10¹⁰), and fail; review checklist (physics reviewer): s is B11's test on the plane's own basis and the cut is B11's (REQ-GUI-203).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: lock
  (K), tilt to a corner (τ₁ = τ₂ = 90°, then a rotation γ of 45°), and drag the tilt back off it: the view stays put,
  where before it jumped.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5.
- **Where it came from.** Physics review P15 of PR #227, decided per R-369: B11 rejects the sine test for the compass,
  and the merged guards use that same kind of test, so they fail at the same corners. B11 cites REQ-GUI-203 and this
  task for the locked case.
- **The compass's own cut.** TASK-M6-33 builds the compass's `s` on the cube's frame; this task's `s` is the same
  quantity on the plane's whole basis, equal to it when the basis lies in the cube's frame. Either may land first; the
  one that lands second shares the other's function rather than adding a second.
- **The physics reviewer** checks the test and the cut against R-451 B11, and the control against the review's case.
