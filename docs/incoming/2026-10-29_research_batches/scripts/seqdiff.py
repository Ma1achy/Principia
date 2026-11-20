"""seqdiff.py <reading_a> <reading_b> [--skip-a N] [--skip-b N] [--cols C1,C2]: compare the sequences of numbers in two readings.

For tables whose printed layout (brackets, intervals, matrices) does not map row-for-row onto OCR rows.
Each file is read as text; numbers are extracted in reading order after removing the thin and ordinary
spaces that typeset digit groups ("0.347 116 886" -> "0.347116886") and mapping Unicode minus to '-'.
For a CSV, the header line is skipped and cells are read left to right (only the --cols columns,
if given, so label columns holding digits are not read as data). --skip-a/--skip-b drop that many
leading numbers (page numbers, table numbers in a caption). Prints every position where the two
sequences differ and the counts.
"""
import re, sys

def nums(path, skip):
    t = open(path, encoding="utf-8").read()
    if path.endswith(".csv"):
        import csv, io
        rows = list(csv.reader(io.StringIO(t)))
        cols = sys.argv[sys.argv.index("--cols") + 1].split(",") if "--cols" in sys.argv else rows[0]
        idx = [rows[0].index(c) for c in cols]
        t = "\n".join("\t".join(r[i] for i in idx) for r in rows[1:] if r)
    t = t.replace("−", "-").replace(" ", "").replace(" ", "")
    t = re.sub(r"(?<=\d) (?=\d)", "", t)
    out = re.findall(r"-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?", t)
    return out[skip:]

a = nums(sys.argv[1], int(sys.argv[sys.argv.index("--skip-a") + 1]) if "--skip-a" in sys.argv else 0)
b = nums(sys.argv[2], int(sys.argv[sys.argv.index("--skip-b") + 1]) if "--skip-b" in sys.argv else 0)
n = 0
for i in range(max(len(a), len(b))):
    x = a[i] if i < len(a) else None
    y = b[i] if i < len(b) else None
    if x != y:
        print("position %d: A=%r B=%r" % (i, x, y)); n += 1
print("numbers A=%d B=%d; disagreements %d" % (len(a), len(b), n))
