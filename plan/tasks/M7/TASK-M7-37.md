# TASK-M7-37 — The Chazy flat stains: the seven-class default, four families, Okabe–Ito, the status map, the filter, the ranks and palette separation

- **Milestone:** M7
- **Closes:** REQ-COL-072, REQ-COL-073, REQ-COL-075, REQ-COL-076, REQ-COL-077, REQ-COL-080, REQ-COL-081, REQ-COL-084, REQ-COL-085, REQ-COL-086, REQ-COL-087
- **Depends on:** TASK-M7-36, TASK-M7-08, TASK-M7-17, TASK-M7-18, TASK-M7-20, TASK-M1-14, TASK-M2-34
- **Needs (earlier milestones):** REQ-COL-002, REQ-COL-053, REQ-COL-064, REQ-COL-067, REQ-COL-068
- **Reviewers:** code, qa, gui, physics
- **Pitfalls:** PIT-5, PIT-3
- **Size:** ~450 lines (palettes and presets as data ~150, the separation test ~150, goldens are fixtures, the ranks' definition in the drill-down)

## Goal
The Chazy stains with no lightness encoding, compositions on the backbone (R-442; the Chazy drill-down §10): the
seven-class palette on `chazy_final` as the default final-motion stain, with B "bound, or nobody has left yet"; the
four-family and Okabe–Ito variants; the status map on `chazy_status`, its own map; the categorical filter unchanged.
In every stain collision is magenta `#DA85B6` and B is `#141418`; running samples draw their class, never the "not
yet" stipple, and `sim_failed`, reserved codes and NaN draw `#141418`, B's black, and the decoder's own
`decode_failed` the canvas background, with no key or share (R-511, ruling RQ-379), never the hatch, which only the
debug view draws (§7; R-467, ruling RQ-319). *Was (R-467): "and `sim_failed`, the decoder's own `decode_failed`,
reserved codes and NaN draw `#141418`, B's black, never the hatch". Stale, replaced by R-511.* *Was: "and `sim_failed`, the decoder's
own `decode_failed`, reserved codes and NaN the hatch (§7; RQ-319, decided per R-369)". Stale, replaced by R-467.* Greyscale and hue modes rank `chazy_final` B, OS, PE, HE, P, HP, H, collision, and the other
fields' ranks are written (R-72). The palettes' separation is measured against τ and the palettes proposed for the M7
gate (R-71).

## References
- `docs/design/principia_dd_chazy_final_motion.md` § "7. Validity and styles"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.1 The two rules that hold in every stain"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.2 Final motion: Chazy's seven (the default)"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.3 Final motion: four families"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.4 Final motion: Okabe–Ito"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.6 Status"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.7 Filter"
- `docs/design/principia_dd_chazy_final_motion.md` § "10.9 Greyscale and hue modes"
- `docs/design/principia_dd_chazy_final_motion.md` § "11.2.1 Palette separation, measured"
- `docs/design/principia_dd_chazy_final_motion.md` § "12.2 New, for Principia"
- `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default palette"
- `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants"
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"
- `decisions.md` § "R-442 — Chazy final motion is a separate system with its own stains: hue carries the class, B is black `#141418` and one class, collision is magenta `#DA85B6`, transient draws as B, the working states have their own status map, and the seven-class palette is the default"
- `decisions.md` § "R-511 — A `decode_failed` sample the chart does not classify as not a system draws blank in a presentation view, as a not-a-system pixel, the forbidden-region overlay drawing over it too; `sim_failed` keeps `#141418` *(closes RQ-379; amends R-467 and R-470)*"

## Deliverables
- The seven-class, four-family, Okabe–Ito and status palettes as swatch sets, user-editable, and their presets; the
  seven-class stain the default final-motion preset.
- The validity styles in every Chazy stain; collision `#DA85B6`.
- The ranks of `chazy_final` and, written into the drill-down's §10.9, of `chazy_class`, `chazy_status` and
  `chazy_structure` (REQ-COL-085).
- `cargo test -p render chazy_palette_separation` over the seven-class, four-family, Okabe–Ito, structure and status
  palettes, and the Chazy palettes added to `cvd_hatch_distinct`'s entries.
- Golden images of each flat stain on a synthetic payload, and the calibration proposal for the palettes.

## Acceptance tests
- `cargo test -p render chazy_validity` — through each Chazy stain, a running sample draws its class's colour and no pixel equals `debug_not_yet`'s pattern; `sim_failed`, a reserved `state` code and NaN draw `#141418` on the CPU twin and in WGSL, and a `decode_failed` sample with a `DEGENERATE` tag, a `decode_failed` sample from a non-finite decode and a forbidden-lane pixel draw the canvas background, outside every share (R-511); no pixel equals `debug_invalid`'s pattern; controls: a control stain drawing the failures with `debug_invalid`'s hatch, one drawing the forbidden lane `#141418`, and one drawing a `decode_failed` sample `#141418`, each fail (REQ-COL-072; R-467, R-511).
  *Was (R-467): "`sim_failed`, a `decode_failed` sample with a `DEGENERATE` tag, a `decode_failed` sample from a non-finite decode, a reserved `state` code and NaN draw `#141418` on the CPU twin and in WGSL, and a forbidden-lane pixel draws the canvas background". Stale, replaced by R-511 (gui review 5587650119 of PR #255, G11).*
  *Was (RQ-319 per R-369): "… draw `debug_invalid`'s hatch on the CPU twin and in WGSL; a control stain drawing the `decode_failed` samples black `#141418` fails". Stale, replaced by R-467: the control is flipped.*
- `cargo test -p render chazy_collision` — runs terminated at `r_coll` for each pair, a triple collision and a t = 0 collision read collision in `chazy_final` and `chazy_status` and render `#DA85B6` in every flat stain (REQ-COL-073).
- `cargo xtask golden chazy` — synthetic payloads with one sample per class render exactly the seven-class colours (REQ-COL-075), the four-family colours (REQ-COL-076), the Okabe–Ito colours (REQ-COL-077) and the status colours (REQ-COL-080).
- `cargo test -p render chazy_restyle` — a swatch change re-styles the view and the dispatch counter is unchanged; the default final-motion preset is the seven-class stain (REQ-COL-075).
- `cargo test -p render chazy_filter` — the seven-class stain filtered to {HE, PE} keeps their colours and mutes every other sample by REQ-COL-058's transform; the same on the status stain (REQ-COL-081).
- `cargo test -p render chazy_rank` — greyscale mode gives `chazy_final`'s class `r` the lightness `(r + ½)/8` and hue mode `t = r/8`, in the order B, OS, PE, HE, P, HP, H, collision (REQ-COL-084).
- Review (physics): the drill-down's §10.9 gives the ranks and K of `chazy_class`, `chazy_status` and `chazy_structure`, consistent with colour_composition §4.1, and their greyscale and hue renders use them (REQ-COL-085).
- `cargo test -p render chazy_palette_separation` — prints each palette's tightest pair and tightest style distance unsimulated and under each simulation; status (0.114, 0.111) and four-family (0.110, 0.137) clear τ = 0.101 unsimulated; seven-class asserts H / HP 0.083 and §11.2.1's simulated values, Okabe–Ito asserts P / collision 0.041 unsimulated and under each simulation, the structure hues their §11.2.1 values, and every entry of each is ≥ τ from the styles unsimulated (0.137, 0.121, 0.137); `cvd_hatch_distinct` passes with the Chazy palettes added (REQ-COL-086).
- Review (gui): the palettes' proposal (seven-class, four families, Okabe–Ito, structure hues, status) with REQ-COL-086's measurements, Okabe–Ito's 0.041 P / collision among them, confirmed by the human at the M7 gate, deciding RQ-311 (REQ-COL-087).

## Notes
- R-442: the seven-class palette is the default; whether it stays below τ or moves to Okabe–Ito (whose P sits 0.041
  from collision) is RQ-311, for the human at the gate. REQ-COL-087 waits on it and on RQ-317 (B's chroma); this task
  builds R-442's default meanwhile.
- The time views' dimming and lightness by time are REQ-COL-106, TASK-M7-40's, not this task's (R-71: proposed by the
  task that needs them).
- R-467 (20 Oct 2026) rules RQ-319 and replaces the note below: every failure and non-finite value draws `#141418`,
  which in the Chazy stains is B's black, in every Chazy stain, and a pixel the chart classifies as not a system draws
  the canvas background; the hatch, `debug_invalid`, and the NaN / Inf style appear only in the debug view (TASK-M2-34,
  REQ-COL-109). The note below is kept as the record of the R-369 reading R-467 overrides (decisions.md § R-467, F4).
- *Superseded by R-467:* What the decoder's own `decode_failed` samples (its `DEGENERATE` tags and a non-finite decode) draw is RQ-319,
  restated against R-446 (the outcome view draws them black `#141418`, which here is B), decided per R-369: option
  (a), on gui review 5555494498 of PR #223. They draw the invalid hatch, `debug_invalid`, in every Chazy stain, as
  `sim_failed` does (dd_chazy_final_motion §7); REQ-COL-072 no longer waits on it, and its statement and verify name
  them (`chazy_validity`). A pixel the chart hatches keeps the chart's hatch (chart_decoder_contract Part 3; R-446).
- The filter's mute is REQ-COL-058's (TASK-M7-08); the spec's preview `#3A3B40` is not a second rule (R-442, applied
  per R-369).
- R-467 D13 (code review 5585240188 of PR #253, F4; applied per R-369): this task depends on TASK-M2-34, whose
  render draws the forbidden lane's canvas background that `chazy_validity` (REQ-COL-072) checks.
