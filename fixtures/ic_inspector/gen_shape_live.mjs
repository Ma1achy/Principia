// Generates fixtures/ic_inspector/shape_live.json: the IC Inspector's shape-sphere point `n` on 5,000 random ICs, the
// cross-check of the live shape readout, `kernel::driver::shape::shape` (dd_integrator §5 test 9; TASK-M3-11;
// REQ-INT-042; R-14, R-184). gen_shape_vec.mjs's extraction and draws, at a seed and a count of its own.
//
// Run from the repository root under the Node version `.nvmrc` pins (R-169), which this script checks:
//
//     node fixtures/ic_inspector/gen_shape_live.mjs
//
// It extracts two pieces of docs/gui/reference/ic_inspector.html's <script> block by fixed markers, the physics
// section ("// ==== physics" up to "// ==== state") and the collision landmarks ("// binary-collision shape points" up
// to "// ==== drawing"), neither of which touches the page, and runs them in a fresh context holding only `Math`. The
// draws come from a fixed seed (mulberry32), so a run reproduces the file bit for bit. CI reads the checked-in file and
// never runs this (R-184).
//
// One set, each number written as its IEEE 754 binary64 bits, 16 hex digits, so the Rust side reads it exactly: ICs
// drawn as the inspector's Randomise button draws them (positions on [−0.9, 0.9]², velocities on [−0.6, 0.6]², masses
// on [0.4, 1.8]), put through its `canonicalise`; recorded are the raw positions, the normalised masses and
// `n = shapePoint(α, β)` of the canonical state (the upper-hemisphere representative: the canonicalisation mirrors
// `λ̃_y < −δ_λ`, so `w ≥ 0`, the mirror fold dd test 9 compares under).

import { readFileSync, writeFileSync } from 'node:fs';
import vm from 'node:vm';

const SEED = 20261114;
const COUNT = 5000;
const HTML = 'docs/gui/reference/ic_inspector.html';
const OUT = 'fixtures/ic_inspector/shape_live.json';
const MARKERS = [
  ['// ============================ physics', '// ============================ state'],
  ['// binary-collision shape points', '// ============================ drawing'],
];

const pinned = readFileSync('.nvmrc', 'utf8').trim();
if (process.version !== `v${pinned}`) {
  console.error(`gen_shape_live: Node ${process.version}, but .nvmrc pins v${pinned}`);
  process.exit(1);
}

const html = readFileSync(HTML, 'utf8');
const open = html.indexOf('<script>');
const close = html.indexOf('</script>', open);
if (open < 0 || close < 0) {
  console.error(`gen_shape_live: no <script> block in ${HTML}`);
  process.exit(1);
}
const script = html.slice(open + '<script>'.length, close);
const pieces = MARKERS.map(([from, to]) => {
  const a = script.indexOf(from);
  const b = script.indexOf(to, a);
  if (a < 0 || b < 0) {
    console.error(`gen_shape_live: marker "${a < 0 ? from : to}" not found in ${HTML}'s <script>`);
    process.exit(1);
  }
  return script.slice(a, b);
});
const inspector = vm.runInNewContext(
  `"use strict";\n${pieces.join('\n')}\n({ decode, canonicalise, shapePoint, collisionPhi });`,
  { Math },
);

// mulberry32: a fixed seed names every draw.
let state = SEED >>> 0;
const random = () => {
  state = (state + 0x6d2b79f5) >>> 0;
  let t = state;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};
const on = (lo, hi) => lo + (hi - lo) * random();

const view = new DataView(new ArrayBuffer(8));
const bits = (x) => {
  view.setFloat64(0, x);
  return view.getBigUint64(0).toString(16).padStart(16, '0');
};

const raw = [];
for (let k = 0; k < COUNT; k++) {
  const bodies = [0, 1, 2].map(() => ({
    r: [on(-0.9, 0.9), on(-0.9, 0.9)],
    v: [on(-0.6, 0.6), on(-0.6, 0.6)],
    m: on(0.4, 1.8),
  }));
  const c = inspector.canonicalise(bodies);
  raw.push({
    r: bodies.map((b) => b.r.map(bits)),
    m: c.m.map(bits),
    n: inspector.shapePoint(c.alpha, c.beta).map(bits),
  });
}

const fixture = {
  description:
    "The IC Inspector's shape-sphere point n on 5,000 random ICs, for the live shape readout's cross-check " +
    '(dd_integrator §5 test 9; REQ-INT-042). Each number is its binary64 bits in hex. raw: Randomise-distributed ICs, ' +
    'canonicalised by the inspector; n is the canonical (upper-hemisphere) representative.',
  generator: 'fixtures/ic_inspector/gen_shape_live.mjs',
  source: HTML,
  markers: MARKERS,
  node: process.version,
  seed: SEED,
  raw,
};
// One IC per line, the header's fields one per line.
const head = Object.entries(fixture)
  .filter(([key]) => key !== 'raw')
  .map(([key, value]) => ` ${JSON.stringify(key)}: ${JSON.stringify(value)}`);
const rows = raw.map((row) => `  ${JSON.stringify(row)}`).join(',\n');
writeFileSync(OUT, `{\n${head.join(',\n')},\n "raw": [\n${rows}\n ]\n}\n`);
console.log(`gen_shape_live: wrote ${OUT}: ${raw.length} ICs (seed ${SEED})`);
