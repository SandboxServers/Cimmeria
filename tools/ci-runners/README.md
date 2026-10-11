# Self-hosted CI runners

Ten GitHub Actions runners for Cimmeria's Linux CI: five on node5, five on node2. Each
runs in its own container, paired with its own docker-in-docker daemon. Every
`test.yml` job routes to them through the `cimmeria-linux` label, except that fork PRs
always get GitHub-hosted runners. Windows and macOS jobs, and the other workflows' light
Linux jobs, stay on GitHub-hosted runners.

| File | What it is |
| --- | --- |
| `Dockerfile` | The runner image: Ubuntu 24.04 (as `ubuntu-latest`), the Actions runner, the pinned Rust toolchain, sccache, clang + mold, psql 17, lua5.1, gh, docker CLI |
| `compose.yml` | Five runner/dind pairs with their volumes, CPU weight and memory caps, and the `egress` proxy |
| `entrypoint.sh` | Registers on first start, restores the registration afterwards, writes the job environment, waits for dind |
| `hooks/job-started.sh`, `hooks/job-completed.sh` | Per-job disk report and cleanup |
| `egress_proxy.py`, `egress.Dockerfile` | The HTTPS proxy for the colo egress detour, node and exit roles |
| `start-exit.ps1` | Starts the exit proxy in Docker on the exit machine |
| `status.ps1` | Cache and disk report from every runner's job log |
| `deploy.ps1` | Copies the files to a node, mints a registration token with `gh`, builds and starts the stack |

## Colo egress

The colo cannot open TCP connections to Microsoft's `57.150.0.0/15` (Azure Storage),
which is where GitHub stores job logs, artifacts and test results. Outbound reaches
Microsoft, but the replies are lost on the upstream's return path (a ticket is open
with the colo provider). Without a workaround, every job finishes its steps and then
hangs on the log upload.

The workaround is a detour for that one block:

- Each node runs an `egress` container (`egress_proxy.py`, node role). Runners and
  their job steps get `HTTPS_PROXY=http://egress:3128` (with `NO_PROXY` for
  localhost). It connects most destinations directly, and sends CONNECTs to
  `57.0.0.0/8` to the exit proxy in the node's `.env` as `EGRESS_EXIT`
  (`deploy.ps1 -EgressExit host:port`).
- The exit proxy (`egress_proxy.py`, exit role, `start-exit.ps1`) runs in Docker on
  the operator's workstation, published only on its colo-VPN address. It only
  connects to `57.0.0.0/8`. A Windows firewall rule on the VPN adapter admits only the
  colo runner subnet:

  ```powershell
  New-NetFirewallRule -DisplayName 'Cimmeria CI egress proxy (colo runners)' -Direction Inbound `
      -Protocol TCP -LocalPort 3128 -LocalAddress <vpn-address> -RemoteAddress <runner-subnet> `
      -InterfaceAlias '<vpn-adapter>' -Action Allow -Profile Any
  ```

  Windows adds *block* rules for a program the first time it listens on a Public
  network, and they override this rule. That's why the exit runs in Docker, not as
  a bare `pythonw` task.

If the workstation is off, uploads fail and jobs hang again. Set the repository variable
`CI_LINUX_RUNNER=hosted` to fall back to GitHub-hosted runners. When the upstream is
fixed, redeploy with `-EgressExit ''` cleared in each node's `.env` and the egress proxy
connects everything directly. Check the colo path with:

```bash
ssh cimmeria-node2 'timeout 5 bash -c "</dev/tcp/57.150.86.161/443" && echo ok || echo blocked'
```

Each node's `docker compose logs egress` shows every CONNECT and the route it took.
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
- **One compile cache per node.** Each node's five runners share a Redis-backed sccache (compose.yml `cache`), so a crate any job compiled is a hit for every runner on that node. It lives on the runners' compose network: the colo networks are isolated from each other (the gateway's zone firewall blocks node-to-node traffic), and Docker drops traffic between its own bridge networks, so neither a cross-node cache nor a host-published port works. LRU at 12 GB on node5 and 24 GB on node2, no persistence.
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
