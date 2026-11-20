"""Copies ChatGPT's numerical return times for the Kapela-Simó nominal points into a labelled CSV, verbatim.

Usage: python3 -I kapela_return_times.py <numerical_returns.json (ChatGPT)> <out.csv>
These numbers are UNCERTIFIED, COMPUTED BY CHATGPT: a non-interval integration of nominal points, not the authors'
results and not re-derived here. The archive prints no physical return-time or rotation interval. Each row carries the
label in its own column so no copy of the file loses it.
"""
import csv
import json
import sys

d = json.load(open(sys.argv[1], encoding="utf-8"))
with open(sys.argv[2], "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["label", "id", "return_time", "rotation_angle_rad", "closure_linf_after_rotation",
                "authors_certified_return_time_interval"])
    for b in d["benchmarks"]:
        w.writerow(["uncertified, computed by ChatGPT", b["id"], b["return_time_estimate"],
                    b["rotation_principal_radians_estimate"], b.get("closure_linf_after_rotation", ""),
                    "none published" if b.get("authors_certified_physical_return_time_interval") is None
                    else b["authors_certified_physical_return_time_interval"]])
print(f"{sys.argv[2]}: {len(d['benchmarks'])} rows, every one labelled 'uncertified, computed by ChatGPT'")
