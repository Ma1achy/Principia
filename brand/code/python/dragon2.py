exec(open('dragon.py').read().split("dv=np.array([np.cos(phi)")[0])
rng=np.random.default_rng(5)
Ys=Y[::3]
def build(P,YY):
    psi,phi,cx,cy,S,tau=P
    dv=np.array([np.cos(phi),np.sin(phi),0.0]); e=np.array([0,0,1.0]); k=dv
    e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
    c,s=np.cos(psi),np.sin(psi); X=(YY[...,0]*c-YY[...,1]*s)*S; Yy=(YY[...,0]*s+YY[...,1]*c)*S
    return np.stack([cx+X*dv[0]+Yy*e[0], cy+X*dv[1]+Yy*e[1], X*dv[2]+Yy*e[2]],-1)
def holecount(W):
    cnt=0
    for b in range(3):
        p=W[:,b,:]; z=p[:,2]
        for i in np.where(np.sign(z)!=np.sign(np.roll(z,-1)))[0]:
            a,c2=p[i],p[(i+1)%len(p)]; t=a[2]/(a[2]-c2[2]); q=a+(c2-a)*t
            if ph.contains_point(q[:2]): cnt+=1
    return cnt
P0=np.array(P); best=(hits(build(P0,Ys).reshape(-1,3),0.1),P0)
print('start',best[0])
for it in range(400):
    Q=best[1]+rng.normal(0,1,6)*np.array([0.01,0.01,0.008,0.008,0.004,0.015])
    h=hits(build(Q,Ys).reshape(-1,3),0.1)
    if h<=best[0]: best=(h,Q)
    if best[0]==0: break
Wf=build(best[1],Y)
print('end coarse',best[0],'full-res hits',hits(Wf[::2].reshape(-1,3),0.095),'hole passes',holecount(Wf[::10]))
json.dump(list(map(float,best[1])),open('dragon_P.json','w'))
