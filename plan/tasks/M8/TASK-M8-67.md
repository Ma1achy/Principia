# TASK-M8-67 — The initial-condition constructor's shape sphere: every point of both halves reachable, the drag across the equator flipping the triangle, and the mirror twin shown as a faint marker with its note (R-544)

- **Milestone:** M8
- **Closes:** REQ-GUI-343, REQ-GUI-344
- **Depends on:** TASK-M8-15, TASK-M2-43
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
R-544 keeps two shape spheres apart (the plain summary). The initial-condition constructor's sphere, the Inspector's
pane 2 sphere beside the triangle and the bodies (render_gui_spec §G8; TASK-M8-15 builds the pane), is the one place
where the mirror is information: every point of both halves is reachable, nothing blocked off; its point is the IC's
signed `n̂`, and dragging it sets the IC's shape the way the shape sphere chart places one (the masses and the momenta
in the frame `ρ̃` on `+x` held), so a drag across the equator flips the triangle's orientation in pane 1's triangle
view. The current point's mirror twin is drawn as a faint marker at `(θ, π − φ)`, with the inspector's note "mirror
twin of the system at `(θ, π − φ)`": shown, never enforced, never a snap or a limit.

## References
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `docs/gui/principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)"
- `docs/design/principia_trajectory_viewing.md` § "4. Click inspector — the three-panel absolute surface"
- `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"

## Deliverables
- In `crates/gui/src/windows/inspector/pane_canonical.rs` (TASK-M8-15's file), the sphere view's point draggable over
  the whole sphere, a drag setting the IC's shape through the chart's decode of the dragged `n̂` (TASK-M2-43's signed
  decode), the masses and the held Jacobi momenta kept; pane 1's triangle view following it.
- The twin marker at `(θ, π − φ)`, faint, and the inspector's note "mirror twin of the system at `(θ, π − φ)`" with the
  twin's angles.
- Tests `constructor_sphere_reach` (`crates/gui`) and screenshot cases `05_inspectors/constructor_upper`,
  `05_inspectors/constructor_lower` and `05_inspectors/constructor_equator`.

## Acceptance tests
- `cargo test -p gui constructor_sphere_reach` and `cargo xtask screenshot 05_inspectors` (a lower-hemisphere point
  and its triangle) — REQ-GUI-343's verify: drags to points of both hemispheres, the poles and the equator included,
  set the IC's `shape_vec` to the dragged point within REQ-CHART-053's bound; a drag from `(θ, φ)` to `(θ, π − φ)`
  flips the triangle's orientation (its signed area changes sign) and keeps the masses and the held Jacobi momenta;
  controls: a drag clamped to the upper hemisphere, a snap to the twin, and a drag that mirrors the held momentum,
  each fail (REQ-GUI-343).
- `cargo xtask screenshot 05_inspectors` (the constructor's twin) — REQ-GUI-344's verify: with the point on each
  hemisphere the faint marker sits at `(θ, π − φ)` and the note reads "mirror twin of the system at `(θ, π − φ)`" with
  the twin's angles; a drag never snaps to the marker; on the equator the marker coincides with the point; controls: a
  marker at the point itself off the equator, and a drag that stops at the marker, each fail (REQ-GUI-344).

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
