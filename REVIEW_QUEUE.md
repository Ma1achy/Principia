# Review queue

Items I did not decide. Each gives the file, the section, and both versions where they disagree. The reviewer
or the human rules; I don't choose between them.

Only open entries are kept here (R-292). Once an entry has a ruling it moves, unchanged, to
`docs/archive/review_queue/`: `untangling.md` holds RQ-1 to RQ-128, and `M0.md` RQ-129 onward; each later
milestone gets its own file after its gate. Ids never change.

## RQ-341: The physical compass views need the decoder's Jacobian with respect to z along all eight latent directions, into Jacobi or shape-sphere coordinates, and its Gram area factor; the corpus's J_D is the derivative with respect to the chart's (u, v) into the particle frame, not yet built, and no Gram factor is defined for it *(silence and conflict with the ruling's words, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026). Nothing is chosen. For the human: R-452 says "use the existing J_D machinery"
and "the Gram-determinant area factor already defined for J_D", and neither exists in that form.*

- **File, section:**
  - `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision": "compute `J_D = ∂D/∂(u,v)` at the
    centre by **central differences in f64** (two decode evals per axis)"; § "Couplings to the contract", a: "The
    linearised object is the full chart→IC composite, not the decoder `D` alone", and b: "`|det J_D|` is the shared
    measure weight … `J_D = ∂physical/∂(u,v)`". `docs/contracts/principia_integrator_contract.md`: `(m, r, p)` "comes from
    `x₀ + J_D·δ`". It is built by REQ-DEC-033 (M6); `crates/kernel/src/stage/decode.rs` says the linearised path lands
    in M6, and nothing in `crates/` computes a Jacobian of the decode today.
  - `docs/design/principia_dd_generation_root.md` gives `√det(JᵀJ)` as "the log of the area element" for the links'
    forward Jacobians only; nothing defines `|det J_D|` for a non-square `J_D`, nor the Gram factor
    `√(|J q₁|²|J q₂|² − (J q₁ · J q₂)²)` that R-452 quotes.
- **Silence:** (1) the views' `J` is `∂(Jacobi or shape coordinates)/∂z`, applied to `q₁`, `q₂` (and, for the steer,
  RQ-344, its pseudo-inverse over all eight directions); the corpus's `J_D` has two columns (∂/∂u, ∂/∂v) and the
  particle frame's 15 rows. On an affine chart `J_D`'s columns are `2·(∂D/∂z)·q₁` and `2·(∂D/∂z)·q₂` through
  `(m, r, p)`, so the views' pushforward could be read off `J_D` up to that factor and the change of coordinates, but
  the steer cannot. (2) Which stage the views differentiate: before or after canonicalisation and the scale gauge
  (whether before or after `η_E` is RQ-345). (3) The derivative's method and step (central differences in f64, as
  deep_zoom's, or analytic). (4) The Gram factor's definition: `√(|J q₁|²|J q₂|² − (J q₁ · J q₂)²)` is the
  pushforward's area factor only for an orthonormal pair, and the decoder's `q₁`, `q₂` carry the zoom σ (`|q| = σ`,
  R-83) and, with both tilts set, the shear `u₁ · u₂ = sin τ₁ sin τ₂` (RQ-340), so on them the "stretch" picks up σ²
  and `√(1 − sin²τ₁ sin²τ₂)` and changes with the zoom; and the factor of 2 from the chart's `(2s−1)`. (5) The metric
  on the physical coordinates, which the grid depends on as the steer does, is RQ-346. (Points 2, 4 and 5 added in
  the fix round of PR #227, physics review P7 to P9.)
- **Options seen:**
  - (a) A definition requirement (R-72): a CPU, f64, central-difference Jacobian `∂x/∂z` (8 columns) of the decoder's
    output in the view's coordinates at a latent point, the views reading `J q₁`, `J q₂` from it, the Gram factor
    read on the orthonormalised pair, `√(|J q̂₁|²|J q̂₂|² − (J q̂₁ · J q̂₂)²)` with `q̂₁`, `q̂₂` orthonormal (the physics
    review's proposed reading of R-452's formula); a new requirement in M6 beside REQ-DEC-033, sharing its method.
  - (b) Read the views off REQ-DEC-033's `J_D` once built (two columns, through `(m, r, p)` to Jacobi or shape
    coordinates by the chain rule), and drop the physical steer or give it another construction.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

## RQ-342: The Jacobi view's eight coordinates are not eight degrees of freedom of the decoder's image: ρ̃ᵧ is identically 0 and the four position components carry two, the masses move none of them, and the momenta are not mass-weighted *(physics review, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for before the set is built. Nothing is
chosen. For the physics reviewer, then the human (the set is the human's proposal).*

- **File, section:**
  - `decisions.md` § R-452 (the human's words, 3.2): "Jacobi view, 8 coordinates: the mass-weighted Jacobi vectors and
    their momenta, ρ̃ₓ, ρ̃ᵧ, λ̃ₓ, λ̃ᵧ, p_ρₓ, p_ρᵧ, p_λₓ, p_λᵧ, as the decoder writes them."
  - `docs/design/principia_dd_decoder.md` §3.2 and `docs/design/principia_chart_reference.md` §0.2: the canonical frame
    `ρ̃ = R̃ cos α · (1, 0)`, `λ̃ = R̃ sin α · (cos β, sin β)`, `R̃ = 1`; §0.3: `p_ρ = (q0, q1)`, `p_λ = (q2, q3)`,
    physical Jacobi momenta (`docs/notes/ic_inspector_scratchpad.md`: "*no* `√μ` unweighting").
  - `docs/contracts/principia_chart_decoder_contract.md` Part 1: the eight latent coordinates are "2 config ⊕ 4
    momentum ⊕ 2 mass", rotation and scale fixed.
- **The points:** (1) `ρ̃ᵧ ≡ 0` on the decoder's image, so every cell in its row and column is always 0; (2) `ρ̃ₓ, λ̃ₓ,
  λ̃ᵧ` carry the two configuration degrees of freedom (α, β) with `R̃ = 1`, so they are redundant; (3) with `η_E` off,
  `z_μ1`, `z_μ2` move none of the eight, so a plane tilted into the mass directions pushes forward degenerately
  (`J q = 0` for `q` in them) and the view cannot draw it; the physics review measured the set's rank at exactly 6 at
  every sample, its kernel the two mass directions; (4) the positions are mass-weighted and the momenta are not, so
  `Σ`-type comparisons mix weightings; (5) whether the decoder's output is read before or after the scale gauge;
  `η_E` is RQ-345: with it on, `η_E` depends on the masses and the configuration, so the masses do move the momenta,
  and point 3 holds only with it off; (6) a non-redundant 8-set exists: the decoder `z ↦ (α, β, q₀…q₃, m₀, m₁)` is a
  diffeomorphism onto its image (each block a monotone link of its own latents, the masses through softmax∘tanh), so
  the decoder's own physical set, `(α, β)` or a chart of the hemisphere, the four Jacobi momenta and two masses, gives
  an 8 × 8 `J` of full rank (the physics review: rank 8 at 200 of 200 samples, `η_E` off). With such a set
  `J⁺ = J⁻¹`, the steer is metric-independent and lights cell (i, j) at exactly ±1 at `z₀` for every cell, and its
  only refusals are chart singularities (RQ-344). (Point 6, and the rank in point 3, added in the fix round of
  PR #227, physics review P6 and P7.)
- **Options seen:**
  - (a) Keep the eight as proposed; the view shows the dead row and the degenerate cases (readout "J q = 0: the plane
    moves only the masses"). This fails the criterion the human set in 3.2 ("no redundant coordinate"): a row and
    column always 0, and a steer refused or lighting below ±1 on most cells.
  - (b) Eight coordinates that chart the image, the full-rank set of point 6: e.g. `α, β` (or `ρ̃ₓ, λ̃` in polar form),
    the four momenta, and the two masses (`m₀/M`, `m₁/M`), each named as the decoder writes it.
  - (c) The momenta mass-weighted as the positions are (`p̃ = p/√μ`), so the coordinates share one weighting. This
    complements (a) or (b) rather than replacing them: mass-weighting the momenta alone leaves the rank at 6.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200: the steer's cells are this set's).

## RQ-343: The Shape view's eight coordinates are redundant and incomplete: n̂ is three numbers for two degrees of freedom, R is 1 on the decoder's image, no doc defines ṅ or Ṙ, and the masses and the overall rotation rate are missing *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for. Nothing is chosen. For the physics
reviewer, then the human (the set is the human's proposal).*

- **File, section:**
  - `decisions.md` § R-452 (the human's words, 3.2): "Shape view, 8 coordinates: the shape-sphere vector n̂ = (n₁, n₂,
    n₃), the hyperradius R, and their rates ṅ₁, ṅ₂, ṅ₃, Ṙ."
  - `docs/design/principia_chart_reference.md` § "3. Shape-sphere chart (θ, φ)", §3.1: `n = ((a−b)/I, 2p/I, 2q/I)`,
    `I = a + b`; the full reduced phase space is `T*S²`. The scale gauge fixes `I = 1` (`principia_dd_decoder.md` §3.5).
- **The points:** (1) `|n̂| = 1`, so `n₁, n₂, n₃` carry two degrees of freedom and their pushforward is constrained
  (`n̂ · ṅ = 0`); (2) `R = √I = 1` on the image, so its row is always 0; (3) no doc defines `ṅ` or `Ṙ` (from the
  momenta through which masses, in which frame); (4) the set has no mass coordinate and no overall rotation rate
  (`L_z`, the fibre of `T*S²`) as coordinates. The masses do move its rates: `ṅ` and `Ṙ` are `d/dt` of the
  mass-weighted `ρ̃`, `λ̃`, with `ρ̃̇ = p_ρ/√μ_ρ` and `λ̃̇ = p_λ/√μ_λ`, e.g.
  `Ṙ = cos α · p_ρx/√μ_ρ + sin α (cos β p_λx + sin β p_λy)/√μ_λ` (the physics review: the `z_μ` columns' norms about
  19 and 17, not 0). The Shape Jacobian has rank 5 at every sample (`n̂` 2, `ṅ` 2, `Ṙ` 1), and its 3-dimensional
  kernel is mixed: the rotation (`L_z`-only) direction in the momentum block, and two mass directions each compensated
  by a momentum change. So a plane in the `L_z` direction pushes forward degenerately; a mass-direction tilt does not
  push forward to 0. The masses enter Shape's rates but not Jacobi's coordinates (RQ-342 point 3); (5) the 2-to-1
  cover of the shape sphere (chart_reference §3.3); (6) a non-redundant 8-set exists (RQ-342 point 6): (b)'s set
  `(θ, φ, θ̇, φ̇, L_z, Ṙ, m₀, m₁)` has full rank (the physics review: rank 8 at 299 of 300 samples, the one other at
  8e-8 relative being a saturated mass logit). (Point 4 corrected, and point 6 added, in the fix round of PR #227,
  physics review P4 and P6.)
- **Options seen:**
  - (a) Keep the eight; define `ṅ`, `Ṙ` from the decoded state (`ṅ = dn/dt` along the flow at `t = 0`), and show the
    dead `R` row. This fails the criterion the human set in 3.2 ("no redundant coordinate"): a row and column always
    0, rank at most 5, and a steer refused or lighting below ±1 on most cells.
  - (b) A chart of the image, the full-rank set of point 6: `θ, φ` (or two of `n̂`, naming which: `(n₁, n₂)` is
    singular on `n₃ = 0`, the collinear configurations, which is the decoder's mirror boundary `β ∈ {0, π}`), their
    rates, `L_z` (or the fibre rate), `Ṙ` and the two masses.
  - (c) Drop the Shape view until the set is settled.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200: the steer's cells are this set's).

## RQ-344: The physical steer's latent preimage q ∝ J_D⁺ e_i needs a Jacobian over all eight latent directions, a metric, a rank test and a rule for a coordinate outside the Jacobian's range; none is written *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by R-452's port (16 Oct 2026), the physics review the human asked for on this construction. Nothing is
chosen. For the physics reviewer, then the human.*

- **File, section:** `decisions.md` § R-452 (the human's words, 3.4): "clicking physical cell (i, j) sets the slice
  plane to the latent preimage of that physical plane at z₀: q₁ ∝ J_D⁺ e_i, q₂ ∝ J_D⁺ e_j, Gram–Schmidt
  orthonormalised, with J_D⁺ the pseudo-inverse. If J_D is rank-deficient there, so the preimage is not a 2-plane,
  refuse the click and log why to the console." No doc has a pseudo-inverse or a latent preimage
  (`principia_inverse_encode_contract.md` inverts by closed-form blocks after canonicalisation and rejects a "smallest
  latent norm" argmin for invariant charts).
- **The points:** (1) it needs RQ-341's eight-column `∂x/∂z`, not the chart's two-column `J_D`; (2) the
  Moore–Penrose inverse assumes the Euclidean metric on `z` (and on the physical coordinates, whose units differ:
  positions and momenta; the physical metric is RQ-346); (3) by RQ-342 and RQ-343 the proposed sets' Jacobians are
  rank-deficient on the whole image (rank at most 6 for Jacobi, at most 5 for Shape), so "rank-deficient there" needs
  reading as "the two preimages do not span a 2-plane", with a threshold (a singular
  value or the Gram factor of `J⁺e_i`, `J⁺e_j` below a tolerance); (4) where `e_i` is not in the Jacobian's range
  (`ρ̃ᵧ`, `R`), `J⁺e_i = 0` and the click is always refused, and where it is partly in range the steered plane's
  pushforward lights cell (i, j) below ±1, so R-452's test "lights that physical cell at ±1 (to tolerance)" holds only
  for `e_i`, `e_j` in the range; (5) the steered basis is a general 8D plane, not a preset with tilts, so how the
  Manifold view's sliders follow needs saying (as the Chart builder's Apply does, §G7, is the nearest); (6) the
  coupling to RQ-342 and RQ-343: with a full-rank 8-set (their option (b), `η_E` off), `J⁺ = J⁻¹`, the steer is
  metric-independent and lights (i, j) at exactly ±1 at `z₀` for every cell, its only refusals chart singularities
  (link saturation, the α and β poles, the chosen sphere chart's poles), and points 2 to 4 largely dissolve; options
  (a) and (b) below matter only if the human keeps a rank-deficient set. With `η_E` on, no 8-set has full rank
  (RQ-345). (Point 3 corrected, and point 6 added, in the fix round of PR #227, physics review P5 and P6.)
- **Options seen:**
  - (a) As written, the test restricted to coordinates in the range, and the sliders following as after Chart
    builder's Apply, with one of two rank tests: (a1) the smaller singular value of `[J⁺e_i, J⁺e_j]` above a
    tolerance, a quantity in `J`'s inverse units that scales with `J`, so a fixed tolerance on it is not dimensionless;
    or (a2) the smaller singular value of `[P e_i, P e_j]`, with `P = J J⁺` the projector onto `J`'s range, which lies
    in [0, 1], is scale-free, and also predicts how close to ±1 cell (i, j) lights (the physics review, P10).
  - (b) A weighted pseudo-inverse with stated metrics on `z` and on the physical coordinates (the latter RQ-346's).
  - (c) Only the latent steer (the Plücker view's); the physical cells are not clickable.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-200).

## RQ-345: With the energy normalisation E* set, every physical Jacobian of the decoder has rank at most 7, whatever the coordinates; the physical views and the steer must read the state before or after η_E, and the steer needs a rule for the direction normal to the energy shell *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by PR #227's physics review (P7, 16 Oct 2026), filed in its fix round as a new entry rather than folded into
RQ-341, because it bounds every coordinate set of RQ-342 and RQ-343 and the steer of RQ-344 alike. Nothing is chosen.
For the human (physics: it changes what the physical views show).*

- **File, section:** `docs/design/principia_chart_reference.md` § "0.6 Energy normalisation — optional, and forbidden
  on some charts": "`η_E = √((E* − U)/K0)          pᵢ ← η_E·pᵢ          feasibility: E* ≥ U`". `decisions.md` § R-452
  (the human's words, 3.2) pushes the plane "through the decoder's Jacobian at z₀" and names no stage.
- **The points:** (1) with `E*` set, `p ← η_E p` puts the decoder's whole image on the level set `E = E*`, so every
  physical Jacobian has rank at most 7, whatever the eight coordinates (the physics review: rank 7 at every sample,
  even for RQ-342's full-rank set); no 8-coordinate set is non-degenerate there. (2) `η_E` depends on the masses and the
  configuration, so with it on the masses move Jacobi's momenta (RQ-342 point 3 holds only with it off). (3) The
  steer on an `E*`-normalised chart has a direction it cannot reach: the one normal to the energy shell.
- **Options seen:**
  - (a) The physical views and the steer read the state before `η_E` (the full-rank sets stay full rank; the views show
    a state the integrator does not start from when `E*` is set).
  - (b) They read it after `η_E`, as the integrator starts from it; on an `E*`-normalised chart the steer refuses the
    one direction normal to the energy shell, and a cell whose plane needs it is refused with that reason logged.
  - (c) After `η_E`, with the Jacobi and Shape views replaced on an `E*`-normalised chart by a 7-coordinate set on the
    level set.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

## RQ-346: The physical views' grid and the physical steer both depend on a metric on the physical coordinates, whose units differ (positions, momenta, angles, masses); no metric is written *(physics review, silence, R-72, R-452; context: TASK-M6-35)*

*Found by PR #227's physics review (P8, 16 Oct 2026), filed in its fix round as one question for the view and the
steer; RQ-344 point 2 keeps the metric on `z`. Nothing is chosen. For the human (physics).*

- **File, section:** `decisions.md` § R-452 (the human's words, 3.2): "u₁ = J_D q₁, u₂ = J_D q₂, orthonormalised";
  3.4: "J_D⁺ the pseudo-inverse". `docs/design/principia_chart_reference.md` §0.2, §0.3: the positions mass-weighted,
  the momenta physical.
- **The points:** (1) R-452's view orthonormalises `u₁`, `u₂` and normalises `Σ p² = 1` in the physical coordinates'
  Euclidean metric; rescaling one coordinate (the units of momenta against positions, angles against masses) rescales
  its row and column of `p_ij` and can change the dominant cell, so the grid and the dominant pair depend on the
  metric. (2) The steer's `J⁺` depends on the same metric (RQ-344 point 2), unless the set has full rank (RQ-342 and
  RQ-343 point 6, where `J⁺ = J⁻¹`). (3) One choice should serve both, so the grid shows the planes the steer reaches.
- **Options seen:**
  - (a) The Euclidean metric in the coordinates as named, stated as a convention, with the help text saying the grid
    depends on the units.
  - (b) The canonical pairing for Jacobi, `(ρ̃, p_ρ/√μ_ρ)` and `(λ̃, p_λ/√μ_λ)`, which gives a natural metric there
    (the physics review's suggestion), and a stated one for Shape and the masses.
  - (c) Each coordinate scaled by a stated reference range (e.g. its spread over the chart's image), so every row is
    dimensionless.
- **Applied meanwhile:** nothing; TASK-M6-35 waits.
- **Waits:** TASK-M6-35 (REQ-GUI-199, REQ-GUI-200).

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

## RQ-406: On a periodic axis, whether the view centre is stored unwrapped (continuous pan, an unbounded number) or canonicalised after each gesture (bounded, with an invisible jump of one period) *(the human's queued question, R-537 Q1, EX9; REQ-GUI-270; context: TASK-M8-55)*

*Queued by the human (`principia_extension_interaction.md` §4, Q1). For the human: yes, it changes what the view state
records.*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q1.** On a periodic axis, is the view
  centre stored unwrapped (continuous pan, unbounded number) or canonicalised after each gesture (bounded, with an
  invisible jump of one period)? *Recommendation:* unwrapped in the view state, canonical in keys and addresses."; EX9:
  "The view state (centre, zoom, and on a periodic axis the unwrapped centre, Q1) is recorded as it is."
- **Options seen:** (a) unwrapped in the view state, canonical in keys and addresses (the file's recommendation);
  (b) canonicalised after each gesture, the view state bounded, the pan jumping by one period invisibly; (c) unwrapped,
  and canonicalised only on export.
- **Applied meanwhile:** (a), which EX9 assumes (R-537 U14). A ruling other than (a) is a follow-up change to
  REQ-GUI-270. Nothing waits.

## RQ-407: Whether repeats of the field are drawn at full strength *(the human's queued question, R-537 Q2, EX2, EX6; REQ-GUI-268; context: TASK-M8-55)*

*Queued by the human (`principia_extension_interaction.md` §4, Q2). For the human: yes, it changes what is drawn.*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q2.** Are repeats of the field drawn at
  full strength? *Recommendation:* yes; they are the same systems. Repeats of a trace are fainter, as in the demo."
- **Options seen:** (a) yes, the field's repeats at full strength, a trace's repeats fainter (the file's
  recommendation); (b) the field's repeats dimmed too, so the primary range reads first; (c) a setting, full strength
  by default.
- **Applied meanwhile:** (a) (R-537 U14). Nothing waits.

## RQ-408: An outline of the chart's primary square when the view goes past it *(the human's queued question, R-537 Q3; context: TASK-M8-55, TASK-M8-08)*

*Queued by the human (`principia_extension_interaction.md` §4, Q3). For the human: yes, it adds an overlay.*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q3.** An outline of the chart's primary
  square when the view goes past it. *Recommendation:* an Overlays › Chart toggle, off by default and on in the debug
  view. The demo has it on."
- **Options seen:** (a) an Overlays › Chart toggle, off by default and on in the debug view (the file's
  recommendation); (b) always drawn while the view shows past the square; (c) none.
- **Applied meanwhile:** nothing is built: no outline is drawn and Overlays › Chart gains no toggle until this is ruled
  (R-537 U14). Nothing else waits.

## RQ-409: A zoom-out limit on periodic charts, how many periods may show *(the human's queued question, R-537 Q4; REQ-GUI-267; context: TASK-M8-55)*

*Queued by the human (`principia_extension_interaction.md` §4, Q4). For the human: yes, it changes navigation.*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q4.** A zoom-out limit on periodic charts
  (how many periods may show). *Recommendation:* none beyond the frame budget and baseline cover. The demo caps at 3×
  for its fixed maps."
- **Options seen:** (a) none beyond the frame budget and the baseline cover (the file's recommendation); (b) a fixed
  number of periods; (c) a limit set by the baseline cover's depth (REQ-SCHED-074).
- **Applied meanwhile:** (a) (R-537 U14). Nothing waits.

## RQ-410: The level-0 pole repeat: drawn from its two level-1 halves, or the bootstrap raised to `L ≥ 1` on every chart with a pole-crossing axis *(the human's queued question, R-537 Q5, EX7(b); REQ-SCHED-101; context: TASK-M8-56)*

*Queued by the human (`principia_extension_interaction.md` §4, Q5). For the human: yes (physics: the quadtree's
addressing).*

- **File, section:** `decisions.md` § "R-537 — …", the applied text, EX7(b): "At `L = 0` the half-period shift splits
  the root, so **a level-0 repeat across a pole is drawn from its two level-1 halves** (Q5)."; `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q5.** The level-0
  pole repeat: draw it from its two level-1 halves, or raise the bootstrap to `L ≥ 1` on every chart with a
  pole-crossing axis? *Recommendation:* draw from the children; the bootstrap is already ≥ 2 in practice."
- **Options seen:** (a) draw it from its two level-1 children (the file's recommendation); (b) raise the bootstrap to
  `L ≥ 1` on every chart with a pole-crossing axis.
- **Applied meanwhile:** (a), as EX7(b) says (R-537 U14). Nothing waits.

## RQ-411: Whether a comment pinned in a repeat attaches to the canonical point, so that it appears in every image *(the human's queued question, R-537 Q6, EX5; context: TASK-M8-55; waits with RQ-420)*

*Queued by the human (`principia_extension_interaction.md` §4, Q6). For the human: yes; it needs RQ-420 first.*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_extension_interaction.md` § "4. Queue, don't rule": "**Q6.** Does a comment pinned in a repeat
  attach to the canonical point, so that it appears in every image? *Recommendation:* yes."
- **Options seen:** (a) yes, to the canonical point, shown in every image (the file's recommendation); (b) to the
  pixel's own place, shown in that image only.
- **Applied meanwhile:** (a) is the reading, but nothing is built: the corpus defines no comment on the figure
  (RQ-420). Nothing else waits.

## RQ-412: Repeats of a trace at half strength or full *(the human's queued question, R-538 Q1, TM7, R-537 EX6; REQ-GUI-275; context: TASK-M8-58)*

*Queued by the human (`principia_trace_markers.md` §12, Q1). For the human: yes, it changes what is drawn.*

- **File, section:** `decisions.md` § "R-538 — …", the applied text, TM7: "the trace is drawn **in every image of the
  chart in view**, with every marker and icon …, at half strength"; `docs/incoming/2026-10-20_extension_trace/principia_trace_markers.md` § "12. Queue, don't rule": "**Q1.** Repeats of a trace at half strength
  or full. *Recommendation:* half, so the primary reads first."
- **Options seen:** (a) half strength (the file's recommendation); (b) full strength; (c) a setting, half by default.
- **Applied meanwhile:** (a) (R-538 I13). Nothing waits.
- **Note (R-538's follow-up, J8):** along an axis that mixes θ with a non-periodic coordinate (a tilted chart) there
  are no images (TF2.2; RQ-430), so this entry applies to the images of untilted periodic, pole-crossing and fold
  charts; the other θ branches of a tilted chart are RQ-421's.

## RQ-413: The start icon: the star in every set, or Pictures' initial-triangle glyph as an option elsewhere *(the human's queued question, R-538 Q2, TM4; REQ-GUI-272; context: TASK-M8-58)*

*Queued by the human (`principia_trace_markers.md` §12, Q2). For the human: yes (look).*

- **File, section:** `decisions.md` § "R-538 — …", the applied text, TM4's table, Start: "star | star | the initial
  triangle in body colours | star"; `docs/incoming/2026-10-20_extension_trace/principia_trace_markers.md` § "12. Queue, don't rule": "**Q2.** Start icon: the star in every set, or Pictures' initial-triangle glyph
  as an option elsewhere. *Recommendation:* star by default, the triangle as Pictures'."
- **Options seen:** (a) the star by default, the triangle as Pictures' (the file's recommendation); (b) the star in
  every set; (c) the triangle offered as a start option in every set.
- **Applied meanwhile:** (a) (R-538 I13). Nothing waits.

## RQ-414: Where a syzygy and an end event coincide in time, which draws on top *(the human's queued question, R-538 Q3, TM5; REQ-GUI-273; context: TASK-M8-58)*

*Queued by the human (`principia_trace_markers.md` §12, Q3). For the human: yes (look).*

- **File, section:** `docs/incoming/2026-10-20_extension_trace/principia_trace_markers.md` § "12. Queue, don't rule": "**Q3.** Where a syzygy and an end event
  coincide in time, which draws on top? *Recommendation:* the end event."
- **Options seen:** (a) the end event on top (the file's recommendation); (b) the syzygy on top; (c) the syzygy
  hidden where they coincide.
- **Applied meanwhile:** (a) (R-538 I13). Nothing waits.

## RQ-415: Whether "Leaves / enters the window" chevrons and edge-pinned icons merge into one edge treatment *(the human's queued question, R-538 Q4, TM6; REQ-GUI-274; context: TASK-M8-58)*

*Queued by the human (`principia_trace_markers.md` §12, Q4). For the human: yes (look).*

- **File, section:** `decisions.md` § "R-538 — …", the applied text, TM6; `docs/incoming/2026-10-20_extension_trace/principia_trace_markers.md` § "12. Queue, don't rule": "**Q4.** Should "Leaves / enters the
  window" chevrons and edge-pinned icons merge into one edge treatment? *Recommendation:* keep them separate; pins are
  events, chevrons are the line."
- **Options seen:** (a) keep them separate (the file's recommendation); (b) one edge treatment, a pinned icon drawn
  with the chevron's direction.
- **Applied meanwhile:** (a) (R-538 I13). Nothing waits.

## RQ-416: F3's Fit scales "the view's own square, wherever the view sits, not the chart's `[0,1]²`" (R-537 EX1), where R-472 X2, the human's words, scales "the chart's `[0,1]²` … with no extension" *(conflict: the human's words against a ruling, R-537 EX1, R-472 X2, R-468 E2; REQ-GUI-230, REQ-GUI-267; context: TASK-M8-46, TASK-M8-55)*

*Found by the port of R-537 (the instruction: "If anything in these files contradicts main or that branch, or my words
contradict a ruling, tell me and queue it rather than resolving it"). Nothing is chosen. For the human: yes, it changes
what Fit shows once the view can sit past the chart's square.*

- **File, section:**
  - `decisions.md` § "R-537 — …", the applied text, EX1: "F3 Fit scales the window's square to the screen (R-468). Its
    "square" is the view's own square, wherever the view sits, not the chart's `[0,1]²`."
  - `decisions.md` § "R-472 — …", the human's words, X2: "**Fit**: the chart's `[0,1]²` is scaled to fill the window,
    with no extension."; § "R-468 — …", the human's words, clause 2: "Fit: the chart is scaled up to fill the window."
  - `decisions.md` § "R-468 — …", E2 (applied per R-369): ""The chart square" is the view's square, `UV ∈ [0,1]²`, the
    depth-0 root the figure shows at the current `(z₀, q₁, q₂)`, the chart's own `[0,1]²` at the preset's window … so
    Fit shows the view's square and nothing beyond it."
  - `docs/gui/principia_render_gui_spec.md` § "G1. …", the F3 bullet: "**Fit**: the view's square, `UV ∈ [0,1]²` at
    the current `(z₀, q₁, q₂)`, is scaled uniformly to the window's shorter side".
- **The conflict:** until R-537 the view's square sat inside the chart's square whenever the controls were shown, so
  "the chart's `[0,1]²`" and "the view's square" did not part. With pan and zoom reaching past the square (EX1), they
  do: EX1 scales the view's square, which may then hold extension, blank or repeats; R-472 X2's words scale the chart's
  own `[0,1]²`, "with no extension".
- **Options seen:**
  - (a) EX1: Fit scales the view's own square wherever it sits, drawing whatever it holds, extension included; "with no
    extension" read as no window area beyond that square (R-468 E2's reading).
  - (b) R-472 X2 literally: Fit scales the chart's own `[0,1]²`, the preset's window at the plane anchor, wherever the
    view sits, so pressing F3 in Fit can change what is shown.
  - (c) Fit scales the view's square, and the extension inside it draws as the canvas background under Fit.
- **Applied meanwhile:** (a), R-468 E2's reading, which EX1 states (R-537 U4). Nothing waits; a ruling for (b) or (c)
  is a follow-up change to REQ-GUI-230 and REQ-GUI-267.

## RQ-417: R-537 EX5 and R-538 TM8 put `decode_failed`, its detail and "forbidden" in the readout of a pixel that is not a system, where R-470 N3, the human's words, shows them only in the debug views *(conflict: the human's words against a ruling, R-537 EX5, R-538 TM8, R-470 N3, R-467; REQ-GUI-269, REQ-GUI-276; context: TASK-M8-55, TASK-M8-58)*

*Found by the port of R-537 and R-538. Nothing is chosen. For the human: yes, it changes what a presentation view
shows.*

- **File, section:**
  - `decisions.md` § "R-537 — …", the applied text, EX5: "The readout says "not a system", names the check that
    failed (for example "a mass ≤ 0") and gives the label (`decode_failed`, its detail)."
  - `decisions.md` § "R-538 — …", the applied text, TM8: "**Readout:** the readout says "forbidden: not integrated",
    why (for example "a mass is ≤ 0, so this is not a three-body system"), the label `decode_failed`, detail 2, and
    the masses."
  - `decisions.md` § "R-470 — …", the human's words, N3: "NaN, Inf, `sim_failed`, `decode_failed`, and the
    classification behind the hatch are shown **only in the debug views** (R-41's baked variants)."; § "R-467 — …",
    its heading: "Forbidden pixels and non-finite values are debug-only".
- **The conflict:** the newer words show the label, its detail and the word "forbidden" in the readout of any view;
  N3 shows `decode_failed` and the classification behind the hatch only in the debug views, and R-467 keeps "forbidden"
  to the debug view.
- **Options seen:**
  - (a) EX5 and TM8 as written: the readout shows "not a system" (or "forbidden: not integrated"), the failed check, the
    label and its detail in every view; N3 read as about drawing, not the readout.
  - (b) N3 holds for the readout: in a presentation view it says "not a system" and names the failed check, and the
    label, its detail and "forbidden" appear only while a debug view is the figure.
  - (c) The readout as (b), the label and detail behind a "details" toggle in the readout.
- **Applied meanwhile:** (b) (R-537 U7), the reading that keeps both texts' common part: "not a system" and the failed
  check, as R-469 A5's readout names a stop. A ruling for (a) or (c) is a follow-up change to REQ-GUI-269 and
  REQ-GUI-276. Nothing waits.
- **Note (R-538's follow-up, J5):** the trace's readout gains a second count on a curve axis, "past the domain's end:
  N samples", which is not a bug and logs no warning (RQ-425). It is a readout line of the trace, as "in forbidden"
  is, and follows whichever option this entry takes. TF1 does not otherwise touch this entry.

## RQ-418: R-538 TM9's guard (b) ends a run whose step size is not finite "as incomplete", where the integrator contract labels NaN/Inf during integration `sim_failed` *(conflict: the human's instruction against a contract, R-538 TM9, R-445; REQ-INT-096, REQ-EVT-002, REQ-SYS-017; context: TASK-M8-58)*

*Found by the port of R-538. Nothing is chosen. For the human: yes (physics: what a run's label means).*

- **File, section:**
  - `decisions.md` § "R-538 — …", the applied text, TM9: "`computeIC` needs both guards: … **(b)** a step size that is
    not finite ends the run as incomplete."; TM3: "**Incomplete** | the hover's step budget ran out".
  - `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer" (the terminal states): "`sim_failed` (NaN/Inf during
    integration) and `decode_failed` …"; REQ-SYS-017: the CPU f64 instance of the kernel is the one `computeIC` runs.
- **The conflict:** "incomplete" is the hover's step budget running out, a run cut short but sound; a non-finite step
  is a run that failed, which the contract labels `sim_failed`. The trace file uses "incomplete" for both.
- **Options seen:**
  - (a) `sim_failed`, as the contract says; the trace still ends with the "?" icon, since it is not whole.
  - (b) incomplete, as TM9 says: a hover-only state, the kernel's label unchanged.
  - (c) `sim_failed` with its own end icon, distinct from incomplete's "?".
- **Applied meanwhile:** (a) (R-538 I8): the run ends at once and never loops, which both texts require. A ruling for
  (b) or (c) is a follow-up change to REQ-INT-096. Nothing waits.

## RQ-419: R-537 EX4's "never any gaps … raw canvas background where a system exists is a bug, never a loading state" leaves open the frames before any ancestor exists: launch, or a cold cache with no backdrop and no snapshot *(silence, R-537 EX4, R-72; REQ-GUI-268, REQ-SCHED-056; context: TASK-M8-55, TASK-M5-15)*

*Found by the port of R-537. Nothing is chosen. For the human: no; the orchestrator may rule it, as a mechanical
consequence, unless the human wants it.*

- **File, section:**
  - `decisions.md` § "R-537 — …", the applied text, EX4: "Every visible point of the window is drawn, at whatever
    resolution is available. Until a quad is computed it shows its nearest computed ancestor … Raw canvas background
    where a system exists is a bug, never a loading state."
  - `docs/contracts/principia_caching_contract.md` § "Part 4 — …": "Something low-resolution appears in one or two
    dispatches; refinement then proceeds beneath it."; § "Part 5 — …", the fallback: "When there are **no payloads
    behind the picture** — the spotlight-reel handoff … or a cold cache — the original screen-texture snapshot applies".
- **The question:** at launch, on a chart switch with no backdrop, or on a cold cache with no snapshot, no ancestor
  exists for the first dispatches; EX4 does not say what those frames draw.
- **Options seen:** (a) those frames are excepted: the background until the first baseline cover lands, which EX4's
  checks follow; (b) the first frame waits for the baseline cover (a blocking first dispatch); (c) a neutral loading
  fill drawn as a figure mark until then.
- **Applied meanwhile:** (a) (R-537 U6). Nothing waits.

## RQ-420: R-537 EX5 names "comments" and "the console's pick" among the interactions that must work at every visible pixel, and the corpus defines neither *(silence, R-537 EX5 and Q6, R-72; REQ-GUI-269; context: TASK-M8-55; RQ-411 waits on it)*

*Found by the port of R-537. Nothing is chosen. For the human: yes; a definition is a design choice.*

- **File, section:**
  - `decisions.md` § "R-537 — …", the applied text, EX5: "That means hover, click and pin (R-453), lock, the IC
    inspector, the trace, keep, readouts, comments and the console's pick."; §4, Q6: "Does a comment pinned in a repeat
    attach to the canonical point".
  - `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)": the console's controls, which include
    no pick; no section of the spec or of the GUI state contract defines a comment on the figure.
- **The question:** what a comment on the figure is (where it is stored, how it is placed, whether it is exported), and
  what the console's pick is (a console command that picks a pixel or an IC), so that EX5 can apply to them.
- **Options seen:** (a) a definition requirement for each (R-72), written by the GUI tasks that need them, EX5 applying
  when they exist; (b) the words read as naming features not yet in the corpus, to be specified by the human; (c) both
  dropped from EX5.
- **Applied meanwhile:** EX5 is built for the interactions the corpus defines (R-537 U9). Nothing waits; RQ-411 is
  answered with it.

## RQ-421: On a tilted chart with a periodic parameter, whether the other θ branches (θ ± 2π) are also drawn, each faded by its own out-of-plane distance *(the human's queued question, R-538 TF7 Q5, TF2.2; REQ-GUI-278; context: TASK-M8-58)*

*Queued by the human (`principia_trace_followup.md` F7, Q5). For the human: yes, it changes what is drawn.*

- **File, section:** `decisions.md` § "R-538 — …", the follow-up's applied text, TF7: "**Q5.** On a tilted chart with a
  periodic parameter, should the other θ branches (θ ± 2π) also be drawn, each faded by its own out-of-plane distance,
  since they are the same system projected at different distances? *Recommendation:* no by default; an option. The
  demo draws the continuous branch only."
- **Options seen:** (a) no by default; an option (the file's recommendation); (b) always drawn, faded; (c) never drawn.
- **Applied meanwhile:** (a) (R-538 J13): the continuous branch only. The option fades each branch by its own
  out-of-plane distance, which exists today only as R-509's proxy on the shape chart; it is built when REQ-GUI-266's
  distance lands (RQ-341). Flagged. Nothing waits.

## RQ-422: The time ramp's default: Sunset, as the mock-ups, or a perceptually uniform one (Viridis) *(the human's queued question, R-538 TF7 Q6, TF4; REQ-GUI-282; context: TASK-M8-58)*

*Queued by the human (`principia_trace_followup.md` F7, Q6). For the human: yes (look).*

- **File, section:** `decisions.md` § "R-538 — …", the follow-up's applied text, TF7: "**Q6.** The ramp's default:
  Sunset, as the mock-ups, or a perceptually uniform one (Viridis)? *Recommendation:* Sunset, which reads on the dark
  field; Viridis's dark end vanishes on it."; TF4: "**Presets:** Sunset (the default, `#FFF27A #FF8A5B #E0457B #7A4CFF
  #3BC7FF`), Viridis, …".
- **Options seen:** (a) Sunset (the file's recommendation); (b) Viridis.
- **Applied meanwhile:** (a) (R-538 J12, J13). Nothing waits.

## RQ-423: The follow-up names text the delivered bundle does not have ("§3a", the superseded message's "T13") and says F2 "adds two bug classes" while listing three *(conflict of wording: the human's words against the delivered bundle and against themselves, R-538 TF1, TF2; REQ-GUI-272, REQ-GUI-277 to REQ-GUI-279; context: TASK-M8-58)*

*Found by the port of the follow-up into R-538. Nothing is chosen. For the human: no; nothing built depends on it, and
the reading applied is the one the human's message gives.*

- **File, section:**
  - `docs/incoming/2026-10-20_trace_followup/principia_trace_followup.md`, the preface: "It amends that bundle's
    `principia_trace_markers.md`: F1 replaces its §3a, F2 adds two bug classes, F3 and F4 change §1 and §2"; the
    heading "## F1. Where a trace can leave a chart (replaces §3a)"; "## F2. Two bug classes this rule exposes (new,
    with tests)", then items 1, 2 and 3.
  - The human's message (R-538, the follow-up): "F1 replaces §3a." and "F2 adds three bug classes, each with a test".
  - The superseded message (R-538, history): "One correction, in principia_trace_markers.md §3a and T13".
  - `docs/incoming/2026-10-20_extension_trace/principia_trace_markers.md`, as delivered: its sections are "## 3. The
    markers" (no §3a) and its tests "T1" to "T12" (no T13); the leaving text is §3's row "**Leaves the chart's reach** |
    the state leaves what the chart can show (on the latent chart: a momentum past `q_max`, or α past its limits)".
- **The conflict:** "§3a" and "T13" name sections of an updated bundle that was never sent; the file's "two" and its
  three items disagree.
- **Options seen:** (a) F1 replaces what the delivered bundle says on leaving (§3's row, I5 of R-538), and three bug
  classes, as the human's message says; (b) an updated bundle with a §3a and a T13 is still to come and is ported when
  it arrives.
- **Applied meanwhile:** (a) (R-538 J2). Nothing waits.

## RQ-424: T15 leaves "while the target's `|q| ≥ q_max`", where the demo's tab E leaves "while `|p_λ,y| > q_max`": the boundary and the quantity differ *(conflict: the human's test against the demo the human sent, R-538 TF6 T15, TF1; REQ-GUI-277; context: TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: no for the quantity (one component, read as the
same); yes for the boundary only if it matters which way a sample exactly at `q_max` falls.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF6, T15: "A latent-momentum tilt leaves exactly while the target's `|q| ≥ q_max`;
    the physical-momentum tilt never does, over 100 runs".
  - `docs/incoming/2026-10-20_trace_followup/principia_trace_followup.md` § "F5", tab E: "A trace leaves (dotted
    ring, dashed line) while `|p_λ,y| > q_max` and comes back."; the demo, `docs/gui/reference/trace/trace_markers.html`,
    E's note: "a trace whose p_λ,y passes q_max = 2 has no point on this chart".
  - TF1: "**A trace leaves only where a coordinate entering a shown axis with non-zero weight has no value.** That
    means: ±∞ from a saturated link; NaN; a bounded axis's inverse outside its domain."
  - The superseded message (R-538, history): "The fade uses R-509's proxy |q|."; R-509 Y3: the proxy is "the
    scale-normalised Jacobi momentum magnitude".
- **The conflict:** `≥` against `>` at the boundary; "the target's `|q|`" against `|p_λ,y|`, where `|q|` is also the
  superseded message's name for R-509's proxy, a magnitude over all four momenta.
- **Options seen:** (a) the target component's own `|q₃|` (`= |p_λ,y|`, REQ-DEC-019), leaving at `|q₃| ≥ q_max`, where
  the link's inverse is ±∞ (TF1); (b) the same quantity, leaving at `|q₃| > q_max`, the boundary sample kept; (c) the
  proxy's magnitude.
- **Applied meanwhile:** (a) (R-538 J4). Nothing waits.

## RQ-425: A trace sample whose projection falls past a bounded edge: TF1's rule 2 says it leaves, its curve-axis row says it is not drawn and "counted apart from the not-a-system count", R-538 TM8 counts not-a-system samples in one count that must read 0, and R-470 N1 calls a bounded axis past its edge "not a system" *(conflict: the human's words against themselves and against rulings, R-538 TF1, TM8, R-470 N1, R-512 F3; REQ-GUI-276, REQ-GUI-280; context: TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: yes, it changes what is drawn and counted.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF1, rule 2: "A trace leaves only where … That means: … a bounded axis's inverse
    outside its domain."; its table, the curve-axis row: "**No.** The encode projects onto the nearest point of the
    curve …, and the distance to the curve is fade. If the nearest point lies past a bounded edge, the point is not
    drawn there: those are real systems the bounded axis does not show (§8, the domain's end). They are counted apart
    from the not-a-system count, whose non-zero value is a bug; this one is not."
  - The same ruling, TM8: "A sample whose chart point is not a system (fails `validate` layers 2 and 3) is not drawn …
    The readout counts such samples ("in forbidden: N samples"). It reads 0 on every run tried. A non-zero count is a
    bug in the encode or the chart map".
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — …", "Chart-aware validation", layer 2: "Burrau
    charts: $\nu \in (0,1)$."
  - `decisions.md` § "R-470 — …", N1: "A pixel that is not a system draws **blank** … This covers: a bounded axis past
    its edge; …"; § "R-512 — …", F3: "A blank pixel past a bounded edge inside the window is the domain's end".
  - The demo (version 27), tab H, readout: "in forbidden 4718 samples" (`docs/incoming/2026-10-20_trace_followup/images/ramp.png`).
- **The conflict:** for one sample past a curve axis's bounded edge, rule 2 says the trace leaves (rings, dashed);
  the row says it is only not drawn and counted apart; TM8 and layer 2 put it in the forbidden count that must read 0;
  R-470 N1 names the place "not a system", R-512 F3 "the domain's end"; the demo counts it as forbidden.
- **Options seen:**
  - (a) The row on a curve axis: not drawn, the line broken, no ring, counted in its own readout line ("past the
    domain's end"), never a bug; rule 2 on every other bounded axis (leaves, with rings); TM8's count keeps only samples
    whose chart point is not a system.
  - (b) Rule 2 everywhere: past any bounded edge the trace leaves, with rings, counted apart from TM8's.
  - (c) TM8 as written: past a bounded edge is not a system, in the forbidden count, which then is not always 0.
- **Applied meanwhile:** (a) (R-538 J5). Nothing waits; a ruling for (b) or (c) is a follow-up change to REQ-GUI-276
  and REQ-GUI-280.

## RQ-426: TF1's curve row and T18 make the distance to the curve the trace's fade, where R-509 fades only by the proxy where it is defined, the shape chart, and draws no fade elsewhere *(conflict: the human's words against a ruling, R-538 TF1, TF6 T18, R-509 Y3; REQ-GUI-280, REQ-GUI-249, REQ-GUI-266; context: TASK-M8-58, TASK-M8-54)*

*Found by the port of the follow-up. Nothing is chosen. For the human: yes, it changes what a test asserts.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF1's table, the curve-axis row: "the distance to the curve is fade"; rule 1:
    "Anything orthogonal to the chart's plane feeds only the off-slice distance, so it shows as the fade. Where that
    distance has no value, R-509 applies: no fade, and the GUI says so."; TF6, T18: "The curve chart's trace is the
    nearest ν: perturb a run's shape off the curve, the point stays and the fade grows".
  - `decisions.md` § "R-509 — …", (3): "The trace fade uses the proxy where it is defined; elsewhere there is no fade,
    and the GUI says so. The real off-plane distance waits on RQ-341."; Y3: "It is "defined" on a chart whose plane
    holds it at zero, the shape chart of the mock-ups, …; on every other chart the trace draws at full opacity".
  - The demo (version 27), the Trace panel: "Fade off the slice (momentum proxy |q|, R-509; floor 0.18)".
- **The conflict:** on the ν × dilation chart, whose shown axis carries a momentum, R-509's proxy is not defined, so
  there is no fade and T18's "the fade grows" cannot hold; TF1 rule 1 defers to R-509, its row and T18 do not.
- **Options seen:** (a) T18's fade arm runs only where an off-plane distance is defined (R-509's proxy now,
  REQ-GUI-266's distance when RQ-341 is ruled); elsewhere its point-stays arm runs and the settings line says there is
  no fade; (b) the distance to the curve is a defined off-plane distance on a curve chart now, a second proxy beside
  R-509's; (c) T18 waits on RQ-341 whole.
- **Applied meanwhile:** (a) (R-538 J6). Nothing waits.

## RQ-427: TF3 draws a trace that is not being played or scrubbed "from `t = 0` to its end", and does not say what becomes of a chosen `Window` or `LastDt` length, or of the own clock's "Once" *(silence, R-538 TF3, R-476 §8, R-72; REQ-GUI-281, REQ-GUI-271; context: TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: no; the orchestrator may rule it, as a
mechanical consequence, unless the human wants it.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF3: "**Not playing or scrubbed:** when the playhead is not playing and is not being
    scrubbed, every trace is drawn from `t = 0` to its end (its event, or the horizon). … **The own-clock mode** is
    always animated."
  - `docs/design/principia_lines.md` § "8. Traces", Length: "To the playhead (the default), a fixed window, or the last
    `Δt`."; Animation: "Own clock: the trace plays over its own window at its own speed …: once, looped, or back and
    forth."
- **The question:** whether "every trace … to its end" overrides a fixed window or the last `Δt` while paused, and what
  the own clock draws once a "Once" play has finished.
- **Options seen:** (a) TF3 is the default length's (`ToPlayhead`): a fixed window or the last `Δt` is kept as chosen,
  and "Once" holds its last frame, the whole trace; (b) TF3 for every length: paused, every trace is drawn whole;
  (c) the last `Δt` is measured back from the trace's end while paused.
- **Applied meanwhile:** (a) (R-538 J11). Nothing waits.
- **See also:** RQ-465, R-541 G5's footer and R-540 S4, which restate TF3 for every trace (found merging main into
  rulings/R-540).

## RQ-428: TF4 and T20 keep a trace's ramp "with kept orbits" through save and load, where the GUI state contract puts the kept orbits in `ViewUI`, which is never serialised with `SimConfig` and `RenderState` *(conflict: the human's words against a contract, R-538 TF4, TF6 T20, R-453 D5; REQ-GUI-282; context: TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: yes, it decides whether a saved view holds its
kept orbits.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF4: "It is part of the trace style, so it is kept with kept orbits, saved with the
    view, and travels in exports (the round-trip rule's tier 2)."; T20: "The ramp round-trips: a kept orbit's ramp
    survives save and load, and Reverse reverses it | a ramp held only in GUI state".
  - `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface":
    "ViewUI (pure UI) … kept orbits … — never read by the engine"; "Provenance already serialises `SimConfig +
    RenderState`".
  - `docs/gui/principia_render_gui_spec.md` § "G2. …", the Trajectory panel: "The pin, like the kept orbits, is
    `ViewUI` state, never undoable"; `decisions.md` § "R-453 — …", D5.
  - `docs/design/principia_lines.md` § "3. The data model": "`Trace { kind: Hover | Pinned | Kept(id), ic: …, … }`" and
    "`Lines { …, traces: [Trace], … } // part of the exported view state`".
- **The conflict:** a kept orbit's `Trace`, its IC and its ramp, sits in `Lines` (exported); the kept orbit itself is
  `ViewUI` (not exported). T20's "survives save and load" needs the first; the contract says the second.
- **Options seen:** (a) a kept orbit's `Trace` (its IC and style, the ramp included) is `Lines`' state and round-trips
  with `RenderState`; reopened, the view redraws it; the Trajectory panel's kept-orbit tabs stay `ViewUI`; (b) the kept
  orbits move into `RenderState` whole; (c) only the ramp round-trips, as a style in the library, and kept orbits are not
  saved.
- **Applied meanwhile:** (a) (R-538 J12). Nothing waits; a ruling for (b) or (c) is a follow-up change to REQ-GUI-282.

## RQ-429: TF2.1 forbids the trace to reuse the lookup and lock validation (inverse encode Part 6), where R-538 TM8 decides a sample is not a system by `validate`'s layers 2 and 3, one of whose checks is "θ ∈ [0, 2π)" *(conflict: the human's words against themselves and a contract, R-538 TF2.1, TM8, inverse encode Part 6; REQ-GUI-276, REQ-GUI-277; context: TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: no if (a), which keeps both texts; yes otherwise.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF2.1: "The inverse encode contract's validation (Part 6, the three layers) is
    written for lookup and lock: it clamps, projects or rejects the whole state if any component fails. **The trace must
    not reuse it.** … The trace checks each shown axis on its own (F1)."
  - The same ruling, TM8: "A sample whose chart point is not a system (fails `validate` layers 2 and 3) is not drawn".
  - `docs/contracts/principia_inverse_encode_contract.md` § "Part 6 — …", "Chart-aware validation": "Lookup and lock
    validate against the **active chart's** constraints before committing."; layer 2: "Shape sphere: $\theta \in [0,
    2\pi)$ (azimuth) and $\varphi \in [0, \pi]$"; "Mixed-axis charts inherit both axes' constraints."
- **The conflict:** TM8 runs two of Part 6's layers on each sample; TF2.1 says the trace must not reuse Part 6 and must
  check per shown axis; layer 2's θ range would fail a branch-continuous θ (TF2.2).
- **Options seen:** (a) TM8's test is the not-a-system test of the sample's chart point on the shown axes, after the
  types fold it, never Part 6's ladder on the sample's whole state (TF2.1); (b) TM8 as written, Part 6's layers 2 and 3
  on the whole state, with θ folded first; (c) TM8 dropped, TF1's per-axis check alone.
- **Applied meanwhile:** (a) (R-538 J10). Nothing waits.

## RQ-430: TF2.2 says a tilted chart with a periodic parameter "is affine along that axis and does not repeat", where R-537 EX2 says "Periodic (θ): the chart repeats" and EX7(b)'s canonical addresses fold θ with no word on tilt *(silence: R-537 EX2 and EX7(b) on a tilted chart, R-538 TF2.2, TF6 T17, R-472 X3a; REQ-CHART-080, REQ-SCHED-101, REQ-GUI-278; context: TASK-M8-56, TASK-M8-58)*

*Found by the port of the follow-up. Nothing is chosen. For the human: no; it follows from R-472 X3a (the types act on
the chart's own parameters `(a, b)`, the tilt target's `d` not folded), and the orchestrator may rule it as a
mechanical consequence, unless the human wants it.*

- **File, section:**
  - `decisions.md` § "R-538 — …", TF2.2: "On a tilted chart those branches are **not** repeats of one image. θ wraps
    but the target does not, so a shift of `2π / cos τ` along the tilted axis is a different system. Such a chart is
    affine along that axis and does not repeat."; T17: "The tilted chart does not repeat: the systems at `s` and
    `s + 2π/cos τ` differ | drawing it with periodic repeats".
  - `decisions.md` § "R-537 — …", EX2: "**Periodic** (θ): the chart repeats."; EX7(b): "On the shape sphere at level
    `L` … **θ period:** `ix' = ix mod n`."
  - `decisions.md` § "R-472 — …", X3a, and R-473 V5: on a tilted chart the types act on the chart's own parameters
    `(a, b)`.
- **The question:** whether EX2's and EX7(b)'s folding applies along an axis that mixes θ with a non-periodic
  coordinate.
- **Options seen:** (a) it does not: canonical folding applies along an axis only where every term with non-zero weight
  has that period, so a tilted θ axis is affine (TF2.2); the untilted sphere's maps stand; (b) EX7(b) folds the
  window's quad addresses on every chart whose parameter is periodic.
- **Applied meanwhile:** (a) (R-538 J8). Nothing waits.
- **See also:** RQ-472, EX7(b)'s pole and equator maps on a tilted view against chart_reference §3.3's tilted fold
  (found merging main into this branch).

## RQ-440: Listen's audio backend in prin-rs: a Rust synthesis graph on `cpal`, transcribed node for node from the demo's Web Audio, or sound kept web-only *(the human's queued question, R-540 Q1, S8, S9; REQ-GUI-315, REQ-GUI-317 to REQ-GUI-320; context: TASK-M8-12, TASK-M8-59)*

*Queued by the human (`principia_listen.md` S14, Q1; this bundle's Q-numbers, distinct from the trace follow-up's F7
Q5 and Q6, which are RQ-421 and RQ-422). For the human: yes; the human's message says it needs an answer before Listen
is built.*

- **File, section:** `decisions.md` § "R-540 — …", the applied text, S14: "**Q1. Audio backend for prin-rs.** The demo
  is Web Audio. Recommendation (a): a Rust synthesis graph on `cpal` for output, with the voice and effects chain
  transcribed node for node. (b) Keep sound web-only."; the human's message: "Q1 (the audio backend) and Q3
  (`RMS_FLOOR` against our integrator) need answers before Listen is built."; TASK-M8-12's Deliverables: "audio out
  with cross-fade (native: an audio-output backend; browser: WebAudio in TASK-M8-38's shell)".
- **Options seen:** (a) a Rust synthesis graph on `cpal` for output, the voice and effects chain transcribed node for
  node (the file's recommendation); (b) sound web-only.
- **Applied meanwhile:** nothing is built. TASK-M8-12, TASK-M8-59 and TASK-M8-60 wait on this entry and RQ-442
  (R-540 H14).

## RQ-441: Where Listen's analysis runs: on the CPU from the trajectory viewer's own samples, or on the GPU with the integrator *(the human's queued question, R-540 Q2, S2; REQ-GUI-311, REQ-SYS-060; context: TASK-M8-12)*

*Queued by the human (`principia_listen.md` S14, Q2). For the human: yes.*

- **File, section:** `decisions.md` § "R-540 — …", S14: "**Q2. Where the analysis runs.** Recommendation (a): on the
  CPU, from the trajectory viewer's own samples, at the sizes in S2; it is two FFTs per frame. (b) On the GPU with the
  integrator."
- **Context:** REQ-SYS-060, "Trajectory integration and sonification must run in the dedicated inspector worker";
  pointer_channels § "6. What this reuses rather than adds" (working notes, R-109) suggests reusing frequency
  diffusion's FFT, a requirement retired (REQ-TOOL-093, "R-109: only pointer_channels §4 is normative; §6's FFT reuse has
  no normative source").
- **Options seen:** (a) the CPU, from the viewer's `computeIC` samples, at S2's sizes, in the inspector worker
  (REQ-SYS-060) (the file's recommendation); (b) the GPU with the integrator.
- **Applied meanwhile:** nothing is built (TASK-M8-12 waits on RQ-440 and RQ-442); when it starts, (a) unless this is
  ruled otherwise first.

## RQ-442: `RMS_FLOOR` against prin-rs's integrator: measure Lagrange's shape rms under *Angles only* over `t ≤ 400` and set the floor at 10× its maximum *(the human's queued question, R-540 Q3, S7, S13; REQ-GUI-316; context: TASK-M8-12)*

*Queued by the human (`principia_listen.md` S14, Q3). For the human: yes; the human's message says it needs an answer
before Listen is built.*

- **File, section:** `decisions.md` § "R-540 — …", S14: "**Q3. `RMS_FLOOR` against prin-rs's integrator.**
  Recommendation (a): measure Lagrange's shape rms under *Angles only* with the shipped integrator over `t ≤ 400`, and
  set the floor at 10× its maximum. The demo's 1e-7 is its own RK4's drift."; S7: "Measured in the demo under *Angles
  only*: Lagrange's shape rms reads **2.65e-7**, just above the floor, so it plays at the min level with lines at
  181.5 Hz and above — that is the demo integrator's error, not motion."; S13: "| Lagrange (angles only) | level 0,
  because the shape rms is below the floor | the floor removed, or set above prin-rs's drift (Q3) |".
- **Found by the port (the code is the transcription source):** the demo's code says, on the floor's line, "below this
  the shape is not moving (the rotating Lagrange triangle reads ~1e-11)", where S7 measures 2.65e-7; and its integrator
  is Dormand–Prince 5(4) (`// integrator: Dormand–Prince 5(4), adaptive`), where Q3 says "its own RK4's drift". With the
  demo's floor S13's Lagrange (angles only) test fails by S7's own measurement; it passes only once the floor is set
  against prin-rs (this entry).
- **Found by physics review 5595386853 of PR #267, P1 (facts beside both sides; nothing chosen):**
  - Equal-mass Lagrange is linearly unstable: Routh's 27(m₁m₂ + m₂m₃ + m₃m₁)/M² = 9 > 1, so any seed, integration error
    or f64 rounding, grows exponentially, as e^{0.31 t}, ω/√2 (an e-fold about every 3.2 time units; *was:* "an
    e-fold about every 0.31 time units", physics review 5595462873 of PR #267, P5, applied per R-369). The reviewer's port (DOP853, rtol
    10⁻¹²) measured max |n⊥| at 4.0e-13 for t ≤ 20, 9.8e-8 for t ≤ 60, 2.4e-2 for t ≤ 100, and broken up by t ≈ 150;
    the Angles-only rms 1.7e-7 over [0, 60], 3.0e-2 over [0, 100] and 0.87 over [0, 400].
  - A floor no integrator lowers: an exactly rotating triangle in f64 reads an Angles-only rms of 4.9e-8, φ = acos(n3)
    being quantised at √(2·2⁻⁵³) = 1.49e-8 at the pole and the pole weight multiplying it by 10. Under Shape vector it
    reads 4.9e-9 (the code forms sin φ from acos(n3)).
  - So Q3's (a) as worded, "over `t ≤ 400` … 10× its maximum", gives a floor of about 9, which silences every run; the
    demo's 2.65e-7 is the instability's growth by t = 60, neither "its own RK4's drift" nor a steady integrator floor;
    and the code comment's "~1e-11" matches neither signal.
  - Meanwhile REQ-GUI-316's arm has no fixed horizon and waits here (`rq: RQ-442`), REQ-GUI-309's Lagrange arm states
    t ≤ 60, and `principia_listen.md` §7 no longer asserts that prin-rs's integrator is "far quieter".
- **Options seen:** (a) measure as the file says and set the floor at 10× the maximum (the file's recommendation);
  (b) keep 10⁻⁷.
- **Applied meanwhile:** nothing is built; `principia_listen.md` §7 carries 10⁻⁷ as the demo's value and points here.

## RQ-443: The superseded sketch row, "listen · separations → pitch": marked stale, replaced by R-540's signal, or kept as a fourth `Signal` *(the human's queued question, R-540 Q4, S1; REQ-GUI-090; context: TASK-M8-12)*

*Queued by the human (`principia_listen.md` S14, Q4). For the human: no for (a), which the human's message applies.*

- **File, section:** `decisions.md` § "R-540 — …", S14: "**Q4. The superseded sketch row.** The dev-GUI sketch's
  "listen · separations → pitch" is replaced by this file's signal. Recommendation (a): mark it stale, replaced by this
  ruling. (b) Keep separations as a fourth `Signal`."; the human's message: "The GUI file supersedes … the sketch's
  "listen · separations → pitch" row (queued as Q4). Record each replaced line as "stale value, replaced by R-xxx"."
- **Where the row is:** in the sketch's `01_main.png` (`docs/gui/design/`) and, in words, `principia_render_gui_spec.md`
  § "G2. …", the Trajectory panel's listen bullet ("the artboard's selector ("separations → pitch") is illustrative
  (R-68)"), § "G13. …" ("the sound mapping (`θ(t), φ(t)` → spectrum)"), REQ-GUI-090 ("the artboard's 'separations →
  pitch' selector must not be implemented") and TASK-M8-12's Goal.
- **Options seen:** (a) stale, replaced by R-540 (the file's recommendation); (b) separations kept as a fourth `Signal`.
- **Applied meanwhile:** (a) (R-540 H13): the listen bullet's words are recorded stale, replaced by R-540; REQ-GUI-090's
  "must not be implemented" stands. Nothing waits.

## RQ-444: The close-approach threshold `0.2 R` and the event depth scale: kept, stated as parameters *(the human's queued question, R-540 Q5, S9; REQ-GUI-319; context: TASK-M8-59)*

*Queued by the human (`principia_listen.md` S14, Q5). For the human: no for (a).*

- **File, section:** `decisions.md` § "R-540 — …", S14: "**Q5. Close-approach threshold** `0.2 R` and the event depth
  scale are demo choices. Recommendation (a): keep them, stated as parameters."; S9: "a pair's separation at a local
  minimum below `0.2 R` (`R = √I`) … (`depth = min(1, log₁₀(0.2R/d)/2)`)".
- **Options seen:** (a) kept, stated as parameters (the file's recommendation); (b) calibrated (R-71).
- **Applied meanwhile:** (a) (R-540 H13): `principia_listen.md` §9 states both. Nothing waits.

## RQ-445: Listen's section in the Trajectory panel: collapsed until first used, then remembered per user, or always collapsed *(the human's queued question, R-541 Q6, G4; REQ-GUI-326; context: TASK-M8-61)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q6; this bundle's Q-numbers, distinct from the trace
follow-up's F7 Q5 and Q6, RQ-421 and RQ-422). For the human: no for (a).*

- **File, section:** `decisions.md` § "R-541 — …", G11: "**Q6. Listen's default collapsed state.** Recommendation (a):
  collapsed until first used, then remembered per user. (b) Always collapsed."; G4's table: "**Listen** (collapsed until
  first used, then remembered)".
- **Options seen:** (a) collapsed until first used, then remembered per user (the file's recommendation); (b) always
  collapsed.
- **Applied meanwhile:** (a) (R-541 P11): remembered in the GUI's own settings, as a window's place is (R-461), never the
  contract. Nothing waits.
- **See also:** since PR #267's split (applied per R-369) the Listen section is REQ-GUI-334's, TASK-M8-64's.

## RQ-446: The expanded strip's spectrum takes the Legend's column, or the Legend stays and the spectrum stacks under the spectrogram (the band ≈ 520 px) *(the human's queued question, R-541 Q7, G7; REQ-GUI-330; context: TASK-M8-63)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q7). For the human: no for (a).*

- **File, section:** `decisions.md` § "R-541 — …", G11: "**Q7. The expanded strip's spectrum in the Legend's column.**
  Recommendation (a): borrow the column, as G7. (b) Keep the Legend and stack the spectrum under the spectrogram, making
  the band ≈520 px."; G7: "| **Strip expanded (⏶)** | … | **the spectrum at the head**, until `⏷` |".
- **Options seen:** (a) borrow the Legend's column (the file's recommendation); (b) keep the Legend, the band ≈ 520 px.
- **Applied meanwhile:** (a) (R-541 P11). Nothing waits.

## RQ-447: Kept-trace colours: a fixed cycle of six, distinct from the body colours and the stain classes, to be chosen against the stain palettes, where R-538 TM2 has already fixed a six-colour trace palette *(the human's queued question, R-541 Q8, G9; conflict with R-538 TM2; REQ-GUI-271, REQ-GUI-333; context: TASK-M8-58, TASK-M8-61)*

*Queued by the human (`principia_trace_listen_gui.md` G11, Q8). Found by the port: the recommendation reads as if no
cycle were chosen. For the human: yes, if the cycle is to change.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G11: "**Q8. Kept-trace colours.** Recommendation (a): a fixed cycle of six, distinct
    from the body colours and the stain classes; to be chosen against the stain palettes."; G9: "Kept traces take
    distinct colours: kept #2 is `#5ab4ff` in the boards."
  - `decisions.md` § "R-538 — …", TM2's Colour row: "fixed `#FFF27A` for the hover or pinned trace. Kept traces take
    the trace palette in turn (`#7FE3FF #FF9EC4 #B7F28A #FFB86B #C9A7FF #9EF0D2`)"; `principia_lines.md` § "8. Traces",
    Colour, and REQ-GUI-271 carry it.
  - `#5ab4ff` is also Listen's left channel and the shape sphere's window highlight (R-540 S11, R-541 G4).
- **The conflict:** R-538's six are a fixed cycle already, not checked against the stain palettes; the boards' kept #2
  is not one of them and is a Listen colour.
- **Options seen:** (a) a fixed cycle of six, distinct from the body colours and the stain classes, chosen against the
  stain palettes (the file's recommendation), replacing R-538's six if they fail that check; (b) R-538's six stand; the
  boards' `#5ab4ff` illustrative.
- **Applied meanwhile:** R-538's six (the ruling), the boards' `#5ab4ff` illustrative (R-541 P10). Nothing waits.

## RQ-448: R-541 G2 withdraws `Lines ▾` ("An earlier art-board pass had both; they are withdrawn"), where `Lines ▾` is R-476's menu for the whole line system: contours, axes, grids, presets and class edges as well as the traces *(conflict: the human's words against a ruling, R-541 G2, R-476 L2, R-509 Y5, R-538 I2; REQ-GUI-247, REQ-GUI-271, REQ-GUI-324; context: TASK-M8-50, TASK-M8-58, TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, it decides whether the line system keeps its menu.*

- **File, section:**
  - The human's message (R-540): "The GUI file supersedes the earlier art-board pass's `Lines ▾` and `Listen ▾`
    menus".
  - `decisions.md` § "R-541 — …", G2: "**No `Lines ▾` and no `Listen ▾`.** An earlier art-board pass had both; they are
    withdrawn." and "A trace is an overlay, so it lives with the overlays."; G10's One place test: "a stray control
    (e.g. a surviving `Lines ▾`)".
  - `decisions.md` § "R-476 — …", L2, and `principia_lines.md` § "2. Where it sits": "One menu: **Lines ▾** beside
    Overlays ▾, with a count, `Alt+L` to show or hide all."; `principia_render_gui_spec.md` § "G2. …": "**Lines ▾
    (R-476).** … The menu lists the contour sets, traces and axes with their visibility, opens the contour-set editor
    and the style library, and holds the presets, among them **Grid** …, **t_end** … and **Class edges** (… R-509),
    which were Overlays toggles."; REQ-GUI-247 (TASK-M8-50).
  - `decisions.md` § "R-538 — …", I2: "They are the trace's entry in Lines ▾ (render_gui_spec §G2, R-476), in TM2's
    order".
  - `Listen ▾` is in no corpus file: the port looked in `docs/` (less `docs/archive/` and `docs/reference/`),
    `decisions.md`, `REVIEW_QUEUE.md` and `plan/`; it is only in the demos' mock menu bars
    (`docs/gui/reference/trace/trace_markers.html` version 28, `trace_sound.html` version 9) and the earlier art-board
    canvases under `workbench/`, which are not normative.
- **The conflict:** the human's words and G2 withdraw a menu "an earlier art-board pass had"; that pass's `Lines ▾` was
  adopted by R-476 as the line system's one menu, and G2 rehomes only the trace. Withdrawing it whole leaves contours,
  axes, grids, the presets and class edges (R-509) without a menu.
- **Options seen:**
  - (a) The trace leaves `Lines ▾` for `Overlays ▾ › Trace`, the Trajectory panel and `Trace…` (G2, G4, G5); `Lines ▾`
    stays for the line system's other lines; G10's One place test reads "a surviving `Lines ▾`" as a `Lines ▾` that holds
    a trace control.
  - (b) `Lines ▾` withdrawn whole: contours, axes, grids, presets and class edges move to `Overlays ▾` (a Lines group),
    their editors staying windows, `Alt+L` kept.
  - (c) `Lines ▾` withdrawn and the line system's controls placed by a later design pass.
- **Applied meanwhile:** (a) (R-541 P2): the trace's place in `Lines ▾` is stale, replaced by R-541; the rest of
  R-476's menu stands. Nothing waits; a ruling for (b) or (c) is a follow-up change to REQ-GUI-247 and TASK-M8-50.
- **See also:** PR #263's gui review G3 gave REQ-GUI-271 a keyboard reach and order arm on the trace's `Lines ▾` entry;
  merging main into rulings/R-540 moved it, with its presence screenshot, to REQ-GUI-328 (`Trace…`, TASK-M8-62), read
  on R-541 G5's table order (applied per R-369).

## RQ-449: R-541 G2's `K` "keep the current trajectory" is the Figure's lock key (R-409, §G4), where Space keeps the orbit; and G2 does not say in which scope `T`, `⇧T`, `Y`, `K` and `L` act *(conflict and silence, R-541 G2, R-409, R-460; REQ-GUI-097, REQ-GUI-325; context: TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, for `K`.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G2: "**Keys**: `T` trace, `⇧T` kept traces, `Y` syzygies, `K` keep the current
    trajectory, `L` sound on/off. Add them to the keyboard sheet."; the art board `Main.dc.html`'s top bar: "♪ L listen ·
    T trace · K keep".
  - `principia_render_gui_spec.md` § "G3. …": "Figure — arrows pan; + / − zoom, Space keeps the orbit, L listens, K locks
    (§G4), P pins the sample under the pointer in the Trajectory panel (R-453)."; § "G4. …": "Locking (K, a shortcut with
    the focus in the Figure, in either keyboard mode (§G3, R-409); …"; REQ-GUI-097.
- **The conflict:** `K` cannot both lock and keep. `L` agrees (the Figure's L listens). `T`, `⇧T` and `Y` are bound
  nowhere today; §G3 has no global letter keys but `Alt+L` and `?`.
- **Options seen:** (a) `K` stays lock and Space stays keep, G2's "K keep" read as Space; `T`, `⇧T`, `Y` and `L` are the
  Figure's scope shortcuts, as L is (R-409); (b) `K` becomes keep, lock moving to another key; (c) the five global, in
  either mode wherever the focus is, as `Alt+L`.
- **Applied meanwhile:** (a) (R-541 P3). Nothing waits.

## RQ-450: pointer_channels §4 (normative, R-109) sets a floor on trajectory length, "roughly 20+" periods, below which "the sound is meaningless", and REQ-VAL-103 measures it; R-540 S2 plays every run that moves, the human's words "that shouldn't be a thing" *(conflict: the human's words against a ruling, R-540 S2, R-109, R-144; REQ-VAL-103, REQ-GUI-312; context: TASK-M8-12)*

*Found by the port. Nothing is chosen. For the human: yes, it decides whether a short run is heard.*

- **File, section:**
  - `decisions.md` § "R-540 — …", §0: "ah i don't like how some are too short to hear, that shouldn't be a thing."; S2:
    "**No run is too short to hear.** Where no line clears the 1.5-period cut, the **mean rates** are played instead …
    A run that moves at all makes a sound."
  - `docs/design/principia_scratchpad_pointer_channels.md` § "4. Audio-specific constraints, which are NOT the frame
    budget" (normative, R-109, R-144): "**The floor on trajectory length is set by frequency resolution, not by
    compute.** A spectrum needs enough oscillation periods to resolve a line — roughly 20+. So there is a minimum `t`
    below which the sound is meaningless *even though the integration was cheap*. **Measure it; do not guess it.**"
  - REQ-VAL-103: "The minimum trajectory length t for a usable spectrum (about 20+ oscillation periods) must be set by a
    recorded measurement, not guessed." (verify: "gives the minimum t used by listen").
  - The demo's readout (`trace_sound.html`): "— under 20: too short to resolve a note".
- **Options seen:** (a) S2 holds: every moving run sounds, the mean rates below the 1.5-period cut; REQ-VAL-103's
  measured minimum sets the readout's "too short to resolve a note" warning, not a silence; (b) §4 holds: below the
  measured minimum Listen is silent and says why; (c) both: below it the mean rates play, marked as rates, not a note.
- **Applied meanwhile:** (a) (R-540 H6), the human's later words. Nothing waits beyond RQ-440 and RQ-442.

## RQ-451: R-540's default signal, "Angles + rotation", adds the triangle's orientation in space `ψ` to the sound, where R-109's mapping (pointer_channels §1 and §2, REQ-GUI-090) is `θ(t), φ(t)` → spectrum, "the same measurement" as the shape-sphere trace *(conflict: the applied text against a ruling, R-540 S1, R-109; REQ-GUI-090, REQ-GUI-309; context: TASK-M8-12)*

*Found by the port. Nothing is chosen. For the human: yes (physics: what is heard).*

- **File, section:**
  - `decisions.md` § "R-540 — …", S1: "**The sound is the spectrum of the motion of the triangle's shape, plus its
    rotation in space.**"; its table: "| **Angles + rotation (default)** | `w · e^{iθ}` and, summed in power, `e^{iψ}` |
    `φ` | Lagrange's shape is fixed and it still sounds, through `ψ` |"; §0: "shouldn't you use the angles?" and "still
    can't here Lagrange btw".
  - `principia_scratchpad_pointer_channels.md` § "1. There is ONE mechanism, and it now has THREE consumers": "|
    **sonification** | `θ(t), φ(t)` → spectrum → audible | audio out |"; § "2. Why the trace and the sound are the SAME
    measurement": "The loop drawn on the shape sphere *is* `θ(t), φ(t)` traced out. The sound is the spectrum of that
    same path."
  - REQ-GUI-090: "listen … must sonify the corpus's mapping, θ(t), φ(t) → spectrum → audible range" (verify: "the audio
    path's input is the θ(t), φ(t) series of the hovered IC's computeIC trajectory"); TASK-M8-12's test
    `sonify_input_is_theta_phi`.
- **The conflict:** `ψ` is not on the shape sphere, so under the default the sound is no longer the spectrum of the path
  the trace draws there; *Angles only* is §4's mapping.
- **Options seen:** (a) R-540 extends R-109: the default adds `ψ`, *Angles only* is §4's `θ(t), φ(t)`; REQ-GUI-090's
  verify reads the selected signal, the input still only the `computeIC` trajectory; (b) the default is *Angles only*,
  the rotation an option; (c) `ψ` dropped.
- **Applied meanwhile:** (a) (R-540 H5). Nothing waits beyond RQ-440 and RQ-442; the physics reviewer checks the signal
  either way (REQ-GUI-309).

## RQ-452: A struck instrument strikes every voice of each new hovered IC's chord, where pointer_channels §4 and REQ-GUI-050 say "cross-fade, never retrigger" per pixel *(conflict: the transcribed code against a ruling, R-540 S5, S8, R-109; REQ-GUI-050, REQ-GUI-315, REQ-GUI-317; context: TASK-M8-12, TASK-M8-59)*

*Found by the port. Nothing is chosen. For the human: yes, it is how a hover sweep sounds with a struck instrument.*

- **File, section:**
  - `principia_scratchpad_pointer_channels.md` § "4. …": "**Cross-fade, never retrigger.** Hovering across a basin
    boundary is the most interesting moment available — you would *hear* one basin become another — but only if the
    sound is continuous. Retriggering per pixel turns the best part into clicking."; REQ-GUI-050: "Sonification must
    cross-fade between hovered ICs and never retrigger per pixel."
  - `decisions.md` § "R-540 — …", S5: "**Chord** (whole run): one voice per peak, cross-faded in as a bank over ~140 ms,
    and cross-faded out when the run changes. … **Never retriggered.**"; S8: "They are **struck once and ring at their
    floor**."
  - The demo's code, the chord for each new run (`makeBank`): "`if (v.struck) v.strike(lv, now); else v.setG(lv, now,
    0.01);`", the bank's gain then cross-faded in.
- **The conflict:** a hover moving across pixels makes a new chord per run; with a struck instrument each is struck
  (its attack under the bank's cross-fade), which reads as a retrigger per pixel; with the default instrument nothing
  is struck.
- **Options seen:** (a) as the code: the bank cross-fades, and a struck instrument's strike on a new chord is its timbre,
  not a retrigger; (b) a new hovered IC's chord enters a struck instrument at its held floor, unstruck, struck only on
  the trajectory's beats; (c) struck instruments silent while the hover moves, struck on dwell.
- **Applied meanwhile:** (a), the code (R-540 H2). Nothing waits beyond RQ-440 and RQ-442.

## RQ-453: REQ-VAL-102's anchors, spectral entropy 0.076 for the figure-eight against 0.177 for a chaotic IC (the spectrum of `θ(t)`), and R-540's entropy, over the mono spectrum of the selected signal, measured 0.309 against Burrau's 0.352 *(conflict: the applied text against a requirement, R-540 S2, S3; REQ-VAL-102, REQ-VAL-114; context: TASK-M8-12)*

*Found by the port. Nothing is chosen. For the human: yes, it is an M8 gate test.*

- **File, section:**
  - REQ-VAL-102: "The sonification spectrum of θ(t) must separate periodic from chaotic orbits: spectral entropy 0.076
    for the figure-eight against 0.177 for the scratchpad's chaotic IC."; `principia_trajectory_viewing.md` § "1. The
    single mechanism": "Measured, figure-eight vs chaotic: **spectral entropy 0.076 vs 0.177**".
  - `decisions.md` § "R-540 — …", S2: "**Entropy**: Shannon entropy of the normalised power spectrum over `log(bins)`";
    S3: "Burrau 3:4:5 … entropy 0.352 against the figure-eight's 0.309".
- **The conflict:** the anchors are for a different signal and spectrum; under R-540's the two orbits are far closer.
- **Options seen:** (a) REQ-VAL-102 stays a check of the `θ(t)` spectrum as it says, separate from Listen's readout; (b)
  re-anchored to R-540's entropy, the values measured with prin-rs's integrator and confirmed at the M8 gate
  (REQ-VAL-114); (c) both.
- **Applied meanwhile:** (a) (R-540 H6). Nothing waits beyond RQ-440 and RQ-442.

## RQ-454: Listen's defaults in the files against the demo's code: the instrument (Warm sine against Sine (pure)), reverb and stereo width (on against off) and the sound itself (on against off) *(conflict: the human's files against the transcription source, R-540 S8, S9, R-541 G6; REQ-GUI-317, REQ-GUI-320, REQ-GUI-329; context: TASK-M8-59, TASK-M8-62)*

*Found by the port. Nothing is chosen. For the human: yes (one line: which defaults).*

- **File, section:**
  - `decisions.md` § "R-540 — …", S8: "39 instruments, default **Warm sine**."; S9: "| Reverb (on, ×1.40) |" and "|
    Stereo width (on, ×1.50) |".
  - `decisions.md` § "R-541 — …", G6: "| Sound | **♪ sound on**, … instrument (**Warm sine**) …" and "| Effects | … **reverb
    ← spread (×1.40)**, … **width ← angular momentum (×1.50)** |" (bold the default).
  - The demo's code (`docs/gui/reference/trace/trace_sound.html`, the transcription source): `<option value="sine"
    selected>Sine (pure)</option>` with `inst()` falling back to `INSTRUMENTS.sine`; `<input type="checkbox" id="fx_rev">`
    and `<input type="checkbox" id="fx_width">`, unchecked, no script checking them; `SND = { … on: false, … }`, the
    button reading "Sound on" until pressed. The art board `TrajectoryPanel.dc.html`'s at-rest state shows "♪ off".
- **Options seen:** (a) the code's: Sine (pure), reverb off, width off, sound off at launch; (b) the files': Warm sine,
  reverb on, width on, sound on.
- **Applied meanwhile:** (a), the human's message making the code the transcription source (R-540 H2). Nothing waits
  beyond RQ-440 and RQ-442.

## RQ-455: "Grows, last half": the file's window `[t₁/2, t₁]`, the code's `[t₁ − W, t₁]` with `W = max(min(t₁, ½t_c), t₁/2)` *(conflict: the human's file against the transcription source, R-540 S4; REQ-GUI-314; context: TASK-M8-12)*

*Found by the port. Nothing is chosen. For the human: no; the code is applied, as the human's message says.*

- **File, section:** `decisions.md` § "R-540 — …", S4: "| Grows, last half | `[t₁/2, t₁]` |"; the demo's code
  (`windowAt`): "`W = m === 'grow' ? t1 : Math.max(Math.min(t1, 0.5 * tcr), t1 / 2); return [t1 - W, t1];`".
- **The conflict:** below `t₁ = t_c` the code's window is half a crossing time long (or the whole drawn run), not half
  the drawn run.
- **Options seen:** (a) the code's; (b) the file's.
- **Applied meanwhile:** (a) (R-540 H2). Nothing waits beyond RQ-440 and RQ-442.

## RQ-456: A struck note's decay: the file's "time constant `strike`", the code's `strike/3` (and the attack's `att/3`) *(conflict: the human's file against the transcription source, R-540 S8; REQ-GUI-317; context: TASK-M8-59)*

*Found by the port. Nothing is chosen. For the human: no; the code is applied.*

- **File, section:** `decisions.md` § "R-540 — …", S8's table: "| `strike`, `sus` | **struck**: decays with time constant
  `strike` **to a held floor `sus`**, never to zero |"; the demo's code (`makeVoice`'s `strike`):
  "`out.gain.setTargetAtTime(ng2, now, 0.004); out.gain.setTargetAtTime(ng2 * (I.sus ?? 0.25), now + 0.02, I.strike /
  3);`"; `setG`: "`Math.max(tc, (I.att || 0) / 3)`".
- **Options seen:** (a) the code's: rise at 4 ms, from 20 ms a decay at time constant `strike/3` to `sus × level`
  (`sus` default 0.25); an attack at `att/3`; (b) the file's: the time constant `strike`.
- **Applied meanwhile:** (a) (R-540 H2). Nothing waits beyond RQ-440 and RQ-442.

## RQ-457: "Strikes on": the file makes "only when a line changes" one of three options; the code re-strikes a voice whose line has moved more than a quarter-tone in every mode *(conflict: the human's file against the transcription source, R-540 S8; REQ-GUI-318; context: TASK-M8-59)*

*Found by the port. Nothing is chosen. For the human: no; the code is applied.*

- **File, section:** `decisions.md` § "R-540 — …", S8: "**They are struck again on the trajectory's own beats**
  (`Strikes on`): each syzygy the head passes (**default**); every crossing time ÷ 1, 2 or 4; only when a line changes,
  i.e. a voice's partial has moved more than a quarter-tone since it last sounded."; the demo's code (`trackTo`, every
  frame, whatever the mode): "`const moved = !x.lastNote || Math.abs(Math.log2(p.f / x.lastNote)) > 1 / 48; if
  (x.struck) { if (moved) { x.strike(lv, now); x.lastNote = p.f; } } else { … if (x.retrig && moved) { x.note(now); …`";
  the beats (`soundTick`): "`if (mode === 'syz') … else if (mode === 'tc') …`", *only when a line changes* adding none.
- **Options seen:** (a) the code's: the line-change strike in every mode, the mode adding beats; (b) the file's: the line
  change only under its own option.
- **Applied meanwhile:** (a) (R-540 H2). S13's Struck floor test ("struck exactly once without a beat") holds for a line
  that does not move. Nothing waits beyond RQ-440 and RQ-442.

## RQ-458: Under *Stereo by winding* the chord voices only the winding spectra, so channel B, `φ`, is not heard in it, while the follow track finds its lines in the mono sum, `φ` included *(silence, R-540 S1, S6; REQ-GUI-310; context: TASK-M8-12)*

*Found by the port. Nothing is chosen. For the human: yes, it is what is heard in the default.*

- **File, section:** `decisions.md` § "R-540 — …", S1's table: "| **Angles + rotation (default)** | … | `φ` |" (Channel
  B); S6: "| **Stereo by winding (default)** | anticlockwise winding (positive frequencies) | clockwise winding
  (negative frequencies) |", with no word on B; the demo's code: "`L = ch === 'wind' ? { mag: pos, … } : fold, R = ch ===
  'wind' ? { mag: neg, … } : { mag: sp.mag, … }`" (the chord's sides), and the frames' "`peaks(mono, K)`" panned by
  "`(R - L) / (L + R || 1)`".
- **The question:** whether `φ`'s lines are heard in the default chord, and how a `φ`-only line is panned.
- **Options seen:** (a) as the code: the chord drops `φ`, the follow track hears it, a `φ`-only line panned by the
  winding spectra at its bin; (b) `φ` added to both sides of the chord, centred; (c) `φ` only under *Stereo*.
- **Applied meanwhile:** (a) (R-540 H2). Nothing waits beyond RQ-440 and RQ-442.

## RQ-459: The instrument menu's families and their counts (pure 5, keys 3, struck 10, bowed and blown 7, voices 4, synths 8, textures 2) name no instrument, and the code has none *(silence: a definition, R-540 S8, R-541 G6, R-72; REQ-GUI-329; context: TASK-M8-62)*

*Found by the port. Nothing is chosen. For the human: no; the task that builds the menu can write it (R-72), unless the
human wants to.*

- **File, section:** `decisions.md` § "R-540 — …", S8: "Families, for the menu (GUI file G6): pure (5), keys (3), struck
  (10), bowed and blown (7), voices (4), synths (8), textures (2)."; § "R-541 — …", G6: "**The instrument menu is
  grouped**: pure (5), keys (3), struck (10), …, one instrument per row in the build."; the demo's menu groups 19
  instruments ungrouped, 15 under "More" and 5 under "Synths"; S8 lists twelve `strike` instruments, against "struck
  (10)", and five named synths, against "synths (8)".
- **Options seen:** (a) TASK-M8-62 writes the assignment (R-72), meeting the counts (the electric piano among keys, the
  chiptune, raw saw and sub drone among synths, say), reviewed by gui; (b) the human gives it.
- **Applied meanwhile:** (a) (R-541 P8). Nothing waits.
- **See also:** since PR #267's split (applied per R-369) the assignment is TASK-M8-65's (`Listen…`).

## RQ-460: R-541 G1's "Every setting appears in at most two places", where G2, G4 and G5 put the syzygies, the end event icon, the off-screen pins and the wrapped copies in three: `Overlays ▾ › Trace`, the Trajectory panel's marker chips and `Trace…` *(conflict: the applied text against itself, R-541 G1, G2, G4, G5; REQ-GUI-324; context: TASK-M8-61, TASK-M8-62)*

*Found by the port. Nothing is chosen. For the human: no for (a), which keeps every control the file names.*

- **File, section:** `decisions.md` § "R-541 — …", G1: "**Every setting appears in at most two places**: an everyday
  control in the panel, and the full set in a window (`Trace…`, `Listen…`)"; G2: "`Overlays ▾` gains a Trace group … :
  trace under cursor (`T`); kept traces (`⇧T`); syzygies (`Y`); end event icon; off-screen icons, pinned to the edge;
  repeat in wrapped copies; outline the chart's square; a `Trace…` button."; G4: "The marker chips are toggle buttons
  that show the icon they draw: end event, syzygies, leaving the chart, off-screen pins, wrapped copies, head marker.";
  G5's Markers and At the edges rows.
- **Options seen:** (a) all three stay, one state behind them: G1 is read as settings, `Overlays ▾ › Trace` as the view's
  toggles of the same fields, a third route to the same `SetField`; (b) the panel's chips lose the items `Overlays ▾`
  has; (c) `Overlays ▾ › Trace` keeps only the trace, kept traces and `Trace…`.
- **Applied meanwhile:** (a) (R-541 P4). Nothing waits.
- **See also:** RQ-466, the Inspector's listen, a third place for sound (gui review 5595359543 of PR #267, G1).

## RQ-461: R-541 G2 and G5 turn "outline the chart's square" on by default in `Overlays ▾ › Trace` and `Trace…`, where RQ-408 (R-537 Q3) is open, nothing built meanwhile, its recommendation an Overlays › Chart toggle, off by default *(conflict: the applied text against an open entry, R-541 G2, G5, R-537 Q3; REQ-GUI-324, REQ-GUI-328; context: TASK-M8-61, TASK-M8-62, TASK-M8-55)*

*Found by the port. Nothing is chosen. For the human: yes, with RQ-408.*

- **File, section:** `decisions.md` § "R-541 — …", G2: "outline the chart's square"; G5's At the edges row:
  "**outline the chart's square**" (bold the default); the art board `Main.dc.html`'s Trace group shows it ticked;
  RQ-408: "(a) an Overlays › Chart toggle, off by default and on in the debug view (the file's recommendation); (b)
  always drawn while the view shows past the square; (c) none." and "**Applied meanwhile:** nothing is built: no outline
  is drawn and Overlays › Chart gains no toggle until this is ruled".
- **Options seen:** (a) RQ-408 governs: the outline's entries in the Trace group and `Trace…` are built only once RQ-408
  is ruled, as it rules; (b) R-541 answers RQ-408: an outline toggle, on by default, in the Trace group and `Trace…`.
- **Applied meanwhile:** (a) (R-541 P5). Only the outline's two entries wait.

## RQ-462: R-541 G4 lists the Trajectory panel from the top and leaves out the summary's FTLE and `E₀`, the panel's own playhead and the Open full viewer… and IC Inspector… buttons, which §G2 and REQ-GUI-088 have *(silence, R-541 G4, R-65, R-453; REQ-GUI-088, REQ-GUI-326; context: TASK-M8-11, TASK-M8-61)*

*Found by the port. Nothing is chosen. For the human: yes, it decides what the panel shows.*

- **File, section:** `decisions.md` § "R-541 — …", G4: "1. **Header** … 2. **Status line**: running / pinned, the end
  event with its time, `|ΔE/E|`. 3. **Previews** … 4. **Three collapsible sections**"; "The kept list moves here and into
  `Trace…`, and out of the left panel."; G1: "the bottom band's Time column shows the trajectory over time"; the art
  board `TrajectoryPanel.dc.html`'s state A ends with "Open full viewer…" and "IC Inspector…".
  `principia_render_gui_spec.md` § "G2. …", the Trajectory panel: "a one-line summary (fate, `t_end`, FTLE, `E₀`)", "a
  playhead for this orbit", "**Open full viewer…** and **IC Inspector…** (both open §G8)", "**Kept orbits** below, each
  with its fate and time, removable"; REQ-GUI-088.
- **Options seen:** (a) a port adds: the status line carries the summary (fate and `t_end`, FTLE, `E₀`) with G4's items;
  the panel's playhead and the two buttons stay, after Numbers, as art board A has them; the kept list moves as G4 says;
  (b) G4 is the whole panel: FTLE, `E₀` and the panel's playhead go, the buttons stay; (c) G4 is the whole panel, the
  buttons going too.
- **Applied meanwhile:** (a) (R-541 P6). Nothing waits.

## RQ-463: R-541 G7 fixes the bottom band's sizes (196 px, 420 px expanded, the Compass at the left panel's width, the Legend 360 px), where every panel boundary is a remembered splitter (R-462); and "Compass: cube only" does not say what becomes of the compass's buttons, which are keyboard siblings *(conflict and silence, R-541 G7, R-462, R-409, R-452, R-459; REQ-GUI-213, REQ-GUI-330; context: TASK-M8-63)*

*Found by the port. Nothing is chosen. For the human: no for (a), which reads the sizes as R-462 reads the artboards'.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G7: "**Compass · Time · Legend**, 196 px, with the Compass at the Explore left panel's
    width and the Legend at 360 px"; "| **Strip expanded (⏶)** | **grows upward to 420 px**; the view gives up the
    height | … | **cube only** | …"; G10's Band height test: "expanding the strip changes it by exactly the band's
    growth".
  - `principia_render_gui_spec.md` § "G2. …", "Layout: every panel boundary is a splitter (R-462)": "Each panel has a
    minimum size; a double-click on a splitter resets that splitter to its default; the layout is remembered across
    sessions"; § "G13. …": "The artboards' panel sizes and window positions are the defaults".
  - § "G3. …": "The Compass scope holds the compass, then its Tilt and Slice buttons, then its view buttons Cube,
    Plücker, Jacobi and Shape, then the cube's axis buttons H, V and D and, while it is shown, the "4D: see Plücker"
    badge".
- **Options seen:** (a) the sizes are defaults (R-462): the band's splitter keeps its height, 196 px by default; the
  expanded strip is the band's second height, 420 px by default, its own splitter position, remembered; while "cube
  only" the compass draws its cube in its current view and mode, its buttons hidden and out of the keyboard tree until
  `⏷`, a drag on it working as before; (b) fixed sizes, the band's splitter disabled while the strip shows; (c) the
  compass keeps its buttons, the cube shrunk.
- **Applied meanwhile:** (a) (R-541 P9). Nothing waits.

## RQ-464: Listen's horizon and dwell: R-540 S10 integrates a moving hover to `t ≤ 20` ("the maps' horizon") and, after a 280 ms dwell or a pin, on to Listen's own `Horizon` (20, 60, 150, 400), the trace switching with it, where pointer_channels §3 bounds the moving hover by a step budget, the dwell lifting it, and the hover runs to the sim key's horizon, set in the Run window *(conflict: the applied text against a ruling and the GUI spec, R-540 S10, R-541 G6, R-144; REQ-GUI-321, REQ-GUI-329; context: TASK-M8-60)*

*Found by the port. Nothing is chosen. For the human: yes, it decides how long a trace runs.*

- **File, section:**
  - `decisions.md` § "R-540 — …", S10: "While the hover is moving, the run is integrated to `t ≤ 20` (the maps'
    horizon), so the sound is coarse." and "On dwell (280 ms) or pin, the same run is integrated on to the **horizon**
    (`Horizon`: 20, **60**, 150, 400), and the trace and the sound both switch to it and sharpen."; § "R-541 — …", G6's
    Listen row: "horizon t ≤ 20 · **60** · 150 · 400".
  - `principia_scratchpad_pointer_channels.md` § "3. UPDATE TO THE RESPONSIVENESS MODEL — hover is affordable, and the
    earlier caution was wrong" (normative, R-144): "**Give the hover trajectory a STEP BUDGET. If it exhausts it, report
    what it got and mark it incomplete.**" and "**Dwell releases the bound.**"
  - `principia_render_gui_spec.md` § "G2. …": "**Run settings are NOT on the page** (they're rarely changed). Horizon,
    integrator, escape settings, quality, budget, and recompute / cancel live in the Run window (§G5)."; the hover
    integrates with the sim key (`principia_trajectory_viewing.md` § "1. The single mechanism": "`traj =
    computeIC(chart, uv, simKey)`").
- **The conflict:** 20 is the demo's precomputed maps' horizon, not prin-rs's; S10's moving-hover bound is a time, §3's a
  step budget; Listen's `Horizon` extends the trace past the run's horizon, from a control outside the Run window.
- **Options seen:** (a) moving: the map's own horizon (the sim key's `t_max`) under §3's step budget, "20" read as the
  demo's; on dwell (280 ms) or a pin, on to Listen's `Horizon` where it is longer, the trace and the sound both, the map
  unchanged; (b) Listen uses the run's horizon only, its `Horizon` control dropped; (c) as the file: 20 while moving.
- **Applied meanwhile:** (a) (R-540 H8). Nothing waits beyond RQ-440 and RQ-442.

## RQ-465: R-541 G5's footer and R-540 S4 restate R-538 TF3 for every trace, "the whole trace to the horizon is drawn" when nothing plays, where PR #263 (merged to main after this port began) applies RQ-427 (a): only a trace of the default length is drawn whole while paused, a `Window` or `LastDt` length kept as chosen *(conflict: the human's words against an applied option, R-541 G5, R-540 S4, R-538 TF3, RQ-427; REQ-GUI-281, REQ-GUI-314, REQ-GUI-328; context: TASK-M8-58, TASK-M8-12, TASK-M8-62)*

*Found merging main (7daf689b) into rulings/R-540. Nothing is chosen. For the human: yes if G5 was meant to answer
RQ-427; RQ-427 itself said the orchestrator may rule it.*

- **File, section:**
  - `decisions.md` § "R-541 — …", G5 (`principia_trace_listen_gui.md` §G5): "Footer: `reset`, `save as default`. When
    nothing is playing or scrubbing, the whole trace to the horizon is drawn (follow-up F3)."
  - `decisions.md` § "R-540 — …", S4 (`principia_listen.md` S4): "| Follow the head | when nothing is playing or
    scrubbing | **the whole run as one chord**, matching the trace being drawn whole (follow-up F3) |".
  - `docs/design/principia_lines.md` § "8. Traces", Animation, as PR #263 left it: "every trace of the default length
    (to the head) is drawn from `t = 0` to its end (its event, or the horizon), with no head marker, and a `Window` or
    `LastDt` length is kept as chosen (RQ-427 (a), applied meanwhile)"; `principia_render_gui_spec.md` § "G2. …", the
    same; REQ-GUI-281's arm "paused, a Window(t0, t1) trace still draws exactly [t0, t1] and a LastDt trace its last Δt
    (RQ-427 (a)); control: either drawn whole while paused fails" (gui review 5594407176 of PR #263, G2).
  - RQ-427: "(a) TF3 is the default length's (`ToPlayhead`): a fixed window or the last `Δt` is kept as chosen, … (b)
    TF3 for every length: paused, every trace is drawn whole".
- **The conflict:** the bundle's two files, written after TF3, say the whole trace is drawn whenever nothing plays, with
  no word on a chosen length; main now draws a `Window` or `LastDt` trace as chosen while paused, and tests that it
  does. If the files mean every trace, they are RQ-427's option (b) and REQ-GUI-281's new arm fails correct code; if
  they mean the default trace, they agree with (a). S4's chord is the whole run either way, so under (a) a paused
  `Window` trace and Listen's chord no longer "match".
- **Options seen:** (a) RQ-427 (a) stands: G5's footer and S4's "matching" read as the default length's; a paused
  `Window` or `LastDt` trace is drawn as chosen while Listen plays the whole run; (b) the files answer RQ-427 as (b):
  paused, every trace is drawn whole, REQ-GUI-281's `Window` / `LastDt` arm reversed; (c) as (a) for the trace, and
  Listen's paused chord follows the trace's drawn span.
- **Applied meanwhile:** (a): main's text and tests stand; `principia_render_gui_spec.md`'s `Trace…` footer and
  `principia_listen.md` S4 carry "of the default length (RQ-427 (a), RQ-465)". Nothing waits.

## RQ-466: render_gui_spec §G8 puts listen in the Inspector ("**listen** is here too") and REQ-GUI-090 says "listen (in the Trajectory panel and the Inspector)", where R-541 G1 puts everything about one trajectory, its sound included, in the Trajectory panel, and REQ-GUI-324's One place test fails a sound control anywhere else *(conflict: the applied text against the GUI spec and a requirement, R-541 G1, R-65, R-109; REQ-GUI-090, REQ-GUI-324; context: TASK-M8-12, TASK-M8-61)*

*Found by gui review 5595359543 of PR #267, G1. Nothing is chosen. For the human: no; the orchestrator may decide it
as a routine design choice, unless the human wants it.*

- **File, section:**
  - `principia_render_gui_spec.md` § "G8. Inspector — one IC, its trajectory, one timeline (`05_inspectors.png`)":
    "Integration is `computeIC` on the CPU in f64 (trajectory_viewing §1); the engine is reached only through it (§6).
    **listen** is here too."
  - REQ-GUI-090's statement: "listen (in the Trajectory panel and the Inspector) must sonify the corpus's mapping".
  - `decisions.md` § "R-541 — …", G1 (`principia_trace_listen_gui.md` §G1): "**Everything about one trajectory lives in
    one place: the Trajectory panel.** That covers its previews, its trace and its sound." and "**Every setting appears
    in at most two places**: an everyday control in the panel, and the full set in a window (`Trace…`, `Listen…`)".
  - REQ-GUI-324's verify (R-541 G10 One place): "walking the widget tree finds no trace or sound control outside the
    Trajectory panel, Overlays ▾ › Trace, Trace…, Listen… and the Time column".
- **The conflict:** the Inspector's listen either fails the One place test or is dropped against REQ-GUI-090.
- **Options seen:** (a) R-541 G1 holds: listen in the Trajectory panel only; the Inspector holds no sound control, and
  REQ-GUI-090's "and the Inspector" and §G8's sentence are stale; (b) the Inspector keeps listen as a view of the same
  Listen state, and the One place test's list adds the Inspector window (a third place for sound, see RQ-460).
- **Applied meanwhile:** (a) (applied per R-369, PR #267's fix round): §G8 and REQ-GUI-090's note carry it, and
  REQ-GUI-324's control fails a listen control in the Inspector. A ruling for (b) is a follow-up change to REQ-GUI-324,
  TASK-M8-61 and §G8.

## RQ-467: Listen's pitch silences moving runs, against R-540 S2's "A run that moves at all makes a sound" and S13's "No silent run": a bound run with small |E| (t_c = (2|E|)^{−3/2} unbounded as E → 0⁻, Fit to hearing capped at ±8 octaves), and the code's runs with `t_end ≤ 10⁻⁶` or fewer than three samples *(conflict: the transcription source against the human's file, physics, R-540 S2, S3, S13, R-109; REQ-GUI-312, REQ-GUI-313; context: TASK-M8-12)*

*Found by physics review 5595386853 of PR #267, P2. Nothing is chosen. For the human: yes; it changes what is heard
along the bound / escape boundary (physics, CLAUDE.md "When to ask the human").*

- **File, section:**
  - `decisions.md` § "R-540 — …", S2 (`principia_listen.md` S2): "**No run is too short to hear.** … A run that moves at
    all makes a sound."; S13: "| No silent run | every run with `t_end > 0` that moves has at least one voiced line
    (fallback included) | the fallback missing |".
  - The transcription source, `docs/gui/reference/trace/trace_sound.html` v9:
    `return E < 0 ? Math.pow(2 * -E, -1.5) : Math.pow(I, 0.75);` (`crossingTime`);
    `while (f * 2 ** k > 880 && k > -8) k--; while (f * 2 ** k < 110 && k < 8) k++;` (`fitShift`);
    `if (!(tEnd > 1e-6) || sim.samples.length < 3) return null;` (`analyseSound`); and only lines strictly between
    30 Hz and 6 kHz voiced (`principia_listen.md` §3).
- **The physics review's measurements** (figure-eight ICs with the velocities scaled to a target E, t ≤ 20, the
  defaults): E = −1e-3, t_c 1.1e4, shift −8, 8 lines voiced; E = −3e-4, t_c 6.8e4, shift −8, 0 lines (the strongest at
  11.4 kHz); E = −1e-4, t_c 3.5e5, shift −8, 0 lines; E = +1e-4, t_c 0.74, shift +2, 16 lines. The fallback never fires,
  because mono peaks exist, only out of band. Across E = 0 the pitch mapping jumps by about 19 octaves. With Fit off,
  lines outside 30 Hz to 6 kHz are silent too.
- **The conflict:** a moving bound run near E = 0, which a hover crosses at the bound / escape boundary, makes no sound,
  as does a moving run too short to analyse.
- **Options seen:** the file recommends none. (a) the code as transcribed: the silences stand, REQ-GUI-312's test
  excepts them, and the readout names the reason; (b) Fit to hearing uncapped, any whole number of octaves, so every
  moving run with a peak is brought into 110–880 Hz (the jump across E = 0 stays); (c) a crossing time continuous across
  E = 0, a scale-free definition to be given (R-72), removing the silence and the jump; (d) as (a), (b) or (c), and a
  run with `t_end ≤ 10⁻⁶` or fewer than three samples that moves plays its mean rates from the samples it has.
- **Applied meanwhile:** (a) (R-540 H2, the code wins): REQ-GUI-312's verify names the gap and does not assert it;
  `principia_listen.md` §3 points here. TASK-M8-12 waits on RQ-440 and RQ-442 already; a ruling other than (a) is a
  follow-up change to §3 and REQ-GUI-312 / -313.

## RQ-470: R-503 makes the shape sphere's exponential-map pixels past `r = π` inside `[0,1]²` the domain's end by design, and R-467 clause 4, as R-506 W1 now confirms for every gallery chart, makes such a pixel a decoder bug with a console warning *(conflict, R-503, R-467 clause 4, R-506 W1, REQ-CHART-075, REQ-GUI-227; context: TASK-M2-28, TASK-M8-46)*

*Found merging PR #258 (R-500 to R-505, R-515) into PR #255 (R-469 to R-477, R-506 to R-514). Nothing is chosen.
R-503 flagged this and left it to RQ-372; R-506 has since ruled RQ-372 for the Chart builder's physical-quantity axes
only, so the flag has no entry to wait on.*

- **File, section:**
  - `docs/design/principia_chart_reference.md` § "3.3 The chart map" (R-503): "**The radius is bounded at π (R-503,
    closing RQ-371):** a pixel with `r = ‖d‖ > π`, which `[0,1]²` reaches where `s√2 > π`, is past the antipode of
    `n0`, where the map wraps; it is the domain's end, not a system, out of the count and the total (§5.4)."
  - `decisions.md` § "R-503 — …", "Flagged, not ruled": "a pixel past `r = π` lies inside `[0,1]²` when `s√2 > π`, an
    in-square pixel that is not a system; R-467 makes an in-square validity failure a decoder bug with a console
    warning, and RQ-372 (open, another batch) asks how R-467's in-square rule meets such ranges. R-503 is applied as
    the human wrote it: the domain's end."
  - `decisions.md` § "R-467 — …", the human's words: "Inside [0,1]², every chart is valid by construction" and clause
    4: "Inside [0,1]², a validity failure is a decoder bug: debug view only, plus a console warning naming the chart
    and pixel. The real engine should produce zero."
  - `decisions.md` § "R-506 — …", W1: "Gallery charts warn as R-467 has it, their energy-type axes under the linear
    option excepted (R-471 H5)."
  - `docs/gui/principia_render_gui_spec.md` § "G12. Console (`12_console.png`)": "Inside the square every gallery chart
    is valid by construction, so a pixel the chart classifies as not a system there (a validity failure, or a
    `Chart::map` that fails) is a decoder bug"; REQ-GUI-227: "a pixel a gallery chart classifies as not a system inside
    [0,1]² must log REQ-GUI-226's console warning".
- **Also in the conflict:** REQ-CHART-078, the definition of how pixels exactly on `r = π` count, offers "counted
  once, like the equator, or as the domain's end"; if TASK-M2-28 defines them as the domain's end, those in-square
  pixels meet the same warning.
- **Conflict:** with `s√2 > π` the exponential map classifies its pixels past `r = π` inside the square as not a system
  (REQ-CHART-075), by design; REQ-GUI-227 and §G12 then log "decoder bug" for them on every such view, and REQ-CHART-058's
  sweep ("the real engine should log none") would fail for that chart. R-506 W1 excepts only the energy-type axes'
  linear option.
- **Options seen:**
  - (a) The exponential map's `r > π` pixels are by design, as an energy-type axis's linear option is: they draw blank
    and log no warning; §G12 and REQ-GUI-227 name the exception.
  - (b) The chart keeps the square inside its cap: `s` is bounded so that `s√2 ≤ π`, every pixel inside `[0,1]²` a
    system. Past the square the exponential map is bounded and declares nothing (chart_reference §5.4; R-503, "Past
    the square the map is bounded"), so R-503's `r > π` arm becomes unreachable: REQ-CHART-075's "a pixel with r > π
    must be the domain's end", R-503 review round 2's fixture requirement `s√2·(N−1)/N > π`, and TASK-M2-28's
    acceptance line and its control ("one that counts the pixels with r > π") would be retired or amended, and, if
    TASK-M2-28 merges first, (b) changes built code.
  - (c) The warning stands for them, as R-506 W1 reads literally.
- **Applied meanwhile:** both texts as merged; nothing in the code changes. TASK-M2-28 builds REQ-CHART-075 as R-503
  has it.
- **Waits:** REQ-GUI-227's warning for the exponential map (TASK-M8-46), and REQ-CHART-058's sweep if it covers that
  map with `s√2 > π`. Under option (b): REQ-CHART-075's `r > π` clause, its verify's fixture `s√2·(N−1)/N > π` (R-503
  review round 2) and TASK-M2-28's acceptance line and its control; and REQ-CHART-078's "as the domain's end"
  (physics review 5593301205 of PR #255, P2). TASK-M2-28 does not wait: it builds R-503 as written.

## RQ-471: Lean each axis with one target at `|τ₁| = |τ₂| = 90°`, a reachable corner, maps the whole window onto one line of systems; the corpus gives what the chart shows there no `system_image`, fold or statistics *(silence, R-474 R8 and K15, R-451 B11, R-473, R-408; context: TASK-M2-36)*

*Found answering physics review 5593414323 of PR #255 (P4), while restating the tilted fold's criterion without
dividing by `N_d`. Nothing is chosen.*

- **File, section:**
  - `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)": "**Lean each axis**, an option
    (`lean`), is the formula above per axis, `u₁ = cos τ₁ e_a + sin τ₁ e_X`, `u₂ = cos τ₂ e_b + sin τ₂ e_X`, skewed
    (`u₁·u₂ = sin τ₁ sin τ₂`)"; "The range `τ ∈ [−90°, +90°]` above is Lean each axis's"; "Under Lean each axis an area
    statistic weights each pixel by the Gram factor `√(|u₁|²|u₂|² − (u₁·u₂)²)` (R-408)".
  - `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract": the
    four `system_image` values, "ray-degenerate — whole lines of pixels map to the same system".
  - `docs/design/principia_chart_reference.md` § "3.3 The chart map", the tilted fold (applied per R-369): the fold is
    kept iff `N_b = 0` for `N ∝ u₁ × u₂`.
- **What the corpus does say (gui review 5593448572 of PR #255, G1):** the corner is reachable and handled on the GUI's
  side. `decisions.md` § "R-474 — …", R8: the readout flags "the degenerate corner `τ₁ = ±τ₂ = ±90°`, where the plane
  collapses to a line, which R-451 B11 already handles"; K15: Lean each axis's range is closed at ±90°, and a switch
  to it clamps each angle to ±90°. `docs/gui/principia_render_gui_spec.md` § "G2. …", the compass: The map; the
  Readout's "no plane drawn: s < 10⁻³ at τ₁ = <τ₁>°, τ₂ = <τ₂>° (RQ-340)" line, ending "(Lean each axis)" under that
  map (R-451 B11, R-474); the Drag text: "a tilt-mode drag acts as anywhere else", the GUI's guards declining where
  `s < 10⁻⁶` on the plane's own basis, no `z₀` edit sent (R-451 B11; REQ-GUI-203).
- **Silence (narrowed):** what the chart shows there. At `|τ₁| = |τ₂| = 90°` with `X = Y`, `u₁ = ±e_X` and
  `u₂ = ±e_X`, so `(R·e_a) × (R·e_b) = 0` and the Gram factor is 0: every pixel maps to the slice centre's `(a, b)` and
  `d = ±u ± v`, whole lines of pixels decoding to one system. No text gives that view's `system_image`, its fold
  (the tilted fold's criterion has no normal there) or its area statistics (every pixel's Gram weight is 0). Under
  Tip as sheet `R` is a rotation and keeps rank 2, with two targets the tilted vectors are orthonormal, and an edge-on
  view (`N_d = 0`) keeps rank 2; none of these is in question.
- **Options seen:**
  - (a) The view stays as R-474 and R-451 B11 have it and the chart reports ray-degenerate, its statistics weighted 0
    as the Gram factor says; the label then reads ray-degenerate and every legend share is undefined (0 of a total of
    0), which the legend would have to show.
  - (b) The edit path refuses the second angle at 90° when the first is at 90° with one target (a stop, as R-469 P2
    stops an edit), the readout saying why.
  - (c) Lean each axis's range is open, `(−90°, 90°)`, for both angles together.
  - Options (b) and (c) would amend R-474 R8 and K15, R-451 B11 and §G2's Readout and Drag lines, which reach and
    handle the corner today.
- **Applied meanwhile:** nothing; no task has built Lean each axis's fold.
- **Waits:** REQ-CHART-062's fold at that corner (`rq: RQ-471`; TASK-M2-36 neither builds nor tests it until this is
  ruled); TASK-M2-36 does not wait for the rest.

## RQ-472: R-537 EX7(b) folds the shape sphere's quads over the poles and at the equator with no word on tilt, and RQ-430 (a) narrows only the θ period, where chart_reference §3.3 (#255) keeps the mirror fold under a tilt only where `N_b = 0`, decided exactly on the stored angles, and makes the tilted view bijective elsewhere *(conflict: a ruling against a design doc's applied rule, and silence, R-537 EX2, EX7(b) to (d) and (g), R-538 TF2.2 and J8, R-412, R-473, R-474; REQ-CHART-062, REQ-CHART-079, REQ-CHART-080, REQ-SCHED-101, REQ-GUI-278; context: TASK-M8-56, TASK-M8-58, TASK-M2-36)*

*Found merging main (8484e2f1, PR #255's final rounds) into PR rulings/R-537 (PR #263). Nothing is chosen. For the
human: no unless the human wants it; on a tilted view the fold changes which system a pixel shows, not only the area
statistics, and the orchestrator may rule it as a mechanical consequence of §3.3 and R-472 X3a. *Was:* "it is physics
only in that the fold changes area statistics" (physics review 5594430380 of PR #263, P6).*

- **File, section:**
  - `decisions.md` § "R-537 — …", EX2: "**Pole-crossing** (φ): the chart continues over the pole, with θ shifted by
    half its period." and "**The hemisphere's fold at the equator** (R-412): the mirror hemisphere, the same systems.";
    EX7(b): "**Pole `φ = 0` (`t = 1`):** `iy' = 2n − 1 − iy`, and `ix' = (ix + n/2) mod n`." … "**Hemisphere fold at
    the equator** (R-412, equator at `t = 0`): `iy' = −1 − iy`, `ix` unchanged."; EX7(c): "A canonical quad is in view
    if any of its images is."; EX7(d): "Neighbours cross seams, poles and folds through the same map."
  - `decisions.md` § "R-538 — …", J8: "R-537 EX7(b)'s canonical address applies along an axis only where every term
    with non-zero weight has that period; the untilted sphere's maps stand." (RQ-430 (a), applied meanwhile, which
    asks about θ's period only).
  - `docs/design/principia_chart_reference.md` § "3.3 The chart map", the tilted fold (#255, applied per R-369): "Two
    pixels fold only when their full `(a, b, d)` decode to the same system."; "A pixel's mirror lies on that plane, and
    the pair keeps its fold, exactly when the tilt adds no hidden offset that depends on the mirrored axis:
    **`N_b = 0`**. When `N_b ≠ 0` a pixel and the pixel with its mirrored `(a, b)` differ in `d` and are distinct
    systems: the tilted view is **bijective** there"; "It is an exact comparison on the stored tilt angles": Tip as
    sheet `τ₂ == 0` or `m == 180`, Lean each axis `τ₁ == ±90` or `τ₂ == 0`, two targets `τ₂ ∈ {0, ±180}`.
  - `docs/design/principia_chart_reference.md` § 5.4's table, the exponential map's row (§3.3's tilted fold, read for either map): "under a tilt the fold is the
    tilted view's: the mirror pair keeps its fold iff the tilt adds no hidden offset that depends on the mirrored
    axis".
- **The conflict:** on a tilted shape-sphere view, EX7(b)'s equator map (`ix` unchanged, `iy' = −1 − iy`) folds the
  mirror hemisphere's quads onto the canonical ones whatever the tilt, so the repeat draws, schedules and counts the
  canonical quad's systems; where `N_b ≠ 0` §3.3 makes those pixels distinct systems, each its own. The pole maps,
  a half-period shift of `ix` with a reflection of `iy`, are covered by J8 only in so far as the shift is a period;
  their reflection, like the equator's, is not. The two rules also differ in kind: §3.3 pairs a pixel with whichever
  pixel of the view carries the mirrored `(a, b)` and the same `d` (at Lean each axis's `|τ₁| = 90°` that pixel is at
  another `u`), while EX7(b) maps a quad to the one at the same `ix`.
- **Options seen:**
  - (a) On a tilted view the equator fold (a reflection along `b`) is kept iff §3.3's exact `N_b = 0` condition holds
    on the stored angles; the pole maps (a shift along `a` and a reflection along `b`) are kept only if `d` depends on
    neither window axis, `N_a = 0` and `N_b = 0`, decided the same way; elsewhere the tilted view is affine along that
    axis, no image drawn, as TF2.2 has it for θ; §3.3's pixel fold stands for the statistics (EX7(g)). *Was:* "apply
    only where they are a symmetry of the full `(a, b, d)` at the same window address, decided exactly on the stored
    angles as §3.3 decides its fold" (physics review 5594430380 of PR #263, P6: the pole maps move the address).
  - (b) EX7(b)'s maps apply on every view, tilted or not, and §3.3's tilted fold is read as the in-square statistics
    only.
  - (c) RQ-430 is widened to every map of EX7(b) and ruled with it.
- **Applied meanwhile:** nothing; no task has built EX7(b), and the untilted maps (U10) and RQ-430 (a) for θ stand.
- **Waits:** TASK-M8-56's pole and equator maps on a tilted view (REQ-SCHED-101, `rq: RQ-472`; REQ-REF-054's
  neighbours and REQ-CHART-079's statistics through them, noted), and TASK-M8-58's trace images there (REQ-GUI-278,
  `rq: RQ-472`); neither task waits for the rest. TASK-M2-36's §3.3 fold does not wait. The decoder contract's
  "Repeats and the canonical point" paragraph names this entry (applied per R-369, PR #263's fix round).

## RQ-473: T16 and T16b hold the trace's screen step "within 3× the median" at a θ wrap, a pole passage and a syzygy, where TF2.3 measures correct code at 5–97× the median, genuine fast motion at close approach *(conflict: the human's tests against the human's own measurement, R-538 TF6 T16 and T16b, TF2.3; REQ-GUI-278, REQ-GUI-279, REQ-GUI-283; context: TASK-M8-58)*

*Found by physics review 5594430380 of PR #263 (P1). Nothing is chosen. For the human: no unless the human wants it;
the threshold is a value, and R-71 makes it a calibration requirement meanwhile.*

- **File, section:**
  - `decisions.md` § "R-538 — …", the follow-up's applied text, F6's table: "T16b | Frame: on the sphere tilted toward
    `p_y`, the trace's screen step at a syzygy is within 3× the median | reading `p_y` in the canonical (mirrored) frame
    flips it at every syzygy" and "T16 | No false jumps: on the tilted chart, the trace's screen step at a θ wrap and at
    a pole passage is within 3× the median step | per-sample `[0, 2π)` θ gives a jump of `2π·cos τ`".
  - The same, F2, item 3: "Mixing the two flipped the sign of `p_y` at every syzygy, so the tilted trace jumped there: up to
    ~2000× the median step." and "Read in the sphere's own frame (`ρ` along `+x`, no mirror), the worst jump falls to
    5–97× the median, the size of genuine fast motion at close approach."
- **The conflict:** T16b's fixture, Burrau 3:4:5 to `t = 70`, is full of close approaches near syzygies; read in the
  right frame its worst step is up to ~97× the median by the follow-up's own measurement, so a correct implementation
  fails "within 3×". T16's tilted runs fail it on the same grounds: genuine fast motion at close approach, which F2
  item 3 measures as "5–97× the median, the size of genuine fast motion at close approach", can fall at a θ wrap or a
  pole passage, as it falls at T16b's syzygies. (A pole passage itself keeps the step small: REQ-GUI-278 takes each
  sample on the branch nearest the previous one, over a pole θ + π with φ reflected; TF1's "passes as a jump" is the
  unbranched coordinate's.) The artefacts the tests exist for are far larger (~2000× for the
  frame flip; `2π·cos τ` for the wrap).
- **Options seen:** (a) the threshold is set from the measured gap between genuine motion (≤ ~97× the median, TF2.3)
  and the artefacts (the `2π·cos τ` wrap jump, the ~2000× flip), an R-71 calibration value (REQ-GUI-283); (b) the step
  is compared with the same run read in the chart's own untilted frame, not with the median; (c) 3× as written, on a
  fixture with no close approach.
- **Applied meanwhile:** (a): REQ-GUI-278's and REQ-GUI-279's verifies hold the step within REQ-GUI-283's calibrated
  threshold, their controls unchanged.
- **Waits:** nothing; the threshold is proposed by TASK-M8-58 with its measurement and confirmed at the M8 gate.
- *Was (physics review 5594542116 of PR #263, P8: wrong for the branch-continuous trace T16 tests):* the heading's "and
  TF1 passes θ's pole singularity as a jump" and its "TF1;"; the File, section line "The same, F1's table, θ or φ of the
  shape sphere: "θ is undefined at the poles, a coordinate singularity the line passes as a jump.""; and "A pole
  passage resolved at polar distance `ε` steps θ by about `Δ/ε`, more than 3× the median for small `ε`, so T16's pole
  arm fails correct code too."

## RQ-474: T15's second arm runs "the same tilt toward `p_λ,y`", the physical momentum, where R-473 tilts the shape sphere only toward its latent controls, so no chart offers that target *(conflict: the human's test against a ruling, R-538 TF1 and TF6 T15, R-473 J1 and J2; REQ-GUI-277; context: TASK-M8-58)*

*Found by physics review 5594430380 of PR #263 (P5). Nothing is chosen. For the human: yes if the answer is a new tilt
target; it changes what the Tilt section offers.*

- **File, section:**
  - `decisions.md` § "R-538 — …", the follow-up's applied text, F1: "**Tilted toward the physical momentum:** the same
    tilt never leaves." … "The demo's tabs E and F are that pair."; F6's table: "T15 | A latent-momentum tilt leaves exactly
    while the target's `|q| ≥ q_max`; the physical-momentum tilt never does, over 100 runs".
  - `decisions.md` § "R-473 — …", J1: "on the shape sphere the four momentum controls and the two mass logits"; J2:
    "every hidden coordinate is set and tilted through its latent control (a mass through its logit)".
- **The conflict:** under R-473 J1 and J2 the shape sphere's tilt targets are its latent controls, so a tilt toward the
  physical `p_λ,y` exists on no chart in the corpus; the demo's tab F builds one. T15's second arm has nothing to run on.
- **Options seen:** (a) a physical-momentum tilt target, its definition a requirement (R-72), the arm run on it; (b) the
  arm run on a chart whose shown axis is the physical momentum (a mixed-axis chart, chart_reference §1.3), untilted; (c) the arm
  dropped, TF1's row kept as the reason.
- **Applied meanwhile:** nothing; REQ-GUI-277's latent-momentum arm and its controls stand.
- **Waits:** REQ-GUI-277's physical-momentum arm (`rq: RQ-474`); TASK-M8-58 does not wait for the rest.

## RQ-475: REQ-VAL-022 asks for "equilateral → pole and collinear → `w = 0` exactly", and on floating-point states neither holds bit for bit on the Rust decode and encode, nor can it in general: what does "exactly" mean? *(silence, a missing definition, R-72, REQ-VAL-022, RQ-355; context: TASK-M2-27)*

*Found by TASK-M2-27 while building the `inspector-roundtrip` gate. Nothing is chosen.*

- **File, section:**
  - `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)" (:9): "**Verified
    numerically:** … canonical COM = origin to 1.7×10⁻¹⁶; equilateral → shape-sphere pole and collinear → equator
    exactly."
  - `plan/requirements.yaml` REQ-VAL-022 (:20679): "with the canonical CoM at the origin, equilateral configurations at a
    shape-sphere pole and collinear ones on the equator exactly"; its verify detail: "equilateral → pole and collinear
    → w = 0 exactly".
  - `plan/tasks/M2/TASK-M2-27.md` § "Acceptance tests": "equilateral → pole and collinear → w = 0 exactly
    (REQ-VAL-022)".
- **Why it is open:** no float triangle is exactly equilateral (√3 is irrational), and a collinear state on a line at
  a general angle is not exactly collinear once its coordinates are rounded. Even an exactly collinear float state
  (every `y` equal) is not mapped to `w = 0` bit for bit: `C`'s rotation `R(−φ)`, `φ = atan2(ρ_y, ρ_x)`, rounds
  (`sin π ≠ 0`, R-23's step 2), and the CoM subtraction leaves the three `y` unequal by an ulp. Measured on the f64
  encode, `w` of `C`'s output through `kernel::shape::shape`:
  - equal-mass equilateral triangles, at rest, through a random gauge element (scale `2^[−4, 4)`, any turn, either
    parity, so `L⁺` and `L⁻` both, offsets and boosts on `[−2, 2)²`): `max |w − 1| = 2.2×10⁻¹⁶`, `w = 1` exactly in
    1067 of 2000;
  - collinear states, bodies on the `x`-axis, masses from the mass block at `z_μ` on `[−4, 4)²`, through the same
    gauge: `max |w| = 6.8×10⁻¹⁴`, `w = 0` exactly in 0 of 2000; their `z_β` sits exactly at its clamp image, with its
    `lookup_clamped` bit set, in 2000 of 2000 (the chart's own equator, `β ∈ {0, π̂}` clamped to `ε_z`), and no
    equilateral state's does;
  - exactly collinear float states, every `y` equal, no gauge: `w = 0` exactly in 943 of 992 with `ρ_x > 0` and 340 of
    1008 with `ρ_x < 0`, `max |w| = 8.8×10⁻³¹`.
  The tool's "exactly" was read on its presets; the corpus does not say on which states, or to what, it holds.
- **Options seen:**
  - (a) "Exactly" is "to rounding": a stated bound on `|w − 1|` and `|w|` over random gauge images (a calibration,
    R-71; the measurement above as its evidence).
  - (b) "Exactly" is bit for bit on the canvas presets only (equal-mass unit triangle and bodies on the `x`-axis in
    order, no gauge), with the general case at rounding as in (a).
  - (c) "Exactly" is the chart's: collinear → `z_β` at its clamp image with its bit set, bit for bit (holds on every
    state measured), and equilateral → `(z_α, z_β) = (0, 0)` to rounding.
- **Applied meanwhile:** nothing is judged. `cargo xtask gate inspector-roundtrip` reports both families' `w` and the
  `z_β` clamp count beside its verdict, which judges the round trip and the canonical CoM only. The pole is the
  equal-mass one, as RQ-355 holds it for other masses.
- **Waits:** REQ-VAL-022's two shape checks; REQ-VAL-022 carries `rq: [RQ-355, RQ-475]`, held, not closed, by
  TASK-M2-27's PR (applied per R-369). The round trip, the CoM, REQ-VAL-021 and REQ-VAL-121's proposal do not wait.

## RQ-476: REQ-VAL-022's "2×10⁴ random z with |z| < 4" does not say whether `|z|` is the vector's norm (the ball `‖z‖ < 4`) or each component's (the cube `[−4, 4]⁸`), and the Rust round trip passes the reference's bound under one and fails it under the other *(silence, a missing definition, R-72, R-71, REQ-VAL-022, REQ-VAL-121; context: TASK-M2-27)*

*Found by TASK-M2-27's physics review (5594956833, P1) and qa review (5595411216, Q3) of PR #266. Nothing is chosen.*

- **File, section:**
  - `docs/notes/ic_inspector_scratchpad.md` § "Status — as built (supersedes stale details below)" (:23): "round-trip
    `decode→canonicalise→z` max `‖z−z'‖ = 1.2×10⁻¹³` over 2×10⁴ random `z` (|z|<4; residual
    boundary-conditioning-dominated, as expected)". The domain is written with single bars, `|z|<4`, and the error
    with double bars, `‖z−z'‖`; "boundary-conditioning-dominated" fits draws that reach `|zₖ| → 4`.
  - `plan/requirements.yaml` REQ-VAL-022: "over 2×10⁴ random z with |z| < 4"; its verify detail: "2×10⁴ random z,
    |z| < 4: max round-trip error ≤ 1.2×10⁻¹³".
  - `plan/tasks/M2/TASK-M2-27.md` § "Notes": "|z| < 4 keeps s(1−s) ≥ ~0.018", true under both readings.
- **Measured on the f64 Rust decode and encode, the defaults links** (`‖z − E(D(z))‖`, Euclidean):
  - **ball** `‖z‖ < 4`: 9.73×10⁻¹⁴ at the gate's seed over 2×10⁴; 5.8×10⁻¹⁴ to 1.21×10⁻¹³ over 8 seeds; over 10⁶
    draws max 1.67×10⁻¹³, p99.999 1.09×10⁻¹³, 6 per million past 1.2×10⁻¹³.
  - **cube** `[−4, 4)⁸`: 2.4 to 2.65×10⁻¹³ on every seed over 2×10⁴; over 10⁶ draws max 3.15×10⁻¹³, p99.999
    2.66×10⁻¹³, p99.99 2.42×10⁻¹³, 9150 per million past 1.2×10⁻¹³. The tail is the mass block's `artanh` near
    `|z_μ| → 4`.
  - **The handles gate under the cube reading** (REQ-VAL-021 names no domain; its ICs `g₀·D(z)` draw `z` from the same
    reading): 5.0 to 6.6×10⁻¹⁴ over 2×10⁴ on seeds 1–3, against 1.6×10⁻¹⁴ (174 to 198 handle applications past it),
    and 1.54×10⁻¹³ over 5×10⁵ ICs. qa measured 3.03×10⁻¹⁴ on the Inspector's Randomise ICs kept inside the cube.
    Under the ball it passes: 8.45×10⁻¹⁵ at the gate's seed, 1.02×10⁻¹⁴ over 5×10⁵ ICs.
- **Options seen:**
  - (a) The ball, with the gate at the reference's 1.2×10⁻¹³ on the seeded 2×10⁴ sample.
  - (b) The ball, with the gate above its measured distribution, at or above 1.67×10⁻¹³.
  - (c) The cube, with the gate above its measured distribution (above 3.15×10⁻¹³), and the handles gate's bound
    reconsidered with it (REQ-VAL-121).
- **Applied meanwhile:** (a). `inspector-roundtrip` and `inspector-handles` draw `z` in the ball, the round trip
  gated at 1.2×10⁻¹³; both thresholds stay provisional under REQ-VAL-121.
- **Waits:** REQ-VAL-022 and REQ-VAL-121 carry `rq: RQ-476`, held, not closed, by TASK-M2-27's PR. The gates run
  meanwhile.

## RQ-477: REQ-VAL-021's "2×10⁴ random ICs" does not say what a random IC is, and the Rust handles hold the reference's 1.6×10⁻¹⁴ on one distribution and not on the reference tool's own *(silence, a missing definition, R-72, R-71, REQ-VAL-021, REQ-VAL-121; context: TASK-M2-27)*

*Found by TASK-M2-27's qa review (5595411216, Q2) of PR #266. Nothing is chosen.*

- **File, section:**
  - `plan/requirements.yaml` REQ-VAL-021: "Translate-all, boost-all, rotate-all and scale-all must each leave z fixed,
    to the bound verified in the reference tool (1.6×10⁻¹⁴)"; its verify detail: "2×10⁴ random ICs, each handle
    applied: max ‖Δz‖ ≤ 1.6×10⁻¹⁴ (the reference tool's re-verified bound)".
  - `docs/notes/ic_inspector_scratchpad.md` § "Status — as built" (:23): "all four gauge handles hold `z` to
    `1.6×10⁻¹⁴`", and (:14) "Plus Reset / Randomise / Burrau-ish presets"; no distribution is given.
  - The reference tool's Randomise (`docs/gui/reference/ic_inspector.html`:710–712, record only, R-159): masses on
    `[0.4, 1.8]` (normalised), positions on `±0.9`, velocities on `±0.6`, each uniform.
- **Measured on the f64 Rust encode:**
  - **The gate's distribution**, `g₀·D(z)` with `z` in the ball `‖z‖ < 4` (RQ-476) and `g₀` a random gauge element
    with offsets and boosts up to the state's own size: 8.45×10⁻¹⁵ at the gate's seed over 2×10⁴; 1.02×10⁻¹⁴ over
    5×10⁵ ICs. At offsets and boosts up to twice the state's size, one sample in 2×10⁴ reaches 1.75×10⁻¹⁴.
  - **Randomise** (qa, kept where `‖E(x)‖ < 4`, handles at the tool's button sizes and at random sizes): rotate-all
    1.82×10⁻¹⁴ and scale-all 1.69×10⁻¹⁴ in qa's test run; the worst over seeds 1–5 1.91, 2.48, 1.56, 1.75 and 1.48×10⁻¹⁴, so
    3 of 5 seeds fail, on rotate-all at any turn and on translate-all.
  - **The canonical CoM on a Randomise IC** (qa, exact reading): 1.35×10⁻¹⁵ from the origin, against 1.16×10⁻¹⁶ on
    `C(D(z))`, REQ-VAL-022's domain and 1.7×10⁻¹⁶ bound.
- **Options seen:**
  - (a) The gate's `g₀·D(z)`, `z` in RQ-476's domain.
  - (b) The Inspector's Randomise, where the Rust handles reach 1.82×10⁻¹⁴ (rotate-all) and 1.69×10⁻¹⁴ (scale-all),
    and 3 of 5 seeds fail 1.6×10⁻¹⁴: the bound would need a ruling (REQ-VAL-121), not a loosening here.
  - (c) Both, each with its own bound.
- **Applied meanwhile:** (a). The 1.6×10⁻¹⁴ bound is not loosened.
- **Waits:** REQ-VAL-021 and REQ-VAL-121 carry `rq: RQ-477`, held, not closed, by TASK-M2-27's PR (REQ-VAL-121's
  handles gate value depends on the answer). The gate runs meanwhile.

## RQ-478: R-526 rules a field input "the stain's normalisation", and the design doc §4.4 and §12 q3 take that to be "over the visible range", rescaling while zooming; but the stain normalises by `range_norm`, a field's declared range fixed (`t_end` → lin [0, T]) and only an unbounded field measured over the draw; and which range applies when the stain does not colour by that field *(conflict: the human's words and the bundle against docs on origin/main, render_gui_spec §10.1 and render contract Part 6, R-526, R-486, R-536; context: the plan PR's field-inputs task)*

*Found by physics review 5596610796 of PR #268 (P2). For the human: yes, it decides whether a field-driven medium
rescales while zooming; physics, since it is the field's normalisation.*

- **File, section:**
  - `decisions.md` § "R-526 — …", the human's words: "RQ-390 the stain's normalisation."
  - `docs/design/principia_print_graph.md` § "4.4 Payload fields as inputs": "The field is the same value the stain
    reads, normalised over the visible range with the stain's own window rule." § "12. Open questions …", question 3:
    "A field normalised over the visible range changes its normalisation while zooming, so a "Relief of lifetime"
    relief rescales as you zoom."
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library", `range_norm`: "**`auto = false`** uses
    the **fixed `[lo,hi]`** — a per-field domain **declared in the manifest** … (a `u16` step index
    `[0, horizon_steps]`; …). **`auto = true`** ignores the declared range and uses the **measured min/max** of the
    field over the current draw".
  - `docs/gui/principia_render_gui_spec.md` § "10.1 The shared prelude library", the debug fields' `colour()`:
    "`RANGE_AUTO` is the fixed↔auto flag, editable identically in the node inspector, on the node in the graph, and in
    the code": a setting of the stain's node that colours by the field.
  - `docs/contracts/principia_render_contract.md` § "Part 6 — The debug catalogue (first build target)": "`t_end` → lin
    [0, T]".
- **The conflict:** under the stain's own rule `t_end` has a declared range and does not rescale while zooming, so
  Relief of lifetime would not rescale, against §4.4's "visible range" and §12 q3's premise; the ensemble spread, with
  no declared bound, would. And the stain's `RANGE_AUTO` belongs to the field the stain colours by: when the stain does
  not bind the Print graph's field, there is no stain window to share.
- **Options seen:**
  - (a) Each field's own `range_norm` rule: fixed over its ledger-declared range where it has one, measured over the
    draw where it does not; a field the stain does not bind takes its ledger default.
  - (b) The visible range for every field input (measured over the draw), as §4.4 words it, whatever the stain does.
  - (c) The stain's window when the stain binds the same field, (a) otherwise.
- **Applied meanwhile:** the human's words literally: each field's own `range_norm` rule, fixed where the ledger
  declares a range and measured over the draw where it does not (option (a)). R-526's text no longer decides "visible
  range". What waits: the plan PR's field-inputs task writes the normalisation from the ruling.

## RQ-479: R-488 counts the Print graph's post stage in the tier controller's accounting, and the Auto arbiter's rungs set N, E, depth and FTLE gating, so a heavy Print graph changes numbers the instrument reports, against R-489 and the design doc §3 *(conflict: the human's P4 against the human's P5 and `principia_quality_device_note.md` §3 on origin/main, R-488, R-489; context: the plan PR's post-stage budget task)*

*Found by physics review 5596610796 of PR #268 (P3). For the human: yes, it decides whether a look can change a
measurement.*

- **File, section:**
  - `decisions.md` § "R-488 — …", what it decides (P4): "The post stage is a stage of the telemetry frame record
    (`stage_ms.post`), the tier controller's accounting includes it". `docs/design/principia_print_graph.md` § "8.2 The
    budget": "It is not exempt from the tier controller's accounting; the tier simply has a post-stage share."
  - `docs/design/principia_quality_device_note.md` § "3. The quality ladder": "each a complete preset (N, E, depth,
    render_scale, FTLE/word gating, live-set size) … **The policy's whole output is one integer: the rung.**"
  - `decisions.md` § "R-489 — …", the human's P5: "measurement never reads the print". `principia_print_graph.md` § "3.
    What never changes because of it": "A Print graph cannot change any number the instrument reports."
- **The conflict:** under Auto, a heavy Print graph's cost lowers the resting rung, which changes E (and so the
  ensemble spread's value and presence), N and depth (the Inspector's quad statistics and the area statistics) and can
  gate FTLE out: the print feeds back into measurement through the cost loop.
- **Options seen (the physics review's):**
  - (a) The post stage's cost is excluded from the arbiter's rung solve; only §8.1's own degrade (reduced, then Plain)
    answers it.
  - (b) The post stage counts only toward render-key rungs (render_scale), never E, N, depth or FTLE.
  - (c) As written, with §3's sentence amended to allow it.
- **Applied meanwhile:** none; both texts stand as written. What waits: the plan PR's post-stage budget task and the
  arbiter's accounting of `post`.

## RQ-480: A Chart position target other than the view's own chart: allowed, with the target's inverse generated into the fragment pipeline, or the view's own chart only? *(question, the state map's Q1, R-542, R-116; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13 (the file's "Queue, don't rule"). Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q1. A target other than the
  view's chart.** Recommendation (a): allowed, with the target's inverse generated into the fragment pipeline beside the
  view's. (b) The view's own chart only." SM2: "A target other than the view's chart needs that chart's inverse in the
  fragment pipeline as well (Q1)."
- **Options seen:**
  - (a) Allowed, with the target's inverse generated into the fragment pipeline beside the view's (R-116's one Rust
    source).
  - (b) The view's own chart only.
- **Applied meanwhile:** (a), as the prototype has it (its target menu lists every chart). Reading any chart at all from a
  colour occupant is RQ-489, which TASK-M7-47 waits on.

## RQ-481: Tilted targets: the branch nearest the slice, as prototyped, or not offered as a target? *(question, the state map's Q2, R-542, R-473; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q2. Tilted targets.**
  Recommendation (a): the branch nearest the slice, as prototyped. (b) Not offered as a target." § "SM4. Destination
  side": "a tilted axis (E, F) | not periodic and multi-valued in θ | take the branch nearest the slice (smallest
  out-of-plane `w`); this is a choice, Q2".
- **Options seen:**
  - (a) The branch nearest the slice (the prototype's `smWhere`: `k ∈ [−4, 4]`, the smallest `|w|`).
  - (b) Not offered as a target.
- **Applied meanwhile:** (a).

## RQ-482: The state map and the refinement criterion: never a criterion input, or a spread over it as a candidate signal, measured first? *(question, the state map's Q3, R-542; scheduler contract Part 1; context: TASK-M8-66)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q3. The state map and the
  criterion.** Recommendation (a): never a criterion input (the firewall). (b) A spread over it as a candidate signal,
  measured first." SM3: "It is a colouring: the scheduler's criterion does not read it (the firewall, scheduler contract
  Part 1)."
- **Options seen:**
  - (a) Never a criterion input (the firewall).
  - (b) A spread over it as a candidate signal, measured first.
- **Applied meanwhile:** (a); SM12's Firewall test holds it (TASK-M8-66).

## RQ-483: The vMF shape colouring is not one-to-one: keep it as the hue carrier it is and say so in colour composition §4.1, or also offer SM6's sphere map as a shape colouring? *(question, the state map's Q4, R-542; dd_colouring §3.2; context: none, recorded only)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled, and nothing in the vMF colouring changes
(the human's message: "Q4 concerns the existing vMF shape colouring: record it, change nothing in it").*

- **File, section:**
  - `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q4. The vMF shape colouring.** It is not
    one-to-one, by SM6's theorem. Recommendation (a): keep it as the hue carrier it is, and state that in colour
    composition §4.1. (b) Offer SM6's sphere map as an alternative shape colouring."
  - SM6: "the vMF blend at fixed L puts **antipodal shapes** (chord 2.000) in one bin at κ = 1, 3 and 6 — the pair
    `±(1,1,1)/√3`, both neutral grey".
  - `docs/design/principia_dd_colouring.md` § "3.2 The vMF engine": "fully parameterised by `(L, C, κ)`", and now its
    paragraph "Not one-to-one (R-542; RQ-483)", which records it.
- **Options seen:**
  - (a) Keep it as the hue carrier it is, and state that in colour composition §4.1.
  - (b) Offer SM6's sphere map as an alternative shape colouring.
- **Applied meanwhile:** recorded only, in dd_colouring §3.2. Colour composition §4.1 is not changed until this is ruled;
  the vMF colouring is unchanged.

## RQ-484: Ensemble colour: the nominal copy, or the OKLab mean over the copies, labelled as not one-to-one? *(question, the state map's Q5, R-542, R-102; context: TASK-M7-49)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q5. Ensemble colour.**
  Recommendation (a): the nominal copy. (b) The OKLab mean over copies, labelled as not one-to-one." SM10: "An OKLab
  mean over the `E+1` copies would turn undetermined regions grey, which is informative but not one-to-one (Q5)."
- **Options seen:**
  - (a) The nominal copy (copy 0).
  - (b) The OKLab mean over the copies, labelled as not one-to-one.
- **Applied meanwhile:** (a).

## RQ-485: After its run ends: hold the end state, or paint the outcome palette? *(question, the state map's Q6, R-542, R-95, R-103; context: TASK-M7-47)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q6. After its end.**
  Recommendation (a): hold the end state. (b) The outcome palette." SM5a: "**Ended runs hold their end state
  (absorbing).** … Option: *after its end*, paint the outcome palette instead."
- **Options seen:**
  - (a) Hold the end state (the default), the outcome palette an option.
  - (b) The outcome palette.
- **Applied meanwhile:** (a), as the prototype has it (its "after its end" control defaults to hold; "outcome colour"
  paints the outcome palette at 0.78 of its value).

## RQ-486: History fields (winding on any chart, time in a tile, first passage): out of scope, or specified as accumulators, measured for cost first? *(question, the state map's Q7, R-542; context: none)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q7. History fields**
  (winding on any chart, time in a tile, first passage). Recommendation (a): out of scope; each needs its own
  accumulator, which is a struct change. (b) Specify the accumulators, measured for cost first." SM11: "**Fields that
  need history stay out of scope without a struct change** (Q7)."
- **Options seen:**
  - (a) Out of scope; each needs an accumulator, which is a struct change (SM2).
  - (b) Specify the accumulators, measured for cost first.
- **Applied meanwhile:** (a). No task builds a history field; SM2's constraint stands either way until a ruling changes
  it.

## RQ-487: The tiling defaults and the state map's constants are calibration values: adopt the prototype's as the starting values? *(question, the state map's Q8, R-542, R-71; REQ-COL calibration; context: TASK-M7-48)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q8. Tiling defaults**
  (SM11): pattern, tiles across (6) and the index palette chroma (0.08) are calibration values (R-71). Recommendation
  (a): adopt the prototype's as the starting values. Chart units as the default is settled by §0, not queued."
- **Added by the port (applied per R-369):** the compaction scales `κ` of SM6 (latent 3, momenta 0.3, the tilt's `s` π,
  dilation 0.6) are values the file gives no source for, so they join the same calibration requirement as starting
  values (R-71), as do the layer line width 0.045, the dot radius 0.13 and the plane gradient's `L = 0.75` and chroma
  0.095.
- **Options seen:**
  - (a) Adopt the prototype's values as the starting values, confirmed at the M7 gate.
  - (b) Other values, given at the gate.
- **Applied meanwhile:** (a), the prototype's values, provisional under the calibration requirement.

## RQ-488: The plane's 2D gradient is grey at the chart origin: accept it, or offset the map so its achromatic point falls outside the home square? *(question, the state map's Q9, R-542; context: TASK-M7-48)*

*Filed by R-542's port from `principia_state_map.md` SM13. Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` § "SM13. Queue, don't rule": "**Q9. The plane's grey
  origin.** The linear map is achromatic at the chart origin. Recommendation (a): accept it — on the latent charts the
  origin is the Lagrange landmark, a named place, and it is far in L from the undetermined and no-value colours. (b)
  Offset the map so the achromatic point falls outside the home square."
- **Options seen:**
  - (a) Accept it.
  - (b) Offset the map so the achromatic point falls outside the home square.
- **Applied meanwhile:** (a).

## RQ-489: Chart position reads the target chart and its inverse, and SM11's "a fixed t" reads a state other than the playhead's, where seam 6 and render contract Part 4 keep every chart identifier from a colour occupant, a source included, and SM2 allows only data the fragment already has *(conflict: the human's design against a contract and a seam, and a need for state SimState does not hold; R-542, R-116, REQ-COL-005, REQ-RENDER-015; context: TASK-M7-47)*

*Found by R-542's port. Nothing is ruled. SM2 is the human's hard constraint ("If anything turns out to need a stored
field, stop and queue it; do not add one"): no field is added.*

- **File, section:**
  - `docs/design/principia_dd_colouring.md` § "4. Seams": "| **6** meets compute at the payload only | no colour
    equation references chart, integrator, or scheduler state |".
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules", "The occupant rules `cargo xtask lint
    wgsl` checks": "**A colour occupant** is the occupant of any stain node but `OUT` (Part 2): a source, colour,
    brightness, combiner or post"; "**Chart, integrator and scheduler identifiers** (seam 6) are, in the fragment's
    context: the chart's, the chart lane (`ctx.chart`; …)"; the named exceptions are views "when they are built, the
    DECODE view, its agreement presets and ROUNDTRIP, which read `ctx.chart.z`", each "excepted by the task that builds
    it".
  - `docs/design/principia_colour_composition.md` § "3. The `ctx` contract", "Fragment-side recompute": the fragment can
    "recompute cheap quantities (decode `z` → shape/energy; `encode(decode(z))` residual)". The encode is
    `E = (block/chart inverses) ∘ C` (`principia_dd_encode.md` § "2. Consolidated contract"), so ROUNDTRIP already runs
    the view chart's inverse per pixel.
  - `docs/design/principia_state_map.md` § "SM2": "the encode, including the chart inverses | WGSL generated from the
    one Rust source (R-116); `ROUNDTRIP` already runs it per pixel"; "It is live-compatible: only the playhead state is
    read." § "SM11" and § "SM15": "at (playhead · t = 0 · a fixed t)". The human's message: "position on a chosen chart
    at the playhead (or t = 0, or a fixed t)" and "**SM2 is a hard constraint** … The field is derived at read in the
    fragment stage from data it already has (`r, p` at the playhead, the masses, the `state` lane, the WGSL encode)."
- **The conflict:**
  - Chart position is a source, so a colour occupant; its projection `P_T` reads the target chart (its id, params and
    inverse), which seam 6 forbids an occupant and the lint fails. No exception names it. The view chart's inverse is
    already in the encode `ROUNDTRIP` runs (dd_encode §2); only a target other than the view's chart (Q1, RQ-480) needs
    a second inverse in the fragment. *Was (before PR #270's physics review, P5):* "and the chart inverses are not yet
    in fragment WGSL (`ROUNDTRIP` runs the encode)".
  - "A fixed t" other than `0` and the playhead needs the state at that t. `SimState` holds the playhead's only (payload
    §1), the prototype offers no fixed t (its menu: playhead, `t = 0`), and the state there is not data the fragment
    already has: it needs a stored field (which SM2 forbids) or a second march.
- **Options seen:**
  - (a) A stated seam-6 exception for Chart position, as for ROUNDTRIP and DECODE (render contract Part 4's list and the
    lint's), with the target chart's inverse generated to WGSL from the one Rust source (R-116); a fixed t dropped.
  - (b) The target chart and its parameters as the node's own uniforms (render-side, not `ctx.chart`), so seam 6 is kept
    as written; a fixed t only while the playhead is at it.
  - (c) Chart position not offered until the seam is re-ruled; a fixed t as a second march, measured for cost first.
- **Applied meanwhile:** the playhead and `t = 0` only, no fixed t; no field is added. TASK-M7-47, which builds the
  source, waits on this.

## RQ-495: R-488's whole-graph fallback reads the measured post-stage time each frame, and the quality device note gives every knob one writer, model-based, with no per-frame setting change, and switches the arbiter off under a named tier: who owns the post level, and does the fallback run under a pinned tier *(silence and conflict: the human's P4 against `principia_quality_device_note.md` §1, §2, §5 and §10 on origin/main, R-488, R-499; context: the plan PR's degrade task)*

*Found by perf review 5596612647 of PR #268 (F2). For the human: yes, it decides whether a pinned tier can still be
overridden while moving.*

- **File, section:**
  - `decisions.md` § "R-488 — …", the human's P4: "a whole-graph fallback when over budget".
    `docs/design/principia_print_graph.md` § "8.1 Degrade while moving", step 2: "If the post stage still exceeds its
    share of the frame budget, the **whole graph** drops a level: reduced, then Plain." `docs/design/principia_benchmark.md`
    § "8. Determinism of the work": degrade-while-moving "uses its policy, not a measured budget" in a swept row.
  - `docs/design/principia_quality_device_note.md` § "1. One arbiter, many sensors": "a single policy owns every knob
    write"; § "2. Model-based, not reactive"; § "5. Timescale cascade", the per-frame layer: "**No settings change,
    ever.**"; § "10. Two sanctities": "Named tier or custom → the arbiter is OFF, entirely."
- **The gap:** neither R-488 nor the design doc says whether the post level is the arbiter's lever or a second,
  reactive writer, nor whether the over-budget fallback runs when the user has pinned a named tier or Custom.
- **Options seen:**
  - (a) The post level is one of the arbiter's levers: the fallback runs only under Auto, model-based, and never under a
    named tier or Custom, where only the per-node motion policies (step 1) and settle (step 3) apply.
  - (b) The fallback is part of the Print graph itself, outside the arbiter, and runs under every tier, as §8.1 words
    it; the quality device note's rules are read as covering the tier's knobs only.
  - (c) (b) under Auto, and under a pinned tier the fallback runs only while moving and is recorded, the tier untouched.
- **Applied meanwhile:** none; §8.1 stands as written. What waits: the plan PR's degrade task writes the owner from the
  ruling.

## RQ-496: SM2's "the brightness slot is None" against L-ownership's combiner choice, and SM15's Replace-L chroma scaling against Replace-L as ruled, "`(a, b_ab)` untouched" *(conflict: the human's design against the render contract, a design doc's tests and a ruling's truth table; R-542, R-77, R-64; REQ-COL Replace-L requirements; context: TASK-M7-50, TASK-M7-48)*

*Found by R-542's port. Nothing is ruled.*

- **File, section:**
  - `docs/design/principia_state_map.md` § "SM2", item 3: "**The colour map owns lightness** (SM6), so under
    L-ownership (render contract) it is a full-colour occupant: the brightness slot is None." § "SM11": "The plane's 2D
    gradient … hold L fixed, so a brightness node can take L without losing position", and its Displacement preset puts
    a brightness node after the 2D gradient. § "SM15": "Replace-L scales chroma by `min(1, L/0.75, (1 − L)/0.35)` along
    with L"; "**Backwards compatibility.** Nothing to migrate: every saved stain loads and renders as before"; test (1)
    "Every saved stain renders bitwise the same". § "SM12" Shape n̂ unchanged: "Replace-L now scales chroma (below),
    which changes a sphere stain *with* a brightness node".
  - `docs/contracts/principia_render_contract.md` § "Part 4 — Semantic rules": "**L-ownership (reserved lightness).**
    When a brightness metric is bound, L belongs to brightness; the colour occupant contributes hue/chroma only. …
    a semantic conflict, not a bug, resolved by combiner choice (Replace default; Multiply preserves base L)."
    "Full-colour occupant" appears nowhere on main.
  - `docs/design/principia_dd_colouring.md` § "3.5 Combiners": "`(a, b_ab)` untouched; → RGB. Preserves hue and chroma
    exactly." § "5. Unit tests", 6: "Replace-L leaves `(a, b_ab)` bit-stable".
  - `docs/design/principia_colour_composition.md` § "4.1 Backbone & `Option` occupants": "| C | B |
    `OKLab(L=B, a=Cₐ, b=C_b)` |", and "Replace-L here is `L = B`: the range form … (R-77)".
- **The conflict:**
  - SM2 item 3 makes the brightness slot None for the whole occupant; L-ownership and SM11/SM15 let a brightness node
    follow an L-fixed base, the conflict surfaced as a Problems warning.
  - The chroma scaling changes Replace-L for every stain, not only the state map's: it breaks dd_colouring §3.5 and test
    6, the §4.1 truth table, and SM15's own "renders bitwise the same" for any saved stain with a brightness node
    (SM12 says so).
- **Options seen:**
  - (a) SM2 item 3 read as "an L-spending base takes no brightness node without a Problems warning; Multiply keeps its
    L"; the chroma scaling a Replace-L option (or only on Chart position's bases), the default unchanged, so saved stains
    render bitwise as before.
  - (b) The chroma scaling for every stain, as SM15 has it, with dd_colouring §3.5, its test 6 and the §4.1 truth table
    re-ruled, and SM15's backwards-compatibility test narrowed to stains without a brightness node.
  - (c) No chroma scaling; the 0.30 level's near-black left to the Problems warning.
- **A consequence for the ruling** (PR #270's gui review, note): under main's Replace-L the Mixing preset's "stayed"
  level (`B = 0.30`) clips toward near-black, close to the undetermined `#141418`; TASK-M7-51 builds Mixing before this
  is ruled.
- **Applied meanwhile:** main's Replace-L, unscaled, for every stain; SM2 item 3 read as (a)'s first half (colour
  composition §4.1's R-542 paragraph). The scaling requirement carries `rq: RQ-496`, and TASK-M7-50, which builds it,
  waits on this.

## RQ-497: The stain graph's types and nodes: Shape n̂ is a `vector` field with one colour editor on main, where the state map calls it `point⟨sphere⟩` with Position, Site blend and Bands nodes to swap with; a field subtype is no hard gate on main, where SM15 offers only what the source takes; a source has one param, and band-masks are posts; and the Print graph (PR #268) uses the same words *(conflict: the human's design against the render GUI spec, colour composition and an open branch; R-542, R-64; PR #268; context: TASK-M7-51, TASK-M7-48)*

*Found by R-542's port and its check of PR #268 (`rulings/print-graph`, 1b7ca0ed). Nothing is ruled.*

- **File, section:**
  - `docs/gui/principia_render_gui_spec.md` Part II § "3. The node graph — model": "`field` — a ctx field, **tagged
    with a subtype**: `scalar` / `vector` / `categorical`"; § "9. Node inspector — morphing editors": "| `colour` ←
    `vector` (n̂) | **shape-sphere / SiteBlend editor** |"; § "3.1 Node kinds": "| `source` | — | `field` | one param:
    **which ctx field** (dropdown) |" and "| `post` | … | one op (overlay / band-mask / tone); chained |"; § "6. Port
    typing & wire rules": "**Subtype within `field` is *not* a hard gate:** … the node's inspector **morphs** to the new
    subtype's editor".
  - `docs/design/principia_colour_composition.md` § "7.1 The complete map list": no Shape n̂ node "Position (linear in
    n̂)" or "Bands over n̂"; §1.2's `bands` is a ramp on (θ, φ).
  - `docs/design/principia_state_map.md` § "SM15": "Shape n̂ is a `point⟨sphere⟩`, and its nodes accept exactly that, as
    now"; "Swapping the source between Shape n̂ and Chart position puts each base's counterpart in its place (Position ↔
    Gradient, Site blend ↔ Site blend, Bands ↔ Palette)"; "The library and the drag-off-a-pin menu offer only what the
    wired source takes"; test (3) "A mixed quotient on Displacement is an error". § "SM12" Shape n̂ unchanged: "every one
    of its colour nodes (Position, Site blend …, Bands)". § "SM11": layers are "a `bandmask` … or for site dots a
    `site_overlay`" in the colour slot.
  - Board 4 (`docs/gui/reference/state_map/boards/Ports.dc.html`): "Chart position's chart from Latent to Shape sphere
    and the wire becomes point⟨sphere⟩. The flat base takes its sphere counterpart's place (Gradient → Position, Site
    blend → Site blend, Palette → Bands); the layers have no sphere form and are kept, ghosted". SM11 and the prototype
    instead give Chart position on the sphere its own bases and layers.
  - PR #268 (`rulings/print-graph`): the Print graph's nodes `palette`/`palslot` ("Palette"), `flow` ("Flow"), a field
    input node of type `"field"`, and its stage "post" (the profiler's `stage_ms.post`); print_graph §4.4: "A graph may read a payload field as a
    `scalar` input", by its `range_norm`, which that branch queues. SM14 adds "chart position at the playhead" to payload §5's derived
    table, a `point⟨q⟩` with no ledger range.
- **The conflict:** main's Shape n̂ is `vector` with one colour editor, so the Position and Bands counterparts SM15 swaps
  with do not exist on main (they are the prototype's); SM15's gate against §6's morphing; §3.1's one-param source and
  band-masks as posts against Chart position's four params and its layers inside the colour slot; board 4 against SM11;
  and, on the open branch, two graphs sharing "Palette", "Flow", "field" and "post", and a derived payload quantity that
  could read as a Print graph field input without a range.
- **Options seen:**
  - (a) `point⟨q⟩` a fourth subtype carried by Chart position only, Shape n̂ staying `vector`; Site blend ↔ Site blend
    the only swap until Shape n̂'s Position and Bands nodes are ruled in; §6's morph kept between subtypes and SM15's
    filter applied to menus only; Chart position a source with its own params, its layers combinators inside the
    occupant (colour composition §1.3); board 4's sphere paragraph superseded by SM11; the state map's terms qualified
    in the GUI where both graphs show ("Palette base", "Picture flow"), and Chart position not a Print graph field input.
  - (b) Shape n̂ re-typed `point⟨sphere⟩`, with its Position and Bands nodes added to §7.1 and the golden suite (a change
    to Shape n̂, which §0 keeps "exactly as it is").
  - (c) Each part ruled separately.
- **Applied meanwhile:** (a), in render GUI spec Part II §3, §3.1, §6 and §9 (R-542), except the qualified names, which
  wait for the ruling: the GUI uses the file's names ("Palette") and the code's preset names (RQ-499) meanwhile (PR #270's
  gui review, G2). TASK-M7-51 builds to it; the Shape n̂ unchanged requirement carries `rq: RQ-497`. Whether the graph
  draws the layers as their own node is RQ-502.

## RQ-498: Where Chart position draws a system that is not at a plain chart point: SM1 folds the mirror where SM4 and the prototype's code read the signed n̂, and R-141's decode gauge puts a lower-hemisphere pixel's system on the upper one; mirror pixels inside the square are equal to rounding, not bitwise; the trace draws a left or past-the-end sample as no value, where SM5a clamps; and on an affine chart there is no navigation edge *(conflict: the file against itself and its code, and against rulings, a design doc and an open branch; R-542, R-141, R-412, R-469, R-470, R-503, R-537 EX3 and EX7, R-538 TF1; context: TASK-M7-47, TASK-M7-48, TASK-M7-52, TASK-M8-66)*

*Found by R-542's port and its check of `task/TASK-M2-24` (PR #262), `task/TASK-M2-28` and main; the mirror part
restated and R-141's gauge added after PR #270's physics review (5598441029, P1 and P2). Nothing is ruled.*

- **File, section:**
  - **The mirror, in the file and the code.** `docs/design/principia_state_map.md` § "SM1. Definition": "`E` is the
    encode …: scale and rotation are removed, mirror images identified." § "SM4. Destination side": "| the shape sphere
    (θ, φ with pole crossing) | the sphere, via `n̂` | be a function of `n̂`, never of (θ, φ) |", the unfolded row; only
    "| the hemisphere (R-412) | the sphere mod the mirror |" folds. The prototype's code
    (`docs/gui/reference/state_map/state_flow.html`, `encode`, "shape sphere, before any canonicalisation (§3.1)", and
    `smWhere` case 'B'): `(θ, φ)` from the signed `n̂`, over both hemispheres. Main: `principia_chart_reference.md`,
    "encode's `β = atan2(|λ̃_y|, λ̃_x)` reads `|λ̃_y|`" (the encode's chart point folds); payload §5's derived `n`,
    "`shape(r)` or `shape(r, masses)`", keeps its sign.
  - **R-141's decode gauge.** `decisions.md` § R-141: "the canonical decode gauges λ̃_y → −λ̃_y, so both hemispheres
    decode to the same system". On main a lower-hemisphere pixel decodes to a system with `n₃ ≥ 0`; the prototype's
    `decodeShape` does not gauge (`ψ = atan2(n₃/2, n₂/2)`), so its lower hemisphere decodes to the true mirror system,
    which is where SM7's "sphere 1.5e-6" was measured.
  - **Mirror pixels in the square.** R-141 (the spherical map's two hemispheres decode to one system) and R-503 (the
    exponential map's cap: "a pixel counts once unless its mirror `w → −w` is in view"): each pixel is decoded from its
    own coordinate. R-537 EX7(b)'s canonical address covers repeats outside the primary range. `principia_state_map.md`
    § "SM3": "**only if a copy reuses its canonical quad's samples (EX7's canonical addressing)**"; § "SM12" Copies agree:
    "its IC is **bitwise** equal".
  - **Left the chart.** `docs/design/principia_lines.md` § "8. Traces", "Where a trace can leave (R-538 TF1)": "A
    trace leaves only where a coordinate entering a shown axis with non-zero weight has no value"; the curve axis: "A
    point whose nearest ν lies past a bounded edge is not drawn there, the line broken, with no ring … counted in the
    readout's "past the domain's end: N samples""; a left sample is drawn as a dotted ring, the line dashed until it
    re-enters. `principia_state_map.md` § "SM5": "**at the edge it left by** (default, SM5a)"; § "SM9": "the trace is
    this field for one pixel"; § "SM12" Four results: "neither is drawn as a colour | left drawn as a clamped colour".
    R-470 N1 and R-472 use "past a bounded edge" for a source pixel drawn blank.
  - **The navigation edge.** `decisions.md` § R-469: the centre "accepted only while the chart's centre passes
    `validate`"; R-537 EX3: "The only navigation limit is R-469's … **Affine latent axes:** no stop at all."
    `principia_state_map.md` § "SM3": "on an affine chart it is the navigation limit"; § "SM11": "A system that has left
    the chart takes the cell at the navigation edge it left by".
- **The conflict:**
  - SM1's fold against SM4's unfolded shape-sphere row and the code's signed `n̂`. Under the fold, Chart position ⟨shape
    sphere⟩ and Shape n̂ (signed `n`) read mirror points where `n₃ < 0`, and SM11's "the two read the same point" and
    SM12's Same site blend (for `axes6`, not mirror-symmetric) fail; under the code's reading both hold as written.
  - R-141's gauge: on a both-hemispheres view a lower-hemisphere pixel's system lies on the upper hemisphere, so its
    Chart position at `t = 0` is the mirror of its own pixel's point under either reading, and SM7's identity holds there
    only modulo the mirror; the prototype, which does not gauge, does not show it.
  - SM3's and SM12's bitwise identity holds for EX7 repeats only.
  - SM5a's default draws as a colour what the trace draws as no value, and SM12's Four results test says the opposite
    of SM5a.
  - The latent charts have no navigation edge to clamp to.
- **Options seen:**
  - (a) The code's reading: Chart position ⟨shape sphere⟩ reads the signed `n̂` (payload §5's `n`, no stored field) over
    the full sphere, SM1's "mirror images identified" read for the encode's other coordinates; the `t = 0` identity on
    the sphere stated modulo the mirror (a lower-hemisphere pixel compared with its mirror point, R-141); bitwise copies
    for EX7 repeats, rounding-level for in-square mirrors; the clamp kept as the default, SM9 read for on-chart samples,
    Four results run under "when it leaves: none", where its arm holds as written; the clamp's edge the compacted edge
    `s = ±1` (where main's clamped encode puts it, `|z| ≈ logit(1 − 10⁻⁶)`), and the prototype's base grid a demo device
    (the app's fill is EX4's coarse ancestor and EX7(a)'s roots).
  - (b) SM1's fold: the shape-sphere target is the hemisphere (`E`'s fold), the identity and site-blend tests compare
    folded points and SM11's sentence is read for `n₃ ≥ 0`; "no value" the default, as the trace has it, the clamp an
    option.
  - (c) Each part ruled separately.
- **Applied meanwhile:** (a), the code's reading (the human's message: "Where the md and the code disagree, the code
  wins and the disagreement is queued"). *Was (before PR #270's physics review):* the meanwhile folded the shape-sphere
  target onto the hemisphere, the identity and site-blend tests measured there. The requirements on the identity, the
  copies, the clamp, the four results and the sphere's site blend carry `rq: RQ-498`; their tests run as (a) reads them.
  The hemisphere view's target is RQ-504.

## RQ-499: The prototype's code and the state map's text disagree: preset names, the triangle palette's lightness, Picture under a brightness node, the fade's function, and the Fibonacci cells' swatches *(conflict: the file against its transcription source; R-542; context: TASK-M7-48, TASK-M7-49, TASK-M7-51)*

*Found by R-542's port, reading `docs/gui/reference/state_map/state_flow.html` as source (no browser was available).
The human's message: "Where the md and the code disagree, the code wins and the disagreement is queued, not resolved."
Nothing is ruled.*

- **File, section:** `docs/design/principia_state_map.md` against the prototype's code (each transcribed in the
  section's *Transcribed from the prototype (R-542)* paragraph):
  - **Preset names** (§ "SM15"): "Where …; Graph paper …; Basins …; Displacement …; Flow …; Mixing", and board 5;
    the code's `smPreset` buttons read "Classic shape colour", "Where now", "Graph paper", "Basin tiles",
    "Displacement", "Picture flow", "Mixing".
  - **The triangle palette** (§ "SM11"): "**by index** — … nine OKLab swatches at `L = 0.72`"; the code gives triangles
    `L = 0.75` or `0.69` by orientation.
  - **Picture and brightness** (§ "SM8", § "SM11"): the brightness slot is "outside the colour slot" and "unchanged";
    the code applies no brightness under a Picture base.
  - **The fade** (§ "SM10"): "R-509's distance, as the trace"; the code's fade is `max(0.18, 1/(1 + (d/0.35)²))`, the
    trace's `max(0.18, e^{−d/1.2})`.
  - **Fibonacci cells** (§ "SM11"): "**by index** … or **golden-angle** hues by tile"; the code colours Fibonacci cells
    golden-angle always.
- **Options seen:**
  - (a) The code, as transcribed: these are the behaviour, and the file's words are read through the transcriptions.
  - (b) The file's words, the prototype's code a bug.
  - (c) Each ruled separately.
- **Applied meanwhile:** (a), the code (the human's message). The presets requirement carries `rq: RQ-499` for the
  names; TASK-M7-48, TASK-M7-49 and TASK-M7-51 build the code's behaviour.

## RQ-500: Board 6's "Edit graph…" in Explore's top bar is the label PR #268 gives the Print panel's button *(conflict: the human's board against an open branch; R-542, PR #268; context: TASK-M8-66)*

*Found by PR #270's gui review (5598419574, G1). Nothing is ruled.*

- **File, section:** `docs/gui/reference/state_map/boards/` board 6 (Explore): the top bar's "Stain [Chart position]"
  and "Edit graph…", which opens the stain editor. PR #268 (`rulings/print-graph`), the Print panel: its button "Edit
  graph…" opens the Print graph's editor.
- **The conflict:** one label, two editors, both reachable from Explore.
- **Options seen:**
  - (a) Each qualified by its graph: "Edit stain…" in the stain's place, the Print panel's "Edit graph…" as PR #268 has
    it (or "Edit print graph…").
  - (b) Board 6's label kept, PR #268's renamed.
  - (c) Board 6's button not built: the stain editor is reached as it is on main (§G2, `02_stain.png`).
- **Applied meanwhile:** (c) until ruled: the shell follows `01_main.png` (render GUI spec §G13, R-542's bullet), so no
  "Edit graph…" is added to Explore's top bar by the state map.

## RQ-501: Board 6 draws a ring on the hovered pixel with no hover trace, and its hover label has no "L listen", where R-541 and §G2 keep both *(conflict: the human's board against rulings; R-542, R-540, R-541, §G2; context: TASK-M8-66)*

*Found by PR #270's gui review (5598419574, G1). Nothing is ruled.*

- **File, section:** board 6 (Explore): the figure shows a ring at the hovered pixel and no trace; the hover label
  reads the system without "L listen". `principia_state_map.md` § "SM9": "The prototype draws no trace, only a ring on
  the hovered pixel" (it does not say the app drops the trace). `docs/gui/principia_render_gui_spec.md` § "G2. Explore —
  the everyday view (`01_main.png`)": the hover trace and its label; R-541 G1–G4 and R-540: the label's listen hint.
- **The conflict:** the board, read as a layout, would drop the hover trace and the listen hint under a Chart position
  stain.
- **Options seen:**
  - (a) The hover trace and its label stay as R-541 and §G2 give them, and the figure gets no ring while the trace is
    shown: the trace's start star marks the pixel, and SM9's marker is the legend's ring (render GUI spec §G13's R-542
    bullet; TASK-M8-66).
  - (b) Board 6's ring on the hovered pixel: under a Chart position stain the trace is hidden and the ring replaces it,
    as board 6 draws.
  - (c) Both: the ring added on the figure beside the trace, written into §G2 with its key entry (a figure mark under
    R-539, in R-541 P13's trace block).
- **Applied meanwhile:** (a). *Was (before PR #270's gui re-review, G7):* "(a) The hover trace and its label stay as
  R-541 and §G2 give them; the ring is added on the hovered pixel beside them, and the legend's ring marks the system
  (SM9)."

## RQ-502: Whether the stain graph draws Chart position's layers as their own LAYERS node, as boards 1 and 4 do, or inside the colour occupant, as render GUI spec §3.1 and §9 now say *(conflict: the human's boards against the applied text; R-542, R-64; context: TASK-M7-51)*

*Found by PR #270's gui review (5598419574, G4). Nothing is ruled.*

- **File, section:** board 1 (editor) draws BASE and LAYERS as two graph nodes, LAYERS with in-ports `base` (vec3) and
  `point`; board 4 (ports) lists "Layer … base: vec3 · point → vec3". `docs/gui/principia_render_gui_spec.md` Part II
  § "3.1 Node kinds" (R-542 paragraph): "Its layers are combinators inside the colour occupant (colour composition
  §1.3, §1.5), not `post` nodes"; § "9. Node inspector — morphing editors": one base-and-layers editor (board 3). §3.1's
  table has no kind with a `vec3` in-port ahead of the combiner. `principia_state_map.md` § "SM15": "the inspector shows
  them as a stack under the base, reorderable, each removable."
- **The conflict:** the graph either shows one colour node with the stack in its inspector, or a base node feeding a
  layers node with a `vec3` in-port, a node kind §3.1 does not have.
- **Options seen:**
  - (a) One colour node; the base and the layer stack in its inspector (§3.1, §9, board 3, SM15's text), boards 1 and
    4's LAYERS node read as a drawing of that stack.
  - (b) A LAYERS node kind with a `vec3` in-port ahead of the combiner, added to §3.1 (an extension of R-64).
- **Applied meanwhile:** (a): the graph draws one colour node for the base and its layers; REQ-GUI-335's screenshot
  is read against board 3's inspector and board 1's library and Problems rows, not board 1's two nodes.

## RQ-503: The prototype's latent chart uses `AMIN = 0.05`, where R-21 sets `α_min = 0`, so SM5a's latent-chart figures and "it leaves because α passes the decoder's limit" describe the prototype, not the app *(conflict: the transcription source against a ruling; R-542, R-21; context: TASK-M7-47, TASK-M7-52)*

*Found by PR #270's physics review (5598441029, P3). Nothing is ruled.*

- **File, section:** `docs/gui/reference/state_map/state_flow.html`: `AMIN = 0.05`, its latent decode and encode
  `α = AMIN + (π/2 − 2·AMIN)·σ(z_α)`. `decisions.md` § R-21 and `docs/design/principia_chart_reference.md` § 0.2: `α_min
  = 0` ("full-sphere coverage"); main's latent α axis runs out only at α = 0 or π/2 (λ̃ = 0 or ρ̃ = 0), within
  `ε_z = 10⁻⁶`. `principia_state_map.md` § "SM5a": "On the latent chart it leaves because α passes the decoder's limit";
  "On the latent chart at `t = 8`, 400 random systems, 129 of them left"; "On the latent chart, 3186 of 9216 have
  **left**".
- **The conflict:** on main almost no system leaves the latent chart off a collision, so SM5a's latent figures and a
  test built on them (a population of left systems on the latent chart) would be empty and could not fire.
- **Options seen:**
  - (a) Main's `α_min = 0` stands; SM5a's latent figures are the prototype's, reference only; the edge-clamp test runs
    on a target that leaves on main (the latent momenta at the clamp, or the `z_q3` tilt), its figures measured on the
    app; the latent α case is tested on a constructed state (λ̃ or ρ̃ within `ε_z` of 0).
  - (b) Chart position's latent target uses the prototype's `α_min = 0.05` (a second latent chart, a chart change).
- **Applied meanwhile:** (a). REQ-COL-116's verify is on the latent momenta; REQ-COL-113 constructs the α case.

## RQ-504: The one-hemisphere view's Chart position target: the prototype targets the full sphere, SM4 the sphere mod the mirror *(conflict: the file against its code; R-542, R-412; context: TASK-M7-47)*

*Found by PR #270's physics review (5598441029, P1). Nothing is ruled.*

- **File, section:** `principia_state_map.md` § "SM4. Destination side": "| the hemisphere (R-412) | the sphere mod the
  mirror | be a function of the canonical (folded) `n̂` |". The prototype's `smTarget`: "the view's own" on the
  one-hemisphere view resolves to the full sphere (transcribed in SM1's *Transcribed from the prototype* paragraph).
- **The conflict:** the code colours a hemisphere view's systems by their signed `n̂` on the full sphere; the file folds
  them.
- **Options seen:**
  - (a) The code's: the one-hemisphere view targets the full sphere, signed `n̂` (with RQ-498 (a)).
  - (b) The file's: the sphere mod the mirror, the canonical folded `n̂`.
- **Applied meanwhile:** (a), the code (the human's message).

## RQ-505: "'none' must be available as a debug option" (the bundle's draft requirement) against SM5, SM11 and SM15, which offer "none" as an ordinary setting of the node *(conflict: the human's draft against the file and its code; R-542; REQ-COL-116; context: TASK-M7-47, TASK-M7-51)*

*Found by PR #270's code review (5598464229, 3). Nothing is ruled.*

- **File, section:** `docs/incoming/2026-10-21_state_map/requirements_patch.yaml`, the edge-clamp draft: "A system that
  has left the chart must be drawn at the edge it left by, with no stored history; 'none' must be available as a debug
  option." `principia_state_map.md` § "SM5": "**at the edge it left by** (default, SM5a); or no value, in its own
  style"; § "SM11" and § "SM15": "when it leaves the chart (clamp to the edge · none)", a node setting; the prototype's
  "When it leaves" control offers both.
- **The conflict:** debug-only, or an ordinary node setting.
- **Options seen:**
  - (a) An ordinary setting of the node, as the file and the code have it (which includes the debug use).
  - (b) Debug only: offered in the debug views, not in the node's inspector.
- **Applied meanwhile:** (a), the code. REQ-COL-116 keeps the draft's words, its note citing this entry.

## RQ-520: The frame record's `substep_histogram` is ruled `[u32; 16]` with "equal bins of width `N_max/16`" and "cap hits as their own last bin", but sixteen bins of width `N_max/16` fill `[0, N_max]` and leave no entry for a separate cap bin *(conflict within the human's ruling on RQ-286; R-545, R-288, REQ-TOOL-098, REQ-TOOL-187; context: TASK-M5-28, TASK-M8-28)*

*Found porting R-545 (the human's batch of 21 Oct 2026). Nothing is chosen.*

- **File, section:**
  - `docs/incoming/2026-10-21_rulings/rulings_2026-10-21.md` line 17, the human's ruling on RQ-286: "The frame record
    gains `substep_histogram: [u32; 16]`: equal bins of width `N_max/16`, with cap hits as their own last bin. It is
    read back with R-288's counters on the existing readback. This is a schema v1 change made by this ruling."
  - `docs/design/principia_dd_telemetry_and_tiers.md` § "2. What to record — and the rule that makes it useful", the
    `substep_histogram` line (R-545); `docs/gui/principia_render_gui_spec.md` § "Profiler": "a substeps-per-pixel
    histogram, with the cap marked".
  - The mock (`crates/gui/src/mock/profile.rs`, `SUBSTEPS_PER_PIXEL`, TASK-M6-28's PR #212) draws placeholder bins of 4
    substeps up to `N_max` 64, the cap marked at 64: sixteen bins of width `N_max/16`, with no separate cap bin.
- **The conflict:** sixteen equal bins of width `N_max/16` cover `[0, N_max]`, so a cap hit (a count of `N_max`) falls
  in the sixteenth with every count in `[15·N_max/16, N_max)`, not in a bin of its own; a bin of cap hits alone needs a
  seventeenth entry, or bins of another width. The words do not say which, nor which side of each edge a count on it
  falls.
- **Options seen:**
  - (a) `[u32; 17]`: sixteen equal bins of width `N_max/16` over `[0, N_max)`, half-open, and the cap hits (`N_max`) in
    a seventeenth.
  - (b) `[u32; 16]`: fifteen equal bins of width `N_max/15` over `[0, N_max)`, half-open, and the cap hits in the
    sixteenth.
  - (c) `[u32; 16]`: sixteen equal bins of width `N_max/16`, the last closed at `N_max`, so the cap hits are counted in
    the last bin with the counts just below the cap, and the Profiler marks the cap at its top edge (the mock's
    layout).
- **Applied meanwhile:** telemetry §2 and §5 carry `substep_histogram: [u32; 16]` as ruled; REQ-TOOL-187 (TASK-M5-28)
  builds the field, its GPU counting and its readback, and waits on this entry for the bin edges only.
- **Waits:** REQ-TOOL-187's bin edges (TASK-M5-28) and REQ-TOOL-098's drawing of them (TASK-M8-28).

## RQ-521: Under R-548's E ≥ 0 guard, an E < 0 state that passes the dispersal step's geometric tests goes on to the hierarchy step, where it may read HE, PE or excursion, but the human's words say it "falls through to B-stable, OS-candidate or B-so-far" *(physics, silence: the human's ruling against the option it adopts; R-548, R-442; REQ-PAY-099; context: TASK-M7-35)*

*Found porting R-548 (the human's batch of 21 Oct 2026). Nothing is chosen. For the human: yes, as physics where the
choice changes results (R-369): it decides which label a rare E < 0 state reads, and the texts do not settle it.*

- **File, section:**
  - The human's ruling on RQ-325 (`docs/incoming/2026-10-21_rulings/rulings_2026-10-21.md`, line 37; R-548): "(b). "H,
    HP, P and the two-stage H each require `E ≥ 0`. An E < 0 state that passes their geometric tests falls through to
    B-stable, OS-candidate or B-so-far. …""
  - RQ-325's option (b), which the ruling adopts: "Guard H, HP, P and the two-stage H with `E ≥ 0`, so an E < 0 state
    that passes their geometric tests falls through to the later steps (B-stable, OS-candidate or B-so-far)."
  - `docs/design/principia_dd_chazy_final_motion.md` § "3.2 The rule, in order", the order `classify` evaluates:
    step 1 (dispersal) ends "Otherwise go on"; step 2 (hierarchy) reads HE, PE, B-stable or excursion when its gate
    passes and the outer body recedes with the pair bound (`K_pair < U_pair`); steps 3 to 6 are transient, B-stable,
    OS-candidate and B-so-far.
- **The gap:** the two-stage H's guard sends a guarded state on to step 3, whose outcomes at E < 0 are exactly the
  three the words list. Step 1's guard sends a guarded state on to step 2. Step 1's geometric tests (every separation
  above `h_far · R_0`, every pair receding, no pair's band `−1`) leave a pair with `1 − ε ≤ K/U < 1`, bound by §3.4's
  sign test, so step 2 can read HE or PE (or B-stable or excursion) for it when its hierarchy gate passes. That label
  is not in the words' list, though PE and HE are allowed at E < 0 (§2's table). The homothetic fixture (R-548's
  §12.2 item 11) reads B-so-far under either reading (its hierarchy ratio is about 0.87, below `h_far`), and no fixture
  reaches the case.
- **Options seen:**
  - (a) The later steps from step 2 on: a guarded dispersal state goes on to the hierarchy step, as `classify`'s
    "Otherwise go on" and option (b)'s "the later steps" read; it may read HE, PE, B-stable or excursion there.
  - (b) The words' list: a guarded dispersal state skips the hierarchy step and goes on to step 3, so it reads
    B-stable, OS-candidate or B-so-far only.
- **Applied meanwhile:** (a), the drill-down's §3.2 as R-548 applies it (decided per R-369, R-548 C2); REQ-PAY-099
  carries this entry for that clause only. Nothing else of TASK-M7-35 waits.
