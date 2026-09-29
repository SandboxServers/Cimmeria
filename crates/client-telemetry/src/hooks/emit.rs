//! One place the entity, net and Lua detours hand an event to the queue.
//!
//! Every event goes to the telemetry producer; under the `lab-bridge`
//! feature it is also pushed to the bridge's local ring, so
//! `client_events_read` sees it without a SigNoz round trip. The ring kind
//! is the target without its `client.` prefix (`entity.create`,
//! `net.out`, ...).

/// The ring kind for a telemetry target: `client.entity.create` becomes
/// `entity.create`.
#[cfg_attr(not(feature = "lab-bridge"), allow(dead_code))]
pub(crate) fn ring_kind(target: &str) -> &str {
    target.strip_prefix("client.").unwrap_or(target)
}

/// Emit one event. Non-blocking; a full queue drops it and counts.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(crate) fn emit(target: &'static str, level: &'static str, fields: super::entity_trace::Fields) {
    #[cfg(feature = "lab-bridge")]
    crate::bridge::events::push(
        ring_kind(target),
        crate::bridge::crash::now_ms(),
        serde_json::Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ),
    );

    if let Some(p) = crate::boot::producer() {
        let mut b = crate::events::ClientNativeEvent::builder(target, level);
        for (k, v) in fields {
            b = b.field(k, v);
        }
        p.try_emit(b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_kind_drops_the_client_prefix() {
        assert_eq!(ring_kind("client.entity.create"), "entity.create");
        assert_eq!(ring_kind("client.net.out"), "net.out");
        assert_eq!(ring_kind("other"), "other");
    }
}
