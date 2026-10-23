# Instruction: the gap sweep, its tests and designs, 21 Oct 2026

From me, Malachy. Everything in this bundle is mine to rule and I have ruled it; my words are in
`DECISIONS_2026-10-21.md` and inside each design's "Decided" section. Quote them verbatim wherever you record a ruling.
Record the rulings in `decisions.md` from the next free R-number, grouped as the PRs below, each naming the rulings it
amends and any queue entries it closes.

Native app first throughout. Where anything here conflicts with a ruling or a queue entry these texts don't address,
queue it rather than resolving it. The security rule stands: never delete or modify anything outside the repo and
build/scratch directories without asking.

Supporting evidence: the measurements were run in prin-rs and are on its PR #34 (examples `heggie_stepper_probe`,
`budget_horizon_probe`, `sync_grid_probe`, `parting_probe`), with their outputs in `evidence/`.

## PR 1 — The integrator contract, corrected (`designs/integrator_findings.md`)

The physics PR. Do this one first.

- **Default occupant: Heggie × RK4.** "Yep Heggie x RK4 as default". RK4 is what every Heggie result was measured
  with. REQ-GUI-102's default and the render GUI spec's Run window (§G5) change with it.
- **Every regularisation × stepper pair is allowed, regularisation off included.** A pair measured not to work carries
  a warning beside the selector, in the Problems panel and in each export's settings. My words: "don't forbid combos
  of integrators & regularisers… maybe just have a warning somewhere that they might not work". Each such pair is
  still defined exactly, so its results reproduce. REQ-INT-006 and -017 become "every pair runs, and the pairs
  measured not to work carry a warning", with a table of pairs marked measured good, measured bad or not measured.
- **Regularisation is v1, not v2.** Remove the "v2" text (integrator contract Part 6) and REQ-INT-033.
- **The integration floor follows the occupant.**
  - Regularisation off: the existing substep machinery is unchanged.
  - Regularised: running out of the step budget makes the result undetermined, never discarded. The separation floor
    and the entry-cap hits are counted, non-terminal signals. Re-point REQ-SCHED-063 and -077.
- **The step budget is a rate**, steps per unit physical time. The calibration value starts at about 4e4 per unit time
  per copy.
- **The sync grid is a fixed quantum of physical time**, the same for every run and independent of the horizon and of
  frame timing. Steps are sized from it, the march stops only on it, and the playhead reads boundary states. Live
  march, catch-up and re-boot then reach the same state bitwise. Time windows such as the closure window are stated in
  physical time.
- **Energy.** The two-body collision gate (REQ-VAL-045) and any per-step suspect flag under a regularised occupant read
  the regularised energy. The Cartesian energy is for diagnostics, read only from runs that did not stop at an event.

## PR 2 — New design documents (one doc per feature, under `docs/design/`, with requirements via reqio)

Each file in `designs/` has six parts: what exists, the gap, the proposal, the rulings it touches, the decisions
(already made, in a "Decided" section), and tests. Write each as a design doc and add its requirements and tests.

- `results.md`: "Result", the full-precision batch job, end to end (f32 or f64 only). Delete "payload summary" from
  TASK-M8-02, which defines nothing and is used nowhere. Philosophy §2's "arbitrary precision" text becomes the
  sentence the draft gives. REQ-SYS-007's parked dump tier needs amending; queue it if that is a larger change.
- `keyframes_reel.md` and `reel_click.md`: keyframed animations, with easing per segment between keyframes (amends
  REQ-TOOL-076). The spotlight reel is a **v2 browser product** feature: build its machinery and test it natively, and
  keep the native presentation rough. The reel plays inside the Explore view, looking like Explore running; any
  interaction takes over live at that exact frame and keeps playing.
- `honesty_precision.md`: in the trajectory viewer, four runs of the clicked point (f32, f64, and each nudged by one
  ulp), with two marks:
  - **"faithful until"** at shape-sphere chord 10⁻³;
  - **"parted at"** at chord 0.1.

  Both are measured (`evidence/parting_probe_output.txt`) and from the literature, both visible and editable. The
  one-line trust text appears in the trajectory inspector panel when a trajectory is clicked. **Nothing changes in the
  atlas.** The horizon stays 50 by default and can be raised **or lowered**, so the Run window's `T_horizon ∈ [50, 200]`
  and R-132 are amended. The README's "Honesty" line is reworded as the draft gives.
- `export_details.md`: a recording's colour range is frozen at Record, with clipping reported. Colour-blindness
  simulation is never baked into an export unless asked, and is recorded when it is. One size rule ("base px") for
  lines, labels and marks; PrintPt is retired (amends REQ-RENDER-088 and print_graph §6.2).
- `ic_save.md`: an IC file, copy and paste, reopening in the constructor, and trajectory CSV with bodies labelled
  1-based (`body1_x` …).
- `uploaded_picture.md`: a SHA-256 reference in the view; texels in a private PNG chunk; stretched to fit; a visible
  "Picture missing" badge, never a substitute.
- `video_carrier_test.md`: a recording carries its settings once per file in the format's own metadata (GIF comment,
  MP4 comment, WebM tag), and **always** (my words: "yes always embed the settings"); PNG frames keep the low-bit
  record; the settings file is always written beside the export. Amend §G9's "pxpack in every frame" and the tests of
  REQ-TOOL-083 and -106. Embedding §9's "off by default for figure export" stays for single figures.

## PR 3 — State map and Listen follow-ups, and the boards

- `fit_to_screen.md`: SM7's colour window becomes Chart / Fit to the screen / Fit to the screen, pinned, implemented
  as one affine map applied before any base. Port it from `demo/state_flow.html` (`smFitWin`, `smFit`). Tests: the
  t = 0 identity under fit (equal to Home, pixel for pixel, with a mirror control), and a pinned window holding under a
  pan.
- `momentum_seam.md`: on a chart showing a mirror-reversed coordinate, a trace breaks at a syzygy like θ's wrap and is
  never joined across (`principia_lines.md` §8). Two tests.
- `listen_fit_doppler.md`: Fit to hearing fits the strongest line's heard range with Doppler included; with Doppler
  off it is bitwise today's rule (`principia_listen.md` §3 and §9). Two tests, the second with its control.
- **Boards 1, 4 and 6** (`boards/`) replace the state-map bundle's boards of the same names. They now match my rulings
  on RQ-500, RQ-501 and RQ-502: one colour node holding the base and its layers, a hover trace with "L listen" and no
  figure ring, and "Edit stain…". Renders are in `boards/png/`.

## PR 4 — Small things

- `INSTRUCTION_now.md`: the stale display-chain order in `principia_systems_architecture.md` :65 and :112 brought to
  R-531; Print's failed pixels recorded as a non-issue in my words; three items added to the backlog (the browser
  build as the whole native app in the browser, cross-build links, accessibility).
- `Listen_effects.md`: send this **only if** it was not already applied (reverb, Doppler and width on by default).

One PR each, with the test output pasted into each. Tell me before you start which, if any, you think should be
split differently.
