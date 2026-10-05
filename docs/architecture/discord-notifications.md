# Discord notifications

> **Last updated**: 2026-10-04

The server posts structured events to Discord channels via webhooks for
development-time ops visibility — login bursts, world entry, errors,
panic, etc. Eight logical channels, 44 toggleable event types, live
reload, automatic warn/error harvest from the tracing layer.

Player IP addresses are deliberately **never** rendered in any embed — see
[Privacy](#privacy-whisper-content-is-always-hidden) below.

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
| `gameplay` | PlayerLevelUp, MissionAccepted, MissionCompleted, CharacterCreated, MinigameResult (others off) |
| `gm` | GmCommand, GmTeleport, GmSpawn, GmItemGrant |
| `errors` | Error, WireFormatError, DbError, AssertionFailure, MercuryTimeout (Warning off) |
| `ops` | HighLatency, PacketLossSpike, MemoryWarning, TickStall, AoiBurstWarning, OutboxLag |

The routing table is in [`crates/discord/src/router.rs`](../../crates/discord/src/router.rs) and is pinned by `every_channel_has_at_least_one_event` and `every_event_kind_routes` tests — adding an `EventKind` without choosing a channel is a compile error (no `_ =>` arm).

## Event types — full list

44 variants. Each has an explicit on/off toggle in `[discord.events]`.

```
Lifecycle:   server_startup, server_shutdown, server_panic
Auth:        player_login, player_logout, player_disconnect, player_auth_failed
World:       player_world_entry, player_world_exit
Chat:        chat_global, chat_say, chat_whisper, chat_guild, chat_team, chat_command
Gameplay:    player_level_up, player_death, player_respawn,
             mission_accepted, mission_completed, mission_failed, mission_reward_granted,
             loot_generated, item_used,
             character_created, npc_death, minigame_result, dialog
GM:          gm_command, gm_teleport, gm_spawn, gm_item_grant
Errors:      warning, error, wire_format_error, db_error, assertion_failure, mercury_timeout
Ops:         high_latency, packet_loss_spike, memory_warning, tick_stall,
             aoi_burst_warning, outbox_lag
```

Unknown toggle keys in the TOML are rejected at parse time (typo guard:
`playr_login = false` fails to load).

## Defaults — signal/noise tiers

The defaults in [`EventToggles::default`](../../crates/discord/src/config/mod.rs) prioritise signal:

- **High-signal, always on**: every lifecycle event, every auth event, every world event, every GM event, every ops alert, level-up, mission accept/complete, character creation, minigame result.
- **Low-signal, off by default but toggleable**: warning (noisy), all chat except global (volume + privacy), death/respawn (volume), mission failed/reward (per-event noise), loot/item-used (very noisy), chat command (every `/who` would post), npc_death + dialog (very high-volume during combat/questing).

## Privacy: whisper content is always hidden

`chat_whisper` posts the *fact* of a whisper (who → whom, when) but
**never** the message body — the embed always reads `[hidden]`. This is
enforced in [`embed::format_chat`](../../crates/discord/src/embed/format.rs) regardless of how the channel is configured. A test (`whisper_content_is_hidden_regardless_of_input`) pins this; reverting the privacy branch trips it.

If you ever need to investigate harassment reports without a code change, the right move is to add an `EventKind::ChatWhisperContent` and route it to a separate audit channel with much stricter access — not to soften this guard.

## Privacy: player IPs are never rendered

Events still carry the connection `SocketAddr` for internal correlation, but
[`embed/format.rs`](../../crates/discord/src/embed/format.rs) **never** writes it into an
embed field — login, disconnect, auth-failed, mercury-timeout, wire-format
error, and high-latency embeds all omit it. Identity is reported by account
name + id (and character name where known), not by IP. This keeps player IPs
out of Discord, which is a less-controlled surface than the server logs /
SigNoz where the addr is still available for debugging.

## No internal links

Discord embeds never link to SigNoz, the admin API or any other VPN-only
host: most of the team reading Discord can't open them
([Rule 6, "Discord"](instrumentation-discipline.md#discord)). The last
step of [`build_embed`](../../crates/discord/src/embed/builder.rs), for every
event type, runs [`embed/links.rs`](../../crates/discord/src/embed/links.rs)
over the whole rendered embed. Any `http://` or `https://` URL in a title,
description, field or footer is replaced by `[link removed]`, and a
URL-typed key (`url`, `icon_url`, ...) whose value is such a link is dropped.
The message itself still posts.

The allowlist, `ALLOWED_LINK_HOSTS`, is for public, team-reachable hosts and
is empty today. A host on it matches itself and its subdomains. A URL
that carries another URL in its path or query (`?next=https://...`, plain or
percent-encoded) is removed even when its own host is allowed.

The trace ID is the one SigNoz handle Discord keeps, as plain text in the
footer (`trace_id 4bf92f…`) that a developer pastes into SigNoz (D-NT3). A
`trace_id` or `span_id` that arrives as a link keeps only its own hex ID:
32 digits for the trace, 16 for the span (from `spanId=` first).
`signoz_url_in_a_field_value_renders_without_it` pins the guard.

## Naming in typed events

Every object a typed `Event` names is a `Named { id, name }` pair
(`event/named.rs`, NT-10), and every pair renders through one renderer,
`named` in `embed/format.rs`: `Name (#id)`, then `#id` when the name is
missing, the bare name when the ID is, and `?` when both are. It wraps the
same `name_with_id` the tracing path folds pairs with, so a pair reads the
same in every embed. Examples: `steve (#6)`, `Jaffa Guard (#9001)`,
`Castle_CellBlock (#4)`, `#2576`.

| Object | ID | Name, and where it comes from |
|---|---|---|
| Account | `account_id` | The login name (D-NT2), threaded from the login ticket (`PendingLogin.account_name` → `ConnectedClientState.account_name`), so no extra DB lookup happens at the Mercury login seam. |
| Character | `player_id` | The character name: `ConnectedClientState` on the base (`discord_account` / `discord_character`), `CellEntity` on the cell (`SpaceManager::discord_character`). `entity:<id>` before the cell has cached either. |
| NPC | cell `entity_id` | `CellEntity::npc_name`. A killer or GM target is a character when it has a `player_id`, an NPC otherwise (`SpaceManager::discord_entity`). |
| World | `world_id` | The world name. The base reverses it through `NameBook::world_id`; the cell through `SpaceManager::world_id_for_world`. |
| Mission, item type, dialog, template | the seed ID | The NameBook (`cimmeria_names::book()`). |
| Archetype | `EArchetype` ordinal | `cimmeria_names::archetype_name`. |
| Dialog choice | the cooked `ButtonID` | Only `-1` (a buttonless dialog closed) has a server-side name, `closed`. Button text lives in the client's `CookedDataDialogs.pak`, which the server does not index, so other buttons render `#id`. |
| Minigame | its name | The minigame catalogue is keyed by name; there is no numeric ID. The player is the `player_id` and name the base hands the minigame server at registration; "For chains" lists the victory chains the game was played for, as `#id` (chains have no name). |

Two kinds of field stay unpaired on purpose. `PlayerAuthFailed` carries only
the attempted login name: a rejected login may name no account, and the auth
path doesn't say which. Free-text fields (`cause`, `reason`, `source`, `args`)
are not objects.

`every_typed_event_renders_each_object_as_name_and_id` (`embed/pairing_tests.rs`)
builds every variant with sentinel names and IDs and asserts each `Name (#id)`
appears in the rendered embed. A new variant fails to compile until the
test file's exhaustive match has an arm for it, and the test fails until the
table covers every `EventKind`; neither check notices a new `Named` field on
an existing variant, so add its expected string to that variant's row.
`no_event_renders_the_player_ip` runs the same table against the IP rule
above.

The killer field is labelled from the death's `cause`, `Killer (player)` or
`Killer (NPC)`, because a player killer's `#id` is a `player_id` and an NPC
killer's is an `entity_id`. An `ItemUsed` target that is another player
renders as that player's character pair; any other target is `entity:<id>`.

## Naming in harvested warnings and errors

Every object in an embed renders as `Name (#id)`, or `#id` when its name
is unresolved ([Rule 6](instrumentation-discipline.md#rule-6--every-id-field-is-paired-with-its-name)).
The tracing layer posts a `warn!`/`error!` event's own fields, and
[`embed/tracing_fields.rs`](../../crates/discord/src/embed/tracing_fields.rs)
folds them before posting:

- **Who** comes first: `player_id`/`player_name` and
  `account_id`/`account_name` fold into one field,
  `Alice (#100) · steve (#6)`. Each half degrades on its own, so a line
  with only `account_id = 6` shows `#6`. A line not yet swept from
  `character_name` to `player_name` still folds.
- **Objects** come next, in field order: each ID key folds with its name
  key into one field under the key's prefix. `ability_id = 880` +
  `ability_name = "Staff Blast"` posts as `ability: Staff Blast (#880)`.
  The pairing follows Rule 6's key table, mirrored in
  [`embed/naming.rs`](../../crates/discord/src/embed/naming.rs): the default
  `<p>_id` → `<p>_name`, the bare entity keys (`target` → `target_name`), and
  the exceptions (`space_id` → `world` under `space`, `item_type_id` →
  `item_name`, `msg_id` → `msg_name`, `method_index` → `method_name`, ...).
  Change the doc's table and that file together.
- **The rest** come last, unchanged, then the event's log target under
  `Log target`.

An embed holds 25 fields, one of them the log target. When the folded
fields don't fit, the cut falls on the unpaired tail and the last slot
says `+N more fields`, so Who and every object pair survive. The marker never takes a pair's slot: when the pairs alone fill the embed, the unpaired fields are dropped without it.

## Muted accounts

`[discord] muted_accounts = ["lab"]` keeps an account's events out of every channel: the live research lab's account, or a test account running scripted repros. Entries are login names (any case) or numeric account ids. An event is muted when it names the account (`account_id` / `account_name`, including an `account_id` field on a harvested warning), or when it names a character that account was seen with. Many events carry only the character name, so the characters are learned from the events that carry both, such as login and world entry. Muted events count as `filtered` in the sender stats. Code: [`crates/discord/src/mute/`](../../crates/discord/src/mute/mod.rs).

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

**For new emit sites, use the typed helpers.** Every object is a `Named`
pair; pass what the seam has and let the renderer degrade:

```rust
use cimmeria_discord::Named;

cimmeria_discord::emit_player_login(Named::new(account_id, Some(login)), None, addr);
cimmeria_discord::emit_mission_completed(
    space_mgr.discord_character(entity_id),
    Named::new(mission_id, cimmeria_names::book().mission(mission_id).map(str::to_string)),
);
// ...etc.
```

Helpers live in [`crates/discord/src/emit.rs`](../../crates/discord/src/emit.rs); add a new one alongside the existing pattern when you add a new permanent emit seam.

**For existing `warn!`/`error!` sites with structured fields**, do nothing — the tracing layer already harvests them into `Event::TracingEvent` automatically. The [negative-logging convention](negative-logging-convention.md) (`reason=`, `entity_id=`, `rows_affected=`, etc.) is what gives those tracing events their structure; the embed builder reads the fields into the embed's `fields` array.

## Emit-site coverage

Wiring `emit_*` calls into the server is incremental. Current state:

| Channel | Live emit sites | Notes |
|---|---|---|
| `lifecycle` | `ServerStartup`, `ServerShutdown`, `ServerPanic` | from `server/src/main.rs` |
| `auth` | `PlayerLogin`, `PlayerLogout`, `PlayerDisconnect`, `PlayerAuthFailed` | login/logoff/teardown in `base/`; auth-fail in `auth/handlers.rs` |
| `world` | `PlayerWorldEntry`, `PlayerWorldExit` | entry in `play_character.rs`; exit on gate travel |
| `gameplay` | `PlayerLevelUp`, `ItemUsed`, `MissionAccepted`, `MissionCompleted`, `MissionFailed`, `PlayerDeath`, `PlayerRespawn`, `CharacterCreated`, `NpcDeath`, `MinigameResult`, `Dialog` | level-up/item-used base-layer; mission/death/respawn/npc-death cell-side (see name cache below); character-create in `base/character_create.rs`; minigame in `minigame/server/result_dispatch.rs` (player name handed over at registration); dialog in `cell/content/event_dispatch/dialog.rs` |
| `errors` | `Warning`/`Error` (harvest), `WireFormatError`, `DbError`, `MercuryTimeout` | decode/db/peer-silence seams in `base/` + `auth/`. **`movement.validation` warns are suppressed** — see below |
| `gm` | `GmCommand` | `.`-console dispatch in `cell/console/mod.rs` |
| `ops` | — | **deferred**: needs measurement infra |

**`player_disconnect` is the single choke point.** Every teardown path
(`logoff`, `inactivity_timeout`, `send_error`, `duplicate_login`,
`client_disconnect`) funnels through `base::helpers::destroy_client_entities`,
which maps the stable label to a typed [`DisconnectReason`] via
`DisconnectReason::from_label`. A clean logoff fires *both* `PlayerLogout`
(gameplay-level) and `PlayerDisconnect { reason: Clean }` (connection-level) —
by design.

**`movement.validation` is filtered from the harvest.** The speed/teleport
validator emits warn-only telemetry (`movement.speed_warning`,
`movement.validation_reject`) under `target: "movement.validation"`. It's
calibration data destined for SigNoz (to compute the legitimate p99.9 speed
before the speed layer is ever promoted to snap-back) and it fires during
normal play — sub-tick deltas produce huge / infinite implied-speed ratios.
[`DiscordLayer::on_event`](../../crates/discord/src/layer/mod.rs) drops this target
outright (same mechanism as the recursion guard) so it never floods the errors
channel; the data still flows to logs and SigNoz. Pinned by
`movement_validation_target_filtered`.

**Client telemetry replays are filtered from the harvest.** The upload ingest
re-emits each client-side row (the game DLL's events and the launcher's tailed
client logs) as a server log record under `client.native`,
`launcher.client_log`, `launcher.debug_log` or `launcher.session_meta`. Those are
the client's own warn/error stream, so `DiscordLayer::on_event` drops them
(`CLIENT_REPLAY_TARGETS`); a lab client in a repro campaign posted hundreds to
the errors channel on 2026-09-29. They still reach SigNoz. The server's own
ingest records (`launcher.ingest`, `launcher.bundle`) still post. Pinned by
`client_telemetry_replays_are_filtered`.

**Content-quality events are SigNoz-only.** `SIGNOZ_ONLY_EVENTS` lists
`(target, event)` pairs that stay WARN in the logs and SigNoz but never post.
The match is on the structured `event` field, so every other event on the
same target still posts. Each row is a data gap that repeats on every deploy
or every instance and has nothing new for an operator to act on:

| Target | Event | Why it is SigNoz-only |
|---|---|---|
| `spawner.npc_behaviour` | `spawn_off_mesh` | Seed-placement data, read from the SigNoz views. It is deduplicated per spawn id per process, but the colo restarts on every deploy and Castle_CellBlock is instanced per login, so the same spawns posted all day (90 posts in 7 days, 2026-09-29). |
| `abilities` | `effect_script_unregistered` | One row per boot for a known seed gap that a live-DB guard pins, so it posted once per deploy (12 in 7 days). |

Pinned by `signoz_only_events_are_filtered`, which also checks that another
event on the same target still posts. Add a row only for an event that is
data, not a fault; a fault that repeats needs fixing at its source.

**The logOff teardown race logs at DEBUG.** When a player logs off or
disconnects, the base unmaps the player's entity id at once, but the cell may
already have queued a tick of position relays for that player. Each of them
missed the address map and posted `AoI: no client addr for witness` with
`reason=entity_to_addr_miss`: 23 posts from one logOff on 2026-09-29. The two
teardown paths now record the departed witness
(`base-session` `helpers/departed_witnesses.rs`), and a miss for a witness
whose session ended in the last 30 s logs at DEBUG with
`reason=witness_session_ended`. A miss for a live witness still posts. This is
a fix at the source, not a Discord filter.

**Cell-side name cache.** The cell service has no character/GM display name of
its own — names live in the base `ConnectedClientState`. `GmCommand` and the
cell-side gameplay events (`MissionAccepted/Completed/Failed`, `PlayerDeath`,
`PlayerRespawn`, `Dialog`) read `CellEntity::character_name` and
`CellEntity::player_id`, which are threaded in from the base via
`BaseToCellMsg::InitPlayerState` at world entry. Emits fall back to
`entity:<id>` if neither is cached yet. Mission, dialog and template names come
from the NameBook, not from `MissionDefEntry`.

### Deferred seams and why

- **`LootGenerated`**: loot is rolled onto an NPC corpse at death
  (`cell/abilities/loot_drop.rs`); the *looter* isn't known until someone takes
  it, so there's no single character to attribute the generation to. Needs a
  decision on whether to attribute to the killer or the looter before wiring.
- **`GmTeleport` / `GmSpawn` / `GmItemGrant`**: GM teleport, spawn and give now
  execute through the client's native `/` console (#518) and the GM-gated `.`
  console in `crates/cell-console/src/cell/console/` (#523), but neither path
  calls the Discord helpers yet, so the typed embeds have no emit site.
- **`MissionRewardGranted`**: reward dispatch isn't implemented cell-side (no
  reward catalog; see `cell/console/mission.rs`).
- **`AssertionFailure`**: no explicit assertion-failure log site exists today;
  invariant violations surface as generic `error!` and are caught by the
  `errors` harvest.
- **`ops` channel** (`HighLatency`, `PacketLossSpike`, `MemoryWarning`,
  `TickStall`, `AoiBurstWarning`, `OutboxLag`): each needs a measurement +
  threshold loop (RSS sampling, tick-duration timing, RTT thresholding) that
  doesn't exist yet. Tracked separately.

[`DisconnectReason`]: ../../crates/discord/src/event/mod.rs

## Operations

- **Stats**: `SenderStats { enqueued, sent, filtered, dropped_full, dropped_closed, dropped_rate_limit, retried, rate_limited_429, failed }`. Available via `cimmeria_discord::global().unwrap().stats()`.
- **Force reload**: `cimmeria_discord::global().unwrap().reload()`.
- **Disable at runtime**: edit `discord.toml`, set `enabled = false`, save. File watcher picks it up on the next event.
- **Disable per-event at runtime**: edit `discord.toml`, set the toggle false, save. No restart required.

## Deployment

### Local dev

Copy [`config/discord.toml.example`](../../config/discord.toml.example) to `config/discord.toml` and fill in webhook URLs (either inline or via `${ENV_VAR}` interpolation — the crate substitutes from the process environment at parse time). Missing file → Discord disabled (soft-fail). Invalid TOML or unset `${VAR}` references → hard-fail with exit code 2.

### Colo (containerised release)

The container reads `/opt/cimmeria/config/discord.toml`. There are two ways to put it there; an operator uses one.

**Host file (the colo's route).** The operator writes `config/discord.toml` beside the compose files, readable by the container's `cimmeria` user (uid 1001, mode 0440), and adds [`docker/compose.discord-file.yml`](../../docker/compose.discord-file.yml) to `COMPOSE_FILE` in `.env`. That overlay bind-mounts the file, which survives watchtower's recreate. Any channels, muted accounts and event toggles work without a release. The webhook URLs then live on the colo in that file, which is why it is 0440 and never committed.

**Release-rendered overlay.** Generated by [`.github/workflows/release-container.yml`](../../.github/workflows/release-container.yml):

1. Webhook URLs live as GitHub Actions secrets (`DISCORD_LIFECYCLE_WEBHOOK`, `DISCORD_ERRORS_WEBHOOK`) on the source repo.
2. On every release the workflow renders [`docker/compose.discord.yml`](../../docker/compose.discord.yml), substituting the `__DISCORD_*_WEBHOOK__` sentinels in the inlined TOML with the secret values, and attaches the rendered file to the GitHub release as `compose.discord.yml`.
3. The operator downloads it beside `compose.yml`, adds it to `COMPOSE_FILE` in `.env`, and runs `docker compose up -d`. (An explicit `-f compose.yml -f compose.discord.yml` would replace `COMPOSE_FILE` and drop the other overlays.) The overlay passes the substituted TOML in `DISCORD_CONFIG_TOML`, and the container entrypoint writes it to `/opt/cimmeria/config/discord.toml`, owned by the `cimmeria` user with mode 0440. It is an environment variable rather than a compose `configs:` mount because watchtower's recreate keeps the environment but not a compose-copied file.

On this route the rendered overlay is the secret-bearing file on the host (`chmod 0600`), and it carries only the channels the workflow renders. Channel-by-channel: a `[discord.channels.X]` block whose corresponding GH Actions secret was unset at render time is stripped from the rendered TOML entirely. Channels not in the rendered file are silently dropped from routing — see [`should_post`](../../crates/discord/src/config/mod.rs).

See [colo-deploy.md → Discord notifications](../operations/colo-deploy.md#discord-notifications) for the operator-facing runbook.

## Testing

- Unit tests in `crates/discord/src/` (covers formula, embed shape, truncation, rate limiter, retry/429 handling, layer harvest, recursion guard, whisper privacy, the player-IP rule, `movement.validation` suppression, Rule 6 pair folding and field budget, the typed-event pairing table, and the no-internal-links guard).
- `MockSender` for tests that need to assert wire bytes without HTTP.
- Wire-format tests for the embed JSON shape — title/description/field caps + `total_chars ≤ 6000` enforcement.

## Adding a new event type

1. Add a variant to `EventKind` in `event/event_kind.rs` — **and to the
   `EventKind::ALL` array in the same file** — plus the corresponding `Event`
   variant in `event/payload.rs`.
2. Add the matching field to `EventToggles` (`crates/discord/src/config/toggles.rs`)
   + default + `is_enabled` arm.
3. Route it in `router.rs::channel_for` (no `_` fallback — must be explicit).
4. Add a `format_event` arm in `embed/format.rs` (or `embed/format_gameplay.rs`
   for a gameplay or GM event). Every object field is a `Named` and renders
   through `named`.
5. Add a row to `every_variant` in `embed/pairing_tests.rs` and an arm to its
   `variant_is_covered` match.
6. Add a typed helper in `emit.rs` for emit-site authors.
7. Document it in `config/discord.toml.example`.

The `event_kind_all_matches_variant_count` test pins the count so step 2 not getting done trips a test failure immediately.
