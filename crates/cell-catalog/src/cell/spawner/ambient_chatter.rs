//! Ambient chatter catalog: NPCs who talk among themselves on a schedule.
//!
//! Loaded once at startup from `resources.ambient_chatter_groups` and
//! `resources.ambient_chatter_lines` (Debug Area DA-09, the System Lords'
//! summit). A group is a set of NPCs in one world, named by their spawn tags;
//! its lines are grouped into exchanges (short scenes) that the
//! `cimmeria-cell-chatter` plugin speaks in turn as say chat to the players
//! standing near the speaker. The 2009 data has nothing like it, so the
//! groups and their text are Cimmeria seed data.
//!
//! Like every other startup cache this is a snapshot: a seed edit needs a
//! server restart.

use std::collections::BTreeMap;
use std::time::Duration;

use sqlx::PgPool;

/// One `resources.ambient_chatter_groups` row.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ChatterGroupRow {
    pub group_id: i32,
    pub world_id: i32,
    pub name: String,
    pub hear_radius: f32,
    pub exchange_gap_secs: i32,
}

/// One `resources.ambient_chatter_lines` row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ChatterLineRow {
    pub group_id: i32,
    pub exchange_id: i32,
    pub line_index: i32,
    pub speaker_tag: String,
    pub delay_ms: i32,
    pub text: String,
}

/// One spoken line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatterLine {
    /// The `spawnlist.tag` of the NPC who speaks it, in the group's world.
    pub speaker_tag: String,
    /// The pause before the line: after the previous line, or after the
    /// exchange starts for the first.
    pub delay: Duration,
    pub text: String,
}

/// One exchange: its lines in `line_index` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatterExchange {
    pub exchange_id: i32,
    pub lines: Vec<ChatterLine>,
}

/// One group, with its exchanges in `exchange_id` order. Never empty: a
/// group row with no lines is dropped at load.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatterGroup {
    pub group_id: i32,
    /// `resources.worlds.world_id` of the world its speakers stand in.
    pub world_id: i32,
    /// For log lines.
    pub name: String,
    /// How far from a speaking NPC (metres) a player hears its line.
    pub hear_radius: f32,
    /// Quiet time between one exchange's last line and the next one's first.
    pub exchange_gap: Duration,
    pub exchanges: Vec<ChatterExchange>,
}

impl ChatterGroup {
    /// Every distinct speaker tag in the group, sorted.
    pub fn speaker_tags(&self) -> Vec<&str> {
        let mut tags: Vec<&str> = self
            .exchanges
            .iter()
            .flat_map(|e| e.lines.iter().map(|l| l.speaker_tag.as_str()))
            .collect();
        tags.sort_unstable();
        tags.dedup();
        tags
    }
}

/// Every chatter group, in `group_id` order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AmbientChatterCatalog {
    pub groups: Vec<ChatterGroup>,
}

impl AmbientChatterCatalog {
    /// Build the catalog from rows. Lines are ordered by exchange and index
    /// whatever order the rows come in. A group with no lines is dropped with
    /// a WARN (it could never speak), and so is a line whose group has no row
    /// (the foreign key rules that out for DB rows).
    pub fn from_rows(groups: Vec<ChatterGroupRow>, lines: Vec<ChatterLineRow>) -> Self {
        let mut by_group: BTreeMap<i32, BTreeMap<i32, BTreeMap<i32, ChatterLineRow>>> =
            BTreeMap::new();
        for line in lines {
            by_group
                .entry(line.group_id)
                .or_default()
                .entry(line.exchange_id)
                .or_default()
                .insert(line.line_index, line);
        }
        let mut out: Vec<ChatterGroup> = Vec::new();
        for g in groups {
            let Some(exchanges) = by_group.remove(&g.group_id) else {
                tracing::warn!(
                    group_id = g.group_id,
                    group_name = %g.name,
                    world_id = g.world_id, // nt:id-only the loader has no world table; group_name names the group
                    reason = "group_without_lines",
                    "ambient chatter group has no lines -- it will never speak"
                );
                continue;
            };
            out.push(ChatterGroup {
                group_id: g.group_id,
                world_id: g.world_id,
                name: g.name,
                hear_radius: g.hear_radius,
                exchange_gap: Duration::from_secs(g.exchange_gap_secs.max(0) as u64),
                exchanges: exchanges
                    .into_iter()
                    .map(|(exchange_id, lines)| ChatterExchange {
                        exchange_id,
                        lines: lines
                            .into_values()
                            .map(|l| ChatterLine {
                                speaker_tag: l.speaker_tag,
                                delay: Duration::from_millis(l.delay_ms.max(0) as u64),
                                text: l.text,
                            })
                            .collect(),
                    })
                    .collect(),
            });
        }
        for (group_id, exchanges) in by_group {
            tracing::warn!(
                group_id, // nt:id-only no group row exists, so there is no name to give
                lines = exchanges.values().map(BTreeMap::len).sum::<usize>(),
                reason = "lines_without_group",
                "ambient chatter lines name a group with no row -- dropped"
            );
        }
        out.sort_by_key(|g| g.group_id);
        Self { groups: out }
    }

    /// Number of groups.
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    /// True when no group will speak (the tables are empty or failed to
    /// load).
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// Load `resources.ambient_chatter_groups` and `ambient_chatter_lines` into
/// an [`AmbientChatterCatalog`].
pub async fn load_ambient_chatter(pool: &PgPool) -> Result<AmbientChatterCatalog, sqlx::Error> {
    let groups = sqlx::query_as::<_, ChatterGroupRow>(
        "SELECT group_id, world_id, name, hear_radius, exchange_gap_secs \
         FROM resources.ambient_chatter_groups",
    )
    .fetch_all(pool)
    .await?;
    let lines = sqlx::query_as::<_, ChatterLineRow>(
        "SELECT group_id, exchange_id, line_index, speaker_tag, delay_ms, text \
         FROM resources.ambient_chatter_lines",
    )
    .fetch_all(pool)
    .await?;
    let line_count = lines.len();
    let catalog = AmbientChatterCatalog::from_rows(groups, lines);
    tracing::info!(
        groups = catalog.len(),
        lines = line_count,
        "Loaded ambient chatter"
    );
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(group_id: i32) -> ChatterGroupRow {
        ChatterGroupRow {
            group_id,
            world_id: 1300,
            name: format!("group {group_id}"),
            hear_radius: 20.0,
            exchange_gap_secs: 30,
        }
    }

    fn line(group_id: i32, exchange_id: i32, line_index: i32, tag: &str) -> ChatterLineRow {
        ChatterLineRow {
            group_id,
            exchange_id,
            line_index,
            speaker_tag: tag.to_string(),
            delay_ms: 1000 * line_index,
            text: format!("{exchange_id}.{line_index}"),
        }
    }

    /// Rows in any order come out grouped and in exchange and line order, so
    /// a seed file written out of order still plays its scenes as authored.
    #[test]
    fn rows_are_grouped_and_ordered() {
        let catalog = AmbientChatterCatalog::from_rows(
            vec![group(2), group(1)],
            vec![
                line(1, 2, 1, "B"),
                line(1, 1, 1, "B"),
                line(2, 1, 0, "C"),
                line(1, 2, 0, "A"),
                line(1, 1, 0, "A"),
            ],
        );
        assert_eq!(catalog.len(), 2);
        let g = &catalog.groups[0];
        assert_eq!(g.group_id, 1);
        assert_eq!(g.exchange_gap, Duration::from_secs(30));
        let texts: Vec<Vec<&str>> = g
            .exchanges
            .iter()
            .map(|e| e.lines.iter().map(|l| l.text.as_str()).collect())
            .collect();
        assert_eq!(texts, vec![vec!["1.0", "1.1"], vec!["2.0", "2.1"]]);
        assert_eq!(g.exchanges[0].lines[1].delay, Duration::from_millis(1000));
        assert_eq!(g.speaker_tags(), vec!["A", "B"]);
        assert_eq!(catalog.groups[1].speaker_tags(), vec!["C"]);
    }

    /// A group with no lines never speaks, so it is not in the catalog, and
    /// orphan lines are dropped rather than invented a group.
    #[test]
    fn a_group_without_lines_and_orphan_lines_are_dropped() {
        let catalog = AmbientChatterCatalog::from_rows(
            vec![group(1), group(3)],
            vec![line(1, 1, 0, "A"), line(9, 1, 0, "Z")],
        );
        assert_eq!(
            catalog
                .groups
                .iter()
                .map(|g| g.group_id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert!(AmbientChatterCatalog::from_rows(vec![], vec![]).is_empty());
    }
}
