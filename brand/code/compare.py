#!/usr/bin/env python3
"""Decides, file by file, whether a rebuilt brand folder reproduces the committed one.

    python3 compare.py REFERENCE_BRAND REBUILT_BRAND [--only SUBSTRING] [--md FILE] [--json FILE]

Both arguments are brand/-shaped folders (icons/, titles/, code/web, code/boards).
Exit status 0 only if every reference file is rebuilt and within its criterion.

The criteria below are fixed before the first full rebuild is compared and are not tuned
afterwards. Chrome and encoder builds differ between machines, so exact bytes are expected only
for files written by plain Python; pictures are compared by decoded pixels.

Why these numbers. A "rebuilt properly" picture must be the same picture: same shapes, colours
and trail weights. Renderer and codec differences are antialiasing and quantisation: they move
single pixels by tens of levels along edges and everything by a few levels, and average out over
a block. A real change (a body in another colour, a trail 1.6 times as heavy, an orbit point
moved, the loop shifted) changes whole blocks by tens of levels. So each criterion bounds
 (a) the mean error over the whole frame        (the whole picture is the same),
 (b) the 99th percentile of the per-pixel error (antialiasing may spike, it may not be widespread),
 (c) the worst block mean                       (no local region is different),
and for transparent pictures the alpha channel separately. Colour is compared premultiplied by
alpha, because the colour under zero alpha is arbitrary. All numbers are levels out of 255.
"""
import argparse, fnmatch, json, os, struct, subprocess, sys
import numpy as np
from PIL import Image

# ---- the criteria -------------------------------------------------------------------------
CRITERIA = {
    # PNG / ICO / ICNS entries: one Chrome render, resampled with Lanczos.
    'still': {'mean': 1.0, 'p99': 16.0, 'block': 8.0, 'alpha_mean': 1.0},
    # MP4 and WebM: the same Chrome render, then H.264 crf 15 or VP9, so codec noise is added.
    'video': {'mean': 1.5, 'p99': 24.0, 'block': 10.0, 'alpha_mean': 1.5},
    # GIF: 256-colour dither adds per-pixel noise that a block average removes.
    'gif': {'mean': 2.0, 'p99': 40.0, 'block': 10.0, 'alpha_mean': 2.0},
}
VIDEO_DURATION_TOL = 0.0001      # seconds: frame count and fps are equal, so duration is
BYTE_EXACT = ('.json', '.html', '.js', '.xml', '.webmanifest', '.py')
BLOCK_PX = 16                    # block edge at 1024 px; scaled with image size, never below 4


def block_edge(w):
    return max(4, round(BLOCK_PX * w / 1024)) if w < 1024 else BLOCK_PX


# ---- pixel metrics ------------------------------------------------------------------------
def premult(a):
    """RGBA uint8 -> premultiplied RGBA int16 (colour scaled by alpha)."""
    a = a.astype(np.int32)
    al = a[..., 3:4]
    return np.concatenate([(a[..., :3] * al + 127) // 255, al], axis=-1).astype(np.int16)


def frame_diff(ref, new, edge=None):
    """Distribution of the difference between two RGBA frames of equal shape."""
    d = np.abs(premult(ref) - premult(new))
    pm = d.max(axis=-1)                              # per-pixel worst channel
    dm = d[..., :3].mean(axis=-1)                    # per-pixel mean over colour
    h, w = dm.shape
    e = edge or block_edge(w)
    hh, ww = h // e * e, w // e * e
    blocks = dm[:hh, :ww].reshape(hh // e, e, ww // e, e).mean(axis=(1, 3)) if hh and ww else dm
    return {'mean': float(dm.mean()), 'alpha_mean': float(d[..., 3].mean()),
            'hist': np.bincount(pm.ravel(), minlength=256), 'block': float(blocks.max()),
            'block_at': tuple(int(v) for v in np.unravel_index(blocks.argmax(), blocks.shape)),
            'pmax': int(pm.max())}


def hist_percentile(hist, q):
    c = np.cumsum(hist)
    return int(np.searchsorted(c, q / 100.0 * c[-1]))


def judge(kind, stats):
    """stats: mean, alpha_mean, p99, block (worst over frames). Returns (ok, ratio, why)."""
    crit, bad, ratio = CRITERIA[kind], [], 0.0
    for k in ('mean', 'p99', 'block', 'alpha_mean'):
        r = stats[k] / crit[k]
        ratio = max(ratio, r)
        if stats[k] > crit[k]:
            bad.append(f"{k} {stats[k]:.2f} > {crit[k]}")
    return not bad, ratio, '; '.join(bad)


def aggregate(frames):
    """frames: per-frame diff dicts -> file-level statistics, worst frame named."""
    hist = sum(f['hist'] for f in frames)
    worst = max(range(len(frames)), key=lambda i: frames[i]['block'])
    return {'mean': max(f['mean'] for f in frames), 'mean_avg': float(np.mean([f['mean'] for f in frames])),
            'alpha_mean': max(f['alpha_mean'] for f in frames), 'p50': hist_percentile(hist, 50),
            'p99': hist_percentile(hist, 99), 'p999': hist_percentile(hist, 99.9),
            'pmax': max(f['pmax'] for f in frames), 'block': frames[worst]['block'],
            'worst_frame': worst, 'frames': len(frames)}


# ---- still images -------------------------------------------------------------------------
def to_rgba(im):
    return np.asarray(im.convert('RGBA'))


def still_entries(path):
    """[(label, mode, RGBA array)] for a PNG, or every embedded size of an ICO or ICNS."""
    ext = os.path.splitext(path)[1].lower()
    im = Image.open(path)
    out = []
    if ext == '.ico':
        for s in sorted(im.ico.sizes()):
            e = im.ico.getimage(s)
            out.append((f'{s[0]}', e.mode, to_rgba(e)))
    elif ext == '.icns':
        for w, h, sc in sorted(im.info['sizes']):
            e = Image.open(path)
            e.size = (w, h)
            e.load(scale=sc)
            out.append((f'{w}@{sc}x', e.mode, to_rgba(e)))
    else:
        out.append(('', im.mode, to_rgba(im)))
    return out


def compare_still(ref, new):
    a, b = still_entries(ref), still_entries(new)
    if [(x[0], x[1], x[2].shape) for x in a] != [(x[0], x[1], x[2].shape) for x in b]:
        return {'ok': False, 'ratio': 99.0, 'why': f"format differs: {[(x[0], x[1], x[2].shape[:2]) for x in a]} vs "
                f"{[(x[0], x[1], x[2].shape[:2]) for x in b]}", 'stats': {}}
    fr = [frame_diff(x[2], y[2]) for x, y in zip(a, b)]
    st = aggregate(fr)
    ok, ratio, why = judge('still', st)
    st['entries'] = len(a)
    if len(a) > 1:                       # name the worst entry: the small sizes carry the heavier-trail cut
        st['worst_entry'] = a[st['worst_frame']][0]
    return {'ok': ok, 'ratio': ratio, 'why': why, 'stats': st}


# ---- video --------------------------------------------------------------------------------
def probe(path):
    """Container facts, read with libvpx for VP9 so the alpha plane is not hidden."""
    cmd = ['ffprobe', '-v', 'error', '-count_frames', '-select_streams', 'v:0', '-show_entries',
            'stream=codec_name,pix_fmt,width,height,r_frame_rate,nb_read_frames:stream_tags=alpha_mode:format=duration',
            '-of', 'json', path]
    j = json.loads(subprocess.check_output(cmd))
    s = j['streams'][0]
    num, den = s['r_frame_rate'].split('/')
    if path.endswith('.webm'):                     # ffprobe's own VP9 decoder reports yuv420p even with alpha
        err = subprocess.run(['ffmpeg', '-hide_banner', '-c:v', 'libvpx-vp9', '-i', path, '-frames:v', '1', '-f', 'null', '-'],
                             capture_output=True, text=True).stderr
        s['pix_fmt'] = 'yuva420p' if 'yuva420p' in err else 'yuv420p'
    return {'codec': s['codec_name'], 'pix_fmt': s['pix_fmt'], 'w': s['width'], 'h': s['height'],
            'fps': float(num) / float(den), 'frames': int(s['nb_read_frames']),
            'duration': float(j['format']['duration']), 'alpha_tag': s.get('tags', {}).get('alpha_mode')}


def decode(path, info=None):
    """Yield RGBA uint8 frames, in order, without holding the video in memory."""
    info = info or probe(path)
    cmd = ['ffmpeg', '-v', 'error', '-nostdin']
    if path.endswith('.webm'):
        cmd += ['-c:v', 'libvpx-vp9']            # the native VP9 decoder silently drops alpha
    cmd += ['-i', path, '-f', 'rawvideo', '-pix_fmt', 'rgba', '-']
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    n = info['w'] * info['h'] * 4
    try:
        while True:
            buf = p.stdout.read(n)
            if len(buf) < n:
                break
            yield np.frombuffer(buf, np.uint8).reshape(info['h'], info['w'], 4)
    finally:
        p.stdout.close(); p.wait()


def compare_video(ref, new, kind=None, ref_frames=None, new_frames=None):
    kind = kind or ('gif' if ref.endswith('.gif') else 'video')
    ia, ib = probe(ref), probe(new)
    problems = []
    for k in ('codec', 'pix_fmt', 'w', 'h', 'frames', 'alpha_tag'):
        if ia[k] != ib[k]:
            problems.append(f"{k} {ia[k]} vs {ib[k]}")
    if abs(ia['fps'] - ib['fps']) > 1e-6:
        problems.append(f"fps {ia['fps']} vs {ib['fps']}")
    if abs(ia['duration'] - ib['duration']) > VIDEO_DURATION_TOL:
        problems.append(f"duration {ia['duration']} vs {ib['duration']}")
    if problems:
        return {'ok': False, 'ratio': 99.0, 'why': 'format differs: ' + '; '.join(problems), 'stats': {'ref': ia, 'new': ib}}
    gen_a = ref_frames if ref_frames is not None else decode(ref, ia)
    gen_b = new_frames if new_frames is not None else decode(new, ib)
    fr = []
    for x, y in zip(gen_a, gen_b):
        fr.append(frame_diff(x, y))
    if len(fr) != ia['frames']:
        return {'ok': False, 'ratio': 99.0, 'why': f"decoded {len(fr)} frames, container says {ia['frames']}", 'stats': {}}
    st = aggregate(fr)
    st['alpha_present'] = ia['pix_fmt'] == 'yuva420p'
    ok, ratio, why = judge(kind, st)
    return {'ok': ok, 'ratio': ratio, 'why': why, 'stats': st}


def compare_bytes(ref, new):
    a, b = open(ref, 'rb').read(), open(new, 'rb').read()
    if a == b:
        return {'ok': True, 'ratio': 0.0, 'why': '', 'stats': {'bytes': len(a)}}
    first = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
    return {'ok': False, 'ratio': 99.0, 'why': f"bytes differ (sizes {len(a)} vs {len(b)}, first at {first})", 'stats': {}}


def compare_file(ref, new):
    ext = os.path.splitext(ref)[1].lower()
    if ext in ('.png', '.jpg', '.ico', '.icns'):
        return compare_still(ref, new)
    if ext in ('.mp4', '.webm', '.gif'):
        return compare_video(ref, new)
    return compare_bytes(ref, new)


# ---- the folders --------------------------------------------------------------------------
COMPARED = ('icons', 'titles', 'social/out', 'code/web', 'code/boards')


def list_files(root):
    out = []
    for sub in COMPARED:
        base = os.path.join(root, sub)
        for dp, _, fns in os.walk(base):
            for fn in fns:
                out.append(os.path.relpath(os.path.join(dp, fn), root))
    return sorted(out)


def compare_trees(ref_root, new_root, only=None, log=print):
    ref_files = [f for f in list_files(ref_root) if not only or only in f]
    new_files = set(list_files(new_root))
    rows = []
    for f in ref_files:
        if f not in new_files:
            rows.append({'file': f, 'ok': False, 'ratio': 99.0, 'why': 'missing from the rebuild', 'stats': {}})
            log(f'  MISSING {f}')
            continue
        try:
            r = compare_file(os.path.join(ref_root, f), os.path.join(new_root, f))
        except Exception as e:                      # a file that cannot be read is a failure, not a skip
            r = {'ok': False, 'ratio': 99.0, 'why': f'could not compare: {e!r}', 'stats': {}}
        r['file'] = f
        rows.append(r)
        log(f"  {'ok  ' if r['ok'] else 'FAIL'} {f}  {r['why']}")
    extra = sorted(f for f in new_files if f not in set(list_files(ref_root)) and (not only or only in f))
    rows.sort(key=lambda r: -r['ratio'])
    return rows, extra


def fmt_stats(s):
    if not s:
        return ''
    if 'bytes' in s:
        return f"identical, {s['bytes']} bytes"
    if 'ref' in s:
        return ''
    parts = [f"mean {s['mean']:.2f}", f"p50 {s['p50']}", f"p99 {s['p99']}", f"p99.9 {s['p999']}", f"max {s['pmax']}",
             f"block {s['block']:.1f}", f"alpha {s['alpha_mean']:.2f}"]
    if s.get('frames', 1) > 1 and 'entries' not in s:
        parts.append(f"worst frame {s['worst_frame']}/{s['frames']}, mean over frames {s['mean_avg']:.2f}")
    if 'worst_entry' in s:
        parts.append(f"worst size {s['worst_entry']}")
    return ', '.join(parts)


def markdown(rows, extra):
    out = ['| file | verdict | measured (levels of 255; worst entry or frame) | criterion ratio |', '|---|---|---|---|']
    for r in rows:
        out.append(f"| `{r['file']}` | {'within' if r['ok'] else 'NOT within: ' + r['why']} | {fmt_stats(r['stats'])} | {r['ratio']:.2f} |")
    if extra:
        out += ['', 'Rebuilt files with no committed counterpart: ' + ', '.join(f'`{e}`' for e in extra)]
    return '\n'.join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('reference'); ap.add_argument('rebuilt')
    ap.add_argument('--only'); ap.add_argument('--md'); ap.add_argument('--json')
    a = ap.parse_args(argv)
    rows, extra = compare_trees(a.reference, a.rebuilt, a.only)
    md = markdown(rows, extra)
    print(md)
    if a.md:
        open(a.md, 'w').write(md + '\n')
    if a.json:
        json.dump({'rows': rows, 'extra': extra}, open(a.json, 'w'), indent=1, default=lambda o: o.tolist() if hasattr(o, 'tolist') else str(o))
    bad = [r for r in rows if not r['ok']]
    print(f"\n{len(rows) - len(bad)} of {len(rows)} reference files within their criterion; {len(bad)} not.")
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
