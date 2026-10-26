//! The identity config (contract C4): `config/auth-providers.toml`, its
//! validation, and the kill switches read from the environment.
//!
//! `load` is pure over the file text and an env lookup, so the tests need no
//! process environment. A provider that cannot run is skipped with a
//! [`ConfigWarning`]; a file that is wrong is a [`ConfigError`] and the server
//! refuses to start.

use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::provider::{ProviderId, ProviderKind};

/// The scopes an `oidc` provider asks for when the file names none.
const OIDC_DEFAULT_SCOPES: [&str; 3] = ["openid", "email", "profile"];

/// Issuer hosts whose `email` claim is mutable, so `trusted_email` is refused.
const MICROSOFT_ISSUER_HOSTS: [&str; 2] = ["login.microsoftonline.com", "login.live.com"];

/// The validated identity config the server runs with.
#[derive(Debug)]
pub struct IdentityConfig {
    /// `https://` base of the public site, no trailing slash.
    pub public_base_url: String,
    /// Whether new accounts may be created through a provider.
    pub signup_open: bool,
    /// Sign-up rate limits.
    pub limits: Limits,
    /// Optional links shown on the game's login screen.
    pub links: Links,
    /// The providers that run; skipped and refused ones are not here.
    pub providers: Vec<ProviderConfig>,
    /// The kill switches read at startup.
    pub kill: KillSwitches,
}

/// Sign-up rate limits (`[limits]`). Missing keys take the example's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Limits {
    /// Sign-ups allowed from one client address per day.
    pub signups_per_ip_per_day: u32,
    /// Sign-ups allowed across the whole server per hour.
    pub signups_global_per_hour: u32,
    /// Minimum age of the provider account before it may sign up.
    pub min_provider_account_age_days: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            signups_per_ip_per_day: 3,
            signups_global_per_hour: 30,
            min_provider_account_age_days: 0,
        }
    }
}

/// Optional links for the login screen (`[links]`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Links {
    /// The project's website.
    pub website: Option<String>,
    /// The page where players manage their account.
    pub account: Option<String>,
    /// The community forums.
    pub forums: Option<String>,
}

/// How an `oidc` provider authenticates itself at the token endpoint. The
/// default is `client_secret_basic` (RFC 6749 2.3.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TokenAuthMethod {
    /// The client id and secret go in an `Authorization: Basic` header.
    #[default]
    ClientSecretBasic,
    /// The client id and secret go in the form body.
    ClientSecretPost,
}

impl TokenAuthMethod {
    /// The method's name as the config file and the discovery document spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            TokenAuthMethod::ClientSecretBasic => "client_secret_basic",
            TokenAuthMethod::ClientSecretPost => "client_secret_post",
        }
    }
}

/// One `[[provider]]` table after validation.
#[derive(Debug)]
pub struct ProviderConfig {
    /// The config id, unique and matching the id pattern.
    pub id: ProviderId,
    /// Which provider implementation runs.
    pub kind: ProviderKind,
    /// The name the login screen shows.
    pub display_name: String,
    /// The OAuth client id registered at the provider.
    pub client_id: String,
    /// The client secret, read from the variable `client_secret_env` names.
    pub client_secret: Secret,
    /// The OIDC issuer URL; `None` for the other kinds.
    pub issuer: Option<String>,
    /// The scopes asked for; empty for the other kinds.
    pub scopes: Vec<String>,
    /// Whether the provider's email claim may be trusted as verified.
    pub trusted_email: bool,
    /// How an `oidc` provider sends its secret at the token endpoint; the
    /// default for the other kinds, which ignore it.
    pub token_auth: TokenAuthMethod,
}

/// A provider client secret. `Debug` never shows the value (R7).
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// The secret value, for the one request that needs it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// The three env kill switches, read once at startup. `"1"` means on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KillSwitches {
    /// `CIMMERIA_IDP_DISABLED`: no providers run.
    pub idp_disabled: bool,
    /// `CIMMERIA_IDP_SIGNUP_DISABLED`: `signup_open` is forced to false.
    pub signup_disabled: bool,
    /// `CIMMERIA_SOAP_RELAY_DISABLED`: the SOAP relay is off.
    pub soap_relay_disabled: bool,
}

impl KillSwitches {
    /// Reads the three switches through `env`.
    pub fn from_env(env: &dyn Fn(&str) -> Option<String>) -> KillSwitches {
        let on = |name: &str| env(name).as_deref() == Some("1");
        KillSwitches {
            idp_disabled: on("CIMMERIA_IDP_DISABLED"),
            signup_disabled: on("CIMMERIA_IDP_SIGNUP_DISABLED"),
            soap_relay_disabled: on("CIMMERIA_SOAP_RELAY_DISABLED"),
        }
    }
}

/// A provider that was skipped without stopping the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    /// The variable named by `client_secret_env` is unset or empty.
    SecretMissing { provider: String },
    /// A `dev_mock` provider while developer mode is off.
    DevMockRefused { provider: String },
}

impl ConfigWarning {
    /// The fixed reason string for the `WARN` line.
    pub fn reason(&self) -> &'static str {
        match self {
            ConfigWarning::SecretMissing { .. } => "secret_missing",
            ConfigWarning::DevMockRefused { .. } => "dev_mock_refused",
        }
    }

    /// The config id of the skipped provider.
    pub fn provider(&self) -> &str {
        match self {
            ConfigWarning::SecretMissing { provider }
            | ConfigWarning::DevMockRefused { provider } => provider,
        }
    }
}

/// A config that must stop the server from starting. Each message names the
/// field or provider at fault; none echoes a secret or the file text.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file is not TOML, or misses a required field; the text names the field.
    #[error("the providers file is not valid TOML or misses a required field: {0}")]
    Parse(String),
    /// The file could not be read (other than being absent).
    #[error("cannot read the providers file: {0}")]
    Read(#[source] std::io::Error),
    /// A `client_secret` key appears in the file; secrets go only in env variables.
    #[error("the providers file holds a literal client_secret; use client_secret_env")]
    LiteralSecret,
    /// `public_base_url` is not an `https://` URL.
    #[error("public_base_url must be an https:// URL")]
    InsecureBaseUrl,
    /// `public_base_url` ends with a slash.
    #[error("public_base_url must not end with a slash")]
    TrailingSlash,
    /// An `oidc` provider's `token_endpoint_auth_method` is not one of the two.
    #[error("provider {0}: token_endpoint_auth_method must be client_secret_basic or client_secret_post")]
    UnknownAuthMethod(String),
    /// A provider's `kind` is not discord, oidc or dev_mock.
    #[error("provider {0}: kind must be discord, oidc or dev_mock")]
    UnknownKind(String),
    /// Two providers share an id.
    #[error("provider id {0} is used twice")]
    DuplicateId(String),
    /// A provider id is not 1-32 characters of a-z, 0-9, `_` or `-`.
    #[error("provider id {0} must be 1-32 characters of a-z, 0-9, _ or -")]
    BadId(String),
    /// An `oidc` provider names no issuer.
    #[error("provider {0}: kind oidc needs an issuer")]
    MissingIssuer(String),
    /// An `oidc` provider's issuer is not an `https://` URL.
    #[error("provider {0}: issuer must be an https:// URL")]
    InsecureIssuer(String),
    /// `trusted_email` is set for a Microsoft issuer, whose email claim is mutable.
    #[error("provider {0}: trusted_email is refused for Microsoft issuers")]
    MicrosoftEmailUntrusted(String),
}

/// The file as TOML deserializes it, before validation.
#[derive(Deserialize)]
struct RawConfig {
    public_base_url: String,
    #[serde(default = "default_true")]
    signup_open: bool,
    #[serde(default)]
    limits: Limits,
    #[serde(default)]
    links: Links,
    #[serde(default, rename = "provider")]
    providers: Vec<RawProvider>,
}

/// One `[[provider]]` table as written.
#[derive(Deserialize)]
struct RawProvider {
    id: String,
    kind: String,
    display_name: String,
    client_id: String,
    client_secret_env: String,
    issuer: Option<String>,
    scopes: Option<Vec<String>>,
    #[serde(default)]
    trusted_email: bool,
    token_endpoint_auth_method: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Parses and validates the config text (C4). `env` looks up variables by
/// name; `developer_mode` decides whether `dev_mock` providers may run.
pub fn load(
    text: &str,
    env: &dyn Fn(&str) -> Option<String>,
    developer_mode: bool,
) -> Result<(IdentityConfig, Vec<ConfigWarning>), ConfigError> {
    let table: toml::Table =
        toml::from_str(text).map_err(|e| ConfigError::Parse(e.message().to_owned()))?;
    if names_key(&toml::Value::Table(table), "client_secret") {
        return Err(ConfigError::LiteralSecret);
    }
    let raw: RawConfig =
        toml::from_str(text).map_err(|e| ConfigError::Parse(e.message().to_owned()))?;
    check_base_url(&raw.public_base_url)?;

    let kill = KillSwitches::from_env(env);
    let mut warnings = Vec::new();
    let mut providers = Vec::new();
    let mut seen = HashSet::new();
    for raw_provider in raw.providers {
        let RawProvider {
            id: raw_id,
            kind: raw_kind,
            display_name,
            client_id,
            client_secret_env,
            issuer,
            scopes,
            trusted_email,
            token_endpoint_auth_method,
        } = raw_provider;
        let id = ProviderId::new(&raw_id).ok_or_else(|| ConfigError::BadId(raw_id.clone()))?;
        if !seen.insert(id.clone()) {
            return Err(ConfigError::DuplicateId(raw_id));
        }
        let kind = ProviderKind::parse(&raw_kind)
            .ok_or_else(|| ConfigError::UnknownKind(raw_id.clone()))?;
        if kind == ProviderKind::DevMock && !developer_mode {
            skipped(
                &mut warnings,
                ConfigWarning::DevMockRefused { provider: raw_id },
            );
            continue;
        }
        let (scopes, token_auth) = match kind {
            ProviderKind::Oidc => {
                check_oidc_issuer(&raw_id, issuer.as_deref(), trusted_email)?;
                let auth = parse_auth_method(&raw_id, token_endpoint_auth_method.as_deref())?;
                let scopes = scopes
                    .unwrap_or_else(|| OIDC_DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect());
                (scopes, auth)
            }
            _ => (Vec::new(), TokenAuthMethod::default()),
        };
        let Some(secret) = env(&client_secret_env).filter(|value| !value.is_empty()) else {
            skipped(
                &mut warnings,
                ConfigWarning::SecretMissing { provider: raw_id },
            );
            continue;
        };
        providers.push(ProviderConfig {
            id,
            kind,
            display_name,
            client_id,
            client_secret: Secret(secret),
            issuer,
            scopes,
            trusted_email,
            token_auth,
        });
    }
    if kill.idp_disabled {
        providers.clear();
    }

    let config = IdentityConfig {
        public_base_url: raw.public_base_url,
        signup_open: raw.signup_open && !kill.signup_disabled,
        limits: raw.limits,
        links: raw.links,
        providers,
        kill,
    };
    Ok((config, warnings))
}

/// Reads and validates the config file at `path`. A missing file means no
/// providers (C6); the kill switches still apply.
pub fn load_file(
    path: &Path,
    developer_mode: bool,
) -> Result<(IdentityConfig, Vec<ConfigWarning>), ConfigError> {
    let env = |name: &str| std::env::var(name).ok();
    match std::fs::read_to_string(path) {
        Ok(text) => load(&text, &env, developer_mode),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let kill = KillSwitches::from_env(&env);
            let config = IdentityConfig {
                public_base_url: String::new(),
                signup_open: !kill.signup_disabled,
                limits: Limits::default(),
                links: Links::default(),
                providers: Vec::new(),
                kill,
            };
            Ok((config, Vec::new()))
        }
        Err(err) => Err(ConfigError::Read(err)),
    }
}

/// Records a skipped provider and logs it at WARN with its reason.
fn skipped(warnings: &mut Vec<ConfigWarning>, warning: ConfigWarning) {
    tracing::warn!(
        reason = warning.reason(),
        provider = warning.provider(),
        "identity provider skipped"
    );
    warnings.push(warning);
}

/// `public_base_url` must be an `https://` URL with no trailing slash.
fn check_base_url(raw: &str) -> Result<(), ConfigError> {
    let parsed = url::Url::parse(raw).ok();
    if !parsed.is_some_and(|u| u.scheme() == "https" && u.host_str().is_some()) {
        return Err(ConfigError::InsecureBaseUrl);
    }
    if raw.ends_with('/') {
        return Err(ConfigError::TrailingSlash);
    }
    Ok(())
}

/// An `oidc` provider needs an `https://` issuer, and may not claim
/// `trusted_email` from a Microsoft issuer.
fn check_oidc_issuer(
    id: &str,
    issuer: Option<&str>,
    trusted_email: bool,
) -> Result<(), ConfigError> {
    let issuer = issuer.ok_or_else(|| ConfigError::MissingIssuer(id.to_owned()))?;
    let parsed = url::Url::parse(issuer)
        .ok()
        .filter(|u| u.scheme() == "https" && u.host_str().is_some())
        .ok_or_else(|| ConfigError::InsecureIssuer(id.to_owned()))?;
    let host = parsed.host_str().unwrap_or_default().trim_end_matches('.');
    if trusted_email && MICROSOFT_ISSUER_HOSTS.contains(&host) {
        return Err(ConfigError::MicrosoftEmailUntrusted(id.to_owned()));
    }
    Ok(())
}

/// Reads an `oidc` provider's `token_endpoint_auth_method`: absent or
/// `client_secret_basic` is Basic, `client_secret_post` is Post, anything else
/// is refused.
fn parse_auth_method(id: &str, raw: Option<&str>) -> Result<TokenAuthMethod, ConfigError> {
    match raw {
        None | Some("client_secret_basic") => Ok(TokenAuthMethod::ClientSecretBasic),
        Some("client_secret_post") => Ok(TokenAuthMethod::ClientSecretPost),
        Some(_) => Err(ConfigError::UnknownAuthMethod(id.to_owned())),
    }
}

/// True when any table in `value`, at any depth, has a key named `key`.
fn names_key(value: &toml::Value, key: &str) -> bool {
    match value {
        toml::Value::Table(table) => table.iter().any(|(k, v)| k == key || names_key(v, key)),
        toml::Value::Array(items) => items.iter().any(|v| names_key(v, key)),
        _ => false,
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
