# Sequence routing investigation

> 2026-10-06; static Ghidra inspection of QA `SGW.exe`.
> Image base `0x00400000`; x86 32-bit. SHA-256:
> `109f307763a5c6c59ff484840739860bdc7163092f0644343d0b2c03e4925783`.
> Companion: [evidence ledger](evidence-ledger.md).

The existing chain's `ViewType=0` is not proven wrong. Changing it to 3 alone
does not change the designer-event activation-filter mapping recovered below.
Playback still needs client observation and source/map/culling diagnostics.

| Finding | Address-cited evidence | Confidence |
|---|---|---|
| Source entity must resolve | `0x00d05790` reads SourceID and calls `0x00dd0de0`; a zero result skips request construction and scheduling. | CONFIRMED static; independently agrees with `docs/reverse-engineering/findings/ability-client-hook-anchors.md`. |
| ViewType reaches request offset +0x1c | `0x00d13780` reads ViewType using byte getter `0x00d434d0`, then stores its integer conversion at +0x1c. | CONFIRMED static. |
| ViewType participates in distance handling | `0x00d06dd0` exempts request view values 1 and 2, and event 0x1389, from its distance rejection branch. Values 0 and 3 take the same branch. It later passes request +0x1c into `0x00d01bd0`. | CONFIRMED static branch; selected distance threshold and live entity position require runtime observation. |
| Event 6000 uses the designer callback | Registry initializer `0x00d02de0` binds event 6000 to `0x00d11760`. `0x00d01bd0` dispatches by event ID. | CONFIRMED static. |
| Designer callback forwards activation-view filter | `0x00d11760` finds designer-event objects, sets their event identifier, looks up its third stack argument through `0x00d0f480` against the table at `0x01ef2308`, and invokes virtual offset +0x170. | CONFIRMED dataflow; Ghidra's inferred prototype omits the third argument, so do not rely on its displayed two-argument prototype. |
| Filter mapping collapses 0 and 3 | Tail of `0x00d02de0` initializes view→activation-array entries: 0→[0], 1→[1], 2→[2], 3→[0], 4→[1]. | CONFIRMED static table construction. This is an activation filter, not proof that every downstream camera behavior is identical. |
| Designer index gates activation | `USeqEvent_Designer` virtual 92 at `0x006a0a10` checks enabled flag, non-test mode and event identifier equal to designer-index byte at +0xec plus 6000; it delegates to `USequenceEvent` virtual 92 at `0x006a0360`. | CONFIRMED static. |
| Director track can change the view target | `UInterpTrackDirector` virtual 79 at `0x007a5590` resolves its group/player controller, saves the prior target and invokes controller virtual +0x3a4 with the selected camera actor; restore path also invokes this slot. | CONFIRMED call structure; naming the controller slot as SetViewTarget is INFERENCE pending prototype confirmation. |

Current server emitter is `crates/cell-content/src/cell/content/executor/dispatch.rs`
`Action::PlaySequence`: it uses the invoking entity for SourceID and TargetID and hardcodes view 0. Marsh removal therefore does not by itself prove the sequence source disappears. The seed comment discussing a future
view 3 change is a hypothesis, not a recovered original Straegis contract.

The campaign's camera experiment must hold sequence/source/target/instance constant
when comparing view modes, verify source presence and map loading, record camera
acquire/restore and interruption, and repeat for an observer. A byte-correct packet
or a server-side sequence log cannot establish successful Director playback.

No binary names, types, comments or code were modified during this inspection.

## Follow-up: activation gates and camera controller (2026-10-07)

Read-only decompilation on the same open `SGW.exe` closes one naming uncertainty:
`APlayerController_execSetViewTarget` at `0x0052ee10`, identified by its UE3
registration-table name, dispatches virtual slot `+0x3a4`. The Director at
`0x007a5590` invokes that same slot. Calling this operation SetViewTarget is now
high-confidence static evidence rather than a slot-name inference. The displayed
wrapper argument types are imperfect; this does not recover the complete ABI.

The Director first casts its group actor through `0x0055fcc0`. That routine walks
the object's class ancestry against a class constructed by `0x0077a870`, whose
literal class name is `PlayerController`. A null result skips the Director work.
Thus successful designer activation alone is insufficient: the Director group
must resolve to a PlayerController and its selected camera group must contain a
valid actor. Whether those references resolve in the live Straegis scene remains
unverified.

`USequenceEvent` slot92 at `0x006a0360` also checks the active level, object/parent
sequence state, nonzero originator, a conditional instigator check, trigger count
and retrigger time before dispatching slot95. Field names for the checks are
inferences from their operations; offsets are explicit: count `+0xd0`, limit
`+0xd4`, time interval `+0xd8`, last trigger time `+0xcc`. Slot95 at `0x006a8fe0`
stores its two actor arguments at `+0xc4` and `+0xc8`, updates time/count, marks
selected output links active and queues the event on the parent sequence.
The decompiler exposes an undeclared output-array argument as `unaff_retaddr`;
do not use that inferred prototype as a callable ABI.

CA-04A / CA-U10 should therefore record event activation, parent enable state,
trigger counters/timing, Director group controller, camera actor resolution and
camera restoration separately. The static evidence still does not establish the
current failure's cause, an original view mode, or a need for a server change.
