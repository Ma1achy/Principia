# Re-measure of the axis cap: the criterion, written before measuring (6 Nov 2026)

The owner's feedback item 2: mark the cap provisional, re-measure with the real region-D pair on the Mac and on the
Windows runner (Vulkan, RTX 2080 SUPER), and report startup size and time at 1, 4 and 8 pairs.

P = baked copies of the real pair (region D's `(xi, eta)` closed form, the inner pair's centre-of-mass offset included,
eq. 8's mask, eq. 3's momentum through the explicit-configuration path), each a distinct monomorphisation. P in {0,1,4,8}.

Reported per P: `cargo xtask build-kernel` time (median of 3); spv and wgsl bytes (exact); naga parse and validate (median
of 5); the 12 gallery decode pipelines created in one pass with the module at P, three passes, reported first and
median of last two ("startup time"); the eager set 12+P, same; one pair's first creation.

Pass, as before, unchanged: C1 build ratio <= 1.25 and +30 s; C2 wgsl ratio <= 1.25; C3 eager set 12+P <= 2 s, one
creation <= 1.25 x baseline; C4 naga validates, parameters fit 24 floats.
Added, since pairs are created on first use (V7): C3' the 12 gallery pipelines at P <= 1.25 x at P=0, because every
pipeline parses the whole module, so baked pairs cost startup even when none is created.

The cap stays provisional until the Windows arm is in. A criterion is not relaxed for either machine. If the two
machines disagree, the lower number stands in the proposal and the item says so.
Copies of one pair stand in for four different pairs: the others aren't built.
Honest limits: this Mac under queue load, load recorded; sizes exact; times not.
