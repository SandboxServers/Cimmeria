# PP-S1 ruling (architect, 2026-10-25)

Input: [PP-S1.md](PP-S1.md) (integration commit `84272bf5e`; the same text as
`.claude/reports/launcher-linux-play-proton/PP-S1-findings.md`). Launch code
on `origin/main` (`85cc212f3`) is unchanged since the design:
`git log 0a25ef5d4..origin/main -- crates/launcher tools/launcher-release` is
empty, so the packet line numbers in the ledger still hold.

## Success criterion

PP-S1 passes its stop condition. Bare mode passes step 2 on all four Protons,
and **GE-Proton11-11 passes steps 3-6** (login screen, patches DLL, both DLLs,
login and world entry), with two changes the design did not have:

1. Native `d3dx9_40.dll` (and `xinput1_3.dll`) in the prefix's `syswow64`,
   with `WINEDLLOVERRIDES` `d3dx9_40,xinput1_3=n`. Wine's builtin `d3dx9_40`
   crashes SGW at `SGW.exe!0x00EC6F9D` (`ShaderRef != NULL`).
2. The launcher's windowed client profile (`windowedMode` true,
   `StartupResolution` written) in the prefix's `steamuser` Documents.
   Without it the window stays black until a focus change.

Failed: step 7's pass condition (the prerequisite worker answers
`failed/probe`, although the MSI returned 0 and the game then initialises
PhysX), and container mode (step 2: exit 1, no stdout, on SLR_4 and sniper).

Not tested:

| Item | Why it matters | Where it goes now |
|---|---|---|
| Steps 3-6 on Proton Experimental, Proton 11.0 and Proton 9.0 with the two changes | D-PP4's ranking past GE | PP-S2 step C |
| A first Play in a prefix with no client profile (the game writes the profile on its first run; the launcher only edits existing files) | the first Play would show the black window | PP-S2 step B, then PP-17 |
| The launcher's default profile (windowed 1024x768) rather than 1280x720 | the default most players get | PP-S2 step B |
| Fullscreen under Proton | D-PP13 offers it | PP-S2 step F; Linux is windowed-only until then |
| Whether native `xinput1_3` is needed | one DLL fewer | PP-S2 step D (optional); both stay native meanwhile |
| Why the probe fails | PP-08's acceptance | PP-S2 step A, then PP-08W |
| SD card (step 8), Game Mode (step 9) | U6 is a follow-up (OD-2) | PP-U |
| The AppImage itself (the spike used a debug build with a test `XDG_DATA_HOME`) | packaging | PP-U U1 |
| Whether 2-3 minute world entry also happens on Windows | performance, not Play | FU-7 |
| A prefix upgrade to another Proton keeping the native DLLs | Proton may rewrite `syswow64` | made moot: PP-16 checks the pinned hash on every Play |

## Rulings

| Decision | Ruling | Evidence |
|---|---|---|
| D-PP1 | **Confirmed.** `<root>/proton runinprefix <Z:\ worker>`; stdin and stdout reach the worker; the host process exits within 60 ms of `process_exited`; `wineserver` is gone within 2 s. | Steps 2-6, lifecycle rows |
| D-PP2 | **Confirmed; the setup target is `wineboot.exe`.** `proton run wineboot.exe`, exit 0 in 4-7 s, `pfx/system.reg` and DXVK `d3d9.dll` present. The `cmd.exe` fallback is not needed. | Step 1 ×4 |
| D-PP3 | **Re-specced: always bare.** Never wrap in `_v2-entry-point`. `ProtonInstall.runtime` stays in C-1 (PP-01 is unchanged) and is only logged; C-3 drops the container shape. Container stdio is FU-6. | Step 2 container column, ×4 |
| D-PP4 | **Order kept, evidence partial.** GE first is the only tested working Proton. The rest of the order stays provisional until PP-S2 step C; no packet waits on it. | Step 3 table, D-PP4 row |
| D-PP6 | **Confirmed.** DXVK `d3d9` from the prefix loaded native. | Step 3b load trace |
| D-PP7 | **Confirmed: PhysX is needed**, through the prerequisite worker's MSI. **The acceptance rule is unchanged**: only `probed` with `initialized_and_released` sets the stamp `Installed`. Taking `failed/probe` as success would launder a failed check. PP-08 ships as specified plus a stderr log; PP-S2 step A finds the failing call, and PP-08W fixes it. Until PP-08W lands, Linux Play stops at `proton_setup_failed` (PhysX). | Step 7 |
| D-PP13 | **Amended: Linux offers windowed only** (as macOS) until PP-S2 step F shows fullscreen renders. The frame cap and the prefix profile root stay. | Step 3 rows, black window with `windowedMode=false` |
| D-PP20 (new) | Linux Play installs the game's own native `d3dx9_40.dll` and `xinput1_3.dll` into `pfx/drive_c/windows/syswow64/`, extracted **natively** from `.cimmeria-prerequisites/DX9.0c/Nov2008_d3dx9_40_x86.cab` and `APR2007_xinput_x86.cab`, with size and SHA-256 pins on the DLL bytes, checked on every Play; the game's `WINEDLLOVERRIDES` gains `d3dx9_40=n;xinput1_3=n`. No `DXSETUP.exe` (exit 247 under Proton), no Wine `expand.exe` (fails), no redistributed Microsoft bytes in the AppImage. | Step 3b |
| D-PP21 (new) | The d3dx9_40 cab is **LZX** (window 21); cab_portable refuses LZX. Add the pure-Rust `lzxd` crate to `cimmeria-launcher-core` for a new **single-member** extract only. The install path (`expand_chain`) stays MSZIP/stored-only, and `lzx_and_quantum_folders_are_refused_by_name` stays as it is. | `7z l -slt`: `Method = LZX:21` (d3dx9_40), `MSZip` (xinput) |
| D-PP22 (new) | The display step writes the windowed profile into the prefix before every Linux Play (PP-17). What happens when no profile exists yet is decided after PP-S2 step B. | Black-window rows |

Pins, read on the Deck from the launcher-installed copy:

| File | Bytes | SHA-256 |
|---|---|---|
| `d3dx9_40.dll` (in `Nov2008_d3dx9_40_x86.cab`) | 4379984 | `16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b` |
| `xinput1_3.dll` (in `APR2007_xinput_x86.cab`) | 81768 | `8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7` |
| `Nov2008_d3dx9_40_x86.cab` | 1551228 | `ce8d54fedb855a0ca0ea7b3ee6e6b2e1dc5cd991232a4e59b9d28ad5a6439b34` |
| `APR2007_xinput_x86.cab` | 54318 | `fca066ad1912eb9698798e3a4e0b9723868411fe058775afe343f600eaea93cc` |

## What changes in the packets

- Re-specced: C-3 and PP-05 (bare only), PP-07 (DirectX step, profile hand-off),
  PP-08 (stderr log; acceptance unchanged), PP-11 (display moved out), PP-13
  (new sections), PP-U (new rows).
- New: PP-S2 (second Deck spike), PP-15 (single-member cab extract with LZX),
  PP-16 (native DirectX in the prefix), PP-17 (Linux display profile),
  PP-08W (worker probe fix, specced after PP-S2).
- New wave W1B, Linux-install quality from the 2026-10-25 Deck install test
  (owner-approved): PP-20..PP-25 in [work-packets-w1b.md](../work-packets-w1b.md).
- Owner decision raised: OD-3 (drop the third full read of the archive).
</content>
</invoke>
