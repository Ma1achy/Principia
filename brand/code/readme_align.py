#!/usr/bin/env python3
"""Measures whether the README's title lines up with the banner below it.

    python3 readme_align.py OUT_DIR [--root REPO] [--widths 700,1012,1280] [--times 4] [--hero-width 100%]

The README's picture block is rendered in Chromium at each width the way GitHub lays it out, on GitHub's page
colours, and in each variant it can show:
  dark   the animated title on #0d1117       light  the animated title on #ffffff
  still-dark / still-light  the reduced-motion title (a still SVG)
For each render it finds, in the rendered pixels, the x of the P's stem and the x of the right end of "Planar
Three-Body Problem", and the left and right edges of the banner (from the page's layout), and reports

    dL = x(P stem) - banner left        dR = banner right - x(subtitle end)       (px)

Criterion, written before the first run: |dL| <= 1 and |dR| <= 1 at 1012 px wide in every variant and every
sampled frame. The 700 and 1280 px renders are reported with the same limit. An edge is where the pixel colour
crosses half way between the page colour and the letter colour (found along rows, with the position inside the
pixel interpolated), so a half pixel of anti-aliasing is the measurement's own error. A frame in which the
glow covers the stem's edge in too many rows to read it is counted as "unreadable" and reported, never skipped silently.
--hero-width replaces the banner's width attribute, which is how the controls render the old full-width layout.
"""
import argparse, asyncio, os, re, sys
import numpy as np
from PIL import Image
from playwright.async_api import async_playwright

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
LIMIT = 1.0
VARIANTS = [  # name, colour scheme, reduced motion, page colour, letter colour, still?
    ('dark', 'dark', False, (0x0d, 0x11, 0x17), (0xef, 0xe9, 0xdc), False),
    ('light', 'light', False, (255, 255, 255), (0x1d, 0x1b, 0x19), False),
    ('still-dark', 'dark', True, (0x0d, 0x11, 0x17), (0xef, 0xe9, 0xdc), True),
    ('still-light', 'light', True, (255, 255, 255), (0x1d, 0x1b, 0x19), True)]


def dist(row, bg):
    return np.abs(row.astype(float) - np.array(bg, float)).max(axis=1)


def edge(row, bg, ref, side):
    """Position of the half-way crossing along one row: first from the left ('L') or last from the right ('R').
    Returns None unless the crossing is a steep step (a glyph edge, not a soft glow)."""
    d = dist(row, bg) / max(dist(np.array([ref]), bg)[0], 1)
    idx = np.where(d >= 0.5)[0]
    if not len(idx):
        return None
    if side == 'L':
        i = idx[0]
        if i < 2 or i + 1 >= len(d) or d[i - 2] > 0.3 or d[i + 1] < 0.7:
            return None
        return i - 1 + (0.5 - d[i - 1]) / (d[i] - d[i - 1]) + 0.5
    i = idx[-1]
    if i < 1 or i + 2 >= len(d) or d[i + 2] > 0.3 or d[i - 1] < 0.7:
        return None
    return i + (d[i] - 0.5) / (d[i] - d[i + 1]) + 0.5


def measure(img, bg, ink, still):
    """x of the P's stem and of the subtitle's end, in the picture's own pixels (None if unreadable).
    The stem is read along rows (a steep step, half way between page and letter colour). The subtitle's end is read
    from the columns: coverage of a column = its strongest pixel against the subtitle's own colour, the end is the
    last column with coverage of at least a quarter, plus that coverage as the part of the pixel the glyph fills.
    (Thin strokes, the tail of a y, never reach full coverage at small sizes, so a half-way rule would skip them.)"""
    a = np.asarray(img.convert('RGB'))
    h, w = a.shape[:2]
    rows = range(int(h * (0.50 if still else 0.60)), int(h * (0.66 if still else 0.72)))
    left = [e for e in (edge(a[y, :w // 4], bg, ink, 'L') for y in rows) if e is not None]
    x0 = int(w * 0.75)
    reg = a[:, x0:]
    ref = subtitle_ref(reg, bg)
    cov = np.clip(np.abs(reg.astype(float) - np.array(bg, float)).max(axis=2) / max(abs(np.array(ref, float) - np.array(bg, float)).max(), 1), 0, 1).max(axis=0)
    idx = np.where(cov >= 0.25)[0]
    xp = float(np.median(left)) if len(left) >= 3 else None
    xs = float(idx[-1] + min(1.0, cov[idx[-1]]) + x0) if len(idx) else None
    return xp, xs


def subtitle_ref(region, bg):
    """The subtitle's colour: the pixel in the region furthest from the page colour."""
    d = np.abs(region.astype(float) - np.array(bg, float)).max(axis=2)
    y, x = np.unravel_index(d.argmax(), d.shape)
    return tuple(region[y, x])


async def run(out, root, widths, times, hero_width):
    txt = open(os.path.join(root, 'README.md')).read()
    block = txt.split('\n---')[0]
    if hero_width:
        block = re.sub(r'(<img src="docs/readme/hero\.jpg"[^>]*?)\s+width="[^"]*"', rf'\1 width="{hero_width}"', block)
    os.makedirs(out, exist_ok=True)
    rows = []
    async with async_playwright() as p:
        b = await p.chromium.launch()
        for width in widths:
            for name, scheme, reduced, bg, ink, still in VARIANTS:
                ctx = await b.new_context(viewport={'width': width, 'height': 700}, color_scheme=scheme,
                                          reduced_motion='reduce' if reduced else 'no-preference')
                pg = await ctx.new_page()
                hp = os.path.join(out, f'_page-{width}-{name}.html')
                bgc = '#%02x%02x%02x' % bg
                open(hp, 'w').write(f'<!doctype html><base href="file://{root}/"><body style="margin:0;background:{bgc}">'
                                    f'<div style="width:{width}px">{block}</div>')
                await pg.goto('file://' + os.path.abspath(hp))
                await pg.wait_for_function("[...document.images].every(i => i.complete && i.naturalWidth)")
                title = await pg.query_selector('picture img')
                hero = await pg.query_selector('img[src$="hero.jpg"]')
                hb = await hero.bounding_box()
                for k in range(1 if still else times):
                    await pg.wait_for_timeout(0 if still else 1500)
                    tb = await title.bounding_box()
                    shot = os.path.join(out, f'align-{width}-{name}-{k}.png')
                    await pg.screenshot(path=shot, clip={'x': tb['x'], 'y': tb['y'], 'width': tb['width'], 'height': tb['height']})
                    img = Image.open(shot)
                    xp, xs = measure(img, bg, ink, still)
                    if xp is None or xs is None:
                        rows.append((width, name, k, None, None, 'unreadable'))
                        continue
                    sx = tb['width'] / img.width                          # screenshot pixels to CSS pixels
                    dl = tb['x'] + xp * sx - hb['x']
                    dr = (hb['x'] + hb['width']) - (tb['x'] + xs * sx)
                    rows.append((width, name, k, dl, dr, ''))
                await ctx.close()
        await b.close()
    return rows


def main(argv=None):
    ap = argparse.ArgumentParser()
    ap.add_argument('out'); ap.add_argument('--root', default=REPO)
    ap.add_argument('--widths', default='700,1012,1280'); ap.add_argument('--times', type=int, default=4)
    ap.add_argument('--hero-width')
    a = ap.parse_args(argv)
    rows = asyncio.run(run(a.out, os.path.abspath(a.root), [int(w) for w in a.widths.split(',')], a.times, a.hero_width))
    bad = 0
    print(f'{"width":>5} {"variant":<12} {"sample":>6} {"dL (px)":>9} {"dR (px)":>9}')
    for w, n, k, dl, dr, why in rows:
        if dl is None:
            print(f'{w:>5} {n:<12} {k:>6} {why}'); continue
        fail = abs(dl) > LIMIT or abs(dr) > LIMIT
        bad += fail
        print(f'{w:>5} {n:<12} {k:>6} {dl:>9.2f} {dr:>9.2f}{"   <-- OUT" if fail else ""}')
    meas = [(w, r) for w, *r in rows if r[2] is not None]
    unread = [r for r in rows if r[3] is None]
    print(f'\n{len(rows) - len(unread)} readings, {len(unread)} unreadable, {bad} outside {LIMIT} px; '
          f'worst |dL| {max(abs(r[2]) for w, r in meas):.2f}, worst |dR| {max(abs(r[3]) for w, r in meas):.2f}' if meas else 'no readings')
    return 1 if bad or unread or not meas else 0


if __name__ == '__main__':
    sys.exit(main())
