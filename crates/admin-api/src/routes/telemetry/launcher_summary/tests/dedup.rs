//! Deduplication by `event_id`: one row per id however often and from
//! wherever it arrives, a set that never grows and never refuses.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Barrier;

use super::super::dedup::{Dedup, CAPACITY};
use super::super::dto::Verdict::{self, Accepted, Duplicate, Rejected};
use super::{batch, batch_rows, capture, element, summary_rows, Env, Harness, Rows};

/// The same id twice in one request: accepted, then duplicate, one row.
/// A different id in the second position is the control.
///
/// Skipping the insert in `Dedup::judge` makes the second verdict
/// `accepted` and writes a second row.
#[test]
fn a_repeat_inside_one_request_is_a_duplicate() {
    let _env = Env::install();
    let h = Harness::new();
    let (results, rows) = capture(|| h.verdicts(&batch(vec![element(1), element(1)])));
    assert_eq!(results, [Accepted, Duplicate]);
    assert_eq!(summary_rows(&rows).len(), 1);
    let counts = &batch_rows(&rows)[0].fields;
    assert_eq!((&*counts["accepted"], &*counts["duplicate"]), ("1", "1"));

    let h = Harness::new();
    let (results, rows) = capture(|| h.verdicts(&batch(vec![element(1), element(2)])));
    assert_eq!(results, [Accepted, Accepted], "control");
    assert_eq!(summary_rows(&rows).len(), 2);
}

/// The same id in a second request (the launcher resending after a lost
/// response) is a duplicate there and writes no second row.
#[test]
fn a_resend_in_a_later_request_is_a_duplicate() {
    let _env = Env::install();
    let h = Harness::new();
    let (results, rows) = capture(|| {
        let first = h.verdicts(&batch(vec![element(1)]));
        let resend = h.verdicts(&batch(vec![element(1), element(2)]));
        (first, resend)
    });
    assert_eq!(results.0, [Accepted]);
    assert_eq!(results.1, [Duplicate, Accepted]);
    assert_eq!(summary_rows(&rows).len(), 2, "one row per distinct id");
    assert_eq!(batch_rows(&rows).len(), 2, "one batch row per request");
}

/// Two threads post the same id through one shared state at the same
/// moment, forty times over with a fresh id each round: every round has
/// exactly one `accepted` and one `duplicate`, and one row.
#[test]
fn two_threads_posting_one_id_get_one_accepted_between_them() {
    let _env = Env::install();
    let h = Harness::new();
    let rows = Rows::default();
    const ROUNDS: u32 = 40;
    for round in 1..=ROUNDS {
        let body = batch(vec![element(round)]);
        let start = Barrier::new(2);
        let post = || {
            rows.record(|| {
                start.wait();
                h.verdicts(&body)
            })
        };
        let (a, b) = std::thread::scope(|scope| {
            let a = scope.spawn(post);
            let b = scope.spawn(post);
            (a.join().unwrap(), b.join().unwrap())
        });
        let mut verdicts: Vec<Verdict> = a.into_iter().chain(b).collect();
        verdicts.sort_by_key(|v| *v == Duplicate);
        assert_eq!(verdicts, [Accepted, Duplicate], "round {round}");
    }
    let rows = rows.snapshot();
    assert_eq!(summary_rows(&rows).len(), ROUNDS as usize);
    assert_eq!(batch_rows(&rows).len(), 2 * ROUNDS as usize);
}

/// **One lock acquisition per request.** Two threads judge the same
/// request at the same moment, straight through `Dedup::judge`, 4000 times
/// over with four fresh ids each time: every time, one thread gets all four
/// as `accepted` and the other all four as `duplicate`.
///
/// The test above cannot tell a request judged under one lock from one
/// that takes the lock per id (its requests hold one id), and it rarely
/// catches one that checks under one lock and inserts under another: a
/// `Barrier` lets its last arrival run at once and wakes the other thread
/// tens of microseconds later, by which time the first request is done.
/// Here both threads are on a core before either starts, and there are
/// enough rounds that a split of either kind shows up in some of them: the
/// per-id lock gives both threads a mixture, the two-step one gives both
/// threads `accepted` for the same ids. (Measured with each split put in:
/// the per-id lock shows within the first few rounds, the two-step one in
/// roughly one round in 300, which is why there are thousands.)
#[test]
fn two_threads_judging_the_same_ids_get_all_accepted_and_all_duplicate() {
    const ROUNDS: usize = 4_000;
    const PER_REQUEST: usize = 4;
    // One set for the whole test, and nothing is forgotten during it.
    const { assert!(ROUNDS * PER_REQUEST <= CAPACITY) };
    let requests: Vec<Vec<Option<u128>>> = (0..ROUNDS)
        .map(|round| {
            let first = (round * PER_REQUEST) as u128 + 1;
            (first..first + PER_REQUEST as u128).map(Some).collect()
        })
        .collect();

    let dedup = Dedup::new();
    let start = Barrier::new(2);
    let running = AtomicUsize::new(0);
    let judge_every_request = || -> Vec<Vec<Verdict>> {
        let mut verdicts = Vec::with_capacity(ROUNDS);
        for (round, ids) in requests.iter().enumerate() {
            start.wait();
            // The barrier has released both threads, but one of them is
            // still being woken. Wait, without sleeping, until both are
            // running, so the two calls below start together.
            running.fetch_add(1, Ordering::SeqCst);
            let mut spins = 0u32;
            while running.load(Ordering::SeqCst) < 2 * (round + 1) {
                spins += 1;
                if spins.is_multiple_of(4_096) {
                    std::thread::yield_now();
                } else {
                    std::hint::spin_loop();
                }
            }
            verdicts.push(dedup.judge(ids));
        }
        verdicts
    };
    let (a, b) = std::thread::scope(|scope| {
        let a = scope.spawn(judge_every_request);
        let b = scope.spawn(judge_every_request);
        (a.join().unwrap(), b.join().unwrap())
    });

    let all = |verdicts: &[Verdict], wanted: Verdict| {
        verdicts.len() == PER_REQUEST && verdicts.iter().all(|v| *v == wanted)
    };
    assert_eq!((a.len(), b.len()), (ROUNDS, ROUNDS));
    for (round, (a, b)) in a.iter().zip(&b).enumerate() {
        assert!(
            (all(a, Accepted) && all(b, Duplicate)) || (all(a, Duplicate) && all(b, Accepted)),
            "round {round}: a request was judged in more than one step: {a:?} and {b:?}"
        );
    }
    // Control: every id was remembered, so a third pass is all duplicates.
    for ids in &requests {
        assert!(all(&dedup.judge(ids), Duplicate));
    }
}

/// A rejected element's id is not remembered: the same id in a valid
/// element afterwards is accepted, not a duplicate. The control is the
/// valid element sent twice.
#[test]
fn a_rejected_elements_id_is_not_remembered() {
    let _env = Env::install();
    let mut invalid = element(1);
    invalid["operation"] = "reinstall".into();

    let h = Harness::new();
    assert_eq!(h.verdicts(&batch(vec![invalid])), [Rejected]);
    assert_eq!(h.verdicts(&batch(vec![element(1)])), [Accepted]);
    assert_eq!(h.verdicts(&batch(vec![element(1)])), [Duplicate], "control");
}

/// The set is bounded and never refuses. At the real capacity: after
/// `CAPACITY` distinct ids all are still remembered; one more is accepted
/// and pushes out only the oldest, which is then accepted again.
#[test]
fn a_full_set_forgets_the_oldest_and_refuses_nothing() {
    assert_eq!(CAPACITY, 16_384);
    let dedup = Dedup::new();
    let judge = |id: u128| dedup.judge(&[Some(id)])[0];

    for id in 1..=CAPACITY as u128 {
        assert_eq!(judge(id), Accepted, "id {id}");
    }
    assert_eq!(judge(1), Duplicate, "the oldest is still remembered");
    assert_eq!(judge(CAPACITY as u128), Duplicate);

    assert_eq!(judge(CAPACITY as u128 + 1), Accepted, "never refused");
    assert_eq!(judge(1), Accepted, "the oldest was forgotten");
    // That re-insert pushed out id 2 and nothing else.
    assert_eq!(judge(3), Duplicate);
    assert_eq!(judge(CAPACITY as u128 + 1), Duplicate);
}

/// The same through the ingest, with a two-id set: three distinct ids are
/// all accepted, and the first, pushed out by the third, is accepted and
/// emitted again. Nothing is rejected.
#[test]
fn the_ingest_accepts_past_the_dedup_capacity() {
    let _env = Env::install();
    let h = Harness::new().with_dedup_capacity(2);
    let (results, rows) = capture(|| {
        let first = h.verdicts(&batch(vec![element(1), element(2), element(3)]));
        let again = h.verdicts(&batch(vec![element(3), element(1)]));
        (first, again)
    });
    assert_eq!(results.0, [Accepted, Accepted, Accepted]);
    assert_eq!(results.1, [Duplicate, Accepted]);
    assert_eq!(summary_rows(&rows).len(), 4);
}
