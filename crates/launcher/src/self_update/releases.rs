//! Launcher release discovery through the GitHub Releases API.
//!
//! One unauthenticated `GET /repos/<repo>/releases?per_page=100` per check
//! (the anonymous limit is 60 requests an hour per IP). `/releases/latest`
//! alone is not enough: server releases (`v2026-…`) and the
//! `content-current` prerelease share the repository, so the newest release
//! is usually not a launcher. The list is filtered to non-draft,
//! non-prerelease `launcher-*` tags that carry both the exe and its
//! `.sha256`.

use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;
use tracing::debug;

use super::build_info::{tag_date, LAUNCHER_TAG_PREFIX};

/// Where the launcher looks for releases and which hosts it may download
/// from. Production uses [`UpdateEndpoints::github`]; tests point it at a
/// loopback stub.
#[derive(Debug, Clone)]
pub struct UpdateEndpoints {
    /// The releases-list URL.
    pub releases_api: String,
    /// Hosts an asset URL, or any redirect on the way to it, may name.
    pub allowed_hosts: Vec<String>,
    /// Refuse plain http. Off only for the loopback test stub.
    pub require_https: bool,
}

/// The repository whose releases the launcher follows.
pub const RELEASE_REPO: &str = "SandboxServers/Cimmeria";

impl UpdateEndpoints {
    pub fn github() -> Self {
        Self {
            releases_api: format!(
                "https://api.github.com/repos/{RELEASE_REPO}/releases?per_page=100"
            ),
            // github.com serves browser_download_url, which redirects to the
            // release-asset CDN. The CDN host has changed over the years, so
            // both names are accepted.
            allowed_hosts: vec![
                "github.com".into(),
                "objects.githubusercontent.com".into(),
                "release-assets.githubusercontent.com".into(),
            ],
            require_https: true,
        }
    }

    /// The release page for `tag`, the fallback link when anything fails.
    pub fn release_page(tag: &str) -> String {
        format!("https://github.com/{RELEASE_REPO}/releases/tag/{tag}")
    }

    /// The page listing every release, for when there is no tag to name.
    pub fn releases_page() -> String {
        format!("https://github.com/{RELEASE_REPO}/releases")
    }

    /// True when `url` is one the updater may fetch: the right scheme and a
    /// host on the allow-list.
    pub fn url_allowed(&self, url: &reqwest::Url) -> bool {
        let scheme_ok = match url.scheme() {
            "https" => true,
            "http" => !self.require_https,
            _ => false,
        };
        let host_ok = url
            .host_str()
            .is_some_and(|h| self.allowed_hosts.iter().any(|a| a.eq_ignore_ascii_case(h)));
        scheme_ok && host_ok
    }

    /// The updater's HTTP client: a User-Agent (the GitHub API refuses
    /// requests without one), https-only in production, and a redirect
    /// policy that stops at the first hop leaving the allow-list.
    pub fn client(&self, user_agent: &str) -> reqwest::Result<reqwest::Client> {
        let policy_endpoints = self.clone();
        let policy = reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("too many redirects")
            } else if policy_endpoints.url_allowed(attempt.url()) {
                attempt.follow()
            } else {
                let host = attempt.url().host_str().unwrap_or("").to_string();
                attempt.error(format!(
                    "redirect to a host the updater does not trust: {host}"
                ))
            }
        });
        reqwest::Client::builder()
            .user_agent(user_agent)
            .https_only(self.require_https)
            .redirect(policy)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
    }
}

/// One asset of a launcher release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

/// A launcher release the updater could install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LauncherRelease {
    pub tag: String,
    /// `published_at`, Unix seconds UTC.
    pub published_at: i64,
    pub page_url: String,
    /// `sgw-launcher-<tag>.exe`.
    pub exe: ReleaseAsset,
    /// `sgw-launcher-<tag>.exe.sha256`.
    pub sha256: ReleaseAsset,
}

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Debug, Deserialize)]
struct GhAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// The launcher releases in a releases-list body, newest first. Releases
/// the updater cannot use (drafts, prereleases, other tag families, no
/// publish time, or missing the exe or its checksum) are left out, each
/// with a DEBUG line saying why.
pub fn launcher_releases(body: &[u8]) -> Result<Vec<LauncherRelease>, serde_json::Error> {
    let all: Vec<GhRelease> = serde_json::from_slice(body)?;
    let mut out: Vec<LauncherRelease> = all.into_iter().filter_map(to_candidate).collect();
    out.sort_by_key(|r| std::cmp::Reverse(r.published_at));
    Ok(out)
}

fn to_candidate(r: GhRelease) -> Option<LauncherRelease> {
    if !r.tag_name.starts_with(LAUNCHER_TAG_PREFIX) {
        return None;
    }
    let skip = |reason: &'static str| {
        debug!(
            target: "launcher.update",
            event = "release_skipped",
            tag = %r.tag_name,
            reason,
            "launcher release not an update candidate"
        );
    };
    if r.draft {
        skip("draft");
        return None;
    }
    if r.prerelease {
        skip("prerelease");
        return None;
    }
    if tag_date(&r.tag_name).is_none() {
        skip("malformed_tag");
        return None;
    }
    let Some(published_at) = r
        .published_at
        .as_deref()
        .and_then(|p| chrono::DateTime::parse_from_rfc3339(p).ok())
        .map(|d| d.timestamp())
    else {
        skip("no_published_at");
        return None;
    };
    let exe_name = format!("sgw-launcher-{}.exe", r.tag_name);
    let sha_name = format!("{exe_name}.sha256");
    let find = |name: &str| {
        r.assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| ReleaseAsset {
                name: a.name.clone(),
                url: a.browser_download_url.clone(),
                size: a.size,
            })
    };
    let Some(exe) = find(&exe_name) else {
        skip("no_exe_asset");
        return None;
    };
    // Releases before the updater shipped have no checksum; without one
    // the download cannot be verified, so they are never installed.
    let Some(sha256) = find(&sha_name) else {
        skip("no_sha256_asset");
        return None;
    };
    Some(LauncherRelease {
        page_url: if r.html_url.is_empty() {
            UpdateEndpoints::release_page(&r.tag_name)
        } else {
            r.html_url
        },
        tag: r.tag_name,
        published_at,
        exe,
        sha256,
    })
}

#[derive(Debug, Error)]
pub enum FetchError {
    /// 403 with the rate-limit header at zero, or 429.
    #[error("GitHub rate limit reached; the launcher will check again next start")]
    RateLimited,
    /// No connection, DNS failure, timeout.
    #[error("could not reach GitHub ({0})")]
    Offline(String),
    #[error("GitHub answered HTTP {0}")]
    Status(u16),
    #[error("the releases list could not be read: {0}")]
    Parse(String),
}

impl FetchError {
    /// Stable `reason` value for the telemetry row.
    pub fn reason(&self) -> &'static str {
        match self {
            FetchError::RateLimited => "rate_limited",
            FetchError::Offline(_) => "offline",
            FetchError::Status(_) => "http_status",
            FetchError::Parse(_) => "parse_failed",
        }
    }
}

/// True when a response is GitHub's rate-limit refusal rather than some
/// other failure.
pub fn is_rate_limited(status: StatusCode, remaining: Option<&str>) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || (status == StatusCode::FORBIDDEN && remaining.map(str::trim) == Some("0"))
}

/// Fetch and filter the releases list.
pub async fn fetch_launcher_releases(
    http: &reqwest::Client,
    endpoints: &UpdateEndpoints,
) -> Result<Vec<LauncherRelease>, FetchError> {
    let resp = http
        .get(&endpoints.releases_api)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| FetchError::Offline(super::download::describe(&e)))?;
    let status = resp.status();
    let remaining = resp
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    if is_rate_limited(status, remaining.as_deref()) {
        return Err(FetchError::RateLimited);
    }
    if !status.is_success() {
        return Err(FetchError::Status(status.as_u16()));
    }
    let body = resp
        .bytes()
        .await
        .map_err(|e| FetchError::Offline(super::download::describe(&e)))?;
    launcher_releases(&body).map_err(|e| FetchError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("testdata/releases.json");

    // Bug shape: taking the first release, or /releases/latest, picks a
    // server release or content-current; taking any launcher-* picks a
    // draft or one without a checksum.
    #[test]
    fn only_published_launcher_releases_with_both_assets_are_candidates() {
        let got = launcher_releases(FIXTURE.as_bytes()).unwrap();
        let tags: Vec<&str> = got.iter().map(|r| r.tag.as_str()).collect();
        assert_eq!(
            tags,
            ["launcher-20261002-bbbbbbb", "launcher-20261002-aaaaaaa"],
            "newest first; server, content, draft, prerelease and no-checksum releases dropped"
        );
        let newest = &got[0];
        assert_eq!(
            newest.exe.name,
            "sgw-launcher-launcher-20261002-bbbbbbb.exe"
        );
        assert_eq!(newest.exe.size, 21_000_000);
        assert_eq!(
            newest.sha256.name,
            "sgw-launcher-launcher-20261002-bbbbbbb.exe.sha256"
        );
        assert!(newest
            .page_url
            .ends_with("/releases/tag/launcher-20261002-bbbbbbb"));
    }

    #[test]
    fn an_empty_or_garbage_list_is_handled() {
        assert!(launcher_releases(b"[]").unwrap().is_empty());
        assert!(launcher_releases(b"{\"message\":\"Not Found\"}").is_err());
    }

    #[test]
    fn rate_limit_is_403_at_zero_remaining_or_429() {
        assert!(is_rate_limited(StatusCode::TOO_MANY_REQUESTS, None));
        assert!(is_rate_limited(StatusCode::FORBIDDEN, Some("0")));
        assert!(!is_rate_limited(StatusCode::FORBIDDEN, Some("12")));
        assert!(!is_rate_limited(StatusCode::FORBIDDEN, None));
        assert!(!is_rate_limited(StatusCode::OK, Some("0")));
    }

    #[test]
    fn production_urls_must_be_https_on_github_hosts() {
        let e = UpdateEndpoints::github();
        let ok = |u: &str| e.url_allowed(&reqwest::Url::parse(u).unwrap());
        assert!(ok(
            "https://github.com/SandboxServers/Cimmeria/releases/download/t/x.exe"
        ));
        assert!(ok("https://objects.githubusercontent.com/x"));
        assert!(ok("https://release-assets.githubusercontent.com/x"));
        assert!(!ok("http://github.com/x"), "plain http refused");
        assert!(!ok("https://github.com.evil.example/x"));
        assert!(!ok("https://evil.example/github.com"));
        assert!(!ok("file:///C:/x.exe"));
    }
}
