//! Build disabled native-slash descriptors and conservative method evidence.

use serde::{Deserialize, Serialize};

use super::commands::{
    ArgDescriptor, ArgKind, Availability, CommandChannel, CommandDescriptor, SelectionRule,
    TargetKind, TargetPolicy, COMMAND_METADATA_VERSION,
};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

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
    let entries = parse_patch_table(PATCH_TOML)?;
    Ok(entries
        .into_iter()
        .map(|source| {
            let method_candidate = method_candidate(&source);
            let descriptor = disabled_descriptor(&source);
            SlashCatalogEntry {
                source,
                descriptor,
                method_candidate,
            }
        })
        .collect())
}

fn disabled_descriptor(source: &SlashEntry) -> CommandDescriptor {
    let args = source
        .parameters
        .iter()
        .map(|parameter| ArgDescriptor {
            name: parameter.name.clone(),
            kind: match parameter.parameter_type {
                SlashParameterType::Boolean => ArgKind::Boolean,
                SlashParameterType::Integer => ArgKind::Integer,
                SlashParameterType::Float => ArgKind::Float,
                SlashParameterType::String => ArgKind::Text,
            },
            required: parameter.required,
            default: None,
            range: None,
            choices: Vec::new(),
            help: "Patch 012 client parser parameter; effect and formatting pending review".into(),
            sensitive: false,
        })
        .collect();
    CommandDescriptor {
        id: source.name.clone(),
        channel: CommandChannel::Slash,
        category: "native slash inventory".into(),
        aliases: Vec::new(),
        summary: format!("{} client event (effect pending review)", source.event),
        example: None,
        availability: Availability::Unsupported,
        disposition_reason: Some("native submission unproved".into()),
        manual_pending: true,
        args,
        target: TargetPolicy {
            kind: TargetKind::None,
            selection_rule: SelectionRule::None,
            argument_name: None,
        },
        impact: None,
        verify: None,
        compensation: None,
        metadata_version: COMMAND_METADATA_VERSION,
    }
}

fn method_candidate(source: &SlashEntry) -> Option<MethodCandidate> {
    let suffix = source.event.strip_prefix("Event_SlashCmd_")?;
    // Only these non-name matches are named explicitly in the dispatch table's
    // slash-word column. Keep the event in the key so a changed patch binding
    // does not silently inherit an old candidate.
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
