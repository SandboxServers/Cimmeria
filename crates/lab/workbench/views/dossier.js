"use strict";

const FIELD_GROUPS = [
  ["Client", [
    ["Actor", "client.actor"], ["Rendered", "client.rendered"],
    ["Targetable", "client.targetable"], ["Mob ID", "client.mob_id"],
    ["Name", "client.name"], ["Position (UE3)", "client.position_ue3"]
  ]],
  ["Server", [
    ["Template ID", "server.template_id"], ["Health", "server.health_cur"],
    ["Health maximum", "server.health_max"], ["State", "server.state_field"],
    ["Spawn ID", "server.spawn_id"], ["Tag", "server.tag"],
    ["Position (m)", "server.position_m"],
    ["Witnessed by", "server.witnessed_by"], ["Witnesses", "server.witnesses"]
  ]],
  ["Cross-source", [
    ["Position delta (m)", "position_delta"], ["Recent messages", "recent_messages"]
  ]]
];

function validEnvelope(payload, instance, entityId) {
  const context = payload?.context;
  return payload?.schema_version === 1 && Array.isArray(payload.gaps)
    && typeof payload.data === "object" && payload.data !== null
    && typeof context?.instance === "string"
    && context.instance.toLowerCase() === instance.toLowerCase()
    && context.entity_id === entityId;
}

function readField(data, path) {
  return path.split(".").reduce((value, key) => value?.[key], data);
}

function fieldDisplay(field) {
  if (!field || typeof field !== "object" || field.value === null || field.value === undefined) {
    return { value: "Unknown", badge: "unknown", source: field?.source ?? "unavailable", time: null };
  }
  const age = Number.isFinite(field.age_ms) && field.age_ms >= 0 ? field.age_ms : null;
  const stale = age === null || age > 5000;
  const value = typeof field.value === "object" ? JSON.stringify(field.value) : String(field.value);
  return {
    value,
    badge: stale ? "stale" : "observed",
    source: typeof field.source === "string" ? field.source : "unavailable",
    time: Number.isFinite(field.at_ms) ? field.at_ms : null,
    age
  };
}

function dossierPresentation(payload, instance, entityId) {
  if (!validEnvelope(payload, instance, entityId)) throw new Error("Invalid dossier response");
  const selected = payload.context.selected_entity_id?.value;
  const current = payload.context.current === true && selected === entityId;
  return {
    status: current ? "current" : "stale",
    entityId,
    observedAt: Number.isFinite(payload.context.observed_at_ms)
      ? payload.context.observed_at_ms : null,
    groups: FIELD_GROUPS.map(([title, fields]) => ({
      title,
      fields: fields.map(([label, path]) => ({
        label, path,
        display: current ? fieldDisplay(readField(payload.data, path))
          : { value: "Unknown", badge: "stale", source: "target_changed", time: null }
      }))
    })),
    gaps: payload.gaps.map(gap => ({
      source: String(gap.source ?? "unknown"),
      code: String(gap.code ?? "unavailable"),
      detail: String(gap.detail ?? "")
    }))
  };
}

function createDossierController({ instance, fetchDossier, onChange }) {
  let entityId = null;
  let sequence = 0;
  let active = null;
  let snapshot = { phase: "empty", entityId: null, model: null, error: null };
  function publish(next) {
    snapshot = next;
    onChange(snapshot);
  }
  function cancel() {
    sequence += 1;
    active?.abort();
    active = null;
  }
  async function refresh() {
    cancel();
    if (entityId === null) {
      publish({ phase: "empty", entityId: null, model: null, error: null });
      return;
    }
    const request = new AbortController();
    active = request;
    const ticket = sequence;
    publish({ phase: "loading", entityId, model: null, error: null });
    try {
      const payload = await fetchDossier(entityId, request.signal);
      if (ticket !== sequence || request.signal.aborted) return;
      const model = dossierPresentation(payload, instance, entityId);
      publish({ phase: model.status, entityId, model, error: null });
    } catch (error) {
      if (ticket !== sequence || request.signal.aborted) return;
      publish({ phase: "error", entityId, model: null, error: error.message });
    } finally {
      if (ticket === sequence) active = null;
    }
  }
  function select(next, load = true) {
    if (next !== null && (!Number.isInteger(next) || next <= 0 || next > 0xffffffff)) {
      throw new Error("Invalid target entity ID");
    }
    if (next !== entityId) {
      cancel();
      entityId = next;
      publish({ phase: "empty", entityId, model: null, error: null });
    }
    if (next === null) return;
    if (load) return refresh();
  }
  return { select, refresh, get snapshot() { return snapshot; } };
}

function addText(document, parent, tag, value, className) {
  const element = document.createElement(tag);
  element.textContent = value;
  if (className) element.className = className;
  parent.appendChild(element);
  return element;
}

function renderDossier(document, snapshot) {
  const status = document.getElementById("dossier-status");
  const content = document.getElementById("dossier-content");
  content.replaceChildren();
  status.dataset.state = snapshot.phase;
  if (snapshot.phase === "empty") {
    status.textContent = "Enter a runtime entity ID to inspect its dossier.";
    return;
  }
  if (snapshot.phase === "loading") {
    status.textContent = `Loading entity ${snapshot.entityId}…`;
    return;
  }
  if (snapshot.phase === "error") {
    status.textContent = `Entity ${snapshot.entityId}: read unavailable (${snapshot.error}).`;
    return;
  }
  const model = snapshot.model;
  status.textContent = model.status === "current"
    ? `Entity ${model.entityId}: selection matched at sample time`
    : `Entity ${model.entityId}: stale selection; values hidden`;
  status.dataset.state = model.status;
  if (model.observedAt !== null) {
    addText(document, content, "p", `Dossier sampled ${new Date(model.observedAt).toLocaleString()}`, "sample-time");
  }
  for (const group of model.groups) {
    const section = addText(document, content, "section", "", "dossier-group");
    addText(document, section, "h3", group.title);
    const list = addText(document, section, "dl", "", "dossier-fields");
    for (const field of group.fields) {
      const row = addText(document, list, "div", "", "dossier-field");
      addText(document, row, "dt", field.label);
      addText(document, row, "dd", field.display.value);
      const badge = addText(document, row, "span", field.display.badge, `badge badge-${field.display.badge}`);
      badge.setAttribute("aria-label", `${field.label}: ${field.display.badge}`);
      let provenance = field.display.source;
      if (field.display.time !== null) {
        provenance += ` · ${new Date(field.display.time).toLocaleString()}`;
      }
      if (field.display.age !== undefined && field.display.age !== null) {
        provenance += ` · ${Math.round(field.display.age / 1000)}s old`;
      }
      addText(document, row, "small", provenance, "provenance");
    }
  }
  if (model.gaps.length) {
    const section = addText(document, content, "section", "", "dossier-gaps");
    addText(document, section, "h3", "Missing evidence");
    const list = addText(document, section, "ul", "");
    for (const gap of model.gaps) {
      addText(document, list, "li", `${gap.source}: ${gap.code}${gap.detail ? ` — ${gap.detail}` : ""}`);
    }
  }
}

const exported = { createDossierController, dossierPresentation, fieldDisplay, renderDossier };
if (typeof module !== "undefined") module.exports = exported;
if (typeof window !== "undefined") window.CimmeriaDossier = exported;
