---
name: reference-self-hosted-runners
description: 2026-10-10 ten dockerized cimmeria-linux runners (5 node5, 5 node2), one dind each; dockerd-entrypoint TCP trap; fork-PR approval set to all outside contributors
metadata:
  type: reference
---

- Ten repo-level runners, `node5-r1..5` and `node2-r1..5`, label `cimmeria-linux`, deployed by `tools/ci-runners/deploy.ps1` (PR #1410). `test.yml` and `wireclient.yml` route to them; fork PRs and repo variable `CI_LINUX_RUNNER=hosted` fall back to `ubuntu-latest`. GitHub has no hosted-to-self-hosted overflow; a job waits for its label.
- Each runner shares its own dind daemon's network namespace, so `services:` ports such as `5432:5432` don't clash. The link is a unix socket on a shared volume (`/run/dind/docker.sock`, `--group=1001`, the image's fixed runner gid).
- Trap: `docker:*-dind`'s `dockerd-entrypoint.sh` always adds a `tcp://0.0.0.0:2375` listener when TLS is off, even with your own `--host`, and the daemon crash-loops on "port is already allocated". Use `entrypoint: ["dind", "dockerd", ...]` to skip it.
- A named volume first mounted by the root-only dind sidecar comes up root-owned, so the runner entrypoint `sudo chown`s its mounts.
- Since 2026-10-10 the repo's fork-PR workflow approval policy is `all_external_contributors` (was `first_time_contributors`): a fork PR's workflow file sets its own `runs-on`, so approval is the only thing keeping fork code off node5.
- The shell guard hook refuses any `cargo` word, even inside a remote `ssh` probe; `cargo-clippy --version` or a `# guard-ok` suffix gets past it.
- **Blocker found 2026-10-11:** jobs run but never finish. The runner's step-log upload goes to Azure blob storage at 57.150.x, and TCP to 57.150.x times out from both colo nodes while it works from the home network. Traceroute reaches Microsoft's backbone (104.44.x), so it isn't the UDM; the likely cause is Microsoft filtering the colo range (cf. the Spamhaus listing in [[reference_agent_board]]). Repo variable `CI_LINUX_RUNNER=hosted` was set as the fallback until egress works.
