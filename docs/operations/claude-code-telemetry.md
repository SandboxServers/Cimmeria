# Claude Code telemetry to the colo SigNoz

> Type: how-to. Audience: the Claude Code coordinator, the person who runs the workstations, and the colo operator.
> Updated: 2026-10-03 (TP-04 of [#957](https://github.com/SandboxServers/Cimmeria/issues/957)). Ledger: [docs/analysis/token-usage/](../analysis/token-usage/README.md). Field reference: Claude Code [monitoring-usage](https://code.claude.com/docs/en/monitoring-usage), read 2026-10-03.

Claude Code can export OpenTelemetry metrics and events for every API request, tool call and permission decision. This runbook sends them from the workstations to the colo SigNoz, so token use can be checked live and reconciled against the transcript profiler ([TP-05](../analysis/token-usage/README.md#packets)).

Decision D-TP2 sets the rules: the sink is the colo SigNoz behind Cloudflare Access, `OTEL_LOG_TOOL_DETAILS=1`, prompt logging off, and no endpoint or credential in the repo. Every address below is a placeholder.

## Where the colo stands (checked 2026-10-03)

| Question | Answer |
|---|---|
| Is a Cloudflare Tunnel running on the colo? | **No.** There is no `cloudflared` container, service or config on the colo. [signoz-remote-access.md](signoz-remote-access.md) describes a setup that was never deployed there. |
| How does the colo run SigNoz? | From SigNoz's own upstream compose project (`signoz`), not from [`docker/compose.yml`](../../docker/compose.yml). Its collector publishes OTLP gRPC `4317` and OTLP HTTP `4318`, and the UI publishes `8080`, to the colo's private network. The repo compose's `OTLP_BIND=127.0.0.1` default does not apply there. |
| How do workstations reach it today? | Over the private network. The SigNoz MCP container on the main workstation already uses that route to the UI port, and the OTLP ports answer on it too (an OTLP/HTTP `GET /v1/logs` returns 405, gRPC `4317` accepts connections). |
| Is OTLP ingest exposed through Cloudflare Access? | **No.** Nothing is. |
| Does any Claude Code data arrive yet? | No. SigNoz has no `claude_code.*` metric. |

Not checked: whether the colo's upstream network edge forwards `4317`, `4318` or `8080` from the internet. Docker-published ports bypass the host's `INPUT` chain, so if the edge forwards them, anyone could write to the collector or open the UI. The operator should confirm the edge does not forward them (step 0 below).

So D-TP2 needs a colo change before it can be met as written. Two ways to proceed:

- **Path B, D-TP2 as decided:** an operator adds an OTLP/HTTP hostname to a Cloudflare Tunnel behind an Access service token. Steps in [Colo setup for Path B](#colo-setup-for-path-b-operator).
- **Path A, interim:** export straight to the collector over the private network, with no credentials. It works today from a workstation on that network and never crosses the public internet, but it is not behind Access, so it **needs the owner's OK** as a stand-in for D-TP2.

## What gets sent

Claude Code exports two signals. Traces stay off: they are a beta and would add the `tool.output` content options this setup refuses.

**Metrics** (every 60 s, delta temporality as configured here):

| Metric | Unit | Attributes used here |
|---|---|---|
| `claude_code.cost.usage` | USD (estimate) | `model`, `query_source` (`main`, `subagent`, `auxiliary`), `agent.name`, `skill.name`, `mcp_server.name`, `mcp_tool.name`, `effort`, `speed` |
| `claude_code.token.usage` | tokens | `type` (`input`, `output`, `cacheRead`, `cacheCreation`) plus the cost attributes |
| `claude_code.session.count` | count | `start_type` |
| `claude_code.active_time.total` | s | `type` (`user`, `cli`) |
| `claude_code.lines_of_code.count`, `claude_code.commit.count`, `claude_code.pull_request.count`, `claude_code.code_edit_tool.decision` | count | as documented upstream |

**Events** (OTLP logs, batched every 5 s; the log body is the event name, for example `claude_code.api_request`):

| Event | What it gives the profiling work |
|---|---|
| `claude_code.api_request` | One row per request: `model`, `cost_usd`, `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`, `duration_ms`, `request_id`, `query_source`, `agent.name`. `request_id` joins it to the transcript record. |
| `claude_code.tool_result` | `tool_name`, `success`, `duration_ms`, `tool_input_size_bytes`, `tool_result_size_bytes` (the context a tool adds), and with tool details the parameters. |
| `claude_code.tool_decision` | `decision` and `source`; the `user_*` sources are human interventions. |
| `claude_code.api_error`, `claude_code.api_retries_exhausted` | Failures and retries, by `status_code`. |
| `claude_code.user_prompt` | `prompt_length` and command name only; the prompt text is redacted. |
| `claude_code.mcp_server_connection`, `claude_code.skill_activated`, `claude_code.hook_registered`, ... | Session furniture, low volume. |

What OTel cannot answer: cache writes are not split into 5-minute and 1-hour TTLs, and `cost_usd` is Claude Code's own estimate. The transcript profiler stays the source for both (see the ledger's corrections table).

## Privacy

What leaves the workstation, with the settings below:

- **Always attached, by Claude Code:** `user.email` (the login email, when known), `user.id` (a random install id), `organization.id`, `session.id`, `terminal.type`. `OTEL_METRICS_INCLUDE_ACCOUNT_UUID=false` drops the account UUID from metrics; nothing in Claude Code drops the email. [Scrub it at the collector](#optional-scrub-the-login-email-at-the-collector-operator) if the owner wants it gone.
- **Because of `OTEL_LOG_TOOL_DETAILS=1` (D-TP2):** tool parameters and `tool_input`, with each value cut at 512 characters. That means full Bash command lines, file paths (which contain the Windows user name), the first 512 characters of every `Write` body, `Edit` string and subagent prompt, MCP tool names and arguments, skill and agent names, and git branch and commit ids. A secret typed into a command line reaches SigNoz. `workspace.host_paths` on events carries local paths as well.
- **Never sent, and never to be turned on:** `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_ASSISTANT_RESPONSES`, `OTEL_LOG_TOOL_CONTENT`, `OTEL_LOG_RAW_API_BODIES`, `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA`. Leave them unset; the defaults are off.

The data lives only in the colo ClickHouse, under SigNoz's log and metric retention, and is visible to anyone who can open the colo SigNoz. The TP-01b privacy scrubber covers committed reports, not SigNoz, so never paste raw `tool_parameters` from SigNoz into the repo or an issue.

`OTEL_*` variables are not passed to the commands Claude Code runs, so a `cimmeria-server` started from a Bash tool call keeps its own `OTEL_EXPORTER_OTLP_ENDPOINT` and does not inherit this one.

## Cost

- **Claude usage:** none. Export happens in the Claude Code process and adds no tokens.
- **Volume:** the 2026-09-14 to 2026-10-03 transcripts held about 90k requests in three weeks, so roughly 4-5k `api_request` events a day, a few times that in `tool_result` events, and about 1-2 KB per event with tool details. Expect tens of MB a day of logs in ClickHouse, small next to the `cimmeria-network` stream.
- **Metric series:** `session.id` is on every series by default, so each session adds a new series set. About 900 sessions in three weeks is well within SigNoz's limits. If cardinality ever matters, set `OTEL_METRICS_INCLUDE_SESSION_ID=false`; the events keep `session.id` regardless.
- **Workstation:** a 5-second log batch and a 60-second metric export. With Path B, the header helper calls the Azure CLI at startup and every 29 minutes, which adds a second or two to each Claude Code start.

## Colo setup for Path B (operator)

These are changes to the colo and to Cloudflare. TP-04 did not make any of them.

0. **Confirm the edge does not forward the SigNoz ports.** From outside the private network, `4317`, `4318` and `8080` on the colo's public address must not answer. If they do, remove the forwards before anything else.
1. **Run `cloudflared` on the colo.** Either finish [signoz-remote-access.md](signoz-remote-access.md) (its UI ingress must point at `http://signoz:8080`, the colo's actual UI service, not `frontend:3301`), or create a separate tunnel. The container has to reach the collector: attach it to the SigNoz compose project's network (`docker network ls` on the colo lists it) and use the collector's service name, `signoz-otel-collector`.
2. **Add an OTLP ingress rule** above the catch-all in the tunnel config:

   ```yaml
   ingress:
     - hostname: <OTLP-HOSTNAME>
       service: http://signoz-otel-collector:4318
     # ...the UI rule, then:
     - service: http_status:404
   ```

   Use OTLP/HTTP (`4318`). gRPC through a tunnel needs extra origin settings and gains nothing here.
3. **Route DNS:** `cloudflared tunnel route dns <tunnel-name> <OTLP-HOSTNAME>`.
4. **Create the service token:** Zero Trust → Access → Service Auth → Service Tokens → Create, named `claude-code-otel`, duration one year. Copy the Client ID and Client Secret once; Cloudflare will not show the secret again.
5. **Create the Access application:** Access → Applications → Add → Self-hosted, domain `<OTLP-HOSTNAME>`, path empty. Add one policy with **action Service Auth** (not Allow, which would redirect a machine client to a login page), Include → Service Token → `claude-code-otel`. Add no browser policy; nobody browses this hostname.
6. **Store the token in the project Key Vault** as two secrets, without putting values on a command line:

   ```powershell
   # Write each value to a temp file first, then:
   az keyvault secret set --vault-name $env:CIMMERIA_KEY_VAULT --name claude-code-otel-cf-client-id     --file <tmp-id-file>     --encoding utf-8
   az keyvault secret set --vault-name $env:CIMMERIA_KEY_VAULT --name claude-code-otel-cf-client-secret --file <tmp-secret-file> --encoding utf-8
   # Delete the temp files.
   ```

7. **Smoke test** from a workstation (fetch the two values into variables from the vault first). An empty OTLP/HTTP JSON payload is accepted and writes nothing:

   ```powershell
   curl.exe -s -o NUL -w "%{http_code}`n" -X POST "https://<OTLP-HOSTNAME>/v1/logs" `
     -H "Content-Type: application/json" -H "CF-Access-Client-Id: $id" -H "CF-Access-Client-Secret: $secret" `
     --data '{"resourceLogs":[]}'
   ```

   Expect `200`. The same call without the two headers must return `403`; if it returns `200`, the Access application is not in front of the hostname.

### Optional: scrub the login email at the collector (operator)

Claude Code always attaches `user.email` when it knows it. To keep it out of ClickHouse, add a processor to the colo collector config (the SigNoz deploy's `otel-collector-config.yaml`) and put it in the `logs` and `metrics` pipelines before `batch`:

```yaml
processors:
  transform/claude-code-scrub:
    log_statements:
      - context: log
        statements:
          - delete_key(attributes, "user.email")
      - context: resource
        statements:
          - delete_key(attributes, "user.email")
    metric_statements:
      - context: datapoint
        statements:
          - delete_key(attributes, "user.email")
      - context: resource
        statements:
          - delete_key(attributes, "user.email")
```

Restart the collector afterwards. Record the change in [colo-deploy.md](colo-deploy.md), because a SigNoz upgrade that replaces the deploy directory would drop it.

## Workstation setup (coordinator, with the user)

Put the settings in the **user** settings file (`~/.claude/settings.json`), not in the repo's `.claude/settings.json`: a committed setting would switch telemetry on for every contributor, and every worktree inherits the user file anyway. Merge the `env` keys into any `env` block already there.

1. Pick a workstation label that is not a hostname or a person, for example `ws1`. It becomes `cimmeria.workstation`.
2. **Path B:** set the user environment variable `CIMMERIA_KEY_VAULT` to the vault's name, make sure `az login` is current, and check the helper prints a JSON object:

   ```powershell
   pwsh -NoProfile -File docs/operations/claude-code-telemetry/otel-headers.ps1 | ConvertFrom-Json | Get-Member -MemberType NoteProperty | Select-Object Name
   ```

   It should list the two header names and nothing else. Then merge [settings.cloudflare.example.json](claude-code-telemetry/settings.cloudflare.example.json) into the user settings, with `<ABSOLUTE-PATH-TO-REPO>` set to the main checkout (not a worktree that will be retired), `<OTLP-HOSTNAME>` and `<WORKSTATION-LABEL>` filled in.
3. **Path A (only with the owner's OK):** merge [settings.private-network.example.json](claude-code-telemetry/settings.private-network.example.json), with `<COLO-PRIVATE-ADDRESS>` set to the address the SigNoz MCP container already uses.
4. Restart Claude Code. Settings are read at start; running sessions keep exporting nothing until they restart.

The keys and why each is set:

| Key | Value | Why |
|---|---|---|
| `CLAUDE_CODE_ENABLE_TELEMETRY` | `1` | Master switch. |
| `OTEL_METRICS_EXPORTER`, `OTEL_LOGS_EXPORTER` | `otlp` | Both signals; traces stay off. |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `http/protobuf` (B) or `grpc` (A) | HTTP through the tunnel; gRPC straight to the collector. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | placeholder | Base URL; the exporter appends `/v1/logs` and `/v1/metrics` for HTTP. |
| `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE` | `delta` | The committed dashboard sums delta points. |
| `OTEL_LOG_TOOL_DETAILS` | `1` | D-TP2: real agent, MCP, skill and tool names. |
| `OTEL_METRICS_INCLUDE_ACCOUNT_UUID` | `false` | No account id on metric series. |
| `OTEL_METRICS_INCLUDE_VERSION` | `true` | `app.version` on metrics, so a Claude Code upgrade can be told apart from a workflow change. |
| `OTEL_RESOURCE_ATTRIBUTES` | `deployment.environment=cimmeria-dev,cimmeria.workstation=<label>` | Tags without personal data. No spaces or commas inside values. |
| `otelHeadersHelper` (B only) | the helper script | Fetches the Access headers from the vault at start and every 29 minutes, so the secret is never in a settings file. |

## Check that data arrives

1. Start a fresh Claude Code session and send one prompt that runs a tool.
2. After about a minute, in SigNoz (UI or the `signoz` MCP):
   - **Metrics:** `claude_code.cost.usage` and `claude_code.token.usage` exist (`signoz_list_metrics` with search `claude_code`).
   - **Logs:** `body = 'claude_code.api_request' AND cimmeria.workstation = '<label>'` returns rows with `request_id`, token counts and a non-redacted `agent.name` for subagent requests. If `agent.name` reads `custom`, tool details are not on.
   - **Privacy check:** in the same rows, `prompt` must be absent or redacted. If it holds text, prompt logging is on: stop and remove it.
3. If nothing arrives, run `claude --debug-file <scratch-path>` and look for `[3P telemetry]` lines (exporter errors). A `403` means the headers are missing or wrong: run the helper by hand. With Path B, Zero Trust → Access → Logs shows each request and the service token it used.

## Dashboards

- **Stock template:** SigNoz ships "Claude Code Metrics" (`claude-code/claude-code-dashboard.json` in the [SigNoz dashboards repo](https://github.com/SigNoz/dashboards)): sessions, active time, cost and tokens by model and type, cache efficiency, commits and PRs, tool use and success rate. Import it from the SigNoz UI (Dashboards → New dashboard → Import JSON) or with the `signoz` MCP's `signoz_import_dashboard`. It is a schema `v6` export; if the colo SigNoz rejects it, use the repo dashboard alone.
- **Repo dashboard:** [signoz/claude-code-usage.dashboard.json](signoz/claude-code-usage.dashboard.json) (`v5`, the same format as the NPC AI dashboard) adds what the profiling work needs and the stock one lacks: cost by `query_source` (main vs subagent), by `agent.name`, by MCP server and tool, by session and by workstation; tokens by model and type; API requests per agent; tool calls with count, p95 duration and result bytes; permission decisions by source; API errors by status code. Import it the same way.

Both show `cost_usd` and `claude_code.cost.usage`, which are list-price estimates. Under the Max subscription (D-TP1) they are a plan-usage proxy, not a bill; label them so wherever they are quoted.

If the `p95(duration_ms)` or `sum(tool_result_size_bytes)` columns are empty, SigNoz has typed those attributes as strings. Check with `signoz_get_field_keys` on the logs signal and change the panel to match.

## Turn it off

- **One workstation:** remove the keys (or set `CLAUDE_CODE_ENABLE_TELEMETRY` to `0`) in the user settings and restart Claude Code.
- **Everyone at once (Path B):** delete or revoke the `claude-code-otel` service token in Zero Trust. Exports start failing with `403` at the edge within seconds; Claude Code keeps working.
- **Remove stored data:** delete the dashboards in SigNoz. The rows age out with SigNoz retention; deleting them sooner is a ClickHouse operation on the colo.

## Rotating the service token

Yearly, or at once if a workstation or the vault is suspected compromised: create a new token in Zero Trust, add it to the Access policy, overwrite the two vault secrets, and delete the old token. Workstations pick up the new value within 29 minutes, or at their next start. This matches the rotation table in [signoz-remote-access.md](signoz-remote-access.md#rotating-credentials).
