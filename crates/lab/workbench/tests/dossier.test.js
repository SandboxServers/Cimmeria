"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { createDossierController, dossierPresentation, fieldDisplay, renderDossier } = require("../views/dossier.js");

function reply(id, current = true) {
  return {
    schema_version: 1,
    context: {
      instance: "default", entity_id: id, current,
      selected_entity_id: { value: current ? id : id + 1 }, observed_at_ms: 1000
    },
    data: {
      client: { actor: { source: "client.entity_table", at_ms: 950, age_ms: 50, value: "0x1234" } },
      server: { template_id: { source: "server_entity_get", at_ms: null, age_ms: null, value: null } }
    },
    gaps: [{ source: "server.sessions", code: "server_session_unbound", detail: "no proof" }]
  };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test("selection switch aborts old request and ignores late response even if fetch ignores abort", async () => {
  const pending = new Map();
  const snapshots = [];
  const controller = createDossierController({
    instance: "default",
    fetchDossier: (id, signal) => {
      const next = deferred();
      pending.set(id, { ...next, signal });
      return next.promise;
    },
    onChange: snapshot => snapshots.push(snapshot)
  });
  const old = controller.select(42);
  const current = controller.select(43);
  assert.equal(pending.get(42).signal.aborted, true);
  assert.equal(controller.snapshot.entityId, 43);
  pending.get(43).resolve(reply(43));
  await current;
  pending.get(42).resolve(reply(42));
  await old;
  assert.equal(controller.snapshot.phase, "current");
  assert.equal(controller.snapshot.entityId, 43);
  assert.equal(snapshots.at(-1).model.groups[0].fields[0].display.value, "0x1234");
});

test("failed read removes previous values and leaves a visible error", async () => {
  let fail = false;
  const controller = createDossierController({
    instance: "default",
    fetchDossier: async id => {
      if (fail) throw new Error("HTTP 503");
      return reply(id);
    },
    onChange: () => {}
  });
  await controller.select(42);
  assert.equal(controller.snapshot.phase, "current");
  fail = true;
  await controller.refresh();
  assert.equal(controller.snapshot.phase, "error");
  assert.equal(controller.snapshot.model, null);
  assert.match(controller.snapshot.error, /503/);
});

test("clearing a target publishes empty state once and keeps no old model", async () => {
  const phases = [];
  const controller = createDossierController({
    instance: "default", fetchDossier: async id => reply(id),
    onChange: snapshot => phases.push(snapshot.phase)
  });
  await controller.select(42);
  controller.select(null);
  assert.equal(controller.snapshot.phase, "empty");
  assert.equal(controller.snapshot.model, null);
  assert.deepEqual(phases, ["empty", "loading", "current", "empty"]);
});

test("stale or mismatched target has no displayed values", () => {
  const stale = dossierPresentation(reply(42, false), "default", 42);
  assert.equal(stale.status, "stale");
  assert.equal(stale.groups[0].fields[0].display.value, "Unknown");
  assert.equal(stale.groups[0].fields[0].display.badge, "stale");
  assert.throws(() => dossierPresentation(reply(42), "default", 43), /Invalid dossier/);
  assert.throws(() => dossierPresentation({ ...reply(42), context: { ...reply(42).context, instance: "other" } }, "default", 42));
});

test("field badges retain source and sample time without inventing unknown data", () => {
  assert.deepEqual(fieldDisplay({ source: "client.actor", at_ms: 900, age_ms: 100, value: false }), {
    value: "false", badge: "observed", source: "client.actor", time: 900, age: 100
  });
  assert.equal(fieldDisplay({ source: "server", at_ms: 900, age_ms: 6000, value: 5 }).badge, "stale");
  assert.equal(fieldDisplay({ source: "server", at_ms: null, age_ms: null, value: null }).badge, "unknown");
});

test("DOM rendering uses text nodes and a target switch clears old content", () => {
  function element(tag) {
    return {
      tag, children: [], textContent: "", dataset: {},
      appendChild(child) { this.children.push(child); },
      replaceChildren() { this.children = []; },
      setAttribute() {}
    };
  }
  const status = element("p");
  const content = element("div");
  const document = {
    getElementById: id => id === "dossier-status" ? status : content,
    createElement: element
  };
  const model = dossierPresentation(reply(42), "default", 42);
  renderDossier(document, { phase: "current", entityId: 42, model });
  assert.ok(content.children.length > 0);
  renderDossier(document, { phase: "loading", entityId: 43, model: null });
  assert.equal(content.children.length, 0);
  assert.match(status.textContent, /43/);
});
