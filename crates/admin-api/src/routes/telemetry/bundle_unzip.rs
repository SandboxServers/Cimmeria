//! Expanding and replaying a bundle zip within its budgets (see the table
//! in [`super::bundle`]).

use std::io::Read;

use crate::routes::dev_session::TokenClaims;

use super::dto::IngestError;
use super::field_caps::{capped, MAX_LABEL_BYTES, MAX_MESSAGE_BYTES};
use super::refusal_log::Truncation;
use super::upload_gate::UploadLimits;

/// What one bundle replayed, and where it stopped if it passed a budget.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct BundleCounts {
    pub files: u64,
    pub lines: u64,
    pub truncation: Option<Truncation>,
}

/// The entry count a zip's end-of-central-directory record declares, read
/// without opening the archive (opening it reads every central-directory
/// entry). `u64::MAX` for a Zip64 archive, whose real count lives in
/// another record: the launcher's bundles never need Zip64, and anything
/// that does is over every budget here. `None` when there is no end record;
/// the archive open then refuses it.
pub(super) fn declared_zip_entries(bytes: &[u8]) -> Option<u64> {
    const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const EOCD_LEN: usize = 22;
    if bytes.len() < EOCD_LEN {
        return None;
    }
    // The record sits at the very end, followed only by a comment of up
    // to 64 KiB whose length it states.
    let last = bytes.len() - EOCD_LEN;
    let first = last.saturating_sub(u16::MAX as usize);
    (first..=last).rev().find_map(|pos| {
        let rec = &bytes[pos..pos + EOCD_LEN];
        let comment_len = u16::from_le_bytes([rec[20], rec[21]]) as usize;
        if rec[..4] != EOCD_SIG || pos + EOCD_LEN + comment_len != bytes.len() {
            return None;
        }
        let total = u16::from_le_bytes([rec[10], rec[11]]);
        Some(if total == u16::MAX {
            u64::MAX
        } else {
            u64::from(total)
        })
    })
}

/// Expand and replay the zip's files, newest first, within `limits`. Only
/// a malformed zip or one over the hard entry cap is an error; an
/// expansion budget stops the replay and is reported in the counts.
pub(super) fn unpack_and_replay(
    claims: &TokenClaims,
    zip_bytes: &[u8],
    limits: &UploadLimits,
) -> Result<BundleCounts, IngestError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| IngestError::Zip(e.to_string()))?;
    if zip.len() > limits.bundle_entries_hard {
        return Err(IngestError::OverBudget {
            what: "zip entries",
            limit: limits.bundle_entries_hard as u64,
        });
    }

    let mut order = newest_first(&mut zip)?;

    // `kept` is filled in on the way out.
    let truncation = |budget, limit, dropped_estimate| Truncation {
        budget,
        limit,
        kept: 0,
        dropped_estimate,
    };
    let mut counts = BundleCounts::default();
    if order.len() > limits.bundle_entries {
        let dropped = (order.len() - limits.bundle_entries) as u64;
        order.truncate(limits.bundle_entries);
        counts.truncation = Some(truncation(
            "zip entries",
            limits.bundle_entries as u64,
            dropped,
        ));
    }
    replay_entries(claims, &mut zip, order, limits, &mut counts, truncation)?;
    if let Some(t) = counts.truncation.as_mut() {
        t.kept = counts.lines;
    }
    Ok(counts)
}

/// The zip's files, newest first: by the entry's own timestamp, then by
/// its place in the archive (later first), so the session that just ended
/// is replayed before any budget runs out. Returns `(stamp, index)` pairs.
pub(super) fn newest_first<R: std::io::Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
) -> Result<Vec<(u32, usize)>, IngestError> {
    let mut order = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let entry = zip
            .by_index_raw(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if entry.is_file() {
            let stamp = entry.last_modified().map_or(0, |d| {
                (u32::from(d.datepart()) << 16) | u32::from(d.timepart())
            });
            order.push((stamp, i));
        }
    }
    order.sort_unstable_by(|a, b| b.cmp(a));
    Ok(order)
}

/// Replay the entries at `order` (archive indexes, newest first) until the
/// byte or line budget runs out, recording where in `counts`.
fn replay_entries<R: std::io::Read + std::io::Seek>(
    claims: &TokenClaims,
    zip: &mut zip::ZipArchive<R>,
    order: Vec<(u32, usize)>,
    limits: &UploadLimits,
    counts: &mut BundleCounts,
    truncation: impl Fn(&'static str, u64, u64) -> Truncation,
) -> Result<(), IngestError> {
    let mut expanded = 0u64;
    for (_, i) in order {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        let path = capped(entry.name(), MAX_LABEL_BYTES).into_owned();
        let remaining = limits.bundle_expanded_bytes - expanded;
        let over_bytes = truncation("expanded bytes", limits.bundle_expanded_bytes, entry.size());
        // The declared size first, so an entry known to be too big is not
        // expanded at all...
        if entry.size() > remaining {
            counts.truncation = Some(over_bytes);
            break;
        }
        // ...then the bytes actually read, because the declared size is
        // the zip's own claim. The whole entry is read before any of it is
        // replayed.
        let mut content = Vec::new();
        (&mut entry as &mut dyn Read)
            .take(remaining + 1)
            .read_to_end(&mut content)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if content.len() as u64 > remaining {
            counts.truncation = Some(over_bytes);
            break;
        }
        expanded += content.len() as u64;

        // Tolerate non-UTF8 binary files (key dumps may contain binary).
        // Skip with a debug-level note rather than failing the whole
        // bundle.
        let Ok(content) = String::from_utf8(content) else {
            tracing::debug!(
                target: "launcher.bundle",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                path = %path,
                "skipping non-UTF8 bundle entry"
            );
            continue;
        };
        counts.files += 1;
        let lines: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
        for (n, line) in lines.iter().enumerate() {
            if counts.lines >= limits.bundle_lines {
                counts.truncation = Some(truncation(
                    "lines",
                    limits.bundle_lines,
                    (lines.len() - n) as u64,
                ));
                return Ok(());
            }
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                cimmeria.session_kind = claims.session_kind(),
                lab = claims.is_lab(),
                source = "bundle",
                source_file = %path,
                message = %capped(line, MAX_MESSAGE_BYTES),
            );
            counts.lines += 1;
        }
    }
    Ok(())
}
