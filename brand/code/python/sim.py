import numpy as np, json
from scipy.integrate import solve_ivp
def rhs(t,y,m):
    p=y[:6].reshape(3,2); v=y[6:]; a=np.zeros((3,2))
    for i in range(3):
        for j in range(3):
            if i!=j:
                d=p[j]-p[i]; a[i]+=m[j]*d/np.linalg.norm(d)**3
    return np.concatenate([v,a.ravel()])
def energy(y,m):
    p=y[:6].reshape(3,2); v=y[6:].reshape(3,2)
    K=0.5*sum(m[i]*v[i]@v[i] for i in range(3))
    U=-sum(m[i]*m[j]/np.linalg.norm(p[i]-p[j]) for i in range(3) for j in range(i+1,3))
    return K+U
def run(p0,v0,m,T,n):
    y0=np.concatenate([np.ravel(p0),np.ravel(v0)])
    ts=np.linspace(0,T,n)
    s=solve_ivp(rhs,(0,T),y0,args=(m,),method='DOP853',rtol=1e-13,atol=1e-13,t_eval=ts)
    E0=energy(y0,m); E1=energy(s.y[:,-1],m)
    print('  dE/E', abs((E1-E0)/E0), ' closure', np.abs(s.y[:6,-1]-y0[:6]).max())
    return s.y[:6].T.reshape(-1,3,2)
out={}
print('figure8')
x1=np.array([-0.97000436,0.24308753]); v3=np.array([-0.93240737,-0.86473146])
out['eight']=run([x1,-x1,[0,0]],[-v3/2,-v3/2,v3],[1,1,1],6.32591398,361)
for name,(p1,p2,T) in {'butterfly':(0.30689,0.12551,6.2356),'moth':(0.46444,0.39606,14.8939),'yinyang':(0.51394,0.30474,17.3284)}.items():
    print(name); v=np.array([p1,p2])
    out[name]=run([[-1,0],[1,0],[0,0]],[v,v,-2*v],[1,1,1],T,901)
print('burrau')
out['burrau']=run([[1,3],[-2,-1],[1,-1]],[[0,0]]*3,[3,4,5],70,2801)
np.savez('traj.npz',**out)
