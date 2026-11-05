// What the service worker answers a page with, navigation after navigation, on the real `index.html` and the vendored
// `coi-serviceworker.js` (TASK-M8-86, R-627; caching_contract Part 6a: when it is active, the page is isolated):
// `node web_skeleton_coi.mjs <directory holding them> safari|chrome`. Exits non-zero, naming what failed.
// `web_skeleton_coi.rs` runs it, on the real files and on edited ones for its controls.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import vm from "node:vm";

const [dir, browser] = process.argv.slice(2);
const html = readFileSync(join(dir, "index.html"), "utf8");
const coiSource = readFileSync(join(dir, "coi-serviceworker.js"), "utf8");

// The scripts that run before the loader, in the page's order, as a browser runs them.
function classicScripts() {
  const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/g)]
    .filter(([, attributes]) => !attributes.includes('type="module"'))
    .map(([, attributes, body]) => (attributes.includes('src="coi-serviceworker.js"') ? coiSource : body));
  if (!scripts.includes(coiSource)) throw new Error("the page does not load coi-serviceworker.js");
  return scripts;
}

// The worker's global scope: no `window`, so the vendored file takes its worker branch.
const listeners = {};
const worker = vm.createContext({
  self: { addEventListener: (type, listener) => { listeners[type] = listener; } },
  console,
  Headers,
  Response,
  fetch: async () => new Response("the page"),
});
vm.runInContext(coiSource, worker);

async function navigate() {
  let answered;
  listeners.fetch({
    request: { mode: "navigate", cache: "default" },
    respondWith: (response) => { answered = response; },
  });
  return (await answered).headers;
}

// A page the worker controls, isolated as Safari is: by `require-corp` and not by `credentialless`, the header's value
// `headers` gave it. Its scripts run, and what they post reaches the worker.
function loadPage(headers) {
  const window = {
    crossOriginIsolated: headers.get("Cross-Origin-Embedder-Policy") === "require-corp",
    isSecureContext: true,
    location: { reload() {} },
    document: { currentScript: { src: "coi-serviceworker.js" } },
    ...(browser === "chrome" ? { chrome: {} } : {}),
  };
  const serviceWorker = {
    controller: { postMessage: (data) => listeners.message({ data }) },
    register: async () => ({ addEventListener() {} }),
  };
  const page = vm.createContext({ window, navigator: { serviceWorker }, console });
  for (const script of classicScripts()) vm.runInContext(script, page);
}

// The first navigation, a fresh worker's, then each that follows the page before it.
for (let navigation = 1; navigation <= 4; navigation++) {
  const headers = await navigate();
  const coep = headers.get("Cross-Origin-Embedder-Policy");
  if (coep !== "require-corp") throw new Error(`navigation ${navigation}: COEP ${coep}`);
  const coop = headers.get("Cross-Origin-Opener-Policy");
  if (coop !== "same-origin") throw new Error(`navigation ${navigation}: COOP ${coop}`);
  const corp = headers.get("Cross-Origin-Resource-Policy");
  if (corp !== "cross-origin") throw new Error(`navigation ${navigation}: CORP ${corp}`);
  loadPage(headers);
}
