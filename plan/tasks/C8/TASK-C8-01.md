# TASK-C8-01 — Benchmark scenarios as data, one runner, and `prin bench run/show/report/compare`

- **Checkpoint:** C8. **After:** the fixed scripted suite and the calibration campaign (TASK-M8-39).

## Goal
The benchmark as `docs/design/principia_benchmark.md` describes it, for the command line: a scenario is a file in
`bench/scenarios/` read by the runner, "Adding one needs no code" (§2); one scenario runner and one registry behind
`prin profile`, `cargo xtask bench` and the GUI's button (§1); the suite and its hash (§3); the sweep and the Quick,
Full and Soak modes (§4); `prin bench run [--mode quick|full|soak]`, `show`, `report` and `compare` (§5.2); the result
file's benchmark block (§6); and `prin bench report FILE`, a Markdown report fixed in structure, with the anomaly list
and the device-comparison table (§7).

## Touches
`crates/prin` (the `bench` subcommands, beside `profile`), the scenario runner and registry where `prin profile`'s
scenarios live today, `bench/scenarios/`, `xtask`'s bench entry.

## Tests
The tests of §9, each with a control arm that fails; the work's determinism as §8 states it. Timings are
measurements, never gates.
