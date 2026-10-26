//! Completing a browser flow with its grant (CT-89).

use std::sync::Arc;

use super::commands::{SignInFailure, SignInStatus};
use super::discovery::Discovery;
use super::driver::{SignInDriver, Site};
use super::flow::Continue;
use super::host_slot::Epoch;
use super::oauth::CodeGrant;
use super::session::SignIn;

impl SignInDriver {
    /// Completes a browser flow with its grant, as its continuation says: a plain
    /// sign-in, or an Add method followed by the account read (CT-89). `epoch` is the
    /// one the original command captured, so a sign-out since voids it.
    pub(super) async fn continue_flow(
        &self,
        site: &Site<'_>,
        discovery: &Discovery,
        grant: CodeGrant,
        cont: Continue,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        let fence = Epoch::new(&self.epoch, epoch);
        match cont {
            Continue::Begin { base, provider } => {
                let slot = self.slot(&base);
                let session = Arc::new(SignIn::new_in(
                    base,
                    self.http.clone(),
                    self.store.clone(),
                    slot,
                ));
                let outcome = session
                    .complete(grant, &fence)
                    .await
                    .map_err(SignInFailure::from)?;
                self.finish(site, discovery, session, outcome, &provider, epoch)
                    .await
            }
            Continue::AddMethod { session, provider } => {
                let outcome = session
                    .complete(grant, &fence)
                    .await
                    .map_err(SignInFailure::from)?;
                let status = self
                    .finish(
                        site,
                        discovery,
                        Arc::clone(&session),
                        outcome,
                        &provider,
                        epoch,
                    )
                    .await?;
                // A reply for another server carries that server's status, not this account.
                if site.base != Some(session.base()) {
                    return Ok(status);
                }
                let view = session.account().await.map_err(SignInFailure::from)?;
                Ok(SignInStatus {
                    account: Some(view.into()),
                    ..status
                })
            }
        }
    }
}
