# The owner's fetch list of 31 Oct 2026: what was checked, fetched and held

A record, not authority (R-1, R-159). Proposals only: nothing here is a target until the owner rules, and no table is
transcribed (the owner's triage: transcription waits for approved items).

The list came from an unverified literature table made by another model (`orch/r630/gemini-literature-table-31oct-UNVERIFIED.txt`,
not in the repo), used as leads only. Every citation below was checked at Crossref, on the arXiv abstract page or on the
paper's first page, on 31 Oct 2026.

Wording, the owner's: data we hold is "already have"; "rejected" is only for what was actually ruled out.

## The sources

| Source (as verified) | Licence, quoted from the source | Where it is | Note |
|---|---|---|---|
| Ginat & Perets 2021, *Analytical, Statistical Approximate Solution of Dissipative and Nondissipative Binary-Single Stellar Encounters*, Phys. Rev. X 11, 031020, doi:10.1103/PhysRevX.11.031020, arXiv:2011.00010 | The PDF's p. 1: "Published by the American Physical Society under the terms of the Creative Commons Attribution 4.0 International license." | **Committed**, `papers/ginat_perets_2021_PRX_11_031020.pdf` | The publisher's version; its first page reads "corrected 28 April 2022" |
| Boekholt & Portegies Zwart 2015, *On the reliability of N-body simulations*, Comput. Astrophys. Cosmol. 2, 2, doi:10.1186/s40668-014-0005-3, arXiv:1411.6671 | Crossref's record: `creativecommons.org/licenses/by/4.0`. arXiv: non-exclusive distribution licence 1.0 | Already have the arXiv version, `../2026-10-25_replication/papers/boekholt_pz_2015_ComAC_2_2_arXiv1411.6671.pdf` | The publisher's PDF was not obtained: the site served a bot check, which was not worked round. The owner can fetch it; CC BY allows committing it. *31 Oct 2026, later (R-661):* the owner's copy is now committed, `../2026-10-31_precision_set/papers/boekholt_pz_2015_ComAC_2_2_version_of_record.pdf` |
| Parischewsky, Trani & Leigh 2023, *Chaos in the vicinity of a singularity in the three-body problem: The equilateral triangle experiment*, SciPost Phys. Core 6, 016, doi:10.21468/SciPostPhysCore.6.1.016, arXiv:2108.06335 | Crossref's record: `creativecommons.org/licenses/by/4.0`. arXiv: non-exclusive distribution licence 1.0 | Already have the arXiv version, `../2026-10-25_replication/papers/parischewsky_trani_leigh_2023_SciPostCore_6_016_arXiv2108.06335.pdf` | The publisher's PDF was not obtained: an Anubis bot check, not worked round. The owner can fetch it; CC BY allows committing it |
| Mikkola & Tanikawa 2007, *Correlation of macroscopic instability and Lyapunov times in the general three-body problem*, MNRAS 379, L21, doi:10.1111/j.1745-3933.2007.00326.x, arXiv:0704.2671 | R-643's audit table: arXiv "assumed 1991–2003" | Already have, `../2026-10-25_replication/papers/mikkola_tanikawa_2007_MNRAS_379_L21_arXiv0704.2671.pdf` | Nothing fetched |
| Tanikawa & Mikkola 2008, *A trial symbolic dynamics of the planar three-body problem*, arXiv:0802.2465 | arXiv page: CC BY-NC-SA 3.0 | Already have, `../2026-10-25_replication/papers/tanikawa_mikkola_2008_arXiv0802.2465.pdf` | **"PASJ 60, 883" does not verify:** RQ-645 |
| Portegies Zwart & Boekholt 2014, *On the minimal accuracy required for simulating self-gravitating systems by means of direct N-body methods*, ApJL 785, L3, doi:10.1088/2041-8205/785/1/L3, arXiv:1402.6713 | Crossref lists IOP's copyright and text-mining pages, no open licence. arXiv: non-exclusive distribution licence 1.0 | `_local`, arXiv v1 | |
| Kol 2021, *Flux-based statistical prediction of three-body outcomes*, CeMDA 133, 17, doi:10.1007/s10569-021-10015-x, arXiv:2002.11496 | Crossref lists Springer's text-mining licence only. arXiv: non-exclusive distribution licence 1.0 | `_local`, arXiv v5 | Springer's own page redirected to a sign-in step and was not read, so Crossref is the only publisher-side check |
| Urminsky 2010, *Shadowing unstable orbits of the Sitnikov elliptic three-body problem*, MNRAS 407, 804–811, doi:10.1111/j.1365-2966.2010.16974.x, arXiv:1005.1225 | Crossref lists none. arXiv: non-exclusive distribution licence 1.0 | `_local`, arXiv v1 | The table's DOI, `…16973.x`, is another paper (Maraston et al., MNRAS 407, 830). The title says this is the Sitnikov problem; whether it counts as the restricted problem for triage is unsettled here |
| Broucke 1975, *On relative periodic solutions of the planar general three-body problem*, Celest. Mech. 12, 439–462, doi:10.1007/BF01595390 | The scan's p. 1: "All Rights Reserved / Copyright © 1975 by D. Reidel Publishing Company" | `_local`, ADS scan, for citation only | The table's DOI, `BF01228563`, is Hadjidemetriou 1975 (Celest. Mech. 12, 255) |
| Hénon 1976, *A family of periodic solutions of the planar three-body problem, and their stability*, Celest. Mech. 13, 267–285, doi:10.1007/BF01228647 | The scan's p. 1: "All Rights Reserved / Copyright © 1976 by D. Reidel Publishing Company" | `_local`, ADS scan, for citation only | |
| "Barrio et al. 2009, PRE 79, 036208" | — | **Not obtained: the citation does not verify** | RQ-645: that DOI is Barré & Yamaguchi, on Hamiltonian mean-field clusters |

`_local` is `docs/incoming/_local/fetch_list_2026-10-31/`, ignored by `.gitignore` line 21. `sources.tsv` gives each file's
hash and URL.

## Boekholt & Portegies Zwart 2015 against our never-discard rule

The paper's cut, from the arXiv version already in the repo:

- Abstract: "the error due to divergence is unbiased, as long as the time-step parameter, η ≤ 2⁻⁵ and when simulations
  which violate energy conservation by more than 10% are excluded."
- § 6.1: "If we filter out simulations with a relative energy conservation |∆E/E| > 1, or |∆E/E| > 0.1, we observe that
  the bias in the average error of the semimajor axis of the binaries vanishes. We conclude that this bias is caused by a
  few simulations which grossly violate energy conservation."
- Fig. 14's caption: "In the left column we show the results for the total ensemble of solutions, in the middle column
  for a maximum level of unity and in the right column for 10⁻¹. The bias in the left column for the binary semimajor
  axis is caused by solutions which grossly violate energy conservation. Note that this only happens for the cold
  collapse simulations. When these outliers are taken out of the ensemble, the bias vanishes."
- § 6.3, with no cut stated: "for a time-step parameter η < 2⁻⁵, the distributions are statistically indistinguishable,
  even though at least half of the solutions diverged from the converged solution. If however, energy conservation is
  grossly violated, biases are introduced".

Our rule, `docs/design/principia_honesty_precision.md` line 278: "**Never discard:** a failed or budget-exhausted run is
shown as such, with its end time (R-563)."

**The claim conflicts; the side-by-side does not.** The paper's unbiased result is stated with those runs removed, so as
a claim it is the opposite of never discarding. The paper also shows the whole ensemble beside the cuts (Fig. 14, left
column), so the bias is reported, not hidden, and that method is compatible, once the count removed is given. Nothing in
Principia depends on the cut. BZ-1 is reopened on the cold sets, which are planar (R-661; `../2026-10-27_validation_plan/PLAN.md`
§ 3.2).

What the conflict touches is two lines of our record, which read the paper as agreeing with the rule:
`../2026-10-25_replication/catalogue.md` § 6.1 ("A direct analogue of the project's "never discard a copy" and
`error_ratio` rules", beside its own "runs with |dE/E| > 0.1 are rejected") and PLAN.md's BZ-1 ("Its method is what
REQ-VAL-199 and "never discard a copy" already apply"). The paper's analogue is its ensemble comparison, not its energy
cut. RQ-644 asked how to word this, and R-661 closes it: those lines are reworded.

## Gaps added

PLAN.md § 7 "Gaps" gains items 9 and 10, in the owner's words: no per-point FTLE data, and no f32 N = 3 statistical
benchmark. That list is the one the owner means; no other gaps list was found for validation (`LR` § 15 is its source).
Each is stated as that list states everything: not found in these records, a hypothesis until searched further.

## Not done, by the owner's correction

The owner's later correction takes Hristov, Simó and Broucke/Hénon data off the fetch list, because the atlas is already
held, and gives the long-period and L ≠ 0 check to a separate atlas triage. So this record proposes no closure or
relative-periodic target, and no orbit data was fetched from threebodyorbits.com.
