exec(open('hole2.py').read().split("cands=[]")[0])
import time; t0=time.time()
cands=json.load(open('cands.json'))
def rotm(ax,a):
    c,s=np.cos(a),np.sin(a)
    if ax=='x': return np.array([[1,0,0],[0,c,-s],[0,s,c]])
    if ax=='y': return np.array([[c,0,s],[0,1,0],[-s,0,c]])
    return np.array([[c,-s,0],[s,c,0],[0,0,1]])
v=rotm('y',0.62)@rotm('x',-0.17)@np.array([0,0,1.0])
res=[]
for psi,phi,cx,cy,S,nh in cands:
    dvec=np.array([np.cos(phi),np.sin(phi),0.0]); c,s=np.cos(psi),np.sin(psi)
    X=(mo[...,0]*c-mo[...,1]*s)*S; Y=(mo[...,0]*s+mo[...,1]*c)*S
    for tau in np.linspace(-0.75,0.75,7):
        e=np.array([0,0,1.0]); k=dvec; e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
        n=np.cross(dvec,e); face=abs(n@v)
        if face<0.45: continue
        W=np.stack([cx+X*dvec[0]+Y*e[0], cy+X*dvec[1]+Y*e[1], X*dvec[2]+Y*e[2]],-1)
        if hits(W[::3].reshape(-1,3),M_B)>0: continue
        if hits(W.reshape(-1,3),M_B)>0: continue
        res.append((nh*0.8+face*3-abs(S-2.3)*1.5-abs(cx)-abs(cy-0.36), int(nh), round(float(face),2), [float(x) for x in (psi,phi,cx,cy,S,tau)]))
    if time.time()-t0>540: break
res.sort(key=lambda r:-r[0])
print('valid',len(res), round(time.time()-t0,1),'s')
for r in res[:8]: print(r)
json.dump([r[3] for r in res[:12]],open('hole.json','w'))
