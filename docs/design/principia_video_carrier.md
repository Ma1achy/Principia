# Principia: the video carrier — a recording carries its settings in its own file

*Adopted by R-584 (gap sweep PR 2c, 21 Oct 2026), from the human's test `video_carrier_test.md`
(`docs/incoming/2026-10-21_gap_sweep_exports_picture_ic/designs/`, its script `evidence/vt.py` beside it, reference
only), and his ruling, "yes always embed the settings".*

## 1. What exists

The corpus promises "pxpack in every frame" for every recording format (`principia_render_gui_spec.md` §G9, Record a
time sweep), and "Exported videos embed (or link) the spec" (`principia_export_animation_contract.md` Part 6;
REQ-TOOL-083, REQ-TOOL-106). The only carrier designed is the image's low bit (`principia_dd_image_embedding.md`).
Embedding §9 puts the low-bit record "off by default for figure export", on for share. The Sequence writes its spec
beside every export as `.principia-seq.json` (R-577, REQ-TOOL-200).

## 2. What was measured

`vt.py`: 12 frames of 256², a 6 KB spec, ffmpeg and Pillow. The low-bit route, the spec's worst-frame bit error rate
(0 intact, 0.5 pure noise):

```
PNG frames (control)                   0
GIF (ffmpeg palette, PIL palette)      0.50–0.51
MP4 H.264, normal quality              0.50
MP4 H.264 lossless, YUV 4:4:4          0.39   (colour conversion alone breaks it)
MP4 H.264 lossless RGB (libx264rgb)    0
WebM VP9, normal quality               0.50
WebM VP9 lossless                      0
```

Only lossless RGB video keeps it, and those files are large and poorly supported by players, so the low-bit record is
for PNG only. Container metadata:

```
GIF comment block                          intact      (lost if ffmpeg re-encodes the GIF)
MP4 standard comment (udta)                intact      survives remux AND an ffmpeg re-encode; a 64 KB spec fits
MP4 custom key (use_metadata_tags)         intact      lost on remux: do not use
WebM tag PRINCIPIA_SPEC                    intact      survives remux and re-encode
MKV attachment                             exact bytes
```

## 3. The design

### 3.1 Once per file, in the format's own metadata, always

A recording (Record a time sweep, §G9; the Sequence's export, `principia_sequence.md` §3.7) carries its record **once
per file**, in the format's own metadata, **always**: "yes always embed the settings". No option turns it off. A
frame's playhead follows from its index and the recording's settings (§3.4), so per-frame copies add nothing.

- **The record** is the embedding's record (`principia_dd_image_embedding.md` §2, "RECORD = header ‖ payload ‖
  crc32(payload)"): its header, its CRCs and its payload, the recording's settings (§3.3) as one JSON object in the
  canonical serialisation (R-309, R-318) compressed with raw DEFLATE. The record is written when the recording ends, so
  it holds each frame's clipped shares (`principia_export_details.md` §3.4).
- **The carriers**, each holding the record's bytes as base64 (RFC 4648 §4, the standard alphabet with padding, no line
  breaks) where the field is text (applied per R-369: the draft's "the compressed record", as `vt.py` wrote it):
  - **GIF**: a Comment Extension block holding the base64 text, and an Application Extension with the identifier
    `PRINCPIA` and the authentication code `REC` holding the record's raw bytes, each in sub-blocks of at most 255
    bytes, both after the last frame and before the trailer. A reader that finds both takes the application extension's,
    and reports the two as disagreeing if they differ.
  - **MP4**: the standard comment field, written through the system `ffmpeg` (R-131) as `-metadata comment=<base64>`, the
    `udta` comment that survives a remux and a re-encode. No custom key: it is lost on remux.
  - **WebM** (the browser's, through WebCodecs where supported, R-131): a tag named `PRINCIPIA_SPEC` holding the base64
    text, on the file as a whole.
- **Reading a recording back** (Import picture, §G9, given a GIF, MP4 or WebM) decodes the carrier and reads the record
  as the embedding's reader does: the three outcomes, never two (embedding §9: *no embedded state*, *state present,
  corrupt*, *recovered*), then R-483's and R-484's outcomes of recreating it.

### 3.2 PNG frames keep the low-bit record

A recording written as PNG frames keeps the low-bit pxpack in every frame, as now, each frame's record its own,
carrying its own playhead (R-520 F5), with the `tEXt` chunk beside it; the lossless PNG keeps it intact (§2). This is
always on for a recording's frames: embedding §9's "off by default for figure export" stays for a single figure, an
image exported from Export & share, and does not apply to a recording (the human's instruction: "Embedding §9's 'off by
default for figure export' stays for single figures").

### 3.3 The settings file beside every export

No metadata survives an upload to most websites, which strip it and re-encode, so the settings file is **always
written beside the export** as well, in every format, PNG frames included:

- the Sequence's export writes its spec's `.principia-seq.json` (R-577, REQ-TOOL-200);
- Record a time sweep writes `<name>.principia-sweep.json` (applied per R-369: the name of the sweep's file): one JSON
  object in the canonical serialisation, `{ format: "principia.sweep/1", build, schema, entry_hashes, sim: SimConfig,
  render: RenderState, sweep: { t_start, t_end, frames, quality, width, height, format, overlays }, export }`, the
  export block of `principia_export_details.md` §3.4 with one clipped map per frame.

The record the container carries (§3.1) has the same JSON as the file beside it, so the two are bitwise the same
settings.

### 3.4 A frame's playhead

Record a time sweep's frame `i` of `n` is at playhead `t_i = t_start + (t_end − t_start) × i / (n − 1)`, evaluated in
f64 in that order, for `i` from 0 to `n − 1`; one frame (`n = 1`) is at `t_start` (applied per R-369: the rule the
corpus did not write down, which a reader needs to recover a frame's state from its index). A Sequence frame's state
follows from its index by export contract Part 3's frame grid.

### 3.5 What does not change

- The spotlight reel's film (R-578) is a release output made by the reel builder with its manifest, which maps every
  film frame to its state; it is not a user's recording, and this design adds nothing to it (applied per R-369).
- A single figure from Export & share keeps embedding §9's defaults: low bits off for a figure, on for share, and the
  `tEXt` chunk always.

## 4. Fit with rulings

- **Amends** §G9's "pxpack in every frame", which becomes "the record in the file, by format" (§3.1, §3.2), and
  REQ-TOOL-083's and REQ-TOOL-106's tests, which read the container's record back bitwise, with a re-encode arm for MP4
  and WebM.
- Export contract Part 6's "embed (or link) the spec" is met by both: the record in the file and the file beside it.
- Embedding §9's "off by default for figure export" stays for single figures (§3.5); a recording always embeds.
- R-131 and R-155: the encoders are unchanged; the GIF carrier is written by the one `gif` crate, MP4's through the
  system `ffmpeg`.
- R-577's `.principia-seq.json` is the Sequence's file beside the export; this design adds the time sweep's.

## 5. Decided

The human, 21 Oct 2026 (`DECISIONS_2026-10-21.md`, verbatim): "Video: always embed the settings. "yes always embed the
settings"". His instruction (`INSTRUCTION.md` § "PR 2 — New design documents", the `video_carrier_test.md` item,
verbatim): "a recording carries its settings once per file in the format's own metadata (GIF comment, MP4 comment, WebM
tag), and **always** (my words: "yes always embed the settings"); PNG frames keep the low-bit record; the settings file
is always written beside the export. Amend §G9's "pxpack in every frame" and the tests of REQ-TOOL-083 and -106.
Embedding §9's "off by default for figure export" stays for single figures."

That answers the draft's one open question ("Pick one default for recordings"): always.

## 6. Tests

- A GIF, an MP4 and a WebM recording each carry the record once, read back bitwise equal to the file beside it; an MP4
  and a WebM after an ffmpeg remux and a re-encode at its defaults still do; a GIF re-encoded by ffmpeg loses it and
  reads *no embedded state*, the file beside it still read.
- PNG frames each carry their own low-bit record with their own playhead.
- Every recording writes its settings file beside it, in every format.
- Embedding the settings cannot be turned off for a recording; a single figure's low bits stay off by default.
- A reader given a GIF whose comment differs from its application extension reads the application extension's record,
  recovered, and reports the two as disagreeing; a byte changed in an MP4's base64 reads *state present, corrupt*.
- Known answers pin the layouts: the GIF's `PRINCPIA` / `REC` extension, its sub-blocks of at most 255 bytes and its
  padded, single-line base64 are checked against checked-in expected bytes, so a writer and reader that agree on
  another layout fail.
- The WebM tag is written only by the browser build (R-131), so its writer is tested in the M8 Playwright browser suite
  (Chromium), and the native reader on a checked-in WebM fixture carrying the tag.
- In CI the ffmpeg remux and re-encode arms fail, never skip, when the system ffmpeg is absent.
*(These four added by fix round 2 of PR #287, qa review 5638084688, Q1, Q2, Q3 and Q9, applied per R-369.)*
