# Observation: screen-recording review (2026-10-24 12:22)

Source: `~/screenshots/CleanShot 2026-10-24 at 12.22.37 PM.mp4`. It is 52.4 s long, a 1046x824 region capture of the `Stargate Worlds` window (Wine/DXVK/MoltenVK, launched from the Cimmeria Launcher on macOS). It has no audio track.
Frames: `/tmp/sgw-video-review/sNN.jpg`, one per second (`s01` is t=0 s, `sNN` is t≈NN-1 s). The scene-change filter at 0.3 found no cuts, so the video is one continuous in-world session. Contact sheets: `grid-walk.jpg`, `grid-misc.jpg` and `grid36-44.jpg`.

## Timeline

| t (s) | Frame | What happens |
|---|---|---|
| 0 | s01 | In world, Castle_CellBlock stasis room. The character "Test Soldier" wears an orange jumpsuit and the yellow prison boot. The "Prison Boot Control" prop (GA-PuzzleStation) glows beside them. The chat shows the hold line twice: "Your prison boot is locked to the floor. Right-click the boot control beside you and win the Livewire game to work it loose." Quest tracker: "Arm Yourself!" |
| 1-14 | s02-s15 | The player runs off the anchor, toward the Organization Registrar and the green terminal wall (Airman Lance). |
| 15-17 | s16-s18 | The player is back beside the boot control. This matches the server snap-back of the cellblock boot lock (D-BL4; the client flag is staged off by D-BL5, so the client walks locally and is then snapped back). |
| 18 | s19 | The player right-clicks the prop and the Livewire window opens. For one frame it is a blank, translucent panel while the layout loads. |
| 19-33 | s20-s34 | The Livewire intro/briefing screen (DANGER panel, legend, red start button). The cursor hovers the tool column; the "Wire Cutter" tooltip shows (s25). |
| 33-42 | s35-s43 | The puzzle board shows (wire bundle). The wires look unchanged between frames, so no tool appears to have been applied. The centre indicator turns **red** at about t=40-42 (s41-s43). |
| 43 | s44 | The Livewire window closes. The boot is still on and the hold line is still in the chat, which fits a loss, cancel or close (F8: only a Victory runs the release chain). |
| 44-51 | s45-s52 | The player is self-targeted (orange brackets, plus a second "Test Soldier" frame, which is the target frame). The player-frame portrait now draws a face instead of the flat green disc seen in s01-s43. The player stands beside the prop; the lock is not released. |

## Rendering check

There are no black or missing textures, no flicker and no stretched or offset image. The UI is complete: hotbars, chat, minimap and Livewire panel all draw. The cursor lines up with the tooltips it triggers. The window fits on screen.

Minor details:

- A one-frame blank Livewire panel while it loads (s19). This is a cosmetic load delay.
- The player portrait is a flat green disc until about t=43, then becomes a face (s44). This is probably a deferred portrait render-to-texture. It is cosmetic and not confirmed as Wine-specific.
- The right edge of the HUD sits close to the window border; the minimap "+" button is partly cut off (s01). This is cosmetic.

## Logs

`Interface.log` in prefix `41e4cc59…/8a8fafb6…` was last written at 12:21:33, which is UI layout load, including `StartMinigame.layout`. It has the usual repeated CEGUI `There is no Property named 'Image'` errors and nothing written during the recording. `observation-windowed.md` does not exist yet. The server-side minigame result (loss, cancel or timeout) is not in any client log; it would be in the server/SigNoz minigame logs.

## Verdict

**Expected behaviour plus user action; this is not a rendering glitch.** The boot lock works as designed (#1952). The player cannot leave the anchor, and the Livewire game opens from the prop. The game was not won: no wire was cut, the indicator went red, and the window closed. So the lock stays on, and per the design that is the retry path. To get free, right-click the boot control again. Read the briefing, press the red button, then select a tool (for example Wire Cutter) and click the matching wire, following the legend (Goal, Inactive, Danger, Caution). Repeat until it shows Victory, which prints "The prison boot unlocks. You can move again."

Follow-up: if the red indicator came up with no player action (an instant timeout or auto-loss), confirm the session result in the server minigame log or SigNoz for this session around 12:23:15-12:23:20.
