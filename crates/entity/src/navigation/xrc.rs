//! XRC binary-format reader helpers and header sanity caps.
//!
//! The XRC format stores a single-tile Recast polygon mesh with detail
//! triangulation. These helpers parse the little-endian scalar fields and
//! bound every header count against a documented maximum before it can
//! drive an allocation in [`super::NavMesh::load`].

use std::io::Read as IoRead;

// ── XRC binary reader helpers ────────────────────────────────────────────

pub(super) fn read_f32(r: &mut impl IoRead) -> std::io::Result<f32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(f32::from_le_bytes(buf))
}

pub(super) fn read_u32(r: &mut impl IoRead) -> std::io::Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub(super) fn read_u16(r: &mut impl IoRead) -> std::io::Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_le_bytes(buf))
}

pub(super) fn read_u8(r: &mut impl IoRead) -> std::io::Result<u8> {
    let mut buf = [0u8; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

// ── XRC header sanity caps ──────────────────────────────────────────────
//
// These mirror the caps in `cimmeria_navmesh_extractor::nav_roundtrip` so a
// hostile `.nav` file is rejected with the same diagnostic whether it reaches
// the build-time tool or the runtime loader. Castle Cellblock — the largest
// shipped navmesh at PR-time — has `nverts = 2778`, `npolys = 1479`,
// `detail_nverts = 6031`, `detail_ntris = 3102`. The caps below leave
// 360× to 1500× headroom over real assets while still bounding the worst-case
// allocation to a few hundred MB instead of a u32-multiplication wrap that
// produces a tiny `Vec` the read loop then walks past.

/// Upper bound on `nverts` accepted from a `.nav` header.
///
/// Worst-case `verts` allocation at this cap is
/// `1_000_000 * 3 * size_of::<u16>() = ~6 MB`.
pub(super) const MAX_NVERTS: u32 = 1_000_000;

/// Upper bound on `npolys`.
pub(super) const MAX_NPOLYS: u32 = 1_000_000;

/// Upper bound on `nvp` (max vertices per polygon). Recast's poly mesh
/// uses values in the 3..=12 range in practice; cap at 64 as a sanity
/// limit. Combined with [`MAX_NPOLYS`], the worst-case `polys` Vec is
/// `1_000_000 * 64 * 2 * 2 = ~256 MB`.
pub(super) const MAX_NVP: u32 = 64;

/// Upper bound on `detail_nmeshes`. Per-poly mesh count, so practically
/// `<= npolys`. Cap at the same 1M to avoid coupling the checks.
pub(super) const MAX_DETAIL_NMESHES: u32 = 1_000_000;

/// Upper bound on `detail_nverts`. Castle Cellblock: 6031.
pub(super) const MAX_DETAIL_NVERTS: u32 = 10_000_000;

/// Upper bound on `detail_ntris`. Castle Cellblock: 3102.
pub(super) const MAX_DETAIL_NTRIS: u32 = 10_000_000;

/// Validate a header count against its documented maximum and return the
/// value on success. Used by [`super::NavMesh::load`] to reject hostile
/// inputs before they reach a multiplication that could overflow into a
/// too-small `Vec` allocation.
pub(super) fn check_count(
    value: u32,
    max: u32,
    field: &'static str,
) -> cimmeria_common::Result<u32> {
    if value > max {
        // Negative log per docs/architecture/negative-logging-convention.md.
        // The error propagates to NavMesh::load's caller, but callers
        // historically used `unwrap_or_default()` patterns that
        // silently swallowed the failure — turning a hostile `.nav`
        // into a navmesh-less space without any greppable signal.
        // The error! here makes the rejection alertable (target:
        // "navmesh.load", reason: "header_out_of_range") regardless
        // of how the caller chooses to handle the propagated Err.
        tracing::error!(
            target: "navmesh.load",
            field = field,
            value,
            max,
            reason = "header_out_of_range",
            "rejected hostile .nav header field -- space will be navmesh-less"
        );
        return Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
            field,
            value: value as u64,
            reason: "exceeds runtime sanity cap",
        });
    }
    Ok(value)
}

/// Compute `count * stride` as `usize`, failing with a descriptive error
/// on overflow. Both inputs are passed as `u32` to match the on-disk
/// header types; the multiplication is performed in `u64` so the failure
/// mode is the same on 32-bit and 64-bit targets.
///
/// `field` names the header-count source of the multiplication (always a
/// real header field — e.g. `"nverts"`, not a compound expression). The
/// caller passes `alloc_desc` to describe the multiplication shape so the
/// error names *which* downstream allocation would have busted (e.g.
/// `"polys = npolys * nvp * 2 u16s"`). On overflow we widen `value` to
/// report whichever number the operator can still diagnose: for the u64
/// `checked_mul` failure that's the raw count (the product is by
/// definition unrepresentable in u64); for the `usize::try_from` failure
/// the product fits in u64 and is reported as-is.
pub(super) fn checked_alloc_size(
    count: u32,
    stride: u32,
    field: &'static str,
    alloc_desc: &'static str,
) -> cimmeria_common::Result<usize> {
    let product = (count as u64).checked_mul(stride as u64).ok_or(
        cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
            field,
            value: count as u64,
            reason: alloc_desc,
        },
    )?;
    usize::try_from(product).map_err(|_| cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
        field,
        value: product,
        reason: alloc_desc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── scalar reader helpers ────────────────────────────────────────────

    /// Each reader decodes its little-endian scalar from an exact-length
    /// slice. Pins the success path of all four readers, which the
    /// header-cap tests never exercise directly.
    #[test]
    fn readers_decode_little_endian_scalars() {
        let mut r = &[0x78u8, 0x56, 0x34, 0x12][..];
        assert_eq!(read_u32(&mut r).unwrap(), 0x1234_5678);

        let mut r = &1.5f32.to_le_bytes()[..];
        assert_eq!(read_f32(&mut r).unwrap(), 1.5);

        let mut r = &[0xCDu8, 0xAB][..];
        assert_eq!(read_u16(&mut r).unwrap(), 0xABCD);

        let mut r = &[0x2Au8][..];
        assert_eq!(read_u8(&mut r).unwrap(), 0x2A);
    }

    /// A reader whose source ends before the field is fully read must
    /// surface the `read_exact` short-read error rather than returning a
    /// truncated/zero value. Covers the hostile-input `?`-propagation
    /// branch in each reader (the `Err(_)` arm of `read_exact`). A
    /// truncated `.nav` file is exactly this shape.
    #[test]
    fn readers_propagate_short_read_error() {
        // f32/u32 need 4 bytes — give 3.
        let mut r = &[0x00u8, 0x01, 0x02][..];
        assert!(read_u32(&mut r).is_err());
        let mut r = &[0x00u8, 0x01, 0x02][..];
        assert!(read_f32(&mut r).is_err());

        // u16 needs 2 bytes — give 1.
        let mut r = &[0x00u8][..];
        assert!(read_u16(&mut r).is_err());

        // u8 needs 1 byte — give 0 (empty).
        let mut r = &[][..];
        assert!(read_u8(&mut r).is_err());
    }

    /// `check_count` returns the value unchanged when it is within the
    /// cap (including the boundary `value == max`). The existing
    /// `check_count_emits_error_log_on_reject` test only covers the
    /// over-cap reject path; this pins the `Ok(value)` accept path.
    #[test]
    fn check_count_accepts_value_at_or_below_cap() {
        assert_eq!(check_count(0, MAX_NVERTS, "nverts").unwrap(), 0);
        assert_eq!(
            check_count(MAX_NVERTS, MAX_NVERTS, "nverts").unwrap(),
            MAX_NVERTS,
            "boundary value == max must be accepted"
        );
        assert_eq!(
            check_count(MAX_NPOLYS - 1, MAX_NPOLYS, "npolys").unwrap(),
            MAX_NPOLYS - 1
        );
    }

    /// Pin the u32→u64 widening contract of `checked_alloc_size`.
    ///
    /// Reviewer flagged that the existing hostile-input guards exercise
    /// the `check_count` cap path but never the `checked_mul`/`try_from`
    /// inside `checked_alloc_size` itself. The current `check_count` caps
    /// make the helper's overflow branches structurally unreachable from
    /// `NavMesh::load` on a 64-bit target — every count is `<= 10M` and
    /// strides are `<= 64` so `count * stride <= 640M`, well below
    /// `u32::MAX`. But the helper is intentionally written so that
    /// removing the caps (or raising them past u32 boundaries) still
    /// can't bust the allocation: the multiplication widens to u64 first.
    ///
    /// This test calls `checked_alloc_size` directly with `count =
    /// u32::MAX, stride = 2` — a product that wraps to `0xFFFF_FFFE` if
    /// the widening is removed, but expands to `0x1_FFFF_FFFE` (=
    /// `2 * u32::MAX`) when widened. We assert the helper returns the
    /// widened product. A refactor that drops the `as u64` casts (e.g.
    /// `count.checked_mul(stride).map(|p| p as u64)`) would wrap inside
    /// the u32 multiplication and return `Ok(0xFFFF_FFFE)`, failing this
    /// test — that's the realistic regression shape this guard catches.
    ///
    /// On a 64-bit target this returns `Ok` (usize is u64). On a 32-bit
    /// target the same call surfaces `NavHeaderOutOfRange` via the
    /// `usize::try_from` branch with the `alloc_desc` carried in
    /// `reason`. The test asserts both shapes.
    #[test]
    fn checked_alloc_size_widens_before_multiplying() {
        const DESC: &str = "test stride";
        let result = checked_alloc_size(u32::MAX, 2, "synthetic", DESC);

        #[cfg(target_pointer_width = "64")]
        {
            let got = result.expect("widened product must fit in 64-bit usize");
            // 2 * (2^32 - 1) = 2^33 - 2 = 0x1_FFFF_FFFE. If the function
            // wraps in u32 first this would be 0xFFFF_FFFE — half as
            // large — and the assertion fires.
            assert_eq!(
                got, 0x1_FFFF_FFFE_usize,
                "checked_alloc_size must widen u32 -> u64 BEFORE multiplying; \
                 got {got:#x}, expected {:#x}. If you see {:#x}, the multiplication \
                 wrapped in u32 — restore the `as u64` casts in `checked_alloc_size`.",
                0x1_FFFF_FFFE_u64, 0xFFFF_FFFE_u64,
            );
        }

        #[cfg(target_pointer_width = "32")]
        {
            // Product fits in u64 but not usize=u32; expect the
            // `usize::try_from` branch to fire with alloc_desc as reason.
            match result {
                Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange {
                    field,
                    value,
                    reason,
                }) => {
                    assert_eq!(field, "synthetic");
                    assert_eq!(value, 0x1_FFFF_FFFE_u64);
                    assert_eq!(reason, DESC);
                }
                other => panic!("expected NavHeaderOutOfRange on 32-bit target, got {other:?}"),
            }
        }
    }

    /// Companion to the widening guard: when the u64 product genuinely
    /// would not fit in any addressable size, the helper reports the
    /// product via `value` and the caller-supplied `alloc_desc` via
    /// `reason`. We can only synthesise the u64-overflow path by passing
    /// the helper inputs that exceed what `NavMesh::load` ever produces,
    /// so this test exercises the helper directly.
    ///
    /// `u32::MAX * u32::MAX = 2^64 - 2^33 + 1` — fits in u64 (just), so
    /// the `checked_mul` branch in `checked_alloc_size` is unreachable
    /// with u32-typed inputs. The function is still correct: the only
    /// way that branch can fire today is from a u32 overflow in the
    /// product that's caught by u64 widening, which is what the
    /// `checked_alloc_size_widens_before_multiplying` test pins.
    /// We document the dead-code branch here so a future maintainer
    /// raising the helper to u64 inputs knows the test surface they
    /// need to expand.
    #[test]
    fn checked_alloc_size_max_u32_inputs_fit_in_u64() {
        let result = checked_alloc_size(u32::MAX, u32::MAX, "synthetic", "max product");
        #[cfg(target_pointer_width = "64")]
        {
            // (2^32 - 1)^2 = 2^64 - 2^33 + 1 = 0xFFFF_FFFE_0000_0001
            let got = result.expect("u32*u32 product must fit in 64-bit usize");
            assert_eq!(got, 0xFFFF_FFFE_0000_0001_usize);
        }
        #[cfg(target_pointer_width = "32")]
        {
            // 32-bit usize can't hold the product; expect rejection via
            // the `usize::try_from` branch (NOT the `checked_mul` branch,
            // which is unreachable with u32 inputs).
            assert!(matches!(
                result,
                Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { .. })
            ));
        }
    }

    /// Regression guard (audit #482): a header field exceeding its
    /// cap must emit a structured `error!` on the `navmesh.load`
    /// target with `reason = "header_out_of_range"` BEFORE returning
    /// the propagating Err. Callers historically swallow the Err via
    /// `unwrap_or_default()` patterns — without this log, a hostile
    /// `.nav` silently turns the affected space navmesh-less and
    /// players fall through the world with no alertable signal.
    ///
    /// Bug shape: revert the error! line in `check_count` and the
    /// assertion below fails on the empty event list.
    #[test]
    fn check_count_emits_error_log_on_reject() {
        use std::sync::{Arc, Mutex};
        use tracing::{
            field::{Field, Visit},
            Event, Subscriber,
        };
        use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

        // Minimal in-test event capture. tracing-subscriber is a
        // dev-dependency only — see this crate's Cargo.toml.
        struct CapLayer {
            events: Arc<
                Mutex<
                    Vec<(
                        tracing::Level,
                        String,
                        std::collections::HashMap<String, String>,
                    )>,
                >,
            >,
        }
        impl<S: Subscriber> Layer<S> for CapLayer {
            fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
                let metadata = event.metadata();
                struct V {
                    fields: std::collections::HashMap<String, String>,
                }
                impl Visit for V {
                    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                        self.fields.insert(f.name().to_string(), format!("{v:?}"));
                    }
                    fn record_str(&mut self, f: &Field, v: &str) {
                        self.fields.insert(f.name().to_string(), v.to_string());
                    }
                    fn record_u64(&mut self, f: &Field, v: u64) {
                        self.fields.insert(f.name().to_string(), v.to_string());
                    }
                    fn record_i64(&mut self, f: &Field, v: i64) {
                        self.fields.insert(f.name().to_string(), v.to_string());
                    }
                }
                let mut visitor = V {
                    fields: std::collections::HashMap::new(),
                };
                event.record(&mut visitor);
                self.events.lock().unwrap().push((
                    *metadata.level(),
                    metadata.target().to_string(),
                    visitor.fields,
                ));
            }
        }
        let events = Arc::new(Mutex::new(Vec::new()));
        let layer = CapLayer {
            events: events.clone(),
        };
        let subscriber = tracing_subscriber::Registry::default().with(layer);
        let _guard = tracing::subscriber::set_default(subscriber);

        // Synthesise a hostile header value (`nverts` exceeds the cap).
        let result = check_count(MAX_NVERTS + 1, MAX_NVERTS, "nverts");
        assert!(
            matches!(
                result,
                Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { .. })
            ),
            "check_count must propagate Err on out-of-range input"
        );

        let captured: Vec<_> = events.lock().unwrap().clone();
        let reject_log = captured.iter().find(|(level, target, fields)| {
            *level == tracing::Level::ERROR
                && target == "navmesh.load"
                && fields.get("reason").map(String::as_str) == Some("header_out_of_range")
        });
        assert!(
            reject_log.is_some(),
            "check_count rejection must emit error! at target='navmesh.load' with \
             reason='header_out_of_range'. Reverting the error! line in `check_count` \
             breaks the silent-fall-through alert path documented in audit #482. \
             Captured: {captured:#?}"
        );
        let (_, _, fields) = reject_log.unwrap();
        assert_eq!(
            fields.get("field").map(String::as_str),
            Some("nverts"),
            "warn must carry the rejected field name: {fields:#?}"
        );
    }
}
