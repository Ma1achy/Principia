"""Where the title's letters sit inside its frame, and the margins that line them up with the README banner.

The 2D title's frame is 1600 px wide and carries margins for the orbits' glow: the P's stem starts 65 px in and
"Planar Three-Body Problem" ends 42 px before the right edge. The README shows the title and the banner (hero.jpg)
one above the other, so the banner is narrowed to the lettering. That only lines up if the title's two insets
are equal, so the title is padded on the side that has less margin (never cropped: the glow reaches the edge),
and the banner gets width = (frame width - 2 inset) / frame width.

    python3 titlelayout.py            # prints the numbers
    python3 titlelayout.py OUT_DIR    # also writes the two still SVGs (the reduced-motion title)
"""
import json, os, re, sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, 'python', 'data')
DOCS = os.path.join(os.path.dirname(os.path.dirname(HERE)), 'docs', 'readme')
FRAME_W = 1600
PAD_LEFT = 12                        # margin of the page colour round the page-colour variants (see build.py)
PAD_TB = 12


# Where the P's stem and the end of the subtitle look to sit in the 1600 px frame, read from a rendered frame with
# readme_align.measure (the same reading the alignment check uses). The polygon extents, from geometry(), are 64.93 and
# 1557.95: anti-aliasing makes the visible edges a fraction of a pixel inside them.
OPTICAL = (64.63, 1557.76)


def geometry():
    """The polygon extents (x of the P's stem, x of the subtitle's right edge) in the 1600 px frame, for reference."""
    k, cx, cy = json.load(open(os.path.join(DATA, 'word_frames.json')))['WordFlatSub']
    p = min(x for x, y in json.load(open(os.path.join(DATA, 'pgeo.json')))['out'])
    s = max(x for q in json.load(open(os.path.join(DATA, 'sub_loops.json'))) for x, y in q['pts'])
    return cx + k * p, cx + k * s


def layout():
    """The pads (left, top, right, bottom) in frame pixels, the inset, the canvas width and the banner width."""
    xp, xs = OPTICAL
    a = PAD_LEFT + xp                                       # inset on the left
    pad_r = round(a - (FRAME_W - xs))                       # the right pad that makes the right inset equal
    b = pad_r + (FRAME_W - xs)
    w = FRAME_W + PAD_LEFT + pad_r
    inner = w - a - b
    return {'pad': (PAD_LEFT, PAD_TB, pad_r, PAD_TB), 'frame_w': w, 'inset_left': a, 'inset_right': b,
            'inset_fraction': (a + b) / 2 / w, 'hero_percent': round(100 * inner / w, 1), 'exact_percent': 100 * inner / w}


def still_svgs(out_dir):
    """The reduced-motion titles: docs/readme/title-{light,dark}.svg with a viewBox widened on the sides so that
    their lettering has the same inset fraction as the animated title. The docs files are not changed."""
    from svgpathtools import parse_path
    phi = layout()['inset_fraction']
    os.makedirs(out_dir, exist_ok=True)
    made = []
    for theme in ('light', 'dark'):
        s = open(os.path.join(DOCS, f'title-{theme}.svg')).read()
        paths = re.findall(r'<path [^>]*\bd="([^"]+)"', s)
        word = paths[0]
        x0 = parse_path(word[:word.index('M183.492')]).bbox()[0]          # the P is the first glyph
        x1 = max(parse_path(d).bbox()[1] for d in paths)                  # the subtitle ends furthest right
        w = (x1 - x0) / (1 - 2 * phi)
        vx = x0 - phi * w
        hdr = re.search(r'<svg[^>]*>', s).group(0)
        vb = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', hdr)
        h = float(vb.group(2))
        new = re.sub(r'viewBox="[^"]*"', f'viewBox="{vx:.3f} 0 {w:.3f} {h:g}"', hdr)
        new = re.sub(r'\swidth="[^"]*"', f' width="{w:.0f}"', new, count=1)
        p = os.path.join(out_dir, f'principia-title-still-{theme}.svg')
        open(p, 'w').write(s.replace(hdr, new))
        made.append(p)
    return made


if __name__ == '__main__':
    xp, xs = geometry()
    print(f'polygon extents: P stem at {xp:.2f}, subtitle ends at {xs:.2f}; as seen: {OPTICAL} of {FRAME_W}')
    print(json.dumps(layout(), indent=1))
    if len(sys.argv) > 1:
        print(still_svgs(sys.argv[1]))
