# Self-hosted CI runners

> **Type:** Reference · **Audience:** CI operators · **Updated:** 2026-10-23
> **Companions:** [build system](../../docs/architecture/build-system.md), [pre-PR checks](../../docs/agents/pre-pr-checks.md), [container builds](../../docker/README.md)

Fifteen GitHub Actions runners for Cimmeria's Linux CI: five each on node5, node2 and
node1. Each runs in its own container, paired with its own docker-in-docker daemon. The
`test.yml`, `wireclient.yml` and `pr-container.yml` jobs route to them through the
`cimmeria-linux` label. Fork PRs and Dependabot PRs always get GitHub-hosted runners.
Three Windows runners on node3 take the launcher workflows' Windows jobs (see
[Windows pool](#windows-pool)). macOS jobs, the other Windows jobs and the other
workflows' light Linux jobs stay on GitHub-hosted runners.

| File | What it is |
| --- | --- |
| `Dockerfile` | The runner image: Ubuntu 24.04 (as `ubuntu-latest`), the Actions runner, the pinned Rust toolchain, sccache, clang + mold, psql 17, lua5.1, gh, docker CLI |
| `compose.yml` | A node's stack: five build runner/dind pairs with their volumes, CPU weight and memory caps, two quick runners, plus the node's `cache` (sccache Redis), `buildkitd` and `egress` proxy |
| `entrypoint.sh` | Registers on first start, restores the registration afterwards, writes the job environment, waits for dind |
| `hooks/job-started.sh`, `hooks/job-completed.sh` | Per-job disk report, cache stats and cleanup |
| `egress_proxy.py`, `egress.Dockerfile` | The HTTPS proxy for the colo egress detour, node and exit roles |
| `start-exit.ps1` | Starts the exit proxy in Docker on the exit machine |
| `status.ps1` | Cache and disk report from every runner's job log |
| `deploy.ps1` | Copies the files to a node, mints a registration token with `gh`, builds and starts the stack |
| `windows/setup.ps1` | Run on node3 as admin: the Windows toolchain, the egress proxy task, the runner account and three runner services |
| `windows/deploy.ps1` | Copies the Windows files to node3, hands over the token and cache password, runs `setup.ps1` there |
| `windows/hooks/job-started.ps1`, `windows/hooks/job-completed.ps1` | The Windows hooks: compile-cache choice, per-job cache stats, cleanup |

## Colo egress

The colo cannot open TCP connections to Microsoft's `57.150.0.0/15` or to some of
`52.239.0.0/16` (both Azure Storage), which is where GitHub stores job logs, artifacts
and test results. Outbound reaches
Microsoft, but the replies are lost on the upstream's return path (a ticket is open
with the colo provider). Without a workaround, every job finishes its steps and then
hangs on the log upload.

The workaround is a detour for that one block:

- Each node runs an `egress` container (`egress_proxy.py`, node role). Runners and
  their job steps get `HTTPS_PROXY=http://egress:3128`, with `NO_PROXY` for localhost
  and the stack's own services. It connects most destinations directly, and sends
  CONNECTs to `57.0.0.0/8` and `52.239.0.0/16` to the exit proxy named in the node's
  `.env` as `EGRESS_EXIT` (`deploy.ps1 -EgressExit host:port`). A host goes through the
  exit if any of its addresses is in those ranges: GitHub's results accounts
  `productionresultssa9`, `16`, `17` and `18` share the stamp `bn9prdstrz04a`, whose
  DNS rotates `20.150.82.228`, `20.150.88.228` and `52.239.172.36`, and only the last
  is unreachable. Before `52.239.0.0/16` was added (2026-10-11), every connect that
  drew `52.239.172.36` first timed out after 20 s (about 450 a day across the three
  nodes) and failed the artifact upload it carried.
- The exit proxy (`egress_proxy.py`, exit role, `start-exit.ps1`) runs in Docker on
  the operator's workstation, published only on its colo-VPN address. It only
  connects to those ranges. A Windows firewall rule on the VPN adapter admits only the
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
pwsh tools/ci-runners/status.ps1 -Nodes node3      # the Windows pool
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
  The Dockerfile also cooks dependencies into ordinary layers for fresh hosted
  builders, then seeds empty/new-recipe mounts from those layers. GitHub's
  `type=gha` cache saves layers but never cache mounts, and exporting it from the
  colo would upload GBs through the egress detour, so self-hosted runs skip it. GC keeps
  10 GB, leaves 100 GB of disk free, and caps at 50 GB (node5) or 80 GB (node2, node1).
- **Parallel compiles per job.** `CARGO_BUILD_JOBS` comes from `deploy.ps1`: a node's `Jobs` if it has one, else its core count (never empty, which cargo rejects). Five jobs each running one compile per core oversubscribe a 16-core node five times over; on 2026-10-11 node1 spent about 19% of its CPU in the kernel under that load. Every node runs `Jobs = 8`: under the same load node1's kernel share fell from 15% to 7% with it, while node2 at 16 compiles per job stayed at 16%.
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

## Routing, overflow and the kill switches

Three pools:

| Pool | Jobs | Where |
| --- | --- | --- |
| `cimmeria-linux` | Builds: clippy, contracts, build-and-test, coverage, wireclient e2e, pr-container, client-launch coverage, spec-lint | Five per node, plus three on the operator's desktop |
| `cimmeria-linux-quick` | Seconds-long jobs: `changes`, `fmt` (ci, client-patches, client-telemetry), `overlay-lua`, markdownlint, spec-touch, agent-skills, figure-sources, figure-style, and the `main-red` reporters (`test.yml`'s `nightly-report`, `pr-container.yml`'s `report`) | Two per node: compose `quick1`/`quick2`, no dind, 2 CPUs and 4 GB each |
| `ubuntu-latest` | Fork PRs, Dependabot PRs, overflow, anything sent there by a variable, and jobs that publish or need tools the image lacks: release-container, regen-docs, prune-container, the release dispatchers, bootstrap-lint (PowerShell), senpai-review (opt-in, per `senpai` label) | GitHub-hosted |

GitHub has no overflow between pools (a job waits for a runner with its label), so
`test.yml`'s `changes` job decides: it counts idle, online `cimmeria-linux` runners
with the `RUNNER_STATUS_TOKEN` secret (a fine-grained token with Administration: read
on this repo only), and when fewer than `CI_OVERFLOW_MIN_IDLE` (default 2) are idle it
sends that run's build jobs (clippy, build-and-test, coverage) to GitHub-hosted runners.
Without the secret, or if the call fails, they stay self-hosted.

Repository variables (Settings, Secrets and variables, Actions, Variables):

- `CI_LINUX_RUNNER=hosted`: every routed job to GitHub-hosted runners, e.g. while the nodes are down.
- `CI_HOSTED_JOBS`: comma-separated job ids (e.g. `coverage-workspace,e2e`) that always run GitHub-hosted. It matches the job id, not the workflow: `lint` covers markdownlint's, figure-style's and spec-lint's `lint` jobs, and `fmt` covers every `fmt` job.
- `CI_OVERFLOW_MIN_IDLE`: the idle-runner threshold for overflow; `0` turns overflow off. Set to 0 on 2026-10-11: GitHub's hosted pool (shared with the Windows jobs) was as full as ours, so overflowed build jobs queued there while our runners sat idle. The decision is made once per run and never revisited. The merge-queue change of the same day leaves it at 0.

Since 2026-10-11 the pool's load is shaped by `main`'s merge queue ([pre-pr-checks.md § Merge queue](../../docs/agents/pre-pr-checks.md#merge-queue)): a PR push runs `fmt`, `clippy` and the quick jobs only; `build-and-test` and the independent Python `contracts` job run in the merge queue (`merge_group`), nightly at 09:23 UTC, and on PRs labelled `full-ci`; coverage runs nightly; and `pr-container` runs on PRs only for container files and otherwise on `main` after a merge, coalesced to the newest commit. Before that, in the two weeks to 2026-10-11, the pool's p90 queue wait was 19–23 minutes against a 7-minute build + nextest, branches averaged about ten CI runs each, and `pr-container` used more runner time than the gating job.

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

## Windows pool

Three runners on node3, `node3-win-1` to `node3-win-3`, with the label `cimmeria-windows`.
node3 is a Windows Server 2022 Hyper-V host that also runs other people's VMs, so the
pool shares the machine and never touches Hyper-V, the VMs or the network settings.
There are no containers: each runner is a Windows service running as a non-admin
account, with its own directory, toolchain and sccache server.

| Workflow | Jobs on the pool |
| --- | --- |
| `launcher-desktop.yml` | `native-engine (windows-latest)` (the Windows matrix leg), `desktop release build dry run (Windows)` |
| `client-launch.yml` | `cargo clippy -D warnings (client launch, Windows)`, `cargo nextest (client launch, Windows)` |
| `launcher-runtime-probe.yml` | `runtime-probe`, `local-lab-tools` |

`launcher-release.yml` stays on GitHub-hosted runners. Every job on the pool can write
to the shared compile cache, its own cargo registry and its toolchain, so a bad job could
plant an artifact that a later build reuses. A signed release must never be built from
that, so releases build from scratch on a clean hosted machine. The `client-*-build`,
`client-dll-boot`, `lab` and `build-lane-scripts` workflows' Windows jobs are candidates
for the pool too; they are not moved yet.

### Routing and the kill switch

Each moved job has

```yaml
runs-on: ${{ (github.event.pull_request.head.repo.fork || github.actor == 'dependabot[bot]' || vars.CI_WINDOWS_RUNNER == 'hosted') && 'windows-latest' || 'cimmeria-windows' }}
```

The `native-engine` matrix keeps `os: [macos-latest, windows-latest]` and maps only the
Windows leg in `runs-on`, because `tools/build-lane/ship.py` waits on the check names
`native-engine (windows-latest)` and `native-engine (macos-latest)`. Self-hosted jobs
skip `Swatinem/rust-cache` (`if: runner.environment == 'github-hosted'`).

Set the repository variable `CI_WINDOWS_RUNNER=hosted` to send every one of these jobs
back to `windows-latest`, for example while node3 or the egress exit is down. There is
no overflow: when all three runners are busy, a job waits for one.

Hosted runtime-probe and local-lab-tools jobs restore main-written Rust caches;
self-hosted jobs continue to use sccache. Superseded runtime-tool PR runs cancel;
main runs finish to populate caches. The runtime probe uses `target/runtime-probe`,
while player and lab-bridge telemetry remain isolated. CI dev/test debug overrides
do not alter shipped release builds. Client-launch tests and client DLL build/test
jobs no longer wait for independent clippy jobs.

### What is on node3

| Where | What |
| --- | --- |
| Machine-wide | VS 2022 Build Tools (MSVC x64/x86, Windows 11 SDK 26100), Python 3.12.10 for all users, Node 24.21.0, the WebView2 Runtime, Git for Windows (already there) |
| `F:\gha\deploy` | `setup.ps1`, the download cache, `node.json` (cache endpoint and password, egress exit) and the transient `secrets\`; SYSTEM and Administrators only |
| `F:\gha\tools\sccache` | sccache 0.18.0, the same version as the Linux image |
| `F:\gha\hooks`, `F:\gha\shared` | The job hooks and the cache settings they read; read-only to the runner account |
| `F:\gha\egress` | `egress_proxy.py` and its rotated log |
| `F:\gha\rN` | One runner: `runner\` (the Actions runner and its read-only `.env`), `work\`, `rust\` (its rustup and cargo homes), `sccache\`, `state\jobs.jsonl`, `tmp\`, `lane\` |

Versions are pinned in `setup.ps1` (`$Pins`), checked against the publishers' SHA-256
where one exists (the runner, rustup-init, sccache, Node) and by Authenticode signer
otherwise (VS Build Tools, WebView2, and Python, whose MD5 is also checked against
python.org). The Rust version comes from `rust-toolchain.toml`.

### Why it's built this way

- **A non-admin account.** The services run as the local user `gha-runner`, so a job
  is not admin on a host that runs other people's VMs. Its password is random, given to
  `config.cmd` and the services, and never stored; `setup.ps1` resets it only when a
  runner needs registering, and updates every runner service when it does.
- **A toolchain per runner.** The rust-toolchain action runs `rustup toolchain install`
  and `rustup default` in every job, and rustup does not lock against a second rustup
  in the same home. The cargo registry is per runner too.
- **Git's `bin` first on PATH.** `C:\Windows\System32` holds WSL's `bash.exe`, and a
  `shell: bash` step runs the first `bash` on PATH. Each runner's `.env` sets the full
  PATH: its cargo `bin`, Git, Python, Node, sccache, then the machine PATH.
- **Builds yield to the VMs.** Image File Execution Options give `RunnerService.exe`,
  `Runner.Listener.exe` and `Runner.Worker.exe` below-normal CPU priority, and a child of
  a below-normal process starts below-normal, so every compile does.
- **One sccache server per runner.** Each runner has its own `SCCACHE_SERVER_PORT`
  (4227 to 4229). On the default port the three would share one server, with the first
  job's settings, and one job's end-of-job `--stop-server` would kill another's builds.
- **Cache hits are per runner.** sccache's Rust keys include absolute paths (the work
  tree, `CARGO_MANIFEST_DIR`, the `target\` dependency paths), and three runners on one
  host can't share one set of paths, so a runner gets hits only from its own earlier
  jobs. `SCCACHE_BASEDIRS` does not change that in sccache 0.18 (tested on node3,
  2026-10-11: no path form gave a hit from a second root). On the Linux pool every
  runner builds in the same container path, so there hits cross runners and nodes.
- **The compile cache is chosen per job.** sccache 0.18 does not fail open when its
  Redis drops packets: the startup storage check outlasts the client's 10-second wait,
  and the compile that started the server fails. So the endpoint is not in `.env`.
  `job-started.ps1` connects to the cache host's Redis (two seconds at most) and writes
  `SCCACHE_CONF` with the Redis backend only when it answers; otherwise sccache uses the
  runner's `SCCACHE_DIR`. The job log says which, without the address.
- **One build lane per runner.** The lane's slot semaphore lives under `LANE_ROOT`,
  which defaults to the account's `%LOCALAPPDATA%`, shared by all three runners. Each
  runner sets its own. The lane keeps any `RUSTC_WRAPPER` it is given, so sccache runs
  through it unchanged.
- **Parallel compiles per job.** `CARGO_BUILD_JOBS=10`: node3 has 32 hardware threads
  for three runners (`deploy.ps1 -Jobs`).
- **The colo egress detour, on the host.** A scheduled task, `cimmeria-ci-egress`, runs
  `egress_proxy.py` in the node role on `127.0.0.1:3128` as LOCAL SERVICE, at boot and
  every five minutes (a no-op while it runs), so a crash is back within minutes. The
  listener reads the proxy from its service's environment (it uploads the logs), job
  steps from `.env`. `NO_PROXY` covers localhost and the cache host, and for job steps
  also `github.com` and its subdomains. Through a CONNECT proxy git falls back to
  HTTP/1.1, and on node3 a git fetch over HTTP/1.1 stalls once the pack's response
  headers arrive, direct or proxied (plain curl over HTTP/1.1 works, and so does git
  over HTTP/2). Going direct lets git use HTTP/2, so `actions/checkout` works.
- **The shared cache needs a firewall rule.** node3 reaches the cache host's Redis
  (node2) only once the gateway's UniFi policy admits TCP 6379 from node3, as it does
  for the other nodes. Until then every job uses its runner's local cache.

### Deploy or update

```powershell
pwsh tools/ci-runners/windows/deploy.ps1 -EgressExit <exit-host:port>   # first time
pwsh tools/ci-runners/windows/deploy.ps1                                 # update; keeps node3's settings
pwsh tools/ci-runners/windows/deploy.ps1 -Stop                           # stop the three services
```

`deploy.ps1` needs repo admin through `gh` (for the registration token), the
`cimmeria-node3` ssh alias (an admin account) and `cimmeria-node2` (sudo, to read the
cache password from its `.env`). It sends the token and the password over ssh stdin
into `F:\gha\deploy\secrets\`, never through this machine's disk or a command line, and
`setup.ps1` deletes the file when it ends. The first run installs VS Build Tools, which
takes a while. A rerun changes only what differs; a runner whose `.env` or service
environment changed is restarted, which cuts off a job running on it, so update while the
pool is idle. The runner updates itself between jobs.

### Disk use per runner

| Item | Bound |
| --- | --- |
| The job's `target\` | Deleted by `job-completed.ps1` after every job |
| Compile cache | The cache host's Redis, or the runner's `sccache\` (trimmed to 20 GB by sccache) |
| Toolchain and cargo registry | A few GB in the runner's `rust\` |
| `RUNNER_TEMP`, the runner's `tmp\` | `RUNNER_TEMP` is emptied after every job; `tmp\` entries older than an hour go |
| Node versions from `actions/setup-node` | Under `work\_tool`, one copy per version |

### Security on node3

- **Jobs are not admin.** Unlike the Linux pool, a job does not get the host. It runs
  as `gha-runner`, which can write only to its runner directories (its `.env` and the
  hooks are read-only to it).
- **It is denied the VMs' storage.** Like any local user it would get what the host's
  ACLs give the Users group, which on node3 includes read access to the VM folders.
  `setup.ps1` adds an inherited deny (read and list) for `gha-runner` on every Hyper-V
  storage folder it finds (the host defaults, and each VM's configuration, checkpoint,
  smart-paging and disk folders; on 2026-10-11 `D:\VHDs`, `E:\VMs`, `F:\VHDs`, `F:\VMs`
  and `G:\VHDs`). It changes no other entry and never touches the VMs. A check run as
  `gha-runner` listed its own runner directory and was refused every VM folder and a VHD.
- **Runner state persists**, as on Linux: the work tree, the toolchain, the cargo
  registry and its `bin\`, so a bad job can leave something behind for later jobs on that
  runner. The cache password is readable by every job, as on Linux.
- **Who can run code here** is the same as the Linux pool: people with write access;
  fork PRs need a maintainer's approval and the routing sends them to GitHub-hosted
  runners; Dependabot PRs go to GitHub-hosted runners.
- **Defender stays on.** Real-time scanning slows builds, but no exclusions are added.
