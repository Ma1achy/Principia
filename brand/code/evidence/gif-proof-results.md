# GIF proof: measured numbers against the criterion (`gif-proof-criterion.md`)

Rebuild: `build.py --part videos --only icon-light --only icon-dark --only title-2d-light --only title-2d-dark`, Chromium 158.0.8127.36, ffmpeg 6.1.1, Pillow 13.0.0, Python 3.13.18 (sim, boards, web parts first). Reference: committed GIFs from git, committed MP4s from the release brand-media-2026-11-25.

| GIF | (a) rebuilt MP4 vs committed MP4: mean / p99 / block (limits 1.5 / 24 / 10) | (b) GIF vs own MP4, rebuilt <= committed: mean / p99 / block | (c) rebuilt GIF vs committed GIF: frames, size, fps; mean (limit 2.0) | verdict |
|---|---|---|---|---|
| `principia-icon-dark.gif` | 0.77 / 6 / 5.7 pass | 1.63 <= 2.07; 18 <= 23; 27.4 <= 30.8 pass | 432/432, 400x400/400x400, 12/12; mean 2.08 FAIL: mean 2.08 > 2.0 | NOT proven, stays in git |
| `principia-icon-light.gif` | 0.57 / 6 / 7.2 pass | 1.30 <= 1.96; 16 <= 26; 23.3 <= 49.0 pass | 432/432, 400x400/400x400, 12/12; mean 1.76 pass | proven, removed from git |
| `principia-title-2d-dark.gif` | 0.16 / 4 / 5.3 pass | 1.18 <= 1.23; 9 <= 10; 13.2 <= 17.5 pass | 540/540, 800x300/800x300, 15/15; mean 0.40 pass | proven, removed from git |
| `principia-title-2d-light.gif` | 0.14 / 4 / 6.3 pass | 0.98 <= 1.15; 9 <= 13; 12.7 <= 41.2 pass | 540/540, 800x300/800x300, 15/15; mean 0.50 pass | proven, removed from git |

## Controls (each must fail (b) or (c))

| GIF | control | (b) vs committed bound: mean / p99 / block | (b) | (c) mean (limit 2.0), frames | (c) | fails |
|---|---|---|---|---|---|---|
| `principia-icon-dark.gif` | 32-colour global palette | 4.56 (<= 2.07) / 51 (<= 23) / 37.8 (<= 30.8) | FAIL | 4.81, 432 | FAIL | yes |
| `principia-icon-dark.gif` | shifted by one GIF frame | 3.14 (<= 2.07) / 44 (<= 23) / 166.4 (<= 30.8) | FAIL | 3.56, 432 | FAIL | yes |
| `principia-icon-light.gif` | 32-colour global palette | 3.88 (<= 1.96) / 57 (<= 26) / 54.9 (<= 49.0) | FAIL | 3.71, 432 | FAIL | yes |
| `principia-icon-light.gif` | shifted by one GIF frame | 3.10 (<= 1.96) / 41 (<= 26) / 151.5 (<= 49.0) | FAIL | 3.26, 432 | FAIL | yes |
| `principia-title-2d-dark.gif` | 32-colour global palette | 2.01 (<= 1.23) / 32 (<= 10) / 39.1 (<= 17.5) | FAIL | 1.23, 540 | pass | yes |
| `principia-title-2d-dark.gif` | shifted by one GIF frame | 1.40 (<= 1.23) / 13 (<= 10) / 97.6 (<= 17.5) | FAIL | 0.69, 540 | pass | yes |
| `principia-title-2d-light.gif` | 32-colour global palette | 2.00 (<= 1.15) / 32 (<= 13) / 46.7 (<= 41.2) | FAIL | 1.42, 540 | pass | yes |
| `principia-title-2d-light.gif` | shifted by one GIF frame | 1.36 (<= 1.15) / 13 (<= 13) / 68.2 (<= 41.2) | FAIL | 0.85, 540 | pass | yes |

Every control fails (b), on every GIF. The icons' controls also fail (c); the titles' controls pass (c): the title's 32-colour palette and its one-frame shift both keep the mean against the committed GIF under 2.0, so only (b) catches them there. (b) is the test that carries the proof; (c) alone would not.
