#!/usr/bin/env python3
"""Write sources.tsv: every source file this ingest used or recorded, where it lives, its hash, and how to get it.

Usage (from the repository root): python3 -I <this> <INVENTORY.tsv> <out sources.tsv>
Sizes and sha256 are computed from the files in the worktree (committed or under docs/incoming/_local/); for files kept
in neither place (Peter 2025's Zenodo files) they are copied from INVENTORY.tsv. fetch.sh fetches exactly the rows
marked 'yes' in the fetchable column and checks them against these hashes.
"""
import csv
import hashlib
import os
import sys

ES = "docs/incoming/2026-10-28_external_sources"
L = "docs/incoming/_local/external_sources_2026-10-28"
LO = "docs/incoming/_local"
UJ = "https://ww2.ii.uj.edu.pl/~kapela/papers/"
ZP = "https://zenodo.org/records/17635887/files/"

# (source, file path, origin, url, fetchable, licence)
ROWS = [
    ("Tanikawa, Saito & Mikkola 2019, CeMDA 131, 24 (accepted manuscript)",
     f"{LO}/tanikawa_saito_mikkola_2019/tanikawa_saito_mikkola_2019_CeMDA_131_24_accepted.pdf",
     "owner's copy, not fetchable", "-", "no", "none (accepted manuscript)"),
    ("Jankovic, Dmitrasinovic & Suvakov 2020, CPC 250, 107052",
     f"{L}/jankovic_2020/Jankovic_Dmitrasinovic_Suvakov_2020.pdf", "ChatGPT package (author's site)",
     "https://www.scl.rs/papers/MJankovic_2020_CPC.pdf", "yes", "(c) Elsevier"),
    ("Kapela & Simo 2007, Nonlinearity 20, 1241 (author preprint)",
     f"{L}/kapela_simo/Kapela_Simo_2007_author_preprint.pdf", "ChatGPT package (author's site)",
     UJ + "kamstab.pdf", "yes", "none stated"),
    ("Kapela & Simo 2017, Nonlinearity 30, 965 (author preprint; arXiv:1105.3235)",
     f"{L}/kapela_simo/Kapela_Simo_2017_author_preprint.pdf", "ChatGPT package (author's site)",
     UJ + "nosym.pdf", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Kapela & Simo 2007 code and results, KAMstability.tar.gz", f"{L}/kapela_simo/KAMstability.tar.gz",
     "ChatGPT package (author's site)", UJ + "KAM/KAMstability.tar.gz", "yes", "none stated"),
    ("Kapela & Simo 2017 results, results.zip", f"{L}/kapela_simo/results.zip", "ChatGPT package (author's site)",
     UJ + "nosym/results.zip", "yes", "none stated"),
    ("Kapela & Simo 2017 code, choreo.zip", f"{L}/kapela_simo/choreo.zip", "ChatGPT package (author's site)",
     UJ + "nosym/choreo.zip", "yes", "GPL v2 (local only: not a licence the owner allows for commits)"),
    ("Kapela & Zgliczynski 2003, Nonlinearity 16, 1899 (author preprint)",
     f"{L}/record_only/Kapela_Zgliczynski_2003_author_preprint.pdf", "ChatGPT package (author's site)",
     UJ + "nbody.pdf", "yes", "none stated"),
    ("Quarles et al. 2020 (arXiv:1912.11019v1)", f"{L}/record_only/Quarles_et_al_2020_arxiv_1912.11019.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/1912.11019v1", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Dandekar et al. 2026, arXiv:2608.30414v1", f"{ES}/dandekar_2026/Dandekar_et_al_2026_arXiv_2608.30414.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/2608.30414v1", "committed", "CC BY 4.0"),
    ("Dandekar & Kol 2025, CeMDA 137, 32", f"{ES}/dandekar_kol_2025/Dandekar_Kol_2025_CeMDA_137_32.pdf",
     "ChatGPT package (Springer, open access)",
     "https://link.springer.com/content/pdf/10.1007/s10569-025-10262-2.pdf", "committed", "CC BY 4.0"),
    ("Dandekar & Kol 2025, supplementary material (ESM 1)",
     f"{L}/dandekar_kol_2025/Dandekar_Kol_2025_supplement.pdf", "ChatGPT package (Springer)",
     "https://media.springernature.com/original/springer-static/esm/art%3A10.1007%2Fs10569-025-10262-2/"
     "MediaObjects/10569_2025_10262_MOESM1_ESM.pdf", "yes", "none stated"),
    ("Mikkola & Merritt 2008, AJ 135, 2398 (arXiv:0709.3367v2)",
     f"{L}/mikkola_merritt_2008/Mikkola_Merritt_2008.pdf", "ChatGPT package (arXiv)",
     "https://arxiv.org/pdf/0709.3367v2", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Rose & Dullin 2013, CeMDA 117, 169 (arXiv:1302.0912v4)", f"{L}/record_only/Rose_Dullin_2013.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/1302.0912v4", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Nauenberg 2001, PLA 292 (arXiv:nlin/0112003v2)", f"{L}/record_only/Nauenberg_2001.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/nlin/0112003v2", "yes", "arXiv assumed-1991-2003"),
    ("Rein & Spiegel 2015, MNRAS 446, 1424 (arXiv:1409.4779v2)", f"{L}/record_only/Rein_Spiegel_2015.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/1409.4779v2", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Dandekar et al. 2022, CeMDA 134, 55 (arXiv:2205.04294v1)", f"{L}/record_only/Dandekar_et_al_2022.pdf",
     "ChatGPT package (arXiv)", "https://arxiv.org/pdf/2205.04294v1", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Johnstone & Rucinski 1991, PASP 103, 359", f"{L}/record_only/Johnstone_Rucinski_1991.pdf",
     "ChatGPT package (IOP)", "https://iopscience.iop.org/article/10.1086/132828/pdf", "no (publisher site)",
     "none stated"),
    ("Mel'nikov, Orlov & Shevchenko 2013, Astron. Rep. 57, 429",
     f"{L}/melnikov_2013/Melnikov_Orlov_Shevchenko_2013_Lyapunov_exponents.pdf",
     "owner's copy, local only (the owner's 'research ChatGPT' batch)", "-", "no", "(c) Pleiades"),
    ("Orlov, Titov & Shombina 2016, Astron. Rep. 60, 1083", f"{LO}/orlov_titov_shombina_2016/S1063772916110056.pdf",
     "owner's copy, local only, (c) Pleiades", "-", "no", "(c) Pleiades"),
    ("Titov 2024, DSc dissertation, SPbU", f"{LO}/titov_2024/disser_titov.pdf", "owner's copy (public URL)",
     "https://disser.spbu.ru/files/2024/disser_titov.pdf", "yes", "none stated"),
    ("Li & Liao 2018, arXiv:1805.07980v1 (New Astronomy 70, 22)", f"{LO}/li_liao_2018/1805.07980v1.pdf",
     "owner's copy (arXiv)", "https://arxiv.org/pdf/1805.07980v1", "yes", "arXiv nonexclusive-distrib 1.0"),
    ("Tanikawa, Umehara & Abe 1995, CeMDA 62, 335",
     f"{L}/springer/Tanikawa_Umehara_Abe_1995_Collision_orbits_I.pdf", "owner's copy (Springer batch)", "-", "no",
     "(c) Kluwer"),
    ("Anosova 1986, Ap&SS 124, 217", f"{L}/springer/Anosova_1986_Dynamical_evolution_of_triple_systems.pdf",
     "owner's copy (Springer batch)", "-", "no", "(c) Reidel"),
    ("Umehara & Tanikawa 2000, CeMDA 76, 187",
     f"{L}/springer/Umehara_Tanikawa_2000_Binary_and_triple_collisions.pdf", "owner's copy (Springer batch)", "-",
     "no", "(c) Kluwer"),
    ("Saito & Tanikawa 2007, CeMDA 98, 95", f"{L}/springer/Saito_Tanikawa_2007_Rectilinear_symbol_sequence_I.pdf",
     "owner's copy (Springer batch)", "-", "no", "(c) Springer"),
    ("Saito & Tanikawa 2009, CeMDA 103, 191",
     f"{L}/springer/Saito_Tanikawa_2009_Rectilinear_symbol_sequence_II.pdf", "owner's copy (Springer batch)", "-",
     "no", "(c) Springer"),
]
PETER = ["CITATION.cff", "LICENSE.txt", "README.md", "butterfly_I_trajectory.json", "checksums.txt",
         "figure8_trajectory.json", "methods.md", "moth_I_trajectory.json", "three_body_trust_catalog_v1_fast.ipynb",
         "three_body_trust_catalog_v1_full.ipynb", "three_body_trust_catalog_v1_summary.json", "verification.py",
         "yinyang_I_branch2a_trajectory.json"]


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main(inventory, out):
    inv = {}
    with open(inventory, encoding="utf-8") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            if r["package"].startswith("zip:planar_three_body_research_complete") and "Peter_2025/" in r["path"] \
                    and "previous_verified_bundle" not in r["path"]:
                inv[r["path"].split("/")[-1]] = (r["size_bytes"], r["sha256"])
    with open(out, "w", encoding="utf-8") as f:
        f.write("source\tpath\tsize_bytes\tsha256\torigin\turl\tfetchable\tlicence\n")
        for src, path, origin, url, fetchable, lic in ROWS:
            f.write("\t".join([src, path, str(os.path.getsize(path)), sha256(path), origin, url, fetchable, lic]) + "\n")
        for name in PETER:
            size, digest = inv[name]
            f.write("\t".join(["Peter 2025, Zenodo 10.5281/zenodo.17635887", f"{L}/fetched/peter_2025/{name}", size,
                               digest, "ChatGPT package (Zenodo); not kept, fetch.sh retrieves it", ZP + name, "yes",
                               "MIT"]) + "\n")
    print(out, len(ROWS) + len(PETER), "rows")


if __name__ == "__main__":
    main(*sys.argv[1:3])
