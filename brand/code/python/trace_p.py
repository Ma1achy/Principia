"""Trace the P (and the whole wordmark) from the README title SVG into polygons, in the P's own units:
x = (X - 56.5) / 71.5, y = -(Y - 78.5) / 71.5  (cap height = 2, centred on the P). Curves are sampled densely."""
import numpy as np, json, re, sys
from svgpathtools import parse_path, Line
from matplotlib.path import Path
TITLE = sys.argv[1] if len(sys.argv) > 1 else 'title-light.svg'
s = open(TITLE).read()

def flatten(d, per_curve, min_step):
    loops = []
    for sub in parse_path(d).continuous_subpaths():
        pts = []
        for seg in sub:
            if isinstance(seg, Line): pts.append(seg.start)
            else: pts += [seg.point(t) for t in np.linspace(0, 1, per_curve + 1)[:-1]]
        a = np.array([[(z.real - 56.5) / 71.5, -(z.imag - 78.5) / 71.5] for z in pts])
        keep = [a[0]]
        for q in a[1:]:
            if np.linalg.norm(q - keep[-1]) > min_step: keep.append(q)
        a = np.array(keep)
        if np.linalg.norm(a[0] - a[-1]) < min_step: a = a[:-1]
        if len(a) >= 3: loops.append(a)
    return loops

def area(a): return 0.5 * np.sum(a[:, 0] * np.roll(a[:, 1], -1) - np.roll(a[:, 0], -1) * a[:, 1])

def orient(loops):
    """outer loops anticlockwise, holes clockwise (a loop inside an odd number of others is a hole)"""
    out = []
    for i, a in enumerate(loops):
        depth = sum(1 for j, b in enumerate(loops) if j != i and abs(area(b)) > abs(area(a)) and Path(b).contains_point(a.mean(0)))
        hole = depth % 2 == 1
        if hole == (area(a) > 0): a = a[::-1]
        out.append({'pts': np.round(a, 5).tolist(), 'hole': hole})
    return out

word = re.search(r'<path fill="#1d1b19" d="([^"]+)"', s).group(1)
P = word[:word.index('M183.492')]                      # first glyph only
p = orient(flatten(P, 96, 0.0015))
outer = [q for q in p if not q['hole']][0]['pts']; hole = [q for q in p if q['hole']][0]['pts']
json.dump({'out': outer, 'hole': hole}, open('data/pgeo.json', 'w'))
json.dump(sorted(orient(flatten(word, 24, 0.004)), key=lambda q: min(x for x, y in q['pts'])), open('data/word_loops.json', 'w'))
subs = re.findall(r'<path fill="#3d3a34" d="([^"]+)"', s)
json.dump(orient([l for d in subs for l in flatten(d, 12, 0.002)]), open('data/sub_loops.json', 'w'))
print('P outline', len(outer), 'points; hole', len(hole), 'points')
