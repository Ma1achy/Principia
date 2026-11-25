# TASK-M7-86 — Field ramps as one shipped node: field, gradient, curve, polarity and range as parameters

- **Milestone:** M7
- **Depends on:** TASK-M7-79, TASK-M7-85

## Goal
One shipped `field_ramp` source replaces the shaders `colour/field_ramp.rs` builds (37 fields, each a colour, a brightness and a style post), and their generator; each becomes that node with its parameters set, and is offered in the library, where none is today. The CPU mirror stays as the check.

## References
- `docs/design/principia_node_sdk.md`

## Acceptance tests
- Every ramp's picture equals the old one pixel by pixel on the zero view and on a fixture holding every invalid, unset and infinite value; differences are listed and explained or they block. Control: the sentinel test removed.
