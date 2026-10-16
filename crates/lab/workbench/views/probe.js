"use strict";

function validEnvelope(payload, instance) {
  return payload?.schema_version === 1 && Array.isArray(payload.gaps)
    && payload.context?.instance?.toLowerCase() === instance.toLowerCase()
    && payload.data && typeof payload.data === "object";
}

function emptyDraft() {
  return { name: "", mode: "read", chunk: "", favorite: false };
}

function draftOf(definition) {
  return definition ? {
    name: definition.name, mode: definition.mode,
    chunk: definition.chunk, favorite: definition.favorite === true
  } : emptyDraft();
}

function sameDraft(left, right) {
  return left.name === right.name && left.mode === right.mode
    && left.chunk === right.chunk && left.favorite === right.favorite;
}

function resultPresentation(data) {
  const result = data?.result;
  if (!result || typeof result.status !== "string") throw new Error("Invalid probe result");
  const partial = result.status === "partial" || result.status === "truncated"
    || result.gaps?.includes("tree_truncated") || result.value?.kind === "truncated"
    || data.gaps?.length > 0;
  return {
    status: result.status, partial,
    detail: typeof result.detail === "string" ? result.detail : "",
    value: result.value ?? null,
    gaps: [...(result.gaps ?? []), ...(data.gaps ?? []).map(gap => gap.reason ?? gap.code ?? "gap")]
  };
}

function createProbeController({ instance, api, onChange }) {
  let revision = 0;
  let csrf = null;
  let leaseId = "";
  let definitions = [];
  let selected = null;
  let draft = emptyDraft();
  let phase = "empty";
  let error = "";
  let result = null;
  let browser = null;
  function original() { return definitions.find(def => def.name === selected) ?? null; }
  function snapshot() {
    return {
      definitions: definitions.map(def => ({ name: def.name, mode: def.mode, favorite: def.favorite })),
      selected, draft: { ...draft }, dirty: !sameDraft(draft, draftOf(original())),
      phase, error, result, browser, leaseReady: leaseId.trim().length > 0
    };
  }
  function publish() { onChange(snapshot()); }
  function fail(message) { phase = "error"; error = message; publish(); }
  function checkEnvelope(payload) {
    if (!validEnvelope(payload, instance)) throw new Error("Invalid probe response");
    return payload.data;
  }
  function requireLease() {
    const value = leaseId.trim();
    if (!value) throw new Error("Enter a current lab lease ID before running or browsing");
    return value;
  }
  async function load(force = false) {
    if (snapshot().dirty && !force) throw new Error("Unsaved changes; discard them before reloading");
    const ticket = ++revision;
    phase = "loading"; error = ""; result = null; browser = null; publish();
    try {
      const [session, listed] = await Promise.all([api.session(), api.list()]);
      if (ticket !== revision) return;
      if (session?.instance?.toLowerCase() !== instance.toLowerCase() || typeof session.csrf !== "string") {
        throw new Error("Invalid workbench session");
      }
      const data = checkEnvelope(listed);
      if (!Array.isArray(data.definitions)) throw new Error("Invalid probe catalog");
      csrf = session.csrf;
      definitions = data.definitions;
      selected = definitions.some(def => def.name === selected) ? selected : null;
      draft = draftOf(original());
      phase = "ready"; publish();
    } catch (cause) {
      if (ticket === revision) fail(cause.message);
    }
  }
  function select(name) {
    if (snapshot().dirty) throw new Error("Unsaved changes; discard them before switching probes");
    if (name !== null && !definitions.some(def => def.name === name)) throw new Error("Unknown probe");
    revision += 1;
    selected = name; draft = draftOf(original()); result = null; browser = null;
    phase = "ready"; error = ""; publish();
  }
  function edit(patch) {
    revision += 1;
    draft = { ...draft, ...patch };
    result = null; browser = null; phase = "ready"; error = ""; publish();
  }
  function setLeaseId(value) {
    if (value !== leaseId) {
      if (phase === "running" || phase === "browsing") revision += 1;
      if (phase === "running" || phase === "browsing" || phase === "result" || phase === "browser") {
        phase = "ready";
      }
      result = null; browser = null;
    }
    leaseId = value; publish();
  }
  async function save() {
    if (!csrf) { fail("Workbench session unavailable; reload probes"); return; }
    const ticket = ++revision;
    phase = "saving"; error = ""; publish();
    try {
      const previous = original();
      const data = checkEnvelope(await api.save({
        instance, ...draft,
        expected_sha256: previous?.name === draft.name ? previous.sha256 : null
      }, csrf));
      if (ticket !== revision) return;
      const saved = data.definition;
      if (!saved || saved.name !== draft.name || typeof saved.sha256 !== "string") {
        throw new Error("Invalid saved probe");
      }
      definitions = [...definitions.filter(def => def.name !== saved.name), saved]
        .sort((left, right) => left.name.localeCompare(right.name));
      selected = saved.name; draft = draftOf(saved);
      phase = "saved"; publish();
    } catch (cause) { if (ticket === revision) fail(cause.message); }
  }
  async function run(parameters = {}) {
    let lease;
    try { lease = requireLease(); } catch (cause) { fail(cause.message); return; }
    const definition = original();
    if (!definition || snapshot().dirty) { fail("Save the current probe before running it"); return; }
    if (!csrf) { fail("Workbench session unavailable; reload probes"); return; }
    const ticket = ++revision;
    phase = "running"; error = ""; result = null; publish();
    try {
      const payload = await api.run({
        instance, lease_id: lease, name: definition.name,
        sha256: definition.sha256, parameters
      }, csrf);
      const data = checkEnvelope(payload);
      if (ticket !== revision) return;
      result = resultPresentation({ ...data, gaps: payload.gaps });
      phase = "result"; publish();
    } catch (cause) { if (ticket === revision) fail(cause.message); }
  }
  async function browse(kind, name) {
    let lease;
    try { lease = requireLease(); } catch (cause) { fail(cause.message); return; }
    if (!csrf) { fail("Workbench session unavailable; reload probes"); return; }
    if (kind !== "globals" && kind !== "window") { fail("Unknown object browser"); return; }
    const ticket = ++revision;
    phase = "browsing"; error = ""; browser = null; publish();
    try {
      const payload = await api.browse(kind, { instance, lease_id: lease, name }, csrf);
      const data = checkEnvelope(payload);
      if (ticket !== revision) return;
      browser = { kind, name, source: data.source ?? "unknown",
        result: data.error ? { status: data.error, detail: data.detail ?? "", partial: true, value: null, gaps: [] }
          : kind === "window"
            ? { status: "ok", detail: "", partial: payload.gaps.length > 0 || data.result?.truncated === true,
              value: data.result, gaps: payload.gaps.map(gap => gap.reason ?? "gap") }
            : resultPresentation({ result: data.result, gaps: payload.gaps }) };
      phase = "browser"; publish();
    } catch (cause) { if (ticket === revision) fail(cause.message); }
  }
  return { load, select, edit, setLeaseId, save, run, browse, get snapshot() { return snapshot(); } };
}

function addText(document, parent, tag, value, className) {
  const element = document.createElement(tag);
  element.textContent = String(value);
  if (className) element.className = className;
  parent.appendChild(element);
  return element;
}

function renderTree(document, parent, value, depth = 0) {
  if (depth > 6) { addText(document, parent, "span", "Truncated: display depth limit"); return; }
  if (!value || typeof value !== "object") {
    addText(document, parent, "span", String(value)); return;
  }
  if (value.kind === "table" && Array.isArray(value.items)) {
    const list = addText(document, parent, "ul", "", "probe-tree");
    for (const item of value.items.slice(0, 128)) {
      const row = addText(document, list, "li", "");
      renderTree(document, row, item.key, depth + 1);
      addText(document, row, "span", ": ");
      renderTree(document, row, item.value, depth + 1);
    }
    if (value.items.length > 128) addText(document, parent, "span", "Truncated: display node limit");
  } else if (value.kind === "cycle" || value.kind === "truncated") {
    addText(document, parent, "span", `${value.kind}: ${value.reason ?? "bounded tree"}`, "badge badge-stale");
  } else if (value.kind) {
    addText(document, parent, "span", `${value.kind}: ${value.value ?? ""}`);
  } else {
    addText(document, parent, "pre", JSON.stringify(value, null, 2));
  }
}

function renderResult(document, parent, title, result) {
  const section = addText(document, parent, "section", "", "probe-output");
  addText(document, section, "h3", title);
  addText(document, section, "p", `${result.status}${result.partial ? " · partial/truncated" : ""}`,
    result.partial || result.status !== "ok" ? "badge badge-stale" : "badge badge-observed");
  if (result.detail) addText(document, section, "p", result.detail);
  if (result.gaps.length) addText(document, section, "p", `Gaps: ${result.gaps.join(", ")}`);
  if (result.value !== null) renderTree(document, section, result.value);
}

function renderProbe(document, state) {
  const status = document.getElementById("probe-status");
  const list = document.getElementById("probe-list");
  const output = document.getElementById("probe-output");
  status.textContent = state.error || (state.dirty ? "Unsaved edits" : state.phase);
  status.dataset.state = state.phase;
  list.replaceChildren(); output.replaceChildren();
  const blank = addText(document, list, "option", "New probe");
  blank.value = "";
  for (const def of state.definitions) {
    const option = addText(document, list, "option", `${def.favorite ? "★ " : ""}${def.name} · ${def.mode}`);
    option.value = def.name;
  }
  list.value = state.selected ?? "";
  for (const [id, value] of [["probe-name", state.draft.name], ["probe-mode", state.draft.mode],
    ["probe-chunk", state.draft.chunk]]) {
    const input = document.getElementById(id);
    if (input.value !== value) input.value = value;
  }
  document.getElementById("probe-favorite").checked = state.draft.favorite;
  document.getElementById("probe-mode-badge").textContent = state.draft.mode === "mutate"
    ? "MUTATE · may change client state" : "READ · source can still have side effects";
  document.getElementById("probe-run").disabled = !state.leaseReady || state.dirty || !state.selected;
  document.getElementById("probe-browse").disabled = !state.leaseReady;
  if (state.result) renderResult(document, output, "Probe result", state.result);
  if (state.browser) renderResult(document, output,
    `${state.browser.kind}: ${state.browser.name} · ${state.browser.source}`, state.browser.result);
}

function browserApi(instance) {
  const base = `/workbench/api/v1/probes`;
  async function request(path, body, csrf) {
    const response = await fetch(`${base}${path}?instance=${encodeURIComponent(instance)}`, {
      method: body ? "POST" : "GET", credentials: "same-origin", cache: "no-store",
      headers: body ? { "Content-Type": "application/json", "x-cimmeria-csrf": csrf } : {},
      body: body ? JSON.stringify(body) : undefined
    });
    const payload = await response.json().catch(() => null);
    if (!response.ok) throw new Error(payload?.data?.detail ?? payload?.data?.error ?? `HTTP ${response.status}`);
    return payload;
  }
  return {
    session: async () => {
      const response = await fetch(`/workbench/session?instance=${encodeURIComponent(instance)}`,
        { credentials: "same-origin", cache: "no-store" });
      if (!response.ok) throw new Error(`Session HTTP ${response.status}`);
      return response.json();
    },
    list: () => request(""),
    save: (body, csrf) => request("/save", body, csrf),
    run: (body, csrf) => request("/run", body, csrf),
    browse: (kind, body, csrf) => request(`/${kind}`, body, csrf)
  };
}

function mountProbe(document, instance) {
  const controller = createProbeController({ instance, api: browserApi(instance),
    onChange: state => renderProbe(document, state) });
  const byId = id => document.getElementById(id);
  function showError(cause) { byId("probe-status").textContent = cause.message; }
  byId("probe-list").addEventListener("change", event => {
    try { controller.select(event.target.value || null); }
    catch (cause) { renderProbe(document, controller.snapshot); showError(cause); }
  });
  byId("probe-new").addEventListener("click", () => {
    try { controller.select(null); } catch (cause) { showError(cause); }
  });
  byId("probe-reload").addEventListener("click", () => {
    if (controller.snapshot.dirty && !window.confirm("Discard unsaved probe edits?")) return;
    controller.load(true);
  });
  for (const [id, key, read] of [
    ["probe-name", "name", input => input.value], ["probe-mode", "mode", input => input.value],
    ["probe-chunk", "chunk", input => input.value], ["probe-favorite", "favorite", input => input.checked]
  ]) byId(id).addEventListener("input", event => controller.edit({ [key]: read(event.target) }));
  byId("probe-lease").addEventListener("input", event => controller.setLeaseId(event.target.value));
  byId("probe-save").addEventListener("click", () => controller.save());
  byId("probe-run").addEventListener("click", () => {
    let parameters;
    try {
      parameters = JSON.parse(byId("probe-parameters").value || "{}");
      if (!parameters || Array.isArray(parameters) || typeof parameters !== "object") throw new Error("Parameters must be a JSON object");
    } catch (cause) { showError(cause); return; }
    if (controller.snapshot.draft.mode === "mutate" && !window.confirm("Run a mutating Lua probe?")) return;
    controller.run(parameters);
  });
  byId("probe-browse").addEventListener("click", () => controller.browse(
    byId("probe-browse-kind").value, byId("probe-browse-name").value));
  return controller;
}

const exported = { createProbeController, resultPresentation, renderTree, renderProbe, mountProbe };
if (typeof module !== "undefined") module.exports = { ...exported, browserApi };
if (typeof window !== "undefined") window.CimmeriaProbe = exported;
