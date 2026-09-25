# TASK-M7-19 — The seam-free guarantee and the two rotations

- **Milestone:** M7
- **Closes:** REQ-VAL-097, REQ-RENDER-062
- **Depends on:** TASK-M7-18
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Every map declared continuous (all vMF modes, the LUT sphere, latitude and longitude stripes for integer f, 3-D Cartesian noise) has colour difference → 0 across dense sample pairs straddling the antimeridian and both poles, with the intentionally discontinuous maps excluded by name. `sph_uv` takes the config-space shape normal; the view orbit is display-only and never changes which UV a physics point maps to; pattern auto-rotate is a UV offset inside the colour function.

## References
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `docs/design/principia_dd_colouring.md` § "3.3 Sphere sampling conventions"

## Deliverables
- `crates/render/tests/seam_free.rs` — dd_colouring unit test 2 as a property test over the continuous-map list.
- `crates/render/tests/two_rotations.rs` — the two-rotations property test.
- The auto-rotate UV offset inside the colour function and the display-only view orbit of the sphere widget.

## Acceptance tests
- `cargo test -p render seam_free` (proptest) — dd_colouring unit test 2: for every continuous map, pair differences across the antimeridian and both poles go to 0 as the pair spacing shrinks; exclusion list Octant, Voronoi 6, Hemispheres, Icosahedral, Fibonacci (hard), Checkerboard, Truchet (REQ-VAL-097).
- `cargo test -p render two_rotations` (proptest) — rotating the view orbit leaves the UV of each physics landmark unchanged; auto-rotate changes colour, not the sampled point (REQ-RENDER-062).

## Notes
- "Zero colour difference" is asserted as convergence under shrinking pair spacing (a finite pair of an honest continuous map differs by O(spacing)); the property test states the spacing sequence it uses.
