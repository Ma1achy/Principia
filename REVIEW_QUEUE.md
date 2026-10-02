# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-192: four choices applied per R-204 that no ruling has settled: two older ones, and R-354's two *(R-298, TASK-M0-23, R-354)*

*Ruled in part by R-355 (2 Oct 2026): items 1, 2 and 4 stand, and item 3 is corrected. Ruled in part again by R-356 (2
Oct 2026): A is ruled, and B's bullets of R-297 stand but two. Open: those two bullets and the five marks (B), and
R-356's own mark (C). Under R-292 the entry moves to the archive, unchanged, once all of it is ruled; until then the
ruled items stay here, marked "ruled by R-355" or "ruled by R-356", their Mark lines closed.*

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
  - *Ruled by R-356 (A): option 1.* "A cut-off line after the summary line is valid: the summary and frames are kept
    and the dropped bytes reported. A file whose only line is a cut-off header stays an error. A small reader task for
    the cloud session." The reader task is TASK-M0-51 (REQ-TOOL-148). Whether the session then reads complete is C,
    below.
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
- **B. R-297's re-opened block, and the five marks that copy it** *(R-355 (c), R-356 (B))*.
  - *Ruled by R-356 (B): these stand,* "as direct applications of R-297's own text": `decisions.md` § "R-297 — Fast-math
    per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84,
    R-116)*", its bullets Design (ROUNDTRIP's tolerance, `:3076`), Design (where the setting lives, `:3082`), Design
    (what is recorded: the session header, `:3084`), Design (what the Profiler shows: the Profiler line, `:3088`),
    Design (the difference report, `:3092`) and Amendment (R-84 and R-116, `:3097`).
  - **Open, for the human to rule:** R-356: "Hold "GUI presence-only" and the R-133 amendment, and quote both, with the
    five marks in full, for me to rule." Each is quoted in full as it stands.
    1. R-297's GUI bullet, `decisions.md:3095-3096`:
       > - GUI: the Run window control and the Profiler line have no artboard, so they are checked by presence only
       >   until the M8 dev GUI (R-129).

       *What it decides:* the Run window's compute fast-math control (REQ-GUI-163) and the Profiler's compute line
       (REQ-GUI-164), which no artboard draws, are checked only for being there, not against a screenshot, until the
       M8 dev GUI (R-129).
    2. R-297's R-133 amendment bullet, `decisions.md:3099-3104`:
       > - Amendment (R-133), a consequence of R-297, not a mechanical one: REQ-COL-006's agreement gate moves, from
       >   TASK-M2-29 on, off REQ-DEC-043's calibrated f32 decode factor, which R-133 accepted as its tolerance until
       >   REQ-VAL-064 sets Tier N, and onto REQ-COL-060's calibrated tolerance. The fragment decode may compile with
       >   fast-math and the kernel's decode doesn't, so the agreement presets need the stated tolerance R-297 asks
       >   for. REQ-DEC-043's factor stays the tolerance for the fragment decode against the f64 `decodeOnly()`
       >   (REQ-TOOL-029). R-133 carries a forward line, and REQ-COL-006's note says the same.

       *What it decides:* from TASK-M2-29 on, the DECODE view's agreement gate (REQ-COL-006: the fragment decode
       against the compute kernel) is judged by REQ-COL-060's calibrated tolerance, not by REQ-DEC-043's f32 decode
       factor, which R-133 made its tolerance; that factor stays the tolerance against the f64 `decodeOnly()`.

       Both bullets are under R-297's block mark, `decisions.md:3065-3067`, which stays open for them alone: "*Applied
       per R-204 — veto?, open for the GUI bullet and the R-133 amendment bullet only (RQ-192); the two Plan bullets
       (the placements) accepted by R-354 (R-355), and the five Design bullets and the R-84 and R-116 amendment bullet
       by R-356:*".
       - **Mark:** `decisions.md` — "open for the GUI bullet and the R-133 amendment bullet only"
    3. REQ-RENDER-025's note, `plan/requirements.yaml:9269`:
       > The kernel keeps only the bring-up mode (R-75); R-41's baked-variant rule now applies to that mode alone.
       > ε_phys: REQ-ENC-024 (calibration). ε_phys is ROUNDTRIP's stated tolerance under R-297 too: the residual is
       > computed wholly in the fragment, so REQ-COL-060 (the agreement presets, fragment against compute) doesn't
       > apply to it; a residual that exceeds ε_phys under fragment fast-math is a REVIEW_QUEUE entry, not a second
       > tolerance (applied per R-204 — veto?).

       *What it decides:* ROUNDTRIP is held to ε_phys under fragment fast-math, not to REQ-COL-060's tolerance, and
       a residual above ε_phys is escalated, not met with a second tolerance. *Note:* it rests only on R-297's
       ROUNDTRIP bullet, which R-356 accepts; it stays open, as the human asked to rule all five.
       - **Mark:** `plan/requirements.yaml` — "is a REVIEW_QUEUE entry, not a second tolerance (applied per R-204"
    4. REQ-COL-060's note, `plan/requirements.yaml:11196`:
       > ROUNDTRIP's residual is computed wholly in the fragment, and its stated tolerance is ε_phys (REQ-ENC-024,
       > REQ-RENDER-025), not this one (applied per R-204 — veto?, R-297's Applied note).

       *What it decides:* REQ-COL-060's calibrated tolerance covers the agreement presets only, not ROUNDTRIP.
       *Note:* it rests only on R-297's ROUNDTRIP bullet, which R-356 accepts; it stays open, as the human asked to
       rule all five.
       - **Mark:** `plan/requirements.yaml` — "(REQ-ENC-024, REQ-RENDER-025), not this one"
    5. REQ-GUI-164's note, `plan/requirements.yaml:13682`:
       > Showing the compiled compute mode beside the setting where they differ is applied per R-204 — veto? (design,
       > R-297's Applied note); the vertex and fragment modes are recorded in the header (REQ-TOOL-141), not shown in
       > the Profiler, since R-297 asks only that the compute setting be shown.

       *What it decides:* the Profiler shows the compute setting, and beside it the compute stage's compiled mode
       where the two differ (as on lavapipe); the vertex and fragment modes are in the session header only. *Note:*
       it rests only on R-297's Profiler bullet, which R-356 accepts; it stays open, as the human asked to rule all
       five.
       - **Mark:** `plan/requirements.yaml` — "Showing the compiled compute mode beside the setting where they differ"
    6. TASK-M2-26's Notes, `plan/tasks/M2/TASK-M2-26.md:47`:
       > R-297: ROUNDTRIP's residual is computed wholly in the fragment, and its stated tolerance is ε_phys, which
       > R-297's "stated tolerance" names for it; REQ-COL-060 (TASK-M2-29) is the agreement presets' tolerance,
       > fragment against compute, and doesn't apply here. If the residual under fragment fast-math exceeds ε_phys,
       > that is a REVIEW_QUEUE entry, not a second tolerance (applied per R-204 — veto?).

       *What it decides:* TASK-M2-26's ROUNDTRIP golden checks the residual against ε_phys, not REQ-COL-060, and a
       residual above ε_phys under fragment fast-math becomes a REVIEW_QUEUE entry. *Note:* it rests only on R-297's
       ROUNDTRIP bullet, which R-356 accepts; it stays open, as the human asked to rule all five.
       - **Mark:** `plan/tasks/M2/TASK-M2-26.md` — "that is a REVIEW_QUEUE entry, not a second tolerance"
    7. TASK-M2-29's Notes, `plan/tasks/M2/TASK-M2-29.md:43`:
       > ROUNDTRIP is not this task's: its residual is fragment-only, `encode(decode(z))` with nothing from the
       > compute shader, and its stated tolerance is ε_phys (REQ-ENC-024), checked by TASK-M2-26's golden on both
       > backends (applied per R-204 — veto?).

       *What it decides:* TASK-M2-29, which calibrates REQ-COL-060, leaves ROUNDTRIP out; ROUNDTRIP's ε_phys is
       checked by TASK-M2-26's golden on both backends. *Note:* it rests only on R-297's ROUNDTRIP bullet, which
       R-356 accepts; it stays open, as the human asked to rule all five.
       - **Mark:** `plan/tasks/M2/TASK-M2-29.md` — "checked by TASK-M2-26's golden on both backends"
  - **Options seen:**
    1. **Accept 1–7 (recommended);** veto any, saying what replaces it.
    2. Veto 1: give the Run window control and the Profiler line a GUI screenshot check with an artboard drawn for
       them, or another check the human names.
    3. Veto 2: keep REQ-DEC-043's factor as REQ-COL-006's tolerance, with REQ-COL-060 beside it or in its place.
  - **Needed:** accept or veto. Nothing waits on it: TASK-M2-26, TASK-M2-29 and TASK-M8-43 have not started. When
    ruled, each mark names the ruling in place of "veto?".
- **C. R-356's own mark: a file with a cut-off line after its summary line reads as a complete session** *(R-356 (A),
  R-298, R-299, R-323)*.
  - `decisions.md` § "R-356 — A cut-off line after the summary line is valid; R-297's design bullets and its R-84 and
    R-116 amendments stand *(amends R-299, R-304)*", `decisions.md:4450-4458`:
    > *Applied per R-204 — veto?:* the session reads complete, since its summary line is present: `leak_flags` and
    > `hot_paths` are read from it, and the bytes after it are reported as dropped, not as "session incomplete", which
    > R-298 gives to a file missing its summary line. The human's words keep the summary and report the bytes, and
    > don't say which. `Trace`'s rule that a trace with dropped bytes is incomplete gives way: a complete trace may
    > have dropped bytes. `prin profile diff`'s notice for a cut-off trace (render_gui_spec § "Profiler", R-323) is
    > given to a session that ended before its summary line, so it doesn't reach this file, which loses no frame: the
    > diff compares it as a complete trace, and prints no notice for it. `prin profile show`, which today says
    > "session incomplete" whenever bytes were dropped, says so only for an incomplete session; for this file it
    > states the dropped bytes alone. Open in RQ-192 (C).
    - **Mark:** `decisions.md` — "the session reads complete, since its summary line is present"
  - The same, in `docs/design/principia_dd_telemetry_and_tiers.md` § 5, "A line cut off at the end", `:274-275`:
    "(applied per R-204 — veto?, RQ-192: the bytes after the summary line are reported as dropped, not as "session
    incomplete")".
    - **Mark:** `docs/design/principia_dd_telemetry_and_tiers.md` — "the bytes after the summary line are reported as dropped"
  - *What it decides:* such a file is a complete session: its summaries are read, `Trace::dropped_bytes` gives the
    bytes, and "session incomplete" is not reported, by the reader or by `prin profile show`, which states the dropped
    bytes alone; `prin profile diff` compares it with no notice. TASK-M0-51 builds it.
  - **The rulings it departs from, for this case:**
    - R-299's own words, `decisions.md:3154-3156`: "#79 item b (R-299): the reader drops an unterminated final line
      that doesn't parse, reports the session incomplete, and states how many bytes it dropped. A malformed line
      ending in a newline stays an error. Implementer change plus re-checks, then merge." R-299's "Still in force"
      line notes that "reports the session incomplete" is open here for this case.
    - R-323's human words, `decisions.md:3530-3531`, quoted under R-323: "a cut-off trace is reported as "session
      incomplete" with its dropped bytes, never silently compared over fewer frames". This file loses no frame, but it
      is a cut-off trace that the diff compares without a notice.
  - **Options seen:**
    1. **(Applied, recommended)** as above.
    2. As above, but `prin profile diff` prints the dropped bytes for such a file too, without "session incomplete";
       a few lines in TASK-M0-51.
    3. Report "session incomplete" for it too, as R-299's words have it for every dropped line: the summaries are
       kept, so `Session` needs a third state, or the rule that an incomplete session's summaries are absent changes;
       `prin profile show` then keeps today's notice.
  - **Needed:** accept or veto. TASK-M0-51 builds option 1 meanwhile; a veto before it merges changes it there.

---

## RQ-195: on Metal the generated display fraction is not exactly 1 at its endpoint *(TASK-M0-15, REQ-GEN-006, R-86)*

- **File, section:**
  - `plan/requirements.yaml`, REQ-GEN-006: "the derived display fraction (step / horizon_steps) must have exact
    endpoints"; its verify detail: "fuzz 0..65535: exact round trip on CPU and GPU; the fraction is exactly 0 and 1 at
    the endpoints". `plan/tasks/M0/TASK-M0-15.md` § "Goal": "with display-fraction endpoints exactly 0 and 1".
  - `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; …)":
    "`fn tm_t_end_fraction(w:u32, horizon_steps:u32)->f32  { return select(0.0, f32(tm_t_end_step(w))/f32(horizon_steps), horizon_steps > 0u); }`",
    and `tm_t_dmin_fraction` the same; `crates/render/frag/generated/payload_unpack.wgsl` emits both so
    (`crates/ledger/src/gen/wgsl.rs:420`).
  - The same doc, § "Why closure is here, at this width, unconditionally": "WGSL permits operation-specific
    floating-point error, FMA/fused ops, and binary16 subnormal flushing".
- **What:** TASK-M0-15's `codegen_selftest_times` dispatches the generated `tm_t_end_fraction(pack_times(n, 0), n)` and
  `tm_t_dmin_fraction(pack_times(0, n), n)` for every `horizon_steps` n in 1..=65535 on the GPU. On Metal (Apple M3
  Pro, this Mac) `f32(n)/f32(n)` is not 1.0 for 5658 of the 65535 horizons: 0.99999994 (`0x3f7fffff`) for n = 33, 47,
  55, 66, 94, …, and 1.0000001 (`0x3f800001`) for n = 115, …. The 0 endpoint is exact for every n, and the host Rust
  gives exactly 1.0 for every n. WGSL's f32 division is not correctly rounded, so the generated formula cannot give the
  exact endpoint the requirement asks for; the corpus states both and does not say which gives way. Interior fractions
  differ from the host's by at most 2 ULP; nothing requires them to match.
- **Options seen:**
  1. **(Recommended)** The generator emits the endpoint exactly: `select(f32(s)/f32(h), 1.0, s == h)` inside the
     existing `horizon_steps > 0u` guard, for both fractions; the Rust side is unchanged (IEEE division already gives
     1.0 there). Payload §6's two lines get the same form. A few lines in `crates/ledger/src/gen/wgsl.rs`, the
     regenerated file and the doc; TASK-M0-15 would carry them, or a task before it.
  2. Amend REQ-GEN-006: the endpoints are exact on the CPU, and within WGSL's division tolerance on the GPU.
- **Needed:** a ruling. TASK-M0-15 waits: its `codegen_selftest_times` fails on Metal as the generated WGSL stands; the
  rest of its suite passes.

---
