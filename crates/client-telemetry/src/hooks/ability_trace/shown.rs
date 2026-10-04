//! What the client asked its UI to show: `client.ability.shown` (AB-C5).
//!
//! Floating combat text, the combat chat line, chat lines and the effect
//! bar are drawn by Lua, not by a native call the DLL could hook
//! (`docs/reverse-engineering/findings/ability-client-hook-anchors.md`
//! § AB-C5). The native side raises a UI event, and the script window
//! calls the Lua function subscribed to it by name, through `lua_pcall`.
//! The existing `lua_pcall` / `lua_call` IAT detours therefore see each of
//! these handlers start, with the event's arguments on the stack. This
//! module names the handlers by where their function is defined
//! (`short_src` and `linedefined` from `lua_getinfo(">S")`) and builds the
//! event from their arguments.
//!
//! | `kind` | Handler (stock client UI) | Subscribed to |
//! |---|---|---|
//! | `combat_text` | `SCTMod.onUnitCombat(window, abilityId, hitType, statList)`, `SCT.lua:83` | `Events.UnitCombat` |
//! | `combat_chat_line` | `CHAT_onUnitCombat(window, abilityId, hitType, statList)`, `ChatEvents.lua:63` | `Events.UnitCombat` |
//! | `feedback_line` | `ChatMod.onMessageReceived(this, speaker, speakerFlags, channelId, channelName, text)`, `ChatWindow.lua:93`, **feedback channel only** | `Events.MessageReceived` |
//! | `effect_bar_ui` | `EffectsMod.onUnitEffectsUpdate(this, unitID)`, `Effect.lua:7` | `Events.UnitEffectsUpdate` |
//!
//! The lines are the stock client's (`Content/UI/Core/...`, 2026-10-04);
//! a UI patch that moves one of these functions needs its row updated, or
//! its handler is silently not reported. A row says the handler **ran**,
//! and `status` whether it returned cleanly; whether the text then fit on
//! screen is the handler's business (`SCTMod` writes a `Debug:warn` when
//! its frame cache is empty, which `client.lua.debug_log` reports).
//!
//! The feedback filter is on `channelId`: the native passes the server's
//! `EChannel` byte, and 9 is feedback (`CHAN_FEEDBACK`; the server's
//! channel bytes select `ChatMod.ChannelMap` entries directly). Other
//! channels are players' chat and are not reported.

use serde_json::{json, Value};

use super::recv_methods::CHAN_FEEDBACK;
use crate::hooks::entity_trace::{sequences::RequestIds, Fields};

/// `client.ability.shown`.
pub(crate) const TARGET_SHOWN: &str = "client.ability.shown";

/// Lua arguments read per call.
pub(crate) const MAX_ARGS: usize = 6;
/// Characters kept per string argument.
pub(crate) const MAX_ARG_CHARS: usize = 256;

/// One UI handler the event reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Handler {
    /// The `kind` field.
    pub kind: &'static str,
    /// The function, for the `handler` field.
    pub function: &'static str,
    /// Its source file's name.
    pub file: &'static str,
    /// `linedefined`.
    pub line: i32,
    /// The parameter names, `window` / `this` first: the fields the
    /// arguments become when the call has this many.
    pub params: &'static [&'static str],
}

/// Every handler reported.
pub(crate) const HANDLERS: &[Handler] = &[
    Handler {
        kind: "combat_text",
        function: "SCTMod.onUnitCombat",
        file: "SCT.lua",
        line: 83,
        params: &["window", "ability_id", "hit_type", "stat_list"],
    },
    Handler {
        kind: "combat_chat_line",
        function: "CHAT_onUnitCombat",
        file: "ChatEvents.lua",
        line: 63,
        params: &["window", "ability_id", "hit_type", "stat_list"],
    },
    Handler {
        kind: "feedback_line",
        function: "ChatMod.onMessageReceived",
        file: "ChatWindow.lua",
        line: 93,
        params: &[
            "window",
            "speaker",
            "speaker_flags",
            "channel_id",
            "channel_name",
            "text",
        ],
    },
    Handler {
        kind: "effect_bar_ui",
        function: "EffectsMod.onUnitEffectsUpdate",
        file: "Effect.lua",
        line: 7,
        params: &["window", "unit"],
    },
];

/// The file name in a Lua `short_src`: `[string "SCT.lua"]`,
/// `...\Core\SCT\SCT.lua` and `SCT.lua` all give `SCT.lua`.
pub(crate) fn file_name(short_src: &str) -> &str {
    let s = short_src
        .strip_prefix("[string \"")
        .map_or(short_src, |s| s.strip_suffix("\"]").unwrap_or(s));
    s.rsplit(['/', '\\']).next().unwrap_or(s)
}

/// The handler defined at `short_src:line`, if it is one of ours.
pub(crate) fn identify(short_src: &str, line: i32) -> Option<&'static Handler> {
    let file = file_name(short_src);
    HANDLERS
        .iter()
        .find(|h| h.line == line && h.file.eq_ignore_ascii_case(file))
}

/// One Lua argument, as read from the stack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LuaArg {
    /// A number.
    Number(f64),
    /// A string (cut to [`MAX_ARG_CHARS`]).
    Text(String),
    /// Anything else, by its type name.
    Other(&'static str),
}

impl LuaArg {
    fn value(&self) -> Value {
        match self {
            LuaArg::Number(n) if n.fract() == 0.0 && n.abs() < 9.0e15 => json!(*n as i64),
            LuaArg::Number(n) => json!(n),
            LuaArg::Text(s) => json!(s),
            LuaArg::Other(t) => json!(format!("<{t}>")),
        }
    }

    fn as_number(&self) -> Option<f64> {
        match self {
            LuaArg::Number(n) => Some(*n),
            _ => None,
        }
    }
}

/// Whether `handler`'s call with `args` should be reported. Only the
/// feedback channel of the chat handler is.
pub(crate) fn wanted(handler: &Handler, args: &[LuaArg]) -> bool {
    if handler.kind != "feedback_line" {
        return true;
    }
    args.get(3).and_then(LuaArg::as_number) == Some(f64::from(CHAN_FEEDBACK))
}

/// How a handler call ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Completion {
    /// Under `lua_pcall`, which returned this status (0 is success).
    Pcall(i32),
    /// Under `lua_call`, which returned normally.
    Returned,
    /// Under `lua_call`, and the Lua error unwound through the detour (a
    /// C++ throw: the client's `lua51.dll` is C++-compiled).
    Raised,
}

/// Reports a handler call when it ends, including when a Lua error unwinds
/// through the `lua_call` detour that holds it.
///
/// Why a drop guard is sound here: the detour and the trampoline type it
/// calls the original through are `extern "C-unwind"`, so a foreign (C++)
/// exception may unwind through the detour's frame, and Rust runs that
/// frame's destructors on the way (the `C-unwind` contract). On
/// `i686-pc-windows-msvc` the destructors are SEH cleanup funclets, which
/// the MSVC C++ runtime runs for any C++ throw: the same mechanism the
/// entity-dispatch and sequence scopes (`CtxScope`, `Restore`) already
/// rely on. What runs in the drop must not unwind and must not touch the
/// Lua state, which is mid-error: `report` is the arguments already read
/// before the call, a throttle check and a queue push, each panic-contained
/// by the caller's `report` (`native::after_call`).
pub(crate) struct ReportOnExit<T, F: FnMut(T, Completion)> {
    pending: Option<T>,
    report: F,
}

impl<T, F: FnMut(T, Completion)> ReportOnExit<T, F> {
    /// Hold `pending` until the call ends.
    pub(crate) fn new(pending: T, report: F) -> Self {
        Self {
            pending: Some(pending),
            report,
        }
    }

    /// The call returned: report it as `completion`.
    pub(crate) fn finish(mut self, completion: Completion) {
        if let Some(p) = self.pending.take() {
            (self.report)(p, completion);
        }
    }
}

impl<T, F: FnMut(T, Completion)> Drop for ReportOnExit<T, F> {
    fn drop(&mut self) {
        // Reached with `pending` still set only when the call did not
        // return: it is being unwound.
        if let Some(p) = self.pending.take() {
            (self.report)(p, Completion::Raised);
        }
    }
}

/// The event for one handler call, with how it ended. Arguments are named by the
/// handler's parameters when the call passed exactly that many (the
/// window or `this` is not reported); otherwise they go out as `args`
/// with `arity_mismatch`, which is evidence the native side passes
/// something other than the script expects.
pub(crate) fn shown_fields(
    handler: &Handler,
    nargs: i32,
    args: &[LuaArg],
    completion: Completion,
) -> Fields {
    let ok = matches!(completion, Completion::Pcall(0) | Completion::Returned);
    let mut f: Fields = vec![
        ("kind", json!(handler.kind)),
        ("handler", json!(handler.function)),
        ("status", json!(if ok { "ok" } else { "failed" })),
    ];
    match completion {
        Completion::Pcall(s) if s != 0 => f.push(("lua_status", json!(s))),
        Completion::Raised => f.push(("raised", json!(true))),
        _ => {}
    }
    if nargs as usize == handler.params.len() && args.len() == handler.params.len() {
        for (name, a) in handler.params.iter().zip(args).skip(1) {
            // A table (the stat list) is reported by type only.
            f.push((name, a.value()));
        }
    } else {
        f.push(("nargs", json!(nargs)));
        f.push(("arity_mismatch", json!(true)));
        f.push((
            "args",
            Value::Array(args.iter().map(LuaArg::value).collect()),
        ));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) use native::{after_call, before_call};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod native {
    //! The calls from the `lua_pcall` / `lua_call` detours.
    use super::*;
    use crate::hooks::lua_stack;
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::ffi::c_void;
    use std::time::{Duration, Instant};

    /// Functions known not to be one of [`HANDLERS`], by closure address,
    /// so the hot calls (every window's `PreRender`, every frame) cost one
    /// `lua_topointer` and a set lookup. A collected closure's address can
    /// be reused by a new function, so the set is forgotten every
    /// [`NEGATIVE_TTL`] and when it reaches [`NEGATIVE_CAP`]; a handler
    /// that inherits a stale address is missed for at most that long.
    /// A match is never cached: it is re-checked with `lua_getinfo` on
    /// every call, which only these few handlers pay.
    const NEGATIVE_CAP: usize = 4096;
    const NEGATIVE_TTL: Duration = Duration::from_secs(60);
    /// Calls with more arguments than this are not UI event handlers.
    const MAX_NARGS: i32 = 16;

    thread_local! {
        static NOT_HANDLERS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
        static SINCE: Cell<Option<Instant>> = const { Cell::new(None) };
    }

    fn known_not_handler(ptr: usize) -> bool {
        let now = Instant::now();
        let stale = SINCE.with(|s| match s.get() {
            Some(t) if now.duration_since(t) < NEGATIVE_TTL => false,
            _ => {
                s.set(Some(now));
                true
            }
        });
        NOT_HANDLERS.with(|n| {
            let mut n = n.borrow_mut();
            if stale || n.len() >= NEGATIVE_CAP {
                n.clear();
            }
            n.contains(&ptr)
        })
    }

    fn remember_not_handler(ptr: usize) {
        NOT_HANDLERS.with(|n| {
            n.borrow_mut().insert(ptr);
        });
    }

    fn read_arg(l: *mut c_void, idx: i32) -> LuaArg {
        match lua_stack::value_type(l, idx) {
            Some(lua_stack::LUA_TNUMBER) => {
                lua_stack::number_at(l, idx).map_or(LuaArg::Other("number"), LuaArg::Number)
            }
            Some(lua_stack::LUA_TSTRING) => lua_stack::read_string(l, idx, MAX_ARG_CHARS)
                .map_or(LuaArg::Other("string"), |(s, _)| LuaArg::Text(s)),
            Some(t) => LuaArg::Other(lua_stack::type_name(t)),
            None => LuaArg::Other("unknown"),
        }
    }

    /// One handler call in progress.
    pub(crate) struct Pending {
        handler: &'static Handler,
        nargs: i32,
        args: Vec<LuaArg>,
    }

    /// Before the call: is the function being called one of the handlers?
    /// Reads its arguments while they are still on the stack.
    pub(crate) fn before_call(l: *mut c_void, nargs: i32) -> Option<Pending> {
        if !(0..=MAX_NARGS).contains(&nargs) {
            return None;
        }
        let func = -(nargs + 1);
        let ptr = lua_stack::pointer_at(l, func).filter(|&p| p != 0)?;
        if known_not_handler(ptr) {
            return None;
        }
        // No room for `lua_getinfo`'s copy is transient: not cached.
        let (src, line) = lua_stack::function_info_at(l, func)?;
        let Some(handler) = identify(&src, line) else {
            remember_not_handler(ptr);
            return None;
        };
        let args: Vec<LuaArg> = (0..nargs.min(MAX_ARGS as i32))
            .map(|k| read_arg(l, k - nargs))
            .collect();
        wanted(handler, &args).then_some(Pending {
            handler,
            nargs,
            args,
        })
    }

    /// After the call returned: report it. `status` is `lua_pcall`'s.
    /// After the call ended: report it. Never unwinds (it runs from
    /// [`ReportOnExit`]'s drop while a Lua error is in flight).
    pub(crate) fn after_call(p: Pending, completion: Completion) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let key = format!("shown:{}", p.handler.kind);
            if let Some(f) = crate::hooks::ability_trace::admit(&key, || {
                shown_fields(p.handler, p.nargs, &p.args, completion)
            }) {
                crate::hooks::emit::emit(TARGET_SHOWN, "info", f);
            }
        }));
    }
}

// ---------------------------------------------------------------------
// Sequences that played

/// The Kismet event the server picks an interrupt's sequence for
/// (`EVENT_ABILITY_INTERRUPT` in `cimmeria-cell-catalog`).
pub(crate) const EVENT_ABILITY_INTERRUPT: u32 = 1002;

/// `kind = sequence_played`: the `SequenceManager` play step
/// (`0x00d06dd0`) got a Kismet instance for a request. `event_id` is the
/// cooked sequence's Kismet event, which is how the client knows an
/// interrupt (1002); the wire never says.
pub(crate) fn sequence_played_fields(
    ids: &RequestIds,
    event_id: Option<u32>,
    stage: &'static str,
) -> Fields {
    let mut f: Fields = vec![
        ("kind", json!("sequence_played")),
        ("stage", json!(stage)),
        ("sequence_id", json!(ids.sequence_id)),
        ("entity_id", json!(ids.source_id)),
        ("target_id", json!(ids.target_id)),
        ("instance_id", json!(ids.instance_id)),
        ("event_id", json!(event_id)),
        (
            "interrupt",
            json!(event_id == Some(EVENT_ABILITY_INTERRUPT)),
        ),
    ];
    if let Some(cast) = ids.instance_id.filter(|&v| v != 0) {
        f.push(("cast_id", json!(cast)));
    }
    f
}

// ---------------------------------------------------------------------
// `ui_area` on Lua errors

/// The ability UI's scripts, lower case. A Lua error whose message or
/// function source names one of them is tagged `ui_area = ability`.
const ABILITY_UI_NAMES: &[&str] = &[
    "actionbuttons.lua",
    "actionbuttonmod",
    "effect.lua",
    "effectsmod",
    "sct.lua",
    "sct.settings.lua",
    "sctframe.lua",
    "sctframecache.lua",
    "sctmod",
];

/// `Some("ability")` when any of `texts` names an ability UI script or
/// table. A name must start a word (`SideEffect.lua` is not `Effect.lua`).
pub(crate) fn ui_area(texts: &[&str]) -> Option<&'static str> {
    texts
        .iter()
        .any(|t| {
            let lower = t.to_ascii_lowercase();
            ABILITY_UI_NAMES.iter().any(|name| {
                lower
                    .match_indices(name)
                    .any(|(at, _)| at == 0 || !lower.as_bytes()[at - 1].is_ascii_alphanumeric())
            })
        })
        .then_some("ability")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &Fields, k: &str) -> Value {
        f.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Null)
    }

    #[test]
    fn short_src_spellings_resolve_to_the_file_name() {
        assert_eq!(file_name("[string \"SCT.lua\"]"), "SCT.lua");
        assert_eq!(
            file_name("..\\SGWGame\\Content\\UI\\Core\\SCT\\SCT.lua"),
            "SCT.lua"
        );
        assert_eq!(
            file_name("UI/Core/ChatWindow/ChatEvents.lua"),
            "ChatEvents.lua"
        );
        assert_eq!(file_name("Effect.lua"), "Effect.lua");
    }

    #[test]
    fn handlers_are_named_by_file_and_line() {
        assert_eq!(
            identify("[string \"SCT.lua\"]", 83).unwrap().kind,
            "combat_text"
        );
        assert_eq!(identify("sct.lua", 83).unwrap().kind, "combat_text");
        assert!(identify("[string \"SCT.lua\"]", 84).is_none());
        assert!(identify("[string \"SCTFrame.lua\"]", 83).is_none());
        assert_eq!(
            identify("ChatWindow.lua", 93).unwrap().function,
            "ChatMod.onMessageReceived"
        );
        assert_eq!(identify("Effect.lua", 7).unwrap().kind, "effect_bar_ui");
    }

    /// The line numbers are the stock client's. The client's UI is not in
    /// git, so this checks them against the copy at `SGW_CLIENT_UI` when
    /// it is set (`.../Working/SGWGame/Content/UI/Core`) and skips
    /// otherwise.
    #[test]
    fn handler_lines_match_the_client_ui_when_available() {
        let Ok(root) = std::env::var("SGW_CLIENT_UI") else {
            return;
        };
        let dirs = [
            ("SCT.lua", "SCT"),
            ("ChatEvents.lua", "ChatWindow"),
            ("ChatWindow.lua", "ChatWindow"),
            ("Effect.lua", "Effect"),
        ];
        for h in HANDLERS {
            let dir = dirs.iter().find(|(f, _)| *f == h.file).unwrap().1;
            let text = std::fs::read_to_string(format!("{root}/{dir}/{}", h.file)).unwrap();
            let line = text.lines().nth(h.line as usize - 1).unwrap();
            assert!(
                line.starts_with(&format!("function {}(", h.function)),
                "{}:{} is `{line}`",
                h.file,
                h.line
            );
        }
    }

    #[test]
    fn combat_text_names_its_arguments() {
        let h = identify("SCT.lua", 83).unwrap();
        let args = [
            LuaArg::Other("userdata"),
            LuaArg::Number(597.0),
            LuaArg::Number(2.0),
            LuaArg::Other("table"),
        ];
        let f = shown_fields(h, 4, &args, Completion::Pcall(0));
        assert_eq!(get(&f, "kind"), json!("combat_text"));
        assert_eq!(get(&f, "handler"), json!("SCTMod.onUnitCombat"));
        assert_eq!(get(&f, "status"), json!("ok"));
        assert_eq!(get(&f, "ability_id"), json!(597));
        assert_eq!(get(&f, "hit_type"), json!(2));
        assert_eq!(get(&f, "stat_list"), json!("<table>"));
        assert_eq!(get(&f, "window"), Value::Null, "the window is not reported");
    }

    /// A call with another arity keeps every argument, unnamed, and says so.
    #[test]
    fn an_unexpected_arity_is_reported_raw() {
        let h = identify("SCT.lua", 83).unwrap();
        let args = [LuaArg::Number(597.0), LuaArg::Number(1.5)];
        let f = shown_fields(h, 2, &args, Completion::Pcall(2));
        assert_eq!(get(&f, "arity_mismatch"), json!(true));
        assert_eq!(get(&f, "args"), json!([597, 1.5]));
        assert_eq!(get(&f, "status"), json!("failed"));
        assert_eq!(get(&f, "lua_status"), json!(2));
    }

    /// Only channel 9 of the chat handler is reported; the other handlers
    /// always are.
    #[test]
    fn chat_lines_are_feedback_only() {
        let chat = identify("ChatWindow.lua", 93).unwrap();
        let line = |channel: f64| {
            vec![
                LuaArg::Other("userdata"),
                LuaArg::Text(String::new()),
                LuaArg::Number(0.0),
                LuaArg::Number(channel),
                LuaArg::Other("nil"),
                LuaArg::Text("You cannot do that yet.".into()),
            ]
        };
        assert!(wanted(chat, &line(9.0)));
        let f = shown_fields(chat, 6, &line(9.0), Completion::Returned);
        assert_eq!(get(&f, "text"), json!("You cannot do that yet."));
        assert_eq!(get(&f, "channel_id"), json!(9));
        for other in [0.0, 3.0, 8.0, 10.0, 12.0] {
            assert!(!wanted(chat, &line(other)));
        }
        assert!(wanted(identify("SCT.lua", 83).unwrap(), &[]));
    }

    #[test]
    fn played_sequences_carry_the_cast_and_name_an_interrupt() {
        let ids = RequestIds {
            sequence_id: Some(4711),
            source_id: Some(77),
            target_id: Some(88),
            view_type: Some(3),
            instance_id: Some(31337),
            created: None,
        };
        let f = sequence_played_fields(&ids, Some(1002), "cache_ready");
        assert_eq!(get(&f, "kind"), json!("sequence_played"));
        assert_eq!(get(&f, "cast_id"), json!(31337));
        assert_eq!(get(&f, "interrupt"), json!(true));
        assert_eq!(get(&f, "entity_id"), json!(77));
        let begin = sequence_played_fields(
            &RequestIds {
                instance_id: Some(0),
                ..ids
            },
            Some(1000),
            "cache_ready",
        );
        assert_eq!(get(&begin, "interrupt"), json!(false));
        assert_eq!(get(&begin, "cast_id"), Value::Null);
    }

    /// A handler under `lua_call` whose Lua error unwinds through the
    /// detour is still reported, as `raised`; one that returns is reported
    /// once, as returned. A Rust panic stands in for the client's C++
    /// throw, as in the other unwind tests: both run the frame's drops.
    #[test]
    fn a_handler_that_raises_is_still_reported() {
        use std::cell::RefCell;
        let seen: RefCell<Vec<(u32, Completion)>> = RefCell::new(Vec::new());
        let record = |p: u32, c: Completion| seen.borrow_mut().push((p, c));

        // The detour's shape: hold the guard, call the original, finish.
        fn lua_call_original(raises: bool) {
            if raises {
                panic!("Lua error thrown by lua51.dll");
            }
        }
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let guard = ReportOnExit::new(7u32, record);
            lua_call_original(true);
            guard.finish(Completion::Returned);
        }));
        assert!(caught.is_err());
        assert_eq!(*seen.borrow(), vec![(7, Completion::Raised)]);

        seen.borrow_mut().clear();
        ReportOnExit::new(8u32, record).finish(Completion::Returned);
        assert_eq!(*seen.borrow(), vec![(8, Completion::Returned)]);

        let h = identify("SCT.lua", 83).unwrap();
        let f = shown_fields(h, 0, &[], Completion::Raised);
        assert_eq!(get(&f, "status"), json!("failed"));
        assert_eq!(get(&f, "raised"), json!(true));
        assert_eq!(get(&f, "lua_status"), Value::Null);
    }

    #[test]
    fn ability_ui_errors_are_tagged() {
        assert_eq!(
            ui_area(&["[string \"ActionButtons.lua\"]:160: attempt to index nil"]),
            Some("ability")
        );
        assert_eq!(ui_area(&["", "[string \"SCT.lua\"]"]), Some("ability"));
        assert_eq!(
            ui_area(&["SCTMod.createEventWindow failed"]),
            Some("ability")
        );
        assert_eq!(ui_area(&["Effect.lua:12: bad argument"]), Some("ability"));
        assert_eq!(ui_area(&["SideEffect.lua:12: bad argument"]), None);
        assert_eq!(ui_area(&["[string \"BlackMarket.lua\"]:212"]), None);
        assert_eq!(ui_area(&[]), None);
    }
}
