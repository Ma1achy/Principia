#!/usr/bin/env python3
"""Rebuilds the Principia brand pictures from the committed sources.

    pip install -r brand/code/requirements.txt && python3 -m playwright install chromium
    python3 brand/code/build.py --out /path/to/rebuild            # everything
    python3 brand/code/build.py --out /path/to/rebuild --part stills

Needs: Python 3.13, ffmpeg 6.1 with libx264 and libvpx (ffmpeg -version is recorded in build-info.json),
and the Chromium that requirements.txt's playwright installs. It writes only inside --out and never
deletes anything; it refuses an --out that is inside (or contains) the brand/ folder it reads.

The parts run in this order, each reading what the earlier ones wrote into --out:
  sim      the orbit integrals (sim.py, cat.py) and the Dragonfly path (dragon.py)
                                                              -> code/python/data/traj.npz, catalog.npz, dragon_adapt.json
  boards   the ten design-canvas boards (final.py, wordmarks.py)  -> code/boards/
  web      the self-contained scripts and demo pages          -> code/web/
  stills   Chrome masters, then every icon pack               -> icons/ (except animated/)
  videos   every frame of the 36 s loops, then MP4/WebM/GIF   -> icons/animated/, titles/
  social   the social-preview images                          -> social/out/
Frames of the videos are resumable: an interrupted run continues where it stopped.
Compare the result with the committed folder:  python3 brand/code/compare.py brand <out>
"""
import argparse, json, os, platform, shutil, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
BRAND = os.path.dirname(HERE)
REPO = os.path.dirname(BRAND)
PY = os.path.join(HERE, 'python')
sys.path.insert(0, HERE)

PARTS = ['sim', 'boards', 'web', 'stills', 'videos', 'social']
OPTIONAL_PARTS = ['explore']      # slow, and not needed for the pictures: the placement searches, run for the record
# The placement searches. Each chain runs in its own fresh copy of the committed data, its scripts one after another
# (hole4 to hole7 read what hole3 writes, clear2 and clear3 refine clear.json); chains run side by side.
EXPLORE = [['dragon.py'], ['dragon2.py'], ['dragon3.py'], ['sorb.py dragonfly'], ['sorb.py butterfly1'], ['sorb.py butterfly2'],
           ['sorb.py goggles'], ['hole.py'], ['hole2.py'], ['hole3.py', 'hole4.py', 'hole5.py', 'hole6.py', 'hole7.py'],
           ['clear.py', 'clear2.py', 'clear3.py']]
# The ten boards whose output is committed in code/boards (final.py also writes exploration boards).
BOARDS = ['Dragonfly', 'DragonflyNight', 'Word3D', 'Word3DSub', 'WordFlat', 'WordFlatNight', 'WordFlatSub',
          'WordFlatSubNight', 'WordMixed', 'WordMixedSub']
# The plan is unclear here; chose the 1024 px crop (x, y, size on the 2048 render) that reproduces the
# committed 1024 stills to the pixel, because the code that made them was not kept.
CROP = (80, 117, 1802, 1839)
# (variant, folder, file stem, frames per second, GIF frames per second, GIF width, theme)
VIDEOS = [('icon-light', 'icons/animated', 'principia-icon-light', 24, 12, 400, 'light'),
          ('icon-dark', 'icons/animated', 'principia-icon-dark', 24, 12, 400, 'dark'),
          ('title-2d-light', 'titles', 'principia-title-2d-light', 30, 15, 800, 'light'),
          ('title-2d-dark', 'titles', 'principia-title-2d-dark', 30, 15, 800, 'dark'),
          ('title-2d-subtitle-light', 'titles', 'principia-title-2d-subtitle-light', 30, 15, 800, 'light'),
          ('title-2d-subtitle-dark', 'titles', 'principia-title-2d-subtitle-dark', 30, 15, 800, 'dark')]


WEBP_WIDTH, WEBP_LIMIT = 1200, 4_000_000
# Page-colour variants of the subtitle title for GitHub's default dark and light pages: the glow and the
# ink-style mixing are computed on that colour (the runtime's backdrop option), not on the paper colour.
# The glow reaches the picture's own edge in some frames (up to 10 levels on dark), so the page-colour variants carry
# a margin of the page colour: the outer border is then exactly that colour, and Lanczos (radius 4 px at 1200 px) cannot reach it.
GITHUB_PAD = 12
GITHUB = [('github-dark', 'title-2d-subtitle-dark', '#0d1117'), ('github-light', 'title-2d-subtitle-light', '#ffffff')]


def log(*a):
    print(time.strftime('%H:%M:%S'), *a, flush=True)


def run(cmd, cwd, **kw):
    log('$', ' '.join(cmd), f'(in {os.path.basename(cwd)})')
    subprocess.run(cmd, cwd=cwd, check=True, **kw)


def check_out(out):
    out = os.path.realpath(out)
    b = os.path.realpath(BRAND)
    if out == b or out.startswith(b + os.sep) or b.startswith(out + os.sep):
        sys.exit(f'refusing --out {out}: it overlaps the committed brand folder {b}')
    return out


def work_py(out):
    """A private copy of code/python and its data/: the scripts read and write relative to it."""
    wp = os.path.join(out, 'work', 'py')
    if not os.path.isdir(wp):
        shutil.copytree(PY, wp, ignore=shutil.ignore_patterns('__pycache__'))
        os.makedirs(os.path.join(wp, 'p4'), exist_ok=True)
    return wp


def need(path, part):
    if not os.path.exists(path):
        sys.exit(f'{path} is missing: run --part {part} first')


def part_sim(out):
    wp = work_py(out)
    run([sys.executable, 'sim.py'], wp)
    run([sys.executable, 'cat.py'], wp)
    data_to_cwd(wp)
    run([sys.executable, 'dragon.py'], wp, stdout=subprocess.DEVNULL)      # dragon_P.json -> dragon_adapt.json
    # these scripts write to the working folder; keep the results beside the committed data
    for f in ('traj.npz', 'catalog.npz', 'dragon_adapt.json'):
        os.makedirs(os.path.join(out, 'code/python/data'), exist_ok=True)
        shutil.copy(os.path.join(wp, f), os.path.join(out, 'code/python/data', f))
        shutil.copy(os.path.join(wp, f), os.path.join(wp, 'data', f))


def data_to_cwd(wp):
    """The scripts open data files from their working folder."""
    d = os.path.join(wp, 'data')
    for f in os.listdir(d):
        shutil.copy(os.path.join(d, f), os.path.join(wp, f))


def part_boards(out):
    wp = work_py(out)
    data_to_cwd(wp)
    run([sys.executable, 'final.py'], wp, stdout=subprocess.DEVNULL)
    run([sys.executable, 'wordmarks.py'], wp, stdout=subprocess.DEVNULL)
    dst = os.path.join(out, 'code/boards')
    os.makedirs(dst, exist_ok=True)
    for b in BOARDS:
        shutil.copy(os.path.join(wp, 'p4', b + '.dc.html'), dst)


def part_web(out):
    wp = work_py(out)
    need(os.path.join(wp, 'p4', 'Dragonfly.dc.html'), 'boards')
    web = os.path.join(out, 'code/web')
    os.makedirs(web, exist_ok=True)
    run([sys.executable, 'build_runtime.py', web], wp, stdout=subprocess.DEVNULL)


def render_dir(out):
    import render
    web = os.path.join(out, 'code/web')
    need(os.path.join(web, 'principia-icon-light.js'), 'web')
    wd = os.path.join(out, 'work', 'render')
    render.make_render_dir(wd, web)
    return wd


def part_stills(out):
    import render
    from PIL import Image
    wd = render_dir(out)
    masters = os.path.join(out, 'work', 'masters')
    jobs = [(f'icon-{v}', f'{masters}/icon-{v}-{"small-" if w != 1 else ""}2048.png', 2048, 17, w)
            for v in ('light', 'dark') for w in (1, 1.6)]
    render.stills(wd, jobs)
    for v in ('light', 'dark'):
        big = Image.open(f'{masters}/icon-{v}-2048.png').convert('RGBA')
        sm = Image.open(f'{masters}/icon-{v}-small-2048.png').convert('RGBA')
        box = (CROP[0], CROP[1], CROP[2], CROP[3])
        big.crop(box).resize((1024, 1024), Image.LANCZOS).save(f'{masters}/icon-{v}-1024.png')
        sm.crop(box).resize((1024, 1024), Image.LANCZOS).save(f'{masters}/icon-{v}-small-1024.png')
    run([sys.executable, os.path.join(PY, 'packs.py'), masters, os.path.join(out, 'icons')], PY)


def png_size(path):
    from PIL import Image
    return Image.open(path).size


def record_webp(out, name, q, size):
    p = os.path.join(out, 'webp-settings.json')
    d = json.load(open(p)) if os.path.exists(p) else {}
    d[name] = {'quality': q, 'alpha_quality': q, 'method': 4, 'width': WEBP_WIDTH, 'fps': 15, 'bytes': size}
    json.dump(d, open(p, 'w'), indent=1)
    log(name, d[name])


def _render_job(args):
    wd, v, fps, frames, backdrop, pad = args
    import render
    render.frames(wd, v, fps, frames, backdrop=backdrop, pad=pad)
    return frames


def _encode_job(args):
    import encode
    kind, a = args
    if kind == 'webp':
        frames, fps, dst = a
        return dst, encode.webp(frames, fps, 15, WEBP_WIDTH, dst, limit=WEBP_LIMIT)
    getattr(encode, kind)(*a)
    return None, None


def part_videos(out, only=None, workers=None):
    """Frames first (one Chrome per variant, run side by side), then the encoders (also side by side)."""
    from concurrent.futures import ProcessPoolExecutor
    wd = render_dir(out)
    workers = workers or min(4, os.cpu_count() or 1)
    sel = lambda name: not only or any(o in name for o in only)
    render_jobs, encode_jobs, webps = [], [], []
    for v, folder, stem, fps, gfps, gw, theme in VIDEOS:
        if sel(v):
            render_jobs.append((wd, v, fps, os.path.join(out, 'work', 'frames', v), None, 0))
    for name, v, bd in GITHUB:
        if sel(name):
            render_jobs.append((wd, v, 30, os.path.join(out, 'work', 'frames', f'title-2d-subtitle-{name}'), bd, GITHUB_PAD))
    log('rendering frames:', [os.path.basename(j[3]) for j in render_jobs])
    with ProcessPoolExecutor(workers) as ex:
        list(ex.map(_render_job, render_jobs))
    for v, folder, stem, fps, gfps, gw, theme in VIDEOS:
        if not sel(v):
            continue
        frames = os.path.join(out, 'work', 'frames', v)
        n = int(round(36 * fps))
        missing = [i for i in range(n) if not os.path.exists(f'{frames}/f{i:05d}.png')]
        if missing:
            sys.exit(f'{v}: {len(missing)} frames missing')
        w, h = png_size(f'{frames}/f00000.png')
        dst = os.path.join(out, folder)
        os.makedirs(dst, exist_ok=True)
        encode_jobs += [('mp4', (frames, fps, w, h, theme, f'{dst}/{stem}.mp4')),
                        ('webm', (frames, fps, f'{dst}/{stem}-transparent.webm')),
                        ('gif', (frames, fps, w, h, theme, gfps, gw, f'{dst}/{stem}.gif'))]
        if 'subtitle' in v:
            encode_jobs.append(('webp', (frames, fps, f'{dst}/{stem}.webp')))
    for name, v, bd in GITHUB:
        if sel(name):
            frames = os.path.join(out, 'work', 'frames', f'title-2d-subtitle-{name}')
            os.makedirs(os.path.join(out, 'titles'), exist_ok=True)
            encode_jobs.append(('webp', (frames, 30, os.path.join(out, 'titles', f'principia-title-2d-subtitle-{name}.webp'))))
    log('encoding', len(encode_jobs), 'files')
    with ProcessPoolExecutor(workers) as ex:
        for dst, res in ex.map(_encode_job, encode_jobs):
            if dst:
                record_webp(out, os.path.basename(dst), *res)


def part_social(out):
    web = os.path.join(out, 'code/web')
    need(os.path.join(web, 'principia-title-2d-subtitle-dark.js'), 'web')
    dst = os.path.join(out, 'social/out')
    os.makedirs(dst, exist_ok=True)
    run([sys.executable, os.path.join(BRAND, 'social/make_social_preview.py'), '--web', web, '--out', dst,
         '--hero', os.path.join(REPO, 'docs/readme/hero.jpg')], BRAND)


def _explore_chain(args):
    out, chain, timeout = args
    import filecmp
    name = ' + '.join(chain)
    wd = os.path.join(out, 'work', 'explore', chain[0].replace(' ', '_').replace('.py', ''))
    if os.path.exists(wd):
        shutil.rmtree(wd)                       # our own scratch folder, made by an earlier run of this part
    shutil.copytree(PY, wd, ignore=shutil.ignore_patterns('__pycache__'))
    os.makedirs(os.path.join(wd, 'p4'), exist_ok=True)
    data_to_cwd(wd)
    steps = []
    for cmd in chain:
        t0 = time.time()
        try:
            p = subprocess.run([sys.executable] + cmd.split(), cwd=wd, capture_output=True, text=True, timeout=timeout)
            status = 'ran' if p.returncode == 0 else f'failed (exit {p.returncode})'
            tail = (p.stdout + p.stderr).strip().splitlines()[-6:]
        except subprocess.TimeoutExpired:
            status, tail = f'timed out after {timeout} s', []
        steps.append({'script': cmd, 'status': status, 'seconds': round(time.time() - t0), 'tail': tail})
        if status != 'ran':
            break
    wrote = {}
    for f in sorted(os.listdir(wd)):
        ref, cur = os.path.join(PY, 'data', f), os.path.join(wd, f)
        if f.endswith(('.json', '.npz')) and os.path.isfile(cur):
            if not os.path.exists(ref):
                wrote[f] = 'new file (not committed)'
            elif not filecmp.cmp(ref, cur, shallow=False):
                wrote[f] = 'differs from the committed file'
    return name, {'steps': steps, 'changed_or_new': wrote}


def part_explore(out, only=None, timeout=1800):
    """Runs each placement-search chain in its own fresh copy of the committed data and records what happened: whether
    each script ran, how long it took, and which data files the chain wrote that differ from the committed ones."""
    from concurrent.futures import ThreadPoolExecutor
    jobs = [(out, c, timeout) for c in EXPLORE if not only or any(o in ' '.join(c) for o in only)]
    res = {}
    with ThreadPoolExecutor(min(4, os.cpu_count() or 1)) as ex:
        for name, r in ex.map(_explore_chain, jobs):
            res[name] = r
            log(name, [(s['script'], s['status'], s['seconds']) for s in r['steps']], r['changed_or_new'])
    p = os.path.join(out, 'explore.json')
    old = json.load(open(p)) if os.path.exists(p) else {}
    old.update(res)
    json.dump(old, open(p, 'w'), indent=1)


def build_info(out):
    def cmd(c):
        try:
            return subprocess.check_output(c, text=True, stderr=subprocess.STDOUT).splitlines()[0]
        except Exception as e:
            return f'unavailable: {e}'
    info = {'python': platform.python_version(), 'platform': platform.platform(),
            'ffmpeg': cmd(['ffmpeg', '-version']), 'git': cmd(['git', '-C', REPO, 'rev-parse', 'HEAD'])}
    from importlib import metadata
    for p in ('numpy', 'scipy', 'matplotlib', 'svgpathtools', 'pillow', 'playwright'):
        try:
            info[p] = metadata.version(p)
        except Exception as e:
            info[p] = f'missing: {e}'
    try:
        from playwright.sync_api import sync_playwright
        with sync_playwright() as pw:
            b = pw.chromium.launch(); info['chromium'] = b.version; b.close()
    except Exception as e:
        info['chromium'] = f'unavailable: {e}'
    json.dump(info, open(os.path.join(out, 'build-info.json'), 'w'), indent=1)
    log('build-info.json', info)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0], formatter_class=argparse.RawDescriptionHelpFormatter,
                                 epilog=__doc__)
    ap.add_argument('--out', default=os.path.join(os.getcwd(), 'brand-rebuild'), help='where everything is written (default ./brand-rebuild)')
    ap.add_argument('--part', action='append', choices=PARTS + OPTIONAL_PARTS, help='run only this part (repeatable); default: all')
    ap.add_argument('--only', action='append', help='videos: only variants whose name contains this, e.g. icon-light')
    a = ap.parse_args()
    out = check_out(a.out)
    os.makedirs(out, exist_ok=True)
    t0 = time.time()
    build_info(out)
    for p in (a.part or PARTS):
        log('== part', p)
        if p in ('videos', 'explore'):
            globals()['part_' + p](out, a.only)
        else:
            globals()['part_' + p](out)
    log(f'done in {time.time() - t0:.0f} s; compare with: python3 {os.path.join(HERE, "compare.py")} {BRAND} {out}')


if __name__ == '__main__':
    main()
