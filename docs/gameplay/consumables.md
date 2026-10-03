---
title: "Consumables"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Consumables

> **Last updated**: 2026-09-28
> **Status**: Implemented server-side, not yet tested in a client. Heal items (the Health Slappack, the Health and Focus Heal Consumables) and the Mark III / V / VII / X stimpacks work from their seed data with no content chain. Stealth, Energy and Disguise boosts do nothing yet.
> **Design and code**: [consumable-via-onitemuse-pattern.md](../content/consumable-via-onitemuse-pattern.md#native-consumables-items_event_sets-event-5) (which items, how a use is decided) and decision 28 of [abilities-and-effects-system.md](../architecture/abilities-and-effects-decisions-23-33.md#28-native-consumables-the-base-consumes-before-the-cell-applies-and-timed-stat-buffs-live-in-their-own-ledger) (why the item is paid for first, how buffs stack).

What a player sees when they double-click a heal or buff item in their bags.

## Heals

| Item | Heals |
|---|---|
| Health Slappack TC1 (2893, 4735) | 500 health |
| Health Consumable (6132, 6239, 6737-6744) | 162 to 353 health, by tier |
| Focus Heal Consumable (6106, 6237, 6243, 6244, 6253, 6255, 6257, 6734-6736) | 384 to 1420 focus, by tier |

A use heals the stated amount at once, up to the pool's maximum, and takes one item off the stack you clicked. The health or focus bar moves and the stack count drops.

A use that would do nothing is refused, and the item is kept:

| When | The chat shows |
|---|---|
| Your health (for a health item) or focus (for a focus item) is already full | "You are already at full health." / "You are already at full focus." |
| You are dead | "You cannot use that while dead." |

## Stimpacks

| Tier | Items | Raises |
|---|---|---|
| Mark III | 6677-6682 | one attribute by 5 |
| Mark V | 6697, 6719, 6722, 6725, 6728, 6731 | two attributes, by 7 and 3 |
| Mark VII | 6717, 6720, 6723, 6726, 6729, 6732 | two attributes, by 7 each |
| Mark X | 6718, 6721, 6724, 6727, 6730, 6733 | two attributes, by 10 each |

The attributes are Coordination, Engagement, Fortitude, Intellect, Morale and Perception. A stimpack lasts one hour. Using one raises the attribute at once, adds a buff icon with the hour counting down, and takes one item. When the hour is up the attribute returns to its value and the icon goes.

- **One buff per attribute.** Using a stimpack on an attribute that is already buffed replaces the old buff with the new one, whatever the tiers: a Mark III Coordination (+5) followed by a Mark V Coordination/Engagement leaves Coordination at +7, not +12, and starts a fresh hour. The same stimpack twice refreshes the hour.
- **Different attributes add up.** A Coordination stim and an Engagement stim both hold.
- A stimpack is never refused for being "full".
- What the attributes do: Coordination raises your ranged hit quality and Engagement your melee hit quality, Perception lowers an attacker's hit quality against you, Fortitude resists health damage and Intellect focus damage ([combat-system.md](combat-system.md)). Morale has no effect on the server yet.

**Known limits.** A buff ends early if you log out, change zone or travel through a gate, or respawn in another zone: it is not saved, although the item data says its hour should keep counting while offline. It survives dying and respawning in the same zone. Whether the buff icon survives a same-zone respawn, and which icon the client draws for a stimpack effect, have not been checked in a client.

## Items that do nothing yet

Using one of these shows "This item has no effect yet." in chat, and the item is kept.

| Items | Why |
|---|---|
| Stealth Boost Consumable (6206, 6762-6770) | Nothing on the server uses stealth rating yet. |
| Energy Boost Consumable (6209, 6753-6761) | Nothing on the server uses the energy pool yet. |
| Disguise Boost Consumable (8403, 6196, 6745-6752) | Nothing on the server uses disguise rating yet. |
| Antidotes (6577, 6597-6599, 6656, 6657, 6659-6662, 6664-6666) | The effects that would remove a condition are not implemented. |

Mission items keep working through their missions (the Ambernol vial, radios, scanners). Many mission items carry a leftover "Heal Focus" binding in the item data; it is ignored, so using a quest item never heals you.

## Testing

In-game steps: [unified-uat.md, Consumables](../guides/unified-uat.md#consumables).
