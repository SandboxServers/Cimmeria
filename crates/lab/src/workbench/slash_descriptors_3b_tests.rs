//! Regression guards for patch-012 slash forms 136–162.

use super::commands::{Availability, CommandChannel, Compensation, ImpactKind, SelectionRule};
use super::slash_descriptors_3b::{descriptors_3b, HandlerStatus, NAMES};
use super::slash_inventory::{parse_patch_table, SlashEntry, SlashParameterType};

const PATCH: &str =
    include_str!("../../../../data/client-patches/012-gm-slash-commands/commands.toml");

fn slice() -> Vec<SlashEntry> {
    parse_patch_table(PATCH)
        .unwrap()
        .into_iter()
        .skip(135)
        .collect()
}

#[test]
fn exact_final_slice_rejects_source_event_and_argument_drift() {
    let entries = slice();
    let rows = descriptors_3b(&entries).unwrap();
    assert_eq!(rows.len(), 27);
    assert_eq!(
        rows.iter()
            .map(|row| row.source.name.as_str())
            .collect::<Vec<_>>(),
        NAMES
    );
    for (index, entry) in entries.iter().enumerate() {
        let mut changed = entries.clone();
        changed[index].event.push('X');
        assert!(
            descriptors_3b(&changed).is_err(),
            "{} event drift",
            entry.name
        );
        if !entry.parameters.is_empty() {
            let mut changed = entries.clone();
            changed[index].parameters[0].required = !changed[index].parameters[0].required;
            assert!(
                descriptors_3b(&changed).is_err(),
                "{} required drift",
                entry.name
            );
            let mut changed = entries.clone();
            changed[index].parameters[0].parameter_type = match entry.parameters[0].parameter_type {
                SlashParameterType::Integer => SlashParameterType::String,
                _ => SlashParameterType::Integer,
            };
            assert!(
                descriptors_3b(&changed).is_err(),
                "{} type drift",
                entry.name
            );
        }
    }
    let mut changed = entries;
    changed.swap(0, 1);
    assert!(descriptors_3b(&changed).is_err());
}

#[test]
fn unproved_handlers_remain_disabled_with_per_command_reason() {
    let rows = descriptors_3b(&slice()).unwrap();
    let reasons: std::collections::HashSet<_> = rows
        .iter()
        .map(|row| {
            let descriptor = &row.descriptor;
            assert_eq!(descriptor.channel, CommandChannel::Slash);
            assert_eq!(descriptor.availability, Availability::Unsupported);
            assert!(descriptor.manual_pending);
            assert_eq!(descriptor.compensation, Some(Compensation::None));
            assert_eq!(descriptor.target.selection_rule, SelectionRule::None);
            assert!(descriptor.verify.is_none());
            assert!(!row.target_scope.is_empty());
            assert!(!row.readback_note.is_empty());
            descriptor.disposition_reason.as_deref().unwrap()
        })
        .collect();
    assert_eq!(reasons.len(), 27);
    assert_eq!(rows[0].handler_status, HandlerStatus::Absent);
    assert_eq!(
        rows[2].handler_status,
        HandlerStatus::ImplementedOrdinaryMethod
    );
    assert_eq!(rows[21].handler_status, HandlerStatus::ReceiptOnly);
    for row in &rows[18..21] {
        assert_eq!(row.handler_status, HandlerStatus::Absent);
    }
}

#[test]
fn destructive_chat_and_signature_mismatches_do_not_gain_actions() {
    let rows = descriptors_3b(&slice()).unwrap();
    assert_eq!(
        rows[8].descriptor.impact.as_ref().unwrap().kind,
        ImpactKind::Irreversible
    );
    assert!(rows[8].descriptor.args[0].sensitive);
    for index in [14, 15] {
        assert_eq!(
            rows[index].descriptor.impact.as_ref().unwrap().kind,
            ImpactKind::Irreversible
        );
        assert!(rows[index]
            .descriptor
            .disposition_reason
            .as_ref()
            .unwrap()
            .contains("Destructive"));
    }
    assert!(rows[10]
        .descriptor
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("targetId"));
    assert!(rows[12]
        .descriptor
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("INT8"));
    assert!(rows[16]
        .descriptor
        .disposition_reason
        .as_ref()
        .unwrap()
        .contains("interaction guard"));
    assert!(rows[19].descriptor.args.is_empty());
    assert!(rows[20].descriptor.args.is_empty());
    assert!(rows[26].descriptor.args.is_empty());
}
