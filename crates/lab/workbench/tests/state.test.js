"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { createWorkbenchState, validEntityId } = require("../app.js");

function storage() {
  const items = new Map();
  return {
    getItem: key => items.get(key) ?? null,
    setItem: (key, value) => items.set(key, value),
    removeItem: key => items.delete(key)
  };
}

test("view and target survive reload per instance and clearing persists", () => {
  const saved = storage();
  const first = createWorkbenchState(saved, "Default");
  assert.equal(first.select("dossier"), true);
  assert.equal(first.selectTarget("4294967295"), true);
  assert.equal(createWorkbenchState(saved, "default").view, "dossier");
  assert.equal(createWorkbenchState(saved, "default").target, 4294967295);
  assert.equal(createWorkbenchState(saved, "other").target, null);
  first.clearTarget();
  assert.equal(createWorkbenchState(saved, "default").target, null);
});

test("legacy view selection migrates to the per-instance key on reload", () => {
  const saved = storage();
  saved.setItem("cimmeria.workbench.view.Default", "dossier");
  const reloaded = createWorkbenchState(saved, "Default");
  assert.equal(reloaded.view, "dossier");
  assert.equal(saved.getItem("cimmeria.workbench.default.view"), "dossier");
  assert.equal(saved.getItem("cimmeria.workbench.view.Default"), null);
  assert.equal(createWorkbenchState(saved, "Default").view, "dossier");
  assert.equal(createWorkbenchState(saved, "other").view, "overview");
});

test("new view selection takes precedence over legacy state", () => {
  const saved = storage();
  saved.setItem("cimmeria.workbench.default.view", "palette");
  saved.setItem("cimmeria.workbench.view.default", "dossier");
  assert.equal(createWorkbenchState(saved, "default").view, "palette");
});

test("invalid target input cannot replace persisted selection", () => {
  const saved = storage();
  const state = createWorkbenchState(saved, "default");
  assert.equal(state.selectTarget(42), true);
  for (const invalid of ["", "0", "-1", "4.2", "4294967296", "Infinity", "abc"]) {
    assert.equal(state.selectTarget(invalid), false);
    assert.equal(state.target, 42);
  }
  assert.equal(validEntityId("42"), 42);
});
