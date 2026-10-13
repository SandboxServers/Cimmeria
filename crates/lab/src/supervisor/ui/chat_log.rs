//! `client_chat_log`: every chat line the client showed, with its channel,
//! speaker, colour and tabs, read through a named cursor so a caller gets
//! only the lines it has not seen.
//!
//! There is one chat capture in the lab: the `chat.line` ring in
//! [`crate::supervisor::events::lua_rings`], which wraps
//! `ChatMod.onMessageReceived` (records, then calls the original) and
//! re-subscribes the chat window by name. Every line the chat window shows
//! goes through that function: network chat, `/tell` errors, feedback,
//! Server Message lines and combat chatter. This tool pumps that ring into
//! the supervisor's event store and reads the store's `chat.line` events
//! through its own cursor (`chat_log:<name>`), so it never steals lines from
//! `client_wait_event` or `client_events_read`.
//!
//! On top of the raw record it adds what the chat window would do with the
//! line, read from the stock module: the channel's name (`UIChannel`), the
//! speaker flag names (`UISpeakerFlags`), and the colour and output tabs the
//! chat window uses for that channel (`ChatMod.ChannelMap`,
//! `channelColors`, `channel2tabs`).
//!
//! Lines shown before the ring was first installed are not captured; read
//! `client_ui_state`'s chat tail for those. Centre-screen splash text does
//! not go through the chat handler and is not in this log.

use std::time::Instant;

use serde_json::{json, Map, Value};

use super::{memory, memory_key, stamp_read, Supervisor};
use crate::supervisor::events::store::{StoredEvent, STORE_CAP};
use crate::supervisor::events::KIND_CHAT;

/// Lua body: the channel and flag tables, and per channel the colour and
/// tabs the chat window uses. Returns
/// `{channels: {id: name}, chat_base, flags: {name: bit}, display: {id: {client_channel, colour, tabs}}}`.
pub const META_CHUNK: &str = r#"
local channels, flags, display = {}, {}, {}
if type(UIChannel) == 'table' then
  for k, v in pairs(UIChannel) do if type(v) == 'number' then channels[tostring(v)] = k end end
end
channels['9'] = channels['9'] or 'Feedback'
if type(UISpeakerFlags) == 'table' then
  for k, v in pairs(UISpeakerFlags) do if type(v) == 'number' then flags[k] = v end end
end
if type(ChatMod) == 'table' and type(UIChannel) == 'table' and type(UIChannel.Chat) == 'number' then
  for id = 0, UIChannel.Chat - 1 do
    pcall(function()
      local cch = ChatMod.ChannelMap[id](id, 0)
      local tabs = {}
      local l = ChatMod.channel2tabs[cch]
      if l then for w, _ in pairs(l) do tabs[#tabs + 1] = __jcall(function() return w:getName() end) end end
      display[tostring(id)] = { client_channel = cch, colour = ChatMod.channelColors[cch], tabs = tabs }
    end)
  end
end
return __jenc({ channels = channels, chat_base = UIChannel and UIChannel.Chat, flags = flags, display = display })"#;

/// Caller-side filters over the returned lines.
#[derive(Debug, Default, Clone)]
pub struct ChatFilter {
    /// Channel by name (`Say`, `Tell`, `Feedback`, `Server`, a custom
    /// channel name) or number, case-insensitive.
    pub channel: Option<String>,
    /// Substring of the text, case-insensitive.
    pub contains: Option<String>,
    /// Substring of the speaker, case-insensitive.
    pub speaker: Option<String>,
}

impl ChatFilter {
    pub fn matches(&self, line: &Value) -> bool {
        let lc = |v: &Value| v.as_str().map(str::to_ascii_lowercase);
        let has = |field: &Value, needle: &str| {
            lc(field).is_some_and(|s| s.contains(&needle.to_ascii_lowercase()))
        };
        if let Some(c) = &self.channel {
            let want = c.to_ascii_lowercase();
            let by_name = [&line["channel"], &line["channel_name"]]
                .iter()
                .any(|v| lc(v).as_deref() == Some(want.as_str()));
            let by_id = line["channel_id"].as_i64().map(|n| n.to_string()) == Some(want);
            if !by_name && !by_id {
                return false;
            }
        }
        if let Some(t) = &self.contains {
            if !has(&line["text"], t) {
                return false;
            }
        }
        if let Some(s) = &self.speaker {
            if !has(&line["speaker"], s) {
                return false;
            }
        }
        true
    }
}

/// One stored `chat.line` event as a chat line, enriched from `meta`.
pub fn enrich(ev: &StoredEvent, meta: &Value) -> Value {
    let f = &ev.fields;
    let id = f["channel"].as_i64();
    let key = id.map(|n| n.to_string()).unwrap_or_default();
    // Custom chat channels (id >= UIChannel.Chat) are named by the server;
    // the stock channels by the UIChannel table.
    let custom = matches!((id, meta["chat_base"].as_i64()), (Some(i), Some(b)) if i >= b);
    let channel = if custom {
        f["channel_name"].clone()
    } else {
        meta["channels"]
            .get(&key)
            .cloned()
            .unwrap_or_else(|| f["channel_name"].clone())
    };
    let flags = f["flags"].as_i64().unwrap_or(0);
    let mut flag_names: Vec<&str> = meta["flags"]
        .as_object()
        .map(|m| {
            m.iter()
                .filter(|(_, bit)| bit.as_i64().is_some_and(|b| b > 0 && flags & b == b))
                .map(|(n, _)| n.as_str())
                .collect()
        })
        .unwrap_or_default();
    flag_names.sort_unstable();
    let display = meta["display"].get(&key).cloned().unwrap_or(Value::Null);
    let mut line = Map::new();
    line.insert("seq".into(), json!(ev.seq));
    line.insert("ts_ms".into(), json!(ev.ts_ms));
    line.insert("channel_id".into(), f["channel"].clone());
    line.insert("channel".into(), channel);
    line.insert("channel_name".into(), f["channel_name"].clone());
    line.insert("speaker".into(), f["speaker"].clone());
    line.insert("speaker_flags".into(), json!(flags));
    line.insert("flag_names".into(), json!(flag_names));
    line.insert("text".into(), f["text"].clone());
    line.insert("colour".into(), display["colour"].clone());
    line.insert("tabs".into(), display["tabs"].clone());
    line.insert("ui_time".into(), f["ui_time"].clone());
    Value::Object(line)
}

/// A store read to shape: every stored event after `since` (all kinds),
/// the store's newest seq, and whether events after `since` were evicted.
#[derive(Debug, Clone, Copy)]
pub struct StoreRead<'a> {
    pub events: &'a [StoredEvent],
    pub since: u64,
    pub head: u64,
    pub gap: bool,
}

/// Shape a store read: keep the chat lines, filter, cap, and say where the
/// cursor goes next.
pub fn shape(read: StoreRead<'_>, meta: &Value, filter: &ChatFilter, max: usize) -> Value {
    let chat: Vec<Value> = read
        .events
        .iter()
        .filter(|e| e.kind == KIND_CHAT)
        .map(|e| enrich(e, meta))
        .collect();
    let total = chat.len();
    let mut lines: Vec<Value> = chat.into_iter().filter(|l| filter.matches(l)).collect();
    let more = lines.len() > max;
    lines.truncate(max);
    // Resume after the last line handed out when the cap cut the list, else
    // after everything the store had (lines the filter hid included).
    let next = if more {
        lines
            .last()
            .and_then(|l| l["seq"].as_u64())
            .unwrap_or(read.since)
    } else {
        read.head.max(read.since)
    };
    json!({
        "lines": lines,
        "unfiltered_count": total,
        "more": more,
        "since_seq": read.since,
        "next_seq": next,
        "latest_seq": read.head,
        "gap": read.gap,
    })
}

/// The store cursor this tool keeps for a caller's cursor name.
pub fn store_cursor(name: &str) -> String {
    format!("chat_log:{name}")
}

/// A caller-facing warning when the chat capture was not already running, so
/// lines from before this call may be missing from the log.
pub fn capture_warning(install: &[(String, String)]) -> Option<&'static str> {
    match install
        .iter()
        .find(|(r, _)| r == "chat")
        .map(|(_, s)| s.as_str())
    {
        Some("ok") => None,
        Some(s) if s.starts_with("installed") => Some(
            "the chat capture was installed during this call; lines shown before now are not in this log. Read client_ui_state's chat tail for them.",
        ),
        Some("missing") => Some(
            "the chat window is not loaded (not in the world yet): nothing is being captured.",
        ),
        _ => Some("the chat capture could not be read (see capture.lua_error)."),
    }
}

impl Supervisor {
    /// `client_chat_log`. `since_seq` overrides the named cursor; `peek`
    /// reads without moving it.
    pub async fn chat_log(
        &self,
        cursor: &str,
        since_seq: Option<u64>,
        filter: &ChatFilter,
        max: usize,
        peek: bool,
    ) -> Result<Value, String> {
        let t0 = Instant::now();
        let rep = self.pump_events(true).await?;
        let meta = self.lua_json(META_CHUNK).await.unwrap_or(Value::Null);
        let key = store_cursor(cursor);
        let mut out = {
            let mut st = self.events.inner.lock().await;
            let since = since_seq.unwrap_or_else(|| st.cursor(&key).unwrap_or(0));
            let slice = st.since(since, STORE_CAP);
            let read = StoreRead {
                events: &slice.events,
                since,
                head: slice.head,
                gap: slice.gap,
            };
            let out = shape(read, &meta, filter, max);
            if !peek {
                if let Some(n) = out["next_seq"].as_u64() {
                    st.reset_cursor(&key, n);
                }
            }
            out
        };
        if let (Ok(mut m), Some(n)) = (memory().lock(), out["next_seq"].as_u64()) {
            m.chat_cursors
                .insert(memory_key(self.instance(), cursor), n);
        }
        out["cursor"] = json!(cursor);
        out["capture"] = rep.to_json();
        if let Some(w) = capture_warning(&rep.lua_install) {
            out["capture_warning"] = json!(w);
        }
        if meta.is_null() {
            out["meta_error"] = json!("channel names, colours and tabs could not be read");
        }
        stamp_read(&mut out, t0.elapsed().as_millis() as u64);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Execute the same encoder and metadata Lua sent to the client against
    /// stock-shaped tables. Feedback is intentionally absent from UIChannel.
    fn client_meta() -> Value {
        let lua = mlua::Lua::new();
        lua.load(
            r#"
UIChannel = { Say = 1, Chat = 10 }
UISpeakerFlags = { GM = 1 }
ChatMod = {
  ChannelMap = { [9] = function(id, _) assert(id == 9); return 3 end },
  channelColors = { [3] = 'stock-feedback-colour' },
  channel2tabs = { [3] = { [{ getName = function() return 'Inst1Chat_TabOutput' end }] = true } }
}
"#,
        )
        .exec()
        .unwrap();
        let source = super::super::lua_json::chunk(META_CHUNK);
        let json: String = lua.load(&source).eval().unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn client_lua_reads_feedback_display_and_preserves_numeric_filter() {
        let meta = client_meta();
        assert_eq!(meta["channels"]["9"], "Feedback");
        assert_eq!(meta["display"]["9"]["client_channel"], 3);
        assert_eq!(meta["display"]["9"]["colour"], "stock-feedback-colour");
        assert_eq!(meta["display"]["9"]["tabs"][0], "Inst1Chat_TabOutput");

        let feedback = events()[2].clone();
        let line = enrich(&feedback, &meta);
        assert_eq!(line["channel_id"], 9);
        assert_eq!(line["channel"], "Feedback");
        assert_eq!(line["colour"], "stock-feedback-colour");
        assert!(ChatFilter {
            channel: Some("9".into()),
            ..Default::default()
        }
        .matches(&line));

        let mut unknown = feedback;
        unknown.fields["channel"] = json!(77);
        let unknown = enrich(&unknown, &meta);
        assert!(unknown["colour"].is_null());
        assert!(unknown["tabs"].is_null());
    }

    fn meta() -> Value {
        json!({
            "channels": { "1": "Say", "7": "Tell", "9": "Feedback", "8": "Server" },
            "chat_base": 100,
            "flags": { "None": 0, "GM": 1, "Dev": 2 },
            "display": {
                "1": { "client_channel": 1, "colour": "ffffffff", "tabs": ["Inst1Chat_TabOutput"] },
                "9": { "client_channel": 9, "colour": "stock-feedback-colour", "tabs": ["Inst1Chat_TabOutput"] }
            }
        })
    }

    fn ev(seq: u64, kind: &str, fields: Value) -> StoredEvent {
        StoredEvent {
            seq,
            kind: kind.into(),
            ts_ms: 1000 + seq as i64,
            fields,
        }
    }

    fn events() -> Vec<StoredEvent> {
        vec![
            ev(
                10,
                "chat.line",
                json!({ "channel": 1, "channel_name": "", "speaker": "Labone", "flags": 3, "text": "hello", "ring_seq": 1 }),
            ),
            ev(
                11,
                "cme.event",
                json!({ "event": "Event_NetIn_onDialogDisplay" }),
            ),
            ev(
                12,
                "chat.line",
                json!({ "channel": 9, "channel_name": "", "speaker": "", "flags": 0, "text": "You cannot do that." }),
            ),
            ev(
                13,
                "chat.line",
                json!({ "channel": 104, "channel_name": "Trade", "speaker": "Bob", "flags": 0, "text": "wts zat" }),
            ),
        ]
    }

    fn read(evs: &[StoredEvent], since: u64) -> StoreRead<'_> {
        StoreRead {
            events: evs,
            since,
            head: 13,
            gap: false,
        }
    }

    #[test]
    fn chat_lines_are_named_coloured_and_flagged() {
        let evs = events();
        let s = shape(read(&evs, 9), &meta(), &ChatFilter::default(), 50);
        let lines = s["lines"].as_array().unwrap();
        assert_eq!(lines.len(), 3, "the cme.event is not a chat line");
        assert_eq!(lines[0]["channel"], "Say");
        assert_eq!(lines[0]["flag_names"], json!(["Dev", "GM"]));
        assert_eq!(lines[0]["colour"], "ffffffff");
        assert_eq!(lines[0]["tabs"][0], "Inst1Chat_TabOutput");
        assert_eq!(lines[1]["channel"], "Feedback");
        assert_eq!(lines[1]["colour"], "stock-feedback-colour");
        assert_eq!(lines[1]["tabs"][0], "Inst1Chat_TabOutput");
        // A custom channel keeps the server's name.
        assert_eq!(lines[2]["channel"], "Trade");
        assert_eq!(lines[2]["ts_ms"], 1013);
    }

    #[test]
    fn unknown_channel_keeps_null_display_metadata() {
        let mut event = events()[0].clone();
        event.fields["channel"] = json!(77);
        let line = enrich(&event, &meta());
        assert_eq!(line["channel_id"], 77);
        assert!(line["colour"].is_null());
        assert!(line["tabs"].is_null());
    }

    #[test]
    fn filters_by_channel_name_or_number_text_and_speaker() {
        let evs = events();
        let run = |f: ChatFilter| shape(read(&evs, 9), &meta(), &f, 50);
        let by_name = run(ChatFilter {
            channel: Some("feedback".into()),
            ..Default::default()
        });
        assert_eq!(by_name["lines"][0]["seq"], 12);
        let by_id = run(ChatFilter {
            channel: Some("104".into()),
            ..Default::default()
        });
        assert_eq!(by_id["lines"][0]["text"], "wts zat");
        let text = run(ChatFilter {
            contains: Some("CANNOT".into()),
            ..Default::default()
        });
        assert_eq!(text["lines"][0]["seq"], 12);
        let speaker = run(ChatFilter {
            speaker: Some("labo".into()),
            ..Default::default()
        });
        assert_eq!(speaker["lines"].as_array().unwrap().len(), 1);
    }

    /// The cursor moves past everything the store had, filtered-out lines
    /// and other event kinds included, unless the cap cut the list short.
    #[test]
    fn next_seq_covers_filtered_lines_but_not_capped_ones() {
        let evs = events();
        let all = shape(read(&evs, 9), &meta(), &ChatFilter::default(), 50);
        assert_eq!(all["next_seq"], 13);
        assert_eq!(all["more"], false);
        let capped = shape(read(&evs, 9), &meta(), &ChatFilter::default(), 1);
        assert_eq!(capped["next_seq"], 10);
        assert_eq!(capped["more"], true);
        let beyond = StoreRead {
            events: &[],
            since: 20,
            head: 13,
            gap: true,
        };
        let none = shape(beyond, &meta(), &ChatFilter::default(), 5);
        assert_eq!(
            none["next_seq"], 20,
            "an explicit since beyond the head is kept"
        );
        assert_eq!(none["gap"], true);
    }

    /// Without the metadata (the read failed) the raw line still comes back.
    #[test]
    fn lines_survive_missing_metadata() {
        let evs = events();
        let s = shape(read(&evs, 0), &Value::Null, &ChatFilter::default(), 50);
        assert_eq!(s["lines"][0]["text"], "hello");
        assert_eq!(s["lines"][0]["channel"], "");
        assert_eq!(s["lines"][0]["flag_names"], json!([]));
    }

    #[test]
    fn cursors_are_namespaced_in_the_shared_store() {
        assert_eq!(store_cursor("default"), "chat_log:default");
    }

    #[test]
    fn capture_warning_names_a_late_or_absent_capture() {
        let s = |status: &str| vec![("chat".to_string(), status.to_string())];
        assert!(capture_warning(&s("ok")).is_none());
        assert!(capture_warning(&s("installed"))
            .unwrap()
            .contains("installed during this call"));
        assert!(capture_warning(&s("installed_no_resubscribe")).is_some());
        assert!(capture_warning(&s("missing"))
            .unwrap()
            .contains("not loaded"));
        // No install line at all (the Lua read failed).
        assert!(capture_warning(&[]).unwrap().contains("could not be read"));
    }
}
