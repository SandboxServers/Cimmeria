//! The decision behind a `versionInfoRequest`: given the version a client
//! holds for one cooked-data category, does it get a plain `onVersionInfo`
//! or a full resync of the category?

use super::super::resources::ResourceCache;

/// What the server does for one `versionInfoRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReply {
    /// No resource cache is loaded, or it lacks this category. Echo the
    /// client's version with no invalidation: it keeps its local cache.
    NoServerData { client_version: u32 },
    /// The client already holds the served version: nothing to send but the
    /// reply.
    UpToDate { version: u32 },
    /// The versions differ. The client is made to hold exactly the server's
    /// category: `invalidate_all = 1` with `RequiredUpdates = entry_count`,
    /// every entry pushed, then the server's version stamped (see
    /// [`super::task`]). Covers categories with and without Cimmeria
    /// overrides alike, since the push serves the category as loaded, with
    /// its overrides applied.
    FullResync {
        client_version: u32,
        server_version: u32,
        entry_count: u32,
    },
}

impl VersionReply {
    /// Decide the reply for `category_id` given the client's cached version.
    pub fn decide(cache: Option<&ResourceCache>, category_id: u32, client_version: u32) -> Self {
        match cache.and_then(|c| c.category(category_id)) {
            None => Self::NoServerData { client_version },
            Some(cat) => Self::from_parts(cat.metadata, cat.elements.len() as u32, client_version),
        }
    }

    /// [`Self::decide`] once the category is known to be served.
    pub fn from_parts(server_version: u32, entry_count: u32, client_version: u32) -> Self {
        if client_version == server_version {
            Self::UpToDate {
                version: server_version,
            }
        } else {
            Self::FullResync {
                client_version,
                server_version,
                entry_count,
            }
        }
    }

    /// The server's version of the category, when it serves one.
    pub fn server_version(&self) -> Option<u32> {
        match self {
            Self::NoServerData { .. } => None,
            Self::UpToDate { version } => Some(*version),
            Self::FullResync { server_version, .. } => Some(*server_version),
        }
    }

    /// Low-cardinality label for the branch taken (`outcome=` in the log).
    pub fn outcome(&self) -> &'static str {
        match self {
            Self::NoServerData { .. } => "no_server_data",
            Self::UpToDate { .. } => "up_to_date",
            Self::FullResync { .. } => "full_resync",
        }
    }

    /// Why that branch was taken (`reason=` in the log).
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NoServerData { .. } => "category_not_served",
            Self::UpToDate { .. } => "versions_match",
            Self::FullResync { .. } => "version_mismatch",
        }
    }
}

/// The `Version` the resync's opening `onVersionInfo` stamps on the client.
///
/// The client writes the reply's `Version` into its cache PAK's `MetaData` as
/// soon as it handles the reply (`ServerSource_SetVersion`, `0x00479e90`),
/// before any entry arrives. Stamping the server's real version there would
/// make a client that disconnects mid-push look up to date on its next login,
/// with a half-filled category. So the opening reply stamps a value that can
/// never equal the server's, and a closing reply, queued behind every entry
/// on the same reliable channel, stamps the real one: the client only takes
/// it after it has taken every entry.
pub fn resync_pending_version(server_version: u32) -> u32 {
    !server_version
}
