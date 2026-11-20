// the per-pixel march for the state map: DOPRI5, snapshots landed exactly at t_k = k·DT, terminal state held
const DA = [[], [1 / 5], [3 / 40, 9 / 40], [44 / 45, -56 / 15, 32 / 9], [19372 / 6561, -25360 / 2187, 64448 / 6561, -212 / 729], [9017 / 3168, -355 / 33, 46732 / 5247, 49 / 176, -5103 / 18656], [35 / 384, 0, 500 / 1113, 125 / 192, -2187 / 6784, 11 / 84]];
const DE = [71 / 57600, 0, -71 / 16695, 71 / 1920, -17253 / 339200, 22 / 525, -1 / 40];
function march(y, m, T, DT, TOL, HMAX, RCOLL, out, off) {
  const K = [...Array(7)].map(() => new Float64Array(12)), tmp = new Float64Array(12), yn = new Float64Array(12);
  const deriv = (y, d) => { for (let k = 0; k < 6; k++) d[k] = y[6 + k]; for (let k = 6; k < 12; k++) d[k] = 0;
    for (let i = 0; i < 3; i++) for (let j = i + 1; j < 3; j++) { const dx = y[2 * j] - y[2 * i], dy = y[2 * j + 1] - y[2 * i + 1], r2 = dx * dx + dy * dy, ir3 = 1 / (r2 * Math.sqrt(r2));
      d[6 + 2 * i] += m[j] * dx * ir3; d[7 + 2 * i] += m[j] * dy * ir3; d[6 + 2 * j] -= m[i] * dx * ir3; d[7 + 2 * j] -= m[i] * dy * ir3; } };
  const coll = y => { let c = 0, pair = -1; for (const [i, j, p] of [[0, 1, 0], [0, 2, 1], [1, 2, 2]]) if (Math.hypot(y[2 * j] - y[2 * i], y[2 * j + 1] - y[2 * i + 1]) < RCOLL) { c++; pair = p; } return c >= 2 ? 3 : pair; };
  const NK = Math.round(T / DT) + 1; let t = 0, h = 1e-3, steps = 0, k = 1, ev = 0;   // ev: 0 none, 1 collision (+pair), 2 budget/non-finite
  const put = kk => { for (let q = 0; q < 12; q++) out[off + kk * 12 + q] = y[q]; };
  put(0); deriv(y, K[0]);
  const c0 = coll(y); if (c0 >= 0) { for (let kk = 1; kk < NK; kk++) put(kk); return { tEnd: 0, ev: 1, pair: c0, steps }; }
  while (k < NK) {
    const tk = k * DT;
    if (!(h > 0) || steps > 200000) { ev = 2; break; }
    let hh = Math.min(h, tk - t), land = hh >= tk - t - 1e-14; if (land) hh = tk - t;
    for (let s = 1; s < 7; s++) { for (let q = 0; q < 12; q++) { let a = y[q]; for (let p = 0; p < s; p++) a += hh * DA[s][p] * K[p][q]; tmp[q] = a; } deriv(tmp, K[s]); }
    yn.set(tmp); let err = 0;
    for (let q = 0; q < 12; q++) { let e = 0; for (let p = 0; p < 7; p++) e += DE[p] * K[p][q]; e *= hh; err = Math.max(err, Math.abs(e) / (TOL * (1e-3 + Math.max(Math.abs(y[q]), Math.abs(yn[q]))))); }
    if (!Number.isFinite(err)) { ev = 2; break; }
    if (err <= 1) { t += hh; y.set(yn); K[0].set(K[6]); steps++;
      if (land) { t = tk; put(k); k++; }
      const c = coll(y); if (c >= 0) { for (let kk = k; kk < NK; kk++) put(kk); return { tEnd: t, ev: 1, pair: c, steps }; } }
    h = Math.min(HMAX, hh * Math.min(4, Math.max(.2, .9 * Math.pow(err || 1e-10, -.2))));
  }
  if (ev === 2) for (let kk = k; kk < NK; kk++) for (let q = 0; q < 12; q++) out[off + kk * 12 + q] = NaN;
  return { tEnd: ev ? t : T, ev, pair: -1, steps };
}
if (typeof module !== 'undefined') module.exports = { march };
