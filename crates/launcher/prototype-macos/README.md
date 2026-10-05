# Approved single-game launcher experience

Type: prototype reference. Audience: launcher contributors. Updated: 2026-10-03.
Companion: [launcher guide](../../../docs/client/launcher-guide.md).

This throwaway browser prototype records the approved visual and interaction
direction for Stargate Worlds. It is not a native Mac app or an implementation
of installation, telemetry, repair, or uninstall. Keep it on the prototype
branch; implement the production design in the existing launcher separately.

From the repository root, run:

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory crates/launcher/prototype-macos
```

Open `http://127.0.0.1:8765/?variant=A`. The bottom arrows switch between A
(split layout), B (Stargate home), and C (compact). The shared style and flows
were approved; no particular layout variant was explicitly selected.

## Approved behavior

- One launcher for one game: no game library or game selector.
- Dark-only appearance, retaining the blue-gray surfaces, cyan primary action,
  gate motif, and clear hierarchy shown here.
- One install screen transitions to Play. Installation phases are progress,
  not separate onboarding forms. Failures expose recovery without starting over.
- Telemetry opt-in/out stays visible on both tabs and with settings open. Default
  off; changing it while the game runs explicitly applies on the next launch.
- Patch Notes reads patch titles/descriptions from the launcher content manifest.
  Missing descriptions are identified rather than invented. It is a content
  list, not release chronology or proof of installed patches.
- Gear: local install directory, open folder, repair, and confirmed uninstall.
  Repair/uninstall are unavailable during installation or gameplay. Uninstall
  retains launcher preferences and does not delete server-side characters.
- On Windows, use Explorer and the actual Windows install path in place of the
  Mac-specific folder labels. Preserve the same design and interaction flow.

## Evidence and limits

`manifest.json` is the unmodified snapshot downloaded from the GitHub
`content-current` release during this review. Seven entries; three descriptions,
four missing descriptions. The prototype displays this local snapshot and does
not verify its signature. Production must use the launcher's signature-verified
manifest and existing failure handling, not this prototype fetch path.

The state module was exercised with Node: install-to-ready, repair/retry,
launch/return, tab switching without resetting progress, opt-in retention,
opt-out during gameplay, uninstall confirmation/cancel, preference retention,
and guards while running. Patch rendering was exercised with the downloaded
manifest and HTML-like text to check escaping. Initial A/B/C layouts were
visually inspected in the browser; later controls received logic checks.

All state lives in memory and resets on refresh. No real filesystem mutation,
telemetry transmission, game execution, native accessibility, or Wine integration
was covered. Prototype scenario controls are review tools, not production UI.
