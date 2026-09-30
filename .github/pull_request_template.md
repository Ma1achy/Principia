<!-- Title: `<TASK-id>: <title>`. Labels: `design`, `investigation`, `validation` make their sections below mandatory,
and `cargo xtask pr-check` fails naming each one missing or empty; it checks the answers are present, and the physics
reviewer judges them. Text inside these comments is guidance, not an answer. -->

## Task and Closes
<!-- The task id, then one line per requirement closed: `REQ-… → acceptance command → output` (the output in a
fenced block below it). A benchmark command runs on the human's Mac: say it waits for that run (R-186). -->

## Design questions
<!-- Mandatory with `design`: answer each of philosophy §6's four drift questions under its heading. If a design
choice cannot answer them, it is probably wrong here even if it is sensible elsewhere. -->

### Which path is this for?
<!-- Interactive is optimised for attention; prebake for truth. -->

### Where does the constant come from?
<!-- If the answer is "it looked right", stop (philosophy §4.2). -->

### Could this measurement have failed?
<!-- If not, it is not evidence (philosophy §4.4). -->

### Does the instrument still say where it stops knowing?
<!-- A result quoted past its horizon, or a pixel whose failure is hidden, breaks it (philosophy §4.1, §4.3). -->

## Investigation
<!-- Mandatory with `investigation` (pitfalls §4.5): a mechanism belongs in principia_01_pitfalls.md, not only in
the investigation record that found it. Give one of these lines, with its statement:
- pitfalls entry: <the entry added or updated: observed / assumed / actual / measurement>
- none applies: <why no pitfalls entry applies>
-->

## Validation record
<!-- Mandatory with `validation`: one line per error meter and per discriminator (R-180):
- meter: <name> — <the quantity the occupant integrates>
- discriminator: <name> — <its dependency on termination; recomputed on fully integrated runs where it depends>
A meter must measure a quantity the integrator advances: COM "drift" under an integrator of relative coordinates is
the correct value of a non-dynamical quantity, not an error (philosophy §4.5). A discriminator can be poisoned by
its own subject: `d_min` of a run stopped early is larger because it terminated early (pitfalls §3). -->

## Removed lines
<!-- For commits touching `docs/`, `decisions.md` or `plan/`: each removed line, "reworded, kept at <file:line>" or
"stale value, replaced by <ruling>" (decisions.md § "Porting rule"). -->
