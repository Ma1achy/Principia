const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch();
  for (const [f, tag] of [['trace/trace_markers.v28pre.html','markers PRE'],['trace/trace_markers.html','markers'],['sound/trace_sound.v12pre.html','sound PRE'],['sound/trace_sound.html','sound']]) {
    const p = await b.newPage({ viewport: { width: 1400, height: 1000 } });
    const errs = []; p.on('pageerror', e => errs.push('PAGEERR ' + e.message));
    await p.goto('file://' + process.cwd() + '/' + f); await p.waitForTimeout(1500);
    const r = await p.evaluate(() => {
      const out = {};
      // A. restart on a new point while playing
      $('anim').value = 'playhead'; playing = true; playT = 6; compute([0.3, 0.4]); out.restartPlaying = playT;
      playing = false; playT = 6; compute([0.35, 0.45]); out.pausedUnchanged = playT;
      // B. icons at the corners and edges, every set, several rotations: count opaque pixels on the canvas border
      const W = cv.width, H = cv.height, s = W / chart.W * 1.6; let edge = 0, drawn = 0;
      const sets = ['principia', 'arrows', 'pictures', 'orbits'];
      for (const k of sets) for (const ang of [0, 0.7, 1.6, 2.5, 3.14, -1.2]) for (const pt of [[0, 0], [W, 0], [0, H], [W, H], [W / 2, 0], [0, H / 2], [W - 3, H / 2], [W / 2, H - 2]]) {
        const I = ICONS[k]; const fns = [q => I.collision(q, '12', ang, s), q => I.escape(q, 2, ang, s), q => I.triple(q, s), q => I.ejection(q, s), q => I.horizon(q, s), q => I.exit(q, ang, s), q => I.enter(q, ang, s), q => I.start(q, ang, '#fff27a', decodeShape(0.5, 0.6), s), q => syzygyIcon(q, 1, s, false, ang)];
        for (const fn of fns) { g.clearRect(0, 0, W, H); fn(pt); const d = g.getImageData(0, 0, W, H).data; let e = 0, n = 0;
          for (let i = 0; i < W * H; i++) if (d[i * 4 + 3] > 8) { n++; const x = i % W, y = (i / W) | 0; if (x === 0 || y === 0 || x === W - 1 || y === H - 1) e++; }
          drawn++; if (e > 0) edge++; } }
      out.iconCases = drawn; out.touchingEdge = edge;
      return out; });
    // C. paused at t = 0: dotted
    await p.evaluate(() => { playing = false; playT = 0; $('ph').value = 0; setT(); compute([0.62, 0.30]); });
    await p.waitForTimeout(300); await p.locator('#cv').screenshot({ path: `tt_${tag.replace(' ', '_')}_t0.png` });
    await p.evaluate(() => { playT = 8; $('ph').value = 400; setT(); }); await p.waitForTimeout(300); await p.locator('#cv').screenshot({ path: `tt_${tag.replace(' ', '_')}_t8.png` });
    console.log(tag, JSON.stringify(r), errs.join(' ') || 'no errors'); await p.close();
  }
  await b.close();
})();
