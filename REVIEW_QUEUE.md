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
