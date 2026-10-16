//! Build disabled native-slash descriptors and conservative method evidence.

use serde::{Deserialize, Serialize};

use super::commands::CommandDescriptor;
use super::slash_descriptors_1a::descriptors_for as descriptors_1a;
use super::slash_descriptors_1b::descriptors_1b;
use super::slash_descriptors_2a::descriptors_for as descriptors_2a;
use super::slash_descriptors_2b::descriptors_for as descriptors_2b;
use super::slash_descriptors_3a::descriptors_3a;
use super::slash_descriptors_3b::descriptors_3b;
use super::slash_inventory::{parse_patch_table, SlashEntry};

const PATCH_TOML: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");
const DISPATCH_TABLE: &str =
    include_str!("../../../../docs/protocol/cell-method-dispatch-table.md");
const STOCK_FIXTURE: &str = include_str!("fixtures/stock_slash_addendum.json");

/// A method row from the documented dispatch table. This is a
/// candidate only: event-to-method binding still needs client-side proof.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MethodCandidate {
    pub index: u16,
    pub name: String,
    pub status: String,
    pub binding_verified: bool,
}

/// One patch command with its disabled form and any documented method row.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct SlashCatalogEntry {
    pub source: SlashEntry,
    pub descriptor: CommandDescriptor,
    pub method_candidate: Option<MethodCandidate>,
}

/// Result of inspecting stock files, distinct from the patch 012 table.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StockInventoryState {
    Complete,
    Unknown,
}

/// Explicit stock audit snapshot; unknown is never interpreted as no extras.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct StockAddendum {
    pub stock_inventory: StockInventoryState,
    pub inspected_sources: Vec<String>,
    pub missing_sources: Vec<String>,
    pub verified_extras: Vec<String>,
    pub exclusions: Vec<String>,
}

/// Load the stock audit fixture, preserving unknown when client XML is absent.
pub fn stock_addendum() -> Result<StockAddendum, String> {
    let audit: StockAddendum = serde_json::from_str(STOCK_FIXTURE).map_err(|e| e.to_string())?;
    if audit.stock_inventory == StockInventoryState::Unknown && audit.missing_sources.is_empty() {
        return Err("unknown stock inventory must name missing files".into());
    }
    if audit.stock_inventory == StockInventoryState::Complete && !audit.missing_sources.is_empty() {
        return Err("complete stock inventory cannot name missing files".into());
    }
    Ok(audit)
}

/// Return all patch 012 words in source order as non-runnable typed entries.
pub fn patch_slash_catalog() -> Result<Vec<SlashCatalogEntry>, String> {
    catalog_from_entries(parse_patch_table(PATCH_TOML)?)
}

pub(super) fn catalog_from_entries(
    entries: Vec<SlashEntry>,
) -> Result<Vec<SlashCatalogEntry>, String> {
    if entries.len() != 162 {
        return Err(format!(
            "patch slash roster has {} entries, expected 162",
            entries.len()
        ));
    }
    let mut descriptors = Vec::with_capacity(entries.len());
    descriptors.extend(
        descriptors_1a(&entries)?
            .into_iter()
            .map(|row| row.descriptor),
    );
    descriptors.extend(descriptors_1b(&entries[27..54])?);
    descriptors.extend(
        descriptors_2a(&entries)?
            .into_iter()
            .map(|row| row.descriptor),
    );
    descriptors.extend(
        descriptors_2b(&entries)?
            .into_iter()
            .map(|row| row.descriptor),
    );
    descriptors.extend(descriptors_3a(&entries[108..135])?);
    descriptors.extend(
        descriptors_3b(&entries[135..162])?
            .into_iter()
            .map(|row| row.descriptor),
    );
    if descriptors.len() != entries.len() {
        return Err("six slash descriptor slices do not cover patch roster".into());
    }
    let mut seen = std::collections::HashSet::with_capacity(entries.len());
    entries
        .into_iter()
        .zip(descriptors)
        .map(|(source, descriptor)| {
            if source.name != descriptor.id || !seen.insert(descriptor.id.clone()) {
                return Err(format!(
                    "duplicate or noncanonical slash descriptor: {}",
                    descriptor.id
                ));
            }
            let method_candidate = method_candidate(&source);
            Ok(SlashCatalogEntry {
                source,
                descriptor,
                method_candidate,
            })
        })
        .collect()
}

fn method_candidate(source: &SlashEntry) -> Option<MethodCandidate> {
    let suffix = source.event.strip_prefix("Event_SlashCmd_")?;
    // Fly, ghost and shout have explicit slash-word rows. GiveCash and SpawnByCmd
    // are semantic candidates: the table names stock /GiveNaqahdah and /Spawn,
    // while patch 012 names /gmgivecash and /gmspawnbycmd with matching events.
    // Keep the event in the key so a changed patch binding does not silently
    // inherit an old candidate. None of these prove the client's event binding.
    let documented_alias = match (source.name.as_str(), source.event.as_str()) {
        ("/gmgivecash", "Event_SlashCmd_GiveNaqahdah") => Some(("gmGiveCash", 134)),
        ("/gmspawnbycmd", "Event_SlashCmd_Spawn") => Some(("gmSpawnByCmd", 185)),
        ("/gmsetfly", "Event_SlashCmd_SetFly") | ("/gmsetghost", "Event_SlashCmd_SetGhost") => {
            Some(("onPhysics", 221))
        }
        ("/gmsendgmshout", "Event_SlashCmd_GMShout") => Some(("sendGMShout", 222)),
        _ => None,
    };
    let expected = documented_alias
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| format!("gm{suffix}"));
    let mut matches = DISPATCH_TABLE.lines().filter_map(|line| {
        let mut cells = line.split('|').map(str::trim);
        cells.next()?;
        let index = cells.next()?.parse::<u16>().ok()?;
        if !(109..=225).contains(&index) {
            return None;
        }
        if documented_alias.is_some_and(|(_, expected_index)| index != expected_index) {
            return None;
        }
        let cell = cells.next()?;
        let name = cell.strip_prefix('`')?.split('(').next()?;
        if name != expected && name != suffix {
            return None;
        }
        let status = line
            .rsplit('|')
            .nth(1)?
            .trim()
            .trim_matches('*')
            .to_string();
        if !matches!(status.as_str(), "DONE" | "ADAPT" | "NEW") {
            return None;
        }
        Some(MethodCandidate {
            index,
            name: name.into(),
            status,
            binding_verified: false,
        })
    });
    let candidate = matches.next()?;
    matches.next().is_none().then_some(candidate)
}
