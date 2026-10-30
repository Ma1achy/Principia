"""Reading A extractor: turns a ChatGPT JSON transcription (a list of rows under one key) into a CSV, verbatim.

Usage: python3 -I json_rows_to_csv.py <in.json> <rows_key> <out.csv> <col1,col2,...> [--fields k1,k2,...]
If the rows are lists, they are written in order under the given column names. If they are objects, --fields names the
keys to take, in the order of the columns. Every value is written as the string the JSON holds (numbers via repr of the
parsed JSON token, which json keeps exact for ints and shortest-round-trip for floats; the script reports any float).
"""
import csv
import json
import sys


def main():
    src, key, out, cols = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4].split(",")
    fields = None
    if "--fields" in sys.argv:
        fields = sys.argv[sys.argv.index("--fields") + 1].split(",")
    with open(src, encoding="utf-8") as f:
        data = json.load(f, parse_float=lambda s: ("FLOAT", s))
    rows = data
    for part in key.split("/"):
        if part:
            rows = rows[part]
    floats = 0
    with open(out, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(cols)
        for r in rows:
            vals = r if isinstance(r, list) else [r.get(k, "") for k in fields]
            outv = []
            for v in vals:
                if isinstance(v, tuple) and v and v[0] == "FLOAT":
                    floats += 1
                    v = v[1]
                outv.append("" if v is None else str(v))
            w.writerow(outv)
    print(f"{out}: {len(rows)} rows; {floats} cells were JSON floats (kept as their literal token)")


if __name__ == "__main__":
    main()
