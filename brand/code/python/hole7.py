exec(open('hole6.py').read().split("# seeds:")[0])
import time
def cost2(P,HH):
    W,face=build(P)
    if face<0.5 or not (1.15<=P[4]<=1.75): return 1e9,0,face
    h=hitsH(W.reshape(-1,3),0.095,HH)
    return h - 0.01*face, holecount(W), face
t0=time.time(); out=[]
for HH in [0.2,0.15,0.12]:
    best=None
    for trial in range(40):
        P=np.array([rng.uniform(0,2*np.pi),rng.uniform(0,np.pi),rng.uniform(-0.15,0.15),rng.uniform(0.25,0.45),rng.uniform(1.2,1.7),rng.uniform(-1.2,1.2)])
        c0,hc,f=cost2(P,HH)
        step=np.array([0.08,0.08,0.03,0.03,0.05,0.1])
        for it in range(120):
            Q=P+rng.normal(0,1,6)*step; c1,hc1,f1=cost2(Q,HH)
            if c1<=c0 and hc1>0: P,c0,hc,f=Q,c1,hc1,f1
        if hc>0 and (best is None or c0<best[0]): best=(c0,hc,f,list(map(float,P)))
        if time.time()-t0>170*(1+[0.2,0.15,0.12].index(HH)): break
    print('thickness',2*HH,'best contact',None if best is None else round(best[0],3),best)
    out.append((HH,best))
json.dump(out,open('hole7.json','w'))
