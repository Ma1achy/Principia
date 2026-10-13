# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

## RQ-274: R-423's M1-gate check cannot confirm `q_mass` and `mean_y` on R-400's cool-warm ramp: `q_mass`'s formula and `mean_y`'s per-sample `y` are undefined, each ledger range is the unbounded default, and the corpus's own words read each as non-negative *(silence, R-400, R-423, R-72, REQ-RENDER-084, REQ-RENDER-085, REQ-PAY-088, REQ-PAY-073; context: the M1 gate, TASK-M1-17, TASK-M1-20, TASK-M2-03, TASK-M3-13)*

*Found by the physics review of PR #197 (review 5484757609, § "The M1-gate five-field check (R-423)"), which ran
R-423's gate check of the five fields early. Filed per `plan/MILESTONES.md` § "M1"'s gate item: "A field that fails
goes to `REVIEW_QUEUE.md` for the human". Nothing is chosen.*

- **File, section:**
  - `decisions.md` § "R-423 — At the M1 gate physics checks the six fields R-400 moved to cool-warm; `theta`, an angle,
    takes the cyclic ramp, not the diverging one *(amends R-400)*", the human's words: "For the M1 gate: physics to
    check the six fields moving to cool-warm." `plan/MILESTONES.md` § "M1 — The synthetic payload and the eyes", the
    gate item: "a read-only physics check of `q_mass`, `ftle`, `mean_y`, `S` and `diffusion`: that each is a signed
    quantity whose zero is a meaningful centre for a diverging ramp centred at zero. A field that fails goes to
    `REVIEW_QUEUE.md` for the human".
  - `decisions.md` § "R-400 — A numeric field view whose declared ledger range spans zero is on a diverging ramp
    centred at zero; fields that can't be negative keep viridis *(closes RQ-260)*": "Where §3 gives a field no range
    the ledger declares (−∞, ∞) (`crates/ledger/src/payload.rs`:7), which spans zero; a field the corpus gives a
    non-negative domain must not read as signed by that default. So TASK-M1-17 declares the lower bound of each field
    whose domain the corpus or the field's own definition makes non-negative". REQ-RENDER-084 (TASK-M1-17, merged in
    PR #195) is the requirement that draws a field whose range spans zero on `ramp_coolwarm`.
  - `q_mass`: `crates/ledger/src/payload.rs`:261, `ic("q_mass", 3, Scale::Lin),` with no `.range(…)`, so the
    unbounded default, beside `m0`–`m2` (`.range(fraction)`) and `rho_mag`, `lambda_mag` (`.range(from_zero)`).
    `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)" (:171) lists "`q_mass`" with no
    scale or range; `docs/design/principia_dd_decoder.md` § "3.6 ICDescriptor derived quantities (decode-time,
    pre-integration)" gives no formula. `plan/requirements.yaml` REQ-PAY-088 (`kind: definition`, `milestone: M2`,
    TASK-M2-03): "The formulas of the ICDescriptor fields `q_mass`, `rho_mag`, `lambda_mag`, `rho_ratio`, `rho_angle`
    and `r_min_pair_0` … must be defined in dd_decoder §3.6 by the task that needs it".
    `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`" (:95) lists among
    the decoded-IC quantities "mass ratios".
  - `mean_y`: `crates/ledger/src/payload.rs`:217, `f32_at("mean_y", 26, Scale::Lin),`, the unbounded default.
    `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)": "Slope of spread `y`
    on time `t` (the diffusion coefficient)"; `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars —
    with presentation metadata" (:113), the mini-spec: "Regression of spread `y` on time `t`, slope = diffusion rate",
    and (:123) "`mean_y += dy / n`", so `mean_y` is the running mean of `y`.
    `plan/requirements.yaml` REQ-PAY-073 (`kind: definition`, `milestone: M3`, TASK-M3-13): "The per-sample value `y`
    that the Welford diffusion regression fits against time must be defined in `principia_dd_simstate_payload.md` §4 by
    the task that needs it".
- **Physics' result** (review 5484757609): `S`, `ftle` and `diffusion` pass; each is signed at finite time, with zero
  a meaningful centre, and none is an angle. `q_mass` and `mean_y` are **not confirmed**:
  - `q_mass`: "Its formula is unwritten (REQ-PAY-088, TASK-M2-03, M2; dd_decoder §3.6 lists only the name). Its ledger
    range … is the unbounded default, not derived. A mass ratio would be ≥ 0, which is viridis under R-400; a
    log-ratio or asymmetry would be signed. The corpus doesn't settle it."
  - `mean_y`: "It is the running mean of the per-sample y, which is undefined (REQ-PAY-073, M3, R-72). The corpus calls
    y a 'spread' (payload §4; generation_root §3.4 mini-spec), which would make mean_y ≥ 0 and viridis. Its ledger
    range (payload.rs:217) is again the unbounded default."
- **Silence:** the corpus does not say whether `q_mass` or `mean_y` can be negative. Each is on `ramp_coolwarm` only
  because no range was declared, which is R-400's default reading, not a finding that the field is signed; and the
  corpus's own words ("mass ratios"; "spread `y`") lean the other way. Their definitions are later milestones' (M2 and
  M3), so the M1 gate cannot settle the check from the corpus.
- **Options seen:**
  - (a) Keep both on `ramp_coolwarm` provisionally, until REQ-PAY-088 defines `q_mass` and REQ-PAY-073 defines `y`;
    nothing changes now, and the M1 gate records them as provisional.
  - (b) Switch both to viridis now, reading `q_mass` as a non-negative ratio and `mean_y` as the mean of a
    non-negative spread: each gets a declared lower bound of 0 in the ledger (R-400's mechanism), its view moves to
    viridis and its golden is re-rendered; REQ-PAY-088's and REQ-PAY-073's definitions are then bound to be
    non-negative, or the bound changes through an RQ.
  - (c) Re-check at their definitions: `q_mass` when TASK-M2-03 writes REQ-PAY-088 (M2), `mean_y` when TASK-M3-13
    writes REQ-PAY-073 (M3), each task declaring the field's range from its definition and the physics reviewer
    checking the ramp then.
  - (a) and (c) may be taken together, and the options may be taken per field.
- **Applied meanwhile:** the gate check's result is recorded in `plan/MILESTONES.md` § "M1"'s gate item and in R-423;
  `S`, `ftle` and `diffusion` stay on `ramp_coolwarm`, confirmed. `q_mass` and `mean_y` stay on `ramp_coolwarm` as
  TASK-M1-17 left them, which settles nothing here.
- **Waits:** the M1 gate's R-423 cool-warm item, for `q_mass` and `mean_y` only: the gate report cannot give either
  field as confirmed until this entry is ruled, at or before the M1 gate. No task waits, so no requirement is marked
  `rq: RQ-274`:
  - TASK-M1-20 (REQ-RENDER-085) does not wait: `theta`'s move to the cyclic ramp is the same under every option. Its
    "`q_mass` … and `mean_y` keep `ramp_coolwarm`" assertion pins the state as TASK-M1-17 left it; under (b) it
    changes as a ruling-forced change (R-290, R-369), made by whichever task the ruling's port names.
  - TASK-M2-03 (REQ-PAY-088) and TASK-M3-13 (REQ-PAY-073) do not wait: each writes its definition under every option,
    and (c) needs them to run. They are named, not marked, as RQ-271 named REQ-PAY-088.
  - REQ-RENDER-084 is closed by TASK-M1-17, merged; a mark would hold nothing.

---

## RQ-275: render_contract Part 6's quad-fields row lists five payload/status flags read from `ctx.quad` that no contract defines: `RenderQuad` and the stain's `CtxQuad` carry none, contains-ensemble and contains-FTLE exist only as the prelude's per-variant and per-frame guards, the schema version only as a WGSL constant, and per-quad sim-failed and cache-valid have no source *(silence and placement, R-72, R-113, R-145, REQ-TOOL-011, REQ-TOOL-026; context: TASK-M1-19, TASK-M1-13)*

*Found in TASK-M1-19's read-only pre-flight. Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)" (line 315), the quad
    fields row: "Quad fields — all 9 (depth, state enum, coherence, impurity, spread, suspect fraction, priority, cache
    age, ancestor gap) + payload/status flags (sim-failed, cache-valid, contains-ensemble, contains-FTLE, schema
    version) | `ctx.quad` | the **CPU scheduler** and the payload compatibility signature — CPU-written, so a wrong view
    here exonerates the GPU".
  - `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record" (lines 462–485):
    "it is the ledger table the generated `RenderQuad` follows, member for member … the fragment reads it as
    `ctx.quad` (render contract Part 1; Part 6's "Quad fields" view)", with ten members, `quad_depth`, `quad_state`,
    `coherence_score`, `outcome_impurity`, `ensemble_spread`, `suspect_fraction`, `priority_score`, `ancestor_gap`,
    `cache_age` and `dominant_outcome`, and no flag. It adds: "It is scheduler state, not payload, and the payload
    compatibility signature excludes scheduler state … so it is not hashed into the schema version."
  - The code at origin/main 55513b7b: `crates/render/shaders/wgsl/stain/context.wgsl`, `struct CtxQuad` (lines 19–36),
    holds `index`, `tl`, `centre`, `half_width`, `uv`, `sample_count` and the `RenderQuad` members, and no flag.
  - contains-FTLE and contains-ensemble: `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline,
    swappable slots" (line 46): "`has_ftle`/`has_word` are baked consts; `has_ensemble` is a uniform, read the same
    way, so toggling E never re-bakes (R-145 …)". `crates/render/shaders/wgsl/lib/prelude.wgsl` (lines 6–25): "This
    variant's tier (lowering Part 3a): baked … `has_ftle` is whether the stored variant is `SimStateFTLE`",
    `const has_ftle: bool = true;`, and the `PreludeUniforms` member `has_ensemble`, read by `fn has_ensemble()`.
    `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array —
    lockstep, ratified)" (line 167): "ensemble-on/off is a quad/tier flag (`contains-ensemble`)", and (line 156) "Tier gates
    existence; `contains-ensemble` / `contains-FTLE` advertise it". Each is per variant or per frame, not a
    `RenderQuad` member. RQ-239 (archived, `docs/archive/review_queue/M0.md`) decided for REQ-TOOL-026's spread view
    only that "Present iff contains-ensemble" reads `has_ensemble()`.
  - The schema version: `crates/render/frag/generated/payload_unpack.wgsl` (line 7), `const PAYLOAD_SCHEMA_VERSION:
    vec2<u32> = …`, one constant per build, which `RenderQuad` is "not hashed into" (above).
  - sim-failed: the per-sample `state` code 4, `sim_failed` (`docs/design/principia_dd_generation_root.md` § "3.1
    `sample_descriptor` (u32)", line 37; render contract Part 5's `sd_is_failed`). No quad-level failed flag or count is defined: `QuadReduction`
    (§ "3.7 `QuadReduction` — completed ledger") carries `terminated_fraction`, `class_histogram[N]` and
    `dominant_outcome`, and `RenderQuad` none of these but `dominant_outcome`.
  - cache-valid: `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"
    (line 14): "**Validity — *computed under which physics config*.** The payload compatibility signature: … payload
    schema version (a content hash of the ledger, R-36)". No per-quad valid flag is defined, and no M1 cache exists.
  - `plan/requirements.yaml` REQ-TOOL-011 (`milestone: M1`, TASK-M1-19): "The field-view catalogue must cover every
    group in render contract Part 6's field-view table that a synthetic payload can show: … all 9 quad fields plus
    payload/status flags, and structural overlays". REQ-TOOL-026 (M1, TASK-M1-13, merged), the quad views: "quad-depth,
    quad-state …, coherence/impurity …, ensemble spread (present iff contains-ensemble), suspect fraction, priority
    score, cache age / ancestor gap, leaf outlines, fallback tint and pending hatch", with no status flag.
  - `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*", as REQ-TOOL-011's
    note records it, split off the DECODE, ensemble and live-effort rows; § "R-420 — The uniform echo row moves …
    *(closes RQ-273)*" moved the uniform echo row and removed the Fourier row. Neither split off the flags.
- **Silence (and a placement question):** the row reads five flags from `ctx.quad`, but no contract makes any of them a
  `RenderQuad` member or says what each holds per quad. Two have a per-variant or per-frame reading in the prelude
  (`has_ftle`, `has_ensemble()`) and one a per-build constant (`PAYLOAD_SCHEMA_VERSION`), none of them quad data; and
  generation_root §3.7a says `RenderQuad` is "not hashed into the schema version". Per-quad sim-failed and
  cache-valid have no data source at all: no quad-level failure flag is defined, and cache validity is the M5 cache's
  signature. An M1 checklist that ticks the row "on a synthetic payload" has nothing to tick for the flags; R-113 and
  R-420 placed other rows that need later machinery, but not these.
- **Options seen:**
  - (a) Define them now (R-72, a definition requirement): each flag's meaning per quad and its source (a `RenderQuad`
    member, the prelude guard, or the schema constant), written into dd_generation_root §3.7a or render_contract Part
    6, as a TASK-M1-19 deliverable or a new task's.
  - (b) Split them off to the task that builds their source, as R-113 split the DECODE, ensemble and live-effort rows:
    cache-valid with the M5 cache, per-quad sim-failed with the reduction (`QuadReduction`), and so on, each under a
    named requirement that REQ-TOOL-011's checklist then cites for the cell.
  - (c) Drop the flags from Part 6's row (a ruling; render_contract Part 6 then conformed). Dropping is the human's
    call only.
  - The options may be taken per flag: for example contains-ensemble and contains-FTLE ticked against the prelude's
    `has_ensemble()` and `has_ftle` (as RQ-239 read contains-ensemble for REQ-TOOL-026), and the rest split or dropped.
- **Applied meanwhile:** TASK-M1-19 is built with every other row of Part 6's table; its PR's checklist marks the five
  flag cells of the quad fields row as waiting on RQ-275. The row's nine quad fields are ticked as usual. This settles
  nothing here.
- **Waits:** REQ-TOOL-011 carries `rq: RQ-275`, for the quad fields row's five flags only, so TASK-M1-19's PR, which
  closes REQ-TOOL-011, does not merge until this entry is ruled (`plan/OPERATIONS.md` § "Autonomy (R-369)": a PR
  merges once "it waits on no open `REVIEW_QUEUE.md` entry"). With it wait the M1 exit and the chain behind
  TASK-M1-19 that RQ-273 listed (TASK-M2-25 and its dependents), which is behind the M1 exit anyway. TASK-M1-13
  (REQ-TOOL-026), merged, is not reopened.

---

## RQ-276: render_contract Part 6's structural-overlays row lists a "visible-set" and a "locked/stale" overlay that nothing defines or builds; render_gui_spec §12.1 calls them Tier 3, and TASK-M5-29, which builds Tier 3, names neither *(silence and placement, R-72, REQ-TOOL-011, REQ-TOOL-026, REQ-RENDER-055, REQ-PERF-079; context: TASK-M1-19, TASK-M1-13, TASK-M5-29, TASK-M8-08)*

*Found in TASK-M1-19's read-only pre-flight. Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)" (line 316), the
    structural overlays row: "Structural overlays (quadtree boundaries, active leaf outlines, fallback tint, pending
    hatch, visible-set, locked/stale) | post node + quad + `ctx.uv` | the quad/instance render path and cache
    behaviour".
  - `docs/gui/principia_render_gui_spec.md` § "12.1 Structural overlays and tile debug shaders" (lines 893–895): "The
    rest of that render-contract list (**fallback tint**, **pending hatch**, **visible-set**, **locked/stale**) is
    Tier 3 by this test: those are *values attached to a quad*, not functions of position within it." Its tier table's
    Tier 3 examples are "refine-decision, dirty, resident, generation, budget, ancestor gap", neither overlay among
    them.
  - `plan/tasks/M5/TASK-M5-29.md` ("Tier 3 tile debug shaders and the quad-state transition table"; REQ-RENDER-055,
    REQ-RENDER-056, REQ-TOOL-116) builds the Tier 3 path and names no overlay; its note: "Its per-visible-quad record is
    REQ-PERF-079's (TASK-M8-08)", whose statement is "The Tier-3 tile-debug buffer's per-visible-quad record must be
    defined in render_gui_spec §12.1 by the task that needs it".
  - `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata,
    not payload)" lists the overlays "quadtree boundaries / leaf outlines" ("active leaf set correct; pinned chain never
    evicted") and "fallback ancestor tint / pending hatch", and no visible-set or locked overlay. REQ-TOOL-026 (M1,
    TASK-M1-13, merged) names "leaf outlines, fallback tint and pending hatch", which TASK-M1-13 built as post
    occupants reading `ctx.quad` (RQ-239); REQ-TOOL-011 (M1, TASK-M1-19) covers "structural overlays".
  - "stale": `docs/design/principia_dd_generation_root.md` § "3.7a `RenderQuad` — the CPU-written quad record":
    "`quad_state` | u32 | 0 loaded · 1 pending · 2 refinable · 3 terminal · 4 stale", which the quad-state view
    `s_state` (`crates/render/shaders/wgsl/frag/debug/s_state.wgsl`, REQ-TOOL-026) already draws as a discrete colour.
  - "locked": defined nowhere as a quad property. Elsewhere the corpus's "locked" is the chart lock
    (`docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)", "locked mode") and locked debug presets
    (`docs/gui/principia_render_gui_spec.md` § "11. Presets = whole graphs"); the caching contract's per-quad
    exemption is "pinned" (`docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute
    tier, not a priority weight": "the baseline cover and its ancestor chain for the visible region, **plus the
    backdrop identity's leaf cover (Part 5)**, are pinned"), and no text links it to "locked".
  - "visible-set": defined nowhere as an overlay. `RenderQuad` is written "one per visible quad" (§3.7a), and the
    caching contract speaks of "the visible region"; no text says what a visible-set overlay draws.
  - Search at origin/main 55513b7b: "visible-set" occurs only at render_contract:316 and render_gui_spec:894, and
    "locked/stale" only there; no requirement in `plan/requirements.yaml` and no task file in `plan/tasks/` names either
    (outside `docs/archive/`, `docs/reference/` and `workbench/`, record only per R-159).
- **Silence (and a placement question):** the row lists two overlays that no doc defines (what a visible-set overlay
  marks, what "locked" means for a quad), no requirement names, and no task builds. render_gui_spec §12.1 places them in
  Tier 3, which TASK-M5-29 builds at M5 with its record defined at M8 (REQ-PERF-079), but neither task names them; the
  other two overlays that sentence placed in Tier 3, the fallback tint and the pending hatch, were built at M1 as post
  occupants under RQ-239. "stale" already has a reading as `quad_state` 4, drawn by `s_state`; "locked" has none. The
  row's other overlays (quadtree boundaries, active leaf outlines, fallback tint, pending hatch) exist.
- **Options seen:**
  - (a) Define both now (R-72, a definition requirement) and build them in M1, as TASK-M1-19's deliverable or a new
    task's, reading `ctx.quad` as the fallback tint and pending hatch do.
  - (b) Move both to TASK-M5-29's Tier 3 path, under a named requirement that defines them (R-72); REQ-TOOL-011's
    checklist then cites that requirement for the two cells, as it cites REQ-RENDER-027 for the DECODE row.
  - (c) "stale" is covered by `s_state`'s `quad_state` 4, ticked against it; "locked" is dropped (a ruling) or defined
    (R-72, under (a) or (b)).
  - (d) Drop both from Part 6's row (a ruling; render_contract Part 6 and render_gui_spec §12.1 then conformed).
    Dropping is the human's call only.
  - The options may be taken per overlay.
- **Applied meanwhile:** TASK-M1-19 is built with every other row of Part 6's table; its PR's checklist marks the
  visible-set and locked/stale cells of the structural overlays row as waiting on RQ-276. The row's other four
  overlays are ticked as usual. This settles nothing here.
- **Waits:** REQ-TOOL-011 carries `rq: RQ-276`, for those two overlays only, so TASK-M1-19's PR does not merge until
  this entry is ruled (`plan/OPERATIONS.md` § "Autonomy (R-369)"), with the M1 exit and the chain behind TASK-M1-19, as
  under RQ-275. TASK-M5-29 does not wait: it builds the Tier 3 path under every option, and (b) needs it to run; it is
  named, not marked. TASK-M1-13 (REQ-TOOL-026), merged, is not reopened.
