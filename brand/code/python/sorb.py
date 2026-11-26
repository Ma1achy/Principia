import sys, json, time, numpy as np
exec(open('clear.py').read().split("res={}")[0])
name=sys.argv[1]; t0=time.time()
C=np.load('catalog.npz'); arr=C[name][:-1]
stride=max(1,len(arr)//900); mo=arr[::stride]           # (n,3,2)
R=np.linalg.norm(mo,axis=-1).max()
M_B=0.095
def rotm(ax,a):
    c,s=np.cos(a),np.sin(a)
    if ax=='x': return np.array([[1,0,0],[0,c,-s],[0,s,c]])
    if ax=='y': return np.array([[c,0,s],[0,1,0],[-s,0,c]])
    return np.array([[c,-s,0],[s,c,0],[0,0,1]])
v=rotm('y',0.62)@rotm('x',-0.17)@np.array([0,0,1.0])
def crossings(psi):
    c,s=np.cos(psi),np.sin(psi); out=[]
    for b in range(3):
        p=mo[:,b,:]; X=p[:,0]*c-p[:,1]*s; Y=p[:,0]*s+p[:,1]*c; Yn=np.roll(Y,-1); Xn=np.roll(X,-1)
        for i in np.where(np.sign(Y)!=np.sign(Yn))[0]:
            t=Y[i]/(Y[i]-Yn[i]); out.append(X[i]+(Xn[i]-X[i])*t)
    return np.array(out)
def build(P,arr=mo):
    psi,phi,cx,cy,S,tau=P
    dv=np.array([np.cos(phi),np.sin(phi),0.0]); c,s=np.cos(psi),np.sin(psi)
    X=(arr[...,0]*c-arr[...,1]*s)*S; Y=(arr[...,0]*s+arr[...,1]*c)*S
    e=np.array([0,0,1.0]); k=dv; e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
    return np.stack([cx+X*dv[0]+Y*e[0], cy+X*dv[1]+Y*e[1], X*dv[2]+Y*e[2]],-1), abs(np.cross(dv,e)@v)
def holecount(W):
    cnt=0
    for b in range(3):
        p=W[:,b,:]; z=p[:,2]
        for i in np.where(np.sign(z)!=np.sign(np.roll(z,-1)))[0]:
            a,c2=p[i],p[(i+1)%len(p)]; t=a[2]/(a[2]-c2[2]); q=a+(c2-a)*t
            if ph.contains_point(q[:2]): cnt+=1
    return cnt
Ss=np.linspace(1.45,2.1,7)/R
cands=[]
for psi in np.linspace(0,2*np.pi,48,endpoint=False):
    Xc=crossings(psi)
    if len(Xc)==0: continue
    for phi in np.linspace(0,np.pi,24,endpoint=False):
        dv=np.array([np.cos(phi),np.sin(phi)])
        for cx in [-0.06,0,0.06]:
            for cy in [0.3,0.36,0.42]:
                Q=(np.array([cx,cy])[None,None,:]+Ss[:,None,None]*Xc[None,:,None]*dv[None,None,:]).reshape(-1,2)
                ih=ph.contains_points(Q)&(segdist(Q,HOLE)>M_B); outside=(~po.contains_points(Q))&(segdist(Q,OUT)>M_B)
                ok=(ih|outside).reshape(len(Ss),-1).all(1); nh=ih.reshape(len(Ss),-1).sum(1)
                for k in np.where(ok&(nh>0))[0]: cands.append((psi,phi,cx,cy,Ss[k],int(nh[k])))
cands.sort(key=lambda c:-c[5])
found=[]; near=[]
for psi,phi,cx,cy,S,nh in cands[:250]:
    for tau in np.linspace(-0.75,0.75,7):
        P=[psi,phi,cx,cy,S,tau]; W,face=build(P)
        if face<0.5: continue
        h=hits(W[::2].reshape(-1,3),M_B)
        near.append((h,P))
        if h==0 and hits(W.reshape(-1,3),M_B)==0: found.append((holecount(W),float(face),list(map(float,P))))
    if time.time()-t0>200: break
rng=np.random.default_rng(1)
if not found and near:
    near.sort(key=lambda x:x[0])
    for h0,P in near[:5]:
        P=np.array(P); W,f=build(P); c0=hits(W.reshape(-1,3),M_B)
        for it in range(150):
            Q=P+rng.normal(0,1,6)*np.array([0.05,0.05,0.02,0.02,0.03/R,0.06])
            if not (1.4/R<=Q[4]<=2.15/R): continue
            W,f=build(Q)
            if f<0.5: continue
            c1=hits(W.reshape(-1,3),M_B)
            if c1<=c0: P,c0=Q,c1
            if c0==0: break
        W,f=build(P)
        if c0==0 and holecount(W)>0: found.append((holecount(W),float(f),list(map(float,P))))
        if time.time()-t0>280: break
found.sort(key=lambda x:(-x[0]*0.5-x[1]*3))
best_near=min([n[0] for n in near]) if near else None
json.dump({'name':name,'R':float(R),'found':found[:5],'cands':len(cands),'best_near':None if best_near is None else int(best_near)},open(f'orb_{name}.json','w'))
print(name,'cands',len(cands),'clean',len(found),'best',found[0][:2] if found else None,'near',best_near,round(time.time()-t0),'s')
