# UO-15 worknote: service bark lines and speakers

Ready-to-paste `npc_bark` lines for the seven screens that hang off holder dialog 63900 (UO-24). Index = screen_id - 63901. No seed or code is changed. The lines carry no visible label (H-LBL); provenance sits only in the SQL comment above each block.

## § Lines

| Screen | Index | Consumer | NPC | Text (as read) | Characters |
|---|---|---|---|---|---|
| 63901 | 0 | UO-35 | Biology guides (spawns 63810, 63817) | Three vendors sell Biomedical Components: Human Biomedical Components in the SGC wing, Common Biomedical Components in the Common Area, Jaffa Biomedical Components in the Free Jaffa wing. | 187 |
| 63902 | 1 | UO-35 | Electronics guides (63811, 63812) | Three vendors sell Electronics Components: Human Electronics Components in the SGC wing, Common Electronics Components in the Common Area, Jaffa Electronics Components in the Free Jaffa wing. | 191 |
| 63903 | 2 | UO-35 | Engineering guides (63813, 63814) | Three vendors sell Power Systems Components: Human Power Systems Components in the SGC wing, Common Power Systems Components in the Common Area, Jaffa Power Systems Components in the Free Jaffa wing. | 199 |
| 63904 | 3 | UO-35 | Physics guides (63815, 63816) | Three vendors sell Materials Components: Human Materials Components in the SGC wing, Common Materials Components in the Common Area, Jaffa Materials Components in the Free Jaffa wing. | 183 |
| 63905 | 4 | UO-33 | the medic (63818) | Your injuries have been seen to, and you are patched up and ready to go. | 72 |
| 63906 | 5 | UO-36 | the inn NPC (63822) | This is the inn, a meeting place for off-duty personnel. We do not offer rooms or rest here. | 92 |
| 63907 | 6 | UO-36 | the battlemaster NPC (63823) | This is the battle hall, where unit commanders plan their drills. No battle queue is run from here. | 99 |

Vendor strings match `db/resources/Texts/Seed/texts.sql` (monikers 27232-27239 and 27248-27251) exactly. Area names come from `omega-concept-art.md`: Human vendors sit in the SGC wing, Common vendors in the Common Area, Jaffa vendors in the Free Jaffa wing. Each guide line names all three vendors, so the same line serves both guides of a specialty and never says "here".

The medic line reads true at full health: the chain refills health and speaks the line even when the player is not hurt, and "patched up" holds in both cases. The inn and battle hall lines state what the building is and promise no service (D-UO32 b, F39).

## § Speakers

| Role | Speaker | Moniker | Shipped text (texts.sql) | Match |
|---|---|---|---|---|
| Medic (default, D-UO33) | `Castle Medic` | 6964 | Castle Medic | exact |
| Medic if the owner picks 7000 | `Nurse Collins` | 7000 | Nurse Collins | exact |
| Medic if the owner picks 7952 | `SGC Scientist` | 7952 | SGC Scientist | exact |
| Crafting guides | `SGC Scientist` | 7952 | SGC Scientist | exact |
| Inn and battlemaster | `Airman` | 8360 | Airman | exact |

No deviation: every shipped text matches the table, so the speaker strings are copied as the file has them.

## § SQL

The screen text is single-quoted in the SQL. None of the seven lines contains an apostrophe, so no doubling is needed.

### For UO-33

```sql
-- CIMMERIA_AUTHORING (UO-15; H-LBL: no visible label)
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63905, 'Your injuries have been seen to, and you are patched up and ready to go.', 0, 4);
```

### For UO-35

```sql
-- CIMMERIA_AUTHORING (UO-15; H-LBL: no visible label)
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63901, 'Three vendors sell Biomedical Components: Human Biomedical Components in the SGC wing, Common Biomedical Components in the Common Area, Jaffa Biomedical Components in the Free Jaffa wing.', 0, 0);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63902, 'Three vendors sell Electronics Components: Human Electronics Components in the SGC wing, Common Electronics Components in the Common Area, Jaffa Electronics Components in the Free Jaffa wing.', 0, 1);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63903, 'Three vendors sell Power Systems Components: Human Power Systems Components in the SGC wing, Common Power Systems Components in the Common Area, Jaffa Power Systems Components in the Free Jaffa wing.', 0, 2);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63904, 'Three vendors sell Materials Components: Human Materials Components in the SGC wing, Common Materials Components in the Common Area, Jaffa Materials Components in the Free Jaffa wing.', 0, 3);
```

### For UO-36

```sql
-- CIMMERIA_AUTHORING (UO-15; H-LBL: no visible label)
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63906, 'This is the inn, a meeting place for off-duty personnel. We do not offer rooms or rest here.', 0, 5);
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (63900, 63907, 'This is the battle hall, where unit commanders plan their drills. No battle queue is run from here.', 0, 6);
```

## § Checks

Run on 2026-10-19 against this file.

1. Every § SQL `text` value is at most 200 characters: 63905 = 72, 63901 = 187, 63902 = 191, 63903 = 199, 63904 = 183, 63906 = 92, 63907 = 99. All seven rows sit on dialog 63900 with `speaker_id` 0, screens 63901-63907, indexes 0-6.
2. Guide rows name their three vendors verbatim (63901-63904: `True` each), checked against the shipped texts for monikers 27232-27239 and 27248-27251, which match the § Lines names.
3. Forbidden-word scan (`Cimmeria|authored|reconstruction|placeholder|restored|Woolsey|Otin|Asgard`, case-insensitive, plus any bracketed tag). Hits are only the three `-- CIMMERIA_AUTHORING` comment lines (40, 47, 57). No `VALUES` row, and no line or speaker, matches.
