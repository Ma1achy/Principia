# TASK-M8-125 — Locking doesn't move the view: the pin at the clicked pixel, tilt and zoom about it, centre on pin and the auto-snap setting, exact on nonlinear charts or saying why (R-866, R-863 stage 1)

- **Milestone:** M8
- **Closes:** REQ-GUI-425, REQ-GUI-429
- **Depends on:** TASK-M4-37, TASK-M8-124
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-866, the owner's words: "locking must not move the view." A lock pins the clicked pixel where it is, its initial
in-plane excursion that pixel's offset from the view's centre, and edits neither `z₀`, the basis nor a nonlinear
chart's window; tilt and zoom turn about the pin, so the pinned pixel stays fixed on screen. "Centre on pin" is an
optional action, and a setting, off by default, snaps automatically. On nonlinear charts, where locking today re-centres
the window to compute the pin, the pin is exact, or the task says why it can't be.

## References
- `decisions.md` § "R-866 — Locking must not move the view: the pin is set at the clicked pixel, its initial in-plane excursion that pixel's offset from the view's centre, and tilt and zoom turn about the pin, so the pinned pixel stays fixed on screen; an optional "centre on pin" action and a setting to snap automatically; exact on nonlinear charts, or the task says why *(amends R-69, R-473 J7 and render_gui_spec §G4)*"
- `decisions.md` § "R-69 — What is undoable *(closes the step-6 open question)*"
- `decisions.md` § "R-473 — Tilt and slice on every chart ⚑: the tilt is a rotation in the 3-space of the two screen axes and the hidden target, applied to the chart's own parameters, `(a, b, d) = R(τ₁, τ₂)·(u, v, 0)`, `a` and `b` through the chart map exactly and `d` added to the hidden coordinate at the slice centre, untilted bitwise today's; a mass, and every linked coordinate, is tilted in its link's pre-image; the units are queued (RQ-375); the slice moves any hidden coordinate *(extends R-457)*"
- `decisions.md` § "R-863 — When the chart-composition design's tasks are released, they are built in five stages, each merging only with the previous stage's tests green: the path object with today's operations bitwise through it; locked paths; the primitive registry and composed axes; morph families, easing and Sequence segments; time-axis charts and the symmetry filter *(the build order of PR #422's tasks; refines no ruling)*"
- `decisions.md` § "R-864 — The chart-path work is one object: core types designed first as one module, a type sketch with a one-page rationale reviewed by physics and code and brought to the owner before any implementation; today's operations re-expressed as named families of it and their old routes deleted; bitwise reproduction first; special cases are named families, never if-branches; the kernel/engine split stays; restructuring `engine/nav`, `engine/lock.rs` and `engine/chart_authoring` is approved *(the architecture of PR #422's chart-path work; refines no ruling on `main`)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/gui/principia_render_gui_spec.md` § "G4. Lock — the reticle and the pin (`08_lock.png`)"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/design/principia_chart_space_model.md` § "0. Revision after #422's review (7 Nov): the corpus wins where it measured"
- `decisions.md` § "R-899 — The chart-path brief's nine questions: the lock test reads the recorded state at the pin; snap-back undoes the slice only, auto-snap off; jumps under a lock kept and labelled until the sliding anchor exists; the pin's screen position stored, "assumed" at the centre where none can be derived; a pivot record in the view, untilt (a1), amending R-473; points of interest one task in stage 3; their star filled within the lock's tolerance, the off-slice distance shown; marks mid-morph to pixel resolution, the filter off, labelled; the block line built in stage 2 *(amends R-473, and the snap-back of chart_decoder_contract Part 4 and design § 17.8)*"

## Deliverables
- The lock as a family of the chart-path object (R-864): locking edits the lock group alone; "centre on pin" one
  navigation edit; the setting applying it in the same command as the lock, one history entry; tilt and zoom turning
  about the pin on the latent chart and on curved charts (R-473 J7 as R-866 amends it), in `engine` and `gui`.
- On nonlinear charts (mixed and invariant): the pin held without re-centring the window, its construction written into
  chart_decoder_contract Part 4 in place of "The window is re-centred", under physics review (R-72); exact, or the reason
  it can't be, stated in the PR and taken to the owner before anything inexact merges, its bound an R-71 calibration
  (REQ-GUI-429).
- The lock badge's **centre on pin** and the setting, **Centre on pin when locking**, off by default, a preference outside
  `SimConfig` and `RenderState` (render_gui_spec § G4).

## Acceptance tests
- `cargo test -p engine lock_moves_no_view` and `cargo test -p gui lock_moves_no_view` — REQ-GUI-425's verify: a lock at
  an off-centre pixel on the latent chart, a mixed chart and an invariant chart leaves `z₀`, the basis and the window bit
  for bit; on the latent chart its `z_locked` is today's point at that pixel bit for bit; the pinned pixel decodes the
  clicked state after tilts and zooms about it, bit for bit, or within REQ-GUI-429's calibrated bound where the task
  states why it can't be exact; centre
  on pin brings the pin to `(½, ½)` as one undoable edit; the setting on gives lock then centre on pin in one history
  entry, and is off by default; controls: a lock that re-centres fails, a tilt about the centre with an off-centre pin
  fails (REQ-GUI-425).
- `cargo test -p engine -- path_lock_bitwise path_tilt_bitwise` — TASK-M8-124's tests re-run, passing, the lock's baseline re-recorded only where
  this ruling changes it and each such change listed in the PR (REQ-GUI-425).
- Review: the nonlinear chart's construction in chart_decoder_contract Part 4, exact or with its stated reason, approved
  by the physics reviewer (REQ-GUI-425).
- Proposal, only if the pin can't be exact on a nonlinear chart: the bound with its evidence and the reason, checked by
  the physics reviewer and ruled by the owner before this task merges, recorded in decisions.md; never a bound chosen in
  the PR (R-71; REQ-GUI-429). If the pin is exact, the PR shows it bit for bit and proposes no value.
- `cargo test -p engine lock_moves_no_view` — snap-back after a slice under an off-centre lock clears the hidden excursion bit for bit, the pin's pixel unmoved; snap-back then centre on pin equals `z₀ ← z_locked` bit for bit (R-899 Q2 (a); REQ-GUI-425).

## Notes
- Added by R-866 (8 Nov 2026), the owner's reply 2 to the code map of that day: "Confirm this works on nonlinear charts,
  where locking currently re-centres the window to compute the pin; if it can't be exact there, say why."
- **Build order (R-863), applied per R-369 (R-866):** stage 1, after TASK-M8-124's bitwise re-expression, which records
  today's lock as its baseline; TASK-M8-117, stage 2, depends on this task.
- The shape sphere becomes lockable once the anchor is a stored signed state (R-867, TASK-M8-117) or the mirror step is
  gone (R-865, TASK-M2-57); until then this task doesn't owe a shape-sphere lock.
- What stage 2's test reads once the pin is off the centre is RQ-749's; this task builds R-866 either way.
- TASK-M8-07 builds the lock's look on the mock engine and keeps REQ-GUI-099, which R-866 rewords: the reticle at the pin
  and the badge's centre on pin.
- *Physics review 5817348709 of PR #449, P5:* an inexact pin's bound is a calibration ruled by the owner before merge, not the task's choice. Holding the pin exactly looks achievable: evaluated as pivot + R·(offset), the pin's chart parameters come out bitwise, pin + R·0 = pin.
- *8 Nov 2026 (R-899):* The brief's Q2 (a): snap-back undoes the slice only, clearing the hidden excursion and leaving the pin where it is on screen; centre on pin clears the in-plane part; the two together are `z₀ ← z_locked` bit for bit. Auto-snap stays off by default (R-866). RQ-749 is ruled (R-897): stage 2 reads the recorded state at the pin; the note above is answered.
