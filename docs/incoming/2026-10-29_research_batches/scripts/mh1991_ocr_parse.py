"""mh1991_ocr_parse.py <tableI_ocr_raw.txt> <out.csv>: turn the OCR rows of Mikkola & Hietarinta (1991) Table I
into the eye-reading layout (one row per orbit). Lines are classified by content only: 'Ia, Ib' lines carry
the index pairs; '(' lines the masses; a lone degree value line theta; 'exp' lines lambda_l; the remaining
all-numeric lines, in order, R, P, k_l, k_t (k_t is the 4th; a block's lambda line may be partly empty).
Known OCR glyph confusions mapped: ':' inside a number -> '.', '×'/'x 10°' kept as text. No digit edited.
"""
import csv, re, sys
L = [l.rstrip("\n").split("\t") for l in open(sys.argv[1], encoding="utf-8")]
blocks, cur = [], None
for c in L:
    t = " ".join(c)
    if re.search(r"\d+,\s*-?\d+\s+-?\d+,", t) or (len(c) >= 3 and all(re.fullmatch(r"-?\d+,\s*\d+", x) for x in c[-3:])):
        cur = {"I": [x for x in c if re.fullmatch(r"-?\d+,\s*\d+", x)], "num": []}; blocks.append(cur); continue
    if cur is None: continue
    if "exp" in t: cur["lam"] = [re.sub(r"[^0-9.]", "", x.split("exp")[-1]) for x in c if "exp" in x]; continue
    if "(" in t: cur["m"] = re.findall(r"\(([^)]*)\)", t); continue
    vals = [x for x in c if re.fullmatch(r"-?[0-9.:]+°?", x)]
    if len(vals) == 3 and all(v.endswith("°") for v in vals): cur["th"] = [v[:-1] for v in vals]; continue
    if len(vals) == 3 or (len(c) == 3 and "10" in t): cur["num"].append([v.replace(":", ".") for v in c[-3:]])
w = csv.writer(open(sys.argv[2], "w", newline=""), lineterminator="\n")
w.writerow("Ia,Ib,m1,m0,m2,theta_deg,R,P,k_l,lambda_l_deg,k_t".split(","))
for b in blocks:
    for j in range(3):
        ia, ib = [s.strip() for s in b["I"][j].split(",")]
        m = [s.strip() for s in b["m"][j].split(",")]
        lam = b.get("lam", [])
        n = b["num"]
        w.writerow([ia, ib] + m + [b["th"][j], n[0][j], n[1][j], n[2][j], lam[j] if j < len(lam) else "", n[3][j]])
