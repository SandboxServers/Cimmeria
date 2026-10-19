---
title: "Persistent player data runbook"
type: how-to
audience: operators
last_updated: 2026-10-18
---

# Persistent player data runbook

How to run the container's database so accounts and characters survive restarts and releases: what is kept, how to update, where the backups are, and what to do when a start fails or you need to restore. For the setup of the host itself, see [colo-deploy.md](colo-deploy.md); for the image and its environment, see [container.md](container.md). The design and decisions (D1-D8) are in the [campaign ledger](../analysis/persistent-player-data/README.md).

> **Verification status.** The shell scripts under `docker/` (`db-sync.sh`, `db-rebuild.sh`) are proven in CI only, by [`tools/container-persistence-smoke.sh`](../../tools/container-persistence-smoke.sh). No command in this runbook has been run on the colo. Each command block says **untested on the colo** until an operator runs it there and removes the mark.

The colo host is Linux and the commands below are for a shell on it (`docker compose` in `/opt/cimmeria`, the layout in [colo-deploy.md](colo-deploy.md#files-on-the-host)). If you drive the host from a Windows workstation, run them through `ssh <host> '<command>'`, not as local PowerShell.

## What persists

| Kept across restart, reboot and image update | Rebuilt from the image's seeds on every image update |
|---|---|
| Accounts, characters and everything hanging off them: inventory, mail, guilds, missions, contacts, bank, the other tables in [`db/persistent/persistent-public-tables.txt`](../../db/persistent/persistent-public-tables.txt) | The `resources` schema (items, worlds, missions, abilities, dialogs, loot and the rest) and the `public` tables in [`db/persistent/reseed-public-tables.txt`](../../db/persistent/reseed-public-tables.txt) (today: `shards`) |

Three rules sit under that table:

- **Restart keeps everything** (D2). `docker restart cimmeria`, a Docker restart and a host reboot change nothing in the database.
- **Seed accounts are topped up** (D6). On a rebuild, a seed account whose id and name are both absent from the database is inserted again. An existing account row is never changed. A seed account whose id or name is already taken by a different row is not inserted; it is logged as `skip_seed_account`. If you deleted a seed account on purpose, the next image update brings it back.
- **Nothing here wipes player data** (D3). A forced rebuild and a restore both keep players. The only full wipe is removing the Docker volume by hand ([below](#the-docker-volume)).

A rebuild is blue/green: it builds `sgw_next` from the image's seeds, copies the player tables across, repairs references the new resources broke, checks row counts, and swaps by renaming `sgw` to `sgw_prev` and `sgw_next` to `sgw`. The live `sgw` is renamed, never dropped. A reference to an item or world that the new resources removed is handled by the reconcile step: orphaned rows are deleted with the full row logged to `cimmeria_meta.reconcile_log`, and a player standing in a removed world is moved to `Castle_CellBlock` (D5).

Schema changes to the persistent tables are additive only (D-PP7). Anything else is an owner call.

## Normal update and restart

You do nothing special. Watchtower pulls a new `latest-prerelease`, the new container starts, and `db-sync` sees a different image id and runs a rebuild (a backup first). A restart with the same image skips the rebuild.

The first boot of a fresh volume is different: there is no player data to keep, so the image's baked database is used as it stands and its seed accounts are the only accounts. The first persistent deploy starts fresh this way (D4).

Expect a longer start after an image update. The image's `HEALTHCHECK` has a 300 s start period and s6 has no startup wait limit, so a slow rebuild is not killed. A rebuild logs a `[db-rebuild] step N` line per step ([below](#reading-the-db-sync-log-lines)).

Compose edits still need `docker compose up -d` ([colo-deploy.md](colo-deploy.md#what-happens-on-every-update)). That recreates the container and restarts the game server, but the database is kept.

## Backups

`db-sync` dumps `sgw` (`pg_dump -Fc`) before every rebuild, adoption and restore, and keeps the newest `CIMMERIA_DB_BACKUP_KEEP` files (default 10, D7). A restart that skips the rebuild writes none.

| Setting | Meaning | Default |
|---|---|---|
| `CIMMERIA_BACKUP_DIR` | Host directory bound to `/var/backups/cimmeria` in the container. Set it in `.env`. | `./backups` (relative to `/opt/cimmeria`) |
| `CIMMERIA_DB_BACKUP_KEEP` | How many backups to keep. A non-numeric value or one below 1 stops the start. | `10` |

Files are named `sgw-<UTC yyyymmddThhmmssZ>-<old image id>.dump`, or `...-pre-meta.dump` for a database from before this feature. Each is written as `.part` and renamed when complete, so a `.dump` file is whole.

List them, newest first (**untested on the colo**). From inside the container this works whatever `CIMMERIA_BACKUP_DIR` is set to in `.env`:

```bash
docker exec cimmeria ls -lt /var/backups/cimmeria
```

On the host the same files are in `CIMMERIA_BACKUP_DIR` from `.env` (`/opt/cimmeria/backups` when it is unset). Copy the newest one off the host now and then. The backup directory sits on the same disk as the volume, so it protects against a bad rebuild, not against losing the disk.

During a crash loop on the same failing image, `db-sync` reuses the newest backup (`reusing backup <path>` in the log) instead of writing a new one each time, so the loop does not push your older backups out.

## Restore

**Disaster recovery only.** A restore replaces the live database with a backup; everything written since that backup is gone from `sgw`. Do not use it for a normal deploy (D4). A bad image update does not need it: the previous database is kept as `sgw_prev`, and the pre-rebuild backup is in the backup directory.

1. Pick the file in the backup directory (see [Backups](#backups)). Note its name.
2. Set the name in `.env` (**untested on the colo**):

   ```bash
   cd /opt/cimmeria
   sed -i '/^CIMMERIA_DB_RESTORE=/d' .env
   echo 'CIMMERIA_DB_RESTORE=sgw-20261018T120000Z-<old image id>.dump' >> .env
   ```

   A bare name is looked up in the backup directory. A value containing `/` is used as a path inside the container.
3. Recreate the container so it reads `.env` (**untested on the colo**):

   ```bash
   docker compose up -d
   docker logs cimmeria 2>&1 | grep '\[db-sync\]'
   ```
4. Wait for `[db-sync] restore <file>` and a healthy container (`docker ps --format 'table {{.Names}}\t{{.Status}}'`).
5. **Remove the line from `.env`** (**untested on the colo**):

   ```bash
   sed -i '/^CIMMERIA_DB_RESTORE=/d' .env
   docker compose up -d
   ```

How it works: `db-sync` first backs up the current `sgw` (a restore never destroys the only copy), restores the file into `sgw_restore`, records the file name in `cimmeria_meta.restore_done` inside it, then swaps it in by rename (`sgw` becomes `sgw_prev`). The marker stops the same file from restoring again on the next start, so step 5 is housekeeping, not a safeguard. The file must exist, or the start stops with `restore file not found`.

A restore does not rebuild the resources in the same start. The restored database carries the resources, and the image id, of the image that made the backup, so the server runs on those until the next start. If the running image is newer, that next start (step 5's `docker compose up -d`) sees the different image id and rebuilds the resources by itself, keeping the restored players; you do not need `CIMMERIA_DB_FORCE_REBUILD`. The marker also means that restoring the same file name a second time does nothing: use a copy under another name.

## Forced resources rebuild

To rebuild the resources without a new image (for example after a failed rebuild, or to re-apply the seeds), set `CIMMERIA_DB_FORCE_REBUILD=1` for one start. Player data is kept (D3). Only the literal values `1`, `true`, `yes`, `TRUE` and `True` count. Do not write `CIMMERIA_DB_FORCE_REBUILD=0` to turn it off; remove the line instead.

(**untested on the colo**)

```bash
cd /opt/cimmeria
echo 'CIMMERIA_DB_FORCE_REBUILD=1' >> .env
docker compose up -d
docker logs cimmeria 2>&1 | grep '\[db-sync\] rebuild'     # reason=forced
sed -i '/^CIMMERIA_DB_FORCE_REBUILD=/d' .env
docker compose up -d
```

Leaving the variable set rebuilds on every start. The second `docker compose up -d` also restarts the game server; do it in a quiet hour, or leave the variable in until the next planned restart.

## Reading the db-sync log lines

`db-sync` runs once per container start, after Postgres and before the game server, and logs one decision line. Read them with (**untested on the colo**):

```bash
docker logs cimmeria 2>&1 | grep -E '\[db-sync\]|\[db-rebuild\]'
```

| Line starts with | What happened | Player data |
|---|---|---|
| `[db-sync] first-boot image_id=<id>` | A new volume booting the image's baked database. State flips from `baked` to `live`. | None yet |
| `[db-sync] skip image_id=<id>` | Same image as last time: a restart, crash or reboot. Nothing changed. | Kept |
| `[db-sync] rebuild reason=image-changed\|forced old=<id> new=<id>` | A backup was taken, then the resources were rebuilt. `[db-rebuild] step N` lines follow, ending in `swapped; previous database kept as sgw_prev`. | Kept |
| `[db-sync] adopt (no cimmeria_meta; keeping player data)` | The volume holds a database from before this feature (no `cimmeria_meta`). It is backed up and rebuilt, keeping its players. | Kept |
| `[db-sync] restore <file>` | `CIMMERIA_DB_RESTORE` named a backup that had not been restored yet, and it is now `sgw`. | From the backup |
| `[db-sync] backup <path>` | A backup was written. Precedes `rebuild`, `adopt` and `restore`. | n/a |
| `[db-sync] reusing backup <path>` | A retry of a failed rebuild reused the newest backup. | n/a |
| `[db-sync] ERROR ...` | The start failed closed ([next section](#fail-closed-behavior)). | `sgw` untouched |

A `skip` line may add `(an earlier rebuild to <id> failed; set CIMMERIA_DB_FORCE_REBUILD=1 to retry)`. That is the image running now matching the database, while an earlier start failed to rebuild to a different image (for example after you rolled back to the previous image). The server runs on the old resources. The note is reported once and then cleared.

## Fail-closed behavior

If anything in `db-sync` fails (a backup that cannot be written, a rebuild step, a count mismatch, a restore file that is missing), it logs `[db-sync] ERROR <cause>`, exits non-zero and leaves `sgw` as it was. The container then stops (`S6_BEHAVIOUR_IF_STAGE2_FAILS=2`) and `restart: unless-stopped` retries it. The game server never starts on a half-built database (D7).

On a failed rebuild, the old database records `last_failed_image_id` and `last_failed_at` in `cimmeria_meta.db_state`. The `_next` database is dropped.

What to do:

1. Read the cause (**untested on the colo**):

   ```bash
   docker logs cimmeria 2>&1 | grep -E '\[db-sync\] ERROR|\[db-rebuild\] ERROR'
   ```
2. Match it:
   - `bad CIMMERIA_DB_BACKUP_KEEP`: fix the value in `.env`, `docker compose up -d`.
   - `postgres not ready after 120s`: Postgres did not come up. Read the container log above the line for the Postgres error.
   - `restore failed, sgw untouched`: the backup file is unreadable or damaged. Try another file. `sgw` is as it was.
   - `backup failed` or `backup rename failed`: the backup directory is full or not writable. Free space or fix permissions on `CIMMERIA_BACKUP_DIR`, then `docker compose up -d`.
   - `restore file not found`: the name in `CIMMERIA_DB_RESTORE` is wrong. Fix or remove it.
   - `[db-rebuild] ERROR <cause>`: a bug in the rebuild or a data problem the reconcile cannot repair. The old database is intact. Pin the previous image tag in `compose.yml` to get players back in, and report the cause line. Do not delete the volume.
3. Do not clear the failure by wiping the volume. The old `sgw` still holds every player.

While the container is stopped, the database is intact. Watchtower will try the next image when one is pushed.

## The Docker volume

The database lives on the named, external volume `cimmeria-server-pgdata` (mounted at `/var/lib/postgresql/data`). It is **external**, so `docker compose down -v` never removes it, and watchtower's `--remove-volumes` (which removes only the replaced container's anonymous volumes) leaves it alone.

Create it once, before the first `docker compose up -d` (**untested on the colo**):

```bash
docker volume create cimmeria-server-pgdata
```

An empty volume starts the first-boot path: the entrypoint copies the image's baked database into it. An existing cluster is kept as it is.

**Never run `docker volume prune` blindly.** It can remove volumes that no container uses, and a removed `cimmeria` container (for example between `docker compose down` and `up`, or during a hand-run `docker rm`) leaves `cimmeria-server-pgdata` unused. What a prune takes depends on the Docker version (recent engines take anonymous volumes only unless `--all` is given), so do not rely on that. Check what it would take first (**untested on the colo**):

```bash
docker volume ls --filter dangling=true
```

[colo-deploy.md](colo-deploy.md#operating-it) mentions pruning leftover anonymous volumes; prune those by name, not with a blanket command. Take a backup copy off the host before any volume work.

Do not confuse it with `cimmeria-pgdata`, which `setup.ps1 -UseDocker` creates for a developer's local database. The two are separate on purpose.

**The deliberate full wipe** is removing the volume. It deletes every account and character. Copy the newest backup off the host first, then (**untested on the colo**):

```bash
cd /opt/cimmeria
docker compose down
docker volume rm cimmeria-server-pgdata
docker volume create cimmeria-server-pgdata
docker compose up -d
```

The next start is a first boot: the seeds only, with the seed accounts.

## Inspecting the database state

For questions like "when was this last rebuilt" or "what did the last reconcile delete" (**untested on the colo**):

```bash
docker exec cimmeria psql -h 127.0.0.1 -U w-testing -d sgw -c \
  "SELECT state, image_id, rebuild_count, last_rebuild_at, last_failed_image_id FROM cimmeria_meta.db_state"
docker exec cimmeria psql -h 127.0.0.1 -U w-testing -d sgw -c \
  "SELECT logged_at, action, table_name FROM cimmeria_meta.reconcile_log ORDER BY id DESC LIMIT 20"
```

`reconcile_log.action` is one of `delete_dangling_item`, `relocate_player`, `count_dangling_ref`, `sequence_fixup`, `insert_seed_account` or `skip_seed_account`. The `detail` column holds the full deleted row, so a deleted item can be reinstated by hand from it.

## Related

- [colo-deploy.md](colo-deploy.md): host setup, updates, ports.
- [container.md](container.md): image, environment table, volume reference.
- [Campaign ledger and decisions](../analysis/persistent-player-data/README.md): why it works this way (D1-D8).
- [`docker/.env.example`](../../docker/.env.example): the four database settings, commented.
- [`tools/container-persistence-smoke.sh`](../../tools/container-persistence-smoke.sh): the CI scenario that walks first boot, skip, forced rebuild, reconcile, a failed rebuild, adopt, restore and a full wipe against its own scratch volume.
