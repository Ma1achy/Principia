# Open questions

Everything left open, each with its source. Nothing is deferred silently.

- **Dangling cross-references** (step 1, `spec_sources/SECTION_MAP.md`, class D): integrator_contract
  "§collision", "§terminal-detection" and "§regularisation", and deep-zoom "§switchover". None of them is a
  heading anywhere, and the same four names are cited from validation_ground_truth_note:121, deep_zoom:101 and
  scheduler_contract:88. Step 3 repoints them to the sections that actually hold the content
  (integrator_contract Part 7, Part 2b/6, deep_zoom §2).
- ~~`spec_sources/findings.md` is a stale copy of prin-rs `FINDINGS.md`~~ **Corrected in step 2:** it is the
  toolchain spike's findings, not a prin-rs copy. Where it belongs is REVIEW_QUEUE RQ-7.
- **Fold pending change 11 (escape = closure + energy) into the detectors** (step 4). The landed rule is in
  `docs/principia_spec_pending_changes.md` change 11 and `principia_01_pitfalls.md` §2.2. But
  `principia_integrator_contract.md` Part 7 and `principia_dd_integrator.md` §3.6 still specify the older
  three-gate detector (distance, outward, outer energy, persistence to `k_esc`). Step 3 didn't port the LaTeX's
  escape section (it's superseded) and didn't touch the markdown detectors. Source: step 3, port 5.
