exec(open('dragon2.py').read().split("P0=np.array(P)")[0])
base=np.array(json.load(open('dragon_P_base.json')))
results=[]
for g in [1.2]:
    P0=base.copy(); P0[2]/=g; P0[3]/=g; P0[4]/=g
    best=(hits(build(P0,Ys).reshape(-1,3),0.1),P0)
    for it in range(500):
        Q=best[1]+rng.normal(0,1,6)*np.array([0.012,0.012,0.008,0.008,0.003,0.015])
        Q[4]=min(Q[4],base[4]/g)          # never let the orbit grow back
        W=build(Q,Ys); h=hits(W.reshape(-1,3),0.1)
        if h<=best[0] and holecount(W)>0: best=(h,Q)
        if best[0]==0: break
    Wf=build(best[1],Y)
    hf=hits(Wf[::2].reshape(-1,3),0.095); hc=holecount(Wf[::10])
    print('P scale',g,'coarse',best[0],'full',hf,'hole passes',hc)
    results.append((g,int(hf),hc,list(map(float,best[1]))))
json.dump(results,open('dragon_scale.json','w'))
