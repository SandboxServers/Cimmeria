//! The sign-in status the shell gets back (C13): the panel state the session and
//! the pending choice give, with the play refusal and the server's providers.

use std::sync::Arc;

use url::Url;

use super::commands::{
    AccountPanelView, ProviderEntry, SignInFailure, SignInStateView, SignInStatus,
};
use super::driver::{guard, SignInDriver, Site};
use super::session::SignIn;

impl SignInDriver {
    /// The status for `site`, with the panel state the session and pending choice give.
    pub(super) async fn reply(
        &self,
        site: &Site<'_>,
        session: Option<&Arc<SignIn>>,
        account: Option<AccountPanelView>,
    ) -> Result<SignInStatus, SignInFailure> {
        let view = self
            .panel_state(session, site.discovery.map(|d| &d.auth_base))
            .await?;
        let play_refusal = guard(site.state)?.play_refusal();
        let waiting = self.flow.is_waiting();
        let discovery = site.discovery;
        Ok(SignInStatus {
            schema_version: 1,
            available: discovery.is_some(),
            server_name: site.server.map(str::to_owned),
            signup_open: discovery.is_some_and(|d| d.signup_open),
            providers: discovery
                .map(|d| {
                    d.providers
                        .iter()
                        .cloned()
                        .map(ProviderEntry::from)
                        .collect()
                })
                .unwrap_or_default(),
            waiting,
            state: view,
            account,
            play_refusal,
        })
    }

    /// Where the player stands: signed in, a live `needs_link` choice for `base`
    /// (the server on screen), or signed out.
    async fn panel_state(
        &self,
        session: Option<&Arc<SignIn>>,
        base: Option<&Url>,
    ) -> Result<SignInStateView, SignInFailure> {
        if let Some(session) = session {
            if let Some(summary) = session.summary().await {
                return Ok(SignInStateView::SignedIn {
                    account_name: summary.account_name,
                    display_name: summary.display_name,
                });
            }
        }
        self.pending.view_for(base)
    }
}
