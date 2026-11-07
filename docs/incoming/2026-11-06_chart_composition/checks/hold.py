import numpy as np
rng=np.random.default_rng(1)
J=lambda a:np.stack([-a[...,1],a[...,0]],-1)
def ip(a,b,m): return np.sum(m[:,None]*a*b)
def L(r,v,m): return np.sum(m*(r[:,0]*v[:,1]-r[:,1]*v[:,0]))
def U(r,m):
    return -sum(m[i]*m[j]/np.linalg.norm(r[i]-r[j]) for i,j in [(0,1),(0,2),(1,2)])
def state():
    m=rng.dirichlet([3,3,3])
    r=rng.normal(size=(3,2)); r-=np.sum(m[:,None]*r,0); r/=np.sqrt(np.sum(m[:,None]*r*r))
    v=rng.normal(size=(3,2)); v-=np.sum(m[:,None]*v,0)
    return m,r,v
# 1 exact identities in f64
w=np.zeros(6)
for _ in range(20000):
    m,r,v=state(); I=ip(r,r,m); L0=L(r,v,m); K0=.5*ip(v,v,m); u=U(r,m)
    vL=L0/I*J(r); d=v-vL; Kd=.5*ip(d,d,m)
    w[0]=max(w[0],abs(ip(J(r),v,m)-L0))           # L = <Jr,v>
    w[1]=max(w[1],abs(ip(vL,d,m)))                 # orthogonal
    w[2]=max(w[2],abs(K0-L0**2/2/I-Kd))            # Pythagoras
    Ls=rng.normal(); a2=rng.exponential(); Es=u+Ls**2/2/I+a2
    eta=np.sqrt((Es-u-Ls**2/2/I)/Kd); vn=Ls/I*J(r)+eta*d
    w[3]=max(w[3],abs(L(r,vn,m)-Ls)); w[4]=max(w[4],abs(.5*ip(vn,vn,m)+u-Es)); w[5]=max(w[5],np.abs(np.sum(m[:,None]*vn,0)).max())
    # nearest point on fibre: random other fibre points are farther
    for __ in range(3):
        x=rng.normal(size=(3,2)); x-=np.sum(m[:,None]*x,0); x-=L(r,x,m)/I*J(r); x*=np.sqrt(2*a2/ip(x,x,m))
        assert ip(vn-v,vn-v,m)<=ip(Ls/I*J(r)+x-v,Ls/I*J(r)+x-v,m)+1e-12
print('f64 worst: L=<Jr,v>,orth,Pyth,L*,E*,Sp',w)
# 2 f32 amplification
def run(frac,N=4000,passes=1,kdform='direct'):
    F=np.float32; errL=[];errE=[];neg=0
    for _ in range(N):
        m,r,v=state()
        vL=L(r,v,m)*J(r)*1.0
        d=v-vL; d/=np.sqrt(ip(d,d,m)); c=rng.normal()*1.2+0.1
        vt=c*J(r)+frac*abs(c)*d       # near-rigid decode with |delta|/|v|~frac
        Ls=0.7; a2=1.0
        mf,rf,vf=m.astype(F),r.astype(F),vt.astype(F)
        L0=np.sum(mf*(rf[:,0]*vf[:,1]-rf[:,1]*vf[:,0]),dtype=F)
        df=vf-L0*(np.stack([-rf[:,1],rf[:,0]],-1))
        if kdform=='sub':
            K0=F(.5)*np.sum(mf[:,None]*vf*vf,dtype=F); Kd=K0-L0*L0/F(2)
            if Kd<=0: neg+=1; continue
            eta=np.sqrt(F(a2)/Kd); dn=eta*df
        else:
            s=np.abs(df).max(); dh=df/s
            for p in range(passes-1):
                b=np.sum(mf*(rf[:,0]*dh[:,1]-rf[:,1]*dh[:,0]),dtype=F)
                dh=dh-b*np.stack([-rf[:,1],rf[:,0]],-1)
            kh=F(.5)*np.sum(mf[:,None]*dh*dh,dtype=F)
            dn=np.sqrt(F(a2)/kh)*dh
        vn=(F(Ls)*np.stack([-rf[:,1],rf[:,0]],-1)+dn).astype(F)
        rr=rf.astype(float);vv=vn.astype(float);mm=mf.astype(float)
        errL.append(abs(L(rr,vv,mm)-Ls)); errE.append(abs(.5*ip(vv,vv,mm)-(Ls**2/2+a2)))
    return np.median(errL),np.max(errL),np.max(errE),neg
print('frac  | one pass med/max L err,maxKerr | two passes | K_delta by subtraction')
for f in [1e-1,1e-2,1e-3,1e-4,1e-5,1e-6,1e-7]:
    a=run(f); b=run(f,passes=2); c=run(f,kdform='sub')
    print(f'{f:.0e}', '%.1e %.1e %.1e'%a[:3], '|', '%.1e %.1e %.1e'%b[:3], '|', '%.1e %.1e %.1e neg=%d'%c)
# 3 rank
def jac(f,uv,h=1e-6):
    a=(f(uv[0]+h,uv[1])-f(uv[0]-h,uv[1]))/(2*h); b=(f(uv[0],uv[1]+h)-f(uv[0],uv[1]-h))/(2*h)
    return np.linalg.svd(np.stack([a.ravel(),b.ravel()]),compute_uv=False)
m,r,v=state(); I=1.0; 
def hoL(x,Ls=.5): return x+(Ls-L(r,x,m))/ip(r,r,m)*J(r)
def both(x,Ls=.5,a2=1.):
    I=ip(r,r,m); d=x-L(r,x,m)/I*J(r); return Ls/I*J(r)+np.sqrt(2*a2/ip(d,d,m))*d
e=[]
for _ in range(2):
    x=rng.normal(size=(3,2)); x-=np.sum(m[:,None]*x,0); e.append(x)
Jr=J(r)
print('plane generic  both   ',jac(lambda a,b:both(v+a*e[0]+b*e[1]),(.3,.2)))
print('plane contains Jr  L  ',jac(lambda a,b:hoL(v+a*Jr+b*e[1]),(.3,.2)))
print('plane through rest both',jac(lambda a,b:both(a*e[0]+b*e[1]),(.3,.2)))
print('plane through rest L  ',jac(lambda a,b:hoL(a*e[0]+b*e[1]),(.3,.2)))
print('plane through rigid both',jac(lambda a,b:both(1.3*Jr+a*e[0]+b*e[1]),(.3,.2)))
print('plane through rest, E only',jac(lambda a,b:(lambda x:x/np.sqrt(ip(x,x,m)))(a*e[0]+b*e[1]),(.3,.2)))
