//! Live-DB regression guards for the Harset entity templates seeded by packet
//! H11 (`docs/analysis/harset-rebuild/work-packets.md`).
//!
//! These are seed-data guards, not fixture tests: the rows under test are the
//! production seed in `db/resources/Entities/Seed/entity_templates.sql` and
//! `db/resources/Abilities/Seed/ability_set*.sql`. Every test here fails if the
//! H11 rows are removed — the counts and the per-id assertions are the point.
//!
//! The three defects they pin, all from
//! `docs/analysis/harset-rebuild/audit.md`:
//!
//! - **H-B7** — `respawn_secs` was NULL on all 153 templates and all 167 spawn
//!   rows, so `mark_npc_dead` never stamped `respawn_at` and clearing the Harset
//!   plaza emptied it until server restart.
//! - **H-B8** — `ability_set_id` was set on 3 of 153 templates; every other NPC
//!   fell back to `NPC_DEFAULT_ABILITY` (592, Pistol Shot), so Jaffa with staff
//!   models fired a Tau'ri pistol. Template 163 (Petbe) additionally shipped
//!   with NULL faction, level and alignment.
//! - **spec L-01** — Harset has no random loot; `loot_table_id` must stay NULL
//!   across the whole zone roster.
//!
//! Split three ways by what each guard is about:
//!
//! - [`templates`]: the seeded rows themselves — ability set present, respawn
//!   delay, no loot table, allocated names, a renderable appearance, a
//!   resolvable display name.
//! - [`factions`]: the faction design rules — Petbe's NULL-column fix, the
//!   talk-vs-kill template pairs, and the two Praxis guard rows shared with
//!   Castle.
//! - [`ability_sets`]: the two new ability sets, a loader round-trip proving a
//!   spawn row picks them up through `load_spawns_from_db`, and (packet H09)
//!   the composite primary key that lets a set hold more than one ability at
//!   all, plus the loader-to-`choose_npc_ability` round-trip over a multi-row
//!   set.
//! - [`world57_placement`]: live-DB guards on the world-57 population and
//!   named regions placed by pass B (packets H14 and H15) -- tags the merged
//!   mission chains already dispatch on, the D-H03 no-hostiles rule, the
//!   recorded on-mesh/off-mesh verdict per row, and the region volumes.

mod ability_sets;
mod factions;
mod templates;
mod world57_placement;

use crate::cell::combat::{HOSTILE_FACTION, NPC_DEFAULT_ABILITY};
use crate::cell::spawner::{load_spawn_templates, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// Template-id block the packet ledger reserves for Harset.
const BLOCK_MIN: i32 = 200;
const BLOCK_MAX: i32 = 299;

/// The documented per-template respawn default for this packet (D-H17).
const RESPAWN_DEFAULT: i32 = 300;

/// Ability 584 "Staff Auto Attack" — the ranged half of ability set 4, and
/// (lowest id in the set) the Jaffa's primary pick.
const STAFF_AUTO_ATTACK: i32 = 584;
/// Ability 710 "Staff Melee AA" — the melee half of set 4, added by packet
/// H09 once the composite key made a second row possible.
const STAFF_MELEE_AA: i32 = 710;
/// Ability 712 "Ribbon Device Auto Attack" — the ranged half of set 5.
const RIBBON_AUTO_ATTACK: i32 = 712;
/// Ability 711 "Ribbon Device Melee AA" — the melee half of set 5. Sorts
/// *below* 712, so it is set 5's primary pick; see `ability_sets.rs`.
const RIBBON_MELEE_AA: i32 = 711;

/// The full membership of the two Harset ability sets after H09, in the
/// ascending-`ability_id` order every loader's `array_agg(... ORDER BY
/// asa.ability_id)` returns and `choose_npc_ability` then selects from.
const HARSET_ABILITY_SETS: [(i32, [i32; 2]); 2] = [
    (4, [STAFF_AUTO_ATTACK, STAFF_MELEE_AA]),
    (5, [RIBBON_MELEE_AA, RIBBON_AUTO_ATTACK]),
];

/// One seeded template per Harset ability set, as
/// `(ability_set_id, members, template_id)`, for the H09 melee-reach guard.
///
/// 200 "Mala'c" is one of the thirteen templates on set 4; 210 "Haughty
/// Goa'uld" is one of the two on set 5. Both are used rather than a synthetic
/// record so the guard also pins the template → set wiring.
const HARSET_SET_PROBE_TEMPLATES: [(i32, [i32; 2], i32); 2] = [
    (4, [STAFF_AUTO_ATTACK, STAFF_MELEE_AA], 200),
    (5, [RIBBON_MELEE_AA, RIBBON_AUTO_ATTACK], 210),
];

/// Every `class = 'mob'` template H11 seeds, as `(template_id, name)`.
const MOB_TEMPLATES: [(i32, &str); 24] = [
    (200, "Mala'c"),
    (201, "Lo'rak"),
    (202, "Bra'hin"),
    (203, "Ra's Jaffa Infiltrator"),
    (204, "Ra's Former Jaffa"),
    (205, "Suspicious Jaffa"),
    (206, "Angry Jaffa"),
    (207, "Jaffa Volunteer"),
    (208, "Free Jaffa Attacker"),
    (209, "Anat's Royal Guard"),
    (210, "Haughty Goa'uld"),
    (211, "Ashrak Assassin"),
    (212, "Hansen"),
    (213, "Jacobs"),
    (214, "Blackstock"),
    (215, "Opheltes"),
    (216, "Lance Corporal Grogan"),
    (217, "Dawson"),
    (218, "NID Operative"),
    (219, "Storage Lo'taur"),
    (220, "Lethander's Contact"),
    (221, "Petbe (hostile)"),
    (222, "Lance Corporal Grogan (hostile)"),
    (223, "Dawson (hostile)"),
];

/// Every prop template H11 seeds. Props are `class = 'being'`: they never
/// AI-tick (`all_npc_entity_ids` admits only class_id `0x04`) so they carry
/// no ability set and no respawn.
const PROP_TEMPLATES: [(i32, &str); 9] = [
    (240, "Harset Camera Location"),
    (241, "Replitech Crate"),
    (242, "Harset Storage Crate"),
    (243, "Harset Shield Tower"),
    (244, "Petbe's Quarters Search Object"),
    (245, "Anat's Symbiote Tank"),
    (246, "Strange Beacon Technology"),
    (247, "Devlin's Device"),
    (248, "Harset Monitoring Device Anchor"),
];

/// Harset templates that predate this packet: Ba'al, Anat, Lethander,
/// CaptCoppleman, Nerus, Moh'Katan, the two Praxis guard rows, Petbe and the
/// mission-742 merchant basket. Folded into the loot sweep so spec L-01 is
/// asserted across the whole zone roster, not just the new rows.
const PREEXISTING_HARSET_TEMPLATES: [i32; 10] = [42, 43, 46, 48, 53, 54, 159, 160, 163, 164];

/// Sentinel spawn id for the loader round-trip. Sits well clear of every
/// `0x7000_xxxx` base already in use by `crates/services` (highest today is
/// `0x7000_5000`). Deleted by exact id before any assertion runs.
const SENTINEL_SPAWN_ID: i32 = 0x7000_6100;

/// Sentinel `ability_sets` row for the H09 three-row chooser round-trip.
/// A three-member set has to be built rather than borrowed: padding the
/// shipped set 4 to three rows would put a third ability on every live
/// Harset Jaffa and change their attack rate a second time.
const SENTINEL_ABILITY_SET_ID: i32 = 0x7000_6200;

/// Sentinel `entity_templates` row that points at [`SENTINEL_ABILITY_SET_ID`],
/// so the three abilities travel the real `load_spawn_templates` query rather
/// than being hand-placed in a `SpawnRecord`.
const SENTINEL_TEMPLATE_ID: i32 = 0x7000_6300;

/// The three abilities the sentinel set holds, in the ascending order the
/// loader must return them in. They are real ids because
/// `ability_set_abilities_ability_id_fkey` is `ON DELETE/UPDATE RESTRICT` —
/// an invented `0x7000_xxxx` ability id would be rejected by the FK, unlike
/// [`SENTINEL_SPAWN_ID`] which references nothing.
const SENTINEL_SET_ABILITIES: [i32; 3] = [STAFF_AUTO_ATTACK, STAFF_MELEE_AA, RIBBON_AUTO_ATTACK];

/// World 57 = Harset, confirmed against `resources.worlds`.
const HARSET_WORLD_ID: i32 = 57;
