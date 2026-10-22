# TASK-M7-54 — The Print graph's data model and registry: `principia.print/1`, settings and `$` references, presets as nodes, typed ports, the registry checked at build time, and its own codegen (R-486, R-492)

- **Milestone:** M7
- **Closes:** REQ-RENDER-101, REQ-RENDER-102, REQ-RENDER-103, REQ-RENDER-104
- **Depends on:** TASK-M7-03, TASK-M7-11
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, gui, perf
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
The Print graph as data and its node registry, before any node is ported: the design doc §4.1's format (`format`, `name`, `settings`, `nodes`, `out`; `$name` and `$name+k` references; `preset:<name>` as a sub-graph; editor positions outside it; the one canonical serialisation), §4.2's five port types with typed defaults, and §4.3's registry, each entry with its unit-tagged parameters, WGSL passes, cost class, motion policy, `temporally_stable`, `screen_medium` and listed deviations, checked at build time (§11 test 8). The registry generates the Print graph's own WGSL; the stain's codegen never sees it. The Render node's category is "Print source". The `render` (Render) and `output` nodes are registered here, so a graph of the two runs once the executor lands.

## References
- `docs/design/principia_print_graph.md` § "1. What it is"
- `docs/design/principia_print_graph.md` § "4.1 A graph is data"
- `docs/design/principia_print_graph.md` § "4.2 Port types"
- `docs/design/principia_print_graph.md` § "4.3 Node types are a registry"
- `docs/design/principia_print_graph.md` § "11. Tests (each with an arm that can fail)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/gui/principia_render_gui_spec.md` § "12. Display stage — global, outside the pipeline"
- `decisions.md` § "R-485 — Post-processing is the Print graph, a node graph after the stain and separate from it; it replaces the Display window's "Style", and R-130's v1 styles become presets of it, still available *(supersedes R-130)*"
- `decisions.md` § "R-486 — The Print graph's node set is the lab's, all 78 node types and all 103 presets, ported from GLSL to WGSL; its data format is the Python prototype's, as the design doc §4 merges them *(supersedes R-130, with R-485)*"
- `decisions.md` § "R-492 — Fidelity to the lab is measured per node and per preset against the bundled reference renders; a deviation the design requires (units, seeding, real fields for the lab's stand-in) is listed in the node's registry entry, never tuned to match; the design doc's §11 tests, each with an arm that can fail"
- `decisions.md` § "R-525 — The Viewing slot is off by default while the Medium is a screen medium, and the panel shows the reason *(closes RQ-389)*"
- `decisions.md` § "R-558 — The state map's twenty entries are ruled: any chart may be a target, its inverse generated beside the view's; a tilted target the branch nearest the slice; never a criterion input; the vMF colouring kept and stated not one-to-one; the nominal copy; an ended run held; no history fields; the prototype's constants the starting values; Chart position a named seam-6 exception, with no fixed t; Replace-L's chroma scaling an option, "keep in gamut", off by default; Shape n̂ stays `vector`; the code's reading of the mirror and the clamp; the file's preset names and the trace's fade; "Edit stain…"; no figure ring under the trace; one colour node; `α_min = 0`; "none" a setting of the node *(closes RQ-480 to RQ-489 and RQ-496 to RQ-505; amends R-542)*"

## Deliverables
- `crates/render/src/print/` — `graph.rs` (the model, parse, `$` resolution, sub-graph expansion, canonical serialisation), `ports.rs` (the five types and wire validation), `registry.rs` (entries and the build-time check), `codegen.rs` (WGSL from the registry's passes).
- `crates/render/src/print/nodes/` — the Render (`render`, "Print source") and `output` entries.
- `build.rs` (or an xtask check run by the build) failing on an invalid registry or preset, naming it.
- Tests `print_graph_model`, `print_port_types`, `print_registry_entries`, `print_registry_check`.

## Acceptance tests
- `cargo test -p render print_graph_model` — REQ-RENDER-101's verify (§4.1's Linocut graph parses and serialises back bitwise; "$depth" and "$seed+2" resolve to the setting and the setting plus 2; `{ "type": "preset:Polaroid" }` expands to the Polaroid graph as a sub-graph; two equal graphs whose node ids differ only in order serialise equal; controls: a graph carrying editor positions is refused, a "$missing" reference fails naming the setting, and a preset that includes itself fails naming the cycle).
- `cargo test -p render print_port_types` — REQ-RENDER-102's verify (an `image` → `direction` wire is refused with both ports and types named; an unconnected `ground` input of `linocut` takes `paper` and an unconnected `field` input `white`, as the lab's defaults declare; control: a validator that accepts every wire passes the mismatched fixture and fails the test).
- `cargo test -p render print_registry_entries` — REQ-RENDER-103's verify (every registered entry carries each declared field, the Render node's category reads "Print source" and no Print graph category reads "Source" or "post"; the generated WGSL of a two-node graph compiles under naga and holds each node's passes; controls: an entry without a motion policy, and one without `temporally_stable`, each fail to register, and the stain's codegen given a Print graph node refuses it).
- `cargo test -p render print_registry_check` — REQ-RENDER-104's verify (the shipped registry and presets pass; controls: a fixture preset naming `linocutt` fails naming it, a fixture setting `linocut.depth` outside its range fails naming the preset, node and parameter, and a length parameter registered without a unit fails).

## Notes
- The lab's catalogue (`docs/gui/reference/print_graph/catalogue/node_catalogue.json`, 78 types) is the porting checklist, and the Python prototype (`reference/fx_prototype/`) the reference for the format; both are reference, not code to build on (R-485).
- Cost classes here are provisional: TASK-M7-68 measures and replaces them (REQ-PERF-096; applied per R-369).
- Reviewers (R-485, the human's point 5): the gui and perf reviewers for the Print graph, with code and qa as always; the physics reviewer for the field inputs only.
