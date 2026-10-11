---
name: otel-log-redaction-wrap-provider
description: To redact OTLP log fields (opentelemetry 0.33), wrap the LoggerProvider the tracing bridge uses; a LogProcessor cannot change attributes
metadata:
  type: reference
---

opentelemetry_sdk 0.33 gives a `LogProcessor` `&mut SdkLogRecord`, but there is no way to change or remove an attribute already on it, and `SdkLogRecord::new` is crate-private. To redact field values, wrap the provider: `OpenTelemetryTracingBridge<P, L>` is generic over `LoggerProvider`/`Logger`, so a wrapper whose `LogRecord` cleans values in `set_body`/`add_attribute` sees every field before storage (TG-PIPE-17, `crates/lab/src/otlp/logger.rs`).

- tracing's event name is `event <file!()>:<line>`; for a dependency that is an absolute cargo-registry path (`C:\Users\<name>\.cargo\...`). It is `&'static`, so drop it rather than redact.
- `cimmeria-lab` is a hakari traversal exclude: adding deps there changes only `Cargo.lock`, no workspace-hack lines.
- labd uses `CIMMERIA_LAB_OTLP_ENDPOINT`, not `OTEL_EXPORTER_OTLP_ENDPOINT`, which other tools set user-wide.
