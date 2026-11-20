# State map follow-up: colour maps that tile the infinite plane

A follow-up from me, Malachy, 21 Oct 2026, to the state map bundle (R-542, PR #270). This is a ruling of mine; quote
§0 verbatim wherever it is recorded.

## 0. My words

> "i geuss what i'm reaching for are colour maps that tile cleanly over the whole infinire plane because the tiling is
> weird right now ?"

To the proposal of a Torus gradient base and a Repeat wrap mode for the Picture base:

> "yeah"

And on the prototype built from it (`demo/state_flow.html` here, version 9 of the State Flow page):

> "ok add this as follow up instructiosn bc i already sent of f that bundle"

## 1. What was wrong

The Picture base is finite: it is laid on the target chart's home square, and SM8 extends it past the square by
**mirror-repeating and dimming** it. On a picture made of cells this does two bad things:

- **The pattern breaks at every seam.** A mirror repeats the edge row, so the same cell appears on both sides of the
  square's edge (one bright, one dim) and a checkerboard stops being a checkerboard.
- **The colours fold back.** Past the edge the picture runs backwards, so a system far outside gets the colour of one
  just inside.

Mirroring stays right for photos. For a cell picture it is wrong, and no base so far tiles the plane with no seam and
no special home square.

## 2. What to add

### 2.1 Torus gradient (a Gradient option, flat charts and the cylinder)

A colour map periodic on the plane. Each period maps onto a torus inside OKLab:

```
X, Y  = position in periods, in CHART UNITS (never the squashed square), n periods across the home square;
        on the plane the y period count is round(n · H / W), as the Palette's tiling already does
θ = 2π X,  φ = 2π Y
(a, b) = (R + r cos φ) (cos θ, sin θ)
L      = L₀ + r_L sin φ
R = 0.074,  r = 0.037,  L₀ = 0.70,  r_L = 0.12
```

- On the cylinder, X is θ itself once round (wrap(θ)/2π), and only s repeats.
- A torus sits in 3-D colour without touching itself (R > r), so within one period the map is one-to-one and
  continuous; across periods it repeats exactly.
- Fixed shape, never clipped per pixel (SM6's rule). The constants are 0 out of gamut on a 240² sweep of (θ, φ), at
  0.975 of the largest scale that stays in gamut (`checks/torus_gam.js`). They are calibration values under R-71,
  confirmed at the M7 gate with the rest of SM6's.
- It spends L on position, so a brightness node after it gets the existing "no longer one-to-one" warning.
- The honest cost: a system exactly one period away gets the same colour. The legend states the period in chart
  units; "Left its starting tile" brightness already separates such systems.
- Setting: **Repeats** (1, 2, 3, 4, 6, 8 across the square; default 2). Shown only when Torus is chosen.
- SM6's table already lists circle × circle → torus; this realises it for the plane by making the plane periodic.

### 2.2 Picture wrap mode

The Picture base gets **Past its square**: Repeat · Mirror · Clamp, and a separate tickbox **Dim past the square**.

- Repeat: `s − floor(s)`. Mirror: the existing triangle wave. Clamp: the edge colour.
- **Defaults are Mirror with dimming on**, so every saved stain renders bitwise as before.
- The hovered-pixel readout and the graph strip name the wrap mode ("Picture · repeat").

### 2.3 A tileable built-in picture

**Checkerboard, tileable**: 8 × 8 cells, cell (i, j) coloured by the torus at its centre
(θ = 2π(i + ½)/8, φ = 2π(j + ½)/8), chroma × 0.8, L × 0.8 about L₀, then ±0.07 L by parity. Its colours wrap, so under
Repeat it has no seam. 0 of 64 cells at a channel limit. The existing Checkerboard stays as it is: under Repeat its
parity holds but its colour ramp jumps at the seams, which is the picture's property, not the wrap mode's.

### 2.4 The Flow preset

The Flow preset becomes **Checkerboard, tileable** with **Repeat** and dimming **off**. That is the one deliberate visual
change in this follow-up.

## 3. Spec changes (`docs/design/principia_state_map.md`)

- **SM6:** add the Torus gradient to the plane and cylinder rows of the map table, with the constants and the
  periodicity cost above. Mark the circle × circle row as realised by it.
- **SM8:** replace "mirror-repeated and dimmed" with the wrap mode and the dim tickbox, defaults Mirror and on; say why
  Mirror is wrong for cell pictures (§1).
- **SM11:** add Torus to the Gradient row of the bases table (plane and cylinder); add the wrap setting to Picture;
  add "Checkerboard, tileable" to the built-in pictures.
- **SM11 lightness paragraph:** list the Torus gradient among the bases that spend L on position.
- **SM12:** add the tests in §5.
- **SM15:** the Flow preset as in §2.4. If my ruling on RQ-499 has renamed the presets by then, use the new name.
- Add a §0 entry quoting §0 here.

## 4. Requirements (via `plan/tools/reqio.py`, area COL; next free numbers)

1. "Chart position's Gradient base must offer a Torus gradient on flat charts and the cylinder: periodic in chart
   units with a set number of periods across the home square, one-to-one within a period, seamless across periods,
   and in gamut with no per-pixel clipping."
2. "The Picture base must offer a wrap mode past its square (Repeat, Mirror or Clamp) and a separate dim setting; the
   defaults must be Mirror and dim on, so existing stains render unchanged."
3. "The built-in pictures must include a tileable checkerboard whose colours wrap, so it tiles the plane with no seam
   under Repeat."

Each cites the sections in §3, `rulings: [R-<this ruling>]`, milestone as the state map's own.

## 5. Tests (each with the arm that fires)

| test | asserts | fires on |
|---|---|---|
| Torus seam | colour at (u, v) equals colour at (u + P, v) and (u, v + P_y) to 1e-12 over a sweep reaching well past the square (prototype: 2.7e-17) | a period computed in squashed or compacted units; an off-by-one in the period count |
| Torus one-to-one | within one period, distinct grid points have distinct colours (minimum OKLab separation > 0 on a 200² grid) | R ≤ r, or L and chroma swapped so the tube self-intersects |
| Torus gamut | 0 samples out of sRGB on a 240² (θ, φ) sweep, with no clipping in the shader path | constants raised past the gamut, or a per-pixel clamp added |
| Wrap defaults bitwise | a stain saved before this change renders bitwise identically (Gradient 2D, Picture, Palette hex; the prototype checks all three) | the default wrap or dim flipped |
| Wrap arm | the same Picture under Repeat and under Mirror differs past the square and is identical inside it | a wrap setting that is not wired through |
| Tileable checker seam | under Repeat, the cell across each seam has opposite parity and the colour difference across the seam equals that between neighbouring cells inside | the old checkerboard silently substituted |

## 6. How to apply

- Record this as a new ruling in `decisions.md` from the next free R-number, quoting §0, naming R-542 as the ruling it
  amends.
- Spec, requirements and tests as above; then `check_plan`.
- Port from `demo/state_flow.html`: `SM_TORUS`, `smTorusXY`, `smTorusLab`, `smWrapFn`, and the `tchecker` branch of
  `smImage`. The constants are the prototype's; the algebra is short enough to transcribe, not re-derive.
- Where this conflicts with a ruling or a queue entry these texts do not address, queue it rather than resolving it.
- One PR. Paste the test output in it.
- The security rule stands: never delete or modify anything outside the repo and build/scratch directories without
  asking.
