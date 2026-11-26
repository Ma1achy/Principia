import numpy as np, json
from scipy.integrate import solve_ivp
exec(open('sim.py').read().split("def run(")[0])
exec(open('clear.py').read().split("res={}")[0])
p1,p2,T=0.08058,0.58884,21.2710
v=np.array([p1,p2]); y0=np.concatenate([np.ravel([[-1,0],[1,0],[0,0]]),np.ravel([v,v,-2*v])])
sol=solve_ivp(rhs,(0,T),y0,args=([1,1,1],),method='DOP853',rtol=1e-13,atol=1e-13,dense_output=True)
tt=np.linspace(0,T,120001)[:-1]; Y=sol.sol(tt)[:6].T.reshape(-1,3,2)
P=json.load(open('dragon_P.json'))
psi,phi,cx,cy,S,tau=P
dv=np.array([np.cos(phi),np.sin(phi),0.0]); e=np.array([0,0,1.0]); k=dv
e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
c,s=np.cos(psi),np.sin(psi)
X=(Y[...,0]*c-Y[...,1]*s)*S; Yy=(Y[...,0]*s+Y[...,1]*c)*S
W=np.stack([cx+X*dv[0]+Yy*e[0], cy+X*dv[1]+Yy*e[1], X*dv[2]+Yy*e[2]],-1)   # (n,3,3)
print('dense clearance hits', hits(W[::4].reshape(-1,3),0.095), 'of', W[::4].reshape(-1,3).shape[0])
# adaptive resampling per body: keep a point when the path has turned > 2.5 deg or moved > 0.035, or time gap > T/500
out=[]
for b in range(3):
    p=W[:,b,:]; keep=[0]; last_dir=None
    for i in range(1,len(p)):
        dvec=p[i]-p[keep[-1]]; dist=np.linalg.norm(dvec)
        if dist<1e-9: continue
        d1=dvec/dist
        turn=0 if last_dir is None else np.degrees(np.arccos(np.clip(d1@last_dir,-1,1)))
        if turn>2.5 or dist>0.035 or tt[i]-tt[keep[-1]]>T/500:
            keep.append(i); last_dir=(p[i]-p[keep[-2]])/np.linalg.norm(p[i]-p[keep[-2]])
    keep=np.array(keep)
    out.append({'t':np.round(tt[keep]/T,6).tolist(),'p':np.round(p[keep],4).ravel().tolist()})
    print('body',b,'samples',len(keep))
json.dump(out,open('dragon_adapt.json','w'))
