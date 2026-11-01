// The skeleton's main-thread loader (TASK-M8-86, R-627). It reads the URL's flags, takes what only the main thread can
// measure, and starts the app in its layout. The layout flag: `?layout=worker`, a Web Worker on the OffscreenCanvas this
// loader transfers once, or `?layout=main`, eframe's web runner on this thread; with no flag, the worker, falling back
// to the main thread. A worker that fails before its first frame has returned is recorded, its reason and the layout it
// fell back to in the panel; a failure after that is fatal.
//
// In the worker layout this loader forwards what a worker cannot receive: pointer, wheel, keys, text, the clipboard's
// paste, copy and cut, focus and resize. Pointer moves are coalesced, one a frame (caching_contract Part 6a). All of it
// is undone when the worker layout is given up, or its hidden input would keep the keyboard from the app that replaces it.
//
// The panel's fields are published as `window.__principiaMeasure`, for the runner's scripts.
import init, { start_main, title } from "./gui.js";

const WORKER_START_TIMEOUT_MS = 30000;
const IDLE_LOOP_MS = 2000;
const COI_RELOAD_WAIT_MS = 1500;

const params = new URLSearchParams(location.search);
const measuring = params.has("measure");
const isMac = /Mac|iPhone|iPad/.test(navigator.platform);

function publish(fields) {
  window.__principiaMeasure = fields;
}

function fatal(reason, environment) {
  console.error(reason);
  publish({ status: "failed", reason: String(reason), ...environment });
  const note = document.createElement("p");
  note.id = "failed";
  note.textContent = `The skeleton could not start: ${reason}`;
  document.getElementById("app").replaceWith(note);
}

// The empty requestAnimationFrame loop that measures the display, run before the app starts, so the app's frames cannot
// slow it. Bounded by a timer too: a hidden page gets no animation frames.
function idleLoop() {
  return new Promise((resolve) => {
    const stamps = [];
    const start = performance.now();
    const finish = () => resolve(stamps);
    const tick = (at) => {
      stamps.push(at);
      if (performance.now() - start < IDLE_LOOP_MS) requestAnimationFrame(tick);
      else finish();
    };
    requestAnimationFrame(tick);
    setTimeout(finish, IDLE_LOOP_MS + 500);
  });
}

// The wasm-threads probe: a wasm module on a shared WebAssembly.Memory, a second worker writing a value the first reads
// through Atomics. "ok", or why not.
function threadsProbe() {
  return new Promise((resolve) => {
    let workers = [];
    const finish = (result) => {
      clearTimeout(timer);
      workers.forEach((w) => w.terminate());
      resolve(result);
    };
    const timer = setTimeout(() => finish("timed out: the workers did not answer"), 5000);
    try {
      const memory = new WebAssembly.Memory({ initial: 1, maximum: 1, shared: true });
      workers = ["first", "second"].map((role) => {
        const worker = new Worker("probe-worker.js", { type: "module" });
        worker.onerror = (e) => finish(`the ${role} worker failed: ${e.message}`);
        worker.onmessageerror = () => finish(`the ${role} worker could not receive the shared memory`);
        return worker;
      });
      workers[0].onmessage = ({ data }) => finish(data);
      workers[1].onmessage = ({ data }) => { if (data !== "written") finish(data); };
      workers[0].postMessage({ role: "first", memory });
      workers[1].postMessage({ role: "second", memory });
    } catch (e) {
      finish(`${e.name}: ${e.message}`);
    }
  });
}

async function environment() {
  // A first load is reloaded by coi-serviceworker once its worker is in place: nothing is started for that reload.
  if (!crossOriginIsolated && navigator.serviceWorker && !navigator.serviceWorker.controller) {
    await new Promise((resolve) => setTimeout(resolve, COI_RELOAD_WAIT_MS));
  }
  if (!measuring) return null;
  return {
    crossOriginIsolated: self.crossOriginIsolated === true,
    sharedArrayBuffer: typeof SharedArrayBuffer !== "undefined",
    wasmThreads: await threadsProbe(),
    idleLoopMs: await idleLoop(),
    frameDelayMs: params.get("frame_delay_ms"),
    run: params.get("run"),
  };
}

function encode(environment, layout, fallbackReason) {
  return environment && JSON.stringify({ ...environment, layout, fallbackReason });
}

// A fresh canvas, in place of one whose control went to a failed worker.
function freshCanvas() {
  const old = document.getElementById("app");
  const canvas = old.cloneNode(false);
  old.replaceWith(canvas);
  return canvas;
}

async function startMain(env, fallbackReason) {
  document.title = title();
  const canvas = document.getElementById("app");
  await start_main(canvas, encode(env, "main", fallbackReason), (json) => publish(JSON.parse(json)));
  // eframe reads keys only while its canvas or its text agent has the focus.
  canvas.focus();
}

function canvasSize(canvas) {
  const dpr = devicePixelRatio;
  return [Math.round(canvas.clientWidth * dpr), Math.round(canvas.clientHeight * dpr), dpr];
}

// Installs the forwards; returns what undoes them.
function forward(worker, canvas) {
  const listening = new AbortController();
  const { signal } = listening;
  const post = (method, ...args) => worker.postMessage({ type: "call", method, args });
  const mods = (e) => [e.shiftKey, e.ctrlKey, e.altKey, e.metaKey];
  let moved = null;
  const flush = () => {
    if (moved) post("pointer_move", ...moved);
    moved = null;
  };
  canvas.addEventListener("pointermove", (e) => {
    if (!moved) requestAnimationFrame(flush);
    moved = [e.clientX, e.clientY];
  }, { signal });
  canvas.addEventListener("pointerleave", () => { flush(); post("pointer_gone"); }, { signal });
  const button = (pressed) => (e) => {
    flush();
    if (pressed) canvas.setPointerCapture(e.pointerId);
    post("pointer_button", e.clientX, e.clientY, e.button, pressed, ...mods(e));
    ime.focus();
    e.preventDefault();
  };
  canvas.addEventListener("pointerdown", button(true), { signal });
  canvas.addEventListener("pointerup", button(false), { signal });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault(), { signal });
  canvas.addEventListener("wheel", (e) => {
    flush();
    post("wheel", e.deltaX, e.deltaY, e.deltaMode, ...mods(e));
    e.preventDefault();
  }, { passive: false, signal });
  // A hidden input holds the keyboard: typed text and compositions arrive on it, and the page's clipboard events.
  const ime = document.createElement("input");
  ime.id = "ime";
  ime.autocomplete = "off";
  document.body.append(ime);
  ime.focus();
  const typed = (text) => { if (text) post("text", text); ime.value = ""; };
  ime.addEventListener("input", (e) => { if (!e.isComposing) typed(e.data); }, { signal });
  ime.addEventListener("compositionend", (e) => typed(e.data), { signal });
  const key = (pressed) => (e) => {
    if (e.isComposing) return;
    post("key", e.key, e.code, pressed, e.repeat, ...mods(e));
    // A printable key stays with the input, for its text, as does the clipboard's chord; every other key is the app's.
    const chord = e.metaKey || e.ctrlKey;
    if (e.key.length !== 1 && e.key !== "Dead" && !chord) e.preventDefault();
    if (chord && !"vcx".includes(e.key.toLowerCase())) e.preventDefault();
  };
  addEventListener("keydown", key(true), { signal });
  addEventListener("keyup", key(false), { signal });
  addEventListener("paste", (e) => {
    post("paste", e.clipboardData.getData("text"));
    e.preventDefault();
  }, { signal });
  addEventListener("copy", (e) => { post("copy"); e.preventDefault(); }, { signal });
  addEventListener("cut", (e) => { post("cut"); e.preventDefault(); }, { signal });
  addEventListener("focus", () => { ime.focus(); post("focus", true); }, { signal });
  addEventListener("blur", () => post("focus", false), { signal });
  const resized = new ResizeObserver(() => post("resize", ...canvasSize(canvas)));
  resized.observe(canvas);
  return () => {
    listening.abort();
    resized.disconnect();
    ime.remove();
    moved = null;
  };
}

// Starts the worker layout; rejects with the reason it cannot run.
function startWorker(env) {
  return new Promise((resolve, reject) => {
    if (typeof OffscreenCanvas === "undefined") {
      reject("OffscreenCanvas is missing");
      return;
    }
    const canvas = document.getElementById("app");
    if (!canvas.transferControlToOffscreen) {
      reject("transferControlToOffscreen is missing");
      return;
    }
    const worker = new Worker("worker.js", { type: "module" });
    let started = false;
    let unforward = () => {};
    const stop = () => {
      worker.terminate();
      unforward();
    };
    const timer = setTimeout(() => {
      stop();
      reject(`the worker did not start in ${WORKER_START_TIMEOUT_MS} ms`);
    }, WORKER_START_TIMEOUT_MS);
    const failed = (reason) => {
      clearTimeout(timer);
      stop();
      if (started) fatal(reason, {});
      else reject(reason);
    };
    worker.onerror = (e) => failed(e.message || "the worker failed to load");
    worker.onmessage = ({ data }) => {
      switch (data.type) {
        case "started":
          started = true;
          clearTimeout(timer);
          document.title = title();
          resolve();
          break;
        case "failed":
          failed(data.reason);
          break;
        case "report":
          publish(JSON.parse(data.json));
          break;
        case "copy":
          navigator.clipboard.writeText(data.value).catch((e) => console.error("clipboard:", e));
          break;
        case "cursor":
          canvas.style.cursor = data.value;
          break;
      }
    };
    const [width, height, dpr] = canvasSize(canvas);
    const offscreen = canvas.transferControlToOffscreen();
    unforward = forward(worker, canvas);
    worker.postMessage(
      { type: "start", canvas: offscreen, environment: encode(env, "worker", null), width, height, dpr, isMac },
      [offscreen],
    );
  });
}

async function main() {
  const asked = params.get("layout") ?? "worker";
  if (asked !== "worker" && asked !== "main") {
    fatal(`the layout flag ${JSON.stringify(asked)} is neither worker nor main`, {});
    return;
  }
  const env = await environment();
  if (!navigator.gpu) {
    fatal("this browser has no WebGPU (navigator.gpu is missing)", { ...env, layout: asked });
    return;
  }
  await init();
  if (asked === "main") {
    await startMain(env, null);
    return;
  }
  try {
    await startWorker(env);
  } catch (reason) {
    console.warn("the worker layout failed; falling back to the main thread:", reason);
    freshCanvas();
    await startMain(env, String(reason));
  }
}

main().catch((e) => fatal(e, {}));
