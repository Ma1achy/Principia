# Orbit catalogues: an addition to the replication suite, 26 Oct 2026

A record, not authority (R-1, R-159). The instruction quotes the human's words. It was drafted with Claude at his
request and adds to `../2026-10-25_replication/`, replacing nothing in it.

| Path | sha256 | What it is |
|---|---|---|
| `INSTRUCTION_orbit_catalogues_2026-10-26.md` | `5c7ed8588488856ce1ba5d019715f7986c2cd31829148c8aa272d888ad144039` | The instruction: the two sources, their status, and what to test with them |
| `threebodyorbits_catalogue.json` | `597c66bd937764825ccaa38da5372d2c71fc6490ab9b09a51902339e5e3c0eb0` | `https://data.threebodyorbits.com/catalogue.json`, downloaded 26 Oct 2026, unmodified. Generated 27 Sep 2026 (stamp 1790531492), 3,943 orbits |
| `hudomal_2015_msc_belgrade.pdf` | `b30dbad23f114b646721e65110fcffe265214765c2cdf2ac21b0ecbb7b2c1fb0` | Hudomal, *New Periodic Solutions to the Three-Body Problem and Gravitational Waves*, MSc thesis, University of Belgrade, 2015, from `https://www.scl.rs/theses/msc_ahudomal.pdf`, unmodified: the Belgrade group's tables of initial conditions, periods and free-group words |
| `extract_ics.py` | `cbc3d2a840f34c52016c5084215d78dbac8d49a561115491431ce81697a5eb35` | The extraction, with the file layout it reads and how the initial-condition order was confirmed |

The per-orbit files (`https://data.threebodyorbits.com/orbits/<key>.bin`, 157 MB) are not committed. `threebodyorbits_catalogue.json`
is used under CC BY 4.0, attributed to https://www.threebodyorbits.com, by the operator's permission of 26 Oct 2026; see
the instruction's "Data terms". The Belgrade gallery's archived pages could not be fetched
(the Internet Archive is blocked from the session, and the site returns HTTP 500); Hudomal's thesis stands in for
them.

**Only `threebodyorbits_catalogue.json` is committed from the site**, the one file the operator's grant names (the
human's call, 26 Oct 2026: "guess maybe just use the ones explicitly been given permission to"). The extracted
initial conditions (`threebodyorbits_ics.csv`, sha256 `52a67e5db73261b002f4985d16c6aab9b36d20ec4b4a6f79c2d991b71b42c8e9`) and
the operator's `atlas-params.json` (sha256 `a5378e2b1c099a02bce36f85b12eec6449623fdf4ebd18bbed15de0135413296`) are
held outside the repo until the grant is confirmed to cover them. `extract_ics.py` regenerates the CSV from the site's
per-orbit files, byte for byte.
