"""Check every preset graph against the hand-written medium it replaces, pixel for pixel."""
import sys, os, time, json, numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__))); sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import graph, nodes
import biggrid as B
from PIL import Image
from concurrent.futures import ProcessPoolExecutor
KEYS = [k for _, c in B.FAMILIES for k, _ in c]
def one(a):
    key, i = a
    im = B.load(B.IMGS[i][0]); src = np.asarray(im)
    t = time.time(); ref = np.asarray(B.fn(key)(im, i).convert('RGB')); t1 = time.time()-t
    t = time.time(); got = graph.render(key, src, seed=i); t2 = time.time()-t
    got = np.asarray(got)
    if got.dtype != np.uint8: return key, i, 'not uint8', 0, t1, t2
    d = np.abs(ref.astype(int)-got.astype(int))
    return key, i, int(d.max()), float((d > 0).mean()), t1, t2
if __name__ == '__main__':
    keys = sys.argv[1].split(',') if len(sys.argv) > 1 else KEYS
    imgs = range(len(B.IMGS)) if len(sys.argv) < 3 else [int(x) for x in sys.argv[2].split(',')]
    jobs = [(k, i) for k in keys for i in imgs]
    res = {}
    with ProcessPoolExecutor(2) as ex:
        for key, i, mx, frac, t1, t2 in ex.map(one, jobs):
            res.setdefault(key, []).append((i, mx, frac, t1, t2))
    for k in keys:
        r = res[k]; worst = max(x[1] if isinstance(x[1], int) else 999 for x in r)
        print(f'{k:30s} max diff {worst:3d}   differing px {max(x[2] for x in r)*100:6.3f}%   {"EXACT" if worst == 0 else ""}')
    json.dump(res, open('verify_result.json', 'w'))
