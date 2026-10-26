# client-tls-oidc wave 5: colo HTTPS, password at rest, publish, UAT

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders, packet-reviewers and the owner
> **Last updated:** 2026-10-25
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract](contract.md), [wave 4](packets-wave4.md), [ledger](README.md)

Wave 5 ships **one PR per packet**, each from main as its gate opens. Nothing here blocks waves 0-4 from merging; waves 0-4 are safe to ship with no colo change, because the launcher only offers sign-in when `https://<login host>/auth/v1/providers` answers (C9), and until CT-52 nothing answers.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-50 Colo read-only check | ops | none (read-only); **Done** 2026-10-25 ([worknote](worknotes/colo-https-check.md)) | - |
| CT-51 Auth proxy overlay and runbook (board Caddy route; re-sliced 2026-10-25) | packet-coder, then documentation-writer | none | CT-50 worknote |
| CT-52 Apply on the colo: overlay, then the board Caddy route | a `hands-on` agent (the runbook over `ssh`) | none (owner decided the route 2026-10-25; certificate, DNS and the 443 forward already exist); a quiet time for the game-server restart | CT-51 merged, the campaign released in the colo image |
| CT-53 Register providers | owner | **owner**: provider portals, secrets | CT-52 |
| CT-54 Password algo 3 | packet-coder; `network-security-auth` review | none | wave 2 merged (CT-21 uses `sha1_upper_hex`) |
| CT-55 Server `/password` routes | - | **Cancelled** (D-CT11, 2026-10-24) | - |
| CT-56 Account deletion | - | **BlockedDecision D-CT16** | - |
| CT-57 Auto-submit on | packet-coder | CT-04 V2 passed | wave 0 merged |
| CT-58 Build and publish 017 | ops builds; owner signs | **lab** (approved in principle 2026-10-24; confirm the lab is free and the lab MCP is up) and **owner** signing | wave 0 merged, CT-04 passed |
| CT-59 Launcher game-password UI | - | **Cancelled** (D-CT11, 2026-10-24) | - |
| CT-U1..U6 UAT | orchestrator with the `lab-uat` skill | **lab** (approved in principle 2026-10-24; confirm at execution) | CT-52, CT-53, CT-58 and the launcher release |
| CT-60 Close-out | documentation-writer | after the UATs ran or were deferred | all |

Every code packet carries R1-R9 from [work-packets.md](work-packets.md#rules-every-packet-carries). Each gets a fresh worktree from current main (`mk-worktree.ps1 client-tls-oidc/CT-NN-<slug> cto-NN`) and ships with the `ship-pr` skill after its packet-review loop returns `ship`.

## CT-50 Colo read-only check

- **Host:** windows · **Agent:** ops (read-only colo checks are on its command list) · **Branch:** `client-tls-oidc/CT-50-colo-check` · **Worktree:** `cto-50`
- **Output:** `docs/analysis/client-tls-oidc/worknotes/colo-https-check.md` (CRLF), committed by ops with an explicit path; docs-only PR, or folded into CT-51's PR when CT-51 is dispatched the same day.

Over `ssh <colo-alias>`, read only; change nothing:

1. `docker ps --format '{{.Names}} {{.Image}} {{.Ports}}'`.
2. `ss -ltn` filtered to `:80 `, `:443 ` and `:13480 `: is anything bound?
3. `docker network ls --format '{{.Name}}'`: is `signoz-net` there?
4. `ls /opt/cimmeria` and `ls /opt/cimmeria/config` (names only).
5. `cut -d= -f1 /opt/cimmeria/.env`: the key **names** only, never values. Record whether `COMPOSE_FILE` is set; then `grep '^COMPOSE_FILE=' /opt/cimmeria/.env` (its value is file names, not a secret).
6. From the workstation: `Resolve-DnsName <login host>` where `<login host>` is the host of `default_servers()` in `crates/launcher-core/src/client_setup/login_servers.rs`. Record only whether it resolves and whether it is the same name players use for 8081; **never write the address**.
7. The gateway port forwards: only when `ssh <gw-alias>` read commands are on the ops list. Otherwise write "edge forwards not checked; the colo runbook lists 8081/tcp, 32832/udp and 30000/tcp".

The worknote answers: Is 443 free on the host? Is 80 needed (no: CT-51 uses TLS-ALPN-01)? Which overlays run? Does `config/` exist? Is the login host a DNS name (a certificate needs one)? No IPs, no `.env` values, no account names (R7).

## CT-51 Auth proxy overlay and runbook (re-sliced 2026-10-25)

- **Host:** windows · **Agent:** packet-coder for items 1-4 (first commit), then a documentation-writer for items 5-9 in the same branch (second commit) · **Branch:** `client-tls-oidc/CT-51-auth-proxy-overlay` · **Worktree:** `cto-51`
- **Commits:** `feat(docker): publish the auth proxy listener on host loopback (CT-51)`, then `docs(operations): sign-in HTTPS through the board's Caddy (CT-51)`.
- **Why re-sliced:** the CT-50 worknote found host 443 and 80 held by the board's Caddy (`board-proxy`, host networking), which already serves `https://play.cimmeria.app` (the login host of `default_servers()`) with a certificate, DNS and the 443 edge forward in place; today it forwards five telemetry routes to the server and answers 404 for the rest ([telemetry-upload-endpoint.md § The colo route](../../operations/telemetry-upload-endpoint.md#the-colo-route)). Owner decision 2026-10-25 (D-CT4 amended): no second Caddy; the board's Caddy gets a route for `/auth/v1/*` and `/SGWLogin/*`. The old `docker/compose.https.yml` and `docker/caddy/Caddyfile` are dropped. That Caddyfile is host config in no repository (OPS-LT1), so this packet ships only the repo side and documents the Caddy block; CT-52 edits the host.
- **Rules (full text in [work-packets.md](work-packets.md#rules-every-packet-carries)):**
  - R1: PowerShell only; start every command with `Set-Location <worktree>;`; absolute paths for every file write.
  - R3: edit only the files listed below; `git add` explicit paths; commit with the subject above and the attribution lines; no stash, rebase or push.
  - R4: `docs/operations/telemetry.md` is 1052 lines, over the hard cap, so edit it in place and its line count must not grow. Every other file stays under 500.
  - R5: `docs/**/*.md` is CRLF; check `([IO.File]::ReadAllText('<abs>') -match "[^\r]\n")` is `False` for each. `docker/*.yml`, `docker/.env.example` and the workflow keep the endings their neighbours have (check `docker/compose.lab.yml`).
  - R7: no colo address, `.env` value, gateway address, secret or account name anywhere. Loopback `127.0.0.1`, the documentation range `192.0.2.x` and the default `172.16.0.0/12` are allowed.
  - R9: never hand-edit a `<!-- gen:NAME -->` block.
  - No cargo: this packet compiles nothing; do not run the lane.
- **Contract excerpt (C2, C4).** `AUTH_PROXY_ADDR` (unset by default) turns on the proxy listener. Every request on it passes `proxy_layer`: (0) the TCP peer must lie in `AUTH_PROXY_TRUSTED_PEERS` (comma-separated CIDR or bare addresses; unset or empty means the default `127.0.0.0/8,::1`, loopback only since D-CT26 = b; one bad entry trusts no peer and logs ERROR `bad_auth_proxy_trusted_peers`), else `X-Forwarded-*` is ignored and `/auth/v1` answers `403 {"error":"https_required"}` with a `proxy_untrusted_peer` WARN; (1) a trusted peer must send `X-Forwarded-Proto: https`, else `400`; (2) the rightmost `X-Forwarded-For` entry must parse as an IP, else `400`, and it replaces the peer address for the handlers; (3) the request gets `TlsConn`. The plain `8081` listener ignores `X-Forwarded-*`, so `/auth/v1` there is always `403 https_required`. C4: `CIMMERIA_AUTH_PROVIDERS` names the provider file; secrets come from the env names its `client_secret_env` gives; a literal `client_secret` is a fatal config error.
- **Who the server sees as the peer.** The board's Caddy runs in the host network namespace and dials `127.0.0.1:13480`. Docker forwards a loopback-published port into the container through `docker-proxy` (or, with the userland proxy off, a NAT rule that masquerades), so the server's peer is the **bridge gateway of the container's network**, not `127.0.0.1`. `cimmeria` joins only `signoz-net` (`docker/compose.yml`). Docker's default pools put that gateway inside `172.16.0.0/12`, which the default list (loopback only since D-CT26 = b) does not trust, so the proxy answers 403 `https_required` until the list names the gateway. CT-52 sets the list on the colo to the gateway alone (D-CT22), which also keeps the `signoz` and `otel-collector` containers on `signoz-net` out of it; they can reach `cimmeria:13480` directly and could forge `X-Forwarded-For`. Caddy's `reverse_proxy` sets `X-Forwarded-Proto` from its own TLS connection and appends the caller to `X-Forwarded-For`, so the rightmost entry is the real caller and C2 needs no `header_up` line.

Files:

1. New `docker/compose.auth-proxy.yml` (LF, like `docker/compose.lab.yml`), a header comment block, then:

```yaml
services:
  cimmeria:
    environment:
      # The auth proxy listener (contract C2), bound inside the container.
      AUTH_PROXY_ADDR: "0.0.0.0:13480"
      # Empty keeps the server default 127.0.0.0/8,::1 (loopback only).
      AUTH_PROXY_TRUSTED_PEERS: "${AUTH_PROXY_TRUSTED_PEERS:-}"
      CIMMERIA_AUTH_PROVIDERS: "/opt/cimmeria/config/auth-providers.toml"
      CIMMERIA_IDP_DISCORD_SECRET: "${CIMMERIA_IDP_DISCORD_SECRET:-}"
      CIMMERIA_IDP_OIDC_SECRET: "${CIMMERIA_IDP_OIDC_SECRET:-}"
    ports:
      # Host loopback ONLY, for a TLS-terminating proxy on this host.
      # Never "13480:13480": that publishes it on every interface.
      - "127.0.0.1:13480:13480"
    volumes:
      - ./config/auth-providers.toml:/opt/cimmeria/config/auth-providers.toml:ro
```

   The header says, in this order: what it does (turns on the C2 proxy listener and publishes it on host loopback for a TLS-terminating reverse proxy on the same host; on the colo that is the board's Caddy, runbook `docs/operations/auth-https.md`); that the listener trusts `X-Forwarded-*` from its trusted peers, so 13480 is never in a public `ports:` entry and never edge-forwarded; that through a loopback publish the server sees the container network's bridge gateway, not loopback, and `AUTH_PROXY_TRUSTED_PEERS` in `.env` may narrow the default to that gateway (`docker network inspect signoz-net --format '{{range .IPAM.Config}}{{.Gateway}}{{end}}'`, written as `<gateway>/32`, never committed); that `config/auth-providers.toml` must exist before `docker compose up -d` (otherwise Docker creates an empty directory in its place, as with `discord.toml`), owned by uid 1001 and mode 0440; that the secret variable names must match the file's `client_secret_env`; how to enable (append `:compose.auth-proxy.yml` to `COMPOSE_FILE`, `docker compose up -d`, which recreates `cimmeria` and restarts the game server); that an image without the proxy listener ignores `AUTH_PROXY_ADDR`, so the overlay is harmless there.
2. `docker/.env.example`: in the `COMPOSE_FILE` examples at the top, one more line `#   COMPOSE_FILE=compose.yml:compose.discord-file.yml:compose.lab.yml:compose.auth-proxy.yml   (the colo, with launcher sign-in)`. After the lab block (after `# CIMMERIA_LAB_MCP_TOKEN=`), a commented block in the file's style: `# compose.auth-proxy.yml only (docs/operations/auth-https.md): the provider secrets named by config/auth-providers.toml's client_secret_env, and an optional narrower trusted-peer list for the proxy listener (empty keeps 127.0.0.0/8,::1,172.16.0.0/12).`, then `# CIMMERIA_IDP_DISCORD_SECRET=`, `# CIMMERIA_IDP_OIDC_SECRET=`, `# AUTH_PROXY_TRUSTED_PEERS=`.
3. `docker/compose.yml`, the overlay list in the header (l.20-23) only: `#   compose.auth-proxy.yml     Auth proxy listener on host loopback (launcher sign-in)`. Nothing else in that file.
4. `.github/workflows/pr-container.yml`, after the step `validate docker/compose.yml` (l.239-244): a step `validate docker/compose.auth-proxy.yml` with the same `env` (`BASE_EXTERNAL: 127.0.0.1`), whose `run:` does two things: `docker compose -f docker/compose.yml -f docker/compose.auth-proxy.yml config -q`, then the loopback guard `docker compose -f docker/compose.yml -f docker/compose.auth-proxy.yml config --format json | python3 -c 'import json,sys; p=[x for x in json.load(sys.stdin)["services"]["cimmeria"]["ports"] if str(x.get("target"))=="13480"]; assert p and all(x.get("host_ip")=="127.0.0.1" for x in p), p'`. Run `rg -n "python3|runs-on" .github/workflows/pr-container.yml` first and report which runner the job uses; if python3 is not certain there, keep it and say so. The `paths:` filter already has `docker/**`; do not touch it.
5. Second commit, documentation-writer: new `docs/operations/auth-https.md` (CRLF, Diátaxis how-to, under 250 lines), with these parts:
   - A status table first, like `telemetry-upload-endpoint.md`'s: the overlay is checked by CI (`config -q` and the loopback guard); the Caddy route is **not applied** (CT-52) and nothing on the page has been probed.
   - What it is: the board's Caddy terminates TLS for `play.cimmeria.app` and forwards `/auth/v1/*` and `/SGWLogin/*` to the server's proxy listener on host `127.0.0.1:13480`; every other path stays 404 apart from the telemetry routes; plain 8081 keeps working for unmodified clients (D-CT7).
   - Who owns what: the overlay is in this repo; the Caddyfile belongs to the board stack, is host config, and is in no repository until OPS-LT1 mirrors it into the private agent-board repo; the certificate, DNS record and 443 edge forward already exist for the board and the telemetry route, so nothing changes at the edge and port 80 is untouched.
   - Prerequisites: an image that carries the proxy listener (the server logs that the auth proxy listener started; take the exact text from `crates/auth/src/auth/service.rs`); `config/auth-providers.toml` in place before step 1: for a first start, `config/auth-providers.toml.example` with every `[[provider]]` block and the `[links]` table removed, `public_base_url = "https://play.cimmeria.app"`, `signup_open = false`, then `sudo chown 1001:1001` and `sudo chmod 0440` (providers come in CT-53).
   - Step 1, the container: back up `.env` (`.env.bak-<date>`), append `:compose.auth-proxy.yml` to `COMPOSE_FILE`, optionally narrow `AUTH_PROXY_TRUSTED_PEERS` to the `signoz-net` gateway (the recipe from the overlay header; through a shell variable, never printed), `docker compose up -d` (a game-server restart, like a watchtower swap). Host checks: `ss -ltn` shows 13480 on `127.0.0.1` only; `curl -s -o /dev/null -w '%{http_code}\n' -H 'X-Forwarded-Proto: https' -H 'X-Forwarded-For: 192.0.2.10' http://127.0.0.1:13480/auth/v1/providers` gives `200`; the same without `X-Forwarded-Proto` gives `400`; a `403` with a `proxy_untrusted_peer` WARN means the narrowed list missed the peer: remove the line and `docker compose up -d` again.
   - Step 2, the Caddy route: find the live file with `docker inspect board-proxy --format '{{range .Mounts}}{{.Type}} {{.Source}} -> {{.Destination}}{{"\n"}}{{end}}'` (if the Caddyfile is not bind-mounted it is baked into the board image and only the agent-board repo can change it); back it up beside itself as `Caddyfile.bak-<date>`; add the block below **inside the existing `play.cimmeria.app` site block**, before its catch-all (`handle` with no matcher, or `respond 404`), because a second site block for the same host is a Caddy error. A single-file bind mount pins the inode: write the new content over the old file (`cat new > live`), never `sed -i`, `mv` or an editor that replaces the file, or the container keeps the old one; check with `docker exec board-proxy grep -c cimmeria_auth <in-container path>` (2). Then `docker exec board-proxy caddy validate --config <path> --adapter caddyfile`, then `caddy reload` the same way; if reload is refused (admin API off), `docker restart board-proxy`, a few seconds of board downtime.

```caddyfile
	# Cimmeria launcher sign-in (client-tls-oidc CT-51): only these two path
	# families reach the auth proxy listener; nothing else of the server does.
	@cimmeria_auth path /auth/v1/* /SGWLogin/*
	handle @cimmeria_auth {
		reverse_proxy 127.0.0.1:13480
	}
```

   - Checks from outside the colo network (`curl.exe` on Windows): `https://play.cimmeria.app/auth/v1/providers` starts `{"api":1`; `POST /SGWLogin/UserAuth` answers with a SOAP status, not 404; `/admin` and `/api/config` give 404; `POST {}` to `/api/auth/dev-session` still gives 422 (the telemetry route is unchanged); `https://board.cimmeria.app/` still answers 200; `http://play.cimmeria.app:8081/auth/v1/providers` gives 403 `https_required` (the plain listener is unchanged); the server log has no `proxy_untrusted_peer`, `proxy_missing_https_proto` or `proxy_bad_forwarded_for` WARN since the reload.
   - How the forwarded headers work (the contract excerpt above, in plain words) and why the peer is the bridge gateway, not loopback (the same reason as OPS-LT2 on the telemetry route; the proxy listener trusts the gateway's range by default, the telemetry quota key does not).
   - Kill switches (C4, C14): `CIMMERIA_IDP_DISABLED=1`, `CIMMERIA_IDP_SIGNUP_DISABLED=1`, `CIMMERIA_SOAP_RELAY_DISABLED=1`, `CIMMERIA_PASSWORD_REHASH_DISABLED=1`; each needs `docker compose up -d`.
   - Rollback, fastest first: (1) restore the newest `Caddyfile.bak-*` over the live file (same inode rule), validate, reload: the two path families give 404 again, the telemetry route and the game are untouched, and launchers stop offering sign-in at their next discovery (C9). (2) Only to remove the listener too: restore `.env.bak-*` (or drop `:compose.auth-proxy.yml` and `AUTH_PROXY_TRUSTED_PEERS`), `docker compose up -d --remove-orphans` (a game-server restart). Never `docker compose down -v`.
   - The OPS-LT1 warning: a `board-proxy` redeploy from the agent-board repo before the live Caddyfile is mirrored there removes this route and the telemetry route; the symptom is 404 on both, and launchers fall back to no sign-in.
   - Self-hosters: the same overlay works behind any TLS proxy on the same host (put the proxy's peer network in `AUTH_PROXY_TRUSTED_PEERS` if the server logs `proxy_untrusted_peer`), or use the in-process TLS listener (`AUTH_TLS_CERT_PATH`, `AUTH_TLS_KEY_PATH`, C2); with neither, the launcher never offers sign-in and Play works as before.
6. `docs/operations/colo-deploy.md`: in the Ports table, a row `13480/tcp | Auth proxy listener (overlay compose.auth-proxy.yml) | The board's Caddy on the same host | 127.0.0.1 only`; after the "Forward only `8081/tcp`, `32832/udp` and `30000/tcp`" sentence, one sentence: the host's 443/tcp and 80/tcp belong to the board's Caddy (`board-proxy`), which serves `https://play.cimmeria.app` and forwards Cimmeria's https routes; Cimmeria publishes nothing on them. In "Files on the host", lines for `compose.auth-proxy.yml` and `config/auth-providers.toml`. Under "Optional overlays", a subsection `### Launcher sign-in (HTTPS)` of at most six lines linking `auth-https.md`.
7. `docs/operations/telemetry-upload-endpoint.md` § The colo route (l.93-96 and the OPS-LT1 paragraph l.108-114): the board's Caddy also forwards `/auth/v1/*` and `/SGWLogin/*` to the auth proxy listener once [auth-https.md](auth-https.md) is applied; a redeploy before OPS-LT1 removes both routes.
8. `docs/operations/telemetry.md` l.170-172: in place, no line added: the board's Caddy forwards the five telemetry routes (and the sign-in routes, see `auth-https.md`) and answers 404 for everything else.
9. `docs/agents/doc-update-map.md`: none unless a row names the colo runbooks or `docker/compose*.yml`; list the rows walked in the report.

Tests: config only. The CI step in item 4 is the guard: the loopback assertion fails if anyone publishes 13480 on every interface. Locally, if Docker is available, run both commands of item 4 and paste the output; then the revert proof: change the port entry to `"13480:13480"` in the working tree, rerun the assertion, show it failing, restore with `git checkout -- docker/compose.auth-proxy.yml`. Without Docker, say so; CI runs it.

Checks: R5 on every touched doc; the YAML parses (`python -c "import yaml,sys;yaml.safe_load(open(sys.argv[1]))" docker/compose.auth-proxy.yml`, and the same for the workflow); `rg -n "compose.https|cimmeria-caddy|ACME_EMAIL|AUTH_HOST=" docker docs/operations .github` finds nothing new.

## CT-52 Apply on the colo: overlay, then the board Caddy route

- **Host:** windows · **Agent:** `hands-on` (the orchestrator dispatches it; `ops` cannot write on the colo). The owner may run any step from their own shell instead.
- **Gate:** CT-51 merged to main, and the colo's running image carries the proxy listener (the campaign PR merged and a release swapped in by watchtower). No owner gate: the owner decided the board Caddy route on 2026-10-25, and the certificate, DNS record and 443 edge forward already exist. Step 2 restarts the game server like a watchtower swap, so the orchestrator runs it at a quiet time.
- **Rules:** over `ssh <colo-alias>`, `sudo -n` where a file is root-owned (`.env` is 0600 root; if it is, run every `docker compose` with `sudo -n` too, per colo-deploy.md). Never write an address (colo, gateway or LAN), an `.env` value or any Caddyfile line other than the CT-51 block into a file, worknote, commit or report; pass such values through shell variables only. Never `docker compose down`, never `-v`. Touch no board container except `board-proxy`, and restart it only as the reload fallback.
- **Step 0, preflight (read-only):** `docker ps` shows `board-proxy` and `cimmeria` up; `ss -ltn | grep ':13480 '` is empty; `docker inspect cimmeria --format '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}'` prints only `signoz-net` (anything else: skip the narrowing in step 2 and record why); from the workstation `curl.exe -sI https://play.cimmeria.app/` returns a status line (the certificate and the forward work; record the status code only).
- **Step 1, files:** in `/opt/cimmeria`, fetch `docker/compose.auth-proxy.yml` from main (`curl -fsSL https://raw.githubusercontent.com/SandboxServers/Cimmeria/main/docker/compose.auth-proxy.yml -o compose.auth-proxy.yml`); write `config/auth-providers.toml` as the runbook's first-start file (no providers, no `[links]`, `public_base_url = "https://play.cimmeria.app"`, `signup_open = false`), `chown 1001:1001`, `chmod 0440`.
- **Step 2, the container:** `sudo -n cp -p .env .env.bak-$(date +%Y%m%d-%H%M)`; append `:compose.auth-proxy.yml` to `COMPOSE_FILE`; narrowing (D-CT22; required, because the code default trusts loopback only since D-CT26 = b): `gw=$(docker network inspect signoz-net --format '{{range .IPAM.Config}}{{.Gateway}}{{end}}')`; when it is empty, restore `.env.bak-*` and return `blocked - no signoz-net gateway`; else append `AUTH_PROXY_TRUSTED_PEERS=$gw/32` to `.env` without echoing it to the terminal; `docker compose up -d`; wait for `cimmeria` healthy. Then the runbook's host checks: 13480 bound on `127.0.0.1` only; `200` with both headers, `400` without `X-Forwarded-Proto`. A `403` with `proxy_untrusted_peer`: the gateway is not the peer the server sees, and removing the line no longer helps (the default trusts loopback only); restore `.env.bak-*`, `docker compose up -d`, return `blocked - proxy peer not trusted`.
- **Step 3, the Caddy route:** per the runbook's step 2: find the mount (no bind mount for the Caddyfile: return `blocked - board Caddyfile is baked into the board image; owner changes it in the agent-board repo`); exactly one `play.cimmeria.app` site address in the file (none: return `blocked - no play.cimmeria.app site in the board Caddyfile`); back up; insert the CT-51 block before the catch-all; write in place; `grep -c cimmeria_auth` inside the container is 2; validate (a failure: restore the backup in place and return `blocked - caddy validate: <first error line>`); reload, falling back to `docker restart board-proxy`. Record `caddy version` and whether the global options contain `trusted_proxies` (yes or no only).
- **Step 4, checks from outside the colo network:** the runbook's seven checks.
- **Output:** `worknotes/colo-https-apply.md` (CRLF): date, the image revision, who ran each step, narrowing applied (yes or no, and why not), Caddyfile mount type (file or directory), backup file names, Caddy version, `trusted_proxies` present, each check's result. No addresses, no `.env` values, no other Caddyfile content.
- **Pass:** the seven checks pass from outside the colo network.
- **Rollback:** the runbook's, fastest first: restore the Caddyfile backup and reload (no game restart); only if the listener must go, restore `.env.bak-*` and `docker compose up -d --remove-orphans`.
- **After a pass:** the orchestrator tells the owner that OPS-LT1 (mirror the live Caddyfile into the agent-board repo) now protects the sign-in route as well as the telemetry route.

## CT-53 Register providers (owner action)

- Discord developer portal: an application owned by the project account; redirect `https://<login host>/auth/v1/callback/discord`; scopes `identify email`. The client id goes into the colo's `config/auth-providers.toml`; the secret into `cimmeria-kv` and the colo `.env` as `CIMMERIA_IDP_DISCORD_SECRET`.
- The generic OIDC slot (D-CT3): one provider of the owner's choice, redirect `.../auth/v1/callback/<id>`, secret `CIMMERIA_IDP_OIDC_SECRET`. `trusted_email = true` only for an issuer on the trusted list (D-CT13); Microsoft is refused by config validation.
- `signup_open` and `[limits]` per D-CT5 and C7; `min_provider_account_age_days` is the owner's call (suggest 7).
- Apply: a changed provider file needs `docker restart cimmeria` (compose sees no change in a bind-mounted file); a new secret in `.env` needs `docker compose up -d`. Then `GET /auth/v1/providers` lists the providers. Record in `worknotes/providers-registered.md`: which providers, which are trusted; no client ids, no secrets.

## CT-54 Password algo 3: argon2id over SHA1HEX

- **Host:** windows · **Agent:** packet-coder; then one read-only `network-security-auth` pass ("review CT-54 against contract C14 and design § 5.6 #6; numbered fix notes only") · **Branch:** `client-tls-oidc/CT-54-password-algo3` · **Worktree:** `cto-54`
- **Commit:** `feat(auth): store legacy passwords as argon2id over SHA-1 (CT-54)`
- **Rules:** R1-R9. Contract: [C14](contract.md#c14-wave-5-server-addition-password-algo-3), C15 (c), C1 (`subtle`), D-CT7, D-CT10, D-CT11.

Why: today a legacy (algo 1) row is unsalted SHA-1 at rest, and a plaintext TLS login migrates it to algo 2, after which the unmodified client (which sends SHA-1 hex) is locked out. Algo 3 fixes both: salted at rest, and every client still logs in. It applies only to accounts without a linked identity (the seed users today); a linked account has no password (D-CT11).

Files (all under `crates/auth/src/auth/`):

0. First, `rg -n "password_algo|password_hash_v2|ALGO_" crates tools db --glob '!target'` and list every hit in the report; a hit outside the files below that assumes algo 1 or 2 is `blocked - contract gap`. Known and already handled, not a gap: `password_check.rs`, `password_check_tests_live_db.rs` and `password_check_gate_tests.rs` (the link-form check reads algo 3 and keeps its own gate, D-CT30); do not edit them.
1. `credentials.rs` (562 lines): move the whole `#[cfg(test)] mod tests` to a new sibling `credentials_tests.rs` (`#[cfg(test)] #[path = "credentials_tests.rs"] mod tests;`) **as its own first step**, unchanged, so the move is reviewable. Then:
   - `pub(super) const ALGO_SHA1_ARGON2ID: i16 = 3;` already exists beside the other two (wave 10 fix, D-CT30); do not add it again. The module doc lists algo 3.
   - `sha1_upper_hex` becomes `pub(super)` (CT-21 may already have done this).
   - The match in `validate_credentials` gains `(ALGO_SHA1_ARGON2ID, cred)`: compute `key` = `h.to_ascii_uppercase()` for `LegacySha1Hex(h)`, `sha1_upper_hex(pw)` for `Plaintext(pw)`; `password_hash::verify_blocking(key, phc).await`; a NULL `password_hash_v2` is `InvalidCredentials`.
   - `(ALGO_SHA1_LEGACY, LegacySha1Hex(h))`: the compare becomes `subtle::ConstantTimeEq` over the uppercase bytes (lengths differ: not equal); on success `password_hash::migrate_to_algo3(db, account_id, key).await`.
   - `(ALGO_SHA1_LEGACY, Plaintext(pw))`: on success migrate to algo 3 (not 2) with `sha1_upper_hex(pw)`.
   - `(ALGO_ARGON2ID, Plaintext(pw))`: on success migrate to algo 3 with `sha1_upper_hex(pw)`.
   - `(ALGO_ARGON2ID, LegacySha1Hex(_))` stays `InvalidCredentials`.
2. `password_hash.rs` (175 lines):
   - `const ARGON2_PERMITS: usize = 4;` `static ARGON2_GATE: LazyLock<tokio::sync::Semaphore>`. This gate serves the SOAP credential path (`credentials.rs`) only. `password_check.rs` keeps its own 2-permit gate and calls the sync `verify_argon2id` inside it; never route it through `verify_blocking` (D-CT30).
   - `pub(super) async fn hash_blocking(input: String) -> Option<String>` and `pub(super) async fn verify_blocking(input: String, phc: String) -> bool`: acquire a permit, then `tokio::task::spawn_blocking` around the existing sync functions; a join error is `None` / `false`.
   - `pub(super) async fn migrate_to_algo3(db, account_id: i32, sha1_upper: String)`: when `std::env::var("CIMMERIA_PASSWORD_REHASH_DISABLED").as_deref() == Ok("1")`, log `info!(account_id, from_algo, to_algo = 3, reason = "rehash_disabled")` and return. Else hash, then `UPDATE account SET password_hash_v2 = $1, password_algo = 3, password = NULL WHERE account_id = $2`. Failure logs `warn!` with `reason` and never fails the login; the C5 guard's refusal (SQLSTATE `23514`, an identity was linked meanwhile) logs `reason = "account_has_identity"`. Read the env var through a function taking a lookup closure so the test needs no process env.
   - `migrate_to_argon2id` is removed (no caller left). Its log line carried `account_name`; the new one carries `account_id`, `from_algo`, `to_algo`, `reason` only (C14).
3. `crates/auth/Cargo.toml`: `subtle` if wave 1 (C1) has not added it; `tokio` features `rt` and `sync` must already be there; check.

Tests (`credentials_tests.rs`, live-DB ones keep `live_db` in the name and use sentinel ids the file already uses; new sentinels continue that range):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_legacy_hash_login_moves_to_algo3` (renames the l.394 test) | algo 3, `password` NULL, `password_hash_v2` starts `$argon2id$` | D-CT10 |
| `live_db_unmodified_client_still_logs_in_after_algo3` | after the migration, `LegacySha1Hex` of the same password is `Ok` again; a wrong hex is `InvalidCredentials` | the D-CT7 lockout |
| `live_db_plaintext_login_moves_legacy_to_algo3` (renames the plaintext-migrate test) | algo 3; then SHA-1 hex logs in | the old code went to algo 2 |
| `live_db_argon2_plaintext_login_moves_to_algo3` | an algo 2 row, plaintext login: algo 3; then SHA-1 hex logs in | heals rows already locked out |
| `live_db_argon2_account_rejects_legacy_hash_shape` (existing) | unchanged | refusal kept |
| `live_db_rehash_disabled_keeps_algo1` | lookup closure returns `"1"`: login `Ok`, algo stays 1 | the kill switch |
| `live_db_rehash_refused_on_a_linked_account` | `migrate_to_algo3` on an account that has an identity row: no panic, the row keeps both password columns NULL | C15 (c): the guard refuses, the code survives it |
| `argon2_gate_bounds_concurrency` | 8 concurrent `verify_blocking` calls on a test-local semaphore of 2 (make the gate a parameter of an inner fn) never exceed 2 in flight (an `AtomicUsize` high-water mark) | DoS bound |
| `legacy_compare_is_constant_time_shape` | the algo-1 compare goes through a `fn hex_eq(a: &str, b: &str) -> bool` using `ct_eq`; unit-test equal, case-different and different-length inputs | the `subtle` change |

Also run the other suites that insert algo-1 rows (`session_kind.rs`, `tls_smoke.rs`, `crates/wireclient/tests/it/support/mod.rs`): they log in once with the SHA-1 and must stay green; a fixture that re-reads `account.password` after login must read `password_algo` instead; report each.

Checks: S1 `cimmeria-auth`, S2 `cimmeria-auth`, S3 `credentials`, S3 `password_check_tests_live_db`, S3 `session_kind`, S3 `tls_smoke`, and the wireclient live suite through S3 with the filter its README names (report the filter). Revert proof: restore `credentials.rs` from main in the working tree, run S3 `credentials`, show `live_db_unmodified_client_still_logs_in_after_algo3` failing to compile or failing, restore.

**Deploy note for the PR body:** only seed accounts carry passwords today (D-CT11), so no player data is at stake and no deploy approval is needed; still fix forward rather than revert once it ran (C14); the kill switch exists; the first login of each legacy account costs one argon2id hash (64 MiB, about 50-100 ms) inside the permit bound.

## CT-55 Server game-password routes (Cancelled)

Cancelled by D-CT11 (owner, 2026-10-24): an account with a linked identity never has or uses a password (contract C15), so there is no route that sets one. Not dispatchable; do not re-spec without a new owner decision.

## CT-56 Full account deletion (BlockedDecision D-CT16)

Not dispatchable. The open question for the owner: does a player-requested deletion remove the `account` row (C5's `ON DELETE CASCADE` then removes identities, refresh tokens and events, and every `sgw_player` row cascades through `sgw_player_account_id_fkey`), or does it disable and anonymise the row and keep characters for a grace period? Until decided, GDPR erasure of provider data is covered by unlink (it deletes the `account_identity` row, which holds the only provider data) and by a GM request. When decided, the architect writes this packet.

## CT-57 Turn on auto-submit

- **Gate:** CT-04's worknote records V2 as passed (the overlay screen with `auto = true` logs in without a hang). Otherwise this packet stays BlockedDependency and the architect re-specs it.
- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-57-auto-submit` · **Worktree:** `cto-57`
- **Commit:** `feat(launcher-core): submit the launcher sign-in automatically (CT-57)`
- **Rules:** R1-R9. Contract: C8, D-CT2.

Files:

1. `crates/launcher-core/src/client_setup/login_session.rs`: `pub const TICKET_AUTO_SUBMIT: bool = true;` with a comment citing the CT-04 worknote. Rename the test `auto_submit_stays_off_until_v2` to `auto_submit_is_on_after_v2` and assert `true`.
2. `data/client-patches/017-launcher-sign-in/test/LoginInternal.ticket.lua`: `CimmeriaLogin.auto = false` becomes `true`. The file is `-text` ASCII CRLF: edit with `[IO.File]::ReadAllText`/`WriteAllText` on the absolute path and check the CRLF count is unchanged.
3. `data/client-patches/017-launcher-sign-in/test/run.lua`: the "ticket, manual" case now replaces `auto = true` with `false` in memory; the "ticket, auto" case loads the golden file as is.

Tests: `renders_the_golden_ticket_file` (existing) and L1. Checks: S1 `cimmeria-launcher-core`, S2 `cimmeria-launcher-core`, L1. Ships with the next launcher release (the desktop launcher links `launcher-core`).

## CT-58 Build, lab-load and publish patch 017

- **Gate:** lab use approved in principle (owner, 2026-10-24); at execution time confirm the lab is free and the lab MCP is up (lab-load before publishing is a standing rule). The owner signs. CT-04 passed.
- **Who:** ops builds and prepares; the orchestrator runs the lab-load with the `lab-uat` skill; the owner signs and uploads. No agent reads or uses the release key.

Steps:

1. Build the zip through the lane, from main at the commit that holds 017: `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-patchset -- build data/client-patches/017-launcher-sign-in/patch.json --stock <stock dir> --patched <scratch dir> --out <scratch>\017-launcher-sign-in.zip --blob-url <release asset URL>` (read `cimmeria-patchset --help` for the exact flag names first; the 008 patch's README shows a full example).
2. Lab-load: apply the zip to a lab client (it replaces `Login.lua` and `Login.layout`), launch without a launcher sign-in: the stock login works (password typed); then with a ticket file: the ticket screen works. Without this, do not publish.
3. Prepare the manifest per [launcher-distribution-setup.md](../../client/launcher-distribution-setup.md) (the `prepare-file-index.ps1` runbook): the 017 entry goes `after` the current last patch; the chain is append-only; rebuild `file-index.json`.
4. Owner: sign, verify, back up, upload, swap and verify again, per the same doc's "Publish and roll back" list.
5. Order: the launcher release carrying CT-35 does not need to precede 017 (017 without a ticket file behaves as stock, so `min_launcher` stays unchanged); CT-57's release should follow 017 or come with it.

Output: `worknotes/publish-017.md` (CRLF): the zip SHA-256, the manifest entry (no key material), the lab-load result.

## CT-59 Launcher game-password UI (Cancelled)

Cancelled with CT-55 by D-CT11 (2026-10-24). The account panel's fixed no-password line (CT-43) replaces it.

## CT-U1..U6 UAT (lab-gated)

Lab use is approved in principle (owner, 2026-10-24). Before each lab session, confirm the lab is free and the lab MCP is up; if either fails, the session waits. The orchestrator runs the `lab-uat` skill: a Haiku `lab-driver` drives; a Sonnet verifier checks the server log, `login_audit`, `account_identity_event` and SigNoz. Use the `dev_mock` provider on a dev server, then Discord on the colo once CT-53 is done. Write each result to `docs/analysis/client-tls-oidc/worknotes/uat-<id>.md` and add rows to `docs/guides/unified-uat.md` in CT-60.

| ID | What | Pass |
|---|---|---|
| CT-U1 | Sign-in, sign-up, link to an existing non-staff, non-test account by password and another by GM link code (fresh accounts made for the UAT; every seed account is staff or test and is refused), add a second provider, unlink one, try to unlink the last | each step visible in the launcher on the first press; the removal warning and the confirm press appear before each link; after each link the old password fails in an unmodified client (error 4) and `password_cleared` is written; the last unlink is refused; events written; no subject, email or token in any log |
| CT-U2 | Play with a ticket: the overlay screen, Enter (and auto-submit after CT-57), reach character select | `login_audit.outcome = launch_ticket` over TLS; the ticket file is scrubbed after exit |
| CT-U3 | Change Server at character select (V3), then exit | the second `UserAuth` succeeds through the relay; `launcher.relay.stopped` |
| CT-U4 | GM `.account identities`, `unlink`, `linkcode` from the game console | outputs as C10; a non-GM is refused |
| CT-U5 | macOS launcher with Wine (V5): sign-in, Keychain entry, Play through the relay | the game reaches character select; the Keychain item exists; skip with a note when no Mac is free |
| CT-U6 | Close the launcher at character select (close guard), Keep running and Close anyway; then "Use password" on the ticket screen with a seed (non-OIDC) account's name and password; then an unmodified client on plain 8081 with a seed account | the prompt text as C13/CT-41; Close anyway ends the relay; the seed password and the unmodified client both log in (D-CT7) |

## CT-60 Close-out

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `client-tls-oidc/CT-60-close-out` · **Worktree:** `cto-60`
- **Commit:** `docs: close out client-tls-oidc (CT-60)`

1. Status docs, once for the campaign: `docs/project-status.md` and the `docs/gap-analysis*` rows for login security, account linking and TLS (find them with `rg -n "SHA-1|OIDC|login" docs/gap-analysis*`).
2. `docs/guides/unified-uat.md`: one row per CT-U1..U6 with its state (passed, pending with reason).
3. The ledger README: every packet's final state; "still open" refreshed.
4. `.claude/agent-memory/network-security-auth/`: one note on the algo 3 scheme, the ticket and the relay, with links (no addresses, no account names); update that folder's index file.
5. Walk `docs/agents/doc-update-map.md` and list the rows touched in the PR body.
6. `docs/operations/auth-https.md`: the status table and the colo-route lines in `telemetry-upload-endpoint.md` from `worknotes/colo-https-apply.md` (applied, probed, the date).
