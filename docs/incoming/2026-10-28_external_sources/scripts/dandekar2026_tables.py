"""Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026 (arXiv:2608.30414v1): readings A and B of Tables 1-4, B.1-B.3.

Usage: python3 -I dandekar2026_tables.py <chatgpt_data_dir> <ocr_rows_dir> <eye_fixes.csv> <out_dir>
Reading A: ChatGPT's JSON (Dandekar_2026_Table*.json), written out in our column schema, every number as its JSON
literal (ChatGPT stored numbers as JSON numbers, so trailing zeros are lost: 0.300 became 0.3).
Reading B: the OCR rows of the rendered pages (ocr_grid.py --raw on ocr.swift output, page images at 400 dpi, crops
upscaled 2x), parsed by table, then the eye fixes applied: each fix replaces one OCR row's text by the text read by
eye from the same page image, with the reason.
Diff: per cell; numbers compared as decimals (so 0.3 = 0.300, and the lost zero is not a disagreement but is counted),
labels as strings. Writes <table>_reading_a_chatgpt.csv, <table>_reading_b.csv and <table>_diff.txt per table, and
<table>_reconciled.csv, which is reading B (the page's printed strings, every OCR doubt already read by eye); each
remaining disagreement is resolved against the page in the README.
"""
import csv
import json
import os
import re
import sys
from decimal import Decimal, InvalidOperation

MASSES = ["15,15,15", "12.5,15,17.5", "12,15,18", "10,10,20", "10,15,20", "10,20,20", "8,21,21", "5,15,25"]
SCHEMA = {
    "table1": ["mass_set", "E", "L", "L0_1", "L0_2", "L0_3"],
    "table2": ["mass_set", "index", "index_err"],
    "table3": ["mass_set", "overlap_3D_2D"],
    "table4": ["mass_set", "overlap_pro_ret"],
    "tableB1": ["mass_set", "row", "escaper", "pct_np", "P_np", "se_np", "pct_es", "P_es", "se_es", "Kol_3D", "Kol_2D",
                "VK_2D"],
    "tableB2": ["mass_set", "row", "escaper", "N", "alpha", "alpha_se", "LFc_sim", "LFc_th"],
    "tableB3": ["mass_set", "row", "escaper", "pro_ret_ratio", "P_pro", "P_ret", "P_2d"],
}


def lit(v):
    return v[1] if isinstance(v, tuple) else str(v)


def mset(m):
    return ",".join(lit(x).rstrip("0").rstrip(".") if "." in lit(x) else lit(x) for x in m)


def reading_a(src):
    def load(name):
        with open(os.path.join(src, f"Dandekar_2026_{name}.json"), encoding="utf-8") as f:
            return json.load(f, parse_float=lambda s: ("F", s))["rows"]
    out = {}
    out["table1"] = [[mset(r["masses"]), lit(r["E"]), lit(r["L"])] + [lit(x) for x in r["L0_by_escaper"]]
                     for r in load("Table1")]
    out["table2"] = [[mset(r["masses"]), lit(r["index"]), lit(r["printed_error"])] for r in load("Table2")]
    out["table3"] = [[mset(r["masses"]), lit(r["overlap"])] for r in load("Table3")]
    out["table4"] = [[mset(r["masses"]), lit(r["overlap"])] for r in load("Table4")]
    out["tableB1"] = [[mset(r["masses"]), str(r["escaper_row"]), lit(r["escaper_mass"]), lit(r["nonprompt_percent"]),
                       lit(r["P_nonprompt"]), lit(r["se_nonprompt"]), lit(r["ergodic_percent"]), lit(r["P_ergodic"]),
                       lit(r["se_ergodic"]), lit(r["Kol_3D_theory"]), lit(r["Kol_2D_theory"]), lit(r["VK_2D_theory"])]
                      for r in load("TableB1")]
    out["tableB2"] = [[mset(r["masses"]), str(r["escaper_row"]), lit(r["escaper_mass"]), lit(r["N"]), lit(r["alpha"]),
                       lit(r["alpha_se"]), lit(r["LF_c_sim"]), lit(r["LF_c_theory"])] for r in load("TableB2")]
    out["tableB3"] = [[mset(r["masses"]), str(r["escaper_row"]), lit(r["escaper_mass"]),
                       lit(r["prograde_to_retrograde_ratio"]), lit(r["P_prograde"]), lit(r["P_retrograde"]),
                       lit(r["P_overall"])] for r in load("TableB3")]
    return out


NUM = re.compile(r"^-?\d+(\.\d+)?%?$")


def toks(line):
    t = line.replace("±", " ").replace("÷", " ").replace("‡", " ").replace("+", " ").replace("=", " ")
    t = re.sub(r"-\s+(\d)", r"-\1", t)
    return t.split()


def reading_b(rows_dir, fixes):
    def lines(name):
        with open(os.path.join(rows_dir, name + ".rows"), encoding="utf-8") as f:
            ls = [l.rstrip("\n") for l in f]
        return [fixes.get((name, i + 1), l) for i, l in enumerate(ls)]
    out = {}
    simple = {"table1": ("T1", 6), "table2": ("T2", 3), "table3": ("T3", 2), "table4": ("T4", 2)}
    for key, (name, n) in simple.items():
        rows = []
        for l in lines(name):
            t = toks(l)
            if len(t) == n and t[0] in MASSES:
                rows.append(t)
        out[key] = rows
    for key, name, width in (("tableB1", "TB1", 8), ("tableB2", "TB2", 6), ("tableB3", "TB3", 5)):
        rows, group, k = [], [], 0
        for l in lines(name):
            t = toks(l)
            ms = [x for x in t if x in MASSES]
            rest = [x for x in t if x not in MASSES]
            if not rest or not re.match(r"^\d+(\.\d+)?$", rest[0]):
                continue
            if key == "tableB2" and len(rest) > 1 and rest[1].startswith("(") and rest[1].endswith(")"):
                rest = [rest[0] + " " + rest[1]] + rest[2:]
            group.append((ms[0] if ms else None, rest))
            if len(group) == 3:
                m = [g[0] for g in group if g[0]]
                if len(m) != 1:
                    raise SystemExit(f"{name}: a group of three rows without exactly one mass set: {group}")
                pcts = [x for g in group for x in g[1] if x.endswith("%")]
                extra = [x for g in group for x in g[1][1:] if x.endswith("%") is False]
                for i, (_, r) in enumerate(group, start=1):
                    vals = [x for x in r if not x.endswith("%")]
                    if key == "tableB1":
                        if len(pcts) != 2 or len(vals) != 8:
                            raise SystemExit(f"{name}: unparsed row {r}")
                        rows.append([m[0], str(i), vals[0], pcts[0].rstrip("%"), vals[1], vals[2], pcts[1].rstrip("%"),
                                     vals[3], vals[4], vals[5], vals[6], vals[7]])
                    elif key == "tableB2":
                        if len(vals) != width:
                            raise SystemExit(f"{name}: unparsed row {r}")
                        rows.append([m[0], str(i)] + vals)
                    else:
                        ratio = [x for g in group for x in g[1][1:] if len(g[1]) == 5][0:1]
                        vals = r if len(r) == 4 else [r[0]] + r[2:]
                        if len(vals) != 4 or len(ratio) != 1:
                            raise SystemExit(f"{name}: unparsed row {r}")
                        rows.append([m[0], str(i), vals[0], ratio[0]] + vals[1:])
                del extra
                group = []
        out[key] = rows
    return out


def same(a, b):
    try:
        return Decimal(a) == Decimal(b)
    except InvalidOperation:
        return a == b


def main():
    src, rows_dir, fixes_path, out = sys.argv[1:5]
    fixes = {}
    with open(fixes_path, newline="", encoding="utf-8") as f:
        for r in csv.DictReader(f):
            fixes[(r["ocr_file"], int(r["row"]))] = r["eye_text"]
    A, B = reading_a(src), reading_b(rows_dir, fixes)
    total_cells = total_bad = total_zero = 0
    for key, cols in SCHEMA.items():
        for tag, rows in (("reading_a_chatgpt", A[key]), ("reading_b", B[key])):
            with open(os.path.join(out, f"{key}_{tag}.csv"), "w", newline="", encoding="utf-8") as f:
                w = csv.writer(f, lineterminator="\n")
                w.writerow(cols)
                w.writerows(rows)
        lines = []
        bad = zeros = cells = 0
        if len(A[key]) != len(B[key]):
            lines.append(f"ROW COUNT A={len(A[key])} B={len(B[key])}")
            bad += 1
        for ra, rb in zip(A[key], B[key]):
            for c, x, y in zip(cols, ra, rb):
                cells += 1
                if not same(x, y):
                    lines.append(f"DIFF\t{ra[0]}\trow {ra[1] if len(ra) > 1 else ''}\t{c}\tA={x}\tB={y}")
                    bad += 1
                elif x != y:
                    zeros += 1
        lines.append(f"{key}: rows A={len(A[key])} B={len(B[key])}; cells={cells}; disagreements={bad}; "
                     f"equal as numbers but not as printed strings (A lost a trailing zero or a '.0')={zeros}")
        with open(os.path.join(out, f"{key}_diff.txt"), "w", encoding="utf-8") as f:
            f.write("\n".join(lines) + "\n")
        print(lines[-1] if bad == 0 else "\n".join(lines))
        with open(os.path.join(out, f"{key}_reconciled.csv"), "w", newline="", encoding="utf-8") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(cols)
            w.writerows(B[key])
        total_cells += cells
        total_bad += bad
        total_zero += zeros
    print(f"all tables: cells={total_cells}; disagreements={total_bad}; trailing-zero-only differences={total_zero}")


if __name__ == "__main__":
    main()
