# Desktop updater parity research

> **Type:** Reference
> **Audience:** Launcher implementation and release owners
> **Last updated:** 2026-10-04
> **Local evidence:** `1193bc46d`; read-only source audit, no runtime validation
> **Companions:** [Implementation plan](../launcher-implementation-plan.md), [migration audit](migration-audit.md)

## Existing contract and the parity boundary

The old updater is **checksum-verified, not release-signature-verified**. It
fetches the first 100 GitHub releases, filters published non-prerelease
`launcher-YYYYMMDD-<sha>` releases with the exact executable and `.sha256`
assets, and sorts by publication time. Its HTTPS download policy allowlists
GitHub and the two GitHub asset hosts, including redirects. It resumes an
executable-adjacent partial download and checks advertised size plus SHA-256
before swapping. Source: `crates/launcher/src/self_update/releases.rs` and
`download.rs`; publishing contract: `.github/workflows/launcher-release.yml`.
The signed **game manifest** is a separate trust mechanism.

`build_info.rs` and `version.rs` make unstamped builds development builds: no
updates and no minimum-launcher gate. Release ordering uses differing tags,
nondecreasing tag date, and publication time after the compiled build epoch.
The release workflow serializes jobs. The old `min_launcher` gate blocks
Install and Launch when the required tag is known newer; malformed minima
warn without blocking, and unknown same-day ordering lets the build through.
Preserving that exception is parity, not strict minimum enforcement.

`swap.rs` renames the old executable aside and restores it on replacement
failure. `handoff.rs` restores it if spawning the replacement fails, otherwise
releases the lock and exits immediately. New startup waits for the old owner,
with narrowly checked old-process termination. This is executable replacement,
not a Windows package installer and not an application-bundle replacement.

At the audited commit, `desktop/shell/tauri.conf.json` declares `0.1.0` and no
updater key/endpoints/artifacts. `desktop/shell/Cargo.toml` has no updater plugin.
`desktop/engine/src/catalog/` retains the verified game manifest, including
`min_launcher`, but no desktop minimum check exists. Configuration import does
not close any of these gaps.

## Tauri contracts verified against primary sources

The [official updater guide](https://v2.tauri.app/plugin/updater/) requires an
embedded public key and signed updater artifacts. Generate a dedicated updater
key; release jobs use `TAURI_SIGNING_PRIVATE_KEY` and its optional password.
Do not substitute the game's raw Ed25519 key. Enable
`bundle.createUpdaterArtifacts: true`. A static feed requires SemVer `version`
and platform entries containing artifact `url` and literal `.sig` contents.
A dynamic endpoint returns 204 for no update or 200 with version, URL and
signature. Platform keys include `darwin-aarch64`, `darwin-x86_64` and
`windows-x86_64`. Mac updater artifacts are `.app.tar.gz`; Windows uses NSIS
`.exe` or MSI packages. This is independent of OS code-signing/notarization.

Source inspection is pinned to released `updater-v2.13.1`, commit
`e51128438011755f9e7277bad29b8c0978cf281c`, not an assumed latest API. Adoption
must also pin a compatible Tauri CLI and verify its emitted signatures.

- [Signature verification](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs#L1634)
  decodes base64 Minisign material and verifies artifact bytes. With
  `requireSignedVersion: true`, it also requires a signed version in the trusted
  comment matching the announced version. The
  [configuration](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/config.rs#L117)
  explains why this matters: the feed itself is unsigned; otherwise an inflated
  version can point at a legitimately signed old artifact. Use this requirement
  for a new desktop channel; do not assume all CLI versions emit that comment.
- [Download](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs#L774)
  accumulates bytes in memory, with no built-in partial-file resume in this
  method. Its download-finished callback runs **before** signature verification.
  Only successful return means verified bytes. `install(bytes)` does not
  reverify them: keep verified bytes native-owned and inaccessible to renderer
  substitution. The inspected transport has no explicit total-byte cap here.
- [Request configuration](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs#L429)
  permits a native client policy. Preserve HTTPS and redirect host restrictions
  deliberately, plus endpoint/artifact URL checks and timeouts; defaults alone
  do not reproduce the old allowlist. Do not enable insecure transport in a
  release configuration. A bounded/resumable staging adapter remains a separate
  parity decision, especially if bundled runtime packages are large.
- [Windows install](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs#L938)
  extracts an installer, calls `on_before_exit`, invokes `ShellExecuteW`, then
  exits on successful launch. That does not establish installer completion.
  The hook precedes installer launch, so releasing ownership there can leave a
  live unlocked launcher if launch fails. Persist the attempt before calling
  install; keep native ownership until process exit or restore it on failure.
- [Mac install](https://github.com/tauri-apps/plugins-workspace/blob/e51128438011755f9e7277bad29b8c0978cf281c/plugins/updater/src/updater.rs#L1380)
  extracts a bundle, moves the installed bundle to temporary backup, then moves
  the new bundle into place. The inspected error path does not restore the old
  bundle after the final rename fails. The permission fallback uses privileged
  replacement. These paths do not reproduce the legacy swap/spawn-failure rollback contract;
  neither old nor new updater proves application health after startup. Mac also
  needs a subsequent relaunch. Native tests must include unwritable and
  cross-volume paths.

## Proposed interface and version policy

These are implementation recommendations, not existing interfaces.

Keep one native desktop updater owner. The legacy executable continues to own
only its own updater; never offer a Tauri installer under the old
`sgw-launcher-<legacy-tag>.exe` asset contract. Use a distinct desktop release
family/feed. Separate an ordinary monotonically increasing desktop SemVer from
legacy compatibility identity: preserve a compiled compatibility tag and build
epoch for `min_launcher` evaluation. Do not reinterpret a date/hash tag as
SemVer or use build metadata to order releases. A release manifest should record
desktop version, source revision, compatibility stamp, platform and artifact
digests; validate platform package version limits before release publication.

Proposed native commands:

| Command | Contract |
|---|---|
| `check_launcher_update` | Native configured endpoint only; yields opaque offer ID, version and bounded display notes; distinguishes disabled, up-to-date, available and failed. |
| `prepare_launcher_update(offer_id, operation_id, expected_revision)` | Admit against the existing durable operation owner; download and verify into native ownership; no URL, key or arbitrary bytes accepted from JS. |
| `apply_launcher_update(operation_id, expected_revision)` | Recheck no game/install/repair/runtime mutation, persist handoff intent, then platform install. Reject stale or already-consumed offers. |
| `launcher_update_snapshot` | Revisioned phase and allowlisted failure code; after reopen reconcile expected version with running embedded version. |

Phases should distinguish checking, downloading, verifying, ready, installing,
restart-required, reconciliation-required and failed. Download completion is not
verification; installer launch is not successful update. Effect sequences these
commands and subscriptions. Stopping a fiber is not native cancellation. Apply
minimum compatibility gates in native Install/Launch admission using the signed
manifest, including cached evidence on offline launch; a disabled button alone
is insufficient. Repair and recovery must not become inaccessible by accident.

## Bounded next worker packet

Implement **native compatibility policy and minimum admission gates**, not the
whole updater. Own new `desktop/engine/src/launcher_compatibility/` policy/tests
and narrow native Install/Launch admission changes; coordinate any shared
contract edits with their owner. Keep network discovery and executable mutation
out of this first packet. Inject compiled release identity and known legacy
release ordering into the policy, port the old minimum semantics explicitly,
and expose typed minimum status. A standalone SemVer/legacy mapping record and
fixtures make the subsequent signed-feed adapter reviewable.

Per `TESTING.md`, use unit guards for ordering and malformed/dev/unknown cases,
native temp-directory admission tests for signed/cached minimum enforcement,
and concurrency guards for mutation ownership where touched. Cover different
and same-day releases, absent/offline ordering, stale evidence, bypass attempts
through direct native calls, and no durable mutation on blocked admission. Add
Effect logic REPL UAT when visible gate wiring lands, including persisted reopen
and a clear statement that OS installation/relaunch was not covered.

Then split signed local-feed download/verification from platform apply/recovery.
Use test-only keys and local fixtures first. Production key custody, release
feed publication, OS signing and native Windows/Mac update/relaunch recovery
proof remain release gates; this research creates no keys and authorizes no
publication. Do not claim resumed downloads or rollback parity until tested.

## Validation limits and integration notes

No builds, executable updates, runtime tests, UI UAT, production requests,
telemetry or deployment were performed. Sources were read via `gh api` and the
official guide. The worker owns only this note and a unique project-memory
entry; the coordinator must add index links and correct any earlier description
of legacy checksum verification as signed-update verification during integration.
