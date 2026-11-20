# Hold L_z's tags, RQ-715's question (physics review 5809833272, Z5; code review 5809819710). s = 0 and K-hat = 0 against the exact predicate
# rigid(v, Jr) at five values of tau, Dirichlet masses, on f32-representable inputs. The column "s/Khat tags 32:64 disagree" at ratio 0 is the 16.5%.
# Python with Fraction: tau's calibration at masses log-uniform to 1e-3 and on the two GPU backends is still owed (RQ-715).
import numpy as np
from fractions import Fraction as F
rng=np.random.default_rng(9)
J=lambda r:np.stack([-r[:,1],r[:,0]],-1)
def ip(m,a,b):return (m[:,None]*a*b).sum()
def comp(m,r,v,T):
    m=m.astype(T);r=r.astype(T);v=v.astype(T);w=J(r).astype(T);I=ip(m,r,r);L=ip(m,w,v);dv=v-(L/I)*w;s=np.abs(dv).max()
    if s==0:return 's0',None
    d=dv/s;d=d-(ip(m,w,d)/I)*w;K=T(.5)*ip(m,d,d)
    if K==0:return 'K0',None
    return 'go',(m,w,d,K,I)
def wedge_ratio(r,v):
    w=[F(float(x)) for x in J(r).ravel()];u=[F(float(x)) for x in v.ravel()]
    p=max(range(6),key=lambda i:abs(w[i]));V=max(abs(x) for x in u)
    if V==0:return F(0)
    return max(abs(u[b]*w[p]-u[p]*w[b]) for b in range(6))/(V*abs(w[p]))
def state():
    mm=rng.dirichlet([3,3,3]);a=rng.uniform(0,np.pi/2);b=rng.uniform(-3.1,3.1);M=mm[0]+mm[1];mr=mm[0]*mm[1]/M;ml=mm[2]*M
    rho=np.array([np.cos(a),0])/np.sqrt(mr);lam=np.sin(a)*np.array([np.cos(b),np.sin(b)])/np.sqrt(ml)
    r01=-mm[2]*lam;return mm,np.array([r01-mm[1]/M*rho,r01+mm[0]/M*rho,M*lam])
taus=[2.**-k for k in(22,21,20,19,18)]
print('ratio  | s/Khat tags 32:64 disagree | exact zero wedge, computed tag at f32 with nonzero wedge | per tau: tagged share; go-with-K0 count above tau; worst dLz/a f32 above tau')
for ratio in(0,1e-7,3e-7,1e-6,3e-6,1e-5):
    N=6000;dis=0;zw=0;wz=0;R=[]
    st={t:[0,0,0.] for t in taus};nw=0
    for _ in range(N):
        m,r=state();c=10**rng.uniform(-3,0);m=m.astype(np.float32).astype(float);r=r.astype(np.float32).astype(float)
        w=J(r);v=(c*w);
        if ratio: 
            n=rng.normal(size=v.shape);v=v+ratio*np.abs(v).max()*n/np.abs(n).max()
        v=v.astype(np.float32).astype(float)
        a32,x=comp(m,r,v,np.float32);a64,_=comp(m,r,v,np.float64)
        dis+=(a32=='go')!=(a64=='go')
        q=wedge_ratio(r,v)
        if q==0: zw+=1
        elif a32!='go': wz+=1
        for t in taus:
            if q<=F(t): st[t][0]+=1
            else:
                if a32!='go': st[t][1]+=1
                else:
                    ma,wa,d,K,I=x;a=np.float32(1.);T=np.float32
                    eta=np.sqrt(T(.5)/K);out=(T(.1)/I)*wa+eta*d
                    st[t][2]=max(st[t][2],abs(float(ip(ma,wa,out))-.1))
    print('%.0e | %.3f | zero wedge %.3f, nonzero wedge but no direction %.3f | '%(ratio,dis/N,zw/N,wz/N)+' '.join('2^%d: %.3f %d %.1e'%(np.log2(t),st[t][0]/N,st[t][1],st[t][2]) for t in taus))
