# SGW Client UI Layout Inventory

**150 layout files** across 4 categories. Each module is defined by a `.toc` file that declares which layouts and Lua scripts to load.

Legend:
- **ACTIVE** = Referenced by `.toc` with `<Enabled>true</Enabled>`
- **DISABLED** = Referenced by `.toc` with `<Enabled>false</Enabled>`
- **SUB-LAYOUT** = Not in a `.toc` directly, but used as a template/child by an active module's Lua
- **UNUSED** = Not referenced anywhere — dead/prototype files
- **CEGUI STOCK** = Shipped with the CEGUI library, not game-specific

---

## CEGUIData/layouts/ (9 files — CEGUI stock + root)

| File | Status | Description |
|------|--------|-------------|
| `SGWRoot.layout` | SUB-LAYOUT | Root window for the CEGUI system, loaded by engine |
| `VanillaConsole.layout` | SUB-LAYOUT | In-game developer console |
| `Demo7Windows.layout` | CEGUI STOCK | CEGUI demo/test |
| `Demo8.layout` | CEGUI STOCK | CEGUI demo (only one loaded from Lua: `demo8.lua`) |
| `FontDemo.layout` | CEGUI STOCK | CEGUI font test |
| `TabControlDemo.layout` | CEGUI STOCK | CEGUI tab demo |
| `TabPage.layout` | CEGUI STOCK | CEGUI tab page demo |
| `VanillaWindows.layout` | CEGUI STOCK | CEGUI vanilla test |
| `test.layout` | CEGUI STOCK | CEGUI test |

---

## Startup/ (9 files — login flow)

### Login
| File | Status | Description |
|------|--------|-------------|
| `Login/Login.layout` | **ACTIVE** | Username/password fields, server dropdown, login/quit/options buttons, version text |

### EULA
| File | Status | Description |
|------|--------|-------------|
| `EULA/EULA.layout` | **ACTIVE** | End User License Agreement acceptance screen |

### Server Select
| File | Status | Description |
|------|--------|-------------|
| `ServerSelect/ServerSelect.layout` | **ACTIVE** | Multi-column list (server name, population, load), select/back buttons |
| `ServerSelect/ServerSelect_new.layout` | UNUSED | Redesign attempt — empty widget names, never wired |

### Character Select
| File | Status | Description |
|------|--------|-------------|
| `CharacterSelect/CharacterSelect.layout` | **ACTIVE** | 8 character slots (name, level, archetype, location each), play/create/delete/back/change server/addons buttons, selected char highlight |
| `CharacterSelect/CharacterSelect_new.layout` | UNUSED | Redesign with `ComplexFrame_2` — empty names, only 2 slots. Never wired |
| `CharacterSelect/SGWUI_CharacterSelect.layout` | UNUSED | Oldest prototype using TaharezLook skin. Monolithic: includes char select list, faction selection screen (SGU/Praxis with logos), AND full char creation panel (name, archetype radios, gender, head/torso/hand/foot/leg combos, rotate/zoom). Completely different widget names from production |

### Character Create
| File | Status | Description |
|------|--------|-------------|
| `CharacterCreate/CharacterCreate.layout` | **ACTIVE** | Faction buttons (SGU/Praxis), archetype buttons (6 per faction), gender, 3 name schemes (two-name, one-name for Asgard, name+suffix for Goa'uld/Jaffa), 8 appearance groups with prev/next, skin tint, rotate/zoom, create/back/random buttons |
| `CharacterCreate/CharacterCreate_new.layout` | UNUSED | Redesign — empty names, not wired |

---

## Common/ (9 files — shared utilities)

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `AddOns/AddOns.layout` | **ACTIVE** | AddOns | Add-on manager: enable/disable UI mods, load out-of-date toggle |
| `Background/Background.layout` | **ACTIVE** | Background | Root background window, custom drag-drop handler support |
| `ColorPicker/ColorPicker.layout` | **ACTIVE** | ColorPicker | Color picker widget (used by chat customization etc.) |
| `Development/Development.layout` | **ACTIVE** | Development | Developer debug window (Ctrl+Alt+D), FPS/memory/entity stats |
| `FlashManager/FlashManager.layout` | **ACTIVE** | FlashManager | Container for Flash/SWF-based UI (minigames use Flash) |
| `MoviePlayer/MoviePlayer.layout` | **ACTIVE** | MoviePlayer | In-game video playback |
| `Options/Options.layout` | **ACTIVE** | Options | System options: video, audio, gameplay, keybindings tabs |
| `Performance/Performance.layout` | **ACTIVE** | Performance | Performance overlay (Shift+F): FPS, render stats |
| `Popup/Popup.layout` | **ACTIVE** | Popup | Tooltip/hover popup container |
| `Prompt/Prompt.layout` | **ACTIVE** | Prompt | Modal dialog box (OK/Cancel/Yes/No). Instantiated 5x via `<LayoutInstance>` with prefixes Prompt1_ through Prompt5_ for stacking |

*Note: `Bindings.toc` and `Global.toc` exist but have no layout — they're script-only modules (keybind definitions, global constants/localization).*

---

## Core/ — In-Game HUD & Systems (123 files)

### Combat

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `AutoAttack/AutoAttack.layout` | **ACTIVE** | AutoAttack | Auto-attack toggle indicator |
| `CoverIndicator/AutoAttackButton.layout` | SUB-LAYOUT | CoverIndicator | Cover system auto-attack button (loaded by CoverIndicator module) |
| `CoverIndicator/ReloadButton.layout` | SUB-LAYOUT | CoverIndicator | Cover system reload button |
| `DeploymentBar/DeploymentBar.layout` | **ACTIVE** | DeploymentBar | Deployment ability cooldown bar |
| `Reload/Reload.layout` | **ACTIVE** | Reload | Weapon reload progress bar |
| `SCT/SCT.layout` | **ACTIVE** | SCT | Scrolling Combat Text — damage/heal numbers floating above targets |
| `Target/Target.layout` | **ACTIVE** | Target | Target unit frame: name, health bar, level, faction |
| `SelfStatus/SelfStatus.layout` | **ACTIVE** | SelfStatus | Player health/shield bars, level, combat state |
| `SelfStatus/SelfStatusAttackQuarters.layout` | **ACTIVE** | SelfStatus | Directional damage indicator (hit direction) |
| `SelfStatus/SelfStatusBlip.layout` | SUB-LAYOUT | SelfStatus | Individual damage direction blip widget |
| `PlayerDefeat/SGWUI_PlayerDefeat.layout` | **ACTIVE** | PlayerDefeat | Death/defeat screen — uses TaharezLook (older style, but active!) |
| `Duel/Duel.layout` | **ACTIVE** | Duel | PvP duel request/accept UI |

### Weapons & Action Bars

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `WeaponBar/WeaponBar.layout` | **DISABLED** | WeaponBar | Weapon slot bar — **disabled in .toc** |
| `WeaponBar/WeaponBar_new.layout` | UNUSED | — | WeaponBar redesign v1 |
| `WeaponBar/WeaponBar_NEW2.layout` | UNUSED | — | WeaponBar redesign v2 |
| `ActionButtons/ActionButtonWin.layout` | **ACTIVE** | ActionButtons | Main action button bar container (holds up to 60 buttons, 5 layers) |
| `ActionButtons/ActionButton.layout` | SUB-LAYOUT | ActionButtons | Individual action button template |
| `ActionButtons/ActionButtonDrag.layout` | **ACTIVE** | ActionButtons | Action button drag ghost |
| `ActionButtons/ActionProfileEdit.layout` | **ACTIVE** | ActionButtons | Action bar profile editor |
| `Bandolier/Bandolier.layout` | SUB-LAYOUT | Bandolier | Weapon bandolier (5 weapon slots, F1-F5) |
| `Bandolier/BandolierDrag.layout` | **ACTIVE** | Bandolier | Bandolier drag item |
| `Bandolier/Bandolier_NEW.layout` | UNUSED | — | Bandolier redesign |

### Abilities & Progression

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Ability/Ability.layout` | **ACTIVE** | Ability | Ability tree window — all learned/available abilities |
| `Ability/AbilityDragItem.layout` | **ACTIVE** | Ability | Ability drag ghost for placing on action bar |
| `Trainer/Trainer.layout` | **DISABLED** | Trainer | Ability trainer NPC interaction — **disabled in .toc** |
| `Trainer/Trainer_NEW.layout` | UNUSED | — | Trainer redesign |
| `DisciplineTrainer/DisciplineTrainer.layout` | **ACTIVE** | DisciplineTrainer | Crafting discipline tree (separate from ability trainer) |
| `DisciplineTrainer/Column.layout` | SUB-LAYOUT | DisciplineTrainer | Discipline tree column template |
| `DisciplineTrainer/Square.layout` | SUB-LAYOUT | DisciplineTrainer | Discipline tree node/square template |
| `Character/Character.layout` | **ACTIVE** | Character | Character sheet: stats, equipped gear, level/XP |
| `ExpBar/ExpBar.layout` | **ACTIVE** | ExpBar | Experience bar |
| `Effect/BeneficialEffects.layout` | **ACTIVE** | Effect | Buff icon tray |
| `Effect/HarmfulEffects.layout` | **ACTIVE** | Effect | Debuff icon tray |

### Inventory & Economy

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Inventory/Inventory.layout` | **ACTIVE** | Inventory | Inventory grid, money display |
| `Inventory/InventoryDragItem.layout` | **ACTIVE** | Inventory | Item drag ghost |
| `Inventory/InventoryExamine.layout` | **ACTIVE** | Inventory | Item tooltip/examine popup — stats, description |
| `Loot/Loot.layout` | **ACTIVE** | Loot | Loot window from kills/containers |
| `Trade/Trade.layout` | **ACTIVE** | Trade | Player-to-player trade window |
| `Trade/TradeLocalItem.layout` | SUB-LAYOUT | Trade | Your offered item slot template |
| `Trade/TradeRemoteItem.layout` | SUB-LAYOUT | Trade | Their offered item slot template |
| `Vendor/Vendor.layout` | **ACTIVE** | Vendor | NPC vendor buy/sell interface |
| `Vault/Vault.layout` | **ACTIVE** | Vault | Personal bank/vault storage |
| `Vault/VaultDragItem.layout` | **ACTIVE** | Vault | Vault item drag ghost |
| `BlackMarket/BlackMarket.layout` | **ACTIVE** | BlackMarket | Auction house — search, browse, buy/sell |
| `BlackMarket/BlackMarket_ItemRow.layout` | SUB-LAYOUT | BlackMarket | Auction listing row template |
| `BlackMarket/BlackMarket_YourAuctionRow.layout` | SUB-LAYOUT | BlackMarket | Your active auction row template |

### Crafting

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Crafting/Crafting.layout` | **ACTIVE** | Crafting | Main crafting window container with tab pages |
| `Crafting/CraftingPage.layout` | SUB-LAYOUT | Crafting | Crafting recipes tab |
| `Crafting/AlloyPage.layout` | SUB-LAYOUT | Crafting | Alloy combining tab |
| `Crafting/ResearchPage.layout` | SUB-LAYOUT | Crafting | Research/experimentation tab |
| `Crafting/ReverseEngineeringPage.layout` | SUB-LAYOUT | Crafting | Reverse engineering (disenchant) tab |
| `Crafting/CraftingButtonDrag.layout` | **ACTIVE** | Crafting | Crafting result drag ghost |

### Missions & Dialog

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `MissionLog/MissionLog.layout` | **ACTIVE** | MissionLog | Mission/quest journal |
| `MissionReward/MissionReward.layout` | **ACTIVE** | MissionReward | Mission completion reward selection |
| `MissionTimer/MissionTimer.layout` | **ACTIVE** | MissionTimer | Timed mission countdown |
| `MissionTracker/MissionTracker.layout` | **ACTIVE** | MissionTracker | On-screen objective tracker |
| `MissionTracker/MissionTrackerStep.layout` | SUB-LAYOUT | MissionTracker | Individual objective step template |
| `Dialog/Dialog.layout` | **ACTIVE** | Dialog | NPC dialog conversation window |
| `Dialog/Dialog_NEW.layout` | UNUSED | — | Dialog redesign |
| `Dialog/Blurb.layout` | **ACTIVE** | Dialog | NPC speech bubble / blurb |
| `Dialog/Blurb_NEW.layout` | UNUSED | — | Blurb redesign |
| `Dialog/LatestDialogWindow.layout` | SUB-LAYOUT | Dialog | Most recent dialog reference |
| `Dialog/RadioLureButton.layout` | **ACTIVE** | Dialog | Radio lure interaction button (archeologist ability) |
| `Dialog/RealizationButton.layout` | **ACTIVE** | Dialog | Realization interaction button (archeologist ability) |
| `Dialog/Tutorial.layout` | **ACTIVE** | Dialog | Tutorial popup window |
| `Dialog/TutorialButton.layout` | **ACTIVE** | Dialog | Tutorial trigger button |

### Social & Communication

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `ChatWindow/ChatWindow.layout` | SUB-LAYOUT | ChatWindow | Chat window with tabs (loaded by ChatWindow module) |
| `ChatWindow/ChatWindow_NEW.layout` | UNUSED | — | Chat window redesign |
| `ChatWindow/ChatTab.layout` | SUB-LAYOUT | ChatWindow | Individual chat tab template |
| `ChatWindow/ChatTabConfigure.layout` | **ACTIVE** | ChatWindow | Chat tab filter configuration |
| `Social/Social.layout` | **ACTIVE** | Social | Friends/ignore list |
| `Social/Social_MemberRow.layout` | SUB-LAYOUT | Social | Friend list member row template |
| `Social/Social_MemberRow_Edit.layout` | SUB-LAYOUT | Social | Friend edit row template |
| `Social/Social_GroupRow.layout` | SUB-LAYOUT | Social | Friend group row template |
| `Social/Social_GroupRow_Edit.layout` | SUB-LAYOUT | Social | Group edit row template |
| `Social/Social_AddMemberRow.layout` | SUB-LAYOUT | Social | Add friend row template |
| `GateMail/GateMailInbox.layout` | **ACTIVE** | GateMail | In-game mail inbox |
| `GateMail/GateMailRead.layout` | **ACTIVE** | GateMail | Read mail view |
| `GateMail/GateMailCreate.layout` | **ACTIVE** | GateMail | Compose new mail |
| `Greet/Greet.layout` | **ACTIVE** | Greet | Emote/greeting interaction |
| `PlayerSearch/PlayerSearch.layout` | **ACTIVE** | PlayerSearch | Search for players by name/level/archetype |

### Squad & Teams

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Squad/Squad.layout` | **ACTIVE** | Squad | Squad/party member frames (up to 6 members) |
| `Squad/SquadMember.layout` | SUB-LAYOUT | Squad | Individual squad member frame template |
| `UnitFrames/UnitFrames.layout` | **ACTIVE** | UnitFrames | Generic unit frame container |
| `UnitFrames/UnitFrame.layout` | SUB-LAYOUT | UnitFrames | Individual unit frame template |
| `Team/CreateTeam.layout` | **ACTIVE** | Team | Create custom team (command macros) |
| `Team/TeamEditor.layout` | **ACTIVE** | Team | Team command editor |
| `Team/TeamVault.layout` | **ACTIVE** | Team | Team shared vault |
| `Team/TeamVaultDragItem.layout` | **ACTIVE** | Team | Team vault drag ghost |
| `Organization/Organization.layout` | **ACTIVE** | Organization | Guild/organization window |
| `Organization/EditMember.layout` | **ACTIVE** | Organization | Edit guild member rank/notes |

### Pets & Minions

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Pet/PetInfo.layout` | **ACTIVE** | Pet | Pet stats/info panel |
| `Pet/PetLocations.layout` | **ACTIVE** | Pet | Pet location management |
| `Pet/PetContainer.layout` | SUB-LAYOUT | Pet | Pet action bar container template |
| `Pet/PetDefaultContainer.layout` | SUB-LAYOUT | Pet | Default pet container layout |
| `Pet/PetContainerDrag.layout` | **ACTIVE** | Pet | Pet ability drag ghost |
| `Pet/PetDrag.layout` | **ACTIVE** | Pet | Pet slot drag ghost |

### Commands (Custom Macros)

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Command/CommandVault.layout` | **ACTIVE** | Command | Command macro library |
| `Command/CommandEditor.layout` | **ACTIVE** | Command | Macro editor (icon, script, name) |
| `Command/CreateCommand.layout` | **ACTIVE** | Command | New macro creation dialog |
| `Command/CommandVaultDragItem.layout` | **ACTIVE** | Command | Command drag ghost |

### Navigation & World

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `MiniMap/Minimap.layout` | **ACTIVE** | MiniMap | Minimap with compass, zoom buttons |
| `MiniMap/MinimapButton.layout` | SUB-LAYOUT | MiniMap | Minimap button template (zoom, world map, etc.) |
| `MiniMap/MinimapMissionDir.layout` | SUB-LAYOUT | MiniMap | Mission direction indicator on minimap |
| `WorldMap/WorldMap.layout` | **ACTIVE** | WorldMap | Full world map overlay |
| `WorldMap/WorldMap_LayerWidget.layout` | SUB-LAYOUT | WorldMap | Map layer toggle widget |
| `WorldMap/WorldMap_MissionWidget.layout` | SUB-LAYOUT | WorldMap | Mission marker widget on world map |
| `DHD/DHD.layout` | **ACTIVE** | DHD | Stargate DHD (Dial Home Device) — gate address dialing interface |
| `SpaceQueue/SpaceQueue.layout` | **ACTIVE** | SpaceQueue | Zone/instance queue waiting screen |
| `SpaceQueue/SpaceQueueReady.layout` | **ACTIVE** | SpaceQueue | Queue ready/accept popup |

### Minigames

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Minigame/Minigame.layout` | **ACTIVE** | Minigame | Minigame container (hosts Flash .swf games) |
| `Minigame/MinigameObserver.layout` | **ACTIVE** | Minigame | Spectator view for minigames |
| `StartMinigame/StartMinigame.layout` | **ACTIVE** | StartMinigame | Minigame start/join dialog |
| `StartMinigame/MinigameHelpRegister.layout` | **ACTIVE** | StartMinigame | Register as minigame helper |
| `StartMinigame/MinigameHelpRequestCreate.layout` | **ACTIVE** | StartMinigame | Create help request for minigame |
| `StartMinigame/MinigameHelpRequestPending.layout` | **ACTIVE** | StartMinigame | Pending help request status |
| `StartMinigame/MinigameHelpRequestQuery.layout` | **ACTIVE** | StartMinigame | Query available minigame help |
| `StartMinigame/MinigameHelpSettings.layout` | **ACTIVE** | StartMinigame | Minigame helper preferences |

### HUD Chrome

| File | Status | Module | Description |
|------|--------|--------|-------------|
| `Access/Access.layout` | **ACTIVE** | Access | Main menu bar / access panel (ESC menu) |
| `SelfWindow/SelfWindow.layout` | **ACTIVE** | SelfWindow | Player portrait/nameplate |
| `SplashText/SplashText.layout` | **ACTIVE** | SplashText | Zone name / area discovery text splash |
| `SplashText/SplashHack.layout` | **ACTIVE** | SplashText | Alternative splash text (workaround) |

---

## CookedPC/UI/ (compiled assets — NOT editable text)

| Path | Description |
|------|-------------|
| `CEGUIData/packages/SGW_UI.upk` | Compiled CEGUI widget skins/imagesets (UE3 package) |
| `CEGUIData/packages/TestComponents.upk` | Test widget package |
| `Flash/*.upk` | Flash minigame SWFs as UE3 packages: Activate, Alignment, Analyze, Bypass, Converse, ConverseBasicHumanoid, CrystalGame, DHD, GoauldCrystals, Hack, Livewire |

---

## Summary

| Category | Total | Active | Disabled | Sub-layout | Unused |
|----------|-------|--------|----------|------------|--------|
| CEGUIData | 9 | 0 | 0 | 2 | 7 (stock demos) |
| Startup | 9 | 5 | 0 | 0 | 4 |
| Common | 9 | 9 | 0 | 0 | 0 |
| Core | 123 | 71 | 2 | 33 | 8 |
| **Total** | **150** | **85** | **2** | **35** | **19** |

### Disabled Modules (2)
- **Trainer** — Ability trainer NPC. Has `Trainer_NEW.layout` redesign too. Functionality may have moved to DisciplineTrainer.
- **WeaponBar** — Weapon bar. Has TWO redesign attempts (`_new`, `_NEW2`). Bandolier system may have replaced it.

### Unused Layouts (19)
- 7 CEGUI stock demos
- 4 Startup `_new.layout` redesigns (ServerSelect, CharSelect, CharCreate) + `SGWUI_CharacterSelect.layout` prototype
- 8 Core `_NEW` redesigns: WeaponBar (x2), Bandolier, ChatWindow, Dialog, Blurb, Trainer

### Flash Minigames (11 .upk packages)
Activate, Alignment, Analyze, Bypass, Converse, ConverseBasicHumanoid, CrystalGame, DHD, GoauldCrystals, Hack, Livewire
