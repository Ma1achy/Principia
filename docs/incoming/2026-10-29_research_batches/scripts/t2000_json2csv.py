"""t2000_json2csv.py <Tanikawa_2000_all_eight_tables.json> <outdir>: reading A (ChatGPT's JSON) as one CSV per table.

Columns are renamed positionally to the reading-B (eye) headers so the two readings diff cell by cell;
cell text is kept exactly as the JSON gives it. Run with python3 -I; the JSON is untrusted input and is
only parsed, never executed.
"""
import csv, json, os, sys

EYE = {
    "I": ["n_min", "x", "y", "alpha", "time"],
    "V": ["n_min", "x", "y", "theta_deg", "time", "alpha_deg"],
    "VIII": ["n_min", "x", "y", "theta_deg", "time", "type", "phi"],
}
for t in ("II", "III", "IV"):
    EYE[t] = EYE["I"]
for t in ("VI", "VII"):
    EYE[t] = EYE["V"]

def main():
    d = json.load(open(sys.argv[1], encoding="utf-8"))
    out = sys.argv[2]
    for t in d["tables"]:
        cols, hdr = t["columns"], EYE[t["table"]]
        assert len(cols) == len(hdr), (t["table"], cols)
        with open(os.path.join(out, t["table"] + ".csv"), "w", newline="") as f:
            w = csv.writer(f, lineterminator="\n")
            w.writerow(hdr)
            for r in t["rows"]:
                w.writerow([str(r[c]) for c in cols])
        print(t["table"], "pdf p.", t["pdf_page"], "rows", len(t["rows"]))

main()
