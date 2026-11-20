// The libm-pin harness's wasm arm (R-991; `cargo xtask measure libm-pin --wasm`): runs the cdylib built for
// wasm32-unknown-unknown under Node's WebAssembly, every chart and zoom, and writes the file `libm-pin write` writes,
// its host `wasm32-node-<version>`, its arm `<arm>`, the arm the cdylib was built as (`pinned` or `std`).
// Usage: node run.mjs <libm_pin.wasm> <file> <arm>
import { readFileSync, writeFileSync } from "node:fs";

const [wasmPath, outPath, arm] = process.argv.slice(2);
if (arm !== "pinned" && arm !== "std") throw new Error(`run.mjs: arm \`${arm}\` is not pinned or std`);
const CHARTS = 3, ZOOMS = [0, 10, 20], WIDTH = 1920, HEIGHT = 1080;
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const { memory, libm_pin_alloc, libm_pin_fill } = instance.exports;
const n = WIDTH * HEIGHT;
const out = libm_pin_alloc(n);
const parts = [Buffer.from(`libm-pin 1 ${arm} wasm32-node-${process.versions.node} ${WIDTH} ${HEIGHT}\n`)];
for (let chart = 0; chart < CHARTS; chart++) {
  for (const zoom of ZOOMS) {
    libm_pin_fill(chart, zoom, WIDTH, HEIGHT, out);
    parts.push(Buffer.from(new Uint8Array(memory.buffer, out, n * 8)));
    process.stderr.write(`libm-pin wasm: chart ${chart} at 2^${zoom} done\n`);
  }
}
writeFileSync(outPath, Buffer.concat(parts));
