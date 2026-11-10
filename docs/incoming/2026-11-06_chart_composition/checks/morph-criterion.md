# Morph frame cost: the criterion, written before measuring (6 Nov 2026)

Question: does the CPU side of a morph frame (decode A, decode B, blend, decode the blend) stand beside the integration
it starts, or does it become the cost? Design doc section 17.

Measured: single-thread f64 `stage::decode::sample` on this Mac, ShapeSphere and Latent, ns per sample, median of 5
passes of 200,000 samples. Three decodes stand in for one morph sample.

Pass: 1920x1080 at one sample per pixel, the three decodes on the 5 performance cores, in at most 1 s per frame. The 1 s
is a design number, for veto; it is an interactive limit on a blocking export, chosen as well under a frame's march.

Honesty: one Mac, under queue load, release profile; a stand-in for the blend's own decode; no integration measured, so
this can't give a frame's total. It says only whether the CPU side is a bottleneck. SSAA multiplies it.
