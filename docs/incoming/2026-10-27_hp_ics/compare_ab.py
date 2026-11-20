# Compares route A and route B digit for digit. Usage: python3 -I compare_ab.py a.json b.json
import sys, json, collections
A = json.load(open(sys.argv[1])); B = json.load(open(sys.argv[2]))
def sig(c): return len(c.split('e')[0].lstrip('-')[2:])
for k in A:
    a, b = A[k], B[k]
    same = sum(1 for x, y in zip(a, b) if x == y)
    nval = sum(len(r) for r in a)
    vdiff = sum(1 for x, y in zip(a, b) for p, q in zip(x, y) if p != q)
    dig = collections.Counter(sig(v) for r in a for v in r)
    print(f'{k}: rows A={len(a)} B={len(b)}, rows identical {same}, values {nval}, values differing {vdiff}, sig digits {sorted(dig.items())}')
r = B['hristov2026_971_regions']; a = A['hristov2026_971']
print('971 regions as multiset equal to A:', collections.Counter(map(tuple, r)) == collections.Counter(map(tuple, a)))
