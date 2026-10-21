# TASK-M8-67 — The initial-condition constructor's shape sphere: every point of both halves reachable, the drag across the equator flipping the triangle, and the mirror twin shown as a faint marker with its note (R-544)

- **Milestone:** M8
- **Closes:** REQ-GUI-343, REQ-GUI-344
- **Depends on:** TASK-M8-15, TASK-M2-43
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** PIT-9
- **Size:** ~250 lines

## Goal
R-544 keeps two shape spheres apart (the plain summary). The initial-condition constructor's sphere, the Inspector's
pane 2 sphere beside the triangle and the bodies (render_gui_spec §G8; TASK-M8-15 builds the pane), is the one place
where the mirror is information: every point of both halves is reachable, nothing blocked off; its point is the IC's
signed `n̂`. A press within 8 logical px of the point, on the 3D sphere or the unwrap, drags it; any other drag
orbits the 3D view or pans the unwrap, as R-453 D7 and D8 give it. A drag changes the IC's shape only: the IC keeps
its own `√I`, the direction of `ρ`, its CoM position and CoM velocity, its masses and its Jacobi momenta in the frame
`ρ̃` on `+x`, so pane 1's triangle changes shape, and flips when the point crosses the equator, without jumping in
size or turning. A binary-collision landmark on the equator is reachable like any point. The IC is set through the
decode, never the lookup, with no NaN, and the run ends `collision` at `t = 0`, its bodies within `r_coll` (R-60;
integrator_contract Part 5). A drag reaches the landmark only to rounding (the unwrap's `(π, π/2)` decodes to
`‖ρ̃‖ ≈ 6.8e−17`), so there `V₀` and `E₀` are `−∞` where the emitted `r₀ = r₁` and finite otherwise; no snap is
added. At an exact `n̂ = (−1, 0, 0)`, `ρ̃ = 0` and `r₀ = r₁`, and the descriptor stores `V₀` and `E₀` as `−∞`
(dd_decoder §3.6, R-436). A non-finite `V₀` or `E₀` is drawn as R-436 and R-467 draw a non-finite value; a lookup of that IC elsewhere reads `lookup_clamped` (R-13). At `ρ̃ = 0` the direction of `ρ` is
undefined, so the constructor keeps the frame angle from the IC's last point with `ρ̃ ≠ 0` (the decode's R-437 angle 0
if it never had one) and holds the momenta in that frame; a drag off the landmark restores `ρ` along it (a
definition, R-72). The current IC's mirror twin, `Π·x`, positions and momenta reflected, `L_z` opposite, whose shape sits at
`(θ, π − φ)`, is drawn as a faint marker there, with the inspector's note "mirror twin of the system at
`(θ, π − φ)`": shown, never enforced, never a snap or a limit.

## References
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-453 — A click on the figure pins that sample's trajectory in the Trajectory panel, and hover never replaces it; the cursor moving into the panel never clears it; the panel's real-space view pans and zooms and its shape-sphere view orbits and zooms"
- `decisions.md` § "R-436 — The descriptor's `±∞` at the poles are stored as computed; every consumer, CPU and GPU alike, colours a non-finite input as invalid, tested by comparing the stored value, never by arithmetic on `∞` *(closes RQ-284; extends R-79)*"
- `decisions.md` § "R-60 — No t = 0 escape outcome *(closes RQ-19)*"
- `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time, pre-integration)"
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"

## Deliverables
- In `crates/gui/src/windows/inspector/pane_canonical.rs` (TASK-M8-15's file), the sphere view's point draggable over
  the whole sphere by a press within 8 logical px of it (any other drag orbiting or panning, R-453 D7, D8), a drag
  setting the IC's shape through the chart's decode of the dragged `n̂` (TASK-M2-43's signed decode) and keeping the
  IC's `√I`, the direction of `ρ`, its CoM position and velocity, its masses and its Jacobi momenta in the frame `ρ̃`
  on `+x`, and the frame angle recorded from the last point with `ρ̃ ≠ 0` for use at `ρ̃ = 0`; pane 1's triangle view
  following it.
- The twin marker at `(θ, π − φ)`, faint, and the inspector's note "mirror twin of the system at `(θ, π − φ)`" with the
  twin's angles; the twin's readout `Π·x`.
- Tests `constructor_sphere_reach` and `constructor_twin` (`crates/gui`) and screenshot cases
  `05_inspectors/constructor_upper`, `05_inspectors/constructor_lower` and `05_inspectors/constructor_equator`.

## Acceptance tests
- `cargo test -p gui constructor_sphere_reach` and `cargo xtask screenshot 05_inspectors` (a lower-hemisphere point
  and its triangle) — REQ-GUI-343's verify: drags to points of both hemispheres, the poles and the equator included,
  set the IC's `shape_vec` to the dragged point within REQ-CHART-053's bound; a drag from `(θ, φ)` to `(θ, π − φ)`
  flips the triangle's orientation (its signed area changes sign) and keeps the masses and the held Jacobi momenta;
  after Scale × 1.25, Rotate 15° and Boost →, a drag keeps `√I`, the direction of `ρ`, the CoM position and the CoM
  velocity; a press 4 px from the point drags it, and one 20 px away orbits (or pans) and leaves the IC unchanged; a
  drag onto the landmark `n = (−1, 0, 0)`, which it reaches only to rounding, sets an IC with no NaN anywhere, its
  `V₀` and `E₀` either `−∞` on their bits (where the emitted `r₀ = r₁`) or finite, drawn as R-436 and R-467 have it
  when non-finite, and its run reports `collision` at `t = 0` either way, its bodies within `r_coll`
  (integrator_contract Part 5, R-60); the exact case, `n̂ = (−1, 0, 0)` set through the IC's own setter in the test,
  not by a drag, stores `V₀` and `E₀` as `−∞` (checked on their bits); after Rotate 15°, the exact case set and a drag
  off it restore `ρ` at 15°, with `√I` and the lab-frame momenta continuous; controls: a drag clamped to the upper hemisphere, a snap to the twin, a drag that mirrors the held
  momentum, a drag that re-places the IC at `I = 1` in the canonical frame, a far press that moves the IC, a press on
  the point that orbits, a drag that refuses the landmark, a finite sentinel for `V₀` in the exact case, and a drag off
  the exact case that resets `ρ` to `+x`, each fail (REQ-GUI-343).
- `cargo test -p gui constructor_twin` and `cargo xtask screenshot 05_inspectors` (the constructor's twin) —
  REQ-GUI-344's verify: with the point on each hemisphere the marker is at `(θ, π − φ)` and the note reads "mirror twin
  of the system at `(θ, π − φ)`" with the twin's angles; the twin's readout is `Π·x`, its `L_z` the IC's negated; with
  held `p_y ≠ 0` a drag onto the marker gives a state whose `L_z` is not the negated one and the readout does not label
  it the twin; a drag passes over the marker without stopping; on the equator the marker coincides with the point;
  controls: a configuration-only twin (`L_z` unchanged), a marker at the point off the equator, a drag that stops at the
  marker, and a readout calling the dragged state the twin when `p_y ≠ 0`, each fail; the screenshot shows the marker's
  place and faintness and the note (REQ-GUI-344).

## Notes
- R-544 item 9, the first addendum and the plain summary's second version; the R-369 decision "The constructor": it is
  the Inspector window's pane 2 sphere (render_gui_spec §G8), not a new window. REQ-GUI-060's "lower half shaded
  mirror-folded" goes with it (TASK-M8-15).
- The note's text is the human's (the first addendum: "with the inspector's "mirror twin of …" note"), completed by the
  instruction's §2 item 6 ("mirror twin of the system at `(θ, π − φ)`").
- The twin is the IC reflected, positions and momenta, which ends the same way (render_gui_spec §G8); its shape sits
  at `(θ, π − φ)`. The drag holds the momenta in the frame `ρ̃` on `+x`, unmirrored, as the chart holds `p_fixed`
  (TASK-M2-43), so dragging the point onto the marker reaches the twin only when the held momenta have no
  `y`-component (REQ-CHART-082's equivariant case); otherwise it reaches a different system, and the marker still
  shows the twin, not the drag's target.
- **Review round 1 of PR #273 (applied per R-369).** The hit radius and the orbit/pan split (gui G1), what a drag holds
  (G2; the human's "Dragging across the equator flips the triangle in the triangle view" is read as a change of
  shape, the gauge buttons' Scale, Rotate and Boost surviving a drag), the drag arms moved to `constructor_twin` (G3),
  the collision landmarks (G5), physics and PIT-9 added (G6), and the twin on the full state (physics P5).
- **Review round 2 of PR #273 (physics review 5603529679, P8; applied per R-369).** The landmark IC comes from the
  decode (dd_decoder §3.6, R-436, R-60), not the lookup path (REQ-ENC-019, REQ-ENC-033, cited before); `V₀` and `E₀`
  are `−∞`, not "no non-finite value"; the frame at `ρ̃ = 0` is a definition under R-72.
- **Review round 3 of PR #273 (physics review 5603638748, P10; applied per R-369, its option (ii)).** Neither drag
  surface reaches an exact `n̂ = (−1, 0, 0)`, so the dragged landmark accepts `V₀` and `E₀` as `−∞` or finite and
  asserts the `t = 0` collision; the `−∞` arm, the frame arm and the finite-sentinel control are on the exact case,
  set through the IC's own setter in the test. No snap, which "never a snap" rules out.
