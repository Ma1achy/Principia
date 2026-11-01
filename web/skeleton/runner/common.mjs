// What the Chrome and Safari scripts share: the arguments, the URL of a run, and the file they write.
import { writeFileSync } from "node:fs";

export const USAGE = "usage: --url <site URL> --layout worker|main --out <file.json> [--frame-delay-ms <n>] [--runs pan,idle]";
// The idle loop, the probe and a load; then the 5 s warm-up and the 30 s window, with room to spare.
export const RUN_TIMEOUT_MS = 90_000;
export const POLL_MS = 500;

export function parseArgs(argv) {
  const options = { runs: "pan,idle" };
  for (let i = 0; i < argv.length; i += 2) {
    const [flag, value] = [argv[i], argv[i + 1]];
    if (!flag?.startsWith("--") || value === undefined) throw new Error(USAGE);
    options[flag.slice(2).replace(/-([a-z])/g, (_, c) => c.toUpperCase())] = value;
  }
  if (!options.url || !options.out || !["worker", "main"].includes(options.layout)) throw new Error(USAGE);
  return options;
}

export function runUrl(options, run) {
  const url = new URL(options.url);
  url.searchParams.set("measure", "");
  url.searchParams.set("layout", options.layout);
  url.searchParams.set("run", run);
  if (options.frameDelayMs) url.searchParams.set("frame_delay_ms", options.frameDelayMs);
  return url.href;
}

// The fields of a run, once `read` shows it ended; `read` may fail while coi-serviceworker reloads the page.
export async function waitForRun(read) {
  const deadline = Date.now() + RUN_TIMEOUT_MS;
  let last = null;
  while (Date.now() < deadline) {
    try {
      last = (await read()) ?? last;
    } catch {
      // The page is reloading.
    }
    if (last?.status === "done" || last?.status === "failed") return last;
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
  return { status: "timed out", last };
}

export function writeFields(options, browser, results) {
  const failed = Object.values(results).some((fields) => fields.status !== "done");
  writeFileSync(options.out, `${JSON.stringify({ browser, url: options.url, layout: options.layout, results }, null, 2)}\n`);
  console.log(`wrote ${options.out}`);
  return failed ? 1 : 0;
}
