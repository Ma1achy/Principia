"""Pools ocr.swift outputs of vertical strips of one crop into one OCR table, strip order kept left to right.

Usage: python3 -I pool_strips.py <out.tsv> <strip1.tsv> <strip2.tsv> ...
Each strip's min-x and max-x are shifted by its index (strip k occupies [k, k+1)), so ocr_grid.py's left-to-right sort
keeps the strip order. Mid-y is unchanged (all strips share the crop's full height). Text is unchanged.
"""
import sys

with open(sys.argv[1], "w", encoding="utf-8") as out:
    for k, path in enumerate(sys.argv[2:]):
        with open(path, encoding="utf-8") as f:
            for line in f:
                p = line.rstrip("\n").split("\t")
                if len(p) < 4:
                    continue
                out.write(f"{p[0]}\t{float(p[1]) + k:.4f}\t{float(p[2]) + k:.4f}\t{p[3]}\n")
