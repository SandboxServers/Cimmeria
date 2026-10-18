"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { createWorkbenchState, statusText, sessionData } = require("./app.js");

function memoryStorage() {
  const values = new Map();
  return {
    getItem: key => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value)
  };
}

test("view selection survives reload and stays isolated per instance", () => {
  const storage = memoryStorage();
  const first = createWorkbenchState(storage, "default");
  assert.equal(first.select("palette"), true);
  assert.equal(createWorkbenchState(storage, "default").view, "palette");
  assert.equal(createWorkbenchState(storage, "p2").view, "overview");
  assert.equal(first.select("not-a-view"), false);
  assert.equal(createWorkbenchState(storage, "default").view, "palette");
  assert.equal(createWorkbenchState(storage, "p2").select("trace"), true);
  assert.equal(createWorkbenchState(storage, "default").view, "palette");
});

test("session envelope rejects wrong instance and status keeps gaps visible", () => {
  const reply = {
    schema_version: 1,
    context: { instance: "p2" },
    data: { instance: "p2", csrf: "test", client: null, lease: { held: false } },
    gaps: [{ source: "client", reason: "bridge_unavailable" }]
  };
  assert.equal(sessionData(reply, "p2").instance, "p2");
  assert.equal(statusText(reply.data.client, reply.gaps), "Unavailable: bridge_unavailable");
  assert.throws(() => sessionData(reply, "default"), /Invalid session response/);
  assert.throws(() => sessionData({ ...reply, schema_version: 2 }, "p2"), /Invalid session response/);
});

test("a retained PID never implies that the process is running", () => {
  assert.equal(statusText({ pid: 456, running: false, uptime_state: "exited" }), "Stopped (exited)");
  assert.equal(statusText({ pid: 456, running: false, uptime_state: "pid_reused" }), "Stopped (pid reused)");
  assert.equal(statusText({ pid: 456, running: false, uptime_state: "process_unavailable" }), "Unavailable: process status unknown");
  assert.equal(statusText({ pid: 456, running: true, uptime_state: "available" }), "Running (PID 456)");
});
