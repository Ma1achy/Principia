# Principia — chart composition: axes from primitives, in the Chart builder

**Status: Proposed, 6 Nov 2026, on the owner's orders of that day. Not a ruling. Nothing here is implemented until the
rulings report is answered.** No code is written by it. Its one-way doors are not decided here: each is marked **door**,
and waits on the rulings item named beside it (RQ-712 to RQ-731). RQ-733 sits in § 14 but outside the gate, TASK-M2-58: it holds only its option (b). RQ-734 and RQ-735 sit there too, and hold no task of § 15. RQ-736 is ruled, 7 Nov: TASK-M4-34's preset and TASK-M8-120 read it. Two are agreed in principle by the owner, and still not
rulings: no fifth axis kind (RQ-712) and outside region D blank and uncounted (RQ-714), the second since overtaken for the presets by the owner's filter order (§ 10.4).
Where this document meets a ruling, the ruling wins. Ids are final, from the id pass of 7 Nov 2026. The rulings branches of 4 and 5 Nov and PR #387, TASK-M2-37, have merged, and this
document's base is `main` at 13edf773, so R-694 to R-720, R-791 to R-809, RQ-682, RQ-700 and RQ-701 are read from there. PR #388,
TASK-M2-52's branch, is read as it stood on 6 Nov.

**Ruled 7 Nov 2026, by the owner's reply of that day and the orders it held (the rulings PR).** Where the body below
still reads as Proposed, these rule, and TASK-M2-58 carries each into the body. RQ-712 (a), R-834; RQ-713 (a), R-835;
RQ-714 time (a), R-845, with exact rest the velocity default and eq. 3 the second preset, R-851, its outside still open;
RQ-715 (a) with (i), R-836; RQ-716 (a), R-837, C3's 2 s on Vulkan still open; RQ-717 as RQ-682 (a), kind 3, R-838 and
R-816; RQ-718, R-846: R-669's reversal rewrites it, a user's own plane stays unscored; RQ-719 (a), R-839; RQ-720 (a),
R-840; RQ-721 (a), amended, R-850, with its rider, R-849: no labelled jump under the lock, the anchor slides
continuously to its nearest point in B, the jump kept only for time-axis to IC-only charts; RQ-722 (b), a label "not at
rest", R-841; RQ-723 (b), orbital elements as axes at a labelled pair, plus overlays, R-847; RQ-724, `h` a builder axis,
R-848; RQ-725 and RQ-726, R-669 reversed, fixed-`k` and binary-plus-single charts built when their turn comes, R-847;
RQ-727 (a), R-842; RQ-728 (a), R-843, its mask read with RQ-714's open part; RQ-730 (a), R-844; RQ-733 (b), a general
numerical solve on CPU batch, R-849; RQ-736, R-853, with the symmetry rule, R-854, and the filter order, R-855; the model's
§ 6b classification, R-852. RQ-711 (a), R-820, the zoom row's scale still open. RQ-700 (b), R-817, in chart_reference §2.1, which
TASK-M2-70 carries into § 9: at `Q₀ > 0` the virial floor sits under the rise only, so § 9's axis reads its coordinate `Q(t)`, the state's own ratio
on a column where `Q₀` is feasible, and `Q above the floor` is its label at `Q₀ = 0`; its label at `Q₀ > 0` is RQ-741's. Open: RQ-729, RQ-731 and RQ-732, the
record PR's; RQ-734, RQ-735; RQ-738, RQ-736's correction list and the filter's conflicts; RQ-739, the mirror-step
proposal; RQ-741, the virial axis's label at `Q₀ > 0`.

**The owner's rulings of 8 Nov 2026 on building it.** R-863 orders the tasks of § 15 in five stages, each merging only
with the previous stage's tests green, and adds TASK-M2-61, the path object with today's operations bitwise through it
(§ 15). R-864 is the architecture rule for the chart-path work of § 17: one object, today's operations re-expressed as its
named families, bitwise first, and a type sketch brought to the owner before any implementation.

**The owner's replies of 8 Nov 2026 to the code map, and their points of interest.** R-865, on RQ-739 (which stays open on its item 3, the optional mirror-pairs statistic): the decoder's
mirror step is removed and latent `β` runs over the full circle, conditional on the bitwise check, built by TASK-M2-57
(§ 6.6), which waits on RQ-750, the full-circle link. R-866: locking doesn't move the view, the pin at the clicked pixel and tilt and zoom turning about it, built by
TASK-M8-125 in stage 1 (§ 15, § 17.8). R-867: the lock's anchor is stored as the signed starting state, not a latent
point, TASK-M2-65's (§ 17.8). R-868: the model's wording follows R-474, and the owner's corrected model replaces the
repo's; RQ-738 stays open on the rest of its list. R-869: the owner's direction on RQ-745, which stays open. R-870: points
of interest, § 10.5, built by TASK-M8-126.

**The owner's reply of 8 Nov 2026 to that day's rulings report, and their messages of the same day.** Where the body
still reads as Proposed, these rule, and TASK-M2-58 carries each into the body as it does the 7 Nov rulings. RQ-714's
outside, (b) with RQ-738's conflict 3 read as its (c), the full disc rendered literally, every mirror and body swap
declared, the filter giving exactly D with no mask, R-871 (§ 8.4); RQ-716's 2 s, the Mac's, #427 setting any Vulkan and
Windows budget, R-872 (§ 11); RQ-744, the stages' leftovers placed, R-873 (§ 15); RQ-739, closed, the full count the
default and "mirror pairs counted once" an optional statistic, off by default, R-874 (§ 6.4, § 6.6); RQ-745 (b), the
acute-angle Burrau charts unfolded through the filter, R-875; RQ-740, a refusal in the f32 band labelled as rounding,
R-876; RQ-741 (a), the virial axis labelled `Q` at `Q₀ > 0`, R-877 (§ 9); RQ-738's conflicts settled by their readings
held, its model items 2 to 11 the owner's, R-878; RQ-734, final-condition charts built after the M8 charts by
TASK-M8-128, the rejection of § 3.1 narrowed to trajectory functionals and event reads, R-879 (§ 3.1, § 10.3); RQ-729 (a),
R-884 (§ 20); RQ-711, the saved view reading 1×, R-888; RQ-735 (b), a WASM CPU route, R-889 (§ 7.5); RQ-731 and RQ-732,
the record corrections, R-891 and R-892. On the chart-path work (§ 17): the consolidation step between stages 1 and 2,
TASK-M8-129, R-894; RQ-746 (a), R-895; RQ-747 (a), the tilt angles stored beside `tilt_map`, R-896; RQ-749 (a), stage 2's
test at the pin, R-897 (§ 17.8); RQ-750 (a) + (c), a periodic latent `β`, R-898; the brief's nine questions, R-899
(§ 17.8); stage 2's saved-file rewrite as RQ-750, R-900; easing per path segment, R-901 (§ 17.9); the lock as the path's,
the mid-morph tilt and stage 2's gate test, R-902 (§ 17.5, § 17.8). Open: none of § 14's items; RQ-738's model items.

**The owner's rulings of 9 Nov 2026 on where it is built (R-916 to R-919).** Where the body names a task by its old id,
R-916's Was/Now table gives the new one, and where it places work in M4 or M8 that is now M2's, § 15 rules. R-916: M2 is the
whole non-visual chart system, this design's registry, axes, pairs, refusals, labels, curved slices, CPU decode and encode
and path maths, each an engine function with its tests, today's charts and operations re-expressed bit for bit first; GPU
lowering, display and the GUI stay in their milestones as thin adapters onto it, and tasks mixing the two are split
(§ 15). R-917: the build order is the chart system, then the integrator, then everything visual, and M2's gate is the
checklist G1 to G14 (`plan/MILESTONES.md`). R-918: TASK-M3-14 waits for that gate, and M2's maths that reads a trajectory
reads it through the interface of § 18.7, tested on exact fixtures. R-924: TASK-M3-14 resumes now; following it, per R-369,
FTLE's chart independence (REQ-CHART-132) moves to TASK-M3-69, which waits on TASK-M2-71 and that gate. R-919: the owner's answers to the plan's frictions: the
merged GUI routes made adapters in TASK-M2-64 (F1 (a)); the gate test's round trip read as R1, bitwise (F10; § 17.14); "round
trips across paths" read as G4 (F11); `τ` provisional at M2 (F13 (b)); RQ-679's spans answered now, proposed by physics for
the owner to confirm, tilts into `ν₀`, `L_z` and `K` in M2 (F14); R-864's sketch approved when TASK-M2-61 brings it; the
trajectory fixtures the Lagrange equilateral homographic solution and the figure-eight (F18, § 18.7); every calibration set
by measurement with its criterion written first (R-649, R-650). R-920: the owner confirmed RQ-679's spans as physics
proposed (`principia_chart_reference.md` § 4.5). The status above, Proposed, stands until TASK-M2-58 (was
TASK-M4-37) carries the rulings into the body.

**The owner's decisions of 10 Nov 2026 on the chart system (R-925, R-926).** Where the body reads otherwise, these
rule; the one-way questions they leave are filed with the plan that proposes their tasks,
`plan/charts/m2-chart-system-replan.md`, and no task is dispatched from it. A chart is a map, from named coordinates
to a system state and a time, composed with a flat window, a centre and a basis; pan, zoom, rotate, tilt and slice
are edits of the window, an axis change a tilt to 90° (R-926, item 1). Time is the last coordinate of every map and the
playhead the window's centre along it (item 2), so § 18.1's "a chart mode" and § 18.4's "time is never a tilt
target" and its cut from a time-axis chart to an IC-only chart give way, R-842 amended in part; what time as a
coordinate does on the integrator's integer grid is open. A morph is a blend of two charts' states, linear in masses,
momenta and time and a slerp on shape, exact at both ends, navigated by one pixel-space edit on both ends (item 3;
R-925, item 4): "morph" names this system blend only, a turn of the window within one map, § 17.2's `Path::Tilt` and
§ 17.7's plane-to-plane path included, is a tilt, and a blend is a chart; what becomes of § 17.2's other arms is open.
The CPU evaluates every chart in f64 and the GPU only integrates records, each quad dispatched with a forward table
built on the CPU, with no inverse on that path (items 5 and 6): § 7's three lowerings are no longer three integrator
routes, § 7.2's CPU batch is how every view is filled, and its seam-3 exception, R-839, is superseded. The fragment
side keeps a chart's generated shader code where it has closed-form code and otherwise reads CPU-built tables, a baked
backwards table with a table search as fallback for Chart position, `ctx.chart.z` and the decode and round-trip
presets, never a refusal (items 7 and 8; R-925, items 1 and 2): § 3's "`numerical` (CPU batch only, never in the
kernel)" holds for the integrator, and the fragment reaches such a chart through tables. Slopes come in three tiers,
exact, implicit and none (item 9); a small quad's one-cell table is the linearised decoder (item 11).

**The owner's reply of 10 Nov 2026 to that day's rulings report, and their messages of the same day (R-931 to R-945).**
Where the body reads otherwise, these rule, and TASK-M2-58 carries each into the body. The chart re-plan's one-way
questions (R-935): RQ-758 (a), exact or implicit slopes as `J_D`, central differences the check; RQ-759 (a), the lock's
re-solve continuous along a path, stopping where the followed minimiser ends, R-849's time-axis jump dropped; RQ-761 (a)
with (a-i), a per-sample stop index on the integer grid and a new label, "reached its time", the zoom along time floored
at one boundary; RQ-762 (a), the window's scale; RQ-763 (b), measure first; RQ-765 (a), the GPU decode entries as test
entries, R-837's cap retired once measured; RQ-766 (a), one exact node per sample; RQ-767 (a), the table seeding a
refinement on the forward code; RQ-768 (a-table); RQ-769 (a), two tracks; RQ-770 (a), pan and zoom still re-address the
cache. RQ-760 (R-936): § 17.2's `AxisBlend`, `PairMap` bend and `PerField` are retired as morphs and offered as maps;
the mass block blends `μ_k = ln(m_k/m₀)`, "linear in masses" having been loose wording for that; no area statistic on a
mid-morph frame, § 17.5 kept there; no true copy at a fold. RQ-764 (a) (R-934): the seed `w = r/√I` everywhere, its
images updated under R-931, the goldens rule. The general rule for a morph (R-943): a pairing, a path and an easing; the
default path the shortest in each block's natural geometry, other paths allowed and not built now. The principle
(R-944): a morph frame is a chart, and every chart feature works on it; a stated mid-morph limitation is a gap to close;
area statistics are the one exception. R-945: R-926 item 7's list was not exhaustive; the backwards direction serves
anything that places a running or saved state on a chart. Chart edges (R-932, closing RQ-773 (a)): no blank areas;
saturate past a true edge, unfold onto a second sheet, wrap where the coordinate repeats, the mechanism per edge physics
review's call. RQ-771 (a) with (ii-2) (R-937): the invariant charts' spin edges unfold onto the time-reversed sheet, and
below the floor at a non-zero spin the rigid rotation at the pixel's own spin is drawn. RQ-772 (R-938): every chart's
edges built now, time before zero the starting state. RQ-774 (b) (R-939): a marking toggle, off by default. RQ-777
(R-940): no summed axis number; a dual ruler in a blend, window units on a tilt or rotation, every true value on hover.
RQ-775, RQ-776, RQ-778, RQ-780 and RQ-781, each (a) (R-942): § 3's extension per registry row becomes kinds per edge
with a default, a view holding an override; § 17's "axes carry no ticks" is withdrawn. Open: RQ-779, the per-edge
defaults by eye (R-941: per the rule, the mass simplex saturating); RQ-783, a record made before the f64 decode route.

**The owner's reply of 10 Nov 2026 to the draft of the chart re-plan's addition (R-947 to R-952), and R-944 applied item
by item.** Where the body reads otherwise, these rule. One placement function answers "where is this state on this
chart" for every consumer, by the encode where a chart has an inverse and by the nearest point everywhere else, every
blend included, refined to the lock's precision on the CPU (R-947): § 10.6, new, which also gives § 10.5 the definition
it owed. Traces at the new edges (R-948): a sample sits on the sheet its state lies on, the trace jumps at the fold with
an optional tick, and nothing is drawn inside a clamped or saturated area, physics review confirming;
`principia_lines.md` § 8 has it. A chart change that changes a link id morphs like any chart change (R-949): § 17.12's
conflicts table, § 17.13 and § 17.14. The lock's leftover jump from a time-axis chart to one with no time axis is
removed (R-950): the first paragraph's "the jump kept only for time-axis to IC-only charts" no longer holds. Two
measurements, P2 and P4, are reported and are not gates, shown to the owner before the trace tasks are built (R-951);
P4's unit and the set-up are R-977's (*12 Nov 2026, the owner's answer on RQ-785:* P4's gaps are counted as a fraction of the picture width, their distribution reported and the share over 8 px read at 512×512, 1024×1024 and 1920×1080 from the same run; a pinned trace, P2 at both ends, the fixture's own output steps at each chart's home window, zoom 1; *later the same day, R-984, the owner's follow-up, which closes RQ-785:* P2's jump is measured as a fraction of the picture width too, its median and worst read in px at the same three sizes; *was (R-977's line):* "P2's unit stays open on RQ-785"; *was:* "the picture size their pixels are counted at is open, RQ-785"). Contours are evaluated on the CPU for every chart (R-952):
§ 10.2 and `principia_lines.md` § 4.2. § 17.5 is rewritten item by item under R-944, each stated mid-morph limitation
closed, naming its mechanism, or kept as the one accepted exception, area statistics.

**The physics review of PR #482 (review 5852979675, 11 Nov 2026), on what waited for it.** R-948's three answers are
confirmed, (i-a), (ii-a) and (iii-a), with three points of definition owed (`principia_lines.md` § 8, "Owed"). (D): the
mechanism is approved, and the row-major representative is acceptable as a gauge choice, with what the written
definition must state (§ 10.4). (F): answered tag by tag, three tags blending and three not, with two conditions
(§ 17.11). The collision-point case of § 17.11 is confirmed, and a second case, `M01_TINY` between two valid ends, is
open as RQ-787 (*12 Nov 2026:* ruled by R-975, the pixel saturating by physics review's smooth hold, § 17.11). Where the paragraph below says "not settled until it confirms", read it with these.

**Decided per R-369 on the way, each listed for the owner's veto.** The owner's words on the first five (10 Nov 2026),
verbatim, under their heading "THE FOUR YOU DECIDE":

> Go ahead with (C), (D), (E) and (F) as you proposed, listed for my veto. The unlocked pivot for a
> saved frame (each end about its own centre) is fine too.

That is a go-ahead to decide them, not a ruling on any of them. Applied per R-369: (C), a Chart or Latent contour
variable on a mid-morph frame has one value per end chart and is drawn once per end at that end's share
(`principia_lines.md` § 4.1); (D), on a mid-morph frame marks are found to the lock's tolerance and the symmetry filter
is on (§ 10.4, § 10.5), conditional on physics review and not settled until it confirms; (E), a change of the hovered
system caused by a playing morph does not restart the trace (`principia_lines.md` § 8); (F), an end pixel that is tagged
and carries a state blends as a state, only an end pixel with no state staying blank mid-path (§ 17.11, V21),
conditional on physics review, tag by tag, and not settled until it confirms; the unlocked pivot of a saved frame's
tilt, each end about its own centre (§ 17.5). Applied per R-369 as mechanical consequences of the rulings named at each,
and listed for veto with them: the hold's blank does not open mid-path under the state blend (§ 17.11; R-926 item 3,
R-943; *was:* "no blank opens mid-path under the state blend", which the physics review of PR #482 found incomplete:
`M01_TINY` can open between two valid ends, RQ-787, where the pixel saturates by R-975 of 12 Nov 2026); a `gauge`
end's paper scale is drawn on its own side of the dual ruler (§ 17.11; R-940); a pan or zoom on a blend re-addresses the
cache as on any chart (§ 17.12; R-935); the "CPU batch" label and the GPU blend entry are moot (§ 17.12; R-926 items 5
and 6); two time windows blend like any two windows, V29 retired (§ 18.4; R-926 item 3); a blend's well-posedness, in
words (§ 17.5; R-943); a blend's repeats (§ 17.5).

**The order, in a line.** Charts are built from primitives and combinations of primitives, each axis independent and
customisable, composing with what exists and adding no chart type. Any axis may be paired with any other. The only
refusals are genuine contradictions, each with a named reason. Reduced capability is a label, not a refusal.

**What this document finds, in ten lines.**

1. Nearly all of it is already there: the four kinds, `Mixed`, the warps, the validator, the links, tilt and Save view
   as chart. The new things are a registry, a third lowering (CPU batch), a constraint (hold `L_z`) and two pair maps.
2. Every primitive the order names fits one of the four kinds, so no fifth kind is needed (§ 2), but the virial ratio
   fits only by a ruling already asked: RQ-682.
3. Region D's axis pair `(ξ, η)` is the **stereographic projection of the mass-weighted shape sphere from the collision
   of bodies 0 and 1** (§ 8.2). So it is closed-form at every mass, needs no `atan2`, has no pole in view, and it carries
   the angle and side-ratio pairs with it.
4. The virial axis is built by TASK-M2-37 (`AxisVirial`), with its warp written. What it lacks is RQ-682's
   answer, so "then virial" is a descriptor and a ruling, not a build (§ 9).
5. A baked axis pair costs little to build and more in the module, which every pipeline parses whole. 4 passes the
   measured criterion on the Mac and on the Windows Vulkan run and 8 doesn't, on every test that moves with the count.
   **4 is the proposal.** One test doesn't pass on Windows at any count, none included: the 12 gallery pipelines alone
   take 2.7 s against C3's 2 s, which RQ-716 has to settle. It holds only while decode stays its own entry (§ 11).
6. A view is a point, and every edit a path, but morph is not the general form of tilt and zoom in one formula: no single
   blend reproduces them bitwise. The path type does, with tilt and zoom as arms running their own code (§ 17).
7. Of the 6 Nov primitives (§§ 3.1, 3.2), most are axes on CPU batch or aliases of existing rows. Orbital elements and
   Mardling–Aarseth aren't axes, since a ruling parks them. The scattering set conflicts with R-669 and stays definitions until
   ruled. The rejected list is stated there.
8. `(ξ, η)` is geometric at every mass: `ξ + iη = d + λ/ρ` (§ 8.8). `d` stays. Anosova 1969 isn't region D.
9. Time as an axis is a chart mode, no fifth kind. Visible rows cost a normal payload but still amend the temporal note; dense output on the macro-step grid conflicts with the core. The default occupant resolves time only to `Δ_sync`, and its re-measure is owed (§ 18, § 19).
10. Paper frames are presets, § 20; several listed gaps are readable, others remain.

---

## 0. The order's rules, checked

Each rule was read against the code, `decisions.md` and both rulings branches. A conflict is the ruling's, and goes to § 14.

| Item | Result |
|---|---|
| 1. Extend the builder | §G7, R-471, R-506, R-407, R-408, R-475, R-513, R-514, R-478 to R-482, §0.6 and R-670 are as the order cites them. "A new axis kind" meets "a closed set": RQ-712. R-798's words are about PA-1: RQ-712 |
| 2. Two layers | Holds, but a feasibility warp is not per-axis (§ 5.5, row 3). Hold `E` exists; hold `L_z` is new physics: RQ-715 |
| 3. Registry | R-407 clause 3 is as cited; `I = 1` at every decode is dd_decoder §5's identity 1. "CPU batch only" meets seam 3: RQ-719. Side ratios meet R-798: RQ-713 |
| 4. Gaps | "Today only the TR-1 engine scan" is not so: no code has it (§ 10.1). TASK-M7-43 and R-476 are as cited. Rule 6 is as cited. The already-covered list holds |
| 5. Region D, then virial | Defined in dd_validation_orbits §1.6. REQ-CHART-070's virial warp is written by TASK-M2-37: § 9. Virial's kind is RQ-682 |
| 6. Tests | § 12 |
| 7. Doors | § 13, § 14 |
| Any pair | §1.3 and REQ-CHART-008 hold. R-669's refused charts: RQ-718. R-594 item 5: RQ-720 |

---

## 1. What exists, and is kept

The model is the Chart builder (`principia_render_gui_spec.md` §G7). None of this is rebuilt, and each row was checked
against the code.

| Exists | Where | Used here as |
|---|---|---|
| Four axis kinds, "a closed set"; "no fifth" | chart_decoder_contract Part 3; `chart_authoring::axis::AxisKind`; REQ-CHART-004 | every primitive declares one (§ 3) |
| Axis kinds lower, never interpreted | core_design seam 2; lowering Parts 1 to 4; REQ-CHART-004 | a pair runs on the GPU only as a baked entry (§ 7) |
| `Mixed<H, V, P>`, any two quantities, the other six frozen | `kernel::chart::mixed`; chart_reference §1.3; REQ-CHART-008 | the pair's home |
| Warps and the linear option | R-471, R-506, R-508; `kernel::chart::energy::{Mapping, Mapped}`, TASK-M2-37's | the warp slot of each axis (§ 4) |
| Extension types, primary ranges | R-407, R-408, R-472, R-537; chart_reference §5.4 | each primitive's natural type (§ 3) |
| Tilt, slice, Save view as chart | R-473, R-474, R-475, R-513, R-514, R-801 | composition with a composed chart: `Tilted<C, P>` wraps it |
| Domain preview | §G7; `validate(u, v)`, R-26 | shows why (§ 6.3) |
| Links | R-478 to R-482 | every axis reads the active link; a preset may pin (R-516) |
| `forbids_energy_normalisation` | chart_reference §0.6; R-25; R-670 | the pattern hold `L_z` copies (§ 5.3) |
| Well-posedness validator | `chart_authoring::validate`; contract Part 5 | gains refusals, loses none (§ 6) |
| Annotation axes, a 1-swept family in 2D | contract Part 5; the Euclid plane | the 1D chart (§ 10.1) |
| Point imports | `crates/validation/src/import.rs`, TASK-M2-20 | region D's points (§ 8.5) |

**Already covered, not rebuilt (the order's list, confirmed):** symmetry reduction by canonicalisation; the shape
sphere's two halves kept (R-544) and relabelling kept distinct; landmark-centred presets (R-798's among them); pivot
through Save view as chart; tilted configuration–momentum planes (R-473).

**Existing presets and saved charts reopen bitwise.** Every new field defaults to what is built today, and a default
field is not serialised, so the canonical form (gui_state_contract §2; R-484) and the sim key do not move. § 12 tests it.

---

## 2. The two layers

```
Axis    (independent): primitive or combination · warp · extension type · what it holds fixed
Chart   (optional, couples both axes): pair map · constraints (hold E, hold L_z)
Lower   (one of three): uniform affine · baked entry · CPU batch        ← a label, never a refusal
```

An axis written alone is independent. § 5 says each place where that is not so. In the CPU authoring layer:

```rust
struct AxisSpec   { primitive: Combination, warp: Warp, extension: Extension, holds: Residual }
struct ChartSpec  { axes: [AxisSpec; 2], pair_map: PairMap, constraints: Constraints, slice: Slice }
```

`ChartSpec` is data, authored on the CPU, and the kernel never sees it (REQ-CHART-004). `validate` turns it into a
`ValidatedChart`, as today, and `resolve` lowers it.

**The kinds stay four.** The order's item 1 says "a new axis kind, warp or pair map"; the contract says "a closed set"
and "no fifth". Read together: a primitive is a new *entry* under one of the four, with its metadata. This is
**door** RQ-712. In this reading the lowering appendix's Anosova row, "physical × physical", is kind 2 × kind 2 on the
configuration block, as the shape sphere's row is, residual none: the row is conformed, not a new kind.

---

## 3. The primitive registry

One table, `chart_authoring::registry`, CPU-side. Every row declares:

- **forward**: state to value, always; the Inspector, the contours and every test read it.
- **inverse class**: `closed` (live on the GPU where baked), `numerical` (CPU batch only, never in the kernel) or `none`
  (overlay only). A closed inverse may need a partner: it is then registered on the pair.
- **kind**: one of the four.
- **extension**: the natural type. Angle-like rows are flagged: a periodic or pole-crossing declaration, or affine on an
  angle-like axis, needs physics review (R-407 clause 3; chart_reference §5.4, "an angle-like quantity declared affine
  would redraw systems and count them again").
- **scale**: `free` (a shape or a ratio), `gauge` (fixed by `I = 1`, so a paper's value needs the recorded rescale,
  dd_validation_orbits §2) or `size-only`, refused as an axis, since `I = 1` at every decode (dd_decoder §5, identity 1)
  makes it constant.

In the formulas `r_ij = ‖r_i − r_j‖`, `γ_k` is the interior angle at body `k`, and `(ξ, η)` is § 8's plane. Bodies are
0-based (R-22), shown 1-based (R-449).

| Primitive | Forward | Inverse | Kind | Extension | Angle-like | Scale |
|---|---|---|---|---|---|---|
| latent direction `q` | `z·q` | closed, assigned | 1 | affine | no | free |
| `α`, `β` | decoder §0.2 | closed, the link's | 2 | bounded | **yes** | free |
| shape `θ`, `φ` | §3.1 | closed on the pair, §3.2 | 2 | periodic, pole-crossing (existing) | **yes** | free |
| Jacobi `p_ρ`, `p_λ` components | §0.3 | closed, the link's | 2 | affine | no | gauge |
| third body `(ξ, η)` | § 8.2 | closed on the pair | 2 | `ξ` bounded, `η` bounded | no | free |
| interior angles `γ₀, γ₁` (any two of three) | `acos` of the law of cosines | closed on the pair, § 8.3 | 2 | bounded | **yes** | free |
| side ratios `r₀₂/r₀₁`, `r₁₂/r₀₁` | `r_ij` | closed on the pair, § 8.3 | 2 | bounded | no | free |
| Jacobi ratio `‖λ‖/‖ρ‖` | §0.2: `tan α·√(μ_ρ/μ_λ)` | closed, residual `β` | 2 | bounded | no | free |
| mass ratio `m_b/m_a` | `m` | closed, through the link (R-506 W3) | 2 | bounded warped, affine linear | no | free |
| mass control `z_μ` | `z` | closed, assigned | 1 | affine | no | free |
| Burrau `ν`, `θ` | §4.3 | closed, the embed | 4 | bounded | `θ` **yes** | free |
| body position, `BodyPlane` | chart_reference | closed, the chart's | 2 | affine | no | free |
| `E` | `K + U` | closed, solve into momentum | 3 | warped bounded (R-471) | no | gauge |
| `K`, `KE` | §2.2 | closed, solve into momentum | 3 | bounded, `t > 1` continues (R-508) | no | gauge |
| `L_z` | `Σ m r ∧ v` | closed, solve into momentum | 3 | bounded warped, affine linear | no | gauge |
| virial ratio `Q` | by mapping, dd_decoder §3.6, § 9: linear, or at `L_z = 0`, `2K/\|U\|`; warped at a held `L_z ≠ 0`, `2(K − L_z²/2I)/\|U\|` | closed; at `L_z = 0`, `K* = Q·\|U\|/2`; at a held `L_z ≠ 0` the warped axis reads above the floor, § 9 | **3, by RQ-682** | bounded both sides | no | free |
| area, perimeter | triangle | numerical on the pair (figure's Tier 2c) | 2 | bounded | no | gauge |
| `PE` (`U`) | §0.6 | **none**: REQ-CHART-070, no chart offers it until a warp and a residual are written | 3 | — | no | gauge |
| `min(r_ij)/max(r_ij)`, any `min` or `max` of primitives | piecewise | **none** | — | — | no | free |
| pair binding energies | `½μv² − m_i m_j/r_ij` | **none** | — | — | no | gauge |
| `I`, `R`, `ℓ` | §0.5 | refused: **size-only** | — | — | no | size-only |

*[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]* **Combinations.** A combination is an expression over primitives, in the stain's expression syntax, the subset §4.1 of
`principia_lines.md` needs (R-509). Its forward is the expression. Its inverse class is not read from its parts: `exp(K) + K`
has closed parts and no closed inverse, and `K + L_z` doesn't say which part moves. It is `closed` only with a registered
closed inverse, `numerical` only with a registered numerical one, naming what it solves for and its branch rule (§ 7.3), and
otherwise `none`: a forward-only overlay, and as an axis `NoJointInverse` (RQ-733). Its `angle-like` is true if any part is.
Its scale is computed, not declared. On the GPU no expression is ever interpreted (lowering Part 1): a combination reaches
the GPU only as a registered, baked row. Otherwise, with a registered inverse, it runs as CPU batch, where an interpreter may
evaluate its forward, since it is not a kernel.

**Scale-size refusal is a computed property too.** The `size-only` column is declared, and a test checks the declaration:
it samples decodes and fails any row whose forward is constant on them and is not declared `size-only` (§ 12).

**Mass-weighted shape.** The shape sphere is mass-weighted (§3.1). Geometric angles and side ratios are not: they invert
onto the sphere with the masses in the formula (§ 8.3). So an axis states which it means, in its `holds`: `(γ₀, m₂)`
holding a geometric triangle while the mass moves is a different family from `(θ, m₂)` holding a point on the sphere.
Both are legal and each is labelled.

**What is not an axis in the registry, and why (R-669).** R-669: "no binary-plus-single chart is built", and TS-2 reads "not
scored: no fixed-k velocity chart". So no row with an inverse names a binary phase, an eccentricity or an impact
parameter, and none is a fixed-`k` prescription; forward-only overlay rows may (§ 3.1). A builder option can't quietly reverse
a ruling. **Door** RQ-718 asks what a user-built pair that lands on one of those planes by other means counts as;
meanwhile nothing built here scores those targets.

### 3.1 The primitives of 6 Nov

The owner's order of 6 Nov, part C. Each identity `prim_check.py` names is checked to 2·10⁻¹³ over 20,000 random states, masses Dirichlet(3, 3, 3)
(`docs/incoming/2026-11-06_chart_composition/checks/prim_check.py`), except the polar row's 2 × 2 solve, whose determinant vanishes at a collinear state: 8.5·10⁻¹⁴ at `|det| > 10⁻³` on that draw, where the radial ratios stay small. Its error is `0.5 eps·r̂/|det|`, `r̂` the largest of 1 and the two ratios, and `r̂` grows as a gap nears `π`: on a draw there, 3.5·10⁻¹² from `|det| = 10⁻³`, 3.5·10⁻⁸ from `10⁻⁵`. `√chazy_qk[2]` is checked to 5.7·10⁻¹⁴ on the root, which is 2.9·10⁻¹¹ absolute and 3.9·10⁻¹³ relative on `q`, at `M = 1`, and `K_shape = I|ṅ|²/8` to 2.6·10⁻⁷, a finite difference (`docs/incoming/2026-11-06_chart_composition/checks/sweep.py`). **Lowering: all CPU batch, labelled `CPU batch: not baked`**,
since the cap's slots are taken (§ 11); only `SuvakovPlane` is baked, by its own ruling. Each closed row is a case of
`prim_round_trip` (§ 12), which tests `forward(inverse(x)) = x` and reads back the row's `Holds`, with a control that fires on that row.

| Primitive | Forward (`I = 1`) | Inverse | Extension | Holds | Contradicts | Targets | Verdict |
|---|---|---|---|---|---|---|---|
| Jacobi momentum polar: `‖p_ρ‖, ∠p_ρ`, `‖p_λ‖, ∠p_λ` | `hypot`, `atan2` on `(q₀, q₁)`, `(q₂, q₃)` | closed on the pair; it's polar on the pair map (§ 5.1) | `‖·‖` bounded; `∠` periodic, **angle-like** | the other vector | a component axis of the same vector with two of these | none named | **axis**, no new primitive: a pair map |
| body `i`'s speed, heading in the canonical frame | `‖pᵢ‖/mᵢ`, `atan2(pᵢ)` | closed, the other Jacobi vector frozen; two body speeds as a pair are a circle intersection, two roots or none | bounded; periodic, **angle-like** | `p_λ` for bodies 0, 1; `p_ρ` for body 2 | body 2 is `‖p_λ‖/m₂`, `∠p_λ`: **an alias**, `SameQuantity` | none named | **axis**; body 2 aliased. Heading is relative to `ρ̃` on `+x`, so under a shape axis it turns with the frame |
| flight-path angle `γ_ρ`, `γ_λ` | `atan2(x × p, x·p)` per Jacobi vector | closed, config downstream | periodic, **angle-like** | the speed | `γ_ρ = ∠p_ρ`, since `ρ̃` is on `+x`: **an alias**. `γ_λ = ∠p_λ − β` | none named | **axis** for `γ_λ`; `γ_ρ` aliased |
| third body's speed against escape | `√chazy_qk[2]` (dd_chazy § 3.1, `out_kup`), `= √(K_λ‖λ‖/μ_λ)` at `I = 1`, `M = 1`: below 1 bound, 1 parabolic. A test-particle `v_esc` reads 0.816 at marginal escape, equal masses: wrong at finite mass (P-note 2) | closed, direction frozen | bounded | direction, `p_ρ` | `‖p_λ‖` at a frozen shape, `SameQuantity` | none named | **axis**. It isn't `k` (R-669's fixed-`k` is the whole system's rigid rotation; RQ-725) |
| `K`, `PE` | § 3 | `K` closed; `PE` **none** (REQ-CHART-070) | § 3 | § 3 | § 5.5 | as § 3 | **existing**; `PE` stays no axis until a warp and residual are written |
| `K_ρ`, `K_λ` | `‖p‖²/2μ` per Jacobi vector | closed, scale of that vector | bounded | direction | rank 2 with any of `K_ρ, K_λ, K`; refused under a hold on `K` with both terms (§ 5.5 row 17) | none named | **axis**, gauge |
| `L_ρ`, `L_λ` | `x × p` per Jacobi vector | closed, one component | affine | the rest of that vector | rank 2 with any of `L_ρ, L_λ, L_z` (row 17) | none named | **axis**, gauge; "inner" is the pair 0, 1 by label |
| `h`, the dilation rate | `Σ m r·v` on the `I = 1` gauged state, RQ-680's scale-free `(Σ m r·v)/I · I^¾` | closed: `v ← v + ((h* − h₀)/I)·r`, `L_z` kept | bounded warped, affine linear, as `L_z` | `L_z`, shape momenta | `K_dil` | state map's targets, REQ-COL-129 | **axis, door RQ-724**, free |
| Saari split `K = K_dil + K_rot + K_shape` | `h²/2I`, `L²/2I`, the rest: `r` and `Jr` are `m`-orthogonal | `K_shape` closed, scale of the rest; `K_dil` loses `h`'s sign; `K_rot` is `L_z` | bounded | the other two terms | three of the four; `(K_rot, L_z)`; `(K_dil, h)` | none named | `K_shape` **axis**; `K_rot`, `K_dil` **aliases** of `L_z`, `h` |
| `L²E` | scale-free combination | closed, `E` or `L_z` frozen; pole at `L_z = 0`; with `E` frozen it loses `L_z`'s sign | bounded | the frozen one | rank 2 with `L_z` or `E`; row 18 | none named | **axis** at `ε = 0`. `ε ≠ 0` sets a length, so it isn't scale-free there: label. A both-held chart is one value of it (§ 5.3) |
| interior angles at A, B, C, any two | § 3 | closed on the pair | bounded, **angle-like** | the shape's other DOF | § 5.5 row 1; any other shape pair | none named | **in the registry**; `γ₀ + γ₁ ≥ π` blank |
| angle of C about AB's midpoint | `atan2(η, ξ)` | closed with a radius: polar on § 8's pair | bounded, **angle-like** | radius | `β` at equal masses, `d = 0`: `SameQuantity` | none named; P-1's polar view is about a body (§ 5.1) | **axis**, a pair map; no new primitive |
| angle of C about AB's COM | `β` | | | | | | **is `β`**: reuse |
| bodies' polar angles about the COM | `φ₁ − φ₀`, `φ₂ − φ₀` | closed on the pair: a 2 × 2 solve from `Σ m r = 0`, then `I = 1` | periodic, **angle-like** | masses | other shape pairs | none named | **axis**, on the 25% of the torus where every gap is under `π`, the COM being inside the triangle; blank elsewhere. Each edge of that region is a single shape, a body at the COM; each corner a line of shapes, a declared seam and `ray-degenerate` |
| signed area | `½ ρ × λ = n_w/(4√(μ_ρ μ_λ))` | closed, with an azimuth | bounded | azimuth | `n_w`; elongation | none named | **axis**; through zero is collinear, so it needs the signed path, or `C`'s mirror folds it (R-544) |
| distance to one named shape | `acos(n̂·n̂_k)` | closed with azimuth: the radius of § 5.2's exponential map | bounded, **angle-like** | azimuth | | none named | **axis**, a pair map about `n̂_k`. Euler `n̂_k` needs a solve per mass |
| distance to the *nearest* of a set | `min` of the above | **none** (§ 3's `min` row) | | | | none named | **overlay only**: not an axis, seams at the bisectors |
| elongation, principal-moment ratio | `I₁,₂ = (1 ∓ √(1 − n_w²))/2`: `I₁I₂ = n_w²/4` | closed, one orientation | bounded | azimuth | **signed area's size: one quantity at `I = 1`** | none named | **alias** of `\|n_w\|`, warped |
| inner share of `L_z` | `L_ρ/L_z` | closed, `L_z` frozen; pole at `L_z = 0`, tagged | affine | `L_z` | row 18 | V-11, parked | **axis**. Counter-rotating is `s < 0` or `s > 1`, not the sign of `s` (53% of counter-rotating draws have `s > 0`, P15) |
| inner, outer elements `a, e, ω, f` | osculating, pairs by label | closed on a pair, a radial-sign branch; mean anomaly numerical | | | | V-11, parked; SL-1, TR-1, MK-1 "no chart" | **overlay only, door RQ-723** |
| periapsis ratio | `a_out(1 − e_out)/a_in` | none | | | | V-11, parked | **overlay only**; undefined at `a_in ≤ 0` or `e_out ≥ 1` |
| distance from Mardling–Aarseth | `(R_p^out/a_in)` against `2.8[(1 + q_out)(1 + e_out)/√(1 − e_out)]^0.4 × factor` (dd_chazy § 3.5), a labelled pair | none closed | | | | V-11, parked | **overlay only**: R-668 parks hierarchical triples. Blank unless `a_in > 0`, `a_out > 0`, `e_out < 1` |
| distance from Hill stability (Marchal, Saari, Zare) | `c = L²\|E\|` against `f_k`, `f = IU²/2` at the Euler configuration with body `k` in the middle: **three thresholds at unequal masses**, `25/972` at equal masses, `M = 1`. Which applies: **door RQ-730** | at a frozen mass a warp of `L²E`; moving masses `numerical`, the Euler quintic per pixel | bounded | as `L²E` | as `L²E` | none named | **alias** of `L²E` at a frozen mass, once ruled; a definition requirement. Blank at `E ≥ 0`; at `ε ≠ 0` blank, the threshold no longer a function of `c`; `L_z = 0` reads −1, true. Label: Hill-stable means no exchange, not bound: the third body may still escape (P18) |
| `SuvakovPlane`, two velocity parameters | REQ-CHART-091 | closed, baked by R-669 | its definition | R-630's labels: the middle body is body 1, outer bodies at `∓1`, equal velocities, the middle body `−2` times them at equal masses. `m₃` is the middle body's mass in the catalogue's labels, whatever R-630's labelling calls that body: at fixed `m₃` the middle body moves `−2/m₃` times them by `P = 0`, as TASK-M9-40's definition must state | a shape axis: collinear | HU-1, HU-2, AT-D1, AT-D3, OC-4, SD-1's closure | **stays one M9 task** (TASK-M9-40): a reference here, nothing built |
| isosceles families with matched velocities | **no definition in the corpus** | | | | | none named | **door RQ-725**; a definition requirement |

**The verdicts that aren't as asked.**

- **Mardling–Aarseth isn't an axis.** Its form is in the corpus, dd_chazy § 3.5, with its domain, "Not a hierarchy unless
  `a_in > 0`, `a_out > 0` and `e_out < 1`", so an overlay needs no new source, and that domain is its blank. A numerical
  inverse may exist; the reason it is no axis is R-668's parking of hierarchical triples.
- **Elements aren't results, but they aren't axes yet.** Osculating elements exist at `t = 0` for any state, by the labelled
  pair. They mean an orbit only with a clear hierarchy, and choosing the pair by nearness adds a seam. As axes they would
  be the parked hierarchical slice, and the R-669 binary-plus-single chart by another name. As overlays they're forward only,
  and build no chart.
- **Hill stability** is sound under `I = 1`, since `L²E` is scale-invariant, and the theorem holds for all masses, but it gives
  three numbers at unequal masses: the allowed region is `{f ≥ c}` on the shape sphere, saddles at the three Euler
  configurations, and pair `(i, j)` is cut off iff `c > max(f_i, f_j)`, the region first disconnecting at the median `f_k`.
  At `m = (.6, .3, .1)`, 0.011599, 0.012261, 0.012917. RQ-730. Under softening `I K ≥ L²/2` survives; the threshold stops
  being a function of `c`. The maths enters the chart reference as a definition under physics review; attribution needs the
  paper read (candidates unverified, `prim_round_trip`'s control the restricted-limit Jacobi constants and `25/972`).
- **Saari against RQ-680.** `Σ m r·v` is RQ-680's rate only on the `I = 1` gauged state, and `K_dil = h²/2I`. So the split gives
  `h` its meaning, not what option (b) lacks, its "`h` axis, range and link". SM1 works from the encode of a live state, so the
  forward runs there or in the scale-free form; "never left" makes `h` the whole real line as a target; and REQ-COL-129's
  `t = 0` identity tolerance for `θ × h` needs a decode with residuals stated (`φ`, `L_z`, `K_shape`, masses): CPU batch can
  serve, and the definition must say which. **Against REQ-CHART-051:** no conflict. `d_E` removes `Jr` only, so it raises `K_dil` and `K_shape` together; `d_h ∝ r`
  would be a new named direction, defined the same way.

**Rejected as axes**, as the order states them, each checked:

| Rejected | Why | Check |
|---|---|---|
| total momentum | zero by COM | `C`'s boost; `Σp = 0` by construction |
| absolute direction | rotation gauge | `C`'s rotation |
| anything size-only | `I = 1` | `SizeOnly`; a computed test (§ 12) |
| `z` components | planar | no third coordinate exists |
| discrete labels | colourings, not axes | no inverse: `none`, overlay; the third-body pair's fold stays a label |
| anything measured after `t = 0` | results, not axes | an escape time or a final eccentricity is a coloured output |

### 3.2 The primitives of the full order

The owner's latest order of 6 Nov, part C, its longer list. Same columns as § 3.1, the same lowering (CPU batch, labelled), the same
`prim_round_trip`. The paths are records, not rulings: where one differs from a contract or `decisions.md`, that governs.
`INC` is `docs/incoming/`; `PP` is `INC/2026-10-25_replication/papers/`; a path under `INC/_local/` is held locally and never
committed. Identities: `docs/incoming/2026-11-06_chart_composition/checks/a5_check.py`.

| Primitive | Forward, source | Inverse | Extension | Holds | Contradicts | Targets | Verdict |
|---|---|---|---|---|---|---|---|
| velocity angle `Ψ`, a body's velocity against its binary's line | `Ψ = ∠p_λ` at `t = 0` for the labelled pair, `ρ̃` on `+x`. The paper reads it at minimum `I`. `INC/2026-10-27_papers/TARGETS.md` § AAOS 1994b | closed | periodic, **angle-like** | `‖p_λ‖` | `∠p_λ`: `SameQuantity` | Q-2 reads it at min `I`: a result | **alias** at `t = 0`; the paper's `Ψ` is a result, rejected as an axis |
| isosceles vertex angle, a 1-D family | `γ₂ = 2 atan(1/(2η))` on `ξ = 0`. `INC/2026-10-28_external_sources/tanikawa_umehara_abe_1995/`, Tanikawa, Umehara & Abe 1995 | closed | bounded, **angle-like** | `ξ = 0`, geometric at any mass | `η` on that line | TUA-1, proposed | **axis** of a 1-D chart (§ 10.1); `η` warped. In D, 60° to 180° only; below 60° is the arc or past the mask |
| homology radius `R_H` | shortest over longest side. `PP/manwadkar_trani_leigh_2020_MNRAS_497_3694_arXiv2004.05475.pdf` | **none** over the sphere: `min/max` (§ 3); in D, closed | bounded | azimuth | in D, side ratio `r₁₂/r₀₁`: `SameQuantity` | none named | **alias** of `r₁₂/r₀₁` inside D; overlay elsewhere |
| focal coordinates `Z`, `σ` | `Z = \|r₁ − r₂\|/(r₁ + r₂)`, `σ = (r₁ + r₂)/r₁₂`, foci `A₁`, `A₂`. `TARGETS.md` Q-2, Q-4 | closed on the pair: `τ = ±Zσ`, `ξ = στ/2`, `η = ½√((σ² − 1)(1 − τ²))`, on `(ξ, η)` to 8·10⁻¹⁴ over D at `η ≥ 10⁻³`, and about 8·10⁻¹⁷/`η` below it: `σ = 1` is `η = 0`, where the root's error goes as `1/η` (7.4·10⁻¹⁴ near `η = 10⁻³`, 6.6·10⁻¹¹ near `10⁻⁶`, `a5_check.py`); `ξ ≥ 0` in D fixes the sign | bounded | | another shape pair | Q-4's null; Q-2 reads `Z` at min `I`: a result | **axis**, a pair map. `σ = 1` is the segment `A₁A₂`, one-to-one in `ξ = Z/2`, a fold of the map, not ray-degenerate; `η` loses digits there, so f32's round trip takes a band. In D `σ ≤ 2` and the domain isn't a product |
| Routh `γ = Σ m_i m_j/M²` | threshold `1/27`. Source record: `INC/2026-11-02_library_triage/triage-81-153.csv` (Nauenberg 2002, row 104) | closed with one mass ratio held, a two-root branch | bounded | that ratio | other mass axes | none named | **axis** of mass; not unique alone |
| elliptic Lagrange `(e, γ)` | equilateral shape, homographic motion of eccentricity `e` | closed. A periapsis start is `h = 0`, rigid rotation, `Q = 2k = 1 + e`, apoapsis `1 − e`; the choice stays a **definition requirement** | bounded | shape | any shape axis | none named | **preset**, on CPU batch, once defined. One shape, so not R-669's plane: RQ-718 names it |
| binding ratio `b = q/(1 + q)` | `q = K/U` of a pair, dd_chazy § 3.1 | **not unique**; for pair 0, 1 at a frozen shape, closed through `K_ρ` | bounded | shape | `K_ρ`: `SameQuantity` at a frozen shape | none named | **overlay**; alias of `K_ρ` warped under that held set |
| tidal ratio | `2m_b/(m_i + m_j)·(r_pair/d_out)³`, dd_chazy § 3.6, § 3.7 | **not unique** | | | at rest a function of `ρ_b` | none named | **overlay** |
| hierarchy ratio `ρ_b` | `d_out(b)/d_pair`, dd_chazy § 3.1 | closed with azimuth, per labelled `b` | bounded | azimuth | `ρ₂ = ‖λ‖/‖ρ‖`, § 3: `SameQuantity` | none named | **alias** for `b = 2`; the same row relabelled for 0, 1. "Most separated body" is a label: overlay. dd_chazy § 3.8's labels likewise |
| Donnison's `(μ, Δ)`, `(q₂/a₁, μ*)` | definitions unread: `INC/2026-10-31_library_triage/triage-14-80.csv`, `triage.csv` row for Donnison & Mikulskis 1992, PLOT-ONLY, not on hand | | | | element axes | parked with V-11 | **overlay**, variants of the element rows; a definition requirement; RQ-723 |
| `v_∞/v_crit` | `v_crit` the speed of zero total energy. `PP/trani_etal_2024_AA_689_A24_arXiv2403.03247.pdf` App. A; Ginat & Perets 2021, `INC/2026-10-31_fetch_list/README.md` | closed, by the pull-in below | bounded | binary | | TR-1, not scored | **door RQ-726**: definition only |
| `b/b_max`, signed | planar sign is pro- or retrograde; `b_max` **undefined in the corpus** | closed | bounded | binary | | TR-1, not scored | **door RQ-726** |
| binary phase | at `t = 0` for a labelled pair | closed on a Kepler branch; mean anomaly numerical | periodic, **angle-like** | | R-669 | TR-1, MK-1, not scored | **door RQ-726** |
| orientation `u` | Heggie & Sweatman 1991; full setup unread, § 20 | | | | R-669 | M9, not scored | **door RQ-726** |
| `d/a`, single at rest | `PP/manwadkar_kol_trani_leigh_2021_MNRAS_arXiv2101.03661.pdf` § 3: circular binary, single at rest at 100 AU, `a` 5 AU | closed | bounded | true longitude | `λ`'s polar at rest | MK-1, not scored | **door RQ-726** |
| parabolic `(q/a, apsidal angle)` | Harrington 1970, `INC/2026-11-02_library_triage/triage-81-153.csv` row 63 | closed | | | R-669 | M9-41, not scored | **door RQ-726** |
| Tanikawa–Mikkola `(α₂, β₁)` | `V₁ = −2α₂R₁ + β₁R₂`, `V₂ = α₂R₁ − 2β₁R₂`, `V₃ = −V₁ − V₂`, `Rᵢ` from the true centre of mass. `INC/2026-10-29_research_batches/tanikawa_mikkola_2023/decode_check.txt`; § 6.1's form, not Table 8's caption | closed: linear in `(α₂, β₁)` | affine | `L = 0` at any mass by algebra: with `A = R₁ × R₂`, the mass-weighted sum cancels through `m₃R₃ = −m₁R₁ − m₂R₂`, which needs `R` from the true centre of mass (from the centroid it fails: −0.0025 at `(0, 0.3)`, `(0.1, 0)`, `(1, 0.8, 0.6)`; `docs/incoming/2026-11-06_chart_composition/checks/tm23.py`, `a5_check.py`). The paper's form at equal masses, our extension otherwise, unscored. `P = 0` at equal masses only, `C`'s boost restoring it without moving `L` | hold `L* ≠ 0` | TM23-1, proposed | **axis pair**, the momentum block's construction, § 8.6's rescale per pixel. **Not fixed `k`**: no conflict with R-669 |
| Šuvakov plane, `m₃` free and the middle-body-labelled-2 variant | § 3.1's row: `m₃` the middle body's mass in the catalogue's labels, whatever R-630's labelling calls that body, and the middle body `−2/m₃` times the outer velocity by `P = 0`; the variant is a relabelling | | | | | as § 3.1 | **stays one M9 task**; the variant is its preset |
| Li & Liao 2017 generalised plane | `r₁ = −r₂ = (x₁, x₂)`, `r₃ = 0`, `ṙ₁ = ṙ₂ = (v₁, v₂)`, `ṙ₃ = −2ṙ₁`. `PP/li_liao_2017_SciChina_60_129511_arXiv1705.00527.pdf` | | | | `(x₁, x₂)` are scale and rotation: **size-only and gauge** after `C` | LL's rows | **`SuvakovPlane` after `C`**: its only valid 2-plane is that plane. No new row |
| Li–Li–Liao 2020 collinear `L = 0` `(x₁, v₁)`, `(m₁, m₂)` plane | `r = (x₁, 1, 0)` on `x`, perpendicular `v`, `v₂ = −m₁v₁x₁/m₂` (0.4531080538 reproduced), `v₃` by `P = 0`. `PP/li_li_liao_2020_arXiv2007.10184.pdf` | closed | bounded | `L = 0`, collinear | any shape pair | LLL-1, "not scored: no continuation" | **axis pair**, unscored; R-669's "one M9 task" doesn't reach it |
| BHH collinear-perpendicular Jacobi `(a, c; L)` | definition unread: `INC/2026-10-26_orbit_catalogues/CITATIONS.md` `bhh2026`, an atlas note | | | | | atlas rows | **a definition requirement** (R-72) |
| isosceles choreography `c₁…c₄` | `x₁ = −2c₁, x₂ = x₃ = c₁, y₂ = −y₃ = c₂, u₁ = 0, u₂ = −u₃ = c₃, v₁ = −2c₄, v₂ = v₃ = c₄`. `INC/2026-10-27_hp_ics/README.md`, Simó's normalisation | closed on a 2-plane | bounded | three DOF after `C`: `c₁`'s scale is gone | | Simó's rows | **axis pair** on a named 2-plane, any two of the three |
| Titov isosceles, collinear | `INC/2026-10-28_external_sources/titov_2024/ch5_decoding_check.txt`; `INC/_local/titov_2024/disser_titov.pdf` | closed | | unit masses; the paper's `h = −1` is a unit conversion (§ 20), not a chart hold | as McGehee | TI-1, proposed | **preset** of the row below |
| Orlov collinear `(k, φ)`, at the paper's fixed `E` | virial `k = K/\|U\|` and a velocity angle, as the central body passes through the COM with all three on a line. **Two scans:** rectilinear, `tan φ = ṙ/ρ̇`, `INC/_local/research_batches_2026-10-29/further_research/Orlov_2002_Linear_Triple_Encounters.pdf` p. 933, eqs. 4 to 9, `INC/2026-10-29_research_batches/orlov_2002/`; planar, `φ` the central body's velocity against the line, `INC/_local/martynova_orlov_rubinov_2009_AstRep_53_710.pdf` p. 2 | closed | `k` bounded in `(0, 1)`, `φ` periodic | shape and masses. **Not `E`:** at `I = 1` the shape fixes `U`, so `E_c = (k − 1)\|U\|` moves with `k`, and a hold `E*` is `AxisAndHold`. The paper's `h = −1` is a unit conversion (§ 20), per start: `ℓ(k) = G M²(1 − k)\|U_c\|/\|h\|`, `(5/√6)(1 − k)` at OR02-1's unit masses, with `t` and length by § 8.6, so its `t = 100` runs from 60.3 to 5.94·10⁴ in `t_c` (RQ-643) and the chart carries `paper scale differs` | `Q = 2k` | OR02-1, proposed | planar: **polar view of the Šuvakov plane** at equal masses, unscored until TASK-M9-40. Rectilinear: **not that plane**; it is a pair on the collinear momentum block, `p_ρ` and `p_λ` along the line, `φ` the paper's angle and not the mass-weighted polar one, unscored, no task. The two meet only on Schubart's line, Orlov's `φ = 135°, 315°`, the planar `φ = 0` |
| Mikkola–Hietarinta `(R, θ)`, at the paper's fixed `E` | their eq. 7; `INC/2026-10-29_research_batches/mikkola_hietarinta_1989/` | definition unread on the page | | the paper's `E = −1` is a unit conversion (§ 20), not a chart hold; what the chart holds is the definition requirement's | | MH89-1, proposed | **a definition requirement** |
| McGehee-type, collinear and isosceles | `T = ½(ρ′² + ¼ρ²E′²)`, Titov ch. 5, checked in `ch5_decoding_check.txt` | closed | | `I = 1` fixes `ρ` | `ρ` is **size-only**; `ρ′` is `h`; the angle is the arc on `n̂` | TI-1 | **aliases**: shape arc and `h`. Regularised time is no axis |

**Against R-669 and RQ-593.** The order's "feeds RQ-593's BinaryPlusSingle chart" is a real conflict. R-669: "no binary-plus-single
chart is built, and each reads "not scored: no chart"", closing RQ-593 at (c). Six axes of a scattering set over a binary and a
single **are** that chart, whatever the builder calls it. So the rows above are definitions and forward-only overlays until
RQ-726 is ruled; the ruling stands until changed. Nothing here builds around it.

**The pull-in, the one stated convention** (for RQ-726, unbuilt): a state "at infinity" `(v_∞, b, phase)` is brought to a finite
`d₀` along the hyperbola of the single about the binary's centre of mass, total mass `M`, `G = 1`: `v₀² = v_∞² + 2M/d₀`,
`h = b·v_∞`, the true anomaly from the conic. The flight time from infinity diverges, so **the phase is defined at the hyperbola's pericentre passage**, and carried back to `d₀`
by the finite flight time, a Kepler solve (physics note 3, B1). `d₀` is recorded in the spec. The `I = 1` rescale runs after it, and so does § 8.6's per-pixel time. The tidal error at `d₀` is the convention's and
must be stated.

**Fixed `k`.** Tanikawa & Mikkola 2008 § 3.1 and § 4.2, `PP/tanikawa_mikkola_2008_arXiv0802.2465.pdf`, give "maximal angular momentum
for a given configuration triangle" and "virial coefficient k = T/|U|": the rigid rotation `v = ωJr`, `ω² = 2k|U|/I`, `Q = 2k`,
over `D₁₁ ∪ D₁₂`. It is in the repo, not in the 2015 paper REQ-VAL-326 names (RQ-731). A chart of it **is** R-669's refused
"fixed-k velocity chart": it stays unbuilt and TS-2 stays "not scored". `(α₂, β₁)`, the third-body ratio and a user's rest arm
under hold `L*` (a rigid rotation at constant `L`, not `k`) aren't it, and RQ-718 and RQ-722 name that neighbour.

**Rejected as axes, the longer list.** As § 3.1, and from the order's latest words: size-only includes **Chitan et al.'s
mass/distance-unit maps** (`INC/_local/chitan_myllari_haque_2022_MNRAS_509_1919.pdf`), which rescale the same system; discrete labels
include **the L/M/A/H regions**, overlays or colourings at most, and undefined until § 20's gap closes; measured after `t = 0` includes
the papers' `Ψ`, `Z`, `Y`, `V_b` and `V_e` at minimum `I`. Catalogue energy normalisations aren't axes: § 20.

### 3.3 Every primitive's class

The chart-space model sorts quantities into three classes (`docs/design/principia_chart_space_model.md` § 6b, the owner's
order of 7 Nov): **state functions**; **conserved quantities**, exact or approximate, labelled; and **trajectory
functionals**, which are many-to-one and never an axis. This table applies § 6b to the registry; § 6b is the source for
what the classes mean. Each row carries its class in the registry (`primitive_class`, § 12), so a functional has no way in.

`Source` is this file, `docs/design/principia_chart_composition.md`, by section and line. **Exact** means constant under the
equations of motion in physical units, at `ε = 0`, a run's drift being integration error; in the `I = 1` gauge only `L_z²E` and the sign of `E` stay fixed (the model's §§ 0.5 and 6b). **Approximate** means steady only in a named limit, labelled
`approximate`. **Unclassified** means the row's definition is unread, so it stays a definition requirement and no axis.

| Source | Primitive | Class | Note |
|---|---|---|---|
| § 3, line 132 | latent direction `q` | **state** |  |
| § 3, line 133 | `α`, `β` | **state** |  |
| § 3, line 134 | shape `θ`, `φ` | **state** | the model's own example |
| § 3, line 135 | Jacobi `p_ρ`, `p_λ` components | **state** |  |
| § 3, line 136 | third body `(ξ, η)` | **state** |  |
| § 3, line 137 | interior angles `γ₀, γ₁` | **state** |  |
| § 3, line 138 | side ratios | **state** |  |
| § 3, line 139 | Jacobi ratio `‖λ‖/‖ρ‖` | **state** |  |
| § 3, line 140 | mass ratio `m_b/m_a` | **conserved, exact** | a parameter: no motion moves it |
| § 3, line 141 | mass control `z_μ` | **conserved, exact** | as the mass ratio |
| § 3, line 142 | Burrau `ν`, `θ` | **state** | `ν` with its masses following it |
| § 3, line 143 | body position, `BodyPlane` | **state** |  |
| § 3, line 144 | `E` | **conserved, exact** at `ε = 0` | at `ε ≠ 0` this axis is the Newtonian energy on the `(L_z, E)` chart, which softened dynamics doesn't conserve; the run's own `E₀` is conserved and isn't this axis (R-791). The other `E` axes: RQ-709 |
| § 3, line 145 | `K`, `KE` | **state** |  |
| § 3, line 146 | `L_z` | **conserved, exact** |  |
| § 3, line 147 | virial ratio `Q` | **state** | warped or not; `Q above the floor` is a state function too |
| § 3, line 148 | area, perimeter | **state** |  |
| § 3, line 149 | `PE` (`U`) | **state** |  |
| § 3, line 150 | `min(r_ij)/max(r_ij)`, any `min` or `max` of primitives | **state** | at one instant. Not the minimum separation over a run, which is a functional |
| § 3, line 151 | pair binding energies | **state** |  |
| § 3, line 152 | `I`, `R`, `ℓ` | **state** | size-only, refused as axes. Fixed at `t = 0` by the gauge, and **not conserved**: `F_t` moves them (the model's § 6b) |
| § 3.1, line 186 | Jacobi momentum polar | **state** |  |
| § 3.1, line 187 | body `i`'s speed, heading | **state** |  |
| § 3.1, line 188 | flight-path angle `γ_ρ`, `γ_λ` | **state** |  |
| § 3.1, line 189 | third body's speed against escape | **state** |  |
| § 3.1, line 190 | `K`, `PE` | **state** | as § 3 |
| § 3.1, line 191 | `K_ρ`, `K_λ` | **state** |  |
| § 3.1, line 192 | `L_ρ`, `L_λ` | **state** | only their sum is conserved |
| § 3.1, line 193 | `h`, the dilation rate | **state** |  |
| § 3.1, line 194 | Saari split | **state** |  |
| § 3.1, line 195 | `L²E` | **conserved, exact** | a product of two exact invariants, and scale-free, so at `ε = 0` exact in the `I = 1` gauge too. `ε ≠ 0` sets a length, so it isn't scale-free there: line 195's label |
| § 3.1, line 196 | interior angles at A, B, C | **state** |  |
| § 3.1, line 197 | angle of C about AB's midpoint, about AB's COM (line 198) | **state** |  |
| § 3.1, line 199 | bodies' polar angles about the COM | **state** |  |
| § 3.1, line 200 | signed area | **state** |  |
| § 3.1, line 201 | distance to one named shape, to the nearest of a set (line 202) | **state** |  |
| § 3.1, line 203 | elongation, principal-moment ratio | **state** |  |
| § 3.1, line 204 | inner share of `L_z` | **state** |  |
| § 3.1, line 205 | inner, outer elements `a, e, ω` | **conserved, approximate** | state at any instant; `a` steady only in a hierarchy, `e` and `ω` only below the secular scale (`ω` drifts on it, and `e` moves under von Zeipel–Lidov–Kozai cycles), labelled `approximate`. Overlay only (RQ-723) |
| § 3.1, line 205 | true anomaly `f` | **state** | the fast angle, as the binary phase. Overlay only (RQ-723) |
| § 3.1, line 206 | periapsis ratio | **conserved, approximate** | as the elements. Overlay only |
| § 3.1, line 207 | distance from Mardling–Aarseth | **conserved, approximate** | as the elements. Overlay only |
| § 3.1, line 208 | distance from Hill stability | **conserved, exact** | an alias of `L²E` at frozen masses |
| § 3.1, line 209 | `SuvakovPlane` | **state** |  |
| § 3.1, line 210 | isosceles families with matched velocities | **unclassified** | no definition in the corpus (RQ-725) |
| § 3.2, line 254 | velocity angle `Ψ` | **state** | at `t = 0`. The paper's `Ψ`, read at minimum `I`, is a **functional**, already rejected |
| § 3.2, line 255 | isosceles vertex angle | **state** |  |
| § 3.2, line 256 | homology radius `R_H` | **state** |  |
| § 3.2, line 257 | focal coordinates `Z`, `σ` | **state** | at `t = 0`. The paper's `Z` at minimum `I` is a **functional**, already rejected |
| § 3.2, line 258 | Routh `γ` | **conserved, exact** | masses only |
| § 3.2, line 259 | elliptic Lagrange `(e, γ)` | **state** | `e` a parameter of one homographic family, `γ` as Routh's |
| § 3.2, line 260 | binding ratio `b` | **state** |  |
| § 3.2, line 261 | tidal ratio | **state** |  |
| § 3.2, line 262 | hierarchy ratio `ρ_b` | **state** |  |
| § 3.2, line 263 | Donnison's `(μ, Δ)`, `(q₂/a₁, μ*)` | **conserved, approximate** | variants of the element rows, definitions unread; to confirm |
| § 3.2, line 264 | `v_∞/v_crit` | **conserved, approximate** | an asymptotic two-body speed, read at a finite distance (RQ-726) |
| § 3.2, line 265 | `b/b_max` | **conserved, approximate** | as `v_∞` (RQ-726) |
| § 3.2, line 266 | binary phase | **state** | at `t = 0` (RQ-726) |
| § 3.2, line 267 | orientation `u` | **unclassified** | setup unread, § 20 (RQ-726) |
| § 3.2, line 268 | `d/a`, single at rest | **state** | (RQ-726) |
| § 3.2, line 269 | parabolic `(q/a, apsidal angle)` | **conserved, approximate** | osculating, as the elements (RQ-726) |
| § 3.2, line 270 | Tanikawa–Mikkola `(α₂, β₁)` | **state** |  |
| § 3.2, line 271 | Šuvakov plane, `m₃` free | **state** |  |
| § 3.2, line 272 | Li & Liao 2017 generalised plane | **state** |  |
| § 3.2, line 273 | Li–Li–Liao 2020 collinear plane | **state** |  |
| § 3.2, line 274 | BHH `(a, c; L)` | **unclassified** | definition unread |
| § 3.2, line 275 | isosceles choreography `c₁…c₄` | **state** |  |
| § 3.2, line 276 | Titov isosceles, collinear | **state** |  |
| § 3.2, line 277 | Orlov collinear `(k, φ)` | **state** | read at the passage through the COM, which is this chart's `t = 0`: a section chosen by the start, not an event found in a run |
| § 3.2, line 278 | Mikkola–Hietarinta `(R, θ)` | **unclassified** | definition unread |
| § 3.2, line 279 | McGehee-type | **state** | aliases of shape arc and `h` |
| § 3.1, lines 238 to 243 (the "Rejected" table) | total momentum | **conserved, exact** | zero, and no axis |
| | absolute direction, `z` components, size-only | not axes | gauge, or absent |
| | discrete labels | not axes | colourings |
| | anything measured after `t = 0`; line 301's list | **functional** where read at an event: the flag below | |
| the model § 6b | minimum separation, `t_end`, outcome class, Lyapunov exponent, syzygy word | **functional** | no row in the registry, and none may be added: colourings and plots of results |

**No axis here is a functional.** No row of §§ 3, 3.1 or 3.2 that has an inverse reads one. The rows whose papers read
at an event (`Ψ` and `Z` at minimum `I`) are already rejected in that form. `plan/tasks/` was searched for lifetime, `t_end`,
FTLE, Lyapunov, minimum, syzygy and outcome beside "axis": the one hit, TASK-M9-26's Lehto lifetime map, is a colouring.

**One flag, not resolved here.** **Line 243 is wider than § 6b.** "Anything measured after `t = 0`" rejects a state function read at a fixed time `T`, which
the model's Theorem 11 admits. The two agree on an event read, such as minimum `I`, which is a functional at any `T`. The
rejection stands until RQ-734 rules (§ 10.3).

---

## 4. The axis layer: warps

The warp is `t ∈ [0,1]` to the axis's value. Four, closed:

| Warp | Formula | Extension it implies | Exists |
|---|---|---|---|
| linear | `lo + (hi − lo)·t` | affine | yes, `Mixed` and R-471 E2 |
| power | `lo + (hi − lo)·t^γ` | bounded below `t = 0` | yes, §2.1's `t^γ_K` |
| log | `lo·(hi/lo)^t`, `lo, hi > 0` | bounded | yes, the Euclid plane's `m(u)`, the mass ratio's `q(t)` |
| feasibility | the axis's own (R-471, R-506, R-708) | the axis's own | yes |

**R-506 stands over all of them.** A physical-quantity axis is warped to its feasible range by default, linear as an
option. "Warped" there means the *feasibility* warp. Power and log are options an axis takes on top where its own warp
is written; on an energy-type axis they are not offered until written (REQ-CHART-070's pattern).

**A warp is not lowered as a kind.** Its choice is a uniform word, as `ShapeMap` is (TASK-M2-24's precedent; lowering
Part 3, "workgroup-uniform"): reversible, V1 in § 13. The linear arm is the expression `Mixed` computes today, the
same operations in the same order, so an existing chart's bits do not move.

---

## 5. The chart layer, and where two axes can't be independent

### 5.1 Pair maps

`(a, b) = M(u, v)` sits before the chart's axes. The default is affine, the identity on the axes, and `M` is part of the
chart's type, baked with it: it is `Φ` structure (lowering Part 3). A generic wrapper, `Paired<C, M>`, follows `Tilted<C, P>`.

| Map | Formula | Extension | Note |
|---|---|---|---|
| affine | `(a, b) = (u, v)` | the axes' | the default; bitwise today's |
| polar about `c` | `a = c_a + r cos ϕ`, `b = c_b + r sin ϕ` | `ϕ` periodic, `r` pole-crossing, partner `ϕ`, or bounded | no new type (R-407 clause 3's review); P-1's rings as a view, not their sampling |
| log-polar | `r = r₀ e^{t}` | `r` bounded | the same, no origin in view |
| exponential | §3.3's form, about `n₀` | bounded | generalised off the sphere, § 5.2 |

Polar and log are general options, so they apply to any pair, and a polar view of `(ξ, η)` about a body is P-1's picture.

### 5.2 Curved slices off the shape sphere

§3.3's exponential map needs a point on a 2-sphere and two tangents. It does not need the shape sphere. Two other
2-spheres sit in the state:

- **Momentum.** At fixed configuration and masses, `{p : L_z = L*, K = K*}` is generically 2-dimensional
  (inverse_encode_contract Part 5, kind 3). It is `v = v^(L) + a·w`, `a` fixed and `w` a unit vector in the 3-space
  orthogonal to `Jr` in `⟨·,·⟩_m`: a 2-sphere, and §2.2's seeded direction is one point on it. The map sweeps `w`.
- **Mass.** `(√m₀, √m₁, √m₂)` is a unit vector, so the simplex is an octant of a 2-sphere. The map runs on the sphere, and
  chart_reference §3.3 stops it only at `r = π`, so it leaves the octant: from equal masses at `r` between 0.6155 and 0.9553.
  A pixel with any component `≤ 0` is the domain's end, blank, out of the count and the total. `mᵢ = nᵢ²` is read only inside,
  where the chart is one-to-one, since `r < π`; read outside it would fold 8 sphere points onto one mass set. At an edge or
  corner the rule is the mass simplex's: validation layer 2's narrowed simplex, buffered `mᵢ ≥ ε_m`.

Each is a definition under physics review, closed by TASK-M2-72 (R-72). Neither is a new chart type: each is
`ExpMap<Target>`, the shape sphere's being `ExpMap<Shape>`, its bits unchanged.

The momentum sphere's frame is built from R-82's seed, which flips with the configuration. With the configuration frozen
across the chart, the frame is constant. Tilted into a configuration direction it can flip, and that is a seam (§ 5.5).

### 5.3 Constraints: hold `E`, and new: hold `L_z`

**Hold `E` is `η_E`, unchanged:** `pᵢ ← √((E* − U)/K₀)·pᵢ` (§0.6). It is a projection after the decode, and it scales
`L_z` too.

The physics reviewer's note of 6 Nov (PR #422, review 5803342655) checked this section and amended it; the amendments are
folded in below, and its script is `docs/incoming/2026-11-06_chart_composition/checks/hold.py`.

**The split.** With `⟨a, b⟩_m = Σ mᵢ aᵢ·bᵢ` and `J(x, y) = (−y, x)`, `L_z(v) = ⟨Jr, v⟩_m` and `‖Jr‖²_m = I`, so every
velocity splits orthogonally:

```
v = (L₀/I)·J r + δv        ⟨Jr, δv⟩_m = 0        K = L₀²/2I + K_δ        K_δ = ½‖δv‖²_m
```

**Hold `L_z` (`L*`)** is `η_E`'s twin, after the decode and before the derived invariants, in the same place. It replaces
the first term:

```
v ← v + ((L* − L₀)/I)·J r            with I = 1 at every decode; Σp stays 0 because the CoM is at the origin
```

That is the `m`-orthogonal projection of `v` onto `{L_z = L*}`, the smallest velocity change `‖Δv‖_m` that reaches `L*`.
It is not a minimal-kinetic-energy state: `δv` stays. (inverse_encode_contract Part 5's theorem, that `v ∝ Jr` minimises
`K` at given `L_z`, is about the fibre's bottom, not this step.)

**Both held.** `η_E` alone would break the held `L_z`. `{L_z = L*, K = K*}`, with `K* = E* − U`, is a sphere of radius
`a = √(2K* − L*²/I)` about `(L*/I)·Jr` in the 3-space orthogonal to `Jr`, and the nearest point to `v` keeps `δv`'s direction:

```
v ← (L*/I)·J r + η·δv        η = √((K* − L*²/2I)/K_δ)        feasibility: 2K* ≥ L*²  (I = 1)
```

This is inverse_encode_contract Part 5's `v^(L) + a·w` with `w = δ̂v`, and the curve `v(s) = v⁽ᴸ⁾ + eˢ(v − v⁽ᴸ⁾)` that
chart_decoder_contract gives for `d_E` (REQ-CHART-051), with `η = eˢ`: the corpus's own decomposition, twice. Checked over
20,000 random states in f64: both targets and `Σp = 0` within 1.1·10⁻¹⁴. With hold `L_z` off this path is not entered, the
choice being on the switch, never on `L* = L₀`, and `η_E`'s bits do not move. This is new physics, off by default:
**door** RQ-715. Three definitions (R-72) go with that door: REQ-CHART-108 to REQ-CHART-110.

**The computed form is not the formula above.** Near a rigid rotation `K₀ − L₀²/2I` cancels, goes negative and makes `η`
non-real; and the Lagrange and Euler relative equilibria are such states, so a chart locked on one sits there. `L_z(δv)` is
rounding-sized, not 0, and `η` multiplies it. So `K_δ` is computed from `δv` itself, in dd_decoder §3.7's scaled form, and
`Jr` is projected out a second time after normalising:

```
δv ← v − (L₀/I)·J r
s  ← largest component of |δv|                       this line and those after it are reached only when not rigid(v, Jr), below
δ̂  ← δv/s      δ̂ ← δ̂ − (⟨Jr, δ̂⟩_m/I)·J r            the second projection
K̂  ← ½‖δ̂‖²_m
v  ← (L*/I)·J r + √((K* − L*²/2I)/K̂)·δ̂
```

The reviewer's f32 figures, worst `|ΔL_z|`, one pass against two: `‖δv‖/‖v‖ = 10⁻¹`: 2.9·10⁻⁶ against 1.8·10⁻⁷; `10⁻³`:
3.2·10⁻⁴ against 1.9·10⁻⁷; `10⁻⁵`: 2.8·10⁻² against 1.4·10⁻⁷; `10⁻⁷`: 1.3 against 4.2·10⁻⁷. An exactly rigid start goes further, and it is the common case: over 200,000 rigid draws, of those applied, worst `|ΔL_z|` 2.1·10⁻⁴ at f32, about 1.6·10⁻⁴ of `a`, and 2.7·10⁻¹³ at f64 (`hold.py`), since a residual of rounding size, after cancelling, has an orthogonality error of its own size. So 4.2·10⁻⁷ doesn't set REQ-CHART-102's tolerance: the rigid row does under RQ-715's option (ii) below, and under option (i) the worst above `τ` is 1.8·10⁻⁷ of `a`. A near-rigid pixel stays
ill-conditioned in direction, as `η_E` is near rest: f32 and f64 can differ there by `O(1)` in `δ̂` with no tag (0.49 median, 1.15 worst at `10⁻⁷`). The tag's form
is **open, under RQ-715, and both options below are Proposed**.

**The tag at outcomes 4 and 5.** dd_decoder § 5's test 12 and REQ-VAL-017 require branch tags bit-identical across backends on identical inputs. A test on `s` or `K̂` can't meet that: both come through divisions at each float's rounding, and `s = 0` and `K̂ = 0` differ between f32 and f64 on 16.5% of 6,000 exactly rigid draws on identical inputs (`docs/incoming/2026-11-06_chart_composition/checks/exact.py`, Dirichlet masses). Nor is `‖δv‖²_m ≤ ε_w‖v‖²_m` an answer, since it makes a branch tag depend on a tolerance, against the comparison-only rule. The table below reads as (i), the recommendation:

- **(i), recommended.** The tag is the exact comparison `rigid(v, Jr)`, never a test on `s` or `K̂`. With `w = Jr`, `p` the first index of the largest `|wᵢ|` and `V` the largest `|vᵢ|`, over the six components: `|v_b w_p − v_p w_b| ≤ τ·V·|w_p|` for every component `b`, each comparison decided exactly from the bits, as `infeasible`, with no division and no mass. `τ` is a dyadic calibration (REQ-CHART-126), proposed `2⁻²²`. Not `rigid` gives `|δv|∞ ≥ τV/2`, so a direction exists. Measured by `exact.py`, 6,000 draws at each of six `‖δv‖/‖v‖` from 0 to 10⁻⁵, five `τ` from `2⁻²²` to `2⁻¹⁸`: no `s = 0` or `K̂ = 0` above `τ`, every exactly rigid draw tagged, worst `|ΔL_z|/a` at f32 above `τ` 1.8·10⁻⁷. The price: a pixel within `τ` of rigid tags at f64 too, though f64 could compute it. At `τ = 2⁻²²` that is every draw at ratio 10⁻⁷, 0.097 of them at `3·10⁻⁷` and none at 10⁻⁶, a labelled disc about `2·10⁻⁷` wide on a chart locked on a relative equilibrium.
- **(ii), the alternative.** Keep `s = 0` and `K̂ = 0`, stopping at each with the outcome, and write an exception as `NO_QUALIFYING_SEED` has: not claimed branch-identical within rounding of rigid. Rows 4 and 5 then read "`s = 0` or `K̂ = 0`", and row 6 "`K̂ ≠ 0`". On the same 16.5% one backend keeps the decoded momenta while the other applies a rounding direction, so the two differ by `O(a)`, with `|ΔL_z|/a` up to 1.6·10⁻⁴ at f32.

Owed with `τ`'s calibration, its criterion written first (R-649): `τ`'s measurement at masses log-uniform to 10⁻³, on the two GPU backends, and the kernel routine's cost. `exact.py` is Dirichlet only, in Python.

**The outcomes, in this order** (REQ-CHART-109). dd_decoder §3.7's list doesn't cover them, and §2's cause set is closed:

| # | Test | Outcome |
|---|---|---|
| 1 | the collision pole | `ENERGY_UNSCALABLE` |
| 2 | `E* < U` | `ENERGY_INFEASIBLE` |
| 3 | `2K* < L*²`, by the existing exact `infeasible` routine, never `E* < U + L*²/2I`, which rounds at the decision | **new**: `ENERGY_INFEASIBLE`, widened, since this stage raises it. §2's table and REQ-DEC-041 need the words, and §3.7's "every `E* ≥ 0` is feasible" is false with `L*` held |
| 4 | `rigid(v, Jr)`, at `2K* = L*²`: a rest or rigid `v` on the floor | no tag, and **the rotation still applies**: a rest pixel with `L* ≠ 0` becomes `(L*/I)·Jr`. §3.7's "momenta untouched" can't be copied |
| 5 | `rigid(v, Jr)`, above the floor | `ENERGY_UNSCALABLE`. No seed: §3.7 tags the rest start and doesn't seed it, and `d_E`'s seed fallback is a tangent, not a state |
| 6 | otherwise | apply; the floor when not `rigid` gives `η = 0`, the rigid rotation |

A tagged pixel keeps its momenta as decoded, with neither hold applied (§2.2's `a = 0` is the construction's, not this
step's). A non-finite `L*` is refused as a non-finite `E*` is, with `physical.rs`'s treatment of an `L_z²` that has no
finite float as §2.1's exception. Every outcome is a label on the decode, never a refusal.

**Lowering.** `L*` is a uniform. `DecodeUniforms.energy` has two spare words and an unused bit, so the switch and the
value fit with no new row, and with the switch off the uniform's bytes are today's. `sample_with` is the one place the
step goes, and the fragment decode calls it. "No new variant" holds only with no tilt target on `L*`, as below.

**Tilt and slice: two different things.**

- *A chart holding `L_z` by construction* (`(ν, K)`, `Φ_θ,K`, `Φ_strip,K`): R-720 applies, by value, `L* + d`, and to
  slice as well: "The same rule governs slicing the held L_z as well as tilting it." The hold is read inside the chart,
  as `held`, and `d` reaches it there. For `MixedEnergy` it is RQ-701's.
- *A free chart* (latent, `Mixed`, `MixedMassRatio`, the shape sphere, region D's eq. 3 family): `L*` is not a hidden
  coordinate. It is a view setting like `E*`, and `E*` has no tilt. A `z` tilt leaves `L*` and `E*` fixed at every pixel,
  which is what "held" means, and the hold runs per pixel on the tilted `r`, `U` and `v`. **Recommended: no tilt target
  on a free chart**; the `L*` slider is a slice, by value. A by-value `L* + d` varying over the window would be a new tilt
  target, like R-554's `p_λ`, and `d` doesn't reach the decode stage: `Shift` targets control indices and is consumed in
  `Tiltable::decode_shifted`, while the hold sits after it.
- **RQ-715 closes the held-`L_z` half of RQ-701 only.** RQ-701's held-`K` half, `K_max` on a lone `L_z` axis, is
  untouched; RQ-679, the slider span, stays as it is (answered since by R-920, 9 Nov 2026).
- Infeasible pixels under a tilt are labels, per pixel. No hold is a warped axis, so R-708's "every pixel a system" doesn't
  bind here.

**Encode (REQ-CHART-110).** inverse_encode_contract Part 5 says encode reuses decode, and Part 6 step 4 tests
`‖D(Φ(c(x))) − C(x)‖_phys`. Under a hold neither is automatic: where `D` is unheld a state lands on a pixel that shows a
different system, and the kernel's encode reads no `e_star` today, so hold `E` has the same gap. So step 4 uses the held
decode, and a state off `{L_z = L*}` gets a stated outcome. Which one is a definition REQ-CHART-110 owes (R-72); this
document doesn't pick it.

**Same refusal rules** (R-25, R-670): the flag `forbids_lz_hold`, true where `L_z` is an axis (`(L_z, E)`, `(L_z, K)`,
`Φ_θ,L_z`, an `L_z`-axis `MixedEnergy`); every `Some(L*)` refused there, `Some(0)` included (§ 6.1). `(ν, K)`, `Φ_θ,K`, the
strips and the `K`, `KE`, `E` and virial `MixedEnergy` charts forbid `E*` and read `L*` as their held value (V10), so the
rotation never runs after a construction and can't break a `K` axis: R-708's floor keeps every pixel a system. Both held is
reachable only on free charts, and no chart was found where holding both contradicts what the chart holds. A softened
run (`ε ≠ 0`) keeps hold `L_z`, since `L_z` does not read `ε`; hold `E` stays refused there (R-670).

**What else it means.** `I = 1` holds where the step runs, after `chart.decode`, so after `C` and the scale gauge; on charts
bypassing the canonical frame, `L*` is read in `C`'s frame, after any mirror. `E·L²` is scale-invariant, so at `I = 1` both
held fixes `L²|E|` over the window even where each paper-unit value varies with `ℓ`; a held `L*` or `E*` on region D takes
the `paper scale differs` label. On the shape sphere under `L* ≠ 0`, `(θ, φ)` and `(θ, π − φ)` aren't mirror images even
for a mirror-symmetric `p_fixed`, since the mirror has `−L*`: that qualifies chart_reference §3's sentence. Area statistics
under a hold stay `measure: chart area`: the projection is not Liouville. A statistic on a shape-sphere chart under a held `L* ≠ 0` so counts two different systems' halves, and its legend must not read as a share of one whole sphere.

### 5.4 What the constraints do to a momentum axis

Hold `E` already moves a chart's momentum axis off its label: `q_k` is what the axis swept, not what is integrated. That
is today's behaviour on `Mixed`. Hold `L_z` does the same. It is reduced capability, so it is a label: "axis reads the
value before the hold", which also names a hidden `p_fixed` or `p_λ` tilt target.

**Rank loss has three cases**, each `ray-degenerate` (§ 5.5, row 9). (i) Hold `L_z`, alone or with hold `E`: the swept plane
contains `Jr`, which is an axis plane at a real configuration, `(p_ρ,y, p_λ,y)` at a collinear shape; singular values 1.35 and
2·10⁻¹⁰ on a general plane containing `Jr`, 4.4 and 3·10⁻¹¹ on that axis plane, 0.70 and 6·10⁻¹⁰ with both held. (ii) Hold `E`
alone: **a plane through rest**. `η_E` is a radial projection centred on rest, so it sends the plane to a circle and the chart is
rays: today, 2.5 and 10⁻¹⁰. (iii) Both held: **a plane through a point with `δv = 0`**, rest or a rigid rotation, since the scale
is centred on `(L*/I)·Jr`: through rest, 5.3 and 2·10⁻¹⁰; through a rigid rotation, 5.3 and 6·10⁻¹⁰. Rest is the default `z₀`, so a
momentum-momentum plane at rest under both holds is one curve of systems. **A plane through a rigid rotation that misses rest
keeps rank 2 under hold `E` alone** (2.0 and 0.99), since rest, not the rotation, is that projection's centre: over 2,000 random
states and planes it never dropped rank, and with both held it always did. A generic plane keeps rank 2: 6.2 and 0.45 both
held, 2.3 and 0.85 under hold `E` alone. Hold `L_z` alone through rest keeps it too (1.7 and 1.0): it is a shift, not a scale.

### 5.5 Every case where two axes can't be independent

The order: "Document every case where two axes cannot be independent instead of hiding it". They fall in three groups:
the same quantity (refused, § 6.1), coupled by a map (the pair is legal, and it says so), and coupled by the state.

| # | Case | Why | What the chart does |
|---|---|---|---|
| 1 | Two of `γ₀, γ₁, γ₂` | `Σγ = π`, so the third is fixed | any two are the shape's two DOF; a third is no axis |
| 2 | Side ratios | `(r₀₂/r₀₁)·(r₁₂/r₀₂) = r₁₂/r₀₁` | two free; the triangle inequality couples their domain: a non-triangle is blank |
| 3 | `(L_z, E)`, `(L_z, K)` | `\|L_z\| ≤ √(2IK)`, §2.1 | the feasibility warp is a **pair** warp: `L_z`'s line reads `t`. Not per-axis, and it can't be |
| 4 | A held `L_z` under a `K` axis | `K ≥ L_z²/2I` | R-708's additive floor: the axis's line reads the held value |
| 5 | Pole-crossing | R-407: "the partner axis shifted by half its period" | the extension couples the axes past the square |
| 6 | Polar at `r = 0`, exponential at `r = π` | the angle has no meaning there | `ray-degenerate` row or disc; `−n₀` counted once (REQ-CHART-078) |
| 7 | Kind 4 | one `ν` slaves config and mass | a config or mass axis beside it is a same-block collision, `Coupling`'s flag; the slice can't pin those blocks |
| 8 | Axes in one block, generic | `(θ, α)`, `(γ₀, θ)`, `(U, r₀₂/r₀₁)` | independent except on a curve where the Jacobian drops rank: numerical, and the curve is the seam |
| 9 | Two momentum axes under a hold | the swept plane contains `Jr` (hold `L_z`, alone or with `E`), or passes through rest (hold `E` alone), or through a point with `δv = 0`, rest or a rigid rotation (both held) | `ray-degenerate` where it does; a label (§ 5.4) |
| 10 | Region D's edge | eq. 8 couples `ξ` and `η` | not a product domain: § 8.4 |
| 11 | Mass axis and a shape held | `m` moves `U` and the geometric shape | the axis states what it holds (§ 3) |
| 12 | Raw `z_μ` axis | softmax couples all three masses | contract Part 4: the line is "identical except one mass logit" |
| 13 | Hold `E` and hold `L_z` | `2K* ≥ L*²`, `K* = E* − U` | a feasibility curve across the chart; a label per pixel |
| 14 | Interior angles at a collinear point | `γ₀ = γ₁ = 0` is every point between the two bodies | the angle pair can't tell collinear shapes apart: that corner is `ray-degenerate`, and a label says so |
| 15 | Any `gauge` axis against the paper's quantity | `I = 1` rescales per pixel | a row of `E` here is not a row of a paper's `E`: § 8.6 |
| 16 | Frozen hidden coordinates | six are frozen, each read as fixed | only `gauge` axes change meaning under a slice |
| 17 | Splits of `K` and `L_z` (§ 3.1) | `K = K_ρ + K_λ = K_dil + K_rot + K_shape`; `L_z = L_ρ + L_λ` | any two are free: `(K_ρ, K_λ)`, `(L_ρ, L_λ)`, `(L_ρ, L_z)` are rank 2. Refused only: two terms under a hold on their sum (`SameQuantity`: the hold fixes the second) |
| 18 | `L_z`, `E`, `L²E`; `L_ρ`, `L_z`, the inner share | each of three is a function of the other two | any two are free, off `L_z = 0`; determining a third isn't fixing one twice. `(L_z, E)` is the gallery's own chart. Refused only: the pair under a hold on the third |
| 19 | `\|n_w\|`, elongation; signed area, elongation | `I₁I₂ = n_w²/4` at `I = 1` | `\|n_w\|` and elongation: one quantity, always, refused. Signed area and elongation: one at a frozen mass, refused there |
| 20 | Body 2's speed or heading, `‖p_λ‖` or `∠p_λ`; `γ_ρ`, `∠p_ρ`; `h`, `K_dil`; `K_rot`, `L_z`; `v₂/v_esc`, `‖p_λ‖` at a frozen shape | aliases | refused |
| 21 | `h` and `L_z` under hold `E` | `r` and `Jr` are orthogonal, so independent, but `K_dil + K_rot ≤ K` | feasible region, tagged, as row 3 |
| 22 | Polar angles about the COM at a collinear shape | a line of shapes on one point | `ray-degenerate`, a seam |
| 23 | The mass sphere's octant edge | `Σ mᵢ = 1` with every `mᵢ > 0`, so `√m` stays in one octant while the map runs on the sphere | not a product domain: a pixel with a component `≤ 0` is blank, out of the count and the total (§ 5.2); at the edge, the mass simplex's narrowed simplex, buffered `mᵢ ≥ ε_m` |

---

## 6. Refusals and labels

**Any axis may be paired with any other, of any kind** (§1.3's mixed-axis rule, extended). The list below is closed.

### 6.1 Refusals: genuine contradictions only

Each is a variant of `chart_authoring::validate::Refusal`, in code, with a named reason.

| Refusal (new unless marked) | The contradiction | Example |
|---|---|---|
| `AxisAndHold { axis, quantity }` | an axis and a constraint on one quantity | `L_z` axis + hold `L_z`; `E`, `K`, `KE` or `Q` axis + hold `E` (generalises R-25) |
| `SameQuantity { a, b }` | two axes are rank 1 | `(K, KE)`; `(E, K)`, `(E, Q)` or `(K, Q)` at a frozen configuration; `(α, ‖λ‖/‖ρ‖)`; `(m_b/m_a, m_a/m_b)`; § 5.5 rows 17 to 20 as they refuse |
| `NoJointInverse { a, b }` | the pair has no inverse | either axis's class is `none` |
| `SizeOnly { axis }` | it would be constant | `I`, `R` |
| `HoldAgainstConvention { hold, convention }` | **not settled: RQ-722.** A rest chart under hold `L*` is a well-defined family, each pixel `(L*/I)·Jr`, and reachable anyway through the latent chart at `z_q = 0`, so this is no contradiction in the states; it is the chart's identity | hold `L_z` on a chart declared at rest, by a declared flag: the mass simplex, the Euclid plane, `Φ_strip,δm`, region D's exact-rest arm |
| `View`, existing | R-25, R-670 | a `forbids_energy_normalisation` chart + any `E*`; `ε ≠ 0` + any `E*` |
| `NoWarpWritten`, TASK-M2-37's | no warp written (R-471 H2, R-506 W2) | `PE` |
| `InvariantNotDownstream`, `UnderDetermined`, `DoubleWritten`, `DependentDirections`, `NonFiniteDirection`, `NoSweptAxis`, `Flag`, existing | contract Part 5 | unchanged |

**`UnderDetermined`, `InvariantNotDownstream` and `NoWarpWritten` are not contradictions in the order's sense**, but they
are the contract's check and R-506's words, and those win. In the builder the first can't be reached, since the slice
always fills the DOF an axis leaves.

A chart that holds `L_z` by construction, `(ν, K)` and its strips, is not refused: hold `L_z` there **is** its held value.
`Q` and `K` at a *moving* configuration are still one quantity under hold `E`, since `K = E − U`.

Two axes fixing one quantity is decided by the registry's table, and a test proves the table complete: it estimates the
pair's rank on a grid, and fails any pair of rank below 2 everywhere that the table does not refuse, and any refused pair
of rank 2 (§ 12).

### 6.2 Labels: reduced capability, never a refusal

| Label | Meaning |
|---|---|
| `live` | uniform affine, or a baked entry |
| `CPU batch: not baked` | closed-form, past the cap (§ 11) or not in the baked set |
| `CPU batch: numerical` | a numerical inverse: its branch rule and seam shown |
| `ray-degenerate` | a line of pixels is one system; areas are not system fractions (contract Part 5) |
| `1-swept family` | an annotation axis (§ 10.1) |
| `measure: chart area` | area is not Liouville measure (contract § "Links carry a measure") |
| `branch: <rule>`, `seam` | § 7.3 |
| `axis reads before the hold` | § 5.4, a hidden `p_fixed` or `p_λ` tilt target included |
| `reduced domain` | a paper's domain of starts shown as a labelled view: the mirror for `η ≥ 0`, the body swap at equal masses for `ξ ≥ 0`. On the third-body pair it's the symmetry filter's output and the full disc is its one click off (§ 10.4; RQ-736). The angle pair's axes can't carry the sign, so it keeps the label with no filter (§ 8.8) |
| `one orientation` | an inverse reaching one orientation by its own coordinates, as `\|n_w\|`'s aliases do. Every shape-sphere chart shows the full `θ` and `φ` range (R-544, § 6.6). *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]* |
| `mirror pairs shown once: η ≥ 0` | the filter's statement, one per declared symmetry (§ 10.4) |
| `paper scale differs` | a `gauge` axis at `I = 1` (§ 8.6), a held `L*` or `E*` on region D (§ 5.3), or a chart read against a paper that normalises `E`, so `ℓ` moves across it (§ 3.2's Orlov row) |
| `Q above the floor` | a warped virial axis at a held `L_z ≠ 0`: it reads the excess above the rotational floor, the state's own `Q` in the readout (§ 9) |
| `approximate` | a conserved row steady only in a limit (§ 3.3) |
| `per pixel of this chart` | a statistic not weighted into a stated measure (§ 6.4) |
| `seam-contaminated` | a pixel's spread or `error_ratio`, where its copies straddle a seam (§ 7.4) |

**R-506 W1 unchanged.** A chart with any linear axis logs no decoder-bug warning; a chart whose physical axes are all warped
does. A chart with a mask or a seam logs none on the pixels the mask or seam explains.

### 6.3 The Domain preview shows why

§G7's preview gains one line under the hatch. A refused pair shows its reason's name and text, with no quick render. A
labelled pair shows its label, and a numerical axis its seam, drawn. It reads the `Refusal` and the label, so nothing
is written twice.

### 6.4 Measure: a statistic says what it counts

**Refinement density is not probability density.** The scheduler contract's Part 2 applies to every composed chart
(`docs/contracts/principia_scheduler_contract.md` Part 2): "The quadtree is a rendering / compute-allocation structure.
Its leaf density is `1 − coherence`-driven — high where boundaries are filamentary, low in smooth basins. This is **not**
physical probability density, and it must never feed a quantitative claim." Its rule is "adaptive structure for viewing,
uniform sampling for statistics", so a statistic starts from a flat grid and never from a count of leaves.

A composed or warped chart adds a second bias: its pixels aren't equal in any physical measure (contract § "Links carry a
measure"). So **any statistic over a composed or warped chart** (outcome fractions, `forbidden in view`, replication
scores) is one of two things, and there is no third:

- **Weighted.** Each counted pixel of the flat grid carries the chart's area factor, the Gram determinant `√det(JᵀJ)` of
  § 17.11, `J = ∂Y/∂(u, v)`, into a reference measure the statistic names. `J` is taken on the pixel's own branch, never
  across a seam (§ 7.4). A `ray-degenerate` chart has no such factor, and R-554 rules there: the legend shows each share as
  undefined ("—"), never 0%. No label replaces that.
- **Labelled `per pixel of this chart`.** The count is what it says, and claims nothing physical.

**The label is the default,** since the metric isn't defined yet: `Y`'s metric is a definition requirement (§ 17.11,
REQ-CHART-124). A statistic is weighted only once its measure is defined and named. `measure: chart area` stays the
chart's label, and `per pixel of this chart` is the same fact on the statistic. R-408's rule of what counts is unchanged:
the weight falls on the pixels it counts, and pixels that leave the count leave the total too. The default label lifts no
area weight a view already carries: under Lean each axis the Gram weight stands (R-408, R-554).

**Replication fractions use each paper's own measure,** not the area factor above: a flat grid in the paper's own
coordinates on its domain, or on a warped chart the Jacobian into those coordinates. The target's row names that measure.
One not yet read from the paper is a gap, in § 20's form. A target stays a measurement, never a gate (R-649).

### 6.5 Chaos measures live in phase space

FTLE and Lyapunov exponents are computed from the trajectory in physical phase space, `δ⃗ = x' − x` on one system's
`{rᵢ, pᵢ}` (`docs/design/principia_dd_integrator.md` § 3.8), never in chart coordinates. The chart is over by the time they
start: it picks `x₀`, and nothing after that reads `u`. **So a chart's warp can't change them.** The statement is per system:
two charts that decode one state give it one exponent. A resolved pixel's FTLE anti-aliases over its copies (dd_integrator
§ 3.8), whose offsets are in chart coordinates, so a displayed pixel can differ by chart; only copy 0 is shared.

What follows from that:

- The shadow's `δ₀` is a function of the state, never of the chart. Its direction is to be fixed by dd_integrator § 3.8
  (REQ-INT-077), and it isn't a chart tangent.
- Sensitivity in chart coordinates is a different quantity, and already has a name: the ensemble's spread, which does depend
  on the chart (§ 7.4). Neither is read as the other.
- The gauge isn't the chart. `I = 1`'s per-pixel time (§ 8.6) sets the unit an exponent is in, for every chart alike, so
  against a paper's unit the `paper scale differs` label applies.
- A final-condition chart's backward run (§ 10.3) carries no shadow. The exponent starts at the start it finds.

### 6.6 The symmetry rule

**The standing rule of this design. It arrives by the rulings PR (RQ-736), the owner's words of 7 Nov:**

> symmetry reductions apply only to how a STARTING state is described at t = 0, never to the dynamics or the simulation. Trajectories always evolve in full physical space: nothing is folded, mirrored, quotiented or canonicalised during integration or in anything displayed about a trajectory (for example, the shape-sphere polar angle crosses 90 degrees freely at syzygies). Where a symmetry is used at all (counting mirror pairs once, the IC constructor showing that a triangle and its flip are mirror images), it is a declared, optional view or statistic, never a removal of starting states or angles. Every chart shows the full range of the angles it sweeps.

> Add to the 7 Nov symmetry rule: during integration the only operation besides the step itself is the per-step centre-of-mass projection (integrator contract Part 1), which removes floating-point drift and no physical degree of freedom. Energy and L_z are monitored, never corrected. No rotation removal, rescaling, mirroring or canonicalisation ever happens during a run. Regularised integrators change variables internally for accuracy; they evolve the same physical system.

The model's § 0.6 says the same. What this design's own sections owe: § 8's region D and § 20's presets render the full range and take the paper's domain from the filter (§ 10.4); § 10.3's start stays signed; `trajectory_polar_unfolded` and `path_lower_half_stays` test it. The state map and latent-chart traces are ruled to sign too (RQ-736), and those changes are the rulings PR's, in their own documents.

**The mirror step's removal, ruled 8 Nov 2026 (R-865, on RQ-739), conditional on the bitwise check:** the decoder's mirror step is removed and latent `β` runs over the full circle, so every chart, the shape sphere included, uses one decode path, once TASK-M2-57 has shown that every preset, saved chart and golden with `β ∈ [0, π]` reproduces unchanged; if anything doesn't, the list goes back to the owner before it proceeds. The owner's words add that it also fixes the latent-chart traces' jump at syzygies. Each section here it touches carries *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]*: §§ 3, 6.2, 8.4, 10.3, 10.4, 17.2 and 17.8. They stand as written until TASK-M2-57 lands, and change then. The condition meets the link: `β = π·σ(z_β)` already covers `(0, π)`, so a full-circle link changes every stored latent `z`, and the task waits on the owner's answer, RQ-750. RQ-739 stays open on the proposal's item 3, the optional "mirror pairs counted once" statistic. *Was:* "**The owner's proposal, not applied:** removing the decoder's mirror step, with latent `β` over the full circle. "Mirror-step removal stays a proposal until ruled: don't apply it, just tag the sections it would touch." Each section it would touch here carries *[mirror-step proposal: § 6.6]*: §§ 3, 6.2, 8.4, 10.3, 10.4, 17.2 and 17.8."

*14 Nov 2026 (R-865, R-898, R-919; TASK-M2-107, built by TASK-M2-57):* the removal is applied to the contracts and design docs it names (canonical_spec § 3, chart_decoder_contract Part 2 and Part 5, inverse_encode_contract Part 1, chart_reference § 0.4 and § 3, dd_decoder § 3.3, dd_encode § 3.2, principia_lines § 8's `z_β` row), with R-898's periodic `β` (`z_β` stored unreduced on the lift, read modulo `2π`) and R-919's one `β` link, and R-898's conversion test in place of the bitwise check; the definitions are `principia_dd_decoder.md` § 3.2 (REQ-DEC-056). The sections here marked *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]*, §§ 3, 6.2, 8.4, 10.3, 10.4, 17.2 and 17.8, read with it: where one speaks of `C`'s mirror, it has none; of the encode's `β ∈ [0, π]` or `|λ̃_y|`, the encode's `β` is the signed angle onto `[0, 2π̂]`, `β ∈ (π, 2π)` for a lower-half state; of the shape sphere's separate path or R-544's skip, every chart decodes by the one path. Each section's own text is rewritten by the task that next edits it, keeping its *Was*; its marker stands until then. What they say of the physical mirror `Π` as a symmetry of fates, never an identification, stands unchanged.

---

## 7. Lowering: three ways a pair runs

| Way | Which pairs | What runs | Navigation |
|---|---|---|---|
| **Uniform affine** | both axes latent directions | `Latent`, its `z₀, q₁, q₂` | live, uniforms only |
| **Baked entry** | the gallery's, and the pairs inside the cap | a `#[spirv(compute)]` entry, lowering Part 4 | live, uniforms only |
| **CPU batch** | everything else | f64 on the CPU, each start rounded once to f32, the IC buffer filled, then the integrator | not live: a re-render per view, labelled |

### 7.1 What runs on the GPU

`axis kinds` never reach the kernel, and nothing dispatches on them (REQ-CHART-004). A `DecodeVariant` wrapper, as in
`stage::decode`, is what a baked pair is. The composed chart's `resolve` looks its `(H, V, M)` up in the baked table; a
hit returns the entry, a miss returns `CpuBatch`.

### 7.2 CPU batch

`stage::decode` writes each sample's `(m, r, p)` to a buffer of its own, "the stage's only output to the integrator side".
So the integrator already reads a buffer it does not know the origin of, and CPU batch writes the same words
(`write_ic`'s layout), as R-669's Path B batch does for BZ-1C's starts, plus a `branch` word (§ 7.4).

**Core design seam 3 says** "*Not* a pre-decoded IC buffer — pan/zoom/tilt change every pixel's IC every frame, so precompute
is impossible at scale". That reason holds for a live view, and a batch view isn't one: the label says so, and a gesture
never reaches it. The owner's "CPU batch only" is seam 3's exception: **door** RQ-719.

### 7.3 A numerical inverse: branch and seam

Quads resolve in any order, so a solve can't continue from its neighbour. **A branch rule is a pure function of the pixel.**
So a rule is continuous only away from where branches fold, and **a rule continuous everywhere can't be promised**: the
figure's Tier 2c shows why. Two triangles share an `(area, perimeter)`, a chart must pick one, and where they merge, the
rule's choice flips. That curve is the seam.

- **Solve:** Newton in f64 from a fixed list of starts, the pixel's own coordinates as the only input.
- **Rule:** declared per registered inverse, and recorded on the sample (`branch` in the IC record), with R-82's pattern
  for a tie: a fixed `δ`, and within it, one named choice.
- **Seam:** the set where the solver's Jacobian changes sign, or where `branch` differs across a pixel edge. It is drawn on
  the Domain preview and offered as an overlay.
- **Failure:** no root is `decode_failed`, blank (R-470), the label naming the solver.
- **Round trip:** `forward(inverse(x)) = x` at every sample, within the tolerance, or the sample is `decode_failed`.

### 7.4 Seams: jitter, spread and refinement

A composed chart is continuous away from its declared seams (the model's Theorem 10). Across one, two nearby pixels can
be far apart in `Z`, and the difference belongs to the chart and not the dynamics. Three rules follow.

**Jitter stays in the chart's own coordinates.** That is the standing rule: offsets "in the pixel's IC-space footprint, fixed
by the pixel alone" (`docs/notes/principia_sampling_msaa_note.md`, R-80, R-100; scheduler contract Part 9). A composed chart
changes nothing. `copy_index` offsets `(u, v)`, and the warps and the pair map then run on the offset coordinates. No copy is
moved off a seam: that would read a neighbour and lose recreate-from-image.

**A copy that straddles a seam is flagged and kept.** A footprint is `seam_straddled` iff its copies aren't all on one branch.
The rule is for composed charts, the owner's words: a chart of today's `kernel::chart::REGISTRY`, masked or not, is bitwise
today's. The flag is a pure function of the pixel and the copy index (§ 7.3, R-100), and it's evaluated where the latch is, in
the resolve pass, on the GPU, with only the verdict returning (R-142; `docs/design/principia_dd_generation_root.md`, "The latch
is evaluated on the GPU"). A baked chart's branch and mask are computed in that pass. Any chart on CPU batch, closed-form or
numerical, has no kernel entry, so that pass can't repeat its solve: it reads `branch` from the IC record (§ 7.3), and a copy the
chart's mask blanks reaches it as that record's `decode_failed`. No crossing is added. **Two layouts change.** `write_ic`'s words
and `Ic<f32>` have no `branch`, so CPU batch's IC record gains one word per sample, on CPU batch only; and `n_seam` below adds a
slot to `QuadReduction`'s 44 (REQ-PAY-077). Each is a schema change, as R-603's was, so it bumps the payload schema version and
invalidates cached payloads. TASK-M8-102 owes both. Every copy stays in the SSAA pool and the spread pool. No copy is ever dropped, the uniformity invariant of
`docs/design/principia_dd_generation_root.md` § "Ensemble spread — the two bounded contributors". A composed chart's mask counts
as a seam here: a copy the mask blanks is on a different branch from one it doesn't, and it stays blank as today.

**Its spread is reported as seam-contaminated, never read as chaos.** On a flagged pixel `spread_shape`, `spread_event`,
`ensemble_spread` and `error_ratio` keep their values and carry `seam-contaminated`, in the readout and as an overlay.
`spread` is large because the copies began on different branches. `error_ratio`, `sigma_E(t)/sigma_E(0)`, goes the other way
where the branches differ in `E`: a branch jump inflates `sigma_E(0)`, so the ratio is pulled to 1 and can hide integration
error, and a low one doesn't clear the pixel. On an `E` axis the branches don't differ in `E`, and under hold `E` `sigma_E(0)` is
near rounding. What does clear a flagged pixel is `worst_energy_drift`, per copy against its own `E₀`, which a seam can't
contaminate.

**Declared seams are masked from the refinement criterion.** A seam is a curve at every depth, so `unresolved(f)` would stay
true along it for ever, and the scheduler would chase the chart's own fold as structure. So on a flagged footprint the
spread clause, the event-class clause and the latch of `principia_dd_refinement_policy.md` § 1 don't fire, and the latch
doesn't set. Its undetermined clause doesn't fire on a copy the chart's mask made unusable, and still does on a non-finite
one. The footprint leaves `n_unresolved`.

**Its decision is `Seam`.** `Keep ⟺ n_unresolved == 0 ∧ tile_size_px ≤ 1` (dd_refinement_policy § 0.1) reads that count as
resolved, and a flagged footprint isn't. R-603's `Undetermined` is the pattern and not the variant, since it means finer `eta`.
So a quad with any flagged footprint and none unresolved, which would otherwise keep, takes a new decision, `Seam`, in policy § 6's list and its stop reasons,
and `QuadReduction` gains a count, `n_seam`, evaluated with `n_unresolved` (R-142), which the readout shows.

**Order too.** In view the criterion decides order (policy § 0.1). `coherence = 1 − n_unresolved / N²` (generation root § 3.7), and `P_complexity` is
`1 − coherence` (scheduler contract Part 6), so taking flagged footprints out of `n_unresolved` takes them out of the priority
as well, and the scheduler doesn't chase them there either. So a quad of flagged footprints reads full coherence and is evicted
early. That is stated and accepted: `n_seam`, not `coherence`, says what such a quad is, and TASK-M2-59's port must say that a flagged footprint isn't a resolved one, or generation root § 3.7's "every footprint resolved" misreads such a quad.

**The price.** Real structure crossing a seam isn't chased inside the flagged band, which is one footprint wide. In view
the camera still decides depth (dd_refinement_policy § 0.1), so the band narrows with zoom, and unflagged neighbours fire as
before. A final-condition chart's dynamical folds aren't declared seams, and stay real structure (§ 10.3).

### 7.5 The browser build: what it gets

R-571 fixes the aim: "the whole native app, compiled to run in the browser, with the same interface, the same features and
the same stains", and where the browser can't do something, "v1 web says so in the app". What composition adds to that is
CPU batch (§ 7.2), and that is the door: **RQ-735**. The GPU is f32 in the browser as natively, and the CPU engine is Rust
in wasm (`docs/contracts/principia_canonical_spec.md`), so both routes below are possible in principle.

| Chart | (a) closed-form GPU only | (b) a WASM CPU route too |
|---|---|---|
| baked pair, uniform affine | `live` | `live` |
| closed form past the cap, or not baked | the app says it can't | CPU batch in the engine's worker |
| numerical axis | the app says it can't | CPU batch, seam and branch as § 7.3 |
| a morph with a CPU-only end (§ 17.12) | the app says it can't | CPU batch, not live |
| final-condition chart, if RQ-734 builds one | the app says it can't | CPU batch, costliest of all |

**Reasoned, not measured:** every line of this section. Nothing of this design has run in a browser. § 11 measured Metal
and Vulkan, not WebGPU. TASK-M8-96's Pages test of wasm threads (R-627) hasn't run, so whether CPU batch has more than one
thread there isn't known. Whether a wasm f64 solve matches the native one bit for bit isn't measured either; if it doesn't, a
saved numerical chart reopens with its round trip intact and its pixels not bitwise.

**Recommended: (b).** V24 puts every § 3.1 and § 3.2 row on CPU batch, so (a) leaves the browser the cap's few baked pairs:
nearly none of the primitives, which is a different app from R-571's. (b) is R-571's own shape: the same features, slower,
and the app says so where it can't keep up. It's the owner's call. Nothing is built for either until it's ruled.

---

## 8. First customer: region D

### 8.1 The definition

From dd_validation_orbits §1.6, the port of R-592; the M9 targets name it through "the Anosova physical-frame entry"
(replication § 2 and its rows), and the lowering appendix gives it a row.

- Bodies at `A₁ = (−½, 0)`, `A₂ = (+½, 0)`, `A₃ = (ξ, η)`, equal masses `m = 1` in the papers' equal-mass runs. At other masses the
  plane is the same plane, geometric, and § 8.8 says what it covers.
- **Region D** (eq. 8): `ξ ≥ 0`, `η ≥ 0`, `(ξ + ½)² + η² ≤ 1`. A point outside it is not a region-D start.
- Velocities, one of two: **exact rest**; or **eq. 3**, `ẋ₁ = ẋ₂ = +5·10⁻⁵`, `ẋ₃ = −10⁻⁴`, every `ẏᵢ = 0`, near-rest. Eq. 3 is an equal-mass start: at other masses it has `P ≠ 0` (§ 8.6).
- Units: `G = 6.67·10⁻⁸` in the papers' units, length 1 as stated there. Energy, eq. 4.

**At equal masses and `L_z = 0`,** the whole of D is one fundamental domain under relabelling and the mirror: its three edges are the isosceles line `ξ = 0`,
the collinear line `η = 0`, and the arc `r₀₂ = r₀₁`. Its corners: `(0, 0)`, `(½, 0)` a binary collision of bodies 1 and 2,
and `(0, √3/2)` the equilateral triangle.

**`reduced domain`.** D is the papers' reduced domain of starts: `η ≥ 0` the mirror, `ξ ≥ 0` the body swap at equal masses. It stands as a labelled view, not a reduced chart domain: the preset renders the full disc, the symmetry filter on by default gives D, and the filter's toggle is the one click back (§ 10.4; RQ-736, the later order governing).

**This definition is the door: RQ-714.** Four things it needs ruled are in § 8.4 to § 8.6.

### 8.2 The closed form

`A₁, A₂, A₃` are bodies 0, 1, 2. In dd_decoder §3.2's terms `ρ = A₂ − A₁ = (1, 0)` and `λ` runs from the inner pair's
centre of mass to `A₃`. That centre sits at `(d, 0)`, not at the origin, unless `m₀ = m₁`:

```
d = (m₁ − m₀)/(2 M₀₁)                        0 at equal masses
λ = (ξ − d, η)
ℓ² = μ_ρ + μ_λ ((ξ − d)² + η²)               I before the gauge, = 1/6 + (2/9)(ξ² + η²) at equal masses
ρ̃ = √μ_ρ (1, 0)/ℓ          λ̃ = √μ_λ (ξ − d, η)/ℓ
```

The plane stays the literal one, `A₃`'s position, so the geometric axes of § 8.3 keep their meaning at every mass.

That is all of it: no `atan2`, no pole at `(0, 0)`, the canonical frame by construction, `C` a no-op, the mirror not
taken at `η = 0` (the deadband, R-82); past `η = 0` the decode takes the signed `C`, `canonicalise_signed`, so `η < 0`
stays its own system (§ 8.4, RQ-736). The lowering appendix's "physical (m, r, p) construction → canonicalise entry" is
this, and `ShapeSphere`'s explicit-configuration path carries it.

**It is the shape sphere, stereographically.** With §3.1's `n = (u, v, w)`:

```
(ξ − d, η) = √(μ_ρ/μ_λ)·(n_v, n_w)/(1 + n_u)
```

the projection from `n = (−1, 0, 0)`, §3.1's collision of bodies 0 and 1. So the plane is conformal to the mass-weighted
sphere, and the figure's Tier 1 circles are circles because of it. At equal masses D's apex is the pole `L⁺`, and the
collinear row `η = 0` is the equator. Frames of other papers map onto this plane by § 8.8's ratio: affine for most, a Möbius map for Breen's (§ 20). Checked numerically at random
masses, 0.05 to 1 before normalising, over `ξ ∈ [−2, 2]`, `η ∈ [0, 2]`, against the identity in `λ`, to 5.6·10⁻¹⁵ (`docs/incoming/2026-11-06_chart_composition/checks/regiond_check.py`; a tested function
in TASK-M4-34). The same script's inverses in `(ξ, η)`, drawn near each `η` over that range and on D: from the side ratios, the focal row's root (§ 3.2), 2·10⁻¹² at `η ≥ 10⁻³` and about 2·10⁻¹⁵/`η` below it, or 10⁻¹³ and about 10⁻¹⁶/`η` on D; from the interior angles, with both above 10⁻⁶, about 4·10⁻¹⁶/`η` (4.1·10⁻¹¹ near `η = 10⁻⁵`), and flat on D, 2.7·10⁻¹⁶ at every `η` drawn, down to 10⁻⁶. That check took `λ = (ξ, η)` as given, so it could not see `d`: the offset comes from dd_decoder §3.2, not from it.

### 8.3 The pairs it carries

Each inverts into `(ξ, η)`, in the frame above, so one baked entry could serve all three, its warp word choosing:

```
angles:       η = 1/(cot γ₀ + cot γ₁)           ξ = −½ + η cot γ₀
side ratios:  ξ = (a² − b²)/2                   η = √(a² − (ξ + ½)²)     a = r₀₂/r₀₁, b = r₁₂/r₀₁
```

Whether they share one entry is V5. Each is `forward(inverse(x)) = x` (§ 12), and R-798's remap of PA-1's side
ratios onto the shape sphere is this line followed by § 8.2's: one source, as REQ-CHART-093 asks. **R-798 keeps "There is
no `SideRatio` chart"**; whether a side-ratio axis in the builder is one is **door** RQ-713, and PA-1's route is not
touched either way.

### 8.4 What the chart is: the domain

D is not a product of two ranges. Three ways to draw it:

- **(a) A mask.** Axes `ξ ∈ [0, ½]`, `η ∈ [0, √3/2]`, linear; `validate` refuses a pixel outside eq. 8 as a named layer-2
  constraint, blank, out of the count (R-470, R-408), no warning (R-506 W1). The picture is the papers', and pixels are
  uniform in `(ξ, η)`, the sampling measure region-D statistics assume.
- **(b) The plane.** No mask: the outside is real systems, at equal masses and `L_z = 0` relabelled copies, otherwise other
  systems (§ 8.8), and D is a contour. Statistics then filter.
- **(c) A warp onto the square,** polar about `A₁`, `v ↦ r ∈ [1/(2 cos γ), 1]`, every pixel inside D. Non-uniform measure, the
  apex a ray-degenerate edge.

**Recommended (a), as a preset of the third-body pair**, whose own axes are `ξ` bounded and `η` bounded. The lowering
appendix's "region D bounds" uniform is the mask. **Door** RQ-714. *Superseded by R-871 (record correction, 9 Nov
2026, applied per R-369):* no region-D mask uniform is built; the full disc is rendered unmasked, and the appendix's
Anosova row, region D's pair, is lowered by TASK-M4-34 (R-914, REQ-CHART-165). The sentence before is kept as a *Was*.

**The owner's filter order overtakes this for the presets (RQ-736):** "region D presets render the full disc, with the filter on by default for the paper-frame presets only." So the preset is (b) with the filter of § 10.4, and D is its output; the decode must reach `η < 0` and `ξ < 0` signed. What "the full disc" is, and so the preset's axes, is a choice on RQ-736's list, not made here: the lens `(|ξ| + ½)² + η² ≤ 1` of D's four images with the box's corners masked, the box `ξ ∈ [−½, ½]`, `η ∈ [−√3/2, √3/2]` drawn whole, or any window holding the lens with all three swaps declared, which filters to exactly D (§ 10.4). What TASK-M4-34's preset does before TASK-M8-120 exists is on that list too. *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]*

**Ruled 8 Nov 2026 (R-871, R-878):** (b), with "the full disc" read as the third reading above, "any window holding the
lens with all three swaps declared, which filters to exactly D": no mask is built, in M4 or later, and R-554's exception
is not extended. The filter is on by default for the paper-frame presets only (R-855), and its check is the owner's: at
equal masses and rest, the filtered area is 0.307, D's own (TASK-M8-120). Until TASK-M8-120 lands, TASK-M4-34's preset
renders that window with D as a contour, and TASK-M4-06's golden reads `in_region_d` (RQ-738 conflict 8's reading held).
*Ruled 9 Nov 2026 (R-914, RQ-752 (a)):* that GPU reference, the golden that reads region D, is TASK-M4-34's own, not
TASK-M4-06's; TASK-M4-34 can't close until it reads region D as R-871 requires, and TASK-M4-06 gains no edge to it.
*Ruled 9 Nov 2026 (R-916):* the filtered area's check is on the filter's maths, TASK-M2-84 (REQ-CHART-176), on a
synthetic payload at M2; the legend, the defaults and the toggle stay TASK-M8-120's.

A mask has a cost the ruling must meet: contract Part 3 says "**Inside `[0,1]²` every gallery chart is valid by
construction** … so a pixel hatched there is a decoder bug", and REQ-CHART-058 sweeps for it. R-554 made the same exception
for the exponential map past `r = π`: "the domain's end by design", blank, unwarned. Option (a) extends that exception by
name. A masked pixel is then the domain's end, out of R-408's count and total, not "forbidden". R-506's "warped to its
feasible range by default" is met trivially: every `(ξ, η)` is a system, so the feasible range is the whole range.

**Extension.** Both bounded, the default: past `η = 0` the mirror, past `ξ = 0` the relabelling (on the region-D preset both are now inside the axes, § 10.4), and neither is a
continuation anybody has defined. R-407's rule: "Nothing ever invents a continuation". The plane chart without the mask
declares nothing more.

### 8.5 Which M9 target needs which part

The "Anosova entry" of the replication rows is three things. Point import exists; the plane does not.

| Part | Exists | Targets |
|---|---|---|
| **Points**: a published start through the import | yes, TASK-M2-20 | LL-1, LL-2, LL-3 (TASK-M9-06, -07), TM-1 (-09), AA-6 (-10), AA-2, AA-3, AA-5, AA-7, AN-1 (-19), BR-1 (-22) |
| **Samples**: many points in D, drawn, or a small grid of f64 starts through the import; no GPU run (R-671) | the membership test, `in_region_d`; the import | Q-1 (-18), Q-2, L-3, L-4, L-5 (-20), V-9, MT-1 (-21), GG-1's 1/64 grid (-22) |
| **The plane**: a pixel grid over D, run on the GPU | **no** | L-1 (-26), L-2 (-31), TS-1, V-1/3/7 (-28), and TASK-M4-34's region-D golden (REQ-RENDER-123, R-914; *was* TASK-M4-06's, REQ-RENDER-082) |

Velocities: exact rest for AA-6 and every free-fall map; eq. 3 for AA-2 and AA-7, and AA-3 and AA-5 read paper I, whose runs
are eq. 3's, though their rows do not say so (flagged for RQ-714). Masses: equal, but AN-1 and the LL unequal sets at a mass
point: the plane takes the masses as frozen. **No unequal-mass target runs eq. 3:** AN-1 and the LL sets are free fall, exact rest (REQ-VAL-279; `INC/2026-10-25_replication/catalogue.md` § 2.5; `docs/design/principia_dd_replication.md` § 5.3, and § 9, "Free fall, unequal masses"), so the unequal-mass case adds nothing to RQ-714, which still holds AA-3 and AA-5.

**Frames.** Each paper's frame is a preset over the registry, written out from the sources in § 20, tested there and in § 8.8. **One source:** if RQ-729 rules for the presets, TASK-M9-20,
-22 and -28 take them rather than each writing a function, by edges TASK-M2-58 applies then. Until it does, those tasks' own tested functions stand, as on `main`. Each ends in `(ξ, η)` or in a point, so the plane does not depend on them.

### 8.6 The momentum, and the rescale per pixel

**Rest** is `q = 0`, exact. **Eq. 3** is not uniform at `I = 1`: the import's rescale makes it depend on the pixel.
With `p̂ᵢ = (mᵢ/M)·vᵢ/√(GM)` (the import's `unit_state`), then `C`'s `pᵢ ← √ℓ·pᵢ`:

```
q(ξ, η) = √ℓ(ξ, η)·q̄      q̄ = the Jacobi form of p̂      for eq. 3 at equal masses, q̄ = (0, 0, −10⁻⁴/(3√(3G)), 0) = (0, 0, −0.0745, 0)
```

`q̄` is a uniform, at most four floats. At equal masses the sum matches eq. 3 and eq. 4 to rounding (checked as above, at `η > 10⁻³`: relative energy
1.2·10⁻¹⁴). At other masses `q̄` is the Jacobi form of `p̂` at those masses, after `C`'s boost: eq. 3's `P_x` is 3·10⁻⁵ at `(1, 0.8, 0.6)`, and `−0.0745` isn't its Jacobi form. This is a residual convention on the momentum block, which reads the configuration: the pattern of kind 3.

**Every `gauge` quantity carries that `ℓ`, and so does time:**

```
E_paper = G M² E_c/ℓ      L_paper = √G M^{3/2} √ℓ L_c      t_paper = ℓ^{3/2} t_c/√(GM)
```

So over D at equal masses, `ℓ` runs from `0.408` to `0.577`, and **one sim-key `T` is a horizon in paper time that varies by a factor of
1.68 across the chart.** Other masses differ, on the same grid (`docs/incoming/2026-11-06_chart_composition/checks/sweep.py`): 0.431 to 0.571 and 1.525 at `(1, 0.8, 0.6)`, 0.400 to 0.529 and 1.521 at `(3, 1, 1)`, 0.421 to 0.480 and 1.217 at `(9, 3, 1)`, so 0.408 is no floor. A fixed paper horizon needs the run to reach `T_c = T_p √(GM)/ℓ_min^{3/2}`, `ℓ_min` taken at the run's masses, and the comparison to cut
each pixel at its own. The scored comparison reads `ℓ` as a tested function (rule 11); nothing in the kernel moves.
The horizon in M9 is a replication question, flagged in RQ-714.

### 8.7 Where it lands

Before TASK-M4-06's golden, once RQ-714 rules. TASK-M4-34 builds the pair and its entry. **No existing task gains a dependency on this design while it
is Proposed** (applied per R-369): TASK-M4-06's golden (REQ-RENDER-082) needs RQ-714's domain, so its edge to TASK-M4-34 is named in TASK-M2-58's
Deliverables, to apply when RQ-714 rules, and it would then hold 230 tasks until TASK-M4-34 merges (§ 15). The plane targets of § 8.5 then reach
TASK-M4-34 through TASK-M4-06 and need no edge of their own; the samples need none. It is not "built ahead of" a target (R-594 item 5): its first
customer would be M4's own golden.

*Ruled 9 Nov 2026 (R-914, RQ-752 (a)):* the edge is dropped and never applied. With it, TASK-M4-06 would have waited on
TASK-M4-34, which R-873 releases in stage 3, after stage 2, while stage 2 waits, through TASK-M8-124 (now TASK-M2-61),
TASK-M8-48, TASK-M8-18 and TASK-M4-17, on TASK-M4-06: a cycle (a record: R-916 removed TASK-M2-61's edge to TASK-M8-48,
so stage 2, TASK-M2-65, no longer waits on TASK-M4-06; the edge stays dropped). The GPU reference that reads region D, the DECODE-preset golden of
the Anosova physical-frame chart (REQ-RENDER-123), is TASK-M4-34's, which depends on TASK-M4-06 and can't close until
that reference reads region D as R-871 requires. The appendix's Anosova physical-frame row is this pair, so its
lowering is TASK-M4-34's too, through `region_d_variant`, unmasked and never folded to `(ξ, −η)` (REQ-CHART-165).
TASK-M4-06 and the tasks behind it wait on no task of this design.
The plane targets reach TASK-M4-34 through TASK-M2-77, by R-884's edges.

*Ruled 9 Nov 2026 (R-916):* the pair lands in M2, before any GPU work: TASK-M2-68 builds it on the CPU, its chart, its
preset with both velocity presets, `region_d_plane` and `closed_inverse_round_trip` (REQ-CHART-101, REQ-CHART-142,
REQ-CHART-171), with stage 2's region-D slide arm. TASK-M4-34 keeps the baked entry, the lowering row and the GPU
reference (REQ-CHART-100, REQ-CHART-165, REQ-RENDER-123), depending on TASK-M2-68, TASK-M4-33 and TASK-M4-06; R-917 holds
it until M2's gate.

### 8.8 Region D must match the papers (order item A5)

**The frame, answered plainly.** `(ξ, η)` is geometric at every mass: body 2's position with bodies 0 and 1 at `(∓½, 0)`, centred on
the *midpoint* of the pair. It is not COM-centred, and § 8.2's `d` doesn't make it so. `d` is where the pair's centre of mass sits in
that plane, and it enters only on the way to dd_decoder's Jacobi `λ`, which is measured from that centre. Dropping `d`, `λ = (ξ, η)`,
is what *would* centre the plane on the pair's COM, the frame the owner rules out, and it is the defect Codex found. **So `d` stays:
it is the right map into the geometric frame, and the chart's axes never carry it.**

**The exact map, any masses, any paper's frame.** Write positions as complex numbers. For bodies 0, 1, 2 in any frame,
`ρ = r₁ − r₀`, `λ = r₂ − (m₀r₀ + m₁r₁)/M₀₁`:

```
ξ + iη = d + λ/ρ = d + √(μ_ρ/μ_λ)·(n_v + i n_w)/(1 + n_u)        d = (m₁ − m₀)/(2M₀₁)
```

The ratio is invariant under translation, rotation and scale, so it needs no paper's origin or unit; `η`'s sign is the triangle's
orientation, the signed `n̂`. In the canonical frame `ρ̃` is on `+x`, so `λ/ρ = √(μ_ρ/μ_λ)·λ̃/ρ̃`. **The chart reads it back by the ratio
form**, which is well conditioned. The sphere form is equal but loses digits at large radius.

**"Bitwise-to-rounding", as tested.** From a published `(ξ, η, m)`, through the state in the chart's gauge and back, measured here
(f64 in ulp of the coordinate; `a5_check.py`): AA and TM-1 points 1; Li & Liao's F5, F27, F6, F24 at most 2; Burrau's triangle `(0.14, 0.48)`
exact; AN-1's `(1.530844530, 0.281758510)` at most 1, and `(9.897502790, 0.351546184)` 1 by ratio, 33 by sphere. F1's `ξ = 9.1·10⁻⁴` reads 68 ulp,
which is 1.5·10⁻¹⁷ absolute, `d` cancelling. So the tolerance is absolute: `|Δ| ≤ 4 eps·max(1, |ξ|, |η|)`, in f64 and f32, **read from
the blocks `ρ̃, λ̃`** (physics note 3: worst 2.7 eps at f64). Read from particle positions the worst is 4.3 eps·scale, so that read takes a measured
tolerance (R-71). A row whose offset is below it, P-1's `δ = 5·10⁻⁸`, H-3's `Δ = 10⁻⁸`, PA-1's Set B, runs at f64 only: at f32 it would collapse to one point and pass. Over D at random masses
to 10⁻³, over two draws, one Dirichlet and one log-uniform reaching that floor, with `η` down to 10⁻⁶, the sphere form's worst is 7.4·10⁻¹⁴ at f64 and 2.9·10⁻⁵ at f32, so an f32 read back goes by the ratio. **Bits, as such,
are the published decimal's own:** rounded once to the working float, as the import does.

**The preset at unequal masses (RQ-728; RQ-736).** `ξ ≥ 0` isn't a symmetry there. The chart declares its symmetries (§ 10.4), so at rest it declares the mirror and, only with equal masses at `A₁` and `A₂`, the swap. Elsewhere the swap isn't declared, its partners are absent, the `ξ < 0` side stays drawn, and a statistic still reads `one labelling`, RQ-728's option (a). So at a Li & Liao mass set D isn't a filter's output: it's a contour on the half the mirror leaves. Under a held `L* ≠ 0` the mirror is lost too and nothing is blanked. The corpus says what the labelling is; **it doesn't say how to show D at unequal masses once the mask is gone: a gap**, on RQ-736's list. Eq. 3's near-rest start has `ẋ ≠ 0`, so the mirror holds and the swap doesn't, and at equal masses too its `ξ ≥ 0` isn't the filter's: also on that list.

**The angle pair and Parischewsky's pair.** Their axes are unsigned, `η ≥ 0: chirality lost` (§ 20), so the chart holds no partner to filter and no full disc to offer. They keep `reduced domain`, with no filter. RQ-736's list says so.

**Three things in the order that don't hold as written.**

1. **"Longest side = 1 (TARGETS.md:475; catalogue 1.1)" is derived, not stated.** Those lines give equal masses, `A₁`, `A₂`, and eq. 8. In D,
   `r₁₂ ≤ r₀₂ ≤ r₀₁ = 1` follows. Li & Liao's p. 2 and Valtonen et al. 2005's text agree. It isn't a property of the plane.
2. **Anosova 1969 is not region D.** `INC/_local/anosova_1969_Ap_5_81_part3.pdf`, p. 81: "In the case of different masses, the domain D does not
   include all the possible configurations. The new domain is the entire half-plane bounded by the ξ axis", and "a new domain D′ … defined
   by a semicircle of radius 10 units of distance and centered on the origin". Table 1's `(ξ′, η′)` run to 9.9. The sign of `ξ′` is assigned to the
   first 50 pairs at (9, 3, 1) and (3, 1, 1), and all 100 are taken at (3, 3, 1), where the axis is a symmetry. So in AN-1 **`A₁A₂ = 1` is not the
   longest side**. The frame is geometric and centred on the origin, which the order has right. The region-D mask can't reach it; a preset
   can: same plane, full disc, the half-disc the filter's output (§ 10.4). The placement isn't printed, nor the formulas "transforming D", but it is derived: "the coordinate axis is the axis of symmetry" at (3, 3, 1) holds only with the equal masses at
   `A₁, A₂`, so `C` is `m₃`, the lightest; both signs of `ξ′` make the set unchanged under `A₁ ↔ A₂`, so the rest doesn't change a statistic. The table carries the result. The catalogue's "region D" title for § 2.5 needs that note (RQ-728).
3. **At unequal masses D is one labelling.** Li & Liao keep D at (1, 0.8, 0.6) and say it holds "all possible configurations"; true at equal
   masses. TARGETS's caveat is right: D holds one of six labelled sectors, so "Orbit 19" isn't in it. A statistic over D isn't over all triangles,
   and under a held `L* ≠ 0` the mirror is lost too (physics note 2, P11). "Outside D blank and uncounted" was agreed for equal masses and `L_z = 0`; its
   reach is RQ-714's and RQ-728's.

**Every target, with its test.** `region_d_native` (TASK-M2-75, § 12) runs each row's own coordinates through the row's preset (§ 20) and asserts § 8.8's
tolerance. **A round trip alone can't see a wrong frame:** with `d` dropped on both sides, F5 still reads back to 5.6·10⁻¹⁷ while its `r₀₂/r₀₁` is 0.699858 against the paper's
0.746066 (physics note 3, A1). So each row also asserts an **independent oracle**: the paper's three separations, computed in the paper's own frame from its printed
positions, against the decoded state's, and eq. 4's energy on AA rows. Controls: `d` dropped on both sides; bodies 0 and 1 swapped at unequal masses; `η` mirrored. Unequal-mass rows are starred.

| Target | Native coordinates | Preset | Case in the test |
|---|---|---|---|
| AA-1 to AA-8 | `(ξ, η)`, eq. 8 | Agekyan–Anosova | every row of `fixtures/anosova/table1.csv` |
| LL-1, LL-2, LL-3 at (1, 1, 1) | `(x, y)`, G = 1 | Agekyan–Anosova | Table II's F10 |
| LL-1 to LL-3* , LL-5* | `(x, y)` at the mass set | Agekyan–Anosova, masses frozen | F1, F24, F5, F27, F6 and each table's first row |
| AN-1* | `(ξ′, η′)` | Anosova half-disc | row 1 at both signs of `ξ′` at (9, 3, 1) and (3, 1, 1); rows 1 and 51 at (3, 3, 1). Rows 51 to 100 exist only there |
| CH-1* | the Pythagorean triangles | Agekyan–Anosova after relabelling by the ratio | Burrau → `(0.14, 0.48)` |
| AT-D2* | atlas rows | as their source | a row per mass plane |
| Q-1, Q-2 | D | Agekyan–Anosova | 100 random points |
| L-1 to L-5 | `(x, y)`, bodies at `(0, 0)`, `(1, 0)` | Lehto lens | one point per quadrant of the lens |
| V-1, V-3, V-7, V-9 | D at ×2 | Valtonen book | `(0.4, 0.6) → (0.2, 0.3)` |
| GG-1 | the 1/64 grid of D | Agekyan–Anosova | every grid point |
| MT-1 | D | Agekyan–Anosova | a sample |
| BR-1 | Breen's `(a, b)` | Breen | `(−0.3, 0.4) → (0.6081…, 0.6486…)`, closed form to the ratio |
| TS-1, TM-1, TSM-1, TUA-1, TM23-1, LI21-1 | Tanikawa `(x, y)` | Tanikawa `D₁₁`, under RQ-650 | a TCP, `(0.19208270, 0.30936018)` |
| TS-2 | `D₁₁ ∪ D₁₂` at fixed `k` | **not built** (R-669) | the frame's map only |
| H-3 | `(Δx, √3 + Δy)` at side 2 | Lagrange apex offset, no D mask | Table III's rows, f64 |
| PA-1 | `(S₁/S₂, S₁/S₃)` | Parischewsky, through R-798's remap until RQ-713 is ruled | `(0.8498…, 0.7211…) → (0.1, 0.6)` |
| TASK-M4-34's region-D golden (R-914; *was* TASK-M4-06's) | a grid over D | Agekyan–Anosova | its grid |

---

## 9. Then virial

Built by TASK-M2-37: `kernel::chart::physical::AxisVirial`, and chart_reference §2.1's paragraph, which warps `Q = 2K/|U|`.
REQ-CHART-070's debt is paid there.

**What the axis reads.** At `L_z = 0` it is the virial ratio. In the warped mode at a held `L_z ≠ 0` it is not (in the linear
option `K* = Q|U|/2` the axis reads the state's own ratio, blank below `Q_min(U)`, P12): the warped line is
`K* = L_z²/2I + Q(t)·|U|/2`, R-708's additive floor, so, as chart_reference §2.1 says, "The pixel's own virial ratio
`2K*/|U|` is `Q(t) + L_z²/(I·|U|)`". The axis reads the excess above the rotational floor, `Q(t) = 2(K − L_z²/2I)/|U|`, and
the state's own ratio is larger by `Q_min(U) = L_z²/(I|U|)`, which varies with `U` across a row. So § 3's `K* = Q·|U|/2`
is the `L_z = 0` line and the registry carries both; the label at `L_z ≠ 0` is `Q above the floor`, with the state's own
`Q` in the readout. Whether the floor should sit under the rise only is **RQ-700**.

What is missing is its descriptor. RQ-682 (open, in the queue): "Which of the four axis kinds a virial-ratio axis is:
R-471 H3 says it is not kind 3", and "No kind fits, and the set is closed". This document needs option (a) of RQ-682: kind 3,
`InvariantQuantity` gaining `Q`, solving into the momenta, reading config and mass. H3's sentence is about B1's extension,
which R-506 superseded. RQ-700 (where a virial bottom row sits, `Q₀ > 0` at a held `L_z ≠ 0`) stays open. It blocks nothing today only because
RQ-682 keeps the axis off every builder chart; once RQ-682 lets the axis in, RQ-700 decides what a row of it means, so
TASK-M2-70 waits on it for `L_z ≠ 0`, and with `(a)` applied the label above says so.

The second pair is `(<configuration axis>, Q)`, a `MixedEnergy`, baked as TASK-M2-70, for the virial pair the owner names.
`Q` at exact-rest region D is 0, and `Q` varying over D is the eq. 3 family's own reading. TS-2's "virial ratio 0 to 0.7" (that is `k = T/|U|`, so `Q = 2k`; RQ-731)
stays "not scored: no fixed-k velocity chart" (R-669).

---

## 10. The gaps, each as a builder option

| Gap (item 4) | Covered by | Task |
|---|---|---|
| 1D charts in the GUI | § 10.1 | TASK-M8-104 |
| Curved slices off the shape sphere | § 5.2 | TASK-M2-72 |
| Hold `L_z` on any chart | § 5.3 | TASK-M2-69, its GPU tags and `τ` TASK-M8-118 (R-916) |
| Polar and log as general options | § 4, § 5.1 | TASK-M2-71 |
| Tier 1 overlays of any primitive | § 10.2 | TASK-M7-72 |
| Final-condition charts (the model's Theorem 11) | § 10.3 | none: door RQ-734 |
| The symmetry filter (the owner's order of 7 Nov) | § 10.4 | TASK-M2-84, its maths; TASK-M8-120, the display, the toggle and the owner's test on real runs (R-916) |

### 10.1 1D charts

The order says "today only the TR-1 engine scan". That scan isn't built: `TASK-M9-23` describes it, R-669 parks it with
"no binary-plus-single chart", and no code has it. The nearest existing thing is the Euclid plane, a 1-swept chart with an
annotation axis (contract Part 5). A 1D chart is that: one swept axis, the other annotation-only, well-posed as 1 + 7,
labelled `1-swept family`, statistics per value along the axis only. It lowers as its 2D twin does, the annotation
coordinate unread, so a pair is not used up. The GUI draws it as a strip, and a line of the chosen field beside it.

### 10.2 Tier 1 overlays

A contour of any registered primitive is its `forward`, in the line system (R-476, TASK-M7-43), display-only and never
scored (replication rule 6: "the conversion is a tested function, not a figure overlay"). §4.1's variables gain the
interior angles and the pair binding energies, and its functions gain `acos`, which the stain's expression syntax needs too
(R-509 Y1). TASK-M7-43 is unmerged, so its list grows by a note, and the rest is TASK-M7-72.

An overlay of a primitive evaluates on the decode, so every registered forward needs f32 evaluation on the GPU or the CPU
fallback: §4.2 already says "a GPU pass on the shared decode, with a CPU fallback at reduced resolution". The Domain
preview draws the same contours.

*11 Nov 2026 (R-952; R-926 item 5):* an overlay of a primitive evaluates on the CPU's f64 decoded state, on every chart,
a chart with no shader code and a blend included: each registered forward is evaluated there, and lines § 4.2's GPU pass
with its reduced-resolution CPU fallback is replaced. *Was:* "every registered forward needs f32 evaluation on the GPU
or the CPU fallback".

### 10.3 Final-condition charts

A design item for the rulings report, the owner's order of 7 Nov: **door RQ-734.** No axis and no task exists until it's
ruled. The chart-space model's Theorem 11 is the source: a chart `ψ` describes the state at time `T`, and
`φ_T(u) = Enc(F_{−T}(D(ψ(u))))` is an ordinary IC chart, with `F_{−T} = R ∘ F_T ∘ R`. The model states its limits, and
they stand: masked pixels where the backward run doesn't survive, real folds, one extra integration per pixel. Its § 0.1 puts
a chart's image in the signed state space.

**As this design would build it.** A CPU batch chart, never baked, since each decode integrates. `T` is on the sim key and
the chart carries `final condition, T = …`. The extra run is per sample, so `E + 1` per pixel. Its cost is unmeasured. A
pixel whose backward run ends in a collision or an escape is blank, counted apart, and its reason named. Such a mask is
declared. `φ_T`'s dynamical folds aren't seams, so refinement chases them and spread reads them (§ 7.4). Jitter stays in `ψ`'s
coordinates. Functionals stay excluded at every `T`.

*12 Nov 2026 (R-985, the owner's direction on the chart audit's F17):* under R-926 a final-condition chart is a tier-3 map like any other (no slope of its own, R-926 item 9), evaluated on the CPU as every chart is; R-879's "CPU batch chart" reads as that evaluation. It takes the coarse cover in motion and is exact at rest (REQ-SCHED-020), and carries no label of its own as a route ("not live", "CPU batch" or the like). R-879's other decisions stand as written, the per-pixel horizon labelled as the final-condition charts' exception and the round-trip miss measured and large misses masked among them. Its measure weight is RQ-800's hard case and waits on it. *Was:* the paragraph above stands as written.

**The mirror half.** `Enc` mirrors a start with `λ̃_y < −δ_λ` (encode contract Part 6 step 3), and clamps one past `q_max`
(§ 17.2). The owner's rider to RQ-736 says the mirror half is never dropped or folded, in any chart. So a start found stays in
the signed state: it takes the signed `C` of § 17.2 and fills the IC from the state through CPU batch, with no `z`, so neither
the mirror nor the clamp runs. RQ-734 confirms this reading. *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]* Through `Enc` proper, `φ_T` would jump along every curve where the
start found is collinear, with copies across it mirror images, flagged per copy from `lookup_mirrored`, and a clamped start
would need a mask of its own.

**The checks.** Each would be a test that can fail:

- **`T = 0` agrees with the ordinary chart within `ε_phys`,** through the real path. Bit for bit is out of reach: `E(D(z)) = z`
  bitwise on 0 of 20,000 (§ 17.8), and a `T = 0` arm that skipped the path would pass and prove nothing.
- **`L_z²E` and the sign of `E` are independent of `T`,** at `ε = 0`, within the run's drift, after signed `C` as before it: the
  check as the model states it (§§ 0.5 and 6b). `E` and `L_z` are independent of `T` only before the rescale. On this design's
  signed start the sign of `L_z` is independent of `T` after signed `C` too, an added check. Only on the `Enc` arm above does
  the mirror flip it (encode contract Part 1, `G`'s `Z₂`), unless corrected by `lookup_mirrored`. An `approximate` row holds within
  its label. State rows move, which is the control.
- **The round-trip miss.** The start found was moved by signed `C`, so the forward run reaches `g·D(ψ(u))`, not `D(ψ(u))`, at
  `T' = T/I_in^{3/4}`, not `T` (encode contract Part 2), `I_in` the moment of inertia before it. So the miss compares signed `C` of
  the forward state at `T'` with `D(ψ(u))`. The contract's `C` would mirror a lower-half `ψ(u)` and show a miss the size of the
  mirror on correct physics, and a mask built on that would drop the mirror half. It's measured per pixel, reported, and masked
  where it's large.

**What RQ-734 decides, beyond whether to build it:**

1. **The integrator.** Theorem 11 is exact for the true flow, and in the app it's an approximate inverse. The default occupant,
   Heggie × RK4, isn't run-reversible: only logH's TTL mode is (integrator contract Part 2, R-162, R-500), and adaptive
   substepping and the COM projection break reversal of a run. So on the default the forward run misses by the
   integrator's error, amplified by chaos, and that miss is what's measured and masked. An exactly reversible version
   needs a time-symmetric integrator with a reversible step rule: an option, not an assumption.
2. **The gauge.** `F_{−T}` leaves `I ≠ 1`, and `C` rescales lengths by `s`, with `E` going as `1/s`, `L` as `√s`, and
   `L²E` unchanged. The rescale also changes the time unit per pixel, as § 8.6's does, so a run of `T` in code units doesn't
   reach time `T` on that pixel.
3. **Line 243 of § 3.1,** "anything measured after `t = 0`", bars this chart as written (§ 3.3).

### 10.4 The symmetry filter

The owner's order of 7 Nov, verbatim in RQ-736: "a display option applied after rendering, never a change to what's computed". TASK-M8-120 builds it. It is the rule of § 6.6's "declared, optional view". *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]*

- **What it does.** An optional toggle, in the GUI and on export, that draws as transparent every pixel whose **starting state** is the symmetry image of another start present in the same chart, keeping one canonical representative of each pair. A self-partner (`η = 0`) is kept.
- **Its symmetries are declared per chart, and extensible.** The mirror, whole system, positions and velocities; body-label swaps where the masses make them exact. A chart's declaration is a list on `ChartSpec`, off the sim key, so a new symmetry is a new entry. A chart may declare fewer than are exact, as Anosova's preset does (the table below). Under a swap a body-indexed outcome field (the escaper, the binary pair, the collision pair) is permuted, the rest equal; under the mirror outcome fields are equal and signed fields flipped.
- **The correctness rule.** "Blank a pixel only if its partner is actually in the chart's domain (check via the chart's own map)." For a pixel `u`: form `g·D(φ(u))` and take it through signed `C` (§ 10.3: translation, rotation and scale removed, the mirror step not taken), then the chart's own axis inverses (§ 3's registry, never `Enc`) to `u'`, and blank only if `u'` is in the chart's domain and signed `C` of `D(φ(u'))` is signed `C` of `g·D(φ(u))` within `ε_phys`. Compared raw, a swap's partner never matches: on the third-body pair the 1↔2 image is the chart's pixel turned by `π`, body 3 at `(−ξ, −η)`, and the 1↔3 and 2↔3 images are rescaled to `A₁A₂ = 1`. Never the contract's `C`, nor `Enc`: their mirror makes `C(x) = C(Π·x)`, so `u' = u` would pass and every pixel would be its own partner. **The shape sphere's frame:** `p_fixed` is relative to `ρ̃` on `+x`, unmirrored, `λ̃` in either half-plane (`principia_chart_reference.md` § 3, R-538 J9's frame rule), so `Π` maps that frame onto itself and the partner of `(θ, φ)` is `(θ, π − φ)`, present exactly where `p_fixed` is its own mirror image (`p_ρ,y = p_λ,y = 0`). On a chart that holds something breaking the symmetry, a shape-sphere chart with fixed non-zero velocities or a held `L* ≠ 0`, partners are absent and nothing is blanked. The toggle then reads **"no duplicates in this chart"**, not an empty toggle. A partner outside the view but inside the domain still counts (V43).
- **The canonical choice is stated** in the legend and the export sidecar, for example "mirror pairs shown once: eta >= 0".
- **Default.** Off, except on the paper-frame presets: region D's and § 20's presets render the full disc with the filter on. That gives region D's quarter disc, the Anosova half-disc and the half shape sphere at rest, so the papers' figures are reproducible with the full disc still computed underneath.
- **Later, not built here:** a "skip computing duplicates" export setting for speed, off by default and labelled. No task owes it.

*11 Nov 2026, the filter on a mid-morph frame (R-944; applied per R-369, listed for the owner's veto, and conditional on
physics review).* R-899's Q8 (a) had the filter off on a mid-morph frame, with a label; R-944 makes that a gap to close.
The mechanism: the frame declares the symmetries either end declares; a pixel's partner is `place(g·D(φ(u)), frame)` (§
10.6), and the correctness rule above then decides each pixel, unchanged, so a declared symmetry that the blend breaks
blanks nothing ("no duplicates in this chart"). The canonical representative of a pair on a blend is a definition owed
(R-72), since `η ≥ 0` is an end's coordinate and not the frame's: proposed, the partner whose pixel comes first in the
frame's row-major order, a pure function of the pixel, for the physics reviewer to approve where TASK-M2-84 builds it.
Until physics review confirms the mechanism and that definition, the filter on a mid-morph frame is not recorded as
settled.

*11 Nov 2026, the physics review of PR #482 (review 5852979675, its confirmation 2).* The mechanism is approved: the
partner as `place(g·D(φ(u)), frame)` with the correctness rule above unchanged, so nothing is blanked unless its partner
is shown, and a symmetry the blend breaks blanks nothing. Either representative of a pair is physically correct, so a
pure order on the pixel is admissible, and the row-major proposal is acceptable as a gauge choice. The written
definition (REQ-CHART-209) is still owed and still approved where TASK-M2-84 builds it, and the physics review holds it
to five points, which it must state:

- (a) the order is a strict total order on chart points in the frame's window coordinates (row first, then column), not
  on a resolution-dependent pixel index, so it is unchanged by pan, zoom and picture size;
- (b) the rule is "blank `u` iff an equivalent point strictly precedes it", so the least member of each orbit is kept:
  with a body swap and the mirror both declared an orbit has more than two members, and where a blend folds `place`
  returns several points;
- (c) a self-partner is kept, as on an end;
- (d) the partner relation is tested to be symmetric at the tolerance, the control a pair with both members blanked;
- (e) the legend names the rule, as "The canonical choice is stated" above requires.

Two things follow from the owner's own rulings and are theirs to know, not physics: with the filter on, the kept half of
a mirror pair changes from `η ≥ 0` at `s = 0` to the row-major half at `s = 0⁺`, and it depends on the window's
in-plane rotation.

**Display only.** Nothing computed changes: not the sim key, the payload, refinement, an outcome or a statistic. It is a mask read by the display pass and the export writer, built on the CPU from the chart's map, off the sim key. So **R-408's statistics are unchanged**: a transparent pixel is still counted, and the optional "mirror pairs counted once" is the mirror-step proposal's, not this. The legend says its shares are of the computed chart. **Replication changes nothing:** a target scores what's computed, in its paper's measure (§ 6.4), selected by `in_region_d` or its own domain, never by what the filter drew. A paper-frame preset does compute more: the lens is 1.228 against D's 0.307, four times, and the box 1.732, 5.64 times.

**Checked against.**

| Chart | Declares | Partners | The filter gives |
|---|---|---|---|
| Region D's preset, equal masses, exact rest | the mirror and the 1↔2 swap, or all three swaps, as RQ-736 rules the window | present | on the lens, D; on the box with one swap, the box's quadrant `ξ, η ≥ 0`, not D (`(0.5, 0.8)`: outside D, no image in the lens); with all three swaps, exactly D. Stated "mirror pairs shown once: eta >= 0" and the swaps' |
| The same, eq. 3 near-rest | the mirror only: eq. 3 has `ẋ ≠ 0`, and the 1↔2 image needs a turn by `π`, which reverses every `ẋ`, so no swap's image is on the chart | the mirror's | `η ≥ 0`; `ξ < 0` stays (§ 8.8) |
| Anosova's half-disc, `R = 10`, at (3, 3, 1) | the mirror only, although the 1↔2 swap is exact at rest there: the paper keeps both signs of `ξ′`, and declaring the swap would give a quarter | the mirror's | `η ≥ 0` of the full disc |
| Li & Liao's unequal sets, at rest | the mirror only | the mirror's | `η ≥ 0`; D a contour (§ 8.8) |
| Half shape sphere at rest | the mirror | `(θ, π − φ)` | `φ ≤ 90°`; the chart's default stays off unless a paper-frame preset, and the whole sphere is computed (R-544) |
| Shape sphere with fixed velocities not their own mirror (`p_ρ,y` or `p_λ,y` ≠ 0) | the mirror | absent | nothing blanked, "no duplicates in this chart" |
| Parischewsky's pair, the angle pair | nothing | no partner: unsigned axes | no filter; `reduced domain` (§ 8.8) |

Where this meets R-544's words, "no hemisphere toggle", and the other conflicts: RQ-736's list.

### 10.5 Points of interest

The owner's feature of 8 Nov 2026 (R-870, verbatim there), a design item for this document. TASK-M2-80 builds its maths, the
nearest chart point, the off-slice distance and the near-match, as engine functions, and TASK-M8-126 draws it (R-916).

- **A point.** Any pixel's starting state may be saved as a point: any number of them, each named, each stored as the
  signed state (R-854: its mirror half kept, never folded).
- **Marks, on every chart and every frame of every path** (morph, tilt, slice, zoom, chart change): a **filled star**
  where the chart passes through the point exactly; otherwise a **hollow star** at the nearest chart point, labelled
  with the off-slice distance in a stated metric.
- **"Similar systems" overlays, optional.** The user picks properties from the primitive registry (§ 3), and the chart
  draws where each property equals the point's value: one property gives a contour line, two give intersection points,
  three or more give near-matches within a stated tolerance. They reuse § 10.2's Tier 1 contour overlays (TASK-M7-43,
  TASK-M7-72) and need forward formulas only, never an inverse.
- **Display only, never scored,** like § 10.4's filter: nothing computed changes, not the sim key, the payload, an
  outcome or a statistic. **Distinct from the lock:** points mark, the lock constrains (§ 17.8).
- **The owner's tests:** a point lying in the chart draws a filled star at the pixel that decodes to it; a one-property
  overlay passes through the star; overlays redraw consistently across a morph's frames.
- **Owed, not stated by the owner's words.** Definitions (R-72; REQ-GUI-427), written here by TASK-M2-80 (R-916; *was* TASK-M8-126) and approved by
  the physics reviewer: what "exactly" means for a chart passing through a point, "the nearest chart point", the
  off-slice distance's metric, how a near-match of three or more properties is measured, how marks are found on a path
  frame with no inverse, which state group holds the saved points, off the sim key, and the gauge in which property
  values and the off-slice distance are compared: chart pixels decode in the `I = 1` gauge, where of the conserved
  quantities only the scale-free `L_z²E` and the signs hold (the model's § 7), so "equals the point's value" is
  undefined for a scale-dependent primitive (`E`, `L_z`, …) until the gauge is stated; and whether the off-slice
  distance is taken on canonical representatives (centre of mass, rotation and scale quotiented) with the mirror not
  quotiented, the point being a signed state (R-854). A value (R-71; REQ-GUI-428): the
  near-match tolerance, proposed by TASK-M2-80 with its evidence, its criterion written first and set by measurement
(R-649, R-650), and confirmed by the owner at the M2 gate (R-916; *was:* "proposed by TASK-M8-126 … at the M8 gate"). A point's
  signed state on the latent chart needs the full-circle `β` of R-865 (TASK-M2-57).
- *11 Nov 2026 (R-947; R-944).* "How marks are found on a path frame with no inverse" is now defined: by § 10.6's
  placement function, its nearest-point route. So on a mid-morph frame a mark is found as on any chart with no inverse,
  with its off-slice distance, the route's residual. R-899's Q8 (a), "to pixel resolution" on a mid-morph frame, was a
  limitation with a label, which R-944 makes a gap to close; that the marks then meet the lock's tolerance mid-morph,
  with § 10.4's filter on, is applied per R-369, listed for the owner's veto, and conditional on physics review, not
  settled until it confirms.

---

### 10.6 Placement: one function for "where is this state on this chart"

*11 Nov 2026 (R-947, the owner's answer of 10 Nov 2026 to the re-plan addition's question (A); R-945; R-944).* Six
consumers ask one question, "which chart point shows this state, and how far off is it": Chart position and the state
map, the traces, the marks of § 10.5, § 10.4's partner check, the lock's re-solve (§ 17.8) and the decode and round-trip
presets. One function answers all of them.

`place(state, chart) → { point, sheet, residual, route, status }` is one Rust source in `crates/kernel`, beside the
encode, as R-542's H11 has the one projection; every consumer calls it, never a second implementation. Its input is a
signed state (R-854), its chart any chart, a blend or a saved blend included. It has two routes, chosen by the chart,
never by the consumer:

1. **The encode reading,** where the chart has a closed-form or registered numerical inverse: the encode, read on the
   shown axes, the hidden coordinates giving the off-slice distance. This is the one projection, unchanged, and R-538
   TF2's per-axis reading; it is what keeps a conserved axis showing a dot under a tilt
   (`principia_trajectory_viewing.md` § 3).
2. **The nearest point,** where the chart has neither, every blend included: the chart point whose decoded state is
   nearest the state, found by the lock's solver on the chart's own f64 forward map, seeded by the caller's hint (a
   trace's previous sample, a lock's previous centre) or by the backwards table. The residual is the off-slice distance.

`status` is `placed`, `left` (the leaving rule of `principia_lines.md` § 8, or a failed solve) or `ill-conditioned`
(below the floor R-935 has for RQ-767 (a)). On a chart with a second sheet, `sheet` names it (R-942, RQ-776; R-948 for a
trajectory state). Where a blend folds and more than one point lies within the lock's tolerance (REQ-CHART-158), every
such point is returned and the consumer applies its own rule: a trace takes the one nearest its previous sample, a mark
draws a star at each, the lock follows its local minimiser (R-935, RQ-759 (a)). The ends of a path are selected exactly:
at `s = 0` and `s = 1` the end chart's own route answers, bitwise.

**Accuracy.** On the CPU every route refines to the lock's tolerance, mid-morph included. "Approximate is acceptable
during a morph" (R-926 item 7) binds the fragment's table route only, which is measured against this function. **What
"refines to the lock's tolerance" tests depends on where the state is,** as R-935 has it under RQ-767 (a) (`decisions.md`
§ R-935, the bullet "RQ-767, the table route's accuracy outside a morph: (a)", its criterion "on-chart and off-chart
apart"). On the chart's image the test is the residual: "on the chart's image the residual in state reaches f32
rounding", here held to the lock's tolerance. Off the image the residual is not an error and is never bounded by the
tolerance: "off the image the converged residual is the projection distance `‖r‖_W`, a measured quantity never read as
solver error, convergence judged by first-order optimality". So a trace vertex off the image, which is almost every
one, is `placed` when the solve meets first-order optimality, whatever its residual; `left` by "a failed solve" means
the solve did not converge, never that the residual is large. **The
fragment** reads the same function's generated WGSL where the chart has closed-form code, and the backwards table with
its search where it has none (R-925).

**The two routes differ off the chart's image, by design.** The encode reading projects along the chart's hidden
coordinates; the nearest point projects orthogonally in the metric. They agree on the chart's own image. So at `s = 0⁺`
a trace vertex whose state is off the image may jump between chart A's encode reading and the blend's nearest point, and
on a blend the nearest point of an off-image state can snap from one place to another. Two measurements report this, P2
and P4, and neither is a gate; the owner decides on the numbers, and no threshold or fallback is chosen by anyone else
(R-951).

**Owed (R-72), under physics review, by the task that builds it:** the metric "nearest" is measured in (REQ-CHART-124's
mass-weighted metric on the `I = 1` state is where it starts), and which nearest point answers for a state off the
chart's image (R-935, RQ-767 (a)).

**This is the definition § 10.5 owed,** "how marks are found on a path frame with no inverse" (REQ-GUI-427): by route 2.

## 11. The axis cap

**The number here, 4, is the proposal.** The Windows Vulkan run is in (below), and one question stays open for RQ-716: C3's 2 s on Vulkan. No primitive of § 3.1 takes a slot: they
stay CPU batch with the label, and `SuvakovPlane` is R-669's own.

The order asks for a number with its measured cost (item 5). The bound's duty is REQ-PERF-007's: "bounded to the
active/plausible variant set, not the full cross-product". The value is a one-way door, RQ-716.

**The criterion, written before the first measurement.** `P` is the baked pairs added to the kernel of main 9188686f,
each its own compute entry in the one module. The cap passes all of:

| | Test | Limit | Why |
|---|---|---|---|
| C1 | `cargo xtask build-kernel` after a kernel edit | `T(P) − T(0)` ≤ 30 s and `T(P)/T(0)` ≤ 1.25 | it runs on every kernel edit and in CI |
| C2 | size of the WGSL module | `wgsl(P)/wgsl(0)` ≤ 1.25 | every pipeline parses the whole module |
| C3 | the eager set, 12 decode entries plus `P`, one after another on this Mac; and one creation | ≤ 2 s; ≤ 1.25 × baseline | "Chart switching must be instant" (lowering Part 4); the 2 s is a design number, for veto |
| C4 | naga validates; a pair's parameters fit `DecodeUniforms.chart` | 24 floats | the uniform's size is a layout |
| C5 | C1 to C3 again under the multiplier the plan implies | as C1 to C3 | `(chart × occupant × tier × links)`, lowering Part 4 |

The cap is the largest `P` passing, not above what the known customers need. The stand-in pair is at least as heavy as
region D's: the mass-weighted Jacobi from `(ξ, η)` with `sqrt`, `atan`, `atan2` and the near-rest momentum, each entry a
distinct monomorphisation. Each time is the median of three.

**What was measured.** On an Apple M3 Pro, Metal, under the queue's other work (load average 4 to 13 for this table):

| `P` | build (s) | SPIR-V (bytes) | WGSL (bytes) | naga (ms) | one creation (ms) | eager set (ms) |
|---|---|---:|---:|---|---|---|
| 0 | 10.1 | 766,248 | 1,004,550 | 38.9 | 83 | 1012 |
| 1 | 11.3 | 784,332 | 1,030,342 | 43.8 | 89 | 1790 |
| 4 | 11.2 | 838,676 | 1,107,749 | 41.0 | 85 | 1993 |
| 8 | 13.4 | 911,140 | 1,211,100 | 44.5 | 91 | 2656 |
| 16 | 14.9 | 1,056,100 | 1,419,411 | 50.0 | 117 | 4361 |
| 16, cheapest pair (`Mixed<H, V>`) | 21.9 | 1,029,104 | 1,381,194 | 52.4 | 114 | 4405 |
| 4, integrator inlined, one pair × 4 occupants | 18.9 | 997,940 | 1,441,010 | 73.3 | 172 | 1940 warm |

A pair is about 18 KB of SPIR-V and 26 KB of WGSL. The cheapest pair is about 23 KB of WGSL, so the size is the inlined
decoder and not the pair's own maths: a cheaper pair doesn't buy a higher cap.

A second run, built to force Metal's first compile (load average 21 to 42, so its times are for comparison between its
own passes only): eager set 12: 998 to 1116 ms; eager set 16: first pass 4424 ms, then 1610 and 1494; eager set 20: first
pass 3459, then 2218 and 2213; four integrator-inlined entries: first pass 3323, then 1942 and 1931, and a first sample
of 8817 at load 13. The first compile of a pair entry is thus between about 150 and 700 ms, and of an integrator-inlined
entry between about 350 and 1900; the spread is the load, and those are ranges, not estimates of a mean.

**Against the criterion:**

| `P` | C1 ratio (limit 1.25) | C2 ratio | C3 eager (limit 2 s) | C3 one creation | C4 |
|---|---|---|---|---|---|
| 4 | 1.11 pass | 1.10 pass | 1.99 s, a bare pass; 1.5 to 1.6 warm in the second run | 1.02 pass | pass |
| 8 | 1.33 **fail** | 1.21 pass | 2.66 s **fail** | 1.10 pass | pass |
| 16 | 1.48 **fail** | 1.41 **fail** | 4.36 s **fail** | 1.40 **fail** | pass |

So **the owner's "about 4" passes, and 8 does not.** `P` of 5, 6 and 7 weren't built; a line through the table puts C3's
eager limit at about 4 and C1's at about 6. The 30 s test in C1 passes throughout and never decides.

**C4.** Region D's pair needs at most 8 frozen floats, 4 ranges, 3 masses and `q̄`, which fit the 24; naga validated every
arm. `DecodeUniforms` needs no new field for a pair of that weight.

**C5: the multiplier decides the design.** One pair, integrator inlined, across 4 occupants fails C2 outright (1.43): an
integrator-inlined entry costs about 109 KB of WGSL and 58 KB of SPIR-V, four times a decode-only one, so C2's budget of
251 KB is spent after two entries. The cap of 4 therefore holds **only because decode is its own entry** (V2), writing the
initial conditions the integrator reads, which keeps chart and occupant additive. TASK-M4-06's "chart × link × occupant ×
FTLE" has to stay additive for a pair, or no useful cap exists. The cap counts **entries**, not pairs: a pair under two
link sets is two, and a tilt of a baked pair (`Tilted<C, P>`) baked as its own entry is one more. A tilted pair past the
cap runs as CPU batch, with the label.

**Start-up objects are not this design's.** The kernel creates 12 compute pipeline objects at start-up against the
lowering contract's "~10". That is issue #427, filed on its own at the owner's word, with no ruling yet. The facts this
design needs: eager pairs are what fail C3, so pairs are created on first use (V7), and if #427 changes the eager set V7
is re-read against it. The customers need four, and the re-measure below allows no more:

| Entry | Customer |
|---|---|
| 1 | region D's third-body pair (§ 8), which the angle and side-ratio axes share (V5) |
| 2 | the virial pair (§ 9) |
| 3 | R-669's `SuvakovPlane`, if RQ-712 leaves it baked as one chart. It is ruled as its own chart, so its entry is in the module whatever the cap says, and counting it here is the cautious reading |
| 4 | one spare, for the first pair a replication target wants live; `stage_blend` takes it if live paths are adopted (§ 17.12) |

**Proposed: 4 baked entries beyond the gallery, created lazily, decode-only.** Everything past it is `CPU
batch: not baked`, a label and never a refusal (§ 6). The owner asked for the Windows runner's Vulkan result on an RTX 2080 SUPER before it's ruled, and it's below.

**The re-measure, with the real pair.** Criterion written first (`docs/incoming/2026-11-06_chart_composition/checks/cap2-criterion.md`): C1 to C4 as above,
unchanged, plus C3′: the 12 gallery pipelines at `P` within 1.25 × at `P = 0`, since every pipeline parses the whole module,
so baked pairs cost start-up even when none is created. The pair is region D's own, `d` included, eq. 8's mask and eq. 3's
momentum, baked in distinct copies, since the others aren't built. This Mac, Metal, arms interleaved three times so load
drift falls on all of them, load average 12 to 28. Sizes are exact. A first pass with arms in a row gave times too noisy
to read, and had a slip in the eager set, so it isn't used. Times below are the least of the warm passes, the least
contaminated by other jobs; the build is the median of three.

| `P` | SPIR-V | WGSL | WGSL ratio | build (s) | 12 gallery (ms) | one creation, ratio | eager `12 + P` (ms) | a pair's first creation (ms) |
|---|---:|---:|---|---|---|---|---|---|
| 0 | 766,248 | 1,004,550 | | 21.5 | 958 | | 958 | |
| 1 | 813,344 | 1,064,947 | 1.06 | 20.1 | 1019 | 1.06 | 1101 | 325 to 885 |
| 4 | 954,368 | 1,246,645 | **1.24** | 22.4 | 1178 | **1.23** | 1591 | 239 to 255 |
| 8 | 1,142,400 | 1,489,065 | 1.48 **fail** | 22.4 | 1410 **fail** | 1.47 **fail** | 2345 **fail** | 243 to 1006 |

**The real pair costs more than twice the stand-in**: 60 KB of WGSL and 47 KB of SPIR-V each, against 26 and 18. Module tests
are exact, so **`P = 4` passes C2 at 1.24 and a fifth pair would fail it at 1.30.** Four is the ceiling, not a comfortable
margin, and C1's limit, read in the first run as about 6, isn't what binds. C1 passes at every `P` on this run, with a spread
from 14 to 38 s between repeats, so that ratio can't be read finely here. C3 passes at 1.6 s. C3′ passes at 1.23. Each pair adds
about 56 ms to start-up across the gallery, built or not.

**The Windows run, the same criterion unchanged.** Vulkan on the runner's RTX 2080 SUPER, Windows 11, GitHub run 42270588241 from `measure/chart-pairs` at 6543782c. All 12 arms reported, three repeats of the four `P`, each kernel test passing; the job shows red only through a mistake in the workflow's final check, not the measurement. Results: `docs/incoming/2026-11-06_chart_composition/checks/windows-pairs-results.txt`. The module bytes equal the Mac's exactly at every `P`, so both machines measured the same modules. Times are the median of each arm's last two passes, then of the three repeats, as the criterion says; the Mac's are the least of its warm passes, because of its load, so compare ratios, not milliseconds.

| `P` | WGSL ratio, both | build (s), Mac / Windows | 12 gallery (ms), Mac / Windows | gallery ratio, Mac / Windows | eager `12 + P` (ms), Mac / Windows | eager minus gallery, per pair (ms), Mac / Windows | a pair's first creation (ms), Mac / Windows |
|---|---|---|---|---|---|---|---|
| 0 | | 21.5 / 26.3 | 958 / 2729 | | 958 / **2703 fail** | | |
| 1 | 1.06 | 20.1 / 28.0 | 1019 / 2869 | 1.06 / 1.05 | 1101 / **3053 fail** | 82 / 176 | 325 to 885 / 326 to 338 |
| 4 | **1.24** | 22.4 / 32.7 | 1178 / 3390 | **1.23 / 1.24** | 1591 / **4397 fail** | 103 / 246 | 239 to 255 / 367 to 373 |
| 8 | 1.48 **fail** | 22.4 / 39.2 | 1410 / 4076 | 1.47 / 1.49 **fail** | 2345 **fail** / **6743 fail** | 117 / 330 | 243 to 1006 / 419 to 431 |

Against the criterion, on Windows:

| Test | `P = 4` | `P = 8` | Note |
|---|---|---|---|
| C1 build, +30 s and ≤ 1.25 | **pass**, +6.4 s, 1.243 | **fail**, 1.49 | a clean read here, unlike the Mac's, and it binds |
| C2 module ≤ 1.25 | **pass**, 1.24 | **fail**, 1.48 | exact, and the same on both |
| C3 eager `12 + P` ≤ 2 s | **fail**, 4.40 s | **fail**, 6.74 s | **fails at `P = 0` too, 2.70 s, with no pairs at all** |
| C3 one creation ≤ 1.25 × baseline | **pass**, 1.24 | **fail**, 1.49 | as on the Mac, from the gallery's time over its 12 pipelines |
| C3′ 12 gallery ≤ 1.25 × at `P = 0` | **pass**, 1.24 | **fail**, 1.49 | at the limit: the first repeat alone reads 1.251, and the least of the warm passes 1.248 |
| C4 validates, fits 24 floats | **pass** | **pass** | the same module |

**On every test that moves with `P`, the machines agree: 4 passes and 8 fails.** So the lower-number rule picks nothing. Windows makes the ceiling tighter, not lower. Scaling by the measured slopes, a fifth pair would read about 1.30 on C1 and on C3′ as well as on C2, an extrapolation for the two timed ones.

**C3's 2 s is the one test that doesn't pass, and the criterion's two sentences read differently on it.** The test as written is "on this Mac", which leaves the 2 s the Mac's and 4 passing it. The re-measure also says "A criterion is not relaxed for either machine"; applied that way the 2 s fails at every `P`, zero included, so no count passes on Windows and the rule "the largest `P` passing C1 to C4" gives no number. Neither reading gives a number other than 4, and neither was changed to get there: the failure is the existing gallery's on this machine, about 225 ms a pipeline at `P = 0`, and pairs only add to it. The 2 s was this design's own number, for veto, so **the reading is the owner's, in RQ-716**, beside #427's start-up budget. With pairs created on first use (V7), the Windows start-up at the cap is the gallery's 3.39 s against 2.73 s today, about 165 ms a pair built or not, and a pair's first use about 0.37 s.

**Three cautions.**
- *A measurement only.* It gates nothing (R-649).
- *Possibly faulty hardware.* The owner reports this card may have faulty cores, so these times are one possibly faulty card's. The sizes don't depend on it.
- *The first arm ran cold.* The run's first arm took 119.5 s to build against 26.1 and 26.3 for its repeats, and 3340 ms on the gallery's first pass against 2779 and 2794; the medians set both aside, but comparing absolute times across arms carries that cache state. **Eager minus gallery within an arm** shares the module and the cache state, so it's the sound comparison: about nothing at `P = 0` (−35 to −4 ms), and per pair 176, 246 and 330 ms, near one gallery pipeline at that `P` (239, 282 and 340 ms). An eager pair costs what a creation costs, then, and no cache is hiding it. Eager first passes run about 60 to 85 ms a pair above the later ones at `P` = 4 and 8, and Windows load wasn't recorded.

**So 4 stands as the proposal, more tightly, with C3's 2 s on Vulkan open.** The run the cap was waiting for is in; the number is the owner's in RQ-716. `stage_blend`, if
live paths are adopted (§ 17.12), is one more entry in the same module, and at about a pair's size it takes the fourth slot.

**What this can't say.** Of the Mac's run: one Mac. Times are under the queue's other jobs, whose load ranged from 4 to 42; sizes are exact
and times aren't, so C1 and C3's small ratios carry that noise, and `P` = 4's eager pass is a bare one. Metal alone,
and with the Windows run Vulkan on one card: nothing here says what DX12 or WebGPU pay, or another Vulkan driver; wgpu and naga's parse is shared, the driver's compile isn't, and on
Metal the disk cache hides the first compile unless defeated, which the second run did with salted entries. A pipeline's
GPU run time wasn't measured, nor was CI's own build time. The first run's stand-in was a heavy region-D pair; the re-measure
uses the real pair, in copies, since the other three aren't built. The measurement branch is a throwaway, not in the repo.

---

## 12. Tests that can fail

Each with a control (R-176), named here and owned by a task.

| Test | Asserts | Control | Task |
|---|---|---|---|
| `presets_bitwise` | every `kernel::chart::REGISTRY` chart, each gallery preset and a saved chart of each kind (latent oblique, curved with tilt) decodes bit-for-bit to a digest fixture written **before** the first change; `SimConfig`'s canonical form unchanged | one mantissa bit flipped in a digest; a default field serialised | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `closed_inverse_round_trip` | each closed row, each pair: `forward(inverse(x)) = x` over its domain, f64 and f32 at their tolerances | a named mutation per row that fires there, § 12's round-trip rule | TASK-M2-68 (R-916; *was* TASK-M4-34) |
| `size_only_refused` | `I`, `R`, `ℓ` refused by name; the sampled check of § 3 passes | a registered row constant on decodes but declared `free` fails | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `pair_table_complete` | § 6.1's rank test agrees with the table in both directions | a table row removed; a rank-2 pair added to it | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `constraint_axis_refused` | every row of § 6.1, in code, by name; `Some(0)` included; the view through `resolve` is unreachable | the flag cleared on `(L_z, E)` lets a chart through and the arm fails | TASK-M2-69 (R-916; *was* TASK-M8-118) |
| `numerical_branch_seam` | a numerical pair reports its rule, per-pixel branch and seam; the rule is a pure function: the same pixel in two orders gives one bit pattern; a fixture of known solvable samples solves, with no failures | a rule reading a neighbour fails; the area-perimeter pair with no rule; the solve's target mutated to `F(z) + x`, a wrong root written | TASK-M2-73 (R-916; *was* TASK-M8-102) |
| `region_d_plane` | every Table 1 row's marked pixel decodes to the import's state; eq. 8's edges; eq. 3 momenta at `√ℓ`; `I = 1` | body 0 and 1 swapped; `q` not scaled by `√ℓ` | TASK-M2-68 (R-916; *was* TASK-M4-34) |
| `stereographic_identity` | § 8.2's line at random masses, against a state built from `A₁, A₂, A₃` by dd_decoder §3.2 | `μ_ρ/μ_λ` inverted; `d` dropped | TASK-M2-68 (R-916; *was* TASK-M4-34) |
| `hold_lz` | `L_z = L*` to tolerance on every chart allowing it, a near-rigid fixture included (`‖δv‖/‖v‖` from 10⁻¹ to 10⁻⁷ at f32), and an exactly rigid start, whose § 5.3 figure, for the option RQ-715 rules, sets the tolerance; `Σp = 0`; both held: `E*` too; off: bitwise | `η_E` applied alone with both on fails `L_z`; one projection fails the near-rigid fixture | TASK-M2-69 (R-916; *was* TASK-M8-118) |
| `hold_outcomes` | § 5.3's six outcomes in order, each on an exact fixture; a tagged pixel's momenta unchanged; a rest pixel on the floor becomes `(L*/I)·Jr`; at f32, above the floor and on it, the state the second projection leaves at `K̂ = 0`: masses `(0.25, 0.25, 0.5)`, positions `(∓√2, 0)`, `(0, 0)`, `v = 0.005·Jr`. Under RQ-715's option (i): the outcome tags bit-identical at f32 and f64, Metal and lavapipe beside the CPU, on identical `(m, r, v)` across a fixture grid on both sides of `τ`, that state and exactly rigid draws included; no NaN on any arm. Under (ii): rigid draws are exempt from test 12's tag equality; each float's outcome is finite | the order swapped at 2 and 3; under (i), a tag from `s = 0` and `K̂ = 0` differs on the rigid draws; under (ii), the `K̂ = 0` test removed gives NaN | TASK-M2-69, its Metal and lavapipe arm TASK-M8-118 (REQ-CHART-179) (R-916; *was* TASK-M8-118) |
| `hold_rank` | a momentum plane containing `Jr`, one through rest under hold `E` alone, and one through rest and one through a rigid rotation with both held, each report `ray-degenerate`; a generic plane doesn't | the label removed; a plane through a rigid rotation under hold `E` alone reporting rank loss | TASK-M2-69 (R-916; *was* TASK-M8-118) |
| `hold_encode` | Part 6 step 4 reads the held decode; a state off `{L_z = L*}` gets the stated outcome | step 4 on the unheld decode lands on a wrong pixel | TASK-M2-69 (R-916; *was* TASK-M8-118) |
| `trajectory_polar_unfolded` | the figure-eight, run by the kernel: `φ = acos(n_w)` from the signed `n̂` at every macro-step lies on both sides of 90°, crosses it at every syzygy, and the sign of `n_w` matches the triangle's signed area from the positions; R-544, the owner's second rider to RQ-736 | the polar angle folded, `min(φ, π − φ)` | TASK-M8-58 |
| `symmetry_filter_display_only` | filter on and off: one sim key, payload and statistic, bit for bit; the toggle on the GUI and on export | the filter feeding a statistic | TASK-M2-84 (R-916; *was* TASK-M8-120) |
| `symmetry_filter_rest_reconstructs` | the owner's: on a rest chart, filtered + its mirror reproduces the full chart bitwise, outcome fields equal, signed fields flipped; fixture a third-body rest chart declaring the mirror only, one copy, whose pairs decode to exact mirrors | a signed field not flipped; the canonical side off by one row | TASK-M8-120 on real runs; on a synthetic payload TASK-M2-84 (REQ-CHART-175) (R-916; *was* TASK-M8-120) |
| `symmetry_filter_fixed_velocity` | the owner's: on a fixed-velocity shape-sphere chart, the `p_ρ = (0.1, 0.1)`, `p_λ = (0, 0)` fixture relative to `ρ̃` on `+x`, the test first asserts `Π·p ≠ p` after signed `C`, then that the filter blanks nothing and reads "no duplicates in this chart"; the same chart at `p_ρ = (0.1, 0)` does blank | the partner check removed, blanking by the map's `(θ, π − φ)` alone | TASK-M2-84 (R-916; *was* TASK-M8-120) |
| `symmetry_filter_toggle` | the toggle on the chart's panel and in the export dialog; its no-partners state; one click to the full disc with no new run | the toggle left live with no partners | TASK-M8-120 |
| `symmetry_filter_statement` | the legend and sidecar state the canonical choice; a paper-frame preset opens with it on, any other chart with it off | the statement dropped; the default on for every chart | TASK-M8-120; its region-D area check TASK-M2-84 (REQ-CHART-176) (R-916; *was* TASK-M8-120) |
| `baked_cap` | baked pairs in `stage::decode` at or under the ledger's constant | one entry over | TASK-M4-33 |
| `excluded_primitives` | no row with an inverse names a binary phase, eccentricity, impact parameter or fixed `k` (R-669) | one added | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `region_d_native` | § 8.8's table: read-back within tolerance **and** the paper's three separations in the paper's own frame against the decoded state, eq. 4's energy on AA rows | `d` dropped on both sides; bodies 0, 1 swapped at unequal masses; `η` mirrored | TASK-M2-75 |
| `preset_paper_coords` | § 20's table: the same oracle per frame; offsets below the working float at f64 only, neighbours distinct | the same three; a sub-resolution row at f32 | TASK-M2-77 |
| `prim_round_trip` | each closed row of §§ 3.1 and 3.2, `forward(inverse(x)) = x` on the axis, f64 and f32, in eps, measured worsts here, the row's task setting the tolerance and its margin: focal flat, 1 eps on `(Z, σ)` at f64, 0.5 to 1 eps at f32, down to `η` near 10⁻⁴; polar about the COM, as its row's forward is, `6 eps·r_max/r_min`, measured 135 eps at `\|det\|` from 10⁻¹ and 8,644 from 10⁻⁵ at f64, about 5.3 eps·`r_max/r_min`. A separate clause, `inverse(forward(x)) = x`, takes the row's scaling, also measured: `0.35 eps/η` for § 3.2's focal row, `0.5 eps·r̂/\|det\|` for § 3.1's polar row, 0.31 at Dirichlet masses and 0.41 at masses log-uniform to 10⁻³, 300,000 draws each. Saari's split sums and is orthogonal; `K_shape = I\|ṅ\|²/8`; `I₁I₂ = n_w²/4`; `h`'s step keeps `L_z`; the aliases equal; Tanikawa–Mikkola's mass-weighted `L_com = 0` at random masses after `C`, from an independent `Σ m r × v`; each case also reads back the row's `Holds` | a named mutation per row that fires there; `R` from the centroid, −0.0025 at § 3.2's point; an unweighted sum | TASK-M2-74, TASK-M2-76 |
| `prim_rejected` | each row of "Rejected as axes" has no axis; `L²E` and Hill's distance label `ε ≠ 0`; Hill's distance blank at `E ≥ 0`; pair table rows 19 and 20 refused, rows 17 and 18 refused only under a hold, `(L_z, E)`, `(K_ρ, K_λ)`, `(L_ρ, L_z)` accepted | one accepted; `(L_z, E)` refused | TASK-M2-74 |
| `round_trip_complete` | every registry row with an inverse, closed or numerical, has a `forward(inverse(x)) = x` case, a read-back of its `Holds` and a named mutation control; a row without one fails by name | a case removed; an inverse added with no case; a `K` case with its momentum direction wrong | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `primitive_class` | every row carries § 3.3's class; a row declared conserved and exact holds along a short run at `ε = 0`; a row declared `approximate` shows the label; a functional is refused as an axis by name | a state row declared conserved; a functional accepted | TASK-M2-66 (R-916; *was* TASK-M4-33) |
| `seam_jitter_flagged` | on a composed chart the copies are the sampling note's offsets in `(u, v)`, whatever the seam; a footprint across a numerical seam, a closed-form CPU batch row's two-root branch or mask, polar's branch line or a baked chart's mask, is flagged, its `E + 1` copies all in the pool, its spread and `error_ratio` carrying `seam-contaminated`; two orders, one flag | a copy dropped at the seam; the flag read from a neighbour | TASK-M8-102 |
| `seam_refinement_masked` | a flagged footprint doesn't fire `unresolved`, a mask-blanked copy included; a quad with any flagged footprint and none unresolved takes `Seam`, a mixed quad included, with `n_seam` counted, and its `coherence` doesn't raise its priority; a non-finite copy on it still fires; an unflagged neighbour with real spread still splits; a chart of today's registry bitwise today's | the mask removed: quads split along the seam at every depth; the mask applied to an unflagged footprint; `Keep` taken for `Seam`; `Seam` taken only when every footprint is flagged | TASK-M8-102 |
| `statistic_measure` | a statistic over a warped chart is weighted into a named measure or labelled `per pixel of this chart`; a fixture of known fraction recovered through a power warp; a fraction in a paper's own measure equals the flat grid in the paper's coordinates | no weight and no label; the weight inverted; a count of quadtree leaves fed in | TASK-M2-71 (R-916; *was* TASK-M8-100) |
| `ftle_chart_independent` | a linear chart, a warped one at a pixel where the warp's value is exact, and a third with a different tangent through the same state, a tilted pair, decode one bit pattern and give one `λ_T` bit for bit, read on copy 0 (§ 6.5) | `δ₀`'s direction taken from a chart tangent, which only the third chart can catch: a warp rescales the tangent and keeps its direction | TASK-M3-69 (per R-369, following R-924; *was* TASK-M3-14 (R-918), and before it TASK-M8-100) |
| Golden | `cargo xtask golden decode-preset`: region D through the DECODE preset, TASK-M4-34's acceptance (R-914; *was* TASK-M4-06's), unchanged | as that task's | TASK-M4-34 |

The test names `round_trip_complete` to `ftle_chart_independent`, `trajectory_polar_unfolded`, the `symmetry_filter_` tests and `path_lower_half_stays` (§ 17.14) are this design's proposals; the two the order asks for are the owner's.

*9 Nov 2026 (R-916, R-919):* the Task column names the M2 task that builds each test's maths; a test that reads the GPU, the
real march or real outcome fields keeps its M4 or M8 task, as each row says. **Tolerances (G5 of M2's gate, R-917):** a
round trip's tolerance this section gives only as measured worsts is a calibration (R-71), proposed by the row's task with
its criterion written first and set by measurement (R-649, R-650), never chosen to make the test pass, and confirmed at
the M2 gate: REQ-CHART-180 (§ 3's closed rows and pairs), REQ-CHART-181 (warps and pair maps), REQ-CHART-182 (the curved
slices' targets), REQ-CHART-183 (numerical inverses), REQ-CHART-184 and REQ-CHART-185 (§§ 3.1 and 3.2), REQ-CHART-186
(paper frames) and REQ-CHART-187 (the virial row). Region D's native read-back keeps § 8.8's `4ε·max(1, |ξ|, |η|)`, and
Table 1's rows REQ-VAL-120's tolerance, a calibration confirmed at the M2 gate.

**Round trips: every claimed inverse (the owner's review of 7 Nov).** Every primitive with a claimed inverse ships with a
`forward(inverse(x)) = x` test that can fail, with a mutation control: one inverse made wrong on purpose, the test going
red (R-176). **Identities alone aren't enough to build on:** `prim_check.py`'s identities test the formulas once, in Python,
and say nothing of the inverse that ships. `round_trip_complete` makes the rule structural: an inverse with no case can't pass.

**The control must fire on its row.** A sign flip does only on a row odd in that sign. On a scale solve (`K`, `E`, `K_ρ`, `K_λ`,
`K_shape`, `‖p‖`, third-body speed, the virial row) negating the scale reverses the momenta, a different system, and
`forward(inverse(x)) = x` still passes, so the control there is a factor or an exponent changed. **Each case also reads back the
row's `Holds`:** `forward(inverse(x)) = x` reads only the axis, so `K` solved into the wrong momentum direction would pass. A
numerical inverse's control mutates the target, `F(z) + x`, so a wrong root is written: a flipped residual only sends Newton
away, every sample goes `decode_failed`, and nothing is written to fail. So its test also asserts that a fixture of known
solvable samples solves.

**What § 3's solves hold.** § 3's table has no `Holds` column, so here: `K`, `E` and the virial row scale the momenta, holding their direction (inverse_encode_contract Part 5, kind 3). `L_z` doesn't: it holds `v`'s part orthogonal to `Jr` in `⟨·,·⟩_m` and sets the rigid part (`v^(L)`, inverse encode contract Part 5, kind 3). Each case reads that back.

| Inverses | Test | Mutation control | Task |
|---|---|---|---|
| § 3's closed rows and pairs | `closed_inverse_round_trip` | named per row, as above | TASK-M2-68 (R-916; *was* TASK-M4-34) |
| warps, polar, log-polar | `warps_and_pair_maps` | named per inverse | TASK-M2-71 (R-916; *was* TASK-M8-100) |
| the curved slices' targets | `exp_map_targets` | named per target | TASK-M2-72 |
| numerical inverses | `numerical_branch_seam` | the target mutated to `F(z) + x` | TASK-M2-73 (R-916; *was* TASK-M8-102) |
| § 3.1's closed rows | `prim_round_trip` | named per row | TASK-M2-74 |
| § 3.2's closed rows | `prim_round_trip` | named per row | TASK-M2-76 |
| region D's exact map | `region_d_native` | a sign flipped on one side only: `d` dropped on both sides passes a round trip, which is why the oracle stays | TASK-M2-75 |
| paper frames | `preset_paper_coords` | as `region_d_native` | TASK-M2-77 |
| the virial row | `virial_descriptor` | a factor changed in its inverse | TASK-M2-70 |

Forward-only overlays and combinations classed `none` claim no inverse, so they need no case. Hold `L_z` is a constraint,
not a primitive: it is many-to-one, and `hold_encode` tests its read-back.

---

## 13. Reversible choices: decided, for veto

Each states its criterion, written first, then the result. Any may be vetoed; each is recorded here and changes no ruling.

| # | Choice | Criterion | Decision | Evidence |
|---|---|---|---|---|
| V1 | A warp's kind: a uniform word, not a baked variant | bitwise on the linear arm; the branch taken before the decode, as `ShapeMap`; TASK-M4-33 measures decode time on the GPU, and past +2% it becomes baked (R-916, REQ-PERF-116; *was* TASK-M8-100, the warp itself TASK-M2-71's) | uniform | precedent; the measurement is that task's |
| V2 | Keep decode an entry of its own, apart from the integrator | the variant count must stay additive, § 11 | keep it | § 11's numbers |
| V3 | Region D by explicit configuration, not `atan2` plus the link | the lower worst f32 error against f64 over D; a tie goes to the lowering appendix's row | explicit | worst `1.28·10⁻⁷` against `2.18·10⁻⁷`, 400,000 samples at equal masses and `η > 10⁻⁴`; `1.24·10⁻⁷` against `1.98·10⁻⁷` with no cut on `η`; `2.14·10⁻⁷` against `3.67·10⁻⁷` at masses log-uniform to `10⁻³`, `docs/incoming/2026-11-06_chart_composition/checks/regiond_f32.py` |
| V4 | The registry on the CPU, `chart_authoring` | `kernel` must not learn a kind (REQ-CHART-004) | CPU | by construction |
| V5 | Angles and side ratios: their own entries, or a warp word on the third-body entry | fewest baked entries, with `closed_inverse_round_trip` green at f32 | decided by TASK-M4-34's test | not measured here |
| V6 | Pair-table completeness by a rank test on a grid | a table alone can drift; a test finds a missing row | table and test | § 12 |
| V7 | Pair pipelines created on first use, not eager at start-up | § 11's C3 | on first use | § 11; re-read if #427 changes the eager set |
| V8 | A 1D chart is an annotation axis | no new lowering; statistics rule already written | annotation | contract Part 5 |
| V9 | Hold `L_z` solves by the rigid rotation, not in the hidden momentum directions | the least velocity change `‖Δv‖_m`; closed form; independent of which axes are swept | rigid | § 5.3; the physics note |
| V10 | A chart holding `L_z` by construction reads the hold as its held value | no double write; R-720's by-value | as such | § 6.1 |
| V11 | Numerical solves: Newton from a fixed start list, no continuation | a pure function of the pixel | as such | § 7.3 |
| V12 | *Retired (physics note 2, P5):* the bend needs no orientation choice | | | § 17.6 |
| V13 | Orthogonal planes pair `q₁ → q₁′`, `q₂ → q₂′`, and `θ₂ = 0`'s free axis by `q₂`'s sign; the in-plane rotation eases on the same `s` | deterministic per segment; ends bit for bit | as such | § 17.7 |
| V14 | Alignment by Procrustes on the 16 × 16 probe grid | a pure function of `(A, B)`, recorded; travel not worse than none | as such | § 17.10; not measured on nonlinear ends |
| V15 | A frame is `degenerate` with a probe below `10⁻³` or above `10³` of its median Gram factor | flags sphere to plane, a linear pair-map blend and a vortex, and no affine rotation | `10⁻³`, `10³` | a design value; TASK-M2-81 tests it on those fixtures |
| V16 | Export doesn't cache each end's blocks per pixel | cache only if the CPU side passes 1 s | no cache | 0.11 s, § 17.12 |
| V17 | Re-timing: 33 knots, 16 × 16 probes, on the decoded blocks | spread at most 1.25 on the saturated fixture | as such | 1.12, § 17.9 |
| V18 | Live target: 4 ms for one coarse cover's chart side | written before measuring | 4 ms; missed on the stand-in, about 15 ms | § 17.12 |
| V19 | Re-centre once per end and segment, not per frame, unless the anchor moves | a pure function, so a repeat gives the same bits | once | § 17.8 |
| ~~V20~~ | ~~A saved mid frame reopens as a record, with no tilt or slice~~ | ~~it has no hidden coordinate; labelled~~ | retired 11 Nov 2026 (R-944): a saved frame tilts and slices, the edit going to both end charts | § 17.5 |
| V21 | A pixel blank or tagged at either end is blank mid-path | no invented state (R-470), no undeclared seam (P3) | as such. *11 Nov 2026:* only an end pixel with no state stays blank, a tagged end that carries a state blending as one (applied per R-369, listed for veto, conditional on physics review; until it confirms, as such) | § 17.11 |
| V22 | The dilation axis is signed `h`, not `K_dil` | an axis has to invert | `h` | `K_dil` loses the sign, § 3.1 |
| V23 | Aliases are one row, not two: body 2, `γ_ρ`, `K_rot`, `K_dil`, elongation; the midpoint angle at `m₀ = m₁` only; Hill's distance at a frozen mass | one quantity, one row; `pair_table_complete` agrees | aliased | § 3.1, identities to 10⁻¹³ |
| V24 | Every § 3.1 and § 3.2 row is CPU batch | the cap's slots are taken | CPU batch | § 11 |
| V25 | Hill's distance blank at `E ≥ 0` and `ε ≠ 0`, labelled, not refused; `L_z = 0` reads −1, true | undefined or not a function of `c` there; a defined value at `L_z = 0` | label | § 3.1, P18 |
| V26 | Stored entries at most `K = H`, by distinct boundary, the required set being the rows' boundaries; `2H` a labelled extra | closest to the temporal note; `T` the stored top | `H` | § 18.3 |
| V27 | Transport stays paused on leaving the mode | a frozen playhead never restarts on its own | paused | § 19 |
| V28 | `t_lo` is new beside the horizon; the window `t_lo, t_hi` a view, off the sim key, a Navigate lane field | zoom in time is navigation; `T` stays on the key | as such | § 18.2 |
| ~~V29~~ | ~~Time morphs need equal windows and warps; else a cut~~ | ~~"time fixed"~~ | retired 11 Nov 2026 (R-926 item 3: time blends linearly, so two time windows blend like any two windows; applied per R-369, listed for veto) | § 18.4 |
| V30 | Time up the vertical axis, `t_lo` at the bottom; time on either axis allowed | the order's "vertical" | vertical | § 19 |
| V31 | `Ctrl + / −` zooms time | a proposed binding, for the gui reviewer to check against §G3 | `Ctrl` | § 19 |
| V32 | `√chazy_qk[2]` is the third body's speed row | the corpus's own ratio | as such | § 3.1 |
| V33 | Presets use § 8.8's ratio, no per-paper function | one source | ratio | § 20 |
| V34 | Each principal angle of the plane geodesic is `atan2(‖w‖, A·B)`, not `arccos` | an implementation choice that changes no result; applied per R-369 | `atan2` | § 17.7 |
| V35 | A new column marches to `t_hi`, and a window moved past its front marches it on, not every column to `T` up front | the order's "extending the window to later t integrates further per column"; it changes no result, a continuation being bitwise and the store deciding what is ready, never what a row shows; applied per R-369 | on from the front | § 18.3 |
| V36 | `seam_straddled` is evaluated in the resolve pass (R-142) from the pixel and the copy index; any CPU batch chart's branch is read from the IC record, one new word, a mask from its `decode_failed` | a pure function already holds it; R-100's recreate-from-image is untouched; no crossing is added | derived | § 7.4 |
| V37 | A composed chart's mask counts as a seam for § 7.4; today's registry is out of scope, the owner's item being composed charts | the same contamination, and its "unusable copy" would otherwise hold a quad unresolved at the mask's edge | counts | § 7.4 |
| V38 | Seam masking is by footprint, the flag's own unit, one footprint wide | the owner's "masked" gives no width | a footprint | § 7.4 |
| V39 | Statistics default to `per pixel of this chart` until a measure is defined | the owner's two options, and the second is honest today; no value changes | the label | § 6.4 |
| V40 | § 3.3's class is a registry field, tested | the owner's "classify every registry primitive" | a field | § 3.3 |
| V41 | Flagged footprints take a new decision, `Seam`, with a `QuadReduction` count, `n_seam`; a quad of them reads full coherence | `Keep` reads `n_unresolved == 0` as resolved, and `Undetermined` means finer `eta` | `Seam` | § 7.4 |
| V42 | With Time on the horizontal axis the chart transposes the screen's coordinates, so `u` is always the IC coordinate | V30 allows either axis; a transpose changes no result | transpose | § 18.1 |
| V43 | A partner outside the view but in the chart's domain still blanks its pixel | the order's "in the chart's domain"; a zoomed view doesn't change a statement | domain, not view | § 10.4 |
| V44 | A self-partner is kept | "keeping one canonical representative" | kept | § 10.4 |
| V45 | The filter's mask is built on the CPU, off the sim key, and read by the display pass and the export writer | "applied after rendering" | CPU | § 10.4 |

---

## 14. Doors: the rulings items

Each is an RQ, written out with its file, section, quoted text and options in its `REVIEW_QUEUE.md` entry.
Nothing below is decided here.

| Item | The door | Recommendation |
|---|---|---|
| RQ-712 | "A new axis kind" against "a closed set"; R-798's reading: does "no new chart type" reach beyond PA-1? | keep four kinds (agreed in principle by the owner, not a ruling); yes, every new capability is a builder option |
| RQ-713 | A side-ratio axis in the builder against "There is no `SideRatio` chart" | allow the axis; PA-1 keeps its remap |
| RQ-714 | Region D's definition: domain (a), (b) or (c); eq. 3 for AA-3 and AA-5; the horizon | (a), outside D blank and uncounted (agreed in principle by the owner, not a ruling), overtaken for the presets by RQ-736's filter order, § 8.4; yes; the comparison cuts by pixel |
| RQ-715 | Hold `L_z`: the physics of § 5.3, and the held half of RQ-701 | (a) amended, as § 5.3, on the physics reviewer's note |
| RQ-722 | Whether a rest chart refuses hold `L_z`, or labels it "not at rest" | the owner's choice; the physics reviewer leans to a declared flag |
| RQ-716 | The axis cap; C3's 2 s on Vulkan | 4; the Windows Vulkan run agrees on every test that moves with the count, and C3's 2 s fails there at every count, none included; § 11. The start-up budget itself is issue #427 |
| RQ-717 | The virial kind: RQ-682 | RQ-682 (a) |
| RQ-718 | R-669: a user-built plane that lands on a refused one | it may be built, is never scored, and the row stays |
| RQ-719 | Seam 3: a CPU-filled IC buffer | an exception for labelled batch views |
| RQ-720 | R-594 item 5: "Do not build charts ahead of it", against general polar, log and curved options | it governs charts built for a target; the options are the owner's |
| RQ-721 | Chart paths: does a path type meet the bitwise constraint; export contract Part 3 and REQ-TOOL-075 against morphing; `z` or blocks; the default segment; live | a path type, tilt and zoom as arms; the rules give way; blocks; morph the default for a new segment; R-520 and R-669 stand |
| RQ-723 | Inner and outer elements, Mardling–Aarseth: axes against R-668's parking and R-669 | forward-only overlays; no axis |
| RQ-724 | The Saari split and RQ-680's dilation rows | define `h`, RQ-680 (b), forward first |
| RQ-725 | Fixed `k` and symmetric families against R-669 | R-669 stands; `SuvakovPlane` stays one M9 task; a rigid-rotation hold is a user's picture, never scored |
| RQ-726 | The scattering primitives against R-669's "no binary-plus-single chart" | definitions and overlays only; R-669 stands |
| RQ-727 | Time as an axis: mode or fifth kind; stored rows against "store no history"; `T` against the caching contract; `Δ_sync` | a chart mode, no fifth kind; `K = H` by distinct boundary; amend the note and the caching row; time zoom floors at a boundary |
| RQ-728 | Region D at unequal masses: labelled D, Anosova's half-disc, and reach | the plane is geometric; D a labelled mask; AN-1 its own preset |
| RQ-729 | Paper frames the library doesn't settle | presets where read; each gap unbuilt |
| RQ-730 | Hill's threshold at unequal masses: labelled pair, `max`, or median | physics, the owner's; labelled pair |
| RQ-731 | REQ-VAL-326's pointer to the 2015 paper | correct it to the 2008 paper |
| RQ-733 | A user-typed combination with no registered inverse: an overlay only, or a general solve | registered inverses only; an overlay otherwise |
| RQ-734 | Final-condition charts (the model's Theorem 11): whether to build; the occupant; the gauge; the mirror half; line 243's rejection | build later, as a CPU batch chart: the default occupant, the round-trip miss measured and masked per pixel, an exactly reversible integrator the option; the start kept signed, the mirror half never folded; keep `I = 1` and label it; narrow the rejection to event reads |
| RQ-735 | The browser build: closed-form GPU charts only, or a WASM CPU route for numerical axes and CPU-only morphs | the WASM CPU route, reasoned from R-571; nothing measured |
| RQ-736 | The chart-space model against the corpus | ruled by the owner on 7 Nov: the corpus and § 17 stand, and the model is corrected to match; its open questions ruled the same day; the symmetry filter (§ 10.4) gives the papers' domains; closes in the rulings PR, with a correction list and the filter's conflicts |

**New extension types: none proposed.** Polar's `ϕ` and `r` declare existing types, under R-407 clause 3's physics review.

---

## 15. Plan

**Ruled 9 Nov 2026 (R-916 to R-919): M2 is the whole non-visual chart system, and this is its plan.** The primitive
registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels,
curved slices, CPU decode and encode through the composition, and the non-visual path maths are M2's, each an engine
function with its tests, landing before any further downstream chart work, today's charts and view operations
re-expressed bit for bit first (R-916). GPU lowering (M4), contours and display (M7), and the GUI's routes, panels,
Sequence, time-axis UI and overlays (M6, M8) keep their milestones, each a thin adapter onto the M2 core, built after
M2's gate (R-917's order: the chart system, then the integrator, then everything visual). A task that mixed maths and
visuals is split, its maths to a new M2 task and its visual remainder keeping its old id. The tasks wholly of the core
moved to M2 and were renumbered; a record before R-916 that names an old id means the new one (R-916's Was/Now table).
The text after this plan, from "**Before R-916, for the record.**", is the plan as it stood, kept as written.

**The stage order** (R-863's five stages, now M2's; each stage's tasks depend on the stage before and merge only with its
tests green):

| Step | Tasks | What |
|---|---|---|
| Before stage 1, docs first | TASK-M2-58, TASK-M2-59, TASK-M2-60 | the rulings carried into this design, its status no longer Proposed; the composition and the chart paths in the contracts |
| Before stage 1, finishing | TASK-M2-38, TASK-M2-56; TASK-M2-39 | the navigation stop and the saved Lean re-tilt, which the core calls, not redoes; the past-edge pixels before the baseline, since they change existing pixels |
| Any time | TASK-M2-82 | the trajectory interface and its exact fixtures (R-918, § 18.7) |
| Stage 1 | TASK-M2-61, then TASK-M2-62 and TASK-M2-63 | R-864's type sketch to the owner, approved when TASK-M2-61 brings it (R-919), then the baseline commit (`fixtures/chart_composition/digest.json`), then today's operations re-expressed bitwise; locking without moving the view; Save view as chart on curved views, the pivot at a centred pin, the held-coordinate tilts (RQ-679) |
| Between stages 1 and 2 | TASK-M2-64 | the consolidation (R-894): one leaf per operation through REQ-CHART-060's stop, the merged GUI routes thin adapters (R-919, F1 (a)) |
| Stage 2 | TASK-M2-65 | locked paths: the based morph, the sliding anchor, the owner's lock-with-morphs gate test (R-902, R-919 F10) |
| Stage 3 | TASK-M2-66 to TASK-M2-80, TASK-M2-55, TASK-M2-57 | the registry and digest; CPU decode and encode through it; region D on the CPU; hold `L_z`; the virial pair; warps and pair maps; curved slices; numerical inverses; the primitives of 6 Nov and of the full order; region D's exact map; paper frames; the fixed-`k` and scattering charts; points of interest's maths; the saved-chart rest and the latent `β` change as conversions (R-894, R-898) |
| Stage 4 | TASK-M2-81 | path re-timing, alignment and flags |
| Stage 5 | TASK-M2-83, TASK-M2-84, TASK-M2-85 | time-axis mode's maths through the trajectory interface; the symmetry filter's maths; the acute Burrau charts over the full range |

TASK-M2-61's baseline is the digest every later task is held to, TASK-M2-66 included. TASK-M2-55 sits in stage 3: it is new
capability, and the re-expression lands before any (R-864 item 3, R-916).

**After M2's gate,** each an adapter on the M2 core: TASK-M3-69 (FTLE's chart independence, REQ-CHART-132, split per R-369 from
TASK-M3-14, the FTLE shadow, when R-924 resumed it) and TASK-M3-67 (the trajectory
interface on the real integrator, the outcome-class arms) in M3; TASK-M4-33 (baked entries, the cap, V1's decode time),
TASK-M4-34 (region D's baked variant, its lowering row and GPU reference) and TASK-M4-06's GPU band walk in M4; TASK-M7-72
(contours of any primitive) in M7; and in M8 TASK-M8-48, TASK-M8-125 and TASK-M8-129 (the panels, the lock badge, D19 and
D23), TASK-M8-102 (seam flags, refinement, the browser route), TASK-M8-103 and TASK-M8-104 (the builder's panels, 1D
charts), TASK-M8-107 and TASK-M8-108 (live paths, the Sequence's segment and export), TASK-M8-113 to TASK-M8-116 (the
column march, storage, the UI), TASK-M8-118 (`τ` and the GPU tags; `τ` provisional at M2, R-919 F13), TASK-M8-120 (the
filter's display, toggle and real-run test), TASK-M8-126 (points of interest drawn) and TASK-M8-128 (final-condition
charts). TASK-M8-100 is retired, its warps TASK-M2-71's and its FTLE test TASK-M3-69's (per R-369, following R-924; *was* TASK-M3-14's).

**What can't be tested in M2, and where it goes** (R-918): "outcome class unchanged" in the conversion tests needs
integrated outcomes (TASK-M3-67); the time-axis column against the playhead and its extension read the real march
(TASK-M8-113); the owner's `symmetry_filter_rest_reconstructs` on real runs (TASK-M8-120); final-condition charts
(TASK-M8-128); FTLE's independence of the chart (TASK-M3-69, per R-369 following R-924; *was* TASK-M3-14). Nothing of the core is impossible in M2.

**Before R-916, for the record.**

**Nothing here starts until ruled.** TASK-M4-37, the owner's rulings applied, sits in the Depends on of all 26 tasks below, and each names the rulings item it waits on, or the design as a whole; R-847's TASK-M8-122 and TASK-M8-123 wait on it too. Only those 28 tasks wait behind it; no other existing task, in M4 or M9, does.

Tasks, milestones chosen by what depends on them. Nothing in M4 depends on hold `L_z` or the virial pair, so those two sit in M8. **One edge is named, not made:** TASK-M4-06's golden needs RQ-714's domain, so TASK-M4-06 gains TASK-M4-34 in Depends on when RQ-714 rules, TASK-M4-37's Deliverables say so, and from then until TASK-M4-34 merges it holds 230 tasks: TASK-M4-06 and the 217 other existing open tasks behind it, and 12 of the tasks below. *Ruled 9 Nov 2026 (R-914, RQ-752 (a)):* the edge is never made, since it closed a cycle through stage 2; region D's GPU reference is TASK-M4-34's, and TASK-M4-34 depends on TASK-M4-06 instead. The seven M9 edges an earlier draft made are dropped: TASK-M9-18, -20, -21 and -22 have no GPU run (R-671), and -26, -28 and -31 reach TASK-M4-06 through TASK-M4-17.

| Task | Milestone | What | Waits |
|---|---|---|---|
| TASK-M4-32 | M4 | the docs: contract, reference, lowering, §G7, §4.1; § 5.3's three definitions | RQ-712 to RQ-716, RQ-719 |
| TASK-M4-33 | M4 | registry, classes, round-trip completeness, refusals, labels, bitwise digest, cap test | TASK-M4-32, RQ-716, RQ-722 |
| TASK-M4-34 | M4 | the third-body pair, region D's preset, angles and side ratios | TASK-M4-33, RQ-713, RQ-714, RQ-736 for the preset's domain |
| TASK-M8-118 | M8 | hold `L_z` | TASK-M4-33, RQ-715, RQ-722 |
| TASK-M8-119 | M8 | the virial pair's descriptor and entry | TASK-M4-33, RQ-717; RQ-700 at `L_z ≠ 0` |
| TASK-M8-100 | M8 | warps, polar and log pair maps; a statistic's measure; FTLE's independence of the chart | TASK-M4-33, TASK-M3-14, RQ-720 |
| TASK-M8-101 | M8 | curved slices: momentum and mass | TASK-M8-100, RQ-720 |
| TASK-M8-102 | M8 | CPU batch, numerical inverses; seam flags and masking | TASK-M4-33, TASK-M4-06, TASK-M4-34, TASK-M6-03, TASK-M8-100, RQ-719 |
| TASK-M7-72 | M7 | contours of any primitive | TASK-M7-43, TASK-M4-33; the design as a whole |
| TASK-M8-103 | M8 | the builder's panels | TASK-M8-18, TASK-M4-34, TASK-M8-118, TASK-M8-100, TASK-M8-102; the design as a whole |
| TASK-M8-104 | M8 | 1D charts | TASK-M8-18, TASK-M4-33; the design as a whole |
| TASK-M8-105 | M8 | chart paths in the contracts | RQ-721, TASK-M4-32 |
| TASK-M8-106 | M8 | the path type, the line, export through CPU batch | TASK-M8-105, TASK-M8-102, TASK-M8-31, RQ-721 |
| TASK-M8-107 | M8 | live: the blend entry, the coarse cover | TASK-M8-106, RQ-721 |
| TASK-M8-108 | M8 | the Sequence's morph segment | TASK-M8-106, TASK-M8-77, TASK-M8-78, RQ-721 |
| TASK-M8-117 | M8 | paths under a lock: the stored anchor | TASK-M8-106, TASK-M2-23, RQ-721 |
| TASK-M8-109 | M8 | the primitives of 6 Nov, axes and overlays | TASK-M4-33, TASK-M8-100, TASK-M7-72, RQ-723 to RQ-725, RQ-730 |
| TASK-M8-110 | M8 | region D's exact map, every target's native coordinates | TASK-M4-34, RQ-728 |
| TASK-M8-111 | M8 | the primitives of the full order | TASK-M8-109, RQ-723, RQ-725, RQ-726, RQ-730 |
| TASK-M8-112 | M8 | paper frames as presets | TASK-M8-110, RQ-729, RQ-650 |
| TASK-M8-113 | M8 | time-axis chart mode | TASK-M4-33, RQ-727 |
| TASK-M8-114 | M8 | time-axis storage and column cost | TASK-M8-113, RQ-727 |
| TASK-M8-115 | M8 | time-axis mode in the GUI | TASK-M8-113, RQ-727 |
| TASK-M8-116 | M8 | time-axis mode in Sequence and sidecars | TASK-M8-115, TASK-M8-77, RQ-727 |
| TASK-M8-120 | M8 | the symmetry filter | TASK-M8-112, TASK-M8-103, TASK-M8-32, RQ-736 |
| TASK-M8-124 | M8 | the path object; pan, zoom, tilt, slice, lock, chart switch and Save view as chart re-expressed through it, bitwise (R-863 stage 1, R-864) | TASK-M2-14, TASK-M2-22, TASK-M2-23, TASK-M2-40, TASK-M2-52, RQ-721; the owner's answer to R-864's type sketch |

**The build order (R-863, the owner's order of 8 Nov 2026).** Once TASK-M4-37 releases them, the tasks are built in five
stages, each merging only with the previous stage's tests green. Applied per R-369 (R-863), each stage's tasks depend on
the previous stage's, placed so: (1) TASK-M8-124; (2) TASK-M8-117, the based morph with the stored anchor and then the
sliding anchor, which R-849 gives it in place of § 17.8's `anchor not in B` (RQ-744 (D), filed before R-849 merged, is
moot); stage 2's test that the centre's IC is constant applies where the anchor stays in B, and on a sliding path, where
the centre follows the anchor (R-849), it checks R-849's slide instead: the stored words at `s = 0`, B's nearest point
at `s = 1`, continuous between, with no label and no jump; (3) TASK-M4-33, TASK-M4-34, TASK-M8-100, TASK-M8-101,
TASK-M8-109, TASK-M8-110, TASK-M8-111 and TASK-M8-119; (4) TASK-M8-108; (5) TASK-M8-113 to TASK-M8-116 and
TASK-M8-120. TASK-M8-124 is the 26th of this section's tasks behind TASK-M4-37, and with R-847's TASK-M8-122 and
TASK-M8-123 (R-863 names neither) 28 tasks wait on it. What the order leaves open, the tasks that fit no stage or
straddle two and the stage-3 edges that would point forward across milestones or close a cycle, is RQ-744's.

*8 Nov 2026 (R-866, R-870):* two tasks join, each behind TASK-M4-37, so 30 tasks now wait on it:

| Task | Milestone | What | Waits |
|---|---|---|---|
| TASK-M8-125 | M8 | locking doesn't move the view: the pin at the clicked pixel, tilt and zoom about it, "centre on pin" and the auto-snap setting, exact on nonlinear charts or saying why (R-866) | TASK-M8-124 |
| TASK-M8-126 | M8 | points of interest, their marks and "similar systems" overlays (R-870, § 10.5) | TASK-M7-43, TASK-M7-72, TASK-M4-33, TASK-M8-124, TASK-M8-108, TASK-M2-57 |

TASK-M8-125 is in stage 1, after TASK-M8-124's bitwise re-expression, which records today's lock as its baseline, and
TASK-M8-117 (stage 2) depends on it (applied per R-369, R-866). TASK-M8-126 is in no stage: the owner's stages don't name
it, and it joins RQ-744 (C)'s list.

*8 Nov 2026, the owner's reply to that day's report (R-873, R-894, R-875, R-879).* RQ-744 is ruled, and this section's
order now reads: **before stage 1**, TASK-M8-105 (docs first), and, by (E) (i), TASK-M6-49, TASK-M6-51 and TASK-M8-48,
on which TASK-M8-124 depends; **(1)** TASK-M8-124, then TASK-M8-125; **between (1) and (2)**, TASK-M8-129, the
consolidation step (R-894); **(2)** TASK-M8-117, which no longer depends on TASK-M8-106 and builds the line on blocks as
the locked path's carrier (Q9 (a), R-899); **(3)** as above, with TASK-M8-102, TASK-M8-103, TASK-M8-104, TASK-M8-112,
TASK-M8-118, TASK-M8-122, TASK-M8-123, TASK-M8-126 (one task, its across-a-morph test on stage 2's line) and TASK-M7-72
added, TASK-M4-32 before them; the M4 and M7 tasks of stage 3 are released by dispatch, the milestones kept (A) (b);
**(4)** TASK-M8-106, now its re-timing, alignment and export only, its stage-1 tests (REQ-CHART-113) moved to
TASK-M8-124, then TASK-M8-107 and TASK-M8-108; **(5)** as above. Three tasks join, each behind TASK-M4-37 but the first:

| Task | Milestone | What | Waits |
|---|---|---|---|
| TASK-M8-127 | M8 | the acute-angle Burrau charts over the full acute range through the filter (R-875) | TASK-M8-120 |
| TASK-M8-128 | M8 | final-condition charts (R-879, § 10.3) | TASK-M8-102, the M8 charts |
| TASK-M8-129 | M8 | the consolidation step: one leaf per operation, the GUI routes as adapters, D19 and D23 fixed (R-894) | TASK-M8-124, TASK-M8-125 |

TASK-M8-127 is not given TASK-M4-37 directly: it rebuilds three gallery charts and waits on TASK-M8-120 alone of the
design's tasks, so it is behind the gate through TASK-M8-120. *Was:* "TASK-M8-127 is outside the design's gate: it
rebuilds three gallery charts, and waits on TASK-M8-120 alone of the design's tasks."

---

## 16. What this does not settle

- § 11 measured Metal on one Mac and Vulkan on one RTX 2080 SUPER, which may be faulty and whose result is a measurement only; DX12 and WebGPU aren't planned. Whether C3's 2 s applies on Vulkan, where the gallery alone takes 2.7 s, isn't settled: RQ-716.
- `D₁₁` against RQ-650, still open, and the other gaps of § 20.
- Whether a composed chart's linearised deep-zoom decode needs its own derivative: `J_D` on a baked pair follows the shared
  path at M6; a CPU batch view has no deep zoom until a task gives it one.
- § 3.1: Hill's threshold owes its attribution; `D₁₂` rests on one figure, and the sense of TS-2's rotation isn't printed (§ 20). § 3.1's identities aren't round trips: `prim_round_trip`, with its mutation control, is owed by TASK-M2-74, and a closed row doesn't ship without it (§ 12).
- Chart paths: § 17's whole-frame time, integration included, a built `stage_blend`, and any nonlinear-end alignment:
  not measured. A faster CPU-only live path is a goal with no task. The linearised decoder's tangent planes wait on M6.
- RQ-701, RQ-700 and RQ-682 stand open in the queue; this document assumes nothing of them but where it says so.

---

## 17. Chart paths

The owner's consolidated order of 6 Nov: "a view is a point in chart space; every edit is a path." This section takes it
clause by clause, each checked against the code, the corpus and the rulings branches of 4 and 5 Nov, which change nothing
here. Every numeric claim comes from `docs/incoming/2026-11-06_chart_composition/checks/morph_check.py` and `path_check.py`, on a
Python replica of chart_reference §0, unless it says otherwise. All of it is **door** RQ-721.

**R-864 governs this section's types (the owner's architecture rule of 8 Nov 2026).** One object: a view is a point in
chart space and every edit a path through it, its core types (Chart, Path and its families, Lock and anchor, Axis and
primitive, PairMap, Hold) designed as one module, and a type sketch with a one-page rationale, reviewed by physics and
code against `principia_chart_space_model.md`, brought to the owner before any implementation (TASK-M2-61). Where this
section has an arm call the existing code (§ 17.3), R-864's item 2 reads: the operation becomes a constructor of the one
object, a path of a named family, and its old route is deleted once it runs through it. Bitwise reproduction comes first
(R-863's stage 1), and a special case is a named family or a gap brought back, never an if-branch.

**Where the order can't hold as stated:**

1. **Morph is not the general form of tilt and zoom, as one formula.** No single blend reproduces them bitwise (§ 17.3). A
   *path type* does, with tilt and zoom as arms running their own code. Under the owner's rule the one-formula
   generalisation is **not adopted**; the path type is the proposal, and whether that satisfies the rule is the owner's.
2. **"Pixel → latent `z`" can't be literal.** The shape sphere's lower half has no `z`, and neither has a momentum past
   `q_max` (§ 17.2). The blend runs on the 8D manifold `z` parametrises.
3. **"The anchor IC bitwise fixed" can't come from inverse-encode** on a nonlinear end. It holds from stored words (§ 17.8).
4. **"Every intermediate frame is a valid chart"** holds as reproducible and decodable, not as contract Part 5's
   well-posedness (§ 17.5).
5. **"Live" conflicts with four written rules**, and a changed link id stays a cut under R-520 (§ 17.12). Its frame-time
   target, written first, is missed on the stand-in (about 15 ms against 4), and stands.

### 17.1 The framing

It fits the contract, which already has the classes: "Anchor-preserving (basis edits): tilt, zoom, chart-mode switch" and
"Anchor-excursion (centre edits): slice, and slider moves in locked mode". The order adds morph to the first, and it
belongs there under a lock. Three things the framing leaves out: **pan** is a centre edit in the operation table, and it
re-addresses (R-92), so it is no excursion; the classes describe **locked** behaviour, and free of a lock a basis edit has
no anchor but `z₀`; and a **lock** is no path at all, since it sets the anchor and changes neither key (R-92).

Sequence keyframes are waypoints and easing its timing: export contract Part 3 already reads so, with one `s` per frame.

### 17.2 The mechanism

**The rule: blend at the last layer where the two ends differ, and run everything downstream once, on the blend.** It is
the one statement that covers the order's cases:

| The ends differ in | Arm | Blended quantity | Downstream, unchanged |
|---|---|---|---|
| tilt angles | `Path::Tilt` | `τ(s)`, its own map (R-474) | `Tilted<C, P>`, then the chart |
| zoom | `Path::Zoom` | `exp(ln a + s(ln b − ln a))` | the view; no sim-key change (R-92) |
| the plane of an affine chart | `Path::PlaneRotation` | the basis, on the geodesic (§ 17.7) | the latent decode |
| one axis | `Path::AxisBlend` | that axis's value or control | the other axis's own construction, then holds |
| the pair map | `Path::PairMap` | `(a, b)` | both axes, then holds |
| the chart | `Path::Line` | the state's blocks, below | the decode, then any common hold |
| several independent view fields | `Path::PerField` | each field by Part 3's rule | as today |

**Why not literal `z`.** Tested:

| Question | Result |
|---|---|
| Is the blended `z` on the shape sphere, `I = 1`? | Yes: `D`'s image is the sphere at `I = 1` for every `z`; worst 8.9·10⁻¹⁶ over 20,000 blends of `z ∈ [−6, 6]⁸`. **No reprojection is needed.** A blend of `n̂` leaves it (worst `‖n‖ − 1` of 0.99) and a blend of positions leaves `I = 1` (worst 0.998), so neither is an option |
| Does `z` reach every end? | **No.** `β = π·σ(z_β)` puts every `z` on `n₂ ≥ 0` (held at 5,000 `z`). The shape sphere's lower half is its own systems (R-544), and encode mirrors it (`D(E(x)) = C(x)`). Leaving such an end by `s = 10⁻⁶` jumps by 1.75 median, 3.3 worst |
| Does `z` reach a held momentum? | **No,** past `q_max`: `q_k = q_max(2σ − 1)` is bounded. Under hold `E` at equal masses: 0.2% of feasible sphere pixels at `E* = −0.5`, near collisions; 0.005% at `E* = 0`. Clamped, with `lookup_clamped` |
| Is the blend's speed even? | **No,** near a saturated control. From a collinear end, 10% of the angle takes `s = 0.79`: speed spread 247 to 1 |

So `Path::Line` blends the 8D manifold of contract Part 1, `2 config ⊕ 4 momentum ⊕ 2 mass`, block by block, on each end's
own signed state (`canon::canonicalise_signed`):

```
mass      μ_k = ln(m_k/m₀)   a + s(b − a)       link-free; every ratio reachable
shape     n̂     the great circle from n̂_A to n̂_B   stays on the sphere; signed; no reprojection
momentum  q     a + s(b − a)                    unbounded; body frame, as held_momentum
```

`z_μ` itself has the fault above: `μ = 5·tanh z_μ` reaches no ratio past `e⁵`, and it is link-specific (physics note 2, P4).
Then `place`, `to_particles` and the signed `C`: the shape sphere's own decode, so `I = 1` by construction and no new
physics. Leaving a lower-hemisphere end by `s = 10⁻⁶` now moves by 2.3·10⁻⁶. Inside the hypercube and upper half this is a
reparametrisation of the literal line, not the same line. The order said `z`, and the owner has withdrawn "blend in Z only"
(RQ-736): the model's § 0.1 agrees with this section, a chart mapping into the signed state space and a path blending each
block in its own geometry. RQ-721 still applies it.

*[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]* **The mirror half is never dropped or folded (R-544; the owner's rider to RQ-736).** That holds in any chart and any morph, and
it's why the blend is signed: a morph through shape-sphere charts keeps both halves. A path between two lower-half views stays
in the lower half, since the great circle between two points of an open hemisphere stays in it (`path_lower_half_stays`, § 17.14).

**The mirror fold is a start's, never a trajectory's (R-544; the owner's second rider to RQ-736).** It applies only to how the
latent chart encodes a starting state at `t = 0`, and to the initial-condition constructor's teaching display. It never applies
to trajectories, dynamics or the shape-sphere charts: every shape-sphere chart shows the full `θ` 0° to 360° and `φ` 0° to 180°
range of starts, and trajectories cross the equator freely (`trajectory_polar_unfolded`, § 12).

**What the blend means.** A morph is of *addresses*, not of systems: at one pixel, A's system and B's are unrelated. So a
mid frame is a continuous picture of a family of ICs, and its structure is not a landscape of either chart. Where fibres
differ nothing special happens (a seeded axis on a blended configuration is the exception, § 17.11): each end has one state at a pixel, by its own construction (§2.2's seed included), and the
fibre is not blended. Where **branches** differ, a numerical end's state jumps across its seam, so the frame has that seam
for every `s > 0`, with a jump of `s` times the end's, at both ends' seams at once, each drawn. **The great circle has its own
seam:** at `n̂_B = −n̂_A` it isn't unique. That is two equations on two pixel coordinates, so generically isolated pixels, a
curve only for a symmetric pair of charts. Around such a pixel a ring of radius 0.01 spans the whole great circle at `s = ½`,
a vortex (physics note 2, P6), so the area factor there is large and V15's flag gains an upper arm. Pure function of the
pixel, R-82's pattern for the tie, a declared seam. A pair of charts whose bodies are labelled differently is blended
by index: relabelling stays distinct, so the path is not a physical one.

### 17.3 The constraint: bitwise through the mechanism

The order: tilt, zoom, lock and Save-view-as-chart "reproduce bitwise through the general mechanism, a test per operation, or
the generalisation isn't adopted." Tested through two readings of "the general mechanism":

| Operation | One formula: blended `z`, or a general geodesic | The path type (§ 17.2) | Test |
|---|---|---|---|
| **Tilt** | **Fails.** On a curved chart a tilt is `(a, b, d) = R·(u, v, 0)` on the chart's own parameters, so the blend misses it: 0.016 median, 0.124 worst in `n̂`, over 3,000 sphere pixels. On an affine chart the general geodesic reaches Tip-as-sheet's and two-target tilts' plane to 10⁻¹⁵ but not their bits: 20 of 64 projector entries equal. **Lean each axis isn't a geodesic at all,** 0.02 median, and it isn't orthonormal | **Passes,** by being the same code: `τ(s)` goes through `Tilted<C, P>`, so a frame is the view at that `τ`, bit for bit | `path_tilt_bitwise` |
| **Zoom** | **Fails.** A blend of `q` isn't log-stepped, and a frame carrying `(A, B, s)` would change the sim key where R-92 says a zoom only re-addresses | **Passes:** the arm writes the view's own field, so the sim key, the quads and the image are the direct zoom's | `path_zoom_bitwise` |
| **Lock** | Nothing to blend: a lock is no path. Equal ends do pass, except `−0`: `−0 + s·0` is `+0` | **Passes,** by not running: equal ends take the end rule (§ 17.4); `lock::kernel_chart` is unchanged | `path_lock_bitwise` |
| **Save view as chart** | Same: nothing differs | **Passes** for an existing view: a saved chart reopens as itself, R-513 and R-514's composition untouched. **A saved mid frame is new** (§ 17.5) | `path_save_view_bitwise` |

**The model agrees.** Affine tilt is R-474's own family, tested against its own formula (the model's § 0.2; the contract's "Two
tilt maps"; `path_tilt_bitwise`), and a slice is a translation in the chart's hidden coordinates (its § 0.3). RQ-736 records the
owner's ruling, and one open line on how § 0.2 names the family.

**The verdict, without softening.** Morph as the general formula of which tilt and zoom are special cases is **not adopted**:
its test fails on tilt and on zoom. What passes is narrower: a path *type* whose arms call the existing code, with morph one
arm among them. That keeps the framing, the record and the timing, and it doesn't unify the mathematics. Whether it meets the
order's constraint is RQ-721's first question.

**Each operation is tested against its own family (the model's § 0).** Tilt against R-474's formula, zoom against the view's own, a morph against the block line. A tilt isn't compared with the morph within a tolerance: the 0.016 above is why they're different families, not a margin.

**What the four tests do and don't show (physics note 2).** A bit identity with existing code needs the same floating-point
operations in the same order, so no unified formula could pass except by dispatching to that code. The path type meets the
constraint's letter and not "morph is the general form": two operations pass by not running and two by calling the old
code, so these tests can fail only if dispatch fails, and say nothing about morph. The real test of a special case is a
limit, and it fails: `Line(A, B_δ)` doesn't tend to the tilt as `δ → 0`, staying 0.016 median off in `n̂`, so a tiny extra
change to a tilt keyframe switches arms and the mid frames jump. What is adopted, if it is, is one record, one timing and one
rule, passing by construction, not a generalisation.

**The arm is a pure function of `(A, B)`,** tilt and zoom first (P9): a tilt-only segment takes `Path::Tilt` on any chart, and
the four tests start from two specs and pass through that function. **Independent layers differing together** (P10): Part 3
eases `z₀`, zoom, basis and slice together today and R-484 holds them, so a sixth arm, `Path::PerField`, eases each field by
Part 3's own rule, with a fifth bitwise test, `path_keyframe_bitwise`. Part 3 says basis by geodesic where `Path::Tilt` eases
`τ`; § 17.3 shows those differ on Lean: in § 17.12's conflict table.

### 17.4 `s = 0` and `s = 1`

**Ends never run the blend.** `s` is compared exactly: `0.0` takes A's own path and `1.0` B's, the keyframe rule of Part 3
("Each keyframe's own frame takes that keyframe's `SimConfig` and `RenderState` bit for bit"). It can't be left to arithmetic:
`a + (b − a) ≠ b` on 23% of random f64 pairs, and any reprojection, alignment or retiming in the path is rounding on top.
`u` is exact at its ends and REQ-TOOL-197's curves have `s(0) = 0`, `s(1) = 1`; a retiming table is pinned at both (§ 17.9).
So alignment can't be applied to B's own frame: it fades to the identity at `s = 1` (§ 17.10). The price is a step between
`s = 1 − ε` and `s = 1`, of rounding size with § 17.7's `atan2` (with `arccos` at near planes, about 7·10⁻⁸ in the basis), inside the declared tolerance, not a seam.

### 17.5 A frame: valid, reproducible, recorded

**Recorded, by value:** `(A, B, path, s)`, with `A` and `B` the whole chart and view, never a name; `s` as f64 bits, in the
canonical serialisation (R-309); the retiming table's hash, the alignment and, under a lock, the anchor's words. Round trip
per R-484. Absent, the record's bytes are today's. For a `Path::Tilt` or `Path::Zoom` frame, the triple stays in the spec
only: the frame **is** a plain view and its `SimConfig` is that view's. Only `Line`, `AxisBlend`, `PairMap` and a general
`PlaneRotation` put a morph in the sim key, a new `ChartSpec` arm, so an old file's key doesn't move.

**"Valid" holds in this sense:** every pixel decodes to a system or is tagged, and one record gives one image at one
lowering.

**A frame is a chart: what it does, item by item** *(11 Nov 2026; R-944, the owner's principle of 10 Nov 2026: "Every
pixel of a morph frame is a valid system, so a morph frame is a chart and every feature that works on a chart works on a
morph frame")*. Each limitation this section stated is closed below, naming its mechanism, or kept as the one accepted
exception. What the section said before is kept as *Was*, after the list.

1. **Area statistics: kept, the one accepted exception** (R-936, RQ-760 (iii-a); R-944). A frame's `system_image` is
   unknown: a blend may fold (§ 17.6), and it can show the same systems twice without a fold. So `measure` is `none` and
   no statistic is admitted (`admit_statistic` refuses). The two end charts keep their own statistics.
2. **Well-posedness: a fact, and nothing is refused by it** (wording, applied per R-369, listed for veto). Contract Part
   5's "all 8 DOF are determined exactly once" is a check on swept axes and blocks, and a frame has neither; a blend
   touches every block. Part 5's check runs on A and B. A blend is well-posed when both ends are and its path meets
   R-943's three conditions: both ends are exactly the end charts, every in-between frame is a valid system, and it is
   continuous. That is the blend's Part 5.
3. **Axes: closed** (R-940, R-942). A frame carries a dual ruler, one axis line with the starting chart's real ticks on
   one side and the target chart's on the other, each fading with its share; hover gives every true value. A `gauge`
   end's paper scale: § 17.11.
4. **Contours: closed** (R-952; `principia_lines.md` § 4.1, § 4.2). A contour of a primitive reads the decoded state (§
   10.2), evaluated on the CPU in f64 as on every chart. A Chart or Latent variable has one value per end chart and is
   drawn once per end at that end's share, so the grid is both ends' grids, each at its own ruler's ticks (applied per
   R-369, listed for veto).
5. **Traces: closed** (R-947, R-948; `principia_lines.md` § 8). Each sample is placed by § 10.6's nearest-point route;
   the first vertex of a hover trace is the hovered pixel. Whether a trace breaks or jumps on a blend is measured, P2
   and P4, reported and not gates (R-951).
6. **Marks and the symmetry filter: closed** (R-947; § 10.4, § 10.5). Marks are found by § 10.6's placement, as on any
   chart with no inverse. That they meet the lock's tolerance mid-morph, and that the filter is on, is applied per
   R-369, listed for veto, and conditional on physics review.
7. **Tilt and slice on a live path** (R-902, 8 Nov 2026): a tilt or slice made mid-morph goes to both endpoint charts of
   the morph, each in its own hidden coordinates and R-474 map, about the pinned state, and the morph continues from its
   parameter between the tilted ends. No hidden coordinate is invented for the blend.
8. **Saving a mid frame: closed** (R-944, amending R-902's "§17.5 stands as written"). Saving a mid frame gives a frame
   record. Reopened, it is a blend chart like a live one, at its saved `s`, and every chart feature works on it, area
   statistics excepted. A tilt or a slice of a saved frame goes to both end charts, each in its own hidden coordinates
   and its own R-474 map, as item 7 has it for a live morph: the record holds both ends whole, so nothing is missing.
   Under a lock the edit is about the pinned state; without a lock it is about each end's own centre, the pixel at the
   window's centre, as a tilt of that chart alone turns (the unlocked pivot: applied per R-369, the owner's "is fine
   too", listed for veto). An end that can't take the tilt (no free hidden coordinate in that direction) holds, and
   under a lock the anchor slides (R-849). The tilted frame saves again as a record of the tilted ends at the same `s`.
   A saved frame may also be morphed on: a path from it is a blend whose end is a blend. "Reopened as CPU batch" is
   moot: every chart is evaluated on the CPU (R-926 item 5).
9. **Repeats** (a statement this section lacked; applied per R-369, listed for veto, a mechanical consequence: a blend's
   pixel shows the same system again only where both ends do). A blend repeats where both ends repeat with the same
   lattice in pixel space; elsewhere it is affine past its square, and its traces are drawn in the images it has
   (`principia_lines.md` § 8, "Repeats").
10. **Never scored: kept** (R-669, below). Not a feature a frame lacks: a replication score is an area statistic on a
    paper's chart, so it falls under item 1, and no paper's chart is a blend.

*Was (before 11 Nov 2026), the rest of the paragraph "Valid" opens:* "**It does not hold as contract Part 5's
well-posedness**: "all 8 DOF are determined exactly once" is a check on swept axes and blocks, and a frame has neither;
a blend touches every block. Part 5's check runs on A and B. A frame's `system_image` is unknown, since a blend may fold
(§ 17.6), so `measure` is `none`, no statistic is admitted (`admit_statistic` refuses), and axes carry no ticks, only
"A, B, `s`". Contours of a primitive still work: they read the decoded state (§ 10.2). **Saving a mid frame** gives a
frame record, reopened as CPU batch, with no tilt or slice: its chart has no hidden coordinate to turn. That is reduced
capability, a label." And the note under it: "*Read with R-902 (8 Nov 2026):* a tilt or slice made **mid-morph**, on a
live path rather than a saved frame, goes to both endpoint charts of the morph, each in its own hidden coordinates and
R-474 map, about the pinned state, and the morph continues from its parameter between the tilted ends. No hidden
coordinate is invented for the blend, so the sentence above stands as written."

*10 Nov 2026 (R-944, the owner's principle; R-936; R-940, R-942):* "Every pixel of a morph frame is a valid system, so a
morph frame is a chart", and every feature that works on a chart works on a morph frame. So this section's limitations
are gaps to close, not labels to keep: "axes carry no ticks, only "A, B, `s`"" is withdrawn, a morph frame carrying a
dual ruler, each end chart's own ticks fading with its share (R-940, R-942); and "Saving a mid frame … with no tilt or
slice" and its "reduced capability, a label" no longer stand as the answer, a saved morph frame tilting and slicing as a
live one does. One limitation is kept, the owner's one accepted exception: no area statistic on a mid-morph frame,
`measure` staying `none`, the two end charts keeping their own (R-936, RQ-760 (iii-a)). This note does not rewrite the
section: the chart re-plan's addition does, item by item, with the tasks affected. *11 Nov 2026:* done, in the list
above.

**R-669.** A frame is never a target's chart and never scored (replication rule 6, R-649). A morph doesn't make a refused
chart reachable as a chart: each end has to be built, and `excluded_primitives` still holds. It can pass close to a refused
plane, with the same answer as RQ-718: it is a user's picture, and the row stays "not scored".

### 17.6 Per-axis and pair-map morphs

They compose by § 17.2's rule, and that rule is what makes one axis stay fixed. If `V` is a latent axis its control is
untouched. If `V` is a constructed axis, `E` or `K`, it runs on the blended configuration, so its value is kept wherever
feasible. A chart-level blend wouldn't: between two pixels at `E* = −0.1`, the blended state misses by 0.054 median and 2.1
worst, against 1.4·10⁻¹⁶ with the step run after the blend.

**A linear blend of `(a, b)` folds; the bend doesn't.** Flat to polar through the plane, centre `(½, ½)`, radius `v/2`: folded
pixels 32% at `s = ½`, 13% with `θ` reversed; `θ = 2πu, r = v` about the origin gives 84% and 0%. Those are the fixture's and
the linear blend's, not topology's (physics note 2, P5). **The arm is a bend:** a strip curls to a ring of opening angle `2πs`
about a centre coming in from infinity: the Jacobian keeps one sign at every `s` tested up to 0.999, one-to-one for `s < 1`,
least `|det|` 0.50 at `s = ½`. `Path::PairMap` takes it; V12 is retired. Frames that do fold are flagged (§ 17.11), and the
order's "valid chart" is the sense of § 17.5.

### 17.7 Plane to plane

A linear blend squashes: planes swapped give area 0 at `s = ½`; orthogonal planes 0.5; random planes 0.513 median, 0.149 worst.
Between random planes the geodesic keeps the frame orthonormal to 1.3·10⁻¹⁵ and reaches B's plane to 2.2·10⁻¹⁵. Each principal angle is `atan2(‖w‖, A·B)`, with `w = B − A(A·B)` on each principal pair, not `arccos` of its cosine, and that keeps those figures at every separation: between planes `δ` apart, about 2·10⁻¹⁵ on both down to `δ = 10⁻¹³`, reach 1.4·10⁻¹⁴ at `10⁻¹⁴` (`morph_check.py`, 2,000 pairs at each `δ`). `arccos` is why: it loses half its digits near 1, reaching B's plane only to 1.9·10⁻⁹ at `δ = 10⁻⁶`, about 10⁻¹⁵/`δ`, and to 7.3·10⁻⁸ at worst below `δ = 10⁻⁷`, with orthonormality down to 1.4·10⁻⁹ at `δ = 10⁻¹⁴`. Export contract Part 3 and
REQ-TOOL-075 already rule it; nothing builds it yet. Three things the existing words leave open:

- **In-plane rotation.** The geodesic reaches B's plane, not B's `(q₁, q₂)`. R-92 counts in-plane orientation. The remaining
  rotation is eased on the same `s`.
- **Handedness.** If B's frame has the opposite orientation in the plane, no rotation *within the plane* reaches it. The
  path takes the second principal angle's far side, `π − θ₂`, a rotation; at `θ₂ = 0` its axis is free, a tie the rule below
  settles by `q₂`'s own sign (P8).
- **Ties.** Orthogonal planes, all angles 90°, have no unique geodesic, and that's the common case, `(z_α, z_β)` to
  `(z_μ₁, z_μ₂)`. The rule: `q₁ → q₁′`, `q₂ → q₂′`. The choice is per segment, not per pixel, so it's no seam.

Zoom is log-lerped and `z₀` eased, as Part 3. This arm is general `PlaneRotation`, for ends that aren't tilts of one another.

### 17.8 Lock: a based homotopy

**The model.** `docs/design/principia_chart_space_model.md` is the statement of how navigation and chart paths work. Its § 0
agrees with this section: it describes, and doesn't override a measured or ruled section (RQ-736, the owner's ruling of 7 Nov).
Its sliding anchor (Theorem 8, "the RQ-721 rider") is RQ-721's, not applied here: until then `anchor not in B` stands, below. *[mirror-step removal, ruled 8 Nov 2026, conditional (R-865): § 6.6]*

**What the code holds.** `lock::centre` is `z(½,½) = z₀`, and "the kernel's chart is `(z₀, q₁, q₂)` and nothing of the lock".
On a nonlinear chart the rebuilt chart's centre decodes the clicked state bit for bit through `Φ`, while `z_locked`'s own
decode is only within `ε_phys`, or at a clamp's image. So today's bitwise guarantee is on `z₀`, and REQ-GUI-005's anchor test
is a test on it.

**Does "the anchor IC bitwise fixed" hold?**

| Ends | Verdict |
|---|---|
| Both affine | **Holds, by construction.** The path moves `q₁, q₂` only, and `z(½,½) = z₀` at every `s` |
| Any nonlinear end, through inverse-encode | **Doesn't.** `E(D(z)) = z` bitwise on 0 of 20,000 at f64, and `D(E(D(z)))` is off by up to 1.3·10⁻¹⁵, over `z ∈ [−4, 4]⁸`; lookup's tolerance is `ε_phys`, and kind 4 and invariant axes solve or project. A clamp moves it further. Iteration doesn't converge to bits |
| Across the f32/f64 boundary | **Doesn't, by recomputing.** A CPU batch rounds an f64 state once; the GPU computes at f32, and a component of the two agrees on 58% of samples. Stored words work at any lowering feeding the same precision (P8) |
| With stored words | **Holds.** The anchor is a state: at the path's start the current view's own lowering gives the centre's IC words, they are recorded, and a sample at chart coordinate exactly the centre takes them by a comparison |

So the design is **a based homotopy by stored anchor**. Each end is re-centred on the anchor by inverse-encode, once per end
and segment, since the answer doesn't change between frames unless the anchor does: the order's "per frame" is needed only
then. Each pixel's *offset* from its end's centre is taken by the log map at the anchor on `n̂`, additively on `q` and `μ`,
split into a first-order part, `J_end(c − ½)`, and a remainder. The first-order part is transported along § 17.7's geodesic
(tangent planes rotate), and only the remainder is blended; the exponential map returns to the sphere. So `H_s(c) = anchor`
to rounding and the frame's tangent at the anchor keeps its area, where a linear blend of offsets gives
`(1−s)J_A + sJ_B`, area 0 at `s = ½` for swapped planes and 0.5 for orthogonal ones, the owner's "never squashing" failing at the
one point a lock looks at (physics note 2, P1). Offset alone isn't bitwise (`−0 + 0 = +0`, and the decode's rounding), and the
select is what gives the bits.

**The anchor may not be in B (P2).** A region D preset, a rest chart or a Burrau chart holds no anchor with other masses or
momenta, and encode may clamp or reject a qualitatively different start (inverse_encode_contract). Then the stored word would
put at B's centre a state B can't produce. **The outcome:** label `anchor not in B`, the morph runs unbased, with no bitwise
claim and `path_lock_centre` not applying. **The end rule and the anchor rule meet at one pixel:** `s = 1` must be B, and B's own centre differs from the words
by `ε_phys`. They agree only if a locked B, as a view, carries the words too: a chart-mode switch under a lock then records
them. That's a change to the lock's state (`SimConfig`'s lock group gains an IC, outside the sim key, or R-92 needs amending, since
a lock "changes neither key"; P8) and it adds nothing to the GPU's chart. The
test reads a probe at `(½, ½)`, since an even grid has no pixel there.

**The owner's rulings of 8 Nov 2026 on the lock.** R-867: the anchor is stored as the signed starting state, its mirror
half kept, not as a latent point; the words above are that state's, and what `z_locked` still serves beside it (the
sliders' re-basing, snap-back, a chart-mode switch) is a definition TASK-M2-65 writes here under physics review (R-72).
R-866: locking doesn't move the view. The pin is the clicked pixel, in general off the centre, its initial in-plane
excursion that pixel's offset from the centre, and tilt and zoom turn about it; TASK-M2-62 builds it in the engine (R-916; *was* TASK-M8-125, which keeps the badge and the setting) in stage 1, before
this task. What stage 2's test reads once the pin is off the centre, R-863's "the centre IC is constant" and this
section's probe at `(½, ½)` against R-866's pin at the clicked pixel, is not settled by either ruling: RQ-749 asks it, and
nothing here changes until it is ruled.

**The owner's reply of 8 Nov 2026 on the lock (R-897, R-899, R-900, R-902).** RQ-749 (a): the test reads the recorded
state at the pin, not `(½, ½)`; on a sliding path the follower and the area factor are taken at the pin. The pin's screen
position is stored, the centre labelled "assumed" where none can be derived (Q4 (a), (a-i)); a curved chart's pivot
record lives in the view, stored as the pinned state (Q5 (a), untilt (a1)); the line on blocks is built here as the
locked path's carrier (Q9 (a)); a point's star is filled within the lock's tolerance, a calibration (Q7 (c)). The lock is
the path's: every locked path, blend segments and chart switches included, keeps the pinned state, sliding by R-849
where B can't hold it, a labelled jump until the slide exists (Q3 (a)). The gate test, the owner's: lock → morph A → B →
tilt → slice → morph back, the pinned state within the lock's tolerance throughout, the round trip returning the same
view. Stage 2's saved-file rewrite is handled as RQ-750: versioned, converted on load, a conversion test in place of the
bitwise check, golden hashes regenerated with a record of why (R-900).

**Default correspondence:** the anchor, pixel to pixel by offset. **With an excursion** (`δ ≠ 0`): the pin is the current
`z₀`, as "turns about the pin"; the words are that point's; `z_locked` and `δ` are not touched. Snap-back undoes the
slice only, clearing the hidden excursion and leaving the pin where it is on screen, and "centre on pin" clears the
in-plane part; together they are `z₀ ← z_locked`, bitwise by definition, centre on pin after snap-back assigning
`z₀ ← z_locked`; snap-back alone, a projection on an oblique basis, is exact to rounding, its rounding bound is derived
from the construction TASK-M2-62 defines (of order `ε_R·κ(B)·‖δ‖` for a solve on the basis `B = [q₁ q₂ | hidden]`, or a
few ulps if built as `z_locked` plus the in-plane part recomputed from the pin's stored chart parameters) and stated by
that task, its decoded state within REQ-ENC-024's `ε_phys` only where that bound and the decoder's conditioning allow; a
bound that can't be derived is a calibration (R-71); the test's fixture names its `κ(B)` and `‖δ‖`, its construction
TASK-M2-62's (R-72) (the brief's Q2 (a), R-899; R-916). *Was:* "`z_locked` and `δ` are
not touched, so snap-back, `z₀ ← z_locked`, stays bit for bit." **Deep zoom:** REQ-DEC-033's linearised decoder lands in M6. `J_D` at the anchor doesn't change along the
path, since the anchor doesn't; what changes is the chart's tangent plane, per frame. At depth a frame is locally a plane
morph, the same first-order part transported above, so a deep frame is the linearisation of the coarse frame over it. CPU work per
frame, f64, not built, owed to the M6 task.

### 17.9 Path shape and timing

**Easing** is R-576's, untouched: `s = ease_k(u)`. **Path shape is separate**, a field of the segment beside easing and
transition: `segments: [(easing, transition, path)]`. Waypoint charts are keyframes, so a chain of segments is already that.
Control charts bend a segment: de Casteljau's construction with each lerp replaced by the arm's own interpolation, so
planes are built from rotations, shapes from great circles. It passes through neither control, and needs both ends'
pixel coverage: each control's blank region is blank on that path.

**The named default** appears in every export's planner row and record, a pure function of `(A, B)` in this order: `tilt`,
`zoom`, `per-field`, `plane rotation` for two affine ends, `pair-map bend`, `per-axis quantity blend` for one axis differing,
otherwise `block line`, § 17.2's. A tilt-only segment never takes a later arm (P9).

**Re-timing for constant visual speed.** A table `g` from arc length to path parameter, then `s_path = g(ease(u))`, so easing
governs timing and `g` only removes the saturation's distortion. Arc length is measured on a fixed 16 × 16 probe grid at 33
knots, as chord length of the decoded blocks, in f64 on the CPU: a pure function of `(A, B, path)`, its hash recorded, pinned
at `g(0) = 0`, `g(1) = 1`. On the worst case above: max over median speed falls from 247 to 1.12. **It conflicts with Part
3:** "No field takes `u` directly and none has a second curve". Only the blend takes `g`; every eased field keeps `s`. In a
chart-change segment those are the playhead and render uniforms, so the zoom-against-pan reason doesn't reach it, but it's
a second curve, and RQ-721 says so.

*Amended 8 Nov 2026 (R-901):* easing and the path curve are a property of each **path segment**, shared by live morphs,
saved paths and export, with the model's Theorem 4 (concatenation of eased segments) and Theorem 5 (timing changes no
frame's content) stated and tested; TASK-M2-61 builds the field with the path object, so stage 4 adds nothing beside
it. Where a Sequence segment holds one path segment, its easing is that segment's; where it holds several, each keeps
its own, concatenated (export contract Part 3, R-576's forward line).

### 17.10 Without a lock: alignment

An alignment `T` in the window's similarity group, rotation and offset, chosen to minimise `Σ‖Y_A(p) − Y_B(T p)‖²` over the
probe grid, Procrustes on the grid (V14), pure and recorded; or the user picks matching points: one fixes the offset, two a
similarity. Under a lock the anchor is the correspondence, so `T` turns about it, or is off. **`T` can't be applied to B
outright**, since `s = 1` has to be B: `B` is evaluated at `T_s p`, `T_s` along the group's geodesic from `T` at `s = 0` to the
identity at `s = 1`, so at `s = 0` it has no weight and at `s = 1` it's gone. Per pixel it's cheap and adds no seam.

### 17.11 Honesty

- **Seams and degenerate frames.** The Gram factor exists: `tilt.rs`'s per-pixel area weight, `√(|û₁|²|û₂|² − (û₁·û₂)²)`, with
  `ray-degenerate` at its zero (R-554, R-474 R7). The general form is `√det(JᵀJ)`, `J = ∂Y/∂(u, v)`, by differences on the
  probe grid per frame. **Flag:** a frame with any probe below `10⁻³` of its median or above `10³` of it (the vortex, § 17.2), values for V15, is
  `degenerate`, and a frame-to-frame jump in the fraction below is `seam`. `Y` needs a metric: it adds a unit `n̂`, gauge-scaled
  `q` and mass controls, so arc length and the ratio depend on weights. The mass-weighted metric on the `I = 1` state is the
  natural one, a definition requirement (R-72; REQ-CHART-124, P7). Unsigned: a fold reads as a zero crossing, which a coarse grid
  can step over, and a generic surface in 8D never drops rank, so the Gram flag sees folds only where a frame stays in a
  two-dimensional subspace, as a pair-map morph does. So it's a flag and not a proof. Charts of different topology, the sphere against a plane, may pass through such frames; no such frame is forced (P5).
- **Out of feasible.** Tagged, never dropped (contract, "tagged, never dropped"). **A blank can open mid-path**: at
  `E* = −0.3`, equal masses, 1,173 of 1,748 pixels feasible at both ends are infeasible at `s = ½`, since the blended shape
  has a higher `U`.
  *11 Nov 2026 (R-926 item 3, R-943; a record correction applied per R-369, listed for veto):* that count was a hold run
  on a blended configuration. The state blend runs no hold: masses blend in log-ratios, shape on the great circle,
  momenta and time linearly, and "every in-between frame is a valid system" (R-943), so the hold's blank does not open
  mid-path on the default path. *Was, before the physics review of PR #482 (review 5852979675, its confirmation 4 and
  finding P1):* "so no blank opens mid-path on the default path. One case is the physics reviewer's to confirm where the
  blend chart is built", with "isolated pixels of a frame" for the collision case. Two cases are left, and the second
  shows that the statement as first written was not complete:
  1. **A collision point: confirmed by physics review.** A great-circle path can pass through one of the shape sphere's
     three binary-collision points. The pixels there, a small disc of them, everything with two bodies within `r_coll`
     and not only isolated pixels, have finite `(m, r, p)`: under R-933, R-60 and R-13 that is a collision at `t = 0`,
     recorded with the pair and the least separation, never a blank and never `decode_failed`. The antipodal case
     `n̂_B = −n̂_A` is § 17.2's declared tie.
  2. **`M01_TINY` between two valid ends: the pixel saturates, never blank (R-975, RQ-787 (a)).** *12 Nov 2026 (R-975,
     the owner's answer on RQ-787, option (a) with physics review's smooth hold):* such a pixel holds the blended
     `m₁ : m₀` and lifts the light pair just to the allowed minimum, `M₀₁ = ε_M01`, the rest of the blend unchanged: a
     valid system, continuous in `s` and across pixels, equal to the plain blend where the dip begins and ends (R-943).
     How the lifted state sits against the guard, which fires strictly below `ε_M01`, is the building task's
     definition under physics review. *Was (the bullet's title):* "a blank can open mid-path, and what the pixel shows
     is open (RQ-787)"; the account below is kept as it was written. The
     masses blend linearly in `μ_k = ln(m_k/m₀)`. Then `M₀₁ = g/(1 + g)` with `g(s) = e^{−μ₂(s)} + e^{μ₁(s) − μ₂(s)}`,
     a sum of exponentials of linear functions: convex in `s`, so it can dip below both ends. The physics review's
     example: end A `μ = (−L, 0)` (`M₀₁ ≈ ½`), end B `μ = (L, L)` (`M₀₁ ≈ ½`); at `s = ½`, `μ = (0, L/2)` and
     `M₀₁ = 2/(2 + e^{L/2})`, below `10⁻⁵` for `L > 24.4`. Both ends are valid pixels of a chart that sets its masses
     directly (the edge-reaching simplex link); the midpoint raises `M01_TINY`, "on the emitted masses of every chart"
     (`principia_dd_decoder.md` § 2), and no edge kind saturates a blend's interior pixel. Latent-link ends cannot
     reach it (`|μ| ≤ 5` gives `g ≥ e^{−5}`). *11 Nov 2026 (the physics re-check of PR #482, review 5853427038,
     finding P9):* that example is right but is not the typical case, and no extreme mass is needed. With
     `r_k = m_k/m₂` the blend gives `r_k(s) = r_kA^(1−s)·r_kB^s`, and `g = r₀ + r₁`. Take two ends on or just outside
     the band's edge, `g = g*`, whose light pair has the ratios `q` and `1/q`: then `g(½) = g*·2√q/(1 + q)`, below `g*`
     for every `q ≠ 1`. In masses: end A ∝ `(1, 0.01, 10⁵)` and end B ∝ `(0.01, 1, 10⁵)` both have
     `M₀₁ = 1.01·10⁻⁵`; the midpoint ∝ `(0.1, 0.1, 10⁵)` has `M₀₁ = 2·10⁻⁶`. The review's brute-force search over ends
     with every mass at least `10⁻⁹` and `M₀₁ ≥ 10⁻⁵` finds midpoints down to `7·10⁻⁷`. It reaches saturated ends
     too: by condition (i) of the table under "Tagged at an end" below, a saturated end blends its saturated state,
     which sits on the band's edge, so wherever both ends saturate a pixel to states with different `m₁/m₀`, the
     interior dips for every `0 < s < 1`. The region can be the whole overlap of the two ends' saturated bands, not a
     few deep-zoom pixels, and its share of a frame is not known until measured. The blend chart's fixture therefore
     has three arms: the `L > 24.4` example, the pair above at the band with modest ratios, and one where both ends
     are saturated pixels with different `m₁/m₀`. What such a pixel shows is R-975's, above: it saturates, and the
     fixture's arms assert the hold. *Was:* "No ruling says what such a pixel shows: it is the owner's, RQ-787, and
     nothing is chosen here. Meanwhile the decoder's rule stands as written: the pixel is tagged `M01_TINY`, never
     dropped and never NaN, and the blend chart's fixture for this case (TASK-M2-90) asserts, on each arm, the tag
     and that both ends are valid, and nothing about what is drawn."
     *(11 Nov 2026: the arm with both ends saturated is checked by TASK-M2-103, the task that builds the saturated
     pixels it blends, under REQ-CHART-211; TASK-M2-90 checks the other two. What each arm asserts is unchanged.)*
     *(12 Nov 2026, R-975: each arm now asserts the hold, not the tag.)*
- **Blank or forbidden at an end.** A pixel in a morph frame is in only if it's in both ends. No state exists for a blank end
  to blend from, and substituting the other end invents a continuity (R-470, R-467). So the blank is held for every `0 < s <
  1` and resolves at the end frame, a step at each end. A region-D mask behaves so: outside D blank at every `s < 1`.
- **Tagged at an end (P3).** A pixel tagged at either end is treated as blank at that end, so V21 reaches it: blended, the hold
  would open undeclared seams. A rest end, tagged unscalable with momenta 0, at `s = 10⁻⁶` would take `q_B`'s direction and the
  full `K = E* − U`, a median jump of 0.29 at `E* = −0.1`; at the feasibility boundary a tagged pixel keeps its momenta where a
  just-feasible one has `η ≈ 0`, a jump of all of it, 0.19 of 0.19; and on a blended configuration R-82's seed for a constructed
  axis can flip, which § 17.2's "nothing special happens" doesn't cover: such a flip is a declared seam. `path_small_step` has
  a fixture for each. Rest is the default `z₀`, so this is the common case, and it stays blank to `s = 1`.
  *11 Nov 2026, on this bullet and the one above (applied per R-369, listed for the owner's veto, and conditional on
  physics review, which answered tag by tag on PR #482, review 5852979675, its confirmation 3):* under the state blend
  two of P3's three cases are gone, the third is not. Gone: the rest end, whose zero momenta blend as
  `(1 − s)·0 + s·p_B`, continuous, a valid system; and the seed's flip on a blended configuration. **Not gone:** across
  an end's feasibility boundary "a tagged pixel keeps its momenta where a just-feasible one has `η ≈ 0`", a jump in
  space that belongs to the end and survives mid-path at weight `(1 − s)`. It is a declared seam of the frame, drawn as
  an end's branch seam is (§ 17.2), with a `path_small_step` fixture. *Was, before that review:* "under the state blend
  P3's reason, a hold run on the blend, is gone". So a pixel is blank mid-path only where an end has no state at it, and
  a tagged end that carries a state blends as one. Which tags carry one, as the physics review gave it, over
  `principia_dd_decoder.md` § 2's closed set of five:

  | Tag at an end | Blends? | Why |
  |---|---|---|
  | `ENERGY_UNSCALABLE` | yes | the rest start, or an already-collided start: finite `(m, r, p)`, a real system (a collision at `t = 0` at the pole, R-933) |
  | `ENERGY_INFEASIBLE` | yes | the momenta as decoded, a real system that misses the target `E*`; mid-morph no held value is claimed (R-850) |
  | `INVARIANT_INFEASIBLE`, the arm `K* < L_z²/(2I)` | yes | the rigid rotation at the pixel's own `L_z`, the same state R-937's clamp draws |
  | `INVARIANT_INFEASIBLE`, the arm "`2K*` has no finite float" | **no** | its `L_z` is replaced by 0 by a select; R-937 keeps it "not a system" |
  | `NO_QUALIFYING_SEED` | **no** | no construction ran; unreachable under R-934's seed, reachable only under the legacy option |
  | `M01_TINY` | **no** | the Jacobi construction is not valid there; under R-932 the end saturates instead, and then the saturated state is what blends, untagged |

  Code 0 (a non-finite decode) has no state. Two conditions bind the "yes" rows, and the physics review holds
  TASK-M2-81 to both. (i) The state blended is the one the end chart itself draws at that pixel once R-932's kinds
  apply, never the tagged decode's raw output where the two differ; otherwise the frame at `s → 0` does not converge on
  the frame at `s = 0` (R-943's continuity). (ii) The feasibility-boundary seam above is declared and drawn, with its
  `path_small_step` fixture. For the three "no" rows V21 stands as written. *Was, the condition as first recorded:*
  "The condition: the physics reviewer confirms, tag by tag for the decoder's five tags, that the tag carries a state
  that may be blended, where TASK-M2-81 builds it. Until then V21 stands as written, and this is not recorded as
  settled." Under R-932 an end pixel with no state is left only where a chart's own validity fails (R-467). A blend's
  interior pixel that raises `M01_TINY` between two valid ends is another matter, the bullet "Out of feasible" above
  and RQ-787, ruled by R-975 (12 Nov 2026): it saturates.
- **Held quantities mid-morph.**

| Held | Mid-morph |
|---|---|
| The same `E*` (or `L*`) at both ends | the step runs on the blend, so it **is** held, within feasibility |
| Different targets, both on | the target is eased, like a slice value; held at the eased value. Under a lock this moves the held centre, a centre edit: an **excursion** by the contract's classes, unbased, with no bitwise claim (P2) |
| On at one end only | **not held**: a switch has no middle. The end's own state carries it; label `hold off mid-path` |
| Either end forbids it | not held; `E` drifts, label as above |
| Held by construction (`K`, `(ν, K)`) | held if that axis is the fixed one of a per-axis morph; else not |
| `I = 1` | always, by `place` and `C` |

  A `gauge` axis's paper scale has no meaning mid-path, so `t` is canonical time only, and region D's `t_paper` isn't drawn.
  *11 Nov 2026 (R-944: the table above is kept).* It says which systems a frame shows, not something a frame cannot do:
  every feature that reads a held quantity, a contour of `E` or the hover readout, works mid-morph and reads the true
  value. A path that holds a quantity is one of R-943's other paths, allowed and not built now.
  *11 Nov 2026, the gauge axis (R-940; applied per R-369, listed for veto):* with the dual ruler each side of an axis
  carries its own end's real ticks, so a `gauge` end's paper scale is drawn on that end's side, fading with its share,
  and region D's `t_paper` with it; canonical time stays the blend's own. *Was:* the sentence above.
- **One colour window per morph.** The window is `RenderState.frozen_ranges` (R-581 to R-583), frozen once for the segment,
  from `s = 0`. Measured windows would re-range per frame and manufacture motion.
- **Optional simultaneous playhead change.** Part 3's policies, unchanged: hold, advance, set. Cost per Part 2 (b).

### 17.12 Performance

**Live: on the GPU with no baked pair.** A frame doesn't need one. Baking `(A, B)` would be `N²` entries, each with two decoders
inlined, about 52 KB of WGSL, so the module test's 251 KB would be spent on five. Instead: **three dispatches**, A's existing
entry and B's writing `(m, r, p)` to their own buffers, then one generic entry, `stage_blend`, reading both, blending on
blocks, writing the IC buffer the integrator reads. The kernel stays chart-agnostic, `(m, r, p)` in and out, and needs one entry
in all, with `s` a uniform. **So mid-morph frames don't count against the cap**, but both ends do need a GPU entry, and an
unbaked end is CPU batch: that's "on the GPU when both ends are closed-form", narrowed to "and baked". `stage_blend` costs the
module about one pair, an estimate, so under § 11's C2 it uses the cap's fourth slot.

*11 Nov 2026 (R-926 items 5 and 6; a record correction applied per R-369, listed for veto):* every chart, every blend
included, is evaluated on the CPU in f64 and sent to the integrator as forward tables. So no blend needs a GPU entry,
`stage_blend` is moot, and no end is "CPU batch" as a lesser route: the label goes, here, in § 17.5 and under "CPU-only
ends" below. A frame's speed is the CPU evaluator's, measured by the chart re-plan
(`plan/charts/m2-chart-system-replan.md`).

It runs under the two-regime policy (caching contract): in motion the coarse cover, at rest refinement. A frame is a new
identity, as a tilt's is, its covers flotsam. It isn't re-addressable: pan and zoom inside a morph frame re-integrate, unlike
R-92's plane, except on `PlaneRotation`.

*11 Nov 2026 (R-926 item 3; R-935, RQ-770 (a); applied per R-369, listed for veto):* a blend is a chart, and a pan or
zoom on it is one pixel-space edit of both ends, moving the blend's window as it moves any chart's. So a pan or zoom
inside a morph frame re-addresses the quads already integrated on the same plane, as on any chart (R-92). It changes
cost, not results. *Was:* "It isn't re-addressable: pan and zoom inside a morph frame re-integrate, unlike R-92's plane,
except on `PlaneRotation`."

**The target, written before measuring** (`docs/incoming/2026-11-06_chart_composition/checks/frame-criterion.md`): one coarse cover's chart side
(1920×1080 at 1/8 linear, 32,400 samples) within 4 ms on this Mac, a quarter of a 60 Hz frame, so a morph leaves the
integrator what a tilt leaves it. The corpus has no number: `frame budget` is a setting.

**The measurement.** Real entries, `DecodeStage::run`, 32,400 samples, p50 of nine, on this Mac under load: one cover's decode
takes 2.4 to 4.2 ms typically, with outliers to 9.2; 0.2 to 0.4 of it is fixed overhead, and all of it includes the readback a
live frame wouldn't do. Five decode-sized steps is **about 15 ms. The target is missed, by a factor of about 3.4, on this stand-in.**
That number overstates by the readback, unmeasured, and it isn't a built entry. It is not loosened. In the in-motion regime a
miss means coarser or rarer covers ("coarse live glimpses updating every few frames"), not a failure to run. **What's owed:** a
dispatch-only time for a built `stage_blend`, the whole frame with integration, and the Windows arm: TASK-M8-107. The target
is V18's, the owner's to veto.

**The CPU side, for export.** Criterion in `docs/incoming/2026-11-06_chart_composition/checks/morph-criterion.md`: at 1920×1080, three decodes per sample
on five cores, at most 1 s. f64 `sample`, single thread, 83 to 87 ns per sample under load: 0.53 s on one thread, **0.11 s on five.
Passes.** A closed-form end's CPU side doesn't limit export; a frame's march does, and no frame's total is measured here. A
numerical end's solve wasn't measured.

**Conflicts with the written rules, all for RQ-721:**

| Rule | Words | The order |
|---|---|---|
| export contract Part 3 | "chart *switches* are cuts or dissolves (two renders composited), never parameter interpolation across chart types"; Part 2 (b) itself says "the chart morphs" | morph |
| REQ-TOOL-075 | "treat chart switches as cuts or dissolves, never parameter interpolation" | morph |
| principia_sequence § 3.4; REQ-GUI-379 | "live playback shows a dissolve as a cut at the second keyframe, as it shows every chart switch" | live first-class |
| R-520 F1; REQ-TOOL-175 | "A changed link id is a cut everywhere … there is nothing between two links to interpolate" | **morph** (R-949, the owner, 10 Nov 2026: "blend, don't cut"): a chart change that changes a link id morphs like any chart change, a blend of the two charts' states; R-520's rule for link parameters inside one chart stands. *Was:* "**stands**: the ruling wins. `Line` could cross it, its blocks being link-free with `μ` (P4), but a morph across a link id stays a cut unless ruled" |
| export contract Part 3 | "none has a second curve" | re-timing |
| export contract Part 3 | basis by geodesic, where `Path::Tilt` eases `τ`, and they differ on Lean (P10) | `Path::Tilt` keeps R-474's map; a Part 3 basis field keeps its geodesic in `Path::PerField` |

**CPU-only ends.** Export works from day one: each frame is CPU batch, `CPU batch: path`, f64 on the CPU, rounded once, seam 3's
exception (RQ-719) extended. Live is allowed and not yet fast, a re-render per frame, labelled. Faster CPU-only live is a
recorded goal, with no task: a numerical solve per pixel per frame can't reach the target. Caching each end's blocks per
pixel would help only while the view stays put, and export doesn't need it (V16).

**Export.** Part 2's class (b): each frame its own blocking march `0 → t_frame`. So a morph of `n` frames costs `n` renders
against a dissolve's two, plainly, and the planner says so. A frame is cached under its sim key like any other; no frame
reuses another's quads.

### 17.13 The Sequence

**Today.** "A segment whose chart or any link id changes shows a **cut mark** at its second keyframe", and a chart-change
segment offers "cut or dissolve". TASK-M8-77 (mock engine) and TASK-M8-31 (spec and planner) write them; neither is built in
`crates/`: no `Dissolve` or cut-mark code exists.

**A morph segment.** `transition` gains `morph { path, retime, align }`. On the timeline, a morph segment shows a band across
the Chart lane instead of the mark at its second keyframe, labelled with the path's name. The band lights Chart; the segment
is class (b); the frame count is unchanged. The planner's row gains "a morph: every frame a different decoder", the
artefact the flag beside its spec, R-520 F5's pattern, each frame's record holding its own `SimConfig`. Physics review (⚑)
where it's built.

**"Replaces" can't be total.** A file's segments reopen bitwise (R-484), so an existing `cut` stays a cut. A changed link id
stays a cut (R-520). And a morph's cost is `n` renders, so cut stays on offer, with dissolve. **Recommended default for a new
chart-change segment: morph**, a default change, so it's the owner's. Cut marks remain only on what's actually a cut. `Play
preview` then drives the morph live rather than cutting, and the banner's words change. Reel takeover (REQ-GUI-383) on a
morph frame loads that frame, then R-624's hold-and-cut.

*11 Nov 2026 (R-949):* a changed link id no longer forces a cut: a chart change that changes a link id morphs like any
chart change, with cut still on offer, and a cut mark only on what is a cut. *Was:* "A changed link id stays a cut
(R-520)."

REQ-GUI-374, REQ-GUI-376, REQ-GUI-379, REQ-GUI-381 and REQ-TOOL-075 change only after the ruling.

### 17.14 Tests

Each with a control (R-176). `path_lock_centre` is TASK-M2-65's, `blend_entry_counted` TASK-M8-107's, the rest TASK-M2-81's.
*9 Nov 2026 (R-919, F10, F11):* the owner's gate test, `path_lock_round_trip` and `path_mid_morph_tilt` (TASK-M2-65,
REQ-CHART-161), gates on R1: after the mid-morph tilt and slice, the return to `s = 0` equals the same tilt and slice applied
directly to A about the pin, bitwise, the same leaf with the same inputs (A's basis, `p0`, `x*`); a measured reason that
prevents bitwise goes to the owner, never loosened to the lock's tolerance. R2, recovering the start by undo, is tested
only where an undo feature exists. "Round trips across paths" read as M2's gate G4 has it: the ends bitwise, tilt, zoom, pan
and slice frames plain views, the record bitwise, and a blend frame, which has no inverse, read to pixel resolution
(R-899 Q8).

*11 Nov 2026 (R-944, R-947):* "a blend frame, which has no inverse, read to pixel resolution (R-899 Q8)" is a gap to
close: a blend frame is read through § 10.6's placement (the lock's tolerance as the marks' standard mid-morph is
applied per R-369 and conditional on physics review, § 10.5). `blend_entry_counted` is moot (§ 17.12; R-926 item 5).

| Test | Asserts | Control |
|---|---|---|
| `path_ends_bitwise` | `s = 0.0`, `1.0`: A's and B's IC words and image hash, on every arm, with alignment, re-timing and control charts on | an end run through the blend fails on 23% of a fixture |
| `path_tilt_bitwise` | a `Path::Tilt` frame's sim key and IC words equal the view at that `τ`, on Sheet, Lean, two targets, curved | the general geodesic in its place |
| `path_zoom_bitwise` | sim key, quad addresses and image equal the direct zoom | a triple in the key |
| `path_lock_bitwise` | a path with equal ends leaves every word, `−0` included; `kernel_chart` unchanged | blending equal ends |
| `path_save_view_bitwise` | a saved chart, tilted and untilted, reopens to one digest through a path | none blended: saved chart's `R` dropped |
| `path_lock_centre` | a probe at `(½, ½)` reads one set of words on every frame, ends included, affine and nonlinear, `δ = 0` and `δ ≠ 0`; snap-back bit for bit; the area factor at the centre stays within 0.5 of the ends' on swapped and orthogonal planes; `anchor not in B` labelled, unbased | the select removed; offsets blended linearly |
| `path_lower_half_stays` | two views wholly in the open lower hemisphere, whose ends differ at the chart layer so the arm function returns `Path::Line`, which the test asserts first: `n₂ < 0` at every pixel of every frame, ends included, on the signed state; on the other arms it asserts only what's proved, signed, no mirror taken; R-544, the owner's rider to RQ-736 | `canon::canonicalise` for the signed form, which mirrors; a literal `z` blend, which reaches `n₂ ≥ 0` only |
| `path_keyframe_bitwise` | a segment easing `z₀`, zoom, basis and slice together equals Part 3's frames bit for bit | one field through `Line` |
| `path_small_step` | gauge-invariant `‖ΔIC‖ ≤ L·ds` away from declared seams, `L` from the arm, with a declared band at `n = (−1, 0, 0)`, where the canonical frame turns (`\|dr/ds\|` 17 to 577, gauge; P8); fixtures: a rest end, the feasibility boundary, a seed flip, planes `δ = 10⁻¹⁰` apart (§ 17.7); a step at a seam is declared | a frame with an undeclared seam; `arccos` for `atan2` on the near-plane fixture |
| `path_record_round_trip` | `(A, B, path, s)` round-trips bitwise and re-renders one image at one lowering; absent, old bytes | `s` written at f32 |
| `path_gram_flags` | sphere to plane flags a degenerate frame; a linear pair-map blend flags folds; the bend doesn't; a vortex flags on the upper arm | the flag removed |
| `path_retime` | speed spread at most 1.25 on the saturated fixture; `g` pinned at both ends | the table off |
| `path_blank_ends` | a pixel blank or tagged at either end is blank for `0 < s < 1`, tagged, never dropped. *11 Nov 2026:* a tagged end that carries a state blends, continuous by `path_small_step`; a no-state end stays blank (applied per R-369, conditional on physics review, § 17.11; until it confirms, as written) | substituting the other end |
| `path_hold_table` | each row of § 17.11's table | the step before the blend |
| `path_links_cut`; *11 Nov 2026 (R-949):* `path_links_morph` | across a changed link id the ends are exact and `path_small_step` holds. *Was:* "a changed link id is a cut, live and export" | a frame that interpolates the link function itself. *Was:* "a morph across it" |
| `blend_entry_counted` | `stage_blend` is the only morph entry; the cap test unchanged. *11 Nov 2026:* moot, no blend has a GPU entry (R-926 item 5) | a baked `(A, B)` entry |

*8 Nov 2026 (R-897, R-873):* `path_lock_centre` reads the recorded state at the pin, not a probe at `(½, ½)`, the area
factor taken at the pin; and `path_tilt_bitwise`, `path_zoom_bitwise`, `path_lock_bitwise`, `path_save_view_bitwise`,
`path_keyframe_bitwise`, `path_ends_bitwise` and `path_record_round_trip` (REQ-CHART-113) are TASK-M2-61's, not
TASK-M2-81's.

---

## 18. Time as an axis (order part D)

**Status: Proposed, a door, RQ-727.** A chart may take time as one axis beside one IC axis: a pixel is `(IC, t)`. At most one time axis.
The live playhead stops; the horizon is the top of the axis.

### 18.1 Verdict: a chart mode, no fifth axis kind

The four kinds map an axis into `z` (contract Part 3; REQ-CHART-004, "a closed set"). Time maps nothing into `z`: it picks *when to read* a run
the IC axis already defines. So the pixel-to-`z` map is one-dimensional, `z(u)`, and `v` selects the row, `u` being the IC coordinate and `v` the time coordinate on either orientation: with Time on the horizontal axis (§ 19) the chart transposes the screen's coordinates (V42). That is a **mode of the chart**,
`ChartMode::TimeAxis { ic_axis }`, a new `ChartSpec` arm, with `T` the sim key's own field as the march's top, and the window `(t_lo, t_hi, warp, rows)` **a view over it, off the sim key**, and **it needs no fifth `AxisKind`**: RQ-712's agreement in
principle stands. The IC axis is any registered axis, of any kind. `validate` refuses two time axes by name, `TwoTimeAxes`. **The owner's
"offers time as an axis kind" (UI) and this verdict collide in words only:** the builder's kind selector lists **Time** beside the kinds
(§ 19), and the record is the mode. RQ-727 says so.

### 18.2 Against the core

| Point | Source | What the mode does |
|---|---|---|
| Integrate by column | core seam 3, "map → z → decode → (m, r, p) → integrate" | Holds: each column runs once from `t = 0`, decoding in the kernel on a baked axis. An IC axis on CPU batch, nearly every § 3.1 and § 3.2 row, takes RQ-719's IC buffer exception |
| Never re-integrate per row | the order | Holds. It isn't a discretisation question any more where the order thinks: R-565 made the sync grid `t = k·Δ_sync`, "never `t_max / n_sync`", so **`n_sync` no longer exists as a parameter** |
| Rows as boundaries, **per occupant** | integrator contract, "the march stops only on the grid"; "the playhead reads boundary states"; R-602 | A row reads the latest boundary at or before its time: unregularised, the macro-step grid; regularised, `Δ_sync`. There is no clamp to add and **a row can't land between boundaries**. Rows are kept as **integer boundary indices**, since `⌊(k·10⁻³)/10⁻³⌋ ≠ k` for 731 of the 65,535 `k` in f64. Ticks read the real times |
| The default occupant's resolution | R-560, Heggie × RK4; REQ-INT-080, `Δ_sync` uncalibrated, 0.1 to 0.0125 measured, 13/32 the old grid | **Time resolves only to `Δ_sync`.** At `H = 1080`, `T = 13`: 32 boundaries at 0.406, 34 rows each; 130 at 0.1, 8 rows each; 1040 at 0.0125, 1 row each. At `T = 50`: 123, 500 and 4000. So a chart shows bands, everywhere, not just near `t_lo`. **Time zoom floors at one boundary**, said in the UI ("one boundary: `Δ_sync`"): re-integrating lands on the same boundaries, and nothing finer exists short of a build constant. So this mode now bears on REQ-INT-080's value, whose cost is 2% at 0.1 and about 50% at 0.0125 |
| The horizon does still move one grid, unregularised only | integrator contract Part 3, `dt_macro = max(1e-3, T/65535)`; R-602 | **For an unregularised occupant, past `T = 65.535`, `dt_macro` changes with `T`.** So the order's "changing `t_max` compares different discretisations" holds there: raising `T` past that line is a new run from `t = 0`, labelled. A regularised occupant, the default, has no such line |
| Terminal states | Part 6, Part 7 | A terminal latches at its step, between boundaries or not; every later row holds that state, as the playhead past `t_end` does. Escape is sampled at boundaries |
| Warps | § 4 | Linear and log. Log needs `t_lo ≥` one boundary. Rows that floor to one boundary show one state: a stack of equal rows is true, not a fault |
| Jitter | sampling note; R-102 | `copy_index` offsets the IC coordinate `u` only, whichever screen axis it's on (§ 18.1). A copy is a column, so `(E + 1)` columns per pixel; never `t`. There is no time anti-aliasing, and fast binaries alias in `t`: labelled |
| Colour windows | R-581 to R-583 | `range_policy: frozen`, one window over all rows; `per_frame` refused for the mode |
| Payload purity | canonical spec, "`SimState = f(IC, sim key)`" | Becomes `f(IC, t, sim key)`, `t` from the sample's own address: still pure |
| "Never present mixed time" | temporal note | Holds per column: rows whose boundary is held are sharp, the rest "arriving" under the blur grammar. Export takes the hard barrier: every column to `t_hi`, every required boundary held |
| A stain reading the playhead | render contract; dd_colouring | The uniform becomes the row's `t`, per pixel, from the address: a colour-side change, one uniform to one expression |
| The word | payload § 3 | A reduced word isn't append-only: a later crossing can pop symbols ("free reduction: pop"), so the word at row `t` is not a prefix of the column's final word, and no length recovers it. So a stored row keeps its own word, 16 B, one cold-buffer entry per stored row, as an ordinary chart keeps one per pixel |
| Tier, quadtree | memory tiers § 4 | Rows and columns are render pixels. `E`, FTLE and `render_scale` mean what they did. The quadtree refines in `u` only: new columns, each row inheriting; `N` stays a power of two. REQ-PERF-086's rungs are unchanged in name and need a re-measure |
| Anchor | § 17.8, G4 | `(IC, t)`, with the words of § 17.8; `t` outside the sim key. Across a cut to an IC-only chart the lock keeps the IC and drops `t`: the playhead is restored, a different time, and the lock badge says so |
| Paper time | § 8.6 | Time is `gauge`. If the IC axis moves `ℓ`, a row isn't one paper time, up to 1.68× across D at equal masses (§ 8.6): the axis reads canonical time, labelled `paper scale differs` |

**Cache keys (physics note 3, C1).** The mode is on the sim key, through its new arm, so no old key moves. `T` is the march's top and stays on the key; the window is a view, off it,
`t_hi ≤ T`, so changing `t_lo`, `t_hi` or the warp is **navigation**, off the key: re-addressing where held and, past a column's front, a march with `T` unchanged, which moves no key (§ 18.3). **Raising `T` amends the caching contract:** its table says of `T`, "**All sim buffers** (signature
change; the march re-boots from `t = 0`)", so a continuation needs a carved exception. The physics is fine: below the line no step reads `T`, and only `bounded` at the top row belongs to it.
The exception holds for regularised occupants always and for unregularised ones while `dt_macro` is unchanged, up to R-610's cap, `T ≤ 200`. RQ-727 names it. A column's state at a row is independent of `rows`, so a cache of
columns is shareable across row sets, at a later version. The bitwise test: **`time_column_matches_playhead`** at the same `T`.

### 18.3 Stored rows and dense output: the conflict, plainly

The order asks for "stored per-column dense output" so zoom and pan in time re-read. **Core seam 3's rule isn't what it collides with:** that one forbids a
pre-decoded IC buffer, and a column decodes in the kernel. **It collides with seam 4 and the temporal note.** Seam 4: "the marching state at the playhead, O(1) in time".
The note: "**store no history**", and its footgun 4, "Viewport cache regrowing into a history buffer … the V2 siren. Guard with a ceiling", whose load-bearing
reason is that Principia became memory-bound only because of stored trajectories. R-66: "Scrubbing back re-integrates"; "never replays stored frames".

Budget at 1920 × 1080, `H = 1080` rows, 152 B per sample (memory tiers § 3; 104 without FTLE), per copy, so multiply by `E + 1`:

| Stored per column | Arithmetic | E = 0 | E = 3, High |
|---|---|---|---|
| The march's live state only | 1920 × 152 B | 292 KB | 1.17 MB |
| **Visible rows** | 1920 × 1080 × 152 B = `payload_mem` of the same tier | **315 MB** | **1.26 GB** |
| 2 × rows | twice that | 630 MB | 2.52 GB |
| 4 × rows | four times | 1.26 GB | 5.04 GB |
| Dense, `T = 13`, `dt = 10⁻³`, every macro-step | 13,000 × 1920 × 152 B | **3.79 GB** | **15.2 GB** |
| Dense, `T = 50`, the default | 50,000 × 1920 × 152 B | **14.6 GB** | **58.4 GB** |
| Dense, `T = 200`, the 65,535 steps | 65,535 × 1920 × 152 B | **19.1 GB** | **76.5 GB** |
| Dense, `r` and `p` only, `T = 13` | 13,000 × 1920 × 48 B | 1.20 GB | 4.79 GB, and no accumulator: no stain but position works |

The first two rows are the point: **the visible rows cost what an ordinary chart's payload costs**, one `SimState` per render pixel, with no breach of the tier model. **But it is
history in the note's words** (physics note 3, C4): "store no history — discard each step's state after rendering", and `H` past states of one trajectory is that, at one per
screen row. So **every option amends the note**, the recommended one bounded by `payload_mem`. `K > H` is footgun 4 almost word for word: "the 'cache a bit of history for smoothness' temptation";
at High, 1080p, a margin of `H` is 1.26 GB on a 1.44 GB tier total. Dense output on the macro-step grid exceeds High's whole payload by twelve times at `T = 13` and is the V2 design back.
**That dense output conflicts with the core, and it isn't what part D's rows-as-boundaries needs.** Reading a row at a boundary needs that boundary's state once, as the march crosses it.

**Store by distinct boundary** (C2). Rows that floor to one boundary share a slot, so the store per column is `min(H, B)`, `B` the boundaries in the window. Under the default occupant that is
small: at 152 B, 1920 columns, per copy, `T = 50`:

| `Δ_sync` | `B` | Every boundary stored |
|---|---|---|
| 0.406 | 123 | 36 MB |
| 0.1 | 500 | 146 MB |
| 0.0125 | 4000 | 1.17 GB |

Those are hot bytes. With symbolic features active each stored row also keeps its own word (§ 18.2), 16 B, so 168 B a row, as the payload's +Word column: visible rows are 348 MB per copy and 1.39 GB at E = 3, High, and the three boundary rows above 40 MB, 161 MB and 1.29 GB.

So for Heggie × RK4 "dense" output on its own grid fits inside `payload_mem` unless `Δ_sync` is calibrated small: **the size conflict is the unregularised occupants', and the word conflict with the note is everyone's.**

**The design, recommended in RQ-727:** the store is a per-column cache of boundary states, keyed by integer boundary index, at most `K` entries, default **`K = H`** (V26), the option closest to the note. An entry is `f(IC, k, sim key)`, so what a column holds decides what is ready, never what a row shows: a row reads its own boundary (§ 18.2), from an entry or from a march, and no other. **The required set** is the boundaries the window's rows read, at or below the column's front `f`: a function of `(t_lo, t_hi, warp, rows)` and the occupant's grid, at most `min(H, B)`, so never above `K`, the same for every column up to its front, and independent of `T` and of the order of visits. A march stores what it crosses while there is room. Entries outside the required set go first at `K`, least recently read; that order moves no value and is the build's. `f ≤ T` is the column's march front: a new column's is `t_hi` (§ 18.4), and a terminal latches it (§ 18.2). Beside the entries a column keeps one live state at `f`. `bounded` isn't stored: a read sets it at `T`'s own boundary, since nothing else in an entry reads `T` (§ 18.2). `T` is the stored top and the label says so, so the march never passes the axis's horizon. `K = 2H` is a labelled extra under the tier's budget and the "are you sure?" system.

| Event | Required set | Entries | March |
|---|---|---|---|
| Pan or zoom of the window, a change of warp or rows, a finer window | follows the window | kept; one read again is a re-read | a required boundary below `f` not held re-integrates the column from `t = 0`, progressively, saying so ("refining · 72%"), as scrub-back under R-66, its rows "arriving" until then |
| Window past `f`, `T` unchanged | the cut rises | gains the new rows | on from `f`: the order's "extending the window to later t integrates further per column". It is the same run, so no exception and no 65.535 line applies, and the build doesn't gate it on § 18.2's exception. A column pays only for the part it reaches, and § 18.5's cost stands |
| Raising `T` | unchanged: the window hasn't moved | kept where § 18.2's exception holds; otherwise a new run, empty, labelled | none until the window passes `f` |
| Lowering `T` | `t_hi` follows it, `t_hi ≤ T` | dropped: the caching contract's `T` row stands, RQ-727's exception being for raising only | a new run from `t = 0` |
| IC pan or zoom | the same as its neighbours' | a new column, empty | from `t = 0` to `t_hi` |

A finer window is the first row of this table; it stops **when the row spacing reaches one boundary: there is nothing finer**. Going above `T` raises it, § 18.2's continuation. So the order's "re-reads stored trajectories" holds in full where every boundary fits, `B_T ≤ K`: under the default occupant at `T = 50`, `H = 1080`, any `Δ_sync` above 0.0463 (50/1080). Past that a pan or zoom can ask for boundaries not held. It holds for those held. A Result (R-572) with the rows as
"stored times" is the sanctioned way to keep more: disk, streamed, budgeted.

**Why a cache and not a fixed stride.** Rows read `⌊t/Δ⌋`, so a store on one boundary in `b = ⌈B_T/K⌉`, `B_T` the boundaries up to `T`, misses most of what the rows read. Measured by `docs/incoming/2026-11-06_chart_composition/checks/store.py`, `H = 1080`:

| `T`, `Δ` | `B_T` | `b` | Rows off the stride, linear / log warp |
|---|---|---|---|
| 50, 0.0125 | 4,000 | 4 | 75% / 73% |
| 13, 0.001 | 13,000 | 13 | 92% / 92% |
| 50, 0.001 | 50,000 | 47 | 98% / 98% |
| 50, 0.1 | 500 | 1 | 0% / 0% |

The same script's second part is a bookkeeping model over random events, 1,500 at each of four `Δ`, at `K = H = 270`: 0 wrong reads, 0 entries over `K`. Entries there are pure by construction, so it tests the rule's counting, not kernel bits. The kernel test is `time_column_matches_playhead`.

### 18.4 Interactions

| Interaction | Specification |
|---|---|
| Hover trace | shows the column's trajectory, with a marker at the hovered row's `t`. Vertical motion moves only the marker, with no new run. Horizontal motion changes the IC, a new trace, as a hover does today |
| IC Inspector | the column's IC, editable, and the state at the row's `t`, read-only. An edit that stays on the chart's line moves the selection to that column. One that leaves it stays open, marked `off this chart`, encode's landing as today |
| Tilt | the IC axis only. Time is never a tilt target, and the compass says so ("time is not tiltable in this chart"). The tilt plane is `z(u)` with the slice's normal |
| Time zoom and pan | § 18.3. The window is a view; integration always starts at `t = 0`; zoom floors at one boundary. A window moved past a column's front marches that column on from it. Raising `T` marches nothing by itself; a window then past `f` marches the column on, bitwise a continuation (§ 18.2): regularised occupants to R-610's cap; unregularised to `T = 65.535`, past it a new run, labelled |
| IC zoom and pan | new columns, each a run from `t = 0` to `t_hi`, its front until the window passes it (§ 18.3). Rows' times stay |
| Morph, time to time | allowed when both have the same time window and warp: time fixed, the IC axis morphs by § 17's arms. Every frame is a new set of columns, class (b). Different windows: a cut. *11 Nov 2026 (R-926 item 3; applied per R-369, listed for veto):* time blends linearly, so two time windows blend like any two windows; "Different windows: a cut" and V29 are retired |
| Morph, time to IC-only | **a chart discontinuity: a hard cut,** shown with Sequence's cut mark. No dissolve is offered: `TimeToIcCut`, named in the planner row. This replaces part D's "refused for now". *11 Nov 2026 (R-926 items 2 and 3, as the status block has it; R-950):* gives way: time is a coordinate of every map and the morph runs; under a lock the anchor slides and no jump stays |
| The hybrid | blending readout time towards a fixed playhead: **recorded as a possible future option, not designed.** No field, no test |

### 18.5 Cost per column

Criterion, written before measuring: `docs/incoming/2026-11-06_chart_composition/checks/column-criterion.md`. Stand-in: the kernel's own wrapper loop, Yoshida-4, f32, native CPU, one thread, no GPU;
64 columns over region D at `η = 0.3`, `T = 13`, `dt = 10⁻³`, 1080 rows, on a throwaway, never pushed, under load.

| Criterion | Limit | Measured | Verdict |
|---|---|---|---|
| C1, correct | every row's `r`, `p`, `mean_y`, `C_ty`, `total_substeps` bitwise equal to a run with that row as horizon | 0 mismatches over 8 columns, every 97th row | **pass**. The state word differs by design: `bounded` belongs to the horizon |
| C2, overhead | snapshots ≤ 1.05 × none, median of 5 interleaved | 1.78 ms against 1.70 ms per column, **1.05**; fastest passes 1.35 and 1.35 | **pass, barely, under load** (load 27 to 46; passes ranged 1.35 to 2.09 ms). The 96-byte `SimStateBase`, not the 152 of § 18.3 |
| C3, saving | `H` naive renders ≥ `H/4` columns | 17 sampled rows cost 15.1 ms against 1.2 ms for a column: 0.74 of a column per row, scaled to about 800 columns at `H = 1080` | **pass**; theory says 0.5 per row, and the sample is small and loaded |


**Cost:** about 1.7 ms per column on this stand-in, 1.6 µs per row. A column costs one run to `t_hi`, against `H` runs at their rows, which on average take half of it, so about `H/2` times less than the naive reading: per column, not
per pixel, as the order asks. The live cost per frame is `W` columns' `dt`, against `W × H` samples' for an ordinary chart. **The stand-in is an unregularised occupant, and C1 proves nothing for a regularised march. The re-measure on Heggie × RK4, the default (R-560), is owed**, with row spacing at `Δ_sync`, TASK-M8-114. **Also not measured:** the GPU, a built column
entry (its size against the cap, § 11: it is an integrate entry, not a decode pair, and counts in C2), the colour pass with a per-pixel `t`, and Windows.

### 18.6 Tests

Each with a control (R-176), TASK-M2-83 unless named (R-916; *was* "TASK-M8-113 unless named"), through § 18.7's trajectory
interface on its exact fixtures. `time_column_matches_playhead` and `time_extend_continues` read the real march, so they are
TASK-M8-113's (REQ-CHART-178), after M2, on the real integrator through the same interface (TASK-M3-67).

| Test | Asserts | Control |
|---|---|---|
| `time_column_matches_playhead` | a column at row `t` equals a normal render of that IC at playhead `t` under the same `T`, bitwise, on `r`, `p`, accumulators, `total_substeps`, the state word and the free-group word, a column whose later crossing cancels symbols included; an unregularised arm and a regularised arm; after any order of § 18.3's events, every row not "arriving" (TASK-M8-114's arm) | unregularised: a row re-integrated at another `T` past 65.535. Regularised: a row read one boundary off. The word: a row reading a prefix of the final word. A row reading the nearest stored boundary instead of its own (TASK-M8-114's arm) |
| `time_zoom_floor` | the window stops at one boundary, per occupant, with the UI's reason; rows are integer indices | a float row time, which lands one low on 731 `k` |
| `time_extend_continues` | raising `T` continues bitwise from the front where the exception holds, else a new run, labelled; it leaves the required set and the entries, and the old top boundary reads without `bounded` (TASK-M8-114's arm); a window moved past a column's front, `T` unchanged, continues it bitwise, a column made by an IC pan included; a column marched to `t_hi` and on to `T` holds, on every boundary both reach, the bits of one march to `T` | the signature re-boot taken; an entry keeping the old `T`'s `bounded` (TASK-M8-114's arm) |
| `time_rows_on_boundaries` | every row reads the latest boundary at or before it; log rows stack on one boundary | a row stopped between boundaries |
| `time_terminal_holds` | rows past termination hold the terminal state | marching past it |
| `time_jitter_ic_only` | copies differ in the IC coordinate only, with Time on either axis | a `t` offset; `u` hard-wired to the horizontal axis |
| `time_two_axes_refused` | `TwoTimeAxes`, by name | a second time axis accepted |
| `time_window_frozen` | one window over all rows | `per_frame` |
| `time_morph_rules` | time to time needs equal windows; time to IC-only is a cut, `TimeToIcCut`. *11 Nov 2026 (R-926 item 3):* a time-to-time morph with different windows runs, its ends exact, and time to IC-only morphs (§ 18.4) | a dissolve offered. *11 Nov 2026:* a cut |
| `time_anchor` | a lock reads `(IC, t)` bitwise across navigation | `t` on the sim key |
| `time_memory_budget` | the store is `min(H, B)` per column up to `K`, each row with its word when symbolic features are active; after any order of § 18.3's events, the entries are at most `K`, every required boundary is held or its row is "arriving", the required set equals across columns up to their fronts, and one live state per column sits beside them; it honours the tier's budget | no ceiling; a required entry evicted while one outside the set remains |

### 18.7 The trajectory interface (R-918, R-919)

**A definition (R-72), its substance the owner's (R-918, R-919 F18); built by TASK-M2-82 (REQ-CHART-173).** M2's maths
that reads a trajectory, the time-axis rows of § 18.2 to § 18.4, G9's time arm (§ 17.8, REQ-CHART-174) and the anchor at
`(IC, t)`, reads it through one interface in `engine` (R-864 item 5), never through the integrator, so that it is tested in
M2 against trajectories known exactly, and the real integrator plugs in after M2 (TASK-M3-67, TASK-M8-113) with no change
to the maths that reads it.

**What a provider gives,** for a signed start state `(m, r, p)` in the scale gauge and a horizon `T`:

| Item | Definition |
|---|---|
| The boundary grid | integer boundary indices `k = 0, 1, …`, per occupant, at the quantum the provider declares; on the real integrator R-565's fixed sync quantum (TASK-M3-40), or the macro-step grid for an unregularised occupant (§ 18.2). A row's time maps to the latest boundary at or before it by index arithmetic, never a float division (§ 18.2's 731 `k`) |
| The state at a boundary | the signed state at boundary `k`, `(m, r, p)` in the start's gauge, the mirror never folded (R-854) |
| The terminal | none, or the boundary index at which the trajectory latches and its kind (Part 6's classes); every later boundary reads the terminal state (§ 18.2) |
| Purity | the same bits for the same `(start, T, k)`, in any order of reads and after any other read |

**The fixtures (R-919, F18).** The three-body fixtures are exact:

1. **The Lagrange equilateral homographic solution** (`principia_dd_validation_orbits.md` § 1.3), in closed form at any
   masses: the equilateral central configuration given its circular velocity turns rigidly, the state at `t` the start
   rotated by `ωt` about the centre of mass, `ω² = G·M/a³` with `M` the total mass and `a` the side; its elliptic and
   homothetic members through Kepler's equation, the homothetic collapse from rest a terminal (a triple collision) at its
   closed-form time.
2. **The figure-eight** (§ 1.2), exactly periodic: its state at `kT_8`, `T_8` its period, is declared equal to the start, its
   start known to the digits that section gives, so its test asserts the declaration, not the start's digits. Its
   boundaries are the multiples of `T_8`, so every boundary row it gives equals its start: no check that tells boundaries
   apart can fire on it.

**Two-body Kepler,** in closed form through Kepler's equation, is used only where the interface is generic, a test of the
interface itself, never as a three-body state. **The constants** of every fixture live in a crate below `engine` and
`validation` (`ledger` or `kernel`), and `validation`'s `figure_eight()` reads them from there, so that `engine`'s tests read
the same numbers.

**What the tests assert** (TASK-M2-82): a row reads the provider's state bitwise; the figure-eight's row at `kT_8` equals
its row at 0 bitwise, by the declaration; the Lagrange states, evaluated in closed form, within the bound derived from that
evaluation's rounding, or else a calibration (R-71), its criterion written first and set by measurement (R-649, R-650);
the terminal at its index and kind; two reads in either order, one bit pattern. Controls: a row reading one boundary off,
a terminal not held, a read that depends on an earlier read. The row-one-boundary-off control, and every check that tells
boundaries apart, runs on the Lagrange fixture's circular member (the rigid rotation, whose period is `2π/ω`; the
elliptic members run on the Kepler period and the homothetic one has none), with every boundary the test reads within one
period, `kΔ < 2π/ω` for each boundary index `k` it reads at the quantum `Δ`. Then any two boundary rows it reads differ,
which a quantum that is merely no multiple of the period does not give (at `Δ = T/2` or `T/3`, rows `k` and `k + 2` or
`k + 3` are the same state, equal within the closed form's rounding, since `cos` and `sin` at `ωkΔ` and `ωkΔ + 2π` differ
in the last bits); on the figure-eight every boundary row is its start, so such a check passes there whatever it reads
(physics reviews 5834013503, 5834538186 and 5834775665 of PR #465).

---

## 19. Time-axis mode in the GUI (order part D, UI)

Checked against `docs/gui/principia_render_gui_spec.md`, `docs/design/principia_sequence.md`, and `docs/design/principia_export_details.md`. **The rule: the
playhead stops being a control and becomes the vertical axis.** The artboards are PNGs under `docs/gui/`; this document has no image form, so the mock-ups are text.
GUI review (⚑) where it's built.

| Order's bullet | Specification | Checked against |
|---|---|---|
| Playhead controls disabled, with the reason shown | Play, step, the scrubber, speed and their keys are dimmed, shown and never removed, each with the reason "time-axis chart: time is the vertical axis", beside the control, never over the figure | §G1 (shown disabled, R-303's pattern, R-571); §G9a's "disabled, each with the reason … shown … beside it, never over the figure" |
| The horizon control becomes the time axis's range | Run's `T_horizon` stays the top of the axis, and the row gains the window `t_lo`, `t_hi ≤ T` and the warp. The strip's horizon item (R-621) reads `t 0.5–13`. `t_lo` is new; integration still starts at 0. Time zoom stops at one boundary, reading "one boundary: `Δ_sync`" | §G5 Run; R-621 |
| Entering and leaving keep the playhead | `RenderState`'s playhead isn't written in the mode, so leaving restores it exactly. Transport stops on entry and stays paused on leaving (V27). The trace's own clock stops with it | §G2 Time; R-101, playback never enters undo. Entry is a chart change: one undo entry |
| Inspector and hover read `(IC, t)` | The hover readout's first line gains `t = 7.21` (the real boundary time). A click pins, and **Open full viewer…** opens at that `t` | §G2's pin order; §G8; trajectory viewing |
| Builder offers time once per chart, linear or log | **Time** in the kind selector of either axis, once: on the other axis it is dimmed with "one time axis per chart". The row shows range, warp, rows | §G7, "Each axis has a kind": its sentence gains the mode; § 18.1 |
| Ticks and a badge | Ticks in canonical time units; a `gauge` paper time isn't offered, and the axis carries `paper scale differs` where `ℓ` moves. A badge in the header strip, `Time axis · t 0.5–13 · playhead held at 7.20`, so a frozen playhead never reads as a fault | R-523's badge rule, as the result badge is |
| Sequence | the Playhead lane is dimmed with the reason beside it. A keyframe's playhead policy is disabled. Its stored value is kept | §G9a's lanes; result view's precedent |
| Lock shows `(IC, t)` | The badge reads `locked · IC … · t 7.20`, the gold reticle at that pixel; **open in Inspector** opens the IC at that `t`. Across a cut to IC-only the lock keeps the IC, `t` dropped, and the badge says so | §G4 |
| Export sidecars | The export block gains `time_axis: { ic_axis, t_lo, t_hi, warp, rows }`, beside `range_policy: frozen`; `SimConfig` carries the mode, so the round trip is R-484's | export details § "export block"; gui_state_contract's sidecar; canonical serialisation |
| Keyboard, §G3 | Below | §G3; R-466; R-409 |

**Keyboard (§G3).** Scope order is unchanged, as R-466 has for a dimmed control: "Enter on a reset acts as its click, and on a dimmed one does nothing, the order unchanged". In the mode:

| Key | Scope | Today | In the mode |
|---|---|---|---|
| Space | global | plays | **does nothing**, and posts the reason to the footer once per press, as `Alt+L`'s dimmed precedent (R-476) |
| ← → | Time, interaction mode | step | do nothing, the same |
| Tab | Time | scrubber, then strip | still reaches each control; dimmed ones announce the reason (accessibility text), unchanged order |
| arrows | Figure | pan | along the IC axis pans the IC, along the time axis pans time; Shift ×10, Alt ×0.1, R-409 |
| + / − | Figure | zoom | zooms the IC axis; **Ctrl + / −** zooms time. New, a binding to be checked against §G3's table |
| arrows | Compass, tilt mode | tilt | `τ₁` only; `τ₂` does nothing, with the reason |
| K | Figure | lock | locks `(IC, t)` |

**Mock-ups.** Each changed screen; artboard values are illustrative.

Main view (§G2), the band below the figure, the header strip with its badge:

```
┌ Figure header ─────────────────────────────────────────────────────────────────────────┐
│ Region D · η = 0.30   [Time axis · t 0.5–13 · playhead held at 7.20]        t 0.5–13 ▾ │
├───────────────────────────────────────────────────────────────────────────────────────┤
│  t                                                                                    │
│  13 ┤ ░░░░░▓▓▒▒░░  ▒▒▒▒▓▓░░░░░  terminal rows hold                                     │
│  10 ┤ ░░▒▒▓▓▒░░░░░▒▒▓▓░░░░░░░░                                                         │
│   5 ┤ ▒▒▒▒▒▒▒▒▒▓▓▓▒▒▒▒▒▒▒▒▒▒▒▒     ◉ locked (IC, t)                                    │
│ 0.5 ┤ ────────────────────────                                                         │
│       ξ 0.0        0.2        0.4       0.5                                           │
├─ Compass ────────┬─ Time ───────────────────────────────────┬─ Legend ─────────────────┤
│ tilt: IC axis    │ ▶ ▮▮ ⏭ speed  ▬▬▬●▬▬▬  (all dimmed)       │ fixed window              │
│ time is not      │ time-axis chart: time is the vertical axis│ across all rows           │
│ tiltable here    │ playhead held at 7.20, restored on leaving│                           │
└──────────────────┴───────────────────────────────────────────┴───────────────────────────┘
```

Hover and click:

```
  column ξ = 0.214                t = 7.21            hovered row, a real boundary
  ┌ trace ───────────────┐
  │    ╭──●──╮  ← marker │        up/down: marker only, no run
  │  ──╯     ╰─          │        left/right: new column, new trace
  └──────────────────────┘
  click → pin → Open full viewer… (opens at t = 7.21) · IC Inspector…
```

Chart builder (§G7):

```
┌ Chart builder ───────────────────────────────────────────────────────┐
│ H axis  kind [ Physical quantity ▾ ]   ξ   range 0 – 0.5             │
│ V axis  kind [ Time ▾ ]                                              │
│         range 0.5 – 13    warp ( ) linear  (•) log    rows 1080      │
│         stored rows 1×  ≈ 1.3 GB at this tier   [ 2× ]               │
│ kinds:  Latent · Physical quantity · Burrau · Curve │ Time           │
│         (on H: Time dimmed: one time axis per chart)                 │
│ Pairing: tilt IC axis only · morph to IC-only: cut                   │
└──────────────────────────────────────────────────────────────────────┘
```

IC Inspector (§G8):

```
┌ IC Inspector ─ column ξ = 0.214 ─────────┬ State at t = 7.21 (read-only) ─┐
│ body 1 ●  drag · edit: moves to its      │ r₁ …  p₁ …                     │
│ body 2 ●  column on this chart           │ E  …  L_z …                    │
│ body 3 ●  [off this chart] if it leaves  │ status running                 │
└──────────────────────────────────────────┴────────────────────────────────┘
```

Lock badge (§G4):

```
 ◉ locked · IC ξ 0.214 η 0.30 · t 7.20        unlock · open in Inspector
```

Run window's horizon row (§G5):

```
 Horizon T [13]  window t_lo [0.5] t_hi [13]  warp [log ▾]   integration starts at t = 0
          regularised: one boundary is Δ_sync, the finest row
          unregularised: past 65.535 the macro-step changes: a new run
```

Sequence (§G9a):

```
 lanes  Navigate ─◆───◆──  Plane ─────  Chart ──◆──┃cut┃──◆
        Playhead  ░░░░░░░░ time-axis chart: time is the vertical axis
 segment  time → IC-only: cut (no morph or dissolve), TimeToIcCut
 keyframe  playhead policy: hold · advance · set  (dimmed, same reason)
```

Export sidecar:

```
export { range_policy: frozen,
         time_axis: { ic_axis: "ξ", t_lo: 0.5, t_hi: 13.0, warp: "log", rows: 1080 }, … }
```

**Gaps.** "Time zoom" `Ctrl + / −` needs §G3's table checked by the gui reviewer. A morph between two time charts has no Sequence words yet beyond § 18.4.

---

## 20. Paper frames as presets (order part E)

Composition, not machinery. A preset is a saved pair of registered axes with its ranges, base point, centre, window and mask (§G7), plus the frame's map, always § 8.8's ratio. For these presets, whose axes aren't two latent coordinate axes at one scale, the base point, centre and window are undefined until RQ-711 is answered or a task defines them under R-72, as TASK-M9-45 does for R-798's preset. Each is tested by
`preset_paper_coords` against the paper's own numbers, TASK-M2-77. `INC` and `PP` as § 3.2. A preset whose offset is below the working float's resolution, `δ = 5·10⁻⁸` or `10⁻⁷` at f32, carries the label `offset below float resolution` rather than
a tolerance: a CPU batch rounds once to f32, so such a chart would integrate one start. `BodyPlane` is registered in § 3.

| Target frame | Preset over the registry | Map | Test | Source |
|---|---|---|---|---|
| Agekyan–Anosova `(ξ, η)` | third-body pair, full disc, filter on: D (§ 10.4) | the identity | `fixtures/anosova/table1.csv` | `INC/2026-10-27_papers/TARGETS.md` lines 472–476; `INC/2026-10-25_replication/catalogue.md` § 1.1 |
| Anosova 1969 `(ξ′, η′)`* | third-body pair, full disc `R = 10`, filter on: `η ≥ 0` (§ 10.4) | the identity, no D mask | as § 8.8's AN-1 row | `INC/_local/anosova_1969_Ap_5_81_part3.pdf` pp. 81–82; **already have** |
| Tanikawa `D₁₁`, **RQ-650 open** | third-body pair | **as printed, not a mirror:** bodies 2, 3 at `(∓½, 0)`, body 1 at `(x, y)` | a TCP | `PP/tanikawa_mikkola_2008_arXiv0802.2465.pdf` § 3.1, Fig. 3; `INC/_local/tanikawa_mikkola_2015_PASJ_67_115.pdf` Fig. 1; `INC/2026-10-28_external_sources/README.md`, RQ-650's record |
| Tanikawa `D₁₂` | third-body pair, mask below | Fig. 3 right: `y > 0`, inside the circle about `(½, 0)`, outside that about `(−½, 0)` | membership | the same Fig. 3 |
| Lehto lens, four copies of D | third-body pair, mask `\|P\| ≤ 1`, `\|P − (1, 0)\| ≤ 1` | `x ↦ ξ = x − ½`, `y ↦ η`, mirror-symmetric | one point per quadrant | `TARGETS.md` § Lehto; `PP/lehto_etal_2008_MNRAS_388_965_arXiv0805.2844.pdf`. The lens is **inferred** from the stated ranges |
| Valtonen book units, ×2 | third-body pair | `(ξ, η) = (x/2, y/2)` | `(0.4, 0.6)` | `TARGETS.md` V-1 |
| Breen barycentric | a body-plane pair | `ξ + iη = −(3/2)(1 + a + ib)/(a − 1 + ib)`, a Möbius map | to the ratio, 10⁻¹⁵ | `PP/breen_etal_2020_MNRAS_494_2465_arXiv1910.07291.pdf` Fig. 1. **Singular point `(−0.5, 0)` confirmed**: `x₂ = x₃` there; the text's `(0.5, 0)` is a sign slip |
| Lagrange apex offset (H-3) | third-body pair about `(0, √3/2)`, log warp available | `(Δx, √3 + Δy)/2` at side 2 | Table III's rows | `TARGETS.md` H-3, Heggie 1974 |
| Parischewsky `(S₁/S₂, S₁/S₃)` | side ratios under a pair map, `a = q/p` coupling both axes; through R-798's remap until RQ-713 is ruled | `a = q/p`, `b = q`, then § 8.3; `S_k` opposite particle `k`, unsorted; `η ≥ 0`: **chirality lost**, `reduced domain` (§ 8.8) | `(0.8498…, 0.7211…) → (0.1, 0.6)` | `PP/parischewsky_trani_leigh_2023_SciPostCore_6_016_arXiv2108.06335.pdf` §§ 2–3 |
| Near-rest offset (AA-8) | a momentum axis through a point | eq. 3, § 8.6's `q̄` | relative energy 10⁻¹⁴ | `TARGETS.md` AA-8; Anosova I eq. 3 |
| Polar displacement about a base IC (AAOS 1994a) | polar pair map on `BodyPlane` about the body's Burrau position, log warp on `r`, COM re-centred after | `xᵢ = x̄ᵢ + δ cos α` | a ring point, per body | `TARGETS.md` § AAOS 1994a, eq. 1 |
| Log-polar about a TCP (TM-1) | polar pair map on `(ξ, η)` about the point, log warp | radii 10⁻³ to 10⁻⁵, azimuths per Table 4 | Table 4's points | `INC/2026-10-25_replication/catalogue.md` § 1.5; `INC/2026-10-29_research_batches/tanikawa_mikkola_2021/` |
| Rotating family (Valtonen 2005) | `L²E` | `w = −L²E/(G²m⁵)`: **an alias of `L²E`** | `w` at a built state | `INC/_local/valtonen_myllari_orlov_rubinov_2005_MNRAS_364_91.pdf` eq. 1 |

**Catalogue energy normalisations are a stated unit conversion, never an axis.** `E = −½` (Simó: `M = 1`, § `INC/2026-10-27_hp_ics/README.md` line 107), Hudomal's (`TARGETS.md` § Hudomal), are § 8.6's `E_paper = G M² E_c/ℓ`, with `ℓ` scaled so the energy matches: a function in the comparison, one number per orbit, on the import's path. A scan at fixed paper energy, § 3.2's Orlov, Mikkola–Hietarinta and Titov rows, is the same conversion with `ℓ` a function of the pixel, not one number per orbit.

**Gaps.** The order lists these as frames the library can't determine. Checked; four of them it can:

| Item | Finding | What's left |
|---|---|---|
| Tanikawa–Mikkola's fixed-`k` prescription | **In the repo:** 2008 § 3.1, § 4.2, rigid rotation at `k = T/\|U\|` (§ 3.2) | the rotation's sign, and Kuwabara & Tanikawa's paper, not held. R-669 stands |
| `D₁₂` | **In the repo:** 2008 Fig. 3, read above. And `{x ≥ 0, y ≥ 0, (x − 0.5)² + y² ≤ 1}`, printed as "D11", **is exactly `D₁₁ ∪ D₁₂`**: that reading, not a typo or a mirror, is new for RQ-650 | RQ-650 is open; the catalogue's "mirror labelling" stands until it is ruled |
| Hristov 2025's conventions | `INC/2026-10-27_hp_ics_freefall/SOURCES.md`: free-fall `(px, py, T, T*)`, Euler `(vx, vy, T, T*)`; paper not held | what `px, py` are: a gap |
| Valtonen 2005's `w` | **Held:** `w = −L₀²E₀/(G²m₀⁵)`, eq. 1 | Method D's velocities are "isotropic": statistical, no plane |
| Martynova 2009 | **Held:** scan of `(k, φ)`, § 3.2's row | body positions by formula, read on the page at the task |
| Boyd & McMillan 1993 | `INC/_local/research_batches_2026-10-29/papers/Boyd_1993_Chaotic_Scattering.pdf`, held, not read; R-669's reach | a gap; RQ-726 |
| Hut 1983 | catalogue § 4.1 only; mostly 3D | a gap |
| Heggie & Sweatman 1991's full setup | triage row 67; not on hand; M9, not scored | a gap |
| Hénon 1974's `A` | R-669, its values stored; `A`'s definition not read here | a gap |
| L/M/A/H boundaries | `TARGETS.md` V-1: "It does **not** define the boundaries quantitatively" | a gap; `INC/_local/heinamaki_lehto_valtonen_chernin_1999_MNRAS_310_811.pdf` not read |

**Gaps found here.** Region D's title in catalogue § 2.5 (§ 8.8). Li & Liao's "all possible configurations" at unequal masses (§ 8.8). REQ-VAL-326's pointer (RQ-731). TARGETS's Lehto lens is inferred. Breen's sign slip. Order's Chazy path: it is `docs/design/principia_dd_chazy_final_motion.md`, not under `docs/read_first/`.
