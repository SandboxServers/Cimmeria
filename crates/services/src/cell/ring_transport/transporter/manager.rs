//! Process-wide registry of ring transporters (one entry per region_id).
//!
//! All worlds' transporters live in the same map because cross-world rings
//! need to find each other by region_id.

use std::collections::HashMap;
use std::time::Instant;

use super::super::regions::RingRegion;
use super::{DeadlineKind, RingTransporter};

#[derive(Debug, Default)]
pub struct RingTransporterManager {
    pub regions: HashMap<i32, RingTransporter>,
}

impl RingTransporterManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build one transporter per region in the supplied table. Idempotent —
    /// re-init clears any in-flight FSM state (use only at startup).
    pub fn load(&mut self, ring_regions: &HashMap<i32, RingRegion>) {
        self.regions.clear();
        for (id, region) in ring_regions {
            self.regions
                .insert(*id, RingTransporter::from_region(region));
        }
    }

    pub fn get(&self, region_id: i32) -> Option<&RingTransporter> {
        self.regions.get(&region_id)
    }

    pub fn get_mut(&mut self, region_id: i32) -> Option<&mut RingTransporter> {
        self.regions.get_mut(&region_id)
    }

    /// Return all region_ids that have an elapsed deadline at `now`. Used by
    /// the tick to drive timers forward.
    pub fn ready_regions(&self, now: Instant) -> Vec<(i32, RawDeadline)> {
        self.regions
            .iter()
            .filter_map(|(id, r)| r.elapsed_deadline(now).map(|dk| (*id, RawDeadline(dk))))
            .collect()
    }
}

/// Opaque deadline-kind handle returned by `ready_regions`. Wrapping it
/// keeps `DeadlineKind` private while still letting the tick code dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawDeadline(DeadlineKind);

impl RawDeadline {
    pub(crate) fn is_hide(self) -> bool {
        self.0 == DeadlineKind::Hide
    }
    pub(crate) fn is_warmup(self) -> bool {
        self.0 == DeadlineKind::Warmup
    }
    pub(crate) fn is_remote_warmup(self) -> bool {
        self.0 == DeadlineKind::RemoteWarmup
    }
    pub(crate) fn is_cooldown(self) -> bool {
        self.0 == DeadlineKind::Cooldown
    }
}
