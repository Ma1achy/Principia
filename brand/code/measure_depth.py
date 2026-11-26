#!/usr/bin/env python3
"""Measures, and only measures, whether the 2D title draws the orbit in front of or behind the letters as the orbit's
own depth says.

    python3 measure_depth.py OUT_DIR [--web DIR] [--every 18]

For frames sampled across the 36 s loop (every 18th of the 1080 at 30 fps) of the light and dark subtitle titles, and of
the page-colour versions (the same script drawn on #ffffff and #0d1117), it takes from the title's own script the
orbit's 3D points (three bodies, three trails of 220 points) and the camera, and works out from the geometry alone
which of them fall on the letters' solid area on screen (a point at least 3 px inside the letter, found from the traced
outlines) and whether each is in front of the letters' plane (z > 0) or behind it (z < 0). Then it reads the rendered
pixel there:
  in front  the pixel should differ from the letter's colour (the body or trail shows);
  behind    the pixel should be the letter's colour (the letter hides it), within 40 levels: the letter face takes
            a faint light pool from a body behind it, at most 12 % opacity, so up to about 31 levels, plus rounding.
A behind point whose pixel is further than that is a mismatch; it is "explained" if a front body or trail point is
within 70 px (their glow and the light pool they cast, up to 100 px, lie on the letter there), "unexplained" otherwise.
A front point whose pixel is within 24 levels of the letter's colour is "indistinguishable": the title's blend modes
(multiply on light, screen on dark) make a trail over a letter of nearly its own colour show little; it is counted, not
called a mismatch. Also reported: points within 0.02 of the plane, points with z = 0 exactly, plane crossings near a
letter's edge, and whether the glow of a trail behind the letter shows just past the letter's edge.
Nothing in the drawing is changed by this script.
"""
import argparse, json, os, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
DATA = os.path.join(HERE, 'python', 'data')
T_BEHIND, T_VISIBLE, R_EXPLAIN, INSIDE_PX, NEAR_Z = 40, 24, 70, 3, 0.02
T_LOCAL, RING = 12, 10      # a behind point whose pixel stands out from the letter 10 px around it by more than 12 levels shows through
VARIANTS = [  # name, script, backdrop, letter colour, page colour
    ('subtitle-light', 'title-2d-subtitle-light', None, (0x1d, 0x1b, 0x19), (255, 255, 255)),
    ('subtitle-dark', 'title-2d-subtitle-dark', None, (0xef, 0xe9, 0xdc), (0x0d, 0x11, 0x17)),
    ('github-light', 'title-2d-subtitle-light', 'ffffff', (0x1d, 0x1b, 0x19), (255, 255, 255)),
    ('github-dark', 'title-2d-subtitle-dark', '0d1117', (0xef, 0xe9, 0xdc), (0x0d, 0x11, 0x17))]

NODE = r"""
const fs=require('fs'); const [file,name,out,tlist]=process.argv.slice(1); const times=JSON.parse(tlist);
let js=fs.readFileSync(file,'utf8').replace('const o = orb(t)','globalThis.__orb = orb; globalThis.__pj = pj; globalThis.__fs=fs; globalThis.__cam=[CX,CY,K]; const o = orb(t)');
delete globalThis.PrincipiaMark; new Function(js).call(globalThis);
PrincipiaMark[name].svgAt(6,{});
const res=[];
for (const t of times){ const o=__orb(t);
  res.push({t, bodies:o.bodies.map(b=>({i:b.i,p:b.p,s:__pj(b.p)})), trails:o.trails.map(tr=>({i:tr.i,pts:tr.pts.map(p=>({p,s:__pj(p)}))}))}); }
fs.writeFileSync(out, JSON.stringify({cam:__cam,fs:__fs,frames:res}));
"""


def letter_mask(cam, size=(1600, 360), erode=INSIDE_PX):
    """Boolean mask of the letters' solid area on screen, and the polygons (screen px) for edge distances."""
    cx, cy, k = cam
    g = json.load(open(os.path.join(DATA, 'pgeo.json')))
    loops = [(g['out'], False), (g['hole'], True)] + [(q['pts'], q['hole']) for q in json.load(open(os.path.join(DATA, 'word_loops.json')))[2:]]
    polys = [([(cx + k * x, cy - k * y) for x, y in pts], hole) for pts, hole in loops]
    im = Image.new('L', size, 0)
    d = ImageDraw.Draw(im)
    for pts, hole in polys:
        if not hole:
            d.polygon(pts, fill=255)
    for pts, hole in polys:
        if hole:
            d.polygon(pts, fill=0)
    inner = im.filter(ImageFilter.MinFilter(2 * erode + 1))
    return np.asarray(inner) > 0, [np.array(p) for p, h in polys]


def seg_dist(pt, polys):
    best = 1e9
    for poly in polys:
        a, b = poly, np.roll(poly, -1, axis=0)
        ab = b - a
        t = np.clip(((pt - a) * ab).sum(1) / np.maximum((ab * ab).sum(1), 1e-12), 0, 1)
        best = min(best, float(np.sqrt(((pt - (a + t[:, None] * ab)) ** 2).sum(1)).min()))
    return best


def cases(frame, fsign, mask, polys):
    """Every body and trail point on the letters' solid area, with its depth side, from geometry alone."""
    out, front_pts, crossings, zero, near = [], [], [], 0, 0
    objs = [('body', b['i'], b['p'], b['s']) for b in frame['bodies']]
    for tr in frame['trails']:
        pts = tr['pts']
        objs += [('trail', tr['i'], q['p'], q['s']) for q in pts]
        for q0, q1 in zip(pts, pts[1:]):
            if (q0['p'][2] > 0) != (q1['p'][2] > 0):
                u = q0['p'][2] / (q0['p'][2] - q1['p'][2])
                crossings.append((q0['s'][0] + u * (q1['s'][0] - q0['s'][0]), q0['s'][1] + u * (q1['s'][1] - q0['s'][1])))
    for kind, i, p, s in objs:
        if abs(p[2]) < 1e-9:
            zero += 1
        if abs(p[2]) < NEAR_Z:
            near += 1
        if p[2] * fsign > 0:
            front_pts.append((s[0], s[1]))
    for kind, i, p, s in objs:
        x, y = int(round(s[0])), int(round(s[1]))
        if 0 <= y < mask.shape[0] and 0 <= x < mask.shape[1] and mask[y, x] and abs(p[2]) >= NEAR_Z:
            out.append({'kind': kind, 'i': i, 'x': s[0], 'y': s[1], 'z': p[2], 'front': p[2] * fsign > 0})
    return out, np.array(front_pts) if front_pts else np.zeros((0, 2)), crossings, zero, near


def run(out_dir, web, every):
    import render
    wd = os.path.join(out_dir, 'render')
    render.make_render_dir(wd, web)
    idx = list(range(0, 1080, every))
    times = [6 + i / 30 for i in idx]
    report, worst = [], {}
    for name, script, backdrop, letter, page in VARIANTS:
        fr = os.path.join(out_dir, 'frames', name)
        render.frames(wd, script, 30, fr, only=set(idx), backdrop=backdrop)
        dump = os.path.join(out_dir, f'geometry-{script}.json')
        if not os.path.exists(dump):
            subprocess.check_call(['node', '-e', NODE, os.path.join(web, f'principia-{script}.js'), script, dump, json.dumps(times)])
        g = json.load(open(dump))
        mask, polys = letter_mask(g['cam'])
        stats = dict(front=0, front_indist=0, behind=0, behind_bad=0, behind_expl=0, behind_unexpl=0, near_plane=0, exact_zero=0,
                     local_checked=0, local_leaks=0, crossings=0, crossings_near_edge=0, edge_cases=0, edge_bad=0, glow_checked=0, glow_visible=0)
        deltas_front, deltas_behind, bad_list, local_ex, leaks = [], [], [], [], []
        for fi, frame in zip(idx, g['frames']):
            im = np.asarray(Image.open(os.path.join(fr, f'f{fi:05d}.png')).convert('RGBA')).astype(float)
            a = im[..., 3:4] / 255
            rgb = im[..., :3] * a + np.array(page, float) * (1 - a)
            cs, front_pts, crossings, zero, near = cases(frame, g['fs'], mask, polys)
            stats['near_plane'] += near; stats['exact_zero'] += zero
            near_edge = [c for c in crossings if seg_dist(np.array(c), polys) <= 6]
            stats['crossings'] += len(crossings); stats['crossings_near_edge'] += len(near_edge)
            for c in cs:
                px = rgb[int(round(c['y'])), int(round(c['x']))]
                delta = float(np.abs(px - np.array(letter, float)).max())
                if c['front']:
                    stats['front'] += 1; deltas_front.append(delta)
                    stats['front_indist'] += delta < T_VISIBLE
                else:
                    stats['behind'] += 1; deltas_behind.append(delta)
                    ring = [rgb[int(round(c['y'] + RING * np.sin(th))), int(round(c['x'] + RING * np.cos(th)))] for th in np.linspace(0, 2 * np.pi, 16, endpoint=False)
                            if 0 <= int(round(c['y'] + RING * np.sin(th))) < mask.shape[0] and 0 <= int(round(c['x'] + RING * np.cos(th))) < mask.shape[1]
                            and mask[int(round(c['y'] + RING * np.sin(th))), int(round(c['x'] + RING * np.cos(th)))]]
                    if len(ring) >= 6:
                        stats['local_checked'] += 1
                        ex = float(np.abs(px - np.median(np.array(ring), axis=0)).max())
                        stats['local_leaks'] += ex > T_LOCAL
                        local_ex.append(ex)
                        if ex > T_LOCAL:
                            leaks.append((ex, name, fi, c))
                    near_front = float(np.sqrt(((front_pts - [c['x'], c['y']]) ** 2).sum(1)).min()) if len(front_pts) else 1e9
                    if delta > T_BEHIND:
                        stats['behind_bad'] += 1
                        explained = near_front <= R_EXPLAIN
                        stats['behind_expl' if explained else 'behind_unexpl'] += 1
                        bad_list.append((delta, name, fi, c, near_front, explained))
                    if any(np.hypot(c['x'] - ex, c['y'] - ey) <= 12 for ex, ey in near_edge):
                        stats['edge_cases'] += 1
                        stats['edge_bad'] += delta > T_BEHIND
            # the behind trail's glow past the letter's edge: behind points 2 to 5 px outside the letter
            for tr in frame['trails']:
                for q in tr['pts']:
                    if q['p'][2] * g['fs'] < -NEAR_Z:
                        x, y = int(round(q['s'][0])), int(round(q['s'][1]))
                        if 0 <= y < 360 and 0 <= x < 1600 and not mask[y, x]:
                            d = seg_dist(np.array([q['s'][0], q['s'][1]]), polys)
                            inside_full = False
                            if 2 <= d <= 5:
                                stats['glow_checked'] += 1
                                stats['glow_visible'] += float(np.abs(rgb[y, x] - np.array(page, float)).max()) > 12
        stats['local_median'] = float(np.median(local_ex)) if local_ex else 0.0
        stats['local_p99'] = float(np.percentile(local_ex, 99)) if local_ex else 0.0
        leaks.sort(key=lambda l: -l[0]); stats['worst_leak'] = leaks[0] if leaks else None
        report.append((name, stats, deltas_front, deltas_behind))
        bad_list.sort(key=lambda b: (b[5], -b[0]))
        worst[name] = bad_list[0] if bad_list else None
        if bad_list:
            d, nm, fi, c, nf, ex = bad_list[0]
            im = Image.open(os.path.join(out_dir, 'frames', nm, f'f{fi:05d}.png')).convert('RGBA')
            bg = Image.new('RGBA', im.size, page + (255,)); bg.alpha_composite(im)
            x, y = int(c['x']), int(c['y'])
            bg.crop((x - 40, y - 40, x + 40, y + 40)).resize((320, 320), Image.NEAREST).convert('RGB').save(os.path.join(out_dir, f'worst-{name}.png'))
    lines = []
    for name, st, df, db in report:
        df, db = np.array(df or [0]), np.array(db or [0])
        lines.append(f"{name}: {st['front'] + st['behind']} points on the letters' solid area ({st['front']} in front, {st['behind']} behind), "
                     f"{len(idx)} frames; front: {st['front'] - st['front_indist']} differ from the letter by at least {T_VISIBLE} levels, "
                     f"{st['front_indist']} are indistinguishable from it (median difference {np.median(df):.0f}); behind: {st['behind'] - st['behind_bad']} within {T_BEHIND} of the letter "
                     f"(median {np.median(db):.0f}, 95th percentile {np.percentile(db, 95):.0f}), {st['behind_bad']} further ({st['behind_expl']} explained by a front object within {R_EXPLAIN} px, "
                     f"{st['behind_unexpl']} unexplained); local test (the pixel against the letter 10 px around it, 12 levels): {st['local_checked']} behind points checked, {st['local_leaks']} stand out "
                     f"(median {st['local_median']:.1f}, 99th percentile {st['local_p99']:.1f}); within {NEAR_Z} of the plane: {st['near_plane']}; exactly z = 0: {st['exact_zero']}; plane crossings: {st['crossings']}, "
                     f"{st['crossings_near_edge']} within 6 px of a letter's edge, {st['edge_cases']} test points within 12 px of those with {st['edge_bad']} mismatches; "
                     f"glow of behind trails 2 to 5 px past a letter's edge: {st['glow_visible']} of {st['glow_checked']} visible")
        if st['worst_leak']:
            lk = st['worst_leak']
            lines.append(f"   most prominent behind point by the local test: frame {lk[2]} (t = {6 + lk[2] / 30:.2f} s), {lk[3]['kind']} {lk[3]['i']} at ({lk[3]['x']:.0f}, {lk[3]['y']:.0f}) z = {lk[3]['z']:.3f}, {lk[0]:.0f} levels")
        w = worst[name]
        if w:
            lines.append(f"   worst behind mismatch: frame {w[2]} (t = {6 + w[2] / 30:.2f} s), body/trail {w[3]['kind']} {w[3]['i']}, at ({w[3]['x']:.0f}, {w[3]['y']:.0f}) z = {w[3]['z']:.3f}, "
                         f"difference {w[0]:.0f}, nearest front object {w[4]:.0f} px ({'explained' if w[5] else 'UNEXPLAINED'}); crop in worst-{name}.png")
    print('\n'.join(lines))
    open(os.path.join(out_dir, 'depth-report.txt'), 'w').write('\n'.join(lines) + '\n')


if __name__ == '__main__':
    ap = argparse.ArgumentParser()
    ap.add_argument('out'); ap.add_argument('--web', default=os.path.join(os.path.dirname(HERE), 'code', 'web')); ap.add_argument('--every', type=int, default=18)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    run(a.out, a.web, a.every)
