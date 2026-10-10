//! `POST /api/telemetry/upload-bundle`: the end-of-session zip of the
//! client's log files, replayed one line per event.
//!
//! The gate ([`super::upload_gate::admit`]) runs before the multipart body
//! is read. Then the budgets, each refusing the upload with 413 at the
//! first one hit:
//!
//! | Budget | Checked |
//! |---|---|
//! | [`super::MAX_BUNDLE_BYTES`] request body | by the route's `DefaultBodyLimit`, as the parts stream in |
//! | [`super::MAX_BUNDLE_PARTS`] multipart parts, one `zip` part | per part |
//! | [`super::MAX_BUNDLE_METADATA_BYTES`] metadata | as the part streams in |
//! | [`super::MAX_BUNDLE_ENTRIES`] zip entries | from the zip's end record, before the archive is opened |
//! | [`super::MAX_BUNDLE_EXPANDED_BYTES`] expanded bytes, all entries together | from the declared sizes before any entry is expanded, then against the bytes actually read |
//! | [`super::MAX_BUNDLE_LINES`] replayed lines | per line; lines already replayed stay replayed |
//!
//! Each line is cut to [`super::field_caps::MAX_MESSAGE_BYTES`] before it
//! is replayed.

use std::io::Read;
use std::net::{IpAddr, SocketAddr};
use std::time::Instant;

use axum::extract::multipart::Field;
use axum::extract::{ConnectInfo, FromRequest, Multipart, Request};
use axum::Json;

use crate::routes::dev_session::TokenClaims;

use super::dto::{BundleResponse, IngestError};
use super::field_caps::{capped, MAX_LABEL_BYTES, MAX_MESSAGE_BYTES};
use super::upload_gate::{
    admit, upload_state, Admitted, Route, UploadLimits, UploadPolicy, UploadState, Uploader,
};

pub(super) async fn upload_bundle(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Json<BundleResponse>, IngestError> {
    bundle_inner(
        upload_state(),
        &UploadPolicy::from_env(),
        peer.ip(),
        request,
        Instant::now(),
    )
    .await
    .map(Json)
}

/// The bundle ingest with its state and policy passed in, for tests.
pub(super) async fn bundle_inner(
    state: &UploadState,
    policy: &UploadPolicy,
    peer: IpAddr,
    request: Request,
    now: Instant,
) -> Result<BundleResponse, IngestError> {
    let mut who = Uploader::anonymous(peer);
    let result = bundle_flow(state, policy, &mut who, request, now).await;
    if let Err(e) = &result {
        state.refusals.report(Route::Bundle, &who, e, now);
    }
    result
}

async fn bundle_flow(
    state: &UploadState,
    policy: &UploadPolicy,
    who: &mut Uploader,
    request: Request,
    now: Instant,
) -> Result<BundleResponse, IngestError> {
    let Admitted { claims, permit } =
        admit(state, policy, Route::Bundle, request.headers(), who, now)?;
    // The slot moves into the unzip worker, so it is held for as long as
    // the zip is expanded even if the client goes away and this future is
    // dropped.
    let mut permit = Some(permit);
    let limits = &state.limits;
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|e| IngestError::Multipart(e.body_text()))?;

    let mut metadata_seen = false;
    let mut parts = 0usize;
    let mut files = 0u64;
    let mut lines = 0u64;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| IngestError::Multipart(e.to_string()))?
    {
        parts += 1;
        if parts > limits.bundle_parts {
            return Err(IngestError::OverBudget {
                what: "multipart parts",
                limit: limits.bundle_parts as u64,
            });
        }
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "metadata" => {
                let bytes =
                    read_field_capped(field, limits.bundle_metadata_bytes, "metadata bytes")
                        .await?;
                log_metadata(&claims, &bytes);
                metadata_seen = true;
            }
            "zip" => {
                // One zip per bundle: the budgets below are per zip, and a
                // second part would get a fresh set.
                let Some(slot) = permit.take() else {
                    return Err(IngestError::OverBudget {
                        what: "zip parts",
                        limit: 1,
                    });
                };
                let bytes =
                    read_field_capped(field, limits.bundle_body_bytes, "body bytes").await?;
                if let Some(entries) = declared_zip_entries(&bytes) {
                    if entries > limits.bundle_entries as u64 {
                        return Err(IngestError::OverBudget {
                            what: "zip entries",
                            limit: limits.bundle_entries as u64,
                        });
                    }
                }
                // Bundle unzip is CPU-bound and synchronous (the `zip`
                // crate is blocking), and each line is a tracing event:
                // run it on a blocking worker, which the slot limits.
                let claims_clone = claims.clone();
                let limits_clone = limits.clone();
                let (f, l) = tokio::task::spawn_blocking(move || {
                    let _slot = slot;
                    unpack_and_replay(&claims_clone, &bytes, &limits_clone)
                })
                .await
                .map_err(|e| IngestError::Multipart(format!("bundle unpack join failed: {e}")))??;
                files += f;
                lines += l;
            }
            other => {
                tracing::debug!(
                    target: "launcher.bundle",
                    session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                    field = %capped(other, MAX_LABEL_BYTES),
                    "ignoring unexpected multipart field"
                );
            }
        }
    }

    if !metadata_seen {
        tracing::warn!(
            target: "launcher.bundle",
            session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
            "bundle uploaded without metadata field"
        );
    }

    Ok(BundleResponse { files, lines })
}

/// Read one multipart part, refusing the upload once it passes `cap`.
async fn read_field_capped(
    mut field: Field<'_>,
    cap: usize,
    what: &'static str,
) -> Result<Vec<u8>, IngestError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|e| IngestError::Multipart(e.to_string()))?
    {
        if bytes.len() + chunk.len() > cap {
            return Err(IngestError::OverBudget {
                what,
                limit: cap as u64,
            });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Replay the bundle metadata as a single tracing event so SigNoz queries
/// can correlate the chunk stream with the end-of-session totals. Loud on
/// parse failure: malformed metadata indicates a launcher/server schema
/// drift and would silently hide the bundle's session_id correlator.
fn log_metadata(claims: &TokenClaims, bytes: &[u8]) {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(meta) => {
            let meta = meta.to_string();
            tracing::info!(
                target: "launcher.bundle",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                install_id = %claims.sub, // nt:id-only launcher install UUID from the token; it names nothing
                metadata = %capped(&meta, MAX_MESSAGE_BYTES),
                "bundle metadata"
            );
        }
        Err(_) => {
            tracing::warn!(
                target: "launcher.bundle",
                session_id = %claims.sid, // nt:id-only telemetry session UUID from the token; it names nothing
                reason = "bad_metadata",
                metadata_bytes = bytes.len(),
                "failed to parse bundle metadata JSON; correlator lost"
            );
        }
    }
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

/// Expand and replay every file in the zip within `limits`. Returns the
/// files and lines replayed, or the first budget hit.
pub(super) fn unpack_and_replay(
    claims: &TokenClaims,
    zip_bytes: &[u8],
    limits: &UploadLimits,
) -> Result<(u64, u64), IngestError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| IngestError::Zip(e.to_string()))?;
    let over_bytes = || IngestError::OverBudget {
        what: "expanded bytes",
        limit: limits.bundle_expanded_bytes,
    };

    if zip.len() > limits.bundle_entries {
        return Err(IngestError::OverBudget {
            what: "zip entries",
            limit: limits.bundle_entries as u64,
        });
    }
    // Refuse on the declared sizes before anything is expanded or
    // replayed. A zip can lie about them; the reads below are bounded by
    // what is left of the budget either way.
    let mut declared = 0u64;
    for i in 0..zip.len() {
        let entry = zip
            .by_index_raw(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if entry.is_file() {
            declared = declared.saturating_add(entry.size());
        }
    }
    if declared > limits.bundle_expanded_bytes {
        return Err(over_bytes());
    }

    let mut expanded = 0u64;
    let mut files = 0u64;
    let mut lines = 0u64;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if !entry.is_file() {
            continue;
        }
        let path = capped(entry.name(), MAX_LABEL_BYTES).into_owned();

        // Read the whole entry, bounded by what is left of the budget,
        // before replaying any of it.
        let remaining = limits.bundle_expanded_bytes - expanded;
        let mut content = Vec::new();
        (&mut entry as &mut dyn Read)
            .take(remaining + 1)
            .read_to_end(&mut content)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if content.len() as u64 > remaining {
            return Err(over_bytes());
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
        files += 1;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            if lines >= limits.bundle_lines {
                return Err(IngestError::OverBudget {
                    what: "lines",
                    limit: limits.bundle_lines,
                });
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
            lines += 1;
        }
    }

    Ok((files, lines))
}
