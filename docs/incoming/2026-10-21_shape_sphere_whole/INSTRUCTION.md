# The shape sphere is the whole sphere

A new instruction from me, Malachy, 21 Oct 2026. This is my ruling; quote §0 verbatim wherever it is recorded.

## 0. My words

> "One thing I don't really get why is the thing actually now thinking about it more is on the shapesphere chart, the
> one hemisphere mode doesn't actually make any sense. It makes sense in terms of the initial condition inspector,
> right? When it's like how it's the same system, right? Reflected, right? Because of how the like angle wrapping or
> like symmetry reduction works and like the initial condition like generator thing. It makes sense there. But in terms
> of like an actual chart that the Explorer uses and the Atlas uses and the Trace system uses and the Listening system
> uses, basically just what everything else uses, it doesn't actually make any sense because, especially with traces,
> like the system can go through the angles on the opposite hemisphere, on like the other side of the hemisphere. So
> like removing it just doesn't make any sense because you're like cutting away like an entire set of angles and like
> reflecting, and you're sort of lying because it did actually go through that angle. if you get me?"

And, to the proposal in §1–§2 below:

> "yes"

## 1. The reasoning I am ruling on

- **The mirror is a symmetry of fates, not of the space systems move through.** Reflecting a system (positions and
  velocities) is an exact symmetry of Newtonian gravity: a system and its mirror twin have the same fate, at the same
  time, with the same bodies. That is what R-141 and R-412 used, and for the inspector ("this is the same system,
  reflected") it stays true.
- **As a chart it is wrong.** The two hemispheres are the triangle's two orientations and the equator is the collinear
  configurations. A trajectory crosses the equator at every syzygy, so folding turns each crossing into a bounce at the
  equator: a trace kinks where the system went straight through, and a system is placed at its mirror twin's angles.
- **Even for initial conditions the fold is only honest for mirror-invariant fields** (fate, `t_end`, spread).
  Signed quantities (`L_z`, winding direction, Chart position's signed n̂) change sign under the mirror.
- **The root is R-141's decode gauge.** The canonical decode gauges `λ̃_y → −λ̃_y`, so a lower-hemisphere pixel decodes
  to the *upper* system. With both hemispheres drawn, a pixel's coordinate is still not its own system's n̂. That is
  the tangle RQ-498 records.
- **Precedent already on main:** `principia_listen.md` §1 builds the shape vector "from the state with no canonical
  frame (a canonical frame mirrors at every syzygy and would put a jump into the signal)". Listen is already right;
  this ruling makes the chart agree with it.

## 2. The ruling

1. **The shape sphere chart is the whole sphere, everywhere** (Explore, the Atlas, traces, Listen, the state map,
   prints, the batch job). Pixel `(θ, φ)` decodes to the system whose **signed** n̂ is that point. R-14's
   `φ = π·(1 − t)`, pole to pole, is the only φ span.
2. **R-141's decode gauge is removed for the shape sphere.** A lower-hemisphere pixel decodes to the true mirror
   system. The chart's `system_image` is one-to-one over the sphere (still n-to-1 over its genuine identifications:
   θ's period and the poles). R-157 (`DoubleCover` for the full-range Burrau chart) is untouched: that is a relabelling
   of bodies, a different thing.
3. **The hemisphere toggle is removed** from the Manifold view's Chart section (R-113's placement, render_gui_spec
   §G2). R-412's remap is withdrawn with it.
4. **The mirror is no longer a repeat.** R-537's repeats on the shape sphere are θ's period and the pole crossing
   only; a pixel past the equator is its own system, with its own pick, readout, trajectory, outcome and inspector.
5. **The mirror becomes a test, not an identification.** Where every other coordinate the chart holds fixed is
   mirror-equivariant (released from rest is), mirror-invariant fields must be exactly symmetric across the equator and
   signed fields exactly antisymmetric. Where a fixed coordinate is not mirror-equivariant (a fixed `L_z ≠ 0`, say), the
   two hemispheres are genuinely different systems and the test does not apply; the test must say which case it is in.
6. **The fold survives in two places only:**
   - **The IC inspector** notes a system's mirror twin ("mirror twin of the system at (θ, π − φ)").
   - **An optional compute saving** (EX7's fold, R-537): for a field declared mirror-invariant, on a chart whose fixed
     coordinates are mirror-equivariant, integrate one hemisphere and reflect. Exact, so allowed. Never for traces,
     Listen, the state map, or any signed field. Off by default until the test in item 5 passes on the build.
7. **Area statistics** are counted over the whole sphere. Where item 5's symmetry holds, every fate is counted twice
   equally, so fractions are unchanged; state that beside any statistic that was measured under one hemisphere.
8. **Traces on the shape sphere** use the signed n̂ over the whole sphere, as Listen already does. "Show the canonical
   bodies" in the trajectory panel (trajectory_viewing pane 2) may stay as a view of the bodies, but must not fold the
   path.

## 3. What this touches

Find every site; these are the ones known:

- `decisions.md`: R-113 (the toggle's placement), R-141 (the gauge and the 2-to-1 cover), R-412 (the remap), R-537 EX7
  (the hemisphere fold in the repeat group), R-59 D5 where it cites the 2-to-1 cover.
- `docs/contracts/principia_chart_decoder_contract.md` Part 1 ("Shape-sphere redundancy … Render one hemisphere or flag
  the redundancy"), Part 5, and the "Past the unit square" and "Repeats and the canonical point" sections.
- `docs/design/principia_chart_reference.md` §3.3 (hemisphere redundancy, the toggle), §4.5, §5.4 (the sphere's row)
  and the extension-type table.
- `docs/gui/principia_render_gui_spec.md` §G2 (the toggle).
- `docs/design/principia_trajectory_viewing.md` pane 2.
- The state map: RQ-498 and RQ-504. My drafts there already chose the full sphere with signed n̂; this ruling settles
  them the same way, and SM1's "the sphere mod the mirror" row and the "one-hemisphere view" go.
- Requirements citing any of the above (REQ-CHART-054, -056, -057 at least), via `plan/tools/reqio.py`.
- Code: the canonical decode's gauge, the hemisphere toggle, R-412's remap, EX7's equator fold in the scheduler and
  cache, and any test that asserts the 2-to-1 cover.

## 4. Tests (each with the arm that fires)

| test | asserts | fires on |
|---|---|---|
| Signed decode | for a sweep of `(θ, φ)` over the whole sphere, the decoded system's signed n̂ equals the pixel's point to f64 round-trip | the gauge left in (every lower pixel lands at `π − φ`) |
| Mirror symmetry | released from rest: outcome and `t_end` equal at `(θ, φ)` and `(θ, π − φ)`, bitwise where the integration is mirror-exact, else to rounding; `L_z` and winding opposite | a decoder or momentum rule that breaks the mirror |
| Mirror control | on a chart holding `L_z ≠ 0` fixed, the test reports "not mirror-equivariant" and does not assert | a test that passes by assuming the symmetry |
| Trace through a syzygy | a run with a known syzygy: its traced n̂₃ changes sign through 0 with a nonzero derivative, no kink | a folded path |
| No repeat at the equator | picking at `(θ, π − φ)` returns that system, not its twin | R-537's equator fold left in |
| Fold saving is exact | where item 6 enables it, the folded field equals the fully integrated one | the saving applied to a signed field |

## 5. How to apply

- Record this in `decisions.md` from the next free R-number, quoting §0, naming the rulings it amends or withdraws
  (R-113's toggle, R-141, R-412, R-537 EX7's equator fold) and the entries it closes (RQ-498 and RQ-504, where they
  turn on the hemisphere).
- Spec, requirements and tests as above; then `check_plan`.
- Results measured under one hemisphere are kept, with this ruling's number beside them; do not re-measure them in this
  PR. List them in the PR so I can decide which to re-run.
- Where this conflicts with a ruling or a queue entry these texts do not address, queue it rather than resolving it.
- One PR. Paste the test output in it.
- The security rule stands: never delete or modify anything outside the repo and build/scratch directories without
  asking.
