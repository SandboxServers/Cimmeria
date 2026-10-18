//! Stock chat Lua candidate for native slash submission.
//!
//! This is only a pure Lua builder. No callable submit method exists until
//! the supervisor can derive and recheck the client identity itself.

use super::flows::widgets::lua_quote;

/// Build a single main-thread stock UI call with exact input readback.
/// The returned marker proves only that the Lua handler returned.
#[allow(
    dead_code,
    reason = "DW-24B keeps the adapter dormant until client proof"
)]
pub fn stock_chat_chunk(line: &str) -> String {
    let quoted = lua_quote(line);
    format!(
        "local input = Inst1Chat_Input\n\
         if not input or type(ChatMod) ~= 'table' or type(ChatMod.onTextAccepted) ~= 'function' then return 'missing_chat_ui' end\n\
         input:setText({quoted})\n\
         if input:getText() ~= {quoted} then return 'readback_mismatch' end\n\
         ChatMod.onTextAccepted(input, input)\n\
         return 'client_submitted'"
    )
}
