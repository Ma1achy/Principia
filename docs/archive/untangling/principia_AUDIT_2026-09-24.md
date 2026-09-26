# Principia — corpus audit before the build

*24 Sep 2026. Scope: the 48 current (non-archive) `principia_*.md` files in outputs, against
`principia_INDEX.md` (22 Sep). Goal: nothing missed, nothing skipped, nothing silently deferred.*

**Verdict.** The contracts and drill-downs are strong, but the corpus is **not yet buildable by an
agent from scratch.** Five blocking issues, then the decision list.

---

## A. Blocking — corpus integrity

### A1. The LaTeX dependency is live, and its authority is contradicted

- `principia_canonical_spec.md` §11: *the `.md` files are the source of truth; the LaTeX compiles from them.*
- `principia_chart_reference.md` line 4: *where this doc and the LaTeX disagree, **the LaTeX wins**.*
- Ruling (24 Sep): **the LaTeX is stale — do not reference it.** It is not in outputs at all.

**85 references in 19 current files** point into the LaTeX, **21 by named section**. Where a drill-down
says "specified in the spec, not re-derived here", that content exists **nowhere in the corpus**:

| section | relied on by |
|---|---|
| `§mass_decode` `§hyperspherical_jacobi` `§canon_config` `§jacobi_mom` `§scale_gauge` `§energy_norm` | dd_decoder — the decode's closed forms |
| `§com_projection` `§collision` `§terminal` `§regularisation` | integrator_contract, validation_ground_truth_note, deep_zoom |
| event-detection section (priority order) · shape-sphere section (axis order) · `§generation` | dd_integrator |
| `§lookup` | inverse_encode_contract |
| `§measure` `§switchover` | scheduler_contract |
| `§ensemble` | sampling_msaa_note |
| `§tile_reduction` `§consolidated` | dd_generation_root |
| `§parity` `§correctness` | systems_architecture, canonical_spec, parity_contract *(check: may be internal)* |
| Burrau section (1-based indices) | chart_reference |

**Fix:**
1. One ruling in `canonical_spec`: markdown is authoritative, the LaTeX is retired.
2. Delete the "LaTeX wins" clause.
3. **Port each section's content** (formulae, orders, constants) into the drill-down that relies on
   it, from the last good source: the `.tex`, or the prin-rs code where it's already implemented and
   tested.

### A2. Sixteen files are not in the index

An agent can't tell current from superseded. Each needs a verdict — *current* (add to index),
*absorbed* (name where), or *archive*:

| file | likely verdict |
|---|---|
| `render_gui_spec.md` | **current, but stale** — predates the Sep GUI design (see A4) |
| `trajectory_viewing.md` | current (Observation ring) |
| `debug_tooling_plan.md` | current; overlaps the new Overlays menu and profiler |
| `sampling_msaa_note.md` | current (Halton committed) |
| `gpu_determinism_note.md`, `validation_ground_truth_note.md` | current |
| `core_design.md`, `dd_generation_root.md` | check; generation_root is heavily referenced |
| `ftle_shadow_precision_experiment.md` | evidence — move to results |
| `parity_testing_note.md`, `validation_scratchpad.md` | **self-declare SUPERSEDED** → archive |
| `spike_brief.md` | delivered brief → archive |
| `dd_refinement_external_report{,2,3,4}.md` | evidence for the refinement policy → results/archive |

### A3. The pending-changes register disagrees with the index

- **The index says** changes 7, 8, 10, 11 and 12 have landed.
- **The register marks** only 1 (resolved), 9, 11 and 12 (landed). **2–8 and 10 carry no status line.**
- **2, 3, 4 and 6 are LaTeX wording edits.** With the LaTeX retired, the edits are moot, but their
  *substance* must be in markdown:
  - the Burrau leg-swap range;
  - the constrained-inverse wording;
  - removing α_min from the shape-sphere link.
- **5 (body-index naming) is still undecided** — see B1.

### A4. The GUI design is formalised on the canvas, not in the corpus

`canonical_spec` §11 lists "GUI design — formalisation session pending". That session happened
(Principia dev GUI canvas, Sep 2026). `render_gui_spec.md` (Jul) and `gui_state_contract.md` (Jul)
predate it. **Needed:** `gui-dev/*.md` written from the canvas, plus every artboard exported as a PNG
into the repo.

### A5. There is no build plan

`canonical_spec` §11: "M0–M8 phasing and the task DAG are unwritten." This is the `plan/` tree:
one file per task, citing file and section, requirement IDs, acceptance tests and reviewers.

---

## B. Decisions to make now (each is a one-liner in `decisions.md`)

**From the corpus:**

1. **Body indexing** — decode is 0-indexed; `ICDescriptor` says `m1 m2 m3`. The drill-down recommends
   0-indexed throughout. Choosing it forces a schema-version change, which is correct.
2. **Encode projection metric** — chordal ⊕ Euclidean with unit weights: confirm or veto (dd_encode §3.4).
3. **Encode steps 0/1 order** — CoM before I, pinned in shared source (dd_encode §3.2).
4. **Event priority order** — pinned in dd_integrator §3.6; confirm (its LaTeX cross-check is gone, A1).
5. **Shape-map axis order** — component → axis assignment (dd_integrator §3.7).
6. **Schema version = content hash of the ledger** — recommended to adopt (dd_generation_root §6).
7. **Symbolic-dynamics `dominant_pair` attribution** — OPEN. Per-pair quantities stay untrusted until
   it's decided. Word storage is settled.
8. **The independent convergence reference** — *what* it is is settled; *which* integrator is not.
9. **Energy normalisation η_E** — exists only in the LaTeX (A1). Keep (flag-gated), or drop.
10. **Memory tiers and quality device** — marked "pending redefinition to key off `eps`". Do it
    before they're implemented.

**From the index's own open items:**

11. `alpha_area` — the empty-mask vs full-mask ambiguity, and d > 2 from negative exponents.
12. A cheap `sea_fraction` estimator.
13. **`Decision::Undetermined` has no budget line.**
14. **`N = 16` vs 8 on WebGPU** — measure, don't assume.
15. **Frontier scoping** — the margin, derived from the refill rate.
16. **Relevance arithmetic in global UV** — fix before any deep-zoom demo.
17. **FMA contraction control per backend** — unchecked; it can move a branch input.

**From the GUI sessions:**

18. **Shape-sphere collision landmarks** — mass-weighted, or fixed at 120°.
19. **Node interface declares input domains** — the generated legend depends on it.
20. **Chart domain functions** — each chart type supplies its admissible region; physical axes make
    the chart nonlinear (Φ).
21. **Where undo lives** — in the contract (proposed), or per GUI.
22. **The deep-zoom precision warning's thresholds** — tie to `AT_F32_FLOOR` and the linearised-decoder
    switchover.
23. **Profiler JSON schema v1** — scopes, GPU passes, counters, allocations, events.
24. **Cursor bias** — named, never specified. Specify it, or drop it.
25. **Momentum-decode mass weighting (§2.6)** and **chart constants** μ_max = 4, α_min = 0.05, q_max = 2 —
    confirm against the real chart config (IC Inspector open items).

---

## C. Transcription checks — before they enter the shared source

- **Yoshida-6 coefficients** against Yoshida (1990), Table 1, solution A.
- **OKLab coefficients** against Ottosson's reference implementation.

## D. Deliberately parked — fine as they are

- `00_philosophy.md` §7 (post-1.0, each with a reason).
- **Reversibility replay and KS regularisation** (integrator contract, Part 6).
- **Cross-chart payload sharing** (caching contract, Part 3).
- **Quantised checkpoints** — moot under lockstep.
- **Batch literature import** — a validation-phase tool.
- **Tier-table numbers** — guesses by design, calibrated from telemetry.

---

## E. Order of work

1. **Rule on A1** (markdown is authoritative) and port the 21 sections.
2. **Classify the 16 files** (A2) and close the register (A3).
3. **Record decisions B1–B25.**
4. **Write `gui-dev/*.md` and export the artboards** (A4).
5. **Write the build plan** (A5), with a requirement-ID traceability matrix, so no requirement can
   close without a passing test.
