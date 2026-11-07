import numpy as np
exec(open(__import__("os").path.join(__import__("os").path.dirname(__import__("os").path.abspath(__file__)),"morph_check.py")).read().split('print("T1')[0])
rng=np.random.default_rng(7)
def geo(QA,QB,t):
    U,c,Vt=np.linalg.svd(QA.T@QB);c=np.clip(c,-1,1);th=np.arccos(c);A=QA@U;B=QB@Vt.T
    W=np.zeros_like(A)
    for k in range(2):
        w=B[:,k]-A[:,k]*c[k];n=np.linalg.norm(w);W[:,k]=w/n if n>1e-14 else 0
    return A*np.cos(t*th)+W*np.sin(t*th)
def rod(q1,q2,dt,t1,t2):   # sheet
    m=np.hypot(t1,t2)
    k=(t2*q1-t1*q2)/m
    def R(x): return x*np.cos(m)+np.cross3(k,x)*np.sin(m) if False else None
    # 3-space (q1,q2,d): rotation about k by m taking q1 toward d
    a=np.array([t1,t2])/m
    e=q1*a[0]+q2*a[1]; f=-q1*a[1]+q2*a[0]
    em=np.cos(m)*e+np.sin(m)*dt
    return em*a[0]-f*a[1], em*a[1]+f*a[0]
def dist(a,b):
    Pa=np.linalg.qr(a)[0];Pb=np.linalg.qr(b)[0];return np.linalg.norm(Pa@Pa.T-Pb@Pb.T)
res=[];bits=0;tot=0;lean=[];two=[]
for _ in range(4000):
    Q=np.linalg.qr(rng.normal(size=(8,4)))[0];q1,q2,d,d2=Q.T
    t1,t2=rng.uniform(-1.05,1.05,2);f=rng.uniform(.1,.9)
    b1,b2=rod(q1,q2,d,t1,t2);m1,m2=rod(q1,q2,d,f*t1,f*t2)
    G=geo(np.c_[q1,q2],np.c_[b1,b2],f)
    res.append(dist(G,np.c_[m1,m2]));
    tot+=1;bits+= (dist(G,np.c_[m1,m2])==0)
    # lean, one target
    lb1=np.cos(t1)*q1+np.sin(t1)*d;lb2=np.cos(t2)*q2+np.sin(t2)*d
    lm1=np.cos(f*t1)*q1+np.sin(f*t1)*d;lm2=np.cos(f*t2)*q2+np.sin(f*t2)*d
    lean.append(dist(geo(np.c_[q1,q2],np.linalg.qr(np.c_[lb1,lb2])[0],f),np.c_[lm1,lm2]))
    # two targets
    tb1=np.cos(t1)*q1+np.sin(t1)*d;tb2=np.cos(t2)*q2+np.sin(t2)*d2
    tm1=np.cos(f*t1)*q1+np.sin(f*t1)*d;tm2=np.cos(f*t2)*q2+np.sin(f*t2)*d2
    two.append(dist(geo(np.c_[q1,q2],np.c_[tb1,tb2],f),np.c_[tm1,tm2]))
print("P1 sheet vs SVD geodesic plane dist worst %.1e median %.1e, exactly equal bases n/a; lean one target median %.3f worst %.3f; two targets worst %.1e"%(max(res),np.median(res),np.median(lean),max(lean),max(two)))
# frame bits
q1,q2,d=np.linalg.qr(rng.normal(size=(8,3)))[0].T
b1=np.cos(.7)*q1+np.sin(.7)*d;G=geo(np.c_[q1,q2],np.c_[b1,q2],.5);m1=np.cos(.35)*q1+np.sin(.35)*d
Fq=np.c_[m1,q2];PG=G@G.T;PF=Fq@Fq.T
print(" one-axis tilt, mid: projector entries bitwise equal %d of 64, worst diff %.1e"%((PG==PF).sum(),abs(PG-PF).max()))
print("P2 curved: tilt of the sphere chart at s vs z-blend of its ends")
R=lambda t:np.array([[np.cos(t),0,-np.sin(t)],[0,1,0],[np.sin(t),0,np.cos(t)]])
def sph(a,b):
    th,ph=2*np.pi*a,np.pi*(1-b);return np.array([np.sin(ph)*np.cos(th),np.sin(ph)*np.sin(th),np.cos(ph)])
w=[]
for _ in range(3000):
    u,v=rng.uniform(-.2,.2,2);t=rng.uniform(.2,1.2);h0=.5;v0=.5
    def chart(tt):
        a,b,dd=R(tt)@np.array([u,v,0]);return sph(.25+a,.75+b),dd
    na,_x=chart(0);nb,db=chart(t);nm,dm=chart(t/2)
    if na[2]<.02 or nb[2]<.02: continue
    m=np.ones(3)/3;za=Ez(m,na,np.zeros(4));zb=Ez(m,nb,np.zeros(4));ns=Dz(.5*(za+zb))[1];w.append(np.linalg.norm(ns-nm))
print(" |n(z blend)-n(tilt)| median %.3f worst %.3f of %d"%(np.median(w),max(w),len(w)))
print("P3 retime: speed spread, collinear-end example")
n=np.array([.6,.8,0.]);nb=np.array([.6,0,.8]);m=np.ones(3)/3;za=Ez(m,n,np.zeros(4));zb=Ez(m,nb,np.zeros(4))
sg=np.linspace(0,1,33);pts=np.array([Dz(za+t*(zb-za))[1] for t in sg]);arc=np.r_[0,np.cumsum(np.linalg.norm(np.diff(pts,axis=0),axis=1))];arc/=arc[-1]
def g(x): return np.interp(x,arc,sg)
xs=np.linspace(0,1,61);a=np.array([Dz(za+g(x)*(zb-za))[1] for x in xs]);sp=np.linalg.norm(np.diff(a,axis=0),axis=1);b=np.array([Dz(za+x*(zb-za))[1] for x in xs]);sq=np.linalg.norm(np.diff(b,axis=0),axis=1)
print(" max/median speed raw %.1f, retimed with 33 knots %.2f; g(0)=%r g(1)=%r"%(sq.max()/np.median(sq),sp.max()/np.median(sp),float(g(0)),float(g(1))))
print("P4 anchor: encode round trip bits")
same=0;N=20000;d32=0;wz=[];f32eq=0
for _ in range(N):
    z=rng.uniform(-4,4,8);m,n,q=Dz(z);z2=Ez(m,n,q);m2,n2,q2=Dz(z2)
    same+= (z2==z).all(); wz.append(np.abs(np.r_[m2-m,n2-n,q2-q]).max())
    zf=z.astype(np.float32);sf=lambda x:(1/(1+np.exp(-x.astype(np.float32)))).astype(np.float32)
    nf=np.float32(np.cos(np.float32(np.pi/2)*2*sf(zf[:1])))[0]; f32eq+= (nf==np.float32(np.cos(np.pi*s(zf[0].astype(float)))))
print(" E(D(z))==z bitwise %d of %d; D(E(D(z))) vs D(z) worst %.1e; one f32 component equal to the rounded f64: %.2f"%(same,N,max(wz),f32eq/N))
a=np.float64(-0.0);print(" -0 + (x-x) bits:",np.signbit(a+(1.5-1.5)),"(False = +0: the offset form alone changes the bit)")
print("P5 equal ends: a+s(b-a) with a==b: exact except -0")
print("P6 small ds: worst IC step per ds over a morph on blocks")
w=[]
for _ in range(4000):
    na,nb=rs(),rs();qa,qb=rng.uniform(-1,1,4),rng.uniform(-1,1,4);t=rng.uniform();ds=1e-4
    x1=np.r_[slerp(na,nb,t),qa+t*(qb-qa)];x2=np.r_[slerp(na,nb,t+ds),qa+(t+ds)*(qb-qa)];w.append(np.linalg.norm(x2-x1)/ds)
print(" median %.2f worst %.2f (pi+|dq| bound)"%(np.median(w),max(w)))
