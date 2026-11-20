# TASK-C2-01 — The chart comparison pass: every preset beside prin-rs and the original app, for the owner's eye
- **Checkpoint:** C2, its last task: it starts after every other chart task has merged. No per-chart comparisons
  during C2. For the owner to judge by eye: no new tests or gates, nothing found is fixed without the owner's say.
## Goal
For every chart preset, one row of images in the PR or an artifact page: ours, prin-rs's, and principia-ii's where
it has the preset. All over the same window, coloured by outcome class.
- **The chart itself** (decode, constants, preset window and basis) read against the original app,
  https://github.com/Ma1achy/principia-ii (`src/shaders/principia/frag.glsl` and its presets).
- **The field** against prin-rs, https://github.com/Ma1achy/prin-rs (branch `main`), the physics reference. Its
  presets and windows are in its `src` (`Chart::default_half`, the GLSL presets), its render harnesses in
  `examples/`, its committed renders under `results/`. Use a committed render only if its settings match; otherwise
  re-render with prin-rs at its production config (`EnsembleCfg::production`).

What the owner looks for: the same structure, and no obvious warp (stretched, sheared, rotated or mirrored
structure), which would mean wrong basis vectors or a wrong window. Before showing the owner, look yourself and list
every pair whose structure clearly differs, naming the chart and what looks off. A difference is a finding to
explain, not a failure to tune away. Until escape detection (TASK-M3-12) lands, compare collision regions only
(prin-rs labels escapes and we don't yet); re-run the field comparison once it does.
## Touches
Only the render and capture harness code the images need; read-only on the kernel and the charts.
