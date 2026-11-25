#!/usr/bin/env python3
"""Control arms for compare.py: each one is a wrong picture that the comparison must reject, plus
two harmless changes (codec noise, resampling noise) that it must accept.

    python3 controls.py REFERENCE_BRAND [--scratch DIR] [--render REBUILD_DIR]

Every wrong picture is made from the committed files themselves, so nothing here depends on a
rebuild. With --render, the arms that need Chrome (bodies' colours swapped, trails 1.6 times as
heavy, one orbit point moved, dark palette) are rendered from REBUILD_DIR/work/render and
compared with the committed still and with the committed video frames at the same times.
Exit status is 1 if any wrong picture passes or any harmless change fails: then compare.py is broken.
"""
import argparse, os, subprocess, sys, tempfile
import numpy as np
from PIL import Image
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import compare as C

RESULTS = []


def record(name, want_pass, r):
    got = r['ok']
    RESULTS.append((name, want_pass, got, r['why'] or 'within criterion', C.fmt_stats(r['stats'])))


def save_png(arr, path, mode='RGBA'):
    Image.fromarray(arr).convert(mode).save(path)


def still_controls(ref, tmp):
    src = os.path.join(ref, 'icons/source')
    light = os.path.join(src, 'principia-icon-1024-transparent.png')
    dark = os.path.join(src, 'principia-icon-dark-1024-transparent.png')
    small = os.path.join(src, 'principia-icon-smallsizes-1024.png')
    a = np.asarray(Image.open(light).convert('RGBA'))
    # harmless: +-3 levels of noise on colour, and a Lanczos round trip through half size
    rng = np.random.default_rng(1)
    noisy = a.copy().astype(np.int16)
    noisy[..., :3] += rng.integers(-3, 4, noisy[..., :3].shape).astype(np.int16)
    save_png(np.clip(noisy, 0, 255).astype(np.uint8), f'{tmp}/noise.png')
    record('harmless: colour noise of +-3 levels', True, C.compare_still(light, f'{tmp}/noise.png'))
    rt = Image.open(light).convert('RGBA').resize((512, 512), Image.LANCZOS).resize((1024, 1024), Image.LANCZOS)
    rt.save(f'{tmp}/roundtrip.png')
    record('harmless: Lanczos 1024 -> 512 -> 1024 (softer edges)', True, C.compare_still(light, f'{tmp}/roundtrip.png'))
    # wrong: dark palette for the light file
    record('wrong: dark picture for the light file', False, C.compare_still(light, dark))
    # wrong: trails 1.6 times as heavy (the committed small-sizes cut is exactly that)
    record('wrong: trails 1.6x heavier (smallsizes cut for the 1024 still)', False, C.compare_still(light, small))
    # wrong: alpha dropped (kept as RGBA but opaque, and saved as RGB)
    b = a.copy(); b[..., 3] = 255
    save_png(b, f'{tmp}/opaque.png')
    record('wrong: alpha forced opaque (still RGBA)', False, C.compare_still(light, f'{tmp}/opaque.png'))
    Image.fromarray(a).convert('RGB').save(f'{tmp}/rgb.png')
    record('wrong: alpha channel dropped (saved as RGB)', False, C.compare_still(light, f'{tmp}/rgb.png'))
    # wrong: red and blue swapped on the bodies (a pixel-level stand-in for swapped body colours;
    # the rendered version is in the --render arms)
    c = a.copy(); c[..., [0, 2]] = c[..., [2, 0]]
    save_png(c, f'{tmp}/swapped.png')
    record('wrong: red and blue channels swapped', False, C.compare_still(light, f'{tmp}/swapped.png'))
    # wrong: one orbit point moved = a body (about 36 px across) drawn 24 px away
    d = a.copy(); ys, xs = np.nonzero((a[..., 0] > 200) & (a[..., 1] < 160) & (a[..., 3] > 200))
    if len(ys):
        y0, x0 = int(ys.mean()), int(xs.mean())
        patch = a[y0 - 20:y0 + 20, x0 - 20:x0 + 20].copy()
        d[y0 - 20:y0 + 20, x0 - 20:x0 + 20] = 0
        d[y0 - 20:y0 + 20, x0 + 4:x0 + 44] = patch
        save_png(d, f'{tmp}/moved.png')
        record('wrong: a 40 px patch of the picture moved 24 px', False, C.compare_still(light, f'{tmp}/moved.png'))
    # ICO / ICNS: the set of sizes is part of the file
    ico = os.path.join(ref, 'icons/windows/Principia.ico')
    ico_dark = os.path.join(ref, 'icons/windows/Principia-dark.ico')
    record('wrong: dark .ico for the light .ico', False, C.compare_still(ico, ico_dark))
    big = Image.open(light).convert('RGBA')
    big.save(f'{tmp}/fewer.ico', sizes=[(16, 16), (32, 32), (256, 256)])
    record('wrong: .ico with missing sizes', False, C.compare_still(ico, f'{tmp}/fewer.ico'))
    icns = os.path.join(ref, 'icons/macos/Principia.icns')
    record('wrong: dark .icns for the light .icns', False, C.compare_still(icns, os.path.join(ref, 'icons/macos/Principia-dark.icns')))
    record('right: a file against itself', True, C.compare_still(icns, icns))


def video_controls(ref, tmp, quick):
    mp4 = os.path.join(ref, 'icons/animated/principia-icon-light.mp4')
    mp4d = os.path.join(ref, 'icons/animated/principia-icon-dark.mp4')
    webm = os.path.join(ref, 'icons/animated/principia-icon-light-transparent.webm')
    info = C.probe(mp4)
    fps = info['fps']
    # harmless: the same frames through x264 at crf 18 instead of 15
    subprocess.check_call(['ffmpeg', '-v', 'error', '-y', '-i', mp4, '-c:v', 'libx264', '-crf', '18', '-preset', 'veryfast',
                           '-pix_fmt', 'yuv420p', f'{tmp}/crf18.mp4'])
    record('harmless: committed MP4 re-encoded at crf 18', True, C.compare_video(mp4, f'{tmp}/crf18.mp4'))
    # wrong: dark video for the light one
    record('wrong: dark video for the light video', False, C.compare_video(mp4, mp4d))
    # wrong: loop shifted by half a second
    shift = round(0.5 * fps)
    frames = list(C.decode(mp4, info))
    rolled = iter(frames[shift:] + frames[:shift])
    record('wrong: loop shifted by half a second', False, C.compare_video(mp4, mp4, new_frames=rolled, ref_frames=iter(frames)))
    # wrong: truncated to 35 s (stream copy) and to 18 s
    for secs in (35, 18):
        subprocess.check_call(['ffmpeg', '-v', 'error', '-y', '-i', mp4, '-t', str(secs), '-c', 'copy', f'{tmp}/trunc{secs}.mp4'])
        record(f'wrong: truncated to {secs} s', False, C.compare_video(mp4, f'{tmp}/trunc{secs}.mp4'))
    # wrong: alpha dropped from the transparent WebM (flattened on black, re-encoded without alpha)
    subprocess.check_call(['ffmpeg', '-v', 'error', '-y', '-c:v', 'libvpx-vp9', '-i', webm, '-vf', 'format=rgb24',
                           '-c:v', 'libvpx-vp9', '-crf', '20', '-b:v', '0', '-pix_fmt', 'yuv420p', f'{tmp}/noalpha.webm'])
    record('wrong: alpha dropped from the transparent WebM', False, C.compare_video(webm, f'{tmp}/noalpha.webm'))
    # wrong: alpha forced opaque inside the decoded frames (same container, so only pixels differ)
    ref_frames = list(C.decode(webm)) if not quick else None
    if ref_frames:
        flat = [np.concatenate([f[..., :3], np.full_like(f[..., 3:], 255)], axis=-1) for f in ref_frames]
        record('wrong: alpha forced opaque in every frame', False, C.compare_video(webm, webm, new_frames=iter(flat), ref_frames=iter(ref_frames)))
    # harmless: the WebM against the MP4 is not a control (different backgrounds)


def render_controls(ref, rebuild, tmp):
    """Rendered wrong pictures: needs Chrome and the rebuild's work/render folder."""
    import render
    wd = os.path.join(rebuild, 'work', 'render')
    light = os.path.join(ref, 'icons/source/principia-icon-1024-transparent.png')
    arms = [('bodies colours swapped (magenta <-> cyan)', dict(v='icon-light', swap='1')),
            ('trails 1.6x heavier', dict(v='icon-light', weight='1.6')),
            ('one orbit point moved', dict(v='icon-light', nudge='1')),
            ('dark palette for the light file', dict(v='icon-dark')),
            ('harmless: the same render again (Chrome is deterministic?)', dict(v='icon-light'))]
    for name, q in arms:
        out = f'{tmp}/{q["v"]}-{abs(hash(name)) % 10**6}.png'
        render.still(wd, out, size=1024, t=17, **{k: v for k, v in q.items() if k != 'v'}, v=q['v'])
        record(('' if name.startswith('harmless') else 'wrong: ') + name, name.startswith('harmless'), C.compare_still(light, out))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('reference'); ap.add_argument('--scratch'); ap.add_argument('--render'); ap.add_argument('--quick', action='store_true')
    a = ap.parse_args()
    tmp = a.scratch or tempfile.mkdtemp(prefix='brand-controls-')
    os.makedirs(tmp, exist_ok=True)
    still_controls(a.reference, tmp)
    video_controls(a.reference, tmp, a.quick)
    if a.render:
        render_controls(a.reference, a.render, tmp)
    broken = 0
    print(f'{"control":<66} {"must":<7} {"got":<7} detail')
    for name, want, got, why, st in RESULTS:
        flag = '' if want == got else '   <-- COMPARISON BROKEN'
        broken += want != got
        print(f'{name:<66} {"pass" if want else "fail":<7} {"pass" if got else "fail":<7} {why}{flag}')
        print(f'{"":<66} {st}')
    print(f'\n{len(RESULTS) - broken} of {len(RESULTS)} controls behaved; {broken} did not.')
    return 1 if broken else 0


if __name__ == '__main__':
    sys.exit(main())
