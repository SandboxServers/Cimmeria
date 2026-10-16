//! Offline regression guards for the disabled slash candidate.

use super::slash_transport::{PlainShoutCandidate, SlashObservation, SlashReceipt, SlashStage};
use crate::supervisor::slash_adapter::stock_chat_chunk;

fn candidate(line: &str) -> PlainShoutCandidate {
    PlainShoutCandidate::parse(line, "unverified-build").expect("plain fixture")
}

fn observation(receipt: &SlashReceipt, stage: SlashStage) -> SlashObservation {
    SlashObservation {
        attempt_id: receipt.attempt_id,
        line: receipt.line.clone(),
        fingerprint: receipt.fingerprint.clone(),
        generation: receipt.generation,
        stage,
        independent_source: true,
    }
}

fn record(receipt: &mut SlashReceipt, stage: SlashStage) -> Result<(), String> {
    let row = observation(receipt, stage);
    receipt.observe(&row)
}

#[test]
fn documented_plain_shout_shape_still_refuses_unverified_binary() {
    let plan = candidate("/gmsendgmshout 1 HPUshout1");
    assert!(plan.require_verified_client().is_err());
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
        assert!(
            PlainShoutCandidate::parse(line, "unverified-build").is_err(),
            "{line}"
        );
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
fn submitted_or_forged_ack_never_means_accepted() {
    let mut receipt = SlashReceipt::new(candidate("/gmsendgmshout 1 HPUshout1"), 7);
    let forged = SlashObservation {
        independent_source: false,
        stage: SlashStage::ClientAcknowledged,
        ..observation(&receipt, SlashStage::ClientSubmitted)
    };
    assert!(receipt.observe(&forged).is_err());
    record(&mut receipt, SlashStage::ClientSubmitted).unwrap();
    assert!(!receipt.accepted());
    record(&mut receipt, SlashStage::ClientAcknowledged).unwrap();
    assert!(!receipt.accepted());
}

#[test]
fn concurrent_attempts_never_cross_join() {
    let mut first = SlashReceipt::new(candidate("/gmsendgmshout 1 First"), 7);
    let mut second = SlashReceipt::new(candidate("/gmsendgmshout 1 Second"), 7);
    assert_ne!(first.attempt_id, second.attempt_id);
    record(&mut first, SlashStage::ClientSubmitted).unwrap();
    record(&mut second, SlashStage::ClientSubmitted).unwrap();
    assert!(first
        .observe(&observation(&second, SlashStage::ClientAcknowledged))
        .is_err());
    assert!(first
        .observe(&observation(&second, SlashStage::OutgoingMethod))
        .is_err());
    assert!(!first.accepted());
    assert!(!second.accepted());
}

#[test]
fn exact_order_and_generation_are_required_for_receipt() {
    let mut receipt = SlashReceipt::new(candidate("/gmsendgmshout 1 HPUshout1"), 7);
    assert!(record(&mut receipt, SlashStage::ServerReceived).is_err());
    let wrong_generation = SlashObservation {
        generation: 8,
        ..observation(&receipt, SlashStage::ClientSubmitted)
    };
    assert!(receipt.observe(&wrong_generation).is_err());
    for stage in [
        SlashStage::ClientSubmitted,
        SlashStage::ClientAcknowledged,
        SlashStage::OutgoingMethod,
        SlashStage::ServerReceived,
    ] {
        record(&mut receipt, stage).unwrap();
    }
    assert!(receipt.server_received);
    assert!(
        !receipt.accepted(),
        "synthetic stages cannot prove a cross-source join"
    );
}
