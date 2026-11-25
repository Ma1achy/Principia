exec(open('clear.py').read().split("res={}")[0])
import json
rng=np.random.default_rng(7)
mo=d['moth'][:-1]                      # (900,3,2)
sub=mo[::3]                             # coarse for screening
HOLE_IN=Path(HOLE)
def rotm(ax,a):
    c,s=np.cos(a),np.sin(a)
    if ax=='x': return np.array([[1,0,0],[0,c,-s],[0,s,c]])
    if ax=='y': return np.array([[c,0,s],[0,1,0],[-s,0,c]])
    return np.array([[c,-s,0],[s,c,0],[0,0,1]])
spin,pitch=-0.62,0.17
# direction towards the camera, in the P's own frame
v=rotm('y',-spin)@rotm('x',pitch)@np.array([0,0,1.0])   # rot = Rx(pitch)·Ry(spin) acting on (x,y,z) with the code's sign conventions
def world(arr,P):
    psi,al,be,cx,cy,cz,S=P
    R=rotm('x',be)@rotm('y',al)@rotm('z',psi)
    e1,e2=R[:,0],R[:,1]
    X=arr[...,0]*S; Y=arr[...,1]*S
    return np.stack([cx+X*e1[0]+Y*e2[0], cy+X*e1[1]+Y*e2[1], cz+X*e1[2]+Y*e2[2]],-1), np.cross(e1,e2)
def holepasses(W):
    # W: (n,3,3) per body; count z sign changes whose crossing lands in the hole
    cnt=0
    for b in range(3):
        p=W[:,b,:]; z=p[:,2]; idx=np.where(np.sign(z[:-1])!=np.sign(z[1:]))[0]
        for i in idx:
            a,c=p[i],p[i+1]; s=a[2]/(a[2]-c[2]); q=a+(c-a)*s
            if HOLE_IN.contains_point(q[:2], radius=-0.0): cnt+=1
    return cnt
best=[]; tried=0
while tried<60000:
    tried+=1
    P=(rng.uniform(0,np.pi), rng.uniform(-1.5,1.5), rng.uniform(-1.2,1.2), rng.uniform(-0.4,0.6), rng.uniform(-0.3,0.8), rng.uniform(-0.3,0.3), rng.uniform(0.9,1.6))
    W,n=world(sub,P)
    face=abs(n@v)/np.linalg.norm(v)
    if face<0.55: continue
    if hits(W.reshape(-1,3),0.12)>0: continue
    hp=holepasses(W)
    if hp<2: continue
    Wf,_=world(mo,P)
    if hits(Wf.reshape(-1,3),0.12)>0: continue
    hp=holepasses(Wf)
    score=hp*1.0+face*4+P[6]*3-abs(P[3])*2-abs(P[4]-0.3)*2
    best.append((score,hp,face,P)); best.sort(key=lambda x:-x[0]); best=best[:5]
print('tried',tried)
for b in best: print(round(b[0],2),'holepasses',b[1],'face',round(b[2],2),[round(x,3) for x in b[3]])
json.dump([list(map(float,b[3])) for b in best],open('hole.json','w'))
