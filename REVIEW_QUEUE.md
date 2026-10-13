# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-274: R-423's M1-gate check cannot confirm `q_mass` and `mean_y` on R-400's cool-warm ramp: `q_mass`'s formula and `mean_y`'s per-sample `y` are undefined, each ledger range is the unbounded default, and the corpus's own words read each as non-negative *(silence, R-400, R-423, R-72, REQ-RENDER-084, REQ-RENDER-085, REQ-PAY-088, REQ-PAY-073; context: the M1 gate, TASK-M1-17, TASK-M1-20, TASK-M2-03, TASK-M3-13)*

*Found by the physics review of PR #197 (review 5484757609, § "The M1-gate five-field check (R-423)"), which ran
R-423's gate check of the five fields early. Filed per `plan/MILESTONES.md` § "M1"'s gate item: "A field that fails
goes to `REVIEW_QUEUE.md` for the human". Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-423 — At the M1 gate physics checks the six fields R-400 moved to cool-warm; `theta`, an angle,
    takes the cyclic ramp, not the diverging one *(amends R-400)*", the human's words: "For the M1 gate: physics to
    check the six fields moving to cool-warm." `plan/MILESTONES.md` § "M1 — The synthetic payload and the eyes", the
    gate item: "a read-only physics check of `q_mass`, `ftle`, `mean_y`, `S` and `diffusion`: that each is a signed
    quantity whose zero is a meaningful centre for a diverging ramp centred at zero. A field that fails goes to
    `REVIEW_QUEUE.md` for the human".
  - `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp
    centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*": "Where §3 gives a field no range
    the ledger declares (−∞, ∞) (`crates/ledger/src/payload.rs`:7), which spans zero; a field the corpus gives a
    non-negative domain must not read as signed by that default. So TASK-M1-17 declares the lower bound of each field
    whose domain the corpus or the field's own definition makes non-negative". REQ-RENDER-084 (TASK-M1-17, merged in
    PR #195) is the requirement that draws a field whose range spans zero on `ramp_coolwarm`.
  - `q_mass`: `crates/ledger/src/payload.rs`:261, `ic("q_mass", 3, Scale::Lin),` with no `.range(…)`, so the
    unbounded default, beside `m0`–`m2` (`.range(fraction)`) and `rho_mag`, `lambda_mag` (`.range(from_zero)`).
    `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)" (:171) lists "`q_mass`" with no
    scale or range; `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time,
    pre-integration)" gives no formula. `plan/requirements.yaml` REQ-PAY-088 (`kind: definition`, `milestone: M2`,
    TASK-M2-03): "The formulas of the ICDescriptor fields `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio`, `rho_angle`
    and `r_min_pair_0` … must be defined in dd_decoder §3.6 by the task that needs it".
    `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`" (:95) lists among
    the decoded-IC quantities "mass ratios".
  - `mean_y`: `crates/ledger/src/payload.rs`:217, `f32_at("mean_y", 26, Scale::Lin),`, the unbounded default.
    `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)": "Slope of spread `y`
    on time `t` (the diffusion coefficient)"; `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars —
    with presentation metadata" (:113), the mini-spec: "Regression of spread `y` on time `t`, slope = diffusion rate",
    and (:123) "`mean_y += dy / n`", so `mean_y` is the running mean of `y`.
    `plan/requirements.yaml` REQ-PAY-073 (`kind: definition`, `milestone: M3`, TASK-M3-13): "The per-sample value `y`
    that the Welford diffusion regression fits against time must be defined in `principia_dd_simstate_payload.md` §4 by
    the task that needs it".
- **Physics' result** (review 5484757609): `S`, `ftle` and `diffusion` pass; each is signed at finite time, with zero
  a meaningful centre, and none is an angle. `q_mass` and `mean_y` are **not confirmed**:
  - `q_mass`: "Its formula is unwritten (REQ-PAY-088, TASK-M2-03, M2; dd_decoder §3.6 lists only the name). Its ledger
    range … is the unbounded default, not derived. A mass ratio would be ≥ 0, which is viridis under R-400; a
    log-ratio or asymmetry would be signed. The corpus doesn't settle it."
  - `mean_y`: "It is the running mean of the per-sample y, which is undefined (REQ-PAY-073, M3, R-72). The corpus calls
    y a 'spread' (payload §4; generation_root §3.4 mini-spec), which would make mean_y ≥ 0 and viridis. Its ledger
    range (payload.rs:217) is again the unbounded default."
- **Silence:** the corpus does not say whether `q_mass` or `mean_y` can be negative. Each is on `ramp_coolwarm` only
  because no range was declared, which is R-400's default reading, not a finding that the field is signed; and the
  corpus's own words ("mass ratios"; "spread `y`") lean the other way. Their definitions are later milestones' (M2 and
  M3), so the M1 gate cannot settle the check from the corpus.
- **Options seen:**
  - (a) Keep both on `ramp_coolwarm` provisionally, until REQ-PAY-088 defines `q_mass` and REQ-PAY-073 defines `y`;
    nothing changes now, and the M1 gate records them as provisional.
  - (b) Switch both to viridis now, reading `q_mass` as a non-negative ratio and `mean_y` as the mean of a
    non-negative spread: each gets a declared lower bound of 0 in the ledger (R-400's mechanism), its view moves to
    viridis and its golden is re-rendered; REQ-PAY-088's and REQ-PAY-073's definitions are then bound to be
    non-negative, or the bound changes through an RQ.
  - (c) Re-check at their definitions: `q_mass` when TASK-M2-03 writes REQ-PAY-088 (M2), `mean_y` when TASK-M3-13
    writes REQ-PAY-073 (M3), each task declaring the field's range from its definition and the physics reviewer
    checking the ramp then.
  - (a) and (c) may be taken together, and the options may be taken per field.
- **Applied meanwhile:** the gate check's result is recorded in `plan/MILESTONES.md` § "M1"'s gate item and in R-423;
  `S`, `ftle` and `diffusion` stay on `ramp_coolwarm`, confirmed. `q_mass` and `mean_y` stay on `ramp_coolwarm` as
  TASK-M1-17 left them, which settles nothing here.
- **Waits:** the M1 gate's R-423 cool-warm item, for `q_mass` and `mean_y` only: the gate report cannot give either
  field as confirmed until this entry is ruled, at or before the M1 gate. No task waits, so no requirement is marked
  `rq: RQ-274`:
  - TASK-M1-20 (REQ-RENDER-085) does not wait: `theta`'s move to the cyclic ramp is the same under every option. Its
    "`q_mass` … and `mean_y` keep `ramp_coolwarm`" assertion pins the state as TASK-M1-17 left it; under (b) it
    changes as a ruling-forced change (R-290, R-369), made by whichever task the ruling's port names.
  - TASK-M2-03 (REQ-PAY-088) and TASK-M3-13 (REQ-PAY-073) do not wait: each writes its definition under every option,
    and (c) needs them to run. They are named, not marked, as RQ-271 named REQ-PAY-088.
  - REQ-RENDER-084 is closed by TASK-M1-17, merged; a mark would hold nothing.
