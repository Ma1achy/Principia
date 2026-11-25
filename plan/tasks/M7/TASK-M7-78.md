# TASK-M7-78 — Richer node parameters: gradients, lists, lookup tables, field inputs and how controls relate

- **Milestone:** M7
- **Depends on:** TASK-M7-77

## Goal
Gaps 1, 4 and 6 of `docs/design/principia_node_sdk.md`: the parameter types `gradient`, `list`, `lut`, `field` and `image`; `pair`, `soft`, `unit` and `format` on a control; labels, accepted subtypes and optionality on an input. The custom-stop ramp becomes a gradient parameter.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- `cargo test -p render -p engine`: each new type round-trips through the canonical form and reaches the shader; each malformed declaration is refused at its line; today's ramps draw bit-identically as gradients.
