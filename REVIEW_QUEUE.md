# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-209: the colour-vision reference implementation, its version and its source, are not named *(REQ-COL-042, REQ-COL-045; TASK-M7-20)*
- **File, section:**
  - `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD": "The matrices and golden values come from a
    published reference implementation, named with its version when the task lands (R-78)."
  - `docs/design/principia_colour_composition.md` § "4.3 Display stage (terminal, outside the pipeline)": "with matrices
    and golden values from a published reference implementation, named with its version when the task lands (R-78;
    dd_colouring §3.8)."
  - `decisions.md` § "R-78 — Real Viénot and Brettel colour-vision simulation *(closes RQ-29)*": "Implement real Viénot
    (protan, deutan) and Brettel (tritan) through LMS, with golden values from a published reference implementation.
    dd_colouring §3.8's matrices are replaced."
  - `plan/tasks/M7/TASK-M7-20.md` § "Notes": "Gap: R-78 leaves the reference implementation (and version) to be named
    when the task lands; no requirement carries that choice to the human, so it is flagged in the milestone report."
- **What:** TASK-M7-20 needs the Viénot (protan, deutan) and Brettel (tritan) matrices, the LMS transform from linear
  sRGB they use, Brettel's two half-plane projections and separating plane, and the reference golden values for
  `fixtures/cvd/`. The corpus names no implementation, no version and no source to take them from, and the task file
  pins no fetchable source. The published implementations differ in the LMS basis (Smith–Pokorny, Hunt–Pointer–Estévez
  and others), in the anchor colours of Brettel's planes, and in whether Viénot's single-plane form is also used for
  tritan, so the choice changes every simulated colour and every golden value. The implementer may not choose one and
  transcribe its coefficients from memory (never guess a published coefficient), and may fetch only a source the task
  file pins. Achromatopsia (§3.8's M_achrom) and off need no source; the task's two acceptance tests both need the
  golden values, so the task can't close without them.
- **Options seen:**
  1. Name an implementation that has both methods through LMS from linear sRGB, with a released version, and pin it in
     the task file (repository URL, release tag or commit) so the implementer can fetch it read-only into scratch,
     transcribe its matrices and generate the golden values from it. DaltonLens-Python (Nicolas Burrus) is one such
     candidate: it implements Viénot 1999 and Brettel 1997 as separate simulators. Its coefficients and the version
     would then be transcribed into dd_colouring §3.8, as the task's third deliverable says.
  2. Name a different reference (another library, or the papers themselves, Viénot, Brettel & Mollon 1999 and Brettel,
     Viénot & Mollon 1997, with the LMS basis stated), pinned the same way.
- **Needed:** the name, version and pinned source of the reference implementation. Whether this is the human's (it
  changes results, but is a choice of colour reference rather than physics) or the orchestrator's per R-369 is itself
  for the orchestrator to judge; either way the source must be pinned in `plan/tasks/M7/TASK-M7-20.md` before the task
  resumes. TASK-M7-20 waits.
