# Container build inputs and caches

> **Type:** Reference · **Audience:** CI maintainers · **Updated:** 2026-10-23
> **Companions:** [operator runbook](../docs/operations/container.md), [build system](../docs/architecture/build-system.md#14-shorten-waits-without-reducing-validation), [CI runner reference](../tools/ci-runners/README.md), [measured design](../docs/analysis/ci-iteration-speed.md)

The canonical deployment and release procedure is the operator runbook. This directory contains the server Dockerfile, compose overlays, database boot/update scripts and s6 service definitions. Pushing a PR or merging it does not authorize publishing or deploying an image.

## Rust dependency reuse

`Dockerfile` pins cargo-chef to 0.1.78. It prepares a dependency recipe from the workspace, then cooks server and login-probe dependencies separately with the same package/release options as their final builds. Keeping those invocations separate avoids unifying the probe's test-harness features into the server.

Cooked target artifacts and the Cargo registry are retained in ordinary image layers. Docker's [GHA backend does not preserve RUN cache mounts](https://docs.docker.com/build/ci/github-actions/cache/#cache-mounts), so a hosted runner cannot recover compiled dependencies from a mount alone. Layers provide the portable seed; persistent mounts retain warm self-hosted builds. Empty mounts, or mounts for a new recipe, are seeded from the dependency layer.

Real workspace sources are restored after cooking. Actual release builds produce both binaries before runtime/export stages copy them; a cooked placeholder binary must never reach either artifact. The toolchain, lockfile, native dependencies and absolute Cargo target paths must agree between cooking and final compilation. Source changes should reuse dependency layers; manifest, lockfile or toolchain changes may invalidate them.

## Validation boundary

`pr-container.yml` still builds without pushing and runs persistence/login smoke checks. `release-container.yml` still publishes from hosted runners with its scan-before-push, provenance/SBOM and smoke-before-promotion gates. This cache change does not authorize a release or alter those gates.

A warm local BuildKit daemon is insufficient evidence of hosted reuse. Validate both a cold build and an imported layer cache on a fresh builder, then check that real binaries are built and exported. Record runner, revision, cache state and job/step duration with any speed claim. Baseline run IDs and the validation plan are in the measured design; post-change release timings require a separately authorized real release.
