// The skeleton's start and fallback, run on Node with a stub `gui.js` and a stub page (TASK-M8-86, R-627):
// `node web_skeleton_fallback.mjs <directory holding worker.js and loader.js> <scenario>`. Exits non-zero, naming what
// failed. `web_skeleton_fallback.rs` runs it, on the real files and on edited ones for its controls.
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [dir, scenario] = process.argv.slice(2);

function check(holds, what) {
  if (!holds) throw new Error(what);
}

function define(name, value) {
  Object.defineProperty(globalThis, name, { value, writable: true, configurable: true });
}

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((resolve) => setImmediate(resolve));
};

// What the stub gui.js does and saw.
const gui = { frames: 0, failFrame: null, startRejects: false, mainCanvases: [] };
define("__gui", gui);
writeFileSync(join(dir, "package.json"), '{ "type": "module" }\n');
writeFileSync(join(dir, "gui.js"), `
const gui = globalThis.__gui;
export default async function init() {}
export function title() { return "stub"; }
export async function start_main(canvas) { gui.mainCanvases.push(canvas); }
export class WorkerApp {
  static async start() {
    if (gui.startRejects) throw new Error("forced start failure");
    return new WorkerApp();
  }
  frame() {
    gui.frames += 1;
    if (gui.frames === gui.failFrame) throw new Error("forced frame failure");
  }
}
`);

const load = (file) => import(pathToFileURL(join(dir, file)).href);

// worker.js: the messages it posts, and its requestAnimationFrame run by hand.
async function startWorker() {
  const posted = [];
  const frames = [];
  define("self", globalThis);
  define("navigator", { gpu: {} });
  define("postMessage", (message) => posted.push(message));
  define("requestAnimationFrame", (callback) => frames.push(callback));
  await load("worker.js");
  globalThis.onmessage({ data: { type: "start", width: 1, height: 1, dpr: 1, isMac: false } });
  await settle();
  const types = () => posted.map((m) => m.type);
  return { types, posted, frame: () => frames.shift()() };
}

const WORKER_SCENARIOS = {
  async worker_started_after_first_frame() {
    const worker = await startWorker();
    check(!worker.types().includes("started"), "started was posted before any frame ran");
    worker.frame();
    check(worker.types().join() === "started", `the first frame returned, and it posted: ${worker.types()}`);
    worker.frame();
    check(worker.types().join() === "started", `started is posted once, and it posted: ${worker.types()}`);
  },
  async worker_first_frame_failure_is_not_started() {
    gui.failFrame = 1;
    const worker = await startWorker();
    worker.frame();
    check(!worker.types().includes("started"), "a first frame that failed was reported as started");
    check(worker.types().join() === "failed", `the first frame failed, and it posted: ${worker.types()}`);
    check(worker.posted[0].reason === "forced frame failure", `the reason: ${worker.posted[0].reason}`);
  },
  async worker_later_frame_failure_follows_started() {
    gui.failFrame = 2;
    const worker = await startWorker();
    worker.frame();
    worker.frame();
    check(worker.types().join() === "started,failed", `the second frame failed, and it posted: ${worker.types()}`);
  },
  async worker_start_failure_is_not_started() {
    gui.startRejects = true;
    const worker = await startWorker();
    check(worker.types().join() === "failed", `the start failed, and it posted: ${worker.types()}`);
  },
};

const scenarios = { ...WORKER_SCENARIOS };
if (!scenarios[scenario]) throw new Error(`no scenario ${scenario}`);
await scenarios[scenario]();
process.exit(0);
