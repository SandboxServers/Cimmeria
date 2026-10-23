---
name: Development builds require the release public key for public content
description: Public-catalog key preflight and honest archive failure mapping.
type: reference
---

2026-10-23: [Design](../../../docs/analysis/launcher-uat-fixes/design-development-catalog-key.md)
and [build recipe](../../../crates/launcher/desktop/README.md#build-against-released-game-content)
own this reference. Debug builds without LAUNCHER_MANIFEST_PUBKEY_HEX use the
default test key. Public catalog preflight now refuses it before networking
with DevelopmentSigningKey / development_key; missing usable keys remain
SigningKeyUnavailable / signing_key_unavailable. An explicit compile-time
release public verification key supports published content in debug builds.
Local verify_release fixtures remain unchanged. Rebuild to change the key;
setting a runtime environment variable cannot change a compiled launcher.

Archive IPC distinguishes signing_key_unavailable, release_verification_failed
and network release_unavailable. Preflight refusal leaves the archive unchecked
and unchanged. Install uses its existing SigningKeyUnavailable error. Shared
copy directs players to the published ZIP and developers to the release public
key; never use a private signing key or weaken signature/archive verification.
No player-specific telemetry diagnosis, real game or system install is claimed.
New extension tests, final review and CI are pending at this checkpoint.
