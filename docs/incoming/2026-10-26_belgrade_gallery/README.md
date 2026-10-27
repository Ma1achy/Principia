# The Belgrade three-body gallery, recovered from the Internet Archive, 26 Oct 2026

A record, not authority (R-1, R-159). It is the second source named in `../2026-10-26_orbit_catalogues/` (on its own
branch, not yet merged): Šuvakov and Dmitrašinović's gallery at `http://three-body.ipb.ac.rs/`, now offline, with
initial conditions, periods and **free-group words** for the Belgrade orbits. The human recovered it from the Internet
Archive on his own machine, because the archive is blocked from the session, and asked for it on its own branch.

**Terms.** The gallery's own "Use and Copyright" page (`archive/ref.php.html`): "You may copy and use any images or
data from this web gallery. Please, credit us by citing this page and our paper [8]." Paper [8] is Šuvakov &
Dmitrašinović, *Phys. Rev. Lett.* 110, 114301 (2013), already in `../2026-10-25_replication/papers/`. Every use of this
data cites `http://three-body.ipb.ac.rs/` and that paper.

| Path | sha256 | What it is |
|---|---|---|
| `archive/` | (331 files; `archive/capture-provenance.json` records each one's original URL and Wayback capture) | The recovered site without its 671 trajectory images (657 MB, not committed; they stay on the human's machine). Target snapshot `20260606193512`; older captures fill gaps, so pages may come from different years (`archive/README.txt`). 447 linked URLs had no capture: 402 images and 45 Hénon solution pages (`hsol.php?id=0..44`) (`archive/download-report.json`) |
| `archive/initial-conditions.pdf` | `5b76cfd632e9cdc8f4f43641eabd2164ca1e01fa2b6c29df322c2945be3ade5b` | A. Hudomal, *New Periodic Three-Body Orbits*, Institute of Physics Belgrade, typeset 5 Jan 2019: the initial conditions and periods of all 206 sequence orbits, linked from the gallery |
| `extract_gallery.py` | `2b0aa1f676d5d6ae35beaf40314f18cb450fbc1378d75cf9a79e10046d73d054` | Extracts every solution page into the CSV. Regenerating it reproduces the CSV byte for byte |
| `belgrade_gallery.csv` | `3558b26b1d306a538c98812ff8cdf5ddb7a18c31a13305dac8450b63a839335b` | One row per solution page (301): name, discovery, free-group word, period, energy, angular momentum, initial conditions in whichever form the page gives, the decoded reference, and the page's raw text |
| `crosscheck.py` | `632d21e0d27d3fe7c2972ed4125102f3aaa5a288c1613eedc22849d8cd011e57` | The cross-check against the published sources, below |
| `crosscheck.txt` | (its output) | `python3 -I crosscheck.py belgrade_gallery.csv archive/initial-conditions.pdf <Hudomal 2015 thesis> <Šuvakov & Dmitrašinović 2013 PRL>` |

## What the pages hold

Each solution page carries its data in a JavaScript string typed out by a `type()` function, which substitutes some
characters on display. **The substitution map differs from page to page** (`t` is reference [11] on some pages and
[6-7] on others), so the extractor keeps the raw string and decodes `REF` with each page's own map. A `%` in a name is
such a reference marker, not part of the name.

- **286 solution pages and 15 id-less pages.** Each page kind's id-less page (`sI_sol.php.html` and so on) is the
  page rendered with no id. Fourteen are empty templates (zeros and blanks); `sol.php.html` shows Broucke A7. They
  are in the CSV with an empty `id`.
- **206 sequence orbits** (I, II, III, IVa, IVb, IVc, V, VI, VIIa, VIIb, VIII), zero angular momentum, in the
  collinear convention `x1 = -1, x2 = 1, x3 = 0`, velocities `(p1, p2), (p1, p2), (-2p1, -2p2)`.
- **16 named orbits** on the main gallery page (`sol.php?id=1..16`): the 15 of the 2013 PRL's Table 1 and the
  figure-eight.
- **57 BHH satellites** (`ssol`), with non-zero angular momentum, in relative coordinates (`REL. COORD. a`,
  `REL. VEL. c`), words written with exponents (`a^3`).
- **6 Sheen orbits** (2016), Cartesian initial conditions, and the word field reads `a` on every one: a
  placeholder, not a word.
- **1 Broucke page** (`bsol.php?id=18`, Broucke R 3); the Hénon pages were not archived.

Words use `a`, `b` for the generators and `A`, `B` for their inverses (`c` and `d` are display substitutions for
`a⁻¹`, `b⁻¹` and never occur in a word).

## The cross-check (`crosscheck.txt`)

1. **All 206 sequence orbits agree exactly with Hudomal's 2019 table**, `archive/initial-conditions.pdf`, in `p1`,
   `p2` and `T` to every printed digit. Same 206 labels on both sides.
2. **All 206 words are consistent with their labels**: `n_a = n_b`, `n_A = n_B`, and the label's `n` is
   `min(n_a, n_A)`, the convention of Hudomal's 2015 thesis §4.1.
3. **The 15 orbits of the 2013 PRL's Table 1** agree in `p1`, `p2` and `T` within the PRL's printed precision
   (largest difference 4.1e-5). **Their words agree only up to a symmetry, and which symmetry varies by orbit**:
   - up to cyclic rotation for 9 of the 15 (butterflies I, II and IV, bumblebee, moths II and III, yarn, yin-yang
     II a and b);
   - only under a symmetry for the other 6: the inverse word or the exchange `(a, b) ↔ (A, B)` for moth I,
     butterfly III, goggles and dragonfly, and only the exchange for yin-yang I a and b;
   - **moth III's published word is not freely reduced**: `(babABA)²a(abaBAB)²b` contains `A·a`, 26 letters
     against the gallery's 22. After free and cyclic reduction it matches.
   So the gallery and the PRL write the same conjugacy classes with different representatives. The encounter-word
   reader (R-125) must declare which equivalence it compares under, and a comparison up to rotation alone would fail
   6 of these 15 for a reason that is not an error.
4. **Hudomal's 2015 thesis, Tables 16–22** (38 rows, rescaled to `E = −1/2`) agree with the gallery to the thesis's
   printed precision once the gallery is rescaled the same way, except for **five rows whose labels changed**: thesis
   IVa.6.A, IVa.8.A, IVa.8.B, IVa.8.C and IVb.15.A are gallery IVa.14.C, IVa.8.F, IVa.8.G, IVa.8.H and IVb.15.D (each
   matching to 2e-6 or better). Thesis V.4.A is gallery V.4.A.α. The later labels are the gallery's and the 2019
   table's; the thesis's are earlier.

## One internal discrepancy, recorded

The gallery gives **two periods for butterfly I and butterfly II**: the main page has the PRL's 6.235641 and 7.003907,
the sequence pages (I.2.A, I.2.B) have 6.234671 and 7.003707, with the same `p1`, `p2` and energy. The sequence values
agree with the thesis's scale-invariant `T|E|^{3/2}` (19.9325 and 19.9313, Table 3) and with the Three Body Orbits
atlas's refined periods (6.23489 at E = −2.170144, 7.00370 at E = −2.008195), so the PRL's are the outliers, at about
1e-4. Recorded, not resolved: both values are in the CSV as the pages give them.
