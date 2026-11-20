"""arxiv_licence.py <id>...: read the licence link from each arXiv abstract page, one request at a time.

Polite: one connection, 3 s between requests, a descriptive User-Agent. Prints the id and the licence
URL from the abstract page's licence block (arXiv's "nonexclusive-distrib" and "assumed-1991-2003"
are not open licences; a creativecommons.org URL is).
"""
import re, sys, time, urllib.request

UA = "principia-ingest/1.0 (licence check; polite, one request at a time)"
for i, aid in enumerate(sys.argv[1:]):
    if i:
        time.sleep(3)
    req = urllib.request.Request("https://arxiv.org/abs/" + aid, headers={"User-Agent": UA})
    html = urllib.request.urlopen(req, timeout=30).read().decode("utf-8", "replace")
    m = re.search(r'class="abs-license"[^>]*>.*?href="([^"]+)"', html, re.S)
    v = re.findall(r"\[v(\d+)\]", html)
    print("%s\t%s\tversions %s" % (aid, m.group(1) if m else "no licence link found", max(v, key=int) if v else "?"))
