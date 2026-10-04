---
applyTo: ".github/workflows/**,.github/actions/**"
---

# CI workflow review rules

- Do not use `pull_request_target` for jobs that check out or run PR code.
- Never interpolate untrusted input (`github.event.pull_request.title`,
  `.body`, `.head_ref`, issue or comment text) straight into a `run:` script.
  Pass it through `env:` and quote it.
- Declare the narrowest `permissions:` the job needs. Most workflows here set
  `permissions: {}` at the top and grant `contents: read` (or more) per job;
  flag a new workflow with no `permissions:` block.
- Install Rust through `.github/actions/rust-toolchain`, which reads
  `rust-toolchain.toml`, so CI's clippy is the pinned one. A hard-coded
  toolchain version is a finding.
- Workspace-wide cargo steps use the same seven `--exclude` flags as
  `.github/workflows/test.yml`. A new GUI app or Windows-only cdylib must be
  added to every one of those lists, and to the list in `CLAUDE.md`.
- Pin a newly added third-party action (anything outside `actions/*` and
  `github/*`) to a commit SHA. Some existing steps are tag-pinned
  (`Swatinem/rust-cache@v2`, `taiki-e/install-action`); flag those only when
  the PR touches the line.
- Uploaded artefacts must not contain secrets, `.env` files or signing keys.
- A new required job, or a change to what a job gates, needs the matching
  row in `docs/agents/pre-pr-checks.md`.
