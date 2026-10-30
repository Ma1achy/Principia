"""ocrrows.py <png> [lang]: run the Vision OCR helper and group its boxes into table rows (tab-separated).

The helper is ocr.swift, compiled with `swiftc -O ocr.swift -o <bin>`; OCR_BIN names the binary
(default: ./ocr beside this script). It reads only the rendered page image, never a PDF text layer.
"""
import subprocess, sys, statistics, os
here = os.path.dirname(os.path.abspath(__file__))
args = [os.environ.get("OCR_BIN", os.path.join(here, "ocr")), sys.argv[1]] + sys.argv[2:3]
out = subprocess.run(args, capture_output=True, text=True, check=True).stdout
boxes = []
for line in out.splitlines():
    if "\t" not in line: continue
    head, text = line.split("\t", 1)
    y, x, h = map(float, head.split())
    boxes.append((y, x, h, text))
if not boxes: sys.exit(0)
tol = statistics.median(b[2] for b in boxes) * 0.5
boxes.sort()
rows, cur = [], [boxes[0]]
for b in boxes[1:]:
    if abs(b[0] - statistics.mean(c[0] for c in cur)) <= tol: cur.append(b)
    else: rows.append(cur); cur = [b]
rows.append(cur)
for r in rows:
    print("\t".join(t for _, _, _, t in sorted(r, key=lambda b: b[1])))
