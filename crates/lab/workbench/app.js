"use strict";

const views = {
  dossier: "Inspect the selected runtime entity and the evidence behind each field.",
  probe: "Save and run Lua probes or browse bounded client objects.",
  mission: "Inspect mission definitions, progress, packets, client state and tracker evidence.",
  palette: "Slash and dot GM command forms will appear here with preview and receipts.",
  trace: "Causal traces and five-layer truth snapshots will appear here."
};

function validEntityId(value) {
  const id = Number(value);
  return Number.isInteger(id) && id > 0 && id <= 0xffffffff ? id : null;
}

function createWorkbenchState(storage, instance) {
  const prefix = `cimmeria.workbench.${instance.toLowerCase()}`;
  const viewKey = `${prefix}.view`;
  let saved = storage.getItem(viewKey);
  if (saved === null) {
    const legacyKey = `cimmeria.workbench.view.${instance}`;
    const legacy = storage.getItem(legacyKey);
    if (legacy === "overview" || Object.hasOwn(views, legacy)) {
      saved = legacy;
      storage.setItem(viewKey, legacy);
      storage.removeItem(legacyKey);
    }
  }
  let view = saved === "overview" || Object.hasOwn(views, saved) ? saved : "overview";
  let target = validEntityId(storage.getItem(`${prefix}.target`));
  return {
    get view() { return view; },
    get target() { return target; },
    select(next) {
      if (next !== "overview" && !Object.hasOwn(views, next)) return false;
      view = next;
      storage.setItem(viewKey, next);
      return true;
    },
    selectTarget(raw) {
      const next = validEntityId(raw);
      if (next === null) return false;
      target = next;
      storage.setItem(`${prefix}.target`, String(next));
      return true;
    },
    clearTarget() {
      target = null;
      storage.removeItem(`${prefix}.target`);
    }
  };
}

function statusText(client, gaps = []) {
  if (!client) {
    const gap = gaps.find(item => item.source === "client");
    return gap ? `Unavailable: ${gap.reason}` : "Unavailable";
  }
  if (client.running === true) {
    return client.pid ? `Running (PID ${client.pid})` : "Running";
  }
  if (client.uptime_state === "process_unavailable") return "Unavailable: process status unknown";
  if (client.uptime_state === "exited" || client.uptime_state === "pid_reused") {
    return `Stopped (${client.uptime_state.replace("_", " ")})`;
  }
  return "Stopped";
}

function sessionData(payload, instance) {
  if (payload.schema_version !== 1 || !Array.isArray(payload.gaps)
      || payload.context?.instance?.toLowerCase() !== instance.toLowerCase()
      || payload.data?.instance?.toLowerCase() !== instance.toLowerCase()) {
    throw new Error("Invalid session response");
  }
  return payload.data;
}

if (typeof document !== "undefined") {
  const instance = document.body.dataset.instance;
  const state = createWorkbenchState(sessionStorage, instance);
  const tabs = [...document.querySelectorAll("[data-view]")];
  const overview = document.getElementById("overview");
  const placeholder = document.getElementById("placeholder");
  const dossier = document.getElementById("dossier");
  const probe = document.getElementById("probe");
  const mission = document.getElementById("mission");
  const probeController = window.CimmeriaProbe.mountProbe(document, instance);
  const missionController = window.CimmeriaMission.mountMission(document, instance, sessionStorage);
  const targetInput = document.getElementById("target-entity-id");
  const dossierController = window.CimmeriaDossier.createDossierController({
    instance,
    onChange: snapshot => window.CimmeriaDossier.renderDossier(document, snapshot),
    fetchDossier: async (entityId, signal) => {
      const url = `/workbench/api/v1/dossier?instance=${encodeURIComponent(instance)}&entity_id=${entityId}`;
      const response = await fetch(url, { credentials: "same-origin", cache: "no-store", signal });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return response.json();
    }
  });

  function renderView() {
    for (const tab of tabs) {
      if (tab.dataset.view === state.view) tab.setAttribute("aria-current", "page");
      else tab.removeAttribute("aria-current");
    }
    overview.hidden = state.view !== "overview";
    dossier.hidden = state.view !== "dossier";
    probe.hidden = state.view !== "probe";
    mission.hidden = state.view !== "mission";
    placeholder.hidden = state.view === "overview" || state.view === "dossier"
      || state.view === "probe" || state.view === "mission";
    if (!placeholder.hidden) {
      const tab = tabs.find(item => item.dataset.view === state.view);
      document.getElementById("placeholder-title").textContent = tab.textContent;
      document.getElementById("placeholder-description").textContent = views[state.view];
    }
    if (state.view === "dossier") dossierController.refresh();
    if (state.view === "probe" && probeController.snapshot.phase === "empty") probeController.load();
    if (state.view === "mission" && missionController.snapshot.phase === "empty") missionController.refresh();
  }

  async function refresh() {
    const connection = document.getElementById("connection");
    connection.textContent = "Refreshing…";
    try {
      const response = await fetch(`/workbench/api/v1/session?instance=${encodeURIComponent(instance)}`, {
        credentials: "same-origin", cache: "no-store"
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const payload = await response.json();
      const data = sessionData(payload, instance);
      document.getElementById("client-status").textContent = statusText(data.client, payload.gaps);
      document.getElementById("lease-status").textContent = data.lease?.held
        ? `Held by ${data.lease.lease?.owner ?? "another session"}` : "Free";
      connection.textContent = payload.gaps.length ? "Partial status" : "Connected";
    } catch (error) {
      connection.textContent = `Status unavailable: ${error.message}`;
      document.getElementById("client-status").textContent = "Unknown";
      document.getElementById("lease-status").textContent = "Unknown";
    }
  }

  tabs.forEach(tab => tab.addEventListener("click", () => {
    if (state.select(tab.dataset.view)) renderView();
  }));
  document.getElementById("refresh").addEventListener("click", refresh);
  document.getElementById("target-form").addEventListener("submit", event => {
    event.preventDefault();
    if (!state.selectTarget(targetInput.value)) {
      targetInput.setCustomValidity("Enter a runtime entity ID from 1 to 4294967295.");
      targetInput.reportValidity();
      return;
    }
    targetInput.setCustomValidity("");
    dossierController.select(state.target);
  });
  targetInput.addEventListener("input", () => targetInput.setCustomValidity(""));
  document.getElementById("target-clear").addEventListener("click", () => {
    state.clearTarget();
    targetInput.value = "";
    dossierController.select(null);
  });
  document.getElementById("dossier-refresh").addEventListener("click", () => dossierController.refresh());
  targetInput.value = state.target ?? "";
  dossierController.select(state.target, false);
  renderView();
  refresh();
}

if (typeof module !== "undefined") {
  module.exports = { createWorkbenchState, statusText, sessionData, validEntityId };
}
