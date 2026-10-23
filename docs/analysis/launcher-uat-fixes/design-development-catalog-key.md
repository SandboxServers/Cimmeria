# Development-build archive verification diagnosis

> Type: Explanation · Audience: launcher contributors · Date: 2026-10-23
> Companions: [Catalog trust and build recipe](../../../crates/launcher/desktop/README.md#build-against-released-game-content), [archive contract](../../../crates/launcher/desktop/docs/seed-archive.md#public-catalog-preflight), [ledger](README.md#development-catalog-key-follow-up-2026-10-23)

The owner reports a newly downloaded RAR refused with both “release could not be
verified” and “could not reach the signed release”, then confirms this is a
current-main development build. `launcher-core::manifest::MANIFEST_SIGNING_PUBKEY`
uses the public development test key in debug builds when the compile-time
LAUNCHER_MANIFEST_PUBKEY_HEX value is absent/empty. That key cannot authenticate
the published content manifest. The archive selector collapses every catalog
error into release_unavailable, so the displayed network diagnosis is incorrect.
An aggregate query found no matching forwarded release_fetch_failed events in
the last two hours; no laptop-specific telemetry diagnosis is claimed.

Keep all signature and archive verification. Add a distinct development-key
failure at the fixed public-catalog fetch boundary, before network access, when
a debug build is using the default test key. Local signed fixtures continue to
use verify_release unchanged. A debug build explicitly compiled with the release
public verification key remains able to fetch released content. Missing release
keys remain a distinct error. Log those preflight failures with closed reasons.

Preserve catalog error distinctions across the archive IPC: network remains
release_unavailable, missing/default-development key gets actionable build-key
copy, and signature/body/schema rejection reads as verification failure. The
install area also recognizes the development-key code. Direct players to the
published launcher ZIP or developers to rebuilding with the public release key;
never ask for the private signing key or permit unverified RARs. No public key
value, secret storage, release signing, archive bytes or Plan fields change.

Validate native policy/mappings, error telemetry and DOM copy for Install and
archive selection. Prove a reverted policy/mapping fails a regression. Extend
the existing seed-archive JS logic UAT, run affected tests/builds and native
checks through the lane, then review with the PhysX change in the same PR.
No real game, laptop access, system installer or lab is required. The owner can
use launcher-current's published ZIP as the immediate workaround.
