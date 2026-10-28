# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

## RQ-378: Whether "Save view as chart" on a curved chart should also offer a flattened oblique-latent version, where one exists locally *(the human's queued question, R-469 §11, R-475; context: TASK-M2-40, TASK-M8-48)*

*Queued by the human (R-469 §11: "Whether "Save view as chart" on a curved chart should also offer a flattened
oblique-latent version, where one exists locally."). For the human: yes, it changes what Save view as chart builds.*

- **File, section:**
  - `decisions.md` § "R-475 — …", the human's words, S2: on a curved chart the saved chart is "the parent chart, its
    slice centre, and the tilt (map, angles, targets)"; on a latent view, "`(z₀, q₁, q₂)` as an oblique chart".
  - `docs/gui/principia_render_gui_spec.md` § "G2. …", the Chart bullet's Save view as chart.
- **The question:** a curved view near its centre is approximated by its tangent plane, an oblique latent chart
  `(z₀, J q̂₁, J q̂₂)`; whether saving should also offer that flattened chart, and what "exists locally" requires (a
  full-rank Jacobian at the centre, the window small enough).
- **Options seen:** (a) no, the parent chart and its tilt only (S2 as written); (b) yes, a second option "Save as
  flat chart", offered where the decoder's Jacobian at the centre has rank 2 (it needs the derivative RQ-341 raises);
  (c) yes, with the flattened chart shown only as provenance, not offered.
- **Applied meanwhile:** (a). Nothing waits.
- **Parked:** R-510 (20 Oct 2026), the human: "RQ-378: parked until RQ-341." A deferral by ruling: the entry stays
  open, blocked on RQ-341, and returns to the human when RQ-341 is ruled; (a) stays applied and nothing waits.
- **21 Oct 2026:** RQ-341 is ruled (R-550), so R-510's parking has ended and the entry is with the human again. The
  human's batch of 21 Oct leaves it open (R-545); (a) stays applied and nothing waits.
- **21 Oct 2026 (evening):** RQ-341 is ruled (R-550), and the human parked RQ-378 again, with no target (R-568): "yeah
  ok we can park that for now then". The entry stays open, (a) stays applied, nothing waits, and it returns when he
  raises it. *Was:* "RQ-341 is ruled (the 21 Oct batch, PR #274)", before R-550 was on main.
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-510, R-568); what it applies stays applied, and nothing waits.

## RQ-386: Whether tier 2 of the embedding should split further, the ramp window and colour mode above the overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes *(the human's queued question, R-484 RT2; context: TASK-M7-44)*

*Queued by the human (the round-trip rule, "Queue, don't rule"). For the human: yes, it changes what a small image
carries.*

- **File, section:**
  - `decisions.md` § "R-484 — …", the human's words: "Whether tier 2 should split further, for example ramp window and
    colour mode above overlays and lines, if measurement shows tier 2 itself often does not fit at small sizes." and
    RT2's table: "**2. Colour** | `RenderState` as built-ins: colour mode enum and every parameter, the ramp window,
    overlays and the line system."
  - `docs/design/principia_dd_image_embedding.md` § "6. What travels": "**The ramp window is not optional.** An
    auto-ranged ramp manufactures or hides the difference it is meant to show".
  - The same, § "7. Measured capacity and behaviour", to be re-measured with the tiers (R-483 H6, TASK-M7-44).
- **Options seen:** (a) tier 2 whole, as RT2 has it; (b) tier 2a (the stain graph's built-ins, their parameters and the
  ramp window) above tier 2b (overlays, lines, the playhead and hover, `f3_fill`), 2b dropping first; (c) decided by
  TASK-M7-44's measurement: split only if tier 2 fails to fit at 64² or 128² for a typical stain.
- **Applied meanwhile:** (a). Nothing waits; TASK-M7-44's measurement reports tier 2's size so the human can rule with
  it.
- **Parked:** R-522 (20 Oct 2026), the human: "RQ-386 after the capacity re-measure." A deferral by ruling: the entry
  stays open, waiting on TASK-M7-44's capacity re-measure (REQ-TOOL-174), which reports tier 2's own size and, at
  64² and 128², whether tiers 1 and 2 fit; it then returns to the human with those numbers. Meanwhile (a), tier 2
  whole, stands, and nothing else waits.
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-522); what it applies stays applied, and nothing waits.

## RQ-391: Whether the Export Styles' frames (Polaroid border, banknote, poster) become a later "Frame" stage after the Print graph *(the human's queued question, R-485, §0 "separate, later", the message's point 6; context: none until ruled)*

*Queued by the human (point 6, as RQ-388), and §0: "On presentation frames (Polaroid, banknote, posters): separate,
later." For the human: yes, whether there is a Frame stage at all.*

- **File, section:** `docs/design/principia_print_graph.md` § "12. Open questions (for the review queue, not the agent
  to settle)", question 4: "Should the **Export Styles** frames (Polaroid border, banknote, poster) become a later
  "Frame" stage after the Print graph? Ruled out of scope for this bundle."
- **Options:** (a) a Frame stage after the Print graph, its own design later; (b) frames as Print graph presets, a
  sub-graph like any other; (c) frames as an export option only, outside `RenderState`.
- **Applied meanwhile:** nothing is built (§0: "separate, later"); the lab's Polaroid medium (a Photographic node and
  preset) is a medium, not a frame, and is ported with the rest (R-486). Nothing waits.
- **Parked:** R-527 (20 Oct 2026), the human: "RQ-391 later." A deferral by ruling, with no target: the entry
  stays open and returns to the human when the human raises it. Nothing is built; the lab's Polaroid medium stays a
  medium (R-486).
- **25 Oct 2026 (R-625):** the human: "Parked (RQ-378, RQ-386, RQ-391): stay parked." The entry stays open and parked (R-527); what it applies stays applied, and nothing waits.

## RQ-570: Table 1's column 8 (E·10⁻⁷) of Agekyan & Anosova (1968) does not equal eq. (4) on columns 2 and 3 *(found by TASK-M2-20; REQ-VAL-019; context: TASK-M2-20, the RQ-566 ruling's item 5)*

*Filed as the RQ-566 ruling's item 5 asks. Physics, for the human; not resolved here. TASK-M2-20 records column 8
without asserting it until this is ruled.*

- **File, section:** `docs/incoming/2026-10-24_anosova/papers/agekyan_anosova_1968_SvA_11_1006.pdf`, p. 1007 eq. (4),
  "E = −G(1 + 1/√((ξ+½)²+η²) + 1/√((ξ−½)²+η²)) + L₀", and Table 1, pp. 1010–1012, column 8 "E·10⁻⁷"; the RQ-566
  ruling (24 Oct 2026), item 5: "Table 1's column 8 (E·1e-7) does not equal eq. (4) evaluated on columns 2–3."
- **The comparison, all 100 rows** (G = 6.67·10⁻⁸, L₀ = K₀ = 7.5·10⁻⁹, m = 1; the 62 rows that print a value): the
  ratio column 8 / eq. (4) runs from 0.029 (N = 24: eq. (4) −7.228, printed −0.207) to 1.571 (N = 26: −2.056,
  printed −3.231), wider than the ruling's 0.23–1.25, which came from a sample. N = 1: −2.464 against −2.254; N = 3:
  −2.589 against −2.041; N = 10: −3.858 against −0.887, as the ruling found. The 38 rows that print "—" have nothing to
  compare. Every row's figures are in `fixtures/anosova/col8_comparison.csv`, committed beside the fixture.
- **What paper II says** (`agekyan_anosova_1968_Ap_4_11_part2.pdf`, p. 11 and Table 1, pp. 12–14): the same 100
  configurations ("exactly the same as in [1]"), units "the same as in [1]", started at exact rest; its column 6 is
  "the total energy E (with minus sign)". Its values sit K₀ = 0.075·10⁻⁷ from paper I's column 8, not from eq. (4):
  N = 1, II 2.329, I −2.254; N = 3, II 2.116, I −2.041 (both differ by exactly 0.075); N = 10, II 0.971, I −0.887
  (0.084). Eq. (4) at rest gives −2.539, −2.664 and −3.933. So both papers print one energy that is not eq. (4) on the
  printed (ξ, η): a scan or OCR error in one paper alone does not explain it. II's Table 3 gives −E·10⁻⁷ for the 38
  rows I left at "—".
- **Possible causes:** (a) column 8 (and II's column 6) is a different E from eq. (4): another configuration than
  the printed (ξ, η), another normalisation, or another quantity; (b) a print error common to both tables (unlikely,
  since II was set separately and agrees with I to K₀); (c) the printed (ξ, η) are not the configurations the
  energies were computed from, though column 11, printed on 62 rows, matches η·10⁻⁴ on 57 (RQ-571).
- **Also recorded here, as dispatched:** N = 17's ξ in paper I has a broken fifth decimal. Transcription A read
  0.115219340 (a damaged "1", flagged uncertain: "it could be a '7'"), B 0.115279340: the one difference between the
  two transcriptions. At 600 dpi the glyph is a slanted stem with no top bar, the slant of row 16's "7", while every
  "1" in the cell is upright with a flat foot, so it is read "7"; paper II prints 0.115279340 cleanly. The fixture
  carries 0.115279340 and both readings.
- **Applied meanwhile:** column 8 is transcribed and recorded, never asserted.
- **Options seen:** (a) column 8 stays recorded and unasserted at M2, and the Anosova chart's milestone decides; (b)
  the human identifies column 8's definition and it is asserted as that.

## RQ-575: Under R-600's "all 256 starts, not a subset", how a start whose pair has not parted by the horizon enters the ensemble ratio's median *(physics; found by R-600's port; R-600, R-574, R-610; REQ-VAL-199; context: TASK-M8-76)*

*Found by R-600's port. For the human: yes (physics: it changes what the gate passes).*

- **File, section:**
  - The human, 25 Oct 2026 (R-600, `docs/incoming/2026-10-25_rulings/rulings_2026-10-25.md`): "Predict each start's
    parting time from its measured initial gap, ln(tol/δ0) for each precision, rather than from ln ε32 / ln ε64. Run it
    on all 256 starts, not a subset. The tolerance is a calibration for the gate."
  - RQ-557's options, as archived: "Under any: the ICs from the chaotic regions, at `T = 200`, over the ICs where both
    pairs part, the count reported."
  - `docs/incoming/2026-10-21_gap_sweep_results_honesty/evidence/parting_probe_output.txt`, "deep interior", at
    `θ = 0.1`: f64 vs f64′, **19 %** parted by 50.
  - R-610: `T_horizon ∈ [0.4, 200]`, the cap 200 until long horizons are measured for cost.
- **The question:** the ratio `t_part(f32)/t_part(f64)` needs both parting times; a start whose f64–f64′ pair (or
  f32–f64 pair) has not parted by the horizon has no `t_part`, only a bound `t_part > T`. "All 256, not a subset"
  rules out dropping such starts, and the probe shows many of them at `T = 50`; nothing says how they enter.
- **Options seen:**
  - (a) Censored: an unparted pair's `t_part` is taken as `T`, the ratio a bound, the median computed over all 256 and
    the count of censored starts reported; the gate valid only if fewer than half are censored.
  - (b) The gate runs at `T = 200`, and a start still unparted there fails the gate's precondition (all 256 must part),
    the starts chosen from the chaotic regions so that they do.
  - (c) The comparison per start, measured against predicted, a censored start counted as agreeing when its predicted
    `t_part` also exceeds `T`.
- **Applied meanwhile:** none. REQ-VAL-199 carries `rq: RQ-575` for this rule only.
- **Waits:** REQ-VAL-199's gate (TASK-M8-76).
- **Second question (physics review 5644374613 of PR #293, F3; added on R-600's port, the text above unchanged):**
  which statistic the gate compares. The prediction is per start, but `principia_honesty_precision.md` §6
  (ensemble_ratio) says "the median measured `t_part(f32)/t_part(f64)` compared with the predicted one", REQ-VAL-199's
  statement "the median … against its expected value" and its verify "the measured ratios against the predicted". Two
  readings fit, and they pass different ensembles.
  - **Options seen:** (i) the median of the measured ratios against the median of the predicted ratios, the tolerance
    on their difference; (ii) the median, over the starts, of each start's measured ratio divided by its predicted
    ratio, against 1, the tolerance on its distance from 1.
  - **Applied meanwhile:** none; REQ-VAL-199's `rq: RQ-575` covers this rule too.

## RQ-576: Under which equivalence the computed free-group word is compared with the published one: the Belgrade gallery and the 2013 PRL write the same orbits' words with different representatives *(definition, R-72; the human's queued question on the 26 Oct incoming record; R-125; REQ-VAL-043, REQ-PAY-070; context: TASK-M3-30, TASK-M3-16, TASK-M8-35)*

*Queued by the human, 27 Oct 2026, on `docs/incoming/2026-10-26_belgrade_gallery/` (PR #296): "So the reader's
comparison must declare its equivalence: free and cyclic reduction, plus which symmetries count. That is a definition
requirement (R-72): queue it in REVIEW_QUEUE.md with the crosscheck.txt numbers. Don't pick it yourself." The options
are listed with no choice made.*

- **File, section:**
  - `decisions.md` § "R-125 — The branch-cut convention is M3's, transcribed from the literature *(closes RQ-96)*": "Its
    convention (the a/b branch-cut assignment and the crossing sign) is transcribed from the literature (Montgomery;
    Šuvakov–Dmitrašinović) and verified by REQ-VAL-043 against the published braid classes."
  - `plan/requirements.yaml` REQ-VAL-043: "A word-calculus test must assert the free-group word encoder reproduces the
    published braid class of Šuvakov–Dmitrašinović orbits."; verify: "for each gallery orbit used, computed word (under
    REQ-PAY-070's transcribed convention) equals the published class". TASK-M3-30's acceptance line:
    "`cargo test -p validation word_braid_class` — for each gallery orbit used, the computed word, under REQ-PAY-070's
    transcribed convention, equals the published class (REQ-VAL-043)."
  - `docs/design/principia_dd_validation_orbits.md` § "1.4 Šuvakov–Dmitrašinović (13 families, 2013) and
    Broucke–Hénon–Hadjidemetriou": "a known orbit has a known word, and if the encoder produces a different one, that is
    a bug with a named right answer."
  - `docs/incoming/2026-10-26_belgrade_gallery/crosscheck.txt` § "3. Šuvakov & Dmitrašinović 2013, Table 1", and the
    record's `README.md` § "The cross-check (`crosscheck.txt`)", item 3: "**Their words agree only up to a symmetry, and
    which symmetry varies by orbit**"; "So the gallery and the PRL write the same conjugacy classes with different
    representatives. The encounter-word reader (R-125) must declare which equivalence it compares under, and a comparison
    up to rotation alone would fail 6 of these 15 for a reason that is not an error."
- **The numbers (`crosscheck.txt` § 3).** `crosscheck.py` freely and cyclically reduces each PRL word, then lists every
  variant of it whose cyclic rotations contain the gallery's word: `cyclic` (the word itself), `inverse` (reversed and
  each letter inverted), `reverse` (reversed), `a<->b`, and `(a,b)<->(A,B)`. Of the 15 orbits of the PRL's Table 1
  (`p1`, `p2`, `T` agreeing within the PRL's printed precision, largest difference 4.1e-5):
  - **9 match up to cyclic rotation alone:** butterfly I (`cyclic, (a,b)<->(A,B)`), butterfly II (`cyclic,
    (a,b)<->(A,B)`), bumblebee (`cyclic, (a,b)<->(A,B)`), moth II (`cyclic, reverse, a<->b`), moth III (`cyclic,
    reverse, a<->b`, after reduction, below), butterfly IV (`cyclic, reverse, a<->b`), yarn (`cyclic, reverse, a<->b`),
    yin-yang 2a (`cyclic`), yin-yang 2b (`cyclic`).
  - **6 match only under a symmetry:** moth I (`inverse, (a,b)<->(A,B)`; PRL `ba(BAB)ab(ABA)`, gallery `abaBAbabAB`),
    butterfly III (`inverse, (a,b)<->(A,B)`; PRL `(ab)2(ABA)(ba)2(BAB)`, gallery `ABabaBABAbabAB`), goggles (`inverse,
    (a,b)<->(A,B)`; PRL `(ab)2ABBA(ba)2BAAB`, gallery `abbaBABAbaabABAB`), dragonfly (`inverse, (a,b)<->(A,B)`; PRL
    `(b2(ABabAB))(a2(BAbaBA))`, gallery `abABabAAbaBAbaBB`), and, under the exchange only, yin-yang 1a and yin-yang 1b
    (`(a,b)<->(A,B)`; PRL `(ab)2(ABA)ba(BAB)` for both, gallery `AbabABABabaB` and `ABabaBAbabAB`).
  - **Moth III's published word is not freely reduced:** `(babABA)²a(abaBAB)²b` contains `A·a`, 26 letters against the
    gallery's 22 (`AbabABAbabABabaBABabaB`); after free and cyclic reduction it matches up to rotation.
  - The PRL words above are as `crosscheck.py`'s table `T1` transcribes them from the PRL (a record, not checked here).
- **The question:** REQ-VAL-043's "equals the published class" does not say under which equivalence a computed word
  equals a published one. Which reduction is applied to each side (free reduction; cyclic reduction), and which
  symmetries count as the same class: none (cyclic rotation only), the inverse word, the reversed word, the exchange
  `a ↔ b`, the exchange `(a, b) ↔ (A, B)`, or a set of these. A second part: whether the encoder's own word (REQ-PAY-070's
  transcribed convention, not yet written) is compared with the gallery's representatives or the PRL's, which differ by
  the symmetries above, and whether the published side is the gallery, the PRL, or both.
- **Options seen** (none chosen, as the human asked):
  - (a) free and cyclic reduction of both words, then equality up to cyclic rotation only (the conjugacy class); 6 of
    the 15 fail against the PRL's words, none against a source written in the encoder's own convention.
  - (b) (a), plus the inverse word counted as the same class.
  - (c) (a), plus the exchange `(a, b) ↔ (A, B)` counted as the same class.
  - (d) (a), plus both the inverse and the exchange: every one of the 15 matches.
  - (e) (a), plus every variant `crosscheck.py` lists (inverse, reverse, `a ↔ b`, `(a, b) ↔ (A, B)`) and their
    compositions.
  - (f) a per-orbit declaration: each reference row names the symmetry under which its published word relates to the
    encoder's, and the comparison applies exactly that one.
  - (g) other.
- **Recorded, not resolved (the human: "Record, don't resolve"):**
  - **Two periods for butterflies I and II.** The record's `README.md` § "One internal discrepancy, recorded": "the main
    page has the PRL's 6.235641 and 7.003907, the sequence pages (I.2.A, I.2.B) have 6.234671 and 7.003707, with the same
    `p1`, `p2` and energy. The sequence values agree with the thesis's scale-invariant `T|E|^{3/2}` (19.9325 and 19.9313,
    Table 3) and with the Three Body Orbits atlas's refined periods (6.23489 at E = −2.170144, 7.00370 at E = −2.008195),
    so the PRL's are the outliers, at about 1e-4." Both values are in `belgrade_gallery.csv` as the pages give them.
  - **Five thesis labels renamed later** (`crosscheck.txt` § 4; `README.md` item 4): thesis IVa.6.A, IVa.8.A, IVa.8.B,
    IVa.8.C and IVb.15.A are gallery IVa.14.C, IVa.8.F, IVa.8.G, IVa.8.H and IVb.15.D (matching to 1.0e-6, 1.5e-6,
    1.7e-6, 2.0e-6 and 5.1e-7), and thesis V.4.A is gallery V.4.A.α (3.5e-6). The human: "the gallery's labels are
    current"; the README: "The later labels are the gallery's and the 2019 table's; the thesis's are earlier."
- **The reference, as the human named it:** "belgrade_gallery.csv is the reference for the encounter-word reader
  (R-125), together with Hudomal 2015 and the 2013 and 2018 papers. Transcribe twice, as usual." The sources:
  `docs/incoming/2026-10-26_belgrade_gallery/belgrade_gallery.csv` (sha256 `3558b26b…a839335b`); A. Hudomal's 2015
  thesis; Šuvakov & Dmitrašinović, *Phys. Rev. Lett.* 110, 114301 (2013)
  (`docs/incoming/2026-10-25_replication/papers/suvakov_dmitrasinovic_2013_PRL_110_114301_arXiv1303.0181.pdf`); and
  Dmitrašinović, Hudomal, Shibayama & Sugita, *J. Phys. A* 51, 315101 (2018)
  (`…/papers/dmitrasinovic_etal_2018_JPhysA_51_315101_arXiv1705.03728.pdf`). The 2019 table
  (`archive/initial-conditions.pdf`) agrees with all 206 sequence orbits to every printed digit (`crosscheck.txt` § 1).
  **Credit line**, from the gallery's own terms (`archive/ref.php.html`: "You may copy and use any images or data from
  this web gallery. Please, credit us by citing this page and our paper [8]."): every use cites
  `http://three-body.ipb.ac.rs/` and *Phys. Rev. Lett.* 110, 114301 (2013).
- **Unclear in the records, said here, not resolved:**
  - The "encounter-word reader" is the human's and the record's term; the corpus's are R-125's convention
    (REQ-PAY-070, TASK-M3-16) and REQ-VAL-043's "free-group word encoder" (TASK-M3-30). This entry reads them as the
    same comparison.
  - The Hudomal 2015 MSc thesis is not on `main` yet: it arrives with open PR #295 (`docs/orbit-catalogues`) as
    `docs/incoming/2026-10-26_orbit_catalogues/hudomal_2015_msc_belgrade.pdf`. `crosscheck.txt`'s command takes it as a
    path argument (`<Hudomal 2015 thesis>`).
  - R-125 says the convention is "transcribed from the literature (Montgomery; Šuvakov–Dmitrašinović)"; the gallery and
    the PRL, both Šuvakov–Dmitrašinović, already differ by the symmetries above, so which of their representatives
    REQ-PAY-070's transcription follows is part of the question.
- **Applied meanwhile:** none; TASK-M3-30 and TASK-M3-16 are not built. No requirement is added or changed by this
  entry (the human's instruction for this PR is the queue only); the definition requirement R-72 asks for, and
  REQ-VAL-043's `rq: RQ-576`, are the port's next step.
- **Waits:** REQ-VAL-043's comparison (TASK-M3-30, `word_braid_class`). The rest of TASK-M3-30, and TASK-M3-16's
  convention, do not wait on the equivalence.

## RQ-577: The precision-ladder proposal's seven open questions (§6), and the amendments it proposes to R-265, R-572, R-90, canonical spec §1 item 3, deep zoom §2 and its layer table, and integrator contract Part 2 and Part 2b *(the human's queued questions on the 26 Oct incoming proposal; R-265, R-572, R-90, R-561, R-547, R-33; context: none built)*

*Queued by the human, 27 Oct 2026, on `docs/incoming/2026-10-26_precision_ladder/` (PR #297): "A PROPOSAL. Nothing in
it is in force and nothing in it is built until I rule on its §6." and "Do not amend R-265, R-572, R-90, the canonical
spec, deep_zoom or the integrator contract from it." **Nothing in the proposal is in force and nothing in it is built
until the human rules on its §6.** For the human: yes, every question changes what is built. No option is recommended
here.*

- **File, section:** `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md`
  (sha256 `57eca498…852b192d7`), § "6. Rulings touched and open questions"; its design in § "3. The design" (3.1 to
  3.10), its build order in § "4. Build order" and its twelve tests in § "5. Tests (each with what makes it fail)". Its
  header: "**Draft design note, 26 Oct 2026. Not authority.** It proposes changes to rulings in force (R-265, R-572,
  canonical spec §1 item 3, R-90). Nothing here is built until the human rules on §6."
- **The amendments it proposes** (§6, "Rulings it would amend", quoted):
  - **Canonical spec §1 item 3:** "**Canonical spec §1 item 3**, "The GPU is f32-locked". Proposed reading: the GPU's
    native arithmetic is f32 (and f64 where the hardware offers it); extended precision on the GPU is emulated, opt-in,
    and guarded by the canary." In force: `docs/contracts/principia_canonical_spec.md` § "1. The substrate (the
    defining decision)", item 3: "**The GPU is f32-locked** (WebGPU has no f64)."
  - **R-265:** "**R-265**, the kernel is f32 and f64 only and `Dd` is parked. Proposed: the kernel's `Real` gains `df32`,
    `qf32`, `dd64` and `mp(bits)`. R-265's condition for unparking has two clauses, "a precision question needs it and
    Brutus is too slow". The case made here is the Burrau neighbourhood mapped past t ≈ 52 (§2), where a per-pixel Brutus
    run is too slow; the single trajectory does not meet the second clause." In force: `decisions.md` § "R-265 — The
    shared kernel is f32 and f64 only; double-double is parked *(closes RQ-161, amends R-33)*".
  - **R-572:** "**R-572**, "f32 or f64 only" in a Result. Proposed: the ladder, with f64 still the default."; and "**R-572
    and results §3.2**: a Result's uniform grid is a complete tree at one level, run in quads rather than rows (§3.10)."
  - **Integrator contract Part 2 and Part 2b:** "**Integrator contract Part 2 and Part 2b**: Taylor joins the stepper
    axis; precision becomes a third axis; R-561's warn-never-block extends to it." (`docs/contracts/principia_integrator_contract.md`
    § "Part 2 — Occupants and the capability profile" and § "Part 2b — Regularisation is a SECOND swappable axis, not a
    property of the stepper".)
  - **R-90:** "**R-90**, `ℓ_switch = 20` as a fixed bound: per precision (§3.9)." Its proposed rule (§3.9): "`ℓ_switch`
    becomes a per-precision backstop, `ℓ_switch(P)`, never below `b(P)/2`, and is a frozen value of the sim key (Part 2c
    rule 1)."; and "`AT_F32_FLOOR` generalises to `AT_PRECISION_FLOOR(P)`."
  - **Deep zoom §2:** "**Deep zoom §2**, "Floor moves from ~23 to ~50+" with an f32 sum on the GPU: contradicted by the
    prin-rs measurement (§3.9 item 4). This one stands on its own and goes to `REVIEW_QUEUE.md` even if the ladder is not
    adopted." That finding is RQ-578, queued separately as the human asked.
  - **Deep zoom's layer table:** "**Deep zoom's layer table**: the linearised decoder belongs to depth, not to layer 2's
    adaptivity (§3.10)." In force: `docs/design/principia_deep_zoom.md` § "3. Three-layer quadtree", layer 2: "CPU→GPU
    adds the linearised-decoder uniforms".
  - Held, not amended, by the proposal's own account: "**Integrator contract Part 2c** is held, not amended"; "**R-547**
    governs the emulated arithmetic's branch inputs and forbids an FMA-based TwoProd on this project's backends (§3.5)";
    "**R-33** is unchanged; §3.8 states why."
- **The seven open questions** (§6, "Open questions for the human", verbatim):
  1. "Unpark extended precision now, or only after M3 (the current integrator milestone) closes?"
  2. "Is the live view allowed the ladder by choice (§3.6), or is it Result-only?"
  3. "Taylor's default order rule: Jorba & Zou's, or a fixed order per tier?"
  4. "What does the profile's `force_evals` mean for Taylor: a measured equivalent, or a new field?"
  5. "Event location by dense output (§3.4): adopt it for Taylor only, or keep events where every stepper samples them,
     so steppers stay comparable?"
  6. "Should a Result above f64 be able to store raw extended state, or rounded fields only?"
  7. "Per-quad precision in the tree: may a deep quad run at a higher rung than its parent, so that only the deep part of
     a view pays for it? It costs the scheduler a precision field per quad and makes neighbouring quads differ in
     arithmetic, which R-547's determinism rules would have to cover."
- **Beside the seven, implied by the amendments:** whether the ladder is adopted at all (the amendments above, each
  accepted, changed or refused), and whether uniform mode becomes "a complete tree at one level" (§3.10), which the
  proposal lists as an amendment but not as a question.
- **Unclear in the record, said here, not resolved:**
  - The proposal's header names four rulings it changes (R-265, R-572, canonical spec §1 item 3, R-90); its README and
    §6 add deep zoom §2 and its layer table, integrator contract Part 2 and Part 2b, and §6 adds results §3.2
    (`docs/design/principia_results.md` § "3.2 Where it runs: the job"). This entry lists every one §6 names.
  - Test 6 (§5) reads `atlas-params.json`, which "**is held outside the repo** until its data terms are confirmed
    (`docs/incoming/2026-10-26_orbit_catalogues/`, unmerged)".
  - §7's sources are "not yet checked against the papers".
- **Applied meanwhile:** none. R-265, R-572, R-90, the canonical spec, deep_zoom and the integrator contract stand as
  written; no requirement or task is added from the proposal.
- **Waits:** nothing built waits; no task builds the ladder. RQ-578 (deep zoom §2's floor) does not wait on this entry.

## RQ-578: Deep zoom §2's "Floor moves from ~23 to ~50+" does not hold for an f32 sum on the GPU: the ICs are absolute O(1) numbers, so the integration's precision sets the floor *(conflict: deep_zoom §2 and §4 (iv), scheduler_contract Part 3 and Part 4, dd_refinement_policy §0 against the prin-rs measurement the precision-ladder proposal reports; R-90, R-265, canonical_spec §1 item 3; REQ-DEC-036, REQ-TOOL-159, REQ-SCHED-062; context: TASK-M5-04, TASK-M6-07, TASK-M6-08, TASK-M6-02, TASK-M6-21, TASK-M6-01, TASK-M8-57)*

*Queued by the human, 27 Oct 2026: "One finding stands on its own, ladder or not: deep_zoom §2's "Floor moves from ~23
to ~50+" does not hold for an f32 sum on the GPU. The ICs are absolute O(1) numbers, so the integration precision sets
the floor (proposal §3.9 item 4; prin-rs measured x0 + J·δ all in f32 collapsing with plain f32). Queue it separately
in REVIEW_QUEUE.md, with file, section and quoted text. It is not RQ-554. It affects deep-zoom tasks now, so flag which
tasks depend on that floor." For the human: yes, it sets what depth the deep-zoom gates assert and what the linearised
decoder is for. It is not RQ-554, which is the band quad's terminal stop at `AT_F32_FLOOR` on a saturated axis; this
entry is where that floor sits at all on the f32 path.*

- **File, section (the claim):**
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision": "**Why the precision survives.** The
    nonlinear precision lives in `x₀` (computed in f64, so its f32 cast is the best possible representation of the true
    centre IC). `J_D` is *rates of change* — `O(1)`, no precision lost in f32. Floor moves from ~23 to **~50+**."; and
    "**GPU kernel (f32):** `x = x₀ + J_D · δ`, where `δ` is the quad-local UV offset."
  - `docs/design/principia_deep_zoom.md` § "4. Sliding depth bound": "(iv) the **decode floor proper** (`AT_F32_FLOOR`) —
    only if the *linearised* decoder's samples collapse (~depth 50+), genuinely terminal, a deep-zoom backstop".
  - `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)", its table: "GPU
    (kernel) | f32 | quad-local sample positions, IC via `x₀ + J_D·δ`, integration".
  - `docs/contracts/principia_scheduler_contract.md` § "The absolute floor is emergent": "the **decode floor**
    (`AT_F32_FLOOR`, pushed to ~depth 50 by the linearised decoder)"; § "Part 4 — Terminal vs refinable, tied to the two
    floors": "Refinement **continues** on the linear path (depth ~23 → ~50)." and "Only when the *linearised* decoder's
    samples collapse to bitwise-identical ICs (~depth 50+) is f32 genuinely exhausted — the quad-local offsets `δ` are
    too small for f32 to represent distinct neighbours".
  - `docs/design/principia_dd_refinement_policy.md` § "0. What the mechanism is actually for", item 2: "The adaptive tree
    plus the linearised decoder (`principia_deep_zoom.md`) reaches depth ~50. **This is the feature; the tree exists for
    it.**"
  - `plan/requirements.yaml` REQ-DEC-036, verify: "the linear path distinguishes adjacent samples to depth ≥ 50".
- **File, section (the finding):** `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md`
  § "3.9 Deep zoom: the decoder switchover depends on the precision", item 4: "**The floor that matters is the
  integration's precision, not the decoder's.** The ICs are absolute numbers of order 1, and the integrator starts from
  them. If two pixels' ICs differ by less than `eps(P)` relative to O(1), they are the same IC to an integrator at
  precision `P`, whatever decoder produced them. prin-rs measured exactly this: "the literal formula `x₀ + J_D·δ` all in
  f32 collapses on exactly the same curve as plain f32", and the linearised split bought ~24 levels only by summing in
  f64 ("~24 levels over f32 and none over f64"). So deep zoom §2's "~50+" holds only if the sum and the integration run
  at f64. On the GPU at f32 the floor stays near 16–23 with either decoder. **That looks like a conflict in the corpus as
  it stands and should go to `REVIEW_QUEUE.md`** with this measurement, whether or not the ladder is adopted. (It is not
  RQ-554, which is a different issue.)"
- **The prin-rs measurement, as the proposal reports it** (§3.9; prin-rs is not authority, and the measurement is quoted
  from the proposal only): "the literal formula `x₀ + J_D·δ` all in f32 collapses on exactly the same curve as plain
  f32"; the linearised split "bought ~24 levels only by summing in f64 ("~24 levels over f32 and none over f64")";
  item 1: "prin-rs measured f32 collapsing at 16–23 and every f64 path at exactly rung 48"; its floor table: "f32 |
  ~16–23 (measured)", "f64 | ~50 (measured 48)"; and "At the chart origin there is no such floor (the standing prin-rs
  result: "the deep-zoom floor is a property of where you zoom")."
- **The conflict:** the corpus places the precision loss in `δ` ("the quad-local offsets `δ` are too small for f32 to
  represent distinct neighbours") and the floor at ~50+; but `x = x₀ + J_D·δ` is an absolute f32 number of order
  `|x₀|`, and the integrator, f32 on the GPU (canonical spec §1 item 3, R-265), starts from it. Adjacent ICs that differ
  by less than f32's ulp at `|x₀|` round to the same f32 IC whichever decoder produced them, so on the GPU at O(1)
  coordinates the linear path's samples collapse where the full decoder's do, ~16–23 by the measurement, and
  `AT_F32_FLOOR` fires a few levels past the switchover, not at ~50+. Deep zoom §1 already says "The deep-zoom floor is a
  property of where you zoom: a floor depth is quoted with the coordinate magnitude"; §2's and the scheduler contract's
  "~50+" quote none. The deep-zoom note's § "Reserved: the integration floor" covers ICs "*distinct* in physical
  coordinates" whose integration error exceeds their difference; this case is ICs not distinct at f32 at all.
- **Options seen:**
  - (a) The floor is restated: on the f32 GPU path the linearised decoder's floor is about the full decoder's (~16–23
    at O(1) coordinates, quoted with the coordinate magnitude); the linear path is kept for speed and for the
    f64-anchored decode (deep zoom § "Couplings to the contract", c); the ~50+ text above and REQ-DEC-036's and test
    11's "depth ≥ 50" are rewritten to the measured floor.
  - (b) The ~50+ floor is kept by running the sum and the integration past the switchover at a precision above f32:
    on the CPU at f64 for deep quads, or emulated on the GPU (the proposal's `df32`, which is RQ-577's ladder and needs
    R-265 and canonical spec §1 item 3 amended).
  - (c) Measured first: a gate measures the f32 GPU path's distinguishable depth, as integrated, at stated coordinate
    magnitudes (as the proposal's test 12's f32 arm, "it must collapse at 16–23"), and the docs and the gates take the
    measured value (a calibration, R-71).
  - (d) Other.
- **Applied meanwhile:** none; the docs and requirements stand as written. This PR adds the queue entry only; the
  requirements below do not yet carry `rq: RQ-578`, which is the port's next step.
- **Waits** (the arms that assert the ~50+ floor, or need distinct f32 ICs past ~23 at O(1) coordinates):
  - TASK-M5-04: `cargo xtask gate linear-decode-depth`'s arm "the linear path distinguishes adjacent samples to depth ≥
    50" (REQ-DEC-036's verify). Its other arms (the O(h²) agreement, no nonlinear decode on the GPU) and the rest of
    TASK-M5-04 do not wait.
  - TASK-M6-07: decoder test 11's "depth ≥ 50 distinguishability" (`cargo xtask gate linear-decode`, its Deliverables).
    Test 11's O(h²) slope (REQ-DEC-034, REQ-DEC-042) does not wait.
  - TASK-M6-08: `cargo test -p engine past_switch_routing` (REQ-TOOL-159), "quads at depths 21 and 30 with distinct
    adjacent ICs (no collapse)", a fixture that exists at O(1) coordinates only if the floor is past 30; and where
    REQ-SCHED-062's `AT_F32_FLOOR` (the linear path's collapse, "~depth 50+") fires. The switchover itself (REQ-DEC-033,
    REQ-DEC-037) does not wait.
- **Read the floor, do not wait** (their tests hold wherever the floor falls; what they show moves with it):
  - TASK-M6-02: REQ-SCHED-057's `infinite_zoom_no_absolute_cap`, "drive a scripted zoom to depth > 30 … until a floor
    fires": past 30 only where no floor fires first, so the fixture's coordinates decide whether it reaches 30.
  - TASK-M6-21: REQ-GUI-010 and REQ-GUI-012, the precision warning at `AT_F32_FLOOR` ("zoom a visible region through the
    switchover and to the linear floor"); under the measurement it fires a few levels after the switchover.
  - TASK-M6-01: REQ-REF-011 and REQ-SCHED-064, `AT_F32_FLOOR` as a terminal floor.
  - TASK-M8-57: REQ-TOOL-182's T10 arm, "adjacent samples give distinct ICs to the same depth as at the root", and
    REQ-TOOL-183's tolerance, the floor quoted with `|x₀|`.
  - The proposal's §3.10 (uniform mode, the layer table) and §3.9's per-precision switchover are RQ-577's, not this
    entry's.

## RQ-579: Redistribution terms for the high-precision IC databases *(found while pulling the reference data for the precision ladder's test 6, PR #299; `docs/incoming/2026-10-27_hp_ics/README.md` § "Terms: why the data is not committed"; context: the precision-ladder proposal, §5 test 6)*

*Found on 27 Oct 2026. Licensing third-party data is outside the repo, so it is the human's call. Nothing is
committed meanwhile.*

- **File, section:**
  - `docs/incoming/2026-10-27_hp_ics/README.md` § "Terms: why the data is not committed": "None of the sources states
    a licence or terms of use: the five pages of `db2.fmi.uni-sofia.bg` (its root, `3body123`, `3bodychor462`,
    `3bodychor164`, `3bodystab971`), Masson's page and site, the SJTU pages, and the `sjtu-liao/three-body` GitHub
    repository (no licence file; the API returns 404 for it). The three Hristov arXiv papers carry arXiv's
    non-exclusive distribution licence, which covers the papers, not the databases."
  - The precedent: `docs/incoming/2026-10-26_orbit_catalogues/README.md`: "**The extracted initial conditions are
    committed under the site's own terms.**"; `docs/incoming/2026-10-26_belgrade_gallery/README.md`: "**Terms.** The
    gallery's own "Use and Copyright" page (`archive/ref.php.html`): "You may copy and use any images or data from
    this web gallery. …"". Both folders committed third-party data only under stated terms.
  - `docs/incoming/2026-10-26_precision_ladder/PROPOSAL_precision_ladder_and_taylor_2026-10-26.md` § 5, test 6, which
    needs the values above f64.
- **The question:** the Hristov et al. databases (123 solutions at 150 digits, 462 and 164 + 259 choreographies at
  180, 971 stable i.c.s at 100), and the Simó and Li & Liao tables, state no terms. May their values be committed?
- **Options seen:** (a) commit the values with attribution (the citations in `docs/incoming/2026-10-27_hp_ics/README.md`);
  (b) ask the authors for permission first, and commit under whatever they grant; (c) keep fetch-and-verify only, as
  now.
- **Applied meanwhile:** (c). `fetch.sh` downloads all 599 files listed in `sources.tsv` and checks each against its
  recorded sha256, so the reference data is reproducible byte for byte without being committed. Waits: the precision
  proposal's test 6 reference data, in the repo.

## RQ-581: The Cartesian drift latches `dE_max` and `dLz_max` are linear binary16, whose floor sits above the f64 path's drift *(physics; found by physics review 5662346996 of PR #293, a non-blocking note on F1; payload §1, §2; REQ-PAY-012, REQ-PAY-041, REQ-PAY-044, REQ-TOOL-023; context: none in this PR)*

*Found by physics review 5662346996 of PR #293, outside its diff. For the human: yes (physics: the encoding sets what
the drift view and the drift fields can show).*

- **File, section:**
  - `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct", the field:
    "`packed_b : u32,               // dE_max:f16[low] | dLz_max:f16[high]`".
  - The same file, § "Why closure is here, at this width, unconditionally", "**Why f32 and not f16.**": "f16 cannot
    hold that linearly — its smallest subnormal is 6.0e-8 with ~1 digit, and a closure of 1e-9 **flushes to zero**,
    making every periodic orbit indistinguishable from a merely quiet region. … because it does not truncate the **f64
    CPU path**, where closure genuinely reaches ~1e-16."
  - `docs/contracts/principia_integrator_contract.md` § "Part 1 — The shape: a swappable `step()` slot inside a fixed
    wrapper": "the regularised energy stays at round-off (prin-rs `HgOut::drift` against `drift_reg`)".
  - The same payload file, the precision rule: "**f16, safe — but they are MONOTONE LATCHES** … `dE_max = max(dE_max,
    |ΔE|)`, `dLz_max = max(dLz_max, |ΔLz|)` … **threshold/suspect decisions must use the live f32 measurement, never
    the unpacked f16 latch** (the f16 latch is for display, not for control flow)."
- **The question:** on the f64 CPU path (and wherever a good integrator holds the Cartesian drift near round-off), a
  drift below about 6e-8 stores as zero or with about one digit, so quiet runs and mildly drifting ones read alike in
  the stored latches. The precision rule keeps control flow on the live value, so no decision is wrong; what is lost
  is what the stored latch, and anything that reads it at rest, can show. Is linear binary16 the right encoding for
  `dE_max` and `dLz_max`?
- **Options seen:**
  - (a) Keep linear binary16, display only, as the precision rule has it, and state its floor where the latches are
    shown (the field views, REQ-TOOL-160's log floor).
  - (b) A log-encoded 16-bit form in the same bits, so `packed_b` keeps its size; the field views and REQ-TOOL-023's
    round-trip change with it.
  - (c) The latches follow `Real` (f32 at f32, f64 at f64), a width change with its own REQ-PAY-009 recheck at every
    precision row.
  - (d) The same encoding as `dE_reg`'s halves, which TASK-M3-41 chooses against REQ-INT-106's measured drift
    (REQ-PAY-111; physics review 5662346996 of PR #293, F1).
- **Applied meanwhile:** none; `packed_b` stays as built (TASK-M0-09, TASK-M0-10).
- **Waits:** nothing in PR #293 waits on it. Raised as a physics question for the human (R-369); no requirement
  carries it yet.

## RQ-605: A weekly whole-corpus consistency audit by our own agents, proposed and not built *(the owner's request of 28 Oct 2026, item 6; R-635; context: none built)*

*Not built; for the owner. Proposed as the owner asked: "A queue entry (not built yet) proposing a weekly whole-corpus
consistency audit by our own agents: it reads plan/, docs/design, docs/contracts, decisions.md and the requirements;
checks cross-references resolve; finds contradictions between documents; and lists requirements with no task and tasks
with no requirement. It reports to me as one PR of fixes or queue entries."*

- **File, section:** `decisions.md` § "R-635 — …", which keeps whole-corpus consistency out of the AI reviewers'
  brief (`AGENTS.md` § "Code Review Rules" checks only what a PR touches).
- **What it reads:** `plan/` (`requirements.yaml`, `tasks.yaml`, `plan/tasks/`, `MILESTONES.md`, `WORKFLOW.md`,
  `OPERATIONS.md`, `reviewers/`), `docs/design/`, `docs/contracts/` and `decisions.md`. Not `docs/incoming/`,
  `docs/archive/`, `docs/reference/` or `workbench/`, which are records, not authority (R-1, R-159).
- **What it checks:**
  - every cross-reference resolves: a file and § heading, an R-, RQ-, REQ- or TASK- id, a contract Part;
  - contradictions between documents: two statements of one rule, value or definition that disagree, and a
    ruling's text against the doc lines it says it was applied in;
  - requirements with no task, and tasks with no requirement.
- **Its output:** one PR to the owner a week, holding fixes for what is mechanical (a stale reference, a heading
  renamed) and a `REVIEW_QUEUE.md` entry for each contradiction, with file, section and quoted text, as any queue
  entry. Nothing is merged without its reviews and green CI.
- **The questions:** whether to build it; on what schedule and which machine (a cloud session suits read-and-think
  work, R-357); and which agent role runs it and which reviewers its PR names.
- **Overlap seen:** `python3 plan/check_plan.py` already fails on unresolved R- and RQ- references in the live files,
  on references to missing files and sections in the plan, and on live requirements with no task and tasks closing
  none. The audit would add the cross-document contradictions and the references `check_plan.py` doesn't scan.
- **Applied meanwhile:** none; nothing is built, and nothing waits.
