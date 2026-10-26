//! Fixtures for the run tests: made-up categories, index entries, PAKs and
//! mock server routes.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cimmeria_cooked_pak::catalog::{encode_body, CategoryBody, Index, IndexEntry};
use cimmeria_cooked_pak::digest::{category_digest, digest_hex};
use cimmeria_cooked_pak::pak::write_pak;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::cooked_presync::apply::TEMP_SUFFIX;

pub(super) const INDEX_PATH: &str = "/api/cooked/v1/index";
pub(super) const PAK_4: &str = "cat4.pak";
pub(super) const PAK_5: &str = "cat5.pak";

/// The old records of a made-up category, as the stock and the old cache PAK hold them.
pub(super) fn base() -> BTreeMap<u32, Vec<u8>> {
    BTreeMap::from([
        (1, b"one".to_vec()),
        (2, b"two".to_vec()),
        (3, b"three".to_vec()),
    ])
}

/// The records after the served change: key 2 is replaced.
pub(super) fn composed() -> BTreeMap<u32, Vec<u8>> {
    BTreeMap::from([
        (1, b"one".to_vec()),
        (2, b"TWO".to_vec()),
        (3, b"three".to_vec()),
    ])
}

/// A made-up served body of `category_id` at `version`, with its encoded bytes.
pub(super) fn served(category_id: u32, version: u32) -> (CategoryBody, Vec<u8>) {
    let composed = composed();
    let body = CategoryBody {
        category_id,
        version,
        keys: composed.keys().copied().collect(),
        changed: BTreeMap::from([(2, b"TWO".to_vec())]),
        digest: category_digest(
            category_id,
            version,
            composed.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
        ),
    };
    let bytes = encode_body(&body);
    (body, bytes)
}

/// The index entry for a served body, with its real size and digest.
pub(super) fn entry(category_id: u32, pak: &str, served: &(CategoryBody, Vec<u8>)) -> IndexEntry {
    IndexEntry {
        category_id,
        pak: pak.to_owned(),
        version: served.0.version,
        entries: 3,
        changed: 1,
        body_bytes: served.1.len() as u64,
        digest: digest_hex(&served.0.digest),
    }
}

pub(super) fn index(categories: Vec<IndexEntry>) -> Index {
    Index {
        schema: 1,
        categories,
    }
}

/// The cache folder under a temp root, whose firesky holds `SGWGame/Cache.en-US`.
pub(super) fn cache_of(root: &Path) -> PathBuf {
    apply::cache_dir(&root.join("firesky"))
}

/// Writes a PAK the stock way, creating its folder.
pub(super) fn write_pak_at(dir: &Path, name: &str, version: u32, entries: &BTreeMap<u32, Vec<u8>>) {
    fs::create_dir_all(dir).unwrap();
    write_pak(File::create(dir.join(name)).unwrap(), version, entries).unwrap();
}

/// The temp files left in a folder, by name.
pub(super) fn temps(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(TEMP_SUFFIX))
        .collect()
}

/// A fetcher whose login server is the mock itself.
pub(super) fn fetcher_for(server: &MockServer) -> Fetcher {
    let url = server.uri();
    Fetcher::new(&url, std::slice::from_ref(&url)).expect("the mock is a login server")
}

pub(super) async fn serve_index(server: &MockServer, index: Index) {
    Mock::given(method("GET"))
        .and(path(INDEX_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(index))
        .mount(server)
        .await;
}

pub(super) async fn serve_body(
    server: &MockServer,
    category_id: u32,
    version: u32,
    bytes: Vec<u8>,
) {
    let body_path = format!("/api/cooked/v1/category/{category_id}");
    let version = version.to_string();
    Mock::given(method("GET"))
        .and(path(body_path.as_str()))
        .and(query_param("version", version.as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_raw(bytes, "application/octet-stream"))
        .mount(server)
        .await;
}

/// Runs the presync under `limit` with no progress callback and no cancel.
pub(super) async fn run_quiet(
    server: &MockServer,
    root: &Path,
    source: &Path,
    limit: Duration,
) -> Report {
    let fetcher = fetcher_for(server);
    let quiet = |_: Progress| {};
    run(
        &fetcher,
        &root.join("firesky"),
        source,
        Instant::now() + limit,
        &CancellationToken::new(),
        &quiet,
    )
    .await
}
