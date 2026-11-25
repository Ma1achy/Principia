import numpy as np, json, re
from svgpathtools import parse_path, Line
from matplotlib.path import Path
import sys
s=open(sys.argv[1]).read()   # the README title SVG, e.g. docs/readme/title-light.svg
d=re.search(r'<path fill="#1d1b19" d="([^"]+)"',s).group(1)
path=parse_path(d)
loops=[]
for sub in path.continuous_subpaths():
    pts=[]
    for seg in sub:
        if isinstance(seg,Line): pts.append(seg.start)
        else:
            for t in np.linspace(0,1,25)[:-1]: pts.append(seg.point(t))
    a=np.array([[(z.real-56.5)/71.5, -(z.imag-78.5)/71.5] for z in pts])
    keep=[a[0]]
    for q in a[1:]:
        if np.linalg.norm(q-keep[-1])>0.004: keep.append(q)
    a=np.array(keep)
    if np.linalg.norm(a[0]-a[-1])<0.004: a=a[:-1]
    loops.append(a)
def area(a): return 0.5*np.sum(a[:,0]*np.roll(a[:,1],-1)-np.roll(a[:,0],-1)*a[:,1])
# nesting: a loop inside an odd number of others is a hole
info=[]
for i,a in enumerate(loops):
    depth=sum(1 for j,b in enumerate(loops) if j!=i and Path(b).contains_point(a.mean(0)) and abs(area(b))>abs(area(a)))
    hole=depth%2==1
    if hole and area(a)>0: a=a[::-1]
    if (not hole) and area(a)<0: a=a[::-1]
    info.append({'pts':a,'hole':hole,'x0':float(a[:,0].min()),'x1':float(a[:,0].max())})
info.sort(key=lambda q:q['x0'])
for q in info: print('hole' if q['hole'] else 'outer', round(q['x0'],2), round(q['x1'],2), len(q['pts']))
json.dump([{'pts':np.round(q['pts'],4).tolist(),'hole':q['hole']} for q in info],open('word_loops.json','w'))
