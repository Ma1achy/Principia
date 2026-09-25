# Embedding a slice's provenance in the pixels of its own PNG

**One channel: the low bit of every RGB pixel, plus alpha as a free second copy.** No block
modulation, no DCT, no visible watermark. The design target is *someone shares a PNG, someone
else drops it in and gets the exact slice back* — and everything beyond that was measured and
rejected (§7).

---

## 1. Why this exists — provenance first, sharing second

**The sharing feature is the smaller half.** Every artefact in this project has needed a
provenance line, and they kept getting separated from the images: a `refine_flagged` value that
nothing printed; an auto-ranged ramp window that lived in a text file; a render whose config was
only recoverable by finding the commit that produced it. **An image that carries its own config
cannot be separated from it.**

So this is the standing provenance discipline made structural, and image sharing falls out.

---

## 2. Layout

```
RGB low bit    payload, repeated as whole records in a 2-D TILE GRID
Alpha low bit  the same payload again, more records
```

**Alpha is systematic, not parity.** On an opaque image `255 → 254` is invisible and free; if a
pipeline flattens it, the RGB copy is untouched and nothing is lost but redundancy. Losing the
second plane costs **copies, not content**.

**Every record carries its own CRC32 and its own header copy.** A record is trusted whole or
discarded whole. The first prototype had one header at the front of the stream and 1% corruption
orphaned everything behind it even though the records survived — **a single point of failure in
the recovery path is still a single point of failure.**

```
RECORD = header(16B) ‖ payload ‖ crc32(payload)
header = magic(4) ‖ version(1) ‖ flags(1) ‖ payload_len(4) ‖ n_records(2) ‖ crc32(header)(4)
```

**Read = collect every intact record, majority-vote per byte.** With 9+ records at the smallest
resolution, repetition *is* the error correction and Reed–Solomon buys less than a dependency
costs.

---

## 3. Tiles, not raster order

**Each record lives in its own square tile.** Raster layout means a crop takes a column-slice of
*every* record and destroys all of them; tiling means a crop that keeps any tile keeps a whole
record.

Tile side is derived, not chosen: `side = ceil(sqrt(ceil(record_bits / 3)))`.

---

## 4. The three searches on read

**Each fixes a different transform, and two of them were things I initially wrote off.**

| search | fixes | why it works |
|---|---|---|
| **offset** `(dx, dy)` over one tile | **crop, arbitrary translation** | cropping shifts the origin so a fixed grid misaligns and every tile reads garbage |
| **dihedral**, 8 transforms | **rot 90/180/270, flip, transpose** | rotation loses *nothing*; the bits are present, just moved |
| **decimation** by `s` | **nearest-neighbour upscale ×2, ×3** | upscale *duplicates* pixels; take every `s`-th pixel of every `s`-th row |

> **The one idea that transfers from QR codes is not the error correction. It is the FINDER
> PATTERN** — a way to *locate* the grid when the frame has moved. That single change took
> cropping from `lost` to `OK` on every test, including an arbitrary 37-pixel offset.

**The CRC is the oracle.** A wrong offset, transform or decimation simply fails to decode, so the
reader brute-forces candidates and the first that verifies is correct. A blind 0–89° angle search
over the *redundant* variant found 34° in 35 tries in **0.1 s**.

**And the reader reports what it found** — `{transform: rot270, decimate: 2, tiles: 9/9}` — so a
recovered image says *how* it was recovered rather than silently succeeding.

---

## 5. Two variants, one choice

**Whole-record repetition and per-bit redundancy fix orthogonal failures and cannot substitute
for each other.**

| | localised damage (crop, overwrite) | uniform noise |
|---|---|---|
| **tiled** (record repetition) | ✓ any surviving tile is the whole message | ✗ a 382 B payload spans 3056 bits, so 1% flips hit *every* record |
| **redundant** (each bit written `k`×) | ✗ | ✓ `k=9` → 5%, `k=25` → 15% |
| **hybrid** | ✓ | ✓ | at reduced capacity |

**Default: tiled.** Lossless PNG does not introduce uniform noise, so per-bit redundancy pays for
a threat that does not occur, and the capacity is better spent on more tiles or on the shader
source. **Hybrid is available** for arbitrary rotation, which needs it — nearest rotation is
recoverable but leaves ~8% bit error, which whole-record repetition cannot absorb and `k=25` can.

---

## 6. What travels

```
build hash          REFUSE to recreate silently across a version where the decoder changed.
                    A slice config is only meaningful relative to a decoder, and this project
                    has changed its decoder more than once.
slice               z0[10], dimH, dimV, mag, zoom, pan, tilt, gamma
sim                 horizon, dtMacro, maxSteps, rColl, rEsc, eta, nSync,
                    integrator, REGULARISATION
ensemble            E, N, jitter_frac
colour              mode + every parameter: kernel, temperature, blend space, site set,
                    brightness field AND ITS POLARITY, and THE RAMP WINDOW
tier                the quality settings actually used
```

**The ramp window is not optional.** An auto-ranged ramp manufactures or hides the difference it
is meant to show — measured, not hypothesised — so a recreated image without it is a different
image.

**Colour modes encode as an ENUM ID, not as source.** The colour spec is a small algebra, so most
"custom" shaders are a *parameterisation* of the same primitives and encode as values. Only a
genuinely ejected WGSL node ships its source; otherwise every default render carries 3 KB it
already has, and an old embedded copy would override a fixed one.

**A fallback must be visible.** A recreated slice rendering with a *different* colouring than the
image it came from is exactly the class of silent wrongness this project exists to eliminate.

---

## 7. Measured capacity and behaviour

**Config-only payload 596 B JSON → 382 B deflated. Full `frag.glsl` 11,367 B → 2,969 B.**

| resolution | config only | config + full shader |
|---|---|---|
| 64² | 1 tile | too small |
| 128² | 9 tiles | 1 tile |
| 256² | 49 tiles | 4 tiles |
| 512² | 225 tiles | 25 tiles |
| **1024²** | **961 tiles** | **100 tiles** |

**From 128² up, an image carries the code that coloured it.**

**Survives** — max pixel delta **1** throughout, invisible:

- PNG save/load, `optimize=True`, `compress_level=9` — 225/225 tiles
- **alpha stripped entirely** — 225/225, RGB is systematic
- crop to 50%, crop at an arbitrary 37 px offset, crop + PNG round-trip
- rotate 90/180/270, flip h/v, transpose — transform reported
- nearest upscale ×2, ×3, and rot90 + upscale together
- top 50% of the image overwritten (top 90% under raster layout)
- **arbitrary rotation** (7°, 34°, 45°, 7.3°, 34.7°) under the hybrid variant

**Does not survive, and these are genuine:**

- **LANCZOS/bilinear rescaling** — resampling *averages*, so the low bit is destroyed rather
  than moved. Bilinear fails at **half a degree** of rotation.
- **JPEG at any quality** — there is no LSB plane to read.
- uniform LSB noise > 0.1% under the tiled variant.

> **Permutation is recoverable. Averaging is not.** Crop, translation, rotation, flip, integer
> nearest scale are all permutations. Resampling and JPEG are averaging.

---

## 8. Rejected, with the measurement

**Block-mean modulation at low spatial frequency.** It *works* — JPEG q60 and LANCZOS to quarter
scale both survive, which LSB cannot touch at any redundancy. It was still rejected:

- **~26 bytes** at block 16 on 1024². A pointer, not a payload — which means a lookup, which
  means hosting, IDs and availability, to recover a config from an image someone JPEG'd rather
  than just sending the PNG.
- **Not usable as parity for LSB either**: when LSB fails it fails *totally* (the bit plane is
  gone), and 26 bytes cannot reconstruct 382 from nothing. When it fails *partially*, tile
  repetition already handles it.
- Visible at delta 4/255 against LSB's 1.
- Fails on high-variance images by a **predictive rule**: it needs `amp > pixel_σ / block_size`.
  Salt-and-pepper (σ=128) fails at every block size; a 1-pixel checkerboard *passes*, because
  every block mean is identical. **Regularity matters, not frequency.**

**Knowing the ceiling was worth the experiment even though the answer is no.**

---

## 9. Obligations

- **Off by default for figure export**, on by default for share. A scientific figure must be
  exactly the pixels the renderer produced — otherwise someone eventually measures an embedded
  image and finds structure that is not physics.
- **`tEXt` chunk alongside**, carrying the same payload. Strippable, but free and trivially
  readable by anything.
- **Three distinguishable outcomes**, never two: *no embedded state* / *state present, corrupt* /
  *recovered, and here is how*. Silent wrongness is the failure mode this project exists to
  eliminate.
- **Golden-image test**: embed → PNG round-trip → extract → assert bitwise-equal config, at every
  supported resolution.
