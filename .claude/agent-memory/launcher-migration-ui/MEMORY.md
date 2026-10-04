# Migration settings integration — 2026-10-04

- `shell/src/host/migration/` caches a native-selected preview and its preferences
  revision; confirm IPC carries only its digest, revision and explicit Boolean.
  Extra command fields are rejected. Engine source reread remains authoritative.
- `frontend/src/migration-workflow.ts` does not retry confirmation; inspection
  reconciles a lost response. A failed import clears the native pending preview.
- Imports archive legacy configuration and identity, retain separate game consent,
  and preserve desktop summary consent. They grant no signed ownership. Current
  Play does not consume the archived legacy configuration. Verified adoption and
  updater/installed/launch parity remain separate work.
- Native fixture UAT is `frontend/migration-native-uat.mjs`, driven by the ignored
  shell test `migration_native_uat_bridge`; it touches only temporary fixture data.
