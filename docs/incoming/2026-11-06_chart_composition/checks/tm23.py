import numpy as np
cr=lambda a,b:a[0]*b[1]-a[1]*b[0]
def build(x,y,a,b,m,frame):
    A=[np.array(p,float) for p in [(x,y),(-.5,0),(.5,0)]]
    m=np.array(m,float)
    if frame=='centroid': c=sum(A)/3
    else: c=sum(mi*p for mi,p in zip(m,A))/m.sum()
    R=[p-c for p in A]
    V1=-2*a*R[0]+b*R[1]; V2=a*R[0]-2*b*R[1]; V3=-V1-V2
    return m,A,R,[V1,V2,V3]
def com_L(m,R,V):
    M=m.sum();rc=sum(mi*r for mi,r in zip(m,R))/M;vc=sum(mi*v for mi,v in zip(m,V))/M
    return sum(mi*cr(r-rc,v-vc) for mi,r,v in zip(m,R,V)),sum(mi*v for mi,v in zip(m,V))
for lab,(x,y,a,b,m) in {'Codex':(0,.3,.1,0,(1,.8,.6)),'Codex beta':(.2,.3,.1,.03,(1,.8,.6)),'eq':(.2,.3,.1,.03,(1,1,1)),'eq scaled':(.2,.3,.1,.03,(1/3,1/3,1/3))}.items():
    for fr in('centroid','com'):
        m_,A,R,V=build(x,y,a,b,m,fr)
        L,P=com_L(m_,R,V);print(lab,fr,'L_com %.6e'%L,'P',P)
# analytic: L_com for centroid-frame R
rng=np.random.default_rng(2)
w={'centroid':0,'com':0};wf=0
for _ in range(20000):
    m=rng.dirichlet([3,3,3]);x,y=rng.uniform(-1,1,2);a,b=rng.uniform(-1,1,2)
    for fr in w:
        m_,A,R,V=build(x,y,a,b,m,fr);L,_p=com_L(m_,R,V);w[fr]=max(w[fr],abs(L))
print(w)
# which body plays which label: permuting which body sits at the apex
