# Superseded by ../render.py and ../build.py (part "stills"); kept as a thin wrapper.
# usage: render_masters.py WORK_RENDER_DIR OUT_MASTERS_DIR
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import render
wd, out = sys.argv[1], sys.argv[2]
render.stills(wd, [(f'icon-{v}', f'{out}/icon-{v}-{"small-" if w != 1 else ""}2048.png', 2048, 17, w)
                   for v in ('light', 'dark') for w in (1, 1.6)])
