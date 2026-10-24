# Principia: the uploaded picture — storing and carrying "Your image…"

*Adopted by R-585 (gap sweep PR 2c, 21 Oct 2026), from the human's design `uploaded_picture.md`
(`docs/incoming/2026-10-21_gap_sweep_exports_picture_ic/designs/`), its "Decided" section his words: "Accepted as
proposed". Where the draft gave a choice with a recommendation (its §5), the recommendation is what was accepted.*

## 1. What exists

The Picture base lays a picture on the target's square, looked up nearest and, by default, mirror-repeated and dimmed
at ×0.62 past it; a user's upload is "scaled to the same 256 × 256" (`principia_state_map.md` SM8). The prototype
stretches it with `drawImage(SM.upload, 0, 0, N, N)` at the browser's own smoothing (`docs/gui/reference/state_map/`,
`state_flow.html`). Colour composition §3 makes it "a `RenderState` field, assigned its tier in R-484's table by the
task that builds it". REQ-COL-130 requires a render-side texture, never a sim struct; REQ-GUI-340 requires one
`SetField`, one undo entry, labelled "Your image…". The embedding's tiers are 1 physics, 2 `RenderState` "as
built-ins", 3 custom source; `tEXt` carries everything; "never substitute"; "a fallback must be visible"
(`principia_dd_image_embedding.md` §6, §9). Saved views are pxpack snapshots (§G9) with no store defined; the share
link's format is undefined (TASK-M8-31). The corpus's hash is the 64-bit FNV-1a (`principia_render_contract.md`,
R-36's).

## 2. The gap

There is no field content, no tier, no carrier for the pixels and no missing-picture behaviour, and the prototype's
resample depends on the platform, so one file can give two pictures.

## 3. The design

### 3.1 Ingest

"Your image…" takes a PNG or a JPEG. It refuses, with a reason naming the limit, a file over **32 MB** or one larger
than **8192 × 8192** decoded; both are calibration values (R-71, REQ-COL-160), proposed here. The file is decoded,
converted to 8-bit sRGB from its embedded colour profile (an ICC profile, or PNG's `sRGB`, `gAMA` and `cHRM`; no
profile is sRGB), the conversion recorded in the import report (the draft's open question 4, (a)), and composited over
the canvas background where it has alpha. It is then **resampled to 256 × 256 by area averaging in Principia's own
code**, stretched to the square whatever its aspect, as the prototype stretches it (the draft's open question 3, (a)),
the dialog showing the stretched result before it is applied. Row 0 is the top edge, `v = 1`. **The canonical picture
is those texels**, 256 × 256 × 3 = 196,608 bytes of RGB8; the original file is not kept. The resolution is a fixed
256² until it is measured otherwise (the draft's open question 6, (a)).

**Area averaging** (applied per R-369: the draft names the method, this states it so two machines agree bitwise): each
output texel is the mean, over the source pixels it covers, of their 8-bit sRGB values, each weighted by the area of
its overlap with the texel's footprint, computed in exact integer arithmetic on the overlap areas (the footprint's edges
are at multiples of `W/256` and `H/256`, which are rationals with denominator 256), and rounded half to even to 8 bits.
No floating point and no platform resampler are used, so the texels depend only on the decoded pixels. The decode
and the profile conversion before it are the app's own pinned code, never the platform's image or colour-management
libraries, so one file gives the same 8-bit sRGB pixels on every machine (applied per R-369).

### 3.2 Identity

The `RenderState` field is a reference, never the texels:

```
PictureRef { hash, name, w: 256, h: 256, resample: "area/1" }
```

`hash` is SHA-256 (the draft's open question 1, (a)) over the ASCII bytes `principia.picture/1`, then `w` and `h` as
u32 big-endian, then the texels row by row from row 0, written as 64 lowercase hex digits. One image gives one picture,
and a later resampler, named `area/2`, gives a new hash, never a silent change. `name` is the file's base name, shown,
outside the hash. The render key covers `hash`, so R-484's identity holds: `RenderState` holds the reference.

### 3.3 The store

A content-addressed store, `pictures/<hash>.png` in the app's data folder (applied per R-369: the GUI's own data, beside
its settings), holds each picture as a lossless 256 × 256 RGB PNG, re-hashed on every read; a file whose texels do not
hash to its name is not used. An entry is kept while any history entry, saved view or result refers to it, and is
collected only when none does. An upload writes the store first, then sends **one `SetField` with the reference only**
(REQ-GUI-340's one entry); undo restores the previous reference, and the bytes stay in the store.

### 3.4 How it travels

| carrier | holds |
|---|---|
| snapshot JSON, share link, animation spec | the reference (196 KB cannot ride a URL) |
| saved view | the reference, pinning the store's entry |
| PNG low-bit record | the reference, in **tier 2** with the rest of `RenderState` |
| PNG chunks | `tEXt` with the reference; the texels in a private ancillary chunk, `prPc` (ancillary, private, safe to copy) |
| result folder | `picture.png`, the canonical texels |

**`prPc`** (the draft's open question 2, (a)) holds the reference's `hash` as 32 raw bytes, then the 196,608 texel bytes
row by row, uncompressed (applied per R-369: PNG's own CRC covers the chunk). It is written into every exported PNG
whose `RenderState` refers to a picture, whatever the embedding's tiers, and named in the export's report.

**The texels stay out of the low bits by arithmetic.** An `S × S` image's RGB low bits hold `3S²/8` bytes, so 196,608 B
needs `S ≥ 725` for a single copy with nothing else, while the tiers are measured at 64² to 1024²
(`principia_dd_image_embedding.md` §7). So the picture is an **attachment**: a channel outside the tiers, for content
too big for the low bits, in its own chunk and named in the report. Tier 2 becomes `RenderState` as built-ins **plus
references**.

### 3.5 Missing

When a reference's hash is in neither the store nor a `prPc` chunk, the view recreates everything else, bitwise:

- the Picture base draws a presentation view's invalid colour, `#141418` (`principia_colour_composition.md` §3, R-467),
  with its layers over it, as they would draw over the picture;
- the header strip says **"Picture missing: <name> · <hash, 8>"**, the first 8 hex digits (R-523);
- the import report lists it.

**Locate picture…**, offered by the badge and the report, accepts only a file whose ingested texels (§3.1) hash to the
reference, and then writes the store; any other file is refused, naming both hashes. **No built-in ever stands in**,
and the hatch is not used: it is the debug views' only (R-467; the draft's open question 5, (a)). A `prPc` whose texels
do not hash to the reference is "state present, corrupt" (embedding §9), and the view draws as missing.

## 4. Fit with rulings

- **Settles** colour composition §3's "assigned its tier in R-484's table by the task that builds it" and TASK-M7-49's:
  the reference is tier 2 in `tiers.rs`.
- **Amends** embedding §6 and §9: a new **attachment** channel outside the tiers, for content too big for the low bits,
  in its own chunk and named in the report; tier 2 becomes built-ins plus references.
- R-484's identity holds, because `RenderState` holds the reference and the render key covers `hash`. REQ-COL-130 and
  REQ-GUI-340 are unchanged.
- A second hash beside R-36's FNV-1a, for content: a matching hash is the only check before rendering, and a 64-bit FNV
  can be made to collide. R-36's hash keeps its uses. The store gives saved views a model for the one §G9 does not
  define.

## 5. Decided

The human, 21 Oct 2026 (`uploaded_picture.md`, "Decided", verbatim):

> Accepted as proposed: SHA-256, stretch to fit, a visible "Picture missing" badge, texels carried in a private PNG
> chunk, never substituted.

`DECISIONS_2026-10-21.md`, verbatim: "Uploaded picture: yes, as drafted (SHA-256, stretch, missing badge)."

## 6. Tests

| Test | Fails when |
|---|---|
| One file ingested on two machines gives one hash; a 1-pixel source change moves it | the platform's resampler is used, or the hash skips texels |
| Upload then undo: one entry, reference restored, bytes kept | bytes in history, or a referenced entry is collected |
| An exported PNG reopened with an empty store renders the picture | `prPc` is not written or not read |
| A share link with an empty store: `#141418` plus badge, the rest bitwise | a built-in substitutes, or the restore aborts |
| A tampered `prPc` reads "corrupt" | the chunk is used unverified |
| `PictureRef` is tier 2 in `tiers.rs`, and a tier-2 drop reports the picture | no tier, or a silent cut |
| A file over the limits is refused with the reason | silent downscale, or a crash |
