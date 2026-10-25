//! In-memory sign-in state (contract C6, C12): the pending authorizations
//! between `/authorize` and the provider's callback, the handoff codes the
//! callback issues, the access tokens of `/token`, and the link grants of a
//! `needs_link` answer.
//!
//! Nothing here survives a restart; a restart ends every sign-in in flight and
//! the launcher starts it again. Each store keeps its map behind one `Mutex`
//! and takes the clock as an argument, so the tests move it without sleeping.
//! Secrets are never printed: the records that hold a PKCE verifier or a
//! verified identity have no `Debug` impl.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngExt;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::provider::VerifiedIdentity;

/// How long a pending authorization waits for the provider's callback.
pub const PENDING_LIFE: Duration = Duration::from_secs(10 * 60);

/// The most pending authorizations held; the oldest is dropped past this.
pub const PENDING_CAP: usize = 10_000;

/// Most entries the handoff, access-token and link-grant stores hold. Past it
/// the oldest entry goes first.
pub const STORE_CAP: usize = 50_000;

/// How long a handoff code can be redeemed at `/token`.
pub const HANDOFF_LIFE: Duration = Duration::from_secs(60);

/// How long an access token is accepted.
pub const ACCESS_LIFE: Duration = Duration::from_secs(15 * 60);

/// How long a link grant can be redeemed at `/link/*`.
pub const LINK_GRANT_LIFE: Duration = Duration::from_secs(10 * 60);

/// How long a `/link/start` intent can be spent at `/authorize` (C6).
pub const INTENT_LIFE: Duration = Duration::from_secs(10 * 60);

/// A `/link/start` intent: the signed-in account the new identity attaches to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkIntent {
    /// The account the callback links the identity to.
    pub account_id: u32,
}

/// A `/authorize` request waiting for the provider's callback. It holds the
/// server's PKCE verifier, so it has no `Debug` impl.
pub struct PendingAuth {
    /// The provider the request was sent to.
    pub provider: String,
    /// The launcher's loopback `redirect_uri`, checked again at `/token`.
    pub redirect_uri: String,
    /// The launcher's own `state`, echoed back on the handoff redirect.
    pub launcher_state: String,
    /// The launcher's PKCE challenge, bound to the handoff code.
    pub launcher_challenge: String,
    /// The server's PKCE verifier towards the provider.
    pub pkce_verifier: String,
    /// The nonce the provider's ID token must carry.
    pub nonce: String,
    /// Set when the request came from `/link/start`.
    pub intent: Option<LinkIntent>,
}

/// What the callback hands to `/token`: the verified identity and the link
/// intent, if any. It holds the identity, so it has no `Debug` impl.
pub struct HandoffOutcome {
    /// The identity the provider verified.
    pub identity: VerifiedIdentity,
    /// The link intent carried over from the pending record.
    pub intent: Option<LinkIntent>,
}

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
}

/// One pending authorization, with the bookkeeping the store needs.
struct PendingEntry {
    auth: PendingAuth,
    issued: Instant,
    seq: u64,
}

/// The pending map, the order of its entries, and the next sequence number.
#[derive(Default)]
struct PendingInner {
    by_state: HashMap<String, PendingEntry>,
    by_age: BTreeMap<u64, String>,
    next_seq: u64,
}

/// Pending authorizations keyed by the server's `state` (C6).
#[derive(Default)]
pub struct PendingStore {
    inner: Mutex<PendingInner>,
}

impl PendingStore {
    /// Stores `rec` under a fresh server `state` and returns the state. Past
    /// [`PENDING_CAP`] entries the oldest one is dropped first.
    pub fn insert(&self, rec: PendingAuth, now: Instant) -> String {
        let state = random_b64url(32);
        let mut inner = locked(&self.inner);
        while inner.by_state.len() >= PENDING_CAP {
            let Some((_, oldest)) = inner.by_age.pop_first() else {
                break;
            };
            inner.by_state.remove(&oldest);
        }
        let seq = inner.next_seq;
        inner.next_seq += 1;
        inner.by_age.insert(seq, state.clone());
        inner.by_state.insert(
            state.clone(),
            PendingEntry {
                auth: rec,
                issued: now,
                seq,
            },
        );
        state
    }

    /// Removes and returns the record for `server_state`. The record is single
    /// use: it is gone after this call even when it has expired.
    pub fn take(&self, server_state: &str, now: Instant) -> Option<PendingAuth> {
        let mut inner = locked(&self.inner);
        let entry = inner.by_state.remove(server_state)?;
        inner.by_age.remove(&entry.seq);
        live(entry.issued, now, PENDING_LIFE).then_some(entry.auth)
    }

    /// Drops the records past their life.
    pub fn reap(&self, now: Instant) {
        let mut inner = locked(&self.inner);
        let PendingInner {
            by_state, by_age, ..
        } = &mut *inner;
        by_state.retain(|_, entry| {
            let keep = live(entry.issued, now, PENDING_LIFE);
            if !keep {
                by_age.remove(&entry.seq);
            }
            keep
        });
    }
}

/// A handoff code's record: the outcome, the launcher's challenge and
/// `redirect_uri`, and when the code was issued.
struct HandoffEntry {
    outcome: HandoffOutcome,
    challenge: String,
    redirect_uri: String,
    issued: Instant,
}

/// Handoff codes the callback issues and `/token` redeems (C6).
#[derive(Default)]
pub struct HandoffStore {
    codes: Mutex<HashMap<String, HandoffEntry>>,
}

impl HandoffStore {
    /// Stores `outcome` under a fresh code bound to the launcher's `challenge`
    /// and `redirect_uri`, and returns the code.
    pub fn issue(
        &self,
        outcome: HandoffOutcome,
        challenge: &str,
        redirect_uri: &str,
        now: Instant,
    ) -> String {
        let code = random_b64url(32);
        let mut codes = locked(&self.codes);
        codes.retain(|_, entry| live(entry.issued, now, HANDOFF_LIFE));
        make_room(&mut codes, STORE_CAP, |entry| entry.issued);
        codes.insert(
            code.clone(),
            HandoffEntry {
                outcome,
                challenge: challenge.to_owned(),
                redirect_uri: redirect_uri.to_owned(),
                issued: now,
            },
        );
        code
    }

    /// Redeems `code`. The code is spent by this call, so a wrong verifier or
    /// redirect uses it up too. The verifier must hash to the bound challenge
    /// (compared in constant time), the `redirect_uri` must match, and the code
    /// must be inside [`HANDOFF_LIFE`].
    pub fn redeem(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
        now: Instant,
    ) -> Option<HandoffOutcome> {
        let entry = locked(&self.codes).remove(code)?;
        let computed = pkce_challenge(verifier);
        let pkce_ok: bool = computed.as_bytes().ct_eq(entry.challenge.as_bytes()).into();
        let uri_ok = entry.redirect_uri == redirect_uri;
        (pkce_ok && uri_ok && live(entry.issued, now, HANDOFF_LIFE)).then_some(entry.outcome)
    }

    /// Drops the codes past their life.
    pub fn reap(&self, now: Instant) {
        locked(&self.codes).retain(|_, entry| live(entry.issued, now, HANDOFF_LIFE));
    }
}

/// Access tokens, keyed by the SHA-256 of their text (C6).
#[derive(Default)]
pub struct AccessTokens {
    by_hash: Mutex<HashMap<[u8; 32], AccessGrant>>,
}

impl AccessTokens {
    /// Mints a token for `account_id` in `family_id` and returns its text. The
    /// token is 32 random bytes in base64url; only its hash is kept.
    pub fn mint(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant) -> String {
        let token = random_b64url(32);
        let mut by_hash = locked(&self.by_hash);
        by_hash.retain(|_, grant| live(grant.issued_at, now, ACCESS_LIFE));
        make_room(&mut by_hash, STORE_CAP, |grant| grant.issued_at);
        by_hash.insert(
            hash_of(&token),
            AccessGrant {
                account_id,
                family_id: family_id.to_owned(),
                via_code,
                issued_at: now,
            },
        );
        token
    }

    /// The grant behind `token`, while it is inside [`ACCESS_LIFE`].
    pub fn check(&self, token: &str, now: Instant) -> Option<AccessGrant> {
        let by_hash = locked(&self.by_hash);
        let grant = by_hash.get(&hash_of(token))?;
        live(grant.issued_at, now, ACCESS_LIFE).then(|| grant.clone())
    }

    /// Ends every token of `family_id`.
    pub fn revoke_family(&self, family_id: &str) {
        locked(&self.by_hash).retain(|_, grant| grant.family_id != family_id);
    }

    /// Ends every token of `account_id`.
    pub fn revoke_account(&self, account_id: u32) {
        locked(&self.by_hash).retain(|_, grant| grant.account_id != account_id);
    }

    /// Ends every token of `account_id` except those of `keep_family`, the
    /// caller's own family. A family keeps all its access tokens, so an older
    /// token of the caller's sign-in still works.
    pub fn revoke_account_except(&self, account_id: u32, keep_family: &str) {
        locked(&self.by_hash)
            .retain(|_, grant| grant.account_id != account_id || grant.family_id == keep_family);
    }

    /// Drops the tokens past their life.
    pub fn reap(&self, now: Instant) {
        locked(&self.by_hash).retain(|_, grant| live(grant.issued_at, now, ACCESS_LIFE));
    }
}

/// Link grants: the verified identity a `needs_link` answer holds until the
/// launcher chooses `/link/new` or `/link/existing` (C6).
#[derive(Default)]
pub struct LinkGrants {
    grants: Mutex<HashMap<String, (VerifiedIdentity, Instant)>>,
}

impl LinkGrants {
    /// Stores `identity` under a fresh grant and returns the grant.
    pub fn issue(&self, identity: VerifiedIdentity, now: Instant) -> String {
        let grant = random_b64url(32);
        let mut grants = locked(&self.grants);
        grants.retain(|_, (_, issued)| live(*issued, now, LINK_GRANT_LIFE));
        make_room(&mut grants, STORE_CAP, |(_, issued)| *issued);
        grants.insert(grant.clone(), (identity, now));
        grant
    }

    /// Removes and returns the identity behind `grant`, while it is inside
    /// [`LINK_GRANT_LIFE`]. The grant is single use.
    pub fn take(&self, grant: &str, now: Instant) -> Option<VerifiedIdentity> {
        let (identity, issued) = locked(&self.grants).remove(grant)?;
        live(issued, now, LINK_GRANT_LIFE).then_some(identity)
    }

    /// Returns a copy of the identity behind `grant` without spending it, while
    /// it is inside [`LINK_GRANT_LIFE`]. A link answers its refusals with the
    /// grant still held, so a wrong password does not burn it.
    pub fn peek(&self, grant: &str, now: Instant) -> Option<VerifiedIdentity> {
        let grants = locked(&self.grants);
        let (identity, issued) = grants.get(grant)?;
        // `VerifiedIdentity` is not `Clone` (its `Debug` hides the subject and
        // email), so the copy is made field by field.
        live(*issued, now, LINK_GRANT_LIFE).then(|| VerifiedIdentity {
            provider: identity.provider.clone(),
            issuer: identity.issuer.clone(),
            subject: identity.subject.clone(),
            display_name: identity.display_name.clone(),
            email: identity.email.clone(),
            email_verified: identity.email_verified,
            provider_account_created_ms: identity.provider_account_created_ms,
        })
    }

    /// Drops the grants past their life.
    pub fn reap(&self, now: Instant) {
        locked(&self.grants).retain(|_, (_, issued)| live(*issued, now, LINK_GRANT_LIFE));
    }
}

/// Link intents: the token `link/start` returns, which `/authorize` spends to
/// attach the callback's identity to an account (C6). Single use.
#[derive(Default)]
pub struct IntentStore {
    by_hash: Mutex<HashMap<[u8; 32], (LinkIntent, Instant)>>,
}

impl IntentStore {
    /// Stores `intent` under a fresh token and returns the token. Only its hash
    /// is kept.
    pub fn issue(&self, intent: LinkIntent, now: Instant) -> String {
        let token = random_b64url(32);
        let mut by_hash = locked(&self.by_hash);
        by_hash.retain(|_, (_, issued)| live(*issued, now, INTENT_LIFE));
        make_room(&mut by_hash, STORE_CAP, |(_, issued)| *issued);
        by_hash.insert(hash_of(&token), (intent, now));
        token
    }

    /// Removes and returns the intent behind `token`, while it is inside
    /// [`INTENT_LIFE`]. The token is single use.
    pub fn take(&self, token: &str, now: Instant) -> Option<LinkIntent> {
        let (intent, issued) = locked(&self.by_hash).remove(&hash_of(token))?;
        live(issued, now, INTENT_LIFE).then_some(intent)
    }

    /// Drops the intents past their life.
    pub fn reap(&self, now: Instant) {
        locked(&self.by_hash).retain(|_, (_, issued)| live(*issued, now, INTENT_LIFE));
    }
}

/// The sign-in state the routes share: every store, built once.
#[derive(Default)]
pub struct Stores {
    /// Pending authorizations between `/authorize` and the callback.
    pub pending: PendingStore,
    /// Handoff codes between the callback and `/token`.
    pub handoff: HandoffStore,
    /// Access tokens of `/token` and the link routes.
    pub access: AccessTokens,
    /// Link grants of a `needs_link` answer.
    pub grants: LinkGrants,
    /// Link intents of `/link/start`, spent at `/authorize`.
    pub intents: IntentStore,
}

impl Stores {
    /// Drops every expired record in every store.
    pub fn reap(&self, now: Instant) {
        self.pending.reap(now);
        self.handoff.reap(now);
        self.access.reap(now);
        self.grants.reap(now);
        self.intents.reap(now);
    }
}

/// The PKCE S256 challenge of `verifier`: `BASE64URL(SHA256(verifier))`,
/// without padding (RFC 7636).
pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(hash_of(verifier))
}

/// `bytes` random bytes in base64url without padding.
pub fn random_b64url(bytes: usize) -> String {
    let mut rng = rand::rng();
    let buf: Vec<u8> = (0..bytes).map(|_| rng.random()).collect();
    URL_SAFE_NO_PAD.encode(buf)
}

/// The SHA-256 of a text: the only form a token or a verifier is kept in.
fn hash_of(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

/// Whether an entry issued at `issued` is still inside `life` at `now`.
fn live(issued: Instant, now: Instant, life: Duration) -> bool {
    now.saturating_duration_since(issued) < life
}

/// Past `cap` entries, drops the one issued longest ago, so a flood of issues
/// cannot grow a store without bound. Runs only when the store is full.
fn make_room<K, V>(map: &mut HashMap<K, V>, cap: usize, issued: impl Fn(&V) -> Instant)
where
    K: Clone + Eq + std::hash::Hash,
{
    while map.len() >= cap {
        let Some(oldest) = map
            .iter()
            .min_by_key(|(_, entry)| issued(entry))
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        map.remove(&oldest);
    }
}

/// Locks a store's map, ignoring poisoning: a panicked holder leaves the map
/// in a state every method can still read.
fn locked<T>(map: &Mutex<T>) -> MutexGuard<'_, T> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
#[path = "stores_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "stores_intent_tests.rs"]
mod intent_tests;
