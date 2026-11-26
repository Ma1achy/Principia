#!/usr/bin/env python3
"""Decides whether the rebuilt GIFs may replace the committed ones, by the criterion in evidence/gif-proof-criterion.md.

    python3 gif_proof.py REFERENCE_BRAND REBUILT_BRAND [--only NAME] [--control NAME] [--json FILE]

REFERENCE_BRAND is a brand/-shaped copy that holds the committed GIFs and the release's committed MP4s.
For each GIF: (a) rebuilt MP4 within compare.py's 'video' criterion of the committed MP4; (b) the rebuilt GIF is no
further from the rebuilt MP4 than the committed GIF is from the committed MP4 (mean, p99, block, no margin);
(c) same frame count, size and fps as the committed GIF, and mean <= the 'gif' mean limit.
--control NAME also makes two wrong GIFs from the rebuilt MP4 of NAME (32-colour palette; shifted by one GIF frame),
which must each fail (b) or (c).
"""
import argparse, json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import compare

GIFS = [('icons/animated/principia-icon-dark', 24, 12, 400), ('icons/animated/principia-icon-light', 24, 12, 400),
        ('titles/principia-title-2d-dark', 30, 15, 800), ('titles/principia-title-2d-light', 30, 15, 800)]
STATS = ('mean', 'p99', 'block')


def measure(ref, new, stem):
    a = compare.compare_video(f'{ref}/{stem}.mp4', f'{new}/{stem}.mp4', kind='video')
    committed = compare.gif_vs_own_mp4(f'{ref}/{stem}.gif', f'{ref}/{stem}.mp4')
    rebuilt = compare.gif_vs_own_mp4(f'{new}/{stem}.gif', f'{new}/{stem}.mp4')
    r = gif_tests(ref, new, stem, committed, rebuilt)
    r['a'] = {'ok': a['ok'], 'why': a['why'], 'stats': a['stats']}
    r['ok'] = a['ok'] and r['b']['ok'] and r['c']['ok']
    return r


def gif_tests(ref, new, stem, committed, rebuilt, new_gif=None):
    """(b) and (c) for the GIF at new_gif (default: the rebuilt one)."""
    gif = new_gif or f'{new}/{stem}.gif'
    b_bad = [f'{k} {rebuilt[k]:.2f} > {committed[k]:.2f}' for k in STATS if rebuilt[k] > committed[k]]
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
    return {'committed_vs_mp4': {k: committed[k] for k in STATS}, 'rebuilt_vs_mp4': {k: rebuilt[k] for k in STATS},
            'b': {'ok': not b_bad, 'why': '; '.join(b_bad)},
            'c': {'ok': not c_bad, 'why': '; '.join(c_bad), 'stats': {k: c_stats[k] for k in STATS} if 'mean' in c_stats else {},
                  'frames': ib['frames'], 'size': f"{ib['w']}x{ib['h']}", 'fps': ib['fps'],
                  'ref_frames': ia['frames'], 'ref_size': f"{ia['w']}x{ia['h']}", 'ref_fps': ia['fps']}}


def ffmpeg(args):
    subprocess.run(['ffmpeg', '-v', 'error', '-nostdin', '-y'] + args, check=True)


def control_gifs(mp4, fps, gfps, gw, tmp):
    """From the rebuilt MP4, the encoder's own settings except for one change each."""
    scale = f'fps={gfps},scale={gw}:-1:flags=lanczos'
    low = f'{scale},split[a][b];[a]palettegen=max_colors=32[p];[b][p]paletteuse'
    shifted = os.path.join(tmp, 'shifted.gif')
    pal32 = os.path.join(tmp, 'pal32.gif')
    ffmpeg(['-i', mp4, '-filter_complex', low, '-loop', '0', pal32])
    # the loop is seamless, so the shift wraps: frames 1 to N, then N+1 (= frame 0 of the next pass) cut by index
    n = 36 * gfps
    sh = f'fps={gfps},trim=start_frame=1:end_frame={n + 1},setpts=PTS-STARTPTS,scale={gw}:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1'
    ffmpeg(['-stream_loop', '1', '-i', mp4, '-filter_complex', sh, '-loop', '0', shifted])
    return {'32-colour global palette': pal32, 'shifted by one GIF frame': shifted}


def control(ref, new, stem, fps, gfps, gw):
    committed = compare.gif_vs_own_mp4(f'{ref}/{stem}.gif', f'{ref}/{stem}.mp4')
    out = {}
    with tempfile.TemporaryDirectory() as tmp:
        for name, g in control_gifs(f'{new}/{stem}.mp4', fps, gfps, gw, tmp).items():
            m = compare.gif_vs_own_mp4(g, f'{new}/{stem}.mp4')
            out[name] = gif_tests(ref, new, stem, committed, m, new_gif=g)
            out[name]['fails'] = not (out[name]['b']['ok'] and out[name]['c']['ok'])
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('reference'); ap.add_argument('rebuilt')
    ap.add_argument('--only'); ap.add_argument('--control'); ap.add_argument('--json')
    a = ap.parse_args()
    res = {}
    for stem, fps, gfps, gw in GIFS:
        if a.only and a.only not in stem:
            continue
        if a.control:
            if a.control in stem:
                res[stem] = control(a.reference, a.rebuilt, stem, fps, gfps, gw)
        else:
            res[stem] = measure(a.reference, a.rebuilt, stem)
        print(stem, json.dumps(res.get(stem), default=float), flush=True)
    if a.json:
        json.dump(res, open(a.json, 'w'), indent=1, default=float)


if __name__ == '__main__':
    main()
