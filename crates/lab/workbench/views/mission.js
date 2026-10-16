"use strict";

const SOURCE_LABELS = [
  ["definition", "Authored definition"],
  ["steps", "Authored steps"],
  ["objectives", "Authored objectives"],
  ["chain_references", "Mission-scoped chain references"],
  ["live_server", "Live server mission instance"],
  ["server", "Persisted server state"],
  ["wire", "Decoded packet evidence"],
  ["client", "Client mission state"],
  ["ui", "Visible client tracker"]
];

function validMissionId(raw) {
  const value = String(raw ?? "");
  if (!/^[1-9]\d*$/.test(value)) return null;
  const id = Number(value);
  return Number.isSafeInteger(id) && id <= 2147483647 ? id : null;
}

function missionPresentation(payload, instance, missionId) {
  if (payload?.schema_version !== 1 || !Array.isArray(payload.gaps)
      || payload.context?.instance?.toLowerCase() !== instance.toLowerCase()
      || payload.context?.mission_id !== missionId || !payload.data
      || typeof payload.data !== "object") {
    throw new Error("Invalid mission response");
  }
  const gaps = payload.gaps.map(gap => ({
    source: String(gap?.source ?? "unknown"),
    code: String(gap?.code ?? "unavailable"),
    detail: String(gap?.detail ?? "")
  }));
  const layers = {};
  for (const [key, label] of SOURCE_LABELS) {
    const field = payload.data[key];
    if (!field || typeof field.source !== "string" || !Object.hasOwn(field, "value")) {
      throw new Error(`Invalid ${key} evidence`);
    }
    const matchingId = (key !== "definition" && key !== "server")
      || field.value === null || field.value?.mission_id === missionId;
    const reason = !matchingId ? "response_mission_mismatch"
      : gaps.filter(gap => gap.source === field.source)
        .map(gap => `${gap.code}${gap.detail ? `: ${gap.detail}` : ""}`).join("; ");
    const value = matchingId ? field.value : null;
    const age = Number.isFinite(field.age_ms) && field.age_ms >= 0 ? field.age_ms : null;
    layers[key] = {
      label, source: field.source, value, age,
      at: Number.isFinite(field.at_ms) ? field.at_ms : null,
      status: value === null ? "unknown" : age === null || age > 5000 ? "stale" : "observed",
      reason
    };
  }
  const server = layers.server.value;
  const matched = payload.data.matched_tracker;
  const matchedTracker = layers.ui.value !== null && layers.definition.value !== null
    && matched && typeof matched === "object" ? matched : null;
  const activeStep = server !== null && server.status === 1
    && payload.data.active_step?.id === server.current_step_id
    ? payload.data.active_step : null;
  const disagreement = activeStep !== null && matchedTracker !== null
    && typeof payload.data.step_disagreement === "boolean"
    ? payload.data.step_disagreement : null;
  return {
    missionId, layers, gaps, matchedTracker, activeStep, disagreement,
    observedAt: Number.isFinite(payload.context.observed_at_ms)
      ? payload.context.observed_at_ms : null
  };
}

function createMissionController({ instance, fetchMission, onChange, storage }) {
  const key = `cimmeria.workbench.${instance.toLowerCase()}.mission_id`;
  let missionId = validMissionId(storage?.getItem(key));
  let revision = 0;
  let active = null;
  let snapshot = { phase: "empty", missionId, model: null, error: null };
  function publish(phase, model = null, error = null) {
    snapshot = { phase, missionId, model, error };
    onChange(snapshot);
  }
  function cancel() {
    revision += 1;
    active?.abort();
    active = null;
  }
  async function refresh() {
    cancel();
    if (missionId === null) { publish("empty"); return; }
    const request = new AbortController();
    active = request;
    const ticket = revision;
    publish("loading");
    try {
      const payload = await fetchMission(missionId, request.signal);
      if (ticket !== revision || request.signal.aborted) return;
      const model = missionPresentation(payload, instance, missionId);
      publish("ready", model);
    } catch (cause) {
      if (ticket === revision && !request.signal.aborted) publish("error", null, cause.message);
    } finally {
      if (ticket === revision) active = null;
    }
  }
  function select(raw) {
    const next = validMissionId(raw);
    if (next === null) return false;
    cancel();
    missionId = next;
    storage?.setItem(key, String(next));
    publish("empty");
    refresh();
    return true;
  }
  function clear() {
    cancel();
    missionId = null;
    storage?.removeItem(key);
    publish("empty");
  }
  return { select, clear, refresh, get snapshot() { return snapshot; } };
}

function addText(document, parent, tag, text, className) {
  const element = document.createElement(tag);
  element.textContent = String(text);
  if (className) element.className = className;
  parent.appendChild(element);
  return element;
}

function showLayer(document, content, layer) {
  const section = addText(document, content, "section", "", "dossier-group");
  addText(document, section, "h3", layer.label);
  addText(document, section, "span", layer.status, `badge badge-${layer.status}`);
  let provenance = layer.source;
  if (layer.at !== null) provenance += ` · ${new Date(layer.at).toLocaleString()}`;
  if (layer.age !== null) provenance += ` · ${Math.round(layer.age / 1000)}s old`;
  addText(document, section, "p", provenance, "provenance");
  if (layer.value === null) {
    addText(document, section, "p", `Unknown${layer.reason ? `: ${layer.reason}` : ""}`);
  }
  return section;
}

function showRows(document, parent, rows, describe) {
  if (!Array.isArray(rows)) return;
  if (!rows.length) { addText(document, parent, "p", "No rows observed in this source."); return; }
  const list = addText(document, parent, "ul", "");
  for (const row of rows) addText(document, list, "li", describe(row));
}

function renderMission(document, snapshot) {
  const status = document.getElementById("mission-status");
  const content = document.getElementById("mission-content");
  content.replaceChildren();
  status.dataset.state = snapshot.phase;
  if (snapshot.phase === "empty") {
    status.textContent = snapshot.missionId === null ? "Enter a mission ID to inspect."
      : `Mission ${snapshot.missionId} is selected; refresh to read it.`;
    return;
  }
  if (snapshot.phase === "loading") {
    status.textContent = `Loading mission ${snapshot.missionId}…`;
    return;
  }
  if (snapshot.phase === "error") {
    status.textContent = `Mission ${snapshot.missionId}: read unavailable (${snapshot.error}).`;
    return;
  }
  const model = snapshot.model;
  status.textContent = `Mission ${model.missionId}: independent source observations`;
  if (model.observedAt !== null) {
    addText(document, content, "p", `Response assembled ${new Date(model.observedAt).toLocaleString()}`, "sample-time");
  }
  const { layers } = model;
  const definition = showLayer(document, content, layers.definition);
  if (layers.definition.value !== null) {
    const row = layers.definition.value;
    addText(document, definition, "p", `ID ${row.mission_id} · ${row.mission_defn ?? "title unknown"}`);
    if (row.mission_label) addText(document, definition, "p", `Label: ${row.mission_label}`);
  }
  const steps = showLayer(document, content, layers.steps);
  if (layers.steps.value !== null) {
    showRows(document, steps, layers.steps.value, row =>
      `Step ${row.step_id}: ${row.step_display_log_text ?? "description unknown"}`);
  }
  const objectives = showLayer(document, content, layers.objectives);
  if (layers.objectives.value !== null) {
    showRows(document, objectives, layers.objectives.value, row =>
      `Step ${row.step_id}, objective ${row.objective_id}: ${row.display_log_text ?? "description unknown"}${row.is_optional ? " · optional" : ""}${row.is_hidden ? " · hidden" : ""}`);
  }
  const chains = showLayer(document, content, layers.chain_references);
  if (layers.chain_references.value !== null) {
    showRows(document, chains, layers.chain_references.value, row =>
      `Chain ${row.chain_id}: ${row.description ?? "description unknown"}`);
  }
  showLayer(document, content, layers.live_server);
  const server = showLayer(document, content, layers.server);
  if (layers.server.value !== null) {
    const row = layers.server.value;
    addText(document, server, "p", `Mission ${row.mission_id} · status ${row.status ?? "unknown"} · persisted step ${row.current_step_id ?? "none"}`);
    if (model.activeStep) addText(document, server, "p", `Active step ${model.activeStep.id}: ${model.activeStep.description ?? "description unknown"}`);
    for (const key of ["active_objective_ids", "completed_objective_ids", "failed_objective_ids"]) {
      if (Array.isArray(row[key])) addText(document, server, "p", `${key}: ${row[key].join(", ") || "none"}`);
    }
  }
  for (const key of ["wire", "client"]) {
    const section = showLayer(document, content, layers[key]);
    if (layers[key].value !== null) addText(document, section, "pre", JSON.stringify(layers[key].value, null, 2).slice(0, 4000));
  }
  const tracker = showLayer(document, content, layers.ui);
  if (layers.ui.value !== null) {
    const rows = layers.ui.value.mission_tracker;
    showRows(document, tracker, rows, row => `${row.kind ?? "line"}: ${row.text ?? ""}`);
    addText(document, tracker, "p", model.matchedTracker
      ? `Selected title matched: ${model.matchedTracker.title}; visible step: ${model.matchedTracker.step_text ?? "none"}`
      : "Selected mission ID has no uniquely bound tracker title.");
  }
  const comparison = addText(document, content, "section", "", "dossier-group");
  addText(document, comparison, "h3", "Persisted step vs visible tracker");
  addText(document, comparison, "p", model.disagreement === null
    ? "Unknown: one or both comparable observations are unavailable."
    : model.disagreement ? "Disagreement observed between persisted step and visible tracker."
      : "Descriptions agree in this sample.");
  if (model.gaps.length) {
    const section = addText(document, content, "section", "", "dossier-gaps");
    addText(document, section, "h3", "Missing evidence and disagreements");
    showRows(document, section, model.gaps,
      gap => `${gap.source}: ${gap.code}${gap.detail ? ` — ${gap.detail}` : ""}`);
  }
}

function mountMission(document, instance, storage) {
  const controller = createMissionController({
    instance, storage, onChange: state => renderMission(document, state),
    fetchMission: async (missionId, signal) => {
      const url = `/workbench/api/v1/missions?instance=${encodeURIComponent(instance)}&mission_id=${missionId}`;
      const response = await fetch(url, { credentials: "same-origin", cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    }
  });
  const input = document.getElementById("mission-id");
  document.getElementById("mission-form").addEventListener("submit", event => {
    event.preventDefault();
    if (!controller.select(input.value)) {
      input.setCustomValidity("Enter a mission ID from 1 to 2147483647.");
      input.reportValidity();
      return;
    }
    input.setCustomValidity("");
  });
  input.addEventListener("input", () => input.setCustomValidity(""));
  document.getElementById("mission-clear").addEventListener("click", () => {
    controller.clear();
    input.value = "";
  });
  document.getElementById("mission-refresh").addEventListener("click", () => controller.refresh());
  input.value = controller.snapshot.missionId ?? "";
  renderMission(document, controller.snapshot);
  return controller;
}

const exported = { validMissionId, missionPresentation, createMissionController, renderMission, mountMission };
if (typeof module !== "undefined") module.exports = exported;
if (typeof window !== "undefined") window.CimmeriaMission = exported;
