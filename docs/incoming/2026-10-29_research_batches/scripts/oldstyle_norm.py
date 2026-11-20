"""oldstyle_norm.py <ocr_rows.txt> <header,comma,separated> <out.csv>: OCR rows of an old-style-numeral table to CSV.

The 1956 Astronomische Nachrichten face sets the numerals 1 and 0 as old-style figures that OCR reads as
the letters 'I' and 'O' (and 3 as Cyrillic 'З'); this maps only those glyph confusions (I->1, O->0, o->0,
З->3, a non-character in place of the decimal point -> '.'), in rows with as many cells as the header.
No digit is otherwise edited. Other rows are listed on stderr.
"""
import csv, sys
T = str.maketrans({"I": "1", "O": "0", "o": "0", "З": "3", "￾": "."})
hdr = sys.argv[2].split(",")
with open(sys.argv[3], "w", newline="") as f:
    w = csv.writer(f, lineterminator="\n"); w.writerow(hdr)
    for line in open(sys.argv[1], encoding="utf-8"):
        cells = line.rstrip("\n").split("\t")
        if len(cells) == len(hdr):
            w.writerow([c.translate(T).strip() for c in cells])
        else:
            print("dropped:", line.rstrip("\n"), file=sys.stderr)
