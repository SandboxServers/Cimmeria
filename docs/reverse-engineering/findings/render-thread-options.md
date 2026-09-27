# Render Thread Options — which client shadow settings actually reach the renderer

> **Last updated**: 2026-09-21
> **Binary**: SGW.exe (32-bit x86 PE, MSVC; image base `0x00400000`)
> **Source files named by asserts**: `.\Src\RenderThreadOptions.cpp`, `.\Src\LaunchMisc.cpp`, `.\Src\SceneRendering.cpp`, `.\Src\RenderResource.cpp`
> **Companion docs**: [atrea-editor.md](atrea-editor.md) (the `Event_Option_*` family table), [address-map.md](../address-map.md)
> **Scope**: client-only. Nothing here touches the server, the wire, or any Cimmeria crate.

## TL;DR

`RenderThreadOptionManager` is the only native consumer of `Event_Option_Rendering`. It reads exactly eight system options, packs them into a 16-byte block, and ships the block to the render thread. Two of those eight — `maxShadowResolution` and `minShadowResolution` — are **dead**: they are read, cast to `int`, copied, and compared, but their accessor functions have zero references in the binary. Nothing renders differently because of them.

The shadow resolution that *does* matter comes from `BaseEngine.ini` (`MinShadowResolution` / `MaxShadowResolution`, read as `UEngine` config properties), and it is clamped against a shadow depth buffer whose size is a **static 1024** that nothing in the binary ever writes. The effective per-shadow ceiling is therefore `min(MaxShadowResolution, 1024) - 10 = 1014` texels. `MaxShadowResolution=2048` behaves identically to `1024`.

| Setting | Where it lives | Reaches the renderer? |
|---|---|---|
| `MaxShadowResolution` / `MinShadowResolution` | `BaseEngine.ini` | **Yes** — but capped at 1014 by the fixed 1024 depth buffer |
| `maxShadowResolution` / `minShadowResolution` | `SystemOptions.xml` / `SavedSystemOptions.xml`, group `internal` | **No** — read and discarded |
| `allowDynamicShadows` | same, group `internal` | **Yes** — gates `InitDynamicShadows` and the shadow depth render-target allocation |
| `allowFogVolumes` | same, group `internal` | Yes — fog-volume render targets and pass |
| `postprocessing`, `ppBloom`, `ppDepthOfField`, `ppMotionBlur` | same, group `video` | Yes |
| `ShaderModel` | same, group `video` | **Startup only** — read once in `LaunchMisc.cpp`; no `changeEvent`, so a change needs a client restart |
| `ShadowFilterQuality`, `ShadowFilterRadius`, `DepthBias`, VSM / Branching PCF toggles | `BaseEngine.ini` | Not system options at all; not covered by this pass |

## Why this was investigated

An external shadow / character-lighting handoff (2026-09-21) flagged that `SystemOptions.xml` declares `maxShadowResolution` and `minShadowResolution` as `optType='float'` while supplying only `defaultInt` / `defaultEditorInt`, and asked whether that mismatch was a real native bug. A stock local client independently showed both options persisted as `valueFloat="1"` in `SavedSystemOptions.xml` — consistent with the float default never being populated. The question was whether that bogus `1` reaches the renderer. It does not.

## The option path

### Game thread: `RenderThreadOptionManager::UpdateRenderThreadOptions` — `0x0057a440`

Identified by the `IsInGameThread()` asserts citing `.\Src\RenderThreadOptions.cpp` (lines `0x21`, `0x4a`) and by the render-command vtable symbol `` `RenderThreadOptionManager::UpdateRenderThreadOptions'::`l10'::UpdateOptions::vftable `` (`0x018403f4`) constructed at `0x0057a3b0`.

The singleton is `INSTANCE` at `DAT_01ee2ac8`, a 16-byte heap object created on first use by the getter at `0x0057b1e0` (`"INSTANCE"` assert, `RenderThreadOptions.cpp:0x53`). The constructor at `0x0057b160` subscribes the manager to `Event_Option_Rendering`. **The 16-byte object is itself the render-thread copy of the options block.**

The function looks up eight options by `(group, name)` and builds this block on the stack:

| Offset | Type | Option | Group | Also written to |
|---|---|---|---|---|
| `+0x0` | `bool` | `ppMotionBlur` (forced `0` when `postprocessing` is off) | `video` | `DAT_01db5900` |
| `+0x1` | `bool` | `allowDynamicShadows` | `internal` | `DAT_01db58f4` (game-thread global) |
| `+0x2` | `bool` | `allowFogVolumes` | `internal` | — |
| `+0x4` | `int` | `(int)minShadowResolution` | `internal` | — |
| `+0x8` | `int` | `(int)maxShadowResolution` | `internal` | — |
| `+0xC` | `bool` | `postprocessing` | `video` | — |

`ppBloom` → `DAT_01db5908` and `ppDepthOfField` → `DAT_01db5904` are written as game-thread globals only (both forced `0` when `postprocessing` is off); they are not in the block.

Each option's value is selected from one of three slots on the option object: editor (`+0xB1` bool / `+0xB4` float) when `FUN_00574430` reports editor mode, otherwise `+0xB2` / `+0xB8` when the flag byte at `option+0x61` is set, else `+0xB3` / `+0xBC`. Which of the last two is "saved value" and which is "default" was not resolved in this pass.

The block is then enqueued as an `UpdateOptions` render command (or executed inline when there is no render thread, `DAT_01ee2618 == 0`).

### Render thread: `UpdateOptions::Execute` body — `0x0057a330`

`IsInRenderingThread()` assert, `RenderThreadOptions.cpp:0x7f`. Compares the incoming block with the stored one field-by-field (`+0`, `+1`, `+2`, `+4`, `+8`), copies all 16 bytes over, and if anything differed calls `FRenderResource::UpdateRHI` (`0x005f5350`, assert `RenderResource.cpp:0x30`) on `GSceneRenderTargets` (`DAT_01ee6860`) — a full release + re-init of the scene render targets.

So changing `maxShadowResolution` or `minShadowResolution` *does* cause a render-target rebuild. That rebuild is the entire observable effect.

### The accessors, and the two with no callers

Six one-instruction accessors sit at `0x0057a2d0`–`0x0057a320`, 16 bytes apart:

| Address | Bytes | Returns | Callers |
|---|---|---|---|
| `0x0057a2d0` | `8B C1 C3` | `this+0x0` (motion blur) | `FSceneRenderTargets::InitDynamicRHI` `0x00950200`, `FSceneRenderer::Render` `0x00909ea0` |
| `0x0057a2e0` | `8D 41 01 C3` | `this+0x1` (allowDynamicShadows) | `0x00950200`, `0x009cbd00` |
| `0x0057a2f0` | `8D 41 02 C3` | `this+0x2` (allowFogVolumes) | `0x00950200`, `0x009c7180` (`FogVolumeRendering.cpp`) |
| `0x0057a300` | `8D 41 04 C3` | `this+0x4` (**minShadowResolution**) | **none** — no xrefs; Ghidra never created a function here |
| `0x0057a310` | `8D 41 08 C3` | `this+0x8` (**maxShadowResolution**) | **none** — no xrefs; Ghidra never created a function here |
| `0x0057a320` | `8D 41 0C C3` | `this+0xC` (postprocessing) | `0x00950200` |

`INSTANCE` (`DAT_01ee2ac8`) is referenced only from inside its getter, and the getter has exactly five callers: startup (`0x0041f620`), `0x00950200`, `0x00909ea0`, `0x009c7180`, `0x009cbd00`. All five were decompiled in full and none reads `+4` or `+8` inline — `0x009cbd00` (the per-light shadow/light pass) touches the block only through the `+0x1` accessor, behind the same `ShowFlags & 0x20` test described below. The two shadow-resolution fields have no consumer.

## Where shadow resolution really comes from

### The depth buffer is a fixed 1024

`FSceneRenderTargets::InitDynamicRHI` (`0x00950200`) allocates the shadow depth surfaces only when `allowDynamicShadows` (`+0x1`) is set, and sizes them `DAT_01e6ea4c × DAT_01e6ea4c`. `DAT_01e6ea4c` is static initialised data holding `0x00000400` (1024). Its only xrefs are reads — eight inside `InitDynamicRHI` and one in the getter `GetShadowDepthTextureResolution` at `0x0094fb50` (`return DAT_01e6ea4c;`). Nothing writes it.

### `FSceneRenderer::CreateProjectedShadow` — `0x009dde70`

```text
Min = LightSceneInfo->MinShadowResolution (+0x11C);  if < 1 → GEngine->MinShadowResolution  (GEngine + 0x360)
Max = LightSceneInfo->MaxShadowResolution (+0x120);  if < 1 → GEngine->MaxShadowResolution  (GEngine + 0x364)

Cap = GetShadowDepthTextureResolution() - 10          // 1024 - 10 = 1014  (SHADOW_BORDER * 2)
Max = min(Max - 10, Cap)
Min = min(Min, Cap)

Resolution = clamp(ScreenRadius * factor, Min, Max)   // per view; the largest view wins
```

The `GEngine` base pointer is currently mislabelled `g_pFlashExternalWindowModule` in the Ghidra project; the `+0x360` / `+0x364` fields are the `UEngine` config properties that `BaseEngine.ini` sets.

Consequences:

- Stock `MaxShadowResolution=512` → ceiling 502. Raising it to 1024 roughly doubles the ceiling (1014). This is the real gain available from the ini.
- `MaxShadowResolution=2048` → `min(2038, 1014)` = 1014. **Identical to 1024.** Any value above 1024 is inert.
- Going past 1014 would need the static dword at `0x01e6ea4c` patched (on disk or at runtime before `InitDynamicRHI`). Every reader goes through that one global — including all fourteen `TShadowProjectionPixelShader` / `FBranchingPCFProjectionPixelShader` parameter setters (five plain-PCF variants, nine Branching-PCF variants) and the shadow-depth render at `0x009dd480`, that call `0x0094fb50` for texel-size maths — so a single-dword patch is *plausibly* sufficient. **Untested**; it is a client patch and needs a maintainer decision before anyone tries it.

### `allowDynamicShadows` is genuinely wired

`UpdateRenderThreadOptions` mirrors the option into `DAT_01db58f4`. Its only reader is at `0x00907de7` inside `0x00906ea0` (called at the top of `FSceneRenderer::Render`; most likely `InitViews`):

```text
if ((ShowFlags & 0x20 /* presumably SHOW_DynamicShadows */) && DAT_01db58f4)
    InitDynamicShadows();            // 0x009de780
```

The handoff reported that `allowDynamicShadows=false` persisted across a restart yet a projected character shadow was still visible. The render-side gate above is unambiguous, so that observation is **not explained by this pass**. Two untested leads: the three-slot value selection described above (the `+0x61` flag may make the manager read a slot other than the one the saved file populates), or the visible shadow not being a dynamic projected shadow at all.

## `ShaderModel` is a startup-only option

The launch routine at `0x0041f620` (`.\Src\LaunchMisc.cpp`; asserts on `Core.System` / `XMLPath`) loads the system-options XML, calls `UpdateRenderThreadOptions` once, then reads `video` / `ShaderModel`:

- missing option → `4`
- value accepted only if it is one of `{0, 3, 4, 5}`; anything else is ignored
- the accepted value is passed to `FUN_00ec48b0`

`{0, 3, 4, 5}` matches the handoff's working menu mapping (3 High = 0, 2 High = 3, 3 Low = 4, 2 Low = 5). `RenderThreadOptionManager` never reads `ShaderModel`, and the option carries no `changeEvent` in `SystemOptions.xml`, so a change can only take effect on the next launch. The handoff's separate observation — that the *menu* saved `0` for both "3 Low" and "3 High" — is a UI/persistence question this pass did not examine.

## What this means in practice

1. **Do not tune `maxShadowResolution` / `minShadowResolution` in the XML files.** They do nothing. The `optType='float'` vs `defaultInt` mismatch in `SystemOptions.xml`, and the `valueFloat="1"` it leaves in `SavedSystemOptions.xml`, are harmless. Editing `SystemOptions.xml` to "fix" the defaults is unnecessary.
2. **`MaxShadowResolution=1024` in `BaseEngine.ini` is the useful ceiling.** `2048` is safe but buys nothing over `1024`.
3. **Shadow-map resolution is not a lever for the character-lighting defect.** With the ceiling pinned at 1014 regardless, the remaining Human Female face/sleeve problem has to be in the light-environment / appearance-compositing path (`CompositedAppearanceProxy::ApplyToPawn` at `0x00ec0840` and the jobs feeding it), which is still un-traced for lighting state.

## Confidence

| Claim | Confidence | Basis |
|---|---|---|
| `0x0057a440` is `RenderThreadOptionManager::UpdateRenderThreadOptions`; reads exactly these eight options | HIGH | Source-file asserts, vtable symbol, all eight wide-string literals in the decompile |
| 16-byte block layout | HIGH | Stack layout in `0x0057a440` + field compares in `0x0057a330` + accessor bytes |
| `min`/`maxShadowResolution` have no consumer | HIGH | Zero xrefs to `0x0057a300` / `0x0057a310`; `INSTANCE` referenced only in its getter; all five of the getter's callers decompiled in full |
| Shadow depth buffer is a fixed 1024 | HIGH | `DAT_01e6ea4c` static `0x400`, read-only xrefs |
| Effective per-shadow ceiling is 1014 | HIGH | Clamp arithmetic in `0x009dde70` |
| `GEngine+0x360/+0x364` are the `BaseEngine.ini` Min/MaxShadowResolution | MEDIUM | Matches stock UE3 `CreateProjectedShadow`; the ini → property binding itself was not traced |
| `ShaderModel` valid set `{0,3,4,5}`, startup-only | HIGH | Decompile of `0x0041f620`; no `changeEvent` in `SystemOptions.xml` |
| Meaning of each `ShaderModel` value | LOW | Taken from the external handoff; `FUN_00ec48b0` not decompiled |
| Patching `0x01e6ea4c` alone would raise the ceiling | LOW | Reasoned from xrefs only; untested |

## Open questions

- Which of option slots `+0xB8` / `+0xBC` (selected by `option+0x61`) is the saved value and which is the default. This decides whether a hand-edited `SavedSystemOptions.xml` value is what the manager reads — and may explain the `allowDynamicShadows=false` observation.
- `FUN_00ec48b0` — confirm the `ShaderModel` value → shader platform mapping.
- Where `ShadowFilterQuality`, `ShadowFilterRadius` and `DepthBias` are consumed. They are `BaseEngine.ini` keys, not system options, and were out of scope here.
- Light-environment / `ShadowParent` / shadow-flag handling in the appearance compositor — the actual lead for the character-lighting defect.
