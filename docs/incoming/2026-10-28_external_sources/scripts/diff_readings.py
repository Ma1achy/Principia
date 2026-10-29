"""Cell-by-cell string diff of two transcriptions (reading A, reading B) of one table.

Usage: python3 -I diff_readings.py <reading_a.csv> <reading_b.csv> [key_column] [--numeric]
Both CSVs must have the same header. Rows are matched on the key column (default: the first column). A composite key
names several columns joined by '+' (for example table+m1+m2+m3+Fi); the row's key is then those cells joined by '+'.
Prints every disagreement (key, column, A, B), every row present in only one reading, and a count line.
Cells are compared as strings after stripping surrounding whitespace; nothing is rounded or re-formatted. With
--numeric, two cells that both parse as decimals are compared as numbers (so 4.24e11 = 424000000000.0); cells equal
as numbers but not as strings are counted separately.
Exit 0 if the readings agree in every cell, 1 otherwise.
"""
import csv
import sys
from decimal import Decimal, InvalidOperation


def numeq(x, y):
    try:
        return Decimal(x) == Decimal(y)
    except InvalidOperation:
        return False


def load(path, key):
    cols = key.split("+")
    with open(path, newline="", encoding="utf-8") as f:
        r = csv.DictReader(f)
        rows = {}
        order = []
        for row in r:
            k = "+".join(row[c].strip() for c in cols)
            if k in rows:
                raise SystemExit(f"{path}: duplicate key {k!r}")
            rows[k] = {c: (v or "").strip() for c, v in row.items()}
            order.append(k)
        return r.fieldnames, rows, order


def main():
    numeric = "--numeric" in sys.argv
    sys.argv = [x for x in sys.argv if x != "--numeric"]
    a_path, b_path = sys.argv[1], sys.argv[2]
    with open(a_path, newline="", encoding="utf-8") as f:
        header = next(csv.reader(f))
    key = sys.argv[3] if len(sys.argv) > 3 else header[0]
    ha, a, order = load(a_path, key)
    hb, b, _ = load(b_path, key)
    if ha != hb:
        raise SystemExit(f"headers differ:\n  A {ha}\n  B {hb}")
    cells = 0
    bad = 0
    loose = 0
    for k in order:
        if k not in b:
            print(f"ONLY-IN-A\t{k}")
            bad += 1
            continue
        for c in ha:
            cells += 1
            if a[k][c] != b[k][c] and numeric and numeq(a[k][c], b[k][c]):
                loose += 1
            elif a[k][c] != b[k][c]:
                print(f"DIFF\t{k}\t{c}\tA={a[k][c]}\tB={b[k][c]}")
                bad += 1
    for k in b:
        if k not in a:
            print(f"ONLY-IN-B\t{k}")
            bad += 1
    extra = f"; equal only as numbers={loose}" if numeric else ""
    print(f"rows A={len(a)} B={len(b)}; cells compared={cells}; disagreements={bad}{extra}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
