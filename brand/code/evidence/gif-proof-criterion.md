# Proof that the brand build regenerates the four animated GIFs

Written and committed before any rebuilt file is measured. The criteria below are not changed after measuring.

GIFs covered: `icons/animated/principia-icon-{dark,light}.gif` and `titles/principia-title-2d-{dark,light}.gif`.
The two subtitle GIFs are not covered and stay in git.

Why this test. A GIF is a lossy 256-colour encoding of its MP4, and the MP4 is what the rebuild must reproduce.
The original GIF encoder settings were not kept, so a rebuilt GIF cannot be compared with the committed one
by `compare.py`'s block limit: the dither moves 16 px blocks by tens of levels. `compare.py`'s `'gif'`
criterion is not changed and is not used as the pass test here.

Setup. The committed MP4s and GIFs are the reference. The MP4s are the release assets of
`brand-media-2026-11-25`, downloaded into a copy of `brand/` outside the repo. The rebuild is
`build.py --part videos --only <name>` with Chromium 158.0.8127.36 and ffmpeg 6.1.

## Pass criterion, per GIF: ALL of (a), (b), (c)

(a) The rebuilt MP4 meets `compare.py`'s `'video'` criterion against the committed MP4 (`compare_video`
    with its existing limits: mean 1.5, p99 24, block 10, alpha mean 1.5; same codec, size, frame count, fps).

(b) The rebuilt GIF is at least as faithful to the rebuilt MP4 as the committed GIF is to the committed MP4.
    With `M(gif, mp4) = compare.gif_vs_own_mp4(gif, mp4)`:
        mean(M(rebuilt))  <= mean(M(committed))
        p99(M(rebuilt))   <= p99(M(committed))
        block(M(rebuilt)) <= block(M(committed))
    No margin. `mean` is the worst-frame mean, `p99` the 99th percentile over all pixels of all frames,
    `block` the worst 16 px block mean, as `aggregate()` defines them.

(c) The rebuilt GIF against the committed GIF: same frame count, width and height, and frame rate, and
    `mean <= 2.0` levels (the existing `'gif'` mean limit; block and p99 are not asked of it).

A GIF whose (a), (b) and (c) all hold is removed from git. Any other stays.

## Control (must fail, or the proof is void and nothing is removed)

Two deliberately wrong GIFs for each of the four GIFs, made from that GIF's rebuilt MP4, with the rebuilt encoder settings
except for one change each:
  1. a 32-colour global palette (`palettegen=max_colors=32`, `paletteuse` with it);
  2. the same encoding as the rebuilt GIF but shifted by one GIF frame (frame i shows the loop at i+1, wrapping),
     with the same frame count, size and fps.
Each is put through (b) and (c) with the thresholds of its own GIF. Each must FAIL at least one of (b) or (c). The report names which test
catches which control. If either control passes both, the proof is void.
The control is run on all four GIFs. A GIF whose control does not fail is not proven (its thresholds cannot tell a wrong
GIF from a right one), and stays. If any control passes both tests, the whole proof is reported as void and nothing is removed.
