"""Renders the mark in headless Chrome (the Chromium that playwright pins in requirements.txt).

The page is render/r.html; it loads principia-<variant>.js from the folder it sits in, so a render
folder holds r.html plus the generated code/web scripts (build.py makes it under <out>/work/render).
"""
import asyncio, os, shutil
from playwright.async_api import async_playwright

HERE = os.path.dirname(os.path.abspath(__file__))


def make_render_dir(wd, web_dir):
    """A folder with r.html next to a copy of the generated scripts."""
    os.makedirs(wd, exist_ok=True)
    shutil.copy(os.path.join(HERE, 'render', 'r.html'), wd)
    for f in os.listdir(web_dir):
        if f.endswith('.js'):
            shutil.copy(os.path.join(web_dir, f), wd)


async def _open(p, wd, v, size, t, weight, vw, vh, backdrop=None, pad=0):
    b = await p.chromium.launch()
    pg = await b.new_page(viewport={'width': vw, 'height': vh})
    q = f'v={v}&t={t}' + (f'&size={size}' if size else '') + (f'&weight={weight}' if weight else '') + (f'&backdrop=%23{backdrop.lstrip("#")}' if backdrop else '') + (f'&pad={pad}' if pad else '')
    await pg.goto(f'file://{os.path.abspath(wd)}/r.html?{q}')
    await pg.wait_for_function("document.title==='ready'")
    await pg.wait_for_timeout(300)
    return b, pg


async def _stills(wd, jobs):
    """jobs: [(variant, out_png, size, t, weight)]"""
    async with async_playwright() as p:
        for v, out, size, t, weight in jobs:
            b, pg = await _open(p, wd, v, size, t, weight, max(size, 1024), max(size, 1024))
            os.makedirs(os.path.dirname(out), exist_ok=True)
            await (await pg.query_selector('#m')).screenshot(path=out, omit_background=True)
            await b.close()


def stills(wd, jobs):
    asyncio.run(_stills(wd, jobs))


def still(wd, out, v, size=1024, t=17, weight=1):
    stills(wd, [(v, out, size, t, weight)])


async def _frames(wd, v, fps, outdir, loop=36.0, t0=6.0, only=None, backdrop=None, pad=0):
    """Frame i is the mark at t0 + i/fps seconds: the loop starts 6 s in. only: a set of frame numbers."""
    os.makedirs(outdir, exist_ok=True)
    n = int(round(loop * fps))
    async with async_playwright() as p:
        b, pg = await _open(p, wd, v, None, 0, None, 1700, 800, backdrop, pad)
        el = await pg.query_selector('#w' if pad else '#m')
        for i in range(n):
            if only is not None and i not in only:
                continue
            fn = f'{outdir}/f{i:05d}.png'
            if os.path.exists(fn):          # resumable
                continue
            await pg.evaluate(f'window.__seek({t0 + i / fps})')
            await el.screenshot(path=fn[:-4] + '.part.png', omit_background=True)
            os.replace(fn[:-4] + '.part.png', fn)
        await b.close()


def frames(wd, v, fps, outdir, only=None, backdrop=None, pad=0):
    """backdrop: a colour drawn behind the mark, so glow and ink mixing are computed on it.
    pad: pixels of backdrop colour added around the mark (the glow is clipped at the mark's own edge)."""
    asyncio.run(_frames(wd, v, fps, outdir, only=only, backdrop=backdrop, pad=pad))
