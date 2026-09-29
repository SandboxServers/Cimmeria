use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::client_setup::LoginServer;
use crate::state::atomic_write;

/// Default manifest URL. Points at the GitHub Release that owns the
/// rolling `content-current` tag — operators publish a new manifest by
/// `--clobber`-uploading `manifest.json` to that tag. See
/// [`docs/client/launcher-distribution-setup.md`](../../docs/client/launcher-distribution-setup.md).
pub const DEFAULT_MANIFEST_URL: &str =
    "https://github.com/SandboxServers/Cimmeria/releases/download/content-current/manifest.json";

/// SAS URL for log uploads (PUT-only, scoped to the `logs/` prefix), baked
/// in at compile time from the `LAUNCHER_LOG_SAS_URL` env var. In release CI
/// this is injected from the repo secret; local dev builds without the env
/// var get `None` and the log-upload button is disabled with a friendly note.
pub const LOG_UPLOAD_SAS_URL: Option<&str> = option_env!("LAUNCHER_LOG_SAS_URL");

/// Current persisted config-file schema. Bump when a breaking change to
/// the on-disk shape lands; the load path then refuses to deserialise
/// against the wrong schema rather than silently defaulting fields.
///
/// - 1: the first schema. A file without `schema_version` is schema 1.
/// - 2: the default telemetry `auth_url` moved from the local admin API
///   to the public server's login port; see [`LauncherConfig::load`].
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

/// The schema a file without `schema_version` was written with.
const FIRST_SCHEMA_VERSION: u32 = 1;

/// The schema-1 default `telemetry.auth_url`: the admin API on the
/// player's own machine, which a remote player never runs.
const V1_DEFAULT_TELEMETRY_AUTH_URL: &str = "http://localhost:8443/api";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(
        "Config schema version {got} unsupported (expected {expected}). \
        Delete the config file to regenerate with defaults."
    )]
    UnsupportedSchema { got: u32, expected: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    /// Bumped on incompatible changes to this struct's on-disk shape.
    /// A missing field means schema 1, the schema files were written
    /// with before the field existed, so they get schema 1's migrations.
    #[serde(default = "first_schema_version")]
    pub schema_version: u32,
    pub install_path: PathBuf,
    /// Login servers written into the client's `LoginInternal.lua`; see
    /// [`crate::client_setup::login_servers`]. Replaces the old
    /// `server_host`, which fed the retired `.rdata` patch and is ignored
    /// when an old config still has it.
    #[serde(default = "crate::client_setup::login_servers::default_servers")]
    pub login_servers: Vec<LoginServer>,
    pub manifest_url: String,
    #[serde(default)]
    pub telemetry: TelemetrySettings,
    #[serde(default)]
    pub client_patches: ClientPatchesSettings,
}

/// The always-injected `cimmeria-client-patches` DLL (Black Market
/// plan D2). Independent of [`TelemetrySettings`]: gameplay must not
/// depend on the telemetry opt-in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientPatchesSettings {
    /// Opt-out switch. False ⇒ `SGW.exe` launches without the DLL, and
    /// features it restores (the Black Market window) stay off.
    #[serde(default = "default_client_patches_enabled")]
    pub enabled: bool,
    /// Load this DLL instead of the one bundled with the launcher. For
    /// testing a local build; unset for players.
    #[serde(default)]
    pub dll_override: Option<PathBuf>,
}

impl Default for ClientPatchesSettings {
    fn default() -> Self {
        Self {
            enabled: default_client_patches_enabled(),
            dll_override: None,
        }
    }
}

fn default_client_patches_enabled() -> bool {
    true
}

/// User-controllable telemetry preferences. Lives in `LauncherConfig`
/// (rarely-changing user choice); per-session runtime state lives in
/// [`crate::state::TelemetryState`] so config-file rewrites don't
/// fire on every tick.
///
/// Telemetry is opt-in: nothing is sent until the player ticks the box.
/// The field was `enabled`, defaulting to true, and every launcher that
/// saved its config wrote `"enabled": true` without the player choosing
/// it. The new name means those old files load opted out; serde ignores
/// the old key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TelemetrySettings {
    /// Opt-in switch. False ⇒ no token fetch, no tail, no upload.
    #[serde(default)]
    pub opted_in: bool,
    /// True once the player answered the launcher's telemetry prompt,
    /// either way, so it stops asking.
    #[serde(default)]
    pub prompt_answered: bool,
    /// Base URL where the telemetry auth handshake
    /// (`POST /auth/dev-session`) lives. Defaults to the public server's
    /// SOAP login port, which serves the telemetry routes (decision
    /// @Cadacious, 2026-09-29); a local dev server sets
    /// `http://localhost:8443/api` (the admin API) or
    /// `http://localhost:8081/api`.
    #[serde(default = "default_telemetry_auth_url")]
    pub auth_url: String,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            opted_in: false,
            prompt_answered: false,
            auth_url: default_telemetry_auth_url(),
        }
    }
}

/// The public server's login port, a host every player already reaches:
/// it is in the default [`login servers`], so the plain-http endpoint
/// policy accepts it.
///
/// [`login servers`]: crate::client_setup::login_servers::default_servers
pub const DEFAULT_TELEMETRY_AUTH_URL: &str = "http://play.cimmeria.app:8081/api";

fn default_telemetry_auth_url() -> String {
    DEFAULT_TELEMETRY_AUTH_URL.to_string()
}

fn first_schema_version() -> u32 {
    FIRST_SCHEMA_VERSION
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            install_path: default_install_path(),
            login_servers: crate::client_setup::login_servers::default_servers(),
            manifest_url: DEFAULT_MANIFEST_URL.to_string(),
            telemetry: TelemetrySettings::default(),
            client_patches: ClientPatchesSettings::default(),
        }
    }
}

impl LauncherConfig {
    /// Read the config, migrating an older schema to the current one.
    ///
    /// A migrated config is written back straight away, so each
    /// migration runs exactly once per file: a value the player sets
    /// afterwards (say, `auth_url` back to `http://localhost:8443/api`
    /// for a local dev server) is never migrated again. A failed
    /// write-back is logged and the migrated config is still returned.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let data = std::fs::read_to_string(path)?;
        let mut cfg: LauncherConfig = serde_json::from_str(&data)?;
        if cfg.schema_version == CONFIG_SCHEMA_VERSION {
            return Ok(cfg);
        }
        if cfg.schema_version != FIRST_SCHEMA_VERSION {
            return Err(ConfigError::UnsupportedSchema {
                got: cfg.schema_version,
                expected: CONFIG_SCHEMA_VERSION,
            });
        }
        cfg.migrate_v1_to_v2();
        if let Err(e) = cfg.save(path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "launcher config migrated in memory but not saved; it migrates again next start"
            );
        }
        Ok(cfg)
    }

    /// Schema 1 -> 2: move the old default telemetry `auth_url` (the local
    /// admin API) to the new default (the public login port). Only the
    /// exact old default string moves; any value the player chose stays.
    fn migrate_v1_to_v2(&mut self) {
        if self.telemetry.auth_url == V1_DEFAULT_TELEMETRY_AUTH_URL {
            tracing::info!(
                from = V1_DEFAULT_TELEMETRY_AUTH_URL,
                to = DEFAULT_TELEMETRY_AUTH_URL,
                "launcher config: telemetry auth_url moved to the new default"
            );
            self.telemetry.auth_url = default_telemetry_auth_url();
        }
        self.schema_version = 2;
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let bytes = serde_json::to_vec_pretty(self)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }
}

pub fn config_path() -> PathBuf {
    exe_dir().join("launcher-config.json")
}

pub fn ledger_path() -> PathBuf {
    exe_dir().join("uploaded.json")
}

pub fn telemetry_state_path() -> PathBuf {
    exe_dir().join("telemetry-state.json")
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Default install directory. Lives under `%LOCALAPPDATA%` so the launcher
/// can write without UAC elevation — `%ProgramFiles%` requires admin and
/// produces an opaque mid-extract failure for non-admin users.
fn default_install_path() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local).join("Stargate Worlds");
    }
    // Non-Windows fallback used for tests and Linux dev builds.
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/Stargate Worlds");
    }
    PathBuf::from("Stargate Worlds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        let cfg = LauncherConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            install_path: PathBuf::from("X"),
            login_servers: vec![LoginServer {
                name: "Y".into(),
                url: "http://y:8081".into(),
            }],
            manifest_url: "Z".into(),
            telemetry: TelemetrySettings {
                opted_in: true,
                prompt_answered: true,
                auth_url: "http://test/api".into(),
            },
            client_patches: ClientPatchesSettings {
                enabled: false,
                dll_override: Some(PathBuf::from("D")),
            },
        };
        cfg.save(&path).unwrap();
        let loaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(loaded.install_path, PathBuf::from("X"));
        assert_eq!(loaded.login_servers[0].name, "Y");
        assert_eq!(loaded.manifest_url, "Z");
        assert!(loaded.telemetry.opted_in, "the opt-in must roundtrip");
        assert!(loaded.telemetry.prompt_answered);
        assert_eq!(loaded.client_patches, cfg.client_patches);
    }

    /// A config written before the client-patches DLL existed loads
    /// with the DLL on: always-inject is the default, and an upgrade
    /// must not leave the Black Market silently off.
    #[test]
    fn load_legacy_config_without_client_patches_defaults_to_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"install_path":"X","server_host":"Y","manifest_url":"Z","telemetry":{"enabled":false}}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert!(cfg.client_patches.enabled);
        assert_eq!(cfg.client_patches.dll_override, None);
        assert!(!cfg.telemetry.opted_in);
    }

    /// The opt-out is independent of telemetry in the other direction too.
    #[test]
    fn client_patches_opt_out_is_independent_of_telemetry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"install_path":"X","server_host":"Y","manifest_url":"Z","client_patches":{"enabled":false}}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert!(!cfg.client_patches.enabled);
        assert!(!cfg.telemetry.opted_in);
    }

    // Telemetry is opt-in. A config written before `telemetry` existed
    // loads opted out, and so does one saved by an opt-out launcher,
    // which wrote `"enabled": true` on the player's behalf: only an
    // explicit `opted_in` turns it on.
    #[test]
    fn load_legacy_config_without_telemetry_field_is_opted_out() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert!(!cfg.telemetry.opted_in);
        assert!(!cfg.telemetry.prompt_answered);
    }

    #[test]
    fn a_saved_opt_out_era_enabled_flag_does_not_opt_in() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"install_path":"X","manifest_url":"Z","telemetry":{"enabled":true,"auth_url":"http://a/api"}}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert!(
            !cfg.telemetry.opted_in,
            "the old default-on flag must not count as consent"
        );
        assert_eq!(cfg.telemetry.auth_url, "http://a/api");
    }

    // A config from a launcher that still had `server_host` loads, and gets
    // the default login server list.
    #[test]
    fn load_config_with_the_retired_server_host_uses_default_login_servers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert_eq!(
            cfg.login_servers,
            crate::client_setup::login_servers::default_servers()
        );
    }

    #[test]
    fn telemetry_settings_default_is_opted_out() {
        let t = TelemetrySettings::default();
        assert!(!t.opted_in);
        assert!(!t.prompt_answered);
    }

    #[test]
    fn load_missing_file_errors() {
        assert!(LauncherConfig::load(&PathBuf::from("/no/such/path.json")).is_err());
    }

    // Legacy config files written before schema_version existed should
    // load cleanly — `#[serde(default)]` populates the missing field.
    #[test]
    fn load_legacy_without_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let cfg = LauncherConfig::load(&path).unwrap();
        assert_eq!(cfg.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(cfg.install_path, PathBuf::from("X"));
    }

    fn write(dir: &tempfile::TempDir, json: &str) -> PathBuf {
        let path = dir.path().join("c.json");
        std::fs::write(&path, json).unwrap();
        path
    }

    /// A schema-1 file that stored the old default auth URL moves to the
    /// public login port, and the move is saved to disk.
    #[test]
    fn v1_config_with_the_old_default_auth_url_migrates_to_the_new_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            &dir,
            r#"{"schema_version":1,"install_path":"X","manifest_url":"Z","telemetry":{"opted_in":true,"auth_url":"http://localhost:8443/api"}}"#,
        );
        let cfg = LauncherConfig::load(&path).unwrap();
        assert_eq!(cfg.schema_version, 2);
        assert_eq!(cfg.telemetry.auth_url, DEFAULT_TELEMETRY_AUTH_URL);
        assert!(cfg.telemetry.opted_in, "other settings survive");

        let on_disk: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(on_disk["schema_version"], 2);
        assert_eq!(on_disk["telemetry"]["auth_url"], DEFAULT_TELEMETRY_AUTH_URL);
    }

    /// Files from before `schema_version` existed are schema 1 and get
    /// the same migration.
    #[test]
    fn a_config_without_schema_version_gets_the_v1_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            &dir,
            r#"{"install_path":"X","manifest_url":"Z","telemetry":{"auth_url":"http://localhost:8443/api"}}"#,
        );
        let cfg = LauncherConfig::load(&path).unwrap();
        assert_eq!(cfg.telemetry.auth_url, DEFAULT_TELEMETRY_AUTH_URL);
    }

    /// Only the exact old default moves; a URL the player chose stays.
    #[test]
    fn v1_config_with_a_custom_auth_url_keeps_it() {
        for custom in [
            "http://localhost:8443/api/",
            "http://127.0.0.1:8443/api",
            "https://telemetry.example.org/api",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let json = serde_json::json!({
                "schema_version": 1,
                "install_path": "X",
                "manifest_url": "Z",
                "telemetry": { "auth_url": custom },
            });
            let path = write(&dir, &json.to_string());
            let cfg = LauncherConfig::load(&path).unwrap();
            assert_eq!(cfg.schema_version, 2);
            assert_eq!(cfg.telemetry.auth_url, custom);
        }
    }

    /// The migration runs once: a local dev who sets the old localhost URL
    /// again after migrating keeps it on every later load.
    #[test]
    fn localhost_set_after_the_migration_sticks() {
        let dir = tempfile::tempdir().unwrap();
        let path = write(
            &dir,
            r#"{"schema_version":1,"install_path":"X","manifest_url":"Z","telemetry":{"auth_url":"http://localhost:8443/api"}}"#,
        );
        let mut cfg = LauncherConfig::load(&path).unwrap();
        cfg.telemetry.auth_url = V1_DEFAULT_TELEMETRY_AUTH_URL.into();
        cfg.save(&path).unwrap();

        let reloaded = LauncherConfig::load(&path).unwrap();
        assert_eq!(reloaded.telemetry.auth_url, V1_DEFAULT_TELEMETRY_AUTH_URL);
        let again = LauncherConfig::load(&path).unwrap();
        assert_eq!(again.telemetry.auth_url, V1_DEFAULT_TELEMETRY_AUTH_URL);
    }

    #[test]
    fn a_fresh_config_uses_the_public_login_port() {
        assert_eq!(
            LauncherConfig::default().telemetry.auth_url,
            "http://play.cimmeria.app:8081/api"
        );
    }

    // Future-schema configs should be rejected explicitly rather than
    // silently coerced (would otherwise erase fields not in the current
    // struct).
    #[test]
    fn load_unsupported_schema_version_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        std::fs::write(
            &path,
            r#"{"schema_version":99,"install_path":"X","server_host":"Y","manifest_url":"Z"}"#,
        )
        .unwrap();
        let err = LauncherConfig::load(&path).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::UnsupportedSchema {
                got: 99,
                expected: CONFIG_SCHEMA_VERSION
            }
        ));
    }

    // Windows-only: the assertion compares against a path built from
    // backslash-separated string literals (`Z:\LocalApp\Stargate Worlds`),
    // which only parse as multi-component paths on Windows. On Linux
    // `PathBuf::from("Z:\\LocalApp")` is a single-component string and
    // `.join("Stargate Worlds")` produces `Z:\LocalApp/Stargate Worlds`
    // — different on the byte level from the expected literal. The
    // production behavior under test (`LOCALAPPDATA` → `<dir>/Stargate
    // Worlds`) is a Windows convention; the cross-platform fallback
    // branches in `default_install_path` are exercised implicitly by
    // every other test that constructs a `LauncherConfig::default()`.
    #[cfg(target_os = "windows")]
    #[test]
    fn default_uses_localappdata_when_set() {
        let prev_local = std::env::var("LOCALAPPDATA").ok();
        std::env::set_var("LOCALAPPDATA", "Z:\\LocalApp");
        let cfg = LauncherConfig::default();
        assert_eq!(
            cfg.install_path,
            PathBuf::from("Z:\\LocalApp\\Stargate Worlds")
        );
        match prev_local {
            Some(v) => std::env::set_var("LOCALAPPDATA", v),
            None => std::env::remove_var("LOCALAPPDATA"),
        }
    }
}
