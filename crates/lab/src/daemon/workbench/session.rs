//! In-memory, daemon-lifetime opener and browser sessions.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const OPENER_LIFETIME: Duration = Duration::from_secs(60);
const SESSION_LIFETIME: Duration = Duration::from_secs(8 * 60 * 60);
const MAX_OPENERS: usize = 128;
const MAX_SESSIONS: usize = 32;

#[derive(Default)]
struct Inner {
    openers: HashMap<String, Instant>,
    sessions: HashMap<String, BrowserSession>,
}

struct BrowserSession {
    csrf: String,
    expires: Instant,
}

/// One daemon's volatile sessions. Dropping the router invalidates all of them.
#[derive(Default)]
pub struct SessionStore(Mutex<Inner>);

/// A consumed opener's new browser credentials. Only the cookie goes in the
/// redirect; the CSRF value is obtained later from a same-origin GET.
pub struct Credentials {
    pub cookie: String,
}

fn secret() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

impl SessionStore {
    /// Mint a 60-second opener for a bearer-authenticated local caller.
    pub fn open(&self) -> String {
        self.open_at(Instant::now())
    }

    fn open_at(&self, now: Instant) -> String {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner.openers.retain(|_, expires| *expires > now);
        if inner.openers.len() >= MAX_OPENERS {
            if let Some(oldest) = inner
                .openers
                .iter()
                .min_by_key(|(_, t)| *t)
                .map(|(k, _)| k.clone())
            {
                inner.openers.remove(&oldest);
            }
        }
        let nonce = secret();
        inner.openers.insert(nonce.clone(), now + OPENER_LIFETIME);
        nonce
    }

    /// Consume an opener exactly once, returning a fresh browser session.
    pub fn redeem(&self, nonce: &str) -> Option<Credentials> {
        self.redeem_at(nonce, Instant::now())
    }

    fn redeem_at(&self, nonce: &str, now: Instant) -> Option<Credentials> {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let expires = inner.openers.remove(nonce)?;
        if expires <= now {
            return None;
        }
        inner.sessions.retain(|_, session| session.expires > now);
        if inner.sessions.len() >= MAX_SESSIONS {
            if let Some(oldest) = inner
                .sessions
                .iter()
                .min_by_key(|(_, s)| s.expires)
                .map(|(k, _)| k.clone())
            {
                inner.sessions.remove(&oldest);
            }
        }
        let cookie = secret();
        inner.sessions.insert(
            cookie.clone(),
            BrowserSession {
                csrf: secret(),
                expires: now + SESSION_LIFETIME,
            },
        );
        Some(Credentials { cookie })
    }

    /// Return the CSRF secret only for a live browser session.
    pub fn csrf(&self, cookie: &str) -> Option<String> {
        let inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .sessions
            .get(cookie)
            .filter(|s| s.expires > Instant::now())
            .map(|s| s.csrf.clone())
    }

    #[cfg(test)]
    pub(crate) fn expire_openers(&self) {
        let mut inner = self.0.lock().unwrap_or_else(|e| e.into_inner());
        for expires in inner.openers.values_mut() {
            *expires = Instant::now() - Duration::from_secs(1);
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
