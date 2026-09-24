# LaTeX reference map — step 1

*24 Sep 2026. Every place the current corpus (`outputs/principia_*.md`, excluding `ARCHIVE_*` and the
audit) points at the LaTeX, or looks as if it does. This map is the work order for step 3. Line numbers
refer to the files as committed at step 0.*

## The sources

| source | where | status |
|---|---|---|
| LaTeX spec | `spec_sources/principia_spec_revised.tex` (8 Jul, 3,056 lines) | **retired** (handoff ruling). Port what still holds, then remove it. |
| COM-projection mini spec | `spec_sources/com_projection_mini_spec.pdf` (Mar 2026) | **retired (R-3).** The detail behind `sec:com_projection`, ported with it into the integrator contract. |
| Sphere colour-map spec | `spec_sources/sphere_colour_map_spec.pdf` | **retired (R-3).** Port Eq. 5 and §8 into the files that rely on them (see the work order below). |
| Measured evidence | **prin-rs**: `github.com/Ma1achy/prin-rs`, `FINDINGS.md` and `results/` on `main` @ `8600d45` (8 Sep). Local clone: `~/src/principia-rs-test` | canonical. *(Corrected in step 2: `spec_sources/findings.md` is **not** a prin-rs copy. It is the toolchain spike's findings. See RQ-7.)* |
| Display adequacy | prin-rs `52caf14` (15 Sep), cited by the index | **local only.** It is on the unpushed `lowering-spike` branch of `~/src/principia-rs-test` (9 commits ahead of its remote). See RQ-1. |

**The check for step 3 (R-1, `decisions.md`):** each LaTeX passage is checked against the **current
markdown**, not against prin-rs `FINDINGS.md`. Where they genuinely conflict, add a `REVIEW_QUEUE.md` entry and don't
choose. FINDINGS citations can stay as notes. For reference, FINDINGS covers the regularisation default
(Heggie) in §2, step control and the predictive limit in §3, termination (closure + energy escape) in §4,
and refinement (`Policy::Tolerance`) in §5.

## `.tex` section index (labels → line ranges)

| label | lines | | label | lines |
|---|---|---|---|---|
| `sec:mass_decode` | 199–203 | | `sec:inverse_encode` | 737–770 |
| `sec:hyperspherical_jacobi` | 204–241 | | `sec:chart_validation` | 771–815 |
| `sec:canon_config` | 242–246 | | `sec:named_directions` | 816–827 |
| `sec:jacobi_mom` | 247–255 | | `sec:sim` / `sec:yoshida` | 867–901 / 902–956 |
| `sec:scale_gauge` | 256–260 | | `sec:com_projection` | 957–986 |
| `sec:energy_norm` | 261–267 | | `sec:events` / `sec:escape` | 987–993 / 994–1026 |
| `sec:no_holes_impl` | 268–273 | | `sec:shape_sphere_obs` | 1030–1051 |
| `sec:views` … `sec:mixed_axis` | 274–413 | | `sec:ensemble` | 1135–1172 |
| `sec:LE_view` | 282–327 | | `sec:quality_tiers` | 1959–1980 |
| `sec:shape_sphere_view` | 333–383 | | `sec:linearised_decoder` | 2380–2442 |
| `sec:measure` | 414–430 | | `sec:tile_payload` / `sec:tile_summary` | 2443–2455 / 2456–2607 |
| `part:burrau` (`sec:discrete` … `sec:hypothesis`) | 431–593 | | `par:tile_reduction` | 2608–2711 |
| `sec:burrau_jacobi` | 469–489 | | `sec:split_criteria` / `sec:scheduling` | 2712–2732 / 2733–2784 |
| `sec:burrau_charts` | 530–582 | | `sec:priority` | 2785–2890 |
| `sec:lock` / `sec:inspector` / `sec:lookup` | 658–669 / 670–729 / 730–736 | | `sec:cache` (constants table ~2991) | 2918–3056 |

## Classes

- **T**: a real dependency on the LaTeX. The content has to be ported in step 3.
- **S**: a statement *about* the LaTeX (its authority, or its history). Rewritten under the retirement ruling. Nothing to port.
- **M**: a cross-reference inside the markdown that *resolves*. No action.
- **D**: dangling. The target is neither a markdown heading nor a `.tex` label. Repoint it in step 3.
- **X**: *(retired class)* depended on a PDF. Since R-3 the PDFs are retired like the LaTeX, so these are now **T**.
- **F**: false positive. "spec" here means something else (an animation spec object, the canonical spec, the payload spec).
- **R**: the pending-changes register. Closed in step 4, not step 3.

## The audit's 21 named sections: what they actually are

The audit counts all of these as LaTeX dependencies. **Eleven are** (the six decoder sections,
`com_projection`, `lookup`, `measure`, `ensemble`, Burrau). One (`tile_reduction`) has already been
resolved without the LaTeX. Four dangle and five are markdown-internal:

| audit section | verdict | resolves to |
|---|---|---|
| `mass_decode` `hyperspherical_jacobi` `canon_config` `jacobi_mom` `scale_gauge` `energy_norm` | **T** | the `.tex` labels of the same names |
| `com_projection` | **T** | `sec:com_projection` + mini-spec PDF |
| `lookup` | **T** | `sec:lookup` (and `sec:inverse_encode`, `sec:chart_validation`) |
| `measure` | **T** | `sec:measure` |
| `ensemble` | **T** | `sec:ensemble` |
| `tile_reduction` | **S** | `par:tile_reduction`. `dd_generation_root` §3.7 has already derived the list without it, so what's left is wording. |
| Burrau (1-based indices) | **T** | `sec:burrau_jacobi` |
| `collision` | **D** | "integrator contract §collision". No such heading exists. The nearest `.tex` is `sec:events` (the one-line collision test). |
| `terminal` | **D** | "§terminal-detection". There's no heading anywhere. The content is integrator_contract Part 7. |
| `regularisation` | **D** | "integrator contract §regularisation". No such heading. The candidates are Part 2b or Part 6. |
| `switchover` | **D** | "deep-zoom note §switchover". No such heading. The content is deep_zoom §2, and in the `.tex` it's `sec:linearised_decoder`. |
| `generation` | **M** | "§generation-root ledger" means `principia_dd_generation_root.md` |
| `consolidated` | **M** | each drill-down's own "§2 Consolidated contract". No reference to the LaTeX found. |
| `correctness` | **M** | validation_ground_truth_note, "The correctness factoring" |
| `parity` | **M?** | systems_architecture has no parity section. It means `principia_parity_contract.md`. Repoint for clarity. |

Two cross-checks the drill-downs expect **can't be done against the `.tex`**:
- **Event priority order** (dd_integrator §3.6 / audit B4). `sec:events` lists Collision and then Escape. It has no priority rule to check against.
- **Shape-sphere axis order** (dd_integrator §3.7 / audit B5). The component → axis convention is in the **colour-map PDF §8**, not in the `.tex`. Under R-3 it's ported from the PDF.

## Every reference

Totals: **51 T references (on 43 lines) · 7 S · 5 M · 7 D · 11 F · 22 R**, which is 103 references on 95 lines
across 19 files. Two T lines (dd_integrator 185 and 246) also need the colour PDF ported, which were X before R-3. The audit counts 85.
What's left of step 3 after the rulings: 50 T references to port, plus telemetry:398 to repoint only (R-4).

### principia_dd_decoder.md
| line | class | reference | `.tex` target |
|---|---|---|---|
| 23 | T | "invertible with the spec's closed forms" | 199–260 |
| 33 | T | "Exact, from the spec (§mass_decode, §hyperspherical_jacobi, §canon_config, §jacobi_mom, §scale_gauge)", 5 refs | 199–260 |
| 170 | T | `η_E` "(spec §energy_norm) … specified in the spec, not re-derived here" | 261–267 |

### principia_chart_reference.md
| line | class | reference | target |
|---|---|---|---|
| 3 | T | "Source: … §§ decoder, views, Burrau family", 3 refs | 196–267, 274–413, 431–593 |
| 4 | S | "**Where this and the LaTeX disagree, the LaTeX wins**" | delete (ruling) |
| 16 | T | "The LaTeX uses 1-based body indices in the Burrau section" | 469–489 |
| 83 | T | crossed `m0/m1` momentum factors "as written in the spec" | 247–255 |
| 358 | T | "The spec calls this the *bifurcation strip*" | 570–582 |

### principia_chart_decoder_contract.md
| line | class | reference | target |
|---|---|---|---|
| 47 | T | "Exact hyperspherical formulae live in the spec's Jacobi section" | 204–241 |
| 76 | T | "pre-saturate logits with `μ_max·tanh` (as the spec does)" | 199–203; constant ~2991 |
| 90 | T | "The catch the spec already flags: links carry a measure" | 414–430 |
| 171 | T | "the spec's named compound directions" | 816–827 |
| 194 | T | "the spec's two options" (lock re-centring) | 658–669 |
| 209 | T | "The spec's Euclid plane" | 441–468, 490–495 |

### principia_inverse_encode_contract.md
| line | class | reference | target |
|---|---|---|---|
| 3 | T | "The spec has the block formulae and a validation…" | 737–815 |
| 31 | T | "missing from the spec's policy" | 737–770 |
| 33 | T | "The spec's canonical inverse policy translates, rotates, mirrors" (**the policy content**) | 737–770 |
| 101 | T | "the spec contains two conflicting definitions" | 737–770 vs `sec:LE_view` 282–327 |
| 106 | T | "project / clamp / reject per the spec's ladder" | 771–815 |
| 112 | T | "the spec's case 3" | 737–770 |
| 120 | T | "The spec's five steps" | 730–770 |
| 128 | T | "the spec's three layers" | 771–815 |

### principia_integrator_contract.md
| line | class | reference | target |
|---|---|---|---|
| 3 | T | "The physics … is fully specified in the LaTeX spec and treated as settled" | 867–1026. **The escape criterion is superseded by closure + energy (FINDINGS §4).** |
| 21 | T | `project_com` "(spec §com_projection)" | 957–986 + PDF |
| 34 | T | quotes §com_projection's cadence | 957–986 |
| 42 | T | "spec's tier table gives the *default* binding" | 923–956 (`Integrator selection by quality tier`). **The default is superseded by Heggie (FINDINGS §2.6).** |
| 279 | D | `r_coll` "(§collision: …)" | none. Nearest is 987–993. |
| 330 | T | "The spec is explicit that KS regularisation is **v2**" | ~941–956 |
| 332 | T | "Per the spec's inspector caveat" | ~726–729 |
| 344 | D | "§terminal-detection" | none. Content is Part 7. |
| 345 | D | "the integration floor, §regularisation" | none |
| 348 | D | "§terminal-detection" | none |

### principia_scheduler_contract.md
| line | class | reference | target |
|---|---|---|---|
| 3 | T | "The *policy* … is fully specified in the LaTeX spec and treated as settled" | 2712–3056. **The refinement policy is superseded by `Policy::Tolerance` (FINDINGS §5).** |
| 27 | T | "The spec states it (§measure)" | 414–430 |
| 42 | T | "the spec's single `MAX_DEPTH`" (**its old meaning**) | 2718, 2858–2882 |
| 52 | T | "the spec's 8–14 were written as *absolute*" | 2882 |
| 54 | T | "The spec has: split(C) ⟺ … ℓ < MAX_DEPTH" | 2858 |
| 88 | D | "deep-zoom note §switchover" | none in md. `sec:linearised_decoder` 2380–2442 (`ℓ_switch = 20`). |
| 110 | T | "Part 6 — The settled policy (from the spec…)" | 2712–2890 |
| 112 | T | "unchanged from the spec except the `MAX_REL_DEPTH` rename" | 2712–2890 |
| 116 | T | "the spec's old 'below-screen-resolution' trigger is struck" | 2712–2732 |

### principia_caching_contract.md
| line | class | reference | target |
|---|---|---|---|
| 9 | T | "The spec has a quad address … the spec's old name `TileID`", 2 refs | 2443–2455, 2918–3056 |
| 51 | T | "The spec has this as emergent behaviour (`ensure_baseline_tiles` …)" | 2785–2890 (~2828) |

### principia_dd_integrator.md
| line | class | reference | target |
|---|---|---|---|
| 185 | T | "transcribe the exact component→axis order from the spec's shape-sphere section" · "colour spec §8.1" | 333–383, 1030–1051 · colour PDF §8 (R-3) |
| 216 | M | "§generation-root ledger" | principia_dd_generation_root.md |
| 244 | T | "confirm against the spec's event-detection section" | 987–993. **It has no priority order.** |
| 246 | T | "against the spec's shape-sphere section + colour-spec §8.1" | as line 185 |

### principia_dd_generation_root.md
| line | class | reference | target |
|---|---|---|---|
| 126 | F | "payload spec §4" | dd_simstate_payload §4 (resolves) |
| 165 | S | "spec §tile_reduction … the LaTeX is the document being replaced" | wording only |
| 369 | F | "render GUI spec §12.1" | render_gui_spec §12.1 (resolves; rewritten in step 6) |
| 455 | S | "the LaTeX target held only prose" | wording only |

### principia_sampling_msaa_note.md
| line | class | reference | target |
|---|---|---|---|
| 27 | T | "spec §ensemble … see the sampling-pattern section", 2 refs | 1135–1172; jitter pattern at ~1932 |

### principia_lowering_contract.md · principia_render_contract.md · principia_dd_telemetry_and_tiers.md
| file:line | class | reference | target |
|---|---|---|---|
| lowering:51 | T | "matches the spec's `QuadRequest.flags` design" | `TileRequest` 2749–2784 |
| render_contract:85 | S | "The bit layouts exist in several places — LaTeX spec, …" | wording only. The payload drill-down owns layouts. |
| telemetry:398 | T | "the spec-keyed defaults are placeholders" | **Resolved by R-4: repoint to telemetry §3.5, no port.** Don't port `sec:quality_tiers`. |

### principia_canonical_spec.md · principia_dd_predictability_horizon.md
| file:line | class | reference | target |
|---|---|---|---|
| canonical:144 | S | "`spec_pending_changes` (LaTeX edit queue)" | wording |
| canonical:155 | S | "The LaTeX spec is being rewritten from this corpus…" | replaced by the retirement ruling |
| canonical:151 | M | "validation_ground_truth §correctness-factoring" | resolves |
| predictability:352 | S | "Do not put … in the spec until §7.7 resolves it" | "the spec" now means the corpus. Wording only. |

### Cross-references that resolve or are unrelated to the LaTeX
| file:line | class | note |
|---|---|---|
| parity_contract:13 | M | §correctness-factoring and systems-architecture §1 both resolve |
| systems_architecture:142, 238 | M? | "§parity" → repoint to `principia_parity_contract.md` |
| validation_ground_truth_note:121 | D | "integrator contract §collision" (as integrator:279) |
| deep_zoom:101 | D | "integrator contract §regularisation" (as integrator:345) |
| export_animation_contract:44, 64, 68, 70, 76, 80 | F | the animation "spec object" |
| systems_architecture:46, 155 | F | "spec objects" |
| render_gui_spec:240 | F | "canonical spec §8" |

### principia_spec_pending_changes.md: 22 lines, class R
This whole file is the LaTeX edit queue. It is closed in step 4 (fold each change's substance in, then archive the file). The
`.tex` sections it names are: `sec:tile_summary` + `par:tile_reduction` (change 1), `sec:burrau_charts` (2),
`sec:lookup`/`sec:inverse_encode`/`sec:LE_view` (3), the reduction-cascade figure caption (4),
`ICDescriptor` naming (5), and the `α` link, `α_min` 216–218, 758 and 2991 (6).

## Why this map and the audit count differently

The audit counts 85. This map counts 103, and of those **51 are real LaTeX dependencies (T)**. The
difference is mostly the 22 register lines, 11 false positives, and lines that name several sections at
once. Every T has a target above. Nothing was left unresolved.

## Step-3 work order: the PDF port items (R-3)

Besides the T references above, step 3 ports these from the retired PDFs:

| from | into | also repoint |
|---|---|---|
| colour PDF **Eq. 5** (the vMF engine) | `principia_dd_colouring.md` §3.2 | dd_colouring:3 ("the colour-spec PDF is already publication-grade"), §3.2's heading ("colour-spec Eq. 5, verbatim") |
| colour PDF **§8** (the shape-sphere physics overlay: the component → axis convention) | `principia_dd_integrator.md` §3.7 | dd_integrator:185, 246 |
| **COM-projection mini spec** | the integrator contract, together with `sec:com_projection` | integrator_contract:21, 34 |

## When step 3 finishes

This map has done its job at that point. `SECTION_MAP.md` moves to `archive/`, together with the `.tex` and the
two retired PDFs.
