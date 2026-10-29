# Install and build record: Brutus, run here

Everything below happened on a disposable Linux machine (Ubuntu 24.04.5, Intel Xeon @ 2.80 GHz, 4 cores, 15 GB). Nothing was
installed on the owner's machines. Machine, apt and compiler versions: `install_record_machine.txt`.

## Source (pinned; not AMUSE's copy)

| | |
|---|---|
| Repository | https://github.com/pgbreen/NvMgeneration |
| Commit built | `98906f472f66b127bde62f771410b4974d116a3b` ("Update README.md", 16 Jan 2020, `master` HEAD) |
| Its README | "Brutus: version used in Newton vs the Machine https://arxiv.org/abs/1910.07291. Note: Brutus is availbe as part of AMUSE and please cite https://arxiv.org/abs/1411.6671" |
| Licence | `license.txt`, sha256 `b6df573e85e99fb67895db18829d46292ff7061ba38fa42cda0fbc991379bc52`: "GNU GENERAL PUBLIC LICENSE / Version 3, 29 June 2007". `copying.txt` (sha256 `4e3366a0…aa060`) is also the GPL v3 text. |
| Bundled `mpreal.h` | v3.6.2, "Copyright (c) 2008-2015 Pavel Holoborodko … (A) MPFR C++ is under GNU General Public License ("GPL")" (it also offers a paid non-free licence). sha256 `8b07725e…117e1` |
| Files hashed | `Brutus.cpp d8cc6edf…`, `Bulirsch_Stoer.cpp ed6d7951…`, `Cluster.cpp 1659de40…`, `Star.cpp c77bec15…`, `main.cpp b0fe21e3…`, `makefile 20be5ece…` |

Not vendored here: `build.sh` clones the pinned commit into a scratch directory, checks `HEAD` and a clean tree, and builds. Only our
driver `src/pilot.cpp` is committed. It is built against GPL-3.0 code; whether that makes a distributed binary GPL is not our call here,
and no binary is committed.

## Installed

`apt-get install libgmp-dev libmpfr-dev g++` installed (new today) `libgmp-dev 2:6.3.0+dfsg-2ubuntu6.1`, `libgmpxx4ldbl 2:6.3.0+dfsg-2ubuntu6.1`,
`libmpfr-dev 4.2.1-1build1.1`. Already on the image: `g++ 13.3.0-6ubuntu2~24.04.1`, `make 4.3`, `libmpfr6 4.2.1`, GNU ld 2.42.
Not installed because the standalone code does not need them: MPI, AMUSE, a Python venv. Analysis scripts use the system `python3` (3.13.17, numpy 2.5.3).

## Build, and the two deviations from the makefile as shipped

`make main.exe` fails on this toolchain with the makefile as shipped. Both deviations go through `CXXFLAGS`; no source file is edited
(`git status` on the pinned tree is clean; `build.sh` checks it).

1. `-DMPFR_USE_NO_MACRO`: the bundled `mpreal.h` 3.6.2 does not compile against MPFR 4.2's function-like macros
   (`mpreal.h:582: no matching function for call to 'mpfr::mpreal::mpfr_srcptr(mpfr_srcptr)'`). `-fpermissive` does not help.
2. `-Wl,--no-as-needed`: the makefile puts `-lmpfr -lgmp` before the object files on the link line; Ubuntu's linker then drops them
   (`undefined reference to 'mpfr_clear'`).

Flags otherwise as the makefile: `g++ -O1 -std=gnu++11`. (`-w` silences the deprecation warnings in `mpreal.h`.) The final link of the
stock `main.exe` is in `build_stock.log`.

Our driver is compiled the same way and linked to the unmodified `Star.o Cluster.o Bulirsch_Stoer.o Brutus.o`:
`g++ -O1 -std=gnu++11 -DMPFR_USE_NO_MACRO -Wl,--no-as-needed -w -I. -o pilot src/pilot.cpp Brutus.o Bulirsch_Stoer.o Cluster.o Star.o -lmpfr -lgmp`.

## Why our own driver

**Unverified.** The source files are not committed, so the statements in this section about what `main.cpp` and `Brutus.cpp` contain rest on
the pilot's own reading of them, and stay unverified until the pinned source is read again (`REPORT.md`, "Not verified from committed
material"). As recorded: the pinned `main.cpp` **never applies the word length**. It reads `numBits`, echoes it in its header line, and passes it to `Brutus`,
which stores it; nothing calls `mpreal::set_default_prec`. The stock executable therefore runs at MPFR's default 53 bits whatever
L_w is. Evidence: `stock_lw_test.txt` (identical output for L_w = 20, 53, 128, 1024; 16 significant digits only). It also has no escape
test, step count or wall cap. `src/pilot.cpp` sets `mpreal::set_default_prec(L_w)` first and, as recorded (unverified), copies the six-line `Brutus::evolve` step loop.
All "L_w = 128 / 1024 bits" in this folder come from that call.

## Binaries used (sha256 in `binaries.sha256`)

| Build dir | Used for | Differences |
|---|---|---|
| `work` | the GG-1 sample at eps = 1e-6, L_w = 1024 (`gg1/pilot_eps1e-6_Lw1024.csv`) | first driver |
| `work2` | BR-1 runs, GG-1 scaling probes, p_max probe | adds a BR-1 wall cap |
| `work3` | forward-only re-run of the sample, T-definition check, robustness check | adds `T_hist`, `norev`. **= the committed `src/pilot.cpp`** (sha256 `a5b187ac…71237`) |

`gg1/pilot_fwdonly_final_driver_eps1e-6_Lw1024.csv` re-ran the 16 points forward-only with the final driver: steps, T and status are
identical on all 16 (determinism across builds).
