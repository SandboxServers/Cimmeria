//! The [`NameBook`]: one immutable snapshot of the seed's names.

use std::collections::HashMap;

/// A seed table the book reads names from. The order is the order of the
/// `names.loaded` fields and the gap list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Table {
    /// `items.name`, by `item_id` (the item type, not an instance).
    Items,
    /// `abilities.name`, by `ability_id`.
    Abilities,
    /// `effects.name`, by `effect_id`.
    Effects,
    /// `missions.mission_defn` (the mission's name; `mission_label` is its
    /// zone or group), by `mission_id`.
    Missions,
    /// `mission_steps.step_display_log_text`, by `step_id`.
    MissionSteps,
    /// `mission_objectives.display_log_text`, by `objective_id`.
    MissionObjectives,
    /// `dialogs.name`, else the dialog's first `dialog_set_maps.topic_text`,
    /// by `dialog_id`.
    Dialogs,
    /// `dialog_sets.name`, by `dialog_set_id`.
    DialogSets,
    /// `speakers.name`, by `speaker_id`.
    Speakers,
    /// `entity_templates.template_name`, by `template_id`.
    Templates,
    /// `monikers.name`, by `moniker_id`.
    Monikers,
    /// `texts.text`, by `moniker_id` (an entity's `name_id`).
    Texts,
    /// `error_texts.moniker_name`, by `error_id` (the client's
    /// `CONDITION_FEEDBACK_*` code).
    ErrorTexts,
    /// `stargates.name`, by `stargate_id`.
    Stargates,
    /// `respawners.name`, by `respawner_id`.
    Respawners,
    /// `spawn_sets.name`, by `set_id`.
    SpawnSets,
    /// `containers.name`, by `container_id`.
    Containers,
    /// `item_lists.name`, by `item_list_id`.
    ItemLists,
    /// `applied_science.name`, by `id`.
    AppliedScience,
    /// `worlds.world`, by `world_id`.
    Worlds,
    /// `content_chains.description`, by `chain_id`: the designer's one-line
    /// summary of a content chain (`701 - Gerschon interact (Human): offer
    /// dialog 2573`), logged as `chain_name`.
    Chains,
    /// `loot_tables.description`, by `loot_table_id`.
    LootTables,
    /// `trainer_ability_lists.description`, by `list_id`.
    TrainerAbilityLists,
    /// `event_sets.name` (the Kismet event set), by `event_set_id`.
    EventSets,
    /// `sequences.kismet_script_name`, by `sequence_id` (the client's
    /// `KismetEventSetSeqID`).
    Sequences,
    /// `dialog_set_maps.topic_text`, by `dialog_set_map_id`: the line the
    /// player clicks to open the entry's dialog.
    DialogSetMaps,
}

impl Table {
    /// Every table, in order.
    pub const ALL: [Table; 26] = [
        Table::Items,
        Table::Abilities,
        Table::Effects,
        Table::Missions,
        Table::MissionSteps,
        Table::MissionObjectives,
        Table::Dialogs,
        Table::DialogSets,
        Table::Speakers,
        Table::Templates,
        Table::Monikers,
        Table::Texts,
        Table::ErrorTexts,
        Table::Stargates,
        Table::Respawners,
        Table::SpawnSets,
        Table::Containers,
        Table::ItemLists,
        Table::AppliedScience,
        Table::Worlds,
        Table::Chains,
        Table::LootTables,
        Table::TrainerAbilityLists,
        Table::EventSets,
        Table::Sequences,
        Table::DialogSetMaps,
    ];

    /// The `resources` table name.
    pub fn as_str(self) -> &'static str {
        match self {
            Table::Items => "items",
            Table::Abilities => "abilities",
            Table::Effects => "effects",
            Table::Missions => "missions",
            Table::MissionSteps => "mission_steps",
            Table::MissionObjectives => "mission_objectives",
            Table::Dialogs => "dialogs",
            Table::DialogSets => "dialog_sets",
            Table::Speakers => "speakers",
            Table::Templates => "entity_templates",
            Table::Monikers => "monikers",
            Table::Texts => "texts",
            Table::ErrorTexts => "error_texts",
            Table::Stargates => "stargates",
            Table::Respawners => "respawners",
            Table::SpawnSets => "spawn_sets",
            Table::Containers => "containers",
            Table::ItemLists => "item_lists",
            Table::AppliedScience => "applied_science",
            Table::Worlds => "worlds",
            Table::Chains => "content_chains",
            Table::LootTables => "loot_tables",
            Table::TrainerAbilityLists => "trainer_ability_lists",
            Table::EventSets => "event_sets",
            Table::Sequences => "sequences",
            Table::DialogSetMaps => "dialog_set_maps",
        }
    }

    /// The table named `name`, the inverse of [`Table::as_str`].
    pub fn from_name(name: &str) -> Option<Table> {
        Table::ALL.into_iter().find(|t| t.as_str() == name)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// The names in every [`Table`], keyed by the row's id. Built once by the
/// loader (or a test), then read-only: a reload builds a new book and swaps
/// it in whole (see [`crate::NameBookHandle`]).
///
/// Every lookup takes any integer id type that widens to `i64` (the seed
/// keys are `integer`, `monikers.moniker_id` is `bigint`, the wire uses
/// `u32`), and returns `None` for an unknown id, a blank name or a
/// placeholder. Log the result as an `Option` field so an unresolved name is
/// left off the line, never written as `"unknown"`.
#[derive(Debug, Default, Clone)]
pub struct NameBook {
    tables: [HashMap<i64, Box<str>>; Table::ALL.len()],
    /// `entity_templates.name_id`, by `template_id`: the key into
    /// [`Table::Texts`] for the template's player-facing name.
    template_name_ids: HashMap<i64, i64>,
}

impl NameBook {
    /// An empty book: every lookup returns `None`. The process starts with
    /// one until the boot load lands.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Record `name` for `id` in `table`. The loader filters blanks and
    /// placeholders before calling this; tests call it directly.
    pub fn insert(&mut self, table: Table, id: i64, name: &str) {
        self.tables[table.index()].insert(id, name.into());
    }

    /// Record a template's `name_id`.
    pub fn insert_template_name_id(&mut self, template_id: i64, name_id: i64) {
        self.template_name_ids.insert(template_id, name_id);
    }

    /// The name for `id` in `table`.
    pub fn get(&self, table: Table, id: impl Into<i64>) -> Option<&str> {
        self.tables[table.index()].get(&id.into()).map(|n| &**n)
    }

    /// How many ids in `table` have a name.
    pub fn len(&self, table: Table) -> usize {
        self.tables[table.index()].len()
    }

    /// True when no table has a name (the book before the boot load).
    pub fn is_empty(&self) -> bool {
        self.tables.iter().all(HashMap::is_empty)
    }

    /// `items.name` for an item type id (`item_id` in the seed, the
    /// `design_id` / `item_type_id` of an instance).
    pub fn item(&self, item_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Items, item_id)
    }

    /// `abilities.name`.
    pub fn ability(&self, ability_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Abilities, ability_id)
    }

    /// `effects.name`.
    pub fn effect(&self, effect_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Effects, effect_id)
    }

    /// `missions.mission_defn`, the mission's name.
    pub fn mission(&self, mission_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Missions, mission_id)
    }

    /// `mission_steps.step_display_log_text`.
    pub fn mission_step(&self, step_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::MissionSteps, step_id)
    }

    /// `mission_objectives.display_log_text`.
    pub fn mission_objective(&self, objective_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::MissionObjectives, objective_id)
    }

    /// `dialogs.name`, else the topic text of the first dialog-set entry
    /// that opens the dialog (the seed names only a handful of dialogs).
    pub fn dialog(&self, dialog_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Dialogs, dialog_id)
    }

    /// `dialog_sets.name`.
    pub fn dialog_set(&self, dialog_set_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::DialogSets, dialog_set_id)
    }

    /// `speakers.name`.
    pub fn speaker(&self, speaker_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Speakers, speaker_id)
    }

    /// `entity_templates.template_name`, the designer's name for the
    /// template (`template_name` in logs, D-NT5).
    pub fn template(&self, template_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Templates, template_id)
    }

    /// The player-facing name of a template's entities: the
    /// [`text`](Self::text) of its `name_id` (`entity_name` in logs, D-NT5).
    pub fn template_display(&self, template_id: impl Into<i64>) -> Option<&str> {
        let name_id = *self.template_name_ids.get(&template_id.into())?;
        self.text(name_id)
    }

    /// `monikers.name`.
    pub fn moniker(&self, moniker_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Monikers, moniker_id)
    }

    /// `texts.text` for a moniker id, such as an entity's `name_id`.
    pub fn text(&self, moniker_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Texts, moniker_id)
    }

    /// `error_texts.moniker_name` for an error code
    /// (`CONDITION_FEEDBACK_InvalidEntity` for 0).
    pub fn error_code(&self, error_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::ErrorTexts, error_id)
    }

    /// `stargates.name`.
    pub fn stargate(&self, stargate_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Stargates, stargate_id)
    }

    /// `respawners.name`.
    pub fn respawner(&self, respawner_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Respawners, respawner_id)
    }

    /// `spawn_sets.name`.
    pub fn spawn_set(&self, set_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::SpawnSets, set_id)
    }

    /// `containers.name`.
    pub fn container(&self, container_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Containers, container_id)
    }

    /// `item_lists.name` (vendor and loot lists).
    pub fn item_list(&self, item_list_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::ItemLists, item_list_id)
    }

    /// `applied_science.name`.
    pub fn applied_science(&self, id: impl Into<i64>) -> Option<&str> {
        self.get(Table::AppliedScience, id)
    }

    /// `worlds.world`, the world's name (`world_name` in logs).
    pub fn world(&self, world_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Worlds, world_id)
    }

    /// `content_chains.description`, the chain's one-line summary
    /// (`chain_name` in logs).
    pub fn chain(&self, chain_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Chains, chain_id)
    }

    /// `loot_tables.description`.
    pub fn loot_table(&self, loot_table_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::LootTables, loot_table_id)
    }

    /// `trainer_ability_lists.description`.
    pub fn trainer_ability_list(&self, list_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::TrainerAbilityLists, list_id)
    }

    /// `event_sets.name`, the Kismet event set.
    pub fn event_set(&self, event_set_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::EventSets, event_set_id)
    }

    /// `sequences.kismet_script_name` for a `KismetEventSetSeqID`.
    pub fn sequence(&self, sequence_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::Sequences, sequence_id)
    }

    /// `dialog_set_maps.topic_text`, the entry's clickable topic line.
    pub fn dialog_set_map(&self, dialog_set_map_id: impl Into<i64>) -> Option<&str> {
        self.get(Table::DialogSetMaps, dialog_set_map_id)
    }

    /// The `world_id` of the world named `world`, the reverse of
    /// [`world`](Self::world). Most seams carry the world by name only;
    /// this pairs it with its ID. A scan, but `resources.worlds` is a
    /// few dozen rows and the callers are Discord-rate seams.
    pub fn world_id(&self, world: &str) -> Option<i64> {
        self.tables[Table::Worlds.index()]
            .iter()
            .find(|&(_, name)| &**name == world)
            .map(|(&id, _)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_names_round_trip() {
        for t in Table::ALL {
            assert_eq!(Table::from_name(t.as_str()), Some(t));
            assert_eq!(Table::ALL[t.index()], t, "ALL is in declaration order");
        }
        assert_eq!(Table::from_name("nope"), None);
    }

    /// Every lookup answers `None` for an id the book does not have, on an
    /// empty book and on one that has other ids.
    #[test]
    fn every_lookup_is_none_for_an_unknown_id() {
        let mut book = NameBook::empty();
        assert!(book.is_empty());
        for pass in 0..2 {
            let lookups: [(&str, Option<&str>); 21] = [
                ("item", book.item(999_999)),
                ("ability", book.ability(999_999)),
                ("effect", book.effect(999_999)),
                ("mission", book.mission(999_999)),
                ("mission_step", book.mission_step(999_999)),
                ("mission_objective", book.mission_objective(999_999)),
                ("dialog", book.dialog(999_999)),
                ("dialog_set", book.dialog_set(999_999)),
                ("speaker", book.speaker(999_999)),
                ("template", book.template(999_999)),
                ("template_display", book.template_display(999_999)),
                ("moniker", book.moniker(999_999)),
                ("text", book.text(999_999)),
                ("error_code", book.error_code(999_999)),
                ("stargate", book.stargate(999_999)),
                ("respawner", book.respawner(999_999)),
                ("spawn_set", book.spawn_set(999_999)),
                ("container", book.container(999_999)),
                ("item_list", book.item_list(999_999)),
                ("applied_science", book.applied_science(999_999)),
                ("world", book.world(999_999)),
            ];
            for (name, got) in lookups {
                assert_eq!(got, None, "{name} on pass {pass}");
            }
            for t in Table::ALL {
                book.insert(t, 1, "One");
            }
            book.insert_template_name_id(1, 1);
        }
    }

    /// Each typed lookup reads its own table, and any integer id type works.
    #[test]
    fn typed_lookups_read_their_own_table() {
        let mut book = NameBook::empty();
        for t in Table::ALL {
            book.insert(t, 7, t.as_str());
        }
        assert_eq!(book.item(7u32), Some("items"));
        assert_eq!(book.ability(7i32), Some("abilities"));
        assert_eq!(book.effect(7u16), Some("effects"));
        assert_eq!(book.mission(7i64), Some("missions"));
        assert_eq!(book.mission_step(7), Some("mission_steps"));
        assert_eq!(book.mission_objective(7), Some("mission_objectives"));
        assert_eq!(book.dialog(7), Some("dialogs"));
        assert_eq!(book.dialog_set(7), Some("dialog_sets"));
        assert_eq!(book.speaker(7), Some("speakers"));
        assert_eq!(book.template(7), Some("entity_templates"));
        assert_eq!(book.moniker(7), Some("monikers"));
        assert_eq!(book.text(7), Some("texts"));
        assert_eq!(book.error_code(7u8), Some("error_texts"));
        assert_eq!(book.stargate(7), Some("stargates"));
        assert_eq!(book.respawner(7), Some("respawners"));
        assert_eq!(book.spawn_set(7), Some("spawn_sets"));
        assert_eq!(book.container(7), Some("containers"));
        assert_eq!(book.item_list(7), Some("item_lists"));
        assert_eq!(book.applied_science(7), Some("applied_science"));
        assert_eq!(book.world(7), Some("worlds"));
    }

    /// `world_id` reverses `world`; an unknown name is `None`.
    #[test]
    fn world_id_reverses_world() {
        let mut book = NameBook::empty();
        book.insert(Table::Worlds, 4, "Castle_CellBlock");
        book.insert(Table::Worlds, 9, "Harset");
        assert_eq!(book.world_id("Harset"), Some(9));
        assert_eq!(book.world_id("Castle_CellBlock"), Some(4));
        assert_eq!(book.world_id("Nowhere"), None);
    }

    /// A template's display name goes through its `name_id` into `texts`;
    /// a `name_id` with no text is `None`, not the template name.
    #[test]
    fn template_display_reads_texts_through_name_id() {
        let mut book = NameBook::empty();
        book.insert(Table::Templates, 64, "Thor");
        book.insert(Table::Texts, 7435, "Supreme Commander Thor");
        book.insert_template_name_id(64, 7435);
        book.insert(Table::Templates, 87, "Demon Jaffa");
        book.insert_template_name_id(87, 1);
        assert_eq!(book.template(64), Some("Thor"));
        assert_eq!(book.template_display(64), Some("Supreme Commander Thor"));
        assert_eq!(book.template_display(87), None);
    }
}
