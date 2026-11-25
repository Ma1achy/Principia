import asyncio, sys, os
from playwright.async_api import async_playwright
v=sys.argv[1]; fps=int(sys.argv[2]); T=36.0; out=f'/home/claude/pack/frames/{v}'; os.makedirs(out,exist_ok=True)
start=int(sys.argv[3]) if len(sys.argv)>3 else 0
async def main():
    async with async_playwright() as p:
        b=await p.chromium.launch(executable_path='/usr/bin/google-chrome')
        pg=await b.new_page(viewport={'width':1600,'height':700})
        await pg.goto(f'file:///tmp/render/r.html?v={v}&t=0&size=640')
        await pg.wait_for_function("document.title==='ready'")
        await pg.evaluate("window.__m = PrincipiaMark[new URLSearchParams(location.search).get('v')].mount(document.getElementById('m').cloneNode(), {})")
        await pg.evaluate("document.getElementById('m').innerHTML=''; window.__mk = PrincipiaMark[new URLSearchParams(location.search).get('v')].mount(document.getElementById('m'), {still:true, frame:0, transparent:true, backdrop:'transparent'})")
        el=await pg.query_selector('#m'); n=int(T*fps)
        for i in range(start,n):
            fn=f'{out}/f{i:05d}.png'
            if os.path.exists(fn): continue
            await pg.evaluate(f"window.__mk.seek({6 + i/fps})")
            await el.screenshot(path=fn, omit_background=True)
        await b.close()
asyncio.run(main())
