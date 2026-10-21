# TASK-M2-43 — The shape sphere chart is the whole sphere: the signed decode, the held momentum unmirrored below the equator, the signed lookup, the spherical map's system_image one-to-one, and the tests that asserted the 2-to-1 cover (R-544) ⚑

- **Milestone:** M2
- **Closes:** REQ-CHART-081, REQ-CHART-082, REQ-ENC-034
- **Depends on:** TASK-M2-08, TASK-M2-14, TASK-M2-17
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~350 lines

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
- **The mirror test** (REQ-CHART-082): a test that classifies the chart's held coordinates as mirror-equivariant or
  not, and in the first case checks mirror-invariant fields equal and signed fields opposite across the equator, in the
  second reports "does not apply" and asserts nothing.

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
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `decisions.md` § "R-178 — The shape-sphere round trip composes Φ alone *(closes A3)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"

## Deliverables
- `crates/kernel/src/canon.rs`: the shared `C` gains a way to run with step 3 not taken (a parameter or a second entry
  over the same steps, never a copy of the steps); `crates/kernel/src/chart/shape_sphere.rs`: `decode` calls it so,
  and `held_momentum` holds `p_fixed` unmirrored at every pixel (no `mirror_decision` there).
- `crates/kernel/src/encode/by_kind.rs` (the `ShapeSphere` coordinates, about line 450): the signed lookup, and the
  landing test ("Where possible", the module doc's step 4) comparing, on the shape sphere chart, against `C(x)` with
  step 3 undone on the full state (inverse_encode_contract Part 6; physics review 5603309678 of PR #273, P1).
- `crates/engine/src/chart_authoring/charts.rs` (about line 215): the spherical map's `system_image: Bijective`.
- Tests `shape_sphere_signed_decode`, `shape_sphere_mirror_test` and `encode_shape_sphere_signed` (`crates/kernel`),
  each with its controls.
- The implementer's tests that asserted the 2-to-1 cover, changed to R-544: `crates/engine/tests/well_posedness.rs`
  (about lines 1062–1064, `check_registered(shape(map), 2, SystemImage::NToOne(2))`, the spherical map `Bijective`);
  `crates/kernel/tests/shape_sphere.rs` (`shape_sphere_canonical_fold`, about lines 747–810, and the decode-canonical
  check about lines 945–980); `crates/kernel/tests/encode_by_kind.rs` (about lines 220–235, "the lower at its mirror
  (θ, π − φ) (R-141)"). None of these is a qa file.

## Acceptance tests
- `cargo test -p kernel shape_sphere_signed_decode` — REQ-CHART-081's verify: over random masses and pixels of both
  hemispheres, the poles and the equator included, `shape_vec` of the decoded state equals `n(θ, φ)` with its sign,
  within REQ-CHART-053's bound; the state is canonical in CoM, scale, boost and rotation; the held Jacobi momenta are
  equal at `(θ, φ)` and `(θ, π − φ)`; every other chart's decode is bit-identical to before; controls: R-141's gauge
  left in (C's mirror step run), a held momentum mirrored below the equator, and a decode skipping `C`, each fail
  (REQ-CHART-081).
- `cargo test -p kernel shape_sphere_mirror_test` — REQ-CHART-082's verify: released from rest and with a held
  momentum whose `y`-components are zero, the test reports mirror-equivariant; the run from `Π·x`, `Π` applied to a
  decoded state, is the run from `x` mirrored, bitwise; each pixel pair's shapes are mirror images,
  `shape_vec(Π·decode(θ, φ))` against `shape_vec(decode(θ, π − φ))` within REQ-CHART-053's bound, and its decoded
  states within `ε_phys` (REQ-ENC-024; physics review 5603529679 of PR #273, P6); on a named fixture (masses, pairs and horizon stated in the test, shown non-chaotic there by
  the pair's state separation staying below a stated bound) outcome and `t_end` are equal and `L_z` and the frame's
  winding opposite across each pixel pair; with a held
  `p_λ,y ≠ 0` it reports "does not apply" and asserts nothing; controls: the gauge left in, a momentum rule that turns
  `p_x` below the equator, and a test asserting the symmetry in the "does not apply" case, each fail (REQ-CHART-082).
- `cargo test -p kernel encode_shape_sphere_signed` — REQ-ENC-034's verify: a state and its mirror land at `(θ, φ)`
  and `(θ, π − φ)`; decode then lookup returns the pixel on both hemispheres; T1 holds bitwise as before for `z` and
  every other chart; landing: a lower-hemisphere `x` with `p_y ≠ 0` whose unmirrored Jacobi momenta and masses are
  the slice's lands on the chart and decodes back to `x` within `ε_phys`, `L_z`'s sign kept, and `Π·x` with the slice
  unchanged goes to latent `z`; controls: a lookup that leaves the mirror in, T1 applied to the shape sphere's
  coordinates, and a landing test comparing against the mirrored `C(x)` (every lower-hemisphere state to latent `z`),
  each fail (REQ-ENC-034).
- `cargo test -p engine chart_descriptors` — REQ-CHART-014's amended verify: the shape sphere's spherical map reports
  `Bijective` (R-544); the full-range Burrau chart `DoubleCover` (R-157).
- `cargo xtask gate shape-roundtrip` and `cargo test -p kernel shape_sphere` — REQ-CHART-020's amended verify: `Φ`
  alone round-trips on both hemispheres (R-178), and the decode's canonical form keeps the sign below the equator.
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
  - `crates/kernel/tests/qa_TASK-M2-08.rs:1109-1267` — `folded()`, `check_decode_canonical` (`λ̃_y ≥ −δ_λ`, "the
    fold of n") and `shape_sphere_decode_canonical_qa` with its controls: the decode's `ic` keeps `λ̃_y`'s sign and
    its shape is the signed `n(u, v)`; the `decode_unmirrored` control (expected "C's mirror is left undone"), which
    R-544 makes the passing behaviour, is replaced by a control that runs `C`'s mirror step (R-141's gauge), which must
    fail; the `decode_unfolded` control stands (ρ off `+x` still fails), its message reworded.
  - `crates/kernel/tests/qa_TASK-M2-08.rs:1270-1329` — `check_energy_norm`'s "its shape the fold of `n(u, v)`"
    (:1284-1285, `folded(n)`) and its control's `expected = "not the fold of n"` (:1318): the signed `n`.
  - The module doc of `qa_TASK-M2-08.rs` (:26-:30), where it describes those tests (`λ̃_y ≥ −δ_λ`, "the shape `C`'s
    fold of `n(u, v)`").
- **Not exceptions.** `qa_TASK-M2-08.rs:1006-1070` (`check_fold`, `shape_sphere_canonical_fold_qa`) tests `C` itself
  on `Φ`'s state, and `C` is unchanged: it stands. `qa_TASK-M2-08.rs` about :348 compares the inspector fixture
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
