"""Writes the preset graphs as JSON. (Authoring convenience only: the JSON files are the presets.)"""
import json, os
OUT = os.path.join(os.path.dirname(__file__), 'presets')
def N(op, i=None, **p):
    d = {'op': op}
    if i: d['in'] = i
    if p: d['p'] = p
    return d
def X(f, **i): return {'op': 'expr', 'in': i, 'p': {'f': f}}
def save(name, title, doc, nodes, out, settings=None):
    spec = {'name': title, 'doc': doc, 'settings': dict(seed=0, **(settings or {})), 'nodes': nodes, 'out': out}
    json.dump(spec, open(os.path.join(OUT, name+'.json'), 'w'), indent=1, ensure_ascii=False)
S = '$seed'
def sd(k): return f'$seed+{k}'
RAK = [-0.55, -0.75, 0.9]

# ======================================================================= watercolour pencil (the core everything shared)
save('watercolour_pencil_core', 'Watercolour pencil (core)',
 'Median-cut washes that pool at their edges, pigment colour, Lu-Xu-Jia pencil strokes and tonal hatching, on paper. Returns the painting as floats.',
 {
  'K': N('count_colours', {'image': 'src'}, cap=12),
  'q': N('mediancut', {'image': 'src'}, K='@K', mode=3),
  'C0': N('fftblur', {'x': 'src'}, s=1.2),
  'C1': N('to_pigment', {'image': 'C0'}, enabled='$pigments'),
  'paper': N('wcp_paper'),
  'C': X('C - 0 if False else (lambda l: (l+(C-l)*sat)*0.93+paper[None,None,:]*0.07)(C.mean(-1,keepdims=True))', C='C1', paper='paper', sat='$sat'),
  'g': X('hypot(*gradient(C.mean(-1)))+hypot(*gradient(C[...,0]-C[...,2]))', C='C'),
  'Eb': N('fftblur', {'x': 'g'}, s=6),
  'E': X('clip(Eb/(percentile(Eb,90)+1e-9),0,1)', Eb='Eb'),
  'tex': N('wcp_texture'),
  'pool': N('pool', {'labels': 'q.labels', 'E': 'E'}),
  'r': N('rng', seed=S),
  'nz': N('draw', {'rng': 'r'}, kind='normal', shape='HW'),
  'nzb': N('fftblur', {'x': 'nz'}, s=40),
  'wash0': X('''absorb = 1-C/paper[None,None,:]
dens = pool*((1+0.10*z(nzb))*(1+0.10*tex*(1-0.5*E)))
out = paper[None,None,:]*clip(1-clip(dens*0.97,0,1.5)[...,None]*absorb,0,1)''', C='C', paper='paper', pool='pool', nzb='nzb', tex='tex', E='E'),
  'wash': N('fftblur', {'x': 'wash0'}, s=0.6),
  'Lm': X('I.mean(-1)', I='src'),
  'L': N('fftblur', {'x': 'Lm'}, s=1.0),
  'S0': N('pencil_strokes', {'x': 'L'}, ks=5),
  'P1': N('pencil_texture', angle=45, length=20, seed=S),
  'P2': N('pencil_texture', angle=-45, length=20, seed=sd(1)),
  'Jh': X('1-(1-(0.55+0.45*L))*(0.45*(1-0.75*E))', L='L', E='E'),
  'Jh2': X('clip(Jh+0.15,0,1)', Jh='Jh'),
  'T1': N('tone', {'P': 'P1', 'J': 'Jh'}),
  'T2': N('tone', {'P': 'P2', 'J': 'Jh2'}),
  'art': X('''S = 1-(1-S0)*(1-0.55*E)
J = 0.55+0.45*L
dark = clip((0.62-J)/0.25,0,1)*(1-E)
T = T1*(1-dark*(1-T2))
out = clip(wash*(0.16+0.84*S*T)[...,None],0,1)''', S0='S0', E='E', L='L', T1='T1', T2='T2', wash='wash'),
 }, 'art', dict(pigments=True, sat=0.95))

for name, title, pal in (('wcp', 'Watercolour pencil', 'wheel'), ('wcp_lim', 'Watercolour pencil, limited palette', 'limited')):
    save(name, title, f'The {pal} palette, then watercolour pencil.',
     {'pal': N('palette_mode', {'image': 'src'}, mode=pal),
      'paint': N('preset:watercolour_pencil_core', {'src': 'pal'}, seed=sd(3), pigments=False, sat=0.85),
      'out': N('u8', {'x': 'paint'})}, 'out')

# ======================================================================= paint
save('oil', 'Oil', 'Kuwahara-flattened colour dragged along the forms, low stroke ridges with bristle grooves, canvas in the thin passages, an aged varnish.', {
 'b': X('blur(I,0.8)', I='src'), 'k1': N('kuwahara', {'image': 'b'}, r=6), 'k2': N('kuwahara', {'image': 'k1'}, r=3),
 'f': N('flow', {'image': 'k2'}, sigma=6),
 'C': N('smear', {'x': 'k2', 'tx': 'f.tx', 'ty': 'f.ty'}, length=9),
 's1': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=2.6, density=0.004, length=20, seed=S),
 's2': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=1.6, density=0.006, length=12, seed=sd(1)),
 'r': N('rng', seed=sd(2)), 'n': N('draw', {'rng': 'r'}, kind='normal'),
 'ng': X('gaussian(n,0.6)', n='n.out'), 'gl': N('smear', {'x': 'ng', 'tx': 'f.tx', 'ty': 'f.ty'}, length=10),
 'cv': N('canvas', period=4.5, seed=S),
 'hgt': X('''strokes = maximum(s1, 0.8*s2)
grooves = z(gl)
h = strokes*(1+0.08*grooves)*(0.6+0.7*lum(C)) + 0.05*cv*clip(1-strokes, 0, 1)
out = gaussian(h, 0.7)''', s1='s1', s2='s2', gl='gl', C='C', cv='cv'),
 'li': N('light', {'height': 'hgt'}, strength=2.2, shin=36, L=RAK),
 'col': X('''out = C*(0.86+0.14*diff[..., None])
out = out + 0.08*spec[..., None]*array([1, .97, .9])
g = lum(out)[..., None]; out = g+(out-g)*1.1
out = out*array([1.0, 0.975, 0.91]); out = out*0.94+0.03''', C='C', diff='li.diff', spec='li.spec'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('acrylic', 'Acrylic, sculptural', 'Heavy-body acrylic whose brushwork follows the field: thick where bright, fine strokes in the filaments, broad where calm, lit low.', {
 'L': X('stretch(blur(0.5*lum(I) + 0.5*I.max(-1), 1.2))', I='src'),
 'E': N('edge_density', {'image': 'src'}, s=5),
 'f': N('field_flow', {'image': 'src', 'E': 'E'}),
 'fine': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=1.1, density=0.020, length=22, seed=S, amp_var=0.5),
 'broad': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=3.6, density=0.0035, length=34, seed=sd(1), amp_var=0.5),
 'fine2': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=1.1, density=0.012, length=18, seed=sd(2), amp_var=0.5),
 'broad2': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=3.6, density=0.002, length=28, seed=sd(3), amp_var=0.5),
 'r': N('rng', seed=sd(4)), 'n': N('draw', {'rng': 'r'}, kind='normal'),
 'ng': X('gaussian(n,0.5)', n='n.out'), 'br': N('smear', {'x': 'ng', 'tx': 'f.tx', 'ty': 'f.ty'}, length=14),
 'hgt': X('''wE = clip(E*1.5, 0, 1)
S = clip(wE*fine + (1-wE)*broad, 0, 1.6)
S2 = clip(wE*fine2 + (1-wE)*broad2, 0, 1.6)
st = maximum(S, 0.85*S2)
st = st/(1+st)*1.6
bristle = z(br)
Ls = gaussian(L, 1.5)
h = depth*(0.45*Ls + 0.55*Ls*st*(1+0.06*bristle)) + depth*0.12*st
out = gaussian(h, 0.8)''', E='E', fine='fine', broad='broad', fine2='fine2', broad2='broad2', br='br', L='L', depth='$depth'),
 'cb': X('blur(I,0.5)', I='src'), 'C': N('smear', {'x': 'cb', 'tx': 'f.tx', 'ty': 'f.ty'}, length=6),
 'li': N('light', {'height': 'hgt'}, strength=1.0, L=[-0.7, -0.55, 0.5], shin=22),
 'hs': X('hgt/depth*2.5', hgt='hgt', depth='$depth'),
 'sh': N('cast_shadow', {'height': 'hs'}, L=[-0.7, -0.55], reach=10, k=0.55),
 'col': X('''ao = clip(1-(gaussian(hgt, 5)-hgt)/(depth*0.5), 0.55, 1)
Ls = gaussian(L, 1.5)
out = C*(0.42+0.58*clip(diff, 0, 1.6)[..., None])*(sh*ao)[..., None] + 0.14*spec[..., None]*(0.4+0.6*Ls[..., None])''',
   hgt='hgt', depth='$depth', L='L', C='C', diff='li.diff', spec='li.spec', sh='sh'),
 'out': N('u8', {'x': 'col'})}, 'out', dict(depth=22.0))

save('gouache', 'Gouache', 'Flat, opaque, matte colour a shade lighter, ragged cut edges, dried rims, the ghost of the brush.', {
 'rg': N('regions', {'image': 'src'}, K=14, b=1.2, mode=5),
 'n1': N('noise', scale=6, octaves=2, seed=S), 'n2': N('noise', scale=6, octaves=2, seed=sd(1)),
 'dy': X('1.6*n', n='n1'), 'dx': X('1.6*n', n='n2'),
 'lab': N('warp', {'x': 'rg.labels', 'dy': 'dy', 'dx': 'dx'}, order=0),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'st': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=3.0, density=0.004, length=18, seed=sd(2)),
 'bd': N('boundary', {'labels': 'lab'}), 'pt': N('paper_tooth'),
 'col': X('''C = pal[lab]; g = lum(C)[..., None]; C = g+(C-g)*0.95; C = C*0.88+0.09
C = C*(1+0.035*z(st))[..., None]
rim = gaussian(bd.astype(float), 0.7)
C = C*(1-0.12*rim)[..., None]
out = C*(1+0.018*z(pt))[..., None]''', pal='rg.palette', lab='lab', st='st', bd='bd', pt='pt'),
 'out': N('u8', {'x': 'col'})}, 'out')

nodes = {'b': X('blur(I,0.6)', I='src'), 'C': N('kuwahara', {'image': 'b'}, r=3), 'f': N('flow', {'image': 'src'}, sigma=4)}
for k, (dens, ln, tw) in enumerate([(0.02, 5, 0.35), (0.03, 5, -0.25), (0.04, 4, 0.12)]):
    nodes[f'ax{k}'] = X(f'tx*cos({tw})-ty*sin({tw})', tx='f.tx', ty='f.ty')
    nodes[f'ay{k}'] = X(f'tx*sin({tw})+ty*cos({tw})', tx='f.tx', ty='f.ty')
    nodes[f'st{k}'] = N('strokes', {'tx': f'ax{k}', 'ty': f'ay{k}'}, width=0.6, density=dens, length=ln, seed=sd(k) if k else S, amp_var=0.4)
nodes['n7'] = N('noise', scale=9, octaves=3, seed=sd(7))
nodes['col'] = X('''G = array([0.96, 0.95, 0.91]); out = np.ones((H, W, 3))*G
for st, mix in ((st0, 0.45), (st1, 0.7), (st2, 0.92)):
    s = clip(st*2.2, 0, 1)
    col = G*(1-mix) + C*mix
    out = out*(1-s[..., None]) + col*s[..., None]
out = 0.55*out + 0.45*C
crack = (abs(n7) < 0.035).astype(float)
out = out*(1-0.12*gaussian(crack, 0.5))[..., None]
out = out*array([1.0, 0.99, 0.96])''', st0='st0', st1='st1', st2='st2', C='C', n7='n7')
nodes['out'] = N('u8', {'x': 'col'})
save('egg_tempera', 'Egg tempera', 'Thousands of tiny hatched strokes over gesso, pale layers first, a unifying glaze, faint craquelure.', nodes, 'out')

save('encaustic', 'Encaustic', 'Translucent wax layers, scraped back with a blade, fused with melted ripples and a waxy sheen.', {
 'under': X('blur(I,5)', I='src'), 'top': N('kuwahara', {'image': 'src'}, r=4),
 'n0': N('noise', scale=40, octaves=3, seed=S), 'n2': N('noise', scale=25, octaves=3, seed=sd(2)), 'n3': N('noise', scale=18, octaves=2, seed=sd(3)),
 'dx': X('np.full((H, W), cos(deg2rad(-12)))'), 'dy': X('np.full((H, W), sin(deg2rad(-12)))'),
 'scr0': N('strokes', {'tx': 'dx', 'ty': 'dy'}, width=1.0, density=0.0009, length=70, seed=sd(1), amp_var=0.3),
 'pre': X('''thick = sig(n0*2.2+0.8)
scr = clip(scr0*1.5, 0, 1)
out = gaussian(0.5*n2 + 0.35*thick - 0.6*scr + 0.08*sin(6*n3), 1.5)''', n0='n0', scr0='scr0', n2='n2', n3='n3'),
 'li': N('light', {'height': 'pre'}, strength=2.2, shin=7, L=RAK),
 'col': X('''thick = sig(n0*2.2+0.8)
out = under*(1-0.85*thick[..., None]) + top*(0.85*thick[..., None])
out = out*0.9 + array([0.95, 0.92, 0.82])*0.1
scr = clip(scr0*1.5, 0, 1)
out = out*(1-0.6*scr[..., None]) + under*0.6*scr[..., None]
out = out*(0.86+0.14*diff[..., None]) + 0.10*spec[..., None]
out = out*array([1.0, 0.98, 0.92])''', under='under', top='top', n0='n0', scr0='scr0', diff='li.diff', spec='li.spec'),
 'out': N('u8', {'x': 'col'})}, 'out')

# ======================================================================= drawing
save('graphite', 'Graphite', 'Hatching along the forms, crossed in the darks, broken by the tooth, burnished to silver.', {
 'L': X('stretch(blur(lum(I), 0.7))', I='src'),
 'f': N('flow', {'image': 'src'}, sigma=4),
 'a2': X('tx*cos(pi/4)-ty*sin(pi/4)', tx='f.tx', ty='f.ty'), 'b2': X('tx*sin(pi/4)+ty*cos(pi/4)', tx='f.tx', ty='f.ty'),
 'a3': X('ty', ty='f.ty'), 'b3': X('-tx', tx='f.tx'),
 'h1': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=0.7, density=0.012, length=18, seed=S, amp_var=0.5),
 'h2': N('strokes', {'tx': 'a2', 'ty': 'b2'}, width=0.7, density=0.012, length=18, seed=sd(1), amp_var=0.5),
 'h3': N('strokes', {'tx': 'a3', 'ty': 'b3'}, width=0.7, density=0.012, length=18, seed=sd(2), amp_var=0.5),
 'pt': N('paper_tooth'), 'fine': N('noise', scale=1.0, octaves=1, seed=sd(3)),
 'col': X('''T = (1-L)**1.1
h1 = clip(h1*1.8, 0, 1); h2 = clip(h2*1.8, 0, 1); h3 = clip(h3*1.8, 0, 1)
tooth = z(pt)
bite = clip(0.62+0.2*tooth+0.16*fine, 0, 1)
hatch = 1-(1-h1*clip(T*1.5, 0, 1)*0.8)*(1-h2*clip(T*1.9-0.55, 0, 1)*0.75)*(1-h3*clip(T*2.4-1.35, 0, 1)*0.75)
cov = 0.62*T**1.15 + 0.5*hatch
gx, gy = gradient(gaussian(L, 1.0)); edge = clip(hypot(gx, gy)*8-0.1, 0, 1)
cov = clip(cov*bite*1.05 + 0.45*edge*bite + 0.08*gaussian(cov, 4), 0, 0.93)
lead = array([0.17, 0.17, 0.19]); sheen = array([0.40, 0.41, 0.45])
burn = clip(cov*2.0-1.25, 0, 1)[..., None]
ink = lead*(1-burn*0.5)+sheen*burn*0.5
out = array(paper)*(1-cov[..., None]) + ink*cov[..., None]
out = out*(1+0.02*tooth)[..., None]''', L='L', h1='h1', h2='h2', h3='h3', pt='pt', fine='fine', paper='$paper'),
 'out': N('u8', {'x': 'col'})}, 'out', dict(paper=[0.95, 0.94, 0.91]))

save('charcoal', 'Charcoal', 'Smudged tone along the forms on a heavy tooth, highlights lifted with an eraser, loose dust.', {
 'r': N('rng', seed=S),
 'L': X('stretch(blur(lum(I), 0.8))', I='src'), 'f': N('flow', {'image': 'src'}, sigma=6),
 'streak': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=2.5, density=0.004, length=22, seed=S),
 'lift': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=1.6, density=0.004, length=14, seed=sd(5)),
 'pt': N('paper_tooth'), 'n1': N('noise', scale=1.3, octaves=1, seed=sd(1)),
 'dust': N('draw', {'rng': 'r'}, kind='random'),
 'col': X('''T = (1-L)**1.05
base = gaussian(T, 1.6)*(0.8+0.35*clip(streak, 0, 1.3))
tooth = 0.75*z(pt) + 0.5*n1
cov = clip(base*1.1, 0, 1)*sig(tooth*1.1 + (base-0.45)*6)
cov = 0.6*cov + 0.4*gaussian(cov, 3)
gx, gy = gradient(gaussian(L, 1.0)); edge = clip(hypot(gx, gy)*7-0.1, 0, 1)
cov = clip(cov + 0.35*edge*sig(tooth+1), 0, 1)
lf = clip(lift*1.6, 0, 1)
cov = cov*(1 - 0.85*lf*clip((L-0.5)*3, 0, 1))
d = gaussian((dust < 0.0012).astype(float), 0.6)*3
cov = clip(cov + d*0.5, 0, 0.96)
out = array([0.93, 0.92, 0.89])*(1-cov[..., None]) + array([0.08, 0.08, 0.09])*cov[..., None]
out = out*(1+0.02*z(pt))[..., None]''', L='L', streak='streak', lift='lift', pt='pt', n1='n1', dust='dust.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('chalk', 'Chalk pastel', 'Soft pastel on dark laid paper: side-of-the-stick strokes caught on the tooth, a dusty halo, finger-blended passages.', {
 'f': N('flow', {'image': 'src'}, sigma=7),
 'side': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=4.0, density=0.0025, length=22, seed=S),
 'n1': N('noise', scale=1.0, octaves=1, seed=sd(1)), 'n7': N('noise', scale=70, octaves=2, seed=sd(7)), 'pt': N('paper_tooth'),
 'col': X('''C = blur(I, 1.2); mx = C.max(-1); mn = C.min(-1); sat = (mx-mn)/(mx+1e-6)
pig = (C*0.72+0.28)*(0.85+0.3*sat[..., None])
amount = clip(maximum(lum(C)*1.1, mx*0.9)**0.9, 0, 1)
pressure = clip(amount*(0.75+0.45*clip(side, 0, 1.2)), 0, 1.2)
laid = 0.45*sin(2*pi*yy/3.2) + 0.3*n1 + 0.55*z(pt)
peaks = 1/(1+exp(-(laid+(pressure-0.5)*5.0)))
cover = clip(pressure*peaks*1.2, 0, 1)
dust = gaussian(cover[..., None]*pig, (3, 3, 0))*0.16
out = array([0.13, 0.135, 0.15])*(1-cover[..., None]) + pig*cover[..., None] + dust
smear = gaussian((n7 > 1.2).astype(float), 6)[..., None]
out = out*(1-smear) + blur(out, 3)*smear
out = out*(1+0.03*z(pt))[..., None]''', I='src', side='side', n1='n1', n7='n7', pt='pt'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('ink_wash', 'Ink wash', 'Sumi-e: one ink at four dilutions bleeding into rice-paper fibres, blooms, dry-brush breaks.', {
 'r': N('rng', seed=S), 'fr': N('draw', {'rng': 'r'}, kind='random'),
 'n1': N('noise', scale=120, octaves=2, seed=sd(1)),
 'fx': X('cos(pi*n)', n='n1'), 'fy': X('sin(pi*n)', n='n1'),
 'fd': X('(fr < 0.02).astype(float)', fr='fr.out'),
 'fs': N('smear', {'x': 'fd', 'tx': 'fx', 'ty': 'fy'}, length=9),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'dry': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=2.0, density=0.006, length=22, seed=sd(4)),
 'n2': N('noise', scale=30, octaves=3, seed=sd(2)), 'n3': N('noise', scale=7, octaves=2, seed=sd(3)),
 'col': X('''L = stretch(blur(lum(I), 1.2)); D = (1-L)**1.1
fibre = z(fs)
Dq = floor(clip(D*4 + 0.25*n2, 0, 3.999))/3.0
Dq = gaussian(Dq, 0.8)
bleed = gaussian(Dq, 3.5)*(1+0.18*fibre)
Dw = maximum(Dq, bleed*0.92)
edge = abs(Dq - gaussian(Dq, 2.5))
Dw = Dw + 0.35*clip(edge*3, 0, 1)*(Dq > 0.2)
bloom = clip(n3-1.1, 0, 1)*(abs(Dq-0.5) < 0.3)
Dw = Dw - 0.15*gaussian(bloom, 1.0)
dr = clip(dry*1.4, 0, 1)
Dw = where(D > 0.8, Dw*(0.55+0.45*dr), Dw)
Dw = clip(Dw*(1+0.06*fibre), 0, 1)
out = array([0.94, 0.92, 0.86])*(1-Dw[..., None]) + array([0.06, 0.06, 0.08])*Dw[..., None]
out = out*(1 + 0.025*fibre)[..., None]''', I='src', fs='fs', n2='n2', n3='n3', dry='dry'),
 'out': N('u8', {'x': 'col'})}, 'out')

# ======================================================================= printmaking
save('engraving', 'Engraving', 'Banknote line engraving: lines bending with the forms, swelling in the dark, crossed in the shadows, a platemark.', {
 'n0': N('noise', scale=120, octaves=2, seed=S), 'n1': N('noise', scale=90, octaves=2, seed=sd(1)),
 'plate': X('''plate = np.zeros((H, W)); m = int(0.035*min(H, W)); plate[m:H-m, m:W-m] = 1
out = gaussian(plate, 1.2)'''),
 'ph': X('-plate*2.5', plate='plate'),
 'li': N('light', {'height': 'ph'}, strength=1.0, L=RAK),
 'col': X('''L = stretch(blur(lum(I), 1.0)); T = (1-L)**0.9
warp = 22*gaussian(L, 4) + 3*n0
p = 5.2
def lines(ang, width):
    a = deg2rad(ang); s = (xx*cos(a)+yy*sin(a) + warp)/p
    d = abs((s % 1) - 0.5)*2*p/2
    return clip((width*p/2 - d)/0.8 + 0.5, 0, 1)
cov = lines(28, clip(T*0.95, 0.04, 0.95))
cov = maximum(cov, lines(-52, clip((T-0.55)*1.6, 0, 0.8)))
a = deg2rad(28); s = (xx*cos(a)+yy*sin(a)+warp)/p
t2 = (-xx*sin(a)+yy*cos(a))/p
dots = (((s % 1)-0.5)**2 + ((t2 % 1)-0.5)**2) < (clip(T-0.82, 0, 1)*2.5)**2*0.25
cov = maximum(cov, dots)
cov = gaussian(cov, 0.45)
tone = 0.035*clip(0.6+0.4*n1, 0, 1)
cov = cov*plate + tone*plate
out = array([0.95, 0.93, 0.87])*(1-cov[..., None]) + array([0.08, 0.07, 0.08])*cov[..., None]
out = out*(0.92+0.08*clip(diff, 0, 1.3))[..., None]''', I='src', n0='n0', n1='n1', plate='plate', diff='li.diff'),
 'out': N('u8', {'x': 'col'})}, 'out')

nodes = {}
for k, ang in enumerate((0, 37, 74, 111, 148)):
    nodes[f'cx{k}'] = X(f'np.full((H, W), cos(deg2rad({ang})))'); nodes[f'cy{k}'] = X(f'np.full((H, W), sin(deg2rad({ang})))')
    nodes[f'g{k}'] = N('strokes', {'tx': f'cx{k}', 'ty': f'cy{k}'}, width=0.55, density=0.05, length=3, seed=sd(k) if k else S, amp_var=0.6)
nodes['n9'] = N('noise', scale=1.2, octaves=1, seed=sd(9))
nodes['col'] = X('''L = stretch(blur(lum(I), 0.9)); T = (1-L)**0.85
g = np.zeros((H, W))
for x in (g0, g1, g2, g3, g4): g = g + x
g = (g-g.min())/(np.ptp(g)+1e-9)
cov = sig((T - (1-g)*0.9 - 0.05)*9)
cov = 0.85*cov + 0.15*gaussian(cov, 1.5)
out = array([0.95, 0.93, 0.88])*(1-cov[..., None]) + array([0.10, 0.08, 0.08])*cov[..., None]
out = out*(1+0.02*n9)[..., None]''', I='src', g0='g0', g1='g1', g2='g2', g3='g3', g4='g4', n9='n9')
nodes['out'] = N('u8', {'x': 'col'})
save('mezzotint', 'Mezzotint', 'A rocked plate printing velvet black, the lights burnished out; rocker grain in many directions.', nodes, 'out')

save('linocut', 'Linocut', 'White gouges along the forms, more and wider where light; solid lands in the dark; burnished mottle.', {
 'f': N('flow', {'image': 'src'}, sigma=6),
 'g1': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=1.1, density=0.006, length=24, seed=S, amp_var=0.3),
 'g2': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=0.8, density=0.01, length=14, seed=sd(1), amp_var=0.3),
 'n0': N('noise', scale=14, octaves=3, seed=S),
 'col': X('''L = stretch(blur(lum(I), 1.0))
gouge = maximum(g1, 0.9*g2)
cut = (gouge*(0.4+1.6*L) > 0.55-0.2*L) | (L > 0.86)
inked = ndi.binary_opening(~cut, iterations=1)
mottle = clip(0.93+0.07*n0, 0.75, 1.0)
cov = gaussian(inked.astype(float), 0.5)*mottle
out = array([0.95, 0.93, 0.87])*(1-cov[..., None]) + array([0.09, 0.08, 0.08])*cov[..., None]''', I='src', g1='g1', g2='g2', n0='n0'),
 'out': N('u8', {'x': 'col'})}, 'out')

nodes = {'r': N('rng', seed=S)}
prev = 'r'
for k in range(4):
    nodes[f'd{k}'] = N('draw', {'rng': prev if k == 0 else f'd{k-1}.rng'}, kind='normal')
    nodes[f'n{k}'] = N('noise', scale=30, octaves=3, seed=sd(k) if k else S)
nodes['col'] = X('''cmy = 1-I; k = cmy.min(-1)*0.85; cmy = (cmy-k[..., None])/(1-k[..., None]+1e-6)
dens = [cmy[..., 1], cmy[..., 0], cmy[..., 2], k]
inks = [(1.0, 0.28, 0.56), (0.0, 0.47, 0.75), (1.0, 0.91, 0.0), (0.12, 0.12, 0.13)]
paper = array([0.96, 0.95, 0.92])
out = np.ones((H, W, 3))*paper
for i, (ink, ang, off, dn, nz) in enumerate(zip(inks, (75, 15, 0, 45), ((0, 0), (1.8, -1.2), (-1.4, 1.6), (0.7, 0.9)), (d0, d1, d2, d3), (n0, n1, n2, n3))):
    a = deg2rad(ang); u = xx*cos(a)+yy*sin(a); v = -xx*sin(a)+yy*cos(a)
    screen = 0.5+0.3*sin(2*pi*u/3.6)*sin(2*pi*v/3.6) + 0.16*dn
    d = ndi.shift(gaussian(dens[i], 0.7), off, order=1, mode='nearest')
    dot = 1/(1+exp(-(d-screen)*10))
    cov = (dot*clip(0.9+0.1*nz, 0.6, 1))[..., None]
    out = out*((1-cov)+cov*array(ink)/paper)''', I='src', **{f'd{k}': f'd{k}.out' for k in range(4)}, **{f'n{k}': f'n{k}' for k in range(4)})
nodes['out'] = N('u8', {'x': 'col'})
save('risograph', 'Risograph', 'Four drums (pink, blue, yellow, black), each a grainy screen at its own angle, slightly out of register.', nodes, 'out')

save('cyanotype', 'Cyanotype', 'Prussian blue and paper white, streaky hand-coated sensitiser, full bleed.', {
 'n0': N('noise', scale=60, octaves=3, seed=S), 'n1': N('noise', scale=12, octaves=2, seed=sd(1)),
 'r': N('rng', seed=S), 'nz': N('draw', {'rng': 'r'}, kind='normal'),
 'ng': X('gaussian(n, 3)', n='nz.out'), 'one': X('np.ones((H, W))'), 'zero': X('np.zeros((H, W))'),
 'sm': N('smear', {'x': 'ng', 'tx': 'one', 'ty': 'zero'}, length=30),
 'pt': N('paper_tooth'),
 'col': X('''L = stretch(blur(lum(I), 0.8)); d = (1-L)**0.85
d = d*(0.93+0.07*clip(n0, -2, 2)) + 0.07
edge = np.full((H, W), 40.0)
strokes = 6*z(sm)
coat = sig((edge + strokes + 6*n1)/2.5)
d = clip(d*(0.94+0.06*coat), 0, 1)
out = array([0.95, 0.94, 0.90])*(1-d[..., None]) + array([0.04, 0.17, 0.38])*d[..., None]
out = out*(1+0.025*z(pt))[..., None]''', I='src', n0='n0', n1='n1', sm='sm', pt='pt'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('pen_plotter', 'Pen plotter', 'Evenly spaced streamlines in fineliners, closer where darker, crossed in the shadows.', {
 'L': X('stretch(blur(lum(I), 1.0))', I='src'),
 'T': N('mono_tone', {'image': 'src', 'L': 'L'}),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'mtx': X('-tx', tx='f.tx'),
 'la': N('streamlines', {'tx': 'f.tx', 'ty': 'f.ty', 'density': 'T'}, dmin=2.2, dmax=10.0, tmin=0.06, seed=S, maxlen=500, minlen=8),
 'lb': N('streamlines', {'tx': 'f.ty', 'ty': 'mtx', 'density': 'T'}, dmin=2.2, dmax=10.0, tmin=0.6, seed=sd(1), maxlen=500, minlen=8),
 'out': N('pen_draw', {'image': 'src', 'lines_a': 'la', 'lines_b': 'lb'})}, 'out')

# ======================================================================= print and screen
PLATE_INKS = "[array([0, 164, 228])/255, array([232, 18, 130])/255, array([255, 230, 0])/255, array([38, 34, 36])/255]"
save('offset_press', 'Offset press (NASA Facts, 1978)', 'Four plates, rich black, each slipping out of register, ink feathering into cream stock.', {
 'sep': N('separate_cmyk', {'image': 'src'}), 'paper': N('paper_ground', seed=S), 'n1': N('noise', scale=120, octaves=2, seed=sd(1)),
 'col': X(f'''dens = 1 + 0.05*n1
out = paper
for cov, ink, (dy, dx) in zip((c, m, y, k), {PLATE_INKS}, ((-4, -3), (4, 1), (-1, 5), (0, 0))):
    cc = clip(gaussian(cov, 1.0)*dens, 0, 1)
    cc = ndi.shift(cc, (dy, dx), order=1, mode='nearest')
    out = out*(1 - cc[..., None]*(1-ink[None, None]))''', c='sep.c', m='sep.m', y='sep.y', k='sep.k', paper='paper', n1='n1'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('halftone', 'Halftone', 'AM CMYK screens at 15/75/0/45 degrees forming rosettes, dot gain, a little slip.', {
 'b': X('blur(I, 0.6)', I='src'), 'sep': N('separate_cmyk', {'image': 'b'}), 'paper': N('paper_ground', seed=S),
 'col': X(f'''out = paper; cell = 6.0
for cov, ink, ang, (dy, dx) in zip((c, m, y, k), {PLATE_INKS}, (15, 75, 0, 45), ((-1.5, -1), (1.5, 0.5), (0, 1.5), (0, 0))):
    a = deg2rad(ang); u = (xx*cos(a)+yy*sin(a))/cell; v = (-xx*sin(a)+yy*cos(a))/cell
    spot = 0.5*(cos(2*pi*u)+cos(2*pi*v))
    cc = clip(cov*1.08, 0, 1)
    dot = sig((spot - (1-2*cc))*7.0)*(cc > 0.02)
    dot = gaussian(dot, 0.45)
    dot = ndi.shift(dot, (dy, dx), order=1, mode='nearest')
    out = out*(1 - dot[..., None]*(1-ink[None, None]))''', c='sep.c', m='sep.m', y='sep.y', k='sep.k', paper='paper'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('dot_matrix', 'Dot-matrix printer', 'A 9-pin impact printer on paper; `bands` sets how strongly the passes show (0 = none).', {
 'T': X('(1-stretch(blur(lum(I), 0.8)))**0.8', I='src'),
 'r': N('rng', seed=S),
 'dots': N('impact_dots', {'tone': 'T', 'rng': 'r'}, pitch=3.2, bands='$bands'),
 'n2': N('noise', scale=1.0, octaves=1, seed=sd(2)),
 'col': X('''ink = gaussian(d, 0.5)*(1+0.12*n2)
ink = clip(ink, 0, 1)
out = array([0.95, 0.94, 0.90])*(1-ink[..., None]) + array([0.10, 0.08, 0.13])*ink[..., None]''', d='dots.out', n2='n2'),
 'out': N('u8', {'x': 'col'})}, 'out', dict(bands=1.0))
save('dot_matrix_clean', 'Dot-matrix printer, no scan lines', 'The dot-matrix preset with its bands turned off.',
 {'p': N('preset:dot_matrix', {'src': 'src'}, seed=S, bands=0.0)}, 'p')

save('luminet', 'Stipple (after Luminet, 1979)', 'Hand-drawn ink dots, denser where brighter, printed as a negative.', {
 'L': X('stretch(blur(lum(I), 1.0))', I='src'), 'p': X('0.05 + 0.95*L**2.2', L='L'),
 'r': N('rng', seed=S),
 'pts': N('jitter_points', {'rng': 'r'}, spacing=3.4, jitter=0.5),
 'kept': N('keep_points', {'points': 'pts.points', 'rng': 'pts.rng', 'p': 'p'}),
 'dots': N('stamp_dots', {'points': 'kept.points', 'rng': 'kept.rng'}, SS=3, rad=[0.6, 1.05], ecc=[0.85, 1.15], ink=[0.75, 1.0]),
 'gr': N('draw', {'rng': 'dots.rng'}, kind='normal', shape='HW1'),
 'col': X('''d = clip(gaussian(d, 0.35)*1.25, 0, 1)
out = array([0.07, 0.065, 0.055])*(1-d[..., None]) + array([0.93, 0.90, 0.78])*d[..., None]
out = out + 0.012*gr''', d='dots.out', gr='gr.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

def dither_preset(name, title, kind, size, steps, back='#000c38', front='#94ffaf', highlight='#eaff94', original=False):
    nodes = {'pd': N('pixelate_down', {'image': 'src'}, size=size, rounding='ceil'),
             'r': N('rng', seed=S), 'thr': N('threshold_map', {'like': 'pd.out', 'rng': 'r'}, kind=kind)}
    q = f'''steps = {steps}
def q(v):
    x = v*(steps-1); lo = floor(x); return clip((lo + (x-lo > thr)) / max(steps-1, 1), 0, 1)
'''
    if original:
        q += 'out = stack([q(small[..., c]) for c in range(3)], -1)'
    else:
        q += f'''L = small @ array([0.299, 0.587, 0.114])
t = q(L)[..., None]
B, F, Hc = hexc("{back}"), hexc("{front}"), hexc("{highlight}")
lvl = rint(t*(steps-1))
g = clip((lvl-1)/max(steps-2, 1), 0, 1)
out = where(lvl == 0, B, F + (Hc-F)*g)'''
    nodes['small'] = X(q, small='pd.out', thr='thr.out')
    nodes['big'] = N('pixelate_up', {'small': 'small'}, size=size)
    nodes['out'] = N('u8', {'x': 'big'})
    save(name, title, 'Ordered or random dithering on a coarse pixel grid (after paper.design Image Dithering).', nodes, 'out')
dither_preset('dither', 'Dither, 8x8, two colours', '8x8', 3, 2)
dither_preset('dither_3', 'Dither, 8x8, three colours', '8x8', 3, 3)
dither_preset('dither_original', 'Dither, 4x4, original colours', '4x4', 3, 3, original=True)
dither_preset('dither_random', 'Dither, random', 'random', 2, 2, '#1d1b19', '#efe9dc', '#efe9dc')

# ======================================================================= craft
save('marbling', 'Marbling', 'Inks floated and combed through, dispersant rings at the edges, stones dropped on last.', {
 'r': N('rng', seed=S),
 'rg': N('regions', {'image': 'src'}, K=7, b=2.5, mode=7),
 'n4': N('noise', scale=90, octaves=2, seed=sd(4)),
 'dx': X('9*sin(2*pi*yy/64 + 2*n4)', n4='n4'), 'dy': X('6*sin(2*pi*xx/110 + 1.3)'),
 'lw': N('warp', {'x': 'rg.labels', 'dy': 'dy', 'dx': 'dx'}, order=0),
 'lab': N('mode_filter', {'labels': 'lw'}, size=7),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'C0': X('(lambda C: (lambda g: g+(C-g)*0.88)(lum(C)[..., None]))(pal[lab])', pal='rg.palette', lab='lab'),
 'C1': N('smear', {'x': 'C0', 'tx': 'f.tx', 'ty': 'f.ty'}, length=3),
 'bd': N('boundary', {'labels': 'lab'}), 'nb': X('b == 0', b='bd'), 'dist': N('distance', {'mask': 'nb'}),
 'st': N('stones', {'palette': 'rg.palette', 'rng': 'r'}), 'pt': N('paper_tooth'),
 'col': X('''C = C1*(1 - 0.25*exp(-dist/0.8))[..., None]
C = C*(1-0.12*exp(-((dist-3)/1.5)**2))[..., None] + 0.12*exp(-((dist-3)/1.5)**2)[..., None]
s = gaussian(st, 0.6)[..., None]
C = C*(1-s) + scol*s
C = C*0.93 + array([0.95, 0.93, 0.87])*0.07
out = C*(1+0.02*z(pt))[..., None]''', C1='C1', dist='dist', st='st.mask', scol='st.colour', pt='pt'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('mosaic', 'Mosaic', 'Opus vermiculatum: rows of smalti following the forms, tilted, bevelled and glossy, in grout.', {
 'r': N('rng', seed=S), 'f': N('flow', {'image': 'src'}, sigma=6),
 'rows': N('streamlines', {'tx': 'f.tx', 'ty': 'f.ty', 'density': 'one'}, dmin='$s', dmax='$s', tmin=0.0, seed=S, maxlen=1500, minlen=2),
 'one': X('1.0'),
 'tiles': N('tiles_along', {'lines': 'rows', 'rng': 'r'}, s='$s'),
 'cell': N('voronoi', {'points': 'tiles.points'}),
 'n': X('len(p)', p='tiles.points'),
 'mc': N('mean_by_label', {'image': 'src', 'labels': 'cell'}, n='@n'),
 'mcol': X('np.nan_to_num(m, nan=0.5)', m='mc'),
 'sm': N('smalti', {'colours': 'mcol', 'rng': 'tiles.rng'}, K=28),
 'tilt': N('draw', {'rng': 'sm.rng', 'n': 'tiles.points'}, kind='normal', a=0.0, b=0.12, shape='N2'),
 'edge': N('boundary', {'labels': 'cell'}), 'ne': X('~e', e='edge'), 'd': N('distance', {'mask': 'ne'}),
 'hgt': X('''h = clip(d/3.0, 0, 1)*3 + tilt[cell, 0]*(xx-pts[cell, 0]) + tilt[cell, 1]*(yy-pts[cell, 1])
h = where(edge, 0, h)
out = gaussian(h, 0.6)''', d='d', tilt='tilt.out', cell='cell', pts='tiles.points', edge='edge'),
 'li': N('light', {'height': 'hgt'}, strength=1.0, shin=40, L=RAK),
 'n2': N('noise', scale=1.5, octaves=1, seed=sd(2)),
 'col': X('''G = array([0.74, 0.72, 0.67])*(1+0.06*n2)[..., None]
out = where(edge[..., None], G*(0.8+0.2*diff[..., None]), sm[cell]*(0.78+0.22*diff[..., None]) + 0.25*spec[..., None])''',
   n2='n2', edge='edge', diff='li.diff', spec='li.spec', sm='sm.colours', cell='cell'),
 'out': N('u8', {'x': 'col'})}, 'out', dict(s=11.0))

save('stained_glass', 'Stained glass', 'Fate regions cut into panes of streaky cathedral glass, leaded along the boundaries, lit from behind.', {
 'r': N('rng', seed=S),
 'rg': N('regions', {'image': 'src'}, K=9, b=2.0, mode=9),
 'jp': N('jitter_points', {'rng': 'r'}, spacing=64, jitter=0.45),
 'vor': N('voronoi', {'points': 'jp.points'}),
 'pc': X('lab*10000 + vor', lab='rg.labels', vor='vor'),
 'piece': N('relabel', {'labels': 'pc'}),
 'mc': N('mean_by_label', {'image': 'src', 'labels': 'piece.labels'}, n='@piece.n'),
 'cn': N('draw', {'rng': 'jp.rng', 'n': 'mc'}, kind='normal', shape='N1'),
 'n1': N('noise', scale=150, octaves=2, seed=sd(1)),
 'sx': X('cos(pi*n)', n='n1'), 'sy': X('sin(pi*n)', n='n1'),
 'sn': N('draw', {'rng': 'cn.rng'}, kind='normal'),
 'ss': N('smear', {'x': 'sn.out', 'tx': 'sx', 'ty': 'sy'}, length=14),
 'n2': N('noise', scale=35, octaves=3, seed=sd(2)),
 'sd': N('draw', {'rng': 'sn.rng'}, kind='random'),
 'bd': N('boundary', {'labels': 'piece.labels'}),
 'lead': N('morph', {'mask': 'bd'}, iterations=2),
 'lh': X('gaussian(l.astype(float), 1.2)*4', l='lead'),
 'li': N('light', {'height': 'lh'}, strength=1.0, shin=30, L=RAK),
 'col': X('''g = col @ array([0.299, 0.587, 0.114]); col = g[:, None] + (col-g[:, None])*1.45
col = clip(col, 0.05, 0.92)*(1+0.08*cn)
G = clip(col[piece], 0, 1)
streak = z(ss)
G = G*(1 + 0.12*streak + 0.10*n2)[..., None]
seeds = gaussian((sd < 0.0005).astype(float), 1.0)*8
G = G + clip(seeds, 0, 1)[..., None]*0.5
G = G + 0.25*blur(clip(G-0.6, 0, 1), 7)
leadcol = array([0.10, 0.10, 0.11])[None, None]*(0.6+0.5*diff[..., None]) + 0.25*spec[..., None]
out = where(lead[..., None], leadcol, G)''', col='mc', cn='cn.out', piece='piece.labels', ss='ss', n2='n2', sd='sd.out', lead='lead', diff='li.diff', spec='li.spec'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('kintsugi', 'Kintsugi', 'The render as glazed ceramic, broken along its fate boundaries and mended with gold.', {
 'r': N('rng', seed=S),
 'rg': N('regions', {'image': 'src'}, K=5, b=3.5, mode=11),
 'n0': N('noise', scale=20, octaves=2, seed=S),
 'wide': X('''seam = gaussian(bd.astype(float), 1.0) > 0.12
seam = seam & ~ndi.binary_opening(seam, iterations=3)
w = ndi.binary_dilation(seam, iterations=1) | (seam & (n0 > 0.4))
out = ndi.binary_dilation(w, iterations=1)''', bd='bd', n0='n0'),
 'bd': N('boundary', {'labels': 'rg.labels'}),
 'jp': N('jitter_points', {'rng': 'r'}, spacing=26, jitter=0.5),
 'cz': N('voronoi', {'points': 'jp.points'}), 'craze': N('boundary', {'labels': 'cz'}),
 'n1': N('noise', scale=3, octaves=1, seed=sd(1)),
 'gh': X('gaussian(w.astype(float), 1.3)*3 + 0.3*n1*w', w='wide', n1='n1'),
 'lg': N('light', {'height': 'gh'}, strength=1.0, shin=60, L=RAK),
 'n3': N('noise', scale=80, octaves=2, seed=sd(3)), 'n3h': X('n*2', n='n3'),
 'lw': N('light', {'height': 'n3h'}, strength=1.0, shin=80, L=RAK),
 'col': X('''glaze = blur(I, 1.3); g = lum(glaze)[..., None]; glaze = g+(glaze-g)*0.9
glaze = glaze*(1-0.10*gaussian(craze.astype(float), 0.5))[..., None]
G = array([0.80, 0.62, 0.24])*(0.55+0.6*clip(diff, 0, 1.4))[..., None] + spec[..., None]*array([1, 0.95, 0.8])*0.9
yi, xi = np.mgrid[0:H, 0:W]
win = sig((0.25*W - abs(xi-0.3*W))/12)*sig((0.18*H - abs(yi-0.22*H))/12)
out = glaze*0.96 + 0.10*gaussian(win, 18)[..., None]
out = out + 0.10*gspec[..., None]
a = gaussian(wide.astype(float), 0.6)[..., None]
out = out*(1-a) + G*a''', I='src', craze='craze', diff='lg.diff', spec='lg.spec', gspec='lw.spec', wide='wide'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('cut_paper', 'Cut paper', 'One sheet per colour stacked by lightness, blade-cut, lit edges, shadows on the sheets beneath.', {
 'rg': N('regions', {'image': 'src'}, K=6, b=2.5, mode=9), 'pt': N('paper_tooth'),
 **{f't{k}': N('noise', scale=1.4, octaves=2, seed=sd(k) if k else S) for k in range(7)},
 'col': X('''used = np.unique(lab); order = used[np.argsort(lum(pal[used]))]
rank = np.zeros(pal.shape[0], int); rank[order] = np.arange(len(order)); R = rank[lab]
T = (t0, t1, t2, t3, t4, t5, t6)
def tex(i): return (1+0.035*T[i] + 0.02*z(pt))[..., None]
c0 = pal[order[0]]; g = c0.mean(); c0 = g+(c0-g)*0.9
out = np.ones((H, W, 3))*c0*tex(0)
for i in range(1, len(order)):
    m = R >= i
    m = ndi.binary_opening(ndi.binary_closing(m, iterations=2), iterations=2)
    m = gaussian(m.astype(float), 1.2) > 0.5
    sh = gaussian(ndi.shift(m.astype(float), (4, 3), order=0), 4.0)
    out = out*(1-0.38*sh*(~m))[..., None]
    c = pal[order[i]]; g = c.mean(); c = g+(c-g)*0.9
    lit = m & ~ndi.shift(m, (-1.5, -1.5), order=0)
    sheet = c*tex(i)*(1+0.12*lit[..., None])
    out = where(m[..., None], sheet, out)''', lab='rg.labels', pal='rg.palette', pt='pt', **{f't{k}': f't{k}' for k in range(7)}),
 'out': N('u8', {'x': 'col'})}, 'out')

save('thread_painting', 'Thread painting', 'Long-and-short satin stitches along the forms in floss colours, sheen changing with direction.', {
 'r': N('rng', seed=S), 'q': N('quantize', {'image': 'src'}, K=26), 'f': N('flow', {'image': 'src'}, sigma=4),
 'cv': N('canvas', period=6, seed=S, scale=2),
 'base': X('array([0.84, 0.80, 0.72])[None, None]*(1+0.08*cv[..., None])', cv='cv'),
 'jp': N('jitter_points', {'rng': 'r'}, spacing=3.2, jitter=0.5),
 'sh': N('shuffle', {'points': 'jp.points', 'rng': 'jp.rng'}),
 'out': N('stitches', {'points': 'sh.points', 'rng': 'sh.rng', 'tx': 'f.tx', 'ty': 'f.ty', 'labels': 'q.labels', 'palette': 'q.palette', 'base': 'base'}, SS=2),
}, 'out.out')

# ======================================================================= photographic, optical
save('polaroid', 'Polaroid', 'Instant film colour: lifted blacks, cyan shadows, warm highlights, softness, a light leak, grain. Full bleed.', {
 'r': N('rng', seed=S), 'b': N('draw', {'rng': 'r'}, kind='random'), 'g': N('draw', {'rng': 'b.rng'}, kind='normal', shape='HW1'),
 'col': X('''L = lum(I)[..., None]
out = 0.07 + 0.86*I
out = out + array([-0.015, 0.02, 0.035])*(1-L) + array([0.04, 0.02, -0.035])*L
g = lum(out)[..., None]; out = g+(out-g)*0.82
out = blur(out, 0.9)
rr = hypot((xx-W/2)/W, (yy-H/2)/H); out = out*(1-0.35*rr**2*2)[..., None]
leak = exp(-(((xx-W)/(0.35*W))**2 + ((yy)/(0.4*H))**2))
out = out + leak[..., None]*array([0.35, 0.10, 0.02])
bub = gaussian((b < 0.00001).astype(float), 2.5)*60
out = out + clip(bub, 0, 0.5)[..., None]*0.4
out = out + 0.03*gr
out = clip(out, 0, 1)''', I='src', b='b.out', gr='g.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

NEON = '''n = I/maximum(I.max(-1, keepdims=True), 0.08)
g = n.mean(-1, keepdims=True); n = clip(g + (n-g)*SAT, 0, 1)
'''
save('light_painting', 'Light painting', 'Long-exposure light trails along the field in neon, adding where they cross, with film glow.', {
 'r': N('rng', seed=S),
 'L': X('stretch(blur(0.5*lum(I)+0.5*I.max(-1), 1.0))', I='src'),
 'E': N('edge_density', {'image': 'src'}, s=5), 'f': N('field_flow', {'image': 'src', 'E': 'E'}),
 'lines': N('streamlines', {'tx': 'f.tx', 'ty': 'f.ty', 'density': 'L'}, dmin=6.0, dmax=20.0, tmin=0.05, seed=S, maxlen=1200, minlen=20),
 'neon': X(NEON.replace('SAT', '1.7') + 'out = clip(n*0.95 + 0.05, 0, 1)', I='src'),
 'tr': N('trails', {'lines': 'lines', 'rng': 'r', 'colour': 'neon', 'weight': 'L'}),
 'e0': X('blur(acc, 0.8)*4.0 + (blur(acc, 2.5)*1.0 + blur(acc, 8.0)*1.6 + blur(acc, 24.0)*1.2)*2.2', acc='tr.acc'),
 'e': N('film_glow', {'e': 'e0'}),
 'gr': N('draw', {'rng': 'tr.rng'}, kind='normal', shape='HW1'),
 'col': X('''out = 1 - exp(-e)
hot = clip(e.mean(-1, keepdims=True)-0.9, 0, None)
out = out + (1-out)*(1-exp(-hot*1.5))
gnd = array([0.055, 0.05, 0.05])
out = gnd + (1-gnd)*out
out = out + 0.012*gr''', e='e', gr='gr.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('light_painting_solid', 'Light painting, solid', 'Light trails packed edge to edge, thin in the filaments and broad where calm, with film glow.', {
 'r': N('rng', seed=S),
 'L': X('stretch(blur(0.5*lum(I)+0.5*I.max(-1), 1.0))', I='src'),
 'E': N('edge_density', {'image': 'src'}, s=5), 'f': N('field_flow', {'image': 'src', 'E': 'E'}),
 'lines': N('streamlines', {'tx': 'f.tx', 'ty': 'f.ty', 'density': 'E'}, dmin=2.2, dmax=12.0, tmin=0.0, seed=S, maxlen=1500, minlen=6),
 'neon': X(NEON.replace('SAT', '1.6') + 'out = n', I='src'),
 'tr': N('trails_solid', {'lines': 'lines', 'rng': 'r', 'colour': 'neon', 'weight': 'L', 'E': 'E'}, dmin=2.2, dmax=12.0),
 'e0': X('core*2.2 + (blur(core, 3.0)*0.6 + blur(core, 12.0)*0.6)*0.9', core='tr.core'),
 'e': N('film_glow', {'e': 'e0'}, bloom=0.55, halation=1.2, bleed=0.5),
 'gr': N('draw', {'rng': 'tr.rng'}, kind='normal', shape='HW1'),
 'col': X('''out = 1 - exp(-e)
hot = clip(e.mean(-1, keepdims=True)-1.1, 0, None)
out = out + (1-out)*(1-exp(-hot*1.2))
gnd = array([0.05, 0.045, 0.045])
out = gnd + (1-gnd)*out
out = out + 0.01*gr''', e='e', gr='gr.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

for base in ('light_painting', 'light_painting_solid'):
    save(base+'_polaroid', 'Light painting on a Polaroid' if base == 'light_painting' else 'Light painting, solid, on a Polaroid',
         'A light painting, developed on instant film: one preset feeding another.',
         {'lp': N('preset:'+base, {'src': 'src'}, seed=S), 'pol': N('preset:polaroid', {'src': 'lp'}, seed=S)}, 'pol')

save('thin_film', 'Thin film', 'Interference colours: the render sets the film thickness.', {
 'f': N('flow', {'image': 'src'}, sigma=6),
 't0': X('90 + 380*blur(lum(I), 1.0)', I='src'),
 't1': N('smear', {'x': 't0', 'tx': 'f.tx', 'ty': 'f.ty'}, length=5),
 'n0': N('noise', scale=60, octaves=2, seed=S), 'n1': N('noise', scale=60, octaves=2, seed=sd(1)),
 'h': X('n*3', n='n1'), 'li': N('light', {'height': 'h'}, strength=1.0, shin=60, L=RAK),
 'col': X('''t = t1 + 8*n0
lam = array([640., 545., 455.])
R = 0.5 - 0.5*cos(4*pi*1.38*t[..., None]/lam)
R = R**1.2
g = R.mean(-1, keepdims=True); R = g + (R-g)*0.8
out = 0.04 + 0.9*R*array([1.0, 0.97, 1.05])
out = out + 0.12*spec[..., None]''', t1='t1', n0='n0', spec='li.spec'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('film_negative', 'Film negative', 'Colour negative on the orange C-41 base, grain, dust and fine scratches.', {
 'r': N('rng', seed=S), 'gn': N('grain', seed=S, size=1.0), 'du': N('draw', {'rng': 'r'}, kind='random'),
 'base': X('''neg = 1 - blur(I, 0.6)
out = array([0.93, 0.60, 0.36])*(0.18 + 0.82*neg)
out = out**array([0.9, 1.0, 1.15])
out = out*(1 + 0.06*gn*(0.5+neg.mean(-1)))[..., None]
dust = gaussian((du < 0.00015).astype(float), 0.9)*10
out = out*(1-clip(dust, 0, 0.6))[..., None]''', I='src', gn='gn', du='du.out'),
 'sc': N('scratches', {'image': 'base', 'rng': 'du.rng'}),
 'out': N('u8', {'x': 'sc.out'})}, 'out')

save('film_35mm', '35mm film, developed', 'Printed colour film: S-curve, lifted blacks, warm and cool, halation, mid-tone grain, vignette.', {
 'pre': X('''C = blur(I, 0.7)
C = 0.03 + 0.94*(1/(1+exp(-(C-0.5)*5.2)) - 0.069)/0.862
L = lum(C)[..., None]
C = C + array([0.035, 0.012, -0.03])*L + array([-0.02, 0.005, 0.03])*(1-L)
g = L; C = g + (C-g)*0.92
out = -log(clip(1-C, 1e-3, 1))''', I='src'),
 'L': X('''C = blur(I, 0.7)
C = 0.03 + 0.94*(1/(1+exp(-(C-0.5)*5.2)) - 0.069)/0.862
out = lum(C)[..., None]''', I='src'),
 'gl': N('film_glow', {'e': 'pre'}, bloom=0.10, halation=0.35, bleed=0.0),
 'gr': N('grain', seed=S, size=1.4),
 'col': X('''C = 1 - exp(-gl)
C = C + 0.022*gr[..., None]*(4*L*(1-L))
v = 1 - 0.28*(((xx-W/2)/W)**2 + ((yy-H/2)/H)**2)*2
out = C*v[..., None]''', gl='gl', gr='gr', L='L'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('lcd', 'Old LCD', 'A coarse panel of RGB sub-pixels with the screen door, few colour bits dithered, edge backlight bleed, off-axis wash.', {
 'pd': N('pixelate_down', {'image': 'src'}, size=6, rounding='floor'),
 'thr': N('bayer', {'like': 'pd.out'}, n=4), 'n0': N('noise', scale=80, octaves=2, seed=S),
 'col': X('''cell = 6; lv = 2**5 - 1
small = floor(small*lv + thr[..., None])/lv
gh, gw = small.shape[:2]
gy = np.mgrid[0:gh, 0:gw][0]
small = small**(0.75 + 0.25*(gy/gh))[..., None]
big = np.repeat(np.repeat(small, cell, 0), cell, 1)
big = np.pad(big, ((0, H-big.shape[0]), (0, W-big.shape[1]), (0, 0)), mode='edge')
sub = np.zeros((H, W, 3)); xs = np.arange(W) % cell; ys = np.arange(H) % cell
w = cell/3
for c in range(3):
    sub[:, (xs >= c*w+0.5) & (xs < (c+1)*w-0.4), c] = 1
sub[ys >= cell-1, :, :] = 0
sub = gaussian(sub, (0.35, 0.35, 0))
out = big*sub*2.9
out = blur(out, 0.45) + 0.30*blur(out, 2.5)
edge = minimum(minimum(xx, W-1-xx), minimum(yy, H-1-yy))/min(H, W)
bleed = 0.05*exp(-edge/0.06)*(1+0.5*n0)
out = out + bleed[..., None]*array([0.85, 0.9, 1.0])''', small='pd.out', thr='thr', n0='n0'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('crt', 'CRT television', 'Scanlines through an aperture grille, bandwidth blur, misconvergence, glow, snow, a slow roll.', {
 'r': N('rng', seed=S), 'sn': N('draw', {'rng': 'r'}, kind='normal', shape='HW1'), 'ph': N('draw', {'rng': 'sn.rng'}, kind='uniform', a=0.0, b=1.0),
 'col': X('''C = gaussian(I, (0.3, 1.6, 0))
C = stack([ndi.shift(C[..., 0], (0, 1.2), order=1, mode='nearest'), C[..., 1], ndi.shift(C[..., 2], (0.4, -1.2), order=1, mode='nearest')], -1)
period = H/200; yc = np.arange(H)[:, None]
ph_ = (yc % period)/period
beam = exp(-((ph_-0.5)/(0.30 + 0.14*lum(C)))**2)
out = C*beam[..., None]*1.55
xs = np.arange(W) % 3.0
grille = stack([(xs < 1.0), (xs >= 1.0) & (xs < 2.0), (xs >= 2.0)], -1).astype(float)
grille = gaussian(grille, (0.5, 0))
out = out*(0.35 + 0.65*grille[None, :, :])*1.35
out = out + blur(out, 2.0)*0.35 + blur(out, 8.0)*0.25
out = out + 0.025*sn
roll = 1 + 0.04*sin(2*pi*(np.arange(H)/H*1.3 + ph))
out = out*roll[:, None, None]
v = 1 - 0.35*(((xx-W/2)/W)**2 + ((yy-H/2)/H)**2)*2
out = out*v[..., None]''', I='src', sn='sn.out', ph='ph.out'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('gameboy', 'Game Boy', 'The original Game Boy screen: 144 rows, four greens, dithered, pixel grid, shadows on the backing.', {
 'pd': N('pixelate_down', {'image': 'src'}, size=1, rounding='rows:144', gray=True),
 'thr': N('bayer', {'like': 'pd.out'}, n=4), 'n0': N('noise', scale=120, octaves=2, seed=S),
 'col': X('''n = 144; cell = H/n; gw = int(round(W/cell))
lo, hi = percentile(small, [1, 99]); small = clip((small-lo)/(hi-lo+1e-9), 0, 1)
lvl = clip(floor(small*3 + thr), 0, 3).astype(int)
pal = stack([hexc(p) for p in ('#0f380f', '#306230', '#8bac0f', '#9bbc0f')])
cy = clip((yy/cell).astype(int), 0, n-1); cx = clip((xx/cell).astype(int), 0, gw-1)
dark = (3-lvl)[cy, cx]/3.0
fy = (yy/cell) % 1; fx = (xx/cell) % 1
inpix = sig((minimum(minimum(fx, 1-fx), minimum(fy, 1-fy)) - 0.06)*40)
d = dark*inpix
d = 0.8*d + 0.2*gaussian(d, (0, cell*0.8))
shadow = ndi.shift(gaussian(d, cell*0.25), (cell*0.22, cell*0.22), order=1, mode='nearest')
back = pal[3]
out = back*(1 - 0.18*shadow[..., None])
out = out*(1-d[..., None]) + (pal[0] + (pal[1]-pal[0])*0.15)*d[..., None]
out = out + (pal[2]-pal[3])*clip(d*(1-d)*2.5, 0, 1)[..., None]*0.6
out = out*(1 + 0.015*n0)[..., None]''', small='pd.out', thr='thr', n0='n0'),
 'out': N('u8', {'x': 'col'})}, 'out')

# ======================================================================= driven by the data
save('relief_of_lifetime', 'Relief of lifetime', 'Paint stands up where beginnings survive longest. Binds data.lifetime; lightness stands in until it exists.', {
 'proxy': X('stretch(blur(lum(I), 2.0))', I='src'),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'st': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=2.2, density=0.005, length=16, seed=S),
 'hgt': X('14*gaussian(life, 2.0) + 1.2*st', life='data.lifetime|proxy', st='st'),
 'li': N('light', {'height': 'hgt'}, strength=1.0, L=[-0.8, -0.5, 0.35], shin=20),
 'h6': X('h/6', h='hgt'), 'sh': N('cast_shadow', {'height': 'h6'}, L=[-0.8, -0.5], reach=14, k=0.8),
 'b': X('blur(I, 0.8)', I='src'), 'C': N('kuwahara', {'image': 'b'}, r=4),
 'col': X('C*(0.35+0.65*clip(diff, 0, 1.6))[..., None]*sh[..., None] + 0.1*spec[..., None]', C='C', diff='li.diff', spec='li.spec', sh='sh'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('wet_where_chaotic', 'Wet where chaotic', 'Watercolour pencil that runs wet-in-wet where the fate is most sensitive. Binds data.sensitivity.', {
 'proxy': N('edge_density', {'image': 'src'}, s=10),
 'dry': N('preset:watercolour_pencil_core', {'src': 'src'}, seed=sd(3), pigments=True, sat=0.95),
 'n1': N('noise', scale=16, octaves=3, seed=sd(1)), 'n2': N('noise', scale=30, octaves=3, seed=sd(2)),
 'col': X('''wet = blur(dry, 5)
ring = (abs(n1) < 0.12).astype(float)
wet = wet*(1-0.12*gaussian(ring, 1.0))[..., None]
wet = wet*0.92 + 0.08*array([0.94, 0.93, 0.89])
m = sig((E - 0.55 + 0.15*n2)*9)[..., None]
out = dry*(1-m) + wet*m''', dry='dry', n1='n1', n2='n2', E='data.sensitivity|proxy'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('craquelure', 'Craquelure', 'An oil painting cracking where neighbouring beginnings separate fastest. Binds data.sensitivity.', {
 'proxy': N('edge_density', {'image': 'src'}, s=10),
 'r': N('rng', seed=S),
 'base': N('preset:oil', {'src': 'src'}, seed=S),
 'jf': N('jitter_points', {'rng': 'r'}, spacing=14, jitter=0.5), 'jc': N('jitter_points', {'rng': 'jf.rng'}, spacing=38, jitter=0.5),
 'vf': N('voronoi', {'points': 'jf.points'}), 'vc': N('voronoi', {'points': 'jc.points'}),
 'bf': N('boundary', {'labels': 'vf'}), 'bc': N('boundary', {'labels': 'vc'}),
 'n1': N('noise', scale=6, octaves=2, seed=sd(1)), 'n2': N('noise', scale=6, octaves=2, seed=sd(2)),
 'cr': X('''wy = 1.2*n1; wx = 1.2*n2
def wob(b): return ndi.map_coordinates(b.astype(float), [yy+wy, xx+wx], order=1) > 0.3
out = (wob(bf) & (E > 0.5)) | (wob(bc) & (E > 0.25))''', bf='bf', bc='bc', n1='n1', n2='n2', E='data.sensitivity|proxy'),
 'lift': X('-gaussian(c.astype(float), 2.0)*3', c='cr'),
 'li': N('light', {'height': 'lift'}, strength=1.0, L=RAK),
 'col': X('''c = gaussian(cr.astype(float), 0.5)
out = base*(1-0.75*c[..., None])*(0.9+0.1*diff[..., None])
out = out*array([1.0, 0.97, 0.88])''', base='base', cr='cr', diff='li.diff'),
 'out': N('u8', {'x': 'col'})}, 'out')

save('unfinished_where_uncertain', 'Unfinished where uncertain', 'Painted only where the result can be trusted; elsewhere it stops at the pencil underdrawing. Binds data.uncertainty.', {
 'proxy': N('edge_density', {'image': 'src'}, s=6),
 'done': N('preset:watercolour_pencil_core', {'src': 'src'}, seed=sd(3), pigments=True, sat=0.95),
 'sk': N('preset:graphite', {'src': 'src'}, seed=S),
 'f': N('flow', {'image': 'src'}, sigma=6),
 'st': N('strokes', {'tx': 'f.tx', 'ty': 'f.ty'}, width=3.0, density=0.004, length=18, seed=sd(3)),
 'n4': N('noise', scale=20, octaves=2, seed=sd(4)),
 'col': X('''paper = array([0.95, 0.94, 0.91])
sketch = paper + 0.55*(sk - paper)
rag = 0.25*clip(st, 0, 1.5) + 0.1*n4
m = sig((U - 0.62 + rag)*14)[..., None]
out = done*(1-m) + sketch*m''', done='done', sk='sk', st='st', n4='n4', U='data.uncertainty|proxy'),
 'out': N('u8', {'x': 'col'})}, 'out')
print('written', len(os.listdir(OUT)))
