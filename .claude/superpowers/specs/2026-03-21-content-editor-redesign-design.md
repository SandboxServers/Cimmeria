# Content Editor Redesign: Celestial Monolith

**Date:** 2026-03-21
**Branch:** `feature/admin-tool-redesign-spike`
**Scope:** Reskin the Cimmeria Content Editor (`tools/ContentEditor`) with the "Celestial Monolith" design system. No new Rust backend commands. Existing editors (chains, scripts, data) retain their current behavior. Minor frontend state additions for HUD data and disconnect flow.

All file paths below are relative to `tools/ContentEditor/`.

---

## 1. Design System

### 1.1 Color Palette

Replace the current HSL CSS variables in `ui/src/index.css` with the Celestial Monolith tokens.

| Token | Hex | Usage |
|---|---|---|
| `surface` | `#131313` | App background, void |
| `surface-container-lowest` | `#0E0E0E` | Deepest wells, input backgrounds |
| `surface-container-low` | `#1C1B1B` | Floating stone slabs, cards |
| `surface-container` | `#201F1F` | Mid-level panels |
| `surface-container-high` | `#2A2A2A` | Raised control surfaces |
| `surface-container-highest` | `#353534` | Active states, hover backgrounds |
| `primary` | `#F2CA50` | Living Gold — CTAs, active nav, headlines |
| `primary-container` | `#D4AF37` | Weathered gold — button backgrounds |
| `on-primary` | `#3C2F00` | Text on gold surfaces |
| `secondary` | `#A7C8FF` | Lapis — linked items, secondary info |
| `secondary-container` | `#1F477B` | Lapis deep — nav active background |
| `tertiary` | `#7FDEDD` | Teal Energy — vial fills, accents |
| `on-tertiary-container` | `#004E4E` | Teal gradient endpoint |
| `on-surface` | `#E5E2E1` | Primary text (never pure white) |
| `on-surface-variant` | `#D0C5AF` | Muted text, secondary labels |
| `outline-variant` | `#4D4635` | Ghost borders (use at 15% opacity) |
| `error` | `#FFB4AB` | Destructive actions, failures |

Note: `surface-container-highest` and `surface-variant` from the reference mockups share the same hex (`#353534`). This is intentional — they serve different semantic roles (interactive state vs texture) but resolve to the same visual tone. Only `surface-container-highest` is kept as a token to avoid confusion.

### 1.2 Typography

Three-font stack replaces the current two-font setup:

| Role | Font | Package | Weight | CSS Variable | Usage |
|---|---|---|---|---|---|
| Headlines | Newsreader | `@fontsource-variable/newsreader` | 400, 700 | `--font-headline` | Section titles, page headers. Uppercase, letter-spacing 0.1-0.2em |
| Body / Labels | Space Grotesk | `@fontsource/space-grotesk` (existing) | 300-700 | `--font-sans` | UI labels, data readouts, nav items |
| Dense UI / Data | Inter | `@fontsource/inter` | 400-600 | `--font-dense` | Property values, dense text, inspector fields |

**Font registration in Tailwind v4** (`ui/src/index.css` `@theme` block):
```css
--font-headline: "Newsreader", serif;
--font-sans: "Space Grotesk", "Segoe UI", sans-serif;
--font-dense: "Inter", sans-serif;
--font-mono: "IBM Plex Mono", "SFMono-Regular", monospace; /* KEEP for connection strings, hex values */
```

**Monospace preservation:** `font-mono` stays in the CSS theme and continues to be used for connection strings (`ConnectDialog`), hex color values (`Inspector`), and other fixed-width data. IBM Plex Mono is kept for these usages. Inter (`font-dense`) replaces `font-mono` only in places that display proportional data like property names and inspector field values.

**Rules:**
- All headlines uppercase with wide tracking
- No pure white text — max is `on-surface` (`#E5E2E1`). Project-wide sweep: replace all `text-white` usages with `text-[#E5E2E1]` or the `on-surface` token.
- Display-scale headers next to label-scale readouts for "God-King" hierarchy

### 1.3 Global Style Rules

- **Border radius:** `0px` everywhere. Enforce via Tailwind v4 theme override: set `--radius-*` tokens to `0px` in the `@theme` block. This catches all named `rounded-*` utility usage globally. **Arbitrary-value radius classes** (`rounded-[Npx]`) used in existing components bypass theme tokens and must be manually removed during restyling. Files with arbitrary radius that need cleanup: `MissionNode.tsx`, `ChainFrame.tsx`, `ScriptNode.tsx`, `ScriptCommentNode.tsx`, `MissionCardLibrary.tsx`, `ValidationPanel.tsx`, `ScriptNodePalette.tsx`. Xyflow internal elements that need radius (port circles) will use inline styles.
- **No 1px borders for layout.** Separate areas with tonal surface shifts (e.g. `surface-container-low` panel on `surface` background).
- **Ghost borders:** `outline-variant` at 15% opacity, only where accessibility requires a visible boundary.
- **Elevation:** Tonal layering, not drop shadows. When floating elements need depth, use 40-80px blur shadows at 6% opacity of `#001B3C` (cool lapis ambient occlusion).
- **Hieroglyph grid background:** Subtle gold grid lines at 3-5% opacity (`linear-gradient` pattern, 40px spacing) on main content areas.
- **Scrollbars:** 4px width, `#4D4635` thumb on `#0E0E0E` track.
- **Transitions:** All hover/active state transitions use `transition-all duration-300` unless otherwise specified. Sidebar nav hover uses `transition-all duration-200`.

### 1.4 Icon System

The sidebar nav uses **lucide-react** icons (not Material Symbols) to stay consistent with the existing icon system throughout the app. No new icon library is introduced.

Sidebar icon mapping (lucide-react):
- Dialing Computer: `Keyboard`
- Artifact Lab: `Wand` (was `Wand2`, renamed in lucide-react 0.4xx+)
- Personnel: `Users`
- Settings: `Settings`
- Relinquish: `LogOut`
- DIAL_GATE button: `Zap`

---

## 2. Shell Layout

Replace the current `AppLayout.tsx` mode-tab layout with a persistent shell.

### 2.1 Top Bar (fixed, full width, z-50)

```
+--[ SGC ADMIN ]---[ SECTOR_ALPHA | SECTOR_BETA | SECTOR_GAMMA ]------ [ icons ] [ avatar ]--+
|  gold glow text    cosmetic tabs (no function)                        save/reload/export    |
+-- gradient hairline -----------------------------------------------------------------------+
```

- **Left:** "SGC ADMIN" in gold, `font-headline`, `tracking-[0.3em]`, gold drop-shadow glow
- **Center:** Three cosmetic sector tabs in `on-surface-variant`, no click behavior
- **Right:** Action buttons (Save, Hot Reload, Export) moved here from the old Toolbar. Styled as subtle icon buttons with `on-surface-variant` color, gold on hover. Avatar placeholder square.
- **Background:** `surface` at 90% opacity + `backdrop-blur-xl`
- **Bottom edge:** Gradient hairline (`outline-variant` at 20%, transparent ends)
- **Height:** 64px (`h-16`)

### 2.2 Sidebar (fixed left, full height, z-40)

```
+--[ COMMAND_STRATA ]--+
|  V1.0.4_ANCIENT      |
+-----------------------+
|  > Dialing Computer   |  <- Chain Editor
|  > Artifact Lab       |  <- Script Editor
|  > Personnel          |  <- Data Editor
|                       |
|                       |
|  [ DIAL_GATE ]        |  <- Reconnect action
|  Settings             |
|  Relinquish           |  <- Disconnect
+-----------------------+
```

- **Width:** 256px (`w-64`)
- **Background:** `surface-container-lowest` (`#0E0E0E`)
- **Header:** COMMAND_STRATA in gold + version label in `on-surface-variant` at 50% opacity
- **Nav items:** `font-headline`, uppercase, `tracking-widest`, `text-sm`. Each has a lucide-react icon.
  - Default state: `on-surface-variant` at 70% opacity
  - Hover: `surface-container-highest` background, full opacity, `translate-x-1` shift, `duration-200`
  - Active: `secondary-container` at 30% opacity background, `primary` text, `shadow-[inset_4px_0_0_0_#F2CA50]` left border
- **DIAL_GATE button:** Full-width, `primary-container` background, `on-primary` text, `font-headline`, beveled edges (`box-shadow: inset 2px 2px 0 rgba(255,255,255,0.05), inset -2px -2px 0 rgba(0,0,0,0.5)`). Click triggers DB reconnect.
- **Bottom links:** Settings (placeholder, no-op) + Relinquish (disconnects, returns to ConnectDialog). `error` tinted text for Relinquish at 70% opacity.
- **Right edge:** `outline-variant` at 10% opacity border

### 2.3 Main Content Area

- `margin-left: 256px` (sidebar width), `padding-top: 64px` (top bar height)
- `hieroglyph-bg` class applied (subtle gold grid)
- Renders the active page component based on sidebar selection
- Each page manages its own internal split panes (Allotment stays)

### 2.4 Page State Management

Replace the `mode` state variable in `AppLayout` with a `page` state:

```typescript
type Page = 'chains' | 'scripts' | 'data';
```

Sidebar nav clicks set the active page. Each page renders its existing editor content. The chain editor's tab bar (for multiple open spaces) moves into the main content area header, styled as obsidian tabs with gold active indicator.

### 2.5 Disconnect / Reconnect Flow

Both are frontend-only — no new Rust commands needed.

- **Relinquish:** Calls `onDisconnect` callback passed from `App.tsx`. In `App.tsx`, this sets `connected = false`, which re-renders the ConnectDialog. The Rust `AppState` connection pool is dropped on the next `connect_db` call (already handles reconnection).
- **DIAL_GATE:** Calls the same `onDisconnect` to return to ConnectDialog, where the user can reconnect with same or different credentials.

`App.tsx` changes: add `onDisconnect` callback, pass connection metadata (database name, server URL) to `AppLayout` as props for the HUD.

```typescript
// App.tsx additions
const [connectionMeta, setConnectionMeta] = useState<{ db: string; server: string } | null>(null);
const handleDisconnect = useCallback(() => {
  setConnected(false);
  setConnectionMeta(null);
}, []);
// On successful connect, extract db name from connection string and store
```

---

## 3. Page Mapping

### 3.1 Dialing Computer (Chain Editor)

Current layout preserved: `TreeNav | ChainEditor canvas | Inspector` in three Allotment panes.

Changes:
- TreeNav restyled: "Content Browser" header becomes "GATE_ADDRESSES" in `font-headline` gold. Space entries get teal map-pin icons. Active space highlighted with gold left border.
- Space tabs (currently in Toolbar) move to a tab bar between the top bar and the editor canvas. Obsidian background, gold underline on active tab.
- Inspector panel: Section headers in `font-headline` gold uppercase. Property fields use `surface-container-lowest` background. Family color badges keep their current colors but get beveled treatment. `font-mono` usages: keep for hex color values and node IDs, switch to `font-dense` for property labels and field names.
- Bottom panel toggles (Card Palette, Validation) styled as beveled stone buttons with gold/teal active glow.
- Floating panel toggle buttons: remove `rounded-full`, use beveled stone treatment.

### 3.2 Artifact Lab (Script Editor)

Current layout preserved: `ScriptBrowser | ScriptEditor canvas | ScriptPropertyPanel` in three Allotment panes.

Changes:
- ScriptBrowser restyled same as TreeNav — gold header, obsidian entries
- Property panel: Input fields on `surface-container-lowest`, labels in gold uppercase `font-dense`. `font-mono` usages: keep for port type labels and raw values, switch to `font-dense` for property names and display names.
- Node Palette toggle gets the same beveled treatment (remove `rounded-full`)
- "Convert to Chains" button styled as a secondary stone button

### 3.3 Personnel (Data Editor)

Current DataEditor component preserved. Its internal tab navigation (Entities, Items, Missions, Loot Tables, Abilities) restyled with the `font-headline` uppercase treatment. Grid rows use tonal alternation (`surface` / `surface-container-low`). Form inputs follow the "Etched Slabs" pattern from the design doc.

---

## 4. Connect Dialog

Restyle the existing `ConnectDialog.tsx` as "Establish Uplink":

- Centered on `surface` background
- Card: `surface-container-low` with ghost border, no rounded corners
- Title: "ESTABLISH UPLINK" in `font-headline` gold, wide tracking
- Subtitle: "Initialize connection to the Cimmeria database" in `on-surface-variant`
- Input labels: Gold uppercase with Space Grotesk, icon in `on-surface-variant`
- Input fields: `surface-container-lowest` background, no border-radius, ghost border on focus turns gold
- Connect button: Full-width `primary-container` background, `on-primary` text, beveled edges, gold glow shadow on hover. Label: "INITIATE_LINK"
- Error state: `error` border, error background at 10% opacity
- Loading state: Keep the spinner, teal tint

---

## 5. Floating HUD

Fixed position, bottom-left corner (offset from sidebar), shows real connection stats.

```
+-- [ Connected: sgw@localhost:5433 ] --[ Chains: 66 ] --[ Last Save: 2m ago ] --+
```

- Position: `fixed bottom-10 left-[calc(256px+2.5rem)]` (sidebar width + offset)
- `surface-container-low` background with `primary` ghost border (`outline-variant` at 15%)
- `backdrop-blur-md`
- Three data points: gold `font-headline` values with `on-surface-variant` labels in `font-dense`
- Separated by vertical gold hairlines (`primary` at 20%)
- Hidden on screens below `xl` breakpoint
- `z-50`

### 5.1 HUD Data Flow

Add three pieces of state to `AppLayout`, passed as props from `App.tsx` and internal callbacks:

| Data | Source | Update Trigger |
|---|---|---|
| Connection display (e.g. `sgw@localhost:5433`) | Extracted from connection string in `App.tsx`, passed as prop | On connect |
| Chain count | Sum of `spaces.reduce((sum, s) => sum + s.chain_count, 0)` | On space list load |
| Last save | New `lastSaveTime: Date \| null` state in `AppLayout` | Set in `handleSave` on success |

The HUD formats `lastSaveTime` as relative time ("2m ago", "just now") using a simple helper, no external dependency.

---

## 6. Node Editor Styling

The xyflow-based editors (ChainEditor, ScriptEditor) keep their current node structure. Visual changes only.

### 6.1 Chain Nodes (MissionNode.tsx)

`MissionNode.tsx` is the primary node renderer. Currently uses `rounded-[28px]` extensively.

- Remove all `rounded-*` classes (global theme override handles this)
- Node background: `surface-container-low` with `obsidian-panel` gradient (`linear-gradient(145deg, #1c1b1b 0%, #0e0e0e 100%)`)
- Left border: 4px, colored by family (keep current family colors — they already fit the palette)
- Title text: `on-surface`, `font-headline`
- Property labels: `on-surface-variant` at 60%, `font-dense` uppercase
- Hover: border brightens to full family color

### 6.2 Chain Summary Nodes (ChainNode.tsx)

- Same obsidian gradient treatment
- Gold text for chain name

### 6.3 Chain Frames (ChainFrame.tsx)

- Frame border: `outline-variant` at 15% dashed
- Header: Gold `font-headline` text with chain name, enable/disable toggle
- Background: `surface` at 50% opacity

### 6.4 Script Nodes (ScriptNode.tsx)

- Same obsidian gradient treatment as chain nodes
- Port colors stay as-is (they're functional)

### 6.5 Script Comment Nodes (ScriptCommentNode.tsx)

- `surface-container-lowest` background with italic `on-surface-variant` text

### 6.6 Edges (SequenceEdge.tsx, ScriptEdge.tsx)

- Default: `outline-variant` at 40%
- Selected/hover: `primary` with glow
- Animated edges (if any): teal pulse

### 6.7 Canvas Background (ChainEditor.tsx, ScriptEditor.tsx)

Both files contain xyflow `<Background>` component configuration. Change to:

- `color`: `rgba(242, 202, 80, 0.03)` (primary at 3% opacity)
- `gap`: 40 (matching hieroglyph grid spacing)
- `size`: 1
- Background color of the `<ReactFlow>` wrapper: `surface` (`#131313`)

### 6.8 Xyflow Controls

Both `ChainEditor.tsx` and `ScriptEditor.tsx` have `<Controls>` components with existing `!important` CSS overrides. Update these overrides to use obsidian background, gold icon color on hover, no rounded corners.

### 6.9 Xyflow MiniMap

Both `ChainEditor.tsx` and `ScriptEditor.tsx` render `<MiniMap>` components with style overrides. Update to match the Celestial Monolith palette:
- Background: `surface-container-lowest` (`#0E0E0E`)
- Node fill: `surface-container-high` (`#2A2A2A`)
- Border: `outline-variant` at 15% opacity
- Mask fill: `surface` at 80% opacity

---

## 7. Allotment (Split Panes) Styling

Override via CSS in `index.css` targeting allotment's internal selectors:

```css
.split-view-view .sash-container .sash {
  background: rgba(77, 70, 53, 0.1); /* outline-variant at 10% */
}
.split-view-view .sash-container .sash:hover {
  background: rgba(242, 202, 80, 0.2); /* primary at 20% */
}
```

No change to min/max/preferred sizes.

---

## 8. File Changes Summary

All paths relative to `tools/ContentEditor/`.

### Package & Config
| File | Change |
|---|---|
| `ui/package.json` | Add `@fontsource-variable/newsreader`, `@fontsource/inter`. Keep `@fontsource/ibm-plex-mono` (still used for `font-mono`). |

### Core Layout & Styles
| File | Change |
|---|---|
| `ui/src/index.css` | Replace HSL variables with Celestial Monolith tokens. Add `--font-headline`, `--font-dense` to `@theme`. Set `--radius-*` tokens to `0px`. Add `hieroglyph-bg`, `obsidian-panel`, `bevel-stone` utilities. Update scrollbar styles. Import Newsreader + Inter fonts. Add allotment sash overrides. |
| `ui/index.html` | No changes needed — fonts loaded via `@fontsource` imports in CSS, not CDN links. |
| `ui/src/App.tsx` | Add `connectionMeta` state, `handleDisconnect` callback. Pass both to `AppLayout`. |
| `ui/src/components/AppLayout.tsx` | Major restructure: replace mode tabs with shell (TopBar + Sidebar + page routing). Toolbar actions move to TopBar. Add floating HUD component. Add `lastSaveTime` state. Accept `connectionMeta` and `onDisconnect` props. |
| `ui/src/components/Toolbar.tsx` | Remove or gut — action buttons move to TopBar in AppLayout. Chain editor tab bar extracted as inline component within the chain editor page section. |

### Restyled Components (no logic changes)
| File | Change |
|---|---|
| `ui/src/components/ConnectDialog.tsx` | Restyle to "Establish Uplink" theme. |
| `ui/src/components/TreeNav.tsx` | Gold headers, obsidian entries, teal icons. |
| `ui/src/components/Inspector.tsx` | `font-headline` section headers, obsidian field backgrounds. |
| `ui/src/components/ScriptBrowser.tsx` | Restyle same as TreeNav. |
| `ui/src/components/ScriptPropertyPanel.tsx` | Restyle inputs and labels. |
| `ui/src/components/DataGrid.tsx` | Tonal row alternation, gold header text. |
| `ui/src/components/MissionCardLibrary.tsx` | Remove `rounded-[24px]`/`rounded-[28px]`, apply obsidian/gold card treatment. |
| `ui/src/components/ValidationPanel.tsx` | Remove `rounded-[20px]`, `rounded-[24px]`, `rounded-[18px]`, apply tonal surface styling. |
| `ui/src/components/ScriptNodePalette.tsx` | Remove `rounded-[32px]`/`rounded-[28px]`/`rounded-[20px]`, apply obsidian treatment. |
| `ui/src/components/ConvertScriptDialog.tsx` | Remove `rounded-lg`, apply ghost border + obsidian card styling. |
| `ui/src/components/ItemForm.tsx` | Etched-slab input treatment. |
| `ui/src/components/MissionForm.tsx` | Etched-slab input treatment. |
| `ui/src/components/AbilityForm.tsx` | Etched-slab input treatment. |
| `ui/src/components/LootTableForm.tsx` | Etched-slab input treatment. |
| `ui/src/components/EntityTemplateForm.tsx` | Etched-slab input treatment. |

### Editor Components (visual changes only)
| File | Change |
|---|---|
| `ui/src/editors/MissionNode.tsx` | Remove `rounded-[28px]`, obsidian gradient, gold text. Primary node renderer. |
| `ui/src/editors/ChainNode.tsx` | Obsidian gradient, gold text. Summary node. |
| `ui/src/editors/ChainFrame.tsx` | Ghost border, gold header. Remove `rounded-[34px]`, `rounded-[28px]`. |
| `ui/src/editors/ChainEditor.tsx` | Update `<Background>` component props (color, gap). Update `<Controls>` overrides. |
| `ui/src/editors/ScriptNode.tsx` | Obsidian gradient. Remove `rounded-[20px]`. |
| `ui/src/editors/ScriptCommentNode.tsx` | Darkest surface, italic text. Remove `rounded-[16px]`. |
| `ui/src/editors/ScriptEditor.tsx` | Update `<Background>` component props. Update `<Controls>` overrides. |
| `ui/src/editors/SequenceEdge.tsx` | Outline-variant color, gold on select. |
| `ui/src/editors/ScriptEdge.tsx` | Same edge treatment. |
| `ui/src/editors/DataEditor.tsx` | Restyle category tabs, CRUD toolbar buttons. Remove `rounded` classes. |

**New files:** None. All changes are modifications to existing files.

**No Rust changes.** Backend commands, state management, and Tauri config are untouched.

---

## 9. Out of Scope

- Admin Panel (separate app, separate initiative)
- Logs page / real-time log streaming
- Sector tab functionality (cosmetic only)
- New Rust backend commands
- Settings page (placeholder nav item only)
- Mobile/responsive layout (desktop Tauri app only)
- Keyboard navigation / accessibility improvements (follow-up work)
