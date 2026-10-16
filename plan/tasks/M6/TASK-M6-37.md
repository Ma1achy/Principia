# TASK-M6-37 — Degenerate-tilt guards

- **Milestone:** M6
- **Closes:** REQ-GUI-203
- **Depends on:** TASK-M6-26, TASK-M6-27, TASK-M6-31
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
On the mock engine, the GUI's tests for a degenerate basis add R-451 B11's scale-free test, at the guards' own cut.
Today `in_plane` (`det = 1 − g² ≤ 1e-12` on the unit directions) and `chart_coords` (`det ≤ 1e-24·g₁₁g₂₂`) are
sine-type tests, and at `τ₁ = ±τ₂ = ±90°` with `γ ≠ 0` they do not fire: one of `q₁`, `q₂` is rounding alone and the
sine between them is between about 0.27 and 1 depending on `σ`. Locked, `about_the_pin` then takes its pivot from that
rounding (physics review P15 of PR #227: `t ≈ −2.887·10¹⁴` with `z₀ = 0`, `z_locked = (0.1, 0.05, 0.2)`, and a tilt
drag to `(89°, 90°, γ = 45°)` sends a `z₀` edit of about `(−7.13·10¹², 0, 6.22·10¹⁰)`), and the mock's figure, built
through the same guards in `mock/quads.rs` and `mock/sample.rs`, is placed along the rounding direction. This task
keeps both guards' sine tests and adds `s = 2√(g₁₁g₂₂ − g₁₂²) / (g₁₁ + g₂₂) < 10⁻⁶`, `g` the Gram matrix of the plane's
own `q₁`, `q₂`: the guards' own cut, B11's, set by `s`'s rounding floor `√(17ε) ≈ 6.1·10⁻⁸` (numerical, not an R-71
calibration). It is not the compass's 10⁻³ drawing cut: `s` is also small where the vectors' norms differ (`s ≈ 2r`
for orthogonal vectors of norm ratio `r`), and inside the compass's 0.057° disc, off the corner points, a plane and an
accurate pivot exist (physics review P17 and P18). Where the guards decline, locked a basis edit sends no `z₀` edit and
passes as it does free (`about_the_pin`'s rule for a degenerate basis), and the figure shows nothing new; everywhere
else they return what they do today, so a locked drag inside the disc turns about the pin (§G4).

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
- One scale-free degeneracy test, `s` from the Gram matrix of a plane's `q₁`, `q₂` with the guards' 10⁻⁶ cut, used by
  `in_plane` and `chart_coords` beside their sine tests, which stay; where neither fires both return what they do
  today.
- `about_the_pin` and the mock's figure (`mock/quads.rs`, `mock/sample.rs`) unchanged in their own code beyond what
  the guards' new answer gives them: no `z₀` edit while locked and no new sample where the guards decline, and the
  pivot kept, the drag turning about the pin, everywhere else, the compass's disc included.
- Test `degenerate_tilt_guards`.

## Acceptance tests
- `cargo test -p gui degenerate_tilt_guards` — at each of the four corners τ₁ = ±τ₂ = ±90°, for γ = 0 and γ = 45° and over a sweep of σ in [0.01, 4], in_plane and chart_coords return none (s < 10⁻⁶ or the sine test fires); over a sweep of tilts off the corners, among them (90°, 89.9°), γ = 0, s = 1.75·10⁻³, and inside the compass's disc (90°, 89.97°, γ = 45°), s = 5.2·10⁻⁴, and (90°, 89.999°, γ = 45°), s = 1.75·10⁻⁵, they return today's values unchanged; a typed orthogonal basis q₁ = e_x, q₂ = 10⁻⁴e_y (norm ratio r = 10⁻⁴, s = 2·10⁻⁴), z₀ = 0, z = (0.5, 2·10⁻⁵, 0, …): both return values, chart_coords (0.75, 0.6) and in_plane (0.5, 2·10⁻⁵); s is unchanged by scaling q₁ and q₂ together; locked, with z₀ = 0 and z_locked = (0.1, 0.05, 0.2) on the corner basis (90°, 90°, γ = 45°, σ = 1), a tilt drag to (89°, 90°, γ = 45°) sends no SetField on z₀ and its basis edit passes unchanged, and likewise at each corner, γ = 0 and 45°, over the σ sweep; locked inside the disc, on (90°, 89.97°, γ = 45°, σ = 1) with z₀ = 0 and the pin z_locked = 0.3q₁ + 0.2q₂ (chart point (0.65, 0.60)), a tilt drag to (89°, 89.97°, γ = 45°) turns about the pin: its z₀ edit leaves the pin's chart point within 1e-12 of (0.65, 0.60); at a corner the mock's figure places no new sample (mock/quads.rs and mock/sample.rs give none); the failing control: today's guards (in_plane's det ≤ 1e-12, chart_coords' det ≤ 1e-24·g₁₁g₂₂), which at the corners with γ = 45° do not fire, give the pivot t ≈ −2.887·10¹⁴ and the drag's z₀ edit of about (−7.13·10¹², 0, 6.22·10¹⁰), and fail; review checklist (physics reviewer): s is B11's test on the plane's own basis, the guards' cut is B11's 10⁻⁶ with its derivation, and the compass's 10⁻³ is not applied to the guards (REQ-GUI-203).

## Notes
- **What to try.** The PR description has a "What to try" section (R-390), on `cargo run -p gui --features mock`: lock
  (K), tilt to a corner (τ₁ = τ₂ = 90°, then a rotation γ of 45°), and drag the tilt back off it: the view stays put,
  where before it jumped. Then, locked, drag the tilt to just short of a corner, where the readout says "no plane
  drawn" but the tilts are not both ±90°, and drag again: the view still turns about the pin, which stays put.
- **A GUI-track task** (R-390): it merges once its reviews pass and CI is green, without waiting for the gates of M1
  to M5.
- **Where it came from.** Physics review P15 of PR #227, decided per R-369: B11 rejects the sine test for the compass,
  and the merged guards use that same kind of test, so they fail at the same corners. B11 cites REQ-GUI-203 and this
  task for the locked case. Fix round 6, physics review P17 and P18: the guards get their own cut, 10⁻⁶, not the
  compass's 10⁻³, and keep their pivot inside the compass's disc.
- **The compass's own cut.** TASK-M6-33 builds the compass's `s` on the cube's frame; this task's `s` is the same
  quantity on the plane's whole basis, equal to it when the basis lies in the cube's frame. Either may land first; the
  one that lands second shares the other's function rather than adding a second. The cuts differ: the compass draws
  where `s ≥ 10⁻³`, the guards decline only where `s < 10⁻⁶` (or their sine test fires).
- **The physics reviewer** checks the test and the cut against R-451 B11, and the control against the review's case.
