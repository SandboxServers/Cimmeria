//! Offline regression guards for the structurally disabled slash candidate.

use super::slash_transport::{PlainShoutCandidate, SlashObservation, SlashReceipt, SlashStage};
use crate::supervisor::slash_adapter::stock_chat_chunk;

fn candidate(line: &str) -> PlainShoutCandidate {
    PlainShoutCandidate::parse(line).expect("plain fixture")
}

fn claim(receipt: &SlashReceipt, stage: SlashStage) -> SlashObservation {
    SlashObservation {
        attempt_id: receipt.attempt_id,
        line: receipt.line.clone(),
        fingerprint: "caller-supplied-build".into(),
        generation: 7,
        stage,
    }
}

#[test]
fn documented_plain_shout_shape_is_pure_and_carries_no_identity() {
    let plan = candidate("/gmsendgmshout 1 HPUshout1");
    assert_eq!(plan.line, "/gmsendgmshout 1 HPUshout1");
    let receipt = SlashReceipt::new(plan);
    assert_eq!(receipt.stage(SlashStage::ClientSubmitted), None);
    assert_eq!(receipt.stage(SlashStage::ServerReceived), None);
}

#[test]
fn quotes_backslashes_empty_and_optional_shapes_are_refused() {
    for line in [
        "/gmsendgmshout 1 two words",
        "/gmsendgmshout 1 \"quoted\"",
        "/gmsendgmshout 1 a\\b",
        "/gmsendgmshout 1",
        "/gmsendgmshout 1 ",
        "/gmsendgmshout 1 token ",
        "/gmsendgmshout 0 HPUshout1",
        "/gmspawn 7",
    ] {
        assert!(PlainShoutCandidate::parse(line).is_err(), "{line}");
    }
}

#[test]
fn stock_chat_chunk_reads_back_before_submitting() {
    let chunk = stock_chat_chunk("/gmsendgmshout 1 HPUshout1");
    let readback = chunk.find("input:getText()").unwrap();
    let submit = chunk.find("ChatMod.onTextAccepted(input, input)").unwrap();
    assert!(readback < submit);
    assert!(chunk.contains("return 'client_submitted'"));
    assert!(!chunk.contains("accepted=true"));
}

#[test]
fn forged_fingerprint_and_generation_cannot_supply_supervisor_identity() {
    let mut receipt = SlashReceipt::new(candidate("/gmsendgmshout 1 HPUshout1"));
    let mut forged = claim(&receipt, SlashStage::ClientSubmitted);
    for fingerprint in ["caller-supplied-build", "pretend-expected-build"] {
        for generation in [7, 8] {
            forged.fingerprint = fingerprint.into();
            forged.generation = generation;
            assert!(receipt.observe(&forged).is_err());
            assert_eq!(receipt.stage(SlashStage::ClientSubmitted), None);
        }
    }
}

#[test]
fn forged_ack_and_server_receipt_stay_unknown() {
    let mut receipt = SlashReceipt::new(candidate("/gmsendgmshout 1 HPUshout1"));
    for stage in [
        SlashStage::ClientSubmitted,
        SlashStage::ClientAcknowledged,
        SlashStage::OutgoingMethod,
        SlashStage::ServerReceived,
    ] {
        let row = claim(&receipt, stage);
        assert!(receipt.observe(&row).is_err());
        assert_eq!(receipt.stage(stage), None);
    }
    assert!(!receipt.accepted());
}

#[test]
fn same_line_concurrent_attempts_cannot_cross_join() {
    let line = "/gmsendgmshout 1 HPUshout1";
    let mut first = SlashReceipt::new(candidate(line));
    let mut second = SlashReceipt::new(candidate(line));
    assert_ne!(first.attempt_id, second.attempt_id);
    let second_row = claim(&second, SlashStage::ServerReceived);
    assert!(first.observe(&second_row).is_err());
    assert!(second.observe(&second_row).is_err());
    assert_eq!(first.stage(SlashStage::ServerReceived), None);
    assert_eq!(second.stage(SlashStage::ServerReceived), None);
}
