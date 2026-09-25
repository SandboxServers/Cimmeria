---
name: qr-direction-and-cover
description: The python QR beta branches were inverted (positive QR -> more misses, flat damage); NA32 swapped them. Cover QR units from client alias.xml; any new QR term must be checked against the sampler direction
metadata:
  type: project
---

**The python sampler was backwards.** `AbilityManager.py:181-184` used
`betavariate(a, a + qr*m)` for qr >= 0, so the mean of `qr_rand` fell as the
attacker got stronger. With the `(1 + qr)` damage term, expected damage was
nearly flat in QR (0.36 at QR 0, 0.43 at +1.5) and result codes inverted
(QR +1.5: ~45% Miss/Glancing). A -1 QR cover shift produced the SAME damage
(25 vs 25 on one seed) with more "Critical" floaters. NA32 (2026-09-25,
branch `npcai/na32-cover-defense-stepback`) swapped the branches in
`combat/damage/qr.rs`; QR 0 is unchanged.

**Why:** client `alias.xml:204-205` says accuracy is +0.01 outgoing QR and
defense -0.01 incoming QR; designer buffs ("+200 Accuracy", "+1 QR") only make
sense if higher QR is better for the attacker.

**How to apply:** before adding any QR-shifting term (crouch, tracking,
stabilization), test that it moves *damage* the intended way through
`apply_damage_to_target`, not just the QR number. If someone "restores python
parity" on the sampler, `expected_damage_rises_with_qr` and the NA32 cover
test fail by design.

**Cover units (all client `alias.xml`, ORIGINAL-DATA):** coverDefense -0.01
QR/pt (235), coverAccuracy +0.01 vs covered target (234), coverQRModifier 1
QR/pt both sides behind cover (216). NA32 caps penetration at the cover
(`max(0, def - pen)`) because every attacker-side cover text is penetration.
Cover Stance = effect 4565 +100 (not ability text +200). With `(1+qr)`, a
covered guard takes ~5% damage; tune via a `CoverDefense` NVP on 4565.

SGW.exe resolves no hits: its only cover string is the `CoverQRModifier` Lua
label at 0x01956eac. See [[shipped-data-combat-evidence]].
