//! The provider identity a sign-in can ask the player to link (C6 `needs_link`).

use std::fmt;

use super::api::NeedsLink;

/// A provider identity with no account yet (C6 `needs_link`). Its `Debug` hides the grant.
#[derive(Clone, PartialEq, Eq)]
pub struct NeedsLinkView {
    /// The single-use grant the link calls take. It is never shown or logged.
    pub link_grant: String,
    pub provider: String,
    pub display_name: String,
    /// `none` or `email_match`: whether the server saw a likely existing account.
    pub hint: String,
    pub signup_open: bool,
}

impl fmt::Debug for NeedsLinkView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NeedsLinkView")
            .field("link_grant", &"<hidden>")
            .field("provider", &self.provider)
            .field("hint", &self.hint)
            .field("signup_open", &self.signup_open)
            .finish()
    }
}

impl From<NeedsLink> for NeedsLinkView {
    fn from(link: NeedsLink) -> Self {
        Self {
            link_grant: link.link_grant,
            provider: link.identity.provider,
            display_name: link.identity.display_name,
            hint: link.hint,
            signup_open: link.signup_open,
        }
    }
}
