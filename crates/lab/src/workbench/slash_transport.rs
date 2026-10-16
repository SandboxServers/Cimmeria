//! Bounded stock-chat slash candidate and separate receipt observations.
//!
//! One hub-pass UAT used `/gmsendgmshout 1 HPUshout1` through
//! `ChatMod.onTextAccepted` and saw a native server receipt. That proves a
//! narrow plain-token precedent, not a general patch-table serializer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The only shape with a documented stock-chat server receipt so far.
pub const PLAIN_SHOUT_WORD: &str = "/gmsendgmshout";
/// No client binary fingerprint has been verified for this adapter.
pub const VERIFIED_CHAT_FINGERPRINT: Option<&str> = None;

/// A prepared command whose input shape matches the documented precedent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainShoutCandidate {
    pub line: String,
    pub fingerprint: String,
}

impl PlainShoutCandidate {
    /// Refuse characters and optional shapes whose tokenizer is untraced.
    pub fn parse(line: &str, fingerprint: &str) -> Result<Self, String> {
        if line.len() > 120 || line.chars().any(char::is_control) {
            return Err("slash line is unbounded or contains control characters".into());
        }
        let parts: Vec<_> = line.split(' ').collect();
        if parts.len() != 3
            || parts[0] != PLAIN_SHOUT_WORD
            || parts[1] != "1"
            || parts[2].is_empty()
            || !parts[2].bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err("slash argument shape lacks parser and receipt proof".into());
        }
        if fingerprint.is_empty() || fingerprint.len() > 128 {
            return Err("client fingerprint is missing or unbounded".into());
        }
        Ok(Self {
            line: line.into(),
            fingerprint: fingerprint.into(),
        })
    }

    /// A known command shape still needs a verified binary fingerprint.
    pub fn require_verified_client(&self) -> Result<(), String> {
        match VERIFIED_CHAT_FINGERPRINT {
            Some(expected) if expected == self.fingerprint => Ok(()),
            _ => Err("slash stock-chat submission blocked: client fingerprint unverified".into()),
        }
    }
}

/// A distinct observation stage. Only the server stage can establish receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashStage {
    ClientSubmitted,
    ClientAcknowledged,
    OutgoingMethod,
    ServerReceived,
}

/// Source evidence for one stage, scoped to an attempt and client generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashObservation {
    pub attempt_id: Uuid,
    pub line: String,
    pub fingerprint: String,
    pub generation: u64,
    pub stage: SlashStage,
    pub independent_source: bool,
}

/// Receipt state. A claimed acknowledgement alone cannot imply acceptance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashReceipt {
    pub attempt_id: Uuid,
    pub line: String,
    pub fingerprint: String,
    pub generation: u64,
    pub client_submitted: bool,
    pub client_acknowledged: bool,
    pub outgoing_method: bool,
    pub server_received: bool,
    receipt_binding_verified: bool,
}

impl SlashReceipt {
    /// Start a unique receipt for one command in one client generation.
    pub fn new(candidate: PlainShoutCandidate, generation: u64) -> Self {
        Self {
            attempt_id: Uuid::new_v4(),
            line: candidate.line,
            fingerprint: candidate.fingerprint,
            generation,
            client_submitted: false,
            client_acknowledged: false,
            outgoing_method: false,
            server_received: false,
            receipt_binding_verified: false,
        }
    }

    /// Fold independently sourced, exact-context observations only.
    pub fn observe(&mut self, observation: &SlashObservation) -> Result<(), String> {
        if !observation.independent_source
            || observation.attempt_id != self.attempt_id
            || observation.line != self.line
            || observation.fingerprint != self.fingerprint
            || observation.generation != self.generation
        {
            return Err("slash observation has no matching independent provenance".into());
        }
        match observation.stage {
            SlashStage::ClientSubmitted => self.client_submitted = true,
            SlashStage::ClientAcknowledged if self.client_submitted => {
                self.client_acknowledged = true;
            }
            SlashStage::OutgoingMethod if self.client_submitted => self.outgoing_method = true,
            SlashStage::ServerReceived if self.outgoing_method => self.server_received = true,
            _ => return Err("slash observation arrived before its prerequisite".into()),
        }
        Ok(())
    }

    /// Native acceptance needs a verified cross-source binding, unavailable
    /// until the client and server emit a correlation key for this attempt.
    pub fn accepted(&self) -> bool {
        self.receipt_binding_verified
            && self.client_acknowledged
            && self.outgoing_method
            && self.server_received
    }
}
