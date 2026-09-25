# IC Inspector — design scratchpad

*Working name; rename candidate. Static single-sample tool. Not spec yet.*

## Status — as built (supersedes stale details below)

Built and verified: `ic_inspector.html`, single file, hand-rolled HTML/canvas/JS (not Tweakpane), ships as a standalone artefact. Ports the canonical-frame decode (`principia_dd_decoder.md` §3) and its inverse (`principia_inverse_encode_contract.md` Part 3) verbatim, CPU-side f64.

**Verified numerically:** round-trip decode→canonicalise→z to 5×10⁻¹⁵; gauge invariance (translate/rotate/scale/boost leave z fixed) to 2×10⁻¹⁶; canonical COM = origin to 1.7×10⁻¹⁶; equilateral → shape-sphere pole and collinear → equator exactly.

**Decisions taken during build (differ from / extend the design below):**
- Editing canvas is **velocity-only**; the canonical pane always shows **momentum** (the chart's actual input, the Jacobi q). The v↔p toggle is gone.
- **Jacobi frame and vertex angles are always on** — no toggles. COM drawn and labelled on both panes; interior angles drawn at each vertex on both triangles.
- Whole-system handles are **buttons** (Rotate 15° / Scale ×1.25 / Boost), not rings; translation is body-dragging. Plus Reset / Randomise / Burrau-ish presets.
- Both panes **auto-fit**; canonical pane carries a `√I = 1` stamp — the pinned quantity is the mass-weighted hyperradius, not the particle-position scale.
- Shape-sphere mode is a full **equirectangular map**: **mass-weighted** Hopf map n = (cos2α, sin2α cosβ, sin2α sinβ); **θ azimuthal** horizontal (0–360°), **φ polar** vertical (0–180°, L⁺ top / L⁻ bottom); collinear equator; **binary-collision landmarks computed from the actual masses**; lower half shaded mirror-folded.

**Verification flags — RESOLVED (checked against the `.md` decode/encode corpus; fixes applied to the HTML):**
- **Momentum decode convention — CONFIRMED correct.** dd_decoder §3.4 maps the 4 controls → **physical** Jacobi momenta `(p_ρ, p_λ)` directly (`qₖ = q_max·(2σ(z_qk)−1)`), *no* `√μ` unweighting. The asymmetry with the position path is intended: config detours through mass-weighted Jacobi `ρ̃` only because the shape sphere and `I = |ρ̃|²+|λ̃|²` live there, then unweights to physical `ρ`; momentum is conjugate to that physical `ρ`, so it stays physical Jacobi momentum. The literal port of the momentum decode (`principia_dd_decoder.md` §3.4) is right.
- **Chart constants — one was wrong, now fixed.** `q_max = 2` ✓. **`μ_max` was 4 → corrected to 5** (dd_decoder §3.1, chart_decoder §87/96, inverse_encode §63; at 4 the mass-saturation range was too narrow and `MASS_SAT` fired at the wrong `z_μ`). The block-inverse **ε clamps were hardcoded `1e-9` → corrected to `ε = 1e-6`** (`ε_z=ε_μ=ε_q`, chart_decoder §87 / inverse_encode §63) so the SAT flags fire at the chart's true boundary. **`α_min` REMOVED** (chart-definition change, propagated). It was never a numerical guard — nothing divides by α; it only excised a ~3° polar cap off each pole. Dropped for full-sphere coverage: `α = (π/2)·σ(z_α) ∈ (0, π/2)`, inverse `s_α = 2α/π`. The two poles (`λ̃=0` / `ρ̃=0`) are now represented; their degeneracies (β/φ undefined, inner-pair collision) are fenced by the conditioning readout, SAT flags, and COLLISION detection — **verified**: exact-pole configs give finite, SAT-flagged `z` (`a = ±13.816`), no NaN. Propagated to decode/encode docs (dd_decoder §3.2, dd_encode, inverse_encode); pending change 6 is its register entry.
- **Mirror deadband — added.** The one mirror test (R-82, in encode's frame): mirror iff `λ̃_y < −δ_λ`, `δ_λ = 10⁻¹²`, and `|λ̃_y| ≤ δ_λ` → no mirror; the code had a bare `λ̃_y < 0` that flaps sign at the collinear seam. Now `λ̃_y < −δ_λ`.

*Fixes re-verified in node after patching: round-trip `decode→canonicalise→z` max `‖z−z'‖ = 1.2×10⁻¹³` over 2×10⁴ random `z` (|z|<4; residual boundary-conditioning-dominated, as expected), and all four gauge handles hold `z` to `1.6×10⁻¹⁴`. μ_max=5 + ε=1e-6 regressed neither.*

**Convention call — RESOLVED: keep mass-weighted.** The mass-weighted landmarks *are* the actual collision loci, so "the dot lands on a BC marker exactly when two bodies coincide, at any mass ratio" is a **correctness invariant** — the same collision-locus agreement the tool exists to check — not merely a stylistic match to the `(α,β)` decode. Geometric/pinned landmarks would be a *different* chart and would break that property for unequal masses. (Reach for a geometric, mass-invariant reference only if a future cross-ratio comparison viz wants one — a separate tool, not this audit.)

**Deferred (designed below, not built):** g-transform ghost animation; round-trip-residual ghost overlay (only the OUT_OF_CHART flag exists, not the drawn ghost); live conditioning-number readout; direct-physical-inject flip + two-path cross-check; both main-view crosshair bridges (no main view in the standalone tool).

## Purpose

The round-trip test made continuous. Static checks sample points; dragging sweeps continuous paths through physical IC space and watches the chart trace them — which is where conditioning failures and branch cuts actually surface. Three jobs at once:

1. **Pipeline test** — exercises the real encode / canonicalise / decode / conditioning code, so it's a test, not a viz of a reimplementation.
2. **Visual function test** — you *see* the gauge bookkeeping hold or leak.
3. **Research instrument** — inspect, perturb, and read out any physical IC; bridge a config you have in mind to where it lives in the fractal.

No live-march, no tile scheduler, no barrier. Single sample, one `SimState`. Cheap to stand up as a standalone artefact in the dev-GUI phase.

## The quotient set (the thing being audited)

**Fixed by the reduction (gauge / frame — invisible to z):**

- COM position = 0 — translation, via Jacobi coords
- total **P = 0** — COM frame / Galilean boost
- rotation — ρ̃ pinned to +x
- scale — R̃ = 1

**Free on the 8D manifold (physical — z reads them):** shape (α, β), the 4 momentum DOF, both masses. **E and L are free-valued functions on the manifold — not fixed, just reported.**

> Correction to last note: I said momentum was "fully free, nothing to project." Too strong. Total **P = 0** is a genuine momentum-side frame fixing — the boost conjugate to the translation quotient — that I under-counted. Momentum is 4 DOF free *within the COM frame*, not free absolutely. So it's **four** gauge/frame steps, not three. E and L stay free; there is still no energy/angular-momentum constraint surface.

## Whole-system handles — the invariance audit

Four handles, each ↔ exactly one reduction step, each must leave **z fixed**:

| handle | acts on | ↔ reduction step |
|---|---|---|
| translate-all | positions | Jacobi / COM |
| boost-all | uniform velocity | COM frame (P = 0) |
| rotate-all | positions **and** velocities | ρ̃ → +x pin |
| scale-all | positions, momenta, time (similarity) | R̃ = 1 |

Watching z **not move** while the config visibly transforms is the live proof the symmetry bookkeeping is right. The first handle that nudges z localises which gauge-fixing leaks — rotate-all moving z_α means the rotation isn't fully quotiented, etc.

These four compose into the gauge transform **g** between any dragged config and its canonical representative: translate → boost → rotate → scale. That composition *is* the round-trip.

**Decided — COM frame is always on, and shown.** Don't forbid or silently project P ≠ 0. Let the user build a config with net COM momentum, then show it being quotiented away: display **v_COM** as its own vector and draw each body's velocity decomposed into the removed boost + the COM-frame residual (raw arrow → COM-frame arrow shift). The encode always consumes the COM-frame velocities, so total P feeding the chart is 0 by construction. This makes boost-all's z-invariance directly watchable — add COM momentum, watch it stripped, z doesn't budge. Same treatment is the natural template for the other three steps if you want them live rather than only in the canonical window.

## Physical canvas — interaction

- **left-drag body** → move position
- **drag arrow tip** → edit that body's velocity/momentum vector; toggle **v ↔ p** (draw p = m·v scaled)
- **right-click body → properties popover** (in — R-96; render_gui_spec §G8): mass, position (x, y), velocity (vₓ, v_y or speed/angle), momentum, distance to COM, distance to each other body, per-body contribution to P / L / E. All fields editable — drag and numeric entry stay in sync.
- **mass edit** → disc radius ∝ ∛mass (area/volume reads better than linear) (in — R-96)
- **whole-system handles** → empty-canvas drag = translate-all; a rotate ring + scale ring around the COM; a boost handle. Or a dedicated "gauge-test" mode that swaps the per-body handles for the four system handles.
- **overlay toggles** → COM marker, Jacobi frame (ρ, λ vectors — makes the encode legible since the chart is built on them), canonical ghost, shape-sphere mini-widget.

## Canonical window (separate panel)

Yes — separate window, and it earns its place. **Live-canonicalised, always.** The editing canvas holds raw physical state at any gauge — translated, boosted, rotated, arbitrary scale, non-zero COM momentum, all allowed. The canonical window shows the decoded **canonical representative** — COM at origin, ρ̃ → +x, R̃ = 1, COM frame — recomputed every edit. Physical→chart is many-to-one, so this is *not* the config you dragged; it's the class representative. The split *is* the tool: raw on the left, quotiented on the right, updated in real time.

The payoff: under all four whole-system handles the canonical window **does not move**. That stillness, next to a visibly transforming physical canvas, is the invariance proof made visual. COM momentum is the most legible instance — build a system with non-zero total P and the boost arrow sits there on the left while the right window shows it already gone. Don't forbid off-frame construction; watching it *get quotiented away* teaches the frame better than blocking it would.

Two extra modes:

- **ghost/transform** → animate g stepping the dragged config into the canonical one (four named stages), so the gauge move is legible rather than instantaneous.
- **round-trip residual** → take decode(z), push it back through g⁻¹ into the dragged frame, ghost it over the physical config. Any drift by something other than a pure gauge move is round-trip error — and a failed round-trip is always a *located* bug, never an ambiguity.

**Two views, toggle (or split panel):**

- **bodies** → the canonical representative drawn as three physical bodies in the canonical frame, momentum arrows included. Answers *what the class representative looks like*; the direct visual comparison against the raw left canvas.
- **shape-sphere point** → the config as a point on S² — the larger, primary version of the readout widget. Answers *where this shape lives in state space*, and it's the natural bridge to the main view: this is the exact point whose colour you'd read off the fractal (shape-sphere encoding). Config-only, so momentum doesn't show here — that's what the bodies view is for.

## Measurement readouts (gauge-invariant → hold under all four handles)

- pairwise separations r₀₁, r₀₂, r₁₂ (0-based, R-22)
- distance from COM per body
- internal triangle angles / angles subtended at the COM
- hyperradius R, shape angles (α, β) — the actual chart config coords
- momentum magnitudes |pᵢ|, the Jacobi momenta (q0–q3 pre-encode)
- totals: **P** — show the *raw* built P and the removed **v_COM** (encoded P is 0 by construction); **L**, **E** (free, just reported)
- moment of inertia I
- **live z** (8 numbers)
- **shape-sphere marker** (S² widget) live under the drag — "the surface *is* the state space"

Because they're all invariant, the measurement panel doubles as a second invariance check: rotate or scale the whole system and none of these numbers move.

## Degeneracy routing

- **conditioning number** for canonicalisation, surfaced live.
- **ρ → 0**: rotation pin undefined. **R → 0**: scale undefined. These are the *only* two ill-conditioning sites — and they're exactly the two gauge-fixings that aren't translation or boost. The readout is literally instrumenting those two steps.
- past threshold → flip chart-encode ▸ **direct-physical-inject**, and flag it. Free degeneracy-finder, and it exercises both ingestion paths.
- **cross-check**: for non-degenerate configs, run both paths and diff. Agreement validates direct-inject against chart-encode; divergence is a bug in one of them.

## Bridges to the main view

- inspector emits z → drop a crosshair at z's projection on the current slice: "specific physical config in mind → where it lives in the fractal."
- reverse → click a point on a slice, load that IC into the inspector, then perturb and watch.

## Build notes

- static, single-sample. **Reframe under the Rust substrate:** the production decode/encode is now Rust, so this HTML tool is an *independent JS re-port* of the decode maths (`principia_dd_decoder.md` §3) — a **second implementation**. Its value is therefore *cross-implementation* validation (it catches Rust-decode logic bugs the way an independent reference does), not exercising the one codebase. When the Inspector becomes an **egui panel** in the F3 debug menu (Rust, calling the engine's actual decode), *that* is the verbatim-reuse test; this standalone stays valuable as the independent oracle. Both worth keeping.
- hand-rolled HTML/canvas for both the numeric fields and the drag layer (not Tweakpane). Standalone artefact for the dev phase; the eventual home is an **egui** panel in the F3 debug menu (Rust) — now the dev GUI's Inspector window, which absorbs this tool (R-65; `principia_render_gui_spec.md` §G8), with the production TS GUI re-skinning the same operations later.
- everything here is CPU-side f64 single-sample — no shader work.

## Open questions

- name (still "IC Inspector" working title)
- ~~shape sphere convention: mass-weighted vs geometric~~ → resolved: **mass-weighted** (the correctness-invariant call in the status block)
- ~~default arrow: v or p~~ → resolved: editing velocity-only, canonical momentum-only
- ~~whole-system handle UI: rings vs modal~~ → resolved: buttons
