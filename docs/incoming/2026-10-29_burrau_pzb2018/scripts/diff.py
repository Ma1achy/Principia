import csv, sys
a = list(csv.reader(open(sys.argv[1]))); b = list(csv.reader(open(sys.argv[2])))
assert a[0] == b[0] and len(a) == len(b)
n = 0; cells = 0
print("run,t,mass,column,pass_a,pass_b")
for ra, rb in zip(a[1:], b[1:]):
    for j, h in enumerate(a[0]):
        cells += 1
        if ra[j] != rb[j]:
            n += 1; print(",".join(ra[:3] + [h, ra[j], rb[j]]))
print(f"# {cells} cells compared (48 numeric), {n} disagreements", file=sys.stderr)
print(f"# {cells} cells compared, {n} disagreements")
