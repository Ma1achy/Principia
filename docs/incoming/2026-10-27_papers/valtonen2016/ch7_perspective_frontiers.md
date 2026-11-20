# Chapter 7, "Three Body Problem in Perspective": history, regularisation, numerical studies, hierarchical triples, new frontiers

Valtonen et al. 2016, book pp. 131–145 (PDF pp. 142–156; PDF = book + 11 here). Our words; quotations marked and
paged. Chapter 7 contains **no Pythagorean or free-fall initial conditions**; its free-fall material is the symbolic
partition (p. 141). Fig. 7.2 (the "line dance") is the only three-body plot, and it has no axes.

---

## 1. The problem and its history (pp. 131–137)

- Statement (p. 131): three point masses under Newtonian gravity; given initial positions and velocities, predict
  them at any time, past or future. Euler and Lagrange solved two special cases: bodies on a rotating straight line and
  at the vertices of a rotating equilateral triangle (p. 131; dated 1763 and 1772 in Chapter 1, p. 4).
- Laplace's demon and first-order stability of the Solar System (p. 133); the lunar "secular acceleration" resolved
  only in 1939, once the slowing of the Earth's rotation was recognised (p. 134).
- **King Oscar's question** (p. 134), quoted in full in the book: series in a known function of time, converging
  uniformly, under the assumption that no two points collide. Poincaré worked on the restricted problem (p. 135);
  his corrected memoir "provided the foundation for the modern theory of chaos" (p. 135).
- **Birkhoff (1931)**: the system "forgets" its initial state, so the general problem can be solved only
  statistically (p. 135).
- **Sundman's series** (p. 136): published 1913. Belorizky (**1930**): "to calculate one orbital period in the
  Sun—Jupiter—Saturn three-body problem with reasonable accuracy requires 10 to the power 80,000 terms" (p. 136).
  - **Internal inconsistency.** Chapter 1 (p. 6) attaches the same 10⁸⁰,⁰⁰⁰ to "Lagrange's triangular three body
    problem … just for one sixth of the period". Chapter 1 also dates Sundman's result to 1912 (p. 6), Chapter 7 to
    1913 (p. 136); Chapter 3 explains the issue was printed in 1912 (p. 71). Belorizky's paper is not in the repo.
- Bruns and Poincaré: non-integrability (p. 137). **Xia (1992)** extended Sundman's solution to the n-body problem
  (p. 137). Aarseth's thesis (Cambridge, **1963**) and his openly shared codes (p. 137).

## 2. Regularisation after KS (pp. 137–138)

- "The three-body regularization of Kustaanheimo and Stiefel has to be extended to a larger number of bodies. In
  1974 Heggie showed first how this could be done for the four-body problem" (p. 137).
  - **CONFLICT.** Heggie 1974 (in the repo, Celest. Mech. 10, 217) is a *global regularisation of the three-body
    problem*, extending Aarseth & Zare (1974; also in the repo), then generalised to perturbed three-body motion and
    to the N-body problem (his abstract, p. 217). Its subject is not the four-body problem, and Aarseth & Zare's
    three-body KS regularisation, which preceded it, is not mentioned in the book.
- Mikkola began four-body PhD work and visited Aarseth in Cambridge in **1981**; Princeton **1987** (Heggie's
  regularisation in Hut's code) (pp. 137–138).
- **Chain regularisation**, invented by Mikkola and Aarseth in Cambridge in **1989**: link the closest pair, then the
  body closest to either, and so on (p. 138). (Published as Mikkola & Aarseth 1990, CeMDA 47, 375,
  doi:10.1007/BF00051012, Crossref; not in the repo.)
- **Algorithmic regularisation**, Mikkola and Tanikawa (KT), Tokyo, **1999**: bypasses the four-dimensional KS space
  (p. 138). (Mikkola & Tanikawa 1999, MNRAS 310, 745, doi:10.1046/j.1365-8711.1999.02982.x, and CeMDA 74, 287,
  doi:10.1023/A:1008368322547, Crossref; not in the repo.) The book does not mention Preto & Tremaine 1999 (in the
  repo), who note in their introduction that they learned of Mikkola & Tanikawa's paper as their own neared
  completion — an omission, not a conflict.
- **Chazy (1922)** classified the final states, including escape (p. 138). Triple collision studied by Saari,
  McGehee and Tanikawa; its probability is zero "among any reasonable range of initial states", but in physical
  problems (three black holes, three galaxies) triple collisions "occur readily" (pp. 138–139).

## 3. Numerical studies (p. 139)

- Restricted problem: Copenhagen (Strömgren) and Japanese (Matukuma) groups before computers; Hénon's search for
  periodic orbits after them.
- Heggie (1974) derived binary–single-star statistics assuming isotropic arrival; tested by **Heggie and MV in 1979**
  and by Hut and others in the mid-1980s.
- The same **25-body problem** was solved by **nine** groups in **1970**, with different escapers and escape times;
  a later comparison used **32** bodies (p. 139).

## 4. Hierarchical triples (pp. 139–140)

- Stability can mean no escape, no exchange, or orbital changes below a limit; the required duration matters
  (p. 140).
- **Valtonen & Karttunen** (MV with Hannu Karttunen): for a circular inner binary of orbital radius 1, with the third
  body coplanar on an **eccentric** orbit, the third body should not come closer than **4.8** units for prograde
  motion, and **2.6** for retrograde (p. 140). **Masses and the outer eccentricity are not given.** The primary source
  is Valtonen & Karttunen, *The Three-Body Problem*, Cambridge University Press (2006), doi:10.1017/CBO9780511616006
  (Crossref; not in the repo); the later quantitative criterion of the same group is Mylläri, Valtonen, Pasechnik &
  Mikkola 2018, MNRAS 476, 830, doi:10.1093/mnras/sty237 (Crossref; not in the repo).
- Compare the other hierarchical numbers in the book: Harrington's **3.5** (doubled for safety), Bohlin's **4.7**
  major axes (Chapter 3, pp. 82–84); Innanen's **70 %** (prograde) and **144 %** (retrograde) of the Hill radius
  (Chapter 5, p. 114).

## 5. New frontiers (pp. 140–142)

- **Symbolic dynamics** (pp. 140–141): Alexeyev, Sitnikov problem, **1969**; Mikkola and KT, the rectilinear problem,
  **2000** (bodies on a line with elastic bounces) (p. 140). (Crossref has Tanikawa & Mikkola 2000, *Chaos* 10, 649,
  doi:10.1063/1.1287064, "One-dimensional three-body problem via symbolic dynamics"; Tanikawa & Mikkola 2008, in the
  repo, cites the 2000 CeMDA 76, 23 paper.)
- **Free-fall symbolic sequence** (p. 141): Chernin and colleagues classify the triangle's shape as L ("nearly
  Lagrangian triangle, with three almost equal sides"), H ("one side is much shorter than the other two"), A
  ("the bodies lie almost on a straight line") or M ("none of the other categories apply"). "A new symbol is added to
  the sequence when the three-body shape enters a new configuration"; e.g. MHMHAMLH…; the sequence does not repeat
  (p. 141). **No quantitative thresholds.** For the boundaries as drawn in Fig. 4.1, see
  `ch4_maps_entropy_fractals_symbolic.md` §1.
  - Contrast: Tanikawa & Mikkola 2008 and 2015 (both in the repo, KT being a book author) define free-fall symbols by
    **syzygy crossings** — which body is in the middle and the sign change of the oriented area — not by L/M/A/H
    regions. Two different symbolic dynamics; not a conflict.
- **Figure-eight** (p. 141): Moore **1993** (PRL 70, 3675, doi:10.1103/PhysRevLett.70.3675); "choreographic" is
  Simó's term; studied by Chenciner & Montgomery (Ann. Math. 152, 881 (2000), doi:10.2307/2661357); Fujiwara for
  other force laws. Chapter 1 adds that JA and VO "came close to the discovery … a decade before Moore" (p. 6,
  footnote 7).
- **The "line dance"** (p. 141; Fig. 7.2, p. 142): "discovered by Michel Hénon in 1977 and it was confirmed by …
  Daniel Benest and Monique Fulconis in 1982 by numerical investigations of 2700 triples"; JA and VO found it
  independently. The central body pirouettes with each end body in turn in a near-linear formation.
  - Pointer: Hénon 1977, "Stability of interplay motions", Celest. Mech. 15, 243, doi:10.1007/BF01228465 (Crossref).
    Martynova, Orlov & Rubinov 2009 (in the repo) cite Hénon 1977 as a linear stability analysis and attribute the
    underlying collinear periodic orbit to **Schubart (1956)**, AN 283, 17 (their refs. 11 and 15). The book does not
    mention Schubart. Benest & Fulconis 1982 was not found on Crossref.
- "JA and VO have also discovered more rather stable dance formations in triple systems since 1984"; such long-lived
  formations appear "about once in a thousand cases"; in all of them the bodies stay well apart (p. 141). **No
  set-up is given.**
- Gaia: about a billion stars, **70** measurements per star over a **5**-year mission, at Sun–Earth L2, **1.5 million
  km** from the Earth (Fig. 7.3 caption, p. 142).

## 6. Planets, stars and black holes (pp. 143–145)

- ~**2000** known exoplanets (p. 143); Kepler 19b/19c by transit timing (p. 143).
- **CH Cygni**: inner period about **2** years, outer **14** years; explained by Mikkola and KT via a Lidov–Kozai
  cycle driving the inner binary into contact (p. 143).
- **Final parsec problem**: Begelman, Blandford & Rees (**1980**); binaries stall near **1 pc**; OJ287's binary is
  **0.05 pc** (p. 144). Mikkola & MV (**1992**), three-body and n = **10,000** runs: black holes of **10⁶ M☉** each
  merge in about **100 million years**, larger ones do not (pp. 144–145). Milosavljević & Merritt, n = **260,000**: the
  binary stalls at about **twice** the size needed to merge within a galaxy's age (p. 145). Iwasawa et al. (**2011**):
  unequal masses give high eccentricity (Lidov–Kozai), then gravitational-wave inspiral (p. 145).

---

## Replication hooks

1. **The L/M/A/H symbolic sequence** on free-fall trajectories, with the digitised Fig. 4.1 boundaries as one
   explicit choice of thresholds, against the syzygy symbols of Tanikawa & Mikkola (in the repo). → **V-7** (new),
   **V-3**.
2. **Figure-eight and the Schubart/Hénon "line dance"** as stable-orbit landmarks in the `Latent` chart: neither is
   given by the book with initial conditions; Martynova et al. 2009 (in the repo) give their section's coordinates
   (e.g. Schubart orbit k = 0.206, φ = 0°, their p. 713). → pointer under **V-5**; existing orbit-catalogue targets.
3. **Hierarchical stability limits** 4.8 (prograde) / 2.6 (retrograde) for a circular inner binary. → **V-11** (new).
4. **"About once in a thousand"** long-lived formations: a frequency to compare with a long-lifetime tail on any
   sampled surface, set-up unknown. → **V-5**, **V-9**.
