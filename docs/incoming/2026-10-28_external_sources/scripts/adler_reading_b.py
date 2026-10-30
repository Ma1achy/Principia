"""Adler 2016 (Space Exploration Stack Exchange, answer 15369 to question 15364; CC BY-SA 3.0): reading B.

Usage: python3 -I adler_reading_b.py <answers.json from the Stack Exchange API, filter=withbody> <out.csv>
Parses the answer's own <pre><code> block (the author's text, not ChatGPT's file): each line is a time and three
{x,y} pairs, for the masses 3, 4, 5 in the question's order. Values are copied as printed, except that a trailing '.'
on a whole number (Mathematica's "1.") is dropped, which is recorded in the README.
"""
import csv
import html
import json
import re
import sys

d = json.load(open(sys.argv[1], encoding="utf-8"))
a = [x for x in d["items"] if x["answer_id"] == 15369][0]
block = html.unescape(re.search(r"<pre><code>(.*?)</code></pre>", a["body"], re.S).group(1))
rows = []
for line in block.strip().splitlines():
    t = line.split()[0].rstrip(".")
    pairs = re.findall(r"\{([^,]+),([^}]+)\}", line)
    vals = [v.strip() for p in pairs for v in p]
    vals = [v[:-1] if v.endswith(".") else v for v in vals]
    rows.append([t] + vals)
with open(sys.argv[2], "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["t", "x_m3", "y_m3", "x_m4", "y_m4", "x_m5", "y_m5"])
    w.writerows(rows)
print(f"{sys.argv[2]}: {len(rows)} rows from answer {a['answer_id']} by {a['owner']['display_name']}, "
      f"licence {a.get('content_license')}")
