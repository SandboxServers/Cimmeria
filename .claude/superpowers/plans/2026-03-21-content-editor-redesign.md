# Content Editor Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reskin the Content Editor with the Celestial Monolith design system — obsidian/gold/lapis/teal palette, sidebar nav shell, themed components.

**Architecture:** Design-system-first approach. Replace CSS theme tokens, then build the new shell layout, then restyle each component in place. No backend changes, no new files.

**Tech Stack:** React 19, Tailwind v4, Vite, @xyflow/react, allotment, lucide-react, Tauri v2

**Spec:** `.claude/superpowers/specs/2026-03-21-content-editor-redesign-design.md`

---

### Task 1: Foundation — Fonts & Design Tokens

**Files:**
- Modify: `tools/ContentEditor/ui/package.json`
- Modify: `tools/ContentEditor/ui/src/index.css`

- [ ] **Step 1:** Add font packages to package.json: `@fontsource-variable/newsreader`, `@fontsource/inter`. Keep `@fontsource/ibm-plex-mono`.
- [ ] **Step 2:** Run `npm install` in `tools/ContentEditor/ui/`
- [ ] **Step 3:** Rewrite `index.css` — replace HSL variables with Celestial Monolith hex tokens. Add `--font-headline`, `--font-dense`. Set `--radius-*` to `0px`. Add `hieroglyph-bg`, `obsidian-panel`, `bevel-stone` utility classes. Update scrollbar styles. Add allotment sash overrides. Import new fonts.
- [ ] **Step 4:** Verify vite dev server starts without errors
- [ ] **Step 5:** Commit: "feat(editor): add Celestial Monolith design tokens and fonts"

---

### Task 2: Shell Layout — App.tsx + AppLayout.tsx

**Files:**
- Modify: `tools/ContentEditor/ui/src/App.tsx`
- Modify: `tools/ContentEditor/ui/src/components/AppLayout.tsx`
- Modify: `tools/ContentEditor/ui/src/components/Toolbar.tsx`

- [ ] **Step 1:** Update `App.tsx` — add `connectionMeta` state, `handleDisconnect` callback, pass both + `onDisconnect` to AppLayout.
- [ ] **Step 2:** Restructure `AppLayout.tsx` — replace mode tabs with shell: fixed TopBar (SGC ADMIN branding + cosmetic sector tabs + action buttons), fixed Sidebar (nav items + DIAL_GATE + Relinquish), main content area rendering active page. Add floating HUD component. Add `lastSaveTime` state.
- [ ] **Step 3:** Gut `Toolbar.tsx` — remove action buttons (moved to TopBar). Keep only the chain tab bar as an inline element within the chains page section.
- [ ] **Step 4:** Verify the shell renders with sidebar navigation switching between pages
- [ ] **Step 5:** Commit: "feat(editor): add SGC shell layout with sidebar nav and top bar"

---

### Task 3: Connect Dialog Restyle

**Files:**
- Modify: `tools/ContentEditor/ui/src/components/ConnectDialog.tsx`

- [ ] **Step 1:** Restyle to "Establish Uplink" theme — obsidian card, gold title, etched-slab inputs, beveled connect button, themed error state.
- [ ] **Step 2:** Commit: "feat(editor): restyle connect dialog as Establish Uplink"

---

### Task 4: Navigation Components — TreeNav, ScriptBrowser, Inspector

**Files:**
- Modify: `tools/ContentEditor/ui/src/components/TreeNav.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ScriptBrowser.tsx`
- Modify: `tools/ContentEditor/ui/src/components/Inspector.tsx`

- [ ] **Step 1:** Restyle TreeNav — gold "GATE_ADDRESSES" header, obsidian entries, teal icons, gold active left border.
- [ ] **Step 2:** Restyle ScriptBrowser — same treatment as TreeNav.
- [ ] **Step 3:** Restyle Inspector — font-headline section headers, obsidian field backgrounds, font-dense for labels, keep font-mono for hex/IDs.
- [ ] **Step 4:** Commit: "feat(editor): restyle navigation and inspector components"

---

### Task 5: Data Editor & Forms

**Files:**
- Modify: `tools/ContentEditor/ui/src/editors/DataEditor.tsx`
- Modify: `tools/ContentEditor/ui/src/components/DataGrid.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ItemForm.tsx`
- Modify: `tools/ContentEditor/ui/src/components/MissionForm.tsx`
- Modify: `tools/ContentEditor/ui/src/components/AbilityForm.tsx`
- Modify: `tools/ContentEditor/ui/src/components/LootTableForm.tsx`
- Modify: `tools/ContentEditor/ui/src/components/EntityTemplateForm.tsx`

- [ ] **Step 1:** Restyle DataEditor category tabs with font-headline uppercase treatment. Remove rounded classes. Style CRUD toolbar buttons.
- [ ] **Step 2:** Restyle DataGrid — tonal row alternation, gold header text, obsidian search input.
- [ ] **Step 3:** Restyle all 5 form files with etched-slab input treatment — surface-container-lowest backgrounds, gold labels, ghost borders.
- [ ] **Step 4:** Commit: "feat(editor): restyle data editor, grid, and forms"

---

### Task 6: Palette, Validation & Dialog Components

**Files:**
- Modify: `tools/ContentEditor/ui/src/components/MissionCardLibrary.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ValidationPanel.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ScriptNodePalette.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ScriptPropertyPanel.tsx`
- Modify: `tools/ContentEditor/ui/src/components/ConvertScriptDialog.tsx`

- [ ] **Step 1:** Restyle MissionCardLibrary — remove all rounded-[Npx], apply obsidian/gold card treatment.
- [ ] **Step 2:** Restyle ValidationPanel — remove rounded-[Npx], tonal surface styling.
- [ ] **Step 3:** Restyle ScriptNodePalette — remove rounded-[Npx], obsidian treatment.
- [ ] **Step 4:** Restyle ScriptPropertyPanel — etched-slab inputs, gold labels. Audit font-mono: keep for port types, switch to font-dense for property names.
- [ ] **Step 5:** Restyle ConvertScriptDialog — remove rounded-lg, ghost border + obsidian card.
- [ ] **Step 6:** Commit: "feat(editor): restyle palettes, validation panel, and dialogs"

---

### Task 7: Node Editor Components

**Files:**
- Modify: `tools/ContentEditor/ui/src/editors/MissionNode.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ChainNode.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ChainFrame.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ScriptNode.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ScriptCommentNode.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/SequenceEdge.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ScriptEdge.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ChainEditor.tsx`
- Modify: `tools/ContentEditor/ui/src/editors/ScriptEditor.tsx`

- [ ] **Step 1:** Restyle MissionNode — remove rounded-[28px], obsidian gradient background, gold title, themed property labels.
- [ ] **Step 2:** Restyle ChainNode — obsidian gradient, gold text.
- [ ] **Step 3:** Restyle ChainFrame — remove rounded-[34px]/[28px], ghost border, gold header.
- [ ] **Step 4:** Restyle ScriptNode — remove rounded-[20px], obsidian gradient.
- [ ] **Step 5:** Restyle ScriptCommentNode — remove rounded-[16px], darkest surface.
- [ ] **Step 6:** Restyle SequenceEdge — outline-variant default color, gold on select. Update label pill.
- [ ] **Step 7:** Restyle ScriptEdge — same edge treatment.
- [ ] **Step 8:** Update ChainEditor.tsx — Background (gold dots, 40px gap), Controls (obsidian/gold), MiniMap (obsidian palette).
- [ ] **Step 9:** Update ScriptEditor.tsx — same Background/Controls/MiniMap treatment.
- [ ] **Step 10:** Commit: "feat(editor): restyle all node editor components with Celestial Monolith"

---

### Task 8: Final Polish & Verification

- [ ] **Step 1:** Replace all `text-white` usages across the project with `on-surface` equivalent.
- [ ] **Step 2:** Run full build (`npm run build`) and fix any TypeScript/Tailwind errors.
- [ ] **Step 3:** Visual verification — launch the app and check all pages render correctly.
- [ ] **Step 4:** Commit any remaining fixes.
- [ ] **Step 5:** Final commit: "feat(editor): complete Celestial Monolith redesign"
