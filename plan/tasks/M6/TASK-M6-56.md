# TASK-M6-56 — A typed basis read whole, and zoom scaling the stored basis (R-705)

- **Milestone:** M6
- **Closes:** REQ-GUI-415
- **Depends on:** TASK-M6-40
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
R-705 takes RQ-663's option (c), both, on the owner's "it's a real bug". With X ≠ Y, a typed basis with Y on q₁ or X on q₂ reads with that component ignored: q₁ = 0.8 e_μ1, q₂ = e_β reads σ = 0, an infinite zoom, and the next zoom edit rewrites the plane.

## References
- `decisions.md` § "R-705 — REQ-PERF-115 and TASK-M8-96 name `--release`; a least window size is set, and the layout's minimums are re-measured; a typed basis's off-frame components are counted and zoom scales the stored basis *(closes RQ-657, RQ-662 and RQ-663)*"
- `decisions.md` § "R-457 — The Manifold view's Navigate section loses its grid of eight `z₀` boxes; "Centre z₀" is renamed "Slice centre" and "Slice & tilt" "Tilt"; each tilt angle tilts into a latent dimension the user picks, "tilt τ₁ into [X]" and "tilt τ₂ into [Y]", the defaults today's one shared target; the step slider is removed *(amends render_gui_spec §G2's Navigate and Slice & tilt, R-409 A3's "the slice step" and R-451 B1's single tilt target)*"
- `decisions.md` § "R-459 — The cube's H, V and D axis labels are buttons, each a dropdown of latent dimensions, no dimension twice, each pick one undoable edit, H and V in step with Chart's horizontal q₁ and vertical q₂; in Tilt mode the third axis shows what the gesture tilts into, and a "4D: see Plücker" badge appears when both angles are set into different targets *(amends R-451 B1's labels)*"

## Deliverables
- `crates/gui/src/explore/manifold_view/mod.rs`: `View::framed`'s preset test `outside(a, b)` counts the X and Y components on the wrong vector, so the basis reads in its dominant axes' frame; `View::in_frame` accordingly.
- `crates/gui/src/explore/manifold_view/navigate.rs`: the zoom row scales the stored basis, in place of `basis_edit(view.with_zoom(zoom).basis())`.
- Tests beside the view.

## Acceptance tests
- `cargo test -p gui manifold_view_basis` — REQ-GUI-415's verify, with its controls.

## Notes
- Added by R-705 (4 Nov 2026, RQ-663 (c)).
- It depends on TASK-M6-40, whose PR #366 touches this reading and leaves it as it is.
- A gui task runs its mutants under R-439's limits.
