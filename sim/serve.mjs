// Serves the simulator on http://localhost:8000. Run with:
//   pnpm run serve:sim            (or --port=8080)
// --dir serves another folder, relative to sim/; serve:site uses it.
//
// Unlike `python3 -m http.server`, it tells the browser never to cache, so a
// reload after `pnpm run build:sim` always gets the new page, script and
// WebAssembly together. A mix of old and new files breaks the page.
import { createServer } from "node:http";

import { argv, fs, path } from "zx";

const root = path.resolve(import.meta.dirname, argv.dir ?? "web");
const port = Number(argv.port ?? 8000);
const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".wav": "audio/wav",
  ".mp3": "audio/mpeg",
  ".json": "application/json",
};

createServer(async (request, response) => {
  const url = new URL(request.url ?? "/", "http://localhost");
  let file;
  try {
    file = path.join(root, decodeURIComponent(url.pathname.endsWith("/") ? url.pathname + "index.html" : url.pathname));
  } catch {
    response.writeHead(400, { "Content-Type": "text/plain" }).end("Bad request");
    return;
  }
  // Never serve anything outside the served folder.
  const inside = file.startsWith(root + path.sep);
  // A folder without its trailing slash: redirect, as GitHub Pages does.
  if (inside && (await fs.pathExists(file)) && (await fs.stat(file)).isDirectory()) {
    response.writeHead(301, { Location: url.pathname + "/" }).end();
    return;
  }
  if (!inside || !(await fs.pathExists(file))) {
    response.writeHead(404, { "Content-Type": "text/plain" }).end("Not found");
    return;
  }
  response.writeHead(200, {
    "Content-Type": TYPES[path.extname(file)] ?? "application/octet-stream",
    "Cache-Control": "no-store",
  });
  fs.createReadStream(file).pipe(response);
}).listen(port, () => console.log(`Serving ${path.relative(process.cwd(), root)} on http://localhost:${port}  (Ctrl+C to stop)`));
