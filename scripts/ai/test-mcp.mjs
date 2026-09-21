// Interoperability against the official SDK and a real, isolated Rust listener.
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { resolve, join, sep } from "node:path";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const home = await mkdtemp(join(tmpdir(), "serverbond-mcp-test-"));
const binary = resolve(
  "target/debug",
  process.platform === "win32" ? "serverbond.exe" : "serverbond",
);
const env = { ...process.env, SERVERBOND_HOME: home };
let server;
let closed;
let client;
try {
  const listener = createServer();
  listener.listen(0, "127.0.0.1");
  await once(listener, "listening");
  const port = listener.address().port;
  await new Promise((resolve, reject) =>
    listener.close((e) => (e ? reject(e) : resolve())),
  );
  const token = execFileSync(binary, ["api", "token"], {
    env,
    windowsHide: true,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
  assert.match(token, /^sb_[a-f0-9]{64}$/);
  const base = `http://127.0.0.1:${port}`;
  server = spawn(binary, ["api", "serve", String(port)], {
    env,
    windowsHide: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let diagnostics = "";
  server.stdout.on("data", (chunk) => {
    diagnostics += chunk;
  });
  server.stderr.on("data", (chunk) => {
    diagnostics += chunk;
  });
  closed = once(server, "close");
  for (let attempt = 0; ; attempt++) {
    try {
      const response = await fetch(`${base}/api/v1/health`, {
        signal: AbortSignal.timeout(1000),
      });
      if (response.ok) break;
    } catch {
      /* listener may not be ready yet */
    }
    assert.ok(
      attempt < 100 && server.exitCode === null,
      `API did not start: ${diagnostics}`,
    );
    await delay(100);
  }
  const headers = {
    Authorization: `Bearer ${token}`,
    "Content-Type": "application/json",
  };
  const saved = await fetch(`${base}/api/v1/api`, {
    method: "PUT",
    headers,
    body: JSON.stringify({ enabled: true, port, mcpEnabled: true }),
  });
  assert.equal(saved.status, 200);
  client = new Client({ name: "serverbond-sdk-test", version: "1.0.0" });
  await client.connect(
    new StreamableHTTPClientTransport(new URL(`${base}/mcp`), {
      requestInit: { headers: { Authorization: `Bearer ${token}` } },
    }),
  );
  assert.equal(client.getServerVersion().name, "serverbond");
  await client.ping();
  const { tools } = await client.listTools();
  const contract = await (
    await fetch(`${base}/api/v1/openapi.json`, { headers })
  ).json();
  const operationCount = Object.values(contract.paths).reduce(
    (count, operations) => count + Object.keys(operations).length,
    0,
  );
  assert.equal(tools.length, operationCount);
  assert.ok(
    tools.some((tool) => tool.name === "serverbond_post_projects_by_id_deploy"),
  );
  const status = await client.callTool({
    name: "serverbond_get_status",
    arguments: {},
  });
  assert.equal(status.isError, false);
  assert.equal(status.structuredContent.data.settings.api.mcpEnabled, true);
  const stop = await client.callTool({
    name: "serverbond_post_services_by_id_stop",
    arguments: { id: "redis" },
  });
  assert.equal(stop.isError, false);
  const missing = await client.callTool({
    name: "serverbond_get_services_by_id",
    arguments: { id: "missing" },
  });
  assert.equal(missing.isError, true);
  await assert.rejects(
    () =>
      client.callTool({
        name: "serverbond_get_services_by_id",
        arguments: { id: "../settings" },
      }),
    /-32602/,
  );
  await client.close();
  client = null;
  console.log(
    `MCP SDK interoperability passed: initialize, ping, ${tools.length} tools, read/write, errors, close.`,
  );
} finally {
  await client?.close();
  if (server && server.exitCode === null) {
    server.stdin.end("\n");
    let timer;
    await Promise.race([
      closed,
      new Promise((resolve) => {
        timer = setTimeout(() => {
          server.kill();
          resolve();
        }, 5000);
      }),
    ]);
    clearTimeout(timer);
    await closed;
  }
  const target = resolve(home);
  assert.ok(
    target.startsWith(resolve(tmpdir()) + sep) &&
      target.includes("serverbond-mcp-test-"),
  );
  await rm(target, { recursive: true, force: true, maxRetries: 3 });
}
