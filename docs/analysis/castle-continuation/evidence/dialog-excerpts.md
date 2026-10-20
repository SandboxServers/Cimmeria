# CS-00 exact dialog excerpts

Baseline: `d3efb7d41aad0b6008397f112d224ce7430d7acf`. Companion: [mission/item excerpts](seed-excerpts.md). Current seed relations do not prove runtime triggers.

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5957`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5824, 1571, 4982, 'Infirmary', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5959`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5825, 1571, 4984, 'Infirmary', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5961`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5826, 1571, 4985, 'Infirmary', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5963`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5827, 1571, 4983, 'Infirmary', 16777216, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5965`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5828, 1571, NULL, 'Infirmary', 1073741824, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5967`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5829, 1572, NULL, 'Symbiote', 1073741824, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5971`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5830, 1572, 4989, 'Symbiote', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5973`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5831, 1572, 4990, 'Symbiote', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_set_maps.sql:5975`

```sql
INSERT INTO dialog_set_maps (dialog_set_map_id, dialog_set_id, dialog_id, topic_text, interaction_flags, min_level, missions_completed, missions_not_accepted, alignments, factions) VALUES (5832, 1572, 4991, 'Symbiote', 0, 1, '{}', '{}', '{}', '{}');
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21489`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4982, 113564, 'Do we have any medics on the channel? I have soldiers collapsing with seizures - some of them are in a pretty bad way. I repeat, we need medics...', 1110, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21491`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4982, 113565, 'Captain, this is $name$. Are these soldiers just out of stasis pods?', 0, 1);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21493`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4982, 113566, 'Most of them are from the East Wing stasis pods - why?', 1110, 2);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21495`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4982, 113567, 'They''ve got stasis sickness. They need ambernol.', 0, 3);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21497`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4982, 113568, 'There''s an infirmary on Level 5. Doubletime it there and secure a supply. We''ll be coming behind you as fast as we can with the affected.', 1110, 4);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21499`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96921, 'What''s going on out there? I''ve been recovering from my last bout with Romney. I had no idea.', 2490, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21501`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96922, 'We''re breaking out... ', 0, 1);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21503`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96923, 'Oh, thank god... They pulled me out of stasis a few days ago for Romney sessions but... They seemed preoccupied. I figured something was up.', 0, 2);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21505`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96924, 'It is. Do you know where they keep the ambernol?', 0, 3);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21507`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96925, 'Yeah, that med dispenser over there is programmed to control the supply. There is a code for it.', 2490, 4);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21509`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96926, 'Do you know it?', 0, 5);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21511`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4983, 96927, 'No. i saw them type it in. You''d probably have to hack the dispenser.', 2490, 6);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21513`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4984, 96928, 'You''ve modified the Med Dispenser so that it now provides an unlimited supply of Ambernol.', 0, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21515`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4985, 96929, 'Have you found the ambernol? We''ve got a lot of marines in need.', 1110, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21517`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4985, 96930, 'Supply is secure. Ogilvie is here - he''s in charge of it. ', 0, 1);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21519`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4985, 96931, 'Ogilvie? Glad to hear he''s alive... We were worried about him. Get yourself to the Throne Room - we have to override the Goa''uld defenses so we can boost power to communications.', 1110, 2);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21527`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96950, 'Jaffa, kree! I am glad you have survived your captivity. What is your status?', 2499, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21529`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96951, 'I am aiding the Tau''ri in their escape, Moh''katan. ', 0, 1);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21531`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96952, 'You must also aid our people. The NID Tau''ri removed the symbiotes from our warriors as part of their torture. All are left weakened.', 2499, 2);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21533`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96953, 'Indeed. I suffer as well.', 0, 3);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21535`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96954, 'Colonel Marsh tells me that his Op-CORE humans have found symbiotes in storage in the infirmary on level 5. Proceed there and confirm this assertion.', 2499, 4);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21537`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4989, 96955, 'Yes, Moh''katan.', 0, 5);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21539`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4990, 96946, 'You quickly bypass the security protections on the Symbiote Tank. You reach inside and pull out a symbiote from the tank.', 0, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21541`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4991, 96956, 'Have you found symbiotes?', 2499, 0);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21543`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4991, 96957, 'Several tanks are here. The Tau''ri were correct. ', 0, 1);
```

`db/resources/Dialogs/Seed/dialog_screens.sql:21545`

```sql
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (4991, 96958, 'Excellent. I will send our warriors there. In the meantime, make your way to the Throne Room and disable the Goa''uld defenses. You must move quickly. There is not much time. ', 2499, 2);
```
