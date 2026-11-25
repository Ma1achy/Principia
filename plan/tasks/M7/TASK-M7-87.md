# TASK-M7-87 — The outcome palette read from its own source by the legend, the CPU image and the editor

- **Milestone:** M7
- **Depends on:** TASK-M7-77

## Goal
The swatch colours are the shipped source's declared defaults; `SWATCHES` and the CPU twin are generated from it, and no code names its id.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render -p engine -p gui`: the CPU image and the legend are bit-identical to before; changing a declared default in a fixture changes shader, CPU image and legend together, control: a swatch written by hand.
