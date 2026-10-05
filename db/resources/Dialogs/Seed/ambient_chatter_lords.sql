--
-- NEW CONTENT (Debug Area, DA-09): the System Lords' summit chatter.
-- Group 1 is spoken by the circle of System Lords in the south compound
-- courtyard of world 1300 (spawns 13850-13856,
-- spawnlist_debug_area_lords.sql); docs/content/debug-area.md#system-lords-summit.
--
-- Every line is Cimmeria-written: the 2009 client ships no NPC-to-NPC
-- chatter. Each exchange is one petty squabble; the group plays them in
-- exchange order, one every exchange_gap_secs once the last line is out,
-- and only while a player stands within hear_radius of a speaker. 18 m
-- reaches every lord from the circle and from the landing spot behind
-- Ba'al (Ra's Jaffa is 16.5 m from it), and nothing from the services
-- plaza or the Z1 arrival.
-- delay_ms is the pause before a line, sized to read the line before it
-- (2.5 s plus 55 ms a character, 3 to 9 s); the first line of an
-- exchange has none.
--
INSERT INTO ambient_chatter_groups (group_id, world_id, name, hear_radius, exchange_gap_secs) VALUES (1, 1300, 'Debug Area System Lords summit', 18, 30);

-- Exchange 1: The seating plan.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 1, 0, 'DebugArea_Lords_Ra', 0, 'Why is my throne the same height as everyone else''s? I asked for the tall one.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 1, 1, 'DebugArea_Lords_Baal', 7000, 'There are no thrones, Ra. We are standing in a circle. The committee decided.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 1, 2, 'DebugArea_Lords_Ra', 6500, 'I do not recall approving a committee.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 1, 3, 'DebugArea_Lords_Anat', 4500, 'You were sulking on Abydos at the time.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 1, 4, 'DebugArea_Lords_RaJaffa', 4500, 'Indeed.');

-- Exchange 2: Labelled lunch.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 2, 0, 'DebugArea_Lords_Nerus', 0, 'Who ate the last of the Tau''ri crisps? They were labelled. ''Nerus. Do not touch.''');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 2, 1, 'DebugArea_Lords_Morrigan', 7000, 'My crows were hungry.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 2, 2, 'DebugArea_Lords_Nerus', 3500, 'Your crows have eaten my lunch every day for four hundred years!');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 2, 3, 'DebugArea_Lords_Athena', 6000, 'Statistically, Nerus, you should have stopped bringing lunch.');

-- Exchange 3: The letters.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 3, 0, 'DebugArea_Lords_Baal', 0, 'Has anyone else had a strongly worded letter from the shol''va Teal''c?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 3, 1, 'DebugArea_Lords_Athena', 6500, 'Three. Each one says only ''Indeed.'' I have no idea what he is agreeing with.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 3, 2, 'DebugArea_Lords_Morrigan', 6500, 'He raised one eyebrow at me once. I did not sleep for a week.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 3, 3, 'DebugArea_Lords_Ra', 6000, 'He would not dare raise an eyebrow at the Supreme System Lord.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 3, 4, 'DebugArea_Lords_Baal', 6000, 'He raised both at you. We all saw it.');

-- Exchange 4: The eyes.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 4, 0, 'DebugArea_Lords_Morrigan', 0, 'Ra, you are doing the glowing eyes again.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 4, 1, 'DebugArea_Lords_Ra', 5000, 'I am a god. My eyes glow when I am displeased.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 4, 2, 'DebugArea_Lords_Morrigan', 5000, 'You have been displeased since the Bronze Age. It is giving me a headache.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 4, 3, 'DebugArea_Lords_Athena', 6500, 'It is also why your Jaffa keep walking into walls at night.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 4, 4, 'DebugArea_Lords_RaJaffa', 5500, 'Indeed.');

-- Exchange 5: The voice.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 5, 0, 'DebugArea_Lords_Nerus', 0, 'Why must we all use the deep echo voice? I have a lovely natural tenor.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 5, 1, 'DebugArea_Lords_Baal', 6500, 'Because nobody kneels for a tenor, Nerus.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 5, 2, 'DebugArea_Lords_Nerus', 5000, 'I kneel for a good tenor. And for a good souffle.');

-- Exchange 6: The paperwork.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 6, 0, 'DebugArea_Lords_Ra', 0, 'Let it be recorded: the sun is mine. I have the paperwork.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 6, 1, 'DebugArea_Lords_Athena', 5500, 'Your paperwork is a cartouche of yourself pointing at the sun.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 6, 2, 'DebugArea_Lords_Ra', 6000, 'It is legally binding on eleven worlds.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 6, 3, 'DebugArea_Lords_Anat', 4500, 'Eight. Three of them exploded.');

-- Exchange 7: The sarcophagus rota.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 7, 0, 'DebugArea_Lords_Anat', 0, 'Someone left the sarcophagus lid open again. Who was in it last?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 7, 1, 'DebugArea_Lords_Baal', 6000, 'I was. Well. One of me was. It is hard to say which.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 7, 2, 'DebugArea_Lords_Morrigan', 5500, 'Tell all of you that the sign-up sheet exists for a reason.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 7, 3, 'DebugArea_Lords_Baal', 5500, 'Seven of me signed it. That is the sign-up sheet working.');

-- Exchange 8: The decor.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 8, 0, 'DebugArea_Lords_Athena', 0, 'Who keeps putting gold leaf on everything? I can see my reflection in the floor.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 8, 1, 'DebugArea_Lords_Ra', 7000, 'Gold is the colour of divinity.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 8, 2, 'DebugArea_Lords_Athena', 4000, 'Gold is the colour of a man who never took a design course.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 8, 3, 'DebugArea_Lords_Nerus', 5500, 'Is the gold edible? Asking for myself.');

-- Exchange 9: Kree.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 9, 0, 'DebugArea_Lords_Morrigan', 0, 'My First Prime says ''kree'' means ''attention''. Ba''al''s says it means ''hurry up''.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 9, 1, 'DebugArea_Lords_Baal', 7000, 'It means whatever I need it to mean. That is the beauty of kree.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 9, 2, 'DebugArea_Lords_Ra', 6000, 'Kree is mine. I invented it.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 9, 3, 'DebugArea_Lords_RaJaffa', 4000, 'With respect, my lord, you also say you invented bread.');

-- Exchange 10: Reply all.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 10, 0, 'DebugArea_Lords_Athena', 0, 'Ba''al. You replied to the entire galaxy again.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 10, 1, 'DebugArea_Lords_Baal', 5000, 'It was an important announcement about my new clone.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 10, 2, 'DebugArea_Lords_Morrigan', 5500, 'You have announced the same clone forty times.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 10, 3, 'DebugArea_Lords_Baal', 5000, 'Forty different clones. Forty different announcements. Do keep up.');

-- Exchange 11: The agenda.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 11, 0, 'DebugArea_Lords_Nerus', 0, 'Item one on the agenda: the snack budget.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 11, 1, 'DebugArea_Lords_Ra', 5000, 'Item one is always my glory.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 11, 2, 'DebugArea_Lords_Nerus', 4000, 'Your glory is item two. Last time it was item one and we ran out of time for snacks.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 11, 3, 'DebugArea_Lords_Anat', 7000, 'I move that we skip both and go straight to complaining about the Tau''ri.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 11, 4, 'DebugArea_Lords_Baal', 6500, 'Seconded. By most of me.');

-- Exchange 12: The standing.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 12, 0, 'DebugArea_Lords_Anat', 0, 'The shol''va was seen on Chulak again, saying nothing, very loudly.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 12, 1, 'DebugArea_Lords_Morrigan', 6000, 'He stood outside my palace for six hours. No demands. Just standing.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 12, 2, 'DebugArea_Lords_Athena', 6000, 'It is the most terrifying negotiating tactic I have ever seen.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 12, 3, 'DebugArea_Lords_Ra', 6000, 'If he comes here, I shall command him to kneel.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 12, 4, 'DebugArea_Lords_RaJaffa', 5000, 'He will not kneel, my lord. He will say ''indeed''. And then he will not kneel.');

-- Exchange 13: The vote.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 0, 'DebugArea_Lords_Baal', 0, 'Show of hands: who is Supreme System Lord this week?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 1, 'DebugArea_Lords_Ra', 5500, 'Me.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 2, 'DebugArea_Lords_Anat', 3000, 'Me.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 3, 'DebugArea_Lords_Morrigan', 3000, 'My crows vote for me.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 4, 'DebugArea_Lords_Nerus', 3500, 'I vote for lunch.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 5, 'DebugArea_Lords_Baal', 3500, 'I abstain. Several times over.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 13, 6, 'DebugArea_Lords_Athena', 4000, 'Three candidates, one sandwich and seven abstentions. A typical summit.');

-- Exchange 14: The thermostat.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 14, 0, 'DebugArea_Lords_Morrigan', 0, 'Who set the palace to desert heat again?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 14, 1, 'DebugArea_Lords_Ra', 4500, 'It is the correct temperature for a sun god.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 14, 2, 'DebugArea_Lords_Morrigan', 5000, 'I am from a misty island. My hair has given up.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 14, 3, 'DebugArea_Lords_Anat', 5000, 'Your hair gave up when you started wearing the feathers.');

-- Exchange 15: The coffee.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 15, 0, 'DebugArea_Lords_Baal', 0, 'Can one get a proper cappuccino on this planet, or must I conquer another one?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 15, 1, 'DebugArea_Lords_Nerus', 7000, 'I conquered three worlds for a decent cheese. Two were worth it.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 15, 2, 'DebugArea_Lords_Athena', 6000, 'And the third?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 15, 3, 'DebugArea_Lords_Nerus', 3500, 'We do not talk about the third.');

-- Exchange 16: Being watched.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 16, 0, 'DebugArea_Lords_Athena', 0, 'Does anyone else feel watched? By the ones they call GMs?');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 16, 1, 'DebugArea_Lords_Ra', 5500, 'Gods are not watched. Gods watch.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 16, 2, 'DebugArea_Lords_Baal', 4500, 'One of them just typed ''.gotolocation'' and appeared beside my left foot.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 16, 3, 'DebugArea_Lords_Morrigan', 6500, 'Smile, everyone. They are checking whether we render.');

-- Exchange 17: Honour.
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 17, 0, 'DebugArea_Lords_Ra', 0, 'My Jaffa, remind the council what a Jaffa values above all.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 17, 1, 'DebugArea_Lords_RaJaffa', 5500, 'Honour, my lord.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 17, 2, 'DebugArea_Lords_Ra', 3500, 'Wrong. Me.');
INSERT INTO ambient_chatter_lines (group_id, exchange_id, line_index, speaker_tag, delay_ms, text) VALUES (1, 17, 3, 'DebugArea_Lords_RaJaffa', 3000, '...Indeed.');
