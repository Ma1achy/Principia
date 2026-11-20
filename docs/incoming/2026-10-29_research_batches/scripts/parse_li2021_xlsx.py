"""Parse Li, Li, He & Liao 2021's supplementary workbook into CSV, from the raw XML.

Usage: python3 -I parse_li2021_xlsx.py <workbook.xlsx> <out_dir>

The workbook is untrusted input: it is read as a zip of XML only (zipfile + ElementTree, stdlib),
nothing in it is executed, and cell values are copied as the exact text stored in the XML (no
float round trip), so the CSV carries every digit the workbook carries. Writes
li2021_xlsx_ics.csv (label, x, y, T, sequence) and li2021_xlsx_parse_report.txt.
"""
import csv
import hashlib
import re
import sys
import xml.etree.ElementTree as ET
import zipfile

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main"}


def shared_strings(z):
    root = ET.fromstring(z.read("xl/sharedStrings.xml"))
    out = []
    for si in root.findall("m:si", NS):
        out.append("".join(t.text or "" for t in si.iter("{%s}t" % NS["m"])))
    return out


def sheet_cells(z, path, sst):
    root = ET.fromstring(z.read(path))
    rows = {}
    for row in root.iter("{%s}row" % NS["m"]):
        r = int(row.get("r"))
        for c in row.findall("m:c", NS):
            v = c.find("m:v", NS)
            if v is None:
                continue
            col = re.match(r"[A-Z]+", c.get("r")).group(0)
            t = c.get("t", "n")
            val = sst[int(v.text)] if t == "s" else v.text
            rows.setdefault(r, {})[col] = (t, val)
    return rows


def main():
    xlsx, out = sys.argv[1], sys.argv[2]
    data = open(xlsx, "rb").read()
    rep = ["workbook sha256 %s, %d bytes" % (hashlib.sha256(data).hexdigest(), len(data))]
    z = zipfile.ZipFile(xlsx)
    for n in z.namelist():
        assert not n.startswith("/") and ".." not in n, n
    sst = shared_strings(z)
    wb = ET.fromstring(z.read("xl/workbook.xml"))
    names = [s.get("name") for s in wb.find("m:sheets", NS)]
    rep.append("sheets: %r" % names)
    ic = sheet_cells(z, "xl/worksheets/sheet2.xml", sst)
    sq = sheet_cells(z, "xl/worksheets/sheet3.xml", sst)
    rep.append("Initial condition A1: %r" % (ic[1]["A"][1],))
    rep.append("Initial condition header row 2: %r" % ([ic[2][k][1] for k in sorted(ic[2])],))
    rep.append("symbol sequences A1: %r" % (sq[1]["A"][1],))
    rep.append("symbol sequences header row 2: %r" % ([sq[2][k][1] for k in sorted(sq[2])],))
    ic_rows = sorted(r for r in ic if r >= 3)
    sq_rows = sorted(r for r in sq if r >= 3)
    rep.append("IC data rows %d (%d..%d); sequence data rows %d (%d..%d)" % (
        len(ic_rows), ic_rows[0], ic_rows[-1], len(sq_rows), sq_rows[0], sq_rows[-1]))
    types = {}
    for r in ic_rows:
        for k, (t, _) in ic[r].items():
            types[(k, t)] = types.get((k, t), 0) + 1
    rep.append("IC cell types (column, xml type): count %r" % (sorted(types.items()),))
    labels_ic = [ic[r]["A"][1].strip() for r in ic_rows]
    labels_sq = [sq[r]["A"][1].strip() for r in sq_rows]
    rep.append("labels unique (IC sheet): %s; IC labels == sequence labels, in order: %s" % (
        len(set(labels_ic)) == len(labels_ic), labels_ic == labels_sq))
    digits = {}
    with open(out + "/li2021_xlsx_ics.csv", "w", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["row", "label", "x", "y", "T", "symbol_sequence"])
        seq_by_label = {sq[r]["A"][1].strip(): sq[r].get("B", ("", ""))[1].strip() for r in sq_rows}
        for r in ic_rows:
            lab = ic[r]["A"][1].strip()
            x, y, T = (ic[r][k][1] for k in "BCD")
            for k, v in zip("xyT", (x, y, T)):
                float(v)
                m = re.sub(r"[eE].*", "", v).lstrip("-").replace(".", "").lstrip("0")
                digits[(k, len(m))] = digits.get((k, len(m)), 0) + 1
            w.writerow([r, lab, x, y, T, seq_by_label.get(lab, "")])
    rep.append("significant digits stored per value (column, digits): count %r" % (sorted(digits.items()),))
    open(out + "/li2021_xlsx_parse_report.txt", "w").write("\n".join(rep) + "\n")
    print("\n".join(rep))


if __name__ == "__main__":
    main()
