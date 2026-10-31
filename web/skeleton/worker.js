// The skeleton's worker (TASK-M8-86, R-627): it starts the app on the OffscreenCanvas it is sent, runs its frame loop on
// this worker's own requestAnimationFrame, and applies what the loader forwards. Anything that stops it is reported, and
// the loader falls back.
import init, { WorkerApp } from "./gui.js";

let app = null;
const early = [];

function failed(e) {
  self.postMessage({ type: "failed", reason: String(e?.message ?? e) });
}

function call({ method, args }) {
  app[method](...args);
}

function tick() {
  try {
    app.frame();
    requestAnimationFrame(tick);
  } catch (e) {
    failed(e);
  }
}

async function start({ canvas, environment, width, height, dpr, isMac }) {
  if (!self.navigator.gpu) throw new Error("this worker has no WebGPU (navigator.gpu is missing)");
  if (!self.requestAnimationFrame) throw new Error("this worker has no requestAnimationFrame");
  await init();
  const report = (json) => self.postMessage({ type: "report", json });
  const output = (type, value) => self.postMessage({ type, value });
  app = await WorkerApp.start(canvas, environment ?? undefined, width, height, dpr, isMac, report, output);
  early.splice(0).forEach(call);
  self.postMessage({ type: "started" });
  requestAnimationFrame(tick);
}

self.onmessage = ({ data }) => {
  if (data.type === "start") {
    start(data).catch(failed);
  } else if (app) {
    try {
      call(data);
    } catch (e) {
      failed(e);
    }
  } else {
    early.push(data);
  }
};
