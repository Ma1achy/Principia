import numpy as np
cr=lambda a,b:a[0]*b[1]-a[1]*b[0]
J=lambda a:np.stack([-a[...,1],a[...,0]],-1)
rng=np.random.default_rng(4)
# TM23: doc's P figure, which R?
def tm(x,y,a,b,m,fr):
    A=np.array([(x,y),(-.5,0),(.5,0)]);m=np.array(m,float)
    c=A.mean(0) if fr=='cent' else (m[:,None]*A).sum(0)/m.sum();R=A-c
    V=np.array([-2*a*R[0]+b*R[1],a*R[0]-2*b*R[1],0*R[0]]);V[2]=-V[0]-V[1];return m,R,V
for fr in('cent','com'):
  for P0 in[(.1994373,.3164830,.1,.03),(.2,.3,.1,.03),(.1994373,.3164830,.1,0)]:
    for ms in[(1,.8,.6),(.6,.8,1)]:
      for norm in(1,sum(ms)):
        m,R,V=tm(*P0,np.array(ms)/norm,fr);print(fr,P0,ms,'M=%g'%(sum(ms)/norm),'P',np.round((m[:,None]*V).sum(0),4))
# chazy at random masses, M=1: K_lam ||lam||/mu_lam vs out_kup K/U
w=0;ks=0
for _ in range(20000):
    m=rng.dirichlet([3,3,3]);M=m[0]+m[1];mr=m[0]*m[1]/M;ml=m[2]*M
    a=rng.uniform(0,np.pi/2);b=rng.uniform(-3,3);rt=np.array([np.cos(a),0]);lt=np.sin(a)*np.array([np.cos(b),np.sin(b)])
    rho=rt/np.sqrt(mr);lam=lt/np.sqrt(ml);r01=-m[2]*lam;r=np.array([r01-m[1]/M*rho,r01+m[0]/M*rho,M*lam])
    pr,pl=rng.normal(size=2),rng.normal(size=2);p=np.array([-pr-m[0]/M*pl,pr-m[1]/M*pl,pl]);v=p/m[:,None]
    # out_kup: reduced mass mu=m2 M01/(M01+m2), K=mu dv^2/2, U=m2 M01/d
    cm=(m[0]*r[0]+m[1]*r[1])/M;vc=(m[0]*v[0]+m[1]*v[1])/M;d=np.linalg.norm(r[2]-cm);dv=v[2]-vc
    mu=m[2]*M/(M+m[2]);K=.5*mu*dv@dv;U=m[2]*M/d;q=K/U
    Kl=pl@pl/(2*ml);w=max(w,abs(q-Kl*np.linalg.norm(lam)/ml))
    # K_shape = I|ndot|^2/8
    def nv(r):
        I=(m[:,None]*r*r).sum();R=r[1]-r[0];cmm=(m[0]*r[0]+m[1]*r[1])/M;l=r[2]-cmm
        Rt=np.sqrt(mr)*R/np.sqrt(I);Lt=np.sqrt(ml)*l/np.sqrt(I)
        z=complex(*Rt);y=complex(*Lt)
        return np.array([abs(z)**2-abs(y)**2,2*(z.conjugate()*y).real,2*(z.conjugate()*y).imag])
    h=1e-6;nd=(nv(r+h*v)-nv(r-h*v))/(2*h)
    I=(m[:,None]*r*r).sum();hh=(m[:,None]*r*v).sum();L=(m[:,None]*J(r)*v).sum();K0=.5*(m[:,None]*v*v).sum()
    Ks=K0-hh**2/2/I-L**2/2/I;ks=max(ks,abs(Ks-I*(nd@nd)/8))
print('chazy q vs K_lam|lam|/mu_lam worst',w,'K_shape worst',ks)
# ell range at masses, and eq.3 at unequal
for ms in[(1,1,1),(1,.8,.6),(1,.8,.2),(9,3,1),(3,1,1)]:
    m=np.array(ms,float);m/=m.sum();M=m[0]+m[1];mr=m[0]*m[1]/M;ml=m[2]*M;d=(m[1]-m[0])/(2*M)
    X,Y=np.meshgrid(np.linspace(0,.5,601),np.linspace(0,.8661,601));ok=(X+.5)**2+Y**2<=1
    e=np.sqrt(mr+ml*((X-d)**2+Y**2))[ok];print(ms,'ell %.3f %.3f time factor %.3f'%(e.min(),e.max(),(e.max()/e.min())**1.5))
m=np.array([1,.8,.6]);v=np.array([5e-5,5e-5,-1e-4]);print('eq3 Px at (1,.8,.6)',(m*v).sum())
# Suvakov at m3: outer m=1, middle m3, v3=-2v: P
print('Suvakov P at m3=.5, v3=-2v:',1+1+.5*(-2),' with -2/m3:',1+1+.5*(-2/.5))
