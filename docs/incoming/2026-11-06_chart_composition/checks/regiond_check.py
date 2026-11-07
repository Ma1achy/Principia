# Checks the region-D closed forms against chart_reference 0.2, 0.3, 0.5 and dd_validation_orbits 1.6 eq. 3, 4.
import math, random
G=6.67e-8
def state(xi,eta,m=(1/3,1/3,1/3),qa=None):
    m0,m1,m2=m; M01=m0+m1; mr=m0*m1/M01; ml=m2*M01
    rl=math.hypot(xi,eta)
    alpha=math.atan2(math.sqrt(ml)*rl,math.sqrt(mr)); beta=math.atan2(eta,xi) if rl else 0.0
    rt=(math.cos(alpha),0); lt=(math.sin(alpha)*math.cos(beta),math.sin(alpha)*math.sin(beta))
    rho=[x/math.sqrt(mr) for x in rt]; lam=[x/math.sqrt(ml) for x in lt]
    r01=[-m2*x for x in lam]; r2=[M01*x for x in lam]
    r0=[a-(m1/M01)*b for a,b in zip(r01,rho)]; r1=[a+(m0/M01)*b for a,b in zip(r01,rho)]
    return alpha,beta,(r0,r1,r2)
def phys(xi,eta):
    r=[(-.5,0.),(.5,0.),(xi,eta)]; c=[sum(x[i] for x in r)/3 for i in (0,1)]
    return [(x[0]-c[0],x[1]-c[1]) for x in r]
random.seed(1); worst=0; wp=0; wE=0
for _ in range(20000):
    while True:
        xi,eta=random.uniform(0,.5),random.uniform(0,.87)
        if (xi+.5)**2+eta**2<=1 and eta>1e-3: break
    a,b,rc=state(xi,eta); rp=phys(xi,eta)
    I0=sum((1/3)*(x*x+y*y) for x,y in rp); ell=math.sqrt(I0)
    assert abs(I0-(1/6+(2/9)*(xi*xi+eta*eta)))<1e-14
    for c,p in zip(rc,rp):
        worst=max(worst,abs(c[0]-p[0]/ell),abs(c[1]-p[1]/ell))
    # eq.3 momenta after unit_state then C
    v=[(5e-5,0),(5e-5,0),(-1e-4,0)]
    p=[tuple((1/3)*x/math.sqrt(G*3) for x in vi) for vi in v]   # M=3 start: (m_i/M) v_i / sqrt(GM)
    p=[tuple(math.sqrt(ell)*x for x in pi) for pi in p]
    p_lam=p[2]; p_rho=(p[1][0]+.5*p_lam[0],p[1][1]+.5*p_lam[1])
    q2=-(1e-4)/(3*math.sqrt(3*G))*math.sqrt(ell)
    wp=max(wp,abs(p_lam[0]-q2),abs(p_rho[0]),abs(p_rho[1]),abs(p_lam[1]))
    # energy: paper E vs canonical
    r12=1; r13=math.hypot(xi+.5,eta); r23=math.hypot(xi-.5,eta)
    Ep=-G*(1/r12+1/r13+1/r23)+7.5e-9
    def u(rr,mm=1/3):
        d=lambda a,b: math.hypot(a[0]-b[0],a[1]-b[1])
        return -mm*mm*(1/d(rr[0],rr[1])+1/d(rr[0],rr[2])+1/d(rr[1],rr[2]))
    Kc=sum((pi[0]**2+pi[1]**2)/(2/3) for pi in p); Ec=Kc+u(rc)
    Eback=G*9*Ec/ell   # E = G M^2 lambda E_c, lambda = 1/ell
    wE=max(wE,abs(Eback/Ep-1))
print('positions',worst,'momenta',wp,'energy',wE, 'q2 at ell=.5', -(1e-4)/(3*math.sqrt(3*G))*math.sqrt(.5))
for pt,n in (((0,0),'(0,0)'),((.5,0),'(1/2,0)'),((0,math.sqrt(3)/2),'apex')):
    I0=1/6+(2/9)*(pt[0]**2+pt[1]**2); ell=math.sqrt(I0); print(n,'ell',round(ell,4),'ell^1.5',round(ell**1.5,4))
# stereographic identity: (xi,eta) = sqrt(mu_rho/mu_lambda) (n_v, n_w)/(1+n_u)
import random
worst=0
for _ in range(20000):
    m=[random.uniform(.05,1) for _ in range(3)]; t=sum(m); m=[x/t for x in m]
    xi,eta=random.uniform(-2,2),random.uniform(0,2)
    M01=m[0]+m[1]; mr=m[0]*m[1]/M01; ml=m[2]*M01
    a=mr; b=ml*(xi*xi+eta*eta); p=math.sqrt(mr*ml)*xi; q=math.sqrt(mr*ml)*eta
    I=a+b; n=((a-b)/I,2*p/I,2*q/I)
    k=math.sqrt(mr/ml)
    worst=max(worst,abs(k*n[1]/(1+n[0])-xi),abs(k*n[2]/(1+n[0])-eta),abs(math.sqrt(sum(x*x for x in n))-1))
    # interior angles and side ratios invert to (xi,eta) in Anosova's frame
    g0=math.atan2(eta,xi+.5); g1=math.atan2(eta,.5-xi)
    if eta>1e-6 and g0>1e-6 and g1>1e-6:
        e2=1/(1/math.tan(g0)+1/math.tan(g1)); x2=-.5+e2/math.tan(g0)
        worst=max(worst,abs(e2-eta)/(1+eta),abs(x2-xi)/(1+abs(xi)))
    A=math.hypot(xi+.5,eta); B=math.hypot(xi-.5,eta)
    x3=(A*A-B*B)/2; e3=math.sqrt(max(A*A-(x3+.5)**2,0))
    worst=max(worst,abs(x3-xi),abs(e3-eta)/(1+eta)) if eta>1e-3 else worst
print('stereographic, angle and ratio inverses: worst',worst)
