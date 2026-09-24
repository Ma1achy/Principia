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
- **`DEBUG_MODE` has no bits in `QuadRequest.flags`** (step 3, port 8). The render contract ("Kernel debug dispatch
  modes") and `principia_debug_tooling_plan.md` put a four-value `DEBUG_MODE` enum in the dispatch flags beside
  `DECODE_MODE`. The flags table now in scheduler contract Part 5 assigns bits 0–5 and leaves 6–7 reserved. Two bits
  are enough for `DEBUG_MODE`, but nothing assigns them. The lowering contract bakes the debug modes as variants,
  so they may not need a flag at all.
- **`FULL_RETENTION` has no markdown owner** (step 3, port 8). The flag (bit 4, "skip reduction, keep every
  per-sample result") is in the scheduler contract's flags table. No markdown file describes the full-retention path
  it selects.
- **Fold R-15 and R-18 into the scheduler contract** (step 4, with pending change 12). Part 6's split/keep/merge rule
  defers to `principia_dd_refinement_policy.md` (R-15). Its split list still names `ensemble_outcome_agreement`, which
  R-18 retires in favour of `spread_event`. Source: rulings R-15 and R-18, step 3.
