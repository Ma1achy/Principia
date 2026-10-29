#!/usr/bin/env python3
"""Run the BR-1 pilot sample through our driver, one run at a time ("Brutus, run here").

For each run in sample_12.csv: x1 = (1, 0), x2 = the authors' printed row-1 values (read as doubles, as the stock main
does), x3 from the barycentre, equal masses at rest, outputs every 2^-8 to the authors' last row (t_end_to_run).
usage: run_br1.py <pilot binary> <sample_12.csv> <results.csv> <traj dir> <cap_s> <eps,Lw> [<eps,Lw> ...]
"""
import csv
import os
import subprocess
import sys

pilot, sample, results, trajdir, cap = sys.argv[1:6]
configs = [c.split(",") for c in sys.argv[6:]]
os.makedirs(trajdir, exist_ok=True)
rows = list(csv.DictReader(open(sample)))
new = not os.path.exists(results)
with open(results, "a", newline="") as out:
    if new:
        out.write("run,set,stratum,t_end,kind,x2x,x2y,eps,Lw,status,t_reached,steps,wall_s,cpu_s,energy_relerr,min_pair_sep\n")
        out.flush()
    for eps, lw in configs:
        for r in rows:
            if lw == "88" and r["authors_rows"] != "2561":
                continue  # the Lw = 88 timing is taken on the full-length runs only
            traj = os.path.join(trajdir, f"{r['name']}_eps{eps}_Lw{lw}.dat")
            p = subprocess.run([pilot, "br1", r["x2x_as_printed"], r["x2y_as_printed"], eps, lw, r["t_end_to_run"],
                                traj, "4", cap], capture_output=True, text=True)
            line = p.stdout.strip().splitlines()[-1] if p.stdout.strip() else f"br1,,,{eps},{lw},crash,,,,,,"
            out.write(f"{r['name']},{r['set']},\"{r['stratum']}\",{r['t_end_to_run']},{line}\n")
            out.flush()
