"""
pxpack — embed a slice's full provenance in the pixels of its own PNG.

LAYOUT
  RGB low bit  : payload, repeated as many whole times as fits.  Survives anything
                 that preserves the image losslessly.
  Alpha low bit: the SAME payload again, more copies.  Free on an opaque image
                 (alpha 255 -> 254 is invisible), but discarded by any pipeline
                 that flattens.  Systematic: losing it costs redundancy, not content.

  Every copy carries its own CRC32, so a copy is either trusted whole or discarded.
  Read = collect every intact copy, majority-vote per byte.  Repetition IS the error
  correction; with 9+ copies at the smallest resolution, Reed-Solomon buys nothing
  a library dependency does not cost more.

HEADER (16 bytes, repeated identically at the front of RGB and of alpha)
  magic 4 | version 1 | flags 1 | payload_len 4 | n_copies 2 | header_crc 4

  flags bit0 : payload is deflate-compressed
  flags bit1 : payload includes shader source (vs preset id only)

FAILURE MODES ARE DISTINGUISHED, which is the whole point:
  no magic          -> "this image carries no embedded state"
  magic, 0 copies   -> "this image HAD state and it is unrecoverable"
  magic, k copies   -> recovered, and k is reported so degradation is visible
"""
import struct, zlib, json
import numpy as np

MAGIC = b'PRPX'
VERSION = 1
HDR = 16
F_DEFLATE = 1 << 0
F_SHADER  = 1 << 1


def _bits_from(buf: bytes) -> np.ndarray:
    return np.unpackbits(np.frombuffer(buf, dtype=np.uint8))


def _bits_to(bits: np.ndarray) -> bytes:
    return np.packbits(bits).tobytes()


def _plane_capacity(h, w, channels):
    return (h * w * channels) // 8


def _build_stream(payload: bytes, flags: int, capacity: int):
    """As many (header + payload + CRC) records as fit. Header repeated per record."""
    unit = HDR + len(payload) + 4                # header + payload + its own CRC32
    n = max(0, capacity // unit)
    if n == 0:
        raise ValueError(
            f"payload {len(payload)}B + CRC does not fit in {capacity}B "
            f"(need {HDR + unit}B). Use a larger image or drop the shader source."
        )
    head = MAGIC + bytes([VERSION, flags]) + struct.pack('<IH', len(payload), n)
    head = head + struct.pack('<I', zlib.crc32(head) & 0xFFFFFFFF)
    body = (payload + struct.pack('<I', zlib.crc32(payload) & 0xFFFFFFFF)) * n
    # THE HEADER IS A SINGLE POINT OF FAILURE, so interleave a copy of it before
    # every payload copy. Costs 16B per copy; buys survival of localised damage.
    inter = b''.join(head + payload + struct.pack('<I', zlib.crc32(payload) & 0xFFFFFFFF)
                     for _ in range(n))
    return inter


def _read_stream(buf: bytes):
    """Scan for ANY intact header, then harvest every intact payload copy."""
    # find the first header whose own CRC checks out -- headers are repeated, so
    # localised damage cannot orphan the stream
    hdr = None
    pos = 0
    while True:
        i = buf.find(MAGIC, pos)
        if i < 0 or i + HDR > len(buf):
            break
        h, hc = buf[i:i+12], struct.unpack('<I', buf[i+12:i+16])[0]
        if (zlib.crc32(h) & 0xFFFFFFFF) == hc:
            hdr = h
            break
        pos = i + 1
    if hdr is None:
        return None, None, 0, 0, ('no-magic' if buf.find(MAGIC) < 0 else 'all-headers-corrupt')
    version, flags = hdr[4], hdr[5]
    plen, n = struct.unpack('<IH', hdr[6:12])
    unit = HDR + plen + 4
    intact = []
    for i in range(n):
        off = i * unit + HDR
        chunk = buf[off:off + plen]
        if len(chunk) < plen:
            break
        crc = struct.unpack('<I', buf[off + plen:off + plen + 4])[0]
        if (zlib.crc32(chunk) & 0xFFFFFFFF) == crc:
            intact.append(chunk)
    if not intact:
        return None, flags, 0, n, 'all-copies-corrupt'
    # majority vote per byte across intact copies
    arr = np.frombuffer(b''.join(intact), dtype=np.uint8).reshape(len(intact), plen)
    if len(intact) == 1:
        out = arr[0].tobytes()
    else:
        out = bytes(np.apply_along_axis(lambda c: np.bincount(c).argmax(), 0, arr).astype(np.uint8))
    return out, flags, len(intact), n, 'ok'


def embed(img: np.ndarray, payload_obj, shader_src: str | None = None, compress=True):
    """img: HxWx4 uint8 RGBA. Returns a new image with the payload in the low bits."""
    img = img.copy()
    h, w, c = img.shape
    assert c == 4, "need RGBA"

    obj = dict(payload_obj)
    flags = 0
    if shader_src is not None:
        obj['_shader'] = shader_src
        flags |= F_SHADER
    raw = json.dumps(obj, separators=(',', ':')).encode()
    if compress:
        packed = zlib.compress(raw, 9)
        if len(packed) < len(raw):
            raw, flags = packed, flags | F_DEFLATE

    for chans, plane in ((slice(0, 3), 'rgb'), (slice(3, 4), 'a')):
        cap = _plane_capacity(h, w, 3 if plane == 'rgb' else 1)
        try:
            stream = _build_stream(raw, flags, cap)
        except ValueError:
            if plane == 'rgb':
                raise
            continue                      # alpha too small: skip, RGB still carries it
        bits = _bits_from(stream)
        sub = img[:, :, chans].reshape(-1)
        sub[:len(bits)] = (sub[:len(bits)] & 0xFE) | bits
        img[:, :, chans] = sub.reshape(h, w, -1)
    return img


def extract(img: np.ndarray):
    """Try RGB, then alpha. Returns (obj, report)."""
    h, w, c = img.shape
    report = {}
    best = None
    for chans, plane, nch in ((slice(0, 3), 'rgb', 3), (slice(3, 4), 'a', 1)):
        if c < 4 and plane == 'a':
            report['a'] = 'channel-absent'
            continue
        bits = (img[:, :, chans].reshape(-1) & 1).astype(np.uint8)
        buf = _bits_to(bits[: (len(bits) // 8) * 8])
        obj, flags, k, n, why = _read_stream(buf)
        report[plane] = f'{why} ({k}/{n} copies intact)' if n else why
        if obj is not None and best is None:
            best = (obj, flags)
    if best is None:
        return None, report
    raw, flags = best
    if flags & F_DEFLATE:
        raw = zlib.decompress(raw)
    obj = json.loads(raw.decode())
    shader = obj.pop('_shader', None)
    return (obj, shader), report

# ---------------------------------------------------------------------------
# MEASURED BEHAVIOUR (prototype, 256^2 unless stated)
#
# ROUND TRIP
#   64^2 / 128^2 / 256^2 / 1024^2 : all exact, MAX PIXEL DELTA 1 (imperceptible)
#   config-only payload: 596B JSON -> 382B deflated
#   copies that fit:  64^2 -> 3    128^2 -> 15    256^2 -> 61    1024^2 -> 978
#
# FULL SHADER SOURCE EMBEDDED (principia-ii frag.glsl, 11,367B -> 2,969B deflated)
#   128^2 -> 1 copy    256^2 -> 7    512^2 -> 29    1024^2 -> 116
#   So an image from 128^2 up can carry the CODE THAT COLOURED IT, not just a hash.
#
# DEGRADATION
#   alpha flattened to 255      RECOVERED from RGB alone (61/61) — systematic, as designed
#   saved + reloaded as PNG     RECOVERED (61/61)
#   top 10% of rows overwritten RECOVERED (55/61)
#   top 50%                     RECOVERED (30/61)
#   top 90%                     RECOVERED (6/61)
#   top 97%                     RECOVERED (1/61)   <- one surviving copy is enough
#   no payload present          reported 'no-magic' — DISTINGUISHABLE from corruption
#   JPEG q95                    lost, reported. Expected: JPEG has no LSB plane.
#
# TWO HONEST LIMITATIONS
#
#   1. REPETITION PROTECTS AGAINST LOCALISED DAMAGE, NOT UNIFORM NOISE.
#      1% of RGB LSBs flipped uniformly -> 0/61 copies intact, because a 382B payload
#      spans 3056 bits and 1% hits every copy. Localised damage is the realistic threat
#      (crop, overlay, watermark, corrupt block) and repetition handles it to 97%.
#      Uniform LSB noise does not happen to a losslessly-stored PNG. If it ever needed
#      to survive that, the fix is Reed-Solomon WITHIN each copy, not more copies.
#
#   2. CROPPING BREAKS IT. Copies are laid out in raster order, so a crop takes a
#      column-slice of every record rather than a subset of whole records. Fixable by
#      tiling the payload in 2-D blocks so a crop keeps whole copies — worth doing if
#      cropped screenshots are a real sharing path.
#
# DESIGN NOTES THAT MATTER
#   - The header is repeated before EVERY copy. The first version had one header at the
#     front, and 1% corruption orphaned the whole stream even though copies survived.
#     A single point of failure in the recovery path is still a single point of failure.
#   - Failure modes are DISTINGUISHED: 'no-magic' (no state) vs 'all-headers-corrupt' vs
#     'all-copies-corrupt' (state present, unrecoverable). Silently-wrong is the failure
#     mode this project exists to eliminate.
#   - Presets should encode as an ENUM ID, not as source. Only genuinely custom nodes
#     ship their source, or every default render carries 3KB it already has — and an old
#     embedded copy would override a fixed one.
#   - Include the BUILD HASH and refuse to recreate silently across a version where the
#     decoder changed. A slice config is only meaningful relative to a decoder.
#   - Watermarking must be OFF for figure export. A scientific figure should be exactly
#     the pixels the renderer produced.
