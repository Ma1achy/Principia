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
    nr=np.linalg.norm(r,axis=1);w[5]=max(w[5],abs(x[0]-nr[1]/nr[0]),abs(x[1]-nr[2]/nr[0])) if abs(np.linalg.det(A))>1e-3 else w[5]
    # h inverse: v + (h*-h)/I r
    hs=rng.normal();v2=v+(hs-h)/I*r;w[6]=max(w[6],abs(ip(r,v2)-hs),abs(ip(J(r),v2)-L))
print("Saari split orth+sum %.1e; K_rho+K_lam %.1e; L_rho+L_lam %.1e; area=n2/(4 sqrt) %.1e; I1 I2=n2^2/4,I1+I2=1 %.1e; COM polar inverse %.1e; h inverse keeps L %.1e"%tuple(w[:7]))
# Euler crit equal masses
a=1.;m=1/3;I=2*m*a*a;U=-(m*m)*(2/a+1/(2*a));print("equal-mass Euler L^2|E| crit, M=1: %.5f (25/972=%.5f)"%(I*U*U/2,25/972))
