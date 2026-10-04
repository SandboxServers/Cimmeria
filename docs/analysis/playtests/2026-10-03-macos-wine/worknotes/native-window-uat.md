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

## Integrated Play bundle admission

Build job `20261004-110836-27950` produced the integrated Play development bundle
with pinned native Windows helper/client-patches and upstream D9VK. The initial
inspection waited on Documents access again: macOS logged that the existing
code requirement did not match the newly rebuilt development app and issued a
new `AUTHREQ_PROMPTING`. A process sample placed inspection in installed-owner
file reading. At that point Play had not been pressed; the operator then resolved the
protected prompt, allowing the first Play attempt described below.

## First Play and graphics diagnosis

After the second access decision, the unchanged app session reached Ready to
Play. One Play request observed a guest process and an early exit with code 3;
the UI reported the early exit and enabled a new explicit attempt. The patch
DLL log proved hook installation, not rendering. No telemetry DLL was loaded.

A separate controlled diagnostic, with the launcher closed and installation,
prefix, runtime-cache and launcher locks retained, reproduced the same exit.
Local bounded Wine stderr reported D9VK missing `VK_KHR_surface`. Selecting
only the bundled `lib/vulkan/icd.d/MoltenVK_icd.json` through `VK_DRIVER_FILES`
kept the game running. The operator reported an SGW login screen and fullscreen
minimization on focus loss. These are operator observations: the computer-use
provider rejected both Wine and SGW application names. Login/world entry has
not been confirmed. The new production environment guard is in `wine.rs` with
revert-verified tests; a rebuilt production-path Play pass is still required.

The operator subsequently closed the diagnostic game to adjust framerate. The
retained helper observed guest exit code 0 and itself exited 0, releasing its
ownership locks. This is a normal-exit observation, not a login/world result.
Further game launches and graphics-setting changes are deferred while the
operator makes those changes. The first local-lab CI build compiled the MCP
supervisor but failed because the injector command selected the library package;
the corrected workflow builds `sgw-start32` from `cimmeria-start32`. No lab DLL has
yet been injected and no live telemetry endpoint has been used.

The corrected native Windows lab-tools job passed in run `37217424712` at
`e259ace94`. Downloaded artifacts match the CI-recorded SHA-256 values: supervisor
`b6e432d59730154af438187094a0f6ce9dcb7a441d8411a4beb30ed8968ae87f`,
x86 injector `6e1207b853697def6b55b9ac73a3d15d7833afab91f98537777c06e6e76e5f6f`,
and lab DLL `193735c01301470f3ac1bbed52832394f8a60f53a16ef62b3fe8e0f28f0e31ce`.
They remain development-only artifacts and have not yet been injected. Windows
desktop job `111478341598` in run `37216040220` also passed at `590050084`,
including the previously corrected locked-owner launch path; it does not cover
the subsequent updater/adoption integration.
