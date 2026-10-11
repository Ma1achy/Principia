# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-268: three more f16 `QuadReduction` members have no declared range, and no calibration requirement names them *(silence, missing value; context: TASK-M5-01)*

*Found by TASK-M5-01's implementer. The task's pre-flight found five f16 members with no range and made them
REQ-PAY-093 (R-71); three more have none either. TASK-M5-01 proposes their ranges as it does the five's, marked
proposed, so the ledger generates; nothing waits on this but their confirmation.*

- **File, section:**
  - `docs/design/principia_dd_generation_root.md` § "3.8 Metadata schema (what every entry must carry)": "**A field
    without a complete entry fails generation loudly.**" and "`f16` (R-312) … follows R-248's rules as `f16-pair`
    does: exactly 16 bits, a declared range within ±65504, and an unbounded end allowed only when the entry states
    `overflow`."
  - `docs/design/principia_dd_generation_root.md` § "Validity and diagnostics": "| `escape_time_min` / `_max` | f16 × 2
    | diagnostic range |" and "| `mean_orbit_count_spread` | f16 | tiebreaker |", with no range, unit or bound.
  - `plan/requirements.yaml` REQ-PAY-093 names only "`error_ratio`, `roundtrip_error`, `alpha_area`, `alpha_energy`
    and `worst_energy_drift`".
- **Missing value:** the declared `range` of `escape_time_min`, `escape_time_max` and `mean_orbit_count_spread`, and
  their `overflow` where an end is unbounded (R-248). An escape time is a time within the horizon `[0, T]` (§3.4's
  `t_end`), but `T` is a sim-key parameter with no corpus bound; an orbit-count spread is ≥ 0 with no upper bound.
- **Missing value, the no-escape case (added at physics's review of TASK-M5-01's PR, F2):** `escape_time_min` and
  `escape_time_max` have no value when no copy of the quad escaped. §3.7 gives them neither a value nor a sentinel for
  that case, and 0 or any finite time would read as an escape that happened.
- **Proposed (TASK-M5-01's PR, as R-71 proposes a value):** each `[0, ∞)` with `overflow: saturate`. The lower ends
  follow from the definitions (a time, a spread). Saturate rather than inf: a diagnostic range or a tiebreaker past
  65504 reads "at least 65504", a finite value, where +∞ would read as a time never reached.
- **The no-escape case, options (corrected at physics's review of TASK-M5-01's PR, F3):** the minimum of the empty
  set of escape times is +∞, but its maximum is −∞, so no one value is both reductions' identity. Either option is
  tested by its bits, as `d_min`'s unset value is (R-271) and `first_divergence_t`'s is (§3.7), and saturate keeps every
  escape time finite, so the sentinel is distinct from every time recorded. Either sentinel lies outside a finite
  escape time; the human chooses it with the ranges, and until then §3.7 holds it open and the ledger entries carry
  none.
  - (a) **+∞ for both** (binary16 `0x7c00`, f32 bits `0x7f80_0000` once widened). For `_min` it is the min
    reduction's identity. For `_max` it is a chosen sentinel, not the empty max, so the writer (TASK-M5-17 to M5-19)
    needs a no-escape branch that stores +∞ when no copy escaped. One test, `== +∞`, reads "no escape" on either
    member.
  - (b) **Each its reduction's identity**: +∞ for `_min` and **−∞** for `_max` (binary16 `0xfc00`, f32 bits
    `0xff80_0000`). Both reductions start from their identity and need no branch, and an empty range reads
    `_min > _max`. `_max`'s −∞ lies below its proposed range `[0, ∞)`, as a sentinel may.
- **Options seen for the ranges' confirmation:**
  - (a) REQ-PAY-093 names the three as well (reqio), and the human confirms the proposal with the five at the M5 gate.
  - (b) A calibration requirement of their own, confirmed at the M5 gate.
  - (c) A ruling gives their ranges now.
