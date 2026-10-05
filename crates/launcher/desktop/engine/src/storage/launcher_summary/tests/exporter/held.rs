//! A withdrawal of consent while the POST is in flight. The server holds its
//! answer; the opt-out is issued once the request is with the server, and the
//! cycle must end before the held answer could have arrived. The same run
//! without the opt-out, which waits for the answer and delivers, is the control.
use super::*;

/// How long the server holds its answer. A cycle that ends sooner cannot have
/// had it.
const HELD: Duration = Duration::from_secs(3);

#[tokio::test]
async fn an_opt_out_aborts_a_held_post() {
    for withdrawn in [false, true] {
        let rig = Rig::opted_in().await;
        mount(&rig.server, results(&["accepted"]).set_delay(HELD)).await;
        rig.fail(1);
        let generation = stored(&rig.owner()).generation;

        let started = std::time::Instant::now();
        let (outcome, ()) = tokio::join!(rig.cycle(), async {
            if withdrawn {
                // The request is with the server before consent is withdrawn.
                assert_eq!(rig.paths_after(1).await, [INGEST]);
                set_consent(&mut rig.owner(), false);
            }
        });
        let elapsed = started.elapsed();
        if !withdrawn {
            assert_eq!(outcome, DELIVERED_ONE);
            assert!(elapsed >= HELD, "the answer really was held");
            assert_eq!(rig.paths().await, [INGEST]);
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn);
        assert!(
            elapsed < HELD,
            "took {elapsed:?}, so the request was not aborted"
        );
        // Nothing followed the aborted request, and nothing waited.
        assert_eq!(rig.paths().await, [INGEST]);
        assert_eq!(rig.probe.sleeps(), []);
        assert_eq!(stored(&rig.owner()), Queue::empty(generation + 1));
    }
}
