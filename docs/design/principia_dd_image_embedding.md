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

**The record, field by field (R-72, REQ-TOOL-110).** One format description generates both the writer and the reader
(`crates/render/src/embed/record.rs`), so the two cannot disagree on a field.

| bytes | field | definition |
|---|---|---|
| 0–3 | `magic` | four fixed bytes, proposed below (R-71) |
| 4 | `version` | the record layout's version; R-81's contract-name layout bumps it above the prototype's (§6), to 3, proposed below (R-71, R-380) |
| 5 | `flags` | the flag bits, below |
| 6–9 | `payload_len` | the payload's length in bytes, u32 |
| 10–11 | `n_records` | how many records the writer placed in the RGB plane, u16, below |
| 12–15 | `crc32(header)` | the CRC of bytes 0–11 |
| 16 to 16 + `payload_len` − 1 | payload | below |
| the next 4 | `crc32(payload)` | the CRC of the payload bytes alone |

- **Byte order: big-endian.** Every multi-byte integer (`payload_len`, `n_records`, both CRCs) is written most
  significant byte first, the byte order PNG itself uses for its lengths and CRCs.
- **The CRC is PNG's:** CRC-32 with the reflected polynomial `0xEDB88320`, initial value `0xFFFFFFFF` and final XOR
  `0xFFFFFFFF` (the CRC of the ASCII bytes `123456789` is `0xCBF43926`).
- **Flag bits** (bit 0 is the least significant):
  - bits 0–1, the variant the writer used (§5): `0` tiled, `1` redundant, `2` hybrid; `3` is reserved;
  - bit 2, `source`: the payload carries the source of at least one ejected WGSL node (§6), so a reader knows before
    it inflates the payload that the image carries code that overrides a built-in colouring;
  - bits 3–7 are reserved and written `0`.

  A record whose variant is `3` or whose reserved bits are not all `0` is discarded whole: a later layout that needs a
  new flag bumps `version` instead.
- **`n_records`** counts the records the writer placed in the RGB plane: its grid's tiles at the RGB side (below), at
  most 65,535 (the field's largest value). The reader reports the records it recovered from the RGB plane against it,
  as §4's `tiles: 9/9` and §7's 225/225 count them: a pristine 128² image reports 9/9, and a 512² image with its alpha
  stripped reports 225/225.
- **The alpha plane's records are an extra redundancy layer, not counted in `n_records`.** §4 and §7 report no alpha
  count, so this defines it (R-72): the alpha plane holds its grid's tiles at the alpha side (below), 4 at 128² for a
  402 B record, a count the reader derives from the same tile rule. Each intact alpha record joins the vote as an RGB
  record does, and the reader reports how many it recovered separately from `tiles`, against that count. Losing the
  alpha plane loses only those copies (above: copies, not content).
- **A record is trusted or discarded whole.** The reader discards a record, and keeps nothing from it, when it is
  shorter than its header, when its `magic` differs, when `crc32(header)` fails, when its flags are not a defined
  combination, when it is shorter than its `payload_len` says, or when `crc32(payload)` fails. Bytes after the
  payload's CRC are not part of the record. A record's `version` is read, not checked: an intact record of another
  layout is reported as such, never as corrupt.
- **Payload serialisation.** The payload is one JSON object (§6's fields) in the canonical serialisation, JCS
  (RFC 8785; `principia_gui_state_contract.md` §2, R-309, R-318), UTF-8, compressed with raw DEFLATE (RFC 1951, no
  zlib or gzip wrapper: the record's own CRC covers it). `payload_len` is the compressed length. JSON compressed with
  DEFLATE is the encoding §7 measured (596 B of JSON to 382 B), so the payload is §7's size class: a config-only
  record is 16 + 382 + 4 = 402 B.
- **Bit order.** The record is read as a bit stream, each byte most significant bit first: bit `i` of the record is
  bit `7 − (i mod 8)` of byte `⌊i / 8⌋`.
- **Within a tile**, the bit stream fills the tile's pixels row by row from the tile's top-left pixel (the top row
  first, the top row being the first row PNG stores, each row left to right), and within a pixel its channels in
  order. In the RGB plane a pixel holds three bits, in R, G, B order, so bit `i` goes to the low bit of channel
  `i mod 3` of the tile's pixel `p = ⌊i / 3⌋`, at column `p mod side`, row `⌊p / side⌋`. In the alpha plane a pixel
  holds one bit, so bit `i` goes to the low bit of alpha at column `i mod side`, row `⌊i / side⌋`. A tile's slots after
  the record's last bit are not written: those pixels keep their own low bits.
- **Tile side and the grid.** A plane holding `b` bits per pixel has tiles of side `ceil(sqrt(ceil(record_bits / b)))`,
  `record_bits = 8 × (16 + payload_len + 4)`: §3's side is the RGB plane's, `b = 3`, and the alpha plane's is `b = 1`.
  Each plane's tiles sit in a grid from the image's top-left pixel, tile `(c, r)` at pixel `(c × side, r × side)`, with
  `⌊width / side⌋` columns and `⌊height / side⌋` rows; the strip beyond the last whole tile is not written. A
  402 B record gives an RGB side of 33, and the grid then holds §7's 1, 9, 49, 225 and 961 tiles at 64², 128², 256²,
  512² and 1024²; a record carrying both of §7's payloads, 382 + 2,969 B, gives a side of 95 and §7's second column.
- **Magic and version: proposed (R-71, REQ-TOOL-109), to be confirmed by the human at the M7 gate.** The proposed magic
  is `8F 50 72 6E` (`0x8F` then ASCII `Prn`). It is none of the five 4-byte windows of the PNG signature
  `89 50 4E 47 0D 0A 1A 0A`, and its first byte is above `0x7F`, so it is no PNG chunk type (those are four ASCII
  letters) and no ASCII text. It has 16 of its 32 bits set, so a blank or saturated low-bit plane never reads as it, and
  it differs from its own bit reversal, so a record read back to front does not start with it.

  **The prototype's values, which a new record must not reuse (R-380):** the prototype's magic is `PRPX`
  (`50 52 50 58`), and its highest version byte is 2. The proposed magic `8F 50 72 6E` differs from `50 52 50 58` in
  every byte. The proposed version byte is **3**, the prototype's highest plus one: R-81's bump for the contract-name
  layout (§6). Both are R-71 proposals (REQ-TOOL-109), used provisionally until the human confirms them at the M7 gate;
  the prototype's version, 2, is the orchestrator's reading of the human's ruling, flagged in R-380 for that gate.

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
slice               chart id + params, z₀ (the 8-D latent), q₁, q₂ (the basis; zoom is their
                    common scale, R-83), slice values, lock flag, z_locked, δ
sim                 T_horizon, dt_macro, N_max, r_sub, gamma_sub, r_coll, r_close,
                    eps_E, eps_L, tau and the escape window,
                    the integrator occupant (stepper × REGULARISATION),
                    the compute shaders' fast-math setting (R-297)
ensemble            E, N
colour              mode + every parameter: kernel, temperature, blend space, site set,
                    brightness field AND ITS POLARITY, and THE RAMP WINDOW
tier                the quality settings actually used
```

**Contract names, not the prototype's (R-81).** Every field is named as the contracts name it
(`principia_gui_state_contract.md` §2, integrator_contract Part 3), over the 8-D latent. There is
no jitter field: the footprint fixes the copies' offsets (R-80). This record layout bumps the
embedding `version`; the byte value is set with the rest of the header (R-71).

**The record's `SimConfig` and `RenderState` are in their canonical serialisation (R-309)**, the one snapshot JSON,
share links and the profiler header's `config` carry (`principia_gui_state_contract.md` §2).

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
  readable by anything. Defined by R-72, REQ-TOOL-110:
  - **Its keyword is `Principia`.**
  - **Its text is the record's `version` in decimal digits, one line feed (`0x0A`), then the payload's JSON** before
    it is compressed (§2). A `tEXt` text is Latin-1 with no control character but the line feed, so every character of
    the JSON outside `0x20`–`0x7E` is written as its JSON escape `\uXXXX` (a surrogate pair above U+FFFF), its four hex
    digits lowercase, as JCS writes its own `\u` escapes (RFC 8785 §3.2.2.2), so two writers produce the same chunk
    bytes. That is the same JSON value: a reader parses it and serialises it canonically (§2), and gets back the
    canonical JSON the writer compressed into the payload, byte for byte.
  - The chunk carries no magic, flags or CRC of its own: PNG gives every chunk a length and a CRC.
- **Three distinguishable outcomes**, never two: *no embedded state* / *state present, corrupt* /
  *recovered, and here is how*. Silent wrongness is the failure mode this project exists to
  eliminate.
- **Golden-image test**: embed → PNG round-trip → extract → assert bitwise-equal config, at every
  supported resolution.
