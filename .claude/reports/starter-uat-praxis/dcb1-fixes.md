# dcb1-fixes: SP-DCB1 p3-p6 brief fixes

Docs only. Files: `docs/analysis/starter-uat-praxis/briefs/SP-DCB1-p3..p6.md`, `verify/SP-DCB1-p3..p6.md`, and the estimate column of `work-packets.md` (p4 16->18, p5 20->21, p6 18->22). `rows.md` unchanged.

## Defects fixed

1. p4/p5 scope. p4 stops at the inventory read by design; its verify now grades only Frost search, Guard body dialog and the single grant, and lists equip, reload, shots, Focus, relog duplicate and H-CB-U02-1 as NOT-RUN here, moved to p5. p5 now drives all of them inside 21 calls.
2. p5 ids (traced):
   - Equip grants 708 and 579, not 592: `crates/cell-combat/src/cell/cell_methods/inventory/bandolier/weapon_abilities.rs:45-46`, event at `:121`; seed `db/resources/Abilities/Seed/abilities.sql:5160`, `:5111`.
   - 592 Pistol Shot is a kit ability (`abilities.sql:5149`; chain 1005, `db/resources/Archetypes/Seed/char_creation_abilities.sql:5`), not on the hotbar, so the first cast uses `place:true` (`crates/lab/src/supervisor/combat/use_ability/mod.rs:9-14`, `:217`).
   - Reload 596 (`abilities.sql:296`, `reload.rs:11`) is not in the kit. The manual reload is the `R` key to `requestReload` (`auto_reload.rs:5-6`); the step is now a `client_ui_sequence` key press plus a 3.5 s wait.
3. p6 fixture: 638 is accepted on entering Region2 (`docs/content/mission-chains.md:187`); the prisoner (z -113.14) and the old GOTO sit outside the region (z -141..-115, `scenario-map.md:109`). New recipe: GOTO `-289.7 65.6 -111.0`, `client_move_to` to `-289.8 65.47 -117.0`, turn round (`HP-P6.md:71` recipe), then click. Gotcha 22 forbids Region9 (`scenario-map.md:115`). Verify gains a Region2-entry row and a BLOCKED grade when 638 is missing.
4. Hover time-outs: gotcha 21 added to p4, p5, p6 (`click_hover.rs:25`, `:28`, `:46-52`); every affected first click is labelled 2 calls (p4 x2, p5 x1, p6 x2); `client_hover_probe` is banned (never turns the camera).
5. p3: all rows marked `BLOCKED(harness gap: Jaffa create form)` in brief and verify, with "must not be tried again". Labels kept; estimate stays 20.

## Checks (C2a)

| Brief | sum | ESTIMATE | bad labels |
|---|---|---|---|
| p3 | 20 | 20 | 0 |
| p4 | 18 | 18 | 0 |
| p5 | 21 | 21 | 0 |
| p6 | 22 | 22 | 0 |

Gotcha 11 untouched. All edited files verified CRLF (no bare LF).

## Documentation debt / unverified

- The `R` reload binding is read from source and not seen live; p5 step 2 records the result and moves on.
- The p6 `client_move_to` point (-289.8, 65.47, -117.0) is inside Region2 per the bounding box but the scenario-map W04 point is marked unverified there.
- p3 stays blocked until `lab_create_character` supports the Jaffa form (single Name plus Suffix); no issue filed here.
- p6 dropped the early tracker check to stay at 22.
