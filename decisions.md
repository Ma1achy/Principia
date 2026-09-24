# Decisions

The log of human rulings. Each entry is dated and names the step that applies it. The open decisions are in
`DECISIONS_TO_MAKE.md` (step 5). Once a ruling is made it is recorded here.

---

## R-1 — The markdown corpus is the only authority
*24 Sep 2026 · applied in step 3*

Step 3 checks each LaTeX passage against the **current markdown**, not against prin-rs `FINDINGS.md`.
Where the two genuinely conflict, add a `REVIEW_QUEUE.md` entry and don't choose. Citations to FINDINGS
can stay as notes.

## R-2 — The index doesn't cite the unpushed commit `52caf14` *(closes RQ-1)*
*24 Sep 2026 · applied in step 3*

The index shouldn't cite prin-rs `52caf14`, because it exists only on an unpushed branch. Repoint the
citation to `principia_dd_refinement_policy.md` §0.1, which records the same result.

## R-3 — The two PDFs are retired on the same terms as the LaTeX *(closes RQ-3)*
*24 Sep 2026 · applied in step 3*

`sphere_colour_map_spec.pdf` and `com_projection_mini_spec.pdf` are retired. Port what the markdown
relies on into the file that relies on it:
- colour PDF Eq. 5 → `principia_dd_colouring.md` §3.2
- colour PDF §8 (the shape-sphere component → axis convention) → `principia_dd_integrator.md` §3.7
- the COM-projection mini spec → wherever `sec:com_projection` is ported (integrator contract)

Then repoint the references.

## R-4 — "spec-keyed defaults" means the markdown's tier tables *(closes RQ-5)*
*24 Sep 2026 · applied in step 3*

`principia_dd_telemetry_and_tiers.md` line 398 refers to the markdown's tier tables: telemetry §3.5, the
three-axis `eps` / frame-budget / hard-cap model. Repoint it there. **Don't port** the LaTeX
`sec:quality_tiers`, because it predates the move to `eps`.

## R-5 — The chart constants go to the decision sheet *(closes RQ-2)*
*24 Sep 2026 · applied in step 5*

List them with audit B25. `μ_max` is 5 in the LaTeX and 4 in the IC Inspector notes. `α_min` is 0.05 in
the LaTeX and B25, and 0 in the markdown corpus and the tool.

## R-6 — The event priority order goes to the decision sheet *(closes RQ-4)*
*24 Sep 2026 · applied in step 5*

List it with audit B4. It has no LaTeX cross-check. Until it's decided, the markdown's pin in
`principia_dd_integrator.md` §3.6 (collision beats escape) stands.

## R-7 — Step 3's done-check *(closes RQ-6)*
*24 Sep 2026 · applied in step 3*

Step 3 is done when both of these hold:

**(a)** Run over the corpus markdown only, after step 2's move:

```
grep -rniwE --include='*.md' "latex|spec\.tex" docs/ --exclude-dir=archive
```

It returns only:
- the retirement ruling in `principia_canonical_spec.md`;
- `principia_spec_pending_changes.md`, which is exempt by name until step 4 archives it.

The root working docs (HANDOFF, REVIEW_QUEUE, decisions, open-questions), `spec_sources/`, `workbench/`
and `.git` are out of scope by construction.

**(b)** `spec_sources/SECTION_MAP.md` is the ledger for everything the grep can't judge ("the spec's …"
phrases). Every T, S and D row carries `done: <commit>` or a REVIEW_QUEUE reference. F and M rows need no
action. R rows are closed in step 4.

**After step 4:** rerun (a) with no exemptions. It returns only the retirement ruling.

## R-8 — The IC Inspector copies
*24 Sep 2026 · applied in step 3*

The `docs/` copies are canonical: `docs/gui/reference/ic_inspector.html` (19 Jul, 39.6 KB) and
`docs/notes/ic_inspector_scratchpad.md` (the later, longer one). The `spec_sources/` copies are older
uploads. They go to `docs/archive/` when `spec_sources/` is archived at the end of step 3.
