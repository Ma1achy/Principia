#!/usr/bin/env python3
"""Build INVENTORY.tsv for the 2026-10-28 external-sources ingest.

Run with `python3 -I inventory.py <inputs_root> <out.tsv>`. <inputs_root> holds the scratch copies:
  zip_<name>/<name>.zip and zip_<name>/x/<unzipped tree>  (unzipped with `unzip -n` after a traversal check)
  dir_<name>/<name>/<tree>                               (copy of the owner's unzipped folder)
  dir_springer/, owner_melnikov/                         (owner's own downloads)
Nothing in the inputs is executed; files are only read and hashed.

Each file gets: package, path, size, sha256, kind, claimed source, licence (and whether we verified it),
disposition (committed / _local / not committed, with the reason), and, for the zip and folder copies,
whether the other copy holds the same bytes.
"""
import hashlib
import os
import re
import sys

ES = "docs/incoming/2026-10-28_external_sources"
LOCAL = "docs/incoming/_local/external_sources_2026-10-28"

CHATGPT = ("file written by ChatGPT/Codex", "ChatGPT/Codex package", "n/a (ChatGPT output)", "-",
           "not committed: untrusted ChatGPT output; its transcriptions are reading A only, superseded by our "
           "own readings in " + ES)

# (regex on the path inside the package, kind, claimed source, licence, licence status, disposition)
RULES = [
    (r"\.DS_Store$", "macOS metadata", "-", "-", "-", "not committed: Finder metadata"),
    # archives
    (r"archives/choreo\.zip$", "code archive (C++, CAPD-based)",
     "Kapela & Simo 2017 code, https://ww2.ii.uj.edu.pl/~kapela/papers/nosym/choreo.zip",
     "GPL v2 (root license.txt; CAPD files carry the GPL notice; Kapela's private/ files say "
     "'Copyright (C) Tomasz Kapela 2006' only)", "verified in the archive",
     "_local: " + LOCAL + "/kapela_simo/ (GPL v2: not a licence the owner allows for commits; fetch.sh retrieves it)"),
    (r"archives/KAMstability\.tar\.gz$", "code and results archive",
     "Kapela & Simo 2007, https://ww2.ii.uj.edu.pl/~kapela/papers/KAM/KAMstability.tar.gz",
     "none stated", "verified: no licence file in the archive", "_local: " + LOCAL + "/kapela_simo/"),
    (r"archives/results\.zip$", "results archive",
     "Kapela & Simo 2017, https://ww2.ii.uj.edu.pl/~kapela/papers/nosym/results.zip",
     "none stated", "verified: no licence file in the archive", "_local: " + LOCAL + "/kapela_simo/"),
    # papers in the complete package
    (r"papers/Jankovic_Dmitrasinovic_Suvakov_2020\.pdf$", "paper PDF (author copy)",
     "Jankovic, Dmitrasinovic & Suvakov 2020, CPC 250, 107052, https://www.scl.rs/papers/MJankovic_2020_CPC.pdf",
     "(c) Elsevier", "verified in the PDF", "_local: " + LOCAL + "/jankovic_2020/"),
    (r"papers/Kapela_Simo_2007_author_preprint\.pdf$", "paper PDF (author preprint)",
     "Kapela & Simo 2007, Nonlinearity 20, 1241, https://ww2.ii.uj.edu.pl/~kapela/papers/kamstab.pdf",
     "none stated", "verified: no licence in the PDF", "_local: " + LOCAL + "/kapela_simo/"),
    (r"papers/Kapela_Simo_2017_author_preprint\.pdf$", "paper PDF (author preprint)",
     "Kapela & Simo 2017, Nonlinearity 30, 965, https://ww2.ii.uj.edu.pl/~kapela/papers/nosym.pdf (arXiv:1105.3235)",
     "arXiv nonexclusive-distrib 1.0; none in the PDF", "verified on the arXiv abstract page",
     "_local: " + LOCAL + "/kapela_simo/"),
    (r"papers/Kapela_Zgliczynski_2003_author_preprint\.pdf$", "paper PDF (author preprint)",
     "Kapela & Zgliczynski 2003, Nonlinearity 16, 1899, https://ww2.ii.uj.edu.pl/~kapela/papers/nbody.pdf",
     "none stated", "verified: no licence in the PDF", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Orlov_Titov_Shombina_2016_ABSTRACT_ONLY\.pdf$", "publisher abstract page (1 page)",
     "Orlov, Titov & Shombina 2016, https://www.pleiades.online/abstract/asteng/16/asteng1083_abstract.pdf",
     "(c) Pleiades", "verified in the PDF",
     "not committed: superseded by the owner's full copy (" + LOCAL + "/orlov_titov_shombina_2016/)"),
    (r"papers/Quarles_et_al_2020_arxiv_1912\.11019\.pdf$", "paper PDF (arXiv)",
     "Quarles et al. 2020, arXiv:1912.11019", "arXiv nonexclusive-distrib 1.0",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Tanikawa_Saito_Mikkola_2019_accepted_manuscript\.pdf$", "paper PDF (accepted manuscript)",
     "Tanikawa, Saito & Mikkola 2019, CeMDA 131, 24, utupub.fi repository",
     "none (accepted manuscript, no open licence)", "verified in the PDF",
     "not committed: the owner's identical copy is at docs/incoming/_local/tanikawa_saito_mikkola_2019/"),
    (r"papers/Tanikawa_2019_DOWNLOAD_LINK\.txt$", *CHATGPT),
    # papers in the expanded package
    (r"papers/Dandekar_et_al_2026\.pdf$", "paper PDF (arXiv)",
     "Dandekar, Springer, Zard, Trani, Kol & Birnholtz 2026, arXiv:2608.30414", "CC BY 4.0",
     "verified on the arXiv abstract page", "committed: " + ES + "/dandekar_2026/Dandekar_et_al_2026_arXiv_2608.30414.pdf"),
    (r"papers/Dandekar_Kol_2025\.pdf$", "paper PDF (publisher version)",
     "Dandekar & Kol 2025, CeMDA 137, 32, doi:10.1007/s10569-025-10262-2", "CC BY 4.0",
     "verified in the PDF ('Open Access This article is licensed under a Creative Commons Attribution 4.0 "
     "International License')", "committed: " + ES + "/dandekar_kol_2025/Dandekar_Kol_2025_CeMDA_137_32.pdf"),
    (r"papers/Dandekar_Kol_2025_supplement\.pdf$", "supplementary PDF",
     "Dandekar & Kol 2025, ESM 1", "none stated", "verified: no licence in the PDF",
     "_local: " + LOCAL + "/dandekar_kol_2025/"),
    (r"papers/Dandekar_et_al_2022\.pdf$", "paper PDF (arXiv v1)",
     "Dandekar et al. 2022, CeMDA 134, 55, arXiv:2205.04294", "arXiv nonexclusive-distrib 1.0",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Johnstone_Rucinski_1991\.pdf$", "paper PDF (publisher)",
     "Johnstone & Rucinski 1991, PASP 103, 359, doi:10.1086/132828", "none stated",
     "checked: no licence found in the PDF (scanned)", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Mikkola_Merritt_2008\.pdf$", "paper PDF (arXiv)",
     "Mikkola & Merritt 2008, AJ 135, 2398, arXiv:0709.3367", "arXiv nonexclusive-distrib 1.0",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/mikkola_merritt_2008/"),
    (r"papers/Nauenberg_2001\.pdf$", "paper PDF (arXiv)",
     "Nauenberg 2001, PLA 292, arXiv:nlin/0112003", "arXiv assumed-1991-2003",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Rein_Spiegel_2015\.pdf$", "paper PDF (arXiv)",
     "Rein & Spiegel 2015, MNRAS 446, 1424, arXiv:1409.4779", "arXiv nonexclusive-distrib 1.0",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/record_only/"),
    (r"papers/Rose_Dullin_2013\.pdf$", "paper PDF (arXiv)",
     "Rose & Dullin 2013, CeMDA 117, 169, arXiv:1302.0912", "arXiv nonexclusive-distrib 1.0",
     "verified on the arXiv abstract page", "_local: " + LOCAL + "/record_only/"),
    (r"papers/.*\.txt$", "text extraction of the PDF beside it (made by ChatGPT)", "ChatGPT/Codex output",
     "as the PDF", "-", "not committed: derived from a PDF; our reading A is our own extraction or ChatGPT's "
     "JSON, reading B from page images"),
    # data from openly licensed upstreams, kept out of the commit
    (r"data/Peter_2025/", "Zenodo record file", "Peter 2025, Zenodo 10.5281/zenodo.17635887",
     "MIT", "verified via the Zenodo API (license mit-license); md5 of all 13 files match the record",
     "not committed: record only (double precision only, no target); fetch.sh retrieves it (applied per R-369)"),
    (r"data/Quarles_planar_.*\.txt$", "excerpt of a Zenodo data file (made by ChatGPT)",
     "Quarles et al. 2020, Zenodo 3579202", "CC BY 4.0 (upstream)", "verified via the Zenodo API",
     "not committed: finite-radius stars, out of scope; record only"),
    (r"web_sources/", "saved web page (fetched by ChatGPT)", "see file name", "website terms",
     "not verified", "not committed: page snapshot; we fetched the pages we cite ourselves"),
    # springer batch (owner's downloads)
    (r"Tanikawa_Umehara_Abe_1995_Collision_orbits_I\.pdf$", "paper PDF (publisher, scanned)",
     "Tanikawa, Umehara & Abe 1995, CeMDA 62, 335, doi:10.1007/BF00692285", "(c) Springer (Kluwer)",
     "verified in the PDF", "_local: " + LOCAL + "/springer/"),
    (r"Anosova_1986_.*\.pdf$", "paper PDF (publisher, scanned)",
     "Anosova 1986, Ap&SS 124, 217, doi:10.1007/BF00656037", "(c) Springer (Reidel)", "verified in the PDF",
     "_local: " + LOCAL + "/springer/"),
    (r"Umehara_Tanikawa_2000_.*\.pdf$", "paper PDF (publisher)",
     "Umehara & Tanikawa 2000, CeMDA 76, 187, doi:10.1023/A:1008305928852", "(c) Springer (Kluwer)",
     "verified in the PDF", "_local: " + LOCAL + "/springer/"),
    (r"Saito_Tanikawa_2007_.*\.pdf$", "paper PDF (publisher)",
     "Saito & Tanikawa 2007, CeMDA 98, 95, doi:10.1007/s10569-007-9070-0", "(c) Springer",
     "verified in the PDF", "_local: " + LOCAL + "/springer/"),
    (r"Saito_Tanikawa_2009_.*\.pdf$", "paper PDF (publisher)",
     "Saito & Tanikawa 2009, CeMDA 103, 191, doi:10.1007/s10569-008-9175-0", "(c) Springer",
     "verified in the PDF", "_local: " + LOCAL + "/springer/"),
    (r"Aarseth_Zare_1974_.*\.pdf$", "paper PDF (publisher)",
     "Aarseth & Zare 1974, Celest. Mech. 10, 185, doi:10.1007/BF01227619", "(c) Springer (Reidel)",
     "verified in the PDF", "not copied: already recorded with this sha256 in "
     "docs/incoming/2026-10-25_replication/README.md:13 (owner said skip if already in the repo)"),
    (r"Heggie_1974_.*\.pdf$", "paper PDF (publisher)",
     "Heggie 1974, Celest. Mech. 10, 217, doi:10.1007/BF01227621", "(c) Springer (Reidel)",
     "verified in the PDF", "not copied: already recorded with this sha256 in "
     "docs/incoming/2026-10-25_replication/README.md:14 (owner said skip if already in the repo)"),
    (r"Pasechnik_.*\.pdf$", "paper PDF (publisher version with Turku cover sheet)",
     "Pasechnik, Myllari, Valtonen & Mikkola 2023, MNRAS 525, 1929, doi:10.1093/mnras/stad2372",
     "CC BY 4.0 (Crossref vor licence 2023-08-04; PDF '(c) 2023 The Author(s)'; cover sheet 'All rights reserved')",
     "verified via Crossref and the PDF",
     "not committed: byte-identical to docs/incoming/2026-10-28_rq598_data/papers/"
     "pasechnik_myllari_valtonen_mikkola_2023_MNRAS_525_1929.pdf (PR #327)"),
    (r"^(README\.md|download_manifest\.json|checksums\.sha256)$", "batch notes or checksum list (ChatGPT-written)",
     "ChatGPT/Codex package, or the owner's download session", "n/a", "-",
     "not committed: notes; each fact we use is re-verified and cited in README.md"),
    (r"Melnikov_Orlov_Shevchenko_2013_Lyapunov_exponents\.pdf$", "paper PDF (publisher)",
     "Mel'nikov, Orlov & Shevchenko 2013, Astron. Rep. 57, 429, doi:10.1134/S1063772913060048",
     "(c) Pleiades", "verified in the PDF", "_local: " + LOCAL + "/melnikov_2013/"),
    (r"(^|/)raw/(3stability\.txt|KAMstability/result/eight(_148|_488)?\.txt)$",
     "result log copied out of the Kapela archives", "Kapela & Simo 2007/2017 archives",
     "none stated (as the archives)", "verified: byte-identical to the file inside the archive",
     "not committed: the archives are in " + LOCAL + "/kapela_simo/"),
    # previous_verified_bundle inside the expanded package: a copy of the complete package
    # everything else ChatGPT wrote (README, RESEARCH_*, TABLES, LINKS, JSON, scripts, checksums)
    (r".*", *CHATGPT),
]


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def walk(root):
    out = {}
    for d, _, files in os.walk(root):
        for fn in files:
            p = os.path.join(d, fn)
            rel = os.path.relpath(p, root)
            out[rel] = (os.path.getsize(p), sha256(p))
    return out


def classify(rel):
    key = rel.split("previous_verified_bundle/", 1)[-1]
    for rx, *rest in RULES:
        if re.search(rx, key):
            return rest
    raise SystemExit("no rule for " + rel)


def main(root, out_path):
    pkgs = [
        ("kapela_simo_comparison_package", "kapela_simo_benchmarks", "kapela_simo_benchmarks"),
        ("planar_three_body_research_complete", "planar_three_body_research", "planar_three_body_research"),
        ("planar_three_body_research_expanded", "planar_three_body_research_expanded",
         "planar_three_body_research_expanded"),
    ]
    rows = []
    summary = []
    trees = {}
    for zname, inner, dname in pkgs:
        zdir = os.path.join(root, "zip_" + zname)
        zpath = os.path.join(zdir, zname + ".zip")
        size, digest = os.path.getsize(zpath), sha256(zpath)
        rows.append(["zip:" + zname + ".zip", "(the zip itself)", size, digest, "zip archive",
                     "ChatGPT/Codex package", "n/a (ChatGPT output)", "-",
                     "not committed: contents inventoried below", "-"])
        ztree = walk(os.path.join(zdir, "x", inner))
        ftree = walk(os.path.join(root, "dir_" + dname, dname))
        trees[zname] = ztree
        for label, tree, other in (("zip:" + zname + ".zip", ztree, ftree),
                                   ("folder:" + dname, ftree, ztree)):
            for rel in sorted(tree):
                size, digest = tree[rel]
                if rel not in other:
                    eq = "absent from the other copy"
                elif other[rel][1] == digest:
                    eq = "identical in the other copy"
                else:
                    eq = "DIFFERS from the other copy"
                kind, src, lic, licst, disp = classify(rel)
                if "previous_verified_bundle/" in rel:
                    kind = kind + " (in previous_verified_bundle/)"
                rows.append([label, rel, size, digest, kind, src, lic, licst, disp, eq])
        only_z = sorted(set(ztree) - set(ftree))
        only_f = sorted(set(ftree) - set(ztree))
        diff = sorted(r for r in set(ztree) & set(ftree) if ztree[r][1] != ftree[r][1])
        summary.append(f"{zname}: zip {len(ztree)} files, folder {len(ftree)} files; only in zip {only_z}; "
                       f"only in folder {only_f}; differing {diff}")
    # previous_verified_bundle vs the complete package
    exp = trees["planar_three_body_research_expanded"]
    comp = trees["planar_three_body_research_complete"]
    pvb = {r.split("previous_verified_bundle/", 1)[1]: v for r, v in exp.items()
           if r.startswith("previous_verified_bundle/")}
    only_p = sorted(set(pvb) - set(comp))
    only_c = sorted(set(comp) - set(pvb))
    diff = sorted(r for r in set(pvb) & set(comp) if pvb[r][1] != comp[r][1])
    summary.append(f"expanded/previous_verified_bundle vs complete package: {len(pvb)} vs {len(comp)} files; "
                   f"only in bundle {only_p}; only in complete {only_c}; differing {diff}")
    # owner's own downloads
    for label, sub in (("owner:springer", "dir_springer"), ("owner:melnikov", "owner_melnikov")):
        tree = walk(os.path.join(root, sub))
        for rel in sorted(tree):
            size, digest = tree[rel]
            kind, src, lic, licst, disp = classify(rel)
            rows.append([label, rel, size, digest, kind, src, lic, licst, disp, "-"])
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("package\tpath\tsize_bytes\tsha256\tkind\tclaimed_source\tlicence\tlicence_status\t"
                "disposition\tzip_vs_folder\n")
        for r in rows:
            f.write("\t".join(str(c).replace("\t", " ") for c in r) + "\n")
    for s in summary:
        print(s)
    print(len(rows), "rows")


if __name__ == "__main__":
    main(*sys.argv[1:3])
