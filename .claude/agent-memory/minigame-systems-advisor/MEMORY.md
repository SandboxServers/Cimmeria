# Memory Index

- [Game implementation status](game-implementation-status.md) — only Livewire is real; 6 names auto-win via placeholder; Alignment/GoauldCrystals are commented-out TODOs; all 11 SWF .upk packages DO ship
- [Chain wiring and gaps](chain-wiring-and-gaps.md) — `start_minigame` params (target_key, on_victory_chains, difficulty 1-5 since CA04); full cell→minigame→cell loop; tech_competency still hardcoded to 1
- [Original session lifecycle](session-lifecycle-original.md) — NO timeout in the original (cleanup is 2 client RPCs, both unimplemented in Rust); abort reports Canceled=0 not Defeat=2; the 3 AbortReason triggers
- [Difficulty ranges](difficulty-ranges.md) — content/def layer asserts 1-5, every per-game table only has tiers 1-4; the mismatch is original
- [Converse minigame evidence](converse-minigame-evidence.md) — "Trump" mechanic from ability passives 778/779/792/793, 20s engagement state, ability-initiated not interact-initiated
- [Mission seed has no minigame columns](mission-seed-has-no-minigame-columns.md) — mission_steps/objectives/tasks column lists; task_type is 1 for all 4358 rows
- [Search without a minigame](search-without-minigame.md) — the chain-1032 interact_tag+add_item+destroy_entity pattern, and NpcInteractionType::Loot for real containers
