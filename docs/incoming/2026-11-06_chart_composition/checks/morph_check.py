import numpy as np
rng=np.random.default_rng(6)
s=lambda x:1/(1+np.exp(-x)); logit=lambda x:np.log(x/(1-x))
MU=5.;QM=2.;EZ=1e-6
def masses(z):
    mu=MU*np.tanh(z[...,6:8]); e=np.exp(np.concatenate([np.zeros(z.shape[:-1]+(1,)),mu],-1)); return e/e.sum(-1,keepdims=True)
def state(m,n,q):
    """signed shape n, Jacobi momenta q -> r,p at I=1"""
    a=np.arccos(np.clip(n[0],-1,1))/2; b=np.arctan2(n[2],n[1])
    M=m[0]+m[1]; mr=m[0]*m[1]/M; ml=m[2]*M
    rho=np.array([np.cos(a),0])/np.sqrt(mr); lam=np.sin(a)*np.array([np.cos(b),np.sin(b)])/np.sqrt(ml)
    r01=-m[2]*lam; r=[r01-m[1]/M*rho,r01+m[0]/M*rho,M*lam]
    pr=q[:2];pl=q[2:]; p=[-pr-m[0]/M*pl,pr-m[1]/M*pl,pl]
    return np.array(r),np.array(p)
def inv(m,r,p):
    I=sum(m[i]*r[i]@r[i] for i in range(3))
    K=sum(p[i]@p[i]/(2*m[i]) for i in range(3))
    L=sum(r[i][0]*p[i][1]-r[i][1]*p[i][0] for i in range(3))
    U=-sum(m[i]*m[j]/np.linalg.norm(r[i]-r[j]) for i,j in((0,1),(0,2),(1,2)))
    return I,K,U,L
def Dz(z):
    m=masses(z); a=np.pi/2*s(z[0]); b=np.pi*s(z[1])
    n=np.array([np.cos(2*a),np.sin(2*a)*np.cos(b),np.sin(2*a)*np.sin(b)])
    q=QM*(2*s(z[2:6])-1); return m,n,q
def Ez(m,n,q):
    """encode: mirror step, clamps; q in the mirrored frame"""
    q=q.copy(); n=n.copy(); mir=n[2]<0
    if mir: n[2]=-n[2]; q[[1,3]]*=-1
    a=np.arccos(np.clip(n[0],-1,1))/2; b=np.arctan2(n[2],n[1])
    sa=np.clip(2*a/np.pi,EZ,1-EZ); sb=np.clip(b/np.pi,EZ,1-EZ)
    sq=np.clip((q/QM+1)/2,EZ,1-EZ)
    mu=np.log(m[1:]/m[0]); zm=np.arctanh(np.clip(mu/MU,-1+1e-6,1-1e-6))
    return np.concatenate([[logit(sa),logit(sb)],logit(sq),zm])
def slerp(a,b,t):
    w=np.arccos(np.clip(a@b,-1,1))
    if w<1e-12: return a
    return (np.sin((1-t)*w)*a+np.sin(t*w)*b)/np.sin(w)
def rs(): 
    v=rng.normal(size=3); return v/np.linalg.norm(v)
print("T1 blended z on the sphere, I=1; n lerp and r lerp not")
d=[];dn=[];di=[]
for _ in range(20000):
    za,zb=rng.uniform(-6,6,8),rng.uniform(-6,6,8);t=rng.uniform()
    m,n,q=Dz(za+t*(zb-za)); r,p=state(m,n,q); I=inv(m,r,p)[0]
    d.append(max(abs(n@n-1),abs(I-1)))
    ma,na,qa=Dz(za);mb,nb,qb=Dz(zb); dn.append(abs(np.linalg.norm((1-t)*na+t*nb)-1))
    ra=state(ma,na,qa)[0];rb=state(ma,nb,qb)[0];rr=(1-t)*ra+t*rb;di.append(abs(sum(ma[i]*rr[i]@rr[i] for i in range(3))-1))
print(" z blend worst |n|^2-1,I-1 %.1e; n lerp worst |n|-1 %.3f (median %.3f); r lerp worst I-1 %.3f"%(max(d),max(dn),np.median(dn),max(di)))
print("T2 reach: z decode n2>=0 always:",all(Dz(rng.uniform(-30,30,8))[1][2]>=0 for _ in range(5000)))
print("T3 leaving an end: distance of the frame at s=1e-6 from end A (n and q, Euclid)")
J=[];JY=[]
for _ in range(5000):
    n=rs(); n[2]=-abs(n[2]); q=rng.uniform(-.5,.5,4); m=masses(rng.uniform(-1,1,8))
    nb=rs(); qb=rng.uniform(-.5,.5,4); t=1e-6
    za,zb=Ez(m,n,q),Ez(m,nb,qb); m2,n2,q2=Dz(za+t*(zb-za))
    J.append(np.linalg.norm(n2-n)+np.linalg.norm(q2-q))
    ny=slerp(n,nb,t); qy=q+t*(qb-q); JY.append(np.linalg.norm(ny-n)+np.linalg.norm(qy-q))
print(" lower hemisphere end: literal z median %.3f worst %.3f; on blocks median %.1e worst %.1e"%(np.median(J),max(J),np.median(JY),max(JY)))
# momentum past q_max: energy held, near a pair
f={};
for Es in(-0.5,0.,1.):
  cnt=0;tot=0;jump=[]
  for _ in range(20000):
    m=np.ones(3)/3; u=rng.uniform(); v=rng.uniform()
    th,ph=2*np.pi*u,np.pi*v; n=np.array([np.sin(ph)*np.cos(th),np.sin(ph)*np.sin(th),np.cos(ph)])
    q=np.array([0.1,0.05,-0.08,0.03]); r,p=state(m,n,q); I,K,U,L=inv(m,r,p)
    if Es<U: continue
    eta=np.sqrt((Es-U)/K); qq=eta*q; tot+=1
    if np.abs(qq).max()>QM: cnt+=1; jump.append((np.abs(qq).max()-QM))
  f[Es]=(cnt/tot,tot,max(jump) if jump else 0)
print(" q past q_max under an energy hold, sphere sweep, equal masses, fraction, feasible pixels, worst excess:",f)
print(" near a pair: U at n0=0.999:",inv(np.ones(3)/3,*state(np.ones(3)/3,np.array([-.9999,np.sqrt(1-.9999**2),0]),np.zeros(4)))[2])
print("T4 collinear end: clamp jump, and the stall")
n=np.array([.6,.8,0.]);za=Ez(np.ones(3)/3,n,np.zeros(4));print(" beta clamp jump in n %.1e"%np.linalg.norm(Dz(za)[1]-n))
nb=np.array([.6,0,.8]);zb=Ez(np.ones(3)/3,nb,np.zeros(4));ts=np.linspace(0,1,1001)
ang=[np.arccos(np.clip(Dz(za+t*(zb-za))[1]@nb,-1,1)) for t in ts];tot=ang[0]
print(" z blend: s at which 10%% of the angle is covered %.3f, 50%% %.3f, 90%% %.3f"%tuple(ts[np.argmax(np.array(ang)<=tot*(1-x))] for x in(.1,.5,.9)))
print("T5 rank: area of the plane per unit pixel, along s")
e=np.eye(8)
def area(Q): g=Q.T@Q; return np.sqrt(max(np.linalg.det(g),0))
def geo(QA,QB,t):
    U,c,Vt=np.linalg.svd(QA.T@QB); c=np.clip(c,-1,1); th=np.arccos(c)
    A=QA@U; B=QB@Vt.T
    if np.linalg.det(U)*np.linalg.det(Vt)<0: # handed: take the far way on the second
        B[:,1]*=-1; th[1]=np.arccos(c[1]*-1)*0+np.arccos(A[:,1]@B[:,1])
    W=np.zeros_like(A)
    for k in range(2):
        w=B[:,k]-A[:,k]*(A[:,k]@B[:,k]); nw=np.linalg.norm(w)
        W[:,k]=w/nw if nw>1e-14 else 0
    return A*np.cos(t*th)+W*np.sin(t*th),U,Vt,th
QA=e[:,[0,1]];QB=e[:,[1,0]]
print(" swap: lerp area at .5 %.3f"%area(.5*QA+.5*QB))
QB=e[:,[2,3]]
print(" orthogonal planes: lerp area at .5 %.3f; geodesic %.3f"%(area(.5*QA+.5*QB),area(geo(QA,QB,.5)[0])))
wl=[];wg=[];orth=[]
for _ in range(2000):
    a,_r=np.linalg.qr(rng.normal(size=(8,2)));b,_r=np.linalg.qr(rng.normal(size=(8,2)))
    wl.append(min(area((1-t)*a+t*b) for t in np.linspace(0,1,21)))
    G=geo(a,b,.37)[0]; orth.append(abs(G.T@G-np.eye(2)).max()); 
    Ge=geo(a,b,1.)[0]; # plane reached
    wg.append(np.linalg.norm(Ge@Ge.T-b@b.T))
print(" random planes: lerp worst area %.3f median-min %.3f; geodesic orthonormal to %.1e, reaches plane to %.1e"%(min(wl),np.median(wl),max(orth),max(wg)))
print("T6 flat to polar: folded share of pixels by s")
N=400;u,v=np.meshgrid((np.arange(N)+.5)/N,(np.arange(N)+.5)/N,indexing='ij')
for t in(0,.1,.25,.5,.75,.9,1):
    x=(1-t)*u+t*(.5+.5*v*np.cos(2*np.pi*u));y=(1-t)*v+t*(.5+.5*v*np.sin(2*np.pi*u))
    xu,xv=np.gradient(x,1/N,1/N);yu,yv=np.gradient(y,1/N,1/N);det=xu*yv-xv*yu
    print("  s=%.2f share det<=0 %.3f  min|det| %.4f"%(t,(det<=0).mean(),np.abs(det).min()))
print("T7 holds along a chart-level blend")
dE=[];dL=[];de2=[];blank=0;tot=0
m=np.ones(3)/3
for _ in range(5000):
    na,nb=rs(),rs();na[2]=abs(na[2]);nb[2]=abs(nb[2])
    def held(n,E):
        q=rng.uniform(-.3,.3,4);r,p=state(m,n,q);I,K,U,L=inv(m,r,p)
        return None if E<U else np.sqrt((E-U)/K)*q
    Es=-.1
    qa,qb=held(na,Es),held(nb,Es)
    if qa is None or qb is None: continue
    t=.5;ny=slerp(na,nb,t);qy=qa+t*(qb-qa);r,p=state(m,ny,qy);I,K,U,L=inv(m,r,p)
    dE.append(abs(K+U-Es)); tot+=1
    if Es<U: blank+=1
    elif K>0:
        eta=np.sqrt((Es-U)/K); de2.append(abs(eta*eta*K+U-Es))
print(" hold E=-0.1, both ends feasible: blend alone |dE| median %.3f worst %.3f; the step after the blend worst %.1e; blank mid-morph %d of %d"%(np.median(dE),max(dE),max(de2),blank,tot))
print("T8 float ends")
a=rng.uniform(-10,10,1000000);b=rng.uniform(-10,10,1000000)
print(" a+(b-a)!=b share %.3f; (1-1)a+1b exact; a+s(b-a) outside [min,max], s<1: %d"%((a+(b-a)!=b).mean(),((a+.999999*(b-a))>np.maximum(a,b)).sum()))
print("T9 s precision: u=f/(F) f64 distinct at F=1e6:",len({i/1000000 for i in range(1000001)}),"; f32 distinct:",len({float(np.float32(i/1000000)) for i in range(1000001)}))
print("T6b flat to polar with the angle reversed, keeping orientation")
for t in(.25,.5,.75,1):
    x=(1-t)*u+t*(.5+.5*v*np.cos(-2*np.pi*u));y=(1-t)*v+t*(.5+.5*v*np.sin(-2*np.pi*u))
    xu,xv=np.gradient(x,1/N,1/N);yu,yv=np.gradient(y,1/N,1/N);det=xu*yv-xv*yu
    print("  s=%.2f share det<=0 %.3f"%(t,(det<=0).mean()))
print("T7b hold E=-0.3, near the feasible edge; T7c hold L_z")
dE=[];blank=0;tot=0;dL=[];dL2=[]
for _ in range(60000):
    na,nb=rs(),rs();na[2]=abs(na[2]);nb[2]=abs(nb[2])
    Es=-.3;ok=True;qs=[]
    for n in(na,nb):
        q=rng.uniform(-.3,.3,4);r,p=state(m,n,q);I,K,U,L=inv(m,r,p)
        if Es<U: ok=False;break
        qs.append(np.sqrt((Es-U)/K)*q)
    if not ok: continue
    tot+=1;ny=slerp(na,nb,.5);qy=.5*(qs[0]+qs[1]);r,p=state(m,ny,qy);I,K,U,L=inv(m,r,p)
    if Es<U: blank+=1
print(" feasible at both ends %d, infeasible at s=.5 %d"%(tot,blank))
