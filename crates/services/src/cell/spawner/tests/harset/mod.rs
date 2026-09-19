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
//! - [`ability_sets`]: the two new ability sets, and a loader round-trip
//!   proving a spawn row picks both up through `load_spawns_from_db`.

mod ability_sets;
mod factions;
mod templates;

use crate::cell::combat::{HOSTILE_FACTION, NPC_DEFAULT_ABILITY};
use crate::cell::spawner::load_spawns_from_db;
use crate::test_support::require_db_or_skip;

/// Template-id block the packet ledger reserves for Harset.
const BLOCK_MIN: i32 = 200;
const BLOCK_MAX: i32 = 299;

/// The documented per-template respawn default for this packet (D-H17).
const RESPAWN_DEFAULT: i32 = 300;

/// Ability 584 "Staff Auto Attack" — the single member of ability set 4.
const STAFF_AUTO_ATTACK: i32 = 584;
/// Ability 712 "Ribbon Device Auto Attack" — the single member of set 5.
const RIBBON_AUTO_ATTACK: i32 = 712;

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

/// World 57 = Harset, confirmed against `resources.worlds`.
const HARSET_WORLD_ID: i32 = 57;
