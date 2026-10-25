//! # cimmeria-identity
//!
//! The identity providers of the Cimmeria server emulator: the provider types
//! (Discord, generic OpenID Connect, and the developer-mode mock), the HTTP
//! client their calls share, and the loader for `config/auth-providers.toml`.
//!
//! This crate sits above `cimmeria-auth` and may depend on it. Nothing below
//! the auth crate may depend on this one, so the login handshake stays
//! independent of the providers. The design and its decisions are in the
//! [campaign ledger](../../../docs/analysis/client-tls-oidc/README.md).

pub mod config;
pub mod http;
pub mod limits;
pub mod link;
pub mod names;
pub mod provider;
