// Tests for the local preview server. Run with: pnpm run test:site
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import path from "node:path";
import { after, before, test } from "node:test";

let server;
let base;

before(async () => {
  const port = 20000 + Math.floor(Math.random() * 20000);
  base = `http://localhost:${port}`;
  // Serves sim/ itself, so the test needs no build output.
  server = spawn(process.execPath, [path.join(import.meta.dirname, "serve.mjs"), "--dir=.", `--port=${port}`]);
  const [line] = await once(server.stdout, "data");
  assert.match(line.toString(), /Serving/);
});

after(() => server.kill());

test("a malformed URL gets a 400 and the server keeps running", async () => {
  assert.equal((await fetch(`${base}/%E0%A4%A`)).status, 400);
  assert.equal((await fetch(`${base}/web/index.html`)).status, 200);
});

test("a folder without its trailing slash redirects", async () => {
  const res = await fetch(`${base}/web`, { redirect: "manual" });
  assert.equal(res.status, 301);
  assert.equal(res.headers.get("location"), "/web/");
});

test("nothing outside the served folder is served", async () => {
  assert.equal((await fetch(`${base}/%2e%2e/package.json`)).status, 404);
});
