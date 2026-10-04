//! The batch is invalidated while a cycle is under way. The mock responder, or
//! the injected sleeper, makes the change at an exact point of the cycle, so no
//! test depends on timing. Each run without the change is the positive control.
//!
//! A withdrawal of consent does two things: it cancels the batch's token, which
//! aborts the request in flight, and it replaces the queue, which the re-checks
//! under the lock notice. `Change::Replaced` replaces the queue without
//! cancelling anything, so each re-check is pinned by a case only it can catch.
//! `held.rs` pins the abort itself.
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
    /// on, the endpoint is the same again and no token is cancelled: only the
    /// queue generation differs.
    Replaced,
}
impl Change {
    fn apply(self, state: &mut DesktopState, base: &str) {
        match self {
            Self::None => (),
            Self::OptOut => withdraw(state),
            Self::Replaced => {
                state.configure_summaries(config(None));
                state.configure_summaries(config(Some(base)));
            }
        }
    }
}

const CHANGES: [Change; 3] = [Change::None, Change::OptOut, Change::Replaced];

#[tokio::test]
async fn a_change_inside_the_mint_response_sends_no_summary() {
    for change in CHANGES {
        let rig = Rig::opted_in().await;
        let (meddler, base) = (rig.meddler(), rig.base.clone());
        mount(&rig.server, MINT, move |_: &Request| {
            meddler.run(|state| change.apply(state, &base));
            minted()
        })
        .await;
        mount(&rig.server, INGEST, accept_all).await;
        rig.fail(1);
        let generation = stored(&rig.owner()).generation;

        let outcome = rig.cycle().await;
        assert!(rig.lock_was_free());
        if change == Change::None {
            assert_eq!(outcome, DELIVERED_ONE);
            assert_eq!(rig.paths().await, [MINT, INGEST]);
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{change:?}");
        assert_eq!(rig.paths().await, [MINT], "{change:?}: no ingest request");
        // Noticed at once, not after a retry's wait.
        assert_eq!(rig.probe.sleeps(), [], "{change:?}");
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

#[tokio::test]
async fn a_change_during_the_backoff_sends_nothing_more() {
    for change in CHANGES {
        let rig = Rig::opted_in().await;
        mount(&rig.server, MINT, minted()).await;
        // The server asks for more patience than the cap allows.
        mount(
            &rig.server,
            INGEST,
            ResponseTemplate::new(503).insert_header("retry-after", "120"),
        )
        .await;
        let rows = rig.fail(1);
        let (meddler, base) = (rig.meddler(), rig.base.clone());
        rig.probe
            .during_sleep(move || meddler.run(|state| change.apply(state, &base)));

        let outcome = rig.cycle().await;
        assert!(rig.lock_was_free());
        if change == Change::None {
            assert_eq!(outcome, CycleOutcome::GaveUp);
            assert_eq!(
                rig.paths().await,
                [MINT, INGEST, MINT, INGEST, MINT, INGEST]
            );
            assert_eq!(rig.probe.sleeps(), [CAP, CAP]);
            assert_eq!(queued(&rig.owner()), rows);
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{change:?}");
        assert_eq!(
            rig.paths().await,
            [MINT, INGEST],
            "{change:?}: one POST and no second mint"
        );
        assert_eq!(rig.probe.sleeps(), [CAP], "min(Retry-After, cap)");
        assert_eq!(queued(&rig.owner()), [], "{change:?}");
    }
}

#[tokio::test]
async fn a_change_inside_the_post_response_applies_nothing_to_the_new_queue() {
    for change in CHANGES {
        let rig = Rig::opted_in().await;
        let (meddler, base) = (rig.meddler(), rig.base.clone());
        mount(&rig.server, MINT, minted()).await;
        mount(&rig.server, INGEST, move |_: &Request| {
            // The queue is replaced and a new attempt is queued in the new one,
            // all before the old batch's verdict arrives.
            meddler.run(|state| {
                change.apply(state, &base);
                if change == Change::OptOut {
                    set_consent(state, true);
                }
                if change != Change::None {
                    fail(state, 1);
                }
            });
            results(&["rejected"])
        })
        .await;
        let old = rig.fail(1);
        let generation = stored(&rig.owner()).generation;

        let outcome = rig.cycle().await;
        assert!(rig.lock_was_free());
        assert_eq!(rig.paths().await, [MINT, INGEST], "no second POST");
        // Taking the state waits for a responder that is still at work.
        let on_disk = stored(&rig.owner());
        if change == Change::None {
            assert_eq!(
                outcome,
                CycleOutcome::Delivered {
                    accepted: 0,
                    duplicate: 0,
                    rejected: 1,
                }
            );
            assert_eq!(on_disk.entries, []);
            assert_eq!(on_disk.dropped.rejected, 1);
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{change:?}");
        // The new queue is untouched: its one new row is there and the old
        // batch's rejection was not counted against it.
        let replacements = if change == Change::OptOut { 2 } else { 1 };
        assert_eq!(on_disk.generation, generation + replacements, "{change:?}");
        assert_eq!(on_disk.entries.len(), 1, "{change:?}");
        assert_ne!(on_disk.entries[0].summary.event_id, old[0].event_id);
        assert_eq!(on_disk.dropped, DroppedCounts::default(), "{change:?}");
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
        assert_eq!(rig.paths().await, [MINT, INGEST], "exactly one POST");
        let posts = rig.bodies(INGEST).await;
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
            assert_eq!(rig.paths().await, [MINT, INGEST]);
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
