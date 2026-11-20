# Can a recording carry its settings? Tested

Item 5. The corpus promises "pxpack in every frame" for every recording format (render_gui_spec §G9 :1230), and
"exported videos embed (or link) the spec" (export contract Part 6 :96, REQ-TOOL-083, REQ-TOOL-106). The only carrier
designed is the image's lowest bit. Script: `vidtest/vt.py`. 12 frames of 256², a 6 KB spec, ffmpeg and Pillow.

## A. The low-bit route: destroyed by every ordinary format

Worst-frame bit error rate of the spec (0 = intact, 0.5 = pure noise):

```
PNG frames (control)                   0
GIF (ffmpeg palette, PIL palette)      0.50–0.51
MP4 H.264, normal quality              0.50
MP4 H.264 lossless, YUV 4:4:4          0.39   (colour conversion alone breaks it)
MP4 H.264 lossless RGB (libx264rgb)    0
WebM VP9, normal quality               0.50
WebM VP9 lossless                      0
```

Only lossless RGB video keeps it, and those files are large and poorly supported by players. So low-bit embedding is
for PNG only.

## B. Container metadata: works in every format

```
GIF comment block                          intact      (lost if ffmpeg re-encodes the GIF)
MP4 standard comment (udta)                intact      survives remux AND an ffmpeg re-encode; a 64 KB spec fits
MP4 custom key (use_metadata_tags)         intact      lost on remux: do not use
WebM tag PRINCIPIA_SPEC                    intact      survives remux and re-encode
MKV attachment                             exact bytes
```

## Proposed

- **A recording carries its spec once per file, not in every frame.** For GIF that's the comment block (plus an
  application extension), for MP4 the standard comment field, and for WebM a tag, each holding the compressed record.
  A frame's playhead follows from its index and the recording's settings, so per-frame copies add nothing.
- **PNG frame sequences keep the low-bit pxpack in each frame**, as now.
- **The spec file is always written beside the export as well.** No metadata survives upload to most websites, which
  strip it and re-encode.
- **Amend:** §G9's "pxpack in every frame" becomes "the record in the file, by format", and REQ-TOOL-083/106's tests
  read the container record back bitwise, with a re-encode arm for MP4 and WebM.
- **Also unresolved, for you:** embedding §9 says pxpack is **off by default for figure export** and on for share,
  while §G9 implies it is always on for recordings. Pick one default for recordings.
