import sys, asyncio
from playwright.async_api import async_playwright
async def main(files):
    async with async_playwright() as p:
        b=await p.chromium.launch()
        pg=await b.new_page(viewport={'width':1600,'height':640})
        for f in files:
            await pg.goto('file://'+f); await pg.wait_for_timeout(150)
            await pg.screenshot(path=f.replace('.html','.png'))
        await b.close()
asyncio.run(main(sys.argv[1:]))
