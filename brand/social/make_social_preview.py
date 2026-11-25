"""Render Principia's social preview (1280 x 640, GitHub's recommended size) in headless Chrome.

The README hero banner (docs/readme/hero.jpg) fills the frame, tinted dark or light so the title reads, with a still frame of the
animated 2D title (and its subtitle, from brand/code/web/) centred on top. Writes out/social-preview-{dark,light}.{png,jpg}.

    pip install playwright pillow && python -m playwright install chromium
    python make_social_preview.py            # or: --frame 20 to pick another moment of the 36 s loop
"""
import argparse, asyncio, pathlib
from PIL import Image
from playwright.async_api import async_playwright

HERE = pathlib.Path(__file__).resolve().parent
OUT = HERE / 'out'
HERO = HERE.parent.parent / 'docs' / 'readme' / 'hero.jpg'   # the README banner
WEB = HERE.parent / 'code' / 'web'                            # the self-contained title modules
W, H = 1280, 640
TITLE_WIDTH = 1180  # px across the frame; the title's viewBox keeps its proportions

# the tint knocks the busy pencil banner back so the title reads; stronger at the edges than the middle
VARIANTS = {
    'dark':  ('title-2d-subtitle-dark',  'linear-gradient(90deg, rgba(13,12,11,0.86), rgba(13,12,11,0.72) 55%, rgba(13,12,11,0.82))'),
    'light': ('title-2d-subtitle-light', 'linear-gradient(90deg, rgba(239,233,220,0.80), rgba(239,233,220,0.86) 55%, rgba(239,233,220,0.80))'),
}


def page(variant, tint, frame):
    return f'''<!doctype html><html><head><meta charset="utf-8"><style>
html,body{{margin:0;width:{W}px;height:{H}px;overflow:hidden}}
.bg{{position:absolute;inset:0;background:url({HERO.as_uri()}) center/cover}}
.tint{{position:absolute;inset:0;background:{tint}}}
#m{{position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);line-height:0}}
#m svg{{width:{TITLE_WIDTH}px;height:auto}}
</style></head><body><div class="bg"></div><div class="tint"></div><div id="m"></div>
<script src="{(WEB / f'principia-{variant}.js').as_uri()}"></script>
<script>
  const el = document.getElementById('m');
  el.innerHTML = PrincipiaMark['{variant}'].svgAt({frame}, {{ transparent: true, backdrop: 'transparent' }});
  // svgAt wraps the SVG in a fixed-size box; take the SVG out so it scales and centres
  const svg = el.querySelector('svg'); el.replaceChildren(svg); svg.removeAttribute('width'); svg.removeAttribute('height');
  document.title = 'ready';
</script></body></html>'''


async def main(frame, chrome):
    OUT.mkdir(exist_ok=True)
    async with async_playwright() as p:
        browser = await p.chromium.launch(**({'executable_path': chrome} if chrome else {}))
        pg = await browser.new_page(viewport={'width': W, 'height': H})
        for name, (variant, tint) in VARIANTS.items():
            html = HERE / f'_{name}.html'
            html.write_text(page(variant, tint, frame))
            await pg.goto(html.as_uri())
            await pg.wait_for_function("document.title === 'ready'")
            await pg.wait_for_timeout(300)
            png = OUT / f'social-preview-{name}.png'
            await pg.screenshot(path=str(png))
            Image.open(png).convert('RGB').save(OUT / f'social-preview-{name}.jpg', quality=90, optimize=True)
            html.unlink()
            print(png.name, 'and', png.with_suffix('.jpg').name)
        await browser.close()


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('--frame', type=float, default=14, help='moment of the 36 s loop to freeze, in seconds')
    ap.add_argument('--chrome', help='path to a Chrome binary, if not using Playwright\'s own')
    a = ap.parse_args()
    asyncio.run(main(a.frame, a.chrome))
