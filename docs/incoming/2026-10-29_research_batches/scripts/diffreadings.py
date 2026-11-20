"""diffreadings.py <a.csv> <b.csv> <n_key_cols> [--label-a A --label-b B] [--key COL,COL]: cell-by-cell diff of two readings.

Both files are CSV with a header row and the same columns. Rows are matched on the first n_key_cols
columns, or on the named --key columns (used where an OCR reading dropped a short cell such as a
lone "1", so that the row still pairs with its partner and the gap shows as a disagreement). Cells are compared after normalisation only of typography (Unicode minus and dashes to '-',
'×10^' forms left alone, surrounding spaces stripped, OCR confusables in numeric cells: '€', 'c' and
'C' in an exponent position, Latin 'O'/'o' inside an otherwise numeric cell, Cyrillic look-alikes). Every disagreement is printed; the exit status is 0.
"""
import csv, sys, re

def norm(s):
    s = s.strip().replace("−", "-").replace("–", "-").replace("—", "-")
    s = s.replace("О", "0").replace("о", "0").replace("З", "3").replace(" ", "").replace(" ", "")
    if re.fullmatch(r"[-+]?[0-9.Oo]+", s) and re.search(r"[0-9]", s):
        s = s.replace("O", "0").replace("o", "0")
    if re.fullmatch(r"[-+]?[0-9.]+[€cC][-+]?[0-9]+", s):
        s = re.sub(r"[€cC]", "e", s)
    return s

def main():
    a, b, k = sys.argv[1], sys.argv[2], int(sys.argv[3])
    la = sys.argv[sys.argv.index("--label-a") + 1] if "--label-a" in sys.argv else "A"
    lb = sys.argv[sys.argv.index("--label-b") + 1] if "--label-b" in sys.argv else "B"
    ra, rb = list(csv.reader(open(a))), list(csv.reader(open(b)))
    ha, hb = ra[0], rb[0]
    assert ha == hb, (ha, hb)
    idx = [ha.index(c) for c in sys.argv[sys.argv.index("--key") + 1].split(",")] if "--key" in sys.argv else list(range(k))
    da = {tuple(norm(r[i]) for i in idx): r for r in ra[1:] if r}
    db = {tuple(norm(r[i]) for i in idx): r for r in rb[1:] if r}
    n = cells = 0
    for key in list(da) + [x for x in db if x not in da]:
        if key not in db or key not in da:
            print("row %r only in %s" % (key, la if key in da else lb)); n += 1; continue
        for col, x, y in zip(ha, da[key], db[key]):
            cells += 1
            if norm(x) != norm(y):
                print("row %r col %s: %s=%r %s=%r" % (key, col, la, x, lb, y)); n += 1
    print("rows %s=%d %s=%d; cells compared %d; disagreements %d" % (la, len(da), lb, len(db), cells, n))

main()
