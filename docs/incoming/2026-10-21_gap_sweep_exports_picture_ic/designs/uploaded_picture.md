# The uploaded picture — storing and carrying "Your image…"

*Draft against main @45caf8be. Everything not cited is new.*

## 1. What exists today

The Picture base lays a picture on the target's square, looked up nearest and mirror-repeated at ×0.62 past it; a
user's upload is "scaled to the same 256 × 256" (`principia_state_map.md` SM8 :320–336). The prototype stretches it
with `drawImage(SM.upload, 0, 0, N, N)` at the browser's own smoothing (`docs/incoming/2026-10-21_state_map/demo/
state_flow.html`:1066). Colour composition §3 makes it "a `RenderState` field, assigned its tier in R-484's table by
the task that builds it" (:482–484). REQ-COL-130 requires a render-side texture, never a sim struct
(`requirements.yaml`:14949); REQ-GUI-340 requires one `SetField`, one undo entry, labelled "Your image…" (:20666,
:20682). The tiers are 1 physics, 2 `RenderState` "as built-ins", 3 custom source; `tEXt` carries everything; "never
substitute"; "a fallback must be visible" (`principia_dd_image_embedding.md` §6 :270–299, §9 :419). Saved views are
pxpack snapshots (§G9 :1225) with no store defined (TASK-M8-08 :12); the share-link format is undefined
(TASK-M8-31 :48). The corpus's hash is the 64-bit FNV-1a (render_contract :69, R-36).

## 2. The gap

There is no field content, no tier, no carrier for the pixels and no missing-picture behaviour, and the prototype's
resample depends on the platform, so one file can give two pictures.

## 3. Proposal (new)

**Ingest.** "Your image…" takes PNG or JPEG, refusing with a reason anything over 32 MB or larger than 8192² decoded
(calibration values, R-71). It is decoded to 8-bit sRGB, composited over the canvas background where it has alpha, and
**resampled to 256 × 256 by area averaging in Principia's own code**, stretched as the prototype stretches it, with the
dialog showing the stretch. Row 0 is the top edge, `v = 1`. **The canonical picture is those texels**, 196,608 B of
RGB8; the original file is not kept.

**Identity.** The `RenderState` field is a reference:
`PictureRef { hash, name, w: 256, h: 256, resample: "area/1" }`. `hash` is SHA-256 over `"principia.picture/1"`, the
dimensions and the texels, so one image gives one picture, and a later resampler gives a new version, not a silent
change.

**Store.** A content-addressed store, `pictures/<hash>.png` in the app's data folder, lossless and re-hashed on read,
kept while any history entry, saved view or result refers to it. An upload writes the store first, then sends **one
`SetField` with the reference only**. Undo restores the previous reference, and the bytes remain.

**How it travels:**

| carrier | holds |
|---|---|
| snapshot JSON, share link, animation spec | the reference (196 KB cannot ride a URL) |
| saved view | the reference, pinning the store entry |
| PNG low-bit record | the reference, **tier 2** with the rest of `RenderState` |
| PNG chunks | `tEXt` with the reference; the texels in a private ancillary chunk `prPc` (ancillary, private, safe to copy) |
| result folder | `picture.png` |

The texels stay out of the low bits **by arithmetic**: an `S × S` image's RGB low bits hold `3S²/8` bytes, so 196,608 B
needs `S ≥ 725` for a single copy with nothing else, and the tiers are measured at 64² to 1024² (§7).

**Missing.** When a hash is in neither the store nor a `prPc` chunk, the view recreates the rest. The base draws a
presentation view's invalid colour, `#141418` (colour_composition §3, R-467), with the layers over it; the header strip
says "Picture missing: <name> · <hash, 8>" (R-523); and the import report lists it. *Locate picture…* accepts only a
file whose resampled hash matches. **No built-in ever stands in.** A `prPc` whose texels do not hash to the reference
is "state present, corrupt".

## 4. Fit with rulings

- **Settles** colour_composition §3 :483 and TASK-M7-49 :31: the reference is tier 2 in `tiers.rs`.
- **Amends** embedding §6 and §9: a new **attachment** channel outside the tiers, for content too big for the low bits,
  in its own chunk and named in the report. Tier 2 becomes built-ins plus references.
- R-484's identity holds, because `RenderState` holds the reference, and the render key covers `hash`.
  REQ-COL-130 and REQ-GUI-340 are unchanged.
- A second hash beside R-36's (Q1). The store gives saved views a model for the one §G9 does not define.

## 5. Open questions

1. **Hash.** (a) SHA-256; (b) R-36's FNV-1a; (c) BLAKE3. *Recommend (a)*: a matching hash is the only check before
   rendering, and a 64-bit FNV can be made to collide.
2. **Texels in the PNG.** (a) a `prPc` chunk; (b) base64 in `tEXt`; (c) the reference only. *Recommend (a)*.
3. **Aspect.** (a) stretch, as prototyped and shown; (b) fit with bars; (c) crop to the centre. *Recommend (a)*.
4. **Colour profile.** (a) convert ICC to sRGB, recorded; (b) treat the image as sRGB; (c) refuse non-sRGB.
   *Recommend (a)*: the texels are canonical either way, and (a) matches what the user saw.
5. **Missing.** (a) `#141418` plus a badge; (b) refuse to recreate; (c) the hatch. *Recommend (a)*: the hatch is the
   debug views' only (R-467).
6. **Resolution.** (a) a fixed 256²; (b) a choice of 256 to 1024; (c) native. *Recommend (a)* until measured.

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


## Decided (Malachy, 21 Oct 2026)

Accepted as proposed: SHA-256, stretch to fit, a visible "Picture missing" badge, texels carried in a private PNG
chunk, never substituted.
