# TASK-M7-41 — The Chazy one-body stains: the body lens, its energy form, the escaper map, the mass-rank views and their tie fallback

- **Milestone:** M7
- **Closes:** REQ-COL-090, REQ-COL-091, REQ-COL-092, REQ-COL-093, REQ-COL-094, REQ-COL-095, REQ-COL-096, REQ-COL-097, REQ-COL-098, REQ-COL-099, REQ-COL-100, REQ-COL-107, REQ-VAL-188, REQ-VAL-189
- **Depends on:** TASK-M7-36, TASK-M7-38
- **Needs (earlier milestones):** REQ-COL-055, REQ-COL-064, REQ-GUI-188
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** PIT-5
- **Size:** ~600 lines (the role and rank logic and the tie fallback ~200, palettes and presets as data ~100, the separation test ~100, the property tests ~200; goldens are fixtures, the definitions in the drill-down)

## Goal
The views that follow one body (R-443; the Chazy drill-down §10.11), compositions on the backbone like the other
Chazy stains: the body lens, with its lens body `k` a stain parameter (0-based inside, shown and chosen as body 1, 2 or
3), drawing body k's role (escaping hyperbolic or
parabolic, in the pair left behind, flying apart, bound, collision) from `chazy_class` and `chazy_escaper`; the same
roles with `chazy_qk[k]` in the brightness slot under Replace-L; the escaper map, hue by escaping body and the darker
shade for parabolic; and, where masses vary across a slice, the escaper map by mass rank and the lightest- and
heaviest-body lenses, falling back to the label view where masses tie and flagging it for the legend. The palettes'
separation is measured against τ, and the palettes and the tie tolerance are proposed for the M7 gate (R-71).
`chazy_escaper`'s greyscale and hue-mode rank is written into the drill-down (R-72).

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "5.6 `chazy_escaper` and `chazy_qk` (R-443)"
- `docs/design/principia_dd_chazy_final_motion.md` § "7. Validity and styles"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.5 With an energy channel"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.9 Greyscale and hue modes"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11 One body at a time (R-443)"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11.1 Body lens"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11.2 Body lens with energy"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11.4 Escaper map"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11.5 By mass instead of by label"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.11.6 Measured, for the record"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2 Calibrations to confirm at a gate (R-71)"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2.2 One-body palettes, measured (R-443)"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.3 One body at a time (R-443)"
- `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default palette"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `decisions.md` § "R-443 — The one-body views of the Chazy stains are added as stains: the lens body is a stain parameter, and mass rank is offered alongside label, with a tie fallback"
- `decisions.md` § "R-449 — What a person reads is 1-based: the bodies are 1, 2, 3 and a pair is named by its two bodies, never by its pair id, through one conversion function; storage, computation and machine-readable exports stay 0-based and say so *(supersedes R-22 as it applies to display labels; amends R-444, R-445, R-447 and R-448 as they name the classes)*"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"

## Deliverables
- The body-lens palette and the escaper palette (by body and by rank) as swatch sets, user-editable, and their presets:
  the body lens (parameter `k`), the body lens with energy, the escaper map, the escaper by mass rank, and the
  lightest- and heaviest-body lenses. Preset names and stain labels name bodies 1, 2, 3 and the lens parameter is
  chosen as body 1, 2 or 3, through TASK-M2-33's one display conversion (R-449, REQ-GUI-188); `k` and `chazy_escaper`
  stay 0-based inside.
- The mass rank from the `ICDescriptor`'s masses, undefined where a body's mass ties another's within the tolerance;
  the rank views' fallback to the label view and the fallback flag the legend reads (TASK-M8-45).
- The drill-down's §10.11.5 gains the label body a rank lens falls back to and the tie comparison's form
  (REQ-COL-096), and its §10.9 `chazy_escaper`'s rank and K = 3 (REQ-COL-107).
- `cargo test -p render chazy_one_body_separation`, and these palettes added to `cvd_hatch_distinct`'s entries.
- Golden images of the body lens and the escaper map on a synthetic payload, and the calibration proposals for the
  two palettes and the tie tolerance.

## Acceptance tests
- `cargo test -p render chazy_lens_roles` — role table, exhaustive (§12.3 item 1): for every label, collision and every (k, escaper) pair the role and its sRGB are as §10.11.1 tables, OS-candidate bound black; a negative control adds an unmapped label and fails to build; and `cargo xtask golden chazy-lens`, one sample per role at k = 0, 1, 2, renders exactly those colours (REQ-COL-090).
- `cargo test -p render chazy_lens_energy` — energy lens recovers the escape (§12.3 item 5): body k escaping hyperbolically has `chazy_qk[k] > 1` and OKLab L above L(b = ½), a bound k below; control: the flat lens draws two bound samples with different `q_k` identically (REQ-COL-091).
- `cargo xtask golden chazy-escaper` — an HE and a PE sample per escaper body and one sample per other class render exactly §10.11.4's colours, OS-candidate black (REQ-COL-092).
- `cargo test -p render chazy_mass_rank` — mass rank (§12.3 item 4): on a distinct-mass fixture the escaper-by-rank map and the lightest and heaviest lenses equal the label views composed with the rank permutation, pixel for pixel; on an equal-mass fixture they equal the label views exactly with the flag set (REQ-COL-093, REQ-COL-094).
- `cargo test -p render chazy_mass_tie` — a one-tied-pair fixture falls back only where the tie matters (the escaper map where the escaper is a tied body; the lightest lens where the lightest masses tie, the heaviest lens not), the flag set exactly there; masses 2 × the tolerance apart rank, 0.5 × apart do not (REQ-COL-095).
- Review (physics): the drill-down's §10.11.5 gives the rank lens's fallback body and the tie comparison's form, consistent with R-443, and `chazy_mass_rank` and `chazy_mass_tie` use them (REQ-COL-096).
- Review (physics): the drill-down's §10.9 gives `chazy_escaper`'s rank and K = 3, consistent with colour_composition §4.1's rank rule, and its greyscale and hue renders use them; the one-body roles take no rank (REQ-COL-107).
- `cargo test -p render chazy_one_body_separation` — prints each palette's tightest pair and tightest style distance unsimulated and under each simulation; asserts the body lens's 0.084 (HE / PE), `#4158BD` ≥ 0.177 from every entry and style, the escaper palette's 0.102 (body 1 HE / collision), the grey `#8A8A90` against the stipple by R-434's criterion (0.212 from the block's farthest pixel, ≥ τ) and as its dot distance (0.020 measured), every entry ≥ τ = 0.101 from the hatch's colours and, by R-434's criterion, from the stipple, unsimulated; `cvd_hatch_distinct` passes with these palettes added (REQ-COL-097).
- Review (gui): the body-lens palette's proposal with REQ-COL-097's measurements, confirmed by the human at the M7 gate, deciding RQ-322 (the grey's chroma) (REQ-COL-098).
- Review (gui): the escaper palette's proposal, by body and by rank, with REQ-COL-097's measurements and §10.11.6's maps, confirmed by the human at the M7 gate, deciding RQ-322 (REQ-COL-099).
- Review (physics): the mass tie tolerance, relative 10⁻⁶, with `chazy_mass_tie` and the equal-mass presets falling back entirely, confirmed by the human at the M7 gate (REQ-COL-100).
- `cargo test -p render chazy_one_escaping_body` — property test (§12.3 item 2): over a gallery chart at t = 10, 50, 200 and the horizon and every §12.1 fixture, on each HE/PE sample exactly one of the three lenses reads escaping and the other two in the pair, and on every other sample none reads escaping (REQ-VAL-188).
- `cargo test -p render chazy_label_symmetry` — property test (§12.3 item 3): for each of the six permutations of an IC's bodies, `chazy_escaper` maps through the permutation and lens k of the permuted IC equals lens π⁻¹(k) of the original; control: a fixture whose escaper is not body 0; the fixtures keep the largest hierarchy ratio strictly largest (REQ-VAL-189).

## Notes
- The escaping body is `chazy_escaper`, as the block gives it; whether the views read the event's escape detail where
  the run has an escape event is RQ-321, for the human. REQ-COL-090, REQ-COL-092 and REQ-COL-093 wait on it for that
  clause (`rq: RQ-321`); this task builds them on `chazy_escaper` meanwhile.
- `chazy_escaper`'s rank (REQ-COL-107) is this task's because it builds the views that read the field; REQ-COL-085
  (TASK-M7-37) covers the other three fields. The roles are a stain's output, not a field, and take no rank.
- The grey `#8A8A90` clears the stipple by R-434's criterion (0.212) and sits 0.020 from its dot, both asserted as
  measured (REQ-COL-097, which does not wait). Its chroma ≈ 0.009 is RQ-322 (for the human at the M7 gate, RQ-317's
  sibling); REQ-COL-091's neutral-grey clause and the two palette calibrations wait on it. The stains draw `#8A8A90`
  meanwhile, and never draw the stipple (§7).
- OS-candidate draws bound black in the lens and the escaper map, where `chazy_final` draws it OS green: the block's
  reading ("the B and OS folds"), recorded in §10.11.1, not an RQ (applied per R-369).
- The energy form reuses TASK-M7-38's Replace-L range and compaction (REQ-COL-088, REQ-COL-089), hence the dependency.
- The three panels (§10.11.3) and the one-body legends are TASK-M8-45's; this task gives them the stains and the
  fallback flag.
- R-449 (16 Oct 2026): every body name this task shows (preset names, stain labels, the lens parameter's choices) comes from TASK-M2-33's conversion module (REQ-GUI-188), 1-based, never "body 0"; TASK-M2-33 is reached through TASK-M7-36.
