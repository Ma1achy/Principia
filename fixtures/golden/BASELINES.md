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
between backends, a golden near a tie among them (R-296), keeps one reference per backend (R-269), each with its own
row, `<suite>/<case>@<backend>` (`metal`, `vulkan`).

A new case's row names the ruling that created it. The self-test's references are computed from the formulas in
`selftest/gradient/gradient.wgsl`, not rendered; R-186's placement note puts the fixture in TASK-M0-06.

| case | sha256 | decision |
|---|---|---|
| `selftest/gradient` | `2338cb3d4aa95ce96ee34a3c7c857aa3bb1fa70998becb67749588b9b2570887` | R-186 (created by TASK-M0-06: the analytic gradient) |
| `selftest/gradient_shifted` | `4cdef4300cd3863d6e8ed83036841798fc372aa95531549bec9bc3f072344ecc` | R-186 (created by TASK-M0-06: the analytic gradient shifted by one 8-bit step, 255 to 254; `expect: fail`) |
| `quantise/halfway@metal` | `9d8bc5350a7d7911b046967b2f02e65e3af6f7e0fa1fe139ae1b6c0d5a3514ba` | R-296 (created by TASK-M0-43: R-269's half-way fragment, quantised in the shader, rendered on Metal, M3 Pro and hosted `macos-15` byte for byte; one reference per backend, a golden near a tie; confirmed, R-376) |
| `quantise/halfway@vulkan` | `11c7202434422713c2b66dcede3f3c5bc5e92f70d1b1f2f988a381643bc587a7` | R-296 (created by TASK-M0-43: R-269's half-way fragment, quantised in the shader, on lavapipe; byte-identical to lavapipe's automatic conversion, R-296's Result (run 36736481929), so the same image as `quantise/halfway_automatic@vulkan`; one reference per backend, a golden near a tie; confirmed, R-376) |
| `quantise/halfway_automatic@metal` | `814af924469ec4a91b8330be3d91909a214f471041fdc07b0de0e90a7430f921` | R-287 (created by TASK-M0-43: the control, the backend's automatic conversion, rendered on Metal; confirmed, R-376) |
| `quantise/halfway_automatic@vulkan` | `11c7202434422713c2b66dcede3f3c5bc5e92f70d1b1f2f988a381643bc587a7` | R-287 (created by TASK-M0-43: the control, the backend's automatic conversion, rendered on lavapipe, run 36736481929; confirmed, R-376) |
| `m1-coords/coords` | `30b3ef16650c646758c0efdb837832db5d0338fc43354a59bd538837f01af4b4` | R-369 (created by TASK-M1-07 per RQ-210, decided under R-369: the UV-passthrough coordinate view, its fragment taking the flip from `lib/coords.wgsl`, which the runner prepends; computed from the formula in `m1-coords/coords/coords.wgsl`, R = x/W and G = 1 − y/H at each pixel's centre, quantised half to even, not rendered) |
| `m1-numeric/ftle` | `a60ad77082d6e88452ef1e65e6037dc4981e83a7fbf114e45f8617e360643e7c` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `ftle`'s view: an unstepped and a failed sample read NaN and are hatched (R-253, R-254), the rest on the ramp; one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/diffusion` | `4b0456cdf6f4c083f7dde1a6d42920f15ca468c366160f33557e4fcd1115ede0` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `diffusion`'s view: n = 0 and n = 1 read NaN and are hatched (R-245), n = 2 and the other valid slopes on the ramp; one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/de_max_failed` | `f922609a8c72a4b8898ab4174df228940f88ede320782c21fbdd6e890404788a` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `dE_max`'s view: a forced-failure sample's stored 0.0 on the ramp at 0.0, not hatched (R-79); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/dlz_max_failed` | `0155bed0742d38a0e2adb56a82039420eeca14c67fd6f183fb7c03dbfecf32f3` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `dLz_max`'s view: a forced-failure sample's stored 0.0 on the ramp at 0.0, not hatched (R-79); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/d_min` | `250f60524440dfee4bdb86465001059d91365463d7efaf56f6623054688a8d5f` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `d_min`'s view: a forced failure and an unstepped sample unset, f16 +inf by its bits, in the "not yet" grey (R-271, R-280); an f16 NaN hatched; a failed-state 0.0 and four valid values on the ramp; one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/length_view` | `bb61737b0f5baaaa7563b87d1aa6ea96a0ef6da4d306a1d8777e049e71a6b53d` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: the word `length`'s view: the sentinel 127 as its literal value on the ramp (R-136); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/length_ramp` | `372830053325d831749332d96343fc9b6b2dea5d6f7adbcb7aff21fefbeddd62` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: the word `length`'s field ramp: 127 in the invalid pattern (REQ-COL-001); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/length_ramp_override` | `3348e0b4fc44b64ba4a03470d3d1dd76503fe974175cfac023a3edc965ac0eaf` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: the word `length`'s field ramp, invalid colour overridden: 127 in the override colour, nothing else changed (REQ-COL-001); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/d_min_ramp` | `02b99fdc0a4ae4d211d423fe26634e53393d67c300e95737ed37d3af10335860` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `d_min`'s field ramp: NaN in the invalid pattern, the unset value in the grey (R-280); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
| `m1-numeric/d_min_ramp_override` | `235b5f17b99c9f5bf6e1a38e078515307651afdede463ec2d8d225bfe29b900b` | R-369 (created by TASK-M1-09 per RQ-229, a harness case rendered by `golden_harness` on Metal: `d_min`'s field ramp, invalid colour overridden: NaN in the override colour, the unset value still grey (R-280); one reference across backends, every channel at least 0.01 of an 8-bit step from a rounding tie; proposed, confirmed at the M1 gate) |
