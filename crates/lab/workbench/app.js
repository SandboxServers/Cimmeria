"use strict";

const views = {
  dossier: "Target dossier joins client and server observations with source labels.",
  palette: "Slash and dot GM command forms will appear here with preview and receipts.",
  trace: "Causal traces and five-layer truth snapshots will appear here."
};

function createWorkbenchState(storage, instance) {
  const key = `cimmeria.workbench.view.${instance}`;
  const saved = storage.getItem(key);
  let view = saved === "overview" || Object.hasOwn(views, saved) ? saved : "overview";
  return {
    get view() { return view; },
    select(next) {
      if (next !== "overview" && !Object.hasOwn(views, next)) return false;
      view = next;
      storage.setItem(key, next);
      return true;
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

  function renderView() {
    for (const tab of tabs) {
      if (tab.dataset.view === state.view) tab.setAttribute("aria-current", "page");
      else tab.removeAttribute("aria-current");
    }
    overview.hidden = state.view !== "overview";
    placeholder.hidden = state.view === "overview";
    if (state.view !== "overview") {
      const tab = tabs.find(item => item.dataset.view === state.view);
      document.getElementById("placeholder-title").textContent = tab.textContent;
      document.getElementById("placeholder-description").textContent = views[state.view];
    }
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
  renderView();
  refresh();
}

if (typeof module !== "undefined") {
  module.exports = { createWorkbenchState, statusText, sessionData };
}
