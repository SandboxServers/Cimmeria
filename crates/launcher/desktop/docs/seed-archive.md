# Player seed archive

> **Type:** Reference
> **Audience:** Launcher contributors

See the [desktop overview](../README.md) for installation and the
[maintenance reference](maintenance.md) for installed-content identity. The
seed archive is a player-chosen copy of the signed seed RAR. The launcher
verifies it, then uses it in place of the seed download. It is never rehosted; see
[Rehosting](#rehosting).

## Where players choose it

Players choose the archive in two places: Settings, and the install area on the
Play tab, both behind the label `Use a downloaded game archive…`. The archive
is `Stargate Worlds (0.8348.1.4046) (2009-06-30) (beta).rar`, 4,135,724,034
bytes. Adoption uses the chosen archive too when it needs the seed, and its
review says `Uses your game archive`.

## Record and lazy check

A chosen archive is remembered in `seed-archive.json` under the launcher state
root. The record holds the verified file (path, size, SHA-256 and file
identity) and the signed seed SHA-256 it was checked against. The panel shows it as chosen only while that SHA-256 matches the cached release seed. Loading is
tolerant: a missing, unreadable or corrupt record is ignored, and the player
chooses again.

The record is only a choice. Nothing is re-checked when the launcher starts.
When Install, Repair or Update prepares a release, `DesktopState::seed_source_for`
compares the record's seed SHA-256 with the current release's seed SHA-256
(case-insensitively). A match applies the archive to the shared seed source. A
mismatch, including a record made for an older release, leaves the source in
download mode. The record is kept, so it applies again if a release with the
same seed is current later.

Verification runs on a blocking thread through `start_verify`, never on the UI
thread. It checks the file's size, then its SHA-256 with progress and
cancellation, then records the file identity. A replaced file is noticed
because the identity no longer matches.

## Copy, not move

The player's file is never moved, renamed or deleted. It is only opened for
reading when it is checked, and when a seed step needs it. The seed step copies
the archive to a scratch file beside the destination, re-checks identity, size
and SHA-256 while copying, fsyncs, and renames the copy into place only when
the hash matches. Only that scratch copy is created or removed.

## Switching mid-download

Choosing an archive while a seed download is running switches that step to the
archive. The step stops its download at the next chunk and removes the partial
download file. Later seed steps use the archive too.

Going back to download mode applies from the next seed step. A step that
already read the current source keeps its archive, so the switch is never
partial within one step. Later steps get a fresh switch token, so they are not
switched at once.

## Forget

`DesktopState::forget_seed_archive` returns the shared source to download mode
and removes `seed-archive.json`. The archive file itself is untouched. As with
a switch, forgetting applies from the next seed step: a seed step already
running keeps the archive it started with.

## Offer from Downloads

While no archive is chosen, the launcher looks for the seed in the top level of
the player's Downloads folder only. It offers a regular file whose name is the
signed seed's archive file name and whose size is exactly the seed's size.
Subfolders, links and other files are never offered. Only the name and size are
compared: nothing in the file is read or hashed before the player presses Yes.

Settings and the install area both show the offer with its exact name and size,
asking `Found <name> in Downloads. Use it?` with a Yes button. Yes runs the same
verification as choosing the file, so the file is checked before it is used. The
webview never receives a path, only the name and size.

The offer is cached for the session. Downloads is scanned on the first status
check after the panel mounts and again after Forget, not on every poll. Status
checks fetch the signed release only while no seed is cached and no fetch is running, and wait at least
a minute after a failed fetch. Accepting an offer always fetches the release,
because verification needs it.

## Rehosting

D-AI5: the seed is never rehosted. The launcher does not upload or offer the
player's archive to any service. It is read only on the player's machine, for
that player's own installation.

## Tests

`seed_archive/tests.rs` checks that a record for another release is ignored
after reopening. `install_uses_remembered_archive_without_downloading_seed`
dispatches an install with the seed download URL unroutable, so the install
succeeds only if the remembered archive is used.
