import numpy as np
rng=np.random.default_rng(11)
J=lambda a:np.stack([-a[...,1],a[...,0]],-1)
w=np.zeros(9)
for _ in range(20000):
    m=rng.dirichlet([3,3,3]);a=rng.uniform(0,np.pi/2);b=rng.uniform(-np.pi,np.pi)
    M=m[0]+m[1];mr=m[0]*m[1]/M;ml=m[2]*M
    rt=np.array([np.cos(a),0]);lt=np.sin(a)*np.array([np.cos(b),np.sin(b)])
    rho=rt/np.sqrt(mr);lam=lt/np.sqrt(ml)
    r01=-m[2]*lam;r=np.array([r01-m[1]/M*rho,r01+m[0]/M*rho,M*lam])
    pr,pl=rng.normal(size=2),rng.normal(size=2)
    p=np.array([-pr-m[0]/M*pl,pr-m[1]/M*pl,pl]);v=p/m[:,None]
    ip=lambda x,y:np.sum(m[:,None]*x*y)
    I=ip(r,r);K=.5*ip(v,v);L=ip(J(r),v);h=ip(r,v)
    vs=v-(h/I)*r-(L/I)*J(r);Ks=.5*ip(vs,vs)
    w[0]=max(w[0],abs(K-h*h/2/I-L*L/2/I-Ks),abs(ip(r,J(r))))
    Kr=pr@pr/(2*mr);Kl=pl@pl/(2*ml);w[1]=max(w[1],abs(K-Kr-Kl))
    Lr=rho[0]*pr[1]-rho[1]*pr[0];Ll=lam[0]*pl[1]-lam[1]*pl[0];w[2]=max(w[2],abs(L-Lr-Ll))
    area=.5*(rho[0]*lam[1]-rho[1]*lam[0]);n2=2*(rt[0]*lt[1]-rt[1]*lt[0]);w[3]=max(w[3],abs(area-n2/(4*np.sqrt(mr*ml))))
    g=sum(m[i]*np.outer(r[i],r[i]) for i in range(3));e=np.linalg.eigvalsh(g)  # planar second moments
    w[4]=max(w[4],abs(e[0]*e[1]-n2*n2/4),abs(e.sum()-1))
    # polar angles about COM -> radial ratios
    u=r/np.linalg.norm(r,axis=1,keepdims=True);A=np.c_[m[1]*u[1],m[2]*u[2]];x=np.linalg.solve(A,-m[0]*u[0])
    nr=np.linalg.norm(r,axis=1);e_=max(abs(x[0]-nr[1]/nr[0]),abs(x[1]-nr[2]/nr[0])); dt=abs(np.linalg.det(A))
    w[5]=max(w[5],e_) if dt>1e-3 else w[5]; w[7]=max(w[7],e_*dt); w[8]=max(w[8],e_/max(1,nr[1]/nr[0],nr[2]/nr[0])*dt)  # the inverse divides by det: the cut is its band, w[7] the scaling, w[8] the same relative to the largest ratio
    # h inverse: v + (h*-h)/I r
    hs=rng.normal();v2=v+(hs-h)/I*r;w[6]=max(w[6],abs(ip(r,v2)-hs),abs(ip(J(r),v2)-L))
print("Saari split orth+sum %.1e; K_rho+K_lam %.1e; L_rho+L_lam %.1e; area=n2/(4 sqrt) %.1e; I1 I2=n2^2/4,I1+I2=1 %.1e; COM polar inverse %.1e; h inverse keeps L %.1e"%tuple(w[:7]))
# Euler crit equal masses
a=1.;m=1/3;I=2*m*a*a;U=-(m*m)*(2/a+1/(2*a));print("equal-mass Euler L^2|E| crit, M=1: %.5f (25/972=%.5f)"%(I*U*U/2,25/972))
print("polar inverse: worst error times |det|, no cut: %.1e, relative to the largest ratio %.1e (band above: |det| > 1e-3)"%(w[7],w[8]))
# 7 Nov (physics reviews 5809751370 and 5809833272, Z2): both sides of the polar row's round trip, each float. |det| = m1 m2 |sin(phi2 - phi1)|, by band.
# inverse: gaps (g1, g2) on the region where every gap is under pi, 2 x 2 solve, I = 1. forward: the row's, polar angles about the COM, so it
# subtracts the COM: atan2 about the origin returns u's angle whatever the solve gave, and tests nothing. Probes: review's polar.py, polar2.py.
rng=np.random.default_rng(12)
for T,nm in ((np.float64,'f64'),(np.float32,'f32')):
    e=float(np.finfo(T).eps)
    for lo in (1e-1,1e-2,1e-3,1e-4,1e-5):
        wa=wr=wq=0;n=0
        while n<8000:
            m=rng.dirichlet([3,3,3]); k=rng.integers(3)                  # k: the gap that nears pi, so |det| is small
            gap=np.pi-np.arcsin(min(1.,lo*(1+rng.random())/(m[1]*m[2])))*rng.uniform(.5,1)
            o=rng.uniform(.3,np.pi-.3); g=[o,2*np.pi-gap-o,gap]; g=[g[(i+k)%3] for i in range(3)]
            if max(g)>=np.pi or min(g)<=0: continue
            mf=m.astype(T); g1,g2=T(g[0]),T(g[0]+g[1])
            u=np.array([[1,0],[np.cos(g1),np.sin(g1)],[np.cos(g2),np.sin(g2)]],dtype=T)
            A=np.c_[mf[1]*u[1],mf[2]*u[2]].astype(T); D=A[0,0]*A[1,1]-A[0,1]*A[1,0]; dt=abs(float(D))
            b=-mf[0]*u[0]; x=np.array([(b[0]*A[1,1]-A[0,1]*b[1])/D,(A[0,0]*b[1]-b[0]*A[1,0])/D],dtype=T)
            if not lo<=dt<3*lo: continue
            n+=1
            if x.min()<=0: continue
            rho=np.array([T(1),x[0],x[1]],dtype=T); s=np.sqrt(np.sum(mf*rho*rho,dtype=T)); rho=rho/s; r=(rho[:,None]*u).astype(T)
            rc=r-(mf[:,None]*r).sum(0)/mf.sum()
            ph=np.arctan2(rc[:,1],rc[:,0]); d1=float(ph[1]-ph[0]); d2=float((ph[2]-ph[0])%T(2*np.pi))
            wa=max(wa,abs(d1-float(g1)),abs(d2-float(g2)))
            A2=np.c_[mf[1]*r[1]/np.hypot(*r[1]),mf[2]*r[2]/np.hypot(*r[2])].astype(T); u0=r[0]/np.hypot(*r[0]); D=A2[0,0]*A2[1,1]-A2[0,1]*A2[1,0]; b=-mf[0]*u0
            y=np.array([(b[0]*A2[1,1]-A2[0,1]*b[1])/D,(A2[0,0]*b[1]-b[0]*A2[1,0])/D],dtype=T)
            n1=np.hypot(*r[0]); q=[float(np.hypot(*r[1])/n1),float(np.hypot(*r[2])/n1)]; ee=max(abs(float(y[0])-q[0]),abs(float(y[1])-q[1])); wr=max(wr,ee); wq=max(wq,ee/max(1,*q))
        print("polar %s |det| from %.0e: forward(inverse) on the angles, about the COM, %.0f eps; inverse(forward) on the ratios %.1e, relative to the largest ratio %.1e, that times det/eps %.2f"%(nm,lo,wa/e,wr,wq,wq*lo/e))
# the two constants, 300,000 draws at each mass law
rng=np.random.default_rng(7); e=2.22e-16
def solve(m,u):
    A=np.c_[m[1]*u[1],m[2]*u[2]];D=A[0,0]*A[1,1]-A[0,1]*A[1,0];b=-m[0]*u[0]
    return np.array([(b[0]*A[1,1]-A[0,1]*b[1])/D,(A[0,0]*b[1]-b[0]*A[1,0])/D]),abs(D)
for label,draw in (('Dirichlet(3,3,3)',lambda:rng.dirichlet([3,3,3])),('log-uniform to 1e-3',lambda:(lambda z:z/z.sum())(np.exp(rng.uniform(np.log(1e-3),0,3))))):
    wd=wc=0
    for _ in range(300000):
        m=draw();sn=10**rng.uniform(-5,-.5);th=rng.uniform(0,6.28)
        r1=rng.uniform(.2,2)*np.array([np.cos(th),np.sin(th)]);th2=th+np.pi-sn*rng.choice([-1,1]);r2=rng.uniform(.2,2)*np.array([np.cos(th2),np.sin(th2)])
        rng.integers(3); r0=-(m[1]*r1+m[2]*r2)/m[0]
        if np.hypot(*r0)==0: continue
        r=np.array([r0,r1,r2]);nr=np.linalg.norm(r,axis=1);u=r/nr[:,None]
        x,dt=solve(m,u);t=np.array([nr[1]/nr[0],nr[2]/nr[0]]);wd=max(wd,np.abs(x-t).max()*dt/(e*max(1,*t)))
        if x.min()>0:
            rho=np.array([1,x[0],x[1]]);q=rho[:,None]*u;q2=q-(m[:,None]*q).sum(0)
            a=lambda v:np.arctan2(v[:,1],v[:,0]);d=lambda v:np.angle(np.exp(1j*(a(v)[1:]-a(v)[0])))
            wc=max(wc,np.abs(d(q2)-d(u)).max()/(e*rho.max()/rho.min()))
    print("polar constants, %s: inverse(forward) error times |det|/(eps r-hat) %.2f; forward(inverse) about the COM /(eps rmax/rmin) %.2f"%(label,wd,wc))
