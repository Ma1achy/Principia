# Consistency of the Table 2 transcription: E, L, P and the centre of mass, G = 1, in Decimal at 60 digits.
import csv, sys
from decimal import Decimal as D, getcontext
getcontext().prec = 60
def inv(st):
    m = [D(b[0]) for b in st]
    T = sum(mi*(vx*vx+vy*vy) for mi,(x,y,vx,vy) in zip(m,[b[1:] for b in st]))/2
    U = D(0)
    for i in range(3):
        for j in range(i+1,3):
            dx = st[i][1]-st[j][1]; dy = st[i][2]-st[j][2]
            U -= m[i]*m[j]/(dx*dx+dy*dy).sqrt()
    M = sum(m)
    L = sum(b[0]*(b[1]*b[4]-b[2]*b[3]) for b in st)
    Px = sum(b[0]*b[3] for b in st); Py = sum(b[0]*b[4] for b in st)
    Cx = sum(b[0]*b[1] for b in st)/M; Cy = sum(b[0]*b[2] for b in st)/M
    return T+U, L, Px, Py, Cx, Cy
def g(v): return f"{float(v):.6e}"
rows = list(csv.DictReader(open(sys.argv[1])))
Z = D(0)
ics = {
 "t=0 Table 1 unperturbed": [(D(3),D(1),D(3),Z,Z),(D(4),D(-2),D(-1),Z,Z),(D(5),D(1),D(-1),Z,Z)],
 "t=0 Table 1 perturbed":   [(D(3),D("1.0000000001"),D(3),Z,Z),(D(4),D(-2),D(-1),Z,Z),(D(5),D(1),D(-1),Z,Z)],
 "t=0 Appendix B text":     [(D(3),D(1),D(3),Z,Z),(D(4),D(-2),D(1),Z,Z),(D(5),D(1),D(-1),Z,Z)],
}
res = {}
print("state\tE\tL\tPx\tPy\tCx\tCy")
for k, st in ics.items():
    r = inv(st); res[k] = r
    print(k, *[str(+r[0])[:45]] , *map(g, r[1:]), sep="\t")
for run in ("unperturbed","perturbed"):
    for t in ("100","200"):
        st = [(D(r["mass"]),D(r["x"]),D(r["y"]),D(r["vx"]),D(r["vy"])) for r in rows if r["run"]==run and r["t"]==t]
        r = inv(st); res[(run,t)] = r
        print(f"t={t} {run}", str(+r[0])[:45], *map(g, r[1:]), sep="\t")
print()
print("relative energy change against the run's own Table 1 start, |E(t)-E0|/|E0|:")
for run, k0 in (("unperturbed","t=0 Table 1 unperturbed"),("perturbed","t=0 Table 1 perturbed")):
    E0 = res[k0][0]
    for t in ("100","200"):
        print(f"  {run} t={t}: {g(abs(res[(run,t)][0]-E0)/abs(E0))}")
    print(f"  {run} t=100 against Appendix B's start: {g(abs(res[(run,'100')][0]-res['t=0 Appendix B text'][0])/abs(E0))}")
print("perturbed minus unperturbed E0:", g(res["t=0 Table 1 perturbed"][0]-res["t=0 Table 1 unperturbed"][0]))
# distance of the t=200 positions from Table 1's start
for run, k0 in (("unperturbed","t=0 Table 1 unperturbed"),("perturbed","t=0 Table 1 perturbed")):
    st0 = ics[k0]
    st = [(D(r["mass"]),D(r["x"]),D(r["y"]),D(r["vx"]),D(r["vy"])) for r in rows if r["run"]==run and r["t"]=="200"]
    mx = max(max(abs(a[i]-b[i]) for i in (1,2)) for a,b in zip(st,st0))
    mxb = max(max(abs(a[i]-b[i]) for i in (1,2)) for a,b in zip(st,ics["t=0 Appendix B text"]))
    mv = max(max(abs(a[i]) for i in (3,4)) for a in st)
    print(f"{run} t=200: max |position - Table 1 start| = {g(mx)}; against Appendix B text = {g(mxb)}; max |v| = {g(mv)}")
