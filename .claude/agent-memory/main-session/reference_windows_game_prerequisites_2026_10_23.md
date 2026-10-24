---
name: reference_windows_game_prerequisites_2026_10_23
description: Windows ARM archive-install incident, legacy DirectX detection and shared prerequisite setup admission
metadata:
  type: reference
---

# Windows game prerequisites — 2026-10-23

User report: on Windows 11 ARM on a Snapdragon system, installing from an
existing archive completed but Play opened no game window. Manually installing
the DirectX redistributable then reached the login screen (user screenshot).
The exact absent DLL was not established. Earlier telemetry ending at
`play_helper_spawn` is not proof that helper creation caused the failure.
The helper's invalid-request smoke returned exit code 1 without hanging.

Implementation reference: `crates/launcher/desktop/docs/prerequisites.md`
(Windows game prerequisite checks), with player steps in
`docs/client/launcher-guide.md` (Set up Windows game prerequisites).

- SGW imports x86 `d3dx9_40.dll` and `xinput1_3.dll`. Detect both in the game
  binaries or explicit x86 Windows system directory; on ARM request I386 via
  `GetSystemWow64Directory2W`. Inspect PE headers without loading DLL code.
- PhysX detection uses the x86 AGEIA registry view and registered 2.6.3 core;
  an installed modern PhysX product alone is insufficient.
- Install/adoption completion and Play recheck actual prerequisites. Native
  admission blocks missing dependencies; copying game files is insufficient.
- DirectX uses pinned Microsoft item 35 `dxwebsetup.exe`, 295,320 bytes,
  SHA-256 `2cf71d098c608c56e07f4655855a886c3102553f648df88458df616b26fd612f`.
  The downloaded pin had a valid Microsoft Authenticode signature on this date.
- Setup is an explicit action with vendor license and UAC interaction intact.
  Both setup flows hold a shared native mutation lease through vendor UI.
  It is the only install lock; the legacy PhysX path takes it too.
  Redetect afterward; installer exit zero alone is insufficient. Lost replies
  require inspection, never an automatic retry.
- Adopted/older copies can lack the retained authenticated PhysX installer.
  Explicit **Install legacy PhysX…** (engine `windows_physx::legacy`, no automatic fallback from Install) downloads NVIDIA Legacy MSI 9.13.0604, 20,154,368
  bytes, SHA-256
  `2b78df4a9004f5c4220d62c363709e5ac5e2e58df9f2284df0254cbfa4970d91`.
  The downloaded pin had a valid NVIDIA Corporation Authenticode signature.
  Reread/hash the staged MSI under deny-write/delete sharing before the same
  interactive elevated `/norestart` flow. A retained package hash mismatch is
  refused, not replaced by the fallback. Inspect/Play do not trigger network
  or installation. Download failure allows an explicit retry.

Evidence boundary: the user's manual repair verifies their own login screen,
not the new launcher setup flows or native ARM installer UAT. Consult the final
change's validation report for automated checks; none are asserted here.
