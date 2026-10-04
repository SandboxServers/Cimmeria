//! Reading a [`NameBook`] from the `resources` schema.

use sqlx::PgPool;

use crate::book::{NameBook, Table};
use crate::placeholder::{classify, Unresolved};

/// The query for each table: `(id bigint, name text)`, one row per seed row.
/// The name may be NULL; [`classify`] decides whether it is a name.
pub fn query(table: Table) -> &'static str {
    match table {
        Table::Items => "SELECT item_id::bigint, name::text FROM resources.items",
        Table::Abilities => "SELECT ability_id::bigint, name::text FROM resources.abilities",
        Table::Effects => "SELECT effect_id::bigint, name::text FROM resources.effects",
        // `mission_defn` is the mission's name; `mission_label` is its zone or
        // group (`Harset`, `General`), and 202 rows have `NO MISSION LABEL`.
        Table::Missions => "SELECT mission_id::bigint, mission_defn::text FROM resources.missions",
        Table::MissionSteps => {
            "SELECT step_id::bigint, step_display_log_text::text FROM resources.mission_steps"
        }
        Table::MissionObjectives => {
            "SELECT objective_id::bigint, display_log_text::text \
             FROM resources.mission_objectives"
        }
        // The seed names only the six sandbox dialogs, so a dialog without a
        // name takes the topic text of the first dialog-set entry that opens
        // it: the line the player clicked to get there.
        Table::Dialogs => {
            "SELECT d.dialog_id::bigint, \
                    COALESCE(NULLIF(btrim(d.name), ''), \
                             (SELECT m.topic_text FROM resources.dialog_set_maps m \
                              WHERE m.dialog_id = d.dialog_id \
                                AND btrim(COALESCE(m.topic_text, '')) <> '' \
                              ORDER BY m.dialog_set_map_id LIMIT 1))::text \
             FROM resources.dialogs d"
        }
        Table::DialogSets => "SELECT dialog_set_id::bigint, name::text FROM resources.dialog_sets",
        Table::Speakers => "SELECT speaker_id::bigint, name::text FROM resources.speakers",
        Table::Templates => {
            "SELECT template_id::bigint, template_name::text FROM resources.entity_templates"
        }
        Table::Monikers => "SELECT moniker_id::bigint, name::text FROM resources.monikers",
        Table::Texts => "SELECT moniker_id::bigint, text::text FROM resources.texts",
        Table::ErrorTexts => {
            "SELECT error_id::bigint, moniker_name::text FROM resources.error_texts"
        }
        Table::Stargates => "SELECT stargate_id::bigint, name::text FROM resources.stargates",
        Table::Respawners => "SELECT respawner_id::bigint, name::text FROM resources.respawners",
        Table::SpawnSets => "SELECT set_id::bigint, name::text FROM resources.spawn_sets",
        Table::Containers => "SELECT container_id::bigint, name::text FROM resources.containers",
        Table::ItemLists => "SELECT item_list_id::bigint, name::text FROM resources.item_lists",
        Table::AppliedScience => "SELECT id::bigint, name::text FROM resources.applied_science",
        Table::Worlds => "SELECT world_id::bigint, world::text FROM resources.worlds",
        Table::Chains => "SELECT chain_id::bigint, description::text FROM resources.content_chains",
        Table::LootTables => {
            "SELECT loot_table_id::bigint, description::text FROM resources.loot_tables"
        }
        Table::TrainerAbilityLists => {
            "SELECT list_id::bigint, description::text FROM resources.trainer_ability_lists"
        }
        Table::EventSets => "SELECT event_set_id::bigint, name::text FROM resources.event_sets",
        Table::Sequences => {
            "SELECT sequence_id::bigint, kismet_script_name::text FROM resources.sequences"
        }
        Table::DialogSetMaps => {
            "SELECT dialog_set_map_id::bigint, topic_text::text FROM resources.dialog_set_maps"
        }
    }
}

/// What one table's load found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableCount {
    /// Seed rows read.
    pub rows: usize,
    /// Rows whose name is NULL, empty or whitespace.
    pub blank: usize,
    /// Rows whose name is a placeholder (`NO ITEM NAME`, ...).
    pub placeholder: usize,
}

impl TableCount {
    /// Rows that resolve to a name.
    pub fn named(&self) -> usize {
        self.rows - self.blank - self.placeholder
    }
}

/// Per-table counts from one load, for the `names.loaded` event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadReport {
    counts: [TableCount; Table::ALL.len()],
}

impl LoadReport {
    /// The counts for `table`.
    pub fn count(&self, table: Table) -> TableCount {
        self.counts[table as usize]
    }

    /// Record the counts for `table`.
    pub fn set(&mut self, table: Table, count: TableCount) {
        self.counts[table as usize] = count;
    }

    /// Tables where no row resolved to a name: an empty table, or one
    /// whose every name is blank or a placeholder.
    pub fn empty_tables(&self) -> Vec<Table> {
        Table::ALL
            .into_iter()
            .filter(|t| self.count(*t).named() == 0)
            .collect()
    }

    /// Rows with no name, across every table.
    pub fn unresolved(&self) -> usize {
        self.counts.iter().map(|c| c.blank + c.placeholder).sum()
    }
}

impl NameBook {
    /// Read every table. Fails on the first query error, so a reload that
    /// fails keeps the book it had.
    pub async fn load(pool: &PgPool) -> Result<(NameBook, LoadReport), sqlx::Error> {
        let mut book = NameBook::empty();
        let mut report = LoadReport::default();
        for table in Table::ALL {
            let rows: Vec<(i64, Option<String>)> =
                sqlx::query_as(query(table)).fetch_all(pool).await?;
            let mut count = TableCount {
                rows: rows.len(),
                ..TableCount::default()
            };
            for (id, name) in &rows {
                match classify(name.as_deref()) {
                    Ok(n) => book.insert(table, *id, n),
                    Err(Unresolved::Blank) => count.blank += 1,
                    Err(Unresolved::Placeholder) => count.placeholder += 1,
                }
            }
            report.set(table, count);
        }
        let name_ids: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT template_id::bigint, name_id::bigint FROM resources.entity_templates \
             WHERE name_id IS NOT NULL",
        )
        .fetch_all(pool)
        .await?;
        for (template_id, name_id) in name_ids {
            book.insert_template_name_id(template_id, name_id);
        }
        Ok((book, report))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_excludes_blank_and_placeholder_rows() {
        let c = TableCount {
            rows: 10,
            blank: 3,
            placeholder: 2,
        };
        assert_eq!(c.named(), 5);
    }

    #[test]
    fn empty_tables_lists_tables_with_no_names() {
        let mut report = LoadReport::default();
        for t in Table::ALL {
            report.set(
                t,
                TableCount {
                    rows: 2,
                    blank: 1,
                    placeholder: 0,
                },
            );
        }
        report.set(
            Table::Speakers,
            TableCount {
                rows: 3,
                blank: 2,
                placeholder: 1,
            },
        );
        report.set(Table::Worlds, TableCount::default());
        assert_eq!(report.empty_tables(), [Table::Speakers, Table::Worlds]);
        assert_eq!(report.unresolved(), 24 + 3);
    }
}
