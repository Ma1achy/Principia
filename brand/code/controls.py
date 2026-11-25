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


def png_frames(d, perturb=None, limit=None):
    for k, n in enumerate(sorted(os.listdir(d))[:limit]):
        a = np.asarray(Image.open(os.path.join(d, n)).convert('RGBA')).copy()
        yield perturb(k, a) if perturb else a


def webp_controls(ref, rebuild):
    """WebP files have no committed counterpart: the transparent ones are judged against the committed WebM of
    the same loop, the page-colour ones against the frames they were made from. Each wrong picture is fed in as
    the reference side, which is the same comparison with the pictures the other way round."""
    t = os.path.join(rebuild, 'titles')
    wd, wl = (os.path.join(t, f'principia-title-2d-subtitle-{n}.webp') for n in ('dark', 'light'))
    webm = lambda n: os.path.join(ref, f'titles/principia-title-2d-subtitle-{n}-transparent.webm')
    record('right: transparent dark WebP against the committed dark WebM', True, C.compare_webp_to_frames(wd, C.decode(webm('dark')), 30))
    record('wrong: dark WebP against the light WebM (wrong theme)', False, C.compare_webp_to_frames(wd, C.decode(webm('light')), 30))
    frames = list(C.decode(webm('dark')))
    def a_opaque(k, a): a = a.copy(); a[..., 3] = 255; return a
    record('wrong: alpha dropped (reference forced opaque)', False, C.compare_webp_to_frames(wd, (a_opaque(0, f) for f in frames), 30))
    shift = 8 * 2                                   # half a second at 15 fps is 7.5 frames; 8 frames = 16 source frames
    record('wrong: loop shifted by half a second', False, C.compare_webp_to_frames(wd, iter(frames[shift:] + frames[:shift]), 30))
    record('wrong: truncated (only the first 18 s)', False, C.compare_webp_to_frames(wd, iter(frames[:540]), 30))
    swap = [f[..., [2, 1, 0, 3]] for f in frames[::30]]
    sw = [frames[i] if i % 30 else swap[i // 30] for i in range(len(frames))]
    record('wrong: red and blue swapped in every 30th frame', False, C.compare_webp_to_frames(wd, iter(sw), 30))
    del frames
    # page-colour variants: border and picture against their own rendered frames
    for n, col in (('dark', (13, 17, 23)), ('light', (255, 255, 255))):
        w = os.path.join(t, f'principia-title-2d-subtitle-github-{n}.webp')
        fd = os.path.join(rebuild, 'work/frames', f'title-2d-subtitle-github-{n}')
        if not (os.path.exists(w) and os.path.isdir(fd)):
            continue
        other = 'light' if n == 'dark' else 'dark'
        ocol = (255, 255, 255) if n == 'dark' else (13, 17, 23)
        fo = os.path.join(rebuild, 'work/frames', f'title-2d-subtitle-github-{other}')
        record(f'right: github-{n} WebP, border and picture', True, C.compare_page_colour(w, png_frames(fd), 30, col))
        record(f'wrong: github-{n} WebP judged as the {other} page colour', False, C.compare_page_colour(w, png_frames(fd), 30, ocol))
        if os.path.isdir(fo):
            record(f'wrong: github-{n} WebP against the {other} variant frames', False, C.compare_page_colour(w, png_frames(fo), 30, col))
        # a frame whose background is 2 levels off (more than the 1 level a lossy file may be off): must fail the border test
        def off(k, a, d=2):
            if k == 300:
                a = a.copy(); a[..., :3] = np.clip(a[..., :3].astype(int) + (d if n == 'dark' else -d), 0, 255).astype(np.uint8)
            return a
        record(f'wrong: one frame of github-{n} with its background 2 levels off', False,
               C.compare_page_colour(w, png_frames(fd, off), 30, col))
        # the border test itself, on the rendered frames: one level off passes (tolerance 1), two fails
        wst, _ = C.border_check([off(300, next(png_frames(fd)), 1)], col, 1)
        wst2, _ = C.border_check([off(300, next(png_frames(fd)), 2)], col, 1)
        RESULTS.append((f'border test: background 1 level off reads {wst}, 2 levels off reads {wst2}', True, wst <= 1 < wst2 or wst2 > 1, '', ''))


def patched_render_dir(rebuild, tmp, name, patch):
    """A copy of the rebuild's render folder whose icon-light script has been changed by patch(text)."""
    import render
    src = os.path.join(rebuild, 'work', 'render')
    dst = os.path.join(tmp, 'rd-' + name)
    render.make_render_dir(dst, src)
    p = os.path.join(dst, 'principia-icon-light.js')
    t = open(p).read()
    new = patch(t)
    assert new != t, f'patch {name} changed nothing'
    open(p, 'w').write(new)
    return dst


def swap_bodies(t):
    """Magenta and cyan bodies exchange their colour ramps."""
    import re
    m = re.search(r'const RAMP = (\[\[.*?\]\]),', t)
    ramp = eval(m.group(1))
    ramp[0], ramp[2] = ramp[2], ramp[0]
    return t.replace(m.group(1), str(ramp).replace("'", '"'))


def nudge_point(t):
    """One orbit point of the first body, the one nearest to t = 17 s into the loop, moved 0.15 (7 % of the
    P's cap height) in x. The points are the adaptive samples in the board's AD list."""
    import re
    m = re.search(r'\{ t: \[([^\]]*)\], p: \[([^\]]*)\] \}', t)
    ts = [float(x) for x in m.group(1).split(',')]
    ps = [float(x) for x in m.group(2).split(',')]
    i = min(range(len(ts)), key=lambda k: abs(ts[k] - 17 / 36))
    ps[3 * i] += 0.15
    return t.replace(m.group(2), ','.join(format(x, '.6g') for x in ps), 1)


def render_controls(ref, rebuild, tmp):
    """Rendered wrong pictures: needs Chrome and the rebuild's work/render folder."""
    import render
    light = os.path.join(ref, 'icons/source/principia-icon-2048-uncropped.png')
    wd = os.path.join(rebuild, 'work', 'render')
    arms = [('harmless: the same render again (Chrome is deterministic here)', wd, 'icon-light', 1),
            ('wrong: bodies colours swapped (magenta <-> cyan)', patched_render_dir(rebuild, tmp, 'swap', swap_bodies), 'icon-light', 1),
            ('wrong: trails 1.6x heavier', wd, 'icon-light', 1.6),
            ('wrong: one orbit point moved by 0.15', patched_render_dir(rebuild, tmp, 'nudge', nudge_point), 'icon-light', 1),
            ('wrong: dark palette for the light file', wd, 'icon-dark', 1)]
    for i, (name, d, v, w) in enumerate(arms):
        out = f'{tmp}/ctl{i}.png'
        render.stills(d, [(v, out, 2048, 17, w)])
        record(name, name.startswith('harmless'), C.compare_still(light, out))


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
        webp_controls(a.reference, a.render)
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
