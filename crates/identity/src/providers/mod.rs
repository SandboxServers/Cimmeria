//! The identity providers the server runs (contract C12). [`build`] turns the
//! validated config into one provider per `[[provider]]` table. A provider
//! that cannot run is skipped with a `WARN` line, as C4 says. The generic
//! OpenID Connect provider lives in [`oidc`], with its discovery and key cache
//! in [`oidc_keys`].

pub mod dev_mock;
pub mod discord;
pub mod oidc;
pub mod oidc_keys;

use std::sync::Arc;

use url::form_urlencoded;

use crate::config::IdentityConfig;
use crate::provider::{IdentityProvider, ProviderKind};
use dev_mock::DevMockProvider;
use discord::DiscordProvider;
use oidc::OidcProvider;

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
                // Discovery is lazy: the provider fetches its document on the
                // first `prepare`, so building it never touches the network.
                let oidc = OidcProvider::new(provider, http.clone());
                list.push(Arc::new(oidc));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    /// Two developer-mode mock tables; only the first may run.
    const CONFIG: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
trusted_email = true

[[provider]]
id = "dev2"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
trusted_email = true
"#;

    #[test]
    fn a_second_dev_mock_table_is_skipped() {
        let env = |name: &str| (name == "CIMMERIA_IDP_DEV_SECRET").then(|| "dev-secret".to_owned());
        let (cfg, _) = config::load(CONFIG, &env, true).expect("the fixture loads");
        let providers = build(&cfg, crate::http::provider_client());
        assert_eq!(providers.list().len(), 1);
        assert_eq!(providers.dev_mock().unwrap().id().as_str(), "dev");
        assert!(providers.get("dev2").is_none());
    }
}
