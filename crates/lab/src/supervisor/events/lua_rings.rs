//! The lab's three Lua rings inside the client: combat text, chat lines
//! and CEGUI window clicks.
//!
//! The stock UI already receives the first two, as `Events.UnitCombat` (subscribed
//! by `SCTMod.onUnitCombat`, the floating combat text) and
//! `Events.MessageReceived` (subscribed by `ChatMod.onMessageReceived`).
//! The lab wraps those two handlers: the wrapper appends a record to a
//! bounded ring in `_G.CimmeriaLab` and then calls the original, so the
//! player-visible UI is unchanged. It reads the raw event arguments, not
//! the rendered text, so the SCT verbosity option cannot filter it.
//!
//! Install and read are one chunk, run on every pump: it (re)installs a
//! wrapper whenever the stock function is not ours (an interface reload
//! redefines `SCTMod`), then returns the records after the supervisor's
//! last-read seq. The ring carries an `epoch` so a rebuilt Lua state (its
//! ring restarts at seq 1) is recognised.
//!
//! Why the unsubscribe/subscribe after wrapping: the stock code subscribes
//! by *name* (`'SCTMod.onUnitCombat'`); if the event system caches the
//! resolved function on first fire, replacing the global alone would never
//! be called. Re-subscribing the same name makes it resolve again, and is
//! harmless if the lookup happens per fire.
//!
//! A wrapper that finds itself nested (someone else wrapped ours and we
//! wrapped theirs again) records once: `busy` guards re-entry.
//!
//! # The click ring (LR-10)
//!
//! A title-bar X or a tab click sends no server call, so nothing else the
//! lab reads sees it. The click ring subscribes one handler to the CEGUI
//! global event `Window/MouseClick` through the client's own Lua binding,
//! `window:subscribeGlobal(namespace, event, 'Handler.name')`: three Lua
//! strings (the tolua stub in SGW.exe, error text
//! `error in function 'subscribeGlobal'`), used by the stock chat window
//! for `RichTextItemEntry/LinkClicked` (`ChatWindow.lua:1317`). A global
//! handler is called `(this, eventWindow, ...args)`, the MouseEventArgs
//! unpacked as for a per-window `EventMouseClick` (`position, moveDelta,
//! button, ...`; `ChatMod.onTabButtonClicked`). Evidence and limits:
//! `docs/analysis/lab-record/worknotes/LR-10.md`.
//!
//! Each record is `{ window, button, mouse }`: `button` is the clicked
//! window's name (`TutorialWin__auto_closebutton__`), `window` the
//! top-level window holding it (the ancestor whose parent is `Root`, the
//! same names `client_ui_state` lists), `mouse` `left`/`right`/`middle`.
//! It is the lab's injected Lua only: nothing ships to players, nothing in
//! the client is patched, and the handler returns nothing, so it never
//! marks the click handled and the stock UI sees every click as before.
//!
//! The binding has no `unsubscribeGlobal`, so the subscription is made
//! once per Lua state (`L.click_host`), never per pump: re-subscribing would
//! stack handlers. An interface reload rebuilds the Lua state (a new
//! `epoch`), which installs it again. The hosts are tried in order,
//! `BackgroundWin` then `Root`, moving on when the subscribe itself fails
//! (`Root` is not a child of a ScriptedWindow, so it raises; the global
//! event fires for every window's click whichever window hosts it). The
//! status names the host used, or the error text of every host that
//! failed. In CEGUI 0.6 an
//! unhandled `MouseClick` bubbles to the parents, firing the global event
//! once per ancestor; a fire on one of the last record's ancestors (never
//! the clicked window itself), at the same UI time and cursor position, is
//! that bubble and is skipped, and a fire on `Root` is never a click. The
//! click install runs under its own `pcall`, so a fault there cannot stop
//! the combat and chat rings. The chunk is never run by the Rust tests (no
//! Lua runtime in the workspace); it was run under Lua 5.1 with stub
//! windows when written (LR-10 worknote).

use serde_json::{json, Value};

/// Records returned per ring per pump.
pub const READ_MAX: u32 = 200;
/// Records the in-client rings keep.
pub const RING_CAP: u32 = 512;

/// Field separator inside one record line (a line is tab-separated; the
/// stat list of a combat record uses record/unit separators).
const RS: char = '\u{1e}';
const US: char = '\u{1f}';

/// Handler name the click ring subscribes by (resolved from `_G`).
pub const CLICK_HANDLER: &str = "CimmeriaLab.on_click";

/// Build the install-and-read chunk.
pub fn pump_chunk(combat_after: u64, chat_after: u64, click_after: u64, max: u32) -> String {
    format!(
        r#"local L = rawget(_G, "CimmeriaLab")
if type(L) ~= "table" then L = {{}} _G.CimmeriaLab = L end
if not L.epoch then
  local okt, t = pcall(getSystemTime)
  L.epoch = string.gsub(tostring(L), "table: ", "") .. "-" .. tostring(okt and t or 0)
end
local CAP = {cap}
local function clean(s) if s == nil then return "" end return (string.gsub(tostring(s), "%c", " ")) end
local function enum_name(tbl, v)
  if type(tbl) ~= "table" then return "" end
  for k, x in pairs(tbl) do if x == v then return tostring(k) end end
  return ""
end
local function ring(name)
  local r = L[name]
  if type(r) ~= "table" then r = {{ seq = 0, items = {{}} }} L[name] = r end
  return r
end
local function push(r, e)
  r.seq = r.seq + 1 e.seq = r.seq
  r.items[#r.items + 1] = e
  while #r.items > CAP do table.remove(r.items, 1) end
end
local function now() local ok, t = pcall(getSystemTime) return ok and t or 0 end
L.record_combat = function(abilityId, hitType, statList)
  local e = {{ ability = tostring(abilityId), hit = tostring(hitType), t = now() }}
  e.hit_name = enum_name(HitType, hitType)
  local oki, info = pcall(getAbilityInfo, abilityId)
  e.ability_name = (oki and type(info) == "table" and info.name) or ""
  local function nm(slot) local ok, n = pcall(unitName, slot) return ok and n or "" end
  local function isp(slot) local ok, v = pcall(unitsEqual, slot, Unit.Player) return (ok and v) and "1" or "0" end
  e.source = nm(Unit.EffectSource) e.target = nm(Unit.EffectTarget)
  e.sp = isp(Unit.EffectSource) e.tp = isp(Unit.EffectTarget)
  local parts, mortal = {{}}, "0"
  if type(statList) == "table" then
    for id, s in pairs(statList) do
      if type(s) == "table" then
        if Stat ~= nil and StatResultType ~= nil and id == Stat.Health and s.resultCode == StatResultType.Mortal then mortal = "1" end
        parts[#parts + 1] = table.concat({{ clean(id), enum_name(Stat, id), clean(s.value), clean(s.resultCode), enum_name(StatResultType, s.resultCode), clean(s.damageCode) }}, "\31")
      end
    end
  end
  e.stats = table.concat(parts, "\30") e.mortal = mortal
  push(ring("combat"), e)
end
L.record_chat = function(speaker, flags, channelId, channelName, text)
  push(ring("chat"), {{ t = now(), speaker = clean(speaker), flags = clean(flags), channel = clean(channelId), channel_name = clean(channelName), text = clean(text) }})
end
local st_combat, st_chat = "missing", "missing"
if type(SCTMod) == "table" and type(SCTMod.onUnitCombat) == "function" then
  if SCTMod.onUnitCombat == L.combat_wrapper then st_combat = "ok" else
    local orig = SCTMod.onUnitCombat
    L.combat_wrapper = function(window, abilityId, hitType, statList)
      if not L.combat_busy then
        L.combat_busy = true
        pcall(L.record_combat, abilityId, hitType, statList)
        local ok, err = pcall(orig, window, abilityId, hitType, statList)
        L.combat_busy = false
        if not ok then error(err, 0) end
        return
      end
      return orig(window, abilityId, hitType, statList)
    end
    SCTMod.onUnitCombat = L.combat_wrapper
    local oks = pcall(function() SCTWin:unsubscribe(Events.UnitCombat) SCTWin:subscribe(Events.UnitCombat, "SCTMod.onUnitCombat") end)
    st_combat = oks and "installed" or "installed_no_resubscribe"
  end
end
if type(ChatMod) == "table" and type(ChatMod.onMessageReceived) == "function" then
  local wrapped = ChatMod.onMessageReceived == L.chat_wrapper
  if not wrapped then
    local orig = ChatMod.onMessageReceived
    L.chat_wrapper = function(this, speaker, flags, channelId, channelName, text)
      if not L.chat_busy then
        L.chat_busy = true
        pcall(L.record_chat, speaker, flags, channelId, channelName, text)
        local ok, err = pcall(orig, this, speaker, flags, channelId, channelName, text)
        L.chat_busy = false
        if not ok then error(err, 0) end
        return
      end
      return orig(this, speaker, flags, channelId, channelName, text)
    end
    ChatMod.onMessageReceived = L.chat_wrapper
  end
  local host = Inst1ChatWin
  if host == nil then
    st_chat = "missing_window"
  elseif host == L.chat_host and wrapped then
    st_chat = "ok"
  else
    local had_host = L.chat_host ~= nil
    -- The module and wrapper can outlive the CEGUI window they were first
    -- attached to. Rebind to a recreated window, without subscribing twice
    -- when the same window remains alive across pumps.
    if L.chat_host ~= nil then pcall(function() L.chat_host:unsubscribe(Events.MessageReceived) end) end
    local oks = pcall(function() host:unsubscribe(Events.MessageReceived) host:subscribe(Events.MessageReceived, "ChatMod.onMessageReceived") end)
    if oks then
      L.chat_host = host
      L.chat_gap = had_host and "resubscribed" or "installed"
    end
    st_chat = oks and (had_host and "resubscribed" or "installed") or "installed_no_resubscribe"
  end
end
if L.chat_gap ~= nil then
  st_chat = "gap_" .. L.chat_gap
  if {max} > 0 then L.chat_gap = nil end
end
L.chat_install_status = st_chat
local function wname(w)
  if w == nil then return nil end
  local ok, n = pcall(function() return w:getName() end)
  if ok and n ~= nil then return tostring(n) end
  return nil
end
local function wparent(w)
  local ok, p = pcall(function() return w:getParent() end)
  if ok then return p end
  return nil
end
local function mouse_name(b)
  if b == nil then return "" end
  if type(CEGUI) == "table" then
    if b == CEGUI.LeftButton then return "left" end
    if b == CEGUI.RightButton then return "right" end
    if b == CEGUI.MiddleButton then return "middle" end
  end
  return clean(b)
end
local function pos_key(pos)
  if pos == nil then return "" end
  local ok, k = pcall(function() return tostring(pos.x or pos.d_x) .. "," .. tostring(pos.y or pos.d_y) end)
  return ok and k or ""
end
L.record_click = function(w, mouse, pos)
  local name = wname(w)
  -- An unhandled click bubbling all the way up fires once more for Root; it is never a click of its own.
  if name == nil or name == "Root" then return end
  local chain, top, p, depth = {{ name }}, name, wparent(w), 0
  while p ~= nil and depth < 32 do
    local pn = wname(p)
    if pn == nil or pn == "Root" then break end
    top = pn chain[#chain + 1] = pn
    p = wparent(p) depth = depth + 1
  end
  -- A bubbled fire reaches an ancestor of the last click at the same instant and cursor position.
  -- Only ancestors (chain[2..]) are skipped, so two real clicks on one widget both record; with no
  -- clock (t == 0) the position alone has to tell them apart.
  local t, key = now(), pos_key(pos)
  local last = L.click_last
  if type(last) == "table" and last.key == key and (t == 0 or last.t == t) then
    for i = 2, #last.chain do if last.chain[i] == name then return end end
  end
  L.click_last = {{ t = t, key = key, chain = chain }}
  push(ring("click"), {{ t = t, window = clean(top), button = clean(name), mouse = mouse_name(mouse) }})
end
L.on_click = function(this, window, position, moveDelta, button)
  if L.click_busy then return end
  L.click_busy = true
  pcall(L.record_click, window, button, position)
  L.click_busy = false
end
local st_click = "missing"
-- Its own pcall: a fault in the click install must not cost the combat and chat rings their pump.
local okc, cerr = pcall(function()
  if L.click_host ~= nil then st_click = "ok host=" .. tostring(L.click_host_name) return end
  local ns, ev = "Window", "MouseClick"
  if type(CEGUI) == "table" and type(CEGUI.Window) == "table" then
    ns = CEGUI.Window.EventNamespace or ns
    ev = CEGUI.Window.EventMouseClick or ev
  end
  -- Hosts in order. "Root" is not a child of a ScriptedWindow, so its subscribe raises; BackgroundWin
  -- takes it, and the global event fires for every window's click whichever window hosts it.
  -- L.on_click is defined above, before any subscribe: the handler is bound by name at subscribe time.
  local errs = {{}}
  for _, hn in ipairs({{ "BackgroundWin", "Root" }}) do
    local okw, host = pcall(function() return rawget(_G, hn) or getWindow(hn) end)
    if not okw then
      errs[#errs + 1] = hn .. "=" .. string.sub(clean(host), 1, 120)
    elseif host == nil then
      errs[#errs + 1] = hn .. "=no window"
    else
      local oks, serr = pcall(function() host:subscribeGlobal(ns, ev, "{click_handler}") end)
      if oks then
        L.click_host = host
        L.click_host_name = hn
        st_click = "installed host=" .. hn
        return
      end
      errs[#errs + 1] = hn .. "=" .. string.sub(clean(serr), 1, 120)
    end
  end
  st_click = "failed: " .. table.concat(errs, "; ")
end)
if not okc then st_click = "failed: " .. string.sub(clean(cerr), 1, 120) end
local out = {{}}
local function add(...) out[#out + 1] = table.concat({{...}}, "\t") end
add("epoch", clean(L.epoch))
add("install", "combat", st_combat)
add("install", "chat", st_chat)
add("install", "click", st_click)
local c, n = ring("combat"), 0
for _, e in ipairs(c.items) do
  if e.seq > {combat_after} and n < {max} then
    n = n + 1
    add("combat", e.seq, clean(e.t), e.ability, clean(e.ability_name), e.hit, e.hit_name, clean(e.source), e.sp, clean(e.target), e.tp, e.mortal, e.stats)
  end
end
add("head", "combat", c.seq)
local h, m = ring("chat"), 0
for _, e in ipairs(h.items) do
  if e.seq > {chat_after} and m < {max} then
    m = m + 1
    add("chat", e.seq, clean(e.t), e.channel, e.channel_name, e.speaker, e.flags, e.text)
  end
end
add("head", "chat", h.seq)
local k, q = ring("click"), 0
for _, e in ipairs(k.items) do
  if e.seq > {click_after} and q < {max} then
    q = q + 1
    add("click", e.seq, clean(e.t), e.window, e.button, e.mouse)
  end
end
add("head", "click", k.seq)
return unpack(out)"#,
        cap = RING_CAP,
        click_handler = CLICK_HANDLER,
    )
}

/// One pump's read of the Lua rings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RingRead {
    pub epoch: String,
    /// `(ring, status)`: `ok` (already ours), `installed`,
    /// `installed_no_resubscribe`, or `missing` (the stock handler is not
    /// loaded: the client is not in the world yet). The click ring reports
    /// `ok host=<name>`, `installed host=<name>`, `failed: <host>=<error>; ...`
    /// (one entry per host tried, with its Lua error text) or `missing`.
    pub install: Vec<(String, String)>,
    /// `(ring seq, fields)` per record.
    pub combat: Vec<(u64, Value)>,
    pub chat: Vec<(u64, Value)>,
    /// Click records: `window`, `button`, `mouse` (LR-10).
    pub click: Vec<(u64, Value)>,
    pub combat_head: u64,
    pub chat_head: u64,
    /// The click ring's latest seq, read or not.
    pub click_head: u64,
    /// Lines that did not parse (kept for the error report, never fatal).
    pub bad_lines: Vec<String>,
}

fn num(s: &str) -> Value {
    if let Ok(i) = s.parse::<i64>() {
        return json!(i);
    }
    match s.parse::<f64>() {
        Ok(f) if f.is_finite() => json!(f),
        _ => json!(s),
    }
}

/// Parse a combat record's stat list.
pub fn parse_stats(s: &str) -> Vec<Value> {
    s.split(RS)
        .filter(|r| !r.is_empty())
        .map(|r| {
            let f: Vec<&str> = r.split(US).collect();
            let get = |i: usize| f.get(i).copied().unwrap_or_default();
            json!({
                "stat_id": num(get(0)),
                "stat": get(1),
                "value": num(get(2)),
                "result": num(get(3)),
                "result_name": get(4),
                "damage_code": num(get(5)),
            })
        })
        .collect()
}

/// Parse [`pump_chunk`]'s results.
pub fn parse_ring_read(lines: &[String]) -> RingRead {
    let mut r = RingRead::default();
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        match f.as_slice() {
            ["epoch", e] => r.epoch = e.to_string(),
            ["install", ring, status] => r.install.push((ring.to_string(), status.to_string())),
            ["head", "combat", n] => r.combat_head = n.parse().unwrap_or(0),
            ["head", "chat", n] => r.chat_head = n.parse().unwrap_or(0),
            ["head", "click", n] => r.click_head = n.parse().unwrap_or(0),
            ["click", seq, t, window, button, mouse] => match seq.parse::<u64>() {
                Ok(seq) => r.click.push((
                    seq,
                    json!({
                        "window": window,
                        "button": button,
                        "mouse": mouse,
                        "ui_time": num(t),
                    }),
                )),
                Err(_) => r.bad_lines.push(line.clone()),
            },
            ["combat", seq, t, ability, ability_name, hit, hit_name, source, sp, target, tp, mortal, stats] => {
                match seq.parse::<u64>() {
                    Ok(seq) => r.combat.push((
                        seq,
                        json!({
                            "ability_id": num(ability),
                            "ability_name": ability_name,
                            "hit_type": num(hit),
                            "hit_name": hit_name,
                            "source": source,
                            "source_is_player": *sp == "1",
                            "target": target,
                            "target_is_player": *tp == "1",
                            "mortal": *mortal == "1",
                            "stats": parse_stats(stats),
                            "ui_time": num(t),
                        }),
                    )),
                    Err(_) => r.bad_lines.push(line.clone()),
                }
            }
            ["chat", seq, t, channel, channel_name, speaker, flags, text] => {
                match seq.parse::<u64>() {
                    Ok(seq) => r.chat.push((
                        seq,
                        json!({
                            "channel": num(channel),
                            "channel_name": channel_name,
                            "speaker": speaker,
                            "flags": num(flags),
                            "text": text,
                            "ui_time": num(t),
                        }),
                    )),
                    Err(_) => r.bad_lines.push(line.clone()),
                }
            }
            _ => r.bad_lines.push(line.clone()),
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn chunk_wraps_both_handlers_and_reads_after_the_marks() {
        let c = pump_chunk(12, 34, 56, 50);
        assert!(c.contains("SCTMod.onUnitCombat = L.combat_wrapper"));
        assert!(c.contains("ChatMod.onMessageReceived = L.chat_wrapper"));
        // Re-subscribes by the stock name so a cached lookup is refreshed.
        assert!(c.contains(r#"SCTWin:subscribe(Events.UnitCombat, "SCTMod.onUnitCombat")"#));
        assert!(c.contains("e.seq > 12 and n < 50"));
        assert!(c.contains("e.seq > 34 and m < 50"));
        // The original is always called after recording.
        assert!(c.contains("pcall(orig, window, abilityId, hitType, statList)"));
        // Lua sees the separators as decimal escapes.
        assert!(c.contains(r#""\31""#) && c.contains(r#""\30""#));
    }

    #[test]
    fn chunk_subscribes_the_click_ring_globally_and_reads_after_its_mark() {
        let c = pump_chunk(12, 34, 56, 50);
        // CEGUI global event Window/MouseClick, through the stock binding,
        // by the lab's handler name; bound constants preferred, literals
        // as the fallback.
        assert!(c.contains(r#"local ns, ev = "Window", "MouseClick""#));
        assert!(c.contains("ns = CEGUI.Window.EventNamespace or ns"));
        assert!(c.contains("ev = CEGUI.Window.EventMouseClick or ev"));
        assert!(c.contains(r#"host:subscribeGlobal(ns, ev, "CimmeriaLab.on_click")"#));
        assert!(c.contains("L.on_click = function(this, window, position, moveDelta, button)"));
        // The record goes into its own capped ring.
        assert!(c.contains(
            r#"push(ring("click"), { t = t, window = clean(top), button = clean(name), mouse = mouse_name(mouse) })"#
        ));
        // The top-level window is the ancestor below Root.
        assert!(c.contains(r#"if pn == nil or pn == "Root" then break end"#));
        // Read after the click mark, bounded, with its head and status.
        assert!(c.contains("e.seq > 56 and q < 50"));
        assert!(c.contains(r#"add("head", "click", k.seq)"#));
        assert!(c.contains(r#"add("install", "click", st_click)"#));
        // The handler never returns a value: the click is never marked
        // handled, so the stock UI still sees it.
        let handler = &c[c.find("L.on_click = function").unwrap()..];
        let handler = &handler[..handler.find("\nend").unwrap()];
        assert!(!handler.contains("return true"), "{handler}");
    }

    /// The binding has no `unsubscribeGlobal`: a second subscribe would
    /// stack a second handler. The chunk runs on every pump, so the
    /// subscription must be gated on the per-Lua-state `L.click_host`, which
    /// only a successful subscribe sets.
    #[test]
    fn the_click_install_is_idempotent() {
        let c = pump_chunk(0, 0, 0, READ_MAX);
        assert_eq!(c.matches("subscribeGlobal").count(), 1);
        let gate = c
            .find(r#"if L.click_host ~= nil then st_click = "ok host=" .. tostring(L.click_host_name) return end"#)
            .expect("install gate");
        let sub = c.find("host:subscribeGlobal").unwrap();
        let set = c
            .find("L.click_host = host")
            .expect("host recorded only on success");
        assert!(gate < sub && sub < set);
        // BackgroundWin is tried first and Root is only the fallback: Root is
        // not a child of a ScriptedWindow, so its subscribe raises.
        let hosts = c
            .find(r#"ipairs({ "BackgroundWin", "Root" })"#)
            .expect("hosts in order, BackgroundWin first");
        assert!(gate < hosts && hosts < sub);
        assert!(
            !c.contains(r#"pcall(getWindow, "Root")"#),
            "the Root-only host choice is back"
        );
        // The handler is defined before the subscribe (it is bound by name).
        assert!(c.find("L.on_click = function").unwrap() < sub);
        // The install has its own pcall, opened before the gate, so a fault
        // there cannot stop the combat and chat reads that follow.
        let guard = c
            .find("local okc, cerr = pcall(function()")
            .expect("install pcall");
        assert!(guard < gate && sub < c.find("add(\"head\", \"combat\"").unwrap());
    }

    /// A failed click install names every host it tried with the Lua error,
    /// and the host that worked (#1660, probes C-P4 and C-P7). `failed` alone
    /// gave the caller nothing to act on, and the Root-only choice failed
    /// on every client.
    #[test]
    fn a_failed_click_install_reports_each_hosts_error_text() {
        let c = pump_chunk(0, 0, 0, READ_MAX);
        // Failure carries the error text of each host, clipped to 120 chars.
        assert!(c.contains(r#"local oks, serr = pcall(function() host:subscribeGlobal(ns, ev, "CimmeriaLab.on_click") end)"#));
        assert!(c.contains(r#"errs[#errs + 1] = hn .. "=" .. string.sub(clean(serr), 1, 120)"#));
        assert!(c.contains(r#"st_click = "failed: " .. table.concat(errs, "; ")"#));
        assert!(c.contains(
            r#"if not okc then st_click = "failed: " .. string.sub(clean(cerr), 1, 120) end"#
        ));
        // Success names the host.
        assert!(c.contains(r#"st_click = "installed host=" .. hn"#));
        // The pump parses the new statuses as one field each.
        let r = parse_ring_read(&s(&[
            "install\tclick\tfailed: BackgroundWin=no window; Root=Window 'Root' is not a child of a ScriptedWindow",
            "install\tchat\tok",
        ]));
        assert_eq!(
            r.install[0],
            (
                "click".to_string(),
                "failed: BackgroundWin=no window; Root=Window 'Root' is not a child of a ScriptedWindow".to_string()
            )
        );
        assert!(r.bad_lines.is_empty());
        let r = parse_ring_read(&s(&["install\tclick\tinstalled host=BackgroundWin"]));
        assert_eq!(
            r.install,
            vec![(
                "click".to_string(),
                "installed host=BackgroundWin".to_string()
            )]
        );
    }

    /// A bubbled fire is skipped only on the last click's ancestors, never
    /// on the clicked window itself, so two real clicks on one button both
    /// record; a fire on Root is never a click.
    #[test]
    fn the_click_dedup_skips_ancestors_only() {
        let c = pump_chunk(0, 0, 0, READ_MAX);
        assert!(c.contains("if name == nil or name == \"Root\" then return end"));
        assert!(c.contains("last.key == key and (t == 0 or last.t == t)"));
        assert!(
            c.contains("for i = 2, #last.chain do if last.chain[i] == name then return end end")
        );
        assert!(
            !c.contains("ipairs(last.chain)"),
            "the clicked window itself must not dedup"
        );
    }

    #[test]
    fn click_records_parse_with_window_and_button() {
        let r = parse_ring_read(&s(&[
            "install\tclick\tinstalled",
            "click\t2\t812.25\tTutorialWin\tTutorialWin__auto_closebutton__\tleft",
            "click\t3\t813\tChatWin\tChatWin_Tab2\t",
            "click\tnope\t1\ta\tb\tleft",
            "head\tclick\t3",
        ]));
        assert_eq!(
            r.install,
            vec![("click".to_string(), "installed".to_string())]
        );
        assert_eq!(r.click_head, 3);
        assert_eq!(r.click.len(), 2);
        let (seq, ev) = &r.click[0];
        assert_eq!(*seq, 2);
        assert_eq!(ev["window"], "TutorialWin");
        assert_eq!(ev["button"], "TutorialWin__auto_closebutton__");
        assert_eq!(ev["mouse"], "left");
        assert_eq!(ev["ui_time"], 812.25);
        assert_eq!(r.click[1].1["mouse"], "");
        assert_eq!(r.bad_lines, vec!["click\tnope\t1\ta\tb\tleft".to_string()]);
    }

    #[test]
    fn combat_records_parse_with_their_stat_list() {
        let stats =
            format!("1{US}Health{US}-57{US}0{US}Normal{US}2{RS}2{US}Focus{US}-5{US}0{US}{US}0");
        let line = format!("combat\t3\t1234.5\t1100\tPistol Shot\t1\tHit\tLabone\t1\tCellblock Guard\t0\t0\t{stats}");
        let r = parse_ring_read(&s(&[
            "epoch\t0A1B2C3D-99.5",
            "install\tcombat\tinstalled",
            "install\tchat\tok",
            &line,
            "head\tcombat\t3",
            "head\tchat\t0",
        ]));
        assert_eq!(r.epoch, "0A1B2C3D-99.5");
        assert_eq!(
            r.install,
            vec![
                ("combat".to_string(), "installed".to_string()),
                ("chat".to_string(), "ok".to_string())
            ]
        );
        assert_eq!(r.combat_head, 3);
        let (seq, ev) = &r.combat[0];
        assert_eq!(*seq, 3);
        assert_eq!(ev["ability_id"], 1100);
        assert_eq!(ev["hit_name"], "Hit");
        assert_eq!(ev["source_is_player"], true);
        assert_eq!(ev["target"], "Cellblock Guard");
        assert_eq!(ev["mortal"], false);
        assert_eq!(ev["stats"][0]["stat"], "Health");
        assert_eq!(ev["stats"][0]["value"], -57);
        assert_eq!(ev["stats"][1]["result_name"], "");
        assert!(r.bad_lines.is_empty());
    }

    #[test]
    fn chat_records_keep_text_with_spaces() {
        let r = parse_ring_read(&s(&[
            "chat\t7\t88\t9\tFeedback\t\t0\tYou must have a target",
            "head\tchat\t7",
        ]));
        let (seq, ev) = &r.chat[0];
        assert_eq!(*seq, 7);
        assert_eq!(ev["channel"], 9);
        assert_eq!(ev["text"], "You must have a target");
        assert_eq!(r.chat_head, 7);
    }

    #[test]
    fn malformed_lines_are_kept_not_fatal() {
        let r = parse_ring_read(&s(&["combat\tnotanumber", "wat"]));
        assert_eq!(r.bad_lines.len(), 2);
        assert!(r.combat.is_empty());
    }

    #[test]
    fn empty_stat_list_is_empty() {
        assert!(parse_stats("").is_empty());
    }
}
