# Route A: whitespace split + decimal.Decimal, from each database's combined file.
# Usage: python3 -I route_a.py <downloads_dir> <out.json>
import sys, json, os
from decimal import Decimal
D = sys.argv[1]
def canon(tok):
    d = Decimal(tok)
    s, digs, e = d.as_tuple()
    ds = ''.join(map(str, digs)).lstrip('0')
    lead = len(''.join(map(str, digs))) - len(ds)
    # exponent of the first significant digit: value = 0.d1d2... * 10^E
    E = len(digs) + e - lead
    return ('-' if s else '') + '0.' + ds + 'e' + str(E)
def rows(path, ncols, skip_label=False):
    out = []
    with open(path, 'r', encoding='ascii') as f:
        for line in f:
            t = line.split()
            if not t: continue
            if skip_label: t = t[1:]
            assert len(t) == ncols, (path, len(t))
            out.append([canon(x) for x in t])
    return out
res = {
 'hristov2022_123': rows(os.path.join(D,'sofia_3body123/all_150_digits_123.txt'), 4),
 'hristov2023_462': rows(os.path.join(D,'sofia_3bodychor462/digits180.txt'), 4),
 'hristov2024_nontriv164': rows(os.path.join(D,'sofia_3bodychor164/nontriv_180.txt'), 4),
 'hristov2024_triv259': rows(os.path.join(D,'sofia_3bodychor164/triv_180.txt'), 4),
 'hristov2026_971': rows(os.path.join(D,'sofia_3bodystab971/ics_971_100.txt'), 4),
 'masson_simo_345': rows(os.path.join(D,'analyticphysics/simo-initial-conditions.txt'), 5),
}
json.dump(res, open(sys.argv[2],'w'), indent=0)
for k,v in res.items(): print(k, len(v))
