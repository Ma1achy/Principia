# TASK-M7-91 — Shaders that are not nodes: one box, run through an adapter, the instrument's guarantees kept

- **Milestone:** M7
- **Depends on:** TASK-M7-81, TASK-M7-90

## Goal
Gaps 28 to 30 of `docs/design/principia_node_sdk.md`: a whole `shade(ctx)` as one box, and a foreign fragment shader (WGSL, or GLSL and SPIR-V through naga) compiled as its own module and drawn as its own pass through a fixed adapter.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render`: a foreign shader whose names clash with the prelude's draws as it does alone, control: the same text spliced in; invalid pixels keep their treatment whatever it outputs, control: the mask removed; a compute shader, a storage write and an unknown binding are each refused with the last good picture kept.
- `cargo test -p gui`: a click, the Inspector and a measure read the same numbers under a foreign shader as under the default stain; a shader that reads no data says so.
