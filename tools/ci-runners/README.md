# Self-hosted CI runners

Fifteen GitHub Actions runners for Cimmeria's Linux CI: five each on node5, node2 and
node1. Each runs in its own container, paired with its own docker-in-docker daemon. The
`test.yml`, `wireclient.yml` and `pr-container.yml` jobs route to them through the
`cimmeria-linux` label. Fork PRs and Dependabot PRs always get GitHub-hosted runners.
Windows and macOS jobs, and the other workflows' light Linux jobs, stay on GitHub-hosted
runners.

| File | What it is |
| --- | --- |
| `Dockerfile` | The runner image: Ubuntu 24.04 (as `ubuntu-latest`), the Actions runner, the pinned Rust toolchain, sccache, clang + mold, psql 17, lua5.1, gh, docker CLI |
| `compose.yml` | A node's stack: five runner/dind pairs with their volumes, CPU weight and memory caps, plus the node's `cache` (sccache Redis), `buildkitd` and `egress` proxy |
| `entrypoint.sh` | Registers on first start, restores the registration afterwards, writes the job environment, waits for dind |
| `hooks/job-started.sh`, `hooks/job-completed.sh` | Per-job disk report, cache stats and cleanup |
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
  their job steps get `HTTPS_PROXY=http://egress:3128`, with `NO_PROXY` for localhost
  and the stack's own services. It connects most destinations directly, and sends
  CONNECTs to `57.0.0.0/8` to the exit proxy named in the node's `.env` as `EGRESS_EXIT`
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
fixed, clear `EGRESS_EXIT` in each node's `.env` and redeploy; the egress proxy then
connects everything directly. Check the colo path with:

```bash
ssh cimmeria-node2 'timeout 5 bash -c "</dev/tcp/57.150.86.161/443" && echo ok || echo blocked'
```

Each node's `docker compose logs egress` shows every CONNECT and the route it took.

## Cache and disk report

```powershell
pwsh tools/ci-runners/status.ps1   # per job: sccache hit rate, target/ size, free space; volume totals per node
```

## Deploy or update

```powershell
pwsh tools/ci-runners/deploy.ps1                                   # every node, cache host first
pwsh tools/ci-runners/deploy.ps1 -Nodes node2                      # one node
pwsh -c "& tools/ci-runners/deploy.ps1 -Nodes node5,node1"         # several
pwsh tools/ci-runners/deploy.ps1 -Nodes node2 -Build               # after changing an image
pwsh tools/ci-runners/deploy.ps1 -Nodes node5 -Down                # stop a node's stack
```

`deploy.ps1` needs repo admin through `gh` (for the registration token) and the
`cimmeria-colo`, `cimmeria-node2` and `cimmeria-node1` ssh aliases. A new runner reads
its one-hour registration token from the file `runner-token` in the compose directory,
mounted read-only; the script empties it a minute after the stack starts. It is not an
environment variable: a changed variable makes compose recreate every runner, which
kills running jobs. Each runner keeps its registration in its `state` volume, so
restarts and rebuilds don't re-register, and a deploy that changes nothing a runner
uses leaves it running. Images are built only when missing: a rebuild gives a new image
ID even when every layer is cached, which recreates every runner, so it takes `-Build`.
Changing the image, `.env` or a runner's compose entry recreates it, so do those when
the runners are idle.

The compose project lives in `/mnt/nvme/gha-runners` on node5 and `/var/lib/gha-runners`
on node2 and node1; volumes go under each node's docker root, on NVMe. Watchtower is
disabled on every container here, because restarting dind mid-job would kill the job.

To bump the runner, Rust or sccache version, edit the `ARG`s in the `Dockerfile` and
redeploy. The runner also updates itself between jobs. When `rust-toolchain.toml` moves,
the job's rust-toolchain step installs the new version; bump `RUST_VERSION` here so it
is baked in again.

## Why it's built this way

- **One dind per runner.** A job's `services:` Postgres publishes `5432:5432` into its own
  daemon's network namespace, which the runner shares, so five live-DB jobs on one node
  don't clash on a port. The runner reaches its daemon through a unix socket on a shared
  volume (dockerd is retiring non-TLS TCP). This does *not* isolate jobs from the host;
  see Security.
- **Production comes first on node5.** Runners and their daemons get a CPU weight of 256
  against the default 1024, so they use idle cores but yield to the game server and
  SigNoz. Memory caps are 16 GB per runner on node5 and node1, 20 GB on node2.
- **One compile cache for every node.** The cache host's Redis (`deploy.ps1 -CacheHost`,
  node2 by default) is every runner's sccache backend, so a crate any job compiled is a
  hit everywhere. The colo networks are isolated from each other by the gateway's zone
  firewall; a UniFi policy admits TCP 6379 from the other nodes' networks to the cache
  host only. The cache host's own runners use it on the compose network (`cache:6379`),
  because Docker drops traffic between its bridge networks. LRU at `CACHE_MAXMEMORY`
  (24 GB on node2), no persistence. If it is down, sccache counts every lookup as a miss
  and compiles; builds slow down but don't fail. `-CacheHost ''` gives each node its own.
- **One BuildKit daemon per node.** `pr-container.yml` builds through the node's
  `buildkitd` (setup-buildx `driver: remote`, `tcp://buildkitd:1234`), so image layers
  and the Dockerfile's cargo cache mounts (registry, `/build/target`) persist across jobs.
  GitHub's `type=gha` cache saves layers but never cache mounts, and exporting it from the
  colo would upload GBs through the egress detour, so self-hosted runs skip it. GC keeps
  10 GB, leaves 100 GB of disk free, and caps at 50 GB (node5) or 80 GB (node2, node1).
- **Parallel compiles per job.** `CARGO_BUILD_JOBS` comes from `deploy.ps1`: a node's `Jobs` if it has one, else its core count (never empty, which cargo rejects). Five jobs each running one compile per core oversubscribe a 16-core node five times over; on 2026-10-11 node1 spent about 19% of its CPU in the kernel under that load. node1 runs `Jobs = 8` as an experiment: compare its job durations with node2's and node5's in `status.ps1` before changing the others.
- **sccache instead of rust-cache.** Self-hosted jobs skip `Swatinem/rust-cache`
  (`if: runner.environment == 'github-hosted'`); downloading a multi-GB cache from GitHub
  is slower than a shared sccache hit. Every runner builds in the same path, so cache
  keys match across runners.

## Disk use per runner

| Item | Bound |
| --- | --- |
| The job's `target/` | Deleted by `job-completed.sh` after every job |
| Compile cache | In the cache host's Redis, not on the runner |
| Cargo registry and git checkouts | A few GB; a volume per runner |
| dind images | Stopped containers, networks, volumes and dangling images are pruned after every job. Tagged images (postgres:17.9) stay, unless the daemon's store passes 20 GB |
| `RUNNER_TEMP`, `/tmp` | `RUNNER_TEMP` is emptied after every job; `/tmp` entries older than an hour go, except the listener's .NET files |

Peak use is about 30 GB per runner during a build and a few GB at rest. To see the
real figures:

```bash
docker system df -v | grep gha-runners
```

## Routing and the kill switch

The routed jobs use:

```yaml
runs-on: ${{ (github.event.pull_request.head.repo.fork || github.actor == 'dependabot[bot]' || vars.CI_LINUX_RUNNER == 'hosted') && 'ubuntu-latest' || 'cimmeria-linux' }}
```

Set the repository variable `CI_LINUX_RUNNER` to `hosted` (Settings, Secrets and
variables, Actions, Variables) to send everything back to GitHub-hosted runners while
the nodes are down. GitHub has no automatic overflow between pools: a job targets a
label and waits for a runner that has it.

## Security

**A job on these runners can get root on the node.** Each runner's dind daemon runs
privileged, and the job controls that daemon, so a job can start its own privileged
container, which sees the host's devices and can mount its disks. On node5 that means
the production game server, SigNoz and their data. Treat every job as trusted code
running as root on the node.

Who can run code here:

- **Same-repo PRs and pushes:** people with write access, who can already push to the
  repo. Their code includes every dependency's `build.rs` and proc macros, so a
  compromised crate in a `Cargo.lock` change runs here too.
- **Fork PRs:** not routed here. A fork PR's workflow file comes from the PR, so the
  routing expression alone can't keep it off these machines. The repository setting that
  requires approval before workflows run for outside contributors (set to all outside
  contributors) is what does; a maintainer approving a fork PR's run checks that it
  doesn't change `runs-on`.
- **Dependabot PRs:** routed to GitHub-hosted runners (`github.actor ==
  'dependabot[bot]'`), because they exist to bring in new third-party code.

Runner state persists between jobs (work tree, cargo registry, dind images, the
container's writable layer), so a bad job can leave something behind for later ones.
The compile cache's Redis password is in every job's environment (sccache needs it), so
a job that dumps its environment into a public log leaks it. It only protects the cache.

Making jobs unable to reach the host needs a runtime that isolates nested containers
without `--privileged` (Sysbox, or rootless dind with user namespaces), or runners kept
off the production node. Neither is done yet.
