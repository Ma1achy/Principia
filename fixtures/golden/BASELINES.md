# Golden baselines

A baseline image changes only with a recorded gate decision (R-110: "no re-baselining without a gate decision"). The
M8 Playwright browser suite checks against these same baselines (R-110), so a baseline is never re-made to make a
render pass.

`cargo xtask golden` hashes each case's reference image and refuses the case unless the table below holds that
SHA-256 for it, with the decision that set it, named as its `R-n` in `decisions.md`. To change a baseline: record the
gate decision in `decisions.md`, replace the image, and update its row here with the new hash and that decision, in
the same commit. A reference that changed without such a row is refused.

The runner checks only that the row holds the reference's current hash and names an `R-n` that `decisions.md` records;
it cannot tell whether that ruling is the gate decision that changed the baseline. That half of the rule rests on
review: a change to a row is reviewed for its decision cell naming the gate decision that set the new hash.

A case keeps one reference across backends, its row named `<suite>/<case>` (R-287). A case whose bytes still differ
between backends keeps one reference per backend (R-269), each with its own row, `<suite>/<case>@<backend>`
(`metal`, `vulkan`).

A new case's row names the ruling that created it. The self-test's references are computed from the formulas in
`selftest/gradient/gradient.wgsl`, not rendered; R-186's placement note puts the fixture in TASK-M0-06.

| case | sha256 | decision |
|---|---|---|
| `selftest/gradient` | `2338cb3d4aa95ce96ee34a3c7c857aa3bb1fa70998becb67749588b9b2570887` | R-186 (created by TASK-M0-06: the analytic gradient) |
| `selftest/gradient_shifted` | `4cdef4300cd3863d6e8ed83036841798fc372aa95531549bec9bc3f072344ecc` | R-186 (created by TASK-M0-06: the analytic gradient shifted by one 8-bit step, 255 to 254; `expect: fail`) |
| `quantise/halfway` | `9d8bc5350a7d7911b046967b2f02e65e3af6f7e0fa1fe139ae1b6c0d5a3514ba` | R-287 (created by TASK-M0-43: R-269's half-way fragment, quantised in the shader; rendered on Metal, M3 Pro and hosted `macos-15` byte for byte; proposed, R-71) |
| `quantise/halfway_automatic@metal` | `814af924469ec4a91b8330be3d91909a214f471041fdc07b0de0e90a7430f921` | R-287 (created by TASK-M0-43: the control, the backend's automatic conversion, rendered on Metal; proposed, R-71) |
| `quantise/halfway_automatic@vulkan` | `11c7202434422713c2b66dcede3f3c5bc5e92f70d1b1f2f988a381643bc587a7` | R-287 (created by TASK-M0-43: the control, the backend's automatic conversion, rendered on lavapipe, run 36736481929; proposed, R-71) |
