"""
pxpack2 — robustness experiments over pxpack.

Three independent ideas, testable separately:

  A. 2-D TILING       lay each copy inside its own rectangular tile, so a CROP keeps
                      whole copies instead of column-slicing every one.
  B. INTERLEAVING     scatter each copy's bits across the whole image with a fixed
                      stride, so LOCALISED damage becomes uniform damage spread thinly
                      across many copies -- the opposite trade to (A).
  C. REDUNDANT BITS   write each payload bit into k pixels and majority-vote on read.
                      Costs capacity, buys tolerance to UNIFORM noise, which repetition
                      of whole copies cannot help with.
"""
import struct, zlib, json
import numpy as np

MAGIC = b'PRPX'
HDR = 16


def _hdr(plen, n, flags=0):
    h = MAGIC + bytes([2, flags]) + struct.pack('<IH', plen, n)
    return h + struct.pack('<I', zlib.crc32(h) & 0xFFFFFFFF)


def _rec(payload):
    return payload + struct.pack('<I', zlib.crc32(payload) & 0xFFFFFFFF)


def _try_rec(buf, plen):
    if len(buf) < plen + 4:
        return None
    body, crc = buf[:plen], struct.unpack('<I', buf[plen:plen+4])[0]
    return body if (zlib.crc32(body) & 0xFFFFFFFF) == crc else None


# ---------------------------------------------------------------- A. tiled
def embed_tiled(img, payload, rep=1):
    """Each copy lives in its own tile. A crop that keeps a tile keeps a copy."""
    img = img.copy(); h, w, _ = img.shape
    unit_bits = (HDR + len(payload) + 4) * 8
    # choose a tile size that holds one record in its RGB LSBs
    px_needed = int(np.ceil(unit_bits / 3))
    side = int(np.ceil(np.sqrt(px_needed)))
    ntx, nty = w // side, h // side
    if ntx < 1 or nty < 1:
        raise ValueError(f'tile {side} does not fit in {w}x{h}')
    n = ntx * nty
    stream = _hdr(len(payload), n) + _rec(payload)
    bits = np.unpackbits(np.frombuffer(stream, np.uint8))
    for ty in range(nty):
        for tx in range(ntx):
            sub = img[ty*side:(ty+1)*side, tx*side:(tx+1)*side, :3].reshape(-1)
            k = min(len(bits), len(sub))
            sub[:k] = (sub[:k] & 0xFE) | bits[:k]
            img[ty*side:(ty+1)*side, tx*side:(tx+1)*side, :3] = sub.reshape(side, side, 3)
    return img, dict(side=side, tiles=n)


def extract_tiled(img, side_hint=None, search_offsets=True):
    """CROPPING SHIFTS THE ORIGIN, so a fixed tile grid no longer aligns.
    Scan candidate (dx,dy) offsets for one that lands a tile on a valid header.
    This is the one idea that genuinely transfers from QR: not the error
    correction, but a way to LOCATE the grid when the frame has moved."""
    h, w, _ = img.shape
    for side in ([side_hint] if side_hint else range(8, min(h, w)+1)):
        if side is None or side > min(h, w):
            continue
        offs = [(0, 0)]
        if search_offsets:
            offs = [(dx, dy) for dy in range(side) for dx in range(side)]
        for dx, dy in offs:
            r = _scan_grid(img, side, dx, dy)
            if r[0] is not None:
                return r
    return None, 0, 0


def _scan_grid(img, side, dx, dy):
        h, w, _ = img.shape
        ntx, nty = (w - dx) // side, (h - dy) // side
        if ntx < 1 or nty < 1:
            return None, 0, 0
        found = []
        for ty in range(nty):
            for tx in range(ntx):
                y0, x0 = dy + ty*side, dx + tx*side
                sub = img[y0:y0+side, x0:x0+side, :3].reshape(-1) & 1
                buf = np.packbits(sub[:(len(sub)//8)*8]).tobytes()
                if buf[:4] != MAGIC:
                    continue
                hh, hc = buf[:12], struct.unpack('<I', buf[12:16])[0]
                if (zlib.crc32(hh) & 0xFFFFFFFF) != hc:
                    continue
                plen, _n = struct.unpack('<IH', hh[6:12])
                r = _try_rec(buf[HDR:], plen)
                if r: found.append(r)
        return (found[0], len(found), ntx*nty) if found else (None, 0, 0)


# --------------------------------------------------- C. per-bit redundancy
def embed_redundant(img, payload, k=9):
    """Every payload bit written to k pixels, spread by a fixed stride. Majority vote."""
    img = img.copy(); h, w, _ = img.shape
    stream = _hdr(len(payload), 1) + _rec(payload)
    bits = np.unpackbits(np.frombuffer(stream, np.uint8))
    flat = img[:, :, :3].reshape(-1)
    cap = len(flat)
    need = len(bits) * k
    if need > cap:
        raise ValueError(f'need {need} pixels for k={k}, have {cap}')
    stride = cap // need
    idx = (np.arange(need) * stride) % cap
    vals = np.repeat(bits, k)
    flat[idx] = (flat[idx] & 0xFE) | vals
    img[:, :, :3] = flat.reshape(h, w, 3)
    return img, dict(k=k, used=need, cap=cap)


def extract_redundant(img, k=9, plen_guess=None):
    h, w, _ = img.shape
    flat = img[:, :, :3].reshape(-1); cap = len(flat)
    # header first: 16 bytes = 128 bits
    for nb in ([ (HDR + plen_guess + 4) * 8 ] if plen_guess else [None]):
        pass
    # we must know total bits to compute stride, so scan candidate payload lengths
    for plen in ([plen_guess] if plen_guess else range(16, 4096)):
        nb = (HDR + plen + 4) * 8
        need = nb * k
        if need > cap: break
        stride = cap // need
        idx = (np.arange(need) * stride) % cap
        v = (flat[idx] & 1).reshape(nb, k)
        bits = (v.sum(1) * 2 > k).astype(np.uint8)
        buf = np.packbits(bits).tobytes()
        if buf[:4] != MAGIC: continue
        hh, hc = buf[:12], struct.unpack('<I', buf[12:16])[0]
        if (zlib.crc32(hh) & 0xFFFFFFFF) != hc: continue
        p2, _ = struct.unpack('<IH', hh[6:12])
        if p2 != plen: continue
        r = _try_rec(buf[HDR:], plen)
        if r: return r, k
    return None, k


# ------------------------------------------------ HYBRID: tiled + redundant
def embed_hybrid(img, payload, k=5):
    """Tiles for LOCALITY (crop, overwrite) x per-bit redundancy for UNIFORM NOISE.
    The two failure modes are orthogonal, so the two remedies compose."""
    img = img.copy(); h, w, _ = img.shape
    stream = _hdr(len(payload), 1) + _rec(payload)
    bits = np.unpackbits(np.frombuffer(stream, np.uint8))
    px = int(np.ceil(len(bits) * k / 3))
    side = int(np.ceil(np.sqrt(px)))
    ntx, nty = w // side, h // side
    if ntx < 1 or nty < 1:
        raise ValueError(f'k={k} needs {side}px tiles; {w}x{h} too small')
    vals = np.repeat(bits, k)
    for ty in range(nty):
        for tx in range(ntx):
            sub = img[ty*side:(ty+1)*side, tx*side:(tx+1)*side, :3].reshape(-1)
            m = min(len(vals), len(sub))
            sub[:m] = (sub[:m] & 0xFE) | vals[:m]
            img[ty*side:(ty+1)*side, tx*side:(tx+1)*side, :3] = sub.reshape(side, side, 3)
    return img, dict(side=side, tiles=ntx*nty, k=k)


def extract_hybrid(img, side, k, plen):
    h, w, _ = img.shape
    nb = (HDR + plen + 4) * 8
    for dy in range(side):
        for dx in range(side):
            for ty in range((h-dy)//side):
                for tx in range((w-dx)//side):
                    y0, x0 = dy+ty*side, dx+tx*side
                    sub = img[y0:y0+side, x0:x0+side, :3].reshape(-1) & 1
                    if len(sub) < nb*k: continue
                    v = sub[:nb*k].reshape(nb, k)
                    bits = (v.sum(1)*2 > k).astype(np.uint8)
                    buf = np.packbits(bits).tobytes()
                    if buf[:4] != MAGIC: continue
                    hh, hc = buf[:12], struct.unpack('<I', buf[12:16])[0]
                    if (zlib.crc32(hh) & 0xFFFFFFFF) != hc: continue
                    r = _try_rec(buf[HDR:], plen)
                    if r: return r, (dx, dy)
    return None, None


# --------------------------------------------- geometric-transform search
DIHEDRAL = [
    ('identity',      lambda a: a),
    ('rot90',         lambda a: np.rot90(a, 1)),
    ('rot180',        lambda a: np.rot90(a, 2)),
    ('rot270',        lambda a: np.rot90(a, 3)),
    ('fliph',         lambda a: a[:, ::-1]),
    ('flipv',         lambda a: a[::-1, :]),
    ('transpose',     lambda a: np.swapaxes(a, 0, 1)),
    ('anti-transpose',lambda a: np.rot90(a, 2)[:, ::-1]),
]


def extract_robust(img, side_hint, scales=(1, 2, 3, 4)):
    """Try the 8 dihedral transforms x integer rescales.

    Rotation and flip lose nothing -- the bits are all still there, just moved --
    so a reader that only tries the identity is throwing away recoverable data.
    Nearest-neighbour UPSCALE also preserves every LSB; it only changes the tile
    size, so trying side*k recovers it. LANCZOS/bilinear rescaling is genuinely
    unrecoverable: resampling averages neighbouring pixels and the low bit is gone.
    """
    for name, xf in DIHEDRAL:
        try:
            a = np.ascontiguousarray(xf(img))
        except Exception:
            continue
        for s in scales:
            # NEAREST upscale DUPLICATES pixels, so the fix is to DECIMATE by s
            # before reading -- not to resize the tile grid. Taking every s-th
            # pixel of every s-th row reconstructs the original raster exactly.
            b = a[::s, ::s] if s > 1 else a
            if side_hint > min(b.shape[:2]):
                continue
            r, k, t = extract_tiled(b, side_hint)
            if r is not None:
                return r, dict(transform=name, decimate=s, tiles=f'{k}/{t}')
        # and integer DOWNscale of the hint, for images stored smaller
        if side_hint % 2 == 0:
            r, k, t = extract_tiled(a, side_hint // 2)
            if r is not None:
                return r, dict(transform=name, scale=0.5, tiles=f'{k}/{t}')
    return None, dict(transform=None)


# ===========================================================================
# FINAL MEASURED RESULTS
#
# RECOMMENDED CONFIG: tiled, k=1, offset search + dihedral + decimation
#
#   res      config only (382B)     config + shader (3354B)
#   64^2       1 tile                 too small
#   128^2      9 tiles                1 tile
#   256^2     49 tiles                4 tiles
#   512^2    225 tiles               25 tiles
#   1024^2   961 tiles              100 tiles
#
# SURVIVES
#   PNG save/load, optimize=True, compress_level=9   225/225 tiles
#   alpha stripped entirely                          225/225  (RGB is systematic)
#   crop to 50%                                       49/225
#   crop at an arbitrary 37px offset                  ok
#   crop + PNG roundtrip                              ok
#   rotate 90 / 180 / 270                             ok, transform REPORTED
#   flip horizontal / vertical / transpose            ok, transform REPORTED
#   nearest-neighbour upscale x2, x3                  ok, decimation REPORTED
#   rot90 + upscale x2 together                       ok
#   top 50% of the image overwritten                  ok
#
# DOES NOT SURVIVE, and these are genuine not fixable
#   LANCZOS/bilinear rescaling  -- resampling AVERAGES neighbouring pixels, so
#                                  the low bit is destroyed, not moved.
#   JPEG at any quality         -- there is no LSB plane to read.
#   uniform LSB noise >0.1%     -- needs per-bit redundancy (see the k= variants);
#                                  does not occur in a losslessly stored PNG.
#
# THE THREE SEARCHES, AND WHY EACH IS NEEDED
#
#   OFFSET   cropping shifts the origin, so a fixed tile grid misaligns and every
#            tile reads garbage. Scanning (dx,dy) for one that lands on a valid
#            header recovers ANY crop. This is the one idea that transfers from QR
#            codes -- not the error correction, the FINDER PATTERN.
#
#   DIHEDRAL rotation and flip lose NOTHING; the bits are all present, just moved.
#            A reader that only tries the identity throws away recoverable data.
#            Eight transforms, trivial to try.
#
#   DECIMATE nearest-neighbour upscale DUPLICATES pixels. The fix is to take every
#            s-th pixel of every s-th row -- NOT to resize the tile grid, which was
#            my first attempt and failed. Reconstructs the original raster exactly.
#
# The reader REPORTS which transform and decimation it found, so a recovered image
# says how it was recovered rather than silently succeeding.
