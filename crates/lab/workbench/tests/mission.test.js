"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { validMissionId, missionPresentation, createMissionController, renderMission, updateMissionAges, mountMission } = require("../views/mission.js");
const { createWorkbenchState } = require("../app.js");

function sample(source, value, age_ms = 100) {
  return { source, value, at_ms: 900, age_ms };
}

function response(missionId = 622) {
  return {
    schema_version: 1,
    context: { instance: "default", mission_id: missionId, observed_at_ms: 1000 },
    data: {
      definition: sample("resources.missions", { mission_id: missionId, mission_defn: "Arm Yourself!" }),
      steps: sample("resources.mission_steps", [
        { mission_id: missionId, step_id: 80623, step_display_log_text: "Search the Guard", step_enabled: false }
      ]),
      objectives: sample("resources.mission_objectives", [
        { step_id: 80623, objective_id: 91023, display_log_text: "Search the Guard", is_enabled: false, is_optional: false }
      ]),
      chain_references: sample("resources.content_chains", []),
      live_server: sample("server_runtime.mission", null, null),
      server: sample("sgw_mission", { mission_id: missionId, status: 1, current_step_id: 80623,
        active_objective_ids: [91023] }),
      wire: sample("server_packet_tap", null, null),
      client: sample("client.mission_state", null, null),
      ui: sample("client_ui_state.mission_tracker", { mission_tracker: [
        { kind: "mission", text: "Arm Yourself!" }, { kind: "step", text: "Search Frost" }
      ] }),
      matched_tracker: { title: "Arm Yourself!", step_text: "Search Frost" },
      active_step: { id: 80623, description: "Search the Guard" },
      step_disagreement: true
    },
    gaps: [
      { source: "server_packet_tap", code: "unavailable", detail: "no decoded packet reader" },
      { source: "client.mission_state", code: "unavailable", detail: "no selected-ID reader" },
      { source: "server_runtime.mission", code: "unavailable", detail: "no bound live mission instance reader" },
      { source: "sgw_mission+client_ui_state", code: "step_disagreement", detail: "different text" }
    ]
  };
}

function storage() {
  const items = new Map();
  return { getItem: key => items.get(key) ?? null,
    setItem: (key, value) => items.set(key, value),
    removeItem: key => items.delete(key) };
}

function fakeDocument() {
  function element(tag) {
    return {
      tag, textContent: "", children: [], dataset: {},
      appendChild(child) { this.children.push(child); },
      replaceChildren() { this.children = []; },
      addEventListener() {}, setCustomValidity() {}, reportValidity() {}
    };
  }
  const nodes = { "mission-status": element("p"), "mission-content": element("div"),
    "mission-id": element("input"), "mission-form": element("form"),
    "mission-clear": element("button"), "mission-refresh": element("button") };
  return { getElementById: id => nodes[id], createElement: element, nodes };
}

function allText(node) {
  return [node.textContent, ...node.children.map(allText)].join(" ");
}

test("mission selection is per-instance browser state and reload retains only the ID", () => {
  const saved = storage();
  const controller = createMissionController({ instance: "default", storage: saved,
    fetchMission: async id => response(id), onChange: () => {} });
  assert.equal(controller.select("622"), true);
  assert.equal(createMissionController({ instance: "default", storage: saved,
    fetchMission: async id => response(id), onChange: () => {} }).snapshot.missionId, 622);
  assert.equal(createMissionController({ instance: "p2", storage: saved,
    fetchMission: async id => response(id), onChange: () => {} }).snapshot.missionId, null);
  assert.equal(createWorkbenchState(saved, "default").select("mission"), true);
  assert.equal(createWorkbenchState(saved, "default").view, "mission");
  controller.clear();
  assert.equal(saved.getItem("cimmeria.workbench.default.mission_id"), null);
  assert.equal(validMissionId("2147483648"), null);
  assert.equal(validMissionId("0"), null);
});

test("switching missions clears old server state and ignores late response", async () => {
  const pending = new Map();
  const controller = createMissionController({ instance: "default", storage: storage(),
    onChange: () => {}, fetchMission: id => new Promise(resolve => pending.set(id, resolve)) });
  controller.select("622");
  controller.select("623");
  assert.equal(controller.snapshot.missionId, 623);
  assert.equal(controller.snapshot.model, null);
  pending.get(622)(response(622));
  await Promise.resolve();
  assert.equal(controller.snapshot.model, null);
  pending.get(623)(response(623));
  await Promise.resolve();
  assert.equal(controller.snapshot.model.missionId, 623);
  assert.equal(controller.snapshot.model.layers.server.value.mission_id, 623);
});

test("stale refresh and wrong-mission server evidence cannot show as current", async () => {
  let finish;
  const controller = createMissionController({ instance: "default", storage: storage(),
    onChange: () => {}, fetchMission: id => id === 622 ? response(622)
      : new Promise(resolve => { finish = resolve; }) });
  controller.select("622");
  await Promise.resolve();
  assert.equal(controller.snapshot.model.layers.server.value.mission_id, 622);
  controller.select("623");
  assert.equal(controller.snapshot.model, null);
  const wrong = response(623);
  wrong.data.server.value.mission_id = 622;
  wrong.data.server.age_ms = 9000;
  finish(wrong);
  await Promise.resolve();
  assert.equal(controller.snapshot.model.layers.server.value, null);
  assert.equal(controller.snapshot.model.layers.server.reason, "response_mission_mismatch");
  assert.equal(controller.snapshot.model.disagreement, null);
  const old = response(623);
  old.data.server.age_ms = 9000;
  assert.equal(missionPresentation(old, "default", 623).layers.server.status, "stale");
});

test("missing DB leaves authored and server layers unknown while tracker stays only UI", () => {
  const payload = response();
  for (const [key, source] of [["definition", "resources.missions"],
    ["steps", "resources.mission_steps"], ["objectives", "resources.mission_objectives"],
    ["server", "sgw_mission"]]) {
    payload.data[key] = sample(source, null, null);
    payload.gaps.push({ source, code: "unavailable", detail: "database unavailable" });
  }
  payload.data.active_step = { id: null, description: null };
  payload.data.step_disagreement = null;
  const model = missionPresentation(payload, "default", 622);
  assert.equal(model.layers.server.value, null);
  assert.equal(model.layers.objectives.status, "unknown");
  assert.match(model.layers.objectives.reason, /database unavailable/);
  assert.equal(model.matchedTracker, null);
  assert.equal(model.disagreement, null);
  const document = fakeDocument();
  renderMission(document, { phase: "ready", missionId: 622, model });
  const rendered = allText(document.nodes["mission-content"]);
  assert.match(rendered, /Visible client tracker/);
  assert.match(rendered, /Search Frost/);
  assert.match(rendered, /Unknown: one or both comparable observations/);
  assert.doesNotMatch(rendered, /Active step 80623/);
});

test("tracker/server disagreement renders both values without rewriting persisted progress", () => {
  const model = missionPresentation(response(), "default", 622);
  assert.equal(model.disagreement, true);
  assert.equal(model.layers.server.value.current_step_id, 80623);
  assert.equal(model.matchedTracker.step_text, "Search Frost");
  const document = fakeDocument();
  renderMission(document, { phase: "ready", missionId: 622, model });
  const rendered = allText(document.nodes["mission-content"]);
  assert.match(rendered, /Authored objectives/);
  assert.match(rendered, /objective 91023/);
  assert.match(rendered, /step_enabled: false/);
  assert.match(rendered, /is_enabled: false/);
  assert.match(rendered, /Active step 80623: Search the Guard/);
  assert.match(rendered, /visible step: Search Frost/);
  assert.match(rendered, /Disagreement observed/);
  assert.match(rendered, /no decoded packet reader/);
  assert.match(rendered, /Live server mission instance/);
  assert.match(rendered, /no bound live mission instance reader/);
});

test("displayed sample ages and stale badges advance while the response stays open", () => {
  const model = missionPresentation(response(), "default", 622, 1000);
  const document = fakeDocument();
  const snapshot = { phase: "ready", missionId: 622, model };
  const ageTargets = renderMission(document, snapshot, 1000);
  const sameContent = document.nodes["mission-content"].children[1];
  const initial = allText(document.nodes["mission-content"]);
  assert.match(initial, /observed/);
  assert.match(initial, /0s old/);
  updateMissionAges(ageTargets, model.receivedAt, 7000);
  assert.equal(document.nodes["mission-content"].children[1], sameContent);
  const aged = allText(document.nodes["mission-content"]);
  assert.match(aged, /stale/);
  assert.match(aged, /6s old/);
  assert.equal(model.layers.server.value.current_step_id, 80623);
});

test("mounted mission age timer updates badges without replacing evidence content", async () => {
  const oldWindow = global.window;
  const oldFetch = global.fetch;
  let tick;
  global.window = { setInterval: callback => { tick = callback; } };
  global.fetch = async () => ({ ok: true, json: async () => response(622) });
  try {
    const document = fakeDocument();
    const controller = mountMission(document, "default", storage());
    controller.select("622");
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(controller.snapshot.phase, "ready");
    const content = document.nodes["mission-content"].children[1];
    controller.snapshot.model.receivedAt -= 6100;
    tick();
    assert.equal(document.nodes["mission-content"].children[1], content);
    assert.match(allText(document.nodes["mission-content"]), /stale/);
    assert.match(allText(document.nodes["mission-content"]), /6s old/);
  } finally {
    global.window = oldWindow;
    global.fetch = oldFetch;
  }
});

test("response envelope rejects wrong instance and mission ID", () => {
  assert.throws(() => missionPresentation(response(622), "p2", 622), /Invalid mission response/);
  assert.throws(() => missionPresentation(response(622), "default", 623), /Invalid mission response/);
});
