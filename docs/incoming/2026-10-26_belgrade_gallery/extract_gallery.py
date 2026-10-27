"""Extract every solution page of the recovered Belgrade three-body gallery into one CSV.

Usage: python3 -I extract_gallery.py <gallery_dir> <out.csv>

Each solution page (`<page>_sol.php__id_<n>.html`, and the id-less default page of each kind) carries its data in a
JavaScript string, `var text = '...'`, typed out character by character by `type()`. That function substitutes some
characters on display, and the substitution map differs from page to page (`t` is reference [11] on some pages and
[6-7] on others). The raw string is kept, and the `REF` field is decoded with the page's own map. The free-group words
use `a`, `b` for the generators and `A`, `B` for their inverses; `c`/`d` (displayed as a^-1, b^-1) never occur in a
word, and `^` with a number is an exponent, as typed on the satellite pages.
"""

import csv
import glob
import html
import os
import re
import sys

SUB = re.compile(r"text\.charAt\(index\)=='(.)'\)\s*document\.getElementById\('screen'\)\.innerHTML \+= '([^']*)'")


def page_map(src):
    m = {}
    for c, rep in SUB.findall(src):
        r = re.search(r"\[([^\]]+)\]", rep)
        m[c] = f"[{r.group(1)}]" if r else re.sub(r"<[^>]+>", "", rep)
    return m


def main(gdir, out):
    rows = []
    for path in sorted(glob.glob(os.path.join(gdir, "*sol.php*.html"))):
        f = os.path.basename(path)
        src = open(path, encoding="utf-8").read()
        raw = re.search(r"var text = '(.*?)';", src, re.S).group(1)
        kind = f.split(".php")[0]
        pid = re.search(r"__id_(\d+)", f)
        fields = {}
        for line in raw.replace("\\n", "\n").replace("@", "\n").split("\n"):
            for k, v in re.findall(r"([A-Za-z0-9. ]+?):\s*([^:]*?)(?=\s+[A-Z][A-Z0-9]*\d?:|$)", line.strip()):
                fields.setdefault(k.strip(), v.strip())
        mp = page_map(src)
        ref_raw = fields.get("REF", "")
        ref = " ".join(mp.get(c, c) for c in ref_raw.split()) if ref_raw else ""
        rows.append({
            "file": f, "kind": kind, "id": pid.group(1) if pid else "",
            "name": html.unescape(fields.get("NAME", "")), "discovered": fields.get("DISCOVERED", ""),
            "word": fields.get("FREE GROUP ELEMENT", ""), "period": fields.get("PERIOD", ""),
            "energy": fields.get("ENERGY", ""), "angular_momentum": fields.get("ANGULAR MOMENTUM", ""),
            "positions": fields.get("INITIAL POSITIONS", ""), "velocities": fields.get("INITIAL VELOCITIES", ""),
            "p1": fields.get("p1", ""), "p2": fields.get("p2", ""),
            "rel_coord_a": fields.get("REL. COORD. a", ""), "rel_vel_c": fields.get("REL. VEL. c", ""),
            **{k.lower(): fields.get(k, "") for k in ("X1", "Y1", "X2", "Y2", "X3", "Y3", "VX1", "VY1", "VX2", "VY2", "VX3", "VY3")},
            "ref_raw": ref_raw, "ref": ref, "raw_text": raw,
        })
    with open(out, "w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)
    print(len(rows), "rows")


if __name__ == "__main__":
    main(*sys.argv[1:3])
