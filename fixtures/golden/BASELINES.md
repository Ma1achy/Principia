# Golden baselines

A baseline image changes only with a recorded gate decision (R-110: "no re-baselining without a gate decision"). The
M8 Playwright browser suite checks against these same baselines (R-110), so a baseline is never re-made to make a
render pass.

`cargo xtask golden` hashes each case's reference image and refuses the case unless the table below holds that
SHA-256 for it, with the decision that set it, named as its `R-n` in `decisions.md`. To change a baseline: record the
gate decision in `decisions.md`, replace the image, and update its row here with the new hash and that decision, in
the same commit. A reference that changed without such a row is refused.

A new case's row names the ruling that created it. The self-test's references are computed from the formulas in
`selftest/gradient/gradient.wgsl`, not rendered; R-186's placement note puts the fixture in TASK-M0-06.

| case | sha256 | decision |
|---|---|---|
| `selftest/gradient` | `2338cb3d4aa95ce96ee34a3c7c857aa3bb1fa70998becb67749588b9b2570887` | R-186 (created by TASK-M0-06: the analytic gradient) |
| `selftest/gradient_shifted` | `4cdef4300cd3863d6e8ed83036841798fc372aa95531549bec9bc3f072344ecc` | R-186 (created by TASK-M0-06: the analytic gradient shifted by one 8-bit step, 255 to 254; `expect: fail`) |
