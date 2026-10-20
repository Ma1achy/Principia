import numpy as np
from scipy.integrate import solve_ivp
QMAX=2.0; MUMAX=5.0; DL=1e-12
def rhs(t,y,m):
    q=y[:6].reshape(3,2); v=y[6:].reshape(3,2); a=np.zeros((3,2))
    for i in range(3):
        for j in range(3):
            if i!=j:
                d=q[j]-q[i]; a[i]+=m[j]*d/np.linalg.norm(d)**3
    return np.concatenate([v.ravel(),a.ravel()])
def integrate(m,q0,v0,T,n=20000):
    y0=np.concatenate([np.array(q0,float).ravel(),np.array(v0,float).ravel()])
    ts=np.linspace(0,T,n+1)
    s=solve_ivp(rhs,(0,T),y0,args=(np.array(m,float),),method='DOP853',rtol=1e-12,atol=1e-12,t_eval=ts)
    return s.t,s.y.T
def logit(x): return np.log(x/(1-x))
def encode(m,r,v):
    """chart_reference §0.2–0.5 inverse: physical (m, r, v) -> canonical-frame latents. m any total; normalised to M=1."""
    M=m.sum(); mn=m/M; vn=v/np.sqrt(M)       # mass gauge: same orbit in M=1 units
    p=mn[:,None]*vn
    com=(mn[:,None]*r).sum(0); r=r-com; p=p-p.sum(0)/3*0  # COM frame (momenta: total ~0 for COM-at-rest ICs)
    m0,m1,m2=mn; M01=m0+m1
    rho=r[1]-r[0]; r01=(m0*r[0]+m1*r[1])/M01; lam=r[2]-r01
    prho=(m0*p[1]-m1*p[0])/M01; plam=p[2]
    mur=m0*m1/M01; mul=m2*M01
    phi=0.0 if (rho[0]==0 and rho[1]==0) else np.arctan2(rho[1],rho[0])
    c,s=np.cos(-phi),np.sin(-phi); R=np.array([[c,-s],[s,c]])
    rho,lam,prho,plam=R@rho,R@lam,R@prho,R@plam
    rt,lt=np.sqrt(mur)*rho,np.sqrt(mul)*lam
    mirrored=lt[1]< -DL
    if mirrored:
        F=np.array([1,-1.]); rho,lam,prho,plam,rt,lt=rho*F,lam*F,prho*F,plam*F,rt*F,lt*F
    I=rt@rt+lt@lt; l=np.sqrt(I)
    rt,lt=rt/l,lt/l; prho,plam=prho*np.sqrt(l),plam*np.sqrt(l)
    alpha=np.arctan2(np.linalg.norm(lt),np.linalg.norm(rt))
    beta=np.arctan2(abs(lt[1]),lt[0])
    q=np.array([prho[0],prho[1],plam[0],plam[1]])
    with np.errstate(divide='ignore',invalid='ignore'):
        za=logit(alpha/(np.pi/2)); zb=logit(beta/np.pi)
        zq=np.where(np.abs(q)<QMAX, logit((q/QMAX+1)/2), np.nan)
    # shape sphere (§3.1), from unnormalised mass-weighted vectors (sign of cross before mirror!)
    return dict(za=za,zb=zb,q=q,zq=zq,mirrored=mirrored,alpha=alpha,beta=beta)
def shape(m,r):
    M=m.sum(); mn=m/M; m0,m1,m2=mn; M01=m0+m1
    rho=r[1]-r[0]; lam=r[2]-(m0*r[0]+m1*r[1])/M01
    rt=np.sqrt(m0*m1/M01)*rho; lt=np.sqrt(m2*M01)*lam
    a,b=rt@rt,lt@lt; I=a+b; pp=rt@lt; qq=rt[0]*lt[1]-rt[1]*lt[0]
    n=np.array([(a-b)/I,2*pp/I,2*qq/I]); n/=np.linalg.norm(n)
    th=np.arctan2(n[1],n[0])%(2*np.pi); ph=np.arccos(np.clip(n[2],-1,1))
    return n,th,ph
