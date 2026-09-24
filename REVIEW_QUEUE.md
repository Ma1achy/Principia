# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

---

## RQ-1: The display-adequacy evidence exists only on one machine *(step 1)*

- **Where:** `principia_INDEX.md`, "Known open items": "Display adequacy landed at prin-rs `52caf14`".
- **Finding:** `52caf14` (15 Sep) is **not on prin-rs `main`** (`8600d45`, 8 Sep). It sits on the local
  `lowering-spike` branch of `~/src/principia-rs-test`, which is 9 commits ahead of `origin/lowering-spike`.
  Those commits have never been pushed, so a fresh clone can't see the evidence the index cites.
- **Needed:** push `lowering-spike` or merge it into `main`, or tell me what the index should cite instead.
- **Ruling:** R-2 (decisions.md) — applied in step 3

## RQ-2: μ_max: the LaTeX says 5, the audit says 4 *(step 1)*

- **LaTeX** (`principia_spec_revised.tex` ~2991): "Mass logit saturation `μ_max = 5`; … `α_min = 0.05`".
- **Audit B25** (IC Inspector open items): "chart constants μ_max = 4, α_min = 0.05, q_max = 2".
- **Also:** pending change 6 and the `.md` corpus say `α_min = 0` ("the `.md` corpus and the tool are
  already consistent at `α_min = 0`"), and B25 says 0.05.
- **Needed:** the real chart config's values. This joins B25 on the decision sheet in step 5.
- **Ruling:** R-5 (decisions.md) — applied in step 5

## RQ-3: The sphere colour-map PDF: is it retired too? *(step 1)*

- The handoff retires the LaTeX. It says nothing about `spec_sources/sphere_colour_map_spec.pdf`.
- `principia_dd_colouring.md` line 3 calls the PDF "already publication-grade" and takes "Eq. 5, verbatim" from it (§3.2).
  `principia_dd_integrator.md` 185 and 246 take the shape-sphere axis convention from its §8.1.
- **Needed:** a ruling. If the PDF is retired too, step 3 ports its §8 and Eq. 5 in the same way. If it stays
  authoritative, the markdown has two sources of truth and `canonical_spec` should say so.
  `com_projection_mini_spec.pdf` raises the same question on a smaller scale.
- **Ruling:** R-3 (decisions.md) — applied in step 3

## RQ-4: The event priority order has nothing in the LaTeX to check against *(step 1)*

- `principia_dd_integrator.md` §3.6 (line 244): "confirm against the spec's event-detection section or veto and re-pin".
- `.tex` `sec:events` (987–993) has only "Collision: `min‖rᵢ−rⱼ‖ < r_coll`. Record pair." and then "Escape".
  There is **no priority order in it**.
- So the pin in dd_integrator §3.6 stands with no cross-check, and audit B4 has no LaTeX backing for either choice.
  I'm recording this, not choosing.
- **Ruling:** R-6 (decisions.md) — applied in step 5

## RQ-5: Which defaults does "spec-keyed defaults" mean? *(step 1)*

- `principia_dd_telemetry_and_tiers.md` 398: "Until then the spec-keyed defaults are placeholders".
- It could mean the `.tex` quality tiers (`sec:quality_tiers`, 1959–1980) or the tier tables in the markdown.
  Step 3 needs to know which one before it repoints the reference.
- **Ruling:** R-4 (decisions.md) — applied in step 3

## RQ-6: Step 3's new done-check can't pass as written *(step 1, after R-1..R-6)*

- **Check** (HANDOFF step 3): `grep -rniE "latex|spec\.tex|the spec'" --exclude-dir=spec_sources --exclude-dir=archive .`
  must return "only the retirement ruling in `principia_canonical_spec.md`".
- **Why it can't:** it scans the whole repo, so it also matches:
  - the root working docs, whose job is to talk about the LaTeX: the handoff (which contains the grep
    string itself), `REVIEW_QUEUE.md`, `decisions.md` and `open-questions.md`;
  - every HTML file with CSS `translateX`, because the case-insensitive `latex` matches `transLATEX`. That
    includes `principia_dev_gui.html`, `poster_both_sides.html` and `stain_*.html`, which step 2 moves to
    `workbench/` or `docs/gui/`;
  - `.git/` (it isn't excluded);
  - the pending-changes register, which step 4 archives only *after* step 3.
- **Options:** scope the check to `docs/` (the plan's original form), or add `--exclude-dir={workbench,.git}`
  and `--include='*.md'`, and exclude the root working docs. Either way, run it after the register is
  archived, or exempt the register by name. **Applied verbatim in the handoff until you rule.**
