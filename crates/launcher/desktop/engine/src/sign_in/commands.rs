//! The bridge types of `sign_in_command` (contract C13). The shell forwards these
//! as JSON, and the frontend decodes them, so the names and tags here are the contract.
//!
//! Requests are refused on unknown fields. Requests carry a password or a link
//! code, so `SignInCommand` has no `Debug`: nothing can log it by accident.

use serde::{Deserialize, Serialize};

use super::api::{AccountView, IdentityView};
use super::discovery::ProviderView;
use super::session::NeedsLinkView;
use super::SignInError;

/// One request from the frontend (C13). Every variant carries `schema_version`, which must be `1`.
#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum SignInCommand {
    Inspect {
        schema_version: u32,
    },
    Begin {
        schema_version: u32,
        provider: String,
    },
    Cancel {
        schema_version: u32,
    },
    LinkNew {
        schema_version: u32,
    },
    LinkExistingPassword {
        schema_version: u32,
        account_name: String,
        password: String,
        acknowledge_password_removal: bool,
    },
    LinkExistingCode {
        schema_version: u32,
        link_code: String,
        acknowledge_password_removal: bool,
    },
    Account {
        schema_version: u32,
    },
    AddMethod {
        schema_version: u32,
        provider: String,
    },
    Unlink {
        schema_version: u32,
        identity_id: i64,
    },
    SignOut {
        schema_version: u32,
    },
}

impl SignInCommand {
    /// The request's `schema_version`. Anything but `1` is refused.
    pub fn schema_version(&self) -> u32 {
        match self {
            Self::Inspect { schema_version }
            | Self::Begin { schema_version, .. }
            | Self::Cancel { schema_version }
            | Self::LinkNew { schema_version }
            | Self::LinkExistingPassword { schema_version, .. }
            | Self::LinkExistingCode { schema_version, .. }
            | Self::Account { schema_version }
            | Self::AddMethod { schema_version, .. }
            | Self::Unlink { schema_version, .. }
            | Self::SignOut { schema_version } => *schema_version,
        }
    }
}

/// The reply to every command (C13). `account` is filled only by `account`,
/// `add_method` and `unlink`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SignInStatus {
    pub schema_version: u32,
    pub available: bool,
    pub server_name: Option<String>,
    pub signup_open: bool,
    pub providers: Vec<ProviderEntry>,
    pub waiting: bool,
    pub state: SignInStateView,
    pub account: Option<AccountPanelView>,
    /// Why the last signed-in Play ended before the game started (`ticket_unavailable`, `sign_in_ended`).
    pub play_refusal: Option<&'static str>,
    /// The credential store's state (C18.1): `session_only`, `sign_out_incomplete`,
    /// `store_unreadable`, or `None` when the store is in order.
    pub storage_warning: Option<&'static str>,
}

/// One provider the server offers, as the sign-in page lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderEntry {
    pub id: String,
    pub display_name: String,
}

impl From<ProviderView> for ProviderEntry {
    fn from(provider: ProviderView) -> Self {
        Self {
            id: provider.id,
            display_name: provider.display_name,
        }
    }
}

/// Where the player stands (C13 `state`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SignInStateView {
    SignedOut,
    NeedsLink {
        provider_display_name: String,
        identity_display_name: String,
        hint: String,
        signup_open: bool,
    },
    SignedIn {
        account_name: String,
        display_name: String,
    },
}

impl SignInStateView {
    /// The `needs_link` state for a pending identity, named by its provider's display name.
    pub fn needs_link(view: &NeedsLinkView, provider_display_name: String) -> Self {
        Self::NeedsLink {
            provider_display_name,
            identity_display_name: view.display_name.clone(),
            hint: view.hint.clone(),
            signup_open: view.signup_open,
        }
    }
}

/// The account view the panel shows (C13 `account`): `GET /account` minus `account_id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountPanelView {
    pub account_name: String,
    pub display_name: String,
    pub identities: Vec<IdentityPanelView>,
}

/// One linked sign-in method. Times are epoch milliseconds (UTC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdentityPanelView {
    pub identity_id: i64,
    pub provider: String,
    pub display_name: String,
    pub created_at: i64,
    pub last_used_at: i64,
}

impl From<AccountView> for AccountPanelView {
    fn from(view: AccountView) -> Self {
        Self {
            account_name: view.account_name,
            display_name: view.display_name,
            identities: view.identities.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<IdentityView> for IdentityPanelView {
    fn from(view: IdentityView) -> Self {
        Self {
            identity_id: view.identity_id,
            provider: view.provider,
            display_name: view.display_name,
            created_at: view.created_at,
            last_used_at: view.last_used_at,
        }
    }
}

/// Why a command failed (C13). Never carries a code, token, grant or password,
/// except the C6 error code of a `server` failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum SignInFailure {
    UnsupportedSchema,
    Unavailable,
    Busy,
    BrowserFailed {
        url: String,
    },
    Offline,
    Cancelled,
    Denied,
    TimedOut,
    NoPendingLink,
    PasswordRemovalNotAcknowledged,
    Server {
        code: String,
        retry_after: Option<u64>,
    },
    Protocol,
    Io,
}

impl From<SignInError> for SignInFailure {
    fn from(error: SignInError) -> Self {
        match error {
            SignInError::Offline => Self::Offline,
            SignInError::Unsupported => Self::Unavailable,
            SignInError::Cancelled | SignInError::Superseded => Self::Cancelled,
            SignInError::Denied => Self::Denied,
            SignInError::TimedOut => Self::TimedOut,
            SignInError::Protocol => Self::Protocol,
            server @ SignInError::Server(_) => Self::Server {
                code: server.code().to_owned(),
                retry_after: server.retry_after(),
            },
        }
    }
}
