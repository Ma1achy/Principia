# TASK-M8-57 — Deep zoom in repeats and far from the root, and seams between images with a shared edge (R-537 EX8) ⚑

- **Milestone:** M8
- **Closes:** REQ-TOOL-182, REQ-RENDER-100
- **Depends on:** TASK-M8-56, TASK-M6-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
Deep zoom works in the extension and in repeats (R-537 EX8): in a repeat the zoom is on the canonical quad, its f64 centre, half-width, `x₀` and `J_D` the canonical ones; on an affine extension far from the root the quad's f32 centre is taken relative to the root cell's own origin, held in f64; and two adjacent images meet with no background pixel between them at any zoom.

## References
- `decisions.md` § "R-537 — The chart's extension past `[0,1]²` is reached by pan and zoom with the controls shown, not only under F3: wherever the view lands it renders with no gaps, every interaction works there, a pixel in a repeat resolving to its canonical system, and the quadtree, refinement and deep zoom work in extended and wrapped regions, one system computed once ⚑ *(amends R-406, R-407 and R-472)*"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"

## Deliverables
- The canonical quad's deep-zoom uniforms for a repeat, the root-cell anchor for the f32 centre, and the shared-edge rasterisation of image seams, in `crates/kernel/` and `crates/render/`.
- Tests `deep_zoom_extension` and `image_seams`.

## Acceptance tests
- `cargo test -p kernel deep_zoom_extension` — REQ-TOOL-182's verify (R-537 T9, T10).
- `cargo test -p render image_seams` — REQ-RENDER-100's verify (R-537 T11).

## Notes
- **⚑ Physics review is required**: the precision in repeats and far from the root; it does not merge on the code reviewer's word alone.
- **The anchor** is the root cell's own origin, not a re-set plane anchor, which would re-address the cache (R-97; R-537 U12).
