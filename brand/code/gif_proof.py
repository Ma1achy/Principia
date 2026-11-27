#!/usr/bin/env python3
"""Decides whether the rebuilt GIFs may replace the committed ones, by the criterion in evidence/gif-proof-criterion.md.

    python3 gif_proof.py REFERENCE_BRAND REBUILT_BRAND [--only NAME] [--control NAME] [--json FILE]

REFERENCE_BRAND is a brand/-shaped copy that holds the committed GIFs and the release's committed MP4s.
For each GIF: (a) rebuilt MP4 within compare.py's 'video' criterion of the committed MP4; (c) same frame count, size and fps as the committed GIF, and mean <= the 'gif' mean limit.
--control NAME also makes two wrong GIFs from the rebuilt MP4 of NAME (32-colour palette; shifted by one GIF frame),
which must each fail (c).
"""
import argparse, json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import compare, encode

# (stem, mp4 fps, gif fps, gif width, palette colours as build.py gives encode.gif)
GIFS = [('icons/animated/principia-icon-dark', 24, 12, 400, 192), ('icons/animated/principia-icon-light', 24, 12, 400, 192),
        ('titles/principia-title-2d-dark', 30, 15, 800, 256), ('titles/principia-title-2d-light', 30, 15, 800, 256),
        ('titles/principia-title-2d-subtitle-dark', 30, 15, 800, 256), ('titles/principia-title-2d-subtitle-light', 30, 15, 800, 256)]
STATS = ('mean', 'p99', 'block')


def measure(ref, new, stem):
    a = compare.compare_video(f'{ref}/{stem}.mp4', f'{new}/{stem}.mp4', kind='video')
    r = {'c': gif_test(ref, f'{new}/{stem}.gif', stem)}
    r['a'] = {'ok': a['ok'], 'why': a['why'], 'stats': a['stats']}
    r['ok'] = a['ok'] and r['c']['ok']
    return r


def gif_test(ref, gif, stem):
    """(c) for the GIF at `gif` against the committed one."""
    ia, ib = compare.probe(f'{ref}/{stem}.gif'), compare.probe(gif)
    c_bad = [f'{k} {ia[k]} vs {ib[k]}' for k in ('frames', 'w', 'h') if ia[k] != ib[k]]
    if abs(ia['fps'] - ib['fps']) > 1e-6:
        c_bad.append(f"fps {ia['fps']} vs {ib['fps']}")
    c_stats = {}
    if not c_bad:
        c = compare.compare_video(f'{ref}/{stem}.gif', gif, kind='gif')
        c_stats = c['stats']
        if 'mean' not in c_stats:
            c_bad.append(c['why'])
        elif c_stats['mean'] > compare.CRITERIA['gif']['mean']:
            c_bad.append(f"mean {c_stats['mean']:.2f} > {compare.CRITERIA['gif']['mean']}")
    return {'ok': not c_bad, 'why': '; '.join(c_bad), 'stats': {k: c_stats[k] for k in STATS} if 'mean' in c_stats else {},
            'frames': ib['frames'], 'size': f"{ib['w']}x{ib['h']}", 'fps': ib['fps'],
            'ref_frames': ia['frames'], 'ref_size': f"{ia['w']}x{ia['h']}", 'ref_fps': ia['fps']}


def ffmpeg(args):
    subprocess.run(['ffmpeg', '-v', 'error', '-nostdin', '-y'] + args, check=True)


def control_gifs(mp4, gfps, gw, colors, tmp):
    """From the rebuilt MP4, encode.py's own settings except for one change each."""
    shifted = os.path.join(tmp, 'shifted.gif')
    pal32 = os.path.join(tmp, 'pal32.gif')
    ffmpeg(['-i', mp4, '-filter_complex', encode.gif_filter(gfps, gw, 32), '-loop', '0', pal32])
    # the loop is seamless, so the shift wraps: frames 1 to N, then N+1 (= frame 0 of the next pass) cut by index
    n = 36 * gfps
    shift = encode.gif_filter(gfps, gw, colors).replace(f'fps={gfps},', f'fps={gfps},trim=start_frame=1:end_frame={n + 1},setpts=PTS-STARTPTS,', 1)
    ffmpeg(['-stream_loop', '1', '-i', mp4, '-filter_complex', shift, '-loop', '0', shifted])
    return {'32-colour global palette': pal32, 'shifted by one GIF frame': shifted}


def control(ref, new, stem, gfps, gw, colors):
    out = {}
    with tempfile.TemporaryDirectory() as tmp:
        for name, g in control_gifs(f'{new}/{stem}.mp4', gfps, gw, colors, tmp).items():
            out[name] = gif_test(ref, g, stem)
            out[name]['fails'] = not out[name]['ok']
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('reference'); ap.add_argument('rebuilt')
    ap.add_argument('--only'); ap.add_argument('--control'); ap.add_argument('--json')
    a = ap.parse_args()
    res = {}
    for stem, fps, gfps, gw, colors in GIFS:
        if a.only and a.only not in stem:
            continue
        if a.control:
            if a.control in stem:
                res[stem] = control(a.reference, a.rebuilt, stem, gfps, gw, colors)
        else:
            res[stem] = measure(a.reference, a.rebuilt, stem)
        print(stem, json.dumps(res.get(stem), default=float), flush=True)
    if a.json:
        json.dump(res, open(a.json, 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
