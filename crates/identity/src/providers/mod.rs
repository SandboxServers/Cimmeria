//! The identity provider implementations (contract C12). The generic OpenID
//! Connect provider lives in [`oidc`], with its discovery and key cache in
//! [`oidc_keys`]. The Discord and developer-mock providers, and the `build`
//! that turns the config into providers, arrive with CT-12.

pub mod oidc;
pub mod oidc_keys;
