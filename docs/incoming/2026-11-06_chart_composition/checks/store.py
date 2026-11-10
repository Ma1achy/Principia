# The time-axis store (physics review 5809967149; design section 18.3). Part 1: rows off a fixed stride, the reason the rule is a cache keyed by boundary.
# Part 2: a bookkeeping model of that rule over random events. Entries are pure by construction, so it tests the rule's counting, not kernel bits;
# the kernel test is time_column_matches_playhead.
import math,random,hashlib
# Model: boundaries are integers. state(ic,k) stands for f(IC,k,key). rows read floor(t/D) as section 18.2.
def state(ic,k):return hashlib.md5(f'{ic},{k}'.encode()).hexdigest()[:8]
def rows(w,H,D):
    lo,hi,warp=w
    out=[]
    for r in range(H):
        x=(r+.5)/H
        t=lo+(hi-lo)*x if warp=='lin' else math.exp(math.log(lo)+(math.log(hi)-math.log(lo))*x)
        out.append(int(t/D))
    return out
# 1 the X6/X8 stride rule against what rows read
for T,D,H in((50,.0125,1080),(50,.001,1080),(13,.001,1080),(50,.1,1080)):
    BT=int(T/D);b=max(1,math.ceil(BT/H))
    for name,w in(('whole lin',(D,T,'lin')),('lower half',(D,T/2,'lin')),('whole log',(D,T,'log'))):
        R=set(rows(w,H,D));m=sum(1 for k in R if k%b)
        print(f'T={T} D={D} B_T={BT} b={b} {name}: distinct rows {len(R)}, off the stride {m} ({m/len(R):.2f})')
# 2 the keyed rule under random events
random.seed(3)
class C:
    def __init__(s,ic):s.ic=ic;s.f=0;s.st={};s.use={}
def run(H,K,D,events=4000):
    T=50.;w=(D,T,'lin');cols={i:C(i) for i in range(6)};cl=0;bad=over=0;reint=hit=0;nextic=6
    def need():
        return sorted(set(rows(w,H,D)))
    def serve():
        nonlocal bad,over,reint,hit,cl
        R=need();Rs=set(R)
        for c in cols.values():
            cl+=1
            hi=R[-1]
            miss=[k for k in R if k<=c.f and k not in c.st]
            if miss: reint+=1
            else: hit+=1
            # march from 0 for misses, or on from the front
            top=hi if miss else None
            for k in R:
                if k not in c.st:
                    c.st[k]=state(c.ic,k)
                c.use[k]=cl
            c.f=max(c.f,hi)
            if len(c.st)>K:
                extra=sorted((k for k in c.st if k not in Rs),key=lambda k:c.use.get(k,0))
                for k in extra[:len(c.st)-K]:del c.st[k]
            if len(c.st)>K: over+=1
            for k in R:
                if c.st[k]!=state(c.ic,k): bad+=1
    serve()
    for _ in range(events):
        e=random.choice(['pan','zoom','finer','raiseT','lowerT','icpan','past'])
        lo,hi,wa=w
        if e=='pan':
            d=(hi-lo)*random.uniform(-.5,.5);lo=max(D,lo+d);hi=min(T,hi+d)
        elif e=='zoom':
            c=(lo+hi)/2;s=random.choice([.5,2]);lo=max(D,c-(c-lo)*s);hi=min(T,c+(hi-c)*s)
        elif e=='finer':
            lo,hi=lo+(hi-lo)*.25,hi-(hi-lo)*.25
        elif e=='raiseT':T=min(200,T*1.5)
        elif e=='lowerT':
            T=max(5,T/1.5);hi=min(hi,T);lo=min(lo,hi/2)
            for c in cols.values():
                c.st={};c.use={};c.f=0
        elif e=='icpan':
            for i in list(cols)[:2]:del cols[i]
            for _x in range(2):cols[nextic]=C(nextic);nextic+=1
        elif e=='past':hi=min(T,hi*1.3)
        if hi-lo<D*2: lo,hi=max(D,lo),max(lo,hi)+D*2
        w=(lo,hi,wa);serve()
    return bad,over,reint,hit
for D in(.4,.1,.0125,.001):
    print('D',D,'bad reads, over K, columns with a miss, columns all hits',run(270,270,D,1500))
