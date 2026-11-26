# usage: packs.py MASTERS_DIR ICONS_DIR   (writes only the files it makes; never deletes anything)
import os, sys, json
from PIL import Image
M,R=sys.argv[1],sys.argv[2]
BG={'light':'#efe9dc','dark':'#0d0c0b'}
src={v:Image.open(f'{M}/icon-{v}-1024.png').convert('RGBA') for v in ('light','dark')}
small={v:Image.open(f'{M}/icon-{v}-small-1024.png').convert('RGBA') for v in ('light','dark')}
def at(v,px,opaque=False,pad=0.0):
    im=(small if px<=64 else src)[v]
    if pad:
        inner=round(1024*(1-pad)); c=Image.new('RGBA',(1024,1024),(0,0,0,0)); c.alpha_composite(im.resize((inner,inner),Image.LANCZOS),((1024-inner)//2,)*2); im=c
    out=im.resize((px,px),Image.LANCZOS)
    if opaque:
        b=Image.new('RGBA',(px,px),BG[v]); b.alpha_composite(out); out=b.convert('RGB')
    return out
def save(img,path):
    os.makedirs(os.path.dirname(path),exist_ok=True); img.save(path); return path
made=[]
for v in ('light','dark'):
    sfx='' if v=='light' else '-dark'
    # web
    W=f'{R}/web'
    for px in (16,32,48,96): made.append(save(at(v,px),f'{W}/favicon{sfx}-{px}.png'))
    at(v,64).save(f'{W}/favicon{sfx}.ico',sizes=[(16,16),(32,32),(48,48),(64,64)]); made.append(f'{W}/favicon{sfx}.ico')
    made.append(save(at(v,180,opaque=True),f'{W}/apple-touch-icon{sfx}.png'))
    for px in (192,512): made.append(save(at(v,px),f'{W}/icon{sfx}-{px}.png'))
    made.append(save(at(v,512,opaque=True,pad=0.2),f'{W}/icon{sfx}-maskable-512.png'))
    # macOS
    iconset=f'{R}/macos/Principia{sfx}.iconset'
    for base in (16,32,128,256,512):
        made.append(save(at(v,base),f'{iconset}/icon_{base}x{base}.png'))
        made.append(save(at(v,base*2),f'{iconset}/icon_{base}x{base}@2x.png'))
    at(v,1024).save(f'{R}/macos/Principia{sfx}.icns'); made.append(f'{R}/macos/Principia{sfx}.icns')
    # Windows
    os.makedirs(f'{R}/windows',exist_ok=True)
    big=at(v,256); big.save(f'{R}/windows/Principia{sfx}.ico',sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)]); made.append(f'{R}/windows/Principia{sfx}.ico')
    for px in (44,150,310): made.append(save(at(v,px),f'{R}/windows/tiles/Square{px}x{px}Logo{sfx}.png'))
    # Linux (hicolor)
    for px in (16,22,24,32,48,64,128,256,512):
        made.append(save(at(v,px),f'{R}/linux/hicolor{sfx}/{px}x{px}/apps/principia.png'))
    # Android: legacy launcher + adaptive foreground (safe zone = inner 66%)
    for d,px in (('mdpi',48),('hdpi',72),('xhdpi',96),('xxhdpi',144),('xxxhdpi',192)):
        made.append(save(at(v,px,opaque=True),f'{R}/android/res/mipmap-{d}{"-night" if v=="dark" else ""}/ic_launcher.png'))
        made.append(save(at(v,round(px*108/48),pad=0.34),f'{R}/android/res/mipmap-{d}{"-night" if v=="dark" else ""}/ic_launcher_foreground.png'))
    # masters
    made.append(save(src[v],f'{R}/source/principia-icon{sfx}-1024-transparent.png'))
    made.append(save(at(v,1024,opaque=True),f'{R}/source/principia-icon{sfx}-1024-on-background.png'))
    made.append(save(Image.open(f'{M}/icon-{v}-2048.png'),f'{R}/source/principia-icon{sfx}-2048-uncropped.png'))
    made.append(save(small[v],f'{R}/source/principia-icon{sfx}-smallsizes-1024.png'))
# iOS: light icon is opaque, dark-appearance icon is transparent (iOS 18+)
made.append(save(at('light',1024,opaque=True),f'{R}/ios/AppIcon.appiconset/AppIcon-1024.png'))
made.append(save(at('dark',1024),f'{R}/ios/AppIcon.appiconset/AppIcon-1024-dark.png'))
json.dump({"images":[{"filename":"AppIcon-1024.png","idiom":"universal","platform":"ios","size":"1024x1024"},
  {"appearances":[{"appearance":"luminosity","value":"dark"}],"filename":"AppIcon-1024-dark.png","idiom":"universal","platform":"ios","size":"1024x1024"}],
  "info":{"author":"xcode","version":1}},open(f'{R}/ios/AppIcon.appiconset/Contents.json','w'),indent=2)
for v in ('', '-night'):
    os.makedirs(f'{R}/android/res/mipmap-anydpi-v26{v}',exist_ok=True)
    open(f'{R}/android/res/mipmap-anydpi-v26{v}/ic_launcher.xml','w').write('<?xml version="1.0" encoding="utf-8"?>\n<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">\n  <background android:drawable="@color/ic_launcher_background"/>\n  <foreground android:drawable="@mipmap/ic_launcher_foreground"/>\n</adaptive-icon>\n')
os.makedirs(f'{R}/android/res/values',exist_ok=True); os.makedirs(f'{R}/android/res/values-night',exist_ok=True)
open(f'{R}/android/res/values/ic_launcher_background.xml','w').write('<?xml version="1.0" encoding="utf-8"?>\n<resources><color name="ic_launcher_background">#EFE9DC</color></resources>\n')
open(f'{R}/android/res/values-night/ic_launcher_background.xml','w').write('<?xml version="1.0" encoding="utf-8"?>\n<resources><color name="ic_launcher_background">#0D0C0B</color></resources>\n')
json.dump({"name":"Principia","short_name":"Principia","icons":[
  {"src":"icon-192.png","sizes":"192x192","type":"image/png"},{"src":"icon-512.png","sizes":"512x512","type":"image/png"},
  {"src":"icon-maskable-512.png","sizes":"512x512","type":"image/png","purpose":"maskable"}],
  "theme_color":"#0d0c0b","background_color":"#0d0c0b","display":"standalone"},open(f'{R}/web/site.webmanifest','w'),indent=2)
open(f'{R}/web/head-snippet.html','w').write('''<!-- Principia icons: static favicons that follow the light/dark theme -->
<link rel="icon" href="/favicon.ico" sizes="any" media="(prefers-color-scheme: light)">
<link rel="icon" href="/favicon-dark.ico" sizes="any" media="(prefers-color-scheme: dark)">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png" media="(prefers-color-scheme: light)">
<link rel="icon" type="image/png" sizes="32x32" href="/favicon-dark-32.png" media="(prefers-color-scheme: dark)">
<link rel="apple-touch-icon" href="/apple-touch-icon-dark.png">
<link rel="manifest" href="/site.webmanifest">
<meta name="theme-color" content="#efe9dc" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0d0c0b" media="(prefers-color-scheme: dark)">
<!-- Optional: animate the tab icon (Chrome, Edge, Firefox). See code/web/favicon-animated.js -->
''')
print(len(made),'files')
