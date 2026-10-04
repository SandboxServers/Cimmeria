# Native launcher window UAT

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Companions:** [Requirements](../launcher-implementation-plan.md)
> **Last updated:** 2026-10-04
> **Code revision:** `0b10d869c869793ab506dbf9215ddb91714a244b`

## Observed

The development `.app` was rebuilt through the build lane and opened through
native computer use. The dark window rendered, displayed saved preferences,
opened its parented folder chooser, and preserved the selected directory and
consent when the chooser was cancelled. Existing saved summary consent was on;
this was not a fresh-state default-consent test, and the build has no exporter.

A build without compiled helper identities disabled Install with explicit
missing-helper feedback. Its development manifest key rejected live Patch Notes
with an authentication error. Rebuilding with the documented release public key
and the separately documented archive/prerequisite helper digests enabled
Install and rendered seven authenticated patch-note entries through native IPC.
The notes explicitly distinguished available patches from installed content.

The first real Install click displayed immediate confirmation-in-progress
feedback, then the observation timeout and Recheck status. No second Install
was issued. No operation journal existed at the time of investigation.

## Admission wait: native macOS permission

A one-second process sample showed the native task in
`NativeHost::install_command -> start_install_with -> admit_install_backend ->
fresh_destination -> std::fs::read_dir -> open$NOCANCEL`.
The macOS TCC log recorded `AUTHREQ_PROMPTING` for
`kTCCServiceSystemPolicyDocumentsFolder` at the same request. The saved test
destination was under Documents. This establishes a pending folder-access
decision, rather than a manifest fetch or completed installer failure.

The computer-use provider explicitly refused `UserNotificationCenter` access.
The operator resolved the system prompt. The original native request then wrote
its operation journal at revision 2 with state `running`. Clicking Recheck status
reconnected the UI to that same operation and its real download progress without
replaying Install. The same operation subsequently reached revision 3,
`Succeeded`, and the UI displayed “Game content is ready.” Pressing Continue
installation admitted prerequisite preparation, which reached revision 6,
`Succeeded`; the window displayed “Compatibility checked” and explicitly retained
the graphics/Play validation boundary. No second Install was dispatched.

## Evidence boundaries and next pass

The separate `npm run uat --prefix crates/launcher/desktop/frontend --
"$PWD/target/desktop/debug/examples/state_bridge"` pass succeeded against the
native persistence harness: fresh consent off, acknowledged save, process-restart
path/consent persistence, durable opt-out, literal patch-note fixture rendering,
chooser dismissal and a persisted checkbox change. It used isolated state and
did not alter the running app's saved consent. Its patch-note transport was a
fixture, and it did not exercise native installation or Wine.

Build jobs `20261004-103212-76171`, `20261004-103333-81670` and
`20261004-103542-85878` passed. The latter two produced development app bundles;
they are not clean-machine, offline-startup, signing or distribution validation.
The Tauri bundler emitted a static-CRT configuration deprecation warning.

This pass proves native content installation and prerequisite preparation. It
does not prove graphics/device initialization, repair, uninstall,
migration, updater parity, game rendering, login or world entry. Keyboard focus,
minimum-window sizing and integrated feature visuals still need their own pass.
Later worker changes require fresh checks of their changed surface.
