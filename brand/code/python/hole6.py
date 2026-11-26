exec(open('hole5.py').read().split("import collections")[0])
rng=np.random.default_rng(3)
def build(P):
    psi,phi,cx,cy,S,tau=P
    dvec=np.array([np.cos(phi),np.sin(phi),0.0]); c,s=np.cos(psi),np.sin(psi)
    X=(mo[...,0]*c-mo[...,1]*s)*S; Y=(mo[...,0]*s+mo[...,1]*c)*S
    e=np.array([0,0,1.0]); k=dvec; e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
    return np.stack([cx+X*dvec[0]+Y*e[0], cy+X*dvec[1]+Y*e[1], X*dvec[2]+Y*e[2]],-1), abs(np.cross(dvec,e)@v)
def holecount(W):
    cnt=0
    for b in range(3):
        p=W[:,b,:]; z=p[:,2]; idx=np.where(np.sign(z)!=np.sign(np.roll(z,-1)))[0]
        for i in idx:
            a,c2=p[i],p[(i+1)%len(p)]; t=a[2]/(a[2]-c2[2]); q=a+(c2-a)*t
            if ph.contains_point(q[:2]): cnt+=1
    return cnt
def cost(P):
    W,face=build(P)
    if face<0.5: return 1e9,0,face
    h=hitsH(W.reshape(-1,3),0.095,0.2)
    return h - 0.01*face, holecount(W), face
# seeds: best few from coarse grid
seeds=[]
for psi,phi,cx,cy,S,nh in cands[:700]:
    for tau in np.linspace(-0.75,0.75,7):
        P=[psi,phi,cx,cy,S,tau]; W,face=build(P)
        if face<0.5: continue
        h=hitsH(W[::2].reshape(-1,3),0.095,0.2); seeds.append((h,P))
seeds.sort(key=lambda x:x[0]); seeds=seeds[:12]
print('seed hits',[s[0] for s in seeds])
found=[]
for h0,P in seeds:
    P=np.array(P); c0,hc,f=cost(P)
    step=np.array([0.06,0.06,0.03,0.03,0.08,0.08])
    for it in range(350):
        Q=P+rng.normal(0,1,6)*step
        c1,hc1,f1=cost(Q)
        if c1<=c0: P,c0,hc,f=Q,c1,hc1,f1
        if c0<0: break
    print('end hits',round(c0,3),'hole passes',hc,'face',round(f,2),[round(x,3) for x in P])
    if c0<0 and hc>0: found.append((hc,f,list(map(float,P))))
found.sort(key=lambda x:(-x[0],-x[1]))
json.dump(found,open('hole_found.json','w')); print('found',len(found))
