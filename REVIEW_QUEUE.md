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
- **Ruling:** R-7 — applied in step 3

## RQ-7: `findings.md` was archived on a false premise *(step 2, INDEX)*

- **Where:** `docs/archive/findings.md` (it was `spec_sources/findings.md`).
- **The premise:** in step 1 I called it "an older 17 Jul copy of prin-rs `FINDINGS.md`". The reviewer's
  step-2 layout sent it to `archive/` on that basis.
- **What it actually is:** it is byte-identical to `~/src/principia-spike/FINDINGS.md`, the **toolchain spike findings**
  ("Verdict: rust-gpu … bit-identical to native on every golden input"). It isn't a copy of prin-rs.
  It's the evidence for the substrate decision, and `principia_spike_brief.md` says the spike's findings
  "are propagated across the corpus".
- **Options:** keep it archived (the spike was delivered and its findings have been absorbed), or move it to
  `docs/experiments/results/` next to the spike brief, as the evidence for a settled decision. Either way,
  the INDEX row's "why archived" is written as "unclear" until you rule.
- **Ruling:** R-9 — applied in step 3

## RQ-8: R-5's μ_max premise doesn't match the IC Inspector notes *(step 3, re-sweep)*

- **R-5 says:** "`μ_max` is 5 in the LaTeX and 4 in the IC Inspector notes."
- **The IC Inspector notes** (`docs/notes/ic_inspector_scratchpad.md` line 20) say: "**`μ_max` was 4 → corrected
  to 5** (dd_decoder §3.1, chart_decoder §87/96, inverse_encode §63; at 4 the mass-saturation range was too
  narrow and `MASS_SAT` fired at the wrong `z_μ`)". The same passage records "**`α_min` REMOVED** … `α = (π/2)·σ(z_α)`".
- **The rest of the corpus:** `principia_dd_decoder.md:38` has `[μ_max = 5]`, and `principia_chart_reference.md:31` has
  "`μ_max = 4` is the recorded default (an open …)". Audit B25 has 4.
- **So** the 4 appears only in `chart_reference` and in the audit. The IC Inspector notes argue *against* 4.
  I'm not choosing. Under R-5 the step-5 decision sheet should carry this evidence, and the step-3 ports use
  the named symbol.
- **Ruling:** R-10 (decisions.md) — applied in step 3

## RQ-9: The per-body momentum cap exists only in the LaTeX *(step 3, dd_decoder port)*

- **LaTeX** (`sec:jacobi_mom`, `.tex` 247–255): after the Jacobi-to-particle map, "Apply same rotation/mirror as
  positions. **Optional per-body cap with COM re-enforcement.**"
- **Markdown** (`principia_dd_decoder.md` §3.4): the momentum decode and the full-state rotation/mirror rule are there.
  No per-body cap is mentioned anywhere in the corpus.
- **I can't tell** whether the cap was dropped on purpose (the `q_max` saturation already bounds the Jacobi momenta)
  or lost in transcription. It isn't ported. Rule on it: port it as an optional step, or record it as dropped.
- **Ruling:** R-11 (decisions.md) — applied in step 3

## RQ-10: The shape-sphere chart map: axis assignment and polar buffer *(step 3, chart_reference port)*

- **Markdown** (`principia_chart_reference.md` §3.3, spherical option): $\theta = \pi v$, $\varphi = 2\pi u$. θ is on the
  **vertical** axis and there's **no buffer**.
- **LaTeX** (`sec:shape_sphere_view`, `.tex` 333–383): $\theta(u) = \varepsilon + (\pi - 2\varepsilon)u$, $\varphi(v) = 2\pi v$.
  θ is on the **horizontal** axis, with a buffer $\varepsilon > 0$ "to avoid the collision-singularity poles".
- **Also:** the LaTeX's reason for the buffer is itself doubtful. Under the markdown's own §3.4 the collision points
  sit **on the equator**, not at the poles (the poles are the Lagrange configurations).
- **Not ported. The markdown is unchanged.** Rule on the axis assignment and on whether θ gets a buffer.
- **Ruling:** R-12 (decisions.md) — applied in step 3

## RQ-11: Lookup's decode-sanity check vs "a t = 0 collision is a real outcome" *(step 3, inverse_encode port)*

- **Ported, as the LaTeX wrote it** (`sec:chart_validation`, `.tex` 771–815 → `principia_inverse_encode_contract.md`,
  Chart-aware validation, layer 3): decode sanity requires "no two bodies coincident ($r_{ij} > r_{\mathrm{coll}}$)".
  A failure means project, clamp or reject.
- **The markdown elsewhere:** `principia_chart_reference.md` §0.7 and the LaTeX's own `sec:no_holes_impl` say
  "`COLLISION_T0(pair)` with `t_event = 0` if `r_min(0) < r_coll`. **No pixel is ever rejected.**"
  `principia_integrator_contract.md` 348 calls a t = 0 terminal "a real outcome".
- **The tension:** for a *rendered pixel* $r < r_{\mathrm{coll}}$ at t = 0 is a labelled outcome. For a *typed-in lookup*
  the ported rule refuses or moves it. That may be intended (lookup is user entry, not rendering), or it may be stale.
  I'm not choosing. Rule on whether lookup should accept and label a t = 0 collision.
- **Ruling:** R-13 (decisions.md) — applied in step 3
