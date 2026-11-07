# A5: the exact map (xi,eta) <-> bodies <-> n-hat, any masses and any paper frame. Each case: published coordinates -> state
# in the chart's gauge -> back. numpy; f64 and f32.
import numpy as np
def to_state(xi,eta,m,T=np.float64):
    xi,eta=T(xi),T(eta); m=np.array(m,dtype=T)
    M01=m[0]+m[1]; M=m.sum()
    d=(m[1]-m[0])/(2*M01); mr=m[0]*m[1]/M01; ml=M01*m[2]/M
    lx,ly=xi-d,eta
    ell2=mr+ml*(lx*lx+ly*ly); ell=np.sqrt(ell2)
    rt=np.array([np.sqrt(mr)/ell,T(0)],dtype=T); lt=np.array([np.sqrt(ml)*lx/ell,np.sqrt(ml)*ly/ell],dtype=T)
    return rt,lt,d,mr,ml
def back_jacobi(rt,lt,d,mr,ml):
    # complex ratio lambda/rho
    w=complex(lt[0],lt[1])/complex(rt[0],rt[1])*np.sqrt(mr/ml)
    return d+w.real,w.imag
def back_sphere(rt,lt,d,mr,ml):
    u=rt@rt-lt@lt; v=2*rt@lt; w=2*(rt[0]*lt[1]-rt[1]*lt[0])
    s=np.sqrt(mr/ml)
    return d+s*v/(1+u),s*w/(1+u)
def ulp(a,b,T):
    return abs(a-b)/np.spacing(T(max(abs(a),abs(b),1e-30)))
def bodies(rA,rB,rC):
    return [np.array(x,float) for x in (rA,rB,rC)]
def frame_xi(rs,m):
    # any frame: complex ratio, invariant under translation, rotation, scale
    z=[complex(*r) for r in rs]; M01=m[0]+m[1]
    rho=z[1]-z[0]; lam=z[2]-(m[0]*z[0]+m[1]*z[1])/M01
    d=(m[1]-m[0])/(2*M01); w=lam/rho
    return d+w.real,w.imag
cases=[("AA I N=1-like",.2,.3,(1,1,1)),("LL F1 (1,.8,.8)",.0009114239,.3019805958,(1,.8,.8)),
 ("LL F24",.1527845023,.068494294,(1,.8,.8)),("LL F5 (1,.8,.6)",.129088109,.4010761427,(1,.8,.6)),
 ("LL F27 (1,.8,.4)",.083924021,.3307729197,(1,.8,.4)),("LL F6 (1,.8,.2)",.1310631652,.3036588095,(1,.8,.2)),
 ("AN-1 row 1 (9,3,1)",1.530844530,.281758510,(9,3,1)),("AN-1 row1 neg xi",-1.530844530,.281758510,(9,3,1)),
 ("AN-1 row 51 (3,3,1)",9.897502790,.351546184,(3,3,1)),("TM-1 TCP",.19208270,.30936018,(1,1,1)),("Burrau (3,4,5)",.14,.48,(3,4,5))]
print("case | f64 ulp jacobi | f64 ulp sphere | f32 ulp sphere | in D")
for n,x,y,m in cases:
    for T in (np.float64,):
        a=to_state(x,y,m,T); j=back_jacobi(*a); s=back_sphere(*a)
    b=to_state(x,y,m,np.float32); s3=back_sphere(*b)
    u=lambda p,T:max(ulp(p[0],T(x),T),ulp(p[1],T(y),T))
    ind=x>=0 and y>=0 and (x+.5)**2+y**2<=1
    print(n,"|",f"{u(j,np.float64):.1f} | {u(s,np.float64):.1f} | {u(s3,np.float32):.1f} | {ind}")
rng=np.random.default_rng(1); w64=w32=0;ws=0
for i in range(200000):
    m=rng.dirichlet((1,1,1))*3+1e-3; x=rng.uniform(0,.5); y=rng.uniform(0,.87)
    if (x+.5)**2+y**2>1: continue
    a=to_state(x,y,m); s=back_sphere(*a); w64=max(w64,abs(s[0]-x),abs(s[1]-y))
    if i%10==0:
        b=to_state(x,y,m,np.float32); s=back_sphere(*b); w32=max(w32,abs(float(s[0])-x),abs(float(s[1])-y))
print("random masses over D: worst |d| f64",w64,"f32",w32)
# frames
for nm,rs,m,exp in [("Burrau bodies m3 m4 m5",bodies((1,3),(-2,-1),(1,-1)),(3,4,5),(.14,.48)),
 ("Lehto (.3,.2) lens: (0,0),(1,0)",bodies((0,0),(1,0),(.3,.2)),(1,1,1),(-.2,.2)),
 ("book x2",bodies((-1,0),(1,0),(.4,.6)),(1,1,1),(.2,.3)),
 ("Heggie H-3 dx=0 dy=1e-4",bodies((-1,0),(1,0),(0,np.sqrt(3)+1e-4)),(1,1,1),(0,(np.sqrt(3)+1e-4)/2))]:
    print(nm,frame_xi(rs,m),"expected",exp)
# Breen: x1=(1,0), x2=(a,b), x3=-x1-x2; Moebius to (xi,eta) with A=x1,B=x2
a,b=-.3,.4; rs=bodies((1,0),(a,b),(-1-a,-b)); print("Breen via ratio",frame_xi(rs,(1,1,1)),"closed", (lambda z:(z.real,z.imag))(-1.5*(1+complex(a,b))/(complex(a,b)-1)))
print("Breen singular: x2=(-.5,0): x3=",(-1+.5,0))
# Parischewsky: S1,S2,S3 opposite 1,2,3; bodies 1,2 at A,B: S3=r12=1 scale; a=r02/r01=S2/S3; b=r12/r01=S1/S3
A,B,C=[np.array(p) for p in((-.5,0.),(.5,0.),(.1,.6))]
S1=np.linalg.norm(B-C);S2=np.linalg.norm(A-C);S3=np.linalg.norm(A-B)
p,q=S1/S2,S1/S3; a=(q/p); b=q; xi=(a*a-b*b)/2; eta=np.sqrt(a*a-(xi+.5)**2); print("Parischewsky",(p,q),"->",(xi,eta))
# TM23
x,y,a2,b1=.1994373,.3164830,.1,.03
R1=np.array([2/3*x,2/3*y]);R2=-np.array([x,y])/3-[.5,0];R3=-np.array([x,y])/3+[.5,0]
V1=-2*a2*R1+b1*R2;V2=a2*R1-2*b1*R2;V3=-V1-V2
cr=lambda u,v:u[0]*v[1]-u[1]*v[0]
print("TM23 P",V1+V2+V3,"L",cr(R1,V1)+cr(R2,V2)+cr(R3,V3))
# focal
r1,r2=np.linalg.norm(C-A),np.linalg.norm(C-B);s=(r1+r2);t=(r1-r2);print("focal",s*t/2-C[0],.5*np.sqrt((s*s-1)*(1-t*t))-C[1])
# LLL
print("LLL v2",-1*1.21800411067968*-0.372008640907423)
# Routh
m=np.array([1,.8,.6]);print("Routh",(m[0]*m[1]+m[0]*m[2]+m[1]*m[2])/m.sum()**2, "equal",3/9)
# rho_2 alias, rest tidal alias
print("rho_2 at (xi,eta)=(.2,.3),m=(1,.8,.6): d_out/d_pair",np.hypot(.2-(.8-1)/(2*1.8),.3))
