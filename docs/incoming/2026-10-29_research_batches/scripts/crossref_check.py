"""crossref_check.py <dois.txt> <out.tsv>: look up each DOI in the Crossref REST API, one request at a time.

Polite: a single connection, one request per 1.5 s, a descriptive User-Agent, no retries beyond one.
Writes doi, first author, year, container, volume, issue, page, article number, title and every
licence URL Crossref records (a Springer TDM or "springer.com/tdm" URL is not an open licence).
"""
import json, sys, time, urllib.request, urllib.parse

UA = "principia-ingest/1.0 (citation check; polite, one request at a time)"

def get(doi):
    url = "https://api.crossref.org/works/" + urllib.parse.quote(doi, safe="/")
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)["message"]

def main():
    dois = [l.strip() for l in open(sys.argv[1]) if l.strip() and not l.startswith("#")]
    with open(sys.argv[2], "w") as out:
        out.write("doi\tfirst_author\tyear\tcontainer\tvolume\tissue\tpage\tarticle_number\ttitle\tlicences\n")
        for doi in dois:
            try:
                m = get(doi)
                a = m.get("author", [{}])[0]
                yr = (m.get("issued", {}).get("date-parts") or [[None]])[0][0]
                lic = ";".join(sorted({l.get("URL", "") for l in m.get("license", [])})) or "none recorded"
                row = [doi, a.get("family", ""), str(yr), (m.get("container-title") or [""])[0],
                       m.get("volume", ""), m.get("issue", ""), m.get("page", ""), m.get("article-number", ""),
                       (m.get("title") or [""])[0].replace("\t", " ").replace("\n", " "), lic]
            except Exception as e:
                row = [doi, "LOOKUP FAILED: %s" % e] + [""] * 8
            out.write("\t".join(row) + "\n")
            print(row[0], row[1], row[2], row[4], row[6] or row[7], row[9][:80])
            time.sleep(1.5)

main()
