# TASK-C2-01 — The chart comparison pass: every preset against the original app and against prin-rs

- **Checkpoint:** C2, its last task: it starts after every other chart task has merged. No per-chart comparisons
  during C2.
## Goal
For every chart preset, three comparisons, each reported in the PR with its image pair and any difference:
- **The chart itself** (decode, constants, preset window and basis) against the original app,
  https://github.com/Ma1achy/principia-ii (`src/shaders/principia/frag.glsl` and its presets).
- **The field** against prin-rs, https://github.com/Ma1achy/prin-rs (branch `main`), the physics reference: the same
  preset and window, both coloured by outcome class. Its presets and windows are in its `src`
  (`Chart::default_half`, the GLSL presets), its render harnesses in `examples/`, its committed renders under
  `results/`. Use a committed render only if its settings match; otherwise re-render with prin-rs at its production
  config (`EnsembleCfg::production`).
- **A symmetry test** where the preset has one (equal masses at rest: a mirror flip with two bodies swapped), with a
  control that breaks the symmetry and fails. The z_α × z_β origin slice already passes (100% in its solid regions).

A difference is a finding to explain, not a failure to tune away. Until escape detection (TASK-M3-12) lands,
compare collision regions only (prin-rs labels escapes and we don't yet); re-run the field comparison once it does.
## Touches and tests
The validation harness and the presets' tests; read-only on the kernel. Symmetry tests carry control arms; the chart
and field comparisons are measurements, their criterion written in the PR before the run, reported, never tuned.
