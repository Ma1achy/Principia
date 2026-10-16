# TASK-M7-36 — The Chazy fields: chazy_final, chazy_status, chazy_structure, chazy_q, chazy_escaper and chazy_qk, the escape freeze, and the energy certificates

- **Milestone:** M7
- **Closes:** REQ-PAY-100, REQ-PAY-101, REQ-PAY-102, REQ-PAY-103, REQ-PAY-104, REQ-PAY-107, REQ-PAY-108, REQ-EVT-027, REQ-VAL-185, REQ-VAL-186
- **Depends on:** TASK-M7-35, TASK-M3-12, TASK-M3-22, TASK-M3-27
- **Needs (earlier milestones):** REQ-EVT-005, REQ-EVT-006, REQ-GEN-016, REQ-VAL-034
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-10
- **Size:** ~500 lines (six accessors and the folds ~220, the structure definition in the drill-down, the certificate and Burrau runs ~150, tests ~130)

## Goal
The read-side fields the stains read, each derived at read from `chazy_class` and the live state (R-442; the Chazy
drill-down §5): `chazy_final` folds the ten labels onto Chazy's seven plus collision (§5.3: transient, excursion,
B-stable and B-so-far to B), `chazy_status` onto §5.4's seven values, `chazy_structure` onto dispersal, hierarchy,
interacting and oscillatory, and `chazy_q` gives §5.5's `K/|U|`. Each fold is exhaustive over the label enum. Where
several structure rows apply, the precedence is written into §5.5 (R-72). An escape-terminated sample is classified on
its fire-time state (§7). The energy certificates hold per pixel (§2), and the Burrau run ends HE (§12.2).
R-443 adds two per-body fields (§5.6): `chazy_escaper`, the body most separated from its pair (the lowest index on a
tie), and `chazy_qk[k]`, body k's `K/|U|` against the other two's centre of mass, both from the live state and the
`ICDescriptor`'s masses, nothing stored.

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "2. Chazy's classes"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.2 The fields"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.3 `chazy_final`: the labels folded onto Chazy's seven"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.4 `chazy_status`"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.5 `chazy_structure` and `chazy_q`"
- `docs/design/principia_dd_chazy_final_motion.md` § "5.6 `chazy_escaper` and `chazy_qk` (R-443)"
- `docs/design/principia_dd_chazy_final_motion.md` § "3.1 Inputs and definitions"
- `docs/design/principia_dd_chazy_final_motion.md` § "7. Validity and styles"
- `docs/design/principia_dd_chazy_final_motion.md` § "9. Gaps between the prototype and Principia"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.5.2 Structure × energy"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.2 New, for Principia"
- `decisions.md` § "R-103 — Escape ends the production loop; the §2.4 checks run in the harness *(closes RQ-63 and RQ-69)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `decisions.md` § "R-443 — The one-body views of the Chazy stains are added as stains: the lens body is a stain parameter, and mass rank is offered alongside label, with a tie fallback"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"

## Deliverables
- The `chazy_final`, `chazy_status`, `chazy_structure` and `chazy_q` read-side accessors, generated beside
  `chazy_class`, each fold an exhaustive match, collision-terminated samples (pair, triple, t = 0) to collision.
- The drill-down's §5.5 gains the structure rows' precedence and the `q` of each overlapping case (REQ-PAY-104).
- The escape freeze: an escape-terminated sample's fields read its fire-time state.
- The `chazy_escaper` and `chazy_qk[k]` read-side accessors (R-443), their `from` naming the live state and the
  `ICDescriptor`'s `m0 m1 m2`.
- The per-pixel energy-certificate run and the Burrau gate.

## Acceptance tests
- `cargo test -p kernel chazy_fold_final` — the fold table: every label to its §5.3 class, transient to B, a collision-terminated sample to collision; a negative control adds an unmapped label and fails to build (REQ-PAY-100).
- `cargo test -p kernel chazy_fold_status` — every label to its §5.4 status, transient to transient, collision to collision, exhaustive (REQ-PAY-101).
- `cargo test -p kernel chazy_structure` — on the §12.1 fixture states, H/HP/P dispersal, HE/PE/excursion hierarchy, transient interacting, an OS-candidate oscillatory, and each precedence case as §5.5 now defines it (REQ-PAY-102).
- `cargo test -p kernel chazy_q` — each row's `q` equals its hand-computed `K/U` (outer ratios 2, 1, 0.5) to f64 rounding; for an interacting state `q > 1` exactly when E > 0 (REQ-PAY-103).
- Review (physics): §5.5's precedence and overlapping `q` are written, consistent with §10.5.2's cells, and REQ-PAY-102's test uses them (REQ-PAY-104).
- `cargo test -p kernel chazy_escaper` — on `binary_plus_escaper(1, 0, 50, 2)` the escaper is 2, and on the same state relabelled it is the escaper's new index (0 and 1 among them); it equals `classify`'s hierarchy body on every §12.1 fixture; an equilateral state at rest gives 0, the lowest index on a tie; control: a state whose most separated body is 1 gives 1 (REQ-PAY-107).
- `cargo test -p kernel chazy_qk` — on `binary_plus_escaper(1, 0, 50, q)`, q = 2, 1 and 0.5, `chazy_qk[2]` equals q to f64 rounding; on every HE and PE fixture `chazy_qk[chazy_escaper]` equals `chazy_q`; every value ≥ 0 (REQ-PAY-108).
- `cargo test -p kernel chazy_escape_freeze` — a fixture run to escape fire: `chazy_class` at every later playhead equals its value at the fire step (REQ-EVT-027).
- `cargo test -p validation chazy_energy_certificates` — property test: a free-fall chart and an E > 0 chart at t = 10, 50, 200 and the horizon on CPU-f64; no free-fall pixel reads H, HP, P or transient, no E > 0 pixel reads B-so-far, B-stable or OS-candidate; control: a fixture with the energy's sign flipped reaches a forbidden label (REQ-VAL-185).
- `cargo xtask gate chazy-burrau` — the Burrau trajectory classified every step to its horizon: the last label HE, no step H, HP or P (REQ-VAL-186).

## Notes
- REQ-EVT-027 waits on RQ-313 (escape-detector alignment); the freeze is built as the spec gives it, and the ruling
  may add a gate. REQ-VAL-185 waits on RQ-318 for the E > 0 B-stable case only.
- Collision reaches these fields from the terminal latch (TASK-M3-17); the colour is TASK-M7-37's.
- `chazy_escaper` and `chazy_qk` (R-443) extend this task, not a new one (applied per R-369): two accessors beside
  the others, reading the same `out_kup`/`pair_kup` quantities. Which body the one-body views read as escaping is
  RQ-321, a question for the views (TASK-M7-41), not for the field.
