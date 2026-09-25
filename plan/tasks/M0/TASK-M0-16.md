# TASK-M0-16 — The contract surfaces' skeleton in the engine crate, and the vocabulary lint

- **Milestone:** M0
- **Closes:** REQ-SYS-002, REQ-SYS-003
- **Depends on:** TASK-M0-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
`crates/engine` declares the typed surfaces the contracts name, with no behaviour: `SimConfig` (the sim key), `RenderState` (the render key), `ViewUI` (pure UI, never read by the engine), `SetField` (the typed edit, `setField(path, value)`) and the GUI-sized snapshot, each with the field groups gui_state_contract §2 lists. With the first vocabulary-bearing code in the tree, `cargo xtask lint vocab` fails CI on any retired term of canonical_spec §8 or temporal note "The rename" (`TileID`, `computeTile`, `samples_per_tile`, the `M` checkpoint count, a `TIMEOUT` state, `sd_is_untrusted`, "N ensemble shadows", the `Math.fround` `N_sub` rule, a TS layout constant generating WGSL, `SimResult`) and on any identifier outside memory_tiers §1's locked taxonomy (`SAMPLES_PER_TILE_AXIS`, `TileReduction`, `TileSummary`, `TILE_PIXEL_RES`).

## References
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_gui_state_contract.md` § "1. The one-way dependency rule"
- `docs/contracts/principia_canonical_spec.md` § "8. Locked vocabulary"
- `docs/design/principia_temporal_architecture_note.md` § "The rename"
- `docs/design/principia_memory_tiers.md` § "1. Taxonomy (locked vocabulary)"
- `decisions.md` § "R-111 — `SimResult` → `SimState`, `M` → `n_renorm`; the vocabulary lint covers the docs *(closes RQ-80)*"
- `decisions.md` § "R-133 — The seven checkpoint-B interpretations are accepted *(closes RQ-111)*"

## Deliverables
- `crates/engine/src/contract/{sim_config,render_state,view_ui,set_field,snapshot}.rs` — the types, with doc comments citing gui_state_contract §2; fields the corpus names at M0 only, no defaults, no methods.
- `xtask/src/lint_vocab.rs` — `cargo xtask lint vocab` over `crates/`, `xtask/`, `fixtures/`, `web/` and `docs/` (excluding `docs/archive/` and, in docs, any passage wrapped in `<!-- retired-terms -->` … `<!-- /retired-terms -->`, R-111); the term lists come from canonical_spec §8 and memory_tiers §1; registered in `cargo xtask ci`.
- Any retirement passage the lint still flags in `docs/` (beyond those step 7 wrapped for R-111) is wrapped in the `retired-terms` markers in the same PR, as a doc change for review.
- Fixture files containing each retired term and each forbidden taxonomy identifier, and one with a retired term inside the markers.

## Acceptance tests
- `cargo xtask lint vocab` — the lint over code and docs (excluding archive and the `retired-terms` passages) finds none of the retired identifiers; `cargo test -p xtask lint_vocab` — a fixture with each retired term fails naming it, and the same term inside the markers passes (REQ-SYS-002).
- `cargo xtask lint vocab` — no `SAMPLES_PER_TILE_AXIS`, `TileReduction`, `TileSummary` or `TILE_PIXEL_RES` identifier; a fixture with each fails naming it; review checklist (code §7): code uses QUAD, SAMPLE (`SAMPLES_PER_QUAD_AXIS`), TILE, PIXEL and `QuadReduction`/`QuadSummary` (REQ-SYS-003).
- `cargo build -p engine` — the five surfaces compile with no behaviour; `cargo xtask deps` still passes.

## Notes
- The field lists are completed by the requirements that need them (REQ-GUI-036, REQ-GUI-102, M8, and the SimConfig requirements of M2–M6); this task names the surfaces and the §2 groups (R-133: gui_state_contract §2 lists the groups, each group's fields come from the contract that owns them).
- The firewall (engine exposes only these surfaces `pub`) is REQ-SYS-052, M8.
- RQ-80 ruled: R-111 — the docs already read `SimState` in the display chain and `n_renorm` in the uniform echo, and wrap their retirement passages in the `retired-terms` markers (applied in step 7); the lint covers code and docs, excluding the marked passages.
