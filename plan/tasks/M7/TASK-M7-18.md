# TASK-M7-18 — The §7.1 map presets and their golden-image suite

- **Milestone:** M7
- **Closes:** REQ-COL-029, REQ-COL-030, REQ-COL-046, REQ-VAL-096, REQ-COL-052
- **Depends on:** TASK-M7-07, TASK-M7-09, TASK-M7-10, TASK-M7-17
- **Needs (earlier milestones):** REQ-COL-001
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-8, PIT-3
- **Size:** ~500 lines (presets as data; images are fixtures)

## Goal
Every map of colour_composition §7.1 — Artefact 1 (VMF OKLAB, VMF Okabe–Ito, the nine LUT spheres, Turbo, direction cosines) and Artefact 2 (Octant, Voronoi 6, Hemispheres, Icosahedral, Soft Voronoi, Fibonacci and dot lattices, checkerboard, latitude/longitude stripes, Truchet, grid overlay ε = 0.04, iso-hue contours, gradient magnitude, 4-octave value noise, checker + VMF, real spherical harmonics ℓ ≤ 3, Turing standing waves, custom N-pole, basin blend, physics overlay κ = 11 BC / 9 Euler-Lagrange) — and every debug view exists as a composition preset implementing its row's definition, with no Stability × Hue (R-76). Each ships with a golden-image test against its render in the two reference HTML files, `docs/gui/reference/principia_colour_explorer.html` and `principia_colour_presets.html`, named the oracle in colour_composition §7 (R-122); invalid samples render REQ-COL-055's hatched pattern (R-132); the agreement tolerance is proposed with per-map evidence (R-71).

## References
- `docs/design/principia_colour_composition.md` § "7. Preset table & golden-image obligation"
- `docs/design/principia_colour_composition.md` § "7.1 The complete map list (R-16)"
- `decisions.md` § "R-16 — The colour PDF's map lists are ported *(closes RQ-14)*"
- `docs/design/principia_dd_colouring.md` § "3.2 The vMF engine"
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `decisions.md` § "R-76 — Stability × Hue is deleted *(closes RQ-27)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/design/principia_dd_colouring.md` § "3.4 Physics overlay (blob blend) and the house encoding (stability × hue)"

- `decisions.md` § "R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*"
- `decisions.md` § "R-132 — The R-71/R-72 classification is accepted, with three changes *(closes RQ-110)*"
- `decisions.md` § "R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*"
## Deliverables
- `crates/render/presets/maps/` — one preset per §7.1 entry.
- `crates/render/tests/map_formulas.rs` — formula-level tests for the closed forms (Octant index, Voronoi argmax, icosahedron vertices, Fibonacci lattice, dot radius cos(1.4/√N), checkerboard, stripes, Truchet hash, grid ε, gradient magnitude, noise normalisation, the Yℓm forms, the Turing triple, N-pole ring, basin-blend t).
- `fixtures/golden/colour-maps/` — the reference renders, one per §7.1 entry, captured from the two reference HTML files (R-122), and the `colour-maps` golden suite.
- `fixtures/gates/colour-golden-tolerance/` and the gate producing the per-map difference table.
- A generated checklist mapping each §7.1 row and each debug view to its preset file.

## Acceptance tests
- Review (code, qa): every §7.1 entry and every debug view maps to a preset in the library, per the generated checklist (REQ-COL-029).
- `cargo xtask golden colour-maps` and `cargo test -p render map_formulas` — each map vs its render in the reference HTML files within tolerance REQ-COL-052 (calibrated); the closed forms pass their formula tests; LUT samples match the published tables (R-122); the preset library has no Stability × Hue entry (REQ-COL-030).
- `cargo xtask golden colour-maps` — one golden image per §7.1 map, none for Stability × Hue; invalid samples render the REQ-COL-055 pattern (R-132) (REQ-COL-046).
- `cargo xtask golden colour-maps` — per preset, render vs the reference HTML render within tolerance REQ-COL-052 (calibrated); the suite fails while any §7.1 entry lacks a preset or a passing test (REQ-VAL-096).
- `cargo xtask gate colour-golden-tolerance` — the proposal shows per-map pixel differences between the composition engine and the reference HTML renders over the §7.1 list, and the tolerance with its margin; confirmed by the human at the M7 gate (REQ-COL-052).

## Notes
- Calibration (R-71): REQ-COL-052; the same tolerance serves the pre-release fragment golden diffs (parity §6).
- PIT-8: the invalid pattern must stay distinguishable from any map's data colours (R-132: it collides with no palette entry).
- RQ-101 ruled: R-122 — the two reference HTML files are the oracle (colour_composition §7 conformed in step 7); LUT data from the published matplotlib tables and Moreland's cool-warm table, the Principia stops from the explorer (TASK-M7-07). This settles both gaps.
- Gap: Turbo's table source is not named by R-122.
