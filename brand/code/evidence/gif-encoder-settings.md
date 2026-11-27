# GIF encoder settings recovered from the original GIFs

Written before `gif_proof.py` was rerun with the new settings. The proof criterion (`gif-proof-criterion.md`) and
`compare.py` are unchanged.

Evidence: the four original GIFs (`principia-icon-{dark,light}.gif`, `principia-title-2d-{dark,light}.gif`; icon-dark
from git, sha256 `5132fcdf...a9a272`, the others from the release `brand-media-2026-11-25`) and the two subtitle GIFs
in git. The question was which ffmpeg 6.1 filter settings made them. Frames for the comparison below were rendered by
`build.py --part videos` (Chromium 158.0.8127.36) and encoded by candidate settings; each candidate was compared with
the original GIF pixel by pixel (mean absolute difference over all frames and channels, and the fraction of
identical values).

## What the files themselves say

| fact in the original files | what it means |
|---|---|
| one global colour table; frames 2 to N carry no local table (all six GIFs) | one palette for the whole loop: `palettegen` `stats_mode=full` (the default), not `single` (a palette per frame, which the old `encode.py` used) |
| every frame has disposal "do not dispose" and a transparent index; frames after the first are sub-rectangles | ffmpeg's GIF muxer cropping each frame to what changed; it does this by itself, so `paletteuse diff_mode` stays at its default (`none`) |
| palette index 255 is `(0,255,0)`, the colour `palettegen` reserves for transparency | `reserve_transparent` stays on (default) |
| titles: 255 distinct colours + the reserved one | `max_colors` at its default 256 |
| icons (dark and light): 191 distinct colours at indices 0 to 190, indices 191 to 254 repeat the last colour, 255 is the reserved one | the palette was built with `max_colors=192` (191 colours plus the reserved one); `palettegen` pads the rest by repeating. A default build of the same frames gives 255 distinct colours |
| delays 8, 8, 9 cs (12 fps) and 7, 7, 6 cs (15 fps); loop 0 | ffmpeg `fps` filter and GIF muxer defaults; `-loop 0` |
| sizes 400 and 800 px wide | `scale=W:-1` |

## Measured against the originals

Same frames, only the palette and dither settings changed. Mean is in levels out of 255 (0 would be identical).

| GIF | settings | file bytes (original) | mean abs diff | identical values |
|---|---|---|---|---|
| icon-dark | old: `stats_mode=single`, `new=1` | 19,945,648 (16,835,538) | 1.82 | 55.9 % |
| icon-dark | global palette, default 256 colours | 18,551,798 | 1.42 | 70.2 % |
| icon-dark | `max_colors` 190 / 191 / 193 | 16.84 M / 16.85 M / 16.84 M | 0.62 / 0.55 / 0.32 | 93.8 / 94.5 / 96.7 % |
| icon-dark | **`max_colors=192`, default dither** | 16,835,395 | **0.0008** | **99.99 %** |
| icon-dark | `max_colors=192`, `diff_mode=rectangle` | 16,832,896 | 0.0010 | 99.97 % |
| icon-dark | `max_colors=192`, dither floyd_steinberg / heckbert / sierra2 / sierra3 / atkinson / none | 12.3 to 16.1 M | 1.00 to 1.09 | 86 to 87 % |
| icon-dark | `max_colors=192`, bayer (scale 2) | 14,704,716 | 3.65 | 29.5 % |
| icon-light | `max_colors=192` (palette equals the original's except 2 entries by 1 level) | 13,510,000 (13,508,184) | 0.36 | 97.0 % |
| icon-light | default 256 colours | 14,322,096 | 1.12 | 79.8 % |
| title-dark | default (256) | 7,221,919 (7,225,799) | 0.0016 | 99.97 % |
| title-dark | `max_colors=255` | 7,221,946 | 0.043 | 99.3 % |
| title-light | default (256) | 6,650,694 (6,646,928) | 0.13 | 98.5 % |

The dither is ffmpeg's default (`sierra2_4a`, an error-diffusion dither): the other error-diffusion dithers and
`none` are about 1 level worse, ordered Bayer is 3 to 12 levels worse, and the default reproduces icon-dark to 0.0008.
The scale filter stays Lanczos: a scan of the scaler on the first 60 icon-dark frames cannot tell Lanczos from
bicubic (mean difference to the original 1.912 and 1.915; bilinear 1.956, area 1.939), because the dither noise
dominates it. What settles it is the full encode with the settings above: Lanczos gives 0.0008, bicubic 2.23 (23 %
identical values), bilinear 2.06 (25 %).

What is left over (icon-light 0.36, title-light 0.13) is the render, not the encoder: for icon-light the palettes agree to
within 1 level in 2 entries, so the frames the original was made from differ from this machine's by a few pixels
(a palette is sensitive to that), while icon-dark and title-dark come out identical.

## Settings recovered (applied in `encode.py`)

`overlay` on the page colour, `fps`, `scale=W:-1:flags=lanczos`, `palettegen` with `max_colors=192` for the icons and
the default 256 for the titles (stats_mode full, one palette for the whole loop), `paletteuse` with its defaults,
`-loop 0`. Fixed by measuring against the originals. The proof run with them is in `gif-proof-results.md`.
