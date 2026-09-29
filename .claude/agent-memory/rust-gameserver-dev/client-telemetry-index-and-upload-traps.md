---
name: client-telemetry-index-and-upload-traps
description: Why client telemetry never reached SigNoz before 2026-09-29 (base-vs-chunk URL, placeholder lab token, no colo secret, DLL not in the player launch) and how the cimmeria-client index routes
metadata:
  type: project
---

Client telemetry had ~1 SigNoz row in 7 days. Four independent breaks,
found 2026-09-29 while adding the `cimmeria-client` index:

1. **Base vs chunk URL.** The dev-session mint returns `upload_endpoint`
   as the upload *base* (`…/api/telemetry`); the launcher appends
   `/upload-chunk`. The DLL POSTed to it verbatim, so no launcher-written
   session ever delivered a batch. Fixed with `uploader::chunk_url`.
2. **Lab wrote a placeholder.** The supervisor put the bridge token and
   `http://127.0.0.1/api/telemetry/upload-chunk` in the telemetry block.
   Now `supervisor/telemetry_session.rs` mints with `session_kind = lab`.
3. **Colo had no `CIMMERIA_TELEMETRY_HMAC_SECRET`**, and the refusal was
   invisible server-side (`log_refusal` ignored `SecretMissing`). Now an
   ERROR per request and a WARN at boot (`dev_session_secret_unusable`).
   Compose passes `${VAR:-}` → an EMPTY string env var: any env reader
   must treat blank as unset (`upload_endpoint_from`).
4. **Players' normal launch never injects the telemetry DLL** and the
   DLL is not packaged; only the lab and the dead-code
   `LaunchSgwWithClientTelemetry` command inject it. Needs an owner
   decision; not fixed.

**Routing:** `otel::CLIENT_TARGETS` (`client.native`,
`launcher.client_log|debug_log|session_meta`) go only to the fourth
provider (`cimmeria-client`, resource `cimmeria.source=client`), every
level. `routes_to_server`/`routes_to_trace` exclude them. Per-session
facts are record attributes (resources are process-wide). The parity
harness has an `OTLP_CLIENT` sink; `client_index_tests.rs` drives a real
replay into a recording `LogExporter` (implement `export` as
`async fn` + `set_resource`; no `testing` feature needed).

**How to apply:** when a client row "doesn't show up", check in this
order: session file endpoint shape, token (secret set? kind?), boot log
line `dev-session telemetry:`, then `service.name = 'cimmeria-client'`.
Dotted tracing field names (`cimmeria.session_kind = x`) work and
become OTLP attribute keys verbatim. See also
[[tracing-span-fields-not-on-log-records]], [[log-filter-parity-traps]].
