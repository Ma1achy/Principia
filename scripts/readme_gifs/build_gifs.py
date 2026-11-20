#!/usr/bin/env python3
"""Render all three Principia explainer GIFs, light and dark, into ../out/.

    python3 build_gifs.py            # everything
    python3 build_gifs.py symmetry   # just one (symmetry | regtime | scaletime)
    python3 build_gifs.py --preview  # a few still frames instead of the GIFs

Needs: numpy, scipy, matplotlib, pillow (pip install -r requirements.txt).
ffmpeg is optional: if it is on the PATH the GIFs are re-encoded to about a third of the size.
"""
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.environ.get("PRINCIPIA_OUT", os.path.join(HERE, "..", "out"))
SCRIPTS = {
    "symmetry": ("fig_symmetry.py", "symmetry-reduction"),
    "regtime": ("fig_regtime.py", "regularised-time"),
    "scaletime": ("fig_scaletime.py", "scale-time"),
}


def shrink(path):
    """Re-encode one GIF through ffmpeg's palette filters; keep the result only if it is smaller."""
    if not shutil.which("ffmpeg"):
        return
    with tempfile.TemporaryDirectory() as tmp:
        pal, dst = os.path.join(tmp, "pal.png"), os.path.join(tmp, "out.gif")
        quiet = ["ffmpeg", "-y", "-loglevel", "error", "-i", path]
        subprocess.run(quiet + ["-vf", "palettegen=max_colors=32:stats_mode=full", pal], check=True)
        subprocess.run(quiet + ["-i", pal, "-lavfi", "paletteuse=dither=none:diff_mode=rectangle", dst], check=True)
        if os.path.getsize(dst) < os.path.getsize(path):
            shutil.move(dst, path)


def main(argv):
    preview = "--preview" in argv
    chosen = [a for a in argv if a in SCRIPTS] or list(SCRIPTS)
    os.makedirs(OUT, exist_ok=True)
    for theme in ("light", "dark"):
        for key in chosen:
            script, name = SCRIPTS[key]
            env = dict(os.environ, PRINCIPIA_THEME=theme, PRINCIPIA_OUT=OUT)
            print(f"[{theme}] {script} ...", flush=True)
            subprocess.run([sys.executable, os.path.join(HERE, script)] + (["preview"] if preview else []),
                           check=True, env=env)
            if not preview:
                gif = os.path.join(OUT, f"{name}{'-dark' if theme == 'dark' else ''}.gif")
                shrink(gif)
                print(f"    {os.path.basename(gif)}  {os.path.getsize(gif) / 1e6:.2f} MB", flush=True)
    print("done:", os.path.abspath(OUT))


if __name__ == "__main__":
    main(sys.argv[1:])
