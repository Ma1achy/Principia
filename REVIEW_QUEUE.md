# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-192: four choices applied per R-204 that no ruling has settled: two older ones, and R-354's two *(R-298, TASK-M0-23, R-354)*

*Ruled in part by R-355 (2 Oct 2026): items 1, 2 and 4 stand, and item 3 is corrected. Open: the question under item 1
and the marks under item 3. Under R-292 the entry moves to the archive, unchanged, once all of it is ruled; until then
the ruled items stay here, marked "ruled by R-355", their Mark lines closed.*

- **File, section:**
  - `decisions.md` § "R-298 — TASK-M0-17's items 12 and 15 accepted; a trace with no summary line is valid *(amends
    R-286)*": "*Applied per R-204 — veto?:* R-298 is in the "design" group … The type shape and the rest are in PR
    #79."
  - `plan/tasks/M0/TASK-M0-23.md` § "Deliverables": "`.github/workflows/mutants.yml` (applied per R-204, R-305 —
    veto?: its own workflow, …)".
  - `decisions.md` § "R-354 — The seven open veto items stand, R-252 stays amended, not superseded; settled "veto?"
    marks name their ruling, and open ones must be in the review queue": its two "Applied per R-204 — veto?" items.
- **What:** these were applied without asking (R-204) and are open for a veto:
  1. *Ruled by R-355 (a): stands.* R-298's mark, PR #79's items a, c, d and e: `Trace` keeps `leak_flags` and
     `hot_paths` as options and adds a `session` field (a); a header line alone is an incomplete session with no
     frames (c); the writer writes an incomplete trace without a summary line and refuses one with a summary set (d);
     R-298 is in the "design" group (e). R-282 accepted #79's numbered items, R-299 ruled item b and R-304 items f–i;
     no ruling names a, c, d or e.
     - **Mark, closed by R-355:** `decisions.md` — "A header line alone counts as an incomplete session with no frames"
  2. *Ruled by R-355 (b): stands.* TASK-M0-23's deliverable, PR #65's item 16: the per-PR mutation gate is its own
     workflow, `mutants.yml`, which only `pull_request` runs, so a `push` run never reports a skipped
     `mutants-check`. R-305 accepted #65's items 10–13; no ruling names items 14–16 (14 and 15 are in #65's
     description only).
     - **Mark, closed by R-355:** `plan/tasks/M0/TASK-M0-23.md` — "its own workflow, which only `pull_request` runs"
  3. *Corrected by R-355 (c): the human's "item 6 stands" covered the placements only.* R-354: item 6 of the seven is
     read as R-297's whole "Applied per R-204" block (its design, GUI and amendment bullets with the placements),
     though the list named only the placements.
     - **Mark, closed by R-355:** `decisions.md` — "R-297's mark heads one block, and item 6 is read as all of it"
  4. *Ruled by R-355 (d): stands.* R-354: the check's design: open marks are named by open REVIEW_QUEUE.md entries
     ("**Mark:**" lines), not kept in a list of their own; the scan covers decisions.md, CLAUDE.md, `plan/` and
     `docs/` (less `docs/archive/` and `docs/reference/`), not PR descriptions; the negative controls are
     `plan/tools/veto_marks.py`'s built-in cases, run by every plan check, not a new `xtask/tests/plan_check.rs` case.
     - **Mark, closed by R-355:** `decisions.md` — "the open marks live in REVIEW_QUEUE.md, where R-346 already keeps"
- **Options seen:**
  1. **Accept items 1–4 (recommended);** veto any by number, saying what replaces it.
  2. For item 4: keep the open marks in a list of their own (`plan/open_veto_marks.yaml`) rather than in the queue.
- **Needed:** accept or veto. Nothing waits on it: each is merged or applied as written. When it is ruled, each mark
  names the ruling in place of "veto?", in the same commit that moves this entry to the archive.

**Open after R-355:**

- **A. Whether "never rejected" reaches R-299's two error cases** *(R-355 (a), R-299, R-304, TASK-M0-17)*.
  - The human's words (R-355): "R-298's items stand, read as: a cut-off last line is dropped and reported as dropped
    bytes, the file stays valid and the complete frames before it are kept (R-298, R-299). The file is never rejected
    for it."
  - `decisions.md` § "R-299 — The reader drops a cut-off final line and says how many bytes it dropped *(amends
    R-298)*", its note "*Applied per R-204, accepted by R-304:*" (R-304 accepted these as PR #79's items g and h):
    "Because the dropped line held the last place, the line before it keeps a frame's place: a summary line followed
    by a cut-off line is an error, since the writer writes nothing after the summary line. A file whose only line is a
    cut-off header line has no header line to read, and stays an error, as an empty file is; the error states the
    bytes."
  - `docs/design/principia_dd_telemetry_and_tiers.md` § 5, "A line cut off at the end" (R-299): "the line before it
    holds a frame's place, so it is a frame record or the header line" and "A file whose only line is cut off has no
    header line, and a reader rejects it as it does an empty file."
  - **What the merged reader does today** (`engine::contract::profile::read_lines`, TASK-M0-17,
    `crates/engine/src/contract/profile.rs`): (1) a summary line followed by a cut-off line is an error: the reader
    holds each line until the next arrives and parses every line with one after it as a frame record, so the summary
    line fails as "line N, a frame record: …"; (2) a file whose only line is cut off is an error: "line 1 is cut off:
    <n> bytes with no newline that are not JSON, so the file has no header line". Every other cut-off last line is
    dropped, with `Trace::dropped_bytes` set to its length and the session `Incomplete`.
  - **Options seen:**
    1. **(Recommended)** (1) becomes valid: the cut-off line is dropped and reported as dropped bytes, and the summary
       line and the frames are kept; (2) stays an error, because there is no header to read, so there is no trace to
       keep.
    2. Both stay errors, as R-299's note and R-304 have them: "never rejected" covers a cut-off line after a frame
       record or the header line only.
    3. Both become valid; (2) would need a trace with no header, which telemetry §5 has no form for.
  - **Needed:** a choice. If either case changes, the reader needs a code change in a follow-up task, with telemetry
    §5, REQ-TOOL-008 and its tests, to be added after the ruling (R-355: no new task here). Nothing else waits on it.
- **B. The five marks R-354 closed under item 6 beyond the placements, re-opened by R-355 (c)**, each quoted in full
  as it stands:
  1. REQ-RENDER-025's note, `plan/requirements.yaml:9269`:
     > The kernel keeps only the bring-up mode (R-75); R-41's baked-variant rule now applies to that mode alone.
     > ε_phys: REQ-ENC-024 (calibration). ε_phys is ROUNDTRIP's stated tolerance under R-297 too: the residual is
     > computed wholly in the fragment, so REQ-COL-060 (the agreement presets, fragment against compute) doesn't apply
     > to it; a residual that exceeds ε_phys under fragment fast-math is a REVIEW_QUEUE entry, not a second tolerance
     > (applied per R-204 — veto?).
     - **Mark:** `plan/requirements.yaml` — "is a REVIEW_QUEUE entry, not a second tolerance (applied per R-204"
  2. REQ-COL-060's note, `plan/requirements.yaml:11196`:
     > ROUNDTRIP's residual is computed wholly in the fragment, and its stated tolerance is ε_phys (REQ-ENC-024,
     > REQ-RENDER-025), not this one (applied per R-204 — veto?, R-297's Applied note).
     - **Mark:** `plan/requirements.yaml` — "(REQ-ENC-024, REQ-RENDER-025), not this one"
  3. REQ-GUI-164's note, `plan/requirements.yaml:13682`:
     > Showing the compiled compute mode beside the setting where they differ is applied per R-204 — veto? (design,
     > R-297's Applied note); the vertex and fragment modes are recorded in the header (REQ-TOOL-141), not shown in the
     > Profiler, since R-297 asks only that the compute setting be shown.
     - **Mark:** `plan/requirements.yaml` — "Showing the compiled compute mode beside the setting where they differ"
  4. TASK-M2-26's Notes, `plan/tasks/M2/TASK-M2-26.md:47`:
     > R-297: ROUNDTRIP's residual is computed wholly in the fragment, and its stated tolerance is ε_phys, which R-297's
     > "stated tolerance" names for it; REQ-COL-060 (TASK-M2-29) is the agreement presets' tolerance, fragment against
     > compute, and doesn't apply here. If the residual under fragment fast-math exceeds ε_phys, that is a
     > REVIEW_QUEUE entry, not a second tolerance (applied per R-204 — veto?).
     - **Mark:** `plan/tasks/M2/TASK-M2-26.md` — "that is a REVIEW_QUEUE entry, not a second tolerance"
  5. TASK-M2-29's Notes, `plan/tasks/M2/TASK-M2-29.md:43`:
     > ROUNDTRIP is not this task's: its residual is fragment-only, `encode(decode(z))` with nothing from the compute
     > shader, and its stated tolerance is ε_phys (REQ-ENC-024), checked by TASK-M2-26's golden on both backends
     > (applied per R-204 — veto?).
     - **Mark:** `plan/tasks/M2/TASK-M2-29.md` — "checked by TASK-M2-26's golden on both backends"

  Marks 1, 2, 4 and 5 copy R-297's "Design (ROUNDTRIP's tolerance)" bullet (`decisions.md:3074`), and mark 3 its
  "Design (what the Profiler shows)" bullet (`decisions.md:3086`).
  - **Not among the five, re-opened with them:** those two bullets' source, R-297's own block,
    `decisions.md:3065`: "*Applied per R-204 — veto?, but for the two Plan bullets (the placements), accepted by R-354
    (R-355):*". R-354 had closed the whole block under item 6. Under R-355 (c) only its two Plan bullets (the
    placements) stand, so its other bullets are open again with the five: Design (ROUNDTRIP's tolerance), Design
    (where the setting lives: a `SimConfig` field), Design (what is recorded: the setting asked for and each stage's
    compiled mode), Design (what the Profiler shows), Design (the difference report: `cargo xtask gate
    fast-math-diff`), GUI (presence only until M8, R-129), Amendment (R-84 and R-116 carry forward lines) and
    Amendment (R-133: REQ-COL-006's agreement gate moves onto REQ-COL-060's tolerance).
    - **Mark:** `decisions.md` — "but for the two Plan bullets (the placements), accepted by R-354"
  - **Options seen:**
    1. **Accept marks 1–5 and R-297's other bullets (recommended);** veto any, saying what replaces it.
    2. Rule the five only, and the rest of R-297's block separately.
  - **Needed:** accept or veto. Nothing waits on it: TASK-M2-26, TASK-M2-29 and TASK-M8-43 have not started. When
    ruled, each mark names the ruling in place of "veto?", and this entry moves to the archive.

---
