exec(open('hole4.py').read().split("res=[]")[0])
def hitsH(pts,m,HH):
    xy=pts[:,:2]; z=np.abs(pts[:,2])
    inmat=po.contains_points(xy)&~ph.contains_points(xy)
    db=np.minimum(segdist(xy,OUT),segdist(xy,HOLE))
    return ((inmat|(db<m))&(z<HH+m)).sum()
import collections
for HH in [0.2,0.14,0.1]:
    bestc=None; cnt=0
    for psi,phi,cx,cy,S,nh in cands[:700]:
        dvec=np.array([np.cos(phi),np.sin(phi),0.0]); c,s=np.cos(psi),np.sin(psi)
        X=(mo[...,0]*c-mo[...,1]*s)*S; Y=(mo[...,0]*s+mo[...,1]*c)*S
        for tau in np.linspace(-0.75,0.75,7):
            e=np.array([0,0,1.0]); k=dvec; e=e*np.cos(tau)+np.cross(k,e)*np.sin(tau)+k*(k@e)*(1-np.cos(tau))
            n=np.cross(dvec,e); face=abs(n@v)
            if face<0.45: continue
            W=np.stack([cx+X*dvec[0]+Y*e[0], cy+X*dvec[1]+Y*e[1], X*dvec[2]+Y*e[2]],-1)
            h=hitsH(W[::2].reshape(-1,3),0.095,HH)
            if bestc is None or h<bestc[0]: bestc=(h,nh,round(float(face),2),[round(float(x),3) for x in (psi,phi,cx,cy,S,tau)])
            if h==0: cnt+=1
    print('thickness',HH*2,'zero-hit configs',cnt,'best',bestc)
