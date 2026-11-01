# Peek log: external targets scored early

**Information only — never a gate (R-650 addendum).**

A peek scores an external target before M9, when its data is already transcribed and the run is cheap, only when
idle and after the backlog (R-650's addendum; `docs/design/principia_dd_replication.md` § "12.5 Peeks"). Its outcome
is one of R-649's three: PASS, EXPLAINED DIVERGENCE (with the measurement that shows the mechanism) or UNEXPLAINED.
Nothing is tuned to a peek. A peek leads to a fix only for a genuine bug justified independently of the target (a
wrong equation, a misencoded outcome, a broken invariant), with its own test and ruling. The like-for-like checks are
§ "12.3 Outcomes, the like-for-like check and the forbidden moves" of the same file. Rows are added, never removed;
a later peek of the same target is a new row.

| Date | Target | Tier | Data source | Outcome | Like-for-like checks | Mechanism measurement | Commit |
|---|---|---|---|---|---|---|---|
| 29 Oct 2026 | GG-1 (REQ-VAL-295), point (1, 54) | (b), per point | Brutus run here, not Principia (ε = 10⁻⁴ to 10⁻⁶, L_w = 128), converted to HM units, against Fig. 1 read by route 2 (`docs/incoming/2026-10-28_rq598_data/brutus_run_here/hm_units_output.txt`) | UNEXPLAINED: log₁₀ T 0.236 against 0.521, −0.29 dex | Masses equal. Units: ours converted to HM by √(3λ³), a factor of 2.72 here. Not like-for-like: the escape radius (ours 10 of our units, the paper's 10 HM units) and the T cap (ours 1000 of our units, the paper's 10⁴ HM); the definition of T is open (RQ-612). The two panels agree at this point, so RQ-620 does not bear on it. Source accuracy: Fig. 1 read to one LUT step, 3/256 ≈ 0.012 dex (`docs/incoming/2026-10-28_rq598_data/README.md`) | none | 06f3dd8f (PR #335) |
| 29 Oct 2026 | GG-1 (REQ-VAL-295), point (14, 20) | (b), per point | as above | UNEXPLAINED: log₁₀ T 0.838 against 0.686, +0.15 dex | as above, the factor 5.15 | none | 06f3dd8f (PR #335) |
| 29 Oct 2026 | GG-1 (REQ-VAL-295), point (3, 43) | (b), per point | as above | UNEXPLAINED: log₁₀ T 0.262 against 2.713, −2.45 dex; Fig. 1's neighbours read 0.87 to 1.89, so a grid offset is one candidate (RQ-612) | as above, the factor 3.20 | none | 06f3dd8f (PR #335) |
