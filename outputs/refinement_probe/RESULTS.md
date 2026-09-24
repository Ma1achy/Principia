# Refinement criterion probe — Burrau/Pythagorean slice

Numerical probe of whether a **local uncertainty exponent** can serve as the
refinement criterion (split where refining helps, stop where it does not).

## Method
Vectorised planar 3-body KDK leapfrog (f64), softening eps=0.03, dt=2e-4,
median energy drift ~2-4e-8. Pythagorean IC m=(3,4,5); 2D slice = body 0's
initial position over a box. 96x96 grid, integrated to t=40.
Outcome = coarse Chazy-like class (which body escapes, or still-bound).
Uncertain fraction f(eps) measured by comparing outcomes eps apart;
alpha = d log f / d log eps.

## Estimator validation (synthetic basins)
| basin | alpha |
|---|---|
| straight edge | 1.06 |
| smooth curve | 1.02 |
| four smooth basins | 1.06 |
| random (maximally riddled) | -0.01 |
| fine periodic | 0.18 |

The estimator returns ~1 for smooth boundaries and ~0 for riddled ones, as intended.

## Result 1 — alpha is ~0 everywhere, over two decades of scale

| box half-width | h | alpha (outcome) | alpha (word) |
|---|---|---|---|
| 0.25 | 0.0053 | **0.048** | 0.026 |
| 3.0  | 0.0632 | **0.053** | 0.043 |

Uncertain fraction stays at **0.45-0.55 at every scale from 0.005 to 1.0**.
Local alpha per block (12/16/24-cell blocks): median 0.02-0.06, p95 <= 0.42;
local f(h) spans only 0.27-0.67 -- i.e. **every block is roughly half uncertain**.

No region looks smooth. The criterion carries **no spatial information**:
it says "refine everywhere, forever".

## Result 2 — S_word is saturated, not merely early

| | half=0.25 | half=3.0 |
|---|---|---|
| outcome differs | 0.447 | 0.473 |
| word differs | **0.926** | **0.882** |
| word fires, outcome agrees | 0.495 | 0.429 |
| precision as split trigger | **0.466** | **0.514** |
| recall | 0.966 | 0.957 |

The contract's justification ("words branch before fates differ") is literally
true but not useful: the word differs for ~90% of neighbouring pairs at the
finest scale and ~99% at coarse scale. High recall, ~50% precision, and
scale-invariant -- so it cannot indicate *where* to refine.
~10% of words hit the 24-symbol cap (truncation is common, not rare).

## Caveats
- 57-65% of systems are still unresolved at t=40, so this partly measures an
  escape-time boundary rather than final basins. Longer integration needed.
- Softening eps=0.03 smooths the closest approaches.
- No *physical* positive control (a region known to be smooth); the positive
  control is synthetic.

## Implication
If the physics signal is uniform across the region, it cannot steer *where* to
refine. Two consequences:
1. Any criterion built on disagreement magnitude refines uniformly and never
   terminates -- so the **integration floor is load-bearing, not a patch**, and
   cannot be derived from the same signal it is meant to arrest.
2. In a Wada region, "where to refine" is a **rendering** question (viewport,
   zoom, focus, display resolution). The physics signal's job is to say
   **when to stop**, not where to go.
