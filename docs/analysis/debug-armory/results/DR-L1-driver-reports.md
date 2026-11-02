# DR-L1 driver reports (raw, as returned by the lab-driver runs)

Local server pid 45256, DB sgw_dr_lab, instance `default`, character Celltest (Soldier, GM), world 1300. Run date 2026-11-02, UTC times from the client. Drivers could not read files, so the orchestrator pasted the brief; the pasted copies override briefs/DR-L1.md where noted. Chat lines cannot be attested; no `.bug DRL1` line ever came back in a chat read (zero lines each time, one read showed the sent line only), so the verifier must find row timing in the server log or SigNoz by time.

## Part A (ATTEMPT 1), 45 of 56 calls, about 16:25-16:28 UTC, vendors 0-3

- Screenshot `L1-1-vendor-line`: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-102521.043.png
- Template lookup (client_entity_find template_id 1610-1616): ids 100423-100429; spawns (268,7.05,-915) (268,7.09,-918) (268,7.13,-921) (268,7.13,-924) (266,7.02,-927) (266,7.09,-930) (266,7.15,-933). All render with the name "Consumables" (shared name_id 27264).
- BUG lines (UTC): L1-1 start 16:25:09 (resent 16:25:59); vendor 0 16:26:04; vendor 1 16:26:41; vendor 2 16:27:19; vendor 3 16:27:54.
- Vendor 0 (100423): stand 263.5 7.05 -916.5, distance 4.75, VendorWin opened, mouse_over_after 0, 19 rows, first "Test Weapon", last "Composite Combat Knife".
- Vendor 1 (100424): distance 4.74, VendorWin, mouse_over_after 100424, 19 rows, first "Microblade Combat Knife", last "SI 9mm Pistol" (the driver's own note says its raw read was uncertain; check against the seed).
- Vendor 2 (100425): stand 263.5 7.13 -922.5, distance 4.74, VendorWin, mouse_over_after 100425, 19 rows, first "SI 4 9mm Pistol", last "Assault Shotgun" (also uncertain in the driver's note).
- Vendor 3 (100426): distance 4.74, VendorWin, mouse_over_after 100426, 16 rows, first "Strike Shotgun", last "Papa Polecat Flamethrower".
- No failed calls.

## Part B (ATTEMPT 2), about 31 calls, about 16:29-16:31 UTC, vendors 4-6

- BUG lines: vendor 4 16:29:15; vendor 5 16:30:05; vendor 6 16:30:44.
- Vendor 4 (100427) at the brief's stand 261.5 7.02 -928.5: FAIL twice, hover error "every point on the target is off screen or covered by another entity", other_entity 100226 at all 5 points. Screenshot: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-102948.211.png
- Vendor 5 (100428): distance 4.74, VendorWin, mouse_over_after 100428, 19 rows, first "Tri-Naq Alloy Ballistic Gloves", last "Boron-Carbide Ballistic Boots". Close posted_only.
- Vendor 6 (100429): stand 261.5 7.15 -933.0, distance 4.5, VendorWin, mouse_over_after 100429, 3 rows, first "Trinium Ballistic Boots", last "Neutronium Ablative Boots". Close posted_only.

## Vendor 4 retry (separate run, bug line 16:32:01 UTC)

- Entity 100226 is "Archetype Skills Trainer" at (259.42, 6.99, -930.42), 3.31 m from the old stand point, so it covers vendor 4 there.
- New stand 261.5 7.02 -926.0 works: distance 4.61, mouse_over_after 100427, VendorWin, 19 rows, first "Aramid Fiber Ballistic Helmet", last "Trinium Ballistic Gloves". Closed, confirmed.

## Part C (ATTEMPT 3), about 37 calls, from 16:33:19 UTC, L1-3 buy and L1-4 equip

- S2: cash 0, Main: Accessory (10293) slot 1. BUG `L1-3 start` 16:33:24; `/gmgivecash 100000` 16:33:26 (G tier); cash 100000.
- Armory 1 (100424): first click hover_timeout (16369 ms), unchanged retry opened VendorWin. Scroll 12: screen rows 1-5 GL 25 Grenade Launcher, SK37 LMG, SK37 B LMG, SK38 LMG, SK38 X LMG; row 6 SK Gauss LMG, row 7 SI 3 9mm Pistol. `client_vendor_buy row 7 quantity 1`: bought true, cash 99700 -> 99700 (delta -300), items_gained 1, Main slot 2 = SI 3 9mm Pistol (10294), N3 tier.
- Armory 4 (100427, new stand): click ok. Scroll 0: row 1 Aramid Fiber Ballistic Helmet ... row 7 Goggles. `client_vendor_buy row 1 quantity 1`: bought true, cash 99699 (delta -1), Main slot 3 = Aramid Fiber Ballistic Helmet (10295), N1 tier.
- L1-4: equip pistol: effect_ok, Main 2 -> Bandolier slot 1; equip helmet: effect_ok, Main 3 -> Head slot 1. Inventory read: Main Accessory only; Bandolier SI 3 9mm Pistol; Head Aramid Fiber Ballistic Helmet. Bag closed (key b posted_only, UI state confirms closed). Screenshot `L1-4-equipped`: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-103546.358.png
- A `client_window_read` on window "Inventory" failed (no such window); not retried.

## Part D (ATTEMPT 4), 41 calls, 16:36:52-about 16:39 UTC

- First launch stopped with 0 calls (the driver had no `client_combat_log`); rerun used client_chat_log instead.
- S2: cash 99699, pistol and helmet equipped. BUG `L1-5 start` 16:36:58; STAGE 240.0 7.08 -877.0 at 16:37:02.
- Dummy: template 1310, id 100246, "Jaffa", distance 5, level 1. Screenshots: before-fire C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-103722.212.png ; after-fire C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-103742.224.png
- Right-click on 100246: verdict inconclusive (no_post_read), hover_verified, in_range true, mouse_over_after 0. Chat reads: "weapon" none, "damage" none, "ammo" one Combat-channel line seq 25065: "Your SI 3 9mm Pistol is out of ammo." No "no ranged attack" line.
- L1-6 pistol: unequip effect_ok (Bandolier 1 -> Main 2). BUG 16:37:50. Armory 1 (100424), distance 4.74, VendorWin. Sell tab: rows "SI 3 9mm Pistol", "Aramid Fiber Ballistic Helmet", "Eyeglasses"; pistol row 1. Sell ui_sequence results [true,null,true,null], inconclusive (posted_only). Inventory: pistol gone from Main, cash 99699 (unchanged, price 0).
- L1-6 helmet: unequip effect_ok (Head 1 -> Main 2). BUG 16:38:46. Armory 4 (100427), distance 4.61. Sell tab: row 1 helmet. Same ui_sequence result, inconclusive. Inventory: helmet gone, Head empty, cash 99699.
- Final: Main Accessory only, Bandolier and Head empty, cash 99699.

## Deviations from briefs/DR-L1.md found during the run (brief fixes owed)

1. Vendors render as "Consumables": name searches fail; use template_id 1610-1616 (L1-1.4, L1-2, OPEN).
2. Armory 4 stand point `261.5 7.02 -928.5` is covered by the Archetype Skills Trainer (100226); use `261.5 7.02 -926.0`.
3. The dummy lookup by name "Jaffa" is unsafe; use template_id 1310.
4. `lab-driver` agents cannot read files and have no `client_combat_log`; the brief must be pasted and L1-5.5 must use client_chat_log.
5. Selling at price 0: cash unchanged and items gone from Main (sell clause on the server unverified, see verifier).
