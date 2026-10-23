# CI iteration speed: measured design

> Type: Explanation · Audience: CI maintainers · Updated: 2026-10-23
> Scope: one PR from origin/main; preserve queue and all validation/release gates.

## Baseline

Read GitHub job/step timestamps, not only workflow wall time:

| Run | Job | Time / largest costs |
|---|---|---|
| 39955590955 | merge queue build + nextest + live DB | 392s: build 81s, build-lane Python tests 60s, live DB 42s, feature clippy 35s, DB binaries 35s |
| 39953187948 | preceding merge queue build + tests | 293s: build-lane Python tests 56s, build 51s, live DB 35s |
| 39913012340 | client-launch Windows | clippy 283s, then test 571s; dependency serializes independent checks |
| 39845015595 | lab Windows | tests 680s, clippy 378s; no CI debug override |
| 39953500833 | runtime tools | local lab tools 610s, runtime probe 193s; no Rust artifact restore |
| 39874737871 | server container release | 991s total, build/scan-load step 869s; Rust server compile 619.6s, probe 8s |
| 39657604307 | preceding container release | 824s total, build step 718s |
| 39953858643 | main container smoke | 482s, build 364s, persistence smoke 68s |

Run links use https://github.com/SandboxServers/Cimmeria/actions/runs/<id>.
These few recent runs are examples, not population percentiles. Hosted vs
self-hosted and source/cache differences prevent attributing all differences.
Root CI already disables debug info, cancels old PR runs, overlaps schema load
and compilation, and keeps normal/chaos and DB test tiers. Keep those choices.

## Selected implementation

1. Move stock-Python contract suites from the serial Rust job to an independent
   `contracts` job, under the exact same code/full condition. Include it in
   ci gate and nightly reporting. Leave seed/telemetry checks in the Rust job
   before compilation/seeding and preserve their order. No suite disappears.
2. Disable CI dev/test debug info for lab, client-launch, client-patches,
   client-telemetry and runtime-probe workflows. Local and release profiles stay.
   Start client-launch test/coverage and DLL build/test jobs independently of
   clippy: there is no artifact dependency. Keep all commands and job names.
3. Give hosted runtime tools main-written Rust caches and cancel superseded PR
   runs only. Consolidate the runtime probe's check/test/build target path to
   target/runtime-probe; keep player and lab telemetry in separate target dirs.
4. Preserve hosted container publication. Add pinned cargo-chef dependency
   cooking to Docker's builder: server and login-probe cooked separately with
   the same release/package options as their actual builds. Store dependency
   artifacts and registry in an ordinary image layer so GHA can export them.
   Seed empty/new-recipe cache mounts from that layer; keep warm self-hosted
   target mounts for subsequent builds. Restore real sources after cooking,
   then compile and copy the real server/probe. No dummy may reach runtime.
   Test a cold local build and a fresh-builder import before claiming reuse.

## Why this container change

Docker's GHA cache backend exports layers, not RUN cache mounts. Current
hosted releases therefore lose /build/target on each fresh runner despite a
layer cache hit. The observed release log compiled the whole server graph.
Keep runner/token boundaries; do not move write-token jobs onto shared hosts.
Do not merge server/probe Cargo invocations: that unifies test harness features.

Primary sources: [Docker cache mounts](https://docs.docker.com/build/ci/github-actions/cache/#cache-mounts)
and [cargo-chef](https://github.com/LukeMathWalker/cargo-chef). Pin the tool,
keep Cargo.lock, release optimization and target absolute paths consistent.

## Validation and exclusions

Semantic workflow tests must preserve every moved command, exact full/code
condition, fail-closed gate/nightly dependencies, PR-only cancellation,
main-only cache writes, isolated player/lab targets and default/chaos/DB tiers.
Reverting each protected change must fail a regression guard. Docker tests
must preserve separate actual server/probe builds, real-source restoration,
release identities, scan-before-push, provenance/SBOM and smoke-before-promotion.
Run existing Python suites and native CI; do not invoke Bash locally.

No merge queue/ruleset, fleet, token, production server or lab changes; no
container release/deployment authorized. The previously authorized launcher
release is independent. Larger policy changes (targeted tests, weaker gates,
new runner hardware or cache trust) are outside this PR. Record measured
post-change jobs and cold/warm differences, not promised savings.

Doc-update-map: CI checklist/build-system, runner operating reference, container
build/release docs, tool tests and committed project memory; no generated blocks.
