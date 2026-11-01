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

// loader.js on a stub page: its worker, its timers, and the input it installs, run by hand.
class Element extends EventTarget {
  style = {};
  clientWidth = 800;
  clientHeight = 600;
  focus() { page.document.activeElement = this; }
  remove() {
    page.body = page.body.filter((e) => e !== this);
    if (page.document.activeElement === this) page.document.activeElement = null;
  }
  replaceWith(other) { if (page.app === this) page.app = other; }
  cloneNode() { return new Element(); }
  setPointerCapture() {}
  transferControlToOffscreen() { return {}; }
}

const page = {
  app: null,
  body: [],
  observers: [],
  workers: [],
  timers: [],
  window: new EventTarget(),
  document: {
    title: "",
    activeElement: null,
    body: { append(element) { page.body.push(element); } },
    getElementById: () => page.app,
    createElement: () => new Element(),
  },
};

async function startLoader() {
  page.app = new Element();
  define("location", { search: "?layout=worker" });
  define("window", {});
  define("navigator", { gpu: {}, platform: "test" });
  define("document", page.document);
  define("devicePixelRatio", 1);
  define("crossOriginIsolated", true);
  define("OffscreenCanvas", class {});
  define("requestAnimationFrame", () => {});
  define("addEventListener", page.window.addEventListener.bind(page.window));
  define("setTimeout", (callback) => page.timers.push(callback));
  define("clearTimeout", () => { page.timers.length = 0; });
  define("ResizeObserver", class {
    connected = false;
    constructor() { page.observers.push(this); }
    observe() { this.connected = true; }
    disconnect() { this.connected = false; }
  });
  define("Worker", class {
    posts = [];
    terminated = false;
    constructor() { page.workers.push(this); }
    postMessage(message) { this.posts.push(message); }
    terminate() { this.terminated = true; }
  });
  console.warn = () => {};
  console.error = () => {};
  const canvas = page.app;
  await load("loader.js");
  await settle();
  const worker = page.workers[0];
  check(worker, "the worker layout did not start");
  const calls = () => worker.posts.filter((m) => m.type === "call").map((m) => m.method);
  const ime = page.body[0];
  check(ime && page.document.activeElement === ime, "the hidden input does not hold the keyboard");
  return { canvas, worker, calls, ime };
}

const event = (type, fields = {}) => Object.assign(new Event(type, { cancelable: true }), fields);
const KEY = { key: "ArrowLeft", code: "ArrowLeft", repeat: false };
const windowEvents = () => [
  event("keydown", KEY),
  event("keyup", KEY),
  event("paste", { clipboardData: { getData: () => "text" } }),
  event("copy"),
  event("cut"),
  event("focus"),
  event("blur"),
];
const canvasEvents = () => ["pointermove", "pointerleave", "pointerdown", "pointerup", "contextmenu", "wheel"].map((type) => event(type, { pointerId: 1 }));

// The forwards work, or nothing that follows shows they went.
function checkForwarding({ canvas, calls, ime }) {
  const before = calls().length;
  const key = event("keydown", KEY);
  page.window.dispatchEvent(key);
  check(calls().length === before + 1 && calls().at(-1) === "key", "a key is not forwarded");
  check(key.defaultPrevented, "the worker layout left an arrow key to the page");
  check(page.body.includes(ime), "the hidden input went");
  check(page.observers[0].connected, "the ResizeObserver is not connected");
  canvas.dispatchEvent(event("pointerdown", { pointerId: 1 }));
  check(calls().at(-1) === "pointer_button", "a press is not forwarded");
}

// Nothing of the worker layout is left: not its input, its focus, nor a listener on the window, the canvas or the input.
function checkUndone({ canvas, calls, ime }) {
  check(!page.body.includes(ime), "the hidden input stays");
  check(page.document.activeElement !== ime, "the hidden input keeps the focus");
  check(!page.observers[0].connected, "the ResizeObserver stays");
  const before = calls().length;
  for (const e of windowEvents()) {
    page.window.dispatchEvent(e);
    check(calls().length === before, `the window's ${e.type} is still forwarded`);
    check(!e.defaultPrevented, `the window's ${e.type} is still cancelled`);
  }
  check(page.document.activeElement !== ime, "the window's focus gives the hidden input the focus back");
  for (const e of canvasEvents()) {
    canvas.dispatchEvent(e);
    check(calls().length === before, `the canvas's ${e.type} is still forwarded`);
    check(!e.defaultPrevented, `the canvas's ${e.type} is still cancelled`);
  }
  for (const type of ["input", "compositionend"]) {
    ime.dispatchEvent(event(type, { data: "a" }));
    check(calls().length === before, `the input's ${type} is still forwarded`);
  }
}

// The worker layout given up: the worker is stopped, its forwards undone, and the main thread holds a new canvas and the
// keyboard.
async function checkFallback(started) {
  await settle();
  check(started.worker.terminated, "the worker was not stopped");
  checkUndone(started);
  const canvas = gui.mainCanvases[0];
  check(canvas, "the app did not start on the main thread");
  check(canvas !== started.canvas && canvas === page.app, "the fallback runs on the canvas the worker took");
  check(page.document.activeElement === canvas, "the fallback's canvas does not hold the keyboard");
}

const LOADER_SCENARIOS = {
  async loader_failed_worker_gives_up_its_forwards() {
    const started = await startLoader();
    checkForwarding(started);
    started.worker.onmessage({ data: { type: "failed", reason: "forced" } });
    await checkFallback(started);
  },
  async loader_broken_worker_gives_up_its_forwards() {
    const started = await startLoader();
    checkForwarding(started);
    started.worker.onerror({ message: "forced" });
    await checkFallback(started);
  },
  async loader_timed_out_worker_gives_up_its_forwards() {
    const started = await startLoader();
    checkForwarding(started);
    page.timers[0]();
    await checkFallback(started);
  },
  async loader_started_worker_keeps_its_forwards() {
    const started = await startLoader();
    started.worker.onmessage({ data: { type: "started" } });
    await settle();
    check(!started.worker.terminated, "a started worker was stopped");
    check(page.timers.length === 0, "a started worker is still on the start's timer");
    checkForwarding(started);
    check(gui.mainCanvases.length === 0, "a started worker gave way to the main thread");
  },
  async loader_fatal_worker_gives_up_its_forwards() {
    const started = await startLoader();
    started.worker.onmessage({ data: { type: "started" } });
    await settle();
    started.worker.onmessage({ data: { type: "failed", reason: "forced" } });
    await settle();
    check(started.worker.terminated, "the worker was not stopped");
    check(gui.mainCanvases.length === 0, "a worker past its first frame fell back");
    checkUndone(started);
  },
};

const scenarios = { ...WORKER_SCENARIOS, ...LOADER_SCENARIOS };
if (!scenarios[scenario]) throw new Error(`no scenario ${scenario}`);
await scenarios[scenario]();
process.exit(0);
