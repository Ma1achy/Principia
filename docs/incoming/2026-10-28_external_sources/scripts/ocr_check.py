"""Machine cross-check of an eye reading: every numeric cell of a CSV must appear among the OCR tokens of the page.

Usage: python3 -I ocr_check.py <ocr_output.tsv> <reading.csv> [col1,col2,...]
The OCR output is ocr.swift's (mid-y, min-x, max-x, text). Text boxes are split on whitespace into tokens. Each listed
column's cell (default: every cell that parses as a number) is looked up as a string in the multiset of tokens; each
match consumes one token. Prints the cells with no matching token (the OCR missed or misread them, or the reading is
wrong: those are then resolved by eye against the page) and a count line.
"""
import collections
import csv
import sys


def isnum(s):
    try:
        float(s.replace("−", "-"))
        return True
    except ValueError:
        return False


def main():
    toks = collections.Counter()
    with open(sys.argv[1], encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split("\t")
            if len(parts) >= 4:
                for t in parts[3].split():
                    toks[t.replace("−", "-")] += 1
    cols = sys.argv[3].split(",") if len(sys.argv) > 3 else None
    with open(sys.argv[2], newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))
    n = miss = 0
    for i, r in enumerate(rows):
        for c, v in r.items():
            if cols and c not in cols:
                continue
            v = (v or "").strip()
            if not v or (not cols and not isnum(v)):
                continue
            n += 1
            if toks[v] > 0:
                toks[v] -= 1
            else:
                miss += 1
                print(f"NO-OCR-MATCH\trow {i + 1}\t{c}\t{v}")
    print(f"numeric cells checked={n}; matched by an OCR token={n - miss}; unmatched={miss}")


if __name__ == "__main__":
    main()
