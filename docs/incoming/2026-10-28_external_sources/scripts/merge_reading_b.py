"""Builds reading B from its OCR grid plus the rows read by eye from the same page images.

Usage: python3 -I merge_reading_b.py <ocr_grid.csv> <eye_rows.csv> <out.csv>
The eye file has the grid's columns plus 'why'. An eye row replaces the OCR row with the same key (first column) or
fills a row the OCR missed. OCR rows whose key is not a whole number (header lines the OCR caught) are dropped and
listed. Output rows are sorted by key, numerically.
"""
import csv
import sys

with open(sys.argv[1], newline="", encoding="utf-8") as f:
    r = csv.DictReader(f)
    cols = r.fieldnames
    ocr = {}
    for row in r:
        k = row[cols[0]]
        if not k.isdigit():
            print(f"dropped non-row OCR line: {list(row.values())}")
            continue
        ocr[k] = row
with open(sys.argv[2], newline="", encoding="utf-8") as f:
    eye = {row[cols[0]]: {c: row[c] for c in cols} for row in csv.DictReader(f)}
replaced = [k for k in eye if k in ocr]
filled = [k for k in eye if k not in ocr]
ocr.update(eye)
with open(sys.argv[3], "w", newline="", encoding="utf-8") as f:
    w = csv.DictWriter(f, fieldnames=cols, lineterminator="\n")
    w.writeheader()
    for k in sorted(ocr, key=int):
        w.writerow(ocr[k])
print(f"{sys.argv[3]}: {len(ocr)} rows; {len(ocr) - len(eye)} from OCR alone, {len(filled)} filled by eye, "
      f"{len(replaced)} OCR rows replaced by eye ({', '.join(replaced)})")
