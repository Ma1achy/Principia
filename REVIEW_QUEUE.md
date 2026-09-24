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

## RQ-2: μ_max: the LaTeX says 5, the audit says 4 *(step 1)*

- **LaTeX** (`principia_spec_revised.tex` ~2991): "Mass logit saturation `μ_max = 5`; … `α_min = 0.05`".
- **Audit B25** (IC Inspector open items): "chart constants μ_max = 4, α_min = 0.05, q_max = 2".
- **Also:** pending change 6 and the `.md` corpus say `α_min = 0` ("the `.md` corpus and the tool are
  already consistent at `α_min = 0`"), and B25 says 0.05.
- **Needed:** the real chart config's values. This joins B25 on the decision sheet in step 5.

## RQ-3: The sphere colour-map PDF: is it retired too? *(step 1)*

- The handoff retires the LaTeX. It says nothing about `spec_sources/sphere_colour_map_spec.pdf`.
- `principia_dd_colouring.md` line 3 calls the PDF "already publication-grade" and takes "Eq. 5, verbatim" from it (§3.2).
  `principia_dd_integrator.md` 185 and 246 take the shape-sphere axis convention from its §8.1.
- **Needed:** a ruling. If the PDF is retired too, step 3 ports its §8 and Eq. 5 in the same way. If it stays
  authoritative, the markdown has two sources of truth and `canonical_spec` should say so.
  `com_projection_mini_spec.pdf` raises the same question on a smaller scale.

## RQ-4: The event priority order has nothing in the LaTeX to check against *(step 1)*

- `principia_dd_integrator.md` §3.6 (line 244): "confirm against the spec's event-detection section or veto and re-pin".
- `.tex` `sec:events` (987–993) has only "Collision: `min‖rᵢ−rⱼ‖ < r_coll`. Record pair." and then "Escape".
  There is **no priority order in it**.
- So the pin in dd_integrator §3.6 stands with no cross-check, and audit B4 has no LaTeX backing for either choice.
  I'm recording this, not choosing.

## RQ-5: Which defaults does "spec-keyed defaults" mean? *(step 1)*

- `principia_dd_telemetry_and_tiers.md` 398: "Until then the spec-keyed defaults are placeholders".
- It could mean the `.tex` quality tiers (`sec:quality_tiers`, 1959–1980) or the tier tables in the markdown.
  Step 3 needs to know which one before it repoints the reference.
