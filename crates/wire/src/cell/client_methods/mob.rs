//! SGWMob's own ClientMethods (indices number SGWMob, not SGWPlayer; 27/28 on
//! a player are Communicator).

/// `onAggressionOverrideUpdate(INT8 aAggressionLevel)`. Ghidra-confirmed: the
/// client registers it through `MemberCallback<GameMob, ...>` at 0x00d31cd0
/// (paired with `Event_NetIn_onAggressionOverrideCleared`), and the Update
/// handler at 0x00d31bd0 reads the `aAggressionLevel` INT8 arg and stores it at
/// `GameMob + 0x16c`. NA33:
/// docs/reverse-engineering/findings/npc-aggression-broadcast.md.
pub const ON_AGGRESSION_OVERRIDE_UPDATE: u16 = 27;
/// `onAggressionOverrideCleared()`, declared right after the Update method in
/// `entities/defs/SGWMob.def`. Same Ghidra and NA33 source as the Update method.
pub const ON_AGGRESSION_OVERRIDE_CLEARED: u16 = 28;
