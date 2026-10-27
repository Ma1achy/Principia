# Route B: byte-level regex and hand-built canonical strings (no Decimal), from the
# alternate files where the database offers them (per-solution files, numbered files,
# region files); for the two 2024 files and Masson's file, which have no alternate,
# a re-read of the same bytes by this different code path.
# Usage: python3 -I route_b.py <downloads_dir> <out.json>
import sys, json, os, re
D = sys.argv[1]
NUM = re.compile(rb'(-?)(\d+)\.(\d*)(?:[eE]([+-]?\d+))?')
def canon(m):
    sign, ip, fp, ex = m.groups()
    ex = int(ex) if ex else 0
    allds = (ip + fp).decode()
    ds = allds.lstrip('0')
    lead = len(allds) - len(ds)
    E = len(ip) + ex - lead
    return ('-' if sign else '') + '0.' + ds + 'e' + str(E)
def nums(b):
    # drop leading "N)" row labels
    b = re.sub(rb'(?m)^\s*\d+\)', b'', b)
    return [canon(m) for m in NUM.finditer(b)]
def table(path, ncols):
    out = []
    for line in open(path, 'rb').read().splitlines():
        n = nums(line)
        if not n: continue
        assert len(n) == ncols, (path, line[:40], len(n))
        out.append(n)
    return out
def solfiles(d, n):
    out = []
    for i in range(1, n+1):
        v = nums(open(os.path.join(d, f'sol{i}.txt'), 'rb').read())
        assert len(v) == 4, (d, i, len(v))
        out.append(v)
    return out
res = {
 'hristov2022_123': solfiles(os.path.join(D,'sofia_3body123/sol'), 123),
 'hristov2023_462': solfiles(os.path.join(D,'sofia_3bodychor462/sol'), 462),
 'hristov2024_nontriv164': table(os.path.join(D,'sofia_3bodychor164/nontriv_180.txt'), 4),
 'hristov2024_triv259': table(os.path.join(D,'sofia_3bodychor164/triv_180.txt'), 4),
 'hristov2026_971': table(os.path.join(D,'sofia_3bodystab971/ics_971_100_num.txt'), 4),
 'hristov2026_971_regions': sum((table(os.path.join(D,'sofia_3bodystab971',f), 4) for f in
     ['Shubart_S_num.txt','Intermediate_Moth_num.txt','Eight_num.txt','Broucke_num.txt']), []),
 'masson_simo_345': table(os.path.join(D,'analyticphysics/simo-initial-conditions.txt'), 5),
}
json.dump(res, open(sys.argv[2],'w'), indent=0)
for k,v in res.items(): print(k, len(v))
