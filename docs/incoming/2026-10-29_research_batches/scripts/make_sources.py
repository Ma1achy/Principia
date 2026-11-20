"""make_sources.py <local_dir> <doi_check.tsv> <arxiv_licences.tsv> <out.tsv> <manifest.json>...: build sources.tsv.

One row per paper or supplement copied to docs/incoming/_local/research_batches_2026-10-29/. Citation and DOI come
from the batch manifests (parsed as JSON only); the DOI's Crossref record (doi_check.tsv) and the arXiv licence
(arxiv_licences.tsv) are the verification. The route says how fetch.sh gets the file back: "arxiv" (byte-identical
arXiv version, fetched), "publisher" (subscription; fetch.sh only checks a copy you place by hand) or "owner-copy"
(local only; fetch.sh leaves it out).
"""
import csv, hashlib, json, os, sys

ARXIV = {"Hernandez_2015_Symplectic_Collisional_Integrator.pdf": "1503.02728v2",
         "Moore_2006_New_Periodic_Orbits.pdf": "math/0511219v2",
         "Rantala_2020_MSTAR.pdf": "2001.03180v1"}
ARXIV_ONLY_LISTED = {"Kapela_2003_Certified_Choreographies.pdf": "math/0304404",
                     "Vanderbei_2004_New_Orbits.pdf": "astro-ph/0303153"}
ADS = {"Burrau_1913_Pythagorean_Problem.pdf": "1913AN....195..113B",
       "Schubart_1956_Periodic_Orbits.pdf": "1956AN....283...17S"}

def main():
    local, doitsv, arxtsv, out = sys.argv[1:5]
    cr = {r["doi"]: r for r in csv.DictReader(open(doitsv), delimiter="\t")}
    arx = {l.split("\t")[0]: l.split("\t")[1] for l in open(arxtsv) if l.strip()}
    meta = {}
    for m in sys.argv[5:]:
        for it in json.load(open(m)):
            meta[os.path.basename(it["file"])] = it
    rows = []
    for sub in sorted(os.listdir(local)):
        for f in sorted(os.listdir(os.path.join(local, sub))):
            p = os.path.join(local, sub, f)
            it = meta[f]
            doi = it["doi"]
            c = cr[doi]
            cite = it.get("citation") or it.get("title")
            crv = "%s %s, %s %s, %s" % (c["first_author"], c["year"], c["container"], c["volume"],
                                       c["page"] or c["article_number"])
            aid = ARXIV.get(f) or ARXIV_ONLY_LISTED.get(f, "")
            lic_parts = ["Crossref: " + c["licences"]]
            if aid:
                lic_parts.append("arXiv %s: %s" % (aid, arx.get(aid.rsplit("v", 1)[0] if aid[-2:-1] == "v" else aid, "?")))
            lic = "no open licence (" + "; ".join(lic_parts) + ")"
            if f in ADS:
                route, url = "owner-copy", "https://ui.adsabs.harvard.edu/abs/%s" % ADS[f]
            elif f in ARXIV:
                route, url = "arxiv", "https://arxiv.org/pdf/" + ARXIV[f]
            else:
                route, url = "publisher", "https://doi.org/" + doi
            rows.append([sub + "/" + f, hashlib.sha256(open(p, "rb").read()).hexdigest(), doi, crv, cite, lic,
                         route, url, ADS.get(f, ""), "local only"])
    with open(out, "w", newline="") as o:
        w = csv.writer(o, delimiter="\t", lineterminator="\n")
        w.writerow(["path", "sha256", "doi", "crossref_record", "citation", "licence_verified", "route", "url",
                    "ads_bibcode", "committed"])
        w.writerows(rows)
    print(len(rows), "rows")

main()
