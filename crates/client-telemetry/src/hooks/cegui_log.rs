//! What the CEGUI log tee reports: the line text, its level, and a
//! throttle so a layout load cannot flood the queue.
//!
//! The vtable detour on `CEGUI::DefaultLogger::logEvent` (in
//! `vtable_hooks`) calls [`observe`]. A CEGUI exception logs itself
//! through this logger (the wide `"Exception: "` prefix at `0x01925474`),
//! and the client's tolua glue throws `ScriptException` for a failed Lua
//! binding call (`"...ScriptException' was thrown by function
//! 'SubscribeWindowEvent'..."` at `0x0194c6a4`, and a dozen more). So
//! this should be where UI-script failures that never reach `lua_pcall`'s
//! caller become visible. That is read off the binary's strings, not yet
//! seen live.
//!
//! The message is a `const String&`. In this client CEGUI's `String` is an
//! MSVC `std::wstring` (see [`crate::msvc_string`]), so the text is read
//! with the same bounded reader as the event-registry hook, never through
//! CEGUI's own methods.

use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use crate::hooks::name_throttle::{Decision, NameThrottle};

/// Telemetry target, unchanged from the text-less version of this tee.
pub(crate) const TARGET: &str = "client.ui.cegui_log";

/// Longest message kept. CEGUI error lines carry a file name and a Lua
/// traceback line; 512 characters keeps both.
pub(crate) const MAX_MESSAGE_CHARS: usize = 512;

/// CEGUI `LoggingLevel` name. The logger's own level switch
/// (`jmp [level*4 + 0x01212d7c]`, five cases) matches CEGUI 0.6's
/// `Errors, Warnings, Standard, Informative, Insane`.
pub(crate) fn level_name(level: i32) -> &'static str {
    match level {
        0 => "errors",
        1 => "warnings",
        2 => "standard",
        3 => "informative",
        4 => "insane",
        _ => "unknown",
    }
}

/// Telemetry level for a CEGUI level: errors are errors, warnings are
/// warnings, everything else is `debug`.
pub(crate) fn telemetry_level(level: i32) -> &'static str {
    match level {
        0 => "error",
        1 => "warn",
        _ => "debug",
    }
}

/// Throttle key. Errors and warnings are keyed on their text, so two
/// different errors never hide each other and one repeating error is
/// held to the rate; the chatty levels share one bucket per level.
pub(crate) fn throttle_key(level: i32, message: &str) -> String {
    if level <= 1 {
        format!("{}:{message}", level_name(level))
    } else {
        level_name(level).to_string()
    }
}

static THROTTLE: Mutex<Option<NameThrottle>> = Mutex::new(None);
static EPOCH: OnceLock<Instant> = OnceLock::new();

fn throttle(key: &str) -> Decision {
    let now_ms = EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let mut guard = THROTTLE.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .get_or_insert_with(NameThrottle::new)
        .check(key, now_ms)
}

/// Fields for one emitted line, or `None` when throttled.
pub(crate) fn line_fields(
    level: i32,
    message: &str,
    truncated: bool,
    decision: Decision,
) -> Option<Vec<(&'static str, serde_json::Value)>> {
    let Decision::Emit { suppressed } = decision else {
        return None;
    };
    let mut fields = vec![
        ("level", serde_json::json!(level)),
        ("level_name", serde_json::json!(level_name(level))),
        ("message", serde_json::json!(message)),
    ];
    if suppressed > 0 {
        fields.push(("suppressed", serde_json::json!(suppressed)));
    }
    if truncated {
        fields.push(("truncated", serde_json::json!(true)));
    }
    Some(fields)
}

/// Report one `logEvent(message, level)` call.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) fn observe(message: *const std::ffi::c_void, level: i32) {
    // SAFETY: `message` is the `const String&` argument of the hooked
    // call, alive until it returns.
    let decoded = unsafe {
        crate::msvc_string::read(
            message as *const u8,
            crate::msvc_string::Width::Wide,
            MAX_MESSAGE_CHARS,
        )
    };
    let (text, truncated) = match decoded {
        Some(d) => (d.text, d.truncated),
        None => ("<unreadable>".to_string(), false),
    };
    let Some(fields) = line_fields(
        level,
        &text,
        truncated,
        throttle(&throttle_key(level, &text)),
    ) else {
        return;
    };

    #[cfg(feature = "lab-bridge")]
    crate::bridge::events::push(
        "cegui.log",
        crate::bridge::crash::now_ms(),
        serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ),
    );

    if let Some(p) = crate::boot::producer() {
        let mut b = crate::events::ClientNativeEvent::builder(TARGET, telemetry_level(level));
        for (k, v) in fields {
            b = b.field(k, v);
        }
        p.try_emit(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_map_to_cegui_names_and_telemetry_levels() {
        assert_eq!(level_name(0), "errors");
        assert_eq!(level_name(4), "insane");
        assert_eq!(level_name(9), "unknown");
        assert_eq!(telemetry_level(0), "error");
        assert_eq!(telemetry_level(1), "warn");
        assert_eq!(telemetry_level(2), "debug");
    }

    /// Distinct errors get distinct buckets; chatty levels share one.
    #[test]
    fn errors_are_keyed_by_text_and_chatter_by_level() {
        assert_ne!(throttle_key(0, "a"), throttle_key(0, "b"));
        assert_eq!(throttle_key(2, "a"), throttle_key(2, "b"));
        assert_eq!(throttle_key(3, "anything"), "informative");
    }

    #[test]
    fn fields_carry_the_text() {
        let f = line_fields(
            0,
            "(Error) Lua error in Dialog.lua",
            false,
            Decision::Emit { suppressed: 2 },
        )
        .unwrap();
        assert!(f.contains(&("level_name", serde_json::json!("errors"))));
        assert!(f.contains(&(
            "message",
            serde_json::json!("(Error) Lua error in Dialog.lua")
        )));
        assert!(f.contains(&("suppressed", serde_json::json!(2))));
        assert!(line_fields(2, "x", false, Decision::Suppress).is_none());
    }
}
