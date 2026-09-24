# Hand-off: untangle the Principia corpus

*For Claude Code. Written 24 Sep 2026. A reviewer (Claude, in chat) will review every commit.*

Read **`principia_AUDIT_2026-09-24.md`** first — it is the evidence for everything below.

## The rules

- **Work in a git repo.** Commit the untouched corpus first. Then one commit per step below, each
  small enough to review as a diff.
- **Moves never change content.** A commit that moves files moves only — edits come in later commits.
- **Never guess on physics.** When the LaTeX and the current markdown disagree, or you can't tell
  whether a LaTeX passage is stale, **flag it** in `REVIEW_QUEUE.md` (file, section, both versions)
  and move on. Don't choose.
- **Nothing is deferred silently.** Anything left open goes in `open-questions.md` with its source.

## Rulings already made

- **R-1: the markdown corpus is the only authority** (`decisions.md`). Step 3 checks each LaTeX passage
  against the current markdown, not against prin-rs `FINDINGS.md`. Where they genuinely conflict, add a
  `REVIEW_QUEUE.md` entry and don't choose. FINDINGS citations can stay as notes. Later rulings are in `decisions.md`.
- **The LaTeX is retired.** The markdown is the only source of truth. Delete the clause in
  `principia_chart_reference.md` saying the LaTeX wins. Write the ruling into `principia_canonical_spec.md`.
- **`principia_render_gui_spec.md` is rewritten** from the dev-GUI design canvas (step 6), not patched.
- **`principia_debug_tooling_plan.md` stays current.** It overlaps the new profiler and Overlays menu
  only partly, which is fine.
- **Evidence** (`ftle_shadow_precision_experiment`, `dd_refinement_external_report{,2,3,4}`, the
  `xp_results*` and `refinement_probe` folders) → `experiments/results/`.
- **Briefs** (`spike_brief` and the `ARCHIVE_brief_*` files) → `experiments/briefs/`, archived.
- **Self-declared superseded** (`parity_testing_note`, `validation_scratchpad`) → `archive/`.

## Steps, in order

0. **Snapshot.** Commit everything as it is. The zips stay untouched as a fallback.
1. **Locate the sources.** The LaTeX is `spec_sources/principia_spec_revised.tex` (8 Jul), with
   `spec_sources/com_projection_mini_spec.pdf` for `§com_projection`.
2. **Restructure — moves only.** Folders: `read_first/`, `contracts/`, `design/` (the drill-downs),
   `notes/`, `gui/`, `experiments/{briefs,results}/`, `archive/`. Classify the 16 files missing from
   the index (audit A2). Update `INDEX.md` paths.
3. **Port the LaTeX, then remove it.** For every reference (audit A1 — 85 references in 19 files, 21 by
   named section):
   1. extract the content from the `.tex`;
   2. check it against current markdown and measured findings — the Heggie default, the closure +
      energy escape criterion and `Policy::Tolerance` all post-date the LaTeX;
   3. write what's still correct into the drill-down that relies on it;
   4. repoint the reference.

   Done when
   ```
   grep -rniE "latex|spec\.tex|the spec'" --exclude-dir=spec_sources \
     --exclude-dir=archive .
   ```
   returns only the retirement ruling in `principia_canonical_spec.md`.
4. **Close the pending-changes register.** Fold each landed change's substance into its owning file.
   Mark the rest in `open-questions.md`. Archive the register.
5. **Decision sheet.** Write `DECISIONS_TO_MAKE.md`: audit section B's 25 items, each with its options,
   a recommendation and the files it affects. **Stop here** for the human to rule.
6. **GUI docs.** Rewrite `gui/render_gui_spec.md` and update `gui_state_contract.md` from the design
   canvas. The reviewer will supply its artboards as PNGs for `gui/design/`.
7. **Build plan — after 5 and 6.** Extract every requirement with an ID. Write `plan/` (one file per
   task, citing file and section), `tasks.yaml` and the traceability matrix.

## What to hand back after each step

The commit hash, a one-paragraph summary, and anything added to `REVIEW_QUEUE.md`.
