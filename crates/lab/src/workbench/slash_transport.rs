//! Bounded stock-chat slash candidate and fail-closed receipt stages.
//!
//! The hub-pass UAT used `/gmsendgmshout 1 HPUshout1` through
//! `ChatMod.onTextAccepted` and saw a native server receipt. No supervisor
//! producer currently proves the installed client fingerprint, generation,
//! or a cross-source receipt binding for a new command attempt.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The only shape with a documented stock-chat server receipt so far.
pub const PLAIN_SHOUT_WORD: &str = "/gmsendgmshout";

/// A pure parser candidate; this type grants no right to submit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlainShoutCandidate {
    pub line: String,
}

impl PlainShoutCandidate {
    /// Refuse characters and optional shapes whose tokenizer is untraced.
    pub fn parse(line: &str) -> Result<Self, String> {
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
        Ok(Self { line: line.into() })
    }
}

/// A distinct observation stage. An untrusted stage claim proves nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlashStage {
    ClientSubmitted,
    ClientAcknowledged,
    OutgoingMethod,
    ServerReceived,
}

/// A raw observation proposal. Its fields can be forged by a caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashObservation {
    pub attempt_id: Uuid,
    pub line: String,
    pub fingerprint: String,
    pub generation: u64,
    pub stage: SlashStage,
}

/// A future supervisor-owned identity, never constructed from RPC arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
struct VerifiedIdentity {
    fingerprint: String,
    generation: u64,
}

/// Four separate evidence states; `None` means no trusted observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashReceipt {
    pub attempt_id: Uuid,
    pub line: String,
    identity: Option<VerifiedIdentity>,
    client_submitted: Option<bool>,
    client_acknowledged: Option<bool>,
    outgoing_method: Option<bool>,
    server_received: Option<bool>,
    trusted_cross_source_binding: bool,
}

impl SlashReceipt {
    /// Start an unbound attempt. No caller can supply an identity here.
    pub fn new(candidate: PlainShoutCandidate) -> Self {
        Self {
            attempt_id: Uuid::new_v4(),
            line: candidate.line,
            identity: None,
            client_submitted: None,
            client_acknowledged: None,
            outgoing_method: None,
            server_received: None,
            trusted_cross_source_binding: false,
        }
    }

    /// Read one stage without treating a missing producer as a negative event.
    pub fn stage(&self, stage: SlashStage) -> Option<bool> {
        match stage {
            SlashStage::ClientSubmitted => self.client_submitted,
            SlashStage::ClientAcknowledged => self.client_acknowledged,
            SlashStage::OutgoingMethod => self.outgoing_method,
            SlashStage::ServerReceived => self.server_received,
        }
    }

    /// Reject raw claims until a supervisor-owned identity producer exists.
    /// Even a matching line and attempt ID cannot authenticate a network row.
    pub fn observe(&mut self, observation: &SlashObservation) -> Result<(), String> {
        let Some(identity) = &self.identity else {
            return Err("slash observation blocked: no supervisor-verified identity".into());
        };
        if observation.attempt_id != self.attempt_id
            || observation.line != self.line
            || observation.fingerprint != identity.fingerprint
            || observation.generation != identity.generation
        {
            return Err("slash observation does not match verified context".into());
        }
        // A future producer must authenticate each source before it may
        // promote this proposal. There is no such producer in DW-24B.
        Err("slash observation blocked: trusted stage producer unavailable".into())
    }

    /// A forged acknowledgement or server row cannot establish acceptance.
    pub fn accepted(&self) -> bool {
        self.trusted_cross_source_binding
            && self.client_acknowledged == Some(true)
            && self.outgoing_method == Some(true)
            && self.server_received == Some(true)
    }
}
