# TASK-C9-01 — The closure field as a stability reading: well width and `alpha`, and Szebehely–Peters located on it

- **Checkpoint:** C9. A measurement, never a gate: it reports pass, explained divergence or unexplained.

## Goal
The two readings `docs/design/principia_dd_validation_orbits.md` proposes and no task measures yet. §6.2: "`alpha`
on the closure field. A smooth basin gives `alpha ≈ 1` … a filament gives `alpha ≈ 0`", read beside the FTLE on the
same pixels, with the closure well's width as the stability reading, compared with the atlas orbits' published
stability. §6.3: whether Burrau sits on the unstable manifold of a nearby periodic orbit, with the Szebehely–Peters
orbit "a named target with a published location" located on the closure field.

## Touches
The validation harness and the closure field's existing code (TASK-M4-15's), read-only on the kernel.

## Tests
The pass criterion written in the PR before the run; the measurement code's own tests with control arms. The result
is reported, never asserted as a gate.
