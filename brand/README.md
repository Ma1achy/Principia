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

## Videos and GIFs

Every animation is one seamless 36-second loop, rendered frame by frame in Chrome: 30 fps for the titles, 24 fps for the icons.

- **WebM:** has a real transparent background, so the glow fades into whatever is behind it. Use this on the web: `<video autoplay loop muted playsinline>`.
- **MP4:** sits on paper (light versions) or near-black (dark versions), for anywhere that can't play WebM.
- **GIF:** also on its background, because GIF transparency is all-or-nothing and can't carry a soft glow. To keep files manageable, they're smaller and slower: titles are 800 px wide at 15 fps, and icons are 400 px at 12 fps.

## Platform notes

- **macOS:** Pillow's `.icns` writer leaves out a couple of the smallest 1× sizes. For a complete file, run `iconutil -c icns Principia.iconset` on a Mac; the iconsets are included.
- **iOS:** the light icon is opaque, as Apple requires. The dark-appearance icon is transparent, so iOS draws its own dark background behind it.
- **Android:** the adaptive foreground keeps the mark inside the central 66 % safe zone. Backgrounds are `#EFE9DC`, and `#0D0C0B` at night.
- **Small sizes:** at 64 px and below, the packs use the `smallsizes` cut, with heavier trails so the orbits survive. At 16 px the mark is the P with three coloured lights; at 32 px the hole and the orbits read.

## How it was made (`code/python/`)

The paths in these scripts point at the working folders I used, so adjust them before re-running.

1. `trace_p.py`: traces the P, the full wordmark and the subtitle from `docs/readme/title-light.svg` into polygons (96 points per curve for the P).
2. `sim.py` and `cat.py`: integrate the periodic orbits (figure-eight, Butterfly I to III, Moth I to III, Goggles, Dragonfly, Yin-yang, Bumblebee) and Burrau's 3-4-5 problem.
3. `sorb.py`, `hole*.py` and `dragon*.py`: search each orbit's placement in 3D, so that it threads the hole in the P and never touches the solid letter, allowing margin for body size and line width. Dragonfly is checked at 120,000 time steps per period, with zero contact and 9 passes through the hole per loop. `dragon.py` then resamples the paths adaptively (a new point whenever the path turns 2.5°), so the trails stay smooth through close encounters.
4. `final.py` and `wordmarks.py`: build the renderers. The P is extruded and shaded by a key light plus the coloured light of each body, and drawn back to front with the bodies passing behind and in front. Trails are tapered neon ribbons, split exactly where they cross the P's plane. Bloom and halation come from one stacked-blur filter, and glow from behind the P is masked by the letter's silhouette.
5. `build_runtime.py`: turns the boards into the standalone JavaScript in `code/web/`.
6. `render_masters.py`, `packs.py` and `frames.py`: render stills and title frames in headless Chrome, then build the platform packs and videos.
7. `js/verify.js`: confirms that no point of any trail or body is ever inside the solid P.

