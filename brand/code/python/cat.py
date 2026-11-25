import numpy as np
exec(open('sim.py').read().split("out={}")[0])
cat={'butterfly1':(0.30689,0.12551,6.2356),'butterfly2':(0.39295,0.09758,7.0039),'butterfly3':(0.40592,0.23016,13.8658),
     'moth1':(0.46444,0.39606,14.8939),'moth2':(0.43917,0.45297,28.6703),'moth3':(0.38344,0.37736,25.8406),
     'goggles':(0.08330,0.12789,10.4668),'dragonfly':(0.08058,0.58884,21.2710),'yinyang1a':(0.51394,0.30474,17.3284),
     'yinyang1b':(0.28270,0.32721,10.9626),'bumblebee':(0.18428,0.58719,63.5345)}
out={}
for k,(p1,p2,T) in cat.items():
    v=np.array([p1,p2]); n=int(min(3000,max(900,T*90)))
    print(k,end=' '); out[k]=run([[-1,0],[1,0],[0,0]],[v,v,-2*v],[1,1,1],T,n+1)
x1=np.array([-0.97000436,0.24308753]); v3=np.array([-0.93240737,-0.86473146])
print('eight',end=' '); out['eight']=run([x1,-x1,[0,0]],[-v3/2,-v3/2,v3],[1,1,1],6.32591398,901)
np.savez('catalog.npz',**out)
