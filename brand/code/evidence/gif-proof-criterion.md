# Proof that the brand build regenerates the four animated GIFs

Written and committed before any rebuilt file is measured. The criteria below are not changed after measuring.

GIFs covered: `icons/animated/principia-icon-{dark,light}.gif` and `titles/principia-title-2d-{dark,light}.gif`.
The two subtitle GIFs are not covered and stay in git.

Why this test. A GIF is a lossy 256-colour encoding of its MP4. The encoder settings were recovered from the
original GIFs (`gif-encoder-settings.md`) and `encode.py` uses them, so a rebuilt GIF reproduces the committed one
up to the render, and is compared with it directly. `compare.py`'s `'gif'` block and p99 limits are not changed
and are not used as the pass test here.

Setup. The committed MP4s and GIFs are the reference. The MP4s are the release assets of
`brand-media-2026-11-25`, downloaded into a copy of `brand/` outside the repo. The rebuild is
`build.py --part videos --only <name>` with Chromium 158.0.8127.36 and ffmpeg 6.1.

## Pass criterion, per GIF: BOTH (a) and (c)

The owner chose this criterion once the encoder settings were recovered.

(a) The rebuilt MP4 meets `compare.py`'s `'video'` criterion against the committed MP4 (`compare_video`
    with its existing limits: mean 1.5, p99 24, block 10, alpha mean 1.5; same codec, size, frame count, fps).

(c) The rebuilt GIF against the committed GIF: same frame count, width and height, and frame rate, and
    worst-frame `mean <= 2.0` levels (the existing `'gif'` mean limit; block and p99 are not asked of it).

A GIF whose (a) and (c) both hold is removed from git. Any other stays. The subtitle GIFs are measured as a check
only and stay in git.

## Control (must fail, or the proof is void and nothing is removed)

Two deliberately wrong GIFs for each of the four GIFs, made from that GIF's rebuilt MP4, with `encode.py`'s settings
except for one change each:
  1. a 32-colour global palette (`palettegen=max_colors=32`, `paletteuse` with it);
  2. the same encoding as the rebuilt GIF but shifted by one GIF frame (frame i shows the loop at i+1, wrapping),
     with the same frame count, size and fps.
Each is put through (c) with the threshold of its own GIF (its (a) is the rebuilt MP4's and is the same for every GIF made
from it). Each must FAIL (c). The report names the numbers. If either control passes (c), the proof is void.
The control is run on all four GIFs. A GIF whose control does not fail is not proven (its threshold cannot tell a wrong
GIF from a right one), and stays. If any control passes (c), the proof is reported as void and nothing is removed.
