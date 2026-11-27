# GIF proof: measured numbers against the criterion (`gif-proof-criterion.md`)

Rebuild: `build.py --part sim --part boards --part web --part videos --only icon- --only title-2d`, Chromium 158.0.8127.36, ffmpeg 6.1.1, Pillow 13.0.0, `encode.py`'s recovered settings. Reference: committed icon-dark GIF from git (sha256 `5132fcdf...a9a272`, identical to the release's), the other GIFs and every MP4 from the release brand-media-2026-11-25.

## Result: the proof is void, nothing is removed

Every GIF meets (a) and (c). But the controls (32-colour palette, one-frame shift) of the title GIFs and the subtitle GIFs pass (c): at mean 2.0 against the committed GIF, (c) cannot tell those wrong GIFs from a right one. The icons' controls fail (c). By the criterion, any control passing (c) voids the proof, so `principia-icon-dark.gif` stays in git. (The three GIFs already off git stay off; their proof rested on the old test (b).)

## (a) and (c) per GIF

| GIF | (a) rebuilt MP4 vs committed MP4: mean / p99 / block (limits 1.5 / 24 / 10) | (c) rebuilt GIF vs committed GIF: frames, size, fps (rebuilt/committed); mean (limit 2.0) |
|---|---|---|
| `principia-icon-dark.gif` | 0.77 / 6 / 5.7 pass | 432/432, 400x400/400x400, 12/12; mean 0.22 pass |
| `principia-icon-light.gif` | 0.57 / 6 / 7.2 pass | 432/432, 400x400/400x400, 12/12; mean 0.91 pass |
| `principia-title-2d-dark.gif` | 0.16 / 4 / 5.3 pass | 540/540, 800x300/800x300, 15/15; mean 0.05 pass |
| `principia-title-2d-light.gif` | 0.14 / 4 / 6.3 pass | 540/540, 800x300/800x300, 15/15; mean 0.24 pass |
| `principia-title-2d-subtitle-dark.gif` | 0.14 / 4 / 5.5 pass | 540/540, 800x180/800x180, 15/15; mean 0.10 pass |
| `principia-title-2d-subtitle-light.gif` | 0.12 / 4 / 6.5 pass | 540/540, 800x180/800x180, 15/15; mean 0.16 pass |

The four proved GIFs and the two subtitle GIFs (a check only; they stay in git).

## Controls: each must fail (c) (limit mean 2.0)

| GIF | control | (c) mean | result |
|---|---|---|---|
| `principia-icon-dark.gif` | 32-colour global palette | 4.81 | FAIL |
| `principia-icon-dark.gif` | shifted by one GIF frame | 3.31 | FAIL |
| `principia-icon-light.gif` | 32-colour global palette | 3.71 | FAIL |
| `principia-icon-light.gif` | shifted by one GIF frame | 3.13 | FAIL |
| `principia-title-2d-dark.gif` | 32-colour global palette | 1.23 | passes (control does not fail) |
| `principia-title-2d-dark.gif` | shifted by one GIF frame | 0.68 | passes (control does not fail) |
| `principia-title-2d-light.gif` | 32-colour global palette | 1.42 | passes (control does not fail) |
| `principia-title-2d-light.gif` | shifted by one GIF frame | 0.83 | passes (control does not fail) |
| `principia-title-2d-subtitle-dark.gif` | 32-colour global palette | 0.90 | passes (control does not fail) |
| `principia-title-2d-subtitle-dark.gif` | shifted by one GIF frame | 0.47 | passes (control does not fail) |
| `principia-title-2d-subtitle-light.gif` | 32-colour global palette | 1.15 | passes (control does not fail) |
| `principia-title-2d-subtitle-light.gif` | shifted by one GIF frame | 0.51 | passes (control does not fail) |
