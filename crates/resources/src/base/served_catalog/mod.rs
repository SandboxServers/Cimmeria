//! The served cooked-data catalog for the launcher presync (LP-03).
//!
//! [`ServedCatalog::build`] turns the loaded [`ResourceCache`] into one `CPS1`
//! body per category the cache holds, plus the index that lists them. The
//! launcher reads both through the `/api/cooked/v1` routes (LP-10). The
//! catalog is built once at startup and published through [`publish`]; the
//! routes read it back with [`current`].
//!
//! A category the cache can't serve fails the whole build, so a hole in the
//! catalog is loud: Play is blocked on failure (D-LP18).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use cimmeria_cooked_pak::catalog::{
    decode_body, encode_body, CategoryBody, Index, IndexEntry, Limits,
};
use cimmeria_cooked_pak::digest::{category_digest, digest_hex};

use crate::base::resources::{CategoryData, ResourceCache, CATEGORY_PAKS};

/// One category as served: its index entry and its encoded `CPS1` body.
pub struct ServedCategory {
    /// The index row for this category.
    pub entry: IndexEntry,
    /// The `CPS1` body, exactly as the category route returns it.
    pub body: Arc<[u8]>,
}

/// Every category the server serves, with the index that lists them.
pub struct ServedCatalog {
    /// Categories keyed by id, ascending.
    categories: BTreeMap<u32, ServedCategory>,
    /// The index JSON (`{"schema":1,"categories":[...]}`), built once.
    index_json: Arc<[u8]>,
}

impl ServedCatalog {
    /// Build the catalog from `cache`, re-reading each category's raw PAK
    /// under `data_dir` to find the keys Cimmeria changed.
    ///
    /// A category the cache doesn't hold is left out. A category it holds but
    /// can't serve (its raw PAK is unreadable, or its body is too large) fails
    /// the whole build with an error naming the category and a reason code.
    pub fn build(cache: &ResourceCache, data_dir: &str) -> Result<Self, String> {
        Self::build_with_limits(cache, data_dir, &Limits::DEFAULT)
    }

    /// [`Self::build`] with explicit limits, so tests can force the size
    /// refusal with a tiny body cap.
    fn build_with_limits(
        cache: &ResourceCache,
        data_dir: &str,
        limits: &Limits,
    ) -> Result<Self, String> {
        let mut parts = Vec::new();
        for &(id, pak) in CATEGORY_PAKS {
            let Some(cat) = cache.category(id) else {
                continue;
            };
            let raw = ResourceCache::load_pak(&format!("{data_dir}/{pak}"))
                .map_err(|_| format!("category {id}: raw_pak_unreadable"))?;
            parts.push((pak.to_string(), category_body(id, cat, &raw)));
        }
        Self::assemble(parts, limits)
    }

    /// Encode each `(pak, body)` pair and index the result. Fails naming the
    /// category if one body is over `limits.max_total_bytes`, or if the
    /// launcher's decoder would refuse it under `limits`.
    fn assemble(parts: Vec<(String, CategoryBody)>, limits: &Limits) -> Result<Self, String> {
        let mut categories = BTreeMap::new();
        for (pak, body) in parts {
            let bytes = encode_body(&body);
            let size = bytes.len() as u64;
            if size > limits.max_total_bytes || decode_body(&bytes, limits).is_err() {
                return Err(format!(
                    "category {}: category_too_large ({size} bytes)",
                    body.category_id
                ));
            }
            let entry = IndexEntry {
                category_id: body.category_id,
                pak,
                version: body.version,
                entries: body.keys.len() as u32,
                changed: body.changed.len() as u32,
                body_bytes: size,
                digest: digest_hex(&body.digest),
            };
            categories.insert(
                body.category_id,
                ServedCategory {
                    entry,
                    body: bytes.into(),
                },
            );
        }
        let index = Index {
            schema: 1,
            categories: categories
                .values()
                .map(|served| served.entry.clone())
                .collect(),
        };
        let index_json: Arc<[u8]> = serde_json::to_vec(&index)
            .expect("the index holds only plain numbers and strings")
            .into();
        Ok(Self {
            categories,
            index_json,
        })
    }

    /// The index JSON served at `/api/cooked/v1/index`.
    pub fn index_json(&self) -> Arc<[u8]> {
        self.index_json.clone()
    }

    /// The served category `category_id`, if the catalog holds it.
    pub fn category(&self, category_id: u32) -> Option<&ServedCategory> {
        self.categories.get(&category_id)
    }

    /// A catalog from ready-made bodies, for other crates' route tests.
    #[cfg(any(test, feature = "test-support"))]
    pub fn from_bodies(bodies: Vec<(String, CategoryBody)>) -> Self {
        Self::assemble(bodies, &Limits::DEFAULT).expect("test bodies fit the default limits")
    }
}

/// The `CPS1` body for one category: every served key, and the served bytes
/// of each key whose bytes differ from the raw PAK entry or that the raw PAK
/// lacks. The digest covers the served map, in ascending key order.
fn category_body(id: u32, cat: &CategoryData, raw: &CategoryData) -> CategoryBody {
    let mut keys: Vec<u32> = cat.elements.keys().copied().collect();
    keys.sort_unstable();
    let mut changed = BTreeMap::new();
    for key in &keys {
        let bytes = &cat.elements[key];
        if raw.elements.get(key) != Some(bytes) {
            changed.insert(*key, bytes.clone());
        }
    }
    let digest = category_digest(
        id,
        cat.metadata,
        keys.iter().map(|key| (*key, cat.elements[key].as_slice())),
    );
    CategoryBody {
        category_id: id,
        version: cat.metadata,
        keys,
        changed,
        digest,
    }
}

/// The outcome of the one catalog build, as the routes read it.
#[derive(Clone)]
pub enum Published {
    /// The catalog is built and served.
    Ready(Arc<ServedCatalog>),
    /// The build failed; the code is closed (see [`failure_code`]).
    Failed(&'static str),
}

/// The process-wide build outcome. Set once, at startup.
static CATALOG: OnceLock<Published> = OnceLock::new();

/// Publish the build outcome. Returns `false` (and logs WARN) if one was
/// already published: the first outcome stands.
pub fn publish(state: Published) -> bool {
    publish_into(&CATALOG, state)
}

/// The published outcome, or `None` while the build is still running.
pub fn current() -> Option<Published> {
    current_in(&CATALOG)
}

/// Map a [`ServedCatalog::build`] error to its closed reason code. Any error
/// this module doesn't name maps to `build_failed`.
pub fn failure_code(error: &str) -> &'static str {
    if error.contains("raw_pak_unreadable") {
        "raw_pak_unreadable"
    } else if error.contains("category_too_large") {
        "category_too_large"
    } else {
        "build_failed"
    }
}

fn publish_into(slot: &OnceLock<Published>, state: Published) -> bool {
    match slot.set(state) {
        Ok(()) => true,
        Err(_) => {
            tracing::warn!("served cooked catalog already published; keeping the first outcome");
            false
        }
    }
}

fn current_in(slot: &OnceLock<Published>) -> Option<Published> {
    slot.get().cloned()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
