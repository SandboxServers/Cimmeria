use super::*;

/// The test secret. Distinct from every real value; the redaction test
/// checks that it never reaches a `Debug` string.
const SECRET: &str = "s3cr3t-test-value";

const EXAMPLE: &str = include_str!("../../../config/auth-providers.toml.example");

/// Both example secrets are set.
fn both_secrets(name: &str) -> Option<String> {
    match name {
        "CIMMERIA_IDP_DISCORD_SECRET" | "CIMMERIA_IDP_GOOGLE_SECRET" => Some(SECRET.to_owned()),
        _ => None,
    }
}

fn no_secrets(_: &str) -> Option<String> {
    None
}

/// Every secret the inline fixtures below name is set.
fn all_secrets(name: &str) -> Option<String> {
    match name {
        "CIMMERIA_IDP_DEV_SECRET"
        | "CIMMERIA_IDP_DISCORD_SECRET"
        | "CIMMERIA_IDP_GOOGLE_SECRET" => Some(SECRET.to_owned()),
        _ => None,
    }
}

/// One Discord provider, the minimum valid file.
const MINIMAL: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "discord"
kind = "discord"
display_name = "Discord"
client_id = "000000000000000000"
client_secret_env = "CIMMERIA_IDP_DISCORD_SECRET"
"#;

fn refusal(text: &str) -> ConfigError {
    load(text, &all_secrets, false).expect_err("the config must be refused")
}

#[test]
fn example_file_loads() {
    let (cfg, warnings) = load(EXAMPLE, &both_secrets, false).expect("the example loads");
    assert_eq!(cfg.providers.len(), 2);
    assert!(cfg.signup_open);
    assert_eq!(
        (
            cfg.limits.signups_per_ip_per_day,
            cfg.limits.signups_global_per_hour,
            cfg.limits.min_provider_account_age_days
        ),
        (3, 30, 0)
    );
    assert!(
        cfg.links.website.is_some() && cfg.links.account.is_some() && cfg.links.forums.is_some()
    );
    assert!(warnings.is_empty());

    let changed = EXAMPLE.replace("signups_per_ip_per_day = 3", "signups_per_ip_per_day = 7");
    let (cfg, _) = load(&changed, &both_secrets, false).expect("loads");
    assert_eq!(cfg.limits.signups_per_ip_per_day, 7);
}

#[test]
fn missing_secret_skips_the_provider() {
    let only_discord = |name: &str| {
        if name.contains("DISCORD") {
            both_secrets(name)
        } else {
            None
        }
    };
    let (cfg, warnings) =
        load(EXAMPLE, &only_discord, false).expect("a missing secret is a warning, not an error");
    assert_eq!(cfg.providers.len(), 1);
    assert_eq!(cfg.providers[0].id.as_str(), "discord");
    assert_eq!(
        warnings,
        vec![ConfigWarning::SecretMissing {
            provider: "google".to_owned()
        }]
    );
    assert_eq!(warnings[0].reason(), "secret_missing");
}

#[test]
fn refuses_literal_secret_http_base_and_bad_ids() {
    let literal = MINIMAL.replace(
        "client_secret_env = \"CIMMERIA_IDP_DISCORD_SECRET\"",
        "client_secret = \"literal\"",
    );
    assert!(matches!(refusal(&literal), ConfigError::LiteralSecret));

    let http = MINIMAL.replace("https://", "http://");
    assert!(matches!(refusal(&http), ConfigError::InsecureBaseUrl));

    let slash = MINIMAL.replace("play.example.org\"", "play.example.org/\"");
    assert!(matches!(refusal(&slash), ConfigError::TrailingSlash));

    let bad_id = MINIMAL.replace("id = \"discord\"", "id = \"Discord\"");
    assert!(matches!(refusal(&bad_id), ConfigError::BadId(_)));

    let bad_kind = MINIMAL.replace("kind = \"discord\"", "kind = \"saml\"");
    assert!(matches!(refusal(&bad_kind), ConfigError::UnknownKind(_)));

    let no_client_id = MINIMAL.replace("client_id = \"000000000000000000\"\n", "");
    assert!(matches!(
        refusal(&no_client_id),
        ConfigError::Parse(m) if m.contains("client_id")
    ));

    let twice = format!(
        "{MINIMAL}\n[[provider]]\nid = \"discord\"\nkind = \"discord\"\ndisplay_name = \"Again\"\n\
         client_id = \"x\"\nclient_secret_env = \"CIMMERIA_IDP_DISCORD_SECRET\"\n"
    );
    assert!(matches!(refusal(&twice), ConfigError::DuplicateId(_)));
}

#[test]
fn refuses_trusted_microsoft_email() {
    let text = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "work"
kind = "oidc"
display_name = "Work"
issuer = "https://login.microsoftonline.com/x/v2.0"
client_id = "example"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
trusted_email = true
"#;
    assert!(matches!(
        refusal(text),
        ConfigError::MicrosoftEmailUntrusted(_)
    ));

    let no_issuer = text.replace(
        "issuer = \"https://login.microsoftonline.com/x/v2.0\"\n",
        "",
    );
    assert!(matches!(refusal(&no_issuer), ConfigError::MissingIssuer(_)));

    let http_issuer = text.replace(
        "https://login.microsoftonline.com/x/v2.0",
        "http://issuer.example.org",
    );
    assert!(matches!(
        refusal(&http_issuer),
        ConfigError::InsecureIssuer(_)
    ));

    let dotted = text.replace("login.microsoftonline.com/", "login.microsoftonline.com./");
    assert!(matches!(
        refusal(&dotted),
        ConfigError::MicrosoftEmailUntrusted(_)
    ));
}

#[test]
fn dev_mock_needs_developer_mode() {
    let text = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Dev"
client_id = "dev"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
"#;
    let (off, warnings) = load(text, &all_secrets, false).expect("loads");
    assert!(off.providers.is_empty());
    assert_eq!(
        warnings,
        vec![ConfigWarning::DevMockRefused {
            provider: "dev".to_owned()
        }]
    );
    assert_eq!(warnings[0].reason(), "dev_mock_refused");

    let (on, warnings) = load(text, &all_secrets, true).expect("loads");
    assert_eq!(on.providers.len(), 1);
    assert_eq!(on.providers[0].kind, ProviderKind::DevMock);
    assert!(warnings.is_empty());
}

#[test]
fn kill_switches_apply() {
    let idp_off = |name: &str| match name {
        "CIMMERIA_IDP_DISABLED" => Some("1".to_owned()),
        other => all_secrets(other),
    };
    let (cfg, _) = load(EXAMPLE, &idp_off, false).expect("loads");
    assert!(cfg.providers.is_empty());
    assert!(cfg.kill.idp_disabled);

    let signup_off = |name: &str| match name {
        "CIMMERIA_IDP_SIGNUP_DISABLED" => Some("1".to_owned()),
        other => all_secrets(other),
    };
    let (cfg, _) = load(EXAMPLE, &signup_off, false).expect("loads");
    assert!(!cfg.signup_open);
    assert_eq!(cfg.providers.len(), 2);

    let soap_off = |name: &str| match name {
        "CIMMERIA_SOAP_RELAY_DISABLED" => Some("1".to_owned()),
        other => all_secrets(other),
    };
    let (cfg, _) = load(EXAMPLE, &soap_off, false).expect("loads");
    assert!(cfg.kill.soap_relay_disabled);
    assert!(!cfg.kill.idp_disabled);
    assert_eq!(cfg.providers.len(), 2);

    let not_one = |name: &str| match name {
        "CIMMERIA_IDP_DISABLED" => Some("true".to_owned()),
        other => all_secrets(other),
    };
    let (cfg, _) = load(EXAMPLE, &not_one, false).expect("loads");
    assert!(!cfg.kill.idp_disabled);
}

#[test]
fn secret_debug_is_redacted() {
    let (cfg, _) = load(EXAMPLE, &both_secrets, false).expect("loads");
    let debug = format!("{cfg:?}");
    assert!(!debug.contains(SECRET));
    assert!(debug.contains("Secret(***)"));
    assert_eq!(cfg.providers[0].client_secret.expose(), SECRET);
}

#[test]
fn missing_file_means_no_providers() {
    let (cfg, warnings) = load_file(Path::new("config/__absent__/auth-providers.toml"), false)
        .expect("an absent file is not an error");
    assert!(cfg.providers.is_empty());
    assert!(warnings.is_empty());
    assert!(load(MINIMAL, &no_secrets, false)
        .expect("loads")
        .0
        .providers
        .is_empty());
}

#[test]
fn oidc_auth_method_defaults_to_basic_and_reads_post() {
    let text = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "work"
kind = "oidc"
display_name = "Work"
issuer = "https://issuer.example.org"
client_id = "example"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
"#;
    let (cfg, _) = load(text, &all_secrets, false).expect("loads");
    assert_eq!(
        cfg.providers[0].token_auth,
        TokenAuthMethod::ClientSecretBasic
    );

    let post = text.replace(
        "client_secret_env = \"CIMMERIA_IDP_DEV_SECRET\"",
        "client_secret_env = \"CIMMERIA_IDP_DEV_SECRET\"\ntoken_endpoint_auth_method = \"client_secret_post\"",
    );
    let (cfg, _) = load(&post, &all_secrets, false).expect("loads");
    assert_eq!(
        cfg.providers[0].token_auth,
        TokenAuthMethod::ClientSecretPost
    );
    assert_eq!(cfg.providers[0].token_auth.as_str(), "client_secret_post");
}

#[test]
fn unknown_auth_method_is_a_config_error() {
    let text = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "work"
kind = "oidc"
display_name = "Work"
issuer = "https://issuer.example.org"
client_id = "example"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
token_endpoint_auth_method = "private_key_jwt"
"#;
    assert!(matches!(
        refusal(text),
        ConfigError::UnknownAuthMethod(id) if id == "work"
    ));
}

#[test]
fn discord_ignores_the_auth_method_key() {
    let text = MINIMAL.replace(
        "client_secret_env = \"CIMMERIA_IDP_DISCORD_SECRET\"",
        "client_secret_env = \"CIMMERIA_IDP_DISCORD_SECRET\"\ntoken_endpoint_auth_method = \"private_key_jwt\"",
    );
    let (cfg, _) = load(&text, &all_secrets, false).expect("the key is ignored for discord");
    assert_eq!(
        cfg.providers[0].token_auth,
        TokenAuthMethod::ClientSecretBasic
    );
}
