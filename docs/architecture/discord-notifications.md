# Discord notifications

The server posts structured events to Discord channels via webhooks for
development-time ops visibility — login bursts, world entry, errors,
panic, etc. Eight logical channels, 40 toggleable event types, live
reload, automatic warn/error harvest from the tracing layer.

## Why webhooks (not a bot)

Webhooks are one-way HTTP POSTs; no OAuth, no gateway websocket, no
intent permissions, no token rotation. The cost is no Discord → server
direction (you can't run `/restart` from Discord). That's not in scope
today and adding a real bot later doesn't require ripping any of this
out.

## Architecture

```
emit_*() ─┐
          ├─► SenderHandle ─► bounded mpsc(256) ─► sender task ─► reqwest POST
Layer  ───┘    (try_send,                            │
                drop on full)                  per-channel token bucket
                                                     │
                                                     ▼
                                                  Discord
```

Two emit paths feed one pipeline:

1. **Explicit `cimmeria_discord::emit_*` calls** at semantically meaningful
   seams. Typed API — the compiler enforces the payload shape per variant.
2. **`DiscordLayer` (tracing_subscriber::Layer)** auto-harvests `warn!`
   and `error!` events into `Event::TracingEvent`, reading structured
   fields (`reason=`, `entity_id=`, …) directly into the embed. Zero
   instrumentation needed at existing tracing emit sites — the
   [negative-logging convention](negative-logging-convention.md) is
   already doing the field-shape work.

## Channels

Eight logical channels, each backed by one webhook URL in the TOML config:

| Channel | Default event set |
|---|---|
| `lifecycle` | ServerStartup, ServerShutdown, ServerPanic |
| `auth` | PlayerLogin, PlayerLogout, PlayerDisconnect, PlayerAuthFailed |
| `world` | PlayerWorldEntry, PlayerWorldExit |
| `chat` | ChatGlobal (others off by default — see Privacy below) |
| `gameplay` | PlayerLevelUp, MissionAccepted, MissionCompleted (others off) |
| `gm` | GmCommand, GmTeleport, GmSpawn, GmItemGrant |
| `errors` | Error, WireFormatError, DbError, AssertionFailure, MercuryTimeout (Warning off) |
| `ops` | HighLatency, PacketLossSpike, MemoryWarning, TickStall, AoiBurstWarning, OutboxLag |

The routing table is in [`crates/discord/src/router.rs`](../../crates/discord/src/router.rs) and is pinned by `every_channel_has_at_least_one_event` and `every_event_kind_routes` tests — adding an `EventKind` without choosing a channel is a compile error (no `_ =>` arm).

## Event types — full list

40 variants. Each has an explicit on/off toggle in `[discord.events]`.

```
Lifecycle:   server_startup, server_shutdown, server_panic
Auth:        player_login, player_logout, player_disconnect, player_auth_failed
World:       player_world_entry, player_world_exit
Chat:        chat_global, chat_say, chat_whisper, chat_guild, chat_team, chat_command
Gameplay:    player_level_up, player_death, player_respawn,
             mission_accepted, mission_completed, mission_failed, mission_reward_granted,
             loot_generated, item_used
GM:          gm_command, gm_teleport, gm_spawn, gm_item_grant
Errors:      warning, error, wire_format_error, db_error, assertion_failure, mercury_timeout
Ops:         high_latency, packet_loss_spike, memory_warning, tick_stall,
             aoi_burst_warning, outbox_lag
```

Unknown toggle keys in the TOML are rejected at parse time (typo guard:
`playr_login = false` fails to load).

## Defaults — signal/noise tiers

The defaults in [`EventToggles::default`](../../crates/discord/src/config.rs) prioritise signal:

- **High-signal, always on**: every lifecycle event, every auth event, every world event, every GM event, every ops alert, level-up, mission accept/complete.
- **Low-signal, off by default but toggleable**: warning (noisy), all chat except global (volume + privacy), death/respawn (volume), mission failed/reward (per-event noise), loot/item-used (very noisy), chat command (every `/who` would post).

## Privacy: whisper content is always hidden

`chat_whisper` posts the *fact* of a whisper (who → whom, when) but
**never** the message body — the embed always reads `[hidden]`. This is
enforced in [`embed::format_chat`](../../crates/discord/src/embed.rs) regardless of how the channel is configured. A test (`whisper_content_is_hidden_regardless_of_input`) pins this; reverting the privacy branch trips it.

If you ever need to investigate harassment reports without a code change, the right move is to add an `EventKind::ChatWhisperContent` and route it to a separate audit channel with much stricter access — not to soften this guard.

## Live reload

The config file is watched via [`notify`](https://docs.rs/notify). On
change → debounce 150 ms → re-parse → validate → atomically swap into the
`ArcSwap<Config>` that the sender + layer read. Parse failures keep the
previous config in place and log a `warn!` (so a typo while editing
doesn't take the server offline).

Manual reload: `ConfigWatcher::reload()` — exposed on the runtime handle;
plumbed into an admin-api endpoint if you want to force a re-read without
touching the file mtime.

## Best-practices implemented

| Concern | Implementation |
|---|---|
| **Webhook secrets** | `${ENV_VAR}` substitution in URLs. Config file commits cleanly; secrets stay in env. |
| **Back-pressure** | Bounded mpsc (capacity 256). Full → drop with `SenderStats.dropped_full` counter; tick loop never blocks. |
| **Rate limiting** | Per-channel token bucket. Burst capped at 5 (Discord's per-webhook burst budget). |
| **HTTP retries** | 2× exponential (250 ms, 500 ms) on 5xx + network errors. **No retry on 4xx** (config bug — retry wouldn't help). |
| **429 handling** | `Retry-After` honoured in-task before the error bubbles. |
| **Recursion safety** | `DiscordLayer` filters its own emits (explicit `target: "cimmeria_discord"`) so HTTP-error tracing doesn't loop. |
| **Embed size limits** | Title 256, desc 4096, field-value 1024, total 6000 (`enforce_total_budget`). Truncations visible (`…`). |
| **Privacy** | Whisper body never posted (see above). |
| **Mockability** | `DiscordSender` trait; `MockSender` recorder; `HttpDiscordSender` production impl. Same pattern as the mercury `Transport` trait. |
| **Graceful shutdown** | `emit_server_shutdown` + 1 s drain before process exit. Beyond 1 s, drops are accepted. |
| **Panic visibility** | `install_panic_hook` posts `Event::ServerPanic` via synchronous `reqwest::blocking` with 2 s timeout before the default hook lets the process die. |
| **Self-observability** | `SenderStats` counters (enqueued / sent / filtered / dropped_full / dropped_closed / dropped_rate_limit / retried / 429d / failed). |

## Configuration

Path: `config/discord.toml` (override with `DISCORD_CONFIG_PATH`).

Example: [`config/discord.toml.example`](../../config/discord.toml.example).

Missing file → Discord silently disabled. Present-but-invalid file → server
fails to start with a clear error (typo guard).

## Wiring at emit sites

Two strategies, depending on the event type:

**For new emit sites, use the typed helpers:**

```rust
cimmeria_discord::emit_player_login(account_id, Some(character_name), addr);
cimmeria_discord::emit_player_world_entry(account_id, name, world_name, pos);
cimmeria_discord::emit_mission_completed(name, mission_id, mission_name);
// ...etc.
```

Helpers live in [`crates/discord/src/lib.rs`](../../crates/discord/src/lib.rs); add a new one alongside the existing pattern when you add a new permanent emit seam.

**For existing `warn!`/`error!` sites with structured fields**, do nothing — the tracing layer already harvests them into `Event::TracingEvent` automatically. The [negative-logging convention](negative-logging-convention.md) (`reason=`, `entity_id=`, `rows_affected=`, etc.) is what gives those tracing events their structure; the embed builder reads the fields into the embed's `fields` array.

## Emit-site coverage

Wiring `emit_*` calls into the server is incremental (issue #527). Current state:

| Channel | Live emit sites | Notes |
|---|---|---|
| `lifecycle` | `ServerStartup`, `ServerShutdown`, `ServerPanic` | from `server/src/main.rs` |
| `auth` | `PlayerLogin`, `PlayerLogout`, `PlayerDisconnect`, `PlayerAuthFailed` | login/logoff/teardown in `base/`; auth-fail in `auth/handlers.rs` |
| `world` | `PlayerWorldEntry`, `PlayerWorldExit` | entry in `play_character.rs`; exit on gate travel |
| `gameplay` | `PlayerLevelUp`, `ItemUsed`, `MissionAccepted`, `MissionCompleted`, `MissionFailed`, `PlayerDeath`, `PlayerRespawn` | level-up/item-used base-layer; mission/death/respawn cell-side (see name cache below) |
| `errors` | `Warning`/`Error` (harvest), `WireFormatError`, `DbError`, `MercuryTimeout` | decode/db/peer-silence seams in `base/` + `auth/` |
| `gm` | `GmCommand` | `.`-console dispatch in `cell/console/mod.rs` |
| `ops` | — | **deferred**: needs measurement infra |

**`player_disconnect` is the single choke point.** Every teardown path
(`logoff`, `inactivity_timeout`, `send_error`, `duplicate_login`,
`client_disconnect`) funnels through `base::helpers::destroy_client_entities`,
which maps the stable label to a typed [`DisconnectReason`] via
`DisconnectReason::from_label`. A clean logoff fires *both* `PlayerLogout`
(gameplay-level) and `PlayerDisconnect { reason: Clean }` (connection-level) —
by design.

**Cell-side name cache.** The cell service has no character/GM display name of
its own — names live in the base `ConnectedClientState`. `GmCommand` and the
cell-side gameplay events (`MissionAccepted/Completed/Failed`, `PlayerDeath`,
`PlayerRespawn`) read `CellEntity::character_name`, which is threaded in from the
base via `BaseToCellMsg::InitPlayerState` at world entry. Emits fall back to
`entity:<id>` if the name isn't cached yet. (Mission embeds carry no mission
*name* — `MissionDefEntry` has none cell-side — only the id.)

### Deferred seams and why

- **`LootGenerated`**: loot is rolled onto an NPC corpse at death
  (`cell/abilities/loot_drop.rs`); the *looter* isn't known until someone takes
  it, so there's no single character to attribute the generation to. Needs a
  decision on whether to attribute to the killer or the looter before wiring.
- **`GmTeleport` / `GmSpawn` / `GmItemGrant`**: the GM teleport/spawn/give command
  *execution* is still TODO in `game/src/commands/gm_cmds.rs` — there's no
  resolved position/template/quantity to put in the typed embed yet.
- **`MissionRewardGranted`**: reward dispatch isn't implemented cell-side (no
  reward catalog; see `cell/console/mission.rs`).
- **`AssertionFailure`**: no explicit assertion-failure log site exists today;
  invariant violations surface as generic `error!` and are caught by the
  `errors` harvest.
- **`ops` channel** (`HighLatency`, `PacketLossSpike`, `MemoryWarning`,
  `TickStall`, `AoiBurstWarning`, `OutboxLag`): each needs a measurement +
  threshold loop (RSS sampling, tick-duration timing, RTT thresholding) that
  doesn't exist yet. Tracked separately.

[`DisconnectReason`]: ../../crates/discord/src/event.rs

## Operations

- **Stats**: `SenderStats { enqueued, sent, filtered, dropped_full, dropped_closed, dropped_rate_limit, retried, rate_limited_429, failed }`. Available via `cimmeria_discord::global().unwrap().stats()`.
- **Force reload**: `cimmeria_discord::global().unwrap().reload()`.
- **Disable at runtime**: edit `discord.toml`, set `enabled = false`, save. File watcher picks it up on the next event.
- **Disable per-event at runtime**: edit `discord.toml`, set the toggle false, save. No restart required.

## Deployment

### Local dev

Copy [`config/discord.toml.example`](../../config/discord.toml.example) to `config/discord.toml` and fill in webhook URLs (either inline or via `${ENV_VAR}` interpolation — the crate substitutes from the process environment at parse time). Missing file → Discord disabled (soft-fail). Invalid TOML or unset `${VAR}` references → hard-fail with exit code 2.

### Colo (containerised release)

The deployment path is a Compose overlay generated by [`.github/workflows/release-container.yml`](../../.github/workflows/release-container.yml):

1. Webhook URLs live as GitHub Actions secrets (`DISCORD_LIFECYCLE_WEBHOOK`, `DISCORD_ERRORS_WEBHOOK`) on the source repo.
2. On every release the workflow renders [`docker/compose.discord.yml`](../../docker/compose.discord.yml) — substituting the `__DISCORD_*_WEBHOOK__` sentinels in the inlined TOML with the secret values — and attaches the rendered file to the GitHub release as `compose.discord.yml`.
3. The colo operator downloads the rendered overlay alongside `compose.yml` and runs `docker compose -f compose.yml -f compose.discord.yml up -d`. The overlay mounts the substituted TOML at `/opt/cimmeria/config/discord.toml` inside the container via Docker Compose's `configs:` mechanism, owned by the `cimmeria` user with mode 0440.

The colo never holds a `.env` file with webhooks — the rendered overlay carries the URLs in the inlined TOML. The compose overlay file itself becomes a secret-bearing artifact on the colo host (`chmod 0600`).

Channel-by-channel: a `[discord.channels.X]` block whose corresponding GH Actions secret was unset at render time is stripped from the rendered TOML entirely. Channels not in the rendered file are silently dropped from routing — see [`should_post`](../../crates/discord/src/config.rs).

See [colo-deploy.md → Discord notifications](../operations/colo-deploy.md#optional-discord-notifications) for the operator-facing runbook.

## Testing

- Unit tests in `crates/discord/src/` (41 tests; covers formula, embed shape, truncation, rate limiter, retry/429 handling, layer harvest, recursion guard, whisper privacy).
- `MockSender` for tests that need to assert wire bytes without HTTP.
- Wire-format tests for the embed JSON shape — title/description/field caps + `total_chars ≤ 6000` enforcement.

## Adding a new event type

1. Add a variant to `EventKind` in `event.rs` + corresponding `Event` variant.
2. Add the matching field to `EventToggles` + default + `is_enabled` arm.
3. Route it in `router.rs::channel_for` (no `_` fallback — must be explicit).
4. Add a `format_event` arm in `embed.rs`.
5. Add a typed helper in `lib.rs` for emit-site authors.
6. Document it in `config/discord.toml.example`.

The `event_kind_all_matches_variant_count` test pins the count so step 2 not getting done trips a test failure immediately.
