# TASK-M5-20 — SSAA colour resolve: colour per sample, average last, no interpolation

- **Milestone:** M5
- **Closes:** REQ-RENDER-048, REQ-RENDER-049, REQ-RENDER-050, REQ-RENDER-054, REQ-COL-007, REQ-SYS-037, REQ-SCHED-052
- **Depends on:** TASK-M5-16, TASK-M1-15
- **Needs (earlier milestones):** REQ-RENDER-004, REQ-RENDER-005, REQ-RENDER-009, REQ-RENDER-018, REQ-COL-001, REQ-COL-005
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~380 lines

## Goal
Each sample is coloured independently through the full stain pipeline and a render-side resolve pass averages
a footprint's E+1 sample colours into the pixel colour; no data value is averaged before colouring and no category is
interpolated. The resolve is display-only and terminal: nothing reads its output as data, and Benettin shadows are never
coloured or pooled. One sample rasterises to one tile; sharpness comes only from subdividing quads; the only
sample → pixel combining is the SSAA resolve (plus the transient blurred-ancestor state). FTLE anti-aliases per copy;
spread is one value per footprint shared by its copies.

## References
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `docs/notes/principia_sampling_msaa_note.md` § "The core move: colour per sample, average last"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_temporal_architecture_note.md` § "Still open (settle at implementation, or next edit pass)"
- `docs/notes/principia_sampling_msaa_note.md` § "Ensemble copies ARE the SSAA samples"
- `docs/notes/principia_sampling_msaa_note.md` § "The two firewalls (unchanged, restated for this model)"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/design/principia_memory_tiers.md` § "2. Two resolutions — display and internal-render (sample density is not a separate knob)"
- `docs/design/principia_memory_tiers.md` § "1. Taxonomy (locked vocabulary)"
- `docs/notes/principia_sampling_msaa_note.md` § "The point-quantity vs footprint-quantity distinction (intrinsic, not an optimisation)"
- `docs/contracts/principia_gui_state_contract.md` § "4. The occupant model — typed by signature, free inside"
- `docs/contracts/principia_gui_state_contract.md` § "8. Amendment to the colouring drill-down"
- `docs/notes/principia_sampling_msaa_note.md` § "Uniform samples: every sample is a full, normal SimState"
- `decisions.md` § "R-88 — What stops in-view refinement *(closes RQ-39)*"

## Deliverables
- `crates/render/src/resolve.rs` and the WGSL resolve pass.
- Golden suites in `fixtures/golden/ssaa/`: 75/25 class blend (dd_colouring test 8), states 1/3 straddle, flat
  per-sample blocks above the screen floor, FTLE vs spread edges.

## Acceptance tests
- `cargo xtask golden ssaa` — dd_colouring unit test 8: a synthetic quad of 75% escape / 25% bounded samples renders the 75/25 blend of the two class colours, never a colour from averaged class indices (REQ-RENDER-048).
- Review checklist (code) — no buffer written by the resolve pass is bound as input to any compute or reduction pass (REQ-RENDER-049).
- `cargo xtask golden ssaa` — a quad above the screen floor renders as flat per-sample blocks (no interpolation); no TILE_PIXEL_RES / spp knob exists (REQ-RENDER-050).
- `cargo xtask golden ssaa` — FTLE colouring shows blended edges; spread colouring is constant across a footprint's copies (REQ-RENDER-054).
- `cargo xtask golden ssaa` — a pixel straddling states 1 and 3 renders the average of their colours, never state 2's colour (REQ-COL-007).
- Review checklist (code) — resolve pool indices exclude shadow state; no data reads of resolve output (REQ-SYS-037).
- Review checklist (code) — no code path upsamples a sample across multiple pixels other than the display scale; AA only from ensemble copies (REQ-SCHED-052).

## Notes
- The palette used by the golden images is M1's baseline outcome palette; M7's colour work leaves these fixtures'
  structure (blend, flat blocks) unchanged.
