#!/usr/bin/env python3
"""Renders the repository README's title block in Chromium the way GitHub lays it out (1012 px wide) on GitHub's
page colours, and measures the seam: how far the outer rows and columns of the picture are from the page colour.

    python3 readme_render.py OUT_DIR [--readme README.md] [--times 3,12,24]

Writes readme-<scheme>-<page colour>-t<seconds>.png for dark #0d1117, dark dimmed #212830, light #ffffff, each with
the matching prefers-color-scheme (and one with prefers-reduced-motion), and prints the worst deviation of the
picture's 4 px border from the page colour in each render (0 means no box).
"""
import argparse, asyncio, os, re, sys
import numpy as np
from PIL import Image
from playwright.async_api import async_playwright

REPO = os.environ.get('README_ROOT') or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PAGES = [('dark', '0d1117'), ('dark', '212830'), ('light', 'ffffff')]


async def main(out, readme, times, transparent=False):
    txt = open(readme).read()
    block = re.search(r'<p>\s*(<picture>.*?</picture>)', txt, re.S).group(1)
    if transparent:                       # the same block with the transparent WebPs instead of the page-colour ones
        block = block.replace('subtitle-github-', 'subtitle-')
    os.makedirs(out, exist_ok=True)
    rows = []
    async with async_playwright() as p:
        b = await p.chromium.launch()
        for scheme, bg in PAGES + [('dark', '0d1117')]:
            reduced = len(rows) >= len(PAGES) * len(times)
            for t in ([0] if reduced else times):
                ctx = await b.new_context(viewport={'width': 1012, 'height': 420}, color_scheme=scheme,
                                          reduced_motion='reduce' if reduced else 'no-preference')
                pg = await ctx.new_page()
                html = f'<!doctype html><base href="file://{REPO}/"><body style="margin:0;padding:24px 0;background:#{bg}"><div id="r" style="width:1012px">{block}</div>'
                html = html.replace('width="100%"', 'width="100%" style="display:block"')
                hp = os.path.join(out, '_page.html')
                open(hp, 'w').write(html)
                await pg.goto('file://' + os.path.abspath(hp))
                await pg.wait_for_function("[...document.images].every(i => i.complete && i.naturalWidth)")
                await pg.wait_for_timeout(int(t * 1000) + 200)        # the animation runs: the picture is what shows at that moment
                name = f"readme-{'transparent-' if transparent else ''}{'reduced-motion-' if reduced else ''}{scheme}-{bg}-t{t}.png"
                el = await pg.query_selector('img')
                bb = await el.bounding_box()
                m = 24                                        # page colour around the picture, so a box shows as a box
                await pg.screenshot(path=os.path.join(out, name), clip={'x': 0, 'y': max(0, bb['y'] - m), 'width': bb['width'], 'height': bb['height'] + 2 * m})
                a = np.asarray(Image.open(os.path.join(out, name)).convert('RGB')).astype(int)[m:m + round(bb['height']), :]
                c = np.array([int(bg[i:i + 2], 16) for i in (0, 2, 4)])
                e = 4
                edge = np.concatenate([a[:e].reshape(-1, 3), a[-e:].reshape(-1, 3), a[:, :e].reshape(-1, 3), a[:, -e:].reshape(-1, 3)])
                rows.append((name, int(np.abs(edge - c).max()), a.shape[1], a.shape[0]))
                print(f'{name:<50} {a.shape[1]}x{a.shape[0]}  worst border deviation from #{bg}: {rows[-1][1]}')
                await ctx.close()
        await b.close()
    return rows


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('out'); ap.add_argument('--readme', default=os.path.join(REPO, 'README.md'))
    ap.add_argument('--times', default='3,12')
    ap.add_argument('--transparent', action='store_true', help='use the transparent WebPs instead of the page-colour ones')
    a = ap.parse_args()
    asyncio.run(main(a.out, a.readme, [float(x) for x in a.times.split(',')], a.transparent))
