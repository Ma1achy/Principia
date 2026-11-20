#!/usr/bin/env python3
"""Makes the compass's golden fixtures (R-451 B8; TASK-M6-33): renders of the reference
`docs/gui/reference/principia_nav_cube.html` at the compass's size with each case's parameters, and the independent
computation of the decoder's plane for the tilt cases.

The reference is run headless in Google Chrome, unchanged: the script copies it into a scratch directory with two
additions, a style that sizes its canvas to the compass's square (SIDE points) and hides the page around it, and a
script after the reference's own that sets `state` to the case (each displayed value equal to its target, so the
easing holds still), calls the reference's `resize()` and `draw()` once while recording every canvas call, and writes
the record into the page. `--dump-dom` reads the record back; `--screenshot` takes the render. Nothing of the
reference's drawing is reimplemented here: the record is its own canvas calls, and its own `P`, `crossSection` and
`planeSpec` for the corners and the polygon.

The decoder's plane is computed here apart from the code under test: `View::basis`'s construction (`u1 = cos t1 e_a +
sin t1 e_d`, `u2 = cos t2 e_b + sin t2 e_d`, turned by g), its normal `q1 x q2`, through z0's point 0, and the polygon
by intersecting the plane with the cube's twelve edges and ordering the crossings by angle round their centroid in the
orthonormalised (q1, q2).

Run from the repo root: `python3 -I fixtures/screenshot/01_main/compass/make_reference.py`. It writes
`reference/<run>.json` and `reference/<run>.png` for each run, and `decoder/<case>.json` for each tilt case.
"""

import hashlib
import json
import math
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
REFERENCE = os.path.join(ROOT, "docs", "gui", "reference", "principia_nav_cube.html")
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"

# The compass's square on 01_main at 2160 x 1350 pixels and 1.5 pixels per point, in points (`compass::square`).
SIDE = 172

# The mock's preset frame: Width e_a = z_α, Height e_b = z_β, Depth e_d = z_q3 (`manifold_view::AXES`, `SLICE_AXIS`).
NAMES = {"xName": "z_α", "yName": "z_β", "dName": "z_q3", "t1Name": "z_q3", "t2Name": "z_q3"}

# The reference runs: mode, depth (fraction of range), tau1 and tau2 in degrees.
RUNS = {
    "slice_+0.25": ("slice", 0.25, 0.0, 0.0),
    "slice_0": ("slice", 0.0, 0.0, 0.0),
    "tilt_0": ("tilt", 0.0, 0.0, 0.0),
    "tilt_30_0": ("tilt", 0.0, 30.0, 0.0),
    "tilt_45_45": ("tilt", 0.0, 45.0, 45.0),
}

# The tilt cases' decoder planes: (tau1, tau2, gamma) in degrees, z0 = 0 on the frame.
DECODER = {
    "mock_compass_cube_tilt_0": (0.0, 0.0, 0.0),
    "mock_compass_cube_tilt_rect": (30.0, 0.0, 0.0),
    "mock_compass_cube_tilt_hex": (45.0, 45.0, 0.0),
    # R-459's cases: the gesture's plane, the other angle at 0, in the frame (e_a, e_b, e_T): the same construction.
    "mock_compass_cube_third_axis": (0.0, 15.0, 0.0),
    "mock_compass_cube_badge": (20.0, 0.0, 0.0),
}

STYLE = """<style id="principia-golden">
h1, .sub, .controls, .note { display: none !important; }
html, body { margin: 0 !important; }
.wrap { padding: 0 !important; margin: 0 !important; max-width: none !important; }
.card { border: 0 !important; display: inline-block; }
canvas { width: %(side)dpx !important; height: %(side)dpx !important; }
</style>
"""

RECORD = """<pre id="principia-record"></pre>
<script id="principia-golden-record">
(function () {
  const run = %(run)s;
  Object.assign(state, run.names);
  state.anim = false;
  state.mode = run.mode;
  state.depth = state.depthT = run.depth;
  state.tau1 = state.tau1T = run.tau1 * Math.PI / 180;
  state.tau2 = state.tau2T = run.tau2 * Math.PI / 180;
  resize();
  const ops = [];
  let path = [], closed = false, clipDepth = 0, saved = [], recording = false;
  const wrap = (name, f) => { const real = ctx[name].bind(ctx); ctx[name] = (...a) => { if (recording) f(...a); return real(...a); }; };
  wrap("beginPath", () => { path = []; closed = false; });
  wrap("moveTo", (x, y) => path.push([x, y]));
  wrap("lineTo", (x, y) => path.push([x, y]));
  wrap("closePath", () => { closed = true; });
  wrap("save", () => saved.push(clipDepth));
  wrap("restore", () => { clipDepth = saved.pop(); });
  wrap("clip", () => { clipDepth += 1; });
  wrap("stroke", () => ops.push({ op: "stroke", pts: path.slice(), closed, style: String(ctx.strokeStyle), width: ctx.lineWidth, clipped: clipDepth > 0 }));
  wrap("fill", () => ops.push({ op: "fill", pts: path.slice(), style: String(ctx.fillStyle), alpha: ctx.globalAlpha }));
  wrap("fillText", (text, x, y) => ops.push({ op: "text", text, x, y, font: ctx.font }));
  recording = true;
  draw();
  recording = false;
  const corners = V.map(v => { const p = P(...v); return [p.X, p.Y]; });
  // The cube's edge drawn from screen point a to b: (1, 1, 1) and (−1, −1, −1) share the centre, so the edge is
  // found among EDGES, both ends matched.
  const at = (p, q) => Math.abs(p.X - q[0]) < 1e-6 && Math.abs(p.Y - q[1]) < 1e-6;
  const edge3 = (a, b) => { const e = EDGES.find(([u, v]) => at(P(...u), a) && at(P(...v), b)); return e ? [e[0], e[1]] : null; };
  const alpha = s => { const m = /rgba\\([^)]*,\\s*([0-9.]+)\\)/.exec(s); return m ? +m[1] : 1; };
  const spec = planeSpec();
  const poly = crossSection(spec.n, spec.p0);
  const pKey = poly.length ? poly.reduce((s, p) => s + p[0] + p[2], 0) / poly.length : 0;
  const items = [];
  for (const o of ops) {
    if (o.op === "stroke" && !o.clipped && !o.closed && o.pts.length === 2) {
      const e = edge3(o.pts[0], o.pts[1]);
      if (e) items.push({ edge: e, screen: o.pts, opacity: alpha(o.style), width: o.width });
    } else if (o.op === "fill") {
      items.push({ slice: pKey });
    }
  }
  const fills = ops.filter(o => o.op === "fill");
  const texts = ops.filter(o => o.op === "text");
  const size = o => +(/([0-9.]+)px/.exec(o.font) || [0, 0])[1];
  const record = {
    run: run.name,
    square: [cv.clientWidth, cv.clientHeight],
    s: S,
    centre: [CX, CY],
    corners,
    items,
    polygon: poly,
    polygon_screen: fills.length ? fills[0].pts : [],
    fill: fills.length > 0,
    grid: ops.filter(o => o.op === "stroke" && o.clipped).length,
    labels: texts.filter(o => size(o) === 11).map(o => ({ text: o.text, at: [o.x, o.y] })),
    numbers: texts.filter(o => size(o) === 9).map(o => ({ text: o.text, at: [o.x, o.y] })),
    readout: hud.innerText.split("\\n"),
  };
  document.getElementById("principia-record").textContent = JSON.stringify(record);
})();
</script>
"""


def chrome(args, profile):
    return subprocess.run(
        [CHROME, "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
         "--hide-scrollbars", "--force-device-scale-factor=1", "--user-data-dir=" + profile] + args,
        check=True, capture_output=True, text=True, timeout=120,
    )


def version():
    out = subprocess.run([CHROME, "--version"], capture_output=True, text=True, check=True)
    return out.stdout.strip()


def run_reference(name, mode, depth, tau1, tau2, html, scratch):
    run = {"name": name, "mode": mode, "depth": depth, "tau1": tau1, "tau2": tau2, "names": NAMES}
    page = html.replace("</head>", STYLE % {"side": SIDE} + "</head>", 1)
    page = page.replace("</body>", RECORD % {"run": json.dumps(run)} + "</body>", 1)
    path = os.path.join(scratch, name + ".html")
    with open(path, "w", encoding="utf-8") as f:
        f.write(page)
    profile = os.path.join(scratch, "profile")
    dom = chrome(["--dump-dom", "file://" + path], profile).stdout
    found = re.search(r'<pre id="principia-record">(.*?)</pre>', dom, re.S)
    if not found or not found.group(1):
        sys.exit("make_reference: no record from the reference for " + name)
    text = found.group(1).replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", '"').replace("&amp;", "&")
    record = json.loads(text)
    png = os.path.join(HERE, "reference", name + ".png")
    chrome(["--window-size=%d,%d" % (SIDE, SIDE), "--screenshot=" + png, "file://" + path], profile)
    return record


def decoder_plane(tau1, tau2, gamma):
    """View::basis's plane at (tau1, tau2, gamma) degrees, sigma 1, through z0's point 0, in the frame (e_a, e_b, e_d)."""
    t1, t2, g = (math.radians(x) for x in (tau1, tau2, gamma))
    u1 = (math.cos(t1), 0.0, math.sin(t1))
    u2 = (0.0, math.cos(t2), math.sin(t2))
    q1 = tuple(math.cos(g) * a + math.sin(g) * b for a, b in zip(u1, u2))
    q2 = tuple(math.cos(g) * b - math.sin(g) * a for a, b in zip(u1, u2))
    n = (q1[1] * q2[2] - q1[2] * q2[1], q1[2] * q2[0] - q1[0] * q2[2], q1[0] * q2[1] - q1[1] * q2[0])
    length = math.sqrt(sum(x * x for x in n))
    n = tuple(x / length for x in n)
    offset = 0.0
    corners = [(x, y, z) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    points = []
    for i, a in enumerate(corners):
        for b in corners[i + 1:]:
            if sum(abs(p - q) for p, q in zip(a, b)) != 2:
                continue
            da = sum(p * q for p, q in zip(n, a)) - offset
            db = sum(p * q for p, q in zip(n, b)) - offset
            if da == 0:
                points.append(a)
            if db == 0:
                points.append(b)
            if da * db < 0:
                t = da / (da - db)
                points.append(tuple(p + (q - p) * t for p, q in zip(a, b)))
    unique = []
    for p in points:
        if all(max(abs(x - y) for x, y in zip(p, q)) > 1e-9 for q in unique):
            unique.append(p)
    # Order by angle round the centroid in the orthonormalised (q1, q2).
    e1 = tuple(x / math.sqrt(sum(y * y for y in q1)) for x in q1)
    w = tuple(b - sum(x * y for x, y in zip(q2, e1)) * a for a, b in zip(e1, q2))
    e2 = tuple(x / math.sqrt(sum(y * y for y in w)) for x in w)
    c = tuple(sum(p[k] for p in unique) / len(unique) for k in range(3))
    unique.sort(key=lambda p: math.atan2(sum((p[k] - c[k]) * e2[k] for k in range(3)),
                                         sum((p[k] - c[k]) * e1[k] for k in range(3))))
    return {"normal": list(n), "offset": offset, "polygon": [list(p) for p in unique]}


def main():
    with open(REFERENCE, "rb") as f:
        raw = f.read()
    digest = hashlib.sha256(raw).hexdigest()
    html = raw.decode("utf-8")
    made = {
        "reference": "docs/gui/reference/principia_nav_cube.html",
        "sha256": digest,
        "browser": version(),
        "how": "make_reference.py: the reference run headless, its state set to the run, one draw() recorded",
        "square_points": SIDE,
    }
    os.makedirs(os.path.join(HERE, "reference"), exist_ok=True)
    os.makedirs(os.path.join(HERE, "decoder"), exist_ok=True)
    with tempfile.TemporaryDirectory() as scratch:
        for name, (mode, depth, tau1, tau2) in RUNS.items():
            record = run_reference(name, mode, depth, tau1, tau2, html, scratch)
            record["made"] = dict(made, parameters={"mode": mode, "depth": depth, "tau1": tau1, "tau2": tau2,
                                                    "names": NAMES})
            with open(os.path.join(HERE, "reference", name + ".json"), "w", encoding="utf-8") as f:
                json.dump(record, f, indent=1, ensure_ascii=False)
                f.write("\n")
    for case, (tau1, tau2, gamma) in DECODER.items():
        plane = decoder_plane(tau1, tau2, gamma)
        plane["made"] = {"how": "make_reference.py: View::basis's construction computed apart from the code",
                         "parameters": {"tau1": tau1, "tau2": tau2, "gamma": gamma, "z0": 0.0}}
        with open(os.path.join(HERE, "decoder", case + ".json"), "w", encoding="utf-8") as f:
            json.dump(plane, f, indent=1, ensure_ascii=False)
            f.write("\n")


if __name__ == "__main__":
    main()
