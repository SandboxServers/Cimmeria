//! The batch is invalidated while a cycle is under way. The mock responder, or
//! the injected sleeper, makes the change at an exact point of the cycle, so no
//! test depends on timing. Each run without the change is the positive control.
//!
//! A withdrawal of consent does two things: it cancels the batch, which aborts
//! the request in flight, and it replaces the queue, which the re-checks under
//! the lock notice. `Change::Replaced` replaces the queue without cancelling
//! anything, so each generation re-check is pinned by a case only it can catch.
//! `Change::Moved` names a different endpoint and touches nothing else, which
//! does the same for each endpoint re-check. `held.rs` pins the abort itself.
use super::*;

fn withdraw(state: &mut DesktopState) {
    set_consent(state, false);
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Change {
    /// The positive control.
    None,
    /// Consent is withdrawn.
    OptOut,
    /// The queue is dropped and recreated by a reconfiguration. Consent stays
    /// on, the endpoint is the same again and no batch is cancelled: only the
    /// queue generation differs.
    Replaced,
    /// A reconfiguration names a different valid endpoint. Consent stays on,
    /// the queue and its generation are the same and no batch is cancelled:
    /// only the endpoint differs.
    Moved,
}
impl Change {
    fn apply(self, state: &mut DesktopState, base: &str, moved: &str) {
        match self {
            Self::None => (),
            Self::OptOut => withdraw(state),
            Self::Replaced => {
                state.configure_summaries(config(None));
                state.configure_summaries(config(Some(base)));
            }
            Self::Moved => state.configure_summaries(config(Some(moved))),
        }
    }

    /// Whether the queue the batch was taken from is gone afterwards.
    fn replaces_queue(self) -> bool {
        matches!(self, Self::OptOut | Self::Replaced)
    }
}

const CHANGES: [Change; 4] = [
    Change::None,
    Change::OptOut,
    Change::Replaced,
    Change::Moved,
];

/// The token every batch taken from now on carries.
fn batch_cancel(rig: &Rig) -> tokio_util::sync::CancellationToken {
    lock(&rig.owner().summaries).cancel.clone()
}

#[tokio::test]
async fn a_change_during_the_backoff_sends_nothing_more() {
    for change in CHANGES {
        let rig = Rig::opted_in().await;
        // The server asks for more patience than the cap allows.
        mount(
            &rig.server,
            ResponseTemplate::new(503).insert_header("retry-after", "120"),
        )
        .await;
        let (other, moved) = elsewhere().await;
        let rows = rig.fail(1);
        let generation = stored(&rig.owner()).generation;
        let cancel = batch_cancel(&rig);
        let (meddler, base, target) = (rig.meddler(), rig.base.clone(), moved.clone());
        rig.probe
            .during_sleep(move || meddler.run(|state| change.apply(state, &base, &target)));

        let outcome = rig.cycle().await;
        assert!(rig.lock_was_free());
        rig.assert_anonymous(&format!("{change:?}")).await;
        assert_eq!(other.received_requests().await.unwrap().len(), 0);
        // Only a withdrawal cancels the batch; the other changes are left to
        // the re-check after the wait.
        assert_eq!(cancel.is_cancelled(), change == Change::OptOut);
        if change == Change::None {
            assert_eq!(outcome, CycleOutcome::GaveUp);
            assert_eq!(rig.paths().await, [INGEST; 3]);
            assert_eq!(rig.probe.sleeps(), [CAP, CAP]);
            assert_eq!(queued(&rig.owner()), rows);
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{change:?}");
        assert_eq!(rig.paths().await, [INGEST], "{change:?}: no second POST");
        assert_eq!(rig.probe.sleeps(), [CAP], "min(Retry-After, cap)");
        if change == Change::Moved {
            // The endpoint is the only thing that changed: the row waits in
            // the same queue.
            let target = rig.owner().summary_export_target().expect("an endpoint");
            assert_eq!(Some(target.endpoint), SummaryEndpoint::parse(&moved));
            assert_eq!(queued(&rig.owner()), rows);
            assert_eq!(stored(&rig.owner()).generation, generation);
            // Positive control: the other server is reachable and records. An
            // exporter started for it delivers the row there; this one sent it
            // nothing.
            assert_eq!(rig.cycle_where_configured().await, DELIVERED_ONE);
            let received = other.received_requests().await.unwrap();
            assert_eq!(received.len(), 1);
            assert_eq!(received[0].url.path(), INGEST);
            assert_eq!(rig.paths().await, [INGEST], "nothing more to the old one");
            continue;
        }
        let owner = rig.owner();
        assert_eq!(queued(&owner), [], "{change:?}");
        assert_eq!(
            lock(&owner.summaries).queue.generation,
            generation + 1,
            "{change:?}"
        );
        if change == Change::OptOut {
            assert_eq!(stored(&owner), Queue::empty(generation + 1));
        }
    }
}

// Both ways an answer removes rows are covered: one verdict for each row, and
// a refusal of the whole body.
#[tokio::test]
async fn a_change_inside_the_post_response_applies_nothing_to_the_new_queue() {
    let answers = [
        ("a verdict", results(&["rejected"])),
        ("a refusal", ResponseTemplate::new(400)),
    ];
    for (answered, answer) in answers {
        for change in CHANGES {
            let case = format!("{answered}, {change:?}");
            let rig = Rig::opted_in().await;
            let (other, moved) = elsewhere().await;
            let (meddler, base, answer) = (rig.meddler(), rig.base.clone(), answer.clone());
            mount(&rig.server, move |_: &Request| {
                // The queue is replaced and a new attempt is queued in the new
                // one, or the endpoint is moved, all before the old batch's
                // answer arrives.
                meddler.run(|state| {
                    change.apply(state, &base, &moved);
                    if change == Change::OptOut {
                        set_consent(state, true);
                    }
                    if change.replaces_queue() {
                        fail(state, 1);
                    }
                });
                answer.clone()
            })
            .await;
            let old = rig.fail(1);
            let generation = stored(&rig.owner()).generation;
            let cancel = batch_cancel(&rig);

            let outcome = rig.cycle().await;
            assert!(rig.lock_was_free());
            assert_eq!(rig.paths().await, [INGEST], "{case}: no second POST");
            rig.assert_anonymous(&case).await;
            assert_eq!(other.received_requests().await.unwrap().len(), 0);
            assert_eq!(cancel.is_cancelled(), change == Change::OptOut, "{case}");
            // Taking the state waits for a responder that is still at work.
            let on_disk = stored(&rig.owner());
            if change == Change::None {
                assert_eq!(
                    outcome,
                    CycleOutcome::Delivered {
                        accepted: 0,
                        duplicate: 0,
                        rejected: 1,
                    },
                    "{case}"
                );
                assert_eq!(on_disk.entries, [], "{case}");
                assert_eq!(on_disk.dropped.rejected, 1, "{case}");
                continue;
            }
            assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{case}");
            if change == Change::Moved {
                // The same queue, and the answer was not applied to it: the row
                // is still there and its rejection was not counted.
                assert_eq!(on_disk.generation, generation, "{case}");
                assert_eq!(queued(&rig.owner()), old, "{case}");
                assert_eq!(on_disk.entries.len(), 1, "{case}");
                assert_eq!(on_disk.entries[0].summary, old[0], "{case}");
                assert_eq!(on_disk.dropped, DroppedCounts::default(), "{case}");
                continue;
            }
            // The new queue is untouched: its one new row is there and the old
            // batch's rejection was not counted against it.
            let replacements = if change == Change::OptOut { 2 } else { 1 };
            assert_eq!(on_disk.generation, generation + replacements, "{case}");
            assert_eq!(on_disk.entries.len(), 1, "{case}");
            assert_ne!(on_disk.entries[0].summary.event_id, old[0].event_id);
            assert_eq!(on_disk.dropped, DroppedCounts::default(), "{case}");
        }
    }
}

#[tokio::test]
async fn after_an_opt_out_and_a_new_opt_in_only_the_new_attempt_is_sent() {
    for withdrawn in [false, true] {
        let rig = Rig::opted_in().await;
        mount_ok(&rig.server).await;
        let old = rig.fail(1)[0].event_id;
        if withdrawn {
            set_consent(&mut rig.owner(), false);
            set_consent(&mut rig.owner(), true);
        }
        let rows = rig.fail(1);
        let new = rows.last().unwrap().event_id;

        let sent = if withdrawn { 1 } else { 2 };
        assert_eq!(
            rig.cycle().await,
            CycleOutcome::Delivered {
                accepted: sent,
                duplicate: 0,
                rejected: 0,
            }
        );
        assert_eq!(rig.paths().await, [INGEST], "exactly one POST");
        let posts = rig.posts().await;
        let ids: Vec<Uuid> = posts[0]["summaries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|summary| serde_json::from_value(summary["event_id"].clone()).unwrap())
            .collect();
        if withdrawn {
            assert_eq!(ids, [new]);
        } else {
            assert_eq!(ids, [old, new]);
        }
    }
}

#[tokio::test]
async fn an_opt_out_whose_write_fails_sends_nothing_for_the_rest_of_the_run() {
    use crate::storage::atomic::{self, Checkpoint};
    let cases = [
        ("no opt-out", None),
        ("write fails", Some(Checkpoint::BeforeReplace)),
        ("write uncertain", Some(Checkpoint::AfterReplace)),
    ];
    for (case, fault) in cases {
        let rig = Rig::opted_in().await;
        mount_ok(&rig.server).await;
        rig.fail(1);
        if fault.is_some() {
            let mut owner = rig.owner();
            let preferences = owner.preferences().clone();
            let saved = owner.save_preferences_with(
                preferences.install_directory,
                false,
                preferences.revision,
                |root, next| {
                    atomic::write_with(root, "preferences.json", next, |stage| {
                        if Some(stage) == fault {
                            Err(std::io::Error::other("injected write fault"))
                        } else {
                            Ok(())
                        }
                    })
                },
            );
            assert!(saved.is_err(), "{case}");
            // The launcher still believes the last confirmed preferences.
            assert!(owner.preferences().launcher_summary_consent);
        }
        let outcome = rig.cycle().await;
        if fault.is_some() {
            assert_eq!(outcome, CycleOutcome::SkippedNoConsent, "{case}");
            assert_eq!(rig.cycle().await, CycleOutcome::SkippedNoConsent);
            assert_eq!(rig.paths().await, [""; 0], "{case}");
        } else {
            assert_eq!(outcome, DELIVERED_ONE);
            assert_eq!(rig.paths().await, [INGEST]);
        }
    }
}

// The detector the race tests rely on does fire when the mutex is taken.
#[tokio::test]
async fn a_held_state_mutex_is_noticed_by_the_meddler() {
    let rig = Rig::opted_in().await;
    let meddler = rig.meddler();
    meddler.run(|_| ());
    assert!(rig.lock_was_free());
    let guard = rig.owner();
    meddler.run(|_| panic!("the mutex is held"));
    drop(guard);
    assert!(!rig.lock_was_free());
}
