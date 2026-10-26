//! Checks that the deploy files agree on the provider secrets. Every
//! `client_secret_env` name in the providers example and every
//! `CIMMERIA_IDP_<X>_SECRET` in `.env.example` must reach the container through
//! `docker/compose.yml`, or the provider is skipped as `secret_missing`.

/// The base compose file, read at build time.
const COMPOSE: &str = include_str!("../../../docker/compose.yml");
/// The operator's environment template, read at build time.
const ENV_EXAMPLE: &str = include_str!("../../../docker/.env.example");
/// The providers example, read at build time.
const PROVIDERS_EXAMPLE: &str = include_str!("../../../config/auth-providers.toml.example");

/// Every value of a `client_secret_env = "<NAME>"` line in the providers example.
fn client_secret_env_names(toml: &str) -> Vec<&str> {
    toml.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("client_secret_env")?;
            let value = rest.trim_start().strip_prefix('=')?.trim();
            value.strip_prefix('"')?.strip_suffix('"')
        })
        .collect()
}

/// Every `CIMMERIA_IDP_<X>_SECRET` name in the environment template, commented or not.
fn idp_secret_names(env: &str) -> Vec<&str> {
    env.lines()
        .filter_map(|line| {
            let line = line.trim().trim_start_matches('#').trim();
            let name = line.split('=').next()?.trim();
            let is_secret = name.starts_with("CIMMERIA_IDP_") && name.ends_with("_SECRET");
            is_secret.then_some(name)
        })
        .collect()
}

#[test]
fn every_example_provider_secret_reaches_the_container() {
    let mut names = client_secret_env_names(PROVIDERS_EXAMPLE);
    names.extend(idp_secret_names(ENV_EXAMPLE));
    assert!(
        names.len() >= 3,
        "the example names too few secrets: {names:?}"
    );
    for name in names {
        let forwarded = format!("{name}: \"${{{name}:-}}\"");
        assert!(
            COMPOSE.contains(&forwarded),
            "{name} is not forwarded in docker/compose.yml as `{forwarded}`"
        );
    }
}
