# Installed-content maintenance

See the [desktop overview](../README.md) for installation and the
[Wine validation reference](wine-validation.md) for managed-runtime boundaries.
These native APIs do not imply game readiness or launch permission.

## Confirmed uninstall

`DesktopState::uninstall` requires explicit confirmation, an operation UUID, the
inspected journal revision and the saved installation ID. It verifies independent
installed-content identity before admitting removal. Only `game`, that install's
staging/cache directories, owner marker and content receipt are allowed at the
root; foreign top-level entries veto admission. Recursive checks reject links,
special files and Windows reparse points before deletion.

The owned root is renamed to sibling `.cimmeria-uninstall-<operation UUID>`.
A durable detachment checkpoint precedes removal; the owner marker is removed
last. The installed-content reference is forgotten before terminal success.
Errors retain a reconciliation gate rather than asserting removal succeeded.

Recovery requires another explicit confirmation with the same operation ID and
current revision. It handles rename-before-checkpoint and partial removal;
a missing detached tree is accepted only with the matching durable checkpoint.
A replacement folder at the original location is left untouched. There is no
cancellation after admission: a lost reply requires inspection, not automatic
mutation replay.

On macOS, a Wine installation first stops its original owned prefix using the
historical installation/helper identity and existing conservative stop checks.
Preferences, diagnostics consent, logs, signed evidence, managed runtime caches
and extraction prefixes remain. This removes owned installed content, not all
application data.

The API is not wired to shell IPC or frontend controls yet. Windows-native
validation and UI integration remain pending; no visual or gameplay result is
claimed.
