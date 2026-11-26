exec(open('clear.py').read().split("res={}")[0])
import json
mo=d['moth'][:-1]                       # (900,3,2)
M_B=0.095                               # body radius + a little
def crossings(psi):
    c,s=np.cos(psi),np.sin(psi)
    out=[]
    for b in range(3):
        p=mo[:,b,:]; X=p[:,0]*c-p[:,1]*s; Y=p[:,0]*s+p[:,1]*c
        Yn=np.roll(Y,-1); Xn=np.roll(X,-1)
        idx=np.where(np.sign(Y)!=np.sign(Yn))[0]
        for i in idx:
            t=Y[i]/(Y[i]-Yn[i]); out.append(X[i]+(Xn[i]-X[i])*t)
    return np.array(out)
def okpt(q):
    xy=np.array([q])
    inhole=Path(HOLE).contains_point(q) and segdist(xy,HOLE)[0]>M_B
    outside=(not po.contains_point(q)) and segdist(xy,OUT)[0]>M_B
    return inhole or outside, inhole
cands=[]
for psi in np.linspace(0,np.pi*2,96,endpoint=False):
    Xc=crossings(psi)
    for phi in np.linspace(0,np.pi,36,endpoint=False):
        dvec=np.array([np.cos(phi),np.sin(phi)])
        for cx in [-0.15,-0.08,0,0.08,0.15]:
            for cy in [0.26,0.31,0.36,0.41,0.46]:
                for S in np.linspace(1.6,3.6,21):
                    good=True; nh=0
                    for X in Xc:
                        ok,ih=okpt(np.array([cx,cy])+S*X*dvec)
                        if not ok: good=False; break
                        nh+=ih
                    if good and nh>0: cands.append((psi,phi,cx,cy,S,nh))
print('crossing-feasible', len(cands))
cands.sort(key=lambda c:(-c[5], abs(c[4]-2.3)+abs(c[2])+abs(c[3]-0.36)))
cands=cands[:500]
print('testing',len(cands),'max holepasses',cands[0][5])
def rotm(ax,a):
    c,s=np.cos(a),np.sin(a)
    if ax=='x': return np.array([[1,0,0],[0,c,-s],[0,s,c]])
    if ax=='y': return np.array([[c,0,s],[0,1,0],[-s,0,c]])
    return np.array([[c,-s,0],[s,c,0],[0,0,1]])
spin,pitch=-0.62,0.17
res=[]
for psi,phi,cx,cy,S,nh in cands:
    dvec=np.array([np.cos(phi),np.sin(phi),0.0])
    for tau in np.linspace(-0.75,0.75,7):
        e=np.array([0,0,1.0]); 
        # tilt the out-of-plane axis about d by tau
        k=dvec; e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
        c,s=np.cos(psi),np.sin(psi)
        X=(mo[...,0]*c-mo[...,1]*s)*S; Y=(mo[...,0]*s+mo[...,1]*c)*S
        W=np.stack([cx+X*dvec[0]+Y*e[0], cy+X*dvec[1]+Y*e[1], X*dvec[2]+Y*e[2]],-1)
        if hits(W[::2].reshape(-1,3),M_B)>0: continue
        if hits(W.reshape(-1,3),M_B)>0: continue
        n=np.cross(dvec,e)
        # camera direction in the P frame: undo pitch then spin
        v=rotm('y',-spin)@rotm('x',-pitch)@np.array([0,0,1.0])
        face=abs(n@v)
        res.append((nh*0.8+face*3-abs(S-2.3)*1.5-abs(cx)-abs(cy-0.36), nh, round(face,2), [float(x) for x in (psi,phi,cx,cy,S,tau)]))
res.sort(key=lambda r:-r[0])
print(len(res))
for r in res[:8]: print(r)
json.dump([r[3] for r in res[:8]],open('hole.json','w'))
