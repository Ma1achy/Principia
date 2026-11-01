// Safari (WebDriver through `safaridriver`; plan/HUMAN_SETUP.md §6): for each run, loads the site in the given layout,
// waits for the run, and writes the panel's fields as JSON.
import { Builder } from "selenium-webdriver";
import { parseArgs, runUrl, waitForRun, writeFields } from "./common.mjs";

const options = parseArgs(process.argv.slice(2));
const driver = await new Builder().forBrowser("safari").build();
const results = {};
try {
  // Before the first load: Safari would open at its own size, and the rows would not compare. The numbers Chrome's
  // `--window-size` asks for (crates/gui/src/cli.rs, `MEASURED_WINDOW_SIZE`).
  await driver.manage().window().setRect({ width: 1740, height: 900 });
  for (const run of options.runs.split(",")) {
    await driver.get(runUrl(options, run));
    results[run] = await waitForRun(() => driver.executeScript("return window.__principiaMeasure ?? null"));
  }
} finally {
  await driver.quit();
}
process.exit(writeFields(options, "safari", results));
