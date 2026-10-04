//! The process's current [`NameBook`], and how it is loaded and swapped.
//!
//! The base and the cell run in one process and share one book: the
//! [`global`] handle. Either half's startup calls [`load_at_boot`]; only the
//! first reads the database. Content reload calls [`reload`], which builds a
//! new book and swaps it in whole. A reader holding the old book keeps it
//! until it drops its guard, so no reader ever sees a half-loaded book.

use std::sync::{Arc, LazyLock};
use std::time::Instant;

use arc_swap::{ArcSwap, Guard};
use sqlx::PgPool;
use tokio::sync::OnceCell;

use crate::book::{NameBook, Table};
use crate::load::LoadReport;

/// A swappable [`NameBook`]. The process uses the [`global`] one; tests
/// make their own.
#[derive(Debug)]
pub struct NameBookHandle {
    current: ArcSwap<NameBook>,
}

impl Default for NameBookHandle {
    fn default() -> Self {
        Self::new(NameBook::empty())
    }
}

impl NameBookHandle {
    /// A handle holding `book`.
    pub fn new(book: NameBook) -> Self {
        Self {
            current: ArcSwap::from_pointee(book),
        }
    }

    /// The current book. Cheap (no lock, no clone); hold the guard for one
    /// log line or one handler, not across an `.await` that could outlive a
    /// reload you want to see.
    pub fn book(&self) -> Guard<Arc<NameBook>> {
        self.current.load()
    }

    /// Replace the book. Readers that already hold the old one keep it.
    pub fn store(&self, book: NameBook) {
        self.current.store(Arc::new(book));
    }

    /// Read every table and swap the result in. On a database error the
    /// current book stays, and the error is logged and returned.
    pub async fn reload(&self, pool: &PgPool, trigger: Trigger) -> Result<LoadReport, sqlx::Error> {
        let started = Instant::now();
        match NameBook::load(pool).await {
            Ok((book, report)) => {
                self.store(book);
                log_loaded(&report, trigger, started.elapsed().as_millis());
                Ok(report)
            }
            Err(e) => {
                tracing::warn!(
                    target: "names",
                    event = "names.load_failed",
                    reason = "db_error",
                    trigger = trigger.as_str(),
                    error = %e,
                    "name book load failed; log lines keep the names they had"
                );
                Err(e)
            }
        }
    }
}

/// What asked for a load, for the `names.loaded` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Base or cell startup.
    Boot,
    /// The content reload (`ReloadContentEngine`: the admin API's
    /// `POST /api/content/reload` and the lab's `server_content_reload`).
    ContentReload,
}

impl Trigger {
    fn as_str(self) -> &'static str {
        match self {
            Trigger::Boot => "boot",
            Trigger::ContentReload => "content_reload",
        }
    }
}

/// `names.loaded` (INFO) with the named-row count of every table, and
/// `names.tables_empty` (WARN) when any table resolved no name at all.
fn log_loaded(report: &LoadReport, trigger: Trigger, elapsed_ms: u128) {
    let n = |t: Table| report.count(t).named();
    tracing::info!(
        target: "names",
        event = "names.loaded",
        trigger = trigger.as_str(),
        elapsed_ms = elapsed_ms as u64,
        items = n(Table::Items),
        abilities = n(Table::Abilities),
        effects = n(Table::Effects),
        missions = n(Table::Missions),
        mission_steps = n(Table::MissionSteps),
        mission_objectives = n(Table::MissionObjectives),
        dialogs = n(Table::Dialogs),
        dialog_sets = n(Table::DialogSets),
        speakers = n(Table::Speakers),
        entity_templates = n(Table::Templates),
        monikers = n(Table::Monikers),
        texts = n(Table::Texts),
        error_texts = n(Table::ErrorTexts),
        stargates = n(Table::Stargates),
        respawners = n(Table::Respawners),
        spawn_sets = n(Table::SpawnSets),
        containers = n(Table::Containers),
        item_lists = n(Table::ItemLists),
        applied_science = n(Table::AppliedScience),
        worlds = n(Table::Worlds),
        content_chains = n(Table::Chains),
        loot_tables = n(Table::LootTables),
        trainer_ability_lists = n(Table::TrainerAbilityLists),
        event_sets = n(Table::EventSets),
        sequences = n(Table::Sequences),
        dialog_set_maps = n(Table::DialogSetMaps),
        unresolved = report.unresolved(),
        "name book loaded"
    );
    let empty = report.empty_tables();
    if !empty.is_empty() {
        let tables = empty
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(",");
        tracing::warn!(
            target: "names",
            event = "names.tables_empty",
            reason = "no_named_rows",
            trigger = trigger.as_str(),
            tables = %tables,
            "name book tables with no names: ids from them log without a name"
        );
    }
}

static GLOBAL: LazyLock<NameBookHandle> = LazyLock::new(NameBookHandle::default);
static BOOT_LOAD: OnceCell<()> = OnceCell::const_new();

/// The process's handle, shared by the base and the cell. Empty until
/// [`load_at_boot`] lands.
pub fn global() -> &'static NameBookHandle {
    &GLOBAL
}

/// The process's current book: `cimmeria_names::book().item(id)`.
pub fn book() -> Guard<Arc<NameBook>> {
    GLOBAL.book()
}

/// Load the [`global`] book once per process. The base and the cell both
/// call it at startup; the second call returns at once. A failed load is
/// logged and leaves the book empty, and the next call tries again.
pub async fn load_at_boot(pool: &PgPool) {
    let _ = BOOT_LOAD
        .get_or_try_init(|| async { GLOBAL.reload(pool, Trigger::Boot).await.map(|_| ()) })
        .await;
}

/// Reload the [`global`] book from the database (content reload).
pub async fn reload(pool: &PgPool) {
    let _ = GLOBAL.reload(pool, Trigger::ContentReload).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_with(item: &str) -> NameBook {
        let mut b = NameBook::empty();
        b.insert(Table::Items, 5, item);
        b.insert(Table::Worlds, 1, "CombatSim");
        b
    }

    /// A store replaces the whole book at once: a reader that took the old
    /// book keeps every old name, and the next reader sees only new ones,
    /// including an id the new book dropped.
    #[test]
    fn reload_swaps_the_whole_book_atomically() {
        let handle = NameBookHandle::new(book_with("Old Rifle"));
        let before = handle.book();

        let mut next = book_with("New Rifle");
        next.insert(Table::Abilities, 880, "Staff Blast");
        handle.store(next);

        assert_eq!(before.item(5), Some("Old Rifle"));
        assert_eq!(before.ability(880), None);
        assert_eq!(before.world(1), Some("CombatSim"));

        let after = handle.book();
        assert_eq!(after.item(5), Some("New Rifle"));
        assert_eq!(after.ability(880), Some("Staff Blast"));

        let mut shrunk = NameBook::empty();
        shrunk.insert(Table::Items, 5, "New Rifle");
        handle.store(shrunk);
        assert_eq!(handle.book().world(1), None, "a dropped id is gone");
        assert_eq!(
            after.world(1),
            Some("CombatSim"),
            "the held book is unchanged"
        );
    }

    /// A reload whose database read fails keeps the current book whole and
    /// says so: `names.load_failed` with `reason = db_error`, and the error
    /// comes back to the caller. A closed pool fails every query at once, so
    /// no database is needed.
    #[tokio::test]
    async fn failed_reload_keeps_the_current_book_and_warns() {
        let capture = crate::test_support::LogCapture::install();
        let handle = NameBookHandle::new(book_with("Old Rifle"));
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://nobody@127.0.0.1:1/none")
            .expect("lazy pool");
        pool.close().await;

        let result = handle.reload(&pool, Trigger::ContentReload).await;

        assert!(result.is_err(), "the error reaches the caller");
        assert_eq!(handle.book().item(5), Some("Old Rifle"));
        assert_eq!(handle.book().world(1), Some("CombatSim"));
        let warn = capture
            .find_event(tracing::Level::WARN, "name book load failed", "db_error")
            .expect("names.load_failed");
        assert_eq!(warn.target, "names");
        assert!(warn.has_field("event", "names.load_failed"));
        assert!(warn.has_field("trigger", "content_reload"));
        assert!(warn.fields.contains_key("error"));
        assert!(capture
            .find_message(tracing::Level::INFO, "name book loaded")
            .is_none());
    }

    #[test]
    fn a_new_handle_is_empty() {
        let handle = NameBookHandle::default();
        assert!(handle.book().is_empty());
        assert_eq!(handle.book().item(5), None);
    }

    /// `names.loaded` carries every table's count and the trigger, and a
    /// table with no names raises `names.tables_empty` naming it.
    #[test]
    fn loaded_event_carries_counts_and_warns_on_empty_tables() {
        use crate::load::TableCount;
        let capture = crate::test_support::LogCapture::install();
        let mut report = LoadReport::default();
        for t in Table::ALL {
            report.set(
                t,
                TableCount {
                    rows: 4,
                    blank: 1,
                    placeholder: 0,
                },
            );
        }
        report.set(
            Table::Items,
            TableCount {
                rows: 10,
                blank: 0,
                placeholder: 3,
            },
        );
        report.set(Table::Speakers, TableCount::default());
        log_loaded(&report, Trigger::ContentReload, 12);

        let loaded = capture
            .find_message(tracing::Level::INFO, "name book loaded")
            .expect("names.loaded");
        assert_eq!(loaded.target, "names");
        assert!(loaded.has_field("event", "names.loaded"));
        assert!(loaded.has_field("trigger", "content_reload"));
        assert!(loaded.has_field("items", "7"));
        assert!(loaded.has_field("worlds", "3"));
        assert!(loaded.has_field("speakers", "0"));

        let empty = capture
            .find_event(tracing::Level::WARN, "no names", "no_named_rows")
            .expect("names.tables_empty");
        assert!(empty.has_field("tables", "speakers"));
    }

    /// A full book raises no empty-table warning.
    #[test]
    fn no_empty_warning_when_every_table_has_names() {
        use crate::load::TableCount;
        let capture = crate::test_support::LogCapture::install();
        let mut report = LoadReport::default();
        for t in Table::ALL {
            report.set(
                t,
                TableCount {
                    rows: 1,
                    blank: 0,
                    placeholder: 0,
                },
            );
        }
        log_loaded(&report, Trigger::Boot, 1);
        assert!(capture
            .find_message(tracing::Level::WARN, "no names")
            .is_none());
    }
}
