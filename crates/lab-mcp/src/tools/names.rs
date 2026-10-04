//! Rule 6 names for the entity tools (named telemetry NT-41).
//!
//! The cell fills the names it holds live (a character or NPC name, the
//! space's world); these fill the ones that come from the seed, through the
//! NameBook, before a reply is shaped into JSON. A name that does not resolve
//! stays `None`, and the wire types leave a `None` name out of the JSON.

use cimmeria_names::NameBook;
use cimmeria_services::cell::messages::{LabEntityNames, LabEntitySnapshot, LabWitnessReport};

/// Fill a snapshot's NameBook names: `template_name`, `archetype_name`, and
/// `entity_name` when the cell had none (an NPC whose `npc_name` is unset
/// gets the text of its `name_id`, else of its template's).
pub(super) fn name_snapshot(snapshot: &mut LabEntitySnapshot, book: &NameBook) {
    if snapshot.entity_name.is_none() {
        snapshot.entity_name = snapshot
            .name_id
            .and_then(|id| book.text(id))
            .or_else(|| snapshot.template_id.and_then(|t| book.template_display(t)))
            .map(str::to_owned);
    }
    snapshot.template_name = template_name(snapshot.template_id, book);
    snapshot.archetype_name = snapshot
        .archetype_id
        .and_then(cimmeria_names::archetype_name)
        .map(str::to_owned);
}

/// Fill a witness report's NameBook names: the entity's and every listed one's.
pub(super) fn name_witness_report(report: &mut LabWitnessReport, book: &NameBook) {
    name_entity(&mut report.names, book);
    for r in report
        .witnessed_by
        .iter_mut()
        .chain(report.witnesses.iter_mut())
    {
        name_entity(&mut r.names, book);
    }
}

/// Fill one entity's NameBook names: `template_name`, and `entity_name` from
/// the template's display name when the cell had none.
fn name_entity(names: &mut LabEntityNames, book: &NameBook) {
    if names.entity_name.is_none() {
        names.entity_name = names
            .template_id
            .and_then(|t| book.template_display(t))
            .map(str::to_owned);
    }
    names.template_name = template_name(names.template_id, book);
}

fn template_name(template_id: Option<i32>, book: &NameBook) -> Option<String> {
    template_id
        .and_then(|t| book.template(t))
        .map(str::to_owned)
}
