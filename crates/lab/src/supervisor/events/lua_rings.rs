//! The lab's two Lua rings inside the client: combat text and chat lines.
//!
//! The stock UI already receives both, as `Events.UnitCombat` (subscribed
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

use serde_json::{json, Value};

/// Records returned per ring per pump.
pub const READ_MAX: u32 = 200;
/// Records the in-client rings keep.
pub const RING_CAP: u32 = 512;

/// Field separator inside one record line (a line is tab-separated; the
/// stat list of a combat record uses record/unit separators).
const RS: char = '\u{1e}';
const US: char = '\u{1f}';

/// Build the install-and-read chunk.
pub fn pump_chunk(combat_after: u64, chat_after: u64, max: u32) -> String {
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
  if ChatMod.onMessageReceived == L.chat_wrapper then st_chat = "ok" else
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
    local oks = pcall(function() Inst1ChatWin:unsubscribe(Events.MessageReceived) Inst1ChatWin:subscribe(Events.MessageReceived, "ChatMod.onMessageReceived") end)
    st_chat = oks and "installed" or "installed_no_resubscribe"
  end
end
local out = {{}}
local function add(...) out[#out + 1] = table.concat({{...}}, "\t") end
add("epoch", clean(L.epoch))
add("install", "combat", st_combat)
add("install", "chat", st_chat)
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
return unpack(out)"#,
        cap = RING_CAP,
    )
}

/// One pump's read of the Lua rings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RingRead {
    pub epoch: String,
    /// `(ring, status)`: `ok` (already ours), `installed`,
    /// `installed_no_resubscribe`, or `missing` (the stock handler is not
    /// loaded: the client is not in the world yet).
    pub install: Vec<(String, String)>,
    /// `(ring seq, fields)` per record.
    pub combat: Vec<(u64, Value)>,
    pub chat: Vec<(u64, Value)>,
    pub combat_head: u64,
    pub chat_head: u64,
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
        let c = pump_chunk(12, 34, 50);
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
