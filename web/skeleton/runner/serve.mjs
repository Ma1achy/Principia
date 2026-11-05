// A local static server for the built site: `node serve.mjs <site directory> [port]`. localhost is a secure context, so
// the service worker and WebGPU run there as on the Pages URL. It sets no COOP or COEP: as on Pages, those come from
// coi-serviceworker.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const [root, port = "8000"] = process.argv.slice(2);
if (!root) throw new Error("usage: node serve.mjs <site directory> [port]");
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };

createServer(async (request, response) => {
  const path = normalize(new URL(request.url, "http://localhost").pathname).replace(/^(\.\.[/\\])+/, "");
  try {
    const body = await readFile(join(root, path.endsWith("/") ? `${path}index.html` : path));
    response.writeHead(200, { "Content-Type": TYPES[extname(path)] ?? (path.endsWith("/") ? "text/html" : "application/octet-stream") });
    response.end(body);
  } catch {
    response.writeHead(404);
    response.end();
  }
}).listen(Number(port), "127.0.0.1", () => console.log(`http://localhost:${port}/`));
