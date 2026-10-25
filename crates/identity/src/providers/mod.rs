//! The identity providers the server runs (contract C12). [`build`] turns the
//! validated config into one provider per `[[provider]]` table. A provider
//! that cannot run is skipped with a `WARN` line, as C4 says.

pub mod dev_mock;
pub mod discord;

use std::sync::Arc;

use url::form_urlencoded;

use crate::config::IdentityConfig;
use crate::provider::{IdentityProvider, ProviderKind};
use dev_mock::DevMockProvider;
use discord::DiscordProvider;

/// The providers the server runs, in config order.
pub struct Providers {
    list: Vec<Arc<dyn IdentityProvider>>,
    dev_mock: Option<Arc<DevMockProvider>>,
}

impl Providers {
    /// The running provider with this config id, if there is one.
    pub fn get(&self, id: &str) -> Option<&Arc<dyn IdentityProvider>> {
        self.list
            .iter()
            .find(|provider| provider.id().as_str() == id)
    }

    /// Every running provider, in config order.
    pub fn list(&self) -> &[Arc<dyn IdentityProvider>] {
        &self.list
    }

    /// The developer-mode mock provider, when one runs.
    pub fn dev_mock(&self) -> Option<&Arc<DevMockProvider>> {
        self.dev_mock.as_ref()
    }
}

/// Builds the providers the config names, sharing one HTTP client.
pub fn build(cfg: &IdentityConfig, http: reqwest::Client) -> Providers {
    let mut list: Vec<Arc<dyn IdentityProvider>> = Vec::new();
    let mut dev_mock = None;
    for provider in &cfg.providers {
        match provider.kind {
            ProviderKind::Discord => {
                let discord = DiscordProvider::new(provider, http.clone());
                list.push(Arc::new(discord));
            }
            ProviderKind::Oidc => {
                // CT-13 replaces this arm with `oidc::OidcProvider::new`.
                tracing::warn!(
                    reason = "oidc_unavailable",
                    provider = provider.id.as_str(),
                    "identity provider skipped"
                );
            }
            ProviderKind::DevMock => {
                if dev_mock.is_some() {
                    tracing::warn!(
                        reason = "dev_mock_duplicate",
                        provider = provider.id.as_str(),
                        "identity provider skipped"
                    );
                    continue;
                }
                let mock = Arc::new(DevMockProvider::new(provider, &cfg.public_base_url));
                dev_mock = Some(Arc::clone(&mock));
                list.push(mock);
            }
        }
    }
    Providers { list, dev_mock }
}

/// Percent-encodes `pairs` as a query string or a form body. Spaces become
/// `%20`, as Discord's `scope` parameter expects.
fn encode_pairs(pairs: &[(&str, &str)]) -> String {
    form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
        .replace('+', "%20")
}
