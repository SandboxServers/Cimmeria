//! `POST /api/telemetry/upload-bundle`: the end-of-session zip of the
//! client's log files, replayed one line per event.
//!
//! The gate ([`super::upload_gate::admit`]) runs before the multipart body
//! is read, and the body must arrive within the body deadline. Then the
//! budgets. The first four refuse the upload with 413:
//!
//! | Budget | Checked |
//! |---|---|
//! | [`super::MAX_BUNDLE_BYTES`] request body | by the route's `DefaultBodyLimit`, as the parts stream in |
//! | [`super::MAX_BUNDLE_PARTS`] multipart parts, one `zip` part | per part |
//! | [`super::MAX_BUNDLE_METADATA_BYTES`] metadata | as the part streams in |
//! | [`super::MAX_BUNDLE_ENTRIES_HARD`] zip entries | from the zip's end record, before the archive is opened |
//!
//! The expansion budgets stop the replay instead and answer 200 with
//! `truncated: true` (older launchers bundle every past session's logs, so
//! a refusal would only be retried). Files are replayed newest first, so
//! the session that just ended is the one that survives:
//!
//! | Budget | Checked |
//! |---|---|
//! | [`super::MAX_BUNDLE_ENTRIES`] files | the newest are kept |
//! | [`super::MAX_BUNDLE_EXPANDED_BYTES`] expanded bytes, all files together | on each file's declared size before it is expanded, then on the bytes actually read; a file over what is left is not replayed |
//! | [`super::MAX_BUNDLE_LINES`] replayed lines | per line |
//!
//! Each line is cut to [`super::field_caps::MAX_MESSAGE_BYTES`] before it
//! is replayed.

use std::net::{IpAddr, SocketAddr};
use std::time::Instant;

use axum::extract::multipart::Field;
use axum::extract::{ConnectInfo, FromRequest, Multipart, Request};
use axum::Json;

use crate::routes::dev_session::TokenClaims;

use super::bundle_unzip::{declared_zip_entries, unpack_and_replay, BundleCounts};
use super::dto::{BundleResponse, IngestError};
use super::field_caps::{capped, MAX_LABEL_BYTES, MAX_MESSAGE_BYTES};
use super::upload_gate::{
    admit, by_deadline, upload_state, Admitted, Route, UploadPolicy, UploadState, Uploader,
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
    let Admitted { claims, slot } =
        admit(state, policy, Route::Bundle, request.headers(), who, now)?;
    // The slot moves into the unzip worker, so it is held for as long as
    // the zip is expanded even if the client goes away and this future is
    // dropped.
    let mut slot = Some(slot);
    let limits = &state.limits;
    let deadline = tokio::time::Instant::now() + limits.body_timeout;
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|e| IngestError::Multipart(e.body_text()))?;

    let mut metadata_seen = false;
    let mut parts = 0usize;
    let mut counts = BundleCounts::default();

    while let Some(field) = by_deadline(deadline, async {
        multipart
            .next_field()
            .await
            .map_err(|e| IngestError::Multipart(e.to_string()))
    })
    .await?
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
                let bytes = read_field_capped(
                    field,
                    limits.bundle_metadata_bytes,
                    "metadata bytes",
                    deadline,
                )
                .await?;
                log_metadata(&claims, &bytes);
                metadata_seen = true;
            }
            "zip" => {
                // One zip per bundle: the budgets below are per zip, and a
                // second part would get a fresh set.
                let Some(slot) = slot.take() else {
                    return Err(IngestError::OverBudget {
                        what: "zip parts",
                        limit: 1,
                    });
                };
                let bytes =
                    read_field_capped(field, limits.bundle_body_bytes, "body bytes", deadline)
                        .await?;
                // Opening the archive reads every entry's header, so a zip
                // that declares too many is refused from its end record
                // first.
                if let Some(entries) = declared_zip_entries(&bytes) {
                    if entries > limits.bundle_entries_hard as u64 {
                        return Err(IngestError::OverBudget {
                            what: "zip entries (end record)",
                            limit: limits.bundle_entries_hard as u64,
                        });
                    }
                }
                // Bundle unzip is CPU-bound and synchronous (the `zip`
                // crate is blocking), and each line is a tracing event:
                // run it on a blocking worker, which the slot limits.
                let claims_clone = claims.clone();
                let limits_clone = limits.clone();
                counts = tokio::task::spawn_blocking(move || {
                    let _slot = slot;
                    unpack_and_replay(&claims_clone, &bytes, &limits_clone)
                })
                .await
                .map_err(|e| IngestError::Multipart(format!("bundle unpack join failed: {e}")))??;
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

    if let Some(t) = &counts.truncation {
        state.refusals.report_truncated(Route::Bundle, who, t, now);
    }
    Ok(BundleResponse {
        files: counts.files,
        lines: counts.lines,
        skipped_not_log: counts.skipped_not_log,
        truncated: counts.truncation.is_some(),
    })
}

/// Read one multipart part by `deadline`, refusing the upload once it
/// passes `cap`.
async fn read_field_capped(
    mut field: Field<'_>,
    cap: usize,
    what: &'static str,
    deadline: tokio::time::Instant,
) -> Result<Vec<u8>, IngestError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = by_deadline(deadline, async {
        field
            .chunk()
            .await
            .map_err(|e| IngestError::Multipart(e.to_string()))
    })
    .await?
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
