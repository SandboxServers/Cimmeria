# Self-hosted CI runners

Ten GitHub Actions runners for Cimmeria's Linux CI: five on node5, five on node2. Each
runs in its own container, paired with its own docker-in-docker daemon. Every
`test.yml` job routes to them through the `cimmeria-linux` label, except that fork PRs
always get GitHub-hosted runners. Windows and macOS jobs, and the other workflows' light
Linux jobs, stay on GitHub-hosted runners.

| File | What it is |
| --- | --- |
| `Dockerfile` | The runner image: Ubuntu 24.04 (as `ubuntu-latest`), the Actions runner, the pinned Rust toolchain, sccache, clang + mold, psql 17, lua5.1, gh, docker CLI |
| `compose.yml` | Five runner/dind pairs with their volumes, CPU weight and memory caps |
| `entrypoint.sh` | Registers on first start, restores the registration afterwards, writes the job environment, waits for dind |
| `hooks/job-started.sh`, `hooks/job-completed.sh` | Per-job disk report and cleanup |
| `status.ps1` | Cache and disk report from every runner's job log |
| `deploy.ps1` | Copies the files to a node, mints a registration token with `gh`, builds and starts the stack |

## Known blocker: colo egress to Azure 57.150.x

As of 2026-10-11 jobs on these runners run every step and then hang: the runner uploads step logs to Azure blob storage at 57.150.x, which times out from both nodes and works from other networks. The repository variable `CI_LINUX_RUNNER` is set to `hosted` until that is fixed; remove it to turn the runners back on. Check with:

```bash
ssh cimmeria-node2 'timeout 5 bash -c \"</dev/tcp/57.150.86.161/443\" && echo ok || echo blocked'
```

## Cache and disk report

```powershell
pwsh tools/ci-runners/status.ps1   # per job: sccache hit rate, target/ size, sccache size, free space; volume totals per node
```

## Deploy or update

```powershell
pwsh tools/ci-runners/deploy.ps1                 # both nodes
pwsh tools/ci-runners/deploy.ps1 -Nodes node2    # one node
pwsh tools/ci-runners/deploy.ps1 -Nodes node5 -Down
```

`deploy.ps1` needs repo admin through `gh` (for the registration token) and the
`cimmeria-colo` and `cimmeria-node2` ssh aliases. The token lives an hour and is passed
only in the ssh command's environment. Each runner keeps its registration in its `state`
volume, so restarts and rebuilds don't re-register. The compose project lives in
`/mnt/nvme/gha-runners` on node5 and `/var/lib/gha-runners` on node2; volumes go under
each node's docker root (both on NVMe). Watchtower is disabled on every container here,
because restarting dind mid-job would kill the job.

To bump the runner, Rust or sccache version, edit the `ARG`s in the `Dockerfile` and
redeploy. The runner also updates itself between jobs. When `rust-toolchain.toml` moves,
the job's rust-toolchain step installs the new version; bump `RUST_VERSION` here so it
is baked in again.

## Why it's built this way

- **One dind per runner.** A job's `services:` Postgres publishes `5432:5432` into its own
  daemon's network namespace, which the runner shares. Five live-DB jobs on one node
  don't clash on a port, and no job can reach the host's docker socket, which on node5
  controls production. The runner reaches its daemon through a unix socket on a shared
  volume (dockerd is retiring non-TLS TCP).
- **Production comes first on node5.** Runners and their daemons get a CPU weight of 256
  against the default 1024, so they use idle cores but yield to the game server and
  SigNoz. Memory caps are 16 GB per runner on node5 and 20 GB on node2.
- **sccache instead of rust-cache.** Self-hosted jobs skip `Swatinem/rust-cache`
  (`if: runner.environment == 'github-hosted'`); downloading a multi-GB cache from GitHub
  is slower than a local sccache hit. Every runner builds in the same path, so its cache
  keys are stable.

## Disk use per runner

| Item | Bound |
| --- | --- |
| The job's `target/` | Deleted by `job-completed.sh` after every job |
| sccache | `SCCACHE_CACHE_SIZE`, 20 GB (LRU) |
| Cargo registry and git checkouts | A few GB; a volume per runner |
| dind images | Unused images older than a week are pruned after each job; stopped containers, networks and volumes after every job |
| `RUNNER_TEMP`, `/tmp` | Emptied after every job |

Peak use is about 45 GB per runner during a build and about 25 GB at rest, so about
250 GB per node. That fits the free NVMe on both nodes with room to spare. To see the
real figures:

```bash
docker system df -v | grep gha-runners
```

## Routing and the kill switch

`test.yml` jobs use:

```yaml
runs-on: ${{ (github.event.pull_request.head.repo.fork || vars.CI_LINUX_RUNNER == 'hosted') && 'ubuntu-latest' || 'cimmeria-linux' }}
```

Set the repository variable `CI_LINUX_RUNNER` to `hosted` (Settings, Secrets and
variables, Actions, Variables) to send everything back to GitHub-hosted runners while
the nodes are down. GitHub has no automatic overflow between pools: a job targets a
label and waits for a runner that has it.

## Security

The repository is public. A fork PR's workflow file comes from the PR, so the routing
expression alone can't keep fork code off these machines. What does is the repository
setting that requires approval before workflows run for outside contributors (set to
all outside contributors). A maintainer approving a fork PR's run checks that it doesn't
change `runs-on`.
