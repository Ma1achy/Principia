# TASK-M7-47 — Chart position: the source, its projection onto each target's quotient, the four results, the edge clamp, the colour frame and the t = 0 identity (R-542 SM1 to SM5, SM7) ⚑

- **Milestone:** M7
- **Closes:** REQ-COL-113, REQ-COL-114, REQ-COL-115, REQ-COL-116, REQ-COL-117, REQ-COL-118, REQ-COL-119, REQ-COL-120, REQ-COL-141
- **Depends on:** TASK-M7-52, TASK-M1-01, TASK-M1-04, TASK-M2-25, TASK-M2-26, TASK-M7-03, TASK-M7-11
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~900 lines, the tests included

## Goal
The Chart position source: per sample, `colour = C_T(fold_T(P_T(E(x(t)))))`'s point — the encode, the target chart's projection, the fold onto its quotient — at the playhead or `t = 0`, with the four results (on the chart, left, past a bounded edge, undetermined), the edge clamp of SM5a with "none" as an option, "after its end" (hold, outcome colour), and SM7's colour frame. Seam 6 and render contract Part 4 keep the chart from a colour occupant, a source included; Chart position is a named exception to it (R-558, RQ-489 (a)), which this task adds to Part 4's list and the lint's (REQ-COL-141). *Was:* "**It waits on RQ-489**: seam 6 and render contract Part 4 keep the chart from a colour occupant, a source included." **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout. The field is derived at read in the fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode). If anything turns out to need a stored field, stop and queue it in `REVIEW_QUEUE.md`; do not add one (REQ-COL-114).

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM1. Definition"
- `docs/design/principia_state_map.md` § "SM2. It changes no sim struct and reads only what the fragment already has"
- `docs/design/principia_state_map.md` § "SM4. Destination side: why it must be a quotient, not a branch"
- `docs/design/principia_state_map.md` § "SM5. The four results of a projection"
- `docs/design/principia_state_map.md` § "SM5a. Left the chart: drawn at the edge it left by"
- `docs/design/principia_state_map.md` § "SM7. Colour frame and deep zoom"
- `docs/design/principia_state_map.md` § "SM12. Tests (each with the arm that fires)"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules"
- `docs/design/principia_lines.md` § "8. Traces"
- `decisions.md` § "R-558 — The state map's twenty entries are ruled: any chart may be a target, its inverse generated beside the view's; a tilted target the branch nearest the slice; never a criterion input; the vMF colouring kept and stated not one-to-one; the nominal copy; an ended run held; no history fields; the prototype's constants the starting values; Chart position a named seam-6 exception, with no fixed t; Replace-L's chroma scaling an option, "keep in gamut", off by default; Shape n̂ stays `vector`; the code's reading of the mirror and the clamp; the file's preset names and the trace's fade; "Edit stain…"; no figure ring under the trace; one colour node; `α_min = 0`; "none" a setting of the node *(closes RQ-480 to RQ-489 and RQ-496 to RQ-505; amends R-542)*"

## Deliverables
- The Chart position source occupant and its `point⟨q⟩` output (`crates/render/`), each target's projection and fold, generated from the one Rust source where it reads a chart (R-116, R-542 H6).
- Chart position's seam-6 exception, with its reason, in render contract Part 4's list and in `cargo xtask lint wgsl`'s (R-558 S1); test `lint_wgsl_chart_position`.
- Tests `chart_position_status`, `chart_position_identity`, `chart_position_edge_clamp`, `chart_position_four_results`, `chart_position_invariance`, `chart_position_conserved`, `chart_position_frame`.

## Acceptance tests
- `cargo test -p render chart_position_status` — REQ-COL-113's verify.
- Review checklist — REQ-COL-114's verify (the layout diff empty across the PR).
- `cargo test -p render chart_position_identity` — REQ-COL-115's verify.
- `cargo test -p render chart_position_edge_clamp` — REQ-COL-116's verify.
- `cargo test -p render chart_position_four_results` — REQ-COL-117's verify.
- `cargo test -p render chart_position_invariance` — REQ-COL-118's verify.
- `cargo test -p render chart_position_conserved` — REQ-COL-119's verify.
- `cargo test -p render chart_position_frame` — REQ-COL-120's verify.
- `cargo test -p xtask lint_wgsl_chart_position` — REQ-COL-141's verify.
- `cargo test -p render shared_projection_callers` (the Chart position arm) — REQ-COL-135's arm for this consumer: its projection is REQ-COL-134's shared source as generated WGSL, its points within REQ-COL-129's projection agreement tolerance of the native build (R-297's fast-math rules out bitwise), its statuses equal for every state farther from a link margin than that tolerance and either accepted within the band, the band's count reported and its seeds including states just inside and just outside it (R-296's pattern; PR #270 physics re-review P1, applied per R-369, *was:* "its statuses equal (R-297's fast-math rules out bitwise);"); controls: a copied projection with one constant changed, which the test shows differs from the native build by more than that tolerance, fails, and a hand-written projection defined outside `crates/kernel` fails the symbol check, the snippet `cargo xtask build-kernel` emits to `crates/render/wgsl/` exempt. *Was (PR #270 code re-review, C4, applied per R-369):* "`cargo test -p render shared_projection_callers` (the Chart position arm) — REQ-COL-135's arm for this consumer: its projection is REQ-COL-134's shared source (control: a copied projection with one constant changed, and a projection defined outside `crates/kernel`, each fail)."

## Notes
- **RQ-489 is ruled (R-558):** a named seam-6 exception, the playhead and `t = 0` only, no fixed t; this task no longer waits. *Was:* "**Waits on RQ-489** (the chart read from a colour occupant; a fixed t; R-542 H9)."
- **⚑ Physics review is required** on SM1, SM4 and SM5 (the human's message): this task does not merge on the code reviewer's word alone.
- **Ruled by R-558 as applied meanwhile:** RQ-480 to RQ-482 and RQ-485 (a); RQ-498 (a), the code's reading: the shape-sphere target reads the signed n̂ (payload §5's `n`) over both hemispheres, the `t = 0` identity on the sphere modulo the mirror (R-141's gauge; this clause of the RQ-498 ruling is superseded by the human's second addendum to the shape sphere ruling, the identity holding point for point, and is left to that ruling's port), Four results run under "none"; RQ-503 (a), main's `α_min = 0`, the edge-clamp test on the latent momenta; RQ-504 (a), the one-hemisphere view targets the full sphere; RQ-505 (a), "none" an ordinary setting. A fixed t is struck (RQ-489, R-558). *Was:* "RQ-498 (a): the shape-sphere target is the folded hemisphere, Four results run under \"none\"." (PR #270's physics review, P1).
- **The one projection (R-542 H11):** the target projections, the leaving rule and the curve axis's nearest point are TASK-M7-52's Rust source, shared with TASK-M8-58's trace; this task wraps them as the fragment occupant and adds the fold, the statuses' colours and SM7's frame.
- **The tolerances** of the identity are proposed here, measured on the app, as REQ-COL-129's calibration values (R-542 H13).
- The prototype's per-target projection is transcribed in `principia_state_map.md` SM1's table (`smWhere`); its compaction in SM6's.
