#!/usr/bin/env python3
"""Fast checks on the committed brand files. Plain python3 (standard library only), no Chrome, no ffmpeg.

    python3 brand/code/checks.py

The full rebuild (build.py) is run by hand. These only catch the cheap breakages: the generated web scripts
drifting from their generator, a pack losing a file or a size, an absolute path creeping into a script, a
README naming a picture that is not there, a social JPG or WebP over its size limit.
Every check has a control: the same function run on something broken must report the breakage.
"""
import os, re, shutil, struct, subprocess, sys, tempfile, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
BRAND = os.path.dirname(HERE)
REPO = os.path.dirname(BRAND)
sys.path.insert(0, HERE)
BOARDS = ['Dragonfly', 'DragonflyNight', 'Word3D', 'Word3DSub', 'WordFlat', 'WordFlatNight', 'WordFlatSub',
          'WordFlatSubNight', 'WordMixed', 'WordMixedSub']


# ---- image headers without Pillow --------------------------------------------------------------
def png_size(path):
    with open(path, 'rb') as f:
        h = f.read(24)
    if h[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError(f'{path} is not a PNG')
    return struct.unpack('>II', h[16:24])


def ico_sizes(path):
    with open(path, 'rb') as f:
        d = f.read()
    n = struct.unpack('<H', d[4:6])[0]
    return sorted((d[6 + 16 * i] or 256) for i in range(n))


def icns_sizes(path):
    """Embedded images as (type, pixels): the type names the size and scale."""
    with open(path, 'rb') as f:
        d = f.read()
    out, i = [], 8
    while i < len(d):
        t, ln = d[i:i + 4].decode('latin1'), struct.unpack('>I', d[i + 4:i + 8])[0]
        out.append(t)
        i += ln
    return sorted(out)


def webp_canvas(path):
    with open(path, 'rb') as f:
        d = f.read(40)
    if d[:4] != b'RIFF' or d[8:12] != b'WEBP' or d[12:16] != b'VP8X':
        raise ValueError(f'{path} is not an extended (animated) WebP')
    return int.from_bytes(d[24:27], 'little') + 1, int.from_bytes(d[27:30], 'little') + 1


# ---- the checks, each a function of paths so a control can feed it something broken ---------------------
def web_matches_generator(brand, boards_dir=None):
    """Runs build_runtime.py on the committed boards and returns the files that differ from code/web."""
    boards_dir = boards_dir or os.path.join(brand, 'code/boards')
    with tempfile.TemporaryDirectory() as tmp:
        os.makedirs(os.path.join(tmp, 'p4'))
        for f in os.listdir(boards_dir):
            shutil.copy(os.path.join(boards_dir, f), os.path.join(tmp, 'p4', f))
        out = os.path.join(tmp, 'web')
        os.makedirs(out)
        subprocess.run([sys.executable, os.path.join(brand, 'code/python/build_runtime.py'), out], cwd=tmp, check=True,
                       stdout=subprocess.DEVNULL)
        bad = []
        for f in sorted(os.listdir(out)):
            a, b = os.path.join(out, f), os.path.join(brand, 'code/web', f)
            if not os.path.exists(b):
                bad.append(f)
                continue
            with open(a, 'rb') as fa, open(b, 'rb') as fb:
                if fa.read() != fb.read():
                    bad.append(f)
        return bad


def expected_pack():
    """The icon pack's layout: {relative path: (width, height) | list of ico sizes | None}. Written out here
    as the specification; packs.py is what has to agree with it."""
    e = {}
    for v in ('', '-dark'):
        for px in (16, 32, 48, 96):
            e[f'web/favicon{v}-{px}.png'] = (px, px)
        e[f'web/favicon{v}.ico'] = [16, 32, 48, 64]
        e[f'web/apple-touch-icon{v}.png'] = (180, 180)
        for px in (192, 512):
            e[f'web/icon{v}-{px}.png'] = (px, px)
        e[f'web/icon{v}-maskable-512.png'] = (512, 512)
        for base in (16, 32, 128, 256, 512):
            e[f'macos/Principia{v}.iconset/icon_{base}x{base}.png'] = (base, base)
            e[f'macos/Principia{v}.iconset/icon_{base}x{base}@2x.png'] = (base * 2, base * 2)
        e[f'macos/Principia{v}.icns'] = 'icns'
        e[f'windows/Principia{v}.ico'] = [16, 24, 32, 48, 64, 128, 256]
        for px in (44, 150, 310):
            e[f'windows/tiles/Square{px}x{px}Logo{v}.png'] = (px, px)
        for px in (16, 22, 24, 32, 48, 64, 128, 256, 512):
            e[f'linux/hicolor{v}/{px}x{px}/apps/principia.png'] = (px, px)
        n = '-night' if v else ''
        for d, px in (('mdpi', 48), ('hdpi', 72), ('xhdpi', 96), ('xxhdpi', 144), ('xxxhdpi', 192)):
            e[f'android/res/mipmap-{d}{n}/ic_launcher.png'] = (px, px)
            e[f'android/res/mipmap-{d}{n}/ic_launcher_foreground.png'] = (round(px * 108 / 48),) * 2
        e[f'source/principia-icon{v}-1024-transparent.png'] = (1024, 1024)
        e[f'source/principia-icon{v}-1024-on-background.png'] = (1024, 1024)
        e[f'source/principia-icon{v}-2048-uncropped.png'] = (2048, 2048)
        e[f'source/principia{"-icon" if not v else "-icon-dark"}-smallsizes-1024.png'.replace('principia-icon-icon', 'principia-icon')] = (1024, 1024)
    e['ios/AppIcon.appiconset/AppIcon-1024.png'] = (1024, 1024)
    e['ios/AppIcon.appiconset/AppIcon-1024-dark.png'] = (1024, 1024)
    for f in ('ios/AppIcon.appiconset/Contents.json', 'web/site.webmanifest', 'web/head-snippet.html',
              'android/res/values/ic_launcher_background.xml', 'android/res/values-night/ic_launcher_background.xml',
              'android/res/mipmap-anydpi-v26/ic_launcher.xml', 'android/res/mipmap-anydpi-v26-night/ic_launcher.xml'):
        e[f] = None
    return e


def pack_problems(icons_dir):
    """Missing files, wrong sizes and unexpected files (the animated/ folder is not part of the packs)."""
    exp, bad = expected_pack(), []
    for rel, want in exp.items():
        p = os.path.join(icons_dir, rel)
        if not os.path.exists(p):
            bad.append(f'missing {rel}'); continue
        if isinstance(want, tuple) and png_size(p) != want:
            bad.append(f'{rel} is {png_size(p)}, want {want}')
        elif isinstance(want, list) and ico_sizes(p) != want:
            bad.append(f'{rel} holds {ico_sizes(p)}, want {want}')
        elif want == 'icns' and len(icns_sizes(p)) < 8:
            bad.append(f'{rel} holds only {icns_sizes(p)}')
    have = set()
    for dp, _, fns in os.walk(icons_dir):
        for fn in fns:
            have.add(os.path.relpath(os.path.join(dp, fn), icons_dir))
    bad += [f'unexpected {f}' for f in sorted(have - set(exp)) if not f.startswith('animated/')]
    return bad


ABS = re.compile(r'(?<![\w.:/-])(/home/|/tmp/|/usr/|/Users/|/mnt/|[A-Za-z]:\\\\)[\w.-]')


def absolute_paths(root):
    """Hard-coded absolute paths in scripts, pages and data under root. The boards and web scripts are
    generated and large, so only their first lines (comments and templates) are scanned."""
    bad = []
    for dp, dn, fns in os.walk(root):
        dn[:] = [d for d in dn if d not in ('__pycache__', 'out')]
        for fn in fns:
            if not fn.endswith(('.py', '.js', '.html', '.json', '.md', '.txt')):
                continue
            p = os.path.join(dp, fn)
            if os.path.abspath(p) == os.path.abspath(__file__):
                continue
            for n, line in enumerate(open(p, errors='replace'), 1):
                if len(line) > 2000 or line.startswith('#!'):
                    continue
                m = ABS.search(line)
                if m:
                    bad.append(f'{os.path.relpath(p, REPO)}:{n}: {m.group(0)}')
    return bad


def readme_paths(readme, base):
    """Every src, srcset and href to a relative file in an HTML or Markdown README, and every backticked file
    name in brand/README.md-style text, that does not exist (relative to base)."""
    txt = re.sub(r'```.*?```', '', open(readme).read(), flags=re.S)   # code samples show relative names on purpose
    refs = re.findall(r'(?:src|srcset|href)="([^"#]+)"', txt) + re.findall(r'\]\(([^)#\s]+)\)', txt)
    refs = [r for r in refs if not re.match(r'[a-z]+:|//|mailto:', r)]
    return sorted({r for r in refs if not os.path.exists(os.path.join(base, r))})


def tree_names(readme):
    """Folder and file names the brand README's tree block and file mentions rely on."""
    txt = open(readme).read()
    names = set(re.findall(r'`((?:icons|titles|code|social)/[\w./-]+)`', txt))
    return sorted(n for n in names if not os.path.exists(os.path.join(BRAND, n.rstrip('/'))))


def banner_percent(readme_text):
    """The width, in percent, the README gives the banner (hero.jpg)."""
    return float(re.search(r'hero\.jpg"[^>]*width="([\d.]+)%"', readme_text).group(1))


def size_problems(brand):
    bad = []
    so = os.path.join(brand, 'social/out')
    if os.path.isdir(so):
        for f in os.listdir(so):
            if f.endswith('.jpg') and os.path.getsize(os.path.join(so, f)) >= 1_000_000:
                bad.append(f'social/out/{f} is not under 1 MB')
    ti = os.path.join(brand, 'titles')
    for f in sorted(os.listdir(ti)) if os.path.isdir(ti) else []:
        if f.endswith('.webp'):
            p = os.path.join(ti, f)
            if os.path.getsize(p) > 4_000_000:
                bad.append(f'titles/{f} is over 4 MB')
            if webp_canvas(p)[0] < 1200:
                bad.append(f'titles/{f} is narrower than 1200 px')
    return bad


# ---- the tests: the real check, then its control ------------------------------------------------------
class Checks(unittest.TestCase):
    def test_web_scripts_match_their_generator(self):
        self.assertEqual(web_matches_generator(BRAND), [])

    def test_control_web_check_sees_a_changed_board(self):
        with tempfile.TemporaryDirectory() as tmp:
            for f in os.listdir(os.path.join(BRAND, 'code/boards')):
                shutil.copy(os.path.join(BRAND, 'code/boards', f), tmp)
            p = os.path.join(tmp, 'Dragonfly.dc.html')
            t = open(p).read().replace('0.2', '0.21', 1)
            open(p, 'w').write(t)
            self.assertIn('principia-icon-light.js', web_matches_generator(BRAND, tmp))

    def test_all_ten_boards_are_committed(self):
        for b in BOARDS:
            self.assertTrue(os.path.exists(os.path.join(BRAND, 'code/boards', b + '.dc.html')), b)

    def test_icon_packs_have_their_files_and_sizes(self):
        self.assertEqual(pack_problems(os.path.join(BRAND, 'icons')), [])

    def test_control_pack_check_sees_missing_and_wrong_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            shutil.copytree(os.path.join(BRAND, 'icons'), tmp, dirs_exist_ok=True, ignore=shutil.ignore_patterns('animated'))
            os.remove(os.path.join(tmp, 'web/favicon-32.png'))
            shutil.copy(os.path.join(tmp, 'web/favicon-16.png'), os.path.join(tmp, 'web/favicon-48.png'))
            open(os.path.join(tmp, 'stray.png'), 'wb').close()
            bad = pack_problems(tmp)
            self.assertIn('missing web/favicon-32.png', bad)
            self.assertTrue(any(b.startswith('web/favicon-48.png is') for b in bad))
            self.assertIn('unexpected stray.png', bad)

    def test_no_absolute_paths_in_the_scripts(self):
        self.assertEqual(absolute_paths(os.path.join(BRAND, 'code')) + absolute_paths(os.path.join(BRAND, 'social')), [])

    def test_control_absolute_path_check_sees_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            open(os.path.join(tmp, 'x.py'), 'w').write("OUT='/home/claude/pack'\n")
            open(os.path.join(tmp, 'y.py'), 'w').write("chrome='/usr/bin/google-chrome'\n")
            self.assertEqual(len(absolute_paths(tmp)), 2)

    def test_readmes_name_only_files_that_exist(self):
        self.assertEqual(readme_paths(os.path.join(REPO, 'README.md'), REPO), [])
        self.assertEqual(readme_paths(os.path.join(BRAND, 'README.md'), BRAND), [])
        self.assertEqual(tree_names(os.path.join(BRAND, 'README.md')), [])

    def test_control_readme_check_sees_a_missing_picture(self):
        with tempfile.TemporaryDirectory() as tmp:
            open(os.path.join(tmp, 'R.md'), 'w').write('<img src="nope.png"> <source srcset="docs/a.svg">')
            self.assertEqual(readme_paths(os.path.join(tmp, 'R.md'), tmp), ['docs/a.svg', 'nope.png'])

    def test_banner_width_matches_the_title_layout(self):
        import titlelayout
        self.assertEqual(banner_percent(open(os.path.join(REPO, 'README.md')).read()), titlelayout.layout()['hero_percent'])

    def test_control_banner_check_sees_a_wrong_width(self):
        import titlelayout
        old = '<img src="docs/readme/hero.jpg" alt="x" width="100%">'
        self.assertNotEqual(banner_percent(old), titlelayout.layout()['hero_percent'])

    def test_social_jpgs_and_webps_are_within_their_limits(self):
        self.assertEqual(size_problems(BRAND), [])

    def test_control_size_check_sees_an_oversized_jpg(self):
        with tempfile.TemporaryDirectory() as tmp:
            os.makedirs(os.path.join(tmp, 'social/out'))
            with open(os.path.join(tmp, 'social/out/a.jpg'), 'wb') as f:
                f.write(b'\0' * 1_000_000)
            self.assertEqual(size_problems(tmp), ['social/out/a.jpg is not under 1 MB'])


if __name__ == '__main__':
    unittest.main(verbosity=2)
