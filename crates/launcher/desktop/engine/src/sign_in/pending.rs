//! The `needs_link` choice the player has not finished (C13). The driver keeps at
//! most one, for the server it came from, and it expires after [`PENDING_TTL`].

use std::sync::Mutex;
use std::time::{Duration, Instant};

use url::Url;

use super::commands::{SignInFailure, SignInStateView};
use super::driver::guard;
use super::session::NeedsLinkView;
use super::SignInError;

/// How long a `needs_link` choice waits for the player's password or code.
pub(super) const PENDING_TTL: Duration = Duration::from_secs(10 * 60);

/// The link errors that keep the pending choice, so the player can retry it.
const RETRYABLE: [&str; 3] = ["invalid_credentials", "rate_limited", "not_linkable"];

/// A `needs_link` choice the player has not finished.
pub(super) struct PendingLink {
    pub(super) view: NeedsLinkView,
    /// The provider's display name, for the panel.
    pub(super) provider_name: String,
    /// The auth server the choice came from. Link calls go there.
    pub(super) base: Url,
    pub(super) since: Instant,
}

/// The one pending choice the driver may hold.
pub(super) struct PendingSlot {
    link: Mutex<Option<PendingLink>>,
}

impl PendingSlot {
    /// An empty slot: no choice is pending.
    pub(super) fn new() -> Self {
        Self {
            link: Mutex::new(None),
        }
    }

    /// Takes the pending choice. A choice older than [`PENDING_TTL`] is dropped.
    pub(super) fn take(&self) -> Result<Option<PendingLink>, SignInFailure> {
        let taken = guard(&self.link)?.take();
        Ok(taken.filter(|link| link.since.elapsed() < PENDING_TTL))
    }

    /// Keeps `link` as the pending choice.
    pub(super) fn keep(&self, link: PendingLink) -> Result<(), SignInFailure> {
        *guard(&self.link)? = Some(link);
        Ok(())
    }

    /// Drops the pending choice, if any.
    pub(super) fn clear(&self) -> Result<(), SignInFailure> {
        *guard(&self.link)? = None;
        Ok(())
    }

    /// Drops the pending choice unless it came from `base`, the server on screen.
    pub(super) fn drop_unless(&self, base: &Url) -> Result<(), SignInFailure> {
        let mut pending = guard(&self.link)?;
        if pending.as_ref().is_some_and(|link| link.base != *base) {
            *pending = None;
        }
        Ok(())
    }

    /// The panel state the pending choice gives for `base` (the server on screen):
    /// a live `needs_link` choice from that server, or signed out.
    pub(super) fn view_for(&self, base: Option<&Url>) -> Result<SignInStateView, SignInFailure> {
        let pending = guard(&self.link)?;
        let view = match pending.as_ref() {
            Some(link) if link.since.elapsed() < PENDING_TTL && base == Some(&link.base) => {
                SignInStateView::needs_link(&link.view, link.provider_name.clone())
            }
            _ => SignInStateView::SignedOut,
        };
        Ok(view)
    }
}

/// Whether a link failure keeps the pending choice (C13: the player may retry).
pub(super) fn is_retryable(error: &SignInError) -> bool {
    matches!(error, SignInError::Server(_)) && RETRYABLE.contains(&error.code())
}
