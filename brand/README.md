# Principia: icon and wordmark

The mark is the P from the Principia title wordmark, extruded in 3D, with three bodies in magenta, yellow and cyan flying round it and through its hole. The orbit is real: the **Dragonfly** periodic solution of the equal-mass planar three-body problem (Šuvakov and Dmitrašinović), integrated here with DOP853 at a tolerance of 1e-13. Energy is conserved to about 1e-11 over a period.

The light version is a warm-black P (`#1d1b19`) for paper (`#efe9dc`). The dark version is a paper-coloured P for near-black (`#0d0c0b`). Both are drawn from the same camera and geometry; only the colours and the way light blends differ. The dark version blends light additively, so overlapping colours make green, orange and white. The light version mixes them like inks.

## What's in here

```
icons/
  source/    1024 px stills, transparent and on a background, light and dark;
             a 2048 px uncropped render of each; and a "smallsizes" cut with heavier trails
  web/       favicon.ico, favicon-dark.ico, 16/32/48/96 px PNGs, apple-touch-icon,
             192/512 px icons, maskable 512, site.webmanifest, head-snippet.html
  macos/     Principia.icns and Principia-dark.icns, plus the .iconset folders
  windows/   Principia.ico (16 to 256 px) and the tile PNGs, light and dark
  ios/       AppIcon.appiconset: an opaque light icon and a transparent dark-appearance icon (iOS 18+)
  android/   res/: launcher PNGs per density, adaptive foreground and background, night variants
  linux/     hicolor/ and hicolor-dark/ icon themes, 16 to 512 px
  animated/  the icon as a 36-second loop, light and dark: MP4, WebM (transparent) and GIF
titles/      the animated 2D wordmark, with and without "Planar Three-Body Problem",
             light and dark: MP4 on a background, WebM with a transparent background, and GIF
social/      the 1280 x 640 social-preview image (dark and light) and the script that renders it
code/
  web/       self-contained JavaScript for each variant, demo pages, and the animated favicon
  python/    the simulations, the clearance searches and the generators that built everything
  boards/    the design-canvas source for each variant
```

## Using it on the web

Copy the files in `icons/web/` to your site's root and paste `head-snippet.html` into `<head>`. The favicon follows the visitor's light or dark theme.

**Animated favicon (Chrome, Edge, Firefox).** Load the two icon modules and the favicon script; Safari keeps the static favicon.

```html
<script src="principia-icon-light.js"></script>
<script src="principia-icon-dark.js"></script>
<script src="favicon-animated.js" defer></script>
```

It redraws the tab icon about 15 times a second, follows `prefers-color-scheme`, pauses when the tab is hidden and shows a still frame under `prefers-reduced-motion`. App icons can't animate on iOS, macOS, Windows or Android, so those packs use a still frame (17 s into the loop).

**The animated mark or title on a page.** Each file in `code/web/` is self-contained, with no dependencies:

```html
<div id="mark"></div>
<script src="principia-icon-dark.js"></script>
<script>
  PrincipiaMark['icon-dark'].mount(document.getElementById('mark'), { size: 320 });
</script>
```

Options are `size`, `speed`, `glow` (orb size), `weight` (trail thickness), `still` and `frame` (seconds) for a fixed frame, `transparent`, and `backdrop` (a colour shown behind the mark that isn't part of it). `PrincipiaMark[name].svgAt(t, options)` returns the SVG for time `t` as a string. The variants are `icon-light`, `icon-dark`, `title-2d-light`, `title-2d-dark`, `title-2d-subtitle-light` and `title-2d-subtitle-dark`. Open the matching `.html` file to see each one.

## Videos, GIFs and WebP

Every animation is one seamless 36-second loop, rendered frame by frame in Chrome: 30 fps for the titles, 24 fps for the icons. Frame `i` shows the mark at `6 + i/fps` seconds.

- **WebM:** has a real transparent background, so the glow fades into whatever is behind it. Use this on the web: `<video autoplay loop muted playsinline>`.
- **MP4:** sits on paper (light versions) or near-black (dark versions), for anywhere that can't play WebM.
- **GIF:** also on its background, because GIF transparency is all-or-nothing and can't carry a soft glow. They are smaller and slower: titles are 800 px wide at 15 fps, and icons are 400 px at 12 fps (every second frame).
- **WebP (subtitle title only):** `principia-title-2d-subtitle-{dark,light}.webp` keep the transparent background (alpha); `principia-title-2d-subtitle-github-{dark,light}.webp` are drawn on GitHub's own page colours, `#0d1117` and `#ffffff`, so the glow and the ink-style mixing are computed on that colour and no box shows. They are 1200 px wide at 15 fps, lossy, with the quality set to the highest that keeps each file under 4 MB (`webp-settings.json` in a rebuild records it). The page-colour ones carry a 12 px margin of the page colour so that the outer border is exactly that colour.

The repository README shows the page-colour WebPs under `prefers-color-scheme`, and `principia-title-still-{dark,light}.svg` under `prefers-reduced-motion`.

**Lining the title up with the banner.** The title's frame has margins for the glow, so with both at full width the P's stem sat about 48 px to the right of the banner's left edge at 1012 px and the subtitle ended about 34 px short of its right edge. `titlelayout.py` measures where the stem and the end of the subtitle sit in the frame (65 px in, 42 px from the right edge), pads the side with less margin so both insets are equal (never crops: the glow reaches the edge), and gives the banner `width = (frame - 2 inset) / frame` as a percentage (90.7 %). Every title the README can show (both page-colour WebPs, both transparent WebPs, both stills) has that same inset fraction. `readme_align.py` renders the README at 700, 1012 and 1280 px and reads the stem and subtitle edges from the pixels: within 1 px of the banner's edges in every variant (the control with the old full-width banner reads 47 px).

**Where the removed files are.** The MP4 and WebM files of the animated icons and titles are no longer in git (the GIFs stay: no rebuilt GIF met its criterion): they are on the release https://github.com/Ma1achy/Principia/releases/tag/brand-media-2026-11-25 and `build.py --part videos` makes them again. To run `compare.py` or `controls.py` against the originals, put the release files back at their paths in a copy of `brand/` and pass that copy as the reference.

## Rebuilding

Everything in `icons/`, `titles/`, `social/out/`, `code/web/` and `code/boards/` can be rebuilt from a clean checkout:

```
pip install -r brand/code/requirements.txt
python3 -m playwright install chromium          # the pinned Chromium: 158.0.8127.36
python3 brand/code/build.py --out /somewhere/else          # about 30 minutes and 2 GB of frames
python3 brand/code/compare.py brand /somewhere/else        # per-file verdict against the committed files
```

`--out` is required to be outside `brand/`; the build writes only there and deletes nothing. Run one part with `--part sim|boards|web|stills|videos|social` (`--only icon-light` narrows `videos`). It needs ffmpeg 6.1 with libx264 and libvpx; `build-info.json` in the output records every version used.

| part | scripts | makes |
|---|---|---|
| sim | `sim.py`, `cat.py`, `dragon.py` | `traj.npz`, `catalog.npz` (the orbits, DOP853 at 1e-13) and `dragon_adapt.json` |
| boards | `final.py`, `wordmarks.py` | the ten boards in `code/boards/` |
| web | `build_runtime.py` | `code/web/` |
| stills | `render.py`, `packs.py` | the four 2048 px Chrome masters, the 1024 px crops, every pack in `icons/` |
| videos | `render.py`, `encode.py` | frames, MP4, WebM, GIF, WebP |
| social | `social/make_social_preview.py` | `social/out/` |

The inputs that are committed rather than rebuilt are in `code/python/data/`: the traced P and wordmark (`pgeo.json`, `word_loops.json`, `sub_loops.json`, traced by `trace_p.py` from `docs/readme/title-light.svg`, with the corner indices added by hand), the placement of each orbit (`clear.json`, `orb_*.json`, `dragon_P.json`, `dragon_frame.json`, `moth_free.json`; the search scripts that found them are exploration and some of their intermediate files were not kept) and the framing of the titles (`word_frames.json`). 

Encoder settings (the originals were not kept; the MP4 header records libx264 crf 15 and ffmpeg 6.1):

- MP4: libx264, preset veryfast, crf 15, yuv420p, frames composited on the page colour.
- WebM: libvpx-vp9, alpha plane (yuva420p), constant quality `-crf 28 -b:v 0`.
- GIF: ffmpeg `fps`, Lanczos scale, `palettegen stats_mode=single` and `paletteuse new=1` (a palette per frame).
- WebP: Pillow, lossy, method 4, colour and alpha quality equal and chosen per file.

## How the pictures are made (`code/python/`)

The scripts read and write relative to their working folder; `build.py` runs them in a copy of `code/python/` inside `--out`, so none of them touches the committed files.

1. `trace_p.py`: traces the P, the full wordmark and the subtitle from `docs/readme/title-light.svg` into polygons (96 points per curve for the P). Its output is committed in `data/`.
2. `sim.py` and `cat.py`: integrate the periodic orbits (figure-eight, Butterfly I to III, Moth I to III, Goggles, Dragonfly, Yin-yang, Bumblebee) and Burrau's 3-4-5 problem. The initial conditions are the published ones: Table I of Šuvakov and Dmitrašinović, PRL 110, 114301 (2013), and the Chenciner-Montgomery figure-eight; `verify_paths.py` checks every literal against the copies in this repository.
3. `dragon.py` places the Dragonfly orbit using `dragon_P.json` and resamples the paths adaptively (a new point whenever the path turns 2.5°), so the trails stay smooth through close encounters. The searches that found the placements (`sorb.py`, `hole*.py`, `clear*.py`, `dragon2.py`, `dragon3.py`) are exploration, not part of the build; their results are the committed data files. `verify_clearance.py` and `js/verify.js` confirm the result: no point of any trail or body is ever inside the solid P, and the orbit passes through the hole 9 times per loop.
4. `final.py` and `wordmarks.py`: build the renderers. The P is extruded and shaded by a key light plus the coloured light of each body, and drawn back to front with the bodies passing behind and in front. Trails are tapered neon ribbons, split exactly where they cross the P's plane. Bloom and halation come from one stacked-blur filter, and glow from behind the P is masked by the letter's silhouette.
5. `build_runtime.py`: turns the boards into the standalone JavaScript in `code/web/`.
6. `render.py`, `packs.py` and `encode.py`: render stills and title frames in headless Chrome, then build the platform packs and videos. The 1024 px masters are a fixed crop of the 2048 px render, resampled with Lanczos.
7. `compare.py`: the rebuild is judged file by file (identical bytes for text; decoded pixels with the criteria written at the top of the file for pictures and videos). `controls.py` feeds it wrong pictures that it must reject. `checks.py` is the quick version that needs only `python3`.

## Platform notes

- **macOS:** Pillow's `.icns` writer leaves out a couple of the smallest 1× sizes. For a complete file, run `iconutil -c icns Principia.iconset` on a Mac; the iconsets are included.
- **iOS:** the light icon is opaque, as Apple requires. The dark-appearance icon is transparent, so iOS draws its own dark background behind it.
- **Android:** the adaptive foreground keeps the mark inside the central 66 % safe zone. Backgrounds are `#EFE9DC`, and `#0D0C0B` at night.
- **Small sizes:** at 64 px and below, the packs use the `smallsizes` cut, with heavier trails so the orbits survive. At 16 px the mark is the P with three coloured lights; at 32 px the hole and the orbits read.
