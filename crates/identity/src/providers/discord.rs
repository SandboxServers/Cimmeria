//! Discord, the OAuth2 provider (contracts C3 and C4). The server is a
//! confidential client: it sends the code, its PKCE verifier and its secret to
//! Discord's token endpoint, then reads the signed-in user from `@me`.

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::StatusCode;
use serde::Deserialize;
use url::Url;

use super::encode_pairs;
use crate::config::{ProviderConfig, Secret};
use crate::http::{read_capped, PROVIDER_BODY_CAP};
use crate::provider::{IdentityProvider, ProviderError, ProviderId, VerifiedIdentity};

/// The issuer every Discord identity carries.
const ISSUER: &str = "https://discord.com";

/// The base every production call uses; tests point `with_base` elsewhere.
const DEFAULT_BASE: &str = "https://discord.com";

/// Discord's snowflake epoch (2015-01-01) in Unix milliseconds (contract C7).
const DISCORD_EPOCH_MS: i64 = 1_420_070_400_000;

/// The Discord provider. `Debug` is derived: the secret's own `Debug` hides it.
#[derive(Debug)]
pub struct DiscordProvider {
    id: ProviderId,
    display_name: String,
    client_id: String,
    secret: Secret,
    trusted_email: bool,
    /// The base URL without a trailing slash.
    base: String,
    http: reqwest::Client,
}

impl DiscordProvider {
    /// Builds the provider against Discord's public endpoints.
    pub fn new(cfg: &ProviderConfig, http: reqwest::Client) -> DiscordProvider {
        let base = Url::parse(DEFAULT_BASE).expect("the default Discord base is a valid URL");
        DiscordProvider::with_base(cfg, http, base)
    }

    /// Builds the provider against `base`, for tests that stub Discord.
    pub fn with_base(cfg: &ProviderConfig, http: reqwest::Client, base: Url) -> DiscordProvider {
        DiscordProvider {
            id: cfg.id.clone(),
            display_name: cfg.display_name.clone(),
            client_id: cfg.client_id.clone(),
            secret: cfg.client_secret.clone(),
            trusted_email: cfg.trusted_email,
            base: base.as_str().trim_end_matches('/').to_owned(),
            http,
        }
    }

    /// Trades the code for a bearer token at Discord's token endpoint.
    async fn token(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<String, ProviderError> {
        let body = encode_pairs(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", pkce_verifier),
            ("client_id", &self.client_id),
            ("client_secret", self.secret.expose()),
        ]);
        let resp = self
            .http
            .post(format!("{}/api/oauth2/token", self.base))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .header(ACCEPT, "application/json")
            .body(body)
            .send()
            .await
            .map_err(transport_error)?;
        let status = resp.status();
        if status == StatusCode::BAD_REQUEST || status == StatusCode::UNAUTHORIZED {
            return Err(ProviderError::Token("invalid_grant"));
        }
        if !status.is_success() {
            return Err(ProviderError::Http(status.as_u16()));
        }
        let bytes = read_capped(resp, PROVIDER_BODY_CAP).await?;
        let reply: TokenReply =
            serde_json::from_slice(&bytes).map_err(|_| ProviderError::BadResponse("json"))?;
        Ok(reply.access_token)
    }

    /// Reads the signed-in user from `@me` with the bearer token.
    async fn user(&self, token: &str) -> Result<DiscordUser, ProviderError> {
        let resp = self
            .http
            .get(format!("{}/api/users/@me", self.base))
            .bearer_auth(token)
            .header(ACCEPT, "application/json")
            .send()
            .await
            .map_err(transport_error)?;
        if !resp.status().is_success() {
            return Err(ProviderError::Http(resp.status().as_u16()));
        }
        let bytes = read_capped(resp, PROVIDER_BODY_CAP).await?;
        serde_json::from_slice(&bytes).map_err(|_| ProviderError::BadResponse("json"))
    }
}

#[async_trait::async_trait]
impl IdentityProvider for DiscordProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn trusted_email(&self) -> bool {
        self.trusted_email
    }

    fn authorize_url(
        &self,
        state: &str,
        pkce_challenge: &str,
        _nonce: &str,
        redirect_uri: &str,
    ) -> String {
        // Discord issues no id_token, so the nonce has nothing to bind to.
        let query = encode_pairs(&[
            ("response_type", "code"),
            ("client_id", &self.client_id),
            ("scope", "identify email"),
            ("state", state),
            ("redirect_uri", redirect_uri),
            ("code_challenge", pkce_challenge),
            ("code_challenge_method", "S256"),
            ("prompt", "consent"),
        ]);
        format!("{}/oauth2/authorize?{query}", self.base)
    }

    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        _nonce: &str,
        redirect_uri: &str,
    ) -> Result<VerifiedIdentity, ProviderError> {
        let token = self.token(code, pkce_verifier, redirect_uri).await?;
        let user = self.user(&token).await?;
        let snowflake: u64 = user
            .id
            .parse()
            .map_err(|_| ProviderError::BadResponse("subject"))?;
        let display_source = user
            .global_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(user.username);
        Ok(VerifiedIdentity {
            provider: self.id.clone(),
            issuer: ISSUER.to_owned(),
            subject: user.id,
            display_name: VerifiedIdentity::sanitized_display_name(&display_source),
            email: user.email.map(|email| email.to_ascii_lowercase()),
            email_verified: user.verified == Some(true),
            provider_account_created_ms: Some(discord_created_ms(snowflake)),
        })
    }
}

/// The account creation time, in Unix milliseconds, encoded in a snowflake id.
pub fn discord_created_ms(snowflake: u64) -> i64 {
    (snowflake >> 22) as i64 + DISCORD_EPOCH_MS
}

/// Maps a transport failure to the provider error the spans report.
fn transport_error(err: reqwest::Error) -> ProviderError {
    if err.is_timeout() {
        ProviderError::Timeout
    } else {
        ProviderError::BadResponse("transport")
    }
}

/// The token endpoint's answer; only the access token is read.
#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
}

/// The `@me` answer. Not `Debug`, so a user never reaches a log line.
#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    #[serde(default)]
    global_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    verified: Option<bool>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::config;

    /// A snowflake from 2016, with the age C7 gives it.
    const SNOWFLAKE: &str = "175928847299117063";

    /// The client secret the tests load; it must never reach an error or a `Debug` line.
    const SECRET: &str = "s3cr3t-test-value";

    const CONFIG: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "discord"
kind = "discord"
display_name = "Discord"
client_id = "000000000000000000"
client_secret_env = "CIMMERIA_IDP_DISCORD_SECRET"
trusted_email = true
"#;

    /// Loads the one Discord table from [`CONFIG`] with the secret set.
    fn discord_config() -> ProviderConfig {
        let env = |name: &str| (name == "CIMMERIA_IDP_DISCORD_SECRET").then(|| SECRET.to_owned());
        let (mut cfg, _) = config::load(CONFIG, &env, false).expect("the fixture loads");
        cfg.providers.remove(0)
    }

    /// A provider pointed at the stub server.
    fn provider_for(server: &MockServer) -> DiscordProvider {
        let base = Url::parse(&server.uri()).expect("the stub server URI parses");
        DiscordProvider::with_base(&discord_config(), crate::http::provider_client(), base)
    }

    /// Mounts a token answer that accepts any form body.
    async fn mount_token(server: &MockServer, status: u16) {
        Mock::given(method("POST"))
            .and(path("/api/oauth2/token"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(json!({ "access_token": "tok" })),
            )
            .mount(server)
            .await;
    }

    #[test]
    fn discord_authorize_url_carries_pkce_and_state() {
        let server_base = Url::parse("https://discord.example.org/").expect("parses");
        let provider = DiscordProvider::with_base(
            &discord_config(),
            crate::http::provider_client(),
            server_base,
        );
        let raw = provider.authorize_url(
            "state-1",
            "challenge-1",
            "nonce-1",
            "https://play.example.org/auth/v1/callback/discord",
        );
        assert!(raw.contains("scope=identify%20email"), "{raw}");
        let url = Url::parse(&raw).expect("the authorize URL parses");
        assert_eq!(url.path(), "/oauth2/authorize");
        let query: HashMap<String, String> = url.query_pairs().into_owned().collect();
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["client_id"], "000000000000000000");
        assert_eq!(query["scope"], "identify email");
        assert_eq!(query["state"], "state-1");
        assert_eq!(
            query["redirect_uri"],
            "https://play.example.org/auth/v1/callback/discord"
        );
        assert_eq!(query["code_challenge"], "challenge-1");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["prompt"], "consent");
        assert_eq!(query.len(), 8);
    }

    #[tokio::test]
    async fn discord_exchange_maps_the_user() {
        let server = MockServer::start().await;
        mount_token(&server, 200).await;
        Mock::given(method("GET"))
            .and(path("/api/users/@me"))
            .and(header("authorization", "Bearer tok"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": SNOWFLAKE,
                "username": "ada",
                "global_name": "Ada L",
                "email": "Ada@Example.org",
                "verified": true
            })))
            .mount(&server)
            .await;

        let identity = provider_for(&server)
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .expect("the exchange succeeds");
        assert_eq!(identity.subject, SNOWFLAKE);
        assert_eq!(identity.display_name, "Ada L");
        assert_eq!(identity.email.as_deref(), Some("ada@example.org"));
        assert!(identity.email_verified);
        assert_eq!(identity.issuer, "https://discord.com");
        assert_eq!(identity.provider.as_str(), "discord");
        assert_eq!(identity.provider_account_created_ms, Some(1462015105796));
    }

    #[tokio::test]
    async fn discord_unverified_email_is_not_verified() {
        let server = MockServer::start().await;
        mount_token(&server, 200).await;
        Mock::given(method("GET"))
            .and(path("/api/users/@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": SNOWFLAKE,
                "username": "ada",
                "email": "ada@example.org",
                "verified": false
            })))
            .mount(&server)
            .await;

        let identity = provider_for(&server)
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .expect("the exchange succeeds");
        assert_eq!(identity.display_name, "ada", "the username is the fallback");
        assert!(!identity.email_verified);
    }

    #[tokio::test]
    async fn discord_refusals_map_to_errors() {
        let refused = MockServer::start().await;
        mount_token(&refused, 400).await;
        let err = provider_for(&refused)
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .err();
        assert_eq!(err, Some(ProviderError::Token("invalid_grant")));

        let broken = MockServer::start().await;
        mount_token(&broken, 200).await;
        Mock::given(method("GET"))
            .and(path("/api/users/@me"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&broken)
            .await;
        let err = provider_for(&broken)
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .err();
        assert_eq!(err, Some(ProviderError::Http(500)));

        let large = MockServer::start().await;
        mount_token(&large, 200).await;
        Mock::given(method("GET"))
            .and(path("/api/users/@me"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'a'; 70 * 1024]))
            .mount(&large)
            .await;
        let err = provider_for(&large)
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .err();
        assert_eq!(err, Some(ProviderError::BadResponse("too_large")));
    }

    #[tokio::test]
    async fn discord_secret_never_in_errors_or_debug() {
        let server = MockServer::start().await;
        mount_token(&server, 500).await;
        let provider = provider_for(&server);
        assert!(!format!("{provider:?}").contains(SECRET));
        let err = provider
            .exchange("code", "verifier", "nonce", "https://play.example.org/cb")
            .await
            .expect_err("a 500 from the token endpoint fails");
        assert!(!err.to_string().contains(SECRET));
        assert!(!format!("{err:?}").contains(SECRET));
    }

    #[test]
    fn discord_snowflake_age() {
        assert_eq!(
            discord_created_ms(SNOWFLAKE.parse().expect("parses")),
            1462015105796
        );
    }
}
