# TASK-M5-26 — Compositor: the stale backdrop, blur as the one signal, never blank

- **Milestone:** M5
- **Closes:** REQ-RENDER-037, REQ-RENDER-038, REQ-RENDER-041, REQ-RENDER-044, REQ-RENDER-045, REQ-SYS-030, REQ-RENDER-079
- **Depends on:** TASK-M5-20, TASK-M5-25
- **Needs (earlier milestones):** REQ-RENDER-005, REQ-RENDER-009, REQ-SCHED-008
- **Reviewers:** code, qa, perf
- **Pitfalls:** none
- **Size:** ~450 lines

## Goal
A compositor above the stain pipeline owns the layers (backdrop, fresh cover, overlays). On an identity change
the previous content stays up as the live stale layer — rendered from cached frozen states at the current render config,
re-coloured, not animated — blurred by a separable pass on its texture and composited under the fresh quads, falling
back to a GPU-side snapshot copy only when no payloads exist. Blur is the single signal for "not current" (spatially
stale, behind the playhead, arriving): no desaturation, no tint. Every screen pixel is covered by a layer every frame.
Nothing reads the backdrop as data.

## References
- `docs/contracts/principia_caching_contract.md` § "Part 5 — The stale backdrop: blur means loading"
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- `docs/contracts/principia_canonical_spec.md` § "5. Temporal & rendering model *(authoritative: `temporal_architecture_note`, `render_contract`, `scheduler_contract`, `checkerboard_contract`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_temporal_architecture_note.md` § "Staleness semantics strengthen (a free win)"
- `docs/design/principia_temporal_architecture_note.md` § "The frame loop — the one genuinely new object (and what makes lockstep clean)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_temporal_architecture_note.md` § "Refinement must be a live-to-live handoff (or it looks broken)"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/src/compositor/{layers.rs, blur.rs, snapshot.rs}` and the WGSL blur pass.
- Golden suites `fixtures/golden/backdrop/` (tilt fixture; palette change recolours the backdrop; chroma equality) and
  `fixtures/golden/blur-signal/` (stale, catching-up and live regions).
- Property test `crates/engine/tests/never_blank.rs`: scripted pan/zoom/refine, full coverage every frame, no blocking.

## Acceptance tests
- `cargo xtask golden backdrop` — tilt gesture fixture: frames show blurred old content under fresh quads; palette change recolours the backdrop (REQ-RENDER-037).
- `cargo xtask golden backdrop` — backdrop chroma equals the unblurred render's chroma (REQ-RENDER-038).
- Review checklist (code) — snapshot path uses copyTextureToTexture (REQ-RENDER-041).
- `cargo xtask golden blur-signal` — scene with stale, catching-up and live regions: only live regions render sharp (REQ-RENDER-044).
- `cargo test -p engine never_blank` — during scripted pan/zoom/refine every screen pixel is covered by a layer every frame and the main thread never blocks (REQ-RENDER-045).
- Review checklist (code) — no path reads the screen/backdrop texture into data; hover over backdrop shows no trace (REQ-SYS-030).
- Proposal: the backdrop blur radius with captures and pass cost as evidence; the human confirms it at the M5 gate (REQ-RENDER-079).

## Notes
- The blur kernel's radius is not given by caching Part 5 (see Gaps).
- The worker hosting of REQ-RENDER-045 ("the frame loop runs in the wasm-engine worker") is realised in M8; see
  TASK-M5-24's note.
- Waits on RQ-99 (`REVIEW_QUEUE.md`): M5 requirements that need M6, M7 or M8.
- Closes, for gaps the corpus leaves open: REQ-RENDER-079 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
