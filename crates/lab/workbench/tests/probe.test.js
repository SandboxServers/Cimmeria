"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { createProbeController, resultPresentation, renderProbe, browserApi } = require("../views/probe.js");
const { createWorkbenchState } = require("../app.js");

function envelope(data, gaps = []) {
  return { schema_version: 1, context: { instance: "default" }, data, gaps };
}

function definition(chunk = "return 1") {
  return { schema_version: 1, name: "health", mode: "read", chunk,
    favorite: false, sha256: `digest-${chunk}` };
}

function fixture() {
  let saved = definition();
  const calls = [];
  const api = {
    session: async () => ({ instance: "default", csrf: "csrf-local" }),
    list: async () => envelope({ definitions: [saved], history: [] }),
    save: async (body, csrf) => {
      calls.push({ action: "save", body, csrf });
      saved = { ...body, sha256: `digest-${body.chunk}`, schema_version: 1 };
      return envelope({ definition: saved });
    },
    run: async (body, csrf) => {
      calls.push({ action: "run", body, csrf });
      return envelope({ definition: saved,
        result: { status: "ok", value: { kind: "number", value: 1 }, detail: null, gaps: [] } });
    },
    browse: async (kind, body, csrf) => {
      calls.push({ action: kind, body, csrf });
      return envelope({ name: body.name, source: "client_lua_eval",
        result: { status: "partial", value: { kind: "cycle" }, gaps: ["tree_truncated"] } });
    }
  };
  const controller = createProbeController({ instance: "default", api, onChange: () => {} });
  return { controller, calls, api, get saved() { return saved; } };
}

test("dirty edits require explicit discard; save and reload round-trip via catalog", async () => {
  const { controller, calls } = fixture();
  await controller.load();
  controller.select("health");
  controller.edit({ chunk: "return 2" });
  assert.equal(controller.snapshot.dirty, true);
  await assert.rejects(() => controller.load(), /Unsaved changes/);
  assert.equal(controller.snapshot.draft.chunk, "return 2");
  await controller.save();
  assert.equal(calls[0].body.expected_sha256, "digest-return 1");
  assert.equal(calls[0].csrf, "csrf-local");
  assert.equal(controller.snapshot.dirty, false);
  controller.edit({ chunk: "return 3" });
  await controller.load(true);
  assert.equal(controller.snapshot.draft.chunk, "return 2");
  assert.equal(controller.snapshot.dirty, false);
});

test("probe tab survives shell reload for the same instance", () => {
  const values = new Map();
  const storage = { getItem: key => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value), removeItem: key => values.delete(key) };
  assert.equal(createWorkbenchState(storage, "default").select("probe"), true);
  assert.equal(createWorkbenchState(storage, "default").view, "probe");
  assert.equal(createWorkbenchState(storage, "p2").view, "overview");
});

test("browser session request uses the gated route with an encoded instance", async () => {
  const originalFetch = global.fetch;
  const calls = [];
  global.fetch = async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => ({ instance: "p 2&x", csrf: "csrf-local" }) };
  };
  try {
    const session = await browserApi("p 2&x").session();
    assert.equal(session.csrf, "csrf-local");
    assert.equal(calls.length, 1);
    assert.equal(calls[0].url, "/workbench/session?instance=p%202%26x");
    assert.equal(calls[0].options.credentials, "same-origin");
  } finally {
    global.fetch = originalFetch;
  }
});

test("editing while a run is in flight discards its late result", async () => {
  const { controller, api } = fixture();
  await controller.load();
  controller.select("health");
  controller.setLeaseId("lease-test");
  let finish;
  api.run = () => new Promise(resolve => { finish = resolve; });
  const pending = controller.run();
  controller.edit({ chunk: "return 3" });
  finish(envelope({ result: { status: "ok", value: { kind: "number", value: 1 }, gaps: [] } }));
  await pending;
  assert.equal(controller.snapshot.dirty, true);
  assert.equal(controller.snapshot.result, null);
});

test("entering a lease while catalog loads does not discard the catalog reply", async () => {
  const { controller, api } = fixture();
  let finish;
  api.list = () => new Promise(resolve => { finish = resolve; });
  const loading = controller.load();
  controller.setLeaseId("lease-test");
  finish(envelope({ definitions: [definition()], history: [] }));
  await loading;
  assert.equal(controller.snapshot.phase, "ready");
  assert.equal(controller.snapshot.definitions.length, 1);
  assert.equal(controller.snapshot.leaseReady, true);
});

test("unleased run and browse fail locally before reaching the API", async () => {
  const { controller, calls } = fixture();
  await controller.load();
  controller.select("health");
  await controller.run();
  assert.match(controller.snapshot.error, /lease ID/);
  await controller.browse("globals", "Game");
  assert.equal(calls.length, 0);
  controller.setLeaseId("lease-test");
  await controller.run({ key: "value" });
  assert.equal(calls[0].body.lease_id, "lease-test");
  assert.equal(calls[0].body.sha256, "digest-return 1");
  assert.deepEqual(calls[0].body.parameters, { key: "value" });
  await controller.browse("globals", "Game");
  assert.equal(calls[1].body.lease_id, "lease-test");
  assert.equal(controller.snapshot.browser.result.partial, true);
});

test("removing a lease clears a successful result and blocks retry", async () => {
  const { controller, calls } = fixture();
  await controller.load();
  controller.select("health");
  controller.setLeaseId("lease-test");
  await controller.run();
  assert.equal(controller.snapshot.result.value.value, 1);
  assert.equal(calls.length, 1);
  controller.setLeaseId("");
  assert.equal(controller.snapshot.result, null);
  assert.equal(controller.snapshot.leaseReady, false);
  await controller.run();
  assert.equal(calls.length, 1);
  assert.equal(controller.snapshot.result, null);
  assert.match(controller.snapshot.error, /lease ID/);
});

test("CEGUI node limit reports partial even without an envelope gap", async () => {
  const { controller, api } = fixture();
  await controller.load();
  controller.setLeaseId("lease-test");
  api.browse = async () => envelope({ source: "client_window_read",
    result: { window: "Root", found: true, nodes: [], truncated: true } });
  await controller.browse("window", "Root");
  assert.equal(controller.snapshot.browser.result.status, "ok");
  assert.equal(controller.snapshot.browser.result.partial, true);
  assert.equal(controller.snapshot.browser.result.value.truncated, true);
});

test("server lease refusal and Lua error are visible and never preserve a prior value", async () => {
  const { controller, api } = fixture();
  await controller.load();
  controller.select("health");
  controller.setLeaseId("expired");
  api.run = async () => { throw new Error("a current lab lease is required"); };
  await controller.run();
  assert.equal(controller.snapshot.phase, "error");
  assert.match(controller.snapshot.error, /current lab lease/);
  assert.equal(controller.snapshot.result, null);
  api.run = async () => envelope({ result: { status: "lua_error", value: null,
    detail: "syntax near end", gaps: [] } });
  await controller.run();
  assert.equal(controller.snapshot.result.status, "lua_error");
  assert.equal(controller.snapshot.result.detail, "syntax near end");
  assert.equal(controller.snapshot.result.value, null);
});

test("cyclic and truncated trees retain visible partial markers", () => {
  const model = resultPresentation({ result: { status: "partial", value: {
    kind: "table", items: [{ key: { kind: "string", value: "self" }, value: { kind: "cycle" } },
      { key: { kind: "string", value: "deep" }, value: { kind: "truncated", reason: "depth_limit" } }]
  }, gaps: ["tree_truncated"] }, gaps: [] });
  assert.equal(model.partial, true);
  const elements = new Map();
  function element(tag) {
    return { tag, children: [], textContent: "", dataset: {}, value: "", checked: false,
      set innerHTML(_) { throw new Error("Unsafe HTML sink"); },
      appendChild(child) { this.children.push(child); },
      replaceChildren() { this.children = []; }
    };
  }
  const document = {
    createElement: element,
    getElementById: id => {
      if (!elements.has(id)) elements.set(id, element("div"));
      return elements.get(id);
    }
  };
  renderProbe(document, { definitions: [], selected: null, draft: { name: "", mode: "read",
    chunk: "", favorite: false }, dirty: false, phase: "result", error: "", result: model,
    browser: null, leaseReady: true });
  const walk = node => [node, ...node.children.flatMap(walk)];
  const visible = walk(elements.get("probe-output")).map(node => node.textContent).join(" ");
  assert.match(visible, /partial\/truncated/);
  assert.match(visible, /cycle/);
  assert.match(visible, /depth_limit/);
});

test("error details render as literal text", () => {
  const hostile = "<img src=x onerror=alert(1)>";
  const model = resultPresentation({ result: { status: "lua_error", detail: hostile,
    value: null, gaps: [] }, gaps: [] });
  assert.equal(model.detail, hostile);
  assert.equal(model.value, null);
});
