# Superseded by ../render.py and ../build.py (part "videos"); kept as a thin wrapper.
# usage: frames.py WORK_RENDER_DIR VARIANT FPS OUT_FRAMES_DIR
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import render
wd, v, fps, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
render.frames(wd, v, fps, out)
