// QA's harness for TASK-M8-86 (`qa_TASK-M8-86.rs`): `node qa_TASK-M8-86.mjs <directory holding the skeleton's scripts>
// <scenario>`. It runs the real loader, probe worker and runner helpers on Node, a stub page and a stub `gui.js` around
// the loader, and exits non-zero naming what failed. Written from the task's deliverables: the URL flags, the fields the
// panel is given, the wasm-threads probe, and what the runner scripts write.
import { Worker as NodeWorker, isMainThread, parentPort, workerData } from "node:worker_threads";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath, pathToFileURL } from "node:url";

// A probe worker's thread: the real script, with `self` around the thread's port.
if (!isMainThread) {
  globalThis.self = { postMessage: (message) => parentPort.postMessage(message) };
  await import(pathToFileURL(workerData).href);
  parentPort.on("message", ({ role, memory }) => {
    // A plain memory cannot cross threads: each worker makes its own, as a browser without shared memory leaves it.
    const plain = memory === null ? new WebAssembly.Memory({ initial: 1, maximum: 1 }) : memory;
    globalThis.self.onmessage({ data: { role, memory: plain } });
  });
} else {
  await main();
}

async function main() {
  const [dir, scenario] = process.argv.slice(2);
  const scenarios = { ...probeScenarios(dir), ...loaderScenarios(dir), ...runnerScenarios(dir) };
  if (!scenarios[scenario]) throw new Error(`no scenario ${scenario}`);
  await scenarios[scenario]();
  process.exit(0);
}

function check(holds, what) {
  if (!holds) throw new Error(what);
}

function probeScenarios(dir) {
  const probe = (memory) => new Promise((resolve, reject) => {
    const script = join(dir, "probe-worker.js");
    const [first, second] = ["first", "second"].map(() => new NodeWorker(fileURLToPath(import.meta.url), { workerData: script }));
    first.on("error", reject);
    second.on("error", reject);
    first.on("message", (answer) => {
      first.terminate();
      second.terminate();
      resolve(answer);
    });
    first.postMessage({ role: "first", memory });
    second.postMessage({ role: "second", memory });
  });
  return {
    async probe_shared_memory_says_ok() {
      const answer = await probe(new WebAssembly.Memory({ initial: 1, maximum: 1, shared: true }));
      check(answer === "ok", `the probe on shared memory said: ${answer}`);
    },
    async probe_unshared_memory_says_why_not() {
      // The module's import asks for a shared memory: a plain one does not link, and the probe says why.
      const answer = await probe(null);
      check(answer !== "ok", "the probe said ok with no shared memory");
      check(/\w+Error: /.test(answer), `the probe's reason: ${answer}`);
    },
  };
}

// The loader on a stub page: what it started, with what.
async function runLoader(dir, { search, offscreen = true, gpu = true }) {
  const seen = { main: [], workers: [], transferred: 0 };
  const define = (name, value) => Object.defineProperty(globalThis, name, { value, writable: true, configurable: true });
  define("__seen", seen);
  writeFileSync(join(dir, "package.json"), '{ "type": "module" }\n');
  writeFileSync(join(dir, "gui.js"), `
    export default async function init() {}
    export function title() { return "stub"; }
    export async function start_main(canvas, environment) { globalThis.__seen.main.push(environment); }
  `);
  const canvas = {
    clientWidth: 100,
    clientHeight: 100,
    style: {},
    focus() {},
    cloneNode() { return canvas; },
    replaceWith() {},
    addEventListener() {},
    transferControlToOffscreen: offscreen ? () => { seen.transferred += 1; return {}; } : undefined,
  };
  define("location", { search });
  define("window", globalThis);
  define("navigator", { platform: "MacIntel", gpu: gpu ? {} : undefined });
  define("crossOriginIsolated", true);
  define("self", globalThis);
  define("devicePixelRatio", 1);
  define("addEventListener", () => {});
  define("document", {
    title: "",
    body: { append() {} },
    getElementById: () => canvas,
    createElement: () => ({ focus() {}, remove() {}, addEventListener() {}, style: {} }),
  });
  define("requestAnimationFrame", (tick) => setTimeout(() => tick(performance.now()), 16));
  define("ResizeObserver", class { observe() {} disconnect() {} });
  if (offscreen) define("OffscreenCanvas", class {});
  define("Worker", class {
    constructor(url) {
      this.url = String(url);
      this.posts = [];
      seen.workers.push(this);
    }
    postMessage(message) {
      this.posts.push(message);
      if (this.url.includes("probe-worker") && message.role === "first") setTimeout(() => this.onmessage?.({ data: "ok" }), 0);
      if (this.url.includes("worker.js") && !this.url.includes("probe") && message.type === "start") {
        setTimeout(() => this.onmessage?.({ data: { type: "started" } }), 0);
      }
    }
    terminate() {}
  });
  const silent = () => {};
  console.warn = silent;
  console.error = silent;
  await import(pathToFileURL(join(dir, "loader.js")).href);
  const until = Date.now() + 15000;
  const appWorker = () => seen.workers.find((w) => w.url === "worker.js");
  while (Date.now() < until) {
    const failed = globalThis.__principiaMeasure?.status === "failed";
    if (failed || seen.main.length > 0 || appWorker()?.posts.some((m) => m.type === "start")) break;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  await new Promise((resolve) => setTimeout(resolve, 20));
  return { seen, appWorker: appWorker(), published: globalThis.__principiaMeasure };
}

function loaderScenarios(dir) {
  const checkMeasured = (env, layout, delay, run) => {
    check(env, "no environment under ?measure");
    check(env.layout === layout, `the panel's layout is ${env.layout}, not ${layout}`);
    check(env.crossOriginIsolated === true, "crossOriginIsolated not reported");
    check(env.sharedArrayBuffer === true, "SharedArrayBuffer not reported");
    check(env.wasmThreads === "ok", `the probe's result: ${env.wasmThreads}`);
    check(env.frameDelayMs === delay, `the delay flag: ${env.frameDelayMs}`);
    check(env.run === run, `the run flag: ${env.run}`);
    const stamps = env.idleLoopMs ?? [];
    check(stamps.length > 10, `the idle loop has ${stamps.length} stamps`);
    const span = stamps.at(-1) - stamps[0];
    check(span >= 1900 && span <= 2600, `the idle loop ran ${span} ms, not 2 s`);
  };
  return {
    async loader_layout_main() {
      const { seen, appWorker } = await runLoader(dir, { search: "?layout=main&measure&frame_delay_ms=7&run=idle" });
      check(!appWorker, "?layout=main started a worker");
      check(seen.transferred === 0, "?layout=main transferred its canvas");
      check(seen.main.length === 1, `?layout=main started ${seen.main.length} main apps`);
      checkMeasured(JSON.parse(seen.main[0]), "main", "7", "idle");
    },
    async loader_layout_worker() {
      const { seen, appWorker } = await runLoader(dir, { search: "?layout=worker&measure&run=pan" });
      check(seen.main.length === 0, "?layout=worker started the main app");
      check(appWorker, "?layout=worker started no worker");
      check(seen.transferred === 1, `the canvas was transferred ${seen.transferred} times`);
      const start = appWorker.posts.find((m) => m.type === "start");
      checkMeasured(JSON.parse(start.environment), "worker", null, "pan");
    },
    async loader_layout_bad_is_refused() {
      const { seen, appWorker, published } = await runLoader(dir, { search: "?layout=elsewhere&measure" });
      check(published?.status === "failed", "a bad layout flag was not refused");
      check(String(published.reason).includes("elsewhere"), `the reason: ${published.reason}`);
      check(seen.main.length === 0 && !appWorker, "a bad layout flag started an app");
    },
    async loader_no_measure_no_environment() {
      const { seen, appWorker } = await runLoader(dir, { search: "?layout=worker" });
      check(appWorker, "no worker");
      const start = appWorker.posts.find((m) => m.type === "start");
      check(start.environment === null, `no ?measure, and the environment is ${start.environment}`);
      check(seen.workers.length === 1, "no ?measure, and the threads probe ran");
    },
    async loader_missing_api_is_recorded() {
      const { seen, appWorker } = await runLoader(dir, { search: "?layout=worker&measure", offscreen: false });
      check(!appWorker, "a worker started with no OffscreenCanvas");
      check(seen.main.length === 1, "no fallback to the main thread");
      const env = JSON.parse(seen.main[0]);
      check(env.layout === "main", `the layout fallen back to: ${env.layout}`);
      check(String(env.fallbackReason).includes("OffscreenCanvas"), `the reason: ${env.fallbackReason}`);
    },
    async loader_no_webgpu_is_failed() {
      const { seen, appWorker, published } = await runLoader(dir, { search: "?layout=worker&measure", gpu: false });
      check(published?.status === "failed", "no WebGPU was not reported");
      check(seen.main.length === 0 && !appWorker, "no WebGPU, and an app started");
      check(published.crossOriginIsolated === true, "the failure lost the panel's fields");
    },
  };
}

function runnerScenarios(dir) {
  const common = () => import(pathToFileURL(join(dir, "common.mjs")).href);
  return {
    async runner_args_and_url() {
      const { parseArgs, runUrl } = await common();
      const base = ["--url", "http://localhost:8000/", "--out", "o.json"];
      const options = parseArgs([...base, "--layout", "main", "--frame-delay-ms", "40"]);
      check(options.runs === "pan,idle", `the default runs: ${options.runs}`);
      const url = new URL(runUrl(options, "idle"));
      check(url.searchParams.has("measure"), "the run's URL has no measure flag");
      check(url.searchParams.get("layout") === "main", `the layout flag: ${url.search}`);
      check(url.searchParams.get("run") === "idle", `the run flag: ${url.search}`);
      check(url.searchParams.get("frame_delay_ms") === "40", `the delay flag: ${url.search}`);
      const plain = new URL(runUrl(parseArgs([...base, "--layout", "worker"]), "pan"));
      check(!plain.searchParams.has("frame_delay_ms"), "a delay nobody asked for");
      check(plain.searchParams.get("layout") === "worker", `the layout flag: ${plain.search}`);
      for (const bad of [[...base, "--layout", "other"], [...base], ["--url", "u", "--layout", "main"]]) {
        let threw = false;
        try { parseArgs(bad); } catch { threw = true; }
        check(threw, `parseArgs took ${bad}`);
      }
    },
    async runner_writes_the_fields() {
      const { waitForRun, writeFields } = await common();
      let reads = 0;
      const fields = { status: "done", mean_fps: 60, layout: "worker" };
      const read = async () => {
        reads += 1;
        if (reads === 1) throw new Error("the page is reloading");
        return reads < 3 ? { status: "running" } : fields;
      };
      const got = await waitForRun(read);
      check(got === fields, `the run's fields: ${JSON.stringify(got)}`);
      const out = join(mkdtempSync(join(tmpdir(), "qa-m8-86-")), "fields.json");
      const options = { url: "http://localhost/", layout: "worker", out };
      check(writeFields(options, "chrome", { pan: fields, idle: fields }) === 0, "a good run did not exit 0");
      const written = JSON.parse(readFileSync(out, "utf8"));
      check(written.browser === "chrome" && written.layout === "worker", "the run's header");
      check(written.results.pan.mean_fps === 60 && written.results.idle.layout === "worker", "the fields");
      check(writeFields(options, "chrome", { pan: fields, idle: { status: "timed out" } }) === 1, "a timed-out run exited 0");
    },
  };
}
