//! The presync index and the `CPS1` category body.
//!
//! The server publishes one JSON index that lists every category, and one
//! binary body per category. The body holds the category's key list, the
//! records that differ from the stock PAK, and the category digest. A body
//! is decoded only under [`Limits`], and every count and length is checked
//! before anything is allocated, so a hostile body cannot force a large
//! allocation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The magic that opens every category body.
const BODY_MAGIC: &[u8; 4] = b"CPS1";

/// The index document: the schema number and one entry per category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Index {
    /// The index format. Only schema 1 is understood.
    pub schema: u32,
    /// The served categories, ascending by id.
    pub categories: Vec<IndexEntry>,
}

/// One category as the index describes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// The category id, as the client's cache names it.
    pub category_id: u32,
    /// The PAK file name in the cache, such as `CookedDataItems.pak`.
    pub pak: String,
    /// The served category version.
    pub version: u32,
    /// How many records the category holds.
    pub entries: u32,
    /// How many of those records differ from the stock PAK.
    pub changed: u32,
    /// The size of the category's body in bytes.
    pub body_bytes: u64,
    /// The category digest as 64 lower-case hex characters.
    pub digest: String,
}

/// A decoded category body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryBody {
    /// The category id.
    pub category_id: u32,
    /// The served category version.
    pub version: u32,
    /// Every record key of the category, strictly ascending.
    pub keys: Vec<u32>,
    /// The records that differ from the stock PAK, by key. Each key is also
    /// in `keys`.
    pub changed: BTreeMap<u32, Vec<u8>>,
    /// The category digest, as [`crate::digest::category_digest`] gives it.
    pub digest: [u8; 32],
}

/// The bounds a decoded body must stay within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most keys a category may hold.
    pub max_keys: u32,
    /// The most bytes one changed record may hold.
    pub max_entry_bytes: u32,
    /// The most changed bytes one body may hold in total.
    pub max_total_bytes: u64,
}

impl Limits {
    /// The bounds the presync uses: a million keys, 4 MiB per record and 32
    /// MiB of changed bytes per category.
    pub const DEFAULT: Limits = Limits {
        max_keys: 1_000_000,
        max_entry_bytes: 4 << 20,
        max_total_bytes: 32 << 20,
    };
}

/// Why a body did not decode.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum BodyError {
    /// The body does not open with `CPS1`.
    #[error("body does not start with CPS1")]
    BadMagic,
    /// The body ends before its counts and lengths say it should.
    #[error("body is truncated")]
    Truncated,
    /// A key list or a changed-key list is not strictly ascending.
    #[error("keys are not strictly ascending")]
    NotAscending,
    /// A changed record names a key that is not in the key list.
    #[error("a changed key is not in the key list")]
    ChangedKeyNotListed,
    /// A count or length is over the [`Limits`].
    #[error("body exceeds the limits")]
    TooLarge,
    /// Bytes follow the digest.
    #[error("bytes follow the digest")]
    Trailing,
}

/// Encodes a category body in the `CPS1` layout.
///
/// The body is written as given. [`decode_body`] is what checks it.
pub fn encode_body(body: &CategoryBody) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(BODY_MAGIC);
    push_u32(&mut out, body.category_id);
    push_u32(&mut out, body.version);
    push_u32(&mut out, body.keys.len() as u32);
    for key in &body.keys {
        push_u32(&mut out, *key);
    }
    push_u32(&mut out, body.changed.len() as u32);
    for (key, bytes) in &body.changed {
        push_u32(&mut out, *key);
        push_u32(&mut out, bytes.len() as u32);
        out.extend_from_slice(bytes);
    }
    out.extend_from_slice(&body.digest);
    out
}

/// Decodes a `CPS1` body, checking every count and length against `limits`
/// before it allocates.
pub fn decode_body(bytes: &[u8], limits: &Limits) -> Result<CategoryBody, BodyError> {
    let mut reader = Reader { bytes, pos: 0 };
    if reader.take(BODY_MAGIC.len())? != BODY_MAGIC {
        return Err(BodyError::BadMagic);
    }
    let category_id = reader.u32()?;
    let version = reader.u32()?;

    let key_count = reader.u32()?;
    if key_count > limits.max_keys {
        return Err(BodyError::TooLarge);
    }
    if reader.remaining() / 4 < key_count as usize {
        return Err(BodyError::Truncated);
    }
    let mut keys = Vec::with_capacity(key_count as usize);
    for _ in 0..key_count {
        let key = reader.u32()?;
        if keys.last().is_some_and(|&last| key <= last) {
            return Err(BodyError::NotAscending);
        }
        keys.push(key);
    }

    let changed_count = reader.u32()?;
    if changed_count as usize > keys.len() {
        return Err(BodyError::TooLarge);
    }
    let mut changed = BTreeMap::new();
    let mut total: u64 = 0;
    let mut previous: Option<u32> = None;
    for _ in 0..changed_count {
        let key = reader.u32()?;
        if previous.is_some_and(|last| key <= last) {
            return Err(BodyError::NotAscending);
        }
        previous = Some(key);
        if keys.binary_search(&key).is_err() {
            return Err(BodyError::ChangedKeyNotListed);
        }
        let len = reader.u32()?;
        if len > limits.max_entry_bytes {
            return Err(BodyError::TooLarge);
        }
        total += u64::from(len);
        if total > limits.max_total_bytes {
            return Err(BodyError::TooLarge);
        }
        changed.insert(key, reader.take(len as usize)?.to_vec());
    }

    let digest: [u8; 32] = reader
        .take(32)?
        .try_into()
        .map_err(|_| BodyError::Truncated)?;
    if reader.remaining() != 0 {
        return Err(BodyError::Trailing);
    }
    Ok(CategoryBody {
        category_id,
        version,
        keys,
        changed,
        digest,
    })
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// A cursor over a body. Every read checks the bytes it needs first.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], BodyError> {
        if len > self.remaining() {
            return Err(BodyError::Truncated);
        }
        let slice = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, BodyError> {
        let raw: [u8; 4] = self.take(4)?.try_into().map_err(|_| BodyError::Truncated)?;
        Ok(u32::from_le_bytes(raw))
    }
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
