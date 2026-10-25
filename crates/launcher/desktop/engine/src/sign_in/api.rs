//! The JSON bodies of the sign-in API `/auth/v1` (contract C6).
//!
//! The server may add fields, so none of these types deny unknown fields. The
//! types that carry a token or a link grant have a `Debug` that hides it.

use std::fmt;

use serde::Deserialize;

/// `signed_in` (C6): the access token, the rotated refresh token and the account.
/// It is also the reply of the link calls.
#[derive(Deserialize)]
pub struct SignedIn {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    pub account: AccountView,
}

/// `needs_link` (C6): the provider's identity has no account yet.
#[derive(Deserialize)]
pub struct NeedsLink {
    pub link_grant: String,
    pub identity: IdentityBrief,
    pub hint: String,
    pub signup_open: bool,
}

/// The reply of `POST /token`: the `result` field picks the shape.
#[derive(Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum TokenReply {
    SignedIn(SignedIn),
    NeedsLink(NeedsLink),
}

/// The provider of a `needs_link` reply.
#[derive(Debug, Clone, Deserialize)]
pub struct IdentityBrief {
    pub provider: String,
    pub display_name: String,
}

/// `GET /account` (C6). `account` in `signed_in` has no `identities`.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountView {
    pub account_name: String,
    pub display_name: String,
    #[serde(default)]
    pub identities: Vec<IdentityView>,
}

/// One linked sign-in method of the account.
#[derive(Debug, Clone, Deserialize)]
pub struct IdentityView {
    pub identity_id: i64,
    pub provider: String,
    pub display_name: String,
}

/// `POST /link/start` (C6): the provider page to open for adding a method.
#[derive(Deserialize)]
pub struct LinkStart {
    pub authorize_url: String,
}

/// `POST /launch-ticket` (C6): the ticket the game's login uses. Its `Debug` hides it.
#[derive(Deserialize)]
pub struct TicketView {
    pub account_name: String,
    pub ticket: String,
    pub expires_in: u64,
}

impl fmt::Debug for TicketView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TicketView")
            .field("account_name", &self.account_name)
            .field("ticket", &"<hidden>")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// Every C6 error body: `{"error":"<code>"}`, plus `retry_after` on `429`.
#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    pub error: String,
    #[serde(default)]
    pub retry_after: Option<u64>,
}
