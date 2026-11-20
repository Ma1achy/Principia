# Principia print — a compositional post-process prototype

Every medium is a **graph of small nodes, written as JSON**. There are no hand-written media left:
the 47 media built so far are all presets in `presets/`, and each one reproduces its original
pixel for pixel (`verify.py`).

```
render('oil', src)                                    # a preset on an image
render('acrylic', src, depth=30)                      # override an exposed setting
render('relief_of_lifetime', src, data={'lifetime': L})   # bind measured data
```

## A preset

```json
{
 "name": "Dot-matrix printer",
 "settings": {"seed": 0, "bands": 1.0},
 "nodes": {
  "T":    {"op": "expr", "in": {"I": "src"}, "p": {"f": "(1-stretch(blur(lum(I), 0.8)))**0.8"}},
  "r":    {"op": "rng", "p": {"seed": "$seed"}},
  "dots": {"op": "impact_dots", "in": {"tone": "T", "rng": "r"}, "p": {"pitch": 3.2, "bands": "$bands"}},
  "n2":   {"op": "noise", "p": {"scale": 1.0, "octaves": 1, "seed": "$seed+2"}},
  "col":  {"op": "expr", "in": {"d": "dots.out", "n2": "n2"}, "p": {"f": "..."}},
  "out":  {"op": "u8", "in": {"x": "col"}}
 },
 "out": "out"
}
```

**Inputs** (`"in"`) name where data comes from:

| reference | meaning |
|---|---|
| `src` | the image coming in: the render, or an upstream medium's output |
| `data.lifetime` | a measured field supplied with the render |
| `data.lifetime\|proxy` | the field if it was supplied, otherwise the node `proxy` |
| `node` / `node.port` | another node's first output / a named output |
| `$setting` | one of the preset's settings |

**Parameters** (`"p"`) are literals, `$setting` (or `$seed+3`), or `@reference` to make the
parameter a per-pixel field.

**Settings** are a preset's public knobs. Whoever uses the preset can override them, from code or
from another preset.

**Presets are nodes.** `"op": "preset:polaroid"` runs another preset as a sub-graph, so media
chain: `light_painting_polaroid` is two nodes, light painting feeding Polaroid.

**Random streams are values.** Where a medium draws random numbers in sequence (strokes, dots,
stitches), a stream (`rng`) is passed from node to node, so the order of the draws is fixed and
every image is reproducible from its settings.

**`expr`** is the glue: arithmetic over its inputs, like a shader graph's maths node. The named
parts — flow, smearing, strokes, noise, lighting, separation, regions, streamlines, marks — are
nodes, so they can be shared and, later, ported to the GPU one by one.

## Adding things

- **A new medium:** write a JSON preset from existing nodes. No code.
- **A new part:** add a function to `nodes.py` with `@node('name', inputs=(...), outputs=(...), **defaults)`
  and a docstring; it appears in the catalogue.
- **A variant:** a preset of one node that runs another with different settings
  (`dot_matrix_clean` is `dot_matrix` with `bands: 0`).

## Files

- `graph.py`: the engine (about a hundred lines)
- `nodes.py`: the node library
- `presets/`: every medium, as JSON (plus a few `demo_` compositions)
- `verify.py`: checks each preset against its original, pixel for pixel
- `NODES.md`: the node catalogue
