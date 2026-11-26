import numpy as np, json, itertools
from matplotlib.path import Path
d=np.load('traj.npz'); g=json.load(open('pgeo.json'))
OUT=np.array(g['out']); HOLE=np.array(g['hole']); H=0.2
po,ph=Path(OUT),Path(HOLE)
def segdist(P,poly):
    A=poly; B=np.roll(poly,-1,0); AB=B-A
    AP=P[:,None,:]-A[None]; t=np.clip((AP*AB).sum(-1)/(AB*AB).sum(-1),0,1)
    C=A[None]+t[...,None]*AB[None]; return np.sqrt(((P[:,None,:]-C)**2).sum(-1)).min(1)
def hits(pts,m):
    """points (n,3) within margin m of the P solid"""
    xy=pts[:,:2]; z=np.abs(pts[:,2])
    inmat=po.contains_points(xy)&~ph.contains_points(xy)
    db=np.minimum(segdist(xy,OUT),segdist(xy,HOLE))
    near=(inmat|(db<m))&(z<H+m)
    # also points inside material footprint but within slab are hits; near-edge in front/behind with z>H+m are fine
    return near.sum()
def report(name,pts,m): print(name,'hits',hits(pts,m),'of',len(pts))
res={}
# --- Moth (V, VI): at = [x0 + X cos th, y0 + Y, X sin th]
mo=d['moth'][:-1].reshape(-1,2); 
best=None
for th,x0,y0,S in itertools.product(np.linspace(-1.25,-0.5,16),np.linspace(-0.5,1.0,16),np.linspace(-0.4,0.6,11),[1.2,1.3,1.4]):
    X=mo[:,0]*S; Yy=mo[:,1]*S
    pts=np.column_stack([x0+X*np.cos(th),y0+Yy,X*np.sin(th)])
    h=hits(pts[::3],0.13)
    face=abs(np.cos(th+0.62))  # how square-on the orbit plane is to the camera
    score=h*10 - face*3 + abs(x0)*0.6 + abs(y0-0.05)*0.6
    if best is None or score<best[0]: best=(score,h,th,x0,y0,S,face)
print('moth',best); res['moth']=best
# --- Eight (Threaded): at = [x0 + X cos th, y0 + Y, X sin th]
e=d['eight'][:-1,0,:]
best=None
for th,x0,y0,S in itertools.product(np.linspace(0.5,1.5,11),np.linspace(-0.3,0.3,13),np.linspace(0.2,0.5,7),[1.3,1.42,1.55]):
    X=e[:,0]*S; Yy=e[:,1]*S
    pts=np.column_stack([x0+X*np.cos(th),y0+Yy,X*np.sin(th)])
    h=hits(pts,0.12); score=h*10+abs(th-0.72)*0.5+abs(x0)+abs(y0-0.36)
    if best is None or score<best[0]: best=(score,h,th,x0,y0,S)
print('eight',best); res['eight']=best
# --- Lagrange ring (Slices): at = [x0 + R cos a cos th, y0 + R sin a, R cos a sin th]
best=None
a=np.linspace(0,2*np.pi,400,endpoint=False)
for th,x0,y0,R in itertools.product(np.linspace(0.3,1.4,12),np.linspace(-0.2,0.6,9),np.linspace(0.0,0.5,6),[1.15,1.22,1.3,1.4]):
    pts=np.column_stack([x0+R*np.cos(a)*np.cos(th),y0+R*np.sin(a),R*np.cos(a)*np.sin(th)])
    h=hits(pts,0.17); score=h*10+abs(th-0.62)*0.5+abs(x0)+abs(y0-0.36)
    if best is None or score<best[0]: best=(score,h,th,x0,y0,R)
print('lagrange',best); res['lagrange']=best
# --- Burrau ring: at = [X ca + Z sa, y0, -X sa + Z ca] (horizontal plane), allow a tilt about x
b=d['burrau'][::2][:1281].reshape(-1,2)
best=None
for y0,S,tilt in itertools.product(np.linspace(-1.4,-0.6,17),[0.55,0.62],[0,0.15,0.3]):
    X=b[:,0]*S; Z=b[:,1]*S; ca,sa=np.cos(0.35),np.sin(0.35)
    x=X*ca+Z*sa; z=-X*sa+Z*ca; y=y0+z*np.sin(tilt); z=z*np.cos(tilt)
    pts=np.column_stack([x,y,z]); h=hits(pts[::2],0.1); score=h*10+abs(y0+0.7)
    if best is None or score<best[0]: best=(score,h,y0,S,tilt)
print('burrau',best); res['burrau']=best
# --- Butterfly (Wire): at = [x0 + X cos th, y0 + 1.6 Y, X sin th]
bt=d['butterfly'][:-1].reshape(-1,2)
best=None
for th,x0,y0,S in itertools.product(np.linspace(0.4,1.5,12),np.linspace(-0.3,0.3,13),np.linspace(0.2,0.5,7),[1.6,1.9]):
    X=bt[:,0]*S; Yy=bt[:,1]*S*1.6
    pts=np.column_stack([x0+X*np.cos(th),y0+Yy,X*np.sin(th)])
    h=hits(pts[::2],0.09); score=h*10+abs(th-0.8)*0.5+abs(x0)+abs(y0-0.36)
    if best is None or score<best[0]: best=(score,h,th,x0,y0,S)
print('butterfly',best); res['butterfly']=best
json.dump({k:[float(x) for x in v] for k,v in res.items()},open('clear.json','w'))
