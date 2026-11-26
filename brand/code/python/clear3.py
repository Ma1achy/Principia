exec(open('clear.py').read().split("res={}")[0])
import json
res=json.load(open('clear.json'))
mo=d['moth'][:-1]            # (n,3,2)
allp=mo.reshape(-1,2)        # every body
best=None
for th,x0,y0,S in itertools.product(np.linspace(-1.3,-0.4,19),np.linspace(-0.6,1.0,17),np.linspace(-0.6,0.7,14),[1.2,1.3,1.4,1.5]):
    X=allp[:,0]*S; Yy=allp[:,1]*S
    pts=np.column_stack([x0+X*np.cos(th),y0+Yy,X*np.sin(th)])
    h=hits(pts[::2],0.13)
    if h>5: continue
    h=hits(pts,0.13)
    face=abs(np.cos(th+0.62))
    score=h*10 - face*3 + abs(x0)*0.8 + abs(y0-0.05)*0.8 - S*0.5
    if best is None or score<best[0]: best=(score,h,th,x0,y0,S,1.0)
print('moth',best)
res['moth']=[float(x) for x in best]
json.dump(res,open('clear.json','w'))
