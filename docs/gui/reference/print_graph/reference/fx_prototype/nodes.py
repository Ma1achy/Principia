"""The node library: every shared part of the media, as small typed nodes.

Values passed between nodes: images are HxWx3 arrays (uint8 when they came straight from pixels,
float in [0,1] otherwise), scalar fields HxW float, labels HxW int, point sets Nx2, line sets
(lists of polylines), and random streams (a numpy Generator, threaded from node to node so the
draws happen in a fixed order).
"""
import sys, numpy as np
from PIL import Image, ImageDraw, ImageFilter
from scipy import ndimage as ndi
from scipy.signal import fftconvolve
import os; sys.path.insert(0, '/tmp/paper'); sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from graph import node
import media as M, media2 as M2, sketch as SK, draw as DR, paint_any2 as PA, palette as PAL

def F(x):
    """As float: pixels (uint8) become [0,1]."""
    x = np.asarray(x)
    return x.astype(float)/255 if x.dtype == np.uint8 else x
def P(x):
    """As a PIL image: exact for pixel values."""
    x = np.asarray(x)
    if x.dtype != np.uint8: x = (x*255).astype(np.uint8)
    return Image.fromarray(x)

# ----------------------------------------------------------------- glue
SAFE = dict(np=np, ndi=ndi, lum=M.lum, z=M.z, sig=M2.sig, clip=np.clip, gaussian=ndi.gaussian_filter,
            blur=M.blur, maximum=np.maximum, minimum=np.minimum, where=np.where, exp=np.exp, log=np.log,
            sin=np.sin, cos=np.cos, abs=np.abs, sqrt=np.sqrt, floor=np.floor, rint=np.rint, pi=np.pi,
            hypot=np.hypot, gradient=np.gradient, percentile=np.percentile, array=np.array, stack=np.stack,
            deg2rad=np.deg2rad, arctan2=np.arctan2, stretch=M2.stretch, hexc=M2._hex)

@node('expr', f='0')
def expr(_ctx, f, **ins):
    """Arithmetic glue: evaluate `f` over the inputs (pixels arrive as floats). H, W, yy, xx are defined."""
    env = dict(SAFE); env.update({k: F(v) if isinstance(v, np.ndarray) else v for k, v in ins.items()})
    yy, xx = np.mgrid[0:_ctx.H, 0:_ctx.W].astype(float)
    env.update(H=_ctx.H, W=_ctx.W, yy=yy, xx=xx, int=int, float=float, range=range, len=len, min=min, max=max, zip=zip, enumerate=enumerate, tuple=tuple, bool=bool)
    if '\n' in f or 'out =' in f or 'out=' in f:
        exec(f, env); return env['out']
    return eval(f, env)

@node('u8', inputs=('x',))
def u8(c, x):
    """Print to 8-bit pixels (truncating, as the media always have)."""
    x = np.asarray(x)
    return x if x.dtype == np.uint8 else (np.clip(x, 0, 1)*255).astype(np.uint8)

# ----------------------------------------------------------------- random streams
@node('rng', outputs=('rng',), seed=0)
def rng(c, seed): return {'rng': np.random.default_rng(seed)}

@node('draw', inputs=('rng',), outputs=('out', 'rng'), kind='random', shape='HW', a=0.0, b=1.0)
def draw(c, rng, kind, shape, a, b, n=None):
    """Draw from a random stream. shape: 'HW', 'HW1', 'N1' (N from input n), or a list."""
    shp = {'HW': (c.H, c.W), 'HW1': (c.H, c.W, 1)}.get(shape, shape) if isinstance(shape, str) else tuple(shape)
    if shape == 'N1': shp = (len(n), 1)
    if shape == 'N2': shp = (len(n), 2)
    if kind == 'random': out = rng.random(shp)
    elif kind == 'normal': out = rng.standard_normal(shp) if (a, b) == (0.0, 1.0) else rng.normal(a, b, shp)
    elif kind == 'uniform': out = rng.uniform(a, b)
    else: raise ValueError(kind)
    return {'out': out, 'rng': rng}

# ----------------------------------------------------------------- analysis
@node('flow', inputs=('image',), outputs=('tx', 'ty', 'coh'), sigma=4.0)
def flow(c, image, sigma):
    """Unit tangents along the image's contours (structure tensor), and how coherent they are."""
    tx, ty, coh = M.flow(F(image), sigma); return {'tx': tx, 'ty': ty, 'coh': coh}

@node('field_flow', inputs=('image', 'E'), outputs=('tx', 'ty'))
def field_flow(c, image, E):
    """The fine flow where the filaments are fine, the broad flow where the field is calm."""
    tx, ty = M2._field_flow(F(image), E); return {'tx': tx, 'ty': ty}

@node('edge_density', inputs=('image',), s=8)
def edge_density(c, image, s):
    """How fast the picture changes here: the density of fine fringes."""
    return M2.edge_density(F(image), s)

@node('regions', inputs=('image',), outputs=('labels', 'palette'), K=8, b=1.5, mode=5)
def regions(c, image, K, b, mode):
    """Flat colour regions: blur, median-cut to K colours, mode-filter."""
    lab, pal = M2.regions(P(image), K, b, mode); return {'labels': lab, 'palette': pal}

@node('quantize', inputs=('image',), outputs=('labels', 'palette'), K=26)
def quantize(c, image, K):
    """Median-cut to K colours, no smoothing."""
    q = P(image).convert('RGB').quantize(K, method=Image.Quantize.MEDIANCUT)
    return {'labels': np.asarray(q).astype(int), 'palette': np.array(q.getpalette()).reshape(-1, 3)/255}

@node('mode_filter', inputs=('labels',), size=7)
def mode_filter(c, labels, size):
    return np.asarray(Image.fromarray(labels.astype(np.uint8)).filter(ImageFilter.ModeFilter(size))).astype(int)

@node('boundary', inputs=('labels',))
def boundary(c, labels): return M2.boundary(labels)

@node('distance', inputs=('mask',))
def distance(c, mask): return ndi.distance_transform_edt(mask)

@node('morph', inputs=('mask',), op='dilation', iterations=1)
def morph(c, mask, op, iterations):
    return getattr(ndi, 'binary_'+op)(mask, iterations=iterations)

@node('relabel', inputs=('labels',), outputs=('labels', 'n'))
def relabel(c, labels):
    _, inv = np.unique(labels, return_inverse=True); inv = inv.reshape(labels.shape); return {'labels': inv, 'n': inv.max()+1}

@node('mean_by_label', inputs=('image', 'labels'), n=None)
def mean_by_label(c, image, labels, n):
    return M2.means(F(image), labels, int(n) if n is not None else int(labels.max())+1)

@node('pil_gray', inputs=('image',))
def pil_gray(c, image): return np.asarray(P(image).convert('L'))

# ----------------------------------------------------------------- generators
@node('noise', scale=10.0, octaves=4, seed=0, h=None, w=None)
def noise(c, scale, octaves, seed, h, w):
    """Fractal noise, zero mean and unit spread."""
    return M.fbm(h or c.H, w or c.W, scale, octaves, seed)

@node('paper_tooth')
def paper_tooth(c):
    """The scanned cold-press paper, tiled."""
    return M.paper_tooth(c.H, c.W)

@node('paper_ground', seed=0)
def paper_ground(c, seed):
    """The poster's cream stock, with its tooth."""
    return M2._paper_ground(c.H, c.W, seed)

@node('canvas', period=5.0, seed=0, scale=1)
def canvas(c, period, seed, scale):
    """Canvas weave."""
    return M.canvas(c.H*scale, c.W*scale, period, seed)

@node('grain', seed=0, size=1.0)
def grain(c, seed, size):
    """Film grain, unit spread."""
    return M2._grain(c.H, c.W, seed, size)

@node('jitter_points', inputs=('rng',), outputs=('points', 'rng'), spacing=8.0, jitter=0.45)
def jitter_points(c, rng, spacing, jitter):
    """A jittered grid of points: even, never regular."""
    return {'points': M2.jitter_grid(c.H, c.W, spacing, rng, jitter), 'rng': rng}

@node('shuffle', inputs=('points', 'rng'), outputs=('points', 'rng'))
def shuffle(c, points, rng):
    rng.shuffle(points); return {'points': points, 'rng': rng}

@node('voronoi', inputs=('points',))
def voronoi(c, points): return M2.voronoi(c.H, c.W, points)

@node('streamlines', inputs=('tx', 'ty', 'density'), dmin=2.0, dmax=10.0, tmin=0.0, seed=0, maxlen=500, minlen=8)
def streamlines(c, tx, ty, density, dmin, dmax, tmin, seed, maxlen, minlen):
    """Evenly spaced streamlines along the flow, closer where `density` is high."""
    d = density if np.ndim(density) else np.full((c.H, c.W), float(density))
    return M2.trace_streamlines(tx, ty, d, dmin, dmax, tmin, seed, maxlen=maxlen, minlen=minlen)

@node('bayer', inputs=('like',), n=4)
def bayer(c, like, n):
    """An ordered-dither threshold map the size of `like`."""
    gh, gw = like.shape[:2]; b = M2._bayer(n); return np.tile(b, (gh//n+1, gw//n+1))[:gh, :gw]

# ----------------------------------------------------------------- operators
@node('smear', inputs=('x', 'tx', 'ty'), length=6, step=1.0, taper=True)
def smear(c, x, tx, ty, length, step, taper):
    """Drag values along the flow (line-integral convolution)."""
    return M.lic(F(x), tx, ty, length, step, taper)

@node('strokes', inputs=('tx', 'ty'), width=2.0, density=0.005, length=16, seed=0, amp_var=0.6)
def strokes(c, tx, ty, width, density, length, seed, amp_var):
    """A field of tapered strokes along the flow: sparse dabs, `width` across, dragged `length` each way."""
    return M.brush(c.H, c.W, tx, ty, width, density, length, seed, amp_var)

@node('kuwahara', inputs=('image',), r=4)
def kuwahara(c, image, r): return M.kuwahara(F(image), r)

@node('light', inputs=('height',), outputs=('diff', 'spec'), strength=1.0, L=[-0.55, -0.75, 0.9], shin=24)
def light(c, height, strength, L, shin):
    """Light a height field from the upper left: diffuse (1 on flat ground) and specular."""
    d, s = M.light(height, strength, L=tuple(L), shin=shin); return {'diff': d, 'spec': s}

@node('cast_shadow', inputs=('height',), L=[-0.55, -0.75], reach=6, k=0.9)
def cast_shadow(c, height, L, reach, k): return M.cast_shadow(height, L=tuple(L), reach=reach, k=k)

@node('shift', inputs=('x',), dy=0.0, dx=0.0, order=1)
def shift(c, x, dy, dx, order):
    x = F(x); return ndi.shift(x, (dy, dx) + (0,)*(x.ndim-2), order=order, mode='nearest') if order else ndi.shift(x, (dy, dx), order=0)

@node('warp', inputs=('x', 'dy', 'dx'), order=0)
def warp(c, x, dy, dx, order):
    """Resample x at (y+dy, x+dx)."""
    yy, xx = np.mgrid[0:c.H, 0:c.W].astype(float)
    return ndi.map_coordinates(x, [yy+dy, xx+dx], order=order, mode='nearest')

@node('film_glow', inputs=('e',), bloom=1.0, halation=1.0, bleed=1.0)
def film_glow(c, e, bloom, halation, bleed):
    """What film does to bright light: bloom, red-orange halation, and a wide bleed."""
    return M2._film_glow(e, bloom, halation, bleed)

@node('separate_cmyk', inputs=('image',), outputs=('c', 'm', 'y', 'k'), ucr=0.8, rich=0.55)
def separate_cmyk(c, image, ucr, rich):
    """CMYK with grey replacement and a rich-black underlay."""
    a = M2._separate(F(image), ucr, rich); return dict(zip('cmyk', a))

@node('palette_mode', inputs=('image',), mode='wheel')
def palette_mode(c, image, mode):
    """A palette mode: screen, wheel, limited, earth, nocturne, fluoro, duotone, inverse."""
    return np.asarray(PAL.MODES[mode](P(image)).convert('RGB'))

@node('pixelate_down', inputs=('image',), outputs=('out', 'gh', 'gw'), size=3, rounding='ceil', gray=False)
def pixelate_down(c, image, size, rounding, gray):
    """Average the image into cells of `size` pixels (a box filter)."""
    H, W = c.H, c.W
    if isinstance(rounding, str) and rounding.startswith('rows:'):
        gh = int(rounding[5:]); gw = int(round(W/(H/gh)))
    elif rounding == 'ceil': gh, gw = int(np.ceil(H/size)), int(np.ceil(W/size))
    elif rounding == 'floor': gh, gw = H//size, W//size
    else: gh, gw = int(rounding[0]), int(rounding[1])
    im = P(image).convert('L' if gray else 'RGB').resize((gw, gh), Image.BOX)
    return {'out': np.asarray(im).astype(float)/255, 'gh': gh, 'gw': gw}

@node('pixelate_up', inputs=('small',), size=3)
def pixelate_up(c, small, size):
    return np.repeat(np.repeat(small, size, 0), size, 1)[:c.H, :c.W]

# ----------------------------------------------------------------- watercolour pencil (Lu, Xu & Jia strokes; pigment pooling)
@node('mediancut', inputs=('image',), outputs=('labels', 'palette'), K=12, mode=3)
def mediancut(c, image, K, mode):
    I = F(image)
    q = Image.fromarray((I*255).astype(np.uint8)).quantize(colors=K, method=Image.Quantize.MEDIANCUT).filter(ImageFilter.ModeFilter(mode))
    return {'labels': np.asarray(q).astype(int), 'palette': np.array(q.getpalette()[:3*K]).reshape(-1, 3)/255.}

@node('fftblur', inputs=('x',), s=1.0)
def fftblur(c, x, s):
    x = F(x)
    return np.stack([SK.fftblur(x[..., i], s) for i in range(x.shape[-1])], -1) if x.ndim == 3 else SK.fftblur(x, s)

@node('to_pigment', inputs=('image',), amount=0.7, enabled=True)
def to_pigment(c, image, amount, enabled):
    """Pull electric screen colours towards real pigments of the same hue."""
    return PA.to_pigment(F(image), amount) if enabled else F(image)

@node('count_colours', inputs=('image',), cap=12)
def count_colours(c, image, cap):
    """How many distinct colours the image has (capped)."""
    return min(cap, len(P(image).convert('RGB').getcolors(1 << 24)))

@node('threshold_map', inputs=('like', 'rng'), outputs=('out', 'rng'), kind='8x8')
def threshold_map(c, like, rng, kind):
    """A dither threshold per cell: Bayer 2x2, 4x4, 8x8, or random."""
    gh, gw = like.shape[:2]
    if kind == 'random': return {'out': rng.random((gh, gw)), 'rng': rng}
    n_ = int(kind[0]); b = M2._bayer(n_); return {'out': np.tile(b, (gh//n_+1, gw//n_+1))[:gh, :gw], 'rng': rng}

@node('pool', inputs=('labels', 'E'), strength=0.40, reach=3.5)
def pool(c, labels, E, strength, reach):
    """Pigment pooling at the edge of every wash: denser paint near each region's border."""
    dens = np.ones(labels.shape)
    for i in np.unique(labels):
        m = labels == i; d = ndi.distance_transform_edt(np.pad(m, 1))[1:-1, 1:-1]
        dens = np.where(m, 1+strength*(1-0.7*E)*np.exp(-d/reach), dens)
    return dens

@node('pencil_strokes', inputs=('x',), ks=7, ndir=8, boost=1.0)
def pencil_strokes(c, x, ks, ndir, boost):
    """Lu, Xu & Jia (2012): each edge drawn as a short line along its own direction."""
    return SK.strokes(x, ks, ndir, boost)

@node('pencil_texture', angle=45.0, length=26, seed=0)
def pencil_texture(c, angle, length, seed):
    """Graphite hatching texture: noise dragged along one direction."""
    return SK.pencil_texture(c.H, c.W, angle, length, seed)

@node('tone', inputs=('P', 'J'))
def tone(c, P, J): return SK.tone(P, J)

@node('wcp_paper')
def wcp_paper(c): return DR.PAPER
@node('wcp_texture')
def wcp_texture(c): return DR.texture(c.H, c.W)

# ----------------------------------------------------------------- marks
@node('stamp_dots', inputs=('points', 'rng'), outputs=('out', 'rng'), SS=3, rad=[0.6, 1.05], ecc=[0.85, 1.15], ink=[0.75, 1.0])
def stamp_dots(c, points, rng, SS, rad, ecc, ink):
    """Hand-drawn dots: each a slightly different ellipse of ink (drawn supersampled)."""
    cv = Image.new('L', (c.W*SS, c.H*SS), 0); dr = ImageDraw.Draw(cv)
    for (x, y) in points:
        r_ = rng.uniform(*rad)*SS; ex = rng.uniform(*ecc)
        dr.ellipse([x*SS-r_*ex, y*SS-r_/ex, x*SS+r_*ex, y*SS+r_/ex], fill=int(255*rng.uniform(*ink)))
    return {'out': np.asarray(cv.resize((c.W, c.H), Image.LANCZOS)).astype(float)/255, 'rng': rng}

@node('keep_points', inputs=('points', 'rng', 'p'), outputs=('points', 'rng'))
def keep_points(c, points, rng, p):
    """Keep each point with probability p (a field sampled at the point)."""
    ix = np.clip(points[:, 0].astype(int), 0, c.W-1); iy = np.clip(points[:, 1].astype(int), 0, c.H-1)
    keep = rng.random(len(points)) < p[iy, ix]
    return {'points': points[keep], 'rng': rng}

@node('impact_dots', inputs=('tone', 'rng'), outputs=('out', 'rng'), pitch=3.2, bands=1.0, SS=3)
def impact_dots(c, tone, rng, pitch, bands, SS):
    """A 9-pin print head: bands eight dots tall, ordered dither, ribbon wearing down each band,
    each pass landing slightly off the last. `bands` scales the banding (0 = none)."""
    H, W = c.H, c.W; r = rng
    bay = np.array([[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]])/16 + 1/32
    gh, gw = int(H/pitch), int(W/pitch); gy, gx = np.mgrid[0:gh, 0:gw]; band = gy//8; row = gy % 8
    jit = r.normal(0, 0.35, band.max()+1)*bands
    ys = gy*pitch + pitch/2 + (band*0.25*bands); xs = gx*pitch + pitch/2 + jit[band]*pitch
    t = ndi.map_coordinates(tone, [np.clip(ys, 0, H-1), np.clip(xs, 0, W-1)], order=1)
    on = t > bay[gy % 4, gx % 4]
    wear = 1 - bands*(0.28*(row/7) + 0.08*(band % 5 == 4))
    cv = Image.new('L', (W*SS, H*SS), 0); dr = ImageDraw.Draw(cv); rad = pitch*0.62*SS
    for y, x, w in zip(ys[on], xs[on], wear[on]):
        x2, y2 = x*SS + r.normal(0, 0.25*SS), y*SS + r.normal(0, 0.25*SS)
        dr.ellipse([x2-rad, y2-rad, x2+rad, y2+rad], fill=int(255*w*r.uniform(0.82, 1.0)))
    return {'out': np.asarray(cv.resize((W, H), Image.LANCZOS)).astype(float)/255, 'rng': rng}

@node('stones', inputs=('palette', 'rng'), outputs=('mask', 'colour', 'rng'), rate=0.0006, rad=[1.5, 5])
def stones(c, palette, rng, rate, rad):
    """Marbling 'stones': drops of ink in the palette's colours, scattered last."""
    H, W = c.H, c.W; r = rng; n = int(H*W*rate); st = np.zeros((H, W)); col = np.zeros((H, W, 3))
    for (x, y) in r.uniform(0, [W, H], (n, 2)):
        rr = r.uniform(*rad); ix, iy = int(x), int(y); R = int(rr+2)
        y0, y1, x0, x1 = max(0, iy-R), min(H, iy+R+1), max(0, ix-R), min(W, ix+R+1)
        d = np.hypot(*np.mgrid[y0-y:y1-y, x0-x:x1-x]); m = d < rr
        st[y0:y1, x0:x1][m] = 1; col[y0:y1, x0:x1][m] = palette[r.integers(len(palette))]
    return {'mask': st, 'colour': col, 'rng': rng}

@node('stitches', inputs=('points', 'rng', 'tx', 'ty', 'labels', 'palette', 'base'), outputs=('out', 'rng'), SS=2, length=[5, 9], light=[-0.6, -0.8])
def stitches(c, points, rng, tx, ty, labels, palette, base, SS, length, light):
    """Satin stitches along the flow, each a raised twist of thread with a direction-dependent sheen."""
    H, W = c.H, c.W; r = rng
    cv = Image.fromarray((np.clip(base, 0, 1)*255).astype(np.uint8)); dr = ImageDraw.Draw(cv); ldir = np.array(light)
    for (x, y) in points:
        ix, iy = int(min(max(x, 0), W-1)), int(min(max(y, 0), H-1))
        a = np.arctan2(ty[iy, ix], tx[iy, ix]) + r.normal(0, 0.12)
        ln = r.uniform(*length); dx, dy = np.cos(a)*ln, np.sin(a)*ln
        col = palette[labels[iy, ix]]
        sheen = 0.78 + 0.36*abs(np.cos(a)*ldir[0]+np.sin(a)*ldir[1])
        p0 = ((x-dx)*SS, (y-dy)*SS); p1 = ((x+dx)*SS, (y+dy)*SS)
        dr.line([p0, p1], fill=tuple(int(v*255) for v in np.clip(col*0.55, 0, 1)), width=int(2.6*SS))
        dr.line([p0, p1], fill=tuple(int(v*255) for v in np.clip(col*sheen, 0, 1)), width=int(1.8*SS))
        o = (-np.sin(a)*0.5*SS, np.cos(a)*0.5*SS)
        dr.line([(p0[0]-o[0], p0[1]-o[1]), (p1[0]-o[0], p1[1]-o[1])], fill=tuple(int(v*255) for v in np.clip(col*sheen*1.18+0.04, 0, 1)), width=1)
    return {'out': np.asarray(cv.resize((W, H), Image.LANCZOS)), 'rng': rng}

@node('pen_draw', inputs=('image', 'lines_a', 'lines_b'), paper=[0.97, 0.96, 0.93], mono=None, SS=3)
def pen_draw(c, image, lines_a, lines_b, paper, mono, SS):
    """Draw polylines with fineliners: each line in the pen nearest the image's colour along it."""
    I = F(image); H, W = c.H, c.W
    mx = I.max(-1); sat = (mx-I.min(-1))/(mx+1e-6)
    if mono is None: mono = sat.mean() < 0.12
    names = ['black'] if mono else list(M2.PENS); pc = np.array([M2.PENS[n] for n in names])
    cv = Image.new('RGB', (W*SS, H*SS), tuple(int(v*255) for v in paper)); dr = ImageDraw.Draw(cv)
    for line in list(lines_a) + list(lines_b):
        xs = np.array([p[0] for p in line]); ys = np.array([p[1] for p in line])
        col = I[np.clip(ys.astype(int), 0, H-1), np.clip(xs.astype(int), 0, W-1)].mean(0)
        k = 0 if mono else int(np.argmin(((pc - col)**2).sum(1) + 0.15*(np.arange(len(pc)) == 0)))
        dr.line([(x*SS, y*SS) for x, y in line[::2]], fill=tuple(int(v*255) for v in pc[k]), width=int(1.1*SS), joint='curve')
    return np.asarray(cv.resize((W, H), Image.LANCZOS))

@node('mono_tone', inputs=('image', 'L'))
def mono_tone(c, image, L):
    """The pen plotter's density: darkness for monochrome renders, darkness plus colour otherwise."""
    I = F(image); mx = I.max(-1); sat = (mx-I.min(-1))/(mx+1e-6)
    return (1-L) if sat.mean() < 0.12 else np.clip((1-L)*0.75 + 0.55*sat*mx, 0, 1)

@node('trails', inputs=('lines', 'rng', 'colour', 'weight'), outputs=('acc', 'rng'), amp=[0.5, 1.0], speed=[2, 9], swing=0.25, floor=0.25, scale=0.5)
def trails(c, lines, rng, colour, weight, amp, speed, swing, floor, scale):
    """Light trails: each streamline exposed as a line of light, its brightness wavering with the
    hand's speed; light accumulates where trails cross."""
    H, W = c.H, c.W; r = rng; acc = np.zeros((H, W, 3))
    for line in lines:
        p = np.array(line); seg = np.diff(p, axis=0); n = np.maximum(np.hypot(*seg.T)/0.5, 1).astype(int)
        q = np.concatenate([p[i] + np.outer(np.linspace(0, 1, k, endpoint=False), seg[i]) for i, k in enumerate(n)])
        xi = np.clip(q[:, 0].astype(int), 0, W-1); yi = np.clip(q[:, 1].astype(int), 0, H-1)
        a = r.uniform(*amp)*((1-swing)+swing*np.sin(np.linspace(0, r.uniform(*speed), len(q)) + r.uniform(0, 6)))
        w = (a*(floor+(1-floor)*weight[yi, xi]))[:, None]*colour[yi, xi]
        np.add.at(acc, (yi, xi), w*scale)
    return {'acc': acc, 'rng': rng}

@node('trails_solid', inputs=('lines', 'rng', 'colour', 'weight', 'E'), outputs=('core', 'rng'),
      dmin=2.2, dmax=12.0, widths=[0.8, 1.4, 2.4, 3.8, 6.0])
def trails_solid(c, lines, rng, colour, weight, E, dmin, dmax, widths):
    """Light trails packed edge to edge, each as wide as half the gap to its neighbours, so the
    field is lit without gaps; thin in the filaments, broad where calm."""
    H, W = c.H, c.W; r = rng; widths = np.array(widths); accs = [np.zeros((H, W, 3)) for _ in widths]
    for line in lines:
        p = np.array(line); seg = np.diff(p, axis=0); n = np.maximum(np.hypot(*seg.T)/0.5, 1).astype(int)
        q = np.concatenate([p[i] + np.outer(np.linspace(0, 1, k, endpoint=False), seg[i]) for i, k in enumerate(n)])
        xi = np.clip(q[:, 0].astype(int), 0, W-1); yi = np.clip(q[:, 1].astype(int), 0, H-1)
        e = E[yi, xi].mean()
        w = (dmin + (dmax-dmin)*(1-e))/2*r.uniform(0.85, 1.15)
        k = int(np.argmin(np.abs(widths-w)))
        a = r.uniform(0.7, 1.0)*(0.8+0.2*np.sin(np.linspace(0, r.uniform(2, 8), len(q)) + r.uniform(0, 6)))
        wgt = (a*(0.08+0.92*weight[yi, xi]))[:, None]*colour[yi, xi]
        np.add.at(accs[k], (yi, xi), wgt*0.5*(widths[k]/0.8)**0.85)
    return {'core': sum(M.blur(a_, max(wk*0.55, 0.6)) for a_, wk in zip(accs, widths)), 'rng': rng}

@node('tiles_along', inputs=('lines', 'rng'), outputs=('points', 'rng'), s=11.0)
def tiles_along(c, lines, rng, s):
    """Set tile centres along each row, a tile's length apart (give or take)."""
    r = rng; pts = []
    for line in lines:
        off = r.uniform(0, s); acc = 0.0; prev = line[0]
        for p in line[1:]:
            acc += np.hypot(p[0]-prev[0], p[1]-prev[1]); prev = p
            if acc >= s*r.uniform(0.85, 1.25): pts.append(p); acc = 0.0
    return {'points': np.array(pts), 'rng': rng}

@node('smalti', inputs=('colours', 'rng'), outputs=('colours', 'rng'), K=28, vary=0.05)
def smalti(c, colours, rng, K, vary):
    """Snap a set of colours to K glass colours, each piece varying slightly."""
    q = Image.fromarray((colours[None]*255).astype(np.uint8)).quantize(K, method=Image.Quantize.MEDIANCUT)
    s = np.array(q.getpalette()).reshape(-1, 3)[np.asarray(q)[0]]/255
    return {'colours': np.clip(s*(1+vary*rng.standard_normal((len(colours), 1))), 0, 1), 'rng': rng}

@node('scratches', inputs=('image', 'rng'), outputs=('out', 'rng'), count=[1, 4], depth=0.25, width=0.6)
def scratches(c, image, rng, count, depth, width):
    """Fine vertical scratches along a strip of film."""
    out = F(image).copy(); r = rng
    for _ in range(r.integers(*count)):
        x = r.uniform(0, c.W); xx = np.arange(c.W)
        out *= (1 - depth*np.exp(-((xx-x)/width)**2))[None, :, None]*np.ones((c.H, 1, 1))
    return {'out': out, 'rng': rng}
