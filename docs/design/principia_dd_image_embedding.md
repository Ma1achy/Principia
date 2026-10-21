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
  - bit 3, `colour` (from version 4, R-484): the payload carries tier 2, `RenderState` (§6, "Tiers"); any record
    carries tier 1, and bit 2 marks tier 3. `source` set with `colour` clear is not a defined combination;
  - bits 4–7 are reserved and written `0`.

  *Was (version 3):* "bits 3–7 are reserved and written `0`." Version 4 defines bit 3 (R-484, J5).

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
- **The hybrid layout (flag variant `2`; R-72, REQ-TOOL-110).** The hybrid variant (§5) writes each bit of the record
  `k` times, side by side, in a *block* of `m` × `m` pixels, `k = m²`; `m` is the block side (`b` above is a plane's
  bits per pixel). Two redundancies are offered, §5's measured `k = 9` (`m = 3`) and `k = 25` (`m = 5`); `m` is odd, so
  the reader's majority over a block's `k` pixels never ties.
  - **The block grid.** An image's blocks sit in a grid from its top-left pixel, block `(i, j)` covering pixels
    `(i × m + dx, j × m + dy)` for `dx, dy` from 0 to `m − 1`, with `⌊width / m⌋` columns and `⌊height / m⌋` rows. The
    pixels beyond the last whole block, on the right and at the bottom, are not written.
  - **Tiles of blocks.** Each plane's tile side is the one above (`side`, at that plane's `b`), counted in blocks: a
    record's tile is `side` × `side` blocks, tile `(c, r)` at block `(c × side, r × side)`, that is at pixel
    `(c × side × m, r × side × m)`, and the tile grid has `⌊⌊width / m⌋ / side⌋` columns and `⌊⌊height / m⌋ / side⌋`
    rows. Within a tile the record's bits fill the blocks as "Within a tile" above fills a tiled record's pixels, with
    "block" for "pixel": in the RGB plane bit `i` goes to channel `i mod 3` of the tile's block `p = ⌊i / 3⌋`, at
    column `p mod side`, row `⌊p / side⌋`; in the alpha plane bit `i` goes to alpha of the tile's block at column
    `i mod side`, row `⌊i / side⌋`. Every one of the block's `k` pixels holds the bit in that channel's low bit. A
    tile's slots after the record's last bit, and the blocks beyond the last whole tile, are not written: their pixels
    keep their own low bits. The low bits the hybrid writer writes are thus the ones the tiled writer writes in a
    `⌊width / m⌋` × `⌊height / m⌋` image, upscaled `m`-fold by nearest neighbour; the smallest image that holds one RGB
    tile is `side × m` pixels a side (165 px for a 402 B record at `k = 25`).
  - **The two planes.** The RGB plane's tiles are written row by row from the top-left, as many as `n_records` counts,
    which is the number of tiles in the RGB plane's grid of blocks, at most 65,535. On an image with alpha, every tile
    of the alpha plane's grid of blocks is written; they are not counted in `n_records`, as for the tiled variant
    (above).
  - **`k` is not stored.** The record is the one defined above, field for field; the only marker of the hybrid layout
    is variant `2` in its flags. Neither the record nor the image stores `k` or `m`: the reader finds them by search,
    trying each offered `k` (§5), and the CRC tells it which is right (§4).
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

**Version 4 (R-483, R-484; proposed, R-71, REQ-TOOL-109, confirmed with the magic at the M7 gate).** The layout's
version is bumped to **4**: the payload's `SimConfig` carries `Links` filled (§6), which a version-3 reader's
`deny_unknown_fields` cannot read, and the flags define bit 3, `colour`. A version-3 record is read as before, an
intact record of another layout reported as such, and its `"links":{}` read as "links unrecorded" (§6, "Old records").

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

**The hybrid's layout is §2's "The hybrid layout"** (R-72, REQ-TOOL-110): §2's record and tiles laid in `m` × `m`
blocks of pixels, each bit written `k = m²` times side by side, `k = 9` or `k = 25`. Side by side is what a rotation
keeps together: nearest rotation moves a block's pixels as a group, so its bit errors fall at the blocks' edges, and a
vote over each block absorbs them. Unrotated, a hybrid image is an upscaled tiled one, which §4's decimation search
reads. Rotated, it is read by the angle search, run when §4's three searches recover nothing: the reader undoes each
whole degree from 0 to 89 (§4's blind 0–89° search; the dihedral search covers the multiples of 90°), and for each
offered `k` finds the blocks' phase in the rotated-back image (the column and the row, from 0 to `m − 1`, at which its
low bits change most often from the pixel before, as they do at a block's edge), takes each block's majority of each
low bit, and searches the voted blocks for intact records. The CRC is the oracle (§4): the first angle and `k` whose
blocks hold an intact record are the ones read, and the reader reports both. The cost is capacity: a bit takes `k`
slots, so an image holds about `k` times fewer records (§2: 165 px a side at least for a config-only record at
`k = 25`, against the tiled variant's 33).
The angle reported is in whole degrees counter-clockwise, 0 to 89, and is undone before the dihedral transform
reported beside it: the image read is the original under that transform, then rotated counter-clockwise by the angle,
so 97° counter-clockwise reports a 90° rotation and 7°, and 34° clockwise a 270° rotation and 56°.
At each angle `k = 25` is tried before `k = 9`, so the angle reported is the first whole degree at which either `k`'s
blocks hold an intact record, and the `k` reported is the one that verified there.

---

## 6. What travels

```
build hash          REFUSE to recreate silently across a version where the decoder changed.
                    A slice config is only meaningful relative to a decoder, and this project
                    has changed its decoder more than once.
slice               chart id + params, z₀ (the 8-D latent), q₁, q₂ (the basis; zoom is their
                    common scale, R-83), slice values, lock flag, z_locked, δ,
                    links (per-block id and parameters, R-483)
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

**What R-483 adds (X3).** Beside the `slice` line's links, the record carries, in `SimConfig`: the slice frame's tilt
map (`tilt_map`, sheet or lean, R-474) and its tilt angles and targets, so the GUI's tilt controls restore and not only
`q₁` and `q₂` (R-459, R-473); each energy axis's mapping, warped or linear, as chart params (R-471); and a **saved
chart's full definition**, its parent chart, centre, rotation, targets and map (R-475). A saved chart travels **by
value, never by name**: a share or an embed does not depend on the receiver having the same entry in their own
saved-chart library.

**The decoder's identity is the build plus the config (R-483, X4).** The `build hash` line's rule read the build hash
alone. With swappable links, the decoder a record used is the build's link registry plus the record's `links`. On
read, before anything renders:
- **An unknown link id**, one the reading build does not have, is "state present, cannot recreate" (§9), naming the
  missing link and its block.
- **A changed definition**, a link id whose registry entry's content differs from the writer's, is refused the same
  way. **Each recorded link carries its own entry hash (R-521):** the record holds, beside the build hash and the
  schema version, one entry hash per block, the hash of that link's registry entry by its semantic content and of the
  registry constants its decode reaches (`tanh_sat`, `ε_M01`, `δ_λ`, `ε_w`; `principia_dd_generation_root.md` §3.9,
  "Each entry's own hash"), a u64 written as a string (R-322), four in all
  (about 100 B of JSON). The reader compares each against its own registry's entry of that id: a mismatch is "state
  present, cannot recreate", naming exactly that link and the registry constants its hash covers (generation_root
  §3.9; physics review P6 of PR #265, applied per R-369), and, when all four blocks mismatch, leading with the
  constants all four hashes cover, computed from the record's links (`δ_λ` and `ε_w`, and `tanh_sat` when every
  recorded link is tanh-bearing; physics review P8 of PR #265, applied per R-369); when all four match, a schema version other than the reader's
  refuses nothing on the links' account, and the schema version stays the whole-ledger check of what the stored
  payload's bits mean, so a stored payload is not reused across it and the view re-integrates. A record with no entry
  hashes is read as before R-521: a schema version other than the reader's is refused, naming its links as possibly
  changed. *Was:* "meanwhile a record whose schema version differs from the reader's is refused, naming its links as
  possibly changed. Whether each recorded link carries its own entry hash, so that the refusal names exactly the
  changed link, is RQ-385." (stale value, replaced by R-521).
- **Never substitute.** No default link stands in for one the record names, and no default value for a parameter it
  records. A fallback must be visible (below).

**Old records (R-483, X5).** A record or export written before R-483 carries `"links":{}`. Under the **same build
hash** it means "this build's defaults at the time", and recreates, labelled so. Under a **different build hash** it is
"state present, links unrecorded": it recreates only if the user explicitly accepts the current defaults, and the
result is labelled as assumed. The record's `version` is bumped (§2, "Version 4").

**Tiers (R-484).** The record is the one channel whose capacity the image sets, so it degrades by tier, and only it
does. When the record does not fit, the writer drops whole tiers from the bottom:

| Tier | Contents | Dropped |
|---|---|---|
| **1. Physics** | The build hash and schema version, each recorded link's entry hash (R-521), and the whole of `SimConfig`: chart and params (a saved chart by value), plane, slice frame and tilt (map, angles, targets), lock, links and their parameters, integrator, kernel variant, horizon and thresholds, collision radius, quality, and the ensemble `E` and `N`. | Never. If tier 1 does not fit once, the image is reported **not embedded**; it is never written partially. |
| **2. Colour** | `RenderState` as built-ins: the colour mode's enum and every parameter, the ramp window, the overlays, the line system, the playhead, the hover and the debug view; and the sidecar's `f3_fill` (R-472). | Only after tier 3. |
| **3. Custom source** | Each ejected WGSL node's source (custom frag shaders and nodes); the `source` flag (§2, bit 2). | First. |

The writer serialises tier 1, tiers 1 and 2, and all three, and embeds the largest whose record gives at least one RGB
tile at the image's size (§2's tile rule); the header's flags say which tiers the record holds (§2, bits 2 and 3). A
tier is never cut. Every field's tier is assigned in one table, read by the writer and the tests
(`principia_gui_state_contract.md` §2, `crates/engine/src/contract/tiers.rs`). The `tEXt` chunk has no capacity limit,
so it always carries all three tiers. A view recreated without tier 2 renders the default stain; without tier 3, each
custom node with its slot's default built-in occupant; either is badged as a fallback (§9).

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

**To be re-measured (R-483, X6; TASK-M7-44).** The table below was measured at 596 B of JSON, 382 B deflated, before
the record carried the links, the tilt fields and a saved chart's definition (§6). It is re-measured at the new
payload, a saved tilted chart and a non-default link included, for tier 1 alone, tiers 1 and 2, and all three
(§6, "Tiers"), at 64² to 1024², and rewritten here with the measurement. The bound it is read against: a tile of side
`s` holds a record of `3·s²` bits, so at 64² one RGB tile holds a record of at most 1,536 B, a payload of at most
1,516 B deflated. If 64² no longer holds one tile of tier 1, or of tiers 1 and 2, this section says so. **The writer
never truncates** to fit: a record whose tier 1 does not fit is reported as not embedded, and the `tEXt` chunk still
carries it whole.

**Whether tier 2 splits waits for this re-measure (R-522).** RQ-386 asks whether tier 2 should split, its built-ins
and the ramp window above the overlays and lines, if tier 2 often does not fit at small sizes. The human rules it after
the re-measure: TASK-M7-44 reports tier 2's own size and, at 64² and 128², whether tiers 1 and 2 fit for the default
stain and for the fixture `tier2_typical_stain` (REQ-TOOL-174 names its contents), and RQ-386 returns to the human with those numbers. Meanwhile tier 2 is whole.

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
- **Recreating a recovered record has outcomes of its own (R-483, R-484).** The record layer's three stand; a
  recovered record is then recreated, or not: *state present, cannot recreate*, naming why (an unknown link, a changed
  definition, §6's identity); *state present, links unrecorded*, recreated only on the user's acceptance (§6, "Old
  records"); or *recovered*, saying which tiers it recovered and which the original had but the record did not
  embed, for example "recovered: physics and colour; custom shader source not embedded (it was in the original),
  rendering with the built-in fallback". The recreated view is badged as a fallback wherever it differs (§6: "A
  fallback must be visible"). The writer reports at export time which tiers fit at that resolution, so the user knows
  before sharing.
- **Golden-image test**: embed → PNG round-trip → extract → assert bitwise-equal config, at every
  supported resolution.
- **The same golden test with a non-default link and a saved tilted chart** (R-483, C10): bitwise-equal config at
  every supported resolution; the control, the image with its low bits zeroed, reads as *no embedded state*. With the
  tiers (R-484, T3): the recovered tiers are exactly those the writer reported, tier 1 bitwise whenever anything is
  embedded, and at a size too small for tier 1 the result is *not embedded*, never a partial record.
