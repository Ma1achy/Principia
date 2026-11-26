exec(open('clear.py').read().split("res={}")[0])
import json
res=json.load(open('clear.json'))
for key,src,mg,thr in [('eight',d['eight'][:-1,0,:],0.1,(0.6,1.5)),('butterfly',d['butterfly'][:-1].reshape(-1,2),0.08,(0.4,1.5))]:
    best=None
    for th,x0,y0,S,yk in itertools.product(np.linspace(*thr,10),np.linspace(-0.2,0.2,9),np.linspace(0.25,0.45,9),[1.3,1.45,1.6],[0.55,0.7,0.85,1.0]):
        X=src[:,0]*S; Yy=src[:,1]*S*yk
        pts=np.column_stack([x0+X*np.cos(th),y0+Yy,X*np.sin(th)])
        h=hits(pts[::2],mg); score=h*10+(1-yk)*2+abs(x0)+abs(y0-0.36)
        if best is None or score<best[0]: best=(score,h,th,x0,y0,S,yk)
    print(key,best); res[key]=[float(x) for x in best]
json.dump(res,open('clear.json','w'))
