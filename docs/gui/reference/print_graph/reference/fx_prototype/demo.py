import sys, os, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import graph, nodes, biggrid as B, media2 as M2
from PIL import Image, ImageDraw, ImageFont
F = B.F; S = 300
hd = ImageFont.truetype(F+'Heros-700-normal.otf', 18); sm = ImageFont.truetype(F+'Heros-400-normal.otf', 15); big = ImageFont.truetype(F+'Heros-700-normal.otf', 30)
def U(x): x = np.asarray(x); return x if x.dtype == np.uint8 else (np.clip(x, 0, 1)*255).astype(np.uint8)
rows = []
# 1. chains
srcs = {'b': B.load('b'), 'a': B.load('a'), 'g': B.load('g'), 'e': B.load('e'), 'f': B.load('f')}
chains = [('demo_light_halftone', 'a', 'Light painting\n→ halftone'), ('demo_oil_crt', 'f', 'Oil\n→ CRT'), ('demo_acrylic_gameboy', 'g', 'Acrylic\n→ Game Boy'),
          ('demo_riso_wcp', 'b', 'Watercolour pencil (limited)\n→ risograph'), ('demo_negative_of_print', 'e', 'Offset press\n→ film negative'), ('demo_stipple_polaroid', 'g', 'Stipple\n→ Polaroid')]
row = []
for name, k, lab in chains:
    row.append((U(graph.render(name, np.asarray(srcs[k]), seed=1)), lab))
rows.append(('Presets feeding presets: one line of JSON each', row))
# 2. data binding: colour from one render, the field from another
colour = np.asarray(srcs['b']); fieldimg = np.asarray(srcs['g']).astype(float)/255
life = M2.stretch(M2.blur(fieldimg.mean(-1), 2.0))
row = [(colour, 'Colour: config stability'), (U(np.stack([life]*3, -1)), 'A field: "lifetime"\n(three-body (12), as a stand-in)'),
       (U(graph.render('relief_of_lifetime', colour, seed=1)), 'Relief of lifetime,\nproxy (no data)'),
       (U(graph.render('relief_of_lifetime', colour, data={'lifetime': life}, seed=1)), 'Relief of lifetime,\ndata.lifetime bound')]
depth = 4 + 40*life
row += [(U(graph.render('acrylic', colour, seed=1)), 'Acrylic,\ndepth 22 everywhere'),
        (U(graph.render('demo_acrylic_depth_from_data', colour, data={'depth': depth}, seed=1)), 'Acrylic,\ndepth bound to the field')]
rows.append(('Parameters bound to data: the same preset, a different field', row))
W = 40 + 6*(S+10); H = 70 + len(rows)*(S+110)
sheet = Image.new('RGB', (W, H), (239, 237, 229)); d = ImageDraw.Draw(sheet)
d.text((20, 14), 'Compositional post-process prototype', font=big, fill=(29, 27, 25))
y = 70
for title, row in rows:
    d.text((20, y), title, font=hd, fill=(29, 27, 25)); y += 30
    for j, (im, lab) in enumerate(row):
        sheet.paste(Image.fromarray(im).resize((S, S), Image.LANCZOS), (20+j*(S+10), y))
        d.multiline_text((20+j*(S+10), y+S+6), lab, font=sm, fill=(61, 58, 52), spacing=2)
    y += S+80
sheet.save('/home/claude/fx-compositions.jpg', quality=90); print(sheet.size)
