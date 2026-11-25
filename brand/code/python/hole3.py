exec(open('hole2.py').read().split("cands=[]")[0])
import time; t0=time.time()
cands=[]
Ss=np.linspace(1.6,3.6,21)
for psi in np.linspace(0,np.pi*2,96,endpoint=False):
    Xc=crossings(psi)
    for phi in np.linspace(0,np.pi,36,endpoint=False):
        dvec=np.array([np.cos(phi),np.sin(phi)])
        for cx in [-0.15,-0.08,0,0.08,0.15]:
            for cy in [0.26,0.31,0.36,0.41,0.46]:
                Q=(np.array([cx,cy])[None,None,:]+Ss[:,None,None]*Xc[None,:,None]*dvec[None,None,:]).reshape(-1,2)
                ih=Path(HOLE).contains_points(Q)&(segdist(Q,HOLE)>M_B)
                outside=(~po.contains_points(Q))&(segdist(Q,OUT)>M_B)
                ok=(ih|outside).reshape(len(Ss),-1).all(1); nh=ih.reshape(len(Ss),-1).sum(1)
                for k in np.where(ok&(nh>0))[0]: cands.append((psi,phi,cx,cy,Ss[k],int(nh[k])))
print('stage1',len(cands),round(time.time()-t0,1),'s')
cands.sort(key=lambda c:(-c[5], abs(c[4]-2.3)+abs(c[2])+abs(c[3]-0.36)))
json.dump([list(map(float,c)) for c in cands[:1500]],open('cands.json','w'))
