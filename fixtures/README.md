# Fixtures

Test inputs transcribed from the prin-rs reference set (`docs/reference/prin-rs/`, pinned to `8600d45`; R-159, R-166).
Each file names the source lines it was transcribed from. The values are copied, not chosen. Where a fixture and a
contract disagree, the contract wins and the disagreement goes to `REVIEW_QUEUE.md`.

| File | What it holds | Ruling |
|---|---|---|
| `slices.toml` | the named slices: `config_stability`, `config_basin`, the five Burrau regions, `tilt_plambda`, `preset_shape` | R-166 (a) |
| `case_matrix.toml` | the 32-case integrator matrix and its `err>10` metric | R-165, R-166 (b) |
| `moving_pulse.toml` | the moving pulse: a synthetic field, not a chart slice | R-166 (e) |
| `gates/<gate>/` | a numerical gate's `gate.json` (inputs, threshold by requirement id) and inputs; `convergence` holds the pitfalls §3 escape-fraction record and a converging sequence, not transcribed from prin-rs | R-171, R-258 |
| `screenshot/<suite>/` | a GUI screenshot suite: `cases.json` (each case a layout case naming an artboard, or a presence-only case listing controls) and the surfaces it names; `selftest` is the runner's own, with a checked-in reference image the runner rendered, not transcribed from prin-rs | R-68, R-129, R-183 |

Two fixtures are produced at M3 and checked in here when they are, not transcribed now:

- **The legacy `t = 30` set** (R-166 (c)): M3's validation harness regenerates it on the config slices at `t = 30`
  under the legacy classifier (`docs/reference/prin-rs/src/outcome.rs`).
- **BodyPlane** (R-166 (d)): M3 records the Python reference's output (`docs/reference/prin-rs/tools/xcheck`,
  `reference/*.py`).
