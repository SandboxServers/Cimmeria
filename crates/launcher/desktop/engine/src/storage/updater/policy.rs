use super::{Error, Offer};
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use reqwest::Url;
use semver::Version;
use serde::Deserialize;
use std::collections::BTreeMap;

pub const MAX_ARTIFACT: usize = 256 * 1024 * 1024;
pub const MAX_FEED: usize = 64 * 1024;
/// Native composition only: neither configuration nor artifact URLs cross IPC.
#[derive(Clone)]
pub struct Config {
    pub(crate) endpoint: Url,
    pub(crate) public_key: String,
    pub(crate) current: Version,
    pub(crate) platform: String,
    pub(crate) hosts: Vec<String>,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) loopback: bool,
}
impl Config {
    pub fn new(
        endpoint: &str,
        public_key: &str,
        current: &str,
        platform: &str,
        hosts: Vec<String>,
    ) -> Result<Self, Error> {
        let config = Self {
            endpoint: Url::parse(endpoint).map_err(|_| Error::Policy)?,
            public_key: public_key.to_owned(),
            current: Version::parse(current).map_err(|_| Error::Policy)?,
            platform: platform.to_owned(),
            hosts,
            #[cfg(any(test, feature = "test-support"))]
            loopback: false,
        };
        if !matches!(
            platform,
            "darwin-aarch64" | "darwin-x86_64" | "windows-x86_64"
        ) || public_key.len() > 4096
        {
            return Err(Error::Policy);
        }
        config.key()?;
        config.allow(&config.endpoint)?;
        Ok(config)
    }
    pub(crate) fn key(&self) -> Result<PublicKey, Error> {
        PublicKey::decode(&decode(&self.public_key)?).map_err(|_| Error::Signature)
    }
    pub(crate) fn allow(&self, url: &Url) -> Result<(), Error> {
        #[cfg(any(test, feature = "test-support"))]
        if self.loopback
            && url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
        {
            return Ok(());
        }
        if url.scheme() != "https"
            || url.port_or_known_default() != Some(443)
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !url
                .host_str()
                .is_some_and(|host| self.hosts.iter().any(|allowed| allowed == host))
        {
            return Err(Error::Policy);
        }
        Ok(())
    }
    pub(crate) fn newer(&self, version: &str) -> Result<Version, Error> {
        let version = Version::parse(version.trim_start_matches('v')).map_err(|_| Error::Feed)?;
        if !version.pre.is_empty() || version.cmp_precedence(&self.current).is_le() {
            return Err(Error::NotNewer);
        }
        Ok(version)
    }
    pub(crate) fn verify(&self, offer: &Offer, bytes: &[u8]) -> Result<(), Error> {
        if bytes.is_empty() || bytes.len() > MAX_ARTIFACT {
            return Err(Error::Size);
        }
        let announced = self.newer(&offer.version)?;
        self.allow(&Url::parse(&offer.url).map_err(|_| Error::Policy)?)?;
        let signature =
            Signature::decode(&decode(&offer.signature)?).map_err(|_| Error::Signature)?;
        // Tauri updater v2.13.1 contract: verify both the payload and global
        // signature BEFORE consuming the trusted comment's version field.
        self.key()?
            .verify(bytes, &signature, true)
            .map_err(|_| Error::Signature)?;
        let versions: Vec<_> = signature
            .trusted_comment()
            .split('\t')
            .filter_map(|field| field.strip_prefix("version:"))
            .collect();
        if versions.len() != 1 {
            return Err(Error::SignedVersion);
        }
        let signed = Version::parse(versions[0].trim_start_matches('v'))
            .map_err(|_| Error::SignedVersion)?;
        if signed != announced {
            return Err(Error::SignedVersion);
        }
        Ok(())
    }
}
fn decode(value: &str) -> Result<String, Error> {
    let bytes = STANDARD.decode(value).map_err(|_| Error::Signature)?;
    String::from_utf8(bytes).map_err(|_| Error::Signature)
}
#[derive(Deserialize)]
struct Feed {
    version: String,
    #[serde(default)]
    notes: String,
    platforms: BTreeMap<String, Artifact>,
}
#[derive(Deserialize)]
struct Artifact {
    url: String,
    signature: String,
}
pub(crate) fn parse(config: &Config, bytes: &[u8]) -> Result<Option<Offer>, Error> {
    if bytes.len() > MAX_FEED {
        return Err(Error::Size);
    }
    let feed: Feed = serde_json::from_slice(bytes).map_err(|_| Error::Feed)?;
    if feed.version.len() > 128 || feed.notes.len() > 4096 {
        return Err(Error::Feed);
    }
    match config.newer(&feed.version) {
        Err(Error::NotNewer) => return Ok(None),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    let artifact = feed
        .platforms
        .get(&config.platform)
        .ok_or(Error::Platform)?;
    if artifact.signature.len() > 8192 || artifact.url.len() > 2048 {
        return Err(Error::Feed);
    }
    config.allow(&Url::parse(&artifact.url).map_err(|_| Error::Policy)?)?;
    Ok(Some(Offer {
        id: uuid::Uuid::new_v4(),
        version: feed.version,
        notes: feed.notes,
        url: artifact.url.clone(),
        signature: artifact.signature.clone(),
    }))
}
