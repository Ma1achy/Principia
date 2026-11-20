# Saving, sharing and exporting an IC

*Draft against main @45caf8be. Everything not cited is new.*

## 1. What exists today

The Inspector edits one IC; bodies are shown 1, 2, 3 and stored 0, 1, 2 (R-449). *Create & locate* builds one and
offers lock, centre, mark or **keep** (§G8 :1192–1213). Kept orbits are `ViewUI`, not undoable and not exported
(`principia_gui_state_contract.md` :71, :88, :177). `Trace.ic` is "the decoded IC (chart, uv, sim key)"
(`principia_lines.md`:106–107), a chart point, so it cannot hold an off-slice built IC. Integration enters only by
`computeIC(chart, uv, simKey)` (`principia_parity_contract.md`:17). The prototype edits velocity and shows momentum
(`ic_inspector_scratchpad.md`:13); its mirror test has deadband `δ_λ = 1e-12` (:24, R-82). The decoder's
`β ∈ [0, π]` covers one hemisphere (`principia_dd_decoder.md`:145). R-449 keeps machine-readable output 0-based
(`decisions.md`:10788–10800); JCS writes −0 as `0` (state contract :66). The mirror-twin ruling is not yet in
`decisions.md`.

## 2. The gap

An IC cannot be copied, saved, shared or reopened. An off-slice IC has no carrier, and there is no trajectory export.

## 3. Proposal (new)

**The IC record**, `principia.ic/1`, JSON in JCS:
```
bodies:    "0-based; shown as bodies 1, 2, 3 (R-449)"
as_built:  { m: [3], r: [3][2], v: [3][2] }      the constructor's own variables and frame
canonical: { z: [8], mirrored, hemisphere, links, entry_hashes }
source:    "built" | { pixel: { chart, uv } }
sim:       SimConfig (canonical) ; build, schema, label, made (made outside the identity)
```
The primary is `as_built` for a built IC and `source.pixel` with `sim` for a picked one; everything else is re-derived
on load and compared. **Bitwise at f64:** JCS's shortest decimals are exact for every finite f64 except −0, so one
function normalises −0 to +0 as values are written. Non-finite values are refused at entry.

**Copy, save, open.** Ctrl+C in the Inspector copies the record; *Copy as text* gives a 1-based table. Ctrl+V pastes a
record. *File › Save IC…* writes `<label>.principia-ic.json`; *Open IC…* (or a drop) opens it in *Create & locate*,
in its as-built frame, with one of three outcomes: **recovered**; **physics only**, when a link is unknown or its
entry hash differs (R-521), which loads `as_built` (it does not depend on the decoder) and refuses only "locate on this
chart", naming why; or **corrupt**. The stored `z` against the recomputed `z` is shown as a residual.

**Mirror twin.** The constructor shows both hemispheres with a faint marker at the twin. `as_built` keeps the
hemisphere built in; `canonical` is folded, recording `mirrored` and `hemisphere` (the sign of `λ̃_y` past `δ_λ`). A
reopened IC sits where it was built, with its twin opposite, and on the seam there is no twin.

**Integration.** The constructor integrates `canonicalise(as_built)`, the survey's frame, and says so. That needs a
state-in entry beside `computeIC(chart, uv, …)`, since an off-slice IC has no `uv`, and decoding `z` is not bitwise.

**In views.** *Keep* stays scratch. **Mark** becomes the durable act: `Trace.ic` widens to
`IcRef = OnChart{chart, uv} | Built(IcRecord)` inside `RenderState.Lines`, so a marked IC travels in snapshots, saved
views, share links, the animation spec and the pxpack (tier 2), round-tripping under R-484. *Mark all kept* promotes
the list. One record is roughly 0.6 KB (an estimate, to be measured), so marks crowd 64²'s tier 2 (1,516 B, embedding
§7 :330–332): tier 2 drops whole, and `tEXt` keeps everything.

**Trajectory export.** *Export trajectory…* writes `<label>.traj.csv`. A `#` preamble holds the IC record; the
integrator (RK45 f64 and tolerances, R-33); the outcome and its `detail`, 0-based with a 1-based gloss; and "incomplete:
step budget" where that applies (pointer_channels §3 :79–84). The columns are
`t, r0x…r2y, v0x…v2y, E, Lz, n0, n1, n2, dE_E`, one row per step as taken, with no resampling, which would invent
values. An optional stride is labelled "every k-th step; close encounters not resolved" (philosophy §7.3).

## 4. Fit with rulings

- **Amends** state contract §2 and lines §3 (`Trace.ic` becomes `IcRef`), and R-484's tier table (marks are tier 2).
- **Amends** `computeIC`'s contract (parity :17; REQ :15877, "only through computeIC"). This needs a ruling.
- R-449 is honoured: the files are 0-based and say so, and the GUI text is 1-based. The new mirror-twin ruling is
  carried by pane 1 and the record.
- §G8 gains Copy, Save, Open and Export trajectory, and File gains *Open IC…*.

## 5. Open questions

1. **Where marks live.** (a) `RenderState.Lines`; (b) a sidecar outside the identity; (c) IC files only. *Recommend
   (a)*: marks are drawn, so they are render state.
2. **Primary velocity variable.** (a) `v`, with `p` derived; (b) `p`; (c) both stored. *Recommend (a)*: `p = m·v` is
   not bitwise invertible, so only one variable can be primary.
3. **Frame integrated.** (a) canonical; (b) as built; (c) both, with the difference shown. *Recommend (a)*.
4. **−0.** (a) normalise to +0; (b) hex bits, like R-322's u64 strings; (c) both. *Recommend (a)*.
5. **CSV body labels.** (a) 0-based, with the preamble saying so; (b) `body1_x`; (c) both. *Recommend (a)*, per R-449.

## 6. Tests

| Test | Fails when |
|---|---|
| Save, open and re-save give a byte-identical file; `as_built` is bitwise over 10⁴ random ICs, −0 included | a lossy number format, or −0 survives |
| Built in the lower hemisphere, reopened: dot lower, twin upper; on the seam, no twin | `as_built` is replaced by the canonical form |
| A changed link entry hash loads the physics and refuses to locate it, naming the link | a default link stands in |
| A marked IC survives a snapshot, a saved view, `tEXt` and a share link bitwise; kept-only ICs are absent | marks ride `ViewUI`, or `ViewUI` leaks |
| The CSV's rows equal the Inspector's steps; a truncated run is marked incomplete | resampling, or a silent cut |
| The CSV's `detail` is 0-based and the GUI label 1-based for one collision | the conversion is skipped or doubled |


## Decided (Malachy, 21 Oct 2026)

Accepted as proposed, with one change: **CSV body labels are 1-based** (`body1_x` …), as bodies are always displayed
1-based. The preamble says so.
