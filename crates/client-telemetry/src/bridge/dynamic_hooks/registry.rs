//! DLL-side registry of installed dynamic hooks (issue #686 scope 4).
//!
//! Tracks what is hooked *in this client*, for `hook_list` / `hook_remove`
//! and for per-hit sampling + hit-limit decisions. Pure and unit-tested;
//! [`super::native`] owns the actual byte-patching and calls
//! [`HookRegistry::record_hit`] from the detour to decide whether to
//! capture this hit and whether the limit is now reached.
//!
//! The **persistent** flag is stored here so `hook_list` reports it, but
//! the authority for *replaying* persistent hooks after a crash is the
//! supervisor's `PersistentHooks` (`crates/lab/src/supervisor/recovery.rs`)
//! — it is the bridge's single client and survives the crash. This
//! registry dies with the client.

use serde_json::{json, Value};

use super::spec::CaptureSpec;

/// One installed hook.
#[derive(Debug, Clone)]
pub struct HookEntry {
    pub id: u32,
    pub addr: usize,
    /// Calling convention the caller stated (native path is cdecl-only in
    /// phase 3; stored for `hook_list` regardless).
    pub conv: String,
    pub spec: CaptureSpec,
    /// Re-applied by the supervisor after a crash when true.
    pub persistent: bool,
    /// Hits observed so far (every fire, before sampling).
    pub hits: u64,
    /// Cleared once the hit limit is reached; the detour then stops
    /// capturing (the byte patch may still be present until `hook_remove`).
    pub active: bool,
}

impl HookEntry {
    /// A serializable summary for `hook_list`.
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "addr": format!("{:#x}", self.addr),
            "conv": self.conv,
            "persistent": self.persistent,
            "hits": self.hits,
            "active": self.active,
            "capture": self.spec.to_json(),
        })
    }
}

/// What the detour should do with a hit, decided by [`HookRegistry::record_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitDecision {
    /// Emit a `hook.hit` event for this fire (sampling said yes).
    pub capture: bool,
    /// This fire reached the hit limit; the caller should disable the
    /// hook's capture (and may schedule an unhook).
    pub reached_limit: bool,
}

/// Registry of live hooks. Keyed by a monotonic id.
#[derive(Debug, Default)]
pub struct HookRegistry {
    next_id: u32,
    entries: Vec<HookEntry>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            entries: Vec::new(),
        }
    }

    /// Register a hook, returning its id. Does **not** patch bytes — the
    /// native layer does that and calls this to track it.
    pub fn install(
        &mut self,
        addr: usize,
        conv: impl Into<String>,
        spec: CaptureSpec,
        persistent: bool,
    ) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(HookEntry {
            id,
            addr,
            conv: conv.into(),
            spec,
            persistent,
            hits: 0,
            active: true,
        });
        id
    }

    /// Remove a hook by id. Returns the removed entry (so the caller can
    /// unhook the bytes) or `None` if the id is unknown.
    pub fn remove(&mut self, id: u32) -> Option<HookEntry> {
        let pos = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(pos))
    }

    /// Whether an address is already hooked (native install refuses a
    /// duplicate).
    pub fn contains_addr(&self, addr: usize) -> bool {
        self.entries.iter().any(|e| e.addr == addr)
    }

    /// Look up a live hook by id.
    pub fn get(&self, id: u32) -> Option<&HookEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// All hooks as `hook_list` JSON.
    pub fn list_json(&self) -> Vec<Value> {
        self.entries.iter().map(HookEntry::to_json).collect()
    }

    /// Record one fire of hook `id` and decide what the detour should do.
    ///
    /// - Increments the hit counter.
    /// - `capture` is true when this fire passes the sample gate
    ///   (`(hits-1) % sample_rate == 0`, so the first hit always
    ///   captures) and the hook is still active.
    /// - `reached_limit` is true on the fire that meets `hit_limit`; the
    ///   entry is marked inactive so later fires don't capture.
    ///
    /// An unknown id yields `capture: false` (the hook was removed
    /// between fire and record — a benign race).
    pub fn record_hit(&mut self, id: u32) -> HitDecision {
        let Some(e) = self.entries.iter_mut().find(|e| e.id == id) else {
            return HitDecision {
                capture: false,
                reached_limit: false,
            };
        };
        if !e.active {
            return HitDecision {
                capture: false,
                reached_limit: false,
            };
        }
        e.hits += 1;
        let sample_ok = (e.hits - 1) % e.spec.sample_rate as u64 == 0;
        let reached_limit = e.spec.hit_limit.is_some_and(|lim| e.hits >= lim);
        if reached_limit {
            e.active = false;
        }
        HitDecision {
            capture: sample_ok,
            reached_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(sample_rate: u32, hit_limit: Option<u64>) -> CaptureSpec {
        CaptureSpec {
            sample_rate,
            hit_limit,
            ..CaptureSpec::default()
        }
    }

    #[test]
    fn install_list_remove_roundtrip() {
        let mut r = HookRegistry::new();
        let a = r.install(0x401000, "cdecl", spec(1, None), true);
        let b = r.install(0x402000, "cdecl", spec(1, None), false);
        assert_ne!(a, b);
        assert_eq!(r.list_json().len(), 2);
        assert!(r.contains_addr(0x401000));

        let removed = r.remove(a).expect("removed a");
        assert_eq!(removed.addr, 0x401000);
        assert!(removed.persistent);
        assert!(!r.contains_addr(0x401000));
        assert_eq!(r.list_json().len(), 1);

        // Removing an unknown id is a no-op.
        assert!(r.remove(9999).is_none());
    }

    #[test]
    fn list_json_reports_persistent_flag() {
        let mut r = HookRegistry::new();
        let id = r.install(0x401000, "cdecl", spec(1, None), true);
        let entry = &r.list_json()[0];
        assert_eq!(entry["id"], id);
        assert_eq!(entry["persistent"], true);
        assert_eq!(entry["addr"], "0x401000");
    }

    #[test]
    fn first_hit_always_captures_then_samples() {
        let mut r = HookRegistry::new();
        let id = r.install(0x401000, "cdecl", spec(3, None), false);
        // hits 1,4,7 capture (every 3rd starting at the first).
        let caps: Vec<bool> = (0..7).map(|_| r.record_hit(id).capture).collect();
        assert_eq!(caps, vec![true, false, false, true, false, false, true]);
        assert_eq!(r.get(id).unwrap().hits, 7);
    }

    #[test]
    fn hit_limit_disables_capture_after_reached() {
        let mut r = HookRegistry::new();
        let id = r.install(0x401000, "cdecl", spec(1, Some(2)), false);
        let d1 = r.record_hit(id);
        assert!(d1.capture && !d1.reached_limit);
        let d2 = r.record_hit(id);
        assert!(d2.capture && d2.reached_limit, "2nd hit reaches the limit");
        // After the limit, no more captures and the entry is inactive.
        let d3 = r.record_hit(id);
        assert!(!d3.capture && !d3.reached_limit);
        assert!(!r.get(id).unwrap().active);
    }

    #[test]
    fn record_hit_on_unknown_id_is_benign() {
        let mut r = HookRegistry::new();
        let d = r.record_hit(42);
        assert!(!d.capture && !d.reached_limit);
    }
}
