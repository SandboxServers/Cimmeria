# SGW story and oral-history sources for four-zone authoring

Planning source guide, 2026-10-13. Local Claude session `7f6ac8eb-c8a0-438e-a9a3-488273d0be5e` researched Dial the Gate episodes 398, 412, 430, 433, 442 and 459. Its JSONL history preserves detailed investigator notes and timestamped findings, including subagent-written episode notes. Its temporary caption files and separate worktree are no longer present on disk. This guide records the useful findings and their limits; it does not reproduce transcripts. The original public videos remain the primary oral-history sources. Treat speaker recollection, an early pitch and the shipped client/seed as different evidence classes.

## Source inventory and authority

| Source | Relevant finding | Use in these campaigns |
|---|---|---|
| [#430, The Story of Stargate Worlds](https://youtu.be/NAAdmCbqU94), [00:00:39–00:08:29] | Torri Higginson narrates a complete *pitch-level* main-story spine: Ra's return, the Straegis, the Ancient Alliance, and a temporal-prison ending. It names two factions and six race journeys but follows only an SGU recruit. | High-value tone and global stakes; **not** direct proof of Praxis/Cellblock/Castle/Harset/Tollana quest order. |
| [#433, Christopher Klug](https://youtu.be/c-TptxgqSlc), [00:22:03–00:27:32], [00:39:57], [01:11:04] | Klug describes Brad Wright's brief as friendship and leaving no one behind; he identifies #430's outline as the Brad Wright pitch deck written with Steve Garvin and Sean Dugan. He distinguishes advancing story arcs from separate side quests, and says the arcs were written as player actions. | Use as authoring principles: relationships, rescue/loyalty choices, visible player verbs, side quests that support rather than replace the main arc. The deck predates shipped revisions. |
| [#459, Steve Garvin and Sean Dugan](https://youtu.be/3uKha0QJo14), [00:16:17–00:20:24], [00:59:25], [01:16:59–01:22:03] | The content leads describe worlds evolving between visits, story revisions that reduced world size, and planet narratives adapted to concept art. Sean names an Op-CORE prisoner escaping the frozen Castle prison as the scene he most wanted to play; he identifies Colonel Marsh as Op-CORE's leader. | Direct oral-history support for the Cellblock escape premise and for Castle/Harset aftermath stories. Support phased return visits where map and server state allow them. This interview does not supply exact mission scripts or NPC positions. |
| [#412, Dan Elggren](https://youtu.be/EiIhQk3giUs), [00:35:57], [01:08:00–01:20:59] | An archival milestone clip lists mission-giver icons, vendors, pets, gates and rings; Elggren describes environmental cover and NPC encounters, and recalls minigames becoming core. | MMO-life targets: services, readable objectives, cover-aware encounter staging and meaningful minigame moments. Not a source for specific four-zone quest IDs. |
| [#398, Nick LaMartina](https://youtu.be/oO-l9_wbBGY) and [trailer clip](https://youtu.be/ZBYUfqSBDQ0) | Sound/music and the game's presentation were deliberate; the trailer was captured in-engine. | Atmosphere and audio reference only; no mission provenance. |
| [#442, Howard Lyon](https://youtu.be/DOD1oMY6LaY) | Art-direction oral history was researched in the same session. | Consult before claiming a visual motif or world concept was original. No specific four-zone mission link has been established in this guide. |

The local recovery also included episode #416 about David Read, chiefly relevant to the interview briefing, not to zone canon. Notes for all of these episodes survive in the Claude JSONL history as assistant-authored `Write` payloads even though the scratchpad `subs/` and `notes/` directories are empty. The summaries included timestamped caveats: auto captions have uncertain names, some speakers are inferred, and several recollections disagree on how complete beta was.

## Implications by zone

| Zone | Oral-history grounding | Content boundary |
|---|---|---|
| Cellblock | #459 directly supports the frozen-prison Op-CORE escape and Marsh connection. #433's prisoner-letter teaser also fits the confinement tone. | Keep the one-time escape paced. Ambient prisoner/guard moments and one-time aid/evidence quests can reinforce it; the video does not authorize repeatables or exact original event wiring. |
| Castle | #433's friendship/leave-no-one-behind brief supports rescue and medical side stories; #459's world-return idea permits a returnable hub after escape. | The recovered 701–708, 1520 and 1521 data governs actual mission sequence. New stories are labeled authoring, with modest rewards and no invented original status. |
| Harset | #459 identifies Marsh and Op-CORE; the broader two-faction/six-race structure supports distinct Jaffa, Op-CORE and Goa'uld perspectives. | The 73-record census and surviving Harset scripts govern mission selection. Oral history does not revive 39 older-revision missions or supply missing room coordinates. |
| Tollana | #430 gives galactic stakes around Ra/Straegis and alliance politics, but it never narrates the Praxis path or a Tollana chapter. #433 confirms the pitch was an earlier deck. | Base four faction paths on the 51-row mission census, seed steps and client map. New politics and Curia aftermath are explicitly authored; the world-88 Curia map remains a stub. |

## Authoring contract for new side stories

1. Start from a source-backed district, cast member, object or faction conflict in the zone mission matrix. Record the exact source row or map export.
2. Give the player a visible action, a mid-quest state change and a closing conversation or reward. Tie the action to rescue, loyalty, investigation, supply or defense where it fits the zone. These are design principles drawn from #433, not original quest scripts.
3. Label every new mission `CIMMERIA_AUTHORING`; distinguish pitch inspiration and oral-history recollection from shipped 2009 evidence. Do not copy a pitch beat that contradicts the current seed merely to make the arcs match.
4. Target the owner's 5–8 side stories per large hub plus repeatables after recovered arcs pass UAT; rewards begin with useful supplies and modest XP/currency. Cellblock is a one-time exception.
5. Review new quest premises against the original mission/dialog seed and `docs/content/external-data-analysis.md`, which analyzes the on-disk SGW Timeline, Progression and Milky Way mission workbooks. Those workbooks are more direct sources for quest order than the early narrated pitch.

## Evidence gaps

- The caption and note files in the old Claude scratchpad were temporary and are no longer there. The Claude JSONL history preserves the investigator notes and source timestamps; the videos are linked above for rechecking any precise quote or disputed name.
- #430/#433 describe an early pitch. It diverges from seeded SGC_W1, level/planet order and some later Straegis/Furling dialog. For these four campaigns, do not silently replace current mission data with the deck.
- Oral history does not establish exact NPC placement, event-set wiring, reward formulas or whether a specific quest was playable in QA4046. Those questions remain with the map/seed/protocol evidence packets.
