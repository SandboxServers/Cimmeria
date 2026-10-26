//! Access tokens of `/token` and the link routes (contract C6), with the
//! revocation fences of contract C16.3.
//!
//! A route captures [`AccessTokens::fence`] before its first await. Every
//! revoke bumps one sequence number and leaves a tombstone. A mint with an
//! older fence is refused when a later revoke covers its family or account, so
//! a token cannot be minted for a family that was revoked while the request was
//! in flight. Tombstones live as long as an access token does, which is longer
//! than any request.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

use crate::stores::{hash_of, live, locked, random_b64url, ACCESS_LIFE, STORE_CAP};

/// The record behind one access token. The store keys it by SHA-256 of the
/// token, so the token text is never kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessGrant {
    /// The account the token speaks for.
    pub account_id: u32,
    /// The refresh family the token belongs to.
    pub family_id: String,
    /// True when the token came from an `authorization_code` grant or a link.
    pub via_code: bool,
    /// When the token was minted.
    pub issued_at: Instant,
    /// The fence sequence when the token was minted (C16.3).
    pub minted_seq: u64,
}

/// Everything behind the one `Mutex`: the grants and the tombstones of the
/// revokes.
#[derive(Default)]
struct Inner {
    /// Bumped by every `revoke_*`.
    seq: u64,
    by_hash: HashMap<[u8; 32], AccessGrant>,
    /// Family to (the sequence of its revoke, when the revoke ran).
    family_revoked: HashMap<String, (u64, Instant)>,
    account_revoked: HashMap<u32, AccountTomb>,
}

/// What the revokes of one account left behind. `seq_all` refuses every family
/// of the account; `seq_except` refuses every family but `keep`.
struct AccountTomb {
    seq_all: u64,
    seq_except: u64,
    keep: Option<String>,
    at: Instant,
}

/// Access tokens, keyed by the SHA-256 of their text (C6), with the fences of
/// C16.3.
#[derive(Default)]
pub struct AccessTokens {
    inner: Mutex<Inner>,
}

impl AccessTokens {
    /// The current fence: the sequence of the latest revoke. A route captures
    /// it before its first await and passes it to [`Self::mint_fenced`].
    pub fn fence(&self) -> u64 {
        locked(&self.inner).seq
    }

    /// Mints a token for `account_id` in `family_id` and returns its text. The
    /// token is 32 random bytes in base64url; only its hash is kept. This mint
    /// ignores the revokes: tests and tools only, since no route calls it.
    pub fn mint(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant) -> String {
        insert(
            &mut locked(&self.inner),
            account_id,
            family_id,
            via_code,
            now,
        )
    }

    /// Mints like [`Self::mint`], unless a revoke after `fence` covers the
    /// family or the account. Then it answers `None` and mints nothing.
    pub fn mint_fenced(
        &self,
        account_id: u32,
        family_id: &str,
        via_code: bool,
        now: Instant,
        fence: u64,
    ) -> Option<String> {
        let mut inner = locked(&self.inner);
        if revoked_since(&inner, account_id, family_id, fence) {
            return None;
        }
        Some(insert(&mut inner, account_id, family_id, via_code, now))
    }

    /// The grant behind `token`, while it is inside [`ACCESS_LIFE`].
    pub fn check(&self, token: &str, now: Instant) -> Option<AccessGrant> {
        let inner = locked(&self.inner);
        let grant = inner.by_hash.get(&hash_of(token))?;
        live(grant.issued_at, now, ACCESS_LIFE).then(|| grant.clone())
    }

    /// Whether `grant` is still good: it is still held, and no revoke after its
    /// mint covers its family or its account.
    pub fn grant_live(&self, grant: &AccessGrant) -> bool {
        let inner = locked(&self.inner);
        !revoked_since(&inner, grant.account_id, &grant.family_id, grant.minted_seq)
            && inner.by_hash.values().any(|held| held == grant)
    }

    /// The number of grants held, for tests.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        locked(&self.inner).by_hash.len()
    }

    /// Ends every token of `family_id` and refuses the family's later fenced
    /// mints.
    pub fn revoke_family(&self, family_id: &str) {
        let now = Instant::now();
        let mut inner = locked(&self.inner);
        inner.seq += 1;
        let seq = inner.seq;
        inner
            .by_hash
            .retain(|_, grant| grant.family_id != family_id);
        if !inner.family_revoked.contains_key(family_id) {
            make_room(&mut inner);
        }
        inner
            .family_revoked
            .insert(family_id.to_owned(), (seq, now));
    }

    /// Ends every token of `account_id` and refuses every later fenced mint of
    /// the account.
    pub fn revoke_account(&self, account_id: u32) {
        let now = Instant::now();
        let mut inner = locked(&self.inner);
        inner.seq += 1;
        let seq = inner.seq;
        inner
            .by_hash
            .retain(|_, grant| grant.account_id != account_id);
        let tomb = tomb_of(&mut inner, account_id, now);
        tomb.seq_all = seq;
        tomb.at = now;
    }

    /// Ends every token of `account_id` except those of `keep_family`, the
    /// caller's own family. A family keeps all its access tokens, so an older
    /// token of the caller's sign-in still works. A later call with another
    /// `keep_family` first refuses the older keep's families up to the older
    /// call, so the older keep loses its exemption.
    pub fn revoke_account_except(&self, account_id: u32, keep_family: &str) {
        let now = Instant::now();
        let mut inner = locked(&self.inner);
        inner.seq += 1;
        let seq = inner.seq;
        inner
            .by_hash
            .retain(|_, grant| grant.account_id != account_id || grant.family_id == keep_family);
        let tomb = tomb_of(&mut inner, account_id, now);
        if tomb.keep.as_deref() != Some(keep_family) {
            tomb.seq_all = tomb.seq_all.max(tomb.seq_except);
        }
        tomb.seq_except = seq;
        tomb.keep = Some(keep_family.to_owned());
        tomb.at = now;
    }

    /// Drops the tokens and the tombstones past their life.
    pub fn reap(&self, now: Instant) {
        let mut inner = locked(&self.inner);
        inner
            .by_hash
            .retain(|_, grant| live(grant.issued_at, now, ACCESS_LIFE));
        inner
            .family_revoked
            .retain(|_, (_, at)| live(*at, now, ACCESS_LIFE));
        inner
            .account_revoked
            .retain(|_, tomb| live(tomb.at, now, ACCESS_LIFE));
    }
}

/// Stores a grant for a fresh token and returns the token. Expired grants go
/// first, then room is made, and the grant records the fence it was minted at.
fn insert(
    inner: &mut Inner,
    account_id: u32,
    family_id: &str,
    via_code: bool,
    now: Instant,
) -> String {
    let token = random_b64url(32);
    inner
        .by_hash
        .retain(|_, grant| live(grant.issued_at, now, ACCESS_LIFE));
    make_room(inner);
    let minted_seq = inner.seq;
    inner.by_hash.insert(
        hash_of(&token),
        AccessGrant {
            account_id,
            family_id: family_id.to_owned(),
            via_code,
            issued_at: now,
            minted_seq,
        },
    );
    token
}

/// The tombstone of `account_id`, made empty when the account has none yet.
fn tomb_of(inner: &mut Inner, account_id: u32, now: Instant) -> &mut AccountTomb {
    if !inner.account_revoked.contains_key(&account_id) {
        make_room(inner);
    }
    inner
        .account_revoked
        .entry(account_id)
        .or_insert(AccountTomb {
            seq_all: 0,
            seq_except: 0,
            keep: None,
            at: now,
        })
}

/// Whether a revoke after `fence` covers `family_id` of `account_id`: the
/// family itself, every family of the account, or every family but the one the
/// account's last `revoke_account_except` kept.
fn revoked_since(inner: &Inner, account_id: u32, family_id: &str, fence: u64) -> bool {
    let family = inner
        .family_revoked
        .get(family_id)
        .is_some_and(|(seq, _)| *seq > fence);
    let account = inner.account_revoked.get(&account_id).is_some_and(|tomb| {
        tomb.seq_all > fence || (tomb.seq_except > fence && tomb.keep.as_deref() != Some(family_id))
    });
    family || account
}

/// A candidate for eviction: a grant or a tombstone.
enum Victim {
    Grant([u8; 32]),
    Family(String),
    Account(u32),
}

/// Past [`STORE_CAP`] entries, grants and tombstones counted together, drops
/// the one issued or revoked longest ago. Runs only when the store is full.
fn make_room(inner: &mut Inner) {
    while inner.by_hash.len() + inner.family_revoked.len() + inner.account_revoked.len()
        >= STORE_CAP
    {
        let oldest = inner
            .by_hash
            .iter()
            .map(|(hash, grant)| (grant.issued_at, Victim::Grant(*hash)))
            .chain(
                inner
                    .family_revoked
                    .iter()
                    .map(|(family, (_, at))| (*at, Victim::Family(family.clone()))),
            )
            .chain(
                inner
                    .account_revoked
                    .iter()
                    .map(|(account, tomb)| (tomb.at, Victim::Account(*account))),
            )
            .min_by_key(|(at, _)| *at);
        match oldest {
            Some((_, Victim::Grant(hash))) => {
                inner.by_hash.remove(&hash);
            }
            Some((_, Victim::Family(family))) => {
                inner.family_revoked.remove(&family);
            }
            Some((_, Victim::Account(account))) => {
                inner.account_revoked.remove(&account);
            }
            None => break,
        }
    }
}

#[cfg(test)]
#[path = "access_tokens_tests.rs"]
mod tests;
