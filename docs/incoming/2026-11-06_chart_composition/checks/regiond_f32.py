# V3: f32 error of region D's two decode routes against the f64 state, over D, equal masses.
import numpy as np, math
f=np.float32
rng=np.random.default_rng(3); n=400000
xi=rng.uniform(0,.5,n); eta=rng.uniform(0,.87,n); k=((xi+.5)**2+eta**2<=1)&(eta>1e-4); xi=xi[k];eta=eta[k]
m=1/3; mr=m*m/(2*m); ml=m*2*m
def f64(x,y):
    l=np.sqrt(mr+ml*(x*x+y*y)); return np.sqrt(mr)/l, np.sqrt(ml)*x/l, np.sqrt(ml)*y/l
def explicit(x,y):
    x=x.astype(f);y=y.astype(f);a=f(mr);b=f(ml);sa=np.sqrt(a);sb=np.sqrt(b)
    l=np.sqrt(a+b*(x*x+y*y)); return sa/l, sb*x/l, sb*y/l
def viaLink(x,y):
    # atan2 route, then the sigmoid link's inverse and forward, as Mixed does
    x=x.astype(f);y=y.astype(f);a=f(mr);b=f(ml);sa=np.sqrt(a);sb=np.sqrt(b)
    r=np.sqrt(x*x+y*y); al=np.arctan2(sb*r,sa); be=np.arctan2(y,x)
    hp=f(math.pi/2); pi=f(math.pi)
    def sig(s):  # s in (0,1): logit then logistic in f32
        s=s.astype(f); z=np.log(s/(f(1)-s)); return f(1)/(f(1)+np.exp(-z))
    al2=hp*sig(al/hp); be2=pi*sig(be/pi)
    return np.cos(al2), np.sin(al2)*np.cos(be2), np.sin(al2)*np.sin(be2)
t=f64(xi,eta)
for name,g in (('explicit',explicit),('atan2+link',viaLink)):
    o=g(xi,eta); e=[np.abs(o[i].astype(float)-t[i]) for i in range(3)]
    print(name,'max',max(x.max() for x in e),'rms',math.sqrt(sum((x**2).mean() for x in e)/3))
# 7 Nov (physics review 5809670196, sweep): the figures above are equal masses at eta > 1e-4. The same two routes with no cut, and at
# other masses, xi offset by d, masses log-uniform in [1e-3, 1] and eta on three scales.
def worst(g,x,y):
    o=g(x,y); t=f64(x,y); return max(np.abs(o[i].astype(float)-t[i]).max() for i in range(3))
xi=rng.uniform(0,.5,n); eta=rng.uniform(0,.87,n)*rng.choice([1,1e-3,1e-6],n); k=(xi+.5)**2+eta**2<=1
print('equal masses, no cut on eta: explicit %.2e, atan2+link %.2e'%(worst(explicit,xi[k],eta[k]),worst(viaLink,xi[k],eta[k])))
we=wl=0
for _ in range(2000):
    mm=np.exp(rng.uniform(math.log(1e-3),0,3)); M=mm.sum(); M01=mm[0]+mm[1]; d=(mm[1]-mm[0])/(2*M01); mr=mm[0]*mm[1]/M01; ml=M01*mm[2]/M
    xi=rng.uniform(0,.5,2000); eta=rng.uniform(0,.87,2000)*rng.choice([1,1e-3,1e-6],2000); k=(xi+.5)**2+eta**2<=1
    we=max(we,worst(explicit,xi[k]-d,eta[k])); wl=max(wl,worst(viaLink,xi[k]-d,eta[k]))
print('masses log-uniform to 1e-3, no cut on eta: explicit %.2e, atan2+link %.2e'%(we,wl))
