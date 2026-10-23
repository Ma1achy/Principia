# TASK-M2-43 — The shape sphere chart is the whole sphere: the signed decode, the held momentum unmirrored below the equator, the signed lookup, the spherical map's system_image one-to-one, and the tests that asserted the 2-to-1 cover (R-544) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-081, REQ-CHART-082, REQ-ENC-034
- **Depends on:** TASK-M2-08, TASK-M2-14, TASK-M2-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~400 lines

## Goal
R-544 makes the shape sphere chart the whole sphere: pixel `(θ, φ)` decodes to the system whose **signed** `n̂` is
that point, and R-141's decode gauge goes for it. On main (TASK-M2-08, TASK-M2-14, TASK-M2-17) the chart is built to
R-141: `ShapeSphere::decode` runs `canon::canonicalise`, whose step 3 mirrors every lower-hemisphere state onto the
upper; `held_momentum` mirrors `p_fixed` where `canon::mirror_decision` says `C` will; the encode reads `shape_vec` of
`C(x)`, so a state and its mirror land on one point; and the descriptor is `SystemImage::NToOne(2)`. This task changes
the four, and the tests that asserted the 2-to-1 cover:

- **The decode** runs the one shared `C` (REQ-CHART-015) with its step 3 not taken: CoM at the origin and at rest,
  `I = 1`, `ρ̃` on `+x`, and `λ̃_y` keeping its sign. `C` stays one function, written once; only the shape sphere's
  call changes. On every other chart, and in encode, `C` is unchanged.
- **The held momentum** `p_fixed` is held in the shape sphere's own frame, `ρ̃` on `+x`, unmirrored, at every pixel of
  both hemispheres (chart_reference §3, the held values; R-538 J9). So the pixels `(θ, φ)` and `(θ, π − φ)` are mirror
  twins exactly when `p_fixed`'s `y`-components are zero (released from rest included), and otherwise two different
  systems: REQ-CHART-082's "does not apply" case.
- **The lookup** places a state at its signed `n̂`: the shape read from `C(x)` with step 3's mirror undone where
  `lookup_mirrored` is set. Inverse-encode's T1 stands for `z` and every other chart.
- **`system_image`**: the spherical map is `Bijective`; θ's period and the poles are R-407's types, not a fold. The
  exponential map's value is TASK-M2-28's (REQ-CHART-075); until it lands this task leaves that map's descriptor at
  what main has and says so in the PR.
- **The mirror test** (REQ-CHART-082), its static half: a test that classifies the chart's held coordinates as
  mirror-equivariant or not; in the first case it checks `Π·x` the exact mirror image of a decoded state, bitwise, and
  each pixel pair's shapes and states mirror images to the decode's rounding; in the second it reports "does not
  apply" and asserts nothing. The runs (the run from `Π·x` the run mirrored, bitwise; outcome, `t_end`, `L_z` and the
  frame's winding across pixel pairs) need the stepper, the wrapper and the outcome classifier (TASK-M3-02,
  TASK-M3-04, TASK-M3-12), which this task cannot reach. They are TASK-M3-38's (REQ-CHART-085), with the frame's
  winding defined there (REQ-CHART-086, R-72). Taking them from the M2 gate is a deferral, ruled by the human: R-579
  (23 Oct 2026, RQ-523 (a)). REQ-CHART-082 keeps at M2 exactly the classification, the "does not apply" case, the
  shape and state pair checks and the bitwise `Π·x` check, and no longer holds on RQ-523. *Was:* "Taking them from the
  M2 gate is a deferral that waits on the human's ruling, RQ-523; this split is a draft meanwhile, and REQ-CHART-082
  holds on it (`rq: RQ-523`)." and, before that, "This is not a deferral: the arms move to the task that can build
  them."
- **T2 and the `encode-eps-phys` gate on the shape sphere.** A lower-hemisphere pixel now decodes off `C`'s section,
  so `D(E(x)) = C(x)`, `Π·x` to rounding, and the T2 residual against `x` is of order 1. On the shape sphere chart, T2
  and the gate's rows compare `D(E(x))` against `C(x)`, as T3 does (inverse_encode_contract Part 1's T2; REQ-ENC-004,
  REQ-ENC-024). Every other chart's T2 and rows are unchanged. The gate's stored measurements are kept, not
  re-measured, with R-544 beside them.

## References
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart `(θ, φ)`"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "0.4 Canonicalisation `C`"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 1 — The physics: what the 8D actually is"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 2 — The decoder"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 5 — Inverse by axis kind (completing the four kinds)"
- `docs/contracts/principia_inverse_encode_contract.md` § "Chart-aware validation"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 1 — What encode is: a quotient map onto a section"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 4 — Conditioning: assert in physical units, never in z"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `decisions.md` § "R-178 — The shape-sphere round trip composes Φ alone *(closes A3)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-579 — The mirror test's run arms move to M3: REQ-CHART-082's runs become REQ-CHART-085, closed by TASK-M3-38 after TASK-M3-04 and TASK-M3-12; REQ-CHART-082 keeps at M2 the classification, the "does not apply" case, the shape and state pair checks and the bitwise `Π·x` check *(closes RQ-523; a deferral by ruling, from the M2 gate to M3; settles the split R-544's dated line drafted)*"

## Deliverables
- `crates/kernel/src/canon.rs`: the shared `C` gains a way to run with step 3 not taken (a parameter or a second entry
  over the same steps, never a copy of the steps); `crates/kernel/src/chart/shape_sphere.rs`: `decode` calls it so,
  and `held_momentum` holds `p_fixed` unmirrored at every pixel (no `mirror_decision` there).
- `crates/kernel/src/encode/by_kind.rs` (the `ShapeSphere` coordinates, about line 450): the signed lookup, and the
  landing test ("Where possible", the module doc's step 4) comparing, on the shape sphere chart, against `C(x)` with
  step 3 undone on the full state (inverse_encode_contract Part 6; physics review 5603309678 of PR #273, P1).
- `crates/engine/src/chart_authoring/charts.rs` (about line 215): the spherical map's `system_image: Bijective`.
- T2 and the gate on the shape sphere: `crates/kernel/tests/roundtrip.rs` (TASK-M2-16's file, not qa's), whose
  `t2_per_chart` measures `phys(x, D(E(x)))` with `x` the chart's decode, compares against `C(x)` on the shape sphere
  chart; the gate's pass condition, `crates/validation/src/roundtrip.rs` (`trip`'s residual, about line 147) and
  `crates/validation/src/eps_phys.rs` (`row`, `chart_row`), the same on the shape sphere's rows;
  `fixtures/gates/encode-eps-phys/` unchanged; `crates/ledger/src/constants.rs`'s `EPS_PHYS` doc (about lines
  864–870), its measurements kept with R-544 beside them (made under R-141's fold).
- Tests `shape_sphere_signed_decode`, `shape_sphere_mirror_test` and `encode_shape_sphere_signed` (`crates/kernel`),
  each with its controls.
- The implementer's tests that asserted the 2-to-1 cover, changed to R-544: `crates/engine/tests/well_posedness.rs`
  (about lines 1062–1064, `check_registered(shape(map), 2, SystemImage::NToOne(2))`, the spherical map `Bijective`;
  and lines 1097–1104, the negative control "the shape sphere read as bijective fails", inverted: the spherical map
  read as `NToOne(2)` fails); `crates/kernel/tests/shape_sphere.rs` (`shape_sphere_canonical_fold`, about lines
  747–810; `check_shared` and `shape_sphere_shared_decoder`, lines 821–925, whose `d.ic == canonicalise(&joined).ic`
  at 854–858 becomes `C` with step 3 not taken; and the decode-canonical check about lines 945–980);
  `crates/kernel/tests/encode_by_kind.rs` (about lines 220–235, "the lower at its mirror (θ, π − φ) (R-141)"). None of
  these is a qa file.
- Stale docs: `crates/kernel/src/chart/mod.rs:411` (`NToOne`'s "the shape sphere is 2-to-1 over its φ hemispheres
  (R-141)") and `crates/kernel/src/encode/by_kind.rs:449-451` (the `ShapeSphere` coordinates' "on the upper
  hemisphere, `C` having mirrored the lower onto it (R-141)").

## Acceptance tests
- `cargo test -p kernel shape_sphere_signed_decode` — REQ-CHART-081's verify: over random masses and pixels of both
  hemispheres, the poles and the equator included, `shape_vec` of the decoded state equals `n(θ, φ)` with its sign,
  within REQ-CHART-053's bound; the state is canonical in CoM, scale, boost and rotation; the held Jacobi momenta are
  equal at `(θ, φ)` and `(θ, π − φ)`; every other chart's decode is bit-identical to before; controls: R-141's gauge
  left in (C's mirror step run), a held momentum mirrored below the equator, and a decode skipping `C`, each fail
  (REQ-CHART-081).
- `cargo test -p kernel shape_sphere_mirror_test` — REQ-CHART-082's verify: released from rest, and with a held
  momentum with zero y-components, the test reports mirror-equivariant; Π·x, Π applied to a decoded state, has every
  rᵢ and pᵢ of x reflected through the x-axis, bit for bit, compared on their bit patterns (to_bits), signed zero
  included, so its L_z is −L_z(x) and its shape_vec is x's with w negated, bitwise where nonzero (an exactly zero sum
  takes the sign the rounding gives, so a zero compares by value); each pixel pair's shapes are mirror images,
  shape_vec(Π·decode(θ, φ)) against shape_vec(decode(θ, π − φ)) within REQ-CHART-053's bound, at every pixel; its
  decoded states agree within ε_phys (REQ-ENC-024's physical-units norm) on the pixels whose n̂ lies farther than a δ
  stated in the test from each of the three binary-collision landmarks (placed from chart_reference §3.1, as §3.4
  places them), the test giving ulp/δ ≤ ε_phys as δ's reason, since near a landmark λ̃'s direction is conditioned as
  ulp/‖ρ̃‖ (§3.2) and a rounding-level difference in n̂ turns the state by O(1); with a held p_λ,y ≠ 0 it reports
  "does not apply" and asserts nothing; controls: R-141's gauge left in (each pair's shapes equal, not mirror images),
  a momentum rule that turns p_x below the equator (the pair's states not mirror images), a test asserting the
  symmetry in the "does not apply" case (it fails on the pairs), a state comparison that includes BC₀₁'s pixel (s, t)
  = (0.5, 0.5) of an odd grid, and a configuration-only Π (positions reflected, momenta kept) applied to a state with
  p_y ≠ 0, the "does not apply" case's decoded state, on which Π's reflection, a property of Π alone, is checked
  though no symmetry is asserted (not every pᵢ reflected), each fail (REQ-CHART-082). *Was:* "bit for bit, so its
  `L_z` is `−L_z(x)` and its `shape_vec` is `x`'s with `w` negated, bitwise;" and "and a state comparison that
  includes BC₀₁'s pixel `(s, t) = (0.5, 0.5)` of an odd grid, each fail": on the mirror-equivariant states `p_y` is 0,
  so a configuration-only `Π` differed from `Π` only by `−0` against `+0`, which an f64 `==` accepts (qa review
  5635487490 of PR #276, Q4; applied per R-369, within what R-579 keeps at M2). The line is regenerated from the
  verify, word for word (applied per R-369); the paraphrase it replaces cited physics reviews 5603529679 (P6) and
  5603638748 (P9) of PR #273, which REQ-CHART-082's note keeps. The run arms (the run from `Π·x`, and fates, `L_z` and
  the frame's winding across pixel pairs on a named non-chaotic fixture) are TASK-M3-38's acceptance (REQ-CHART-085).
- `cargo test -p kernel encode_shape_sphere_signed` — REQ-ENC-034's verify: a state and its mirror land at `(θ, φ)`
  and `(θ, π − φ)`; decode then lookup returns the pixel on both hemispheres; T1 holds bitwise as before for `z` and
  every other chart; landing: a lower-hemisphere `x` with `p_y ≠ 0` whose unmirrored Jacobi momenta and masses are
  the slice's lands on the chart and decodes back to `x` within `ε_phys`, `L_z`'s sign kept, and `Π·x` with the slice
  unchanged goes to latent `z`; controls: a lookup that leaves the mirror in, T1 applied to the shape sphere's
  coordinates, and a landing test comparing against the mirrored `C(x)` (every lower-hemisphere state to latent `z`),
  each fail (REQ-ENC-034).
- `cargo test -p engine chart_descriptors` — REQ-CHART-014's amended verify: the shape sphere's spherical map reports
  `Bijective` (R-544), its inverted control (the map read as `NToOne(2)`) failing; the full-range Burrau chart
  `DoubleCover` (R-157).
- `cargo test -p kernel t2_per_chart` — REQ-ENC-004's amended verify: T2 per chart in physical units, on the shape
  sphere chart against `C(x)` over both hemispheres, every other chart against `x` as before; the mismatched-links
  control still fails on every chart.
- `cargo xtask gate encode-eps-phys` and `cargo test -p validation eps_phys_gate` — REQ-ENC-024's amended verify: the
  gate passes `interior.json` and fails `mismatched.json`, its shape sphere rows taken against `C(x)`, every row within
  the provisional 64 `ε_R`; the stored measurements unchanged, R-544 beside them.
- `cargo xtask gate shape-roundtrip` and `cargo test -p kernel shape_sphere` — REQ-CHART-020's amended verify: `Φ`
  alone round-trips on both hemispheres (R-178), the decode's canonical form keeps the sign below the equator, and the
  shared decoder's `ic` is `C` of the joined blocks with step 3 not taken.
- Review checklist (physics reviewer): the decode, the held momentum and the lookup against R-544 and its "Applied per
  R-369" decisions; `checks/r544_check.py` (`python3 -I`) is the port's model of checks 1 to 3.

## Notes
- **⚑ Physics review is required** (R-544 is in the physics group); it does not merge on the code reviewer's word
  alone.
- **qa-file exceptions (R-544 forces them; applied per R-369).** R-544 changes behaviour that merged qa files assert.
  In this task's qa commit, qa changes, and only as far as R-544 needs, each assertion below; the implementer does not
  edit them (R-290); the PR lists each `M` with this reason, and the code reviewer confirms no other assertion is
  weakened:
  - `crates/engine/tests/qa_TASK-M2-14.rs:883` — `check_descriptor(shape(ShapeMap::Spherical), plain(NToOne(2), false))`
    becomes `Bijective` (the comment at :882 with it). Line 884, the exponential map, is TASK-M2-28's exception.
  - `crates/engine/tests/qa_TASK-M2-14.rs:16`, the module doc's "the shape sphere is n-to-1 with n = 2 (R-141)": the
    spherical map bijective, the exponential map's value TASK-M2-28's (a doc line, no assertion; code review 5603292175
    of PR #273, F3).
  - `crates/kernel/tests/qa_TASK-M2-08.rs:917-925`, in `shape_sphere_roundtrip_qa_poles` — "C folds L⁻ onto L⁺
    (R-141): the decode's canonical ic is at L⁺ from either pole" becomes: the decode's `ic` at each pole is at that
    pole, `L⁻` at `t = 0`.
  - `crates/kernel/tests/qa_TASK-M2-08.rs:1109-1268` (`mod decode_unmirrored`'s closing brace is :1268) — `folded()`, `check_decode_canonical` (`λ̃_y ≥ −δ_λ`, "the
    fold of n") and `shape_sphere_decode_canonical_qa` with its controls: the decode's `ic` keeps `λ̃_y`'s sign and
    its shape is the signed `n(u, v)`; the `decode_unmirrored` control (expected "C's mirror is left undone"), which
    R-544 makes the passing behaviour, is replaced by a control that runs `C`'s mirror step (R-141's gauge), which must
    fail; the `decode_unfolded` control stands (ρ off `+x` still fails), its message reworded.
  - `crates/kernel/tests/qa_TASK-M2-08.rs:1270-1329` — `check_energy_norm`'s "its shape the fold of `n(u, v)`"
    (:1284-1285, `folded(n)`) and its control's `expected = "not the fold of n"` (:1318): the signed `n`.
  - The module doc of `qa_TASK-M2-08.rs` (:26-:30), where it describes those tests (`λ̃_y ≥ −δ_λ`, "the shape `C`'s
    fold of `n(u, v)`").
- **Not exceptions.** `qa_TASK-M2-08.rs:1006-1070` (`check_fold`, `shape_sphere_canonical_fold_qa`) tests `C` itself
  on `Φ`'s state, and `C` is unchanged: it stands. `qa_TASK-M2-08.rs` at :343 compares the inspector fixture
  `fixtures/ic_inspector/shape_vec.json` on canonical states (R-544's "results measured under one hemisphere"): it
  stands. If the implementer's change to `canon.rs` would make either fail, stop and queue it rather than widening the
  exceptions.
- **Stop and queue** if any consumer of the shape sphere's decoded state needs `λ̃_y ≥ −δ_λ` (a section condition
  that R-544 drops for this chart), rather than restoring the mirror for it.
- REQ-CHART-014 (TASK-M2-14) and REQ-CHART-020 (TASK-M2-08) are closed by merged tasks; their verifies are amended by
  R-544 and this task re-runs them. TASK-M2-28 depends on this task and is redone on top of it (its PR #271, built under
  R-141's and R-503's fold).
- **Review round 1 of PR #273 (applied per R-369).** The landing test on the shape sphere chart (physics P1); the
  mirror test's precision, bitwise for the exactly mirrored state and on a named non-chaotic fixture across pixel pairs,
  under §4's "bitwise where the integration is mirror-exact, else to rounding" (physics P2); `qa_TASK-M2-14.rs:16`
  added to the exceptions (code F3).
- **Review round 3 of PR #273 (physics review 5603638748, P9; applied per R-369, a test detail under §4's "else to
  rounding").** BC₀₁ at `n = (−1, 0, 0)` is the pixel `(s, t) = (0.5, 0.5)` of every odd grid, its own pair; there the
  shapes agree to `1e−16` but the states differ by `0.89`, `λ̃`'s direction being ill-conditioned at `ρ̃ → 0`. The
  states are compared away from the three landmarks, the shapes at every pixel.
- **`canon.rs`, kept minimal, ahead of TASK-M2-47 (applied per R-369).** TASK-M2-47 (open PR #274, the 21 Oct batch rulings) rebuilds
  `C`'s pair differences in `canon.rs`; this task goes first. Add one private function holding the shared steps with a
  `mirror: bool`, and either `canonicalise` and a second public entry over it, or the shape sphere's call through it;
  never a copy of the steps. Don't change `canonicalise`'s signature (about 80 test call sites in 15 files), and leave
  `similarity`, `centre_of_mass`, `inertia` and `mirror_decision` untouched.
- **The pre-flight (applied per R-369 except (A)'s split, 21 Oct 2026; R-544's dated line).** (A) REQ-CHART-082's
  run arms need real
  integrator runs, and main has no stepper (TASK-M3-02), wrapper (TASK-M3-04) or outcome classifier (TASK-M3-12):
  they move to REQ-CHART-085, closed by TASK-M3-38 (after TASK-M3-04, TASK-M3-12 and this task), with REQ-CHART-086
  defining the frame's winding (R-72; chart_reference §3.3's "the signed ones (`L_z`, the winding)"), as a draft that
  waits on RQ-523, the deferral being the human's to rule (code review 5605370280 of PR #276, F1). *Was:* "(applied
  per R-369, 21 Oct 2026; R-544's dated line)" and "Not a deferral: the arms move to the task that can build them."
  RQ-523 is ruled (a) by R-579 (23 Oct 2026): the split stands as drafted and is no longer a draft.
  (B) T2 and the `encode-eps-phys` gate compare against `C(x)` on the
  shape sphere chart, a mechanical consequence of R-544 (inverse_encode_contract Part 1's T2 premise amended;
  REQ-ENC-004 and REQ-ENC-024 amended). (C) `shape_sphere.rs`'s shared-decoder check, `well_posedness.rs`'s inverted
  control and the stale docs at `chart/mod.rs:411` and `by_kind.rs:449-451` added to the sites; the qa exception
  range corrected to :1109-1268 and the fixture line to :343. (D) The `canon.rs` note above.
