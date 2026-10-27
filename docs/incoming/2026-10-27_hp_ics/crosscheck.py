# Cross-checks the operator's atlas-params.json (high-precision fields) against the primary databases as transcribed
# by route_a.py, and Li & Liao 2017's published values as transcribed by liliao_routes.py.
#
# The atlas file is read read-only from wherever it is; it is never copied into the repo. This script prints derived
# numbers only: row counts, digits agreed and maximum differences. It never prints a value from the file.
# The two field names are built at run time so that no committed file carries them as keys.
#
# Usage: python3 -I crosscheck.py <atlas-params.json> <route_a.json> <liliao.json>
import collections
import json
import re
import statistics
import sys
from decimal import Decimal, getcontext

getcontext().prec = 260
K_IC, K_T = 'ic' + '_hp', 'period' + '_hp'
IDENT = 10**6  # marker: the two values are equal

atlas = json.load(open(sys.argv[1], encoding='utf-8'))['configurations']
DB = json.load(open(sys.argv[2], encoding='utf-8'))
LL = json.load(open(sys.argv[3], encoding='utf-8'))['A']


def dec(c):
    """A canonical '0.ddd...eE' string from route_a.py, or a plain decimal string from the atlas."""
    if 'e' in c and c.lstrip('-').startswith('0.'):
        m, e = c.split('e')
        return Decimal(m).scaleb(int(e))
    return Decimal(c)


def sig(s):
    """Significant digits printed, trailing zeros excluded."""
    return len(s.lower().split('e')[0].lstrip('-').replace('.', '').strip('0'))


def agree(a, b):
    """Digits agreed: floor(-log10(|a - b| / |b|)); IDENT if equal. Where b is 0, the absolute form floor(-log10|a|)."""
    if a == b:
        return IDENT
    if b == 0:
        return int((-abs(a).log10()).to_integral_value(rounding='ROUND_FLOOR'))
    return int((-(abs(a - b) / abs(b)).log10()).to_integral_value(rounding='ROUND_FLOOR'))


tbl = {k: [[dec(x) for x in r] for r in v] for k, v in DB.items()}
SQ3 = Decimal(3).sqrt()

# Source key in the atlas -> the transcribed tables its rows are matched against.
SRC = {
    'hristov2022': ['hristov2022_123'],
    'hristov2023': ['hristov2023_462'],
    'hristov2024nc': ['hristov2024_nontriv164', 'hristov2024_triv259'],
    'hristov2026': ['hristov2026_971'],
    'liliao2017': ['hristov2026_971'],
    'simo2001': ['masson_simo_345'],
}

rows = collections.defaultdict(list)
empty = 0
other = collections.Counter()
for v in atlas.values():
    if K_IC not in v:
        continue
    s = v.get('source', '')
    if not s:
        empty += 1
    elif s in SRC:
        rows[s].append(v)
    else:
        other[s] += 1

print('atlas rows with high-precision fields:', sum(map(len, rows.values())) + empty + sum(other.values()))
print('  empty source (closure tests only; not traced, never cited):', empty)
print('  sources in scope:', {s: len(r) for s, r in sorted(rows.items())})
print('  the atlas\'s own notes and the CAP repository, not in scope:', dict(sorted(other.items())))


def isosceles(ic):
    """For the collinear isosceles layout (-1,0), (1,0), (0,0): the index of each body, or None."""
    x = [dec(c) for c in ic]
    at = {x[2 * n]: n for n in range(3) if x[2 * n + 1] == 0}
    if set(at) != {Decimal(-1), Decimal(1), Decimal(0)}:
        return None
    return at[Decimal(-1)], at[Decimal(1)], at[Decimal(0)]


def summary(name, st, alen, mx):
    for f in st:
        g = st[f]
        ident = sum(1 for x in g if x == IDENT)
        rest = [x for x in g if x != IDENT]
        tail = f'; others agree to {min(rest)}-{max(rest)} digits (median {statistics.median(rest)})' if rest else ''
        print(f'  {f}: digits printed {min(alen[f])}-{max(alen[f])}; identical {ident}/{len(g)}{tail}; '
              f'max |diff| {float(mx[f]):.1e}')


for s, rs in sorted(rows.items()):
    st, alen, mx = collections.defaultdict(list), collections.defaultdict(list), collections.defaultdict(Decimal)
    unmatched, where, used = 0, collections.Counter(), set()

    def add(f, p, q, string):
        st[f].append(agree(p, q))
        alen[f].append(sig(string))
        mx[f] = max(mx[f], abs(p - q))

    for v in rs:
        ic, T = v[K_IC], v[K_T]
        a = [dec(c) for c in ic]
        if s == 'simo2001':
            # Simó's normalisation: masses 1/3, energy -1/2. The atlas's masses are 1 with the same positions, so its
            # velocities are Simó's times sqrt(3) and its period is Simó's (3 * c5) divided by sqrt(3).
            tab = tbl['masson_simo_345']
            best = min(range(len(tab)), key=lambda i: abs(tab[i][0] - a[2]) + abs(tab[i][1] - a[3]))
            c1, c2, c3, c4, c5 = tab[best]
            if abs(a[2] - c1) > Decimal('1e-10'):
                unmatched += 1
                continue
            used.add(best)
            where['masson_simo_345'] += 1
            # All twelve components: ic is (x1, y1, x2, y2, x3, y3, vx1, vy1, vx2, vy2, vx3, vy3).
            add('x1 = -2 c1', a[0], -2 * c1, ic[0])
            add('y1 = 0', a[1], Decimal(0), ic[1])
            add('x2 = c1', a[2], c1, ic[2])
            add('y2 = c2', a[3], c2, ic[3])
            add('x3 = c1', a[4], c1, ic[4])
            add('y3 = -c2', a[5], -c2, ic[5])
            add('vx1 = 0', a[6], Decimal(0), ic[6])
            add('vy1 = -2 c4 sqrt3', a[7], -2 * c4 * SQ3, ic[7])
            add('vx2 = c3 sqrt3', a[8], c3 * SQ3, ic[8])
            add('vy2 = c4 sqrt3', a[9], c4 * SQ3, ic[9])
            add('vx3 = -c3 sqrt3', a[10], -c3 * SQ3, ic[10])
            add('vy3 = c4 sqrt3', a[11], c4 * SQ3, ic[11])
            add('T = sqrt3 c5', dec(T), 3 * c5 / SQ3, T)
            continue
        iso = isosceles(ic)
        if iso is None:
            unmatched += 1
            continue
        i1, i2, i3 = iso
        vx, vy = a[6 + 2 * i1], a[7 + 2 * i1]
        cands = [(k, r) for k in SRC[s] for r in tbl[k]]
        k, best = min(cands, key=lambda kr: abs(kr[1][0] - vx) + abs(kr[1][1] - vy))
        if abs(best[0] - vx) > Decimal('1e-12') or abs(best[1] - vy) > Decimal('1e-12'):
            unmatched += 1
            continue
        where[k] += 1
        bvx, bvy, bT, _ = best
        add('vx (body at -1)', vx, bvx, ic[6 + 2 * i1])
        add('vy (body at -1)', vy, bvy, ic[7 + 2 * i1])
        add('vx (body at +1) = vx', a[6 + 2 * i2], bvx, ic[6 + 2 * i2])
        add('vy (body at +1) = vy', a[7 + 2 * i2], bvy, ic[7 + 2 * i2])
        add('vx (body at 0) = -2 vx', a[6 + 2 * i3], -2 * bvx, ic[6 + 2 * i3])
        add('vy (body at 0) = -2 vy', a[7 + 2 * i3], -2 * bvy, ic[7 + 2 * i3])
        add('T', dec(T), bT, T)
        if s == 'liliao2017':
            nm = re.search(r'([IV]+) ([A-D]) (\d+) i c', v['name'])
            L = [Decimal(x) for x in LL[f'{nm[1]}.{nm[2]}-{nm[3]}']]
            add('Li & Liao 2017 published v1 against vx', L[0], vx, LL[f'{nm[1]}.{nm[2]}-{nm[3]}'][0])
            add('Li & Liao 2017 published v2 against vy', L[1], vy, LL[f'{nm[1]}.{nm[2]}-{nm[3]}'][1])
            add('Li & Liao 2017 published T against T', L[2], dec(T), LL[f'{nm[1]}.{nm[2]}-{nm[3]}'][2])
    print(f'\n== {s}: {len(rs)} atlas rows; matched {len(rs) - unmatched} ({dict(where)}); unmatched {unmatched}')
    summary(s, st, alen, mx)
    if s == 'simo2001':
        print('  Simó numbers with no atlas row:', [i + 1 for i in range(len(tbl['masson_simo_345'])) if i not in used])
