//! Axum handlers and the unzip / verify / replay helpers they call.

use std::io::Read;
use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::http::HeaderMap;
use axum::Json;
use bytes::Bytes;
use flate2::read::GzDecoder;

use cimmeria_services::orchestrator::Orchestrator;

use crate::routes::dev_session::{decode_token, AuthError, TokenClaims, SCOPE_TELEMETRY_WRITE};

use super::dto::{BundleResponse, ChunkResponse, IngestError};
use super::replay::replay_ndjson;
use super::{
    MAX_BUNDLE_BYTES, MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES, MAX_CHUNK_BYTES,
    MAX_CHUNK_DECOMPRESSED_BYTES,
};

pub(super) async fn upload_chunk(
    State(_orchestrator): State<Arc<Orchestrator>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ChunkResponse>, IngestError> {
    let claims = verify_bearer(&headers)?;

    if body.len() > MAX_CHUNK_BYTES {
        return Err(IngestError::TooLarge(body.len(), MAX_CHUNK_BYTES));
    }

    // Decompress gzip → NDJSON, bounded to MAX_CHUNK_DECOMPRESSED_BYTES.
    // A gzip bomb (e.g. all-zeros input compressing 1000:1) could
    // otherwise expand 16 MiB of compressed input into multiple GB
    // of allocated `String`. `Read::take` short-circuits the read at
    // the cap; we then check whether the decoder produced anything
    // beyond the cap (it shouldn't, but the explicit check makes the
    // refusal mode visible).
    let mut decoder = GzDecoder::new(&body[..]).take(MAX_CHUNK_DECOMPRESSED_BYTES + 1);
    let mut ndjson = String::new();
    decoder
        .read_to_string(&mut ndjson)
        .map_err(|e| IngestError::Gzip(e.to_string()))?;
    if ndjson.len() as u64 > MAX_CHUNK_DECOMPRESSED_BYTES {
        return Err(IngestError::TooLarge(
            ndjson.len(),
            MAX_CHUNK_DECOMPRESSED_BYTES as usize,
        ));
    }

    let counts = replay_ndjson(&claims, &ndjson).map_err(|e| IngestError::Ndjson {
        line: e.line,
        err: e.err,
    })?;
    let (accepted, parsed) = (counts.accepted, counts.parsed);

    tracing::debug!(
        target: "launcher.ingest",
        session_id = %claims.sid,
        install_id = %claims.sub,
        session_kind = claims.session_kind(),
        accepted,
        parsed,
        body_bytes = body.len(),
        "upload-chunk accepted"
    );

    Ok(Json(ChunkResponse {
        accepted,
        parsed_lines: parsed,
    }))
}

pub(super) async fn upload_bundle(
    State(_orchestrator): State<Arc<Orchestrator>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<BundleResponse>, IngestError> {
    let claims = verify_bearer(&headers)?;

    let mut metadata_seen = false;
    let mut files = 0u64;
    let mut lines = 0u64;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| IngestError::Multipart(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|e| IngestError::Multipart(e.to_string()))?;

        match name.as_str() {
            "metadata" => {
                // Replay the bundle metadata as a single tracing event so
                // SigNoz queries can correlate the chunk stream with the
                // end-of-session totals. Loud on parse failure: malformed
                // metadata indicates a launcher/server schema drift and
                // would silently hide the bundle's session_id correlator.
                match serde_json::from_slice::<serde_json::Value>(&bytes) {
                    Ok(meta) => {
                        tracing::info!(
                            target: "launcher.bundle",
                            session_id = %claims.sid,
                            install_id = %claims.sub,
                            metadata = %meta,
                            "bundle metadata"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "launcher.bundle",
                            session_id = %claims.sid,
                            error = %e,
                            "failed to parse bundle metadata JSON; correlator lost"
                        );
                    }
                }
                metadata_seen = true;
            }
            "zip" => {
                if bytes.len() > MAX_BUNDLE_BYTES {
                    return Err(IngestError::TooLarge(bytes.len(), MAX_BUNDLE_BYTES));
                }
                // Bundle unzip is CPU-bound and synchronous (the `zip`
                // crate is blocking). A 200 MiB bundle with hundreds
                // of thousands of log lines spends most of its time
                // in ZIP decode + tracing event emission, both of
                // which would stall the tokio scheduler if run
                // directly. Move to a blocking worker thread.
                let claims_clone = claims.clone();
                let (f, l) =
                    tokio::task::spawn_blocking(move || unpack_and_replay(&claims_clone, &bytes))
                        .await
                        .map_err(|e| {
                            IngestError::Multipart(format!("bundle unpack join failed: {e}"))
                        })??;
                files += f;
                lines += l;
            }
            other => {
                tracing::debug!(
                    target: "launcher.bundle",
                    session_id = %claims.sid,
                    field = %other,
                    "ignoring unexpected multipart field"
                );
            }
        }
    }

    if !metadata_seen {
        tracing::warn!(
            target: "launcher.bundle",
            session_id = %claims.sid,
            "bundle uploaded without metadata field"
        );
    }

    Ok(Json(BundleResponse { files, lines }))
}

fn unpack_and_replay(claims: &TokenClaims, zip_bytes: &[u8]) -> Result<(u64, u64), IngestError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| IngestError::Zip(e.to_string()))?;

    let mut files = 0u64;
    let mut lines = 0u64;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if !entry.is_file() {
            continue;
        }
        let path = entry.name().to_string();

        // Refuse entries whose declared uncompressed size exceeds the
        // per-entry cap before touching them. A zip bomb advertises a
        // small compressed size but a huge `size()` — bail loud rather
        // than expanding the entry.
        if entry.size() > MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES {
            tracing::warn!(
                target: "launcher.bundle",
                session_id = %claims.sid,
                path = %path,
                declared_size = entry.size(),
                cap = MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES,
                "refusing bundle entry: declared size exceeds cap"
            );
            continue;
        }

        // Bound the actual read too — `size()` is a self-declared field
        // and a malicious zip could lie. `Read::take` caps the bytes
        // we'll ever allocate at the same limit.
        let mut content = String::new();
        let mut bounded =
            (&mut entry as &mut dyn std::io::Read).take(MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES + 1);
        // Tolerate non-UTF8 binary files (key dumps may contain
        // binary). Skip with a debug-level note rather than failing
        // the whole bundle.
        if bounded.read_to_string(&mut content).is_err() {
            tracing::debug!(
                target: "launcher.bundle",
                session_id = %claims.sid,
                path = %path,
                "skipping non-UTF8 bundle entry"
            );
            continue;
        }
        if content.len() as u64 > MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES {
            tracing::warn!(
                target: "launcher.bundle",
                session_id = %claims.sid,
                path = %path,
                decompressed = content.len(),
                cap = MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES,
                "truncating bundle entry: decompressed size exceeded cap"
            );
            // Fall through — emit what we got, but don't grow further.
        }
        files += 1;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                cimmeria.session_kind = claims.session_kind(),
                lab = claims.is_lab(),
                source = "bundle",
                source_file = %path,
                message = %line,
            );
            lines += 1;
        }
    }

    Ok((files, lines))
}

pub(super) fn verify_bearer(headers: &HeaderMap) -> Result<TokenClaims, IngestError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(IngestError::MissingAuth)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(IngestError::MissingAuth)?
        .trim();
    if token.is_empty() {
        return Err(IngestError::MissingAuth);
    }
    // Single source of truth for the HMAC secret — `dev_session::mint`
    // signs with this same loader, so any drift between the two paths
    // would cause every launcher upload to fail HMAC verification.
    let secret = crate::routes::dev_session::load_secret().map_err(IngestError::Auth)?;
    let claims = decode_token(token, &secret).map_err(IngestError::Auth)?;
    let now = chrono::Utc::now().timestamp();
    if claims.exp <= now {
        return Err(IngestError::Auth(AuthError::Expired {
            exp: claims.exp,
            now,
        }));
    }
    // The scope is the only thing that keeps a minted token from
    // being a general-purpose credential, so it has to be checked
    // here rather than assumed from the mint path.
    if !claims.has_scope(SCOPE_TELEMETRY_WRITE) {
        return Err(IngestError::Auth(AuthError::MissingScope {
            wanted: SCOPE_TELEMETRY_WRITE,
        }));
    }
    Ok(claims)
}
