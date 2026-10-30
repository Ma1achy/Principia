# Verifying Principia and replicating published results: one plan, 27 Oct 2026

**A proposal for one ruling. Not authority** (R-1, R-159): this folder is a record, like every `docs/incoming/` folder.
Nothing here changes a requirement, a ruling or a task until the owner rules on it; the draft requirements in §6 are
drafts, and their ids are placeholders.

**What it supersedes, without deleting anything.** If adopted, this plan replaces, as the working plan:

- the tier tables A–D, the "Sequencing" section and the "Papers to fetch" list of
  `../2026-10-25_replication/INSTRUCTION_replication_suite_2026-10-25.md` (the 25 Oct instruction). That file stays as
  the record of what was approved on 25 Oct, with a dated pointer here;
- the "Five to do first" of `../2026-10-27_papers/TARGETS.md`, folded into §5's M3 batch. TARGETS.md stays the
  authoritative transcription of its 49 targets: this plan cites it and copies no number out of it that it does not
  need.

It does not supersede the 25 Oct rules 1–8 (carried over verbatim in §8), the orbit-catalogue instruction's data terms,
or any record's transcriptions.

**Why it exists.** The owner approved the 25 Oct plan in these words (the 25 Oct instruction, its header):

> "but yes this plan A-D seems good."

On 27 Oct he asked, relayed by the coordinating agent:

> "is this all part of a plan to verify Principia and also replicate other peoples results?"

> "has it been updated based on all the new stuff i have sent etc? i.e. these papers, the lit review and the three body
> orbits website catalog etc. etc. along with other tables and orbits etc and other results"

The answer to the first is yes, and to the second, not until now: the 25 Oct plan predates the 26 and 27 Oct records
(§0). This file is the update, written so it can be ruled on once.

---

## 0. Sources, and the abbreviations used below

Every row of §3 cites one of these, with a section, table or line.

| Abbr. | Path (relative to this folder unless marked) | What it is |
|---|---|---|
| `R25` | `../2026-10-25_replication/` | The 25 Oct instruction (`INSTRUCTION…`), `catalogue.md` (about 35 sources) and `papers/` |
| `P27` | `../2026-10-27_papers/` | `TARGETS.md` (49 numbered targets), `README.md`, the three Agekyan & Anosova CSVs, `transcriptions/`, `valtonen2016/` |
| `AN24` | `../2026-10-24_anosova/` | The Agekyan & Anosova papers I and II and Li & Liao's arXiv:1805.07980 |
| `OC26` | `../2026-10-26_orbit_catalogues/` | The Three Body Orbits snapshot: `threebodyorbits_catalogue.json` (3,943 orbits, 41 families), `threebodyorbits_ics.csv`, `CITATIONS.md`, `citations.py`, the instruction (`INSTRUCTION…`) and Hudomal's 2015 thesis |
| `BG26` | `../2026-10-26_belgrade_gallery/` | The recovered Belgrade gallery: `belgrade_gallery.csv` (301 rows), `crosscheck.txt`, `archive/initial-conditions.pdf` (Hudomal 2019, 206 sequence orbits) |
| `HP27` | `../2026-10-27_hp_ics/` | High-precision ICs for the precision ladder's test 6: `README.md`, `agreement.txt`, `sources.tsv`, `fetch.sh` (values not committed, RQ-579) |
| `PL26` | `../2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md` | The precision-ladder proposal; its §5 has twelve tests. Queued as RQ-577; nothing in it is in force |
| `LR` | `../2026-10-25_lit_review/lit_review_2026-10.tex` | The literature review, fourth revision. §13 "Periodic-Orbit Catalogues" (line 663), §14 "Validation Targets" (line 714), §15 "Summary of Gaps" (line 732) |
| `ATP` | `/mnt/user-data/outputs/atlas_into_principia_plan.md` (**outside the repo**) | "Mapping the Three Body Orbits atlas into Principia: a plan", draft of 26 Oct 2026, deliverables D1–D7. Its §3 classification is re-measured in §2.3 below |
| `VO` | `docs/design/principia_dd_validation_orbits.md` (repo root) | The validation-orbits design doc: §2's gate table (R-257), §3's proposed suite |
| `GT` | `docs/notes/principia_validation_ground_truth_note.md` | Ground truth: four tiers, Path A and Path B |
| `REQ` | `plan/requirements.yaml` | The `REQ-VAL-*` requirements: REQ-VAL-001 to REQ-VAL-200, 199 in force and one retired |
| `FND` | `docs/reference/prin-rs/FINDINGS.md` | The prototype's findings, imported as a record (R-159); never normative |

`/mnt/user-data/outputs/precision_ladder_and_taylor.md` (outside the repo) was compared with `PL26`. They differ in one
place: the outside copy attributes arXiv:2001.01221 to "Wang, Nitadori & Makino 2020"; the committed proposal has the
correction to Antoñana, Chartier, Makazaga & Murua 2020 (`PL26` §2 and §7; `LR`'s README, third revision). The
committed one governs.

**What changed since the 25 Oct plan**, in one list:

- 27 Oct papers (`P27`): 49 targets transcribed from eight papers and one book, with the Agekyan & Anosova tables as
  CSVs (twice-transcribed, `P27/README.md` §6).
- The orbit atlas (`OC26`): 3,943 periodic orbits; 1,728 of the 3,916 with catalogued closure < 1e-9 have unequal
  masses and 413 non-zero angular momentum (`OC26/INSTRUCTION…` "What it covers"), the first such data in the suite.
- The Belgrade gallery (`BG26`): 206 sequence orbits with words, 57 BHH satellites at non-zero L, and the word
  equivalence problem (RQ-576).
- High-precision ICs (`HP27`): 937 atlas rows traced to Hristov et al.'s databases at 97–180 digits.
- The precision ladder (`PL26`, RQ-577) and the atlas plan (`ATP`).
- The literature review's fourth revision (`LR` §13–§15).
- On main since 25 Oct: TASK-M2-20 merged (PR #294, 27 Oct, commit `dd8d8ded`), closing REQ-VAL-019, -120 and -174 with
  RQ-570 still open; R-610 closed RQ-555 (horizon `T ∈ [0.4, 200]`, default 50).

---

## 1. Two layers, kept distinct

**Verification: the code computes what it claims.** The reference is a property that any correct implementation must
reproduce, whoever computed it first: a periodic orbit closes, an analytic solution stays analytic, the equations of
motion agree with a finite difference of the Hamiltonian, a symmetry leaves a quantity unchanged, f32 and f64 differ
only where the arithmetic says they may, and two implementations of one algorithm agree. Published periodic-orbit
initial conditions and their free-group words belong here: the paper supplies the initial condition and the period,
but the claim tested is ours. It covers the Python cross-check, the finite-difference test, gauge invariance, energy
and drift, the periodic-orbit closures, the precision ladder's per-tier tests and the f32/f64 honesty checks.

**Replication: independent published results from other codes.** The reference is a quantity another group measured
with another integrator, on a system that is chaotic, or non-periodic, or both: lifetimes, escaper identities, maps,
binned statistics, step counts and energy errors, stability labels. Agreement there is evidence about the physics, not
just about our arithmetic. It is compared as a distribution unless the other code was converged (rule 8).

The distinction matters because the two fail differently. A verification failure is a located bug. A replication
disagreement is a finding to queue (rule 4), and the default suspicion for an old integrator is theirs, but it must be
shown (rule 5).

**Tiers.** The 25 Oct tiers are kept: **A** exact (a deterministic value matched to a tolerance); **B** maps and
statistics (the chaotic sea); **C** numerical reliability; **D** the prototype prin-rs's own results. A row has one tier;
where a row also carries a prototype number, it is noted as "(D)".

**Status**, one per row, by the binding constraint:

- **data transcribed**: the reference values are in the repo in machine-readable form, transcribed twice or extracted by
  a script and cross-checked (rule 2);
- **data in repo**: the source holding the values is committed (a paper, a thesis, a record's table), but they are not
  yet in a machine-readable, twice-transcribed file, or the reference is a property with no external data;
- **blocked**: something outside the engine's milestones stops it: data not in the repo, a ruling or a queued question,
  or a chart or diagnostic that no task builds. The reason is given;
- **out of scope**: outside a planar Newtonian instrument, or a pointer with no target value.

An engine milestone not yet built is sequencing (§4), not a block. Each row also says whether it is already a
requirement ("ruled", with its `REQ-VAL` id and milestone) or only proposed.

---

## 2. Three findings made while consolidating

### 2.1 The atlas's 311 free-fall orbits against Li & Liao's 234: resolved

`P27/README.md` §5 recorded that the atlas attributes 311 free-fall orbits to Li & Liao 2019 (`OC26/CITATIONS.md`,
family `freefall`) while the paper reports 234, and left the extra 77 unresolved. Measured on the committed snapshot
(`OC26/threebodyorbits_catalogue.json`, family `freefall`, every row's `masses`, `code` and `aliases`):

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

- **The extra orbits are the authors' own, published on their website, not in the paper.** Li & Liao's page for the
  paper (`http://numericaltank.sjtu.edu.cn/three-body/free-fall-3b/free-fall-3b.htm`, fetched 27 Oct 2026, sha256
  `357ae374…f592f`) says: "We report 316 collisionless periodic free-fall three-body orbits with different mass ratios.
  Eexcept for three periodic orbits have benn found before, 313 collisionless periodic orbits are entirely new" (the
  page's spelling). Its word table (`…/free-fall-3b-free-group-word.htm`, fetched 27 Oct 2026, sha256
  `60352475…aa8a22`) lists 316 rows: the paper's six sets with the paper's counts, plus 82 on three further mass sets,
  (1, 0.1, 0.01) 48, (0.393, 0.288, 0.662) 19 and (1, 0.412, 0.267) 15. That page gives words only, no initial
  conditions. (The paper itself names the site's former address, `http://numericaltank.sjtu.edu.cn/free-fall-3b/free-fall-3b.htm`,
  arXiv v1 p. 3; that address answers 404.) Neither page is committed; §9 asks.
- **The atlas holds 311 = 316 − 5.** Five rows carry `aliases`, each folding one listed orbit into another:
  F2 into F1, F13 into F12 and F24 into F23 at (1, 1, 1); F16 into F11 at (1, 0.8, 0.6); F47 into F45 at
  (1, 0.1, 0.01). So the atlas carries **230** of the paper's 234 and **81** of the website's 82 extra.
  **311 − 234 = 77 = 81 − 4.**
- **The aliases are one orbit listed twice, and the paper's own numbers say so.** Computed here from the paper's
  printed (x, y, T) (`AN24/papers/li_liao_2018_arXiv_1805.07980.pdf`, Tables A1–A2), the scale-invariant period
  T·|E|^{3/2} of each pair agrees: F1 20.009755 against F2 20.009709 (relative 2.3e-6), F12 against F13 44.917786 to
  about 6e-11, F23 against F24 91.891703 to about 3e-11, and F11 against F16 at (1, 0.8, 0.6) 13.366255 to about 1e-10.
  This plan's reading, not checked against the atlas's own reason (its snapshot gives none): a brake orbit has zero
  velocity at t = 0 and again at T/2 (LL-2), so it passes through region D at rest twice, and Li & Liao's scan found
  both. That makes LL-2 sharper (§3, LL-2), and the paper's 234 rows hold at most 230 distinct orbits.
- **Attribution.** CITATIONS.md's family-level attribution of all 311 to `liliao2019` is right about the authors and
  the orbit family, but 81 of the rows are not in the cited paper. They are the authors' published results (their
  website), and for this plan they are closure tests only until their initial conditions are traced to a source the
  authors publish (the website's word page has none). §9 asks how to cite them.

### 2.2 The precision ladder's test 6 counts: corrected

`PL26` §5 test 6 says `atlas-params.json` carries high-precision fields "on 1,260 rows traced to their source paper".
Verified against `HP27/README.md` § "Agreement with `atlas-params.json`" and `HP27/agreement.txt`:

- 1,359 atlas rows carry high-precision fields. 1,260 is 1,359 less the 99 with an empty source, so it counted the
  344 Simó rows and the 76 rows from the atlas's own notes and the CAP repository, and left out 97 Hristov rows.
- **The rows that can test tiers above f64 are the 937 traced to Hristov et al.**: `hristov2022` 9 (150 digits),
  `hristov2023` 73 and `hristov2024nc` 167 (180 digits; 240 together), and `hristov2026` 586 + the 97 empty-source rows
  matched to `3bodystab971` + the 5 filed under `liliao2017`, which are Hristov's digits: **688 at 97–100 digits**.
  9 + 240 + 688 = 937. These numbers hold as the task stated them.
- **The 344 Simó rows carry about 16 significant digits of data** (Masson's 13–17-digit file scaled to unit masses at
  30 digits), so they test nothing above f64; the proposal's "the 30-digit ones cannot test `dd64`" understates it.
- **The reference values are the databases', not `atlas-params.json`'s**: for the 73 `hristov2023` rows the atlas
  rounds the third body's velocity to about 120 digits (`HP27/README.md` "What it means for test 6").
- The values are fetched and hash-checked by `HP27/fetch.sh`, not committed (RQ-579).

A dated correction note is added to `PL26` (§10).

### 2.3 The atlas plan's slice classification: two of its numbers do not hold

Re-measured on `OC26/threebodyorbits_ics.csv` (`pub_*` columns, masses as given):

| `ATP` §3 says | Measured | |
|---|---|---|
| 1,759 equal-mass orbits on Šuvakov's plane (`x = (−1, 1, 0)`, `v₁ = v₂`, `v₃ = −2v₁`) | 1,759 | holds |
| 1,350 on the same plane with `m₃` free, "about 72 mass values" | 1,350 (1,349 Li, Jing & Liao 2018 rows and the one `certified` row at m₃ = 1.001), on **8** values of m₃: 0.5 (565), 0.75 (401), 2 (237), 4 (85), 5 (35), 8 (17), 10 (9), 1.001 (1) | count holds; **"about 72" does not** |
| 311 free-fall orbits, "one plane, the free-fall shape triangle" | 311, on **nine** mass sets (§2.1) | count holds; **one plane does not**: at rest the shape plane is one plane per mass point, so nine |
| 437 non-zero angular momentum; 86 other | 437 with \|L\| > 1e-6; 86 | hold |

The 87% (3,420 of 3,943) holds as "three families of slices". `LR` §13 repeats "311 lie on the free-fall plane … one
plane" (lines 698–699); that sentence needs the same correction, which this plan records (§10) and does not make: the
lit review is the owner's text.

**One more consequence for D1 and D3, this plan's reading, for the physics reviewer.** Every Šuvakov-plane start is
collinear. In the chart reference's coordinates (`docs/design/principia_chart_reference.md` §0.2), a collinear
configuration lies on the configuration chart's boundary: on `β = 0` or `β = π` (the Euler edges, REQ-VAL-020), or at
the `α = 0` pole where `λ̃ = 0` and `β` is undefined. Which one depends on the labelling. Under `TARGETS.md`'s and
Hudomal Table 1's labelling (bodies at (−1, 0), (1, 0), (0, 0), so the middle body is the decoder's body 2), `λ = 0`
and the plane sits on the `α = 0` pole. Under Hudomal eq. (18)'s labelling (middle body second) it sits on `β = 0`,
which R-417 says the decoder reaches exactly. Only the momenta vary across the plane, so the chart is a mixed-axis
slice (chart reference §1.3) with the configuration frozen on that boundary. §9 asks which.

---

## 3. Every target, one table

**Columns.** *Id*: an existing id is kept (`REQ-VAL-…`, TARGETS's `P-1` … `V-11`); new rows get a new id, its prefix
naming the source. *L*: V verification, R replication. *T*: tier. *Source*: file and section (§0). *Asserted*: what
must hold. *Ref. data*: the file holding the reference values. *Chart*: an existing chart (chart reference §5.1's
list, `BodyPlane`, the Anosova physical-frame entry of the lowering contract), "Path A/B" (`GT`: a known IC encoded, or
injected physically, with no chart), or "new: <name>". *Needs*: the engine prerequisite (§4 names the tasks).
*Compare*: exact (to a tolerance, a calibration under R-71), distribution, or qualitative. *Status*: §1.

Merged rows list every source. Where one target arises from several records, it has one row.

### 3.1 Verification

| Id | L | T | Source | Asserted | Ref. data | Chart | Needs | Compare | Status |
|---|---|---|---|---|---|---|---|---|---|
| REQ-VAL-028 (+ -146, -119, -036) | V | A | `VO` §2 gate table; R-257, R-166; `R25` "Why" | BodyPlane reproduces the Python reference: bit-exact where the operation order is identical, otherwise within a calibrated tolerance; f64 agreement ~1e-10 on a small grid | `docs/reference/prin-rs/tools/xcheck/`, `docs/reference/prin-rs/reference/` (records, R-159) | BodyPlane | M3 integrator (TASK-M3-36) | exact (REQ-VAL-146) | data in repo · ruled, M3 |
| REQ-VAL-170 (+ -171) | V | A | `VO` §2 gate table, "equations of motion"; R-257 | Each occupant's analytic equations of motion agree with a finite difference of its Hamiltonian; a sign-flipped derivative fails | none (self-referential, with its negative control) | none | regularisation occupants (TASK-M3-08) | exact (REQ-VAL-171) | data in repo · ruled, M3 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| REQ-VAL-045 (+ -195) | V | A | `VO` §2; R-257, R-566, R-605 | Two-body radial collision passes `d_min < 1e-10` with `\|dE/E\| < 1e-12`, `E` the regularised energy; both drifts stored | none | Path B | regularisation (TASK-M3-08, TASK-M3-41) | exact | data in repo · ruled, M3 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| REQ-VAL-030 (+ -031, -139) | V | A | `REQ` (dd_integrator tests 1–2) | Energy error over a Kepler-embedded orbit scales as the occupant's order; symplectic bounded, RK4 secular | none | Path B | steppers (TASK-M3-26) | exact (fitted slope) | data in repo · ruled, M3 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| REQ-VAL-041 (+ -133; D-1) | V | A (D) | `VO` §0, §3; `R25` tier D, first bullet; `FND` §3.2 | The figure-eight (`T = 6.32591398`) closes after one period, at the occupant's fitted order. Prototype: 2.08 with the landing clamp and 1.06 without under Aarseth–Zare; **2.40 and 1.03 under Heggie**, the default occupant (R-560) | `VO` §3; `FND` §3.2 | Path A | M3 (TASK-M3-28) | exact + fitted order | data in repo · ruled, M3 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| REQ-VAL-047 (+ -130) | V | A | `VO` §1.3, §3; `GT` tier 2; `LR` §14 | Lagrange circular stays equilateral at every sampled t; the figure-eight traces its shape-sphere curve | analytic | Path B | M3 (TASK-M3-28) | exact | data in repo · ruled, M3 |
| REQ-VAL-020 | V | A | `VO` §1.3; R-417; `LR` §14 | The decoder puts Lagrange at `z = 0` and Euler at the β edges | analytic | Latent | M2 decode (TASK-M2-03) | exact | data in repo · ruled, M2 |
| REQ-VAL-016 (+ -118) | V | A | chart reference §5.2 | Burrau at ν = ½ reproduces (3, 4, 5) | the construction | Burrau {ν, K} | M2 (TASK-M2-10) | exact | data in repo · ruled, M2 |
| REQ-VAL-174 | V | A | `VO` §2; R-257 | The Burrau rest start reads M = 12, R = 2.2361, E = −12.8167 | `VO` §2 | Burrau import | M2 (TASK-M2-20, merged) | exact | data in repo · ruled, M2; closed by PR #294 |
| AA-1 = REQ-VAL-019 (+ -120) | V | A | `P27` AA-1; R-592; `VO` §1.6; `LR` §14 | Region D and the near-rest starts import: `M = η·10⁻⁴` on every row, the printed M on all but N = 16, 23, 26, 32, 34; E by eq. 4; column 8 recorded, never asserted | `fixtures/anosova/table1.csv` | Anosova physical-frame entry | M2 (TASK-M2-20, merged) | exact | data transcribed · ruled, M2; RQ-570 open |
| REQ-VAL-172 | V | A | `VO` §2; R-257 | ICs rescaled by α ∈ {0.25, 1, 4}, t by α^{3/2}: `spread_shape` identical to ~10 decimals | none | any (small grid) | ensemble spread (M5) | exact | data in repo · ruled, M5 |
| REQ-VAL-173 | V | C | `VO` §2; R-257 | `error_ratio` = 1.0000 at t = 13 on near-field | `fixtures/slices.toml` | BodyPlane (near-field) | M5 | exact | data in repo · ruled, M5 |
| REQ-VAL-025 (+ -131) | V | C | R-33, R-265; `PL26` §3.8 | The independent arbitrary-precision CPU reference converges: its output is unchanged between successive precision and tolerance steps | none | Path A/B | convergence reference (TASK-M3-25) | convergence | data in repo · ruled, M3 |
| REQ-VAL-040 (+ -051, -054, -123, -124) | V | C | R-29, R-95; pitfalls §2.4 | Escape precision and recall against independent ground truth; the tau gap re-measured | the legacy t = 30 set (to regenerate, R-166) | `config_stability`, `config_basin`, deep interior | outcome classifier (TASK-M3-12, -33, -34) | distribution | data in repo · ruled, M3 |
| REQ-VAL-044 (+ -134) | V | C | `VO` §3; R-162, R-500 | Free-fall brake orbits are time-symmetric by the reversibility measure ξ, on logH's TTL mode. ξ is still to be defined (REQ-VAL-134); Lehto et al.'s ξ (L-4) is a published candidate | brake-orbit ICs (LL-1, FF-H) | Path A, `z_q = 0` | reversible occupant (TASK-M3-08, -29) | exact | data in repo · ruled, M3; definition pending |
| REQ-VAL-017 | V | C | decoder test 12; R-547 | CPU-f64 and GPU-f32 decodes agree; branch tags bitwise | none | every chart | M2 (TASK-M2-48) | exact | data in repo · ruled, M2 |
| REQ-VAL-070 (+ -071, -074) | V | C | `docs/design/principia_honesty_precision.md` §1; R-119 | The f32 horizon measured on the GPU; the closure floor (~1e-7 f32, ~1e-16 f64) reported as saturation, not a lost orbit | none | any | flat GPU compute (M4) | measurement | data in repo · ruled, M4 |
| REQ-VAL-199 | V | C | honesty_precision; R-574, R-600 | Median `t_part(f32)/t_part(f64)` over all 256 chaotic starts against its prediction | none | — | M8 (TASK-M8-76) | distribution | blocked: RQ-575 · ruled, M8 |
| REQ-VAL-183 | V | C | Chazy drill-down; R-442 | Chazy class f32 against f64 disagreement measured and reported, not asserted | none | a gallery chart | Chazy classifier (M7, TASK-M7-39) | measurement | data in repo · ruled, M7 |
| D-2 = REQ-VAL-052 (+ -136) | V | D | `R25` tier D, second bullet; `FND` §2.6 | The default occupant's drift field is insensitive to doubled re-registration, no pale straight-edged wedges. Prototype: chord 4.832e-2 (Heggie) against 5.162e-1 (AZ), `config_stability`, 256² | `FND` §2.6 | `config_stability` | M3 (TASK-M3-32) | measurement | data in repo · ruled, M3 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| D-3 = REQ-VAL-184 (+ -185 to -187) | V | D | `R25` tier D, third bullet; `docs/design/principia_dd_chazy_final_motion.md` §4.1 | The classifier against Fewbody's termination test: no wrong commit on E < 0 at any playhead; median first correct label 51.6 / 40.8 against Fewbody's 69.6 / 69.6 (δ = 1e-5) | drill-down §4.1 table; its fixtures | free-fall, Burrau | Chazy classifier (M7, TASK-M7-35, -36) | exact (fixtures) + distribution | data in repo · ruled, M7 |
| D-4 | V | D | `R25` tier D, fourth bullet; honesty_precision §1 | f32 against f64 outcome labels on near-field, 32 × 32, t = 13. Prototype (AZ): 1022 of 1024 agree; 2 pixels with \|dE/E\| > 1; 152 of 1024 flip on the unconditioned branch. Re-measure under Heggie × RK4 | honesty_precision §1 | BodyPlane (near-field) | kernel at f32 on the CPU (TASK-M3-01) | measurement | data in repo · proposed · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| D-5 | V | D | `R25` tier D, fifth bullet; `FND` §4 | No near-field escape fires at t = 13 | `FND` §4 | BodyPlane (near-field) | outcome classifier (TASK-M3-12) | exact (a count of 0) | data in repo · proposed |
| D-6 | V | D | `R25` tier D, "hundreds of numbers"; `FND` | Every other prototype number reproduced, or the difference explained by a measurement | `FND` | various | various | measurement | blocked: not enumerated; promote one by name, never as a block (§9) |
| OC-1 (+ `HP27` Simó finding) | V | A | `OC26/INSTRUCTION…` "What to do with it" item 1; `HP27/README.md` "What it means for test 6"; `LR` §14 | One period from `ref_*` at f64 under the default occupant and under AZ returns at the occupant's order. Rows with refined closure < 1e-9, short periods first, small-`rmin` rows on purpose. Simó's 344 test nothing above f64 | `OC26/threebodyorbits_ics.csv` (3,916 rows with closure < 1e-9) | Path A (no chart needed) | M3 integrator, regularisation | exact: floor = f64 rounding × amplification, never the catalogued closure | data transcribed · proposed (DRAFT-VAL-01) · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| OC-2 | V | A | `OC26/INSTRUCTION…` item 2 | E and L recomputed through our decode and encode match each row to its printed precision | `OC26` CSV, `E_cat`, `L_cat` | Path A | M2 encode (TASK-M2-15) | exact | data transcribed · proposed |
| OC-3 (+ `HP27` Li & Liao finding) | V | A | `OC26/INSTRUCTION…` item 3; `HP27/README.md` (Li & Liao's 5 rows) | Our closure from `pub_*` against `closure_published`, row by row: a measurement of each source's published ICs. Li & Liao 2017's 10-digit values against Hristov's 100-digit ones on 5 orbits, where `HP27` found the published tables not correct to the last printed digit | `OC26` CSV; `HP27/agreement.txt` | Path A | M3 | measurement (a disagreement is information) | data transcribed · proposed |
| OC-6 | V | A | `OC26/INSTRUCTION…` item 6 | A per-family coverage report: rows tested, order achieved, failures; a family failing every row is investigated before the engine is blamed | `OC26` CSV | none | M3 | report | data transcribed · proposed |
| AT-D6 | V | A | `ATP` D6, §5 steps 2–3 | Every published atlas IC encoded into the 8D chart; scaled twins land on one point; yin-yang α/β pairs land on different points; each slice family's residual off its fitted plane printed; canonical coordinates exported as a CSV with credits | `OC26` CSV | Latent (8D) | M2 encode | exact | data transcribed · proposed (`ATP` is outside the repo) |
| REQ-VAL-042 (+ FF-H) | V | A | `VO` §1.1, §3, §5 item 3; `R25` tier A "Periodic free-fall orbits" (Hristov part); `R25/catalogue.md` §6.3; `LR` §14 | Two or three free-fall brake orbits return to rest in the same configuration, order reported. The source the 25 Oct plan named: Hristov et al. 2024's database, 24,582 ICs to 80 digits, 12,409 distinct with T* < 80 | not fetched: `db2.fmi.uni-sofia.bg/3bodyfree/` (`R25/catalogue.md` §6.3); the paper is in `R25/papers/` | Anosova entry; Latent `z_q = 0` | M3 (TASK-M3-29) | exact | blocked: the database is not fetched, and states no terms (RQ-579's class) · ruled, M3 |
| LL-1 (+ `R25` tier A, Li & Liao part) | V | A | `P27` LL-1; `R25` tier A; `OC26/CITATIONS.md` `freefall` | Each of the 234 ICs closes after T, collisionless | Tables A1–A5 of `AN24/papers/li_liao_2018_arXiv_1805.07980.pdf` (10 decimals; not yet transcribed); the atlas carries 230 of them (§2.1) and is not the reference (`P27` LL-1) | Anosova entry; MassSimplex point for the five unequal sets | M3; mass-simplex chart (TASK-M2-13) | exact (10-decimal ICs × amplification) | data in repo · proposed (DRAFT-VAL-02); rule 2 transcription first |
| LL-2 | V | A | `P27` LL-2; §2.1 here | Every velocity vanishes at T/2. New, from §2.1: the T/2 rest state, rescaled into region D, is the partner row of each alias pair (F1/F2, F12/F13, F23/F24 at (1, 1, 1); F11/F16 at (1, 0.8, 0.6)), with T·\|E\|^{3/2} equal | as LL-1 | as LL-1 | M3 | exact | data in repo · proposed (DRAFT-VAL-02) |
| LL-3 | V | A | `P27` LL-3 | F1 (0.6, 0.8, 1) is Standish's (1970) orbit; F1 and F2 (1, 1, 1) are Yasko & Orlov's Orbits 16 and 17; §2.1 finds F1 and F2 to be one orbit | as LL-1 | as LL-1 | M3 | exact | blocked: Standish 1970 and Yasko & Orlov 2015 are not in the repo (rule 1) |
| LL-4 | V | A | `P27` LL-4 | T̄* = (T/L_f)\|E\|^{3/2} ≈ 2.363 at equal masses; 1.663, 1.191, 0.776, 0.527 at m₃ = 0.8 … 0.2; fit 1.912 m₃ + 0.084 | Tables A6–A10 (words; not transcribed) | none | word reader | exact per orbit; the fit as a distribution | data in repo · proposed |
| LL-5 | V | A | §2.1 here; `OC26` `freefall` | The 81 atlas rows on the three website-only mass sets close after T (closure only, rule 13) | `OC26` CSV | MassSimplex points | M3; mass-simplex chart | exact | data transcribed · proposed; source page not committed (§9) |
| SD-1 (+ `BG26` cross-check 3) | V | A | `R25` tier A "Velocity-slice periodic orbits"; `R25/catalogue.md` §2.3; `BG26/README.md` item 3; `BG26/crosscheck.txt` §3 | The 15 ICs of Šuvakov & Dmitrašinović 2013's Table 1 close after T | the PRL (`R25/papers/`); `BG26/belgrade_gallery.csv` (agrees with the PRL to 4.1e-5) | Path A (the map part is HU-1) | M3 | exact | data transcribed · proposed |
| BG-1 | V | A | `BG26/README.md` items 1–2; `crosscheck.txt` §1–2 | The 206 sequence orbits close after T at their printed precision (6 decimals in p₁, p₂); each label agrees with its word | `BG26/belgrade_gallery.csv` (identical to Hudomal 2019 in every printed digit) | Path A | M3 | exact | data transcribed · proposed |
| BG-2 | V | A | `BG26/README.md` "What the pages hold"; `OC26/CITATIONS.md` `bhhsat` | The 57 BHH satellites at non-zero L close after T, from relative coordinates; the atlas's 15 `bhhsat` rows (`bhh2026`, the site's own note) as closure only | `BG26` CSV (extracted by script; not cross-checked against a paper); `OC26` CSV | Path A; InvariantLE/LK for a chart | M3 | exact | data in repo · proposed |
| BG-4 | V | A | `BG26/README.md` "One internal discrepancy"; RQ-576 "Recorded, not resolved" | Butterfly I and II have two published periods each (6.235641 / 6.234671; 7.003907 / 7.003707). The return-proximity minimum from the gallery's (p₁, p₂) says which; the result is recorded, the CSV is not edited (rule 10) | `BG26` CSV; `OC26` CSV | Path A | M3 | exact | data transcribed · proposed (DRAFT-VAL-07) |
| HU-3 (+ `BG26` cross-check 4) | V | A | `P27` HU-3; `BG26/README.md` item 4 | The 38 thesis orbits close at six-digit precision; Table 23's return-proximity column is the published bar. Five thesis labels were renamed later | thesis Table 23 (`OC26/hudomal_2015_msc_belgrade.pdf`, not transcribed) | Path A | M3 | exact | data in repo · proposed |
| HU-4 (+ Dmitrašinović et al. 2018; `R25` tier A "Period against word length") | V | A | `P27` HU-4; `R25` tier A | T\|E\|^{3/2} against word length within each sequence: the seven fits of `P27` HU-4 | `P27/TARGETS.md` HU-4 table; `R25/papers/dmitrasinovic_etal_2018…` | none | word reader | exact per orbit; fits as distributions | data in repo · proposed |
| HU-5 | V | A | `P27` HU-5 | Periods at E = −0.5 and their ratios to T_M8 = 26.1281 (for example yarn / moth I = 3.00114 against a word ratio of 3) | `P27/TARGETS.md` HU-5 | none | M3 | exact | data in repo · proposed |
| W-1 = REQ-VAL-043 (+ OC-5, AT-D7) | V | A | R-125; `VO` §1.4; RQ-576; `OC26/INSTRUCTION…` item 5; `ATP` D7 and §1 ("Their word convention"); `BG26` items 2–3 | The computed free-group word equals the published class under a declared equivalence; the syzygy count equals the atlas's `syz` on every row | `BG26` CSV (206 words; the 57 satellites' with exponents); PRL Table 1; Dmitrašinović 2018; the thesis; `OC26` `syz` | none | word reader (TASK-M3-15, -16, -30) | exact under the declared equivalence | blocked: RQ-576 (which equivalence) · ruled, M3 |
| REQ-VAL-076 (+ -137; AT-D2) | V | B | `VO` §6; `ATP` D2 | The closure field on the Config α × β chart's `z_q = 0` plane is low at the known free-fall orbits. D2 marks the atlas's 311, on nine planes, not one (§2.3) | LL-1, LL-5, FF-H positions | Latent `z_q = 0`; MassSimplex points | flat GPU compute (M4); Result for D2 (TASK-M8-74) | qualitative + a calibrated threshold | data in repo · ruled, M4 (D2 proposed) |
| PL-1 | V | C | `PL26` §5 test 1 | The emulated-arithmetic canary: every error-free transform exact on the device; a reassociated build fails | none | none | precision tier | exact | blocked: RQ-577 |
| PL-2 | V | C | `PL26` §5 test 2 | `df32` and `qf32` add, multiply, divide and square root within their precision against `mp` | none | none | precision tier | exact | blocked: RQ-577 |
| PL-3 | V | C | `PL26` §5 test 3 | Taylor coefficients against `mp` finite differences and an independently written recurrence, with the anchor chain checked on the series | none | none | precision tier; Taylor stepper | exact | blocked: RQ-577 |
| PL-4 | V | C | `PL26` §5 test 4 | Taylor order n reads slope n on figure-eight closure | REQ-VAL-041's orbit | Path A | Taylor stepper | exact | blocked: RQ-577 |
| PL-5 | V | C | `PL26` §5 test 5 | Raising precision and order together converges on Burrau pixels | none | BodyPlane | precision tier | convergence | blocked: RQ-577 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| PL-6 (+ `HP27`) | V | C | `PL26` §5 test 6, corrected in §2.2 here; `HP27/README.md` | Closure after the period falls rung by rung above f64 on the 937 Hristov rows (688 at 97–100 digits, 9 at 150, 240 at 180); a tier fails if it does not improve on the one below | `HP27/sources.tsv` + `fetch.sh` (values hash-checked, not committed) | Path A | precision tier | exact per tier | blocked: RQ-577 (the tiers) and RQ-579 (data terms) |
| PL-7 | V | C | `PL26` §5 test 7 | Horizon per tier: where neighbouring rungs disagree on Burrau; fails only if a higher tier's horizon is shorter | none | BodyPlane | precision tier | measurement | blocked: RQ-577 · see DRAFT-VAL-08 (note, 27 Oct 2026: this per-tier horizon is the census's precision horizon at fixed occupant, its §3.2; note, 28 Oct 2026: the owner keeps PR #314's PL-7 edit to the census's §3.2, "it is correct (PL-7 is the precision horizon)", R-633; the census stays a record, its sha256 in RQ-599 unchanged) |
| PL-8 | V | C | `PL26` §5 test 8 | The nominal Burrau pixel at f64 against R-33's reference to t ≈ 60; then, at a tier whose horizon passes 60, a neighbourhood map whose nominal pixel reproduces SP-1 | SP-1 | BodyPlane | convergence reference (first half); precision tier (second) | exact; structural | data in repo for the first half (TASK-M3-25); blocked: RQ-577 for the second |
| PL-9 | V | C | `PL26` §5 test 9 | Taylor × Heggie at f64 agrees with Heggie × RK4 on tame pixels | none | near-field, `config_stability` | Taylor stepper | distribution | blocked: RQ-577 · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| PL-10 | V | C | `PL26` §5 test 10 | A `df32` Result agrees across Metal and Vulkan in every branch input and to `df32` precision in values | none | any | precision tier on the GPU | exact | blocked: RQ-577 |
| PL-11 | V | C | `PL26` §5 test 11 | Decoder switchover per precision, on a deep box at O(1) coordinates; the origin box as control | none | Latent (deep zoom) | precision tier; deep zoom (M6) | exact | blocked: RQ-577 |
| PL-12 | V | C | `PL26` §5 test 12 | Integration floor per tier; f32 with the linearised decoder must collapse at depth 16–23 | none | Latent (deep zoom) | precision tier; deep zoom (M6) | measurement | blocked: RQ-577 |

> **Note, 27 Oct 2026 (R-631).** The data PL-6, LL-5 and FF-H (REQ-VAL-042) read is now committed, no longer fetch-only:
> R-631 closes RQ-579 with option (a). PL-6's reference values, the Hristov databases' digits for the 937 rows, are in
> `HP27/data/` (`sofia_3body123/`, `sofia_3bodychor462/`, `sofia_3bodychor164/`, `sofia_3bodystab971/`), cited in
> `HP27/SOURCES.md`. FF-H's, Hristov et al. 2024's 24,582 free-fall i.c.s, are in
> `docs/incoming/2026-10-27_hp_ics_freefall/data/sofia_3bodyfree/sol_80.txt`. LL-5's source list, the authors' own
> 316-orbit table (`free-fall-3b-pictures.md`, with initial conditions at 10 decimals), is in that folder's
> `data/liliao_github/`; LL-5 stays a closure test on the `OC26` rows. Each folder's `fetch.sh` stays the provenance
> check. The table above is left as written: PL-6's "`HP27/sources.tsv` + `fetch.sh` (values hash-checked, not
> committed)" and LL-5's "source page not committed (§9)" predate R-631.

### 3.2 Replication

| Id | L | T | Source | Asserted | Ref. data | Chart | Needs | Compare | Status |
|---|---|---|---|---|---|---|---|---|---|
| SP-1 = REQ-VAL-034 (+ -127, -132; V-4) | R | A | `R25` tier A "Burrau"; `R25/catalogue.md` §3.1; `VO` §1.5; `P27` V-4; `LR` §14 | Burrau: masses 4 and 5 bind and mass 3 escapes; the close-approach sequence; breakup near t ≈ 59.4. Past f64's ~52 horizon (`VO` §1.5), so structural, never pointwise | Szebehely & Peters 1967 (`R25/papers/`), features not transcribed (REQ-VAL-127 defines them) | BodyPlane Burrau pixel | M3 + outcome classifier; a horizon past 59.4 (R-610 allows 200; the default is 50) | structural + timing window (REQ-VAL-132) | data in repo · ruled, M3 |
| TM-1 | R | A | `R25` tier A "Triple-collision points"; `R25/catalogue.md` §1.5; `LR` §14 | Binary-collision curves converge on each of the 11 triple-collision points of Table 2; `d_min` scales along Table 4's approach ICs (radii 1e-3 to 1e-5) | Tanikawa & Mikkola 2021, Tables 1, 2, 4 | Anosova entry, after the D11 mirror remap | regularisation | exact (scaling) | blocked: the paper is not in the repo (rule 1) |
| CH-1 | R | A | `R25` tier A "The Pythagorean family"; `R25/catalogue.md` §3.4 | Newtonian lifetimes and two-body encounter counts for the 16 Pythagorean triangles with hypotenuse < 100 | Chitan, Mylläri & Haque 2022 (`R25/papers/`), not transcribed | Burrau family; MassSimplex points | M3; outcome classifier | per triangle, structural (rule 8: not a converged code) | data in repo · proposed |
| B-1 (+ `R25` tier A "Burrau Lyapunov time", lifetime) | R | A | `P27` B-1 | The Newtonian lifetime of Burrau: the plot's plateau at log₁₀ T_l ≈ 1.8 (56–71); the paper prints no value | `P27/TARGETS.md` B-1 (a figure reading) | BodyPlane Burrau pixel | outcome classifier | exact to the figure's resolution | data in repo · proposed |
| B-2 (+ `R25` tier A "Burrau Lyapunov time", τ_L) | R | B | `P27` B-2 | λ₁₀ = 0.101 ± 0.005, τ_L = 4.3 ± 0.2, fitted over all ζ, relativistic included | `P27/TARGETS.md` B-2 | BodyPlane Burrau pixel | FTLE (TASK-M3-14) | distribution (loose) | data in repo · proposed |
| B-3 | R | C | `P27` B-3 | A clean forward–backward run to t = 100 needs ε = 1e-24 and 128-bit words; Table I is 1PN, not a Newtonian state | `P27/TARGETS.md` B-3 | BodyPlane Burrau pixel | precision tier | exact (above f64 only) | blocked: RQ-577 |
| P-1 | R | B | `P27` P-1; `R25` tier B "Burrau neighbourhood" | At δ = 5×10⁻⁸ the m₁ ring of β is complete (≈ 70°); m₂ and m₃ are horseshoes | `P27/TARGETS.md` P-1, P-4 (Tables I, II) | BodyPlane, polar ring sampling (six circles × 100 azimuths) | M3; outcome classifier; horizon ≤ 200 | qualitative + distribution | data transcribed · proposed |
| P-2 | R | B | `P27` P-2 | Table I: escape counts and ⟨T⟩ per ring; the counts are the predictable fraction, an analogue of `error_ratio` flagging | Table I (`P27/TARGETS.md` P-2) | as P-1 | as P-1 | distribution | data transcribed · proposed |
| P-3 | R | B | `P27` P-3 | T(α) in four classes on each ring; regular β regions coincide with regular T | figures only | as P-1 | as P-1 | qualitative | data in repo · proposed |
| P-4 | R | B | `P27` P-4 | Tables II–VII per ring: β, A, E, P, I, R; Table VII is their integrator's floor, not a target | Tables II–VII (`P27/TARGETS.md` P-4) | as P-1 | as P-1 | distribution | data transcribed · proposed |
| P-5 | R | B | `P27` P-5 | Table VIII: escaper shares 74.6 / 15.5 / 9.9 % over 831 predictable escapers | Table VIII | as P-1 | as P-1 | distribution | data transcribed · proposed |
| P-6 | R | B | `P27` P-6 | Large T oscillations at the boundaries of three regular regions | text only | BodyPlane, zoomed polar sector | as P-1 | qualitative | data in repo · proposed |
| Q-1 | R | B | `P27` Q-1 | Table I: triple-approach means over 2,500 region-D and 2,500 circle-R systems | Table I (`P27/TARGETS.md` Q-1) | Anosova entry (region D); circle R has no chart | M3; outcome classifier | distribution | data transcribed · proposed |
| Q-2 | R | B | `P27` Q-2 | Table II's binned distributions. Its Monaghan "Theory" row is exact arithmetic with no integration | Table II | as Q-1 | as Q-1; none for the Theory row | distribution; the Theory row exact | data transcribed · proposed (DRAFT-VAL-06 for the Theory row) |
| Q-3 | R | B | `P27` Q-3 | Table III: regions D + R against the 831 Pythagorean escapers, units stated (1994b's τ against 1994a's T_cr = 3.84) | Table III | as Q-1; P-1's rings | as Q-1 | distribution | data transcribed · proposed |
| Q-4 | R | B | `P27` Q-4 | The geometric control for Z: bins 17, 16, 14, 14, 12, 10, 8, 5, 4, 1 %, from 2,500 random triangles in a disc | `P27/TARGETS.md` Q-4 | none | none (geometry only) | distribution | data transcribed · proposed (DRAFT-VAL-06) |
| L-1 (+ `R25` tier B "The Maasai shield") | R | B | `P27` L-1; `R25/catalogue.md` §1.2 | The lifetime map's morphology: the 1:1 band, odd resonances, the sea of tranquillity, trenches, symmetry | figures only | Anosova entry or ShapeSphere at rest, through a tested lens remap (rule 6) | flat compute (M4) or Result (TASK-M8-74) | qualitative | data in repo · proposed |
| L-2 | R | B | `P27` L-2 | Figs. 2 and 3: the three-part yellow area; the 3.125×10⁻⁶ zoom | figures only | as L-1, deep zoom | deep zoom (M6) | qualitative | data in repo · proposed |
| L-3 | R | B | `P27` L-3; `R25` tier B | Local homogeneity: 6,212 of 21,699 (28.6 %) at ε = 1.25×10⁻³; 28.6 → 8.6 % over six bin sizes; p_ε ∝ ε^−0.5 | `P27/TARGETS.md` L-3 | as L-1 | M4 raster, or M3 at a small grid | distribution | data in repo · proposed |
| L-4 | R | C | `P27` L-4 | Reversibility ξ map and populations I and II (Fig. 9 fits) | `P27/TARGETS.md` L-4 | as L-1 | reversible occupant (logH TTL, TASK-M3-08) | distribution / qualitative | data in repo · proposed |
| L-5 | R | B | `P27` L-5 | Step halving only sharpens boundaries; an unregularised run reproduces the main structures | text only | as L-1 | the `none` occupant (M3) | qualitative | data in repo · proposed · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| AA-2 | R | A | `P27` AA-2 | Paper I decay time per near-rest start, 59 rows, each in its own τ | `P27/agekyan_anosova_I_table1_cols5to8.csv` | Anosova entry, eq. (3) velocities | outcome classifier | exact on short-lived rows, distribution otherwise | data transcribed · proposed |
| AA-3 | R | B | `P27` AA-3 | Paper I's mean T over the same 59; Fig. 4's 62 labelled points. The printed 61.6 against the column's 64.63 (rule 10) | the same CSV | Anosova entry | outcome classifier | distribution | data transcribed · proposed |
| AA-4 | R | B | `P27` AA-4 | The N = 3 trajectory against Figs. 2a–2h | figures only | Path A, single trajectory | M3 | qualitative | data in repo · proposed |
| AA-5 | R | B | `P27` AA-5 | Run accounting: our classifier's rates against theirs | `P27/README.md`, `TARGETS.md` AA-5 | Anosova entry | outcome classifier | distribution | data in repo · proposed |
| AA-6 | R | A | `P27` AA-6; `LR` §14 | Paper II decay time per exact-rest start; the 13 rows with T ≤ 10τ exact (N = 50 a conditional decay); the ensemble's 110.1 ± 12.5 τ; Table 2's counts | `P27/agekyan_anosova_II_table1_exact_rest.csv` | Anosova entry at exact rest; ShapeSphere at rest | outcome classifier | exact on the 13 rows; distribution otherwise | data transcribed · proposed (DRAFT-VAL-05) |
| AA-7 | R | B | `P27` AA-7 | Paper II's 38 completed near-rest runs; the pooled means 80.7 τ and 95.4 τ | `P27/agekyan_anosova_II_table3_near_rest_completed.csv` | as AA-2 | outcome classifier | distribution | data transcribed · proposed |
| AA-8 | R | B | `P27` AA-8 | T at rest against T near rest, per row: a published measure of sensitivity to a 10⁻⁴ velocity perturbation | the three CSVs | Latent with one momentum axis, or two runs per pixel | outcome classifier | distribution | data transcribed · proposed |
| HU-1 | R | B | `P27` HU-1; `R25/catalogue.md` §2.3 | The return-proximity map over (p₁, p₂) ∈ [0.3, 0.5] × [0.1, 0.5]: bright candidate spots, dark collision lines | figure only | new: SuvakovPlane (§2.3, §9) | closure fields (TASK-M3-11); flat compute (M4) | qualitative; exact at Table 23's minima | blocked: no task builds the chart |
| HU-2 | R | B | `P27` HU-2 | The butterfly III zoom: three candidates, the middle one at (0.405916, 0.230163), T = 13.866 | figure; Table 23 | new: SuvakovPlane, zoomed | as HU-1 | qualitative + one exact point | blocked: as HU-1 |
| AT-D1 | R | B | `ATP` D1 | The Šuvakov plane rendered at equal masses with all 1,759 orbits marked, the stable ones highlighted, on the papers' axes | `OC26` CSV; `BG26` CSV | new: SuvakovPlane | Result (TASK-M8-74) | qualitative | blocked: as HU-1 |
| AT-D3 | R | B | `ATP` D3 | The same plane at the 8 values of m₃ (§2.3), each with its orbits marked | `OC26` CSV | new: SuvakovPlane at fixed m₃ | Result | qualitative | blocked: as HU-1 |
| OC-4 = AT-D4 (+ AT-D5; `LR` §13 "Proposed contribution") | R | B | `OC26/INSTRUCTION…` item 4; `ATP` D4, D5; `LR` §13 | Stable orbits sit inside resolved regular islands and unstable ones on boundaries, read from the payload (leaf decisions, spread, `error_ratio`), against the catalogue's `stab` and `stable_pub` and their sources; D5 is the per-orbit neighbourhood image | `OC26` catalogue (`stab`, `stable_pub`, `stab_src`) | new: SuvakovPlane; Latent `z_q = 0`; others per family | Result (TASK-M8-74); leaf decisions (M6) | distribution (an agreement table) + every disagreement listed | blocked on the chart for the 1,759 + 1,350; data transcribed for the rest · proposed |
| H-1 | R | C | `P27` H-1 | Burrau to t = 16: steps and \|δE/E\| for Heggie's eq. (21) against AZ at three tolerances; compare the ratio of steps under one stepper (Heggie's: 147/129, 257/225, 455/397) | `P27/TARGETS.md` H-1 | BodyPlane Burrau pixel | regularisation (TASK-M3-07, -08) | ratio, recorded both ways | data in repo · proposed · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| H-2 | R | C | `P27` H-2 | The same for Aarseth & Zare 1974's Example II to t = 0.002 | `P27/TARGETS.md` H-2; `R25/papers/aarseth_zare_1974…` | Path A | regularisation | ratio | data in repo · proposed · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| H-3 | R | A | `P27` H-3 | C/3 against Δ toward the Lagrange triple collision: 4×10⁻⁴, 7×10⁻⁶, 1×10⁻⁷ at Δy = 10⁻⁴, 10⁻⁶, 10⁻⁸ (and 3×10⁻⁴, 6×10⁻⁶ in Δx); the reversal errors are their integrator's | `P27/TARGETS.md` H-3 | ShapeSphere at rest through Lagrange; Latent | regularisation (Heggie, R-560) | exact (scaling) | data in repo · proposed (DRAFT-VAL-03) · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| V-1 / V-3 / V-7 | R | B | `P27` V-1, V-3, V-7; `P27/valtonen2016/ch4_…` | The L/M/A/H partition as drawn (fits in `valtonen2016/data/fig4_1_*`) and a transition-entropy map; the high-entropy share 0.66–0.83 against "roughly one half" | `P27/valtonen2016/data/` | Anosova entry ×2 | a symbol-sequence field (none planned) | qualitative; the share as a distribution | blocked: no task builds a symbol-sequence field, and Chernin et al.'s definitions are not in hand |
| V-2 / V-8 | R | B | `P27` V-2, V-8 | (a) A full-lens render is mirror-symmetric about both axes to rounding; (b) banded structure persists in the three zoom windows | `P27/valtonen2016/data/fig4_5to4_9_axis_windows.csv` | as L-1 | M4; deep zoom (M6) for (b) | (a) exact; (b) qualitative | data transcribed · proposed |
| V-5 | R | B | `P27` V-5 | The book's statistical statements | none (pointers) | — | — | — | out of scope: no set-up stated; each is a pointer to a primary paper |
| V-6 | R | A | `P27` V-6 | Mikkola's near-Pythagorean bounce orbit, shot from the digitised starts | `P27/valtonen2016/data/fig3_16_mikkola_markers.csv` | BodyPlane around Burrau; Latent | M3 | exact once shot; qualitative against the figure | data transcribed · proposed |
| V-9 (+ Orlov et al. 2010; `R25` tier B "Lifetime tails") | R | B | `P27` V-9; `R25` tier B | The t_end histogram's tail: power law (differential index about −1.4 to −1.7) or exponential, with the criterion stated | `P27/TARGETS.md` V-9; `R25/papers/orlov_…` | Anosova entry; P-1's rings | outcome classifier | distribution | data in repo · proposed |
| V-10 | R | B | `P27` V-10 | Post-Newtonian Pythagorean problem | — | — | — | — | out of scope: Principia is Newtonian |
| V-11 | R | B | `P27` V-11 | Hierarchical stability limits, prograde and retrograde | primaries named in `P27` V-11 | Latent or a hierarchical slice | M3 | distribution | blocked: Valtonen & Karttunen 2006 and Mylläri et al. 2018 are not in the repo |
| GG-1 | R | B | `R25` tier B "Converged lifetime grid over D"; `R25/catalogue.md` §1.3; `LR` §14 | Boekholt, Portegies Zwart & Valtonen 2020: 1,212 converged points at spacing 1/64 to t = 10⁴; lifetime to the last close approach | figures only; per-point lifetimes not published | Anosova entry | outcome classifier; convergence reference | per point where reliable; otherwise KS | blocked: no per-point data, and t = 10⁴ is past R-610's cap of 200 (compare only systems that end before 200) |
| GG-2 | R | C | `R25` tier C "Irreversibility" | About 1 % of region D irreversible at ε = 10⁻⁷⁰ | figure | Anosova entry | reversible occupant; precision tier | distribution | blocked: RQ-577; horizon |
| TS-1 | R | B | `R25` tier B "Symbol-sequence maps"; `R25/catalogue.md` §1.4 | Maps of the first 2–4 symbols over the free-fall plane, in Tanikawa's convention and labelling (D11) | figures (`R25/papers/tanikawa_mikkola_2008…`, `…2015_PASJ…`) | Anosova entry, mirror remap | a syzygy-symbol field (none planned) | qualitative | blocked: no task builds the field |
| TS-2 | R | B | `R25` tier B; `R25/catalogue.md` §2.2 | First-symbol maps at non-zero angular momentum, virial ratio 0 to 0.7 (2008, Fig. 7) | figures | new: a fixed-k velocity prescription over D11 ∪ D12 | as TS-1 | qualitative | blocked: as TS-1; the velocity prescription is still to be read from the 2015 PASJ paper (now in the repo) |
| PA-1 | R | B | `R25` tier B "The equilateral-triangle experiment"; `R25/catalogue.md` §5.5 | Escaper maps in side-ratio space around Lagrange; 82.1 % chaotic, 17.9 % regular | figures; text | new: SideRatio, or ShapeSphere at rest by a tested remap | outcome classifier; an N_scramble ≥ 4 classifier for the split | qualitative; the split as a distribution | data in repo for the escaper maps; the split is blocked: no task builds an N_scramble classifier |
| TR-1 | R | B | `R25` tier B "Isles of regularity"; `R25/catalogue.md` §2.1 | The ι = 0 row of Trani et al.'s map as a 1D scan in binary phase: escaper sequence and f_col | figures; Tables 1–2 are for the full 3D map | new: BinaryPlusSingle | outcome classifier; an f_col colouring | qualitative | blocked: no task builds the chart |
| MT-1 | R | B | `R25` tier B "Lifetime against Lyapunov time"; `R25/catalogue.md` §5.6 | t_d ≈ 10 t_e^1.8 on free fall, two-segment fit | `R25/catalogue.md` §5.6 | Anosova entry | FTLE (TASK-M3-14); outcome classifier | distribution | data in repo · proposed |
| MK-1 | R | B | `R25` tier B "Unequal-mass escape probabilities"; `R25/catalogue.md` §5.2 | Per-body escape probabilities over eight mass sets, matched there to about 1 % | Table 1 (not transcribed) | new: BinaryPlusSingle at a mass point | outcome classifier | distribution | blocked: their runs are 3D (a planar re-run is a different experiment), and the chart is not planned |
| AN-1 | R | B | `R25` tier B (Anosova 1969 part); `R25/catalogue.md` §2.5 | 300 rest starts over (9, 3, 1), (3, 3, 1), (3, 1, 1): mean decay (27.8 ± 3.5) τ; the lightest body usually escapes | `R25/papers/anosova_1969…` (tables not transcribed) | Anosova entry at a mass point | outcome classifier; mass-simplex chart | distribution | data in repo · proposed |
| LLL-1 | R | B | `R25` tier B "Mass-plane stability map"; `R25/catalogue.md` §2.4 | Period, angular momentum and stability contours over (m₁, m₂) for one family (13,315 stable of 135,445) | figures; IC tables on the authors' site, not fetched | MassSimplex | periodic-orbit continuation (not planned) | qualitative | blocked: needs continuation, which no task builds; the atlas's 7 `li2021` rows are in OC-1 |
| SL-1 | R | B | `R25` tier B "Ergodic outcome distributions"; `R25/catalogue.md` §5.1 | Binned binary energy and eccentricity against Stone & Leigh's planar closed forms, with N_scramble ≥ 2 | the paper's closed forms | new: BinaryPlusSingle | outcome classifier; N_scramble | distribution | blocked: chart and classifier not planned |
| BZ-1 | R | C | `R25` tier C "Ensemble statistics survive trajectory error"; `R25/catalogue.md` §6.1 | Individual double-precision runs diverge while ensemble statistics agree | figshare (not fetched) | — | — | distribution | out of scope as a replication: 3D Plummer ICs. Its method is what REQ-VAL-199 and "never discard a copy" already apply · see DRAFT-VAL-08 (note, 27 Oct 2026) |
| BR-1 | R | C | `R25` tier C "Converged short trajectories"; `R25/catalogue.md` §1.10 | Per-trajectory agreement with Brutus to t = 10 | not released | Anosova entry (Breen's frame) | M3 | exact | blocked: the data is not available |
| SP-2 (proposed, 28 Oct 2026) | R | A | Portegies Zwart & Boekholt 2018 (CNSNS 61, 160; arXiv:1802.00970v1), Table 2, § 2, Appendix B: `../2026-10-29_burrau_pzb2018/README.md`; cross-references SP-1, PL-8 | **Burrau converged reference.** Brutus at ε = 10⁻²⁴, L_w = 136 bits, from Table 1's start (= SP-1's): the state at t = 100 agrees pointwise with Table 2's, for the unperturbed run and the perturbed one (δx = 10⁻¹⁰ on mass 3); after every velocity is reversed at t = 100, the state at t = 200 agrees pointwise with Table 2's (the start recovered, to ≤ 3.2×10⁻¹³); t = 200 is the time-reversed return, so its expected state is the initial condition, and it is tested twice: as a **reversibility check** (the state at t = 200 against Table 1's t = 0 start) and as a **trajectory check** (against Table 2's t = 200 values) (the owner, RQ-611; R-646); one body escapes at t ≈ 63.4 (§ 2, footnotes 3 and 4; the paper does not name it, SP-1 says mass 3) | `../2026-10-29_burrau_pzb2018/table2/table2_reconciled.csv` (transcribed twice, 0 disagreements) | BodyPlane Burrau pixel | R-33's convergence reference (pointwise past f64's ~52 horizon, `VO` §1.5); outcome classifier for the escape; a run of 200 (100 forward, 100 reversed), at R-610's cap (note below) | exact against a convergence reference, the agreement in digits a calibration (R-71); escape as a timing window | data transcribed · proposed |

*Note added 28 Oct 2026 (row SP-2, proposed; not ruled, not counted in § 3.3).* The owner asked on 28 Oct for Portegies
Zwart & Boekholt 2018 to be proposed as a new target, "Burrau converged reference", noting "its horizon t = 200 vs
R-610's cap of 200". Its record is `../2026-10-29_burrau_pzb2018/`: the sources and licences (the arXiv PDF is kept
local, its licence not CC; the journal version and its supplement could not be fetched, RQ-611), Table 2 transcribed
twice, the initial positions (Table 1's, which are SP-1's; Appendix B's `(-2, 1)` is a slip in the text) and the
conservation check. It cross-references SP-1 (= REQ-VAL-034: the same start, judged structurally because f64's horizon
is ~52) and PL-8 (whose first half runs the nominal Burrau pixel against R-33's reference to t ≈ 60): SP-2 is a
published converged state to check that reference against, pointwise, at t = 100 and, after the reversal, at t = 200.

*The horizon.* R-610, the human's words: "The cap stays at 200 until long horizons are measured for cost." Its range is
`T_horizon ∈ [0.4, 200]`, default 50. SP-2's run is 200 time units long, exactly the cap, so it sits at the cap's edge,
not past it: R-594 item 6 queues targets *past* the cap, and this one is not, on that ground. What follows, without a
ruling: at the default horizon of 50 neither checkpoint (t = 100, t = 200) nor the escape (t ≈ 63.4) is reached; the t =
100 state and the escape need a horizon of at least 100 and about 64; the t = 200 state needs the full 200, or a second
run of 100 from the reversed t = 100 state, and which of the two a test would do is not settled here; any lowering of
the cap below 200 would remove the t = 200 half, and below 100 both checkpoints.

*Note added 28 Oct 2026 (row SP-2, the owner's ruling on RQ-611).* The owner, verbatim: "RQ-611: (b) stands. arXiv v1
is the source for SP-2. I'll also try (a): fetching the journal version and its supplementary snapshots through my own
access into docs/incoming/_local/burrau_pzb2018/. When they arrive, diff Table 2 against the journal and add any
snapshot times as extra checkpoints. No separate author email (c); I'll ask Boekholt in the existing thread if he
replies. Note in SP-2 that t = 200 is the time-reversed return, so the expected state there is the initial condition,
and test it as a reversibility check as well as a trajectory check." So:

- *The reversibility.* t = 200 is the time-reversed return: 100 forward, every velocity reversed, 100 more. The
  expected state at t = 200 is therefore the initial condition. SP-2 tests it both as a **reversibility check** (the
  state at t = 200 against the t = 0 initial condition, Table 1's) and as a **trajectory check** (against Table 2's t =
  200 values).
- *The source.* arXiv v1 is the source (the owner, RQ-611 (b)); the journal version and supplement, if the owner's fetch
  succeeds, are diffed against Table 2 and their snapshot times added as checkpoints.

### 3.3 Counts

Ids folded into a merged row are named in it: AA-1 into REQ-VAL-019, V-4 into SP-1 (= REQ-VAL-034), V-2 into V-8, V-1
and V-3 into V-7, D-1 into REQ-VAL-041, D-2 into REQ-VAL-052, D-3 into REQ-VAL-184, OC-5 and AT-D7 into W-1, AT-D2 into
REQ-VAL-076, AT-D5 into OC-4, FF-H into REQ-VAL-042, and the 25 Oct tier rows into the TARGETS rows they duplicate.

| | Rows |
|---|---|
| **Targets (rows of §3.1 and §3.2)** | **112** |
| Verification / replication | 56 / 56 |
| Tier A / B / C / D | 38 / 42 / 27 / 5 |
| Data transcribed / data in repo / blocked / out of scope | 25 / 50 / 34 / 3 |
| Rows that are already ruled requirements (carrying a `REQ-VAL` id) | 25 |
| Rows only proposed | 87 |

All 49 TARGETS.md ids appear, in 46 rows (V-1, V-3 and V-7 share one, V-2 and V-8 another; AA-1 sits in REQ-VAL-019's row and V-4 in SP-1's), all D1–D7, all twelve precision-ladder
tests, all six orbit-catalogue tests, the Belgrade cross-checks (BG-1, BG-2, BG-4; items 3 and 4 merged into SD-1 and
HU-3), the high-precision closure tests (PL-6, OC-1, OC-3), the 25 Oct tiers A–D, and every target of `LR` §14.

---

## 4. Sequencing by engine milestone

The milestones are `plan/MILESTONES.md`'s, M0 to M8. Merged as of 27 Oct 2026 (`git log origin/main`): M2 through
TASK-M2-20 (PR #294) among others, and in M3 TASK-M3-01, -02, -03 and -15 (PRs #238, #280, #285, #251). Nothing below
asks a task to start early; it says what can run on the day a milestone's gate passes.

**Before any new engine work** (records and transcription, rule 1 and 2):

- Transcribe twice: Li & Liao Tables A1–A5 and A6–A10 (LL-1 to LL-4); Hudomal's Table 23 (HU-3); Szebehely & Peters's
  close-approach features (REQ-VAL-127's definition); Chitan et al.'s Newtonian table (CH-1); Anosova 1969's tables
  (AN-1).
- Fetch, or decide not to: Tanikawa & Mikkola 2021 (TM-1), Hristov et al. 2024's free-fall database (FF-H), Li & Liao's
  website pages (LL-5), Standish 1970 and Yasko & Orlov 2015 (LL-3) (§9).
- Write the remaps as tested functions (rule 6): Agekyan–Anosova's τ per row and G = 6.67×10⁻⁸; Lehto's lens; Tanikawa's
  D11 mirror; the book's ×2; Hudomal's two labellings; Breen's frame.
- Run the two no-integration controls, Q-2's Theory row and Q-4 (DRAFT-VAL-06).
- Port the §6 drafts through `plan/tools/reqio.py`, if the owner rules so.

**M2 — charts, decode and encode** (much already merged): REQ-VAL-016, -017, -019/-120 (= AA-1, merged), -020, -174
(merged); OC-2; AT-D6 (encode every atlas IC, scaled twins, yin-yang α/β).

**M3 — the integrator, events and physics validation (CPU, native).** M3 alone unlocks most of the suite, run on the
validation crate's CPU harness at f64 with the default occupant, AZ beside it, the escape and terminal events, the CPU
FTLE and the word buffer:

- *Verification, ruled:* REQ-VAL-028, -030, -041, -042 (once FF-H is fetched), -043 (once RQ-576 is ruled), -044,
  -045, -047, -052, -170, -025, -040, -195.
- *Verification, proposed:* OC-1, OC-3, OC-6, LL-1 to LL-5, SD-1, BG-1, BG-2, BG-4, HU-3 to HU-5, D-4, D-5, and
  PL-8's first half (the nominal Burrau pixel against R-33's reference).
- *Replication:* SP-1 (= REQ-VAL-034), CH-1, B-1, B-2, P-1 to P-6, Q-1 to Q-4, AA-2 to AA-8, H-1 to H-3, V-6, V-9,
  MT-1, AN-1, L-3 and L-5 at a small grid, L-4 (logH TTL).

That is 57 of the 112 rows, 59 once FF-H is fetched and RQ-576 is ruled (REQ-VAL-042, W-1); PL-8's first half also runs, on R-33's reference. What M3 alone does not unlock: anything needing a map at scale or an ensemble spread, a
chart no task builds, a precision tier, the Chazy classifier, or data not in the repo.

**M4 — flat compute on the GPU:** REQ-VAL-070/-071/-074; REQ-VAL-076/-137 (with D2's markers); L-1, L-3 at full
resolution, L-5; V-8(a); D-4 again on the GPU.

**M5 — quads, the scheduler and caching:** REQ-VAL-172, REQ-VAL-173, and every ensemble-spread reading.

**M6 — adaptive refinement and deep zoom:** L-2 (Fig. 3), V-8(b), and the leaf decisions OC-4/D4 reads.

**M7 — colour and the Chazy classifier:** REQ-VAL-183, REQ-VAL-184 to -187 (D-3).

**M8 — the GUI, Result and the release gates:** REQ-VAL-199; Result runs (TASK-M8-74, R-572) for AT-D1, AT-D3 and
OC-4/D4 at f64, uniform, once their charts exist.

**In no milestone** (each queued as a chart or definition question when its target comes up, as the 25 Oct plan said,
not built ahead of it): the Šuvakov velocity plane (HU-1, HU-2, AT-D1, AT-D3, most of OC-4); a binary-plus-single chart
(TR-1, MK-1, SL-1); a side-ratio chart (PA-1, which may remap onto ShapeSphere instead); a syzygy-symbol field (TS-1,
TS-2, V-7); N_scramble and f_col (PA-1, SL-1, TR-1); periodic-orbit continuation (LLL-1); and the precision ladder
(PL-1 to PL-12, B-3, GG-2) until RQ-577 is ruled.

---

## 5. The first batch M3 can run

Chosen from §4's M3 list because each is exact or nearly so, needs no data the repo lacks, and has an arm that can fail
(REQ-VAL-007):

1. **OC-1**, atlas closure at f64 (DRAFT-VAL-01). The largest exact set, with unequal masses and non-zero L.
2. **LL-1 and LL-2**, Li & Liao closure and brake symmetry, with the alias pairs (DRAFT-VAL-02). Free fall, unequal
   masses, and §2.1's new check. Needs the two transcriptions first.
3. **H-3**, `d_min` scaling toward the Lagrange triple collision under Heggie (DRAFT-VAL-03). The one exact
   regularisation target among the papers.
4. **H-1 and H-2**, the Heggie-to-AZ step ratio under one stepper (DRAFT-VAL-04).
5. **AA-6's 13 short-lived rows** (DRAFT-VAL-05). Principia's own free-fall surface, from exact rest.
6. **Q-2's Theory row and Q-4** (DRAFT-VAL-06). No integration; they pin the eccentricity and Z definitions before any
   dynamics is graded with them.
7. **BG-4**, the butterfly periods (DRAFT-VAL-07). A published discrepancy that closure decides.

They replace TARGETS.md's "Five to do first" (AA-6, LL-1/LL-2, Q-2/Q-4, P-1/P-2, L-3). P-1/P-2 and L-3 stay in M3's
list; they are statistical, so they follow the exact ones.

---

## 6. Draft requirements for the first batch

**The format is the repo's own**: `plan/requirements.yaml`, with `plan/tools/reqio.py`'s key order (id, area, kind,
statement, source, verify, rulings, milestone, rq, note). As in the existing pairs (REQ-VAL-041 with -133, -170 with
-171), each gate comes with a `kind: calibration` requirement for its tolerance. **The ids are placeholders**
(`DRAFT-VAL-nn`); `reqio.py` assigns permanent ones when ported, and the next free number today is REQ-VAL-201. Nothing
here is ported by this commit.

**Every tolerance is a CALIBRATION under R-71:** proposed with its evidence by the task that measures it, checked by a
reviewer, confirmed by the owner at the M3 gate and recorded in `decisions.md`. What is proposed below is the form of
each tolerance and what sets its floor, never a number chosen to match a published value.

```yaml
- id: DRAFT-VAL-01
  area: VAL
  statement: "Every Three Body Orbits atlas row whose catalogued refined closure is below 1e-9 must return to its initial state after one period T when integrated from its f64 refined initial conditions (`ref_*`) under the default occupant (Heggie × RK4, R-560) and under Aarseth–Zare, at each occupant's stated order across an eta ladder; rows from the site's own searches are closure tests only and never cited (rule 13)."
  source:
  - file: docs/incoming/2026-10-26_orbit_catalogues/INSTRUCTION_orbit_catalogues_2026-10-26.md
    section: What to do with it (an addition to tier A)
  - file: docs/incoming/2026-10-27_validation_plan/PLAN.md
    section: 3.1 Verification (OC-1)
  verify:
    method: numerical gate
    detail: "reference data docs/incoming/2026-10-26_orbit_catalogues/threebodyorbits_ics.csv (sha256 52a67e5d…), short periods first, the smallest-rmin rows included on purpose; per row |state(T) − state(0)| at each eta and the fitted order; a row passes when its closure sits within DRAFT-VAL-01c's tolerance of its floor and its order matches the occupant's; control: a row integrated to 0.9·T must fail; every figure carries the CC BY 4.0 credit line of CITATIONS.md"
  rulings: [R-71, R-560]
  milestone: M3
  note: "The floor is the f64 rounding of ref_* (the operator: only the double-precision part of the double-double refinement is stored) amplified along the orbit, never the catalogued closure (median 1.3e-24). The hunt's orbits are not in the snapshot and stay out."
- id: DRAFT-VAL-01c
  area: VAL
  kind: calibration
  statement: "DRAFT-VAL-01's per-row closure tolerance must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71), form proposed: tolerance = k × (relative rounding of ref_*, about 1.1e-16) × A(row), A the measured growth of an initial-condition perturbation over one period (measured per row, for example by a finite-difference monodromy estimate, with the catalogue's stab compared beside it), and k measured on the stable rows; never set from a published closure."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-02
  area: VAL
  statement: "Each of Li & Liao 2019's 234 free-fall initial conditions (Tables A1–A5) must, under the default occupant at f64, have every velocity vanish at T/2 and return to rest in its initial configuration at T; and for each pair the atlas aliases (F1/F2, F12/F13, F23/F24 at masses (1, 1, 1); F11/F16 at (1, 0.8, 0.6)) the T/2 rest configuration, rescaled into region D, must equal the partner row with T·|E|^{3/2} equal."
  source:
  - file: docs/incoming/2026-10-27_papers/TARGETS.md
    section: LL-1, LL-2
  - file: docs/incoming/2026-10-27_validation_plan/PLAN.md
    section: 2.1 The atlas's 311 free-fall orbits against Li & Liao's 234
  verify:
    method: numerical gate
    detail: "reference data: Tables A1–A5 transcribed twice into a CSV (rule 2) from docs/incoming/2026-10-24_anosova/papers/li_liao_2018_arXiv_1805.07980.pdf, not the atlas's rows (TARGETS LL-1); unequal sets on the MassSimplex chart; residuals at T/2 and T per row against DRAFT-VAL-02c; control: the velocities at T/4 must be non-zero, so the T/2 test reads a minimum and not a constant; each alias pair's T·|E|^{3/2} from our run, beside the paper's"
  rulings: [R-71, R-560]
  milestone: M3
  note: "Li & Liao print 10 decimals with trailing zeros dropped; HP27 found their 2017 tables not correct to the last printed digit on 5 of 5 rows checked, so the floor may sit above half a unit in the tenth decimal. Record, don't fix."
- id: DRAFT-VAL-02c
  area: VAL
  kind: calibration
  statement: "DRAFT-VAL-02's tolerances at T/2 and at T must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71), form proposed: tolerance = k × 5e-11 (half a unit in the printed tenth decimal) × A(row), A measured as in DRAFT-VAL-01c; the measured residual distribution printed per mass set before k is proposed."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-03
  area: VAL
  statement: "Released from rest at (−1, 0), (1, 0), (Δx, √3 + Δy) with equal masses, under the default occupant, a third of the minimum perimeter (C/3) must scale with Δ as Heggie 1974's Table III does: 4e-4, 7e-6, 1e-7 at Δy = 1e-4, 1e-6, 1e-8, and 3e-4, 6e-6 at Δx = 1e-4, 1e-6; the reversal errors in that table are Heggie's integrator's and are not asserted."
  source:
  - file: docs/incoming/2026-10-27_papers/TARGETS.md
    section: H-3
  verify:
    method: numerical gate
    detail: "C/3 per Δ and the log-log slope between rungs, recorded beside Heggie's; √3 entered as Heggie did (rounded to 15 decimals) and at full f64, both recorded; the Δ = 0 run (homothetic collapse) reaches its minimum C without a non-finite state; control: the unregularised `none` occupant run alongside."
  rulings: [R-71, R-560]
  milestone: M3
- id: DRAFT-VAL-03c
  area: VAL
  kind: calibration
  statement: "DRAFT-VAL-03's tolerance on C/3 per rung and on the slope must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71), form proposed: Heggie prints one significant figure, so the bound is stated in that figure's own resolution (a factor, not a difference), with our own convergence in eta measured first; the published scaling is not proportional to Δ, and that non-proportionality is the target, not something to fit away."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-04
  area: VAL
  statement: "Under one stepper, the ratio of steps taken by Heggie's regularisation to steps taken by Aarseth–Zare's, at matched final energy error, must be measured for Burrau to t = 16 and for Aarseth & Zare 1974's Example II to t = 0.002, and recorded beside Heggie 1974's Tables I and II (step ratios 147/129, 257/225, 455/397; 48/42, 83/70, 149/124)."
  source:
  - file: docs/incoming/2026-10-27_papers/TARGETS.md
    section: H-1, H-2
  verify:
    method: numerical gate
    detail: "steps and |δE/E| per tolerance or eta, both occupants, one stepper; the ratio is the compared quantity (Heggie used RKF7(8), which we do not ship); a disagreement is queued in REVIEW_QUEUE.md with both numbers (rule 4)"
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-04c
  area: VAL
  kind: calibration
  statement: "The band within which DRAFT-VAL-04's measured step ratio counts as agreeing with Heggie's must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71): the spread of our own ratio across the eta ladder is measured first, and the band is proposed from it; Heggie's three ratios per table set no band."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-05
  area: VAL
  statement: "For the 13 exact-rest region-D starts of Agekyan & Anosova II (Table 1) with T ≤ 10τ (N = 2, 7, 18, 43, 50, 51, 59, 61, 74, 85, 87, 95, 100; N = 50 a conditional decay), each run's decay time, converted to that row's own τ (paper I eq. 7 with L₀ = 0), must match the printed T within DRAFT-VAL-05c's tolerance, under both the paper's decay criterion (Tevzadze, paper I eq. 10, transcribed) and ours, both recorded."
  source:
  - file: docs/incoming/2026-10-27_papers/TARGETS.md
    section: AA-6
  - file: docs/incoming/2026-10-27_papers/agekyan_anosova_II_table1_exact_rest.csv
    section: rows N = 2, 7, 18, 43, 50, 51, 59, 61, 74, 85, 87, 95, 100
  verify:
    method: numerical gate
    detail: "the remap (G = 6.67e-8 in their units against G = 1; τ per row) is a tested function (rule 6); the energy column is recorded and not asserted (RQ-570's kind of discrepancy, TARGETS AA-6); the other 87 rows are compared as a distribution, not per row; control: a run with the remap's τ deliberately set from L₀ = 7.5e-9 (paper I's near-rest value) must move the comparison"
  rulings: [R-71, R-592]
  milestone: M3
  rq: [RQ-570]
- id: DRAFT-VAL-05c
  area: VAL
  kind: calibration
  statement: "DRAFT-VAL-05's per-row tolerance on T must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71), form proposed: the larger of the printed resolution (0.1τ) and the spread of our T across the eta ladder and the two criteria; their integrator was unregularised and reached |ΔE/E| = 0.019, so a disagreement is recorded first and attributed only by measurement (rule 5)."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-06
  area: VAL
  statement: "Two controls that need no integration: (i) Monaghan's planar eccentricity distribution f(E) = E/(1 − E²)^{1/2}, integrated over ten bins of width 0.1 and rounded half-up, must reproduce Aarseth et al. 1994b's Table II 'Theory' row exactly (1, 2, 3, 4, 5, 7, 9, 11, 16, 44 %; median 0.87); (ii) 2,500 triangles drawn uniformly in the unit disc, with Z computed by the validation crate's own Z, must give a distribution compatible with that paper's 17, 16, 14, 14, 12, 10, 8, 5, 4, 1 %."
  source:
  - file: docs/incoming/2026-10-27_papers/TARGETS.md
    section: Q-2, Q-4
  verify:
    method: unit test
    detail: "(i) exact; its negative control, the 3D thermal form f(E) = 2E, must fail; (ii) a χ² or KS statistic against the printed bins, with both readings of 'the maximum distance represents the binary' run and recorded (TARGETS Q-4); the eccentricity and Z functions are the ones the outcome code uses, not copies"
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-06c
  area: VAL
  kind: calibration
  statement: "DRAFT-VAL-06 (ii)'s acceptance level must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by the human at the M3 gate and recorded in decisions.md."
  verify:
    method: review checklist
    detail: "CALIBRATION (R-71): the printed percentages are truncated (1994b p. 135), so the statistic's null is built from repeated draws of 2,500 with the same truncation, measured, and the level proposed from it."
  rulings: [R-71]
  milestone: M3
- id: DRAFT-VAL-07
  area: VAL
  statement: "From the Belgrade gallery's (p₁, p₂) for butterfly I (I.2.A) and butterfly II (I.2.B), the time of the return-proximity minimum near one period must be measured under the default occupant and recorded against both published periods (6.235641 and 6.234671; 7.003907 and 7.003707), and against the atlas's refined periods; the gallery CSV is not edited."
  source:
  - file: docs/incoming/2026-10-26_belgrade_gallery/README.md
    section: One internal discrepancy, recorded
  - file: REVIEW_QUEUE.md
    section: RQ-576, "Recorded, not resolved"
  verify:
    method: numerical gate
    detail: "the minimum's time with its uncertainty propagated from the six-decimal (p₁, p₂) and the eta ladder; the test asserts the uncertainty is smaller than the published periods' difference (about 1e-3 for I, 2e-4 for II) before it reads which period is nearer; the result goes to RQ-576's record"
  rulings: [R-71]
  milestone: M3
```

---

## 7. Coverage map

Which part of initial-condition space each target exercises. Charts are the ones §3 names.

| Region of IC space | Targets | Charts |
|---|---|---|
| **Region D / free fall, equal masses at rest** | AA-1 to AA-8; L-1 to L-5; GG-1, GG-2; TS-1; TM-1; MT-1; Q-1, Q-2; V-2/V-8, V-7, V-9; LL-1 to LL-3 at (1, 1, 1); REQ-VAL-042 (FF-H), -044, -076; BR-1; PA-1 and H-3 at the Lagrange point | Anosova physical-frame entry; ShapeSphere at rest; Latent `z_q = 0` |
| **Free fall, unequal masses** (points on the mass simplex) | LL-1 to LL-5 (nine mass sets); AN-1; CH-1; REQ-VAL-076's nine planes | MassSimplex; Anosova entry at a mass point; the Burrau family |
| **The Burrau neighbourhood** | SP-1 (REQ-VAL-034); P-1 to P-6; Q-3; B-1 to B-3; H-1; CH-1; V-6; PL-5, PL-7, PL-8; REQ-VAL-016, -174; D-4, D-5 | BodyPlane; Burrau {ν, K} |
| **The Šuvakov velocity plane** (equal masses, L = 0, collinear start) | SD-1; HU-1 to HU-5; BG-1, BG-4; W-1; OC-1 (1,759 rows); PL-6 (937 rows); AT-D1; OC-4/D4 | new: SuvakovPlane (§2.3) |
| **The same plane, m₃ free** | OC-1 (1,350 rows, 8 values of m₃); AT-D3; OC-4/D4 | new: SuvakovPlane at fixed m₃ |
| **Non-zero angular momentum** | BG-2 (57 BHH satellites); OC-1 (437 rows: Simó 344, Broucke, BHH, Sheen, Šuvakov's other orbits); TS-2; V-11 | InvariantLE / LK; Latent |
| **Binary + single** (hierarchical infall, scattering) | TR-1; MK-1; SL-1; V-11 | new: BinaryPlusSingle |
| **Near triple collision** | TM-1; H-3; PA-1; REQ-VAL-047, -020 | ShapeSphere at rest; Anosova entry |
| **Whole-engine properties** (no region) | REQ-VAL-028, -170, -045, -030, -172, -173, -025, -040, -017, -070, -199, -183; PL-1 to PL-4, PL-9 to PL-12 | — |

> **Note added 27 Oct 2026.** `PROPOSAL_occupant_matrix_census_2026-10-27.md` beside this file (DRAFT-VAL-08, proposed,
> not in force; RQ-599) crosses the targets above with every occupant, precision, step control and build, as an R-71
> calibration map and never a gate. It extends REQ-VAL-170, -045, -030, -041, H-1, H-2, H-3, D-2, D-4, OC-1, PL-5, PL-7,
> PL-9, L-5 and BZ-1, each marked "see DRAFT-VAL-08" in §3. No target, chart or region here changes.

### Gaps: where nothing published exists, in these records

This list is part of the case for Principia's novelty, so it is stated narrowly. Each gap is "not found in the records
in this repo (`R25/catalogue.md`, `P27`, `OC26`, `BG26`, `LR`)", and `LR` §15's scope caveat applies: "any strong gap
claim should be treated as a research hypothesis rather than an established fact."

1. **No outcome, lifetime or chaos map over the Šuvakov velocity plane.** The one published map there is Hudomal's
   return-proximity map (HU-1), a search tool for periodic orbits, not an outcome map. 3,109 catalogued orbits sit on
   that plane and its m₃ stack (§2.3), with no published picture of what surrounds them.
2. **No pixel-resolution map over the mass simplex.** Mass appears as a map axis only in Li, Li & Liao's continuation of
   one periodic family (LLL-1); otherwise as fixed sets (LL-1 to LL-5, AN-1, MK-1) or ensemble statistics.
3. **Non-zero angular momentum: first-symbol maps only.** Tanikawa & Mikkola 2008's Fig. 7 (TS-2), qualitative, with
   the velocity prescription not yet pinned. No lifetime or outcome map with L ≠ 0, and no map that moves between
   values of L.
4. **The Burrau neighbourhood has no 2D map.** Aarseth et al. 1994a sample six circles × 100 azimuths per body (P-1),
   and every other study integrates the single point. Nothing at non-zero velocity around it.
5. **The Pythagorean family, one trajectory per member** (CH-1): nothing on how the structure around each member
   varies across the family (`LR` §15, "Structured family surveys").
6. **Periodic orbits in their initial-condition context.** No catalogue places its orbits in the IC space around them
   or compares their linear stability with that structure (`LR` §13 "Proposed contribution", §15). OC-4/D4 would be
   the first test of "stable ⇒ island, unstable ⇒ boundary" against independent stability analyses.
7. **Planar binary + single: one-dimensional scans only.** The ι = 0 row of Trani et al.'s map (TR-1) and Hut's
   phase scans (`R25/catalogue.md` §4.1); no planar 2D map.
8. **Per-pixel reliability, simultaneous diagnostics and the per-pixel encounter word** (`LR` §15): no prior map
   reports, pixel by pixel, whether its diagnostic can be trusted.

Region D at rest, by contrast, is well mapped: Agekyan & Anosova, Lehto, Boekholt et al., Tanikawa & Mikkola,
Parischewsky et al., Li & Liao and Hristov et al. all work there. That is why most replication is possible there, and
why novelty is not claimed there.

---

## 8. Rules

### Carried over from the 25 Oct instruction, verbatim

The 25 Oct instruction's rules 1–8, "the ones that worked for Anosova, RQ-566 to RQ-573" (its section "Rules"):

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

### Learnt since

9. **Record the escape criterion as data, with its parameters and its clock.** Rule 7 extended: every comparison row
   stores the paper's criterion with its numbers, the time it reports, and ours. They differ even between companion
   papers: 5R_grav for conditional escape in Aarseth et al. 1994a, 10d in 1994b (`P27` Q-1 caveat); Boekholt et al.
   2021's conditions held for 100 time units, from the centre of mass (`P27` B-1); Boekholt et al. 2020's lifetime to
   the last close approach, not to the criterion firing (`R25` tier B); Lehto's ratio 100 or 2000 (`P27` L-1). The
   crossing time is a unit of the criterion too: 1994a's T_cr = 3.84 against 1994b's τ ≈ 1.255 for the Burrau masses
   (`P27/README.md` §5).
10. **A paper's own numbers can be internally inconsistent: record, don't fix.** Agekyan & Anosova I print a mean of
    61.6 over 59 decay times whose mean is 64.63; paper II's "12 conditional decays" are 14 asterisks, and its "100
    systems" are 97 values (`P27/README.md` §5); the gallery's two butterfly periods (`BG26`); Li & Liao's tables not
    correct to the last printed digit (`HP27`); RQ-570's energy column. Each stays as printed, beside our number, and
    the disagreement is queued, not resolved by choosing.
11. **Remap units and labels, not only coordinates.** Rule 6 extended: τ per row (Agekyan & Anosova eq. 7), G =
    6.67×10⁻⁸ against 1, E = −½ rescaling (Hudomal Tables 16–22), the book's ×2 scale (`P27` V-1), two labellings in
    one thesis (`P27` Hudomal set-up), the atlas's masses as given and not normalised (`OC26/INSTRUCTION…` "Rules"). A
    remap is a tested function with a round trip.
12. **Declare the word equivalence before comparing words.** The gallery and the PRL write the same conjugacy classes
    with different representatives, 6 of 15 differing by more than rotation, and moth III's published word is not
    freely reduced (`BG26` item 3; RQ-576). The atlas states its own canonical rule in code (`js/hunt/word.js`, per
    `ATP` §1): freely reduce the **syzygy sequence**, relabel by mass, and take the smallest spelling over cyclic
    rotations, time reversal and the relabellings the masses allow. It is a different alphabet from the free-group
    words (syzygies, not a/b branch-cut letters), so it answers the question for `syz` and syzygy sequences and is a
    precedent, not a drop-in, for RQ-576.
13. **A compilation is an index, never a source.** A reference value traces to the paper (`OC26/INSTRUCTION…` "Its
    status"). A site's own orbits, and orbits a paper does not contain (LL-5), test closure only and are never cited as
    published results. The hunt's orbits stay out until the operator publishes them.
14. **A tolerance is set by the input's precision times the orbit's instability, never by a catalogued closure**
    (`OC26/INSTRUCTION…` "Precision of the stored initial conditions").
15. **Check where the digits came from.** Digits past the data are arithmetic: Simó's "30-digit" rows carry about 16
    digits of data, and Li & Liao's 100-digit rows are Hristov's (`HP27`). Count the digits of the source, not of the
    file.
16. **A test that cannot fail is decoration.** Every target carries an arm that would fire (REQ-VAL-007, ruled): a
    negative control for an exact test, a shuffled or perturbed control for a distribution.
17. **Commit third-party data only under stated terms.** The atlas and the gallery state theirs; Hristov's databases,
    Simó and Li & Liao do not (RQ-579), so they are fetched and hash-checked, not committed; the Valtonen book is not
    committed (`P27/README.md` §2).
    *Note, 27 Oct 2026:* R-631 commits Hristov's databases, Simó's archive and Li & Liao's tables with full citations
    (HP27 and `docs/incoming/2026-10-27_hp_ics_freefall/`, each with a `SOURCES.md`); the rule above is left as written.
18. **Respect the horizon cap; queue, don't raise it.** R-610 (closing RQ-555): `T_horizon ∈ [0.4, 200]`, default 50.
    A target past 200 (Boekholt et al. 2020's t = 10⁴) is compared only over the systems that end before 200.

---

## 9. Open questions for the owner

Each is a one-line choice, with this plan's recommendation.

1. **Adopt this file as the single plan, superseding the 25 Oct tier tables, sequencing and fetch list?** Recommended:
   yes, by a ruling that names it.
2. **Port §6's drafts into `plan/requirements.yaml` now, or when the M3 tasks that run them are dispatched?**
   Recommended: now, as M3 requirements, so the M3 gate carries them.
3. **The Šuvakov plane's labelling: the middle body as the decoder's body 2 (the `α = 0` pole) or as body 1 (`β = 0`,
   reached exactly, R-417)?** Recommended: body 1, Hudomal eq. (18)'s labelling, queued as a chart question with the
   physics reviewer (§2.3).
4. **Build a Šuvakov-plane chart, or keep its targets as Path A closures until a chart question comes up?** Recommended:
   queue the chart question now; 3,109 orbits and the strongest share-back image wait on it.
5. **RQ-576's word equivalence?** Recommended: option (d), conjugacy plus the inverse and the `(a, b) ↔ (A, B)` exchange,
   with each row recording which symmetry it needed; cite the atlas's `word.js` rule for syzygy sequences. (RQ-576 asked
   for no recommendation from its author; this one is the plan's.)
6. **RQ-579, the high-precision databases' terms?** Recommended: (b), ask the authors, keeping (c) meanwhile.
7. **Fetch Hristov et al. 2024's free-fall database (24,582 ICs at 80 digits) under the same fetch-and-verify rule, for
   REQ-VAL-042?** Recommended: yes.
8. **Li & Liao's website pages (§2.1): commit them, or record their URL and sha256 only?** Recommended: record only, as
   `HP27` does, and treat the 81 extra atlas rows as closure tests (LL-5).
9. **Fetch Tanikawa & Mikkola 2021, Standish 1970 and Yasko & Orlov 2015 (TM-1, LL-3)?** Recommended: yes; TM-1 is one
   of very few exact regularisation targets.
10. **RQ-577, the precision ladder: rule before or after M3 closes?** Recommended: after, as the proposal's own §6
    question 1 allows; PL-8's first half runs on R-33's reference at M3 without it.
11. **Tier D: enumerate the prototype's numbers as targets?** Recommended: no blanket list; promote a prototype number
    by name when a task needs it, re-measured under Heggie × RK4, since several were measured under AZ (§3, D-1).
12. **Queue the §10 corrections to ruled documents (`VO` §1.1, REQ-VAL-042's citation, `LR` §13's sentence)?**
    Recommended: yes, as REVIEW_QUEUE entries; this plan does not edit them.

---

## 10. Superseded or corrected

What this plan changes in the older records, and why. The records themselves are not rewritten; the three files
marked "note added" get a short dated note pointing here.

| Record | Was | Now | Why |
|---|---|---|---|
| `R25` INSTRUCTION, tier tables A–D, "Sequencing", "Papers to fetch" | the working plan | superseded by §3 and §4 (note added) | they predate `P27`, `OC26`, `BG26`, `HP27`, `PL26`, `ATP` and `LR`'s third and fourth revisions |
| `R25` INSTRUCTION, "What cannot be replicated" | "RQ-555 keeps the horizon at most 200" | R-610 (closing RQ-555): `T ∈ [0.4, 200]`, default 50 | RQ-555 was ruled on 25 Oct 2026. R-555 is a different ruling (Listen), not the horizon |
| `R25` INSTRUCTION, tier A "Region D Table 1" | "under way in TASK-M2-20" | merged, PR #294 (27 Oct 2026); RQ-570 open | `git log origin/main` |
| `R25` INSTRUCTION, "Rules" header | "RQ-566 to RQ-573" as Anosova's | Anosova's are RQ-566, -570, -571 and -573; RQ-572 was the lock-validation question (closed by R-593); RQ-567 to -569 do not occur in the repo | `decisions.md` R-592, R-593; a search of the repo |
| `R25` INSTRUCTION, tier D, first bullet | "figure-eight convergence order 2.08 with the landing clamp, 1.06 without" | those are AZ's; under the default Heggie × RK4 (R-560) the prototype measured 2.40 and 1.03 | `FND` §3.2 |
| `R25` INSTRUCTION, tier B "Lifetime tails" (Orlov 2010) | its own row | merged into V-9 | one target |
| `P27` TARGETS.md, "Five to do first" | the first batch | §5's M3 batch | adds OC-1, H-1 to H-3 and BG-4; keeps AA-6, LL-1/LL-2, Q-2/Q-4; moves P-1/P-2 and L-3 after the exact ones |
| `P27` TARGETS.md (note added) | 49 targets, standalone | folded into §3; ids kept | one table |
| `P27/README.md` §5 | "The extra 77 must come from elsewhere … Not resolved here" | resolved (§2.1): the authors' website lists 316; 311 = 316 − 5 aliases | measured on `OC26`'s catalogue and the authors' pages |
| `P27` TARGETS.md LL-1 | "the atlas … carries a free-fall family attributed to this paper" | 230 of the paper's 234 and 81 rows the paper does not contain | §2.1 |
| `PL26` §5 test 6 (note added) | "1,260 rows traced to their source paper" | 937 Hristov rows: 688 at 97–100 digits, 9 at 150, 240 at 180; the values from the databases, fetched and hash-checked | `HP27/README.md`, `agreement.txt` (§2.2) |
| `PL26` §5 test 6 | "the 30-digit ones cannot test `dd64`" | Simó's rows carry about 16 digits of data and test nothing above f64 | `HP27/README.md` |
| `PL26` §5 test 6 | `atlas-params.json` "(… `docs/incoming/2026-10-26_orbit_catalogues/`, unmerged)" | that folder is on `main`; `atlas-params.json` stays outside the repo | `git ls-tree origin/main` |
| `OC26/INSTRUCTION…` "`atlas-params.json`" | "On 1,260 rows `ic_hp` agrees … (Hristov et al., Simó, Li & Liao)"; the 99 "closure tests only" | 97 of the 99 are `hristov2026` by matching; Simó's digits past ~16 are arithmetic; Li & Liao's 5 high-precision rows are Hristov's | `HP27/README.md` |
| `ATP` §3 | "about 72 mass values" for the m₃-free stack | 8 values | §2.3 |
| `ATP` §3, D2; `LR` §13 (lines 698–699) | 311 free-fall orbits on "one plane" | nine planes, one per mass set | §2.3; `LR` is not edited (§9 question 12) |
| `ATP` §5 step 4 | "remember f64's predictability horizon is about 52" | unchanged; stated with R-610's cap | — |
| `VO` §1.1 | "Li & Liao 2018 (234 unequal-mass), Hristov–Hristova–Dmitrašinović–Tanikawa 2023 (12,409 equal-mass)" | 234 over six mass sets, 30 of them equal-mass (`P27/README.md` §4 item 7); Hristov et al. is *CeMDA* 136, 7 (2024), arXiv 2023 | to queue (§9 question 12): `VO` is in force and is not edited here |
| REQ-VAL-042 | "Hristov et al. 2023" | *CeMDA* 136, 7 (2024) | to queue with the above |
| `BG26/README.md` | "on its own branch, not yet merged" | `OC26` and `BG26` are both on `main` | `git ls-tree origin/main` |
| The task brief for this plan | "TASK-M2-20 is under way" | merged | as above |

