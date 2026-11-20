"""inventory.py <out.tsv> <batch>=<dir> ...: list every file of each batch copy with size and SHA-256.

Reads bytes only (hashing); nothing in the batches is opened as code or parsed. The role and disposition
columns follow the file's extension and directory: papers and supplements stay local (docs/incoming/_local/),
notes and ChatGPT transcriptions are inventoried only, and the zips are the containers whose entries are listed
under their own batch name.
"""
import hashlib, os, sys

def role(path):
    p = path.lower()
    if p.endswith(".pdf") or p.endswith(".xlsx"):
        return "paper" if "supp" not in p else "supplement", "local only (docs/incoming/_local/research_batches_2026-10-29/)"
    if p.endswith(".zip"):
        return "archive", "not committed; entries listed under the unzipped batch"
    if "/data/" in p:
        return "ChatGPT transcription (reading A)", "not committed; reading A recorded as CSV in this directory"
    return "batch notes", "not committed; summarised in README.md"

def main():
    out = open(sys.argv[1], "w")
    out.write("batch\tpath\tbytes\tsha256\trole\tdisposition\n")
    for spec in sys.argv[2:]:
        batch, d = spec.split("=", 1)
        for root, dirs, files in os.walk(d):
            dirs.sort()
            for f in sorted(files):
                if f == ".DS_Store":
                    continue
                full = os.path.join(root, f)
                rel = os.path.relpath(full, d)
                h = hashlib.sha256(open(full, "rb").read()).hexdigest()
                r, disp = role("/" + rel)
                out.write("%s\t%s\t%d\t%s\t%s\t%s\n" % (batch, rel, os.path.getsize(full), h, r, disp))

main()
