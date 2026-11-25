import asyncio, os
from playwright.async_api import async_playwright
OUT='/home/claude/pack/masters'; os.makedirs(OUT,exist_ok=True)
jobs=[('icon-light','',1.0,'icon-light-2048.png'),('icon-dark','',1.0,'icon-dark-2048.png'),('icon-light','',1.6,'icon-light-small-2048.png'),('icon-dark','',1.6,'icon-dark-small-2048.png'),('icon-light','',1.0,'icon-light-1024.png'),('icon-dark','',1.0,'icon-dark-1024.png'),
      ('icon-light','',1.6,'icon-light-small-1024.png'),('icon-dark','',1.6,'icon-dark-small-1024.png')]
async def main():
    async with async_playwright() as p:
        b=await p.chromium.launch(executable_path='/usr/bin/google-chrome')
        pg=await b.new_page(viewport={'width':2048,'height':2048})
        for v,bd,w,fn in jobs:
            sz=2048 if '2048' in fn else 1024
            await pg.goto(f'file:///tmp/render/r.html?v={v}&t=17&size={sz}&weight={w}')
            await pg.wait_for_function("document.title==='ready'"); await pg.wait_for_timeout(300)
            await (await pg.query_selector('#m')).screenshot(path=f'{OUT}/{fn}', omit_background=True)
            print(fn)
        await b.close()
asyncio.run(main())
