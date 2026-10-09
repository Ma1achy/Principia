# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

---

*Found applying R-395, from PR #160's evidence (physics review 5438875182; qa's
`qa_uv_preset_reconstruction_global_form_depth_sweep`). Nothing is chosen; it goes to the human with REQ-TOOL-152's
calibration at the M1 gate.*

## RQ-258: R-395's "no banding with absolute coordinates up to ℓ_switch" against REQ-TOOL-152's proposed bound at a non-dyadic N *(calibration, physics, R-395, R-90, REQ-TOOL-019, REQ-TOOL-152, TASK-M1-16, M1 gate)*

- **File, section:**
  - `decisions.md` § "R-395 — REQ-TOOL-019's "no banding" holds with absolute coordinates up to ℓ_switch, checked at
    the M1 gate, and through the per-quad local coordinates beyond it, at M5 and M6 *(closes RQ-242)*", the human's words:
    "REQ-TOOL-019 means no banding with absolute coordinates up to the deep-zoom switchover (ℓ_switch, R-90) … The M1
    gate checks the first half."
  - `decisions.md` § "R-90 — The decoder switchover trigger *(closes RQ-41)*": "Switch to the linearised decoder when
    the full decoder's adjacent samples give bitwise-identical ICs, with ℓ_switch = 20 as an upper bound (whichever
    comes first)."
  - `docs/design/principia_memory_tiers.md` § "4. The six quality tiers", the tier table: `N~` is 8 (Potato, Low) or
    16 (Medium to Extreme), above which the section says "`N~`/`depth~` are indicative"; and § "5. Controller levers,
    ranked by impact": "**Custom mode** exposes `render_scale` (0.25–2.0; …), `N`, `MAX_REL_DEPTH`, E, and FTLE
    directly", with no range given for `N`.
  - `plan/requirements.yaml`, REQ-TOOL-152: the bound on "how far the adjacent-sample deltas … may depart from the exact
    step 2h/N", "confirmed by the human at the M1 gate". PR #160 proposes `BANDING_BOUND = 1/16`
    (`crates/render/src/coords.rs`, marked proposed, R-71).
- **What was measured** (PR #160, and the same f32 arithmetic recomputed for this entry), the absolute coordinate
  `c + h·(2t − 1)` in f32 near `u ≈ 0.6`, as the departure `max |Δ − 2h/N| / (2h/N)` at ℓ = 16 to 22:
  - N = 6: 1/64, 1/32, 1/16, 1/8, 1/4, 1/2, 1.
  - N = 12: 1/32, 1/16, 1/8, 1/4, 1/2, 1, 2.
  - N = 8: 0 to ℓ = 20, then 1. N = 16: 0 to ℓ = 19, then 1.
- **The conflict:** at a power-of-two N the sum is exact until adjacent samples collapse to one coordinate, and a
  collapse makes the full decoder's adjacent ICs bitwise-identical, so R-90's switchover fires first: R-395's first
  half holds. At a non-dyadic N the departure grows by graded steps, so at 1/16 it reads "banded" from ℓ = 19 at N = 6
  and from ℓ = 18 at N = 12, before ℓ_switch = 20 and before any collapse that would fire the switchover. There, R-395's
  first half and the proposed bound cannot both hold.
- **Options seen:**
  1. N is a power of two. The named tiers' indicative N already are (8 and 16); Custom mode would offer only powers of
     two, a range the corpus does not give it now.
  2. The bound is set at the gate so that no N the product offers exceeds it before ℓ_switch. That needs a cap on
     Custom's N as well, which the corpus does not give: the bound needed grows with N, and departures exceed it with
     no collapse (N = 12: 1/4 at ℓ = 19 and 1/2 at ℓ = 20; N = 24: 1/4 at ℓ = 18 and 1/2 at ℓ = 19, collapsing only at
     ℓ = 20; N = 10: 1/4 at ℓ = 19 and 3/8 at ℓ = 20). Just before a collapse, with a step of 1 to 2 ulp, the departure
     tends to 1, so this option is a bound and a cap on N together.
  3. The switchover also fires where the absolute coordinate's departure first exceeds the bound, which adds a trigger
     to R-90's two.
  4. R-395's first half is checked at the named tiers' N only, and a Custom non-dyadic N is outside it.
- **Applied meanwhile:** TASK-M1-16's sweep covers N = 8 and N = 16, where the ruling holds with no choice; a
  non-dyadic N joins it once this is ruled. REQ-TOOL-152 carries `rq: RQ-258`.
- **Waits:** the M1 gate's confirmation of REQ-TOOL-152. Nothing is blocked before it.

---

*Found by the gui review of PR #172 (TASK-M1-09; review 5468425675, its flag F2). Nothing is chosen; it is a look
question, which the human rules (R-390), and it goes to the human with REQ-COL-053's calibration at the M1 gate. It
doesn't block PR #172.*

## RQ-259: the neutral "not yet" grey sits on the greyscale ramp, which colour_composition §1.2 makes the default for magnitude and diagnostic fields *(look, calibration, R-280, R-96, R-71, R-390, REQ-COL-053, REQ-COL-016, TASK-M1-09, TASK-M7-05, TASK-M7-09, M1 gate)*

- **File, section:**
  - `decisions.md` § "R-280 — An unset `d_min` renders in the neutral "not yet" grey *(closes RQ-170)*", the human's
    words: "RQ-170: option 3: an unset d_min renders in the neutral "not yet" style, the same grey as running samples
    (R-96). Not the invalid hatch, not the top of the ramp." Its *Applied* line: "It doesn't draw the invalid hatch,
    which stays NaN's (PIT-8), and it doesn't place +inf on the ramp."
  - `decisions.md` § "R-96 — Colour and GUI definitions *(closes RQ-52 and RQ-54, definitional parts)*": "`running`
    shows neutral grey; `sim_failed` shows the invalid colour."
  - `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`": the payload
    sources include `d_min` ("**payload** — any per-pixel kernel output: `state`, `ftle`, `energy_drift`, `Lz_drift`,
    `diffusion`, `d_min`, …"), and "**Default ramps by field role.** Signed fields (energy, L_z, drifts) default to
    diverging-through-neutral so the zero-crossing is a legible contour; positive fields to sequential; angles to
    cyclic. **Magnitude / diagnostic fields default to *greyscale*, not a sequential LUT**", and "Every default here is
    customisable".
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets": "A field view *is* `colour =
    FieldRamp{field, recommended ramp}, brightness = None, chain = []`."
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library": "`ramp_grey(t)` is OKLab `(t, 0, 0)`,
    `t` clamped: the colour slot's None grey (colour_composition §4.1)."
  - `plan/requirements.yaml`, REQ-COL-053: "The sRGB value of the neutral grey the outcome palette shows for the running
    state must be calibrated: proposed with its evidence by the task that needs it, checked by a reviewer, confirmed by
    the human at the M1 gate and recorded in decisions.md." Its verify asks for the grey's "separation from bounded
    #141418, degenerate #ECECF0 and the other seven classes, from the invalid pattern's two colours (REQ-COL-055) and
    from REQ-COL-062's two triple-outcome swatches once proposed", and from no ramp. PR #172 proposes `#4E4E4E`, OKLab
    lightness 0.424 (R-71).
  - `plan/requirements.yaml`, REQ-COL-016: "Default ramps must follow field role: signed fields diverging through
    neutral, positive fields sequential, angles cyclic, and magnitude/diagnostic fields greyscale, …"
  - `plan/tasks/M1/TASK-M1-09.md` § "Deliverables": "the minimal `FieldRamp` over one `ScalarField` …, a `lin`
    `Compaction`, viridis, and the invalid-colour lane …, instanced on two fields: the word `length`, … and `d_min`,
    whose NaN maps to the invalid pattern and whose unset value is drawn in R-280's grey, never the invalid pattern …
    TASK-M7-05 and TASK-M7-09 extend these files."
  - `plan/tasks/M7/TASK-M7-05.md` § "Goal": "default ramps follow field role (signed → diverging through neutral,
    positive → sequential, angle → cyclic, magnitude/diagnostic → greyscale)"; § "Deliverables": "`field_ramp.rs` —
    extends TASK-M1-09's minimal `FieldRamp` (one `ScalarField`, a `lin` `Compaction`, viridis and the invalid lane;
    RQ-232) to the `FieldRamp{field, ramp | compaction}` node" and "`crates/render/src/colour/defaults.rs` — the
    default-ramp registry keyed by the ledger `scale` and field role."
  - `plan/tasks/M7/TASK-M7-09.md` § "Goal": "FieldRamp's ScalarField sources, each returning (value, valid) through
    `ctx`: payload fields (through the generated accessors), …"; § "Deliverables": "`scalar_field.rs` — the ScalarField
    node variants and their codegen, extending TASK-M1-09's minimal `ScalarField`".
- **The conflict:** a neutral grey has OKLab a = b = 0, so every neutral grey is a point of `ramp_grey`: the proposed
  `#4E4E4E` is `ramp_grey(0.424)`, and whatever value the M1 gate confirms will be `ramp_grey(L)` for its lightness L.
  On a `d_min` field ramp drawn in greyscale, the unset value and a valid `d_min` at that lightness are the same
  colour, though R-280 keeps the unset value off the ramp. §1.2 gives `d_min` no role: as a magnitude or diagnostic
  field its default is greyscale; as a positive field it is sequential, and greyscale is still one customisation away
  ("Every default here is customisable"). REQ-COL-053's verify measures the grey against the classes and the hatch,
  not against any ramp.
- **Today:** nothing built collides. M1's ramps are viridis (RQ-231 item 3; TASK-M1-09's `FieldRamp`), and the
  proposed grey's nearest approach to viridis is 0.101 in OKLab (the gui review). The collision appears once TASK-M7-05's
  default-ramp registry or a user's customisation puts a field ramp that shows the unset grey on greyscale.
- **Options seen:**
  1. The "not yet" grey is not on the grey axis: a slightly tinted grey, off `ramp_grey`. It changes R-96's and R-280's
     "neutral", so it needs a ruling, and REQ-COL-053's separations are measured again.
  2. `d_min` is a positive field, so its default field ramp is sequential, and REQ-COL-053's verify adds the grey's
     separation from the ramps a field that shows it may default to. A user-chosen greyscale still collides.
  3. Where a field ramp that shows the unset grey is on greyscale, by default or by choice, the unset value is drawn in
     a "not yet" style that is not a flat colour, a pattern distinct from the invalid hatch, or the ramp's range is kept
     clear of the grey's lightness.
  4. The collision stands, documented: on a greyscale ramp the unset value reads as a value of the grey's lightness,
     told apart only through the `state` view or the debug view.
- **Applied meanwhile:** nothing is chosen. PR #172 proposes the grey as its task says. REQ-COL-053 and REQ-COL-016
  carry `rq: RQ-259`.
- **Waits:** the M1 gate's confirmation of REQ-COL-053 (option 1 changes the value), and TASK-M7-05's default-ramp
  registry and TASK-M7-09's sources (options 2 and 3). Nothing before the M1 gate is blocked.

---

*Found by the gui review of PR #172 (TASK-M1-09; review 5468425675, its flag F3). Nothing is chosen; it is a look
question, which the human rules (R-390). PR #172 follows RQ-231 as recorded in its task, and this doesn't block it.*

## RQ-260: signed numeric debug views on viridis (RQ-231 item 3) against colour_composition §1.2's diverging-through-neutral default for signed fields *(look, RQ-231, R-369, R-381, R-390, REQ-RENDER-022, REQ-COL-016, TASK-M1-09, TASK-M7-05)*

- **File, section:**
  - `docs/archive/review_queue/M0.md` § "RQ-231: the two-line template meets the ledger's scale metadata: log, cyclic
    and diverging fields, unbounded fixed ranges, which `ramp`, the drift views, and `RANGE_AUTO` as a uniform *(plan,
    TASK-M1-09, REQ-RENDER-022)*", "**Decided:** per R-369 (7 Oct 2026), by the orchestrator (not physics, not a
    value)": item 1, "`diverging` uses a symmetric range"; item 3, "`ramp` is `ramp_viridis`, `dbg_sentinel`'s ramp";
    item 4, "`energy_drift` and `Lz_drift` keep R-381's default, so TASK-M1-09 leaves them TASK-M1-08's view until
    TASK-M3-05."
  - `plan/tasks/M1/TASK-M1-09.md` § "Goal": "`raw` is the field's value compacted per its ledger scale, `ramp` is
    `ramp_viridis`, and `RANGE_AUTO` is a `u32` uniform in the view's header (RQ-231)"; § "Notes": "RQ-229 to RQ-233
    and RQ-241, decided per R-369 (7 Oct 2026): … the template's scales, ranges, ramp and `RANGE_AUTO` uniform
    (RQ-231)".
  - `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`": "**Ramp** (scalar
    → colour): `lut(name)`, `lerp(c0,c1)`, `diverging(c−,c0,c+)` (through a neutral)", and "**Default ramps by field
    role.** Signed fields (energy, L_z, drifts) default to diverging-through-neutral so the zero-crossing is a legible
    contour; positive fields to sequential; angles to cyclic."
  - `docs/design/principia_colour_composition.md` § "6. Debug views as presets": "A field view *is* `colour =
    FieldRamp{field, recommended ramp}, brightness = None, chain = []`. `f_edrift` = `energy_drift · diverging ·
    symlog`", and "As presets they inherit compaction override, palette swap, the post chain, and per-stage shader
    visibility for free."
  - `docs/contracts/principia_render_contract.md` § "Presentation layer (hand-written, small, reused by every debug
    view)": "`fn dbg_lin(x: f32, lo: f32, hi: f32) -> vec3f   // scalar, viridis ramp`"; "**`dbg_lin(x, lo, hi)`:**
    `ramp_viridis(range_norm(x, lo, hi, false, ·))`, the fixed range, clamped."; `dbg_sentinel`: "Any other value, a
    stored sentinel such as the word length's 127 included, shows as its literal value on the viridis ramp at `t = 0.5
    + 0.5·x/(1 + |x|)` (`dbg_literal`), … 0 maps to the middle"; and for the drift views: "`symlog`, the default: the
    compacted value takes `dbg_sentinel`'s place `t = 0.5 + 0.5·x/(1 + |x|)` on `f_edrift`'s `diverging(c−,c0,c+)`,
    `0` at the neutral `c0`, or on viridis through the palette swap (colour_composition §6)".
  - `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)": "per-field
    presentation metadata: `{scale: lin|log|cyclic, range, sentinel?, categorical_n?, tier_gate?}`. So the drift fields
    `energy_drift` and `Lz_drift` default to `diverging · symlog` with the field's floor (`eps_E`, `eps_L`), the preset
    `f_edrift`'s".
  - `plan/requirements.yaml`, REQ-COL-016: "Default ramps must follow field role: signed fields diverging through
    neutral, …"
- **The conflict:** RQ-231 item 3 puts every numeric debug view the template generates on viridis, the signed fields
  included: the ledger's `diverging`-scale `E_0` and `Lz_0` (`crates/ledger/src/payload.rs`:132–133, on item 1's
  symmetric range) and the signed `lin` fields (`C_ty`, `theta`, and the like). Their zero lands mid-viridis (teal),
  not at a neutral, so the zero-crossing is not the "legible contour" §1.2 gives as the reason for its default. The
  render contract's presentation layer draws its scalar helpers (`dbg_lin`, `dbg_log`, `dbg_sentinel`) on viridis,
  which supports viridis for debug views; §1.2 and §6 make a field view a `FieldRamp` with its recommended ramp, and
  give signed fields diverging-through-neutral, which the drift views follow (R-381). The corpus doesn't say which
  governs a signed field's debug view, and RQ-231 chose viridis by the orchestrator under R-369, though it is a look
  choice, which R-390 gives the human.
- **Options seen:**
  1. RQ-231 stands: every numeric debug view is on viridis, `dbg_sentinel`'s ramp; §1.2's role defaults govern field
     ramps (TASK-M7-05), and a debug view reaches a diverging ramp through the palette swap (§6).
  2. A `diverging`-scale field's debug view uses the diverging ramp through a neutral, as the drift views do. The
     template's `ramp` follows the ledger scale (viridis for `lin` and `log`, twilight for `cyclic`, diverging for
     `diverging`), and the diverging ramp's colours are needed by M1, before TASK-M3-05 defines the drift views'
     (render contract, presentation layer; REQ-TOOL-149).
  3. As option 2, for every signed field, those with a signed `lin` range included, which needs a signed-or-not mark in
     the ledger's presentation metadata that the corpus doesn't give.
- **Applied meanwhile:** TASK-M1-09's numeric debug views follow RQ-231 as recorded. REQ-RENDER-022 and REQ-COL-016
  carry `rq: RQ-260`.
- **Waits:** nothing is blocked. Under option 2 or 3, the template, its generated views and the `m1-numeric` goldens
  change in a follow-up task, and so does any later task that reuses RQ-231's choice; TASK-M7-05's default-ramp
  registry follows the ruling.
