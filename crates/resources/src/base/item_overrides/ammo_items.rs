//! The 15 special-ammo reserve items (ammo campaign AM-07, issue #1026).
//!
//! AM-F seeded ids 9000-9014 in `db/resources/Items/Seed/ammo_items.sql` and
//! mapped each to its `EAmmoType` in `ammo_item_types.sql`. None of them is in
//! the shipped `CookedDataItems.pak` (its highest id is 8951), so without these
//! entries the client has no name, icon or stack cap for them. Each entry here
//! is a whole generated `COOKED_ITEM` ([`super::generate_item_xml`]).
//!
//! **Icons.** `AmmoType_Icons` is the client's own ammo-type imageset, declared
//! in `TaharezLook.scheme` and used by `Bandolier.lua` / `WeaponBar.lua` for the
//! loaded-ammo icon. Its image names are exactly the `EAmmoType` labels, so an
//! item shows the same icon the weapon bar shows once that type is loaded.
//!
//! **Agreement with the seed.** `Name`, `MaxStackSize` (500 rounds),
//! `TechComp` (1), `IsSellable` (flags 3072 = `CanBeSold | CanBeDeleted`) and
//! the `ContainerSet`s (1, 15, 17) must match `ammo_items.sql`; a unit test
//! parses the seed and checks every row. `Description` is cooked-only (the
//! server never sends `resources.items.description`), so it is written for the
//! tooltip instead of copying the seed's category label. It says how to use
//! the rounds, not what they do: the per-type effects land in later packets.
//!
//! RECONSTRUCTION: the 2009 client has no ammo item category
//! (`docs/reverse-engineering/findings/ammo-system.md` Q1); these are
//! restoration-team items, not recovered ones.

use super::NewItem;

/// Bags a reserve stack may sit in: the main bag, the crafting bag and the
/// personal vault, matching `container_sets = '{1,15,17}'` in the seed.
const AMMO_CONTAINER_SETS: &[u32] = &[1, 15, 17];

/// Rounds per stack, matching `max_stack_size = 500` in the seed.
const AMMO_MAX_STACK_SIZE: u32 = 500;

/// The special-ammo reserve items, in item-id (and `EAmmoType`) order.
pub const AMMO_ITEMS: &[NewItem] = &[
    NewItem {
        item_id: 9000,
        name: "Armor Piercing Rounds",
        description: "Special ammunition. Select Armor Piercing in a compatible weapon's ammo menu to fire these rounds.",
        icon_location: "set:AmmoType_Icons image:Bullet_Armor_Piercing",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9001,
        name: "Hollow Point Rounds",
        description: "Special ammunition. Select Hollow Point in a compatible weapon's ammo menu to fire these rounds.",
        icon_location: "set:AmmoType_Icons image:Bullet_Hollow_Point",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9002,
        name: "Incendiary Rounds",
        description: "Special ammunition. Select Incendiary in a compatible weapon's ammo menu to fire these rounds.",
        icon_location: "set:AmmoType_Icons image:Bullet_Incendiary",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9003,
        name: "EMP Rounds",
        description: "Special ammunition. Select EMP in a compatible weapon's ammo menu to fire these rounds.",
        icon_location: "set:AmmoType_Icons image:Bullet_EMP",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9004,
        name: "Explosive Rounds",
        description: "Special ammunition. Select Explosive in a compatible weapon's ammo menu to fire these rounds.",
        icon_location: "set:AmmoType_Icons image:Bullet_Explosive",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9005,
        name: "Poison Darts",
        description: "Special ammunition. Select Poison in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Poison",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9006,
        name: "Disease Darts",
        description: "Special ammunition. Select Disease in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Disease",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9007,
        name: "Tranquilizer Darts",
        description: "Special ammunition. Select Tranquilizer in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Tranquilizer",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9008,
        name: "EMP Darts",
        description: "Special ammunition. Select EMP in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_EMP",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9009,
        name: "Radioactive Darts",
        description: "Special ammunition. Select Radioactive in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Radioactive",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9010,
        name: "Stim Darts",
        description: "Special ammunition. Select Stim in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Stim",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9011,
        name: "Coagulant Darts",
        description: "Special ammunition. Select Coagulant in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Coagulant",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9012,
        name: "Nanite Darts",
        description: "Special ammunition. Select Nanite in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Nanites",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9013,
        name: "Antidote Darts",
        description: "Special ammunition. Select Antidote in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Antidote",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
    NewItem {
        item_id: 9014,
        name: "Adrenaline Darts",
        description: "Special ammunition. Select Adrenaline in a compatible weapon's ammo menu to fire these darts.",
        icon_location: "set:AmmoType_Icons image:Dart_Adrenaline",
        max_stack_size: AMMO_MAX_STACK_SIZE,
        tech_comp: 1,
        is_sellable: true,
        container_sets: AMMO_CONTAINER_SETS,
    },
];
