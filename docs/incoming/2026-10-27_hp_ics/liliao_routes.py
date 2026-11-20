# Li & Liao 2017's 695 equal-mass orbits (v1, v2, T at 10 decimals), transcribed twice:
# route A from the GitHub sjtu-liao/three-body three-body-pictures.md (<th> cells),
# route B from numericaltank.sjtu.edu.cn three-body-movies.htm (tags stripped, token scan).
# Usage: python3 -I liliao_routes.py <pictures.md> <movies.htm> <out.json>
import sys, re, json
md = open(sys.argv[1], encoding='utf-8').read()
A = {}
for m in re.finditer(r'<th>([IV]+\.[A-D])<sup>.*?</sup><sub><sub[^>]*>(\d+)</sub></th>\s*<th>\s*([-\d.]+)</th>\s*<th>\s*([-\d.]+)</th>\s*<th>\s*([-\d.]+)</th>', md, re.S):
    A[f'{m[1]}-{m[2]}'] = [m[3], m[4], m[5]]
t = open(sys.argv[2], encoding='utf-8').read()
t = re.sub(r'(?s)<style.*?</style>|<!--.*?-->', '', t); t = re.sub(r'<[^>]+>', ' ', t); t = re.sub(r'(&nbsp;|\s)+', ' ', t)
B = {}
for m in re.finditer(r'\b([IV]+\.[A-D]) i\.c\. (\d+) (-?\d+\.\d+) (-?\d+\.\d+) (-?\d+\.\d+) ', t):
    B[f'{m[1]}-{m[2]}'] = [m[3], m[4], m[5]]
print('A', len(A), 'B', len(B), 'keys equal', A.keys() == B.keys(), 'values differing', sum(A[k] != B.get(k) for k in A))
json.dump({'A': A, 'B': B}, open(sys.argv[3], 'w'))
