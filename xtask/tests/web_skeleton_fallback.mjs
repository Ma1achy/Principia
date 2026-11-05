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
    ime.value = "a";
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

// A key as the browser handles it: the keydown, then, unless it was cancelled, the edit the hidden input makes of it.
// `getModifierState` answers only for AltGraph, as the key's fields say. Returns whether the key was cancelled.
function press({ ime }, fields, altGraph, edit) {
  const key = event("keydown", { code: "KeyQ", repeat: false, ...fields, getModifierState: (m) => m === "AltGraph" && altGraph });
  page.window.dispatchEvent(key);
  if (!key.defaultPrevented && edit) {
    ime.value = edit;
    ime.dispatchEvent(event("input", { data: edit, isComposing: false }));
  }
  return key.defaultPrevented;
}

const LOADER_SCENARIOS = {
  async loader_altgraph_character_reaches_the_input() {
    const started = await startLoader();
    const texts = () => started.worker.posts.filter((m) => m.method === "text").map((m) => m.args[0]);
    const posted = (pressed) => started.worker.posts.filter((m) => m.method === "key" && m.args[2] === pressed);
    const keys = () => posted(true).length + posted(false).length;
    const ctrlAlt = { ctrlKey: true, altKey: true };
    check(!press(started, { key: "@", ...ctrlAlt }, true, "@"), "AltGr's `@` was cancelled");
    check(texts().join() === "@", `AltGr's \`@\` reached the app as: ${texts()}`);
    check(!press(started, { key: "ą", code: "KeyA", ...ctrlAlt }, true, "ą"), "AltGr's `ą` was cancelled");
    page.window.dispatchEvent(event("keyup", { key: "ą", code: "KeyA", repeat: false, ...ctrlAlt, getModifierState: () => true }));
    check(texts().join() === "@,ą", `AltGr's \`ą\` reached the app as: ${texts()}`);
    check(!press(started, { key: "Dead", ...ctrlAlt }, true), "AltGr's dead key was cancelled");
    check(keys() === 0, `AltGr's characters went on as ${keys()} keys, for egui to read as Ctrl+Alt`);
    check(press(started, { key: "Enter", ...ctrlAlt }, true), "AltGr's Enter was left to the page");
    check(keys() === 1, "AltGr's Enter is not forwarded");
    check(press(started, { key: "s", ctrlKey: true }, false, "s"), "Ctrl+S was left to the page");
    check(press(started, { key: "@", ...ctrlAlt }, false, "@"), "Ctrl+Alt with no AltGraph was left to the page");
    check(press(started, { key: "s", ctrlKey: true, metaKey: true }, true, "s"), "Cmd+S under AltGraph was left to the page");
    check(keys() === 4, "a chord is not forwarded");
    check(!press(started, { key: "v", ctrlKey: true }, false), "Ctrl+V's paste was cancelled");
    check(texts().join() === "@,ą", `only AltGr's characters are text, and the app got: ${texts()}`);
    // A key held as AltGr goes down still comes up, under the name it went down by, or it stays down in egui: Space held
    // turns the stain canvas's presses into pans.
    for (const [held, layered, code] of [[" ", " ", "Space"], ["q", "@", "KeyQ"], ["7", "{", "Digit7"]]) {
      check(!press(started, { key: held, code }, false), `plain ${code} was cancelled`);
      check(posted(true).at(-1).args[1] === code, `plain ${code}'s press is not forwarded`);
      press(started, { key: "AltGraph", code: "AltRight", ...ctrlAlt }, true);
      page.window.dispatchEvent(event("keyup", { key: layered, code, repeat: false, ...ctrlAlt, getModifierState: (m) => m === "AltGraph" }));
      const release = posted(false).at(-1);
      check(release?.args[1] === code, `${code}'s keyup under AltGraph did not reach the worker`);
      check(release.args[0] === held, `${code} came up as ${release.args[0]}, and egui holds ${held} down`);
    }
    // A press and release leave nothing behind, so a later press dropped under AltGr keeps its keyup dropped too.
    const keyup = (fields, held) => page.window.dispatchEvent(event("keyup", { repeat: false, ...fields, getModifierState: (m) => m === "AltGraph" && held }));
    press(started, { key: "x", code: "KeyX" }, false);
    keyup({ key: "x", code: "KeyX" }, false);
    const before = keys();
    press(started, { key: "ź", code: "KeyX", ...ctrlAlt }, true, "ź");
    keyup({ key: "ź", code: "KeyX", ...ctrlAlt }, true);
    check(keys() === before, "a keyup whose press was dropped went on");
  },
  async loader_cmd_up_releases_what_the_browser_withholds() {
    const started = await startLoader();
    const releases = (code) => started.worker.posts.filter((m) => m.method === "key" && m.args[1] === code && m.args[2] === false);
    const up = (key, code, held = () => false) =>
      page.window.dispatchEvent(event("keyup", { key, code, repeat: false, getModifierState: held }));
    press(started, { key: "Meta", code: "MetaLeft", metaKey: true }, false);
    press(started, { key: "z", code: "KeyZ", metaKey: true }, false);
    up("Meta", "MetaLeft");
    check(releases("KeyZ").length === 1, "z's release was lost with Cmd's keyup, so egui holds it down");
    check(releases("KeyZ")[0].args[0] === "z", `z came up as ${releases("KeyZ")[0].args[0]}`);
    up("z", "KeyZ");
    check(releases("KeyZ").length === 1, "a late keyup released z again");
    check(releases("MetaLeft").length === 1, "Cmd's own release was not forwarded once");
    press(started, { key: "Shift", code: "ShiftLeft", shiftKey: true }, false);
    press(started, { key: "Meta", code: "MetaLeft", metaKey: true }, false);
    up("Meta", "MetaLeft", (m) => m === "Shift");
    check(releases("ShiftLeft").length === 0, "Shift went up with Cmd while still held");
    up("Shift", "ShiftLeft");
    check(releases("ShiftLeft").length === 1, "Shift's own keyup was lost");
    press(started, { key: "q", code: "KeyQ" }, false);
    up("Control", "ControlLeft");
    check(releases("KeyQ").length === 0, "Control's keyup let a held key up early");
    up("q", "KeyQ");
    check(releases("KeyQ").length === 1, "q's own keyup was lost");
  },
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
