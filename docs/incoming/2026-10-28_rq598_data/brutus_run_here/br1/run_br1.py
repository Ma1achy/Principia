#!/usr/bin/env python3
"""Run the BR-1 pilot sample through our driver, one run at a time ("Brutus, run here").

For each run in sample_12.csv: x1 = (1, 0), x2 = the authors' printed row-1 values (read as doubles, as the stock main
does), x3 from the barycentre, equal masses at rest, outputs every 2^-8 to the authors' last row (t_end_to_run).
x2 is read at run time from the local, gitignored copy of the authors' files (<nvm local dir>/<set>_data/<name>.dat,
row 1, columns 3 and 4). It is passed to the driver and never written out: the driver's x2 fields are dropped from the
results CSV, and the trajectory files (whose row 1 is x2) are written to <traj dir>, which must lie outside the repo or
under a gitignored path.
usage: run_br1.py <pilot binary> <nvm local dir> <sample_12.csv> <results.csv> <traj dir> <cap_s> <eps,Lw> [<eps,Lw> ...]
"""
import csv
import os
import subprocess
import sys

pilot, nvm, sample, results, trajdir, cap = sys.argv[1:7]
configs = [c.split(",") for c in sys.argv[7:]]
os.makedirs(trajdir, exist_ok=True)
rows = list(csv.DictReader(open(sample)))
new = not os.path.exists(results)
with open(results, "a", newline="") as out:
    if new:
        out.write("run,set,stratum,t_end,kind,eps,Lw,status,t_reached,steps,wall_s,cpu_s,energy_relerr,min_pair_sep\n")
        out.flush()
    for eps, lw in configs:
        for r in rows:
            if lw == "88" and r["authors_rows"] != "2561":
                continue  # the Lw = 88 timing is taken on the full-length runs only
            traj = os.path.join(trajdir, f"{r['name']}_eps{eps}_Lw{lw}.dat")
            with open(os.path.join(nvm, r["set"] + "_data", r["name"] + ".dat")) as fh:
                x2x, x2y = fh.readline().split()[2:4]
            p = subprocess.run([pilot, "br1", x2x, x2y, eps, lw, r["t_end_to_run"],
                                traj, "4", cap], capture_output=True, text=True)
            line = p.stdout.strip().splitlines()[-1] if p.stdout.strip() else f"br1,,,{eps},{lw},crash,,,,,,"
            f = line.split(",")
            line = ",".join([f[0]] + f[3:])  # drop the driver's x2x, x2y fields: never written out
            out.write(f"{r['name']},{r['set']},\"{r['stratum']}\",{r['t_end_to_run']},{line}\n")
            out.flush()
