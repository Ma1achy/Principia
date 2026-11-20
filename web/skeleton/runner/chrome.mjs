// Chrome (Playwright on the installed Chrome stable, no browser download): for each run, loads the site in the given
// layout, waits for the run, and writes the panel's fields as JSON. `--headless true` is for a smoke test only: a headless
// page's frames do not read the display.
import { chromium } from "@playwright/test";
import { parseArgs, runUrl, waitForRun, writeFields } from "./common.mjs";

const options = parseArgs(process.argv.slice(2));
// `viewport: null`: the window's own size and device pixel ratio. Playwright's emulated viewport reports a ratio of 1 on a
// Retina display, while eframe sizes its canvas from the device's real pixels, so the two layouts would draw different
// sizes.
const browser = await chromium.launch({
  channel: "chrome",
  headless: options.headless === "true",
  args: ["--window-size=1740,900"],
});
const results = {};
try {
  for (const run of options.runs.split(",")) {
    const page = await browser.newPage({ viewport: null });
    page.on("console", (m) => { if (m.type() === "error") console.error(`[${run}] ${m.text()}`); });
    page.on("pageerror", (e) => console.error(`[${run}] ${e.message}`));
    await page.goto(runUrl(options, run));
    results[run] = await waitForRun(() => page.evaluate(() => window.__principiaMeasure ?? null));
    if (options.screenshot) await page.screenshot({ path: `${options.screenshot}-${run}.png` });
    await page.close();
  }
} finally {
  await browser.close();
}
process.exit(writeFields(options, "chrome", results));
