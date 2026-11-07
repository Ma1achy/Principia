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
