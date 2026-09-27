# Memory Index

- [reference_black_market_serve.md](reference_black_market_serve.md) — BM search/serve: BMSearch shape, onBMAuctions layout (92), where each piece lives after the split (base-session, wire, cell-methods)
- [reference_bm_system_seller.md](reference_bm_system_seller.md) — BM reserved system seller: account/player id=1, sequence bounds, ensure pattern, settlement sink, minimal-player column set
- [reference_chat_speaker_flags.md](reference_chat_speaker_flags.md) — ESpeakerFlags bit values, wire layout, Python getSpeakerFlags logic, Ghidra addresses for onPlayerCommunication
- [reference_contact_list_system.md](reference_contact_list_system.md) — Contact list CM 55-60/85-89 wire formats, DB schema, login/logout fanout, code layout, Phase 5 deferred items
- [reference_duel_design_notes.md](reference_duel_design_notes.md) — Duel SM design: confirmed method indices, state layout, file skeleton, failure modes, RE gaps to fill
- [reference_chat_channel_routing.md](reference_chat_channel_routing.md) — EChannel routing table (which channels are truly wired vs. registered-but-unsupported), legacy onError feedback precedent, shared feedback-send pattern, real gaps (tell/team/squad/command)
