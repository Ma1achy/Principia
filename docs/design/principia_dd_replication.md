# Principia: the replication and verification suite — published three-body results as engine tests

**Adopted by R-594 (25 Oct 2026) and R-629 (27 Oct 2026).** This document is part of the corpus from those rulings on.
R-594 adopted the replication suite of 25 Oct and asked for it as this design doc; R-629 made the consolidated plan of
27 Oct the single plan the port follows. The contract is this document. Its sources are records, transcribed here and
never normative (R-1, R-159):

*30 Oct 2026 (R-649, R-650):* every external target here is a measurement, never a gate: it declares the tier it binds,
its outcome is PASS, EXPLAINED DIVERGENCE or UNEXPLAINED, and it is scored in M9, the final milestone, after the core
build, or early as a peek, information only. The internal correctness tests and the anchor smoke tests stay
continuous and blocking. § 12 is the contract for all of it, and where a line above it says a target gates, or is
confirmed at a milestone's gate, § 12 governs.

- `docs/incoming/2026-10-27_validation_plan/PLAN.md` (the plan, which governs where the records differ, R-629);
- `docs/incoming/2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md` (the 25 Oct instruction: its rules
  1–8, carried over verbatim in § 8.1, and its tiers A–D, which PLAN.md keeps) and its `catalogue.md`;
- `docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md` (the orbit catalogue's six
  tests, § "What to do with it (an addition to tier A)", and its data terms);
- `docs/incoming/2026-10-27_papers/TARGETS.md` (49 numbered targets, transcribed from their papers; the authoritative
  transcription of each, which this document cites and does not copy).

PLAN.md superseded, as the working plan, the 25 Oct instruction's tier tables, its "Sequencing" and its "Papers to
fetch" (R-629). Those records stay records: nothing in them is edited by this port.

---

## 1. What this extends, and what it does not change

This document **extends `principia_dd_validation_orbits.md` and does not replace it** (R-594 item 2). That doc's §3
suite (figure-eight closure with its order check, free-fall closure, Lagrange rigidity, the chart landmarks,
reversibility, word calculus) stays where it is, with the requirements it carries. Its §4 says why it is not enough:
periodic orbits "are measure-zero and non-chaotic … they say nothing about behaviour in the chaotic sea, which is where
the instrument actually operates. **Necessary, not sufficient.**" This document adds what that suite cannot: results
other groups computed with other codes, on chaotic, non-periodic or unequal-mass systems, and the periodic-orbit
catalogues at scale.

**Nothing here changes a ruling or a requirement already in force** (R-594). The targets that are already requirements
are listed by id in § 5.1 and keep their statements, milestones and tasks. Every other target is a new requirement,
REQ-VAL-201 to REQ-VAL-331, each with its source, the chart it needs, its milestone, whether it is exact or
statistical (rule 8) and its tolerance as a calibration (R-71) — the five fields of R-594 item 3 — closed by the
tasks § 7 lists. Where a target's record disagrees with a document in force, the disagreement is queued, not applied
(§ 10, § 11).

---

## 2. Two layers and four tiers

**Verification: the code computes what it claims.** The reference is a property any correct implementation must
reproduce, whoever computed it first: a periodic orbit closes, an analytic solution stays analytic, the equations of
motion agree with a finite difference of the Hamiltonian, a symmetry leaves a quantity unchanged, f32 and f64 differ
only where the arithmetic says they may, and two implementations of one algorithm agree. Published periodic-orbit
initial conditions and their free-group words belong here: the paper supplies the initial condition and the period, but
the claim tested is ours.

**Replication: independent published results from other codes.** The reference is a quantity another group measured
with another integrator, on a system that is chaotic, or non-periodic, or both: lifetimes, escaper identities, maps,
binned statistics, step counts and energy errors, stability labels. Agreement there is evidence about the physics, not
just about our arithmetic. It is compared as a distribution unless the other code was converged (rule 8).

The two fail differently. A verification failure is a located bug. A replication disagreement is a finding to queue
(rule 4); the default suspicion for an old integrator is theirs, but it must be shown (rule 5).

*30 Oct 2026 (R-649):* for an external target of either layer the outcome is PASS, EXPLAINED DIVERGENCE (a named
mechanism, shown by a measurement) or UNEXPLAINED (open, labelled, non-blocking), never a failure that blocks; a
located bug a target exposes is fixed only when it is justified independently of the target (R-650's addendum). Each
target also declares the tier it binds, the CPU IC inspector (b) or the GPU explorer (a) (§ 12.2), beside the tiers
A–D below, which say what kind of result it is.

**Tiers** (the 25 Oct instruction's, kept by PLAN.md):

- **A, exact**: a deterministic value matched to a tolerance;
- **B, maps and statistics**: the chaotic sea;
- **C, numerical reliability**;
- **D, the prototype's own results**: each prototype number a target the new build reproduces or explains. A
  prototype number is not ground truth: it was measured once, on one implementation, and several were later
  overturned (R-594 item 7).

**Compare**, per target: *exact* (to a tolerance, a calibration under R-71), *statistical* (a distribution, at a
calibrated acceptance level), *qualitative* (morphology, reviewed by the physics reviewer against the figure, with a
control that must differ), or a *measurement* recorded beside the published number. Exact or statistical is rule 8's
choice: a chaotic result is statistical unless its source was a converged code.

**Charts**: an existing chart (`principia_chart_reference.md`; `BodyPlane`, `Latent`, `ShapeSphere`, the Burrau
family, `MassSimplex`, the invariant charts), the Anosova physical-frame entry of the lowering contract, "Path A" or
"Path B" (a known IC encoded, or injected physically, with no chart; `principia_validation_ground_truth_note.md`), or
"new: <name>" for a chart no task builds. A target needing a new chart waits for it, and the chart is not built ahead of
it (R-594 item 5); each such chart is a question for the human (§ 10).

---

## 3. Sources

The abbreviations PLAN.md uses, with the paths they stand for. Every one is a record.

| Abbr. | Path | What it is |
|---|---|---|
| R25 | `docs/incoming/2026-10-25_replication/` | the 25 Oct instruction, `catalogue.md` (about 35 sources) and `papers/` |
| P27 | `docs/incoming/2026-10-27_papers/` | `TARGETS.md` (49 targets), `README.md`, the three Agekyan & Anosova CSVs (twice-transcribed, its § 6), `transcriptions/`, `valtonen2016/` |
| AN24 | `docs/incoming/2026-10-24_anosova/` | the Agekyan & Anosova papers I and II, and Li & Liao's arXiv:1805.07980 |
| OC26 | `docs/incoming/2026-10-26_orbit_catalogues/` | the Three Body Orbits snapshot: `threebodyorbits_catalogue.json` (3,943 orbits, 41 families), `threebodyorbits_ics.csv`, `CITATIONS.md`, the instruction and Hudomal's 2015 thesis |
| BG26 | `docs/incoming/2026-10-26_belgrade_gallery/` | the recovered Belgrade gallery: `belgrade_gallery.csv` (301 rows), `crosscheck.txt`, Hudomal 2019's 206 sequence orbits |
| HP27 | `docs/incoming/2026-10-27_hp_ics/` | high-precision initial conditions: `agreement.txt`, `sources.tsv`, `fetch.sh`, and since R-631 the 599 source files under `data/`, cited in `SOURCES.md`; `fetch.sh` stays the provenance check. *Was:* "(values fetched and hash-checked, not committed: RQ-579)" |
| FF27 | `docs/incoming/2026-10-27_hp_ics_freefall/` | since R-631: Hristov et al. 2024's free-fall database (24,582 i.c.s at 80 digits, R-630 item 7), the central-symmetry (4,860) and Euler (12,431) databases at 100 digits, and Li & Liao's own 316-orbit free-fall table with initial conditions; committed under `data/`, cited in `SOURCES.md`, `fetch.sh` the provenance check |
| PL26 | `docs/incoming/2026-10-26_precision_ladder/` | the precision-ladder proposal; its §5 has twelve tests; queued as RQ-577, nothing in it in force |
| LR | `docs/incoming/2026-10-25_lit_review/` | the literature review, fourth revision: §13 periodic-orbit catalogues, §14 validation targets, §15 gaps |
| ATP | outside the repo | "Mapping the Three Body Orbits atlas into Principia: a plan" (26 Oct 2026), deliverables D1–D7, as PLAN.md reports them; not a source of any reference value |
| FND | `docs/reference/prin-rs/FINDINGS.md` | the prototype's findings, a record (R-159) |

**Data terms.** The atlas's catalogue and its extracted CSV are used under CC BY 4.0, attributed to
`https://www.threebodyorbits.com`, with the per-family credits of `OC26/CITATIONS.md`; every figure that uses them
carries that credit line. The gallery's own page grants use of its data. The operator's `atlas-params.json` is not
committed and never enters the repo (`scripts/check_no_atlas_params.sh`); nothing in this suite reads it. Hristov et
al.'s databases, Simó's archive and Li & Liao's tables state no terms; they are committed with full citations (R-631,
27 Oct 2026, closing RQ-579 with option (a)), in HP27 and FF27, each with a `SOURCES.md`, and each folder's `fetch.sh`
stays the provenance check. *Was:* "Hristov et al.'s databases, Simó's and Li & Liao's tables state no terms, so they
are fetched and hash-checked, not committed, while RQ-579 is open." The atlas's hunt orbits are not in the snapshot and stay out until the operator publishes them.

---

## 4. Three findings carried over from the plan

PLAN.md § 2 made three measurements while consolidating. They are recorded here because targets depend on them.

### 4.1 The atlas's 311 free-fall orbits against Li & Liao's 234

The atlas attributes 311 free-fall orbits to Li & Liao 2019; the paper reports 234. Measured on the committed snapshot
(family `freefall`, every row's masses, code and aliases):

| Mass set | Atlas rows | Paper (Table I) | Authors' website |
|---|---|---|---|
| (1, 1, 1) | 27 | 30 | 30 |
| (1, 0.8, 0.8) | 29 | 29 | 29 |
| (1, 0.8, 0.6) | 68 | 69 | 69 |
| (1, 0.8, 0.4) | 44 | 44 | 44 |
| (1, 0.8, 0.2) | 32 | 32 | 32 |
| (0.6, 0.8, 1) | 30 | 30 | 30 |
| (1, 0.1, 0.01) | 47 | — | 48 |
| (0.393, 0.288, 0.662) | 19 | — | 19 |
| (1, 0.412, 0.267) | 15 | — | 15 |
| **Total** | **311** | **234** | **316** |

- The extra orbits are the authors' own, published on their website (316 orbits, 82 on three mass sets the paper does
  not have), not in the paper. The website gives words only, no initial conditions. Its pages are recorded by URL and
  sha256 prefix in PLAN.md § 2.1 and not committed (RQ-585).
  *27 Oct 2026, R-631:* the authors' GitHub repository has a second copy of the list,
  `FF27/data/liliao_github/free-fall-3b-pictures.md`, committed under R-631, and it gives the initial conditions,
  (x, y, T) at 10 decimals, for all 316, the 82 website-only orbits included. The website pages stay recorded only
  (R-630 item 8). LL-5 still tests closure on the atlas's rows only (rule 13).
- The atlas holds 311 = 316 − 5: five rows carry aliases folding one listed orbit into another (F2 into F1, F13 into
  F12 and F24 into F23 at (1, 1, 1); F16 into F11 at (1, 0.8, 0.6); F47 into F45 at (1, 0.1, 0.01)). So the atlas
  carries 230 of the paper's 234 and 81 of the website's 82 extra.
- The aliases are one orbit listed twice: from the paper's printed (x, y, T), the scale-invariant period T·|E|^{3/2}
  of each pair agrees (F1 20.009755 against F2 20.009709; F12/F13, F23/F24 and F11/F16 to about 1e-10). PLAN.md's
  reading: a brake orbit is at rest at t = 0 and at T/2, so it passes through region D at rest twice and the scan
  found both. That is the alias-pair arm of REQ-VAL-203.
- The 81 website-only rows are closure tests only, never cited as the paper's results (rule 13; REQ-VAL-227).

### 4.2 The high-precision rows: 937, not 1,260

The precision-ladder proposal's test 6 counted "1,260 rows traced to their source paper". Verified against
`HP27/agreement.txt`: 1,359 atlas rows carry high-precision fields; **937 are traced to Hristov et al.** (9 at 150
digits, 240 at 180, 688 at 97–100), and only those can test arithmetic above f64. Simó's 344 rows carry about 16
significant digits of data and test nothing above f64. The reference values are the databases', committed
under `HP27/data/` (R-631; cited in `HP27/SOURCES.md`), with `HP27/fetch.sh` as the provenance check. These rows serve
the precision ladder (§ 5.13, PL-6), which waits for RQ-577. *Was:* "The reference values are the databases', fetched
and hash-checked by `HP27/fetch.sh` (RQ-579)."

### 4.3 The atlas's slice families

Re-measured on the atlas CSV:

- **1,759 equal-mass orbits on Šuvakov's plane** (`x = (−1, 1, 0)`, `v₁ = v₂`, `v₃ = −2v₁`).
- **1,350 on the same plane with m₃ free**, on **eight** values of m₃: 0.5 (565), 0.75 (401), 2 (237), 4 (85), 5 (35),
  8 (17), 10 (9), 1.001 (1).
- **311 free-fall orbits on nine planes**, one per mass set (§ 4.1), not one.
- 437 with |L| > 1e-6; 86 other.

Every Šuvakov-plane start is collinear, so in the chart reference's coordinates it lies on the configuration chart's
boundary: on `β = 0` or `β = π` (the Euler edges), or at the `α = 0` pole where `λ̃ = 0` and `β` is undefined, depending
on the labelling (Hudomal's Table 1 against his eq. (18)). Which labelling, and whether a chart over the plane is built,
are RQ-582 and RQ-583. Until they are ruled, Šuvakov-plane orbits are tested by Path A, which needs no chart.
*27 Oct 2026:* R-630 rules the labelling (RQ-582 closed): the middle body is body 1, at `β = 0`, Hudomal eq. (18)'s.
The chart, RQ-583, stays open.
*28 Oct 2026:* R-636 closes RQ-583: the Šuvakov-plane targets stay Path A closures, and no chart is built yet. The five
targets that need one (§ 5.12) wait on RQ-608.

---

## 5. The targets

One row per target, ids kept from PLAN.md § 3 (TARGETS.md's `P-1` … `V-11`, the atlas's `OC-n` and `AT-Dn`, and the
rest). Each new requirement's note carries the five fields of R-594 item 3; the tables give them in short. *L*:
verification (V) or replication (R). *T*: tier. *Tolerance*: the calibration requirement (R-71), or why there is none.

*30 Oct 2026 (R-650):* the *Milestone* and *Task* columns of § 5.2 to § 5.12 and of § 6 give M9 and each task's new
id; § 12.4 gives, for every row, the milestone and task R-594's port gave it. The tier each target binds (R-649 item 2)
is § 12.2's.

### 5.1 Targets already ruled

These are requirements already, from `principia_dd_validation_orbits.md`, the integrator, decoder, honesty and Chazy
docs. PLAN.md § 3 lists them as targets; this port changes none of them.

| PLAN.md row | Requirement | What it asserts | Milestone |
|---|---|---|---|
| Python cross-check | REQ-VAL-028 (+ -146, -119, -036) | BodyPlane reproduces the Python reference | M3 |
| Equations of motion | REQ-VAL-170 (+ -171) | analytic EOM against a finite difference of the Hamiltonian | M3 |
| Two-body radial collision | REQ-VAL-045 (+ -195) | `d_min < 1e-10` with `|dE/E| < 1e-12` | M3 |
| Order scaling | REQ-VAL-030 (+ -031, -139) | energy error scales as the occupant's order | M3 |
| Figure-eight (D-1's orbit) | REQ-VAL-041 (+ -133) | closure at the occupant's order | M3 |
| Lagrange, figure-eight curve | REQ-VAL-047 (+ -130) | analytic at every sampled t | M3 |
| Landmarks | REQ-VAL-020 | Lagrange at `z = 0`, Euler at the β edges | M2 |
| Burrau at ν = ½ | REQ-VAL-016 (+ -118) | (3, 4, 5) reproduced | M2 |
| Burrau rest start | REQ-VAL-174 | M = 12, R = 2.2361, E = −12.8167 | M2 |
| AA-1 | REQ-VAL-019 (+ -120) | region D imports (R-592) | M2 |
| Scale invariance | REQ-VAL-172 | `spread_shape` identical under rescaling | M5 |
| error_ratio | REQ-VAL-173 | 1.0000 at t = 13 on near-field | M5 |
| Convergence reference | REQ-VAL-025 (+ -131) | the arbitrary-precision reference converges | M3 |
| Escape precision and recall | REQ-VAL-040 (+ -051, -054, -123, -124) | against independent ground truth | M3 |
| Free-fall reversibility | REQ-VAL-044 (+ -134) | brake orbits time-symmetric by ξ | M3 |
| CPU/GPU decode | REQ-VAL-017 | branch tags bitwise | M2 |
| f32 horizon | REQ-VAL-070 (+ -071, -074) | measured on the GPU | M4 |
| Ensemble ratio | REQ-VAL-199 | median of measured over predicted parting ratio, against 1 (R-632) | M8 |
| Chazy f32/f64 | REQ-VAL-183 | measured, not asserted | M7 |
| D-2's gate | REQ-VAL-052 (+ -136) | re-registration insensitivity | M3 |
| D-3's gate | REQ-VAL-184 (+ -185 to -187) | the classifier against Fewbody | M7 |
| FF-H | REQ-VAL-042 | two or three free-fall brake orbits close | M3 |
| W-1 (+ OC-5's words, AT-D7) | REQ-VAL-043 | the computed word equals the published class (RQ-576) | M3 |
| REQ-VAL-076 (AT-D2's field) | REQ-VAL-076 (+ -137) | the closure field low at free-fall orbits | M4 |
| SP-1 (V-4) | REQ-VAL-034 (+ -127, -132) | Burrau: binary plus escaper, structurally | M3 |

Two of these now carry a record of what blocks them. REQ-VAL-042's source, Hristov et al. 2024's free-fall database,
is fetched (R-630 item 7, closing RQ-584) and committed (R-631) as `FF27/data/sofia_3bodyfree/sol_80.txt`, so its
`rq: RQ-584` is lifted; its citation is RQ-589's question. *Was:* "REQ-VAL-042's source, Hristov et al. 2024's
free-fall database, is not fetched (RQ-584), so it carries `rq: RQ-584`; its citation is RQ-589's question." REQ-VAL-043 already carries
RQ-576. Where PLAN.md folds a tier-D number into one of these rows (D-1, D-2, D-3) or adds an arm to it (AT-D2, OC-5's
count), the number or the arm is a new requirement below, and the ruled requirement is unchanged.

*30 Oct 2026 (R-650):* these rows stay where they are, continuous and blocking, as the internal correctness tests and
the anchor smoke tests, except three, which are external and scored in M9: FF-H (REQ-VAL-042); the figure-eight's
published shape-sphere curve, split from REQ-VAL-047 as REQ-VAL-333; and SP-1's structural comparison with Szebehely
& Peters, split from REQ-VAL-034 as REQ-VAL-335, with its feature set REQ-VAL-127. SP-1 itself runs to its own
horizon, `t_max` between 60 and 65, the default horizon unchanged (R-649's clarification 1). W-1 stays: R-125 makes it
the verification of the convention we transcribe (§ 12.1).

### 5.2 The orbit atlas

The orbit catalogue's six tests (its instruction, § "What to do with it (an addition to tier A)"), with the atlas
plan's encoding (AT-D6) and markers (AT-D2). The atlas is a compilation, not a primary source: a reference value traces
to the source paper, and the site's own orbits test closure only (rule 13). Its coordinates are the site's
centre-of-mass frame with G = 1 and masses as given, converted into the canonical frame by a tested function (rule 11).
The closure tolerance is set by the f64 rounding of `ref_*` times the orbit's instability, never by the catalogued
closure (rule 14). 667 of the 3,916 rows with refined closure below 1e-9 have T > 200, past R-610's cap; they are not
run while RQ-592 is open.
*28 Oct 2026:* R-636 closes RQ-592: these rows are recorded as "not evaluated within cap", counted and named, never as
failures. A separate raised-cap run evaluates them: TASK-M4-26, closing REQ-VAL-332, the raised cap's calibration.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| OC-1 (DRAFT-VAL-01) | REQ-VAL-201 | V / A | the Three Body Orbits atlas's catalogued periods and refined initial conditions (`threebodyorbits_ics.csv`, 3,916 rows with refined closure < 1e-9) | none (Path A) | M9 | exact | REQ-VAL-202 (calibration, R-71) | TASK-M9-02 |
| OC-2 | REQ-VAL-214 | V / A | the atlas's catalogued E and L per row (`E_cat`, `L_cat`) | none (Path A, through encode and decode) | M9 | exact | REQ-VAL-215 (calibration, R-71) | TASK-M9-01 |
| AT-D6 | REQ-VAL-216 | V / A | the atlas's published initial conditions, every row | Latent (8D) | M9 | exact | REQ-VAL-217 (calibration, R-71) | TASK-M9-01 |
| OC-3 | REQ-VAL-218 | V / A | the atlas's `closure_published` per row; the high-precision record's comparison of Li & Liao 2017's five rows with Hristov et al.'s digits (`docs/incoming/2026-10-27_hp_ics/agreement.txt`) | none (Path A) | M9 | measurement (a disagreement is information) | REQ-VAL-219 (calibration, R-71) | TASK-M9-03 |
| OC-5 (folded into W-1 by PLAN.md; the syzygy count only) | REQ-VAL-220 | V / A | the atlas's syzygy count `syz` per row | none (Path A) | M9 | exact (a count) | none: an integer count, compared exactly | TASK-M9-04 |
| OC-6 | REQ-VAL-222 | V / A | the atlas's 41 families | none | M9 | a report | none: a report | TASK-M9-05 |
| AT-D2 (folded into REQ-VAL-076's row by PLAN.md) | REQ-VAL-308 | V / B | the atlas's 311 free-fall orbits on their nine mass planes (§ 4.3) | Latent `z_q = 0`; MassSimplex points | M9 | qualitative + a calibrated threshold | REQ-VAL-137's (the existing threshold for REQ-VAL-076) | TASK-M9-34 |
| OC-4 = AT-D4 (+ AT-D5), off the Šuvakov plane | REQ-VAL-309 | R / B | the atlas's `stab`, `stable_pub` and `stab_src` per orbit | Latent `z_q = 0`; others per family | M9 | statistical (an agreement table, every disagreement listed) | REQ-VAL-310 (calibration, R-71) | TASK-M9-33 |

**OC-5's counting convention** (REQ-VAL-221, a definition, R-72): written here by the task that needs it.

### 5.3 Free-fall periodic orbits: Li & Liao 2019

Li & Liao 2019 (*New Astron.* 70, 22; arXiv:1805.07980, in AN24): 234 collisionless periodic free-fall orbits over six
mass sets, Tables A1–A5 (10 decimals), with words in Tables A6–A10. The reference is the paper's tables, not the
atlas's rows (§ 4.1).

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| LL-1, LL-2 (DRAFT-VAL-02) | REQ-VAL-203 | V / A | Li & Liao 2019, Tables A1–A5 (234 initial conditions and periods, 10 decimals) | the Anosova physical-frame entry; a MassSimplex point for the five unequal mass sets | M9 | exact | REQ-VAL-204 (calibration, R-71) | TASK-M9-06 |
| LL-3 | REQ-VAL-223 (waits: the owner's copies of the two papers, paywalled, R-630 item 9; was "waits: RQ-586") | V / A | Li & Liao 2019 p. 3: F1 at (0.6, 0.8, 1) is Standish's 1970 orbit; F1 and F2 at (1, 1, 1) are Yasko & Orlov's 2015 Orbits 16 and 17 | the Anosova physical-frame entry; a MassSimplex point | M9 | exact | REQ-VAL-224 (calibration, R-71) | TASK-M9-07 |
| LL-4 | REQ-VAL-225 | V / A | Li & Liao 2019 p. 3 and Fig. 3: T̄* ≈ 2.363 at equal masses; 1.663, 1.191, 0.776, 0.527 at m₃ = 0.8 … 0.2; fit 1.912 m₃ + 0.084 | none | M9 | exact per orbit; the fit statistical | REQ-VAL-226 (calibration, R-71) | TASK-M9-14 |
| LL-5 | REQ-VAL-227 | V / A | the authors' website list of 316 free-fall orbits (82 beyond the paper's 234), carried by the atlas as 81 rows on three further mass sets | MassSimplex points | M9 | exact (closure only) | OC-1's (REQ-VAL-202): the same form, the same f64 refined initial conditions | TASK-M9-06 |

### 5.4 The Belgrade orbits

Šuvakov & Dmitrašinović 2013 (*PRL* 110, 114301, Table 1), the recovered gallery (BG26), Hudomal's 2015 thesis (OC26)
and Dmitrašinović et al. 2018 (*J. Phys. A* 51, 315101). All on the Šuvakov plane or near it, so all by Path A until a
chart is ruled (§ 4.3).

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| BG-4 (DRAFT-VAL-07) | REQ-VAL-213 | V / A | the Belgrade gallery's two published periods each for butterfly I (6.235641 / 6.234671) and butterfly II (7.003907 / 7.003707) | none (Path A) | M9 | exact (a measurement against a published discrepancy) | none: the bar is the published periods' own difference, which the measured uncertainty must undercut (PLAN.md § 6 gives it no calibration) | TASK-M9-12 |
| SD-1 (+ the gallery's cross-check 3) | REQ-VAL-228 | V / A | Šuvakov & Dmitrašinović 2013, PRL 110, 114301, Table 1 (15 initial conditions and periods) | none (Path A); the map part is HU-1 | M9 | exact | REQ-VAL-229 (calibration, R-71) | TASK-M9-13 |
| BG-1 | REQ-VAL-230 | V / A | the Belgrade gallery's 206 sequence orbits (`belgrade_gallery.csv`, identical to Hudomal 2019 in every printed digit) | none (Path A) | M9 | exact | REQ-VAL-231 (calibration, R-71) | TASK-M9-13 |
| BG-2 | REQ-VAL-232 | V / A | the gallery's 57 BHH satellites at non-zero angular momentum; the atlas's 15 `bhhsat` rows (the site's own, `bhh2026`) | none (Path A); InvariantLE / LK where a chart is wanted | M9 | exact | REQ-VAL-233 (calibration, R-71) | TASK-M9-13 |
| HU-3 (+ the gallery's cross-check 4) | REQ-VAL-234 | V / A | Hudomal 2015 (MSc thesis), Table 23: 38 orbits, six-digit initial conditions, periods and return proximities | none (Path A) | M9 | exact | REQ-VAL-235 (calibration, R-71) | TASK-M9-13 |
| HU-4 (+ Dmitrašinović et al. 2018; the 25 Oct tier A's 'period against word length') | REQ-VAL-236 | V / A | Hudomal 2015 §4.1 (Tables 3, 5, 7, 9, 11, 13, 15) and Dmitrašinović et al. 2018: the seven linear fits of T\|E\|^{3/2} against word length | none | M9 | exact per orbit; the fits statistical | REQ-VAL-237 (calibration, R-71) | TASK-M9-14 |
| HU-5 | REQ-VAL-238 | V / A | Hudomal 2015 Table 2 (from its Ref. [22]): periods at E = −0.5 and their ratios to T_M8 = 26.1281 (for example yarn / moth I = 3.00114 against a word ratio of 3) | none (Path A) | M9 | exact | REQ-VAL-239 (calibration, R-71) | TASK-M9-13 |

### 5.5 Regularisation

Heggie 1974 (*Celest. Mech.* 10, 217) and Tanikawa & Mikkola 2021 (*CeMDA* 133, 52). Heggie's integrator was RKF7(8),
which we do not ship, so the compared quantity for steps is a ratio under one stepper; the default occupant is Heggie ×
RK4 (R-560).

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| H-3 (DRAFT-VAL-03) | REQ-VAL-205 | R / A | Heggie 1974, Table III (C/3 against Δ toward the Lagrange triple collision) | ShapeSphere at rest through Lagrange; Latent | M9 | exact (scaling) | REQ-VAL-206 (calibration, R-71) | TASK-M9-08 |
| H-1, H-2 (DRAFT-VAL-04) | REQ-VAL-207 | R / C | Heggie 1974, Tables I and II (steps and \|δE/E\|; step ratios 147/129, 257/225, 455/397; 48/42, 83/70, 149/124) | BodyPlane Burrau pixel (H-1); none, Path A (H-2) | M9 | ratio, recorded both ways | REQ-VAL-208 (calibration, R-71) | TASK-M9-08 |
| TM-1 | REQ-VAL-240 (waits: the owner's copy of the paper, paywalled, R-630 item 9; was "waits: RQ-586") | R / A | Tanikawa & Mikkola 2021, Tables 1, 2 and 4 (eleven triple-collision points, e.g. (0.19208270, 0.30936018); approach ICs at radii 1e-3 to 1e-5) | the Anosova physical-frame entry, after the D11 mirror remap | M9 | exact (scaling) | REQ-VAL-241 (calibration, R-71) | TASK-M9-09 |

### 5.6 Burrau and the Pythagorean family

The Burrau programme's own family, integrated by others. Burrau's breakup near t ≈ 59.4 sits past f64's horizon of
about 52 (`principia_dd_validation_orbits.md` §1.5) and inside R-610's cap of 200.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| CH-1 | REQ-VAL-242 | R / A | Chitan, Mylläri & Haque 2022 (MNRAS 509, 1919): Newtonian lifetimes and two-body encounter counts for the 16 Pythagorean triangles with hypotenuse < 100 | the Burrau family; MassSimplex points | M9 | statistical (structural per triangle; rule 8) | REQ-VAL-243 (calibration, R-71) | TASK-M9-16 |
| B-1 (+ the 25 Oct tier A's Burrau lifetime) | REQ-VAL-244 | R / A | Boekholt, Moerman & Portegies Zwart 2021, Fig. 3 (bottom): the Newtonian lifetime plateau log₁₀ T_l ≈ 1.8 (56 to 71); the paper prints no value | BodyPlane, the Burrau pixel | M9 | exact to the figure's resolution | REQ-VAL-245 (calibration, R-71) | TASK-M9-16 |
| B-2 (+ the 25 Oct tier A's Burrau Lyapunov time) | REQ-VAL-246 | R / B | Boekholt, Moerman & Portegies Zwart 2021, Eq. 1 and Fig. 5: λ₁₀ = 0.101 ± 0.005, τ_L = 4.3 ± 0.2, fitted over all ζ, relativistic runs included | BodyPlane, the Burrau pixel | M9 | statistical (loose) | REQ-VAL-247 (calibration, R-71) | TASK-M9-16 |
| V-6 | REQ-VAL-248 | R / A | Valtonen et al. 2016, Fig. 3.16 (p. 79): Mikkola's near-Pythagorean bounce orbit, digitised starts (`fig3_16_mikkola_markers.csv`) | BodyPlane around Burrau; Latent | M9 | exact once shot; qualitative against the figure | REQ-VAL-249 (calibration, R-71) | TASK-M9-16 |
| PL-8, its first half | REQ-VAL-250 | V / C | R-33's independent arbitrary-precision reference, on the nominal Burrau pixel to t ≈ 60 | BodyPlane, the Burrau pixel | M3 | exact | REQ-VAL-251 (calibration, R-71) | TASK-M3-27 (was TASK-M3-56: R-650; internal, § 12.1) |
| SP-2, the converged Burrau reference (an anchor smoke test, R-650) | REQ-VAL-336 | R / A | Portegies Zwart & Boekholt 2018 (CNSNS 61, 160; arXiv:1802.00970v1), Table 2 (`docs/incoming/2026-10-29_burrau_pzb2018/table2/table2_reconciled.csv`, transcribed twice), from Table 1's start (SP-1's): the state at t = 100, unperturbed and perturbed (δx = 10⁻¹⁰ on mass 3); after every velocity is reversed at t = 100, the state at t = 200, against the initial condition (a reversibility check) and against Table 2's t = 200 values (a trajectory check) (R-646) | BodyPlane, the Burrau pixel; R-33's convergence reference | M3 | exact against a convergence reference, for gross breakage only | REQ-VAL-337 (calibration, R-71) | TASK-M3-27 |

*30 Oct 2026 (R-650):* PL-8's first half compares our f64 with our own R-33 reference, so it is internal and stays
in M3, closed by TASK-M3-27 beside SP-1 and SP-2. SP-2 was PLAN.md § 3.2's proposed row (28 Oct 2026; R-646 its
source and its reversibility check); R-650 names it an anchor smoke test, so it is ported here as one, catching gross
breakage only and never steering design. Its run is 200 time units, at R-610's cap and not past it; whether the
`t = 200` half is one run with the reversal inside it or a second run from the reversed `t = 100` state is the task's
choice, recorded in its PR, both reversing every velocity at `t = 100`.

### 5.7 The Burrau neighbourhood: Aarseth et al. 1994a and 1994b

Aarseth, Anosova, Orlov & Szebehely 1994a (*CeMDA* 58, 1) sample six circles × 100 azimuths around each body, not a
grid; 1994b (*CeMDA* 60, 131) gives triple-approach statistics over region D and circle R. Their escape criteria differ
between the companion papers (5R_grav for conditional escape in 1994a, 10d in 1994b) and are recorded as data (rule 9).
Tables I–VIII of 1994a and I–III of 1994b are transcribed in TARGETS.md.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| Q-2's Theory row, Q-4 (DRAFT-VAL-06) | REQ-VAL-211 | R / B | Aarseth et al. 1994b, Table II's 'Theory' row (1, 2, 3, 4, 5, 7, 9, 11, 16, 44 %; median 0.87) and p. 135's Z control (17, 16, 14, 14, 12, 10, 8, 5, 4, 1 %) | none (no integration) | M9 | (i) exact; (ii) statistical | REQ-VAL-212 (calibration, R-71) | TASK-M9-11 |
| P-1 | REQ-VAL-252 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), Figs. 2 and 3, Table II | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M9 | qualitative + statistical | REQ-VAL-253 (calibration, R-71) | TASK-M9-17 |
| P-2 | REQ-VAL-254 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), Table I | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M9 | statistical | REQ-VAL-255 (calibration, R-71) | TASK-M9-17 |
| P-3 | REQ-VAL-256 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), Fig. 4 | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M9 | qualitative | none: qualitative, reviewed against the figure by the physics reviewer | TASK-M9-17 |
| P-4 | REQ-VAL-257 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), Tables II–VII | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M9 | statistical | REQ-VAL-258 (calibration, R-71) | TASK-M9-17 |
| P-5 | REQ-VAL-259 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), Table VIII | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M9 | statistical | REQ-VAL-260 (calibration, R-71) | TASK-M9-17 |
| P-6 | REQ-VAL-261 | R / B | Aarseth, Anosova, Orlov & Szebehely 1994a (CeMDA 58, 1), p. 8 | BodyPlane, a zoomed polar sector | M9 | qualitative | none: qualitative, reviewed against the figure by the physics reviewer | TASK-M9-17 |
| Q-1 | REQ-VAL-262 | R / B | Aarseth et al. 1994b (CeMDA 60, 131), Table I | the Anosova physical-frame entry (region D); Path B for circle R | M9 | statistical | REQ-VAL-263 (calibration, R-71) | TASK-M9-18 |
| Q-2 | REQ-VAL-264 | R / B | Aarseth et al. 1994b, Table II | as Q-1 | M9 | statistical | REQ-VAL-265 (calibration, R-71) | TASK-M9-18 |
| Q-3 | REQ-VAL-266 | R / B | Aarseth et al. 1994b, Table III | as Q-1; P-1's rings | M9 | statistical | REQ-VAL-267 (calibration, R-71) | TASK-M9-18 |

### 5.8 Region D: Agekyan & Anosova, and Anosova 1969

Agekyan & Anosova I (*Sov. Astron.* 11, 1006) and II (*Astrophysics* 4, 11), whose region D R-592 ported; their
decay-time columns are transcribed twice in P27's three CSVs. Each row's decay time is in that row's own τ (paper I
eq. 7), and G = 6.67×10⁻⁸ in their units: the remap is a tested function (rule 11). Anosova 1969 (*Astrophysics* 5, 81)
adds three unequal-mass sets.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| AA-6 (DRAFT-VAL-05) | REQ-VAL-209 | R / A | Agekyan & Anosova II, Table 1 (decay time per exact-rest start; ensemble 110.1 ± 12.5 τ; Table 2's counts) | the Anosova physical-frame entry at exact rest | M9 | exact on the 13 short-lived rows; statistical otherwise | REQ-VAL-210 (calibration, R-71) | TASK-M9-10 |
| AA-2 | REQ-VAL-268 | R / A | Agekyan & Anosova I, Table 1 cols. 5–8 | the Anosova physical-frame entry, eq. (3) velocities | M9 | exact on the short-lived rows; statistical otherwise | REQ-VAL-269 (calibration, R-71) | TASK-M9-19 |
| AA-3 | REQ-VAL-270 | R / B | Agekyan & Anosova I, pp. 1012–1014 and Fig. 4 | the Anosova physical-frame entry | M9 | statistical | REQ-VAL-271 (calibration, R-71) | TASK-M9-19 |
| AA-4 | REQ-VAL-272 | R / B | Agekyan & Anosova I, Figs. 2a–2h | none (Path A, one trajectory) | M9 | qualitative | none: qualitative, reviewed against the figure by the physics reviewer | TASK-M9-19 |
| AA-5 | REQ-VAL-273 | R / B | Agekyan & Anosova I, pp. 1006, 1008–1009 | the Anosova physical-frame entry | M9 | statistical | REQ-VAL-274 (calibration, R-71) | TASK-M9-19 |
| AA-7 | REQ-VAL-275 | R / B | Agekyan & Anosova II, Table 3 | the Anosova physical-frame entry, eq. (3) velocities | M9 | statistical | REQ-VAL-276 (calibration, R-71) | TASK-M9-19 |
| AA-8 | REQ-VAL-277 | R / B | Agekyan & Anosova I Table 1, II Tables 1 and 3 | Latent with one momentum axis, or two runs per start | M9 | statistical | REQ-VAL-278 (calibration, R-71) | TASK-M9-19 |
| AN-1 | REQ-VAL-279 | R / B | Anosova 1969 (Astrophysics 5, 81), its tables | the Anosova physical-frame entry at a mass point; MassSimplex | M9 | statistical | REQ-VAL-280 (calibration, R-71) | TASK-M9-19 |

### 5.9 Region D: maps and statistics

Lehto et al. 2008 (*MNRAS* 388, 965) and Valtonen et al. 2016 (*The Three-body Problem from Pythagoras to Hawking*),
through Lehto's lens and the book's ×2 scale, each a tested remap (rules 6 and 11); Orlov, Rubinov & Shevchenko 2010;
Mikkola & Tanikawa 2007. The small-grid targets run at M3 on the validation crate's CPU harness; the full-resolution
map at M4 and the zooms at M6.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| L-3 (+ the 25 Oct tier B's Maasai shield statistics) | REQ-VAL-281 | R / B | Lehto et al. 2008 §2.3: 6,212 of 21,699 (28.6 %) locally homogeneous at ε = 1.25×10⁻³; 28.6 → 8.6 % over six bin sizes; p_ε ∝ ε^−0.5 | the Anosova entry, or ShapeSphere at rest, through a tested lens remap (rule 6) | M9 | statistical | REQ-VAL-282 (calibration, R-71) | TASK-M9-20 |
| L-4 | REQ-VAL-283 | R / C | Lehto et al. 2008 §3 and Figs. 4–9: the reversibility ξ map and populations I and II (Fig. 9 fits) | as L-3 | M9 | statistical / qualitative | REQ-VAL-284 (calibration, R-71) | TASK-M9-20 |
| L-5 | REQ-VAL-285 | R / B | Lehto et al. 2008 §4: step halving only sharpens boundaries; an unregularised run reproduces the main structures | as L-3 | M9 | qualitative | none: qualitative, reviewed by the physics reviewer | TASK-M9-20 |
| V-9 (+ Orlov et al. 2010; the 25 Oct tier B's lifetime tails) | REQ-VAL-286 | R / B | Valtonen et al. 2016 p. 77; Orlov, Rubinov & Shevchenko 2010: the t_end histogram's tail, power law (differential index about −1.4 to −1.7) or exponential | the Anosova physical-frame entry; P-1's rings | M9 | statistical | REQ-VAL-287 (calibration, R-71) | TASK-M9-21 |
| MT-1 | REQ-VAL-288 | R / B | Mikkola & Tanikawa 2007 (MNRAS 379, L21): t_d ≈ 10 t_e^1.8 on free fall, a two-segment fit | the Anosova physical-frame entry | M9 | statistical | REQ-VAL-289 (calibration, R-71) | TASK-M9-21 |
| L-1 (+ the 25 Oct tier B's Maasai shield) | REQ-VAL-290 | R / B | Lehto et al. 2008 Fig. 1: the 1:1 band, the odd resonances, the sea of tranquillity, the trenches, the symmetry | as L-3 | M9 | qualitative | none: qualitative; L-3's statistic at its own calibrated level | TASK-M9-26 |
| V-2 / V-8 (a) | REQ-VAL-291 | R / B | Valtonen et al. 2016, Figs. 4.5–4.7: the full Maasai shield | as L-3 | M9 | exact | REQ-VAL-292 (calibration, R-71) | TASK-M9-26 |
| L-2 | REQ-VAL-293 | R / B | Lehto et al. 2008, Figs. 2 and 3: the three-part yellow area; the 3.125×10⁻⁶ zoom | as L-3, at deep zoom | M9 | qualitative | none: qualitative | TASK-M9-31 |
| V-8 (b) | REQ-VAL-294 | R / B | Valtonen et al. 2016, Figs. 4.5–4.7: the three zoom windows (`fig4_5to4_9_axis_windows.csv`) | as L-3, at deep zoom | M9 | qualitative | none: qualitative | TASK-M9-31 |

### 5.10 Converged-code references

Rule 8's exception: a result computed with a converged code (Brutus) is a set of trajectories, compared per point.
Neither data set is in the repo (RQ-598), and Boekholt et al.'s t = 10⁴ is past R-610's cap, so it is compared only
over the systems that end before 200 (rule 18; RQ-592).
*28 Oct 2026:* R-636 closes RQ-592: GG-1's systems that end past 200 are recorded as "not evaluated within cap", never
as failures, and GG-1 waits on RQ-598 alone.
*28 Oct 2026:* R-637 closes RQ-598 with option (a), and R-638, R-640, R-641 and R-642 rule how GG-1 and BR-1 are
scored, R-642 governing where they differ (`docs/incoming/2026-10-28_rq598_data/README.md` § 6 is the record):
- **The authors' own data scores directly, not provisionally.** BR-1 against the 180 trajectories of
  `github.com/pgbreen/NVM` at `a35c378`, never committed (no licence; their per-file sha256 table is in the record),
  which cover 180 of the paper's ~10⁴; why 44 of them end early is checked before they are scored. GG-1 against the
  Fig. 1 extraction (`transcriptions/gg1_fig1/`), all 1212 points per panel, at one LUT step (3/256 ≈ 0.0117 dex), the
  three-entry bin (LUT 29–31) at its wider precision, black meaning censored above 1000. Those parts may close on that
  scoring. The two panels (ε = 10⁻⁶ and ε = 10⁻⁷⁰) differ point by point; GG-1's per-point reference panel is
  undefined: RQ-620 (R-72). No panel is chosen.
- **A reproduction unblocks provisionally.** It is labelled "vs <source>" ("vs Brutus, run here" for ours), never
  "replicates the paper", and qualifies only if it matches the paper's set-up (the same initial-condition distribution,
  at rest, the same frame) and uses a converged arbitrary-precision integrator: our Brutus built from
  `github.com/pgbreen/NvMgeneration` at `98906f4` (GPL-3.0), the paper's own, qualifies. Rejected:
  `dosier/TBP-Group-39-TrainData` and `stipefrkovic/three-body-problem`.
- **Re-scored when the authors' data arrives:** every result against a reproduction is re-scored against it; where ours
  and theirs disagree, theirs governs and the disagreement is queued. BR-1 and GG-1 stay open until scored against the
  authors' data, for what that data covers.
- R-638's gate stands: the measured cost per point is reported before any Brutus run is scaled or goes below
  ε = 10⁻⁶.

*30 Oct 2026 (R-649, R-650):* GG-1 and BR-1 are measurements, scored in M9 (TASK-M9-22) or, early, as peeks;
"those parts may close on that scoring" means in M9. Their tiers: BR-1 (b); GG-1 (b) per point and (a) for its KS
part (§ 12.2; *was:* "GG-1 (b) per point, its KS part (b) with (a) expected to agree statistically", corrected by the
owner's confirmation of 30 Oct 2026, R-649). The first peek, GG-1 at three points from PR #335 ("Brutus, run here"
against Fig. 1), is UNEXPLAINED (`docs/verification/peek_log.md`; RQ-612 holds its like-for-like questions).

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| GG-1 (the 25 Oct tier B's converged lifetime grid) | REQ-VAL-295 (scored in M9, R-650; its per-point reference panel is RQ-620's; *was:* "waits: RQ-592, RQ-598", both closed, by R-636 and R-637) | R / B | Boekholt, Portegies Zwart & Valtonen 2020 (MNRAS 493, 3932): 1,212 points at spacing 1/64, converged with Brutus to t = 10⁴; lifetime to the last close approach | the Anosova physical-frame entry | M9 | per point where reliable (rule 8: a converged code); KS otherwise | REQ-VAL-296 (calibration, R-71) | TASK-M9-22 |
| BR-1 | REQ-VAL-297 (scored in M9, R-650; *was:* "waits: RQ-598", closed by R-637) | R / C | Breen et al. 2020 (MNRAS 494, 2465): per-trajectory Brutus runs to t = 10 | the Anosova physical-frame entry (Breen's frame) | M9 | exact (rule 8: a converged code) | REQ-VAL-298 (calibration, R-71) | TASK-M9-22 |

### 5.11 The prototype's results (tier D)

prin-rs measured hundreds of numbers, each with the settings that produced it. The five numbers the 25 Oct instruction's
tier D names become targets; each is reproduced or the difference explained by a measurement, and several were measured
under Aarseth–Zare where the default occupant is now Heggie × RK4 (R-560). Whether the rest are enumerated as targets is
RQ-587 (D-6); until it is ruled, a prototype number becomes a target by name when a task needs it.

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| D-1 (folded into REQ-VAL-041's row by PLAN.md) | REQ-VAL-299 | V / D | the prototype's figure-eight closure order: 2.40 with the landing clamp and 1.03 without under Heggie; 2.08 and 1.06 under Aarseth–Zare (FINDINGS § 3.2) | none (Path A) | M9 | measurement: reproduce or explain | REQ-VAL-300 (calibration, R-71) | TASK-M9-15 |
| D-2 (folded into REQ-VAL-052's row by PLAN.md) | REQ-VAL-301 | V / D | the prototype's drift-field chord under doubled re-registration: 4.832e-2 (Heggie) against 5.162e-1 (Aarseth–Zare), `config_stability`, 256² (FINDINGS § 2.6) | `config_stability` | M9 | measurement: reproduce or explain | REQ-VAL-302 (calibration, R-71) | TASK-M9-15 |
| D-4 | REQ-VAL-303 | V / D | the prototype's f32 against f64 outcome labels on near-field, 32 × 32, t = 13 (Aarseth–Zare): 1022 of 1024 agree; 2 pixels with \|dE/E\| > 1; 152 of 1024 flip on the unconditioned branch (honesty_precision § 1) | BodyPlane (near-field) | M9 | measurement: reproduce or explain | REQ-VAL-304 (calibration, R-71) | TASK-M9-15 |
| D-5 | REQ-VAL-305 | V / D | the prototype's finding: no near-field escape at t = 13 (FINDINGS § 4) | BodyPlane (near-field) | M9 | exact (a count of 0) | none: a count of 0, compared exactly | TASK-M9-15 |
| D-3 (folded into REQ-VAL-184's row by PLAN.md) | REQ-VAL-306 | V / D | the prototype's classifier against Fewbody's termination test: no wrong commit on E < 0; median first correct label 51.6 / 40.8 against Fewbody's 69.6 / 69.6 (δ = 1e-5) (Chazy drill-down § 4.1) | free-fall; Burrau | M9 | exact on the fixtures; statistical for the medians | REQ-VAL-307 (calibration, R-71) | TASK-M9-32 |

### 5.12 Targets that wait for a chart, a diagnostic or data

Each waits, with its requirement in place and its RQ named, for what it needs. Nothing is built ahead of the ruling
(R-594 item 5).

| Waits for | Targets | RQ |
|---|---|---|
| a chart over the Šuvakov plane (its labelling ruled by R-630, closing RQ-582) | HU-1, HU-2, AT-D1, AT-D3, OC-4 on the plane | RQ-583 (was "RQ-583, RQ-582") |
| a binary-plus-single chart | TR-1, MK-1, SL-1 | RQ-593 |
| a side-ratio chart, or a remap onto ShapeSphere at rest | PA-1 | RQ-594 |
| a syzygy-symbol field and a fixed-k velocity chart | TS-1, TS-2, V-7 | RQ-595 |
| the N_scramble and f_col diagnostics | PA-1's split, SL-1, TR-1's colouring | RQ-596 |
| periodic-orbit continuation | LLL-1 | RQ-597 |
| data not in the repo | V-11, GG-1, BR-1 | RQ-598 |
| papers not in the repo: paywalled, for the owner to fetch (R-630 item 9, closing RQ-586) | LL-3, TM-1 | none (was "RQ-586") |

*28 Oct 2026:* R-636 closes RQ-583 with no chart built yet. The Šuvakov-plane row's five targets, and the rows below
that name RQ-583, wait on RQ-608.
*28 Oct 2026:* R-639 splits each Šuvakov-plane target into a closure part and a chart part, queuing any target whose
split isn't clean; none of the five splits cleanly (R-639), so all five are RQ-618's and stay as they are, under
RQ-608. R-637 closes RQ-598: V-11's primaries are fetched (Part I held locally, never committed; Part II committed,
CC BY 4.0), and V-11 proceeds from Part I, read from the University of Turku repository's version (R-638). The
data-not-in-the-repo row above stands for GG-1 and BR-1 only as far as the authors' full data goes (§ 5.10's note).

| Target | Requirement | L / T | Reproduces | Chart | Milestone | Compare | Tolerance | Task |
|---|---|---|---|---|---|---|---|---|
| OC-4 on the Šuvakov plane | REQ-VAL-311 (waits: RQ-583) | R / B | as OC-4, for the 1,759 equal-mass and 1,350 m₃-free rows on the Šuvakov plane (§ 4.3) | new: SuvakovPlane, at equal masses and at fixed m₃ | M9 | statistical | OC-4's (REQ-VAL-310) | TASK-M9-35 |
| AT-D1 | REQ-VAL-312 (waits: RQ-583) | R / B | the atlas's 1,759 equal-mass orbits on the Šuvakov plane, the stable ones highlighted | new: SuvakovPlane | M9 | qualitative | none: qualitative | TASK-M9-35 |
| AT-D3 | REQ-VAL-313 (waits: RQ-583) | R / B | the atlas's 1,350 m₃-free orbits on the eight values of m₃ | new: SuvakovPlane at fixed m₃ | M9 | qualitative | none: qualitative | TASK-M9-35 |
| HU-1 | REQ-VAL-314 (waits: RQ-583) | R / B | Hudomal 2015 Fig. 8: the return-proximity map over (p₁, p₂) ∈ [0.3, 0.5] × [0.1, 0.5]; Table 23's minima | new: SuvakovPlane | M9 | qualitative; exact at Table 23's minima | REQ-VAL-315 (calibration, R-71) | TASK-M9-27 |
| HU-2 | REQ-VAL-316 (waits: RQ-583) | R / B | Hudomal 2015 Fig. 34 and Table 23: the butterfly III zoom, three candidates, the middle one at (0.405916, 0.230163), T = 13.866 | new: SuvakovPlane, zoomed | M9 | qualitative + one exact point | REQ-VAL-317 (calibration, R-71) | TASK-M9-27 |
| TR-1 | REQ-VAL-318 (waits: RQ-593, RQ-596) | R / B | Trani et al. 2024 (A&A 689, A24): the inclination-0 row of their map; f_col (12 nearest neighbours, regular above 2/3) | new: BinaryPlusSingle | M9 | qualitative | none: qualitative | TASK-M9-23 |
| MK-1 | REQ-VAL-319 (waits: RQ-593) | R / B | Manwadkar, Kol, Trani & Leigh 2021: per-body escape probabilities over eight mass sets (Table 1) | new: BinaryPlusSingle at a mass point | M9 | statistical | REQ-VAL-320 (calibration, R-71) | TASK-M9-23 |
| SL-1 | REQ-VAL-321 (waits: RQ-593, RQ-596) | R / B | Stone & Leigh 2019 (Nature 576, 406): the planar closed forms for the binary's energy and eccentricity | new: BinaryPlusSingle | M9 | statistical | REQ-VAL-322 (calibration, R-71) | TASK-M9-23 |
| PA-1 | REQ-VAL-323 (waits: RQ-594, RQ-596) | R / B | Parischewsky, Trani & Leigh 2023 (SciPost Phys. Core 6, 016): escaper maps in side-ratio space around Lagrange; 82.1 % chaotic, 17.9 % regular | new: SideRatio, or ShapeSphere at rest by a tested remap | M9 | qualitative; the split statistical | REQ-VAL-324 (calibration, R-71) | TASK-M9-24 |
| TS-1 (the 25 Oct tier B's symbol-sequence maps) | REQ-VAL-325 (waits: RQ-595) | R / B | Tanikawa & Mikkola 2008 (arXiv:0802.2465) and 2015 (PASJ 67, 115): maps of the first 2–4 syzygy symbols over the free-fall plane | the Anosova physical-frame entry, through the D11 mirror remap | M9 | qualitative | none: qualitative | TASK-M9-28 |
| TS-2 | REQ-VAL-326 (waits: RQ-595) | R / B | Tanikawa & Mikkola 2008, Fig. 7: first-symbol maps at non-zero angular momentum, virial ratio 0 to 0.7 | new: a fixed-k velocity prescription over D11 ∪ D12 | M9 | qualitative | none: qualitative | TASK-M9-28 |
| V-1 / V-3 / V-7 | REQ-VAL-327 (waits: RQ-595) | R / B | Valtonen et al. 2016, Figs. 4.1 and 4.9 (fits in `valtonen2016/data/fig4_1_*`): the L/M/A/H partition; the high-entropy share 0.66–0.83 against the text's 'roughly one half' | the Anosova physical-frame entry, at the book's ×2 scale | M9 | qualitative; the share statistical | REQ-VAL-328 (calibration, R-71) | TASK-M9-28 |
| LLL-1 | REQ-VAL-329 (waits: RQ-597) | R / B | Li, Li & Liao 2020 (arXiv:2007.10184): period, angular momentum and stability contours over (m₁, m₂) for one family (13,315 stable of 135,445) | MassSimplex | M9 | qualitative | none: qualitative | TASK-M9-29 |
| V-11 | REQ-VAL-330 (scored in M9, R-650; *was:* "waits: RQ-598", closed by R-637) | R / B | Valtonen et al. 2016 pp. 82–84, 114, 140, and its primaries (Valtonen & Karttunen 2006; Mylläri et al. 2018): hierarchical stability limits, prograde and retrograde | Latent, or a hierarchical slice | M9 | statistical | REQ-VAL-331 (calibration, R-71) | TASK-M9-25 |

### 5.13 The precision ladder: waiting for RQ-577

The precision-ladder proposal's twelve tests, PL-1 to PL-12, and the two targets that need a precision tier above f64,
B-3 (a clean forward–backward Burrau run to t = 100 needs ε = 1e-24 and 128-bit words) and GG-2 (about 1 % of region D
irreversible at ε = 10⁻⁷⁰), wait for RQ-577. RQ-577 records that no requirement or task is added from the proposal
meanwhile, and none is. The one exception is PL-8's first half, the nominal Burrau pixel at f64 against R-33's
reference, which needs no tier and runs at M3 (REQ-VAL-250). PL-6's rows are § 4.2's 937.

*27 Oct 2026, R-631:* PL-6's reference values, the Hristov databases' digits for those 937 rows, are committed under
`HP27/data/` (`sofia_3body123/`, `sofia_3bodychor462/`, `sofia_3bodychor164/`, `sofia_3bodystab971/`), cited in
`HP27/SOURCES.md`; PL-6 reads them there, not from a fetch. `HP27/fetch.sh` stays the provenance check. PL-6 still has
no requirement while RQ-577 is open.

### 5.14 Out of scope

- **V-5**, the book's statistical statements: no set-up stated; each is a pointer to a primary paper.
- **V-10**, the post-Newtonian Pythagorean problem: Principia is Newtonian.
- **BZ-1**, Boekholt & Portegies Zwart 2015, f64 against a converged code: the two warm sets stay out of scope, being 3D.
  The cold sets start from rest, so planar (the owner), and are **reopened** as a proposal, with no requirement: the
  paired f32, f64 and converged experiment, `../incoming/2026-10-27_validation_plan/PLAN.md` § 3.2, BZ-1C (R-661). The
  paper has no f32 run: that f32 statistics agree is a hypothesis Principia will measure, and no paper establishes it. Its
  "unbiased" result holds only after an energy cut, so it is not what "never discard a copy" applies; its Fig. 14, cut
  beside uncut, is compatible. REQ-VAL-199 measures parting time, a different quantity. *Was (R-661):* "Boekholt & Portegies Zwart 2015, ensemble
  statistics surviving trajectory error: 3D Plummer initial conditions. Its method is what REQ-VAL-199 and "never discard a copy" already apply."
- **D-6**, the prototype's other numbers, as a list: RQ-587 (§ 5.11).

Most of the scattering literature is 3D; only planar subsets apply. Results with no published data and no figure
detail stay qualitative until the paper is in hand.

### 5.15 Counts

- **Requirements added:** 131, REQ-VAL-201 to REQ-VAL-331: 75 targets, one definition (REQ-VAL-221) and 55 calibrations.
- **By milestone (targets + definition, then calibrations):** M2 2 + 2; M3 58 + 47; M4 8 + 4; M6 2 + 0; M7 1 + 1; M8 5 + 1.
- **Tasks added:** 34 (M2 1, M3 24, M4 4, M6 1, M7 1, M8 3), § 7.
- **Waiting on an RQ:** 18 targets (§ 5.12, § 5.5, § 5.3, § 5.10).
- **No requirement:** PL-1 to PL-7, PL-8's second half, PL-9 to PL-12, B-3 and GG-2 (RQ-577, § 5.13); V-5, V-10 and BZ-1 (out of scope, BZ-1's cold sets reopened as a proposal, R-661); D-6 (RQ-587). FF-H and W-1 are ruled requirements (REQ-VAL-042, -043).
- *30 Oct 2026 (R-649, R-650):* by milestone now, M9 holds every requirement from REQ-VAL-201 to REQ-VAL-332 but PL-8's
  first half (REQ-VAL-250 and REQ-VAL-251, still M3), with REQ-VAL-042 and REQ-VAL-127, and five R-650 adds
  (REQ-VAL-333 to REQ-VAL-335, REQ-VAL-340, REQ-VAL-341); SP-2 adds two in M3 (REQ-VAL-336, REQ-VAL-337). The 34 tasks above and
  TASK-M4-26 are 35 tasks of M9, which adds three (§ 12.4).
- **PLAN.md's 112 rows** are all here: 25 ruled (§ 5.1), the rest a target above, a waiting row (§ 5.13), or out of scope (§ 5.14). OC-5's count and the tier-D numbers PLAN.md folded into ruled rows are separate targets here, so that the ruled requirements stay unchanged.

---

## 6. The first batch

PLAN.md § 5 chose these from the M3 list because each is exact or nearly so, needs no data the repo lacks, and has an
arm that can fail (REQ-VAL-007). R-629 ported its § 6 drafts as M3 requirements, their tolerances as calibrations. They
replace TARGETS.md's "Five to do first"; P-1/P-2 and L-3 stay in M3's list and follow the exact ones.

*30 Oct 2026 (R-650):* the first batch is scored first in M9, not M3; the *Task* column gives the new ids.

| PLAN.md draft | Target | Requirement | Calibration | Task |
|---|---|---|---|---|
| DRAFT-VAL-01 | OC-1 | REQ-VAL-201 | REQ-VAL-202 | TASK-M9-02 |
| DRAFT-VAL-02 | LL-1/LL-2 | REQ-VAL-203 | REQ-VAL-204 | TASK-M9-06 |
| DRAFT-VAL-03 | H-3 | REQ-VAL-205 | REQ-VAL-206 | TASK-M9-08 |
| DRAFT-VAL-04 | H-1/H-2 | REQ-VAL-207 | REQ-VAL-208 | TASK-M9-08 |
| DRAFT-VAL-05 | AA-6 | REQ-VAL-209 | REQ-VAL-210 | TASK-M9-10 |
| DRAFT-VAL-06 | Q-2's Theory row, Q-4 | REQ-VAL-211 | REQ-VAL-212 | TASK-M9-11 |
| DRAFT-VAL-07 | BG-4 | REQ-VAL-213 | none (§ 5.4: the bar is the published difference) | TASK-M9-12 |

---

## 7. Sequencing by need

R-594 item 5, as PLAN.md § 4 applies it: tier A, tier C and tier D's integrator entries at M3; tier B when the outcome
classifier lands, which is M3's escape detector and its re-validation (TASK-M3-12, TASK-M3-33, TASK-M3-34); a target
that needs a chart that does not exist waits for that chart. Nothing below asks a task to start early. Every task is
reviewed by code, qa and physics.

*30 Oct 2026 (R-650):* every task below is an M9 task now, under the new id § 12.4 gives it; the milestone headings and
the ids below are R-594's port, kept as its record, and their order by need orders M9's scoring. PL-8's first half
moves from TASK-M3-56 to TASK-M3-27 (§ 5.6).

**Before engine work** (rules 1 and 2): each task that commits a published table transcribes it twice first. The
fetches PLAN.md lists are the human's (RQ-584, RQ-585, RQ-586, RQ-598). *27 Oct 2026:* R-630 closes RQ-584 to RQ-586;
R-631 commits the fetched data (HP27, FF27); RQ-586's three papers are paywalled and wait for the owner's copies.

**M2.**

- **TASK-M2-51**, The orbit atlas through encode and decode: E and L per row (OC-2), and every atlas IC in the 8D latent chart (AT-D6). Targets: OC-2, AT-D6. Depends on TASK-M2-20, TASK-M2-26.

**M3.**

- **TASK-M3-42**, Atlas closure at order (OC-1, the first batch's DRAFT-VAL-01). Targets: OC-1. Depends on TASK-M3-24, TASK-M3-08, TASK-M2-51.
- **TASK-M3-43**, Published against refined: each atlas source's published initial conditions measured (OC-3). Targets: OC-3. Depends on TASK-M3-42.
- **TASK-M3-44**, The syzygy count against the atlas's `syz` on every row (OC-5), and its counting convention. Targets: OC-5, OC-5's counting convention. Depends on TASK-M3-16, TASK-M3-42.
- **TASK-M3-45**, The atlas's per-family coverage report (OC-6). Targets: OC-6. Depends on TASK-M3-42, TASK-M3-43, TASK-M3-44.
- **TASK-M3-46**, Li & Liao's free-fall orbits: closure, rest at T/2 and the alias pairs (LL-1, LL-2, the first batch's DRAFT-VAL-02), and the website-only rows (LL-5). Targets: LL-1, LL-2, LL-5. Depends on TASK-M3-42, TASK-M2-13.
- **TASK-M3-47**, Li & Liao's known orbits against Standish 1970 and Yasko & Orlov 2015 (LL-3). Targets: LL-3. Depends on TASK-M3-46. Waits on the owner's copies of Standish 1970 and Yasko & Orlov 2015 (paywalled, R-630 item 9; was "Waits on RQ-586.").
- **TASK-M3-48**, Heggie 1974's regularisation targets: the step ratio against Aarseth–Zare (H-1, H-2, DRAFT-VAL-04) and C/3 toward the Lagrange triple collision (H-3, DRAFT-VAL-03). Targets: H-3, H-1, H-2. Depends on TASK-M3-24, TASK-M3-08.
- **TASK-M3-49**, Tanikawa & Mikkola 2021's triple-collision points on the free-fall plane (TM-1). Targets: TM-1. Depends on TASK-M3-48. Waits on the owner's copy of Tanikawa & Mikkola 2021 (paywalled, R-630 item 9; was "Waits on RQ-586.").
- **TASK-M3-50**, Agekyan & Anosova II: decay times from exact rest (AA-6, the first batch's DRAFT-VAL-05). Targets: AA-6. Depends on TASK-M3-34, TASK-M2-20.
- **TASK-M3-51**, The no-integration controls: Monaghan's planar eccentricity row and the random-triangle Z (Q-2's Theory row, Q-4; DRAFT-VAL-06). Targets: Q-2's Theory row, Q-4. Depends on TASK-M3-12, TASK-M3-24.
- **TASK-M3-52**, The butterflies' two published periods, decided by closure (BG-4, the first batch's DRAFT-VAL-07). Targets: BG-4. Depends on TASK-M3-24, TASK-M3-08.
- **TASK-M3-53**, The Belgrade orbits close: the PRL's 15, the gallery's 206 sequence orbits and 57 BHH satellites, Hudomal's 38 and his E = −0.5 periods (SD-1, BG-1, BG-2, HU-3, HU-5). Targets: SD-1, BG-1, BG-2, HU-3, HU-5. Depends on TASK-M3-24, TASK-M3-08.
- **TASK-M3-54**, Period against word length: Li & Liao's period per letter (LL-4) and Hudomal's seven sequence fits (HU-4). Targets: LL-4, HU-4. Depends on TASK-M3-30, TASK-M3-46, TASK-M3-53.
- **TASK-M3-55**, The prototype's integrator and classifier numbers, reproduced or explained (tier D: D-1, D-2, D-4, D-5). Targets: D-1, D-2, D-4, D-5. Depends on TASK-M3-28, TASK-M3-32, TASK-M3-34, TASK-M3-01.
- **TASK-M3-56**, Burrau and the Pythagorean family: Chitan et al.'s sixteen triangles, Boekholt et al.'s lifetime and τ_L, Mikkola's bounce orbit and the f64 run against R-33's reference (CH-1, B-1, B-2, V-6, PL-8's first half). Targets: CH-1, B-1, B-2, V-6, PL-8's first half. Depends on TASK-M3-27, TASK-M3-25, TASK-M3-14, TASK-M3-34, TASK-M2-13.
- **TASK-M3-57**, Aarseth et al. 1994a: the Burrau neighbourhood on six rings per body (P-1 to P-6). Targets: P-1, P-2, P-3, P-4, P-5, P-6. Depends on TASK-M3-27, TASK-M3-34.
- **TASK-M3-58**, Aarseth et al. 1994b: triple approaches and parameter distributions over region D, circle R and the Pythagorean escapers (Q-1 to Q-3). Targets: Q-1, Q-2, Q-3. Depends on TASK-M3-51, TASK-M3-57.
- **TASK-M3-59**, Agekyan & Anosova's near-rest runs and Anosova 1969's unequal masses (AA-2 to AA-5, AA-7, AA-8, AN-1). Targets: AA-2, AA-3, AA-4, AA-5, AA-7, AA-8, AN-1. Depends on TASK-M3-50, TASK-M2-13.
- **TASK-M3-60**, Lehto et al. 2008 at a small grid: local homogeneity, reversibility and the step-halving controls (L-3, L-4, L-5). Targets: L-3, L-4, L-5. Depends on TASK-M3-34, TASK-M3-08.
- **TASK-M3-61**, Region-D statistics: the lifetime tail and lifetime against Lyapunov time (V-9, MT-1). Targets: V-9, MT-1. Depends on TASK-M3-34, TASK-M3-14.
- **TASK-M3-62**, Converged-code references: Boekholt et al. 2020's lifetime grid and Breen et al. 2020's Brutus runs (GG-1, BR-1). Targets: GG-1, BR-1. Depends on TASK-M3-25, TASK-M3-34. Waits on RQ-592, RQ-598.
- **TASK-M3-63**, Binary-plus-single targets: Trani et al.'s planar row, Manwadkar et al.'s escape probabilities, Stone & Leigh's closed forms (TR-1, MK-1, SL-1). Targets: TR-1, MK-1, SL-1. Depends on TASK-M3-34. Waits on RQ-593, RQ-596.
- **TASK-M3-64**, Parischewsky et al.'s equilateral-triangle experiment around Lagrange (PA-1). Targets: PA-1. Depends on TASK-M3-34. Waits on RQ-594, RQ-596.
- **TASK-M3-65**, Hierarchical stability limits, prograde and retrograde (V-11). Targets: V-11. Depends on TASK-M3-34. Waits on RQ-598.

**M4.**

- **TASK-M4-22**, Lehto's lifetime map at full resolution, and the shield's mirror symmetry (L-1, V-8 (a)). Targets: L-1, V-2/V-8 (a). Depends on TASK-M4-17, TASK-M3-60.
- **TASK-M4-23**, Hudomal's return-proximity map over the Šuvakov plane and the butterfly III zoom (HU-1, HU-2). Targets: HU-1, HU-2. Depends on TASK-M4-17, TASK-M3-53, TASK-M3-11. Waits on RQ-583 (RQ-582 closed by R-630; was "Waits on RQ-582, RQ-583.").
- **TASK-M4-24**, Symbol-sequence maps: Tanikawa & Mikkola's free-fall and non-zero-L maps, and Valtonen et al.'s L/M/A/H partition (TS-1, TS-2, V-1/V-3/V-7). Targets: TS-1, TS-2, V-1/V-3/V-7. Depends on TASK-M4-17, TASK-M3-16. Waits on RQ-595.
- **TASK-M4-25**, Li, Li & Liao's mass-plane contours for one periodic family (LLL-1). Targets: LLL-1. Depends on TASK-M4-17, TASK-M2-13. Waits on RQ-597.
- **TASK-M4-26** (added by R-636, 28 Oct 2026), The raised-cap run: the atlas rows OC-1, OC-3 and OC-6 recorded as "not evaluated within cap", evaluated at a calibrated raised horizon (REQ-VAL-332). Depends on TASK-M3-42, TASK-M3-43, TASK-M3-45.

**M6.**

- **TASK-M6-52**, Deep zooms of the shield: Lehto's 3.125×10⁻⁶ zoom and Valtonen et al.'s three windows (L-2, V-8 (b)). Targets: L-2, V-8 (b). Depends on TASK-M6-08, TASK-M4-22.

**M7.**

- **TASK-M7-71**, The prototype's Chazy-classifier numbers against Fewbody, reproduced or explained (D-3). Targets: D-3. Depends on TASK-M7-35, TASK-M7-36.

**M8.**

- **TASK-M8-97**, Stability as structure: atlas orbits off the Šuvakov plane against their published stability (OC-4, AT-D5). Targets: OC-4. Depends on TASK-M8-74, TASK-M6-05, TASK-M3-45.
- **TASK-M8-98**, The atlas's free-fall orbits marked on the closure field, one plane per mass set (AT-D2). Targets: AT-D2. Depends on TASK-M8-74, TASK-M4-15, TASK-M3-46.
- **TASK-M8-99**, The Šuvakov plane rendered, at equal masses and at the eight values of m₃, and OC-4 there (AT-D1, AT-D3, OC-4 on the Šuvakov plane). Targets: OC-4 on the Šuvakov plane, AT-D1, AT-D3. Depends on TASK-M8-97. Waits on RQ-583 (RQ-582 closed by R-630; was "Waits on RQ-582, RQ-583.").

*28 Oct 2026 (R-636):* RQ-583 and RQ-592 are closed. TASK-M4-23 and TASK-M8-99 now wait on RQ-608 (no Šuvakov-plane
chart is built yet), and TASK-M3-62 on RQ-598 alone (GG-1's systems past 200 are recorded as "not evaluated within
cap").
*28 Oct 2026 (R-637, R-640, R-642):* RQ-598 is closed, and TASK-M3-62's and TASK-M3-65's waits on it are lifted: the
authors' own data scores directly and qualifying reproductions provisionally (§ 5.10's note); V-11 proceeds from Part I.
R-638's gate stands: no Brutus run is scaled, or goes below ε = 10⁻⁶, before its measured cost per point is reported.
TASK-M4-23 and TASK-M8-99 still wait on RQ-608, now with RQ-618 (R-639).

---

## 8. Rules

### 8.1 Carried over from the 25 Oct instruction

Rules 1–8 of the 25 Oct instruction, "the ones that worked for Anosova", verbatim:

1. **The source is in the repo.** Each paper goes in `docs/incoming/<date>_<topic>/` with a README giving its citation
   and sha256. A value cited from memory or a secondary source is not a reference value.
2. **Transcribe twice.** Every table is transcribed twice, independently, and the two copies are diffed before the
   commit. Each row records its page.
3. **Tolerances are calibrations (R-71).** Each is proposed from a measurement, with its evidence, and batched for the
   gate.
4. **Never tune to match.** A disagreement is queued in `REVIEW_QUEUE.md` with both numbers, as RQ-570 was. It is not
   absorbed into a tolerance, a fit or an exception list without a ruling.
5. **Old integrators were weaker than ours.** Where we disagree with a pre-1990 result, the default suspicion is
   theirs, but it must be shown, not assumed. Agekyan & Anosova lost 21 of their 100 runs to energy error, and they
   say so themselves.
6. **Remap before comparing.** Coordinate conventions differ between papers. Agekyan–Anosova put two bodies at
   (∓½, 0). Lehto et al. use (0, 0) and (1, 0). Tanikawa uses the mirror labelling D11. Each map is converted into one of
   our charts, and the conversion is a tested function, not a figure overlay.
7. **State the escape criterion.** Published lifetimes depend on the criterion used (50a or 100a plus hyperbolic;
   r > 10, receding, E > 0; Fewbody's tidal tolerance). Outcome identity is robust once an escape is real; a lifetime is
   not. Each comparison records the paper's criterion and ours.
8. **A chaotic result is compared as a distribution, never pixel for pixel.** The exception is a result computed with
   a converged code (Brutus, clean numerical simulation). Those are trajectories, and can be compared per point.

*30 Oct 2026 (R-649, R-650):* rule 3's "batched for the gate" reads, for an external target, "batched for M9's
report"; rule 4 stands, and R-649 item 4 widens it (§ 12.3). Rule 5's suspicion is checked like-for-like first
(R-649 item 5).

### 8.2 Learnt since

PLAN.md § 8's rules 9–18, transcribed:

9. **Record the escape criterion as data, with its parameters and its clock.** Every comparison stores the paper's
   criterion with its numbers, the time it reports, and ours. They differ even between companion papers (5R_grav in
   Aarseth et al. 1994a, 10d in 1994b; Boekholt et al. 2021's conditions held for 100 time units, from the centre of
   mass; Boekholt et al. 2020's lifetime to the last close approach; Lehto's ratio 100 or 2000). The crossing time is a
   unit of the criterion too: 1994a's T_cr = 3.84 against 1994b's τ ≈ 1.255 for the Burrau masses.
10. **A paper's own numbers can be internally inconsistent: record, don't fix.** Agekyan & Anosova I print a mean of
    61.6 over 59 decay times whose mean is 64.63; paper II's "12 conditional decays" are 14 asterisks, and its "100
    systems" are 97 values; the gallery's two butterfly periods; Li & Liao's tables not correct to the last printed
    digit; RQ-570's energy column. Each stays as printed, beside our number, and the disagreement is queued, not
    resolved by choosing.
11. **Remap units and labels, not only coordinates.** τ per row (Agekyan & Anosova eq. 7), G = 6.67×10⁻⁸ against 1,
    E = −½ rescaling (Hudomal Tables 16–22), the book's ×2 scale, two labellings in one thesis, the atlas's masses as
    given and not normalised. A remap is a tested function with a round trip.
12. **Declare the word equivalence before comparing words.** The gallery and the PRL write the same conjugacy classes
    with different representatives, and moth III's published word is not freely reduced (RQ-576). The atlas states its
    own canonical rule for syzygy sequences, a precedent, not a drop-in, for RQ-576.
13. **A compilation is an index, never a source.** A reference value traces to the paper. A site's own orbits, and
    orbits a paper does not contain (LL-5), test closure only and are never cited as published results. The hunt's
    orbits stay out until the operator publishes them.
14. **A tolerance is set by the input's precision times the orbit's instability, never by a catalogued closure.**
15. **Check where the digits came from.** Digits past the data are arithmetic: Simó's "30-digit" rows carry about 16
    digits of data, and Li & Liao's 100-digit rows are Hristov's. Count the digits of the source, not of the file.
16. **A test that cannot fail is decoration.** Every target carries an arm that would fire (REQ-VAL-007): a negative
    control for an exact test, a shuffled or perturbed control for a distribution.
17. **Commit third-party data only under stated terms.** The atlas and the gallery state theirs; Hristov's databases,
    Simó and Li & Liao do not (RQ-579), so they are fetched and hash-checked, not committed; the Valtonen book is not
    committed. *27 Oct 2026, R-631 (closing RQ-579 with option (a)):* Hristov et al.'s databases, Simó's archive and
    Li & Liao's tables are committed, with full citations and no permission asked first, as published numerical
    results credited to their authors and never claimed as ours (HP27, FF27, each with a `SOURCES.md`); `fetch.sh`
    stays the provenance check. Other third-party data still needs stated terms or a ruling.
18. **Respect the horizon cap; queue, don't raise it.** R-610: `T_horizon ∈ [0.4, 200]`, default 50. A target past 200
    is compared only over the systems that end before 200 (RQ-592 asks whether a closure test may run past it).
    *28 Oct 2026, R-636 (closing RQ-592):* a target past the cap is recorded as "not evaluated within cap", never as
    a failure; a separate raised-cap run, at a calibrated cap, is TASK-M4-26 (REQ-VAL-332).

### 8.3 How the tasks apply them

- **Every published table committed as a fixture is transcribed twice, independently, and the two copies are diffed
  before the commit**, each row recording its page; both copies and the diff are kept beside the fixture (R-594 item
  4). A table a record has already transcribed twice and diffed (the three Agekyan & Anosova CSVs, P27's README § 6) is
  copied byte for byte with its sha256, and the record's diff is cited. A table read only from a text layer is a
  second reading, not a second transcription.
- **Never tune to match.** A disagreement with a published number goes to `REVIEW_QUEUE.md` with both numbers, as
  RQ-570 did, and the fixture is not edited. Where an old paper and the engine disagree and it is not clear whose error
  it is, that entry is for the human (R-594 item 6).
- **A tolerance is a calibration**: proposed with its evidence by the task that measures it, checked by a reviewer,
  confirmed by the human at the milestone gate and recorded in `decisions.md` (R-71). The form each calibration
  requirement gives is a proposal of what sets the floor, never a number chosen to match.
- **`atlas-params.json` never enters the repo.** No task reads, copies or commits it; each task that touches the atlas
  runs `scripts/check_no_atlas_params.sh`.
- **The atlas's credit line** (CC BY 4.0, `https://www.threebodyorbits.com`, and `OC26/CITATIONS.md`'s per-family
  credits) is carried by every fixture, export and figure built from it.
- **The gate** is one command, `cargo xtask gate replication --target <id>`, with `--propose` for a calibration's
  proposal and `--render` for a qualitative target's figure beside the paper's.
- *30 Oct 2026 (R-649, R-650):* for an external target "the gate" above is the scoring command, not a gate: it reports
  the outcome (§ 12.3) and blocks nothing, and its tolerance is confirmed by the human with M9's report, not at a
  milestone's gate. A target's control still fires or the test is broken (rule 16).

---

## 9. Coverage of initial-condition space, and the gaps

Which part of initial-condition space each target exercises (PLAN.md § 7).

| Region of IC space | Targets | Charts |
|---|---|---|
| Region D / free fall, equal masses at rest | AA-1 to AA-8; L-1 to L-5; GG-1; TS-1; TM-1; MT-1; Q-1, Q-2; V-2/V-8, V-7, V-9; LL-1 to LL-3 at (1, 1, 1); REQ-VAL-042, -044, -076; BR-1; PA-1 and H-3 at the Lagrange point | the Anosova physical-frame entry; ShapeSphere at rest; Latent `z_q = 0` |
| Free fall, unequal masses | LL-1 to LL-5 (nine mass sets); AN-1; CH-1; AT-D2's nine planes | MassSimplex; the Anosova entry at a mass point; the Burrau family |
| The Burrau neighbourhood | SP-1 (REQ-VAL-034); P-1 to P-6; Q-3; B-1, B-2; H-1; CH-1; V-6; PL-8's first half; REQ-VAL-016, -174; D-4, D-5 | BodyPlane; Burrau {ν, K} |
| The Šuvakov velocity plane (equal masses, L = 0, collinear start) | SD-1; HU-1 to HU-5; BG-1, BG-4; W-1; OC-1 (1,759 rows); AT-D1; OC-4 | new: a Šuvakov-plane chart (RQ-583) |
| The same plane, m₃ free | OC-1 (1,350 rows, 8 values of m₃); AT-D3; OC-4 | the same, at fixed m₃ |
| Non-zero angular momentum | BG-2 (57 BHH satellites); OC-1 (437 rows); TS-2; V-11 | InvariantLE / LK; Latent |
| Binary + single | TR-1; MK-1; SL-1; V-11 | new: BinaryPlusSingle (RQ-593) |
| Near triple collision | TM-1; H-3; PA-1; REQ-VAL-047, -020 | ShapeSphere at rest; the Anosova entry |
| Whole-engine properties | REQ-VAL-028, -170, -045, -030, -172, -173, -025, -040, -017, -070, -199, -183 | — |

**Gaps: where nothing published exists, in these records.** Stated narrowly: "not found in the records in this repo",
and the literature review's caveat applies, "any strong gap claim should be treated as a research hypothesis rather
than an established fact."

1. No outcome, lifetime or chaos map over the Šuvakov velocity plane; the one published map there is Hudomal's
   return-proximity map (HU-1), a search tool. 3,109 catalogued orbits sit on that plane and its m₃ stack.
2. No pixel-resolution map over the mass simplex, except Li, Li & Liao's continuation of one family (LLL-1).
3. Non-zero angular momentum: first-symbol maps only (TS-2); no lifetime or outcome map with L ≠ 0.
4. The Burrau neighbourhood has no 2D map: six circles × 100 azimuths (P-1), and otherwise the single point.
5. The Pythagorean family, one trajectory per member (CH-1).
6. Periodic orbits in their initial-condition context: no catalogue compares linear stability with the surrounding
   structure. OC-4 would be the first test of "stable ⇒ island, unstable ⇒ boundary".
7. Planar binary + single: one-dimensional scans only (TR-1).
8. Per-pixel reliability, simultaneous diagnostics and the per-pixel encounter word.

Region D at rest, by contrast, is well mapped, which is why most replication is possible there and why novelty is not
claimed there.

---

## 10. Open questions

PLAN.md § 9's twelve questions, and R-594 item 6's, each with where it stands. R-594 item 6 asked for these queued,
batched, not decided.

| Question | Where it stands |
|---|---|
| PLAN.md § 9 Q1: adopt PLAN.md as the single plan | yes, R-629 |
| Q2: port § 6's drafts now, as M3 requirements | yes, R-629 (§ 6) |
| Q3: the Šuvakov plane's labelling | RQ-582 |
| Q4: build a Šuvakov-plane chart | RQ-583 |
| Q5: RQ-576's word equivalence | RQ-576 |
| Q6: the high-precision databases' terms | RQ-579 |
| Q7: fetch Hristov et al. 2024's free-fall database | RQ-584 |
| Q8: Li & Liao's website pages | RQ-585 |
| Q9: fetch Tanikawa & Mikkola 2021, Standish 1970, Yasko & Orlov 2015 | RQ-586 |
| Q10: the precision ladder, before or after M3 | RQ-577 |
| Q11: tier D, enumerate the prototype's numbers | RQ-587 |
| Q12: queue the corrections to ruled documents | RQ-588, RQ-589, RQ-590; applied by R-632 |
| R-594 item 6: which targets gate a milestone and which are reported only | RQ-591 |
| R-594 item 6: targets past the horizon cap | RQ-592 |
| R-594 item 6: the new charts and diagnostics targets need | RQ-593 to RQ-597 (and RQ-583) |
| Data and papers not in the repo | RQ-598 (and RQ-584 to RQ-586) |
| R-594 item 6: an old paper and our engine disagree, and whose error is unclear | none yet: nothing has run. Each task queues its own, with both numbers (§ 8.3) |

*28 Oct 2026:* R-636 rules Q4 (RQ-583: the targets stay closures, no chart is built yet; the five targets that need
one are RQ-608's) and R-594 item 6's horizon cap (RQ-592: "not evaluated within cap", never a failure; the raised-cap
run is TASK-M4-26).
*28 Oct 2026:* R-637 rules R-594 item 6's gate question (RQ-591: option (b), recorded as ruled; this port moves no
requirement) and the data and papers (RQ-598: option (a), the targets' scoring ruled by R-638 and R-640 to R-642).
R-639 answers RQ-608; its five targets are RQ-618's.
*30 Oct 2026:* R-649 answers R-594 item 6's gate question for every external target: none gates a milestone; each is
a measurement (RQ-591's option (b) gives way to it, R-637's forward line). R-650 puts their scoring in M9.

---

## 11. Corrections the plan records

PLAN.md § 10 records what it changes in the older records, without rewriting them. Transcribed as the record; nothing
here edits a record or a document in force.

| Record | Was | Now |
|---|---|---|
| R25 instruction: tier tables A–D, "Sequencing", "Papers to fetch" | the working plan | superseded by PLAN.md § 3 and § 4 (R-629); a dated note in the record |
| R25 instruction: "What cannot be replicated" | "RQ-555 keeps the horizon at most 200" | R-610 (closing RQ-555): `T ∈ [0.4, 200]`, default 50 |
| R25 instruction: tier A "Region D Table 1" | "under way in TASK-M2-20" | merged, PR #294; RQ-570 open |
| R25 instruction: the "Rules" header | "RQ-566 to RQ-573" as Anosova's | Anosova's are RQ-566, -570, -571 and -573; RQ-572 was the lock-validation question (R-593) |
| R25 instruction: tier D, first bullet | "figure-eight convergence order 2.08 with the landing clamp, 1.06 without" | those are Aarseth–Zare's; under Heggie × RK4 the prototype measured 2.40 and 1.03 (D-1) |
| R25 instruction: tier B "Lifetime tails" | its own row | merged into V-9 |
| P27 TARGETS.md: "Five to do first" | the first batch | § 6 |
| P27 README § 5 | the extra 77 unresolved | resolved, § 4.1 |
| PL26 § 5 test 6 | "1,260 rows traced to their source paper" | 937 Hristov rows (§ 4.2) |
| ATP § 3 | "about 72 mass values" for the m₃-free stack | 8 values (§ 4.3) |
| ATP § 3, D2; LR § 13 | 311 free-fall orbits on "one plane" | nine planes (§ 4.3); LR not edited (RQ-590) |
| `principia_dd_validation_orbits.md` §1.1 | "Li & Liao 2018 (234 unequal-mass), Hristov–Hristova–Dmitrašinović–Tanikawa 2023 (12,409 equal-mass)" | queued, not edited: RQ-588 |
| REQ-VAL-042 | "Hristov et al. 2023" | queued, not edited: RQ-589 |

R-632 (27 Oct 2026) applies the last three rows' corrections, closing RQ-588 to RQ-590:
`principia_dd_validation_orbits.md` §1.1 and REQ-VAL-042's statement are corrected, each with its *Was*, and the
literature review, a record, gets a dated note in its README. It also closes RQ-570 (column 8 recorded, never
asserted) and RQ-575, whose statistic § 5.1's ensemble ratio row now names (*was:* "median parting ratio (RQ-575)"). The
rows above stay as PLAN.md records them.

## 12. Targets are measurements: tiers, outcomes and M9 (R-649, R-650)

*Added 30 Oct 2026, porting R-649 and R-650 (the owner's rulings of 29 Oct 2026, with his two clarifications and his
addendum on early peeks). Where an earlier section says a target gates, is confirmed at a milestone's gate or runs in
an earlier milestone, this section governs.*

### 12.1 What stays continuous, and what moves to M9

- **Continuous, blocking a merge as before** (R-650): the internal correctness tests, which check our arithmetic or
  our own reference: Hamiltonian FD against analytic derivatives (REQ-VAL-170, REQ-VAL-171); drift-falls-with-step
  (REQ-VAL-030, REQ-VAL-031, REQ-VAL-139; the convergence reference REQ-VAL-025 and REQ-VAL-131; reversibility
  REQ-VAL-032, REQ-VAL-044, REQ-VAL-134, REQ-VAL-190, REQ-VAL-191); xcheck against reference (REQ-VAL-028, REQ-VAL-036,
  REQ-VAL-119, REQ-VAL-146, and PL-8's first half, REQ-VAL-250 and REQ-VAL-251); gauge invariance (REQ-VAL-172, its f32
  tolerance REQ-VAL-338); the Burrau constants (REQ-VAL-174; the import arithmetic REQ-VAL-016, REQ-VAL-019,
  REQ-VAL-118, REQ-VAL-120); W-1 (REQ-VAL-043, R-125's verification of the convention we transcribe); the Chazy
  classifier's Fewbody reference (REQ-VAL-184 to REQ-VAL-187); the closure field (REQ-VAL-076); and every GPU-vs-CPU
  agreement check, within its stated f32 tolerance, measured, never assumed (R-649's clarification 2; REQ-VAL-173's
  tolerance is REQ-VAL-339). A value a contract or design doc adopts as its gate is internal and blocking, pitfall
  regressions included, even where it came from the prototype: REQ-INT-049, REQ-INT-051, REQ-INT-052, REQ-VAL-045,
  REQ-VAL-051, REQ-VAL-052 and REQ-EVT-013 stay in M3 and keep blocking.
- **The anchor smoke tests, continuous and blocking,** which catch gross breakage only and never steer design: the
  figure-eight closure (REQ-VAL-041, REQ-VAL-133); SP-1 run to its own horizon, `t_max` between 60 and 65 as a per-test
  parameter, the default horizon 50 unchanged (REQ-VAL-034, REQ-VAL-132); SP-2 (REQ-VAL-336, REQ-VAL-337; § 5.6).
- **External, measured in M9:** every target that scores against a published or other outside number, or against a
  prototype number held as a tier-D "reproduce or explain" target (§ 5.11, REQ-VAL-299 to REQ-VAL-306). *Was:* "every
  target that scores against a published, prototype or other outside number" (corrected 30 Oct 2026, the physics
  review of PR #342, finding F3). They are:
  REQ-VAL-201 to REQ-VAL-249 and REQ-VAL-252 to REQ-VAL-332, FF-H (REQ-VAL-042), SP-1's feature set (REQ-VAL-127), the
  figure-eight's published shape-sphere curve (REQ-VAL-333, its tolerance REQ-VAL-334) and SP-1's structural comparison
  with Szebehely & Peters (REQ-VAL-335). M9 also holds its report (REQ-VAL-340) and the GPU-path tightening's
  definition (REQ-VAL-341). A target's control (rule 16) is part of its test and still has to fire. *1 Nov 2026 (code
  review 5730758034 of PR #375, R1; applied per R-369):* those ranges say what moved to M9, and they hold each target's
  calibration too, which takes its target's tier (§ 12.2) and goes to the human with the report (§ 12.3); the report's
  rows are § 12.2's table.
- **Ingest and transcription** (a `docs/incoming/` data PR) go on whenever idle; scoring waits for M9, or is a peek
  (§ 12.5).

### 12.2 Tiers

Each external target binds a tier (R-649 item 2): (a) the GPU explorer, f32 and interactive, expected to agree
statistically or qualitatively only, with `error_ratio` flagging what it can't resolve; (b) the CPU IC inspector, f64
or higher, where exact targets bind. The owner confirmed the reading on 30 Oct 2026: "statistical targets are scored
on tier (a), exact targets on tier (b)" (R-649). So an exact target, a measurement, a count, a report or a definition
is scored on (b); a statistical target is scored on (a), statistically only; a qualitative target over a map or an
ensemble is scored on (a) too, (a) being the tier that agrees "statistically or qualitatively"; a mixed target is
scored on (b) for its exact part and on (a) for its statistical or qualitative part (*applied per R-369*, R-649). A
single chaotic trajectory set beside a figure (AA-4, V-6, SP-1's structural comparison) is (b). *31 Oct 2026, R-661:* "expected to agree statistically", "agrees" and "agreeing statistically", here and in this section's other lines, state what tier (a) is scored on, not a known result. That its f32 statistics agree is a hypothesis Principia will measure (BZ-1C), and no paper establishes it; what a score on (a) says meanwhile is RQ-649. Five map targets
whose verify named a Result at f64 (AT-D2, OC-4 off and on the Šuvakov plane, AT-D1, AT-D3) are scored on (a), with
the f64 Result run beside it and recorded as information; an explained divergence between the two is a precision-tier finding
(R-653, RQ-622 (c)). B-2 (REQ-VAL-246), one start, the Burrau pixel, fitted over many runs and so outside the
single-trajectory clause, is scored on (a) by the GPU explorer's FTLE, with the CPU FTLE beside it on the same terms
(R-653). *Was (until R-653):* "are (a) by the rule; how their f64 Result runs square with it is RQ-622, and their
scoring waits on it. B-2 (REQ-VAL-246) is statistical, so (a) by the rule, but its statement names the CPU FTLE and it
is one start, the Burrau pixel; which instrument scores it is RQ-622 too, and its scoring waits on it." A GPU miss on a (b) target, or on
a mixed target's (b) part, is expected, not a defect. A calibration takes its target's tier. *Was (the port's first
reading, corrected 30 Oct 2026 by the owner's confirmation):* "a statistical or qualitative one binds (b), with (a)
expected to agree statistically or qualitatively; one with an exact part and a statistical part binds (b) for the
first, and (a) is expected to agree with the second statistically"; every (a) and mixed row of the table below read
"(b); (a) statistically or qualitatively" or "(b); its statistical part also (a)".

| Target | Requirement | Compare (§ 5) | Tier |
|---|---|---|---|
| OC-1 | REQ-VAL-201 | exact | (b) |
| OC-2 | REQ-VAL-214 | exact | (b) |
| AT-D6 | REQ-VAL-216 | exact | (b) |
| OC-3 | REQ-VAL-218 | measurement | (b) |
| OC-5 | REQ-VAL-220 | exact (a count) | (b) |
| OC-5's counting convention | REQ-VAL-221 | a definition | (b) |
| OC-6 | REQ-VAL-222 | a report | (b) |
| AT-D2 | REQ-VAL-308 | qualitative + a calibrated threshold | (a); its f64 Result beside it, information (R-653) |
| OC-4 (off the Šuvakov plane) | REQ-VAL-309 | statistical | (a); its f64 Result beside it, information (R-653) |
| LL-1, LL-2 | REQ-VAL-203 | exact | (b) |
| LL-3 | REQ-VAL-223 | exact | (b) |
| LL-4 | REQ-VAL-225 | exact per orbit; the fit statistical | (b) per orbit; (a) for the fit |
| LL-5 | REQ-VAL-227 | exact (closure only) | (b) |
| BG-4 | REQ-VAL-213 | exact | (b) |
| SD-1 | REQ-VAL-228 | exact | (b) |
| BG-1 | REQ-VAL-230 | exact | (b) |
| BG-2 | REQ-VAL-232 | exact | (b) |
| HU-3 | REQ-VAL-234 | exact | (b) |
| HU-4 | REQ-VAL-236 | exact per orbit; the fits statistical | (b) per orbit; (a) for the fits |
| HU-5 | REQ-VAL-238 | exact | (b) |
| H-3 | REQ-VAL-205 | exact (scaling) | (b) |
| H-1, H-2 | REQ-VAL-207 | ratio, recorded both ways | (b) |
| TM-1 | REQ-VAL-240 | exact (scaling) | (b) |
| CH-1 | REQ-VAL-242 | statistical | (a) |
| B-1 | REQ-VAL-244 | exact to the figure's resolution | (b) |
| B-2 | REQ-VAL-246 | statistical (loose) | (a); its CPU FTLE beside it, information (R-653) |
| V-6 | REQ-VAL-248 | exact once shot; qualitative against the figure (one trajectory) | (b) |
| Q-2's Theory row, Q-4 | REQ-VAL-211 | (i) exact; (ii) statistical; no integration | (b) for (i); (a) for (ii) |
| P-1 | REQ-VAL-252 | qualitative + statistical | (a) |
| P-2 | REQ-VAL-254 | statistical | (a) |
| P-3 | REQ-VAL-256 | qualitative | (a) |
| P-4 | REQ-VAL-257 | statistical | (a) |
| P-5 | REQ-VAL-259 | statistical | (a) |
| P-6 | REQ-VAL-261 | qualitative | (a) |
| Q-1 | REQ-VAL-262 | statistical | (a) |
| Q-2 | REQ-VAL-264 | statistical | (a) |
| Q-3 | REQ-VAL-266 | statistical | (a) |
| AA-6 | REQ-VAL-209 | exact on the 13 short-lived rows; statistical otherwise | (b) on the 13 short-lived rows; (a) otherwise |
| AA-2 | REQ-VAL-268 | exact on the short-lived rows; statistical otherwise | (b) on the short-lived rows; (a) otherwise |
| AA-3 | REQ-VAL-270 | statistical | (a) |
| AA-4 | REQ-VAL-272 | qualitative (one trajectory) | (b) |
| AA-5 | REQ-VAL-273 | statistical | (a) |
| AA-7 | REQ-VAL-275 | statistical | (a) |
| AA-8 | REQ-VAL-277 | statistical | (a) |
| AN-1 | REQ-VAL-279 | statistical | (a) |
| L-3 | REQ-VAL-281 | statistical | (a) |
| L-4 | REQ-VAL-283 | statistical / qualitative | (a) |
| L-5 | REQ-VAL-285 | qualitative | (a) |
| V-9 | REQ-VAL-286 | statistical | (a) |
| MT-1 | REQ-VAL-288 | statistical | (a) |
| L-1 | REQ-VAL-290 | qualitative | (a) |
| V-2 / V-8 (a) | REQ-VAL-291 | exact (mirror symmetry to rounding) | (b) |
| L-2 | REQ-VAL-293 | qualitative | (a) |
| V-8 (b) | REQ-VAL-294 | qualitative | (a) |
| GG-1 | REQ-VAL-295 | per point where reliable; KS otherwise | (b) per point; (a) for the KS part |
| BR-1 | REQ-VAL-297 | exact (a converged code) | (b) |
| D-1 | REQ-VAL-299 | measurement: reproduce or explain | (b) |
| D-2 | REQ-VAL-301 | measurement: reproduce or explain | (b) |
| D-4 | REQ-VAL-303 | measurement: reproduce or explain (on the CPU) | (b) |
| D-5 | REQ-VAL-305 | exact (a count of 0) | (b) |
| D-3 | REQ-VAL-306 | exact on the fixtures; statistical for the medians | (b) on the fixtures; (a) for the medians |
| OC-4 on the Šuvakov plane | REQ-VAL-311 | statistical | (a); its f64 Result beside it, information (R-653) |
| AT-D1 | REQ-VAL-312 | qualitative | (a); its f64 Result beside it, information (R-653) |
| AT-D3 | REQ-VAL-313 | qualitative | (a); its f64 Result beside it, information (R-653) |
| HU-1 | REQ-VAL-314 | qualitative; exact at Table 23's minima | (b) at Table 23's minima; (a) for the map |
| HU-2 | REQ-VAL-316 | qualitative + one exact point | (b) at the exact point; (a) for the map |
| TR-1 | REQ-VAL-318 | qualitative | (a) |
| MK-1 | REQ-VAL-319 | statistical | (a) |
| SL-1 | REQ-VAL-321 | statistical | (a) |
| PA-1 | REQ-VAL-323 | qualitative; the split statistical | (a) |
| TS-1 | REQ-VAL-325 | qualitative | (a) |
| TS-2 | REQ-VAL-326 | qualitative | (a) |
| V-1 / V-3 / V-7 | REQ-VAL-327 | qualitative; the share statistical | (a) |
| LLL-1 | REQ-VAL-329 | qualitative | (a) |
| V-11 | REQ-VAL-330 | statistical | (a) |
| FF-H | REQ-VAL-042 | exact (closure) | (b) |
| SP-1's feature set | REQ-VAL-127 | a definition | (b) |
| the figure-eight's published shape-sphere curve | REQ-VAL-333 | exact | (b) |
| SP-1, structurally against Szebehely & Peters | REQ-VAL-335 | structural (one trajectory, chaotic) | (b) |

### 12.3 Outcomes, the like-for-like check and the forbidden moves

- **Outcomes** (R-649 item 3): **PASS**; **EXPLAINED DIVERGENCE**, for a named mechanism (regularisation, escape
  criterion, horizon, stopping rule, units or normalisation, precision tier, sampling, the source's own accuracy), with
  a measurement showing the mechanism, not a story; **UNEXPLAINED**, open, labelled and non-blocking. An external target
  has no "fail"; the internal tests of § 12.1 keep pass and fail.
- **Like-for-like first** (R-649 item 5). Before any divergence is recorded, the comparison is checked for: the frame;
  the units and their normalisation; the masses; the definition of the event (escape, collision, period, stopping
  rule, the horizon); the source's stated accuracy; and the tier the number binds. A divergence from our being a
  different instrument is a finding, recorded as such.
- **Forbidden in pursuit of a pass** (R-649 item 4): changing a default, the physics, a criterion or the architecture;
  tuning a parameter per target; loosening a tolerance; special-casing an input. Running a target at its own natural
  horizon, as a per-test parameter, is not a default change (clarification 1: SP-1). A design change a target suggests
  needs its own ruling, justified on Principia's goals (interactive exploration first; accuracy against speed a
  deliberate trade), never on the target alone, with a REVIEW_QUEUE entry before it.
- **A bug a target or a peek exposes** is fixed only when it is justified independently of the target (a wrong
  equation, a misencoded outcome, a broken invariant), with its own test and ruling (R-650's addendum).
- **Calibrations of M9 targets** are confirmed by the human with the M9 report (REQ-VAL-340), not at a gate (rule 3).

### 12.4 The tasks in M9

R-650 moves the 35 replication tasks to M9 and renumbers them in their old order (`plan/WORKFLOW.md` § "Task files":
a task's id names its milestone); each keeps its depends-on edges, renamed, and its waits on a ruling or data, which
hold its scoring, never a milestone. Three are added: TASK-M9-36 (FF-H, the figure-eight's published curve and SP-1's
structural comparison), TASK-M9-37 (the report) and TASK-M9-38 (the GPU-path tightening defined).

| Was | Now |
|---|---|
| TASK-M2-51 | TASK-M9-01 |
| TASK-M3-42 | TASK-M9-02 |
| TASK-M3-43 | TASK-M9-03 |
| TASK-M3-44 | TASK-M9-04 |
| TASK-M3-45 | TASK-M9-05 |
| TASK-M3-46 | TASK-M9-06 |
| TASK-M3-47 | TASK-M9-07 |
| TASK-M3-48 | TASK-M9-08 |
| TASK-M3-49 | TASK-M9-09 |
| TASK-M3-50 | TASK-M9-10 |
| TASK-M3-51 | TASK-M9-11 |
| TASK-M3-52 | TASK-M9-12 |
| TASK-M3-53 | TASK-M9-13 |
| TASK-M3-54 | TASK-M9-14 |
| TASK-M3-55 | TASK-M9-15 |
| TASK-M3-56 | TASK-M9-16 |
| TASK-M3-57 | TASK-M9-17 |
| TASK-M3-58 | TASK-M9-18 |
| TASK-M3-59 | TASK-M9-19 |
| TASK-M3-60 | TASK-M9-20 |
| TASK-M3-61 | TASK-M9-21 |
| TASK-M3-62 | TASK-M9-22 |
| TASK-M3-63 | TASK-M9-23 |
| TASK-M3-64 | TASK-M9-24 |
| TASK-M3-65 | TASK-M9-25 |
| TASK-M4-22 | TASK-M9-26 |
| TASK-M4-23 | TASK-M9-27 |
| TASK-M4-24 | TASK-M9-28 |
| TASK-M4-25 | TASK-M9-29 |
| TASK-M4-26 | TASK-M9-30 |
| TASK-M6-52 | TASK-M9-31 |
| TASK-M7-71 | TASK-M9-32 |
| TASK-M8-97 | TASK-M9-33 |
| TASK-M8-98 | TASK-M9-34 |
| TASK-M8-99 | TASK-M9-35 |

Each row of § 5.2 to § 5.12 and of § 6 had, before 30 Oct 2026, the milestone and task below (*was*); its milestone is
now M9 and its task the new id.

| Target | Requirement | Milestone, was | Task, was | Task, now |
|---|---|---|---|---|
| OC-1 (DRAFT-VAL-01) | REQ-VAL-201 | M3 | TASK-M3-42 | TASK-M9-02 |
| OC-2 | REQ-VAL-214 | M2 | TASK-M2-51 | TASK-M9-01 |
| AT-D6 | REQ-VAL-216 | M2 | TASK-M2-51 | TASK-M9-01 |
| OC-3 | REQ-VAL-218 | M3 | TASK-M3-43 | TASK-M9-03 |
| OC-5 (folded into W-1 by PLAN.md; the syzygy count only) | REQ-VAL-220 | M3 | TASK-M3-44 | TASK-M9-04 |
| OC-6 | REQ-VAL-222 | M3 | TASK-M3-45 | TASK-M9-05 |
| AT-D2 (folded into REQ-VAL-076's row by PLAN.md) | REQ-VAL-308 | M8 | TASK-M8-98 | TASK-M9-34 |
| OC-4 = AT-D4 (+ AT-D5), off the Šuvakov plane | REQ-VAL-309 | M8 | TASK-M8-97 | TASK-M9-33 |
| LL-1, LL-2 (DRAFT-VAL-02) | REQ-VAL-203 | M3 | TASK-M3-46 | TASK-M9-06 |
| LL-3 | REQ-VAL-223 | M3 | TASK-M3-47 | TASK-M9-07 |
| LL-4 | REQ-VAL-225 | M3 | TASK-M3-54 | TASK-M9-14 |
| LL-5 | REQ-VAL-227 | M3 | TASK-M3-46 | TASK-M9-06 |
| BG-4 (DRAFT-VAL-07) | REQ-VAL-213 | M3 | TASK-M3-52 | TASK-M9-12 |
| SD-1 (+ the gallery's cross-check 3) | REQ-VAL-228 | M3 | TASK-M3-53 | TASK-M9-13 |
| BG-1 | REQ-VAL-230 | M3 | TASK-M3-53 | TASK-M9-13 |
| BG-2 | REQ-VAL-232 | M3 | TASK-M3-53 | TASK-M9-13 |
| HU-3 (+ the gallery's cross-check 4) | REQ-VAL-234 | M3 | TASK-M3-53 | TASK-M9-13 |
| HU-4 (+ Dmitrašinović et al. 2018; the 25 Oct tier A's 'period against word length') | REQ-VAL-236 | M3 | TASK-M3-54 | TASK-M9-14 |
| HU-5 | REQ-VAL-238 | M3 | TASK-M3-53 | TASK-M9-13 |
| H-3 (DRAFT-VAL-03) | REQ-VAL-205 | M3 | TASK-M3-48 | TASK-M9-08 |
| H-1, H-2 (DRAFT-VAL-04) | REQ-VAL-207 | M3 | TASK-M3-48 | TASK-M9-08 |
| TM-1 | REQ-VAL-240 | M3 | TASK-M3-49 | TASK-M9-09 |
| CH-1 | REQ-VAL-242 | M3 | TASK-M3-56 | TASK-M9-16 |
| B-1 (+ the 25 Oct tier A's Burrau lifetime) | REQ-VAL-244 | M3 | TASK-M3-56 | TASK-M9-16 |
| B-2 (+ the 25 Oct tier A's Burrau Lyapunov time) | REQ-VAL-246 | M3 | TASK-M3-56 | TASK-M9-16 |
| V-6 | REQ-VAL-248 | M3 | TASK-M3-56 | TASK-M9-16 |
| Q-2's Theory row, Q-4 (DRAFT-VAL-06) | REQ-VAL-211 | M3 | TASK-M3-51 | TASK-M9-11 |
| P-1 | REQ-VAL-252 | M3 | TASK-M3-57 | TASK-M9-17 |
| P-2 | REQ-VAL-254 | M3 | TASK-M3-57 | TASK-M9-17 |
| P-3 | REQ-VAL-256 | M3 | TASK-M3-57 | TASK-M9-17 |
| P-4 | REQ-VAL-257 | M3 | TASK-M3-57 | TASK-M9-17 |
| P-5 | REQ-VAL-259 | M3 | TASK-M3-57 | TASK-M9-17 |
| P-6 | REQ-VAL-261 | M3 | TASK-M3-57 | TASK-M9-17 |
| Q-1 | REQ-VAL-262 | M3 | TASK-M3-58 | TASK-M9-18 |
| Q-2 | REQ-VAL-264 | M3 | TASK-M3-58 | TASK-M9-18 |
| Q-3 | REQ-VAL-266 | M3 | TASK-M3-58 | TASK-M9-18 |
| AA-6 (DRAFT-VAL-05) | REQ-VAL-209 | M3 | TASK-M3-50 | TASK-M9-10 |
| AA-2 | REQ-VAL-268 | M3 | TASK-M3-59 | TASK-M9-19 |
| AA-3 | REQ-VAL-270 | M3 | TASK-M3-59 | TASK-M9-19 |
| AA-4 | REQ-VAL-272 | M3 | TASK-M3-59 | TASK-M9-19 |
| AA-5 | REQ-VAL-273 | M3 | TASK-M3-59 | TASK-M9-19 |
| AA-7 | REQ-VAL-275 | M3 | TASK-M3-59 | TASK-M9-19 |
| AA-8 | REQ-VAL-277 | M3 | TASK-M3-59 | TASK-M9-19 |
| AN-1 | REQ-VAL-279 | M3 | TASK-M3-59 | TASK-M9-19 |
| L-3 (+ the 25 Oct tier B's Maasai shield statistics) | REQ-VAL-281 | M3 | TASK-M3-60 | TASK-M9-20 |
| L-4 | REQ-VAL-283 | M3 | TASK-M3-60 | TASK-M9-20 |
| L-5 | REQ-VAL-285 | M3 | TASK-M3-60 | TASK-M9-20 |
| V-9 (+ Orlov et al. 2010; the 25 Oct tier B's lifetime tails) | REQ-VAL-286 | M3 | TASK-M3-61 | TASK-M9-21 |
| MT-1 | REQ-VAL-288 | M3 | TASK-M3-61 | TASK-M9-21 |
| L-1 (+ the 25 Oct tier B's Maasai shield) | REQ-VAL-290 | M4 | TASK-M4-22 | TASK-M9-26 |
| V-2 / V-8 (a) | REQ-VAL-291 | M4 | TASK-M4-22 | TASK-M9-26 |
| L-2 | REQ-VAL-293 | M6 | TASK-M6-52 | TASK-M9-31 |
| V-8 (b) | REQ-VAL-294 | M6 | TASK-M6-52 | TASK-M9-31 |
| GG-1 (the 25 Oct tier B's converged lifetime grid) | REQ-VAL-295 | M3 | TASK-M3-62 | TASK-M9-22 |
| BR-1 | REQ-VAL-297 | M3 | TASK-M3-62 | TASK-M9-22 |
| D-1 (folded into REQ-VAL-041's row by PLAN.md) | REQ-VAL-299 | M3 | TASK-M3-55 | TASK-M9-15 |
| D-2 (folded into REQ-VAL-052's row by PLAN.md) | REQ-VAL-301 | M3 | TASK-M3-55 | TASK-M9-15 |
| D-4 | REQ-VAL-303 | M3 | TASK-M3-55 | TASK-M9-15 |
| D-5 | REQ-VAL-305 | M3 | TASK-M3-55 | TASK-M9-15 |
| D-3 (folded into REQ-VAL-184's row by PLAN.md) | REQ-VAL-306 | M7 | TASK-M7-71 | TASK-M9-32 |
| OC-4 on the Šuvakov plane | REQ-VAL-311 | M8 | TASK-M8-99 | TASK-M9-35 |
| AT-D1 | REQ-VAL-312 | M8 | TASK-M8-99 | TASK-M9-35 |
| AT-D3 | REQ-VAL-313 | M8 | TASK-M8-99 | TASK-M9-35 |
| HU-1 | REQ-VAL-314 | M4 | TASK-M4-23 | TASK-M9-27 |
| HU-2 | REQ-VAL-316 | M4 | TASK-M4-23 | TASK-M9-27 |
| TR-1 | REQ-VAL-318 | M3 | TASK-M3-63 | TASK-M9-23 |
| MK-1 | REQ-VAL-319 | M3 | TASK-M3-63 | TASK-M9-23 |
| SL-1 | REQ-VAL-321 | M3 | TASK-M3-63 | TASK-M9-23 |
| PA-1 | REQ-VAL-323 | M3 | TASK-M3-64 | TASK-M9-24 |
| TS-1 (the 25 Oct tier B's symbol-sequence maps) | REQ-VAL-325 | M4 | TASK-M4-24 | TASK-M9-28 |
| TS-2 | REQ-VAL-326 | M4 | TASK-M4-24 | TASK-M9-28 |
| V-1 / V-3 / V-7 | REQ-VAL-327 | M4 | TASK-M4-24 | TASK-M9-28 |
| LLL-1 | REQ-VAL-329 | M4 | TASK-M4-25 | TASK-M9-29 |
| V-11 | REQ-VAL-330 | M3 | TASK-M3-65 | TASK-M9-25 |

### 12.5 Peeks

When a target's data is already transcribed and cheap to run, it may be scored early, as information only (R-650's
addendum), when idle and after the backlog, in the owner's priority: long horizons (GG-1, to t = 10⁴); free-fall
triple collisions (Li 2021, Tanikawa 1995 and 2021); unequal-mass and non-zero-L targets (Li 2025, Titov, Dandekar
2026). Each peek is a row of `docs/verification/peek_log.md`: date, target, tier, data source, outcome (§ 12.3), the
like-for-like checks, the mechanism measurement where explained, and the commit. A peek is never a gate, and nothing
is tuned to one.

### 12.6 Verification deepening (post-build)

Making the GPU path agree better while staying interactive is M9's work, after the core build (R-649 item 6). It is
defined first, by REQ-VAL-341 (a definition, R-72; TASK-M9-38), before any of it is built: the corpus does not yet
say what "agree better" is measured against or what stays "interactive". Each change it proposes is a design change
with its own ruling (§ 12.3).
