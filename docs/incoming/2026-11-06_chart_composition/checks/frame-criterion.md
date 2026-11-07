# Live chart-path frame: the target, written before measuring (6 Nov 2026)

Target: a live morph's chart side, on one in-motion coarse cover (1920x1080 at 1/8 linear, 32,400 samples; caching
contract: "full-canvas coarse cover", "one or two dispatches' worth of quads"), takes at most 4 ms of GPU decode on this
Mac (M3 Pro, Metal), p50 of 9.

Reason: the order says live behaves like pan and tilt, so a morph frame must leave the integrator what a tilt frame
leaves it. The corpus sets no number: `frame budget` is a setting. A 60 Hz frame is 16.7 ms; a quarter of it, 4 ms, is
the most the chart side may take before the coverage regime visibly slows compared with a tilt. The number is a design
number, for veto.

Measured: `DecodeStage::run` on real entries, ShapeSphere and Latent. A morph sample is five decode-sized steps: decode A,
decode B, inverse of each, forward of the blend. The stand-in is therefore 5 x the slowest entry's time per cover, the
readback's cost in it. The blend entry isn't built. Readback overstates; a stand-in isn't the entry.

Not measured: the integration of the cover (a tilt pays the same), so the whole frame's time stays owed; an entry
reading two IC buffers; Windows until its arm runs.

CPU side, for export and CPU-only ends: criterion in morph-criterion.md, unchanged.
