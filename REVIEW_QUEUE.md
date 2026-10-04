# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-206: the drift views' scale before `dbg_sentinel`: diverging (debug plan §D) or log with floor (render contract Part 6) *(TASK-M3-05, REQ-TOOL-149)*

- **File, section:**
  - `docs/design/principia_debug_tooling_plan.md` § "D. Payload field views — `SimState` scalars (ledger §3.4)":
    "**The two drift fields are signed → diverging scale, not sequential-log** (generation-root finding)", and its
    rows "`energy_drift` | derived: `H(r,p) − E_0` | **diverging**" and "`Lz_drift` | derived: `L_z(r,p) − Lz_0` |
    **diverging**".
  - `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata" (the note at
    line 146): "**the two drift fields are signed** → *diverging* colour scale, not sequential-log".
  - `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)": "So
    `energy_drift` → log with floor, `t_end` → lin [0, T], …".
  - The layout table (TASK-M0-09, R-263) carries `Scale::Diverging` with `floor: eps_E` for `energy_drift` and
    `floor: eps_L` for `Lz_drift`, so the metadata has both a diverging scale and a floor.
  - `plan/tasks/M3/TASK-M3-05.md` § "Deliverables" and REQ-TOOL-149: the energy-drift and L_z-drift views apply the
    suspect styling on `dbg_sentinel`'s output, and a non-suspect pixel equals `dbg_sentinel`'s output (R-379).
- **What:** `dbg_sentinel` places a value on the viridis ramp at `0.5 + 0.5·x/(1 + |x|)`, with no range (render contract
  Part 5 "Presentation layer"). Fed the raw drift, which is ≪ 1 on any sane run, a drift of 10⁻⁶ to 10⁻³ lands within
  0.0005 of the ramp's middle, and the view is one flat colour. So the drift views must apply the field's scale first and
  pass the scaled value to `dbg_sentinel` (TASK-M3-05's styling definition states it). The corpus gives that scale two
  ways: §D and generation-root say diverging and expressly not sequential-log; render contract Part 6 says log with
  floor. They disagree on whether the sign is shown, and on whether the magnitude is compressed by a log.
- **Options seen:**
  1. **Signed log with floor (diverging):** `x ↦ sign(x)·ln(1 + |x|/ε)`, ε the field's floor (`eps_E`, `eps_L`), fed to
     `dbg_sentinel`. It keeps §D's sign and Part 6's log compression and floor, and reads the layout table's
     `Diverging` plus `floor` as one scale. Part 6's "log with floor" is reworded to say so.
  2. **Diverging, linear:** `x` over a symmetric range `[−R, R]`, which needs a range value (a calibration, R-71).
     Part 6's "log with floor" is corrected to "diverging".
  3. **Log with floor, unsigned:** `ln(1 + |x|/ε)`, the sign dropped, as Part 6 says; §D and generation-root are
     corrected.
- **Needed:** a ruling on which scale the drift views apply before `dbg_sentinel`. TASK-M3-05's suspect styling
  definition (REQ-TOOL-149) waits for it; nothing in TASK-M1-03 does.
