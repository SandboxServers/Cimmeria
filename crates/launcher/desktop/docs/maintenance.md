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

## Settings flow and IPC

Restricted `InstallCommand::Uninstall` exposes removal through the native host.
Status supplies the saved installation ID, owned folder and recovery flag;
preferences do not choose the removal target. Settings opens an inline confirmation
showing the folder and explaining that files/modifications are removed while
preferences and shared compatibility resources remain. Dismissal makes no mutation.

Effect inspects native state before dispatch and observes the reply for up to
35 seconds. Lost replies do not replay removal. Explicit **Finish uninstall**
confirms recovery using the same operation ID. Install cancel, cleanup and
reconcile controls do not apply to Uninstall. Successful removal enables a fresh
Install when the chosen destination qualifies as empty.

Local engine/shell checks passed 257 tests with 11 ignored, including host disk
removal and consent preservation. All 28 frontend tests, type checking/build and
JS installation logic UAT passed. The UAT used fixture IPC to check confirmation,
dismissal, double-click protection, owned-folder targeting, acknowledged removal/
reinstall and unchanged consent; it does not prove native deletion. No app
was opened and no visual UAT is claimed; native Windows validation remains a gate.
