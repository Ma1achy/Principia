"""misc_json2csv.py <kind> <json> <out.csv>: reading A (ChatGPT's JSON) as CSV for the small sources.

kind is tu1998 (Fig. 4 starting points), mo2014 (grid summary and Figs. 9-15 captions) or
melnikov2013 (the p. 438 fit table). Cell text is kept as the JSON gives it. Run with python3 -I; the
JSON is untrusted input and is only parsed, never executed.
"""
import csv, json, sys

def main():
    kind, src, out = sys.argv[1:4]
    d = json.load(open(src, encoding="utf-8"))
    rows = []
    if kind == "tu1998":
        hdr = ["panel", "x", "y"]
        rows = [[c["panel"], c["x"], c["y"]] for c in d["cases"]]
    elif kind == "mo2014":
        hdr = ["item", "xi", "eta", "value"]
        g = d["grid_summary"]
        rows = [["step", g["step_xi"], g["step_eta"], ""],
                ["total", "", "", str(g["total"])],
                ["decayed_before_limit", "", "", str(g["decayed_before_limit"])],
                ["not_decayed", "", "", str(g["not_decayed_before_limit"])],
                ["t_crit_Tcr", "", "", str(g["limit_Tcr"])]]
        rows += [["fig%d" % c["figure"], c["xi"], c["eta"], c["plot_duration_Tcr"]] for c in d["cases"]]
    elif kind == "melnikov2013":
        hdr = ["i", "gamma", "gamma_pm", "C", "C_pm", "R2"]
        rows = [[str(r["i"]), r["gamma"], r["gamma_plus_minus"], r["C"], r["C_plus_minus"], r["R_squared"]]
                for r in d["rows"]]
    else:
        sys.exit("unknown kind")
    with open(out, "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(hdr)
        w.writerows(rows)
    print(kind, len(rows), "rows")

main()
