# Lab UAT reliability progress, 2026-10-13

The packet ledger is `docs/analysis/lab-uat-reliability/README.md`. LR-00,
LR-01/01b, LR-02, LR-03, LR-04a/04b, LR-05c and LR-06 merged, but their
specific live proofs remain pending. LR-05a must not ship
until a client-to-server endpoint binding is proven; a defeated client retains
its entity IDs and process-owned UDP port while `unitName(Unit.Player)` is
blank. LR-05b depends on that decision. LR-05d merged after a live stopped-
client login reached character select without manual foreground activation;
its covered-modal path was exercised in flow tests only.

A fresh Praxis Commando reached the first Castle_CellBlock mission and
equipped the corpse pistol. The guard encounter failed because the pistol
showed no ammo and dealt no damage; see issue #1667. The tool-specific
follow-ups are #1701, #1702, #1703, #1706 and #1708. The run did not reach
Livewire or the ring map. No server runtime change from this campaign has
been merged, so a colo release is not indicated by these packets.
