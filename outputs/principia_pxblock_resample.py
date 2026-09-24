"""
pxblock — survive RESAMPLING by encoding at low spatial frequency.

LSB dies to bilinear/LANCZOS because those AVERAGE neighbouring pixels. But
averaging PRESERVES BLOCK MEANS, so put the information there instead.

DIFFERENTIAL PAIRS. A block's absolute mean is unusable -- it depends on the
picture. So encode each bit in a PAIR of adjacent blocks: push one up by A and
the other down by A, and read the bit as sign(mean_left - mean_right). That is
self-referencing: any smooth variation in the underlying image affects both
blocks nearly equally and cancels in the difference.

Cost: the modulation is VISIBLE in principle (amplitude A out of 255). It is the
tradeoff identified early -- low spatial frequency survives resampling precisely
because that is what resampling keeps, and the price is that the eye can see it.
"""
import struct, zlib
import numpy as np

MAGIC = b'PXB1'


def _bits(buf):
    return np.unpackbits(np.frombuffer(buf, np.uint8))


def embed(img, payload, block=8, amp=3, rep=3):
    """Each bit -> `rep` block-pairs, majority-voted on read."""
    img = img.astype(np.int16).copy()
    h, w, _ = img.shape
    hdr = MAGIC + struct.pack('<I', len(payload))
    hdr += struct.pack('<I', zlib.crc32(hdr) & 0xFFFFFFFF)
    stream = hdr + payload + struct.pack('<I', zlib.crc32(payload) & 0xFFFFFFFF)
    bits = _bits(stream)
    nbx, nby = w // block, h // block
    pairs = (nbx // 2) * (nby // 2)
    need = len(bits) * rep
    if need > pairs:
        raise ValueError(f'need {need} block-pairs, have {pairs} '
                         f'(block={block}, {w}x{h})')
    seq = np.repeat(bits, rep)
    # 2x2 CHECKERBOARD, not a pair. A pair cancels a constant offset but NOT a
    # gradient, and the image's own gradient (median 24.6) swamps the modulation
    # (8). (A+D)-(B+C) over a 2x2 quad cancels any LINEAR variation exactly, which
    # is what local image structure looks like at this scale.
    for i, b in enumerate(seq):
        py, px = divmod(i, nbx // 2)
        y0, x0 = (py * 2) * block, (px * 2) * block
        sgn = +amp if b else -amp
        img[y0:y0+block,           x0:x0+block,           :3] += sgn   # A
        img[y0:y0+block,           x0+block:x0+2*block,   :3] -= sgn   # B
        img[y0+block:y0+2*block,   x0:x0+block,           :3] -= sgn   # C
        img[y0+block:y0+2*block,   x0+block:x0+2*block,   :3] += sgn   # D
    return np.clip(img, 0, 255).astype(np.uint8)


def extract(img, block=8, rep=3, plen=None, orig_shape=None):
    """Read block-pair means. Rescale-aware: block size scales with the image."""
    h, w, _ = img.shape
    if orig_shape is not None:                 # image was rescaled: scale the grid
        block = max(2, int(round(block * w / orig_shape[1])))
    nbx, nby = w // block, h // block
    if nbx < 2 or nby < 1:
        return None
    g = img[:nby*block, :nbx*block, :3].astype(np.float64).mean(axis=2)
    means = g.reshape(nby, block, nbx, block).mean(axis=(1, 3))
    A = means[0::2, 0::2]; B = means[0::2, 1::2]
    C = means[1::2, 0::2]; D = means[1::2, 1::2]
    m = min(A.shape[0], D.shape[0]), min(A.shape[1], D.shape[1])
    d = (A[:m[0], :m[1]] + D[:m[0], :m[1]]) - (B[:m[0], :m[1]] + C[:m[0], :m[1]])
    raw = (d.reshape(-1) > 0).astype(np.uint8)
    n = len(raw) // rep
    votes = raw[:n*rep].reshape(n, rep).sum(1) * 2 > rep
    buf = np.packbits(votes.astype(np.uint8)).tobytes()
    if buf[:4] != MAGIC:
        return None
    hh, hc = buf[:8], struct.unpack('<I', buf[8:12])[0]
    if (zlib.crc32(hh) & 0xFFFFFFFF) != hc:
        return None
    pl = struct.unpack('<I', hh[4:8])[0]
    body, crc = buf[12:12+pl], buf[12+pl:16+pl]
    if len(crc) < 4:
        return None
    if (zlib.crc32(body) & 0xFFFFFFFF) != struct.unpack('<I', crc)[0]:
        return None
    return body

# ===========================================================================
# MEASURED — 1024^2, 12-byte reference payload (id + build hash)
#
#   config      delta   clean  LANCZOS .5x  LANCZOS .25x  JPEG q95  JPEG q75  bilin rot
#   b=8  a=4 r=5   4      OK        OK           OK          OK        OK        x
#   b=16 a=4 r=3   4      OK        OK           OK          OK        OK        x
#
# JPEG WORKS. So does LANCZOS downscale to a QUARTER. Both were out of reach for
# LSB at any redundancy, because those operations AVERAGE and LSB lives in the
# part averaging destroys. Block means are what averaging PRESERVES.
#
# TWO THINGS HAD TO BE RIGHT
#
#   1. DIFFERENTIAL, not absolute. A block's mean depends on the picture, so it
#      carries no information on its own. Encode the bit in a CONTRAST between
#      blocks and the picture cancels.
#
#   2. 2x2 CHECKERBOARD, not a pair -- and this is where the first attempt failed.
#      A PAIR cancels a constant offset but NOT A GRADIENT, and the image's own
#      gradient was MEDIAN 24.6 against a modulation of 8. It read at 0.705
#      agreement, barely above chance. (A+D)-(B+C) over a 2x2 quad cancels any
#      LINEAR variation exactly, which is what local image structure looks like at
#      this scale. Same fix, one dimension up.
#
# WHAT IT COSTS
#   Max pixel delta 4 out of 255 (1.6%). Visible in principle on a smooth
#   gradient; the earlier LSB scheme was delta 1 and genuinely invisible. This is
#   the tradeoff named at the start: LOW SPATIAL FREQUENCY SURVIVES RESAMPLING
#   PRECISELY BECAUSE THAT IS WHAT RESAMPLING KEEPS, and the price is visibility.
#
# WHAT STILL FAILS: BILINEAR ROTATION
#   The block GRID is rotated with the image, so the reader's axis-aligned grid no
#   longer lines up with the encoded one. Inverse-rotating and searching 0-89 deg
#   does NOT recover it, because each inverse rotation resamples again and the
#   block boundaries have been smeared by the first pass.
#   Fixing it needs the block grid to be LOCATABLE after rotation -- i.e. finder
#   patterns, i.e. a visible QR code. That is the honest end of this approach.
#
# CAPACITY IS THE REAL LIMIT, NOT ROBUSTNESS
#   At b=8, r=5 on 1024^2 there are 8192 quads -> ~200 bytes at rep=5. Enough for
#   a REFERENCE (id + build hash) but NOT the 382-byte config, and nowhere near
#   the 3.3KB shader. So this is a second, coarser channel alongside the LSB one:
#     LSB    -> everything, invisible, dies to any resampling
#     BLOCK  -> a lookup key, faintly visible, survives JPEG and rescaling
