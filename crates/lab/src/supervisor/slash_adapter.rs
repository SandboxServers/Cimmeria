//! Stock chat Lua candidate for native slash submission.
//!
//! This route is intentionally gated by the unverified client fingerprint.
//! It never invokes the legacy raw-address `client_console` endpoint.

use crate::workbench::slash_transport::{
    PlainShoutCandidate, SlashObservation, SlashReceipt, SlashStage,
};

use super::flows::widgets::lua_quote;
use super::Supervisor;

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

impl Supervisor {
    /// Submit only after a checked fingerprint is installed in the adapter.
    /// A successful Lua return is recorded as submitted, never accepted.
    #[allow(dead_code, reason = "DW-24B blocks live slash submission")]
    pub async fn submit_plain_shout_candidate(
        &self,
        line: &str,
        fingerprint: &str,
        generation: u64,
    ) -> Result<SlashReceipt, String> {
        let candidate = PlainShoutCandidate::parse(line, fingerprint)?;
        candidate.require_verified_client()?;
        let mut receipt = SlashReceipt::new(candidate, generation);
        let results = self.lua_results(&stock_chat_chunk(line)).await?;
        if results.first().map(String::as_str) != Some("client_submitted") {
            return Err(format!(
                "stock chat did not submit: {}",
                results.first().map(String::as_str).unwrap_or("no result")
            ));
        }
        receipt.observe(&SlashObservation {
            attempt_id: receipt.attempt_id,
            line: receipt.line.clone(),
            fingerprint: receipt.fingerprint.clone(),
            generation,
            stage: SlashStage::ClientSubmitted,
            independent_source: true,
        })?;
        Ok(receipt)
    }
}
