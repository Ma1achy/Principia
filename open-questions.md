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
- **Pending-changes register, closed in step 4.** Where each of the twelve changes went:
  1. Resolved (joint grain, dd_generation_root §3.7). The impurity mask's field is REVIEW_QUEUE RQ-18.
  2. **Open:** which quotient the Burrau survey wants, shape-only (fold the leg swap) or shape × labelling. Stated in
     chart_reference §4.5. It is not in audit section B, so it is a candidate for the step-5 decision sheet.
  3. Folded in step 3 (inverse_encode_contract Part 5, "encode reuses decode").
  4. Moot: a LaTeX figure caption. No markdown file makes the 5-DOF claim.
  5. **Open:** audit decision B1 (body index base).
  6. **Open:** R-10 keeps `α_min` (0 or 0.05) on the step-5 decision sheet. The register's "removed, α_min = 0" is not
     folded, because R-10 overrides it.
  7. Folded (detectors, payload §2). **Open:** the ionisation gate's definition, and whether the old-data consequence
     (two-pair collisions recorded as binary) applies to any stored outcome data.
  8. **Open:** RQ-17.
  9. Folded: payload memory table recomputed, closure and period in the debug catalogue; the caching signature already
     carries the payload schema version. **Open:** the stated fraction that defines `t_min` (payload §1, "Define `t_min`
     gauge-covariantly"); and `principia_memory_tiers.md` §4's tier totals, still at the old 136 B width (with audit B10).
  10. Folded (validation_orbits §5 item 1). **Open:** its consequences — re-run the Python cross-check and the
      divergence-vs-horizon table, and fix the NumPy reference's matching defect. Whether these were done isn't recorded.
  11. Folded (integrator_contract Part 7, dd_integrator §3.6). **Open:** whether escape terminates (pitfalls §2.4, three
      checks); the window length for `|Δn̂|`; which energy `E_rel` is; how the escaping body's id is determined.
  12. Folded (R-15, R-18; scheduler contract Parts 3, 4, 6). **Open, recorded in the policy doc:** the two `alpha_area`
      defects (§2.2), and the camera not wired into priority.

  Status reconciliation (audit A3): the register's own Status sections say 7, 8 and 10 are "Open", while the banners
  added above them later say "LANDED", and the index agrees with the banners. Step 4 takes the banners as the register's
  last word for 7, 10, 11 and 12; 8 is RQ-17. Source: step 4.
