import asyncio, sys, numpy as np
from PIL import Image
from playwright.async_api import async_playwright
PAGE=sys.argv[1] if len(sys.argv)>1 else '/mnt/user-data/outputs/principia_poster_both_sides.html'
async def render():
    async with async_playwright() as p:
        br=await p.chromium.launch(); pg=await br.new_page(viewport={'width':1440,'height':900},device_scale_factor=2)
        errs=[]; pg.on('pageerror',lambda e:errs.append(str(e)[:160]))
        await pg.goto('file://'+PAGE); await pg.wait_for_timeout(3500)
        await pg.mouse.move(1439,899)
        await pg.screenshot(path='/tmp/gold/front_render.png',clip={'x':0,'y':0,'width':1440,'height':900})
        await pg.evaluate("()=>{document.querySelector('.face.front').hidden=true;document.querySelector('.face.back').hidden=false;window.dispatchEvent(new Event('resize'))}")
        await pg.wait_for_timeout(1500)
        await pg.screenshot(path='/tmp/gold/back_render.png',clip={'x':0,'y':0,'width':1440,'height':900})
        await br.close(); return errs
def compare(name):
    g=np.asarray(Image.open('/tmp/gold/%s_golden.png'%name).convert('RGB')).astype(float)
    r=np.asarray(Image.open('/tmp/gold/%s_render.png'%name).convert('RGB').resize((g.shape[1],g.shape[0]))).astype(float)
    d=np.abs(g-r).mean(-1)
    # score in 1x-page blocks of 40 px, so the heat map says where the drift is
    B=80; H,W=d.shape; heat=d[:H//B*B,:W//B*B].reshape(H//B,B,W//B,B).mean((1,3))
    vis=np.clip(d*4,0,255).astype(np.uint8); Image.fromarray(vis).resize((1440,900)).save('/tmp/gold/%s_diff.png'%name)
    side=Image.new('RGB',(2880,900)); side.paste(Image.fromarray(g.astype(np.uint8)).resize((1440,900)),(0,0)); side.paste(Image.fromarray(r.astype(np.uint8)).resize((1440,900)),(1440,0)); side.save('/tmp/gold/%s_side.png'%name)
    print('%s: mean |diff| %.2f /255, pixels off by >24: %.1f%%, worst blocks:'%(name,d.mean(),(d>24).mean()*100),
          sorted([(round(heat[i,j],1),(j*40,i*40)) for i in range(heat.shape[0]) for j in range(heat.shape[1])],reverse=True)[:6])
if __name__=='__main__':
    errs=asyncio.run(render()); print('page errors:',errs or 'none')
    compare('front'); compare('back')
